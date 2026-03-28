---
title: "Alchemy Core Architecture Deep Dive"
subtitle: "Complete technical analysis of Alchemy's resource lifecycle, scope system, and state management"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/alchemy
explored_at: 2026-03-27
---

# 03 - Core Architecture Deep Dive

## Overview

This document provides a complete technical deep-dive into Alchemy's core architecture:
- **Resource System**: How resources are defined, registered, and executed
- **Scope System**: Hierarchical execution context using AsyncLocalStorage
- **Apply Engine**: The create/update lifecycle engine
- **State Management**: FileSystemStateStore, S3StateStore, and custom stores
- **Provider Implementations**: Cloudflare, AWS, GCP patterns

## 1. The Resource System

### 1.1 Resource Factory Function

**Location:** `alchemy/alchemy/src/resource.ts`

The `Resource()` factory is the core abstraction that transforms async functions into managed infrastructure resources:

```typescript
export function Resource<
  const Type extends ResourceKind,
  F extends ResourceLifecycleHandler,
>(type: Type, ...args: [Partial<ProviderOptions>, F] | [F]): Handler<F> {
  const [options, handler] = args.length === 2 ? args : [undefined, args[0]];

  // Register handler in global map
  HANDLERS.set(type, handler);

  const provider = (async (
    resourceID: string,
    props: ResourceProps,
  ): Promise<ResourceAttributes> => {
    const scope = _Scope.current;

    // Get sequence number for ordering
    const seq = scope.seq();

    // Create metadata with well-known Symbols
    const meta = {
      [ResourceKind]: type,
      [ResourceID]: resourceID,
      [ResourceFQN]: scope.fqn(resourceID),
      [ResourceSeq]: seq,
      [ResourceScope]: scope,
      [DestroyStrategy]: options?.destroyStrategy ?? "sequential",
    } as any as PendingResource<Out>;

    // Call apply() to execute lifecycle
    const promise = apply(meta, props, options);
    const resource = Object.assign(promise, meta);

    // Register in scope's resource map
    scope.resources.set(resourceID, resource);
    return resource;
  }) as Provider<Type, F>;

  provider.type = type;
  provider.handler = handler;
  provider.options = options;
  PROVIDERS.set(type, provider);
  return provider;
}
```

### 1.2 Symbol-Keyed Metadata

Alchemy uses `Symbol.for()` keys to store metadata on resource objects, avoiding namespace collision with user properties:

```typescript
// Well-known Symbols for resource metadata
export const ResourceID = Symbol.for("alchemy::ResourceID");
export const ResourceFQN = Symbol.for("alchemy::ResourceFQN");
export const ResourceKind = Symbol.for("alchemy::ResourceKind");
export const ResourceScope = Symbol.for("alchemy::ResourceScope");
export const ResourceSeq = Symbol.for("alchemy::ResourceSeq");

// Interface for PendingResource
export interface PendingResource<Out = unknown> extends Promise<Out> {
  [ResourceKind]: ResourceKind;
  [ResourceID]: ResourceID;
  [ResourceFQN]: ResourceFQN;
  [ResourceScope]: Scope;
  [ResourceSeq]: number;
  [DestroyStrategy]: DestroyStrategy;
}
```

**Why Symbols?**
- Properties don't appear in `JSON.stringify()` by default (handled by serde)
- No collision with user-defined properties on output objects
- Global symbol registry (`Symbol.for()`) allows access across module boundaries

### 1.3 Provider Registry

Providers are registered in global maps for lookup during apply:

```typescript
// Global provider registry
export const PROVIDERS: Map<ResourceKind, Provider<string, any>> =
  (globalThis.ALCHEMY_PROVIDERS ??= new Map());

// Handler registry for duplicate detection
const HANDLERS: Map<ResourceKind, ResourceLifecycleHandler> =
  (globalThis.ALCHEMY_HANDLERS ??= new Map());

// Dynamic resource resolver for deletion (handles missing providers)
const DYNAMIC_RESOURCE_RESOLVERS: DynamicResourceResolver[] =
  (globalThis.ALCHEMY_DYNAMIC_RESOURCE_RESOLVERS ??= []);

export function resolveDeletionHandler(typeName: string): Provider | undefined {
  const provider = PROVIDERS.get(typeName);
  if (provider) return provider;

  // Fall back to dynamic resolvers
  for (const handler of DYNAMIC_RESOURCE_RESOLVERS) {
    const result = handler(typeName);
    if (result) return result;
  }
  return undefined;
}
```

## 2. The Apply Engine

**Location:** `alchemy/alchemy/src/apply.ts`

The `apply()` function implements the core create/update lifecycle logic.

### 2.1 Apply Function Flow

