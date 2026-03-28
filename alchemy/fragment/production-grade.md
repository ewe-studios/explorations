# Production-Grade Fragment: Deployment and Operations Guide

## Overview

This document provides a comprehensive guide for deploying fragment-based AI agent systems to production environments. While the core Fragment framework is designed for educational and prototyping purposes, the patterns and architectures described here scale to enterprise-grade deployments.

**Target Audience:**
- DevOps engineers deploying AI agent systems
- Platform engineers building agent infrastructure
- SREs responsible for agent system reliability
- Architects designing multi-agent architectures

**Prerequisites:**
- Understanding of Fragment core concepts (see [01-fragment-architecture-deep-dive.md](01-fragment-architecture-deep-dive.md))
- Experience with containerization and orchestration
- Familiarity with monitoring and observability tools

---

## 1. What Fragment Leaves Out (Production Considerations)

### 1.1 Educational vs Production Trade-offs

Fragment's current implementation prioritizes clarity and flexibility over production hardening:

| Aspect | Fragment (Current) | Production Requirement |
|--------|-------------------|----------------------|
| **State Storage** | SQLite (single-file) | PostgreSQL with replication |
| **Agent Execution** | Single-threaded Effect runtime | Distributed worker pool |
| **Communication** | In-memory send/query | Message queues (RabbitMQ, Kafka) |
| **Scaling** | Vertical (bigger machine) | Horizontal (more agents) |
| **Availability** | Single-node | Multi-region failover |
| **Security** | Basic input validation | Defense-in-depth, sandboxing |

### 1.2 Performance Gaps

**Current Limitations:**

1. **Sequential Tool Execution**
   ```typescript
   // Current: Sequential execution
   const result1 = yield* tool1(params1);
   const result2 = yield* tool2(params2);
   const result3 = yield* tool3(params3);
   // Total time: t1 + t2 + t3
   ```

   **Production Pattern: Parallel execution**
   ```typescript
   // Production: Batch parallel execution
   const [result1, result2, result3] = yield* Effect.all([
     tool1(params1),
     tool2(params2),
     tool3(params3)
   ]);
   // Total time: max(t1, t2, t3)
   ```

2. **No Connection Pooling**
   - Each tool call creates new database/API connections
   - High latency under load
   - Resource exhaustion risk

3. **Synchronous Context Resolution**
   - `collectReferences()` blocks during deep traversals
   - No caching of resolved contexts
   - Repeated computation for recurring agent interactions

### 1.3 Scale Limitations

| Metric | Fragment (Single-Node) | Production Target |
|--------|----------------------|------------------|
| **Agents per Node** | ~100-500 | 10,000+ |
| **Messages/Second** | ~1,000 | 100,000+ |
| **Context Window** | ~128K tokens | Distributed (vector DB) |
| **State Size** | ~10 GB (SQLite) | Multi-TB (sharded) |
| **Concurrent Users** | ~50 | 10,000+ |

### 1.4 Missing Production Features

- [ ] Rate limiting and throttling
- [ ] Circuit breakers for external APIs
- [ ] Request queuing and backpressure
- [ ] Distributed tracing
- [ ] Hot reloading of agent definitions
- [ ] A/B testing framework
- [ ] Canary deployments
- [ ] Audit logging
- [ ] RBAC/authorization
- [ ] Multi-tenancy isolation

---

## 2. Production Architecture Overview

### 2.1 Single-Node Deployment

For small-scale deployments (development, staging, or low-traffic production):

```
┌─────────────────────────────────────────────────────────────────┐
│                        Single Node                               │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Container/Pod                             ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         ││
│  │  │   Agent 1   │  │   Agent 2   │  │   Agent N   │         ││
│  │  │  (Effect)   │  │  (Effect)   │  │  (Effect)   │         ││
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         ││
│  │         │                │                │                 ││
│  │         └────────────────┼────────────────┘                 ││
│  │                          │                                  ││
│  │              ┌───────────▼───────────┐                      ││
│  │              │   Effect Runtime      │                      ││
│  │              │   (Single-threaded)   │                      ││
│  │              └───────────┬───────────┘                      ││
│  │                          │                                  ││
│  │    ┌──────────┬──────────┼──────────┬──────────┐           ││
│  │    │          │          │          │          │           ││
│  │  ┌─▼──┐  ┌───▼──┐  ┌───▼───┐  ┌───▼───┐  ┌──▼──┐        ││
│  │  │HTTP│  │ WS   │  │ State │  │ Tools │  │ TUI │        ││
│  │  │API │  │Server│  │ Store │  │       │  │     │        ││
│  │  └────┘  └──────┘  └───────┘  └───────┘  └─────┘        ││
│  └─────────────────────────────────────────────────────────────┘│
│                              │                                   │
│                    ┌─────────▼─────────┐                        │
│                    │   SQLite/Postgres │                        │
│                    │   (Local or Net)  │                        │
│                    └───────────────────┘                        │
└─────────────────────────────────────────────────────────────────┘
```

**Configuration Example: `docker-compose.yml`**

```yaml
version: '3.8'

services:
  fragment-agent:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "3000:3000"   # HTTP API
      - "3001:3001"   # WebSocket
    environment:
      - NODE_ENV=production
      - DATABASE_URL=postgresql://user:pass@postgres:5432/fragment
      - REDIS_URL=redis://redis:6379
      - LOG_LEVEL=info
      - MAX_CONCURRENT_AGENTS=100
      - CONTEXT_WINDOW_SIZE=128000
    volumes:
      - ./config:/app/config
      - agent-state:/app/state
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    restart: unless-stopped

  postgres:
    image: postgres:16-alpine
    environment:
      - POSTGRES_USER=fragment
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=fragment
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U fragment"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  postgres-data:
  redis-data:
  agent-state:
```

### 2.2 Multi-Agent Cluster

For production-scale deployments:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              Load Balancer                               │
│                         (nginx/HAProxy/ALB)                             │
└─────────────────────────────────┬───────────────────────────────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
    ┌─────▼─────┐           ┌─────▼─────┐           ┌─────▼─────┐
    │  Node 1   │           │  Node 2   │           │  Node N   │
    │ ┌───────┐ │           │ ┌───────┐ │           │ ┌───────┐ │
    │ │ Agents│ │           │ │ Agents│ │           │ │ Agents│ │
    │ │ Pool  │ │           │ │ Pool  │ │           │ │ Pool  │ │
    │ └───────┘ │           │ └───────┘ │           │ └───────┘ │
    └─────┬─────┘           └─────┬─────┘           └─────┬─────┘
          │                       │                       │
          └───────────────────────┼───────────────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
    ┌─────▼─────┐           ┌─────▼─────┐           ┌─────▼─────┐
    │  Postgres │           │   Redis   │           │   Kafka   │
    │  (Primary)│           │  (Cache)  │           │  (Queue)  │
    └─────┬─────┘           └───────────┘           └───────────┘
          │
    ┌─────▼─────┐
    │  Postgres │
    │  (Replica)│
    └───────────┘
```

**Kubernetes Deployment: `fragment-cluster.yaml`**

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: fragment-config
data:
  NODE_ENV: "production"
  LOG_LEVEL: "info"
  MAX_CONCURRENT_AGENTS: "500"
  CONTEXT_WINDOW_SIZE: "128000"
  REDIS_URL: "redis://fragment-redis:6379"
  KAFKA_BROKERS: "fragment-kafka:9092"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: fragment-agents
spec:
  replicas: 5
  selector:
    matchLabels:
      app: fragment-agents
  template:
    metadata:
      labels:
        app: fragment-agents
    spec:
      containers:
      - name: fragment
        image: fragment:latest
        ports:
        - containerPort: 3000
        - containerPort: 3001
        envFrom:
        - configMapRef:
            name: fragment-config
        - secretRef:
            name: fragment-secrets
        resources:
          requests:
            cpu: "500m"
            memory: "1Gi"
          limits:
            cpu: "2000m"
            memory: "4Gi"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: fragment-service
spec:
  selector:
    app: fragment-agents
  ports:
  - name: http
    port: 80
    targetPort: 3000
  - name: websocket
    port: 8001
    targetPort: 3001
  type: ClusterIP
```

### 2.3 Load Balancing Strategies

**Strategy 1: Round-Robin (Default)**
```yaml
# nginx.conf
upstream fragment_backend {
    least_conn;  # Least connections algorithm
    server fragment-1:3000 weight=1;
    server fragment-2:3000 weight=1;
    server fragment-3:3000 weight=1;
}
```

**Strategy 2: Agent-Affinity (Sticky Sessions)**
```yaml
# HAProxy configuration for agent session affinity
backend fragment_backend
    balance source
    stick-table type string size 100k expire 30m
    stick on hdr(X-Agent-Session-ID)
    server fragment-1 fragment-1:3000 check
    server fragment-2 fragment-2:3000 check
    server fragment-3 fragment-3:3000 check
```

**Strategy 3: Request-Type Routing**
```yaml
# Route based on request type
http {
    map $http_x_request_type $upstream {
        "realtime"    fragment_realtime;
        "batch"       fragment_batch;
        "streaming"   fragment_streaming;
    }

    location /api/ {
        proxy_pass http://$upstream;
    }
}
```

### 2.4 Database Selection (SQLite → PostgreSQL)

**Migration Schema:**

```sql
-- messages table (Fragment's core communication)
CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id UUID NOT NULL,
    agent_id VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL,  -- 'user', 'assistant', 'tool'
    content TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_messages_thread ON messages(thread_id);
CREATE INDEX idx_messages_agent ON messages(agent_id);
CREATE INDEX idx_messages_created ON messages(created_at);

-- parts table (streaming buffer)
CREATE TABLE parts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID REFERENCES messages(id) ON DELETE CASCADE,
    part_type VARCHAR(50) NOT NULL,  -- 'text', 'tool-call', 'tool-result', 'reasoning'
    content TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_parts_message ON parts(message_id);

-- agents table (agent state)
CREATE TABLE agents (
    id VARCHAR(255) PRIMARY KEY,
    type VARCHAR(255) NOT NULL,
    template TEXT NOT NULL,
    config JSONB,
    status VARCHAR(50) DEFAULT 'active',
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- state_store table (general key-value state)
CREATE TABLE state_store (
    key VARCHAR(512) PRIMARY KEY,
    value JSONB NOT NULL,
    version INTEGER DEFAULT 1,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Connection pooling configuration
-- Recommended: pgbouncer for connection pooling
```

