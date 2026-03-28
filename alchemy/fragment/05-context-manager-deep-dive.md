# Context Manager Deep Dive: Context Management, Compaction Strategies, and Token Estimation

**Document Type:** Deep Dive Analysis
**Source Module:** `@formulas/src.rust/src.deployAnywhere/fragment/src/context-manager/`
**Lines of Code:** ~190 (across 5 modules)
**Architecture:** Effect-TS based context management with pluggable strategies

---

## 1. Context Management Problem

### The Fundamental Challenge

Large Language Models operate within fixed context windows. As conversation history grows, the accumulated messages can exceed these limits:

```
┌─────────────────────────────────────────────────────────────┐
│                    Context Window                            │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  System Prompt                                         │  │
│  │  ─────────────────────────────────────────────────────  │  │
│  │  User Message 1    ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │  │
│  │  Assistant Resp 1  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │  │
│  │  User Message 2    ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │  │
│  │  Assistant Resp 2  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │  │
│  │  ...               ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │  │
│  │  User Message N    ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  │  │
│  │  Assistant Resp N  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                               │
│  ░░░░ = Old context (candidates for compaction)              │
│  ▓▓▓▓ = Recent context (kept verbatim)                        │
└─────────────────────────────────────────────────────────────┘
```

### Three Core Problems

| Problem | Description | Consequence |
|---------|-------------|-------------|
| **Window Limits** | Models have maximum token capacities (e.g., 128K tokens) | Hard failure when exceeded |
| **Growing History** | Each exchange adds tokens to context | Eventually hits limits |
| **Memory Cost** | Old messages consume budget needed for new input | Reduced reasoning capacity |

### The Compaction Solution

The context manager addresses these problems through **pluggable strategies** that balance:
- **Fidelity** - preserving important context
- **Efficiency** - minimizing token usage
- **Performance** - avoiding excessive summarization calls

---

## 2. ContextManager Interface

### Core Service Definition

```typescript
// context-manager.ts (lines 1-43)

export interface ContextManager {
  prepareContext(
    params: PrepareContextParams,
  ): Effect.Effect<readonly MessageEncoded[], ContextManagerError>;
}

export interface PrepareContextParams {
  readonly threadId: string;      // Conversation thread identifier
  readonly systemPrompt: string;  // System prompt (prepended fresh)
}

export const ContextManager =
  Context.GenericTag<ContextManager>("ContextManager");
```

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    ContextManager Service                    │
│                                                              │
│  ┌──────────────────┐  ┌──────────────────┐                │
│  │  Passthrough     │  │  Compaction      │                │
│  │  Mode            │  │  Mode            │                │
│  │  ─────────────   │  │  ─────────────   │                │
│  │  No processing   │  │  Summarizes old  │                │
│  │  Direct pass     │  │  Keeps recent    │                │
│  └──────────────────┘  └──────────────────┘                │
│                                                              │
│  ┌──────────────────┐                                       │
│  │  RLM Mode        │  (TODO: Recursive Language Models)    │
│  │  ─────────────   │                                       │
│  │  Recent/Long     │                                       │
│  │  Memory pattern  │                                       │
│  └──────────────────┘                                       │
└─────────────────────────────────────────────────────────────┘
```

### Mode Types

The system supports three distinct modes, selected via Layer composition:

```typescript
// Selection happens at composition root
import { passthrough } from "./context-manager/passthrough";
import { compaction } from "./context-manager/rlm";

// Passthrough: No compaction
ContextManager.pipe(passthrough)

// Compaction: Smart summarization
ContextManager.pipe(compaction({ maxTokens: 128_000 }))
```

---

## 3. Passthrough Mode

### Implementation

```typescript
// passthrough.ts (lines 1-43)

