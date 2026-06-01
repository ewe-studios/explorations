# rauthy — Spec

## Source Codebase Location

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.auth/src.rauthy/rauthy/`
- **Repository:** https://github.com/sebadob/rauthy.git
- **Language:** Rust
- **License:** Apache-2.0
- **Author:** Sebastian Dobe (sebadob)

## What This Project Is

rauthy is a lightweight, secure Identity Provider supporting OpenID Connect (OIDC), OAuth 2.0, and PAM (Pluggable Authentication Modules). It's designed to be:

- **Secure by default** — ed25519 token signing, S256 PKCE flow, FIDO2/WebAuthn passkeys
- **Fast and efficient** — Runs on Raspberry Pi, ~35-65MB memory usage
- **Highly available** — HA mode with Hiqlite or Postgres
- **Easy to operate** — Embedded Hiqlite database (no external DB required)

### Key Features

- OpenID Connect & OAuth 2.0 provider
- FIDO2/WebAuthn passkey support (passwordless login)
- MFA with security keys
- PAM integration for headless/CLI tools
- High availability mode
- Admin UI + User account dashboard
- Client branding/theming
- Events and auditing
- JWT token handling
- API key authentication

### Security Audit

Received independent security audit from Radically Open Security as part of NGI Zero Core funding. Report available [here](https://raw.githubusercontent.com/sebadob/rauthy/refs/heads/main/assets/security_audit_report_v0.32.pdf).

## Documentation Goal

After reading this documentation, an engineer should understand:

1. The OIDC/OAuth2 flow implementation
2. The authentication chain (password, MFA, passkeys)
3. The FIDO2/WebAuthn integration
4. The token handling (JWT, signing, validation)
5. The database layer (Hiqlite vs Postgres)
6. The caching strategy for performance
7. The PAM integration for CLI tools
8. The admin API and user management
9. The client configuration and branding
10. The HA deployment architecture

## Documentation Structure

```
src.auth/rauthy/
├── spec.md                      ← This file
├── exploration.md               ← Original exploration
├── grandfather-review.md        ← Verification report
├── markdown/
│   ├── README.md                ← Index
│   ├── 00-overview.md           ← Philosophy, features
│   ├── 01-architecture.md       ← System architecture
│   ├── 02-authentication.md   ← Auth flows, MFA, passkeys
│   ├── 03-oidc-oauth.md         ← OIDC/OAuth2 implementation
│   ├── 04-jwt-tokens.md         ← JWT handling, signing
│   ├── 05-database.md           ← Hiqlite, Postgres, caching
│   ├── 06-pam.md                ← PAM integration
│   ├── 07-admin-api.md          ← Admin API, user management
│   └── 08-deployment.md           │ HA, config, branding
├── html/
└── (uses ../../build.py)
```

## Tasks

| Phase | Document | Status | Notes |
|-------|----------|--------|-------|
| 1 | Read source code | DONE | Via README and exploration |
| 2 | Create spec.md | DONE | This file |
| 3 | Write README.md | DONE | Index |
| 3 | Write 00-overview.md | DONE | Philosophy, features |
| 3 | Write 01-architecture.md | DONE | System architecture |
| 3 | Write 02-authentication.md | DONE | Auth flows, MFA, passkeys |
| 3 | Write 03-oidc-oauth.md | DONE | OIDC/OAuth2 implementation |
| 3 | Write 04-jwt-tokens.md | DONE | JWT handling, signing |
| 3 | Write 05-database.md | DONE | Hiqlite, Postgres, caching |
| 3 | Write 06-pam.md | DONE | PAM integration |
| 3 | Write 07-admin-api.md | DONE | Admin API, user management |
| 3 | Write 08-deployment.md | DONE | HA, config, branding |
| 4 | Generate HTML | DONE | All 9 documents generated |
| 5 | Grandfather review | TODO | Verify against source |

## Build System

**Script:** `../../build.py`

```bash
python3 build.py src.auth/rauthy
```

## Quality Requirements

All documents must meet the Iron Rules from the markdown directive.

## Resume Point

Resume from the last uncompleted task in the Tasks table.
