# Agent System Deep Dive: spawn, send, query Operations

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/fragment/src/agent.ts` (683 lines)

**Date:** 2026-03-27

---

## Table of Contents

1. [Agent Definition](#1-agent-definition)
2. [Spawning Agents](#2-spawning-agents)
3. [Agent Discovery](#3-agent-discovery)
4. [send() Operation](#4-send-operation)
5. [query() Operation](#5-query-operation)
6. [Tool Registration](#6-tool-registration)
7. [Message Boundary Detection](#7-message-boundary-detection)
8. [Tool-Use ID Deduplication](#8-tool-use-id-deduplication)
9. [Execution Trace](#9-execution-trace)

---

## 1. Agent Definition

### Agent Interface

The Agent system is built on top of the Fragment templating system. An agent is defined as:

```typescript
export interface Agent<
  Name extends string = string,
  References extends any[] = any[],
> extends Fragment<"agent", Name, References> {}
```

**Key characteristics:**
- Extends `Fragment<"agent", Name, References>` - inherits all fragment capabilities
- Generic over `Name` (agent identifier) and `References` (other agents/entities it references)
- Type-safe agent composition through TypeScript generics

### The Agent Builder

Agents are created using the `defineFragment` builder pattern:

```typescript
export const Agent = defineFragment("agent")({
  render: {
    context: (agent: Agent) => `@${agent.id}`,
    tui: {
      content: AgentContent,
      focusable: true,
    },
  },
});
```

**Render Configuration:**

| Property | Purpose |
|----------|---------|
| `context` | Returns `@id` format for template interpolation |
| `tui.content` | React component for terminal UI rendering |
| `tui.focusable` | Enables TUI focus/navigation on this agent |

### Context Rendering

When an agent is referenced in a template context, it renders as `@id`:

```typescript
context: (agent: Agent) => `@${agent.id}`
```

This `@id` syntax is used throughout the system for:
- Agent references in prompts
- Inter-agent communication targeting
- Thread/scoped message attribution

### Type Guard

```typescript
export const isAgent = Agent.is<Agent>;
```

Used for runtime type checking during agent discovery (covered in Section 3).

---

## 2. Spawning Agents

### spawn() Function Signature

```typescript
export const spawn: <A extends Agent<string, any[]>>(
  agent: A,
  threadIdOrOptions?: string | SpawnOptions,
) => Effect.Effect<
  AgentInstance<A>,
  AiError | StateStoreError,
  LanguageModel | Handler<string> | StateStore | FileSystem
>
```

**Effects Environment:**
- `LanguageModel` - AI model provider
- `Handler<string>` - Tool execution handlers
- `StateStore` - Persistent message/part storage
- `FileSystem` - File system access for state persistence

### SpawnOptions Interface

```typescript
export interface SpawnOptions {
  /** Optional thread ID. Defaults to agent's ID */
  threadId?: string;

  /** Optional model name for tool aliasing */
  model?: string;

  /** Skip appending user input (already stored by coordinator) */
  skipUserInput?: boolean;
}
```

### spawn() Execution Flow

```
┌─────────────────────────────────────────────────────────────┐
│                      spawn(agent, options)                   │
├─────────────────────────────────────────────────────────────┤
│  1. Normalize options (string → {threadId})                  │
│  2. Initialize StateStore                                    │
│  3. Crash Recovery: flush() agent's partial parts            │
│  4. Load message history from thread                         │
│  5. Validate & repair messages (duplicate tool_use IDs)      │
│  6. Persist repaired messages if modified                    │
│  7. Initialize Chat from messages                            │
│  8. Create semaphores (main + flush)                         │
│  9. Build agent reference map (BFS discovery)                │
│ 10. Return AgentInstance { agent, send, query }              │
└─────────────────────────────────────────────────────────────┘
```

### StateStore Initialization

```typescript
const store = yield* StateStore;
```

The StateStore is the single source of truth for:
- Thread messages (persisted conversation history)
- Agent parts (streaming accumulation buffer)
- Multi-agent channel coordination

### Crash Recovery: flush on Spawn

```typescript
// Recover from crash: flush any partial parts from previous session
// Use agentId to only flush this agent's parts
yield* flush(store, threadId, agentId);
```

**Why this matters:**
- If an agent crashed mid-stream, parts may be orphaned in the buffer
- Flush converts accumulated parts → messages before new operations
- Agent-scoped: only flushes parts belonging to this agent

### Message History Loading

```typescript
const rawStoredMessages = yield* store.readThreadMessages(threadId);
log("spawn", "loaded chat history", JSON.stringify(rawStoredMessages, null, 2));

