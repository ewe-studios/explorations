# Executor Core SDK — Deep Dive Exploration

**Package:** `@executor/sdk`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/packages/core/sdk`  
**Total Files:** 23 TypeScript files  
**Total Lines:** 3,601 lines  

---

## 1. Module Overview

The Core SDK is the **main public API** for the Executor system. It provides:

- **Unified interfaces** for tools, sources, secrets, and policies
- **Type-safe abstractions** built on Effect.ts
- **Plugin system** for extensibility
- **In-memory implementations** for testing and simple deployments
- **Schema utilities** for TypeScript preview generation

### Key Responsibilities

1. **Tool Registry** — Central catalog of all available tools from all sources
2. **Source Management** — Discovery and lifecycle for API sources (OpenAPI, GraphQL, MCP, etc.)
3. **Secret Store** — Abstraction over secret providers (keychain, env, 1Password, etc.)
4. **Policy Engine** — Access control for tool invocations
5. **Elicitation Protocol** — Structured user interaction flow
6. **Plugin System** — Extension mechanism for custom functionality

---

## 2. File Inventory

### Core API Files

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/executor.ts` | 229 | Main Executor type and factory function |
| 2 | `src/tools.ts` | 217 | Tool registry, schemas, invocation |
| 3 | `src/sources.ts` | 186 | Source management and detection |
| 4 | `src/secrets.ts` | 106 | Secret store and provider interfaces |
| 5 | `src/policies.ts` | 37 | Policy engine interface |
| 6 | `src/scope.ts` | 9 | Scope model for isolation |
| 7 | `src/plugin.ts` | 77 | Plugin system definition |
| 8 | `src/elicitation.ts` | 78 | User interaction protocol |
| 9 | `src/ids.ts` | 13 | Branded ID types |
| 10 | `src/errors.ts` | 39 | Error types |
| 11 | `src/index.ts` | 108 | Public exports |
| 12 | `src/index.test.ts` | 663 | Integration tests |

### Schema Utilities

| # | File | Lines | Description |
|---|------|-------|-------------|
| 13 | `src/schema-refs.ts` | 144 | JSON Schema reference handling |
| 14 | `src/schema-types.ts` | 405 | TypeScript preview generation |
| 15 | `src/schema-types.test.ts` | 283 | Schema type tests |

### Runtime & Testing

| # | File | Lines | Description |
|---|------|-------|-------------|
| 16 | `src/runtime-tools.ts` | 183 | Runtime tool registration |
| 17 | `src/testing.ts` | 38 | Test configuration helpers |
| 18 | `src/plugin-kv.ts` | 74 | KV storage abstraction for plugins |

### In-Memory Implementations

| # | File | Lines | Description |
|---|------|-------|-------------|
| 19 | `src/in-memory/tool-registry.ts` | 210 | In-memory tool registry |
| 20 | `src/in-memory/secret-store.ts` | 133 | In-memory secret store |
| 21 | `src/in-memory/policy-engine.ts` | 27 | In-memory policy engine |

### Built-in Plugins

| # | File | Lines | Description |
|---|------|-------|-------------|
| 22 | `src/plugins/in-memory-tools.ts` | 335 | In-memory tools plugin |
| 23 | `vitest.config.ts` | 7 | Test configuration |

---

## 3. Key Exports

### Main Types & Interfaces

```typescript
// executor.ts
export type Executor<TPlugins extends readonly ExecutorPlugin<string, object>[] = []> = {
  readonly scope: Scope;
  readonly tools: { /* tool operations */ };
  readonly sources: { /* source operations */ };
  readonly policies: { /* policy operations */ };
  readonly secrets: { /* secret operations */ };
  readonly close: () => Effect.Effect<void>;
} & PluginExtensions<TPlugins>;
```

### Tool System

