---
source_location: /Users/darkvoid/GitSources/src.humanlayer/humanlayer
repository: git@github.com:humanlayer/humanlayer.git
explored_at: 2026-11-15
commit: 2f66a45b186d88488fa356862be0117db1ccae9f
language: TypeScript, TypeScript JSX, Rust, SQL, Shell
license: Apache-2.0 (MIT for the vendored folders that NOTICE lists)
---

# Spec: HumanLayer, the agent steering platform

## 1. Source

The repository root is `/Users/darkvoid/GitSources/src.humanlayer/humanlayer`.
The parent directory `/Users/darkvoid/GitSources/src.humanlayer` is only a
container. It is not a Git repository. Every path in this exploration is
relative to the repository root named above unless the text says otherwise.

| Item | Value |
|------|-------|
| Remote | `git@github.com:humanlayer/humanlayer.git` |
| Commit | `2f66a45b186d88488fa356862be0117db1ccae9f` |
| Commit date | 2026-10-23 |
| Commit subject | `Merge pull request #1132 from humanlayer/dexhorthy-patch-13` |
| License | Apache License 2.0; copyright QueryTale, Inc. doing business as HumanLayer |
| Vendored exceptions | `vendor/effect-orpc`, `packages/y-durable-streams`, `tools/oxlint/anti-slop` keep the MIT license that `NOTICE` lists |
| Root manifest | `package.json`, private, package manager `bun@1.3.14`, engine `node >= 24` |

The source of truth is the code at this commit. Every claim names a file and,
where useful, a line number. A citation has the form `path:line` or
`path:line-line`. All line numbers refer to the commit above.

## 2. What the Project Is

HumanLayer runs coding agents on a user's machines and lets a team steer them
from one place.

The product has three parts that work together:

1. A **daemon** runs on each host. It starts, watches, and stops agent
   sessions. It has no inbound network listener. It calls out to the
   Application Programming Interface (API) with a daemon token.
2. A **sync plane** copies PostgreSQL rows to browsers and to daemons. Electric
   streams the rows. An authentication proxy filters them by tenant and by
   access rule. A query path in the API is the fallback when Electric fails.
3. A **client surface** gives a person control of the sessions. It has a React
   web application, two desktop shells (Electron and Tauri), a command-line
   interface, and plugins for Claude Code, OpenCode, and Pi.

The repository also holds a cloud administration application, a billing
integration, four AWS Lambda jobs, a local development stack, and the user
documentation site.

The monorepo uses Bun workspaces and Turborepo. It holds 20 application
folders, 22 package folders, 2 contract folders, and 1 vendored package.

## 3. Documentation Goal

A reader must be able to:

1. State what HumanLayer does, and how the daemon, the sync plane, and the
   client surface divide the work.
2. Map every application, package, and contract to the concern that it owns.
3. Trace one session from a database row to a running agent process, and back
   to a rendered conversation row in the browser.
4. Explain how the daemon receives commands with no inbound listener.
5. Explain how the 13 session statuses move, and which code moves them.
6. Explain how a conversation stream reaches the client, and what the code does
   and does not guarantee.
7. Explain how Electric sync, the health tracker, and the query fallback give
   one readable collection to the user interface.
8. Explain how the proxy enforces tenancy and row access, and which path uses
   which rule.
9. Explain how the contract package keeps client and server types aligned.
10. Explain the database model, identifier convention, and migration history.
11. Explain how Electron and Tauri expose native functions to one shared user
    interface.
12. Explain what the plugins mirror, and how the command-line interface starts
    a managed daemon.
13. State the limits, the open questions, and the places where two
    implementations of one rule can drift apart.

## 4. Documentation Structure

