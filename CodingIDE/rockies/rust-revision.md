---
title: "Rockies: Rust Revision - Complete Translation Guide"
subtitle: "Building IDE systems in Rust for ewe_platform with rockies patterns"
based_on: "rockies MultiGrid, Universe, and Grid implementations"
target: "ewe_platform with valtron executor (no async/await, no tokio)"
level: "Advanced - Requires understanding of Rust and IDE architecture"
---

# Rockies: Rust Revision - Complete Translation Guide

## 1. Overview

### 1.1 What We're Translating

Rockies provides architectural patterns for IDE systems:

| Rockies Component | IDE Equivalent | ewe_platform Translation |
|------------------|----------------|-------------------------|
| `MultiGrid<T>` | Symbol/File index | `SymbolIndex` with TaskIterator |
| `GridIndex` | File/module identifier | `ModuleId` type |
| `UniverseGrid<T>` | Module index | `ModuleIndex` struct |
| `Grid<T>` | File content index | `FileIndex` struct |
| `Cell` | Symbol/Token | `Symbol` enum |
| `Inertia` | Object state | `SymbolState` struct |
| `Universe` | Workspace | `Workspace` struct |
| `Player` | User/cursor | `CursorPosition` struct |
| TLA+ state machine | Document lifecycle | `DocumentState` enum |

### 1.2 Key Design Decisions

#### Ownership Strategy

```rust
// Rockies uses Rc<RefCell<T>> for shared mutable cells
use std::rc::Rc;
use std::cell::RefCell;

pub type GridCellRef<T> = Rc<RefCell<T>>;

// ewe_platform IDE: Use Arc for thread-safe sharing
use std::sync::Arc;

pub type SymbolRef = Arc<Symbol>;
pub type FileRef = Arc<File>;
```

#### Valtron Integration

```rust
// No async/await - use TaskIterator pattern
use valtron::{TaskIterator, TaskStatus, NoSpawner};

// Instead of:
// async fn load_file(path: &Path) -> Result<File> { ... }

// Use:
struct LoadFileTask {
    path: PathBuf,
    state: LoadState,
}

enum LoadState {
    Pending,
    Reading,
    Done(Result<File>),
}

impl TaskIterator for LoadFileTask {
    type Ready = Result<File>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            LoadState::Pending => {
                self.state = LoadState::Reading;
                Some(TaskStatus::Pending { wakeup: () })
            }
            LoadState::Reading => {
                let result = std::fs::read_to_string(&self.path)
                    .map(|content| File { path: self.path.clone(), content });
                Some(TaskStatus::Ready(result))
            }
            LoadState::Done(_) => None,
        }
    }
}
```

---

## 2. Core Type Translations

### 2.1 MultiGrid → SymbolIndex

```rust
// Rockies MultiGrid
use fnv::FnvHashMap;
use std::fmt::Debug;

pub struct MultiGrid<T> {
    grids: FnvHashMap<GridIndex, UniverseGrid<T>>,
    grid_width: usize,
    grid_height: usize,
}

// ewe_platform SymbolIndex
use std::collections::HashMap;
use std::sync::Arc;

pub struct SymbolIndex {
    /// Partition symbols into "grids" by module
    grids: HashMap<ModuleId, ModuleGrid>,
    /// Symbols per grid (like grid_width/height)
    symbols_per_grid: usize,
}

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub struct ModuleId {
    pub package_hash: u64,
    pub module_hash: u64,
}

pub struct ModuleGrid {
    /// Symbols in this module
    symbols: HashMap<String, Vec<Arc<Symbol>>>,
    /// Pre-computed references (like neighbors)
    references: HashMap<SymbolId, Vec<Reference>>,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub container: Option<SymbolId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u64);

#[derive(Debug, Clone, Copy)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    Variable,
    Parameter,
    Field,
    Module,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub uri: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub location: Location,
    pub is_definition: bool,
}
```

### 2.2 GridIndex → ModuleId

```rust
// Rockies GridIndex
#[derive(Hash, Eq, Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GridIndex {
    pub grid_offset: V2i,
}

impl GridIndex {
    pub fn from_pos(pos: V2i, width: usize, height: usize) -> GridIndex {
        GridIndex {
            grid_offset: V2i::new(
                pos.x.div_euclid(width as i32),
                pos.y.div_euclid(height as i32),
            ),
        }
    }

    pub fn to_pos(&self, width: usize, height: usize) -> V2i {
        V2i::new(
            self.grid_offset.x * width as i32,
            self.grid_offset.y * height as i32,
        )
    }
}

// ewe_platform ModuleId
impl ModuleId {
    /// Convert file path to module ID (like GridIndex::from_pos)
    pub fn from_path(path: &Path, symbols_per_grid: usize) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        let hash = hasher.finish();

        Self {
            package_hash: hash / symbols_per_grid as u64,
            module_hash: hash % symbols_per_grid as u64,
        }
    }

    /// Get representative path for module (like GridIndex::to_pos)
    pub fn to_path(&self, base_path: &Path) -> PathBuf {
        // In a real implementation, this would map back to actual paths
        base_path.join(format!("module_{}", self.module_hash))
    }
}
```