```typescript
// tools.ts
export class ToolMetadata extends Schema.Class<ToolMetadata>("ToolMetadata")({
  id: ToolId,
  pluginKey: Schema.String,
  sourceId: Schema.String,
  name: Schema.String,
  description: Schema.optional(Schema.String),
  mayElicit: Schema.optional(Schema.Boolean),
}) {}

export class ToolSchema extends Schema.Class<ToolSchema>("ToolSchema")({
  id: ToolId,
  inputTypeScript: Schema.optional(Schema.String),
  outputTypeScript: Schema.optional(Schema.String),
  typeScriptDefinitions: Schema.optional(
    Schema.Record({ key: Schema.String, value: Schema.String }),
  ),
}) {}

export class ToolRegistry extends Context.Tag("@executor/sdk/ToolRegistry")<
  ToolRegistry,
  {
    readonly list: (filter?: ToolListFilter) => Effect.Effect<readonly ToolMetadata[]>;
    readonly schema: (toolId: ToolId) => Effect.Effect<ToolSchema, ToolNotFoundError>;
    readonly invoke: (toolId: string, args: unknown, options: InvokeOptions) => 
      Effect.Effect<ToolInvocationResult, ToolNotFoundError | ToolInvocationError>;
    readonly definitions: () => Effect.Effect<Record<string, unknown>>;
    readonly registerDefinitions: (defs: Record<string, unknown>) => Effect.Effect<void>;
    readonly registerInvoker: (pluginKey: string, invoker: ToolInvoker) => Effect.Effect<void>;
    readonly register: (tools: readonly ToolRegistration[]) => Effect.Effect<void>;
  }
>() {}
```

### Source System

```typescript
// sources.ts
export class Source extends Schema.Class<Source>("Source")({
  id: Schema.String,
  name: Schema.String,
  kind: Schema.String,
  runtime: Schema.optional(Schema.Boolean),
  canRemove: Schema.optional(Schema.Boolean),
  canRefresh: Schema.optional(Schema.Boolean),
}) {}

export class SourceRegistry extends Context.Tag("@executor/sdk/SourceRegistry")<
  SourceRegistry,
  {
    readonly addManager: (manager: SourceManager) => Effect.Effect<void>;
    readonly list: () => Effect.Effect<readonly Source[]>;
    readonly remove: (sourceId: string) => Effect.Effect<void>;
    readonly refresh: (sourceId: string) => Effect.Effect<void>;
    readonly detect: (url: string) => Effect.Effect<readonly SourceDetectionResult[]>;
  }
>() {}
```

### Secret System

```typescript
// secrets.ts
export interface SecretProvider {
  readonly key: string;
  readonly writable: boolean;
  readonly get: (key: string) => Effect.Effect<string | null>;
  readonly set?: (key: string, value: string) => Effect.Effect<void>;
  readonly delete?: (key: string) => Effect.Effect<boolean>;
  readonly list?: () => Effect.Effect<readonly { id: string; name: string }[]>;
}

export class SecretStore extends Context.Tag("@executor/sdk/SecretStore")<
  SecretStore,
  {
    readonly list: (scopeId: ScopeId) => Effect.Effect<readonly SecretRef[]>;
    readonly resolve: (secretId: SecretId, scopeId: ScopeId) => 
      Effect.Effect<string, SecretNotFoundError | SecretResolutionError>;
    readonly set: (input: SetSecretInput) => Effect.Effect<SecretRef, SecretResolutionError>;
    readonly addProvider: (provider: SecretProvider) => Effect.Effect<void>;
  }
>() {}
```

### Elicitation Protocol

```typescript
// elicitation.ts
export class FormElicitation extends Schema.TaggedClass<FormElicitation>()(
  "FormElicitation",
  {
    message: Schema.String,
    requestedSchema: Schema.Record({ key: Schema.String, value: Schema.Unknown }),
  },
) {}

export class UrlElicitation extends Schema.TaggedClass<UrlElicitation>()(
  "UrlElicitation",
  {
    message: Schema.String,
    url: Schema.String,
    elicitationId: Schema.String,
  },
) {}

export type ElicitationHandler = (
  ctx: ElicitationContext,
) => Effect.Effect<ElicitationResponse>;
```

### Plugin System

