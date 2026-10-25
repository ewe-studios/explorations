# DivKit -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.sdui`
**Language:** Swift (iOS), Kotlin (Android), TypeScript (Web), Python (codegen/build), Dart (Flutter), JSON Schema (the contract)
**Version:** 33.9.0 (`divkit/version`)
**Author:** Yandex (DivKit / DivKit open source)
**License:** Apache-2.0
**Repository:** https://github.com/divkit/divkit (main), https://github.com/divkit/divkit-ios, https://github.com/divkit/divkit-ios-facade
**Tagline:** Server-Driven UI: one JSON format, native views on every platform

**Scale:** Three git repositories. `divkit` is the source of truth: 192 JSON
schema files that define the whole data format, the `api_generator` (Python)
that emits platform models from them, the four rendering clients (Android,
iOS, Web/TypeScript, Flutter), the server-side `json-builder` libraries, and
tooling (visual editor, Figma plugin). `divkit-ios` is the reference Swift
renderer: `DivKit` (223 hand-written + 207 generated Swift files) on top of
`LayoutKit` (311 files), a self-contained layout engine. `divkit-ios-facade`
is a thin stable-ABI wrapper. This is the real engine, not a thin write-up.

## What DivKit Is

DivKit is a Server-Driven UI (SDUI) framework. The server sends a card as
JSON; the client parses that JSON into a tree of models and renders it with
native views. The same JSON renders on iOS, Android, Web, and Flutter, so a
layout is written once and shipped everywhere, and a server can change an app's
UI without an app-store release.

The whole design rests on one idea: **the JSON schema is the single source of
truth.** `divkit/schema/` holds 192 JSON Schema files that describe every
element (`div-text`, `div-container`, ...), every action, every variable, and
every style. A Python tool, `api_generator`, reads those schemas and generates
the Swift, Kotlin, TypeScript, and Dart model types each client compiles
against. The clients are therefore *generated*, not hand-written, against the
contract: the schema's `codegen` blocks steer how each platform's models,
factories, and serializers are produced. A card is a small JSON document with
`templates` (reusable, parameterized sub-trees) and `card` (a `div-data`
object: an ordered list of `states`, a `log_id`, and optional `variables`,
`variable_triggers`, `timers`, and `functions`). Interaction is carried by
`actions` (typed, or a `div-action://` URL), and `set_state` switches the
visible state through a path like `0/nested_state/selected`.

On iOS the pipeline is `JSON -> DivKit models -> LayoutKit blocks -> UIView`.
`DivKit` owns the Div-specific concerns (state, actions, expressions,
templates, variables, timers, resources, images). `LayoutKit` owns geometry:
a block tree computes size and position through a measure-then-layout pass,
with its own box model (width/height, paddings, margins, alignment, spans).

## Documentation Goal

A reader who finishes these documents can:

1. Explain what Server-Driven UI is, why the schema is the source of truth,
   and what one schema generating four platforms buys.
2. Read and write a DivKit card: `div-data`, `states`, `log_id`, `templates`
   (with `$`-prefixed parameters), and the 17 element types.
3. Describe the common element model (`div-base`): width/height, paddings,
   margins, background, alignment, spans, and the size system
   (`match_parent` / `wrap_content` / fixed; only the pager's page width uses
   percentage).
4. Explain actions and state: typed actions, URL actions, `set_state` and the
   state-path notation, `log_id`.
5. Explain variables and expressions: `@{}` expressions, `variables`,
   `variable_triggers`, `timers`, `functions`, and how data binding stays in
   the schema.
6. Describe styling: backgrounds, borders, shadows, gradients, transforms,
   transitions, and animators.
7. Explain code generation: how `api_generator` reads the schema and emits
   Swift/Kotlin/TypeScript/Dart models, factories, and serializers, and what
   the schema's `codegen` and `$description`/`translations.json` machinery does.
8. Trace the iOS renderer: `DivKitComponents`/`DivBlockModeling`, the
   `generated_sources/`, serialization, and how JSON becomes a `UIView`.
9. Explain `LayoutKit`: the block/view-model split, the measure-then-layout
   pass, and the box model.
10. Describe the Web, Android, Flutter, and Kotlin-Multiplatform clients, the
    server-side `json-builder` libraries, the binary-compatibility facade, and
    the tooling (visual editor, Figma plugin).
