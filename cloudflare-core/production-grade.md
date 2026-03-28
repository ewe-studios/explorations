---
title: "Cloudflare Core: Production-Grade Implementation Guide"
subtitle: "From prototype to production - Performance, scaling, monitoring, and security"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/cloudflare-core
explored_at: 2026-03-27
---

# Production-Grade Cloudflare Core

## Overview

This guide covers production deployment considerations for all 8 Cloudflare Core subsystems. It addresses performance optimization, scaling strategies, monitoring, observability, and security hardening.

---

## Table of Contents

1. [Performance Optimization](#1-performance-optimization)
2. [Scaling Strategies](#2-scaling-strategies)
3. [Monitoring & Observability](#3-monitoring--observability)
4. [Security Hardening](#4-security-hardening)
5. [High Availability](#5-high-availability)
6. [Disaster Recovery](#6-disaster-recovery)
7. [Cost Optimization](#7-cost-optimization)

---

## 1. Performance Optimization

### 1.1 Workers Performance

**CPU Time Optimization:**

```typescript
// ❌ Inefficient: Synchronous blocking
function processData(data: string[]) {
  let result = '';
  for (const item of data) {
    result += processItem(item);  // String concatenation is O(n²)
  }
  return result;
}

// ✅ Efficient: Use array join
function processData(data: string[]) {
  return data.map(item => processItem(item)).join('');
}
```

**Memory Optimization:**

```typescript
// ❌ Inefficient: Create many objects
function parseResponse(text: string) {
  const lines = text.split('\n');
  const objects = [];
  for (const line of lines) {
    objects.push({ line, processed: true, timestamp: Date.now() });
  }
  return objects;
}

// ✅ Efficient: Stream processing
async function* parseResponse(stream: ReadableStream) {
  const reader = stream.getReader();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += new TextDecoder().decode(value);
    const lines = buffer.split('\n');
    buffer = lines.pop() || '';

    for (const line of lines) {
      yield { line, processed: true };
    }
  }
}
```

**Caching Strategies:**

```typescript
// Multi-level caching
const caches = {
  // L1: In-memory (fastest, smallest)
  l1: new Map<string, CacheEntry>(),

  // L2: Cloudflare Cache (fast, medium)
  async l2(key: string): Promise<Response | null> {
    const cache = caches.default;
    return await cache.match(key);
  },

  // L3: KV Storage (slower, largest)
  async l3(key: string): Promise<string | null> {
    return await KV.get(key);
  }
};

async function getWithCache(key: string): Promise<string> {
  // Check L1
  const l1 = caches.l1.get(key);
  if (l1 && !l1.expired()) return l1.value;

  // Check L2
  const l2 = await caches.l2(key);
  if (l2) {
    const value = await l2.text();
    caches.l1.set(key, { value, expiry: Date.now() + 60000 });
    return value;
  }

  // Check L3
  const l3 = await caches.l3(key);
  if (l3) {
    caches.l1.set(key, { value: l3, expiry: Date.now() + 60000 });
    return l3;
  }

  // Cache miss - fetch and cache
  const value = await fetchFromOrigin();
  await caches.l3.set(key, value);
  caches.l1.set(key, { value, expiry: Date.now() + 60000 });
  return value;
}
```

### 1.2 Durable Objects Performance

**Batching Operations:**

```typescript
export class BatchedAgent extends Agent<Env, State> {
  private pendingWrites: Map<string, any> = new Map();
  private writeTimer: number | null = null;

  async setState(newState: Partial<State>): Promise<void> {
    // Batch writes
    for (const [key, value] of Object.entries(newState)) {
      this.pendingWrites.set(key, value);
    }

    // Debounce flush
    if (this.writeTimer) clearTimeout(this.writeTimer);
    this.writeTimer = setTimeout(() => this.flushWrites(), 100) as unknown as number;
  }

  private async flushWrites(): Promise<void> {
    if (this.pendingWrites.size === 0) return;

    const batch = Object.fromEntries(this.pendingWrites);
    await this.ctx.storage.put(batch);

    this.pendingWrites.clear();
    this.state = { ...this.state, ...batch };
  }
}
```

**Connection Pooling:**

```typescript
export class PooledAgent extends Agent<Env, {}> {
  private connections: Map<string, Connection> = new Map();
  private maxConnections = 10;

  async getConnection(id: string): Promise<Connection> {
    if (this.connections.has(id)) {
      return this.connections.get(id)!;
    }

    if (this.connections.size >= this.maxConnections) {
      // Evict oldest connection
      const oldest = this.connections.keys().next().value;
      await this.connections.get(oldest)?.close();
      this.connections.delete(oldest);
    }

    const conn = await this.createConnection(id);
    this.connections.set(id, conn);
    return conn;
  }
}
```

### 1.3 AI Inference Performance

**Model Selection:**

| Model | Latency (p50) | Latency (p95) | Use Case |
|-------|---------------|---------------|----------|
| Llama 3 8B | 100ms | 300ms | Simple queries |
| Llama 3 70B | 500ms | 1500ms | Complex reasoning |
| Gemma 7B | 80ms | 250ms | Fast responses |

**Batching Requests:**

```typescript
class AIBatcher {
  private queue: QueueItem[] = [];
  private batchSize = 4;
  private batchTimeout = 50;

  async enqueue(item: QueueItem): Promise<string> {
    return new Promise((resolve) => {
      this.queue.push({ item, resolve });

      if (this.queue.length >= this.batchSize) {
        this.flush();
      } else if (this.queue.length === 1) {
        setTimeout(() => this.flush(), this.batchTimeout);
      }
    });
  }

  private async flush(): Promise<void> {
    const batch = this.queue.splice(0, this.batchSize);
    const prompts = batch.map(q => q.item.prompt);

    const results = await ai.run('@cf/llama-3', {
      messages: prompts.map(p => ({ role: 'user', content: p }))
    });

    batch.forEach((q, i) => q.resolve(results[i]));
  }
}
```

### 1.4 RPC Performance

**Promise Pipelining:**

```typescript
// ❌ Sequential calls (N RTTs)
async function getUserData(userId: string) {
  const user = await api.getUser(userId);      // RTT 1
  const posts = await user.getPosts();         // RTT 2
  const comments = await posts[0].getComments(); // RTT 3
  return { user, posts, comments };
}

// ✅ Pipelined calls (1 RTT)
async function getUserData(userId: string) {
  const pipeline = api.pipeline;
  const userPromise = pipeline.getUser(userId);
  const postsPromise = userPromise.getProperty('posts');
  const commentsPromise = postsPromise.map(ps => ps[0]).getProperty('comments');

  const [user, posts, comments] = await Promise.all([
    userPromise,
    postsPromise,
    commentsPromise
  ]);

  return { user, posts, comments };
}
```

---

## 2. Scaling Strategies

### 2.1 Horizontal Scaling

**Agent Sharding:**

```typescript
function getAgentShard(agentId: string, numShards: number): number {
  // Consistent hashing
  const hash = await crypto.subtle.digest('SHA-256',
    new TextEncoder().encode(agentId)
  );
  const view = new DataView(hash);
  return view.getUint32(0) % numShards;
}

// Route to correct shard
export default {
  async fetch(request: Request, env: Env) {
    const agentId = extractAgentId(request);
    const shard = getAgentShard(agentId, 10);

    const binding = env[`AGENT_SHARD_${shard}`];
    const agent = binding.get(agentId);

    return agent.fetch(request);
  }
};
```

**Load Balancing:**

```typescript
// Weighted round-robin
class LoadBalancer {
  private instances: Instance[];
  private current = 0;
  private weights: number[];

  getNext(): Instance {
    const totalWeight = this.weights.reduce((a, b) => a + b, 0);
    let random = Math.random() * totalWeight;

    for (let i = 0; i < this.instances.length; i++) {
      random -= this.weights[i];
      if (random <= 0) {
        return this.instances[i];
      }
    }

    return this.instances[0];
  }
}
```

### 2.2 Vertical Scaling

**Memory Management:**

```typescript
// Monitor memory usage
export default {
  async fetch(request: Request) {
    const memory = performance.memory;

    if (memory.usedJSHeapSize > memory.jsHeapSizeLimit * 0.8) {
      // Trigger GC-friendly behavior
      globalThis.gc?.();

      // Return early if critical
      if (memory.usedJSHeapSize > memory.jsHeapSizeLimit * 0.9) {
        return new Response('Service overloaded', { status: 503 });
      }
    }

    // Normal processing...
  }
};
```

### 2.3 Geographic Distribution

**Region-aware Routing:**

```typescript
export default {
  async fetch(request: Request, env: Env) {
    const cf = request.cf as CfProperties;
    const region = cf?.continent || 'NA';

    const binding = {
      'NA': env.AGENTS_NA,
      'EU': env.AGENTS_EU,
      'AS': env.AGENTS_AS,
    }[region] || env.AGENTS_NA;

    const agent = binding.get(getAgentId(request));
    return agent.fetch(request);
  }
};
```

---

## 3. Monitoring & Observability

### 3.1 Metrics Collection

**Custom Metrics:**

```typescript
export default {
  async fetch(request: Request, env: Env) {
    const start = Date.now();

    try {
      const response = await handleRequest(request, env);

      // Record success metrics
      env.METRICS.writeDataPoint({
        blobs: ['request', 'success'],
        doubles: [Date.now() - start],
        indexes: [request.method]
      });

      return response;
    } catch (error) {
      // Record error metrics
      env.METRICS.writeDataPoint({
        blobs: ['request', 'error', error.message],
        doubles: [Date.now() - start],
        indexes: [request.method]
      });

      throw error;
    }
  }
};
```

**Key Metrics to Track:**

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| Request latency (p50) | Median response time | > 100ms |
| Request latency (p99) | Tail latency | > 500ms |
| Error rate | % of failed requests | > 1% |
| CPU time | Worker CPU usage | > 40ms |
| Memory usage | Heap size | > 80% |
| Durable Object activations | Active DOs | Sudden spike |

### 3.2 Distributed Tracing

```typescript
import { Span, TraceParent } from '@cloudflare/trace';

export default {
  async fetch(request: Request, env: Env) {
    const traceParent = TraceParent.fromRequest(request);
    const span = env.TRACER.startSpan('handle_request', {
      parent: traceParent,
      tags: {
        method: request.method,
        path: new URL(request.url).pathname
      }
    });

    try {
      const response = await handleRequest(request, env, span);
      span.setTag('status', response.status);
      return response;
    } catch (error) {
      span.setTag('error', error.message);
      throw error;
    } finally {
      span.end();
    }
  }
};
```

### 3.3 Logging

**Structured Logging:**

```typescript
interface LogEntry {
  timestamp: string;
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
  context: {
    requestId: string;
    userId?: string;
    duration?: number;
  };
  error?: {
    name: string;
    message: string;
    stack?: string;
  };
}

class Logger {
  private baseContext: Partial<LogEntry['context']>;

  constructor(baseContext: Partial<LogEntry['context']>) {
    this.baseContext = baseContext;
  }

  info(message: string, context: Record<string, any> = {}) {
    this.log('info', message, context);
  }

  error(message: string, error: Error, context: Record<string, any> = {}) {
    this.log('error', message, {
      ...context,
      error: {
        name: error.name,
        message: error.message,
        stack: error.stack
      }
    });
  }

  private log(level: string, message: string, context: Record<string, any>) {
    console.log(JSON.stringify({
      timestamp: new Date().toISOString(),
      level,
      message,
      context: { ...this.baseContext, ...context }
    }));
  }
}
```

---

## 4. Security Hardening

### 4.1 Authentication

**JWT Validation:**

```typescript
import { jwtVerify } from 'jose';

async function validateJWT(token: string, secret: string) {
  try {
    const { payload } = await jwtVerify(token, new TextEncoder().encode(secret));
    return { valid: true, payload };
  } catch {
    return { valid: false };
  }
}

export default {
  async fetch(request: Request, env: Env) {
    const authHeader = request.headers.get('Authorization');

    if (!authHeader?.startsWith('Bearer ')) {
      return new Response('Unauthorized', { status: 401 });
    }

    const token = authHeader.slice(7);
    const result = await validateJWT(token, env.JWT_SECRET);

    if (!result.valid) {
      return new Response('Unauthorized', { status: 401 });
    }

    // Attach user to request
    return handleRequest(request, { ...env, user: result.payload });
  }
};
```

### 4.2 Rate Limiting

```typescript
class RateLimiter {
  private storage: DurableObjectStorage;
  private limit: number;
  private window: number;

  async check(identifier: string): Promise<boolean> {
    const key = `rate:${identifier}`;
    const now = Date.now();
    const windowStart = now - this.window;

    const requests = await this.storage.get<number[]>(key) || [];
    const validRequests = requests.filter(t => t > windowStart);

    if (validRequests.length >= this.limit) {
      return false;
    }

    validRequests.push(now);
    await this.storage.put(key, validRequests);
    return true;
  }
}
```

### 4.3 Input Validation

```typescript
import { z } from 'zod';

const RequestSchema = z.object({
  name: z.string().min(1).max(100),
  email: z.string().email(),
  age: z.number().min(0).max(150)
});

async function handleRequest(request: Request) {
  const body = await request.json();

  try {
    const validated = RequestSchema.parse(body);
    // Process validated data
  } catch (error) {
    return new Response(JSON.stringify({
      error: 'Validation failed',
      details: error.errors
    }), { status: 400 });
  }
}
```

---

## 5. High Availability

### 5.1 Redundancy

**Multi-region Deployment:**

```toml
# wrangler.toml
[vars]
PRIMARY_REGION = "us-east"
FALLBACK_REGION = "eu-west"

[[durable_objects.bindings]]
name = "AGENT"
class_name = "Agent"
script_name = "agents-us"

[[durable_objects.bindings]]
name = "AGENT_EU"
class_name = "Agent"
script_name = "agents-eu"
```

**Health Checks:**

```typescript
export class HealthAgent extends Agent<Env, {}> {
  async healthCheck(): Promise<HealthStatus> {
    const checks = await Promise.allSettled([
      this.checkStorage(),
      this.checkExternalDeps(),
      this.checkMemory()
    ]);

    const healthy = checks.every(c => c.status === 'fulfilled');

    return {
      healthy,
      timestamp: Date.now(),
      checks: checks.map((c, i) => ({
        name: ['storage', 'external', 'memory'][i],
        status: c.status
      }))
    };
  }
}
```

### 5.2 Circuit Breaker

```typescript
class CircuitBreaker {
  private failures = 0;
  private lastFailure = 0;
  private state: 'closed' | 'open' | 'half-open' = 'closed';
  private threshold = 5;
  private timeout = 60000;

  async execute<T>(fn: () => Promise<T>): Promise<T> {
    if (this.state === 'open') {
      if (Date.now() - this.lastFailure > this.timeout) {
        this.state = 'half-open';
      } else {
        throw new Error('Circuit breaker open');
      }
    }

    try {
      const result = await fn();
      if (this.state === 'half-open') {
        this.state = 'closed';
        this.failures = 0;
      }
      return result;
    } catch (error) {
      this.failures++;
      this.lastFailure = Date.now();

      if (this.failures >= this.threshold) {
        this.state = 'open';
      }

      throw error;
    }
  }
}
```

---

## 6. Disaster Recovery

### 6.1 Backup Strategy

```typescript
export class BackupAgent extends Agent<Env, {}> {
  async backup(): Promise<void> {
    const state = await this.ctx.storage.list();
    const backup = {
      timestamp: Date.now(),
      state: Object.fromEntries(state)
    };

    // Store in R2
    await env.BACKUP_BUCKET.put(
      `backup-${this.id}-${Date.now()}.json`,
      JSON.stringify(backup)
    );
  }

  async restore(backupKey: string): Promise<void> {
    const backup = await env.BACKUP_BUCKET.get(backupKey);
    const data = JSON.parse(await backup.text());

    for (const [key, value] of Object.entries(data.state)) {
      await this.ctx.storage.put(key, value);
    }
  }
}
```

### 6.2 Rollback Procedures

```yaml
# deployment.yaml
strategy:
  type: RollingUpdate
  rollingUpdate:
    maxSurge: 25%
    maxUnavailable: 25%

rollback:
  triggers:
    - errorRate > 5%
    - latencyP99 > 1000ms
    - availability < 99%
```

---

## 7. Cost Optimization

### 7.1 Request Optimization

```typescript
// Batch API calls
async function getUserData(userIds: string[]) {
  // Instead of N individual calls
  // return Promise.all(userIds.map(id => api.getUser(id)));

  // Use batch endpoint
  return api.getUsersBatch(userIds);
}
```

### 7.2 Caching ROI

```typescript
// Calculate cache effectiveness
const cacheMetrics = {
  requests: 1000000,
  cacheHits: 750000,
  originRequests: 250000,

  get hitRate() {
    return this.cacheHits / this.requests;
  },

  get savings() {
    // $0.30 per 10M requests to origin
    return (this.requests - this.cacheHits) * 0.30 / 10000000;
  }
};
```

---

## Production Checklist

### Pre-deployment

- [ ] Load testing completed
- [ ] Security audit passed
- [ ] Monitoring configured
- [ ] Alerting thresholds set
- [ ] Runbook documented
- [ ] Rollback plan tested

### Post-deployment

- [ ] Health checks passing
- [ ] Metrics within thresholds
- [ ] No error spikes
- [ ] Logs clean
- [ ] User feedback monitored

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial production guide created |
| 2026-03-27 | All sections documented |
| 2026-03-27 | Checklist added |

---

*This guide is a living document. Update as production experience grows.*
