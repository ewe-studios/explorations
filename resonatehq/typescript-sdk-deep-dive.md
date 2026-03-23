# Resonate TypeScript SDK Deep Dive

## Overview

The Resonate TypeScript SDK provides the client-side implementation of the Distributed Async Await programming model. It enables developers to write distributed applications using familiar async/await patterns with durability guarantees.

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.resonatehq/resonate-sdk-ts/`

## Architecture

### Directory Structure

```
resonate-sdk-ts/
├── lib/
│   ├── resonate.ts          # Core Resonate and Context classes
│   ├── index.ts             # Public exports
│   └── core/
│       ├── promises/
│       │   ├── promises.ts  # DurablePromise implementation
│       │   └── types.ts     # Type definitions
│       ├── schedules/
│       │   ├── schedules.ts # Schedule implementation
│       │   └── types.ts     # Schedule types
│       ├── stores/
│       │   ├── local.ts     # Local in-memory store
│       │   └── remote.ts    # Remote HTTP store
│       ├── storages/
│       │   ├── memory.ts    # Memory storage backend
│       │   └── withTimeout.ts # Timeout wrapper
│       ├── encoders/
│       │   └── json.ts      # JSON encoder
│       ├── loggers/
│       │   └── logger.ts    # Logger implementation
│       ├── store.ts         # Store interfaces
│       ├── storage.ts       # Storage interfaces
│       ├── encoder.ts       # Encoder interface
│       ├── logger.ts        # Logger interface
│       ├── options.ts       # Configuration options
│       ├── retry.ts         # Retry policies
│       ├── errors.ts        # Error types
│       └── utils.ts         # Utilities
├── test/                     # Test suite
├── examples/                 # Example code
└── package.json
```

## Core Classes

### Resonate Class

The main entry point for the SDK:

```typescript
export class Resonate {
  #registeredFunctions: Record<string, Record<number, { func: Func; opts: Options }>> = {};
  #invocationHandles: Map<string, InvocationHandle<any>>;
  #interval: NodeJS.Timeout | undefined;
  #resources: Map<string, any>;

  readonly store: IStore;
  readonly logger: ILogger;
  readonly defaultInvocationOptions: Options;
  readonly promises: ResonatePromises;
  readonly schedules: ResonateSchedules;

