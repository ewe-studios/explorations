# Rust Revision: OpenContainer

**Source:** OpenContainer Deep Dives
**Repository:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/opencontainer`

This document translates all OpenWebContainer concepts to idiomatic Rust implementations, providing a complete blueprint for building a Rust version of the browser-based virtual container runtime.

---

## Table of Contents

1. [Architecture Translation](#1-architecture-translation)
2. [Virtual Filesystem in Rust](#2-virtual-filesystem-in-rust)
3. [Process Management in Rust](#3-process-management-in-rust)
4. [Shell Implementation in Rust](#4-shell-implementation-in-rust)
5. [HTTP Server and Network Simulation](#5-http-server-and-network-simulation)
6. [JavaScript Runtime Options](#6-javascript-runtime-options)
7. [WASM Compilation](#7-wasm-compilation)
8. [Tokio Integration](#8-tokio-integration)
9. [Complete Working Example](#9-complete-working-example)
10. [Cargo.toml Configuration](#10-cargotoml-configuration)

---

## 1. Architecture Translation

### 1.1 Browser-Based to Native/WASM

OpenWebContainer's browser-based architecture translates to Rust in two deployment modes:

```
┌─────────────────────────────────────────────────────────────────┐
│                    TypeScript (Original)                         │
├─────────────────────────────────────────────────────────────────┤
│  Main Thread         │  Web Worker                               │
│  - ContainerManager  │  - ShellExecutor                          │
│  - ProcessManager    │  - NodeExecutor                           │
│  - FileSystem        │  - QuickJS Runtime                        │
│                      │  - ZenFS Virtual FS                       │
│  Communication: postMessage API                                  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    Rust (Native Daemon)                          │
├─────────────────────────────────────────────────────────────────┤
│  Main Thread         │  Tokio Worker Threads                     │
│  - ContainerHandle   │  - ShellExecutor (Task)                  │
│  - ProcessRegistry   │  - NodeExecutor (Task)                    │
│  - FileSystem Arc    │  - JS Runtime (Task)                      │
│                      │  - VFS Backend (Task)                     │
│  Communication: Tokio Channels (mpsc, broadcast)                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    Rust (WASM in Browser)                        │
├─────────────────────────────────────────────────────────────────┤
│  Main Thread         │  Web Worker (via wasm-bindgen)           │
│  - Container API     │  - ShellExecutor                          │
│  - UI Bindings       │  - Process Manager                        │
│                      │  - Virtual FS                             │
│  Communication: wasm-bindgen futures + postMessage               │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 TypeScript Interfaces to Rust Traits

| TypeScript Concept | Rust Equivalent |
|-------------------|-----------------|
| `interface IFileSystem` | `trait FileSystem` |
| `class VirtualFileSystem` | `struct VirtualFileSystem` |
| `ProcessExecutor` interface | `trait ProcessExecutor` |
| `ContainerConfig` interface | `struct ContainerConfig` |
| Union types (`A \| B`) | `enum AOrB { A, B }` |
| `Promise<T>` | `Future<Output = T>` / `tokio::spawn` |
| `Map<string, T>` | `HashMap<String, T>` |
| `EventEmitter` | `tokio::sync::broadcast` |
| `postMessage` | `tokio::sync::mpsc` |

### 1.3 Architecture Translation Table

```typescript
// TypeScript: ContainerManager interface
interface ContainerManager {
  createContainer(config: ContainerConfig): Promise<Container>;
  getContainer(id: string): Container | undefined;
  destroyContainer(id: string): Promise<void>;
  spawnProcess(containerId: string, cmd: string, args: string[]): Promise<Process>;
}
```

```rust
// Rust: ContainerManager trait
pub trait ContainerManager: Send + Sync {
    async fn create_container(&self, config: ContainerConfig) -> Result<ContainerHandle>;
    fn get_container(&self, id: &str) -> Option<ContainerHandle>;
    async fn destroy_container(&self, id: &str) -> Result<()>;
    async fn spawn_process(
        &self,
        container_id: &str,
        cmd: String,
        args: Vec<String>,
    ) -> Result<ProcessHandle>;
}
```

### 1.4 Complete Architecture Diagram

```
┌────────────────────────────────────────────────────────────────────┐
│                        OpenContainer (Rust)                         │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                    Container Runtime                          │  │
│  │                                                               │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │  │
│  │  │ ContainerManager│  │ ProcessRegistry │  │ FileSystem   │ │  │
│  │  │ ─────────────── │  │ ─────────────── │  │ ──────────── │ │  │
│  │  │ - containers    │  │ - executors     │  │ - backends   │ │  │
│  │  │ - next_id       │  │ - processes     │  │ - mount_pts  │ │  │
│  │  └────────┬────────┘  └────────┬────────┘  └──────┬───────┘ │  │
│  │           │                    │                   │         │  │
│  │           │      Arc<Mutex<SharedState>>          │         │  │
│  │           │◄─────────────────────────────────────>│         │  │
│  └───────────┼────────────────────────────────────────┼─────────┘  │
│              │                    │                   │            │
│              ▼                    ▼                   ▼            │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────────┐   │
│  │ ShellExecutor   │  │  NodeExecutor   │  │ NetworkSimulator │   │
│  │ ─────────────── │  │ ─────────────── │  │ ──────────────── │   │
│  │ - command parser│  │ - JS runtime    │  │ - HTTP interceptor│  │
│  │ - builtins      │  │ - module loader │  │ - route registry │   │
│  │ - pipe support  │  │ - polyfills     │  │ - mock handlers  │   │
│  └─────────────────┘  └─────────────────┘  └──────────────────┘   │
│                                                                     │
│  Communication: tokio::sync::mpsc channels                          │
└────────────────────────────────────────────────────────────────────┘
```

---

## 2. Virtual Filesystem in Rust

### 2.1 FileSystem Trait Definition

```rust
use std::path::{Path, PathBuf};
use tokio::io::{AsyncRead, AsyncWrite};
use thiserror::Error;

/// File metadata information
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub created: std::time::SystemTime,
    pub modified: std::time::SystemTime,
    pub accessed: std::time::SystemTime,
    pub is_directory: bool,
    pub permissions: Option<u32>,
}

/// Filesystem operations error type
#[derive(Error, Debug)]
pub enum FileSystemError {
    #[error("File not found: {0}")]
    NotFound(PathBuf),
    
    #[error("Path is a directory: {0}")]
    IsDirectory(PathBuf),
    
    #[error("Path is not a directory: {0}")]
    NotDirectory(PathBuf),
    
    #[error("Directory not empty: {0}")]
    DirectoryNotEmpty(PathBuf),
    
    #[error("File already exists: {0}")]
    AlreadyExists(PathBuf),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type FileSystemResult<T> = Result<T, FileSystemError>;

/// Core filesystem trait - all implementations must satisfy this
#[async_trait::async_trait]
pub trait FileSystem: Send + Sync {
    /// Read file contents as string
    async fn read_to_string(&self, path: &Path) -> FileSystemResult<String>;
    
    /// Read file contents as bytes
    async fn read(&self, path: &Path) -> FileSystemResult<Vec<u8>>;
    
    /// Write string content to file
    async fn write(&self, path: &Path, content: &str) -> FileSystemResult<()>;
    
    /// Write bytes to file
    async fn write_bytes(&self, path: &Path, content: &[u8]) -> FileSystemResult<()>;
    
    /// Delete a file
    async fn remove_file(&self, path: &Path) -> FileSystemResult<()>;
    
    /// Create a directory
    async fn create_dir(&self, path: &Path) -> FileSystemResult<()>;
    
    /// Create directory and all parent directories
    async fn create_dir_all(&self, path: &Path) -> FileSystemResult<()>;
    
    /// Remove a directory (must be empty)
    async fn remove_dir(&self, path: &Path) -> FileSystemResult<()>;
    
    /// Remove directory and all contents
    async fn remove_dir_all(&self, path: &Path) -> FileSystemResult<()>;
    
    /// List directory contents
    async fn read_dir(&self, path: &Path) -> FileSystemResult<Vec<PathBuf>>;
    
    /// Get file/directory metadata
    async fn metadata(&self, path: &Path) -> FileSystemResult<FileMetadata>;
    
    /// Check if path exists
    async fn exists(&self, path: &Path) -> bool;
    
    /// Check if path is a directory
    async fn is_dir(&self, path: &Path) -> bool;
    
    /// Check if path is a file
    async fn is_file(&self, path: &Path) -> bool;
    
    /// Copy file to destination
    async fn copy(&self, from: &Path, to: &Path) -> FileSystemResult<u64>;
    
    /// Move/rename file or directory
    async fn rename(&self, from: &Path, to: &Path) -> FileSystemResult<()>;
}
```

### 2.2 In-Memory Filesystem Implementation

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Entry in the in-memory filesystem
#[derive(Debug, Clone)]
enum FsEntry {
    File {
        content: Vec<u8>,
        created: DateTime<Utc>,
        modified: DateTime<Utc>,
        accessed: DateTime<Utc>,
        permissions: Option<u32>,
    },
    Directory {
        created: DateTime<Utc>,
        modified: DateTime<Utc>,
        accessed: DateTime<Utc>,
        permissions: Option<u32>,
    },
}

impl Default for FsEntry {
    fn default() -> Self {
        let now = Utc::now();
        FsEntry::Directory {
            created: now,
            modified: now,
            accessed: now,
            permissions: None,
        }
    }
}

/// In-memory filesystem implementation
/// 
/// Thread-safe, async-compatible filesystem stored entirely in RAM.
/// Uses Arc<RwLock<>> for concurrent access without blocking.
pub struct InMemoryFileSystem {
    /// Root path prefix (for isolation)
    root: PathBuf,
    /// Storage: normalized path -> entry
    entries: Arc<RwLock<HashMap<PathBuf, FsEntry>>>,
}

impl InMemoryFileSystem {
    /// Create new in-memory filesystem
    pub fn new() -> Self {
        let mut entries = HashMap::new();
        entries.insert(PathBuf::from("/"), FsEntry::default());
        
        Self {
            root: PathBuf::from("/"),
            entries: Arc::new(RwLock::new(entries)),
        }
    }
    
    /// Create filesystem with isolated root
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let mut entries = HashMap::new();
        entries.insert(root.clone(), FsEntry::default());
        
