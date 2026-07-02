---
title: "Astro Compiler Documentation — Project Spec"
---

# Astro Compiler — Project Spec

## Source Codebase

- **Go/TypeScript compiler:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Astro/compiler/`
  - Language: Go 1.21, TypeScript (wrapper package `@astrojs/compiler` 4.0.0)
  - License: MIT
  - Repository: https://github.com/withastro/compiler
  - Build: `make wasm` produces `packages/compiler/wasm/astro.wasm`
- **Rust compiler:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Astro/compiler-rs/`
  - Language: Rust 1.91.0, edition 2024
  - Workspace: `astro_napi` (NAPI bindings), `astro_codegen` (codegen + printer)
  - Parser: oxc fork at `github.com/withastro/oxc` rev `c429ef28`
  - License: MIT

## What the Project Is

The Astro compiler transforms `.astro` component files — a hybrid of TypeScript frontmatter and JSX-like templates — into server-side JavaScript that the Astro runtime executes to produce HTML. It exists in two parallel implementations: an original Go compiler that compiles to WebAssembly for Node.js consumption, and a Rust rewrite built on the `oxc` parser infrastructure with NAPI bindings for native-speed compilation. Both produce identical JavaScript output targeting `astro/runtime/server/index.js`.

## Documentation Goal

1. How the two-phase pipeline works: parse (tokenizer + parser) then transform (scanner + printer)
2. How the Go tokenizer handles Astro-specific tokens (frontmatter fences, expressions, raw text elements)
3. How the Go parser implements HTML5 insertion modes with Astro extensions (frontmatter, expressions, components)
4. How the Rust compiler uses `oxc` for parsing with an `AstroRoot` AST
5. How NAPI bindings expose the Rust compiler to Node.js (sync + async APIs, type mapping)
6. How the code printer generates JavaScript from the AST (runtime imports, component wrappers, template literals)
7. How AST transforms handle CSS scoping, hydration directives, and script hoisting
8. The `.astro` syntax specification — frontmatter, template, style/script blocks
9. Cross-cutting concerns: sourcemaps, whitespace collapsing, error handling, performance

## Documentation Structure

```
[src.Astro]/compiler/
├── spec.md                          ← This file
├── markdown/
│   ├── README.md                    ← Index / table of contents
│   ├── 00-overview.md              ← What the compilers are, dual-impl architecture
│   ├── 01-go-parser.md             ← Go tokenizer + parser deep dive
│   ├── 02-go-printer.md            ← Go print-to-source + WASM bridge
│   ├── 03-rust-parser.md           ← oxc-based Rust parser + AST
│   ├── 04-napi-bindings.md         ← NAPI sync/async APIs, type mapping
│   ├── 05-ast-transforms.md        ← Scanner pass, hydration, CSS scoping
│   ├── 06-code-generation.md       ← Printer pass, JS output format
│   ├── 07-compiler-api.md          ← @astrojs/compiler package API
│   ├── 08-syntax-spec.md           ← .astro file format specification
│   └── 09-cross-cutting.md         ← Sourcemaps, whitespace, errors, perf
├── html/                            ← Generated HTML
└── build.py                         ← Shared Markdown → HTML script
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | spec.md | DONE |
| 1 | markdown/README.md | DONE |
| 2 | markdown/00-overview.md | DONE |
| 3 | markdown/01-go-parser.md | DONE |
| 4 | markdown/02-go-printer.md | DONE |
| 5 | markdown/03-rust-parser.md | DONE |
| 6 | markdown/04-napi-bindings.md | DONE |
| 7 | markdown/05-ast-transforms.md | DONE |
| 8 | markdown/06-code-generation.md | DONE |
| 9 | markdown/07-compiler-api.md | DONE |
| 10 | markdown/08-syntax-spec.md | DONE |
| 11 | markdown/09-cross-cutting.md | DONE |
| 12 | HTML build | DONE |
| 13 | Grandfather Review | DONE |

## Build System

```bash
cd "[src.Astro]/compiler" && python3 build.py
```

Dependencies: Python 3.12+ stdlib only.

## Quality Requirements

All ten Iron Rules apply. See `documentation_directive.md`.

## Expected Outcome

A developer can read/modify the Go tokenizer and parser, extend the Rust codegen printer, add NAPI bindings, debug compilation errors, and write custom AST transforms.

## Resume Point

Check spec.md Tasks table for TODO items. Write next numbered document in order.
