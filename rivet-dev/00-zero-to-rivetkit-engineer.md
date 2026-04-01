---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/rivet-dev
explored_at: 2026-03-29
prerequisites: TypeScript/JavaScript basics, Node.js familiarity
---

# Zero to RivetKit Engineer - Complete Fundamentals

## Table of Contents

1. [What is RivetKit?](#what-is-rivetkit)
2. [Why RivetKit?](#why-rivetkit)
3. [Installation](#installation)
4. [Your First Actor](#your-first-actor)
5. [State Management](#state-management)
6. [Realtime Events](#realtime-events)
7. [Client Integration](#client-integration)
8. [Storage Drivers](#storage-drivers)
9. [Production Deployment](#production-deployment)

## What is RivetKit?

RivetKit is a **TypeScript library for building stateful actors** - long-lived processes with durable state and realtime capabilities. It's an open-source alternative to Cloudflare Durable Objects that works with your own infrastructure.

### The Problem RivetKit Solves

**Traditional Stateless Architecture:**
```
Request → Load Balancer → Server → Database → Response
              ↓
         Every request hits database
         No memory between requests
         Complex realtime = WebSocket servers + Redis
```

**Actor Model with RivetKit:**
```
Request → Actor (in-memory state) → Response
              ↓
         State lives with compute
         Automatic persistence
         Realtime built-in
```

### Key Concepts

| Term | Definition |
|------|------------|
| **Actor** | Long-lived process with durable state |
| **Registry** | Collection of actor definitions |
| **Driver** | Storage backend (FS, Postgres, Redis) |
| **Client** | SDK for calling actors |
| **Broadcast** | Send events to connected clients |

## Why RivetKit?

### Benefits

1. **Simple Mental Model**: State + Actions = Actor
2. **No JavaScript Required**: Build interactive apps with TypeScript
3. **Automatic Persistence**: State saved automatically
4. **Realtime Built-in**: WebSocket events without extra code
5. **Self-Hostable**: Run on your infrastructure
6. **Multi-Platform**: Node.js, Bun, Cloudflare Workers

### When to Use RivetKit

**Good fit:**
- Realtime collaborative apps
- Chat/messaging systems
- Multiplayer games
- Rate limiting
- Session management
- AI agents with memory

**Not recommended:**
- Simple CRUD apps
- Static content sites
- One-off scripts

## Installation

### Install RivetKit

```bash
npm install rivetkit
```

### Project Setup

```bash
# Create project directory
mkdir my-actor-app
cd my-actor-app

# Initialize package.json
npm init -y

# Install dependencies
npm install rivetkit hono
npm install -D tsx @types/node typescript
```

### TypeScript Configuration

```json
{
    "compilerOptions": {
        "target": "ES2022",
        "module": "ESNext",
        "moduleResolution": "bundler",
        "esModuleInterop": true,
        "strict": true,
        "skipLibCheck": true
    }
}
```

## Your First Actor

### Step 1: Define Actor Registry

```typescript
// registry.ts
import { actor, setup } from "rivetkit";

export const counter = actor({
    // Initial state
    state: { count: 0 },

    // Actions
    actions: {
        increment: (c, amount: number = 1) => {
            c.state.count += amount;
            return c.state.count;
        },

        decrement: (c, amount: number = 1) => {
            c.state.count -= amount;
            return c.state.count;
        },

        getCount: (c) => {
            return c.state.count;
        }
    }
});

export const registry = setup({
    use: { counter }
});
```

### Step 2: Create Server

```typescript
// server.ts
import { registry } from "./registry";

// Create server with file system driver
const { client, serve } = registry.createServer();

serve({ port: 3000 });

console.log("Server running on http://localhost:3000");
```

### Step 3: Run the Server

```bash
npx tsx server.ts

# Output:
# Server running on http://localhost:3000
```

### Step 4: Call the Actor

```bash
# Using curl
curl -X POST http://localhost:3000/counter/my-counter/increment
# Response: {"result": 1}

curl -X POST http://localhost:3000/counter/my-counter/increment
# Response: {"result": 2}

curl http://localhost:3000/counter/my-counter/getCount
# Response: {"result": 2}
```

## State Management

### Defining State

```typescript
// Simple state
const simpleActor = actor({
    state: { count: number }
});

// Complex state
const complexActor = actor({
    state: {
        users: Map<string, { id: string; name: string }>;
        messages: Array<{ text: string; timestamp: number }>;
        metadata: {
            createdAt: Date;
            updatedAt: Date;
        }
    }
});
```

### State Persistence

State is automatically persisted:

```typescript
const myActor = actor({
    state: { data: string },

    actions: {
        update: (c, newData: string) => {
            // This change is automatically saved
            c.state.data = newData;

            // No need to call save() or persist()
            return "ok";
        }
    }
});
```

### State Lifecycle

```typescript
const lifecycleActor = actor({
    state: { value: 0 },

    // Called when actor is created/loaded
    onInit: (c) => {
        console.log("Actor initialized");
    },

    // Called before state is persisted
    onBeforeSave: (c) => {
        console.log("Saving state:", c.state);
    },

    // Called after state is persisted
    onAfterSave: (c) => {
        console.log("State saved");
    },

    actions: {
        update: (c, value: number) => {
            c.state.value = value;
        }
    }
});
```

## Realtime Events

### Broadcasting Events

```typescript
const chatActor = actor({
    state: {
        messages: [] as string[],
        participants: new Set<string>()
    },

    actions: {
        sendMessage: (c, userId: string, text: string) => {
            c.state.messages.push({ userId, text, timestamp: Date.now() });

            // Broadcast to all connected clients
            c.broadcast("message", {
                userId,
                text,
                timestamp: Date.now()
            });

            return "ok";
        },

        join: (c, userId: string) => {
            c.state.participants.add(userId);

            c.broadcast("userJoined", { userId });

            return "ok";
        }
    }
});
```

### Client-Side Subscription

```typescript
// TypeScript client
const chat = client.chat.getOrCreate("room-1");

// Subscribe to events
chat.subscribe("message", (data) => {
    console.log("New message:", data.text);
});

chat.subscribe("userJoined", (data) => {
    console.log("User joined:", data.userId);
});
```

### React Integration

```tsx
import { useActor } from "rivetkit/react";

function ChatRoom({ roomId }: { roomId: string }) {
    const { state, actions, events } = useActor("chat", roomId, {
        messages: [],
        participants: new Set()
    });

    // Listen for new messages
    events.on("message", (data) => {
        // Update UI with new message
    });

    return (
        <div>
            {state.messages.map((msg) => (
                <div key={msg.timestamp}>{msg.text}</div>
            ))}
        </div>
    );
}
```

## Client Integration

### TypeScript Client

```typescript
import { createClient } from "rivetkit/client";

const client = createClient("http://localhost:3000");

// Get or create actor
const counter = client.counter.getOrCreate("my-counter");

// Call actions
const count = await counter.increment(5);
console.log(count); // 5

// Get current state
const state = await counter.getState();
console.log(state.count); // 5
```

### React Hook

```tsx
import { useActor } from "rivetkit/react";

function Counter() {
    const { state, actions, isLoading, error } = useActor(
        "counter",
        "my-counter",
        { count: 0 } // Initial optimistic state
    );

    if (isLoading) return <div>Loading...</div>;
    if (error) return <div>Error: {error.message}</div>;

    return (
        <div>
            <p>Count: {state.count}</p>
            <button onClick={() => actions.increment(1)}>+</button>
            <button onClick={() => actions.decrement(1)}>-</button>
        </div>
    );
}
```

### Rust Client

```rust
use rivetkit::{Client, Actor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("http://localhost:3000");

    let counter = client.actor::<CounterActor>("my-counter");

    let count = counter.call("increment", 1).await?;
    println!("Count: {}", count);

    Ok(())
}
```

## Storage Drivers

### File System Driver (Development)

```typescript
const { client, serve } = registry.createServer({
    driver: "file-system",
    storagePath: "./.rivet-data"
});
```

**Pros:** Simple, no setup, persists across restarts
**Cons:** Single server only, not for production

### Memory Driver (Testing)

```typescript
const { client, serve } = registry.createServer({
    driver: "memory"
});
```

**Pros:** Fast, no I/O
**Cons:** No persistence, data lost on restart

### Postgres Driver (Production)

```typescript
import { createPostgresDriver } from "rivetkit/drivers/postgres";

const { client, serve } = registry.createServer({
    driver: createPostgresDriver({
        connectionString: process.env.DATABASE_URL,
        tableName: "rivet_actors",
        persistInterval: 5000 // Save every 5 seconds
    })
});
```

**Schema:**
```sql
CREATE TABLE rivet_actors (
    actor_type TEXT NOT NULL,
    actor_key TEXT NOT NULL,
    state JSONB NOT NULL,
    updated_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (actor_type, actor_key)
);
```

### Redis Driver (Distributed)

```typescript
import { createRedisDriver } from "rivetkit/drivers/redis";

const { client, serve } = registry.createServer({
    driver: createRedisDriver({
        url: process.env.REDIS_URL,
        keyPrefix: "rivet:",
        ttl: 86400 // 24 hour TTL
    })
});
```

## Production Deployment

### Environment Variables

```bash
# .env
RIVET_DRIVER=postgres
DATABASE_URL=postgresql://user:pass@localhost:5432/rivet
REDIS_URL=redis://localhost:6379
PORT=3000
```

### Docker Deployment

```dockerfile
FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm ci

COPY . .

RUN npm run build

EXPOSE 3000

CMD ["node", "dist/server.js"]
```

### Docker Compose

```yaml
version: "3.8"

services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgresql://user:pass@db:5432/rivet
    depends_on:
      - db

  db:
    image: postgres:15
    environment:
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=pass
      - POSTGRES_DB=rivet
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

### Scaling

```typescript
// Horizontal scaling with shared storage
const { client, serve } = registry.createServer({
    driver: createPostgresDriver({
        connectionString: process.env.DATABASE_URL,
        // Enable connection pooling
        poolSize: 20,
        // Enable replication
        readReplicas: [
            "postgresql://replica1:5432/rivet",
            "postgresql://replica2:5432/rivet"
        ]
    })
});
```

---

**Next Steps:**
- [01-rivetkit-exploration.md](./01-rivetkit-exploration.md) - Full architecture
- [02-storage-drivers-deep-dive.md](./02-storage-drivers-deep-dive.md) - Storage backends
- [03-realtime-patterns-deep-dive.md](./03-realtime-patterns-deep-dive.md) - Realtime features