```typescript
async function _apply<Out extends ResourceAttributes>(
  resource: PendingResource<Out>,
  props: ResourceProps | undefined,
  options?: ApplyOptions,
): Promise<Awaited<Out> & Resource> {
  const scope = resource[ResourceScope];
  const start = performance.now();

  await scope.init();

  // Load previous state
  let state = await scope.state.get(resource[ResourceID]);

  // Get provider from registry
  const provider: Provider = PROVIDERS.get(resource[ResourceKind]);
  if (provider === undefined) {
    throw new Error(`Provider "${resource[ResourceKind]}" not found`);
  }

  // Handle "read" phase (for cross-scope references)
  if (scope.phase === "read") {
    // Wait for state to become available from owner scope
    state = await waitForConsistentState();
    return state.output as Awaited<Out> & Resource;
  }

  // Initialize state if this is a new resource
  if (state === undefined) {
    state = {
      kind: resource[ResourceKind],
      id: resource[ResourceID],
      fqn: resource[ResourceFQN],
      seq: resource[ResourceSeq],
      status: "creating",
      data: {},
      output: { /* resource metadata */ },
      props: {},
    };
    await scope.state.set(resource[ResourceID], state);
  }

  const oldOutput = state.output;

  // Skip update if props haven't changed (memoization)
  if (state.status === "created" || state.status === "updated") {
    const oldProps = await serialize(scope, state.props, { encrypt: false });
    const newProps = await serialize(scope, props, { encrypt: false });

    if (JSON.stringify(oldProps) === JSON.stringify(newProps)) {
      // Resource is memoized - return cached output
      return state.output as Awaited<Out> & Resource;
    }
  }

  // Determine lifecycle phase
  const phase = state.status === "creating" ? "create" : "update";
  state.status = phase === "create" ? "creating" : "updating";
  state.oldProps = state.props;
  state.props = props;

  await scope.state.set(resource[ResourceID], state);

  // Create execution context
  const ctx = context({
    scope,
    phase,
    kind: resource[ResourceKind],
    id: resource[ResourceID],
    fqn: resource[ResourceFQN],
    seq: resource[ResourceSeq],
    props: state.oldProps,
    state,
    isReplacement: false,
    replace: (force?: boolean) => {
      throw new ReplacedSignal(force);
    },
  });

  // Execute provider handler
  let output: any;
  try {
    output = await alchemy.run(
      resource[ResourceID],
      { isResource: true, parent: scope },
      async () => ctx(await provider.handler.bind(ctx)(resource[ResourceID], props)),
    );
  } catch (error) {
    if (error instanceof ReplacedSignal) {
      // Handle resource replacement (delete + recreate)
      output = await handleReplacement();
    } else {
      throw error;
    }
  }

  // Persist final state
  await scope.state.set(resource[ResourceID], {
    kind: resource[ResourceKind],
    id: resource[ResourceID],
    fqn: resource[ResourceFQN],
    seq: resource[ResourceSeq],
    data: state?.data ?? {},
    status: phase === "create" ? "created" : "updated",
    output,
    props,
  });

  return output;
}
```

### 2.2 State Transitions

```
creating → created     (create success)
creating → updating    (update during create phase)
updating → updated     (update success)
created → updating     (props changed)
any → deleting         (destroy initiated)
deleting → deleted     (destroy success)
```

### 2.3 Replacement Signal

Resources can signal they need to be replaced (deleted + recreated) instead of updated:

```typescript
export class ReplacedSignal extends Error {
  readonly kind = "ReplacedSignal";
  public force: boolean;

  constructor(force?: boolean) {
    super();
    this.force = force ?? false;
  }
}

// In provider handler
export const Worker = Resource(
  "cloudflare::Worker",
  async function (this: Context<Worker>, id: string, props: WorkerProps) {
    // Some changes require replacement (e.g., changing compatibility_date)
    if (this.phase === "update" && props.compatibility_date !== this.props.compatibility_date) {
      this.replace(); // Throws ReplacedSignal
    }

    // Normal update logic...
  }
);
```

## 3. The Scope System

**Location:** `alchemy/alchemy/src/scope.ts`

### 3.1 AsyncLocalStorage Context

Scopes use Node.js `AsyncLocalStorage` for implicit context propagation:

```typescript
export class Scope {
  public static storage = (globalThis.__ALCHEMY_STORAGE__ ??=
    new AsyncLocalStorage<Scope>());

  public static getScope(): Scope | undefined {
    return Scope.storage.getStore();
  }

  public static get current(): Scope {
    const scope = Scope.getScope();
    if (!scope) throw new Error("Not running within an Alchemy Scope");
    return scope;
  }

  public async run<T>(fn: (scope: Scope) => Promise<T>): Promise<T> {
    return Scope.storage.run(this, () => fn(this));
  }
}
```

### 3.2 Scope Hierarchy

```typescript
// Scope chain: app/stage/resource
// Example: my-app/prod/api-worker

export class Scope {
  public readonly parent: Scope | undefined;
  public readonly stage: string;
  public readonly name: string;
  public readonly scopeName: string;

  // Resources created in this scope
  public readonly resources = new Map<ResourceID, PendingResource>();

  // Child scopes (nested alchemy.run() calls)
  public readonly children: Map<ResourceID, Scope> = new Map();

  public get chain(): string[] {
    if (!this.parent && this.appName === this.scopeName) {
      return [this.appName];
    }
    const thisScope = this.scopeName ? [this.scopeName] : [];
    if (this.parent) {
      return [...this.parent.chain, ...thisScope];
    }
    const app = this.appName ? [this.appName] : [];
    return [...app, ...thisScope];
  }

  public fqn(resourceID: ResourceID): string {
    return [...this.chain, resourceID].join("/");
  }
}
```

