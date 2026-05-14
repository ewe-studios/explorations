# SnapshotVfs — CF's Workspace-Backed Vfs Implementation

`SnapshotVfs` is the CF-side `Vfs` implementation. Instead of calling `std::fs`, it operates on an in-memory snapshot of the user's Workspace, preloaded before Nushell evaluation and drained back to persistent storage afterward.

**Source:** `src/cf/snapshot_vfs.rs` — 259 lines

## Why Snapshot Instead of Direct Workspace Access

Nu commands run synchronously (`Command::run` is not async). Workspace operations are async (R2 spillover, SQLite writes). The solution: **async preload, sync access**.

```
Request arrives
  → async: walk Workspace into memory (SnapshotVfs)
  → install SnapshotVfs via thread-local
  → sync: run Nu commands against in-memory snapshot
  → async: drain pending writes back to Workspace
```

**Aha:** This design avoids needing an async Nu eval path (which doesn't exist upstream). The tradeoff is that the snapshot is a point-in-time view — writes from concurrent requests during eval won't be visible until the next request. For per-user DO isolation, this is acceptable (each user is usually the only writer to their workspace).

## Storage Shape

```rust
struct SnapshotInner {
    files: HashMap<PathBuf, Vec<u8>>,       // Inlined file content
    dirs: HashMap<PathBuf, Vec<PathBuf>>,    // Directory entries
    stats: HashMap<PathBuf, Stat>,           // File/dir/symlink metadata
    pending_writes: HashMap<PathBuf, Vec<u8>>, // Queued writes
    pending_ops: Vec<PendingOp>,             // Queued mkdir/rm
}

enum PendingOp { Mkdir(PathBuf), Rm(PathBuf) }
```

Both reads and writes go through the same `SnapshotInner`. A write queues into `pending_writes`; a subsequent read returns the pending version. The handle is `Rc<RefCell<SnapshotInner>>` — cheap to clone, and both the handler and the `fetch` method share the same underlying data.

## Workspace Preload

**Source:** `src/cf/snapshot_vfs.rs:59-145`

```rust
pub async fn load_from_workspace(
    ws: &Workspace,
    max_depth: u32,        // 4
    inline_limit: u64,     // 1,500,000 (1.5MB)
) -> worker::Result<Self>
```

The `walk` function recursively traverses the Workspace tree:

1. Calls `ws.read_dir_with_file_types(path)` to get entries
2. For files: records stat, inlines content if ≤ 1.5MB
3. For directories: records stat, recurses if depth < `max_depth`
4. For symlinks: records stat only (no content)
5. Stores child paths in the parent's `dirs` entry

Files larger than 1.5MB are stat-only (their content is left out so the snapshot stays small). A read for them returns `ENOENT`.

**Depth limit of 4** prevents unbounded recursion into deeply nested directories. **Inline limit of 1.5MB** keeps the snapshot under Workers' memory budget (128MB default).

## Vfs Trait Implementation

### Reads

```rust
fn read_bytes(&self, path: &Path) -> io::Result<Vec<u8>> {
    let inner = self.inner.borrow();
    // Check pending writes first (most recent version)
    if let Some(data) = inner.pending_writes.get(path) {
        return Ok(data.clone());
    }
    // Fall back to preloaded snapshot
    inner.files.get(path).cloned().ok_or_else(|| not_found(path))
}

fn exists(&self, path: &Path) -> bool {
    let inner = self.inner.borrow();
    inner.pending_writes.contains_key(path)
        || inner.files.contains_key(path)
        || inner.dirs.contains_key(path)
        || inner.stats.contains_key(path)
}
```

### Writes (Queued)

```rust
fn write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
    self.inner.borrow_mut().pending_writes
        .insert(path.to_path_buf(), data.to_vec());
    Ok(())
}
```

Writes go to `pending_writes`, not directly to Workspace. This is critical — Nu commands run sync and can't await R2/SQLite.

### Directory Operations

```rust
fn mkdir(&self, path: &Path) -> io::Result<()> {
    let mut inner = self.inner.borrow_mut();
    inner.stats.insert(p.clone(), Stat { kind: Dir, size: 0 });
    inner.dirs.entry(p.clone()).or_default();
    inner.pending_ops.push(PendingOp::Mkdir(p));
    Ok(())
}

fn rm(&self, path: &Path) -> io::Result<()> {
    let mut inner = self.inner.borrow_mut();
    inner.files.remove(&p);
    inner.dirs.remove(&p);
    inner.stats.remove(&p);
    inner.pending_writes.remove(&p);
    inner.pending_ops.push(PendingOp::Rm(p));
    Ok(())
}
```

Both queue a `PendingOp` for deferred persistence AND update the in-memory state immediately, so subsequent reads within the same request see the change.

### Drain Methods

```rust
pub fn drain_pending_writes(&self) -> Vec<(PathBuf, Vec<u8>)>
pub fn drain_pending_ops(&self) -> Vec<PendingOp>
```

These are called by `cf::mod.rs::fetch` after Nushell eval completes. `std::mem::take` empties the collections, returning ownership to the caller for async flushing.

## Request-Scoped Lifecycle

The `fetch` handler in `cf/mod.rs` manages the full cycle:

```rust
// 1. Preload
let snapshot = SnapshotVfs::load_from_workspace(&ws, 4, 1_500_000).await?;
crate::vfs::install_vfs(Box::new(snapshot.clone()));

// 2. Eval (Nu commands read/write through Vfs)
let response = handler::handle(&mut req).await;

// 3. Drain and persist (mkdir first, then writes, then rm)
let writes = snapshot.drain_pending_writes();
let ops = snapshot.drain_pending_ops();
for op in &ops { /* flush mkdir */ }
for (path, bytes) in writes { /* flush write */ }
for op in ops { /* flush rm */ }
crate::vfs::drop_vfs();
```

**Aha:** The `Rc<RefCell<>>` sharing means both the `fetch` handler and the installed `Vfs` reference the same `SnapshotInner`. Nu commands queue writes during eval; `fetch` drains them afterward. No serialization or message passing needed.

[← Back to Vfs](02-vfs.md) | [Next → Shadow Commands](04-shadow-commands.md)
