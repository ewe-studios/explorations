---
source: /home/darkvoid/Boxxed/@formulas/src.rivet-dev/rivetkit
repository: github.com/rivet-dev/rivetkit
explored_at: 2026-04-05
focus: Actor lifecycle, state management, initialization, persistence, termination
---

# Deep Dive: Actor Lifecycle and State Management

## Overview

This deep dive examines RivetKit's actor lifecycle - how actors are created, initialized, how state is managed and persisted, and how actors are terminated. Understanding the lifecycle is crucial for building reliable stateful applications.

## Architecture

```mermaid
sequenceDiagram
    participant Client
    participant Registry
    participant Actor
    participant Driver
    participant Storage

    Client->>Registry: getOrCreate("actor-key")
    Registry->>Registry: Check if actor exists
    
    alt Actor exists
        Registry->>Actor: Load from storage
        Actor->>Driver: load("actor-key")
        Driver->>Storage: SELECT state FROM actors WHERE key = "actor-key"
        Storage-->>Driver: State (JSON)
        Driver-->>Actor: State object
        Actor->>Actor: Apply state
        Actor-->>Registry: Actor instance
    else Actor doesn't exist
        Registry->>Actor: Create new instance
        Actor->>Actor: Initialize default state
        Actor->>Actor: Call onInit()
        Actor-->>Registry: New actor instance
    end
    
    Registry-->>Client: Actor proxy
    
    Client->>Actor: action.call(params)
    Actor->>Actor: Execute action
    Actor->>Driver: save(state)
    Driver->>Storage: UPSERT actor state
    Storage-->>Driver: OK
    Driver-->>Actor: Persisted
    Actor-->>Client: Result
```

## Actor Creation

### Registry Pattern

```typescript
// packages/core/src/registry/registry.ts

import { Actor, ActorDefinition } from "../actor";
import { Driver } from "../drivers";

export class Registry {
  private actors: Map<string, Actor<any>> = new Map();
  private definitions: Map<string, ActorDefinition<any>> = new Map();
  private driver: Driver;

  constructor(
    definitions: Record<string, ActorDefinition<any>>,
    driver: Driver
  ) {
    this.definitions = new Map(Object.entries(definitions));
    this.driver = driver;
  }

  /**
   * Get or create an actor instance
   */
  getOrCreate<T extends string>(
    type: T,
    key: string
  ): ActorProxy<this.definitions[T]> {
    const actorId = `${type}:${key}`;

    // Check if actor is already in memory
    let actor = this.actors.get(actorId);

    if (!actor) {
      // Load or create actor
      actor = this.createActor(type, key);
      this.actors.set(actorId, actor);
    }

    return this.createProxy(actor);
  }

  /**
   * Create a new actor instance
   */
  private createActor<T extends string>(
    type: T,
    key: string
  ): Actor<this.definitions[T]> {
    const definition = this.definitions.get(type)!;

    // Try to load state from driver
    const loadedState = this.driver.load(type, key);

    if (loadedState) {
      // Actor exists in storage - load it
      return new Actor(definition, key, loadedState);
    } else {
      // New actor - initialize with default state
      const initialState = this.createInitialState(definition);
      return new Actor(definition, key, initialState);
    }
  }

  /**
   * Create initial state from definition
   */
  private createInitialState(definition: ActorDefinition<any>): any {
    if (typeof definition.state === "function") {
      return definition.state();
    }
    return { ...definition.state };
  }
}
```

### Actor Definition