  constructor({
    auth = undefined,
    encoder = new JSONEncoder(),
    heartbeat = 15000,      // 15s
    logger = new Logger(),
    pid = utils.randomId(),
    pollFrequency = 5000,   // 5s
    retryPolicy = retryPolicies.exponential(),
    store = undefined,
    tags = {},
    timeout = 10000,        // 10s
    url = undefined,
  }: Partial<ResonateOptions> = {}) {
    // Initialize store (local or remote)
    if (url) {
      this.store = new RemoteStore(url, { auth, heartbeat, logger, pid });
    } else {
      this.store = new LocalStore({ auth, heartbeat, logger, pid });
    }
    // ...
  }
}
```

### Key Methods

#### register()

Register a function for durable execution:

```typescript
register(name: string, func: Func, opts: Partial<Options> = {}): void {
  opts.version = opts.version ?? 1;
  const options = this.withDefaultOpts(opts);

  if (options.version <= 0) {
    throw new Error("Version must be greater than 0");
  }

  // Register with version tracking
  const latestVersion = Math.max(...Object.values(this.#registeredFunctions[name]).map(f => f.opts.version));
  if (options.version > latestVersion) {
    this.#registeredFunctions[name][0] = { func, opts: options }; // 0 = latest
  }
  this.#registeredFunctions[name][options.version] = { func, opts: options };
}
```

#### invokeLocal()

Invoke a function locally with durability:

```typescript
async invokeLocal<R>(
  name: string,
  id: string,
  ...argsWithOverrides: [...any, InvocationOverrides?]
): Promise<InvocationHandle<R>> {
  if (this.#invocationHandles.has(id)) {
    return this.#invocationHandles.get(id) as InvocationHandle<R>;
  }

  const { args, opts: optionOverrides } = utils.split(argsWithOverrides);
  const { func, opts: registeredOpts } = this.registeredFunction(name, givenVersion);

  // Merge options
  const opts: Options = utils.merge(optionOverrides, registeredOpts);
  opts.tags = { ...registeredOpts.tags, ...tags, "resonate:invocation": "true" };
  opts.shouldLock = opts.shouldLock ?? false;

  // Create param for recovery
  const param = {
    func: name,
    version: opts.version,
    retryPolicy: opts.retryPolicy,
    args,
  };

  // Create durable promise
  const idempotencyKey = opts.idempotencyKeyFn(id);
  const storedPromise = await this.promisesStore.create(
    id, idempotencyKey, false, undefined,
    opts.encoder.encode(param),
    Date.now() + opts.timeout,
    opts.tags,
  );

  // Create context and run function
  const ctx = Context.createRootContext(this, { id, name, opts, eid: opts.eidFn(id) });
  const resultPromise = _runFunc<R>(
    func, ctx, args, idempotencyKey, storedPromise,
    this.store.locks, this.store.promises,
  );

  const handle = new InvocationHandle(id, resultPromise);
  this.#invocationHandles.set(id, handle);
  return handle;
}
```

#### start()

Start the recovery loop:

```typescript
async start(delay: number = 5000) {
  clearInterval(this.#interval);
  await this.#_start(); // First run immediately
  this.#interval = setInterval(this.#_start.bind(this), delay);
}

async #_start() {
  try {
    // Search for pending invocation promises
    for await (const promises of this.promisesStore.search(
      "*", "pending", { "resonate:invocation": "true" }
    )) {
      for (const promiseRecord of promises) {
        // Decode and re-execute
        const param = this.defaultInvocationOptions.encoder.decode(promiseRecord.param.data);
        if (param && typeof param === "object" && "func" in param) {
          const idempotencyKeyFn = (_: string) => {
            return promiseRecord.idempotencyKeyForCreate ??
                   this.defaultInvocationOptions.idempotencyKeyFn(promiseRecord.id);
          };
          await this.invokeLocal(
            param.func, promiseRecord.id, ...param.args,
            options({ retryPolicy: param.retryPolicy, version: param.version, idempotencyKeyFn }),
          );
        }
      }
    }
  } catch (e) {
    this.logger.error(e); // Squash errors, retry on next interval
  }
}
```

### Context Class

The execution context for durable functions:

```typescript
export class Context {
  #resonate: Resonate;
  #stopAllPolling: boolean = false;
  #invocationHandles: Map<string, InvocationHandle<any>>;
  #aborted: boolean;
  #abortCause: any;
  #resources: Map<string, any>;
  #finalizers: (() => Promise<void>)[];
  childrenCount: number;
  readonly invocationData: InvocationData;
  parent: Context | undefined;
  root: Context;

  // Create root context for top-level invocations
  static createRootContext(resonate: Resonate, invocationData: InvocationData): Context {
    return new Context(resonate, invocationData, undefined);
  }

  // Create child context for nested invocations
  static createChildrenContext(parentCtx: Context, invocationData: InvocationData): Context {
    return new Context(parentCtx.#resonate, invocationData, parentCtx);
  }
}
```

### Context Methods

#### run()

Execute a function (local or remote):

```typescript
async run<F extends Func, R>(
  funcOrId: F | string,
  ...argsWithOpts: [...Params<F>, PartialOptions?]
): Promise<ReturnType<F> | R> {
  let handle: InvocationHandle<R>;
  if (typeof funcOrId === "string") {
    handle = await this.invokeRemote<R>(funcOrId, ...argsWithOpts);
  } else {
    handle = await this.invokeLocal<F, ReturnType<F>>(funcOrId, ...argsWithOpts);
  }
  return await handle.result();
}
```

#### invokeLocal() (Child)

Invoke a child function:

```typescript
async invokeLocal<F extends Func, R>(
  func: F,
  ...argsWithOpts: [...Params<F>, PartialOptions?]
): Promise<InvocationHandle<R>> {
  const { args, opts: givenOpts } = utils.split(argsWithOpts);
  const { opts: registeredOpts } = this.#resonate.registeredFunction(
    this.root.invocationData.name,
    this.root.invocationData.opts.version,
  );

  const opts = { ...registeredOpts, ...givenOpts };
  opts.tags = { ...registeredOpts.tags, ...givenOpts.tags };
  opts.shouldLock = opts.shouldLock ?? false; // false for children

  this.childrenCount++;
  const name = func.name ? func.name : `${this.invocationData.name}__anon${this.childrenCount}`;
  const id = `${this.invocationData.id}.${this.childrenCount}.${name}`;

  // Check for existing handle (deduplication)
  if (this.#invocationHandles.has(id)) {
    return this.#invocationHandles.get(id) as InvocationHandle<R>;
  }

  const ctx = Context.createChildrenContext(this, { name, id, eid: opts.eidFn(id), opts });

  if (!opts.durable) {
    // Non-durable execution
    const runFunc = async () => {
      return await runWithRetry(
        async () => await func(ctx, ...args),
        async () => await ctx.onRetry(),
        opts.retryPolicy,
        Date.now() + opts.timeout,
      );
    };
    const resultPromise = runFunc();
    const handle = new InvocationHandle<R>(id, resultPromise);
    this.#invocationHandles.set(id, handle);
    return handle;
  }

  // Durable execution
  const param = {}; // No params needed for children
  const idempotencyKey = opts.idempotencyKeyFn(id);
  const storedPromise = await this.#resonate.promisesStore.create(
    id, idempotencyKey, false, undefined,
    opts.encoder.encode(param),
    Date.now() + opts.timeout,
    opts.tags,
  );

  const resultPromise = _runFunc<R>(
    func, ctx, args, idempotencyKey, storedPromise,
    this.#resonate.store.locks, this.#resonate.store.promises,
  );

  const handle = new InvocationHandle(id, resultPromise);
  this.#invocationHandles.set(id, handle);
  return handle;
}
```

#### invokeRemote()

Invoke a remote function by polling:

```typescript
async invokeRemote<R>(funcId: string, ...argsWithOpts: [...any, PartialOptions?]): Promise<InvocationHandle<R>> {
  if (this.#invocationHandles.has(funcId)) {
    return this.#invocationHandles.get(funcId) as InvocationHandle<R>;
  }

  const { opts } = utils.split(argsWithOpts);
  opts.tags = { ...registeredOpts.tags, ...givenOpts?.tags };
  opts.shouldLock = false;

  // Create promise for remote invocation
  const param = {};
  const idempotencyKey = opts.idempotencyKeyFn(funcId);
  const storedPromise = await this.#resonate.promisesStore.create(
    funcId, idempotencyKey, false, undefined,
    opts.encoder.encode(param),
    Date.now() + opts.timeout,
    opts.tags,
  );

  // Poll for completion
  const runFunc = async (): Promise<R> => {
    while (!this.#stopAllPolling) {
      const durablePromiseRecord = await this.#resonate.promisesStore.get(storedPromise.id);
      if (durablePromiseRecord.state !== "PENDING") {
        return handleCompletedPromise(durablePromiseRecord, opts.encoder);
      }
      await sleep(opts.pollFrequency);
    }
    throw new Error(`Polling stopped`);
  };

  const resultPromise = runFunc();
  const handle = new InvocationHandle(funcId, resultPromise);
  this.#invocationHandles.set(funcId, handle);
  return handle;
}
```

#### sleep()

Durable sleep:

```typescript
async sleep(ms: number): Promise<void> {
  const id = `${this.invocationData.id}.${this.childrenCount++}`;
  const handle = await this.invokeRemote(
    id,
    options({
      timeout: ms,
      pollFrequency: ms,
      tags: { "resonate:timeout": "true" },
      durable: true,
    }),
  );
  await handle.result();
}
```

### DurablePromise Class

Lower-level durable promise API:

```typescript
export class DurablePromise<T> {
  private readonly completed: Promise<DurablePromise<T>>;
  private complete!: (value: DurablePromise<T>) => void;
  private interval: NodeJS.Timeout | undefined;