**Connection Pool Settings:**

```yaml
# database.pool config
database:
  pool:
    min: 10
    max: 100
    idle_timeout: 30000
    acquire_timeout: 5000
    connection_timeout: 3000
```

---

## 3. Performance Optimizations

### 3.1 Batch Tool Execution

**Problem:** Sequential tool calls create latency bottlenecks.

**Solution:** Implement batch execution with parallel processing.

```typescript
// Before: Sequential
function* processTools(tools: ToolCall[]) {
  const results = [];
  for (const tool of tools) {
    const result = yield* executeTool(tool);
    results.push(result);
  }
  return results;
}

// After: Batched parallel execution
import { Effect, Array } from "effect";

function* processToolsBatched(tools: ToolCall[]) {
  // Group tools by type for optimized execution
  const grouped = Array.groupBy(tools, t => t.type);

  // Execute each group in parallel
  const results = yield* Effect.forEach(
    Object.entries(grouped),
    ([type, batch]) =>
      Effect.matchEffect(
        executeToolBatch(type, batch),
        {
          onSuccess: results => Effect.succeed({ type, results }),
          onFailure: error => Effect.succeed({ type, error })
        }
      ),
    { concurrency: "unbounded" }
  );

  return results;
}

// Tool batch executor
async function executeToolBatch(type: string, calls: ToolCall[]) {
  switch (type) {
    case 'read-file':
      return Promise.all(calls.map(c => fs.readFile(c.path)));
    case 'http-request':
      return Promise.all(calls.map(c => fetch(c.url, c.options)));
    default:
      return Promise.all(calls.map(c => executeTool(c)));
  }
}
```

### 3.2 Parallel Agent Communication

**Architecture:**

```
┌──────────────────────────────────────────────────────────────┐
│                    Message Broker                             │
│                       (Kafka/RabbitMQ)                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │  Queue: A    │  │  Queue: B    │  │  Queue: C    │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└──────────────────────────────────────────────────────────────┘
           │                 │                 │
    ┌──────┴────┐     ┌──────┴────┐     ┌──────┴────┐
    │  Worker   │     │  Worker   │     │  Worker   │
    │  Pool 1   │     │  Pool 2   │     │  Pool 3   │
    └───────────┘     └───────────┘     └───────────┘
```

**Implementation:**

```typescript
// Parallel message processing
import { Queue, Effect, Fiber } from "effect";

class AgentMessageQueue {
  private queue: Queue.Queue<AgentMessage>;
  private workers: Fiber.RuntimeFiber<void, never>[] = [];

  constructor(workerCount: number) {
    this.queue = Queue.unbounded<AgentMessage>();

    // Start worker pool
    for (let i = 0; i < workerCount; i++) {
      const worker = Effect.suspend(() => this.processMessage()).pipe(
        Effect.forever,
        Effect.forkDaemon
      );
      this.workers.push(worker);
    }
  }

  private* processMessage() {
    while (true) {
      const message = yield* Queue.take(this.queue);
      yield* this.handleMessage(message);
    }
  }

  async send(message: AgentMessage) {
    await Queue.offer(this.queue, message);
  }
}
```

### 3.3 Caching Strategies

#### 3.3.1 Context Caching

```typescript
import { Cache, Duration, Effect } from "effect";

class ContextCache {
  private cache: Cache.Cache<string, ResolvedContext>;

  constructor() {
    this.cache = yield* Cache.make({
      capacity: 1000,
      timeToLive: Duration.minutes(5),
      lookup: (key) => this.resolveContext(key)
    });
  }

  private* resolveContext(agentId: string) {
    // Expensive context resolution
    const agent = yield* getAgent(agentId);
    const references = yield* collectReferences(agent.references);
    return { agent, references };
  }

  get(agentId: string) {
    return this.cache.get(agentId);
  }

  invalidate(agentId: string) {
    return this.cache.invalidate(agentId);
  }
}
```

#### 3.3.2 Tool Result Caching

```typescript
// LRU cache for tool results
import { LRUCache } from "lru-cache";

interface ToolCacheKey {
  toolType: string;
  params: unknown;
}

class ToolResultCache {
  private cache: LRUCache<string, unknown>;

  constructor(maxSize: number = 10000) {
    this.cache = new LRUCache({
      max: maxSize,
      ttl: 1000 * 60 * 5, // 5 minutes
      sizeCalculation: (value) => JSON.stringify(value).length
    });
  }

  private makeKey(key: ToolCacheKey): string {
    return `${key.toolType}:${JSON.stringify(key.params)}`;
  }

  get(key: ToolCacheKey): unknown | undefined {
    return this.cache.get(this.makeKey(key));
  }

  set(key: ToolCacheKey, value: unknown): void {
    this.cache.set(this.makeKey(key), value);
  }
}
```

#### 3.3.3 Response Caching

```typescript
// Redis-backed response cache for distributed deployments
import { Redis } from "ioredis";

class ResponseCache {
  constructor(private redis: Redis) {}

  async get(cacheKey: string): Promise<string | null> {
    return this.redis.get(`response:${cacheKey}`);
  }

  async set(cacheKey: string, response: string, ttlSeconds: number = 300): Promise<void> {
    await this.redis.setex(`response:${cacheKey}`, ttlSeconds, response);
  }

  async invalidate(pattern: string): Promise<void> {
    const keys = await this.redis.keys(`response:${pattern}`);
    if (keys.length > 0) {
      await this.redis.del(...keys);
    }
  }
}
```

### 3.4 Connection Pooling

```typescript
// Database connection pool
import { Pool } from "pg";

const pool = new Pool({
  host: process.env.DB_HOST,
  port: parseInt(process.env.DB_PORT || "5432"),
  database: process.env.DB_NAME,
  user: process.env.DB_USER,
  password: process.env.DB_PASSWORD,
  max: 100,                    // Max connections
  min: 10,                     // Min connections
  idleTimeoutMillis: 30000,    // Close idle after 30s
  connectionTimeoutMillis: 2000,
  acquireTimeoutMillis: 5000
});

// HTTP connection pool for external APIs
import { Agent } from "undici";

const httpAgent = new Agent({
  connections: 100,
  keepAliveTimeout: 30000,
  keepAliveMaxTimeout: 30000,
  connect: {
    rejectUnauthorized: false
  }
});

// Usage in tool execution
async function executeHttpTool(url: string, options: RequestInit) {
  const response = await fetch(url, {
    ...options,
    dispatcher: httpAgent
  });
  return response.json();
}
```

---

## 4. Memory Management

### 4.1 Context Window Optimization

**Problem:** LLM context windows are limited (typically 128K-200K tokens).

**Strategies:**

```typescript
interface ContextWindowConfig {
  maxTokens: number;
  reservedTokens: number;    // For response
  safetyMargin: number;      // Buffer (10-20%)
}

class ContextWindowManager {
  private config: ContextWindowConfig;
  private estimator: TokenEstimator;

  constructor(config: ContextWindowConfig) {
    this.config = config;
    this.estimator = new TokenEstimator();
  }

  /**
   * Build context within token limits
   */
  buildContext(messages: Message[], references: Reference[]): BuiltContext {
    const availableTokens =
      this.config.maxTokens -
      this.config.reservedTokens -
      (this.config.maxTokens * this.config.safetyMargin);

    // Priority ordering: system > recent messages > references
    const prioritized = this.prioritizeContent(messages, references);

    // Truncate to fit
    let currentTokens = 0;
    const included: ContentItem[] = [];

    for (const item of prioritized) {
      const itemTokens = this.estimator.count(item);
      if (currentTokens + itemTokens <= availableTokens) {
        included.push(item);
        currentTokens += itemTokens;
      }
    }

    return { items: included, tokenCount: currentTokens };
  }

  private prioritizeContent(messages: Message[], references: Reference[]): ContentItem[] {
    return [
      // Highest priority: System prompts
      ...messages.filter(m => m.role === 'system'),
      // High priority: Recent messages (recency bias)
      ...messages.filter(m => m.role !== 'system').slice(-50),
      // Medium priority: Direct references
      ...references.filter(r => r.priority === 'high'),
      // Low priority: Indirect references
      ...references.filter(r => r.priority === 'low')
    ];
  }
}
```

### 4.2 Message Pruning Strategies

**Strategy 1: Sliding Window**

```typescript
function pruneSlidingWindow(messages: Message[], maxMessages: number): Message[] {
  // Keep last N messages
  return messages.slice(-maxMessages);
}
```

**Strategy 2: Summary-Based Pruning**

```typescript
async function pruneWithSummary(
  messages: Message[],
  summaryWindowSize: number = 20
): Promise<Message[]> {
  if (messages.length <= summaryWindowSize) {
    return messages;
  }

  const toSummarize = messages.slice(0, -summaryWindowSize);
  const recent = messages.slice(-summaryWindowSize);

  // Generate summary of old messages
  const summary = await generateSummary(toSummarize);

  return [
    { role: 'system', content: `Previous conversation summary: ${summary}` },
    ...recent
  ];
}
```

**Strategy 3: Importance-Based Filtering**

```typescript
function pruneByImportance(messages: Message[], maxTokens: number): Message[] {
  // Score messages by importance
  const scored = messages.map(m => ({
    message: m,
    score: calculateImportance(m)
  }));

  // Sort by importance
  scored.sort((a, b) => b.score - a.score);

  // Take top messages until token limit
  const result: Message[] = [];
  let tokens = 0;

  for (const { message } of scored) {
    const msgTokens = estimateTokens(message.content);
    if (tokens + msgTokens <= maxTokens) {
      result.push(message);
      tokens += msgTokens;
    }
  }

  // Maintain chronological order
  return result.sort((a, b) =>
    messages.indexOf(a) - messages.indexOf(b)
  );
}

function calculateImportance(message: Message): number {
  let score = 1;

  // User messages are more important
  if (message.role === 'user') score += 2;

  // Messages with tool calls are important
  if (message.toolCalls?.length > 0) score += 3;

  // Messages with decisions/outcomes
  if (containsDecision(message.content)) score += 5;

  return score;
}
```

### 4.3 Streaming Memory Efficiency

**Problem:** Streaming responses can cause memory buildup.

**Solution:** Process streams incrementally without buffering.

