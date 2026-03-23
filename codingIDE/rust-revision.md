# Rust Revision - codingIDE Reproduction Guide

## Overview

This document provides a comprehensive guide for reproducing the codingIDE projects in Rust, including architecture decisions, dependencies, and implementation strategies.

---

## Project Summary

The codingIDE collection consists of:

| Project | Purpose | Rust Viability |
|---------|---------|----------------|
| **Fresh Editor** | Terminal text editor | ✅ Already Rust |
| **Fresh Plugins** | Editor extensions | ⚠️ TypeScript (can add Rust plugin support) |
| **Fresh Registry** | Plugin metadata | ✅ JSON (Rust can consume) |
| **Radon IDE** | React Native IDE | ❌ VSCode extension (TypeScript) |
| **Rockies** | 2D physics game | ✅ Already Rust + WASM |

---

## Fresh Editor Reproduction

### Core Architecture

```rust
// Main crate structure
fresh-editor/
├── Cargo.toml
├── src/
│   ├── main.rs           // Entry point
│   ├── lib.rs            // Library root
│   ├── app/              // Application state machine
│   ├── model/            // Core data structures
│   ├── view/             // Rendering (ratatui)
│   ├── input/            // Input handling
│   ├── services/         // LSP, plugins, terminal
│   ├── primitives/       // Text utilities
│   ├── config/           // Configuration
│   └── wasm/             // WASM browser support
```

### Key Dependencies

```toml
[package]
name = "fresh-editor"
version = "0.1.0"
edition = "2021"

[dependencies]
# Terminal UI
ratatui = "0.30"
crossterm = "0.29"

# Async runtime
tokio = { version = "1.49", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Text handling
unicode-width = "0.2"
unicode-segmentation = "1.12"

# Syntax highlighting
syntect = { version = "5.3", features = ["default-fancy"] }

# Tree-sitter (optional, runtime-only)
tree-sitter = "0.26"
tree-sitter-highlight = "0.26"

# LSP support
lsp-types = "0.97"
tower-lsp = "0.20"

# Configuration
toml = "0.8"

# File watching
notify = "8.0"

# Git integration
git2 = "0.20"

# Plugin system (JavaScript/TypeScript)
rquickjs = { version = "0.11", features = ["bindgen", "futures"] }
rquickjs-serde = "0.4"

# TypeScript type generation
ts-rs = { version = "11.1", features = ["serde_json"] }

# WASM support (optional)
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
console_error_panic_hook = "0.1"
web-sys = { version = "0.3", features = ["console"] }
```

### Core Data Structures

#### Piece Tree Buffer

```rust
// model/piece_tree.rs
use std::sync::Arc;

/// A piece of content pointing into a rope
#[derive(Clone, Debug)]
pub struct Piece {
    pub start: usize,      // Offset in rope
    pub end: usize,        // End offset in rope
    pub next: Option<usize>, // Next piece index
}

/// Piece tree for efficient text editing
pub struct PieceTree {
    rope: Vec<char>,       // Raw content storage
    pieces: Vec<Piece>,    // Piece pointers
    root: usize,           // Root piece index
}

impl PieceTree {
    pub fn new() -> Self {
        Self {
            rope: Vec::new(),
            pieces: Vec::new(),
            root: 0,
        }
    }

    pub fn from_string(s: &str) -> Self {
        let mut tree = Self::new();
        tree.insert(0, s);
        tree
    }

    pub fn insert(&mut self, offset: usize, text: &str) {
        // Find the piece containing offset
        // Split the piece if necessary
        // Insert new piece with text
        // Update rope
    }

    pub fn delete(&mut self, range: Range<usize>) {
        // Find pieces overlapping range
        // Remove or trim pieces
        // Update rope
    }

    pub fn to_string(&self) -> String {
        // Iterate pieces in order, collect chars
        self.pieces.iter()
            .flat_map(|p| self.rope[p.start..p.end].iter())
            .collect()
    }

    pub fn line_count(&self) -> usize {
        // Count newlines in rope
        self.to_string().lines().count()
    }
}
```