  constructor(
    private store: IPromiseStore,
    private encoder: IEncoder<unknown, string | undefined>,
    private promise: DurablePromiseRecord,
  ) {
    this.completed = new Promise((resolve) => {
      this.complete = resolve;
    });
  }

  // Static factory methods
  static async create<T>(store, encoder, id, timeout, opts?) { ... }
  static async resolve<T>(store, encoder, id, value, opts?) { ... }
  static async reject<T>(store, encoder, id, error, opts?) { ... }
  static async cancel<T>(store, encoder, id, error, opts?) { ... }
  static async get<T>(store, encoder, id) { ... }
  static async *search(store, encoder, id, state?, tags?, limit?) { ... }

  // Instance methods
  async resolve(value: T, opts?) { ... }
  async reject(error: any, opts?) { ... }
  async cancel(error: any, opts?) { ... }

  // Polling methods
  async sync(timeout: number = Infinity, frequency: number = 5000): Promise<void> {
    clearInterval(this.interval);
    this.interval = setInterval(() => this.poll(), frequency);
    await this.poll();

    const timeoutPromise = timeout === Infinity
      ? new Promise(() => {})
      : new Promise(resolve => timeoutId = setTimeout(resolve, timeout));

    await Promise.race([this.completed, timeoutPromise]);
    clearInterval(this.interval);
    clearTimeout(timeoutId);

    if (this.pending) {
      throw new Error("Timeout waiting for promise");
    }
  }

