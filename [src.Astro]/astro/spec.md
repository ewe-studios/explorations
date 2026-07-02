---
title: "Astro 7.0.4 Documentation Spec"
---

# Astro 7.0.4 Documentation Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Astro/astro/`
- **Repository:** https://github.com/withastro/astro
- **Language:** TypeScript
- **Version:** 7.0.4
- **License:** MIT
- **Package Manager:** pnpm 11.5.0
- **Node:** >=22.12.0
- **Main Package:** `packages/astro` (v7.0.4)
- **Monorepo:** Yes — 12+ packages including `astro`, `create-astro`, `astro-prism`, `astro-rss`, `telemetry`, `internal-helpers`, `language-tools`, `underscore-redirects`, `upgrade`, and integration packages

## What Astro Is

Astro is a modern site builder that ships zero JavaScript to the browser by default. It is a content-focused web framework using an islands architecture — server-rendered HTML with selectively hydrated interactive components. Astro compiles `.astro` components via a Rust-based compiler (`@astrojs/compiler-rs`), orchestrates builds through Vite with multiple environments (SSR, prerender, client), and supports SSR via pluggable adapters (Node.js, Cloudflare, Vercel, Netlify, Deno, etc.). It also includes built-in content collections, image optimization, i18n routing, middleware, server actions, and a dev toolbar.

## Documentation Goal

After reading this documentation, a reader should understand:

1. Astro's architecture: monorepo structure, package exports, and build/runtime split
2. Configuration system: `AstroConfig` schema, settings, and Vite integration
3. Routing: file-based routing, dynamic params, redirects, and injected routes
4. Component model: `.astro` files, frontmatter, JSX rendering, and virtual modules
5. Vite plugins: 23 custom plugins and how they transform/serve/build
6. Build pipeline: three-environment sequential build (prerender, SSR, client)
7. SSR runtime: `App` class, manifest, request handling, and adapter integration
8. Integration API: hooks (`astro:build:setup`, `astro:config:setup`, etc.) and plugin injection
9. Content Collections: `defineCollection`, schema validation, content layers, live collections
10. Islands hydration: `astro-island` custom element, client directives (`client:*`), hydration ordering
11. Assets and images: image service pipeline, Sharp integration, font optimization
12. Middleware and Actions: `defineMiddleware`, `sequence`, server actions, `callAction`/`getActionResult`
13. i18n: locale resolution, pathname-prefix vs domain routing, fallback routes
14. Dev toolbar: preferences system, audit plugins, overlay rendering
15. Cross-cutting: CSP, caching, session management, error handling, telemetry

## Documentation Structure

```
[src.Astro]/astro/
├── spec.md                     ← This file: project tracker
├── markdown/                   ← Source documentation
│   ├── README.md               ← Index / table of contents
│   ├── 00-overview.md          ← What Astro is, philosophy, architecture at a glance
│   ├── 01-architecture.md      ← Monorepo, dependency graph, layers, module map
│   ├── 02-config-routing.md    ← Configuration system and routing
│   ├── 03-component-model.md   ← .astro components, JSX, virtual modules
│   ├── 04-vite-plugins.md      ← 23 custom Vite plugins
│   ├── 05-build-pipeline.md    ← Three-environment build process
│   ├── 06-ssr-runtime.md       ← SSR App class, manifest, request handling
│   ├── 07-integration-api.md   ← Integration hooks and plugin system
│   ├── 08-content-collections.md ← Content collections, schemas, loaders
│   ├── 09-islands-hydration.md  ← Islands architecture, hydration directives
│   ├── 10-assets-images.md     ← Image pipeline, fonts, asset handling
│   ├── 11-middleware-actions.md ← Middleware, server actions, API context
│   ├── 12-i18n.md              ← Internationalization and locale routing
│   ├── 13-dev-toolbar.md       ← Dev toolbar, preferences, audits
│   └── 14-cross-cutting.md     ← CSP, caching, session, errors, telemetry
├── html/                       ← Generated HTML
│   ├── index.html
│   ├── styles.css
│   └── *.html
└── build.py                    ← Shared Markdown → HTML script
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | spec.md | DONE |
| 1 | markdown/README.md | DONE |
| 2 | markdown/00-overview.md | DONE |
| 3 | markdown/01-architecture.md | DONE |
| 4 | markdown/02-config-routing.md | DONE |
| 5 | markdown/03-component-model.md | DONE |
| 6 | markdown/04-vite-plugins.md | DONE |
| 7 | markdown/05-build-pipeline.md | DONE |
| 8 | markdown/06-ssr-runtime.md | DONE |
| 9 | markdown/07-integration-api.md | DONE |
| 10 | markdown/08-content-collections.md | DONE |
| 11 | markdown/09-islands-hydration.md | DONE |
| 12 | markdown/10-assets-images.md | DONE |
| 13 | markdown/11-middleware-actions.md | DONE |
| 14 | markdown/12-i18n.md | DONE |
| 15 | markdown/13-dev-toolbar.md | DONE |
| 16 | markdown/14-cross-cutting.md | DONE |
| 17 | Copy build.py | DONE |
| 18 | Generate HTML | TODO |

## Build System

```bash
cd "[src.Astro]/astro" && python3 build.py    # build Astro docs
python3 build.py [src.Astro]/astro             # build from repo root
```

**Script:** `build.py` (copied from repo root)
**Dependencies:** None (Python 3.12+ stdlib only)

## Quality Requirements

All Iron Rules from the documentation directive apply:

1. Detailed sections with real code snippets and file path references
2. Teach key facts, principles, and ideas quickly — first paragraph is the thesis
3. Clear articulation — no overly complex sentences
4. Minimum 2 mermaid diagrams per document
5. Good visual assets (tables, ASCII art, mermaid)
6. Generated HTML with navigation, breadcrumbs, prev/next, theme toggle
7. Cross-references between documents
8. Source path references with file:line notation
9. At least 1 Aha moment per document
10. Navigation: index + prev/next buttons on every page

## Expected Outcome

After reading, a senior engineer should be able to:
- Contribute to the Astro codebase confidently
- Write integrations using the full hook API
- Debug build and runtime issues
- Understand the islands architecture implementation
- Configure and extend any aspect of Astro

## Resume Point

If interrupted, re-read spec.md to see which documents are marked DONE vs TODO, and continue from the first incomplete document.
