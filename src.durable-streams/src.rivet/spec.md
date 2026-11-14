---
source_location: /Users/darkvoid/GitSources/src.Rivet
core_repository: /Users/darkvoid/GitSources/src.Rivet/rivet
repository: git@github.com:rivet-dev/rivet.git
explored_at: 2026-11-14
commit: 4a58441c50d1ce20d8803cfb1ab7c3285a5141d6
version: 2.3.31
language: Rust, TypeScript, Python, Swift, SQL
---

# Spec: Rivet, Actors as durable processes

## 1. Source

| Property | Value |
|----------|-------|
| Snapshot location | `/Users/darkvoid/GitSources/src.Rivet` |
| Core repository | `/Users/darkvoid/GitSources/src.Rivet/rivet` |
| Remote | `git@github.com:rivet-dev/rivet.git` |
| Commit | `4a58441c50d1ce20d8803cfb1ab7c3285a5141d6` |
| Commit date | 2026-10-29 |
| Version | `2.3.31` (`Cargo.toml`, `[workspace.package]`) |
| License | Apache-2.0 |
| Source of truth | The source at the commit above. Every claim names a file and, where useful, a line. |

The snapshot also holds seven sibling repositories outside the core repository:
`agents`, `agentos`, `workflows`, `dynamic-apps`, `open-orange`,
`durable-object-migrator`, and `website`. Document 17 covers them from their own
source and states the revision of each. The core repository is the subject of documents 00
to 16 and 18 to 19.

## 2. What the Project Is

Rivet is an orchestrator for stateful processes that it calls Actors. An Actor
has a key, durable state, a Structured Query Language (SQLite) database, a queue,
realtime connections, and a schedule. Many Actors share one node process. An idle
Actor writes its state and unloads. A later request starts it again on any node.

The control plane is one Rust binary. It schedules Actors, routes Hypertext
Transfer Protocol (HTTP) and WebSocket traffic to them, and stores durable state.
The product description says that it has no separate database and no message bus,
and that the only external durable store is an object storage bucket. The
open-source code in this commit does not match that description in full.
UniversalDB has two drivers: RocksDB and PostgreSQL. The cold tier that writes
SQLite pages to object storage is Enterprise-only. Instances coordinate through
UniversalDB and an optional NATS pub/sub service. The documents state the product
claim and the open-source configuration separately, and they mark which is
which.

The repository also holds RivetKit, the Software Development Kit (SDK) that a
developer uses to define an Actor. RivetKit runs on Node.js, Bun, and Rust. It
also holds the dashboard, the documentation, examples, and the internal design
documents.

## 3. Documentation Goal

A reader must be able to:

1. State what an Actor, a node, a runner, the control plane, and a namespace are,
   and how they relate.
2. Map the repository and name the crate or package that holds each concern.
3. Trace one request from a client to a running Actor and back, across the
   gateway, the control plane, and the runner.
4. Explain how Pegboard schedules an Actor, how a runner pool holds capacity, and
   how the runner protocol carries an Actor start, a request, and a stop.
5. Explain how Gasoline replays a workflow, how an activity records its result,
   and how a code change is versioned.
6. Explain how an Actor's SQLite database is stored, how a write reaches two of
   three control plane instances, and how pages move to the bucket.
7. Explain UniversalDB, the storage abstraction under every durable record, the
   RocksDB and PostgreSQL drivers, and where object storage enters the system.
8. Explain Epoxy and its per-key Fast Paxos protocol, including replicas, ballots, and
   multi-region commit.
9. Explain how Guard routes HTTP and WebSocket traffic, how it holds a
   hibernatable connection, and how it recovers a connection after a wake.
10. Explain how RivetKit TypeScript and RivetKit Rust define an Actor and how
    the client speaks to it.
11. Explain authentication, namespaces, tokens, and the authorization policy.
12. Explain telemetry, metrics, logs, profiling, configuration, and the command
    line interface.
13. Explain the deployment shapes: self-host, Rivet Cloud, and bring your own
    cloud.
