# Datastar -- Overview

## What Is Datastar

Datastar is a reactive frontend framework that lets you build interactive UIs using only HTML `data-*` attributes. Drop a single 11.80 KiB script into your HTML and start adding reactivity without any build step, JSX, or component framework.

```html
<script type="module" src="https://cdn.jsdelivr.net/gh/starfederation/datastar@v1.0.1/bundles/datastar.js"></script>
<input data-bind:title />
<div data-text="$title.toUpperCase()"></div>
<button data-on:click="@post('/endpoint')">Save</button>
```

Underneath the simple API shape lies a sophisticated architecture: a fine-grained reactive signal system with lazy propagation and diamond dependency resolution, a DOM morphing algorithm that preserves component state across patches, and an SSE-based streaming protocol that lets servers push DOM updates and signal patches directly to the browser.

**Aha:** Datastar looks like Alpine.js on the surface (declarative `data-*` attributes on HTML elements), but underneath it uses a Solid.js-style signal system with versioned dependency graphs and lazy dirty-checking. This gives you the developer experience of "just add attributes" with the runtime performance of fine-grained reactivity — no virtual DOM diffing needed.

Source: `library/src/bundles/datastar.js` — 11.80 KiB bundle

## Architecture at a Glance

```mermaid
flowchart TB
    subgraph Bundles
        DS[datastar.js<br/>full bundle]
        CORE[datastar-core.js<br/>engine only]
        ALIAS[datastar-aliased.js<br/>custom prefix]
    end

    subgraph Engine
        ENG[engine.ts<br/>plugin registry, genRx,<br/>mutation observer]
        SIG[signals.ts<br/>ReactiveNode, Link,<br/>propagation, batching]
    end

    subgraph Plugins
        ACT[Action Plugins<br/>peek, setAll, toggleAll, fetch]
        ATTR[Attribute Plugins<br/>bind, on, show, class, style,<br/>text, attr, effect, computed,<br/>init, indicator, ref, signals,<br/>json-signals, on-intersect,<br/>on-interval, on-signal-patch]
        WATCH[Watcher Plugins<br/>patchElements, patchSignals]
    end

    subgraph Utils
        DOM[dom.ts]
        MATH[math.ts]
        PATHS[paths.ts]
        TEXT[text.ts]
        TIME[timing.ts]
        VT[view-transitions.ts]
    end

    DS --> ENG
    DS --> SIG
    DS --> ACT
    DS --> ATTR
    DS --> WATCH

    CORE --> ENG
    CORE --> SIG

    ALIAS --> ENG
    ALIAS --> SIG
    ALIAS --> ACT
    ALIAS --> ATTR
    ALIAS --> WATCH

    ENG -.uses.-> SIG
    ENG -.uses.-> TEXT
    ENG -.uses.-> PATHS
    ATTR -.uses.-> TIME
    ATTR -.uses.-> VT
    ATTR -.uses.-> MATH
    ACT -.uses.-> PATHS
    WATCH -.uses.-> DOM
    WATCH -.uses.-> TEXT
```

## Three Layers

| Layer | What it does | Key files |
|-------|-------------|-----------|
| **Engine** | Plugin registration, DOM mutation observation, expression compilation (`genRx`) | `engine/engine.ts`, `engine/signals.ts`, `engine/consts.ts`, `engine/types.ts` |
| **Plugins** | Concrete behaviors — 4 actions, 17 attributes, 2 watchers | `plugins/actions/*.ts`, `plugins/attributes/*.ts`, `plugins/watchers/*.ts` |
| **Utils** | Case conversion, timing wrappers, math helpers, path utilities | `utils/*.ts` |

## Bundles

Datastar ships three pre-built bundles:

| Bundle | Purpose |
|--------|---------|
| `datastar.js` | Full bundle — engine + all plugins. Use this for most projects. |
| `datastar-core.js` | Engine only — just `signal()`, `computed()`, `effect()`, `beginBatch()`, `endBatch()`, and plugin registration APIs. Use this to build custom plugin sets. |
| `datastar-aliased.js` | Same as `datastar.js` but with a custom `data-*` prefix (e.g., `data-myapp-*` instead of `data-*`). The `ALIAS` global is set at build time via esbuild `define`. |

## Quick Start Mental Model

```mermaid
flowchart LR
    HTML[HTML with data-* attributes] --> ENGINE[Engine scans DOM]
    ENGINE --> PLUGINS[Plugins register on elements]
    PLUGINS --> SIGNALS[Signals store]
    SIGNALS --> EFFECTS[Reactive effects fire]
    EFFECTS --> DOM[DOM updates]
```

1. **Scan**: Engine walks the DOM looking for `data-*` attributes
2. **Register**: Each attribute plugin claims elements matching its `data-*` pattern
3. **Compile**: Attribute expressions are compiled to JS Functions via `genRx`
4. **Execute**: Effects run, signals react, DOM updates

See [Architecture](01-architecture.md) for the full module dependency graph.
See [Reactive Signals](02-reactive-signals.md) for the signal system deep dive.
