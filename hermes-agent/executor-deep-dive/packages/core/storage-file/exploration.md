# Executor Core Storage-File — Deep Dive Exploration

**Package:** `@executor/storage-file`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/packages/core/storage-file`  
**Total Files:** 11 TypeScript files  
**Total Lines:** 1,031 lines  

---

## 1. Module Overview

The Storage-File package provides **SQLite-backed persistence** for the Executor system. It implements:

- **KV-based storage** — Single `kv` table for all data
- **SDK service implementations** — ToolRegistry, SecretStore, PolicyEngine
- **Migration system** — Schema evolution with @effect/sql Migrator
- **Scoped isolation** -- Per-folder namespace prefixing

### Key Responsibilities

1. **SQLite Persistence** — Store all executor data in SQLite
2. **KV Abstraction** — Uniform key-value interface for all collections
3. **Service Factories** — Build SDK services from KV
4. **Scope Isolation** — Multi-tenant support via namespace prefixing

---

## 2. File Inventory

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/index.ts` | 103 | Main exports and config builder |
| 2 | `src/schema.ts` | 13 | Database schema and migrations |
| 3 | `src/tool-registry.ts` | 233 | KV-backed tool registry |
| 4 | `src/secret-store.ts` | 176 | KV-backed secret store (refs only) |
| 5 | `src/policy-engine.ts` | 65 | KV-backed policy engine |
| 6 | `src/plugin-kv.ts` | 122 | SQLite and in-memory KV implementations |
| 7 | `src/sql-utils.ts` | 18 | SQL error absorption |
| 8 | `src/migrations/index.ts` | 13 | Migration loader |
| 9 | `src/migrations/0001_initial.ts` | 19 | Initial schema migration |
| 10 | `src/index.test.ts` | 262 | Integration tests |
| 11 | `vitest.config.ts` | 7 | Test configuration |

---

## 3. Key Exports

### KV Interface

```typescript
// plugin-kv.ts
export interface Kv {
  readonly get: (namespace: string, key: string) => Effect.Effect<string | null>;
  readonly set: (namespace: string, key: string, value: string) => Effect.Effect<void>;
  readonly delete: (namespace: string, key: string) => Effect.Effect<boolean>;
  readonly list: (namespace: string) => Effect.Effect<readonly { key: string; value: string }[]>;
  readonly deleteAll: (namespace: string) => Effect.Effect<number>;
  readonly withTransaction?: <A, E>(effect: Effect.Effect<A, E, never>) => Effect.Effect<A, E, never>;
}
```

### Scoped KV

```typescript
// From SDK
export type ScopedKv = {
  readonly get: (key: string) => Effect.Effect<string | null>;
  readonly set: (key: string, value: string) => Effect.Effect<void>;
  readonly delete: (key: string) => Effect.Effect<boolean>;
  readonly list: () => Effect.Effect<readonly { key: string; value: string }[]>;
  readonly deleteAll: () => Effect.Effect<number>;
  readonly withTransaction?: <A, E>(effect: Effect.Effect<A, E, never>) => Effect.Effect<A, E, never>;
};
```

### Service Factories

```typescript
// index.ts
export { makeSqliteKv, makeInMemoryKv } from "./plugin-kv";
export { makeKvToolRegistry } from "./tool-registry";
export { makeKvSecretStore } from "./secret-store";
export { makeKvPolicyEngine } from "./policy-engine";
export { migrate } from "./schema";

export const makeKvConfig = <
  const TPlugins extends readonly ExecutorPlugin<string, object>[] = [],
>(
  kv: Kv,
  options: { readonly cwd: string; readonly plugins?: TPlugins },
): ExecutorConfig<TPlugins> => {
  const scopeId = makeScopeId(cwd);
  const scope: Scope = {
    id: ScopeId.make(scopeId),
    name: cwd,
    createdAt: new Date(),
  };

  const ns = (name: string) => `${cwd}::${name}`;

  return {
    scope,
    tools: makeKvToolRegistry(scopeKv(kv, ns("tools")), scopeKv(kv, ns("defs"))),
    sources: makeInMemorySourceRegistry(),
    secrets: makeKvSecretStore(scopeKv(kv, ns("secrets"))),
    policies: makeKvPolicyEngine(scopeKv(kv, ns("policies")), scopeKv(kv, ns("meta"))),
    plugins: options.plugins,
  };
};
```