```typescript
// packages/core/src/actor/actor.ts

import { Driver } from "../drivers";

export interface ActorDefinition<TState, TActions = any> {
  /**
   * Initial state or state factory function
   */
  state: TState | (() => TState);

  /**
   * Action handlers
   */
  actions: {
    [K in keyof TActions]: (ctx: ActionContext<TState>, ...args: any[]) => any;
  };

  /**
   * Lifecycle hooks
   */
  onInit?: (ctx: ActionContext<TState>) => void | Promise<void>;
  onBeforeSave?: (ctx: ActionContext<TState>) => void | Promise<void>;
  onAfterSave?: (ctx: ActionContext<TState>) => void | Promise<void>;
  onTerminate?: (ctx: ActionContext<TState>) => void | Promise<void>;

  /**
   * Configuration
   */
  ttl?: number; // Time-to-live in milliseconds
  persistInterval?: number; // Auto-persist interval
}

export interface ActionContext<TState> {
  /**
   * Current state (mutable)
   */
  state: TState;

  /**
   * Actor key
   */
  key: string;

  /**
   * Broadcast event to subscribed clients
   */
  broadcast: (event: string, data: any) => void;

  /**
   * Get actor metadata
   */
  meta: {
    createdAt: Date;
    updatedAt: Date;
    version: number;
  };

  /**
   * Access to external services
   */
  env: Record<string, any>;
}

export class Actor<TDefinition extends ActorDefinition<any>> {
  public state: TDefinition["state"];
  public key: string;
  public meta: ActionContext<any>["meta"];
  
  private definition: TDefinition;
  private driver: Driver;
  private subscribers: Set<(event: string, data: any) => void> = new Set();
  private persistTimer: NodeJS.Timeout | null = null;

  constructor(
    definition: TDefinition,
    key: string,
    initialState: TDefinition["state"]
  ) {
    this.definition = definition;
    this.key = key;
    this.state = initialState;
    this.meta = {
      createdAt: new Date(),
      updatedAt: new Date(),
      version: 1,
    };

    // Initialize actor
    this.initialize();
  }

  /**
   * Initialize actor and call lifecycle hooks
   */
  private async initialize(): Promise<void> {
    const ctx = this.createContext();

    // Call onInit hook
    if (this.definition.onInit) {
      await this.definition.onInit(ctx);
    }

    // Start auto-persist if configured
    if (this.definition.persistInterval) {
      this.startAutoPersist();
    }
  }

  /**
   * Create action context
   */
  private createContext(): ActionContext<any> {
    return {
      state: this.state,
      key: this.key,
      broadcast: (event, data) => this.broadcast(event, data),
      meta: this.meta,
      env: process.env,
    };
  }

  /**
   * Call an action
   */
  async call<TAction extends keyof TDefinition["actions"]>(
    actionName: TAction,
    ...args: any[]
  ): Promise<any> {
    const action = this.definition.actions[actionName];

    if (!action) {
      throw new Error(`Action ${String(actionName)} not found`);
    }

    const ctx = this.createContext();
    const result = await action(ctx, ...args);

    // Update metadata
    this.meta.updatedAt = new Date();
    this.meta.version++;

    // Persist state after action
    await this.persist();

    return result;
  }

  /**
   * Broadcast event to subscribers
   */
  broadcast(event: string, data: any): void {
    for (const subscriber of this.subscribers) {
      try {
        subscriber(event, data);
      } catch (error) {
        console.error("Error notifying subscriber:", error);
      }
    }
  }

  /**
   * Persist state to storage
   */
  async persist(): Promise<void> {
    const ctx = this.createContext();

    // Call onBeforeSave hook
    if (this.definition.onBeforeSave) {
      await this.definition.onBeforeSave(ctx);
    }

    // Save to driver
    await this.driver.save(
      this.constructor.name,
      this.key,
      this.state,
      this.meta
    );

    // Call onAfterSave hook
    if (this.definition.onAfterSave) {
      await this.definition.onAfterSave(ctx);
    }
  }

  /**
   * Start auto-persist timer
   */
  private startAutoPersist(): void {
    const interval = this.definition.persistInterval!;

    this.persistTimer = setInterval(() => {
      this.persist().catch(console.error);
    }, interval);
  }

  /**
   * Terminate actor
   */
  async terminate(): Promise<void> {
    // Stop auto-persist
    if (this.persistTimer) {
      clearInterval(this.persistTimer);
    }

    // Call onTerminate hook
    if (this.definition.onTerminate) {
      const ctx = this.createContext();
      await this.definition.onTerminate(ctx);
    }

    // Remove from registry
    // Cleanup resources
  }
}
```

## State Management

### State Structure

```typescript
// packages/core/src/actor/state.ts

/**
 * State wrapper with change tracking
 */
export class StateContainer<T extends Record<string, any>> {
  private state: T;
  private changes: Set<keyof T> = new Set();
  private listeners: Set<() => void> = new Set();

  constructor(initialState: T) {
    this.state = new Proxy(initialState, {
      get: (target, prop: keyof T) => {
        return target[prop];
      },
      set: (target, prop: keyof T, value) => {
        // Track changes
        this.changes.add(prop);

        // Update value
        target[prop] = value;

        // Notify listeners
        this.notifyListeners();

        return true;
      },
    });
  }

  /**
   * Get state
   */
  get(): T {
    return { ...this.state };
  }

  /**
   * Get changed keys
   */
  getChanges(): (keyof T)[] {
    return Array.from(this.changes);
  }

  /**
   * Mark state as saved (clear changes)
   */
  markSaved(): void {
    this.changes.clear();
  }

  /**
   * Subscribe to state changes
   */
  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notifyListeners(): void {
    for (const listener of this.listeners) {
      listener();
    }
  }

  /**
   * Serialize state for storage
   */
  toJSON(): any {
    return {
      data: this.state,
      changes: Array.from(this.changes),
      timestamp: Date.now(),
    };
  }

  /**
   * Deserialize state from storage
   */
  static fromJSON<T>(json: string): StateContainer<T> {
    const parsed = JSON.parse(json);
    return new StateContainer(parsed.data);
  }
}
```

