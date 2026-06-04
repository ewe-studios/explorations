---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii/engine/src/workers/
repository: part of git@github.com:iii-hq/iii (monorepo)
explored_at: 2026-06-04
documentation_goal: Document the in-process engine workers — configuration, engine_fn, rest_api, pubsub, http_functions, shell, bridge_client.
---

# Spec: Engine Workers Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `iii/engine/src/workers/` (subset) |
| Language | Rust (edition 2024) |
| License | Elastic-2.0 |
| Total LOC | 13,129 |

| Worker | LOC | Purpose |
|--------|-----|---------|
| configuration | 2,693 | Config store with adapters (bridge, filesystem) |
| engine_fn | 2,617 | Built-in engine functions (list, info, channels) |
| rest_api | 4,810 | HTTP server with hot-reloadable routes |
| pubsub | 903 | Pub/sub messaging (local + Redis adapters) |
| http_functions | 592 | HTTP function invocation |
| shell | 973 | Shell execution with allowlists/denylists |
| bridge_client | 541 | Bridge client for external connections |

## 2-10. Standard sections

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-configuration.md | TODO |
| 3 | 02-engine-functions.md | TODO |
| 4 | 03-rest-api.md | TODO |
| 5 | 04-pubsub.md | TODO |
| 6 | 05-http-functions.md | TODO |
| 7 | 06-shell.md | TODO |
| 8 | 07-bridge-client.md | TODO |
| 9 | Grandfather review | TODO |
| 10 | Fix findings | TODO |
| 11 | Generate HTML | TODO |

Build via `python3 build.py .`. Grandfather review mandatory.