#### Cursor Management

```rust
// model/cursor.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CursorId(pub usize);

#[derive(Debug, Clone)]
pub struct Cursor {
    pub id: CursorId,
    pub offset: usize,     // Byte offset in buffer
    pub affinity: Affinity, // Upstream or downstream
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Affinity {
    Upstream,
    Downstream,
}

/// Multiple cursor support
pub struct Cursors {
    cursors: Vec<Cursor>,
    primary: usize,        // Index of primary cursor
}

impl Cursors {
    pub fn new() -> Self {
        Self {
            cursors: vec![Cursor {
                id: CursorId(0),
                offset: 0,
                affinity: Affinity::Downstream,
            }],
            primary: 0,
        }
    }

    pub fn primary(&self) -> &Cursor {
        &self.cursors[self.primary]
    }

    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[self.primary]
    }

    pub fn add_cursor(&mut self, offset: usize) -> CursorId {
        let id = CursorId(self.cursors.len());
        self.cursors.push(Cursor {
            id,
            offset,
            affinity: Affinity::Downstream,
        });
        id
    }

    pub fn iter(&self) -> impl Iterator<Item = &Cursor> {
        self.cursors.iter()
    }
}
```

#### Event Log for Undo/Redo

```rust
// model/event.rs
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct TextEvent {
    pub cursor_id: CursorId,
    pub kind: EventKind,
}

#[derive(Debug, Clone)]
pub enum EventKind {
    Insert {
        offset: usize,
        text: String,
    },
    Delete {
        range: Range<usize>,
        text: String,  // For undo
    },
}

/// Undo/redo log
pub struct EventLog {
    undo_stack: Vec<TextEvent>,
    redo_stack: Vec<TextEvent>,
    max_events: usize,
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_events: 1000,
        }
    }

    pub fn record(&mut self, event: TextEvent) {
        self.undo_stack.push(event);
        self.redo_stack.clear();  // Clear redo on new action

        // Trim if too large
        if self.undo_stack.len() > self.max_events {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) -> Option<TextEvent> {
        if let Some(event) = self.undo_stack.pop() {
            let inverse = self.make_inverse(&event);
            self.redo_stack.push(inverse);
            Some(event)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<TextEvent> {
        if let Some(event) = self.redo_stack.pop() {
            self.undo_stack.push(event);
            Some(event)
        } else {
            None
        }
    }

    fn make_inverse(&self, event: &TextEvent) -> TextEvent {
        match &event.kind {
            EventKind::Insert { offset, text } => TextEvent {
                cursor_id: event.cursor_id,
                kind: EventKind::Delete {
                    range: *offset..(*offset + text.len()),
                    text: text.clone(),
                },
            },
            EventKind::Delete { range, text } => TextEvent {
                cursor_id: event.cursor_id,
                kind: EventKind::Insert {
                    offset: range.start,
                    text: text.clone(),
                },
            },
        }
    }
}
```

### Buffer Implementation

