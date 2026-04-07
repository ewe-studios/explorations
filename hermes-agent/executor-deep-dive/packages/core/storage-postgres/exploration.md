# Executor Core Storage-Postgres — Deep Dive Exploration

**Package:** `@executor/storage-postgres`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/packages/core/storage-postgres`  
**Total Files:** 11 TypeScript files  
**Total Lines:** ~1,490 lines  

---

## 1. Module Overview

The Storage-Postgres package provides **PostgreSQL-backed relational storage** for the Executor system. It replaces the KV-based storage-file with proper relational tables for the cloud/SaaS version. It implements:

- **Drizzle ORM schemas** — Type-safe database operations
- **Relational tables** — Proper foreign keys and relationships
- **Team-based multi-tenancy** — Team-scoped data isolation
- **User management** — Users, teams, invitations, sessions
- **Encrypted secrets** — AES-encrypted secret storage in bytea columns

### Key Responsibilities

1. **Relational Storage** — Proper schema with foreign keys and constraints
2. **Team Isolation** — All data scoped to team_id
3. **User Management** — Auth-ready user/team/invitation tables
4. **Secret Encryption** — Encrypt secrets at rest using AES

---

## 2. File Inventory

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/index.ts` | 66 | Main exports and config builder |
| 2 | `src/schema.ts` | 155 | Drizzle ORM schema definitions |
| 3 | `src/tool-registry.ts` | 261 | PostgreSQL-backed tool registry |
| 4 | `src/secret-store.ts` | 201 | PostgreSQL-backed secret store with encryption |
| 5 | `src/policy-engine.ts` | 73 | PostgreSQL-backed policy engine |
| 6 | `src/pg-kv.ts` | 86 | PostgreSQL KV implementation |
| 7 | `src/user-store.ts` | 161 | User, team, invitation management |
| 8 | `src/crypto.ts` | 42 | AES encryption/decryption |
| 9 | `src/index.test.ts` | 431 | Integration tests |
| 10 | `drizzle.config.ts` | 7 | Drizzle configuration |
| 11 | `vitest.config.ts` | 7 | Test configuration |

---

## 3. Key Exports

### Schema Tables

```typescript
// schema.ts
export const users = pgTable("users", {
  id: text("id").primaryKey(),
  email: text("email").notNull().unique(),
  name: text("name"),
  avatarUrl: text("avatar_url"),
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
});

export const teams = pgTable("teams", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  name: text("name").notNull(),
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
});

export const teamMembers = pgTable("team_members", {
  teamId: text("team_id").notNull(),
  userId: text("user_id").notNull(),
  role: text("role").notNull().$default(() => "member"),
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
}, (table) => [primaryKey({ columns: [table.teamId, table.userId] })]);

export const tools = pgTable("tools", {
  id: text("id").notNull(),
  teamId: text("team_id").notNull(),
  sourceId: text("source_id").notNull(),
  pluginKey: text("plugin_key").notNull(),
  name: text("name").notNull(),
  description: text("description"),
  mayElicit: boolean("may_elicit").$default(() => false),
  inputSchema: jsonb("input_schema"),
  outputSchema: jsonb("output_schema"),
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
}, (table) => [primaryKey({ columns: [table.id, table.teamId] })]);

export const secrets = pgTable("secrets", {
  id: text("id").notNull(),
  teamId: text("team_id").notNull(),
  name: text("name").notNull(),
  purpose: text("purpose"),
  encryptedValue: bytea("encrypted_value").notNull(),
  iv: bytea("iv").notNull(),
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
}, (table) => [primaryKey({ columns: [table.id, teamId] })]);
```

### Config Factory

```typescript
// index.ts
export const makePgConfig = <
  const TPlugins extends readonly ExecutorPlugin<string, object>[] = [],
>(
  db: PgDatabase<any, any, any>,
  options: {
    readonly teamId: string;
    readonly teamName: string;
    readonly encryptionKey: string;
    readonly plugins?: TPlugins;
  },
): ExecutorConfig<TPlugins> => {
  const scope: Scope = {
    id: ScopeId.make(options.teamId),
    name: options.teamName,
    createdAt: new Date(),
  };

  return {
    scope,
    tools: makePgToolRegistry(db, options.teamId),
    sources: makeInMemorySourceRegistry(),
    secrets: makePgSecretStore(db, options.teamId, options.encryptionKey),
    policies: makePgPolicyEngine(db, options.teamId),
    plugins: options.plugins,
  };
};
```

### Crypto Utilities