        Self {
            root,
            entries: Arc::new(RwLock::new(entries)),
        }
    }
    
    /// Normalize path (resolve . and .. components)
    fn normalize_path(&self, path: &Path) -> PathBuf {
        let mut components: Vec<&std::ffi::OsStr> = Vec::new();
        
        for component in path.components() {
            match component {
                std::path::Component::Normal(c) => components.push(c),
                std::path::Component::ParentDir => { components.pop(); },
                std::path::Component::CurDir => {},
                _ => {},
            }
        }
        
        if components.is_empty() {
            self.root.clone()
        } else {
            let mut result = PathBuf::from("/");
            for c in components {
                result.push(c);
            }
            result
        }
    }
    
    /// Get parent directory of a path
    fn parent_dir(&self, path: &Path) -> Option<PathBuf> {
        path.parent().map(|p| p.to_path_buf())
    }
    
    /// Ensure parent directory exists, creating if necessary
    async fn ensure_parent(&self, path: &Path) -> FileSystemResult<()> {
        if let Some(parent) = self.parent_dir(path) {
            let normalized = self.normalize_path(&parent);
            let mut entries = self.entries.write().await;
            
            if !entries.contains_key(&normalized) {
                // Recursively create parent
                drop(entries);
                self.create_dir_all(&parent).await?;
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl FileSystem for InMemoryFileSystem {
    async fn read_to_string(&self, path: &Path) -> FileSystemResult<String> {
        let content = self.read(path).await?;
        String::from_utf8(content)
            .map_err(|e| FileSystemError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData, e
            )))
    }
    
    async fn read(&self, path: &Path) -> FileSystemResult<Vec<u8>> {
        let normalized = self.normalize_path(path);
        let entries = self.entries.read().await;
        
        match entries.get(&normalized) {
            Some(FsEntry::File { content, accessed, .. }) => {
                // Update access time (clone entry with modified time)
                drop(entries);
                let mut entries = self.entries.write().await;
                if let Some(FsEntry::File { accessed, .. }) = entries.get_mut(&normalized) {
                    *accessed = Utc::now();
                }
                Ok(content.clone())
            }
            Some(FsEntry::Directory { .. }) => {
                Err(FileSystemError::IsDirectory(normalized))
            }
            None => Err(FileSystemError::NotFound(normalized)),
        }
    }
    
    async fn write(&self, path: &Path, content: &str) -> FileSystemResult<()> {
        self.write_bytes(path, content.as_bytes()).await
    }
    
    async fn write_bytes(&self, path: &Path, content: &[u8]) -> FileSystemResult<()> {
        let normalized = self.normalize_path(path);
        
        // Ensure parent exists
        self.ensure_parent(&normalized).await?;
        
        let now = Utc::now();
        let mut entries = self.entries.write().await;
        
        entries.insert(normalized, FsEntry::File {
            content: content.to_vec(),
            created: now,
            modified: now,
            accessed: now,
            permissions: None,
        });
        
        Ok(())
    }
    
    async fn remove_file(&self, path: &Path) -> FileSystemResult<()> {
        let normalized = self.normalize_path(path);
        let mut entries = self.entries.write().await;
        
        match entries.get(&normalized) {
            Some(FsEntry::File { .. }) => {
                entries.remove(&normalized);
                Ok(())
            }
            Some(FsEntry::Directory { .. }) => {
                Err(FileSystemError::IsDirectory(normalized))
            }
            None => Err(FileSystemError::NotFound(normalized)),
        }
    }
    
    async fn create_dir(&self, path: &Path) -> FileSystemResult<()> {
        let normalized = self.normalize_path(path);
        
        // Check if parent exists
        if let Some(parent) = self.parent_dir(&normalized) {
            let entries = self.entries.read().await;
            if !entries.contains_key(&parent) {
                return Err(FileSystemError::NotFound(parent));
            }
        }
        
        let mut entries = self.entries.write().await;
        
        if entries.contains_key(&normalized) {
            return Err(FileSystemError::AlreadyExists(normalized));
        }
        
        let now = Utc::now();
        entries.insert(normalized, FsEntry::Directory {
            created: now,
            modified: now,
            accessed: now,
            permissions: None,
        });
        
        Ok(())
    }
    
    async fn create_dir_all(&self, path: &Path) -> FileSystemResult<()> {
        let normalized = self.normalize_path(path);
        
        // Build list of directories to create
        let mut to_create = Vec::new();
        let mut current = &normalized;
        
        while current != self.root && current != Path::new("/") {
            let entries = self.entries.read().await;
            if entries.contains_key(current) {
                break;
            }
            to_create.push(current.clone());
            current = current.parent().ok_or_else(|| {
                FileSystemError::InvalidPath("Cannot create root".to_string())
            })?;
        }
        
        drop(entries);
        
        // Create directories from root downward
        let now = Utc::now();
        let mut entries = self.entries.write().await;
        
        for dir in to_create.into_iter().rev() {
            entries.insert(dir, FsEntry::Directory {
                created: now,
                modified: now,
                accessed: now,
                permissions: None,
            });
        }
        
        Ok(())
    }
    
    async fn remove_dir(&self, path: &Path) -> FileSystemResult<()> {
        let normalized = self.normalize_path(path);
        let mut entries = self.entries.write().await;
        
        // Check if directory exists and is empty
        match entries.get(&normalized) {
            Some(FsEntry::Directory { .. }) => {
                // Check if empty
                let has_children = entries.keys().any(|k| {
                    k.starts_with(&normalized) && k != &normalized
                });
                
                if has_children {
                    Err(FileSystemError::DirectoryNotEmpty(normalized))
                } else {
                    entries.remove(&normalized);
                    Ok(())
                }
            }
            Some(FsEntry::File { .. }) => {
                Err(FileSystemError::NotDirectory(normalized))
            }
            None => Err(FileSystemError::NotFound(normalized)),
        }
    }
    
    async fn remove_dir_all(&self, path: &Path) -> FileSystemResult<()> {
        let normalized = self.normalize_path(path);
        let mut entries = self.entries.write().await;
        
        // Collect all paths to remove
        let to_remove: Vec<_> = entries.keys()
            .filter(|k| k.starts_with(&normalized))
            .cloned()
            .collect();
        
        for k in to_remove {
            entries.remove(&k);
        }
        
        Ok(())
    }
    
    async fn read_dir(&self, path: &Path) -> FileSystemResult<Vec<PathBuf>> {
        let normalized = self.normalize_path(path);
        let entries = self.entries.read().await;
        
        match entries.get(&normalized) {
            Some(FsEntry::Directory { .. }) => {
                let mut children = Vec::new();
                let prefix = if normalized == Path::new("/") {
                    String::from("/")
                } else {
                    format!("{}/", normalized.display())
                };
                
                for key in entries.keys() {
                    if key != &normalized && key.starts_with(&prefix) {
                        // Get immediate child only
                        let remainder = key.strip_prefix(&prefix).unwrap();
                        if let Some(first_component) = remainder.split('/').next() {
                            let child_path = PathBuf::from(format!("{}{}", prefix, first_component));
                            if !children.contains(&child_path) {
                                children.push(child_path);
                            }
                        }
                    }
                }
                
                Ok(children)
            }
            Some(FsEntry::File { .. }) => {
                Err(FileSystemError::NotDirectory(normalized))
            }
            None => Err(FileSystemError::NotFound(normalized)),
        }
    }
    
    async fn metadata(&self, path: &Path) -> FileSystemResult<FileMetadata> {
        let normalized = self.normalize_path(path);
        let entries = self.entries.read().await;
        
        match entries.get(&normalized) {
            Some(FsEntry::File { created, modified, accessed, content, permissions }) => {
                Ok(FileMetadata {
                    size: content.len() as u64,
                    created: created.into(),
                    modified: modified.into(),
                    accessed: accessed.into(),
                    is_directory: false,
                    permissions: *permissions,
                })
            }
            Some(FsEntry::Directory { created, modified, accessed, permissions }) => {
                Ok(FileMetadata {
                    size: 0,
                    created: created.into(),
                    modified: modified.into(),
                    accessed: accessed.into(),
                    is_directory: true,
                    permissions: *permissions,
                })
            }
            None => Err(FileSystemError::NotFound(normalized)),
        }
    }
    
    async fn exists(&self, path: &Path) -> bool {
        let normalized = self.normalize_path(path);
        let entries = self.entries.read().await;
        entries.contains_key(&normalized)
    }
    
    async fn is_dir(&self, path: &Path) -> bool {
        let normalized = self.normalize_path(path);
        let entries = self.entries.read().await;
        matches!(entries.get(&normalized), Some(FsEntry::Directory { .. }))
    }
    
    async fn is_file(&self, path: &Path) -> bool {
        let normalized = self.normalize_path(path);
        let entries = self.entries.read().await;
        matches!(entries.get(&normalized), Some(FsEntry::File { .. }))
    }
    
    async fn copy(&self, from: &Path, to: &Path) -> FileSystemResult<u64> {
        let content = self.read(from).await?;
        let size = content.len() as u64;
        self.write_bytes(to, &content).await?;
        Ok(size)
    }
    
    async fn rename(&self, from: &Path, to: &Path) -> FileSystemResult<()> {
        let normalized_from = self.normalize_path(from);
        let normalized_to = self.normalize_path(to);
        
        let mut entries = self.entries.write().await;
        
        match entries.remove(&normalized_from) {
            Some(entry) => {
                entries.insert(normalized_to, entry);
                Ok(())
            }
            None => Err(FileSystemError::NotFound(normalized_from)),
        }
    }
}

impl Default for InMemoryFileSystem {
    fn default() -> Self {
        Self::new()
    }
}
```

### 2.3 Persistent Filesystem with sled

```rust
use sled::{Db, Tree};

/// Persistent filesystem using sled (embedded B-tree database)
/// 
/// Provides durability across restarts with minimal overhead.
/// All operations are ACID-compliant.
pub struct SledFileSystem {
    db: Db,
    tree_name: Vec<u8>,
}

impl SledFileSystem {
    /// Open or create sled-backed filesystem
    pub fn open(path: impl AsRef<std::path::Path>, tree_name: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        Ok(Self {
            db,
            tree_name: tree_name.as_bytes().to_vec(),
        })
    }
    
    /// Get the tree for this filesystem instance
    fn tree(&self) -> Result<Tree, sled::Error> {
        self.db.open_tree(&self.tree_name)
    }
}

#[async_trait::async_trait]
impl FileSystem for SledFileSystem {
    async fn read_to_string(&self, path: &Path) -> FileSystemResult<String> {
        let bytes = self.read(path).await?;
        String::from_utf8(bytes)
            .map_err(|e| FileSystemError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData, e
            )))
    }
    
    async fn read(&self, path: &Path) -> FileSystemResult<Vec<u8>> {
        let path_key = path.to_string_lossy().into_bytes();
        let tree = self.tree().map_err(|e| {
            FileSystemError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?;
        
        match tree.get(&path_key)? {
            Some(data) => Ok(data.to_vec()),
            None => Err(FileSystemError::NotFound(path.to_path_buf())),
        }
    }
    
    async fn write(&self, path: &Path, content: &str) -> FileSystemResult<()> {
        self.write_bytes(path, content.as_bytes()).await
    }
    
    async fn write_bytes(&self, path: &Path, content: &[u8]) -> FileSystemResult<()> {
        let path_key = path.to_string_lossy().into_bytes();
        let tree = self.tree().map_err(|e| {
            FileSystemError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?;
        
        tree.insert(&path_key, content)?;
        tree.flush()?;
        Ok(())
    }
    
    // ... implement remaining methods similarly
    async fn remove_file(&self, path: &Path) -> FileSystemResult<()> {
        let path_key = path.to_string_lossy().into_bytes();
        let tree = self.tree().map_err(|e| {
            FileSystemError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?;
        
        tree.remove(&path_key)?;
        Ok(())
    }
    
    async fn create_dir(&self, path: &Path) -> FileSystemResult<()> {
        // Store directory marker
        let mut dir_path = path.to_string_lossy().to_string();
        if !dir_path.ends_with('/') {
            dir_path.push('/');
        }
        let path_key = dir_path.into_bytes();
        
        let tree = self.tree().map_err(|e| {
            FileSystemError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?;
        
        tree.insert(&path_key, vec![])?;
        tree.flush()?;
        Ok(())
    }
    
    async fn create_dir_all(&self, path: &Path) -> FileSystemResult<()> {
        let mut current = PathBuf::new();
        
        for component in path.components() {
            current.push(component);
            self.create_dir(&current).await?;
        }
        
        Ok(())
    }
    
    async fn remove_dir(&self, path: &Path) -> FileSystemResult<()> {
        // Similar to remove_file but check for children first
        let mut prefix = path.to_string_lossy().to_string();
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        let prefix_bytes = prefix.into_bytes();
        
        let tree = self.tree().map_err(|e| {
            FileSystemError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?;
        
        // Check for children
        if tree.range(&prefix_bytes..).next().is_some() {
            return Err(FileSystemError::DirectoryNotEmpty(path.to_path_buf()));
        }
        
        // Remove directory marker
        tree.remove(&prefix_bytes)?;
        Ok(())
    }
    
    async fn remove_dir_all(&self, path: &Path) -> FileSystemResult<()> {
        let mut prefix = path.to_string_lossy().to_string();
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        let prefix_bytes = prefix.into_bytes();
        
        let tree = self.tree().map_err(|e| {
            FileSystemError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?;
        
        // Remove all keys with prefix
        let mut batch = sled::Batch::default();
        for (key, _) in tree.range(&prefix_bytes..) {
            if !key.starts_with(&prefix_bytes) {
                break;
            }
            batch.remove(key);
        }
        
        tree.apply_batch(batch)?;
        tree.flush()?;
        Ok(())
    }
    
    async fn read_dir(&self, path: &Path) -> FileSystemResult<Vec<PathBuf>> {
        let mut prefix = path.to_string_lossy().to_string();
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        let prefix_bytes = prefix.clone().into_bytes();
        
        let tree = self.tree().map_err(|e| {
            FileSystemError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?;
        
        let mut children = std::collections::HashSet::new();
        
        for (key, _) in tree.range(&prefix_bytes..) {
            if !key.starts_with(&prefix_bytes) {
                break;
            }
            
            let key_str = String::from_utf8_lossy(&key);
            let remainder = key_str.strip_prefix(&prefix).unwrap();
            
            if let Some(first) = remainder.split('/').next() {
                if !first.is_empty() {
                    let child_path = format!("{}{}", prefix, first);
                    children.insert(PathBuf::from(child_path));
                }
            }
        }
        
        Ok(children.into_iter().collect())
    }
    
    async fn metadata(&self, path: &Path) -> FileSystemResult<FileMetadata> {
        // Implementation would store metadata alongside content
        // For simplicity, return basic metadata
        let exists = self.exists(path).await;
        if !exists {
            return Err(FileSystemError::NotFound(path.to_path_buf()));
        }
        
        let is_dir = path.to_string_lossy().ends_with('/');
        
        Ok(FileMetadata {
            size: 0,
            created: std::time::SystemTime::now(),
            modified: std::time::SystemTime::now(),
            accessed: std::time::SystemTime::now(),
            is_directory: is_dir,
            permissions: None,
        })
    }
    
    async fn exists(&self, path: &Path) -> bool {
        let path_key = path.to_string_lossy().into_bytes();
        let tree = match self.tree() {
            Ok(t) => t,
            Err(_) => return false,
        };
        
        // Check exact path
        if tree.contains_key(&path_key).unwrap_or(false) {
            return true;
        }
        
        // Check directory variant
        let mut dir_path = path.to_string_lossy().to_string();
        if !dir_path.ends_with('/') {
            dir_path.push('/');
        }
        tree.contains_key(dir_path.as_bytes()).unwrap_or(false)
    }
    
    async fn is_dir(&self, path: &Path) -> bool {
        path.to_string_lossy().ends_with('/') || 
        self.exists(Path::new(&format!("{}/", path.display()))).await
    }
    
    async fn is_file(&self, path: &Path) -> bool {
        self.exists(path).await && !self.is_dir(path).await
    }
    
    async fn copy(&self, from: &Path, to: &Path) -> FileSystemResult<u64> {
        let content = self.read(from).await?;
        let size = content.len() as u64;
        self.write_bytes(to, &content).await?;
        Ok(size)
    }
    
    async fn rename(&self, from: &Path, to: &Path) -> FileSystemResult<()> {
        let content = self.read(from).await?;
        self.write_bytes(to, &content).await?;
        self.remove_file(from).await?;
        Ok(())
    }
}
```

### 2.4 Path Handling Utilities

```rust
use std::path::{Path, PathBuf, Component, Prefix};

/// Path resolution utilities for OpenContainer
pub mod path_utils {
    use super::*;
    
    /// Normalize a path by resolving . and .. components
    /// 
    /// Examples:
    /// - "/app/../app/./index.js" -> "/app/index.js"
    /// - "foo/bar" -> "/foo/bar"
    /// - "./utils.js" -> "/utils.js"
    pub fn normalize(path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        let mut components: Vec<&std::ffi::OsStr> = Vec::new();
        let mut is_absolute = path.is_absolute();
        
        for component in path.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => {
                    is_absolute = true;
                    components.clear();
                }
                Component::CurDir => {},
                Component::ParentDir => {
                    components.pop();
                }
                Component::Normal(c) => {
                    components.push(c);
                }
            }
        }
        
        if is_absolute {
            let mut result = PathBuf::from("/");
            for c in components {
                result.push(c);
            }
            result
        } else {
            components.iter().collect()
        }
    }
    
    /// Resolve a path relative to a base directory
    pub fn resolve(base: impl AsRef<Path>, path: impl AsRef<Path>) -> PathBuf {
        let base = base.as_ref();
        let path = path.as_ref();
        
        if path.is_absolute() {
            normalize(path)
        } else {
            normalize(base.join(path))
        }
    }
    
    /// Join multiple path components
    pub fn join(paths: impl Iterator<Item = impl AsRef<Path>>) -> PathBuf {
        let mut result = PathBuf::new();
        for p in paths {
            result.push(p);
        }
        normalize(result)
    }
    
    /// Get the directory containing a path
    pub fn parent(path: impl AsRef<Path>) -> Option<PathBuf> {
        path.as_ref().parent().map(|p| p.to_path_buf())
    }
    
    /// Get the file name component of a path
    pub fn file_name(path: impl AsRef<Path>) -> Option<&std::ffi::OsStr> {
        path.as_ref().file_name()
    }
    
    /// Check if a path starts with a prefix (accounting for normalization)
    pub fn starts_with(path: impl AsRef<Path>, prefix: impl AsRef<Path>) -> bool {
        let norm_path = normalize(path);
        let norm_prefix = normalize(prefix);
        
        norm_path == norm_prefix || norm_path.starts_with(&norm_prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::path_utils::*;
    
    #[test]
    fn test_normalize_absolute() {
        assert_eq!(normalize("/app/src/index.js"), PathBuf::from("/app/src/index.js"));
        assert_eq!(normalize("/app/../app/index.js"), PathBuf::from("/app/index.js"));
        assert_eq!(normalize("/app/./src/../src/index.js"), PathBuf::from("/app/src/index.js"));
    }
    
    #[test]
    fn test_normalize_relative() {
        assert_eq!(normalize("foo/bar"), PathBuf::from("foo/bar"));
        assert_eq!(normalize("./utils.js"), PathBuf::from("utils.js"));
        assert_eq!(normalize("../lib/helper.js"), PathBuf::from("../lib/helper.js"));
    }
    
    #[test]
    fn test_resolve() {
        assert_eq!(
            resolve("/app/src", "./utils.js"),
            PathBuf::from("/app/src/utils.js")
        );
        assert_eq!(
            resolve("/app/src", "../lib/helper.js"),
            PathBuf::from("/app/lib/helper.js")
        );
        assert_eq!(
            resolve("/app", "/etc/passwd"),
            PathBuf::from("/etc/passwd")
        );
    }
}
```

---

## 3. Process Management in Rust

### 3.1 Process State Machine

```rust
use std::fmt;
use tokio::sync::broadcast;

/// Process execution states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Process created but not started
    Created,
    /// Process is running
    Running,
    /// Process completed successfully
    Completed,
    /// Process failed with error
    Failed,
    /// Process was terminated
    Terminated,
}

impl fmt::Display for ProcessState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessState::Created => write!(f, "created"),
            ProcessState::Running => write!(f, "running"),
            ProcessState::Completed => write!(f, "completed"),
            ProcessState::Failed => write!(f, "failed"),
            ProcessState::Terminated => write!(f, "terminated"),
        }
    }
}

/// Process type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessType {
    Shell,
    JavaScript,
    External,
}

/// Process identifier (newtype pattern)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pid(pub u32);

impl Pid {
    pub fn new(id: u32) -> Self {
        Pid(id)
    }
    
    pub fn value(&self) -> u32 {
        self.0
    }
}

/// Process statistics
#[derive(Debug, Clone)]
pub struct ProcessStats {
    pub pid: Pid,
    pub ppid: Option<Pid>,
    pub process_type: ProcessType,
    pub state: ProcessState,
    pub exit_code: Option<i32>,
    pub executable: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub start_time: Option<std::time::Instant>,
    pub end_time: Option<std::time::Instant>,
}

/// Process events for broadcasting
#[derive(Debug, Clone)]
pub enum ProcessEvent {
    Started { pid: Pid },
    Output { pid: Pid, stream: OutputType, data: Vec<u8> },
    Error { pid: Pid, error: String },
    Exited { pid: Pid, exit_code: i32 },
    Terminated { pid: Pid },
}

/// Output stream type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    Stdout,
    Stderr,
}
```

### 3.2 Base Process Trait

```rust
use async_trait::async_trait;
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;

/// Process input/output handles
pub struct ProcessIo {
    /// stdin writer
    pub stdin_tx: mpsc::Sender<Vec<u8>>,
    /// stdout stream
    pub stdout_rx: mpsc::Receiver<Vec<u8>>,
    /// stderr stream
    pub stderr_rx: mpsc::Receiver<Vec<u8>>,
}

/// Process handle for external control
#[derive(Clone)]
pub struct ProcessHandle {
    pid: Pid,
    state: Arc<RwLock<ProcessState>>,
    exit_code: Arc<RwLock<Option<i32>>>,
    event_tx: broadcast::Sender<ProcessEvent>,
}

impl ProcessHandle {
    pub fn new(
        pid: Pid,
        event_tx: broadcast::Sender<ProcessEvent>,
    ) -> Self {
        Self {
            pid,
            state: Arc::new(RwLock::new(ProcessState::Created)),
            exit_code: Arc::new(RwLock::new(None)),
            event_tx,
        }
    }
    
    pub fn pid(&self) -> Pid {
        self.pid
    }
    
    pub async fn state(&self) -> ProcessState {
        *self.state.read().await
    }
    
    pub async fn exit_code(&self) -> Option<i32> {
        *self.exit_code.read().await
    }
    
    pub async fn wait(&self) -> Result<i32, tokio::task::JoinError> {
        let mut rx = self.event_tx.subscribe();
        
        while let Ok(event) = rx.recv().await {
            if let ProcessEvent::Exited { pid, exit_code } = event {
                if pid == self.pid {
                    return Ok(exit_code);
                }
            }
        }
        
        unreachable!("Channel should not close")
    }
    
    pub async fn kill(&self) -> Result<(), ProcessError> {
        let _ = self.event_tx.send(ProcessEvent::Terminated { pid: self.pid });
        let mut state = self.state.write().await;
        *state = ProcessState::Terminated;
        Ok(())
    }
}

/// Core process trait - all process types implement this
#[async_trait]
pub trait Process: Send + Sync {
    /// Get process ID
    fn pid(&self) -> Pid;
    
    /// Get process type
    fn process_type(&self) -> ProcessType;
    
    /// Get executable path
    fn executable(&self) -> &str;
    
    /// Get command-line arguments
    fn args(&self) -> &[String];
    
    /// Get current working directory
    fn cwd(&self) -> &str;
    
    /// Get process state
    async fn state(&self) -> ProcessState;
    
    /// Get exit code (if terminated)
    async fn exit_code(&self) -> Option<i32>;
    
    /// Start the process
    async fn start(&self) -> Result<(), ProcessError>;
    
    /// Send input to stdin
    async fn write_stdin(&self, data: &[u8]) -> Result<(), ProcessError>;
    
    /// Read from stdout
    async fn read_stdout(&self) -> Option<Vec<u8>>;
    
    /// Read from stderr
    async fn read_stderr(&self) -> Option<Vec<u8>>;
    
    /// Wait for process to exit
    async fn wait(&self) -> Result<i32, ProcessError>;
    
    /// Terminate the process
    async fn kill(&self, signal: Option<i32>) -> Result<(), ProcessError>;
    
    /// Get process statistics
    fn stats(&self) -> ProcessStats;
}

/// Process execution error
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Process already started")]
    AlreadyStarted,
    
    #[error("Process not running")]
    NotRunning,
    
    #[error("Process already terminated")]
    AlreadyTerminated,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Execution error: {0}")]
    Execution(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
}

pub type ProcessResult<T> = Result<T, ProcessError>;
```

### 3.3 Shell Process Implementation

```rust
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast};

/// Shell process implementation
pub struct ShellProcess {
    pid: Pid,
    ppid: Option<Pid>,
    executable: String,
    args: Vec<String>,
    cwd: Arc<RwLock<String>>,
    env: Arc<RwLock<HashMap<String, String>>>,
    state: Arc<RwLock<ProcessState>>,
    exit_code: Arc<RwLock<Option<i32>>>,
    
    // I/O channels
    stdin_tx: mpsc::Sender<Vec<u8>>,
    stdout_tx: broadcast::Sender<Vec<u8>>,
    stderr_tx: broadcast::Sender<Vec<u8>>,
    
    // Shell executor
    shell: Arc<RwLock<crate::shell::Shell>>,
    
    // Event broadcaster
    event_tx: broadcast::Sender<ProcessEvent>,
}

impl ShellProcess {
    /// Create new shell process
    pub fn new(
        pid: Pid,
        ppid: Option<Pid>,
        args: Vec<String>,
        cwd: String,
        env: HashMap<String, String>,
        file_system: Arc<dyn FileSystem>,
        event_tx: broadcast::Sender<ProcessEvent>,
    ) -> Self {
        let (stdin_tx, _stdin_rx) = mpsc::channel(100);
        let (stdout_tx, _) = broadcast::channel(100);
        let (stderr_tx, _) = broadcast::channel(100);
        
        let shell = crate::shell::Shell::new(
            file_system,
            cwd.clone(),
            env.clone(),
        );
        
        Self {
            pid,
            ppid,
            executable: String::from("sh"),
            args,
            cwd: Arc::new(RwLock::new(cwd)),
            env: Arc::new(RwLock::new(env)),
            state: Arc::new(RwLock::new(ProcessState::Created)),
            exit_code: Arc::new(RwLock::new(None)),
            stdin_tx,
            stdout_tx,
            stderr_tx,
            shell: Arc::new(RwLock::new(shell)),
            event_tx,
        }
    }
    
    /// Run shell main loop
    async fn run_shell_loop(&self, mut input_rx: mpsc::Receiver<Vec<u8>>) -> Result<i32, ProcessError> {
        let mut input_buffer = String::new();
        
        while let Some(data) = input_rx.recv().await {
            let input = String::from_utf8_lossy(&data);
            input_buffer.push_str(&input);
            
            // Process complete lines
            while let Some(newline_pos) = input_buffer.find('\n') {
                let line = input_buffer[..newline_pos].to_string();
                input_buffer = input_buffer[newline_pos + 1..].to_string();
                
                // Execute command
                let result = {
                    let shell = self.shell.read().await;
                    shell.execute(&line).await
                };
                
                // Send output
                if !result.stdout.is_empty() {
                    let _ = self.stdout_tx.send(result.stdout.into_bytes());
                }
                if !result.stderr.is_empty() {
                    let _ = self.stderr_tx.send(result.stderr.into_bytes());
                }
                
                // Check for exit command
                if line.trim() == "exit" {
                    return result.exit_code;
                }
            }
        }
        
        Ok(0)
    }
}

#[async_trait]
impl Process for ShellProcess {
    fn pid(&self) -> Pid {
        self.pid
    }
    
    fn process_type(&self) -> ProcessType {
        ProcessType::Shell
    }
    
    fn executable(&self) -> &str {
        &self.executable
    }
    
    fn args(&self) -> &[String] {
        &self.args
    }
    
    fn cwd(&self) -> &str {
        // Note: this returns a snapshot, use cwd() for async access
        self.cwd.try_read().map(|g| g.as_str()).unwrap_or("/")
    }
    
    async fn state(&self) -> ProcessState {
        *self.state.read().await
    }
    
    async fn exit_code(&self) -> Option<i32> {
        *self.exit_code.read().await
    }
    
    async fn start(&self) -> Result<(), ProcessError> {
        let mut state = self.state.write().await;
        
        if *state != ProcessState::Created {
            return Err(ProcessError::AlreadyStarted);
        }
        
        *state = ProcessState::Running;
        drop(state);
        
        // Emit started event
        let _ = self.event_tx.send(ProcessEvent::Started { pid: self.pid });
        
        // Clone self for the task
        let this = self.clone();
        let (tx, rx) = mpsc::channel(100);
        
        // Replace stdin receiver
        self.stdin_tx = tx.clone();
        
        // Spawn shell loop in background
        tokio::spawn(async move {
            let exit_code = this.run_shell_loop(rx).await.unwrap_or(1);
            
            // Update state on exit
            *this.state.write().await = ProcessState::Completed;
            *this.exit_code.write().await = Some(exit_code);
            
            // Emit exit event
            let _ = this.event_tx.send(ProcessEvent::Exited { 
                pid: this.pid, 
                exit_code 
            });
        });
        
        Ok(())
    }
    
    async fn write_stdin(&self, data: &[u8]) -> Result<(), ProcessError> {
        self.stdin_tx.send(data.to_vec()).await
            .map_err(|_| ProcessError::NotRunning)?;
        Ok(())
    }
    
    async fn read_stdout(&self) -> Option<Vec<u8>> {
        let mut rx = self.stdout_tx.subscribe();
        rx.recv().await.ok()
    }
    
    async fn read_stderr(&self) -> Option<Vec<u8>> {
        let mut rx = self.stderr_tx.subscribe();
        rx.recv().await.ok()
    }
    
    async fn wait(&self) -> Result<i32, ProcessError> {
        let mut rx = self.event_tx.subscribe();
        
        while let Ok(event) = rx.recv().await {
            if let ProcessEvent::Exited { pid, exit_code } = event {
                if pid == self.pid {
                    return Ok(exit_code);
                }
            }
            if let ProcessEvent::Terminated { pid } = event {
                if pid == self.pid {
                    return Ok(-1);
                }
            }
        }
        
        Err(ProcessError::Execution("Channel closed".into()))
    }
    
    async fn kill(&self, signal: Option<i32>) -> Result<(), ProcessError> {
        let mut state = self.state.write().await;
        
        if *state == ProcessState::Terminated {
            return Err(ProcessError::AlreadyTerminated);
        }
        
        *state = ProcessState::Terminated;
        *self.exit_code.write().await = Some(-1);
        
        let _ = self.event_tx.send(ProcessEvent::Terminated { pid: self.pid });
        
        Ok(())
    }
    
    fn stats(&self) -> ProcessStats {
        ProcessStats {
            pid: self.pid,
            ppid: self.ppid,
            process_type: ProcessType::Shell,
            state: ProcessState::Created, // Snapshot
            exit_code: None,
            executable: self.executable.clone(),
            args: self.args.clone(),
            cwd: self.cwd.try_read().map(|s| s.clone()).unwrap_or_default(),
            start_time: None,
            end_time: None,
        }
    }
}

impl Clone for ShellProcess {
    fn clone(&self) -> Self {
        Self {
            pid: self.pid,
            ppid: self.ppid,
            executable: self.executable.clone(),
            args: self.args.clone(),
            cwd: Arc::clone(&self.cwd),
            env: Arc::clone(&self.env),
            state: Arc::clone(&self.state),
            exit_code: Arc::clone(&self.exit_code),
            stdin_tx: self.stdin_tx.clone(),
            stdout_tx: self.stdout_tx.clone(),
            stderr_tx: self.stderr_tx.clone(),
            shell: Arc::clone(&self.shell),
            event_tx: self.event_tx.clone(),
        }
    }
}
```

### 3.4 Process Manager

```rust
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use dashmap::DashMap;

/// Process executor trait
#[async_trait]
pub trait ProcessExecutor: Send + Sync {
    /// Check if this executor can handle the given executable
    fn can_execute(&self, executable: &str) -> bool;
    
    /// Execute the process
    async fn execute(
        &self,
        pid: Pid,
        executable: String,
        args: Vec<String>,
        cwd: String,
        env: HashMap<String, String>,
        ppid: Option<Pid>,
    ) -> Result<Box<dyn Process>, ProcessError>;
}

/// Process manager - handles process lifecycle
pub struct ProcessManager {
    /// Registered executors
    executors: RwLock<Vec<Arc<dyn ProcessExecutor>>>,
    /// Running processes
    processes: DashMap<Pid, Arc<dyn Process>>,
    /// Next PID counter
    next_pid: RwLock<u32>,
    /// Event broadcaster
    event_tx: broadcast::Sender<ProcessEvent>,
}

impl ProcessManager {
    /// Create new process manager
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        
        Self {
            executors: RwLock::new(Vec::new()),
            processes: DashMap::new(),
            next_pid: RwLock::new(1),
            event_tx,
        }
    }
    
    /// Register a process executor
    pub async fn register_executor(&self, executor: Arc<dyn ProcessExecutor>) {
        self.executors.write().await.push(executor);
    }
    
    /// Get next available PID
    async fn next_pid(&self) -> Pid {
        let mut counter = self.next_pid.write().await;
        let pid = Pid(*counter);
        *counter += 1;
        pid
    }
    
    /// Find executor for executable
    async fn find_executor(&self, executable: &str) -> Option<Arc<dyn ProcessExecutor>> {
        let executors = self.executors.read().await;
        
        for executor in executors.iter() {
            if executor.can_execute(executable) {
                return Some(Arc::clone(executor));
            }
        }
        
        None
    }
    
    /// Spawn a new process
    pub async fn spawn(
        &self,
        executable: String,
        args: Vec<String>,
        cwd: String,
        env: HashMap<String, String>,
        ppid: Option<Pid>,
    ) -> Result<Arc<dyn Process>, ProcessError> {
        // Find executor
        let executor = self.find_executor(&executable).await
            .ok_or_else(|| ProcessError::NotFound(format!("No executor for: {}", executable)))?;
        
        // Get next PID
        let pid = self.next_pid().await;
        
        // Create process
        let process = executor.execute(
            pid,
            executable,
            args,
            cwd,
            env,
            ppid,
        ).await?;
        
        // Store process
        let process: Arc<dyn Process> = Arc::from(process);
        self.processes.insert(pid, Arc::clone(&process));
        
        // Start process
        process.start().await?;
        
        Ok(process)
    }
    
    /// Get process by PID
    pub fn get(&self, pid: Pid) -> Option<Arc<dyn Process>> {
        self.processes.get(&pid).map(|r| Arc::clone(r.value()))
    }
    
    /// List all processes
    pub fn list(&self) -> Vec<Arc<dyn Process>> {
        self.processes.iter().map(|r| Arc::clone(r.value())).collect()
    }
    
    /// Kill process by PID
    pub async fn kill(&self, pid: Pid, signal: Option<i32>) -> Result<(), ProcessError> {
        let process = self.get(pid)
            .ok_or_else(|| ProcessError::NotFound(format!("Process {} not found", pid.0)))?;
        
        process.kill(signal).await?;
        
        // Remove from registry after it exits
        self.processes.remove(&pid);
        
        Ok(())
    }
    
    /// Kill all processes
    pub async fn kill_all(&self) {
        let pids: Vec<_> = self.processes.iter().map(|r| *r.key()).collect();
        
        for pid in pids {
            let _ = self.kill(pid, None).await;
        }
    }
    
    /// Subscribe to process events
    pub fn subscribe(&self) -> broadcast::Receiver<ProcessEvent> {
        self.event_tx.subscribe()
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
```

### 3.5 Signal Handling

```rust
use tokio::signal;
use tokio::sync::watch;

/// Signal handler for graceful shutdown
pub struct SignalHandler {
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
}

impl SignalHandler {
    /// Create new signal handler
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self {
            shutdown_tx: tx,
            shutdown_rx: rx,
        }
    }
    
    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&mut self) {
        let mut rx = self.shutdown_tx.subscribe();
        tokio::spawn(async move {
            // Handle Ctrl+C
            let ctrl_c = signal::ctrl_c();
            
            // Handle SIGTERM (Unix only)
            #[cfg(unix)]
            let sigterm = {
                use tokio::signal::unix::{signal, SignalKind};
                signal(SignalKind::terminate())
            };
            
            #[cfg(unix)]
            tokio::select! {
                _ = ctrl_c => {},
                _ = sigterm.unwrap().recv() => {},
            }
            
            #[cfg(not(unix))]
            {
                let _ = ctrl_c.await;
            }
            
            // Signal shutdown
            let _ = rx.send(true);
        });
        
        // Wait for shutdown signal
        while !*self.shutdown_rx.borrow() {
            self.shutdown_rx.changed().await.ok();
        }
    }
    
    /// Trigger shutdown
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
    
    /// Check if shutdown was requested
    pub fn is_shutdown(&self) -> bool {
        *self.shutdown_rx.borrow()
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## 4. Shell Implementation in Rust

### 4.1 Command Parsing with nom

```rust
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while1, is_not},
    character::complete::{char, multispace0, space0},
    combinator::{map, opt, recognize, eof},
    multi::{separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, terminated},
    IResult,
};

/// Parsed shell command
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub executable: String,
    pub args: Vec<String>,
    pub redirects: Vec<Redirect>,
    pub pipes: Vec<Command>,
}

/// IO redirection
#[derive(Debug, Clone, PartialEq)]
pub enum Redirect {
    /// > file (stdout redirect)
    Stdout(String),
    /// >> file (stdout append)
    StdoutAppend(String),
    /// 2> file (stderr redirect)
    Stderr(String),
    /// &> file (both stdout and stderr)
    Both(String),
    /// < file (stdin from file)
    Stdin(String),
}

/// Parse a complete shell command line
pub fn parse_command(input: &str) -> IResult<&str, Command> {
    let (input, parts) = parse_pipeline(input)?;
    
    Ok((input, Command {
        executable: parts.first().map(|s| s.clone()).unwrap_or_default(),
        args: parts.get(1..).map(|s| s.to_vec()).unwrap_or_default(),
        redirects: Vec::new(),
        pipes: Vec::new(),
    }))
}

/// Parse pipeline (commands separated by |)
fn parse_pipeline(input: &str) -> IResult<&str, Vec<String>> {
    let (input, commands) = separated_list1(
        delimited(space0, char('|'), space0),
        parse_simple_command
    )(input)?;
    
    Ok((input, commands.into_iter().flatten().collect()))
}

/// Parse simple command (no pipes)
fn parse_simple_command(input: &str) -> IResult<&str, Vec<String>> {
    let (input, args) = separated_list1(
        multispace0,
        alt((
            parse_quoted_string,
            parse_unquoted_word,
        ))
    )(input)?;
    
    Ok((input, args))
}

/// Parse quoted string (double or single quotes)
fn parse_quoted_string(input: &str) -> IResult<&str, String> {
    alt((
        map(
            delimited(char('"'), take_while(|c| c != '"'), char('"')),
            |s: &str| s.to_string()
        ),
        map(
            delimited(char('\''), take_while(|c| c != '\''), char('\'')),
            |s: &str| s.to_string()
        ),
    ))(input)
}

/// Parse unquoted word
fn parse_unquoted_string(input: &str) -> IResult<&str, String> {
    map(
        take_while1(|c: char| !c.is_whitespace() && c != '|' && c != '>' && c != '<' && c != '&' && c != '#'),
        |s: &str| s.to_string()
    )(input)
}

/// Parse redirections
fn parse_redirect(input: &str) -> IResult<&str, Redirect> {
    alt((
        map(
            preceded(tag("&>"), preceded(space0, take_while1(|c: char| !c.is_whitespace()))),
            |f: &str| Redirect::Both(f.to_string())
        ),
        map(
            preceded(tag(">>"), preceded(space0, take_while1(|c: char| !c.is_whitespace()))),
            |f: &str| Redirect::StdoutAppend(f.to_string())
        ),
        map(
            preceded(tag(">"), preceded(space0, take_while1(|c: char| !c.is_whitespace()))),
            |f: &str| Redirect::Stdout(f.to_string())
        ),
        map(
            preceded(tag("2>"), preceded(space0, take_while1(|c: char| !c.is_whitespace()))),
            |f: &str| Redirect::Stderr(f.to_string())
        ),
        map(
            preceded(tag("<"), preceded(space0, take_while1(|c: char| !c.is_whitespace()))),
            |f: &str| Redirect::Stdin(f.to_string())
        ),
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_command() {
        let result = parse_command("ls -la").unwrap();
        assert_eq!(result.1.executable, "ls");
        assert_eq!(result.1.args, vec!["-la"]);
    }
    
    #[test]
    fn test_quoted_args() {
        let result = parse_command("echo \"hello world\"").unwrap();
        assert_eq!(result.1.executable, "echo");
        assert_eq!(result.1.args, vec!["hello world"]);
    }
    
    #[test]
    fn test_redirect() {
        let result = parse_command("cat file.txt > output.txt").unwrap();
        assert_eq!(result.1.executable, "cat");
    }
}
```

### 4.2 Built-in Commands as Trait Objects

```rust
use async_trait::async_trait;
use std::sync::Arc;

/// Result of command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandResult {
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: 0,
        }
    }
    
    pub fn error(stderr: impl Into<String>) -> Self {
        Self {
            stdout: String::new(),
            stderr: stderr.into(),
            exit_code: 1,
        }
    }
}

/// Command execution context
pub struct CommandContext {
    pub cwd: String,
    pub env: HashMap<String, String>,
    pub file_system: Arc<dyn FileSystem>,
}

/// Built-in command trait
#[async_trait]
pub trait BuiltinCommand: Send + Sync {
    /// Command name
    fn name(&self) -> &str;
    
    /// Command help text
    fn help(&self) -> &str;
    
    /// Execute the command
    async fn execute(&self, args: &[String], ctx: &CommandContext) -> CommandResult;
}

/// Command registry
pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn BuiltinCommand>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
        };
        
        // Register built-in commands
        registry.register(Arc::new(CdCommand));
        registry.register(Arc::new(LsCommand));
        registry.register(Arc::new(PwdCommand));
        registry.register(Arc::new(MkdirCommand));
        registry.register(Arc::new(TouchCommand));
        registry.register(Arc::new(RmCommand));
        registry.register(Arc::new(CpCommand));
        registry.register(Arc::new(MvCommand));
        registry.register(Arc::new(CatCommand));
        registry.register(Arc::new(EchoCommand));
        
        registry
    }
    
    pub fn register(&mut self, command: Arc<dyn BuiltinCommand>) {
        self.commands.insert(command.name().to_string(), command);
    }
    
    pub fn get(&self, name: &str) -> Option<Arc<dyn BuiltinCommand>> {
        self.commands.get(name).map(Arc::clone)
    }
    
    pub fn list(&self) -> Vec<&str> {
        self.commands.keys().map(|s| s.as_str()).collect()
    }
}

/// cd command
struct CdCommand;

#[async_trait]
impl BuiltinCommand for CdCommand {
    fn name(&self) -> &str { "cd" }
    
    fn help(&self) -> &str { "Change current directory" }
    
    async fn execute(&self, args: &[String], ctx: &CommandContext) -> CommandResult {
        let target = args.first()
            .map(|s| s.as_str())
            .unwrap_or(&ctx.env.get("HOME").cloned().unwrap_or("/".to_string()));
        
        // Resolve path
        let resolved = if target.starts_with('/') {
            target.clone()
        } else {
            format!("{}/{}", ctx.cwd, target)
        };
        
        // Normalize path
        let normalized = crate::path_utils::normalize(&resolved);
        
        // Check if directory exists
        if !ctx.file_system.exists(&normalized).await {
            return CommandResult::error(format!("cd: no such directory: {}", target));
        }
        
        if !ctx.file_system.is_dir(&normalized).await {
            return CommandResult::error(format!("cd: not a directory: {}", target));
        }
        
        // Note: In real implementation, we'd update the shell's cwd
        CommandResult::success("")
    }
}

/// ls command
struct LsCommand;

#[async_trait]
impl BuiltinCommand for LsCommand {
    fn name(&self) -> &str { "ls" }
    
    fn help(&self) -> &str { "List directory contents" }
    
    async fn execute(&self, args: &[String], ctx: &CommandContext) -> CommandResult {
        // Parse flags
        let mut show_all = false;
        let mut long_format = false;
        let mut target = String::from(".");
        
        for arg in args {
            if arg.starts_with('-') {
                if arg.contains('a') {
                    show_all = true;
                }
                if arg.contains('l') {
                    long_format = true;
                }
            } else {
                target = arg.clone();
            }
        }
        
        // Resolve path
        let path = if target.starts_with('/') {
            crate::path_utils::normalize(&target)
        } else {
            crate::path_utils::resolve(&ctx.cwd, &target)
        };
        
        // Read directory
        match ctx.file_system.read_dir(&path).await {
            Ok(entries) => {
                let mut output = String::new();
                
                for entry in entries {
                    let name = entry.file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    
                    // Skip hidden files unless -a
                    if !show_all && name.starts_with('.') {
                        continue;
                    }
                    
                    if long_format {
                        // Get metadata for long format
                        if let Ok(meta) = ctx.file_system.metadata(&entry).await {
                            output.push_str(&format!(
                                "{} {} {}\n",
                                if meta.is_directory { "d" } else { "-" },
                                meta.size,
                                name
                            ));
                        }
                    } else {
                        output.push_str(&name);
                        output.push('\n');
                    }
                }
                
                CommandResult::success(output)
            }
            Err(e) => CommandResult::error(format!("ls: {}", e)),
        }
    }
}

/// pwd command
struct PwdCommand;

#[async_trait]
impl BuiltinCommand for PwdCommand {
    fn name(&self) -> &str { "pwd" }
    
    fn help(&self) -> &str { "Print working directory" }
    
    async fn execute(&self, _args: &[String], ctx: &CommandContext) -> CommandResult {
        CommandResult::success(format!("{}\n", ctx.cwd))
    }
}

/// mkdir command
struct MkdirCommand;

#[async_trait]
impl BuiltinCommand for MkdirCommand {
    fn name(&self) -> &str { "mkdir" }
    
    fn help(&self) -> &str { "Create directory" }
    
    async fn execute(&self, args: &[String], ctx: &CommandContext) -> CommandResult {
        let mut recursive = false;
        let mut paths = Vec::new();
        
        for arg in args {
            if arg == "-p" || arg == "--parents" {
                recursive = true;
            } else {
                paths.push(arg.clone());
            }
        }
        
        if paths.is_empty() {
            return CommandResult::error("mkdir: missing operand");
        }
        
        for path in paths {
            let resolved = crate::path_utils::resolve(&ctx.cwd, &path);
            
            let result = if recursive {
                ctx.file_system.create_dir_all(&resolved).await
            } else {
                ctx.file_system.create_dir(&resolved).await
            };
            
            if let Err(e) = result {
                return CommandResult::error(format!("mkdir: {}: {}", path, e));
            }
        }
        
        CommandResult::success("")
    }
}

/// touch command
struct TouchCommand;

#[async_trait]
impl BuiltinCommand for TouchCommand {
    fn name(&self) -> &str { "touch" }
    
    fn help(&self) -> &str { "Create empty file" }
    
    async fn execute(&self, args: &[String], ctx: &CommandContext) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("touch: missing operand");
        }
        
        for path in args {
            let resolved = crate::path_utils::resolve(&ctx.cwd, path);
            
            if !ctx.file_system.exists(&resolved).await {
                if let Err(e) = ctx.file_system.write(&resolved, "").await {
                    return CommandResult::error(format!("touch: {}: {}", path, e));
                }
            }
        }
        
        CommandResult::success("")
    }
}

/// rm command
struct RmCommand;

#[async_trait]
impl BuiltinCommand for RmCommand {
    fn name(&self) -> &str { "rm" }
    
    fn help(&self) -> &str { "Remove file or directory" }
    
    async fn execute(&self, args: &[String], ctx: &CommandContext) -> CommandResult {
        let mut recursive = false;
        let mut force = false;
        let mut paths = Vec::new();
        
        for arg in args {
            match arg.as_str() {
                "-r" | "-R" | "--recursive" => recursive = true,
                "-f" | "--force" => force = true,
                _ => paths.push(arg.clone()),
            }
        }
        
        if paths.is_empty() {
            return CommandResult::error("rm: missing operand");
        }
        
        for path in paths {
            let resolved = crate::path_utils::resolve(&ctx.cwd, &path);
            
            let result = if recursive {
                ctx.file_system.remove_dir_all(&resolved).await
                    .or_else(|_| ctx.file_system.remove_file(&resolved).await)
            } else {
                ctx.file_system.remove_file(&resolved).await
            };
            
            if let Err(e) = result {
                if !force {
                    return CommandResult::error(format!("rm: {}: {}", path, e));
                }
            }
        }
        
        CommandResult::success("")
    }
}

// ... similar implementations for cp, mv, cat, echo
```

### 4.3 Shell Core Implementation

```rust
use std::sync::Arc;

/// Shell state and execution engine
pub struct Shell {
    cwd: Arc<RwLock<String>>,
    env: Arc<RwLock<HashMap<String, String>>>,
    file_system: Arc<dyn FileSystem>,
    command_registry: CommandRegistry,
    history: Vec<String>,
    history_index: usize,
}

impl Shell {
    /// Create new shell
    pub fn new(
        file_system: Arc<dyn FileSystem>,
        cwd: String,
        env: HashMap<String, String>,
    ) -> Self {
        let mut shell_env = env;
        shell_env.entry("PWD".to_string()).or_insert(cwd.clone());
        shell_env.entry("PATH".to_string()).or_insert("/bin:/usr/bin".to_string());
        shell_env.entry("HOME".to_string()).or_insert("/home".to_string());
        
        Self {
            cwd: Arc::new(RwLock::new(cwd)),
            env: Arc::new(RwLock::new(shell_env)),
            file_system,
            command_registry: CommandRegistry::new(),
            history: Vec::new(),
            history_index: 0,
        }
    }
    
    /// Execute a command line
    pub async fn execute(&self, input: &str) -> CommandResult {
        let input = input.trim();
        
        // Handle empty input
        if input.is_empty() {
            return CommandResult::success("");
        }
        
        // Handle history navigation
        if input == "!!" {
            if let Some(last) = self.history.last() {
                return self.execute(last).await;
            }
            return CommandResult::error("!!: no previous command");
        }
        
        // Parse command
        match parse_command(input) {
            Ok((_, command)) => {
                // Add to history
                self.history.push(input.to_string());
                self.history_index = self.history.len();
                
                // Execute command
                self.execute_command(&command).await
            }
            Err(e) => {
                CommandResult::error(format!("parse error: {}", e))
            }
        }
    }
    
    /// Execute parsed command
    async fn execute_command(&self, command: &crate::parser::Command) -> CommandResult {
        // Handle pipes (simplified)
        if !command.pipes.is_empty() {
            return self.execute_pipeline(command).await;
        }
        
        // Handle redirections
        if !command.redirects.is_empty() {
            return self.execute_with_redirects(command).await;
        }
        
        // Simple command execution
        self.execute_simple(&command.executable, &command.args).await
    }
    
    /// Execute simple command
    async fn execute_simple(&self, executable: &str, args: &[String]) -> CommandResult {
        // Check for built-in command
        if let Some(builtin) = self.command_registry.get(executable) {
            let ctx = CommandContext {
                cwd: self.cwd.read().await.clone(),
                env: self.env.read().await.clone(),
                file_system: Arc::clone(&self.file_system),
            };
            return builtin.execute(args, &ctx).await;
        }
        
        // Check for external command (would spawn actual process)
        // For browser-based container, this would be simulated
        CommandResult::error(format!("command not found: {}", executable))
    }
    
    /// Execute pipeline
    async fn execute_pipeline(&self, command: &crate::parser::Command) -> CommandResult {
        // Execute first command
        let mut result = self.execute_simple(&command.executable, &command.args).await;
        
        // Pipe through remaining commands
        for piped_cmd in &command.pipes {
            if result.exit_code != 0 {
                break;
            }
            
            // For now, just execute sequentially
            // Real implementation would connect stdout->stdin
            result = self.execute_simple(&piped_cmd.executable, &piped_cmd.args).await;
        }
        
        result
    }
    
    /// Execute with redirections
    async fn execute_with_redirects(&self, command: &crate::parser::Command) -> CommandResult {
        let mut result = self.execute_simple(&command.executable, &command.args).await;
        
        for redirect in &command.redirects {
            if result.exit_code != 0 {
                break;
            }
            
            match redirect {
                Redirect::Stdout(file) => {
                    let path = crate::path_utils::resolve(&self.cwd.read().await, file);
                    if let Err(e) = self.file_system.write(&path, &result.stdout).await {
                        return CommandResult::error(format!("redirect error: {}", e));
                    }
                    result.stdout = String::new();
                }
                Redirect::StdoutAppend(file) => {
                    let path = crate::path_utils::resolve(&self.cwd.read().await, file);
                    let existing = self.file_system.read_to_string(&path).await.unwrap_or_default();
                    let new_content = format!("{}{}", existing, result.stdout);
                    if let Err(e) = self.file_system.write(&path, &new_content).await {
                        return CommandResult::error(format!("redirect error: {}", e));
                    }
                    result.stdout = String::new();
                }
                // ... handle other redirect types
                _ => {}
            }
        }
        
        result
    }
    
    /// Get current prompt
    pub async fn prompt(&self) -> String {
        let cwd = self.cwd.read().await;
        let user = self.env.read().await.get("USER").cloned().unwrap_or("user".to_string());
        format!("{}@{}:{}$ ", user, "container", cwd)
    }
}
```

### 4.4 Interactive Shell with rustyline

```rust
use rustyline::{
    Editor, CompletionHelper, Completer, Helper, Highlighter, Hinter,
    error::ReadlineError,
    completion::{Completer, Candidate, FilenameCompleter},
    hint::{Hinter, HistoryHinter},
    validate::{Validator, ValidationContext, ValidationResult},
    history::DefaultHistory,
};
use rustyline::highlight::Highlighter as RustylineHighlighter;

/// Shell completer for tab completion
struct ShellCompleter {
    filename_completer: FilenameCompleter,
}

impl Completer for ShellCompleter {
    type Candidate = Candidate;
    
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Candidate>), ReadlineError> {
        self.filename_completer.complete(line, pos, ctx)
    }
}

/// Shell highlighter for syntax highlighting
struct ShellHighlighter;

impl Highlighter for ShellHighlighter {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        // Add ANSI color codes
        if default {
            Cow::Owned(format!("\x1b[1;32m{}\x1b[0m", prompt))
        } else {
            Cow::Borrowed(prompt)
        }
    }
    
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[36m{}\x1b[0m", hint))
    }
}

/// Interactive shell runner
pub struct InteractiveShell {
    editor: Editor<ShellHelper, DefaultHistory>,
    shell: Arc<Shell>,
}

struct ShellHelper {
    completer: ShellCompleter,
    hinter: HistoryHinter,
}

impl Helper for ShellHelper {}
impl Completer for ShellHelper {
    type Candidate = Candidate;
    fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>) 
        -> Result<(usize, Vec<Candidate>), ReadlineError> 
    {
        self.completer.complete(line, pos, ctx)
    }
}
impl Hinter for ShellHelper {
    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}
impl Highlighter for ShellHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        ShellHighlighter.highlight_prompt(prompt, default)
    }
}

impl InteractiveShell {
    /// Create new interactive shell
    pub fn new(shell: Arc<Shell>) -> Result<Self, ReadlineError> {
        let mut editor = Editor::new()?;
        
        let helper = ShellHelper {
            completer: ShellCompleter {
                filename_completer: FilenameCompleter::new(),
            },
            hinter: HistoryHinter::new(),
        };
        
        editor.set_helper(Some(helper));
        editor.load_history(".shell_history").ok();
        
        Ok(Self { editor, shell })
    }
    
    /// Run interactive shell loop
    pub async fn run(&mut self) -> Result<(), ReadlineError> {
        loop {
            // Get prompt
            let prompt = self.shell.prompt().await;
            
            // Read line
            match self.editor.readline(&prompt) {
                Ok(line) => {
                    // Add to history
                    self.editor.add_history_entry(line.as_str())?;
                    
                    // Execute command
                    let result = self.shell.execute(&line).await;
                    
                    // Print output
                    if !result.stdout.is_empty() {
                        print!("{}", result.stdout);
                    }
                    if !result.stderr.is_empty() {
                        eprint!("{}", result.stderr);
                    }
                    
                    // Check for exit
                    if line.trim() == "exit" {
                        break;
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl+C - print newline
                    println!();
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl+D - exit
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }
        
        // Save history
        self.editor.save_history(".shell_history").ok();
        
        Ok(())
    }
}
```

---

## 5. HTTP Server and Network Simulation

### 5.1 HTTP Server with Axum

```rust
use axum::{
    Router,
    routing::{get, post, put, delete},
    extract::{State, Path, Query},
    response::{Response, IntoResponse},
    http::StatusCode,
    Json,
};
use tokio::sync::RwLock;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// Application state
#[derive(Clone)]
pub struct AppState {
    file_system: Arc<dyn FileSystem>,
    process_manager: Arc<ProcessManager>,
}

/// Create the HTTP router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        
        // Filesystem endpoints
        .route("/fs/*path", get(read_file))
        .route("/fs/*path", put(write_file))
        .route("/fs/*path", delete(delete_file))
        .route("/fs/*path", get(list_directory))
        
        // Process endpoints
        .route("/processes", post(spawn_process))
        .route("/processes/:pid", get(get_process))
        .route("/processes/:pid/kill", post(kill_process))
        
        // Container endpoints
        .route("/containers", post(create_container))
        .route("/containers/:id", get(get_container))
        .route("/containers/:id/destroy", post(destroy_container))
        
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// File system handlers
async fn read_file(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let path = std::path::Path::new(&path);
    
    match state.file_system.read_to_string(path).await {
        Ok(content) => (StatusCode::OK, content).into_response(),
        Err(FileSystemError::NotFound(_)) => {
            (StatusCode::NOT_FOUND, "File not found").into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn write_file(
    State(state): State<AppState>,
    Path(path): Path<String>,
    body: String,
) -> impl IntoResponse {
    let path = std::path::Path::new(&path);
    
    match state.file_system.write(path, &body).await {
        Ok(()) => (StatusCode::CREATED, "File created").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn delete_file(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let path = std::path::Path::new(&path);
    
    match state.file_system.remove_file(path).await {
        Ok(()) => (StatusCode::NO_CONTENT, "").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_directory(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let path = std::path::Path::new(&path);
    
    match state.file_system.read_dir(path).await {
        Ok(entries) => {
            let names: Vec<_> = entries.iter()
                .filter_map(|e| e.file_name())
                .filter_map(|n| n.to_str())
                .map(String::from)
                .collect();
            Json(names).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// Process handlers
#[derive(Deserialize)]
struct SpawnProcessRequest {
    executable: String,
    args: Option<Vec<String>>,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,
}

async fn spawn_process(
    State(state): State<AppState>,
    Json(req): Json<SpawnProcessRequest>,
) -> impl IntoResponse {
    match state.process_manager.spawn(
        req.executable,
        req.args.unwrap_or_default(),
        req.cwd.unwrap_or("/".to_string()),
        req.env.unwrap_or_default(),
        None,
    ).await {
        Ok(process) => {
            Json(serde_json::json!({
                "pid": process.pid().value(),
                "status": "started"
            })).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn get_process(
    State(state): State<AppState>,
    Path(pid): Path<u32>,
) -> impl IntoResponse {
    let pid = Pid::new(pid);
    
    match state.process_manager.get(pid) {
        Some(process) => {
            let stats = process.stats();
            Json(serde_json::json!({
                "pid": stats.pid.value(),
                "type": format!("{:?}", stats.process_type),
                "state": format!("{:?}", stats.state),
                "executable": stats.executable,
                "args": stats.args,
            })).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Process not found").into_response(),
    }
}

async fn kill_process(
    State(state): State<AppState>,
    Path(pid): Path<u32>,
) -> impl IntoResponse {
    let pid = Pid::new(pid);
    
    match state.process_manager.kill(pid, None).await {
        Ok(()) => (StatusCode::NO_CONTENT, "").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// Container handlers would follow similar pattern
```

### 5.2 Network Simulation Layer

```rust
use axum::body::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP method
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

/// Network request
#[derive(Debug, Clone)]
pub struct NetworkRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Bytes>,
}

/// Network response
#[derive(Debug, Clone)]
pub struct NetworkResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

/// Network handler function type
pub type NetworkHandler = Arc<dyn Fn(NetworkRequest) -> futures::future::BoxFuture<'static, Result<NetworkResponse, String>> + Send + Sync>;

/// Route matcher
pub enum RouteMatcher {
    Exact(String),
    Prefix(String),
    Regex(regex::Regex),
}

impl RouteMatcher {
    pub fn matches(&self, path: &str) -> bool {
        match self {
            RouteMatcher::Exact(s) => path == s,
            RouteMatcher::Prefix(p) => path.starts_with(p),
            RouteMatcher::Regex(r) => r.is_match(path),
        }
    }
}

/// Registered route
pub struct Route {
    pub methods: Vec<HttpMethod>,
    pub matcher: RouteMatcher,
    pub handler: NetworkHandler,
    pub priority: i32,
}

/// Network simulator / HTTP interceptor
pub struct NetworkSimulator {
    routes: RwLock<Vec<Route>>,
    default_handler: RwLock<Option<NetworkHandler>>,
    enabled: RwLock<bool>,
    request_log: RwLock<Vec<NetworkRequest>>,
}

impl NetworkSimulator {
    pub fn new() -> Self {
        Self {
            routes: RwLock::new(Vec::new()),
            default_handler: RwLock::new(None),
            enabled: RwLock::new(false),
            request_log: RwLock::new(Vec::new()),
        }
    }
    
    /// Register a route handler
    pub async fn intercept<F>(
        &self,
        method: HttpMethod,
        path_pattern: impl Into<String>,
        handler: F,
    ) where
        F: Fn(NetworkRequest) -> futures::future::BoxFuture<'static, Result<NetworkResponse, String>> 
            + Send + Sync + 'static,
    {
        let mut routes = self.routes.write().await;
        
        routes.push(Route {
            methods: vec![method],
            matcher: RouteMatcher::Exact(path_pattern.into()),
            handler: Arc::new(handler),
            priority: routes.len() as i32,
        });
        
        // Sort by priority
        routes.sort_by(|a, b| a.priority.cmp(&b.priority));
    }
    
    /// Intercept with regex pattern
    pub async fn intercept_regex<F>(
        &self,
        method: HttpMethod,
        pattern: regex::Regex,
        handler: F,
    ) where
        F: Fn(NetworkRequest) -> futures::future::BoxFuture<'static, Result<NetworkResponse, String>> 
            + Send + Sync + 'static,
    {
        let mut routes = self.routes.write().await;
        
        routes.push(Route {
            methods: vec![method],
            matcher: RouteMatcher::Regex(pattern),
            handler: Arc::new(handler),
            priority: routes.len() as i32,
        });
    }
    
    /// Set default handler for unmatched requests
    pub async fn set_default_handler<F>(&self, handler: F) where
        F: Fn(NetworkRequest) -> futures::future::BoxFuture<'static, Result<NetworkResponse, String>> 
            + Send + Sync + 'static,
    {
        *self.default_handler.write().await = Some(Arc::new(handler));
    }
    
    /// Process a request through the simulator
    pub async fn process_request(&self, request: NetworkRequest) -> Result<NetworkResponse, String> {
        // Log request
        self.request_log.write().await.push(request.clone());
        
        // Check if enabled
        if !*self.enabled.read().await {
            return self.passthrough(request).await;
        }
        
        // Find matching route
        let routes = self.routes.read().await;
        
        for route in routes.iter() {
            if route.methods.contains(&request.method) {
                // Extract path from URL
                let path = url::Url::parse(&request.url)
                    .map(|u| u.path().to_string())
                    .unwrap_or_else(|_| request.url.clone());
                
                if route.matcher.matches(&path) {
                    return route.handler(request).await;
                }
            }
        }
        
        // Use default handler if available
        if let Some(handler) = &*self.default_handler.read().await {
            return handler(request).await;
        }
        
        // No match - return 404
        Ok(NetworkResponse {
            status: 404,
            headers: HashMap::new(),
            body: Bytes::from("Not Found"),
        })
    }
    
    /// Passthrough to real network (or simulated failure)
    async fn passthrough(&self, _request: NetworkRequest) -> Result<NetworkResponse, String> {
        // In browser context, this would fail
        // In native context, could make real HTTP request
        Err("Network access disabled".to_string())
    }
    
    /// Enable interception
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
    }
    
    /// Disable interception
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
    }
    
    /// Get request log
    pub async fn get_log(&self) -> Vec<NetworkRequest> {
        self.request_log.read().await.clone()
    }
    
    /// Clear request log
    pub async fn clear_log(&self) {
        *self.request_log.write().await = Vec::new();
    }
}

impl Default for NetworkSimulator {
    fn default() -> Self {
        Self::new()
    }
}

// Helper response builders
impl NetworkResponse {
    pub fn json(status: u16, data: impl Serialize) -> Result<Self, serde_json::Error> {
        let body = serde_json::to_string(&data)?;
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        Ok(Self {
            status,
            headers,
            body: Bytes::from(body),
        })
    }
    
    pub fn text(status: u16, text: impl Into<String>) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Bytes::from(text.into()),
        }
    }
}
```

### 5.3 WebSocket Support with tokio-tungstenite

```rust
use tokio_tungstenite::{
    accept_async,
    tungstenite::Message,
};
use axum::{
    extract::ws::{WebSocketUpgrade, WebSocket},
    response::Response,
};
use futures::{SinkExt, StreamExt};

/// WebSocket connection handler
pub async fn handle_ws_connection(
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(handle_websocket)
}

async fn handle_websocket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    
    // Spawn task to handle incoming messages
    let tx = sender.clone();
    tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Process text message
                    println!("Received: {}", text);
                    
                    // Echo back
                    if let Err(e) = tx.send(Message::Text(format!("Echo: {}", text))).await {
                        eprintln!("Send error: {}", e);
                        break;
                    }
                }
                Ok(Message::Binary(data)) => {
                    println!("Received binary: {} bytes", data.len());
                }
                Ok(Message::Close(close_frame)) => {
                    println!("Connection closed: {:?}", close_frame);
                    break;
                }
                Ok(Message::Ping(data)) => {
                    // Respond with pong
                    if let Err(e) = tx.send(Message::Pong(data)).await {
                        eprintln!("Pong error: {}", e);
                        break;
                    }
                }
                Ok(Message::Pong(_)) => {}
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    });
}

/// WebSocket message types for container communication
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "spawn_process")]
    SpawnProcess {
        executable: String,
        args: Vec<String>,
    },
    #[serde(rename = "write_stdin")]
    WriteStdin {
        pid: u32,
        data: String,
    },
    #[serde(rename = "process_output")]
    ProcessOutput {
        pid: u32,
        stream: String,
        data: String,
    },
    #[serde(rename = "process_exit")]
    ProcessExit {
        pid: u32,
        exit_code: i32,
    },
    #[serde(rename = "fs_write")]
    FsWrite {
        path: String,
        content: String,
    },
    #[serde(rename = "fs_read")]
    FsRead {
        path: String,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
    },
}
```

### 5.4 Mocking with mockito

```rust
#[cfg(test)]
mod tests {
    use mockito::{Server, Mock};
    use super::*;
    
    #[tokio::test]
    async fn test_network_mock() {
        let mut server = Server::new_async().await;
        
        let mock = server.mock("GET", "/api/users")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"users": []}"#)
            .create();
        
        // Make request to mock server
        let client = reqwest::Client::new();
        let response = client.get(format!("{}/api/users", server.url()))
            .send()
            .await
            .unwrap();
        
        assert_eq!(response.status(), 200);
        
        mock.assert();
    }
}
```

---

## 6. JavaScript Runtime Options

### 6.1 Runtime Comparison

| Runtime | Size | ES2020 | Async | WASM | Performance | Maturity |
|---------|------|--------|-------|------|-------------|----------|
| **rquickjs** | ~200KB | Full | Limited | Yes | Good | Stable |
| **deno_core** | ~5MB+ | Full | Full | Yes | Excellent | Stable |
| **boa** | ~1MB | Partial | Limited | Yes | Moderate | Developing |
| **rhino** | ~2MB | Partial | Limited | Yes | Moderate | Mature |

### 6.2 rquickjs Integration

```rust
use rquickjs::{
    Context, Runtime, Module, Function, args::Args,
    atom::PredefinedAtom, CatchResultExt,
};

/// JavaScript runtime wrapper
pub struct JsRuntime {
    runtime: Runtime,
    context: Context,
}

impl JsRuntime {
    /// Create new runtime
    pub fn new() -> Result<Self, rquickjs::Error> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        
        Ok(Self { runtime, context })
    }
    
    /// Execute JavaScript code
    pub fn eval<T>(&self, code: &str) -> Result<T, rquickjs::Error>
    where
        T: rquickjs::FromJs<'static>,
    {
        self.context.with(|ctx| {
            ctx.eval(code)
        })
    }
    
    /// Execute as ES module
    pub fn eval_module<T>(&self, code: &str, name: &str) -> Result<T, rquickjs::Error>
    where
        T: rquickjs::FromJs<'static>,
    {
        self.context.with(|ctx| {
            let module = Module::declare(ctx.clone(), name, code)?;
            module.eval()?;
            
            let namespace = module.namespace();
            ctx.get_predefined(PredefinedAtom::Default)
                .unwrap()
                .get::<T>()
        })
    }
    
    /// Expose Rust function to JavaScript
    pub fn expose<F, A, R>(&self, name: &str, func: F) -> Result<(), rquickjs::Error>
    where
        F: Fn(A) -> R + 'static,
        A: Args<'static>,
        R: rquickjs::IntoJs<'static>,
    {
        self.context.with(|ctx| {
            let js_func = Function::new(ctx.clone(), func)?;
            ctx.globals().set(name, js_func)?;
            Ok(())
        })
    }
    
    /// Set global value
    pub fn set_global<T>(&self, name: &str, value: T) -> Result<(), rquickjs::Error>
    where
        T: rquickjs::IntoJs<'static>,
    {
        self.context.with(|ctx| {
            ctx.globals().set(name, value)?;
            Ok(())
        })
    }
}

/// Example: Execute code with mocked console
pub fn create_sandboxed_runtime() -> Result<JsRuntime, rquickjs::Error> {
    let runtime = JsRuntime::new()?;
    
    // Mock console.log
    runtime.expose("log", |args: rquickjs::Rest<rquickjs::Value>| {
        for arg in args.iter() {
            println!("[JS] {:?}", arg);
        }
    })?;
    
    // Mock fetch
    runtime.expose("fetch", |_url: String| {
        // Return mock response
        serde_json::json!({
            "ok": true,
            "status": 200,
            "json": async { serde_json::json!({ "data": "mocked" }) }
        })
    })?;
    
    Ok(runtime)
}
```

### 6.3 deno_core Integration

```rust
use deno_core::{
    JsRuntime, RuntimeOptions, ModuleSpecifier,
    Extension, op2, ModuleLoader, ModuleSource,
    ModuleSourceCode, ModuleType,
};
use std::rc::Rc;

/// Custom module loader
struct InMemoryModuleLoader {
    modules: HashMap<String, String>,
}

impl ModuleLoader for InMemoryModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: deno_core::ResolutionKind,
    ) -> Result<ModuleSpecifier, deno_core::error::Error> {
        // Simple resolution - in production would handle relative paths
        Ok(ModuleSpecifier::parse(specifier)?)
    }
    
    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: deno_core::RequestedModuleType,
    ) -> deno_core::ModuleLoadResponse {
        let code = self.modules.get(module_specifier.path())
            .cloned()
            .unwrap_or_default();
        
        deno_core::ModuleLoadResponse::Sync(Ok(ModuleSource::new(
            ModuleType::JavaScript,
            ModuleSourceCode::String(code.into()),
            module_specifier,
        )))
    }
}

/// Define custom ops for container integration
#[op2]
#[string]
fn op_read_file(#[string] path: String) -> Result<String, String> {
    // Would integrate with FileSystem trait
    Ok(format!("Content of {}", path))
}

#[op2]
fn op_write_file(#[string] path: String, #[string] content: String) -> Result<(), String> {
    // Would integrate with FileSystem trait
    Ok(())
}

#[op2]
#[serde]
fn op_spawn_process(
    #[string] executable: String,
    #[serde] args: Vec<String>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "pid": 1,
        "status": "started"
    }))
}

/// Create deno-based runtime
pub fn create_deno_runtime() -> JsRuntime {
    let extension = Extension {
        name: "opencontainer",
        ops: std::borrow::Cow::Borrowed(&[
            op_read_file::DECL,
            op_write_file::DECL,
            op_spawn_process::DECL,
        ]),
        ..Default::default()
    };
    
    let loader = Rc::new(InMemoryModuleLoader {
        modules: HashMap::new(),
    });
    
    JsRuntime::new(RuntimeOptions {
        extensions: vec![extension],
        module_loader: Some(loader),
        ..Default::default()
    })
}
```

### 6.4 boa Integration

```rust
use boa_engine::{
    Context, Source, JsValue, JsResult,
    builtins::global::GlobalObject,
};

/// boa-based runtime
pub struct BoaRuntime {
    context: Context,
}

impl BoaRuntime {
    pub fn new() -> Result<Self, boa_engine::context::context_builder::ContextBuilderError> {
        let context = Context::builder().build()?;
        Ok(Self { context })
    }
    
    pub fn eval(&mut self, code: &str) -> JsResult<JsValue> {
        let source = Source::from_bytes(code.as_bytes());
        self.context.eval(source)
    }
}
```

---

## 7. WASM Compilation

### 7.1 Project Structure for WASM

```
opencontainer-wasm/
├── Cargo.toml
├── src/
│   ├── lib.rs          # wasm-bindgen entry points
│   ├── container.rs    # Container API for browser
│   └── bridge.rs       # JS-Rust communication
├── www/
│   ├── index.html
│   ├── package.json
│   └── index.js        # JS bootstrap
└── build.sh
```

### 7.2 wasm-bindgen Setup

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use js_sys::Promise;
use web_sys::{Worker, MessageEvent};

/// Container handle for JavaScript
#[wasm_bindgen]
pub struct WasmContainer {
    inner: Arc<crate::container::Container>,
}

#[wasm_bindgen]
impl WasmContainer {
    /// Create new container
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue) -> Result<WasmContainer, JsValue> {
        let config: crate::config::ContainerConfig = serde_wasm_bindgen::from_value(config)?;
        let container = crate::container::Container::new(config);
        
        Ok(WasmContainer {
            inner: Arc::new(container),
        })
    }
    
    /// Spawn a process
    #[wasm_bindgen]
    pub fn spawn(&self, cmd: String, args: JsValue) -> Result<Promise, JsValue> {
        let args: Vec<String> = serde_wasm_bindgen::from_value(args)?;
        let container = Arc::clone(&self.inner);
        
        // Convert to Promise for JS interop
        let promise = Promise::new(&mut |resolve, reject| {
            spawn_local(async move {
                match container.spawn(cmd, args).await {
                    Ok(process) => {
                        let pid = process.pid().value();
                        resolve(&JsValue::from(pid));
                    }
                    Err(e) => {
                        reject(&JsValue::from(e.to_string()));
                    }
                }
            });
        });
        
        Ok(promise)
    }
    
    /// Write to filesystem
    #[wasm_bindgen(js_name = writeFile)]
    pub fn write_file(&self, path: String, content: String) -> Result<(), JsValue> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            self.inner.fs().write(&path, &content).await
        }).map_err(|e| JsValue::from(e.to_string()))
    }
    
    /// Read from filesystem
    #[wasm_bindgen(js_name = readFile)]
    pub fn read_file(&self, path: String) -> Result<String, JsValue> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            self.inner.fs().read_to_string(&path).await
        }).map_err(|e| JsValue::from(e.to_string()))
    }
}

