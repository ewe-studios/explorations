# Executor Core API — Deep Dive Exploration

**Package:** `@executor/api`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/packages/core/api`  
**Total Files:** 8 TypeScript files  
**Total Lines:** 379 lines  

---

## 1. Module Overview

The API package provides the **REST API layer** for the Executor system. It defines:

- **Effect Platform HTTP APIs** — Type-safe REST endpoints using @effect/platform
- **OpenAPI documentation** — Auto-generated API documentation
- **Resource endpoints** — Tools, sources, secrets, executions, and scope management

### Key Responsibilities

1. **API Definition** — Define HTTP endpoints using Effect Platform
2. **Request/Response Schemas** — Type-safe serialization with Effect Schema
3. **Error Mapping** — Map domain errors to HTTP status codes
4. **Composability** — Allow plugin groups to extend the API

---

## 2. File Inventory

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/api.ts` | 31 | Root API composition |
| 2 | `src/errors.ts` | 26 | API error definitions |
| 3 | `src/index.ts` | 6 | Public exports |
| 4 | `src/executions/api.ts` | 64 | Execution endpoints |
| 5 | `src/scope/api.ts` | 23 | Scope management endpoints |
| 6 | `src/secrets/api.ts` | 89 | Secret management endpoints |
| 7 | `src/sources/api.ts` | 80 | Source management endpoints |
| 8 | `src/tools/api.ts` | 60 | Tool endpoints |

---

## 3. Key Exports

### Core API Composition

```typescript
// api.ts
import { HttpApi, OpenApi } from "@effect/platform";

export const CoreExecutorApi = HttpApi.make("executor")
  .add(ToolsApi)
  .add(SourcesApi)
  .add(SecretsApi)
  .add(ExecutionsApi)
  .add(ScopeApi)
  .annotateContext(
    OpenApi.annotations({
      title: "Executor API",
      description: "Tool execution platform API",
    }),
  );

export const addGroup = <G extends HttpApiGroup.HttpApiGroup.Any>(
  group: G,
) => CoreExecutorApi.add(group);

export const ExecutorApi = CoreExecutorApi;
```

### Tools API

```typescript
// tools/api.ts
export class ToolsApi extends HttpApiGroup.make("tools")
  .add(
    HttpApiEndpoint.get("list")`/scopes/${scopeIdParam}/tools`
      .addSuccess(Schema.Array(ToolMetadataResponse)),
  )
  .add(
    HttpApiEndpoint.get("schema")`/scopes/${scopeIdParam}/tools/${toolIdParam}/schema`
      .addSuccess(ToolSchemaResponse)
      .addError(ToolNotFound),
  )
  .prefix("/v1") {}
```

---

## 4. Line-by-Line Analysis

### API Root Composition (`api.ts:10-28`)

```typescript
export const CoreExecutorApi = HttpApi.make("executor")
  .add(ToolsApi)
  .add(SourcesApi)
  .add(SecretsApi)
  .add(ExecutionsApi)
  .add(ScopeApi)
  .annotateContext(
    OpenApi.annotations({
      title: "Executor API",
      description: "Tool execution platform API",
    }),
  );
```

**Key patterns:**

1. **Modular API groups** — Each resource has its own `HttpApiGroup`
2. **OpenAPI annotation** — Auto-generates API documentation
3. **Named API** — "executor" identifier for the API

### Group Extension (`api.ts:26-28`)

```typescript
export const addGroup = <G extends HttpApiGroup.HttpApiGroup.Any>(
  group: G,
) => CoreExecutorApi.add(group);
```

**Purpose:** Allows plugins to add custom endpoint groups to the API.

### Tools API Definition (`tools/api.ts:50-60`)

```typescript
export class ToolsApi extends HttpApiGroup.make("tools")
  .add(
    HttpApiEndpoint.get("list")`/scopes/${scopeIdParam}/tools`
      .addSuccess(Schema.Array(ToolMetadataResponse)),
  )
  .add(
    HttpApiEndpoint.get("schema")`/scopes/${scopeIdParam}/tools/${toolIdParam}/schema`
      .addSuccess(ToolSchemaResponse)
      .addError(ToolNotFound),
  )
  .prefix("/v1") {}
```

**Key patterns:**

1. **Template literal paths** — Type-safe path parameters
2. **Response schemas** — Annotated success responses
3. **Error annotations** — HTTP status codes mapped to error types
4. **Versioned prefix** — `/v1` path prefix for versioning

### Response Schema Definitions (`tools/api.ts:20-36`)

