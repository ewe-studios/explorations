---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii/console/
repository: part of git@github.com:iii-hq/iii (monorepo)
explored_at: 2026-06-04
documentation_goal: Document iii-console — the developer console for the iii engine with React frontend and Rust backend proxy.
---

# Spec: iii-console Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `iii/console/` |
| Languages | Rust (backend), TypeScript/React (frontend) |
| License | Apache-2.0 |
| Frontend LOC | 16,376 |
| Backend LOC | 2,395 |
| Total | 18,771 |

## 2. What iii-console Is

The developer console for the iii engine — a React SPA served by a Rust axum server that proxies HTTP and WebSocket connections to the engine. Provides UI for functions, triggers, workers, traces, logs, streams, queues, states, and dead-letter queues.

## 3-10. Standard sections

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-backend.md | TODO |
| 3 | 02-frontend.md | TODO |
| 4 | 03-trace-visualization.md | TODO |
| 5 | 04-cross-cutting.md | TODO |
| 6 | Grandfather review | DONE |
| 7 | Fix findings | DONE (no discrepancies — all verified) |
| 8 | Generate HTML | DONE |

Build via `python3 build.py .`. Grandfather review mandatory.
