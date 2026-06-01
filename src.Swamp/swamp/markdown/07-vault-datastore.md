---
title: Vault and Datastore
prev: 06-model-system.md
next: 08-skills-claude.md
---

# Vault and Datastore

Swamp provides secure secret storage via the Vault system and pluggable data persistence via the Datastore abstraction.

## Vault System

### Purpose

Vaults provide secure storage for secrets (passwords, API keys, tokens) that can be referenced in definitions and workflows via CEL expressions.

```yaml
apiVersion: swamp.systeminit.com/v1
kind: Definition
spec:
  model: aws/rds-postgres
  arguments:
    # Reference a vault secret
    master_password: ${vault("prod-db-password")}
```

### Vault Providers

**Source:** `swamp-extensions/vault/`

| Provider | Backend | Authentication |
|----------|---------|------------------|
| `1password` | 1Password vaults | Service account token |
| `aws-secrets-manager` | AWS Secrets Manager | IAM role |
| `azure-keyvault` | Azure Key Vault | Managed identity |

### Vault Interface

**Source:** `swamp/src/domain/vaults/types.ts`

```typescript
export interface VaultProvider {
  readonly name: string;

  // Retrieve secret
  get(secretName: string): Promise<string>;

  // Store secret
  set(secretName: string, value: string): Promise<void>;

  // List available secrets
  list(): Promise<string[]>;

  // Check if provider is available
  isAvailable(): Promise<boolean>;
}
```

### Built-in Vault

**Source:** `swamp/src/domain/vaults/local.ts`

Swamp includes a local vault for development:

```typescript
// local.ts
export class LocalVault implements VaultProvider {
  name = "local";

  async get(secretName: string): Promise<string> {
    // Read from ~/.swamp/vault/
    const path = join(homedir(), ".swamp", "vault", secretName);
    return await Deno.readTextFile(path);
  }

  async set(secretName: string, value: string): Promise<void> {
    const dir = join(homedir(), ".swamp", "vault");
    await ensureDir(dir);
    await Deno.writeTextFile(join(dir, secretName), value);
  }
}
```

**Aha:** The local vault stores secrets in plaintext. Never use in production.

### 1Password Integration

**Source:** `swamp-extensions/vault/1password/`

```typescript
// main.ts
export default defineExtension({
  vaults: [
    {
      name: "1password",
      async isAvailable() {
        return await commandExists("op");
      },

      async get(secretName: string) {
        const { stdout } = await exec([
          "op", "read",
          `op://vault/${secretName}/password`
        ]);
        return stdout.trim();
      }
    }
  ]
});
```

### Vault Resolution

**Source:** `swamp/src/domain/vaults/resolver.ts`

```typescript
// resolver.ts (simplified)
export class VaultResolver {
  private providers: Map<string, VaultProvider> = new Map();

  register(provider: VaultProvider) {
    this.providers.set(provider.name, provider);
  }

  async resolve(expression: string): Promise<string> {
    // Parse "vault('secret-name')" or "vault('provider', 'secret-name')"
    const { providerName, secretName } = this.parse(expression);

    const provider = this.providers.get(providerName ?? "local");
    if (!provider) {
      throw new VaultProviderNotFoundError(providerName);
    }

    return await provider.get(secretName);
  }
}
```

### CEL Vault Function

**Source:** `swamp/src/infrastructure/cel/functions.ts`

```typescript
// Register vault function in CEL
const vaultFunction = {
  name: "vault",
  args: [{ type: "string" }, { type: "string", optional: true }],
  returnType: "string",
  impl: async (ref: string, provider?: string) => {
    return await vaultResolver.resolve(ref, provider);
  }
};
```

## Datastore System

### Purpose

The Datastore provides pluggable storage for:
- Data artifacts (outputs from model runs)
- Workflow state
- Extension data

### Datastore Interface

**Source:** `swamp/src/domain/datastore/interface.ts`

```typescript
export interface Datastore {
  // Read artifact by content hash
  read(id: string): Promise<Uint8Array>;

  // Write content, return content hash
  write(content: Uint8Array): Promise<string>;

  // Check if artifact exists
  exists(id: string): Promise<boolean>;

  // Delete artifact
  delete(id: string): Promise<void>;

  // List all artifacts (optional)
  list?(): Promise<string[]>;
}
```

### Content Addressing

**Aha:** Datastores are content-addressed. The ID is the hash of the content:

```typescript
async write(content: Uint8Array): Promise<string> {
  const hash = await sha256(content);
  await this.store(hash, content);
  return hash;
}
```

This provides:
- Deduplication (same content = same ID)
- Integrity verification
- Caching efficiency

### Local Datastore

**Source:** `swamp/src/infrastructure/persistence/local_datastore.ts`

```typescript
// local_datastore.ts
export class LocalDatastore implements Datastore {
  private basePath: string;

