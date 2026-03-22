---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.InstantDB/instant
repository: https://github.com/instantdb/instant
explored_at: 2026-03-22
language: TypeScript, Clojure, JavaScript
---

# Project Exploration: InstantDB

## Overview

**InstantDB** is a modern Firebase alternative that provides a **real-time database for the frontend**. It fundamentally changes how developers build applications by giving the frontend direct database access with built-in synchronization, offline support, and optimistic updates.

### Key Value Proposition

- **Database on the client** - Write queries directly from frontend code
- **Real-time by default** - All queries are multiplayer/collaborative
- **Optimistic updates** - Changes appear instantly with automatic rollback on error
- **Offline-first** - Works without network, syncs when reconnected
- **No backend code required** - Permission rules replace API endpoints

### Example Usage

```javascript
import { init, tx, id } from "@instantdb/react";

const db = init({ appId: process.env.NEXT_PUBLIC_APP_ID });

function Chat() {
  // 1. Read (real-time query)
  const { isLoading, error, data } = db.useQuery({
    messages: {},
  });

  // 2. Write (optimistic update)
  const addMessage = (message) => {
    db.transact(tx.messages[id()].update(message));
  };

  // 3. Render
  return <UI data={data} onAdd={addMessage} />;
}
```

## Architecture

### High-Level Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Frontend Clients                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │   React App  │  │  React Native│  │  Vanilla JS  │              │
│  │  (IndexedDB) │  │ (AsyncStorage)│  │ (LocalStorage)│             │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              │ WebSocket (real-time sync)
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Instant Sync Server                             │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                   Clojure Sync Engine                           │ │
│  │  - Datalog query processor                                      │ │
│  │  - InstaQL translator                                           │ │
│  │  - Presence & topics (ephemeral state)                          │ │
│  │  - Permission checker (CEL)                                     │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              │ SQL
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       PostgreSQL                                     │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │              Triple Store (EAV Model)                           │ │
│  │  - entities (e)                                                 │ │
│  │  - attributes (a)                                               │ │
│  │  - values (v)                                                   │ │
│  │  - timestamps (t)                                               │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

## Repository Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.InstantDB/instant/
├── client/                          # Frontend SDK monorepo
│   ├── packages/                    # SDK packages
│   │   ├── instant-client/          # Core client (TypeScript)
│   │   ├── instant-react/           # React hooks
│   │   ├── instant-rn/              # React Native
│   │   └── instant-vanilla/         # Vanilla JS
│   ├── sandbox/                     # Test applications
│   ├── www/                         # Documentation site
│   └── scripts/                     # Build scripts
│
├── server/                          # Backend sync server
│   ├── src/                         # Clojure source
│   │   └── instant/                 # Main server code
│   │       ├── auth.clj             # Authentication
│   │       ├── datalog.clj          # Datalog engine
│   │       ├── instaql.clj          # Query language
│   │       ├── permissions.clj      # CEL-based auth
│   │       ├── storage.clj          # Postgres interface
│   │       ├── sync.clj             # WebSocket sync
│   │       └── triples.clj          # Triple store logic
│   ├── resources/                   # Static resources
│   ├── test/                        # Server tests
│   └── refinery/                    # Database migrations
│
└── local-first-landscape-data/      # Research data
```

## Core Concepts

### 1. Triples (Data Model)

All data is stored as **triples** (Entity-Attribute-Value):

```
┌──────────┬─────────────┬────────────┬──────────────┐
│ Entity   │ Attribute   │ Value      │ Timestamp    │
├──────────┼─────────────┼────────────┼──────────────┤
│ user_1   │ name        │ "Alice"    │ 1234567890   │
│ user_1   │ age         │ 30         │ 1234567891   │
│ post_1   │ author      │ user_1     │ 1234567892   │
│ post_1   │ content     │ "Hello!"   │ 1234567892   │
└──────────┴─────────────┴────────────┴──────────────┘
```

Benefits:
- Flexible schema evolution
- Easy history/audit trail
- Efficient incremental sync

### 2. InstaQL (Query Language)

**InstaQL** is a relational query language inspired by GraphQL:

```javascript
// Query: Get all users with their posts and comments
{
  users: {
    posts: {
      comments: {
        author: {}
      }
    }
  }
}
```

Result shape matches query shape:
```javascript
{
  users: [
    {
      id: "user_1",
      name: "Alice",
      posts: [
        {
          id: "post_1",
          content: "Hello!",
          comments: [
            {
              id: "comment_1",
              text: "Nice!",
              author: { id: "user_2", name: "Bob" }
            }
          ]
        }
      ]
    }
  ]
}
```

### 3. Transactions

Transactions use a **fluent API** for mutations:

```javascript
import { tx, id } from "@instantdb/react";

