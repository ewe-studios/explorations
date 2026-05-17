# rust-dinf — Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/rust-dinf/`
- **Crate name:** `dinf`
- **Language:** Rust (edition 2024)
- **Type:** CLI binary (no library)
- **Files:** 6 source files in src/
- **Dependencies:** clap, derive_more, globset, walkdir, simple_fs, num_format

## What the Project Is

`dinf` is a CLI tool for analyzing directory information — total file count, total size, top N biggest files, and top N biggest file extensions. Uses decimal (1000-based) size units and supports glob filtering, summary mode, and children mode.

## Documentation Structure

```
src.jeremychone/rust-dinf/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index
│   └── 00-overview.md          ← CLI usage, architecture, formatting
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
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/rust-dinf
```
