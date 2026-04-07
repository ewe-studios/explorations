# Executor Deep Dive Exploration

**Project:** Executor  
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor`  
**Created:** 2026-04-07  
**Lines:** ~15,000+ across 228 TypeScript files

---

## Executive Summary

Executor is an **integration layer for AI agents** — a unified runtime that exposes tools, secrets, and policies through a consistent API. It serves as a bridge between AI agents (like Claude Code, Cursor, etc.) and external APIs/services (OpenAPI, GraphQL, MCP, Google Discovery).

**Key Value Propositions:**
1. **Unified Tool Catalog** — Every tool from every source available to every agent
2. **Cross-Agent Sharing** — Shared authentication, policies, and tool definitions
3. **Type-Safe Runtime** — Full TypeScript/Effect-based type safety
4. **MCP Server** — Can run as an MCP server for any MCP-compatible agent
5. **Web UI** — Built-in web interface for managing sources and tools
6. **Plugin System** — Extensible via plugins for custom functionality

**Quick Start:**
```bash
npm install -g executor
executor web      # Start runtime + web UI at http://127.0.0.1:4788
executor mcp      # Start MCP endpoint for agents
```

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Executor Architecture                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Application Layer (Apps)                          │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐               │   │
│  │  │   CLI    │ │   Web    │ │  Server  │ │ Desktop  │               │   │
│  │  │  (Bun)   │ │  (React) │ │ (Express)│ │ (Electron)│              │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│         ┌────────────────────┼────────────────────┐                        │
│         │                    │                    │                        │
│         ▼                    ▼                    ▼                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Core Packages (SDK)                               │   │
│  │                                                                      │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │   │
│  │  │  @executor/sdk  │  │ @executor/api   │  │ @executor/      │     │   │
│  │  │                 │  │                 │  │ execution       │     │   │
│  │  │  - ToolRegistry │  │ - REST clients  │  │ - Code execution│     │   │
│  │  │  - SourceReg    │  │ - Auth handling │  │ - Elicitation   │     │   │
│  │  │  - SecretStore  │  │ - Error types   │  │ - Engine        │     │   │
│  │  │  - PolicyEngine │  │                 │  │                 │     │   │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────┘     │   │
│  │                                                                      │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │   │
│  │  │ @executor/      │  │ @executor/      │  │ @executor/      │     │   │
│  │  │ config          │  │ storage-file    │  │ runtime-quickjs │     │   │
│  │  │                 │  │                 │  │                 │     │   │
│  │  │ - Config store  │  │ - SQLite DB     │  │ - QuickJS VM    │     │   │
│  │  │ - Schema (Zod)  │  │ - Migrations    │  │ - Sandboxing    │     │   │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────┘     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│         ┌────────────────────┼────────────────────┐                        │
│         │                    │                    │                        │
│         ▼                    ▼                    ▼                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Plugin System                                     │   │
│  │                                                                      │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │   │
│  │  │  OpenAPI     │  │  GraphQL     │  │  MCP         │              │   │
│  │  │  Plugin      │  │  Plugin      │  │  Plugin      │              │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘              │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │   │
│  │  │  Google      │  │  Secret      │  │  Custom      │              │   │
│  │  │  Discovery   │  │  Providers   │  │  Plugins     │              │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│              ┌───────────────────────────────┐                             │
│              │   External Sources            │                             │
│              │   - REST APIs (OpenAPI)       │                             │
│              │   - GraphQL APIs              │                             │
│              │   - MCP Servers               │                             │
│              │   - Google Discovery          │                             │
│              │   - Custom Sources            │                             │
│              └───────────────────────────────┘                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Directory Structure

```
executor/
├── apps/                           # Application entry points
│   ├── cli/                        # CLI application (Bun runtime)
│   │   ├── bin/
│   │   │   └── executor.ts         # CLI entry point
│   │   └── src/
│   │       ├── main.ts             # Main CLI logic
│   │       ├── build.ts            # Build configuration
│   │       ├── release.ts          # Release handling
│   │       └── embedded-web-ui.*   # Embedded web UI assets
│   ├── web/                        # React web UI
│   │   └── src/
│   │       ├── App.tsx             # Main React component
│   │       ├── main.tsx            # React entry point
│   │       ├── components/         # UI components
│   │       └── pages/              # Application pages
│   ├── server/                     # Backend server
│   │   └── src/
│   │       ├── index.ts            # Server entry
│   │       ├── main.ts             # Main server logic
│   │       ├── mcp.ts              # MCP server handling
│   │       ├── handlers/           # Request handlers
│   │       └── services/           # Business logic services
│   └── desktop/                    # Electron desktop app
│       └── src/
│           ├── main.ts             # Electron main process
│           └── preload.ts          # Preload script
│
├── packages/                       # Core packages (monorepo)
│   ├── clients/
│   │   └── react/                  # React client library
│   │       └── src/
│   │           ├── client.ts       # API client
│   │           ├── atoms.ts        # UI state atoms
│   │           └── use-scope.ts    # Scope management hook
│   │
│   ├── core/
│   │   ├── api/                    # API client layer
│   │   │   └── src/
│   │   │       ├── api.ts          # Core API client
│   │   │       ├── errors.ts       # Error types
│   │   │       └── executions/
│   │   │       ├── scope/
│   │   │       ├── secrets/
│   │   │       ├── sources/
│   │   │       └── tools/
│   │   │
│   │   ├── config/                 # Configuration management
│   │   │   └── src/
│   │   │       ├── config-store.ts # Config persistence
│   │   │       ├── load.ts         # Config loading
│   │   │       ├── schema.ts       # Zod schemas
│   │   │       └── write.ts        # Config writing
│   │   │
│   │   ├── execution/              # Code execution engine
│   │   │   └── src/
│   │   │       ├── engine.ts       # Execution engine
│   │   │       ├── description.ts  # Tool description building
│   │   │       ├── tool-invoker.ts # Tool invocation
│   │   │       └── errors.ts       # Execution errors
│   │   │
│   │   ├── sdk/                    # Core SDK (main public API)
│   │   │   └── src/
│   │   │       ├── executor.ts     # Executor type + factory
│   │   │       ├── tools.ts        # Tool types + registry
│   │   │       ├── sources.ts      # Source types + registry
│   │   │       ├── secrets.ts      # Secret management
│   │   │       ├── policies.ts     # Policy engine
│   │   │       ├── scope.ts        # Scope management
│   │   │       ├── plugin.ts       # Plugin system
│   │   │       ├── elicitation.ts  # Elicitation handling
│   │   │       └── ids.ts          # ID types
│   │   │
│   │   ├── storage-file/           # File-based storage (SQLite)
│   │   │   └── src/
│   │   │       ├── schema.ts       # Database schema
│   │   │       ├── secret-store.ts # Secret storage
│   │   │       ├── policy-engine.ts# Policy storage
│   │   │       └── migrations/     # Database migrations
│   │   │
│   │   └── runtime-quickjs/        # QuickJS runtime for code execution
│   │       └── src/
│   │           └── index.ts        # QuickJS executor
│   │
│   ├── hosts/                      # Host implementations
│   ├── kernel/                     # Core kernel logic
│   ├── plugins/                    # Plugin implementations
│   │   ├── openapi/                # OpenAPI source plugin
│   │   ├── graphql/                # GraphQL source plugin
│   │   ├── mcp/                    # MCP source plugin
│   │   └── google-discovery/       # Google Discovery plugin
│   │
│   └── ui/                         # Shared UI components
│
├── tests/                          # Test files
├── .mcp.json                       # MCP configuration
├── package.json                    # Root package.json
├── turbo.json                      # Turborepo configuration
└── tsconfig.json                   # TypeScript configuration
```

---

## Core Concepts

### 1. Scopes

Scopes are the fundamental organizational unit in Executor. Each scope represents an isolated context with its own:
- Tool registry
- Source configurations
- Secret store
- Policy definitions

```typescript
// From packages/core/sdk/src/scope.ts
export interface Scope {
  readonly id: ScopeId;
  readonly name: string;
  readonly createdAt: Date;
}
```

Scopes enable:
- **Multi-tenancy** — Different projects/teams have separate tool catalogs
- **Policy isolation** — Policies apply per-scope
- **Secret isolation** — Secrets are scoped and not shared across contexts

### 2. Tools

Tools are the primary abstraction for AI agent capabilities. Each tool has:

```typescript
// From packages/core/sdk/src/tools.ts
export interface ToolMetadata {
  readonly id: ToolId;
  readonly sourceId: SourceId;
  readonly name: string;
  readonly description: string;
  readonly path: string;      // e.g., "github.issues.list"
  readonly namespace: string; // e.g., "github"
}

