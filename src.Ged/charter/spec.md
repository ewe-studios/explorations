# charter -- Project Spec

## Source Codebase

- **charter** — `/Users/darkvoid/GitSources/src.Ged/charter/`
  - A Go library and a command-line tool (CLI) for contract-first application
    programming interfaces (APIs) on Cloudflare Workers. One contract becomes
    an OpenAPI and an AsyncAPI spec, Fern software development kits (SDKs),
    Model Context Protocol (MCP) tools, authentication, rate limits, and
    streaming. The same API can be written in Go (with Huma) or in TypeScript
    (with oRPC, shipped as `charter-lib`).
  - Go 1.27.1, module `github.com/joeblew999/charter`. The Go library is built
    with TinyGo and workers-go to a WebAssembly (Wasm) Worker; the TypeScript
    library is bundled with esbuild. Deployments use the Cloudflare REST API, no
    wrangler and no Node. Local development runs the Worker on plain workerd.
  - Key dependencies: BurntSushi/toml, coder/websocket, danielgtaylor/huma/v2,
    evanw/esbuild, go-jose/go-jose/v4, syumai/workers-go, modernc.org/sqlite.
    The TypeScript library builds on @orpc/contract, @orpc/server,
    @orpc/openapi, @modelcontextprotocol/server, jose, and zod.

## What This Explores

charter is one idea pushed to its conclusion: the contract is the single source
of truth, and everything a client or a server needs falls out of it. A Go
`huma.Operation` (or a TypeScript oRPC procedure) declares an operation once.
From that one declaration charter derives the OpenAPI and AsyncAPI documents,
the Fern SDKs, the MCP tool list, the authentication and rate-limit checks, the
problem-details refusals, and the streaming transport. This exploration reads
the library, the CLI, the TypeScript counterpart, and the example APIs to
explain how a contract travels from a struct tag or a zod schema all the way to
a typed SDK and a live WebSocket channel.

## Documentation Structure

```
charter/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index
    ├── 00-overview.md
    ├── 01-architecture.md
    ├── 02-humaworkers.md
    ├── 03-serve.md
    ├── 04-auth.md
    ├── 05-jwt-discovery.md
    ├── 06-mcp.md
    ├── 07-ratelimit-problem.md
    ├── 08-streaming.md
    ├── 09-transport.md
    ├── 10-d1-promise-web.md
    ├── 11-project-configuration.md
    ├── 12-cli.md
    ├── 13-build-deploy-local.md
    ├── 14-spec-sdks-fern.md
    ├── 15-check-test-twins.md
    ├── 16-ci-scaffold-repo.md
    └── 17-typescript-library-examples.md
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 00 | Overview: contract-first APIs on Cloudflare Workers | DONE |
| 01 | Architecture: packages, layers, and the dependency graph | DONE |
| 02 | humaworkers: Huma on Workers | DONE |
| 03 | serve: the standard server | DONE |
| 04 | auth: authentication middleware | DONE |
| 05 | jwt-discovery: JWT verification and discovery documents | DONE |
| 06 | mcp: the Model Context Protocol endpoint | DONE |
| 07 | ratelimit-problem: rate limiting and problem details | DONE |
| 08 | streaming: follow, hub, fragments, and asyncapi | DONE |
| 09 | transport: the platform transport and WebSocket adapter | DONE |
| 10 | d1-promise-web: data and platform helpers | DONE |
| 11 | project-configuration: charter.toml | DONE |
| 12 | cli: the command dispatcher | DONE |
| 13 | build-deploy-local: build, deploy, and local serve | DONE |
| 14 | spec-sdks-fern: spec generation and the SDKs | DONE |
| 15 | check-test-twins: checks, tests, and the twin comparison | DONE |
| 16 | ci-scaffold-repo: CI, scaffolding, and repo management | DONE |
| 17 | typescript-library-examples: charter-lib and the example APIs | DONE |

## Build System

The shared build script is `build.py` at the repository root
(`/Users/darkvoid/Dev/explorations/`). It is standard-library-only Python
3.12+ with no dependencies. It converts markdown to HTML, embeds a Mermaid
client-side loader, a dark/light theme toggle with local storage, and prev/next
navigation, and writes an index page.

```
cd /Users/darkvoid/Dev/explorations && python3 build.py src.Ged/charter
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
6. Footer chain 00→01→…→17→README; no orphan pages.
7. ASD-STE100 Simplified Technical English; spell out abbreviations at first
   use (HTTP, SSE, CDN, JSON, JWT, MCP, CLI, API, WAL, RPC, etc.). Each
   document is standalone, so expand at first use within each document.
8. Every factual claim verified against source; the source wins over the
   README where they disagree.

## Expected Outcome

After reading the series in order, a reader can explain how a contract-first
charter API works end to end: how a Go struct tag or a TypeScript zod schema
becomes an operation, how that operation becomes the OpenAPI and AsyncAPI
specs, how the specs become Fern SDKs and MCP tools, how a caller is
authenticated and rate-limited, how a client receives a gap-free live stream,
how the Worker is built and deployed with no wrangler, and how the same API in
Go and TypeScript is proven to answer identically.

## Resume Point

The source was read in full; every cited line range below was captured
verbatim. To continue or extend, add a document to `markdown/`, register it in
this file and in `markdown/README.md`, re-run `python3 build.py src.Ged/charter`,
and re-run the grandfather review (names, numbers, flows, coverage) against the
source at `/Users/darkvoid/GitSources/src.Ged/charter/`.
