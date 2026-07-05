# Instatic — Self-Hosted CMS with Visual Editor

Single Bun process serving public site, admin editor, CMS API, and published pages. Outputs plain HTML/CSS — no framework in published output.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/` — 2,433 TypeScript files, Bun runtime, MIT

## Documents

| # | Document | What You'll Master |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | Single-process architecture, core engine, 3-layer publishing, key decisions |
| 01 | [Core Engine](01-architecture.md) | NodeTree, mutations, module engine, 3-layer pipeline, Visual Components |
| 03 | [Publisher](03-publisher.md) | Static artifacts, LRU cache, hole runtime, CSS concatenation order |
| 04 | [Plugin System](04-plugin-system.md) | QuickJS-WASM sandbox, 23 capabilities, Bun.Worker isolation |
| 06 | [AI Runtime](06-ai-runtime.md) | HTTP-only drivers, shared tool loop, 35 tools, dual-endpoint bridge |

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.Instatic
python3 build.py .
```