11. Trace one end-to-end flow: server JSON to native view, a user tap, and the
    resulting action, state change, and re-render.

## Documentation Structure

```
divkit/
├── spec.md                          ← THIS FILE (project tracker)
├── markdown/                        ← Source documentation
│   ├── README.md                    ← Index / table of contents
│   ├── 00-overview.md               ← SDUI, one schema many platforms, the three repos at a glance
│   ├── 01-architecture.md           ← schema → codegen → renderers, the dependency map
│   ├── 02-div-format.md             ← div-data, card, states, templates, div.json dispatch
│   ├── 03-elements.md               ← the 17 elements and the shared div-base model
│   ├── 04-layout-and-size.md        ← sizes, box model, container/grid/gallery geometry
│   ├── 05-actions-and-state.md      ← typed + URL actions, set_state paths, log_id
│   ├── 06-variables-expressions.md  ← @{} expressions, variables, triggers, timers, functions
│   ├── 07-styling-and-animation.md  ← backgrounds, borders, gradients, transforms, transitions, animators
│   ├── 08-codegen.md                ← api_generator: schema → Swift/Kotlin/TS/Dart
│   ├── 09-schema.md                 ← the schema format in depth: common.json, codegen blocks, translations.json
│   ├── 10-ios-architecture.md       ← DivKit iOS: components, modeling, the pipeline
│   ├── 11-layoutkit.md              ← the layout engine: blocks, view models, measure/layout
│   ├── 12-ios-rendering.md          ← serialization, patch/update, images, resources, preloading
│   ├── 13-web-client.md             ← TypeScript renderer and divkit-react
│   ├── 14-android-client.md         ← Kotlin renderer
│   ├── 15-flutter-kmp.md            ← Flutter and Kotlin Multiplatform clients
│   ├── 16-json-builder.md           ← server-side builders: TypeScript, Kotlin, Python
│   ├── 17-facade.md                 ← DivKitBinaryCompatibilityFacade (stable ABI)
│   ├── 18-tooling.md                ← visual editor, Figma plugin, compat data
│   ├── 19-data-flow.md              ← one card end to end: JSON → view → tap → action → re-render
│   ├── 20-cross-cutting.md          ← idioms, production-grade, and building an SDUI engine from first principles
│   ├── 21-rendering-and-patch.md    ← JSON → view pipeline, and how iOS/Android/Web apply a fragment without a full rebuild
│   ├── 22-web-rendering-hydration.md ← web entry points, SSR + hydration, host↔card communication
│   ├── 23-live-sync-sandbox.md      ← the visual editor and the WebSocket live sync to native apps
│   ├── 24-native-rendering-performance.md ← caching, recycling, batching, prebinding, constant folding
│   ├── 25-issues-prs-delivery.md    ← bugfix/perf history, bundle size, server delivery
│   └── 26-platform-geometry-compilation.md ← one layout, three geometry authorities: Android LayoutParams/Gravity, Web CSS, iOS LayoutKit
└── html/                            ← Rendered HTML (build.py output)
    ├── index.html
    ├── styles.css
    └── *.html
```

## Tasks

### Phase 1: Core Documentation (Markdown)