### 2.3 Grid → FileIndex

```rust
// Rockies Grid
pub struct Grid<T> {
    width: usize,
    height: usize,
    grid: Vec<GridCell<T>>,
    version: usize,
}

struct GridCell<T> {
    version: usize,
    value: Vec<GridCellRef<T>>,
    neighbors: Vec<GridCellRef<T>>,
}

// ewe_platform FileIndex
pub struct FileIndex {
    pub path: PathBuf,
    pub content_hash: u64,
    pub version: usize,
    /// Symbols at each line (like GridCell value)
    symbols_by_line: Vec<Vec<Arc<Symbol>>>,
    /// Pre-computed references (like neighbors)
    references: Vec<Reference>,
}

impl FileIndex {
    pub fn new(path: PathBuf, num_lines: usize) -> Self {
        Self {
            path,
            content_hash: 0,
            version: 0,
            symbols_by_line: vec![Vec::new(); num_lines],
            references: Vec::new(),
        }
    }

    /// Add symbol at position (like Grid::put)
    pub fn add_symbol(&mut self, symbol: Arc<Symbol>) {
        let line = symbol.location.line as usize;
        if line < self.symbols_by_line.len() {
            self.symbols_by_line[line].push(symbol);
        }
    }

    /// Get symbols at position (like Grid::get)
    pub fn get_symbols(&self, line: u32, column: u32) -> Vec<&Symbol> {
        let line_idx = line as usize;
        if line_idx >= self.symbols_by_line.len() {
            return Vec::new();
        }

        self.symbols_by_line[line_idx]
            .iter()
            .filter(|s| {
                let col = s.location.column;
                col <= column && column <= col + s.name.len() as u32
            })
            .map(|s| s.as_ref())
            .collect()
    }

    /// Update version on change (like Grid version system)
    pub fn update(&mut self, new_content: &str) {
        self.content_hash = self.hash_content(new_content);
        self.version += 1;
    }

    fn hash_content(&self, content: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}
```

### 2.4 Inertia → SymbolState

```rust
// Rockies Inertia
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Inertia {
    pub velocity: V2,
    pub force: V2,
    pub pos: V2,
    pub mass: i32,
    pub elasticity: f64,
    pub collision_stats: usize,
}

// ewe_platform SymbolState
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolState {
    /// Current type information (like position)
    pub type_info: TypeInfo,
    /// References count (like mass - "weight" of symbol)
    pub reference_count: usize,
    /// How "active" the symbol is (like velocity)
    pub activity_score: f64,
    /// Whether symbol is "static" (never changes)
    pub is_static: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeInfo {
    Unknown,
    Primitive(String),
    Struct(String),
    Function(Vec<TypeInfo>, Box<TypeInfo>),
    Generic(String, Vec<TypeInfo>),
}

impl SymbolState {
    pub fn new() -> Self {
        Self {
            type_info: TypeInfo::Unknown,
            reference_count: 0,
            activity_score: 0.0,
            is_static: false,
        }
    }

    /// Mark symbol as static (like Inertia::set_static)
    pub fn set_static(&mut self) {
        self.is_static = true;
        self.activity_score = 0.0;
    }

    /// Update reference count (like collision tracking)
    pub fn add_reference(&mut self) {
        self.reference_count += 1;
        self.activity_score += 1.0;
    }
}

impl Default for SymbolState {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## 3. Workspace Implementation

### 3.1 Universe → Workspace

```rust
// Rockies Universe
pub struct Universe {
    gravity: V2,
    dt: f64,
    pub cells: UniverseCells,
    pub player: Player,
}

// ewe_platform Workspace
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Workspace {
    /// Root path
    pub root: PathBuf,

    /// Open documents
    pub documents: HashMap<PathBuf, Arc<Document>>,

    /// Symbol index
    pub index: SymbolIndex,

    /// Configuration
    pub config: WorkspaceConfig,

    /// Dirty documents
    dirty_documents: HashSet<PathBuf>,
}

pub struct Document {
    pub path: PathBuf,
    pub content: String,
    pub version: i32,
    pub state: DocumentState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentState {
    NotStored,    // New file
    Stored,       // Saved
}

#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    pub exclude_patterns: Vec<String>,
    pub max_file_size: u64,
}

