# Datastar -- Utility Systems

Eight utility modules support the engine and plugins with common operations: case conversion, timing, path manipulation, math, and DOM helpers.

Source: `library/src/utils/*.ts`

## text.ts — Case Conversion and String Utilities

```typescript
// Case conversion functions
export const kebab = (str: string): string =>  // camelCase → camel-case
export const camel = (str: string): string =>  // camel-case → camelCase
export const snake = (str: string): string =>  // camel-case → camel_case
export const pascal = (str: string): string => // camel-case → CamelCase
export const title = (str: string): string =>  // camel case → Camel Case
```

### kebab

Converts any string to kebab-case via a chain of regex replacements:

```typescript
export const kebab = (str) =>
  str
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1-$2')  // "HTMLElement" → "HTML-Element"
    .replace(/([a-z0-9])([A-Z])/g, '$1-$2')     // "camelCase" → "camel-Case"
    .replace(/([a-z])([0-9]+)/gi, '$1-$2')      // "item123" → "item-123"
    .replace(/([0-9]+)([a-z])/gi, '$1-$2')      // "123item" → "123-item"
    .replace(/[\s_]+/g, '-')                     // "hello world" → "hello-world"
    .toLowerCase()
```

### jsStrToObject — JSON with Function Revival

```typescript
export const jsStrToObject = (raw: string, { reviveFunctionStrings = false } = {}) => {
  if (!reviveFunctionStrings) return JSON.parse(raw)
  return JSON.parse(raw, (_k, value) => {
    if (typeof value !== 'string') return value
    const trimmed = value.trim()
    if (!RE_FUNCTION_LITERAL.test(trimmed)) return value
    try {
      const revived = Function(`return (${trimmed})`)()
      return typeof revived === 'function' ? revived : value
    } catch { return value }
  })
}
```

Used by attribute plugins to parse JSON values from HTML attributes. When `reviveFunctionStrings` is true, string values that look like function expressions are evaluated into actual functions.

### modifyCasing

Applies case transformation based on modifiers:

```typescript
export const modifyCasing = (str, mods, defaultCase = 'camel') => {
  for (const c of mods.get('case') || [defaultCase]) {
    str = caseFns[c]?.(str) || str
  }
  return str
}
```

Used by `data-bind:kebab` to convert `someSignalName` to `some-signal-name`.

### aliasify / unaliasify

Adds/removes a custom prefix from attribute names:

```typescript
export const aliasify = (name) => ALIAS ? `data-${ALIAS}-${name}` : `data-${name}`
export const unaliasify = (name) => {
  if (!ALIAS) return name
  if (!name.startsWith(`${ALIAS}-`)) return null
  return name.slice(ALIAS.length + 1)
}
```

When `ALIAS` is set at build time (e.g., `"myapp"`), all attributes become `data-myapp-*` instead of `data-*`.

## timing.ts — Delay, Throttle, Debounce

```typescript
export const delay = (callback, wait) =>
  (...args) => setTimeout(() => callback(...args), wait)

export const throttle = (callback, wait, leading = true, trailing = false, debounce = false) =>
  // Combined throttle + debounce implementation
```

### throttle

A unified throttle/debounce function:

| Mode | leading | trailing | debounce |
|------|---------|----------|----------|
| Throttle (leading) | `true` | `false` | `false` |
| Throttle (trailing) | `false` | `true` | `false` |
| Debounce | `false` | `true` | `true` |

### modifyTiming

Reads timing modifiers from the modifier set and applies them:

```typescript
export const modifyTiming = (callback, mods) => {
  const delayArgs = mods.get('delay')
  if (delayArgs) callback = delay(callback, tagToMs(delayArgs))

  const debounceArgs = mods.get('debounce')
  if (debounceArgs) {
    const wait = tagToMs(debounceArgs)
    const leading = tagHas(debounceArgs, 'leading', false)
    const trailing = !tagHas(debounceArgs, 'notrailing', false)
    callback = throttle(callback, wait, leading, trailing, true)
  }

  const throttleArgs = mods.get('throttle')
  if (throttleArgs) {
    const wait = tagToMs(throttleArgs)
    const leading = !tagHas(throttleArgs, 'noleading', false)
    const trailing = tagHas(throttleArgs, 'trailing', false)
    callback = throttle(callback, wait, leading, trailing)
  }

  return callback
}
```

## tags.ts — Tag/Argument Parsing

```typescript
export const tagToMs = (args: Set<string>) => {
  // "500ms" → 500
  // "2s" → 2000
  // "300" → 300
}

export const tagHas = (tags, tag, defaultValue) => tags?.has(tag.toLowerCase())
export const tagFirst = (tags, defaultValue) => tags?.values().next().value ?? defaultValue
```

## math.ts — Interpolation Utilities

```typescript
export const clamp = (value, min, max) => Math.max(min, Math.min(max, value))
export const lerp = (min, max, t, clamped = true) => min + (max - min) * t
export const inverseLerp = (min, max, value, clamped = true) => (value - min) / (max - min)
export const fit = (value, inMin, inMax, outMin, outMax, clamped, rounded) =>
  lerp(outMin, outMax, inverseLerp(inMin, inMax, value), clamped)
```

Used primarily by the `on-intersect` plugin for threshold calculations.

## paths.ts — Object Path Utilities

```typescript
export const isPojo = (obj) =>
  obj !== null && typeof obj === 'object' &&
  (Object.getPrototypeOf(obj) === Object.prototype || Object.getPrototypeOf(obj) === null)

export const updateLeaves = (obj, fn) => {
  // Recursively walk an object, applying fn to each leaf value
}

export const pathToObj = (paths) => {
  // [['user.name', 'Alice']] → { user: { name: 'Alice' } }
}
```

## polyfills.ts — Object.hasOwn

```typescript
export const hasOwn = Object.hasOwn ?? Object.prototype.hasOwnProperty.call
```

Provides `Object.hasOwn` for older browsers.

## dom.ts — Type Guard

```typescript
export const isHTMLOrSVG = (el: Node): el is HTMLOrSVG =>
  el instanceof HTMLElement || el instanceof SVGElement || el instanceof MathMLElement
```

## view-transitions.ts — View Transitions API

```typescript
export const supportsViewTransitions = !!document.startViewTransition

export const modifyViewTransition = (callback, mods) => {
  if (mods.has('viewtransition') && supportsViewTransitions) {
    callback = (...args) => document.startViewTransition(() => callback(...args))
  }
  return callback
}
```

Wraps callbacks in the View Transitions API when the browser supports it and the `.viewtransition` modifier is present.

See [Attribute Plugins](05-attribute-plugins.md) for how plugins use these utilities.
See [Plugin System](04-plugin-system.md) for how modifiers are parsed.
