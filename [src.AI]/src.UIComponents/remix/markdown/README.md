# remix Documentation

Remix 3: model-first web framework. 25 independently versioned packages for routing, sessions, cookies, HTTP headers, responses, web components, middleware, I/O, and parsing.

| Attribute | Value |
|-----------|-------|
| Packages | 25 (@remix-run/*) |
| Source files | ~294 |
| Language | TypeScript |
| Runtimes | Node.js, Bun, Deno, Cloudflare Workers |

## Navigation

### Core

| # | Document | Purpose |
|---|----------|---------|
| 00 | [Overview](00-overview.md) | What Remix 3 is, 25 packages, design principles |
| 01 | [Philosophy](01-philosophy.md) | 6 core principles: model-first, web APIs, runtime-first |
| 02 | [Packages](02-packages.md) | All 25 packages with versions and descriptions |
| 03 | [Route Pattern](03-architecture.md) | RoutePattern class, parser, RegExpMatcher, TrieMatcher, type-level param extraction |
| 04 | [Session](04-session.md) | Session class, 4-Map model, flash data lifecycle, 3 storage backends |
| 05 | [Cookie](05-cookie.md) | Cookie interface, HMAC-SHA256 signing via Web Crypto, key rotation |
| 06 | [Headers](06-headers.md) | SuperHeaders class, 14 typed header parsers, content negotiation |
| 07 | [Response](07-response.md) | HTML with streaming DOCTYPE injection, redirect, file, compression |
| 08 | [Component](08-component.md) | Web components, VDOM diffing, JSX runtime, stylesheet API |

### Router & Middleware

| # | Document | Purpose |
|---|----------|---------|
| 09 | [Fetch Router](09-node-fetch-server.md) | Router dispatch, middleware chain (3 control flows), route maps, resources, form actions |
| 10 | [Middleware](10-middleware.md) | AsyncLocalStorage, compression (br/gzip/deflate), method override, static files, session, form data, logger |

### I/O & Parsing

| # | Document | Purpose |
|---|----------|---------|
| 12 | [Fetch Proxy](12-fetch-proxy.md) | URL rewriting, cookie domain/path rewriting, X-Forwarded headers |
| 13 | [File Storage](13-file-storage.md) | Key/value File storage with pagination and metadata |
| 14 | [Multipart Parser](14-multipart-parser.md) | Boyer-Moore boundary search, partial tail matching across chunks, state machine |
| 15 | [MIME](15-mime.md) | Extension→MIME type mapping, compressibility check from mime-db |
| 16 | [FS](16-fs.md) | openFile → streaming LazyFile, auto MIME detection |
| 17 | [Lazy File](17-lazy-file.md) | LazyBlob/LazyFile with deferred streaming, ByteRange slicing |
| 18 | [Form Data Parser](18-form-data-parser.md) | Multipart→FormData with FileUpload handler, maxFiles limit |
| 19 | [HTML Template](19-html-template.md) | SafeHtml branded type, XSS-safe interpolation, html.raw for trusted content |
| 20 | [Node Fetch Server](20-node-fetch-server.md) | Request/Response ↔ IncomingMessage/ServerResponse, backpressure, abort signals |
| 21 | [TAR Parser](21-tar-parser.md) | UStar/GNU/PAX formats, checksum verification, streaming entry bodies |
| 22 | [Interaction](22-interaction.md) | Press, keys, popover, form events — WAI-ARIA normalized events |