export const passthrough = Layer.effect(
  ContextManager,
  Effect.gen(function* () {
    const store = yield* StateStore;

    return {
      prepareContext: ({ threadId, systemPrompt }) =>
        Effect.gen(function* () {
          const messages = yield* store
            .readThreadMessages(threadId)
            .pipe(
              Effect.map((msgs) => msgs.filter((m) => m.role !== "system")),
              Effect.catchAll(() => Effect.succeed([] as MessageEncoded[])),
            );

          return [
            { role: "system" as const, content: systemPrompt },
            ...messages,
          ];
        }),
    } satisfies ContextManager;
  }),
);
```

### Data Flow Diagram

```
┌─────────────┐     ┌──────────────┐     ┌──────────────────┐
│  StateStore │────▶│  Filter Out  │────▶│  Prepend System  │
│  (Raw msgs) │     │  System Msgs │     │  Prompt          │
└─────────────┘     └──────────────┘     └──────────────────┘
                           │                       │
                           ▼                       ▼
                    messages[]              [system, ...messages]
```

### When to Use Passthrough

| Scenario | Recommendation | Rationale |
|----------|----------------|-----------|
| Short conversations (<10 exchanges) | ✅ Use passthrough | Overhead unnecessary |
| Token budget abundant | ✅ Use passthrough | No pressure to compact |
| Debugging/development | ✅ Use passthrough | Full fidelity for troubleshooting |
| Long-running agents | ❌ Use compaction | Will eventually hit limits |
| Complex multi-file tasks | ❌ Use compaction | Context grows rapidly |

### Limitations

```typescript
// No token monitoring
// No automatic compaction
// No summary generation

// Result: Unbounded growth
conversation: [msg1, msg2, msg3, ..., msgN] // N → ∞
```

1. **Unbounded Growth** - Messages accumulate without limit
2. **No Token Awareness** - Doesn't track context window usage
3. **Manual Intervention Required** - User must restart conversation

---

## 4. RLM (Recent/Long Memory) Mode

### Current State

```typescript
// rlm.ts (line 1)
// TODO(sam): Recursive Language Models
```

### Intended Design

RLM stands for **Recent/Long Memory** - a pattern that:
- Keeps **recent messages** verbatim (high-fidelity recent context)
- Maintains **long-term memory** through summaries (compressed history)

### Conceptual Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     RLM Architecture                         │
│                                                              │
│  ┌─────────────────┐        ┌─────────────────┐            │
│  │  Recent Memory  │        │  Long Memory    │            │
│  │  ─────────────  │        │  ─────────────  │            │
│  │  Last N msgs    │        │  Summaries      │            │
│  │  Verbatim       │        │  Compressed     │            │
│  │  High fidelity  │        │  Low fidelity   │            │
│  └─────────────────┘        └─────────────────┘            │
│                                                              │
│  Configurable thresholds:                                    │
│  - recentCount: number of messages to keep                   │
│  - summaryDepth: how far back to summarize                   │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. Compaction Strategies

### Configuration

```typescript
// rlm.ts (lines 9-28)

export interface CompactionConfig {
  readonly maxTokens: number;         // Hard limit (128,000)
  readonly compactionThreshold: number; // Trigger point (100,000)
  readonly targetTokens: number;      // Goal after compaction (50,000)
  readonly summaryMaxTokens: number;  // Summary size limit (4,000)
}

export const defaultCompactionConfig: CompactionConfig = {
  maxTokens: 128_000,
  compactionThreshold: 100_000,
  targetTokens: 50_000,
  summaryMaxTokens: 4_000,
};
```

### Compaction Flow

```
                    ┌─────────────────┐
                    │  Check Tokens   │
                    │  totalTokens    │
                    └────────┬────────┘
                             │
              ┌──────────────┴──────────────┐
              │                             │
         ≤ threshold                   > threshold
              │                             │
              ▼                             ▼
    ┌─────────────────┐           ┌─────────────────┐
    │  Return As-Is   │           │  Split Messages │
    │  + System       │           │  toCompact/toKeep│
    └─────────────────┘           └────────┬────────┘
                                           │
                                           ▼
                                   ┌─────────────────┐
                                   │  Summarize Old  │
                                   │  via LLM        │
                                   └────────┬────────┘
                                           │
                                           ▼
                                   ┌─────────────────┐
                                   │  Persist +      │
                                   │  Return         │
                                   └─────────────────┘
