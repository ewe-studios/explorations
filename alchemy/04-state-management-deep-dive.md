---
title: "State Management Deep Dive"
subtitle: "Remote state, locking, versioning, and state store patterns"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/alchemy
explored_at: 2026-03-27
---

# 04 - State Management Deep Dive

## Overview

State is the foundation of Infrastructure as Code. Alchemy's state management tracks what resources exist, their configuration, and their current status. This document explores state stores, locking, versioning, and state migration patterns.

## State Interface

```typescript
// alchemy/src/state.ts
export interface State<
  Kind extends string = string,
  Props extends ResourceProps = ResourceProps,
  Out extends Resource = Resource,
> = {
  status:
    | "creating"
    | "created"
    | "updating"
    | "updated"
    | "deleting"
    | "deleted";
  kind: Kind;
  id: string;
  fqn: string;
  seq: number;
  data: Record<string, any>;
  props: Props;
  oldProps?: Props;
  output: Out;
};
```

## StateStore Interface

```typescript
// alchemy/src/state.ts
export interface StateStore {
  /** Initialize the state container if one is required */
  init?(): Promise<void>;

  /** Delete the state container if one exists */
  deinit?(): Promise<void>;

  /** List all resources in the given stage */
  list(): Promise<string[]>;

  /** Return the number of items in this store */
  count(): Promise<number>;

  /** Get a single resource state */
  get(key: string): Promise<State | undefined>;

  /** Get multiple resource states in batch */
  getBatch(ids: string[]): Promise<Record<string, State>>;

  /** Get all resource states */
  all(): Promise<Record<string, State>>;

  /** Set a resource state */
  set(key: string, value: State): Promise<void>;

  /** Delete a resource state */
  delete(key: string): Promise<void>;
}
```

## FileSystem State Store

### Default Implementation

```typescript
// alchemy/src/fs/file-system-state-store.ts
export class FileSystemStateStore implements StateStore {
  private readonly basePath: string;

  constructor(scope: Scope) {
    // .alchemy/{appName}/{stage}/{scopeName}/
    this.basePath = path.join(
      scope.dotAlchemy,
      scope.appName,
      scope.stage,
      ...scope.chain.slice(1)
    );
  }

  async init(): Promise<void> {
    await fs.mkdir(this.basePath, { recursive: true });
  }

  async list(): Promise<string[]> {
    try {
      const entries = await fs.readdir(this.basePath);
      return entries
        .filter((f) => f.endsWith(".json"))
        .map((f) => f.replace(".json", ""));
    } catch {
      return [];
    }
  }

  async get(key: string): Promise<State | undefined> {
    const filePath = path.join(this.basePath, `${key}.json`);
    try {
      const content = await fs.readFile(filePath, "utf8");
      return deserializeState(content);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") {
        return undefined;
      }
      throw error;
    }
  }

  async set(key: string, value: State): Promise<void> {
    const filePath = path.join(this.basePath, `${key}.json`);
    const content = serializeState(value);
    await fs.writeFile(filePath, content, "utf8");
  }

  async delete(key: string): Promise<void> {
    const filePath = path.join(this.basePath, `${key}.json`);
    await fs.unlink(filePath);
  }
}
```

### State File Format

```json
// .alchemy/my-app/dev/assets.json
{
  "kind": "aws::Bucket",
  "id": "assets",
  "fqn": "my-app/assets",
  "seq": 0,
  "status": "created",
  "data": {
    "codeHash": "abc123...",
    "bundleId": "bundle-456"
  },
  "props": {
    "versioning": true,
    "publicAccess": true,
    "tags": {
      "ManagedBy": "alchemy",
      "App": "my-app"
    }
  },
  "output": {
    "name": "my-app-assets-abc123",
    "arn": "arn:aws:s3:::my-app-assets-abc123",
    "region": "us-east-1"
  }
}
```

## S3 State Store

### Remote State for Teams

