# Ecosystem -- Nushell Integration Projects

## how-to-embed-nu

**Path**: `src.datastar/how-to-embed-nu/`  
**Language**: Rust  
**Structure**: Workspace with 4 crates

A progressive tutorial for embedding Nushell in Rust applications. Four phases of increasing complexity:

### Workspace Members

| Crate | Purpose |
|-------|---------|
| `p1-basic` | Minimal embedding: create engine, eval expression, get result |
| `p2-background` | Run Nushell on background threads (handling Send/Sync) |
| `p3-the-works` | Full integration: custom commands, plugins, env vars, modules |
| `p4-sandbox` | Sandboxed execution: restricted commands, resource limits |

### Why This Exists

Nushell's `EngineState` has specific threading requirements (not Send). This tutorial demonstrates the patterns used by xs, http-nu, and yoke for safe Nushell embedding:
- Dedicated OS threads for evaluation
- Engine state cloning
- Custom command registration
- Plugin loading
- Module VFS

## reedline

**Path**: `src.datastar/reedline/`  
**Repository**: https://github.com/nushell/reedline  
**Language**: Rust (65 source files)  
**Authors**: The Nushell Project Developers  
**License**: MIT

A readline-like crate for CLI text input. This is a fork/clone of the official Nushell reedline library. It provides:

### Core Features
- Multi-line editing with syntax highlighting
- History (file-backed, in-memory, or custom)
- Completions (tab completion with configurable strategies)
- Keybindings (vi mode, emacs mode)
- Hints (history-based autosuggestions)
- Menus (completion menu, history menu)
- Prompt customization
- Unicode support
- Cross-platform (Windows, macOS, Linux)

### Key Types

```rust
pub struct Reedline { /* ... */ }

pub trait Prompt {
    fn render_prompt_left(&self) -> Cow<str>;
    fn render_prompt_right(&self) -> Cow<str>;
    fn render_prompt_indicator(&self, mode: PromptEditMode) -> Cow<str>;
}

pub trait Completer {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion>;
}

pub trait Hinter {
    fn handle(&mut self, line: &str, pos: usize, history: &dyn History) -> String;
}
```

### Why It's Here

The Datastar ecosystem builds custom CLIs and REPLs (particularly for xs's `eval` command and interactive Nushell sessions). Having reedline as a dependency allows for rich interactive terminal experiences.

## stacks.nu

**Path**: `src.datastar/stacks.nu/`  
**Language**: Nushell (5 .nu files, 44 files total)

Nushell scripts for the "stacks" system. Provides user-facing commands for managing stacks — likely environment/service configurations that can be activated, deactivated, and composed.

This is the Nushell scripting layer on top of the `stacks` Rust binary, following the same pattern as `xs.nu` wrapping the `xs` binary.