---

## 4. Line-by-Line Analysis

### Scope ID Generation (`index.ts:38-42`)

```typescript
const makeScopeId = (cwd: string): string => {
  const folder = basename(cwd) || cwd;
  const hash = createHash("sha256").update(cwd).digest("hex").slice(0, 8);
  return `${folder}-${hash}`;
};
```

**Purpose:** Generates a unique, URL-safe scope ID from the working directory.

**Example:** `/home/user/my-project` → `my-project-a1b2c3d4`

### SQLite KV Implementation (`plugin-kv.ts:21-82`)

```typescript
export const makeSqliteKv = (sql: SqlClient.SqlClient): Kv => ({
  get: (namespace, key) =>
    absorbSql(Effect.gen(function* () {
      const rows = yield* sql<KvRow>`
        SELECT value FROM kv WHERE namespace = ${namespace} AND key = ${key}
      `;
      return rows[0]?.value ?? null;
    })),

  set: (namespace, key, value) =>
    absorbSql(sql`
      INSERT OR REPLACE INTO kv (namespace, key, value)
      VALUES (${namespace}, ${key}, ${value})
    `.pipe(Effect.asVoid)),

  delete: (namespace, key) =>
    absorbSql(Effect.gen(function* () {
      const before = yield* sql<{ c: number }>`
        SELECT COUNT(*) as c FROM kv WHERE namespace = ${namespace} AND key = ${key}
      `;
      yield* sql`DELETE FROM kv WHERE namespace = ${namespace} AND key = ${key}`;
      return (before[0]?.c ?? 0) > 0;
    })),

  list: (namespace) =>
    absorbSql(Effect.gen(function* () {
      const rows = yield* sql<KvRow>`
        SELECT key, value FROM kv WHERE namespace = ${namespace}
      `;
      return rows.map((r) => ({ key: r.key, value: r.value }));
    })),

  deleteAll: (namespace) =>
    absorbSql(Effect.gen(function* () {
      const before = yield* sql<{ c: number }>`
        SELECT COUNT(*) as c FROM kv WHERE namespace = ${namespace}
      `;
      yield* sql`DELETE FROM kv WHERE namespace = ${namespace}`;
      return before[0]?.c ?? 0;
    })),

  withTransaction: <A, E>(effect: Effect.Effect<A, E, never>) =>
    absorbSql(
      Effect.uninterruptibleMask((restore) =>
        Effect.gen(function* () {
          yield* sql`BEGIN`;
          const exit = yield* restore(effect).pipe(Effect.exit);

          if (Exit.isSuccess(exit)) {
            yield* sql`COMMIT`;
          } else {
            yield* sql`ROLLBACK`;
          }

          return yield* Exit.matchEffect(exit, {
            onFailure: Effect.failCause,
            onSuccess: Effect.succeed,
          });
        }),
      ),
    ),
});
```

**Key patterns:**

1. **Template literal SQL** — Type-safe SQL queries with @effect/sql
2. **Error absorption** — `absorbSql` converts SQL errors to Effect errors
3. **Transaction support** — `withTransaction` provides atomic operations
4. **Uninterruptible commits** — Ensures transactions complete once started

### KV Scope Wrapper (`index.ts:96-103`)

```typescript
export const makeScopedKv = (kv: Kv, folder: string): Kv => ({
  get: (namespace, key) => kv.get(`${folder}::${namespace}`, key),
  set: (namespace, key, value) => kv.set(`${folder}::${namespace}`, key, value),
  delete: (namespace, key) => kv.delete(`${folder}::${namespace}`, key),
  list: (namespace) => kv.list(`${folder}::${namespace}`),
  deleteAll: (namespace) => kv.deleteAll(`${folder}::${namespace}`),
  withTransaction: kv.withTransaction,
});
```

**Purpose:** Prefixes all KV namespaces with a folder path for isolation.

### Tool Registry with KV (`tool-registry.ts:29-233`)