```rust
// model/buffer.rs
use std::sync::Arc;
use crate::model::{PieceTree, EventLog, Cursors, FileSystem};

pub struct Buffer {
    tree: PieceTree,
    events: EventLog,
    cursors: Cursors,
    fs: Arc<dyn FileSystem + Send + Sync>,
    path: Option<PathBuf>,
    modified: bool,
}

impl Buffer {
    pub fn empty(fs: Arc<dyn FileSystem>) -> Self {
        Self {
            tree: PieceTree::new(),
            events: EventLog::new(),
            cursors: Cursors::new(),
            fs,
            path: None,
            modified: false,
        }
    }

    pub fn from_string(content: &str, fs: Arc<dyn FileSystem>) -> Self {
        Self {
            tree: PieceTree::from_string(content),
            events: EventLog::new(),
            cursors: Cursors::new(),
            fs,
            path: None,
            modified: false,
        }
    }

    pub fn insert(&mut self, text: &str) {
        let offset = self.cursors.primary().offset;
        self.tree.insert(offset, text);

        // Record event for undo
        self.events.record(TextEvent {
            cursor_id: self.cursors.primary().id,
            kind: EventKind::Insert {
                offset,
                text: text.to_string(),
            },
        });

        // Update cursor
        self.cursors.primary_mut().offset += text.len();
        self.modified = true;
    }

    pub fn delete(&mut self, range: Range<usize>) {
        let text = self.tree.to_string()[range.clone()].to_string();
        self.tree.delete(range.clone());

        self.events.record(TextEvent {
            cursor_id: self.cursors.primary().id,
            kind: EventKind::Delete {
                range,
                text,
            },
        });

        self.modified = true;
    }

    pub fn undo(&mut self) {
        if let Some(event) = self.events.undo() {
            // Apply inverse event
            // Update cursor
        }
    }

    pub fn redo(&mut self) {
        if let Some(event) = self.events.redo() {
            // Apply event
            // Update cursor
        }
    }

    pub fn content(&self) -> String {
        self.tree.to_string()
    }

    pub fn save(&mut self) -> Result<(), anyhow::Error> {
        if let Some(path) = &self.path {
            self.fs.write(path, &self.content())?;
            self.modified = false;
        }
        Ok(())
    }
}
```

### File System Abstraction

```rust
// model/filesystem.rs
use std::path::{Path, PathBuf};
use anyhow::Result;

/// File system trait for abstraction
pub trait FileSystem {
    fn read(&self, path: &Path) -> Result<String>;
    fn write(&self, path: &Path, content: &str) -> Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
}

/// Standard file system implementation
#[cfg(feature = "runtime")]
pub struct StdFileSystem;

#[cfg(feature = "runtime")]
impl FileSystem for StdFileSystem {
    fn read(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(path).map_err(anyhow::Error::from)
    }

    fn write(&self, path: &Path, content: &str) -> Result<()> {
        std::fs::write(path, content).map_err(anyhow::Error::from)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        std::fs::read_dir(path)?
            .map(|entry| entry.map(|e| e.path()))
            .collect()
    }
}

/// No-op file system for WASM
pub struct NoopFileSystem;

impl FileSystem for NoopFileSystem {
    fn read(&self, _path: &Path) -> Result<String> {
        anyhow::bail!("NoopFileSystem cannot read files")
    }

    fn write(&self, _path: &Path, _content: &str) -> Result<()> {
        Ok(())
    }

    fn exists(&self, _path: &Path) -> bool {
        false
    }

    fn is_dir(&self, _path: &Path) -> bool {
        false
    }

    fn read_dir(&self, _path: &Path) -> Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }
}
```

### Rendering with Ratatui

```rust
// view/editor.rs
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Widget},
    text::{Line, Span},
};

pub struct EditorView<'a> {
    buffer: &'a Buffer,
    line_numbers: bool,
}

impl<'a> EditorView<'a> {
    pub fn new(buffer: &'a Buffer) -> Self {
        Self {
            buffer,
            line_numbers: true,
        }
    }
}

impl<'a> Widget for EditorView<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let content = self.buffer.content();
        let lines: Vec<&str> = content.lines().collect();

        let cursor = self.buffer.cursors.primary();
        let cursor_pos = offset_to_position(&content, cursor.offset);

        for (row_idx, line) in lines.iter().enumerate() {
            let y = area.y + row_idx as u16;
            if y >= area.bottom() {
                break;
            }

            // Render line number
            if self.line_numbers {
                let line_num = format!("{:>4} ", row_idx + 1);
                buf.set_string(
                    area.x,
                    y,
                    &line_num,
                    Style::default().fg(Color::DarkGray),
                );
            }

            // Render line content
            let content_x = if self.line_numbers { area.x + 5 } else { area.x };

            // Check if cursor is on this line
            if row_idx == cursor_pos.line {
                let col = cursor_pos.column;
                let before = &line[..col.min(line.len())];
                let after = &line[col.min(line.len())..];

                buf.set_string(content_x, y, before, Style::default());

                if col < line.len() {
                    // Render cursor character with cursor style
                    buf.set_string(
                        content_x + col as u16,
                        y,
                        &line[col..col+1],
                        Style::default().add_modifier(Modifier::REVERSED),
                    );
                } else {
                    // Render cursor at end of line
                    buf.set_string(
                        content_x + col as u16,
                        y,
                        " ",
                        Style::default().add_modifier(Modifier::REVERSED),
                    );
                }

                buf.set_string(
                    content_x + col as u16 + 1,
                    y,
                    after,
                    Style::default(),
                );
            } else {
                buf.set_string(content_x, y, line, Style::default());
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

fn offset_to_position(content: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut col = 0;
    let mut current_offset = 0;

    for ch in content.chars() {
        if current_offset == offset {
            return Position { line, column: col };
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        current_offset += ch.len_utf8();
    }

    Position { line, column: col }
}
```