```typescript
// plugin.ts
export interface ExecutorPlugin<TKey extends string = string, TExtension extends object = object> {
  readonly key: TKey;
  readonly init: (ctx: PluginContext) => Effect.Effect<PluginHandle<TExtension>, Error>;
}

export interface PluginContext {
  readonly scope: Scope;
  readonly tools: Context.Tag.Service<typeof ToolRegistry>;
  readonly sources: Context.Tag.Service<typeof SourceRegistry>;
  readonly secrets: Context.Tag.Service<typeof SecretStore>;
  readonly policies: Context.Tag.Service<typeof PolicyEngine>;
}
```

---

## 4. Line-by-Line Analysis

### Executor Factory Function (`executor.ts:132-229`)

```typescript
export const createExecutor = <
  const TPlugins extends readonly ExecutorPlugin<string, object>[] = [],
>(
  config: ExecutorConfig<TPlugins>,
): Effect.Effect<Executor<TPlugins>, Error> =>
  Effect.gen(function* () {
    const { scope, tools, sources, secrets, policies, plugins = [] } = config;

    // Initialize all plugins
    const handles = new Map<string, PluginHandle<object>>();
    const extensions: Record<string, object> = {};

    for (const plugin of plugins) {
      const handle = yield* plugin.init({
        scope,
        tools,
        sources,
        secrets,
        policies,
      });
      handles.set(plugin.key, handle);
      extensions[plugin.key] = handle.extension;
    }

    const base = {
      scope,
      tools: {
        list: (filter?: ToolListFilter) => tools.list(filter),
        schema: (toolId: string) => tools.schema(toolId as ToolId),
        definitions: () => tools.definitions(),
        invoke: (toolId: string, args: unknown, options: InvokeOptions) => {
          const tid = toolId as ToolId;
          return Effect.gen(function* () {
            // 1. Check policy first
            yield* policies.check({ scopeId: scope.id, toolId: tid });

            // 2. Dynamically resolve annotations (e.g., requiresApproval)
            const annotations = yield* tools.resolveAnnotations(tid);
            if (annotations?.requiresApproval) {
              const handler = resolveElicitationHandler(options);
              const response = yield* handler({
                toolId: tid,
                args,
                request: new FormElicitation({
                  message: annotations.approvalDescription ?? `Approve ${toolId}?`,
                  requestedSchema: {},
                }),
              });
              if (response.action !== "accept") {
                return yield* new ElicitationDeclinedError({
                  toolId: tid,
                  action: response.action,
                });
              }
            }

            // 3. Invoke the tool
            return yield* tools.invoke(tid, args, options);
          });
        },
      },
      // ... sources, policies, secrets
    };

    return Object.assign(base, extensions) as Executor<TPlugins>;
  });
```

**Key patterns:**
1. **Policy-first enforcement** — Every tool invocation checks policies before execution
2. **Dynamic annotation resolution** — Plugin-level control over approval requirements
3. **Elicitation interception** — User approval flow before sensitive operations
4. **Plugin extension merging** — `Object.assign(base, extensions)` adds plugin APIs

### Tool Invocation Flow (`tools.ts:81-88`)

The `ToolRegistry` interface defines the core tool operations:

```typescript
readonly invoke: (
  toolId: ToolId,
  args: unknown,
  options: InvokeOptions,
) => Effect.Effect<
  ToolInvocationResult,
  ToolNotFoundError | ToolInvocationError | ElicitationDeclinedError
>;
```

### Source Detection (`sources.ts:166-185`)

```typescript
detect: (url: string) =>
  Effect.gen(function* () {
    const detectors = [...managers.values()]
      .filter((m) => m.detect)
      .map((m) =>
        m.detect!(url).pipe(
          Effect.timeout("5 seconds"),
          Effect.catchAll(() => Effect.succeed(null)),
        ),
      );

    const results = yield* Effect.all(detectors, { concurrency: "unbounded" });
    return results
      .filter((r): r is SourceDetectionResult => r !== null)
      .sort((a, b) => {
        const order = { high: 0, medium: 1, low: 2 };
        return order[a.confidence] - order[b.confidence];
      });
  });
```