```typescript
export const makeKvToolRegistry = (toolsKv: ScopedKv, defsKv: ScopedKv) => {
  const withKvTransaction = <A, E>(
    kv: ScopedKv,
    effect: Effect.Effect<A, E, never>,
  ): Effect.Effect<A, E, never> => kv.withTransaction?.(effect) ?? effect;

  const runtimeTools = new Map<string, ToolRegistration>();
  const runtimeHandlers = new Map<string, RuntimeToolHandler>();
  const runtimeDefs = new Map<string, unknown>();
  const invokers = new Map<string, ToolInvoker>();

  const getPersistedTool = (id: string): Effect.Effect<ToolRegistration | null> =>
    Effect.gen(function* () {
      const raw = yield* toolsKv.get(id);
      if (!raw) return null;
      return decodeTool(raw);
    });

  const getAllTools = (): Effect.Effect<ToolRegistration[]> =>
    Effect.gen(function* () {
      const entries = yield* toolsKv.list();
      return entries.map((e) => decodeTool(e.value));
    });

  const getDefsMap = (): Effect.Effect<Map<string, unknown>> =>
    Effect.gen(function* () {
      const entries = yield* defsKv.list();
      const defs = yield* Effect.try(() =>
        new Map(entries.map((e) => [e.key, JSON.parse(e.value)])),
      ).pipe(Effect.orDie);
      for (const [k, v] of runtimeDefs) defs.set(k, v);
      return defs;
    });

  return {
    list: (filter?: ToolListFilter) =>
      Effect.gen(function* () {
        const byId = new Map<string, ToolRegistration>();
        for (const tool of yield* getAllTools()) byId.set(tool.id, tool);
        for (const tool of runtimeTools.values()) byId.set(tool.id, tool);

        let tools = [...byId.values()];
        if (filter?.sourceId) {
          const sid = filter.sourceId;
          tools = tools.filter((t) => t.sourceId === sid);
        }
        if (filter?.query) {
          const q = filter.query.toLowerCase();
          tools = tools.filter(
            (t) =>
              t.name.toLowerCase().includes(q) ||
              t.description?.toLowerCase().includes(q),
          );
        }
        return tools.map((t) => ({
          id: t.id,
          pluginKey: t.pluginKey,
          sourceId: t.sourceId,
          name: t.name,
          description: t.description,
        }));
      }),

    // ... schema, definitions, register, invoke, etc.
  };
};
```

**Key patterns:**

1. **Hybrid storage** — Persisted tools (KV) + runtime tools (in-memory Map)
2. **Definition merging** — Combines persisted and runtime definitions
3. **Transaction wrapping** — Uses `withKvTransaction` for atomic writes
4. **Lazy loading** — Tools loaded from KV on demand

### Secret Store with KV (`secret-store.ts:23-176`)

```typescript
export const makeKvSecretStore = (refsKv: ScopedKv) => {
  const providers: SecretProvider[] = [];

  const findWritableProvider = (key?: string): SecretProvider | undefined =>
    key ? providers.find((p) => p.key === key) : providers.find((p) => p.writable);

  const resolveFromProviders = (
    secretId: SecretId,
    providerKey: string | undefined,
  ): Effect.Effect<string | null> => {
    if (providerKey) {
      const provider = providers.find((p) => p.key === providerKey);
      return provider ? provider.get(secretId) : Effect.succeed(null);
    }
    return Effect.gen(function* () {
      for (const provider of providers) {
        const value = yield* provider.get(secretId);
        if (value !== null) return value;
      }
      return null;
    });
  };

  return {
    list: (scopeId: ScopeId) =>
      Effect.gen(function* () {
        // Stored refs from KV
        const entries = yield* refsKv.list();
        const storedRefs = entries
          .map((e) => decodeRef(e.value))
          .filter((r) => r.scopeId === scopeId);

        const seenIds = new Set(storedRefs.map((r) => r.id));

        // Merge in secrets from providers that can enumerate
        const providerRefs: SecretRef[] = [];
        for (const provider of providers) {
          if (!provider.list) continue;
          const items = yield* provider.list().pipe(
            Effect.orElseSucceed(() => [] as { id: string; name: string }[]),
          );
          for (const item of items) {
            if (seenIds.has(item.id as SecretId)) continue;
            seenIds.add(item.id as SecretId);
            providerRefs.push(
              new SecretRef({
                id: SecretId.make(item.id),
                scopeId,
                name: item.name,
                provider: Option.some(provider.key),
                purpose: undefined,
                createdAt: new Date(),
              }),
            );
          }
        }

        return [...storedRefs, ...providerRefs];
      }),

    set: (input: SetSecretInput) =>
      Effect.gen(function* () {
        const candidates = input.provider
          ? providers.filter((p) => p.key === input.provider && p.writable && p.set)
          : providers.filter((p) => p.writable && p.set);

        if (candidates.length === 0) {
          return yield* new SecretResolutionError({
            secretId: input.id,
            message: `No writable provider found${input.provider ? ` (requested: ${input.provider})` : ""}`,
          });
        }

        let usedProvider: SecretProvider | undefined;
        for (const candidate of candidates) {
          yield* candidate.set!(input.id, input.value);
          const readBack = yield* candidate.get(input.id);
          if (readBack !== null) {
            usedProvider = candidate;
            break;
          }
        }

        if (!usedProvider) {
          return yield* new SecretResolutionError({
            secretId: input.id,
            message: "All writable providers failed to store the secret",
          });
        }

        const ref = new SecretRef({
          id: input.id,
          scopeId: input.scopeId,
          name: input.name,
          provider: Option.some(usedProvider.key),
          purpose: input.purpose,
          createdAt: new Date(),
        });

        yield* refsKv.set(input.id, encodeRef(ref));
        return ref;
      }),

    // ... list, resolve, status, remove, addProvider, providers
  };
};
```

