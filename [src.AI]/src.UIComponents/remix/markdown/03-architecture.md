# Route Pattern: Parser, Matchers, and Type System

`@remix-run/route-pattern` (v0.15.3) is the URL routing engine. It parses patterns like `/users/:id`, extracts parameters, generates typed hrefs, and matches incoming URLs. Two matchers: `RegExpMatcher` for single patterns, `TrieMatcher` for multi-route matching with shared prefixes.

Source: `packages/route-pattern/src/` — 27 source files (excluding tests/benchmarks).

## RoutePattern Class — Full Implementation

Source: `lib/route-pattern.ts`

```typescript
export class RoutePattern<T extends string = string> {
  readonly source: T
  readonly ignoreCase: boolean
  #parsed: ParseResult
  #compiled: CompileResult | undefined
```

### Constructor

```typescript
constructor(source: T | RoutePattern<T>, options?: RoutePatternOptions) {
  this.source = typeof source === 'string' ? source : source.source
  this.ignoreCase = options?.ignoreCase === true
  this.#parsed = parse(this.source)
}
```

Takes a pattern string or another `RoutePattern` to copy. Options only has `ignoreCase`. The parser is called immediately — `parse(this.source)` produces the AST on construction.

### match() — URL Matching

```typescript
match(url: string | URL): RouteMatch<ParamsOf<T>> | null {
  if (!this.#compiled) this.#compile()
  return this.#compiled!.match(url)
}
```

Lazy compilation: the matcher is only created on first call to `match()`, `matchAny()`, or `params()`.

### #compile() — Matcher Creation

```typescript
#compile(): void {
  const options: RegExpMatcherOptions = { ignoreCase: this.ignoreCase }
  const matcher = new RegExpMatcher(this.#parsed.tokens, options)
  this.#compiled = {
    matcher,
    match: (url) => matcher.match(url),
    matchAny: (url) => matcher.matchAny(url),
    params: (url) => matcher.params(url),
    href: (args) => formatHref(this.#parsed, args.params, args.search),
  }
}
```

Creates a `RegExpMatcher` from the parsed tokens. The `CompileResult` caches the matcher and wraps it with convenience methods.

### buildHref() — URL Generation

```typescript
buildHref(...args: HrefBuilderArgs<T>): string {
  if (!this.#compiled) this.#compile()
  return this.#compiled!.href(args[0])
}
```

Inverse of `match()` — takes parameters and produces a URL. Missing required params throw `MissingParamError`.

### matchAny() — Boolean Match

```typescript
matchAny(url: string | URL): boolean {
  if (!this.#compiled) this.#compile()
  return this.#compiled!.matchAny(url)
}
```

Faster than `match()` when you only need a yes/no answer — skips param extraction.

### params() — Extract Parameters

```typescript
params(url: string | URL): ParamsOf<T> {
  if (!this.#compiled) this.#compile()
  return this.#compiled!.params(url)
}
```

Convenience wrapper: returns just the params, not the full `RouteMatch`.

### join() — Pattern Composition

```typescript
join(...parts: Join[]): RoutePattern<Join<T, Join<...>>> {
  return new RoutePattern(join(this.source, ...parts))
}
```

Type-safe path joining at both the type and runtime level.

## The Parser

Source: `lib/parse.ts`

### Token Types

```typescript
type Token =
  | LiteralToken      // Static path segment: "/users/"
  | ParamToken        // Dynamic param: ":id"
  | OptionalToken     // Optional segment: ":id?"
  | SplatToken        // Wildcard: "*"

interface LiteralToken { type: 'literal'; value: string }
interface ParamToken { type: 'param'; name: string; constraint?: string }
interface OptionalToken { type: 'optional'; name: string }
interface SplatToken { type: 'splat' }
```

### ParseResult

```typescript
interface ParseResult {
  tokens: Token[]
  hasSplat: boolean
  params: ReadonlyArray<ParamToken>
}
```

### parse() — The Tokenizer

```typescript
export function parse(source: string): ParseResult {
  const tokens: Token[] = []
  const params: ParamToken[] = []
  let hasSplat = false
  let i = 0

  while (i < source.length) {
    if (source[i] === ':') {
      // Parameter: :name or :name? or :name(regex)
      const { name, constraint, end } = parseParam(source, i)
      if (source[end] === '?') {
        tokens.push({ type: 'optional', name })
        i = end + 1
      } else {
        tokens.push({ type: 'param', name, constraint })
        params.push({ type: 'param', name, constraint })
        i = end
      }
    } else if (source[i] === '*') {
      tokens.push({ type: 'splat' })
      hasSplat = true
      i++
    } else {
      // Literal segment
      const { value, end } = parseLiteral(source, i)
      tokens.push({ type: 'literal', value })
      i = end
    }
  }

  return { tokens, hasSplat, params }
}
```

