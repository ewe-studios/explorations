---
source_location: https://github.com/anolilab/lunora (commit 50b6008de10020411419a477efedaea505f4a54d)
repository: git@github.com:anolilab/lunora.git
explored_at: 2026-10-27
documentation_goal: Explain how Lunora uses every Cloudflare service, how it shards, how its change log and R2 archive work, and what is good and bad about the design.
---

# Spec: Lunora Documentation

## 1. Source

| Property | Value |
|----------|-------|
| Location | https://github.com/anolilab/lunora, shallow clone at commit 50b6008 |
| Language | TypeScript (pnpm monorepo, 59 packages) |
| Version | Alpha. Root package.json is 0.0.0 |
| License | Functional Source License identifier FSL-1.1-Apache-2.0 (source available, becomes Apache 2.0 after two years) |
| Main file studied | packages/do/src/shard-do.ts (18,454 lines) |

## 2. What Lunora Is

Lunora is a backend framework for Cloudflare. A Durable Object with its own SQLite database stores the data and also holds the WebSocket connections of the clients that read it. Codegen turns a schema and typed functions into a Worker that routes each call to a Durable Object.

## 3. Documentation Goal

1. Know every Cloudflare service Lunora uses, and why.
2. Know how data is sharded, and what is missing (resharding).
3. Know the change log, the epoch, `_commitSeq` and the R2 archive, with all crash windows.
4. Know replicas, relays, D1 bookmarks and read scaling.
5. Know every trick and optimization.
6. Know the strengths, weaknesses and risks, ranked.
7. Understand every Cloudflare concept that the pages use, with no earlier Cloudflare knowledge, and find the Cloudflare documentation file for each claim.

## 4. Documentation Structure

```
[src.lunora]/lunora/
├── exploration.md
├── spec.md
├── build.py
├── styles.css
├── markdown/   (README.md, 00 to 16)
└── html/
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | markdown/README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-architecture.md | DONE |
| 3 | 02-cloudflare-services.md | DONE |
| 4 | 03-worker-runtime.md | DONE |
| 5 | 04-sharding-design.md | DONE |
| 6 | 05-shard-durable-object.md | DONE |
| 7 | 06-change-log-cdc.md | DONE |
| 8 | 07-replication-and-read-scaling.md | DONE |
| 9 | 08-storage-tiers-and-archive.md | DONE |
| 10 | 09-d1-global-and-hyperdrive.md | DONE |
| 11 | 10-realtime-engine.md | DONE |
| 12 | 11-alarms-queues-workflows.md | DONE |
| 13 | 12-optimizations-and-tricks.md | DONE |
| 14 | 13-migrations-and-schema.md | DONE |
| 15 | 14-testing-observability-deployment.md | DONE |
| 16 | 15-critical-assessment.md | DONE |
| 17 | 16-glossary.md | DONE |
| 18 | exploration.md | DONE |
| 19 | Grandfather review | DONE (citations checked by script; some items not verified, see exploration.md) |
| 20 | Generate HyperText Markup Language (HTML) | DONE |
| 21 | Concept explanations for readers new to Cloudflare (D1 bookmarks, Durable Objects, consistency words, every service, Lunora terms) | DONE |

## 6. Build System

Run `python3 -I build.py .` inside this directory. It writes `html/`. Run `python3 -I build.py` in the parent `[src.lunora]` to build the collection index. No dependencies beyond the Python standard library. This copy of `build.py` escapes raw angle brackets and processes inline markup inside table cells.

## 7. Quality Requirements (Iron Rules)

1. Detailed sections with code snippets.
2. Teach key facts, principles and ideas quickly.
3. Clear articulation.
4. Mermaid diagrams (minimum 2 per document).
5. Good visual assets.
6. Generated HyperText Markup Language (HTML).
7. Cross-references.
8. Source path references.
9. Aha moments.
10. Navigation: index plus prev/next buttons.

Also: ASD-STE100 Simplified Technical English. Abbreviations are spelled out where used. The code wins over the docs.

## 8. Expected Outcome

A reader can explain the Lunora design, decide what to copy and what to avoid, and find the code for any claim.

## 9. Resume Point

All documents are written. To continue: re-run the citation check on any changed chapter, then rebuild the HyperText Markup Language (HTML).