```typescript
// crypto.ts
export const encrypt = (
  plaintext: string,
  key: string,
): Effect.Effect<{ ciphertext: Buffer; iv: Buffer }> =>
  Effect.sync(() => {
    const keyHash = createHash("sha256").update(key).digest();
    const iv = randomBytes(16);
    const cipher = createCipheriv("aes-256-cbc", keyHash, iv);
    const ciphertext = Buffer.concat([
      cipher.update(plaintext, "utf8"),
      cipher.final(),
    ]);
    return { ciphertext, iv };
  });

export const decrypt = (
  ciphertext: Buffer,
  iv: Buffer,
  key: string,
): Effect.Effect<string> =>
  Effect.sync(() => {
    const keyHash = createHash("sha256").update(key).digest();
    const decipher = createDecipheriv("aes-256-cbc", keyHash, iv);
    const plaintext = Buffer.concat([
      decipher.update(ciphertext),
      decipher.final(),
    ]);
    return plaintext.toString("utf8");
  });
```

---

## 4. Line-by-Line Analysis

### Tool Registry with Drizzle (`tool-registry.ts`)

```typescript
export const makePgToolRegistry = (
  db: PgDatabase<any, any, any>,
  teamId: string,
) => {
  const runtimeTools = new Map<string, ToolRegistration>();
  const runtimeHandlers = new Map<string, RuntimeToolHandler>();
  const runtimeDefs = new Map<string, unknown>();
  const invokers = new Map<string, ToolInvoker>();

  return {
    list: (filter?: ToolListFilter) =>
      Effect.gen(function* () {
        // Query persisted tools from database
        const persistedTools = yield* Effect.tryPromise(() =>
          db
            .select()
            .from(tools)
            .where(eq(tools.teamId, teamId))
            .then((rows) =>
              rows.map((row) => ({
                id: row.id,
                pluginKey: row.pluginKey,
                sourceId: row.sourceId,
                name: row.name,
                description: row.description ?? undefined,
                mayElicit: row.mayElicit ?? false,
                inputSchema: row.inputSchema as Record<string, unknown> | undefined,
                outputSchema: row.outputSchema as Record<string, unknown> | undefined,
              })),
            ),
        );

        // Merge with runtime tools
        const byId = new Map<string, ToolRegistration>();
        for (const tool of persistedTools) byId.set(tool.id, tool);
        for (const tool of runtimeTools.values()) byId.set(tool.id, tool);

        let result = [...byId.values()];
        if (filter?.sourceId) {
          result = result.filter((t) => t.sourceId === filter.sourceId);
        }
        if (filter?.query) {
          const q = filter.query.toLowerCase();
          result = result.filter(
            (t) =>
              t.name.toLowerCase().includes(q) ||
              t.description?.toLowerCase().includes(q),
          );
        }
        return result.map((t) => ({
          id: t.id,
          pluginKey: t.pluginKey,
          sourceId: t.sourceId,
          name: t.name,
          description: t.description,
        }));
      }),

    register: (newTools: readonly ToolRegistration[]) =>
      Effect.tryPromise(async () => {
        for (const tool of newTools) {
          await db
            .insert(tools)
            .values({
              id: tool.id,
              teamId,
              sourceId: tool.sourceId,
              pluginKey: tool.pluginKey,
              name: tool.name,
              description: tool.description,
              mayElicit: tool.mayElicit,
              inputSchema: tool.inputSchema,
              outputSchema: tool.outputSchema,
            })
            .onConflictDoUpdate({
              target: [tools.id, tools.teamId],
              set: {
                sourceId: tool.sourceId,
                pluginKey: tool.pluginKey,
                name: tool.name,
                description: tool.description,
                mayElicit: tool.mayElicit,
                inputSchema: tool.inputSchema,
                outputSchema: tool.outputSchema,
              },
            });
        }
      }),

    // ... schema, definitions, invoke, etc.
  };
};
```

**Key patterns:**

1. **Hybrid storage** — Persisted (PostgreSQL) + runtime (in-memory Maps)
2. **Upsert on conflict** — `onConflictDoUpdate` for idempotent inserts
3. **Team filtering** — All queries filtered by teamId
4. **JSON serialization** — Schemas stored as JSONB

### Secret Store with Encryption (`secret-store.ts`)

