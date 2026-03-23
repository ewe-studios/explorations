# Fresh Plugins System Exploration

## Overview

The Fresh editor has a powerful TypeScript-based plugin system that allows extending the editor with custom functionality. This document explores both the built-in plugins and the external plugin ecosystem.

---

## Plugin Architecture

### Runtime Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Fresh Editor                           │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │   Main Thread   │    │      Plugin Thread (isolated)   │ │
│  │   (UI + Event)  │───▶│  ┌───────────────────────────┐  │ │
│  │                 │◀───│  │    QuickJS Runtime        │  │ │
│  │  - Render loop  │    │  │  - TypeScript transpiled  │  │ │
│  │  - Input handling│   │  │  - Plugin sandbox         │  │ │
│  │  - Buffer state │    │  │  - Hook handlers          │  │ │
│  └─────────────────┘    │  └───────────────────────────┘  │ │
│                         └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
         │                               │
         │ PluginCommand channel         │ Async operations
         ▼                               ▼
┌─────────────────┐             ┌─────────────────┐
│  Action Handler │             │  Process Spawner│
│  (Rust side)    │             │  (git, etc.)    │
└─────────────────┘             └─────────────────┘
```

### Plugin API Types

The plugin API is auto-generated from Rust types using `ts-rs`:

```typescript
// fresh.d.ts - Auto-generated type definitions
interface EditorAPI {
    // Buffer operations
    buffers(): BufferInfo[];
    bufferContent(id: BufferId): string | null;
    insert(bufferId: BufferId, offset: number, text: string): void;
    delete(bufferId: BufferId, start: number, end: number): void;

    // Cursor operations
    cursors(): CursorState[];

    // Viewport
    viewport(): ViewportInfo;

    // UI
    statusMessage(msg: string): void;
    showPrompt(prompt: string): Promise<string | null>;

    // Decorations
    addOverlay(bufferId: BufferId, overlay: Overlay): void;

    // LSP
    gotoDefinition(): void;
    showHover(): void;
}

// Hook system
interface Hooks {
    'buffer:modified': (buffer: BufferInfo) => void;
    'buffer:saved': (buffer: BufferInfo) => void;
    'cursor:moved': (cursor: CursorState) => void;
    'key:press': (key: KeyEvent) => boolean;
}
```

---

## Built-in Plugins

### Location
`/home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/fresh/crates/fresh-editor/plugins/`

### Core Plugins

#### 1. Git Integration

| Plugin | Description | Lines |
|--------|-------------|-------|
| `git_gutter.ts` | Inline diff indicators | 12.7K |
| `git_log.ts` | Interactive git log viewer | 38.2K |
| `git_blame.ts` | Inline blame annotations | 20.6K |
| `git_explorer.ts` | File tree with git status | 4.5K |
| `git_find_file.ts` | Fuzzy file finder with git | 1.8K |
| `git_grep.ts` | Search across git repo | 1.7K |

**Example: Git Gutter**
```typescript
import { getEditor } from './lib/fresh';

const editor = getEditor();

editor.on('buffer:saved', (buffer) => {
    const diff = git.diff(buffer.path);
    editor.addOverlay(buffer.id, {
        type: 'gutter',
        signs: diff.hunks.map(h => ({
            line: h.new_start,
            type: h.type // 'add' | 'remove' | 'modify'
        }))
    });
});
```

#### 2. Language Server Protocol (LSP)

| Plugin | Language | Description |
|--------|----------|-------------|
| `clangd-lsp.ts` | C/C++ | Clangd server integration |
| `typescript.ts` | TypeScript | TypeScript language server |
| `python-lsp.ts` | Python | Pyright/Pylance integration |
| `go-lsp.ts` | Go | gopls integration |
| `rust-lsp.ts` | Rust | rust-analyzer integration |
| `java-lsp.ts` | Java | JDT LS integration |
| `css-lsp.ts` | CSS | CSS language server |
| `html-lsp.ts` | HTML | HTML language server |
| `json-lsp.ts` | JSON | JSON language server |
| `latex-lsp.ts` | LaTeX | latex-ls integration |

**LSP Features**:
- Go to definition
- Find references
- Hover documentation
- Diagnostics
- Code actions
- Autocompletion
- Rename symbol

#### 3. Productivity Plugins

| Plugin | Description |
|--------|-------------|
| `code-tour.ts` | Interactive code walkthroughs |
| `diagnostics_panel.ts` | LSP diagnostics panel |
| `find_references.ts` | Find references quick panel |
| `audit_mode.ts` | Code audit/review mode |
| `buffer_modified.ts` | Modified buffer indicators |

#### 4. Utility Plugins

| Plugin | Description |
|--------|-------------|
| `emmet.ts` | Emmet abbreviation expansion |
| `color-highlighter.ts` | Color code preview |
| `todo-highlighter.ts` | TODO/FIXME highlighting |
| `calculator.ts` | In-editor calculator |

### Plugin Library

`plugins/lib/` provides shared utilities:

```typescript
// finder.ts - Fuzzy finding utility
export function fuzzyMatch(pattern: string, text: string): Match | null;