/// Initialize WASM module
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    
    // Set up worker communication if in Web Worker
    if let Some(worker) = web_sys::window()
        .and_then(|w| w.worker())
    {
        setup_worker_communication(worker)?;
    }
    
    Ok(())
}

fn setup_worker_communication(worker: Worker) -> Result<(), JsValue> {
    let callback = Closure::wrap(Box::new(move |event: MessageEvent| {
        // Handle messages from main thread
        let data = event.data();
        // Process message...
    }) as Box<dyn FnMut(MessageEvent)>);
    
    worker.set_onmessage(Some(callback.as_ref().unchecked_ref()));
    callback.forget();
    
    Ok(())
}
```

### 7.3 Cargo.toml for WASM

```toml
[package]
name = "opencontainer-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
serde-wasm-bindgen = "0.6"
js-sys = "0.3"
web-sys = { version = "0.3", features = [
    "Worker",
    "MessageEvent",
    "Window",
    "console",
] }
console_error_panic_hook = "0.1"
tokio = { version = "1", features = ["rt", "sync", "macros"] }
async-trait = "0.1"
thiserror = "1"
serde = { version = "1", features = ["derive"] }

# OpenContainer core (path to main crate)
opencontainer-core = { path = "../opencontainer-core" }

[dev-dependencies]
wasm-bindgen-test = "0.3"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

