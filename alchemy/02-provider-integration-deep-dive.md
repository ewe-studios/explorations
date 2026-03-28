---
title: "Provider Integration Deep Dive"
subtitle: "Cloudflare, AWS, GCP provider patterns and implementation strategies"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/alchemy
explored_at: 2026-03-27
---

# 02 - Provider Integration Deep Dive

## Overview

Alchemy providers are the bridge between the core IaC engine and cloud APIs. This document explores how providers are implemented for Cloudflare, AWS, and GCP, and how to replicate these patterns in `ewe_platform`.

## Provider Anatomy

### Provider Structure

```
alchemy/
└── src/
    └── {provider}/
        ├── index.ts              # Barrel exports
        ├── api.ts                # HTTP client
        ├── auth.ts               # Authentication
        ├── user.ts               # Account/user discovery
        ├── {resource}.ts         # Individual resources
        └── types.ts              # Shared types
```

### Provider Registration

```typescript
// alchemy/src/{provider}/{resource}.ts
import { Resource, Context } from "alchemy";

export interface MyResourceProps {
  name: string;
  config: SomeConfig;
}

export interface MyResource extends Resource<"provider::MyResource"> {
  name: string;
  id: string;
  status: string;
}

export const MyResource = Resource(
  "provider::MyResource",
  async function (
    this: Context<MyResource>,
    id: string,
    props: MyResourceProps
  ): Promise<MyResource> {
    // Create, Update, Delete lifecycle
  }
);
```

## Cloudflare Provider

### Architecture

```
alchemy/src/cloudflare/
├── api.ts                    # CloudflareApi HTTP client
├── auth.ts                   # Auth header resolution
├── user.ts                   # Account/user discovery
├── worker.ts                 # Worker resource
├── bindings.ts               # Worker binding definitions
├── d1-database.ts            # D1 Database
├── kv-namespace.ts           # KV Namespace
├── bucket.ts                 # R2 Bucket
├── queue.ts                  # Queue
├── durable-object-namespace.ts
├── vectorize-index.ts
├── hyperdrive.ts
├── zone.ts                   # DNS Zone
├── dns-records.ts            # DNS Records
└── wrangler.json.ts          # Wrangler config generation
```

### API Client

```typescript
// alchemy/src/cloudflare/api.ts
export class CloudflareApi {
  readonly accountId: string;
  readonly authHeaders: Record<string, string>;

  constructor(options: {
    apiToken?: string;
    apiKey?: string;
    email?: string;
    accountId?: string;
  }) {
    // Auto-discover account ID if not provided
    this.accountId = options.accountId ?? this.discoverAccountId();
    this.authHeaders = this.resolveAuth(options);
  }

  private resolveAuth(options): Record<string, string> {
    if (options.apiToken) {
      return { Authorization: `Bearer ${options.apiToken}` };
    }
    if (options.apiKey && options.email) {
      return {
        "X-Auth-Key": options.apiKey,
        "X-Auth-Email": options.email,
      };
    }
    throw new Error("No auth provided");
  }

  async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {
    const url = `https://api.cloudflare.com/client/v4${path}`;
    const response = await fetch(url, {
      method,
      headers: {
        ...this.authHeaders,
        "Content-Type": "application/json",
      },
      body: body ? JSON.stringify(body) : undefined,
    });

    const result = await response.json();
    if (!result.success) {
      throw new CloudflareApiError(result.errors);
    }
    return result.data;
  }
}
```

### Worker Resource (Complex Example)

```typescript
// alchemy/src/cloudflare/worker.ts
export interface WorkerProps {
  name?: string;
  entrypoint: string;
  bindings?: WorkerBindings;
  compatibilityDate?: string;
  compatibilityFlags?: string[];
  tailConsumers?: TailConsumer[];
  migration?: WorkerMigration;
}

export interface Worker extends Resource<"cloudflare::Worker"> {
  id: string;
  name: string;
  url: string;
  entrypoint: string;
  bindings: WorkerBindings;
}