// Validate and repair messages to ensure unique tool_use IDs
const storedMessages = validateAndRepairMessages(rawStoredMessages, agentId, threadId);

// If messages were repaired, persist the fixed version
if (storedMessages !== rawStoredMessages) {
  yield* store.writeThreadMessages(threadId, storedMessages);
  log("spawn", "persisted repaired messages");
}
```

**Context Tracking:**
```typescript
let contextSent = storedMessages.length > 0;
```
- If history exists, context messages already sent (skip on send())
- Prevents duplicate tool_use IDs from context

---

## 3. Agent Discovery

### Building the Agent Reference Map

```typescript
// Build a map of agent ID -> Agent for O(1) lookups
// Uses a queue-based approach to properly resolve thunks before checking isAgent
const agents = new Map<string, Agent>();
const visited = new Set<unknown>();
const queue: unknown[] = [...agent.references];

while (queue.length > 0) {
  const item = queue.shift()!;
  const resolved = resolveThunk(item);

  if (resolved === undefined || resolved === null || visited.has(resolved)) {
    continue;
  }
  visited.add(resolved);

  if (isAgent(resolved) && resolved.id !== self.id) {
    if (!agents.has(resolved.id)) {
      agents.set(resolved.id, resolved);
      // Queue its references for processing
      queue.push(...resolved.references);
    }
  }
}
```

### Queue-Based Thunk Resolution

**Why thunks need resolution:**
- Agent references may be lazy (thunks) for circular dependencies
- `resolveThunk(item)` unwraps to actual agent object
- `isAgent()` only works on resolved values

### Visited Set for Cycle Prevention

```typescript
const visited = new Set<unknown>();
```

**Cycle prevention algorithm:**
1. Before processing any item, check `visited.has(resolved)`
2. Skip if already visited (prevents infinite loops)
3. Add to visited before processing

**Example scenario with cycles:**
```
Agent A → [Agent B, Agent C]
Agent B → [Agent A, Agent D]  // Circular back to A
Agent C → [Agent D]
```

Without `visited`, processing would loop: A → B → A → B → ...

### BFS Traversal

```
Initial: queue = [A.refs]
Step 1:  pop A.ref1 → if Agent → add to map, queue its refs
Step 2:  pop A.ref2 → if Agent → add to map, queue its refs
...
Repeat until queue empty
```

### O(1) Agent Lookup

After discovery, agents are stored in a Map:
```typescript
const agents = new Map<string, Agent>();
// Lookup: agents.get(recipientId) - O(1)
```

Used by `lookupAgent()` in send/query tool handlers.

### Lazy Agent Spawning

```typescript
const spawned = new Map<string, AgentInstance<any>>();

const lookupAgent = Effect.fn(function* (recipient: string) {
  if (!spawned.has(recipient)) {
    const childAgent = agents.get(recipient);
    if (!childAgent) {
      return {
        error: true as const,
        message: `Agent "${recipient}" not found. Available agents: ${[...agents.keys()].join(", ")}`,
      };
    }
    spawned.set(recipient, yield* spawn(childAgent, threadId));
  }
  return spawned.get(recipient)!;
});
```

**Key design decisions:**
- Agents spawned on-demand (not eagerly)
- Error returned as object (not thrown) - allows AI to adapt
- Error message includes available agents for debugging

---

## 4. send() Operation

### Stream-Based Message Sending

```typescript
send: (prompt: string) =>
  Stream.unwrap(
    locked(
      Effect.gen(function* () {
        // ... setup ...
        return chat.streamText({ ... }).pipe(
          Stream.provideLayer(context.toolkitHandlers),
          Stream.retry(Schedule.recurWhile(isRetryableAiError)...),
          Stream.tap((part) => { ... }),
          Stream.map((part) => ({ ...part, sender: agentId })),
        );
      }),
    ),
  ),
