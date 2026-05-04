# Datastar -- Attribute Plugins

Datastar ships with 17 attribute plugins, each implementing a specific reactive behavior. All follow the same pattern: `apply()` receives context, sets up reactive effects, returns cleanup.

Source: `library/src/plugins/attributes/*.ts`

## Common Pattern

Every attribute plugin uses the `attribute()` registration function:

```typescript
attribute({
  name: 'plugin-name',
  requirement: { value: 'must' },  // or 'exclusive', or { key: 'denied', value: 'must' }
  returnsValue: true,               // whether the compiled rx() returns a value
  apply({ el, key, rawKey, mods, rx, error }) {
    // Setup reactive behavior
    return () => { /* cleanup */ }
  },
})
```

## 1. attr — Generic attribute binding

Syncs any HTML attribute to a signal expression.

```html
<!-- Single attribute: data-attr:placeholder="$hint" -->
<!-- Object syntax: data-attr="{ placeholder: $hint, disabled: $isDisabled }" -->
```

```typescript
// plugins/attributes/attr.ts
attribute({
  name: 'attr',
  requirement: { value: 'must' },
  returnsValue: true,
  apply({ el, key, rx }) {
    const syncAttr = (key: string, val: any) => {
      if (val === '' || val === true) el.setAttribute(key, '')
      else if (val === false || val == null) el.removeAttribute(key)
      else if (typeof val === 'string') el.setAttribute(key, val)
      else el.setAttribute(key, JSON.stringify(val))
    }

    const update = key
      ? () => { observer.disconnect(); syncAttr(key, rx()); observer.observe(el, { attributeFilter: [key] }) }
      : () => { observer.disconnect(); const obj = rx(); for (const k of Object.keys(obj)) syncAttr(k, obj[k]); observer.observe(el, { attributeFilter: Object.keys(obj) }) }

    const observer = new MutationObserver(update)
    const cleanup = effect(update)
    return () => { observer.disconnect(); cleanup() }
  },
})
```

The MutationObserver prevents external changes (e.g., from browser extensions) from being overwritten by the effect.

## 2. bind — Two-way data binding

The most complex plugin. Creates bidirectional sync between a form element's value and a signal.

```html
<!-- Text input: data-bind:title -->
<!-- Checkbox: data-bind:kebab="$isChecked" -->
<!-- Radio group: data-bind="$selectedRadio" (auto-creates name attribute) -->
<!-- File upload: data-bind="$files" → { name, contents, mime }[] -->
<!-- Select multiple: data-bind="$selectedOptions" -->
<!-- Custom property: data-bind.prop:value="$customValue" -->
```

**Aha:** The bind plugin has per-element-type adapters. An `<input type="checkbox">` gets a different adapter than `<input type="radio">`, which gets a different adapter than `<select multiple>`. The adapter pattern means each element type handles its native value semantics correctly — checkboxes use `checked` boolean, radios use `value` string, selects use `selectedOptions` NodeList.

Key adapter behaviors:

| Element | Adapter | Events | Notes |
|---------|---------|--------|-------|
| `input[type=range/number]` | valueAdapter(false) | `input` | Parses numeric values |
| `input[type=checkbox]` | Custom | `input` | Boolean or value-based |
| `input[type=radio]` | Custom | `input` | Auto-sets `name` attribute |
| `input[type=file]` | FileReader | `change` | Converts to `{name, contents, mime}[]` |
| `select[multiple]` | Custom | `change` | Array of selected values |
| `select` | valueAdapter(true) | `change` | String value |
| `textarea` | propAdapter('value') | `input` | Text content |
| Custom element (`*-tag`) | prop/attr | `input, change` | `.value` property or attribute |

## 3. class — Conditional CSS classes

```html
<!-- Single: data-class:active="$isActive" -->
<!-- Multiple: data-class="{ active: $isActive, hidden: $isHidden }" -->
<!-- Casing: data-class:kebab="$someClass" → applies as kebab-case -->
```

Splits multi-word class keys on whitespace: `data-class:"font-bold text-red"` adds both classes.

## 4. computed — Derived signal creation

Creates a computed signal from an expression:

```html
<!-- Single: data-computed:fullName="$firstName + ' ' + $lastName" -->
<!-- Object: data-computed="{ fullName: $firstName + ' ' + $lastName, age: 2024 - $birthYear }" -->
```

```typescript
// plugins/attributes/computed.ts
if (key) {
  mergePaths([[modifyCasing(key, mods), computed(rx)]])
} else {
  const patch = rx() as Record<string, () => any>
  // Each value in the object must be a function
  mergePatch(patch)
}
```

## 5. effect — Side-effect execution

Runs an expression on load and whenever signals change:

```html
<!-- data-effect="$console.log($count)" -->
```

The simplest plugin — just wraps `effect(rx)`. No key allowed.

## 6. indicator — SSE request loading state

Sets a signal to `true` while an SSE fetch is in flight for this element:

```html
<!-- data-indicator:loading -->
<!-- Shows "Loading..." while a data-on:click="@get('/api/data')" is pending -->
```

```typescript
// plugins/attributes/indicator.ts
let activeFetches = 0
document.addEventListener('datastar-fetch', (event) => {
  if (event.detail.el !== el) return
  if (event.detail.type === 'started') { activeFetches++; mergePaths([[signalName, true]]) }
  if (event.detail.type === 'finished') { activeFetches = Math.max(0, activeFetches - 1); mergePaths([[signalName, activeFetches > 0]]) }
})
```

**Aha:** The `activeFetches` counter handles concurrent requests. If two fetches start before either finishes, the indicator stays `true` until both complete. Without this counter, the first `finished` event would set the indicator to `false` even while the second fetch is still running.

## 7. init — Run expression on mount

Runs an expression once when the element enters the DOM:

```html
<!-- data-init="$count = 0" -->
<!-- data-init.delay:500ms="$fetchData()" -->
<!-- data-init.viewtransition="$animateIn()" -->
```

Wraps the expression in `beginBatch()` / `endBatch()` to batch any signal updates.

## 8. json-signals — Reactive JSON output

Outputs filtered signals as JSON text content:

```html
<!-- data-json-signals (all signals) -->
<!-- data-json-signals="{ include: /^(?!_)/ }" (exclude underscore-prefixed) -->
<!-- data-json-signals.terse (compact, no indentation) -->
```

Uses `effect()` to re-render whenever any filtered signal changes. Also uses a MutationObserver to prevent external text changes from being overwritten.

## 9. on — Event listener

The most versatile plugin. Attaches event listeners with modifiers:

```html
<!-- data-on:click="$count++" -->
<!-- data-on:submit.prevent="$submit()" -->
<!-- data-on:keyup.escape.window="$closeModal()" -->
<!-- data-on:click.delay:200ms.debounce="$search($event.target.value)" -->
<!-- data-on:click.outside="$closeDropdown()" -->
<!-- data-on:scroll.passive.capture.throttle:100ms="$onScroll($event)" -->
<!-- data-on:keydown.ctrl.k.prevent="$openCommandPalette()" -->
```

Modifiers:

| Modifier | What it does |
|----------|-------------|
| `.prevent` | `event.preventDefault()` |
| `.stop` | `event.stopPropagation()` |
| `.once` | `{ once: true }` in addEventListener |
| `.capture` | `{ capture: true }` |
| `.passive` | `{ passive: true }` |
| `.window` | Listen on `window` instead of element |
| `.document` | Listen on `document` instead of element |
| `.outside` | Only fire when click target is NOT inside element |
| `.delay:Nms` | `setTimeout` before executing |
| `.debounce:Nms` | Debounce (with `.leading`, `.notrailing`) |
| `.throttle:Nms` | Throttle (with `.noleading`, `.trailing`) |
| `.viewtransition` | Wrap in `document.startViewTransition()` |

**Aha:** The on plugin automatically prevents form submission when `data-on:submit` is present. This means you don't need `.prevent` on submit handlers — it's built into the plugin's event listener wrapper.

## 10. on-intersect — IntersectionObserver trigger

Runs an expression when the element enters the viewport:

```html
<!-- data-on-intersect="$loadMore()" -->
<!-- data-on-intersect.once="$trackPageView()" -->
<!-- data-on-intersect.exit="$onLeave()" (trigger when leaving instead of entering) -->
<!-- data-on-intersect.full (require 100% visibility) -->
<!-- data-on-intersect.half (require 50% visibility) -->
<!-- data-on-intersect.threshold:75 (require 75% visibility) -->
<!-- data-on-intersect.delay:100ms (debounce) -->
```