export const Worker = Resource(
  "cloudflare::Worker",
  async function (
    this: Context<Worker>,
    id: string,
    props: WorkerProps
  ): Promise<Worker> {
    const api = new CloudflareApi();
    const name = props.name ?? this.scope.createPhysicalName(id);

    if (this.phase === "create") {
      // 1. Bundle the worker code
      const bundle = await bundleWorker({
        entrypoint: props.entrypoint,
        bindings: props.bindings,
      });

      // 2. Upload the script
      const formData = new FormData();
      formData.append(
        "metadata",
        JSON.stringify({
          main_module: "index.js",
          bindings: serializeBindings(props.bindings),
          compatibility_date: props.compatibilityDate,
          compatibility_flags: props.compatibilityFlags,
        })
      );
      formData.append("index.js", bundle.code);

      await api.request(
        "PUT",
        `/accounts/${api.accountId}/workers/scripts/${name}`,
        formData
      );

      // 3. Create bindings (D1, KV, R2, etc.)
      for (const binding of props.bindings ?? []) {
        await this.bindResource(api, name, binding);
      }

      // 4. Set up routes
      if (props.routes) {
        for (const route of props.routes) {
          await api.request("POST", `/zones/${route.zoneId}/workers/routes`, {
            pattern: route.pattern,
            script: name,
          });
        }
      }

      return this.create({
        id: name,
        name,
        url: `https://${name}.${api.accountId}.workers.dev`,
        entrypoint: props.entrypoint,
        bindings: props.bindings,
      });

    } else if (this.phase === "update") {
      // Compare bundle hash, update if changed
      const oldBundle = await this.get("bundleHash");
      const newBundle = await bundleWorker({ entrypoint: props.entrypoint });

      if (oldBundle !== newBundle.hash || propsChanged(this.props, props)) {
        // Update script
        // Update bindings
        // Update routes
      }

      return this.create({
        id: name,
        name,
        url: `https://${name}.${api.accountId}.workers.dev`,
        entrypoint: props.entrypoint,
        bindings: props.bindings,
      });
    }

    // Delete phase (handled by Context.destroy())
    await api.request(
      "DELETE",
      `/accounts/${api.accountId}/workers/scripts/${name}`
    );
    this.destroy();
  }
);
```

### Binding System

```typescript
// alchemy/src/cloudflare/bindings.ts
export type WorkerBinding =
  | D1Binding
  | KVBinding
  | R2Binding
  | QueueBinding
  | DurableObjectBinding
  | SecretBinding
  | ServiceBinding;

export interface D1Binding {
  type: "d1";
  name: string;  // Binding name in worker
  database: string | D1Database;  // Reference or ID
}

export interface KVBinding {
  type: "kv";
  name: string;
  namespace: string | KVNamespace;
}

export interface R2Binding {
  type: "r2";
  name: string;
  bucket: string | R2Bucket;
}

// Serialize bindings for Worker upload
function serializeBindings(
  bindings: WorkerBinding[]
): Record<string, unknown> {
  const result: Record<string, unknown> = {};

  for (const binding of bindings) {
    switch (binding.type) {
      case "d1":
        result[binding.name] = {
          type: "d1",
          id: resolveId(binding.database),
        };
        break;
      case "kv":
        result[binding.name] = {
          type: "kv_namespace",
          namespace_id: resolveId(binding.namespace),
        };
        break;
      case "r2":
        result[binding.name] = {
          type: "r2_bucket",
          bucket_name: resolveId(binding.bucket),
        };
        break;
    }
  }

  return result;
}
```

## AWS Provider

### Architecture

```
alchemy/src/aws/
├── credentials.ts            # Credential resolution
├── account-id.ts             # Account ID discovery
├── function.ts               # Lambda Function
├── bucket.ts                 # S3 Bucket
├── table.ts                  # DynamoDB Table
├── role.ts                   # IAM Role
├── policy.ts                 # IAM Policy
├── policy-attachment.ts      # Policy attachment
├── queue.ts                  # SQS Queue
└── ses.ts                    # SES Email
```

### Credential Resolution

```typescript
// alchemy/src/aws/credentials.ts
export interface AwsCredentials {
  accessKeyId: string;
  secretAccessKey: string;
  sessionToken?: string;
}