**Key patterns:**
1. **Parallel detection** — All plugins probe the URL concurrently
2. **Timeout protection** — 5-second limit per detector
3. **Graceful degradation** — Failed detectors return `null` instead of erroring
4. **Confidence sorting** — Results sorted by confidence level

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Executor (Public API)                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐             │
│  │    Executor     │  │    Executor     │  │    Executor     │             │
│  │    .tools       │  │   .sources      │  │   .secrets      │             │
│  │                 │  │                 │  │                 │             │
│  │  - list()       │  │  - list()       │  │  - list()       │             │
│  │  - schema()     │  │  - remove()     │  │  - resolve()    │             │
│  │  - invoke()     │  │  - refresh()    │  │  - set()        │             │
│  │                 │  │  - detect()     │  │  - remove()     │             │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘             │
│           │                    │                    │                       │
│           ▼                    ▼                    ▼                       │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐             │
│  │  ToolRegistry   │  │ SourceRegistry  │  │  SecretStore    │             │
│  │  (Context.Tag)  │  │  (Context.Tag)  │  │  (Context.Tag)  │             │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘             │
│           │                    │                    │                       │
│           ▼                    ▼                    ▼                       │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐             │
│  │  In-Memory      │  │  Plugin         │  │  In-Memory      │             │
│  │  KV-Backed      │  │  SourceManager  │  │  Provider       │             │
│  │  Storage        │  │  (OpenAPI,MCP)  │  │  (Keychain,     │             │
│  │                 │  │                 │  │   1Password)    │             │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘             │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │                     Plugin System                                │       │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │       │
│  │  │ inMemoryTools│  │  OpenAPI     │  │  Custom      │          │       │
│  │  │  Plugin      │  │  Plugin      │  │  Plugins     │          │       │
│  │  └──────────────┘  └──────────────┘  └──────────────┘          │       │
│  └─────────────────────────────────────────────────────────────────┘       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### Tool Registration Flow

```
Plugin.init() 
    │
    ├──> ctx.tools.registerInvoker(pluginKey, invoker)
    │    └──> Invokers map: pluginKey → ToolInvoker
    │
    ├──> ctx.tools.registerDefinitions(defs)
    │    └──> Shared defs map: name → JSON Schema
    │
    └──> ctx.tools.register(tools)
         └──> Tools map: toolId → ToolRegistration
```

### Tool Invocation Flow

```
executor.tools.invoke(toolId, args, options)
    │
    ├──> 1. Policy Check
    │    └──> policies.check({ scopeId, toolId })
    │         └──> PolicyDeniedError (if denied)
    │
    ├──> 2. Annotation Resolution
    │    └──> tools.resolveAnnotations(toolId)
    │         └──> ToolAnnotations? (requiresApproval?)
    │
    ├──> 3. Elicitation (if required)
    │    └──> onElicitation handler
    │         └──> ElicitationDeclinedError (if declined)
    │
    └──> 4. Tool Invocation
         └──> invoker.invoke(toolId, args, options)
              └──> ToolInvocationResult
```

### Secret Resolution Flow

```
executor.secrets.resolve(secretId, scopeId)
    │
    ├──> 1. Load SecretRef from KV
    │    └──> refsKv.get(secretId)
    │         └──> SecretNotFoundError (if not found)
    │
    ├──> 2. Extract provider key from ref
    │    └──> ref.provider (Option<string>)
    │
    ├──> 3. Query providers
    │    ├──> If provider specified: query that provider only
    │    └──> If no provider: iterate all providers until value found
    │
    └──> 4. Return value or SecretResolutionError
```

---

## 7. Key Patterns

### Effect.ts Usage

The SDK is built entirely on **Effect.ts** for:

1. **Error handling** — All errors are typed and tracked in the effect signature
2. **Dependency injection** — `Context.Tag` for service locators
3. **Composability** — All operations return `Effect.Effect<A, E, R>`

Example error signature:
```typescript
Effect.Effect<
  ToolInvocationResult,
  | ToolNotFoundError
  | ToolInvocationError
  | PolicyDeniedError
  | ElicitationDeclinedError
>
```

### Schema.Class Pattern

All data models use Effect's `Schema.Class`:

```typescript
export class ToolMetadata extends Schema.Class<ToolMetadata>("ToolMetadata")({
  id: ToolId,
  pluginKey: Schema.String,
  sourceId: Schema.String,
  name: Schema.String,
  description: Schema.optional(Schema.String),
}) {}
```

