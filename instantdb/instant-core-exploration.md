---
name: Instant Core
description: Main InstantDB client and server implementation - TypeScript SDK and Clojure sync server
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.InstantDB/instant/
---

# Instant Core - Main InstantDB Implementation

## Overview

This is the primary InstantDB monorepo containing both the client-side SDKs (TypeScript) and the server-side sync engine (Clojure). It's the heart of InstantDB's real-time database functionality.

## Repository Structure

```
instant/
├── client/                          # Frontend SDK monorepo
│   ├── packages/                    # SDK packages
│   │   ├── instant-client/          # Core client library
│   │   ├── instant-react/           # React hooks and components
│   │   ├── instant-rn/              # React Native adapter
│   │   └── instant-vanilla/         # Vanilla JS adapter
│   ├── sandbox/                     # Test applications
│   ├── www/                         # Documentation website
│   └── scripts/                     # Build and release scripts
│
├── server/                          # Backend sync server
│   ├── src/instant/                 # Main Clojure server code
│   │   ├── auth.clj                 # Authentication handling
│   │   ├── datalog.clj              # Datalog query engine
│   │   ├── instaql.clj              # InstaQL query translator
│   │   ├── permissions.clj          # CEL permission evaluator
│   │   ├── storage.clj              # PostgreSQL storage layer
│   │   ├── sync.clj                 # WebSocket sync protocol
│   │   └── triples.clj              # Triple store operations
│   ├── resources/                   # Static resources
│   ├── test/                        # Server tests
│   └── refinery/                    # Database migrations
│
└── .github/                         # GitHub Actions workflows
```

## Client Architecture

### Package Structure

```
@instantdb/
├── core           # Core client logic (TypeScript)
├── react          # React hooks (useQuery, usePresence)
├── react-native   # React Native persistence layer
└── vanilla        # Plain JavaScript API
```

### Core Client Flow

```typescript
// Simplified client initialization
import { init } from "@instantdb/react";

const db = init({
  appId: "your-app-id",
  websocketUrl: "wss://api.instantdb.com",
});

// Query subscription flow
db.useQuery({ users: {} })
  │
  ├─▶ Check IndexedDB cache
  ├─▶ If stale/missing: send WebSocket query
  ├─▶ Register for real-time updates
  └─▶ Return { isLoading, error, data }
```

### Transaction Pipeline

```
User Action
    │
    ▼
db.transact(tx.users[id].update({...}))
    │
    ▼
┌─────────────────────────────────────────┐
│  Transaction Queue                       │
│  - Assign optimistic transaction ID      │
│  - Apply to local cache immediately      │
│  - Queue for server sync                 │
└─────────────────────────────────────────┘
    │
    ▼
WebSocket Message to Server
    │
    ▼
┌─────────────────────────────────────────┐
│  Server Response                         │
│  Success: Commit optimistic change       │
│  Error: Rollback + notify user           │
└─────────────────────────────────────────┘
```

## Server Architecture (Clojure)

### Sync Server Components

```clojure
;; Core server namespace structure
instant/
├── core.clj           ;; Main entry point
├── sync.clj           ;; WebSocket message handling
├── query.clj          ;; Query registration and invalidation
├── transaction.clj    ;; Transaction processing
├── auth.clj           ;; Authentication (JWT, custom auth)
├── permissions.clj    ;; CEL rule evaluation
└── storage/
    ├── triples.clj    ;; Triple store CRUD
    ├── postgres.clj   ;; PG connection pool
    └── wal.clj        ;; Write-Ahead Log tailing
```

### Message Protocol

```clojure
;; Client → Server messages
{:type :subscribe-query
 :payload {:q {:users {}}
           :cid "client-query-id"}}

{:type :transact
 :payload {:tx [...triples...]
           :cid "client-tx-id"}}

;; Server → Client messages
{:type :query-update
 :payload {:cid "client-query-id"
           :data {:users [...]}}}

{:type :transact-result
 :payload {:cid "client-tx-id"
           :status :ok}}
```

### Datalog Engine

The server translates InstaQL to Datalog for execution:

```clojure
;; InstaQL: { users: { posts: {} } }
;; Translates to Datalog:
[:find ?user ?post
 :where
  [?user :entity/type "users"]
  [?post :entity/type "posts"]
  [?post :post/author ?user]]

;; Execution flow
(defn execute-query [query]
  (let [datalog (instaql->datalog query)
        triples (query-datalog datalog db)
        result (triples->instaql-shape triples query)]
    result))
```

## Storage Layer

### PostgreSQL Schema

