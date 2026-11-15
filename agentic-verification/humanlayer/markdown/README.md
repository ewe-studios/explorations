---
title: HumanLayer Documentation Index
---

# HumanLayer Documentation

> **HumanLayer runs coding agents (Claude Code, Codex, OpenCode, Pi) on your
> machines and lets your team steer them from one place.** A session is a
> database row. A daemon on the host is the remote hand that acts on it. An
> Electric sync plane is the shared eye that shows it to everyone. Electric is
> the sync engine. One oRPC (the procedure-call library) contract package is the
> spine that keeps the API, the daemon, the browser, and the desktop shells in
> agreement about the fields of that row. A second contract covers the desktop
> procedure calls.

The product names in that sentence differ from the schema values. The daemon launches Claude Code and CodeLayer. It runs Fold when the default-off experiment is on. It mirrors Codex, OpenCode, and Pi. The glossary maps each name to its coding-agent value and states whether the daemon can launch it ([Glossary](20-glossary.md#the-product-and-its-actors)).

This set explains the repository at `/Users/darkvoid/GitSources/src.humanlayer/humanlayer`
at commit `2f66a45b186d88488fa356862be0117db1ccae9f` (2026-10-23). Every claim
traces to a file in that snapshot.

## Start here

- [Overview](00-overview.md): the three planes, the session row, the thesis
- [Repository map](01-repository-map.md): all 20 applications, 22 packages, 2
  contracts, and the toolchain versions

## The daemon and the session

- [Daemon and session runtime](02-daemon-and-session-runtime.md): a process with
  no inbound listener, and how it still receives commands
- [Session lifecycle and state machine](03-session-lifecycle-and-state-machine.md):
  the 13 statuses, the executors, and the 13-step launch path
- [Session SDK and durable streams](04-session-sdk-and-durable-streams.md): the
  six session packages, the conversation event schemas, and the stream writer
- [Write paths and idempotency](09-write-paths-and-idempotency.md): one event
  from the daemon to the database and back to the browser

## The sync plane

- [The sync engine](05-sync-engine.md): Electric collections, the health
  tracker, the query fallback, and the keep-alive
- [The Electric proxy](06-electric-proxy.md): token validation, tenant filters,
  version 1 access lists, version 2 organization scope, and shared artifacts
- [The API and the contracts](07-api-and-contracts.md): Hono, oRPC, the typed
  error map, middleware, and billing gates

## Data

- [Database and schemas](08-database-and-schemas.md): the table model, UUIDv7
  identifiers, 144 migrations, and the Zod and Effect schema pairs
- [Observability and attribution](16-observability-and-attribution.md): Sentry,
  PostHog, metrics, and the git attribution tiers

## The client plane

- [The user interface application](10-ui-application.md): React 19, TanStack
  Router and DB, and the Zustand stores
- [Desktop shells](11-desktop-shells.md): Electron, Tauri, the preload bridge,
  and the packaged daemon sidecar
- [Command-line interface and agent plugins](12-cli-and-agent-plugins.md): the
  `humanlayer` CLI and the five plugin families (Claude Code mirroring plugin,
  OpenCode mirroring plugin, Pi extension, `hl` skill plugin, and `rpi` skill
  plugin)
- [Agent integration and AI](15-agent-integration-and-ai.md): executors, model
  access, and what the mirroring plugins do and do not do

## Operations

- [Cloud, billing, and administration](13-cloud-billing-and-admin.md): the admin
  application, Stripe seats, and the fail-open access decision
- [Infrastructure and the development stack](14-infrastructure-and-devstack.md):
  Docker Compose, Caddy, `devstack`, and the four Lambda handlers
- [Security and self-hosting](18-security-and-self-hosting.md): trust
  boundaries, token types, tenancy, and what a self-hoster must supply

## Rules and verdict

- [Invariants and golden documents](17-invariants-and-golden-documents.md): the
  engineering rules the project writes down for itself. A golden document is a
  trusted design specification in `golden/`, not generated output.
- [Comparisons and critical assessment](19-comparisons-and-critical-assessment.md):
  how it compares, and where it is thin
- [Glossary](20-glossary.md): every term, spelled out

## Reading orders

| You are | Read in this order |
|---------|--------------------|
| New to the product | [00](00-overview.md), [01](01-repository-map.md), [03](03-session-lifecycle-and-state-machine.md), [10](10-ui-application.md) |
| Working on the daemon | [02](02-daemon-and-session-runtime.md), [03](03-session-lifecycle-and-state-machine.md), [04](04-session-sdk-and-durable-streams.md), [09](09-write-paths-and-idempotency.md) |
| Working on sync | [05](05-sync-engine.md), [06](06-electric-proxy.md), [08](08-database-and-schemas.md), [09](09-write-paths-and-idempotency.md) |
| Working on the API | [07](07-api-and-contracts.md), [08](08-database-and-schemas.md), [06](06-electric-proxy.md) |
| Self-hosting | [14](14-infrastructure-and-devstack.md), [18](18-security-and-self-hosting.md), [13](13-cloud-billing-and-admin.md) |
| Judging the design | [00](00-overview.md), [17](17-invariants-and-golden-documents.md), [19](19-comparisons-and-critical-assessment.md) |

## The one-paragraph thesis

HumanLayer works because it refuses to make the host a server. The daemon on a
machine opens only outbound connections, so a laptop behind a home router is
reachable without a port, a certificate, or a tunnel. Read authority flows the
other way, through Electric, so every person sees the same rows without a
custom push protocol. The cost of that choice is that the daemon must poll and
subscribe for its instructions, and that the access rule lives in two places:
SQL in the proxy and Drizzle in the API. This set explains the design, the
code that implements it, and the places where the two halves can disagree.

---

Generated from source code. Every claim traces back to implementation.
