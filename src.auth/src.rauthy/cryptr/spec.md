# cryptr — Spec

## Source Codebase Location

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.auth/src.rauthy/cryptr/`
- **Repository:** https://github.com/sebadob/cryptr.git
- **Language:** Rust
- **Version:** 0.10.0
- **License:** Apache-2.0
- **Author:** Sebastian Dobe <sebastiandobe@mailbox.org>

## What This Project Is

cryptr is a Rust library and CLI tool for simple encrypted (streaming) values using ChaCha20Poly1305 AEAD encryption. It provides:

- **Small value encryption** — Encrypt database columns with ~40 byte header overhead
- **Streaming file encryption** — Handle files of any size with minimal memory (~4 buffers)
- **Key rotation support** — Built-in versioning for seamless migration
- **S3 integration** — Stream encrypted data directly to S3 storage
- **Tamper resistance** — MAC validation on each chunk

## Documentation Goal

After reading this documentation, an engineer should understand:

1. The encryption format and header structure
2. The streaming architecture for large files
3. Key derivation using Argon2id
4. Key management and rotation
5. CLI usage and commands
6. Library API and examples
7. S3 integration patterns
8. Performance characteristics

## Documentation Structure

```
src.auth/src.rauthy/cryptr/
├── spec.md                      ← This file
├── exploration.md               ← Original exploration
├── markdown/
│   ├── README.md                ← Index
│   ├── 00-overview.md           ← Philosophy, features
│   ├── 01-encryption.md         ← ChaCha20Poly1305, format
│   ├── 02-keys.md               ← Key derivation, management
│   ├── 03-streaming.md          ← Streaming architecture
│   ├── 04-s3.md                 │ S3 integration
│   └── 05-cli.md                │ CLI usage
├── html/
└── (uses ../../../build.py)
```

## Tasks

| Phase | Document | Status | Notes |
|-------|----------|--------|-------|
| 1 | Read source code | DONE | Via exploration |
| 2 | Create spec.md | DONE | This file |
| 3 | Write README.md | DONE | Index |
| 3 | Write 00-overview.md | DONE | Philosophy, features |
| 3 | Write 01-encryption.md | DONE | ChaCha20Poly1305, format |
| 3 | Write 02-keys.md | DONE | Key derivation, management |
| 3 | Write 03-streaming.md | DONE | Streaming architecture |
| 3 | Write 04-s3.md | DONE | S3 integration |
| 3 | Write 05-cli.md | DONE | CLI usage |
| 4 | Generate HTML | DONE | All 6 documents generated |
| 5 | Grandfather review | TODO | Verify against source |

## Build System

**Script:** `../../../build.py`

```bash
python3 build.py src.auth/src.rauthy/cryptr
```

## Quality Requirements

All documents must meet the Iron Rules from the markdown directive.

## Resume Point

Resume from the last uncompleted task in the Tasks table.