```typescript
export const makePgSecretStore = (
  db: PgDatabase<any, any, any>,
  teamId: string,
  encryptionKey: string,
) => {
  const providers: SecretProvider[] = [];

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
        const rows = yield* Effect.tryPromise(() =>
          db
            .select()
            .from(secrets)
            .where(eq(secrets.teamId, teamId))
            .then((rows) =>
              rows.map((row) => ({
                id: row.id,
                name: row.name,
                purpose: row.purpose ?? undefined,
                createdAt: row.createdAt,
              })),
            ),
        );

        return rows.map(
          (r) =>
            new SecretRef({
              id: SecretId.make(r.id),
              scopeId,
              name: r.name,
              provider: Option.none(),
              purpose: r.purpose,
              createdAt: r.createdAt,
            }),
        );
      }),

    set: (input: SetSecretInput) =>
      Effect.gen(function* () {
        const candidates = input.provider
          ? providers.filter((p) => p.key === input.provider && p.writable && p.set)
          : providers.filter((p) => p.writable && p.set);

        if (candidates.length === 0) {
          return yield* new SecretResolutionError({
            secretId: input.id,
            message: `No writable provider found`,
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
            message: "All writable providers failed",
          });
        }

        // Encrypt and store in database
        const { ciphertext, iv } = yield* encrypt(input.value, encryptionKey);

        yield* Effect.tryPromise(async () => {
          await db
            .insert(secrets)
            .values({
              id: input.id,
              teamId,
              name: input.name,
              purpose: input.purpose,
              encryptedValue: ciphertext,
              iv,
            })
            .onConflictDoUpdate({
              target: [secrets.id, secrets.teamId],
              set: {
                name: input.name,
                purpose: input.purpose,
                encryptedValue: ciphertext,
                iv,
              },
            });
        });

        return new SecretRef({
          id: input.id,
          scopeId: input.scopeId,
          name: input.name,
          provider: Option.some(usedProvider.key),
          purpose: input.purpose,
          createdAt: new Date(),
        });
      }),

    resolve: (secretId: SecretId, _scopeId: ScopeId) =>
      Effect.gen(function* () {
        const rows = yield* Effect.tryPromise(() =>
          db
            .select()
            .from(secrets)
            .where(and(eq(secrets.id, secretId), eq(secrets.teamId, teamId)))
            .limit(1),
        );

        const row = rows[0];
        if (!row) {
          return yield* new SecretNotFoundError({ secretId });
        }

        const plaintext = yield* decrypt(row.encryptedValue, row.iv, encryptionKey);
        return plaintext;
      }),

    // ... status, remove, addProvider, providers
  };
};
```

**Key patterns:**

1. **AES-256-CBC encryption** — Secrets encrypted before storage
2. **Per-secret IV** — Unique initialization vector for each secret
3. **Team scoping** — All queries include teamId filter
4. **Write verification** — Verifies provider storage before encrypting

### User Store (`user-store.ts`)

```typescript
export interface User {
  readonly id: string;
  readonly email: string;
  readonly name?: string;
  readonly avatarUrl?: string;
  readonly createdAt: Date;
}

export interface Team {
  readonly id: string;
  readonly name: string;
  readonly createdAt: Date;
}

export interface TeamMember {
  readonly teamId: string;
  readonly userId: string;
  readonly role: "owner" | "admin" | "member";
  readonly createdAt: Date;
}

export interface Invitation {
  readonly id: string;
  readonly teamId: string;
  readonly email: string;
  readonly invitedBy: string;
  readonly status: "pending" | "accepted" | "declined" | "expired";
  readonly createdAt: Date;
  readonly expiresAt: Date;
}

export const makeUserStore = (db: PgDatabase<any, any, any>) => ({
  createUser: (email: string, name?: string) =>
    Effect.tryPromise(async () => {
      const [user] = await db
        .insert(users)
        .values({
          id: crypto.randomUUID(),
          email,
          name,
        })
        .returning();
      return user as User;
    }),

  createTeam: (userId: string, name: string) =>
    Effect.tryPromise(async () => {
      const [team] = await db.transaction(async (tx) => {
        const [team] = await tx
          .insert(teams)
          .values({ name })
          .returning();

        await tx.insert(teamMembers).values({
          teamId: team.id,
          userId,
          role: "owner",
        });

        return [team];
      });
      return team as Team;
    }),

  inviteMember: (teamId: string, email: string, invitedBy: string) =>
    Effect.tryPromise(async () => {
      const [invitation] = await db
        .insert(invitations)
        .values({
          id: crypto.randomUUID(),
          teamId,
          email,
          invitedBy,
          expiresAt: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000), // 7 days
        })
        .returning();
      return invitation as Invitation;
    }),

  acceptInvitation: (invitationId: string, userId: string) =>
    Effect.tryPromise(async () => {
      await db.transaction(async (tx) => {
        const [invitation] = await tx
          .select()
          .from(invitations)
          .where(eq(invitations.id, invitationId))
          .limit(1);

        if (!invitation || invitation.status !== "pending") {
          throw new Error("Invalid or expired invitation");
        }

        await tx
          .update(invitations)
          .set({ status: "accepted" })
          .where(eq(invitations.id, invitationId));

        await tx.insert(teamMembers).values({
          teamId: invitation.teamId,
          userId,
          role: "member",
        });
      });
    }),
});
```