### 7.4 Build Configuration

```bash
#!/bin/bash
# build.sh - Build WASM package

# Install wasm-pack if not installed
cargo install wasm-pack

# Build for web
wasm-pack build --target web --release

# Build for Node.js
wasm-pack build --target nodejs --release

# Copy to www directory
cp -r pkg/* www/

# Build JS bundle
cd www
npm install
npm run build
```

### 7.5 Size Optimization

```toml
# Cargo.toml optimizations
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Single compilation unit
strip = true         # Strip symbols
panic = "abort"      # Smaller panic handling

# wasm-opt for additional optimization
[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-Oz", "--enable-mutable-globals"]
```

---

## 8. Tokio Integration

### 8.1 Runtime Setup

```rust
use tokio::runtime::{Builder, Runtime};
use std::sync::Arc;

/// Runtime configuration
#[derive(Clone)]
pub struct RuntimeConfig {
    pub worker_threads: usize,
    pub max_blocking_threads: usize,
    pub thread_stack_size: usize,
    pub thread_name: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            max_blocking_threads: 512,
            thread_stack_size: 2 * 1024 * 1024, // 2MB
            thread_name: "opencontainer-worker".to_string(),
        }
    }
}

/// Create tokio runtime
pub fn create_runtime(config: &RuntimeConfig) -> std::io::Result<Runtime> {
    Builder::new_multi_thread()
        .worker_threads(config.worker_threads)
        .max_blocking_threads(config.max_blocking_threads)
        .thread_stack_size(config.thread_stack_size)
        .thread_name_fn(move || {
            static ATOMIC_ID: std::sync::atomic::AtomicUsize = 
                std::sync::atomic::AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            format!("{}-{}", config.thread_name, id)
        })
        .enable_all()
        .build()
}

/// Async main entry point
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Application entry point
    Ok(())
}
```

