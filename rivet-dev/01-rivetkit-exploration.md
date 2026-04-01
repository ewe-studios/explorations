---
location: /home/darkvoid/Boxxed/@formulas/src.rivet-dev/rivetkit
repository: git@github.com:rivet-dev/rivetkit.git
explored_at: 2026-03-29
language: TypeScript, Rust
category: Stateful Actors, Durable Objects Alternative
---

# RivetKit - Exploration

## Overview

RivetKit is an **open-source alternative to Cloudflare Durable Objects** - a library for building long-lived, stateful actors with realtime capabilities. It provides the same developer experience as Durable Objects but works with your own infrastructure and supports multiple storage backends.

### Key Value Proposition

- **Durable Objects Compatible**: Same API pattern, self-hostable
- **Multi-Platform**: Node.js, Bun, Cloudflare Workers, Vercel
- **Multiple Storage Backends**: File system, Postgres, Memory, Redis
- **Realtime Built-in**: WebSocket events, SSE support
- **Type-Safe**: Full TypeScript type inference
- **Framework Agnostic**: Works with Hono, Express, tRPC, Next.js

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    RivetKit Architecture                         │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   React Client  │  │   TypeScript    │  │   Rust Client   │ │
│  │   (useActor)    │  │   Client SDK    │  │   (tokio-based) │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘ │
│           │                    │                    │           │
│           └────────────────────┼────────────────────┘           │
│                                │                                │
│                    ┌───────────▼───────────┐                   │
│                    │   RivetKit Server     │                   │
│                    │   - Actor Registry    │                   │
│                    │   - Connection Mgmt   │                   │
│                    │   - Event Broadcast   │                   │
│                    └───────────┬───────────┘                   │
│                                │                                │
│           ┌────────────────────┼────────────────────┐          │
│           │                    │                    │          │
│  ┌────────▼───────┐  ┌────────▼───────┐  ┌────────▼───────┐   │
│  │ Actor Driver   │  │ Actor Driver   │  │ Actor Driver   │   │
│  │ (File System)  │  │ (Postgres)     │  │ (Memory)       │   │
│  └────────────────┘  └────────────────┘  └────────────────┘   │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              Rivet Engine (Optional)                     │  │
│  │              - Actor scheduling                          │  │
│  │              - Cross-region replication                  │  │
│  │              - Persistence layer                         │  │
│  └─────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Monorepo Structure

```
rivetkit/
├── packages/
│   ├── core/                 # Core RivetKit library
│   │   ├── src/
│   │   │   ├── actor/        # Actor definition & lifecycle
│   │   │   │   ├── actor.ts          # Actor class
│   │   │   │   ├── state.ts          # State management
│   │   │   │   ├── actions.ts        # Action handlers
│   │   │   │   └── events.ts         # Event broadcasting
│   │   │   ├── client/       # Client SDK
│   │   │   │   ├── client.ts         # API client
│   │   │   │   ├── realtime.ts       # WebSocket handling
│   │   │   │   └── sse.ts            # Server-sent events
│   │   │   ├── drivers/      # Storage drivers
│   │   │   │   ├── file-system.ts    # FS persistence
│   │   │   │   ├── postgres.ts       # Postgres storage
│   │   │   │   ├── memory.ts         # In-memory storage
│   │   │   │   └── redis.ts          # Redis storage
│   │   │   ├── registry/     # Actor registry
│   │   │   │   ├── registry.ts       # Actor registry
│   │   │   │   └── config.ts         # Registry config
│   │   │   ├── server/       # Server integration
│   │   │   │   ├── hono.ts           # Hono adapter
│   │   │   │   ├── express.ts        # Express adapter
│   │   │   │   └── standalone.ts     # Standalone server
│   │   │   └── types/        # TypeScript types
│   │   └── package.json
│   │
│   ├── react/                # React hooks
│   │   ├── src/
│   │   │   ├── useActor.ts   # Main actor hook
│   │   │   ├── useActorState.ts  # State-only hook
│   │   │   ├── useActorEvent.ts  # Event subscription
│   │   │   └── provider.ts   # React provider
│   │   └── package.json
│   │
│   ├── rust/                 # Rust client SDK
│   │   ├── src/
│   │   │   ├── client.rs     # Rust client
│   │   │   ├── actor.rs      # Actor proxy
│   │   │   └── websocket.rs  # WS handling
│   │   └── Cargo.toml
│   │
│   └── integrations/         # Framework integrations
│       ├── hono/
│       ├── express/
│       ├── trpc/
│       └── better-auth/
│
├── examples/
│   ├── ai-agent/             # AI agent example
│   ├── chat-room/            # Realtime chat
│   ├── crdt/                 # Yjs collaborative editing
│   ├── game/                 # Multiplayer game
│   ├── sync/                 # Local-first sync
│   ├── rate/                 # Rate limiter
│   ├── database/             # Per-user database
│   ├── tenant/               # Multi-tenant SaaS
│   └── stream/               # Stream processing
│
├── scripts/                  # Build & release scripts
├── package.json              # Root package.json
└── pnpm-workspace.yaml       # pnpm workspace config
```

