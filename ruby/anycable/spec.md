# AnyCable-Go -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.Ruby/anycable/`
**Language:** Go
**Version:** 1.6.20
**Part of:** AnyCable. This repository is the AnyCable server, the Go process that
pairs with the Ruby `anycable` gem and the Rails integration. It speaks the
AnyCable protocol (protobuf over WebSocket) and calls back into the Ruby app
over Remote Procedure Call (RPC).

## What AnyCable-Go Is

AnyCable-Go is a Go reimplementation of the Action Cable server. A browser or
mobile client opens a WebSocket (or a Server-Sent Events, SSE, stream) to
AnyCable-Go. The server authenticates the connection and routes every command
back to the Ruby application over protobuf RPC. Streams are stored in an
in-memory hub, and broadcasts fan out to every node over NATS or Redis
pub/sub. The result is one Go process that holds hundreds of thousands of live
connections with a fraction of the memory a Ruby process would need.

## Documentation Structure

```
ruby/anycable/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index
    ├── 00-what-is-anycable.md
    ├── 01-wire-protocol.md
    ├── 02-connection-lifecycle.md
    ├── 03-hub-and-subscriptions.md
    ├── 04-broadcasts-pubsub.md
    ├── 05-rpc-layer.md
    ├── 06-transports.md
    ├── 07-configuration-cli.md
    ├── 08-telemetry-metrics.md
    └── 09-embedding-and-extras.md
```

## Quality Requirements (Iron Rules)

1. At least two Mermaid diagrams, three code snippets, one `## The Aha Moment`
   (opening with a bolded `**Aha:**`), and two cross-references per document.
2. H1 first line `# NN -- Title: subtitle`, no YAML frontmatter.
3. Every Mermaid diagram begins with an init directive on its own line and uses
   quoted node labels.
4. Footer chain 00→…→09→README; no orphan pages.
5. ASD-STE100 Simplified Technical English; abbreviations spelled out at first use.
6. Real source code snippets with path + approximate line.
7. HTML via `/Users/darkvoid/Dev/explorations/build.py ruby/anycable`.

## Verification

Authored by an agent reading the full Go source; verified by independent
agents (fresh-reader review + grandfather review against the Go source).
