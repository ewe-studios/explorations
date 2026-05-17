# htmd — Documentation Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/htmd/`
- **Crate name:** `htmd` (v0.5.4)
- **Language:** Rust (edition 2024)
- **License:** Apache-2.0
- **Author:** letmutex
- **Files:** 29 source files in src/
- **Dependencies:** html5ever 0.38, markup5ever_rcdom 0.38, phf 0.13

## What the Project Is

htmd is a Rust crate that converts HTML to Markdown, inspired by turndown.js. It uses `html5ever` for DOM parsing and a handler-based architecture for element conversion. Version 0.5.4 introduces two translation modes (`Pure` and `Faithful`), a `Handlers` trait for handler delegation, adjacent inline element merging, and a `phf`-based block element classification.

## Documentation Goal

A reader should understand:
1. The dual translation modes (Pure vs Faithful) and how Faithful embeds HTML when Markdown can't express semantics
2. The `Handlers` trait for handler delegation and child walking
3. The `HandlerResult` struct with `markdown_translated` flag
4. The `can_combine` algorithm for merging adjacent inline elements
5. The `is_plain_text` byte-level optimization
6. The new granular element handlers (tbody, thead, tr, td/th, caption, p, pre, span, html, head/body)
7. The custom `html_escape` module
8. The `tag_to_handler_indices` HashMap for O(1) tag lookup
9. The separated `html_to_tree()` and `tree_to_markdown()` methods

## Documentation Structure

```
src.jeremychone/htmd/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index / table of contents
│   ├── 00-overview.md          ← What htmd is, architecture, public API, key changes from v0.2
│   ├── 01-architecture.md      ← Module map, Handlers trait, translation modes, delegation
│   ├── 02-dom-walker.md        ← DOM traversal, can_combine, is_plain_text, append_normalized_content
│   ├── 03-element-handlers.md  ← All granular handlers + element_util serialization
│   └── 04-faithful-mode.md     ← Deep dive: Faithful translation mode, HTML embedding, serialize_element
├── html/                       ← Generated HTML
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read all source files | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-architecture.md | DONE |
| 4 | Write 02-dom-walker.md | DONE |
| 5 | Write 03-element-handlers.md | DONE |
| 6 | Write 04-faithful-mode.md | DONE |
| 7 | Write README.md | DONE |
| 8 | Write spec.md | DONE |
| 9 | Generate HTML (build.py) | DONE |
| 10 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/htmd
```

## Resume Point

Source read. Next: write markdown docs.