// navigation-controller.ts - Navigation history
export class NavController {
    push(location: Location): void;
    pop(): Location | null;
}

// panel-manager.ts - UI panel management
export class PanelManager {
    create(name: string, config: PanelConfig): Panel;
    destroy(name: string): void;
}

// fresh.d.ts - Type definitions
export * from './fresh';
```

---

## Plugin Development

### Plugin Structure

```typescript
// example-plugin.ts
import { getEditor } from './lib/fresh';

const editor = getEditor();

// Plugin metadata
export const name = 'example-plugin';
export const version = '1.0.0';

// Commands
export const commands = {
    'example:hello': () => {
        editor.statusMessage('Hello from example plugin!');
    }
};

// Hooks
export const hooks = {
    'buffer:modified': (buffer) => {
        console.log('Buffer modified:', buffer.path);
    }
};

// Keybindings
export const keybindings = {
    'Ctrl-Alt-H': 'example:hello'
};
```

### Plugin API Reference

#### Buffer Operations

```typescript
// Get all buffers
const buffers = editor.buffers();

// Get buffer content
const content = editor.bufferContent(bufferId);
const lines = editor.bufferLines(bufferId, start, end);

// Modify content
editor.insert(bufferId, offset, 'new text');
editor.delete(bufferId, startOffset, endOffset);
editor.replace(bufferId, range, 'replacement');

// Save/close
editor.save(bufferId);
editor.close(bufferId);
```

#### Cursor Operations

```typescript
// Get cursor state
const cursors = editor.cursors();
const mainCursor = cursors[0];

// Set cursor position
editor.setCursor(bufferId, offset);
editor.addCursor(bufferId, offset);
```

#### Viewport Operations

```typescript
// Get visible area
const viewport = editor.viewport();
const visibleRange = {
    start: viewport.topByte,
    end: viewport.topByte + viewport.width * viewport.height
};

// Scroll
editor.scrollTo(bufferId, line);
editor.centerCursor(bufferId);
```

#### Decoration Operations

```typescript
// Add overlay/highlight
editor.addOverlay(bufferId, {
    type: 'highlight',
    ranges: [{ start: 0, end: 10, style: { bg: 'red' } }]
});

// Add gutter sign
editor.addGutterSign(bufferId, line, {
    type: 'error',
    text: '!'
});

// Clear decorations
editor.clearDecorations(bufferId);
```

#### UI Operations

```typescript
// Status message
editor.statusMessage('Loading...');
editor.errorMessage('Failed!');

// Prompts
const input = await editor.prompt('Enter value:');
const confirm = await editor.confirm('Are you sure?');

// Quick panel
const choice = await editor.showQuickPanel([
    { label: 'Option 1', value: 'opt1' },
    { label: 'Option 2', value: 'opt2' }
]);
```

#### LSP Operations

```typescript
// Navigation
await editor.gotoDefinition();
await editor.gotoReferences();
await editor.gotoImplementation();

// Information
const hover = await editor.showHover();
const signature = await editor.showSignatureHelp();

// Actions
const actions = await editor.getCodeActions();
await editor.renameSymbol();
```

---

## Plugin Examples

### Hello World

```typescript
// plugins/examples/hello_world.ts
import { getEditor } from '../lib/fresh';

const editor = getEditor();

export const name = 'hello-world';

export const commands = {
    'hello:greet': () => {
        editor.statusMessage('Hello, World!');
    }
};

export const keybindings = {
    'Ctrl-Alt-G': 'hello:greet'
};
```

### Buffer Watcher

```typescript
// plugins/examples/buffer_modified.ts
import { getEditor } from '../lib/fresh';

const editor = getEditor();

export const name = 'buffer-watcher';

// Track all buffer modifications
const modifiedBuffers = new Set<number>();