```typescript
import { Stream, Effect } from "effect";

class StreamingMemoryManager {
  /**
   * Process streaming response without full buffering
   */
  *processStream(chunkStream: Stream.Stream<Chunk>): StreamEffect<Chunk> {
    let buffer = "";
    const flushThreshold = 1000; // chars

    yield* Stream.mapEffect(chunkStream, (chunk) => {
      buffer += chunk.content;

      if (buffer.length >= flushThreshold) {
        const toFlush = buffer;
        buffer = "";
        return Effect.succeed(toFlush);
      }

      return Effect.succeed("");
    });

    // Flush remaining buffer at end
    if (buffer.length > 0) {
      yield* Stream.fromEffect(Effect.succeed(buffer));
    }
  }

  /**
   * Backpressure-aware streaming
   */
  *streamWithBackpressure(
    source: AsyncIterable<string>,
    consumer: (chunk: string) => Effect<void>
  ) {
    const queue = yield* Queue.bounded<string>(10);

    // Producer
    yield* Effect.fork(
      Effect.forEach(source, chunk => Queue.offer(queue, chunk))
    );

    // Consumer with backpressure
    yield* Effect.forever(
      Queue.take(queue).pipe(
        Effect.flatMap(consumer),
        Effect.retry({ schedule: Schedule.exponential(100) })
      )
    );
  }
}
```

### 4.4 GC Considerations

**Node.js GC Tuning:**

```bash
# Production GC flags
NODE_OPTIONS="--max-old-space-size=4096 \
              --max-semi-space-size=128 \
              --initial-old-space-size=2048"

# For memory-intensive workloads
--trace-gc                    # Log GC events
--trace-gc-verbose           # Detailed GC logs
--always-compact             # Force compaction
```

**Memory Leak Prevention:**

```typescript
// Anti-pattern: Growing references without cleanup
class Agent {
  private messageHistory: Message[] = [];

  addMessage(msg: Message) {
    this.messageHistory.push(msg);  // Grows forever!
  }
}

// Pattern: Bounded history with cleanup
class Agent {
  private messageHistory: Message[] = [];
  private readonly MAX_HISTORY = 1000;

  addMessage(msg: Message) {
    this.messageHistory.push(msg);

    // Prune when exceeding limit
    if (this.messageHistory.length > this.MAX_HISTORY) {
      this.messageHistory = this.messageHistory.slice(-this.MAX_HISTORY / 2);
    }
  }

  cleanup() {
    this.messageHistory = [];
    // Clear any closures referencing this
  }
}
```

**WeakRef for Large Objects:**

```typescript
// Use WeakRef for large cached objects
class LargeObjectCache {
  private cache = new Map<string, WeakRef<LargeObject>>();
  private registry = new FinalizationRegistry((key: string) => {
    console.log(`LargeObject ${key} garbage collected`);
    this.cache.delete(key);
  });

  set(key: string, obj: LargeObject) {
    this.cache.set(key, new WeakRef(obj));
    this.registry.register(obj, key);
  }

  get(key: string): LargeObject | undefined {
    return this.cache.get(key)?.deref();
  }
}
```

---

## 5. High Availability Design

### 5.1 Agent State Replication

**Architecture:**

```
┌──────────────────────────────────────────────────────────────┐
│                     Primary Node                              │
│  ┌────────────┐    ┌────────────┐    ┌────────────┐         │
│  │   Agent    │    │   Agent    │    │   Agent    │         │
│  │   State 1  │    │   State 2  │    │   State N  │         │
│  └─────┬──────┘    └─────┬──────┘    └─────┬──────┘         │
│        │                 │                 │                 │
│        └─────────────────┼─────────────────┘                 │
│                          │                                   │
│              ┌───────────▼───────────┐                       │
│              │   State Replicator    │                       │
│              └───────────┬───────────┘                       │
└──────────────────────────┼──────────────────────────────────┘
                           │ (async replication)
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                     Replica Node                              │
│  ┌────────────┐    ┌────────────┐    ┌────────────┐         │
│  │   Agent    │    │   Agent    │    │   Agent    │         │
│  │   State 1  │    │   Agent    │    │   State N  │         │
│  │   (Hot)    │    │   (Standby)│    │   (Hot)    │         │
│  └────────────┘    └────────────┘    └────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

**Implementation:**

```typescript
interface AgentState {
  id: string;
  messages: Message[];
  context: ResolvedContext;
  lastUpdated: number;
  version: number;
}

class StateReplicator {
  private localState = new Map<string, AgentState>();
  private replicationLog: ReplicationEntry[] = [];

  async updateState(state: AgentState): Promise<void> {
    const entry: ReplicationEntry = {
      type: 'UPDATE',
      agentId: state.id,
      state,
      timestamp: Date.now(),
      version: state.version
    };

    // Write to local state
    this.localState.set(state.id, state);

    // Append to replication log
    this.replicationLog.push(entry);

    // Async replicate to replicas
    await this.replicateToReplicas(entry);
  }

  private async replicateToReplicas(entry: ReplicationEntry): Promise<void> {
    const replicas = await this.discoverReplicas();

    await Promise.all(
      replicas.map(replica =>
        fetch(`${replica.url}/api/state/replicate`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(entry)
        }).catch(err => {
          console.error(`Replication to ${replica.url} failed:`, err);
          // Queue for retry
          this.queueForRetry(entry, replica);
        })
      )
    );
  }
}
```

### 5.2 Failover Strategies

**Strategy 1: Leader Election with Redis**

```typescript
import { Redis } from "ioredis";

class LeaderElection {
  private redis: Redis;
  private leaderKey = 'fragment:leader';
  private leaseDuration = 30000; // 30 seconds
  private isLeader = false;

  constructor(redis: Redis, private nodeId: string) {
    this.redis = redis;
    this.startElection();
  }

  private startElection(): void {
    const tryAcquire = async () => {
      const acquired = await this.redis.set(
        this.leaderKey,
        this.nodeId,
        'NX',
        'PX',
        this.leaseDuration
      );

      if (acquired === 'OK') {
        if (!this.isLeader) {
          this.isLeader = true;
          this.onBecomeLeader();
        }
        this.renewLeadership();
      } else {
        const currentLeader = await this.redis.get(this.leaderKey);
        if (currentLeader === this.nodeId) {
          this.renewLeadership();
        } else if (this.isLeader) {
          this.isLeader = false;
          this.onLoseLeadership();
        }
      }
    };

    tryAcquire();
    setInterval(tryAcquire, this.leaseDuration / 2);
  }

  private async renewLeadership(): Promise<void> {
    await this.redis.expire(this.leaderKey, this.leaseDuration / 1000);
  }

  private onBecomeLeader(): void {
    console.log(`${this.nodeId} is now the leader`);
    // Start leader-specific tasks
  }

  private onLoseLeadership(): void {
    console.log(`${this.nodeId} lost leadership`);
    // Stop leader-specific tasks
  }

  isCurrentLeader(): boolean {
    return this.isLeader;
  }
}
```

**Strategy 2: Health-Based Failover**

```typescript
class FailoverManager {
  private healthChecks = new Map<string, HealthStatus>();
  private failoverThreshold = 3;
  private failureCounts = new Map<string, number>();

  async checkHealth(nodeId: string): Promise<boolean> {
    try {
      const response = await fetch(`http://${nodeId}/health/live`, {
        timeout: 5000
      });

      if (response.ok) {
        this.failureCounts.set(nodeId, 0);
        this.healthChecks.set(nodeId, { status: 'healthy', lastCheck: Date.now() });
        return true;
      }
    } catch {
      const count = (this.failureCounts.get(nodeId) || 0) + 1;
      this.failureCounts.set(nodeId, count);

      if (count >= this.failoverThreshold) {
        await this.triggerFailover(nodeId);
      }
    }

    return false;
  }

  private async triggerFailover(failedNodeId: string): Promise<void> {
    console.log(`Initiating failover for node ${failedNodeId}`);

    // Find healthy replica
    const healthyReplica = await this.findHealthyReplica(failedNodeId);

    if (healthyReplica) {
      // Promote replica
      await this.promoteToPrimary(healthyReplica);

      // Update routing
      await this.updateRouting(failedNodeId, healthyReplica);
    } else {
      console.error('No healthy replica found for failover');
      // Alert operators
    }
  }
}
```

### 5.3 Health Checks

**Endpoints:**

```typescript
// Express health check endpoints
app.get('/health/live', (req, res) => {
  // Liveness: Is the service running?
  res.json({ status: 'alive', timestamp: Date.now() });
});

app.get('/health/ready', async (req, res) => {
  // Readiness: Can the service handle requests?
  const checks = await Promise.all([
    checkDatabase(),
    checkRedis(),
    checkModelConnection()
  ]);

  const allHealthy = checks.every(c => c.healthy);

  if (allHealthy) {
    res.json({
      status: 'ready',
      checks: checks.reduce((acc, c) => ({ ...acc, [c.name]: 'healthy' }), {})
    });
  } else {
    res.status(503).json({
      status: 'not_ready',
      checks: checks.reduce((acc, c) => ({ ...acc, [c.name]: c.healthy ? 'healthy' : 'unhealthy' }), {})
    });
  }
});

async function checkDatabase(): Promise<HealthCheck> {
  try {
    await pool.query('SELECT 1');
    return { name: 'database', healthy: true };
  } catch {
    return { name: 'database', healthy: false };
  }
}
```

**Kubernetes Probes:**

```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 3000
  initialDelaySeconds: 30
  periodSeconds: 10
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /health/ready
    port: 3000
  initialDelaySeconds: 5
  periodSeconds: 5
  failureThreshold: 3

startupProbe:
  httpGet:
    path: /health/live
    port: 3000
  failureThreshold: 30
  periodSeconds: 10
```

### 5.4 Recovery Procedures

**Runbook: Agent Recovery**

```markdown
# Agent Recovery Runbook

## Symptoms
- Agent not responding to send/query
- Stale state in database
- Missing messages

## Diagnosis

1. Check agent status
   ```bash
   curl http://agent-node:3000/api/agents/{id}/status
   ```

2. Check message queue
   ```bash
   redis-cli LLEN fragment:messages:{agentId}
   ```

3. Check database state
   ```bash
   psql -c "SELECT * FROM agents WHERE id = '{agentId}'"
   ```

## Recovery Steps

### Step 1: Isolate the Agent
```bash
# Mark agent as unavailable
curl -X POST http://loadbalancer/api/agents/{id}/drain
```

### Step 2: Restore State from Backup
```bash
# Restore from last known good state
pg_restore -d fragment --table=agents --where="id='{agentId}'" /backups/agents_latest.dump
```

