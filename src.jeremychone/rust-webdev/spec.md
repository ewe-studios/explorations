# rust-webdev — Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/rust-webdev/`
- **Crate name:** `webdev`
- **Language:** Rust
- **Type:** CLI binary (no library)
- **Files:** 7 source files (5 .rs + 1 .js + 1 inline HTML)
- **Dependencies:** clap, warp, tokio, futures, notify, notify-debouncer-mini, pathdiff

## What the Project Is

`webdev` is a lightweight static file server built on `warp`. It serves local directories over HTTP with directory listing, extension-less URL routing (auto-appending `.html`), and live reload via WebSocket file watching.

## Documentation Structure

```
src.jeremychone/rust-webdev/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index
│   └── 00-overview.md          ← CLI, architecture, live reload, routing
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
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/rust-webdev
```
