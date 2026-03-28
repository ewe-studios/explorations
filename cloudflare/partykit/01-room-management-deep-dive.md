---
title: "Room Management Deep Dive: Parties, Lifecycle, and Connections"
subtitle: "Complete guide to Durable Objects as rooms, lifecycle hooks, and connection management in PartyServer"
based_on: "PartyServer packages/partyserver/src/index.ts, connection.ts"
---

# Room Management Deep Dive

## Table of Contents

1. [Durable Objects as Rooms](#1-durable-objects-as-rooms)
2. [Server Lifecycle](#2-server-lifecycle)
3. [Connection Management](#3-connection-management)
4. [Routing and Naming](#4-routing-and-naming)
5. [Hibernation Deep Dive](#5-hibernation-deep-dive)
6. [Advanced Patterns](#6-advanced-patterns)

---

## 1. Durable Objects as Rooms

### 1.1 The Room Abstraction

In PartyServer, a "room" is a Durable Object instance that:
- Has a unique name within its namespace
- Maintains isolated state (SQLite database)
- Manages WebSocket connections
- Handles message routing between clients

```typescript
// URL Pattern: /parties/:party/:room
// Example: https://myapp.com/parties/chat/lobby

// "chat" = party (Durable Object namespace/class)
// "lobby" = room (specific DO instance name)
```

### 1.2 Server Class Hierarchy

```typescript
// PartyServer extends DurableObject
export class Server<Env, Props> extends DurableObject<Env> {
  // Core properties
  name: string;           // Room name (write-once)
  ctx: DurableObjectState; // DO state (storage, alarms)
  env: Env;               // Environment bindings

  // Connection management
  #connectionManager: ConnectionManager;

  // Lifecycle hooks (override in subclasses)
  onStart(props?: Props): void | Promise<void>
  onConnect(connection, ctx): void | Promise<void>
  onMessage(connection, message): void | Promise<void>
  onClose(connection, code, reason, wasClean): void | Promise<void>
  onError(connection, error): void | Promise<void>
  onRequest(request): Response | Promise<Response>
  onAlarm(): void | Promise<void>
}
```

### 1.3 Static Options

```typescript
export class MyServer extends Server {
  static options = {
    hibernate: true  // Enable DO hibernation
  };

  // ... rest of implementation
}
```

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `hibernate` | `false` | Enable Durable Object hibernation |

### 1.4 Room Instance Management

```typescript
// Get a room by name
const room = await getServerByName(
  env.MyServer,     // DO namespace
  "room-name",     // Room identifier
  {
    locationHint: "eu",    // Optional: geographic location
    jurisdiction: "eu",    // Optional: data jurisdiction
    props: { maxUsers: 10 } // Optional: initialization props
  }
);

// The returned stub is a reference to the DO
// Use stub.fetch() to make requests to the room
```

---

## 2. Server Lifecycle

### 2.1 Lifecycle States

```
┌─────────────────────────────────────────────────────────┐
│                  Server Lifecycle                        │
│                                                          │
│  [*] ──> Zero ──> Starting ──> Started ──> [*]          │
│           ▲         │          │                         │
│           │         │          │                         │
│           │         ▼          │                         │
│           │      Error ────────┘                         │
│           │                                              │
│           └────── Hibernating ──> [*]                    │
└─────────────────────────────────────────────────────────┘
```

### 2.2 State Transitions

```typescript
#status: "zero" | "starting" | "started" = "zero";

async #ensureInitialized(): Promise<void> {
  if (this.#status === "started") return;

  await this.ctx.blockConcurrencyWhile(async () => {
    this.#status = "starting";
    try {
      await this.onStart(this.#_props);
      this.#status = "started";
    } catch (e) {
      this.#status = "zero";
      throw e;
    }
  });
}
```

### 2.3 onStart Hook

Called when the server starts for the first time or wakes from hibernation:

```typescript
export class ChatServer extends Server {
  async onStart(props?: { maxUsers?: number }) {
    // Initialize SQLite schema
    this.ctx.storage.sql.exec(`
      CREATE TABLE IF NOT EXISTS messages (
        id TEXT PRIMARY KEY,
        content TEXT,
        sender_id TEXT,
        created_at INTEGER
      )
    `);

    // Load configuration
    this.maxUsers = props?.maxUsers ?? 100;

    // Load initial state from external storage
    const snapshot = await this.ctx.storage.get("snapshot");
    if (snapshot) {
      this.restoreState(snapshot);
    }

    console.log(`Room ${this.name} started with max ${this.maxUsers} users`);
  }
}
```

### 2.4 onConnect Hook

Called when a new WebSocket connection is established:

```typescript
onConnect(connection: Connection, ctx: ConnectionContext) {
  // ctx.request contains the original HTTP request
  const userAgent = ctx.request.headers.get("User-Agent");
  const authHeader = ctx.request.headers.get("Authorization");

  // Validate authentication
  const user = this.validateAuth(authHeader);
  if (!user) {
    connection.close(4001, "Unauthorized");
    return;
  }

  // Set connection state
  connection.setState({
    userId: user.id,
    username: user.name,
    joinedAt: Date.now(),
    status: "active"
  });

  // Assign tags for filtering
  // (handled by getConnectionTags)

  // Notify other users
  this.broadcast(JSON.stringify({
    type: "user_joined",
    userId: user.id,
    username: user.name
  }), [connection.id]);

  // Send welcome message
  connection.send(JSON.stringify({
    type: "welcome",
    roomId: this.name,
    userId: user.id
  }));
}
```

### 2.5 onMessage Hook

Called when a message is received from a connection:

```typescript
async onMessage(connection: Connection, message: WSMessage) {
  const data = JSON.parse(message as string);

  switch (data.type) {
    case "chat_message": {
      // Validate message
      if (!data.content || data.content.length > 1000) {
        connection.send(JSON.stringify({
          type: "error",
          message: "Invalid message content"
        }));
        return;
      }

      // Store in database
      const id = crypto.randomUUID();
      this.ctx.storage.sql.exec(
        "INSERT INTO messages (id, content, sender_id, created_at) VALUES (?, ?, ?, ?)",
        id, data.content, connection.state.userId, Date.now()
      );

      // Broadcast to all (including sender)
      this.broadcast(JSON.stringify({
        type: "message",
        id,
        content: data.content,
        senderId: connection.state.userId,
        senderName: connection.state.username,
        timestamp: Date.now()
      }));
      break;
    }

    case "typing": {
      // Broadcast typing indicator (ephemeral)
      this.broadcast(JSON.stringify({
        type: "typing",
        userId: connection.state.userId,
        username: connection.state.username
      }), [connection.id]);
      break;
    }
  }
}
```

### 2.6 onClose Hook

Called when a connection closes:

```typescript
async onClose(connection: Connection, code: number, reason: string, wasClean: boolean) {
  console.log(`Connection ${connection.id} closed: ${reason}`);

  // Clean up connection-specific resources
  const state = connection.state as UserState;

  // Remove from any internal tracking
  this.cleanupUser(state.userId);

  // Notify other users
  this.broadcast(JSON.stringify({
    type: "user_left",
    userId: state.userId,
    username: state.username,
    reason
  }));

  // Set alarm for cleanup if room is empty
  if (this.getConnections().length === 0) {
    // Set alarm to cleanup room after 5 minutes of inactivity
    this.ctx.storage.setAlarm(Date.now() + 5 * 60 * 1000);
  }
}
```

### 2.7 onAlarm Hook

Called when a DO alarm fires:

```typescript
async onAlarm() {
  // Cleanup after period of inactivity
  const connectionCount = this.getConnections().length;

  if (connectionCount === 0) {
    // Save final snapshot
    const snapshot = this.createSnapshot();
    await this.ctx.storage.put("snapshot", snapshot);

    console.log(`Room ${this.name} cleaned up after inactivity`);

    // Note: We can't "delete" the DO, but we can clear state
    // The DO will be evicted by the platform when idle
  }
}
```

### 2.8 onRequest Hook

Called for HTTP requests (non-WebSocket):

```typescript
async onRequest(request: Request): Promise<Response> {
  const url = new URL(request.url);

  switch (url.pathname) {
    case "/api/messages": {
      if (request.method === "GET") {
        // Get recent messages
        const messages = this.ctx.storage.sql.exec(
          "SELECT * FROM messages ORDER BY created_at DESC LIMIT 50"
        ).raw();
        return Response.json(messages);
      }
      break;
    }

    case "/api/users": {
      if (request.method === "GET") {
        // Get connected users
        const users = Array.from(this.getConnections())
          .map(c => c.state);
        return Response.json(users);
      }
      break;
    }
  }

  return new Response("Not Found", { status: 404 });
}
```

---

## 3. Connection Management

### 3.1 Connection Type Definition

```typescript
export type Connection<TState = unknown> = WebSocket & {
  id: string;                              // Unique connection identifier
  uri: string | null;                      // Original WebSocket URL
  state: ConnectionState<TState>;          // Arbitrary state (up to 2KB)
  setState(state: TState | ConnectionSetStateFn<TState>): ConnectionState<TState>;
  tags: readonly string[];                 // Filter tags
  server: string;                          // Server name

  // Deprecated (use state/setState)
  serializeAttachment<T>(attachment: T): void;
  deserializeAttachment<T>(): T | null;
};
```

### 3.2 Connection Managers

PartyServer provides two connection manager implementations:

```typescript
interface ConnectionManager {
  getCount(): number;
  getConnection<TState>(id: string): Connection<TState> | undefined;
  getConnections<TState>(tag?: string): IterableIterator<Connection<TState>>;
  accept(connection: Connection, options: { tags: string[] }): Connection;
}
```

### 3.3 In-Memory Connection Manager

Used when hibernation is disabled:

```typescript
export class InMemoryConnectionManager implements ConnectionManager {
  #connections: Map<string, Connection> = new Map();
  tags: WeakMap<Connection, string[]> = new WeakMap();

  getCount() {
    return this.#connections.size;
  }

  getConnection<T>(id: string) {
    return this.#connections.get(id) as Connection<T> | undefined;
  }

  *getConnections<T>(tag?: string): IterableIterator<Connection<T>> {
    if (!tag) {
      yield* this.#connections.values();
      return;
    }

    for (const connection of this.#connections.values()) {
      const connectionTags = this.tags.get(connection) ?? [];
      if (connectionTags.includes(tag)) {
        yield connection as Connection<T>;
      }
    }
  }

  accept(connection: Connection, options: { tags: string[] }) {
    connection.accept();
    const tags = this.prepareTags(connection.id, options.tags);

    this.#connections.set(connection.id, connection);
    this.tags.set(connection, tags);

    // Auto-cleanup on close
    const removeConnection = () => {
      this.#connections.delete(connection.id);
      connection.removeEventListener("close", removeConnection);
      connection.removeEventListener("error", removeConnection);
    };
    connection.addEventListener("close", removeConnection);
    connection.addEventListener("error", removeConnection);

    return connection;
  }
}
```

### 3.4 Hibernating Connection Manager

Used when hibernation is enabled:

```typescript
export class HibernatingConnectionManager implements ConnectionManager {
  constructor(private controller: DurableObjectState) {}

  getCount() {
    let count = 0;
    for (const ws of this.controller.getWebSockets()) {
      if (isPartyServerWebSocket(ws)) count++;
    }
    return count;
  }

  getConnection<T>(id: string) {
    const sockets = this.controller.getWebSockets(id);
    const matching = sockets.filter(ws => {
      return tryGetPartyServerMeta(ws)?.id === id;
    });

    if (matching.length === 0) return undefined;
    if (matching.length === 1) {
      return createLazyConnection(matching[0]) as Connection<T>;
    }

    throw new Error(`More than one connection found for id ${id}`);
  }

  getConnections<T>(tag?: string) {
    return new HibernatingConnectionIterator<T>(this.controller, tag);
  }

  accept(connection: Connection, options: { tags: string[] }) {
    const tags = this.prepareTags(connection.id, options.tags);
    this.controller.acceptWebSocket(connection, tags);

    // Store metadata in WebSocket attachment
    connection.serializeAttachment({
      __pk: { id: connection.id, tags, uri: connection.uri ?? undefined },
      __user: null
    });

    return createLazyConnection(connection);
  }
}
```

### 3.5 Connection Tags

Tags enable filtering connections:

```typescript
export class MyServer extends Server {
  getConnectionTags(connection: Connection, ctx: ConnectionContext): string[] {
    const state = connection.state as UserState;
    return [
      connection.id,                    // Always include connection ID
      `user:${state.userId}`,           // User identifier
      `team:${state.team}`,             // Team membership
      `status:${state.status}`,         // Active, idle, away
      `room:${state.currentRoom}`       // Current sub-room
    ];
  }

  // Filter by tag
  getConnectionsByUser(userId: string) {
    return Array.from(this.getConnections(`user:${userId}`));
  }

  getConnectionsByTeam(team: string) {
    return Array.from(this.getConnections(`team:${team}`));
  }
}
```

**Tag Constraints:**

| Constraint | Value |
|------------|-------|
| Max tags per connection | 10 (including ID) |
| Max tag length | 256 characters |
| Tag format | Non-empty string |

### 3.6 Broadcast Patterns

```typescript
// Broadcast to all connections
this.broadcast(message);

// Broadcast to all except specific connections
this.broadcast(message, [connection1.id, connection2.id]);

// Broadcast to tagged connections
broadcastToTeam(team: string, message: string) {
  for (const conn of this.getConnections(`team:${team}`)) {
    conn.send(message);
  }
}

// Send to specific connection
sendToUser(userId: string, message: string) {
  const connections = this.getConnections(`user:${userId}`);
  for (const conn of connections) {
    conn.send(message);
  }
}
```

---

## 4. Routing and Naming

### 4.1 routePartykitRequest

The main routing function:

```typescript
async function routePartykitRequest<Env, T extends Server, Props>(
  req: Request,
  env: Env,
  options?: PartyServerOptions<Env, Props>
): Promise<Response | null>
```

### 4.2 URL Pattern Matching

```typescript
// Default pattern: /parties/:party/:room
// Custom prefix: /custom/:party/:room

const url = new URL(req.url);
const parts = url.pathname.split("/").filter(Boolean);

// Expected: [prefix, party, room, ...rest]
// Example: ["parties", "chat", "lobby"]

if (!prefixMatches || parts.length < prefixParts.length + 2) {
  return null; // No match
}

const namespace = parts[prefixParts.length];  // "chat"
const name = parts[prefixParts.length + 1];   // "lobby"
```

### 4.3 Namespace Resolution

```typescript
// Automatically discovers DO namespaces from env
for (const [k, v] of Object.entries(env)) {
  if (v && typeof v === "object" && "idFromName" in v) {
    const kebab = camelCaseToKebabCase(k);
    namespaceMap[kebab] = v as DurableObjectNamespace;
    // "MyServer" -> "my-server"
  }
}
```

### 4.4 CORS Handling

```typescript
const options: PartyServerOptions = {
  cors: true,  // Enable permissive CORS
  // Or explicit headers:
  cors: {
    "Access-Control-Allow-Origin": "https://myapp.com",
    "Access-Control-Allow-Credentials": "true",
    "Access-Control-Allow-Methods": "GET, POST, HEAD, OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type, Authorization"
  }
};
```

### 4.5 Request Interception

```typescript
const options: PartyServerOptions = {
  // Intercept WebSocket upgrade requests
  onBeforeConnect: async (req, lobby) => {
    const auth = req.headers.get("Authorization");
    if (!auth) {
      return new Response("Unauthorized", { status: 401 });
    }

    // Modify request (add headers, etc.)
    const newReq = new Request(req);
    newReq.headers.set("x-user-id", validateToken(auth));
    return newReq;
  },

  // Intercept HTTP requests
  onBeforeRequest: async (req, lobby) => {
    // Rate limiting, auth, etc.
    if (await isRateLimited(req)) {
      return new Response("Too Many Requests", { status: 429 });
    }
  }
};
```

### 4.6 Lobby Interface

```typescript
interface Lobby<Env> {
  /** @deprecated Use className instead */
  party: string;           // Kebab-case namespace
  className: string;       // DO class name
  name: string;            // Room name
}

// Usage in hooks
onBeforeConnect: (req, lobby) => {
  console.log(`Connecting to ${lobby.className}:${lobby.name}`);
}
```

---

## 5. Hibernation Deep Dive

### 5.1 What is Hibernation?

Hibernation allows Durable Objects to:
- Sleep when idle (no active processing)
- Wake on events (WebSocket messages, alarms, HTTP requests)
- Persist connection state without keeping DO active

### 5.2 Enabling Hibernation

```typescript
export class MyServer extends Server {
  static options = { hibernate: true };
}
```

### 5.3 WebSocket Attachment Storage

```typescript
// PartyServer stores metadata in WebSocket attachments
type ConnectionAttachments = {
  __pk: {
    id: string;
    tags: string[];
    uri?: string;
  };
  __user?: unknown;  // User-defined state
};

// Access via helper functions
const meta = tryGetPartyServerMeta(ws);
if (meta) {
  console.log(`Connection ${meta.id} with tags ${meta.tags}`);
}
```

### 5.4 Lazy Connection Rehydration

```typescript
export const createLazyConnection = (ws: WebSocket): Connection => {
  return Object.defineProperties(ws, {
    id: {
      get() {
        return attachments.get(ws).__pk.id;
      }
    },
    tags: {
      get() {
        return attachments.get(ws).__pk.tags ?? [];
      }
    },
    state: {
      get() {
        return ws.deserializeAttachment() as ConnectionState<unknown>;
      }
    },
    setState: {
      value: function setState<T>(setState: T | ConnectionSetStateFn<T>) {
        let state: T;
        if (setState instanceof Function) {
          state = setState((this as Connection<T>).state);
        } else {
          state = setState;
        }
        ws.serializeAttachment(state);
        return state as ConnectionState<T>;
      }
    }
  }) as Connection;
};
```

### 5.5 Hibernation Event Handlers

```typescript
// These methods are called even when DO is hibernating
async webSocketMessage(ws: WebSocket, message: WSMessage) {
  const connection = createLazyConnection(ws);
  await this.#ensureInitialized();
  return this.onMessage(connection, message);
}

async webSocketClose(
  ws: WebSocket,
  code: number,
  reason: string,
  wasClean: boolean
) {
  const connection = createLazyConnection(ws);
  await this.#ensureInitialized();
  return this.onClose(connection, code, reason, wasClean);
}

async webSocketError(ws: WebSocket, error: unknown) {
  const connection = createLazyConnection(ws);
  await this.#ensureInitialized();
  return this.onError(connection, error);
}
```

### 5.6 Hibernation vs. Non-Hibernation

| Aspect | Hibernation | Non-Hibernation |
|--------|-------------|-----------------|
| Resource usage | Low when idle | Constant while connected |
| Wake latency | Small delay on first event | Immediate |
| Connection state | Stored in attachments | Stored in memory |
| Event handling | Lazy rehydration | Direct handlers |
| Cost | Cheaper for sporadic traffic | Better for constant activity |

### 5.7 Best Practices for Hibernation

```typescript
// DO: Store minimal state in connection.setState()
connection.setState({ userId: "123", status: "active" });

// DON'T: Store large data in attachments
// connection.setState({ largeData: hugeObject }); // Bad!

// DO: Use SQL storage for persistent data
this.ctx.storage.sql.exec("INSERT INTO ...");

// DON'T: Rely on in-memory state across hibernation
// this.cache = {}; // Will be lost!

// DO: Reinitialize on onStart
async onStart() {
  this.cache = await this.loadCacheFromStorage();
}
```

---

## 6. Advanced Patterns

### 6.1 Multi-Party Routing

```typescript
// Route to different parties based on URL
export default {
  async fetch(request: Request, env: Env) {
    // Try chat rooms first
    let response = await routePartykitRequest(request, env, {
      prefix: "chat"
    });
    if (response) return response;

    // Try game rooms
    response = await routePartykitRequest(request, env, {
      prefix: "game"
    });
    if (response) return response;

    return new Response("Not Found", { status: 404 });
  }
};
```

### 6.2 Dynamic Room Creation

```typescript
export class LobbyServer extends Server {
  async onRequest(request: Request): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === "/api/rooms" && request.method === "POST") {
      const body = await request.json();
      const roomName = body.name;

      // Create room by getting stub (wakes it up)
      const stub = await getServerByName(
        this.env.GameServer,
        roomName,
        { props: { maxPlayers: body.maxPlayers } }
      );

      return Response.json({
        roomId: roomName,
        url: `/parties/game/${roomName}`
      });
    }

    return new Response("Not Found", { status: 404 });
  }
}
```

### 6.3 Room Discovery

```typescript
export class RoomDirectory extends Server {
  async onRequest(request: Request): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === "/api/rooms") {
      // List all active rooms
      // Note: DOs don't provide a way to list all instances
      // You need to maintain a registry separately

      const rooms = await this.ctx.storage.sql.exec(
        "SELECT * FROM rooms WHERE active = 1"
      ).raw();

      return Response.json(rooms);
    }

    return new Response("Not Found", { status: 404 });
  }
}
```

### 6.4 Inter-Room Communication

```typescript
// Rooms can communicate via stubs
export class GameServer extends Server {
  async broadcastToAllRooms(message: string) {
    // Get all room names from storage
    const rooms = this.ctx.storage.sql.exec(
      "SELECT name FROM rooms"
    ).raw();

    for (const [roomName] of rooms) {
      const stub = await getServerByName(this.env.GameServer, roomName as string);

      // Use RPC (if supported) or HTTP
      await stub.fetch(new Request("http://internal/broadcast", {
        method: "POST",
        body: message
      }));
    }
  }
}
```

### 6.5 Connection Limits

```typescript
export class LimitedServer extends Server<Env, { maxConnections: number }> {
  private maxConnections: number = 100;

  async onStart(props: { maxConnections?: number }) {
    this.maxConnections = props?.maxConnections ?? 100;
  }

  onConnect(connection, ctx) {
    if (this.getConnections().length >= this.maxConnections) {
      connection.close(4003, "Room is full");
      return;
    }

    // Accept connection
    // ...
  }
}
```

### 6.6 Authenticated Rooms

```typescript
export class AuthServer extends Server {
  async onConnect(connection: Connection, ctx: ConnectionContext) {
    const token = new URL(ctx.request.url).searchParams.get("token");

    if (!token) {
      connection.close(4001, "Missing token");
      return;
    }

    try {
      const user = await this.validateToken(token);
      connection.setState({ userId: user.id, username: user.name });
    } catch (e) {
      connection.close(4001, "Invalid token");
    }
  }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial room management deep dive created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