### 3.3 Scope Options

```typescript
export interface ScopeOptions extends ProviderCredentials {
  stage?: string;                    // Environment name (dev, prod)
  parent: Scope | undefined | null;  // Parent scope
  scopeName: string;                 // This scope's name
  password?: string;                 // For encrypting secrets
  stateStore?: StateStoreType;       // Custom state store
  quiet?: boolean;                   // Suppress logging
  phase?: Phase;                     // "up" | "destroy" | "read"
  local?: boolean;                   // Local emulation mode
  watch?: boolean;                   // Reactive updates
  tunnel?: boolean;                  // Create tunnels for resources
  force?: boolean;                   // Force updates
  adopt?: boolean;                   // Adopt existing resources
  destroyStrategy?: DestroyStrategy; // "sequential" | "parallel"
  eraseSecrets?: boolean;            // Skip secret decryption
  rootDir?: string;                  // Project root
  profile?: string;                  // Alchemy auth profile
  isSelected?: boolean;              // Selected via --app flag
}
```

### 3.4 Scope Finalization (Orphan Cleanup)

When a scope finalizes, it deletes orphaned resources (in state but not in code):

```typescript
public async finalize(options?: { force?: boolean; noop?: boolean }) {
  if (this.phase === "read" || this.isErrored || this.isSkipped) {
    return; // Skip finalization
  }

  // Get all resources from state
  const resourceIds = await this.state.list();

  // Get all resources created in memory
  const aliveIds = new Set(this.resources.keys());

  // Find orphans
  const orphanIds = Array.from(
    resourceIds.filter((id) => !aliveIds.has(id)),
  );

  // Destroy orphans
  const orphans = await Promise.all(
    orphanIds.map(async (id) => (await this.state.get(id))!.output),
  );

  await destroyAll(orphans, {
    quiet: this.quiet,
    strategy: this.destroyStrategy,
    force: true,
    noop: options?.noop,
  });
}
```

### 3.5 Provider Credentials in Scope

Scopes can hold provider credentials that child resources inherit:

```typescript
// In scope.ts
export interface ProviderCredentials extends Record<string, unknown> {
  // Extended via module augmentation by providers
}

export class Scope {
  public readonly providerCredentials: ProviderCredentials;

  constructor(options: ScopeOptions) {
    const {
      scopeName, parent, stage, /* core options */,
      ...providerCredentials  // Extracted via rest
    } = options;

    this.providerCredentials = providerCredentials as ProviderCredentials;
  }
}

// Provider extension pattern (aws/scope-extensions.ts)
declare module "../scope.ts" {
  interface ProviderCredentials {
    aws?: AwsClientProps;
  }
}

// Usage
await alchemy("my-app", {
  aws: {
    region: "us-west-2",
    profile: "production"
  }
}, async () => {
  // Resources inherit AWS credentials from scope
  const vpc = await Vpc("main-vpc", { cidrBlock: "10.0.0.0/16" });

  // Resource can override scope credentials
  const crossRegionSubnet = await Subnet("cross-region", {
    vpc,
    region: "us-east-1"  // Override
  });
});
```

## 4. State Management

**Location:** `alchemy/alchemy/src/state.ts`, `alchemy/alchemy/src/state/file-system-state-store.ts`

### 4.1 StateStore Interface

```typescript
export interface StateStore {
  init?(): Promise<void>;
  deinit?(): Promise<void>;
  list(): Promise<string[]>;
  count(): Promise<number>;
  get(key: string): Promise<State | undefined>;
  getBatch(ids: string[]): Promise<Record<string, State>>;
  all(): Promise<Record<string, State>>;
  set(key: string, value: State): Promise<void>;
  delete(key: string): Promise<void>;
}

export type State<
  Kind extends string = string,
  Props extends ResourceProps | undefined = ResourceProps | undefined,
  Out extends Resource = Resource,
> = {
  status: "creating" | "created" | "updating" | "updated" | "deleting" | "deleted";
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

### 4.2 FileSystemStateStore

**Location:** `alchemy/alchemy/src/state/file-system-state-store.ts`

Default state store that persists to JSON files:

```typescript
export class FileSystemStateStore implements StateStore {
  public readonly dir: string;
  private initialized = false;

  constructor(
    public readonly scope: Scope,
    options?: { rootDir?: string }
  ) {
    // Directory structure: .alchemy/app/stage/
    this.dir = path.join(
      options?.rootDir ?? scope.dotAlchemy,
      ...scope.chain
    );
  }

  async init(): Promise<void> {
    if (this.initialized) return;
    this.initialized = true;
    await fs.promises.mkdir(this.dir, { recursive: true });
  }

  async list(): Promise<string[]> {
    try {
      const files = await fs.promises.readdir(this.dir, { withFileTypes: true });
      return files
        .filter((dirent) => dirent.isFile() && dirent.name.endsWith(".json"))
        .map((dirent) => dirent.name.replace(/\.json$/, ""))
        .map((key) => key.replaceAll(":", "/"));
    } catch (error: any) {
      if (error.code === "ENOENT") return [];
      throw error;
    }
  }

