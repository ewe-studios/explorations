---
source_location: /Users/darkvoid/GitSources/src.s3/celld
repository: git@github.com:denoland/celld.git
explored_at: 2026-11-10
commit: 8702f6d186c77698dbfb30cf29e3b1922a88f008
version: 0.6.3
language: Rust, TypeScript, SQL
---

# Spec: celld, the self-hosted Cloudflare Workers platform

## 1. Source

| Property | Value |
|----------|-------|
| Location | `/Users/darkvoid/GitSources/src.s3/celld` |
| Remote | `git@github.com:denoland/celld.git` |
| Commit | `8702f6d186c77698dbfb30cf29e3b1922a88f008` |
| Version | `0.6.3` (`Cargo.toml`) |
| License | Apache-2.0 |
| Source of truth | The source at the commit above. Every claim names a file and, where useful, a line. |

## 2. What the Project Is

celld is a self-hosted implementation of the Cloudflare Workers platform. An operator runs a fleet of nodes on machines that the operator controls. The nodes share one object store bucket. The bucket is the only durable substrate.

A node embeds the V8 JavaScript engine. It runs Worker modules and Durable Objects. A Durable Object instance is a cell. A cell has a name, one owner node at a time, its own SQLite database, and a log of committed transactions in the bucket.

The platform also provides Key-Value (KV), Queues, D1 (SQL databases), R2 (object storage), Workflows, Cron Triggers, static assets, dynamic Workers, and experimental containers and sandboxes.

This exploration is full-scope. It does not stop at storage. It covers the repository, the runtime, every service, ownership and failure, security, operations, and limits. It ends with two comparisons against the Resonate S3 server: the storage model, and work placement.

## 3. Documentation Goal

A reader must be able to:

1. State what a node, a fleet, a cell, and a deployment are, and how they relate.
2. Map the repository and name the crate that holds each concern.
3. Trace one Hypertext Transfer Protocol (HTTP) request from the worker listener to a cell, local or remote.
4. Explain how a Worker module is loaded into V8 and how events enter and leave the isolate.
5. Explain the Durable Object lifecycle, storage Application Programming Interface (API), alarms, WebSockets, and facets.
6. Explain the Lite Transaction (LTX) log, the fencing epoch, the durability proof, and restore.
7. Explain leases, ownership records, takeover, self-fencing, rebalancing, and pressure.
8. Explain how KV, Queues, D1, R2, Workflows, Cron Triggers, static assets, dynamic Workers, containers, and sandboxes are built on cells and on the bucket.
9. Explain deployment, the command line interface, telemetry, testing, and the security model.
10. Name the documented limits and the gaps against Cloudflare.
11. Compare the Resonate S3 origin document with the celld cell, point by point.
12. Compare Resonate task distribution with celld cell placement, and state where each system shards.

## 4. Documentation Structure

```
src.durable-streams/src.celld/
├── spec.md
├── markdown/
│   ├── README.md                          ← index, reading order, and the one-page thesis
│   ├── 00-overview.md                     ← node, fleet, cell, deployment, and two promises
│   ├── 01-repository-map.md               ← every crate, module, package, and document
│   ├── 02-system-architecture.md          ← listeners, routing, deployment, control plane
│   ├── 03-the-worker-runtime.md           ← V8, bundles, events, bindings, gates, limits
│   ├── 04-durable-objects.md              ← identity, lifecycle, storage API, alarms, WebSockets, facets
│   ├── 05-cell-storage-and-durability.md  ← SQLite, LTX, epochs, proofs, restore, GC
│   ├── 06-ownership-failure-and-rebalancing.md ← lease, epoch, takeover, self-fence, pressure
│   ├── 07-the-services.md                 ← KV, Queues, D1, R2, Workflows, Cron, assets, containers
│   ├── 08-deployment-and-operations.md    ← wrangler subset, deploy, CLI, diagnose, telemetry, tests
│   ├── 09-security-and-tenancy.md         ← fleet trust, peer auth, operator API, naming, limits
│   ├── 10-limitations-and-compatibility.md← Cloudflare gaps and operational risks
│   ├── 11-resonate-s3-and-celld.md        ← the storage comparison
│   ├── 12-work-placement-and-sharding.md  ← distribution and sharding in both systems
│   └── 13-glossary.md                     ← every term, spelled out
└── html/                                  ← generated
```

## 5. Tasks

| Phase | Task | Status |
|-------|------|--------|
| 1 | Read repository metadata, README, guarantees, and services documents | DONE |
| 2 | Research repository structure, architecture, routing, deployment, and CLI | DONE |
| 3 | Research the Worker runtime: V8, events, bindings, gates, limits | DONE |
| 4 | Research Durable Objects: lifecycle, storage, alarms, WebSockets, facets | DONE |
| 5 | Research storage and durability: LTX, epochs, proofs, restore, GC | DONE |
| 6 | Research services: KV, Queues, D1, R2, Workflows, Cron, assets, containers | DONE |
| 7 | Research security, tenancy, telemetry, testing, and limitations | DONE |
| 8 | Research Resonate work distribution and sharding | DONE |
| 9 | Write README and documents 00 to 02 | DONE |
| 10 | Write documents 03 to 06 | DONE |
| 11 | Write documents 07 to 10 | DONE |
| 12 | Write documents 11 to 13 | DONE |
| 13 | Grandfather review against source for every document | DONE (every document revised against commit `8702f6d`) |
| 14 | Fresh-reader review for every document | DONE (every document revised from a fresh-reader report; documents 05, 10, and 13 also passed a second pass) |
| 15 | Build HTML, verify navigation, and update the parent index | DONE |

## 6. Build System

```bash
cd /Users/darkvoid/Dev/explorations/src.durable-streams
python3 -I build.py .
```

The root build discovers every direct child directory that matches `src.*` or `tanstackdb` and has a `markdown` tree. It writes the matching `html` tree, copies the parent `styles.css`, and builds `spec.md` when present. No root build change is needed once `src.celld/markdown/` exists.

## 7. Quality Requirements

The ten Iron Rules from `markdown_engineering/documentation_directive.md` apply:

1. Detailed sections with code snippets from the source.
2. The thesis in the first paragraph.
3. One idea per sentence, in ASD Simplified Technical English. Spell out every abbreviation at first use in each file.
4. At least two Mermaid diagrams per document.
5. Tables, trees, and diagrams as visual assets.
6. Generated HTML with consistent navigation.
7. Cross-references between documents.
8. Source paths and line numbers for implementation claims.
9. At least one Aha moment per document.
10. An index and previous and next navigation on every page.

Each deep-dive document must hold at least ten code snippets. Every claim about behaviour must cite the celld source at the commit above or one of its documents.

## 8. Expected Outcome

After reading, an engineer can explain how celld turns a bucket into a Workers platform. The engineer can state why Resonate commits a whole origin document for one promise, while celld commits a SQLite transaction and appends a log segment under a fencing epoch. The engineer can also state which model acknowledges a write sooner, which recovers faster, which costs more per write, and how each system decides which machine does the work.
