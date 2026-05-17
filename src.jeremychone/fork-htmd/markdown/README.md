# fork-htmd — Documentation

**Crate:** htmd v0.2.1 | **License:** Apache-2.0 | **Source:** 17 Rust files, ~2,700 lines

HTML to Markdown converter inspired by turndown.js — parses HTML with html5ever, walks the DOM tree, converts elements via handler registry.

## Foundation

- [Overview](00-overview.html) — What fork-htmd is, architecture at a glance, public API, supported elements, options summary
- [Architecture](01-architecture.md) — Module map, three-layer design, conversion pipeline, handler registry, text processing

## Deep Dives

- [DOM Walker](02-dom-walker.html) — Depth-first DOM traversal, block/inline classification, text escaping, whitespace compression, join_contents
- [Element Handlers](03-element-handlers.html) — All 13+ built-in handlers: headings, code, tables, lists, links, images, blockquotes, emphasis
- [Options](04-options-config.html) — Options struct, all 8 config enums, builder pattern, custom handlers, spacing options
