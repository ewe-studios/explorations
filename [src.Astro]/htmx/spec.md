# HTMX Ecosystem — Documentation Spec

## 1. Source Codebase

- **Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Astro/src.htmx/`
- **Language:** JavaScript
- **Primary library:** htmx v2.0.10 (`htmx.js`, 5342 lines, 0BSD license)
- **Companion libraries:**
  - `_hyperscript.js` v0.9.93 (BSD 2-Clause)
  - `idiomorph.js` (1383 lines)
  - `intercooler.js` v1.2.4 (2084 lines)
  - `fixi.js` (~3.5 KB, 90 lines)
  - `moxi.js` (110 lines)
  - `rexi.js` (115 lines)
  - `ssexi.js` (89 lines)
  - `missing.css` v1.3.0 (classless CSS)
- **GitHub repos:**
  - https://github.com/bigskysoftware/htmx
  - https://github.com/bigskysoftware/_hyperscript
  - https://github.com/bigskysoftware/idiomorph
  - https://github.com/bigskysoftware/intercooler-js
  - https://github.com/bigskysoftware/fixi
  - https://github.com/bigskysoftware/missing

## 2. What This Project Is

The HTMX ecosystem is a family of JavaScript libraries that enable hypermedia-driven web applications — servers return HTML fragments in response to user interactions, and client-side libraries swap those fragments into the DOM without full page reloads. htmx (v2.0.10) is the core library, providing AJAX, CSS Transitions, WebSockets, and Server-Sent Events via HTML attributes. Its companion libraries extend this model: `_hyperscript` adds a lightweight scripting language for local interactivity, `idiomorph` provides intelligent DOM morphing with ID-based matching across 9 swap strategies, `fixi`/`moxi`/`rexi`/`ssexi` form a minimalist toolkit for hypermedia controls, local event handlers, fluent fetch, and SSE streaming, and `missing.css` supplies a classless CSS baseline. `intercooler.js` (v1.2.4) is the jQuery-based predecessor that established the attribute-driven hypermedia pattern htmx later refined.

## 3. Documentation Goal

A reader who completes this documentation will:

1. Understand the hypermedia-driven architecture and how it differs from SPA frameworks
2. Configure htmx via its 30+ `hx-` attributes and 9 swap strategies
3. Build AJAX-powered UIs using `hx-get`, `hx-post`, `hx-put`, `hx-patch`, `hx-delete`, and `hx-trigger`
4. Use the 24+ htmx extensions (e.g., `json-enc`, `ws`, `sse`, `preload`, `loading-states`)
5. Write `_hyperscript` commands and features for local interactivity without JavaScript frameworks
6. Apply `idiomorph` for intelligent DOM morphing during out-of-band swaps
7. Use the fixi/moxi/rexi/ssexi toolkit for minimalist hypermedia patterns
8. Integrate Server-Sent Events (`ssexi`) and WebSockets for real-time server push
9. Style hypermedia applications with `missing.css` classless CSS
10. Trace the historical lineage from `intercooler.js` through htmx v1 to htmx v2

## 4. Documentation Structure

```
htmx/
├── spec.md                          ← This file
├── markdown/
│   ├── README.md                    ← Index: ecosystem overview, document map
│   ├── 00-overview.md               ← What HTMX is, hypermedia philosophy, ecosystem map
│   ├── 01-architecture.md           ← htmx core: request lifecycle, swap engine, extension system
│   ├── 02-htmx-attributes.md        ← All hx-* attributes, config options, trigger modifiers
│   ├── 03-swap-strategies.md        ← 9 swap strategies with DOM before/after diagrams
│   ├── 04-htmx-extensions.md        ← 24+ extensions catalog, usage patterns, extension API
│   ├── 05-hyperscript.md            ← _hyperscript: tokenizer, kernel, parser, runtime, commands
│   ├── 06-idiomorph.md              ← DOM morphing algorithm, ID matching, morph options
│   ├── 07-fixi-moxi-rexi-ssexi.md   ← Minimalist toolkit: fixi controls, moxi events, rexi fetch, ssexi SSE
│   ├── 08-intercooler.md            ← intercooler.js v1.2.4: predecessor patterns, jQuery integration
│   ├── 09-missing-css.md            ← missing.css v1.3.0: classless CSS design system
│   ├── 10-data-flow.md              ← End-to-end: browser event → AJAX → server response → DOM swap
│   ├── 11-extension-authoring.md    ← How to write htmx extensions (extension API, lifecycle hooks)
│   └── 12-comparisons.md            ← HTMX vs SPAs vs HTMX+Hyperscript vs Alpine.js vs Stimulus
├── html/
│   ├── index.html
│   ├── styles.css
│   └── *.html                       ← Generated from markdown
└── build.py                         ← Shared Markdown → HTML script
```

## 5. Tasks

| # | Document | Phase | Status |
|---|----------|-------|--------|
| 1 | `spec.md` | Planning | TODO |
| 2 | `markdown/README.md` | Foundation | TODO |
| 3 | `markdown/00-overview.md` | Foundation | TODO |
| 4 | `markdown/01-architecture.md` | Architecture | TODO |
| 5 | `markdown/02-htmx-attributes.md` | Core | TODO |
| 6 | `markdown/03-swap-strategies.md` | Core | TODO |
| 7 | `markdown/04-htmx-extensions.md` | Core | TODO |
| 8 | `markdown/05-hyperscript.md` | Companion | TODO |
| 9 | `markdown/06-idiomorph.md` | Companion | TODO |
| 10 | `markdown/07-fixi-moxi-rexi-ssexi.md` | Companion | TODO |
| 11 | `markdown/08-intercooler.md` | Historical | TODO |
| 12 | `markdown/09-missing-css.md` | Styling | TODO |
| 13 | `markdown/10-data-flow.md` | Cross-Cutting | TODO |
| 14 | `markdown/11-extension-authoring.md` | Deep Dive | TODO |
| 15 | `markdown/12-comparisons.md` | Deep Dive | TODO |
| 16 | Grandfather Review | Verification | TODO |
| 17 | HTML generation + visual QA | Build | TODO |

## 6. Build System

- **Script:** `documentation/build.py` (shared across all projects)
- **Dependencies:** Python 3.12+ stdlib only (no pip packages)
- **Usage:**
  ```bash
  cd documentation && python3 build.py          # build all projects
  python3 build.py htmx                          # build htmx only
  ```
- **Features:** Markdown-to-HTML conversion, frontmatter title extraction, auto-generated index, Mermaid CDN loader, dark/light theme toggle, prev/next navigation, responsive CSS
- **Output:** `html/` directory with `index.html`, `styles.css`, and one HTML file per markdown source

## 7. Quality Requirements (Iron Rules)

All ten rules from the documentation directive apply verbatim:

1. **Detailed sections with code snippets** — every concept grounded in actual source code with accurate function signatures and logic
2. **Teach key facts quickly** — insight density; first sentence is the thesis
3. **Clear articulation** — one idea per sentence; logical flow; mermaid for processes, tables for comparisons
4. **Mermaid diagrams** — minimum 2 per document; flowcharts, sequence diagrams, class diagrams
5. **Good visual assets** — tables, ASCII art, code blocks — each adding information prose cannot convey as efficiently
6. **Generated HTML** — builds via `build.py`; aligned headers, consistent heading hierarchy, organized index, clear navigation
7. **Cross-references** — no orphan pages; every document links to at least 2 related documents
8. **Source path references** — actual file paths and line numbers so readers can verify claims against the grandfather
9. **Aha moments** — surface non-obvious design decisions, clever tradeoffs, "wait, that's smart" insights (minimum 1 per document)
10. **Navigation** — every HTML page has brand, breadcrumbs, index button, prev/next pills, theme toggle

Plus the mandatory depth requirement: no section should require source-code lookup for comprehension. Every function, algorithm, and data structure must be explained fully within the documentation itself.

## 8. Expected Outcome

A reader who completes this documentation will be able to build a hypermedia-driven web application using htmx and its companion libraries without needing a JavaScript framework. They will understand how htmx intercepts browser events, issues AJAX requests, processes server responses through its swap engine, and applies DOM morphing via idiomorph. They will know when to use `_hyperscript` for local interactivity, when to reach into the fixi/moxi/rexi/ssexi toolkit, and how to author custom extensions. They will be able to debug htmx applications by tracing the request lifecycle, interpret htmx events, and configure the 30+ options for their use case.

## 9. Resume Point

If interrupted, check the **Tasks** table above. Pick the next document with status `TODO` and begin Phase 3 (Write the Markdown) per the documentation directive. If all documents are written, proceed to Phase 5 (Grandfather Review). The build system lives at `documentation/build.py`. All source files are at `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Astro/src.htmx/`.
