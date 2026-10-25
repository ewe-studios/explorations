---
title: "DivKit — Server-Driven UI — Documentation Index"
---

# DivKit Documentation

A deep documentation of DivKit, Yandex's Server-Driven UI framework: one JSON
card renders as native views on iOS, Android, Web, and Flutter. The JSON schema
is the single source of truth; a Python generator compiles it into the models
four renderers read and the builders three server languages write. This index
covers all 27 documents, from the card format to the layout engine to the
production tooling.

## Foundation

- [Overview](00-overview.md) — Server-Driven UI, one schema many platforms, the three repositories at a glance
- [Architecture](01-architecture.md) — schema → codegen → renderers, the dependency map

## The Format

- [Div Format](02-div-format.md) — `div-data`, card, states, templates, the `div.json` dispatch
- [Elements](03-elements.md) — the 17 element types and the shared `div-base` model
- [Layout and Size](04-layout-and-size.md) — sizes, the box model, container/grid/gallery geometry
- [Actions and State](05-actions-and-state.md) — typed and URL actions, `set_state` paths, `log_id`
- [Variables and Expressions](06-variables-expressions.md) — `@{...}` expressions, variables, triggers, timers, functions
- [Styling and Animation](07-styling-and-animation.md) — backgrounds, borders, gradients, transforms, transitions, animators

## Codegen and Schema

- [Codegen](08-codegen.md) — `api_generator`: schema → Swift/Kotlin/TypeScript/Dart
- [Schema](09-schema.md) — the schema format in depth: `common.json`, `codegen` blocks, `translations.json`

## The iOS Renderer

- [iOS Architecture](10-ios-architecture.md) — `DivKitComponents`, `DivBlockModeling`, the JSON-to-`UIView` pipeline
- [LayoutKit](11-layoutkit.md) — the layout engine: blocks, view models, the measure-then-layout pass
- [iOS Rendering](12-ios-rendering.md) — serialization, patch/update, images, resources, preloading

## The Other Clients

- [Web Client](13-web-client.md) — the TypeScript renderer (Svelte) and the React wrapper
- [Android Client](14-android-client.md) — the Kotlin renderer and its view pool
- [Flutter and Kotlin Multiplatform](15-flutter-kmp.md) — the Dart port and the Compose DSL over native renderers
- [Web Rendering and Hydration](22-web-rendering-hydration.md) — the three web entry points, SSR + hydration, and the host↔card communication handle

## Server and Ecosystem

- [JSON Builders](16-json-builder.md) — the server-side builders: TypeScript, Kotlin, Python
- [Binary Compatibility Facade](17-facade.md) — a stable ABI in front of DivKit
- [Tooling](18-tooling.md) — the visual editor, the Figma plugin, the compatibility matrix
- [Sandbox and Live Sync](23-live-sync-sandbox.md) — the visual editor and the WebSocket that renders one card natively on Android and iOS

## Synthesis

- [Data Flow](19-data-flow.md) — one card end to end: JSON → view → tap → action → re-render
- [Cross-Cutting](20-cross-cutting.md) — the idioms, the production concerns, and the first-principles recipe
- [Rendering and Patch](21-rendering-and-patch.md) — the JSON-to-view pipeline, and how iOS/Android/Web apply a fragment without a full rebuild
- [Native Rendering Optimization](24-native-rendering-performance.md) — the caching, recycling, batching, and prebinding that make the native renderers fast
- [Platform Geometry](26-platform-geometry-compilation.md) — how one Div layout compiles to Android `LayoutParams`/`Gravity` and Web CSS flexbox
- [Issues, PRs, Bundle Size, and Delivery](25-issues-prs-delivery.md) — the bugfix/perf history, bundle-size tooling, and server delivery

## Source Repositories

| Repository | Version | GitHub |
|------------|---------|--------|
| `divkit` | 33.9.0 | https://github.com/divkit/divkit |
| `divkit-ios` | — | https://github.com/divkit/divkit-ios |
| `divkit-ios-facade` | — | https://github.com/divkit/divkit-ios-facade |