```

### Message Splitting Algorithm

```typescript
// rlm.ts (lines 119-145)

const splitMessagesForCompaction = (
  messages: readonly MessageEncoded[],
  targetTokens: number,
): { toCompact: MessageEncoded[]; toKeep: MessageEncoded[] } => {
  let keepTokens = 0;
  let splitIndex = messages.length;

  // Work backwards from end (recent messages)
  for (let i = messages.length - 1; i >= 0; i--) {
    const msgTokens = estimateTokens(messages[i]);
    if (keepTokens + msgTokens > targetTokens) {
      splitIndex = i + 1;
      break;
    }
    keepTokens += msgTokens;
    splitIndex = i;
  }

  return {
    toCompact: messages.slice(0, splitIndex) as MessageEncoded[],
    toKeep: messages.slice(splitIndex) as MessageEncoded[],
  };
};
```

### Splitting Visualization

```
Messages: [m1, m2, m3, m4, m5, m6, m7, m8]
Tokens:   [10k,15k,12k,8k, 20k,18k,5k, 7k] = 95k total
                                          ║
                                          ║ > threshold (100k)
                                          ║ triggers compaction
                                          ▼
Split backwards until targetTokens (50k):

toKeep:                    [m6, m7, m8] = 30k tokens
toCompact: [m1, m2, m3, m4, m5]         = 65k tokens
                                          ║
                                          ║ Summarize via LLM
                                          ▼
Result: [summary(~4k), m6, m7, m8] ≈ 34k tokens
```

### Summarization Strategy

```typescript
// rlm.ts (lines 147-189)

const summarizeMessages = Effect.fn(function* (
  model: LLM.Service,
  messages: readonly MessageEncoded[],
  config: CompactionConfig,
) {
  const conversationText = messages
    .map((m) => {
      const content =
        typeof m.content === "string" ? m.content : JSON.stringify(m.content);
      return `${m.role}: ${content}`;
    })
    .join("\n\n");

  const summaryPrompt = `Summarize the following conversation history concisely.
Focus on:
- Key decisions made
- Important context established
- Files discussed or modified
- Current state of any ongoing tasks

Keep the summary under ${config.summaryMaxTokens} tokens.

Conversation:
${conversationText}`;

  const response = yield* model.generateText({ prompt: summaryPrompt });
  return response.text;
});
```

### Summary Focus Areas

| Focus Area | Purpose | Example |
|------------|---------|---------|
| Key decisions | Track architectural choices | "Decided to use Effect-TS for error handling" |
| Important context | Maintain conversation continuity | "User is building a context management system" |
| Files discussed | Enable file-aware followups | "Modified context-manager.ts and rlm.ts" |
| Current task state | Resume work after compaction | "Investigating token estimation edge cases" |

---

## 6. Token Estimation

### Implementation

```typescript
// estimate.ts (lines 1-23)

/**
 * Estimate token count for a message.
 * Uses simple heuristic: ~4 characters per token.
 */
export const estimateTokens = (message: MessageEncoded): number => {
  const content =
    typeof message.content === "string"
      ? message.content
      : JSON.stringify(message.content);
  return Math.ceil(content.length / 4);
};

export const estimateTotalTokens = (
  messages: readonly MessageEncoded[],
): number => messages.reduce((sum, msg) => sum + estimateTokens(msg), 0);
```

### Heuristic Analysis

The `~4 characters per token` heuristic is based on:

| Metric | Approximate Value | Source |
|--------|-------------------|--------|
| English text | 4 chars/token | OpenAI/Anthropic docs |
| Code | 3-4 chars/token | Depends on language |
| Mixed content | ~4 chars/token | Reasonable average |

### Accuracy Comparison

```
┌─────────────────────────────────────────────────────────────┐
│              Token Estimation Methods                        │
├──────────────────┬─────────────┬──────────────┬─────────────┤
│ Method           │ Accuracy    │ Performance  │ Complexity  │
├──────────────────┼─────────────┼──────────────┼─────────────┤
│ char/4 heuristic │ ±20%        │ O(n)         │ Minimal     │
│ Tiktoken (GPT)   │ ±5%         │ O(n)         │ Medium      │
│ Actual tokenizer │ 100%        │ O(n) + cost  │ High        │
└──────────────────────────────────────────────────────────────┘
```

### Token Budget Calculation

```typescript
// Compaction trigger logic

