---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/fresh
repository: https://github.com/sinelaw/fresh
revised_at: 2026-03-27
target_platform: ewe_platform with valtron executor
---

# Rust Revision: Fresh Editor for ewe_platform

## Overview

This document provides a complete Rust translation guide for implementing Fresh's core features in ewe_platform using the valtron executor (no async/await, no tokio).

---

## Part 1: Architecture Differences

### Fresh vs ewe_platform

| Aspect | Fresh | ewe_platform |
|--------|-------|--------------|
| **Async Runtime** | tokio | valtron TaskIterator |
| **Plugin Runtime** | QuickJS + tokio | valtron executor |
| **Threading** | Multi-threaded | Single-threaded (initially) |
| **IPC** | Channels | Direct calls |
| **File I/O** | tokio::fs | std::fs (blocking) |

### Key Translation Patterns

```rust
// Fresh: Async function with tokio
async fn load_plugin(path: &Path) -> Result<Plugin> {
    let source = tokio::fs::read_to_string(path).await?;
    // ...
}

// ewe_platform: TaskIterator pattern
struct LoadPluginTask {
    path: PathBuf,
    state: LoadPluginState,
    result: Option<Result<Plugin>>,
}

enum LoadPluginState {
    Reading,
    Parsing,
    Done,
}

impl TaskIterator for LoadPluginTask {
    type Ready = Plugin;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            LoadPluginState::Reading => {
                // Read file (blocking, but quick for small files)
                match std::fs::read_to_string(&self.path) {
                    Ok(source) => {
                        self.state = LoadPluginState::Parsing;
                        // Store source for next iteration
                    }
                    Err(e) => {
                        self.state = LoadPluginState::Done;
                        self.result = Some(Err(e.into()));
                    }
                }
                TaskStatus::Pending(())
            }
            LoadPluginState::Parsing => {
                // Parse and compile plugin
                match self.parse_plugin() {
                    Ok(plugin) => {
                        self.state = LoadPluginState::Done;
                        TaskStatus::Ready(plugin)
                    }
                    Err(e) => {
                        self.state = LoadPluginState::Done;
                        self.result = Some(Err(e));
                        TaskStatus::Pending(())
                    }
                }
            }
            LoadPluginState::Done => {
                // Should not reach here if handled correctly
                None
            }
        }
    }
}
```

---

## Part 2: Core Data Structures

### Piece Tree (Direct Translation)

Fresh's piece tree is already pure Rust and can be used directly:

```rust
// From fresh/crates/fresh-editor/src/model/piece_tree.rs
// Can be copied directly to ewe_platform

pub enum PieceTreeNode {
    Internal {
        left: Arc<PieceTreeNode>,
        right: Arc<PieceTreeNode>,
        left_bytes: usize,
        lf_left: Option<usize>,
    },
    Leaf {
        location: BufferLocation,
        offset: usize,
        bytes: usize,
        line_feed_cnt: Option<usize>,
    },
}

pub struct PieceTree {
    root: Option<Arc<PieceTreeNode>>,
}

impl PieceTree {
    pub fn empty() -> Self {
        Self { root: None }
    }

    pub fn find_byte_offset(&self, target: usize) -> (PieceInfo, usize) {
        // Implementation from Fresh - no changes needed
        todo!()
    }

    pub fn find_line(&self, target_line: usize) -> (usize, usize) {
        // Implementation from Fresh - no changes needed
        todo!()
    }
}
```

### TextBuffer Adaptation