| # | Task | Status | File |
|---|------|--------|------|
| 1 | Overview: SDUI, one schema many platforms, three repos | DONE | `00-overview.md` |
| 2 | Architecture: schema → codegen → renderers | DONE | `01-architecture.md` |
| 3 | Div format: div-data, card, states, templates | DONE | `02-div-format.md` |
| 4 | Elements: 17 types and div-base | DONE | `03-elements.md` |
| 5 | Layout and size: box model, container/grid/gallery | DONE | `04-layout-and-size.md` |
| 6 | Actions and state: typed + URL, set_state paths | DONE | `05-actions-and-state.md` |
| 7 | Variables and expressions: @{}, triggers, timers, functions | DONE | `06-variables-expressions.md` |
| 8 | Styling and animation: backgrounds, transforms, transitions, animators | DONE | `07-styling-and-animation.md` |
| 9 | Codegen: api_generator → Swift/Kotlin/TS/Dart | DONE | `08-codegen.md` |
| 10 | Schema: common.json, codegen blocks, translations.json | DONE | `09-schema.md` |
| 11 | iOS architecture: components, modeling, pipeline | DONE | `10-ios-architecture.md` |
| 12 | LayoutKit: blocks, view models, measure/layout | DONE | `11-layoutkit.md` |
| 13 | iOS rendering: serialization, patch, images, resources | DONE | `12-ios-rendering.md` |
| 14 | Web client: TypeScript renderer, divkit-react | DONE | `13-web-client.md` |
| 15 | Android client: Kotlin renderer | DONE | `14-android-client.md` |
| 16 | Flutter and KMP clients | DONE | `15-flutter-kmp.md` |
| 17 | json-builder: TypeScript, Kotlin, Python | DONE | `16-json-builder.md` |
| 18 | Facade: DivKitBinaryCompatibilityFacade | DONE | `17-facade.md` |
| 19 | Tooling: visual editor, Figma plugin | DONE | `18-tooling.md` |
| 20 | Data flow: one card end to end | DONE | `19-data-flow.md` |
| 21 | Cross-cutting: idioms, production-grade, from first principles | DONE | `20-cross-cutting.md` |
| 22 | README index | DONE | `README.md` |
| 23 | Rendering and patch: JSON→view pipeline, and patch/partial-update optimization across iOS/Android/Web | DONE | `21-rendering-and-patch.md` |
| 24 | Web rendering, SSR/hydration, host↔card communication | DONE | `22-web-rendering-hydration.md` |
| 25 | Sandbox (visual editor) and WebSocket live sync to native apps | DONE | `23-live-sync-sandbox.md` |
| 26 | Native rendering optimization and production tricks | DONE | `24-native-rendering-performance.md` |
| 27 | Issues, PRs, bundle size, server delivery | DONE | `25-issues-prs-delivery.md` |
| 28 | Platform geometry: one layout → Android LayoutParams/Gravity + Web CSS | DONE | `26-platform-geometry-compilation.md` |

### Phase 2: HTML Rendering

| # | Task | Status | File |
|---|------|--------|------|
| 29 | Generate all HTML pages | DONE | `html/*.html` |
| 30 | Index page with navigation | DONE | `html/index.html` |
| 31 | Shared CSS | DONE | `html/styles.css` |

### Phase 3: Grandfather Review

| # | Task | Status |
|---|------|--------|
| 32 | Names match source | DONE |
| 33 | Numbers match source | DONE |
| 34 | Flows match source | DONE |
| 35 | Nothing missing from public API | DONE |

## Build System

**Script:** `/Users/darkvoid/Dev/explorations/build.py`
**Dependencies:** None (Python 3 stdlib only)

**Usage:**
```bash
cd /Users/darkvoid/Dev/explorations
python3 build.py divkit
```

The `descriptions` dict in `build.py` needs a `divkit` entry (added when the
first build runs).

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
11. All prose in ASD-STE100 Simplified Technical English: every abbreviation
    spelled out at first use, short sentences, one idea per sentence.

## Depth Requirement

DivKit spans three repos and roughly a thousand source files across Swift,
Kotlin, TypeScript, Python, and Dart. The schema (192 files) and the iOS
renderer (`DivKit` + `LayoutKit`) are the reference and get the deepest
coverage. Every element type, every action, every variable/expression feature,
every codegen target, and every client must be documented so a reader never has
to open the source to understand the behavior. No section may hand-wave a
layout algorithm or a codegen step.

## Grandfather Review

The code is the grandfather. Review checks four axes:

1. Names match source: functions, classes, fields, module names, schema names.
2. Numbers match source: defaults, sizes, element counts, schema file counts.
3. Flows match source: the JSON-to-view pipeline, codegen stages, action paths.
4. Nothing missing from the public API surface (elements, actions, variables).

Every name, number, and flow in this documentation must be checked against the
source in `/Users/darkvoid/GitSources/src.sdui/`.

The review is complete and was performed by three independent verification
agents (docs 11-14, 21-23, 24-26), not by the authors who drafted them. Each
agent read the source directly and corrected the docs in place. Every doc
passed all four axes.

## Resume Point

To resume: finish any `TODO` task above in document order, read the relevant
source files in `/Users/darkvoid/GitSources/src.sdui/` before writing, write the
markdown, run `python3 build.py divkit`, and run the grandfather review (names,
numbers, flows, coverage) against the source before marking the task DONE.