```typescript
// alchemy/src/aws/s3-state-store.ts
export class S3StateStore implements StateStore {
  private readonly bucket: string;
  private readonly prefix: string;
  private readonly region: string;
  private credentials: AwsCredentials;

  constructor(
    scope: Scope,
    options: {
      bucket: string;
      prefix?: string;
      region?: string;
    }
  ) {
    this.bucket = options.bucket;
    this.prefix = options.prefix ?? "alchemy-state";
    this.region = options.region ?? process.env.AWS_REGION ?? "us-east-1";
  }

  private key(resourceId: string): string {
    return `${this.prefix}/${scope.appName}/${scope.stage}/${resourceId}.json`;
  }

  async list(): Promise<string[]> {
    const response = await signer.request(
      "GET",
      `https://${this.bucket}.s3.${this.region}.amazonaws.com/${this.prefix}/${scope.appName}/${scope.stage}/`
    );

    const result = await parseS3ListResponse(await response.text());
    return result.keys
      .filter((k) => k.endsWith(".json"))
      .map((k) => path.basename(k, ".json"));
  }

  async get(key: string): Promise<State | undefined> {
    try {
      const response = await signer.request(
        "GET",
        `https://${this.bucket}.s3.${this.region}.amazonaws.com/${this.key(key)}`
      );

      if (response.status === 404) {
        return undefined;
      }

      const content = await response.text();
      return deserializeState(content);
    } catch (error) {
      if (error.status === 404) {
        return undefined;
      }
      throw error;
    }
  }

  async set(key: string, value: State): Promise<void> {
    const content = serializeState(value);

    await signer.request(
      "PUT",
      `https://${this.bucket}.s3.${this.region}.amazonaws.com/${this.key(key)}`,
      {
        Body: content,
        ContentType: "application/json",
      }
    );
  }

  async delete(key: string): Promise<void> {
    await signer.request(
      "DELETE",
      `https://${this.bucket}.s3.${this.region}.amazonaws.com/${this.key(key)}`
    );
  }
}
```

## R2 State Store

### Cloudflare R2 Backend

```typescript
// alchemy/src/cloudflare/r2-rest-state-store.ts
export class R2RestStateStore implements StateStore {
  private readonly bucket: string;
  private readonly api: CloudflareApi;

  constructor(
    scope: Scope,
    options: {
      bucket: string;
      api: CloudflareApi;
    }
  ) {
    this.bucket = options.bucket;
    this.api = options.api;
  }

  private key(resourceId: string): string {
    return `${scope.appName}/${scope.stage}/${resourceId}.json`;
  }

  async get(key: string): Promise<State | undefined> {
    const response = await this.api.request(
      "GET",
      `/accounts/${this.api.accountId}/r2/buckets/${this.bucket}/objects/${this.key(key)}`
    );

    if (response === null) {
      return undefined;
    }

    return deserializeState(response.body);
  }

  async set(key: string, value: State): Promise<void> {
    const content = serializeState(value);

    // R2 PUT object
    await fetch(
      `https://${this.api.accountId}.r2.cloudflarestorage.com/${this.bucket}/${this.key(key)}`,
      {
        method: "PUT",
        headers: {
          ...this.api.authHeaders,
          "Content-Type": "application/json",
        },
        body: content,
      }
    );
  }

  async delete(key: string): Promise<void> {
    await this.api.request(
      "DELETE",
      `/accounts/${this.api.accountId}/r2/buckets/${this.bucket}/objects/${this.key(key)}`
    );
  }
}
```

## State Locking

### Preventing Concurrent Modifications

```typescript
// alchemy/src/util/mutex.ts
export class AsyncMutex {
  private lock: Promise<void> = Promise.resolve();

  async lock<T>(fn: () => Promise<T>): Promise<T> {
    let release: () => void;

    // Wait for previous lock to release
    await new Promise<void>((resolve) => {
      release = resolve;
      this.lock = this.lock.then(() => new Promise<void>((r) => {
        release = r;
      }));
    });

    try {
      return await fn();
    } finally {
      release!();
    }
  }
}

// Usage in scope operations
export class Scope {
  public readonly dataMutex: AsyncMutex = new AsyncMutex();

  public async set<T>(key: string, value: T): Promise<void> {
    return this.dataMutex.lock(async () => {
      const state = await this.state.get(this.scopeName);
      state.data[key] = value;
      await this.state.set(this.scopeName, state);
    });
  }
}
```

### Distributed Locking (S3)

```typescript
// alchemy/src/aws/lock.ts
export class StateLock {
  private readonly lockKey: string;
  private readonly lockFile: string;

  constructor(bucket: string, appName: string, stage: string) {
    this.lockKey = `alchemy-locks/${appName}/${stage}/state.lock`;
    this.lockFile = `${bucket}/${this.lockKey}`;
  }

  async acquire(lockId: string): Promise<void> {
    const maxAttempts = 10;
    const retryDelay = 1000;

    for (let i = 0; i < maxAttempts; i++) {
      try {
        // Conditional PUT with If-None-Match
        await signer.request("PUT", `https://${this.lockFile}`, {
          Body: JSON.stringify({
            lockId,
            acquiredAt: new Date().toISOString(),
            hostname: os.hostname(),
          }),
          ContentType: "application/json",
        }, {
          "If-None-Match": "*",  // Only succeed if file doesn't exist
        });
        return;
      } catch (error) {
        if (error.status === 412) {  // Precondition Failed - lock exists
          const lock = await this.getLock();
          if (this.isStale(lock)) {
            await this.forceRelease();
            continue;
          }
          await sleep(retryDelay * (i + 1));
        } else {
          throw error;
        }
      }
    }

    throw new Error("Failed to acquire state lock");
  }