export interface ToolSchema {
  readonly id: ToolId;
  readonly metadata: ToolMetadata;
  readonly inputSchema: JsonSchema;
  readonly outputSchema: JsonSchema;
  readonly annotations?: ToolAnnotations;
}

export interface ToolAnnotations {
  readonly requiresApproval?: boolean;
  readonly approvalDescription?: string;
  readonly readOnly?: boolean;
  readonly destructiveHint?: boolean;
  readonly idempotentHint?: boolean;
}
```

**Tool Invocation:**
```typescript
// From packages/core/sdk/src/tools.ts
export interface ToolRegistry {
  readonly list: (filter?: ToolListFilter) => Effect.Effect<readonly ToolMetadata[]>;
  readonly schema: (toolId: ToolId) => Effect.Effect<ToolSchema, ToolNotFoundError>;
  readonly definitions: () => Effect.Effect<Record<string, unknown>>;
  readonly invoke: (
    toolId: ToolId,
    args: unknown,
    options: InvokeOptions,
  ) => Effect.Effect<ToolInvocationResult, ToolInvocationError>;
}
```

### 3. Sources

Sources are external API integrations that provide tools:

```typescript
// From packages/core/sdk/src/sources.ts
export interface Source {
  readonly id: SourceId;
  readonly name: string;
  readonly kind: SourceKind;  // 'openapi' | 'graphql' | 'mcp' | 'google-discovery'
  readonly specUrl?: string;
  readonly baseUrl?: string;
  readonly auth: SourceAuthConfig;
  readonly status: 'active' | 'error' | 'refreshing';
}