### 8.2 Task Spawning

```rust
use tokio::task::JoinHandle;
use tokio::sync::mpsc;

/// Task manager for container operations
pub struct TaskManager {
    handles: RwLock<Vec<JoinHandle<Result<(), TaskError>>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            handles: RwLock::new(Vec::new()),
        }
    }
    
    /// Spawn a tracked task
    pub fn spawn<F, T>(&self, future: F) -> JoinHandle<T>
    where
        F: futures::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::spawn(future)
    }
    
    /// Spawn a task with graceful shutdown
    pub fn spawn_with_shutdown<F, T>(
        &self,
        future: F,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> JoinHandle<Option<T>>
    where
        F: futures::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::spawn(async move {
            tokio::select! {
                result = future => Some(result),
                _ = shutdown.changed() => None,
            }
        })
    }
    
    /// Abort all tracked tasks
    pub async fn abort_all(&self) {
        let handles = self.handles.write().await;
        for handle in handles.iter() {
            handle.abort();
        }
    }
    
    /// Wait for all tasks to complete
    pub async fn join_all(&self) -> Vec<Result<Result<(), TaskError>, tokio::task::JoinError>> {
        let handles = {
            let mut h = self.handles.write().await;
            std::mem::take(&mut *h)
        };
        
        futures::future::join_all(handles).await
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}
```

