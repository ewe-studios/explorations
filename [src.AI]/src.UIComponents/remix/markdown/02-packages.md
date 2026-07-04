# Remix 3: All 25 Packages

Each package has a single responsibility, works standalone, and is portable across runtimes.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/remix/packages/`

## Routing

### fetch-router
URL routing with fetch handlers. Routes incoming requests to handlers based on URL patterns.

### route-pattern
URL route pattern matching. Parses route patterns like `/users/:id` and matches against URLs.

### method-override-middleware
HTTP method override. Allows POST requests to simulate PUT/DELETE/PATCH via `_method` parameter.

## Request/Response

### headers
HTTP header utilities. Parse, serialize, and manipulate HTTP headers.

### response
Response utilities. Helpers for creating and transforming HTTP responses.

### fetch-proxy
Fetch proxy utilities. Wrap fetch calls with additional behavior.

## Sessions

### session
Session management. Create, read, update, and destroy sessions.

### session-middleware
Session middleware. Automatically loads/saves sessions for each request.

### cookie
Cookie parsing and serialization. Parse Set-Cookie headers and Cookie headers.

## Forms

### form-data-parser
Multipart form data parser. Parse `multipart/form-data` bodies.

### form-data-middleware
Form data parsing middleware. Automatically parse form data from requests.

### multipart-parser
Multipart body parser. Low-level multipart parser for file uploads.

## File Handling

### fs
File system utilities. Portable file operations across runtimes.

### file-storage
File storage abstraction. Abstract file storage backend.

### lazy-file
Lazy file loading. Load file contents on demand.

### tar-parser
TAR archive parser. Parse TAR archives for deployment.

## Middleware

### async-context-middleware
Async context for request-scoped data. Store per-request data accessible throughout the request lifecycle.

### compression-middleware
Response compression. Compress responses with gzip/deflate.

### logger-middleware
Request logging middleware. Log incoming requests and responses.

### static-middleware
Static file serving. Serve files from a directory.

## Rendering

### html-template
HTML template rendering. Render HTML from templates.

### component
UI component primitives. Base component building blocks.

### interaction
User interaction handling. Handle user events and form submissions.

## Utilities

### mime
MIME type utilities. Detect and map MIME types.

### node-fetch-server
Node.js fetch-based server. HTTP server using the fetch API on Node.js.

## Next

Return to the [README](README.md) for the full index.

## All Packages Summary

### Core Packages (6)

| Package | Version | Description |
|---------|---------|-------------|
| `@remix-run/component` | 0.1.0 | Web components for Remix |
| `@remix-run/route-pattern` | 0.15.3 | Match and generate URLs with strong typing |
| `@remix-run/session` | 0.4.0 | Session management for JavaScript |
| `@remix-run/cookie` | 0.5.0 | Cookie toolkit |
| `@remix-run/response` | 0.2.0 | Response helpers for Fetch API |
| `@remix-run/headers` | 0.18.0 | HTTP headers toolkit |

### Middleware (6)

| Package | Description |
|---------|-------------|
| `@remix-run/async-context-middleware` | Async context propagation |
| `@remix-run/compression-middleware` | Response compression |
| `@remix-run/method-override-middleware` | HTTP method override |
| `@remix-run/static-middleware` | Static file serving |
| `@remix-run/session-middleware` | Session middleware |
| `@remix-run/form-data-middleware` | Form data processing |
| `@remix-run/logger-middleware` | Request logging |

### I/O & Parsing (9)

| Package | Description |
|---------|-------------|
| `@remix-run/fetch-proxy` | Fetch API proxy |
| `@remix-run/fetch-router` | Fetch-based router |
| `@remix-run/file-storage` | File storage abstraction |
| `@remix-run/fs` | File system utilities |
| `@remix-run/html-template` | HTML templating |
| `@remix-run/lazy-file` | Lazy file loading |
| `@remix-run/multipart-parser` | Multipart parser |
| `@remix-run/node-fetch-server` | Node fetch server |
| `@remix-run/tar-parser` | TAR archive parser |

### Utilities (3)

| Package | Description |
|---------|-------------|
| `@remix-run/interaction` | Interaction utilities |
| `@remix-run/mime` | MIME type detection |
| `@remix-run/form-data-parser` | Form data parsing |

[Next → Architecture](03-architecture.md)
