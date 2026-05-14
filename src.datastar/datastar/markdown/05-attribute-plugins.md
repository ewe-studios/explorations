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

## 2. bind — Two-way data binding (Full Source Walkthrough)

The most complex plugin (301 lines). Creates bidirectional sync between form element values and signals.

Source: `plugins/attributes/bind.ts` — 301 lines

### Adapter Pattern (lines 24-52)

Three adapter factory functions handle common cases:

```typescript
// propAdapter — reads/writes a DOM property directly
const propAdapter = (prop: string, ...events: string[]): BindAdapter => ({
  get: (el: any) => el[prop],
  set: (el: any, value: any) => { el[prop] = value },
  events,
})

// attrAdapter — reads/writes a DOM attribute
const attrAdapter = (attr: string, ...events: string[]): BindAdapter => ({
  get: (el: Element) => el.getAttribute(attr),
  set: (el: Element, value: any) => { el.setAttribute(attr, `${value}`) },
  events,
})

// valueAdapter — reads/writes the .value property with type coercion
const valueAdapter = (treatUndefinedAsString = false, ...events: string[]): BindAdapter => ({
  get: (el, type: string) =>
    type === 'string' || (treatUndefinedAsString && type === 'undefined')
      ? el.value : +el.value,
  set: (el, value: string | number) => { el.value = `${value}` },
  events,
})
```

**Aha:** The type-aware `valueAdapter` checks the type of the current signal value to decide whether to return a string or number. If the signal is `string` type, it returns `el.value` as-is. Otherwise it coerces with `+el.value`. This means `data-bind:count` auto-detects whether the signal should be numeric.

### boundPath — Signal Path Resolution (lines 57-114)

```typescript
const boundPath = (el, key, rawKey, signalName, adapter, initialValue) => {
  const rawAttribute = aliasify(CSS.escape(rawKey))
  const selector = key
    ? `[${rawAttribute}]`
    : `[${rawAttribute}="${CSS.escape(signalName)}"]`
```

**What:** Builds a CSS selector to find all elements with the same bind attribute. This is used for radio group synchronization — all radios in a group share the same signal.

Radio group special case (lines 69-82):
```typescript
if (initialValue === undefined && el instanceof HTMLInputElement && el.type === 'radio') {
  const checked = [...document.querySelectorAll(selector)].find(
    (input): input is HTMLInputElement => input.checked,
  )
  if (checked) {
    mergePaths([[signalName, checked.value]], { ifMissing: true })
  }
}
```

If the signal doesn't exist yet and there's a checked radio in the group, adopt that radio's value as the signal's initial value. This means the HTML can define the default, not the JS.

Array binding (lines 84-113):
```typescript
if (!Array.isArray(initialValue) || (el instanceof HTMLSelectElement && el.multiple)) {
  mergePaths([[signalName, adapter.get(el, typeof initialValue)]], { ifMissing: true })
  return signalName
}
// For arrays: find position of this element in the group
const inputs = document.querySelectorAll(selector)
let i = 0
for (const input of inputs) {
  paths.push([`${signalName}.${i}`, adapter.get(input, typeof initialValue[i])])
  if (el === input) break
  i++
}
mergePaths(paths, { ifMissing: true })
return `${signalName}.${i}`
```

### Per-Element-Type Dispatch (lines 119-251)

The `apply` function dispatches to different adapters based on element type:

