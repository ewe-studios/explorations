# Executor Applications — Deep Dive Exploration

**Package:** `@executor/apps`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/apps`  
**Total Applications:** 5 apps  
**Total Files:** ~50 files  

---

## 1. Module Overview

The Applications package provides **end-user applications** for the Executor system:

- **cli** — Command-line interface for code execution
- **server** — Standalone HTTP server with API + MCP
- **web** — React web application (SPA)
- **desktop** — Electron desktop wrapper
- **marketing** — Marketing/landing page site

### Key Responsibilities

1. **CLI** — Terminal-based code execution and management
2. **Server** — Backend API server with plugin integration
3. **Web** — Browser-based UI for source/secret management
4. **Desktop** — Native desktop app with bundled server
5. **Marketing** — Public website and documentation

---

## 2. File Inventory

### cli (7 files)

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `bin/executor.ts` | — | CLI entry point |
| 2 | `src/main.ts` | 419 | CLI commands and server management |
| 3 | `src/build.ts` | — | Build script |
| 4 | `src/release.ts` | — | Release script |
| 5 | `src/embedded-web-ui.gen.ts` | — | Generated embedded UI types |
| 6 | `src/embedded-web-ui.gen.d.ts` | — | Type definitions |

### server (16 files)

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/index.ts` | 5 | Public exports |
| 2 | `src/main.ts` | 129 | Server composition |
| 3 | `src/mcp.ts` | 123 | MCP request handler |
| 4 | `src/dev-backend.ts` | — | Dev mode backend |
| 5 | `src/services/executor.ts` | 206 | Executor service layer |
| 6 | `src/services/engine.ts` | — | Execution engine service |
| 7-15 | `src/handlers/*.ts` | — | API route handlers |

### web (15+ files)

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/main.tsx` | — | React entry point |
| 2 | `src/App.tsx` | 7 | Root App component |
| 3 | `src/router.tsx` | 94 | TanStack Router config |
| 4 | `src/shell.tsx` | 465 | App shell with sidebar |
| 5 | `src/pages/tools.tsx` | — | Tools list page |
| 6 | `src/pages/sources.tsx` | — | Sources list page |
| 7 | `src/pages/sources-add.tsx` | — | Add source page |
| 8 | `src/pages/source-detail.tsx` | — | Source detail page |
| 9 | `src/pages/secrets.tsx` | — | Secrets management page |
| 10 | `src/components/tool-tree.tsx` | — | Tool tree component |
| 11 | `src/components/tool-detail.tsx` | — | Tool detail component |
| 12 | `src/components/mcp-install-card.tsx` | — | MCP install card |

### desktop (3 files)

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/main.ts` | 773 | Electron main process |
| 2 | `src/preload.ts` | — | Electron preload script |
| 3 | `package.json` | — | Electron config |

### marketing (3 files)

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/pages/api/detect.ts` | — | Source detection API |
| 2 | `src/pages/index.tsx` | — | Landing page |
| 3 | `src/pages/_app.tsx` | — | Next.js app wrapper |

---

## 3. Key Exports

### Server Package

```typescript
// index.ts
export { createApiHandler, createServerHandlers, type ApiHandler, type ServerHandlers, ApiLayer } from "./main";
export { createServerHandlersWithExecutor } from "./main";
export { ExecutorServiceLayer, createServerExecutorHandle, disposeExecutor, getExecutor, reloadExecutor } from "./services/executor";
export { createMcpRequestHandler, runMcpStdioServer, type McpRequestHandler } from "./mcp";
```

### CLI Commands

```typescript
// main.ts
executor call [code] --file <path> --stdin --base-url <url>
executor resume --execution-id <id> --action <accept|decline|cancel> --content <json>
executor web --port <port>
executor mcp
```

### Web App Routes

```typescript
// router.tsx
/ → SourcesPage
/tools → ToolsPage
/sources/add/:pluginKey → SourcesAddPage
/sources/:namespace → SourceDetailPage
/secrets → SecretsPage
```

---

## 4. Line-by-Line Analysis

### Server Composition (`server/main.ts:34-57`)

```typescript
const ExecutorApiWithPlugins = addGroup(OpenApiGroup)
  .add(McpGroup)
  .add(GoogleDiscoveryGroup)
  .add(OnePasswordGroup)
  .add(GraphqlGroup);

