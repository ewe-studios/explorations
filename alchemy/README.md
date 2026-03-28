# Alchemy Exploration - Summary Index

**Location:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/alchemy/`
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/alchemy/`
**Status:** COMPLETE
**Date:** 2026-03-27

---

## Documents

### Core Documents

| Document | Description | Size |
|----------|-------------|------|
| [exploration.md](./exploration.md) | High-level overview of Alchemy IaC framework | 377 lines |
| [00-zero-to-deploy-engineer.md](./00-zero-to-deploy-engineer.md) | First principles textbook for infrastructure engineering | ~400 lines |
| [rust-revision.md](./rust-revision.md) | Complete Rust translation guide | 25KB |
| [production-grade.md](./production-grade.md) | Deployment, scaling, monitoring guide | 19KB |

### Deep Dives

| Document | Description | Key Topics |
|----------|-------------|------------|
| [01-distilled-api-specs-deep-dive.md](./01-distilled-api-specs-deep-dive.md) | API spec cloning via git submodules | Smithy, OpenAPI, TypeScript AST, Patch system |
| [02-provider-integration-deep-dive.md](./02-provider-integration-deep-dive.md) | Cloud provider integration patterns | Cloudflare API, AWS SigV4, GCP Discovery |
| [03-core-architecture-deep-dive.md](./03-core-architecture-deep-dive.md) | Resource system, Scope, Apply engine | Symbol metadata, AsyncLocalStorage, lifecycle |
| [03-resource-lifecycle-deep-dive.md](./03-resource-lifecycle-deep-dive.md) | Create/update/delete lifecycle | State transitions, replacement, reconciliation |
| [04-state-management-deep-dive.md](./04-state-management-deep-dive.md) | State stores and serialization | FileSystemStateStore, S3StateStore, serde |
| [05-valtron-integration.md](./05-valtron-integration.md) | Valtron replication patterns | TaskIterator, algebraic effects, no async/tokio |

---

## Architecture Summary

### Core Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Code                                 │
│                    alchemy.run.ts                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      alchemy()                                   │
│                  Creates Root Scope                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Scope                                   │
│              AsyncLocalStorage context                          │
│    ┌─────────────┬─────────────┬─────────────┬─────────────┐   │
│    │  Resources  │   Children  │    State    │   Props     │   │
│    │   (map)     │   (scopes)  │   (store)   │ (creds)     │   │
│    └─────────────┴─────────────┴─────────────┴─────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Resource()                                   │
│              Memoized async function                            │
│    ┌─────────────────────────────────────────────────────┐     │
│    │  Symbol-keyed metadata (ResourceID, Kind, FQN)     │     │
│    │  PendingResource extends Promise<Output>           │     │
│    └─────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      apply()                                     │
│              Lifecycle engine                                   │
│    ┌──────────┬──────────┬──────────┬─────────────────────┐   │
│    │  Load    │ Compare  │ Execute  │    Persist          │   │
│    │  state   │  props   │ handler  │    state            │   │
│    └──────────┴──────────┴──────────┴─────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    StateStore                                   │
│         Pluggable persistence layer                             │
│    ┌─────────────┬─────────────┬─────────────┬─────────────┐   │
│    │ FileSystem  │    S3       │     D1      │  Cloudflare │   │
│    │  (default)  │  (CI/CD)    │  (SQLite)   │   (KV+DO)   │   │
│    └─────────────┴─────────────┴─────────────┴─────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Provider Implementations

| Provider | Resources | Auth | Spec Type |
|----------|-----------|------|-----------|
| Cloudflare | Worker, D1, KV, R2, Queue, DO | API Token / API Key | TypeScript SDK |
| AWS | Lambda, S3, DynamoDB, IAM, SQS | SigV4 / OIDC | Smithy |
| GCP | Compute, Storage, BigQuery | OAuth2 JWT | Discovery JSON |
| Neon | Database, Branch | API Key | OpenAPI |
| Stripe | Customer, Payment | API Key | OpenAPI |

### State Lifecycle

```
creating ──success──> created ──props changed──> updating ──success──> updated
    │                     │                           │                    │
    │                     │                           │                    │
    ▼                     ▼                           ▼                    ▼
 error                 error                        error              (stable)
```

### Key Patterns

1. **Resources as Memoized Functions**: If props unchanged, return cached output
2. **AsyncLocalStorage for Scoping**: Implicit context propagation without wiring
3. **Symbol-Keyed Metadata**: No namespace collision with user properties
4. **Pluggable State Stores**: FileSystem (dev), S3 (CI/CD), D1/Cloudflare (prod)
5. **Serde for Serialization**: Handles circular refs, Secrets, Dates, Symbols
6. **Read Phase for Cross-Scope**: Polling for eventual consistency
7. **Orphan Cleanup on Finalize**: Delete resources removed from code