  async get(key: string): Promise<State | undefined> {
    try {
      const content = await fs.promises.readFile(this.getPath(key), "utf8");
      const state = await deserialize(this.scope, JSON.parse(content));
      state.output[ResourceScope] = this.scope;
      return state;
    } catch (error: any) {
      if (error.code === "ENOENT") return undefined;
      throw error;
    }
  }

  async set(key: string, value: State): Promise<void> {
    await this.init();
    const file = this.getPath(key);
    await fs.promises.mkdir(path.dirname(file), { recursive: true });
    await fs.promises.writeFile(
      file,
      JSON.stringify(await serialize(this.scope, value), null, 2)
    );
  }

  private getPath(key: string): string {
    // Windows compatibility: use - instead of :
    const separator = process.platform === "win32" ? "-" : ":";
    if (key.includes("/")) {
      key = key.replaceAll("/", separator);
    }
    return path.join(this.dir, `${key}.json`);
  }
}
```

### 4.3 S3StateStore

**Location:** `alchemy/alchemy/src/aws/s3-state-store.ts`

For CI/CD and team environments:

```typescript
export class S3StateStore implements StateStore {
  private client: S3Client;
  private prefix: string;
  private bucketName: string;
  private initialized = false;

  constructor(
    public readonly scope: Scope,
    options: S3StateStoreOptions = {}
  ) {
    // Prefix: alchemy/app/stage/
    const scopePath = scope.chain.join("/");
    this.prefix = options.prefix
      ? `${options.prefix}${scopePath}/`
      : `alchemy/${scopePath}/`;

    this.bucketName = options.bucketName ?? "alchemy-state";
    this.client = new S3Client({ region: options.region });
  }

  async get(key: string): Promise<State | undefined> {
    try {
      const response = await this.client.send(
        new GetObjectCommand({
          Bucket: this.bucketName,
          Key: this.getObjectKey(key),
        })
      );

      const content = await response.Body.transformToString();
      const state = await deserialize(this.scope, JSON.parse(content));

      return {
        ...state,
        output: {
          ...(state.output || {}),
          [ResourceScope]: this.scope,
        },
      };
    } catch (error: any) {
      if (error.name === NoSuchKey.name) return undefined;
      throw error;
    }
  }

  async set(key: string, value: State): Promise<void> {
    const objectKey = this.getObjectKey(key);
    const serializedData = JSON.stringify(
      await serialize(this.scope, value),
      null, 2
    );

    await this.client.send(
      new PutObjectCommand({
        Bucket: this.bucketName,
        Key: objectKey,
        Body: serializedData,
        ContentType: "application/json",
      })
    );
  }

  private getObjectKey(key: string): string {
    // S3 uses / as directory separator, but we need to avoid conflicts
    // Replace / with : for storage, reverse on retrieval
    return `${this.prefix}${key.replaceAll("/", ":")}`;
  }
}
```

### 4.4 State Serialization (serde)

**Location:** `alchemy/alchemy/src/serde.ts`

Handles circular references, Secrets, and Symbols:

```typescript
export async function serialize(
  scope: Scope,
  value: any,
  options: { encrypt?: boolean } = {}
): Promise<any> {
  const seen = new Map();

  function visit(key: string, value: any): any {
    // Handle Secret objects
    if (isSecret(value)) {
      return {
        [SERIALIZED_SYMBOL]: true,
        type: "Secret",
        encrypted: options.encrypt
          ? await encrypt(scope.password!, value.unencrypted)
          : value.unencrypted,
      };
    }

    // Handle Symbols (strip them - they can't be serialized)
    if (typeof value === "symbol") {
      return undefined;
    }

    // Handle circular references
    if (typeof value === "object" && value !== null) {
      if (seen.has(value)) {
        return { [SERIALIZED_SYMBOL]: true, type: "CircularRef" };
      }
      seen.set(value, true);
    }

    // Handle arrays
    if (Array.isArray(value)) {
      return value.map((v, i) => visit(i.toString(), v));
    }

    // Handle objects
    if (typeof value === "object") {
      const result: any = {};
      for (const [k, v] of Object.entries(value)) {
        if (!k.startsWith("__")) {  // Skip internal properties
          result[k] = visit(k, v);
        }
      }
      return result;
    }

    return value;
  }

  return visit("root", value);
}
```

## 5. Provider Implementations

### 5.1 Cloudflare Provider

**Location:** `alchemy/alchemy/src/cloudflare/`

#### API Client with Auto-Discovery

```typescript
// alchemy/alchemy/src/cloudflare/api.ts

export interface CloudflareApiOptions {
  baseUrl?: string;
  profile?: string;
  apiKey?: Secret;
  apiToken?: Secret;
  accountId?: string;
  email?: string;
}

