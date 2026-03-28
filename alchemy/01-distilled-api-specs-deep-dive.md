---
title: "Distilled API Specs Deep Dive"
subtitle: "How distilled-* directories clone API specifications locally for code generation"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/alchemy
explored_at: 2026-03-27
---

# 01 - Distilled API Specs Deep Dive

## Overview

The `distilled-*` directories are the foundation of Alchemy's provider ecosystem. They contain **locally cloned API specifications** that enable offline code generation of Effect-native SDKs.

## Directory Layout

```
src.deployAnywhere/
├── distilled/                          # Monorepo root
│   ├── packages/
│   │   ├── core/                       # Shared utilities
│   │   ├── aws/                        # AWS SDK (Smithy)
│   │   ├── cloudflare/                 # Cloudflare SDK (TypeScript)
│   │   ├── gcp/                        # GCP SDK (Discovery)
│   │   ├── neon/                       # Neon SDK (OpenAPI)
│   │   └── ...
│   └── scripts/
│       └── create-sdk.ts               # Scaffold new SDK
│
├── distilled-cloudflare/               # Standalone spec mirror
│   └── cloudflare-typescript/          # Git submodule
│       └── src/
│           └── resources/              # TypeScript SDK source
│
├── distilled-gcp/
│   └── google-api-discovery/           # Git submodule
│       └── apis/                       # Discovery documents
│
├── distilled-neon/
│   └── neon-api-spec/                  # Git submodule
│       └── openapi.json
│
└── distilled-spec-*/                   # Other spec mirrors
```

## How Distilled-* Directories Work

### Step 1: Clone API Specs

Each `distilled-*` directory contains a **git submodule** pointing to upstream API specifications:

```bash
# Initialize submodules (shallow clone for speed)
git submodule update --init --recursive --depth=1

# Example structure after clone:
distilled/packages/cloudflare/specs/
└── cloudflare-typescript/              # Submodule
    ├── .git                            # Git dir reference
    ├── src/
    │   └── resources/
    │       ├── r2/
    │       │   └── buckets.ts          # R2 Bucket API
    │       ├── workers/
    │       │   └── scripts.ts          # Workers API
    │       └── d1/
    │           └── databases.ts        # D1 API
```

### Step 2: Parse Specifications

The generator parses the upstream source:

```typescript
// distilled/packages/cloudflare/scripts/generate.ts

import { parse } from "typescript";

// Read upstream TypeScript file
const source = await fs.readFile(
  "./specs/cloudflare-typescript/src/resources/r2/buckets.ts"
);

// Parse TypeScript AST
const ast = parse(source, {
  allowJs: true,
  jsx: false,
});

// Extract APIResource classes
for (const node of ast.statements) {
  if (isClassDeclaration(node) && extendsAPIResource(node)) {
    const serviceInfo = extractServiceInfo(node);
    await generateService(serviceInfo);
  }
}
```

### Step 3: Extract Operations

From the TypeScript AST, extract operations:

```typescript
// Input (Cloudflare SDK)
class R2Buckets extends APIResource {
  /**
   * Get a bucket by name
   * @param account_id The account ID
   * @param bucket_name The bucket name
   */
  get(
    account_id: string,
    bucket_name: string
  ): Promise<Bucket> {
    return this._client.get(
      `/accounts/${account_id}/r2/buckets/${bucket_name}`
    );
  }
}

// Extracted Operation
interface ParsedOperation {
  operationName: "get";
  urlTemplate: "/accounts/{account_id}/r2/buckets/{bucket_name}";
  pathParams: [
    { name: "account_id", type: "string", required: true },
    { name: "bucket_name", type: "string", required: true },
  ];
  queryParams: [];
  bodyParams: [];
  returnType: "Bucket";
  jsdoc: "Get a bucket by name";
}
```

### Step 4: Generate Effect-Native SDK

Output the generated SDK:

```typescript
// distilled/packages/cloudflare/src/services/r2.ts

import * as Schema from "effect/Schema";
import { API } from "@distilled.cloud/core";

// Parameter schemas
const GetBucketParams = Schema.Struct({
  account_id: Schema.String.pipe(
    Schema.propertySignature,
    Schema.description("The account ID")
  ),
  bucket_name: Schema.String.pipe(
    Schema.propertySignature,
    Schema.description("The bucket name")
  ),
});

// Operation definition
export const getBucket = API.operation({
  method: "GET",
  path: "/accounts/{account_id}/r2/buckets/{bucket_name}",
  pathParams: GetBucketParams,
  success: {
    status: 200,
    schema: BucketSchema,
  },
  errors: {
    NoSuchBucket: { status: 404, code: 10013 },
    InvalidRequest: { status: 400 },
  },
});

// Usage
import * as R2 from "@distilled.cloud/cloudflare/r2";

const program = Effect.gen(function* () {
  const bucket = yield* R2.getBucket({
    account_id: "abc123",
    bucket_name: "my-bucket",
  });

  console.log(bucket.name);
});
```