  async wait(timeout: number = Infinity, frequency: number = 5000): Promise<T> {
    await this.sync(timeout, frequency);
    if (this.resolved) return this.value();
    else throw this.error();
  }

  private async poll() {
    try {
      this.promise = await this.store.get(this.id);
      if (!this.pending) this.complete(this);
    } catch (e) {
      // Log error
    }
  }
}
```

## Store Implementations

### LocalStore

In-memory store for development and testing:

```typescript
export class LocalStore implements IStore {
  public promises: LocalPromiseStore;
  public schedules: LocalScheduleStore;
  public locks: LocalLockStore;

  private toSchedule: Schedule[] = [];
  private next: number | undefined = undefined;

  constructor(
    opts: Partial<StoreOptions> = {},
    promiseStorage: IStorage<DurablePromiseRecord> = new WithTimeout(new MemoryStorage()),
    scheduleStorage: IStorage<Schedule> = new MemoryStorage(),
    lockStorage: IStorage<{ id: string; eid: string }> = new MemoryStorage(),
  ) {
    this.promises = new LocalPromiseStore(this, promiseStorage);
    this.schedules = new LocalScheduleStore(this, scheduleStorage);
    this.locks = new LocalLockStore(this, lockStorage);
    this.init();
  }

  // Schedule management
  addSchedule(schedule: Schedule) {
    this.toSchedule = this.toSchedule.filter(s => s.id != schedule.id).concat(schedule);
    this.setSchedule();
  }

  private setSchedule() {
    clearTimeout(this.next);
    this.toSchedule.sort((a, b) => a.nextRunTime - b.nextRunTime);
    if (this.toSchedule.length > 0) {
      this.next = +setTimeout(() => this.schedulePromise(), this.toSchedule[0].nextRunTime - Date.now());
    }
  }

  private schedulePromise() {
    const schedule = this.toSchedule.shift();
    if (schedule) {
      const id = this.generatePromiseId(schedule);
      try {
        this.promises.create(id, id, false, schedule.promiseParam?.headers,
          schedule.promiseParam?.data, Date.now() + schedule.promiseTimeout,
          { ...schedule.promiseTags, "resonate:schedule": schedule.id });
      } catch (error) {
        this.logger.warn("error creating scheduled promise", error);
      }
      try {
        this.schedules.update(schedule.id, schedule.nextRunTime);
      } catch (error) {
        this.logger.warn("error updating schedule", error);
      }
    }
  }
}
```

### LocalPromiseStore

Read-modify-write pattern for promises:

```typescript
export class LocalPromiseStore implements IPromiseStore {
  constructor(private store: LocalStore, private storage: IStorage<DurablePromiseRecord>) {}