**Key patterns:**

1. **Ref/value separation** — KV stores refs, providers store actual values
2. **Provider chain** — Iterates providers until value is found
3. **Write verification** — Reads back after writing to verify storage
4. **Provider enumeration** — Merges KV refs with provider-enumerated secrets

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      Storage-File Package                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    KV Layer                                          │   │
│  │                                                                       │   │
│  │  makeSqliteKv(sql) → Kv                                             │   │
│  │  makeInMemoryKv() → Kv                                              │   │
│  │                                                                       │   │
│  │  SQLite table: kv (namespace, key, value)                           │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│              ┌───────────────┴───────────────┐                             │
│              ▼                               ▼                             │
│  ┌─────────────────────────┐     ┌─────────────────────────┐             │
│  │  ScopedKV Wrapper       │     │  Scope ID Generation    │             │
│  │                         │     │                         │             │
│  │  makeScopedKv(kv, fw)   │     │  makeScopeId(cwd)       │             │
│  │  → prefixes namespaces  │     │  → folder-hash          │             │
│  └─────────────────────────┘     └─────────────────────────┘             │
│              │                                                       │
│              ▼                                                       ▼
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Service Factories                                 │   │
│  │                                                                       │   │
│  │  makeKvToolRegistry(toolsKv, defsKv)                                │   │
│  │  ├── toolsKv: toolId → ToolRegistration (JSON)                      │   │
│  │  └── defsKv: defName → JSON Schema (JSON)                           │   │
│  │                                                                       │   │
│  │  makeKvSecretStore(refsKv)                                          │   │
│  │  ├── refsKv: secretId → SecretRef (JSON)                            │   │
│  │  └── providers: actual secret values (external)                     │   │
│  │                                                                       │   │
│  │  makeKvPolicyEngine(policiesKv, metaKv)                             │   │
│  │  ├── policiesKv: policyId → Policy (JSON)                           │   │
│  │  └── metaKv: counter → number                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Migration System                                  │   │
│  │                                                                       │   │
│  │  migrate = Migrator.make({})({ loader })                            │   │
│  │  └── migrations/0001_initial.ts                                     │   │
│  │      └── CREATE TABLE kv (namespace, key, value)                    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### Tool Registration Flow

```
tools.register(toolRegistrations)
    │
    ├──> 1. Serialize each tool
    │    └──> encodeTool(tool) → JSON string
    │
    ├──> 2. Write to KV (with transaction)
    │    └──> toolsKv.set(toolId, encodedTool)
    │         └──> INSERT OR REPLACE INTO kv ...
    │
    └──> 3. Tool now persisted in SQLite
```

### Tool Invocation Flow

