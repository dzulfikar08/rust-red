# RustRED Benchmarking Standards

**Version**: 1.0
**Date**: 2026-05-24
**Affects**: RRD-7, RRD-9, RRD-17, RRD-21

## 1. Hardware Baselines

### Reference Hardware

| Tier | Device | CPU | RAM | Storage | Role |
|------|--------|-----|-----|---------|------|
| **Tier 1 (Desktop)** | AMD Ryzen 9 5950X | 16c/32t, 3.4 GHz | 64 GB DDR4 | NVMe SSD | Development baseline |
| **Tier 2 (Laptop)** | Apple M2 MacBook Pro | 8c, 3.5 GHz | 16 GB LPDDR5 | NVMe SSD | Common developer machine |
| **Tier 3 (Edge)** | Raspberry Pi 4 | 4c, 1.5 GHz Cortex-A72 | 8 GB LPDDR4 | SD Card | IoT/edge baseline |
| **Tier 4 (Industrial)** | NVIDIA Jetson Xavier NX | 6c, 1.9 GHz Carmel | 8 GB LPDDR4x | NVMe SSD | Industrial edge baseline |

All benchmark results must be reported for Tier 1 and Tier 3 at minimum.

### Software Environment

- **OS**: Ubuntu 22.04 LTS (Tier 1, 3, 4); macOS 14 (Tier 2)
- **Rust**: Latest stable (pin version in CI)
- **Node-RED baseline**: Latest LTS (for comparison benchmarks)
- **Runtime**: Release build (`cargo build --release`), all optimizations enabled

## 2. Measurement Methodology

### Warmup

- **Warmup iterations**: 50 iterations discarded before measurement
- **Warmup purpose**: CPU branch prediction, cache population, JIT compilation (for Node-RED comparison)

### Measurement

- **Iteration count**: Minimum 1000 iterations per benchmark
- **Statistical method**: Report **95th percentile (p95)** as primary metric, plus mean, median, p99
- **Confidence interval**: 95% confidence, report ± range
- **Outlier handling**: Remove iterations >3σ from mean (document count removed)

### Metrics

| Metric | Unit | Tool |
|--------|------|------|
| Latency (message throughput) | messages/sec | Custom harness |
| Latency (per-operation) | microseconds (µs) | `std::time::Instant` / `quanta` |
| Memory usage (peak RSS) | MB | `/usr/bin/time -v` or `jemalloc` stats |
| Memory usage (steady-state) | MB | `valgrind --tool=massif` |
| Binary size | MB | `ls -la` on release binary |
| Startup time | milliseconds | Time from process spawn to "ready" log |
| CPU usage | % single core | `top` / `perf` |

### System State

- Kill unnecessary background processes before benchmarking
- Disable CPU frequency scaling: `echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor` (Linux)
- Disable turbo boost for reproducibility (optional, document choice)
- Run on AC power, not battery

## 3. Benchmark Scenarios

### Scenario A: Simple Passthrough (Baseline)

```
Inject → [passthrough] → Debug
```
- Measures: raw message throughput, per-hop latency
- Message rate: 100k messages burst, then sustained 10k/sec for 60s
- Target: >500k msgs/sec (Tier 1), >50k msgs/sec (Tier 3)

### Scenario B: Complex Routing

```
Inject → Switch (4 outputs) → [4× Change nodes] → Join → Debug
```
- Measures: routing overhead, message cloning, join/aggregation
- Message rate: 10k msgs/sec sustained for 60s
- Target: >100k msgs/sec (Tier 1), >10k msgs/sec (Tier 3)

### Scenario C: MQTT Fan-Out

```
MQTT Subscribe → [10× Function nodes] → [10× MQTT Publish]
```
- Measures: MQTT throughput, Function node execution overhead
- Uses embedded MQTT broker (RRD-27) for consistency
- Message rate: 50k msgs/sec input, measure output rate
- Target: >100k msgs/sec (Tier 1), >20k msgs/sec (Tier 3)

### Scenario D: WASM Plugin Pipeline

```
Inject → [WASM Echo Plugin × 5 chained] → Debug
```
- Measures: WASM boundary crossing overhead, serialization cost
- Message rate: 10k msgs/sec sustained for 60s
- Target: >50k msgs/sec (Tier 1), >5k msgs/sec (Tier 3)

### Scenario E: JSONata Expression Evaluation

```
Inject → Change (JSONata: $sum(payload.values[*].amount)) → Debug
```
- Measures: JSONata parser + evaluator performance
- Input: 1000-element array, 100 expressions per iteration
- Target: <1ms per expression (p95)

### Scenario F: MessagePack Serialization

```
Host → serialize(WasmMessage) → deserialize → verify
```
- Measures: Serde + MessagePack round-trip overhead
- Message sizes: 100B, 1KB, 10KB, 100KB
- Target: <100µs per round-trip for messages <10KB (p95)

### Scenario G: Startup & Memory

- Measures: Cold start time, peak RSS, steady-state RSS
- Flow: Load a 100-node flow, deploy, process first message
- Compare against Node-RED with equivalent flow
- Target: <500ms cold start, <50 MB steady-state RSS (Tier 1)

## 4. Specific Issue Targets

| Issue | Metric | Target | Methodology |
|-------|--------|--------|-------------|
| RRD-7 (MessagePack) | Serialization round-trip | <100µs for <10KB messages | Scenario F, Tier 1 p95 |
| RRD-9 (MQTT nodes) | Throughput | 100k+ msgs/sec | Scenario C, Tier 1 p95 |
| RRD-17 (JSONata) | Per-expression eval | <1ms | Scenario E, Tier 1 p95 |
| RRD-21 (Perf vs Node-RED) | Memory | 10× less than Node-RED | Scenario G, Tier 1 RSS |
| RRD-21 (Perf vs Node-RED) | Throughput | 2×+ Node-RED throughput | Scenarios A-C, Tier 1 |
| RRD-21 (Perf vs Node-RED) | Startup | 5× faster than Node-RED | Scenario G, cold start |
| RRD-21 (Perf vs Node-RED) | Binary | <25 MB total | `ls -la` on release binary |

All targets are **aspirational goals**, not hard gates. Report actual numbers alongside targets. If a target is not met, document the gap and proposed mitigation.

## 5. CI Integration

Benchmarks run automatically on:
- Every push to `main` (scenarios A, E, F only — fast benchmarks)
- Every PR (full suite on Tier 1)
- Weekly scheduled run (full suite on all tiers)
- Before every release (full suite + comparison against previous release)

Results are published as:
- GitHub Actions artifact (JSON + markdown report)
- Comment on PR with comparison against base branch
- Historical trend chart (stored in repo `docs/benchmarks/history/`)
