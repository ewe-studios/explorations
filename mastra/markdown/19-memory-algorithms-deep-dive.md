---
source: mastra-memory/exploration.md
repository: https://github.com/mastra-ai/mastra
deep_dive_at: 2026-03-19T00:00:00.000Z
type: memory-algorithms
---

# Mastra Memory Algorithms Deep Dive

This document provides implementation-level detail on Mastra's memory algorithms, written so that a developer without university-level CS training could understand and rebuild them from scratch.

---

## Table of Contents

1. [Working Memory System](#1-working-memory-system)
2. [Semantic Recall with Vector Search](#2-semantic-recall-with-vector-search)
3. [Observational Memory (Observer + Reflector)](#3-observational-memory-observer--reflector)
4. [Token Estimation](#4-token-estimation)
5. [Caching Strategies](#5-caching-strategies)

---

## 1. Working Memory System

### What It Is

Working Memory is like a sticky note that the AI keeps on its desk. Every time you talk to the AI, it reads this sticky note first to remember important facts about you.

### Data Structure

```typescript
// Stored in database
interface WorkingMemoryRecord {
  resourceId: string;      // User ID
  content: string;         // Markdown or JSON string
  updatedAt: Date;
}
```

### Two Modes of Operation

#### Mode 1: Template-Based (Markdown)

**Schema:**
```typescript
{
  format: 'markdown',
  content: `
# User Information
- **Name**: John Doe
- **Location**: San Francisco
- **Goals**: Build a todo app
`
}
```

**Update Behavior:** REPLACE - entire content is overwritten

**How It Works:**
```typescript
async function updateWorkingMemory(newContent: string) {
  // Simple overwrite - everything is replaced
  await db.workingMemory.update({
    where: { resourceId },
    data: { content: newContent, updatedAt: new Date() }
  });
}
```

#### Mode 2: Schema-Based (JSON with Merge)

**Schema:**
```typescript
{
  format: 'json',
  content: {
    name: "John Doe",
    location: "San Francisco",
    projects: ["todo app", "weather app"]
  }
}
```

**Update Behavior:** DEEP MERGE - only specified fields change

**Deep Merge Algorithm:**

```typescript
/**
 * Merges two objects with special rules:
 * - null = delete this property
 * - arrays = replace entirely (don't merge items)
 * - objects = merge recursively
 * - other values = overwrite
 */
function deepMerge(existing, update) {
  // Handle empty updates
  if (!update || typeof update !== 'object') {
    return existing || {};
  }

  const result = { ...existing };

  for (const key in update) {
    const updateValue = update[key];
    const existingValue = result[key];

    // Rule 1: null means delete
    if (updateValue === null) {
      delete result[key];
    }
    // Rule 2: arrays replace entirely
    else if (Array.isArray(updateValue)) {
      result[key] = updateValue;
    }
    // Rule 3: objects merge recursively
    else if (
      typeof updateValue === 'object' &&
      typeof existingValue === 'object' &&
      existingValue !== null
    ) {
      result[key] = deepMerge(existingValue, updateValue);
    }
    // Rule 4: everything else overwrites
    else {
      result[key] = updateValue;
    }
  }

  return result;
}
```

**Example Usage:**
```typescript
// Existing memory
const existing = {
  name: "John",
  location: "SF",
  projects: ["app1"]
};

// Update (only change location)
const update = {
  location: "NYC"  // Other fields preserved
};

const merged = deepMerge(existing, update);
// Result: { name: "John", location: "NYC", projects: ["app1"] }

// Update with null (delete)
const update2 = { location: null };
const merged2 = deepMerge(existing, update2);
// Result: { name: "John", projects: ["app1"] }
```

### Scope: Thread vs Resource

```typescript
// Thread-scoped: Each conversation has its own working memory
const thread = await db.thread.find(threadId);
const workingMemory = thread.metadata.workingMemory;

// Resource-scoped: Shared across all conversations for one user
const resource = await db.resource.find(resourceId);
const workingMemory = resource.workingMemory;
```

---

## 2. Semantic Recall with Vector Search

### What It Is

Semantic Recall finds past messages that are similar to what the user just said, even if they use different words.

### How Vector Search Works (Simplified)

**Step 1: Convert Text to Numbers**

Every sentence gets converted to a list of numbers (called a "vector"). Similar sentences have similar numbers.

```
"I love coding"     → [0.12, -0.45, 0.78, 0.23, ...]  // 1536 numbers
"I enjoy programming" → [0.15, -0.42, 0.75, 0.21, ...]  // Very similar!
"I hate food"       → [-0.67, 0.89, -0.34, -0.12, ...] // Very different
```

**Step 2: Measure Similarity**

Use "cosine similarity" to measure how alike two vectors are:

```typescript
function cosineSimilarity(vecA, vecB) {
  // Dot product
  let dotProduct = 0;
  for (let i = 0; i < vecA.length; i++) {
    dotProduct += vecA[i] * vecB[i];
  }

  // Magnitudes
  const magnitudeA = Math.sqrt(vecA.reduce((sum, val) => sum + val * val, 0));
  const magnitudeB = Math.sqrt(vecB.reduce((sum, val) => sum + val * val, 0));

  // Cosine similarity = dot product / (magnitude A * magnitude B)
  return dotProduct / (magnitudeA * magnitudeB);
}

// Result: 1.0 = identical, 0 = unrelated, -1 = opposite
```

**Step 3: Find Most Similar**

```typescript
async function findSimilarMessages(query: string, allMessages, topK = 4) {
  // Step 3a: Convert query to vector
  const queryVector = await embedder.embed(query);

  // Step 3b: Compare to all stored message vectors
  const scores = allMessages.map(msg => ({
    message: msg,
    similarity: cosineSimilarity(queryVector, msg.vector)
  }));

  // Step 3c: Sort by similarity (highest first)
  scores.sort((a, b) => b.similarity - a.similarity);

  // Step 3d: Return top K matches
  return scores.slice(0, topK).map(s => s.message);
}
```

### Complete Implementation

```typescript
class SemanticRecall {
  constructor(options: {
    storage: Storage;       // Message database
    vector: VectorStore;    // Vector database
    embedder: Embedder;     // Text → vector converter
    topK?: number;          // How many results (default: 4)
    messageRange?: number;  // Context messages (default: 1)
  }) {
    this.storage = options.storage;
    this.vector = options.vector;
    this.embedder = options.embedder;
    this.topK = options.topK || 4;
    this.messageRange = options.messageRange || 1;
  }

  async processInput(args: {
    messages: Message[];    // Current conversation
    threadId: string;
    resourceId?: string;
  }): Promise<Message[]> {
    // Step 1: Extract user's query from last message
    const userQuery = this.extractUserQuery(args.messages);
    if (!userQuery) return [];  // No query to search

    // Step 2: Convert query to vector
    const queryVector = await this.embedder.embed(userQuery);

    // Step 3: Search vector store
    const results = await this.vector.query({
      queryVector,
      topK: this.topK,
      filter: { threadId: args.threadId }  // Or resourceId for cross-thread
    });

    // Step 4: Get surrounding context for each match
    const messagesWithContext = [];
    for (const result of results) {
      const context = await this.storage.getMessagesWithContext({
        messageId: result.metadata.message_id,
        before: this.messageRange,
        after: this.messageRange
      });
      messagesWithContext.push(...context);
    }

    return messagesWithContext;
  }

  async processOutput(args: {
    newMessages: Message[];  // Messages to save
  }): Promise<void> {
    // Create embeddings for new messages being saved
    for (const message of args.newMessages) {
      const text = this.extractText(message);
      if (!text) continue;

      const vector = await this.embedder.embed(text);

      await this.vector.upsert({
        id: message.id,
        vector,
        metadata: {
          message_id: message.id,
          thread_id: message.threadId,
          content: text,
          created_at: message.createdAt.toISOString()
        }
      });
    }
  }

  extractUserQuery(messages: Message[]): string | null {
    // Find last user message
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].role === 'user') {
        return this.extractText(messages[i]);
      }
    }
    return null;
  }

  extractText(message: Message): string {
    if (typeof message.content === 'string') {
      return message.content;
    }
    // Handle complex message formats
    if (message.content?.parts) {
      return message.content.parts
        .filter(p => p.type === 'text')
        .map(p => p.text)
        .join('\n');
    }
    return '';
  }
}
```

### Caching Embeddings

To avoid paying for the same embedding twice, cache results:

```typescript
import { LRUCache } from 'lru-cache';

// Global cache shared across all instances
const embeddingCache = new LRUCache({ max: 1000 });

async function embedWithCache(text, indexName) {
  // Create cache key from content hash + index name
  const hash = await xxhash(`${indexName}:${text}`);
  const cached = embeddingCache.get(hash);

  if (cached) return cached;

  // Generate new embedding
  const embedding = await embedder.embed(text);

  // Store in cache
  embeddingCache.set(hash, embedding);

  return embedding;
}
```

### Index Optimization

For large datasets, create indexes to speed up search:

```typescript
// HNSW (Hierarchical Navigable Small World) - Fast, good recall
await vector.createIndex({
  indexName: 'memory_messages',
  dimension: 1536,  // OpenAI embedding size
  metric: 'cosine',
  type: 'hnsw',
  hnsw: {
    m: 16,              // Connections per node
    efConstruction: 64  // Index quality (higher = better but slower)
  }
});

// IVFFlat (Inverted File) - Good balance
await vector.createIndex({
  indexName: 'memory_messages',
  dimension: 1536,
  metric: 'cosine',
  type: 'ivfflat',
  ivf: {
    lists: 100  // Number of clusters
  }
});
```

---

## 3. Observational Memory (Observer + Reflector)

### What It Is

Observational Memory is like having a note-taker (Observer) who summarizes conversations, and an editor (Reflector) who condenses those summaries when they get too long.

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     ACTOR (Main Agent)                       │
│  Sees: Observations + Recent Unobserved Messages            │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
    ┌─────────▼────────┐           ┌─────────▼────────┐
    │    OBSERVER      │           │    REFLECTOR     │
    │ (Extracts facts) │           │ (Condenses info) │
    │ Trigger: 30k tok │           │ Trigger: 40k tok │
    └──────────────────┘           └──────────────────┘
```

### Observer Agent

**Purpose:** Extract key facts from conversation into structured observations

**Prompt Structure:**

```
You are the memory consciousness of an AI assistant.
Your observations will be the ONLY information the assistant has
about past interactions with this user.

=== EXTRACTION INSTRUCTIONS ===

1. DISTINGUISH USER ASSERTIONS FROM QUESTIONS:
   - "I have two kids" → 🔴 User stated has two kids
   - "How many kids do I have?" → 🔴 User asked about kids

2. TEMPORAL ANCHORING:
   Each observation gets TWO timestamps:
   - When it was said (from message timestamp)
   - When it references (if different)

   Example:
   (June 15, 9:15 AM) User will visit parents this weekend.
   (meaning June 17-18, 2025)

3. PRESERVE UNUSUAL PHRASING:
   BAD: User exercised.
   GOOD: User did a "movement session" (their term for exercise).

4. USE PRECISE VERBS:
   BAD: User got something.
   GOOD: User purchased / received / was given something.

5. PRESERVE DETAILS:
   - Names, handles, quantities
   - Sequences and steps
   - Technical values

=== OUTPUT FORMAT ===

<observations>
Date: Dec 4, 2025
* 🔴 (14:30) User prefers direct answers
* 🟡 (14:31) Working on feature X
  * -> viewed auth.ts
  * -> found missing null check
</observations>

<current-task>What agent is working on</current-task>
<suggested-response>Hint for next message</suggested-response>
```

**Parsing Output:**

```typescript
function parseObserverOutput(output: string) {
  // Extract <observations> block
  const obsMatch = output.match(/<observations>([\s\S]*?)<\/observations>/i);
  const observations = obsMatch ? obsMatch[1].trim() : '';

  // Extract <current-task>
  const taskMatch = output.match(/<current-task>([\s\S]*?)<\/current-task>/i);
  const currentTask = taskMatch ? taskMatch[1].trim() : '';

  // Extract <suggested-response>
  const respMatch = output.match(/<suggested-response>([\s\S]*?)<\/suggested-response>/i);
  const suggestedResponse = respMatch ? respMatch[1].trim() : '';

  // Clean up observations
  const sanitized = sanitizeObservationLines(observations);

  return {
    observations: sanitized,
    currentTask,
    suggestedResponse
  };
}

function sanitizeObservationLines(text: string): string {
  const MAX_LINE_LENGTH = 10000;
  return text.split('\n').map(line => {
    if (line.length > MAX_LINE_LENGTH) {
      return line.slice(0, MAX_LINE_LENGTH) + ' ... [truncated]';
    }
    return line;
  }).join('\n');
}
```

### Reflector Agent

**Purpose:** Condense observations when they get too large

**Key Insight:** The Reflector's output becomes the ENTIRE memory - anything not included is forgotten.

**Prompt Structure:**

```
You are the observation reflector.
Your reason for existing is to reflect on all observations,
re-organize and streamline them, and draw connections.

IMPORTANT: Your reflections are THE ENTIRETY of the assistant's memory.
Any information you do not add will be immediately forgotten.

When consolidating:
- Preserve dates/times when present
- Combine related items (e.g., "agent called view tool 5 times")
- Condense older observations more aggressively
- Retain more detail for recent observations

=== OUTPUT FORMAT ===

<observations>
Date: Dec 4, 2025
* 🔴 (14:30) User prefers direct answers
* 🟡 (14:30-14:45) Agent debugged auth issue:
  * Browsed 5 files in src/auth/
  * Found missing null check at line 47
  * Applied fix, tests now pass
</observations>

<current-task>Current priority task</current-task>
<suggested-response>Next response hint</suggested-response>
```

**Compression Validation:**

```typescript
function validateCompression(
  reflectedTokens: number,
  targetThreshold: number
): boolean {
  // Reflection must actually be smaller than input
  return reflectedTokens < targetThreshold;
}

// If validation fails, retry with stronger compression guidance
const COMPRESSION_GUIDANCE = {
  0: '',  // No guidance (first attempt)
  1: `
## COMPRESSION REQUIRED

Your reflection was the same size or larger than the original.
Please compress slightly more:
- Condense early observations into higher-level summaries
- Retain fine details for recent items
- Combine related observations
  `,
  2: `
## AGGRESSIVE COMPRESSION REQUIRED

Your reflection is still too large.
Please compress more aggressively:
- Heavily condense oldest observations
- Remove redundant information
- Keep only key facts, decisions, outcomes
  `,
  3: `
## CRITICAL COMPRESSION REQUIRED

Maximum compression needed:
- Summarize first 50-70% into brief paragraphs
- Ruthlessly merge related items
- Drop procedural details, keep final outcomes
  `
};
```

### Async Buffering System

**Problem:** Waiting for Observer/Reflector to finish blocks the response.

**Solution:** Run compression in background, activate instantly when needed.

**How It Works:**

```typescript
class ObservationalMemory {
  // Configuration
  messageTokens = 30000;      // When observation triggers
  bufferTokens = 0.2;         // Buffer at 20% of threshold (6000 tokens)
  bufferActivation = 0.8;     // Activate 80% of buffer

  // State
  bufferedChunks = [];
  unobservedTokens = 0;

  async processOutput(messages: Message[]) {
    // Count tokens in new messages
    const newTokens = countTokens(messages);
    this.unobservedTokens += newTokens;

    // Check if we should buffer (run in background)
    const bufferThreshold = this.messageTokens * this.bufferTokens;
    if (this.unobservedTokens >= bufferThreshold && !this.isBuffering) {
      this.startBuffering();  // Runs async, doesn't block
    }

    // Check if we need to activate buffer
    if (this.unobservedTokens >= this.messageTokens) {
      if (this.bufferedChunks.length > 0) {
        // Instant activation - no LLM call needed!
        this.activateBuffer();
      } else {
        // No buffer available - must wait for observation
        await this.runObservation();  // Blocking
      }
    }
  }

  async startBuffering() {
    this.isBuffering = true;

    // Call Observer LLM in background
    const observations = await callObserver(this.getUnobservedMessages());

    // Store result without activating yet
    this.bufferedChunks.push({
      observations,
      messageRange: { start, end },  // Which messages this covers
      tokens: countTokens(observations)
    });

    this.isBuffering = false;
  }

  activateBuffer() {
    // Take buffered chunks and make them active
    const chunksToActivate = this.bufferedChunks.slice(
      0,
      Math.ceil(this.bufferedChunks.length * this.bufferActivation)
    );

    // Calculate how many messages this will remove
    const messagesToRemove = chunksToActivate.reduce(
      (sum, chunk) => sum + (chunk.messageRange.end - chunk.messageRange.start),
      0
    );

    // Update active observations
    this.activeObservations = chunksToActivate
      .map(c => c.observations)
      .join('\n');

    // Update token counts
    this.unobservedTokens -= messagesToRemove;

    // Keep remaining chunks in buffer
    this.bufferedChunks = this.bufferedChunks.slice(
      chunksToActivate.length
    );
  }
}
```

### Degeneracy Detection

LLMs sometimes get stuck in loops, repeating the same content. Detect this:

```typescript
function detectDegenerateRepetition(text: string): boolean {
  if (!text || text.length < 2000) return false;

  // Strategy 1: Check for repeated windows
  const windowSize = 200;
  const step = Math.max(1, Math.floor(text.length / 50));
  const seen = new Map();
  let duplicates = 0;
  let total = 0;

  for (let i = 0; i + windowSize <= text.length; i += step) {
    const window = text.slice(i, i + windowSize);
    total++;
    const count = (seen.get(window) || 0) + 1;
    seen.set(window, count);
    if (count > 1) duplicates++;
  }

  // If >40% of windows are duplicates → degenerate
  if (total > 5 && duplicates / total > 0.4) {
    return true;
  }

  // Strategy 2: Check for extremely long lines
  for (const line of text.split('\n')) {
    if (line.length > 50000) return true;
  }

  return false;
}
```

---

## 4. Token Estimation

### Simple Token Counter

For quick estimates without calling the tokenizer API:

```typescript
function estimateTokens(text: string): number {
  // English words average ~4-5 characters
  // Add 20% for punctuation, special tokens
  const words = text.split(/\s+/).length;
  return Math.ceil(words * 1.3);
}

// More accurate for code
function estimateCodeTokens(text: string): number {
  // Code has more symbols, shorter tokens
  const chars = text.length;
  return Math.ceil(chars / 4);  // ~4 chars per token
}
```

### Model-Specific Estimation

```typescript
const MODEL_RATIOS = {
  'gpt-4': 1.3,      // Words to tokens
  'gpt-3.5': 1.3,
  'claude-3': 1.2,
  'gemini': 1.4,
};

function estimateForModel(text: string, modelId: string): number {
  const ratio = MODEL_RATIOS[modelId] || 1.3;
  const words = text.split(/\s+/).length;
  return Math.ceil(words * ratio);
}
```

---

## 5. Caching Strategies

### LRU Cache for Embeddings

```typescript
import { LRUCache } from 'lru-cache';

// Create cache with max size
const cache = new LRUCache({
  max: 1000,  // Keep 1000 entries
  ttl: 1000 * 60 * 60,  // Optional: expire after 1 hour
});

// Generate cache key
async function makeCacheKey(text: string, modelId: string): Promise<string> {
  // Use fast hash (xxhash) instead of storing full text
  const hash = await xxhash(`${modelId}:${text}`);
  return hash.toString(16);  // Convert to hex string
}

// Use cache
async function embedWithCache(text: string) {
  const key = await makeCacheKey(text, 'text-embedding-3-small');
  const cached = cache.get(key);

  if (cached) {
    console.log('Cache hit!');
    return cached;
  }

  // Generate new embedding
  const embedding = await embedder.embed(text);
  cache.set(key, embedding);

  return embedding;
}
```

### Why LRU?

LRU (Least Recently Used) automatically removes old entries when full:

```
Cache: [A, B, C, D, E]  (max 5)

Access A → [B, C, D, E, A]  (A moved to end)
Add F →   [B, C, D, E, A, F] → [C, D, E, A, F]  (B evicted)
```

---

## Implementation Checklist

To rebuild this system from scratch:

### Working Memory
- [ ] Database schema for working memory (resourceId, content, updatedAt)
- [ ] Deep merge function with null-delete, array-replace semantics
- [ ] Thread vs resource scope handling
- [ ] Template formatting (Markdown/JSON)

### Semantic Recall
- [ ] Embedding generation (call embedder API)
- [ ] Vector storage (upsert, query operations)
- [ ] Cosine similarity calculation
- [ ] Context retrieval (get surrounding messages)
- [ ] LRU cache for embeddings
- [ ] Index creation (HNSW/IVFFlat)

### Observational Memory
- [ ] Observer prompt construction
- [ ] Reflector prompt construction
- [ ] Output parsing (XML tag extraction)
- [ ] Token counting
- [ ] Async buffering system
- [ ] Degeneracy detection
- [ ] Compression validation

### Infrastructure
- [ ] Message storage (save, retrieve, list)
- [ ] Thread management (create, get, delete)
- [ ] Resource tracking
- [ ] Processor pipeline (input/output)

---

## Performance Characteristics

| Operation | Time Complexity | Notes |
|-----------|-----------------|-------|
| Working Memory Read | O(1) | Single DB lookup |
| Working Memory Write | O(1) | Single DB update |
| Semantic Recall Query | O(log n) | Vector index lookup |
| Embedding Generation | O(n) | Proportional to text length |
| Observation (Observer) | O(n) | LLM call on n tokens |
| Reflection (Reflector) | O(n) | LLM call on n tokens |
| Buffer Activation | O(1) | Instant swap |

---

## Common Pitfalls and Solutions

### 1. Embedding Dimension Mismatch

**Problem:** Different embedding models produce different dimensions.

**Solution:** Probe embedder on startup:
```typescript
async function getEmbeddingDimension(embedder) {
  const result = await embedder.embed(['test']);
  return result[0].length;  // e.g., 1536 for OpenAI
}
```

### 2. Observation Loop Degeneracy

**Problem:** LLM gets stuck repeating same observations.

**Solution:** Detect repetition, retry with different model or temperature.

### 3. Memory Leaks from Caching

**Problem:** Cache grows unbounded.

**Solution:** Use LRU cache with explicit max size.

### 4. Cross-Thread Message Duplication

**Problem:** Same message appears multiple times from different threads.

**Solution:** Deduplicate by message ID before adding to context.

### 5. Stale Buffer After Crash

**Problem:** Process crashes mid-buffering, leaving stale state.

**Solution:** Track active operations in DB with heartbeat, clean up stale flags on startup.