Benefits:
- Runtime validation
- Automatic serialization
- TypeScript type inference
- JSON Schema generation

### Branded Types for IDs

```typescript
// ids.ts
export const ToolId = Schema.String.pipe(Schema.brand("ToolId"));
export type ToolId = typeof ToolId.Type;
```

This provides compile-time type safety for IDs — a `ToolId` is not interchangeable with a `SecretId`.

### Plugin Extension Pattern

Plugins extend the Executor type through mapped types:

```typescript
export type PluginExtensions<TPlugins> = {
  readonly [P in TPlugins[number] as P["key"]]: P extends ExecutorPlugin<string, infer TExt>
    ? TExt
    : never;
};
```

---

## 8. Integration Points

### Dependencies

| Package | Purpose |
|---------|---------|
| `effect` | Core Effect.ts runtime |
| `@executor/codemode-core` | Sandbox tool invoker type |

### Dependents

| Package | Relationship |
|---------|--------------|
| `@executor/execution` | Uses SDK for tool invocation |
| `@executor/api` | REST API wrapper around SDK services |
| `@executor/storage-file` | KV-backed implementations of SDK services |
| `@executor/storage-postgres` | PostgreSQL-backed implementations |
| `@executor/config` | Config file integration with SDK |
| `@executor/plugins/*` | All source plugins implement SDK interfaces |
| `@executor/hosts/mcp` | MCP server exposes SDK via MCP protocol |

---

## 9. Error Handling

### Error Hierarchy

All errors are tagged errors using Effect's `Schema.TaggedError`:

```typescript
// errors.ts
export class ToolNotFoundError extends Schema.TaggedError<ToolNotFoundError>()(
  "ToolNotFoundError",
  { toolId: ToolId },
) {}

export class ToolInvocationError extends Schema.TaggedError<ToolInvocationError>()(
  "ToolInvocationError",
  {
    toolId: ToolId,
    message: Schema.String,
    cause: Schema.optional(Schema.Unknown),
  },
) {}

export class SecretNotFoundError extends Schema.TaggedError<SecretNotFoundError>()(
  "SecretNotFoundError",
  { secretId: SecretId },
) {}

export class PolicyDeniedError extends Schema.TaggedError<PolicyDeniedError>()(
  "PolicyDeniedError",
  {
    policyId: PolicyId,
    toolId: ToolId,
    reason: Schema.String,
  },
) {}

export class ElicitationDeclinedError extends Schema.TaggedError<ElicitationDeclinedError>()(
  "ElicitationDeclinedError",
  {
    toolId: ToolId,
    action: Schema.Literal("decline", "cancel"),
  },
) {}
```

### Error Propagation

Errors flow through the Effect pipeline:

```typescript
executor.tools.invoke(toolId, args, options)
  .pipe(
    Effect.catchTag("ToolNotFoundError", (err) => 
      Effect.succeed({ error: `Tool not found: ${err.toolId}` })
    ),
    Effect.catchTag("ElicitationDeclinedError", (err) =>
      Effect.succeed({ error: `User declined: ${err.action}` })
    ),
  )
```

---

## 10. Testing Strategy

### Unit Tests

**File:** `src/index.test.ts` (663 lines)

Tests cover:
- Executor creation and plugin initialization
- Tool registration and invocation
- Source management
- Secret storage and resolution
- Policy enforcement
- Elicitation handling

### Test Utilities

**File:** `src/testing.ts` (38 lines)

```typescript
export const makeTestConfig = () => {
  // Returns a pre-configured test setup with in-memory implementations
};
```

### Schema Type Tests

**File:** `src/schema-types.test.ts` (283 lines)

Tests for TypeScript preview generation:
- Schema to TypeScript conversion
- Reference resolution
- Definition hoisting

### In-Memory Implementations

The SDK provides in-memory versions of all services for testing:

```typescript
// In-memory implementations
export { makeInMemoryToolRegistry } from "./in-memory/tool-registry";
export { makeInMemorySecretStore } from "./in-memory/secret-store";
export { makeInMemoryPolicyEngine } from "./in-memory/policy-engine";
export { makeInMemorySourceRegistry } from "./sources";
```

