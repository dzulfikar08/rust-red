# ADR-002: Real-Time Collaboration Protocol

**Status**: Proposed
**Date**: 2026-05-24
**Affects**: RRD-31 (Flow versioning/collaboration), RRD-23 (RBAC/auth)

## Context

Flow versioning and multi-user collaboration (RRD-31) requires a real-time sync protocol so multiple users can edit flows simultaneously without conflicts. This must integrate with the RBAC system (RRD-23) for permission checks.

The three main approaches are:
1. **Operational Transformation (OT)** — Used by Google Docs. Complex to implement correctly.
2. **Conflict-free Replicated Data Types (CRDTs)** — Used by Figma, Notion. Eventually consistent by design.
3. **Last-Write-Wins (LWW) with locking** — Simplest but poorest UX; causes lost work.

## Decision

**Use CRDTs (via the `automerge` Rust crate) for flow state synchronization**, with WebSocket-based real-time transport.

### Rationale

| Criteria | OT | CRDTs | LWW + Locking |
|----------|----|----|---------------|
| Conflict resolution | Server-mediated, complex | Automatic, mathematically correct | None (last write wins) |
| Offline support | Poor (requires server) | Excellent (merge on reconnect) | None |
| Implementation complexity | Very high | Medium (automerge handles it) | Low |
| Lost work risk | Low | None (by design) | High |
| Performance with many users | Degrades linearly | Scales well | Constant |
| Edge/offline editing | Requires central server | Works fully offline | Broken |

### Architecture

```
Client A ←──WebSocket──→ Server ←──WebSocket──→ Client B
   │                       │                       │
   └── automerge doc ──────┘─────── automerge doc ──┘
         (CRDT sync messages, not full state)
```

1. Each flow is an Automerge document
2. Changes are represented as CRDT operations (add node, move node, add wire, etc.)
3. Server relays sync messages between connected clients
4. Server persists the document to storage backend
5. On reconnect, clients sync incremental changes (not full re-download)

### Flow CRDT Schema

```
Flow Document {
  nodes: Map<NodeId, NodeState>,      // Each node is independently editable
  wires: Map<WireId, WireState>,      // Wires between node ports
  groups: Map<GroupId, GroupState>,   // Visual groups
  metadata: Map<String, Value>,       // Flow-level metadata
  version: Counter,                   // Monotonically increasing version
  changelog: List<ChangeEntry>        // Append-only change history
}

NodeState {
  type: String,
  position: { x: f64, y: f64 },      // LWW register for position
  config: Map<String, Value>,         // Node-specific config
  label: LWW<String>,                 // Last-write-wins for label
}
```

### RBAC Integration

- Server validates each CRDT operation against the user's role before relaying
- Viewers receive read-only sync (no mutation operations accepted)
- Custom roles can have granular permissions (e.g., can move nodes but not edit config)

## Consequences

### Positive
- Zero lost work — conflicts merge automatically
- Full offline editing support (critical for edge/industrial use cases)
- Mathematically correct merging (automerge is battle-tested)
- Efficient sync (only deltas transmitted, not full flow JSON)
- Works over unreliable connections (reconnects gracefully)

### Negative
- automerge adds ~500 KB to binary size
- CRDT sync messages add ~10-20% overhead vs raw JSON edits
- Undo/redo requires careful design (must reverse CRDT operations)
- Binary size of stored flow documents is larger than plain JSON

### Mitigations
- Use automerge's compression for sync messages
- Implement flow document compaction periodically (automerge supports this)
- Undo/redo via local operation log (not CRDT-level undo)
