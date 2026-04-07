# Executor Core Config — Deep Dive Exploration

**Package:** `@executor/config`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/packages/core/config`  
**Total Files:** 7 TypeScript files  
**Total Lines:** 880 lines  

---

## 1. Module Overview

The Config package provides **configuration management** for the Executor system. It handles:

- **Config file parsing** — Load and validate `executor.jsonc` files
- **Schema validation** — Zod-based configuration schema
- **Config writing** — Update config files while preserving comments and formatting
- **Source config translation** — Convert between plugin format and config file format

### Key Responsibilities

1. **Config Loading** — Parse JSONC config files with validation
2. **Schema Definition** — Define valid configuration structure
3. **Config Writing** — Add/remove sources and secrets from config
4. **Format Preservation** — Maintain comments and formatting when updating

---

## 2. File Inventory

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/schema.ts` | 127 | Configuration schema definitions |
| 2 | `src/config-store.ts` | 190 | Config file-backed store wrappers |
| 3 | `src/load.ts` | 56 | Config file loading |
| 4 | `src/write.ts` | 172 | Config file writing |
| 5 | `src/index.ts` | 24 | Public exports |
| 6 | `src/config.test.ts` | 304 | Config tests |
| 7 | `vitest.config.ts` | 7 | Test configuration |

---

## 3. Key Exports

### Config Schema

```typescript
// schema.ts
export const ExecutorFileConfig = Schema.Struct({
  $schema: Schema.optional(Schema.String),
  name: Schema.optional(Schema.String),
  sources: Schema.optional(Schema.Array(SourceConfig)),
  secrets: Schema.optional(
    Schema.Record({ key: Schema.String, value: SecretMetadata }),
  ),
});

export type SourceConfig = typeof SourceConfig.Type;
// Union of: OpenApiSourceConfig | GraphqlSourceConfig | McpRemoteSourceConfig | McpStdioSourceConfig
```

### Source Config Types

```typescript
// OpenAPI
export const OpenApiSourceConfig = Schema.Struct({
  kind: Schema.Literal("openapi"),
  spec: Schema.String,
  baseUrl: Schema.optional(Schema.String),
  namespace: Schema.optional(Schema.String),
  headers: Schema.optional(ConfigHeaders),
});

// GraphQL
export const GraphqlSourceConfig = Schema.Struct({
  kind: Schema.Literal("graphql"),
  endpoint: Schema.String,
  introspectionJson: Schema.optional(Schema.NullOr(Schema.String)),
  namespace: Schema.optional(Schema.String),
  headers: Schema.optional(ConfigHeaders),
});

// MCP Remote
export const McpRemoteSourceConfig = Schema.Struct({
  kind: Schema.Literal("mcp"),
  transport: Schema.Literal("remote"),
  name: Schema.String,
  endpoint: Schema.String,
  remoteTransport: Schema.optional(Schema.Literal("streamable-http", "sse", "auto")),
  namespace: Schema.optional(Schema.String),
  queryParams: Schema.optional(StringMap),
  headers: Schema.optional(StringMap),
  auth: Schema.optional(McpAuthConfig),
});

// MCP Stdio
export const McpStdioSourceConfig = Schema.Struct({
  kind: Schema.Literal("mcp"),
  transport: Schema.Literal("stdio"),
  name: Schema.String,
  command: Schema.String,
  args: Schema.optional(Schema.Array(Schema.String)),
  env: Schema.optional(StringMap),
  cwd: Schema.optional(Schema.String),
  namespace: Schema.optional(Schema.String),
});
```

### Header Values

```typescript
// schema.ts
export const SECRET_REF_PREFIX = "secret-public-ref:";

export const ConfigHeaderValue = Schema.Union(
  Schema.String,
  Schema.Struct({
    value: Schema.String,
    prefix: Schema.optional(Schema.String),
  }),
);
```

### Config Store Wrapper

```typescript
// config-store.ts
export const withConfigFile = {
  openapi: <TStore>(inner: TStore, configPath: string, fsLayer: Layer) => TStore,
  graphql: <TStore>(inner: TStore, configPath: string, fsLayer: Layer) => TStore,
  mcp: <TStore>(inner: TStore, configPath: string, fsLayer: Layer) => TStore,
};
```

---

## 4. Line-by-Line Analysis

### Config Loading (`load.ts:19-56`)