export async function resolveCredentials(): Promise<AwsCredentials> {
  // Try environment variables first
  if (process.env.AWS_ACCESS_KEY_ID && process.env.AWS_SECRET_ACCESS_KEY) {
    return {
      accessKeyId: process.env.AWS_ACCESS_KEY_ID,
      secretAccessKey: process.env.AWS_SECRET_ACCESS_KEY,
      sessionToken: process.env.AWS_SESSION_TOKEN,
    };
  }

  // Try shared credentials file (~/.aws/credentials)
  const credentialsFile = path.join(os.homedir(), ".aws", "credentials");
  if (fs.existsSync(credentialsFile)) {
    const profile = process.env.AWS_PROFILE || "default";
    const credentials = parseIniFile(credentialsFile)[profile];
    if (credentials) {
      return {
        accessKeyId: credentials.aws_access_key_id,
        secretAccessKey: credentials.aws_secret_access_key,
      };
    }
  }

  // Try EC2 instance metadata (IMDSv2)
  if (isRunningOnEC2()) {
    const token = await fetch("http://169.254.169.254/latest/api/token", {
      method: "PUT",
      headers: { "X-aws-ec2-metadata-token-ttl-seconds": "21600" },
    });

    const credentials = await fetch(
      "http://169.254.169.254/latest/meta-data/iam/security-credentials/",
      { headers: { "X-aws-ec2-metadata-token": await token.text() } }
    );

    return parseImdsCredentials(await credentials.text());
  }

  throw new Error("No AWS credentials found");
}
```

### Lambda Function Resource

```typescript
// alchemy/src/aws/function.ts
export interface FunctionProps {
  functionName?: string;
  runtime: string;
  handler: string;
  code: string;  // Path to code directory
  role: string | Role;  // IAM Role reference
  environment?: Record<string, string>;
  memorySize?: number;
  timeout?: number;
  tags?: Record<string, string>;
}

export interface Function extends Resource<"aws::Function"> {
  arn: string;
  name: string;
  url?: string;  // If function URL configured
}

export const Function = Resource(
  "aws::Function",
  async function (
    this: Context<Function>,
    id: string,
    props: FunctionProps
  ): Promise<Function> {
    const credentials = await resolveCredentials();
    const region = process.env.AWS_REGION || "us-east-1";
    const name = props.functionName ?? this.scope.createPhysicalName(id);

    // Sign AWS request (SigV4)
    const signer = new AwsSigner(credentials, region);

    if (this.phase === "create") {
      // 1. Zip the code
      const zipBuffer = await zipDirectory(props.code);

      // 2. Create function
      const response = await signer.request(
        "POST",
        `https://lambda.${region}.amazonaws.com/2015-03-31/functions`,
        {
          FunctionName: name,
          Runtime: props.runtime,
          Handler: props.handler,
          Role: resolveId(props.role),
          Code: { ZipFile: zipBuffer.toString("base64") },
          Environment: props.environment
            ? { Variables: props.environment }
            : undefined,
          MemorySize: props.memorySize,
          Timeout: props.timeout,
          Tags: props.tags,
        }
      );

      const result = await response.json();

      // 3. Wait for function to become Active
      await waitForFunctionActive(name, region, signer);

      return this.create({
        arn: result.FunctionArn,
        name: result.FunctionName,
        url: result.FunctionUrl,
      });

    } else if (this.phase === "update") {
      // Update configuration
      if (propsChanged(this.props, props)) {
        await signer.request(
          "PUT",
          `https://lambda.${region}.amazonaws.com/2015-03-31/functions/${name}/configuration`,
          {
            Environment: props.environment
              ? { Variables: props.environment }
              : undefined,
            MemorySize: props.memorySize,
            Timeout: props.timeout,
          }
        );
      }

      // Update code if changed
      const oldHash = await this.get("codeHash");
      const newHash = await hashDirectory(props.code);
      if (oldHash !== newHash) {
        const zipBuffer = await zipDirectory(props.code);
        await signer.request(
          "PUT",
          `https://lambda.${region}.amazonaws.com/2015-03-31/functions/${name}/code`,
          { ZipFile: zipBuffer.toString("base64") }
        );
        await this.set("codeHash", newHash);
      }

      return this.create({
        arn: this.output.arn,
        name,
        url: this.output.url,
      });
    }

    // Delete
    await signer.request(
      "DELETE",
      `https://lambda.${region}.amazonaws.com/2015-03-31/functions/${name}`
    );
    this.destroy();
  }
);
```

### AWS SigV4 Signing

```typescript
// alchemy/src/aws/utils.ts
import { sign } from "aws4fetch";

export class AwsSigner {
  constructor(
    private credentials: AwsCredentials,
    private region: string
  ) {}

