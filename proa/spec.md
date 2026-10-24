# Proa Documentation -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.proa/proa`
**Language:** Rust (edition 2021, `#![deny(unsafe_code)]`)
**Version:** `0.3.0` coordinated release
**Author:** Proa Labs, Inc.
**License:** Apache-2.0
**Repository:** https://github.com/proa-labs/proa
**Homepage:** https://proa.so
**CLI binary:** `proa` (`cargo install proa-cli --locked`)

**Scale:** 41 workspace members, 1,252 Rust source files.

## What Proa Is

Proa is a Rust server-side rendering (SSR) framework. It compiles HTML templates
authored in Rust into optimized byte sequences at compile time. A component is a
plain Rust struct that implements a render trait (`WebRenderSync` or
`WebRender`). The `html_sync!` / `html!` macros validate the template at compile
time, consolidate adjacent static markup, and preserve correct escaping for
dynamic slots. Interactive islands are compiled by RSJS, a Rust-to-JavaScript
compiler, so the browser starts from server-rendered HTML and loads only the
small, route-planned JavaScript the island needs.

The guiding idea is a serialization problem, not a component-tree reconstruction
problem: emit as much HTML and as little JavaScript as possible, as fast as
possible. One typed, compiled interface layer produces the right representation
for whoever asks: HTML for people, targeted behavior for browsers, dense
machine-readable output for agents.

## Documentation Goal

A reader who finishes these documents can:

1. Explain the compiled-template model and why it treats rendering as serialization.
2. Trace a Rust component from `WebRenderSync` through `html_sync!` to emitted bytes.
3. Describe the `WriteBuf` hierarchy and the zero-allocation, zero-copy patterns.
4. Explain `WebContext`, `DataLoader`, `RenderOutcome`, and the sync/async split.
5. Describe the Markdown render path (`MdRenderSync` / `md_sync!`).
6. Explain the streaming substrate: `Suspense`, out-of-order drivers, page pools.
7. Describe the IR (`proa_ir`) and how reactive attributes lower to client glue.
8. Explain RSJS islands: signals, memos, handlers, queries, and JavaScript lowering.
9. Describe the framework adapters, routing, static serving, and HTTP integration.
10. Describe the build tooling, source lints, CLI, fonts, and docs pipelines.
11. See the Rust idioms used, what a production-grade version adds, and how to build
    a resilient storage system like this from first principles.

## Documentation Structure

```
proa/
├── spec.md                          ← THIS FILE (project tracker)
├── markdown/                        ← Source documentation
│   ├── README.md                    ← Index / table of contents
│   ├── 00-overview.md               ← What Proa is, philosophy, crates at a glance
│   ├── 01-architecture.md           ← Crate graph, render pipeline, layering
│   ├── 02-render-traits.md          ← WebRenderSync/WebRender/Md/Send trait family
│   ├── 03-web-context.md            ← WebContext, SendWebContext, DataLoader, RenderOutcome
│   ├── 04-write-buf.md              ← WriteBuf hierarchy, RenderBuffer, checkpoints
│   ├── 05-html-elements.md          ← Elem/Tag/Attr/text/SafeText/UnsafeRaw, escaping
│   ├── 06-classlist-recipes.md      ← ClassList, Variant, ProaRecipe/ProaVariant derives
│   ├── 07-html-macros.md            ← html!/html_sync! lowering, consolidation, slots
│   ├── 08-md-macros.md              ← md!/md_sync!, markdown, html-to-markdown
│   ├── 09-data-loading.md           ← LoadKey/LoadRequest/LoadValue, cache, ErrorBoundary
│   ├── 10-streaming.md              ← Suspense, drivers, PagePool, sinks, discovery
│   ├── 11-ir.md                     ← proa_ir: expr/stmt/value, reactive, codegen, form
│   ├── 12-framework-core.md         ← Manifest, bootstrap, body finalizers, Send policy
│   ├── 13-framework-axum.md         ← Routing, static serving, HTTP, base path, CSP
│   ├── 14-actions.md                ← proa_actions event handlers and form actions
│   ├── 15-build-tooling.md          ← proa_build, proa_source_tools, linking, linting
│   ├── 16-cli.md                    ← proa_cli commands: lint/fmt/check/chunks/font/run
│   ├── 17-rsjs-overview.md          ← Island model, signals, memos, handlers, queries
│   ├── 18-rsjs-ir.md                ← rsjs_ir
│   ├── 19-rsjs-macros.md            ← #[rsjs], JavaScript lowering, route planning
│   ├── 20-rsjs-web-transport.md     ← rsjs_web DOM markers, rsjs_transport
│   ├── 21-fonts.md                  ← proa_fonts: font pipeline, metric-matched fallback
│   ├── 22-docs.md                   ← proa_docs: MDX and documentation rendering
│   ├── 23-supporting-crates.md      ← html_policy, dsl, codegen, react, tailwind-merge, const_json
│   ├── 24-applications.md           ← proa_marketing, proa_demo, proa_showcase, examples
│   ├── 25-data-flow.md              ← End-to-end request → render → stream → island
│   └── 26-cross-cutting.md          ← Rust idioms, production-grade, resilient storage
└── html/                            ← Rendered HTML (build.py output)
    ├── index.html
    ├── styles.css
    └── *.html
```

