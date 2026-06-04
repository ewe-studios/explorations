---
title: Worker Types — Registry, OCI, and Local Workers
---

# Worker Types — Registry, OCI, and Local Workers

**iii-worker supports three worker source types, each with different download, extraction, and execution paths.** This document covers all three and how the CLI routes between them.

## Three Worker Types

```mermaid
flowchart TD
    A[User types: iii worker add <name>] --> B{parse_source_for_cli}
    
    B -->|is_local_path| C[WorkerSource::Local]
    B -->|contains '/' or ':'| D[WorkerSource::Oci]
    B -->|registry name[@version]| E[WorkerSource::Registry]
    
    C --> F[handle_local_add]
    D --> G[handle_bundle_add or handle_binary_add]
    E --> H[fetch_worker_info → resolve type]
    
    H -->|binary worker| I[handle_binary_add]
    H -->|bundle worker| J[handle_bundle_add]
    H -->|engine worker| K[engine-specific path]
    
    F --> L[start_local_worker]
    G --> M[boot VM via worker_manager]
    I --> M
    J --> M
```

## WorkerSource Enum

Source: `core/types.rs`

```rust
pub enum WorkerSource {
    /// Registry worker: "pdfkit", "pdfkit@1.0.0"
    Registry { name: String, version: Option<String> },
    /// OCI image: "ghcr.io/org/worker:tag"
    Oci { reference: String },
    /// Local path: "./my-worker", "/path/to/worker"
    Local { path: String },
}
```

## Registry Workers

The default path. Workers are fetched from the iii workers registry API:

```mermaid
flowchart TD
    A[GET /workers/{name}] --> B{worker type?}
    B -->|binary| C[GET /download/{name}]
    B -->|bundle| D[GET /download/{name}]
    B -->|engine| E[Built into engine]
    C --> F[Extract tarball]
    D --> G[Extract archive + build]
    F --> H[Start VM]
    G --> H
```

1. **Resolve** — `GET /workers/{name}` → worker type (binary/bundle/engine), version, URLs
2. **Download** — `GET /download/{name}?version=X` → binary tarball or bundle archive
3. **Extract** — To `~/.iii/managed/{name}/`
4. **Configure** — Write config.yaml entry
5. **Boot** — Start VM via worker_manager

Source: `cli/managed.rs` — `handle_binary_add`, `handle_bundle_add`

### Builtin Worker Telemetry

Source: `cli/managed.rs:42-85`

For builtin workers (part of the iii engine), a telemetry ping is fired:

```rust
async fn fire_engine_telemetry(name: &str, version: &str) {
    let url = format!("{api_url}/download/{name}");
    // GET returns 204 (no artifact); errors are warnings only
}
```

## OCI Workers

Source: `cli/worker_manager/oci.rs` (1,490 lines)

OCI workers are pulled from container registries:

1. **Pull** — `oci_client::Client::pull()` with auth
2. **Extract layers** — To sandbox rootfs
3. **Build container spec** — From OCI config
4. **Boot** — Via worker_manager

### OCI Gate Test

Source: `tests/oci_gate_smoke.rs`

Basic smoke test verifying OCI worker flow works end-to-end.

## Local Workers

Source: `cli/local_worker.rs` (1,482 lines)

Local workers run from a local path (source directory):

1. **Validate** — Check for `iii.worker.yaml` or known entry points
2. **Configure** — Write config.yaml entry
3. **Start** — Launch via iii-exec or dedicated process

Source: `cli/local_worker.rs` — `handle_local_add`, `is_local_path`, `start_local_worker`

```rust
pub fn is_local_path(input: &str) -> bool {
    input.starts_with("./") || input.starts_with("../")
        || input.starts_with('/') || input.contains('.')
}
```

## Binary vs Bundle Workers

The registry returns different worker types:

| Type | Artifact | Extraction | Execution |
|------|----------|------------|-----------|
| **Binary** | Single tarball with pre-compiled binary | Extract to managed dir | Direct execution in VM |
| **Bundle** | Archive with source + build scripts | Extract, build if needed | Execute via runtime |
| **Engine** | Built into iii engine | N/A | In-process |

**Aha:** The registry API returns a type field that determines the entire downstream path — binary workers skip building, bundle workers may need compilation, and engine workers are already in the binary. The CLI routes to the correct handler based on this single field.

## What's Next

- [04 — Add Pipeline](04-add-pipeline.md) — The add flow: resolve → download → extract → configure → boot
- [05 — Managed Ops](05-managed-ops.md) — Binary add, bundle add, local add in detail
- [07 — VM Lifecycle](07-vm-lifecycle.md) — libkrun VM management