**Key patterns:**

1. **Transactional operations** — Team creation and invitation acceptance use transactions
2. **Invitation expiry** — Invitations expire after 7 days
3. **Role-based access** — Team members have roles (owner, admin, member)
4. **Atomic team creation** — Creates team and adds owner in one transaction

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Storage-Postgres Package                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Drizzle ORM Schema                                │   │
│  │                                                                       │   │
│  │  users         teams         team_members    invitations   sessions  │   │
│  │  tools         sources       secrets         policies      plugin_kv │   │
│  │                                                                       │   │
│  │  All tables have team_id for multi-tenancy                          │   │
│  │  Composite primary keys: (id, team_id)                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│              ┌───────────────┴───────────────┐                             │
│              ▼                               ▼                             │
│  ┌─────────────────────────┐     ┌─────────────────────────┐             │
│  │  Service Factories      │     │  User Management        │             │
│  │                         │     │                         │             │
│  │  makePgToolRegistry()   │     │  makeUserStore()        │             │
│  │  makePgSecretStore()    │     │  - createUser()         │             │
│  │  makePgPolicyEngine()   │     │  - createTeam()         │             │
│  │  makePgKv()             │     │  - inviteMember()       │             │
│  └─────────────────────────┘     │  - acceptInvitation()   │             │
│                                  └─────────────────────────┘             │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Crypto Layer                                      │   │
│  │                                                                       │   │
│  │  encrypt(plaintext, key) → { ciphertext, iv }                       │   │
│  │  decrypt(ciphertext, iv, key) → plaintext                           │   │
│  │                                                                       │   │
│  │  Algorithm: AES-256-CBC                                              │   │
│  │  Key derivation: SHA-256 hash of encryption key                     │   │
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
    ├──> 1. For each tool, build row
    │    └──> { id, teamId, sourceId, pluginKey, name, description, ... }
    │
    ├──> 2. Upsert to database
    │    └──> INSERT ... ON CONFLICT (id, teamId) DO UPDATE
    │
    └──> 3. Tool persisted in PostgreSQL
```

### Secret Set Flow

```
secrets.set({ id, name, value, provider?, purpose? })
    │
    ├──> 1. Find candidate providers
    │    └──> Filter by provider or all writable
    │
    ├──> 2. Try each candidate with verification
    │    ├──> provider.set(id, value)
    │    └──> Verify: provider.get(id) !== null
    │
    ├──> 3. Encrypt value
    │    ├──> keyHash = SHA-256(encryptionKey)
    │    ├──> iv = randomBytes(16)
    │    └──> ciphertext = AES-256-CBC(plaintext, keyHash, iv)
    │
    ├──> 4. Upsert to database
    │    └──> INSERT ... ON CONFLICT DO UPDATE
    │         └──> { id, teamId, name, purpose, encryptedValue, iv }
    │
    └──> 5. Return SecretRef
```

### Secret Resolve Flow

```
secrets.resolve(secretId, scopeId)
    │
    ├──> 1. Query database for secret
    │    └──> SELECT * FROM secrets WHERE id = ? AND teamId = ?
    │
    ├──> 2. Decrypt value
    │    ├──> keyHash = SHA-256(encryptionKey)
    │    └──> plaintext = AES-256-CBC-Decrypt(ciphertext, keyHash, iv)
    │
    └──> 3. Return plaintext
```

### Invitation Acceptance Flow

```
acceptInvitation(invitationId, userId)
    │
    ├──> 1. Start transaction
    │
    ├──> 2. Load invitation
    │    └──> Check status is "pending" and not expired
    │
    ├──> 3. Update invitation status
    │    └──> SET status = "accepted"
    │
    ├──> 4. Add team member
    │    └──> INSERT INTO team_members (teamId, userId, role)
    │
    └──> 5. Commit transaction