const ApiBase = HttpApiBuilder.api(ExecutorApiWithPlugins).pipe(
  Layer.provide([
    ToolsHandlers,
    SourcesHandlers,
    SecretsHandlers,
    ExecutionsHandlers,
    ScopeHandlers,
    OpenApiHandlersLive,
    McpSourceHandlersLive,
    GoogleDiscoveryHandlersLive,
    OnePasswordHandlersLive,
    GraphqlHandlersLive,
  ]),
);
```

**Key patterns:**
1. **Plugin groups** — Each plugin adds its API endpoints
2. **Handler layers** — Effect Layer for each handler group
3. **Composition** — All handlers provided to API base

### Shared Server Handler (`server/main.ts:80-100`)

```typescript
const createApiHandlerWithExecutor = (executor, engine) =>
  HttpApiBuilder.toWebHandler(
    HttpApiSwagger.layer().pipe(
      Layer.provideMerge(HttpApiBuilder.middlewareOpenApi()),
      Layer.provideMerge(ApiBase),
      Layer.provideMerge(Layer.succeed(ExecutorService, executor)),
      Layer.provideMerge(Layer.succeed(ExecutionEngineService, engine)),
      Layer.provideMerge(HttpServer.layerContext),
    ),
    { middleware: HttpMiddleware.logger },
  );

export const createServerHandlers = async (): Promise<ServerHandlers> =>
  createServerHandlersWithExecutor(await getExecutor());
```

**Key patterns:**
1. **Web standard handler** — Works with any HTTP server
2. **Layer merging** — Provide executor and engine instances
3. **Shared instance** — Same executor for API + MCP

### MCP Request Handler (`server/mcp.ts:22-90`)

```typescript
export const createMcpRequestHandler = (config): McpRequestHandler => {
  const transports = new Map<string, WebStandardStreamableHTTPServerTransport>();
  const servers = new Map<string, McpServer>();

  return {
    handleRequest: async (request) => {
      const sessionId = request.headers.get("mcp-session-id");

      if (sessionId) {
        const transport = transports.get(sessionId);
        if (!transport) return jsonError(404, -32001, "Session not found");
        return transport.handleRequest(request);
      }

      // Create new session
      const transport = new WebStandardStreamableHTTPServerTransport({
        sessionIdGenerator: () => crypto.randomUUID(),
        onsessioninitialized: (sid) => {
          transports.set(sid, transport);
        },
      });

      const server = await createExecutorMcpServer(config);
      await server.connect(transport);
      return transport.handleRequest(request);
    },
  };
};
```

**Key patterns:**
1. **Session management** — Map of session IDs to transports
2. **Streamable HTTP** — MCP over HTTP with session persistence
3. **Lazy creation** — New server per session

### Executor Service Layer (`server/services/executor.ts:89-142`)

```typescript
const ExecutorLayer = Layer.effect(
  ExecutorService,
  Effect.gen(function* () {
    const sql = yield* SqlClient.SqlClient;
    yield* migrate.pipe(Effect.catchAll((e) => Effect.die(e)));

    const cwd = process.env.EXECUTOR_SCOPE_DIR || process.cwd();
    const kv = makeSqliteKv(sql);
    const config = makeKvConfig(kv, { cwd });
    const scopedKv = makeScopedKv(kv, cwd);
    const configPath = join(cwd, "executor.jsonc");
    const fsLayer = NodeFileSystem.layer;

    return yield* createExecutor({
      ...config,
      plugins: [
        openApiPlugin({ operationStore: withConfigFile.openapi(...) }),
        mcpPlugin({ bindingStore: withConfigFile.mcp(...) }),
        googleDiscoveryPlugin({ bindingStore: ... }),
        graphqlPlugin({ operationStore: withConfigFile.graphql(...) }),
        keychainPlugin(),
        fileSecretsPlugin(),
        onepasswordPlugin({ kv: scopeKv(scopedKv, "onepassword") }),
      ] as const,
    });
  }),
).pipe(Layer.provide(SqliteClient.layer({ filename: DB_PATH })));
```

**Key patterns:**
1. **SQLite KV** — Bun SQLite for persistence
2. **Config file sync** — `withConfigFile` wrappers
3. **Plugin composition** — All plugins initialized together
4. **Data directory** — `~/.executor/data.db`

### Managed Runtime (`server/services/executor.ts:148-205`)

```typescript
export const createServerExecutorHandle = async (): Promise<ServerExecutorHandle> => {
  const runtime = ManagedRuntime.make(ExecutorLayer);
  const executor = await runtime.runPromise(ExecutorService);
  return {
    executor,
    dispose: async () => {
      await runtime.dispose();
    },
  };
};

let sharedHandlePromise: Promise<ServerExecutorHandle> | null = null;

export const getExecutor = (): Promise<ServerExecutor> =>
  loadSharedHandle().then((handle) => handle.executor);