export const createCloudflareApi = memoize(
  async (options: CloudflareApiOptions = {}) => {
    const apiKey = options.apiKey?.unencrypted ?? process.env.CLOUDFLARE_API_KEY;
    const apiToken = options.apiToken?.unencrypted ?? process.env.CLOUDFLARE_API_TOKEN;
    const accountId = options.accountId ?? process.env.CLOUDFLARE_ACCOUNT_ID;

    if (apiKey) {
      const credentials: Credentials.ApiKey = {
        type: "api-key",
        apiKey,
        email: options.email ?? (await getUserEmailFromApiKey(apiKey)),
      };
      return new CloudflareApi({
        credentials,
        accountId: accountId ?? (await getCloudflareAccountId(credentials)),
      });
    }

    if (apiToken) {
      const credentials: Credentials.ApiToken = {
        type: "api-token",
        apiToken,
      };
      return new CloudflareApi({
        credentials,
        accountId: accountId ?? (await getCloudflareAccountId(credentials)),
      });
    }

    // Fall back to Alchemy profile authentication
    const profile = options.profile ?? process.env.CLOUDFLARE_PROFILE ?? "default";
    const { credentials } = await Provider.getWithCredentials({
      provider: "cloudflare",
      profile,
    });

    return new CloudflareApi({
      profile,
      credentials,
      accountId: accountId ?? provider.metadata.id,
    });
  }
);

export class CloudflareApi {
  readonly baseUrl: string = "https://api.cloudflare.com/client/v4";
  readonly accountId: string;
  readonly credentials: Credentials;

  async fetch(path: string, init: RequestInit = {}): Promise<Response> {
    const headers = {
      "Content-Type": "application/json",
      ...(await CloudflareAuth.formatHeadersWithRefresh({
        profile: this.profile,
        credentials: this.credentials,
      })),
    };

    // Exponential backoff for network errors
    return withExponentialBackoff(
      async () => {
        const response = await safeFetch(`${this.baseUrl}${path}`, {
          ...init,
          headers,
        });

        // Retry on 5xx errors
        if (response.status.toString().startsWith("5")) {
          throw new InternalError(response.statusText);
        }

        // Retry once on 403 (occasional transient)
        if (response.status === 403) {
          throw new ForbiddenError();
        }

        // Retry on rate limits
        if (response.status === 429) {
          const data = await response.json();
          throw new TooManyRequestsError(data.errors[0].message);
        }

        return response;
      },
      (error) => error instanceof InternalError || error instanceof TooManyRequestsError,
      10,  // max attempts
      1000 // initial delay ms
    );
  }
}
```

#### Worker Resource with Bindings

```typescript
// alchemy/alchemy/src/cloudflare/worker.ts

export interface WorkerProps extends CloudflareApiOptions {
  name?: string;
  entrypoint: string;
  bindings?: Bindings;
  compatibilityDate?: string;
  compatibilityFlags?: string[];
  url?: boolean;
  routes?: string[];
  crons?: string[];
  eventSources?: EventSource[];
}

export const Worker = Resource(
  "cloudflare::Worker",
  async function (this: Context<Worker>, id: string, props: WorkerProps): Promise<Worker> {
    const api = await createCloudflareApi(props);
    const name = props.name ?? this.scope.createPhysicalName(id);

    if (this.phase === "create") {
      // Bundle worker code
      const bundle = await bundleWorker({ entrypoint: props.entrypoint });

      // Prepare bindings
      const bindings = await prepareBindings(props.bindings, api);

      // Upload worker script with bindings
      const formData = new FormData();
      formData.append("main", bundle.code);
      formData.append("metadata", JSON.stringify({
        main_module: "index.js",
        bindings,
        compatibility_date: props.compatibilityDate,
        compatibility_flags: props.compatibilityFlags,
      }));

      await api.request(
        "PUT",
        `/accounts/${api.accountId}/workers/scripts/${name}`,
        formData
      );

      // Enable subdomain if requested
      if (props.url) {
        await enableWorkerSubdomain(api, name);
      }

      return this.create({
        id: name,
        name,
        url: `https://${name}.workers.dev`,
        ...props,
      });

    } else if (this.phase === "update") {
      // Compare bundle hash to determine if update needed
      const newBundle = await bundleWorker({ entrypoint: props.entrypoint });
      const oldBundle = this.output.scriptHash;

      if (oldBundle !== newBundle.hash || propsChanged(this.props, props)) {
        // Update worker with new code/bindings
        const formData = new FormData();
        formData.append("main", newBundle.code);
        formData.append("metadata", JSON.stringify({
          main_module: "index.js",
          bindings: await prepareBindings(props.bindings, api),
        }));

        await api.request(
          "PUT",
          `/accounts/${api.accountId}/workers/scripts/${name}`,
          formData
        );
      }

      return this.create({
        id: name,
        name,
        url: `https://${name}.workers.dev`,
        ...props,
        scriptHash: newBundle.hash,
      });
    }
  }
);
```

#### D1 Database Resource

```typescript
// alchemy/alchemy/src/cloudflare/d1-database.ts

export interface D1DatabaseProps extends CloudflareApiOptions {
  name?: string;
  primaryLocationHint?: PrimaryLocationHint;
  readReplication?: { mode: "auto" | "disabled" };
  migrationsDir?: string;
  migrationsTable?: string;
  importFiles?: string[];
  adopt?: boolean;
  clone?: D1Database | { id: string } | { name: string };
}

