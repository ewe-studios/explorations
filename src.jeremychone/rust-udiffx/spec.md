# udiffx — Documentation Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/rust-udiffx/`
- **Language:** Rust (edition 2021)
- **Version:** 0.1.42-WIP
- **License:** MIT OR Apache-2.0
- **Files:** 23 Rust source files, ~5,523 lines
- **Dependencies:** markex (tag extraction), diffy (diff parsing/applying), simple-fs (file I/O), derive_more, tracing

## What the Project Is

udiffx is a Rust crate that parses and applies LLM-optimized unified diff patches and XML-like file change directives. It provides a structured envelope (`<FILE_CHANGES>`) for AI agents to express multiple file operations — create, patch, append, copy, rename, delete — in a single response, then extracts and applies those changes to a filesystem. The core innovation is the patch completer: given an LLM's approximate `@@` hunk (with no line numbers), it locates the matching position in the original file using Strict → Resilient → Fuzzy matching tiers, then reconstructs a valid `diffy`-compatible unified diff patch.

## Documentation Goal

A reader should understand:
1. The two-phase architecture (Extract → Apply) and why it's structured that way
2. How the `markex` crate is used for XML-like tag extraction and self-closing tag handling
3. How the patch completer converts simplified numberless hunks into valid unified diff patches
4. The tiered matching algorithm (Strict, Resilient, Fuzzy) with all comparison strategies
5. How candidate scoring works with exact_ws_count, hint_bonus, uniform_indent, and distance
6. How tilde range shorthand (`~`) is parsed, validated, and expanded
7. How path security is enforced via SPath collapse and starts_with checks
8. How partial patch success works with per-hunk error reporting

## Documentation Structure

```
src.jeremychone/rust-udiffx/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index / table of contents
│   ├── 00-overview.md          ← What udiffx is, two-phase architecture, public API
│   ├── 01-architecture.md      ← Module map, layer diagram, sequence diagram, security
│   ├── 02-extract.md           ← Extraction layer: markex parsing, directives, content
│   ├── 03-patch-completer.md   ← Patch completer: tiered matching, scoring, tilde ranges
│   └── 04-applier.md           ← Applier: filesystem execution, security, incremental patch
├── html/                       ← Generated HTML (by build.py)
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read all 23 source files | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-architecture.md | DONE |
| 4 | Write 02-extract.md | DONE |
| 5 | Write 03-patch-completer.md | DONE |
| 6 | Write 04-applier.md | DONE |
| 7 | Write README.md | DONE |
| 8 | Write spec.md | DONE |
| 9 | Generate HTML (build.py) | DONE |
| 10 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/rust-udiffx
```

Shared `build.py` converts markdown to HTML with mermaid, dark/light theme, prev/next navigation, and index generation.

## Quality Requirements

All Iron Rules from the documentation directive apply:
1. Detailed sections with real code snippets (file paths required)
2. Teach key concepts quickly — first sentence is the thesis
3. Clear articulation — one idea per sentence
4. Minimum 2 mermaid diagrams per document
5. Good visual assets (tables, diagrams, code blocks)
6. Generated HTML with proper navigation
7. Cross-references between all documents
8. Source path references (file:line)
9. At least 1 Aha moment per document
10. Navigation: index + prev/next on every page

## Expected Outcome

After reading these docs, a developer can:
- Integrate udiffx into a Rust project for LLM-driven file manipulation
- Understand why the patch completer uses tiered matching instead of exact diff
- Debug patch failures by reading `DirectiveStatus` match_tier and hunk_errors
- Write correct `<FILE_CHANGES>` XML envelopes for an LLM to consume
- Extend the matching algorithm with new comparison strategies

## Resume Point

All documents are written. Next step: run `build.py src.jeremychone/rust-udiffx` to generate HTML, then perform grandfather review.