### Input Handling

```rust
// input/handler.rs
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::App;

pub struct InputHandler;

impl InputHandler {
    pub fn handle_key(app: &mut App, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    Self::handle_ctrl(app, c);
                } else if key.modifiers.contains(KeyModifiers::ALT) {
                    Self::handle_alt(app, c);
                } else {
                    app.insert_char(c);
                }
            }
            KeyCode::Enter => {
                app.insert_char('\n');
            }
            KeyCode::Backspace => {
                app.delete_backward();
            }
            KeyCode::Delete => {
                app.delete_forward();
            }
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.move_word_left();
                } else {
                    app.move_left();
                }
            }
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.move_word_right();
                } else {
                    app.move_right();
                }
            }
            KeyCode::Up => {
                app.move_up();
            }
            KeyCode::Down => {
                app.move_down();
            }
            KeyCode::Home => {
                app.move_to_line_start();
            }
            KeyCode::End => {
                app.move_to_line_end();
            }
            KeyCode::PageUp => {
                app.scroll_page_up();
            }
            KeyCode::PageDown => {
                app.scroll_page_down();
            }
            _ => {}
        }
    }

    fn handle_ctrl(app: &mut App, c: char) {
        match c {
            's' => app.save(),
            'o' => app.open_file(),
            'q' => app.quit(),
            'z' => app.undo(),
            'y' => app.redo(),
            'c' => app.copy(),
            'x' => app.cut(),
            'v' => app.paste(),
            'f' => app.find(),
            'h' => app.replace(),
            'p' => app.command_palette(),
            _ => {}
        }
    }

    fn handle_alt(app: &mut App, c: char) {
        match c {
            'f' => app.move_word_right(),
            'b' => app.move_word_left(),
            _ => {}
        }
    }
}
```

### LSP Client

```rust
// services/lsp/client.rs
use lsp_types::*;
use tower_lsp::{
    lsp_types::*,
    Client,
    LanguageServer,
};
use tokio::sync::mpsc;

pub struct LspClient {
    client: Client,
    diagnostics: Vec<Diagnostic>,
}

impl LspClient {
    pub async fn start(command: &str, args: &[&str]) -> Result<Self, anyhow::Error> {
        // Start language server process
        // Initialize connection
        // Return client
        todo!()
    }

    pub async fn open_document(&self, uri: &str, text: &str, version: i32) {
        self.client.did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.parse().unwrap(),
                language_id: "rust".to_string(),
                version,
                text: text.to_string(),
            },
        }).await;
    }

    pub async fn goto_definition(&self, uri: &str, position: Position) -> Result<Vec<Location>> {
        let result = self.client.goto_definition(
            GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: uri.parse().unwrap(),
                    },
                    position,
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            }
        ).await?;

        Ok(match result {
            Some(GotoDefinitionResponse::Scalar(locations)) => locations,
            Some(GotoDefinitionResponse::Array(locations)) => locations,
            _ => vec![],
        })
    }

    pub async fn hover(&self, uri: &str, position: Position) -> Result<Option<Hover>> {
        Ok(self.client.hover(
            HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: uri.parse().unwrap(),
                    },
                    position,
                },
                work_done_progress_params: Default::default(),
            }
        ).await?)
    }

    pub async fn completions(
        &self,
        uri: &str,
        position: Position
    ) -> Result<Vec<CompletionItem>> {
        let result = self.client.completion(
            CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: uri.parse().unwrap(),
                    },
                    position,
                },
                context: None,
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            }
        ).await?;

        Ok(match result {
            Some(CompletionResponse::Array(items)) => items,
            Some(CompletionResponse::List(list)) => list.items,
            None => vec![],
        })
    }
}
```

