# durable-streams -- Project Spec

## Source Codebase

- **Durable Streams** — `/Users/darkvoid/GitSources/src.DurableStreams/durable-streams/`
  - A pnpm monorepo (Node >= 22, pnpm 10.25). TypeScript core plus a Go
    production server and ten client libraries. Status: beta, all packages
    pre-1.0.
  - Key versions: `@durable-streams/client` 0.2.7, `@durable-streams/server`
    0.3.9, `packages/caddy-plugin` 0.1.0 (Go 1.25, module
    `github.com/durable-streams/durable-streams/packages/caddy-plugin`).
  - The protocol is the source of truth: `PROTOCOL.md` (a ~93 KB
    specification). Implementations live in `packages/`.

## What This Explores

Durable Streams are the "data primitive for the agent loop": persistent,
addressable, resumable event streams over plain HTTP. This exploration reads
the protocol specification and the production server (the Go Caddy plugin),
the TypeScript development server, the Cloudflare Workers + Durable Objects
server, the reference client, and the higher-level packages to explain how a
client gets a byte-ordered, offset-addressed, replayable log with exactly-once
appends and live tailing, and how the same primitive composes into durable
state, a reactive stream database, a stream filesystem, and CRDT / AI-token
transports.

## Documentation Structure

```
durable-streams/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index
    ├── 00-overview.md
    ├── 01-stream-model.md
    ├── 02-catch-up-reads.md
    ├── 03-live-tailing.md
    ├── 04-appends.md
    ├── 05-idempotent-producers.md
    ├── 06-json-mode.md
    ├── 07-forking.md
    ├── 08-store-layer.md
    ├── 09-caddy-server.md
    ├── 10-cloudflare-server.md
    ├── 11-durable-proxy.md
    ├── 12-durable-state.md
    ├── 13-stream-db.md
    ├── 14-stream-fs.md
    ├── 15-yjs-ai-transports.md
    └── 16-clients-conformance-deployment.md
```

## Quality Requirements (Iron Rules)

1. H1 first line `# NN -- Title: subtitle`, no YAML frontmatter.
2. At least two Mermaid diagrams per document, each preceded on its own line
   by `%%{init: {"flowchart": {"htmlLabels": false}}}%%` (or the sequence
   variant `%%{init: {"sequence": {"htmlLabels": false}}}%%`); every node and
   edge label quoted; no unquoted parentheses in labels.
3. At least three code snippets per document, fenced and attributed
   `// Source: <path>, lines N-M` (Go and TypeScript both use `//`; use
   `# Source:` for TOML or plain-text) immediately after the opening fence.
   Snippets must be verbatim, and the cited line range must be exactly the
   first through last shown line.
4. Exactly one `## The Aha Moment` per document, first line `**Aha:**`.
5. At least two cross-references in a `## Cross-References` section.
6. Footer chain 00→01→…→16→README; no orphan pages.
7. ASD-STE100 Simplified Technical English; spell out abbreviations at first
   use (HTTP, SSE, CDN, JSON, CRDT, LLM, CLI, API, WAL, RPC, etc.). Each
   document is standalone, so expand at first use within each document.
8. Every factual claim verified against source; the source wins over the
   README where they disagree.

## Verification

Authored by an agent that read the protocol and the relevant packages in full,
verified every cited line range verbatim, and confirmed the build runs with no
error:

```
cd /Users/darkvoid/Dev/explorations && python3 build.py durable-streams
```
