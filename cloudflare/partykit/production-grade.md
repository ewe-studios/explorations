---
title: "Production-Grade PartyKit: Deployment, Scaling, and Operations"
subtitle: "Complete guide to deploying, scaling, monitoring, and operating PartyKit/PartyServer in production"
based_on: "PartyServer packages/, Cloudflare Workers production patterns"
---

# Production-Grade PartyKit: Deployment, Scaling, and Operations

## Table of Contents

1. [Production Architecture](#1-production-architecture)
2. [Deployment Strategies](#2-deployment-strategies)
3. [Scaling Patterns](#3-scaling-patterns)
4. [Monitoring and Observability](#4-monitoring-and-observability)
5. [Rate Limiting and Security](#5-rate-limiting-and-security)
6. [Disaster Recovery](#6-disaster-recovery)
7. [Cost Optimization](#7-cost-optimization)

---

## 1. Production Architecture

### 1.1 Production Topology

```
┌─────────────────────────────────────────────────────────────────┐
│                     Cloudflare Edge Network                      │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Workers Runtime                        │   │
│  │                                                            │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │  Frontend   │  │  Frontend   │  │  Frontend   │       │   │
│  │  │   Worker    │  │   Worker    │  │   Worker    │       │   │
│  │  │             │  │             │  │             │       │   │
│  │  │ - Auth      │  │ - Auth      │  │ - Auth      │       │   │
│  │  │ - Routing   │  │ - Routing   │  │ - Routing   │       │   │
│  │  │ - Rate      │  │ - Rate      │  │ - Rate      │       │   │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘       │   │
│  │         │                │                │               │   │
│  │         └────────────────┼────────────────┘               │   │
│  │                          │                                │   │
│  │         ┌────────────────▼────────────────┐               │   │
│  │         │     Durable Object Gateway      │               │   │
│  │         └────────────────┬────────────────┘               │   │
│  │                          │                                │   │
│  │  ┌─────────────┐  ┌──────┴──────┐  ┌─────────────┐       │   │
│  │  │     DO      │  │     DO      │  │     DO      │       │   │
│  │  │   Room A    │  │   Room B    │  │   Room C    │       │   │
│  │  │  (EU)       │  │  (WNAM)     │  │  (ENAM)     │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   External Services                       │   │
│  │                                                            │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │    R2       │  │   D1        │  │  External   │       │   │
│  │  │  (Assets)   │  │  (Analytics)│  │    APIs     │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Multi-Environment Setup

```toml
# wrangler.toml
name = "partykit-prod"
main = "src/worker.ts"
compatibility_date = "2024-01-01"

[vars]
ENV = "production"
LOG_LEVEL = "info"

# Production Durable Objects
[[durable_objects.bindings]]
name = "ChatServer"
class_name = "ChatServer"

# Staging environment
[env.staging]
name = "partykit-staging"
vars = { ENV = "staging", LOG_LEVEL = "debug" }

# Development environment
[env.dev]
name = "partykit-dev"
vars = { ENV = "development", LOG_LEVEL = "debug" }
```

### 1.3 Worker Configuration

```typescript
// src/worker.ts
import { routePartykitRequest, Server } from "partyserver";

export class ProductionChatServer extends Server {
  static options = {
    hibernate: true,  // Enable hibernation for cost savings
  };

  async onStart(props?: { maxUsers?: number; region?: string }) {
    // Initialize with production settings
    this.maxUsers = props?.maxUsers ?? 1000;
    this.region = props?.region ?? "auto";

    // Set up monitoring
    this.setupMetrics();

    // Initialize storage
    this.initializeStorage();
  }
}

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    // Add request ID for tracing
    const requestId = crypto.randomUUID();
    request.headers.set("x-request-id", requestId);

    // Add CORS for production
    const response = await routePartykitRequest(request, env, {
      cors: {
        "Access-Control-Allow-Origin": "https://app.example.com",
        "Access-Control-Allow-Credentials": "true",
        "Access-Control-Allow-Methods": "GET, POST, HEAD, OPTIONS",
        "Access-Control-Allow-Headers": "Content-Type, Authorization",
        "Access-Control-Max-Age": "86400"
      },
      onBeforeConnect: async (req, lobby) => {
        // Production auth check
        const token = new URL(req.url).searchParams.get("token");
        if (!token) {
          return new Response("Unauthorized", { status: 401 });
        }

        // Validate token with external auth service
        const valid = await validateToken(token, env);
        if (!valid) {
          return new Response("Invalid token", { status: 401 });
        }
      }
    });

    if (response) {
      return response;
    }

    return new Response("Not Found", {
      status: 404,
      headers: { "x-request-id": requestId }
    });
  }
} satisfies ExportedHandler<Env>;
```

---

## 2. Deployment Strategies

### 2.1 CI/CD Pipeline

```yaml
# .github/workflows/deploy.yml
name: Deploy PartyKit

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm ci
      - run: npm run check:lint
      - run: npm run check:type
      - run: npm run check:test

  deploy-staging:
    needs: test
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
      - uses: cloudflare/wrangler-action@v3
        with:
          apiToken: ${{ secrets.CF_API_TOKEN }}
          environment: staging

  deploy-production:
    needs: test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - uses: cloudflare/wrangler-action@v3
        with:
          apiToken: ${{ secrets.CF_API_TOKEN }}
          environment: production
```

### 2.2 Blue-Green Deployment

```typescript
// src/deployment.ts
interface DeploymentConfig {
  version: string;
  trafficPercent: number;
  healthStatus: "healthy" | "degraded" | "unhealthy";
}

export class DeploymentManager {
  private currentVersion: string;
  private newVersion: string | null = null;

  async startRollout(newVersion: string) {
    this.newVersion = newVersion;

    // Start with 1% traffic
    await this.setTrafficSplit({ [this.currentVersion]: 99, [newVersion]: 1 });

    // Gradually increase
    const percentages = [1, 5, 10, 25, 50, 75, 100];
    for (const percent of percentages) {
      await this.waitForHealthCheck();
      if (!await this.isHealthy(newVersion)) {
        await this.rollback();
        return;
      }
      await this.setTrafficSplit({ [this.currentVersion]: 100 - percent, [newVersion]: percent });
    }

    this.currentVersion = newVersion;
    this.newVersion = null;
  }

  async rollback() {
    console.error(`Rolling back from ${this.newVersion} to ${this.currentVersion}`);
    await this.setTrafficSplit({ [this.currentVersion]: 100 });
    this.newVersion = null;
  }

  private async isHealthy(version: string): Promise<boolean> {
    // Check error rates, latency, etc.
    const metrics = await this.getMetrics(version);
    return metrics.errorRate < 0.01 && metrics.p99Latency < 500;
  }
}
```

### 2.3 Migration Strategy

```typescript
// src/migrations.ts
interface Migration {
  version: number;
  up(): Promise<void>;
  down(): Promise<void>;
}

const migrations: Migration[] = [
  {
    version: 1,
    async up() {
      this.ctx.storage.sql.exec(`
        CREATE TABLE IF NOT EXISTS messages (
          id TEXT PRIMARY KEY,
          content TEXT,
          created_at INTEGER
        )
      `);
    },
    async down() {
      this.ctx.storage.sql.exec(`DROP TABLE IF EXISTS messages`);
    }
  },
  {
    version: 2,
    async up() {
      this.ctx.storage.sql.exec(`
        ALTER TABLE messages ADD COLUMN sender_id TEXT
      `);
      this.ctx.storage.sql.exec(`
        CREATE INDEX idx_messages_sender ON messages(sender_id)
      `);
    },
    async down() {
      // Can't remove columns in SQLite, need to recreate table
      this.migrateMessagesV2Down();
    }
  }
];

export async function runMigrations(ctx: DurableObjectState) {
  const currentVersion = await ctx.storage.get<number>("schema_version") ?? 0;

  for (const migration of migrations) {
    if (migration.version > currentVersion) {
      console.log(`Running migration ${migration.version}`);
      await migration.up();
      await ctx.storage.put("schema_version", migration.version);
    }
  }
}
```

---

## 3. Scaling Patterns

### 3.1 Location-Based Scaling

```typescript
// src/scaling.ts
import { routePartykitRequest, getServerByName } from "partyserver";

const LOCATION_WEIGHTS = {
  eu: 3,    // 30% of nodes in Europe
  wnam: 4,  // 40% in West North America
  enam: 3,  // 30% in East North America
};

export async function routeWithLocation(request: Request, env: Env) {
  const url = new URL(request.url);
  const country = request.cf?.country;

  // Determine optimal location
  let locationHint: DurableObjectLocationHint = "wnam";
  if (country && isEuropeanCountry(country)) {
    locationHint = "eu";
  } else if (country && isEastUSCountry(country)) {
    locationHint = "enam";
  }

  // Get room with location hint
  const roomName = url.pathname.split("/").pop() || "default";
  const stub = await getServerByName(env.ChatServer, roomName, {
    locationHint
  });

  return stub.fetch(request);
}

function isEuropeanCountry(country: string): boolean {
  const euCountries = ["DE", "FR", "GB", "IT", "ES", "NL", "BE", "SE", "NO", "DK"];
  return euCountries.includes(country);
}
```

### 3.2 Room Sharding

```typescript
// For very large rooms, shard across multiple DOs
export class ShardedChatServer extends Server {
  private readonly SHARD_COUNT = 10;

  getShardForUser(userId: string): number {
    // Consistent hashing
    let hash = 0;
    for (let i = 0; i < userId.length; i++) {
      hash = ((hash << 5) - hash) + userId.charCodeAt(i);
      hash = hash & hash;
    }
    return Math.abs(hash) % this.SHARD_COUNT;
  }

  async broadcastToAllShards(message: string) {
    const promises = [];
    for (let i = 0; i < this.SHARD_COUNT; i++) {
      const shardName = `${this.name}-shard-${i}`;
      const stub = await getServerByName(this.env.ChatServer, shardName);
      promises.push(stub.fetch(new Request("http://internal/broadcast", {
        method: "POST",
        body: message
      })));
    }
    await Promise.all(promises);
  }
}
```

### 3.3 Connection Limits

```typescript
export class LimitedChatServer extends Server {
  private readonly MAX_CONNECTIONS = 1000;
  private readonly MAX_CONNECTIONS_PER_USER = 5;

  onConnect(connection: Connection, ctx: ConnectionContext) {
    // Check total connections
    if (this.getConnections().length >= this.MAX_CONNECTIONS) {
      connection.close(4003, "Room is full");
      return;
    }

    // Check per-user connections
    const state = connection.state as UserState;
    const userConnections = Array.from(this.getConnections())
      .filter(c => (c.state as UserState).userId === state.userId);

    if (userConnections.length >= this.MAX_CONNECTIONS_PER_USER) {
      connection.close(4003, "Too many connections for user");
      return;
    }

    // Accept connection
    // ...
  }
}
```

### 3.4 Load Shedding

```typescript
export class LoadSheddingServer extends Server {
  private readonly CPU_THRESHOLD = 0.8;
  private readonly MEMORY_THRESHOLD = 0.9;

  async onConnect(connection: Connection) {
    // Check system load
    const load = await this.getSystemLoad();

    if (load.cpu > this.CPU_THRESHOLD || load.memory > this.MEMORY_THRESHOLD) {
      // Shed load - reject new connections
      connection.close(5003, "Server under heavy load, please retry");

      // Log for monitoring
      console.warn("Load shedding triggered", { load });
      return;
    }

    // Accept connection
    // ...
  }

  private async getSystemLoad(): Promise<{ cpu: number; memory: number }> {
    // In Workers, we don't have direct CPU/memory access
    // Use connection count and message rate as proxy
    const connectionCount = this.getConnections().length;
    const messageRate = this.getMessageRate();

    return {
      cpu: Math.min(1, messageRate / 1000),  // Normalize to 1000 msg/s
      memory: Math.min(1, connectionCount / this.MAX_CONNECTIONS)
    };
  }

  private getMessageRate(): number {
    // Track messages per second
    // Implementation depends on your metrics system
    return 0;
  }
}
```

---

## 4. Monitoring and Observability

### 4.1 Metrics Collection

```typescript
// src/metrics.ts
interface Metrics {
  connections: {
    total: number;
    active: number;
    errors: number;
  };
  messages: {
    received: number;
    sent: number;
    dropped: number;
  };
  latency: {
    p50: number;
    p95: number;
    p99: number;
  };
  storage: {
    size: number;
    operations: number;
    errors: number;
  };
}

export class MetricsCollector {
  private metrics: Metrics = {
    connections: { total: 0, active: 0, errors: 0 },
    messages: { received: 0, sent: 0, dropped: 0 },
    latency: { p50: 0, p95: 0, p99: 0 },
    storage: { size: 0, operations: 0, errors: 0 }
  };

  incrementConnection() {
    this.metrics.connections.total++;
    this.metrics.connections.active++;
  }

  decrementConnection() {
    this.metrics.connections.active--;
  }

  recordMessage(sent: boolean) {
    if (sent) {
      this.metrics.messages.sent++;
    } else {
      this.metrics.messages.dropped++;
    }
  }

  recordError(type: "connection" | "message" | "storage") {
    this.metrics.connections.errors++;
  }

  async flush(env: Env) {
    // Send to D1 for analytics
    await env.ANALYTICS_DB.prepare(`
      INSERT INTO metrics (timestamp, metric_type, value)
      VALUES (?, ?, ?)
    `).bind(
      Date.now(),
      "connections.active",
      this.metrics.connections.active
    ).run();

    // Send to external monitoring (Datadog, Prometheus, etc.)
    await fetch("https://metrics.example.com/ingest", {
      method: "POST",
      body: JSON.stringify({
        metrics: this.metrics,
        timestamp: Date.now(),
        roomId: this.name
      })
    });

    // Reset counters
    this.metrics = {
      connections: { total: 0, active: 0, errors: 0 },
      messages: { received: 0, sent: 0, dropped: 0 },
      latency: { p50: 0, p95: 0, p99: 0 },
      storage: { size: 0, operations: 0, errors: 0 }
    };
  }
}
```

### 4.2 Distributed Tracing

```typescript
// src/tracing.ts
interface Span {
  traceId: string;
  spanId: string;
  parentSpanId?: string;
  operation: string;
  startTime: number;
  endTime?: number;
  attributes: Record<string, string>;
  status: "ok" | "error";
  errorMessage?: string;
}

export class Tracer {
  private spans: Span[] = [];

  startSpan(operation: string, attributes: Record<string, string> = {}): Span {
    const span: Span = {
      traceId: crypto.randomUUID(),
      spanId: crypto.randomUUID(),
      operation,
      startTime: Date.now(),
      attributes,
      status: "ok"
    };
    this.spans.push(span);
    return span;
  }

  endSpan(span: Span, errorMessage?: string) {
    span.endTime = Date.now();
    if (errorMessage) {
      span.status = "error";
      span.errorMessage = errorMessage;
    }
  }

  async export(env: Env) {
    if (this.spans.length === 0) return;

    // Export to tracing backend (Jaeger, Zipkin, etc.)
    await fetch("https://tracing.example.com/api/traces", {
      method: "POST",
      body: JSON.stringify({ traces: this.spans }),
      headers: { "Content-Type": "application/json" }
    });

    this.spans = [];
  }
}

// Usage in server
export class TracedChatServer extends Server {
  async onConnect(connection: Connection, ctx: ConnectionContext) {
    const span = this.tracer.startSpan("on_connect", {
      connection_id: connection.id,
      room: this.name
    });

    try {
      await super.onConnect(connection, ctx);
      this.tracer.endSpan(span);
    } catch (error) {
      this.tracer.endSpan(span, error.message);
      throw error;
    }
  }
}
```

### 4.3 Logging Strategy

```typescript
// src/logging.ts
type LogLevel = "debug" | "info" | "warn" | "error";

interface LogEntry {
  timestamp: string;
  level: LogLevel;
  message: string;
  roomId?: string;
  connectionId?: string;
  userId?: string;
  error?: string;
  stack?: string;
  context: Record<string, unknown>;
}

export class Logger {
  private level: LogLevel;
  private buffer: LogEntry[] = [];

  constructor(private env: Env, level: LogLevel = "info") {
    this.level = level;
  }

  private shouldLog(level: LogLevel): boolean {
    const levels: LogLevel[] = ["debug", "info", "warn", "error"];
    return levels.indexOf(level) >= levels.indexOf(this.level);
  }

  private format(entry: LogEntry): string {
    return JSON.stringify({
      ...entry,
      timestamp: new Date().toISOString()
    });
  }

  info(message: string, context: Record<string, unknown> = {}) {
    if (this.shouldLog("info")) {
      this.log("info", message, context);
    }
  }

  warn(message: string, context: Record<string, unknown> = {}) {
    if (this.shouldLog("warn")) {
      this.log("warn", message, context);
    }
  }

  error(message: string, error: Error, context: Record<string, unknown> = {}) {
    if (this.shouldLog("error")) {
      this.log("error", message, {
        ...context,
        error: error.message,
        stack: error.stack
      });
    }
  }

  private log(level: LogLevel, message: string, context: Record<string, unknown>) {
    const entry: LogEntry = {
      timestamp: new Date().toISOString(),
      level,
      message,
      roomId: this.name,
      context
    };

    this.buffer.push(entry);

    // Flush buffer periodically
    if (this.buffer.length >= 100) {
      this.flush();
    }
  }

  async flush() {
    if (this.buffer.length === 0) return;

    // Send to log aggregation service
    await fetch("https://logs.example.com/ingest", {
      method: "POST",
      body: JSON.stringify({ logs: this.buffer }),
      headers: { "Content-Type": "application/json" }
    }).catch(console.error);

    this.buffer = [];
  }
}
```

### 4.4 Alerting Rules

```typescript
// src/alerts.ts
interface AlertRule {
  name: string;
  condition: (metrics: Metrics) => boolean;
  severity: "critical" | "warning" | "info";
  message: string;
  cooldown: number;  // milliseconds
  lastTriggered?: number;
}

const alertRules: AlertRule[] = [
  {
    name: "high_error_rate",
    condition: (m) => m.connections.errors / m.connections.total > 0.05,
    severity: "critical",
    message: "Error rate exceeds 5%",
    cooldown: 5 * 60 * 1000
  },
  {
    name: "high_latency",
    condition: (m) => m.latency.p99 > 1000,
    severity: "warning",
    message: "P99 latency exceeds 1000ms",
    cooldown: 5 * 60 * 1000
  },
  {
    name: "room_full",
    condition: (m) => m.connections.active >= m.connections.total,
    severity: "warning",
    message: "Room at capacity",
    cooldown: 10 * 60 * 1000
  },
  {
    name: "storage_error",
    condition: (m) => m.storage.errors > 0,
    severity: "critical",
    message: "Storage operation failed",
    cooldown: 1 * 60 * 1000
  }
];

export function checkAlerts(metrics: Metrics, sendAlert: (rule: AlertRule) => void) {
  const now = Date.now();

  for (const rule of alertRules) {
    if (rule.condition(metrics)) {
      if (!rule.lastTriggered || now - rule.lastTriggered > rule.cooldown) {
        sendAlert(rule);
        rule.lastTriggered = now;
      }
    }
  }
}
```

---

## 5. Rate Limiting and Security

### 5.1 Rate Limiting

```typescript
// src/rate-limit.ts
export class RateLimiter {
  private windows: Map<string, { count: number; resetTime: number }> = new Map();
  private readonly WINDOW_MS = 60 * 1000;  // 1 minute
  private readonly MAX_REQUESTS = 100;

  checkLimit(key: string): { allowed: boolean; remaining: number; resetTime: number } {
    const now = Date.now();
    const window = this.windows.get(key);

    if (!window || now > window.resetTime) {
      // New window
      this.windows.set(key, { count: 1, resetTime: now + this.WINDOW_MS });
      return { allowed: true, remaining: this.MAX_REQUESTS - 1, resetTime: now + this.WINDOW_MS };
    }

    if (window.count >= this.MAX_REQUESTS) {
      return { allowed: false, remaining: 0, resetTime: window.resetTime };
    }

    window.count++;
    return { allowed: true, remaining: this.MAX_REQUESTS - window.count, resetTime: window.resetTime };
  }

  cleanup() {
    const now = Date.now();
    for (const [key, window] of this.windows.entries()) {
      if (now > window.resetTime) {
        this.windows.delete(key);
      }
    }
  }
}

// Usage in server
export class RateLimitedServer extends Server {
  private rateLimiter = new RateLimiter();

  async onConnect(connection: Connection) {
    const state = connection.state as UserState;
    const limit = this.rateLimiter.checkLimit(state.userId);

    if (!limit.allowed) {
      connection.close(4029, "Rate limit exceeded");
      return;
    }

    // Add rate limit headers to connection
    connection.setState(prev => ({
      ...prev,
      rateLimitRemaining: limit.remaining
    }));
  }
}
```

### 5.2 Authentication

```typescript
// src/auth.ts
import * as jose from "jose";

const JWT_SECRET = new TextEncoder().encode(process.env.JWT_SECRET);

export async function validateToken(token: string, env: Env): Promise<{ userId: string } | null> {
  try {
    const { payload } = await jose.jwtVerify(token, JWT_SECRET);
    return { userId: payload.sub as string };
  } catch {
    return null;
  }
}

export async function createToken(userId: string): Promise<string> {
  return await new jose.SignJWT({ userId })
    .setProtectedHeader({ alg: "HS256" })
    .setIssuedAt()
    .setExpirationTime("24h")
    .sign(JWT_SECRET);
}

// Usage
export class AuthServer extends Server {
  async onConnect(connection: Connection, ctx: ConnectionContext) {
    const url = new URL(ctx.request.url);
    const token = url.searchParams.get("token");

    if (!token) {
      connection.close(4001, "Missing authentication token");
      return;
    }

    const user = await validateToken(token, this.env);
    if (!user) {
      connection.close(4001, "Invalid authentication token");
      return;
    }

    connection.setState(prev => ({
      ...prev,
      userId: user.userId,
      authenticated: true
    }));
  }
}
```

### 5.3 Input Validation

```typescript
// src/validation.ts
const MAX_MESSAGE_LENGTH = 1000;
const MAX_USERNAME_LENGTH = 50;
const ALLOWED_CONTENT_TYPES = ["text/plain"];

export function validateMessage(content: unknown): { valid: boolean; error?: string } {
  if (typeof content !== "string") {
    return { valid: false, error: "Message must be a string" };
  }
  if (content.length > MAX_MESSAGE_LENGTH) {
    return { valid: false, error: `Message exceeds maximum length of ${MAX_MESSAGE_LENGTH}` };
  }
  if (content.length === 0) {
    return { valid: false, error: "Message cannot be empty" };
  }
  return { valid: true };
}

export function validateUsername(username: unknown): { valid: boolean; error?: string } {
  if (typeof username !== "string") {
    return { valid: false, error: "Username must be a string" };
  }
  if (username.length > MAX_USERNAME_LENGTH) {
    return { valid: false, error: `Username exceeds maximum length` };
  }
  if (!/^[a-zA-Z0-9_-]+$/.test(username)) {
    return { valid: false, error: "Username contains invalid characters" };
  }
  return { valid: true };
}

// Usage in server
onMessage(connection: Connection, message: WSMessage) {
  const data = JSON.parse(message as string);

  const validation = validateMessage(data.content);
  if (!validation.valid) {
    connection.send(JSON.stringify({
      type: "error",
      message: validation.error
    }));
    return;
  }

  // Process valid message
  // ...
}
```

---

## 6. Disaster Recovery

### 6.1 Backup Strategy

```typescript
// src/backup.ts
export class BackupManager {
  private readonly BACKUP_INTERVAL = 60 * 60 * 1000;  // 1 hour

  async createBackup(ctx: DurableObjectState, env: Env): Promise<void> {
    const snapshot = await this.createSnapshot(ctx);

    // Save to R2
    const key = `backups/${ctx.id}/${Date.now()}.json`;
    await env.BACKUP_BUCKET.put(key, JSON.stringify(snapshot));

    // Save to external storage
    await fetch("https://backup.example.com/store", {
      method: "POST",
      body: JSON.stringify(snapshot),
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${env.BACKUP_API_KEY}`
      }
    });

    // Cleanup old backups (keep last 24 hours)
    await this.cleanupOldBackups(ctx, env);
  }

  private async cleanupOldBackups(ctx: DurableObjectState, env: Env) {
    const cutoff = Date.now() - 24 * 60 * 60 * 1000;

    // List and delete old R2 backups
    const listed = await env.BACKUP_BUCKET.list({
      prefix: `backups/${ctx.id}/`
    });

    for (const object of listed.objects) {
      const timestamp = parseInt(object.key.split("/").pop()?.split(".")[0] || "0");
      if (timestamp < cutoff) {
        await env.BACKUP_BUCKET.delete(object.key);
      }
    }
  }

  async restoreFromBackup(backupKey: string, env: Env): Promise<object> {
    const object = await env.BACKUP_BUCKET.get(backupKey);
    if (!object) {
      throw new Error("Backup not found");
    }
    return await object.json();
  }
}
```

### 6.2 Failover Strategy

```typescript
// src/failover.ts
interface FailoverState {
  primaryRegion: string;
  secondaryRegions: string[];
  currentRegion: string;
  lastFailover?: number;
}

export class FailoverManager {
  private state: FailoverState;

  async checkHealth(region: string): Promise<boolean> {
    // Check regional health
    const response = await fetch(`https://health.${region}.example.com`);
    return response.ok;
  }

  async failover(toRegion: string): Promise<void> {
    console.log(`Failing over from ${this.state.currentRegion} to ${toRegion}`);

    // Update DNS/routing
    await fetch("https://dns.example.com/update", {
      method: "POST",
      body: JSON.stringify({ region: toRegion }),
      headers: { "Authorization": `Bearer ${process.env.DNS_API_KEY}` }
    });

    this.state.currentRegion = toRegion;
    this.state.lastFailover = Date.now();
  }

  async autoFailover(): Promise<void> {
    if (!await this.checkHealth(this.state.currentRegion)) {
      for (const region of this.state.secondaryRegions) {
        if (await this.checkHealth(region)) {
          await this.failover(region);
          return;
        }
      }
      throw new Error("No healthy regions available");
    }
  }
}
```

---

## 7. Cost Optimization

### 7.1 Hibernation Optimization

```typescript
// Enable hibernation to reduce costs for idle rooms
export class OptimizedServer extends Server {
  static options = { hibernate: true };

  // With hibernation:
  // - No CPU cost when idle
  // - Only pay for actual message processing
  // - WebSocket connections persist without DO active

  async onAlarm() {
    // If no connections, let DO be evicted
    if (this.getConnections().length === 0) {
      // Don't set another alarm - let DO sleep
      console.log("Room idle, allowing eviction");
    }
  }
}
```

### 7.2 Storage Optimization

```typescript
// Use efficient storage patterns
export class OptimizedStorageServer extends Server {
  // 1. Use soft deletes with periodic cleanup
  async softDeleteMessage(messageId: string) {
    this.ctx.storage.sql.exec(
      `UPDATE messages SET deleted_at = ? WHERE id = ?`,
      Date.now(), messageId
    );
  }

  async cleanupDeletedMessages() {
    const thirtyDaysAgo = Date.now() - 30 * 24 * 60 * 60 * 1000;
    this.ctx.storage.sql.exec(
      `DELETE FROM messages WHERE deleted_at < ?`,
      thirtyDaysAgo
    );
  }

  // 2. Use compression for large data
  async storeLargeData(key: string, data: Uint8Array) {
    const compressed = await compress(data);  // Use pako or similar
    await this.ctx.storage.put(key, compressed);
  }

  // 3. Archive old data to R2
  async archiveOldData() {
    const threshold = Date.now() - 7 * 24 * 60 * 60 * 1000;

    const oldData = this.ctx.storage.sql.exec(
      `SELECT * FROM messages WHERE created_at < ?`,
      threshold
    ).all();

    // Save to R2
    await this.env.ARCHIVE_BUCKET.put(
      `archive/${this.name}/${Date.now()}.json`,
      JSON.stringify(oldData)
    );

    // Delete from DO storage
    this.ctx.storage.sql.exec(
      `DELETE FROM messages WHERE created_at < ?`,
      threshold
    );
  }
}
```

### 7.3 Message Batching

```typescript
// Batch messages to reduce operation count
export class BatchedServer extends Server {
  private messageBuffer: Array<{ connectionId: string; message: string }> = [];
  private readonly BATCH_SIZE = 100;
  private readonly BATCH_INTERVAL = 100;  // ms

  scheduleFlush() {
    setTimeout(() => this.flushBuffer(), this.BATCH_INTERVAL);
  }

  queueBroadcast(message: string, excludeId?: string) {
    for (const conn of this.getConnections()) {
      if (!excludeId || conn.id !== excludeId) {
        this.messageBuffer.push({ connectionId: conn.id, message });
      }
    }

    if (this.messageBuffer.length >= this.BATCH_SIZE) {
      this.flushBuffer();
    } else {
      this.scheduleFlush();
    }
  }

  async flushBuffer() {
    if (this.messageBuffer.length === 0) return;

    // Send all buffered messages
    for (const { connectionId, message } of this.messageBuffer) {
      const conn = this.getConnection(connectionId);
      if (conn) {
        conn.send(message);
      }
    }

    this.messageBuffer = [];
  }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial production-grade guide created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