### 8.3 Channels for IPC

```rust
use tokio::sync::{mpsc, broadcast, oneshot};

/// Message types for inter-process communication
#[derive(Debug, Clone)]
pub enum ContainerMessage {
    /// Spawn a new process
    SpawnProcess {
        executable: String,
        args: Vec<String>,
        cwd: String,
        response_tx: oneshot::Sender<Result<Pid, ProcessError>>,
    },
    /// Send input to process
    WriteStdin {
        pid: Pid,
        data: Vec<u8>,
    },
    /// Process output
    ProcessOutput {
        pid: Pid,
        stream: OutputType,
        data: Vec<u8>,
    },
    /// Process exited
    ProcessExited {
        pid: Pid,
        exit_code: i32,
    },
    /// Filesystem operation
    FsWrite {
        path: String,
        content: Vec<u8>,
        response_tx: oneshot::Sender<Result<(), FileSystemError>>,
    },
}

/// Create channel pair for container communication
pub fn create_container_channels(
    buffer_size: usize,
) -> (
    mpsc::Sender<ContainerMessage>,
    mpsc::Receiver<ContainerMessage>,
    broadcast::Sender<ContainerMessage>,
    broadcast::Receiver<ContainerMessage>,
) {
    let (tx, rx) = mpsc::channel(buffer_size);
    let (event_tx, event_rx) = broadcast::channel(buffer_size);
    (tx, rx, event_tx, event_rx)
}
```

