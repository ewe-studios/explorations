# Fresh Editor Exploration

## Overview

**Fresh** is a modern, full-featured terminal text editor written in Rust, designed to provide VS Code-like user experience in the terminal with zero configuration.

- **Repository**: https://github.com/sinelaw/fresh
- **Version**: 0.2.3
- **License**: GPL-2.0
- **Author**: Noam Lewis
- **Website**: https://getfresh.dev

### Design Philosophy

> "Fresh brings the intuitive UX of VS Code and Sublime Text to the terminal. Standard keybindings, full mouse support, menus, and a command palette — everything works the way you'd expect, right out of the box. No modes, no memorizing shortcuts."

Fresh handles multi-gigabyte files with negligible memory overhead using a piece-tree data structure, delivering consistently low-latency input regardless of file size.

---

## Architecture

### Workspace Structure

```
fresh/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── fresh-editor/          # Main application binary
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs        # Entry point
│   │   │   ├── lib.rs         # Library root
│   │   │   ├── app/           # Application state machine
│   │   │   ├── model/         # Core data structures
│   │   │   ├── view/          # Rendering (ratatui)
│   │   │   ├── input/         # Input handling
│   │   │   ├── services/      # LSP, plugins, terminal
│   │   │   ├── primitives/    # Text utilities
│   │   │   ├── config/        # Configuration system
│   │   │   ├── wasm/          # WASM browser support
│   │   │   └── ...
│   │   └── plugins/           # TypeScript plugins
│   │
│   ├── fresh-core/            # Shared types & API
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── action.rs      # Action types
│   │   │   ├── api.rs         # Plugin API
│   │   │   ├── command.rs     # Command types
│   │   │   ├── config.rs      # Config types
│   │   │   ├── hooks.rs       # Plugin hooks
│   │   │   ├── file_explorer.rs
│   │   │   ├── menu.rs
│   │   │   ├── overlay.rs
│   │   │   └── text_property.rs
│   │
│   ├── fresh-parser-js/       # JavaScript/TypeScript parser
│   │   └── src/
│   │       └── lib.rs         # oxc-based TS parser
│   │
│   ├── fresh-languages/       # Tree-sitter language bindings
│   │   └── src/
│   │       └── lib.rs         # Language registry
│   │
│   ├── fresh-plugin-runtime/  # TypeScript plugin runtime
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── backend.rs     # QuickJS backend
│   │       ├── process.rs     # Process spawning
│   │       └── thread.rs      # Plugin thread
│   │
│   └── fresh-plugin-api-macros/  # Proc macros for plugin API
│       └── src/
│           └── lib.rs
│
├── plugins/                   # TypeScript plugins (embedded)
│   ├── lib/
│   │   ├── fresh.d.ts         # Auto-generated API types
│   │   └── index.ts
│   ├── examples/              # Example plugins
│   ├── *.ts                   # Built-in plugins
│   └── config-schema.json
│
├── themes/                    # Color themes
│   └── *.json
│
├── locales/                   # i18n translations
│   └── *.yml
│
└── docs/                      # Documentation
    └── wasm.md                # WASM compatibility guide
```

---

## Core Crates

### 1. fresh-core

The foundation crate providing shared types:

```rust
// Unique identifiers
pub struct CursorId(pub usize);
pub struct BufferId(pub usize);
pub struct SplitId(pub usize);
pub struct TerminalId(pub usize);

// Split direction
pub enum SplitDirection {
    Horizontal,
    Vertical,
}
```

**Key modules**:
- `action.rs` - Editor actions (commands, keybindings)
- `api.rs` - Plugin API types
- `command.rs` - Command palette commands
- `config.rs` - Configuration structures
- `hooks.rs` - Plugin hook system

### 2. fresh-editor

Main application crate with feature flags:

```toml
[features]
default = ["plugins", "runtime", "embed-plugins"]
plugins = ["dep:fresh-plugin-runtime", ...]
runtime = ["dep:crossterm", "dep:ratatui", "dep:tokio", ...]
wasm = ["dep:ratatui", "dep:syntect", "syntect/regex-fancy", ...]
dev-bins = ["dep:ratatui"]
```

**Module organization**:

| Module | WASM Status | Description |
|--------|-------------|-------------|
| `model/` | 100% Ready | Buffer, PieceTree, Cursor, Event |
| `primitives/` | 100% Ready | Text manipulation utilities |
| `config/` | 100% Ready | Configuration system |
| `view/` | ~70% Ready | Ratatui rendering |
| `input/` | ~21% Ready | Key/mouse handling |
| `services/` | ~13% Ready | LSP, plugins, clipboard |
| `app/` | Needs work | Application orchestration |
| `wasm/` | In-progress | WASM browser module |

