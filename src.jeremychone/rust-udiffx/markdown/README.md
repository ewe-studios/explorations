# udiffx — Documentation

**Crate:** udiffx v0.1.42-WIP | **License:** MIT OR Apache-2.0 | **Source:** 23 Rust files, ~5,523 lines

Parse and apply LLM-optimized unified diff patches and XML-like file change directives.

## Foundation

- [Overview](00-overview.html) — What udiffx is, two-phase architecture, public API, key types
- [Architecture](01-architecture.html) — Full module map, layer diagram, sequence diagram, type relationships, security model

## Deep Dives

- [Extraction](02-extract.html) — markex-based tag parsing, self-closing tag expansion, Content processing, FileDirective enum
- [Patch Completer](03-patch-completer.html) — Tiered matching algorithm (Strict → Resilient → Fuzzy), hunk parsing, candidate search, scoring, tilde ranges
- [Applier](04-applier.html) — Filesystem execution, path security, incremental patch application, status reporting