export type SourceKind = 'openapi' | 'graphql' | 'mcp' | 'google-discovery' | 'custom';

export type SourceAuthConfig =
  | { kind: 'none' }
  | { kind: 'bearer'; token: string }
  | { kind: 'basic'; username: string; password: string }
  | { kind: 'oauth2'; provider: OAuth2Provider }
  | { kind: 'secret'; secretId: SecretId };
```

### 4. Secrets

Secrets are sensitive values (API keys, tokens) used for authentication:

```typescript
// From packages/core/sdk/src/secrets.ts
export interface SecretRef {
  readonly id: SecretId;
  readonly name: string;
  readonly scopeId: ScopeId;
  readonly provider: string;
  readonly status: 'resolved' | 'missing';
}

export interface SecretProvider {
  readonly key: string;
  readonly get: (ref: SecretRef) => Effect.Effect<string, SecretNotFoundError>;
  readonly set: (ref: SecretRef, value: string) => Effect.Effect<void>;
  readonly delete: (ref: SecretRef) => Effect.Effect<void>;
}
```

**Built-in Providers:**
- `env` — Environment variables
- `file` — File-based storage (encrypted)
- Custom providers via plugin system

### 5. Policies

Policies control tool access and execution rules:

```typescript
// From packages/core/sdk/src/policies.ts
export interface Policy {
  readonly id: PolicyId;
  readonly scopeId: ScopeId;
  readonly name: string;
  readonly description: string;
  readonly condition: PolicyCondition;
  readonly action: 'allow' | 'deny' | 'require-approval';
  readonly createdAt: Date;
}

