# Vfs Abstraction — The Bridge Between Desktop and CF

The `Vfs` trait is the key design decision that makes the CF port work. It provides a filesystem abstraction that both desktop (via `std::fs`) and CF (via Workspace) code can use without any `#[cfg]` gates at the call site.

**Source:** `src/vfs.rs` — 222 lines

## The Trait

```rust
pub trait Vfs {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn read_bytes(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn write(&self, path: &Path, data: &[u8]) -> io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;
    fn stat(&self, path: &Path) -> io::Result<Stat>;
    fn mkdir(&self, path: &Path) -> io::Result<()>;
    fn rm(&self, path: &Path) -> io::Result<()>;
    fn for_each_path(&self, f: &mut dyn FnMut(&str));
}
```

Plus supporting types:

```rust
pub enum StatKind { File, Dir, Symlink }
pub struct Stat { pub kind: StatKind, pub size: u64 }
```

**Aha:** The trait deliberately mirrors POSIX filesystem operations — not cloud APIs. This keeps the abstraction narrow: if you can read/write/list/stat/create/delete, you can run http-nu. The `for_each_path` method exists solely for the `glob` shadow command; it's intentionally a simple callback rather than a full iterator, keeping the trait object-safe.

## Thread-Local Dispatch

**Source:** `src/vfs.rs:57-153`

The active Vfs is stored in a thread-local `RefCell`:

```rust
thread_local! {
    static VFS_HANDLE: RefCell<Option<Box<dyn Vfs>>> = const { RefCell::new(None) };
}
```

Three operations manage it:

- `install_vfs(v: Box<dyn Vfs>)` — set the active Vfs
- `drop_vfs()` — clear it
- `with_vfs(|maybe_vfs| ...)` — run a closure with the current Vfs

On desktop, `with_vfs` returns `OsVfs` as a default when nothing is installed. On wasm, no Vfs installed = `None`. This means desktop code can leave the thread-local unset and still get filesystem access, while CF code explicitly installs `SnapshotVfs` before running Nu commands.

```rust
pub fn with_vfs<F, R>(f: F) -> R
where
    F: FnOnce(Option<&dyn Vfs>) -> R,
{
    VFS_HANDLE.with(|cell| {
        let borrowed = cell.borrow();
        if let Some(boxed) = borrowed.as_deref() {
            return f(Some(boxed));
        }
        #[cfg(feature = "desktop")]
        { f(Some(&OsVfs)) }
        #[cfg(not(feature = "desktop"))]
        { f(None) }
    })
}
```

## Free-Function Shortcuts

**Source:** `src/vfs.rs:95-129`

Convenience wrappers for the common "call one method, error if no Vfs":

```rust
pub fn read_to_string(path: &Path) -> io::Result<String>
pub fn read_bytes(path: &Path) -> io::Result<Vec<u8>>
pub fn write(path: &Path, data: &[u8]) -> io::Result<()>
pub fn exists(path: &Path) -> bool
```

These replace verbose `with_vfs(|maybe| ...)` call sites in upstream code.

## Path Resolution

**Source:** `src/vfs.rs:74-86`

`resolve_relative(path)` handles the desktop/wasm path resolution difference:

```rust
pub fn resolve_relative(path: &Path) -> PathBuf {
    if path.is_absolute() { return path.to_path_buf(); }
    #[cfg(feature = "desktop")]
    { std::env::current_dir().unwrap_or_default().join(path) }
    #[cfg(not(feature = "desktop"))]
    { PathBuf::from("/").join(path) }
}
```

On desktop, relative paths are resolved against `current_dir()`. On wasm, they're treated as workspace-rooted (`/path`).

## OsVfs — Desktop Implementation

**Source:** `src/vfs.rs:157-222` (gated to `#[cfg(feature = "desktop")]`)

`OsVfs` is a zero-sized type that wraps `std::fs` directly:

```rust
pub struct OsVfs;

impl Vfs for OsVfs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }
    fn read_bytes(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }
    fn write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, data)
    }
    // ... (stat detects symlinks via symlink_metadata)
    // ... (rm handles both files and directories)
    fn for_each_path(&self, _f: &mut dyn FnMut(&str)) {
        // No-op: recursive walk has no useful bound on desktop
    }
}
```

Key details:
- `write` auto-creates parent directories (matching `std::fs::write` behavior with `create_dir_all`)
- `stat` uses `symlink_metadata` to detect symlinks (not followed)
- `rm` handles both files (`remove_file`) and directories (`remove_dir_all`), returning `Ok(())` on `NotFound`
- `for_each_path` is a no-op on desktop (no bounded recursive walk)

**Aha:** `OsVfs` being a zero-sized type (`pub struct OsVfs;`) means there's no heap allocation when using the default desktop path. The `with_vfs` function creates a temporary reference to it on the stack, so the desktop hot path is essentially free.

[← Back to Architecture](01-cf-architecture.md) | [Next → SnapshotVfs](03-snapshot-vfs.md)
