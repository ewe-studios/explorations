# Durable Promise Specification Deep Dive

## Overview

The Durable Promise Specification defines an API for promises with identity and state that persists beyond a single runtime. This is the foundational primitive that ResonateHQ builds upon.

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.resonatehq/durable-promise-specification/`

## Motivation

### The Problem

Traditional promises are bound to ephemeral memory. If a process crashes, all in-flight promises are lost. This makes building reliable distributed systems challenging.

### The Solution

Durable promises have:
- **Identity**: Unique ID that survives restarts
- **Durable State**: State persisted to durable storage
- **Idempotent Operations**: Support for deduplication

## API Specification

### Downstream API (Promise Creation)

Components that create and await promises:

#### Create

```
Create(promise-id, idempotency-key, param, header, timeout, strict)
```

Parameters:
- `promise-id`: Unique identifier for the promise
- `idempotency-key`: Client-generated key for deduplication
- `param`: Input parameters (headers + data)
- `header`: Metadata headers
- `timeout`: Time in ms after which promise is considered timed out
- `strict`: If true, only deduplicates if promise is pending

#### Cancel

```
Cancel(promise-id, idempotency-key, value, header, strict)
```

Cancel a pending promise with a cancellation reason.

#### Callback

```
Callback(id, promise-id, root-promise-id, timeout, recv)
```

Register a callback to be triggered when the promise completes.

**Receivers:**

| Type | Data | Shorthand |
|------|------|-----------|
| poll | `{"group": "string", "id": "string"}` | `poll://group:id` |
| http | `{"headers": {...}, "url": "string"}` | `http://example.com` |

### Upstream API (Promise Completion)

Components that resolve or reject promises:

#### Resolve

```
Resolve(promise-id, idempotency-key, value, header, strict)
```

Mark a promise as successfully completed with a value.

#### Reject

```
Reject(promise-id, idempotency-key, value, header, strict)
```

Mark a promise as failed with an error value.

## State Machine

### Promise States

```
Init -> Pending -> Resolved
                     |
                     v
               Rejected
                     |
                     v
               Canceled
                     |
                     v
               Timedout
```

### Complete State Transition Table

The specification defines 290+ state transitions. Here are the key patterns:

#### Creation Transitions

| Current | Action | Next | Output |
|---------|--------|------|--------|
| Init | Create(id, -, T) | Pending | OK |
| Init | Create(id, ikc, T) | Pending | OK |
| Init | Create(id, ikc, F) | Pending | OK |
| Pending | Create(id, ikc, T) | Pending | OK (Deduplicated) |
| Pending | Create(id, ikc*, T) | Pending | Error (Already Pending) |
| Resolved | Create(id, ikc, T) | Resolved | Error (Already Resolved) |

#### Resolution Transitions

| Current | Action | Next | Output |
|---------|--------|------|--------|
| Pending | Resolve(id, -, T) | Resolved | OK |
| Pending | Resolve(id, iku, T) | Resolved | OK |
| Resolved | Resolve(id, iku, T) | Resolved | OK (Deduplicated) |
| Resolved | Resolve(id, iku*, T) | Resolved | Error (Already Resolved) |
| Rejected | Resolve(id, iku, T) | Rejected | OK (Deduplicated) |

#### Key: ikc vs iku

- `ikc`: Idempotency Key for Create
- `iku`: Idempotency Key for Complete (Resolve/Reject/Cancel)

### Strict vs Non-Strict Mode

**Strict Mode (`strict=true`):**
- Only deduplicates if promise is in target state
- Returns error for state mismatches

**Non-Strict Mode (`strict=false`):**
- More permissive state transitions
- Allows operations on timed-out promises

## Idempotency Model

### Idempotency Key Properties

1. **Client-Generated**: Unique per logical operation
2. **Server-Stored**: Associated with the promise
3. **Match Semantics**: Exact string match

### Idempotency Rules

**For Create:**
- First Create with `ikc` creates promise
- Subsequent Create with same `ikc` returns existing (deduplicated)
- Subsequent Create with different `ikc` returns error

**For Complete:**
- First Complete with `iku` completes promise
- Subsequent Complete with same `iku` returns existing (deduplicated)
- Subsequent Complete with different `iku` returns error

### Idempotency Key Generation Strategies

```typescript
// Hash-based
const ikey = hash(`${functionName}:${argumentHash}`);

// UUID-based
const ikey = crypto.randomUUID();

// Deterministic
const ikey = `${parentId}.${childIndex}.${functionName}`;
```

## Callback System

### Callback Registration

```
Callback(id, promise-id, root-promise-id, timeout, recv)
```

**Parameters:**
- `id`: Unique callback ID
- `promise-id`: Promise to watch
- `root-promise-id`: Root promise for tracing
- `timeout`: Callback timeout
- `recv`: Receiver specification

### Receiver Types

#### Poll Receiver

For polling-based callbacks:

```json
{
  "type": "poll",
  "group": "worker-group-1",
  "id": "worker-123"
}
```

Shorthand: `poll://worker-group-1/worker-123`

#### HTTP Receiver

For webhook callbacks:

```json
{
  "type": "http",
  "url": "https://example.com/callback",
  "headers": {
    "Authorization": "Bearer token"
  }
}
```

Shorthand: `http://example.com/callback`

### Callback Delivery

1. Promise completes (resolve/reject/cancel/timeout)
2. System finds all registered callbacks
3. For each callback:
   - Create task for delivery
   - Worker claims task
   - Task executes delivery
   - Task completes, callback deleted

## Timeout Handling

### Timeout States

When a promise times out:

1. State transitions to `Timedout`
2. All pending callbacks are triggered with timeout error
3. Associated tasks are dropped

### Special Timeout Handling

For promises tagged with `resonate:timeout=true`:
- Timeout is treated as success (Resolved state)
- This enables durable sleep patterns

```typescript
// Durable sleep implementation
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
  await handle.result(); // Returns undefined on timeout (treated as success)
}
```

## Tags System

### Tag Structure

```typescript
tags: Record<string, string>
```

### Special Tags

| Tag | Purpose |
|-----|---------|
| `resonate:invocation` | Marks top-level function invocations |
| `resonate:schedule` | Links promise to schedule |
| `resonate:timeout` | Marks timeout promises as success |

### Tag-Based Search

```typescript
// Search by ID pattern, state, and tags
for await (const promises of store.search(
  "user.*",           // ID pattern (wildcards supported)
  "pending",          // State filter
  { "resonate:invocation": "true" },  // Tags
  100                 // Limit
)) {
  // Process promises
}
```

## Consistency Guarantees

### Single Assignment

A promise can only be assigned a value once:
- First Resolve/Reject/Cancel wins
- Subsequent operations with different idempotency keys fail
- Subsequent operations with same idempotency key are deduplicated

### Eventual Consistency

The system provides eventual consistency guarantees:

1. **Create Visibility**: After Create returns, promise is visible
2. **Complete Visibility**: After Complete returns, completion is visible
3. **Callback Ordering**: Callbacks are delivered after completion is visible

### Recovery Guarantees

After a crash:

1. **Promise State**: All persisted promises are recoverable
2. **Idempotency**: Re-execution deduplicates on existing promises
3. **Callbacks**: Pending callbacks are re-registered

## Implementation Notes

### Storage Requirements

```typescript
interface PromiseRecord {
  id: string;
  state: "PENDING" | "RESOLVED" | "REJECTED" | "CANCELED" | "TIMEDOUT";
  param: { headers: Record<string, string>; data: any };
  value: { headers: Record<string, string>; data: any };
  timeout: number;
  idempotencyKeyForCreate?: string;
  idempotencyKeyForComplete?: string;
  tags?: Record<string, string>;
  createdOn: number;
  completedOn?: number;
}
```

### Indexing Requirements

For efficient operations, implementations should index:

1. **By ID**: Primary key for Get operations
2. **By ID Pattern**: For Search operations (wildcard support)
3. **By State**: For filtering pending/completed promises
4. **By Tags**: For tag-based searches
5. **By Timeout**: For timeout processing

### Transaction Semantics

Operations should be atomic:

```typescript
// Read-Modify-Write pattern
async rmw(id, fn) {
  const existing = await this.get(id);
  const result = fn(existing);
  await this.set(id, result);
  return result;
}
```

## Example Flows

### Basic Promise Flow

```
Client                          Server
  |                               |
  |-- Create(id="p1", ikc="k1") ->|
  |                               | (Store: pending)
  |<-- Promise(pending) ---------|
  |                               |
  |-- Resolve(id="p1", iku="k2") >|
  |                               | (Store: resolved)
  |<-- Promise(resolved) --------|
```

### Idempotent Create Flow

```
Client                          Server
  |                               |
  |-- Create(id="p1", ikc="k1") ->|
  |                               | (Store: pending)
  |<-- Promise(pending) ---------|
  |                               |
  |-- Create(id="p1", ikc="k1") ->| (Retry after timeout)
  |                               | (Found with same ikc)
  |<-- Promise(pending) ---------| (Deduplicated)
```

### Callback Flow

```
Client                          Server
  |                               |
  |-- Create(id="p1") ----------->|
  |                               |
  |-- Callback(id="c1", recv=HTTP)|
  |                               | (Store callback)
  |                               |
  |-- Resolve(id="p1") ---------->|
  |                               | (Trigger callback)
  |                               |--> HTTP POST --> Client
```

## Security Considerations

### Idempotency Key Uniqueness

- Keys should be unguessable for sensitive operations
- Use cryptographic randomness or HMACs

### Access Control

The specification doesn't define access control, but implementations should consider:

1. **Authentication**: Verify client identity
2. **Authorization**: Control who can create/complete promises
3. **Isolation**: Multi-tenant support via namespaces

## Performance Characteristics

### Latency

- **Create**: Single write operation
- **Complete**: Read + Write (transactional)
- **Get**: Single read operation
- **Search**: Depends on indexing (should be O(log n) with proper indexes)

### Throughput

- Limited by storage backend
- Horizontal scaling via sharding by promise ID
- Callback delivery can be parallelized

### Durability

- Depends on storage backend
- Write-ahead logging recommended
- Replication for high availability