  constructor(repoRoot: string) {
    this.basePath = join(repoRoot, ".swamp", "data");
  }

  async write(content: Uint8Array): Promise<string> {
    const id = await sha256(content);
    const path = this.objectPath(id);
    await ensureDir(dirname(path));
    await Deno.writeFile(path, content);
    return id;
  }

  async read(id: string): Promise<Uint8Array> {
    const path = this.objectPath(id);
    return await Deno.readFile(path);
  }

  private objectPath(id: string): string {
    // Store as .swamp/data/ab/cdabcdef...
    // First 2 chars as directory for efficiency
    return join(this.basePath, id.slice(0, 2), id.slice(2));
  }
}
```

### S3 Datastore

**Source:** `swamp-extensions/datastore/s3/`

```typescript
// s3_datastore.ts
export class S3Datastore implements Datastore {
  private client: S3Client;
  private bucket: string;

  async write(content: Uint8Array): Promise<string> {
    const id = await sha256(content);
    await this.client.putObject({
      Bucket: this.bucket,
      Key: `data/${id.slice(0, 2)}/${id.slice(2)}`,
      Body: content
    });
    return id;
  }

  async read(id: string): Promise<Uint8Array> {
    const response = await this.client.getObject({
      Bucket: this.bucket,
      Key: `data/${id.slice(0, 2)}/${id.slice(2)}`
    });
    return new Uint8Array(await response.Body.arrayBuffer());
  }
}
```

### Datastore Selection

**Source:** `swamp/src/domain/datastore/selector.ts`

```typescript
// selector.ts
export function selectDatastore(config: Config): Datastore {
  switch (config.datastore.type) {
    case "local":
      return new LocalDatastore(config.repoRoot);
    case "s3":
      return new S3Datastore(config.datastore.s3);
    case "gcs":
      return new GCSDatastore(config.datastore.gcs);
    default:
      throw new UnknownDatastoreTypeError(config.datastore.type);
  }
}
```

### Configuration

**Source:** `.swamp/config.yaml`

```yaml
# Local datastore (default)
datastore:
  type: local

# S3 datastore
datastore:
  type: s3
  s3:
    bucket: my-swamp-data
    region: us-east-1
    prefix: swamp/

# GCS datastore
datastore:
  type: gcs
  gcs:
    bucket: my-swamp-data
    project: my-project
```

## Data Management

**Source:** `swamp/src/domain/data/`

### Tagging

Data artifacts can be tagged for organization:

```typescript
// src/domain/data/tagger.ts
export class DataTagger {
  async tag(artifactId: string, tag: string): Promise<void> {
    // Creates a reference: tags/my-tag → artifacts/{artifactId}
    await this.datastore.writeTag(tag, artifactId);
  }

  async resolveTag(tag: string): Promise<string | null> {
    return await this.datastore.readTag(tag);
  }
}
```

### Garbage Collection

Unused data can be garbage collected:

```typescript
// gc.ts
export async function garbageCollect(
  datastore: Datastore,
  references: Set<string>
): Promise<void> {
  for (const id of await datastore.list()) {
    if (!references.has(id)) {
      await datastore.delete(id);
    }
  }
}
```

## Security Model

### Vault Security

| Provider | Encryption | Access Control |
|----------|------------|----------------|
| Local | None (plaintext) | Filesystem permissions |
| 1Password | AES-256-GCM | 1Password policies |
| AWS SM | KMS | IAM policies |
| Azure KV | RSA-2048 | Azure RBAC |

### Datastore Security

| Datastore | Encryption at Rest | Encryption in Transit |
|-----------|-------------------|----------------------|
| Local | Filesystem | N/A |
| S3 | SSE-S3/SSE-KMS | TLS 1.2+ |
| GCS | Google-managed | TLS 1.2+ |

## Commands

**Source:** `swamp/src/cli/commands/`

### Vault Commands

| Command | Description |
|---------|-------------|
| `vault:create` | Create vault configuration |
| `vault:list` | List vaults |
| `vault:secret:create` | Create secret |
| `vault:secret:show` | Show secret (masked) |
| `vault:secret:delete` | Delete secret |

### Datastore Commands

| Command | Description |
|---------|-------------|
| `datastore:show` | Show datastore config |
| `data:show` | Show data artifact |
| `data:tag` | Tag data artifact |
| `data:untag` | Remove tag |
| `data:gc` | Run garbage collection |

## Next Steps

Continue to [Claude Skills →](08-skills-claude.html) for AI integration and Claude Code skills.
