---
source_location: /Users/darkvoid/GitSources/src.Electric/electric and /Users/darkvoid/GitSources/src.Electric/durable-streams
repository: git@github.com:electric-sql/electric.git and git@github.com:durable-streams/durable-streams.git
explored_at: 2026-11-07
commit: electric 3f9809fb0, durable-streams 461b4026
language: TypeScript, Rust, Elixir, Go, Python, C#, Swift, PHP, Java, Ruby
---

# Spec: Electric and Durable Streams Documentation

## 1. Source

| Property | Value |
|----------|-------|
| Electric location | `/Users/darkvoid/GitSources/src.Electric/electric` |
| Electric remote | `git@github.com:electric-sql/electric.git` |
| Electric commit | `3f9809fb0` (Publish the Electric image to electricsql/electric again) |
| Durable Streams location | `/Users/darkvoid/GitSources/src.Electric/durable-streams` |
| Durable Streams remote | `git@github.com:durable-streams/durable-streams.git` |
| Durable Streams commit | `461b4026` (ci: Version Packages #409) |
| Package manager | pnpm 10.12.1 (Electric), pnpm 10.25.0 (Durable Streams) |
| Electric license | Apache 2.0 |
| Durable Streams license | MIT |
| Source of truth | The source code at the two commits above. Every claim names a file and, where useful, a line. |

There are two repositories but one system. The Electric repository contains the
Electric Agents platform, which stands on the Durable Streams protocol. The
Durable Streams repository contains the protocol and its implementations. This
exploration treats the two repositories as one architecture and states which
repository each component comes from.

## 2. What the Project Is

Electric is two products in one repository:

1. **Electric Sync** — an Elixir service that reads the PostgreSQL logical
   replication stream and serves read-only subsets, called Shapes, over HTTP.
2. **Electric Agents** — a TypeScript platform for durable artificial
   intelligence agents. Each agent is an entity with one append-only event
   stream. A server materializes the stream into typed collections, wakes the
   entity handler, and records the agent's work back into the same stream.

Durable Streams is the storage and delivery primitive under Electric Agents. It
is an HTTP protocol for persistent, addressable, append-only byte streams with
offset-based resumable reads and live tailing.

## 3. Documentation Goal

1. Know how the two repositories fit together and what each package does.
2. Know, in complete detail, how the agent runtime sets up a durable stream.
3. Know where stream bytes and stream metadata are stored, on every supported
   deployment.
4. Know the full request path from a user message to an agent run and back.
5. Know the wake model: webhook wakes, pull wakes, the scheduler, signals, and
   entity lifecycle states.
6. Know the entity stream event schema and how the runtime materializes it into
   TanStack DB collections.
7. Know the Durable Streams protocol: offsets, JSON mode, producers, forking,
   live reads, subscriptions, and the state protocol.
8. Know the server implementation: PostgreSQL registry, stream client, routes,
   projector, bridges, sandboxes, and tenancy.
9. Know each remaining package at a depth that permits maintenance.
10. Know the limits and open questions in the current code.

## 4. Documentation Structure

```
src.durable-streams/src.electric/
├── spec.md
├── markdown/
│   ├── README.md                   ← master index with package sublinks
│   ├── 00-overview.md
│   ├── 01-repository-map.md
│   ├── 02-system-architecture.md
│   ├── 03-the-durable-stream-primitive.md
│   ├── 04-how-the-runtime-sets-up-a-stream.md   ← the key document
│   ├── 05-wake-dispatch-and-entity-lifecycle.md
│   ├── 06-security-tenancy-and-permissions.md
│   ├── 07-critical-assessment.md
│   ├── 08-glossary.md
│   ├── agents-runtime/
│   │   ├── README.md
│   │   ├── 00-overview.md
│   │   ├── 01-entity-model.md
│   │   ├── 02-runtime-handler-and-registration.md
│   │   ├── 03-wake-processing.md
│   │   ├── 04-entity-stream-db.md
│   │   ├── 05-handler-context.md
│   │   ├── 06-agent-loop-and-tools.md
│   │   ├── 07-context-timeline-and-compaction.md
│   │   ├── 08-sandbox.md
│   │   └── 09-server-client-signals-and-testing.md
│   ├── agents-server/
│   │   ├── README.md
│   │   ├── 00-overview.md
│   │   ├── 01-boot-and-configuration.md
│   │   ├── 02-entity-manager.md
│   │   ├── 03-entity-registry-and-postgres.md
│   │   ├── 04-stream-layout-and-stream-client.md
│   │   ├── 05-wake-registry-and-scheduler.md
│   │   ├── 06-http-routing.md
│   │   ├── 07-entity-projector.md
│   │   ├── 08-tenancy-permissions-and-principals.md
│   │   ├── 09-runners-and-sandboxes.md
│   │   ├── 10-pg-sync-and-bridges.md
│   │   └── 11-operations-testing-and-conformance.md
│   ├── durable-streams/
│   │   ├── README.md
│   │   ├── 00-overview.md
│   │   ├── 01-protocol-stream-model.md
│   │   ├── 02-protocol-operations.md
│   │   ├── 03-live-reads-and-subscriptions.md
│   │   ├── 04-node-server.md
│   │   ├── 05-storage-and-durability.md
│   │   ├── 06-typescript-client.md
│   │   ├── 07-state-protocol.md
│   │   ├── 08-server-implementations.md
│   │   ├── 09-language-clients.md
│   │   ├── 10-integrations-and-conformance.md
│   │   └── 11-laws-entities-and-models.md
│   ├── agents/
│   ├── agents-mcp/
│   ├── agents-server-ui/
│   ├── agents-desktop/
│   ├── agents-mobile/
│   ├── agents-server-conformance-tests/
│   ├── durable-streams-rust/
│   ├── electric-ax/
│   ├── sync-service/
│   ├── sync-engines/
│   ├── typescript-client/
│   ├── react-hooks/
│   ├── y-electric/
│   ├── elixir-client/
│   ├── electric-telemetry/
│   ├── start/
│   └── experimental/
└── html/                           ← generated mirror of markdown/
```

Each package directory contains a `README.md` that links back to the master
index and to its sibling documents. The Electric tree is one half of a single
site. The other half is `../src.resonate/`, and the comparison page is
`../index.md`.

## 5. Tasks

| Phase | Task | Status |
|-------|------|--------|
| 1 | Read repository metadata, package manifests, entrypoints | DONE |
| 2 | Trace the runtime stream setup path end to end | DONE |
| 3 | Trace stream storage on embedded, Docker, and external modes | DONE |
| 4 | Write master index and overview documents | DONE |
| 5 | Write agents-runtime package documents | DONE |
| 6 | Write agents-server package documents | DONE |
| 7 | Write durable-streams package documents | DONE |
| 8 | Write all remaining package documents | DONE |
| 9 | Grandfather review of every document against source | TODO |
| 10 | Fresh-reader review of every document | TODO |
| 11 | Build HTML and verify navigation | DONE |

## 6. Build System

```bash
cd /Users/darkvoid/Dev/explorations/src.durable-streams
python3 -I build.py .
```

One build generates both the Electric and the Resonate sites and the root
comparison page. The build script reads every `src.*/markdown/**/*.md`, writes
`html/**/*.html` with the same relative layout, and writes one `index.html` per
directory. It then builds `../index.md` to `../index.html`. It uses only the
Python standard library. Each directory's pages get prev/next navigation within
that directory. Each index lists that directory's child directories and
documents.

## 7. Quality Requirements

The ten Iron Rules from
`markdown_engineering/documentation_directive.md` apply without exception:

1. Detailed sections with code snippets from the source.
2. Teach key facts quickly; the first paragraph is the thesis.
3. Clear articulation; one idea per sentence.
4. At least two Mermaid diagrams per document.
5. Good visual assets: tables, trees, diagrams.
6. Generated HTML with consistent navigation.
7. Cross-references between documents.
8. Source paths and line numbers for implementation claims.
9. At least one Aha moment per document.
10. Index plus prev/next navigation on every page.

Additional rules from `CLAUDE.md`:

- Write in ASD-STE100 Simplified Technical English.
- Spell out every abbreviation at the point of first use.
- Never invent a name, number, or flow. Verify against the source.

## 8. Expected Outcome

After reading, an engineer can:

- Explain how an agent entity becomes a durable stream, byte for byte.
- State where the stream bytes and metadata live in each deployment.
- Trace a wake from the Durable Streams server to an agent handler and back.
- Describe the entity event schema and the materialization into TanStack DB.
- Explain the Durable Streams protocol and implement a compatible client.
- Maintain or extend every package in both repositories.

## 9. Resume Point

Document writing is complete. The remaining work is the grandfather review,
the fresh-reader review, and the final build. To change a page, edit Markdown
in this consolidated tree under `src.durable-streams/src.electric/markdown/` and
run `python3 -I build.py .` from `/Users/darkvoid/Dev/explorations/src.durable-streams`.
Do not edit generated HTML by hand.
