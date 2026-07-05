---
title: remix-the-web — TypeScript Web Framework with 25 Packages
---

# remix-the-web — Model-First TypeScript Web Framework

**remix-the-web is a 25-package TypeScript web framework designed for model-first, LLM-friendly web development.** It's not the React Router v7 Remix from Shopify — it's a separate project with a distinct architecture: a `fetch`-based router (everything is `Request` → `Response`), deterministic route pattern matching (two backends: RegExp and Trie), a virtual DOM component system, streaming-first server, and standalone middleware/cookie/header/session packages. All 25 packages are independently versioned, zero-dependency where possible, and compiled with `@typescript/native-preview` for fast native execution.

Source: `packages/` — 25 packages, TypeScript, pnpm workspace

## Architecture

```mermaid
flowchart TD
    subgraph Router["Router Layer"]
        FetchRouter["fetch-router<br/>Router.map() + Router.fetch()"]
        RoutePattern["route-pattern<br/>RoutePattern<T><br/>RegExpMatcher + TrieMatcher"]
    end

    subgraph Server["Server Layer"]
        NodeFetchServer["node-fetch-server<br/>createRequestListener()"]
        FetchProxy["fetch-proxy<br/>proxy fetch to different origin"]
        StaticMW["static-middleware"]
        CompressionMW["compression-middleware"]
    end

    subgraph HTTP["HTTP Primitives"]
        Headers["headers<br/>37 files of typed HTTP headers"]
        Response["response<br/>Response builders + helpers"]
        Cookie["cookie<br/>parse/serialize"]
        Mime["mime<br/>content-type detection"]
    end

    subgraph State["State Management"]
        Session["session<br/>createSession, SessionStorage"]
        SessionMW["session-middleware"]
        AppStorage["AppStorage<br/>createStorageKey"]
    end

    subgraph Component["Component System"]
        Component["component<br/>createRoot(), createElement()<br/>VDOM + JSX"]
        Interaction["interaction<br/>form actions, event handlers"]
        HtmlTemplate["html-template<br/>Streaming HTML rendering"]
    end

    subgraph Parsing["Parsing"]
        MultipartParser["multipart-parser<br/>streaming multipart"]
        FormDataParser["form-data-parser"]
        FormDataMW["form-data-middleware"]
        TarParser["tar-parser<br/>streaming tar extraction"]
    end

    subgraph I_O["I/O Layer"]
        FS["fs<br/>typed filesystem wrapper"]
        LazyFile["lazy-file<br/>Buffer/Memory/StreamFile"]
        FileStorage["file-storage<br/>plugable disk/S3 storage"]
    end

    FetchRouter --> RoutePattern
    NodeFetchServer --> FetchRouter
    SessionMW --> Session
    Component --> Interaction
    MultipartParser --> FormDataParser --> FormDataMW
```

## Key Design Decisions

### 1. Everything is `Request` → `Response`

**Aha:** There's no framework-specific context object. Every handler receives a standard `Request` and returns a standard `Response`. This means:
- Any middleware that works with the Fetch API works with remix
- Server and client code share the same types
- Testing is `new Request('/url')` → `router.fetch(request)`

```typescript
// fetch-router/src/lib/router.ts:43-67
export interface Router {
  fetch(input: string | URL | Request, init?: RequestInit): Promise<Response>
  readonly size: number
  route<method extends RequestMethod | 'ANY', pattern extends string>(
    method: method,
    pattern: pattern | RoutePattern<pattern>,
    handler: RouteHandler<method, pattern>,
  ): void
  map<route extends RouteMap | Route | RoutePattern<string> | string>(
    route: route,
    handler: /* conditional type that infers the correct handler shape */
  ): void
}
```

### 2. Route Patterns with Two Matching Backends

```typescript
// route-pattern/src/lib/route-pattern.ts:17-24
export class RoutePattern<T extends string = string> {
  readonly source: T
  readonly ignoreCase: boolean

  constructor(source: T | RoutePattern<T>, options?: RoutePatternOptions) { ... }
  href(...args: HrefBuilderArgs<T>): string     // Type-safe URL generation
  join<P extends string>(input: P): RoutePattern<Join<T, P>>  // Compose patterns
  match(url: URL | string): RouteMatch<T> | null  // Match against actual URLs
}
```

Two matcher backends:
- **`RegExpMatcher`** — O(n) over routes, compiles patterns to regex at registration
- **`TrieMatcher`** — O(path_segments) lookup, better for large route tables