  async release(lockId: string): Promise<void> {
    const lock = await this.getLock();
    if (lock.lockId === lockId) {
      await signer.request("DELETE", `https://${this.lockFile}`);
    }
  }

  private async getLock(): Promise<LockInfo> {
    const response = await signer.request("GET", `https://${this.lockFile}`);
    return JSON.parse(await response.text());
  }

  private isStale(lock: LockInfo): boolean {
    const age = Date.now() - new Date(lock.acquiredAt).getTime();
    return age > 30 * 60 * 1000;  // 30 minutes
  }
}
```

## State Versioning

### Optimistic Concurrency

```typescript
// alchemy/src/state.ts - with versioning
export interface StateWithVersion extends State {
  version: number;  // Incrementing version
  updatedAt: string; // ISO timestamp
}

export class VersionedStateStore implements StateStore {
  constructor(private inner: StateStore) {}

  async set(key: string, value: State): Promise<void> {
    const existing = await this.inner.get(key);
    const version = (existing?.version ?? 0) + 1;

    await this.inner.set(key, {
      ...value,
      version,
      updatedAt: new Date().toISOString(),
    });
  }

  async setWithOptimisticLock(
    key: string,
    value: State,
    expectedVersion: number
  ): Promise<void> {
    const existing = await this.inner.get(key);

    if (existing && existing.version !== expectedVersion) {
      throw new StateVersionConflict(
        `State version conflict: expected ${expectedVersion}, got ${existing.version}`
      );
    }

    await this.set(key, value);
  }
}

export class StateVersionConflict extends Error {
  constructor(message: string) {
    super(message);
    this.name = "StateVersionConflict";
  }
}
```

### State History

```typescript
// alchemy/src/state-history.ts
export class StateHistory {
  private readonly historyPath: string;

  constructor(basePath: string) {
    this.historyPath = path.join(basePath, ".history");
  }

  async archive(key: string, state: State): Promise<void> {
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
    const historyFile = path.join(
      this.historyPath,
      `${key}.${timestamp}.json`
    );

    await fs.mkdir(path.dirname(historyFile), { recursive: true });
    await fs.writeFile(historyFile, serializeState(state), "utf8");
  }

  async getHistory(key: string): Promise<StateHistoryEntry[]> {
    const pattern = path.join(this.historyPath, `${key}.*.json`);
    const files = await glob(pattern);

    return Promise.all(
      files.map(async (file) => ({
        timestamp: path.basename(file).split(".")[1],
        state: deserializeState(await fs.readFile(file, "utf8")),
      }))
    );
  }

  async restore(key: string, timestamp: string): Promise<State> {
    const historyFile = path.join(
      this.historyPath,
      `${key}.${timestamp}.json`
    );

    const state = deserializeState(await fs.readFile(historyFile, "utf8"));
    await this.inner.set(key, state);
    return state;
  }
}
```

## State Migration

### Schema Evolution

```typescript
// alchemy/src/state-migration.ts
export function deserializeState(content: string): State {
  const data = JSON.parse(content);

  // Migration 1: Add status field (v0 -> v1)
  if (!data.status) {
    data.status = "created";
  }

  // Migration 2: Add seq field (v1 -> v2)
  if (data.seq === undefined) {
    data.seq = 0;
  }

  // Migration 3: Convert string deps to object (v2 -> v3)
  if (typeof data.deps === "string") {
    data.deps = [data.deps];
  }

  // Migration 4: Add data field for internal state (v3 -> v4)
  if (!data.data) {
    data.data = {};
  }

  // Migration 5: Convert old resource format (v4 -> v5)
  if (data.resourceType) {
    data.kind = data.resourceType;
    delete data.resourceType;
  }

  return data;
}
```

### State Import/Export

```typescript
// alchemy/src/state-migration.ts
export async function exportState(
  scope: Scope,
  outputPath: string
): Promise<void> {
  const state = await scope.state.all();

  await fs.writeFile(
    outputPath,
    JSON.stringify(
      {
        version: 1,
        exportedAt: new Date().toISOString(),
        appName: scope.appName,
        stage: scope.stage,
        resources: state,
      },
      null,
      2
    ),
    "utf8"
  );
}

