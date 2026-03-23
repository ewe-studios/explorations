---
name: Supporting Projects
description: Miscellaneous supporting projects including jsast, irq_safety, uX, glmeshdraw, documentation sites, and resource directories
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/
repository: Various
explored_at: 2026-03-23
language: Rust, JavaScript, HTML, Markdown
---

# Supporting Projects

## Overview

This document covers the smaller supporting projects in the Makepad ecosystem that don't warrant individual deep-dive documents. These range from JavaScript parsers to interrupt-safe primitives, non-standard integer types, documentation sites, and resource directories.

---

## 1. jsast - JavaScript AST Parser

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/jsast/`

A JavaScript parser based on Acorn (by Marijn Haverbeke), consisting of JavaScript source files that implement a complete ECMAScript parser. This is a pure-JavaScript project (not Rust).

### Structure
```
jsast/
├── LICENSE
├── README.md
├── jsparser.js             # Main parser interface
├── jstokenize.js           # Tokenizer
├── jstokentype.js          # Token type definitions
├── jstokencontext.js       # Token context tracking
├── jsexpression.js         # Expression parsing
├── jsstatement.js          # Statement parsing
├── jsdefinitions.js        # Definition parsing
├── jsidentifier.js         # Identifier handling
├── jsnode.js               # AST node types
├── jslval.js               # L-value handling
├── jsformat.js             # Code formatting
├── jstokenformat.js        # Token formatting
├── jsoptions.js            # Parser options
├── jsstate.js              # Parser state
├── jsparseutil.js          # Parser utilities
├── jslocutil.js            # Location utilities
├── jslocation.js           # Source location tracking
├── jswalk.js               # AST walker
├── jswhitespace.js         # Whitespace handling
└── jscomments.js           # Comment handling
```

### Purpose
Used in early Makepad development for JavaScript tooling -- likely for the Makepad Studio IDE's JavaScript editing support or for transpilation purposes.

---

## 2. irq_safety - Interrupt-Safe Primitives

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/irq_safety/`
**Repository:** https://crates.io/crates/irq_safety

A `no_std` crate providing interrupt-safe Mutex and RwLock implementations. Holds interrupts disabled for the duration of the lock guard, re-enabling them on drop only if they were previously enabled.

### Structure
```
irq_safety/
├── Cargo.toml              # v0.1.1
├── README.md
└── src/
    ├── lib.rs              # Re-exports
    ├── mutex_irqsafe.rs    # IRQ-safe Mutex
    ├── rwlock_irqsafe.rs   # IRQ-safe RwLock
    └── held_interrupts.rs  # HeldInterrupts type
```

### Key Details
- Built on top of the `spin` crate (spinlock-based synchronization)
- Supported architectures: x86, x86_64, aarch64, arm
- Used in OS kernel development and embedded contexts
- Created by Kevin Boos (Project Robius contributor), used in the Theseus OS

---

## 3. uX - Non-Standard Integer Types

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/uX/`
**Repository:** https://github.com/kjetilkjeka/uX

Provides non-standard integer types like `u2`, `u3`, `u7`, `u9`, `u10`, etc. (all sizes from 1-127 bits for both signed and unsigned). Useful in embedded and protocol parsing contexts where bit-level precision matters.

### Key Details
- Types use the smallest containing standard integer for storage
- Overflow panics in debug, wraps in release (matching standard integer behavior)
- Supports `From`/`TryFrom` for lossless conversions
- `no_std` compatible with optional `std` feature for Error trait
- Categories: embedded, no-std, data-structures

---

## 4. glmeshdraw - OpenGL Mesh Drawing

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/glmeshdraw/`

A minimal OpenGL mesh drawing experiment. Single source file demonstrating basic OpenGL rendering with glutin.

### Structure
```
glmeshdraw/
├── Cargo.toml              # Package: glwindow v0.1.0
└── src/
    └── main.rs             # Single-file OpenGL demo
```

### Dependencies
glutin, gl, rand, scoped_threadpool

---

## 5. Documentation & Website Projects

### makepad_docs
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/makepad_docs/`

Historical Makepad documentation including whitepapers:
- `Makepad Whitepaper 2020.pdf` - Original Makepad vision document
- `live_language_whitepaper_06052022.pdf` - Live Design language specification

### makepad.github.io
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/makepad.github.io/`

The Makepad GitHub Pages website. Contains compiled Makepad examples that run in the browser via WASM. Includes a Cargo.toml for building examples, HTML/JS loader code, and font encoding tools.

### makepad_history
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/makepad_history/`

Historical version of the Makepad repository, preserved as reference. Includes build scripts, examples, and the earlier rendering architecture.

### book (Robius Book)
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/book/`

The Robius Book - documentation on the Robius project for multi-platform application development in Rust. Built with mdBook.

### robius.rs
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/robius.rs/`

Official Project Robius website (https://robius.rs). Built with Zola static site generator with Sass styling and Tera templates.

---

## 6. Resource Directories

### files
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/files/`

Presentation files and APKs from conferences:
- GOSIM China 2023
- GOSIM China 2024
- RustNL 2024 and GOSIM Europe 2024
- Makepad Performance Benchmarking.pdf

### fonts
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/fonts/`

Font resources used across Makepad projects.

### boiler
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/boiler/`

A Node.js boilerplate project with browser and server code. Likely used for web development tooling around Makepad's WASM deployment.

```
boiler/
├── index.html
├── browsercode.js
├── browserloader.js
├── server.js
├── servercode.js
├── serversocket.js
└── favicon.ico
```

### wasm-index
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/wasm-index/`

A simple HTML index page with Robius branding for WASM deployments.

### ai_mr, ai_ui, ai_xr
These directories contain only LICENSE files -- placeholder projects for future AI + Mixed Reality, AI + UI, and AI + XR explorations.

---

## Key Insights

- The supporting projects reveal the breadth of the ecosystem: from OS-level kernel primitives (irq_safety) to web infrastructure (boiler, wasm-index)
- jsast is a JavaScript-only project in an otherwise Rust-dominated collection, suggesting early web IDE aspirations
- irq_safety and uX are general-purpose crates that happen to be maintained by Robius contributors
- The documentation projects span multiple generations: whitepapers from 2020, GitHub Pages, mdBook, and Zola
- Several placeholder directories (ai_mr, ai_ui, ai_xr) suggest planned but not-yet-started work
- Conference presentations in the files/ directory show the project's community engagement