### Complex State Patterns

```typescript
// Nested state with deep change tracking

interface AppState {
  user: {
    id: string;
    profile: {
      name: string;
      email: string;
      preferences: {
        theme: "light" | "dark";
        language: string;
      };
    };
  };
  posts: Map<string, Post>;
  metadata: {
    lastLogin: Date;
    loginCount: number;
  };
}

const appActor = actor({
  state: (): AppState => ({
    user: {
      id: "",
      profile: {
        name: "",
        email: "",
        preferences: {
          theme: "light",
          language: "en",
        },
      },
    },
    posts: new Map(),
    metadata: {
      lastLogin: new Date(),
      loginCount: 0,
    },
  }),

  actions: {
    updateProfile: (ctx, updates: Partial<AppState["user"]["profile"]>) => {
      // Deep merge profile
      ctx.state.user.profile = {
        ...ctx.state.user.profile,
        ...updates,
      };
    },

    addPost: (ctx, post: Post) => {
      ctx.state.posts.set(post.id, post);
    },

    incrementLogin: (ctx) => {
      ctx.state.metadata.loginCount++;
      ctx.state.metadata.lastLogin = new Date();
    },
  },
});
```

### State Serialization

```typescript
// packages/core/src/actor/serialization.ts

import { serialize, deserialize } from "superjson";

/**
 * Serialize state for storage
 * Handles Date, Map, Set, BigInt, etc.
 */
export function serializeState(state: any): string {
  const { json, meta } = serialize(state);
  return JSON.stringify({
    json,
    meta,
    version: 1,
  });
}

/**
 * Deserialize state from storage
 */
export function deserializeState(serialized: string): any {
  const { json, meta } = JSON.parse(serialized);
  return deserialize({ json, meta });
}

// Example with custom serializers

interface CustomState {
  date: Date;
  map: Map<string, number>;
  set: Set<string>;
  buffer: ArrayBuffer;
}

const customActor = actor({
  state: (): CustomState => ({
    date: new Date(),
    map: new Map(),
    set: new Set(),
    buffer: new ArrayBuffer(0),
  }),

  actions: {
    update: (ctx, updates: Partial<CustomState>) => {
      Object.assign(ctx.state, updates);
    },
  },
});
```

## Persistence Layer

### Driver Interface

```typescript
// packages/core/src/drivers/driver.ts

import { ActorMeta } from "../actor";

/**
 * Storage driver interface
 */
export interface Driver {
  /**
   * Load actor state by key
   */
  load<T>(type: string, key: string): Promise<T | null>;

  /**
   * Save actor state
   */
  save<T>(
    type: string,
    key: string,
    state: T,
    meta: ActorMeta
  ): Promise<void>;

  /**
   * Delete actor state
   */
  delete(type: string, key: string): Promise<void>;

  /**
   * Check if actor exists
   */
  exists(type: string, key: string): Promise<boolean>;

  /**
   * List all actors of a type
   */
  list(type: string): Promise<string[]>;
}

/**
 * Driver options
 */
export interface DriverOptions {
  /**
   * Serialization function
   */
  serialize?: (state: any) => string;

  /**
   * Deserialization function
   */
  deserialize?: (data: string) => any;

  /**
   * Retry configuration
   */
  retry?: {
    maxAttempts: number;
    delay: number;
    backoff: number;
  };
}
```

### File System Driver