```typescript
export const loadConfig = (
  path: string,
): Effect.Effect<
  ExecutorFileConfig | null,
  ConfigParseError | PlatformError,
  FileSystem.FileSystem
> =>
  Effect.gen(function* () {
    const fs = yield* FileSystem.FileSystem;

    const exists = yield* fs.exists(path);
    if (!exists) return null;

    const raw = yield* fs.readFileString(path);

    const errors: jsonc.ParseError[] = [];
    const parsed = jsonc.parse(raw, errors);

    if (errors.length > 0) {
      const msg = errors
        .map((e) => `offset ${e.offset}: ${jsonc.printParseErrorCode(e.error)}`)
        .join("; ");
      return yield* Effect.fail(new ConfigParseError(path, msg));
    }

    const decoded = yield* Schema.decodeUnknown(ExecutorFileConfig)(parsed).pipe(
      Effect.mapError((e) => new ConfigParseError(path, String(e))),
    );

    return decoded;
  });
```

**Key patterns:**

1. **JSONC parsing** — Uses `jsonc-parser` to support comments in config files
2. **Parse error collection** — Collects all parse errors with offsets
3. **Schema validation** — Validates parsed JSON against Effect Schema
4. **Null for missing** — Returns `null` if file doesn't exist (not an error)

### Adding Source to Config (`write.ts:34-82`)

```typescript
export const addSourceToConfig = (
  path: string,
  source: SourceConfig,
): Effect.Effect<void, PlatformError, FileSystem.FileSystem> =>
  Effect.gen(function* () {
    const fs = yield* FileSystem.FileSystem;
    let text = yield* readOrCreate(fs, path);

    // Ensure "sources" array exists
    let tree = jsonc.parseTree(text);
    let sourcesNode = tree ? jsonc.findNodeAtLocation(tree, ["sources"]) : undefined;

    if (!sourcesNode) {
      const edits = jsonc.modify(text, ["sources"], [source], {
        formattingOptions: FORMATTING,
      });
      text = jsonc.applyEdits(text, edits);
    } else {
      // Remove existing entry with same namespace to avoid duplicates
      const ns = "namespace" in source ? source.namespace : undefined;
      if (ns && sourcesNode.children) {
        for (let i = sourcesNode.children.length - 1; i >= 0; i--) {
          const child = sourcesNode.children[i]!;
          const nsNode = jsonc.findNodeAtLocation(child, ["namespace"]);
          if (nsNode && jsonc.getNodeValue(nsNode) === ns) {
            const edits = jsonc.modify(text, ["sources", i], undefined, {
              formattingOptions: FORMATTING,
            });
            text = jsonc.applyEdits(text, edits);
          }
        }
        // Re-parse after removals
        tree = jsonc.parseTree(text);
        sourcesNode = tree ? jsonc.findNodeAtLocation(tree, ["sources"]) : undefined;
      }

      const count = sourcesNode?.children?.length ?? 0;
      const edits = jsonc.modify(text, ["sources", count], source, {
        formattingOptions: FORMATTING,
      });
      text = jsonc.applyEdits(text, edits);
    }

    yield* fs.writeFileString(path, text);
  });
```

**Key patterns:**

1. **AST-based editing** — Uses `jsonc-parser` to edit without reformatting
2. **Duplicate prevention** — Removes existing entry with same namespace
3. **Formatting preservation** — Uses consistent formatting options
4. **Backwards iteration** — Walks array backwards for safe removal

### Header Translation (`config-store.ts:23-42`)

```typescript
const translateHeadersToConfig = (
  headers: Record<string, unknown> | undefined,
): Record<string, string | { value: string; prefix?: string }> | undefined => {
  if (!headers) return undefined;
  const result: Record<string, string | { value: string; prefix?: string }> = {};
  for (const [key, value] of Object.entries(headers)) {
    if (typeof value === "string") {
      result[key] = value;
    } else if (value && typeof value === "object" && "secretId" in value) {
      const v = value as { secretId: string; prefix?: string };
      const ref = `${SECRET_REF_PREFIX}${v.secretId}`;
      if (v.prefix) {
        result[key] = { value: ref, prefix: v.prefix };
      } else {
        result[key] = ref;
      }
    }
  }
  return result;
};
```

**Purpose:** Translates plugin format (with secretId references) to config file format (with secret-public-ref: prefix).

### Config File Wrapper (`config-store.ts:160-189`)