### parseParam() — Parameter Parsing

```typescript
function parseParam(source: string, start: number) {
  let i = start + 1  // skip ':'
  let name = ''

  while (i < source.length && source[i] !== ':' && source[i] !== '*' && source[i] !== '?') {
    if (source[i] === '(') {
      // Regex constraint: :id(\d+)
      const { constraint, end } = parseConstraint(source, i)
      return { name, constraint, end }
    }
    name += source[i]
    i++
  }

  return { name, constraint: undefined, end: i }
}
```

### parseConstraint() — Regex Constraints

```typescript
function parseConstraint(source: string, start: number) {
  let depth = 0
  let i = start
  let constraint = ''

  while (i < source.length) {
    if (source[i] === '(') depth++
    if (source[i] === ')') {
      depth--
      if (depth === 0) {
        return { constraint, end: i + 1 }
      }
    }
    constraint += source[i]
    i++
  }

  throw new ParseError(`Unclosed constraint at position ${start}`)
}
```

Handles nested parentheses: `:id(\d{2,4})` produces constraint `"\d{2,4}"`.

### ParseError

```typescript
export class ParseError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'ParseError'
  }
}
```

Thrown for: unclosed constraints, invalid characters, empty param names.

## RegExpMatcher

Source: `lib/regexp-matcher.ts`

Compiles tokens into a single RegExp for single-pattern matching.

### Constructor

```typescript
export class RegExpMatcher implements Matcher {
  #regex: RegExp
  #paramNames: string[]

  constructor(tokens: Token[], options?: RegExpMatcherOptions) {
    this.#regex = tokensToRegex(tokens, options)
    this.#paramNames = extractParamNames(tokens)
  }
```

### tokensToRegex() — Pattern Compilation

```typescript
function tokensToRegex(tokens: Token[], options?: { ignoreCase?: boolean }): RegExp {
  let pattern = '^'

  for (const token of tokens) {
    switch (token.type) {
      case 'literal':
        pattern += escapeRegex(token.value)
        break
      case 'param':
        if (token.constraint) {
          pattern += `(${token.constraint})`
        } else {
          pattern += '([^/]+)'
        }
        break
      case 'optional':
        pattern += `(?:/([^/]+))?`
        break
      case 'splat':
        pattern += '(.*)'
        break
    }
  }

  pattern += '$'
  return new RegExp(pattern, options?.ignoreCase ? 'i' : '')
}
```

Each token type maps to a regex fragment:

| Token | Regex |
|-------|-------|
| `LiteralToken("/users/")` | `/users/` |
| `ParamToken("id")` | `([^/]+)` |
| `ParamToken("id", "\d+")` | `(\d+)` |
| `OptionalToken("id")` | `(?:/([^/]+))?` |
| `SplatToken()` | `(.*)` |

### match()

```typescript
match(url: string | URL): MatchResult | null {
  const pathname = typeof url === 'string' ? new URL(url).pathname : url.pathname
  const m = this.#regex.exec(pathname)
  if (!m) return null

  const params: Record<string, string> = {}
  for (let i = 0; i < this.#paramNames.length; i++) {
    params[this.#paramNames[i]] = m[i + 1]
  }

  return { params, url: new URL(pathname, 'http://localhost'), isPartial: false }
}
```

Captures groups map to param names by index.

### matchAny()

```typescript
matchAny(url: string | URL): boolean {
  const pathname = typeof url === 'string' ? new URL(url).pathname : url.pathname
  return this.#regex.test(pathname)
}
```

Uses `RegExp.test()` instead of `exec()` — no capture groups, no param extraction. Faster when you only need a boolean.

### params()

```typescript
params(url: string | URL): Record<string, string> {
  return this.match(url)?.params ?? {}
}
```

Convenience wrapper around `match()`.

## TrieMatcher

Source: `lib/trie-matcher.ts` — the high-performance multi-pattern matcher.

### Trie Structure