export interface PolicyCondition {
  readonly toolId?: string;       // Match specific tool
  readonly toolPattern?: string;  // Glob pattern for tool IDs
  readonly namespace?: string;    // Match by namespace
}
```

### 6. Elicitation

Elicitation is the mechanism for pausing execution to request user input:

```typescript
// From packages/core/sdk/src/elicitation.ts
export type ElicitationRequest = FormElicitation | UrlElicitation;

export class FormElicitation {
  readonly _tag = 'FormElicitation';
  readonly message: string;
  readonly requestedSchema: Record<string, unknown>;
}

export class UrlElicitation {
  readonly _tag = 'UrlElicitation';
  readonly message: string;
  readonly url: string;
  readonly completionIndicator: 'callback' | 'polling';
}

export type ElicitationHandler = (
  ctx: ElicitationContext,
) => Effect.Effect<ElicitationResponse>;

export interface ElicitationContext {
  readonly toolId: ToolId;
  readonly args: unknown;
  readonly request: ElicitationRequest;
}
```

---

## Executor SDK API

### Executor Type Definition

```typescript
// packages/core/sdk/src/executor.ts (lines 44-106)
export type Executor<
  TPlugins extends readonly ExecutorPlugin<string, object>[] = [],
> = {
  readonly scope: Scope;

  readonly tools: {
    readonly list: (filter?: ToolListFilter) => Effect.Effect<readonly ToolMetadata[]>;
    readonly schema: (toolId: string) => Effect.Effect<ToolSchema, ToolNotFoundError>;
    readonly definitions: () => Effect.Effect<Record<string, unknown>>;
    readonly invoke: (
      toolId: string,
      args: unknown,
      options: InvokeOptions,
    ) => Effect.Effect<
      ToolInvocationResult,
      | ToolNotFoundError
      | ToolInvocationError
      | PolicyDeniedError
      | ElicitationDeclinedError
    >;
  };

  readonly sources: {
    readonly list: () => Effect.Effect<readonly Source[]>;
    readonly remove: (sourceId: string) => Effect.Effect<void>;
    readonly refresh: (sourceId: string) => Effect.Effect<void>;
    readonly detect: (url: string) => Effect.Effect<readonly SourceDetectionResult[]>;
  };

  readonly policies: {
    readonly list: () => Effect.Effect<readonly Policy[]>;
    readonly add: (policy: Omit<Policy, "id" | "createdAt">) => Effect.Effect<Policy>;
    readonly remove: (policyId: string) => Effect.Effect<boolean>;
  };

  readonly secrets: {
    readonly list: () => Effect.Effect<readonly SecretRef[]>;
    readonly resolve: (secretId: SecretId) => Effect.Effect<string, SecretNotFoundError | SecretResolutionError>;
    readonly status: (secretId: SecretId) => Effect.Effect<"resolved" | "missing">;
    readonly set: (input: Omit<SetSecretInput, "scopeId">) => Effect.Effect<SecretRef, SecretResolutionError>;
    readonly remove: (secretId: SecretId) => Effect.Effect<boolean, SecretNotFoundError>;
    readonly addProvider: (provider: SecretProvider) => Effect.Effect<void>;
    readonly providers: () => Effect.Effect<readonly string[]>;
  };

  readonly close: () => Effect.Effect<void>;
} & PluginExtensions<TPlugins>;
```

### createExecutor Factory

```typescript
// packages/core/sdk/src/executor.ts (lines 132-229)
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
            // 1. Check policy
            yield* policies.check({ scopeId: scope.id, toolId: tid });

            // 2. Resolve annotations from plugins
            const annotations = yield* tools.resolveAnnotations(tid);
            
            // 3. Handle approval requirement via elicitation
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

            // 4. Invoke tool
            return yield* tools.invoke(tid, args, options);
          });
        },
      },

      // ... sources, policies, secrets implementations

      close: () =>
        Effect.gen(function* () {
          for (const handle of handles.values()) {
            if (handle.close) yield* handle.close();
          }
        }),
    };

    return Object.assign(base, extensions) as Executor<TPlugins>;
  });
