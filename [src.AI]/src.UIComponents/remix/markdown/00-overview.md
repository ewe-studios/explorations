# remix: Remix 3 — Model-First Web Framework

Remix 3 is a complete rethinking of the Remix framework, optimized for LLM-first development. 25 packages covering web APIs, middleware, sessions, forms, and more.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/remix/`

## Package Info

| Attribute | Value |
|-----------|-------|
| Files | 294 |
| Packages | 25 |
| Language | TypeScript |
| Runtime | Node.js, Bun, Deno, Cloudflare Workers |

## Design Principles

1. **Model-First Development** — Optimize source code, documentation, and abstractions for LLMs
2. **Build on Web APIs** — Web Streams, Uint8Array, Web Crypto (not Node.js-specific)
3. **Religiously Runtime** — No static analysis expectations, all tests run without bundling
4. **Avoid Dependencies** — Zero dependencies goal
5. **Demand Composition** — Single-purpose, replaceable packages
6. **Distribute Cohesively** — Single `remix` package for distribution

## 25 Packages

| Package | Purpose |
|---------|---------|
| `async-context-middleware` | Async context for request-scoped data |
| `component` | UI component primitives |
| `compression-middleware` | Response compression |
| `cookie` | Cookie parsing and serialization |
| `fetch-proxy` | Fetch proxy utilities |
| `fetch-router` | URL routing with fetch handlers |
| `file-storage` | File storage abstraction |
| `form-data-middleware` | Form data parsing middleware |
| `form-data-parser` | Multipart form data parser |
| `fs` | File system utilities |
| `headers` | HTTP header utilities |
| `html-template` | HTML template rendering |
| `interaction` | User interaction handling |
| `lazy-file` | Lazy file loading |
| `logger-middleware` | Request logging middleware |
| `method-override-middleware` | HTTP method override |
| `mime` | MIME type utilities |
| `multipart-parser` | Multipart body parser |
| `node-fetch-server` | Node.js fetch-based server |
| `response` | Response utilities |
| `route-pattern` | URL route pattern matching |
| `session` | Session management |
| `session-middleware` | Session middleware |
| `static-middleware` | Static file serving |
| `tar-parser` | TAR archive parser |

## Next

Continue to [01-philosophy](01-philosophy.md) for Remix 3 design philosophy.