14. State the benchmark method, the measured limits, and the operational risks.
15. Name the products in the snapshot and state what each adds to an Actor.
16. Compare Rivet with Cloudflare Durable Objects, Kubernetes, and the durable
    promise systems in this parent directory.

## 4. Documentation Structure

```
src.durable-streams/src.rivet/
├── spec.md
├── markdown/
│   ├── README.md                          ← index, reading order, and thesis
│   ├── 00-overview.md                     ← what Rivet is, the Actor, and the promises
│   ├── 01-repository-map.md               ← every workspace crate, package, and sibling
│   ├── 02-control-plane-architecture.md   ← binary, services, pools, config, bootstrap
│   ├── 03-actors-and-lifecycle.md         ← identity, create, start, sleep, wake, destroy
│   ├── 04-pegboard-scheduling.md          ← runner pools, allocation, envoy, protocol
│   ├── 05-gasoline-durable-execution.md   ← workflows, activities, replay, history
│   ├── 06-actor-sqlite-and-depot.md       ← per-Actor SQLite, VFS, log, page index
│   ├── 07-universaldb-and-buckets.md      ← key space, transactions, drivers, storage
│   ├── 08-epoxy-multi-region.md           ← Fast Paxos, replicas, ballots, commit
│   ├── 09-guard-routing.md                ← HTTP and WebSocket routing, TLS, proxy
│   ├── 10-realtime-and-hibernation.md     ← connections, events, sleep, restore
│   ├── 11-rivetkit-typescript.md          ← actor definition, registry, client, server
│   ├── 12-rivetkit-rust-and-clients.md    ← Rust runtime and the other SDKs
│   ├── 13-auth-and-namespaces.md          ← tokens, JWT, namespaces, policy, keys
│   ├── 14-observability-and-operations.md ← telemetry, metrics, logs, profiling, CLI
│   ├── 15-deployment-and-self-hosting.md  ← instances, nodes, storage, cloud, BYOC
│   ├── 16-tests-benchmarks-and-limits.md  ← test harness, snapshots, measurements
│   ├── 17-the-product-family.md           ← agents, workflows, agentOS, dynamic apps
│   ├── 18-comparisons.md                  ← Durable Objects, Kubernetes, Resonate
│   └── 19-glossary.md                     ← every term, spelled out
└── html/                                  ← generated
```

## 5. Tasks

| Phase | Task | Status |
|-------|------|--------|
| 1 | Read the instruction set, the directive, and the core README | DONE |
| 2 | Map the workspace, engine packages, SDKs, and sibling repositories | DONE |
| 3 | Research the control plane, services, config, and bootstrap | DONE |
| 4 | Research Actor identity, lifecycle, rescheduling, and sleep | DONE |
| 5 | Research Pegboard scheduling, runner pools, envoy, and protocol | DONE |
| 6 | Research Gasoline workflows, activities, replay, and history | DONE |
| 7 | Research Actor SQLite, the virtual file system, Depot, and the log | DONE |
| 8 | Research UniversalDB, drivers, buckets, and object storage | DONE |
| 9 | Research Epoxy, Fast Paxos, replicas, and multi-region operation | DONE |
| 10 | Research Guard routing, TLS, WebSocket proxying, and hibernation | DONE |
| 11 | Research RivetKit TypeScript, Rust, Python, and Swift | DONE |
| 12 | Research authentication, namespaces, policy, and keys | DONE |
| 13 | Research telemetry, metrics, logs, CLI, deployment, and tests | DONE |
| 14 | Research the seven sibling repositories | DONE |
| 15 | Write README and documents 00 to 05 | DONE |
| 16 | Write documents 06 to 10 | DONE |
| 17 | Write documents 11 to 15 | DONE |
| 18 | Write documents 16 to 19 and the parent index entry | DONE |
| 19 | Grandfather review against the pinned source for every document | DONE |
| 20 | Fresh-reader review for every document, and fix every reported gap | DONE |
| 21 | Build the HyperText Markup Language (HTML) tree and check every link | DONE |

## 6. Build System

```bash
cd /Users/darkvoid/Dev/explorations/src.durable-streams
python3 -I build.py .
```

