# ADR-004: Storage Backend for Flow Versioning

**Status**: Proposed
**Date**: 2026-05-24
**Affects**: RRD-31 (Flow versioning/collaboration), RRD-19 (Filesystem context store)

## Context

Flow versioning (RRD-31) and the filesystem context store (RRD-19) need a storage backend that supports:
1. Storing flow version history with configurable retention
2. Efficient diffing between versions
3. Crash recovery (atomic writes)
4. Local-first operation (no external DB required for single-instance deployments)
5. Scale-up path for clustering (RRD-25)

## Decision

**SQLite as the primary local storage backend**, with PostgreSQL as the optional clustered backend.

### Rationale

| Criteria | SQLite | Filesystem (JSON files) | PostgreSQL | Sled/Redb |
|----------|--------|------------------------|------------|-----------|
| Zero-config | Yes | Yes | No (requires setup) | Yes |
| Atomic writes | Yes (WAL mode) | Manual (rename trick) | Yes | Yes |
| Concurrent reads | Excellent | Good (OS-level) | Excellent | Good |
| Version history queries | SQL (flexible) | Manual implementation | SQL (flexible) | Manual |
| Binary size impact | +3 MB (bundled) | 0 | 0 (external) | +2 MB |
| Crash recovery | WAL automatic | Fragile | WAL/ PITR | Manual |
| Diff support | SQL + application-level | Application-level | SQL + application-level | Application-level |
| Clustering path | Can export to PG | Manual replication | Native | Manual |

### Storage Schema

```sql
-- Flow versioning
CREATE TABLE flow_versions (
    id TEXT PRIMARY KEY,
    flow_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    author_id TEXT,
    author_name TEXT,
    description TEXT,
    snapshot BLOB NOT NULL,        -- Compressed automerge document
    delta BLOB,                     -- Incremental changes from previous version
    created_at TIMESTAMP NOT NULL,
    UNIQUE(flow_id, version)
);

CREATE INDEX idx_flow_versions_flow ON flow_versions(flow_id, version DESC);

-- Context store
CREATE TABLE context_store (
    scope TEXT NOT NULL,             -- 'node', 'flow', 'global'
    scope_id TEXT NOT NULL,          -- Node ID, flow ID, or 'global'
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    PRIMARY KEY (scope, scope_id, key)
);

-- Change log
CREATE TABLE changelog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    flow_id TEXT NOT NULL,
    user_id TEXT,
    action TEXT NOT NULL,            -- 'deploy', 'rollback', 'branch', 'merge'
    description TEXT,
    metadata TEXT,                   -- JSON metadata
    created_at TIMESTAMP NOT NULL
);
```

### Configuration

```toml
[storage]
# Local mode (default)
backend = "sqlite"
path = "./data/rustred.db"

# Clustered mode (optional)
# backend = "postgresql"
# url = "postgresql://rustred:password@localhost/rustred"
```

### Migration Path

1. **Single instance**: SQLite (zero config, bundled)
2. **Small cluster**: SQLite + file sync (rsync/Syncthing)
3. **Production cluster**: PostgreSQL with streaming replication
4. **Cloud/enterprise**: PostgreSQL + S3 for flow snapshot archival

## Consequences

### Positive
- Zero-config for single-instance (SQLite just works)
- Atomic writes via WAL mode — no corruption on crash
- SQL makes version history queries trivial
- Clear upgrade path to PostgreSQL for clustering
- Delta storage keeps version history efficient

### Negative
- SQLite adds ~3 MB to binary size
- SQLite write concurrency limited (one writer at a time) — acceptable for single-instance
- Migration from SQLite to PostgreSQL requires tooling
- automerge document snapshots can be larger than raw JSON

### Mitigations
- Use SQLite WAL mode for best concurrent read performance
- Provide `rustred migrate` CLI command for SQLite → PostgreSQL migration
- Periodic compaction of automerge documents to reduce snapshot size
- Document retention policy (default: keep last 100 versions per flow)
