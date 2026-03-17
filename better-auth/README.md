---
type: index
created: 2026-03-17
updated: 2026-03-17
---

# Better Auth WASM - Documentation Index

Complete documentation for building a self-contained, portable authentication system using Rust + WASM on CloudFlare Workers.

## Overview

This documentation expands on the vision of Better Auth as a fully portable authentication system that:

- Runs on CloudFlare Workers via WebAssembly
- Uses D2 (SQLite at the edge) for primary storage
- Uses R2 for object storage (avatars, exports)
- Uses KV for caching and rate limiting
- Requires **zero external services** (no Auth0, Firebase, Supabase)
- Provides cryptographically secure operations via Rust

## Document Map

### Core Documentation

| Document | Purpose | Status |
|----------|---------|--------|
| [WASM_DEEP_DIVE.md](./WASM_DEEP_DIVE.md) | Architecture overview, crate structure, authentication flows | ✅ Complete |
| [OAUTH_IMPLEMENTATION.md](./OAUTH_IMPLEMENTATION.md) | OAuth 2.0 providers, PKCE flow, custom providers | ✅ Complete |
| [DATABASE_SCHEMA.md](./DATABASE_SCHEMA.md) | Complete D2/SQLite schema with all plugin tables | ✅ Complete |
| [JWT_SESSIONS.md](./JWT_SESSIONS.md) | JWT/PASETO tokens, session management, cookies | ✅ Complete |
| [CLOUDFLARE_DEPLOYMENT.md](./CLOUDFLARE_DEPLOYMENT.md) | Worker deployment, D2/R2/KV setup, monitoring | ✅ Complete |
| [examples/README.md](./examples/README.md) | Working code examples for CloudFlare, Node.js, browser | ✅ Complete |

### Related Documents

| Document | Purpose |
|----------|---------|
| [exploration.md](./exploration.md) | Original Better Auth codebase exploration |
| [RUST_REVISION_PLAN.md](./RUST_REVISION_PLAN.md) | Initial Rust translation plan |

---

## Quick Start

### 1. Understand the Architecture

Read [WASM_DEEP_DIVE.md](./WASM_DEEP_DIVE.md) for:
- High-level architecture
- Rust crate structure
- Authentication mechanism overview
- CloudFlare Workers integration

### 2. Set Up Database

Use the schema from [DATABASE_SCHEMA.md](./DATABASE_SCHEMA.md):
- Core tables (users, sessions, accounts)
- Plugin tables (2FA, organization, API keys)
- Indexes and views

### 3. Implement Crypto

Follow [JWT_SESSIONS.md](./JWT_SESSIONS.md) for:
- JWT signing/verification
- PASETO tokens
- Password hashing with Argon2id
- Session management

### 4. Add OAuth

Reference [OAUTH_IMPLEMENTATION.md](./OAUTH_IMPLEMENTATION.md) for:
- Authorization code flow with PKCE
- Built-in providers (GitHub, Google)
- Custom OIDC providers

### 5. Deploy to CloudFlare

Follow [CLOUDFLARE_DEPLOYMENT.md](./CLOUDFLARE_DEPLOYMENT.md) for:
- Wrangler configuration
- D2/R2/KV setup
- Worker implementation
- Monitoring and troubleshooting

### 6. Run Examples

Check [examples/README.md](./examples/README.md) for:
- CloudFlare Worker example
- Node.js with SQLite example
- Minimal browser example

---

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                    CloudFlare Worker (WASM)                      │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Better Auth Core (Rust → WASM)                │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │ │
│  │  │ Password │ │   JWT    │ │  OAuth2  │ │   Session    │  │ │
│  │  │ Hashing  │ │ Signing  │ │  Flows   │ │   Manager    │  │ │
│  │  │ Argon2id │ │ PASETO   │ │  PKCE    │ │   Cookies    │  │ │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
│         │                    │                    │              │
│         ▼                    ▼                    ▼              │
│  ┌─────────────┐      ┌─────────────┐     ┌─────────────┐      │
│  │  D2 (SQL)   │      │  R2 (S3)    │     │   KV Store  │      │
│  │  - users    │      │  - avatars  │     │  - sessions │      │
│  │  - sessions │      │  - exports  │     │  - rate-lmt │      │
│  │  - accounts │      │  - backups  │     │  - tokens   │      │
│  └─────────────┘      └─────────────┘     └─────────────┘      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Technologies

### Rust Crates

| Crate | Purpose |
|-------|---------|
| `argon2` v0.5 | Password hashing |
| `jwt-simple` v0.12 | JWT signing/verification |
| `totp-rs` v5.6 | TOTP/HOTP for 2FA |
| `chacha20poly1305` v0.10 | Symmetric encryption |
| `hmac` v0.12 | HMAC signing |
| `rusty_paseto` v0.7 | PASETO tokens |
| `wasm-bindgen` v0.2 | WASM bindings |

### CloudFlare Services

| Service | Purpose | Cost (10K users) |
|---------|---------|------------------|
| Workers | Compute | Free tier |
| D2 | SQLite database | ~$1-2/month |
| R2 | Object storage | ~$0.02/month |
| KV | Session cache | ~$1-2/month |

**Total: ~$5/month or less**

---

## Authentication Flows

### Username + Password

```
1. POST /auth/sign-up { email, password }
   → Validate, hash password (Argon2id), create user

2. POST /auth/sign-in { email, password }
   → Verify password, create session, set cookie

3. GET /auth/session
   → Verify session token, return user info

4. POST /auth/sign-out
   → Revoke session, clear cookie
```

### Magic Link