export const ExecutorServiceLayer = Layer.effect(
  ExecutorService,
  Effect.promise(() => getExecutor()),
);
```

**Key patterns:**
1. **Managed runtime** — Effect runtime for lifecycle management
2. **Singleton pattern** — Shared handle for production
3. **Scoped disposal** — Clean disposal for dev HMR

### CLI Foreground Session (`cli/main.ts:202-255`)

```typescript
const runForegroundSession = (input: { kind: "web" | "mcp"; port: number }) =>
  Effect.gen(function* () {
    const handlers = yield* Effect.promise(() => createServerHandlers());

    const fetch = async (request: Request): Promise<Response> => {
      if (!isAllowedHost(request)) {
        return new Response("Forbidden", { status: 403 });
      }

      const url = new URL(request.url);

      if (url.pathname.startsWith("/mcp")) {
        return handlers.mcp.handleRequest(request);
      }

      if (url.pathname.startsWith("/v1/") || url.pathname.startsWith("/docs")) {
        return handlers.api.handler(request);
      }

      const staticResponse = await serveStatic(url.pathname);
      if (staticResponse) return staticResponse;

      return new Response("Not Found", { status: 404 });
    };

    const serverV4 = Bun.serve({ port: input.port, hostname: "127.0.0.1", fetch });
    const serverV6 = Bun.serve({ port: input.port, hostname: "::1", fetch });

    console.log(renderSessionSummary(input.kind, baseUrl));
    yield* waitForShutdownSignal();

    serverV4.stop(true);
    serverV6?.stop(true);
  });
```

**Key patterns:**
1. **Single server** — API + MCP + Static files in one Bun.serve()
2. **Host validation** — DNS rebinding protection
3. **Dual-stack** — IPv4 and IPv6 loopback

### CLI Static File Serving (`cli/main.ts:146-177`)

```typescript
const serveStatic = async (pathname: string): Promise<Response | null> => {
  const key = pathname.replace(/^\//, "");

  // Compiled binary: serve from embedded bunfs
  if (embeddedWebUI) {
    const match = embeddedWebUI[key] ?? embeddedWebUI["index.html"] ?? null;
    const file = Bun.file(match);
    return new Response(file, {
      headers: { "content-type": file.type || "application/octet-stream" },
    });
  }

  // Dev mode: serve from apps/web/dist on disk
  const filePath = resolve(WEB_DIST_DIR, key);
  if (!filePath.startsWith(WEB_DIST_DIR)) return null;

  const file = Bun.file(filePath);
  if (await file.exists()) {
    return new Response(file, {
      headers: { "content-type": file.type || "application/octet-stream" },
    });
  }

  // SPA fallback
  const index = Bun.file(resolve(WEB_DIST_DIR, "index.html"));
  if (await index.exists()) {
    return new Response(index, { headers: { "content-type": "text/html" } });
  }

  return null;
};
```

**Key patterns:**
1. **Embedded files** — `embedded-web-ui.gen` from build step
2. **Dev mode fallback** — Serve from disk in development
3. **SPA routing** — Fallback to index.html

### Desktop CLI Installation (`desktop/main.ts:48-112`)

```typescript
const installCli = (): void => {
  if (isDev) return;

  const sidecar = join(process.resourcesPath, binaryName);
  const installedVersion = getInstalledCliVersion();
  const appVersion = app.getVersion();

  // Check if upgrade needed
  if (installedVersion) {
    const parse = (v: string) => v.replace(/^v/, "").split(/[.-]/).map((s) => parseInt(s) || 0);
    const installed = parse(installedVersion);
    const bundled = parse(appVersion);
    for (let i = 0; i < Math.max(installed.length, bundled.length); i++) {
      if ((bundled[i] ?? 0) > (installed[i] ?? 0)) break; // Needs update
    }
    if (cmp >= 0) return; // Already up to date
  }

  // Copy binary to ~/.executor/bin
  mkdirSync(CLI_BIN_DIR, { recursive: true });
  copyFileSync(sidecar, CLI_BIN_PATH);
  chmodSync(CLI_BIN_PATH, 0o755);

  // Patch shell profiles
  const pathLine = `export PATH="${CLI_BIN_DIR}:$PATH"`;
  for (const profile of ["~/.zshrc", "~/.bashrc", "~/.bash_profile"]) {
    if (existsSync(profile) && !readFileSync(profile, "utf-8").includes(CLI_BIN_DIR)) {
      appendFileSync(profile, `\n${pathLine}\n`);
    }
  }
};
```

**Key patterns:**
1. **Sidecar binary** — Bundled via extraResources
2. **Version check** — Semantic version comparison
3. **Shell integration** — Auto-add to PATH

### Desktop Server Management (`desktop/main.ts:215-261`)

```typescript
const startServer = async (scopePath: string, port: number): Promise<void> => {
  await stopServer();

  currentScope = scopePath;
  currentPort = port;

  const server = resolveServerCommand();
  const args = [...server.args, "web", "--port", String(port)];
  const cwd = isDev ? resolve(__dirname, "../../..") : scopePath;

  serverProcess = spawn(server.command, args, {
    cwd,
    stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env, EXECUTOR_SCOPE_DIR: scopePath },
  });

  serverProcess.stdout?.on("data", (data) => {
    console.log(`[server] ${data.toString().trim()}`);
  });

  // Wait for server ready
  const deadline = Date.now() + SERVER_STARTUP_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (await isServerReady(port)) return;
    await new Promise((r) => setTimeout(r, 200));
  }

  throw new Error(`Server failed to start within ${SERVER_STARTUP_TIMEOUT_MS / 1000}s`);
};
```

**Key patterns:**
1. **Child process** — Spawn CLI as subprocess
2. **Scope env** — `EXECUTOR_SCOPE_DIR` environment
3. **Health check** — Poll `/docs` endpoint

### Web App Shell (`web/src/shell.tsx:345-464`)

```typescript
export function Shell() {
  const location = useLocation();
  const pathname = location.pathname;
  const scopeId = useScope();
  const refreshSources = useAtomRefresh(sourcesAtom(scopeId));
  const refreshTools = useAtomRefresh(toolsAtom(scopeId));
  const { latestVersion, updateAvailable } = useLatestVersion(VITE_APP_VERSION);
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);

  // Auto-refresh on HMR
  useEffect(() => {
    if (!import.meta.hot) return;
    const refreshBackendData = () => { refreshSources(); refreshTools(); };
    import.meta.hot.on("executor:backend-updated", refreshBackendData);
    return () => import.meta.hot?.off("executor:backend-updated", refreshBackendData);
  }, []);

  return (
    <div className="flex h-screen overflow-hidden">
      {/* Desktop sidebar */}
      <aside className="hidden w-52 md:flex">
        <SidebarContent pathname={pathname} updateAvailable={updateAvailable} />
      </aside>

      {/* Mobile sidebar overlay */}
      {mobileSidebarOpen && (
        <div className="fixed inset-0 z-50 flex md:hidden">
          <button className="absolute inset-0 bg-black/45" onClick={() => setMobileSidebarOpen(false)} />
          <div className="relative h-full w-[84vw] max-w-xs">
            <SidebarContent onNavigate={() => setMobileSidebarOpen(false)} />
          </div>
        </div>
      )}

      {/* Main content */}
      <main className="flex min-h-0 flex-1 flex-col">
        <Outlet />
      </main>
    </div>
  );
}
```

**Key patterns:**
1. **Responsive design** — Desktop sidebar + mobile overlay
2. **HMR integration** — Auto-refresh on backend changes
3. **Update detection** — Check npm for latest version

### Web Router (`web/src/router.tsx:18-87`)

```typescript
const rootRoute = createRootRoute({
  component: () => (
    <ExecutorProvider>
      <Shell />
    </ExecutorProvider>
  ),
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: SourcesPage,
});

const toolsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tools",
  component: ToolsPage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  toolsRoute,
  sourcesAddRoute,
  sourceDetailRoute,
  secretsRoute,
]);

export const router = createRouter({ routeTree });
```

**Key patterns:**
1. **TanStack Router** — Type-safe routing
2. **Root provider** — ExecutorProvider wraps all routes
3. **Nested routes** — Source detail under sources

### Version Update Detection (`web/src/shell.tsx:91-115`)

```typescript
function useLatestVersion(currentVersion: string) {
  const channel = currentVersion.includes("-beta.") ? "beta" : "latest";
  const [latestVersion, setLatestVersion] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetch("/v1/app/npm/dist-tags")
      .then((res) => res.json())
      .then((data) => {
        if (!cancelled) setLatestVersion(data[channel] ?? null);
      });
    return () => { cancelled = true; };
  }, [channel]);

  const updateAvailable =
    latestVersion !== null && compareVersions(currentVersion, latestVersion) === -1;

  return { latestVersion, updateAvailable, channel };
}
```

**Key patterns:**
1. **Dist tags** — Fetch from npm registry via proxy
2. **Channel detection** — Beta vs stable
3. **Semver comparison** — Major.minor.patch comparison

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Applications Architecture                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐         │
│  │   CLI            │  │   Server         │  │   Desktop        │         │
│  │   (cli/main.ts)  │  │   (server/)      │  │   (desktop/)     │         │
│  │                  │  │                  │  │                  │         │
│  │  - call command  │  │  - API handler   │  │  - Electron main │         │
│  │  - resume cmd    │  │  - MCP handler   │  │  - Server spawn  │         │
│  │  - web command   │  │  - Executor svc  │  │  - CLI install   │         │
│  │  - mcp command   │  │  - Plugin init   │  │  - Window mgmt   │         │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘         │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    @executor/server                                  │   │
│  │                                                                       │   │
│  │  createServerHandlers()                                              │   │
│  │    ├── ExecutorService (singleton)                                  │   │
│  │    │   └── ManagedRuntime.make(ExecutorLayer)                       │   │
│  │    ├── ApiHandler (HttpApiBuilder)                                  │   │
│  │    └── McpHandler (session-based)                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Web App (@executor/web)                           │   │
│  │                                                                       │   │
│  │  Shell                                                               │   │
│  │    ├── ExecutorProvider                                             │   │
│  │    │   └── ScopeProvider                                            │   │
│  │    ├── Sidebar (sources list, navigation)                           │   │
│  │    └── Outlet (page content)                                        │   │
│  │                                                                       │   │
│  │  Pages                                                               │   │
│  │    ├── SourcesPage (list + presets)                                 │   │
│  │    ├── SourcesAddPage (plugin add flow)                             │   │
│  │    ├── SourceDetailPage (edit/config)                               │   │
│  │    ├── ToolsPage (tool tree)                                        │   │
│  │    └── SecretsPage (secret management)                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### CLI Execution Flow

```
executor call "console.log('hello')"
    │
    ▼