```sql
-- Core triples table (EAV model)
CREATE TABLE triples (
  id BIGSERIAL PRIMARY KEY,
  entity_id UUID NOT NULL,
  attribute TEXT NOT NULL,
  value JSONB NOT NULL,
  value_type TEXT NOT NULL,  -- 'string', 'number', 'ref', etc.
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),
  is_deleted BOOLEAN DEFAULT FALSE
);

-- Indexes for query performance
CREATE INDEX idx_triples_entity ON triples(entity_id);
CREATE INDEX idx_triples_attr ON triples(attribute);
CREATE INDEX idx_triples_entity_attr ON triples(entity_id, attribute);
CREATE INDEX idx_triples_value ON triples USING GIN(value);

-- Applications and auth
CREATE TABLE applications (
  id UUID PRIMARY KEY,
  name TEXT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE admins (
  id UUID PRIMARY KEY,
  application_id UUID REFERENCES applications(id),
  email TEXT NOT NULL,
  password_hash TEXT
);
```

### Write-Ahead Log (WAL) Tailing

```clojure
;; CDC flow for real-time updates
(defn start-wal-tail []
  (let [conn (pg/connect pool)]
    (pg/execute conn ["SELECT pg_create_logical_replication_slot('instant_sync', 'pgoutput')"])
    (pg/execute conn ["CREATE PUBLICATION instant_public FOR ALL TABLES"])
    (async/go-loop []
      (let [changes (wal/read-changes conn)]
        ;; Find affected queries
        (let [affected-queries (find-affected-queries changes)]
          ;; Broadcast to subscribed clients
          (doseq [q affected-queries]
            (broadcast-update q changes)))
        (recur)))))
```

## Key Implementation Details

### 1. Query Cache Invalidation

```typescript
class QueryCache {
  private cache = new Map<string, CachedQuery>();
  private indexes = new Map<string, Set<string>>(); // triple-key → query-ids

  // Register query for invalidation tracking
  register(queryId: string, query: Query, result: QueryResult) {
    this.cache.set(queryId, { query, result, subscribed: new Set() });

    // Index which triples this query depends on
    const tripleKeys = this.extractTripleKeys(query);
    for (const key of tripleKeys) {
      if (!this.indexes.has(key)) {
        this.indexes.set(key, new Set());
      }
      this.indexes.get(key)!.add(queryId);
    }
  }

  // Invalidate on triple change
  invalidate(changedTriples: Triple[]) {
    const affectedQueryIds = new Set<string>();

    for (const triple of changedTriples) {
      const key = this.tripleKey(triple);
      const queryIds = this.indexes.get(key);
      if (queryIds) {
        for (const qid of queryIds) {
          affectedQueryIds.add(qid);
        }
      }
    }

    // Recompute and broadcast
    for (const queryId of affectedQueryIds) {
      this.recomputeAndBroadcast(queryId);
    }
  }
}
```

### 2. Permission Evaluation (CEL)

```clojure
;; CEL rule evaluation
(defn check-permission [rule context]
  (let [cel-expr (cel/parse rule)
        env (cel/create-env context)]
    (cel/eval cel-expr env)))

;; Example rule: "auth.id == post.author_id"
(defn allow-update? [auth post]
  (check-permission "auth.id == post.author_id"
                    {:auth auth :post post}))
```

### 3. Presence & Topics

```typescript
// Ephemeral state (not persisted to Postgres)
class PresenceManager {
  private rooms = new Map<string, Map<string, Presence>>();

  setPresence(roomId: string, userId: string, data: Presence) {
    if (!this.rooms.has(roomId)) {
      this.rooms.set(roomId, new Map());
    }
    this.rooms.get(roomId)!.set(userId, data);
    this.broadcastPresence(roomId);
  }

  broadcastPresence(roomId: string) {
    const peers = this.rooms.get(roomId);
    if (peers) {
      for (const [userId, data] of peers) {
        this.sendToUser(userId, { type: 'presence', peers: Object.fromEntries(peers) });
      }
    }
  }
}
```

## Performance Characteristics

| Operation | Client Latency | Server Latency |
|-----------|---------------|----------------|
| Cache hit query | <1ms | - |
| IndexedDB query | 10-50ms | - |
| WebSocket query | 50-200ms RTT | 1-10ms |
| Transaction (local) | <5ms | - |
| Transaction (sync) | 50-200ms RTT | 5-20ms |
| Permission check | - | <1ms |
| WAL update broadcast | 50-200ms RTT | <5ms |

## Trade-offs

| Aspect | Choice | Benefit | Cost |
|--------|--------|---------|------|
| Language | Clojure server | Concise, immutable | Niche skillset |
| Data model | Triples (EAV) | Flexible schema | Verbose storage |
| Query lang | Datalog | Declarative, powerful | Learning curve |
| Sync | Custom WebSocket | Optimized for use case | Proprietary protocol |
| Permissions | CEL | Google-backed, powerful | Complex to debug |

## Reproduction Considerations

To reproduce this architecture in Rust:

1. **Storage Layer**: Use `sqlx` for PostgreSQL with triple store schema
2. **Query Engine**: Implement Datalog or use existing crate like `datafrog`
3. **Sync Protocol**: `tokio-tungstenite` for WebSocket handling
4. **CEL Evaluation**: `cel-rust` crate for permission rules
5. **WAL Tailing**: `postgres` crate with logical replication support
