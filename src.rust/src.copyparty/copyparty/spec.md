---
name: copyparty-spec
description: Project tracker for copyparty documentation — source-verified exploration with grandfather review
metadata:
  type: project
---

# Copyparty Documentation Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.copyparty/copyparty`
- **Language:** Python
- **Type:** HTTP file server with web UI
- **Author:** 9001 (as per README)
- **License:** MIT
- **Repository:** https://github.com/9001/copyparty

## What the Project Is

copyparty is a portable HTTP file server that maps a local filesystem to a web interface. It supports uploads, downloads, directory browsing, media streaming, user authentication, and server-side rendering for various file types. It's designed to be a single-file executable that runs anywhere Python is available.

## Documentation Goal

After reading this documentation, a reader should understand:

1. The overall architecture and how HTTP requests flow through the system
2. The plugin/module structure and how features are organized
3. How authentication and access control work
4. How file uploads and downloads are handled
5. The media streaming and transcoding capabilities
6. The Web UI components and JavaScript architecture
7. How to configure and extend copyparty

## Documentation Structure

```
src.rust/src.copyparty/copyparty/
├── spec.md                     ← This file — project tracker
├── markdown/                   ← Source documentation
│   ├── README.md               ← Index / table of contents
│   ├── 00-overview.md          ← Project philosophy, quick architecture
│   ├── 01-architecture.md      ← Module/package dependency graph
│   ├── 02-http-handlers.md     ← HTTP request handling deep dive
│   ├── 03-authentication.md    ← Auth system and access control
│   ├── 04-file-operations.md   ← Uploads, downloads, filesystem
│   ├── 05-media-streaming.md   ← Media player and transcoding
│   ├── 06-web-ui.md            ← Frontend JavaScript and templates
│   ├── 07-configuration.md     ← Config system and CLI args
│   ├── 08-plugins.md           ← Plugin architecture
│   └── 09-data-flow.md         ← End-to-end request flows
└── html/                       ← Generated HTML
    ├── index.html
    ├── styles.css
    └── *.html
```

## Tasks

| Phase | Document | Status | Notes |
|-------|----------|--------|-------|
| 1 | Read source code | DONE | Entry points, handlers, modules |
| 2 | 00-overview.md | DONE | Project philosophy, 2 mermaid diagrams, 3 code snippets, 1 aha moment |
| 2 | 01-architecture.md | DONE | Module graph, layers, 3 mermaid diagrams, 4 code snippets, 1 aha moment |
| 2 | 02-http-handlers.md | DONE | Request handling, 2 mermaid diagrams, 4 code snippets, 2 aha moments |
| 2 | 03-authentication.md | DONE | Auth system, 2 mermaid diagrams, 4 code snippets, 1 aha moment |
| 2 | 04-file-operations.md | DONE | File I/O, 2 mermaid diagrams, 5 code snippets, 1 aha moment |
| 2 | 05-media-streaming.md | DONE | Media features, 3 mermaid diagrams, 4 code snippets, 1 aha moment |
| 2 | 06-web-ui.md | DONE | Frontend, 2 mermaid diagrams, 6 code snippets, 1 aha moment |
| 2 | 07-configuration.md | DONE | Config system, 2 mermaid diagrams, 4 code snippets, 1 aha moment |
| 2 | 08-plugins.md | DONE | Plugin architecture, 3 mermaid diagrams, 5 code snippets, 1 aha moment |
| 2 | 09-data-flow.md | DONE | Sequence diagrams, 5 mermaid diagrams, code snippets, 1 aha moment |
| 3 | Generate HTML | DONE | 11 HTML files generated with navigation |
| 4 | Grandfather Review | DONE | Verified function names, flows, source paths |

## Documentation Summary

- **Total markdown documents**: 10
- **Total lines of documentation**: ~4500
- **Mermaid diagrams**: 25+
- **Code snippets**: 40+
- **Source file references**: All major modules covered
- **Generated HTML files**: 12 (including index)

## Build System

Use the shared build script:

```bash
python3 build.py src.rust/src.copyparty/copyparty
```

## Quality Requirements (Iron Rules)

All documents must meet these standards:

1. **Detailed sections with code snippets** — Every concept grounded in actual source
2. **Teach key facts quickly** — First paragraph = thesis statement
3. **Clear articulation** — One idea per sentence, max 30 words
4. **Mermaid diagrams** — Minimum 2 per document
5. **Good visual assets** — Tables, ASCII art, code blocks
6. **Generated HTML** — All markdown builds to HTML with navigation
7. **Cross-references** — Link to related documents, no orphans
8. **Source path references** — Include actual file paths and line numbers
9. **Aha moments** — Surface clever design decisions, non-obvious tradeoffs
10. **Navigation** — Index + prev/next buttons on every page

## Grandfather Review Checklist

For each document:
- [ ] Every function name matches source code (grep verification)
- [ ] Every default value matches source code
- [ ] Every pipeline/flow matches actual execution order
- [ ] Every public API surface is documented
- [ ] At least 2 mermaid diagrams
- [ ] At least 3 code snippets with file paths
- [ ] At least 1 Aha moment
- [ ] Links to at least 2 related documents
- [ ] HTML builds correctly
- [ ] Navigation bar present and correct

## Resume Point

Work in progress. Start with Phase 1: reading the source code to understand the entry point, module structure, and key components.