┌─────────────────────────────┐
│  readCode()                 │
│  - Check positional arg     │
│  - Check --file             │
│  - Check stdin              │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  ensureServer(baseUrl)      │
│  - Check /docs reachable    │
│  - Start background if needed
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  client.executions.execute()│
│  - POST /v1/executions      │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  Display result             │
│  - Completed: show output   │
│  - Paused: show resume cmd  │
└─────────────────────────────┘
```

### Server Startup Flow

```
getExecutor()
    │
    ▼
┌─────────────────────────────┐
│  ManagedRuntime.make()      │
│  - Creates Effect runtime   │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  ExecutorLayer              │
│    ├── SqliteClient         │
│    ├── migrate()            │
│    ├── makeSqliteKv()       │
│    └── createExecutor()     │
│        └── plugins: [...]   │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  ServerExecutor instance    │
│  - All plugins initialized  │
│  - KV storage ready         │
│  - Config loaded            │
└─────────────────────────────┘
```

### MCP Session Flow

```
MCP Client POST /mcp
    │
    ▼
┌─────────────────────────────┐
│  No session-id header       │
│  Create new session         │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  WebStandardStreamableHTTP  │
│  - sessionIdGenerator()     │
│  - onsessioninitialized     │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  createExecutorMcpServer()  │
│  - Register tools           │
│  - Connect transport        │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  Store transport + server   │
│  in maps by sessionId       │
└───────────┬─────────────────┘
            │
            ▼
    Subsequent requests with
    session-id header → route to existing transport
```

### Desktop App Startup

```
app.whenReady()
    │
    ▼
┌─────────────────────────────┐
│  installCli()               │
│  - Copy sidecar to ~/.executor/bin
│  - Patch shell profiles     │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  loadSettings()             │
│  - recentScopes             │
│  - lastScope                │
└───────────┬─────────────────┘
            │
            ▼
    ┌───────┴───────┐
    │               │
  Has last       No last
  scope          scope
    │               │
    ▼               ▼
┌─────────┐   ┌─────────────┐
│ loadScope│   │ welcomeHTML │
│ → start  │   │ → select    │
│ server   │   │ folder      │
└────┬────┘   └─────────────┘
     │
     ▼
┌─────────────────────────────┐
│  startServer(scope, port)   │
│  - Spawn CLI subprocess     │
│  - Wait for /docs ready     │
│  - Load web UI              │
└─────────────────────────────┘
```

---

## 7. Key Patterns

### Effect Layer Composition

```typescript
const ExecutorLayer = Layer.effect(ExecutorService, Effect.gen(function* () {
  const sql = yield* SqlClient.SqlClient;
  const kv = makeSqliteKv(sql);
  return yield* createExecutor({ plugins: [...] });
})).pipe(Layer.provide(SqliteClient.layer({ filename: DB_PATH })));
```

**Benefits:**
1. **Dependency injection** — All dependencies provided via layers
2. **Resource management** — Automatic cleanup on dispose
3. **Testability** — Easy to swap implementations

### Managed Runtime Singleton

```typescript
let sharedHandlePromise: Promise<ServerExecutorHandle> | null = null;

