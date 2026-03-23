# codingIDE Project Exploration

## Overview

The codingIDE project located at `/home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE` is a collection of 5 sub-projects focused on code editing, IDE functionality, and WebAssembly applications:

| Sub-project | Description | Language | WASM Support |
|-------------|-------------|----------|--------------|
| **fresh** | Terminal-based text editor with LSP and TypeScript plugins | Rust + TypeScript | Yes (in-progress) |
| **fresh-plugins** | Language packs and plugins for fresh editor | TypeScript + Rust | N/A (editor plugins) |
| **fresh-plugins-registry** | Plugin registry/metadata for fresh | JSON schemas | N/A |
| **radon-ide** | React Native IDE (VSCode extension) | TypeScript | No (VSCode only) |
| **rockies** | 2D physics sandbox game | Rust | Yes (production-ready) |

---

## Project Summaries

### 1. Fresh Editor (`/fresh/`)

**A modern, full-featured terminal text editor with zero configuration.**

- **Version**: 0.2.3
- **License**: GPL-2.0
- **Author**: Noam Lewis
- **Website**: https://getfresh.dev

#### Key Features
- Familiar keybindings (VSCode/Sublime-like), mouse support, menus, command palette
- Multi-gigabyte file handling with piece-tree data structure
- LSP integration (go-to-definition, hover, diagnostics, autocomplete)
- TypeScript plugin system (Deno runtime, sandboxed)
- Syntax highlighting for 100+ languages via Syntect/TextMate grammars
- Themes, multi-cursor editing, split views, integrated terminal
- Internationalization support

#### Architecture
```
fresh/
├── crates/
│   ├── fresh-core/          # Core types (CursorId, BufferId, events, API)
│   ├── fresh-editor/        # Main editor application
│   ├── fresh-parser-js/     # JavaScript/TypeScript parser (oxc-based)
│   ├── fresh-languages/     # Tree-sitter language bindings
│   ├── fresh-plugin-runtime/# TypeScript plugin runtime (QuickJS)
│   └── fresh-plugin-api-macros/ # Proc macros for plugin API
├── plugins/                 # TypeScript plugins (git, LSP, etc.)
├── themes/                  # Color themes
└── docs/                    # Documentation including WASM guide
```

#### WASM Status
- Model layer: 100% WASM-compatible (buffer, piece-tree, cursors, events)
- Primitives layer: 100% WASM-compatible (syntax highlighting, indentation)
- Input layer: Needs abstraction (crossterm types)
- View layer: ~70% WASM-ready (ratatui is WASM-compatible, needs Ratzilla backend)
- Services layer: ~13% WASM-ready (gate tokio/LSP/PTY)
- App layer: Needs full refactoring

See `wasm-web-editor-analysis.md` for detailed WASM analysis.

---

### 2. Fresh Plugins (`/fresh-plugins/`)

**Language packs and standalone plugins for the Fresh editor.**

