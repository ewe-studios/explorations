---
source_location: iii/crates/iii-shell-proto/ + iii/crates/iii-shell-client/
repository: part of git@github.com:iii-hq/iii (monorepo)
explored_at: 2026-06-04
documentation_goal: Document iii-shell-proto (wire protocol) and iii-shell-client (async pipe-mode client) for the iii shell-exec channel.
---

# Spec: iii-shell-client + iii-shell-proto Documentation

## 1. Source

| Crate | LOC | Files | Purpose |
|-------|-----|-------|---------|
| iii-shell-proto | 1,026 | 1 (lib.rs) | Wire protocol: frame codec, ShellMessage types |
| iii-shell-client | 1,157 | 1 (lib.rs) | Async pipe-mode client for host side |

## 2. What They Are

**iii-shell-proto** defines the length-prefixed binary frame format and message types for the `iii worker exec` channel. It's consumed by iii-supervisor (re-export), iii-init (guest dispatcher), and iii-shell-client (host async client).

**iii-shell-client** is the host-side async client that connects to `~/.iii/managed/<name>/shell.sock`, speaks the shell protocol, and streams output through a caller-supplied OutputSink.

## 3-10. Standard sections

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-wire-protocol.md | TODO |
| 3 | 02-shell-client.md | TODO |
| 4 | 03-filesystem-ops.md | TODO |
| 5 | 04-cross-cutting.md | TODO |
| 6 | Grandfather review | TODO |
| 7 | Fix findings | TODO |
| 8 | Generate HTML | TODO |

Build via `python3 build.py .`. Grandfather review mandatory.