  async request(
    method: string,
    url: string,
    body?: unknown
  ): Promise<Response> {
    const signedRequest = await sign(
      {
        method,
        url,
        headers: {
          "Content-Type": "application/json",
        },
        body: body ? JSON.stringify(body) : undefined,
      },
      {
        accessKeyId: this.credentials.accessKeyId,
        secretAccessKey: this.credentials.secretAccessKey,
        sessionToken: this.credentials.sessionToken,
        service: "lambda",  // or s3, dynamodb, etc.
        region: this.region,
      }
    );

    return fetch(signedRequest.url, {
      method: signedRequest.method,
      headers: signedRequest.headers,
      body: signedRequest.body,
    });
  }
}
```

## GCP Provider

### Architecture

GCP uses Discovery Documents for API specifications:

```
distilled/packages/gcp/
├── specs/
│   └── google-api-discovery/    # Git submodule
│       └── apis/
│           ├── compute/
│           │   └── v1.json
│           ├── storage/
│           │   └── v1.json
│           └── cloudfunctions/
│               └── v1.json
├── scripts/
│   └── generate.ts              # Discovery doc parser
└── src/
    └── services/
        ├── compute.ts
        ├── storage.ts
        └── cloudfunctions.ts
```

### Discovery Document Parsing

```typescript
// distilled/packages/gcp/scripts/generate.ts
interface DiscoveryDocument {
  id: string;
  name: string;
  version: string;
  rootUrl: string;
  servicePath: string;
  resources: Record<string, ResourceDefinition>;
  schemas: Record<string, SchemaDefinition>;
}

async function parseDiscoveryDocument(
  path: string
): Promise<DiscoveryDocument> {
  const content = await fs.readFile(path, "utf8");
  return JSON.parse(content);
}

function generateService(doc: DiscoveryDocument): string {
  let output = `
import { API } from "@distilled.cloud/core";
import * as Schema from "effect/Schema";

`;

  for (const [resourceName, resource] of Object.entries(doc.resources)) {
    for (const [methodName, method] of Object.entries(resource.methods)) {
      output += generateOperation(resourceName, methodName, method, doc);
    }
  }

  return output;
}

function generateOperation(
  resourceName: string,
  methodName: string,
  method: MethodDefinition,
  doc: DiscoveryDocument
): string {
  const params = method.parameters || {};
  const pathParams = Object.entries(params)
    .filter(([, p]) => p.location === "path")
    .map(([name, p]) => `${name}: Schema.String`);

  const queryParams = Object.entries(params)
    .filter(([, p]) => p.location === "query")
    .map(([name, p]) => `${name}: Schema.String`);

  return `
export const ${resourceName}${capitalize(methodName)} = API.operation({
  method: "${method.httpMethod}",
  path: "${method.path}",
  pathParams: { ${pathParams.join(", ")} },
  queryParams: { ${queryParams.join(", ")} },
  success: {
    status: 200,
    schema: ${doc.schemas[method.response?.$ref]?.schema || "Schema.Unknown"},
  },
});
`;
}
```

## Provider Comparison

| Aspect | Cloudflare | AWS | GCP |
|--------|------------|-----|-----|
| **Auth** | API Token / API Key | SigV4 signing | OAuth2 JWT |
| **API Style** | REST JSON | REST + Query | REST JSON |
| **Spec Format** | TypeScript | Smithy | Discovery JSON |
| **Account ID** | Auto-discovered | From ARN/creds | From project ID |
| **Rate Limits** | Per API token | Per account | Per project |
| **Pagination** | `result_info` | `NextToken` | `pageToken` |

## Resource Reference Pattern

All providers support referencing resources by ID or by Resource:

```typescript
// Cloudflare: D1 Database reference
export interface D1DatabaseProps {
  name?: string;
  primaryLocationHint?: string;
}

export interface D1Database extends Resource<"cloudflare::D1Database"> {
  id: string;
  name: string;
  uuid: string;
}

// Consumer resource can reference by ID or Resource
export interface WorkerProps {
  bindings: {
    type: "d1";
    name: string;
    database: string | D1Database;  // Accept both
  }[];
}

// Helper to resolve reference
function resolveId(resource: string | D1Database): string {
  return typeof resource === "string" ? resource : resource.uuid;
}
```

## Replication in ewe_platform

### Provider Structure for Valtron

```valtron
// ewe_platform/backends/foundation_core/src/providers/cloudflare.valtron

// Provider definition
provider Cloudflare {
  credentials: {
    api_token: String?
    api_key: String?
    email: String?
  },

  account_id: String,

  resources: [
    Worker,
    D1Database,
    KVNamespace,
    R2Bucket,
    Queue,
  ]
}