### Step 3: Replay Messages
```bash
# Replay queued messages
node scripts/replay-messages.js --agent {agentId} --from {timestamp}
```

### Step 4: Verify Recovery
```bash
# Send test message
curl -X POST http://agent-node:3000/api/agents/{id}/send \
  -d '{"content": "Recovery test"}'
```

## Escalation
If recovery fails after 3 attempts, escalate to on-call engineer.
```

**Runbook: Database Failover**

```markdown
# Database Failover Runbook

## Primary Failure Detection

1. Check primary connectivity
   ```bash
   psql -h primary-host -U fragment -c "SELECT 1"
   ```

2. Check replication lag
   ```bash
   psql -h replica-host -U fragment -c "SELECT pg_last_wal_receive_lsn() - pg_last_wal_replay_lsn()"
   ```

## Failover Procedure

### Step 1: Promote Replica
```bash
# Promote standby to primary
psql -h replica-host -U fragment -c "SELECT pg_promote()"
```

### Step 2: Update Connection Strings
```bash
# Update DNS or connection manager
aws route53 change-resource-record-sets \
  --hosted-zone-id ZONE_ID \
  --change-batch file://failover-dns.json
```

### Step 3: Verify Application Connectivity
```bash
# Test from application nodes
for node in $(kubectl get pods -l app=fragment -o jsonpath='{.items[*].status.podIP}'); do
  kubectl exec -n fragment $node -- psql -h new-primary -c "SELECT 1"
done
```

## Post-Failover

1. Set up new replica from promoted primary
2. Update monitoring alerts
3. Document incident
```

---

## 6. Serving Infrastructure

### 6.1 HTTP API Design

**RESTful API Structure:**

```typescript
// API Routes
const router = Router();

// Agent management
router.post('/agents', createAgent);           // Spawn new agent
router.get('/agents', listAgents);             // List all agents
router.get('/agents/:id', getAgent);           // Get agent details
router.delete('/agents/:id', destroyAgent);    // Destroy agent
router.post('/agents/:id/drain', drainAgent);  // Graceful shutdown

// Communication
router.post('/agents/:id/send', sendMessage);      // Send message
router.post('/agents/:id/query', sendQuery);       // Structured query
router.get('/agents/:id/messages', getMessages);   // Message history

// State management
router.get('/agents/:id/state', getAgentState);    // Get state
router.put('/agents/:id/state', updateAgentState); // Update state

// Health and metrics
router.get('/health/live', livenessHandler);       // Liveness probe
router.get('/health/ready', readinessHandler);     // Readiness probe
router.get('/metrics', metricsHandler);            // Prometheus metrics
```

**Request/Response Types:**

```typescript
// Send message request
interface SendRequest {
  content: string;
  context?: Record<string, unknown>;
  streaming?: boolean;
}

// Send message response
interface SendResponse {
  messageId: string;
  response: string;
  latency: number;
  tokensUsed: number;
}

// Query request
interface QueryRequest<T> {
  query: string;
  responseSchema: z.ZodType<T>;
  timeout?: number;
}

// Query response
interface QueryResponse<T> {
  data: T;
  confidence?: number;
  sources?: string[];
}
```

**Error Handling:**

```typescript
// Error response format
interface ErrorResponse {
  error: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
    traceId?: string;
  };
}

// Error codes
enum ErrorCode {
  AGENT_NOT_FOUND = 'AGENT_001',
  AGENT_UNAVAILABLE = 'AGENT_002',
  CONTEXT_EXCEEDED = 'CONTEXT_001',
  RATE_LIMITED = 'RATE_001',
  TOOL_EXECUTION_FAILED = 'TOOL_001',
  INVALID_REQUEST = 'REQ_001',
  INTERNAL_ERROR = 'INT_001'
}
```

### 6.2 WebSocket for Streaming

**Server Implementation:**

```typescript
import { WebSocketServer, WebSocket } from 'ws';

class StreamingService {
  private wss: WebSocketServer;
  private clients = new Map<string, WebSocket>();
  private subscriptions = new Map<string, Set<string>>();

  constructor(port: number) {
    this.wss = new WebSocketServer({ port });
    this.setupHandlers();
  }

  private setupHandlers(): void {
    this.wss.on('connection', (ws, req) => {
      const clientId = this.generateClientId();
      this.clients.set(clientId, ws);

      ws.on('message', (data) => {
        const message = JSON.parse(data.toString());
        this.handleMessage(clientId, message);
      });

      ws.on('close', () => {
        this.clients.delete(clientId);
      });

      // Send connection confirmation
      ws.send(JSON.stringify({
        type: 'connected',
        clientId
      }));
    });
  }

  private handleMessage(clientId: string, message: ClientMessage): void {
    const ws = this.clients.get(clientId);
    if (!ws) return;

    switch (message.type) {
      case 'subscribe':
        this.subscribe(clientId, message.agentId);
        break;
      case 'unsubscribe':
        this.unsubscribe(clientId, message.agentId);
        break;
      case 'send':
        this.forwardSend(clientId, message);
        break;
    }
  }

  private subscribe(clientId: string, agentId: string): void {
    if (!this.subscriptions.has(agentId)) {
      this.subscriptions.set(agentId, new Set());
    }
    this.subscriptions.get(agentId)!.add(clientId);
  }

  // Stream agent response to subscribers
  streamToSubscribers(agentId: string, chunk: string): void {
    const subscribers = this.subscriptions.get(agentId);
    if (!subscribers) return;

    const message = JSON.stringify({
      type: 'stream_chunk',
      agentId,
      chunk,
      timestamp: Date.now()
    });

    for (const clientId of subscribers) {
      const ws = this.clients.get(clientId);
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(message);
      }
    }
  }
}
```

**Client Usage:**

```typescript
class FragmentClient {
  private ws: WebSocket;
  private messageHandlers = new Map<string, (msg: any) => void>();

  constructor(url: string) {
    this.ws = new WebSocket(url);
    this.setupHandlers();
  }

  private setupHandlers(): void {
    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);

      switch (message.type) {
        case 'stream_chunk':
          this.messageHandlers.get(message.agentId)?.(message);
          break;
        case 'response':
          this.messageHandlers.get(message.messageId)?.(message);
          break;
      }
    };
  }

  async send(agentId: string, content: string): Promise<string> {
    return new Promise((resolve) => {
      const messageId = crypto.randomUUID();

      this.messageHandlers.set(messageId, (msg) => {
        resolve(msg.response);
        this.messageHandlers.delete(messageId);
      });

      this.ws.send(JSON.stringify({
        type: 'send',
        messageId,
        agentId,
        content
      }));
    });
  }

  subscribe(agentId: string, onChunk: (chunk: string) => void): () => void {
    this.messageHandlers.set(agentId, (msg) => {
      onChunk(msg.chunk);
    });

    this.ws.send(JSON.stringify({
      type: 'subscribe',
      agentId
    }));

    return () => {
      this.messageHandlers.delete(agentId);
      this.ws.send(JSON.stringify({
        type: 'unsubscribe',
        agentId
      }));
    };
  }
}
```

### 6.3 Rate Limiting

**Token Bucket Implementation:**

```typescript
class TokenBucket {
  private tokens: number;
  private maxTokens: number;
  private refillRate: number; // tokens per second
  private lastRefill: number;

  constructor(maxTokens: number, refillRate: number) {
    this.maxTokens = maxTokens;
    this.refillRate = refillRate;
    this.tokens = maxTokens;
    this.lastRefill = Date.now();
  }

  private refill(): void {
    const now = Date.now();
    const elapsed = (now - this.lastRefill) / 1000;
    this.tokens = Math.min(
      this.maxTokens,
      this.tokens + elapsed * this.refillRate
    );
    this.lastRefill = now;
  }

  tryAcquire(tokens: number = 1): boolean {
    this.refill();

    if (this.tokens >= tokens) {
      this.tokens -= tokens;
      return true;
    }

    return false;
  }

  getWaitTime(tokens: number = 1): number {
    this.refill();

    if (this.tokens >= tokens) {
      return 0;
    }

    return (tokens - this.tokens) / this.refillRate * 1000; // ms
  }
}

// Rate limiting middleware
const rateLimiters = new Map<string, TokenBucket>();

function rateLimitMiddleware(
  requestsPerSecond: number,
  burstSize: number
) {
  return (req: Request, res: Response, next: NextFunction) => {
    const clientId = req.ip || req.headers['x-api-key'] as string;

    if (!rateLimiters.has(clientId)) {
      rateLimiters.set(clientId, new TokenBucket(burstSize, requestsPerSecond));
    }

    const bucket = rateLimiters.get(clientId)!;

    if (bucket.tryAcquire()) {
      next();
    } else {
      const waitTime = bucket.getWaitTime();
      res.set('Retry-After', Math.ceil(waitTime / 1000).toString());
      res.status(429).json({
        error: {
          code: 'RATE_LIMITED',
          message: 'Too many requests',
          retryAfter: Math.ceil(waitTime / 1000)
        }
      });
    }
  };
}
```

**Redis-Based Distributed Rate Limiting:**

```typescript
import { Redis } from 'ioredis';

class DistributedRateLimiter {
  constructor(private redis: Redis) {}

  async isAllowed(key: string, limit: number, window: number): Promise<{
    allowed: boolean;
    remaining: number;
    resetAt: number;
  }> {
    const now = Date.now();
    const windowStart = now - window;

    const multi = this.redis.multi();
    multi.zremrangebyscore(key, 0, windowStart);
    multi.zadd(key, now, `${now}-${crypto.randomUUID()}`);
    multi.zcard(key);
    multi.expire(key, Math.ceil(window / 1000));

    const results = await multi.exec();
    const count = results?.[2]?.[1] as number || 0;

    const allowed = count <= limit;
    const remaining = Math.max(0, limit - count);
    const resetAt = now + window;

    return { allowed, remaining, resetAt };
  }
}
```

### 6.4 Authentication/Authorization

**JWT Authentication:**

```typescript
import jwt from 'jsonwebtoken';
import { Request, Response, NextFunction } from 'express';

interface AuthPayload {
  userId: string;
  tenantId: string;
  permissions: string[];
}

declare global {
  namespace Express {
    interface Request {
      auth?: AuthPayload;
    }
  }
}