## Core Concepts

### 1. Actors

Actors are long-lived, stateful units of computation:

```typescript
// registry.ts
import { actor, setup } from "rivetkit";

export const counter = actor({
    // Initial state
    state: { count: 0, lastUpdated: Date.now() },

    // Actions (methods)
    actions: {
        increment: (c, amount: number = 1) => {
            // State changes are automatically persisted
            c.state.count += amount;
            c.state.lastUpdated = Date.now();

            // Broadcast events to connected clients
            c.broadcast("countChanged", c.state.count);

            // Return value to caller
            return c.state.count;
        },

        decrement: (c, amount: number = 1) => {
            c.state.count -= amount;
            return c.state.count;
        },

        getCount: (c) => {
            return c.state.count;
        },

        // Async actions
        fetchExternal: async (c) => {
            const data = await fetch("https://api.example.com/data");
            return data.json();
        }
    },

    // Lifecycle hooks
    onInit: (c) => {
        console.log("Actor initialized");
    },

    onBeforeSave: (c) => {
        console.log("State about to be persisted");
    }
});

export const registry = setup({
    use: { counter }
});
```

### 2. State Management

State is durable and automatically persisted:

```typescript
// Simple state
state: { count: number }

// Complex state
state: {
    users: Map<string, User>;
    messages: Message[];
    metadata: {
        createdAt: Date;
        updatedAt: Date;
    }
}

// State with Yjs (CRDT)
import * as Y from "yjs";
state: {
    doc: Y.Doc;
    awareness: Y.Awareness;
}
```

**State persistence:**
- File System: JSON files per actor
- Postgres: JSONB columns
- Memory: In-process (no persistence)
- Redis: Redis hashes

### 3. Client Integration

#### TypeScript Client

```typescript
// client.ts
import { createClient } from "rivetkit/client";

const client = createClient("http://localhost:3000");

// Get or create actor by key
const counter = client.counter.getOrCreate("my-counter");

// Call actions
const count = await counter.increment(5);
console.log(count); // 5

// Subscribe to events
counter.subscribe("countChanged", (newCount) => {
    console.log("Count changed:", newCount);
});
```

#### React Hook

```tsx
// CounterComponent.tsx
import { useActor } from "rivetkit/react";

function Counter() {
    const { state, actions, events } = useActor(
        "counter",
        "my-counter",
        { count: 0 } // Initial state (optimistic)
    );

    return (
        <div>
            <p>Count: {state.count}</p>
            <button onClick={() => actions.increment(1)}>
                Increment
            </button>
        </div>
    );
}
```

#### Rust Client

```rust
// main.rs
use rivetkit::{Client, Actor};

#[tokio::main]
async fn main() {
    let client = Client::new("http://localhost:3000");
    let counter = client.actor::<Counter>("my-counter");

    let count = counter.call("increment", 1).await?;
    println!("Count: {}", count);
}
```

### 4. Server Setup

#### Hono Integration

```typescript
// server.ts
import { registry } from "./registry";
import { Hono } from "hono";

// Create server with file system driver (development)
const { client, serve } = registry.createServer();

const app = new Hono();

app.post("/increment/:name", async (c) => {
    const name = c.req.param("name");

    // Get or create actor
    const counter = client.counter.getOrCreate(name);

    // Call action
    const newCount = await counter.increment(1);

    return c.json({ count: newCount });
});

// Start server
serve(app, { port: 3000 });
```

#### Standalone Server

```typescript
// server.ts
import { registry } from "./registry";

const { client, serve } = registry.createServer({
    driver: "file-system",
    storagePath: "./data",
    port: 3000
});

serve();
```

#### Cloudflare Workers

```typescript
// worker.ts
import { registry } from "./registry";

export default {
    async fetch(request, env, ctx) {
        const { client, handle } = registry.createWorker(env);
        return handle(request);
    }
};
```

### 5. Storage Drivers

#### File System Driver