```
agentic-verification/humanlayer/
├── spec.md
├── markdown/
│   ├── README.md                                  ← index, reading order, thesis
│   ├── 00-overview.md                             ← the three planes and the thesis
│   ├── 01-repository-map.md                       ← every app, package, contract, and pin
│   ├── 02-daemon-and-session-runtime.md           ← process, heartbeat, outbound control plane
│   ├── 03-session-lifecycle-and-state-machine.md  ← the 13 statuses and the launch path
│   ├── 04-session-sdk-and-durable-streams.md      ← session SDK packages, streams, y-durable-streams
│   ├── 05-sync-engine.md                          ← collections, health tracker, query fallback
│   ├── 06-electric-proxy.md                       ← authentication, where builders, v1/v2/v3
│   ├── 07-api-and-contracts.md                    ← Hono, oRPC, contracts, middleware, billing gates
│   ├── 08-database-and-schemas.md                 ← tables, identifiers, migrations, schema parity
│   ├── 09-write-paths-and-idempotency.md          ← event creation, transaction IDs, optimistic writes
│   ├── 10-ui-application.md                       ← React app, collections, stores, routing
│   ├── 11-desktop-shells.md                       ← Electron, Tauri, preload, native commands
│   ├── 12-cli-and-agent-plugins.md                ← the CLI and the five plugin families
│   ├── 13-cloud-billing-and-admin.md              ← admin app, Stripe, seats, access decisions
│   ├── 14-infrastructure-and-devstack.md          ← Docker, Caddy, devstack, Lambda jobs
│   ├── 15-agent-integration-and-ai.md             ← executors, model access, MCP mirroring
│   ├── 16-observability-and-attribution.md        ← Sentry, PostHog, metrics, git attribution
│   ├── 17-invariants-and-golden-documents.md      ← golden rules, de-purification, test policy
│   ├── 18-security-and-self-hosting.md            ← trust boundaries, tokens, tenancy, deployment
│   ├── 19-comparisons-and-critical-assessment.md  ← comparisons, strengths, risks
│   └── 20-glossary.md                             ← every term, spelled out
└── html/                                          ← generated
```

The 22 Markdown files map to the reader goals as follows.

| Document | Main sources |
|----------|--------------|
| README | the whole tree, this specification |
| 00 | `README.md`, the root manifests, `golden/` |
| 01 | `package.json`, `turbo.json`, `bunfig.toml`, the `apps/` and `packages/` trees |
| 02 | `apps/riptide-daemon/src`, `packages/logger` |
| 03 | `packages/schemas/src/effect/session`, `apps/riptide-daemon/src/handlers/sessions`, `golden/session_state_machine.md` |
| 04 | `packages/session-sdk-*`, `packages/streams`, `packages/y-durable-streams`, `docker/durable-streams-server` |
| 05 | `packages/sync/src`, `apps/riptide-ui/src/components/providers/SyncProvider.tsx` |
| 06 | `apps/riptide-electric-proxy/src` |
| 07 | `apps/riptide-api/src`, `contracts/riptide-api-contract/src` |
| 08 | `packages/db`, `packages/schemas` |
| 09 | `apps/riptide-api/src/orpc/daemon/sessions/events`, `apps/riptide-ui/src/actions/sessions.ts` |
| 10 | `apps/riptide-ui/src` |
| 11 | `apps/riptide-electron`, `apps/riptide-native`, `contracts/riptide-electron-rpc` |
| 12 | `apps/cli`, `apps/riptide-*-plugin`, `apps/riptide-pi-extension`, `apps/linear-cli` |
| 13 | `apps/riptide-cloud`, `packages/billing`, `apps/riptide-api/src/effects/services/billing` |
| 14 | `docker-compose.yml`, `Caddyfile`, `apps/devstack`, the four Lambda handler apps |
| 15 | `packages/ai`, the executor adapters in the daemon, `packages/schemas/src/effect/model` |
| 16 | Sentry and PostHog call sites, metrics services, `packages/git-attribution` |
| 17 | `golden/`, `CONTRIBUTING.md`, `tools/oxlint` |
| 18 | auth modules in the API and the proxy, tenancy columns, deployment files |
| 19 | every document above, plus external product documentation named in the text |
| 20 | every document above |

## 5. Tasks