```

### Context Message Handling

```typescript
// Only include context messages on first call (when history is empty)
// Otherwise the context messages are already in the chat history
// and including them again would create duplicate tool_use IDs
const includeContext = !contextSent;
const fullPrompt = includeContext
  ? [
      ...context.messages,
      { role: "user" as const, content: prompt },
    ]
  : [
      { role: "user" as const, content: prompt },
    ];

// Mark context as sent for subsequent calls
contextSent = true;
```

**Why this matters:**
- Context includes system prompt + tool definitions
- Tool definitions contain tool_use ID templates
- Duplicating context → duplicate tool_use IDs → validation failures
- `contextSent` flag ensures context sent exactly once per thread

### Chat.streamText with Toolkit

```typescript
chat.streamText({
  toolkit: context.toolkit,
  prompt: fullPrompt,
})
```

**Stream flow:**
```
┌─────────────────────────────────────────────────────────────┐
│ chat.streamText()                                           │
├─────────────────────────────────────────────────────────────┤
│  → Emits: TextStreamPart chunks                             │
│     - text-start, text-delta, text-end                      │
│     - tool-call, tool-result                                │
│     - reasoning-start, reasoning-delta, reasoning-end       │
│  → Each chunk flows through Stream pipeline                 │
│  → tap() intercepts for storage                             │
│  → map() adds sender attribution                            │
│  → Consumer receives MessagePart stream                     │
└─────────────────────────────────────────────────────────────┘
```

### Retry Logic for Transient Errors

```typescript
Stream.retry(
  Schedule.recurWhile(isRetryableAiError).pipe(
    Schedule.intersect(aiRetrySchedule),
  ),
)
```

**Retryable errors:**
```typescript
const isRetryableAiError = (error: unknown): boolean => {
  if (error && typeof error === "object") {
    const err = error as Record<string, unknown>;
    if (err.status === 429) return true;  // Rate limit
    if (typeof err.status === "number" &&
        err.status >= 500 && err.status < 600) return true;  // Server error
    if (typeof err.message === "string" &&
        (err.message.includes("timeout") ||
         err.message.includes("Timeout"))) return true;  // Timeout
  }
  return false;
};
```

**Retry schedule:**
```typescript
const aiRetrySchedule = Schedule.intersect(
  Schedule.exponential("1 second"),  // 1s, 2s, 4s, 8s...
  Schedule.recurs(3),                 // Max 3 retries
);
```

### Tap and Flush to StateStore

```typescript
Stream.tap((part) => {
  const threadPart = { ...part, sender: agentId } as MessagePart;
  return Effect.gen(function* () {
    yield* store.appendThreadPart(threadId, threadPart);
    // Check if message boundary reached
    if (isMessageBoundary(threadPart)) {
      yield* lockedFlush(flush(store, threadId, agentId));
    }
  });
})
```

**Tap flow:**
1. Add `sender` attribution to part
2. Append to thread parts buffer
3. Check if boundary reached (message complete)
4. If boundary → flush parts → messages

### Sender Attribution

```typescript
Stream.map((part) => ({ ...part, sender: agentId }) as MessagePart)
```

All parts include `sender: agentId` for:
- Multi-agent channel message attribution
- Agent-scoped flush filtering
- UI rendering with agent identification

---

## 5. query() Operation

### Structured Response Generation

```typescript
query: <A>(prompt: string, schema: S.Schema<A, any>) =>
  locked(
    Effect.provide(
      chat
        .generateObject({
          toolkit: context.toolkit,
          schema,
          prompt: [
            ...context.messages,
            { role: "user" as const, content: prompt },
          ],
        })
        .pipe(
          Effect.retry(
            Schedule.recurWhile(isRetryableAiError).pipe(
              Schedule.intersect(aiRetrySchedule),
            ),
          ),
          Effect.tapError((error) =>
            Effect.sync(() => log("query", "error", error)),
          ),
        ),
      context.toolkitHandlers,
    ),
  ),
