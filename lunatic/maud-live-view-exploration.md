---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.lunatic/maud_live_view
repository: https://github.com/lunatic-solutions/maud (fork)
explored_at: 2026-03-23T00:00:00Z
language: Rust
---

# Project Exploration: maud_live_view

## Overview

`maud_live_view` is a fork of the Maud HTML template engine (https://maud.lambda.xyz/) adapted for use with lunatic's `submillisecond-live-view` framework. Maud is a compile-time HTML template library that uses a Rust macro (`html!`) to generate HTML at compile time, producing highly efficient pre-rendered markup.

The fork modifies Maud to support LiveView's requirements for diffable HTML output, enabling server-side rendering with efficient WebSocket-based DOM updates.

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.lunatic/maud_live_view`
- **Remote:** Originally `https://github.com/lambda-fairy/maud`, forked by lunatic-solutions
- **Primary Language:** Rust
- **License:** MIT / Apache-2.0

## Directory Structure

```
maud_live_view/
  Cargo.toml                # Workspace root
  README.md                 # Original Maud README
  CHANGELOG.md
  CONTRIBUTING.md
  RELEASE_PROCESS.md
  rustfmt.toml
  maud.png                  # Logo
  maud/
    Cargo.toml              # maud v0.24.0
    src/
      lib.rs                # Core Maud types (Markup, PreEscaped, Render)
      escape.rs             # HTML escaping
  maud_macros/
    Cargo.toml              # maud_macros v0.24.0 (proc-macro)
    src/
      lib.rs                # Proc macro entry point
      ast.rs                # Macro AST types
      parse.rs              # Macro syntax parser
      generate.rs           # Code generation
      escape.rs             # Compile-time escaping
  docs/                     # Documentation source
  doctest/                  # Documentation tests
```

## Architecture

### How Maud Works

Maud provides the `html!` macro that compiles HTML templates to Rust code at compile time:

```rust
use maud::html;

let markup = html! {
    h1 { "Hello, World!" }
    p.intro {
        "Welcome to "
        a href="https://lunatic.solutions" { "Lunatic" }
    }
};
// markup is a pre-rendered String -- no runtime template parsing
```

### Workspace Structure

1. **maud** (library crate): Core types:
   - `Markup` - A pre-rendered HTML string (wrapper around `String`)
   - `PreEscaped<T>` - Marks content as already escaped
   - `Render` trait - Allows custom types to be rendered into Maud templates
   - `DOCTYPE` constant - HTML5 doctype string
   - HTML escaping utilities
   - Optional integrations: `actix-web`, `axum`, `rocket`, `tide`

2. **maud_macros** (proc-macro crate): The `html!` macro implementation:
   - `parse.rs` - Parses the Maud DSL syntax into an AST
   - `ast.rs` - Abstract syntax tree types for Maud templates
   - `generate.rs` - Generates Rust code from the AST (string concatenation)
   - `escape.rs` - Compile-time HTML entity escaping

### Fork Modifications

The key modifications in this fork (compared to upstream Maud) are designed to support LiveView:

- The generated HTML can be split into static and dynamic parts, enabling efficient diffing
- Dynamic values (variables, expressions) are tracked separately from static template structure
- This allows `submillisecond-live-view` to compute minimal diffs when state changes, sending only changed dynamic parts over WebSocket

## Dependencies

### maud
| Crate | Version | Purpose |
|-------|---------|---------|
| maud_macros | 0.24.0 | Proc macro for html! |
| itoa | 1 | Integer-to-string conversion |
| rocket (optional) | >= 0.3, < 0.5 | Rocket integration |
| actix-web (optional) | 4 | Actix integration |
| axum-core (optional) | 0.2 | Axum integration |
| tide (optional) | 0.16.0 | Tide integration |

### maud_macros
Uses only `proc-macro2`, `quote`, and `syn`-like parsing utilities (built-in).

## Ecosystem Role

This fork is a dependency of `submillisecond-live-view`. The LiveView framework needs a template engine that can:
1. Efficiently render HTML on the server
2. Track which parts of the template are static vs. dynamic
3. Compute minimal diffs when state changes
4. Send only the changed parts over WebSocket

Maud's compile-time approach is ideal because the template structure is fully known at compile time, making it possible to separate static scaffolding from dynamic values. The fork adds the necessary hooks for `submillisecond-live-view` to leverage this.

Standard Maud (upstream) renders to a flat `String`, which would require full-page diffing. This fork preserves the structural information needed for targeted updates.