| Phase | Task | Status |
|-------|------|--------|
| 1 | Read the instruction set, the directive, and the top-level README | DONE |
| 2 | Map every application, package, contract, and script folder | DONE |
| 3 | Research the monorepo toolchain, manifests, and workspace policy | DONE |
| 4 | Research the daemon process, heartbeat, and outbound control plane | DONE |
| 5 | Research the session statuses, executors, and launch path | DONE |
| 6 | Research the session Software Development Kit (SDK) packages and durable streams | DONE |
| 7 | Research the sync engine, health tracker, and query fallback | DONE |
| 8 | Research the Electric proxy, authentication, and access filters | DONE |
| 9 | Research the API, the contracts, and the middleware | DONE |
| 10 | Research the database tables, identifiers, and migrations | DONE |
| 11 | Research the write paths and the idempotency rules | DONE |
| 12 | Research the web application and its state model | DONE |
| 13 | Research the Electron and Tauri shells | DONE |
| 14 | Research the command-line interface and the plugins | DONE |
| 15 | Research the cloud application, billing, and the administration surface | DONE |
| 16 | Research the infrastructure, the development stack, and the Lambda jobs | DONE |
| 17 | Research agent integration, model access, and the Artificial Intelligence (AI) package | DONE |
| 18 | Research observability, metrics, and git attribution | DONE |
| 19 | Research the golden documents, the invariants, and the test policy | DONE |
| 20 | Write the README and documents 00 to 04 | DONE |
| 21 | Write documents 05 to 09 | DONE |
| 22 | Write documents 10 to 14 | DONE |
| 23 | Write documents 15 to 20 | DONE |
| 24 | Grandfather review against the pinned source for every document | DONE |
| 25 | Fresh-reader review for every document, and fix every reported gap | DONE |
| 26 | Build the HyperText Markup Language (HTML) tree and check every link | DONE |
| 27 | Commit with an `ADD:` prefix, and push | DONE |

## 6. Build System

```bash
cd /Users/darkvoid/Dev/explorations/agentic-verification/humanlayer
python3 -I build.py .
```

`build.py` reads `markdown/*.md`. It writes the matching `html` tree. It copies
`styles.css`. It takes the page title from the frontmatter `title:` field, or
from the first `#` heading. It writes previous and next navigation between
pages. It writes `index.html` with one link for each document. The script needs
Python 3.12 or a later version, and no other package.

The exploration is a top-level project in the `agentic-verification` folder. It
does not use the `build.py` of `src.durable-streams`.

## 7. Quality Requirements

The ten Iron Rules from `markdown_engineering/documentation_directive.md`
apply:

1. Detailed sections with code snippets from the source.
2. The thesis in the first paragraph.
3. One idea per sentence, in Aerospace and Defence Simplified Technical English
   (ASD-STE100). Spell out every abbreviation at first use in each file.
4. At least two Mermaid diagrams per document.
5. Tables, trees, and diagrams as visual assets.
6. Generated HTML with consistent navigation.
7. Cross-references between documents.
8. Source paths and line numbers for implementation claims.
9. At least one Aha moment per document.
10. An index and previous and next navigation on every page.

Each deep-dive document must hold at least ten code snippets. Every claim about
behaviour must cite the repository at the commit above. A claim about an
external system must name the external document.

The quoted code blocks must be byte-exact against the pinned source. Do not
reflow, reformat, or shorten a quoted block. Use an ellipsis comment line when
a block skips source lines, and state the line ranges on both sides of the
skip.

## 8. Expected Outcome

After reading, an engineer can explain how a person steers a coding agent on a
remote machine. The engineer can follow one session from the database row,
through the daemon control plane, into an agent process, back through the
event write path, through Electric and the proxy, and into a rendered row. The
engineer can state where the code guarantees ordering, where it only retries,
and where two implementations of one access rule can disagree. The engineer can
also state what the product does well, and which parts are young, duplicated,
or absent from the repository.

## 9. Resume Point

If work stops, read this file and the task table. The task table shows the last
completed phase. Continue with the first `TODO` row. A document is complete
only after the grandfather review, the fresh-reader review, and a successful
HTML build.

## 10. Corrections and limits found during research

These facts correct a first reading of the repository. Every document must
follow them.

