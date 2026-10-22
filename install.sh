#!/usr/bin/env bash
# install.sh — curl|sh installer for rust-red
# Usage: curl -fsSL https://raw.githubusercontent.com/dzulfikar08/rust-red/master/install.sh | sh
set -euo pipefail

REPO="dzulfikar08/rust-red"
GITHUB_API="https://api.github.com/repos/${REPO}/releases/latest"
GITHUB_DL_BASE="https://github.com/${REPO}/releases/download"

BINARY_NAME="rust-red"
INSTALL_DIR_DEFAULT="/usr/local/bin"
INSTALL_DIR_FALLBACK="${HOME}/.local/bin"

# ── Colors ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info()  { printf "${BLUE}[info]${NC}  %s\n" "$*"; }
warn()  { printf "${YELLOW}[warn]${NC}  %s\n" "$*"; }
error() { printf "${RED}[error]${NC} %s\n" "$*" >&2; }
success() { printf "${GREEN}[ok]${NC}    %s\n" "$*"; }

# ── Helpers ──────────────────────────────────────────────────────────────────
need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        error "Required command '$1' not found. Please install it and retry."
        exit 1
    fi
}

check_cmd() {
    command -v "$1" >/dev/null 2>&1
}

# ── Preflight checks ────────────────────────────────────────────────────────
need_cmd curl
need_cmd uname
need_cmd mv
need_cmd chmod

# ── Detect OS ───────────────────────────────────────────────────────────────
detect_os() {
    local os
    case "$(uname -s)" in
        Linux*)   os="linux" ;;
        Darwin*)  os="macos" ;;
        MINGW*|MSYS*|CYGWIN*|Windows_NT)
                  os="windows" ;;
        *)        error "Unsupported operating system: $(uname -s)"; exit 1 ;;
    esac
    echo "$os"
}

# ── Detect architecture ─────────────────────────────────────────────────────
detect_arch() {
    local arch
    local machine
    machine="$(uname -m)"

    case "$machine" in
        x86_64|amd64)       arch="x86_64" ;;
        aarch64|arm64)      arch="aarch64" ;;
        armv7l|armv7)       arch="armv7" ;;
        *)
            error "Unsupported architecture: $machine"
            exit 1
            ;;
    esac
    echo "$arch"
}