```

---

## 7. Key Patterns

### Composite Primary Keys

```typescript
export const tools = pgTable("tools", {
  id: text("id").notNull(),
  teamId: text("team_id").notNull(),
  // ...
}, (table) => [primaryKey({ columns: [table.id, table.teamId] })]);
```

**Benefits:**
1. **Multi-tenancy** — Same tool id can exist for different teams
2. **Efficient queries** — Index on (id, teamId) for fast lookups
3. **Data isolation** — Teams cannot access each other's data

### Upsert Pattern

```typescript
await db
  .insert(tools)
  .values(toolData)
  .onConflictDoUpdate({
    target: [tools.id, tools.teamId],
    set: { /* fields to update */ },
  });
```

**Benefits:**
1. **Idempotent operations** — Safe to retry
2. **Single round-trip** — No need for separate exists check
3. **Atomic** — No race conditions between insert and update

### Encryption at Rest

```
┌─────────────────────────────────────────────────────┐
│  Application Layer                                  │
│  plaintext = "sk-abc123..."                        │
└─────────────────────────────────────────────────────┘
         │
         │ encrypt(plaintext, key)
         ▼
┌─────────────────────────────────────────────────────┐
│  Database Layer                                     │
│  secrets table:                                     │
│  - encryptedValue: bytea (ciphertext)              │
│  - iv: bytea (initialization vector)               │
└─────────────────────────────────────────────────────┘
```

**Benefits:**
1. **Defense in depth** — Encrypted even if DB is compromised
2. **Key separation** — Encryption key separate from database
3. **Compliance** — Meets data-at-rest encryption requirements

---

## 8. Integration Points

### Dependencies

| Package | Purpose |
|---------|---------|
| `drizzle-orm` | Type-safe ORM |
| `pg` | PostgreSQL driver |
| `@executor/sdk` | SDK types and services |
| `effect` | Effect runtime |
| `node:crypto` | AES encryption |

### Dependents

| Package | Relationship |
|---------|--------------|
| `@executor/apps/server` | Uses storage-postgres for cloud deployment |
| `@executor/hosts/mcp` | MCP server with team support |

---

## 9. Error Handling

### SQL Error Handling

```typescript
Effect.tryPromise(async () => {
  return await db.select().from(tools).where(...);
})
```

**Pattern:** Wraps Promise-based Drizzle operations in Effect.

### Transaction Error Handling

```typescript
await db.transaction(async (tx) => {
  // All operations in transaction
  // Rolls back automatically on error
  await tx.insert(...);
  await tx.update(...);
});
```

**Pattern:** Transactions auto-rollback on thrown errors.

### Invitation Validation

```typescript
if (!invitation || invitation.status !== "pending") {
  throw new Error("Invalid or expired invitation");
}
```

**Pattern:** Explicit validation with descriptive errors.

---

## 10. Schema Summary

### Identity Tables

| Table | Purpose |
|-------|---------|
| `users` | User accounts |
| `teams` | Team/workspaces |
| `team_members` | Team membership with roles |
| `invitations` | Team invitation management |
| `sessions` | User sessions |

### Domain Tables

| Table | Purpose |
|-------|---------|
| `tools` | Registered tools |
| `sources` | Configured sources |
| `secrets` | Encrypted secret values |
| `policies` | Access control policies |
| `plugin_kv` | Plugin-specific data |

---

## 11. Design Decisions

### Why PostgreSQL Over SQLite?

1. **Multi-tenancy** — Proper team isolation with foreign keys
2. **Concurrency** — Better handling of concurrent connections
3. **Scalability** — Horizontal scaling options
4. **Cloud-native** — Standard for cloud deployments

### Why Drizzle ORM?

1. **Type safety** — Full TypeScript inference from schema
2. **Lightweight** — Minimal runtime overhead
3. **SQL-like** — Familiar API for SQL developers
4. **Migrations** — Built-in migration support

### Why AES-256-CBC?

1. **Industry standard** — Widely audited and trusted
2. **Node.js support** — Built-in crypto module
3. **Performance** — Hardware acceleration on modern CPUs
4. **Compliance** — Meets regulatory requirements

### Why Composite Primary Keys?

1. **Multi-tenancy** — Natural team isolation
2. **Query efficiency** — Index matches query patterns
3. **Data integrity** — Prevents cross-team access

---

## 12. Summary

The Storage-Postgres package provides **relational, multi-tenant storage**:

1. **Drizzle ORM** — Type-safe database operations
2. **Team isolation** — All data scoped to team_id
3. **User management** — Auth-ready user/team/invitation system
4. **Secret encryption** — AES-256-CBC encryption at rest
5. **Composite keys** — Efficient multi-tenant queries

The PostgreSQL layer enables **cloud-scale deployments** with proper **multi-tenancy**, **encryption**, and **user management** for SaaS offerings.