// JWT middleware
function authenticateJWT(secret: string) {
  return (req: Request, res: Response, next: NextFunction) => {
    const authHeader = req.headers.authorization;

    if (!authHeader?.startsWith('Bearer ')) {
      return res.status(401).json({ error: { code: 'UNAUTHORIZED', message: 'Missing token' } });
    }

    const token = authHeader.substring(7);

    try {
      const payload = jwt.verify(token, secret) as AuthPayload;
      req.auth = payload;
      next();
    } catch {
      return res.status(401).json({ error: { code: 'UNAUTHORIZED', message: 'Invalid token' } });
    }
  };
}

// Authorization middleware
function requirePermission(permission: string) {
  return (req: Request, res: Response, next: NextFunction) => {
    if (!req.auth?.permissions.includes(permission)) {
      return res.status(403).json({
        error: { code: 'FORBIDDEN', message: `Missing permission: ${permission}` }
      });
    }
    next();
  };
}

// Usage
app.post('/api/agents',
  authenticateJWT(process.env.JWT_SECRET!),
  requirePermission('agents:create'),
  createAgent
);
```

**RBAC Model:**

```typescript
enum Role {
  ADMIN = 'admin',
  DEVELOPER = 'developer',
  VIEWER = 'viewer'
}

const permissions: Record<Role, string[]> = {
  [Role.ADMIN]: ['*'],
  [Role.DEVELOPER]: [
    'agents:create',
    'agents:read',
    'agents:update',
    'agents:send',
    'agents:query'
  ],
  [Role.VIEWER]: [
    'agents:read',
    'messages:read'
  ]
};
```

---

## 7. Monitoring and Observability

### 7.1 Metrics Collection (Prometheus)

**Key Metrics:**

```typescript
import { Counter, Histogram, Gauge, register } from 'prom-client';

// Agent metrics
const agentsActive = new Gauge({
  name: 'fragment_agents_active',
  help: 'Number of active agents',
  labelNames: ['status']
});

const agentMessagesProcessed = new Counter({
  name: 'fragment_agent_messages_total',
  help: 'Total messages processed by agents',
  labelNames: ['agent_id', 'message_type']
});

// Tool execution metrics
const toolExecutions = new Counter({
  name: 'fragment_tool_executions_total',
  help: 'Total tool executions',
  labelNames: ['tool_type', 'status']
});

const toolExecutionDuration = new Histogram({
  name: 'fragment_tool_execution_duration_seconds',
  help: 'Tool execution duration',
  labelNames: ['tool_type'],
  buckets: [0.01, 0.05, 0.1, 0.5, 1, 5, 10]
});

// Context metrics
const contextTokensUsed = new Histogram({
  name: 'fragment_context_tokens_used',
  help: 'Tokens used in context',
  labelNames: ['agent_id'],
  buckets: [100, 500, 1000, 5000, 10000, 50000, 100000]
});

// Memory metrics
const memoryUsage = new Gauge({
  name: 'fragment_memory_usage_bytes',
  help: 'Memory usage in bytes',
  labelNames: ['type']
});

// Metrics endpoint
app.get('/metrics', async (req, res) => {
  // Update dynamic metrics
  memoryUsage.set({ type: 'heap' }, process.memoryUsage().heapUsed);
  memoryUsage.set({ type: 'rss' }, process.memoryUsage().rss);

  res.set('Content-Type', register.contentType);
  res.end(await register.metrics());
});
```

**Prometheus Configuration:**

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'fragment'
    static_configs:
      - targets: ['fragment-service:80']
    metrics_path: '/metrics'

  - job_name: 'node'
    static_configs:
      - targets: ['fragment-1:9100', 'fragment-2:9100']
```

### 7.2 Distributed Tracing

**OpenTelemetry Integration:**

```typescript
import { NodeTracerProvider } from '@opentelemetry/sdk-trace-node';
import { JaegerExporter } from '@opentelemetry/exporter-jaeger';
import { registerInstrumentations } from '@opentelemetry/instrumentation';
import { HttpInstrumentation } from '@opentelemetry/instrumentation-http';

// Configure tracing
const provider = new NodeTracerProvider({
  resource: new Resource({
    'service.name': 'fragment-agents',
    'service.version': process.env.VERSION || 'unknown'
  })
});

provider.addSpanProcessor(new BatchSpanProcessor(
  new JaegerExporter({
    endpoint: process.env.JAEGER_ENDPOINT || 'http://localhost:14268/api/traces'
  })
));

provider.register();

registerInstrumentations({
  instrumentations: [
    new HttpInstrumentation({
      requestHook: (span, request) => {
        span.setAttribute('http.request_id', request.headers['x-request-id'] || 'unknown');
      }
    })
  ]
});

// Manual tracing in agent operations
import { trace, context, SpanStatusCode } from '@opentelemetry/api';

const tracer = trace.getTracer('fragment');

async function sendAgentMessage(agentId: string, content: string) {
  return tracer.startActiveSpan('agent.send', async (span) => {
    span.setAttribute('agent.id', agentId);
    span.setAttribute('message.content_length', content.length);

    try {
      const result = await executeSend(agentId, content);
      span.setAttribute('message.response_length', result.response.length);
      span.setStatus({ code: SpanStatusCode.OK });
      return result;
    } catch (error) {
      span.setStatus({ code: SpanStatusCode.ERROR, message: String(error) });
      span.recordException(error);
      throw error;
    } finally {
      span.end();
    }
  });
}
```

**Trace Propagation:**

```typescript
// Extract trace context from incoming requests
import { propagation, trace } from '@opentelemetry/api';

app.use((req, res, next) => {
  const extractedContext = propagation.extract(
    context.active(),
    req.headers
  );

  context.with(extractedContext, () => {
    next();
  });
});

// Inject trace context into outgoing requests
async function callExternalApi(url: string, options: RequestInit) {
  const span = tracer.startSpan('external.api_call');

  const headers = new Headers(options.headers);
  propagation.inject(context.active(), headers);

  try {
    const response = await fetch(url, { ...options, headers });
    span.setStatus({ code: SpanStatusCode.OK });
    return response;
  } catch (error) {
    span.setStatus({ code: SpanStatusCode.ERROR });
    throw error;
  } finally {
    span.end();
  }
}
```

### 7.3 Logging Structure

**Structured Logging:**

```typescript
import pino from 'pino';

const logger = pino({
  level: process.env.LOG_LEVEL || 'info',
  formatters: {
    level: (label) => ({ level: label })
  },
  timestamp: () => `,"timestamp":"${new Date().toISOString()}"`
});

// Log context with every message
logger.info({
  event: 'agent.message_received',
  agent_id: 'assistant-1',
  message_id: 'msg-123',
  content_length: 256,
  user_id: 'user-456',
  trace_id: 'trace-789'
}, 'Message received from user');

// Tool execution logging
logger.info({
  event: 'tool.execution',
  tool_type: 'read-file',
  tool_id: 'read-1',
  duration_ms: 45,
  status: 'success',
  trace_id: 'trace-789'
}, 'Tool executed successfully');

// Error logging
logger.error({
  event: 'agent.error',
  agent_id: 'assistant-1',
  error: {
    name: error.name,
    message: error.message,
    stack: error.stack
  },
  trace_id: 'trace-789'
}, 'Agent execution failed');
```

**Log Aggregation (Loki):**

```yaml
# loki-config.yaml
auth_enabled: false

server:
  http_listen_port: 3100

common:
  path_prefix: /loki
  storage:
    filesystem:
      chunks_directory: /loki/chunks
      rules_directory: /loki/rules

schema_config:
  configs:
    - from: 2020-10-24
      store: boltdb-shipper
      object_store: filesystem
      schema: v11
      index:
        prefix: index_
        period: 24h
```

### 7.4 Alerting Rules

**Prometheus Alert Rules:**

```yaml
# alerting-rules.yml
groups:
  - name: fragment-alerts
    rules:
      # High error rate
      - alert: FragmentHighErrorRate
        expr: rate(fragment_tool_executions_total{status="error"}[5m]) > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High tool error rate"
          description: "Error rate is {{ $value }} errors/sec"

      # Agent unavailable
      - alert: FragmentAgentUnavailable
        expr: fragment_agents_active{status="ready"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "No agents available"
          description: "All agents are unavailable"

      # High memory usage
      - alert: FragmentHighMemory
        expr: fragment_memory_usage_bytes{type="rss"} > 3758096384  # 3.5GB
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage"
          description: "Memory usage is {{ $value | humanize1024 }}B"

      # Context window exceeded
      - alert: FragmentContextWindowExceeded
        expr: histogram_quantile(0.99, rate(fragment_context_tokens_used_bucket[5m])) > 100000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Context window near limit"
          description: "99th percentile context size is {{ $value }} tokens"

      # High latency
      - alert: FragmentHighLatency
        expr: histogram_quantile(0.95, rate(fragment_tool_execution_duration_seconds_bucket[5m])) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High tool execution latency"
          description: "95th percentile latency is {{ $value }}s"
```

**Alertmanager Configuration:**

```yaml
# alertmanager.yml
global:
  smtp_smarthost: 'smtp.example.com:587'
  smtp_from: 'alertmanager@example.com'

route:
  group_by: ['alertname', 'severity']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h
  receiver: 'slack-notifications'

  routes:
    - match:
        severity: critical
      receiver: 'pagerduty-critical'
    - match:
        severity: warning
      receiver: 'slack-notifications'

receivers:
  - name: 'slack-notifications'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/xxx/yyy/zzz'
        channel: '#alerts'
        send_resolved: true

  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: 'your-pagerduty-service-key'
```

---

## 8. Security Hardening

### 8.1 Tool Execution Sandboxing

**Docker-Based Sandboxing:**

```typescript
import { Docker } from 'dockerode';

const docker = new Docker();

class SandboxedToolExecutor {
  async execute(toolType: string, params: unknown): Promise<unknown> {
    const container = await docker.createContainer({
      Image: 'fragment-tool-sandbox:latest',
      Cmd: [toolType, JSON.stringify(params)],
      HostConfig: {
        Memory: 512 * 1024 * 1024,        // 512MB limit
        NanoCpus: 1000000000,              // 1 CPU
        NetworkMode: 'none',               // No network access
        ReadonlyRootfs: true,              // Read-only filesystem
        CapDrop: ['ALL'],                  // Drop all capabilities
        SecurityOpt: ['no-new-privileges'] // Prevent privilege escalation
      }
    });

    await container.start();
    const result = await container.wait();

    if (result.StatusCode !== 0) {
      const logs = await container.logs({ stderr: true, stdout: true });
      throw new Error(`Tool execution failed: ${logs.toString()}`);
    }

    await container.remove();

    return JSON.parse(result);
  }
}
```

