# walgit -- Project Spec

## Source Codebase

**Location:** /Users/darkvoid/GitSources/src.s3/walgit/
**Language:** Rust
**Version:** 0.1.0 (`workspace.package.version` in `Cargo.toml`)
**Part of:** Standalone project (MIT license)

## What walgit Is

walgit is a git server that is one binary in front of an object store (Amazon S3, any S3-compatible store, or Google Cloud Storage). It hosts git repositories with no database, no leader, and no local state that matters. Point the binary at a bucket and you get smart HTTP (HyperText Transfer Protocol) v0/v2 fetch and push, Git Large File Storage (LFS), a web UI, a JSON (JavaScript Object Notation) API with a dependency-free SDK (software development kit), per-repository push policy, and webhooks. Every machine that runs walgit is a disposable cache; the bucket is the repository.

The architecture comes from Cursor's "Git at any scale" post (their Continuity system). The core idea: a write-ahead log (WAL) in the object store is the source of truth. A push lands as an immutable object and becomes visible only when a tiny manifest is rewritten with a compare-and-swap (CAS). That CAS is the only commit point, so there is no election, quorum, or primary. Any instance may accept a push; two racing instances cannot both win. Reads are consistent because every read first asks the store whether anything changed (a conditional GET).

## Documentation Structure

```
markdown/
  00-object-store-repository.md
  01-proto-keys.md
  02-store-crate.md
  03-wal-crate.md
  04-git-storage.md
  05-smart-http.md
  06-lfs.md
  07-json-api-sdk.md
  08-push-policy.md
  09-webhooks.md
  10-scaling.md
  11-config-auth-cli.md
  README.md
```

The twelve numbered documents form a reading chain from 00 to 11 and then back to README.md. Each one covers one slice of the system: the object-store-as-repository model, the protobuf contract and bucket keys, the store crate, the WAL crate, git object storage, smart HTTP transfer, LFS, the JSON API and SDK, push policy, webhooks, the disposable-cache scaling story, and finally configuration, authentication, roles, and the CLI (command-line interface).

## Quality Requirements (Iron Rules)

1. First line of every numbered document is `# NN -- Title: subtitle`, with no frontmatter.
2. At least two Mermaid diagrams, each preceded on its own line by `%%{init: {"flowchart": {"htmlLabels": false}}}%%` or the `{"sequence": ...}` variant. All node labels are quoted, and no label has an unquoted parenthesis.
3. At least three real code snippets, each fenced and attributed with `// Source: path, lines N-M` for Rust (or `# Source:` for shell and TOML). All snippets are verbatim from the source; nothing is invented.
4. Exactly one `## The Aha Moment` section, whose first line is bolded `**Aha:**`.
5. At least two cross-references to other documents in the set.
6. The last line of every numbered document is `Continue to [NN-slug.md](NN-slug.md).`, chaining 00 through 11 and ending at README.md. No document is an orphan.
7. Prose follows ASD-STE100 simplified technical English, and every abbreviation is spelled out at first use.
8. Every factual claim is verified against the source. Where the README and the code disagree, the code wins.

## Verification

The markdown is built to HTML with the zero-dependency builder in the explorations tree:

```
cd /Users/darkvoid/Dev/explorations && python3 build.py src3/walgit
```

The builder reads `markdown/*.md`, renders fenced Mermaid blocks, converts headings, tables, lists, and links, and writes `html/*.html` plus an `html/index.html` index. A successful run prints one `✓` line per document and ends with the index URL.