```typescript
export const withConfigFile = {
  openapi: <TStore extends StoreWithSource<{ namespace: string; name: string; config: any }>>(
    inner: TStore,
    configPath: string,
    fsLayer: Layer.Layer<FileSystem.FileSystem>,
  ): TStore => ({
    ...inner,
    putSource: wrapPutSource(inner.putSource, configPath, openApiToSourceConfig as any, fsLayer),
    removeSource: wrapRemoveSource(inner.removeSource, configPath, fsLayer),
  }) as TStore,

  graphql: <TStore>(...) => ({ ...inner, /* same pattern */ }) as TStore,

  mcp: <TStore>(...) => ({ ...inner, /* same pattern */ }) as TStore,
};
```

**Key pattern:** **Decorator pattern** — Wraps existing store to intercept `putSource` and `removeSource` for config file sync.

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Config Package                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Schema Layer                                      │   │
│  │                                                                       │   │
│  │  ExecutorFileConfig                                                  │   │
│  │  ├── sources: SourceConfig[]                                        │   │
│  │  │   ├── OpenApiSourceConfig                                       │   │
│  │  │   ├── GraphqlSourceConfig                                       │   │
│  │  │   ├── McpRemoteSourceConfig                                     │   │
│  │  │   └── McpStdioSourceConfig                                      │   │
│  │  └── secrets: { [id]: SecretMetadata }                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    File Operations                                   │   │
│  │                                                                       │   │
│  │  loadConfig()  ← jsonc-parser → parse + validate                    │   │
│  │  writeConfig() → jsonc-parser → edit preserving format              │   │
│  │  addSourceToConfig()                                                  │   │
│  │  removeSourceFromConfig()                                             │   │
│  │  addSecretToConfig()                                                  │   │
│  │  removeSecretFromConfig()                                             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Store Wrapper                                     │   │
│  │                                                                       │   │
│  │  withConfigFile.openapi(store, path, fs) → Store                    │   │
│  │  withConfigFile.graphql(store, path, fs) → Store                    │   │
│  │  withConfigFile.mcp(store, path, fs) → Store                        │   │
│  │                                                                       │   │
│  │  Intercepts putSource/removeSource to sync with config file         │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### Config Loading Flow

```
loadConfig(path)
    │
    ├──> 1. Check file existence
    │    └──> fs.exists(path)
    │         └──> Return null if not found
    │
    ├──> 2. Read file content
    │    └──> fs.readFileString(path)
    │
    ├──> 3. Parse JSONC
    │    ├──> jsonc.parse(raw, errors)
    │    └──> Fail with ConfigParseError if errors
    │
    ├──> 4. Validate schema
    │    ├──> Schema.decodeUnknown(ExecutorFileConfig)(parsed)
    │    └──> Fail with ConfigParseError if invalid
    │
    └──> 5. Return validated config
```

### Source Addition Flow

```
Plugin puts source → withConfigFile wrapper
    │
    ├──> 1. Call inner store.putSource()
    │    └──> Persists to KV/database
    │
    ├──> 2. Translate to config format
    │    └──> openApiToSourceConfig / graphqlToSourceConfig / mcpToSourceConfig
    │         ├──> Translate headers (secretId → secret-public-ref:)
    │         └──> Map fields to config schema
    │
    ├──> 3. Update config file
    │    ├──> jsonc.parseTree(text)
    │    ├──> Check for duplicate namespace
    │    ├──> jsonc.modify() to add entry
    │    └──> fs.writeFileString(path, text)
    │
    └──> 4. Swallow errors (best-effort sync)
```

---

## 7. Key Patterns

### JSONC for Human-Writable Config

```typescript
import * as jsonc from "jsonc-parser";

const parsed = jsonc.parse(raw, errors);
const tree = jsonc.parseTree(text);
const edits = jsonc.modify(text, ["sources", 0], newValue, { formattingOptions });
text = jsonc.applyEdits(text, edits);
```

**Benefits:**
1. **Comments support** — Users can document their config
2. **AST editing** — Modify without reformatting entire file
3. **Error details** — Parse errors include offset and error code

### Decorator Pattern for Store Wrapping

```typescript
export const withConfigFile = {
  openapi: (inner, configPath, fsLayer) => ({
    ...inner,
    putSource: wrapPutSource(inner.putSource, configPath, openApiToSourceConfig, fsLayer),
    removeSource: wrapRemoveSource(inner.removeSource, configPath, fsLayer),
  }),
};
```

**Benefits:**
1. **Separation of concerns** — Config file sync is orthogonal to storage
2. **Composability** — Wrap any store that implements the interface
3. **Best-effort sync** — Config file errors don't break core functionality