**Firecracker MicroVM Sandboxing:**

```typescript
// For stronger isolation with Firecracker
import { FirecrackerClient } from '@aws-sdk/client-firecracker';

class MicroVMSandbox {
  async execute(toolType: string, params: unknown): Promise<unknown> {
    // Start micro VM
    const vm = await this.startMicroVM({
      kernel: '/path/to/kernel',
      rootfs: '/path/to/rootfs',
      memory: 256,
      vcpu: 1
    });

    try {
      // Execute tool in isolated environment
      const result = await vm.exec([toolType, JSON.stringify(params)]);
      return JSON.parse(result.stdout);
    } finally {
      await vm.terminate();
    }
  }
}
```

### 8.2 Input Validation

**Schema Validation:**

```typescript
import { z } from 'zod';

// Message input validation
const MessageSchema = z.object({
  content: z.string().min(1).max(100000),
  context: z.record(z.unknown()).optional(),
  agent_id: z.string().regex(/^[a-zA-Z0-9-_]+$/)
});

// Tool input validation
const ToolInputSchemas: Record<string, z.ZodType> = {
  'read-file': z.object({
    path: z.string().regex(/^[a-zA-Z0-9/_.-]+$/).max(500)
  }),
  'write-file': z.object({
    path: z.string().regex(/^[a-zA-Z0-9/_.-]+$/).max(500),
    content: z.string().max(1000000)
  }),
  'bash': z.object({
    command: z.string().max(10000),
    timeout: z.number().max(30000).optional()
  }),
  'http-request': z.object({
    url: z.string().url(),
    method: z.enum(['GET', 'POST', 'PUT', 'DELETE']),
    headers: z.record(z.string()).optional(),
    body: z.string().optional()
  })
};

// Validation middleware
function validateInput(schema: z.ZodType) {
  return (req: Request, res: Response, next: NextFunction) => {
    const result = schema.safeParse(req.body);

    if (!result.success) {
      return res.status(400).json({
        error: {
          code: 'INVALID_REQUEST',
          message: 'Invalid input',
          details: result.error.flatten()
        }
      });
    }

    req.body = result.data;
    next();
  };
}
```

**Command Injection Prevention:**

```typescript
// BAD: Vulnerable to command injection
async function executeBash(command: string) {
  return execSync(command);  // DON'T DO THIS
}

// GOOD: Use argument arrays
import { execFile } from 'child_process';

async function executeBashSafe(command: string, args: string[]) {
  return new Promise((resolve, reject) => {
    execFile(command, args, { timeout: 30000 }, (error, stdout, stderr) => {
      if (error) reject(error);
      else resolve({ stdout, stderr });
    });
  });
}

// BEST: Use restricted shell with allowlist
const ALLOWED_COMMANDS = ['ls', 'cat', 'grep', 'find', 'git'];

async function executeBashRestricted(input: string) {
  const parts = input.split(' ');
  const command = parts[0];

  if (!ALLOWED_COMMANDS.includes(command)) {
    throw new Error(`Command not allowed: ${command}`);
  }

  // Validate arguments (no pipes, redirects, etc.)
  const dangerousChars = ['|', '>', '<', '&', ';', '$', '`'];
  for (const char of dangerousChars) {
    if (input.includes(char)) {
      throw new Error('Dangerous characters not allowed');
    }
  }

  return executeBashSafe(command, parts.slice(1));
}
```

### 8.3 Secret Management

**HashiCorp Vault Integration:**

```typescript
import { Vault } from 'node-vault';

class SecretManager {
  private vault: Vault;

  constructor() {
    this.vault = Vault({
      endpoint: process.env.VAULT_ENDPOINT,
      token: process.env.VAULT_TOKEN
    });
  }

  async getSecret(path: string): Promise<string> {
    const result = await this.vault.read(`secret/data/${path}`);
    return result.data.data.value;
  }

  async rotateSecret(path: string): Promise<void> {
    const newSecret = crypto.randomBytes(32).toString('hex');
    await this.vault.write(`secret/data/${path}`, {
      data: { value: newSecret }
    });
  }
}

// Usage in agent
const secretManager = new SecretManager();

async function setupAgent() {
  const apiKey = await secretManager.getSecret('api-keys/anthropic');
  // Use API key...
}
```

**AWS Secrets Manager:**

```typescript
import { SecretsManagerClient, GetSecretValueCommand } from '@aws-sdk/client-secrets-manager';

class AWSSecretManager {
  private client: SecretsManagerClient;

  constructor() {
    this.client = new SecretsManagerClient({});
  }

  async getSecret(secretName: string): Promise<Record<string, string>> {
    const command = new GetSecretValueCommand({ SecretId: secretName });
    const response = await this.client.send(command);

    if (response.SecretString) {
      return JSON.parse(response.SecretString);
    }

    throw new Error('Secret not found');
  }
}
```

### 8.4 Audit Logging

**Audit Log Schema:**

```typescript
interface AuditLog {
  timestamp: string;
  event_type: string;
  actor: {
    type: 'user' | 'agent' | 'system';
    id: string;
  };
  action: string;
  resource: {
    type: string;
    id: string;
  };
  outcome: 'success' | 'failure';
  details?: Record<string, unknown>;
  trace_id?: string;
  ip_address?: string;
}

// Audit logging middleware
function auditLog() {
  return (req: Request, res: Response, next: NextFunction) => {
    const startTime = Date.now();

    res.on('finish', () => {
      const auditEntry: AuditLog = {
        timestamp: new Date().toISOString(),
        event_type: 'http_request',
        actor: {
          type: 'user',
          id: req.auth?.userId || 'anonymous'
        },
        action: `${req.method} ${req.path}`,
        resource: {
          type: 'api',
          id: req.params.id || 'collection'
        },
        outcome: res.statusCode < 400 ? 'success' : 'failure',
        details: {
          status_code: res.statusCode,
          duration_ms: Date.now() - startTime
        },
        trace_id: req.headers['x-trace-id'] as string,
        ip_address: req.ip
      };

      // Write to audit log (async, non-blocking)
      writeAuditLog(auditEntry).catch(console.error);
    });

    next();
  };
}

// Write audit log to separate storage
async function writeAuditLog(entry: AuditLog): Promise<void> {
  // Write to dedicated audit log store
  await auditLogger.info(entry);

  // Also store in database for querying
  await pool.query(
    'INSERT INTO audit_logs (timestamp, event_type, actor, action, resource, outcome, details) VALUES ($1, $2, $3, $4, $5, $6, $7)',
    [
      entry.timestamp,
      entry.event_type,
      JSON.stringify(entry.actor),
      entry.action,
      JSON.stringify(entry.resource),
      entry.outcome,
      JSON.stringify(entry.details)
    ]
  );
}
```

**Audit Log Retention:**

```sql
-- Create partitioned audit log table
CREATE TABLE audit_logs (
  id UUID DEFAULT gen_random_uuid(),
  timestamp TIMESTAMPTZ NOT NULL,
  event_type VARCHAR(100) NOT NULL,
  actor JSONB NOT NULL,
  action VARCHAR(500) NOT NULL,
  resource JSONB NOT NULL,
  outcome VARCHAR(20) NOT NULL,
  details JSONB,
  trace_id VARCHAR(100),
  ip_address INET
) PARTITION BY RANGE (timestamp);

-- Create monthly partitions
CREATE TABLE audit_logs_2026_03 PARTITION OF audit_logs
  FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');

-- Retention policy: Delete logs older than 1 year
CREATE OR REPLACE FUNCTION drop_old_audit_partitions()
RETURNS void AS $$
BEGIN
  EXECUTE (
    SELECT 'DROP TABLE IF EXISTS ' || string_agg(table_name, ', ')
    FROM information_schema.tables
    WHERE table_name LIKE 'audit_logs_%'
      AND table_name < 'audit_logs_' || to_char(now() - interval '1 year', 'YYYY_MM')
  );
END;
$$ LANGUAGE plpgsql;
```

---

## 9. Multi-tenant Deployments

### 9.1 Tenant Isolation

**Database-Level Isolation:**

```sql
-- Schema-per-tenant approach
CREATE SCHEMA tenant_acme;
CREATE SCHEMA tenant_globex;

-- Tables in each schema
CREATE TABLE tenant_acme.agents (...);
CREATE TABLE tenant_acme.messages (...);
CREATE TABLE tenant_acme.state_store (...);

-- Row-Level Security (RLS) approach
ALTER TABLE agents ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON agents
  USING (tenant_id = current_setting('app.current_tenant')::uuid);

-- Set tenant context per connection
SET app.current_tenant = 'acme-uuid';
```

**Application-Level Isolation:**

```typescript
class TenantContext {
  private static storage = new AsyncLocalStorage<TenantContext>();

  constructor(
    public tenantId: string,
    public permissions: string[]
  ) {}

  static get(): TenantContext | undefined {
    return this.storage.getStore();
  }

  static run<T>(tenant: TenantContext, fn: () => T): T {
    return this.storage.run(tenant, fn);
  }
}

// Middleware to set tenant context
async function tenantMiddleware(req: Request, res: Response, next: NextFunction) {
  const tenantId = req.headers['x-tenant-id'] as string;

  if (!tenantId) {
    return res.status(400).json({ error: { message: 'Missing tenant ID' } });
  }

  const tenant = await getTenant(tenantId);

  if (!tenant) {
    return res.status(404).json({ error: { message: 'Tenant not found' } });
  }

  TenantContext.run(new TenantContext(tenant.id, tenant.permissions), () => {
    next();
  });
}

// Repository with automatic tenant filtering
class AgentRepository {
  async findAll(): Promise<Agent[]> {
    const tenant = TenantContext.get();
    if (!tenant) throw new Error('No tenant context');

    return pool.query(
      'SELECT * FROM agents WHERE tenant_id = $1',
      [tenant.tenantId]
    );
  }
}
```

### 9.2 Resource Quotas

**Quota Configuration:**

```typescript
interface TenantQuotas {
  maxAgents: number;
  maxMessagesPerDay: number;
  maxContextTokens: number;
  maxToolExecutionsPerHour: number;
  maxStorageBytes: number;
  allowedModels: string[];
}