```
tools.invoke(toolId, args, options)
    │
    ├──> 1. Check runtime handlers first
    │    └──> runtimeHandlers.get(toolId)
    │
    ├──> 2. Check runtime tools
    │    └──> runtimeTools.get(toolId)
    │
    ├──> 3. Fall back to KV
    │    └──> toolsKv.get(toolId)
    │         └──> SELECT value FROM kv WHERE ...
    │
    ├──> 4. Get invoker for plugin
    │    └──> invokers.get(tool.pluginKey)
    │
    └──> 5. Invoke through plugin invoker
         └──> invoker.invoke(toolId, args, options)
```

### Secret Set Flow

```
secrets.set({ id, name, value, provider?, purpose? })
    │
    ├──> 1. Find candidate providers
    │    ├──> If provider specified: filter to that provider
    │    └──> Else: all writable providers with .set()
    │
    ├──> 2. Try each candidate until one succeeds
    │    ├──> provider.set(id, value)
    │    └──> Verify: provider.get(id) !== null
    │
    ├──> 3. Create SecretRef
    │    └──> { id, scopeId, name, provider: usedProvider.key, purpose }
    │
    ├──> 4. Store ref in KV
    │    └──> refsKv.set(id, encodeRef(ref))
    │
    └──> 5. Return SecretRef
```

### Secret Resolve Flow

```
secrets.resolve(secretId, scopeId)
    │
    ├──> 1. Load ref from KV
    │    └──> refsKv.get(secretId)
    │         └──> decodeRef(raw) → SecretRef
    │
    ├──> 2. Extract provider key from ref
    │    └──> ref.provider (Option<string>)
    │
    ├──> 3. Query providers
    │    ├──> If provider specified: query that provider
    │    └──> Else: iterate all providers until value found
    │
    └──> 4. Return value or error
```

---

## 7. Key Patterns

### Single Table KV Design

```sql
CREATE TABLE kv (
  namespace TEXT NOT NULL,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  PRIMARY KEY (namespace, key)
);
```

**Benefits:**
1. **Simplicity** — One table for all data
2. **Flexibility** — Any schema can be stored as JSON
3. **Isolation** — Namespace prefix provides multi-tenancy
4. **Portability** — Works with SQLite, PostgreSQL, etc.

### Namespace Prefixing for Scoping

```typescript
const ns = (name: string) => `${cwd}::${name}`;

return {
  tools: makeKvToolRegistry(
    scopeKv(kv, ns("tools")),
    scopeKv(kv, ns("defs")),
  ),
  secrets: makeKvSecretStore(scopeKv(kv, ns("secrets"))),
  // ...
};
```

**Benefits:**
1. **Physical isolation** — Each scope's data is fully separated
2. **Easy cleanup** — Delete all data for a scope with `deleteAll`
3. **No foreign keys needed** — Logical grouping via prefix

### Ref/Value Separation for Secrets

```
┌─────────────────────────────────────────┐
│  KV (SQLite)                            │
│  └── secrets namespace                  │
│      └── secretId → SecretRef (JSON)    │
│          - id, scopeId, name            │
│          - provider: "keychain"         │
│          - purpose: "API access"        │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  Secret Providers (External)            │
│  ├── Keychain: secretId → value         │
│  ├── Environment: secretId → value      │
│  └── 1Password: secretId → value        │
└─────────────────────────────────────────┘
```

**Benefits:**
1. **Security** — Actual secrets never stored in database
2. **Flexibility** — Use system keychain, env vars, or external providers
3. **Portability** — Refs can be exported without exposing values

### Write Verification Pattern

```typescript
let usedProvider: SecretProvider | undefined;
for (const candidate of candidates) {
  yield* candidate.set!(input.id, input.value);
  const readBack = yield* candidate.get(input.id);
  if (readBack !== null) {
    usedProvider = candidate;
    break;
  }
}
```

**Purpose:** Verifies that the provider actually stored the value before considering it successful.

---

## 8. Integration Points

### Dependencies

| Package | Purpose |
|---------|---------|
| `@effect/sql` | SQL database layer |
| `@effect/sql-sqlite-bun` | SQLite driver |
| `@executor/sdk` | SDK types and services |
| `effect` | Effect runtime |

### Dependents

| Package | Relationship |
|---------|--------------|
| `@executor/apps/cli` | Uses storage-file for local persistence |
| `@executor/apps/server` | Uses storage-file for single-user mode |
| `@executor/config` | Config wrapper syncs with storage |