```rust
// Adapted for ewe_platform (no async filesystem)
pub struct TextBuffer {
    fs: Arc<dyn FileSystem>,  // Can be blocking implementation
    piece_tree: PieceTree,
    saved_root: Arc<PieceTreeNode>,
    buffers: Vec<StringBuffer>,
    next_buffer_id: usize,
    file_path: Option<PathBuf>,
    modified: bool,
    large_file: bool,
    line_ending: LineEnding,
    encoding: Encoding,
    version: u64,
}

// Blocking FileSystem trait
pub trait FileSystem {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn write(&self, path: &Path, data: &[u8]) -> io::Result<()>;
    fn metadata(&self, path: &Path) -> io::Result<FileMetadata>;
    fn read_range(&self, path: &Path, offset: u64, len: usize) -> io::Result<Vec<u8>>;
}

// Blocking implementation
pub struct StdFileSystem;

impl FileSystem for StdFileSystem {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        std::fs::write(path, data)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let meta = std::fs::metadata(path)?;
        Ok(FileMetadata {
            size: meta.len() as usize,
            modified: meta.modified().ok(),
        })
    }

    fn read_range(&self, path: &Path, offset: u64, len: usize) -> io::Result<Vec<u8>> {
        let mut file = std::fs::File::open(path)?;
        file.seek(std::io::SeekFrom::Start(offset))?;

        let mut buffer = vec![0u8; len];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);

        Ok(buffer)
    }
}
```

---

## Part 3: Valtron Integration

### Executor Setup

```rust
use valtron::{
    Executor,
    TaskIterator,
    TaskStatus,
    NoSpawner,
};

pub struct EditorExecutor {
    executor: Executor<EditorTask>,
    state: EditorState,
}

enum EditorTask {
    LoadBuffer(LoadBufferTask),
    SaveBuffer(SaveBufferTask),
    RunPlugin(RunPluginTask),
    // ... more tasks
}

impl EditorExecutor {
    pub fn new() -> Self {
        Self {
            executor: Executor::new(),
            state: EditorState::new(),
        }
    }

    pub fn load_buffer(&mut self, path: PathBuf) {
        let task = LoadBufferTask::new(path);
        self.executor.spawn(task);
    }

    pub fn tick(&mut self) -> Result<()> {
        // Process all ready tasks
        while let Some(result) = self.executor.poll() {
            match result {
                EditorTaskResult::BufferLoaded(buffer) => {
                    self.state.add_buffer(buffer);
                }
                EditorTaskResult::BufferSaved { path, success } => {
                    if success {
                        self.state.mark_saved(&path);
                    }
                }
                // ... handle other results
            }
        }

        Ok(())
    }
}
```

### LoadBuffer Task

```rust
pub struct LoadBufferTask {
    path: PathBuf,
    state: LoadBufferState,
    data: Option<Vec<u8>>,
}

enum LoadBufferState {
    Reading,
    DetectingEncoding,
    BuildingPieceTree,
    Done,
}

impl TaskIterator for LoadBufferTask {
    type Ready = TextBuffer;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            LoadBufferState::Reading => {
                // Read file (could be large, so chunk it)
                match std::fs::read(&self.path) {
                    Ok(data) => {
                        self.data = Some(data);
                        self.state = LoadBufferState::DetectingEncoding;
                    }
                    Err(_) => {
                        self.state = LoadBufferState::Done;
                        // Error handling
                    }
                }
                TaskStatus::Pending(())
            }
            LoadBufferState::DetectingEncoding => {
                let data = self.data.take().unwrap();
                let encoding = TextBuffer::detect_encoding(&data);

                self.data = Some(data);
                self.state = LoadBufferState::BuildingPieceTree;

                TaskStatus::Pending(())
            }
            LoadBufferState::BuildingPieceTree => {
                let data = self.data.take().unwrap();

                let buffer = if data.len() > LARGE_FILE_THRESHOLD {
                    // Large file: create unloaded buffer
                    TextBuffer::new_lazy(&self.path, Arc::new(StdFileSystem))
                } else {
                    // Small file: load fully
                    TextBuffer::new_with_content(data, Arc::new(StdFileSystem))
                };

                self.state = LoadBufferState::Done;
                TaskStatus::Ready(buffer)
            }
            LoadBufferState::Done => None,
        }
    }
}
```

### SaveBuffer Task