### Discriminated Unions for Source Types

```typescript
export const SourceConfig = Schema.Union(
  OpenApiSourceConfig,  // kind: "openapi"
  GraphqlSourceConfig,  // kind: "graphql"
  McpRemoteSourceConfig, // kind: "mcp", transport: "remote"
  McpStdioSourceConfig,  // kind: "mcp", transport: "stdio"
);
```

**Benefits:**
1. **Type narrowing** — TypeScript knows which fields exist based on `kind`
2. **Validation** — Schema validates correct fields for each type
3. **Extensibility** — Easy to add new source types

---

## 8. Integration Points

### Dependencies

| Package | Purpose |
|---------|---------|
| `@effect/platform` | FileSystem service |
| `effect` | Schema and Effect |
| `jsonc-parser` | JSON with comments parsing/editing |
| `@executor/sdk` | SDK types and schemas |

### Dependents

| Package | Relationship |
|---------|--------------|
| `@executor/storage-file` | Uses config wrapper for source sync |
| `@executor/apps/cli` | Loads config on startup |
| `@executor/apps/server` | Loads config on startup |
| `@executor/plugins/*` | Source plugins use config schema |

---

## 9. Error Handling

### ConfigParseError

```typescript
export class ConfigParseError {
  readonly _tag = "ConfigParseError";
  constructor(
    readonly path: string,
    readonly message: string,
  ) {}
}
```

### Error Handling in Loading

```typescript
const decoded = yield* Schema.decodeUnknown(ExecutorFileConfig)(parsed).pipe(
  Effect.mapError((e) => new ConfigParseError(path, String(e))),
);
```

### Best-Effort Config Sync

```typescript
yield* addSourceToConfig(configPath, toSourceConfig(source)).pipe(
  Effect.provide(fsLayer),
  Effect.catchAll(() => Effect.void), // Swallow errors
);
```

**Rationale:** Config file sync is secondary; primary persistence is in the store.

---

## 10. Config File Format

### Example `executor.jsonc`

```jsonc
{
  "$schema": "https://executor.dev/schema.json",
  "name": "my-project",
  
  "sources": [
    {
      "kind": "openapi",
      "spec": "https://api.github.com/openapi.json",
      "baseUrl": "https://api.github.com",
      "namespace": "github"
    },
    {
      "kind": "mcp",
      "transport": "stdio",
      "name": "filesystem",
      "command": "npx",
      "args": ["@modelcontextprotocol/server-filesystem", "/data"],
      "namespace": "files"
    },
    {
      "kind": "mcp",
      "transport": "remote",
      "name": "stripe",
      "endpoint": "https://api.stripe.com/mcp",
      "auth": {
        "kind": "header",
        "headerName": "Authorization",
        "secret": "secret-public-ref:stripe-key",
        "prefix": "Bearer "
      }
    }
  ],
  
  "secrets": {
    "stripe-key": {
      "name": "Stripe API Key",
      "provider": "keychain",
      "purpose": "Stripe API authentication"
    },
    "github-token": {
      "name": "GitHub Token",
      "purpose": "GitHub API rate limit"
    }
  }
}
```

---

## 11. Design Decisions

### Why JSONC Instead of JSON?

1. **Comments** — Users can document their configuration
2. **Standard format** — Used by VS Code, TypeScript, etc.
3. **Backward compatible** — Valid JSON is valid JSONC

### Why AST Editing Instead of Rewrite?

1. **Format preservation** — Maintains user's indentation, spacing, comments
2. **Minimal diffs** — Only changed lines show in git
3. **Better UX** — Config doesn't get reformatted on every change

### Why Best-Effort Config Sync?

1. **Graceful degradation** — Works even if config file is locked/read-only
2. **Primary vs secondary** — Store is authoritative; config file is convenience
3. **Error isolation** — Config file issues don't break core functionality

### Why Discriminated Unions?

1. **Type safety** — TypeScript narrows based on `kind` field
2. **Validation** — Ensures correct fields for each source type
3. **Documentation** — Clear what options exist for each type

---

## 12. Summary

The Config package provides **robust configuration management**:

1. **JSONC Support** — Human-writable config with comments
2. **Schema Validation** — Type-safe configuration with Effect Schema
3. **AST Editing** — Preserve formatting when updating
4. **Store Decoration** — Sync config file alongside primary storage
5. **Discriminated Types** — Clear source type definitions

The config layer enables **file-based configuration** while maintaining compatibility with database-backed storage through the decorator pattern.