export async function importState(
  scope: Scope,
  inputPath: string,
  options: { merge?: boolean; dryRun?: boolean } = {}
): Promise<void> {
  const content = await fs.readFile(inputPath, "utf8");
  const data = JSON.parse(content);

  if (options.dryRun) {
    console.log("Would import the following resources:");
    for (const [id, state] of Object.entries(data.resources)) {
      console.log(`  - ${id} (${state.kind})`);
    }
    return;
  }

  for (const [id, state] of Object.entries(data.resources)) {
    if (options.merge) {
      const existing = await scope.state.get(id);
      if (existing) {
        logger.warn(`Resource ${id} already exists, skipping`);
        continue;
      }
    }

    await scope.state.set(id, state as State);
  }

  logger.success(`Imported ${Object.keys(data.resources).length} resources`);
}
```

## Serde (Serialization/Deserialization)

### Handling Special Types

```typescript
// alchemy/src/serde.ts
export async function serialize(
  scope: Scope,
  value: any,
  options: { encrypt?: boolean } = {}
): Promise<any> {
  return transform(value, async (v) => {
    // Handle Secret
    if (v instanceof Secret) {
      if (options.encrypt && scope.password) {
        const encrypted = await encrypt(v.unencrypted, scope.password);
        return { "@secret": encrypted };
      }
      return v.unencrypted;
    }

    // Handle Date
    if (v instanceof Date) {
      return { "@date": v.toISOString() };
    }

    // Handle Symbol
    if (typeof v === "symbol") {
      const symbolName = Symbol.keyFor(v);
      if (symbolName) {
        return { "@symbol": symbolName };
      }
    }

    // Handle Scope reference
    if (isScope(v)) {
      return {
        "@scope": {
          appName: v.appName,
          stage: v.stage,
          chain: v.chain,
        },
      };
    }

    // Handle undefined (drop from serialization)
    if (v === undefined) {
      return undefined;
    }

    return v;
  });
}

export async function deserialize(
  scope: Scope,
  value: any,
  options: { decrypt?: boolean } = {}
): Promise<any> {
  return transform(value, async (v) => {
    // Handle @secret
    if (v && typeof v === "object" && "@secret" in v) {
      if (options.decrypt && scope.password) {
        const decrypted = await decrypt(v["@secret"], scope.password);
        return decrypted;
      }
      return undefined;  // Can't decrypt without password
    }

    // Handle @date
    if (v && typeof v === "object" && "@date" in v) {
      return new Date(v["@date"]);
    }

    // Handle @symbol
    if (v && typeof v === "object" && "@symbol" in v) {
      return Symbol.for(v["@symbol"]);
    }

    // Handle @scope
    if (v && typeof v === "object" && "@scope" in v) {
      // Return scope reference (resolved later)
      return v["@scope"];
    }

    // Handle @schema (ArkType)
    if (v && typeof v === "object" && "@schema" in v) {
      return parseSchema(v["@schema"]);
    }

    return v;
  });
}
```

## Replication in ewe_platform

### State Types for Valtron

```valtron
// ewe_platform/backends/foundation_core/src/state.valtron

type State<Kind, Props, Output> = {
  status: LifecycleStatus,
  kind: Kind,
  id: String,
  fqn: String,
  seq: Int,
  data: Map<String, Any>,
  props: Props,
  old_props: Props?,
  output: Output,
  version: Int,
  updated_at: DateTime
}

type StateStore = {
  init: () -> Result<Unit, StateError>,
  deinit: () -> Result<Unit, StateError>,
  list: () -> Result<List<String>, StateError>,
  get: (String) -> Result<State?, StateError>,
  get_batch: (List<String>) -> Result<Map<String, State>, StateError>,
  all: () -> Result<Map<String, State>, StateError>,
  set: (String, State) -> Result<Unit, StateError>,
  delete: (String) -> Result<Unit, StateError>,
}

// File system state store implementation
impl StateStore for FileSystemStateStore {
  fn init() {
    let path = state_path()
    fs_create_dir_all(path)
  }

  fn list() -> List<String> {
    let path = state_path()
    let entries = fs_read_dir(path)
    entries
      |> filter(|e| e.ends_with(".json"))
      |> map(|e| e.replace(".json", ""))
  }

  fn get(key: String) -> State? {
    let path = state_path() ++ "/" ++ key ++ ".json"
    match fs_read_file(path) {
      Ok(content) => deserialize_state(content),
      Err(NotFound) => None,
      Err(e) => panic("Failed to read state: " ++ e)
    }
  }

  fn set(key: String, value: State) {
    let path = state_path() ++ "/" ++ key ++ ".json"
    let content = serialize_state(value)
    fs_write_file(path, content)
  }