Uses a `WeakSet` to track elements that had `.once`, so the observer disconnects after the first intersection.

## 11. on-interval — Timer-based execution

```html
<!-- data-on-interval="$refresh()" (every 1000ms default) -->
<!-- data-on-interval.duration:5000ms="$poll()" -->
<!-- data-on-interval.duration:30s.leading="$tick()" (execute immediately on start) -->
```

## 12. on-signal-patch — React to signal changes

Listens for `datastar-signal-patch` events (broadcast by `mergePatch`):

```html
<!-- data-on-signal-patch="$onPatch($patch)" -->
<!-- data-on-signal-patch:filter="{ include: /^user/ }" (only user signals) -->
```

```typescript
// plugins/attributes/onSignalPatch.ts
const callback = (evt: CustomEvent<JSONPatch>) => {
  if (running) return  // Prevent re-entrant patches
  const watched = filtered(filters, evt.detail)
  if (!isEmpty(watched)) {
    running = true
    beginBatch()
    rx(watched)
    endBatch()
    running = false
  }
}
```

**Aha:** The `running` flag prevents infinite loops. If the expression itself patches signals, this triggers another `datastar-signal-patch` event, which would re-invoke the handler. The `running` flag breaks the cycle by skipping re-entrant calls.

## 13. ref — DOM element reference

Creates a signal holding a reference to the DOM element:

```html
<!-- data-ref:canvas (creates signal "canvas" = <canvas>) -->
<!-- data-ref="$myElement" -->
```

```typescript
// plugins/attributes/ref.ts
mergePaths([[signalName, el]])
```

## 14. show — Conditional visibility

```html
<!-- data-show="$count > 0" -->
```

Sets `display: none` when false, removes it when true. Uses a MutationObserver to detect external style changes.

## 15. signals — Initialize or patch signals

```html
<!-- data-signals:kebab:ifmissing="{ count: 0, message: 'hello' }" -->
<!-- data-signals:camel="$user" (single signal) -->
```

With `.ifmissing`, only creates signals that don't already exist — useful for default values.

## 16. style — Inline CSS binding

```html
<!-- Single: data-style:color="$themeColor" -->
<!-- Object: data-style="{ color: $red, 'font-size': $size + 'px' }" -->
```

Saves initial styles in a `Map` so they can be restored when the plugin is cleaned up or the value becomes falsy.

## 17. text — Text content binding

```html
<!-- data-text="$message" -->
```

The simplest content plugin. Sets `el.textContent` to the expression result. Uses a MutationObserver to handle external text changes.

## Plugin Summary Table

| Plugin | Requirement | Returns Value | Key Support | Primary Use |
|--------|-------------|--------------|-------------|-------------|
| attr | value: must | Yes | Optional | Generic attribute binding |
| bind | exclusive | No | Optional | Two-way data binding |
| class | value: must | Yes | Optional | Conditional classes |
| computed | value: must | Yes | Optional | Create derived signals |
| effect | key: denied, value: must | No | Denied | Side effects |
| indicator | exclusive | No | Optional | Loading indicator |
| init | key: denied, value: must | No | Denied | Run on mount |
| json-signals | key: denied | No | Denied | JSON output |
| on | must | No | Required | Event listeners |
| on-intersect | key: denied, value: must | No | Denied | IntersectionObserver |
| on-interval | key: denied, value: must | No | Denied | setInterval |
| on-signal-patch | value: must | Yes | "filter" only | React to patches |
| ref | exclusive | No | Optional | DOM element ref |
| show | key: denied, value: must | Yes | Denied | Visibility toggle |
| signals | none | Yes | Optional | Initialize signals |
| style | value: must | Yes | Optional | Inline CSS |
| text | key: denied, value: must | Yes | Denied | Text content |

See [Plugin System](04-plugin-system.md) for how plugins register.
See [Action Plugins](06-action-plugins.md) for the 4 action plugins.
See [Expression Compiler](03-expression-compiler.md) for how expressions are compiled.