## Tasks

### Phase 1: Core Documentation (Markdown)

| # | Task | Status | File |
|---|------|--------|------|
| 1 | Overview: philosophy, crates, multi-interface vision | DONE | `00-overview.md` |
| 2 | Architecture: crate graph, render pipeline, layering | DONE | `01-architecture.md` |
| 3 | Render traits: sync/async/markdown/send family | DONE | `02-render-traits.md` |
| 4 | Web context: WebContext, DataLoader, RenderOutcome | DONE | `03-web-context.md` |
| 5 | WriteBuf hierarchy, RenderBuffer, checkpoints | DONE | `04-write-buf.md` |
| 6 | HTML elements, attributes, escaping | DONE | `05-html-elements.md` |
| 7 | ClassList, recipes, variants | DONE | `06-classlist-recipes.md` |
| 8 | html!/html_sync! macros | DONE | `07-html-macros.md` |
| 9 | md!/md_sync! markdown macros | DONE | `08-md-macros.md` |
| 10 | Data loading, cache, error boundary | DONE | `09-data-loading.md` |
| 11 | Streaming: Suspense, drivers, page pool | DONE | `10-streaming.md` |
| 12 | proa_ir: expr, reactive, codegen | DONE | `11-ir.md` |
| 13 | Framework core: manifest, bootstrap, finalizers | DONE | `12-framework-core.md` |
| 14 | Framework axum: routing, static, HTTP | DONE | `13-framework-axum.md` |
| 15 | Actions: event handlers, forms | DONE | `14-actions.md` |
| 16 | Build tooling: build, source tools, linking | TODO | `15-build-tooling.md` |
| 17 | CLI commands | TODO | `16-cli.md` |
| 18 | RSJS overview: islands, signals, memos | TODO | `17-rsjs-overview.md` |
| 19 | rsjs_ir | TODO | `18-rsjs-ir.md` |
| 20 | rsjs_macros: #[rsjs], JS lowering | TODO | `19-rsjs-macros.md` |
| 21 | rsjs_web + rsjs_transport | TODO | `20-rsjs-web-transport.md` |
| 22 | Fonts pipeline | TODO | `21-fonts.md` |
| 23 | Docs (MDX) rendering | TODO | `22-docs.md` |
| 24 | Supporting crates | TODO | `23-supporting-crates.md` |
| 25 | Applications and examples | TODO | `24-applications.md` |
| 26 | Data flow end-to-end | TODO | `25-data-flow.md` |
| 27 | Cross-cutting: Rust, production, resilient storage | TODO | `26-cross-cutting.md` |
| 28 | README index | TODO | `README.md` |

### Phase 2: HTML Rendering

| # | Task | Status | File |
|---|------|--------|------|
| 29 | Generate all HTML pages | TODO | `html/*.html` |
| 30 | Index page with navigation | TODO | `html/index.html` |
| 31 | Shared CSS | TODO | `html/styles.css` |

### Phase 3: Grandfather Review

| # | Task | Status |
|---|------|--------|
| 32 | Names match source | TODO |
| 33 | Numbers match source | TODO |
| 34 | Flows match source | TODO |
| 35 | Nothing missing from public API | TODO |

## Build System

**Script:** `/Users/darkvoid/Dev/explorations/build.py`
**Dependencies:** None (Python 3 stdlib only)

**Usage:**
```bash
cd /Users/darkvoid/Dev/explorations
python3 build.py proa
```

## Quality Requirements (Iron Rules)

1. Detailed sections with real code snippets and source paths.
2. Teach the key idea in the first paragraph, then go deeper.
3. Short, clear sentences. Mermaid diagrams for every process.
4. At least two Mermaid diagrams per document.
5. At least three code snippets per document.
6. At least one Aha moment per document.
7. At least two cross-references per document.
8. Generated HTML via the shared build.py.
9. No orphan pages.
10. Navigation: index button, prev/next buttons, theme toggle.

## Depth Requirement

1,252 Rust files places Proa in the Massive tier: 25+ documents, 50,000+ words.
Every public API surface, every important private helper, every constant and
default, and every error variant must be documented. No section may require a
reader to open the source to understand the behavior.

## Grandfather Review

The code is the grandfather. Review checks four axes:

1. Names match source: types, functions, constants, table names.
2. Numbers match source: defaults, capacities, timeouts, frame limits.
3. Flows match source: state transitions, call graphs.
4. Nothing missing from the public API surface.

Every name, number, and flow in this documentation must be checked against the
source files in `/Users/darkvoid/GitSources/src.proa/proa`.

## Resume Point

To resume: finish any `TODO` task above in document order, write the markdown,
run `python3 build.py proa`, and run the grandfather review (names, numbers,
flows, coverage) against the source before marking the task DONE.