```rust
pub struct SaveBufferTask {
    buffer_snapshot: BufferSnapshot,
    path: PathBuf,
    state: SaveBufferState,
    temp_path: Option<PathBuf>,
}

enum SaveBufferState {
    BuildingWriteRecipe,
    WritingTempFile,
    ReplacingOriginal,
    Done,
}

impl TaskIterator for SaveBufferTask {
    type Ready = SaveResult;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            SaveBufferState::BuildingWriteRecipe => {
                // Build write recipe from piece tree
                let recipe = self.build_write_recipe();
                self.temp_path = Some(self.create_temp_file()?);
                self.state = SaveBufferState::WritingTempFile;
                TaskStatus::Pending(())
            }
            SaveBufferState::WritingTempFile => {
                // Write to temp file
                let temp_path = self.temp_path.as_ref().unwrap();
                match self.write_temp_file(temp_path) {
                    Ok(_) => {
                        self.state = SaveBufferState::ReplacingOriginal;
                    }
                    Err(e) => {
                        self.state = SaveBufferState::Done;
                        return Some(TaskStatus::Ready(SaveResult::Error(e)));
                    }
                }
                TaskStatus::Pending(())
            }
            SaveBufferState::ReplacingOriginal => {
                // Replace original file atomically
                let temp_path = self.temp_path.take().unwrap();
                match std::fs::rename(&temp_path, &self.path) {
                    Ok(_) => {
                        self.state = SaveBufferState::Done;
                        Some(TaskStatus::Ready(SaveResult::Success))
                    }
                    Err(e) => {
                        self.state = SaveBufferState::Done;
                        Some(TaskStatus::Ready(SaveResult::Error(e.into())))
                    }
                }
            }
            SaveBufferState::Done => None,
        }
    }
}
```

---

## Part 4: Plugin System Without Tokio

### Simplified Plugin Architecture

```rust
// Without tokio, plugins run synchronously or use valtron for async

pub struct PluginManager {
    plugins: HashMap<String, LoadedPlugin>,
    task_queue: Vec<PluginTask>,
}

enum PluginTask {
    Load(LoadPluginTask),
    Execute(ExecutePluginTask),
    RunHook(RunHookTask),
}

impl PluginManager {
    pub fn load_plugin(&mut self, path: &Path) {
        let task = LoadPluginTask::new(path.to_path_buf());
        self.task_queue.push(PluginTask::Load(task));
    }

    pub fn execute_action(&mut self, plugin: &str, action: &str) {
        let task = ExecutePluginTask::new(plugin.to_string(), action.to_string());
        self.task_queue.push(PluginTask::Execute(task));
    }

    pub fn process_tasks(&mut self) -> Vec<PluginResult> {
        let mut results = Vec::new();

        for task in self.task_queue.drain(..) {
            match task {
                PluginTask::Load(mut t) => {
                    while let Some(status) = t.next() {
                        if let TaskStatus::Ready(plugin) = status {
                            results.push(PluginResult::PluginLoaded(plugin));
                            break;
                        }
                    }
                }
                PluginTask::Execute(mut t) => {
                    while let Some(status) = t.next() {
                        if let TaskStatus::Ready(result) = status {
                            results.push(PluginResult::ActionExecuted(result));
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        results
    }
}
```

### QuickJS Without Async

```rust
// QuickJS can still be used, but async operations need different handling

pub struct QuickJsPlugin {
    ctx: Ctx<'static>,
    actions: HashMap<String, JsFunction>,
    hooks: HashMap<String, JsFunction>,
}

impl QuickJsPlugin {
    pub fn execute_action(&mut self, action_name: &str) -> Result<serde_json::Value> {
        if let Some(action) = self.actions.get(action_name) {
            // Execute synchronously
            let result: serde_json::Value = action.call(())?;
            Ok(result)
        } else {
            Err(Error::NoSuchAction(action_name.to_string()))
        }
    }

    pub fn run_hook(&mut self, hook_name: &str, args: serde_json::Value) -> Result<()> {
        if let Some(hook) = self.hooks.get(hook_name) {
            hook.call((args,))?;
        }
        Ok(())
    }
}
```

---

## Part 5: Input Handling

### Blocking Input Loop