```

### Schema Validation

```typescript
query: <A>(prompt: string, schema: S.Schema<A, any>)
```

- Generic over response type `A`
- Uses `effect/Schema` for type-safe validation
- Schema passed to `generateObject()` for model guidance

### chat.generateObject

```typescript
chat.generateObject({
  toolkit: context.toolkit,
  schema,
  prompt: [...],
})
```

**Differences from streamText:**
| streamText | generateObject |
|------------|----------------|
| Returns `Stream<MessagePart>` | Returns `Effect<GenerateObjectResponse>` |
| Streaming chunks | Single structured response |
| For conversations | For queries needing specific schema |
| No schema validation | Schema-validated response |

### Error Handling and Retry

Same retry logic as send():
- Exponential backoff starting at 1s
- Max 3 retries
- Handles rate limits, server errors, timeouts

---

## 6. Tool Registration

### Comms Toolkit

```typescript
class Comms extends Toolkit(
  "Comms",
)`Tools for communicating with other agents. Use these tools to coordinate work with other agents.

- ${send}
- ${query}` {}
```

**Toolkit structure:**
- Name: "Comms"
- Description: Includes send and query tool descriptions
- Empty class body - tools defined separately

### Tool Definitions

#### send Tool

```typescript
const message = input("message")`The message to send`;
const recipient = input("recipient")`The absolute path/ID of the recipient agent`;

const send = Tool("send")`Send a ${message} to ${recipient}, receive a response as a ${S.String}`(
  function* ({ message, recipient }) {
    const result = yield* lookupAgent(recipient);
    if ("error" in result) {
      return result.message;
    }
    return yield* result.send(message).pipe(toText("last-message"));
  },
);
```

**Input schema:**
- `message: string` - Message content
- `recipient: string` - Agent ID (absolute path)

**Output schema:**
- `string` - Last message from response

#### query Tool

```typescript
const schema = input("schema")`The expected schema of the query response`;
const object = output("object", S.Any);

const query = Tool("query")`Send a query ${message} to the ${recipient} agent and receive back a structured ${object} with the expected schema ${schema}`(
  function* ({ recipient, message, schema: jsonSchema }) {
    const result = yield* lookupAgent(recipient);
    if ("error" in result) {
      return { object: { error: result.message } };
    }
    return {
      object: (yield* result.query(
        message,
        schemaFromJsonSchema(JSON.parse(jsonSchema) as JsonSchema7Root),
      )).value,
    };
  },
);
```

**Input schema:**
- `message: string` - Query prompt
- `recipient: string` - Agent ID
- `schema: JsonSchema7Root` - Expected response schema (JSON string)

**Output schema:**
- `{ object: any }` - Structured response matching schema

### Handler Layer Provision

```typescript
// In send() Stream:
Stream.provideLayer(context.toolkitHandlers)

// In query() Effect:
Effect.provide(context.toolkitHandlers)
```

The toolkit handlers are provided from context:
```typescript
const context = yield* createContext(agent, {
  tools: [Comms],
  model,
});
```

### Model-Specific Tool Aliasing

```typescript
const model = options.model ?? process.env.ANTHROPIC_MODEL_ID ?? "claude-sonnet-4-5";
```

Used in `createContext()` for:
- Registering tools with provider-specific names
- Example: "AnthropicBash" for Claude Code compatibility
- Ensures tool names match model expectations

---

## 7. Message Boundary Detection

### isMessageBoundary Function

```typescript
const isMessageBoundary = (part: MessagePart): boolean =>
  part.type === "user-input" ||
  part.type === "text-end" ||
  part.type === "reasoning-end" ||
  part.type === "tool-call" ||
  part.type === "tool-result";