```typescript
// Best for development
const { client, serve } = registry.createServer({
    driver: "file-system",
    storagePath: "./.rivet-data"
});
```

#### Postgres Driver

```typescript
// Production-ready
import { createPostgresDriver } from "rivetkit/drivers/postgres";

const { client, serve } = registry.createServer({
    driver: createPostgresDriver({
        connectionString: process.env.DATABASE_URL,
        tableName: "rivet_actors"
    })
});
```

#### Memory Driver

```typescript
// Testing only - no persistence
const { client, serve } = registry.createServer({
    driver: "memory"
});
```

#### Redis Driver

```typescript
// Distributed caching
import { createRedisDriver } from "rivetkit/drivers/redis";

const { client, serve } = registry.createServer({
    driver: createRedisDriver({
        url: process.env.REDIS_URL,
        keyPrefix: "rivet:"
    })
});
```

## Realtime Features

### WebSocket Events

```typescript
// Server: Broadcast events
actions: {
    sendMessage: (c, message: string) => {
        c.state.messages.push(message);
        c.broadcast("message", {
            text: message,
            timestamp: Date.now()
        });
    }
}

// Client: Subscribe to events
counter.subscribe("message", (data) => {
    console.log("New message:", data.text);
});
```

### Server-Sent Events (SSE)

```typescript
// Server-side
app.get("/events/:actorId", async (c) => {
    const actor = client.counter.get(c.req.param("actorId"));
    return actor.sse(); // Stream events via SSE
});

// Client-side
const eventSource = new EventSource("/events/my-counter");
eventSource.addEventListener("countChanged", (e) => {
    console.log("Count:", e.data);
});
```

## Examples

### AI Agent

```typescript
const aiAgent = actor({
    state: {
        conversation: [] as Message[],
        context: {} as Record<string, any>
    },
    actions: {
        chat: async (c, message: string) => {
            c.state.conversation.push({ role: "user", content: message });

            const response = await callLLM(c.state.conversation);

            c.state.conversation.push({
                role: "assistant",
                content: response
            });

            c.broadcast("message", response);
            return response;
        }
    }
});
```

### Chat Room

```typescript
const chatRoom = actor({
    state: {
        messages: [] as Message[],
        participants: new Set<string>()
    },
    actions: {
        join: (c, userId: string) => {
            c.state.participants.add(userId);
            c.broadcast("userJoined", { userId });
        },
        sendMessage: (c, userId: string, text: string) => {
            const message: Message = {
                id: crypto.randomUUID(),
                userId,
                text,
                timestamp: Date.now()
            };
            c.state.messages.push(message);
            c.broadcast("message", message);
        }
    }
});
```

### Rate Limiter

```typescript
const rateLimiter = actor({
    state: {
        requests: [] as number[]
    },
    actions: {
        check: (c, limit: number, window: number) => {
            const now = Date.now();
            c.state.requests = c.state.requests.filter(
                t => now - t < window
            );

            if (c.state.requests.length >= limit) {
                return { allowed: false };
            }

            c.state.requests.push(now);
            return { allowed: true };
        }
    }
});
```

## Production Considerations

### Scaling

- **Horizontal**: Multiple server instances with shared storage (Postgres/Redis)
- **Actor Sharding**: Distribute actors across servers by key hash
- **Connection Pooling**: Reuse database connections

### Persistence

```typescript
// Configure persistence interval
const { client, serve } = registry.createServer({
    driver: createPostgresDriver({
        connectionString: process.env.DATABASE_URL,
        persistInterval: 5000, // Persist every 5 seconds
        maxRetries: 3
    })
});
```

### Monitoring

```typescript
// Add middleware for logging
registry.use({
    onAction: async (ctx, next) => {
        const start = Date.now();
        await next();
        const duration = Date.now() - start;
        console.log(`Action ${ctx.action} took ${duration}ms`);
    }
});
```

---

## Related Deep Dives

- [00-zero-to-rivetkit-engineer.md](./00-zero-to-rivetkit-engineer.md) - Fundamentals
- [01-actor-lifecycle-deep-dive.md](./01-actor-lifecycle-deep-dive.md) - Actor lifecycle
- [02-storage-drivers-deep-dive.md](./02-storage-drivers-deep-dive.md) - Storage backends
- [03-realtime-patterns-deep-dive.md](./03-realtime-patterns-deep-dive.md) - Realtime features
- [rust-revision.md](./rust-revision.md) - Rust implementation considerations
- [production-grade.md](./production-grade.md) - Production deployment guide
