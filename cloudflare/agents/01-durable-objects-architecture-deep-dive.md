# Cloudflare Agents: Durable Objects Architecture Deep Dive

**Last Updated:** 2026-04-05

---

## Table of Contents

1. [Overview](#overview)
2. [Durable Objects Primer](#durable-objects-primer)
3. [Agent Class Architecture](#agent-class-architecture)
4. [Request Routing](#request-routing)
5. [State Persistence](#state-persistence)
6. [Connection Management](#connection-management)
7. [Scaling Characteristics](#scaling-characteristics)
8. [Failure Modes](#failure-modes)

---

## Overview

Cloudflare Agents is built on **Durable Objects (DOs)** - Cloudflare's stateful serverless primitive. This deep-dive examines the architecture layer-by-layer, from the DO abstraction up through the Agent class.

```
┌──────────────────────────────────────────────────────────────┐
│                      Application Layer                        │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Agent Class (agents SDK)                   │  │
│  │  - @callable() methods                                  │  │
│  │  - setState() / state                                   │  │
│  │  - lifecycle hooks (onInit, onStateUpdate, onHibernate) │  │
│  └────────────────────────────────────────────────────────┘  │
│                              ↑                                │
│  ┌────────────────────────────────────────────────────────┐  │
│  │           PartyServer Base Class (partyserver)          │  │
│  │  - WebSocket connection handling                        │  │
│  │  - Connection lifecycle                                 │  │
│  │  - Message routing                                      │  │
│  └────────────────────────────────────────────────────────┘  │
│                              ↑                                │
│  ┌────────────────────────────────────────────────────────┐  │
│  │          Durable Object Base Class (Workers)            │  │
│  │  - fetch(request) handler                               │  │
│  │  - storage API (get/put/list/delete)                    │  │
│  │  - alarm() scheduling                                   │  │
│  │  - blockConcurrencyWhile()                              │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

---

## Durable Objects Primer

### What Are Durable Objects?

Durable Objects are **single-tenant, stateful serverless containers** that:

1. **Run globally** - Deployed to the location nearest the first request
2. **Maintain state** - In-memory state + persistent storage
3. **Handle concurrency** - Sequential or parallel request handling
4. **Support WebSockets** - Native WebSocket connections
5. **Schedule tasks** - `alarm()` method for background work

### DO Lifecycle

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Cold      │ ──► │   Active    │ ──► │ Hibernating │
│   Start     │     │   Running   │     │   Idle      │
└─────────────┘     └─────────────┘     └─────────────┘
      ↑                   │                   │
      │                   │                   │
      └───────────────────┴───────────────────┘
                    Wake on Request
```

### Storage API

```typescript
// Durable Object storage
const storage = this.ctx.storage;

// Put (persist)
await storage.put("key", value);

// Get (retrieve)
const value = await storage.get("key");

// Delete (remove)
await storage.delete("key");

// List (iterate)
const cursor = storage.list();
for await (const key of cursor) {
  console.log(key);
}
```

---

## Agent Class Architecture

### Base Class Structure

The Agent class extends PartyServer's `Server` class, which wraps Durable Objects:

```typescript
// Simplified Agent class structure
import { Server, type Connection } from "partyserver";

export class Agent<Env, State> extends Server<Env> {
  // Initial state when agent is created
  initialState: State;
  
  // Current state (automatically persisted)
  state: State;
  
  // Durable Object state
  ctx: DurableObjectState;
  
  // Environment bindings
  env: Env;
  
  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    this.ctx = ctx;
    this.env = env;
    this.state = this.initialState;
  }
  
  // Lifecycle hooks
  async onInit(): Promise<void> {
    // Called when agent is first instantiated
  }
  
  async onStateUpdate(newState: State): Promise<void> {
    // Called after setState() completes
  }
  
  async onHibernate(): Promise<void> {
    // Called when agent is about to hibernate
  }
  
  // State management
  setState(newState: Partial<State>): void {
    this.state = { ...this.state, ...newState };
    this.ctx.storage.put("state", this.state);
    this.broadcastState();
  }
  
  // Connection handling
  async onConnect(conn: Connection, req: Request): Promise<void> {
    // Called when client connects
  }
  
  async onDisconnect(conn: Connection): Promise<void> {
    // Called when client disconnects
  }
  
  async onMessage(conn: Connection, message: WSMessage): Promise<void> {
    // Called when WebSocket message received
  }
}
```

### Callable Method Decorator

The `@callable()` decorator exposes methods as RPC endpoints:

```typescript
// Source: packages/agents/src/index.ts (simplified)

const callableMetadata = new WeakMap<Function, CallableMetadata>();

export function callable(metadata: CallableMetadata = {}) {
  return function target(descriptor: PropertyDescriptor) {
    // Store metadata for later retrieval
    callableMetadata.set(descriptor.value, metadata);
    
    // Wrap method to handle serialization
    const originalMethod = descriptor.value;
    descriptor.value = async function(...args: unknown[]) {
      // Method implementation
      return originalMethod.apply(this, args);
    };
    
    return descriptor;
  };
}

// Usage
export class CounterAgent extends Agent<Env, CounterState> {
  @callable({ description: "Increment counter" })
  increment(amount: number = 1): number {
    this.setState({ count: this.state.count + amount });
    return this.state.count;
  }
}
```

### Message Type System

The SDK uses a typed message system for client-server communication:

```typescript
// Source: packages/agents/src/types.ts

export enum MessageType {
  CF_AGENT_MCP_SERVERS = "cf_agent_mcp_servers",
  CF_MCP_AGENT_EVENT = "cf_mcp_agent_event",
  CF_AGENT_STATE = "cf_agent_state",
  CF_AGENT_STATE_ERROR = "cf_agent_state_error",
  CF_AGENT_IDENTITY = "cf_agent_identity",
  RPC = "rpc"
}

export type RPCRequest = {
  type: MessageType.RPC;
  id: string;
  method: string;
  args: unknown[];
};

export type RPCResponse = {
  type: MessageType.RPC;
  id: string;
} & (
  | { success: true; result: unknown; done?: false }
  | { success: true; result: unknown; done: true }
  | { success: false; error: string }
);
```

---

## Request Routing

### routeAgentRequest Function

The SDK provides automatic routing for agent requests:

```typescript
// Source: packages/agents/src/index.ts (simplified)

export async function routeAgentRequest(
  request: Request,
  env: Env
): Promise<Response> {
  // Parse URL to extract agent name and instance
  const url = new URL(request.url);
  const pathParts = url.pathname.split("/");
  
  // Expected format: /agents/:agentName/:instanceName
  const agentName = pathParts[2];  // e.g., "counter-agent"
  const instanceName = pathParts[3];  // e.g., "room-1"
  
  // Get Durable Object ID
  const id = env[agentName.toUpperCase()].idFromName(instanceName);
  const stub = env[agentName.toUpperCase()].get(id);
  
  // Forward request to DO
  return stub.fetch(request);
}
```

### URL Structure

```
/agents/:agent/:name/:path?

Examples:
/agents/counter/my-counter
/agents/chat/room-1/messages
/agents/user/profile/settings
```

### Custom Routing with basePath

For session-based routing:

```typescript
// Client
const agent = useAgent({
  agent: "UserAgent",
  basePath: "user"  // Connects to /user instead of /agents/user-agent
});

// Server - manual routing
export default {
  async fetch(request: Request, env: Env) {
    // Get user from session
    const userId = getSessionUserId(request);
    
    // Route to user-specific agent
    const id = env.USER_AGENT.idFromName(userId);
    const stub = env.USER_AGENT.get(id);
    
    return stub.fetch(request);
  }
};
```

---

## State Persistence

### setState() Mechanics

When `setState()` is called:

```typescript
// Source: packages/agents/src/index.ts

setState(newState: Partial<State>) {
  // 1. Merge new state with existing
  const mergedState = { ...this.state, ...newState };
  
  // 2. Update in-memory state
  this.state = mergedState;
  
  // 3. Persist to DO storage
  this.ctx.storage.put("state", mergedState);
  
  // 4. Broadcast to all connected clients
  this.broadcast({
    type: MessageType.CF_AGENT_STATE,
    state: mergedState
  });
  
  // 5. Call lifecycle hook
  this.onStateUpdate(mergedState);
}
```

### State Serialization

State must be **JSON-serializable**:

```typescript
type ValidState = {
  // Primitives
  count: number;
  name: string;
  active: boolean;
  
  // Arrays (of serializable items)
  items: string[];
  history: Array<{ timestamp: number; action: string }>;
  
  // Objects
  user: { id: string; email: string };
  settings: Record<string, unknown>;
  
  // Null
  lastError: string | null;
};

// Invalid - functions
type InvalidState1 = {
  callback: () => void;  // ❌ Functions not serializable
};

// Invalid - Date (serialized as string, loses methods)
type InvalidState2 = {
  createdAt: Date;  // ⚠️ Becomes ISO string
};

// Invalid - Class instances
type InvalidState3 = {
  user: UserClass;  // ❌ Class instances lose methods
};
```

### Storage Keys

The SDK uses specific storage keys:

```typescript
// Internal storage structure
const STORAGE_KEYS = {
  STATE: "state",
  MESSAGES: "messages",
  SCHEDULES: "schedules",
  WORKFLOWS: "workflows"
};

// Persisted on setState()
await this.ctx.storage.put("state", this.state);

// Persisted on schedule()
await this.ctx.storage.put(`schedule:${scheduleId}`, scheduleData);

// Persisted on workflow start
await this.ctx.storage.put(`workflow:${workflowId}`, workflowData);
```

---

## Connection Management

### WebSocket Handling

Agents inherit WebSocket handling from PartyServer:

```typescript
// Simplified connection handling
import { Server, type Connection } from "partyserver";

export class Agent<Env, State> extends Server<Env> {
  // Track connected clients
  private clients: Set<Connection> = new Set();
  
  // Called on WebSocket upgrade
  async onConnect(conn: Connection, request: Request) {
    this.clients.add(conn);
    
    // Send current state to new client
    conn.send(JSON.stringify({
      type: MessageType.CF_AGENT_STATE,
      state: this.state
    }));
    
    // Send identity
    conn.send(JSON.stringify({
      type: MessageType.CF_AGENT_IDENTITY,
      name: this.name,
      agent: this.agent
    }));
  }
  
  // Called on WebSocket disconnect
  async onDisconnect(conn: Connection) {
    this.clients.delete(conn);
  }
  
  // Called on WebSocket message
  async onMessage(conn: Connection, message: string | Buffer) {
    const parsed = JSON.parse(message.toString());
    
    if (parsed.type === MessageType.RPC) {
      // Handle RPC call
      await this.handleRPC(conn, parsed);
    }
  }
  
  // Broadcast to all clients
  broadcast(data: unknown) {
    const message = JSON.stringify(data);
    for (const conn of this.clients) {
      conn.send(message);
    }
  }
}
```

### Client Identity

The server sends identity on connect:

```typescript
// Client receives identity message
{
  type: "cf_agent_identity",
  name: "my-counter",     // Instance name
  agent: "counter-agent"  // Agent class (kebab-case)
}
```

---

## Scaling Characteristics

### Horizontal Scaling

Each agent instance runs independently:

```
┌─────────────────────────────────────────────────────────┐
│               Cloudflare Global Network                  │
│                                                          │
│  Location: SFO                                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │ Counter:1   │  │ Counter:2   │  │ Chat:room-1 │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
│                                                          │
│  Location: LHR                                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │ Counter:3   │  │ Counter:4   │  │ Chat:room-2 │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
│                                                          │
│  Location: NRT                                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │ Counter:5   │  │ Counter:6   │  │ Chat:room-3 │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
└─────────────────────────────────────────────────────────┘
```

### Instance Affinity

Requests for the same agent instance route to the same DO:

```typescript
// Same instance name = same DO
useAgent({ agent: "CounterAgent", name: "my-counter" })  // Always routes to DO "my-counter"
useAgent({ agent: "CounterAgent", name: "my-counter" })  // Same DO (different client)
useAgent({ agent: "CounterAgent", name: "other-counter" }) // Different DO
```

### Memory Limits

Durable Objects have memory limits:

| Limit | Value |
|-------|-------|
| Memory per DO | 128 MB |
| CPU time per request | 30 seconds |
| Storage writes per second | 1000 |
| Storage reads per second | 1000 |

---

## Failure Modes

### Connection Failures

```typescript
// Client handles connection failures
const agent = useAgent({
  agent: "CounterAgent",
  onStateUpdateError: (error) => {
    console.error("State update failed:", error);
    // Possible errors:
    // - "Connection is readonly" - server rejected state update
    // - "Connection closed" - WebSocket disconnected
  }
});
```

### DO Restart

Durable Objects can restart at any time:

```typescript
export class CounterAgent extends Agent<Env, CounterState> {
  initialState: CounterState = { count: 0 };
  
  async onInit() {
    // State is automatically restored from storage
    console.log("Agent started, state:", this.state);
  }
  
  async onHibernate() {
    // Called before hibernation
    // Good place for cleanup
    console.log("Agent hibernating");
  }
}
```

### Storage Failures

```typescript
try {
  await this.ctx.storage.put("key", value);
} catch (error) {
  // Handle storage failure
  // Possible causes:
  // - Storage quota exceeded
  // - Temporary unavailability
  // - Serialization error
}
```

---

## Related Documents

- [State Synchronization Deep Dive](./02-state-sync-deep-dive.md)
- [Callable Methods & RPC Deep Dive](./03-callable-methods-deep-dive.md)
- [Real-time Communication Deep Dive](./04-realtime-websockets-deep-dive.md)
- [Rust Revision](./rust-revision.md)
