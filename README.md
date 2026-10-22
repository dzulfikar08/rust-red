# RustRED

[![CI]][actions]
[![GitHub Release]][releases]
[![License]][license]

[CI]: https://img.shields.io/github/actions/workflow/status/dzulfikar08/rust-red/ci.yml?branch=master&label=CI
[actions]: https://github.com/dzulfikar08/rust-red/actions/workflows/ci.yml
[GitHub Release]: https://img.shields.io/github/v/release/dzulfikar08/rust-red?include_prereleases
[releases]: https://github.com/dzulfikar08/rust-red/releases
[License]: https://img.shields.io/badge/license-Apache--2.0-blue
[license]: LICENSE

## Fork notice

**RustRED is a fork of [EdgeLinkd](https://github.com/oldrev/edgelinkd) by Li Wei, licensed under Apache-2.0.** The runtime core, the Node-RED compatible node implementations and most of the repository history come from EdgeLinkd. The upstream copyright is kept in [LICENSE](LICENSE) and [NOTICE](NOTICE). This fork is not affiliated with or endorsed by the EdgeLinkd authors or the Node-RED project.

What this fork adds on top of EdgeLinkd (see the Git history for details):

- Renamed crates and binary (`rust-red`).
- A Modbus server node and an embedded MQTT broker node.
- Clustering with gossip membership and flow failover (opt-in `cluster` feature).
- An AI assistant, authentication/user management, and a dashboard data node.
- A React based editor experiment in `crates/frontend` (not part of the shipped binary).
- CI, release builds and benchmarks.

The fork does not track EdgeLinkd releases. Upstream fixes are ported by hand, see [Ported from EdgeLinkd](#ported-from-edgelinkd) below.

## Overview

RustRED is a Node-RED compatible flow runtime written in Rust. It runs flows from `flows.json` and serves the Node-RED editor from a directory next to the binary. It can also run headless.

Only the `function` node runs JavaScript (through the QuickJS interpreter); the other nodes are native Rust.

### Measured footprint

These are measurements from one Linux x86_64 machine (12 cores), not a general guarantee. Resident memory is `VmRSS` from `/proc/<pid>/status`.

| Build | Binary size (stripped, LTO, opt-level z) | Idle RSS, empty flow, 30 s, web server on |
|-------|------------------------------------------|-------------------------------------------|
| default features | 16.3 MB | 15.0 MB |
| `--features full` | 24.2 MB | 17.8 MB |

Modbus TCP to PostgreSQL comparison (10 holding registers polled every 500 ms, a function node builds the INSERT, one row per poll, about 2 rows/s on both sides, verified by row counts). Method: 30 s warm-up, then 60 s sampled once per second; CPU is (utime + stime) / wall-clock.

| Runtime | Average RSS | CPU |
|---------|-------------|-----|
| RustRED (full build) | 20.4 MB (max 20.5 MB) | 0.20% |
| Node-RED (Node v26.11.1, node-red-contrib-modbus, node-red-contrib-postgresql) | 126 MB (max 149 MB) | 0.71% |

That is roughly 6x less memory in this one test. The RustRED flow used RustRED's own node types, the Node-RED flow used the contrib nodes. Since 0.2.1 the contrib flow runs on RustRED unchanged, see below. Simulator: pymodbus 3.6.9; database: `postgres:16-alpine` in Docker.

### Node-RED flow compatibility

Flows exported from Node-RED run as they are for core nodes and for these three packages. Each row was run end to end, see [scripts/smoke](scripts/smoke).

| Node-RED export | Served by | Notes |
|-----------------|-----------|-------|
| core nodes (inject, function, change, switch, debug and similar) | the same nodes | |
| `mqtt-broker`, `mqtt in`, `mqtt out` (stock) | `mqtt-broker`, `mqtt in`, `mqtt out` | `broker`, `port`, `usetls`, `clientid`, `keepalive`, `cleansession`, `willTopic` and the user and password are used |
| `modbus-client`, `modbus-read`, `modbus-getter`, `modbus-write`, `modbus-flex-getter`, `modbus-flex-write` (`node-red-contrib-modbus`) | `modbus-config`, `modbus read`, `modbus write`, `modbus-flex-getter`, `modbus-flex-writer` | two outputs and `msg` shapes as in the contrib nodes |
| `postgresql`, `postgreSQLConfig` (`node-red-contrib-postgresql`) | `postgres-query`, `postgres-config` | Mustache in the query, `msg.params`, `msg.queryParameters`, `split` |

How the three packages differ from Node-RED:

- **Credentials.** The user and password are read from a `credentials` object on the config node, from plain `user` and `password` properties, or from environment variables when the field type is `env` (PostgreSQL). The encrypted `flows_cred.json` is not read, so a flow exported without credentials needs them added by one of these ways.
- **A broken config node** is logged as an error and disabled. Nodes that use it report that it is missing. The rest of the flows keep running.
- **Node ids** do not have to be hex. Ids such as `p1` are accepted on both flow nodes and config nodes.
- **MQTT.** Every `mqtt in` and `mqtt out` node opens its own connection, where Node-RED shares one. A configured client id gets a suffix per node for that reason. As in Node-RED, a topic set on `mqtt out` wins over `msg.topic`. `protocolVersion` 5 is supported: the node connects with MQTT 5 and carries response topic, correlation data, content type, message expiry and user properties both ways. The embedded `mqtt-broker` node is MQTT 3.1.1 only, so use an external broker (for example Mosquitto 2) for MQTT 5. Birth and close messages are not sent.
- **Modbus.** `msg.payload` holds the values (an array of numbers, or booleans for coils) and `msg.responseBuffer` holds `{data, buffer}`. Only TCP and RTU serial transports exist, not UDP or ASCII. `msg.modbusRequest` and `msg.input` hold numbers where Node-RED holds the strings from the editor. The nodes `modbus-flex-connector`, `modbus-flex-sequencer`, `modbus-flex-fc`, `modbus-response`, `modbus-queue-info`, `modbus-io-config` and the I/O file options are not available.
- **PostgreSQL.** `numeric` columns come back as text, like node-postgres, and `bigint` beyond 2^53 as text. Timestamps come back as ISO 8601 text, not as Date objects. A query with several statements runs, but returns every column as text. `ssl` is supported (`true`, or `{"rejectUnauthorized": false}`).
- Context storage to the local file system, and part of `RED.util`, are not implemented (see roadmap below).

### Release builds

GitHub Releases ship the **default feature set** for Linux x86_64/aarch64 and macOS x86_64/aarch64, as `tar.gz` archives with the binary, the editor files (`ui_static/`) and a SHA-256 checksum. The `full` feature set (adds clustering, WASM plugins and `rqjs_bindgen`) is built from source with `cargo build --release --features full`.

## Quick Start

### Download a release

Download the archive for your platform from [Releases](https://github.com/dzulfikar08/rust-red/releases), verify it against `checksums-sha256.txt`, then:

```bash
tar xzf rust-red-*.tar.gz
cd rust-red-*/
./rust-red run flows.json
```

Open [http://127.0.0.1:1888](http://127.0.0.1:1888).

### Or build from source

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

**Prerequisites**: Rust 1.80 or later, and Node.js with npm (the build compiles the Node-RED editor from the `3rd-party/node-red` submodule; without the submodule the binary still builds but has no editor files).

```bash
cargo build --release
```

**Windows**: not shipped in releases and not tested by CI. Building needs `patch.exe` in PATH (included with Git) and MSVC.

**Release and CI platforms**: Linux x86_64 and aarch64, macOS x86_64 and aarch64.

`cargo install --git https://github.com/dzulfikar08/rust-red` installs only the binary. The editor files are generated in `target/ui_static` during a repository build and are looked up next to the executable, so use a release archive or a repository build if you want the editor.

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

# Node-RED compatibility test suite
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

Node-RED runs as a single process; if it crashes, everything stops. RustRED's clustering lets several instances share the flows. Clustering is only compiled in with `--features cluster`, and it has not been benchmarked against Node-RED here.

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

RustRED includes a Node-RED compatibility test suite ported from the Node-RED spec tests, covering flow execution and node behavior. CI runs it on every push (`cargo nextest run --workspace --features default,internal-testing`).

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
- [x] Clustering with gossip membership (`cluster` feature, not in the default build)
- [x] Authentication and user management code (experimental)
- [x] AI assistant integration (`ai` feature, experimental)
- [x] Dashboard node with real-time data widgets
- [x] Audit logging subsystem (experimental)
- [x] OpenTelemetry tracing (optional, `otel` feature)
- [x] Frontend plugin registry
- [x] Flow versioning with rollback and diff (experimental)
- [x] Industrial protocol nodes: Modbus, OPC-UA, BACnet
- [x] Database nodes: PostgreSQL, TimescaleDB, MSSQL, SQLite, InfluxDB
- [x] WASM plugin host/sdk (`wasm_plugins` feature, not in the default build)
- [x] Built-in MQTT broker node

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
- [x] JSONata (via the pure-Rust `jsonata-core` engine)
    - [x] `$flowContext()`, `$globalContext()`, `$env()`, `$clone()`, `$I`/`$N` bindings
    - [x] `change` / `switch` / `inject` properties, and environment variables
    - [ ] `$moment()`: an expression calling it fails with an error instead of a value

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

## Changelog

### 0.3.0

#### Ported from EdgeLinkd

Each item is a hand port of an upstream [EdgeLinkd](https://github.com/oldrev/edgelinkd) commit, with tests. The upstream commit hash is in the Git message of the matching commit here (`port(upstream <sha>): ...`).

| Upstream | What changed |
|----------|--------------|
| `a165109` | `sort` reads the editor's `seqKey`, `msgKey` and `targetType`. A message group no longer always sorts by `payload`. |
| `2990687`, `c9cf68e` | New `runtime.flow.node_message_buffer_max_length` setting (Node-RED's `nodeMessageBufferMaxLength`). `sort` drops the oldest incomplete group when the buffer is full. A redeploy from the web UI keeps the engine configuration. |
| `27bc07e` | `join` reads `joiner`, takes its count from `msg.parts` in auto mode and from the node in manual mode, flushes on `msg.complete`, and keeps array holes. |
| `d446881`, `544c68b` | `batch` clones messages emitted with overlap, runs interval mode on a real timer, supports `allowEmptySequence`, and reports overflow as `batch.too-many`. |
| `90403a7` | `delay` calls `done()` at the right time and supports the queue and timed rate modes. |
| `9b1fa42`, `f93287a` | `exec` keeps trailing newlines, returns binary output as a buffer, and waits for stdout and stderr so output is not dropped. |
| `456da27` | `tcp request` in time mode collects the whole response. |
| `1ed579a` | `env.get()` in the function node looks the name up literally, without interpolation. |
| `f3a80f5` | Fewer panics and silent misconfigurations in the kernel: string `_msgid`, zero queue capacity rejected, nested property set on a non-object returns an error. |
| `35075ea` | `trigger` supports the missing property types (JSONata, binary, env, ISO date). |
| `b1867f7` | A node with several output ports sends to all ports at the same time, so one full port does not block the others. |
| `e7f780a`, `7e92f0b` | `file` and `file in` support `hex`, `ucs2` and `utf-16be` encodings. |
| `5e14079` | MQTT 5 in `mqtt in` and `mqtt out` (`protocolVersion` 5 is really used). |
| `f410c40` | JSONata runs on the pure-Rust `jsonata-core` engine, with `$flowContext()`, `$globalContext()`, `$env()`, `$clone()`, `$I` and `$N`. |
| `84614a2` | `html` node: `attr` and `compl` returns, `outproperty`, inner HTML. |

Checked and not needed: `218db7d` (our `debug` node has no JSONata error state to fix).

## Contribution

We welcome contributions!

- **Bug reports** and feature requests
- **Documentation** improvements
- **Code contributions** and new node implementations
- **Testing** on different platforms

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.
