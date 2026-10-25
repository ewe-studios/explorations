# anycable-delivery -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.Ruby/anycable/`
**Language:** Go (the `anycable-go` server)
**Version:** 1.6.20 (source default in `version/version.go`); module
`github.com/anycable/anycable-go` at `go 1.27.0` in `go.mod`

**What it is:** The Go implementation of the AnyCable realtime server. It speaks
the Action Cable protocol over WebSocket and Server-Sent Events (SSE), keeps a
hub of sessions and streams, fans broadcasts out, and talks to a Ruby
application over a Remote Procedure Call (RPC) layer (gRPC or HTTP).

## What This Explores

Two delivery properties, as the code actually enforces them:

1. **Ordering** — in what order messages on one stream reach one client, and
   where that order is broken (across streams, across nodes, across a pub/sub
   re-transmit).
2. **At-least-once delivery** — how the broker's per-stream history plus the
   client-driven offset replay turn at-most-once fan-out into at-least-once
   catch-up, and where duplicates and gaps still occur.

The exploration reads the delivery path end to end: the stream model and
subscriptions, the sharded in-memory hub, the per-connection send queue, the
NATS/Redis broadcast and pub/sub layers, the broker history, session resume,
the offset protocol, and the RPC layer.

## Documentation Structure

```
ruby/anycable-delivery/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index
    ├── 00-the-delivery-question.md
    ├── 01-stream-model-subscriptions.md
    ├── 02-hub-fanout-send-queue.md
    ├── 03-cross-node-broadcasts.md
    ├── 04-broker-history-replay.md
    ├── 05-session-resume-offsets.md
    └── 06-where-the-guarantee-holds.md
```

## Quality Requirements (Iron Rules)

1. First line `# NN -- Title: subtitle`; no YAML frontmatter.
2. At least two Mermaid diagrams per document, each preceded on its own line by
   `%%{init: {"flowchart": {"htmlLabels": false}}}%%` or the `{"sequence": ...}`
   variant; all node labels quoted; no unquoted parentheses in labels.
3. At least three real code snippets per document, each fenced and attributed
   with `// Source: <path>, lines N-M` (Go); verbatim, no invented code.
4. Exactly one `## The Aha Moment` per document, first line bolded `**Aha:**`,
   about the ordering or delivery guarantee.
5. At least two cross-references in a `## Cross-References` section.
6. Last line `Continue to [NN-slug.md](NN-slug.md).`; chain 00 to 06 to README,
   no orphan pages.
7. ASD-STE100 Simplified Technical English; abbreviations spelled out at first
   use (WebSocket, SSE, RPC, NATS, Redis, TTL, JSON, protobuf).
8. Every factual claim verified against source; where the docs or README imply a
   guarantee the code does not enforce, the document says so and shows the code.

## Verification

Authored by reading the full Go source of every file cited in a snippet. The
README in this repository is a badge-only stub; the real product documentation
lives under `docs/` in the source tree and was cross-checked against code, with
code preferred wherever they disagree.