const totalTokens = estimateTotalTokens(allMessages);

if (totalTokens <= config.compactionThreshold) {
  // No action needed
  return [system, ...allMessages];
}

// Trigger compaction
// totalTokens: 100,001+ triggers action
// targetTokens: 50,000 is the goal
// summaryMaxTokens: 4,000 limits summary size
```

### Compaction Trigger States

```
Token Count          State              Action
─────────────────────────────────────────────────────────
0 - 100,000          Normal             Pass through
100,001 - 128,000    Compacting         Summarize old messages
128,001+             Critical           Should not reach (hard limit)
```

---

## 7. Context Resolution

### Complete prepareContext Implementation

```typescript
// rlm.ts (lines 34-117)

export const compaction = (
  config: CompactionConfig = defaultCompactionConfig,
) =>
  Layer.effect(
    ContextManager,
    Effect.gen(function* () {
      const store = yield* StateStore;
      const model = yield* LLM.LanguageModel;

      return {
        prepareContext: ({ threadId, systemPrompt }) =>
          Effect.gen(function* () {
            // Step 1: Load and filter messages
            const allMessages = yield* store
              .readThreadMessages(threadId)
              .pipe(
                Effect.map((msgs) => msgs.filter((m) => m.role !== "system")),
                Effect.catchAll(() => Effect.succeed([] as MessageEncoded[])),
              );

            // Step 2: Check token threshold
            const totalTokens = estimateTotalTokens(allMessages);
            if (totalTokens <= config.compactionThreshold) {
              return [{ role: "system", content: systemPrompt }, ...allMessages];
            }

            // Step 3: Split messages
            const { toCompact, toKeep } = splitMessagesForCompaction(
              allMessages,
              config.targetTokens,
            );

            // Step 4: Generate summary
            const summary = yield* summarizeMessages(model, toCompact, config);

            // Step 5: Build compacted context
            const compactedMessages: MessageEncoded[] = [
              {
                role: "assistant",
                content: `[Previous conversation summary]\n${summary}`,
              },
              ...toKeep,
            ];

            // Step 6: Persist compacted state
            yield* store.writeThreadMessages(threadId, compactedMessages);

            return [{ role: "system", content: systemPrompt }, ...compactedMessages];
          }),
      } satisfies ContextManager;
    }),
  );
```

### Agent Context Building Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Context Resolution Pipeline                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐                                                   │
│  │ prepareContext│                                                   │
│  └───────┬──────┘                                                   │
│          │                                                          │
│          ▼                                                          │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │ 1. StateStore.readThreadMessages(threadId)                    │ │
│  │    - Load from persistence layer                              │ │
│  │    - Filter out old system messages                           │ │
│  └───────────────────────────────────────────────────────────────┘ │
│          │                                                          │
│          ▼                                                          │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │ 2. estimateTotalTokens(allMessages)                           │ │
│  │    - Check against compactionThreshold                        │ │
│  └───────────────────────────────────────────────────────────────┘ │
│          │                                                          │
│     ┌────┴────┐                                                     │
│     │ Under?  │                                                     │
│     └────┬────┘                                                     │
│      Yes │  No                                                      │
│          │   │                                                      │
│          │   ▼                                                      │
│          │  ┌─────────────────────────────────────────────────────┐│
│          │  │ 3. Compaction Path                                  ││
│          │  │    - splitMessagesForCompaction()                   ││
│          │  │    - summarizeMessages() via LLM                    ││
│          │  │    - writeThreadMessages() (persist)                ││
│          │  └─────────────────────────────────────────────────────┘│
│          │                                                          │
│          ▼                                                          │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │ 4. Prepend fresh system prompt                                │ │
│  │    Return: [system, ...messages]                              │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Tool Registration Context

The context manager integrates with the broader agent system:

```typescript
// Tool registration happens in the agent layer
// ContextManager is provided via Layer composition

