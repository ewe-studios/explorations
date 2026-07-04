# Fetch Router: Router, Middleware, Route Maps, and Resources

`@remix-run/fetch-router` is the routing engine: a `fetch()`-based router with middleware chains, typed route maps, CRUD resources, and form actions.

Source: `packages/fetch-router/src/` — 12 source files.

## createRouter — Full Implementation

Source: `lib/router.ts`

```typescript
export function createRouter(options?: RouterOptions): Router {
  let defaultHandler = options?.defaultHandler ?? noMatchHandler
  let matcher = options?.matcher ?? new RegExpMatcher<MatchData>()
  let middleware = options?.middleware
```

### Default 404 Handler

```typescript
function noMatchHandler({ url }: RequestContext): Response {
  return new Response(`Not Found: ${url.pathname}`, { status: 404 })
}
```

Returns `Not Found: /path` with 404 status.

### dispatch() — Route Resolution

```typescript
async function dispatch(context: RequestContext): Promise<Response> {
  for (let match of matcher.matchAll(context.url)) {
    let { handler, method, middleware } = match.data

    if (method !== context.method && method !== 'ANY') {
      continue  // Method doesn't match, try next
    }

    context.params = match.params
    context.url = match.url

    if (middleware) {
      return runMiddleware(middleware, context, handler)
    }

    return raceRequestAbort(Promise.resolve(handler(context)), context.request)
  }

  return raceRequestAbort(Promise.resolve(defaultHandler(context)), context.request)
}
```

**Key behaviors:**

1. **Iterates ALL matches** — not just the first. A POST to `/users/42` will iterate through GET `/users/:id`, POST `/users/:id`, etc. until the method matches.
2. **Mutates context** — `context.params` and `context.url` are set from the matched route.
3. **Per-route middleware** — Each route can have its own middleware chain, separate from global middleware.
4. **Abort-wrapped** — Every handler execution is wrapped with `raceRequestAbort`.

### fetch() — Entry Point

```typescript
async fetch(input: string | URL | Request, init?: RequestInit): Promise<Response> {
  let request = new Request(input, init)

  if (request.signal.aborted) {
    throw request.signal.reason
  }

  let context = new RequestContext(request)
  let response = middleware
    ? await runMiddleware(middleware, context, dispatch)
    : await dispatch(context)

  return response
}
```

If global middleware exists, `dispatch` is passed as the final handler to `runMiddleware`.

### mapRoute() — Complex Route Mapping

```typescript
function mapRoute(routeArg: any, handlerArg: any): void {
  if (typeof routeArg === 'string' || routeArg instanceof RoutePattern) {
    // map(pattern, handler) → register as ANY method
    addRoute('ANY', routeArg, handlerArg)
  } else if (routeArg instanceof Route) {
    // map(route, handler) → use route's method and pattern
    addRoute(routeArg.method, routeArg.pattern, handlerArg)
  } else if (!hasHandlers(handlerArg)) {
    // map(routes, handlers) → recurse through route map
    let handlers = handlerArg
    for (let key in routeArg) {
      let route = routeArg[key]
      let handler = handlers[key]

      if (route instanceof Route) {
        addRoute(route.method, route.pattern, handler)
      } else {
        mapRoute(route, handler)  // Nested route map
      }
    }
  } else {
    // map(routes, { middleware?, handlers }) → with per-route middleware
    let mapMiddleware = handlerArg.middleware
    let handlers = handlerArg.handlers
    for (let key in routeArg) {
      let route = routeArg[key]
      let handler = (handlers as any)[key]

      if (route instanceof Route) {
        let routeMiddleware =
          mapMiddleware && handler.middleware
            ? mapMiddleware.concat(handler.middleware)
            : mapMiddleware || handler.middleware

        if (hasHandler(handler)) {
          addRoute(route.method, route.pattern, {
            middleware: routeMiddleware,
            handler: handler.handler,
          })
        } else {
          addRoute(route.method, route.pattern, { middleware: routeMiddleware, handler })
        }
      } else if (hasHandlers(handler)) {
        // Nested: { middleware?, handlers: { sub: { middleware?, handlers } } }
        let routeMiddleware =
          mapMiddleware && handler.middleware
            ? mapMiddleware.concat(handler.middleware)
            : mapMiddleware || handler.middleware

        mapRoute(route, { middleware: routeMiddleware, handlers: handler.handlers })
      } else {
        mapRoute(route, { middleware: mapMiddleware, handlers: handler })
      }
    }
  }
}
```

**Four calling patterns:**

1. `map('/pattern', handler)` — Simple pattern + handler
2. `map(route, handler)` — Route object with method + pattern
3. `map(routes, handlers)` — Route map + handler map (recursive)
4. `map(routes, { middleware?, handlers })` — With per-route and per-map middleware

