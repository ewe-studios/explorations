# HTML Template: Safe HTML with Branded Type

`@remix-run/html-template` provides a `SafeHtml` branded type and template literal helpers for XSS-safe HTML interpolation.

Source: `packages/html-template/src/lib/safe-html.ts` — 80+ lines.

## SafeHtml Branded Type

```typescript
const kSafeHtml: unique symbol = Symbol('safeHtml')

export type SafeHtml = String & { readonly [kSafeHtml]: true }

function createSafeHtml(value: string): SafeHtml {
  let s = new String(value) as SafeHtml
  ;(s as any)[kSafeHtml] = true
  return s
}

export function isSafeHtml(value: unknown): value is SafeHtml {
  return typeof value === 'object' && value != null && (value as any)[kSafeHtml] === true
}
```

Aha: Uses a `unique symbol` as a brand on a `String` object. The brand is not visible in the type's surface area but prevents accidental mixing of safe and unsafe strings. `isSafeHtml` checks for the symbol property at runtime — this is both a compile-time type guard and a runtime check.

## HTML Escaping

```typescript
const escapeRe = /[&<>"']/g
const escapeMap = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;',
} as const

function escapeHtml(text: string): string {
  return text.replace(escapeRe, (c) => escapeMap[c])
}
```

Escapes the 5 dangerous HTML characters. Uses a single regex pass with a lookup map — faster than chaining 5 `.replace()` calls.

## html Tagged Template

```typescript
export const html = ((strings, ...values) => {
  if (!isTemplateStringsArray(strings)) {
    throw new TypeError('html must be used as a template tag')
  }

  let out = ''
  for (let i = 0; i < strings.length; i++) {
    out += strings[i]
    if (i < values.length) out += stringifyInterpolation(values[i])
  }

  return createSafeHtml(out)
}) as SafeHtmlHelper
```

### stringifyInterpolation — Auto-Escaping

```typescript
function stringifyInterpolation(value: Interpolation): string {
  if (value == null) return ''
  if (Array.isArray(value)) return value.map(stringifyInterpolation).join('')
  if (isSafeHtml(value)) return String(value)      // Trusted: no escaping
  if (typeof value === 'string') return escapeHtml(value)  // Untrusted: escape
  if (typeof value === 'number' || typeof value === 'boolean') return escapeHtml(String(value))
  return escapeHtml(String(value))
}
```

| Value Type | Behavior |
|-----------|----------|
| `null` / `undefined` | Empty string |
| `SafeHtml` | Passed through without escaping (trusted) |
| `string` | **Escaped** (untrusted) |
| `number` | Converted to string, then escaped |
| `boolean` | Converted to string, then escaped |
| Array | Recursively processed |

### html.raw — Trusted Content

```typescript
html.raw = (strings, ...values) => {
  let out = ''
  for (let i = 0; i < strings.length; i++) {
    out += strings[i]
    if (i < values.length) out += stringifyRawInterpolation(values[i])
  }
  return createSafeHtml(out)
}
```

`stringifyRawInterpolation` is like `stringifyInterpolation` but does NOT escape strings. Use only for trusted content:

```typescript
let icon = '<b>OK</b>'
let safe = html.raw`<div>${icon}</div>`  // → '<div><b>OK</b></div>'
```

## Usage in Response

```typescript
// @remix-run/response uses isSafeHtml to detect safe HTML
import { isSafeHtml } from '@remix-run/html-template'

if (isSafeHtml(body)) {
  // Body is trusted HTML — don't escape, just ensure DOCTYPE
}
```

## Cross-References

- [Response](07-response.md) — Uses isSafeHtml for DOCTYPE injection
