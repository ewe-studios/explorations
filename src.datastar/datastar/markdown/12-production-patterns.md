# Datastar -- Production Patterns

Running Datastar in production introduces concerns beyond the core library: state management at scale, security, performance, and error handling.

## Signal Store Organization

In a complex application, the global signal store can grow large. Organizing signals with namespaces prevents collisions:

```typescript
// Convention: prefix signals by feature
data-signals:camel="{ user: { name: '', email: '' }, ui: { modal: false, sidebar: true } }"

// Access: $user.name, $ui.modal
```

**Underscore convention:** Internal signals start with `_` and are excluded from fetch payloads:

```typescript
// fetch plugin default filter
filterSignals: { include: /.*/, exclude: /(^|\\.)_/ }
```

This means `$user.name` is sent to the server but `$_loading` is not.

## Memory Management

Each plugin that registers on an element returns a cleanup function. The engine calls these when elements are removed:

```typescript
// From bind plugin:
return () => {
  cleanup()  // effect cleanup
  for (const eventName of adapter.events) {
    el.removeEventListener(eventName, syncSignal)
  }
  el.removeEventListener(DATASTAR_PROP_CHANGE_EVENT, syncSignal)
}
```

**Aha:** Without proper cleanup, every dynamic element that appears and disappears (e.g., via `data-show`) leaks event listeners and effect subscriptions. Datastar's cleanup pattern ensures that `effect()` returned cleanup functions and `removeEventListener` calls are both executed on element removal.

## Error Handling

The engine provides an error factory to plugins:

```typescript
apply({ el, error }) {
  if (!url?.length) {
    throw error('FetchNoUrlProvided', { action })
  }
}
```

Errors are typed and include context, making them debuggable in production.

## Performance Considerations

### Expression Caching

genRx caches compiled expressions by string:

```typescript
const rxCache = new Map<string, Function>()
```

Two elements with `data-text="$count"` share the same compiled function. The cache prevents recompilation when the same expression appears in multiple places.

### MutationObserver Efficiency

Each reactive attribute plugin creates its own MutationObserver to detect external changes. For pages with many reactive elements, this creates many observers:

```html
<!-- 5 elements × 3 observers each = 15 MutationObservers -->
<div data-bind:name></div>
<div data-bind:email></div>
<div data-show="$isValid"></div>
<div data-class:error="$hasError"></div>
<div data-text="$message"></div>
```

A production optimization would be to use a single shared MutationObserver with a dispatch table, rather than one per plugin instance.

### Batch Coalescing

Multiple signal updates within `beginBatch()` / `endBatch()` coalesce into a single propagation pass:

```html
<!-- Without batching: 3 separate effect fires -->
<div data-init="$count = 0; $message = 'Hello'; $loading = false"></div>

<!-- With batching: 1 combined effect fire -->
<div data-init="beginBatch(); $count = 0; $message = 'Hello'; $loading = false; endBatch()"></div>
```

Note: `beginBatch()`/`endBatch()` must be called explicitly in expressions — the engine doesn't auto-batch across attribute plugins.

## Security Considerations

### Expression Sandbox

genRx-compiled Functions execute in the global JavaScript scope with access to `$` (signal store), `$el` (element), `$evt` (event), and `$plugins` (plugin registry). They do NOT have access to arbitrary global variables unless those are in the enclosing scope.

**Risk:** If user-generated content can influence expression strings, it becomes XSS:

```html
<!-- DANGEROUS: if $userInput contains "); alert(1); //" -->
<div data-text="$userInput"></div>
```

Datastar's expressions are compiled at attribute-parse time, not at runtime, so user input in signal VALUES is safe (it's just data), but user input in attribute VALUES is not.

### Form Validation

The fetch plugin validates forms before submission:

```typescript
if (!formEl.noValidate && !formEl.checkValidity()) {
  formEl.reportValidity()
  return  // Don't submit invalid forms
}
```

This uses the browser's native constraint validation API.

### Content-Type Validation

The SSE parser validates Content-Type before processing:

```typescript
if (ct?.includes('text/event-stream')) { /* parse SSE */ }
else if (ct?.includes('text/html')) { /* patch elements */ }
else if (ct?.includes('application/json')) { /* patch signals */ }
else if (ct?.includes('text/javascript')) { /* execute script */ }
```

The `text/javascript` path is the most dangerous — it creates and executes a `<script>` element from server response content. Ensure your server is trusted before using this path.

## View Transitions API

```html
<!-- data-init.viewtransition="$animateMount()" -->
<!-- data-on:click.viewtransition="$navigate()" -->
```

The View Transitions API is only supported in Chromium browsers. The `supportsViewTransitions` flag gracefully degrades:

```typescript
if (supportsViewTransitions && useViewTransition) {
  document.startViewTransition(() => morph())
} else {
  morph()  // Instant, no animation
}
```

## Debugging

The engine dispatches a `datastar-ready` event when initialization is complete:

```typescript
document.addEventListener('datastar-ready', () => {
  console.log('Datastar initialized')
})
```

Custom events for observability:

| Event | When | Detail |
|-------|------|--------|
| `datastar-ready` | Engine init complete | None |
| `datastar-fetch` | Fetch lifecycle | `{ type, el, argsRaw }` |
| `datastar-signal-patch` | Signals updated | JSONPatch |
| `datastar-prop-change` | Form property changed | None |
| `datastar-scope-children` | Morph scoped children | None |

See [Rust Equivalents](11-rust-equivalents.md) for production Rust patterns.
See [Web Tooling](13-web-tooling.md) for IDE support.