| First reading | What the source shows | Evidence |
|---------------|-----------------------|----------|
| HumanLayer is a small command-line tool for Claude Code | The repository is a full product: a daemon, a sync plane, a React application, two desktop shells, an API, a cloud application, and several plugins | `README.md:3-6`, `apps/`, `packages/` |
| The daemon listens on a port and clients connect to it | The daemon has no inbound listener. It uses an outbound oRPC client at `${apiOrigin}/rpc/daemon/v1` with the `x-daemon-authorization` header, and opens read streams through Electric | `apps/riptide-daemon/src/orpc/client.ts:28-77`, `apps/riptide-daemon/src/subscriptions.ts:361-401` |
| The API and the proxy apply the same access rule | The version 1 rule is written twice: once as Structured Query Language (SQL) in the proxy and once as Drizzle conditions in the API. No shared generator or parity test exists | `apps/riptide-electric-proxy/src/lib/where-builder.ts:63-101`, `apps/riptide-api/src/db/sync-auth.ts:70-104`, `apps/riptide-electric-proxy/src/lib/queries.ts:74-78` |
| Version 2 sync respects task access control lists | Version 2 sync is organization-wide by design, and tests prove that a member reads another member's task with no access-list entry | `apps/riptide-electric-proxy/src/lib/v2-where-builder.ts:37-55`, `apps/riptide-electric-proxy/test/v2.tasks.test.ts:58-83` |
| The conversation stream delivers every event exactly once | The writer uses an `IdempotentProducer` with `maxInFlight` 1 and a 50 millisecond linger. The write path is idempotent on the event identifier. No source comment claims at-least-once or exactly-once delivery | `apps/riptide-daemon/src/effects/services/stream-producer/durable-stream-writer.ts:130-163`, `apps/riptide-api/src/orpc/daemon/sessions/events/create.ts:239-264` |
| The transaction identifier is an Electric offset | The identifier is `pg_current_xact_id()::text`, a PostgreSQL transaction identifier, and the client awaits Electric until the shape passes it | `apps/riptide-api/src/db/get-tx-id.ts:4-7`, `apps/riptide-ui/src/components/providers/SyncProvider.tsx:927-1024` |
| The status list is a state machine with declared transitions | The schema declares 13 status values in one place. No declarative transition table exists; transitions are code in handlers and in the daemon | `packages/schemas/src/effect/session/session-status.ts:7-39`, `apps/riptide-daemon/src/handlers/sessions/` |
| `opencode` and `pi` sessions are launched by the daemon | Their adapters mirror sessions only. The launchable agent set is `claude`, `codelayer`, and `fold` | `packages/schemas/coding-agent/coding-agent.ts:4-38`, `apps/riptide-daemon/src` |
| The repository deploys its infrastructure as code | No Pulumi program and no Lambda definition file exists. `packages/pulumi-helpers` and the documentation site refer to deployment, but the definitions are not in this snapshot | `packages/pulumi-helpers`, `apps/docs` |
| Row-level security in PostgreSQL enforces tenancy | Tenancy is enforced in application SQL and in the API. No row-level security policy was found in the inspected table definitions | `packages/db/src/tables`, `apps/riptide-electric-proxy/src/lib/where-builder.ts:63-101` |

## 11. Verification record

This section is filled after the last edit. The table records the checks that
ran, the numbers they produced, and the result.

| Check | Result |
|-------|--------|
| Markdown documents | 22 deep dives and one index, 109,520 words, 47 Mermaid diagrams, and 51 Aha insights |
| Source citations | 1,480 unique `path:line` citations; the range and blank-line check reports no issue on any file |
| Quoted code blocks | 271 code blocks; every block matches its cited source range without an edge difference |
| Markdown links | 23 files checked, zero broken links |
| HTML pages | 23 pages built from the Markdown; zero broken links in the generated tree |
| Grandfather review | Every document checked against commit `2f66a45b186d88488fa356862be0117db1ccae9f` |
| Fresh-reader review | Six reports from agents with no memory of the writing; every reported gap is fixed or recorded as a limit of the repository |
| Build | `python3 -I build.py .` completes and writes `index.html` plus 22 document pages |
