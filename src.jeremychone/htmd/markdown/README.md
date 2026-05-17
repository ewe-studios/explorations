# htmd — Documentation

**Crate:** htmd v0.5.4 | **License:** Apache-2.0 | **Author:** letmutex | **Source:** 29 Rust files

HTML to Markdown converter using `html5ever` for DOM parsing. Version 0.5.4 introduces dual translation modes (Pure/Faithful), a `Handlers` trait for handler delegation, adjacent inline element merging, and `phf`-based block classification.

## Foundation

- [Overview](00-overview.html) — What htmd is, architecture at a glance, public API, key changes from v0.2.1
- [Architecture](01-architecture.html) — Module map, Handlers trait, HandlerResult, HashMap tag lookup, TranslationMode flow

## Deep Dives

- [DOM Walker](02-dom-walker.html) — walk_node dispatch, walk_children with can_combine pre-merge, is_plain_text byte optimization, escape_if_needed, append_normalized_content
- [Element Handlers](03-element-handlers.html) — All 22+ handlers: anchor with thread-local links, code with adaptive fences, table with pipe formatting, list with aligned numbering, math spans, emphasis, and the catch-all block handler
- [Faithful Mode](04-faithful-mode.html) — Pure vs Faithful decision flow, serialize_if_faithful! macro, serialize_element inline vs block serialization, newline escaping, markdown_translated flag propagation
