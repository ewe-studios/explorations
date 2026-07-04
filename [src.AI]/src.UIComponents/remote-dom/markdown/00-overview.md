# remote-dom: DOM Rendering from Sandboxed Environments

Take a tree of DOM elements created in a sandboxed environment (iframe, web worker) and render them to the DOM in a different environment.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/remote-dom/`

## Architecture

```
Remote Environment (iframe/worker)    Message Channel    Host Environment (main thread)
───────────────────────────────       ──────────────     ──────────────────────────────
Remote DOM Tree                       postMessage/       DOMRemoteReceiver
RemoteConnection ───────────────────▶ MessagePort ────▶ Creates real DOM elements
```

**Aha:** Remote DOM serializes individual DOM *operations* (createElement, appendChild, setAttribute) rather than the entire DOM tree. Updates are incremental and efficient.

## 6 Packages

| Package | Files | Purpose |
|---------|-------|---------|
| `@remote-dom/core` | 7 | Core connection and receiver types |
| `@remote-dom/react` | 6 | React bindings for remote rendering |
| `@remote-dom/preact` | 5 | Preact bindings (lightweight) |
| `@remote-dom/signals` | 2 | Signal-based reactive receiver |
| `@remote-dom/compat` | 1 | Compatibility layer |
| `@remote-dom/polyfill` | 30+ | Minimal DOM API polyfill for workers |

## Core API

```typescript
// Host side
const receiver = new DOMRemoteReceiver()
receiver.connect(rootElement)

// Remote side
const connection = new RemoteConnection(postMessage, onMessage)
connection.createElement('div')
connection.appendChild(parent, child)
```

## Polyfill

Provides minimal DOM APIs so Preact/Svelte can run in a web worker:

| API | File |
|-----|------|
| `Attr` | `Attr.ts` |
| `Document` | `Document.ts` |
| `Element` | `Element.ts` |
| `Text` | `Text.ts` |
| `MutationObserver` | `MutationObserver.ts` |
| `CustomElementRegistry` | `CustomElementRegistry.ts` |

## Use Cases

1. **Plugin UI isolation** — Run plugin UI in iframe, render to main page
2. **Web worker UI** — Run UI libraries in worker, render to main thread
3. **Sandboxed content** — Render untrusted content safely
