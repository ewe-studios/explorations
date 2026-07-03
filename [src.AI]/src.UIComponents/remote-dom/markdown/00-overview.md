# remote-dom: DOM Rendering from Sandboxed Environments

Take a tree of DOM elements created in a sandboxed JavaScript environment (iframe, web worker) and render them to the DOM in a different environment.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/remote-dom/`

## Architecture

```
Remote Environment (iframe/worker)    Message Channel    Host Environment (main thread)
───────────────────────────────       ──────────────     ──────────────────────────────
Remote DOM Tree                       postMessage/       DOMRemoteReceiver
RemoteConnection ───────────────────▶ MessagePort ────▶ Creates real DOM elements
```

**Aha:** Remote DOM doesn't serialize the entire DOM tree on each update. It serializes individual DOM *operations* (createElement, appendChild, setAttribute) and replays them on the host side. This makes updates incremental and efficient.

## 6 Packages

| Package | Purpose | Files |
|---------|---------|-------|
| `@remote-dom/core` | Core connection and receiver types | 7 |
| `@remote-dom/react` | React bindings for remote rendering | 6 |
| `@remote-dom/preact` | Preact bindings (lightweight) | 5 |
| `@remote-dom/signals` | Signal-based reactive receiver | 2 |
| `@remote-dom/compat` | Compatibility layer | 1 |
| `@remote-dom/polyfill` | Minimal DOM API polyfill for workers | 30+ |

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

The polyfill provides minimal DOM APIs so libraries like Preact and Svelte can run inside a web worker:

| API | File |
|-----|------|
| `Attr` | `Attr.ts` |
| `CharacterData` | `CharacterData.ts` |
| `Comment` | `Comment.ts` |
| `CustomElementRegistry` | `CustomElementRegistry.ts` |
| `Document` | `Document.ts` |
| `Element` | `Element.ts` |
| `Text` | `Text.ts` |
| `MutationObserver` | `MutationObserver.ts` |
