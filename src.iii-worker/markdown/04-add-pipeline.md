---
title: Add Pipeline — Resolve, Download, Extract, Configure, Boot
---

# Add Pipeline — Resolve, Download, Extract, Configure, Boot

**The `iii worker add` command is the core workflow — it resolves a worker source, downloads it, extracts to the managed directory, configures config.yaml, and boots the VM.** This document traces the full pipeline.

## Add Flow

```mermaid
sequenceDiagram
    participant User
    participant CLI as core::add::run
    participant Registry as Workers Registry
    participant FS as Filesystem
    participant Config as config_file
    participant VM as worker_manager
    participant Engine as iii Engine

    User->>CLI: iii worker add pdfkit
    CLI->>CLI: parse_source_for_cli("pdfkit")
    
    alt Registry worker
        CLI->>Registry: GET /workers/pdfkit
        Registry-->>CLI: {type: binary, version: 1.2.3, url: ...}
        CLI->>Registry: GET /download/pdfkit?version=1.2.3
        Registry-->>CLI: tarball bytes
    else OCI worker
        CLI->>Registry: Pull OCI image
        Registry-->>CLI: image layers
    else Local worker
        CLI->>FS: Validate local path
    end

    CLI->>FS: Extract to ~/.iii/managed/pdfkit/
    CLI->>FS: Write iii.lock entry
    CLI->>Config: Add worker to config.yaml
    Config->>Engine: File watcher detects change
    Engine->>VM: Boot VM for pdfkit
    VM->>VM: Create krun VM with rootfs
    VM->>Engine: Worker connects via WebSocket
    Engine-->>VM: WorkerRegistered
    CLI->>User: ✓ pdfkit ready in 3.2s
```

## AddOptions

Source: `core/types.rs`

```rust
pub struct AddOptions {
    pub source: WorkerSource,
    pub force: bool,
    pub reset_config: bool,
    pub wait: bool,
}
```

```mermaid
flowchart LR
    A[AddOptions] --> B[source: WorkerSource]
    A --> C[force: bool]
    A --> D[reset_config: bool]
    A --> E[wait: bool]
    B --> F{Registry/OCI/Local}
    C --> G[Delete existing artifacts]
    D --> H[Clear config.yaml entry]
    E --> I[Block until ready]
```

## Add Pipeline Steps

1. **Resolve** — Determine worker type from source (registry/OCI/local)
2. **Download** — Fetch binary/bundle from registry or pull OCI image
3. **Extract** — Unpack to `~/.iii/managed/{name}/`
4. **Configure** — Write config.yaml entry with worker definition
5. **Lock** — Update iii.lock with resolved version
6. **Boot** — Start the worker VM (if `wait: true`)

## Force Re-add

With `--force`, the pipeline deletes existing artifacts before re-downloading:

```rust
if force {
    // Delete existing managed directory
    std::fs::remove_dir_all(managed_dir)?;
}
```

**Aha:** At startup, iii-worker sweeps orphaned staging directories left from interrupted installs (SIGKILL, power cut, OOM). The RAII `StagingGuard` pattern normally cleans up automatically, but SIGKILL bypasses Drop. The sweep prevents accumulation of partial downloads.

## Orphan Staging Sweep

Source: `main.rs:19-26`

At startup, iii-worker sweeps orphaned staging directories:

```rust
// If a previous `iii worker add <bundle>` was killed mid-install
// (SIGKILL / power cut / OOM), the RAII StagingGuard could not run
// and left a directory behind under ~/.iii/workers-bundle/.staging/.
let _ = iii_worker::cli::bundle_download::sweep_orphans();
```

This prevents accumulation of partial downloads from interrupted installs.

## What's Next

- [05 — Managed Ops](05-managed-ops.md) — Binary add, bundle add, local add in detail
- [06 — Sandbox Daemon](06-sandbox-daemon.md) — VM management, overlay filesystems, exec
- [07 — VM Lifecycle](07-vm-lifecycle.md) — libkrun VM management
