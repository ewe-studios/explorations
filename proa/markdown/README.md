# Proa Documentation

Proa is a Rust server-side rendering framework that compiles HTML templates authored in Rust into
pre-escaped byte sequences at compile time, streams them over a chained-bump buffer, and hydrates
interactive islands via the RSJS Rust-to-JavaScript compiler. This is the complete reference: 27
documents that trace a component from `WebRenderSync` through `html_sync!` to emitted bytes, then follow
the bytes through streaming, islands, and out to the browser.

The guiding idea throughout is that rendering is a serialization problem, not a component-tree
reconstruction problem: emit as much HTML and as little JavaScript as possible, as fast as possible, from
one typed, compiled source of truth that renders as HTML for people, targeted behavior for browsers, and
dense machine-readable output for agents.

## Contents

| Doc | Topic |
|---|---|
| [00-overview.md](./00-overview.md) | What Proa is: philosophy, crates at a glance, multi-interface vision |
| [01-architecture.md](./01-architecture.md) | Crate graph, render pipeline, layering |
| [02-render-traits.md](./02-render-traits.md) | `WebRenderSync`/`WebRender`/`MdRenderSend` and the Send trait family |
| [03-web-context.md](./03-web-context.md) | `WebContext`, `SendWebContext`, `DataLoader`, `RenderOutcome` |
| [04-write-buf.md](./04-write-buf.md) | `WriteBuf` hierarchy, `RenderBuffer`, checkpoints |
| [05-html-elements.md](./05-html-elements.md) | `Elem`/`Tag`/`Attr`, `SafeText`/`UnsafeRaw`, escaping |
| [06-classlist-recipes.md](./06-classlist-recipes.md) | `ClassList`, `Variant`, `ProaRecipe`/`ProaVariant` derives |
| [07-html-macros.md](./07-html-macros.md) | `html!`/`html_sync!` lowering, consolidation, slots |
| [08-md-macros.md](./08-md-macros.md) | `md!`/`md_sync!`, markdown, html-to-markdown |
| [09-data-loading.md](./09-data-loading.md) | `LoadKey`/`LoadRequest`/`LoadValue`, cache, `ErrorBoundary` |
| [10-streaming.md](./10-streaming.md) | `Suspense`, drivers, `PagePool`, sinks, discovery |
| [11-ir.md](./11-ir.md) | `proa_ir`: expr/stmt/value, reactive, codegen, forms |
| [12-framework-core.md](./12-framework-core.md) | Manifest, bootstrap, body finalizers, Send policy |
| [13-framework-axum.md](./13-framework-axum.md) | Routing, static serving, HTTP, base path, CSP |
| [14-actions.md](./14-actions.md) | `proa_actions` event handlers and form actions |
| [15-build-tooling.md](./15-build-tooling.md) | `proa_build`, `proa_source_tools`, linking, linting |
| [16-cli.md](./16-cli.md) | `proa_cli` commands: lint/fmt/check/chunks/font/run |
| [17-rsjs-overview.md](./17-rsjs-overview.md) | Island model, signals, memos, handlers, queries |
| [18-rsjs-ir.md](./18-rsjs-ir.md) | `rsjs_ir`: side-tables, client IR v3, staged compiler ABI |
| [19-rsjs-macros.md](./19-rsjs-macros.md) | `#[rsjs]`, JavaScript lowering, route planning |
| [20-rsjs-web-transport.md](./20-rsjs-web-transport.md) | `rsjs_web` DOM markers, `rsjs_transport` |
| [21-fonts.md](./21-fonts.md) | `proa_fonts`: font pipeline, metric-matched fallback |
| [22-docs.md](./22-docs.md) | `proa_docs`: MDX and documentation rendering |
| [23-supporting-crates.md](./23-supporting-crates.md) | html_policy, dsl, codegen, react, tailwind-merge, const_json |
| [24-applications.md](./24-applications.md) | `proa_marketing`, `proa_demo`, `proa_showcase`, examples |
| [25-data-flow.md](./25-data-flow.md) | End-to-end request → render → stream → island |
| [26-cross-cutting.md](./26-cross-cutting.md) | Rust idioms, production-grade, resilient storage |

## How to read this

Each document teaches its key idea in the first paragraph, then goes deeper through numbered `Part`
dividers. Mermaid diagrams render every process; code snippets carry a `// path` header pointing at the
exact source file they abbreviate. Every document ends with an **Aha Moment** (the one insight the
chapter exists to deliver), a **Cross-References** section, and a `Continue to` footer that chains the
set into a single reading order.

The source of truth is the code. Every name, number, and flow in these pages was checked against the
Rust source in the Proa repository, and the rendered HTML is produced by `build.py` (Python 3 stdlib
only): `cd /Users/darkvoid/Dev/explorations && python3 build.py proa`.