```typescript
const ToolMetadataResponse = Schema.Struct({
  id: ToolId,
  pluginKey: Schema.String,
  sourceId: Schema.String,
  name: Schema.String,
  description: Schema.optional(Schema.String),
  mayElicit: Schema.optional(Schema.Boolean),
});

const ToolSchemaResponse = Schema.Struct({
  id: ToolId,
  inputTypeScript: Schema.optional(Schema.String),
  outputTypeScript: Schema.optional(Schema.String),
  typeScriptDefinitions: Schema.optional(
    Schema.Record({ key: Schema.String, value: Schema.String }),
  ),
});
```

### Error with HTTP Status (`tools/api.ts:42-44`)

```typescript
const ToolNotFound = ToolNotFoundError.annotations(
  HttpApiSchema.annotations({ status: 404 }),
);
```

**Key pattern:** Maps domain errors to HTTP status codes.

### Secrets API (`secrets/api.ts:1-89`)

```typescript
export class SecretsApi extends HttpApiGroup.make("secrets")
  .add(
    HttpApiEndpoint.get("list")`/scopes/${scopeIdParam}/secrets`
      .addSuccess(Schema.Array(SecretRefResponse)),
  )
  .add(
    HttpApiEndpoint.get("get")`/scopes/${scopeIdParam}/secrets/${secretIdParam}`
      .addSuccess(SecretRefResponse)
      .addError(SecretNotFound),
  )
  .add(
    HttpApiEndpoint.post("set")`/scopes/${scopeIdParam}/secrets`
      .addRequest(SetSecretRequest)
      .addSuccess(SecretRefResponse)
      .addError(SecretResolutionError),
  )
  .add(
    HttpApiEndpoint.delete("remove")`/scopes/${scopeIdParam}/secrets/${secretIdParam}`
      .addSuccess(Schema.Boolean)
      .addError(SecretNotFound),
  )
  .add(
    HttpApiEndpoint.get("providers")`/scopes/${scopeIdParam}/secrets/providers`
      .addSuccess(Schema.Array(Schema.String)),
  )
  .prefix("/v1") {}
```

### Sources API (`sources/api.ts:1-80`)

```typescript
export class SourcesApi extends HttpApiGroup.make("sources")
  .add(
    HttpApiEndpoint.get("list")`/scopes/${scopeIdParam}/sources`
      .addSuccess(Schema.Array(SourceResponse)),
  )
  .add(
    HttpApiEndpoint.post("add")`/scopes/${scopeIdParam}/sources`
      .addRequest(AddSourceRequest),
  )
  .add(
    HttpApiEndpoint.delete("remove")`/scopes/${scopeIdParam}/sources/${sourceIdParam}`
      .addSuccess(Schema.Boolean),
  )
  .add(
    HttpApiEndpoint.post("refresh")`/scopes/${scopeIdParam}/sources/${sourceIdParam}/refresh`
      .addSuccess(Schema.Void),
  )
  .add(
    HttpApiEndpoint.post("detect")`/detect-sources`
      .addRequest(DetectSourcesRequest)
      .addSuccess(Schema.Array(SourceDetectionResultResponse)),
  )
  .prefix("/v1") {}
```

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Executor API (REST)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    CoreExecutorApi                                   │   │
│  │                     HttpApi.make("executor")                         │   │
│  │                                                                       │   │
│  │  .add(ToolsApi)      → GET /v1/scopes/:scopeId/tools               │   │
│  │  .add(SourcesApi)    → GET/POST /v1/scopes/:scopeId/sources        │   │
│  │  .add(SecretsApi)    → GET/POST /v1/scopes/:scopeId/secrets        │   │
│  │  .add(ExecutionsApi) → POST /v1/executions                         │   │
│  │  .add(ScopeApi)      → GET /v1/scopes                              │   │
│  │                                                                       │   │
│  │  OpenApi.annotations({ title, description })                         │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    HttpApiGroup Extensions                           │   │
│  │                                                                       │   │
│  │  Each group defines:                                                 │   │
│  │  - Endpoint paths (template literals)                                │   │
│  │  - Request schemas (for POST/PUT)                                    │   │
│  │  - Response schemas (Success)                                        │   │
│  │  - Error schemas with HTTP status annotations                        │   │
│  │  - Route prefix (/v1)                                                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Plugin Extension                                  │   │
│  │                                                                       │   │
│  │  addGroup(customGroup) → Extend API with plugin endpoints           │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### API Request Flow

