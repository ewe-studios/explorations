---
title: Filesystem Ops — FsRequest, FsResult, FsEntry
---

# Filesystem Ops — FsRequest, FsResult, FsEntry

**The shell protocol includes native filesystem operations that run inside the guest VM without shell-outs — read, write, mkdir, chmod, grep, sed, tree, etc.**

## FsRequest Operations

```mermaid
flowchart TD
    A[FsRequest] --> B[Read: read file contents]
    A --> C[Write: write file (streaming)]
    A --> D[Mkdir: create directory]
    A --> E[Rmdir: remove directory]
    A --> F[Unlink: remove file]
    A --> G[Chmod: change permissions]
    A --> H[Grep: regex search]
    A --> I[Sed: find and replace]
    A --> J[Tree: recursive listing]
    A --> K[Ls/Stat: directory listing]
```

## FsEntry

Source: `iii-shell-proto/src/lib.rs:69-77`

```rust
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mode: String,       // Octal permission string, e.g. "0644"
    pub mtime: i64,         // Unix seconds
    pub is_symlink: bool,
}
```

**Aha:** The `mode` field is an octal permission *string* (e.g., `"0644"`), not a numeric mode_t. This makes it trivially displayable in JSON responses without client-side formatting.

## FsMatch (grep results)

```rust
pub struct FsMatch {
    pub path: String,
    pub line: u64,          // 1-based line number
    pub content: String,    // Truncated if line > max_line_bytes
}
```

## What's Next

- [04 — Cross-Cutting](04-cross-cutting.md) — Base64 encoding, dependencies
- [01 — Wire Protocol](01-wire-protocol.md) — Return to wire protocol
- [00 — Overview](00-overview.md) — Return to overview
