# ADR-003: Sandboxing Strategy for Untrusted Code

**Status**: Proposed
**Date**: 2026-05-24
**Affects**: RRD-30 (Security), RRD-20 (Function node), RRD-14 (Exec node), RRD-4 (WasmNodeShim)

## Context

RustRED must execute untrusted code from multiple sources:
1. **Function nodes** — User-written JavaScript
2. **WASM plugins** — Third-party compiled modules
3. **Exec nodes** — Arbitrary system commands (highest risk)
4. **Template nodes** — User-written templates with expression evaluation

The sandboxing strategy must align with the "Secure by Default" principle (RRD-30) and support RBAC (RRD-23) where different roles have different code execution permissions.

## Decision

**Three-tier sandboxing model** based on code origin and trust level.

### Tier 1: WASM Sandbox (Plugins & Function Nodes)

```
┌─────────────────────────────────────┐
│ RustRED Host Process                │
│  ┌──────────────────────────────┐   │
│  │ wasmtime Instance            │   │
│  │  ┌────────────────────────┐  │   │
│  │  │ QuickJS (Function)     │  │   │
│  │  │ - No filesystem access │  │   │
│  │  │ - No network access    │  │   │
│  │  │ - Memory limit: 64 MB  │  │   │
│  │  │ - CPU time limit: 30s  │  │   │
│  │  └────────────────────────┘  │   │
│  │  ┌────────────────────────┐  │   │
│  │  │ WASM Plugin            │  │   │
│  │  │ - Capability-based     │  │   │
│  │  │ - Resource limits      │  │   │
│  │  └────────────────────────┘  │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
```

- **Technology**: wasmtime with Wasmtime's `WasmConfig` for limits
- **Memory**: 64 MB default per instance, configurable per node
- **CPU**: 30-second wall-clock timeout, configurable
- **Network**: Denied by default; granted via explicit capability flag
- **Filesystem**: Denied by default; granted via explicit capability flag
- **Syscalls**: Filtered via wasmtime's WASI configuration

### Tier 2: Restricted Process (Exec Nodes)

- Exec nodes are **disabled by default** — admin must explicitly enable in settings
- When enabled, exec commands run in a restricted subprocess:
  - **Linux**: `namespace` + `cgroup` + `seccomp` filter (via `bpf` crate)
  - **macOS**: `sandbox-exec` profile
  - **Fallback**: Process-level timeout + resource monitoring without OS-level sandbox
- Resource limits: configurable CPU time, memory, and output size
- Working directory: restricted to a configurable sandbox directory
- Network access: optional, off by default

### Tier 3: Expression Sandbox (Template/JSONata)

- JSONata expressions and template rendering run in-memory with:
  - No side effects (pure evaluation)
  - Input size limits (max 10 MB input)
  - Recursion depth limit (max 100 levels)
  - Execution time limit (max 5 seconds)
- No filesystem, network, or process access possible by design

### RBAC Integration

| Role | Tier 1 (WASM) | Tier 2 (Exec) | Tier 3 (Expressions) |
|------|---------------|---------------|---------------------|
| Admin | Full (can adjust limits) | Can enable/disable | Full |
| Editor | Default limits | Disabled | Full |
| Viewer | None | None | Read-only evaluation |
| Custom | Per-role config | Per-role config | Per-role config |

## Consequences

### Positive
- Defense-in-depth: WASM sandbox + OS-level sandboxing
- Consistent security model across all code execution paths
- RBAC-aware: different roles get different sandbox capabilities
- No single sandbox escape compromises the host

### Negative
- wasmtime adds ~5 MB to binary (already included for WASM plugin system)
- Exec node sandboxing is OS-specific (Linux gets strongest sandboxing)
- Performance overhead: ~5-10% for WASM sandbox, ~15% for process sandboxing
- cgroup/namespace setup requires root or specific Linux capabilities

### Mitigations
- Document minimum OS requirements for full sandboxing
- Graceful degradation: warn (not fail) when OS-level sandboxing unavailable
- Benchmarking targets include sandboxing overhead