## Spec Types and Sources

Different providers use different specification formats:

| Provider | Spec Type | Source | Generator |
|----------|-----------|--------|-----------|
| AWS | Smithy | `aws-sdk-js-v3` submodule | Smithy TS generator |
| Cloudflare | TypeScript | `cloudflare-typescript` submodule | AST parser |
| GCP | Discovery JSON | `google-api-discovery` submodule | Discovery parser |
| Neon | OpenAPI | `neon-api-spec` submodule | OpenAPI generator |
| Stripe | OpenAPI | `stripe-openapi` submodule | OpenAPI generator |
| PlanetScale | OpenAPI | `planetscale-api` submodule | OpenAPI generator |

### Smithy (AWS)

```smithy
// AWS Smithy model
namespace com.amazonaws.s3

operation GetObject {
  input: GetObjectRequest
  output: GetObjectOutput
  errors: [NoSuchKey, AccessDenied]
  http: {
    method: "GET"
    uri: "/{Bucket}/{Key+}"
  }
}

structure GetObjectRequest {
  @httpLabel
  @required
  Bucket: String

  @httpLabel
  @required
  Key: String
}
```

### OpenAPI (Neon, Stripe)

```yaml
# OpenAPI spec
paths:
  /projects/{project_id}:
    get:
      operationId: getProject
      parameters:
        - name: project_id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Project'
        '404':
          description: Project not found
```

### TypeScript (Cloudflare)

```typescript
// Cloudflare TypeScript SDK
export class D1Databases extends APIResource {
  @APIResource.method()
  @APIResource.path("/accounts/{account_id}/d1/database/{database_uuid}")
  get(
    account_id: string,
    database_uuid: string
  ): Promise<D1Database> {
    return this._client.get(...);
  }
}
```

## The Patch System

Generated SDKs may have inaccuracies. The patch system fixes them:

### Patch File Structure

```json
// distilled/packages/cloudflare/patches/r2/getBucket.json
{
  "errors": {
    "NoSuchBucket": [
      {
        "code": 10013,
        "message": {
          "includes": "The specified bucket does not exist"
        }
      }
    ],
    "AccessDenied": [
      {
        "code": 10000,
        "status": 403
      }
    ]
  },
  "response": {
    "properties": {
      "created_at": {
        "type": "string"
      }
    }
  }
}
```

### Applying Patches

```typescript
// In generate.ts
async function applyPatch(
  operation: ParsedOperation,
  patch: OperationPatch
): Promise<ParsedOperation> {
  // Add error definitions
  for (const [errorName, matchers] of Object.entries(patch.errors)) {
    operation.errors.push({
      name: errorName,
      matchers,
    });
  }

  // Modify response schema
  if (patch.response) {
    operation.responseSchema = applyPropertyPatches(
      operation.responseSchema,
      patch.response.properties
    );
  }

  return operation;
}
```

## Creating a New SDK Package

Use the scaffold script:

```bash
# Create new SDK from OpenAPI spec
cd distilled/
bun run create-sdk myprovider \
  --specs https://api.myprovider.com/openapi.json

# This creates:
# distilled/packages/myprovider/
# ├── src/
# │   ├── client.ts
# │   ├── credentials.ts
# │   └── services/
# │       └── index.ts
# ├── scripts/
# │   └── generate.ts
# ├── specs/
# │   └── myprovider-openapi/  # Submodule
# ├── patches/
# │   └── ...
# └── package.json
```

## Replication in ewe_platform

For the `ewe_platform` project, replicate this pattern using Valtron:

### Directory Structure

```
ewe_platform/backends/foundation_core/
├── src/
│   ├── generated/
│   │   ├── specs/
│   │   │   ├── cloudflare/          # Git submodule
│   │   │   ├── aws/                 # Git submodule
│   │   │   └── gcp/                 # Git submodule
│   │   ├── scripts/
│   │   │   └── generate.valtron     # Generator
│   │   └── output/
│   │       ├── cloudflare.val       # Generated types
│   │       ├── aws.val
│   │       └── gcp.val
│   └── providers/
│       ├── cloudflare.valtron       # Provider implementation
│       ├── aws.valtron
│       └── gcp.valtron
```