  async create(id, ikey, strict, headers, data, timeout, tags): Promise<DurablePromiseRecord> {
    return this.storage.rmw(id, (promise) => {
      if (!promise) {
        // Create new promise
        return {
          state: "PENDING",
          id, timeout,
          param: { headers, data },
          value: { headers: undefined, data: undefined },
          createdOn: Date.now(),
          completedOn: undefined,
          idempotencyKeyForCreate: ikey,
          idempotencyKeyForComplete: undefined,
          tags,
        };
      }

      // Handle existing promise
      if (strict && !isPendingPromise(promise)) {
        throw new ResonateError("Forbidden", ErrorCodes.STORE_FORBIDDEN);
      }
      if (promise.idempotencyKeyForCreate === undefined || ikey !== promise.idempotencyKeyForCreate) {
        throw new ResonateError("Forbidden", ErrorCodes.STORE_FORBIDDEN);
      }
      return promise;
    });
  }

  async resolve(id, ikey, strict, headers, data): Promise<DurablePromiseRecord> {
    return this.storage.rmw(id, (promise) => {
      if (!promise) throw new ResonateError("Not found", ErrorCodes.STORE_NOT_FOUND);

      if (isPendingPromise(promise)) {
        return {
          state: "RESOLVED",
          id: promise.id, timeout: promise.timeout, param: promise.param,
          value: { headers, data },
          createdOn: promise.createdOn, completedOn: Date.now(),
          idempotencyKeyForCreate: promise.idempotencyKeyForCreate,
          idempotencyKeyForComplete: ikey, tags: promise.tags,
        };
      }

      if (strict && !isResolvedPromise(promise)) {
        throw new ResonateError("Forbidden", ErrorCodes.STORE_FORBIDDEN);
      }
      if (!isTimedoutPromise(promise) &&
          (promise.idempotencyKeyForComplete === undefined || ikey !== promise.idempotencyKeyForComplete)) {
        throw new ResonateError("Forbidden", ErrorCodes.STORE_FORBIDDEN);
      }
      return promise;
    });
  }

  // reject(), cancel(), get() follow similar patterns...

  async *search(id, state?, tags?, limit?): AsyncGenerator<DurablePromiseRecord[]> {
    const regex = new RegExp(id.replaceAll("*", ".*"));
    const states = searchStates(state);
    const tagEntries = Object.entries(tags ?? {});

    for await (const promises of this.storage.all()) {
      yield promises
        .filter(p => states.includes(p.state))
        .filter(p => regex.test(p.id))
        .filter(p => tagEntries.every(([k, v]) => p.tags?.[k] == v));
    }
  }
}
```

### RemoteStore

HTTP-based store for production:

```typescript
export class RemoteStore implements IStore {
  private url: string;
  private auth: any;
  private pid: string;
  private heartbeat: number;
  private logger: ILogger;

  constructor(url: string, opts: { auth?, pid, heartbeat, logger }) {
    this.url = url;
    this.auth = opts.auth;
    this.pid = opts.pid;
    this.heartbeat = opts.heartbeat;
    this.logger = opts.logger;
    this.startHeartbeat();
  }

  private startHeartbeat() {
    setInterval(async () => {
      // Send heartbeat to maintain task ownership
      await fetch(`${this.url}/tasks/heartbeat`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ processId: this.pid }),
      });
    }, this.heartbeat);
  }

  async create(id, ikey, strict, headers, data, timeout, tags): Promise<DurablePromiseRecord> {
    const response = await fetch(`${this.url}/promises`, {
      method: "POST",
      headers: { "Content-Type": "application/json", ...this.authHeaders },
      body: JSON.stringify({
        id, idempotencyKey: ikey, strict,
        param: { headers, data }, timeout, tags,
      }),
    });
    return response.json();
  }

  // Other methods follow similar HTTP patterns...
}
```

## Retry System

### Retry Policies

```typescript
export type RetryPolicy = Exponential | Linear | Never;

export type Exponential = {
  kind: "exponential";
  initialDelayMs: number;
  backoffFactor: number;
  maxAttempts: number;
  maxDelayMs: number;
};

