# ADR-001: JavaScript Runtime for Function Node

**Status**: Proposed
**Date**: 2026-05-24
**Affects**: RRD-20 (Function node), RRD-30 (Security)

## Context

The Function node is one of the most critical nodes for Node-RED compatibility. It executes arbitrary JavaScript code provided by users, requiring a sandboxed JS runtime. The choice of runtime impacts binary size, memory usage, ES feature support, security posture, and startup time.

RustRED targets a ~22 MB binary with minimal resource usage — the JS runtime must not undermine this goal.

## Decision

**Use QuickJS as the primary JS runtime**, with wasmtime-based isolation as the sandbox boundary.

### Rationale

| Criteria | QuickJS | V8 | Deno Core | Boa |
|----------|---------|----|-----------|----|
| Binary size impact | +2-3 MB | +30-50 MB | +40-60 MB | +5-8 MB |
| Memory per instance | 256 KB–2 MB | 10-50 MB | 15-60 MB | 1-5 MB |
| ES2023 support | Partial (ES2020+) | Full | Full | Partial |
| Startup time | <5 ms | 50-200 ms | 100-300 ms | <10 ms |
| Embedding in Rust | via libquickjs-sys | complex C++ FFI | via deno_core crate | Pure Rust |
| Sandboxing | Manual (context isolation) | Strong (isolate model) | Strong (isolate model) | Manual |
| Node-RED compat | High (sufficient for Function node JS) | Highest | Highest | Medium |

### Key factors

1. **Binary size**: QuickJS adds ~2-3 MB vs V8's 30-50 MB. The 22 MB target is achievable with QuickJS, impossible with V8.
2. **Multi-tenancy**: RBAC (RRD-23) requires running multiple Function nodes per user. QuickJS instances are cheap (KBs each), V8 isolates are expensive (MBs each).
3. **Security**: Function nodes run inside a WASM sandbox (via wasmtime) with QuickJS embedded. The double isolation (WASM sandbox + QuickJS context) provides defense in depth. V8's stronger sandbox is unnecessary given the WASM outer layer.
4. **Node-RED compat**: QuickJS supports ES2020+ which covers all patterns used in Node-RED Function nodes (arrow functions, async/await, destructuring, template literals).
5. **Rust integration**: `libquickjs-sys` + `quickjs-rs` provide mature Rust bindings.

## Consequences

### Positive
- Binary stays under 25 MB with Function node support
- Can run 100+ concurrent Function node instances on modest hardware
- Defense-in-depth security with WASM + QuickJS isolation
- Fast startup (important for serverless/edge deployments)

### Negative
- Some advanced ES2023+ features not available (top-level await, some Promise methods)
- QuickJS performance is ~2-5x slower than V8 for CPU-bound JS — acceptable since Function nodes are typically I/O-bound
- QuickJS has a smaller community than V8; bug fixes may be slower

### Mitigations
- Document ES feature support matrix for Function node authors
- For enterprise users needing V8, offer a feature-gated `v8_runtime` flag
- Contribute upstream to quickjs-rs for any needed improvements
