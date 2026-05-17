# rust-simple-fs — Documentation Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/rust-simple-fs/`
- **Crate name:** `simple-fs` (v0.12.0-WIP)
- **Language:** Rust (edition 2024)
- **License:** MIT OR Apache-2.0
- **Author:** Jeremy Chone
- **Files:** 39 source files in src/
- **Dependencies:** camino, pathdiff, walkdir, globset, notify, notify-debouncer-full, trash, mime_guess, derive_more, flume, memchr, path-clean
- **Optional features:** `with-json` (serde, serde_json), `with-toml` (serde, toml), `bin-nums` (byteorder), `full` (enables all)

## What the Project Is

simple-fs is a Rust crate providing a simple and convenient API for file system access. It centers around `SPath` — a UTF-8 guaranteed, POSIX-normalized path wrapper — and provides: file/dir listing with glob filtering and negation, safe delete/trash with safety guards, file span reading (byte ranges, line spans, CSV-aware row spans), file watching with debounced events, JSON/TOML load/save, and binary number serialization.

## Documentation Goals

1. The `SPath` type: UTF-8 guarantee, POSIX normalization, collapse without I/O, transformers, mime detection
2. The error model: `Error` enum, `PathAndCause`, `Cause` types, feature-gated variants
3. File/directory listing: `GlobsFileIter` grouping algorithm, negated globs, exclude patterns, `sort_by_globs`
4. Span APIs: `read_span` (byte ranges via platform-specific file I/O), `line_spans` (streaming CRLF-aware), `csv_row_spans` (quote-aware parsing)
5. Safer operations: `safer_remove`, `safer_trash` with safety checks (restrict to cwd, must_contain patterns)
6. Feature-gated modules: JSON load/save/NDJSON, TOML load/save, binary number serialization
7. File watching: `SWatcher`, `SEvent`, debouncing, event deduplication
8. Path reshape: normalization (`\\?\` prefix removal, `//` collapse, `/./` removal) and collapsing (`..` resolution without I/O)

## Documentation Structure

```
src.jeremychone/rust-simple-fs/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index / table of contents
│   ├── 00-overview.md          ← What simple-fs is, architecture, public API, features
│   ├── 01-architecture.md      ← Module map, Error model, SPath contract, feature flags
│   ├── 02-spath.md             ← SPath type: normalization, collapse, transformers, mime, diff
│   ├── 03-listing.md           ← File/dir listing, glob grouping, sort_by_globs
│   ├── 04-spans-safer-watch.md ← Span APIs, safer remove/trash, file watching
│   └── 05-features.md          ← JSON, TOML, binary numbers, pretty size
├── html/                       ← Generated HTML
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read all source files | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-architecture.md | DONE |
| 4 | Write 02-spath.md | DONE |
| 5 | Write 03-listing.md | DONE |
| 6 | Write 04-spans-safer-watch.md | DONE |
| 7 | Write 05-features.md | DONE |
| 8 | Write README.md | DONE |
| 9 | Write spec.md | DONE |
| 10 | Generate HTML (build.py) | DONE |
| 11 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/rust-simple-fs
```

## Resume Point

Source read. Next: write markdown docs (00 through 05), then README, then build.