export const getExecutor = (): Promise<ServerExecutor> =>
  loadSharedHandle().then((handle) => handle.executor);
```

**Benefits:**
1. **Singleton pattern** — One executor per process
2. **Lazy initialization** — Created on first use
3. **Dev HMR support** — Can be disposed and recreated

### Session-Based MCP

```typescript
const transports = new Map<string, WebStandardStreamableHTTPServerTransport>();
const servers = new Map<string, McpServer>();
```

**Benefits:**
1. **Stateless HTTP** — Each session has own transport
2. **Multiple clients** — Support concurrent MCP connections
3. **Clean cleanup** — Close transport and server on session end

### Host Validation for Security

```typescript
const ALLOWED_HOSTS = new Set(["localhost", "127.0.0.1", "[::1]", "::1"]);

const isAllowedHost = (request: Request): boolean => {
  const host = request.headers.get("host");
  if (!host) return true;
  const hostname = host.replace(/:\d+$/, "");
  return ALLOWED_HOSTS.has(hostname);
};
```

**Benefits:**
1. **DNS rebinding protection** — Only allow loopback hosts
2. **Port stripping** — Handle host:port format
3. **Fail-open for missing** — Allow if no host header

### Embedded Static Files

```typescript
// Build step bundles web dist into generated file
import embeddedWebUI from "./embedded-web-ui.gen";