Aha: When both map-level and route-level middleware exist, they're **concatenated** (map middleware runs first, then route middleware). This means you can set auth at the map level and per-route validation at the route level.

## Route Class

Source: `lib/route-map.ts`

```typescript
export class Route<method extends RequestMethod | 'ANY' = RequestMethod | 'ANY', pattern extends string = string> {
  readonly method: method | 'ANY'
  readonly pattern: RoutePattern<pattern>

  constructor(method: method | 'ANY', pattern: pattern | RoutePattern<pattern>) {
    this.method = method
    this.pattern = typeof pattern === 'string' ? new RoutePattern(pattern) : pattern
  }

  href(...args: HrefBuilderArgs<pattern>): string {
    return this.pattern.href(...args)
  }

  match(url: string | URL): RouteMatch<pattern> | null {
    return this.pattern.match(url)
  }
}
```

A thin wrapper around `RoutePattern` that adds the HTTP method.

## createRoutes — Type-Safe Route Maps

```typescript
export function createRoutes<const defs extends RouteDefs>(defs: defs): BuildRouteMap<'/', defs>
export function createRoutes<base extends string, const defs extends RouteDefs>(
  base: base | RoutePattern<base>,
  defs: defs,
): BuildRouteMap<base, defs>
export function createRoutes(baseOrDefs: any, defs?: RouteDefs): RouteMap {
  return typeof baseOrDefs === 'string' || baseOrDefs instanceof RoutePattern
    ? buildRouteMap(
        typeof baseOrDefs === 'string' ? new RoutePattern(baseOrDefs) : baseOrDefs,
        defs!,
      )
    : buildRouteMap(new RoutePattern('/'), baseOrDefs)
}
```

### buildRouteMap() — Runtime Construction

```typescript
function buildRouteMap<base extends string, defs extends RouteDefs>(
  base: RoutePattern<base>,
  defs: defs,
): BuildRouteMap<base, defs> {
  let routes: any = {}

  for (let key in defs) {
    let def = defs[key]

    if (def instanceof Route) {
      routes[key] = new Route(def.method, base.join(def.pattern))
    } else if (typeof def === 'string' || def instanceof RoutePattern) {
      routes[key] = new Route('ANY', base.join(def))
    } else if (typeof def === 'object' && def != null && 'pattern' in def) {
      routes[key] = new Route((def as any).method ?? 'ANY', base.join((def as any).pattern))
    } else {
      routes[key] = buildRouteMap(base, def as any)  // Nested route map
    }
  }

  return routes
}
```

Each definition becomes a `Route` with the base pattern joined. Nested objects become nested route maps.

### Example

```typescript
const routes = createRoutes('/api', {
  users: {
    index: { method: 'GET', pattern: '/' },
    show: { method: 'GET', pattern: '/:id' },
    create: { method: 'POST', pattern: '/' },
  },
  health: '/health',
})

// Produces:
// {
//   users: {
//     index: Route<'GET', '/api'>,
//     show: Route<'GET', '/api/:id'>,
//     create: Route<'POST', '/api'>,
//   },
//   health: Route<'ANY', '/api/health'>,
// }
```

## Middleware System

Source: `lib/middleware.ts`

### Middleware Interface

```typescript
export interface Middleware<method extends RequestMethod | 'ANY', params> {
  (
    context: RequestContext<method, params>,
    next: NextFunction,
  ): Response | undefined | void | Promise<Response | undefined | void>
}

export type NextFunction = () => Promise<Response>
```

### runMiddleware() — The Middleware Chain

```typescript
export function runMiddleware(middleware, context, handler): Promise<Response> {
  let index = -1

  let dispatch = async (i: number): Promise<Response> => {
    if (i <= index) throw new Error('next() called multiple times')
    index = i

    if (context.request.signal.aborted) {
      throw context.request.signal.reason
    }

    let fn = middleware[i]
    if (!fn) {
      return await raceRequestAbort(Promise.resolve(handler(context)), context.request)
    }

    let nextPromise: Promise<Response> | undefined
    let next: NextFunction = () => {
      nextPromise = dispatch(i + 1)
      return nextPromise
    }

    let response = await raceRequestAbort(Promise.resolve(fn(context, next)), context.request)

    // Short-circuit: middleware returned a Response
    if (response instanceof Response) {
      return response
    }

    // Middleware called next() — use downstream response
    if (nextPromise != null) {
      return nextPromise
    }

    // Middleware didn't call next() — auto-dispatch downstream
    return next()
  }

  return dispatch(0)
}
```

**Three control flows:**