### 3. fresh-parser-js

JavaScript/TypeScript parser using oxc:

```toml
[dependencies]
oxc_allocator = "0.112.0"
oxc_ast = "0.112.0"
oxc_parser = "0.112.0"
oxc_transformer = "0.112.0"
oxc_codegen = "0.112.0"
oxc_semantic = "0.112.0"
```

Used for TypeScript plugin transpilation at runtime.

### 4. fresh-languages

Tree-sitter language bindings:

```toml
[dependencies]
tree-sitter = "0.26.5"
tree-sitter-highlight = "0.26.3"
tree-sitter-rust = "0.24.0"
tree-sitter-python = "0.25.0"
tree-sitter-javascript = "0.25.0"
# ... 15+ languages
```

**Features**:
- Syntax highlighting via tree-sitter
- Smart indentation
- Reference highlighting

### 5. fresh-plugin-runtime

TypeScript plugin runtime using QuickJS:

```toml
[dependencies]
rquickjs = "0.11"  # QuickJS bindings
rquickjs-serde = "0.4"
```

**Architecture**:
```
┌─────────────────┐     ┌─────────────────┐
│   Main Thread   │────▶│  Plugin Thread  │
│   (UI/Event)    │◀────│  (QuickJS + TS) │
└─────────────────┘     └─────────────────┘
         │                       │
         │ PluginCommand         │ Hook execution
         │                       │
         ▼                       ▼
┌─────────────────┐     ┌─────────────────┐
│  Async Bridge   │◀────│  Process Spawner│
│  (tokio)        │     │  (git, etc.)    │
└─────────────────┘     └─────────────────┘
```

---

## Plugin System

### Plugin API

Plugins are written in TypeScript and have access to:

```typescript
// Get editor API instance
const editor = getEditor();

// Query state
const buffers = editor.buffers();
const content = editor.bufferContent(bufferId);
const cursors = editor.cursors();

// Modify content
editor.insert(bufferId, offset, text);
editor.delete(bufferId, start, end);

// Visual decorations
editor.addOverlay(bufferId, overlay);

// UI interaction
editor.statusMessage("Hello from plugin!");
```

### Built-in Plugins

| Plugin | Description |
|--------|-------------|
| `git_gutter.ts` | Git diff indicators |
| `git_log.ts` | Git log viewer |
| `git_blame.ts` | Git blame annotations |
| `diagnostics_panel.ts` | LSP diagnostics panel |
| `code-tour.ts` | Code walkthrough/tour |
| `clangd-lsp.ts` | C/C++ LSP integration |
| `typescript.ts` | TypeScript LSP |
| `emmet.ts` | Emmet abbreviation |
| `color-highlighter.ts` | Color preview |
| `todo-highlighter.ts` | TODO annotation highlighting |

### Plugin Hooks

```typescript
// Hook registration
editor.on('buffer:modified', (buffer) => {
  console.log('Buffer modified:', buffer);
});

editor.on('key:press', (key) => {
  // Intercept key events
  return true; // consume event
});
```

---

## Key Features Deep Dive

### 1. Piece Tree Buffer

Fresh uses a piece-tree data structure for efficient large file handling:

```rust
pub struct PieceTree {
    // Tree of "pieces" pointing into a rope of raw content
    // Each piece represents a substring with metadata
}

impl PieceTree {
    pub fn insert(&mut self, offset: usize, text: &str);
    pub fn delete(&mut self, range: Range<usize>);
    pub fn line_count(&self) -> usize;
    pub fn position_to_line_col(&self, offset: usize) -> (usize, usize);
}
```

**Benefits**:
- O(log n) insert/delete
- Efficient undo/redo via event log
- Memory-efficient for large files
- Supports multiple cursors naturally

### 2. LSP Integration

Full Language Server Protocol support:

```rust
// LSP client in services/lsp/
pub struct LspClient {
    server: LanguageServer,
    diagnostics: DiagnosticCollection,
    completions: CompletionCache,
}

// Features:
// - Go to definition
// - Find references
// - Hover documentation
// - Code actions
// - Rename symbol
// - Diagnostics
// - Autocompletion
```

### 3. Syntax Highlighting

Two-tier approach:

1. **TextMate Grammars** (Syntect) - WASM-compatible
   - 100+ language support
   - `.tmLanguage` JSON/plist grammars
   - Fast regex-based highlighting

2. **Tree-sitter** (Runtime only)
   - AST-based semantic highlighting
   - More accurate but slower
   - Requires WASM tree-sitter grammars

