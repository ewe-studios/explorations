# What Is src.Instatic?

Instatic is a self-hosted, open-source CMS (version 0.0.6, pre-release) with a built-in visual editor. It runs as a single Bun process serving the public website, the admin editor, the CMS API, published pages, and uploaded media — backed by either Postgres or SQLite.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/Instatic/`

## What It Is (and Is Not)

Instatic is a **visual CMS** — you design pages by dragging and configuring blocks on a canvas, then publish them as plain semantic HTML. The published output has **no framework runtime**, **no client-side hydration**, and **no decorative markup**.

**It is not** a multi-tenant SaaS. The codebase intentionally avoids assumptions about multi-tenant operation. It is designed for self-hosting: one instance, one database, one set of sites.

**Aha:** Instatic is in **pre-release** — there are no external users, no production traffic, nothing shipped. This means backward compatibility is explicitly not a goal. The CLAUDE.md states: "No backward compatibility. Ever." Breaking changes are expected and acceptable. This shapes how the code is written — no migration paths, no deprecation warnings, no feature flags for gradual rollout.

## Stack at a Glance

| Layer | Technology |
|-------|-----------|
| Runtime | Bun (server + tooling) — **not Node** |
| Language | TypeScript everywhere |
| Frontend | React 19 with **React Compiler** (Babel preset) + Vite |
| State | Zustand + Mutative (patch-based undo history) |
| Code editing | CodeMirror (CSS, HTML, JS, JSON, Markdown) |
| Rich text | TipTap (with table extensions) |
| Drag-and-drop | @dnd-kit/core |
| Server | `Bun.serve` with hand-written router (`server/router.ts`) |
| Database | Postgres (`Bun.sql`) OR SQLite (`bun:sqlite`), selected by `DATABASE_URL` |
| Validation | TypeBox at every untyped boundary — **zod is banned** |
| Sanitization | DOMPurify at publisher boundary |
| Plugins | QuickJS-WASM sandbox with capability-gated SDK |
| Icons | pixel-art-icons (vendored) — **lucide-react is banned** |
| Routing | In-house router (`src/admin/lib/routing/`) — **react-router-dom is banned** |
| AI | HTTP-only drivers (no provider SDKs) — **@anthropic-ai/sdk, @openai/agents are banned** |
| Testing | `bun test` (unit) + Playwright (E2E) |

## Key Architectural Principles

1. **One process** — The same Bun process serves visitors, admins, the API, the streaming agent endpoint, and uploads.
2. **Two off-main-thread workers** — Plugin server code (QuickJS-WASM) and image processing (sharp) run in `Bun.Worker`s to keep the main thread responsive.
3. **One content model** — Posts, pages, and visual components all live in `data_tables` + `data_rows`. No separate `pages` or `page_versions` tables.
4. **Two frontends, one bundle** — The admin app and visual editor run in the same Vite-built SPA under `/admin/*`.
5. **Plugins run sandboxed** — Server entrypoints and canvas module packs execute inside a QuickJS-WASM VM with no host access.
6. **Three publishing layers** — Static bake (A), in-memory LRU (B), lazy-fetch holes (C).
7. **Every untyped boundary uses TypeBox** — HTTP responses, request bodies, persisted JSON, plugin manifests, settings.

## Pre-Release Status

**This project is in PRE-RELEASE.** Consequences:

- No backward compatibility — breaking changes are expected
- No migration paths needed — the database schema is still evolving
- No deprecation cycles — old APIs can be removed without warning
- No feature flags — features are either on or off
- Schema evolution is handled by editing the migration files directly

**Source:** `CLAUDE.md` line 76+: "Development status — READ THIS FIRST"

## Quick Start

```bash
bun install        # Install dependencies
bun dev            # Start dev server (Vite + Bun server)
bun dev:server     # Start server only
bun dev:vite       # Start Vite only
bun test           # Run unit tests
bun test:e2e       # Run Playwright E2E tests
bun doctor         # Health check all subsystems
```

**Source:** `package.json` scripts (38 scripts total).

## Next

Continue to [01-architecture](01-architecture.md) for the system architecture: one process, two workers, 19-route dispatcher, and dual database engines.