impl Workspace {
    pub fn new(root: PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            root: root.clone(),
            documents: HashMap::new(),
            index: SymbolIndex::new(100),  // 100 symbols per grid
            config: WorkspaceConfig {
                exclude_patterns: vec![
                    "**/node_modules/**".to_string(),
                    "**/target/**".to_string(),
                    "**/.git/**".to_string(),
                ],
                max_file_size: 10 * 1024 * 1024,
            },
            dirty_documents: HashSet::new(),
        })
    }

    /// Open a document (like loading a grid)
    pub fn open_document(&mut self, path: &Path) -> std::io::Result<Arc<Document>> {
        if let Some(doc) = self.documents.get(path) {
            return Ok(Arc::clone(doc));
        }

        let content = std::fs::read_to_string(path)?;
        let doc = Arc::new(Document {
            path: path.to_path_buf(),
            content,
            version: 1,
            state: DocumentState::Stored,
        });

        // Index symbols
        self.index_file(path, &doc.content);

        self.documents.insert(path.to_path_buf(), Arc::clone(&doc));
        Ok(doc)
    }

    /// Modify a document (like marking grid dirty)
    pub fn modify_document(&mut self, path: &Path, new_content: String) -> std::io::Result<()> {
        let doc = self.documents.get_mut(path).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Document not found")
        })?;

        let arc = Arc::make_mut(&mut *doc);
        arc.content = new_content;
        arc.version += 1;

        // Mark as dirty
        self.dirty_documents.insert(path.to_path_buf());

        // Re-index
        self.index_file(path, &arc.content);

        Ok(())
    }

    /// Save a document (like StoreGrid action)
    pub fn save_document(&mut self, path: &Path) -> std::io::Result<bool> {
        if let Some(doc) = self.documents.get_mut(path) {
            if self.dirty_documents.contains(path) {
                std::fs::write(path, &doc.content)?;
                doc.state = DocumentState::Stored;
                self.dirty_documents.remove(path);
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn index_file(&mut self, path: &Path, content: &str) {
        // Parse and add symbols to index
        // This would use a language-specific parser
        let symbols = self.parse_symbols(path, content);
        for symbol in symbols {
            self.index.add_symbol(symbol);
        }
    }

    fn parse_symbols(&self, path: &Path, content: &str) -> Vec<Arc<Symbol>> {
        // Placeholder - real implementation would use Tree-sitter or similar
        Vec::new()
    }
}
```

---

## 4. Valtron Executor Integration

### 4.1 Indexing Task

```rust
use valtron::{TaskIterator, TaskStatus, NoSpawner};

/// Task to index a file
pub struct IndexFileTask {
    path: PathBuf,
    content: Option<String>,
    state: IndexState,
}

enum IndexState {
    Pending,
    Reading,
    Parsing,
    Done(Result<Vec<Symbol>>),
}

impl TaskIterator for IndexFileTask {
    type Ready = Result<Vec<Symbol>>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            IndexState::Pending => {
                self.state = IndexState::Reading;
                Some(TaskStatus::Pending { wakeup: () })
            }
            IndexState::Reading => {
                match std::fs::read_to_string(&self.path) {
                    Ok(content) => {
                        self.content = Some(content);
                        self.state = IndexState::Parsing;
                        Some(TaskStatus::Pending { wakeup: () })
                    }
                    Err(e) => {
                        self.state = IndexState::Done(Err(e));
                        Some(TaskStatus::Pending { wakeup: () })
                    }
                }
            }
            IndexState::Parsing => {
                let content = self.content.take().unwrap();
                let symbols = parse_symbols(&self.path, &content);
                self.state = IndexState::Done(Ok(symbols));
                Some(TaskStatus::Pending { wakeup: () })
            }
            IndexState::Done(_) => None,
        }
    }
}

fn parse_symbols(path: &Path, content: &str) -> Vec<Symbol> {
    // Real implementation would use Tree-sitter
    Vec::new()
}
```

### 4.2 Save Document Task

```rust
/// Task to save a document
pub struct SaveDocumentTask {
    path: PathBuf,
    content: String,
    state: SaveState,
}

enum SaveState {
    Pending,
    Writing,
    Done(std::io::Result<()>),
}

impl TaskIterator for SaveDocumentTask {
    type Ready = std::io::Result<()>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            SaveState::Pending => {
                self.state = SaveState::Writing;
                Some(TaskStatus::Pending { wakeup: () })
            }
            SaveState::Writing => {
                let result = std::fs::write(&self.path, &self.content);
                self.state = SaveState::Done(result);
                Some(TaskStatus::Pending { wakeup: () })
            }
            SaveState::Done(ref result) => {
                Some(TaskStatus::Ready(result.clone()))
            }
        }
    }
}
```

### 4.3 Executor Integration

```rust
use valtron::{Executor, TaskHandle};