### 8.4 Timers and Timeouts

```rust
use tokio::time::{timeout, Duration, Instant, Interval, interval};

/// Execute operation with timeout
pub async fn with_timeout<F, T>(
    future: F,
    duration: Duration,
) -> Result<T, tokio::time::error::Elapsed>
where
    F: futures::Future<Output = T>,
{
    timeout(duration, future).await
}

/// Create interval timer
pub fn create_timer(interval_ms: u64) -> Interval {
    interval(Duration::from_millis(interval_ms))
}

/// Example: Process with execution timeout
pub async fn spawn_with_timeout(
    manager: &ProcessManager,
    executable: String,
    args: Vec<String>,
    timeout_duration: Duration,
) -> Result<Arc<dyn Process>, ProcessError> {
    with_timeout(
        manager.spawn(executable, args, "/".to_string(), HashMap::new(), None),
        timeout_duration,
    )
    .await
    .map_err(|_| ProcessError::Execution("Process spawn timed out".into()))?
}

/// Example: Heartbeat monitor
pub struct HeartbeatMonitor {
    interval: Interval,
    last_seen: Instant,
    timeout: Duration,
}

impl HeartbeatMonitor {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            interval: interval(Duration::from_millis(timeout_ms / 2)),
            last_seen: Instant::now(),
            timeout: Duration::from_millis(timeout_ms),
        }
    }
    
    pub async fn wait_for_heartbeat(&mut self) -> bool {
        self.interval.tick().await;
        
        if self.last_seen.elapsed() > self.timeout {
            return false;
        }
        
        true
    }
    
    pub fn record_heartbeat(&mut self) {
        self.last_seen = Instant::now();
    }
}
```