export const D1Database = Resource(
  "cloudflare::D1Database",
  async function (this: Context<D1Database>, id: string, props: D1DatabaseProps): Promise<D1Database> {
    const api = await createCloudflareApi(props);
    const name = props.name ?? this.scope.createPhysicalName(id);

    if (this.phase === "create") {
      // Check if database with same name exists (adopt)
      if (props.adopt) {
        const existing = await findDatabaseByName(api, name);
        if (existing) {
          return this.create({ id: existing.uuid, name, ...props });
        }
      }

      // Create new database
      const response = await api.post(
        `/accounts/${api.accountId}/d1/database`,
        {
          name,
          primary_location_hint: props.primaryLocationHint,
        }
      );

      const database = await response.json();

      // Apply migrations if specified
      if (props.migrationsDir) {
        await applyMigrations(api, database.result.uuid, {
          dir: props.migrationsDir,
          table: props.migrationsTable ?? DEFAULT_MIGRATIONS_TABLE,
        });
      }

      // Import SQL files if specified
      if (props.importFiles) {
        await importD1Database(api, database.result.uuid, props.importFiles);
      }

      return this.create({
        id: database.result.uuid,
        name,
        ...props,
      });

    } else if (this.phase === "update") {
      // Only readReplication mode is mutable
      if (props.readReplication?.mode !== this.props.readReplication?.mode) {
        await api.patch(
          `/accounts/${api.accountId}/d1/database/${this.output.id}`,
          {
            read_replication: props.readReplication,
          }
        );
      }

      // Apply new migrations if migrationsDir changed
      if (props.migrationsDir !== this.props.migrationsDir) {
        await applyMigrations(api, this.output.id, {
          dir: props.migrationsDir!,
          table: props.migrationsTable ?? DEFAULT_MIGRATIONS_TABLE,
        });
      }

      return this.create({ id: this.output.id, name, ...props });
    }
  }
);
```

### 5.2 AWS Provider

**Location:** `alchemy/alchemy/src/aws/`

#### Credential Resolution

```typescript
// alchemy/alchemy/src/aws/credentials.ts

export interface AwsClientProps {
  region?: string;
  profile?: string;
  accessKeyId?: Secret | string;
  secretAccessKey?: Secret | string;
  sessionToken?: Secret | string;
  roleArn?: string;
  externalId?: string;
  roleSessionName?: string;
}

export async function resolveAwsCredentials(
  resourceProps?: AwsClientProps,
): Promise<AwsClientProps> {
  // 1. Global environment variables (lowest priority)
  const globalConfig = {
    accessKeyId: process.env.AWS_ACCESS_KEY_ID
      ? alchemy.secret(process.env.AWS_ACCESS_KEY_ID)
      : undefined,
    secretAccessKey: process.env.AWS_SECRET_ACCESS_KEY
      ? alchemy.secret(process.env.AWS_SECRET_ACCESS_KEY)
      : undefined,
    region: process.env.AWS_REGION || process.env.AWS_DEFAULT_REGION,
    profile: process.env.AWS_PROFILE,
    roleArn: process.env.AWS_ROLE_ARN,
  };

  // 2. Scope-level credentials (medium priority)
  let scopeConfig: AwsClientProps = {};
  try {
    const currentScope = Scope.getScope();
    if (currentScope?.providerCredentials?.aws) {
      scopeConfig = currentScope.providerCredentials.aws;
      validateAwsClientProps(scopeConfig, "scope");
    }
  } catch (error) {
    // Not in scope context, continue
  }

  // 3. Resource-level credentials (highest priority)
  const resourceConfig = resourceProps || {};
  if (resourceProps) {
    validateAwsClientProps(resourceProps, "resource properties");
  }

  // Merge with precedence (later overrides earlier)
  return {
    ...globalConfig,
    ...scopeConfig,
    ...resourceConfig,
  };
}
```

#### Lambda Function Resource

```typescript
// alchemy/alchemy/src/aws/function.ts

export interface FunctionProps {
  functionName?: string;
  bundle: Bundle;
  roleArn: string;
  handler: string;
  runtime?: Runtime;  // e.g., "nodejs20.x"
  architecture?: Architecture;  // "x86_64" | "arm64"
  timeout?: number;
  memorySize?: number;
  environment?: Record<string, string>;
  layers?: string[];
  url?: {
    invokeMode?: "BUFFERED" | "RESPONSE_STREAM";
    authType?: "AWS_IAM" | "NONE";
    cors?: CORSConfig;
  };
}