// Create a new post with author reference
db.transact(
  tx.posts[id()].update({
    title: "My Post",
    content: "Hello world!",
    author: users[userId],  // Link to existing entity
  })
);

// Update multiple entities
db.transact([
  tx.users[userId].update({ lastActive: Date.now() }),
  tx.posts[postId].update({ viewCount: viewCount + 1 }),
]);

// Delete
db.transact(tx.posts[postId].delete());
```

### 4. Presence & Topics (Ephemeral State)

For transient data like cursors or "typing" indicators:

```javascript
// Set presence
db.presence.set({
  typingIn: "room_123",
  cursor: { x: 100, y: 200 }
});

// Subscribe to presence
db.presence.subscribe((peers) => {
  // Updated whenever peers change presence
});

// Topics (broadcast)
db.topic("cursors").publish({ userId, position });
db.topic("cursors").subscribe((updates) => {
  // Handle cursor updates
});
```

## Client Architecture

### Client-Side Store

```
┌─────────────────────────────────────────────────────────────────┐
│                    Instant Client                                │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                   Query Cache                               │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │ │
│  │  │  Query 1     │  │  Query 2     │  │  Query 3     │     │ │
│  │  │  Results     │  │  Results     │  │  Results     │     │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘     │ │
│  └────────────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Transaction Queue                              │ │
│  │  - Pending mutations                                        │ │
│  │  - Optimistic state                                         │ │
│  │  - Rollback handlers                                        │ │
│  └────────────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Persistence Layer                              │ │
│  │  - IndexedDB (web)                                          │ │
│  │  - AsyncStorage (RN)                                        │ │
│  │  - Automatic cache warming                                  │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Optimistic Update Flow

```
1. User triggers mutation
         │
         ▼
2. Generate transaction
         │
         ▼
3. Apply optimistically to cache
         │
         ▼
4. Send to server
         │
         ├──────────────┐
         │              │
         ▼              ▼
5a. Success         5b. Error
    Keep changes        Rollback
    Notify sync         Notify error
```

## Server Architecture

### Sync Server (Clojure)

```clojure
;; Simplified sync flow
(defn handle-message [client msg]
  (case (:type msg)
    :query (handle-query client (:payload msg))
    :transact (handle-transaction client (:payload msg))
    :presence (handle-presence client (:payload msg)))

(defn handle-query [client query]
  ;; 1. Parse InstaQL
  (let [parsed (instaql/parse query)
        ;; 2. Execute against Postgres
        results (execute-datalog parsed)
        ;; 3. Send to client
        _ (send-to-client client results)
        ;; 4. Register for updates
        _ (register-query client query)]
    results))
```

### Datalog Engine

InstantDB uses **Datalog** as its query intermediate representation:

```clojure
;; InstaQL -> Datalog translation
;; Query: { users: { posts: {} } }

;; Becomes:
[:find ?user ?post
 :where
 [?user :entity_type "users"]
 [?post :entity_type "posts"]
 [?post :author ?user]]
```

### PostgreSQL WAL Tail

