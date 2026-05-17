# rust-sqlb — Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/rust-sqlb/`
- **Crate name:** `sqlb`
- **Language:** Rust (edition 2024)
- **Files:** 9 source files in src/
- **Dependencies:** sqlx, async_trait, time, uuid
- **Optional features:** `chrono-support` (chrono), `json` (serde_json), `decimal` (rust_decimal)
- **Also depends on:** `sqlb-macros` (external crate, provides `#[derive(Fields)]`)

## What the Project Is

rust-sqlb is a Postgres-only SQL query builder built on `sqlx`. It provides a fluent, builder-pattern API for SELECT, INSERT, UPDATE, and DELETE queries with parameterized binding, RETURNING support, and automatic table/column name escaping. Safety guards prevent accidental UPDATE/DELETE without WHERE clauses.

## Documentation Structure

```
src.jeremychone/rust-sqlb/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index
│   ├── 00-overview.md          ← What sqlb is, architecture, quick start
│   ├── 01-core-types.md        ← Field, HasFields, SqlBuilder, Whereable, SqlxBindable, Raw
│   └── 02-builders.md          ← Select, Insert, Update, Delete, sqlx_exec
├── html/                       ← Generated HTML
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read all source files | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-core-types.md | DONE |
| 4 | Write 02-builders.md | DONE |
| 5 | Write README.md | DONE |
| 6 | Write spec.md | DONE |
| 7 | Generate HTML (build.py) | DONE |
| 8 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/rust-sqlb
```