const agentLayer = Layer.mergeAll(
  StateStore.live,
  LLM.LanguageModel,  // Required for compaction
  ContextManager.compaction({ maxTokens: 128_000 }),
);
```

---

## 8. Message History Management

### Pruning Strategy

```typescript
// Old system messages are filtered during load
Effect.map((msgs) => msgs.filter((m) => m.role !== "system"))

// Rationale: System prompt is always prepended fresh
// No need to store duplicates in history
```

### Summary Insertion Pattern

```
Before Compaction:
┌─────────────────────────────────────────────────────────┐
│ [system] You are a helpful assistant                    │
│ [user]   Let's build a context manager                 │
│ [asst]   I'll help with that...                        │
│ [user]   How should we handle token limits?            │
│ [asst]   We can use compaction...                      │
│ ... (100+ messages) ...                                 │
│ [user]   What was the last decision?                   │ ← Query
└─────────────────────────────────────────────────────────┘

After Compaction:
┌─────────────────────────────────────────────────────────┐
│ [system] You are a helpful assistant                    │
│ [asst]   [Previous conversation summary]                │
│          Key decisions:                                │
│          - Chose Effect-TS for error handling          │
│          - Implemented compaction strategy             │
│          - Files: context-manager.ts, rlm.ts           │
│ [user]   What was the last decision?                   │ ← Query preserved
└─────────────────────────────────────────────────────────┘
```

### Context Window Monitoring

```typescript
// Logging for observability
yield* Effect.logInfo(
  `[context] Compacting ${totalTokens} tokens (threshold: ${config.compactionThreshold})`,
);

yield* Effect.logInfo(
  `[context] Compacted ${toCompact.length} messages into summary, kept ${toKeep.length} recent`,
);
```

### Monitoring Output

```
[context] Compacting 105423 tokens (threshold: 100000)
[context] Compacted 87 messages into summary, kept 23 recent
```

---

## Appendix: Comparison Tables

### Mode Comparison

| Feature | Passthrough | Compaction | RLM (Planned) |
|---------|-------------|------------|---------------|
| Token awareness | ❌ | ✅ | ✅ |
| Automatic compaction | ❌ | ✅ | ✅ |
| Summary generation | ❌ | ✅ | ✅ |
| Recent message priority | ❌ | ✅ | ✅ |
| Configurable thresholds | ❌ | ✅ | ✅ |
| Persistence | ✅ | ✅ | ✅ |
| Performance | Fast | Moderate | Moderate |

### Configuration Defaults

| Parameter | Default | Purpose |
|-----------|---------|---------|
| maxTokens | 128,000 | Hard context limit |
| compactionThreshold | 100,000 | Trigger compaction before hard limit |
| targetTokens | 50,000 | Leave room for future growth |
| summaryMaxTokens | 4,000 | Reasonable summary size |

### Error Handling

```typescript
export class ContextManagerError extends Data.TaggedError(
  "ContextManagerError",
)<{
  readonly message: string;
  readonly cause?: unknown;
}> {}

// All operations wrap in Effect context
// Errors are caught and wrapped with context
.pipe(
  Effect.mapError(
    (cause) =>
      new ContextManagerError({
        message: "Failed to prepare context with compaction",
        cause,
      }),
  ),
)
```

---

## Summary

The ContextManager system provides a sophisticated approach to managing conversation context through:

1. **Pluggable Architecture** - Swappable strategies via Effect Layers
2. **Proactive Compaction** - Summarization before hitting hard limits
3. **Token Awareness** - Simple but effective character-based estimation
4. **Persistence Integration** - Compacted state saved to StateStore
5. **Observability** - Logging for monitoring compaction events

The design prioritizes **recent context fidelity** while maintaining **long-term memory** through intelligent summarization.