```rust
use crossterm::event::{read, Event, KeyEvent, MouseEvent};

pub struct InputLoop {
    running: bool,
    keybindings: HashMap<KeyBinding, Action>,
}

impl InputLoop {
    pub fn run(&mut self, editor: &mut Editor) -> Result<()> {
        self.running = true;

        while self.running {
            // Process any pending tasks first
            editor.executor.tick()?;

            // Render
            editor.render()?;

            // Wait for input (blocking)
            match read()? {
                Event::Key(key) => self.handle_key(key, editor)?,
                Event::Mouse(mouse) => self.handle_mouse(mouse, editor)?,
                Event::Resize(w, h) => editor.handle_resize(w, h)?,
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent, editor: &mut Editor) -> Result<()> {
        let binding = KeyBinding::from(key);

        if let Some(action) = self.keybindings.get(&binding) {
            editor.execute_action(action)?;
        } else if let KeyCode::Char(c) = key.code {
            if !key.modifiers.ctrl && !key.modifiers.alt {
                editor.insert_char(c)?;
            }
        }

        Ok(())
    }
}
```

---

## Part 6: Rendering (Unchanged)

Fresh's rendering with ratatui works identically in ewe_platform:

```rust
use ratatui::{
    Frame,
    Terminal,
    backend::CrosstermBackend,
};

pub fn render(editor: &Editor, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    terminal.draw(|frame| {
        let area = frame.size();

        // Render buffer
        let buffer_view = editor.get_visible_lines();
        let paragraph = Paragraph::new(buffer_view)
            .block(Block::bordered().title(editor.current_file()));

        frame.render_widget(paragraph, area);

        // Render status bar
        let status = format!("Line {}:{}", editor.cursor_line() + 1, editor.cursor_col() + 1);
        let status_bar = Paragraph::new(status)
            .style(Style::default().bg(Color::Blue).fg(Color::White));

        let status_area = Rect::new(0, area.height - 1, area.width, 1);
        frame.render_widget(status_bar, status_area);
    })?;

    Ok(())
}
```

---

## Part 7: Dependencies

### Cargo.toml for ewe_platform

```toml
[package]
name = "ewe-editor"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
anyhow = "1.0"

# Terminal UI
ratatui = "0.30"
crossterm = "0.29"

# Valtron executor
valtron = { path = "/home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron" }

# Syntax highlighting
syntect = { version = "5.3", default-features = false, features = ["regex-fancy"] }
tree-sitter = "0.26"
tree-sitter-highlight = "0.26"

# JavaScript runtime (for plugins)
rquickjs = { version = "0.11", features = ["bindgen", "macro"] }
rquickjs-serde = "0.4"

# TypeScript transpilation
oxc_allocator = "0.112"
oxc_parser = "0.112"
oxc_transformer = "0.112"
oxc_codegen = "0.112"

# Utilities
unicode-width = "0.2"
unicode-segmentation = "1.12"
encoding_rs = "0.8"
chardetng = "0.1"
dirs = "6.0"

# Optional: for LSP support
lsp-types = "0.97"
```

---

## Part 8: Migration Checklist

### Phase 1: Core Buffer
- [ ] Copy piece_tree.rs (no changes needed)
- [ ] Copy buffer.rs (adapt FileSystem trait)
- [ ] Copy encoding.rs (no changes needed)
- [ ] Implement blocking FileSystem

### Phase 2: View Layer
- [ ] Copy view modules (no changes needed)
- [ ] Copy primitives (no changes needed)
- [ ] Integrate ratatui rendering

### Phase 3: Input
- [ ] Copy input modules (remove async)
- [ ] Implement blocking input loop
- [ ] Copy keybindings system

### Phase 4: Commands
- [ ] Copy command system (no changes needed)
- [ ] Implement undo/redo with snapshots
- [ ] Copy command palette

### Phase 5: Plugins (Optional)
- [ ] Adapt QuickJS backend for sync execution
- [ ] Remove tokio dependency
- [ ] Use valtron for async operations

---

## Resources

- [Valtron Documentation](/home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron/README.md)
- [TaskIterator Spec](/home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators/)
- [Fresh Source Code](/home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/fresh/)