| Behavior | Code | Result |
|----------|------|--------|
| Short-circuit | `return new Response('blocked')` | Returns immediately, skips remaining middleware |
| Pass through | `return next()` | Waits for downstream, returns its response |
| Auto-dispatch | `// no return, no next()` | Downstream runs automatically after middleware returns |

Aha: If a middleware returns `undefined` without calling `next()`, the chain auto-dispatches to the next middleware or handler. This means you don't need to call `next()` explicitly — just don't return a `Response`. This is unlike Express where forgetting `next()` hangs the request.

### Double-Next Guard

```typescript
if (i <= index) throw new Error('next() called multiple times')
index = i
```

The `index` counter tracks which middleware dispatched. If `dispatch(i)` is called with `i <= index`, it means `next()` was called more than once.

## RequestContext

Source: `lib/request-context.ts`

```typescript
export class RequestContext<method extends RequestMethod | 'ANY', params extends Record<string, any>> {
  headers: SuperHeaders
  method: RequestMethod
  params: params
  request: Request
  storage: AppStorage
  url: URL
  #formData?: FormData
  #session?: Session

  constructor(request: Request) {
    this.headers = new SuperHeaders(request.headers)
    this.method = request.method.toUpperCase() as RequestMethod
    this.params = {} as params
    this.request = request
    this.storage = new AppStorage()
    this.url = new URL(request.url)
  }
```

### formData Getter

```typescript
get formData(): method extends RequestBodyMethod ? FormData : FormData | undefined {
  if (this.#formData == null && RequestBodyMethods.includes(this.method as RequestBodyMethod)) {
    this.#formData = new FormData()
  }
  return this.#formData as any
}
```

Lazy initialization: `FormData` is only created for methods that can have bodies (POST, PUT, PATCH).

### files Getter

```typescript
get files(): Map<string, File> {
  let files: Map<string, File> = new Map()
  if (this.#formData != null) {
    for (let [key, value] of this.#formData.entries()) {
      if (value instanceof File) files.set(key, value)
    }
  }
  return files
}
```

Extracts uploaded files from `FormData`.

### session Getter

```typescript
get session(): Session {
  if (this.#session == null) {
    console.warn("Session isn't started yet...")
    this.#session = createSession()
  }
  return this.#session
}
```

Auto-creates a session with a warning. The `session()` middleware should be used instead for proper session management.

## Resources — CRUD Route Generation

Source: `lib/resource.ts`

```typescript
export const ResourceMethods = ['new', 'show', 'create', 'edit', 'update', 'destroy'] as const

export function createResource<base extends string, const options extends ResourceOptions>(
  base: base | RoutePattern<base>,
  options?: options,
): BuildResourceMap<base, options> {
  if (options?.only && options?.exclude) {
    throw new Error('Cannot specify both "only" and "exclude" options')
  }

  let only: readonly ResourceMethod[]
  if (options?.only) {
    only = options.only
  } else if (options?.exclude) {
    only = ResourceMethods.filter((m) => !options.exclude!.includes(m))
  } else {
    only = ResourceMethods
  }

  let routes: any = {}
  if (only.includes('new')) routes[options?.names?.new ?? 'new'] = { method: 'GET', pattern: '/new' }
  if (only.includes('show')) routes[options?.names?.show ?? 'show'] = { method: 'GET', pattern: '/' }
  if (only.includes('create')) routes[options?.names?.create ?? 'create'] = { method: 'POST', pattern: '/' }
  if (only.includes('edit')) routes[options?.names?.edit ?? 'edit'] = { method: 'GET', pattern: '/edit' }
  if (only.includes('update')) routes[options?.names?.update ?? 'update'] = { method: 'PUT', pattern: '/' }
  if (only.includes('destroy')) routes[options?.names?.destroy ?? 'destroy'] = { method: 'DELETE', pattern: '/' }

  return createRoutes(base, routes) as BuildResourceMap<base, options>
}
```

### CRUD Mapping

| Method | HTTP | Pattern | REST Action |
|--------|------|---------|-------------|
| `show` | GET | `/` | Read resource |
| `new` | GET | `/new` | Show creation form |
| `create` | POST | `/` | Create resource |
| `edit` | GET | `/edit` | Show edit form |
| `update` | PUT | `/` | Update resource |
| `destroy` | DELETE | `/` | Delete resource |

### Filtering

```typescript
// Only show and create (no edit/destroy)
createResource('/posts', { only: ['show', 'create'] })

// All except destroy
createResource('/posts', { exclude: ['destroy'] })

// Custom names
createResource('/posts', { names: { show: 'read', create: 'write' } })
```

## Form Actions

Source: `lib/form-action.ts`