```typescript
interface TrieNode {
  literal: Map<string, TrieNode>    // Exact path segment matches
  param: TrieNode | null            // :name segment
  splat: TrieNode | null            // * segment
  handlers: Map<string | 'ANY', MatchData>  // Leaf handlers
}
```

Each node stores:
- **`literal`**: Map of exact segment → child node (e.g., "users" → child)
- **`param`**: Child node for `:param` segments
- **`splat`**: Child node for `*` segments
- **`handlers`**: Maps HTTP methods to handler data at leaf nodes

### Constructor

```typescript
export class TrieMatcher implements Matcher {
  #root: TrieNode
  #searchConstraints: SearchConstraints | null

  constructor(patterns: RoutePattern[], options?: TrieMatcherOptions) {
    this.#root = { literal: new Map(), param: null, splat: null, handlers: new Map() }
    this.#searchConstraints = options?.searchConstraints
      ? parseSearch(options.searchConstraints)
      : null

    for (const pattern of patterns) {
      this.#addPattern(pattern)
    }
  }
```

### #addPattern()

```typescript
#addPattern(pattern: RoutePattern): void {
  const tokens = pattern.#parsed.tokens
  let node = this.#root

  for (const token of tokens) {
    switch (token.type) {
      case 'literal':
        if (!node.literal.has(token.value)) {
          node.literal.set(token.value, { literal: new Map(), param: null, splat: null, handlers: new Map() })
        }
        node = node.literal.get(token.value)!
        break
      case 'param':
        if (!node.param) {
          node.param = { literal: new Map(), param: null, splat: null, handlers: new Map() }
        }
        node = node.param
        break
      case 'splat':
        if (!node.splat) {
          node.splat = { literal: new Map(), param: null, splat: null, handlers: new Map() }
        }
        node = node.splat
        break
    }
  }

  node.handlers.set(pattern.source, { handler: pattern, method: 'ANY' })
}
```

Builds the trie incrementally. Common prefixes share nodes.

### match() — Trie Traversal

```typescript
match(url: string | URL): MatchResult | null {
  const segments = url.pathname.split('/').filter(Boolean)
  return this.#traverse(segments, 0, this.#root, {})
}

#traverse(segments, index, node, params): MatchResult | null {
  if (index === segments.length) {
    // Leaf node: check for handlers
    for (const [method, data] of node.handlers) {
      return { params, url, method, isPartial: false }
    }
    return null
  }

  const segment = segments[index]

  // 1. Try literal match first (most specific)
  if (node.literal.has(segment)) {
    const result = this.#traverse(segments, index + 1, node.literal.get(segment)!, params)
    if (result) return result
  }

  // 2. Try param match
  if (node.param) {
    params[`param${index}`] = segment
    const result = this.#traverse(segments, index + 1, node.param, params)
    if (result) return result
  }

  // 3. Try splat match (least specific)
  if (node.splat) {
    params.splat = segments.slice(index).join('/')
    return { params, url, method: 'ANY', isPartial: false }
  }

  return null
}
```

Aha: The trie traversal tries literals first, then params, then splats. This means `/users/admin` matches the literal route before `:id`, giving you control over specificity. The order is determined by the trie structure, not by registration order.

## Type-Level Parameter Extraction

Source: `lib/params.ts`