// Resource definition
resource Worker {
  props: {
    name: String?,
    entrypoint: String,
    bindings: List<Binding>,
    compatibility_date: String?,
  },

  output: {
    id: String,
    name: String,
    url: String,
  },

  lifecycle: {
    create: create_worker,
    update: update_worker,
    delete: delete_worker,
  }
}

// Operation definitions
operation create_worker(props: WorkerProps) -> WorkerOutput {
  // Bundle code
  let bundle = bundle_worker(props.entrypoint)

  // Upload to Cloudflare
  let response = http_put(
    "https://api.cloudflare.com/client/v4/accounts/{account_id}/workers/scripts/{name}",
    headers = provider.auth_headers,
    body = bundle
  )

  // Return output
  WorkerOutput {
    id: response.result.id,
    name: response.result.id,
    url: "https://" ++ response.result.id ++ ".workers.dev"
  }
}

operation update_worker(id: String, props: WorkerProps) -> WorkerOutput {
  // Compare and update
  if bundle_changed(props.entrypoint) {
    create_worker(props)
  } else {
    get_worker(id)
  }
}

operation delete_worker(id: String) {
  http_delete(
    "https://api.cloudflare.com/client/v4/accounts/{account_id}/workers/scripts/{id}",
    headers = provider.auth_headers
  )
}
```

### Credential Resolution for Valtron

```valtron
// ewe_platform/backends/foundation_core/src/auth/cloudflare.valtron

operation resolve_cloudflare_auth() -> AuthCredentials {
  // Try environment variables
  if env_exists("CLOUDFLARE_API_TOKEN") {
    return AuthCredentials {
      type: "token",
      token: env_get("CLOUDFLARE_API_TOKEN")
    }
  }

  // Try API key + email
  if env_exists("CLOUDFLARE_API_KEY") && env_exists("CLOUDFLARE_EMAIL") {
    return AuthCredentials {
      type: "key_email",
      api_key: env_get("CLOUDFLARE_API_KEY"),
      email: env_get("CLOUDFLARE_EMAIL")
    }
  }

  // Try config file
  let config_path = home_dir() ++ "/.config/alchemy/cloudflare.toml"
  if file_exists(config_path) {
    let config = parse_toml(read_file(config_path))
    return AuthCredentials {
      type: "token",
      token: config.api_token
    }
  }

  error "No Cloudflare credentials found"
}
```

## Best Practices

### 1. Auto-Discovery

```typescript
// Always auto-discover account/project IDs when possible
async function discoverAccountId(auth: Auth): Promise<string> {
  const response = await api.request("GET", "/user/tokens/verify");
  return response.result.account.id;
}
```

### 2. Retry Logic

```typescript
// Implement exponential backoff for API calls
async function requestWithRetry<T>(
  fn: () => Promise<T>,
  maxRetries = 3
): Promise<T> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (error.status >= 500 && i < maxRetries - 1) {
        await sleep(Math.pow(2, i) * 1000);
        continue;
      }
      throw error;
    }
  }
}
```

### 3. Idempotency

```typescript
// Ensure create operations are idempotent
async function createBucket(name: string): Promise<Bucket> {
  try {
    return await api.request("POST", "/buckets", { name });
  } catch (error) {
    if (error.code === "BucketAlreadyExists") {
      return await api.request("GET", `/buckets/${name}`);
    }
    throw error;
  }
}
```

### 4. Resource Tagging

```typescript
// Always tag resources for cost tracking
const tags = {
  ManagedBy: "alchemy",
  App: scope.appName,
  Stage: scope.stage,
  Resource: id,
};
```

## Summary

Provider integration patterns:

1. **API Client** - Shared HTTP client with auth
2. **Credential Resolution** - Environment, config files, metadata
3. **Resource Lifecycle** - Create, update, delete handlers
4. **Reference Resolution** - Accept both IDs and Resource objects
5. **Error Handling** - Typed errors, retry logic
6. **Idempotency** - Safe to run multiple times

For `ewe_platform`, replicate with:
- Provider definitions in Valtron
- Resource lifecycle operations
- Auth resolution operations
- Generated SDK integration

## Next Steps

- [03-resource-lifecycle-deep-dive.md](./03-resource-lifecycle-deep-dive.md) - Create, update, delete, drift detection
- [04-state-management-deep-dive.md](./04-state-management-deep-dive.md) - Remote state, locking, versioning
