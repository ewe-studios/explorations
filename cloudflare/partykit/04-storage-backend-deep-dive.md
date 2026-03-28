---
title: "Storage Backend Deep Dive: Durable Storage, Snapshots, and Recovery"
subtitle: "Complete guide to Durable Objects storage, SQL API, persistence patterns, and recovery in PartyKit"
based_on: "PartyServer packages/partyserver/src/index.ts, packages/y-partyserver/src/server/index.ts"
---

# Storage Backend Deep Dive

## Table of Contents

1. [Durable Objects Storage Overview](#1-durable-objects-storage-overview)
2. [SQL Storage API](#2-sql-storage-api)
3. [Key-Value Storage](#3-key-value-storage)
4. [Snapshot Patterns](#4-snapshot-patterns)
5. [Recovery and Hibernation](#5-recovery-and-hibernation)
6. [External Storage Integration](#6-external-storage-integration)
7. [Production Storage Patterns](#7-production-storage-patterns)

---

## 1. Durable Objects Storage Overview

### 1.1 Storage Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Durable Object Storage                      │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │              Durable Object                      │    │
│  │  ┌─────────────────┐  ┌─────────────────────┐   │    │
│  │  │   In-Memory     │  │   SQLite Database   │   │    │
│  │  │   State Cache   │  │   (Persistent)      │   │    │
│  │  │                 │  │                     │   │    │
│  │  │  - Connections  │  │  - Tables           │   │    │
│  │  │  - Room State   │  │  - Indices          │   │    │
│  │  │  - Cursors      │  │  - WAL Log          │   │    │
│  │  └─────────────────┘  └─────────────────────┘   │    │
│  └─────────────────────────────────────────────────┘    │
│                          │                               │
│                          ▼                               │
│  ┌─────────────────────────────────────────────────┐    │
│  │           Cloudflare Storage Layer               │    │
│  │                                                  │    │
│  │  - Replicated across data centers               │    │
│  │  - Survives DO eviction                         │    │
│  │  - Automatic backups                            │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Storage Types

| Storage Type | Use Case | Persistence | Size Limit |
|--------------|----------|-------------|------------|
| **SQLite** | Structured data, queries | Durable | 512 MB |
| **Key-Value** | Simple state, config | Durable | 128 KB |
| **WebSocket Attachment** | Connection state | Session | 2 KB |
| **In-Memory** | Cache, active state | Volatile | Memory limit |

### 1.3 Storage Access

```typescript
export class MyServer extends Server {
  async onStart() {
    // Access storage via context
    const storage = this.ctx.storage;

    // SQL storage (recommended for structured data)
    storage.sql.exec("CREATE TABLE IF NOT EXISTS ...");

    // Key-value storage (for simple state)
    await storage.put("counter", 0);
    const count = await storage.get<number>("counter");

    // List keys
    const keys = await storage.list();
  }
}
```

---

## 2. SQL Storage API

### 2.1 Schema Definition

```typescript
export class ChatServer extends Server {
  async onStart() {
    // Create tables
    this.ctx.storage.sql.exec(`
      CREATE TABLE IF NOT EXISTS messages (
        id TEXT PRIMARY KEY,
        room_id TEXT NOT NULL,
        sender_id TEXT NOT NULL,
        content TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        edited_at INTEGER,
        deleted_at INTEGER
      )
    `);

    this.ctx.storage.sql.exec(`
      CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY,
        username TEXT UNIQUE NOT NULL,
        display_name TEXT,
        avatar_url TEXT,
        created_at INTEGER NOT NULL,
        last_seen INTEGER
      )
    `);

    this.ctx.storage.sql.exec(`
      CREATE TABLE IF NOT EXISTS rooms (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT,
        created_at INTEGER NOT NULL,
        is_private INTEGER DEFAULT 0
      )
    `);

    // Create indices for performance
    this.ctx.storage.sql.exec(`
      CREATE INDEX IF NOT EXISTS idx_messages_room
      ON messages(room_id, created_at DESC)
    `);

    this.ctx.storage.sql.exec(`
      CREATE INDEX IF NOT EXISTS idx_messages_sender
      ON messages(sender_id, created_at DESC)
    `);
  }
}
```

### 2.2 CRUD Operations

```typescript
// CREATE
this.ctx.storage.sql.exec(
  `INSERT INTO messages (id, room_id, sender_id, content, created_at)
   VALUES (?, ?, ?, ?, ?)`,
  crypto.randomUUID(),
  this.name,
  userId,
  content,
  Date.now()
);

// INSERT with RETURNING
const result = this.ctx.storage.sql.exec(
  `INSERT INTO users (id, username, display_name, created_at)
   VALUES (?, ?, ?, ?)
   RETURNING *`,
  userId, username, displayName, Date.now()
).one();

// READ
const messages = this.ctx.storage.sql.exec(
  `SELECT m.*, u.username, u.avatar_url
   FROM messages m
   JOIN users u ON m.sender_id = u.id
   WHERE m.room_id = ? AND m.deleted_at IS NULL
   ORDER BY m.created_at DESC
   LIMIT 50`,
  this.name
).all();

// Update
this.ctx.storage.sql.exec(
  `UPDATE messages
   SET content = ?, edited_at = ?
   WHERE id = ? AND sender_id = ?`,
  newContent, Date.now(), messageId, userId
);

// Soft delete
this.ctx.storage.sql.exec(
  `UPDATE messages
   SET deleted_at = ?
   WHERE id = ?`,
  Date.now(), messageId
);

// Hard delete
this.ctx.storage.sql.exec(
  `DELETE FROM messages WHERE id = ?`,
  messageId
);
```

### 2.3 Transaction Handling

```typescript
// Transactions are implicit in DO (single-threaded)
// All operations within a fetch/message handler are atomic

export class ChatServer extends Server {
  async onMessage(connection: Connection, message: WSMessage) {
    const data = JSON.parse(message as string);

    try {
      // Start of transaction (implicit)

      // Insert message
      const messageId = crypto.randomUUID();
      this.ctx.storage.sql.exec(
        `INSERT INTO messages (id, room_id, sender_id, content, created_at)
         VALUES (?, ?, ?, ?, ?)`,
        messageId, this.name, connection.state.userId, data.content, Date.now()
      );

      // Update user's last message timestamp
      this.ctx.storage.sql.exec(
        `UPDATE users SET last_seen = ? WHERE id = ?`,
        Date.now(), connection.state.userId
      );

      // Update room activity
      this.ctx.storage.sql.exec(
        `UPDATE rooms SET last_activity = ? WHERE id = ?`,
        Date.now(), this.name
      );

      // End of transaction (implicit on handler completion)

      // Broadcast success
      this.broadcast(JSON.stringify({
        type: "message_created",
        id: messageId,
        content: data.content
      }));

    } catch (error) {
      // Transaction rolls back on error
      console.error("Failed to create message:", error);
      connection.send(JSON.stringify({
        type: "error",
        message: "Failed to send message"
      }));
    }
  }
}
```

### 2.4 Query Helpers

```typescript
// Helper method for typed queries
interface Message {
  id: string;
  room_id: string;
  sender_id: string;
  content: string;
  created_at: number;
  username: string;
}

export class ChatServer extends Server {
  getRecentMessages(limit: number = 50): Message[] {
    return this.ctx.storage.sql.exec<Message[]>(
      `SELECT m.id, m.room_id, m.sender_id, m.content, m.created_at,
              u.username
       FROM messages m
       JOIN users u ON m.sender_id = u.id
       WHERE m.room_id = ? AND m.deleted_at IS NULL
       ORDER BY m.created_at DESC
       LIMIT ?`,
      this.name,
      limit
    ).all();
  }

  getMessageCount(): number {
    const result = this.ctx.storage.sql.exec(
      `SELECT COUNT(*) as count FROM messages
       WHERE room_id = ? AND deleted_at IS NULL`,
      this.name
    ).one() as { count: number };
    return result.count;
  }

  searchMessages(query: string, limit: number = 20): Message[] {
    return this.ctx.storage.sql.exec<Message[]>(
      `SELECT m.*, u.username
       FROM messages m
       JOIN users u ON m.sender_id = u.id
       WHERE m.room_id = ?
         AND m.deleted_at IS NULL
         AND m.content LIKE ?
       ORDER BY m.created_at DESC
       LIMIT ?`,
      this.name,
      `%${query}%`,
      limit
    ).all();
  }
}
```

### 2.5 Schema Migrations

```typescript
export class ChatServer extends Server {
  async onStart() {
    const currentVersion = await this.ctx.storage.get<number>("schema_version") ?? 0;

    // Run migrations sequentially
    if (currentVersion < 1) {
      await this.migrateV1();
    }
    if (currentVersion < 2) {
      await this.migrateV2();
    }
    if (currentVersion < 3) {
      await this.migrateV3();
    }

    // Update version
    await this.ctx.storage.put("schema_version", 3);
  }

  private async migrateV1() {
    this.ctx.storage.sql.exec(`
      CREATE TABLE messages (
        id TEXT PRIMARY KEY,
        sender_id TEXT,
        content TEXT,
        created_at INTEGER
      )
    `);
  }

  private async migrateV2() {
    this.ctx.storage.sql.exec(`
      ALTER TABLE messages ADD COLUMN room_id TEXT DEFAULT ''
    `);
    this.ctx.storage.sql.exec(`
      CREATE INDEX idx_messages_room ON messages(room_id, created_at)
    `);
  }

  private async migrateV3() {
    this.ctx.storage.sql.exec(`
      ALTER TABLE messages ADD COLUMN deleted_at INTEGER
    `);
    this.ctx.storage.sql.exec(`
      ALTER TABLE messages ADD COLUMN edited_at INTEGER
    `);
  }
}
```

---

## 3. Key-Value Storage

### 3.1 Basic Operations

```typescript
export class ConfigServer extends Server {
  async onStart() {
    // Put a value
    await this.ctx.storage.put("config", {
      maxUsers: 100,
      allowGuests: true,
      theme: "dark"
    });

    // Get a value
    const config = await this.ctx.storage.get<{ maxUsers: number }>("config");
    console.log("Max users:", config?.maxUsers);

    // Delete a value
    await this.ctx.storage.delete("temp_data");

    // List all keys
    const keys = await this.ctx.storage.list();
    console.log("All keys:", keys);

    // Get multiple values
    const [config, counter, metadata] = await Promise.all([
      this.ctx.storage.get("config"),
      this.ctx.storage.get<number>("counter"),
      this.ctx.storage.get("metadata")
    ]);
  }
}
```

### 3.2 Counter Pattern

```typescript
export class CounterServer extends Server {
  async incrementCounter(key: string, amount: number = 1): Promise<number> {
    // Atomic increment using SQL
    this.ctx.storage.sql.exec(
      `INSERT INTO counters (key, value) VALUES (?, ?)
       ON CONFLICT(key) DO UPDATE SET value = value + ?`,
      key, amount, amount
    );

    const result = this.ctx.storage.sql.exec(
      `SELECT value FROM counters WHERE key = ?`,
      key
    ).one() as { value: number };

    return result.value;
  }

  async getCounter(key: string): Promise<number> {
    const result = await this.ctx.storage.get<number>(`counter:${key}`);
    return result ?? 0;
  }

  async setCounter(key: string, value: number): Promise<void> {
    await this.ctx.storage.put(`counter:${key}`, value);
  }
}
```

### 3.3 Session Storage

```typescript
interface Session {
  userId: string;
  token: string;
  expiresAt: number;
  data: Record<string, unknown>;
}

export class SessionServer extends Server {
  async createSession(userId: string, ttlMs: number): Promise<Session> {
    const session: Session = {
      userId,
      token: crypto.randomUUID(),
      expiresAt: Date.now() + ttlMs,
      data: {}
    };

    await this.ctx.storage.put(`session:${session.token}`, session);

    // Set alarm to cleanup expired sessions
    this.ctx.storage.setAlarm(Date.now() + ttlMs);

    return session;
  }

  async getSession(token: string): Promise<Session | null> {
    const session = await this.ctx.storage.get<Session>(`session:${token}`);

    if (!session) return null;
    if (session.expiresAt < Date.now()) {
      await this.ctx.storage.delete(`session:${token}`);
      return null;
    }

    return session;
  }

  async deleteSession(token: string): Promise<void> {
    await this.ctx.storage.delete(`session:${token}`);
  }

  async onAlarm() {
    // Cleanup expired sessions
    const keys = await this.ctx.storage.list();
    for (const key of keys) {
      if (key.startsWith("session:")) {
        const session = await this.ctx.storage.get<Session>(key);
        if (session && session.expiresAt < Date.now()) {
          await this.ctx.storage.delete(key);
        }
      }
    }
  }
}
```

---

## 4. Snapshot Patterns

### 4.1 Manual Snapshots

```typescript
interface RoomSnapshot {
  version: number;
  timestamp: number;
  messages: Array<{
    id: string;
    sender_id: string;
    content: string;
    created_at: number;
  }>;
  users: Array<{
    id: string;
    username: string;
    joined_at: number;
  }>;
  metadata: Record<string, unknown>;
}

export class SnapshotServer extends Server {
  async createSnapshot(): Promise<RoomSnapshot> {
    const snapshot: RoomSnapshot = {
      version: 1,
      timestamp: Date.now(),
      messages: this.ctx.storage.sql.exec(
        `SELECT id, sender_id, content, created_at
         FROM messages
         WHERE room_id = ? AND deleted_at IS NULL`,
        this.name
      ).all(),
      users: this.ctx.storage.sql.exec(
        `SELECT id, username, created_at as joined_at
         FROM room_users
         WHERE room_id = ?`,
        this.name
      ).all(),
      metadata: await this.ctx.storage.get<Record<string, unknown>>("metadata") ?? {}
    };

    // Store snapshot
    await this.ctx.storage.put("snapshot", snapshot);
    await this.ctx.storage.put(`snapshot:${Date.now()}`, snapshot);  // Versioned

    return snapshot;
  }

  async loadSnapshot(): Promise<RoomSnapshot | null> {
    return await this.ctx.storage.get<RoomSnapshot>("snapshot");
  }

  async loadSnapshotAt(timestamp: number): Promise<RoomSnapshot | null> {
    const key = `snapshot:${timestamp}`;
    return await this.ctx.storage.get<RoomSnapshot>(key);
  }

  async restoreSnapshot(snapshot: RoomSnapshot): Promise<void> {
    // Clear current state
    this.ctx.storage.sql.exec(`DELETE FROM messages WHERE room_id = ?`, this.name);

    // Restore messages
    for (const msg of snapshot.messages) {
      this.ctx.storage.sql.exec(
        `INSERT INTO messages (id, room_id, sender_id, content, created_at)
         VALUES (?, ?, ?, ?, ?)`,
        msg.id, this.name, msg.sender_id, msg.content, msg.created_at
      );
    }

    // Restore metadata
    await this.ctx.storage.put("metadata", snapshot.metadata);
  }
}
```

### 4.2 Automatic Snapshots

```typescript
export class AutoSnapshotServer extends Server {
  private readonly SNAPSHOT_INTERVAL = 5 * 60 * 1000;  // 5 minutes
  private lastSnapshotTime = 0;

  async onMessage(connection: Connection, message: WSMessage) {
    // Process message
    await this.processMessage(connection, message);

    // Check if snapshot needed
    const now = Date.now();
    if (now - this.lastSnapshotTime > this.SNAPSHOT_INTERVAL) {
      await this.createSnapshot();
      this.lastSnapshotTime = now;

      // Schedule next snapshot alarm
      this.ctx.storage.setAlarm(now + this.SNAPSHOT_INTERVAL);
    }
  }

  async onAlarm() {
    // Create snapshot on alarm (periodic or recovery)
    await this.createSnapshot();
    this.lastSnapshotTime = Date.now();
  }

  async onStart() {
    // Load from snapshot if exists
    const snapshot = await this.loadSnapshot();
    if (snapshot) {
      await this.restoreSnapshot(snapshot);
      console.log(`Restored from snapshot at ${snapshot.timestamp}`);
    }

    this.lastSnapshotTime = Date.now();
  }
}
```

### 4.3 Incremental Snapshots

```typescript
interface IncrementalSnapshot {
  baseTimestamp: number;
  operations: Array<{
    type: "insert" | "update" | "delete";
    table: string;
    id: string;
    data?: unknown;
    timestamp: number;
  }>;
}

export class IncrementalSnapshotServer extends Server {
  async logOperation(
    type: "insert" | "update" | "delete",
    table: string,
    id: string,
    data?: unknown
  ) {
    // Append to operation log
    const op = { type, table, id, data, timestamp: Date.now() };

    this.ctx.storage.sql.exec(
      `INSERT INTO operation_log (type, table_name, record_id, data, timestamp)
       VALUES (?, ?, ?, ?, ?)`,
      type, table, id, JSON.stringify(data), Date.now()
    );

    // Trigger snapshot if log is large
    const count = this.ctx.storage.sql.exec(
      `SELECT COUNT(*) as c FROM operation_log WHERE persisted = 0`
    ).one() as { c: number };

    if (count.c > 100) {
      await this.consolidateSnapshot();
    }
  }

  async consolidateSnapshot() {
    // Create full snapshot
    await this.createSnapshot();

    // Mark operations as persisted
    this.ctx.storage.sql.exec(
      `UPDATE operation_log SET persisted = 1 WHERE persisted = 0`
    );

    // Trim old operations
    this.ctx.storage.sql.exec(
      `DELETE FROM operation_log
       WHERE persisted = 1 AND timestamp < ?`,
      Date.now() - 24 * 60 * 60 * 1000  // Keep 24 hours
    );
  }

  async recoverFromOperations() {
    // Load base snapshot
    const snapshot = await this.loadSnapshot();
    if (snapshot) {
      await this.restoreSnapshot(snapshot);
    }

    // Replay operations since snapshot
    const operations = this.ctx.storage.sql.exec(
      `SELECT * FROM operation_log
       WHERE timestamp > ?
       ORDER BY timestamp`,
      snapshot?.timestamp ?? 0
    ).all();

    for (const op of operations) {
      await this.replayOperation(op);
    }
  }
}
```

### 4.4 Yjs Snapshots

```typescript
export class YjsSnapshotServer extends YServer {
  async onSave() {
    // Encode Yjs document as binary update
    const update = Y.encodeStateAsUpdate(this.document);

    // Store in SQL as blob
    this.ctx.storage.sql.exec(
      `INSERT OR REPLACE INTO yjs_snapshots (room_id, update_data, timestamp)
       VALUES (?, ?, ?)`,
      this.name, update, Date.now()
    );

    console.log(`Saved Yjs snapshot for room ${this.name}`);
  }

  async onLoad() {
    // Load latest snapshot
    const result = this.ctx.storage.sql.exec(
      `SELECT update_data FROM yjs_snapshots
       WHERE room_id = ?
       ORDER BY timestamp DESC
       LIMIT 1`,
      this.name
    ).one() as { update_data: Uint8Array } | undefined;

    if (result?.update_data) {
      // Apply snapshot to document
      Y.applyUpdate(this.document, result.update_data);
      console.log(`Loaded Yjs snapshot for room ${this.name}`);
    }
  }

  // Create checkpoint at specific version
  async createCheckpoint(version: number) {
    const update = Y.encodeStateAsUpdate(this.document);

    this.ctx.storage.sql.exec(
      `INSERT INTO yjs_checkpoints (room_id, version, update_data, timestamp)
       VALUES (?, ?, ?, ?)`,
      this.name, version, update, Date.now()
    );
  }

  // Restore to specific checkpoint
  async restoreToCheckpoint(version: number) {
    const result = this.ctx.storage.sql.exec(
      `SELECT update_data FROM yjs_checkpoints
       WHERE room_id = ? AND version = ?`,
      this.name, version
    ).one() as { update_data: Uint8Array } | undefined;

    if (result?.update_data) {
      // Use replaceDocument to restore to exact state
      this.unstable_replaceDocument(result.update_data);
    }
  }
}
```

---

## 5. Recovery and Hibernation

### 5.1 Hibernation Wake-Up

```typescript
export class HibernationServer extends Server {
  static options = { hibernate: true };

  async onStart(props?: Props) {
    // This is called when:
    // 1. First connection establishes
    // 2. DO wakes from hibernation
    // 3. Alarm fires

    console.log(`Server ${this.name} starting/waking up`);

    // Restore state from storage
    await this.restoreState();

    // Re-sync with connected clients
    // (They may have state we need after hibernation)
    await this.resyncWithClients();
  }

  private async restoreState() {
    // Load from snapshot
    const snapshot = await this.ctx.storage.get("snapshot");
    if (snapshot) {
      this.state = snapshot;
    }

    // Or load from SQL
    const messages = this.ctx.storage.sql.exec(
      `SELECT * FROM messages ORDER BY created_at DESC LIMIT 100`
    ).all();
    this.recentMessages = messages;
  }

  private async resyncWithClients() {
    // After hibernation, send sync request to all connections
    for (const connection of this.getConnections()) {
      connection.send(JSON.stringify({
        type: "resync_request",
        serverTimestamp: Date.now()
      }));
    }
  }

  async webSocketMessage(ws: WebSocket, message: WSMessage) {
    // This is called even when hibernating
    const connection = createLazyConnection(ws);
    await this.#ensureInitialized();
    return this.onMessage(connection, message);
  }
}
```

### 5.2 Crash Recovery

```typescript
export class RecoveryServer extends Server {
  async onStart() {
    // Check for incomplete operations
    const pendingOps = this.ctx.storage.sql.exec(
      `SELECT * FROM pending_operations WHERE completed = 0`
    ).all();

    for (const op of pendingOps) {
      try {
        await this.replayOperation(op);
      } catch (error) {
        console.error("Failed to replay operation:", error);
        // Mark as failed
        this.ctx.storage.sql.exec(
          `UPDATE pending_operations SET status = 'failed' WHERE id = ?`,
          op.id
        );
      }
    }

    // Verify state consistency
    await this.verifyStateConsistency();
  }

  private async verifyStateConsistency() {
    // Run consistency checks
    const messageCount = this.ctx.storage.sql.exec(
      `SELECT COUNT(*) as c FROM messages`
    ).one() as { c: number };

    const logCount = this.ctx.storage.sql.exec(
      `SELECT COUNT(*) as c FROM operation_log WHERE persisted = 1`
    ).one() as { c: number };

    if (messageCount.c !== logCount.c) {
      console.warn(`State inconsistency: ${messageCount.c} messages vs ${logCount.c} log entries`);
      // Trigger recovery procedure
      await this.reconcileState();
    }
  }

  private async reconcileState() {
    // Rebuild state from operation log
    this.ctx.storage.sql.exec(`DELETE FROM messages`);

    const operations = this.ctx.storage.sql.exec(
      `SELECT * FROM operation_log
       WHERE type = 'insert' AND table_name = 'messages'
       ORDER BY timestamp`
    ).all();

    for (const op of operations) {
      const data = JSON.parse(op.data as string);
      this.ctx.storage.sql.exec(
        `INSERT OR IGNORE INTO messages (id, room_id, sender_id, content, created_at)
         VALUES (?, ?, ?, ?, ?)`,
        data.id, data.room_id, data.sender_id, data.content, data.created_at
      );
    }
  }
}
```

### 5.3 Alarm-Based Recovery

```typescript
export class AlarmServer extends Server {
  async onAlarm() {
    // Alarms fire even when DO is hibernating
    // Use for periodic tasks and recovery

    const connectionCount = this.getConnections().length;

    if (connectionCount === 0) {
      // No active connections - good time for maintenance

      // 1. Create snapshot
      await this.createSnapshot();

      // 2. Cleanup old data
      await this.cleanupOldData();

      // 3. Verify state
      await this.verifyState();

      // 4. Set next alarm
      this.ctx.storage.setAlarm(Date.now() + 60 * 60 * 1000);  // 1 hour
    } else {
      // Set alarm for later
      this.ctx.storage.setAlarm(Date.now() + 5 * 60 * 1000);  // 5 minutes
    }
  }

  private async cleanupOldData() {
    const thirtyDaysAgo = Date.now() - 30 * 24 * 60 * 60 * 1000;

    // Delete old messages
    this.ctx.storage.sql.exec(
      `DELETE FROM messages WHERE created_at < ?`,
      thirtyDaysAgo
    );

    // Delete old snapshots (keep last 5)
    this.ctx.storage.sql.exec(`
      DELETE FROM snapshots
      WHERE timestamp NOT IN (
        SELECT timestamp FROM snapshots
        ORDER BY timestamp DESC
        LIMIT 5
      )
    `);

    console.log("Cleanup completed");
  }
}
```

---

## 6. External Storage Integration

### 6.1 Loading from External Storage

```typescript
export class ExternalStorageServer extends Server {
  async onLoad() {
    // Load initial state from external storage (R2, S3, external DB)

    // Example: Load from R2
    const r2Key = `rooms/${this.name}/state.json`;
    const object = await this.env.ROOM_BUCKET.get(r2Key);

    if (object) {
      const state = await object.json();
      await this.restoreState(state);
    }

    // Example: Load from external API
    const response = await fetch(`https://api.example.com/rooms/${this.name}`);
    if (response.ok) {
      const state = await response.json();
      await this.restoreState(state);
    }
  }

  async onSave() {
    // Persist to external storage

    const state = await this.createSnapshot();

    // Save to R2
    await this.env.ROOM_BUCKET.put(
      `rooms/${this.name}/state.json`,
      JSON.stringify(state)
    );

    // Save to external API
    await fetch(`https://api.example.com/rooms/${this.name}`, {
      method: "PUT",
      body: JSON.stringify(state),
      headers: { "Content-Type": "application/json" }
    });
  }
}
```

### 6.2 Hybrid Storage Pattern

```typescript
// Use DO storage for hot data, external for cold/archive
export class HybridStorageServer extends Server {
  private readonly HOT_WINDOW = 24 * 60 * 60 * 1000;  // 24 hours

  async onStart() {
    // Load recent data into DO storage
    const recentMessages = await this.fetchFromExternal(
      `messages?room=${this.name}&since=${Date.now() - this.HOT_WINDOW}`
    );

    for (const msg of recentMessages) {
      this.ctx.storage.sql.exec(
        `INSERT OR IGNORE INTO messages (id, room_id, sender_id, content, created_at)
         VALUES (?, ?, ?, ?, ?)`,
        msg.id, this.name, msg.sender_id, msg.content, msg.created_at
      );
    }
  }

  async archiveOldData() {
    const threshold = Date.now() - this.HOT_WINDOW;

    // Get old messages
    const oldMessages = this.ctx.storage.sql.exec(
      `SELECT * FROM messages WHERE created_at < ?`,
      threshold
    ).all();

    // Archive to external storage
    await this.saveToExternal("messages/archive", oldMessages);

    // Delete from DO storage
    this.ctx.storage.sql.exec(
      `DELETE FROM messages WHERE created_at < ?`,
      threshold
    );
  }

  async searchAllMessages(query: string) {
    // Search hot data in DO
    const hotResults = this.ctx.storage.sql.exec(
      `SELECT * FROM messages
       WHERE content LIKE ?
       ORDER BY created_at DESC
       LIMIT 50`,
      `%${query}%`
    ).all();

    // Search cold data externally
    const coldResults = await this.fetchFromExternal(
      `messages/search?room=${this.name}&q=${encodeURIComponent(query)}`
    );

    return [...hotResults, ...coldResults];
  }
}
```

### 6.3 Multi-DO Coordination

```typescript
// When you need to coordinate state across multiple DOs
export class CoordinatorServer extends Server {
  async broadcastToAllRooms(message: string) {
    // Get all room names from shared storage
    const rooms = await this.env.SHARED_KV.list({ prefix: "room:" });

    for (const key of rooms.keys) {
      const roomName = key.name.replace("room:", "");
      const stub = await getServerByName(this.env.RoomServer, roomName);

      // Send message via fetch
      await stub.fetch(new Request("http://internal/broadcast", {
        method: "POST",
        body: message
      }));
    }
  }

  async getGlobalState() {
    // Aggregate state from all rooms
    const rooms = await this.env.SHARED_KV.list({ prefix: "room:" });
    const states = [];

    for (const key of rooms.keys) {
      const roomName = key.name.replace("room:", "");
      const stub = await getServerByName(this.env.RoomServer, roomName);

      const response = await stub.fetch("http://internal/state");
      const state = await response.json();
      states.push({ room: roomName, ...state });
    }

    return { rooms: states };
  }
}
```

---

## 7. Production Storage Patterns

### 7.1 Connection Pooling Simulation

```typescript
// DO storage is per-instance, but you can simulate pooling
export class PooledStorageServer extends Server {
  private queryCache = new Map<string, { result: unknown; timestamp: number }>();
  private readonly CACHE_TTL = 5000;  // 5 seconds

  async executeCachedQuery<T>(
    key: string,
    query: string,
    params: unknown[]
  ): Promise<T> {
    // Check cache
    const cached = this.queryCache.get(key);
    if (cached && Date.now() - cached.timestamp < this.CACHE_TTL) {
      return cached.result as T;
    }

    // Execute query
    const result = this.ctx.storage.sql.exec(query, ...params).all() as T;

    // Update cache
    this.queryCache.set(key, { result, timestamp: Date.now() });

    return result;
  }

  invalidateCache(prefix?: string) {
    if (prefix) {
      for (const key of this.queryCache.keys()) {
        if (key.startsWith(prefix)) {
          this.queryCache.delete(key);
        }
      }
    } else {
      this.queryCache.clear();
    }
  }
}
```

### 7.2 Rate-Limited Writes

```typescript
export class RateLimitedServer extends Server {
  private readonly WRITE_LIMIT = 100;  // per minute
  private writeTimestamps: number[] = [];

  async canWrite(): Promise<boolean> {
    const now = Date.now();
    const minuteAgo = now - 60 * 1000;

    // Remove old timestamps
    this.writeTimestamps = this.writeTimestamps.filter(t => t > minuteAgo);

    return this.writeTimestamps.length < this.WRITE_LIMIT;
  }

  async recordWrite() {
    this.writeTimestamps.push(Date.now());
  }

  async throttledWrite(query: string, params: unknown[]) {
    if (!await this.canWrite()) {
      throw new Error("Write rate limit exceeded");
    }

    this.ctx.storage.sql.exec(query, ...params);
    await this.recordWrite();
  }
}
```

### 7.3 Storage Monitoring

```typescript
export class MonitoredServer extends Server {
  async getStorageStats() {
    const messageCount = this.ctx.storage.sql.exec(
      `SELECT COUNT(*) as c FROM messages`
    ).one() as { c: number };

    const keys = await this.ctx.storage.list();

    return {
      messageCount: messageCount.c,
      kvKeyCount: keys.length,
      connectionCount: this.getConnections().length,
      lastSnapshot: await this.ctx.storage.get<number>("last_snapshot_time")
    };
  }

  async onRequest(request: Request): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === "/api/stats" && request.method === "GET") {
      const stats = await this.getStorageStats();
      return Response.json(stats);
    }

    return new Response("Not Found", { status: 404 });
  }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial storage backend deep dive created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