export const Function = Resource(
  "aws::Function",
  async function (this: Context<Function>, id: string, props: FunctionProps): Promise<Function> {
    const credentials = await resolveAwsCredentials();
    const client = new LambdaClient({ region: credentials.region });
    const functionName = props.functionName ?? this.scope.createPhysicalName(id);

    if (this.phase === "create") {
      // Create Lambda function
      const command = new CreateFunctionCommand({
        FunctionName: functionName,
        Runtime: props.runtime ?? "nodejs20.x",
        Handler: props.handler,
        Role: props.roleArn,
        Code: {
          ZipFile: props.bundle.code,
        },
        Timeout: props.timeout ?? 3,
        MemorySize: props.memorySize ?? 128,
        Environment: props.environment
          ? { Variables: props.environment }
          : undefined,
        Tags: props.tags,
        Architectures: props.architecture ? [props.architecture] : ["x86_64"],
        Layers: props.layers,
      });

      const result = await client.send(command);

      // Create function URL if requested
      let functionUrl: string | undefined;
      if (props.url) {
        const urlCommand = new CreateFunctionUrlConfigCommand({
          FunctionName: functionName,
          AuthType: props.url.authType ?? "AWS_IAM",
          InvokeMode: props.url.invokeMode,
          Cors: props.url.cors,
        });
        const urlResult = await client.send(urlCommand);
        functionUrl = urlResult.FunctionUrl;
      }

      return this.create({
        ...props,
        functionName,
        arn: result.FunctionArn!,
        qualifiedArn: `${result.FunctionArn!}:${result.Version!}`,
        invokeArn: `arn:aws:apigateway:${credentials.region}:lambda:path/2015-03-31/functions/${result.FunctionArn!}/invocations`,
        version: result.Version!,
        lastModified: result.LastModified!,
        sourceCodeHash: result.CodeSha256!,
        sourceCodeSize: result.CodeSize!,
        architectures: result.Architectures!,
        revisionId: result.RevisionId!,
        url: functionUrl,
      });

    } else if (this.phase === "update") {
      // Update function configuration
      const updateCommand = new UpdateFunctionConfigurationCommand({
        FunctionName: functionName,
        Runtime: props.runtime,
        Handler: props.handler,
        Role: props.roleArn,
        Timeout: props.timeout,
        MemorySize: props.memorySize,
        Environment: props.environment
          ? { Variables: props.environment }
          : undefined,
        Layers: props.layers,
      });
      await client.send(updateCommand);

      // Update code if bundle changed
      const oldBundle = this.output.sourceCodeHash;
      const newBundle = props.bundle.code;
      if (oldBundle !== newBundle) {
        const codeCommand = new UpdateFunctionCodeCommand({
          FunctionName: functionName,
          ZipFile: newBundle,
          Publish: true,
        });
        const result = await client.send(codeCommand);

        return this.create({
          ...props,
          functionName,
          arn: result.FunctionArn!,
          version: result.Version!,
          sourceCodeHash: result.CodeSha256!,
          sourceCodeSize: result.CodeSize!,
        });
      }

      return this.create({ ...props, functionName, arn: this.output.arn });
    }
  }
);
```

### 5.3 GCP Provider

GCP uses Discovery Document parsing for API generation.

## 6. alchemy-web Documentation System

**Location:** `alchemy/alchemy-web/`

### 6.1 Structure

```
alchemy/alchemy-web/
├── src/content/docs/
│   ├── index.md                    # Landing page
│   ├── concepts/
│   │   ├── scope.md                # Scope system documentation
│   │   ├── bindings.md             # Binding system documentation
│   │   ├── resource.md             # Resource system documentation
│   │   └── state.md                # State management documentation
│   ├── guides/
│   │   ├── custom-resources.md     # AI-assisted resource generation
│   │   └── prisma-postgres.md      # Integration guides
│   ├── providers/
│   │   ├── cloudflare/             # Cloudflare provider docs
│   │   └── aws-control/            # AWS Control Tower docs
│   └── advanced/
│       └── serde.md                # Serialization details
└── src/content/config.ts           # Starlight config
```

### 6.2 alchemy() Entry Point

**Location:** `alchemy/alchemy/src/alchemy.ts`

```typescript
export interface Alchemy {
  run: typeof run;
  destroy: typeof destroy;
  env: typeof env;
  secret: typeof secret;

  (appName: string, options?: Omit<AlchemyOptions, "appName">): Promise<Scope>;
}