---

## 9. Complete Working Example

```rust
// main.rs - Complete OpenContainer Rust Implementation
use std::sync::Arc;
use tokio::sync::broadcast;

mod filesystem;
mod process;
mod shell;
mod container;
mod network;

use filesystem::{FileSystem, InMemoryFileSystem};
use process::{ProcessManager, ProcessExecutor};
use container::Container;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OpenContainer v0.1.0");
    
    // Create filesystem
    let fs: Arc<dyn FileSystem> = Arc::new(InMemoryFileSystem::new());
    
    // Create process manager
    let process_manager = Arc::new(ProcessManager::new());
    
    // Register executors
    process_manager.register_executor(Arc::new(
        process::ShellExecutor::new(Arc::clone(&fs))
    )).await;
    
    // Create HTTP server
    let state = container::AppState {
        file_system: Arc::clone(&fs),
        process_manager: Arc::clone(&process_manager),
    };
    
    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    
    axum::serve(listener, container::create_router(state)).await?;
    
    Ok(())
}
```

```rust
// container.rs
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::filesystem::{FileSystem, InMemoryFileSystem};
use crate::process::{ProcessManager, ProcessHandle};

/// Container configuration
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub name: String,
    pub cwd: String,
    pub env: std::collections::HashMap<String, String>,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            name: "container".to_string(),
            cwd: "/".to_string(),
            env: std::collections::HashMap::new(),
        }
    }
}

/// Container instance
pub struct Container {
    config: ContainerConfig,
    file_system: Arc<dyn FileSystem>,
    process_manager: Arc<ProcessManager>,
}

impl Container {
    /// Create new container
    pub fn new(config: ContainerConfig) -> Self {
        let fs: Arc<dyn FileSystem> = Arc::new(InMemoryFileSystem::with_root(&config.cwd));
        let pm = Arc::new(ProcessManager::new());
        
        Self {
            config,
            file_system: fs,
            process_manager: pm,
        }
    }
    
    /// Get filesystem reference
    pub fn fs(&self) -> Arc<dyn FileSystem> {
        Arc::clone(&self.file_system)
    }
    
    /// Get process manager reference
    pub fn pm(&self) -> Arc<ProcessManager> {
        Arc::clone(&self.process_manager)
    }
    
    /// Spawn a process
    pub async fn spawn(
        &self,
        executable: String,
        args: Vec<String>,
    ) -> Result<Arc<dyn crate::process::Process>, crate::process::ProcessError> {
        self.process_manager.spawn(
            executable,
            args,
            self.config.cwd.clone(),
            self.config.env.clone(),
            None,
        ).await
    }
}

/// Application state for HTTP server
#[derive(Clone)]
pub struct AppState {
    pub file_system: Arc<dyn FileSystem>,
    pub process_manager: Arc<ProcessManager>,
}
```

---

## 10. Cargo.toml Configuration

```toml
[workspace]
members = [
    "opencontainer-core",
    "opencontainer-wasm",
    "opencontainer-cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
authors = ["OpenContainer Contributors"]

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"
async-trait = "0.1"
futures = "0.3"

# Web framework
axum = { version = "0.7", features = ["ws"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
thiserror = "1"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Filesystem
tempfile = "3"

# Parsing
nom = "7"
regex = "1"

# WASM
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = "0.3"
console_error_panic_hook = "0.1"

# Testing
mockito = "1"
tokio-test = "0.4"

[package]
name = "opencontainer-core"
version.workspace = true
edition.workspace = true

[dependencies]
tokio.workspace = true
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
axum.workspace = true
nom.workspace = true
regex.workspace = true
tempfile.workspace = true
futures.workspace = true

# Optional: JavaScript runtime
rquickjs = { version = "0.4", optional = true, features = ["parallel"] }
deno_core = { version = "0.270", optional = true }
boa_engine = { version = "0.17", optional = true }

# Optional: Persistent storage
sled = { version = "0.34", optional = true }
rocksdb = { version = "0.21", optional = true }

[features]
default = ["rquickjs"]
javascript = ["rquickjs"]
deno = ["deno_core"]
boa = ["boa_engine"]
persistent = ["sled"]
```

---

## Summary

This document provides a complete blueprint for implementing OpenContainer in Rust:

| TypeScript Concept | Rust Translation |
|-------------------|------------------|
| Web Workers | Tokio tasks/threads |
| postMessage | Tokio channels (mpsc, broadcast) |
| Interfaces | Traits |
| Classes | Structs with Arc<RwLock<>> |
| Promise<T> | Future<Output = T> |
| EventEmitter | broadcast::Sender |
| Map/Set | HashMap/HashSet |
| async/await | async/await with Tokio |

Key crates:
- **tokio** - Async runtime
- **axum** - HTTP server
- **rquickjs/deno_core** - JavaScript runtime
- **nom** - Command parsing
- **rustyline** - Interactive shell
- **sled** - Persistent filesystem
- **wasm-bindgen** - Browser integration