const DEFAULT_QUOTAS: TenantQuotas = {
  maxAgents: 50,
  maxMessagesPerDay: 10000,
  maxContextTokens: 128000,
  maxToolExecutionsPerHour: 1000,
  maxStorageBytes: 1073741824, // 1GB
  allowedModels: ['claude-3-sonnet', 'claude-3-haiku']
};

const PREMIUM_QUOTAS: TenantQuotas = {
  maxAgents: 500,
  maxMessagesPerDay: 100000,
  maxContextTokens: 200000,
  maxToolExecutionsPerHour: 10000,
  maxStorageBytes: 10737418240, // 10GB
  allowedModels: ['claude-3-opus', 'claude-3-sonnet', 'claude-3-haiku']
};
```

**Quota Enforcement:**

```typescript
class QuotaEnforcer {
  constructor(private redis: Redis) {}

  async checkQuota(tenantId: string, quotaType: keyof TenantQuotas): Promise<{
    allowed: boolean;
    current: number;
    limit: number;
  }> {
    const tenant = await getTenant(tenantId);
    const limit = tenant.quotas[quotaType];

    const key = `quota:${tenantId}:${quotaType}`;
    const current = await this.redis.get(key);

    return {
      allowed: current === null || parseInt(current) < limit,
      current: current ? parseInt(current) : 0,
      limit
    };
  }

  async incrementQuota(tenantId: string, quotaType: keyof TenantQuotas, amount: number = 1): Promise<void> {
    const key = `quota:${tenantId}:${quotaType}`;

    // Different reset periods for different quotas
    const resetSeconds = this.getResetPeriod(quotaType);

    await this.redis.incrby(key, amount);
    await this.redis.expire(key, resetSeconds);
  }

  private getResetPeriod(quotaType: keyof TenantQuotas): number {
    switch (quotaType) {
      case 'maxMessagesPerDay':
        return 86400;
      case 'maxToolExecutionsPerHour':
        return 3600;
      default:
        return 86400;
    }
  }
}

// Usage in middleware
async function quotaMiddleware(req: Request, res: Response, next: NextFunction) {
  const tenant = TenantContext.get();
  if (!tenant) return next();

  const enforcer = new QuotaEnforcer(redis);

  const quota = await enforcer.checkQuota(tenant.tenantId, 'maxMessagesPerDay');

  if (!quota.allowed) {
    return res.status(429).json({
      error: {
        code: 'QUOTA_EXCEEDED',
        message: 'Daily message quota exceeded',
        details: { current: quota.current, limit: quota.limit }
      }
    });
  }

  await enforcer.incrementQuota(tenant.tenantId, 'maxMessagesPerDay');
  next();
}
```

### 9.3 Billing Integration

**Stripe Integration:**

```typescript
import Stripe from 'stripe';

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY!);

class BillingService {
  async createCustomer(tenantId: string, email: string): Promise<string> {
    const customer = await stripe.customers.create({
      email,
      metadata: { tenant_id: tenantId }
    });
    return customer.id;
  }

  async recordUsage(tenantId: string, metric: string, quantity: number): Promise<void> {
    const customer = await this.getCustomer(tenantId);

    await stripe.subscriptionItems.createUsageRecord(
      customer.subscriptionItemId,
      {
        quantity,
        action: 'increment',
        timestamp: Math.floor(Date.now() / 1000)
      }
    );
  }

  async getInvoice(tenantId: string): Promise<Stripe.Invoice> {
    const customer = await this.getCustomer(tenantId);
    return stripe.invoices.retrieve(customer.latestInvoice);
  }
}

// Usage tracking
async function trackUsage(tenantId: string, eventType: string, quantity: number) {
  const billing = new BillingService();

  await billing.recordUsage(tenantId, eventType, quantity);

  // Also track internally for quota enforcement
  await trackInternalUsage(tenantId, eventType, quantity);
}
```

### 9.4 Data Separation

**Physical Separation (Multi-Database):**

```typescript
class TenantDatabaseManager {
  private connections = new Map<string, Pool>();

  async getConnection(tenantId: string): Promise<Pool> {
    if (!this.connections.has(tenantId)) {
      // Create dedicated connection for tenant
      const pool = new Pool({
        host: process.env.DB_HOST,
        database: `fragment_${tenantId}`,
        user: process.env.DB_USER,
        password: process.env.DB_PASSWORD
      });

      this.connections.set(tenantId, pool);
    }

    return this.connections.get(tenantId)!;
  }

  async provisionTenant(tenantId: string): Promise<void> {
    // Create database
    await pool.query(`CREATE DATABASE fragment_${tenantId}`);

    // Run migrations
    await runMigrations(`fragment_${tenantId}`);
  }

  async deprovisionTenant(tenantId: string): Promise<void> {
    // Drop database
    await pool.query(`DROP DATABASE fragment_${tenantId}`);

    // Close connection
    const conn = this.connections.get(tenantId);
    if (conn) {
      await conn.end();
      this.connections.delete(tenantId);
    }
  }
}
```

**Backup per Tenant:**

```bash
#!/bin/bash
# backup-tenant.sh

TENANT_ID=$1
BACKUP_DIR="/backups/tenants/${TENANT_ID}"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p "$BACKUP_DIR"

# Dump tenant database
pg_dump -h $DB_HOST -U $DB_USER "fragment_${TENANT_ID}" > "${BACKUP_DIR}/db_${DATE}.sql"

# Export tenant files
tar -czf "${BACKUP_DIR}/files_${DATE}.tar.gz" "/data/tenants/${TENANT_ID}"

# Upload to S3
aws s3 cp "${BACKUP_DIR}" "s3://backups/tenants/${TENANT_ID}/" --recursive

# Retain only last 30 days of backups
find "${BACKUP_DIR}" -mtime +30 -delete
```

---

## 10. Scaling Strategies

### 10.1 Horizontal Scaling (More Agents)

**Agent Distribution Architecture:**

```
┌──────────────────────────────────────────────────────────────┐
│                    Load Balancer                              │
└─────────────────────────┬────────────────────────────────────┘
                          │
    ┌─────────────────────┼─────────────────────┐
    │                     │                     │
┌───▼────┐          ┌────▼────┐          ┌─────▼───┐
│ Node 1 │          │ Node 2  │          │ Node N  │
│ ┌────┐ │          │ ┌────┐ │          │ ┌────┐ │
│ │A1  │ │          │ │A5  │ │          │ │A9  │ │
│ │A2  │ │          │ │A6  │ │          │ │A10 │ │
│ │A3  │ │          │ │A7  │ │          │ │A11 │ │
│ │A4  │ │          │ │A8  │ │          │ │A12 │ │
│ └────┘ │          │ └────┘ │          │ └────┘ │
└────────┘          └────────┘          └────────┘
```

**Consistent Hashing for Agent Assignment:**

```typescript
import { createHash } from 'crypto';

class AgentRouter {
  private nodes: string[] = [];
  private ring: Map<number, string> = new Map();
  private readonly VNODES = 150; // Virtual nodes per physical node

  addNode(nodeId: string): void {
    this.nodes.push(nodeId);

    // Create virtual nodes
    for (let i = 0; i < this.VNODES; i++) {
      const hash = this.hash(`${nodeId}:${i}`);
      this.ring.set(hash, nodeId);
    }

    // Sort ring
    this.ring = new Map([...this.ring.entries()].sort((a, b) => a[0] - b[0]));
  }

  removeNode(nodeId: string): void {
    this.nodes = this.nodes.filter(n => n !== nodeId);

    for (let i = 0; i < this.VNODES; i++) {
      const hash = this.hash(`${nodeId}:${i}`);
      this.ring.delete(hash);
    }
  }

  getNode(agentId: string): string {
    const hash = this.hash(agentId);

    for (const [ringHash, nodeId] of this.ring.entries()) {
      if (ringHash >= hash) {
        return nodeId;
      }
    }

    // Wrap around
    return this.ring.values().next().value;
  }

  private hash(key: string): number {
    return parseInt(createHash('md5').update(key).digest('hex').substring(0, 8), 16);
  }
}
```

### 10.2 Vertical Scaling (Bigger Models)

**Model Selection Strategy:**

```typescript
interface ModelConfig {
  name: string;
  contextWindow: number;
  maxOutput: number;
  costPer1KInput: number;
  costPer1KOutput: number;
  latency: 'low' | 'medium' | 'high';
  capabilities: string[];
}

const MODELS: Record<string, ModelConfig> = {
  'claude-3-haiku': {
    name: 'Claude 3 Haiku',
    contextWindow: 200000,
    maxOutput: 4096,
    costPer1KInput: 0.00025,
    costPer1KOutput: 0.00125,
    latency: 'low',
    capabilities: ['chat', 'code', 'analysis']
  },
  'claude-3-sonnet': {
    name: 'Claude 3 Sonnet',
    contextWindow: 200000,
    maxOutput: 4096,
    costPer1KInput: 0.003,
    costPer1KOutput: 0.015,
    latency: 'medium',
    capabilities: ['chat', 'code', 'analysis', 'creative']
  },
  'claude-3-opus': {
    name: 'Claude 3 Opus',
    contextWindow: 200000,
    maxOutput: 4096,
    costPer1KInput: 0.015,
    costPer1KOutput: 0.075,
    latency: 'high',
    capabilities: ['chat', 'code', 'analysis', 'creative', 'reasoning']
  }
};

// Model selection based on task complexity
function selectModel(task: Task): string {
  if (task.complexity === 'simple' || task.type === 'lookup') {
    return 'claude-3-haiku';
  }

  if (task.complexity === 'complex' || task.requiresReasoning) {
    return 'claude-3-opus';
  }

  return 'claude-3-sonnet'; // Default
}
```

### 10.3 Database Sharding

**Sharding by Agent ID:**

```typescript
class ShardedDatabase {
  private shards: Pool[] = [];
  private readonly SHARD_COUNT = 8;

  constructor() {
    for (let i = 0; i < this.SHARD_COUNT; i++) {
      this.shards.push(new Pool({
        host: `shard-${i}.db.internal`,
        database: `fragment_shard_${i}`,
        user: process.env.DB_USER,
        password: process.env.DB_PASSWORD
      }));
    }
  }

  private getShard(agentId: string): Pool {
    const hash = this.hash(agentId);
    const shardIndex = hash % this.SHARD_COUNT;
    return this.shards[shardIndex];
  }

  async storeMessage(agentId: string, message: Message): Promise<void> {
    const shard = this.getShard(agentId);

    await shard.query(
      'INSERT INTO messages (agent_id, content, role, created_at) VALUES ($1, $2, $3, NOW())',
      [agentId, message.content, message.role]
    );
  }