const serveStatic = (pathname) => {
  const match = embeddedWebUI[key] ?? embeddedWebUI["index.html"];
  return new Response(Bun.file(match));
};
```

**Benefits:**
1. **Single binary** — Web UI embedded in CLI
2. **No external deps** — Works offline
3. **Fast startup** — No file I/O in prod

---

## 8. Integration Points

### CLI Dependencies

| Package | Purpose |
|---------|---------|
| `@effect/cli` | CLI framework |
| `@effect/platform` | HTTP client |
| `@executor/api` | API types |
| `@executor/server` | Server handlers |

### Server Dependencies

| Package | Purpose |
|---------|---------|
| `@effect/platform` | HTTP server, middleware |
| `@effect/sql-sqlite-bun` | SQLite database |
| `@executor/sdk` | Executor creation |
| `@executor/plugins/*` | All source plugins |
| `@executor/storage-file` | KV storage |
| `@executor/config` | Config file handling |

### Web Dependencies

| Package | Purpose |
|---------|---------|
| `@tanstack/react-router` | Routing |
| `@executor/react` | React client atoms |
| `@executor/ui` | UI components |
| `effect-atom` | Reactive state |

### Desktop Dependencies

| Package | Purpose |
|---------|---------|
| `electron` | Desktop framework |
| `@executor/cli` | Bundled sidecar |

---

## 9. Error Handling

### CLI Code Resolution

```typescript
const readCode = (input) =>
  Effect.gen(function* () {
    // Try positional arg
    if (code?.trim()) return code;
    // Try --file
    if (file?.trim()) {
      const contents = yield* Effect.tryPromise(() => Bun.file(file).text());
      if (contents.trim()) return contents;
    }
    // Try stdin
    if (!process.stdin.isTTY) {
      const chunks = [];
      for await (const chunk of process.stdin) chunks.push(chunk);
      return chunks.join("");
    }
    return yield* Effect.fail(new Error("No code provided"));
  });
```

**Strategy:** Fallback chain with clear error message.

### Server Startup Timeout

```typescript
const deadline = Date.now() + SERVER_STARTUP_TIMEOUT_MS;
while (Date.now() < deadline) {
  if (await isServerReady(port)) return;
  await new Promise((r) => setTimeout(r, 200));
}
throw new Error(`Server failed to start within ${timeout}s`);
```

**Strategy:** Poll with timeout, clear error on failure.

### MCP Session Not Found

```typescript
const sessionId = request.headers.get("mcp-session-id");
if (sessionId) {
  const transport = transports.get(sessionId);
  if (!transport) return jsonError(404, -32001, "Session not found");
  return transport.handleRequest(request);
}
```

**Strategy:** JSON-RPC error response for invalid session.

---

## 10. Testing Strategy

### CLI Tests

- **Command parsing** — Verify flags and arguments
- **Code resolution** — Test stdin, file, and positional
- **Server management** — Background start detection

### Server Tests

- **Handler tests** — Individual API endpoint tests
- **Integration tests** — Full request/response cycle
- **Plugin tests** — Plugin initialization verification

### Web Tests

- **Component tests** — React component rendering
- **Router tests** — Route matching and params
- **Atom tests** — Query/mutation behavior

---

## 11. Design Decisions

### Why Bun for CLI?

1. **Fast startup** — Bun's JIT for quick CLI execution
2. **Built-in APIs** — `Bun.file()`, `Bun.serve()`
3. **Single binary** — Easy distribution

### Why ManagedRuntime?

1. **Resource safety** — Automatic cleanup on dispose
2. **Layer composition** — Easy to swap implementations
3. **Effect integration** — Works seamlessly with Effect

### Why Session-Based MCP?

1. **HTTP statelessness** — Each client gets own session
2. **Scalability** — Multiple concurrent clients
3. **Clean isolation** — No cross-client state

### Why Embedded Web UI?

1. **Offline support** — No network needed for CLI
2. **Version sync** — UI version matches CLI version
3. **Simplified deploy** — Single artifact

### Why Dual-Stack Server?

```typescript
const serverV4 = Bun.serve({ hostname: "127.0.0.1" });
const serverV6 = Bun.serve({ hostname: "::1" });
```

1. **Cross-platform** — Windows may resolve localhost to ::1
2. **Fallback** — IPv4 if IPv6 fails
3. **Local only** — Both loopback addresses

---

## 12. Web App Implementation Details

### Sources Page (`apps/web/src/pages/sources.tsx`)

The sources page displays configured sources and preset cards for quick-add:

```typescript
export function SourcesPage() {
  const scopeId = useScope();
  const sources = useAtomValue(sourcesAtom(scopeId));
  const navigate = useNavigate();

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-semibold">Sources</h1>
        <Button onClick={() => navigate({ to: "/sources" })}>
          Add Source
        </Button>
      </div>

      {/* Presets Grid */}
      <div className="grid gap-3">
        <PresetCard preset={githubPreset} />
        <PresetCard preset={stripePreset} />
        <PresetCard preset={firecrawlPreset} />
      </div>

      {/* Sources List */}
      {Result.match(sources, {
        onSuccess: ({ value }) => (
          <div className="flex flex-col gap-2">
            {value.map((source) => (
              <SourceCard key={source.id} source={source} />
            ))}
          </div>
        ),
        onInitial: () => <Spinner />,
        onFailure: () => <EmptyState>No sources configured</EmptyState>,
      })}
    </div>
  );
}
```

### Source Detail Page (`apps/web/src/pages/source-detail.tsx`)

```typescript
export function SourceDetailPage({ namespace }: { namespace: string }) {
  const scopeId = useScope();
  const source = useAtomValue(sourceAtom(namespace, scopeId));
  const navigate = useNavigate();

  const refreshSource = useAtomSet(refreshSource);
  const removeSource = useAtomSet(removeSource);

  return Result.match(source, {
    onSuccess: ({ value }) => value ? (
      <div className="p-6">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-lg font-semibold">{value.name}</h1>
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => refreshSource({})}>
              Refresh
            </Button>
            <Button 
              variant="destructive" 
              onClick={() => {
                removeSource({}).then(() => 
                  navigate({ to: "/sources" })
                );
              }}
            >
              Remove
            </Button>
          </div>
        </div>
        
        {/* Source configuration */}
        <SourceConfig sourceId={namespace} />
        
        {/* Tools list */}
        <ToolsList sourceId={namespace} />
      </div>
    ) : (
      <EmptyState>Source not found</EmptyState>
    ),
    onInitial: () => <Spinner />,
    onFailure: (err) => <ErrorState error={err} />,
  });
}
```

### Sources Add Page (`apps/web/src/pages/sources-add.tsx`)

```typescript
export function SourcesAddPage({ 
  pluginKey, 
  url, 
  preset 
}: { 
  pluginKey: string;
  url?: string;
  preset?: string;
}) {
  const navigate = useNavigate();
  const plugin = usePlugin(pluginKey); // Get plugin from context
  
  return (
    <plugin.add
      onComplete={() => navigate({ to: "/sources" })}
      onCancel={() => navigate({ to: "/sources" })}
      initialUrl={url}
      initialPreset={preset}
    />
  );
}
```

**Key patterns:**
1. **Plugin delegation** — Add flow delegated to plugin component
2. **Initial values** — URL and preset passed from link
3. **Navigation callbacks** — onComplete/onCancel navigate back

---

## 13. CLI Command Parsing Details

### Code Resolution (`cli/main.ts:271-305`)

```typescript
const readCode = (input: {
  code: Option.Option<string>;
  file: Option.Option<string>;
  stdin: boolean;
}): Effect.Effect<string, Error> =>
  Effect.gen(function* () {
    // 1. Try positional argument
    const code = Option.getOrUndefined(input.code);
    if (code && code.trim().length > 0) return code;

    // 2. Try --file flag
    const file = Option.getOrUndefined(input.file);
    if (file && file.trim().length > 0) {
      const contents = yield* Effect.tryPromise({
        try: () => Bun.file(file).text(),
        catch: (e) => new Error(`Failed to read file: ${e}`),
      });
      if (contents.trim().length > 0) return contents;
    }

    // 3. Try stdin
    if (input.stdin || !process.stdin.isTTY) {
      const chunks: string[] = [];
      process.stdin.setEncoding("utf8");
      const contents = yield* Effect.tryPromise({
        try: async () => {
          for await (const chunk of process.stdin) {
            chunks.push(chunk as string);
          }
          return chunks.join("");
        },
        catch: (e) => new Error(`Failed to read stdin: ${e}`),
      });
      if (contents.trim().length > 0) return contents;
    }

    return yield* Effect.fail(
      new Error("No code provided. Pass code as an argument, --file, or pipe to stdin."),
    );
  });
