# Executor Source Plugins — Deep Dive Exploration

**Package:** `@executor/plugins`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/packages/plugins`  
**Total Plugins:** 7 plugins  
**Total Files:** ~100 files  

---

## 1. Module Overview

The Plugins package provides **source plugins** for the Executor system. Each plugin integrates external data sources, secret providers, or services:

- **openapi** — OpenAPI/Swagger spec integration with tool extraction
- **graphql** — GraphQL endpoint integration with introspection
- **mcp** — Model Context Protocol (remote + stdio transports)
- **google-discovery** — Google APIs with OAuth 2.0 PKCE flow
- **onepassword** — 1Password secret provider (read-only)
- **keychain** — System keychain secret provider (macOS/Windows)
- **file-secrets** — File-based secret storage (XDG-compliant)

### Key Responsibilities

1. **Source Management** — Add, remove, refresh external data sources
2. **Tool Extraction** — Convert API specs to callable tools
3. **Secret Providers** — Secure credential storage and retrieval
4. **OAuth Flows** — Handle authentication for protected APIs
5. **Plugin Extensions** — Type-safe API on `executor.<plugin>` namespace

---

## 2. File Inventory

### openapi (27 files)

| # | File | Description |
|---|------|-------------|
| 1 | `src/sdk/plugin.ts` | Main plugin definition with addSpec/removeSpec extensions |
| 2 | `src/sdk/extract.ts` | Tool extraction from OpenAPI paths |
| 3 | `src/sdk/invoke.ts` | HTTP request execution |
| 4 | `src/sdk/operation-store.ts` | Persist operation metadata |
| 5 | `src/sdk/parse.ts` | Spec parsing and validation |
| 6 | `src/sdk/preview.ts` | Spec preview functionality |
| 7 | `src/sdk/types.ts` | Type definitions |
| 8 | `src/api/*` | REST API handlers |
| 9 | `src/react/*` | React client components |

### graphql (23 files)

| # | File | Description |
|---|------|-------------|
| 1 | `src/sdk/plugin.ts` | Main plugin with addSource/removeSource |
| 2 | `src/sdk/extract.ts` | Tool extraction from introspection |
| 3 | `src/sdk/introspect.ts` | Schema introspection |
| 4 | `src/sdk/operation-store.ts` | Persist operation bindings |
| 5 | `src/sdk/invoke.ts` | GraphQL request execution |
| 6 | `src/sdk/kv-operation-store.ts` | KV-backed operation storage |
| 7 | `src/api/*` | REST API handlers |
| 8 | `src/react/*` | React client components |

### mcp (23 files)

| # | File | Description |
|---|------|-------------|
| 1 | `src/sdk/plugin.ts` | Main plugin (794 lines) with OAuth |
| 2 | `src/sdk/connection.ts` | MCP connection management |
| 3 | `src/sdk/discover.ts` | Tool discovery from server |
| 4 | `src/sdk/oauth.ts` | OAuth 2.0 flow handling |
| 5 | `src/sdk/binding-store.ts` | Persist tool bindings |
| 6 | `src/sdk/elicitation.test.ts` | Elicitation tests |
| 7 | `src/api/*` | REST API handlers |
| 8 | `src/react/*` | React client components |

### google-discovery (21 files)

| # | File | Description |
|---|------|-------------|
| 1 | `src/sdk/plugin.ts` | Main plugin with OAuth 2.0 PKCE |
| 2 | `src/sdk/oauth.ts` | OAuth 2.0 PKCE flow |
| 3 | `src/sdk/document.ts` | API document parsing |
| 4 | `src/sdk/invoke.ts` | Request execution |
| 5 | `src/sdk/binding-store.ts` | Persist bindings |
| 6 | `src/sdk/presets.ts` | Google API presets |
| 7 | `src/api/*` | REST API handlers |
| 8 | `src/react/*` | React client components |

### onepassword (12 files)

| # | File | Description |
|---|------|-------------|
| 1 | `src/sdk/plugin.ts` | Main plugin with configure/status |
| 2 | `src/sdk/service.ts` — 1Password SDK wrapper |
| 3 | `src/sdk/types.ts` — Config and auth types |
| 4 | `src/sdk/errors.ts` — Error definitions |
| 5 | `src/api/*` — REST API handlers |
| 6 | `src/react/*` — React client components |

### keychain (5 files)

| # | File | Description |
|---|------|-------------|
| 1 | `src/index.ts` | Main plugin definition |
| 2 | `src/keyring.ts` | Platform keyring access |
| 3 | `src/provider.ts` | Secret provider factory |
| 4 | `src/errors.ts` | Error definitions |
| 5 | `src/index.test.ts` | Tests |

### file-secrets (1 file)

| # | File | Description |
|---|------|-------------|
| 1 | `src/index.ts` | File-based secrets plugin |

---

## 3. Key Exports

### Plugin Factory Pattern

All plugins export a factory function following the same pattern:

```typescript
// openapi
export const openapiPlugin = (
  options: OpenApiPluginOptions,
): ExecutorPlugin<"openapi", OpenApiExtension> => definePlugin({...});

// graphql
export const graphqlPlugin = (
  options: GraphqlPluginOptions,
): ExecutorPlugin<"graphql", GraphqlExtension> => definePlugin({...});

// mcp
export const mcpPlugin = (
  options: McpPluginOptions,
): ExecutorPlugin<"mcp", McpExtension> => definePlugin({...});

// google-discovery
export const googleDiscoveryPlugin = (
  options: GoogleDiscoveryPluginOptions,
): ExecutorPlugin<"google-discovery", GoogleDiscoveryExtension> => definePlugin({...});

// onepassword
export const onepasswordPlugin = (
  options: OnePasswordPluginOptions,
): ExecutorPlugin<"onepassword", OnePasswordExtension> => definePlugin({...});

// keychain
export const keychainPlugin = (
  config?: KeychainPluginConfig,
): ExecutorPlugin<"keychain", KeychainExtension> => definePlugin({...});

// file-secrets
export const fileSecretsPlugin = (
  config?: FileSecretsPluginConfig,
): ExecutorPlugin<"fileSecrets", FileSecretsExtension> => definePlugin({...});
```

### Extension Interfaces

```typescript
// openapi — spec management
interface OpenApiExtension {
  addSpec(spec: SpecInput): Effect.Effect<void, OpenApiError>;
  removeSpec(namespace: string): Effect.Effect<void>;
  previewSpec(namespace: string): Effect.Effect<PreviewResult, OpenApiError>;
}

// graphql — source management
interface GraphqlExtension {
  addSource(config: GraphqlSourceConfig): Effect.Effect<void, GraphqlError>;
  removeSource(sourceId: string): Effect.Effect<void>;
}

// mcp — connection + OAuth
interface McpExtension {
  addRemoteSource(config: McpRemoteConfig): Effect.Effect<void, McpError>;
  addStdioSource(config: McpStdioConfig): Effect.Effect<void, McpError>;
  removeSource(sourceId: string): Effect.Effect<void>;
  startOAuth(sourceId: string): Effect.Effect<OAuthStartResult, McpError>;
  completeOAuth(sourceId: string, code: string): Effect.Effect<void, McpError>;
}

// google-discovery — OAuth flow
interface GoogleDiscoveryExtension {
  startOAuth(apiName: string): Effect.Effect<OAuthStartResult, GoogleError>;
  completeOAuth(apiName: string, code: string): Effect.Effect<void, GoogleError>;
}

// onepassword — config + vaults
interface OnePasswordExtension {
  configure(config: OnePasswordConfig): Effect.Effect<void, OnePasswordError>;
  getConfig(): Effect.Effect<OnePasswordConfig | null>;
  removeConfig(): Effect.Effect<void>;
  status(): Effect.Effect<ConnectionStatus>;
  listVaults(auth: OnePasswordAuth): Effect.Effect<Vault[]>;
  resolve(uri: string): Effect.Effect<string>;
}

// keychain — platform info
interface KeychainExtension {
  displayName: string;
  isSupported: boolean;
  has(secretId: SecretId): Effect.Effect<boolean>;
}

// file-secrets — file path
interface FileSecretsExtension {
  filePath: string;
}
```

---

## 4. Line-by-Line Analysis

### Plugin Initialization Pattern (openapi/plugin.ts:46-87)

```typescript
export const openapiPlugin = (
  options: OpenApiPluginOptions,
): ExecutorPlugin<"openapi", OpenApiExtension> =>
  definePlugin({
    key: PLUGIN_KEY,
    init: (ctx) =>
      Effect.gen(function* () {
        // 1. Create operation store
        const operationStore = yield* makeOperationStore(options.kv);

        // 2. Register tool invoker
        yield* ctx.tools.registerInvoker(
          makeInvoker(operationStore),
        );

        // 3. Add source manager
        yield* ctx.sources.addManager({
          list: () => listSources(operationStore),
          remove: (ns) => removeSpec(operationStore, ns),
          detect: (url) => detectOpenApi(url),
          refresh: (ns) => refreshSpec(operationStore, ns),
        });

        // 4. Build extension API
        const extension: OpenApiExtension = {
          addSpec: (spec) => addSpec(operationStore, spec),
          removeSpec: (ns) => removeSpec(operationStore, ns),
          previewSpec: (ns) => previewSpec(operationStore, ns),
        };

        return { extension };
      }),
  });
```

**Key patterns:**
1. **KV store creation** — Scoped storage for plugin state
2. **Invoker registration** — Enables tool execution
3. **Source manager** — Integrates with source lifecycle
4. **Extension API** — Type-safe plugin-specific methods

### Tool Extraction (openapi/extract.ts:42-118)

```typescript
export const extractToolsFromSpec = (
  spec: OpenAPIV3.Document,
  namespace: string,
): ExtractedTool[] => {
  const tools: ExtractedTool[] = [];

  for (const [path, pathItem] of Object.entries(spec.paths || {})) {
    if (!pathItem) continue;

    for (const [method, operation] of Object.entries(pathItem)) {
      if (!isHttpMethod(method) || !operation) continue;

      const operationId = operation.operationId || generateId(method, path);
      const toolId = `${namespace}.${operationId}`;

      tools.push({
        id: toolId,
        name: operation.summary || operationId,
        description: operation.description,
        inputSchema: buildInputSchema(operation),
        outputSchema: buildOutputSchema(operation),
        annotations: {
          mayElicit: false,
          readOnly: method === "get",
        },
      });
    }
  }

  return tools;
};
```

**Key patterns:**
1. **Path iteration** — Walk all paths and methods
2. **OperationId fallback** — Generate ID if missing
3. **Schema building** — Convert OpenAPI schemas to StandardSchema
4. **Annotations** — Mark read-only operations

### OAuth Flow (mcp/oauth.ts:28-94)

```typescript
export const startOAuth = (
  connection: McpConnection,
): Effect.Effect<OAuthStartResult, McpError, HttpClient> =>
  Effect.gen(function* () {
    const httpClient = yield* HttpClient.HttpClient;

    // 1. Fetch OAuth metadata
    const metadata = yield* fetchOAuthMetadata(httpClient, connection.endpoint);

    // 2. Generate PKCE challenge
    const { codeVerifier, codeChallenge } = yield* generatePkce();

    // 3. Build authorization URL
    const authUrl = buildAuthorizationUrl({
      authorizationEndpoint: metadata.authorization_endpoint,
      clientId: metadata.client_id,
      redirectUri: CALLBACK_URL,
      codeChallenge,
      state: generateState(),
    });

    // 4. Store verifier for callback
    yield* storeVerifier(connection.sourceId, codeVerifier);

    return { authorizationUrl: authUrl };
  });
```

**Key patterns:**
1. **Metadata discovery** — RFC 8414 OAuth 2.0 metadata
2. **PKCE generation** — Code verifier/challenge pair
3. **State parameter** — CSRF protection
4. **Verifier storage** — Persist for callback validation

### ScopedCache for MCP Connections (mcp/plugin.ts:145-203)

```typescript
const connectionCache = yield* ScopedCache.make({
  lookup: (key: string) =>
    Effect.acquireRelease(
      Effect.suspend(() => {
        const config = parseConnectionKey(key);
        return createMcpConnection(config);
      }),
      (connection) => Effect.promise(() => connection.close()),
    ),
  capacity: 64,
  timeToLive: Duration.minutes(5),
});

// Usage with acquireRelease
yield* Effect.acquireRelease(
  connectionCache.get(connectionKey),
  () => Effect.void, // Cache handles release
).pipe(
  Effect.flatMap((connection) => listToolsFromServer(connection)),
);
```

**Key patterns:**
1. **Connection pooling** — Reuse connections across requests
2. **TTL eviction** — 5-minute time-to-live
3. **Acquire/Release** — Automatic cleanup on scope end
4. **Lazy creation** — Connections created on first use

### Secret Provider Pattern (keychain/provider.ts:15-52)

```typescript
export const makeKeychainProvider = (
  serviceName: string,
): SecretProvider => ({
  key: "keychain",
  writable: true,

  get: (secretId) =>
    getPassword(serviceName, secretId).pipe(
      Effect.map((v) => v),
      Effect.orElseSucceed(() => null),
    ),

  set: (secretId, value) =>
    setPassword(serviceName, secretId, value),

  delete: (secretId) =>
    deletePassword(serviceName, secretId),

  list: () =>
    Effect.sync(() => {
      // List keys for the service
      const keys = listKeys(serviceName);
      return keys.map((k) => ({ id: k, name: k }));
    }),
});
```

**Key patterns:**
1. **SecretProvider interface** — Standard interface for all providers
2. **Writable flag** — Indicates provider supports set/delete
3. **Service name scoping** — Scope ID appended to service name
4. **Null on missing** — Returns null instead of error for missing secrets

### File Secrets XDG Compliance (file-secrets/index.ts:14-82)

```typescript
const xdgDataHome = (): string =>
  process.env.XDG_DATA_HOME?.trim() ||
  path.join(
    process.env.HOME || process.env.USERPROFILE || "~",
    ".local",
    "share",
  );

const authFilePath = (overrideDir?: string): string =>
  path.join(overrideDir ?? xdgDataHome(), "executor", "auth.json");

const writeScopeSecrets = (
  filePath: string,
  scopeId: string,
  secrets: Record<string, string>,
): void => {
  const dir = path.dirname(filePath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true, mode: 0o700 });
  }
  const full = readFullFile(filePath);
  if (Object.keys(secrets).length === 0) {
    delete full[scopeId];
  } else {
    full[scopeId] = secrets;
  }
  // Atomic write with restrictive permissions
  const tmp = `${filePath}.tmp`;
  fs.writeFileSync(tmp, JSON.stringify(full, null, 2), { mode: 0o600 });
  fs.renameSync(tmp, filePath);
};
```

**Key patterns:**
1. **XDG compliance** — Follows XDG Base Directory spec
2. **Scope isolation** — Each scope has separate keys in same file
3. **Atomic writes** — Write to temp, then rename
4. **Restrictive permissions** — 0o600 (owner read/write only)

### 1Password URI Resolution (onepassword/plugin.ts:99-138)

```typescript
const makeProvider = (
  getConfig: () => Effect.Effect<OnePasswordConfig | null>,
  ctx: PluginContext,
  timeoutMs: number,
): SecretProvider => ({
  key: "onepassword",
  writable: false, // Read-only

  get: (secretId) =>
    getConfig().pipe(
      Effect.flatMap((config) => {
        if (!config) return Effect.succeed(null);

        // Support both op:// URIs and vault-based lookups
        const uri = secretId.startsWith("op://")
          ? secretId
          : `op://${config.vaultId}/${secretId}/credential`;

        return getServiceFromConfig(config, ctx, timeoutMs).pipe(
          Effect.flatMap((svc) => svc.resolveSecret(uri)),
          Effect.map((v): string | null => v),
          Effect.orElseSucceed(() => null),
        );
      }),
      Effect.orElseSucceed(() => null),
    ),

  list: () =>
    getConfig().pipe(
      Effect.flatMap((config) => {
        if (!config) return Effect.succeed([]);
        return getServiceFromConfig(config, ctx, timeoutMs).pipe(
          Effect.flatMap((svc) => svc.listItems(config.vaultId)),
          Effect.map((items) =>
            items.map((item) => ({ id: item.id, name: item.title })),
          ),
        );
      }),
      Effect.orElseSucceed(() => []),
    ),
});
```

**Key patterns:**
1. **Read-only provider** — Cannot write to 1Password
2. **URI flexibility** — Supports both op:// and simple secretId
3. **Config dependency** — Requires vault ID configuration
4. **Graceful fallback** — Returns null on any error

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           Executor Plugins                                       │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐              │
│  │   openapi        │  │   graphql        │  │   mcp            │              │
│  │   (27 files)     │  │   (23 files)     │  │   (23 files)     │              │
│  │                  │  │                  │  │                  │              │
│  │  - addSpec       │  │  - addSource     │  │  - addRemote     │              │
│  │  - removeSpec    │  │  - removeSource  │  │  - addStdio      │              │
│  │  - previewSpec   │  │  - introspect    │  │  - startOAuth    │              │
│  │                  │  │                  │  │  - completeOAuth │              │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘              │
│                                                                                  │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐              │
│  │   google-        │  │   onepassword    │  │   keychain       │              │
│  │   discovery      │  │   (12 files)     │  │   (5 files)      │              │
│  │   (21 files)     │  │                  │  │                  │              │
│  │                  │  │  - configure     │  │  - displayName   │              │
│  │  - startOAuth    │  │  - status        │  │  - isSupported   │              │
│  │  - completeOAuth │  │  - listVaults    │  │  - has           │              │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘              │
│                                                                                  │
│  ┌──────────────────┐                                                            │
│  │   file-secrets   │                                                            │
│  │   (1 file)       │                                                            │
│  │                  │                                                            │
│  │  - filePath      │                                                            │
│  └──────────────────┘                                                            │
│                                                                                  │
│                              │                                                     │
│                              ▼                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │                    Plugin Architecture                                   │    │
│  │                                                                          │    │
│  │  definePlugin({ key, init(ctx) })                                       │    │
│  │    │                                                                     │    │
│  │    ├──> ctx.tools.registerInvoker(invoker)                              │    │
│  │    ├──> ctx.sources.addManager(SourceManager)                           │    │
│  │    ├──> ctx.secrets.addProvider(SecretProvider)                         │    │
│  │    └──> return { extension: {...} }                                     │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### Plugin Initialization Flow

```
Executor.create(config)
    │
    ▼
Plugin.init(ctx)
    │
    ├──> 1. Create stores (KV, operation, binding)
    │    └──> makeOperationStore(kv)
    │    └──> makeBindingStore(kv)
    │
    ├──> 2. Register tool invoker
    │    └──> ctx.tools.registerInvoker(makeInvoker(store))
    │    └──> Enables tool execution for this plugin
    │
    ├──> 3. Add source manager (if applicable)
    │    └──> ctx.sources.addManager({ list, remove, detect, refresh })
    │    └──> Integrates with source lifecycle
    │
    ├──> 4. Add secret provider (if applicable)
    │    └──> ctx.secrets.addProvider(makeProvider())
    │    └──> Adds to provider chain
    │
    └──> 5. Return extension API
         └──> { extension: { addSource, removeSource, ... } }
         └──> Attached to executor.<pluginKey>
```

### OAuth Flow (MCP + Google Discovery)

```
User clicks "Connect"
    │
    ▼
startOAuth(sourceId)
    │
    ├──> 1. Fetch OAuth metadata
    │    └──> GET /.well-known/oauth-authorization-server
    │
    ├──> 2. Generate PKCE
    │    ├──> codeVerifier = random(32 bytes)
    │    └──> codeChallenge = SHA256(codeVerifier)
    │
    ├──> 3. Build auth URL
    │    └──> authorization_endpoint?
    │         client_id=...&
    │         redirect_uri=...&
    │         code_challenge=...&
    │         state=...
    │
    ├──> 4. Store verifier
    │    └──> kv.set(`oauth:${sourceId}:verifier`, codeVerifier)
    │
    └──> 5. Return auth URL to user
         └──> User opens browser

User completes auth → callback with code + state
    │
    ▼
completeOAuth(sourceId, code)
    │
    ├──> 1. Validate state
    │    └──> Compare stored state
    │
    ├──> 2. Retrieve verifier
    │    └──> kv.get(`oauth:${sourceId}:verifier`)
    │
    ├──> 3. Exchange code for token
    │    └──> POST token_endpoint
    │         grant_type=authorization_code&
    │         code=...&
    │         redirect_uri=...&
    │         code_verifier=...
    │
    ├──> 4. Store tokens
    │    └──> kv.set(`oauth:${sourceId}:tokens`, { access, refresh })
    │
    └──> 5. Refresh source tools
         └──> listToolsFromServer()
```

### Secret Resolution Flow

```
ctx.secrets.resolve(secretId, scopeId)
    │
    ▼
┌─────────────────────────────────────┐
│  Provider Chain (sequential)        │
├─────────────────────────────────────┤
│  1. file-secrets (if configured)    │
│     └──> Read auth.json             │
│     └──> Return value or null       │
│                                      │
│  2. keychain (if supported)         │
│     └──> Query system keychain      │
│     └──> Return value or null       │
│                                      │
│  3. onepassword (if configured)     │
│     └──> Resolve op:// URI          │
│     └──> Return value or null       │
│                                      │
│  4. Return final value or error     │
└─────────────────────────────────────┘
```

---

## 7. Key Patterns

### definePlugin Helper

```typescript
import { definePlugin, type ExecutorPlugin } from "@executor/sdk";

export const myPlugin = (options) =>
  definePlugin({
    key: "my-plugin",
    init: (ctx) =>
      Effect.gen(function* () {
        // Plugin initialization
        return { extension: {...} };
      }),
  });
```

**Benefits:**
1. **Type inference** — Plugin key and extension types inferred
2. **Effect integration** — init() returns Effect for dependency injection
3. **Consistent shape** — All plugins follow same structure

### Source Manager Delegation

```typescript
yield* ctx.sources.addManager({
  list: () => listSourcesFromStore(),
  remove: (sourceId) => removeSourceFromStore(sourceId),
  detect: (url) => detectSourceFromUrl(url),
  refresh: (sourceId) => refreshSourceTools(sourceId),
});
```

**Benefits:**
1. **Separation of concerns** — Plugin manages state, SourceRegistry coordinates
2. **Multiple managers** — Multiple plugins can manage sources
3. **Parallel detection** — SourceRegistry handles parallel detection

### Binding Store Pattern

```typescript
const bindingStore = yield* makeBindingStore(options.kv);

// Store tool bindings
yield* bindingStore.saveBinding(sourceId, {
  sourceId,
  tools: [...],
  headers: {...},
});

// Retrieve for invocation
const binding = yield* bindingStore.getBinding(sourceId);
```

**Benefits:**
1. **Persistence** — Survives restarts
2. **Scope isolation** — Each scope has separate bindings
3. **Type safety** — Effect Schema validation

### OAuth State Management

```typescript
// Generate and store state
const state = crypto.randomBytes(16).toString("hex");
yield* kv.set(`oauth:${sourceId}:state`, state);

// On callback, validate
const storedState = yield* kv.get(`oauth:${sourceId}:state`);
if (state !== storedState) {
  return yield* new OAuthError("Invalid state");
}
```

**Benefits:**
1. **CSRF protection** — Prevents cross-site request forgery
2. **Scope isolation** — State per source
3. **Automatic cleanup** — State removed after use

### Connection Pooling with ScopedCache

```typescript
const cache = yield* ScopedCache.make({
  lookup: (key) =>
    Effect.acquireRelease(createConnection(key), (conn) => conn.close()),
  capacity: 64,
  timeToLive: Duration.minutes(5),
});

// Usage
yield* cache.get(key).pipe(
  Effect.flatMap((conn) => executeRequest(conn)),
);
```

**Benefits:**
1. **Resource efficiency** — Reuse expensive connections
2. **Automatic cleanup** — Release on scope end
3. **TTL eviction** — Prevent stale connections

---

## 8. Integration Points

### Plugin Dependencies

| Plugin | Dependencies | Purpose |
|--------|-------------|---------|
| openapi | @effect/platform, openapi-types | HTTP client, OpenAPI types |
| graphql | @effect/platform, graphql | GraphQL introspection |
| mcp | @effect/platform, @modelcontextprotocol/sdk | MCP protocol |
| google-discovery | @effect/platform, crypto | OAuth PKCE |
| onepassword | @1password/sdk, op-cli | 1Password access |
| keychain | keytar | System keychain |
| file-secrets | node:fs, node:path | File I/O |

### Plugin Dependents

| Package | Relationship |
|---------|-------------|
| @executor/apps/cli | Loads plugins on startup |
| @executor/apps/server | Provides KV storage for plugins |
| @executor/hosts/mcp | Exposes plugin tools via MCP |

### Source Registry Integration

Plugins register source managers with the SourceRegistry:

```typescript
// In plugin init()
yield* ctx.sources.addManager({
  list: () => ...,
  remove: (id) => ...,
  detect: (url) => ...,
  refresh: (id) => ...,
});

// SourceRegistry handles parallel detection
const results = yield* SourceRegistry.detectSources(url);
```

### Secret Provider Chain

Plugins add secret providers to the chain:

```typescript
// In plugin init()
yield* ctx.secrets.addProvider(makeProvider());

// SecretRegistry tries providers in order
const value = yield* ctx.secrets.resolve(secretId, scopeId);
```

---

## 9. Error Handling

### Plugin-Specific Error Classes

```typescript
// openapi
export class OpenApiError extends Schema.Class<{
  operation: string;
  message: string;
  cause?: unknown;
}> {}

// graphql
export class GraphqlError extends Schema.Class<{
  operation: string;
  message: string;
  errors?: GraphQLError[];
}> {}

// mcp
export class McpError extends Schema.Class<{
  operation: string;
  message: string;
  code?: number;
}> {}

// google-discovery
export class GoogleError extends Schema.Class<{
  operation: string;
  message: string;
}> {}

// onepassword
export class OnePasswordError extends Schema.Class<{
  operation: string;
  message: string;
}> {}

// keychain
export class KeychainError extends Schema.Class<{
  operation: string;
  message: string;
}> {}
```

### Error Mapping Pattern

```typescript
yield* someEffect.pipe(
  Effect.mapError((cause) =>
    new OpenApiError({
      operation: "parse-spec",
      message: cause instanceof Error ? cause.message : String(cause),
      cause,
    })
  ),
);
```

### Graceful Degradation

```typescript
// Secret providers return null on failure
get: (secretId) =>
  getPassword(serviceName, secretId).pipe(
    Effect.orElseSucceed(() => null),
  ),

// Source detection returns empty array on failure
detect: (url) =>
  detectOpenApi(url).pipe(
    Effect.orElseSucceed(() => []),
  ),
```

---

## 10. Testing Strategy

### Unit Tests

| Plugin | Test File | Coverage |
|--------|-----------|----------|
| openapi | `src/sdk/index.test.ts`, `src/sdk/plugin.test.ts` | Schema parsing, tool extraction |
| graphql | `src/sdk/plugin.test.ts` | Introspection, operation storage |
| mcp | `src/sdk/plugin.test.ts`, `src/sdk/elicitation.test.ts` | OAuth, elicitation handling |
| google-discovery | `src/sdk/plugin.test.ts` | OAuth PKCE flow |
| keychain | `src/index.test.ts` | Platform detection |

### Test Patterns

```typescript
// Mocking KV storage
const makeTestKv = () => ({
  get: Effect.succeed(null),
  set: Effect.void,
  delete: Effect.void,
});

// Testing plugin initialization
const plugin = openapiPlugin({ kv: makeTestKv() });
const handle = yield* Effect.runPromise(
  plugin.init({ scope, tools, sources, secrets, policies })
);
assert.isDefined(handle.extension.addSpec);
```

### Integration Tests

Real-world testing with actual APIs:

```typescript
// Real OpenAPI specs
describe("real specs", () => {
  it("should parse GitHub API", async () => {
    const spec = await fetch("https://api.github.com/openapi.json");
    const result = extractToolsFromSpec(spec, "github");
    expect(result.length).toBeGreaterThan(0);
  });
});
```

---

## 11. Design Decisions

### Why Separate Plugins per Source Type?

1. **Separation of concerns** — Each plugin handles one source type
2. **Independent versioning** — Plugins can evolve separately
3. **Optional dependencies** — Only install needed plugins
4. **Clear boundaries** — No cross-plugin coupling

### Why Extension API Pattern?

```typescript
executor.onepassword.configure(config);
executor.mcp.startOAuth(sourceId);
```

1. **Type safety** — TypeScript knows available methods
2. **Discoverability** — IDE autocomplete
3. **Consistency** — All plugins follow same pattern
4. **Testability** — Extensions can be mocked

### Why OAuth per Plugin?

1. **Different flows** — MCP uses RFC 8414, Google uses PKCE
2. **Isolation** — OAuth state per source
3. **Flexibility** — Each plugin handles its own tokens

### Why Read-Only 1Password?

1. **Security model** — 1Password is for secure storage, not app data
2. **User control** — Users manage secrets in 1Password app
3. **Audit trail** — Changes tracked in 1Password history

### Why XDG for File Secrets?

1. **Cross-platform** — XDG works on Linux, macOS, Windows
2. **Convention** — Standard location for app data
3. **User expectation** — Users know where to find auth.json

---

## 12. Summary

The Plugins package provides **seven source plugins** following consistent patterns:

1. **definePlugin** — Type-safe plugin factory
2. **Extension API** — `executor.<plugin>.method()` pattern
3. **Invoker registration** — Tool execution integration
4. **Source manager** — Source lifecycle integration
5. **Secret providers** — Credential chain integration
6. **OAuth flows** — MCP and Google Discovery authentication
7. **KV persistence** — Scoped storage for plugin state

The plugin architecture enables **modular source integration** while maintaining **type safety** and **consistent patterns** across all plugins.