  async getMessages(agentId: string, limit: number): Promise<Message[]> {
    const shard = this.getShard(agentId);

    const result = await shard.query(
      'SELECT * FROM messages WHERE agent_id = $1 ORDER BY created_at DESC LIMIT $2',
      [agentId, limit]
    );

    return result.rows;
  }

  private hash(agentId: string): number {
    let hash = 0;
    for (let i = 0; i < agentId.length; i++) {
      hash = ((hash << 5) - hash) + agentId.charCodeAt(i);
      hash |= 0;
    }
    return Math.abs(hash);
  }
}
```

**Cross-Shard Queries:**

```typescript
class CrossShardQuery {
  async getAllAgentsStats(): Promise<AgentStats[]> {
    // Query all shards in parallel
    const results = await Promise.all(
      this.shards.map(shard =>
        shard.query('SELECT agent_id, COUNT(*) as count FROM messages GROUP BY agent_id')
      )
    );

    // Combine results
    return results.flatMap(r => r.rows);
  }

  async getTenantTotalMessages(tenantId: string): Promise<number> {
    const results = await Promise.all(
      this.shards.map(shard =>
        shard.query(
          'SELECT COUNT(*) as count FROM messages WHERE tenant_id = $1',
          [tenantId]
        )
      )
    );

    return results.reduce((sum, r) => sum + parseInt(r.rows[0].count), 0);
  }
}
```

### 10.4 CDN for Static Assets

**CloudFront Configuration:**

```yaml
# CloudFormation template for CloudFront distribution
Resources:
  FragmentCDN:
    Type: AWS::CloudFront::Distribution
    Properties:
      DistributionConfig:
        Origins:
          - Id: S3Origin
            DomainName: fragment-static.s3.amazonaws.com
            S3OriginConfig:
              OriginAccessIdentity: origin-access-identity/cloudfront/XYZ
        Enabled: true
        Comment: CDN for Fragment static assets
        DefaultCacheBehavior:
          TargetOriginId: S3Origin
          ViewerProtocolPolicy: redirect-to-https
          AllowedMethods:
            - GET
            - HEAD
            - OPTIONS
          CachedMethods:
            - GET
            - HEAD
            - OPTIONS
          ForwardedValues:
            QueryString: false
            Cookies:
              Forward: none
          MinTTL: 3600
          DefaultTTL: 86400
          MaxTTL: 604800
        PriceClass: PriceClass_100
        ViewerCertificate:
          AcmCertificateArn: !Ref CertificateArn
          SslSupportMethod: sni-only
```

**Asset Upload:**

```typescript
import { S3Client, PutObjectCommand } from '@aws-sdk/client-s3';

const s3 = new S3Client({});

async function uploadStaticAsset(path: string, content: Buffer, contentType: string): Promise<string> {
  const key = `static/${path}`;

  await s3.send(new PutObjectCommand({
    Bucket: 'fragment-static',
    Key: key,
    Body: content,
    ContentType: contentType,
    CacheControl: 'public, max-age=31536000' // 1 year
  }));

  return `https://cdn.example.com/${key}`;
}
```

---

## Appendix A: Architecture Diagrams

### Complete Production Architecture

```
                                    ┌─────────────────┐
                                    │   CloudFlare    │
                                    │   (CDN + WAF)   │
                                    └────────┬────────┘
                                             │
                                    ┌────────▼────────┐
                                    │  Load Balancer  │
                                    │  (ALB/NGINX)    │
                                    └────────┬────────┘
                                             │
        ┌────────────────────────────────────┼────────────────────────────────────┐
        │                                    │                                    │
┌───────▼────────┐                 ┌─────────▼────────┐                 ┌─────────▼────────┐
│  Agent Tier 1  │                 │  Agent Tier 2    │                 │  Agent Tier N    │
│  ┌──────────┐  │                 │  ┌──────────┐    │                 │  ┌──────────┐    │
│  │  Agent   │  │                 │  │  Agent   │    │                 │  │  Agent   │    │
│  │  Runner  │  │                 │  │  Runner  │    │                 │  │  Runner  │    │
│  └──────────┘  │                 │  └──────────┘    │                 │  └──────────┘    │
│       │        │                 │        │         │                 │        │         │
│  ┌────▼────┐   │                 │  ┌────▼────┐    │                 │  ┌────▼────┐    │
│  │ Effect  │   │                 │  │ Effect  │    │                 │  │ Effect  │    │
│  │ Runtime │   │                 │  │ Runtime │    │                 │  │ Runtime │    │
│  └─────────┘   │                 │  └─────────┘    │                 │  └─────────┘    │
└───────┬────────┘                 └─────────┬────────┘                 └─────────┬────────┘
        │                                    │                                    │
        └────────────────────────────────────┼────────────────────────────────────┘
                                             │
              ┌──────────────────────────────┼──────────────────────────────┐
              │                              │                              │
        ┌─────▼─────┐                 ┌──────▼──────┐              ┌───────▼───────┐
        │ PostgreSQL│                 │    Redis    │              │     Kafka     │
        │  Primary  │                 │   Cluster   │              │    Cluster    │
        └─────┬─────┘                 └─────────────┘              └───────────────┘
              │
        ┌─────▼─────┐
        │ PostgreSQL│
        │  Replicas │
        └───────────┘

        ┌──────────────────────────────────────────────────────────────────┐
        │                      Observability Stack                          │
        │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐         │
        │  │Prometheus│  │  Jaeger  │  │   Loki   │  │ Grafana  │         │
        │  └──────────┘  └──────────┘  └──────────┘  └──────────┘         │
        └──────────────────────────────────────────────────────────────────┘
```

### Data Flow: Agent Message Processing

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Client  │────▶│    LB    │────▶│  Agent   │────▶│  Effect  │────▶│  Model   │
│          │     │          │     │  Runner  │     │  Runtime │     │  (API)   │
└──────────┘     └──────────┘     └──────────┘     └──────────┘     └──────────┘
     │                │                │                │                │
     │  1. HTTP/WS    │                │                │                │
     │  Request       │                │                │                │
     │                │  2. Route      │                │                │
     │                │  to Agent      │                │                │
     │                │                │  3. Enqueue    │                │
     │                │                │  Message       │                │
     │                │                │                │  4. Process    │
     │                │                │                │  Effect        │
     │                │                │                │                │  5. LLM
     │                │                │                │                │  Call
     │                │                │                │◀───────────────┘
     │                │                │◀───────────────┘                │
     │                │◀───────────────┘                                 │
     │◀───────────────┘                                                  │
     │                                                                   │
     │  8. Stream Response                                               │
     │◀──────────────────────────────────────────────────────────────────┘
```

---

## Appendix B: Configuration Examples

### Complete Production Configuration

```yaml
# config/production.yaml
server:
  port: 3000
  workers: 4
  keepAliveTimeout: 65000

database:
  host: postgres.internal
  port: 5432
  name: fragment
  user: fragment
  pool:
    min: 10
    max: 100
    idleTimeout: 30000

redis:
  host: redis.internal
  port: 6379
  cluster:
    enabled: true
    nodes:
      - redis-1.internal:6379
      - redis-2.internal:6379
      - redis-3.internal:6379

kafka:
  brokers:
    - kafka-1.internal:9092
    - kafka-2.internal:9092
    - kafka-3.internal:9092
  topics:
    messages: fragment-messages
    events: fragment-events

agents:
  maxConcurrent: 500
  maxContextTokens: 128000
  defaultModel: claude-3-sonnet
  messageQueueSize: 10000

cache:
  enabled: true
  ttl: 300  # 5 minutes
  maxSize: 10000

rateLimit:
  enabled: true
  requestsPerSecond: 100
  burstSize: 200

monitoring:
  metrics:
    enabled: true
    port: 9090
  tracing:
    enabled: true
    exporter: jaeger
    endpoint: http://jaeger.internal:14268/api/traces
  logging:
    level: info
    format: json

security:
  cors:
    origins:
      - https://app.example.com
  auth:
    type: jwt
    secretEnv: JWT_SECRET
  sandbox:
    enabled: true
    type: docker
```

---

## Appendix C: Operational Runbooks

### Runbook: Scaling Agents

```markdown
# Scaling Runbook

## Horizontal Scaling

### Add New Node

1. Provision new instance
   ```bash
   terraform apply -var="instance_count=+1"
   ```

2. Register with load balancer
   ```bash
   aws elbv2 register-targets \
     --target-group-arn arn:aws:elasticloadbalancing:... \
     --targets Id=i-xxxxxxxx
   ```

3. Verify health
   ```bash
   curl http://new-node:3000/health/ready
   ```

### Remove Node

1. Drain connections
   ```bash
   curl -X POST http://lb/api/nodes/node-id/drain
   ```

2. Wait for active agents to complete
   ```bash
   watch -n 5 'curl http://node:3000/api/agents | jq ".active"'
   ```

3. Deregister from load balancer
   ```bash
   aws elbv2 deregister-targets ...
   ```

4. Terminate instance
   ```bash
   aws ec2 terminate-instances --instance-ids i-xxxxxxxx
   ```

## Vertical Scaling

### Increase Context Window

1. Update configuration
   ```yaml
   agents:
     maxContextTokens: 200000  # Increase from 128000
   ```

2. Restart agents
   ```bash
   kubectl rollout restart deployment/fragment-agents
   ```

3. Monitor memory
   ```bash
   kubectl top pods -l app=fragment-agents
   ```
```

### Runbook: Incident Response

```markdown
# Incident Response Runbook

## P0: Complete Service Outage

### Symptoms
- All health checks failing
- No agents responding
- Error rate at 100%

### Response

1. Page on-call engineer
   ```bash
   pagerduty trigger "Fragment Service Outage"
   ```

2. Check infrastructure status
   ```bash
   kubectl get pods -l app=fragment-agents
   kubectl get events --sort-by='.lastTimestamp'
   ```

3. Check dependencies
   ```bash
   redis-cli ping
   psql -h postgres -c "SELECT 1"
   ```

4. If database issue
   - Follow database failover runbook

5. If application issue
   - Rollback to last known good version
   ```bash
   kubectl rollout undo deployment/fragment-agents
   ```

6. Post-incident
   - Create incident report
   - Schedule post-mortem
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial production-grade guide created |

---

*This document complements the Fragment exploration series. See [exploration.md](exploration.md) for the complete table of contents.*