```

---

## Execution Engine

The Execution Engine (`packages/core/execution/src/engine.ts`) provides code execution capabilities:

### Engine Interface

```typescript
// packages/core/execution/src/engine.ts (lines 252-278)
export type ExecutionEngine = {
  /**
   * Execute code with elicitation handled inline.
   * Use when host supports elicitation (e.g. MCP with elicitation capability).
   */
  readonly execute: (
    code: string,
    options: { readonly onElicitation: ElicitationHandler },
  ) => Promise<ExecuteResult>;

  /**
   * Execute code, intercepting first elicitation as pause point.
   * Returns either completed result or paused execution that can be resumed.
   */
  readonly executeWithPause: (code: string) => Promise<ExecutionResult>;

  /**
   * Resume a paused execution.
   */
  readonly resume: (executionId: string, response: ResumeResponse) => Promise<ExecuteResult | null>;

  /**
   * Get dynamic tool description (workflow + namespaces).
   */
  readonly getDescription: () => Promise<string>;
};
```

### Execution Result Types

```typescript
// packages/core/execution/src/engine.ts (lines 31-45)
export type ExecutionResult =
  | { readonly status: "completed"; readonly result: ExecuteResult }
  | { readonly status: "paused"; readonly execution: PausedExecution };

export type PausedExecution = {
  readonly id: string;
  readonly elicitationContext: ElicitationContext;
  readonly resolve: (response: ElicitationResponse) => void;
  readonly completion: Promise<ExecuteResult>;
};

export type ResumeResponse = {
  readonly action: "accept" | "decline" | "cancel";
  readonly content?: Record<string, unknown>;
};
```

### Execution Flow

```typescript
// packages/core/execution/src/engine.ts (lines 283-346)
export const createExecutionEngine = (config: ExecutionEngineConfig): ExecutionEngine => {
  const { executor } = config;
  const codeExecutor = config.codeExecutor ?? makeQuickJsExecutor();
  const pausedExecutions = new Map<string, PausedExecution>();
  let nextId = 0;

  return {
    execute: async (code, options) => {
      const invoker = makeFullInvoker(executor, {
        onElicitation: options.onElicitation,
      });
      return runEffect(codeExecutor.execute(code, invoker));
    },

    executeWithPause: async (code) => {
      let signalPause: ((paused: PausedExecution) => void) | null = null;
      const pausePromise = new Promise<PausedExecution>((resolve) => {
        signalPause = resolve;
      });

      const elicitationHandler: ElicitationHandler = (ctx: ElicitationContext) =>
        Effect.async((resume) => {
          const id = `exec_${++nextId}`;
          const paused: PausedExecution = {
            id,
            elicitationContext: ctx,
            resolve: (response) => resume(Effect.succeed(response)),
            completion: undefined as unknown as Promise<ExecuteResult>,
          };
          pausedExecutions.set(id, paused);
          signalPause!(paused);
        });

      const invoker = makeFullInvoker(executor, { onElicitation: elicitationHandler });
      const completionPromise = runEffect(codeExecutor.execute(code, invoker));

      // Race: either execution completes, or it pauses for elicitation
      const result = await Promise.race([
        completionPromise.then((r) => ({ kind: "completed" as const, result: r })),
        pausePromise.then((p) => ({ kind: "paused" as const, execution: p })),
      ]);

      if (result.kind === "completed") {
        return { status: "completed", result: result.result };
      }

      (result.execution as { completion: Promise<ExecuteResult> }).completion = completionPromise;
      return { status: "paused", execution: result.execution };
    },

    resume: async (executionId, response) => {
      const paused = pausedExecutions.get(executionId);
      if (!paused) return null;

      pausedExecutions.delete(executionId);
      paused.resolve({ action: response.action, content: response.content });
      return paused.completion;
    },

    getDescription: () => runEffect(buildExecuteDescription(executor)),
  };
};
```

### Result Formatting

```typescript
// packages/core/execution/src/engine.ts (lines 51-91)
const MAX_PREVIEW_CHARS = 30_000;

