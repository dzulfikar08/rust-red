# Rust-Red: Node-RED Reimplemented in Rust

[![Build Status]][actions]
[![GitHub Release]][releases]
[![GitHub Downloads]][releases]

[Build Status]: https://img.shields.io/github/actions/workflow/status/dzulfikar08/rust-red/CICD.yml?branch=master
[actions]: https://github.com/dzulfikar08/rust-red/actions?query=branch%3Amaster
[GitHub Release]: https://img.shields.io/github/v/release/dzulfikar08/rust-red?include_prereleases
[releases]: https://github.com/dzulfikar08/rust-red/releases
[GitHub Downloads]: https://img.shields.io/github/downloads/dzulfikar08/rust-red/total

## Overview

**Rust-Red** is a high-performance, memory-efficient Node-RED compatible runtime engine built from the ground up in Rust, featuring an integrated web UI for complete standalone operation.

**Why Rust-Red?**
- **10x less memory usage** than Node-RED (only 10% of Node-RED's memory footprint)
- **Native performance** with Rust's zero-cost abstractions
- **Integrated web interface** - full Node-RED UI built-in for flow design and management
- **Standalone operation** - no external Node-RED installation required
- **Drop-in replacement** - use your existing `flows.json` files
- **Perfect for edge devices** with limited resources
- **Built-in clustering & HA** - run multiple nodes for fault tolerance and horizontal scaling
- **259+ passing tests** — comprehensive Node-RED compatibility test suite
- **Node-RED compatibility** - design, deploy, and run flows all in one application

Rust-Red includes the complete Node-RED web editor, allowing you to design flows directly in the browser while executing them with native Rust performance. You can also run it headless for production deployments on resource-constrained devices.

Only the `function` node uses the lightweight QuickJS JS interpreter to run JavaScript code; all other functionalities are implemented in native Rust code for maximum performance.

## Quick Start

### 0. Clone the Repository

**Clone the repository with submodules:**

```bash
git clone --recursive https://github.com/dzulfikar08/rust-red.git
```

Or if you've already cloned without submodules:

```bash
git clone https://github.com/dzulfikar08/rust-red.git
cd rust-red
git submodule update --init --recursive
```

### 1. Build

**Prerequisites**: Rust 1.80 or later

```bash
cargo build --release
```

**Windows users**: Ensure `patch.exe` is in your PATH (included with Git) and install Visual Studio for MSVC.

**Supported platforms**:

- `x86_64-pc-windows-msvc`
- `x86_64-pc-windows-gnu`
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `armv7-unknown-linux-gnueabihf`
- `armv7-unknown-linux-gnueabi`

### 2. Run

**Start Rust-Red with integrated web UI (recommended):**

```bash
cargo run --release --
# or after build
./target/release/rust-red
```

By default, your browser will open the Node-RED frontend at [http://127.0.0.1:1888](http://127.0.0.1:1888).

**Main command-line options:**

- `[FLOWS_PATH]`: Optional, specify the flow file (default: `~/.rust-red/flows.json`)
- `--headless`: Headless mode (no Web UI, suitable for production)
- `--bind <BIND>`: Custom web server bind address (default: `127.0.0.1:1888`)
- `-u, --user-dir <USER_DIR>`: Specify user directory (default: `~/.rust-red`)
- See more options with `--help`

**Examples:**

```bash
# Run in headless mode
./target/release/rust-red run --headless

# Specify flow file and port
./target/release/rust-red run ./myflows.json --bind 0.0.0.0:8080
```

> All data and configuration are stored in the `~/.rust-red` directory by default.

### Run Tests

```bash
# Unit and integration tests
cargo test --all

# Node-RED compatibility test suite (259 tests)
cargo test --package rust-red-core --features internal-testing

# Cluster integration tests
cargo test --features cluster --test cluster_integration
```

## Configuration

Rust-Red can be configured through command-line arguments and configuration files.

### Web UI Configuration

**Command-line options:**
- `--bind <address>`: Set the web server binding address (default: `127.0.0.1:1888`)
- `--headless`: Run without the web UI for production deployments
- `--user-dir <path>`: Specify custom user directory for flows and settings

**Configuration file** (`rust-red.toml`):

```toml
[ui-host]
host = "0.0.0.0"
port = 1888
```

---

## Clustering & High Availability

Rust-Red has built-in clustering. Run multiple instances as a single logical unit — each node runs a subset of your flows, and if a node dies, its flows are automatically reassigned to surviving nodes. No external tools required.

### Why This Matters

| Problem | Without Cluster | With Cluster |
|---------|----------------|--------------|
| Node crashes | All flows stop | Flows migrate to surviving nodes |
| Need more throughput | Vertical scaling only | Add more nodes horizontally |
| Deployment downtime | Stop -> deploy -> start | Zero-downtime rolling deploys |
| Single point of failure | Yes | No — automatic failover |

Node-RED runs as a single process — if it crashes, everything stops. Rust-Red's clustering fixes this: run 10+ Rust-Red instances where 1 Node-RED instance runs today, using roughly the same total memory.

### How It Works

```
 +-----------+     +-----------+     +-----------+
 |  Node A   |<--->|  Node B   |<--->|  Node C   |
 | (Leader)  |     | (Follower)|     | (Follower)|
 |           |     |           |     |           |
 | Flows:    |     | Flows:    |     | Flows:    |
 |  flow-0   |     |  flow-1   |     |  flow-2   |
 |  flow-3   |     |  flow-4   |     |  flow-5   |
 |           |     |           |     |           |
 | Gossip ---+-----+--- Gossip-+-----+--- Gossip |
 | :7980     |     | :7980     |     | :7980     |
 +-----------+     +-----------+     +-----------+
        |               |                 |
        +---------------+-----------------+
                        |
                HTTP API :1888
          (each node serves its own)
```

**Five subsystems:**

1. **Gossip Membership** — Each node sends heartbeats to a random peer every 2 seconds. Heartbeats carry membership data. 10% of heartbeats include a full membership table sync. New nodes are auto-discovered.

2. **Failure Detection** — If a node misses heartbeats beyond the timeout (default 10s), it transitions through `Suspect` -> `Dead`. Dead nodes trigger automatic flow rebalancing.

3. **Flow Partitioning** — The leader (lowest alive node ID) assigns flows round-robin across alive nodes. Each node only starts the flows it owns. If a node dies, its flows are redistributed to survivors.

4. **State Synchronization** — Global context is replicated across nodes with version-based last-writer-wins conflict resolution. Deployment requests are coordinated with acknowledgement tracking.

5. **Session Affinity** — Consistent hashing determines which node owns a session key, ensuring sticky routing.

### Step-by-Step: 3-Node Cluster on Ubuntu VPS

This guide assumes three Ubuntu 22.04/24.04 VPS instances on a private network. Adjust IP addresses to match your setup.

```
Node A: 10.0.0.1  (leader)
Node B: 10.0.0.2
Node C: 10.0.0.3
```

#### Step 1: Build and Distribute

```bash
# On your build machine:
git clone --recursive https://github.com/dzulfikar08/rust-red.git
cd rust-red
cargo build --release --features cluster

# Copy the binary to each VPS:
scp target/release/rust-red user@10.0.0.1:/usr/local/bin/
scp target/release/rust-red user@10.0.0.2:/usr/local/bin/
scp target/release/rust-red user@10.0.0.3:/usr/local/bin/

# Copy static UI files:
scp -r target/ui_static/ user@10.0.0.1:/opt/rust-red/ui_static/
scp -r target/ui_static/ user@10.0.0.2:/opt/rust-red/ui_static/
scp -r target/ui_static/ user@10.0.0.3:/opt/rust-red/ui_static/
```

> **Important**: Use `--features cluster` to enable clustering. Without it, all cluster code is compiled out with zero overhead.

#### Step 2: Create Config on Each Node

**Node A** (`/etc/rust-red/config.toml`):

```toml
[ui-host]
host = "0.0.0.0"
port = 1888

[cluster]
enabled = true
node_id = "node-a"
bind = "0.0.0.0:7980"
peers = ["10.0.0.2:7980", "10.0.0.3:7980"]
heartbeat_interval_ms = 2000
failure_timeout_ms = 10000
```

**Node B** (`/etc/rust-red/config.toml`):

```toml
[ui-host]
host = "0.0.0.0"
port = 1888

[cluster]
enabled = true
node_id = "node-b"
bind = "0.0.0.0:7980"
peers = ["10.0.0.1:7980", "10.0.0.3:7980"]
heartbeat_interval_ms = 2000
failure_timeout_ms = 10000
```

**Node C** (`/etc/rust-red/config.toml`):

```toml
[ui-host]
host = "0.0.0.0"
port = 1888

[cluster]
enabled = true
node_id = "node-c"
bind = "0.0.0.0:7980"
peers = ["10.0.0.1:7980", "10.0.0.2:7980"]
heartbeat_interval_ms = 2000
failure_timeout_ms = 10000
```

#### Step 3: Open Firewall Ports

```bash
sudo ufw allow 1888/tcp   # Web UI / API
sudo ufw allow 7980/tcp   # Gossip protocol
```

#### Step 4: Start Each Node

```bash
rust-red -c /etc/rust-red/config.toml
```

Or in headless mode (recommended for production):

```bash
rust-red run --headless -c /etc/rust-red/config.toml
```

#### Step 5: Verify

```bash
curl http://10.0.0.1:1888/cluster/status | jq
```

### Configuration Reference

| Setting | Default | Description |
|---------|---------|-------------|
| `enabled` | `false` | Enable or disable clustering |
| `node_id` | auto UUID | Unique identifier for this node (auto-generated if empty) |
| `bind` | `"0.0.0.0:7980"` | Address to bind the gossip listener |
| `peers` | `[]` | List of peer addresses for initial discovery |
| `heartbeat_interval_ms` | `2000` | How often heartbeats are sent (ms) |
| `failure_timeout_ms` | `10000` | Time before a node is declared dead (ms) |
| `discovery_mode` | `"static"` | Peer discovery: `static`, `multicast`, or `dns` |
| `multicast_addr` | `"239.255.0.1:7980"` | Multicast group address (multicast mode only) |
| `dns_service` | `""` | DNS service name to resolve (dns mode only) |
| `cluster_port` | `7980` | Port for multicast/DNS-discovered peers |

### API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/cluster/status` | Cluster health, members, leader, alive count |
| `GET` | `/cluster/nodes` | List all cluster nodes with details |
| `POST` | `/cluster/deploy` | Deploy flows cluster-wide |
| `GET` | `/cluster/flows` | View flow-to-node assignment table |

### What Happens When a Node Dies

```
Before failure:
  Node A: flow-0, flow-3   (leader)
  Node B: flow-1, flow-4
  Node C: flow-2, flow-5

Node C dies -> Leader rebalances -> Flows redistributed to A and B

After failover:
  Node A: flow-0, flow-2, flow-3
  Node B: flow-1, flow-4, flow-5
```

### Horizontal Scaling

You don't need Kubernetes. Rust-Red clustering works with plain VPS instances.

#### Option A: Manual VPS Scaling

1. Build and copy the binary to the new node
2. Create config with existing peers listed
3. Start the node — gossip auto-discovery handles the rest
4. The leader automatically distributes flows to include the new node

No need to restart existing nodes.

#### Option B: Docker Compose

```yaml
version: "3.8"
services:
  node-a:
    image: rustred/rust-red:latest
    ports: ["1888:1888", "7980:7980"]
    volumes: ["./configs/node-a.toml:/etc/rust-red/config.toml"]
    command: ["-c", "/etc/rust-red/config.toml"]
  node-b:
    image: rustred/rust-red:latest
    ports: ["1888:1888", "7980:7980"]
    volumes: ["./configs/node-b.toml:/etc/rust-red/config.toml"]
    command: ["-c", "/etc/rust-red/config.toml"]
```

#### Option C: Kubernetes

```bash
kubectl scale deployment rust-red --replicas=5
```

Uses DNS-based discovery via headless service. New pods auto-join the cluster.

| Approach | Best For | Scaling | Complexity |
|----------|----------|---------|------------|
| Manual VPS | 2-5 nodes | Edit config, start node | Low |
| Docker Compose | Single-host cluster | `docker compose up --scale` | Low |
| Kubernetes | Large deployments, auto-healing | `kubectl scale` | High |

### Running as a systemd Service

```ini
# /etc/systemd/system/rust-red.service
[Unit]
Description=Rust-Red Flow Engine
After=network.target

[Service]
Type=simple
User=rustred
Group=rustred
WorkingDirectory=/opt/rust-red
ExecStart=/usr/local/bin/rust-red run --headless -c /etc/rust-red/config.toml
Restart=on-failure
RestartSec=5
TimeoutStopSec=15
KillSignal=SIGINT

[Install]
WantedBy=multi-user.target
```

```bash
sudo useradd -r -s /bin/false rustred
sudo systemctl enable rust-red
sudo systemctl start rust-red
```

### Feature Flags

```bash
cargo build --release                          # Single-node
cargo build --release --features cluster       # With clustering
cargo build --release --features full          # Everything
```

---

## Test Suite

Rust-Red includes a comprehensive Node-RED compatibility test suite with **259 passing tests** covering flow execution, node behavior, and edge cases — all ported from the official Node-RED spec tests.

```bash
# Run the full compatibility suite
cargo test --package rust-red-core --features internal-testing -- --test-threads=1

# Run all tests including cluster
cargo test --features cluster --test cluster_integration
```

Test categories include: flow lifecycle, subflows, inject, switch, change, range, template, filter (RBE), delay, trigger, CSV/JSON/XML/YAML parsing, MQTT, HTTP, TCP, UDP, WebSocket, file I/O, and more.

Refer to [REDNODES-SPECS-DIFF.md](tests/REDNODES-SPECS-DIFF.md) for the detailed spec compliance matrix.

---

## Project Status

### Features

- [x] Complete Node-RED editor interface (web UI)
- [x] Flow design, editing, and deployment from browser
- [x] Real-time debug panel and status monitoring
- [x] Node palette with all supported nodes
- [x] Import/Export flows
- [x] Clustering & high availability with gossip protocol
- [x] Authentication & RBAC (JWT, API keys, user management)
- [x] AI assistant integration (multi-provider chat, suggestions)
- [x] Dashboard node with real-time data widgets
- [x] Audit logging subsystem
- [x] OpenTelemetry tracing (optional)
- [x] Frontend plugin registry
- [x] Flow versioning with rollback and diff
- [x] Industrial protocol nodes: Modbus, OPC-UA, BACnet
- [x] Database nodes: PostgreSQL, TimescaleDB, MSSQL, SQLite, InfluxDB
- [x] WASM plugin host/sdk
- [x] Built-in MQTT broker
- [x] 259+ Node-RED compatibility tests passing

### Node-RED Features Roadmap

- [x] Flow
- [x] Sub-flow
- [x] Group
- [x] Environment Variables
- [ ] Context
    - [x] Memory storage
    - [ ] Local file-system storage
- [ ] RED.util (WIP)
    - [x] `RED.util.cloneMessage()`
    - [x] `RED.util.generateId()`
- [x] Plug-in subsystem
- [ ] JSONata

### Supported Nodes

The heavy check mark (:heavy_check_mark:) indicates the node has passed the Node-RED spec test.

- **Common nodes**: Inject, Debug, Complete, Catch, Status, Link In/Call/Out, Comment, Unknown, Junction, Dashboard Data, Global Config, Group
- **Function nodes**: Function (QuickJS), Switch, Change, Range, Template, Delay, Trigger, Exec, Filter (RBE)
- **Network nodes**: MQTT In/Out, HTTP In/Out/Request, WebSocket (Listener/Client/In/Out), TCP In/Out/Get, UDP In/Out, TLS, HTTP Proxy
- **Sequence nodes**: Split, Join, Sort, Batch
- **Parser nodes**: CSV, JSON, XML, YAML, HTML
- **Storage nodes**: File, File In, Watch
- **Database nodes**: PostgreSQL, TimescaleDB, MSSQL, SQLite, InfluxDB
- **Industrial nodes**: Modbus (Read/Write/Flex/Server), OPC-UA (Read/Write), BACnet (Read/Write)

## Contribution

We welcome contributions!

- **Bug reports** and feature requests
- **Documentation** improvements
- **Code contributions** and new node implementations
- **Testing** on different platforms

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.
