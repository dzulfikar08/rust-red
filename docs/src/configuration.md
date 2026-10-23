# Configuration

Rust-RED can be configured via command-line arguments, environment variables, or a TOML config file.

## Command-Line Arguments

```
rust-red [OPTIONS] [FLOWS_PATH]

Arguments:
  [FLOWS_PATH]  Path to flow file [default: ~/.rust-red/flows.json]

Options:
  --headless             Run without web UI
  --bind <BIND>          Web server bind address [default: 127.0.0.1:1888]
  -u, --user-dir <DIR>   User data directory [default: ~/.rust-red]
  --credential-secret <SECRET>  Secret for flows_cred.json [env: NODE_RED_CREDENTIAL_SECRET]
  -c, --config <PATH>    Path to config file [default: ~/.rust-red/config.toml]
  -h, --help             Show help
  -V, --version          Show version
```

## Configuration File

Create `~/.rust-red/config.toml` (or specify with `-c`):

```toml
[ui-host]
host = "0.0.0.0"
port = 1888

[cluster]
enabled = true
node_id = "node-1"
bind = "0.0.0.0:7980"
peers = ["10.0.0.2:7980", "10.0.0.3:7980"]
heartbeat_interval_ms = 2000
failure_timeout_ms = 10000

[logging]
level = "info"          # trace, debug, info, warn, error
file = "/var/log/rust-red.log"
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level filter | `info` |
| `RUST_RED_USER_DIR` | User data directory | `~/.rust-red` |
| `RUST_RED_BIND` | Web server bind address | `127.0.0.1:1888` |
| `RUST_RED_HEADLESS` | Disable web UI | `false` |

## Logging

Rust-RED uses the standard Rust logging ecosystem. Control verbosity with `RUST_LOG`:

```bash
# Standard levels
RUST_LOG=error ./target/release/rust-red    # Only errors
RUST_LOG=warn ./target/release/rust-red     # Warnings and above
RUST_LOG=info ./target/release/rust-red     # Default
RUST_LOG=debug ./target/release/rust-red    # Verbose
RUST_LOG=trace ./target/release/rust-red    # Everything

# Per-module filtering
RUST_LOG=rust_red_core::runtime=debug ./target/release/rust-red
```

## Feature Flags at Build Time

Features are selected at compile time, not runtime:

```bash
# Production build with everything
cargo build --release --features full

# Edge gateway (MQTT + Modbus only)
cargo build --release --no-default-features --features core,js,nodes_network,nodes_modbus

# Headless data logger (PostgreSQL + SQLite)
cargo build --release --no-default-features --features core,nodes_postgres,nodes_sqlite
```

## Default Ports

| Service | Port | Config |
|---------|------|--------|
| Web UI / API | 1888 | `--bind` |
| Gossip (cluster) | 7980 | `[cluster].bind` |
| MQTT broker | 1883 | Flow node config |
