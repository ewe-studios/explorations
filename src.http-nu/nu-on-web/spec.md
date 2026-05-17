# nu-on-web — Spec

## Source

- **Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.http-nu/nu-on-web/`
- **Language:** Rust (WASM32 target)
- **Package:** `nushell-wasm` v0.1.0
- **Author:** Itay Sin Malia
- **License:** Not specified (part of http-nu exploration)
- **10 source files, ~400 LOC**

## What It Is

A WASM compilation of Nushell that runs in the browser, providing a REPL-style interface where Nushell commands (`ls`, `cat`, `rm`) operate on a ZenFS virtual filesystem. Custom commands bridge Rust Nushell internals to JavaScript `@zenfs/core` via `wasm-bindgen` FFI. TypeScript-compatible types are generated via `tsify`.

## Documentation Goal

1. Understand how Nushell is bootstrapped in WASM
2. Understand the Engine wrapper (parse → run → completion pipeline)
3. Understand the 3 custom commands (ls, cat, rm) and their ZenFS bridge
4. Understand the TypeScript type generation via Tsify
5. Understand the JS-Rust FFI boundary (wasm-bindgen module imports)

## Structure

```
nu-on-web/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-wasm-engine.md
│   └── 02-commands-zenfs.md
├── html/
│   └── ... (generated)
```

## Tasks

| Task | Status |
|------|--------|
| Read all 10 source files | DONE |
| Write spec.md | DONE |
| Write 00-overview.md | DONE |
| Write 01-wasm-engine.md | DONE |
| Write 02-commands-zenfs.md | DONE |
| Write README.md | DONE |
| Generate HTML via build.py | DONE |
| Grandfather review | DONE |

## Build System

```bash
python3 build.py src.http-nu/nu-on-web
```

## Quality Requirements

Per documentation_directive.md — minimum 2 mermaid diagrams, 3 code snippets with file paths, 1 Aha moment per document. All names/numbers/flows verified against source.

## Expected Outcome

A reader can understand how to embed Nushell in a browser, create custom WASM commands that bridge to JavaScript APIs, and generate TypeScript types from Rust.

## Resume Point

All files read, spec written. Need to write 3 markdown docs + README + build + review.