---

## 9. Error Handling

### SQL Error Absorption

```typescript
// sql-utils.ts
export const absorbSql = <A, E>(
  effect: Effect.Effect<A, E, SqlClient.SqlError>,
): Effect.Effect<A, unknown> =>
  effect.pipe(
    Effect.catchAll((err) =>
      Effect.fail(new Error(`SQL error: ${err.message}`)),
    ),
  );
```

**Purpose:** Converts SQL errors to generic errors for simpler error handling.

### Transaction Rollback

```typescript
withTransaction: <A, E>(effect) =>
  absorbSql(
    Effect.uninterruptibleMask((restore) =>
      Effect.gen(function* () {
        yield* sql`BEGIN`;
        const exit = yield* restore(effect).pipe(Effect.exit);

        if (Exit.isSuccess(exit)) {
          yield* sql`COMMIT`;
        } else {
          yield* sql`ROLLBACK`;
        }

        return yield* Exit.matchEffect(exit, {
          onFailure: Effect.failCause,
          onSuccess: Effect.succeed,
        });
      }),
    ),
  ),
```

**Pattern:** Ensures transactions are always committed or rolled back.

---

## 10. Testing Strategy

### Integration Tests

**File:** `src/index.test.ts` (262 lines)

Tests cover:
- Full executor creation with SQLite storage
- Tool registration and invocation
- Secret storage and resolution
- Policy enforcement
- Multi-scope isolation

### Test Setup

```typescript
import { SqliteClient } from "@effect/sql-sqlite-bun";
import { makeSqliteKv, makeKvConfig } from "@executor/storage-file";

const testProgram = Effect.gen(function* () {
  const sql = yield* SqlClient.SqlClient;
  const kv = makeSqliteKv(sql);
  const config = makeKvConfig(kv, { cwd: "/test/project" });
  const executor = yield* createExecutor(config);
  
  // Test operations...
}).pipe(
  Effect.provide(SqliteClient.layer({ filename: ":memory:" })),
);
```

---

## 11. Migration System

### Initial Migration (`migrations/0001_initial.ts`)

```typescript
import { sql } from "@effect/sql";

export default sql`
  CREATE TABLE IF NOT EXISTS kv (
    namespace TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (namespace, key)
  );
`;
```

### Migration Runner (`schema.ts`)

```typescript
import * as Migrator from "@effect/sql/Migrator";
import { loader } from "./migrations";

export const migrate = Migrator.make({})({ loader });
```

**Usage:**

```typescript
// Run migrations on startup
yield* migrate.pipe(
  Effect.provide(SqlClient.SqlClient),
);
```

---

## 12. Design Decisions

### Why Single KV Table?

1. **Simplicity** — One table, three columns
2. **Flexibility** — Any data structure as JSON value
3. **No schema migrations** — Add new namespaces without altering schema
4. **Easy backup** — Single table to export/backup

### Why Namespace Prefixing?

1. **Logical isolation** — Each scope is fully separated
2. **No cascade deletes** — Deleting scope is just `DELETE WHERE namespace LIKE 'scope%'`
3. **Clear ownership** — Each namespace belongs to exactly one scope

### Why Separate Refs and Values for Secrets?

1. **Security** — Database doesn't contain sensitive values
2. **Platform integration** — Use OS keychain, environment, or external providers
3. **Audit trail** — Refs in DB show what secrets exist without exposing values

### Why Write Verification?

1. **Provider reliability** — Some providers may silently fail (e.g., keychain on unsupported platforms)
2. **Early detection** — Catch storage issues immediately
3. **Correct provider selection** — Use first provider that actually stores the value

---

## 13. Summary

The Storage-File package provides **SQLite-backed persistence**:

1. **KV Abstraction** — Single table for all data with namespace isolation
2. **Service Factories** — ToolRegistry, SecretStore, PolicyEngine from KV
3. **Transaction Support** — Atomic operations with rollback
4. **Migration System** — Schema evolution with @effect/sql
5. **Secret Security** — Refs in DB, values in external providers

The storage layer enables **durable persistence** while maintaining **flexibility** through the KV abstraction and **security** through secret ref/value separation.