```typescript
apply({ el, key, rawKey, mods, value, error }) {
  const signalName = key != null ? modifyCasing(key, mods) : value

  // Determine adapter based on element type
  if (el instanceof HTMLInputElement) {
    switch (el.type) {
      case 'range': case 'number':
        adapter = valueAdapter(false, 'input')
        break
      case 'checkbox':
        adapter = {
          get: (el, type) => {
            if (el.value !== 'on') {
              return type === 'boolean' ? el.checked : (el.checked ? el.value : '')
            }
            return type === 'string' ? (el.checked ? el.value : '') : el.checked
          },
          set: (el, value) => {
            el.checked = typeof value === 'string' ? value === el.value : value
          },
          events: ['input'],
        }
        break
      case 'radio':
        if (!el.getAttribute('name')?.length) {
          el.setAttribute('name', signalName)  // Auto-assign name
        }
        adapter = {
          get: (el, type) => el.checked ? (type === 'number' ? +el.value : el.value) : empty,
          set: (el, value) => {
            el.checked = value === (typeof value === 'number' ? +el.value : el.value)
          },
          events: ['input'],
        }
        break
      case 'file':
        // FileReader-based upload handler (see below)
        break
      default:
        adapter = valueAdapter(true, 'input')
    }
  } else if (el instanceof HTMLSelectElement && el.multiple) {
    // Multi-select adapter with type tracking
  } else if (el instanceof HTMLSelectElement) {
    adapter = valueAdapter(true, 'change')
  } else if (el instanceof HTMLTextAreaElement) {
    adapter = propAdapter('value', 'input')
  } else if (el instanceof HTMLElement && el.tagName.includes('-')) {
    // Custom elements: try .value property, fall back to attribute
    adapter = 'value' in el ? propAdapter('value', 'input', 'change') : attrAdapter('value', 'input', 'change')
  }
```

### File Upload Adapter (lines 169-208)

The file input is special — it reads files via `FileReader` and converts to base64:

```typescript
case 'file': {
  const syncSignal = () => {
    const files = [...(el.files || [])]
    const signalFiles: SignalFile[] = []
    Promise.all(files.map(f => new Promise<void>((resolve) => {
      const reader = new FileReader()
      reader.onload = () => {
        const match = reader.result.match(dataURIRegex)
        signalFiles.push({
          name: f.name,
          contents: match.groups.contents,  // base64 content
          mime: match.groups.mime,
        })
      }
      reader.onloadend = () => resolve()
      reader.readAsDataURL(f)
    }))).then(() => {
      mergePaths([[signalName, signalFiles]])
    })
  }
  el.addEventListener('change', syncSignal)
  return () => { el.removeEventListener('change', syncSignal) }
}
```

**What:** Each file is read as a Data URI, parsed with regex to extract mime and base64 content, and stored as `{ name, contents, mime }` objects in the signal.

### Signal Sync and Effect (lines 275-300)

```typescript
const syncSignal = () => {
  const signalValue = getPath(path)
  if (signalValue != null) {
    const value = adapter.get(el, typeof signalValue)
    if (value !== empty) {
      mergePaths([[path, value]])
    }
  }
}

// DOM → Signal: listen for input/change events
for (const eventName of adapter.events) {
  el.addEventListener(eventName, syncSignal)
}
el.addEventListener(DATASTAR_PROP_CHANGE_EVENT, syncSignal)

// Signal → DOM: effect watches signal and updates element
const cleanup = effect(() => {
  adapter.set(el, getPath(path))
})

return () => {
  cleanup()
  for (const eventName of adapter.events) {
    el.removeEventListener(eventName, syncSignal)
  }
  el.removeEventListener(DATASTAR_PROP_CHANGE_EVENT, syncSignal)
}
```

**Aha:** The bind plugin uses `DATASTAR_PROP_CHANGE_EVENT` — a custom event dispatched by the morphing system when attributes change during a morph. This means if a morph updates an input's value, the bind plugin catches it and syncs the signal, preventing stale state after DOM patches.

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

## 9. on — Event listener (Full Source Walkthrough)

Source: `plugins/attributes/on.ts` — 70 lines

```typescript
attribute({
  name: 'on',
  requirement: 'must',  // Key (event name) is required
  argNames: ['evt'],    // Pass the event object to the compiled expression
  apply({ el, key, mods, rx }) {
```

### Target Resolution (lines 20-25)

```typescript
let target: Element | Window | Document = el
if (mods.has('window')) {
  target = window
} else if (mods.has('document')) {
  target = document
}
```

Default: listen on the element itself. `.window` modifier → `window`, `.document` → `document`.

### Callback with Batching (lines 26-30)

```typescript
let callback = (evt?: Event) => {
  beginBatch()
  rx(evt)
  endBatch()
}
```

Every event handler runs inside a batch — any signal changes are coalesced into a single propagation cycle.

### Modifier Pipeline (lines 31-33)