const truncate = (value: string, max: number): string =>
  value.length > max
    ? `${value.slice(0, max)}\n... [truncated ${value.length - max} chars]`
    : value;

export const formatExecuteResult = (result: ExecuteResult): {
  text: string;
  structured: Record<string, unknown>;
  isError: boolean;
} => {
  const resultText =
    result.result != null
      ? typeof result.result === "string"
        ? result.result
        : JSON.stringify(result.result, null, 2)
      : null;

  const logText =
    result.logs && result.logs.length > 0 ? result.logs.join("\n") : null;

  if (result.error) {
    const parts = [`Error: ${result.error}`, ...(logText ? [`\nLogs:\n${logText}`] : [])];
    return {
      text: truncate(parts.join("\n"), MAX_PREVIEW_CHARS),
      structured: { status: "error", error: result.error, logs: result.logs ?? [] },
      isError: true,
    };
  }

  const parts = [
    ...(resultText ? [truncate(resultText, MAX_PREVIEW_CHARS)] : ["(no result)"]),
    ...(logText ? [`\nLogs:\n${logText}`] : []),
  ];
  return {
    text: parts.join("\n"),
    structured: { status: "completed", result: result.result ?? null, logs: result.logs ?? [] },
    isError: false,
  };
};
```

---

## Plugin System

### Plugin Interface

```typescript
// packages/core/sdk/src/plugin.ts
export interface ExecutorPlugin<TKey extends string, TExtension extends object> {
  readonly key: TKey;
  readonly init: (ctx: PluginContext) => Effect.Effect<PluginHandle<TExtension>>;
}

export interface PluginContext {
  readonly scope: Scope;
  readonly tools: ToolRegistry;
  readonly sources: SourceRegistry;
  readonly secrets: SecretStore;
  readonly policies: PolicyEngine;
}

export interface PluginHandle<TExtension extends object> {
  readonly extension: TExtension;
  readonly close?: () => Effect.Effect<void>;
}
```

### Plugin Types

| Plugin | Purpose | Package |
|--------|---------|---------|
| OpenAPI | Parse OpenAPI specs, create REST tool invokers | `@executor/plugins-openapi` |
| GraphQL | Parse GraphQL schemas, create query tools | `@executor/plugins-graphql` |
| MCP | Connect to MCP servers as tool sources | `@executor/plugins-mcp` |
| Google Discovery | Google APIs discovery and auth | `@executor/plugins-google` |
| Secret Providers | Custom secret storage backends | Various |

---

## CLI Application

### Entry Point

```typescript
// apps/cli/bin/executor.ts
#!/usr/bin/env bun

import { main } from '../src/main';

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
```

### Main CLI Commands

```typescript
// apps/cli/src/main.ts (simplified)
export async function main() {
  const args = process.argv.slice(2);
  const command = args[0];

  switch (command) {
    case 'web':
      await startWebUI();
      break;
    case 'mcp':
      await startMCPServer();
      break;
    case 'call':
      await executeCode(args);
      break;
    case 'resume':
      await resumeExecution(args);
      break;
    case 'up':
      await startDaemon();
      break;
    case 'down':
      await stopDaemon();
      break;
    case 'status':
      await showStatus();
      break;
    default:
      showHelp();
  }
}
```

### CLI Commands Reference

| Command | Description |
|---------|-------------|
| `executor web` | Start runtime + web UI at http://127.0.0.1:4788 |
| `executor mcp` | Start MCP server endpoint |
| `executor call --file script.ts` | Execute TypeScript file |
| `executor call '<code>'` | Execute inline TypeScript code |
| `executor call --stdin` | Execute code from stdin |
| `executor resume --execution-id <id>` | Resume paused execution |
| `executor up` | Start daemon mode |
| `executor down` | Stop daemon |
| `executor status` | Show daemon status |

---

## Server Application

### MCP Server

```typescript
// apps/server/src/mcp.ts
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { createExecutor } from '@executor/sdk';