```

**Boundary types:**

| Type | Meaning |
|------|---------|
| `user-input` | User message boundary |
| `text-end` | Assistant text response complete |
| `reasoning-end` | Reasoning block complete |
| `tool-call` | Tool invocation (may have more content) |
| `tool-result` | Tool response received |

### Flushing on Boundaries

```typescript
Stream.tap((part) => {
  const threadPart = { ...part, sender: agentId } as MessagePart;
  return Effect.gen(function* () {
    yield* store.appendThreadPart(threadId, threadPart);
    if (isMessageBoundary(threadPart)) {
      yield* lockedFlush(flush(store, threadId, agentId));
    }
  });
})
```

**Why flush on boundaries:**
- Ensures complete messages are persisted atomically
- Prevents partial message corruption on crash
- `tool-call` flush: tool result needs complete call context
- `text-end`/`reasoning-end`: message is complete

### Agent-Scoped Flush

```typescript
yield* flush(store, threadId, agentId);
```

**Agent scoping rationale:**
- Multi-agent channels share a thread
- Each agent has isolated parts buffer
- `readAgentParts(threadId, agentId)` - only reads this agent's parts
- Prevents cross-agent message interleaving

---

## 8. Tool-Use ID Deduplication

### validateAndRepairMessages Function

```typescript
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
  log("spawn", "WARNING: Found duplicate tool_use IDs in persisted messages, repairing", {
    agentId,
    threadId,
    duplicateIds: Array.from(duplicateIds),
  });

  // Second pass: repair by removing duplicate tool-call blocks
  const repairedSeenIds = new Set<string>();
  const repairedMessages: MessageEncoded[] = [];

  for (const message of messages) {
    if (!Array.isArray(message.content)) {
      repairedMessages.push(message);
      continue;
    }

    const repairedContent = message.content.filter((block) => {
      if (
        typeof block === "object" &&
        block !== null &&
        "type" in block &&
        block.type === "tool-call" &&
        "id" in block &&
        typeof block.id === "string"
      ) {
        if (repairedSeenIds.has(block.id)) {
          return false; // Remove duplicate
        }
        repairedSeenIds.add(block.id);
      }
      return true;
    });

    if (repairedContent.length > 0) {
      repairedMessages.push({
        ...message,
        content: repairedContent as any,
      });
    }
  }

  return repairedMessages;
}
```

### extractToolUseIds Helper

```typescript
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

**Pattern matching:**
- Checks `type === "tool-call"`
- Extracts `id` property
- Handles both array and non-array content

### Duplicate Detection and Removal

**Detection algorithm:**
```
1. seenIds = {}, duplicateIds = {}
2. For each message:
   - Extract tool_use IDs
   - If ID in seenIds → add to duplicateIds
   - Else → add to seenIds
3. If duplicateIds > 0 → repair needed
```

**Repair algorithm:**
```
1. repairedSeenIds = {}
2. For each message:
   - Filter content blocks
   - If tool-call with ID in repairedSeenIds → remove
   - Else → keep and add ID to repairedSeenIds
3. Return repaired messages
```

### Crash Recovery Scenarios

**Scenario: Crash during tool execution**

```
1. Agent sends tool-call with ID "abc123"
2. Tool executes, tool-result returned
3. CRASH before flush completes
4. Parts orphaned in buffer
5. On restart:
   - spawn() calls flush() → parts → messages
   - Messages persisted with tool-call "abc123"
   - Next spawn() calls validateAndRepairMessages()
   - If duplicate "abc123" found → removed
```

**Why duplicates occur:**
- Partial flush before crash
- Tool replay on recovery
- Multi-agent race conditions

---

## 9. Execution Trace

### Full Trace: spawn → send → flush

