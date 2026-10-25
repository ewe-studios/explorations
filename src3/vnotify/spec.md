# vnotify -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.s3/vnotify/`
**Language:** Rust
**Version:** 0.1.0
**Part of:** a Deno S3-monitoring library (authored by the Deno authors).

## What vnotify Is

`vnotify` is a lightweight Rust library that monitors changes in an Amazon
Simple Storage Service (S3) bucket and caches objects. It needs no extra
infrastructure: no database, no message queue, no notification service. A
writer touches two small marker objects after each write; a reader polls one
marker with one `HeadObject` call, and only when that marker changed does it
make one `ListObjectsV2` call. The whole library is one source file,
`src/lib.rs` (571 lines).

## Documentation Structure

```
src3/vnotify/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index
    ├── 00-overview.md
    ├── 01-one-headobject-signal.md
    ├── 02-keyspace-as-hashmap.md
    ├── 03-refresh-worker-flow.md
    └── 04-detection-math.md
```

## Quality Requirements (Iron Rules)

1. At least two Mermaid diagrams, three code snippets, one `## The Aha Moment`
   (opening with a bolded `**Aha:**`), and two cross-references per document.
2. H1 first line `# NN -- Title: subtitle`, no YAML frontmatter.
3. Every Mermaid diagram preceded on its own line by
   `%%{init: {"flowchart": {"htmlLabels": false}}}%%` (or the `{"sequence": ...}`
   variant); all node labels quoted; no unquoted parentheses in labels.
4. Every code snippet verbatim from `src/lib.rs` and attributed
   `// Source: src/lib.rs, lines N-M`.
5. Footer chain 00→01→02→03→04→README; no orphan pages.
6. ASD-STE100 Simplified Technical English; abbreviations spelled out at first use.
7. HTML via `/Users/darkvoid/Dev/explorations/build.py src3/vnotify`.

## Verification

Authored by an agent reading the full crate source (`README.md`, `Cargo.toml`,
`rust-toolchain.toml`, and `src/lib.rs` in full). Every factual claim was
checked against the source; where the README and the source differ, the source
wins.