```
1. POST /auth/magic-link/request { email }
   → Generate HMAC-signed token, store in DB

2. Send email with link: /auth/magic-link/verify?token=xxx

3. GET /auth/magic-link/verify?token=xxx&email=yyy
   → Verify token, create session, redirect with cookie
```

### OAuth 2.0 (GitHub, Google, etc.)

```
1. GET /auth/oauth/:provider
   → Generate state + code_verifier (PKCE)
   → Redirect to provider authorization URL

2. Provider redirects to /auth/oauth/:provider/callback?code=xxx

3. Exchange code for tokens
   → Get user info from provider
   → Link or create account
   → Create session
```

### Two-Factor (TOTP)

```
1. POST /auth/2fa/enable
   → Generate TOTP secret, return QR code

2. POST /auth/2fa/verify { code }
   → Verify TOTP code, enable 2FA

3. Subsequent sign-ins require code
   → Sign in → prompt for 2FA → verify → session
```

---

## API Reference

### Authentication Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | /auth/sign-up | Create account |
| POST | /auth/sign-in | Sign in |
| POST | /auth/sign-out | Sign out |
| GET | /auth/session | Get current session |
| POST | /auth/refresh | Refresh session |
| POST | /auth/revoke | Revoke all sessions |

### Password Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | /auth/password/reset/request | Request reset |
| POST | /auth/password/reset/confirm | Confirm reset |
| POST | /auth/password/change | Change password |

### Magic Link Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | /auth/magic-link/request | Request link |
| GET | /auth/magic-link/verify | Verify link |

### OAuth Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | /auth/oauth/:provider | Start OAuth |
| GET | /auth/oauth/:provider/callback | Callback |

### Two-Factor Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | /auth/2fa/enable | Enable 2FA |
| POST | /auth/2fa/disable | Disable 2FA |
| POST | /auth/2fa/verify | Verify code |
| POST | /auth/2fa/backup | Use backup code |

---

## Database Tables

### Core Tables

| Table | Purpose |
|-------|---------|
| `users` | User accounts |
| `sessions` | Active sessions |
| `accounts` | OAuth account links |
| `verification_tokens` | Email verification, password reset |

### Plugin Tables

| Table | Plugin | Purpose |
|-------|--------|---------|
| `two_factor_secrets` | two-factor | TOTP secrets, backup codes |
| `organizations` | organization | Org entities |
| `organization_members` | organization | Org membership |
| `organization_invitations` | organization | Pending invitations |
| `api_keys` | api-key | API key authentication |
| `audit_logs` | admin | Activity logging |
| `passkeys` | passkey | WebAuthn credentials |
| `magic_links` | magic-link | Magic link tracking |

---

## Security Features

- [x] Argon2id password hashing (memory-hard, timing-safe)
- [x] Constant-time comparisons for all sensitive operations
- [x] HMAC-signed tokens (magic links, verification)
- [x] PKCE for OAuth 2.0 flows
- [x] Secure cookie flags (HttpOnly, Secure, SameSite)
- [x] Rate limiting on auth endpoints
- [x] Session invalidation on password change
- [x] CSRF protection via state tokens
- [x] XSS prevention via HttpOnly cookies

---

## Next Steps

1. **Implement the Rust crates** - Start with `better-auth-crypto` for password hashing and JWT
2. **Build WASM bindings** - Use `wasm-bindgen` to expose Rust functions to JavaScript
3. **Create CloudFlare Worker** - Implement request handlers with D2/R2/KV bindings
4. **Add OAuth providers** - Start with GitHub and Google
5. **Implement plugins** - Add two-factor, organization, and other features
6. **Deploy and monitor** - Use Wrangler for deployment and CloudFlare Analytics for monitoring

---

## Gold Nuggets Summary

This documentation covers:

1. **WASM Portability** - How Rust + WASM enables running identical auth logic on CloudFlare Workers, Node.js, Deno, and Bun

2. **Self-Contained Architecture** - No external services required:
   - D2 for ACID-compliant SQLite at the edge
   - R2 for S3-compatible object storage
   - KV for low-latency caching

3. **Cryptographic Excellence** - Best-in-class crypto:
   - Argon2id for passwords (64MB memory, 3 iterations)
   - JWT with HS256/HS512
   - PASETO for type-safe tokens
   - XChaCha20-Poly1305 for session encryption

4. **Complete Auth Flows** - Every authentication method:
   - Username + password
   - Magic links
   - OAuth 2.0 (GitHub, Google, custom OIDC)
   - TOTP two-factor authentication
   - Passkeys (WebAuthn)
   - API keys

5. **Plugin Architecture** - Extensible design matching Better Auth's plugin system:
   - Two-factor authentication
   - Organizations (multi-tenant)
   - API keys
   - Admin/audit logging
   - Passkeys

6. **CloudFlare Deployment** - Complete guide to:
   - Setting up D2, R2, KV
   - Wrangler configuration
   - Worker implementation
   - Cost estimation (~$5/month for 10K users)

7. **Working Examples** - Code examples for:
   - CloudFlare Workers
   - Node.js with SQLite
   - Minimal browser usage

---

## Appendix: File Reference

```
better-auth/
├── WASM_DEEP_DIVE.md           # Main architecture document
├── OAUTH_IMPLEMENTATION.md     # OAuth providers and flows
├── DATABASE_SCHEMA.md          # Complete SQL schema
├── JWT_SESSIONS.md             # Token and session management
├── CLOUDFLARE_DEPLOYMENT.md    # Deployment guide
├── exploration.md              # Original codebase exploration
├── RUST_REVISION_PLAN.md       # Initial Rust translation plan
└── examples/
    └── README.md               # Working code examples
```