```
┌─────────────────────────────────────────────────────────────────────────┐
│  SPAWN AGENT                                                            │
├─────────────────────────────────────────────────────────────────────────┤
│  1. spawn(agent, threadId)                                              │
│  2. StateStore initialized                                              │
│  3. flush() - recover from crash (agent-scoped)                         │
│  4. readThreadMessages(threadId) → storedMessages                       │
│  5. validateAndRepairMessages() → repairedMessages                      │
│  6. if repaired ≠ original → writeThreadMessages()                      │
│  7. Chat.fromPrompt(storedMessages) → chat                              │
│  8. Semaphores: sem(1), flushSem(1)                                     │
│  9. Agent discovery: BFS through references → agents Map                │
│ 10. Return AgentInstance { agent, send, query }                         │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  SEND MESSAGE                                                           │
├─────────────────────────────────────────────────────────────────────────┤
│  1. send("Hello other agent") called                                    │
│  2. sem.withPermits(1) - acquire exclusive access                       │
│  3. includeContext = !contextSent (true if first call)                  │
│  4. fullPrompt = [...context.messages, {role: "user", content: "..."}]  │
│  5. contextSent = true                                                  │
│  6. chat.streamText({ toolkit, prompt }) → Stream                       │
│  7. Stream.provideLayer(toolkitHandlers)                                │
│  8. Stream.retry(exponential backoff, max 3)                            │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  STREAM EVENT FLOW                                                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  text-start                                                             │
│     │                                                                   │
│     ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ text-delta                                                      │   │
│  │    │                                                            │   │
│  │    ▼                                                            │   │
│  │ 1. tap() intercepts                                             │   │
│  │ 2. appendThreadPart({ ...part, sender: agentId })               │   │
│  │ 3. isMessageBoundary? → NO (text-delta not a boundary)          │   │
│  │ 4. map() adds sender                                            │   │
│  │ 5. → downstream consumer                                        │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│     │                                                                   │
│     ▼ (repeats for each delta)                                          │
│  text-end ◄──────────────────────────────────────────────┐              │
│     │                                                    │              │
│     ▼                                                    │              │
│  1. tap() intercepts                                     │              │
│  2. appendThreadPart()                                   │              │
│  3. isMessageBoundary? → YES (text-end)                  │              │
│  4. lockedFlush(flush())                                 │              │
│     │                                                    │              │
│     ▼                                                    │              │
│  ┌────────────────────────────────────────────────────┐ │              │
│  │ FLUSH OPERATION                                    │ │              │
│  │ 1. flushSem.withPermits(1) - acquire               │ │              │
│  │ 2. readAgentParts(threadId, agentId) → parts       │ │              │
│  │ 3. Filter non-AI parts                             │ │              │
│  │ 4. Prompt.fromResponseParts(aiParts) → prompt      │ │              │
│  │ 5. Encode messages                                 │ │              │
│  │ 6. Collect existing tool_use IDs                   │ │              │
│  │ 7. Deduplicate new messages                        │ │              │
│  │ 8. writeThreadMessagesWithSender()                 │ │              │
│  │ 9. truncateAgentParts()                            │ │              │
│  └────────────────────────────────────────────────────┘ │              │
│     │                                                   │              │
│     ▼                                                   │              │
│  map() adds sender                                      │              │
│     │                                                   │              │
│     ▼                                                   │              │
│  → consumer receives MessagePart                        │              │
│                                                         │              │
└─────────────────────────────────────────────────────────┼──────────────┘
                                                          │
┌─────────────────────────────────────────────────────────┼──────────────┐
│  TOOL CALL FLOW                                        │              │
├─────────────────────────────────────────────────────────┼──────────────┤
│  tool-call                                              │              │
│     │                                                   │              │
│     ▼                                                   │              │
│  1. tap() intercepts                                    │              │
│  2. appendThreadPart({ type: "tool-call", ... })        │              │
│  3. isMessageBoundary? → YES (tool-call)                │              │
│  4. lockedFlush(flush()) ← flushes tool-call            │              │
│     │                                                   │              │
│     ▼                                                   │              │
│  Tool executes via toolkitHandlers                      │              │
│     │                                                   │              │
│     ▼                                                   │              │
│  tool-result                                            │              │
│     │                                                   │              │
│     ▼                                                   │              │
│  1. tap() intercepts                                    │              │
│  2. appendThreadPart({ type: "tool-result", ... })      │              │
│  3. isMessageBoundary? → YES (tool-result)              │              │
│  4. lockedFlush(flush()) ← flushes tool-result          │              │
│                                                         │              │
└─────────────────────────────────────────────────────────┴──────────────┘
```

