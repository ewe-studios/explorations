---
title: "State Sync Deep Dive: CRDTs, Operational Transforms, and Consistency"
subtitle: "Complete guide to state synchronization patterns, Yjs integration, and consistency models in PartyKit"
based_on: "PartyServer packages/y-partyserver/src/server/index.ts, packages/partysync/src/server/index.ts"
---

# State Sync Deep Dive

## Table of Contents

1. [The State Sync Problem](#1-the-state-sync-problem)
2. [CRDT Fundamentals](#2-crdt-fundamentals)
3. [Yjs Implementation](#3-yjs-implementation)
4. [Sync Protocol Deep Dive](#4-sync-protocol-deep-dive)
5. [Operational Transforms](#5-operational-transforms)
6. [PartySync for Database Sync](#6-partysync-for-database-sync)
7. [Consistency Models](#7-consistency-models)

---

## 1. The State Sync Problem

### 1.1 The Challenge

When multiple clients modify shared state concurrently:

```
Time ──────────────────────────────────────────►

Client A:  "Hello" ──► "Hello World"
                      (inserts " World" at position 5)

Client B:  "Hello" ──► "Helxlo"
                      (inserts "x" at position 3)

Without proper sync:
- Client A sees: "Helxlo World" ✓
- Client B sees: "Hello Worxld" ✗ (inconsistent!)
```

### 1.2 Requirements for State Sync

| Requirement | Description |
|-------------|-------------|
| **Consistency** | All clients converge to the same state |
| **Concurrency** | Multiple clients can edit simultaneously |
| **Causality** | Causally related operations maintain order |
| **Efficiency** | Minimal bandwidth and computation |
| **Persistence** | State survives disconnections |

### 1.3 Naive Approaches (and Why They Fail)

**Last Write Wins:**
```typescript
// PROBLEM: Lost updates
// T0: State = "Hello"
// T1: Client A sets state = "Hello World"
// T2: Client B sets state = "Hello There"
// Result: Only B's change survives, A's work lost
```

**Central Authority:**
```typescript
// PROBLEM: Single point of failure, high latency
// All edits must round-trip to server
// Server becomes bottleneck, can't work offline
```

**Lock-Based:**
```typescript
// PROBLEM: Poor UX, deadlock potential
// Only one client can edit at a time
// Lock conflicts, timeout handling complexity
```

---

## 2. CRDT Fundamentals

### 2.1 What are CRDTs?

**CRDT** (Conflict-Free Replicated Data Type) is a data structure that:
- Can be replicated across multiple clients
- Allows concurrent modifications
- Automatically converges to the same state

### 2.2 Mathematical Properties

```typescript
// Commutativity: order doesn't matter
op1 ∘ op2 = op2 ∘ op1

// Associativity: grouping doesn't matter
(op1 ∘ op2) ∘ op3 = op1 ∘ (op2 ∘ op3)

// Idempotency: applying twice has no extra effect
op ∘ op = op
```

### 2.3 CRDT Types

#### G-Counter (Grow-only Counter)
```typescript
class GCounter {
  private counts: Map<string, number> = new Map();

  increment(nodeId: string, amount: number = 1) {
    const current = this.counts.get(nodeId) ?? 0;
    this.counts.set(nodeId, current + amount);
  }

  value(): number {
    let sum = 0;
    for (const count of this.counts.values()) {
      sum += count;
    }
    return sum;
  }

  // Merge is just taking max of each counter
  merge(other: GCounter) {
    for (const [nodeId, count] of other.counts.entries()) {
      const current = this.counts.get(nodeId) ?? 0;
      this.counts.set(nodeId, Math.max(current, count));
    }
  }
}
```

#### PN-Counter (Positive-Negative Counter)
```typescript
class PNCounter {
  private positive = new GCounter();
  private negative = new GCounter();

  increment(amount: number = 1) {
    this.positive.increment("local", amount);
  }

  decrement(amount: number = 1) {
    this.negative.increment("local", amount);
  }

  value(): number {
    return this.positive.value() - this.negative.value();
  }

  merge(other: PNCounter) {
    this.positive.merge(other.positive);
    this.negative.merge(other.negative);
  }
}
```

#### LWW-Register (Last-Writer-Wins Register)
```typescript
class LWWRegister<T> {
  private value: T | null = null;
  private timestamp: number = 0;

  set(newValue: T, timestamp: number) {
    if (timestamp > this.timestamp) {
      this.value = newValue;
      this.timestamp = timestamp;
    }
  }

  get(): T | null {
    return this.value;
  }

  merge(other: LWWRegister<T>) {
    if (other.timestamp > this.timestamp) {
      this.value = other.value;
      this.timestamp = other.timestamp;
    }
  }
}
```

#### OR-Set (Observed-Remove Set)
```typescript
class ORSet<T> {
  private elements: Map<T, Set<string>> = new Map();
  private tombstones: Map<T, Set<string>> = new Map();
  private nodeId: string;

  add(element: T) {
    const uniqueId = `${this.nodeId}:${Date.now()}`;
    if (!this.elements.has(element)) {
      this.elements.set(element, new Set());
    }
    this.elements.get(element)!.add(uniqueId);
  }

  delete(element: T) {
    if (this.elements.has(element)) {
      const tags = this.elements.get(element)!;
      this.tombstones.set(element, new Set([...tags]));
      this.elements.delete(element);
    }
  }

  has(element: T): boolean {
    return this.elements.has(element);
  }

  merge(other: ORSet<T>) {
    // Merge elements
    for (const [element, tags] of other.elements.entries()) {
      if (!this.elements.has(element)) {
        this.elements.set(element, new Set());
      }
      const localTags = this.elements.get(element)!;
      for (const tag of tags) {
        localTags.add(tag);
      }
    }

    // Apply tombstones
    for (const [element, tombTags] of other.tombstones.entries()) {
      if (this.elements.has(element)) {
        const localTags = this.elements.get(element)!;
        for (const tag of tombTags) {
          localTags.delete(tag);
        }
        if (localTags.size === 0) {
          this.elements.delete(element);
        }
      }
    }
  }
}
```

### 2.4 CRDT vs. OT

| Aspect | CRDT | Operational Transform |
|--------|------|----------------------|
| Convergence | Automatic (mathematical) | Requires transformation functions |
| History | Maintains full history | May discard old operations |
| Text Editing | Good (RGA, Logoot) | Excellent (established algorithms) |
| Complexity | Lower (easier to implement) | Higher (transformation matrix) |
| Adoption | Growing (Yjs, Automerge) | Established (Google Docs) |

---

## 3. Yjs Implementation

### 3.1 Yjs Overview

Yjs is a high-performance CRDT implementation with:
- Rich data types (Text, Array, Map, XML)
- Efficient binary encoding
- Built-in awareness protocol
- Undo/Redo support

### 3.2 YServer Setup

```typescript
import { YServer } from "y-partyserver";
import * as Y from "yjs";

export class DocServer extends YServer {
  async onLoad() {
    // Load Yjs document from external storage
    const update = await fetchExternalUpdate(this.name);
    if (update) {
      Y.applyUpdate(this.document, update);
    }
  }

  async onSave() {
    // Persist document state
    const update = Y.encodeStateAsUpdate(this.document);
    await saveExternalUpdate(this.name, update);
  }
}
```

### 3.3 Yjs Data Types

```typescript
// Text - collaborative text editing
const text = document.getText("content");
text.insert(0, "Hello");
text.insert(5, " World");
text.delete(0, 6); // Delete "Hello "

// Observe changes
text.observe((event) => {
  console.log("Text changed:", text.toString());
  console.log("Delta:", event.changes);
});

// Array - collaborative lists
const array = document.getArray("items");
array.push(["item1", "item2"]);
array.insert(1, ["new-item"]);
array.delete(0, 1);

// Map - collaborative key-value store
const map = document.getMap("settings");
map.set("theme", "dark");
map.set("fontSize", 14);
map.delete("theme");

// Observe map changes
map.observe((event) => {
  for (const key of event.keysChanged) {
    console.log(`Key ${key} changed:`, map.get(key));
  }
});

// XmlElement - structured content
const xml = document.getXmlElement("content");
const child = new Y.XmlElement("paragraph");
child.insert(0, [new Y.XmlText("Hello")]);
xml.insert(0, [child]);
```

### 3.4 Client-Side Provider

```typescript
import YProvider from "y-partyserver/provider";
import * as Y from "yjs";

const doc = new Y.Doc();
const provider = new YProvider(
  "localhost:8787",  // Host
  "my-document",     // Room/document name
  doc,               // Yjs document
  {
    connect: true,                    // Auto-connect
    party: "main",                    // Party name
    awareness: new Awareness(doc),   // Custom awareness
    params: { token: "auth-token" }, // Query params
    maxBackoffTimeout: 2500,         // Reconnection settings
    disableBc: false                 // Cross-tab communication
  }
);

// Access shared types
const text = doc.getText("content");
const map = doc.getMap("settings");

// React integration
import useYProvider from "y-partyserver/react";

function App() {
  const provider = useYProvider({
    host: "localhost:8787",
    room: "my-document",
    doc: myDoc
  });
}
```

### 3.5 Awareness Protocol

```typescript
// Set local awareness state
provider.awareness.setLocalStateField("cursor", { x: 100, y: 200 });
provider.awareness.setLocalStateField("selection", { start: 0, end: 10 });
provider.awareness.setLocalStateField("user", {
  id: "user-123",
  name: "Alice",
  color: "#ff0000"
});

// Observe awareness changes
provider.awareness.on("change", ({ added, updated, removed }) => {
  // New users
  for (const clientId of added) {
    const state = provider.awareness.getState(clientId);
    console.log(`User ${state.user.name} connected`);
  }

  // Updated states (cursor movement, etc.)
  for (const clientId of updated) {
    const state = provider.awareness.getState(clientId);
    console.log(`User ${state.user.name} moved cursor to`, state.cursor);
  }

  // Disconnected users
  for (const clientId of removed) {
    console.log(`User ${clientId} disconnected`);
  }
});

// Get all awareness states
const allStates = provider.awareness.getStates();
for (const [clientId, state] of allStates.entries()) {
  console.log(`Client ${clientId}:`, state);
}
```

### 3.6 Custom Messages

```typescript
// Server-side: handle custom messages
export class MyYServer extends YServer {
  onCustomMessage(connection: Connection, message: string) {
    const data = JSON.parse(message);

    if (data.action === "ping") {
      // Send response to specific client
      this.sendCustomMessage(
        connection,
        JSON.stringify({ action: "pong", timestamp: Date.now() })
      );

      // Or broadcast to all
      this.broadcastCustomMessage(
        JSON.stringify({ action: "notification", data: "Someone pinged!" })
      );
    }
  }
}

// Client-side: send custom messages
provider.sendMessage(JSON.stringify({ action: "ping" }));
provider.on("custom-message", (message: string) => {
  const data = JSON.parse(message);
  console.log("Custom message:", data);
});
```

### 3.7 Callback Options

```typescript
export class DocServer extends YServer {
  static callbackOptions = {
    debounceWait: 2000,      // Wait 2s after last edit before saving
    debounceMaxWait: 10000,  // Force save after 10s regardless
    timeout: 5000            // Timeout for save operation
  };

  async onSave() {
    // Called with debouncing
    const update = Y.encodeStateAsUpdate(this.document);
    await saveToDatabase(this.name, update);
  }
}
```

---

## 4. Sync Protocol Deep Dive

### 4.1 Yjs Sync Protocol

The Yjs sync protocol uses a three-step handshake:

```
┌─────────────────────────────────────────────────────────┐
│               Yjs Sync Protocol                          │
│                                                          │
│  Client                      Server                      │
│     │                           │                        │
│     │── Step 1: State Vector ──>│                       │
│     │   "I have updates: 1,3,5" │                       │
│     │                           │                       │
│     │<── Step 2: Missing Updates ─                      │
│     │   "Here are updates: 2,4,6"│                      │
│     │                           │                       │
│     │── Step 2: Missing Updates ─>│                     │
│     │   "Here are updates: 7,8"  │                      │
│     │                           │                       │
│     │<── Step 2: Missing Updates ─                      │
│     │   "Here are updates: 9,10" │                      │
│     │                           │                       │
│     │───── Now in Sync ────────>│                       │
│                                                          │
│  After sync:                                             │
│  - All future updates are broadcast immediately         │
│  - Bidirectional update propagation                     │
└─────────────────────────────────────────────────────────┘
```

### 4.2 Sync Step 1 - State Vector

```typescript
// Server sends state vector (summary of known updates)
function writeSyncStep1(encoder: Encoder, doc: YDoc) {
  encoding.writeVarUint(encoder, messageYjsSyncStep1);
  const stateVector = encodeStateVector(doc);
  encoding.writeVarUint8Array(encoder, stateVector);
}

// Client responds with missing updates
function readSyncStep1(decoder: Decoder, encoder: Encoder, doc: YDoc) {
  const stateVector = decoding.readVarUint8Array(decoder);
  const missingUpdates = encodeStateAsUpdate(doc, stateVector);
  writeSyncStep2(encoder, missingUpdates);
}
```

### 4.3 Sync Step 2 - Updates

```typescript
// Send actual content updates
function writeSyncStep2(encoder: Encoder, updates: Uint8Array) {
  encoding.writeVarUint(encoder, messageYjsSyncStep2);
  encoding.writeVarUint8Array(encoder, updates);
}

// Apply received updates
function readSyncStep2(decoder: Decoder, doc: YDoc, conn: Connection) {
  const update = decoding.readVarUint8Array(decoder);
  Y.applyUpdate(doc, update, conn);
}

// Handle incremental updates
function readUpdate(decoder: Decoder, doc: YDoc, conn: Connection) {
  const update = decoding.readVarUint8Array(decoder);
  Y.applyUpdate(doc, update, conn);
}
```

### 4.4 Binary Message Format

```typescript
// Message types
const messageSync = 0;       // Yjs sync protocol
const messageAwareness = 1;  // Awareness protocol
const messageAuth = 2;       // Authentication (reserved)

// Sync message structure
// [messageType, ...payload]

// Sync Step 1: [0, stateVector...]
// Sync Step 2: [1, update...]
// Awareness:   [2, awarenessUpdate...]
```

### 4.5 Reconnection Handling

```typescript
// On reconnect, client and server resync
// Server checks what updates client is missing
// Client requests and applies missing updates

// Hibernation wake-up scenario
async onStart() {
  // After hibernation, doc might be empty
  // Resync by sending sync step 1 to all connections
  const encoder = encoding.createEncoder();
  encoding.writeVarUint(encoder, messageSync);
  syncProtocol.writeSyncStep1(encoder, this.document);
  const message = encoding.toUint8Array(encoder);

  for (const conn of this.getConnections()) {
    send(conn, message);
  }
}
```

### 4.6 Snapshot and Restore

```typescript
// Get complete document state as snapshot
const snapshot = Y.encodeStateAsUpdate(document);

// Restore from snapshot
const newDoc = new Y.Doc();
Y.applyUpdate(newDoc, snapshot);

// Partial snapshots (since a specific state vector)
const partialUpdate = Y.encodeStateAsUpdate(doc, sinceStateVector);

// Replace document with snapshot (using UndoManager trick)
function unstable_replaceDocument(
  doc: YDoc,
  snapshotUpdate: Uint8Array,
  getMetadata: (key: string) => YjsRootType = () => "Map"
) {
  const currentStateVector = encodeStateVector(doc);
  const snapshotDoc = new Y.Doc();
  applyUpdate(snapshotDoc, snapshotUpdate);

  // Get changes between current and snapshot
  const changesSinceSnapshot = encodeStateAsUpdate(doc, encodeStateVector(snapshotDoc));

  // Create UndoManager for snapshot doc
  const undoManager = new UndoManager(
    [...snapshotDoc.share.keys()].map(key => {
      const type = getMetadata(key);
      if (type === "Text") return snapshotDoc.getText(key);
      if (type === "Map") return snapshotDoc.getMap(key);
      if (type === "Array") return snapshotDoc.getArray(key);
      throw new Error(`Unknown type: ${type}`);
    }),
    { trackedOrigins: new Set([snapshotOrigin]) }
  );

  // Apply changes and undo - effectively replacing content
  applyUpdate(snapshotDoc, changesSinceSnapshot, snapshotOrigin);
  undoManager.undo();

  // Get document state after replacement
  const documentChanges = encodeStateAsUpdate(snapshotDoc, currentStateVector);
  applyUpdate(doc, documentChanges);
}
```

---

## 5. Operational Transforms

### 5.1 OT Basics

Operational Transform (OT) is an alternative to CRDTs:

```
Initial: "Hello"

Client A: Insert " World" at position 5
Client B: Insert "x" at position 3

Without transformation:
- A applies: "Hello World"
- B applies: "Helxlo World"  ✗ Wrong!

With transformation:
- Server receives A's op first
- B's op is transformed: Insert "x" at position 3 (unchanged)
- Transformed B op sent to A: Insert "x" at position 3
- A applies transformed: "Helxlo World" ✓
- B applies A's op (transformed): Insert " World" at position 8
- B result: "Helxlo World" ✓
```

### 5.2 Text Operations

```typescript
type TextOp =
  | { type: "insert"; position: number; text: string }
  | { type: "delete"; position: number; length: number }
  | { type: "retain"; length: number };  // Skip over text

// Transform two concurrent operations
function transform(op1: TextOp, op2: TextOp): TextOp {
  if (op1.type === "insert" && op2.type === "insert") {
    // Both inserts
    if (op1.position <= op2.position) {
      // op1 happens first, shift op2
      return { ...op2, position: op2.position + op1.text.length };
    } else {
      return op2; // op2 unchanged
    }
  }

  if (op1.type === "insert" && op2.type === "delete") {
    // Insert vs delete
    if (op2.position <= op1.position) {
      // Delete before insert, shift insert
      return { ...op1, position: op1.position - op2.length };
    } else if (op2.position < op1.position + op1.text.length) {
      // Delete overlaps with insert
      return { ...op1, text: op1.text.slice(0, op2.position - op1.position) };
    } else {
      return op1; // No overlap
    }
  }

  // ... more transform rules for delete/delete, etc.
}
```

### 5.3 OT vs. CRDT Comparison

| Aspect | CRDT (Yjs) | OT |
|--------|------------|-----|
| **Complexity** | Lower - automatic convergence | Higher - need transform functions |
| **Memory** | Higher - maintains history | Lower - can garbage collect |
| **Text Performance** | Good | Excellent (optimized) |
| **Implementation** | Easier | Harder (many edge cases) |
| **Offline Support** | Excellent | Good |
| **Adoption** | Growing (modern apps) | Established (Google Docs) |

---

## 6. PartySync for Database Sync

### 6.1 PartySync Overview

PartySync synchronizes database state between DO and clients:

```typescript
// Server-side
import { SyncServer } from "partysync";

export class TodoServer extends SyncServer<Env, { todos: [TodoRecord, TodoAction] }> {
  onStart() {
    this.ctx.storage.sql.exec(`
      CREATE TABLE IF NOT EXISTS todos (
        id TEXT PRIMARY KEY,
        text TEXT NOT NULL,
        completed INTEGER NOT NULL,
        created_at INTEGER,
        updated_at INTEGER,
        deleted_at INTEGER
      )
    `);
  }

  onAction(channel: "todos", action: TodoAction) {
    switch (action.type) {
      case "create": {
        const { id, text, completed } = action.payload;
        return this.ctx.storage.sql
          .exec(
            "INSERT INTO todos (id, text, completed, created_at, updated_at) VALUES (?, ?, ?, ?, ?) RETURNING *",
            id, text, completed, Date.now(), Date.now()
          )
          .raw() as TodoRecord[];
      }
      case "update": {
        const { id, text, completed } = action.payload;
        return this.ctx.storage.sql
          .exec(
            "UPDATE todos SET text = ?, completed = ?, updated_at = ? WHERE id = ? RETURNING *",
            text, completed, Date.now(), id
          )
          .raw() as TodoRecord[];
      }
      case "delete": {
        return this.ctx.storage.sql
          .exec(
            "UPDATE todos SET deleted_at = ?, updated_at = ? WHERE id = ?",
            Date.now(), Date.now(), action.payload.id
          )
          .raw() as TodoRecord[];
      }
    }
  }
}
```

### 6.2 Client-Side Sync Hook

```typescript
import { useSync } from "partysync/react";

function TodoApp() {
  const [todos, sendAction] = useSync<TodoRecord, TodoAction>(
    "todos",      // Channel name
    socket,       // WebSocket connection
    // Optimistic update function
    (todos, action) => {
      switch (action.type) {
        case "create": {
          const { id, text, completed } = action.payload;
          return [...todos, [id, text, completed, Date.now(), Date.now(), null]];
        }
        case "update": {
          return todos.map(todo =>
            todo[0] === action.payload.id
              ? [...todo.slice(0, 1), action.payload.text, action.payload.completed, todo[3], Date.now(), todo[5]] as TodoRecord
              : todo
          );
        }
        case "delete": {
          return todos.map(todo =>
            todo[0] === action.payload.id
              ? [...todo.slice(0, 5), Date.now()] as TodoRecord
              : todo
          );
        }
      }
    }
  );

  return (
    <div>
      {todos.map(([id, text, completed]) => (
        <div key={id}>{text} {completed ? "✓" : ""}</div>
      ))}
      <button onClick={() => sendAction({
        type: "create",
        payload: { id: crypto.randomUUID(), text: "New todo", completed: 0 }
      })}>
        Add Todo
      </button>
    </div>
  );
}
```

### 6.3 Sync Protocol

```
┌─────────────────────────────────────────────────────────┐
│                  PartySync Protocol                      │
│                                                          │
│  Client                      Server                      │
│     │                           │                        │
│     │──── Subscribe "todos" ───>│                       │
│     │                           │                       │
│     │<── Initial Records ────── │                       │
│     │   [all current records]   │                       │
│     │                           │                       │
│     │──── Action: Create ──────>│                       │
│     │   {type: "create", ...}   │                       │
│     │                           │                       │
│     │<── Changed Records ──────│                       │
│     │   [newly created record]  │                       │
│     │                           │                       │
│     │<── Broadcast to others ──>│                       │
│     │   [changed records]       │                       │
│                                                          │
│  Features:                                               │
│  - Optimistic updates on client                         │
│  - Server validates and persists                        │
│  - Changed records broadcast to all subscribers         │
│  - Soft deletes (deleted_at column)                     │
└─────────────────────────────────────────────────────────┘
```

---

## 7. Consistency Models

### 7.1 Eventual Consistency

```typescript
// All clients eventually converge
// Temporary divergence is acceptable

// Timeline:
// T0: Server state = "A"
// T1: Client 1 sees "A", Client 2 sees "A"
// T2: Client 1 changes to "B", Client 2 changes to "C"
// T3: Server receives both, merges to "BC"
// T4: All clients converge to "BC"
```

### 7.2 Strong Consistency

```typescript
// All clients see same state at all times
// Requires coordination (slower)

// PartyServer with DO provides strong consistency within a room:
// - Single DO instance per room
// - All operations serialized through DO
// - No race conditions
```

### 7.3 Causal Consistency

```typescript
// Causally related operations maintain order

// Example:
// Client A: Post message "Hello"
// Client B: Reply to message

// Causal order preserved:
// - Reply always appears after the original message
// - Even if clients see updates at different times
```

### 7.4 Choosing a Consistency Model

| Use Case | Recommended Model | Why |
|----------|------------------|-----|
| Collaborative editing | Eventual (CRDT) | Offline support, low latency |
| Financial transactions | Strong | No conflicts acceptable |
| Chat/Comments | Causal | Replies must follow originals |
| Presence indicators | Eventual | Slight staleness OK |
| Game state | Strong or Causal | Depends on game type |

### 7.5 Handling Conflicts

```typescript
// Strategy 1: Last-write-wins (for non-critical data)
const finalState = timestampA > timestampB ? stateA : stateB;

// Strategy 2: Merge (for CRDTs)
const finalState = merge(stateA, stateB);

// Strategy 3: Manual resolution (for complex conflicts)
if (hasConflict(stateA, stateB)) {
  markForManualReview(conflict);
  return lastKnownGood;
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial state sync deep dive created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