### Generator Pattern

```valtron
// generate.valtron
// Pseudo-code for Valtron-based generator

spec load_cloudflare_spec() {
  // Read from specs/cloudflare/src/resources/
  // Parse TypeScript AST
  // Extract APIResource classes
  return operations
}

transform operation_to_valtron(op) {
  // Convert TypeScript operation to Valtron type
  return valtron_type {
    name: op.name,
    method: op.method,
    path: op.path,
    params: op.params,
    response: op.response,
    errors: op.errors
  }
}

emit generate_sdk(operations) {
  // Write output/cloudflare.val
  for op in operations {
    write operation_to_valtron(op)
  }
}

main {
  spec = load_cloudflare_spec()
  operations = parse(spec)
  generate_sdk(operations)
}
```

### Generated Valtron Types

```valtron
// output/cloudflare.val

type R2Bucket {
  name: String,
  created_at: DateTime,
  location: String
}

operation GetBucket {
  method: GET,
  path: "/accounts/{account_id}/r2/buckets/{bucket_name}",
  path_params: {
    account_id: String,
    bucket_name: String
  },
  response: R2Bucket,
  errors: [NoSuchBucket, AccessDenied]
}

operation ListBuckets {
  method: GET,
  path: "/accounts/{account_id}/r2/buckets",
  path_params: { account_id: String },
  response: List<R2Bucket>,
  errors: [InvalidAccount]
}
```

## Testing Generated SDKs

### TDD Workflow

1. **Write test** that triggers an error
2. **Run with DEBUG=1** to see raw response
3. **Add error to patch** file
4. **Regenerate**: `bun run generate`
5. **Import typed error** and handle it

```typescript
// test/r2.test.ts
import { describe, it, expect } from "vitest";
import * as R2 from "../src/services/r2";
import { Effect, Layer } from "effect";
import * as FetchHttpClient from "effect/unstable/http/FetchHttpClient";

describe("R2", () => {
  it("handles NoSuchBucket error", async () => {
    const program = Effect.gen(function* () {
      yield* R2.getBucket({
        account_id: "test",
        bucket_name: "nonexistent",
      });
    });

    const CloudflareTest = Layer.mergeAll(
      FetchHttpClient.layer,
      Credentials.fromEnv()
    );

    const result = await program.pipe(
      Effect.provide(CloudflareTest),
      Effect.flip,  // Get error instead of throwing
      Effect.runPromise
    );

    expect(result._tag).toBe("NoSuchBucket");
  });
});
```

## Performance Considerations

### Submodule Optimization

```bash
# .gitmodules
[submodule "specs/cloudflare-typescript"]
  path = specs/cloudflare-typescript
  url = https://github.com/cloudflare/cloudflare-typescript
  shallow = true           # Shallow clone
  ignore = dirty           # Don't scan working tree (faster git status)
```

### Git Configuration

```bash
# Global config (once per machine)
git config --global fetch.recurseSubmodules on-demand
git config --global push.recurseSubmodules on-demand

# Local config (per clone)
git config --local diff.ignoreSubmodules dirty
git config --local status.submoduleSummary false
```

## Spec Update Workflow

```bash
# 1. Update submodule to latest
cd distilled/packages/cloudflare/
bun run specs:update

# 2. Regenerate SDK
bun run generate

# 3. Run tests
bun run test

# 4. Check for new errors
# If tests fail with unknown errors:
# - Run with DEBUG=1
# - Add error to patch file
# - Re-run generate

# 5. Commit changes
git add specs/cloudflare-typescript
git add src/services/
git add patches/
git commit -m "Update Cloudflare SDK to latest"
```

## Summary

The `distilled-*` directories:

1. **Clone API specs locally** via git submodules
2. **Parse specifications** (TypeScript AST, Smithy, OpenAPI, Discovery)
3. **Generate Effect-native SDKs** with typed errors
4. **Apply patches** to fix spec inaccuracies
5. **Enable offline development** - no network calls needed

For `ewe_platform`, replicate this pattern with:
- Git submodules for specs
- Valtron-based generator
- Generated `.val` types
- Patch system for fixes

## Next Steps

- [02-provider-integration-deep-dive.md](./02-provider-integration-deep-dive.md) - Cloud provider integration patterns
- [03-resource-lifecycle-deep-dive.md](./03-resource-lifecycle-deep-dive.md) - Resource create/update/delete lifecycle