### State Mutations

```
Initial state:
┌──────────────────────────────────────────┐
│ Thread: "agent-a"                        │
│ Messages: []                             │
│ Parts (agent-a): []                      │
└──────────────────────────────────────────┘

After spawn():
┌──────────────────────────────────────────┐
│ Thread: "agent-a"                        │
│ Messages: [from history]                 │
│ Parts (agent-a): []  (flushed on spawn)  │
└──────────────────────────────────────────┘

During send() streaming:
┌──────────────────────────────────────────┐
│ Thread: "agent-a"                        │
│ Messages: [from history]                 │
│ Parts (agent-a): [                       │
│   {type: "text-start"},                  │
│   {type: "text-delta", text: "Hello"},   │
│   {type: "text-delta", text: " world"},  │
│   {type: "text-end"},                    │
│ ]                                        │
└──────────────────────────────────────────┘

After text-end triggers flush():
┌──────────────────────────────────────────┐
│ Thread: "agent-a"                        │
│ Messages: [                              │
│   ...existing,                           │
│   {role: "assistant", content: [         │
│     {type: "text", text: "Hello world"}  │
│   ], sender: "agent-a"}                  │
│ ]                                        │
│ Parts (agent-a): []  (truncated)         │
└──────────────────────────────────────────┘
```

### Error Recovery Paths

```
┌─────────────────────────────────────────────────────────────────────┐
│  ERROR RECOVERY MATRIX                                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Error Type          │ Recovery Strategy                            │
│  ────────────────────┼────────────────────────────────────────────  │
│  Rate Limit (429)    │ Retry with exponential backoff (1s, 2s, 4s)  │
│  Server Error (5xx)  │ Retry with exponential backoff               │
│  Timeout             │ Retry with exponential backoff               │
│  Invalid tool_use ID │ Repair on next spawn()                       │
│  Crash mid-stream    │ Flush on next spawn() recovers parts         │
│  Duplicate tool-call │ Deduplicate in validateAndRepairMessages()   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Retry timeline:**
```
t=0s: Request fails (429)
t=1s: Retry 1
t=3s: Retry 2 (1s + 2s exponential)
t=7s: Retry 3 (1s + 2s + 4s exponential)
     → Max retries (3) exceeded → fail
```

---

## Appendix: Key Data Structures

### MessagePart Types

```typescript
type MessagePart =
  | { type: "text-start" }
  | { type: "text-delta"; text: string }
  | { type: "text-end" }
  | { type: "reasoning-start" }
  | { type: "reasoning-delta"; text: string }
  | { type: "reasoning-end" }
  | { type: "tool-call"; ... }
  | { type: "tool-result"; ... }
  | { type: "user-input"; content: string }
  | { type: "coordinator-thinking" }
  | { type: "coordinator-invoke" }
  | { type: "coordinator-invoke-complete" };
```

### AgentInstance Interface

```typescript
interface AgentInstance<A extends Agent<string, any[]>> {
  agent: A;
  send: (prompt: string) => Stream<MessagePart, AiError | StateStoreError, ...>;
  query: <T>(prompt: string, schema: Schema<T>) => Effect<GenerateObjectResponse, ...>;
}
```

---

## Summary

The Agent system provides:

1. **Type-safe agent definitions** via Fragment extension
2. **Robust spawning** with crash recovery and message repair
3. **O(1) agent discovery** using BFS with cycle prevention
4. **Stream-based communication** with retry and attribution
5. **Structured queries** with schema validation
6. **Automatic tool registration** via Comms toolkit
7. **Message boundary detection** for atomic persistence
8. **Tool-use ID deduplication** for crash resilience
9. **Comprehensive error recovery** with exponential backoff

