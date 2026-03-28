# State Management Deep Dive: StateStore, Message Persistence, and Crash Recovery

**Document Length:** ~650 lines
**Source Files:** Fragment agent system state management
**Last Updated:** 2026-03-27

---

## Table of Contents

1. [Why Persist Agent Communication?](#1-why-persist-agent-communication)
2. [StateStore Abstraction](#2-statesstore-abstraction)
3. [Messages vs Parts](#3-messages-vs-parts)
4. [Thread Isolation](#4-thread-isolation)
5. [Message Boundary Detection](#5-message-boundary-detection)
6. [Crash Recovery and Flush Operations](#6-crash-recovery-and-flush-operations)
7. [Tool-Use ID Deduplication](#7-tool-use-id-deduplication)
8. [SQLite Implementation](#8-sqlite-implementation)

---

## 1. Why Persist Agent Communication?

### 1.1 The Problem Space

In a multi-agent AI system, agents communicate through structured message exchanges. Without persistence, several critical failure modes emerge:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    WITHOUT PERSISTENCE                                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Agent A ──► generates response ──► crash ──► LOSE EVERYTHING ✗         │
│                                                                          │
│  Agent A ──► sends to Agent B ──► Agent B restarts ──► NO CONTEXT ✗     │
│                                                                          │
│  User asks "What did we decide?" ──► NO HISTORY ✗                        │
│                                                                          │
│  Debug: "Why did agent X do Y?" ──► NO AUDIT TRAIL ✗                     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Crash Recovery Requirements

When an agent process crashes or is terminated mid-execution, the system must recover gracefully:

```typescript
// From agent.ts - spawn function (lines 129-131)
// Recover from crash: flush any partial parts from previous session
// Use agentId to only flush this agent's parts
yield* flush(store, threadId, agentId);
```

**Recovery Strategy:**
- On spawn, each agent flushes its accumulated parts
- Incomplete streaming responses are converted to complete messages
- Parts buffer is cleared, preventing duplicate processing
- Agent resumes with full conversation context

### 1.3 Multi-Agent Coordination Needs

In multi-agent channels, multiple agents participate in the same thread:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    MULTI-AGENT CHANNEL ARCHITECTURE                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                   │
│  │  Agent A    │    │  Agent B    │    │  Agent C    │                   │
│  │  (planner)  │    │  (coder)    │    │  (reviewer) │                   │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘                   │
│         │                  │                  │                           │
│         └──────────────────┼──────────────────┘                           │
│                            │                                              │
│                            ▼                                              │
│                   ┌─────────────────┐                                     │
│                   │   Shared Thread │                                     │
│                   │   (conversation)│                                     │
│                   │                 │                                     │
│                   │  [user: build]  │                                     │
│                   │  [A: planning]  │                                     │
│                   │  [B: coding]    │                                     │
│                   │  [C: reviewing] │                                     │
│                   └─────────────────┘                                     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

**Coordination Requirements:**
- All agents see the same conversation history
- Each agent's parts are isolated during streaming
- Coordinator can track which agents are "typing"
- Completion events synchronize agent activities

### 1.4 Conversation History Preservation

Conversation history serves multiple purposes:

| Purpose | Benefit |
|---------|---------|
| Context continuity | Agents reference prior decisions |
| Tool-use tracking | Prevents duplicate tool calls |
| User experience | Users see complete conversation |
| Audit trail | Debugging and compliance |

### 1.5 Debug and Audit Capabilities

```typescript
// From agent.ts - logging throughout
log("spawn", "loaded chat history", JSON.stringify(rawStoredMessages, null, 2));
log("flush", "parts", JSON.stringify(parts, null, 2));
log("flush", "writing messages", JSON.stringify(messages, null, 2));
```

**Debug Information Captured:**
- Message content and role
- Sender attribution
- Tool call IDs and parameters
- Timestamps for ordering

---

## 2. StateStore Abstraction

### 2.1 StateStore Interface

The StateStore provides a unified interface for thread state persistence:

```typescript
// From state-store.ts (lines 26-131)
export interface StateStore {
  // Message operations (backwards compatible)
  readThreadMessages(threadId: string): Effect<MessageEncoded[], StateStoreError>;
  writeThreadMessages(threadId: string, messages: MessageEncoded[]): Effect<void>;

  // Message operations with sender attribution
  readThreadMessagesWithSender(threadId: string): Effect<MessageWithSender[], StateStoreError>;
  writeThreadMessagesWithSender(threadId: string, messages: MessageWithSender[]): Effect<void>;

  // Parts operations (bulk)
  readThreadParts(threadId: string): Effect<MessagePart[], StateStoreError>;
  truncateThreadParts(threadId: string): Effect<void>;

  // Parts operations (agent-scoped)
  readAgentParts(threadId: string, sender: string): Effect<MessagePart[], StateStoreError>;
  truncateAgentParts(threadId: string, sender: string): Effect<void>;

  // Part appending (persists + publishes)
  appendThreadPart(threadId: string, part: MessagePart): Effect<void>;

  // Part publishing (PubSub only, no persistence)
  publishThreadPart(threadId: string, part: MessagePart): Effect<void>;

  // Thread management
  getTypingAgents(threadId: string): Effect<string[], StateStoreError>;
  listThreads(): Effect<{ threadId: string }[], StateStoreError>;
  deleteThread(threadId: string): Effect<void>;

  // Streaming
  subscribeThread(threadId: string): Effect<Stream<MessagePart>>;
}
```

### 2.2 Service Registration

```typescript
// From state-store.ts (line 17)
export const StateStore = Context.GenericTag<StateStore>("StateStore");
```

The StateStore is registered as an Effect Context tag, enabling dependency injection:

```typescript
// From agent.ts (line 124)
const store = yield* StateStore;
```

### 2.3 StateStore Factory

```typescript
// From state-store.ts (lines 136-208)
export const createStateStore = (
  persistence: Omit<StateStore, "subscribeThread">,
) => {
  // Map of PubSubs per thread for streaming
  const threads = new Map<string, Thread>();

  const getThread = Effect.fnUntraced(function* (threadId: string) {
    const existing = threads.get(threadId);
    if (existing) return existing;

    // Create PubSub with replay: 0 (ChatView reads from persistence)
    const pubsub = yield* PubSub.unbounded<MessagePart>({ replay: 0 });

    // Daemon keeps PubSub alive for streaming to UI
    const daemon = yield* Stream.fromPubSub(pubsub).pipe(
      Stream.runDrain,
      Effect.forkDaemon,
    );

    const thread = { pubsub, daemon } satisfies Thread;
    threads.set(threadId, thread);
    return thread;
  });

  return {
    ...persistence,

    // Override: persist AND publish for real-time UI updates
    appendThreadPart: Effect.fnUntraced(function* (threadId, part) {
      const pubsub = yield* getPubSub(threadId);
      yield* Effect.all(
        [
          persistence.appendThreadPart(threadId, part),
          PubSub.publish(pubsub, part),
        ],
        { concurrency: "unbounded" },
      );
    }),

    // Publish only - for user-input already in messages table
    publishThreadPart: Effect.fnUntraced(function* (threadId, part) {
      const pubsub = yield* getPubSub(threadId);
      yield* PubSub.publish(pubsub, part);
    }),

    // Streaming subscription
    subscribeThread: Effect.fnUntraced(function* (threadId) {
      const pubsub = yield* getPubSub(threadId);
      return Stream.fromPubSub(pubsub);
    }),
  } satisfies StateStore;
};
```

### 2.4 Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| `replay: 0` for PubSub | ChatView reads existing parts from persistence, then subscribes for new parts only. Using `replay: Infinity` caused duplicates. |
| Direct persistence in `appendThreadPart` | The daemon does NOT persist parts - persistence happens directly in `appendThreadPart`. This fixes the "duplicate tool_use ids" bug. |
| Agent-scoped operations | `readAgentParts` and `truncateAgentParts` enable per-agent flush operations. |

---

## 3. Messages vs Parts

### 3.1 Message Structure

Messages are the canonical, persisted conversation history:

```typescript
// From thread.ts (lines 11-16)
export type MessageWithSender = MessageEncoded & {
  readonly sender?: string;
};

// MessageEncoded from @effect/ai/Prompt:
// {
//   role: "user" | "assistant";
//   content: string | ContentBlock[];
// }
```

**Message Content Block Types:**
```typescript
type ContentBlock =
  | { type: "text"; text: string }
  | { type: "tool-call"; id: string; name: string; params: object }
  | { type: "tool-result"; id: string; result: unknown }
  | { type: "reasoning"; content: string };
```

### 3.2 Part Types (Streaming Units)

Parts are the atomic units of streaming responses:

```typescript
// From thread.ts (lines 27-94)
export type MessagePart =
  | UserInputPart
  | AnyPartWithSender
  | CoordinatorPart;

// User input
export interface UserInputPart {
  readonly type: "user-input";
  readonly content: string;
  readonly timestamp: number;
  readonly sender?: string;
}

// AI response parts (from @effect/ai/Response.AnyPart)
type AnyPartWithSender = AnyPart & {
  readonly sender?: string;
};

// Coordinator events
type CoordinatorPart =
  | CoordinatorThinkingPart      // "Thinking..." indicator
  | CoordinatorInvokePart        // "Invoking @agent..." bubbles
  | CoordinatorInvokeCompletePart; // Agent complete indicator
```

**AI Response Part Types:**
```typescript
type AnyPart =
  | { type: "text-start"; id: string }
  | { type: "text-delta"; id: string; delta: string }
  | { type: "text-end"; id: string }
  | { type: "tool-call"; id: string; name: string; params: object }
  | { type: "tool-result"; id: string; result: unknown; error?: string }
  | { type: "reasoning-start"; id: string }
  | { type: "reasoning-delta"; id: string; delta: string }
  | { type: "reasoning-end"; id: string };
```

### 3.3 Streaming Accumulation in Parts

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    STREAMING ACCUMULATION FLOW                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Time ──►                                                                │
│                                                                          │
│  Parts Buffer (per agent):                                              │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ [text-start] → [text-delta "Hello"] → [text-delta " world"]    │    │
│  │ → [tool-call] → [tool-result] → [text-end]                     │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  When text-end arrives (message boundary):                              │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  FLUSH: Parts → Prompt.fromResponseParts() → Messages           │    │
│  │  Then: truncate parts buffer                                    │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.4 Flushing Parts to Messages

```typescript
// From agent.ts (lines 427-572)
const flush = (
  store: StateStore,
  threadId: string,
  agentId: string,
): Effect<void, StateStoreError> =>
  Effect.gen(function* () {
    // Read only this agent's parts
    const parts = yield* store.readAgentParts(threadId, agentId);

    if (parts.length === 0) return;

    // Filter out non-AI parts
    const aiParts = parts.filter(
      (p) =>
        p.type !== "user-input" &&
        p.type !== "coordinator-thinking" &&
        p.type !== "coordinator-invoke" &&
        p.type !== "coordinator-invoke-complete",
    );

    if (aiParts.length === 0) {
      yield* store.truncateAgentParts(threadId, agentId);
      return;
    }

    // Convert parts to messages using @effect/ai's Prompt.fromResponseParts
    const prompt = Prompt.fromResponseParts(aiParts as any[]);
    const messages = yield* Effect.all(
      prompt.content.map((msg) => encode(msg).pipe(Effect.orDie)),
    );

    // Deduplicate tool_use IDs
    const currentMessages = yield* store.readThreadMessagesWithSender(threadId);
    const existingToolIds = new Set<string>();
    for (const msg of currentMessages) {
      for (const id of extractToolUseIds(msg)) {
        existingToolIds.add(id);
      }
    }

    const deduplicatedMessages = messages.map((msg) => {
      // ... filter duplicate tool-calls ...
    }).filter(msg => msg !== null);

    // Write messages and truncate parts
    yield* store.writeThreadMessagesWithSender(threadId, [
      ...currentMessages,
      ...deduplicatedMessages,
    ]);
    yield* store.truncateAgentParts(threadId, agentId);
  });
```

---

## 4. Thread Isolation

### 4.1 Thread ID Scoping

All state operations are keyed by `threadId`:

```typescript
// Every StateStore method takes threadId as the first parameter
readThreadMessages(threadId: string): Effect<...>;
readAgentParts(threadId: string, sender: string): Effect<...>;
appendThreadPart(threadId: string, part: MessagePart): Effect<...>;
```

### 4.2 Per-Agent Parts Within Threads

While messages are shared across all participants in a thread, parts are isolated by sender:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    THREAD ISOLATION MODEL                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Thread: "planning-session-001"                                         │
│                                                                          │
│  Messages Table (shared):                                               │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ position | role       | sender    | content                     │    │
│  │    0     | user       | undefined | "Build a feature"           │    │
│  │    1     | assistant  | planner   | [planning response]         │    │
│  │    2     | assistant  | coder     | [coding response]           │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  Parts Table (per-agent buffer):                                        │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ thread_id | sender  | type        | content                     │    │
│  │   ...     | planner | text-start  | ...                         │    │
│  │   ...     | planner | text-delta  | "Implementing..."           │    │
│  │   ...     | coder   | tool-call   | {name: "readFile"}          │    │
│  │   ...     | coder   | text-start  | ...                         │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  Query: readAgentParts(threadId, "planner") → only planner's parts      │
│  Query: readAgentParts(threadId, "coder") → only coder's parts          │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 4.3 Multi-Agent Channel Coordination

```typescript
// From state-store-sqlite.ts (lines 138-146)
// Get distinct senders that have incomplete message streams
// An agent is "typing" if they have parts but no text-end yet
selectTypingAgents: yield* conn.prepare<{ sender: string }>(`
  SELECT DISTINCT sender FROM parts
  WHERE thread_id = ?
    AND sender IS NOT NULL
    AND sender NOT IN (
      SELECT DISTINCT sender FROM parts
      WHERE thread_id = ? AND type = 'text-end' AND sender IS NOT NULL
    )
`);
```

**Typing Indicator Logic:**
- Query finds agents with parts in the thread
- Excludes agents who have sent `text-end`
- Result: list of agents currently streaming

---

## 5. Message Boundary Detection

### 5.1 isMessageBoundary Function

```typescript
// From agent.ts (lines 379-384)
const isMessageBoundary = (part: MessagePart): boolean =>
  part.type === "user-input" ||
  part.type === "text-end" ||
  part.type === "reasoning-end" ||
  part.type === "tool-call" ||
  part.type === "tool-result";
```

### 5.2 Boundary Types Explained

| Boundary Type | When It Fires | Purpose |
|---------------|---------------|---------|
| `user-input` | User sends a message | Marks end of user input, triggers agent response |
| `text-end` | Assistant finishes text block | Completes a text message, triggers flush |
| `reasoning-end` | Assistant finishes reasoning | Completes reasoning, may trigger flush |
| `tool-call` | Assistant invokes a tool | Tool calls are complete messages |
| `tool-result` | Tool execution completes | Tool results are complete messages |

### 5.3 When to Flush

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    MESSAGE BOUNDARY TRIGGER FLOW                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Stream from AI ──► tap((part) => {                                     │
│                       add sender                                        │
│                       appendThreadPart(part)  // persist + publish      │
│                       if (isMessageBoundary(part)) {                    │
│                         flush()  // convert parts to messages           │
│                       }                                                 │
│                     })                                                  │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  Boundary Detection Examples:                                    │    │
│  │                                                                  │    │
│  │  "Hello world" → [text-start] [text-delta "Hello"]              │    │
│  │                      [text-delta " world"] [text-end] ◄── FLUSH! │    │
│  │                                                                  │    │
│  │  Tool call → [tool-call {id: "abc", name: "readFile"}] ◄── FLUSH!│    │
│  │                                                                  │    │
│  │  Reasoning → [reasoning-start] [reasoning-delta "..."]           │    │
│  │              [reasoning-end] ◄── FLUSH!                          │    │
│  │                                                                  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.4 Streaming Flow with Boundaries

```typescript
// From agent.ts (lines 327-340)
Stream.tap((part) => {
  // Add sender (agentId) to the part for attribution
  const threadPart = { ...part, sender: agentId } as MessagePart;
  return Effect.gen(function* () {
    yield* store.appendThreadPart(threadId, threadPart);
    // Check if message boundary reached
    // Use lockedFlush to prevent race conditions in concurrent streams
    if (isMessageBoundary(threadPart)) {
      yield* lockedFlush(flush(store, threadId, agentId));
    }
  });
}),
```

---

## 6. Crash Recovery and Flush Operations

### 6.1 Flush on Spawn

Every agent flushes its parts when spawned:

```typescript
// From agent.ts (lines 129-131)
// Recover from crash: flush any partial parts from previous session
// Use agentId to only flush this agent's parts
yield* flush(store, threadId, agentId);
```

**Why This Matters:**
- If agent crashed mid-stream, parts buffer contains incomplete data
- Flush converts whatever exists to complete messages
- Parts buffer is cleared, preventing duplicate processing
- Agent starts with clean state and full context

### 6.2 Agent-Scoped Flush

```typescript
// From agent.ts (lines 427-434)
const flush = (
  store: StateStore,
  threadId: string,
  agentId: string,  // Required parameter - always agent-scoped
): Effect<void, StateStoreError> =>
  Effect.gen(function* () {
    // Read only this agent's parts
    const parts = yield* store.readAgentParts(threadId, agentId);
    // ... rest of flush logic ...
  });
```

**Agent Scoping Benefits:**
- Each agent manages its own parts independently
- No race conditions between agents flushing
- Coordinator can flush specific agents on demand

### 6.3 Semaphore for Flush Operations

```typescript
// From agent.ts (lines 157-161)
// Dedicated semaphore for flush operations to prevent race conditions
// The main semaphore doesn't protect Stream.tap operations after Stream.unwrap
const flushSem = yield* Effect.makeSemaphore(1);
const lockedFlush = (fn: Effect<void, StateStoreError>) =>
  flushSem.withPermits(1)(fn);
```

**Why a Dedicated Semaphore:**
- Main semaphore protects `send()` and `query()` operations
- `Stream.tap` runs outside the main semaphore scope
- Flush operations need their own mutual exclusion
- Prevents concurrent flushes from corrupting state

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    SEMAPHORE PROTECTION MODEL                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Main Semaphore (permits: 1)                                            │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  Protects: send(), query()                                      │    │
│  │  Scope: Inside Stream.unwrap                                    │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  Flush Semaphore (permits: 1)                                           │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  Protects: flush() operations                                   │    │
│  │  Scope: Stream.tap (outside main semaphore)                     │    │
│  │  Prevents: Concurrent flushes corrupting messages/parts         │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 6.4 Execution Trace: Crash Recovery

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    CRASH RECOVERY EXECUTION TRACE                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  T0: Agent A streaming response                                        │
│      Parts buffer: [text-start, text-delta "Hell", text-delta "o"]     │
│      Status: INCOMPLETE (no text-end yet)                              │
│                                                                          │
│  T1: CRASH! Process terminated                                           │
│      Parts buffer persisted to SQLite: ["text-start", "text-delta...", │
│                                           "text-delta..."]             │
│      Messages table: unchanged (no flush occurred)                     │
│                                                                          │
│  T2: Agent A respawned                                                   │
│      spawn() called → flush() triggered                                 │
│                                                                          │
│      flush() execution:                                                  │
│      1. readAgentParts(threadId, "agent-a")                            │
│         → [text-start, text-delta "Hello"]                              │
│      2. Prompt.fromResponseParts(aiParts)                              │
│         → Converts incomplete parts to assistant message               │
│      3. writeThreadMessagesWithSender(...)                             │
│         → Persists: {role: "assistant", sender: "agent-a",             │
│                      content: [{type: "text", text: "Hello"}]}         │
│      4. truncateAgentParts(threadId, "agent-a")                        │
│         → Clears parts buffer                                          │
│                                                                          │
│  T3: Agent A continues normally                                        │
│      Parts buffer: [] (empty, clean state)                             │
│      Messages: includes recovered "Hello" message                      │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Tool-Use ID Deduplication

### 7.1 The Duplicate Problem

Tool calls have unique IDs generated by the AI provider. On crash/restart:
- Tool call may be persisted in parts buffer (not yet flushed)
- Same tool call may appear in recovered messages
- Duplicate tool_use IDs cause provider errors

### 7.2 validateAndRepairMessages Function

```typescript
// From agent.ts (lines 602-682)
function validateAndRepairMessages(
  messages: readonly MessageEncoded[],
  agentId: string,
  threadId: string,
): readonly MessageEncoded[] {
  const seenIds = new Set<string>();
  const duplicateIds = new Set<string>();

  // First pass: identify duplicates
  for (const message of messages) {
    const ids = extractToolUseIds(message);
    for (const id of ids) {
      if (seenIds.has(id)) {
        duplicateIds.add(id);
      } else {
        seenIds.add(id);
      }
    }
  }

  // If no duplicates, return original messages
  if (duplicateIds.size === 0) {
    return messages;
  }

  // Log warning about duplicates
  log("spawn", "WARNING: Found duplicate tool_use IDs...", { duplicateIds });

  // Second pass: repair by removing duplicate tool-call blocks
  const repairedSeenIds = new Set<string>();
  const repairedMessages: MessageEncoded[] = [];

  for (const message of messages) {
    if (!Array.isArray(message.content)) {
      repairedMessages.push(message);
      continue;
    }

    const repairedContent = message.content.filter((block) => {
      if (block.type === "tool-call" && typeof block.id === "string") {
        if (repairedSeenIds.has(block.id)) {
          return false; // Remove duplicate
        }
        repairedSeenIds.add(block.id);
      }
      return true;
    });

    if (repairedContent.length > 0) {
      repairedMessages.push({ ...message, content: repairedContent });
    }
  }

  return repairedMessages;
}
```

### 7.3 extractToolUseIds Helper

```typescript
// From agent.ts (lines 579-596)
function extractToolUseIds(message: MessageEncoded): string[] {
  const ids: string[] = [];
  if (Array.isArray(message.content)) {
    for (const block of message.content) {
      if (
        typeof block === "object" &&
        block !== null &&
        "type" in block &&
        block.type === "tool-call" &&
        "id" in block &&
        typeof block.id === "string"
      ) {
        ids.push(block.id);
      }
    }
  }
  return ids;
}
```

### 7.4 Duplicate Detection Algorithm

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    DUPLICATE DETECTION ALGORITHM                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Input: messages = [msg1, msg2, msg3, ...]                              │
│                                                                          │
│  Pass 1 - Detection:                                                    │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  seenIds = {}          duplicateIds = {}                        │    │
│  │                                                                  │    │
│  │  msg1: tool_use_001 → seenIds = {tool_use_001}                  │    │
│  │  msg2: tool_use_002 → seenIds = {tool_use_001, tool_use_002}    │    │
│  │  msg3: tool_use_001 → DUPLICATE! duplicateIds = {tool_use_001}  │    │
│  │                                                                  │    │
│  │  if duplicateIds.size > 0: repair needed                        │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  Pass 2 - Repair:                                                       │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  repairedSeenIds = {}  repairedMessages = []                    │    │
│  │                                                                  │    │
│  │  msg1: keep tool_use_001, repairedSeenIds = {tool_use_001}      │    │
│  │  msg2: keep tool_use_002, repairedSeenIds = {001, 002}          │    │
│  │  msg3: REMOVE tool_use_001 (duplicate), message may become empty │    │
│  │                                                                  │    │
│  │  Output: [msg1, msg2]  (msg3 removed if empty)                  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 7.5 Repair Strategy

**Keep First, Remove Subsequent:**
- The first occurrence of a tool_use ID is kept
- Subsequent occurrences are removed
- If a message becomes empty after filtering, it's dropped
- This preserves the original tool execution order

**Why This Works:**
- Crash recovery: parts flushed to messages become the "first" occurrence
- On restart, any duplicate in persisted messages is removed
- Tool execution state remains consistent

---

## 8. SQLite Implementation

### 8.1 Schema Design

```sql
-- From state-store-sqlite.ts (MIGRATIONS array)

-- Threads table (unified, no agent_id scoping)
CREATE TABLE threads (
  thread_id TEXT PRIMARY KEY,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Messages table with sender attribution
CREATE TABLE messages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  thread_id TEXT NOT NULL,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  position INTEGER NOT NULL,
  sender TEXT,  -- Added in migration 004
  created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Parts table with sender attribution
CREATE TABLE parts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  thread_id TEXT NOT NULL,
  type TEXT NOT NULL,
  content TEXT NOT NULL,
  position INTEGER NOT NULL,
  sender TEXT,  -- Added in migration 004
  created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Indexes for efficient querying
CREATE INDEX idx_messages_v2_thread ON messages(thread_id);
CREATE INDEX idx_parts_v2_thread ON parts(thread_id);
CREATE INDEX idx_parts_sender ON parts(thread_id, sender);
```

### 8.2 Schema Evolution

| Migration | Key Changes |
|-----------|-------------|
| `001_initial_schema` | Original schema with `agent_id` in primary keys |
| `002_conversations_and_thread_replies` | Added conversations table, thread reply support |
| `003_unify_thread_storage` | Removed `agent_id` from thread/message/part keys - all participants share history |
| `004_add_sender_column` | Added `sender` column for per-part attribution (not scoping) |

### 8.3 Messages Table Structure

```typescript
// From state-store-sqlite.ts (lines 88-108)
selectMessages: yield* conn.prepare<{ role: string; content: string }>(`
  SELECT role, content FROM messages
  WHERE thread_id = ?
  ORDER BY position ASC
`),

insertMessage: yield* conn.prepare(`
  INSERT INTO messages (thread_id, role, content, position)
  VALUES (?, ?, ?, ?)
`),
```

**Message Row Structure:**
```
┌─────────────────────────────────────────────────────────────────────────┐
│  messages table row                                                    │
├─────────────────────────────────────────────────────────────────────────┤
│  id:         INTEGER (auto-increment)                                  │
│  thread_id:  TEXT (foreign key to threads)                             │
│  role:       TEXT ("user" or "assistant")                              │
│  content:    TEXT (JSON-encoded content blocks)                        │
│  position:   INTEGER (ordering within thread)                          │
│  sender:     TEXT (agent ID, nullable for user messages)               │
│  created_at: INTEGER (unix timestamp)                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.4 Parts Table Structure

```typescript
// From state-store-sqlite.ts (lines 110-135)
selectParts: yield* conn.prepare<{ type: string; content: string }>(`
  SELECT type, content FROM parts
  WHERE thread_id = ?
  ORDER BY position ASC
`),

selectAgentParts: yield* conn.prepare<{ type: string; content: string }>(`
  SELECT type, content FROM parts
  WHERE thread_id = ? AND sender = ?
  ORDER BY position ASC
`),

insertPart: yield* conn.prepare(`
  INSERT INTO parts (thread_id, type, content, position)
  VALUES (?, ?, ?, ?)
`),
```

**Part Row Structure:**
```
┌─────────────────────────────────────────────────────────────────────────┐
│  parts table row                                                       │
├─────────────────────────────────────────────────────────────────────────┤
│  id:         INTEGER (auto-increment)                                  │
│  thread_id:  TEXT (foreign key to threads)                             │
│  type:       TEXT (part type: "text-start", "tool-call", etc.)         │
│  content:    TEXT (JSON-encoded part object)                           │
│  position:   INTEGER (ordering within thread+sender)                   │
│  sender:     TEXT (agent ID, nullable for coordinator events)          │
│  created_at: INTEGER (unix timestamp)                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.5 Query Patterns

**Read Thread Messages:**
```typescript
// From state-store-sqlite.ts (lines 166-173)
readThreadMessages: (threadId) =>
  Effect.gen(function* () {
    const rows = yield* stmts.selectMessages.all(threadId);
    return rows.map((row) => ({
      role: row.role as MessageEncoded["role"],
      content: JSON.parse(row.content),
    })) as readonly MessageEncoded[];
  })
```

**Agent-Scoped Parts Read:**
```typescript
// From state-store-sqlite.ts (lines 250-254)
readAgentParts: (threadId, sender) =>
  Effect.gen(function* () {
    const rows = yield* stmts.selectAgentParts.all(threadId, sender);
    return rows.map((row) => JSON.parse(row.content) as MessagePart);
  })
```

**Atomic Part Append with Position:**
```typescript
// From state-store-sqlite.ts (lines 256-281)
appendThreadPart: (threadId, part) =>
  conn.batch([
    // Ensure thread exists
    {
      sql: `INSERT INTO threads (thread_id, created_at, updated_at)
            VALUES (?, unixepoch(), unixepoch())
            ON CONFLICT (thread_id) DO UPDATE SET updated_at = unixepoch()`,
      params: [threadId],
    },
    // Insert part with position from subquery (atomic within batch transaction)
    {
      sql: `INSERT INTO parts (thread_id, type, content, position)
            SELECT ?, ?, ?, COALESCE(MAX(position) + 1, 0), ?
            FROM parts WHERE thread_id = ?`,
      params: [
        threadId,
        part.type,
        JSON.stringify(part),
        (part as any).sender ?? null,
        threadId,
      ],
    },
  ])
```

**Typing Agents Query:**
```typescript
// From state-store-sqlite.ts (lines 138-146)
selectTypingAgents: yield* conn.prepare<{ sender: string }>(`
  SELECT DISTINCT sender FROM parts
  WHERE thread_id = ?
    AND sender IS NOT NULL
    AND sender NOT IN (
      SELECT DISTINCT sender FROM parts
      WHERE thread_id = ? AND type = 'text-end' AND sender IS NOT NULL
    )
`);
```

### 8.6 Concurrency Handling

```typescript
// From state-store-sqlite.ts (lines 72-74)
// Enable WAL mode for better concurrent read performance
// WAL mode for better concurrency, busy_timeout waits up to 30s when locked
yield* conn
  .exec("PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 30000;")
```

**WAL Mode Benefits:**
- Multiple readers can access data simultaneously
- Writer doesn't block readers
- Critical for multi-agent systems with concurrent access

**Busy Timeout:**
- Waits up to 30 seconds for locked resources
- Handles migration race conditions gracefully
- Retry logic for remaining busy errors

---

## Appendix: Execution Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    COMPLETE EXECUTION FLOW                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  USER INPUT                                                              │
│      │                                                                   │
│      ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  MessagingService.handleUserMessage()                            │    │
│  │  - Writes user message to messages table                         │    │
│  │  - Invokes coordinator                                           │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│      │                                                                   │
│      ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  Coordinator selects agents                                      │    │
│  │  - Emits coordinator-thinking part                               │    │
│  │  - Emits coordinator-invoke part                                 │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│      │                                                                   │
│      ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  Agent.spawn()                                                   │    │
│  │  - flush() - recover from crash                                  │    │
│  │  - validateAndRepairMessages() - dedupe tool_use IDs             │    │
│  │  - Load chat history                                             │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│      │                                                                   │
│      ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  Agent.send() - Stream response                                  │    │
│  │                                                                  │    │
│  │  ┌───────────────────────────────────────────────────────────┐  │    │
│  │  │  For each part from AI:                                   │  │    │
│  │  │  1. Add sender attribution                                │  │    │
│  │  │  2. appendThreadPart() - persist + publish                │  │    │
│  │  │  3. if isMessageBoundary(): flush()                       │  │    │
│  │  └───────────────────────────────────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│      │                                                                   │
│      ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  flush()                                                         │    │
│  │  - readAgentParts(threadId, agentId)                             │    │
│  │  - Prompt.fromResponseParts(aiParts)                             │    │
│  │  - Deduplicate tool_use IDs                                      │    │
│  │  - writeThreadMessagesWithSender(...)                            │    │
│  │  - truncateAgentParts(threadId, agentId)                         │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│      │                                                                   │
│      ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  UI receives parts via PubSub                                    │    │
│  │  - Renders streaming text                                        │    │
│  │  - Shows tool call/result displays                               │    │
│  │  - Updates typing indicators                                     │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## References

- **Source Files:**
  - `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/fragment/src/state/state-store.ts`
  - `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/fragment/src/state/state-store-sqlite.ts`
  - `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/fragment/src/state/thread.ts`
  - `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/fragment/src/agent.ts`

- **Related Documents:**
  - Fragment architecture overview
  - Multi-agent coordination patterns
  - Effect-TS concurrency primitives