```
HTTP Request
    │
    ▼
┌─────────────────────────┐
│  HttpApi Router         │
│  (CoreExecutorApi)      │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  HttpApiGroup           │
│  (e.g., ToolsApi)       │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  HttpApiEndpoint        │
│  (e.g., list)           │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Request Validation     │
│  (Schema validation)    │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Handler Implementation │
│  (SDK service call)     │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Response Serialization │
│  (Schema encoding)      │
└───────────┬─────────────┘
            │
            ▼
HTTP Response
```

---

## 7. Key Patterns

### Effect Platform Usage

The API uses **@effect/platform** for type-safe HTTP:

1. **HttpApi.make** — Root API definition
2. **HttpApiGroup.make** — Resource group definition
3. **HttpApiEndpoint** — Individual endpoint definition
4. **Schema annotations** — HTTP status mapping

### Template Literal Paths

```typescript
HttpApiEndpoint.get("list")`/scopes/${scopeIdParam}/tools`
```

Type-safe path parameters using tagged template literals.

### Response Schema Composition

```typescript
.addSuccess(Schema.Array(ToolMetadataResponse))
.addSuccess(ToolSchemaResponse)
.addSuccess(Schema.Boolean)
```

Responses are defined with Effect Schema for automatic validation and serialization.

### Error Status Mapping

```typescript
const ToolNotFound = ToolNotFoundError.annotations(
  HttpApiSchema.annotations({ status: 404 }),
);
```

Domain errors are annotated with HTTP status codes.

---

## 8. Integration Points

### Dependencies

| Package | Purpose |
|---------|---------|
| `@effect/platform` | HTTP API framework |
| `@executor/sdk` | Domain types and errors |
| `effect` | Schema and Context |

### Dependents

| Package | Relationship |
|---------|--------------|
| `@executor/apps/server` | Server implements API handlers |
| `@executor/hosts/mcp` | MCP server may proxy API endpoints |

---

## 9. Error Handling

### HTTP Status Mapping

| Error | Status | Description |
|-------|--------|-------------|
| `ToolNotFoundError` | 404 | Tool not found |
| `SecretNotFoundError` | 404 | Secret not found |
| `SecretResolutionError` | 400/500 | Secret resolution failed |
| `PolicyDeniedError` | 403 | Policy denied access |

### Error Response Format

Errors are serialized using Effect Schema with status annotations:

```typescript
const ToolNotFound = ToolNotFoundError.annotations(
  HttpApiSchema.annotations({ status: 404 }),
);
```

---

## 10. API Endpoints Summary

### Tools API (`/v1/scopes/:scopeId/tools`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/tools` | List all tools |
| GET | `/tools/:toolId/schema` | Get tool schema |

### Sources API (`/v1/scopes/:scopeId/sources`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/sources` | List all sources |
| POST | `/sources` | Add a source |
| DELETE | `/sources/:sourceId` | Remove a source |
| POST | `/sources/:sourceId/refresh` | Refresh a source |
| POST | `/detect-sources` | Detect sources from URL |

### Secrets API (`/v1/scopes/:scopeId/secrets`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/secrets` | List all secrets |
| GET | `/secrets/:secretId` | Get secret metadata |
| POST | `/secrets` | Set a secret |
| DELETE | `/secrets/:secretId` | Remove a secret |
| GET | `/providers` | List secret providers |

### Executions API (`/v1/executions`)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/` | Execute code |
| GET | `/:executionId` | Get execution status |
| POST | `/:executionId/resume` | Resume paused execution |

### Scope API (`/v1/scopes`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | List scopes |
| POST | `/` | Create scope |

---

## 11. Design Decisions

### Why Effect Platform?

1. **Type safety** — Endpoints, requests, and responses are fully typed
2. **Composability** — APIs are built from composable groups
3. **Schema integration** — Effect Schema for validation and serialization
4. **OpenAPI support** — Auto-generated API documentation

### Why Versioned Prefixes?

```typescript
.prefix("/v1")
```

1. **Future-proofing** — Can evolve API without breaking clients
2. **Clear versioning** — Clients know which version they're using
3. **Standard practice** — REST API versioning convention

### Why Modular Groups?

1. **Separation of concerns** — Each resource has its own definition
2. **Plugin extensibility** — Plugins can add their own groups
3. **Maintainability** — Easier to find and update endpoints

---

## 12. Summary

The API package provides a **type-safe, composable REST API** for the Executor system:

1. **Effect Platform** — Leverages @effect/platform for HTTP handling
2. **Schema-based** — All requests/responses use Effect Schema
3. **OpenAPI ready** — Auto-generated API documentation
4. **Extensible** — Plugin groups can extend the API
5. **Error mapping** — Domain errors mapped to HTTP status codes

The API layer enables **remote access** to Executor functionality while maintaining **type safety** and **documentation** through Effect Platform.