# ── Get the latest release tag from GitHub ──────────────────────────────────
get_latest_version() {
    local version
    version=$(curl -fsSL "$GITHUB_API" 2>/dev/null | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/')

    if [ -z "$version" ]; then
        error "Failed to determine the latest version from GitHub."
        error "Please check your internet connection and that the repository has releases."
        exit 1
    fi

    # Strip leading 'v' if present
    version="${version#v}"
    echo "$version"
}

# ── Determine install directory ─────────────────────────────────────────────
determine_install_dir() {
    local dir="$1"

    # If user specified a directory, use it
    if [ -n "$dir" ]; then
        echo "$dir"
        return
    fi

    # Try the default first
    if [ -w "$INSTALL_DIR_DEFAULT" ] || check_cmd sudo; then
        echo "$INSTALL_DIR_DEFAULT"
        return
    fi

    # Fall back to ~/.local/bin
    warn "Cannot write to ${INSTALL_DIR_DEFAULT} (no sudo)."
    warn "Installing to ${INSTALL_DIR_FALLBACK} instead."
    echo "$INSTALL_DIR_FALLBACK"
}

# ── Download, verify, and install ───────────────────────────────────────────
main() {
    local os arch version install_dir
    local download_url archive_name archive_ext binary_file checksum_url
    local tmp_dir

    os="$(detect_os)"
    arch="$(detect_arch)"
    info "Detected OS: ${os}, Architecture: ${arch}"

    # ── Get latest version ──────────────────────────────────────────────
    version="$(get_latest_version)"
    info "Latest version: ${version}"

    # ── Build download URL ──────────────────────────────────────────────
    if [ "$os" = "windows" ]; then
        archive_ext=".zip"
    else
        archive_ext=".tar.gz"
    fi

    archive_name="${BINARY_NAME}-${version}-${os}-${arch}${archive_ext}"
    download_url="${GITHUB_DL_BASE}/v${version}/${archive_name}"
    checksum_url="${GITHUB_DL_BASE}/v${version}/${archive_name}.sha256"

    # ── Create temp directory ───────────────────────────────────────────
    tmp_dir="$(mktemp -d 2>/dev/null || mktemp -d -t '${BINARY_NAME}-install')"
    trap 'rm -rf "$tmp_dir"' EXIT

    info "Downloading ${archive_name}..."
    local archive_path="${tmp_dir}/${archive_name}"

    curl -fsSL --progress-bar -o "$archive_path" "$download_url" || {
        error "Download failed. URL: ${download_url}"
        error "This combination of OS/architecture may not have a pre-built binary."
        error "Try building from source: cargo install --git https://github.com/dzulfikar08/rust-red"
        exit 1
    }

    # ── Verify checksum ─────────────────────────────────────────────────
    info "Verifying checksum..."
    local expected_checksum actual_checksum
    expected_checksum=$(curl -fsSL "$checksum_url" 2>/dev/null | awk '{print $1}') || true

    if [ -n "$expected_checksum" ]; then
        if check_cmd sha256sum; then
            actual_checksum=$(sha256sum "$archive_path" | awk '{print $1}')
        elif check_cmd shasum; then
            actual_checksum=$(shasum -a 256 "$archive_path" | awk '{print $1}')
        elif check_cmd openssl; then
            actual_checksum=$(openssl dgst -sha256 "$archive_path" | awk '{print $NF}')
        else
            warn "No sha256 tool found. Skipping checksum verification."
            expected_checksum=""
        fi

        if [ -n "$expected_checksum" ] && [ "$actual_checksum" != "$expected_checksum" ]; then
            error "Checksum mismatch!"
            error "  Expected: ${expected_checksum}"
            error "  Actual:   ${actual_checksum}"
            error "The downloaded file may be corrupted or tampered with. Aborting."
            exit 1
        fi
    else
        warn "Checksum file not found. Skipping verification."
    fi

    # ── Extract ─────────────────────────────────────────────────────────
    info "Extracting archive..."
    if [ "$os" = "windows" ]; then
        need_cmd unzip
        unzip -o -q "$archive_path" -d "$tmp_dir/extracted" || {
            error "Failed to extract archive."
            exit 1
        }
    else
        tar xzf "$archive_path" -C "$tmp_dir/extracted" 2>/dev/null || {
            mkdir -p "$tmp_dir/extracted"
            tar xzf "$archive_path" -C "$tmp_dir/extracted" || {
                error "Failed to extract archive."
                exit 1
            }
        }
    fi

    # ── Locate binary ───────────────────────────────────────────────────
    binary_file="$(find "${tmp_dir}/extracted" -name "${BINARY_NAME}" -type f 2>/dev/null | head -1)"
    if [ -z "$binary_file" ]; then
        # On windows, try with .exe
        if [ "$os" = "windows" ]; then
            binary_file="$(find "${tmp_dir}/extracted" -name "${BINARY_NAME}.exe" -type f 2>/dev/null | head -1)"
        fi
    fi

    if [ -z "$binary_file" ]; then
        error "Could not find '${BINARY_NAME}' binary in the archive."
        exit 1
    fi

    chmod +x "$binary_file"

    # ── Determine install location ──────────────────────────────────────
    local requested_dir="${RUST_RED_INSTALL_DIR:-}"
    install_dir="$(determine_install_dir "$requested_dir")"

    # ── Install ─────────────────────────────────────────────────────────
    info "Installing ${BINARY_NAME} to ${install_dir}..."
    mkdir -p "$install_dir"

    local dest="${install_dir}/${BINARY_NAME}"
    if [ -w "$install_dir" ]; then
        mv "$binary_file" "$dest"
    elif check_cmd sudo; then
        sudo mv "$binary_file" "$dest"
    else
        error "Cannot write to ${install_dir}. Set RUST_RED_INSTALL_DIR to choose another location."
        exit 1
    fi
    chmod +x "$dest"

    # The web editor is served from a ui_static directory next to the binary.
    local ui_src
    ui_src="$(dirname "$binary_file")/ui_static"
    if [ -d "$ui_src" ]; then
        if [ -w "$install_dir" ]; then
            rm -rf "${install_dir}/ui_static" && cp -R "$ui_src" "${install_dir}/ui_static"
        else
            sudo rm -rf "${install_dir}/ui_static" && sudo cp -R "$ui_src" "${install_dir}/ui_static"
        fi
    fi

    # ── Verify installation ─────────────────────────────────────────────
    if check_cmd rust-red; then
        success "rust-red v${version} installed successfully!"
        success "  Binary:  ${dest}"
        success "  Version: $("${dest}" --version 2>/dev/null || echo "${version}")"
        echo ""
        info "Run 'rust-red --help' to get started."

        # Warn if install dir is not in PATH
        if ! echo ":${PATH}:" | grep -q ":${install_dir}:"; then
            warn "${install_dir} is not in your PATH."
            info "Add it by running:"
            if [ -f "${HOME}/.bashrc" ]; then
                info "  echo 'export PATH=\"${install_dir}:\$PATH\"' >> ~/.bashrc && source ~/.bashrc"
            elif [ -f "${HOME}/.zshrc" ]; then
                info "  echo 'export PATH=\"${install_dir}:\$PATH\"' >> ~/.zshrc && source ~/.zshrc"
            else
                info "  export PATH=\"${install_dir}:\$PATH\""
            fi
        fi
    else
        warn "Installation completed but 'rust-red' is not found in PATH."
        info "The binary is at: ${dest}"
        info "Ensure ${install_dir} is in your PATH."
    fi
}

main "$@"