export function exponential(
  initialDelayMs = 100,
  backoffFactor = 2,
  maxAttempts = Infinity,
  maxDelayMs = 60000,
): Exponential {
  return { kind: "exponential", initialDelayMs, backoffFactor, maxAttempts, maxDelayMs };
}

export function linear(delayMs = 1000, maxAttempts = Infinity): Linear {
  return { kind: "linear", delayMs, maxAttempts };
}

export function never(): Never {
  return { kind: "never" };
}
```

### Retry Execution

```typescript
export async function runWithRetry<T>(
  func: () => Promise<T>,
  onRetry: () => Promise<void>,
  retryPolicy: RetryPolicy,
  timeout: number,
) {
  let error;
  const ctx = { attempt: 0, retryPolicy, timeout };

  for (const delay of retryIterator(ctx)) {
    await new Promise(resolve => setTimeout(resolve, delay));

    if (ctx.attempt > 0) {
      await onRetry(); // Reset context state
    }

    try {
      return await func();
    } catch (e) {
      error = e;
      ctx.attempt++;
    }
  }

  throw error;
}

export function retryIterator<T extends { retryPolicy: RetryPolicy; attempt: number; timeout: number }>(
  ctx: T,
): IterableIterator<number> {
  const { initialDelay, backoffFactor, maxAttempts, maxDelay } = retryDefaults(ctx.retryPolicy);

  return {
    next() {
      const delay = Math.min(
        Math.min(ctx.attempt, 1) * initialDelay * Math.pow(backoffFactor, ctx.attempt - 1),
        maxDelay,
      );

      if (Date.now() + delay >= ctx.timeout || ctx.attempt >= maxAttempts) {
        return { done: true };
      }

      return { done: false, value: delay || 0 };
    },
    [Symbol.iterator]() { return this; },
  };
}
```

## Function Execution Flow

### _runFunc

The core execution function:

```typescript
const _runFunc = async <R>(
  func: Func,
  ctx: Context,
  args: Params<Func>,
  idempotencyKey: string,
  storedPromise: DurablePromiseRecord,
  locksStore: ILockStore,
  promisesStore: IPromiseStore,
): Promise<R> => {
  const { id, eid, opts } = ctx.invocationData;

  // 1. Check if promise already completed
  if (storedPromise.state !== "PENDING") {
    return handleCompletedPromise(storedPromise, opts.encoder);
  }

  try {
    // 2. Acquire lock if needed
    if (opts.shouldLock) {
      while (!(await acquireLock(id, eid, locksStore))) {
        await sleep(opts.pollFrequency);
      }
    }

    let error: any;
    let value!: R;
    let success = true;

    try {
      // 3. Run function with retry
      value = await runWithRetry(
        async () => await func(ctx, ...args),
        async () => await ctx.onRetry(),
        opts.retryPolicy,
        storedPromise.timeout,
      );
    } catch (e) {
      error = e;
      success = false;
    } finally {
      // 4. Finalize context (await children, run finalizers)
      await ctx.finalize();
    }

    // 5. Check for abort
    if (ctx.root.aborted) {
      throw new ResonateError("Unrecoverable Error: Aborting", ErrorCodes.ABORT, ctx.root.abortCause);
    }

    // 6. Complete promise
    let completedPromiseRecord: DurablePromiseRecord;
    if (success) {
      completedPromiseRecord = await promisesStore.resolve(
        id, idempotencyKey, false, storedPromise.value.headers, opts.encoder.encode(value),
      );
    } else {
      completedPromiseRecord = await promisesStore.reject(
        id, idempotencyKey, false, storedPromise.value.headers, opts.encoder.encode(error),
      );
    }

    return handleCompletedPromise(completedPromiseRecord, opts.encoder);
  } catch (err) {
    if (err instanceof ResonateError &&
        (err.code === ErrorCodes.CANCELED || err.code === ErrorCodes.TIMEDOUT)) {
      throw err;
    } else if (err instanceof ResonateError && err.code !== ErrorCodes.ABORT) {
      ctx.abort(err);
      throw new ResonateError("Unrecoverable Error: Aborting", ErrorCodes.ABORT, err);
    } else {
      throw err;
    }
  } finally {
    // 7. Release lock if needed
    if (opts.shouldLock) {
      await locksStore.release(id, eid);
    }
  }
};
```

## Resource Management

### setResource / getResource

Context-scoped resources:

```typescript
// In Context class
setResource(name: string, resource: any, finalizer?: () => Promise<void>): void {
  if (this.#resources.has(name)) {
    throw new Error("Resource already set for this context");
  }
  this.#resources.set(name, resource);
  if (finalizer) {
    this.#finalizers.push(finalizer);
  }
}

