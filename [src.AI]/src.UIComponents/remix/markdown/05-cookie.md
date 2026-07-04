# Cookie Toolkit: Parsing, Serialization, and HMAC Signing

`@remix-run/cookie` (v0.5.0) provides typed cookie parsing, serialization, and cryptographic signing using Web Crypto API (HMAC-SHA256).

Source: `packages/cookie/src/` — 4 source files.

## Cookie Interface

Source: `lib/cookie.ts`

```typescript
export interface Cookie {
  name: string
  isSigned: boolean
  parse(cookieHeader?: string | null): Promise<Record<string, unknown> | null>
  serialize(value: Record<string, unknown> | null, options?: CookieSerializeOptions): Promise<string>
}
```

## createCookie

```typescript
export function createCookie(name: string, options?: CookieOptions): Cookie {
  const secrets = options?.signingKey
    ? typeof options.signingKey === 'string'
      ? [options.signingKey]
      : options.signingKey.filter(Boolean)
    : []

  return new CookieImpl(name, secrets, options)
}
```

### Key Rotation

Multiple secrets are supported: the **first** secret is used for signing, **all** secrets are tried for verification. This enables seamless key rotation:

```typescript
const cookie = createCookie('session', {
  signingKey: ['new-key-2024', 'old-key-2023'],  // First is active, all tried for verify
})
```

## CookieImpl — parse()

```typescript
async parse(cookieHeader?: string | null): Promise<Record<string, unknown> | null> {
  if (!cookieHeader) return null

  const cookies = parseCookie(cookieHeader)
  const cookieValue = cookies[this.name]
  if (!cookieValue) return null

  if (!this.isSigned) {
    return JSON.parse(decode(cookieValue))
  }

  const unsigned = await unsign(cookieValue, this.secrets)
  if (!unsigned) return null  // Signature verification failed

  return JSON.parse(decode(unsigned))
}
```

### parseCookie() — Raw Cookie Header Parsing

```typescript
function parseCookie(cookieHeader: string): Record<string, string> {
  const cookies: Record<string, string> = {}
  for (const piece of cookieHeader.split(/\s*;\s*/)) {
    const [name, ...rest] = piece.split('=')
    if (name) {
      cookies[decodeURIComponent(name)] = decodeURIComponent(rest.join('='))
    }
  }
  return cookies
}
```

Splits on `;`, then on first `=`. Values after `=` may contain `=` characters, so `rest.join('=')` reconstructs them.

### decode() — URL Decoding

```typescript
function decode(value: string): string {
  if (value.startsWith('u:')) {
    return decodeURIComponent(value.slice(2))
  }
  return value
}
```

The `u:` prefix marks URL-encoded values.

## CookieImpl — serialize()

```typescript
async serialize(value: Record<string, unknown> | null, options?: CookieSerializeOptions): Promise<string> {
  if (!value) {
    return serialize(this.name, '', { ...options, maxAge: 0 })  // Expire the cookie
  }

  const encoded = encode(JSON.stringify(value))
  const cookieValue = this.isSigned
    ? await sign(encoded, this.secrets[0])
    : encoded

  return serialize(this.name, cookieValue, options)
}
```

### encode() — URL Encoding

```typescript
function encode(value: string): string {
  return `u:${encodeURIComponent(value)}`
}
```

Prefixes with `u:` to mark as URL-encoded.

## Cookie Signing — HMAC-SHA256

Source: `lib/cookie-signing.ts`

### sign()

```typescript
export async function sign(value: string, secret: string): Promise<string> {
  const key = await crypto.subtle.importKey(
    'raw',
    encoder.encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  )

  const signature = await crypto.subtle.sign('HMAC', key, encoder.encode(value))
  return `${value}.${toBase64(signature)}`
}
```

Uses Web Crypto API (`crypto.subtle`), not Node.js `crypto`. Works in any JS environment: browser, worker, Deno, Node.

### unsign()

```typescript
export async function unsign(cookie: string, secrets: string[]): Promise<string | null> {
  const lastDot = cookie.lastIndexOf('.')
  if (lastDot === -1) return null

  const value = cookie.slice(0, lastDot)
  const signature = cookie.slice(lastDot + 1)

  for (const secret of secrets) {
    const key = await crypto.subtle.importKey(
      'raw',
      encoder.encode(secret),
      { name: 'HMAC', hash: 'SHA-256' },
      false,
      ['verify'],
    )

    const valid = await crypto.subtle.verify(
      'HMAC',
      key,
      fromBase64(signature),
      encoder.encode(value),
    )

    if (valid) return value
  }

  return null
}
```

Tries each secret in order. First match wins. Returns `null` if no secret verifies.

### toBase64() / fromBase64()

```typescript
function toBase64(buffer: ArrayBuffer): string {
  return btoa(String.fromCharCode(...new Uint8Array(buffer)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=/g, '')
}
```

URL-safe Base64 (no `+`, `/`, or `=` padding).

```typescript
function fromBase64(base64: string): Uint8Array {
  const padded = base64
    .replace(/-/g, '+')
    .replace(/_/g, '/')
    .padEnd(base64.length + (4 - (base64.length % 4)) % 4, '=')

  const binary = atob(padded)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes
}
```

Aha: The signed cookie format is `value.signature`. The signature is a URL-safe Base64-encoded HMAC-SHA256 of the value. The `unsign` function splits on the **last** dot, so values containing dots are handled correctly.

## Package Files

```
cookie/src/
├── index.ts                # Public API
└── lib/
    ├── cookie.ts            # Cookie interface, createCookie, CookieImpl
    ├── cookie-signing.ts    # HMAC-SHA256 sign/unsign
    └── cookie.test.ts       # Tests
```

## Cross-References

- [Overview](00-overview.md) — What Remix is
- [Session](04-session.md) — Session management (uses Cookie for CookieStorage)
- [Headers](06-headers.md) — HTTP headers (includes Cookie header parsing)

[Next → Headers](06-headers.md)