### Configuration System

```rust
// config/mod.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub editor: EditorConfig,
    pub keybindings: KeybindingsConfig,
    pub plugins: PluginsConfig,
    pub themes: ThemesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub line_numbers: bool,
    pub line_wrap: bool,
    pub tab_width: usize,
    pub auto_indent: bool,
    pub auto_save: bool,
    pub theme: String,
    pub font_size: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    pub mode: String,  // "default", "vim", "emacs"
    pub custom: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsConfig {
    pub enabled: Vec<String>,
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemesConfig {
    pub default: String,
    pub custom_themes: std::collections::HashMap<String, Theme>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub selection: String,
}

impl Config {
    pub fn load() -> Result<Self, anyhow::Error> {
        let config_path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find config directory"))?
            .join("fresh")
            .join("config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<(), anyhow::Error> {
        let config_path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find config directory"))?
            .join("fresh")
            .join("config.toml");

        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            editor: EditorConfig {
                line_numbers: true,
                line_wrap: false,
                tab_width: 4,
                auto_indent: true,
                auto_save: false,
                theme: "default".to_string(),
                font_size: 14,
            },
            keybindings: KeybindingsConfig {
                mode: "default".to_string(),
                custom: std::collections::HashMap::new(),
            },
            plugins: PluginsConfig {
                enabled: vec![],
                disabled: vec![],
            },
            themes: ThemesConfig {
                default: "default".to_string(),
                custom_themes: std::collections::HashMap::new(),
            },
        }
    }
}
```

---

## Rockies Reproduction

Rockies is already fully implemented in Rust. Here's a summary of the key components:

### Core Dependencies

```toml
[dependencies]
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.5"
noise = "0.9"
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"
chrono = "0.4"
getrandom = { version = "0.3", features = ["wasm_js"] }

# Terminal rendering (optional)
sdl2 = { version = "0.37", optional = true }
```

### Key Structures

```rust
// See rockies-exploration.md for detailed analysis
```

---

## Plugin System (TypeScript)

Fresh uses a TypeScript plugin system. Here's how to set it up:

### QuickJS Runtime

```rust
// plugin/runtime.rs
use rquickjs::{Context, Runtime, Module, Object};

pub struct PluginRuntime {
    runtime: Runtime,
    context: Context,
}

impl PluginRuntime {
    pub fn new() -> Result<Self, anyhow::Error> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;

        Ok(Self { runtime, context })
    }

    pub fn load_plugin(&self, source: &str) -> Result<(), anyhow::Error> {
        self.context.with(|ctx| {
            let globals = ctx.globals();
            let eval: rquickjs::Function = globals.get("eval")?;

            // Execute plugin code
            eval.call::<_, ()>((source,))?;

            Ok(())
        })
    }
}
```

---

## Build and Distribution

### Cargo Workspace

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/fresh-core",
    "crates/fresh-editor",
    "crates/fresh-parser-js",
    "crates/fresh-languages",
    "crates/fresh-plugin-runtime",
    "crates/fresh-plugin-api-macros",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "GPL-2.0"
```

### Build Commands

```bash
# Development build
cargo build

# Release build
cargo build --release

# WASM build
wasm-pack build --target web

# Distribution
cargo dist build
```

---

## Related Documents

- [Fresh Editor](fresh-exploration.md) - Editor exploration
- [Rockies](rockies-exploration.md) - WASM game example
- [WASM Analysis](wasm-web-editor-analysis.md) - Web editor feasibility
