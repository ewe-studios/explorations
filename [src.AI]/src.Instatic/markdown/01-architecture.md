# Instatic Architecture

Instatic is a self-hosted CMS running as a single Bun process with two off-main-thread workers, a hand-written 19-route dispatcher, and dual-engine database support (Postgres or SQLite).

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/Instatic/`

## Boot Sequence

Source: `server/index.ts`

```typescript
await import('./richtextSanitizer')  // Side-effect: initializes sanitizer
const { handleServerRequest } = await import('./router')
const { activateInstalledServerPlugins } = await import('./plugins/runtime')
const { mediaStorageRegistry } = await import('@core/plugins/mediaStorageRegistry')

const config = readServerConfig()
configureTrustedProxyCidrs(config.trustedProxyCidrs)
configurePublicOrigins(config.publicOrigins)

const { db, migrations } = createDbClient(config.databaseUrl)
await runMigrations(db, migrations)
await syncSystemRoles(db)                    // Force-reset Owner capabilities
mediaStorageRegistry.configureLocalDisk({ uploadsDir: config.uploadsDir })
await activateInstalledServerPlugins(db, config.uploadsDir)
startConversationPurgeTick(db)               // Nightly AI conversation cleanup
```

Aha: The top-level `await import()` pattern is used to load modules sequentially rather than at the top of the file. This ensures side-effectful initialization (richtext sanitizer) happens before the server starts accepting requests. The `handleServerRequest` import is deferred until after migrations run — if migrations fail, the server never starts.

## Server Bootstrap

```typescript
Bun.serve({
  port: config.port,
  idleTimeout: 0,  // Disable 10s default — agent streams NDJSON for minutes

  async fetch(req: Request, server: Bun.Server) {
    const origin = req.headers.get('origin')
    const cors = corsHeaders(origin)

    // Stamp real client IP from socket (strips spoofed X-Forwarded-For)
    stampSocketIp(req, server.requestIP(req)?.address ?? null)

    if (req.method === 'OPTIONS') {
      return new Response(null, { status: 204, headers: cors })
    }

    try {
      const res = await handleServerRequest(req, { db, ... })
      for (const [k, v] of Object.entries(cors)) res.headers.set(k, v)
      return res
    } catch (err) {
      // Never echo err.message — could leak internal state
      return new Response('Internal server error', { status: 500 })
    }
  },
})
```

### idleTimeout: 0

Aha: The default Bun idle timeout is 10 seconds. The AI agent endpoint streams NDJSON for as long as Claude's tool loop is running — "thinking" gaps between tool calls regularly exceed 10s on multi-step builds. Without `idleTimeout: 0`, the streaming response would be killed mid-flight, leaving the bridge resolver hanging server-side.

## CORS — Allowlist-Only

```typescript
function corsHeaders(origin: string | null): Record<string, string> {
  if (!origin || !DEV_ORIGIN_ALLOWLIST.includes(origin)) return {}
  return {
    'Access-Control-Allow-Origin': origin,
    'Access-Control-Allow-Credentials': 'true',
    'Access-Control-Allow-Methods': 'GET, POST, PUT, PATCH, DELETE, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type',
    'Vary': 'Origin',
  }
}
```

Aha: In production, the admin shell is same-origin behind Caddy — no CORS headers needed. In dev, only explicitly allowlisted origins get CORS. If an origin isn't on the list, the response gets empty headers — the browser blocks cross-origin reads naturally. This avoids echoing arbitrary origins (a classic broken-CORS bug shape).

## The 19-Route Dispatcher

Source: `server/router.ts`

```typescript
type RouteHandler = (
  req: Request,
  runtime: ServerRuntime,
  url: URL,
  pathname: string,
) => Promise<Response | null> | Response | null

const routes: readonly RouteHandler[] = [
  tryServeHealth,                    // 1. /health
  tryServeAi,                        // 2. /admin/api/ai/*
  tryServeCmsApi,                    // 3. /admin/api/*
  tryServeLoopRuntimeAsset,          // 4. /_instatic/runtime/loop/*
  tryServeLoop,                      // 5. Loop page rendering
  tryServeHoleRuntimeAsset,          // 6. /_instatic/runtime/hole/*
  tryServeHole,                      // 7. Lazy-fetch hole pages
  tryServeModuleJsAsset,             // 8. /_instatic/runtime/module/*.js
  tryServePublicForm,                // 9. Public form submissions
  tryServeRuntimeAsset,              // 10. /_instatic/runtime/* (JS/CSS bundles)
  tryServeRuntimePackageNamespace,   // 11. /_instatic/runtime/cache/*
  tryServeSiteCssNamespace,          // 12. /_instatic/css/*
  tryServeMediaRedirect,             // 13. Media file redirects
  tryServeStaticAsset,               // 14. Static baked pages
  tryServeUpload,                    // 15. /_instatic/uploads/*
  tryServeAdminApp,                  // 16. /admin/* (React SPA)
  tryServePublicRoute,               // 17. Public site routes
  trySetupRedirect,                  // 18. Redirect to /setup if no site
  tryServeNotFoundPage,              // 19. 404 page
]