export const hooks = {
    'buffer:modified': (buffer) => {
        modifiedBuffers.add(buffer.id);
        console.log(`Buffer ${buffer.path} modified`);
    },

    'buffer:saved': (buffer) => {
        modifiedBuffers.delete(buffer.id);
        console.log(`Buffer ${buffer.path} saved`);
    }
};
```

### Async Demo

```typescript
// plugins/examples/async_demo.ts
import { getEditor } from '../lib/fresh';

const editor = getEditor();

export const name = 'async-demo';

export const commands = {
    'async:fetch': async () => {
        const url = await editor.prompt('Enter URL:');
        if (!url) return;

        const handle = await editor.spawnProcess('curl', [url]);
        const result = await handle.result;

        if (result.exit_code === 0) {
            editor.statusMessage(`Fetched ${result.stdout.length} bytes`);
        } else {
            editor.errorMessage(`Failed: ${result.stderr}`);
        }
    }
};
```

### Virtual Buffer Demo

```typescript
// plugins/examples/virtual_buffer_demo.ts
import { getEditor } from '../lib/fresh';

const editor = getEditor();

export const name = 'virtual-buffer-demo';

export const commands = {
    'demo:virtual': () => {
        // Create a virtual (non-file) buffer
        const bufferId = editor.createBuffer({
            name: 'Demo Buffer',
            mode: 'plaintext',
            content: 'This is a virtual buffer\nIt has no file on disk'
        });

        editor.focusBuffer(bufferId);
    }
};
```

---

## Plugin Configuration

### Plugin Config Schema

```json
{
  "$schema": "https://getfresh.dev/schemas/plugin-config.json",
  "plugins": {
    "git-gutter": {
      "enabled": true,
      "settings": {
        "show_added": true,
        "show_modified": true,
        "show_removed": true,
        "colors": {
          "added": "#22c55e",
          "modified": "#fbbf24",
          "removed": "#ef4444"
        }
      }
    }
  }
}
```

### Discoverability

Plugins are discovered in:
- `~/.config/fresh/plugins/` - User plugins
- `/usr/share/fresh-editor/plugins/` - System plugins
- Embedded plugins (compiled into binary)

---

## Plugin Registry

The plugin registry (`/fresh-plugins-registry/plugins.json`) provides metadata:

```json
{
  "packages": {
    "calculator": {
      "description": "In-editor calculator with expression evaluation",
      "repository": "https://github.com/sinelaw/fresh-plugins#calculator",
      "author": "Fresh Editor Team",
      "license": "MIT",
      "keywords": ["calculator", "math", "utility"],
      "latest_version": "1.0.0",
      "fresh_min_version": "0.1.0"
    }
  }
}
```

---

## Plugin Security

### Sandboxing

Plugins run in a sandboxed QuickJS environment:
- No direct filesystem access
- No network access (except via `spawnProcess`)
- No access to Rust internals
- API access limited to exposed editor methods

### Blocklist

`blocklist.json` contains malicious/banned plugins:

```json
{
  "blocked_plugins": [
    {
      "name": "malicious-plugin",
      "reason": "Attempts to access filesystem directly",
      "date_added": "2024-01-01"
    }
  ]
}
```

---

## Plugin Testing

### Test Utilities

```typescript
// plugins/lib/test.ts
export function createTestEditor(): TestEditor;
export function mockBuffer(content: string): MockBuffer;
export function assertEqual<T>(actual: T, expected: T): void;
```

### Example Test

```typescript
// plugins/__tests__/hello_world.test.ts
import { test } from '../lib/test';
import { commands } from '../hello_world';

test('hello:greet command', () => {
    const editor = createTestEditor();
    commands['hello:greet'](editor);
    assertEqual(editor.lastStatusMessage, 'Hello, World!');
});
```

---

## Future Plugin Directions

### WASM Plugins

Potential future support for WASM plugins:
```rust
// Hypothetical WASM plugin API
#[wasm_bindgen]
pub struct WasmPlugin {
    name: String,
    version: String,
}

#[wasm_bindgen]
impl WasmPlugin {
    pub fn on_buffer_modified(&mut self, buffer: JsValue);
    pub fn handle_command(&mut self, cmd: String) -> JsValue;
}
```

### Language-Specific SDKs

TypeScript SDK is primary, but future support for:
- Python plugins (via PyO3)
- Lua plugins (via mlua)
- Rust plugins (native dynamic libraries)

---

## Related Documents

- [Fresh Editor](fresh-exploration.md) - Main editor exploration
- [Plugin Registry](fresh-plugins-registry-exploration.md) - Registry structure
- [Rust Revision](rust-revision.md) - Rust reproduction guide