```

**Key patterns:**
1. **Fallback chain** — Argument → File → Stdin
2. **Effect.tryPromise** — Error handling for file/stdin operations
3. **TTY detection** — Only read stdin if not a TTY (piped input)

### Call Command (`cli/main.ts:311-344`)

```typescript
const callCommand = Command.make(
  "call",
  {
    code: Args.text({ name: "code" }).pipe(Args.optional),
    file: Options.text("file").pipe(Options.optional),
    stdin: Options.boolean("stdin").pipe(Options.withDefault(false)),
    baseUrl: Options.text("base-url").pipe(Options.withDefault(DEFAULT_BASE_URL)),
  },
  ({ code, file, stdin, baseUrl }) =>
    Effect.gen(function* () {
      const resolvedCode = yield* readCode({ code, file, stdin });
      yield* ensureServer(baseUrl);

      const client = yield* makeApiClient(baseUrl);
      const result = yield* client.executions.execute({ 
        payload: { code: resolvedCode } 
      });

      if (result.status === "completed") {
        if (result.isError) {
          console.error(result.text);
          process.exitCode = 1;
        } else {
          console.log(result.text);
        }
      } else {
        // Paused execution
        console.log(result.text);
        const executionId = (result.structured as any)?.executionId;
        if (executionId) {
          console.log(
            `To resume:\n  ${cliPrefix} resume --execution-id ${executionId} --action accept`,
          );
        }
      }
    }),
).pipe(Command.withDescription("Execute code against the local executor"));
```

**Key patterns:**
1. **Effect CLI** — `@effect/cli` Command.make pattern
2. **Server auto-start** — `ensureServer(baseUrl)` starts background if needed
3. **Resume guidance** — Shows resume command when execution pauses

---

## 14. Desktop IPC Handling

### IPC Handlers (`desktop/main.ts:458-474`)

```typescript
const setupIPC = (): void => {
  ipcMain.handle("select-scope", async () => {
    await selectFolder();
    return currentScope;
  });

  ipcMain.handle("get-current-scope", () => currentScope);

  ipcMain.handle("get-recent-scopes", () => settings.recentScopes);

  ipcMain.handle("switch-scope", async (_event, scopePath: string) => {
    if (existsSync(scopePath)) {
      await loadScope(scopePath);
    }
    return currentScope;
  });
};
```

**Key patterns:**
1. **Scope management** — Get/set current scope from renderer
2. **Recent scopes** — Persisted for quick access
3. **Scope switching** — Load scope and update menu

### Preload Script (`desktop/preload.ts`)

```typescript
import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("electronAPI", {
  selectScope: () => ipcRenderer.invoke("select-scope"),
  getCurrentScope: () => ipcRenderer.invoke("get-current-scope"),
  getRecentScopes: () => ipcRenderer.invoke("get-recent-scopes"),
  switchScope: (scopePath: string) => 
    ipcRenderer.invoke("switch-scope", scopePath),
});
```

**Key patterns:**
1. **Context isolation** — Expose limited API to renderer
2. **Type-safe bridge** — Specific methods exposed
3. **Promise-based** — All IPC returns Promises

---

## 15. Summary

The Applications package provides **five end-user applications**:

1. **CLI** — Terminal code execution with embedded server
2. **Server** — HTTP API + MCP with plugin integration
3. **Web** — React SPA for source/secret management
4. **Desktop** — Electron wrapper with bundled CLI
5. **Marketing** — Landing page site

Key patterns include:
- **Effect Layers** — Dependency injection and resource management
- **ManagedRuntime** — Singleton with clean disposal
- **Session-based MCP** — Multiple concurrent clients
- **Embedded static files** — Single binary distribution
- **Host validation** — DNS rebinding protection

The application layer provides **multiple entry points** while sharing **common server infrastructure**.
