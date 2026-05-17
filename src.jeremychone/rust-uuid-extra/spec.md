# rust-uuid-extra — Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/rust-uuid-extra/`
- **Crate name:** `uuid_extra`
- **Language:** Rust
- **Type:** Library crate
- **Files:** 6 source files
- **Dependencies:** uuid, bs58, base64, derive_more

## What the Project Is

`uuid_extra` provides convenience wrappers around the `uuid` crate for generating UUID v4/v7, encoding them in Base58 or Base64 (standard, URL-safe, URL-safe-no-pad), and extracting epoch millisecond timestamps from v7 UUIDs.

## Documentation Structure

```
src.jeremychone/rust-uuid-extra/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index
│   └── 00-overview.md          ← API, encoding, timestamp extraction, errors
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
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/rust-uuid-extra
```