---

## 11. In-Memory Tools Plugin Deep Dive

The built-in `inMemoryTools` plugin allows registering ad-hoc tools:

### Plugin API

```typescript
export interface InMemoryToolsPluginExtension {
  readonly addTools: (
    tools: readonly MemoryToolDefinition<any, any>[],
  ) => Effect.Effect<void>;
}
```

### Tool Definition

```typescript
export interface MemoryToolDefinition<TInput = unknown, TOutput = unknown> {
  readonly name: string;
  readonly description?: string;
  readonly inputSchema: Schema.Schema<TInput>;
  readonly outputSchema?: Schema.Schema<TOutput>;
  readonly handler: MemoryToolHandler<TInput>;
}

export type MemoryToolHandler<TInput> =
  | ((args: TInput) => unknown)
  | ((args: TInput, ctx: MemoryToolContext) => Effect.Effect<unknown, unknown>);
```

### Handler Context

```typescript
export interface MemoryToolContext {
  readonly elicit: (
    request: ElicitationRequest,
  ) => Effect.Effect<Record<string, unknown>, ElicitationDeclinedError>;
  readonly sdk: MemoryToolSdkAccess;
}

export interface MemoryToolSdkAccess {
  readonly secrets: {
    readonly list: () => Effect.Effect<readonly SecretRef[]>;
    readonly resolve: (secretId: SecretId) => Effect.Effect<string, unknown>;
    readonly status: (secretId: SecretId) => Effect.Effect<"resolved" | "missing">;
    readonly set: (input: { id: SecretId; name: string; value: string; purpose?: string }) => 
      Effect.Effect<SecretRef, unknown>;
    readonly remove: (secretId: SecretId) => Effect.Effect<boolean, unknown>;
  };
}
```

### Usage Example

```typescript
import { inMemoryToolsPlugin, tool } from "@executor/sdk";
import { Schema } from "effect";

const plugin = inMemoryToolsPlugin({
  namespace: "myTools",
  tools: [
    tool({
      name: "greet",
      description: "Greet someone by name",
      inputSchema: Schema.Struct({ name: Schema.String }),
      handler: (args) => `Hello, ${args.name}!`,
    }),
    tool({
      name: "fetchSecret",
      description: "Fetch a secret value",
      inputSchema: Schema.Struct({ secretId: Schema.String }),
      handler: async (args, ctx) => {
        const value = await ctx.sdk.secrets.resolve(args.secretId);
        return { value };
      },
    }),
  ],
});
```

---

## 12. Design Decisions

### Why Effect.ts?

1. **Typed errors** — Every possible error is tracked in the type system
2. **Composability** — Effects can be combined, retried, timeout, etc.
3. **Dependency injection** — `Context.Tag` provides clean service location
4. **Resource safety** — Built-in finalizers and scopes

### Why Schema.Class?

1. **Single source of truth** — Schema defines both runtime validation and TypeScript types
2. **JSON Schema generation** — Used for tool documentation and validation
3. **Serialization** — Automatic encoding/decoding for storage
4. **Ref handling** — Built-in support for JSON Schema references

### Why Branded Types for IDs?

1. **Compile-time safety** — Prevents mixing up ToolId, SecretId, PolicyId
2. **Documentation** — Type signature shows exactly what ID type is expected
3. **No runtime cost** — Brands are erased at runtime

### Why In-Memory Implementations?

1. **Testing** — Easy to test without database dependencies
2. **Simple deployments** — Can run entirely in-memory for development
3. **Plugin development** — Plugin authors can test without full infrastructure

---

## 13. Summary

The Core SDK is a **well-architected, type-safe foundation** for the Executor system. Key strengths:

1. **Consistent abstraction** — All services use the same Effect-based patterns
2. **Plugin extensibility** — Clean extension points through the plugin system
3. **Testing support** — In-memory implementations for all services
4. **Type safety** — Branded IDs, Schema.Class, and Effect error tracking
5. **Separation of concerns** — Clear boundaries between tools, sources, secrets, and policies

The SDK enables the Executor to provide a **unified interface** for AI agents to interact with any API, while maintaining **security** through policies and **flexibility** through plugins.
