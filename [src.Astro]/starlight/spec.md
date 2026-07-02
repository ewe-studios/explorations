---
title: "Starlight Documentation - Spec & Tracker"
---

# Starlight Documentation Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Astro/starlight/packages/starlight/`
- **Version:** 0.41.1
- **Package:** `@astrojs/starlight`
- **Repository:** https://github.com/withastro/starlight
- **Author:** Chris Swithinbank <swithinbank@gmail.com>
- **License:** MIT
- **Language:** TypeScript + Astro
- **Package Manager:** pnpm 11.1.1
- **Peer Dependencies:** `astro ^7.0.2`, `@astrojs/markdown-remark ^7.2.0` (optional)

## What Starlight Is

Starlight is a documentation theme and site generator for Astro. It provides a complete, production-ready documentation site framework with built-in internationalization (i18n), sidebar navigation, table of contents, search (Pagefind), code block syntax highlighting (Expressive Code), dark/light themes, and responsive design. It transforms Markdown and MDX content into polished documentation sites with minimal configuration. Starlight operates as an Astro integration, hooking into Astro's lifecycle to inject routes, middleware, Vite plugins, and content collection schemas.

## Documentation Goal

A reader should understand:

1. **What Starlight is** -- its architecture as an Astro integration, how it fits into the Astro ecosystem
2. **Configuration schema** -- every config option, Zod validation, defaults, and transforms
3. **Integration lifecycle** -- how Starlight hooks into Astro's `astro:config:setup`, `astro:config:done`, and `astro:build:done` phases
4. **Virtual module system** -- how `virtual:starlight/*` modules expose config, components, and context at build time
5. **Internationalization (i18n)** -- locale configuration, translation system, fallback content, BCP-47 handling
6. **Sidebar and navigation** -- sidebar schema, autogeneration, tree building, prev/next links
7. **Content loaders** -- `docsLoader` and `i18nLoader`, content collection integration, Markdoc support
8. **Component system** -- page frame components, user components, override mechanism via virtual modules
9. **Plugin system** -- plugin schema, hooks (`config:setup`, `i18n:setup`), `updateConfig`, `addIntegration`, `addRouteMiddleware`
10. **Markdoc integration** -- `@astrojs/starlight/markdoc` package, Markdoc-specific content handling
11. **DocSearch integration** -- `@astrojs/starlight/docsearch` package, Algolia DocSearch integration
12. **Cross-cutting concerns** -- routing, route data generation, CSS layers, Expressive Code, Pagefind, sitemap

## Documentation Structure

```
[src.Astro]/starlight/
  starlight/
    spec.md
    markdown/
      README.md                  ← Index / table of contents
      00-overview.md             ← What Starlight is, philosophy, architecture at a glance
      01-architecture.md         ← Full dependency graph, layer diagram, module map
      02-config-schema.md        ← Config schema deep dive, Zod validation, all options
      03-integration.md          ← Integration lifecycle, hooks, Astro interaction
      04-virtual-modules.md      ← Virtual module system, Vite plugin, config exposure
      05-i18n.md                 ← Internationalization system, locales, translations, fallbacks
      06-sidebar-nav.md          ← Sidebar schema, autogeneration, navigation, prev/next
      07-content-loaders.md      ← docsLoader, i18nLoader, content collection integration
      08-components.md           ← Component system, page frame, user components, overrides
      09-plugins.md              ← Plugin system, hooks, API, examples
      10-markdoc-docsearch.md    ← Markdoc and DocSearch integrations
      11-cross-cutting.md        ← Routing, route data, CSS, Expressive Code, Pagefind
    html/                        ← Generated HTML
    build.py                     ← Shared Markdown → HTML script
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Create spec.md | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-architecture.md | DONE |
| 4 | Write 02-config-schema.md | DONE |
| 5 | Write 03-integration.md | DONE |
| 6 | Write 04-virtual-modules.md | DONE |
| 7 | Write 05-i18n.md | DONE |
| 8 | Write 06-sidebar-nav.md | DONE |
| 9 | Write 07-content-loaders.md | DONE |
| 10 | Write 08-components.md | DONE |
| 11 | Write 09-plugins.md | DONE |
| 12 | Write 10-markdoc-docsearch.md | DONE |
| 13 | Write 11-cross-cutting.md | DONE |
| 14 | Generate HTML | DONE |
| 15 | Grandfather review | DONE |

## Build System

```bash
cd [src.Astro]/starlight && python3 build.py          # build all projects
python3 build.py starlight                             # build one project
```

**Script:** `build.py` (copied from repo root)
**Dependencies:** None (Python 3.12+ stdlib only)

## Quality Requirements

See `markdown_engineering/documentation_directive.md` for the full directive. Key requirements:

- Minimum 2 mermaid diagrams per document
- Minimum 3 code snippets per document with file path references
- Minimum 1 Aha moment per document
- Detailed sections - teach, don't summarize
- First paragraph states core concept clearly
- Cross-references between documents
- Grandfather review: names match, numbers match, flows match, nothing missing

## Expected Outcome

After reading this documentation, a developer should be able to:

1. Configure a Starlight site with any combination of features (i18n, custom CSS, components, plugins)
2. Write Starlight plugins that extend configuration and add integrations
3. Override Starlight components with custom implementations
4. Understand the full request lifecycle from URL to rendered page
5. Debug Starlight issues by understanding the virtual module and routing systems
6. Integrate Markdoc and DocSearch with Starlight
7. Extend Starlight's content and i18n schemas

## Resume Point

If interrupted, check the spec.md tasks table for the last completed item. All markdown files are in `markdown/`, HTML output in `html/`.