export async function startMCPServer() {
  const executor = await createExecutor({ /* config */ });
  
  const server = new Server(
    { name: 'executor', version: '1.0.0' },
    { capabilities: { tools: {}, resources: {} } }
  );

  // List available tools
  server.setRequestHandler('tools/list', async () => {
    const tools = await Effect.runPromise(executor.tools.list());
    return {
      tools: tools.map(t => ({
        name: t.path,
        description: t.description,
        inputSchema: (await Effect.runPromise(executor.tools.schema(t.id))).inputSchema,
      })),
    };
  });

  // Call tool
  server.setRequestHandler('tools/call', async (request) => {
    const { name, arguments: args } = request.params;
    const result = await Effect.runPromise(
      executor.tools.invoke(name, args ?? {}, { onElicitation: 'accept-all' })
    );
    return {
      content: [
        { type: 'text', text: JSON.stringify(result, null, 2) }
      ],
    };
  });

  const transport = new StdioServerTransport();
  await server.connect(transport);
}
```

---

## Data Flow

### Tool Discovery Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   AI Agent   │────▶│   Executor   │────▶│   Source     │
│  (MCP/Cursor)│     │     SDK      │     │   Plugin     │
└──────────────┘     └──────────────┘     └──────────────┘
       │                    │                    │
       │  tools/list        │                    │
       │───────────────────▶│                    │
       │                    │  fetchSpec()       │
       │                    │───────────────────▶│
       │                    │                    │
       │                    │  OpenAPI spec      │
       │                    │◀───────────────────│
       │                    │                    │
       │                    │  ToolMetadata[]    │
       │◀───────────────────│                    │
       │                    │                    │
```

### Tool Invocation Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   AI Agent   │────▶│   Executor   │────▶│  Policy      │
│              │     │     SDK      │     │  Engine      │
└──────────────┘     └──────────────┘     └──────────────┘
       │                    │                    │
       │  tools/call        │                    │
       │───────────────────▶│                    │
       │                    │  check()           │
       │                    │───────────────────▶│
       │                    │                    │
       │                    │  allowed           │
       │                    │◀───────────────────│
       │                    │                    │
       │                    │  (requiresApproval)│
       │                    │─────────┐          │
       │                    │         │          │
       │                    │◀────────┘          │
       │  elicitation       │                    │
       │◀───────────────────│                    │
       │                    │                    │
       │  accept/decline    │                    │
       │───────────────────▶│                    │
       │                    │  invoke()          │
       │                    │───────────────────▶│ Source Plugin
       │                    │                    │
       │  tool result       │  HTTP request      │
       │◀───────────────────│◀───────────────────│
       │                    │                    │
```

### Execution with Elicitation Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Agent      │────▶│  Execution   │────▶│   Executor   │
│              │     │    Engine    │     │     SDK      │
└──────────────┘     └──────────────┘     └──────────────┘
       │                    │                    │
       │  execute(code)     │                    │
       │───────────────────▶│                    │
       │                    │  code.execute()    │
       │                    │───────────────────▶│
       │                    │                    │
       │                    │  tool.invoke()     │
       │                    │───────────────────▶│
       │                    │                    │
       │                    │  (requires approval)
       │                    │  elicitation request
       │                    │◀───────────────────│
       │  paused            │                    │
       │◀───────────────────│                    │
       │                    │                    │
       │  resume(id, accept)│                    │
       │───────────────────▶│                    │
       │                    │  resolve()         │
       │                    │───────────────────▶│
       │                    │                    │
       │  result            │  tool result       │
       │◀───────────────────│◀───────────────────│
       │                    │                    │
```

---

## Integration Points

### MCP Integration

Executor can run as an MCP server, exposing all tools through the MCP protocol:

```json
// .mcp.json for Claude Code / Cursor
{
  "mcpServers": {
    "executor": {
      "command": "executor",
      "args": ["mcp"]
    }
  }
}
```

### React Client

React components for embedding Executor in web applications:

