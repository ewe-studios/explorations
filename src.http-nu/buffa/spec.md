# buffa — Spec

## Source

- **Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.http-nu/src.connect-protocol/buffa/`
- **Language:** Rust (v0.5.2, MSRV 1.85)
- **Package:** `buffa` — pure-Rust Protocol Buffers implementation
- **166 Rust source files across 8 workspace crates**
- **Author:** Not specified (part of connect-protocol monorepo)
- **License:** Not specified

## What It Is

A pure-Rust Protocol Buffers implementation that passes the full conformance suite. Buffa's key differentiators from prost: two-pass serialization with SizeCache (O(n) vs prost's O(depth^2)), zero-copy borrowed views, editions-first design, and `no_std + alloc` support.

## Documentation Goal

1. Understand the core runtime (Message trait, encoding, views, SizeCache)
2. Understand the code generation pipeline (buffa-codegen, protoc-gen-buffa)
3. Understand the build integration (buffa-build fluent API)
4. Understand well-known types and editions support
5. Understand the two-pass serialization strategy vs prost

## Structure

```
buffa/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-core-runtime.md
│   ├── 02-views-zero-copy.md
│   └── 03-codegen-build.md
├── html/
│   └── ... (generated)
```

## Tasks

| Task | Status |
|------|--------|
| Read all 166 source files | DONE |
| Write spec.md | DONE |
| Write 00-overview.md | DONE |
| Write 01-core-runtime.md | DONE |
| Write 02-views-zero-copy.md | DONE |
| Write 03-codegen-build.md | DONE |
| Write README.md | DONE |
| Generate HTML via build.py | DONE |
| Grandfather review | DONE |

## Build System

```bash
python3 build.py src.http-nu/buffa
```

## Quality Requirements

Per documentation_directive.md — minimum 2 mermaid diagrams, 3 code snippets with file paths, 1 Aha moment per document. All names/numbers/flows verified against source.

## Expected Outcome

A reader can understand how Buffa implements Protocol Buffers in Rust, why its two-pass serialization is superior for deeply nested messages, and how the codegen pipeline generates both owned and zero-copy view types.

## Resume Point

Spec written. Need to write 4 markdown docs + README + build + review.