  fn delete(key: String) {
    let path = state_path() ++ "/" ++ key ++ ".json"
    fs_delete(path)
  }
}
```

### Remote State for Valtron

```valtron
// ewe_platform/backends/foundation_core/src/state/remote.valtron

type RemoteStateConfig = {
  provider: S3 | R2 | GCS,
  bucket: String,
  prefix: String,
  region: String?,
}

type RemoteStateStore = {
  config: RemoteStateConfig,
  lock: StateLock?,
}

impl StateStore for RemoteStateStore {
  fn get(key: String) -> State? {
    let object_key = config.prefix ++ "/" ++ key ++ ".json"
    match remote_get(config.bucket, object_key) {
      Ok(content) => deserialize_state(content),
      Err(NotFound) => None,
      Err(e) => panic("Failed to read remote state: " ++ e)
    }
  }

  fn set(key: String, value: State) {
    // Acquire lock
    let lock_id = generate_lock_id()
    lock_acquire(lock_id)

    // Write with version bump
    let existing = get(key)
    let value_with_version = {
      ...value,
      version: (existing?.version ?? 0) + 1,
      updated_at: now()
    }

    let object_key = config.prefix ++ "/" ++ key ++ ".json"
    remote_put(config.bucket, object_key, serialize_state(value_with_version))

    // Release lock
    lock_release(lock_id)
  }
}

type StateLock = {
  bucket: String,
  key: String,
  lock_id: String?,
}

operation lock_acquire(lock_id: String) {
  let max_attempts = 10
  let mut attempt = 0

  while attempt < max_attempts {
    match remote_put_if_not_exists(
      config.bucket,
      config.prefix ++ "/state.lock",
      json_encode({
        lock_id: lock_id,
        acquired_at: now(),
        hostname: hostname()
      })
    ) {
      Ok(()) => return,  // Lock acquired
      Err(PreconditionFailed) => {
        // Lock exists - check if stale
        let lock = remote_get(config.bucket, config.prefix ++ "/state.lock")
        if is_stale(lock) {
          remote_delete(config.bucket, config.prefix ++ "/state.lock")
          continue
        }
        sleep(1000 * (attempt + 1))
        attempt = attempt + 1
      }
      Err(e) => panic("Failed to acquire lock: " ++ e)
    }
  }

  panic("Failed to acquire state lock after " ++ max_attempts ++ " attempts")
}
```

## Best Practices

### 1. State Isolation

```typescript
// Use different state files for different stages
const dev = await alchemy("my-app", { stage: "dev" });
const prod = await alchemy("my-app", { stage: "production" });
```

### 2. State Encryption

```typescript
// Always encrypt secrets
const app = await alchemy("my-app", {
  password: process.env.SECRET_PASSPHRASE,
});

const worker = await Worker("api", {
  bindings: {
    API_KEY: alchemy.secret(process.env.API_KEY),  // Encrypted in state
  },
});
```

### 3. State Backup

```typescript
// Backup state before major operations
await exportState(scope, `.alchemy/backup-${Date.now()}.json`);
```

### 4. State Locking for CI/CD

```typescript
// Always use locking in CI/CD pipelines
const app = await alchemy("my-app", {
  stateStore: {
    type: "s3",
    bucket: "my-terraform-state",
    lock: true,
  },
});
```

### 5. State Migration Testing

```typescript
// Test state migrations
describe("State Migration", () => {
  it("migrates v1 state to v5", async () => {
    const v1State = {
      kind: "aws::Bucket",
      id: "assets",
      resourceType: "aws::Bucket",
      deps: "other-resource",
    };

    const migrated = deserializeState(JSON.stringify(v1State));

    expect(migrated.status).toBe("created");
    expect(migrated.seq).toBe(0);
    expect(migrated.deps).toEqual(["other-resource"]);
    expect(migrated.kind).toBe("aws::Bucket");
  });
});
```

## Summary

State management in Alchemy:

1. **StateStore Interface** - Pluggable storage backends
2. **FileSystem** - Default local state storage
3. **S3/R2** - Remote state for teams
4. **Locking** - Prevent concurrent modifications
5. **Versioning** - Optimistic concurrency control
6. **History** - State change tracking
7. **Migration** - Schema evolution support
8. **Serde** - Handle special types (Secret, Date, Symbol)

For `ewe_platform`, implement:
- State types in Valtron
- FileSystemStateStore implementation
- Remote state with S3/R2 support
- Locking mechanism
- Serde for special types

## Next Steps

- [rust-revision.md](./rust-revision.md) - Rust translation for ewe_platform
- [production-grade.md](./production-grade.md) - Multi-tenant deployment, scaling
