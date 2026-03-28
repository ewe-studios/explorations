---
title: "Project System Deep Dive: Workspaces, File Watching, and Indexing"
subtitle: "Building robust project management with lazy loading, dirty tracking, and file indexing"
based_on: "rockies MultiGrid spatial indexing and grid lifecycle management"
level: "Intermediate - Requires understanding of IDE fundamentals"
---

# Project System Deep Dive

## Table of Contents

1. [Project System Architecture](#1-project-system-architecture)
2. [Workspace Management](#2-workspace-management)
3. [File System Watching](#3-file-system-watching)
4. [Indexing Strategies](#4-indexing-strategies)
5. [Lazy Loading Patterns](#5-lazy-loading-patterns)
6. [Dirty Tracking](#6-dirty-tracking)
7. [Rockies MultiGrid as File Index](#7-rockies-multigrid-as-file-index)
8. [Virtual File System](#8-virtual-file-system)

---

## 1. Project System Architecture

### 1.1 What is a Project System?

A project system manages:
- **Files and directories** in a workspace
- **Indexing** for fast symbol lookup
- **File watching** for external changes
- **Document state** (open, dirty, saved)
- **Build configuration** and dependencies

```
┌─────────────────────────────────────────────────────┐
│              Project/Workspace                      │
├─────────────────────────────────────────────────────┤
│  File System Access Layer                           │
│  - Native file system                               │
│  - Virtual file system (archives, remote)           │
├─────────────────────────────────────────────────────┤
│  File Watcher                                       │
│  - Detect external changes                          │
│  - Trigger re-indexing                              │
├─────────────────────────────────────────────────────┤
│  Document Manager                                   │
│  - Open/close documents                             │
│  - Track dirty state                                │
│  - Save/load operations                             │
├─────────────────────────────────────────────────────┤
│  Symbol Index                                       │
│  - Fast symbol lookup                               │
│  - Reference tracking                               │
│  - Dependency graph                                 │
└─────────────────────────────────────────────────────┘
```

### 1.2 Rockies Parallels

| Rockies | Project System |
|---------|---------------|
| `MultiGrid<T>` | File/symbol index |
| `GridIndex` | File/module identifier |
| `UniverseGrid` | Module index |
| `Cell` | File content / Symbol |
| `get_missing_grids()` | Lazy file loading |
| `save_grid()` / `load_grid()` | Document save/load |
| Grid dirty states | Document dirty states |
| `tick()` loop | Background indexing |

---

## 2. Workspace Management

### 2.1 Workspace Structure

```rust
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// A workspace represents a root folder with projects
pub struct Workspace {
    /// Root path of the workspace
    pub root: PathBuf,

    /// Name of the workspace
    pub name: String,

    /// Projects within the workspace
    pub projects: HashMap<ProjectId, Project>,

    /// Configuration
    pub config: WorkspaceConfig,
}

#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    /// Files/folders to exclude from indexing
    pub exclude_patterns: Vec<String>,

    /// Files/folders to include
    pub include_patterns: Vec<String>,

    /// Maximum file size for indexing
    pub max_file_size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectId(u64);

/// A project within a workspace
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub path: PathBuf,
    pub config: ProjectConfig,
}

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// Project type (Rust, TypeScript, etc.)
    pub project_type: String,

    /// Source directories
    pub source_dirs: Vec<PathBuf>,

    /// Build output directories
    pub output_dirs: Vec<PathBuf>,
}
```

### 2.2 Workspace Operations

```rust
impl Workspace {
    /// Create a new workspace from a root path
    pub fn new(root: PathBuf) -> std::io::Result<Self> {
        let name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace")
            .to_string();

        Ok(Self {
            root,
            name,
            projects: HashMap::new(),
            config: WorkspaceConfig {
                exclude_patterns: vec![
                    "**/node_modules/**".to_string(),
                    "**/target/**".to_string(),
                    "**/.git/**".to_string(),
                    "**/*.min.js".to_string(),
                ],
                include_patterns: vec!["**/*.rs".to_string(), "**/*.ts".to_string()],
                max_file_size: 10 * 1024 * 1024, // 10MB
            },
        })
    }

    /// Discover projects in the workspace
    pub fn discover_projects(&mut self) -> std::io::Result<()> {
        // Look for project files (Cargo.toml, package.json, etc.)
        for entry in walkdir::WalkDir::new(&self.root)
            .min_depth(1)
            .max_depth(3)
            .into_iter()
            .filter_entry(|e| !self.should_exclude(e.path()))
        {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                match file_name {
                    "Cargo.toml" => self.add_rust_project(path)?,
                    "package.json" => self.add_typescript_project(path)?,
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.config.exclude_patterns.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches(&path_str))
                .unwrap_or(false)
        })
    }

    fn add_rust_project(&mut self, cargo_toml: &Path) -> std::io::Result<()> {
        use std::fs;

        let content = fs::read_to_string(cargo_toml)?;
        let manifest: toml::Value = toml::from_str(&content)?;

        let name = manifest
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();

        let project_path = cargo_toml.parent().unwrap().to_path_buf();

        let project = Project {
            id: ProjectId(hash_path(&project_path)),
            name,
            path: project_path.clone(),
            config: ProjectConfig {
                project_type: "rust".to_string(),
                source_dirs: vec![project_path.join("src")],
                output_dirs: vec![project_path.join("target")],
            },
        };

        self.projects.insert(project.id, project);
        Ok(())
    }

    fn add_typescript_project(&mut self, package_json: &Path) -> std::io::Result<()> {
        // Similar to add_rust_project but for TypeScript
        Ok(())
    }
}

fn hash_path(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}
```

---

## 3. File System Watching

### 3.1 Watcher Architecture

```
┌─────────────────────────────────────────┐
│          File System Watcher            │
├─────────────────────────────────────────┤
│  Platform-Specific Backends             │
│  - inotify (Linux)                      │
│  - FSEvents (macOS)                     │
│  - ReadDirectoryChangesW (Windows)      │
├─────────────────────────────────────────┤
│  Event Normalization                    │
│  - Coalesce rapid events                │
│  - Handle synthetic events              │
├─────────────────────────────────────────┤
│  Event Dispatch                         │
│  - Notify interested components         │
│  - Trigger re-indexing                  │
└─────────────────────────────────────────┘
```

### 3.2 File Watcher Implementation

```rust
use notify::{
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

pub type WatchId = u64;

/// File system watcher for detecting changes
pub struct FileSystemWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
    watches: HashMap<WatchId, WatchInfo>,
    next_watch_id: WatchId,
}

struct WatchInfo {
    path: PathBuf,
    recursive: bool,
}

impl FileSystemWatcher {
    /// Create a new file system watcher
    pub fn new<F>(mut handler: F) -> notify::Result<Self>
    where
        F: FnMut(WatchEvent) + Send + 'static,
    {
        let (tx, rx) = channel();

        let watcher = RecommendedWatcher::new(move |result| {
            tx.send(result).unwrap();
        })?;

        // Spawn event processing thread
        std::thread::spawn(move || {
            while let Ok(result) = rx.recv() {
                match result {
                    Ok(event) => {
                        for path in event.paths {
                            let watch_event = match event.kind {
                                EventKind::Create(_) => WatchEvent::Created(path),
                                EventKind::Modify(_) => WatchEvent::Modified(path),
                                EventKind::Remove(_) => WatchEvent::Deleted(path),
                                _ => continue,
                            };
                            handler(watch_event);
                        }
                    }
                    Err(e) => eprintln!("Watch error: {:?}", e),
                }
            }
        });

        Ok(Self {
            watcher,
            receiver: rx,
            watches: HashMap::new(),
            next_watch_id: 1,
        })
    }

    /// Watch a path for changes
    pub fn watch(&mut self, path: &Path, recursive: bool) -> notify::Result<WatchId> {
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        self.watcher.watch(path, mode)?;

        let watch_id = self.next_watch_id;
        self.next_watch_id += 1;

        self.watches.insert(
            watch_id,
            WatchInfo {
                path: path.to_path_buf(),
                recursive,
            },
        );

        Ok(watch_id)
    }

    /// Stop watching a path
    pub fn unwatch(&mut self, watch_id: WatchId) -> notify::Result<()> {
        if let Some(info) = self.watches.remove(&watch_id) {
            self.watcher.unwatch(&info.path)?;
        }
        Ok(())
    }
}

/// Events from the file system watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}
```

### 3.3 Handling File Changes

```rust
pub struct ProjectIndexer {
    workspace: Arc<Workspace>,
    index: Arc<RwLock<SymbolIndex>>,
    watcher: FileSystemWatcher,
}

impl ProjectIndexer {
    pub fn new(workspace: Arc<Workspace>, index: Arc<RwLock<SymbolIndex>>) -> Self {
        let index_clone = index.clone();

        let watcher = FileSystemWatcher::new(move |event| {
            let mut idx = index_clone.write().unwrap();
            match event {
                WatchEvent::Created(path) => {
                    idx.add_file(&path);
                }
                WatchEvent::Modified(path) => {
                    idx.update_file(&path);
                }
                WatchEvent::Deleted(path) => {
                    idx.remove_file(&path);
                }
            }
        })
        .unwrap();

        Self {
            workspace,
            index,
            watcher,
        }
    }

    /// Start watching workspace for changes
    pub fn start_watching(&mut self) -> notify::Result<()> {
        self.watcher.watch(&self.workspace.root, true)
    }
}
```

---

## 4. Indexing Strategies

### 4.1 Full Index vs Incremental Index

**Full Index (initial build):**
```rust
impl SymbolIndex {
    /// Build index from scratch
    pub fn build_full(workspace: &Workspace) -> Self {
        let mut index = Self::new();

        for project in workspace.projects.values() {
            for source_dir in &project.config.source_dirs {
                for entry in walkdir::WalkDir::new(source_dir)
                    .into_iter()
                    .filter_entry(|e| is_source_file(e.path()))
                {
                    if let Ok(entry) = entry {
                        if entry.file_type().is_file() {
                            let content = std::fs::read_to_string(entry.path()).unwrap();
                            index.add_file(entry.path(), &content);
                        }
                    }
                }
            }
        }

        index
    }
}
```

**Incremental Index (on change):**
```rust
impl SymbolIndex {
    /// Update index for a single file change
    pub fn update_file(&mut self, path: &Path, new_content: &str) {
        // Remove old symbols from this file
        self.remove_symbols_for_file(path);

        // Parse and add new symbols
        self.add_file(path, new_content);

        // Update references that may have changed
        self.update_references();
    }

    /// Efficiently apply text changes without full re-parse
    pub fn apply_text_change(&mut self, path: &Path, change: &TextChange) {
        if let Some(file_index) = self.files.get_mut(path) {
            // Apply change to cached AST
            let ast = &mut file_index.ast;
            ast.apply_change(change);

            // Update affected symbols only
            self.update_affected_symbols(file_index, change.range);
        }
    }
}
```

### 4.2 Index Data Structures

```rust
use std::collections::{HashMap, BTreeMap};

/// Main symbol index
pub struct SymbolIndex {
    /// Files by path
    files: HashMap<PathBuf, FileIndex>,

    /// Symbols by name (for quick lookup)
    symbols_by_name: BTreeMap<String, Vec<SymbolId>>,

    /// Symbols by ID
    symbols: HashMap<SymbolId, Symbol>,

    /// References: symbol -> locations where it's referenced
    references: HashMap<SymbolId, Vec<Reference>>,

    /// Next symbol ID
    next_symbol_id: u64,
}

#[derive(Debug, Clone)]
pub struct FileIndex {
    pub path: PathBuf,
    pub content_hash: u64,
    pub symbols: Vec<SymbolId>,
    pub ast: SyntaxTree,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub container: Option<SymbolId>,
    pub metadata: SymbolMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u64);

#[derive(Debug, Clone)]
pub struct Reference {
    pub location: Location,
    pub is_definition: bool,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
}
```

---

## 5. Lazy Loading Patterns

### 5.1 Rockies Lazy Grid Loading

```rust
// Rockies: Only load grids near the player
impl Universe {
    pub fn get_missing_grids(&self) -> Vec<GridIndex> {
        self.cells.get_missing_grids(self.player.inertia.pos)
    }

    pub fn load_grid(&mut self, grid_index: &GridIndex, bytes: JsValue) {
        self.universe.load_from_storage(*grid_index, bytes);
    }
}

// MultiGrid: Find grids that need loading
impl MultiGrid<T> {
    pub fn get_dropped_grids(&self, center: V2i, drop_radius: usize) -> Vec<GridIndex> {
        let r = drop_radius as i32;
        let center_grid = GridIndex::from_pos(center, self.grid_width, self.grid_height);

        let mut res = Vec::new();
        for x in -r..r {
            for y in -r..r {
                let grid_index = GridIndex {
                    grid_offset: V2i::new(
                        center_grid.grid_offset.x + x,
                        center_grid.grid_offset.y + y,
                    ),
                };
                if !self.grids.contains_key(&grid_index) {
                    res.push(grid_index);
                }
            }
        }
        res
    }
}
```

### 5.2 IDE Lazy File Loading

```rust
/// Lazy file loader - only loads files when needed
pub struct LazyFileLoader {
    /// Cache of loaded files
    loaded_files: HashMap<PathBuf, LoadedFile>,

    /// Maximum files to keep loaded
    max_loaded: usize,

    /// LRU order for eviction
    access_order: Vec<PathBuf>,
}

struct LoadedFile {
    content: String,
    content_hash: u64,
    last_accessed: Instant,
}

impl LazyFileLoader {
    pub fn new(max_loaded: usize) -> Self {
        Self {
            loaded_files: HashMap::new(),
            max_loaded,
            access_order: Vec::new(),
        }
    }

    /// Get file content, loading if necessary
    pub fn get_file(&mut self, path: &Path) -> std::io::Result<&LoadedFile> {
        // Check if already loaded
        if !self.loaded_files.contains_key(path) {
            // Evict if necessary
            while self.loaded_files.len() >= self.max_loaded {
                self.evict_oldest();
            }

            // Load the file
            let content = std::fs::read_to_string(path)?;
            let loaded = LoadedFile {
                content,
                content_hash: hash_content(&std::fs::read_to_string(path)?),
                last_accessed: Instant::now(),
            };

            self.loaded_files.insert(path.to_path_buf(), loaded);
        }

        // Update access time
        if let Some(file) = self.loaded_files.get_mut(path) {
            file.last_accessed = Instant::now();
        }

        // Update access order
        self.access_order.retain(|p| p != path);
        self.access_order.push(path.to_path_buf());

        Ok(self.loaded_files.get(path).unwrap())
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_path) = self.access_order.first().cloned() {
            self.loaded_files.remove(&oldest_path);
            self.access_order.remove(0);
        }
    }
}
```

### 5.3 Lazy Symbol Index Loading

```rust
/// Lazy symbol index - only index modules when accessed
pub struct LazySymbolIndex {
    /// Module index (which files belong to which module)
    module_map: HashMap<ModuleId, Vec<PathBuf>>,

    /// Loaded module indexes
    loaded_modules: HashMap<ModuleId, ModuleIndex>,

    /// Module loading threshold
    load_radius: usize,
}

impl LazySymbolIndex {
    /// Get symbols in region, loading modules as needed
    pub fn get_symbols_in_region(
        &mut self,
        active_file: &Path,
        radius: usize,
    ) -> Vec<&Symbol> {
        // Find modules near the active file
        let active_module = self.get_module_for_file(active_file);
        let nearby_modules = self.get_nearby_modules(active_module, radius);

        // Load missing modules
        for module_id in nearby_modules {
            if !self.loaded_modules.contains_key(&module_id) {
                self.load_module(module_id);
            }
        }

        // Collect symbols from loaded modules
        let mut symbols = Vec::new();
        for module_id in nearby_modules {
            if let Some(module) = self.loaded_modules.get(&module_id) {
                symbols.extend(module.symbols.iter());
            }
        }

        symbols
    }

    fn load_module(&mut self, module_id: ModuleId) {
        // Get files for this module
        let files = self.module_map.get(&module_id).cloned().unwrap_or_default();

        let mut module = ModuleIndex::new();
        for file_path in files {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                module.add_file(&file_path, &content);
            }
        }

        self.loaded_modules.insert(module_id, module);
    }
}
```

---

## 6. Dirty Tracking

### 6.1 Rockies Grid Dirty States

From the TLA+ specification:

```
Grid States:
- stored/not_stored  (persisted to disk?)
- loaded/not_loaded  (currently in memory?)
- dirty/unmodified/pristine (modification state)

Valid Transitions:
not_stored + not_loaded  ──LoadMissingGrid──> loaded + pristine
stored + not_loaded      ──LoadStoredGrid───> loaded + unmodified
loaded + pristine        ──MarkDirty────────> loaded + dirty
loaded + unmodified      ──MarkDirty────────> loaded + dirty
loaded + dirty           ──StoreGrid────────> stored + unmodified
loaded + unmodified      ──UnloadGrid───────> not_loaded
loaded + pristine        ──UnloadGrid───────> not_loaded
```

### 6.2 Document Dirty State Machine

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentState {
    /// File has never been saved (new file)
    NotStored,
    /// File is saved to disk
    Stored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    /// Not currently in memory
    NotLoaded,
    /// Currently in memory
    Loaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirtyState {
    /// New file, no changes yet
    Pristine,
    /// Loaded from disk, no changes since
    Unmodified,
    /// Modified since last save
    Dirty,
}

/// Document with full state tracking
pub struct Document {
    pub path: PathBuf,
    pub content: String,

    pub document_state: DocumentState,
    pub load_state: LoadState,
    pub dirty_state: DirtyState,

    pub version: i32,
    pub saved_version: i32,
}

impl Document {
    /// Create a new unsaved document
    pub fn new(path: PathBuf, content: String) -> Self {
        Self {
            path,
            content,
            document_state: DocumentState::NotStored,
            load_state: LoadState::Loaded,
            dirty_state: DirtyState::Pristine,
            version: 1,
            saved_version: 0,
        }
    }

    /// Load a document from disk
    pub fn load(path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;

        Ok(Self {
            path: path.clone(),
            content,
            document_state: DocumentState::Stored,
            load_state: LoadState::Loaded,
            dirty_state: DirtyState::Unmodified,
            version: 1,
            saved_version: 1,
        })
    }

    /// Modify the document content
    pub fn modify(&mut self, new_content: String) {
        self.content = new_content;
        self.version += 1;
        self.dirty_state = DirtyState::Dirty;
    }

    /// Save the document to disk
    pub fn save(&mut self) -> std::io::Result<()> {
        std::fs::write(&self.path, &self.content)?;

        self.document_state = DocumentState::Stored;
        self.dirty_state = DirtyState::Unmodified;
        self.saved_version = self.version;

        Ok(())
    }

    /// Unload from memory (if clean)
    pub fn unload(&mut self) -> bool {
        match self.dirty_state {
            DirtyState::Dirty => false,  // Can't unload dirty documents
            DirtyState::Pristine | DirtyState::Unmodified => {
                self.load_state = LoadState::NotLoaded;
                self.content.clear();
                true
            }
        }
    }

    /// Check if document needs saving
    pub fn is_dirty(&self) -> bool {
        self.dirty_state == DirtyState::Dirty
    }

    /// Check if document is loaded
    pub fn is_loaded(&self) -> bool {
        self.load_state == LoadState::Loaded
    }
}
```

### 6.3 Document Manager with Dirty Tracking

```rust
pub struct DocumentManager {
    documents: HashMap<PathBuf, Document>,
    dirty_documents: HashSet<PathBuf>,
}

impl DocumentManager {
    /// Get a document, loading if necessary
    pub fn get_document(&mut self, path: &Path) -> std::io::Result<&Document> {
        if !self.documents.contains_key(path) {
            let doc = Document::load(path.to_path_buf())?;
            self.documents.insert(path.to_path_buf(), doc);
        }
        Ok(self.documents.get(path).unwrap())
    }

    /// Get mutable document for editing
    pub fn get_document_mut(&mut self, path: &Path) -> std::io::Result<&mut Document> {
        if !self.documents.contains_key(path) {
            let doc = Document::load(path.to_path_buf())?;
            self.documents.insert(path.to_path_buf(), doc);
        }

        let doc = self.documents.get_mut(path).unwrap();
        if !doc.is_loaded() {
            // Reload content
            doc.content = std::fs::read_to_string(path)?;
            doc.load_state = LoadState::Loaded;
        }

        Ok(doc)
    }

    /// Mark document as dirty after edit
    pub fn mark_dirty(&mut self, path: &Path) {
        if let Some(doc) = self.documents.get_mut(path) {
            doc.dirty_state = DirtyState::Dirty;
            self.dirty_documents.insert(path.to_path_buf());
        }
    }

    /// Save a document
    pub fn save_document(&mut self, path: &Path) -> std::io::Result<bool> {
        if let Some(doc) = self.documents.get_mut(path) {
            if doc.is_dirty() {
                doc.save()?;
                self.dirty_documents.remove(path);
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Save all dirty documents
    pub fn save_all(&mut self) -> std::io::Result<usize> {
        let mut saved = 0;
        let paths: Vec<_> = self.dirty_documents.iter().cloned().collect();

        for path in paths {
            if self.save_document(&path)? {
                saved += 1;
            }
        }

        Ok(saved)
    }

    /// Get all dirty documents
    pub fn get_dirty_documents(&self) -> Vec<&Document> {
        self.dirty_documents
            .iter()
            .filter_map(|p| self.documents.get(p))
            .filter(|d| d.is_dirty())
            .collect()
    }
}
```

---

## 7. Rockies MultiGrid as File Index

### 7.1 Direct Translation

```rust
// Rockies MultiGrid
pub struct MultiGrid<T> {
    grids: HashMap<GridIndex, UniverseGrid<T>>,
    grid_width: usize,
    grid_height: usize,
}

// IDE File Index using same pattern
pub struct FileIndex {
    /// Partition files into "grids" by path hash
    grids: HashMap<FileGridIndex, FileGrid>,
    files_per_grid: usize,
}

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub struct FileGridIndex {
    pub hash_bucket: u64,
}

pub struct FileGrid {
    /// Files in this grid
    files: HashMap<PathBuf, FileEntry>,
    /// Pre-computed cross-file references
    references: HashMap<PathBuf, Vec<Reference>>,
}

impl FileIndex {
    /// Convert file path to grid index (like GridIndex::from_pos)
    fn path_to_grid_index(&self, path: &Path) -> FileGridIndex {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        let hash = hasher.finish();

        FileGridIndex {
            hash_bucket: hash / self.files_per_grid as u64,
        }
    }

    /// Add file to index (like Grid::put)
    pub fn add_file(&mut self, path: &Path, content: &str) {
        let grid_index = self.path_to_grid_index(path);

        let grid = self.grids.entry(grid_index).or_insert_with(|| FileGrid {
            files: HashMap::new(),
            references: HashMap::new(),
        });

        grid.files.insert(path.to_path_buf(), FileEntry {
            path: path.to_path_buf(),
            content_hash: hash_content(content),
            symbols: self.extract_symbols(content, path),
        });
    }

    /// Get file entry (like Grid::get)
    pub fn get_file(&self, path: &Path) -> Option<&FileEntry> {
        let grid_index = self.path_to_grid_index(path);
        self.grids.get(&grid_index)?.files.get(path)
    }

    /// Get "nearby" files for reference resolution (like neighbor tracking)
    pub fn get_nearby_files(&self, path: &Path, radius: usize) -> Vec<&FileEntry> {
        let grid_index = self.path_to_grid_index(path);
        let mut results = Vec::new();

        // Get current grid and neighboring grids
        for bucket_offset in -radius as i64..=radius as i64 {
            let neighbor_index = FileGridIndex {
                hash_bucket: (grid_index.hash_bucket as i64 + bucket_offset) as u64,
            };

            if let Some(grid) = self.grids.get(&neighbor_index) {
                results.extend(grid.files.values());
            }
        }

        results
    }

    /// Get files that need loading (like get_dropped_grids)
    pub fn get_missing_files(&self, active_area: &Path) -> Vec<PathBuf> {
        // In a real implementation, this would check which files
        // in the "area" aren't loaded
        Vec::new()
    }
}
```

---

## 8. Virtual File System

### 8.1 VFS Architecture

```rust
/// Virtual File System - unified file access
pub trait FileSystem: Send + Sync {
    /// Read file contents
    fn read(&self, path: &Path) -> io::Result<String>;

    /// Write file contents
    fn write(&self, path: &Path, content: &str) -> io::Result<()>;

    /// Check if file exists
    fn exists(&self, path: &Path) -> bool;

    /// List directory contents
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;

    /// Get file metadata
    fn metadata(&self, path: &Path) -> io::Result<FileMetadata>;
}

/// Native file system implementation
pub struct NativeFileSystem;

impl FileSystem for NativeFileSystem {
    fn read(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn write(&self, path: &Path, content: &str) -> io::Result<()> {
        std::fs::write(path, content)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        std::fs::read_dir(path)?
            .map(|entry| entry.map(|e| e.path()))
            .collect()
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let meta = std::fs::metadata(path)?;
        Ok(FileMetadata {
            size: meta.len(),
            modified: meta.modified()?.into(),
            is_dir: meta.is_dir(),
        })
    }
}
```

---

*Next: [04-intellisense-deep-dive.md](04-intellisense-deep-dive.md)*