pub struct WorkspaceExecutor {
    executor: Executor,
    workspace: Arc<RwLock<Workspace>>,
}

impl WorkspaceExecutor {
    pub fn new(workspace: Arc<RwLock<Workspace>>) -> Self {
        Self {
            executor: Executor::new(),
            workspace,
        }
    }

    /// Schedule file indexing
    pub fn index_file(&mut self, path: PathBuf) -> TaskHandle<Result<Vec<Symbol>>> {
        let task = IndexFileTask {
            path,
            content: None,
            state: IndexState::Pending,
        };
        self.executor.spawn(task)
    }

    /// Schedule document save
    pub fn save_document(&mut self, path: PathBuf, content: String) -> TaskHandle<std::io::Result<()>> {
        let task = SaveDocumentTask {
            path,
            content,
            state: SaveState::Pending,
        };
        self.executor.spawn(task)
    }

    /// Run executor loop
    pub fn run(&mut self) {
        self.executor.run();
    }
}
```

---

## 5. TLA+ State Machine in Rust

### 5.1 Grid State Machine

From rockies TLA+:

```
Grid States:
- stored/not_stored
- loaded/not_loaded
- dirty/unmodified/pristine
```

### 5.2 Rust Implementation

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreState {
    NotStored,
    Stored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    NotLoaded,
    Loaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirtyState {
    Pristine,
    Unmodified,
    Dirty,
}

/// Complete document state (like TLA+ grid state)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentFullState {
    pub stored: StoreState,
    pub loaded: LoadState,
    pub dirty: DirtyState,
}

impl DocumentFullState {
    /// Initial state (like TLA+ Init)
    pub fn initial() -> Self {
        Self {
            stored: StoreState::NotStored,
            loaded: LoadState::NotLoaded,
            dirty: DirtyState::Pristine,
        }
    }

    /// LoadMissingGrid action
    pub fn load_missing(self) -> Self {
        assert_eq!(self.stored, StoreState::NotStored);
        assert_eq!(self.loaded, LoadState::NotLoaded);
        Self {
            stored: StoreState::NotStored,
            loaded: LoadState::Loaded,
            dirty: DirtyState::Pristine,
        }
    }

    /// LoadStoredGrid action
    pub fn load_stored(self) -> Self {
        assert_eq!(self.stored, StoreState::Stored);
        assert_eq!(self.loaded, LoadState::NotLoaded);
        Self {
            stored: StoreState::Stored,
            loaded: LoadState::Loaded,
            dirty: DirtyState::Unmodified,
        }
    }

    /// MarkDirty action
    pub fn mark_dirty(self) -> Self {
        assert_eq!(self.loaded, LoadState::Loaded);
        assert!(self.dirty == DirtyState::Pristine || self.dirty == DirtyState::Unmodified);
        Self {
            dirty: DirtyState::Dirty,
            ..self
        }
    }

    /// StoreGrid action
    pub fn store(self) -> Self {
        assert_eq!(self.loaded, LoadState::Loaded);
        assert_eq!(self.dirty, DirtyState::Dirty);
        Self {
            stored: StoreState::Stored,
            dirty: DirtyState::Unmodified,
            ..self
        }
    }

    /// UnloadGrid action
    pub fn unload(self) -> Self {
        assert_eq!(self.loaded, LoadState::Loaded);
        assert!(self.dirty == DirtyState::Pristine || self.dirty == DirtyState::Unmodified);
        Self {
            loaded: LoadState::NotLoaded,
            ..self
        }
    }
}
```

---

## 6. ewe_platform Integration

### 6.1 Project Structure

```
ewe_platform/
├── backends/
│   └── foundation_core/
│       └── src/
│           ├── valtron/           # Executor
│           └── ide/               # IDE components
│               ├── mod.rs
│               ├── workspace.rs   # Workspace (Universe equivalent)
│               ├── symbol.rs      # Symbol index (MultiGrid equivalent)
│               ├── document.rs    # Document management
│               └── lsp.rs         # LSP server
├── specifications/
│   └── 08-valtron-async-iterators/  # TaskIterator spec
```

### 6.2 Module Declaration

```rust
// ewe_platform/backends/foundation_core/src/ide/mod.rs

pub mod workspace;
pub mod symbol;
pub mod document;
pub mod lsp;

pub use workspace::Workspace;
pub use symbol::{SymbolIndex, Symbol, SymbolKind, SymbolId};
pub use document::{Document, DocumentState, DocumentFullState};
pub use lsp::LspServer;
```

---

*Next: [production-grade.md](production-grade.md)*