**Aha:** `RoutePattern.join()` composes patterns at the TYPE level. `'/users/:id'.join('/posts/:postId')` produces a `RoutePattern<'/users/:id/posts/:postId'>`. The type system tracks parameter names through composition — you can't accidentally reference `:id` after joining with a pattern that also has `:id`.

### 3. Virtual DOM Component System

```typescript
// component/src/index.ts
export { createRoot } from './lib/vdom.ts'       // Mount point
export { createElement } from './lib/create-element.ts'  // JSX factory
export { Fragment } from './lib/component.ts'     // Fragment support
```

**Aha:** `component` implements its own VDOM — no React, no Preact dependency. `createElement()` produces a virtual DOM tree. `createRoot()` mounts it to a real DOM node. This is deliberately minimalistic: it supports JSX, function components, and event handlers, without any of React's concurrent mode, fiber tree, or hooks infrastructure.

### 4. Streaming Node Server

```typescript
// node-fetch-server/src/index.ts
export { createRequestListener } from './lib/request-listener.ts'
// Wraps a Router.fetch() into a Node.js http.RequestListener
// Handles: streaming responses, abort signals, client addresses
// Supports: HTTP/1.1 and HTTP/2
```

### 5. Typed Headers — 37 Files

The `headers` package has 37 files, one per header family:
- `accept.ts`, `authorization.ts`, `cache-control.ts`, `content-type.ts`, `cookie.ts`
- `cors.ts`, `csp.ts`, `etag.ts`, `host.ts`, `range.ts`, `set-cookie.ts`
- ... and 26 more

Each file provides parse/serialize/validate for its header type. This means you get IntelliSense for header values:

```typescript
import { Accept } from '@remix-run/headers'
const accept = Accept.parse(request.headers.get('accept'))
// accept.preferred(['text/html', 'application/json']) → 'text/html'
```

## Package Map — All 25

| Package | Files | Purpose |
|---------|-------|---------|
| **fetch-router** | 24 | Request → Response router with middleware, resources, form actions |
| **route-pattern** | 27 | Deterministic URL pattern matching (RegExp + Trie), typed href generation |
| **headers** | 37 | Typed HTTP header parsers/serializers per header family |
| **session** | 13 | Session creation, SessionStorage interface, cookie-based sessions |
| **response** | 12 | Response builder helpers, typed status codes |
| **component** | 17 | Virtual DOM, JSX factory, Fragment, createRoot |
| **interaction** | 8 | Form actions, event handler binding for VDOM |
| **node-fetch-server** | 5 | Node.js server wrapping fetch-based Router |
| **multipart-parser** | 10 | Streaming multipart/form-data parser |
| **mime** | 7 | Content-type detection from file extension |
| **file-storage** | 8 | Pluggable file storage (disk, S3) |
| **lazy-file** | 6 | Lazy file abstraction (Buffer, Memory, Stream) |
| **html-template** | 3 | Streaming HTML template rendering |
| **form-data-parser** | 3 | Form data parsing with file support |
| **form-data-middleware** | 3 | Auto-parse form data in requests |
| **session-middleware** | 3 | Auto-attach sessions to requests |
| **logger-middleware** | 3 | Request/response logging |
| **method-override-middleware** | 3 | `_method` parameter for form-based PUT/DELETE |
| **static-middleware** | 4 | Static file serving |
| **compression-middleware** | 3 | Response compression (gzip, deflate, brotli) |
| **async-context-middleware** | 3 | `AsyncLocalStorage`-based request context |
| **cookie** | 4 | Cookie parse/serialize |
| **fs** | 3 | Typed filesystem operations |
| **fetch-proxy** | 3 | Fetch proxy to upstream origin |
| **tar-parser** | 6 | Streaming tar archive extraction |

## Middleware Pattern

```typescript
// fetch-router/src/lib/middleware.ts
export interface Middleware<Env = unknown> {
  (request: Request, next: NextFunction): Response | Promise<Response>
}
export interface NextFunction {
  (request?: Request): Response | Promise<Response>
}
```

Middleware follows the standard `(request, next) => response` pattern. Global middleware runs before route matching. Route-specific middleware runs only for matching routes. The `RequestContext` provides an `AsyncLocalStorage`-backed context for request-scoped state.

## Cross-References

- [Instatic](../src.Instatic/markdown/00-overview.md) — Self-hosted CMS (also uses a hand-written router)
- [Streamlit](../src.streamlit/markdown/00-overview.md) — Python web app framework (similar "rewrite from scratch" philosophy)

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.UIComponents/remix
python3 build.py .
```