```typescript
// packages/clients/react/src/client.ts
import { createExecutorClient } from '@executor/clients-react';

const client = createExecutorClient({ baseUrl: 'http://localhost:4788' });

// In React component
function ToolTree() {
  const tools = useQuery(client.tools.list);
  return (
    <ul>
      {tools.map(tool => (
        <li key={tool.id}>{tool.name}</li>
      ))}
    </ul>
  );
}
```

---

## Key Design Decisions

### 1. Effect.ts for Type Safety

Executor uses [Effect.ts](file:///home/darkvoid/Boxxed/%40formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/node_modules/effect) throughout for:
- **Error handling** — Typed error channels
- **Dependency injection** — Context-based DI
- **Concurrency** — Fibers and structured concurrency
- **Resource management** — Scoped resources

**Why Effect?**
```typescript
// Instead of try/catch and implicit errors:
async function invokeTool(toolId: string, args: unknown) {
  // Error handling is implicit and untyped
}

// Effect provides explicit error typing:
function invokeTool(
  toolId: ToolId,
  args: unknown,
): Effect.Effect<
  ToolInvocationResult,
  | ToolNotFoundError
  | ToolInvocationError
  | PolicyDeniedError
>
```

### 2. Plugin Architecture

Plugins are initialized with shared services and can extend the Executor:

```typescript
const plugin: ExecutorPlugin<'my-plugin', MyExtension> = {
  key: 'my-plugin',
  init: async (ctx) => ({
    extension: {
      myCustomMethod: () => { /* ... */ }
    },
  }),
};

const executor = await createExecutor({
  /* ... */
  plugins: [plugin],
});

// Plugin methods are available on executor
executor.myPlugin.myCustomMethod();
```

### 3. Scope-Based Isolation

Scopes provide multi-tenancy:
- Each scope has isolated tools, secrets, and policies
- Scopes can be used for different projects or environments
- Switching scopes changes the available tool catalog

### 4. Elicitation Protocol

Elicitation provides a structured way to pause execution for user input:
- **Form Elicitation** — Request structured data via forms
- **URL Elicitation** — Redirect to external auth/approval flows
- **Pause/Resume** — Executions can be paused and resumed later

---

## File Inventory

### Core SDK (packages/core/sdk/)

| File | Lines | Purpose |
|------|-------|---------|
| `executor.ts` | 230 | Executor type definition and factory |
| `tools.ts` | ~400 | Tool types, registry, invocation |
| `sources.ts` | ~300 | Source types and management |
| `secrets.ts` | ~200 | Secret store and providers |
| `policies.ts` | ~250 | Policy engine and conditions |
| `scope.ts` | ~100 | Scope management |
| `plugin.ts` | ~150 | Plugin system types |
| `elicitation.ts` | ~200 | Elicitation handling |
| `ids.ts` | ~50 | ID type definitions |
| `errors.ts` | ~200 | Error types |

### Execution Engine (packages/core/execution/)

| File | Lines | Purpose |
|------|-------|---------|
| `engine.ts` | 347 | Execution engine with pause/resume |
| `tool-invoker.ts` | ~200 | Tool invocation for execution |
| `description.ts` | ~150 | Tool description building |
| `errors.ts` | ~100 | Execution error types |

### Storage (packages/core/storage-file/)

| File | Lines | Purpose |
|------|-------|---------|
| `schema.ts` | ~300 | SQLite database schema |
| `secret-store.ts` | ~200 | Secret persistence |
| `policy-engine.ts` | ~250 | Policy persistence |
| `plugin-kv.ts` | ~100 | Plugin key-value storage |
| `migrations/` | ~500 | Database migrations |

### Applications

| App | Files | Purpose |
|-----|-------|---------|
| CLI | ~10 | Command-line interface |
| Web | ~20 | React web UI |
| Server | ~15 | Backend server + MCP |
| Desktop | ~5 | Electron desktop app |

---

## Related Files

**In This Repo:**
- None (Executor is in a separate repository)

**External:**
- [Effect.ts Documentation](https://effect.website/)
- [MCP Specification](https://modelcontextprotocol.io/)
- [OpenAPI Specification](https://swagger.io/specification/)

---

*Deep dive created: 2026-04-07*
