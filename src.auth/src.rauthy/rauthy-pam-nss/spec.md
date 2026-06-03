# rauthy-pam-nss — Spec

## Source Codebase Location

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.auth/src.rauthy/rauthy-pam-nss/`
- **Repository:** https://github.com/sebadob/rauthy-pam-nss.git
- **Language:** Rust
- **License:** AGPL-3.0
- **Author:** Sebastian Dobe

## What This Project Is

PAM and NSS modules for rauthy that enable system-level authentication:

- **NSS module** — Resolve users, groups, hosts from rauthy
- **PAM module** — Authenticate via rauthy (password, passkeys)
- **SSH support** — SSH login with rauthy accounts
- **SELinux policies** — Custom security policies

## Documentation Goal

After reading this documentation, an engineer should understand:

1. The NSS module and user/group/host resolution
2. The PAM module and authentication flow
3. Installation and configuration
4. SSH integration with AuthorizedKeysCommand
5. Remote authentication via PAM passwords
6. SELinux configuration
7. Troubleshooting

## Documentation Structure

```
src.auth/src.rauthy/rauthy-pam-nss/
├── spec.md                      ← This file
├── exploration.md               ← Original exploration
├── markdown/
│   ├── README.md                ← Index
│   ├── 00-overview.md           ← Philosophy, features
│   ├── 01-nss.md                ← NSS module
│   ├── 02-pam.md                │ PAM module
│   ├── 03-ssh.md                │ SSH integration
│   ├── 04-install.md              │ Installation
│   └── 05-config.md               │ Configuration
├── html/
└── (uses ../../../build.py)
```

## Tasks

| Phase | Document | Status | Notes |
|-------|----------|--------|-------|
| 1 | Read source code | DONE | Via exploration |
| 2 | Create spec.md | DONE | This file |
| 3 | Write README.md | DONE | Index |
| 3 | Write 00-overview.md | DONE | Philosophy |
| 3 | Write 01-nss.md | DONE | NSS module |
| 3 | Write 02-pam.md | DONE | PAM module |
| 3 | Write 03-ssh.md | DONE | SSH integration |
| 3 | Write 04-install.md | DONE | Installation |
| 3 | Write 05-config.md | DONE | Configuration |
| 4 | Generate HTML | DONE | All 6 documents generated |
| 5 | Grandfather review | TODO | Verify against source |

## Build System

**Script:** `../../../build.py`

```bash
python3 build.py src.auth/src.rauthy/rauthy-pam-nss
```

## Quality Requirements

All documents must meet the Iron Rules from the markdown directive.

## Resume Point

Resume from the last uncompleted task in the Tasks table.