```typescript
// packages/core/src/drivers/file-system.ts

import { Driver } from "./driver";
import * as fs from "fs/promises";
import * as path from "path";

export interface FileSystemDriverOptions {
  storagePath: string;
  fileExtension?: string;
  mkdirIfNotExists?: boolean;
}

export class FileSystemDriver implements Driver {
  private storagePath: string;
  private fileExtension: string;

  constructor(options: FileSystemDriverOptions) {
    this.storagePath = options.storagePath;
    this.fileExtension = options.fileExtension || ".json";
  }

  private getFilePath(type: string, key: string): string {
    // Sanitize key to prevent path traversal
    const sanitizedKey = key.replace(/[^a-zA-Z0-9_-]/g, "_");
    return path.join(this.storagePath, `${type}_${sanitizedKey}${this.fileExtension}`);
  }

  async load<T>(type: string, key: string): Promise<T | null> {
    const filePath = this.getFilePath(type, key);

    try {
      const content = await fs.readFile(filePath, "utf-8");
      return JSON.parse(content);
    } catch (error: any) {
      if (error.code === "ENOENT") {
        return null;
      }
      throw error;
    }
  }

  async save<T>(type: string, key: string, state: T): Promise<void> {
    const filePath = this.getFilePath(type, key);

    // Ensure directory exists
    await fs.mkdir(path.dirname(filePath), { recursive: true });

    // Write state atomically (write to temp, then rename)
    const tempPath = `${filePath}.tmp`;
    await fs.writeFile(tempPath, JSON.stringify(state, null, 2), "utf-8");
    await fs.rename(tempPath, filePath);
  }

  async delete(type: string, key: string): Promise<void> {
    const filePath = this.getFilePath(type, key);
    await fs.unlink(filePath).catch(() => {}); // Ignore if doesn't exist
  }

  async exists(type: string, key: string): Promise<boolean> {
    const filePath = this.getFilePath(type, key);
    try {
      await fs.access(filePath);
      return true;
    } catch {
      return false;
    }
  }

  async list(type: string): Promise<string[]> {
    const files = await fs.readdir(this.storagePath);
    return files
      .filter((f) => f.startsWith(`${type}_`))
      .map((f) => f.replace(`${type}_`, "").replace(this.fileExtension, ""));
  }
}
```

### Postgres Driver

```typescript
// packages/core/src/drivers/postgres.ts

import { Driver } from "./driver";
import { Pool } from "pg";

export interface PostgresDriverOptions {
  connectionString: string;
  tableName?: string;
  poolSize?: number;
  persistInterval?: number;
}

export class PostgresDriver implements Driver {
  private pool: Pool;
  private tableName: string;

  constructor(options: PostgresDriverOptions) {
    this.pool = new Pool({
      connectionString: options.connectionString,
      max: options.poolSize || 20,
    });

    this.tableName = options.tableName || "rivet_actors";

    // Initialize table
    this.initialize();
  }

  private async initialize(): Promise<void> {
    await this.pool.query(`
      CREATE TABLE IF NOT EXISTS ${this.tableName} (
        actor_type TEXT NOT NULL,
        actor_key TEXT NOT NULL,
        state JSONB NOT NULL,
        meta JSONB NOT NULL,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW(),
        PRIMARY KEY (actor_type, actor_key)
      );

      CREATE INDEX IF NOT EXISTS idx_actor_type 
      ON ${this.tableName} (actor_type);

      CREATE INDEX IF NOT EXISTS idx_updated_at 
      ON ${this.tableName} (updated_at);
    `);
  }

  async load<T>(type: string, key: string): Promise<T | null> {
    const result = await this.pool.query(
      `SELECT state FROM ${this.tableName} 
       WHERE actor_type = $1 AND actor_key = $2`,
      [type, key]
    );

    if (result.rows.length === 0) {
      return null;
    }

    return result.rows[0].state as T;
  }

  async save<T>(type: string, key: string, state: T, meta: any): Promise<void> {
    await this.pool.query(
      `INSERT INTO ${this.tableName} (actor_type, actor_key, state, meta, updated_at)
       VALUES ($1, $2, $3, $4, NOW())
       ON CONFLICT (actor_type, actor_key) 
       DO UPDATE SET state = $3, meta = $4, updated_at = NOW()`,
      [type, key, JSON.stringify(state), JSON.stringify(meta)]
    );
  }

  async delete(type: string, key: string): Promise<void> {
    await this.pool.query(
      `DELETE FROM ${this.tableName} 
       WHERE actor_type = $1 AND actor_key = $2`,
      [type, key]
    );
  }

  async exists(type: string, key: string): Promise<boolean> {
    const result = await this.pool.query(
      `SELECT 1 FROM ${this.tableName} 
       WHERE actor_type = $1 AND actor_key = $2`,
      [type, key]
    );

    return result.rows.length > 0;
  }

  async list(type: string): Promise<string[]> {
    const result = await this.pool.query(
      `SELECT actor_key FROM ${this.tableName} 
       WHERE actor_type = $1 
       ORDER BY updated_at DESC`,
      [type]
    );

    return result.rows.map((r) => r.actor_key);
  }

  async close(): Promise<void> {
    await this.pool.end();
  }
}
```

## Lifecycle Hooks

### Hook Execution Order