getResource<R>(name: string): R | undefined {
  let resource = this.#resources.get(name);
  if (resource) return resource as R;

  // Search parent contexts
  resource = this.parent ? this.parent.getResource<R>(name) : undefined;
  if (!resource) {
    return this.#resonate.getResource(name);
  }
  return resource;
}
```

### Finalizers

```typescript
async finalize() {
  // Await all child invocation handles
  await Promise.allSettled(
    Array.from(this.#invocationHandles, ([_, handle]) => handle.result())
  );

  // Run finalizers in reverse order (LIFO)
  for (const finalizer of this.#finalizers.reverse()) {
    await finalizer();
  }

  this.#resources.clear();
  this.#finalizers = [];
}
```

## Error Handling

### Error Types

```typescript
export enum ErrorCodes {
  CANCELED = "CANCELED",
  TIMEDOUT = "TIMEDOUT",
  ABORT = "ABORT",
  STORE_NOT_FOUND = "STORE_NOT_FOUND",
  STORE_FORBIDDEN = "STORE_FORBIDDEN",
  STORE_ALREADY_EXISTS = "STORE_ALREADY_EXISTS",
}

export class ResonateError extends Error {
  constructor(
    message: string,
    public code: ErrorCodes,
    public cause?: any,
  ) {
    super(message);
  }
}
```

### handleCompletedPromise

```typescript
export function handleCompletedPromise<R>(
  p: DurablePromiseRecord,
  encoder: IEncoder<unknown, string | undefined>,
): R {
  assert(p.state !== "PENDING", "Promise was pending when trying to handle its completion");

  switch (p.state) {
    case "RESOLVED":
      return encoder.decode(p.value.data) as R;
    case "REJECTED":
      throw encoder.decode(p.value.data);
    case "REJECTED_CANCELED":
      throw new ResonateError("Resonate function canceled", ErrorCodes.CANCELED, encoder.decode(p.value.data));
    case "REJECTED_TIMEDOUT":
      throw new ResonateError(
        `Resonate function timedout at ${new Date(p.timeout).toISOString()}`,
        ErrorCodes.TIMEDOUT,
      );
  }
}
```

## Usage Patterns

### Basic Function Registration

```typescript
import { Resonate } from "@resonatehq/sdk";

const resonate = new Resonate({ url: "http://localhost:8080" });

// Register a function
resonate.register("hello", async (ctx, name: string) => {
  return `Hello, ${name}!`;
});

// Invoke the function
const result = await resonate.run("hello", "hello-world", "World");
console.log(result); // "Hello, World!"
```

### Child Functions

```typescript
resonate.register("parent", async (ctx) => {
  // Invoke child function
  const childResult = await ctx.run(async (childCtx) => {
    return "child result";
  });

  // Invoke registered child
  const registeredChild = await ctx.run("child", "child-id", "arg1", "arg2");

  return { childResult, registeredChild };
});
```

### Durable Sleep

```typescript
resonate.register("waiter", async (ctx) => {
  console.log("Waiting 10 seconds...");
  await ctx.sleep(10000);
  console.log("Done waiting!");
});
```

### Resource Management

```typescript
resonate.register("db-user", async (ctx) => {
  // Get or create database connection
  let db = ctx.getResource<Database>("db");
  if (!db) {
    db = await createDatabase();
    ctx.setResource("db", db, async () => {
      await db.close();
    });
  }

  return await db.query("SELECT * FROM users");
});
```