The parent build discovers every direct child directory that has a `markdown`
tree. It writes the matching `html` tree, copies the parent `styles.css`, and
adds the page to the parent index. No build change is needed after
`src.rivet/markdown/` exists. The parent `index.md` gets one row and one link for
this exploration.

## 7. Quality Requirements

The ten Iron Rules from `markdown_engineering/documentation_directive.md` apply:

1. Detailed sections with code snippets from the source.
2. The thesis in the first paragraph.
3. One idea per sentence, in ASD Simplified Technical English. Spell out every
   abbreviation at first use in each file.
4. At least two Mermaid diagrams per document.
5. Tables, trees, and diagrams as visual assets.
6. Generated HTML with consistent navigation.
7. Cross-references between documents.
8. Source paths and line numbers for implementation claims.
9. At least one Aha moment per document.
10. An index and previous and next navigation on every page.

Each deep-dive document must hold at least ten code snippets. Every claim about
behaviour must cite the Rivet source at the commit above, the sibling source at
the revision named in document 17, or one of their documents.

## 8. Expected Outcome

After reading, an engineer can explain how Rivet turns one binary and a durable
store into a scheduler for billions of Actors. The engineer can state how an Actor is
placed, how its SQLite pages become durable, how a workflow replays after a
crash, how Guard finds the Actor, and how RivetKit hides the protocol. The
engineer can also state where Rivet is stronger than Cloudflare Durable Objects
and Kubernetes, and where it is weaker.

## 9. Resume Point

If work stops, read this file and the task table. The task table shows the last
completed phase. Continue with the first `TODO` row. Each document is complete
only after the grandfather review, the fresh-reader review, and a successful
HTML build.

## 10. Corrections made during research

The first plan contained three errors. The source corrected them. Every document
must follow the corrected statement.

| Error in the first plan | What the source says | Evidence |
|-------------------------|----------------------|----------|
| Epoxy runs the EPaxos protocol | Epoxy runs single-decree Fast Paxos, one instance per key. The `PreAccept` name comes from EPaxos ancestry and is standard Paxos Phase 2a. EPaxos is a reference for cross-key ordering only. | `engine/packages/epoxy/README.md:3`, `README.md:27`, `README.md:42` |
| UniversalDB has no separate database | UniversalDB has two drivers: RocksDB and PostgreSQL. The default open-source store is a local file system directory. | `engine/packages/universaldb/src/driver/mod.rs` |
| Object storage is the only external durable store | The cold tier that writes SQLite pages to object storage is Enterprise-only. The open-source build does not write it. NATS is an optional cross-process pub/sub service. | `docs-internal/engine/oss-to-ee-upgrade.md` |
| The snapshot holds six sibling repositories | The snapshot holds seven sibling repositories: `agents`, `agentos`, `workflows`, `dynamic-apps`, `open-orange`, `durable-object-migrator`, and `website`. | `git log -1` in each directory |

## 11. Verification record

The exploration is complete as of 2026-11-14. These checks ran against the
pinned checkout at commit `4a58441c50d1ce20d8803cfb1ab7c3285a5141d6`.

| Check | Result |
|-------|--------|
| Cited code blocks compared with the source | 668 blocks. 640 are byte-exact. 28 differ only in indentation, because the document removes a leading tab. One block, `engine/packages/depot/README.md:131-149` in document 00, drops the inner fences and one blank line, and the text says so. No content error remains. |
| Inline citations | 3,799 citations checked. None names a path that does not exist, except bare file names that the surrounding text resolves. None names a line beyond the end of its file. |
| Structure per document | Every deep dive has at least two Mermaid diagrams, at least ten code snippets, and at least one Aha callout. Every fence is balanced and every table row has a consistent pipe count. |
| Sentence length | No prose sentence in any document is longer than 30 words. |
| Internal links | Every relative link in the Markdown tree resolves, including the four cross-exploration links to `src.electric`, `src.resonate`, `src.celld`, and `tanstackdb`. |
| HTML build | `python3 -I build.py .` writes 22 pages and one index under `src.rivet/html/`. Every relative `href` and `src` in the whole `src.durable-streams` HTML tree resolves. |
