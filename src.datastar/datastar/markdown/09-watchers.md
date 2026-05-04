# Datastar -- Watchers

Watcher plugins listen for custom events dispatched on the `document`. Two watchers ship with Datastar: `datastar-patch-elements` and `datastar-patch-signals`.

Source: `library/src/plugins/watchers/patchElements.ts`, `library/src/plugins/watchers/patchSignals.ts`

## How Watchers Work

Watcher plugins register themselves with a name and an `apply` function:

```typescript
// engine/engine.ts
export const watcher = (plugin: WatcherPlugin) => watcherPlugins.set(plugin.name, plugin)
```

When code dispatches an event matching the watcher's name, the engine finds the watcher and calls `apply()`:

```typescript
// Somewhere in the engine:
document.addEventListener(watcherName, (event) => {
  watcherPlugins.get(watcherName)?.apply({ error }, event.detail)
})
```

## 1. datastar-patch-elements

The DOM patching watcher. Receives HTML content and patches it into the DOM.

```typescript
// plugins/watchers/patchElements.ts
watcher({
  name: 'datastar-patch-elements',
  apply(ctx, args) {
    const selector = typeof args.selector === 'string' ? args.selector : ''
    const mode = typeof args.mode === 'string' ? args.mode : 'outer'
    const namespace = typeof args.namespace === 'string' ? args.namespace : 'html'
    const useViewTransition = args.useViewTransition?.trim() === 'true'
    const elements = args.elements

    if (supportsViewTransitions && useViewTransition) {
      document.startViewTransition(() => onPatchElements(ctx, args))
    } else {
      onPatchElements(ctx, args)
    }
  },
})
```

This watcher receives:

| Argument | Type | Purpose |
|----------|------|---------|
| `selector` | `string` | CSS selector for target elements |
| `mode` | `string` | One of 8 patch modes (see [DOM Morphing](07-dom-morphing.md)) |
| `namespace` | `string` | `html`, `svg`, or `mathml` |
| `useViewTransition` | `string` | `"true"` to use View Transitions API |
| `elements` | `string \| Element \| DocumentFragment` | HTML content to patch |

The watcher validates the mode and namespace, then calls `onPatchElements` which parses the HTML and runs the morph algorithm.

## 2. datastar-patch-signals

The signal patching watcher. Receives JSON and merges it into the global signal store.

```typescript
// plugins/watchers/patchSignals.ts
watcher({
  name: 'datastar-patch-signals',
  apply({ error }, { signals, onlyIfMissing }) {
    if (typeof signals !== 'string') {
      throw error('PatchSignalsExpectedSignals')
    }
    const ifMissing = typeof onlyIfMissing === 'string' && onlyIfMissing.trim() === 'true'
    mergePatch(jsStrToObject(signals), { ifMissing })
  },
})
```

Receives:

| Argument | Type | Purpose |
|----------|------|---------|
| `signals` | `string` | JSON string of signal values |
| `onlyIfMissing` | `string` | `"true"` to only create non-existing signals |

## Event Dispatch Sources

Both watchers are triggered by events dispatched via the fetch plugin's SSE handling:

```typescript
// From SSE event handler:
dispatchFetch('datastar-patch-elements', el, {
  selector: '#main',
  mode: 'inner',
  elements: '<p>Hello</p>',
})

// From non-SSE JSON response:
dispatchFetch('datastar-patch-signals', el, {
  signals: '{"count": 42, "message": "Hello"}',
})
```

## Watcher Registration in Engine

The engine listens for watcher events in a centralized dispatch:

```typescript
// engine/engine.ts — simplified
for (const [name, watcher] of watcherPlugins) {
  document.addEventListener(name, (event) => {
    try {
      watcher.apply({ error: createErrorFactory(watcher.name) }, event.detail)
    } catch (e) {
      console.error(`Watcher ${name} error:`, e)
    }
  })
}
```

## Comparison: Watchers vs Attribute Plugins

| Aspect | Watcher Plugin | Attribute Plugin |
|--------|---------------|-----------------|
| Trigger | Custom event on document | Attribute on DOM element |
| Scope | Global (document-level) | Element-local |
| Lifecycle | One-shot per event | Long-lived (effect + cleanup) |
| Cleanup | None needed | Returns cleanup function |
| Examples | patchElements, patchSignals | bind, on, show, class, style |

Watchers are for server-pushed changes. Attribute plugins are for element-bound reactivity.

See [DOM Morphing](07-dom-morphing.md) for the morph algorithm used by patchElements.
See [SSE Streaming](08-sse-streaming.md) for how SSE events trigger watchers.
