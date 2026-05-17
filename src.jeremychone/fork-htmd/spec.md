# fork-htmd — Documentation Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/fork-htmd/`
- **Crate name:** `htmd` (v0.2.1)
- **Language:** Rust (edition 2021)
- **License:** Apache-2.0
- **Author:** letmutex (original), forked by jeremychone
- **Files:** 17 source files, ~2.7K LOC (28 total including tests/benches)
- **Dependencies:** html5ever (HTML parsing), markup5ever_rcdom (DOM), html-escape (entity decoding)

## What the Project Is

fork-htmd is a Rust crate that converts HTML to Markdown. It uses `html5ever` to parse HTML into a DOM tree, then walks the tree depth-first, converting each element to its Markdown equivalent via a handler registry. The architecture is inspired by the JavaScript `turndown.js` library. It supports all common HTML elements (headings, code blocks, tables, lists, links, images, blockquotes, emphasis) with configurable output options and a custom handler API for extending or overriding element conversions.

## Documentation Goal

A reader should understand:
1. The DOM-based architecture: parse → walk → convert → join
2. How `html5ever` parses HTML into an `RcDom` tree
3. The depth-first DOM walker with block/inline element distinction
4. How each element handler converts HTML to Markdown
5. The text processing pipeline: HTML entity decoding, whitespace compression, Markdown escaping
6. The builder pattern for customization (skip tags, custom handlers, options)
7. How reference-style links use thread-local storage for deferred rendering
8. How code block fence markers adapt to content (````` for code containing ````)

## Documentation Structure

```
src.jeremychone/fork-htmd/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index / table of contents
│   ├── 00-overview.md          ← What fork-htmd is, architecture at a glance, public API
│   ├── 01-architecture.md      ← Module map, DOM walker, handler registry, text pipeline
│   ├── 02-dom-walker.md        ← Deep dive: DOM traversal, block/inline handling, text escaping
│   ├── 03-element-handlers.md  ← Deep dive: all 13+ element handlers
│   └── 04-options-config.md    ← Deep dive: Options enum, defaults, builder pattern
├── html/                       ← Generated HTML
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read all 17 source files | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-architecture.md | DONE |
| 4 | Write 02-dom-walker.md | DONE |
| 5 | Write 03-element-handlers.md | DONE |
| 6 | Write 04-options-config.md | DONE |
| 7 | Write README.md | DONE |
| 8 | Write spec.md | DONE |
| 9 | Generate HTML (build.py) | DONE |
| 10 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/fork-htmd
```

## Quality Requirements

All Iron Rules from the documentation directive apply.

## Expected Outcome

After reading these docs, a developer can:
- Integrate fork-htmd into a Rust project for HTML-to-Markdown conversion
- Customize element handlers for specific HTML tags
- Configure output options (heading style, code fence, link style, etc.)
- Understand the DOM traversal algorithm and text processing pipeline
- Write custom handlers for unsupported HTML elements

## Resume Point

Source read. Next: write markdown docs (00 through 04), then README, then build.
