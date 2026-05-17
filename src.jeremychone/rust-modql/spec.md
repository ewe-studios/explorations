# rust-modql — Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/rust-modql/`
- **Crate name:** `modql`
- **Language:** Rust
- **Type:** Library crate
- **Files:** 41 source files across 6 modules
- **Dependencies:** serde, serde_json, sea-query (optional), rusqlite (optional), modql-macros (proc-macro)

## What the Project Is

`modql` is a model query language library that provides expressive filtering (inspired by [joql.org](https://joql.org)), field metadata extraction, and SQL generation for both sea-query and rusqlite backends. It is serialization-agnostic but provides JSON deserialization for convenient filter parsing.

## Documentation Structure

```
src.jeremychone/rust-modql/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index
│   ├── 00-overview.md          ← Architecture, filter system, field system, list options
│   ├── 01-filter-ops.md        ← OpVal types, JSON deserialization, sea-query conversion
│   └── 02-sql-integration.md   ← Sea-query and SQLite field systems, value conversion
├── html/                       ← Generated HTML
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read all source files | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-filter-ops.md | DONE |
| 4 | Write 02-sql-integration.md | DONE |
| 5 | Write README.md | DONE |
| 6 | Write spec.md | DONE |
| 7 | Generate HTML (build.py) | DONE |
| 8 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/rust-modql
```