#### Sub-projects
| Plugin | Description |
|--------|-------------|
| **languages/** | Language-specific plugins (elixir, hare, solidity, templ, zenc) |
| **amp/** | AI coding agent integration |
| **calculator/** | In-editor expression evaluation |
| **color-highlighter/** | Visual color code preview (hex, rgb, hsl) |
| **csv/** | CSV file handling |
| **emmet/** | Emmet abbreviation expansion |
| **spellcheck/** | Spell checking |
| **themes/** | Additional themes |
| **todo-highlighter/** | TODO/FIXME/HACK annotation highlighting |

---

### 3. Fresh Plugins Registry (`/fresh-plugins-registry/`)

**Centralized plugin registry metadata.**

Contains:
- `plugins.json` - Package metadata (description, repository, license, keywords)
- `languages.json` - Language pack registry
- `themes.json` - Theme registry
- `blocklist.json` - Blocked/malicious plugins
- JSON schemas for validation

---

### 4. Radon IDE (`/radon-ide/`)

**React Native IDE extension for VSCode/Cursor.**

- **Website**: https://ide.swmansion.com
- **License**: Commercial (paid)
- **Publisher**: Software Mansion

#### Features
- Element inspector with component hierarchy
- Integrated debugger with source code
- Logging console with jump-to-source
- Device settings (theme, text size, location, language)
- Screen recording and replays
- Component preview functionality
- Works with React Native and Expo projects

**Note**: This is a VSCode extension marketplace product, not a standalone Rust project. The directory contains mostly documentation and issue tracking.

---

### 5. Rockies (`/rockies/`)

**A 2D pixel-based sandbox physics game written in Rust and WebAssembly.**

- **License**: GPL-2.0-only
- **WASM Support**: Full production-ready WASM build

#### Features
- Basic physics: collision detection, gravity, inertia
- User interaction: click/drag objects, keyboard controls
- Procedural terrain generation (Perlin noise)
- Multi-grid universe with cell loading/unloading
- WebAssembly compilation for browser execution

#### Architecture
```
rockies/
├── src/
│   ├── lib.rs          # WASM bindings (Game struct)
│   ├── universe.rs     # Game universe/cell management
│   ├── grid.rs         # Grid data structures
│   ├── multigrid.rs    # Multi-grid coordinate system
│   ├── color.rs        # HSV color handling
│   ├── v2.rs           # 2D vector types (V2, V2i)
│   ├── inertia.rs      # Physics inertia/mass/velocity
│   ├── generator.rs    # Procedural generation
│   └── assets.rs       # Asset loading
├── www/                # Web frontend
└── Cargo.toml          # WASM build config
```

#### WASM Build
```bash
cargo build --target wasm32-unknown-unknown
# or
wasm-pack build
```

The `Game` struct is exposed to JavaScript via `wasm_bindgen`:
- `tick()` - Game loop iteration
- `pixels()` - Frame buffer pointer
- `key_down()`/`key_up()` - Input handling
- `click()` - Mouse interaction
- `load_grid()`/`save_grid()` - Persistence

---

## Cross-Project Analysis

### WASM/Web Capabilities Summary

| Project | WASM Ready | Use Case |
|---------|-----------|----------|
| fresh | Partial | Web-based code editor |
| fresh-plugins | N/A | Editor extensions |
| fresh-plugins-registry | N/A | Metadata only |
| radon-ide | No | VSCode desktop only |
| rockies | Yes | Browser-based game |

### Key Technologies Used

| Technology | Usage |
|------------|-------|
| **Rust** | Core logic for all projects |
| **WebAssembly** | rockies (full), fresh (partial) |
| **wasm-bindgen** | rockies WASM bindings |
| **ratatui** | fresh terminal UI (WASM-compatible) |
| **crossterm** | fresh terminal I/O (needs abstraction) |
| **Syntect** | fresh syntax highlighting (WASM with fancy-regex) |
| **tree-sitter** | fresh language parsing (runtime-only) |
| **QuickJS** | fresh TypeScript plugin runtime |
| **oxc** | fresh TypeScript/JavaScript parsing |
| **tokio** | fresh async runtime (needs gating) |
| **LSP** | fresh language server protocol |
| **sdl2** | rockies terminal rendering (optional) |

---

## Recommendations for Web Editor Use Case

### Using Fresh for Web Editor

1. **Complete WASM refactor** (estimated 3-4 weeks):
   - Phase 1: Input abstraction ( KeyCode, KeyEvent types)
   - Phase 2: Add Ratzilla backend for rendering
   - Phase 3: Gate runtime services (LSP, PTY, tokio)
   - Phase 4: Integration testing

2. **Advantages**:
   - Already has 100% WASM-compatible core (model + primitives)
   - Syntect syntax highlighting works in WASM
   - Plugin system could enable web extensions
   - Production-quality buffer management (piece-tree)

3. **Challenges**:
   - LSP requires server-side component (cannot run in browser)
   - Plugin system uses QuickJS (need WASM JS runtime or wasm-bindgen)
   - Terminal emulator features incompatible with browsers

### Using Rockies as Reference

Rockies demonstrates a complete WASM workflow:
- `wasm-bindgen` for JS interop
- Pixel buffer rendering to canvas
- Keyboard/mouse event handling
- Game loop with `tick()` method

This pattern can be adapted for a web editor:
```rust
#[wasm_bindgen]
pub struct WebEditor {
    buffer: Buffer,
    // ...
}

#[wasm_bindgen]
impl WebEditor {
    pub fn render(&self) -> Vec<u32> { /* render frame */ }
    pub fn key_event(&mut self, key: String) { /* handle input */ }
    pub fn get_content(&self) -> String { /* export content */ }
}
```

---

## File Locations

- **Source**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/`
- **Exploration Output**: `/home/darkvoid/Boxxed/@dev/repo-expolorations/codingIDE/`

### Related Documents
- `fresh-exploration.md` - Deep dive into Fresh editor
- `fresh-plugins-exploration.md` - Plugin system analysis
- `fresh-plugins-registry-exploration.md` - Registry structure
- `radon-ide-exploration.md` - Radon IDE overview
- `rockies-exploration.md` - Rockies game deep dive
- `wasm-web-editor-analysis.md` - WASM/web editor feasibility
- `rust-revision.md` - Rust reproduction guide
