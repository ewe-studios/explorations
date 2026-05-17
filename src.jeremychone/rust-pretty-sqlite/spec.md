# rust-pretty-sqlite — Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/rust-pretty-sqlite/`
- **Crate name:** `pretty_sqlite`
- **Language:** Rust
- **Files:** 4 source files in src/
- **Dependencies:** rusqlite, tabled, derive_more

## What the Project Is

A tiny library for pretty-printing SQLite query results as formatted tables using `tabled`. Supports automatic column name extraction, cell truncation, and row limiting.

## Documentation Structure

```
src.jeremychone/rust-pretty-sqlite/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index
│   └── 00-overview.md          ← API, architecture, formatting
├── html/                       ← Generated HTML
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read all source files | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write README.md | DONE |
| 4 | Write spec.md | DONE |
| 5 | Generate HTML (build.py) | DONE |
| 6 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/rust-pretty-sqlite
```
