# Getting Started

## Prerequisites

- **Rust** 1.80 or later ([install](https://rustup.rs/))
- **Git** (for cloning)
- On Windows: Visual Studio Build Tools with MSVC

## Install from Source

```bash
# Clone with submodules (includes the web UI)
git clone --recursive https://github.com/dzulfikar08/rust-red.git
cd rust-red

# Build release binary
cargo build --release

# The binary is at:
./target/release/rust-red --help
```

If you cloned without `--recursive`:

```bash
git submodule update --init --recursive
```

## Run

```bash
# Start with the web UI (opens browser automatically)
./target/release/rust-red

# Headless mode (no browser, for production)
./target/release/rust-red run --headless

# Custom bind address
./target/release/rust-red --bind 0.0.0.0:8080

# Specify a flow file
./target/release/rust-red /path/to/flows.json

# Custom user directory
./target/release/rust-red -u /opt/rust-red-data
```

When running with the UI, open [http://127.0.0.1:1888](http://127.0.0.1:1888) in your browser.

## Your First Flow

1. Start rust-red: `./target/release/rust-red`
2. The editor opens in your browser at `http://127.0.0.1:1888`
3. Drag an **inject** node from the palette onto the canvas
4. Drag a **debug** node next to it
5. Wire them together by dragging from the inject output to the debug input
6. Click **Deploy**
7. Click the inject node's button to trigger it
8. Check the debug panel on the right - you'll see the message

This is the "Hello World" of Node-RED: inject a timestamp message, display it in the debug panel.

## Import an Existing Flow

If you have a `flows.json` from Node-RED:

```bash
# Place it in the default location
cp flows.json ~/.rust-red/flows.json
./target/release/rust-red
```

Or specify the path directly:

```bash
./target/release/rust-red /path/to/flows.json
```

Rust-RED reads the same flow format as Node-RED. Most flows work without modification.

## Run Tests

```bash
# All tests
cargo test --all

# Node-RED compatibility suite (259 tests)
cargo test --package rust-red-core --features internal-testing

# Cluster tests
cargo test --features cluster --test cluster_integration
```

## Directory Structure

```
~/.rust-red/
  flows.json          # Your flow definitions
  config.toml         # Runtime configuration
  flows_cred.json     # Node credentials in Node-RED's format, encrypted (see README, Credentials)
  .config.runtime.json # Holds _credentialSecret when no credential_secret is configured
```

## Next Steps

- [Configuration](./configuration.md) - all CLI flags, env vars, config file options
- [Node Reference](./nodes/index.md) - every built-in node with examples
- [Clustering](./clustering.md) - multi-node HA setup
- [Deployment](./deployment.md) - Docker, systemd, production best practices
