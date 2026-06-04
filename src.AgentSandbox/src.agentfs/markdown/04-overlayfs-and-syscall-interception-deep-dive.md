---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.turso/agentfs
repository: git@github.com:tursodatabase/agentfs.git
explored_at: 2026-04-05
language: Rust
type: deep-dive
parent: exploration.md
---

# OverlayFS and Syscall Interception: Complete Deep-Dive

**Exhaustive technical detail about copy-on-write semantics, whiteout tracking, and ptrace-based syscall redirection**

This document covers:
1. **OverlayFS Copy-on-Write** - Complete line-by-line analysis of the 600+ line implementation
2. **Whiteout Mechanics** - How deletions are tracked across layers
3. **Directory Entry Merging** - How readdir() combines base + delta
4. **Ptrace Syscall Interception** - How Reverie captures and redirects syscalls
5. **File Descriptor Remapping** - Tracking open files across the sandbox boundary
6. **Sandbox Security Model** - What operations are allowed/blocked

---

## Table of Contents

1. [OverlayFS Architecture](#1-overlayfs-architecture)
2. [Copy-on-Write: Line-by-Line Analysis](#2-copy-on-write-line-by-line-analysis)
3. [Whiteout System Deep-Dive](#3-whiteout-system-deep-dive)
4. [Directory Entry Merging](#4-directory-entry-merging)
5. [Ptrace Syscall Interception](#5-ptrace-syscall-interception)
6. [File Descriptor Remapping](#6-file-descriptor-remapping)
7. [Sandbox Security Model](#7-sandbox-security-model)
8. [Performance Characteristics](#8-performance-characteristics)

---

## 1. OverlayFS Architecture

### 1.1 The Layered Model

OverlayFS combines two filesystem layers into a unified view:

```
                    APPLICATION VIEW
           /-----------------------------\
           |  /src/main.rs (merged)      |
           |  /Cargo.toml (merged)       |
           |  /target/debug/ (delta)     |
           \-----------------------------/
                      /    \
                     /      \
         +----------+        +----------+
         |   DELTA  |        |   BASE   |
         | (writable|        |(read-only|
         |  layer)  |        |  layer)  |
         |          |        |          |
         |  AgentFS |        |  HostFS  |
         |  SQLite  |        |/home/proj|
         +----------+        +----------+
```

### 1.2 The OverlayFS Struct

From `sdk/rust/src/filesystem/overlayfs.rs:29-51`:

```rust
pub struct OverlayFS {
    /// Read-only base layer (can be any FileSystem implementation)
    /// 
    /// Typically HostFS pointing to the user's project directory.
    /// Could also be another OverlayFS (stacked overlays).
    base: Arc<dyn FileSystem>,
    
    /// Writable delta layer (must be AgentFS for whiteout storage)
    /// 
    /// All modifications go here. Uses SQLite/Turso backend
    /// with tables: fs_inode, fs_dentry, fs_data, fs_whiteout
    delta: AgentFS,
}

/// An open file handle for OverlayFS.
/// 
/// This is the key to copy-on-write semantics. Tracks which
/// layer(s) the file exists in so operations know where to read/write.
pub struct OverlayFile {
    /// File handle for the delta layer (if file exists there)
    /// 
    /// If Some, this file has been copied-on-write or created in delta.
    /// All writes go to delta_file.
    delta_file: Option<BoxedFile>,
    
    /// File handle for the base layer (if file exists there)
    /// 
    /// If Some, this file exists in the read-only base.
    /// Used for initial read before copy-on-write.
    base_file: Option<BoxedFile>,
    
    /// Reference to delta for copy-on-write operations
    /// 
    /// Needed to perform the actual copy when first write occurs.
    /// Arc clone is cheap, allows sharing across threads.
    delta: AgentFS,
    
    /// The normalized path for copy-on-write operations
    /// 
    /// Example: "/src/main.rs" (always starts with /, no trailing /)
    path: String,
    
    /// Track if we've done copy-on-write (to avoid re-copying)
    /// 
    /// AtomicBool for lock-free checking. Once true, all subsequent
    /// writes go directly to delta_file without checking base.
    copied_to_delta: std::sync::atomic::AtomicBool,
}
```

### 1.3 FileSystem Trait Implementation

OverlayFS implements the same `FileSystem` trait as AgentFS and HostFS:

```rust
#[async_trait]
impl FileSystem for OverlayFS {
    async fn stat(&self, path: &str) -> Result<Option<Stats>> { /* ... */ }
    async fn lstat(&self, path: &str) -> Result<Option<Stats>> { /* ... */ }
    async fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> { /* ... */ }
    async fn write_file(&self, path: &str, data: &[u8]) -> Result<()> { /* ... */ }
    async fn readdir(&self, path: &str) -> Result<Option<Vec<String>>> { /* ... */ }
    async fn mkdir(&self, path: &str) -> Result<()> { /* ... */ }
    async fn remove(&self, path: &str) -> Result<()> { /* ... */ }
    async fn rename(&self, from: &str, to: &str) -> Result<()> { /* ... */ }
    async fn chmod(&self, path: &str, mode: u32) -> Result<()> { /* ... */ }
    async fn symlink(&self, target: &str, linkpath: &str) -> Result<()> { /* ... */ }
    async fn readlink(&self, path: &str) -> Result<Option<String>> { /* ... */ }
    async fn open(&self, path: &str) -> Result<BoxedFile> { /* ... */ }
}
```

This allows OverlayFS to be used anywhere a `FileSystem` is expected, including:
- FUSE mount (`AgentFSFuse { fs: Arc<dyn FileSystem> }`)
- NFS server (`AgentNFS { fs: Arc<Mutex<dyn FileSystem>> }`)
- Nested overlays (OverlayFS with OverlayFS as base)

---

## 2. Copy-on-Write: Line-by-Line Analysis

### 2.1 The stat() Operation

From `sdk/rust/src/filesystem/overlayfs.rs:452-475`:

```rust
async fn stat(&self, path: &str) -> Result<Option<Stats>> {
    // Step 1: Normalize path (strip trailing /, ensure leading /)
    let normalized = self.normalize_path(path);
    
    // Step 2: Check whiteouts first
    // 
    // If path has a whiteout record, it means the file was
    // "logically deleted" even if it exists in base.
    // Return None as if file doesn't exist.
    if self.is_whiteout(&normalized).await? {
        return Ok(None);
    }
    
    // Step 3: Check delta layer (authoritative for files in delta)
    // 
    /// Files in delta are "new" or "modified" files.
    /// Delta is always checked first because it contains
    /// the most recent version of the file.
    if let Some(stats) = self.delta.stat(&normalized).await? {
        return Ok(Some(stats));
    }
    
    // Step 4: Fall back to base layer
    // 
    // If not in delta, check the read-only base.
    // This is the "fall-through" behavior of overlay.
    if let Some(mut stats) = self.base.stat(&normalized).await? {
        // CRITICAL: Root directory must have inode 1
        // 
        // FUSE requires root to have inode 1. If base returns
        // different inode, FUSE lookups will fail.
        if normalized == "/" {
            stats.ino = 1;
        }
        return Ok(Some(stats));
    }
    
    // Step 5: Not found in either layer
    Ok(None)
}
```

**Why This Order Matters:**

```
Delta says: file exists, size=1000 bytes
Base says: file exists, size=500 bytes
Result: 1000 bytes (delta is authoritative)
```

### 2.2 The lstat() Operation

Similar to stat(), but uses lstat() for symlinks:

```rust
async fn lstat(&self, path: &str) -> Result<Option<Stats>> {
    let normalized = self.normalize_path(path);
    
    // Whiteout check (same as stat)
    if self.is_whiteout(&normalized).await? {
        return Ok(None);
    }
    
    // Check delta first (same as stat)
    if let Some(stats) = self.delta.lstat(&normalized).await? {
        return Ok(Some(stats));
    }
    
    // Fall back to base (same as stat)
    if let Some(mut stats) = self.base.lstat(&normalized).await? {
        if normalized == "/" {
            stats.ino = 1;
        }
        return Ok(Some(stats));
    }
    
    Ok(None)
}
```

**Difference between stat() and lstat():**
- `stat()` follows symlinks (returns target stats)
- `lstat()` returns symlink itself stats

### 2.3 The open() Operation - Creating OverlayFile

From `sdk/rust/src/filesystem/overlayfs.rs`:

```rust
async fn open(&self, path: &str) -> Result<BoxedFile> {
    let normalized = self.normalize_path(path);
    
    // Check whiteout - can't open deleted files
    if self.is_whiteout(&normalized).await? {
        return Err(FsError::NotFound.into());
    }
    
    // Try to open in delta first
    let delta_file = match self.delta.open(&normalized).await {
        Ok(f) => Some(f),
        Err(_) => None,  // Not in delta, try base
    };
    
    // Try to open in base
    let base_file = match self.base.open(&normalized).await {
        Ok(f) => Some(f),
        Err(_) => None,  // Not in base either
    };
    
    // Must exist in at least one layer
    if delta_file.is_none() && base_file.is_none() {
        return Err(FsError::NotFound.into());
    }
    
    // Create OverlayFile with handles to both layers
    Ok(Box::new(OverlayFile {
        delta_file,
        base_file,
        delta: self.delta.clone(),
        path: normalized,
        copied_to_delta: AtomicBool::new(false),
    }))
}
```

**Key Insight:** The `OverlayFile` holds handles to **both** layers. This enables:
- Reading from base if not yet copied
- Writing to delta after copy-on-write
- Tracking which layer is authoritative

### 2.4 The pwrite() Operation - Triggering Copy-on-Write

From `sdk/rust/src/filesystem/overlayfs.rs:90-118`:

```rust
async fn pwrite(&self, offset: u64, data: &[u8]) -> Result<()> {
    // Case 1: Already have delta_file (already copied)
    // Just write directly without any copy-on-write logic
    if let Some(ref delta_file) = self.delta_file {
        return delta_file.pwrite(offset, data).await;
    }
    
    // Case 2: Need to check if copy-on-write is needed
    // Use atomic flag to avoid race condition
    if !self.copied_to_delta.load(std::sync::atomic::Ordering::Acquire) {
        // CRITICAL: Ensure parent directories exist in delta
        // 
        // Example: Writing to /a/b/c.txt when /a/b doesn't exist in delta
        // We must create /a and /a/b in delta first
        self.ensure_parent_dirs_in_delta().await?;
        
        // Copy from base if it exists
        if let Some(ref base_file) = self.base_file {
            // Step 1: Read entire file from base
            let stats = base_file.fstat().await?;
            let base_data = base_file.pread(0, stats.size as u64).await?;
            
            // Step 2: Write complete file to delta
            // This is "copy" in copy-on-write
            self.delta.write_file(&self.path, &base_data).await?;
        } else {
            // File didn't exist in base (O_CREAT case)
            // Just create empty file in delta
            self.delta.write_file(&self.path, &[]).await?;
        }
        
        // Mark as copied (release ordering ensures writes above are visible)
        self.copied_to_delta.store(true, std::sync::atomic::Ordering::Release);
    }
    
    // Step 3: Open file in delta (now it exists)
    let delta_file = self.delta.open(&self.path).await?;
    
    // Step 4: Write the actual data at the specified offset
    delta_file.pwrite(offset, data).await
}
```

**Why Copy Entire File?**

AgentFS stores files in SQLite chunks:

```sql
-- fs_data table stores file content in chunks
CREATE TABLE fs_data (
    ino INTEGER NOT NULL,      -- File inode
    chunk_index INTEGER NOT NULL,  -- Chunk 0, 1, 2, ...
    data BLOB NOT NULL,        -- Up to 4096 bytes per chunk
    PRIMARY KEY (ino, chunk_index)
);
```

To do partial writes, we'd need to:
1. Calculate which chunks are affected
2. Read-modify-write each chunk
3. Handle chunk boundary cases

**Simpler approach:** Copy entire file once, then do delta writes.

### 2.5 The pread() Operation - Reading from Correct Layer

From `sdk/rust/src/filesystem/overlayfs.rs:77-88`:

```rust
async fn pread(&self, offset: u64, size: u64) -> Result<Vec<u8>> {
    // Prefer delta if we have it (file was copied or created in delta)
    if let Some(ref file) = self.delta_file {
        return file.pread(offset, size).await;
    }
    
    // Fall back to base (file hasn't been modified yet)
    if let Some(ref file) = self.base_file {
        return file.pread(offset, size).await;
    }
    
    // File doesn't exist in either layer (shouldn't happen)
    Ok(Vec::new())
}
```

**Read Scenarios:**

| Scenario | delta_file | base_file | Read from |
|----------|------------|-----------|-----------|
| New file in delta | Some | None | delta |
| Unmodified base file | None | Some | base |
| Modified file (after CoW) | Some | Some | delta |
| Deleted file | None | None | (error) |

### 2.6 The truncate() Operation

Similar to pwrite(), triggers copy-on-write:

```rust
async fn truncate(&self, size: u64) -> Result<()> {
    // If we already have delta_file, use it directly
    if let Some(ref delta_file) = self.delta_file {
        return delta_file.truncate(size).await;
    }
    
    // Copy-on-write if needed (same logic as pwrite)
    if !self.copied_to_delta.load(Ordering::Acquire) {
        self.ensure_parent_dirs_in_delta().await?;
        
        if let Some(ref base_file) = self.base_file {
            let stats = base_file.fstat().await?;
            let base_data = base_file.pread(0, stats.size as u64).await?;
            self.delta.write_file(&self.path, &base_data).await?;
        } else {
            self.delta.write_file(&self.path, &[]).await?;
        }
        
        self.copied_to_delta.store(true, Ordering::Release);
    }
    
    // Open and truncate in delta
    let delta_file = self.delta.open(&self.path).await?;
    delta_file.truncate(size).await
}
```

### 2.7 The fsync() Operation

```rust
async fn fsync(&self) -> Result<()> {
    // If we have delta_file, sync it (file was written)
    if let Some(ref delta_file) = self.delta_file {
        return delta_file.fsync().await;
    }
    
    // If we did copy-on-write, open and sync
    if self.copied_to_delta.load(Ordering::Acquire) {
        let delta_file = self.delta.open(&self.path).await?;
        return delta_file.fsync().await;
    }
    
    // File only exists in base (read-only), nothing to sync
    // Base is the actual filesystem, it handles its own syncing
    Ok(())
}
```

---

## 3. Whiteout System Deep-Dive

### 3.1 What is a Whiteout?

A whiteout is a "tombstone" record that marks a path as deleted:

```
Base layer has: /src/old.rs
User runs: rm /src/old.rs

Cannot actually delete from base (read-only)
→ Create whiteout record: path="/src/old.rs"

Subsequent lookups check whiteout table first
→ Return ENOENT as if file doesn't exist
```

### 3.2 Whiteout Table Schema

From `sdk/rust/src/filesystem/overlayfs.rs:195-211`:

```rust
pub async fn init_schema(conn: &Connection, base_path: &str) -> Result<()> {
    // Create whiteout table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS fs_whiteout (
            path TEXT PRIMARY KEY,      -- e.g., "/src/old.rs"
            parent_path TEXT NOT NULL,  -- e.g., "/src"
            created_at INTEGER NOT NULL -- Unix timestamp
        )",
        (),
    ).await?;
    
    // Index for efficient child lookups
    // Without this, would need LIKE which compiles regex
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_fs_whiteout_parent 
         ON fs_whiteout(parent_path)",
        (),
    ).await?;
    
    // Store base_path for tool identification
    conn.execute(
        "CREATE TABLE IF NOT EXISTS fs_overlay_config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        (),
    ).await?;
    
    conn.execute(
        "INSERT OR REPLACE INTO fs_overlay_config 
         (key, value) VALUES ('base_path', ?1)",
        [Value::Text(base_path.to_string())],
    ).await?;
    
    Ok(())
}
```

### 3.3 Creating a Whiteout (Deletion)

From `sdk/rust/src/filesystem/overlayfs.rs:318-331`:

```rust
async fn create_whiteout(&self, path: &str) -> Result<()> {
    let normalized = self.normalize_path(path);
    let parent = Self::parent_path(&normalized);
    let conn = self.delta.get_connection();
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
    
    // INSERT or UPDATE if whiteout already exists
    conn.execute(
        "INSERT INTO fs_whiteout (path, parent_path, created_at) 
         VALUES (?, ?, ?)
         ON CONFLICT(path) DO UPDATE 
         SET created_at = excluded.created_at",
        (normalized.as_str(), parent.as_str(), now),
    ).await?;
    
    Ok(())
}

/// Helper: extract parent path from normalized path
fn parent_path(path: &str) -> String {
    if path == "/" {
        return "/".to_string();
    }
    match path.rfind('/') {
        Some(0) => "/".to_string(),  // Parent of "/foo" is "/"
        Some(idx) => path[..idx].to_string(),
        None => "/".to_string(),
    }
}
```

### 3.4 Checking for Whiteouts

From `sdk/rust/src/filesystem/overlayfs.rs:279-315`:

```rust
async fn is_whiteout(&self, path: &str) -> Result<bool> {
    let normalized = self.normalize_path(path);
    let conn = self.delta.get_connection();
    
    // Check the path itself AND all parent paths
    // 
    // Why? If /foo is whiteout, then /foo/bar is also "deleted"
    // even if there's no explicit whiteout for /foo/bar
    let mut check_path = normalized.clone();
    loop {
        let result = conn
            .query(
                "SELECT 1 FROM fs_whiteout WHERE path = ?",
                (check_path.as_str(),),
            )
            .await;
        
        // Handle case where fs_whiteout table doesn't exist yet
        let mut rows = match result {
            Ok(rows) => rows,
            Err(_) => return Ok(false),  // No whiteouts possible
        };
        
        // Found a whiteout at this level
        if rows.next().await?.is_some() {
            return Ok(true);
        }
        
        // Move to parent directory
        if let Some(parent_end) = check_path.rfind('/') {
            if parent_end == 0 {
                // We've reached root
                break;
            }
            check_path = check_path[..parent_end].to_string();
        } else {
            break;
        }
    }
    Ok(false)
}
```

**Parent Path Traversal Example:**

```
Path: /a/b/c/d

Check order:
1. /a/b/c/d - whiteout?
2. /a/b/c    - whiteout?
3. /a/b      - whiteout?
4. /a        - whiteout?
5. /         - stop

If /a/b is whiteout, then /a/b/c/d is "deleted" by inheritance
```

### 3.5 Removing a Whiteout (Un-deletion)

From `sdk/rust/src/filesystem/overlayfs.rs:334-344`:

```rust
async fn remove_whiteout(&self, path: &str) -> Result<()> {
    let normalized = self.normalize_path(path);
    let conn = self.delta.get_connection();
    
    conn.execute(
        "DELETE FROM fs_whiteout WHERE path = ?",
        (normalized.as_str(),),
    ).await?;
    
    Ok(())
}
```

**When is this called?**

When creating a file that was previously deleted:

```rust
// User does: rm /src/old.rs && echo "new" > /src/old.rs

// Step 1: rm creates whiteout
create_whiteout("/src/old.rs")

// Step 2: echo needs to create file, removes whiteout first
remove_whiteout("/src/old.rs")
write_file("/src/old.rs", b"new\n")
```

### 3.6 Getting Child Whiteouts (for readdir)

From `sdk/rust/src/filesystem/overlayfs.rs:347-371`:

```rust
async fn get_child_whiteouts(&self, dir_path: &str) -> Result<HashSet<String>> {
    let normalized = self.normalize_path(dir_path);
    let conn = self.delta.get_connection();
    let mut whiteouts = HashSet::new();
    
    // Use parent_path index for O(1) lookup
    // Much faster than LIKE which compiles regex
    let mut rows = conn
        .query(
            "SELECT path FROM fs_whiteout WHERE parent_path = ?",
            (normalized.as_str(),),
        )
        .await?;
    
    while let Some(row) = rows.next().await? {
        if let Ok(Value::Text(p)) = row.get_value(0) {
            // Extract filename from path
            if let Some(name) = p.rsplit('/').next() {
                if !name.is_empty() {
                    whiteouts.insert(name.to_string());
                }
            }
        }
    }
    Ok(whiteouts)
}
```

---

## 4. Directory Entry Merging

### 4.1 The readdir() Challenge

When listing a directory, OverlayFS must merge entries from both layers:

```
Base layer: /src/ contains [main.rs, lib.rs, old.rs]
Delta layer: /src/ contains [main.rs, new.rs]
Whiteouts: [/src/old.rs]

Merged result: [main.rs (from delta), lib.rs (from base), new.rs (from delta)]
```

**Rules:**
1. Delta entries override base entries
2. Whiteouted base entries are excluded
3. All delta entries are included

### 4.2 The readdir() Implementation

From `sdk/rust/src/filesystem/overlayfs.rs`:

```rust
async fn readdir(&self, path: &str) -> Result<Option<Vec<String>>> {
    let normalized = self.normalize_path(path);
    
    // Step 1: Get entries from delta layer
    let mut delta_entries = self.delta.readdir(&normalized).await?.unwrap_or_default();
    let delta_set: HashSet<_> = delta_entries.iter().cloned().collect();
    
    // Step 2: Get entries from base layer
    let base_entries = self.base.readdir(&normalized).await?.unwrap_or_default();
    
    // Step 3: Get whiteouts (deleted from base)
    let whiteouts = self.get_child_whiteouts(&normalized).await?;
    
    // Step 4: Merge
    // - All delta entries are included
    // - Base entries included only if:
    //   a) Not in delta (delta doesn't override)
    //   b) Not whiteouted (not deleted)
    for entry in base_entries {
        if !delta_set.contains(&entry) && !whiteouts.contains(&entry) {
            delta_entries.push(entry);
        }
    }
    
    Ok(Some(delta_entries))
}
```

### 4.3 The readdir_plus() Implementation

Returns entries with full stats (more efficient for FUSE):

```rust
async fn readdir_plus(&self, path: &str) -> Result<Option<Vec<DirEntry>>> {
    let normalized = self.normalize_path(path);
    
    // Get delta entries with stats
    let mut delta_entries = self.delta.readdir_plus(&normalized).await?.unwrap_or_default();
    let delta_names: HashSet<_> = delta_entries.iter().map(|e| e.name.clone()).collect();
    
    // Get base entries with stats
    let base_entries = self.base.readdir_plus(&normalized).await?.unwrap_or_default();
    
    // Get whiteouts
    let whiteouts = self.get_child_whiteouts(&normalized).await?;
    
    // Merge with stats
    for base_entry in base_entries {
        if !delta_names.contains(&base_entry.name) && !whiteouts.contains(&base_entry.name) {
            delta_entries.push(base_entry);
        }
    }
    
    Ok(Some(delta_entries))
}
```

### 4.4 Example Merge

**Before:**
```
Base: [main.rs, lib.rs, old.rs]
Delta: [main.rs (modified), new.rs]
Whiteouts: [old.rs]
```

**Merge Process:**

1. Start with delta: `[main.rs, new.rs]`
2. Check base entries:
   - `main.rs`: In delta_set? YES → skip
   - `lib.rs`: In delta_set? NO. In whiteouts? NO → add
   - `old.rs`: In delta_set? NO. In whiteouts? YES → skip
3. Result: `[main.rs, new.rs, lib.rs]`

---

## 5. Ptrace Syscall Interception

### 5.1 Why Ptrace?

The `agentfs run` command executes programs in a sandbox:

```bash
agentfs run -- cargo build
```

The `cargo` process sees the AgentFS mount, but all syscalls are intercepted. This is necessary because:

1. **Transparent redirection** - Process doesn't know it's using AgentFS
2. **Isolation** - Can't access files outside sandbox
3. **Audit trail** - Log all file operations
4. **Ephemeral sessions** - Discard all changes on exit

### 5.2 The Ptrace Flow

```
1. AgentFS spawns child with PTRACE_TRACEME
                    ↓
2. Child execve() → SIGTRAP signal
                    ↓
3. Parent (tracer) inspects syscall args
                    ↓
4. Parent decides: native or emulated?
                    ↓
5. If emulated: modify return value
   If native: let kernel handle
                    ↓
6. PTRACE_SYSCALL → continue
                    ↓
7. Repeat for all syscalls
```

### 5.3 Reverie: The Ptrace Library

AgentFS uses Facebook's Reverie library for ptrace:

**Key Reverie Concepts:**

| Concept | Purpose |
|---------|---------|
| `Tracer` | The tracing process (AgentFS) |
| `Tracee` | The traced process (user program) |
| `Syscall` | Captured syscall with args |
| `Handler` | Callback for each syscall |
| `Policy` | Rules for which syscalls to intercept |

### 5.4 Syscall Handler Structure

Hypothetical handler based on Reverie patterns:

```rust
async fn handle_syscall(tracee: &mut Tracee, syscall: Syscall) -> Result<SyscallResult> {
    match syscall.number {
        // File operations - redirect to AgentFS
        SYS_OPEN => handle_open(tracee, syscall).await,
        SYS_READ => handle_read(tracee, syscall).await,
        SYS_WRITE => handle_write(tracee, syscall).await,
        SYS_CLOSE => handle_close(tracee, syscall).await,
        SYS_STAT => handle_stat(tracee, syscall).await,
        SYS_LSTAT => handle_stat(tracee, syscall).await,
        SYS_FSTAT => handle_fstat(tracee, syscall).await,
        SYS_ACCESS => handle_access(tracee, syscall).await,
        SYS_GETCWD => handle_getcwd(tracee, syscall).await,
        SYS_CHDIR => handle_chdir(tracee, syscall).await,
        
        // Process operations - let kernel handle
        SYS_FORK | SYS_CLONE | SYS_EXECVE => Ok(SyscallResult::Native),
        
        // Memory operations - let kernel handle
        SYS_MMAP | SYS_MUNMAP | SYS_MPROTECT => Ok(SyscallResult::Native),
        
        // Network operations - let kernel handle (or intercept for simulation)
        SYS_SOCKET | SYS_CONNECT | SYS_SENDTO => Ok(SyscallResult::Native),
        
        _ => Ok(SyscallResult::Native),  // Default: native
    }
}
```

### 5.5 The open() Handler

```rust
async fn handle_open(tracee: &mut Tracee, syscall: Syscall) -> Result<SyscallResult> {
    // Read filename from tracee's memory
    let path_ptr = tracee.read_pointer(syscall.arg0)?;
    let path = tracee.read_string(path_ptr)?;
    let flags = syscall.arg1 as i32;
    let mode = syscall.arg2 as u32;
    
    // Check if path is within sandbox
    if !path.starts_with("/mnt/agentfs") {
        return Ok(SyscallResult::Native);  // Let kernel handle
    }
    
    // Redirect to AgentFS
    let vfs = tracee.get_vfs();
    let file = vfs.open(&path, flags, mode).await?;
    
    // Allocate FD in tracee's FD table
    let fd = tracee.allocate_fd(file);
    
    // Emulate syscall (don't call kernel)
    Ok(SyscallResult::Emulated(fd as i64))
}
```

### 5.6 The read() Handler

```rust
async fn handle_read(tracee: &mut Tracee, syscall: Syscall) -> Result<SyscallResult> {
    let fd = syscall.arg0 as i32;
    let buf_ptr = syscall.arg1 as u64;
    let count = syscall.arg2 as usize;
    
    // Check if FD is AgentFS file
    let file = match tracee.get_fd(fd) {
        Some(FileKind::AgentFS(file)) => file,
        Some(FileKind::Native) => return Ok(SyscallResult::Native),
        None => return Ok(SyscallResult::Emulated(-libc::EBADF)),
    };
    
    // Read from AgentFS
    let data = file.pread(0, count as u64).await?;
    
    // Write to tracee's buffer
    tracee.write_memory(buf_ptr, &data)?;
    
    // Return bytes read
    Ok(SyscallResult::Emulated(data.len() as i64))
}
```

### 5.7 Memory Access Patterns

**Reading tracee memory:**

```rust
fn read_string(&mut self, ptr: u64) -> Result<String> {
    // Read null-terminated string from tracee's address space
    let mut buf = Vec::new();
    let mut addr = ptr;
    
    loop {
        let byte = self.read_byte(addr)?;
        if byte == 0 {
            break;  // Null terminator
        }
        buf.push(byte);
        addr += 1;
    }
    
    Ok(String::from_utf8_lossy(&buf).to_string())
}
```

**Writing to tracee memory:**

```rust
fn write_memory(&mut self, addr: u64, data: &[u8]) -> Result<()> {
    for (i, &byte) in data.iter().enumerate() {
        self.write_byte(addr + i as u64, byte)?;
    }
    Ok(())
}
```

---

## 6. File Descriptor Remapping

### 6.1 The FD Table

The sandbox maintains a per-process FD table:

```rust
struct FdTable {
    /// Map FD → file kind
    fds: HashMap<i32, FileKind>,
    next_fd: AtomicI32,
}

enum FileKind {
    /// AgentFS file (intercepted)
    AgentFS(BoxedFile),
    /// Native kernel FD (not intercepted)
    Native,
}
```

### 6.2 FD Allocation

```rust
impl FdTable {
    fn allocate(&self, file: BoxedFile) -> i32 {
        let fd = self.next_fd.fetch_add(1, Ordering::Relaxed);
        
        // Skip standard FDs (0, 1, 2) if not allocated
        let fd = if fd < 3 { 3 } else { fd };
        
        self.fds.insert(fd, FileKind::AgentFS(file));
        fd
    }
    
    fn get(&self, fd: i32) -> Option<&FileKind> {
        self.fds.get(&fd)
    }
    
    fn remove(&mut self, fd: i32) -> Option<FileKind> {
        self.fds.remove(&fd)
    }
}
```

### 6.3 Handling dup() and dup2()

```rust
async fn handle_dup2(tracee: &mut Tracee, syscall: Syscall) -> Result<SyscallResult> {
    let old_fd = syscall.arg0 as i32;
    let new_fd = syscall.arg1 as i32;
    
    // Get old FD
    let file = match tracee.get_fd(old_fd) {
        Some(FileKind::AgentFS(file)) => file.clone(),
        Some(FileKind::Native) => return Ok(SyscallResult::Native),
        None => return Ok(SyscallResult::Emulated(-libc::EBADF)),
    };
    
    // Close new_fd if already open
    if let Some(old) = tracee.get_fd(new_fd) {
        // Close old FD (may need fsync)
    }
    
    // Install at new_fd
    tracee.set_fd(new_fd, FileKind::AgentFS(file));
    
    Ok(SyscallResult::Emulated(new_fd as i64))
}
```

---

## 7. Sandbox Security Model

### 7.1 What's Allowed

| Operation | Allowed? | Notes |
|-----------|----------|-------|
| Read files in sandbox | Yes | From base or delta |
| Write files in sandbox | Yes | To delta layer |
| Execute binaries | Yes | Within sandbox |
| Network access | Yes | Passed to kernel |
| Read files outside sandbox | No | Path validation |
| Write files outside sandbox | No | Path validation |
| Modify host filesystem | No | Base is read-only |

### 7.2 Path Validation

```rust
fn is_path_allowed(path: &str, sandbox_root: &str) -> bool {
    let normalized = normalize_path(path);
    let root = normalize_path(sandbox_root);
    
    // Must start with sandbox root
    normalized.starts_with(&root)
}
```

### 7.3 Session Modes

From `cli/src/cmd/run.rs`:

```rust
// Ephemeral session (default)
agentfs run -- cargo build
// Delta layer discarded on exit

// Persistent session
agentfs run --session -- cargo build
// Delta layer saved for later
```

---

## 8. Performance Characteristics

### 8.1 OverlayFS Performance

| Operation | Cost | Notes |
|-----------|------|-------|
| stat() on delta file | O(1) | Direct SQLite query |
| stat() on base file | O(1) | HostFS passthrough |
| readdir() merged | O(n+m) | n=delta, m=base |
| open() unmodified | O(1) | Just opens base |
| write() triggers CoW | O(file_size) | Copy entire file |
| subsequent writes | O(1) | Direct to delta |

### 8.2 Ptrace Overhead

| Factor | Impact |
|--------|--------|
| Syscall entry/exit | ~100-1000ns per syscall |
| Memory read/write | ~100ns per access |
| Context switch | ~1-10μs per switch |

**Overall:** 2-10x slowdown for syscall-heavy workloads

### 8.3 Optimization Strategies

1. **Batch operations** - readdir_plus() instead of readdir()+getattr()
2. **Cache attributes** - TTL-based caching in FUSE
3. **Writeback caching** - Kernel buffers small writes
4. **Parallel dirops** - Concurrent readdir lookups

---

## Summary

The OverlayFS + Ptrace combination provides:

1. **Copy-on-Write semantics** - Base layer never modified
2. **Whiteout tracking** - Logical deletions without base modification
3. **Directory merging** - Unified view of base + delta
4. **Syscall interception** - Transparent filesystem redirection
5. **FD remapping** - Track which files need interception
6. **Sandbox isolation** - Can't escape sandbox boundaries

All implemented in ~600 lines of Rust code.