### 4. Configuration System

```rust
#[derive(JsonSchema, Serialize, Deserialize)]
pub struct Config {
    pub editor: EditorConfig,
    pub keybindings: KeybindingsConfig,
    pub plugins: PluginConfig,
    pub themes: ThemeConfig,
}

// Schema generation
cargo run --bin generate_schema
```

Configuration files:
- `~/.config/fresh/config.toml` - Main config
- `~/.config/fresh/keybindings.toml` - Keybindings
- `~/.config/fresh/themes/` - Custom themes

---

## Build System

### Cargo Features

```toml
# Default features
default = ["plugins", "runtime", "embed-plugins"]

# Plugin support
plugins = ["dep:fresh-plugin-runtime", "dep:fresh-parser-js"]

# Runtime (native terminal)
runtime = [
    "dep:crossterm",
    "dep:ratatui",
    "dep:tokio",
    "dep:lsp-types",
    "dep:alacritty_terminal",
    "dep:portable-pty",
    # ...
]

# WASM browser build
wasm = [
    "dep:ratatui",  # Without crossterm backend
    "dep:syntect",
    "syntect/regex-fancy",  # Pure Rust regex
]

# Development binaries
dev-bins = ["dep:ratatui"]
```

### Build Commands

```bash
# Native release build
cargo build --release

# WASM browser build
cargo build --no-default-features --features wasm --target wasm32-unknown-unknown

# Schema generation
cargo run --features dev-bins --bin generate_schema

# Event debug binary
cargo run --features dev-bins,runtime --bin event_debug
```

---

## Internationalization

```rust
// i18n setup in src/i18n.rs
rust_i18n::i18n!(
    "locales-empty",
    fallback = "en",
    backend = i18n::runtime_backend::RuntimeBackend::new()
);

// Usage in code
t!("editor.save_prompt", filename = path)
```

Supported locales:
- `en` - English (default)
- Additional locales in `locales/`

---

## Testing Strategy

```rust
// Unit tests in each module
#[cfg(test)]
mod tests {
    #[test]
    fn test_piece_tree_insert() { ... }

    #[test]
    fn test_cursor_movement() { ... }
}

// E2E tests (send keyboard events, examine rendered output)
#[test]
fn test_open_file_and_save() {
    let mut app = App::test_mode();
    app.send_keys("C-o");
    app.type_text("test.txt");
    app.send_key(KeyCode::Enter);
    // ... assertions
}
```

**Testing principles**:
1. Reproduce before fixing (test cases for bugs)
2. E2E tests for new flows
3. No timeouts - use semantic waiting
4. Test isolation (parallel execution)
5. Cross-platform consistency

---

## Dependencies

### Core

| Dependency | Purpose |
|------------|---------|
| `ratatui` | Terminal UI rendering |
| `crossterm` | Terminal I/O (native) |
| `tokio` | Async runtime |
| `serde`/`serde_json` | Serialization |
| `anyhow`/`thiserror` | Error handling |
| `tracing`/`tracing-subscriber` | Logging |

### Language Support

| Dependency | Purpose |
|------------|---------|
| `tree-sitter` | AST parsing |
| `syntect` | Syntax highlighting |
| `lsp-types` | LSP protocol types |
| `oxc_*` | TypeScript parsing |

### Plugin System

| Dependency | Purpose |
|------------|---------|
| `rquickjs` | QuickJS JavaScript runtime |
| `rquickjs-serde` | JS-Rust serialization |
| `ts-rs` | TypeScript type generation |

---

## Installation Methods

| Platform | Method |
|----------|--------|
| macOS | `brew install fresh-editor` |
| Windows | `winget install fresh-editor` |
| Arch Linux | AUR: `fresh-editor-bin` |
| Debian/Ubuntu | `.deb` package |
| Fedora/RHEL | `.rpm` package |
| Nix | `nix run github:sinelaw/fresh` |
| npm | `npm install -g @fresh-editor/fresh-editor` |
| Cargo | `cargo install fresh-editor` |

---

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Open 1GB file | ~200ms | Lazy loading |
| Insert at cursor | O(log n) | Piece tree |
| Multi-cursor edit | O(m * log n) | m = cursors |
| Syntax highlight | O(lines) | Syntect streaming |
| LSP completion | ~50-100ms | Network + parsing |
| Plugin load | ~10-50ms | QuickJS init |

---

## Related Documentation

- [WASM Compatibility](./wasm-web-editor-analysis.md) - Web browser support analysis
- [Plugin Development](fresh-plugins-exploration.md) - Plugin system deep dive
- [Rust Revision](rust-revision.md) - Rust reproduction guide