```typescript
// Actor lifecycle sequence

const lifecycleActor = actor({
  state: { value: 0 },

  // 1. Called when actor is first created/loaded
  onInit: async (ctx) => {
    console.log("1. onInit called");
    // Setup timers, subscriptions, etc.
  },

  // 2. Called before each action
  onBeforeAction: async (ctx, actionName) => {
    console.log(`2. onBeforeAction: ${actionName}`);
    // Logging, validation, etc.
  },

  // 3. Called after each action (before persist)
  onAfterAction: async (ctx, actionName, result) => {
    console.log(`3. onAfterAction: ${actionName}, result: ${result}`);
    // Metrics, caching, etc.
  },

  // 4. Called before state is persisted
  onBeforeSave: async (ctx) => {
    console.log("4. onBeforeSave called");
    // Compression, encryption, etc.
  },

  // 5. Called after state is persisted
  onAfterSave: async (ctx) => {
    console.log("5. onAfterSave called");
    // Cleanup, notifications, etc.
  },

  // 6. Called when actor is terminated
  onTerminate: async (ctx) => {
    console.log("6. onTerminate called");
    // Cleanup resources, close connections, etc.
  },

  actions: {
    update: (ctx, value: number) => {
      ctx.state.value = value;
      return value;
    },
  },
});
```

### Hook Patterns

```typescript
// Logging hook

function withLogging<T extends ActorDefinition<any>>(definition: T): T {
  return {
    ...definition,
    onBeforeAction: async (ctx, actionName) => {
      console.log(`[Actor:${ctx.key}] Action: ${actionName}`);
    },
    onAfterAction: async (ctx, actionName, result) => {
      console.log(`[Actor:${ctx.key}] Action: ${actionName} completed`);
    },
  };
}

// Metrics hook

function withMetrics<T extends ActorDefinition<any>>(definition: T): T {
  return {
    ...definition,
    onBeforeAction: async (ctx, actionName) => {
      ctx.meta.metricsStart = Date.now();
    },
    onAfterAction: async (ctx, actionName, result) => {
      const duration = Date.now() - ctx.meta.metricsStart;
      metrics.histogram("actor.action.duration", duration, {
        action: actionName,
        type: ctx.type,
      });
    },
  };
}

// Usage

const trackedActor = actor(
  withLogging(
    withMetrics({
      state: { count: 0 },
      actions: {
        increment: (ctx) => {
          ctx.state.count++;
          return ctx.state.count;
        },
      },
    })
  )
);
```

## Memory Management

### Actor Eviction

```typescript
// packages/core/src/registry/eviction.ts

export interface EvictionPolicy {
  /**
   * Maximum number of actors in memory
   */
  maxActors: number;

  /**
   * Idle timeout (ms)
   */
  idleTimeout: number;

  /**
   * Check interval (ms)
   */
  checkInterval: number;
}

export class EvictionManager {
  private accessTimes: Map<string, number> = new Map();
  private policy: EvictionPolicy;
  private timer: NodeJS.Timeout;

  constructor(
    private registry: Registry,
    policy: EvictionPolicy
  ) {
    this.policy = policy;
    this.startEvictionCycle();
  }

  /**
   * Record actor access
   */
  recordAccess(actorId: string): void {
    this.accessTimes.set(actorId, Date.now());
  }

  /**
   * Start periodic eviction
   */
  private startEvictionCycle(): void {
    this.timer = setInterval(() => {
      this.evictIdleActors();
    }, this.policy.checkInterval);
  }

  /**
   * Evict idle actors
   */
  private evictIdleActors(): void {
    const now = Date.now();
    const actors = Array.from(this.accessTimes.entries());

    // Sort by access time (LRU)
    actors.sort((a, b) => a[1] - b[1]);

    const currentCount = this.registry.actorCount;

    // Evict if over capacity
    for (const [actorId, lastAccess] of actors) {
      if (currentCount <= this.policy.maxActors) break;

      const idleTime = now - lastAccess;
      if (idleTime > this.policy.idleTimeout) {
        this.registry.evict(actorId);
        this.accessTimes.delete(actorId);
      }
    }
  }

  /**
   * Stop eviction
   */
  stop(): void {
    clearInterval(this.timer);
  }
}
```

## Conclusion

Actor lifecycle management in RivetKit provides:

1. **Automatic Initialization**: onInit hook for setup
2. **Change Tracking**: Proxy-based state observation
3. **Persistence**: Multiple driver options (FS, Postgres, Redis)
4. **Lifecycle Hooks**: onBeforeSave, onAfterSave, onTerminate
5. **Memory Management**: LRU eviction, idle timeout
6. **Serialization**: Handle complex types (Date, Map, Set)