---

## Valtron Replication Summary

### TypeScript → Valtron Translation

```typescript
// TypeScript Resource
export const Worker = Resource(
  "cloudflare::Worker",
  async function (this: Context, id, props) {
    if (this.phase === "create") {
      // Create logic
    }
    return { id, name, url };
  }
);
```

```valtron
// Valtron Translation
provider Cloudflare {
  credentials: { api_token: String? },
  account_id: String,
}

resource Worker {
  props: { name: String?, entrypoint: String },
  output: { id: String, name: String, url: String },
  lifecycle: { create: create_worker, update: update_worker }
}

operation create_worker(props: WorkerProps) -> WorkerOutput {
  // Implementation using TaskIterator pattern (no async/await)
}
```

### StateStore as Algebraic Effect

```valtron
effect StateStore {
  get(key: String) -> Result<State?, String>,
  set(key: String, value: State) -> Result<Unit, String>,
  delete(key: String) -> Result<Unit, String>,
  list() -> Result<List<String>, String>,
}

handler FileSystemStateStore(root: String): StateStore {
  get(key) => read_file("{root}/{key}.json") |> parse_json |> deserialize,
  set(key, value) => serialize(value) |> to_json |> write_file("{root}/{key}.json"),
  ...
}
```

---

## What's Covered

### ✅ Complete Coverage

- [x] **Core Architecture**: Resource system, Scope, Apply engine
- [x] **State Management**: FileSystemStateStore, S3StateStore, serde
- [x] **Provider Patterns**: Cloudflare, AWS, GCP implementations
- [x] **Distilled SDKs**: Git submodule spec cloning, patch system
- [x] **Lifecycle**: Create/update/delete/replacement flows
- [x] **Cross-Scope References**: Read phase, polling
- [x] **Orphan Cleanup**: Finalization, pending deletions
- [x] **Valtron Integration**: TaskIterator pattern, algebraic effects
- [x] **Production Patterns**: CI/CD state stores, encryption
- [x] **First Principles**: Zero-to-engineer textbook

### 🔍 Source Code Reviewed

- `alchemy/alchemy/src/resource.ts` - Resource factory, Symbol metadata
- `alchemy/alchemy/src/scope.ts` - AsyncLocalStorage, hierarchy
- `alchemy/alchemy/src/apply.ts` - Lifecycle engine, state reconciliation
- `alchemy/alchemy/src/state.ts` - StateStore interface
- `alchemy/alchemy/src/state/file-system-state-store.ts` - Default store
- `alchemy/alchemy/src/aws/s3-state-store.ts` - S3 implementation
- `alchemy/alchemy/src/cloudflare/api.ts` - Cloudflare API client
- `alchemy/alchemy/src/cloudflare/worker.ts` - Worker resource
- `alchemy/alchemy/src/cloudflare/d1-database.ts` - D1 resource
- `alchemy/alchemy/src/aws/credentials.ts` - AWS credential resolution
- `alchemy/alchemy/src/aws/function.ts` - Lambda function resource
- `alchemy/alchemy/src/serde.ts` - Serialization/deserialization
- `alchemy/alchemy/src/alchemy.ts` - Entry point, CLI parsing
- `alchemy/alchemy-web/src/content/docs/` - Documentation structure

---

## How to Use This Exploration

### For Infrastructure Engineers

1. Start with `00-zero-to-deploy-engineer.md` for first principles
2. Read `exploration.md` for high-level architecture
3. Deep dive into providers with `02-provider-integration-deep-dive.md`
4. Study state management with `04-state-management-deep-dive.md`

### For Rust Implementation

1. Read `rust-revision.md` for complete Rust translation guide
2. Study `05-valtron-integration.md` for Valtron patterns
3. Reference provider deep dives for API patterns

### For Production Deployment

1. Read `production-grade.md` for deployment patterns
2. Study S3StateStore for CI/CD state management
3. Review serde encryption for secret handling

---

## Next Actions

The alchemy exploration is **COMPLETE** with all template requirements met:

- ✅ Deep/detailed coverage (11 documents, ~200KB)
- ✅ TypeScript implementation details
- ✅ Rust replication guide (rust-revision.md)
- ✅ Production-grade version (production-grade.md)
- ✅ First-principles explainer (00-zero-to-deploy-engineer.md)
- ✅ Valtron integration (05-valtron-integration.md)

**Note:** WebGPU/WASM not applicable - Alchemy is infrastructure-as-code, not graphics.

---

*End of Alchemy Exploration Index*
