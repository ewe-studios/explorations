# celld -- Project Spec

## Source Codebase

- **Location:** `/Users/darkvoid/GitSources/src.s3/celld/`
- **Language:** Rust
- **Version:** 0.6.3 (from `crates/celld/Cargo.toml`)
- **Part of:** celld (Deno Land) — a self-hosted, distributed Durable Objects daemon

## What celld Is

celld is a Rust daemon that runs a Cloudflare Workers application without
Cloudflare. It embeds the V8 JavaScript engine and executes Workers, Durable
Objects, KV namespaces, Queues, D1 databases, R2 buckets, Workflows, Cron
Triggers, and static assets from a `wrangler.json` configuration. A fleet of
celld nodes coordinates over a shared object-storage bucket, and each Durable
Object (a "cell") owns a private SQLite database.

The code is a workspace of three crates:

- `crates/celld` — the executor and adapter host. It owns V8, the bucket
  client, the network server, and the effect loop.
- `crates/logic` — the pure decision core. It holds all state transitions
  with no asynchronous work, no input/output, no clocks, no randomness, and no
  locks.
- `crates/ltx` — a vendored LTX replication library from the rustyriver
  project, used for the write-ahead log that replicates each SQLite database.

The important architectural idea is a serial actor. A cell advances only
through one function, `on_event`. Every event produces effects, and every
asynchronous effect comes back as a versioned event. The core can therefore be
simulated deterministically and tested without a running daemon.

## Documentation Structure

```
src3/celld/
├── spec.md                          <- THIS FILE
└── markdown/
    ├── README.md                    <- index
    ├── 00-overview.md
    ├── 01-the-cell.md
    ├── 02-logic-executor-split.md
    ├── 03-durability.md
    ├── 04-workers-execution.md
    ├── 05-kv-queues-d1-r2.md
    ├── 06-workflows-cron.md
    ├── 07-static-assets.md
    ├── 08-wrangler-deploy.md
    └── 09-fleet-operations.md
```

The numbered documents form a chain from `00-overview.md` to
`09-fleet-operations.md` and then back to `README.md`. Each document ends with
a `Continue to` link to the next document, so a reader can follow the chain in
order.

## Quality Requirements (Iron Rules)

Every numbered document must follow these rules:

1. The first line is `# NN -- Title: subtitle`. No frontmatter.
2. At least two Mermaid diagrams. Each diagram starts, on its own line inside
   the fence, with
   `%%{init: {"flowchart": {"htmlLabels": false}}}%%` or the
   `{"sequence": ...}` variant. Every node label is quoted. No label contains
   an unquoted parenthesis.
3. At least three real code snippets. Each snippet is fenced and attributed
   with `// Source: path, lines N-M` (Rust uses `//`). The code is verbatim
   from the source; nothing is invented.
4. Exactly one `## The Aha Moment` section. Its first line is `**Aha:**`.
5. At least two cross-references in a `## Cross-References` section.
6. The last line is `Continue to [NN-slug.md](NN-slug.md).` The chain is
   00 to 01 to ... to 09 to README. No orphan documents.
7. ASD-STE100 Simplified Technical English. Spell out every abbreviation at
   its first use in each document.
8. Every factual claim is verified against the source. Where the source and
   the README disagree, the source wins.

## Verification

After writing, run the build from the explorations root:

```sh
cd /Users/darkvoid/Dev/explorations && python3 build.py src3/celld
```

The build must generate `html/` under `src3/celld/` without error.