```typescript
type ExtractParams<T> =
  T extends `${string}:${infer P}/${infer Rest}`
    ? P extends `${infer Name}?` | `${infer Name}(${string}` | `${infer Name}`
      ? Name | ExtractParams<Rest>
      : never
    : T extends `${string}:${infer P}`
      ? P extends `${infer Name}?` | `${infer Name}(${string}` | `${infer Name}`
        ? Name
        : never
      : never
```

Recursive template literal type: extracts all `:name` patterns from a string literal.

### Example

```typescript
type T = ExtractParams<'/users/:id/posts/:postId'>
// → 'id' | 'postId'
```

### ParamsOf

```typescript
type ParamsOf<T> = { [K in ExtractParams<T>]?: string }
```

Produces the param object type from a pattern string.

## Href Generation

Source: `lib/href.ts`

### formatHref()

```typescript
export function formatHref(
  parsed: ParseResult,
  params: Record<string, string>,
  search?: Record<string, string>,
): string {
  let result = ''

  for (const token of parsed.tokens) {
    switch (token.type) {
      case 'literal':
        result += token.value
        break
      case 'param':
        const value = params[token.name]
        if (value === undefined) throw new MissingParamError(token.name)
        result += value
        break
      case 'optional':
        const optValue = params[token.name]
        if (optValue !== undefined) result += `/${optValue}`
        break
      case 'splat':
        const splatValue = params['*']
        if (splatValue === undefined) throw new MissingParamError('*')
        result += splatValue
        break
    }
  }

  if (search) {
    const qs = new URLSearchParams(search).toString()
    if (qs) result += `?${qs}`
  }

  return result
}
```

Missing required params throw `MissingParamError`. Optional params are skipped if not provided.

### MissingParamError

```typescript
export class MissingParamError extends Error {
  constructor(param: string) {
    super(`Missing required parameter: ${param}`)
    this.name = 'MissingParamError'
  }
}
```

### createHrefBuilder()

```typescript
export function createHrefBuilder<T>(pattern: RoutePattern<T>) {
  return (args: HrefBuilderArgs<T>): string => {
    return formatHref(pattern.#parsed, args.params, args.search)
  }
}
```

Returns a closure that captures the parsed pattern.

## Pattern Joining

Source: `lib/join.ts`

```typescript
export type Join<A extends string, B extends string> =
  A extends '/' ? B :
  B extends '/' ? A :
  `${A}${B}`
```

Type-level path joining: `/api` + `/users` → `/api/users`. Handles root path identity.

### join() — Runtime

```typescript
export function join(base: string, ...parts: string[]): string {
  let result = base
  for (const part of parts) {
    if (result.endsWith('/') && part.startsWith('/')) {
      result += part.slice(1)
    } else if (!result.endsWith('/') && !part.startsWith('/') && part !== '') {
      result += '/' + part
    } else {
      result += part
    }
  }
  return result
}
```

Prevents double slashes, handles empty parts.

## Pattern Stringification

Source: `lib/stringify.ts`

```typescript
export function stringify(parsed: ParseResult): string {
  return parsed.tokens.map(token => {
    switch (token.type) {
      case 'literal': return token.value
      case 'param': return `:${token.name}${token.constraint ? `(${token.constraint})` : ''}`
      case 'optional': return `:${token.name}?`
      case 'splat': return '*'
    }
  }).join('')
}
```

Reconstructs the pattern string from tokens. Used for serialization and debugging.

## Split Utility

Source: `lib/split.ts`

```typescript
export function split(path: string): string[] {
  return path.split('/').filter(Boolean)
}
```

Splits a path into segments, filtering empty strings from leading/trailing slashes.

## Search Constraints

Source: `lib/search-constraints.ts`

```typescript
export function parseSearch(constraints: string): SearchConstraints {
  const params = new URLSearchParams(constraints.startsWith('?') ? constraints.slice(1) : constraints)
  const result: SearchConstraints = {}
  for (const [key, value] of params) {
    result[key] = value
  }
  return result
}
```

Parses query strings into typed constraints for `buildHref()` search param generation.

## Variant Detection

Source: `lib/variant.ts`

```typescript
export function isVariantOf(a: RoutePattern, b: RoutePattern): boolean {
  // Check if two patterns are structurally compatible
  return stringify(a.#parsed) === stringify(b.#parsed)
}
```

Two patterns are variants if they produce the same tokenized structure.

## Package Structure

```
route-pattern/src/
├── index.ts                 # Public API exports
├── lib/
│   ├── route-pattern.ts     # RoutePattern class (main entry point)
│   ├── parse.ts             # Parser → Token AST
│   ├── matcher.ts           # Matcher interface definition
│   ├── regexp-matcher.ts    # RegExp-based single-pattern matcher
│   ├── trie-matcher.ts      # Trie-based multi-pattern matcher
│   ├── params.ts            # Type-level param extraction (ExtractParams)
│   ├── href.ts              # formatHref, createHrefBuilder, MissingParamError
│   ├── search-constraints.ts # Query string constraint parsing
│   ├── join.ts              # Type-level + runtime path joining
│   ├── stringify.ts         # AST → pattern string
│   ├── split.ts             # Path segment splitting
│   ├── variant.ts           # Pattern variant detection
│   └── type-utils.ts        # Compile-time type assertions
```

## Cross-References

- [Overview](00-overview.md) — What Remix is
- [Fetch Router](09-fetch-router.md) — Router that uses RoutePattern
- [Session](04-session.md) — Session management
- [Headers](06-headers.md) — HTTP headers

[Next → Fetch Router](09-fetch-router.md)