```
┌─────────────────────────────────────────────────────────────────┐
│              Change Data Capture                                 │
│                                                                  │
│  PostgreSQL ──▶ WAL ──▶ Decoder ──▶ Query Invalidator          │
│                                                                  │
│  When data changes:                                              │
│  1. Parse WAL entry                                              │
│  2. Find affected queries                                        │
│  3. Push updates to subscribed clients                           │
└─────────────────────────────────────────────────────────────────┘
```

## Permission System

### CEL-Based Rules

```javascript
// Permission rules (JavaScript-like syntax)
rules = {
  posts: {
    bind: "post == data",
    allow: {
      view: "true",  // Anyone can view
      create: "auth != null",  // Authenticated users only
      update: "auth.id == post.authorId",  // Author only
      delete: "auth.id == post.authorId",
    },
  },
};
```

### Permission Evaluation

```clojure
(defn check-permission [entity action auth data]
  (let [rule (get-rules entity action)
        context {:auth auth :data data}]
    (cel/evaluate rule context)))
```

## Performance Characteristics

### Client Side

| Operation | Latency | Notes |
|-----------|---------|-------|
| Query (cached) | <1ms | In-memory lookup |
| Query (cache miss) | ~50ms | IndexedDB read |
| Transaction (local) | <5ms | Optimistic update |
| Sync roundtrip | 50-200ms | Network dependent |

### Server Side

| Operation | Latency | Notes |
|-----------|---------|-------|
| Datalog query | 1-10ms | In-memory execution |
| Postgres read | 1-5ms | Indexed lookup |
| Permission check | <1ms | CEL evaluation |
| Broadcast update | <5ms | WebSocket push |

## Trade-offs

| Aspect | Choice | Trade-off |
|--------|--------|-----------|
| Data Model | Triples | Flexible but verbose |
| Query Language | Datalog/InstaQL | Declarative but learning curve |
| Sync Protocol | Custom WebSocket | Optimized but proprietary |
| Persistence | IndexedDB | Good browser support but async |
| Permissions | CEL | Powerful but complex |
| Backend | Clojure | Concise but niche language |

## Reproducing the Architecture

### Step 1: Triple Store Schema

```sql
CREATE TABLE triples (
  id BIGSERIAL PRIMARY KEY,
  entity_id UUID NOT NULL,
  attribute TEXT NOT NULL,
  value JSONB NOT NULL,
  value_type TEXT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_entity_attr ON triples(entity_id, attribute);
CREATE INDEX idx_attr_value ON triples USING GIN(attribute, value);
```

### Step 2: Sync Protocol

```typescript
// WebSocket message types
type ClientMessage =
  | { type: "query"; payload: Query }
  | { type: "transact"; payload: Transaction }
  | { type: "presence"; payload: PresenceUpdate };

type ServerMessage =
  | { type: "query_result"; payload: QueryResult }
  | { type: "transaction_result"; payload: TransactionResult }
  | { type: "sync_update"; payload: SyncUpdate };
```

### Step 3: Query Cache

```typescript
class QueryCache {
  private cache = new Map<string, QueryResult>();
  private subscriptions = new Map<string, Set<Client>>();

  register(client: Client, query: Query) {
    const hash = hashQuery(query);
    this.subscriptions.get(hash)?.add(client);
  }

  invalidate(triples: Triple[]) {
    const affectedQueries = this.findAffectedQueries(triples);
    for (const query of affectedQueries) {
      const result = this.recompute(query);
      this.notifySubscribers(query, result);
    }
  }
}
```

## Related Projects

- **local-first-landscape-data** - Research on local-first software
- **instant-react-repro** - React reproduction examples

## Documentation References

- Main docs: https://instantdb.com/docs
- InstaQL: https://instantdb.com/docs/instaql
- Transactions: https://instantdb.com/docs/instaml
- Presence: https://instantdb.com/docs/presence-and-topics