```typescript
callback = modifyViewTransition(callback, mods)
callback = modifyTiming(callback, mods)
const eventName = modifyCasing(key, mods, 'kebab')
```

Three modifier transformations applied in order:
1. **View transitions:** If `.viewtransition` modifier, wrap in `document.startViewTransition()`
2. **Timing:** If `.delay`, `.debounce`, or `.throttle` modifiers, wrap in timing wrappers
3. **Event name casing:** Default is kebab-case (`click`), but `.camel` or `.pascal` modifiers change it

### Event Listener Options (lines 34-38)

```typescript
const evtListOpts: AddEventListenerOptions = {
  capture: mods.has('capture'),
  passive: mods.has('passive'),
  once: mods.has('once'),
}
```

### Outside Click Handler (lines 39-47)

```typescript
if (mods.has('outside')) {
  target = document
  const cb = callback
  callback = (evt?: Event) => {
    if (!el.contains(evt?.target as HTMLElement)) {
      cb(evt)
    }
  }
}
```

The `.outside` modifier redirects listening to `document` and filters out clicks that originate inside the element. This is the standard "click outside to close dropdown" pattern.

### Datastar Event Override (lines 49-54)

```typescript
if (eventName === DATASTAR_FETCH_EVENT || eventName === DATASTAR_SIGNAL_PATCH_EVENT) {
  target = document
}
```

Custom Datastar events (`datastar-fetch`, `datastar-signal-patch`) are always dispatched on `document`, so the listener must be on `document` to receive them.

### Event Listener with Side Effects (lines 56-68)

```typescript
const listener = (evt?: Event) => {
  if (evt) {
    if (mods.has('prevent')) evt.preventDefault()
    if (mods.has('stop')) evt.stopPropagation()
    if (el instanceof HTMLFormElement && eventName === 'submit') evt.preventDefault()
  }
  callback(evt)
}
target.addEventListener(eventName, listener, evtListOpts)
return () => { target.removeEventListener(eventName, listener, evtListOpts) }
```

**Aha:** Form submission is automatically prevented when `data-on:submit` is present. The check `el instanceof HTMLFormElement && eventName === 'submit'` calls `evt.preventDefault()` unconditionally. This means you never need `.prevent` on submit handlers — it's baked in. This is a deliberate design decision to prevent the common footgun of forgetting `.prevent` and getting a full page reload.

## 1. attr — Generic attribute binding (Full Source Walkthrough)

Source: `plugins/attributes/attr.ts` — 61 lines

```typescript
attribute({
  name: 'attr',
  requirement: { value: 'must' },
  returnsValue: true,
  apply({ el, key, rx }) {
```

### Value Serialization (lines 13-30)

```typescript
const syncAttr = (key: string, val: any) => {
  if (val === '' || val === true) {
    el.setAttribute(key, '')           // Boolean attributes: empty string
  } else if (val === false || val == null) {
    el.removeAttribute(key)            // false/null/undefined → remove
  } else if (typeof val === 'string') {
    el.setAttribute(key, val)          // String → set directly
  } else if (typeof val === 'function') {
    el.setAttribute(key, val.toString()) // Function → toString()
  } else {
    el.setAttribute(key, JSON.stringify(val, (_k, v) =>
      typeof v === 'function' ? v.toString() : v))
  }
}
```

**What:** Smart serialization — booleans map to HTML boolean attribute semantics, functions are stringified, objects are JSON-stringified (with nested functions also stringified).

### Two Modes: Single Key vs Object (lines 32-51)

```typescript
const update = key
  ? () => {
      // Single attribute mode: data-attr:placeholder="$hint"
      observer.disconnect()
      syncAttr(key, rx())
      observer.observe(el, { attributeFilter: [key] })
    }
  : () => {
      // Object mode: data-attr="{ placeholder: $hint, disabled: $isDisabled }"
      observer.disconnect()
      const obj = rx() as Record<string, any>
      for (const key of Object.keys(obj)) { syncAttr(key, obj[key]) }
      observer.observe(el, { attributeFilter: Object.keys(obj) })
    }
```

In single-key mode, only the specified attribute is observed. In object mode, all keys from the returned object are observed. The MutationObserver prevents external changes from being overwritten.

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
