# Server: Hand-Written Router and Handlers

Instatic's server is built on `Bun.serve` with a hand-written router — no Express, Hono, or Fastify. This document covers the 19-route routing table, handler pattern, CMS handlers, repositories, and the database adapter.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/Instatic/server/`

## Server Entrypoint

```typescript
// server/index.ts
// Boots Bun.serve with the handleServerRequest dispatcher
// Sets up DbClient (Postgres or SQLite), static dirs, uploads
```

The entrypoint creates the `ServerRuntime` object containing the DB client, static directory, uploads directory, and `DATABASE_URL`, then passes it to the request dispatcher.

**Source:** `server/index.ts`.

## Route Handler Interface

```typescript
// server/router.ts:45-50
type RouteHandler = (
  req: Request,
  runtime: ServerRuntime,
  url: URL,
  pathname: string,
) => Promise<Response | null> | Response | null
```

Each handler checks its own URL path and method. If it matches, it returns a `Response`. If not, it returns `null` and the dispatcher tries the next handler.

## Server Runtime

```typescript
// server/router.ts:25-35
interface ServerRuntime {
  db: DbClient           // Database client (Postgres or SQLite)
  staticDir?: string     // Static files directory
  uploadsDir?: string    // Uploads directory
  databaseUrl?: string   // Raw DATABASE_URL for handlers that need it
}
```

## The 15-Route Table

Routes are tried in strict order. The first match wins:

| # | Handler | URL Pattern | Purpose |
|---|---------|-------------|---------|
| 1 | `tryServeHealth` | `/health` | Health check: `{ status: 'ok', ts: ... }` |
| 2 | `tryServeAi` | `/admin/api/ai/*` | AI chat streams, credentials, models |
| 3 | `tryServeCmsApi` | `/admin/api/cms/*` | CMS CRUD operations |
| 4 | `tryServeLoopRuntimeAsset` | `/_instatic/loop/*` | Loop runtime assets |
| 5 | `tryServeLoop` | `/_instatic/loop/runtime/*` | Loop runtime execution |
| 6 | `tryServeHoleRuntimeAsset` | `/_instatic/hole/runtime/*` | Hole runtime assets |
| 7 | `tryServeHole` | `/_instatic/hole/<nodeId>` | Lazy-fetch dynamic content |
| 8 | `tryServeModuleJsAsset` | `/_instatic/module/*.js` | Module JS assets |
| 9 | `tryServePublicForm` | Form submissions | Native CMS form handling |
| 10 | `tryServeRuntimeAsset` | `/_instatic/runtime/*` | Runtime assets |
| 11 | `tryServeRuntimePackageNamespace` | `/_instatic/runtime/packages/*` | Per-site runtime packages |
| 12 | `tryServeSiteCssNamespace` | `/_instatic/css/*` | Per-page CSS bundles |
| 13 | `tryServeMediaRedirect` | Media redirects | Media file serving |
| 14 | `tryServeStaticAsset` | Published static HTML | Layer A: baked HTML files |
| 15 | `tryServeUpload` | `/uploads/*` | Uploaded media files |
| 16 | `tryServeAdminApp` | `/admin/*` | Admin SPA (React app) |
| 17 | `tryServePublicRoute` | `/*` | Public page rendering (Layer B/C) |
| 18 | `trySetupRedirect` | Setup wizard | Redirects to setup if not configured |
| 19 | `tryServeNotFoundPage` | Any unmatched | 404 page |

**Source:** `server/router.ts` lines 62-85.

### Route Handler Example

```typescript
// server/router.ts:109-112
function tryServeHealth(_req, _runtime, _url, pathname) {
  if (pathname !== '/health') return null
  return jsonResponse({ status: 'ok', ts: Date.now() })
}
```

The pattern is simple: check the path, return `null` if not matching, return a `Response` if matching.

## CMS Handlers

The CMS handler (`server/handlers/cms/index.ts`) dispatches to specific handlers:

```typescript
// server/handlers/cms/handleCmsRequest
// /admin/api/cms/* → specific handler based on path segment
```

### Key Handler Modules

| Module | Purpose | Source |
|--------|---------|--------|
| `server/handlers/cms/` | CMS CRUD (tables, rows, pages, components) | `server/handlers/cms/` |
| `server/handlers/auth/` | Authentication, sessions, access control | `server/handlers/auth/` |
| `server/handlers/cms/imageVariant*` | Image processing (sharp + blurhash) | `server/handlers/cms/` |
| `server/handlers/cms/loop` | Loop runtime execution | `server/handlers/cms/loop.ts` |
| `server/handlers/cms/hole` | Hole runtime (Layer C lazy fetch) | `server/handlers/cms/hole.ts` |
| `server/handlers/cms/moduleJs` | Module JS asset serving | `server/handlers/cms/moduleJs.ts` |
| `server/forms/handler.ts` | Public form submission handling | `server/forms/handler.ts` |

**Source:** `server/handlers/`, `server/forms/`.

## AI Handlers

```typescript
// server/ai/handlers/index.ts
// /admin/api/ai/* → chat streams, browser bridge, credentials, models
```

The AI namespace owns:
- Chat streams (POST `/admin/api/ai/chat/site`)
- Browser bridge (for AI agent browsing)
- Credentials CRUD (API key management)
- Conversation history
- Model discovery

Endpoints live under `/admin/api/` so the admin session cookie — scoped to `Path=/admin` — is carried automatically. The `requireCapability` gates (`ai.chat`, `ai.tools.write`, `ai.providers.manage`) ensure proper authorization.

**Source:** `server/ai/handlers/`.

## Repository Pattern

Repositories encapsulate database access. Each repository works against the `DbClient` interface, not a specific database engine:

```
server/repositories/
├── cms.ts              — CMS table/row operations
├── auth.ts             — User authentication
├── plugins.ts          — Plugin package management
├── setup.ts            — Initial setup state
├── media.ts            — Media file management
├── runtimeAsset.ts     — Runtime asset CRUD
└── ...
```

### Example Repository Pattern

```typescript
// Repositories use template-literal SQL via DbClient
// They are dialect-naive — the DbClient handles $1 vs ? differences
```

**Source:** `server/repositories/`.

## Static File Serving

```typescript
// server/static.ts
// serveStaticFile, serveAdminApp, hardenUploadResponse
```

The static serving module handles:
- Admin SPA serving (`serveAdminApp`) — falls back to `index.html` for client-side routing
- Upload serving with hardened responses (`hardenUploadResponse`)
- General static file serving (`serveStaticFile`)

**Source:** `server/static.ts`.

## HTTP Response Helpers

```typescript
// server/http.ts — jsonResponse helper
// server/binary.ts — binaryResponse, toArrayBuffer
```

**Source:** `server/http.ts`, `server/binary.ts`.

## Next

Continue to [03-publisher](03-publisher.md) for the three-layer publishing pipeline: static baking, LRU caching, and lazy hole rendering.
