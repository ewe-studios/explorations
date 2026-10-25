# walgit-celld-replication -- Project Spec

## Source Codebase

Two Rust codebases, read side by side. Neither is modified here; every claim
is cited against source.

- **walgit** — `/Users/darkvoid/GitSources/src.s3/walgit/`
  - Language: Rust (edition 2024, rust-version 1.90).
  - Version: workspace `0.1.0` (all seven crates inherit
    `version.workspace = true`).
  - One binary in front of an object store: the bucket is the repository.
- **celld** — `/Users/darkvoid/GitSources/src.s3/celld/`
  - Language: Rust (edition 2021, rust-version 1.94.1).
  - Versions: `celld` 0.6.3, `celld-logic` 0.1.0, `celld-ltx` 0.0.0 (vendored
    from rustyriver, a reimplementation of Litestream v0.5).
  - A self-hosted Durable Objects daemon: each cell is a SQLite database.

## What This Compares

Two systems that treat an object-storage bucket as the only durable surface,
and the different ways they make that safe and fast. walgit makes the bucket
the repository: immutable, content-addressed git packs plus a tiny
compare-and-swap (CAS) manifest that is the single commit point. celld makes
the bucket the long-term home of many small SQLite databases, captures each
committed write as an LTX segment, fences ownership with a
CAS on `own.json`, and withholds every acknowledgement until a durability
proof lands, a Recovery Point Objective (RPO) of zero. Each document contrasts
one dimension, drawing code from both trees.

## Documentation Structure

```
src3/walgit-celld-replication/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index
    ├── 00-shared-substrate.md
    ├── 01-unit-of-state.md
    ├── 02-write-path.md
    ├── 03-consistency-fencing.md
    ├── 04-durability-rpo-rto.md
    ├── 05-efficiency.md
    └── 06-when-each-wins.md
```

## Quality Requirements (Iron Rules)

1. H1 first line `# NN -- Title: subtitle`, no YAML frontmatter.
2. At least two Mermaid diagrams per document, each preceded on its own line
   by `%%{init: {"flowchart": {"htmlLabels": false}}}%%` (or the sequence
   variant); every node and edge label quoted; no unquoted parentheses in
   labels.
3. At least three code snippets per document, fenced and attributed
   `// Source: walgit/<path>, lines N-M` or `// Source: celld/<path>, lines
   N-M`; verbatim; each document draws from both trees at least once.
4. Exactly one `## The Aha Moment` per document, first line `**Aha:**`.
5. At least two cross-references in a `## Cross-References` section.
6. Footer chain 00→01→02→03→04→05→06→README; no orphan pages.
7. ASD-STE100 Simplified Technical English; abbreviations spelled out at
   first use (S3, GCS, RPO, RTO, CAS, WAL, LTX, KV, D1, SQLite, LFS, SHA).
8. Every factual claim verified against source; source wins over README where
   they disagree.

## Verification

Authored by an agent that read both trees in full, verified every cited line
range verbatim, and confirmed the build runs with no error:

```
cd /Users/darkvoid/Dev/explorations && python3 build.py src3/walgit-celld-replication
```