```typescript
export function createFormAction<pattern, options>(
  pattern: pattern | RoutePattern<pattern>,
  options?: options,
): BuildFormActionMap<pattern, options> {
  let formMethod = options?.formMethod ?? 'POST'
  let indexName = options?.names?.index ?? 'index'
  let actionName = options?.names?.action ?? 'action'

  return createRoutes(pattern, {
    [indexName]: { method: 'GET', pattern: '/' },
    [actionName]: { method: formMethod, pattern: '/' },
  })
}
```

Creates two routes at the same pattern — one GET (show form), one POST (handle submission).

## AppStorage

Source: `lib/app-storage.ts`

```typescript
export class AppStorage {
  #map: Map<StorageKey<any>, StorageValue<any>> = new Map()

  has<key extends StorageKey<any>>(key: key): boolean {
    return this.#map.has(key)
  }

  get<key extends StorageKey<any>>(key: key): StorageValue<key> {
    if (!this.#map.has(key)) {
      if (key.defaultValue === undefined) {
        throw new Error(`Missing default value in storage for key ${key}`)
      }
      return key.defaultValue
    }
    return this.#map.get(key) as StorageValue<key>
  }

  set<key extends StorageKey<any>>(key: key, value: StorageValue<key>): void {
    this.#map.set(key, value)
  }
}

export function createStorageKey<T>(defaultValue?: T): StorageKey<T> {
  return { defaultValue }
}
```

Type-safe key-value storage. Keys carry their default values, so `get()` never returns `undefined` unless no default was set.

## Request Methods

Source: `lib/request-methods.ts`

```typescript
export const RequestMethods = ['GET', 'HEAD', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS']
export const RequestBodyMethods = ['POST', 'PUT', 'PATCH']
```

`RequestBodyMethods` determines which methods can have `formData`.

## raceRequestAbort

Source: `lib/request-abort.ts`

```typescript
export async function raceRequestAbort<T>(promise: Promise<T>, request: Request): Promise<T> {
  let signal = request.signal

  if (signal.aborted) {
    throw signal.reason
  }

  return new Promise<T>((resolve, reject) => {
    let onAbort = () => reject(signal.reason)
    signal.addEventListener('abort', onAbort, { once: true })

    promise.then(
      (value) => {
        signal.removeEventListener('abort', onAbort)
        resolve(value)
      },
      (error) => {
        signal.removeEventListener('abort', onAbort)
        reject(error)
      },
    )
  })
}
```

Wraps every promise with abort signal checking. If the request is aborted, the promise rejects with `signal.reason`.

## RouteHandlers — Type System

Source: `lib/route-handlers.ts`

```typescript
export type RouteHandlers<routes extends RouteMap> =
  | RouteHandlersWithMiddleware<routes>
  | RouteHandlersWithoutMiddleware<routes>
```

Two shapes:
1. **With middleware**: `{ middleware?: Middleware[], handlers: { home: handler, ... } }`
2. **Without middleware**: `{ home: handler, ... }`

The `hasHandlers()` type guard distinguishes between them at runtime.

## Type Utilities

Source: `lib/type-utils.ts`

```typescript
export type Assert<T extends true> = T
export type IsEqual<A, B> =
  (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false
export type Simplify<T> = { [K in keyof T]: T[K] } & {}
```

- **`Assert`**: Compile-time assertion — `Assert<true>` passes, `Assert<false>` fails.
- **`IsEqual`**: Type equality check using distributive conditional types.
- **`Simplify`**: Flattens intersection types into a single object type.

## Package Structure

```
fetch-router/src/
├── index.ts                  # Public API
├── lib/
│   ├── router.ts             # createRouter, dispatch, mapRoute
│   ├── route-map.ts          # Route class, createRoutes, buildRouteMap
│   ├── middleware.ts         # Middleware interface, runMiddleware
│   ├── request-context.ts    # RequestContext class
│   ├── request-handler.ts    # RequestHandler type
│   ├── request-methods.ts    # HTTP method constants
│   ├── request-abort.ts      # raceRequestAbort
│   ├── route-handlers.ts     # RouteHandlers type system
│   ├── resource.ts           # createResource (CRUD routes)
│   ├── form-action.ts        # createFormAction (GET+POST pairs)
│   ├── app-storage.ts        # AppStorage + createStorageKey
│   └── type-utils.ts         # Assert, IsEqual, Simplify
```

## Cross-References

- [Overview](00-overview.md) — What Remix is
- [Route Pattern](03-architecture.md) — URL pattern matching
- [Session](04-session.md) — Session management
- [Headers](06-headers.md) — HTTP headers
- [Node Fetch Server](09-node-fetch-server.md) — HTTP server

[Next → Node Fetch Server](09-node-fetch-server.md)