export async function handleServerRequest(req: Request, runtime: ServerRuntime): Promise<Response> {
  const url = new URL(req.url)
  const { pathname } = url

  for (const route of routes) {
    const response = await route(req, runtime, url, pathname)
    if (response) return response
  }

  return jsonResponse({ error: 'Not found' }, { status: 404 })
}
```

Aha: Routes return `Response | null`. `null` means "I don't own this request, try the next route." Prefix-namespaced handlers (e.g., `/_instatic/css/`) absorb their entire namespace and emit a 404 themselves rather than falling through — so unknown paths under a known prefix can't accidentally match a later route. This is a "longest prefix wins" pattern implemented via explicit ownership.

### Route Contract

```typescript
// Each handler checks its own method/path:
function tryServeHealth(_req, _runtime, _url, pathname): Response | null {
  if (pathname !== '/health') return null
  return jsonResponse({ status: 'ok', ts: Date.now() })
}
```

No framework routing — just a `for` loop over functions. Adding a new endpoint is a one-line edit to the `routes` array plus a focused `tryServeX` function.

## Database Layer

Source: `server/db/`

```typescript
interface ServerConfig {
  port: number
  databaseUrl: string          // postgres://... or file:sqlite.db
  uploadsDir: string
  staticDir: string
  trustedProxyCidrs: string[]
  publicOrigins: string[]
}

function createDbClient(databaseUrl: string): { db: DbClient; migrations: Migration[] }
```

The same codebase runs against Postgres or SQLite — selected entirely by `DATABASE_URL`.

## Public Origin Resolution

Source: `server/config.ts`

```typescript
export function resolvePublicOrigins(env): string[] {
  // Priority 1: Explicit PUBLIC_ORIGIN env var
  const explicit = readCsvList(env.PUBLIC_ORIGIN)
  if (explicit.length > 0) return normalizeOrigins(explicit)

  // Priority 2: Platform auto-detection
  const derived: string[] = []
  if (env.RENDER_EXTERNAL_URL) derived.push(env.RENDER_EXTERNAL_URL)
  if (env.RAILWAY_PUBLIC_DOMAIN) derived.push(`https://${env.RAILWAY_PUBLIC_DOMAIN}`)
  return normalizeOrigins(derived)

  // Priority 3: [] → CSRF falls back to Host header
}
```

Normalization uses `new URL()` — no regex, no `as` casts. Scheme and host lowercased, path/query/fragment stripped.

## ServerRuntime Interface

```typescript
interface ServerRuntime {
  db: DbClient
  staticDir?: string
  uploadsDir?: string
  databaseUrl?: string  // Forwarded to CMS handlers for on-disk SQLite resolution
}
```

Passed to every route handler — the single source of server-wide state.

## Package Structure

```
Instatic/
├── server/                     # Bun server (hand-written router)
│   ├── index.ts               # Boot sequence, Bun.serve()
│   ├── router.ts              # 19-route dispatcher
│   ├── config.ts              # Server config, origin resolution
│   ├── binary.ts              # Binary response helpers
│   ├── static.ts              # Static file serving, uploads
│   ├── http.ts                # JSON response helpers
│   ├── time.ts                # Time utilities
│   ├── types.ts               # Server types
│   ├── ai/                    # AI runtime (providers, drivers, tools)
│   ├── auth/                  # Auth, security, CSRF
│   ├── db/                    # Database client, migrations
│   ├── handlers/              # CMS handlers
│   ├── plugins/               # Plugin runtime
│   ├── publish/               # Publishing pipeline
│   ├── repositories/          # Data access layer
│   ├── secrets/               # Secret management
│   └── util/                  # Server utilities
├── src/
│   ├── core/                  # 301 files: data models, schemas, utilities
│   ├── admin/                 # 752 files: React admin UI
│   ├── modules/               # 59 files: CMS module definitions
│   ├── ui/                    # 105 files: Shared UI components
│   └── styles/                # CSS
└── vendor/                    # Vendored dependencies
```

## Cross-References

- [Overview](00-overview.md) — What Instatic is
- [Server](02-server.md) — Hand-written router, handlers, repositories
- [Publisher](03-publisher.md) — Three-layer publishing pipeline

[Next → Server](02-server.md)