async function _alchemy(
  appName: string,
  options?: Omit<AlchemyOptions, "appName">,
): Promise<Scope> {
  // Parse CLI arguments
  const cliOptions = {
    phase: cliArgs.includes("--destroy")
      ? "destroy"
      : cliArgs.includes("--read")
        ? "read"
        : "up",
    local: cliArgs.includes("--local") || cliArgs.includes("--dev"),
    watch: cliArgs.includes("--watch") || execArgv.includes("--watch"),
    quiet: cliArgs.includes("--quiet"),
    force: cliArgs.includes("--force"),
    stage: parseStage(),  // --stage my-stage
    password: process.env.ALCHEMY_PASSWORD,
    adopt: cliArgs.includes("--adopt"),
    eraseSecrets: cliArgs.includes("--erase-secrets"),
    rootDir: parseOption("--root-dir", ALCHEMY_ROOT),
    profile: parseOption("--profile"),
  };

  // Merge CLI options with provided options (provided wins)
  const mergedOptions = { ...cliOptions, ...options };

  // CI safety check
  if (mergedOptions.stateStore === undefined && process.env.CI) {
    throw new Error(`Running in CI with local state store.
Use S3StateStore or CloudflareStateStore instead.`);
  }

  // Create root scope
  const scope = new Scope({
    appName,
    phase: mergedOptions.phase,
    stage: mergedOptions.stage,
    password: mergedOptions.password,
    stateStore: mergedOptions.stateStore,
    ...mergedOptions,
  });

  return scope.run(() => scope);
}
```

## 7. Valtron Replication Patterns

### 7.1 Resource Translation

```valtron
// TypeScript Resource
export const Worker = Resource(
  "cloudflare::Worker",
  async function (this: Context<Worker>, id: string, props: WorkerProps) {
    const api = await createCloudflareApi(props);
    const name = props.name ?? this.scope.createPhysicalName(id);

    if (this.phase === "create") {
      const bundle = await bundleWorker({ entrypoint: props.entrypoint });
      await api.request("PUT", `/accounts/${api.accountId}/workers/scripts/${name}`, formData);
      return this.create({ id: name, name, url: `https://${name}.workers.dev` });
    }
  }
);

// Valtron Translation
provider Cloudflare {
  credentials: {
    api_token: String?,
    api_key: String?,
    email: String?,
  },
  account_id: String,
}

resource Worker {
  props: {
    name: String?,
    entrypoint: String,
    bindings: List<Binding>,
    compatibility_date: String?,
  },
  output: { id: String, name: String, url: String },
  lifecycle: {
    create: create_worker,
    update: update_worker,
    delete: delete_worker,
  }
}

operation create_worker(props: WorkerProps) -> WorkerOutput {
  let bundle = bundle_worker(props.entrypoint)
  let name = props.name ?? create_physical_name(id)

  let response = http_put(
    "https://api.cloudflare.com/client/v4/accounts/{account_id}/workers/scripts/{name}",
    headers = provider.auth_headers,
    body = bundle
  )

  WorkerOutput { id: name, name: name, url: "https://{name}.workers.dev" }
}
```

### 7.2 Scope Translation

```valtron
// TypeScript Scope with AsyncLocalStorage
export class Scope {
  public static storage = new AsyncLocalStorage<Scope>();
  public readonly parent: Scope | undefined;
  public readonly resources = new Map<ResourceID, PendingResource>();
  public readonly state: StateStore;

  public static get current(): Scope {
    return Scope.storage.getStore();
  }
}

// Valtron Translation (TaskIterator pattern)
scope App {
  name: String,
  stage: String,
  parent: Scope?,
  state_store: StateStore,
  resources: Map<String, Resource>,
}

operation create_scope(name: String, stage: String) -> Scope {
  let scope = Scope {
    name: name,
    stage: stage,
    parent: current_scope(),
    state_store: create_state_store(stage),
    resources: Map::new(),
  }
  set_current_scope(scope)
  scope
}

operation run_in_scope<T>(scope: Scope, fn: Fn<Scope, T>) -> T {
  let old_scope = current_scope()
  set_current_scope(scope)
  let result = fn(scope)
  set_current_scope(old_scope)
  finalize_scope(scope)
  result
}
```

### 7.3 StateStore Translation

```valtron
// TypeScript FileSystemStateStore
export class FileSystemStateStore implements StateStore {
  private dir: string;

  async set(key: string, value: State): Promise<void> {
    const file = this.getPath(key);
    await fs.promises.writeFile(
      file,
      JSON.stringify(await serialize(this.scope, value), null, 2)
    );
  }
}

// Valtron Translation
state_store FileSystem {
  root_dir: String,
  scope_chain: List<String>,
}

operation fs_set(store: FileSystem, key: String, value: State) {
  let path = build_path(store.root_dir, store.scope_chain, key)
  let serialized = serialize(value)
  let json = to_json(serialized)
  file_write(path, json)
}

operation fs_get(store: FileSystem, key: String) -> State? {
  let path = build_path(store.root_dir, store.scope_chain, key)
  match file_read(path) {
    Some(content) -> {
      let parsed = from_json(content)
      Some(deserialize(parsed))
    },
    None -> None
  }
}
```

## Summary

Alchemy's architecture is built on three core abstractions:

1. **Resources**: Memoized async functions with Symbol-keyed metadata
2. **Scopes**: Hierarchical execution context via AsyncLocalStorage
3. **StateStores**: Pluggable state persistence (FileSystem, S3, etc.)

The `apply()` function implements the create/update lifecycle, comparing serialized props to determine if resources need updating. Providers implement handlers that execute cloud API calls within this lifecycle.

For Valtron replication:
- Resources become `provider` + `resource` definitions
- Scopes become explicit context passing (TaskIterator pattern)
- StateStores become algebraic effect handlers
- AsyncLocalStorage becomes manual scope stack management

## Next Steps

- [04-state-management-deep-dive.md](./04-state-management-deep-dive.md) - Deep dive into state stores and serde
- [05-provider-patterns-deep-dive.md](./05-provider-patterns-deep-dive.md) - Provider implementation patterns
- [rust-revision.md](./rust-revision.md) - Rust replication guide
