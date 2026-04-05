# Production-Grade WebEditors Deployment

A comprehensive guide to deploying Tiptap and tldraw applications in production environments. This document covers everything from containerization to monitoring, security, and scaling collaboration.

---

## Table of Contents

### Part 1: Tiptap Production

1. [Application Architecture](#1-application-architecture)
2. [Deployment Strategies](#2-deployment-strategies)
3. [Scaling Collaboration](#3-scaling-collaboration)
4. [Database Operations](#4-database-operations)
5. [Monitoring](#5-monitoring)

### Part 2: tldraw Production

6. [Application Deployment](#6-application-deployment)
7. [Scaling Multiplayer](#7-scaling-multiplayer)
8. [Asset Management](#8-asset-management)

### Part 3: Common Topics

9. [Security](#9-security)
10. [CI/CD](#10-cicd)

---

## Part 1: Tiptap Production

### 1. Application Architecture

#### 1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Tiptap Production Architecture                        │
│                                                                              │
│  ┌─────────────┐                    ┌─────────────────────────────────┐     │
│  │   Clients   │◄─────HTTPS────────►│         Load Balancer           │     │
│  │  (Browser)  │                    │    (nginx / ALB / CloudFlare)   │     │
│  └─────────────┘                    └──────────────┬──────────────────┘     │
│                                                    │                          │
│           ┌────────────────────────────────────────┼────────────────────────┐│
│           │                                        │                         ││
│           ▼                                        ▼                         ││
│  ┌─────────────────┐                    ┌──────────────────┐                ││
│  │   Static CDN    │                    │  App Servers     │                ││
│  │   - Editor JS   │                    │  (Next.js/React) │                ││
│  │   - CSS Assets  │                    │  - SSR/SSG       │                ││
│  │   - Images      │                    │  - API Routes    │                ││
│  └─────────────────┘                    └────────┬─────────┘                ││
│                                                   │                          ││
│                                                   │ WebSocket                ││
│                                                   ▼                          ││
│                                          ┌──────────────────┐                ││
│                                          │ Hocuspocus       │                ││
│                                          │ Collaboration    │                ││
│                                          │ Servers          │                ││
│                                          └────────┬─────────┘                ││
│                                                   │                          ││
│                      ┌────────────────────────────┼─────────────────────┐   ││
│                      │                            │                     │   ││
│                      ▼                            ▼                     ▼   ││
│             ┌─────────────────┐         ┌─────────────────┐ ┌─────────────────┐│
│             │    PostgreSQL   │         │     Redis       │ │      S3/CDN     ││
│             │  - Documents    │         │  - Pub/Sub      │ │  - File Uploads ││
│             │  - Versions     │         │  - Presence     │ │  - Snapshots    ││
│             │  - Users        │         │  - Caching      │ │                 ││
│             └─────────────────┘         └─────────────────┘ └─────────────────┘│
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### 1.2 Component Responsibilities

| Component | Responsibility | Technology Options |
|-----------|---------------|-------------------|
| **Client** | Editor UI, local state, optimistic updates | React, Vue, Svelte |
| **CDN** | Static asset delivery, edge caching | CloudFlare, Fastly, AWS CloudFront |
| **App Servers** | SSR, API, authentication, business logic | Next.js, Express, Fastify |
| **Hocuspocus** | WebSocket server for Yjs sync | @hocuspocus/server |
| **PostgreSQL** | Document persistence, version history | PostgreSQL, Supabase |
| **Redis** | Pub/sub, presence, caching | Redis, Redis Cluster, AWS ElastiCache |
| **Object Storage** | File uploads, snapshots | AWS S3, GCP Cloud Storage |

#### 1.3 Data Flow

```
1. Client loads editor
   └─► Fetches initial document from API
   └─► Connects to Hocuspocus WebSocket

2. User types in editor
   └─► Yjs creates local update
   └─► Update sent via WebSocket to Hocuspocus
   └─► Hocuspocus broadcasts to other clients
   └─► Hocuspocus queues for persistence

3. Persistence (async)
   └─► Hocuspocus writes update to Redis
   └─► Background worker flushes to PostgreSQL
   └─► Periodic snapshots to S3
```

---

### 2. Deployment Strategies

#### 2.1 Docker Containers

**Base Dockerfile for Tiptap App (Next.js):**

```dockerfile
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/docker/Dockerfile.tiptap
FROM node:20-alpine AS base

# Install dependencies only when needed
FROM base AS deps
RUN apk add --no-cache libc6-compat
WORKDIR /app

COPY package.json yarn.lock* package-lock.json* pnpm-lock.yaml* ./
RUN \
  if [ -f yarn.lock ]; then yarn --frozen-lockfile; \
  elif [ -f package-lock.json ]; then npm ci; \
  elif [ -f pnpm-lock.yaml ]; then yarn global add pnpm && pnpm i --frozen-lockfile; \
  else echo "Lockfile not found." && exit 1; \
  fi

# Rebuild the source code only when needed
FROM base AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .

ENV NEXT_TELEMETRY_DISABLED 1
ENV NODE_ENV production

RUN \
  if [ -f yarn.lock ]; then yarn run build; \
  elif [ -f package-lock.json ]; then npm run build; \
  elif [ -f pnpm-lock.yaml ]; then pnpm run build; \
  else echo "Lockfile not found." && exit 1; \
  fi

# Production image, copy all the files and run next
FROM base AS runner
WORKDIR /app

ENV NODE_ENV production
ENV NEXT_TELEMETRY_DISABLED 1

RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs

COPY --from=builder /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static

USER nextjs

EXPOSE 3000

ENV PORT 3000
ENV HOSTNAME "0.0.0.0"

CMD ["node", "server.js"]
```

**Hocuspocus Server Dockerfile:**

```dockerfile
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/docker/Dockerfile.hocuspocus
FROM node:20-alpine AS base

WORKDIR /app

# Install dependencies
COPY package.json yarn.lock* package-lock.json* pnpm-lock.yaml* ./
RUN \
  if [ -f yarn.lock ]; then yarn --frozen-lockfile --production; \
  elif [ -f package-lock.json ]; then npm ci --production; \
  elif [ -f pnpm-lock.yaml ]; then yarn global add pnpm && pnpm i --frozen-lockfile --production; \
  else echo "Lockfile not found." && exit 1; \
  fi

# Copy source
COPY . .

RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 hocuspocus
USER hocuspocus

EXPOSE 4001

ENV PORT 4001
ENV HOST 0.0.0.0

CMD ["node", "dist/server.js"]
```

**Docker Compose for Local Development:**

```yaml
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/docker/docker-compose.yml
version: '3.8'

services:
  # Tiptap Frontend (Next.js)
  tiptap-app:
    build:
      context: ../apps/tiptap-app
      dockerfile: ../../docker/Dockerfile.tiptap
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=production
      - HOCUSPOCUS_URL=ws://hocuspocus:4001
      - DATABASE_URL=postgresql://postgres:postgres@postgres:5432/tiptap
      - REDIS_URL=redis://redis:6379
    depends_on:
      - hocuspocus
      - postgres
      - redis

  # Hocuspocus Collaboration Server
  hocuspocus:
    build:
      context: ../apps/hocuspocus-server
      dockerfile: ../../docker/Dockerfile.hocuspocus
    ports:
      - "4001:4001"
    environment:
      - PORT=4001
      - HOST=0.0.0.0
      - DATABASE_URL=postgresql://postgres:postgres@postgres:5432/tiptap
      - REDIS_URL=redis://redis:6379
      - SECRET_KEY=your-secret-key-change-in-production
    depends_on:
      - postgres
      - redis
    restart: unless-stopped

  # PostgreSQL Database
  postgres:
    image: postgres:15-alpine
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    environment:
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=postgres
      - POSTGRES_DB=tiptap
    ports:
      - "5432:5432"

  # Redis for Pub/Sub and Caching
  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis_data:/data
    ports:
      - "6379:6379"

  # MinIO for S3-compatible local storage
  minio:
    image: minio/minio
    command: server /data --console-address ":9001"
    volumes:
      - minio_data:/data
    environment:
      - MINIO_ROOT_USER=minioadmin
      - MINIO_ROOT_PASSWORD=minioadmin
    ports:
      - "9000:9000"
      - "9001:9001"

volumes:
  postgres_data:
  redis_data:
  minio_data:
```

#### 2.2 Kubernetes Deployments

**Namespace and ConfigMap:**

```yaml
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: webeditors
  labels:
    app: webeditors
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: webeditors-config
  namespace: webeditors
data:
  NODE_ENV: "production"
  LOG_LEVEL: "info"
  REDIS_HOST: "redis-master.redis.svc.cluster.local"
  REDIS_PORT: "6379"
  POSTGRES_HOST: "postgres.postgresql.svc.cluster.local"
  POSTGRES_PORT: "5432"
  POSTGRES_DB: "tiptap"
  HOCUSPOCUS_PORT: "4001"
  EDITOR_PORT: "3000"
```

**Hocuspocus Deployment:**

```yaml
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/k8s/hocuspocus-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: hocuspocus
  namespace: webeditors
  labels:
    app: hocuspocus
spec:
  replicas: 3
  selector:
    matchLabels:
      app: hocuspocus
  template:
    metadata:
      labels:
        app: hocuspocus
    spec:
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchLabels:
                  app: hocuspocus
              topologyKey: kubernetes.io/hostname
      containers:
      - name: hocuspocus
        image: webeditors/hocuspocus:latest
        ports:
        - containerPort: 4001
          name: websocket
        env:
        - name: PORT
          value: "4001"
        - name: HOST
          value: "0.0.0.0"
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: webeditors-secrets
              key: redis-url
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: webeditors-secrets
              key: database-url
        - name: SECRET_KEY
          valueFrom:
            secretKeyRef:
              name: webeditors-secrets
              key: secret-key
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          websocket:
            port: 4001
            path: /health
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          websocket:
            port: 4001
            path: /health
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: hocuspocus
  namespace: webeditors
spec:
  selector:
    app: hocuspocus
  ports:
  - port: 4001
    targetPort: 4001
    name: websocket
  type: ClusterIP
```

**Tiptap App Deployment:**

```yaml
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/k8s/tiptap-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: tiptap-app
  namespace: webeditors
  labels:
    app: tiptap-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: tiptap-app
  template:
    metadata:
      labels:
        app: tiptap-app
    spec:
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchLabels:
                  app: tiptap-app
              topologyKey: kubernetes.io/hostname
      containers:
      - name: tiptap-app
        image: webeditors/tiptap-app:latest
        ports:
        - containerPort: 3000
          name: http
        env:
        - name: NODE_ENV
          value: "production"
        - name: PORT
          value: "3000"
        - name: HOCUSPOCUS_URL
          value: "ws://hocuspocus:4001"
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: webeditors-secrets
              key: redis-url
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: webeditors-secrets
              key: database-url
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: tiptap-app
  namespace: webeditors
spec:
  selector:
    app: tiptap-app
  ports:
  - port: 3000
    targetPort: 3000
    name: http
  type: ClusterIP
```

**Ingress Configuration:**

```yaml
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: webeditors-ingress
  namespace: webeditors
  annotations:
    kubernetes.io/ingress.class: nginx
    nginx.ingress.kubernetes.io/websocket-services: "hocuspocus"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  tls:
  - hosts:
    - editor.example.com
    secretName: webeditors-tls
  rules:
  - host: editor.example.com
    http:
      paths:
      - path: /ws
        pathType: Prefix
        backend:
          service:
            name: hocuspocus
            port:
              number: 4001
      - path: /
        pathType: Prefix
        backend:
          service:
            name: tiptap-app
            port:
              number: 3000
```

**Horizontal Pod Autoscaler:**

```yaml
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/k8s/hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: hocuspocus-hpa
  namespace: webeditors
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: hocuspocus
  minReplicas: 3
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 10
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 100
        periodSeconds: 60
      - type: Pods
        value: 4
        periodSeconds: 60
      selectPolicy: Max
```

#### 2.3 Cloud Deployments

**AWS ECS/Fargate:**

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/cloud/aws/ecs-stack.ts
import * as cdk from 'aws-cdk-lib';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as elbv2 from 'aws-cdk-lib/aws-elasticloadbalancingv2';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import { Construct } from 'constructs';

export class HocuspocusEcsStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // VPC
    const vpc = new ec2.Vpc(this, 'HocuspocusVpc', {
      maxAzs: 3,
      natGateways: 1,
    });

    // Cluster
    const cluster = new ecs.Cluster(this, 'HocuspocusCluster', {
      vpc,
      enableFargateCapacityProviders: true,
    });

    // Secrets
    const secret = new secretsmanager.Secret(this, 'HocuspocusSecrets', {
      secretObjectValue: {
        'DATABASE_URL': secretsmanager.SecretValue.unsafePlainText('postgresql://...'),
        'REDIS_URL': secretsmanager.SecretValue.unsafePlainText('redis://...'),
        'SECRET_KEY': secretsmanager.SecretValue.unsafePlainText('change-me'),
      },
    });

    // Task Definition
    const taskDefinition = new ecs.FargateTaskDefinition(this, 'HocuspocusTask', {
      memoryLimitMiB: 1024,
      cpu: 512,
    });

    const container = taskDefinition.addContainer('HocuspocusContainer', {
      image: ecs.ContainerImage.fromRegistry('webeditors/hocuspocus:latest'),
      portMappings: [{ containerPort: 4001, protocol: ecs.Protocol.TCP }],
      logging: ecs.LogDriver.awsLogs({
        streamPrefix: 'hocuspocus',
        logRetention: logs.RetentionDays.THIRTY_DAYS,
      }),
      secrets: {
        'DATABASE_URL': ecs.Secret.fromSecretsManager(secret, 'DATABASE_URL'),
        'REDIS_URL': ecs.Secret.fromSecretsManager(secret, 'REDIS_URL'),
        'SECRET_KEY': ecs.Secret.fromSecretsManager(secret, 'SECRET_KEY'),
      },
      environment: {
        PORT: '4001',
        HOST: '0.0.0.0',
      },
    });

    // Service
    const service = new ecs.FargateService(this, 'HocuspocusService', {
      cluster,
      taskDefinition,
      desiredCount: 3,
      minHealthyPercent: 50,
      maxHealthyPercent: 200,
      enableExecuteCommand: true,
    });

    // Load Balancer
    const lb = new elbv2.ApplicationLoadBalancer(this, 'HocuspocusLB', {
      vpc,
      internetFacing: true,
    });

    const listener = lb.addListener('Listener', { port: 443, protocol: elbv2.ApplicationProtocol.HTTPS });

    listener.addTargets('HocuspocusTarget', {
      port: 4001,
      protocol: elbv2.ApplicationProtocol.HTTP,
      targets: [service],
      healthCheck: {
        path: '/health',
        port: '4001',
        protocol: elbv2.Protocol.HTTP,
        interval: cdk.Duration.seconds(30),
        timeout: cdk.Duration.seconds(5),
      },
      stickinessCookieDuration: cdk.Duration.days(1),
    });
  }
}
```

**Vercel Deployment (Tiptap App):**

```json
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/cloud/vercel/vercel.json
{
  "version": 2,
  "name": "tiptap-app",
  "builds": [
    {
      "src": "package.json",
      "use": "@vercel/node"
    }
  ],
  "env": {
    "HOCUSPOCUS_URL": "@hocuspocus-url",
    "DATABASE_URL": "@database-url",
    "REDIS_URL": "@redis-url"
  },
  "regions": ["iad1", "sfo1", "fra1"],
  "trailingSlash": true
}
```

**Cloudflare Workers (Edge Deployment):**

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/cloud/cloudflare/worker.ts
export interface Env {
  HOCUSPOCUS_URL: string;
  REDIS_KV: KVNamespace;
}

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);

    // Handle WebSocket upgrade
    if (url.pathname === '/ws' && request.headers.get('Upgrade') === 'websocket') {
      return handleWebSocket(request, env);
    }

    // Proxy to Hocuspocus
    if (url.pathname.startsWith('/ws/')) {
      const hocuspocusUrl = new URL(env.HOCUSPOCUS_URL);
      hocuspocusUrl.pathname = url.pathname;
      hocuspocusUrl.search = url.search;

      const hocuspocusRequest = new Request(hocuspocusUrl, {
        method: request.method,
        headers: request.headers,
        body: request.body,
      });

      return fetch(hocuspocusRequest);
    }

    // Static assets from KV
    const asset = await env.REDIS_KV.get(url.pathname);
    if (asset) {
      return new Response(asset, {
        headers: { 'Content-Type': 'text/html' },
      });
    }

    return new Response('Not found', { status: 404 });
  },
};

async function handleWebSocket(request: Request, env: Env): Promise<Response> {
  // WebSocket handling for edge
  const pair = new WebSocketPair();
  const [client, server] = Object.values(pair);

  // Connect to Hocuspocus
  const hocuspocusWs = new WebSocket(env.HOCUSPOCUS_URL);

  hocuspocusWs.addEventListener('message', (event) => {
    server.send(event.data);
  });

  server.addEventListener('message', (event) => {
    hocuspocusWs.send(event.data);
  });

  return new Response(null, {
    status: 101,
    webSocket: client,
  });
}
```

---

### 3. Scaling Collaboration

#### 3.1 Hocuspocus Clustering

**Production Hocuspocus Server:**

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/apps/hocuspocus-server/src/server.ts
import { Hocuspocus, IncomingMessage, Connection, Document } from '@hocuspocus/server';
import { Redis } from 'ioredis';
import { Logger } from '@hocuspocus/logger';
import { Database } from '@hocuspocus/database';
import { PostgreSQL } from '@hocuspocus/database-postgresql';
import * as jwt from 'jsonwebtoken';
import { v4 as uuidv4 } from 'uuid';

interface RedisConfig {
  host: string;
  port: number;
  password?: string;
}

interface ServerConfig {
  port: number;
  host: string;
  secretKey: string;
  redis: RedisConfig;
  database: string;
  nodeId: string;
}

class ProductionHocuspocusServer {
  private server: Hocuspocus;
  private redis: Redis;
  private database: Database;
  private config: ServerConfig;

  constructor(config: ServerConfig) {
    this.config = config;
    this.redis = new Redis({
      host: config.redis.host,
      port: config.redis.port,
      password: config.redis.password,
      maxRetriesPerRequest: 3,
      retryStrategy: (times: number) => {
        return Math.min(times * 50, 2000);
      },
    });

    this.database = new PostgreSQL(config.database);
    
    this.server = new Hocuspocus({
      port: config.port,
      host: config.host,
      name: config.nodeId,
      extensions: [
        new Logger(),
        // Database extension for persistence
        {
          onStoreDocument: async ({ documentName, document }) => {
            await this.storeDocument(documentName, document);
          },
          onRetrieveDocument: async ({ documentName }) => {
            return await this.loadDocument(documentName);
          },
        },
      ],
      onConnect: async (data: { connection: Connection; requestHeaders: IncomingMessage['headers'] }) => {
        await this.handleConnect(data);
      },
      onDisconnect: async (data: { connection: Connection; documentName: string }) => {
        await this.handleDisconnect(data);
      },
      beforeBroadcastStateless: (data: { 
        document: Document; 
        connection: Connection; 
        payload: string 
      }) => {
        return this.beforeBroadcastStateless(data);
      },
    });

    this.setupPubSub();
    this.setupHealthCheck();
  }

  private async handleConnect(data: { 
    connection: Connection; 
    requestHeaders: IncomingMessage['headers'];
  }): Promise<void> {
    const token = data.requestHeaders.authorization?.replace('Bearer ', '');
    
    if (!token) {
      throw new Error('Unauthorized: No token provided');
    }

    try {
      const decoded = jwt.verify(token, this.config.secretKey) as {
        userId: string;
        documentId: string;
        permissions: string[];
      };

      // Store connection metadata
      data.connection.context = {
        userId: decoded.userId,
        documentId: decoded.documentId,
        permissions: decoded.permissions,
      };

      // Track connection in Redis
      await this.redis.sadd(`document:${decoded.documentId}:connections`, data.connection.connectionId);
      await this.redis.setex(
        `connection:${data.connection.connectionId}`,
        3600,
        JSON.stringify({
          userId: decoded.userId,
          documentId: decoded.documentId,
          connectedAt: Date.now(),
          nodeId: this.config.nodeId,
        })
      );

      // Publish connection event
      await this.redis.publish('connections:update', JSON.stringify({
        event: 'connect',
        documentId: decoded.documentId,
        connectionId: data.connection.connectionId,
        userId: decoded.userId,
        nodeId: this.config.nodeId,
      }));

    } catch (error) {
      throw new Error('Unauthorized: Invalid token');
    }
  }

  private async handleDisconnect(data: { 
    connection: Connection; 
    documentName: string;
  }): Promise<void> {
    const context = data.connection.context as { userId: string; documentId: string } | undefined;
    
    if (context?.documentId) {
      await this.redis.srem(`document:${context.documentId}:connections`, data.connection.connectionId);
      await this.redis.del(`connection:${data.connection.connectionId}`);

      await this.redis.publish('connections:update', JSON.stringify({
        event: 'disconnect',
        documentId: context.documentId,
        connectionId: data.connection.connectionId,
        userId: context.userId,
        nodeId: this.config.nodeId,
      }));
    }
  }

  private async storeDocument(documentName: string, document: Document): Promise<void> {
    const state = document.getBuffer();
    
    // Write to Redis (fast, with TTL)
    await this.redis.setex(
      `document:${documentName}:state`,
      3600,
      Buffer.from(state).toString('base64')
    );

    // Queue for database persistence
    await this.redis.lpush('document:write-queue', JSON.stringify({
      documentName,
      state: Array.from(state),
      timestamp: Date.now(),
    }));

    // Publish update event
    await this.redis.publish('documents:update', JSON.stringify({
      documentName,
      timestamp: Date.now(),
      nodeId: this.config.nodeId,
    }));
  }

  private async loadDocument(documentName: string): Promise<Uint8Array | null> {
    // Try Redis cache first
    const cached = await this.redis.get(`document:${documentName}:state`);
    if (cached) {
      return Buffer.from(cached, 'base64');
    }

    // Fall back to database
    const stored = await this.database.getDocument(documentName);
    if (stored) {
      // Warm cache
      await this.redis.setex(
        `document:${documentName}:state`,
        3600,
        Buffer.from(stored).toString('base64')
      );
      return stored;
    }

    return null;
  }

  private setupPubSub(): void {
    const subscriber = this.redis.duplicate();
    
    subscriber.subscribe('documents:update', (message) => {
      const data = JSON.parse(message);
      // Handle cross-node document updates
      this.invalidateCache(data.documentName);
    });

    subscriber.subscribe('connections:update', (message) => {
      const data = JSON.parse(message);
      // Handle connection state changes
      this.updatePresence(data);
    });
  }

  private setupHealthCheck(): void {
    // Heartbeat for this node
    setInterval(async () => {
      const stats = await this.getNodeStats();
      await this.redis.setex(
        `node:${this.config.nodeId}:heartbeat`,
        30,
        JSON.stringify({
          ...stats,
          timestamp: Date.now(),
        })
      );
    }, 10000);
  }

  private async getNodeStats() {
    const connections = await this.server.getConnectionsCount();
    const documents = await this.server.getDocumentsCount();
    
    return {
      connections,
      documents,
      uptime: process.uptime(),
      memory: process.memoryUsage(),
    };
  }

  private invalidateCache(documentName: string): void {
    // Invalidate local cache when document is updated from another node
    this.server.documents.delete(documentName);
  }

  private updatePresence(data: any): void {
    // Update awareness/presence across nodes
    // Implementation depends on your presence system
  }

  private beforeBroadcastStateless(data: { 
    document: Document; 
    connection: Connection; 
    payload: string 
  }): void {
    // Rate limiting for stateless messages
    const now = Date.now();
    const lastSend = (data.connection as any)._lastStatelessSend || 0;
    
    if (now - lastSend < 100) {
      // Throttle: skip if sending too fast
      return;
    }
    
    (data.connection as any)._lastStatelessSend = now;
  }

  public async start(): Promise<void> {
    await this.server.listen();
    console.log(`Hocuspocus server listening on ${this.config.host}:${this.config.port}`);
  }

  public async stop(): Promise<void> {
    await this.server.destroy();
    await this.redis.quit();
  }
}

// Start server
const config: ServerConfig = {
  port: parseInt(process.env.PORT || '4001'),
  host: process.env.HOST || '0.0.0.0',
  secretKey: process.env.SECRET_KEY || 'change-me-in-production',
  redis: {
    host: process.env.REDIS_HOST || 'localhost',
    port: parseInt(process.env.REDIS_PORT || '6379'),
    password: process.env.REDIS_PASSWORD,
  },
  database: process.env.DATABASE_URL || 'postgresql://localhost/tiptap',
  nodeId: process.env.NODE_ID || uuidv4(),
};

const server = new ProductionHocuspocusServer(config);
server.start().catch(console.error);

process.on('SIGTERM', async () => {
  await server.stop();
  process.exit(0);
});

process.on('SIGINT', async () => {
  await server.stop();
  process.exit(0);
});
```

#### 3.2 Redis Pub/Sub Configuration

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/apps/hocuspocus-server/src/redis-pubsub.ts
import { Redis, Cluster } from 'ioredis';

interface PubSubConfig {
  nodes: Array<{ host: string; port: number }>;
  password?: string;
}

export class DistributedPubSub {
  private pubClient: Redis | Cluster;
  private subClient: Redis | Cluster;
  private handlers: Map<string, Set<(data: any) => void>> = new Map();

  constructor(config: PubSubConfig) {
    // Use cluster for production
    if (config.nodes.length > 1) {
      this.pubClient = new Cluster(config.nodes, {
        redisOptions: { password: config.password },
        scaleReads: 'slave',
      });
      this.subClient = new Cluster(config.nodes, {
        redisOptions: { password: config.password },
      });
    } else {
      this.pubClient = new Redis({
        host: config.nodes[0].host,
        port: config.nodes[0].port,
        password: config.password,
      });
      this.subClient = this.pubClient.duplicate();
    }

    this.setupListeners();
  }

  private setupListeners(): void {
    const subscriber = this.subClient as any;
    
    subscriber.on('message', (channel: string, message: string) => {
      const handlers = this.handlers.get(channel);
      if (handlers) {
        const data = JSON.parse(message);
        handlers.forEach(handler => handler(data));
      }
    });
  }

  async subscribe(channel: string, handler: (data: any) => void): Promise<void> {
    if (!this.handlers.has(channel)) {
      this.handlers.set(channel, new Set());
      await this.subClient.subscribe(channel);
    }
    this.handlers.get(channel)!.add(handler);
  }

  async unsubscribe(channel: string, handler: (data: any) => void): Promise<void> {
    const handlers = this.handlers.get(channel);
    if (handlers) {
      handlers.delete(handler);
      if (handlers.size === 0) {
        this.handlers.delete(channel);
        await this.subClient.unsubscribe(channel);
      }
    }
  }

  async publish(channel: string, data: any): Promise<number> {
    return this.pubClient.publish(channel, JSON.stringify(data));
  }

  async broadcast(channels: string[], data: any): Promise<void> {
    await Promise.all(channels.map(ch => this.publish(ch, data)));
  }

  async close(): Promise<void> {
    await this.pubClient.quit();
    await this.subClient.quit();
  }
}
```

#### 3.3 Load Balancing with Sticky Sessions

**Nginx Configuration:**

```nginx
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/nginx/nginx.conf
worker_processes auto;
worker_rlimit_nofile 65535;

events {
    use epoll;
    worker_connections 4096;
    multi_accept on;
}

http {
    include /etc/nginx/mime.types;
    default_type application/octet-stream;

    # Logging
    log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                    '$status $body_bytes_sent "$http_referer" '
                    '"$http_user_agent" "$http_x_forwarded_for" '
                    'rt=$request_time uct="$upstream_connect_time" '
                    'uht="$upstream_header_time" urt="$upstream_response_time"';

    access_log /var/log/nginx/access.log main;
    error_log /var/log/nginx/error.log warn;

    # Performance
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;
    types_hash_max_size 2048;
    client_max_body_size 100M;

    # Gzip
    gzip on;
    gzip_vary on;
    gzip_proxied any;
    gzip_comp_level 6;
    gzip_types text/plain text/css text/xml application/json application/javascript 
               application/xml application/xml+rss text/javascript;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api_limit:10m rate=10r/s;
    limit_conn_zone $binary_remote_addr zone=conn_limit:10m;

    # Upstream for Tiptap app
    upstream tiptap_app {
        least_conn;
        server tiptap-app-1:3000;
        server tiptap-app-2:3000;
        server tiptap-app-3:3000;
        keepalive 32;
    }

    # Upstream for Hocuspocus with sticky sessions
    upstream hocuspocus {
        ip_hash;  # Sticky sessions based on client IP
        server hocuspocus-1:4001;
        server hocuspocus-2:4001;
        server hocuspocus-3:4001;
        keepalive 1024;
    }

    # Alternative: Cookie-based sticky sessions
    upstream hocuspocus_cookie {
        hash $cookie_hocuspocus_node consistent;
        server hocuspocus-1:4001;
        server hocuspocus-2:4001;
        server hocuspocus-3:4001;
    }

    server {
        listen 80;
        listen [::]:80;
        server_name editor.example.com;

        # Redirect HTTP to HTTPS
        return 301 https://$server_name$request_uri;
    }

    server {
        listen 443 ssl http2;
        listen [::]:443 ssl http2;
        server_name editor.example.com;

        # SSL
        ssl_certificate /etc/ssl/certs/editor.example.com.crt;
        ssl_certificate_key /etc/ssl/private/editor.example.com.key;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;
        ssl_prefer_server_ciphers off;
        ssl_session_cache shared:SSL:10m;
        ssl_session_timeout 1d;

        # Security headers
        add_header X-Frame-Options "SAMEORIGIN" always;
        add_header X-Content-Type-Options "nosniff" always;
        add_header X-XSS-Protection "1; mode=block" always;
        add_header Referrer-Policy "strict-origin-when-cross-origin" always;
        add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline';" always;

        # WebSocket proxy for Hocuspocus
        location /ws {
            proxy_pass http://hocuspocus;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            
            # WebSocket timeouts
            proxy_read_timeout 86400s;
            proxy_send_timeout 86400s;
            proxy_buffering off;
        }

        # API routes with rate limiting
        location /api/ {
            limit_req zone=api_limit burst=20 nodelay;
            limit_conn conn_limit 10;
            
            proxy_pass http://tiptap_app;
            proxy_http_version 1.1;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # Main app
        location / {
            proxy_pass http://tiptap_app;
            proxy_http_version 1.1;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            
            # Caching
            proxy_cache_valid 200 1m;
            proxy_cache_valid 404 1m;
        }

        # Health check endpoint
        location /health {
            access_log off;
            return 200 "healthy\n";
            add_header Content-Type text/plain;
        }
    }
}
```

**HAProxy Configuration:**

```haproxy
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/haproxy/haproxy.cfg
global
    log /dev/log local0
    log /dev/log local1 notice
    chroot /var/lib/haproxy
    stats socket /run/haproxy/admin.sock mode 660 level admin
    stats timeout 30s
    user haproxy
    group haproxy
    daemon
    
    # Performance
    maxconn 4096
    tune.ssl.default-dh-param 2048

defaults
    log global
    mode http
    option httplog
    option dontlognull
    option http-server-close
    option forwardfor except 127.0.0.0/8
    option redispatch
    retries 3
    timeout connect 5s
    timeout client 50s
    timeout server 50s
    timeout tunnel 86400s
    errorfile 400 /etc/haproxy/errors/400.http
    errorfile 403 /etc/haproxy/errors/403.http
    errorfile 408 /etc/haproxy/errors/408.http
    errorfile 500 /etc/haproxy/errors/500.http
    errorfile 502 /etc/haproxy/errors/502.http
    errorfile 503 /etc/haproxy/errors/503.http
    errorfile 504 /etc/haproxy/errors/504.http

frontend https_front
    bind *:443 ssl crt /etc/haproxy/certs/editor.example.com.pem
    http-request set-header X-Forwarded-Proto https
    
    # ACLs
    acl is_ws path_beg /ws
    acl is_api path_beg /api/
    
    # Routing
    use_backend hocuspocus_ws if is_ws
    use_backend api_backend if is_api
    default_backend tiptap_app

backend hocuspocus_ws
    balance source  # Sticky sessions
    http-request set-header X-Forwarded-Proto https
    option httpchk GET /health
    server hocuspocus1 hocuspocus-1:4001 check inter 5s fall 3 rise 2 cookie node1
    server hocuspocus2 hocuspocus-2:4001 check inter 5s fall 3 rise 2 cookie node2
    server hocuspocus3 hocuspocus-3:4001 check inter 5s fall 3 rise 2 cookie node3

backend api_backend
    balance roundrobin
    http-request set-header X-Forwarded-Proto https
    stick-table type ip size 100k expire 30s store http_req_rate(10s)
    http-request track-sc0 src
    http-request deny deny_status 429 if { sc_http_req_rate(0) gt 100 }
    option httpchk GET /health
    server tiptap1 tiptap-app-1:3000 check inter 5s fall 3 rise 2
    server tiptap2 tiptap-app-2:3000 check inter 5s fall 3 rise 2
    server tiptap3 tiptap-app-3:3000 check inter 5s fall 3 rise 2

backend tiptap_app
    balance roundrobin
    http-request set-header X-Forwarded-Proto https
    option httpchk GET /health
    server tiptap1 tiptap-app-1:3000 check inter 5s fall 3 rise 2
    server tiptap2 tiptap-app-2:3000 check inter 5s fall 3 rise 2
    server tiptap3 tiptap-app-3:3000 check inter 5s fall 3 rise 2

listen stats
    bind *:8404
    stats enable
    stats uri /stats
    stats refresh 10s
    stats admin if TRUE
```

#### 3.4 Room Sharding

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/apps/hocuspocus-server/src/room-sharding.ts
import { createHash } from 'crypto';
import { Redis } from 'ioredis';

interface Node {
  id: string;
  address: string;
  port: number;
  weight: number;
  connections: number;
  documents: number;
}

export class ConsistentHashRing {
  private nodes: Node[] = [];
  private virtualNodes: number = 150;
  private ring: Map<number, Node> = new Map();

  addNode(node: Node): void {
    this.nodes.push(node);
    for (let i = 0; i < this.virtualNodes; i++) {
      const hash = this.hash(`${node.id}:${i}`);
      this.ring.set(hash, node);
    }
  }

  removeNode(nodeId: string): void {
    this.nodes = this.nodes.filter(n => n.id !== nodeId);
    for (let i = 0; i < this.virtualNodes; i++) {
      const hash = this.hash(`${nodeId}:${i}`);
      this.ring.delete(hash);
    }
  }

  getNode(key: string): Node | null {
    if (this.ring.size === 0) return null;

    const hash = this.hash(key);
    const hashes = Array.from(this.ring.keys()).sort((a, b) => a - b);
    
    // Find first node >= hash
    for (const h of hashes) {
      if (h >= hash) {
        return this.ring.get(h)!;
      }
    }
    
    // Wrap around
    return this.ring.get(hashes[0])!;
  }

  private hash(key: string): number {
    return parseInt(createHash('md5').update(key).digest('hex').substring(0, 8), 16);
  }

  getAllNodes(): Node[] {
    return this.nodes;
  }
}

export class RoomShardManager {
  private redis: Redis;
  private hashRing: ConsistentHashRing;
  private nodeId: string;
  private localRooms: Map<string, any> = new Map();

  constructor(redis: Redis, nodeId: string, nodeAddress: string, nodePort: number) {
    this.redis = redis;
    this.nodeId = nodeId;
    this.hashRing = new ConsistentHashRing();
    
    // Add self to hash ring
    this.hashRing.addNode({
      id: nodeId,
      address: nodeAddress,
      port: nodePort,
      weight: 1,
      connections: 0,
      documents: 0,
    });

    this.startDiscovery();
  }

  private async startDiscovery(): Promise<void> {
    // Discover other nodes
    await this.discoverNodes();
    
    // Refresh node list periodically
    setInterval(() => this.discoverNodes(), 10000);
    
    // Heartbeat
    setInterval(() => this.heartbeat(), 5000);
  }

  private async discoverNodes(): Promise<void> {
    const nodeKeys = await this.redis.keys('node:*:heartbeat');
    
    for (const key of nodeKeys) {
      const data = await this.redis.get(key);
      if (data) {
        const node = JSON.parse(data);
        const nodeId = key.split(':')[1];
        
        if (nodeId !== this.nodeId) {
          // Parse address from heartbeat data or separate registration
          this.hashRing.addNode({
            id: nodeId,
            address: node.address || 'unknown',
            port: node.port || 4001,
            weight: 1,
            connections: node.connections || 0,
            documents: node.documents || 0,
          });
        }
      }
    }
  }

  private async heartbeat(): Promise<void> {
    const stats = {
      connections: this.localRooms.size,
      documents: this.localRooms.size,
      address: 'hocuspocus',
      port: 4001,
      timestamp: Date.now(),
    };

    await this.redis.setex(
      `node:${this.nodeId}:heartbeat`,
      15,
      JSON.stringify(stats)
    );
  }

  async getRoomNode(roomId: string): Promise<Node | null> {
    // Check if room is cached locally
    const cachedNode = await this.redis.get(`room:${roomId}:node`);
    if (cachedNode) {
      const node = this.hashRing.getNode(cachedNode);
      if (node && node.id === cachedNode) {
        return node;
      }
    }

    // Use consistent hashing
    const node = this.hashRing.getNode(roomId);
    
    if (node) {
      // Cache the mapping
      await this.redis.setex(`room:${roomId}:node`, 3600, node.id);
    }

    return node;
  }

  async registerRoom(roomId: string): Promise<void> {
    this.localRooms.set(roomId, { createdAt: Date.now() });
    
    // Update room count
    await this.redis.setex(`room:${roomId}:node`, 3600, this.nodeId);
  }

  async unregisterRoom(roomId: string): Promise<void> {
    this.localRooms.delete(roomId);
    await this.redis.del(`room:${roomId}:node`);
  }

  async migrateRoom(roomId: string, targetNode: Node): Promise<void> {
    // Implementation for live room migration
    const room = this.localRooms.get(roomId);
    if (!room) return;

    // Serialize room state
    const state = room.serialize();
    
    // Send to target node
    await this.redis.publish('room:migrate', JSON.stringify({
      roomId,
      targetNode: targetNode.id,
      state,
    }));

    this.localRooms.delete(roomId);
  }

  async rebalance(): Promise<void> {
    // Check if rebalancing is needed
    const nodes = this.hashRing.getAllNodes();
    const avgLoad = nodes.reduce((sum, n) => sum + n.connections, 0) / nodes.length;
    
    for (const node of nodes) {
      if (node.id === this.nodeId) continue;
      
      // If this node is overloaded
      if (node.connections > avgLoad * 1.5) {
        // Find rooms to migrate
        const roomsToMove = Math.floor((node.connections - avgLoad) / 2);
        
        // Move rooms to less loaded nodes
        for (let i = 0; i < roomsToMove; i++) {
          const roomId = Array.from(this.localRooms.keys())[i];
          const targetNode = this.findLeastLoadedNode();
          if (targetNode && targetNode.id !== this.nodeId) {
            await this.migrateRoom(roomId, targetNode);
          }
        }
      }
    }
  }

  private findLeastLoadedNode(): Node | null {
    const nodes = this.hashRing.getAllNodes();
    return nodes.reduce((min, node) => 
      node.connections < min.connections ? node : min
    );
  }
}
```

---

### 4. Database Operations

#### 4.1 PostgreSQL Schema

```sql
-- /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/database/schema.sql

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255),
    avatar_url TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Documents table
CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    owner_id UUID REFERENCES users(id) ON DELETE CASCADE,
    content JSONB NOT NULL DEFAULT '[]'::jsonb,
    content_hash VARCHAR(64),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_sync_at TIMESTAMP WITH TIME ZONE,
    is_deleted BOOLEAN DEFAULT FALSE,
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Document versions for history
CREATE TABLE document_versions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    content JSONB NOT NULL,
    content_hash VARCHAR(64) NOT NULL,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    change_summary TEXT,
    
    UNIQUE(document_id, version)
);

-- Document collaborators
CREATE TABLE document_collaborators (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    permission VARCHAR(20) NOT NULL CHECK (permission IN ('read', 'write', 'admin')),
    granted_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    granted_by UUID REFERENCES users(id),
    
    UNIQUE(document_id, user_id)
);

-- Yjs updates queue for async persistence
CREATE TABLE yjs_updates_queue (
    id BIGSERIAL PRIMARY KEY,
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
    update_data BYTEA NOT NULL,
    version_vec JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    processed BOOLEAN DEFAULT FALSE,
    processed_at TIMESTAMP WITH TIME ZONE
);

-- Document snapshots for backup
CREATE TABLE document_snapshots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
    snapshot_data BYTEA NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    s3_key VARCHAR(512),
    s3_bucket VARCHAR(255)
);

-- Connection logs for audit
CREATE TABLE connection_logs (
    id BIGSERIAL PRIMARY KEY,
    document_id UUID REFERENCES documents(id),
    user_id UUID REFERENCES users(id),
    node_id VARCHAR(255),
    connection_id VARCHAR(255),
    connected_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    disconnected_at TIMESTAMP WITH TIME ZONE,
    duration_ms INTEGER
);

-- Indexes for performance
CREATE INDEX idx_documents_owner ON documents(owner_id);
CREATE INDEX idx_documents_updated ON documents(updated_at DESC);
CREATE INDEX idx_documents_last_sync ON documents(last_sync_at DESC);
CREATE INDEX idx_document_versions_document ON document_versions(document_id, version DESC);
CREATE INDEX idx_document_collaborators_document ON document_collaborators(document_id);
CREATE INDEX idx_document_collaborators_user ON document_collaborators(user_id);
CREATE INDEX idx_yjs_updates_queue_unprocessed ON yjs_updates_queue(document_id, processed) 
    WHERE NOT processed;
CREATE INDEX idx_document_snapshots_document ON document_snapshots(document_id, created_at DESC);
CREATE INDEX idx_connection_logs_document ON connection_logs(document_id, connected_at DESC);

-- Functions
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers
CREATE TRIGGER update_documents_updated_at
    BEFORE UPDATE ON documents
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Full-text search on document names
CREATE INDEX idx_documents_name_search ON documents USING gin(to_tsvector('english', name));
```

#### 4.2 Database Connection Pool

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/database/pool.ts
import { Pool, PoolConfig, QueryResult } from 'pg';
import * as slonik from 'slonik';

interface DatabaseConfig extends PoolConfig {
  maxConnections: number;
  idleTimeout: number;
}

export class DatabasePool {
  private pool: Pool;
  private config: DatabaseConfig;

  constructor(config: DatabaseConfig) {
    this.config = config;
    this.pool = new Pool({
      ...config,
      max: config.maxConnections,
      idleTimeoutMillis: config.idleTimeout,
      connectionTimeoutMillis: 10000,
    });

    this.setupEventHandlers();
  }

  private setupEventHandlers(): void {
    this.pool.on('error', (err: Error, client: any) => {
      console.error('Unexpected error on idle client', err);
    });

    this.pool.on('connect', (client: any) => {
      console.log('New client connected');
    });
  }

  async query<T = any>(
    text: string,
    params?: any[]
  ): Promise<QueryResult<T>> {
    const start = Date.now();
    
    try {
      const result = await this.pool.query<T>(text, params);
      const duration = Date.now() - start;
      
      if (duration > 1000) {
        console.warn('Slow query', { text, duration, params });
      }
      
      return result;
    } catch (error) {
      console.error('Database query error', { text, params, error });
      throw error;
    }
  }

  async withTransaction<T>(fn: (pool: Pool) => Promise<T>): Promise<T> {
    const client = await this.pool.connect();
    
    try {
      await client.query('BEGIN');
      const result = await fn(client);
      await client.query('COMMIT');
      return result;
    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }
  }

  async getStats(): Promise<{
    total: number;
    waiting: number;
    idle: number;
  }> {
    return {
      total: this.pool.totalCount,
      waiting: this.pool.waitingCount,
      idle: this.pool.idleCount,
    };
  }

  async close(): Promise<void> {
    await this.pool.end();
  }
}

// Slonik example for type-safe queries
export async function createSlonikPool(connectionString: string) {
  return slonik.createPool(connectionString, {
    connectionTimeout: 10000,
    acquireConnectionTimeout: 10000,
    idleTimeout: 30000,
    maximumPoolSize: 20,
    minimumPoolSize: 5,
    queryTimeout: 60000,
    interceptors: [
      // Logging interceptor
      {
        transformQuery: ({ sql, values }) => {
          console.log('Executing query:', sql, values);
          return { sql, values };
        },
      },
    ],
  });
}
```

#### 4.3 Document Repository

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/database/document-repository.ts
import { Pool } from 'pg';
import * as Y from 'yjs';

interface Document {
  id: string;
  name: string;
  ownerId: string;
  content: any;
  updatedAt: Date;
  lastSyncAt: Date | null;
}

interface DocumentVersion {
  id: string;
  documentId: string;
  version: number;
  content: any;
  createdBy: string | null;
  createdAt: Date;
  changeSummary: string | null;
}

export class DocumentRepository {
  private pool: Pool;

  constructor(pool: Pool) {
    this.pool = pool;
  }

  async create(data: {
    name: string;
    ownerId: string;
    content?: any;
  }): Promise<Document> {
    const result = await this.pool.query(
      `INSERT INTO documents (name, owner_id, content)
       VALUES ($1, $2, $3)
       RETURNING id, name, owner_id as "ownerId", content, 
                 created_at as "createdAt", updated_at as "updatedAt"`,
      [data.name, data.ownerId, JSON.stringify(data.content || [])]
    );
    
    return result.rows[0];
  }

  async findById(id: string): Promise<Document | null> {
    const result = await this.pool.query(
      `SELECT id, name, owner_id as "ownerId", content,
              created_at as "createdAt", updated_at as "updatedAt",
              last_sync_at as "lastSyncAt"
       FROM documents
       WHERE id = $1 AND is_deleted = false`,
      [id]
    );
    
    return result.rows[0] || null;
  }

  async updateContent(id: string, content: any): Promise<void> {
    await this.pool.query(
      `UPDATE documents
       SET content = $2, updated_at = CURRENT_TIMESTAMP,
           last_sync_at = CURRENT_TIMESTAMP
       WHERE id = $1`,
      [id, JSON.stringify(content)]
    );
  }

  async storeYjsUpdate(documentId: string, update: Uint8Array): Promise<void> {
    await this.pool.query(
      `INSERT INTO yjs_updates_queue (document_id, update_data, version_vec)
       VALUES ($1, $2, $3)`,
      [documentId, update, JSON.stringify({})]
    );
  }

  async processYjsUpdates(documentId: string): Promise<Uint8Array[]> {
    const client = await this.pool.connect();
    
    try {
      await client.query('BEGIN');
      
      const result = await client.query(
        `SELECT update_data
         FROM yjs_updates_queue
         WHERE document_id = $1 AND processed = false
         ORDER BY id
         FOR UPDATE SKIP LOCKED`,
        [documentId]
      );

      if (result.rows.length === 0) {
        await client.query('ROLLBACK');
        return [];
      }

      // Mark as processed
      await client.query(
        `UPDATE yjs_updates_queue
         SET processed = true, processed_at = CURRENT_TIMESTAMP
         WHERE id = ANY($1)`,
        [result.rows.map(r => r.id)]
      );

      await client.query('COMMIT');
      
      return result.rows.map(r => Buffer.from(r.update_data));
    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }
  }

  async createVersion(data: {
    documentId: string;
    version: number;
    content: any;
    createdBy: string | null;
    changeSummary?: string;
  }): Promise<DocumentVersion> {
    const result = await this.pool.query(
      `INSERT INTO document_versions 
       (document_id, version, content, content_hash, created_by, change_summary)
       VALUES ($1, $2, $3, $4, $5, $6)
       RETURNING *`,
      [
        data.documentId,
        data.version,
        JSON.stringify(data.content),
        this.hashContent(data.content),
        data.createdBy,
        data.changeSummary || null,
      ]
    );
    
    return result.rows[0];
  }

  async getVersions(documentId: string, limit: number = 50): Promise<DocumentVersion[]> {
    const result = await this.pool.query(
      `SELECT id, document_id as "documentId", version, content,
              created_by as "createdBy", created_at as "createdAt",
              change_summary as "changeSummary"
       FROM document_versions
       WHERE document_id = $1
       ORDER BY version DESC
       LIMIT $2`,
      [documentId, limit]
    );
    
    return result.rows;
  }

  async getVersion(documentId: string, version: number): Promise<DocumentVersion | null> {
    const result = await this.pool.query(
      `SELECT * FROM document_versions
       WHERE document_id = $1 AND version = $2`,
      [documentId, version]
    );
    
    return result.rows[0] || null;
  }

  async restoreVersion(documentId: string, version: number): Promise<void> {
    const versionData = await this.getVersion(documentId, version);
    
    if (!versionData) {
      throw new Error('Version not found');
    }

    await this.pool.query(
      `UPDATE documents
       SET content = $1, updated_at = CURRENT_TIMESTAMP
       WHERE id = $2`,
      [JSON.stringify(versionData.content), documentId]
    );
  }

  async createSnapshot(documentId: string, doc: Y.Doc, s3Key?: string): Promise<string> {
    const state = Y.encodeStateAsUpdate(doc);
    
    const result = await this.pool.query(
      `INSERT INTO document_snapshots (document_id, snapshot_data, s3_key)
       VALUES ($1, $2, $3)
       RETURNING id`,
      [documentId, state, s3Key || null]
    );
    
    return result.rows[0].id;
  }

  private hashContent(content: any): string {
    const crypto = require('crypto');
    return crypto
      .createHash('sha256')
      .update(JSON.stringify(content))
      .digest('hex');
  }
}
```

#### 4.4 Backup Strategies

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/database/backup.ts
import { Pool } from 'pg';
import { S3Client, PutObjectCommand } from '@aws-sdk/client-s3';
import { exec } from 'child_process';
import { promisify } from 'util';
import * as fs from 'fs';
import * as path from 'path';

const execAsync = promisify(exec);

interface BackupConfig {
  s3Bucket: string;
  s3Region: string;
  retentionDays: number;
  snapshotIntervalHours: number;
}

export class DatabaseBackupManager {
  private pool: Pool;
  private s3: S3Client;
  private config: BackupConfig;

  constructor(pool: Pool, config: BackupConfig) {
    this.pool = pool;
    this.config = config;
    this.s3 = new S3Client({ region: config.s3Region });
  }

  async createBackup(): Promise<string> {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const backupFile = `/tmp/pg_backup_${timestamp}.dump`;
    
    try {
      // Create physical backup using pg_dump
      const pgDumpCommand = `pg_dump -Fc -d ${this.getDatabaseName()} -f ${backupFile}`;
      await execAsync(pgDumpCommand);

      // Read backup file
      const backupData = fs.readFileSync(backupFile);

      // Upload to S3
      const s3Key = `backups/${timestamp}/database.dump`;
      await this.s3.send(new PutObjectCommand({
        Bucket: this.config.s3Bucket,
        Key: s3Key,
        Body: backupData,
        ServerSideEncryption: 'AES256',
      }));

      // Record in database
      await this.pool.query(
        `INSERT INTO backup_records (s3_key, created_at, size_bytes)
         VALUES ($1, CURRENT_TIMESTAMP, $2)`,
        [s3Key, backupData.length]
      );

      // Cleanup local file
      fs.unlinkSync(backupFile);

      console.log(`Backup created: ${s3Key}`);
      return s3Key;

    } catch (error) {
      console.error('Backup failed:', error);
      throw error;
    }
  }

  async createDocumentSnapshots(): Promise<void> {
    // Get all active documents
    const result = await this.pool.query(
      `SELECT id, name, content FROM documents WHERE is_deleted = false`
    );

    for (const doc of result.rows) {
      const timestamp = new Date().toISOString();
      const s3Key = `document-snapshots/${doc.id}/${timestamp}.json`;

      await this.s3.send(new PutObjectCommand({
        Bucket: this.config.s3Bucket,
        Key: s3Key,
        Body: JSON.stringify(doc.content),
        ServerSideEncryption: 'AES256',
      }));

      // Record snapshot
      await this.pool.query(
        `INSERT INTO document_snapshots (document_id, s3_key, created_at)
         VALUES ($1, $2, CURRENT_TIMESTAMP)`,
        [doc.id, s3Key]
      );
    }
  }

  async cleanupOldBackups(): Promise<void> {
    const cutoffDate = new Date();
    cutoffDate.setDate(cutoffDate.getDate() - this.config.retentionDays);

    // Get old backups from S3
    // Delete from S3
    // Update database records
  }

  async restoreFromBackup(s3Key: string, targetDatabase: string): Promise<void> {
    const tempFile = `/tmp/pg_restore_${Date.now()}.dump`;

    try {
      // Download from S3
      const { Body } = await this.s3.send(new PutObjectCommand({
        Bucket: this.config.s3Bucket,
        Key: s3Key,
      }));

      const stream = Body as any;
      const fileStream = fs.createWriteStream(tempFile);
      
      await new Promise((resolve, reject) => {
        stream.pipe(fileStream);
        stream.on('end', resolve);
        stream.on('error', reject);
      });

      // Restore using pg_restore
      const pgRestoreCommand = `pg_restore -d ${targetDatabase} ${tempFile}`;
      await execAsync(pgRestoreCommand);

      fs.unlinkSync(tempFile);

    } catch (error) {
      console.error('Restore failed:', error);
      throw error;
    }
  }

  private getDatabaseName(): string {
    // Extract from connection string
    return 'tiptap';
  }

  public startScheduledBackups(): void {
    // Create database snapshots every N hours
    setInterval(async () => {
      await this.createBackup();
    }, this.config.snapshotIntervalHours * 60 * 60 * 1000);

    // Cleanup old backups daily
    setInterval(async () => {
      await this.cleanupOldBackups();
    }, 24 * 60 * 60 * 1000);
  }
}
```

---

### 5. Monitoring

#### 5.1 Prometheus Metrics

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/monitoring/metrics.ts
import * as prom from 'prom-client';

// Create Registry
const register = new prom.Registry();

// Enable default metrics (CPU, memory, etc.)
prom.collectDefaultMetrics({ register });

// Custom metrics
export const metrics = {
  // Collaboration metrics
  activeConnections: new prom.Gauge({
    name: 'hocuspocus_connections_active',
    help: 'Number of active WebSocket connections',
    registers: [register],
  }),

  documentsOpen: new prom.Gauge({
    name: 'hocuspocus_documents_open',
    help: 'Number of currently open documents',
    registers: [register],
  }),

  yjsUpdates: new prom.Counter({
    name: 'hocuspocus_yjs_updates_total',
    help: 'Total number of Yjs updates received',
    labelNames: ['document_id'],
    registers: [register],
  }),

  updateSize: new prom.Histogram({
    name: 'hocuspocus_update_size_bytes',
    help: 'Size of Yjs updates in bytes',
    buckets: [100, 500, 1000, 5000, 10000, 50000, 100000],
    registers: [register],
  }),

  syncDuration: new prom.Histogram({
    name: 'hocuspocus_sync_duration_seconds',
    help: 'Time to sync document',
    buckets: [0.01, 0.05, 0.1, 0.5, 1, 5, 10],
    labelNames: ['document_id'],
    registers: [register],
  }),

  // Editor metrics
  editorActions: new prom.Counter({
    name: 'tiptap_editor_actions_total',
    help: 'Total number of editor actions',
    labelNames: ['action_type', 'document_id'],
    registers: [register],
  }),

  keypress: new prom.Counter({
    name: 'tiptap_keypress_total',
    help: 'Total number of keypresses',
    labelNames: ['document_id'],
    registers: [register],
  }),

  // Performance metrics
  websocketLatency: new prom.Histogram({
    name: 'hocuspocus_websocket_latency_ms',
    help: 'WebSocket message latency',
    buckets: [1, 5, 10, 25, 50, 100, 250, 500, 1000],
    registers: [register],
  }),

  // Error metrics
  errors: new prom.Counter({
    name: 'hocuspocus_errors_total',
    help: 'Total number of errors',
    labelNames: ['error_type', 'severity'],
    registers: [register],
  }),

  // Persistence metrics
  persistenceQueueSize: new prom.Gauge({
    name: 'hocuspocus_persistence_queue_size',
    help: 'Number of documents waiting to be persisted',
    registers: [register],
  }),

  persistenceDuration: new prom.Histogram({
    name: 'hocuspocus_persistence_duration_seconds',
    help: 'Time to persist document',
    buckets: [0.1, 0.5, 1, 5, 10, 30],
    registers: [register],
  }),

  // Presence metrics
  awarenessUpdates: new prom.Counter({
    name: 'hocuspocus_awareness_updates_total',
    help: 'Total number of awareness (cursor) updates',
    registers: [register],
  }),
};

// Express middleware for /metrics endpoint
export function metricsMiddleware(req: any, res: any, next: any): void {
  if (req.path === '/metrics') {
    res.set('Content-Type', register.contentType);
    res.end(register.metrics());
  } else {
    next();
  }
}

// Update metrics helper
export function updateMetrics(data: {
  connections?: number;
  documents?: number;
  yjsUpdate?: { documentId: string; size: number; duration: number };
  error?: { type: string; severity: string };
}): void {
  if (data.connections !== undefined) {
    metrics.activeConnections.set(data.connections);
  }
  if (data.documents !== undefined) {
    metrics.documentsOpen.set(data.documents);
  }
  if (data.yjsUpdate) {
    metrics.yjsUpdates.inc({ document_id: data.yjsUpdate.documentId });
    metrics.updateSize.observe(data.yjsUpdate.size);
    metrics.syncDuration.observe(
      { document_id: data.yjsUpdate.documentId },
      data.yjsUpdate.duration / 1000
    );
  }
  if (data.error) {
    metrics.errors.inc({ error_type: data.error.type, severity: data.error.severity });
  }
}
```

#### 5.2 Grafana Dashboard

```json
{
  "dashboard": {
    "title": "WebEditors Production Dashboard",
    "panels": [
      {
        "title": "Active Connections",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(hocuspocus_connections_active)",
            "legendFormat": "Total Connections"
          }
        ]
      },
      {
        "title": "Documents Open",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(hocuspocus_documents_open)"
          }
        ]
      },
      {
        "title": "Yjs Updates Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(hocuspocus_yjs_updates_total[5m])"
          }
        ]
      },
      {
        "title": "Sync Latency (p95)",
        "type": "heatmap",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(hocuspocus_sync_duration_seconds_bucket[5m]))"
          }
        ]
      },
      {
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(hocuspocus_errors_total[5m])"
          }
        ]
      }
    ]
  }
}
```

#### 5.3 Health Check Endpoint

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/monitoring/health.ts
import { Redis } from 'ioredis';
import { Pool } from 'pg';

interface HealthStatus {
  status: 'healthy' | 'degraded' | 'unhealthy';
  checks: {
    memory: MemoryHealth;
    redis: RedisHealth;
    database: DatabaseHealth;
    websocket: WebSocketHealth;
  };
  uptime: number;
  timestamp: string;
}

interface MemoryHealth {
  status: 'healthy' | 'warning' | 'critical';
  used: number;
  total: number;
  percentage: number;
}

interface RedisHealth {
  status: 'healthy' | 'unhealthy';
  latency: number;
}

interface DatabaseHealth {
  status: 'healthy' | 'unhealthy';
  connections: { active: number; idle: number; waiting: number };
  latency: number;
}

interface WebSocketHealth {
  status: 'healthy' | 'degraded' | 'unhealthy';
  connections: number;
  messagesPerSecond: number;
}

export class HealthChecker {
  private redis: Redis;
  private db: Pool;
  private getConnections: () => number;
  private getMessagesPerSecond: () => number;

  constructor(redis: Redis, db: Pool, getConnections: () => number, getMessagesPerSecond: () => number) {
    this.redis = redis;
    this.db = db;
    this.getConnections = getConnections;
    this.getMessagesPerSecond = getMessagesPerSecond;
  }

  async check(): Promise<HealthStatus> {
    const memory = this.checkMemory();
    const redis = await this.checkRedis();
    const database = await this.checkDatabase();
    const websocket = this.checkWebSocket();

    const overallStatus = this.calculateOverallStatus([
      memory.status,
      redis.status,
      database.status,
      websocket.status,
    ]);

    return {
      status: overallStatus,
      checks: {
        memory,
        redis,
        database,
        websocket,
      },
      uptime: process.uptime(),
      timestamp: new Date().toISOString(),
    };
  }

  private checkMemory(): MemoryHealth {
    const usage = process.memoryUsage();
    const total = usage.heapTotal;
    const used = usage.heapUsed;
    const percentage = (used / total) * 100;

    let status: MemoryHealth['status'] = 'healthy';
    if (percentage > 90) status = 'critical';
    else if (percentage > 75) status = 'warning';

    return { status, used, total, percentage };
  }

  private async checkRedis(): Promise<RedisHealth> {
    const start = Date.now();
    try {
      await this.redis.ping();
      const latency = Date.now() - start;
      return {
        status: latency < 100 ? 'healthy' : 'unhealthy',
        latency,
      };
    } catch {
      return { status: 'unhealthy', latency: -1 };
    }
  }

  private async checkDatabase(): Promise<DatabaseHealth> {
    const start = Date.now();
    try {
      const result = await this.db.query('SELECT 1');
      const latency = Date.now() - start;
      
      return {
        status: latency < 500 ? 'healthy' : 'unhealthy',
        connections: {
          active: this.db.totalCount - this.db.idleCount,
          idle: this.db.idleCount,
          waiting: this.db.waitingCount,
        },
        latency,
      };
    } catch {
      return {
        status: 'unhealthy',
        connections: { active: 0, idle: 0, waiting: 0 },
        latency: -1,
      };
    }
  }

  private checkWebSocket(): WebSocketHealth {
    const connections = this.getConnections();
    const messagesPerSecond = this.getMessagesPerSecond();

    let status: WebSocketHealth['status'] = 'healthy';
    if (connections > 10000 || messagesPerSecond > 50000) {
      status = 'degraded';
    }

    return { status, connections, messagesPerSecond };
  }

  private calculateOverallStatus(
    statuses: Array<'healthy' | 'warning' | 'critical' | 'unhealthy' | 'degraded'>
  ): 'healthy' | 'degraded' | 'unhealthy' {
    if (statuses.includes('critical') || statuses.includes('unhealthy')) {
      return 'unhealthy';
    }
    if (statuses.includes('warning') || statuses.includes('degraded')) {
      return 'degraded';
    }
    return 'healthy';
  }
}
```

---

## Part 2: tldraw Production

### 6. Application Deployment

#### 6.1 Static Hosting

**Vercel Deployment:**

```json
{
  "version": 2,
  "name": "tldraw-app",
  "builds": [
    {
      "src": "package.json",
      "use": "@vercel/static-build",
      "config": { "distDir": "dist" }
    }
  ],
  "routes": [
    {
      "src": "/assets/(.*)",
      "headers": { "cache-control": "public, max-age=31536000, immutable" },
      "continue": true
    },
    {
      "src": "/(.*)",
      "dest": "/index.html"
    }
  ],
  "env": {
    "MULTIPLAYER_SERVER": "@tldraw-multiplayer-url"
  }
}
```

**AWS S3 + CloudFront:**

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/cloud/aws/s3-hosting.ts
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as iam from 'aws-cdk-lib/aws-iam';
import { Construct } from 'constructs';
import * as cdk from 'aws-cdk-lib';

export class TldrawStaticHostingStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // S3 Bucket for static assets
    const bucket = new s3.Bucket(this, 'TldrawAssets', {
      versioned: true,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      autoDeleteObjects: false,
      publicAccess: false,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      encryption: s3.BucketEncryption.S3_MANAGED,
    });

    // CloudFront Origin Access Identity
    const oai = new cloudfront.OriginAccessIdentity(this, 'TldrawOAI', {
      comment: 'OAI for Tldraw bucket',
    });

    // Grant CloudFront access to bucket
    bucket.grantRead(oai);

    // CloudFront Distribution
    const distribution = new cloudfront.Distribution(this, 'TldrawDistribution', {
      defaultBehavior: {
        origin: new origins.S3Origin(bucket, {
          originAccessIdentity: oai,
        }),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD,
        cachedMethods: cloudfront.CachedMethods.CACHE_GET_HEAD,
        cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
        compress: true,
      },
      errorResponses: [
        {
          httpStatus: 404,
          responseHttpStatus: 200,
          responsePagePath: '/index.html',
        },
        {
          httpStatus: 403,
          responseHttpStatus: 200,
          responsePagePath: '/index.html',
        },
      ],
      defaultRootObject: 'index.html',
      priceClass: cloudfront.PriceClass.PRICE_CLASS_100,
      enabled: true,
      httpVersion: cloudfront.HttpVersion.HTTP2_AND_3,
    });

    // Outputs
    new cdk.CfnOutput(this, 'DistributionId', {
      value: distribution.distributionId,
    });

    new cdk.CfnOutput(this, 'DistributionDomain', {
      value: distribution.distributionDomainName,
    });

    new cdk.CfnOutput(this, 'BucketName', {
      value: bucket.bucketName,
    });
  }
}
```

#### 6.2 Edge Deployment with Cloudflare Workers

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/cloud/cloudflare/tldraw-worker.ts
export interface Env {
  ASSETS: Fetcher;
  MULTIPLAYER_WS: string;
  REDIS_KV: KVNamespace;
}

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);

    // Handle WebSocket connections
    if (url.pathname === '/multiplayer' && request.headers.get('Upgrade') === 'websocket') {
      return handleMultiplayerWS(request, env);
    }

    // Check KV cache for assets
    const cacheKey = `asset:${url.pathname}`;
    const cached = await env.REDIS_KV.get(cacheKey, 'json');
    
    if (cached) {
      return new Response(cached.content, {
        headers: {
          'Content-Type': cached.contentType,
          'Cache-Control': 'public, max-age=31536000, immutable',
        },
      });
    }

    // Fall through to R2/Assets
    return env.ASSETS.fetch(request);
  },
};

async function handleMultiplayerWS(request: Request, env: Env): Promise<Response> {
  const pair = new WebSocketPair();
  const [client, server] = Object.values(pair);

  // Accept the WebSocket on the client side
  server.accept();

  // Connect to the origin multiplayer server
  const originWS = new WebSocket(env.MULTIPLAYER_WS);

  originWS.addEventListener('open', () => {
    // Forward client messages to origin
    server.addEventListener('message', (event) => {
      if (originWS.readyState === WebSocket.OPEN) {
        originWS.send(event.data);
      }
    });
  });

  originWS.addEventListener('message', (event) => {
    // Forward origin messages to client
    if (server.readyState === WebSocket.OPEN) {
      server.send(event.data);
    }
  });

  originWS.accept();

  return new Response(null, {
    status: 101,
    webSocket: client,
  });
}
```

#### 6.3 Multiplayer Server

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/apps/tldraw-multiplayer/src/server.ts
import WebSocket from 'ws';
import { createServer } from 'http';
import { Redis } from 'ioredis';
import { v4 as uuidv4 } from 'uuid';
import * as jwt from 'jsonwebtoken';

interface Client {
  id: string;
  ws: WebSocket;
  roomId: string;
  userId: string;
  presence: any;
  lastActivity: number;
}

interface Room {
  id: string;
  clients: Map<string, Client>;
  document: any;
  version: number;
}

class TldrawMultiplayerServer {
  private wss: WebSocket.Server;
  private redis: Redis;
  private rooms: Map<string, Room> = new Map();
  private clients: Map<string, Client> = new Map();
  private secretKey: string;

  constructor(port: number, redisUrl: string, secretKey: string) {
    this.secretKey = secretKey;
    this.redis = new Redis(redisUrl);
    
    const server = createServer((req, res) => {
      if (req.url === '/health') {
        res.writeHead(200);
        res.end('OK');
      } else {
        res.writeHead(404);
        res.end('Not Found');
      }
    });

    this.wss = new WebSocket.Server({ server });
    this.wss.on('connection', (ws, req) => this.handleConnection(ws, req));

    this.setupRedisPubSub();
    this.setupHeartbeat();

    server.listen(port, () => {
      console.log(`Multiplayer server listening on port ${port}`);
    });
  }

  private handleConnection(ws: WebSocket, req: any): void {
    const clientId = uuidv4();
    
    ws.on('message', async (data: Buffer) => {
      try {
        const message = JSON.parse(data.toString());
        await this.handleMessage(clientId, message);
      } catch (error) {
        console.error('Message handling error:', error);
        ws.send(JSON.stringify({ type: 'error', message: 'Invalid message' }));
      }
    });

    ws.on('close', () => this.handleDisconnect(clientId));
    ws.on('error', (error) => {
      console.error('WebSocket error:', error);
      this.handleDisconnect(clientId);
    });

    // Send welcome message
    ws.send(JSON.stringify({
      type: 'welcome',
      clientId,
    }));
  }

  private async handleMessage(clientId: string, message: any): Promise<void> {
    const client = this.clients.get(clientId);
    
    if (!client) {
      // First message should be authentication
      if (message.type === 'auth') {
        await this.handleAuth(clientId, message);
      }
      return;
    }

    switch (message.type) {
      case 'presence':
        await this.handlePresence(client, message.presence);
        break;
      case 'update':
        await this.handleUpdate(client, message.updates);
        break;
      case 'cursor':
        await this.handleCursor(client, message.cursor);
        break;
    }
  }

  private async handleAuth(clientId: string, message: any): Promise<void> {
    const { token, roomId } = message;
    
    const decoded = jwt.verify(token, this.secretKey) as {
      userId: string;
      permissions: string[];
    };

    const room = await this.getOrCreateRoom(roomId);
    
    const client: Client = {
      id: clientId,
      ws: this.wss.clients.values().next().value,
      roomId,
      userId: decoded.userId,
      presence: null,
      lastActivity: Date.now(),
    };

    this.clients.set(clientId, client);
    room.clients.set(clientId, client);

    // Send room state to new client
    client.ws.send(JSON.stringify({
      type: 'room_state',
      document: room.document,
      version: room.version,
      clients: Array.from(room.clients.values()).map(c => ({
        id: c.id,
        userId: c.userId,
        presence: c.presence,
      })),
    }));

    // Broadcast new client to room
    this.broadcastToRoom(roomId, {
      type: 'client_joined',
      clientId,
      userId: decoded.userId,
    }, clientId);
  }

  private async handlePresence(client: Client, presence: any): Promise<void> {
    client.presence = presence;
    client.lastActivity = Date.now();

    this.broadcastToRoom(client.roomId, {
      type: 'presence',
      clientId: client.id,
      presence,
    }, client.id);
  }

  private async handleUpdate(client: Client, updates: any[]): Promise<void> {
    const room = this.rooms.get(client.roomId);
    if (!room) return;

    // Apply updates to room document
    room.document = this.applyUpdates(room.document, updates);
    room.version++;

    // Broadcast to other clients
    this.broadcastToRoom(client.roomId, {
      type: 'update',
      updates,
      version: room.version,
    }, client.id);

    // Persist to Redis
    await this.persistRoom(room);
  }

  private async handleCursor(client: Client, cursor: any): Promise<void> {
    this.broadcastToRoom(client.roomId, {
      type: 'cursor',
      clientId: client.id,
      cursor,
    }, client.id);
  }

  private broadcastToRoom(roomId: string, message: any, excludeClientId?: string): void {
    const room = this.rooms.get(roomId);
    if (!room) return;

    const data = JSON.stringify(message);
    
    room.clients.forEach((client, id) => {
      if (id !== excludeClientId && client.ws.readyState === WebSocket.OPEN) {
        client.ws.send(data);
      }
    });
  }

  private async getOrCreateRoom(roomId: string): Promise<Room> {
    let room = this.rooms.get(roomId);
    
    if (!room) {
      // Try to load from Redis
      const cached = await this.redis.get(`room:${roomId}`);
      
      room = {
        id: roomId,
        clients: new Map(),
        document: cached ? JSON.parse(cached) : {},
        version: 0,
      };
      
      this.rooms.set(roomId, room);
    }
    
    return room;
  }

  private async persistRoom(room: Room): Promise<void> {
    await this.redis.setex(
      `room:${room.id}`,
      3600,
      JSON.stringify(room.document)
    );
  }

  private setupRedisPubSub(): void {
    const sub = this.redis.duplicate();
    sub.subscribe('rooms:update');
    
    sub.on('message', (channel, message) => {
      const data = JSON.parse(message);
      // Handle cross-room synchronization if needed
    });
  }

  private setupHeartbeat(): void {
    setInterval(() => {
      const now = Date.now();
      const timeout = 60000; // 1 minute
      
      this.clients.forEach((client, id) => {
        if (now - client.lastActivity > timeout) {
          this.handleDisconnect(id);
        }
      });
    }, 30000);
  }

  private handleDisconnect(clientId: string): void {
    const client = this.clients.get(clientId);
    if (!client) return;

    const room = this.rooms.get(client.roomId);
    if (room) {
      room.clients.delete(clientId);
      
      // Broadcast disconnection
      this.broadcastToRoom(client.roomId, {
        type: 'client_left',
        clientId,
      });

      // Cleanup empty rooms
      if (room.clients.size === 0) {
        this.rooms.delete(client.roomId);
      }
    }

    this.clients.delete(clientId);
  }

  private applyUpdates(document: any, updates: any[]): any {
    // Apply CRDT-style updates to document
    // This is simplified - in production use a proper CRDT library
    return { ...document, ...updates.reduce((acc, u) => ({ ...acc, ...u }), {}) };
  }
}

// Start server
const server = new TldrawMultiplayerServer(
  parseInt(process.env.PORT || '3001'),
  process.env.REDIS_URL || 'redis://localhost:6379',
  process.env.SECRET_KEY || 'change-me'
);
```

---

### 7. Scaling Multiplayer

#### 7.1 WebSocket Server Clustering

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/apps/tldraw-multiplayer/src/cluster.ts
import WebSocket from 'ws';
import { Redis } from 'ioredis';
import { createHash } from 'crypto';

interface ClusterNode {
  id: string;
  host: string;
  port: number;
  connections: number;
  lastHeartbeat: number;
}

export class WebSocketCluster {
  private nodeId: string;
  private redis: Redis;
  private nodes: Map<string, ClusterNode> = new Map();
  private localClients: Map<string, WebSocket> = new Map();
  private roomToNode: Map<string, string> = new Map();

  constructor(nodeId: string, redis: Redis) {
    this.nodeId = nodeId;
    this.redis = redis;
    
    this.startHeartbeat();
    this.startDiscovery();
  }

  private startHeartbeat(): void {
    setInterval(async () => {
      await this.redis.setex(
        `node:${this.nodeId}:heartbeat`,
        15,
        JSON.stringify({
          connections: this.localClients.size,
          timestamp: Date.now(),
        })
      );
    }, 5000);
  }

  private async startDiscovery(): Promise<void> {
    await this.discoverNodes();
    setInterval(() => this.discoverNodes(), 10000);
  }

  private async discoverNodes(): Promise<void> {
    const keys = await this.redis.keys('node:*:heartbeat');
    
    for (const key of keys) {
      const nodeId = key.split(':')[1];
      const data = await this.redis.get(key);
      
      if (data) {
        const node = JSON.parse(data);
        this.nodes.set(nodeId, {
          id: nodeId,
          host: nodeId, // In production, store actual host separately
          port: 3001,
          connections: node.connections,
          lastHeartbeat: Date.now(),
        });
      }
    }

    // Remove stale nodes
    const now = Date.now();
    this.nodes.forEach((node, id) => {
      if (now - node.lastHeartbeat > 30000) {
        this.nodes.delete(id);
      }
    });
  }

  public getNodeForRoom(roomId: string): string {
    // Check cached mapping
    const cached = this.roomToNode.get(roomId);
    if (cached && this.nodes.has(cached)) {
      return cached;
    }

    // Use consistent hashing
    const nodeIds = Array.from(this.nodes.keys());
    const hash = this.hashRoom(roomId);
    const index = hash % nodeIds.length;
    const node = nodeIds[index];

    this.roomToNode.set(roomId, node);
    return node;
  }

  private hashRoom(roomId: string): number {
    const hash = createHash('md5').update(roomId).digest('hex');
    return parseInt(hash.substring(0, 8), 16);
  }

  public addClient(clientId: string, ws: WebSocket): void {
    this.localClients.set(clientId, ws);
  }

  public removeClient(clientId: string): void {
    this.localClients.delete(clientId);
  }

  public broadcastToNode(nodeId: string, roomId: string, message: any): void {
    if (nodeId === this.nodeId) {
      // Local broadcast
      this.localClients.forEach((ws) => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify(message));
        }
      });
    } else {
      // Remote broadcast via Redis
      this.redis.publish(`node:${nodeId}:broadcast`, JSON.stringify({
        roomId,
        message,
      }));
    }
  }

  public async forwardToNode(nodeId: string, roomId: string, clientId: string, message: any): Promise<void> {
    await this.redis.publish(`node:${nodeId}:message`, JSON.stringify({
      roomId,
      clientId,
      message,
    }));
  }
}
```

#### 7.2 Room Management

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/apps/tldraw-multiplayer/src/room-manager.ts
import { Redis } from 'ioredis';

interface RoomState {
  id: string;
  document: any;
  version: number;
  clients: Map<string, RoomClient>;
  createdAt: number;
  lastActivity: number;
}

interface RoomClient {
  id: string;
  userId: string;
  presence: any;
  joinedAt: number;
}

export class DistributedRoomManager {
  private redis: Redis;
  private localRooms: Map<string, RoomState> = new Map();
  private nodeId: string;

  constructor(redis: Redis, nodeId: string) {
    this.redis = redis;
    this.nodeId = nodeId;
    
    this.startCleanup();
  }

  async getRoom(roomId: string): Promise<RoomState | null> {
    // Check local cache first
    const local = this.localRooms.get(roomId);
    if (local) {
      return local;
    }

    // Check if room exists on another node
    const ownerNode = await this.redis.get(`room:${roomId}:owner`);
    
    if (ownerNode && ownerNode !== this.nodeId) {
      // Room is on another node - would need to proxy
      return null;
    }

    // Create new room
    const room: RoomState = {
      id: roomId,
      document: {},
      version: 0,
      clients: new Map(),
      createdAt: Date.now(),
      lastActivity: Date.now(),
    };

    this.localRooms.set(roomId, room);
    await this.redis.setex(`room:${roomId}:owner`, 3600, this.nodeId);

    return room;
  }

  async updateRoom(roomId: string, updates: any): Promise<void> {
    const room = this.localRooms.get(roomId);
    if (!room) return;

    room.document = { ...room.document, ...updates };
    room.version++;
    room.lastActivity = Date.now();

    // Async persist
    this.persistRoom(room).catch(console.error);
  }

  async addClient(roomId: string, client: RoomClient): Promise<void> {
    const room = await this.getRoom(roomId);
    if (!room) return;

    room.clients.set(client.id, client);
    room.lastActivity = Date.now();
  }

  async removeClient(roomId: string, clientId: string): Promise<void> {
    const room = this.localRooms.get(roomId);
    if (!room) return;

    room.clients.delete(clientId);
    room.lastActivity = Date.now();

    // Cleanup empty room
    if (room.clients.size === 0) {
      this.localRooms.delete(roomId);
      await this.redis.del(`room:${roomId}:owner`);
    }
  }

  async getRoomClients(roomId: string): Promise<RoomClient[]> {
    const room = this.localRooms.get(roomId);
    if (!room) return [];

    return Array.from(room.clients.values());
  }

  private async persistRoom(room: RoomState): Promise<void> {
    await this.redis.setex(
      `room:${room.id}:state`,
      3600,
      JSON.stringify({
        document: room.document,
        version: room.version,
      })
    );
  }

  private startCleanup(): void {
    setInterval(async () => {
      const now = Date.now();
      const timeout = 3600000; // 1 hour

      this.localRooms.forEach((room, id) => {
        if (now - room.lastActivity > timeout && room.clients.size === 0) {
          this.localRooms.delete(id);
          this.redis.del(`room:${id}:owner`);
        }
      });
    }, 60000);
  }
}
```

#### 7.3 State Synchronization

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/apps/tldraw-multiplayer/src/crdt.ts
import * as Y from 'yjs';

export class DocumentCRDT {
  private doc: Y.Doc;
  private yXml: Y.XmlFragment;
  private awareness: any;

  constructor() {
    this.doc = new Y.Doc();
    this.yXml = this.doc.getXmlFragment('document');
    // this.awareness = new Awareness(this.doc);
  }

  getDocument(): Y.Doc {
    return this.doc;
  }

  getUpdates(): Uint8Array {
    return Y.encodeStateAsUpdate(this.doc);
  }

  applyUpdates(update: Uint8Array): void {
    Y.applyUpdate(this.doc, update);
  }

  getUpdatesSince(stateVector: Uint8Array): Uint8Array {
    return Y.encodeStateAsUpdate(this.doc, stateVector);
  }

  getStateVector(): Uint8Array {
    return Y.encodeStateVector(this.doc);
  }

  toSnapshot(): Buffer {
    return Buffer.from(Y.encodeStateAsUpdate(this.doc));
  }

  static fromSnapshot(snapshot: Buffer): DocumentCRDT {
    const crdt = new DocumentCRDT();
    Y.applyUpdate(crdt.doc, snapshot);
    return crdt;
  }

  onUpdate(callback: (update: Uint8Array, origin: any) => void): void {
    this.doc.on('update', callback);
  }

  destroy(): void {
    this.doc.destroy();
  }
}
```

#### 7.4 Presence System

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/apps/tldraw-multiplayer/src/presence.ts
import { Redis } from 'ioredis';

interface PresenceState {
  userId: string;
  roomId: string;
  cursor: { x: number; y: number };
  selection: any | null;
  color: string;
  name: string;
  lastActive: number;
}

export class PresenceManager {
  private redis: Redis;
  private localPresence: Map<string, PresenceState> = new Map();

  constructor(redis: Redis) {
    this.redis = redis;
    this.startCleanup();
  }

  async updatePresence(presence: PresenceState): Promise<void> {
    const key = `presence:${presence.roomId}:${presence.userId}`;
    
    await this.redis.setex(
      key,
      30, // 30 second TTL
      JSON.stringify(presence)
    );

    this.localPresence.set(`${presence.roomId}:${presence.userId}`, presence);

    // Publish update
    await this.redis.publish('presence:update', JSON.stringify({
      roomId: presence.roomId,
      userId: presence.userId,
      presence,
    }));
  }

  async getRoomPresence(roomId: string): Promise<PresenceState[]> {
    const keys = await this.redis.keys(`presence:${roomId}:*`);
    const presenceList: PresenceState[] = [];

    for (const key of keys) {
      const data = await this.redis.get(key);
      if (data) {
        presenceList.push(JSON.parse(data));
      }
    }

    return presenceList;
  }

  async removePresence(roomId: string, userId: string): Promise<void> {
    const key = `presence:${roomId}:${userId}`;
    await this.redis.del(key);
    this.localPresence.delete(`${roomId}:${userId}`);
  }

  private startCleanup(): void {
    // Presence has TTL, but we can also do periodic cleanup
    setInterval(async () => {
      // Remove stale local presence
    }, 60000);
  }
}
```

---

### 8. Asset Management

#### 8.1 Asset CDN Configuration

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/cloud/aws/asset-cdn.ts
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as iam from 'aws-cdk-lib/aws-iam';
import { Construct } from 'constructs';
import * as cdk from 'aws-cdk-lib';

export class AssetCDNStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // S3 bucket for assets
    const assetBucket = new s3.Bucket(this, 'AssetBucket', {
      versioned: true,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
      autoDeleteObjects: true,
      cors: [
        {
          allowedMethods: [s3.HttpMethods.GET, s3.HttpMethods.PUT, s3.HttpMethods.POST],
          allowedOrigins: ['*'],
          allowedHeaders: ['*'],
          maxAge: 3600,
        },
      ],
    });

    // Lambda@Edge for image optimization
    const imageOptimizer = new lambda.Function(this, 'ImageOptimizer', {
      runtime: lambda.Runtime.NODEJS_18_X,
      handler: 'index.handler',
      code: lambda.Code.fromAsset('lambda/image-optimizer'),
    });

    // CloudFront distribution
    const distribution = new cloudfront.Distribution(this, 'AssetDistribution', {
      defaultBehavior: {
        origin: new origins.S3Origin(assetBucket),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
        compress: true,
      },
      
      // Additional behaviors for different asset types
      additionalBehaviors: {
        '/images/*': {
          origin: new origins.S3Origin(assetBucket),
          cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
          edgeLambdas: [
            {
              functionType: cloudfront.LambdaFunctionEventType.ORIGIN_REQUEST,
              functionVersion: imageOptimizer.currentVersion,
            },
          ],
        },
      },
    });
  }
}
```

#### 8.2 Image Optimization Lambda

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/lambda/image-optimizer/index.ts
import * as sharp from 'sharp';
import * as AWS from '@aws-sdk/client-s3';

const s3 = new S3Client({});

export async function handler(event: any): Promise<any> {
  const record = event.Records[0];
  const key = decodeURIComponent(record.s3.object.key.replace(/\+/g, ' '));
  const bucket = record.s3.bucket.name;

  // Get the image from S3
  const { Body } = await s3.send(new GetObjectCommand({ Bucket: bucket, Key: key }));

  // Optimize image
  const optimized = await sharp(Body as Buffer)
    .resize(1920, 1080, { fit: 'inside', withoutEnlargement: true })
    .jpeg({ quality: 85, progressive: true })
    .toBuffer();

  // Put optimized image back
  const optimizedKey = `optimized/${key}`;
  await s3.send(new PutObjectCommand({
    Bucket: bucket,
    Key: optimizedKey,
    Body: optimized,
    ContentType: 'image/jpeg',
    CacheControl: 'public, max-age=31536000, immutable',
  }));

  return {
    optimizedKey,
    size: optimized.length,
  };
}
```

#### 8.3 Caching Strategies

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/apps/tldraw-multiplayer/src/cache.ts
import { Redis } from 'ioredis';

interface CacheConfig {
  ttl: number;
  prefix: string;
}

export class AssetCache {
  private redis: Redis;
  private config: CacheConfig;

  constructor(redis: Redis, config: CacheConfig) {
    this.redis = redis;
    this.config = config;
  }

  async get(key: string): Promise<string | null> {
    return this.redis.get(`${this.config.prefix}:${key}`);
  }

  async set(key: string, value: string, ttl?: number): Promise<void> {
    const cacheKey = `${this.config.prefix}:${key}`;
    await this.redis.setex(cacheKey, ttl || this.config.ttl, value);
  }

  async delete(key: string): Promise<void> {
    await this.redis.del(`${this.config.prefix}:${key}`);
  }

  async invalidatePattern(pattern: string): Promise<void> {
    const keys = await this.redis.keys(`${this.config.prefix}:${pattern}`);
    if (keys.length > 0) {
      await this.redis.del(...keys);
    }
  }
}

// Multi-tier caching
export class MultiTierCache {
  private lru: Map<string, { value: any; expiry: number }>;
  private redis: Redis;
  private maxLruSize: number;

  constructor(redis: Redis, maxLruSize: number = 1000) {
    this.redis = redis;
    this.maxLruSize = maxLruSize;
    this.lru = new Map();
  }

  async get<T>(key: string): Promise<T | null> {
    // Check LRU first
    const lruEntry = this.lru.get(key);
    if (lruEntry) {
      if (lruEntry.expiry > Date.now()) {
        return lruEntry.value as T;
      }
      this.lru.delete(key);
    }

    // Check Redis
    const redisValue = await this.redis.get(key);
    if (redisValue) {
      const value = JSON.parse(redisValue);
      this.addToLru(key, value);
      return value as T;
    }

    return null;
  }

  async set<T>(key: string, value: T, ttl: number): Promise<void> {
    // Add to LRU
    this.addToLru(key, value);

    // Add to Redis
    await this.redis.setex(key, Math.floor(ttl / 1000), JSON.stringify(value));
  }

  private addToLru(key: string, value: any): void {
    if (this.lru.size >= this.maxLruSize) {
      // Remove oldest entry
      const firstKey = this.lru.keys().next().value;
      this.lru.delete(firstKey);
    }

    this.lru.set(key, {
      value,
      expiry: Date.now() + 300000, // 5 minute LRU TTL
    });
  }
}
```

---

## Part 3: Common Topics

### 9. Security

#### 9.1 Authentication

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/security/auth.ts
import * as jwt from 'jsonwebtoken';
import * as bcrypt from 'bcrypt';
import { Redis } from 'ioredis';

interface User {
  id: string;
  email: string;
  name: string;
  role: string;
}

interface JWTPayload {
  userId: string;
  email: string;
  role: string;
}

export class AuthService {
  private redis: Redis;
  private secretKey: string;
  private tokenExpiry: string;

  constructor(redis: Redis, secretKey: string, tokenExpiry: string = '24h') {
    this.redis = redis;
    this.secretKey = secretKey;
    this.tokenExpiry = tokenExpiry;
  }

  async createToken(user: User): Promise<string> {
    const payload: JWTPayload = {
      userId: user.id,
      email: user.email,
      role: user.role,
    };

    return jwt.sign(payload, this.secretKey, {
      expiresIn: this.tokenExpiry,
    });
  }

  async verifyToken(token: string): Promise<JWTPayload> {
    try {
      // Check if token is blacklisted
      const isBlacklisted = await this.redis.get(`token:blacklist:${token}`);
      if (isBlacklisted) {
        throw new Error('Token has been revoked');
      }

      return jwt.verify(token, this.secretKey) as JWTPayload;
    } catch (error) {
      throw new Error('Invalid token');
    }
  }

  async revokeToken(token: string, expiry: number): Promise<void> {
    const ttl = Math.floor(expiry / 1000);
    await this.redis.setex(`token:blacklist:${token}`, ttl, 'revoked');
  }

  async hashPassword(password: string): Promise<string> {
    return bcrypt.hash(password, 12);
  }

  async verifyPassword(password: string, hash: string): Promise<boolean> {
    return bcrypt.compare(password, hash);
  }
}

// Middleware for Express
export function authMiddleware(authService: AuthService) {
  return async (req: any, res: any, next: any): Promise<void> => {
    const authHeader = req.headers.authorization;
    
    if (!authHeader || !authHeader.startsWith('Bearer ')) {
      res.status(401).json({ error: 'Unauthorized' });
      return;
    }

    const token = authHeader.substring(7);
    
    try {
      const payload = await authService.verifyToken(token);
      req.user = payload;
      next();
    } catch (error) {
      res.status(401).json({ error: 'Invalid token' });
    }
  };
}
```

#### 9.2 Authorization

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/security/authorization.ts
import { Pool } from 'pg';

export enum Permission {
  READ = 'read',
  WRITE = 'write',
  ADMIN = 'admin',
}

export class AuthorizationService {
  private db: Pool;

  constructor(db: Pool) {
    this.db = pool;
  }

  async checkDocumentPermission(
    userId: string,
    documentId: string,
    requiredPermission: Permission
  ): Promise<boolean> {
    // Check direct permissions
    const directResult = await this.db.query(
      `SELECT permission FROM document_collaborators
       WHERE user_id = $1 AND document_id = $2`,
      [userId, documentId]
    );

    if (directResult.rows.length > 0) {
      const permission = directResult.rows[0].permission;
      return this.hasPermission(permission, requiredPermission);
    }

    // Check ownership
    const ownerResult = await this.db.query(
      `SELECT owner_id FROM documents WHERE id = $1`,
      [documentId]
    );

    if (ownerResult.rows[0]?.owner_id === userId) {
      return true;
    }

    return false;
  }

  private hasPermission(actual: string, required: Permission): boolean {
    const permissionLevels = {
      [Permission.READ]: 1,
      [Permission.WRITE]: 2,
      [Permission.ADMIN]: 3,
    };

    return permissionLevels[actual as Permission] >= permissionLevels[required];
  }

  async grantPermission(
    documentId: string,
    userId: string,
    permission: Permission,
    grantedBy: string
  ): Promise<void> {
    await this.db.query(
      `INSERT INTO document_collaborators 
       (document_id, user_id, permission, granted_by)
       VALUES ($1, $2, $3, $4)
       ON CONFLICT (document_id, user_id) 
       DO UPDATE SET permission = $3, granted_by = $4`,
      [documentId, userId, permission, grantedBy]
    );
  }

  async revokePermission(documentId: string, userId: string): Promise<void> {
    await this.db.query(
      `DELETE FROM document_collaborators
       WHERE document_id = $1 AND user_id = $2`,
      [documentId, userId]
    );
  }
}

// Express middleware
export function requirePermission(permission: Permission) {
  return async (req: any, res: any, next: any): Promise<void> => {
    const { userId } = req.user;
    const { documentId } = req.params;

    const authz = new AuthorizationService(req.db);
    const hasAccess = await authz.checkDocumentPermission(
      userId,
      documentId,
      permission
    );

    if (!hasAccess) {
      res.status(403).json({ error: 'Forbidden' });
      return;
    }

    next();
  };
}
```

#### 9.3 Rate Limiting

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/security/rate-limit.ts
import { Redis } from 'ioredis';

interface RateLimitConfig {
  windowMs: number;
  maxRequests: number;
  keyPrefix: string;
}

export class RateLimiter {
  private redis: Redis;
  private config: RateLimitConfig;

  constructor(redis: Redis, config: RateLimitConfig) {
    this.redis = redis;
    this.config = config;
  }

  async checkLimit(key: string): Promise<{ allowed: boolean; remaining: number; resetAt: number }> {
    const now = Date.now();
    const windowStart = now - this.config.windowMs;
    const bucketKey = `${this.config.keyPrefix}:${key}`;

    // Use Redis sorted set for sliding window
    const multi = this.redis.multi();
    
    // Remove old entries
    multi.zremrangebyscore(bucketKey, 0, windowStart);
    
    // Add current request
    multi.zadd(bucketKey, now, `${now}:${Math.random()}`);
    
    // Count requests in window
    multi.zcard(bucketKey);
    
    // Set expiry
    multi.expire(bucketKey, Math.ceil(this.config.windowMs / 1000));

    const results = await multi.exec();
    const count = (results[2][1] as number);

    const remaining = Math.max(0, this.config.maxRequests - count);
    const resetAt = now + this.config.windowMs;

    return {
      allowed: count <= this.config.maxRequests,
      remaining,
      resetAt,
    };
  }
}

// Express middleware
export function rateLimitMiddleware(limiter: RateLimiter, keyExtractor: (req: any) => string) {
  return async (req: any, res: any, next: any): Promise<void> => {
    const key = keyExtractor(req);
    const result = await limiter.checkLimit(key);

    res.set('X-RateLimit-Limit', String(limiter.config.maxRequests));
    res.set('X-RateLimit-Remaining', String(result.remaining));
    res.set('X-RateLimit-Reset', String(result.resetAt));

    if (!result.allowed) {
      res.status(429).json({ error: 'Too many requests' });
      return;
    }

    next();
  };
}

// Usage examples
export const limiters = {
  api: new RateLimiter(redis, {
    windowMs: 60000, // 1 minute
    maxRequests: 100,
    keyPrefix: 'ratelimit:api',
  }),

  websocket: new RateLimiter(redis, {
    windowMs: 60000,
    maxRequests: 1000,
    keyPrefix: 'ratelimit:ws',
  }),

  upload: new RateLimiter(redis, {
    windowMs: 3600000, // 1 hour
    maxRequests: 10,
    keyPrefix: 'ratelimit:upload',
  }),
};
```

#### 9.4 Content Sanitization

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/security/sanitization.ts
import DOMPurify from 'isomorphic-dompurify';
import { escapeHtml } from 'escape-html';

export class ContentSanitizer {
  private allowedTags: string[];
  private allowedAttributes: { [key: string]: string[] };

  constructor() {
    this.allowedTags = [
      'p', 'br', 'strong', 'em', 'u', 's', 'code', 'pre',
      'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
      'ul', 'ol', 'li',
      'blockquote', 'hr',
      'a', 'img',
      'table', 'thead', 'tbody', 'tr', 'th', 'td',
      'div', 'span',
    ];

    this.allowedAttributes = {
      '*': ['class', 'id', 'style'],
      a: ['href', 'target', 'rel', 'title'],
      img: ['src', 'alt', 'title', 'width', 'height'],
      td: ['colspan', 'rowspan'],
      th: ['colspan', 'rowspan', 'scope'],
    };
  }

  sanitizeHtml(html: string): string {
    return DOMPurify.sanitize(html, {
      ALLOWED_TAGS: this.allowedTags,
      ALLOWED_ATTR: this.allowedAttributes,
      ADD_TAGS: [],
      ADD_ATTR: [],
      FORBID_TAGS: ['script', 'iframe', 'object', 'embed', 'form', 'input'],
      FORBID_ATTR: ['onclick', 'onerror', 'onload', 'onmouseover', 'onfocus'],
      ALLOW_DATA_ATTR: false,
      SANITIZE_DOM: true,
    });
  }

  sanitizeUrl(url: string): string | null {
    try {
      const parsed = new URL(url);
      const allowedProtocols = ['http:', 'https:', 'mailto:', 'tel:'];
      
      if (!allowedProtocols.includes(parsed.protocol)) {
        return null;
      }

      return url;
    } catch {
      return null;
    }
  }

  escapeUserInput(input: string): string {
    return escapeHtml(input);
  }

  sanitizeForProseMirror(content: any): any {
    // Recursively sanitize ProseMirror document
    if (typeof content === 'string') {
      return this.sanitizeHtml(content);
    }

    if (Array.isArray(content)) {
      return content.map(item => this.sanitizeForProseMirror(item));
    }

    if (content && typeof content === 'object') {
      const sanitized: any = {};
      
      for (const [key, value] of Object.entries(content)) {
        if (key === 'text' || key === 'html') {
          sanitized[key] = this.sanitizeHtml(value);
        } else if (key === 'attrs') {
          sanitized[key] = this.sanitizeAttributes(value);
        } else {
          sanitized[key] = this.sanitizeForProseMirror(value);
        }
      }
      
      return sanitized;
    }

    return content;
  }

  private sanitizeAttributes(attrs: any): any {
    const sanitized: any = {};
    
    for (const [key, value] of Object.entries(attrs)) {
      if (key === 'href') {
        const sanitizedUrl = this.sanitizeUrl(value as string);
        if (sanitizedUrl) {
          sanitized[key] = sanitizedUrl;
          sanitized['target'] = '_blank';
          sanitized['rel'] = 'noopener noreferrer';
        }
      } else if (key === 'src') {
        const sanitizedUrl = this.sanitizeUrl(value as string);
        if (sanitizedUrl) {
          sanitized[key] = sanitizedUrl;
        }
      } else if (typeof value === 'string') {
        sanitized[key] = this.escapeUserInput(value);
      } else {
        sanitized[key] = value;
      }
    }
    
    return sanitized;
  }
}
```

#### 9.5 XSS Prevention

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/security/xss-prevention.ts
import { Response, NextFunction } from 'express';

export class XSSProtection {
  // Security headers middleware
  static securityHeaders(res: Response): void {
    res.setHeader('X-Content-Type-Options', 'nosniff');
    res.setHeader('X-Frame-Options', 'DENY');
    res.setHeader('X-XSS-Protection', '1; mode=block');
    res.setHeader('Referrer-Policy', 'strict-origin-when-cross-origin');
    res.setHeader(
      'Content-Security-Policy',
      [
        "default-src 'self'",
        "script-src 'self' 'unsafe-inline' 'unsafe-eval'",
        "style-src 'self' 'unsafe-inline'",
        "img-src 'self' data: https:",
        "font-src 'self' data:",
        "connect-src 'self' wss:",
        "frame-ancestors 'none'",
      ].join('; ')
    );
    res.setHeader('Permissions-Policy', 'camera=(), microphone=(), geolocation=()');
  }

  // Cookie security
  static secureCookieOptions: any = {
    httpOnly: true,
    secure: true,
    sameSite: 'strict' as const,
    maxAge: 24 * 60 * 60 * 1000, // 24 hours
    path: '/',
  };

  // Input validation
  static validateInput(input: any, schema: any): { valid: boolean; errors: string[] } {
    const errors: string[] = [];

    // Check for script tags
    const scriptPattern = /<script[^>]*>[\s\S]*?<\/script>/gi;
    const eventPattern = /\s*on\w+\s*=/gi;

    if (typeof input === 'string') {
      if (scriptPattern.test(input)) {
        errors.push('Script tags are not allowed');
      }
      if (eventPattern.test(input)) {
        errors.push('Event handlers are not allowed');
      }
    }

    return {
      valid: errors.length === 0,
      errors,
    };
  }
}

// Express middleware
export function xssProtectionMiddleware(req: any, res: any, next: NextFunction): void {
  XSSProtection.securityHeaders(res);
  next();
}
```

---

### 10. CI/CD

#### 10.1 GitHub Actions Pipeline

```yaml
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/.github/workflows/ci.yml
name: CI/CD Pipeline

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  NODE_VERSION: '20'
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  # Lint and Type Check
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}
          cache: 'npm'
      
      - name: Install dependencies
        run: npm ci
      
      - name: Run ESLint
        run: npm run lint
      
      - name: Run Prettier check
        run: npm run format:check

  # Type Checking
  typecheck:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}
          cache: 'npm'
      
      - name: Install dependencies
        run: npm ci
      
      - name: Run TypeScript
        run: npm run typecheck

  # Unit Tests
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
          POSTGRES_DB: test
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      
      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}
          cache: 'npm'
      
      - name: Install dependencies
        run: npm ci
      
      - name: Run unit tests
        run: npm run test:unit
        env:
          DATABASE_URL: postgresql://test:test@localhost:5432/test
          REDIS_URL: redis://localhost:6379
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3

  # E2E Tests
  e2e:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
          POSTGRES_DB: test
        ports:
          - 5432:5432
      
      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379

    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}
          cache: 'npm'
      
      - name: Install dependencies
        run: npm ci
      
      - name: Install Playwright
        run: npx playwright install --with-deps
      
      - name: Build application
        run: npm run build
      
      - name: Start test servers
        run: |
          npm run test:start &
          sleep 30
      
      - name: Run E2E tests
        run: npm run test:e2e
        env:
          BASE_URL: http://localhost:3000
      
      - name: Upload test results
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: playwright-report
          path: playwright-report/
          retention-days: 30

  # Build and Push Docker Images
  build:
    runs-on: ubuntu-latest
    needs: [lint, typecheck, test]
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    
    permissions:
      contents: read
      packages: write

    steps:
      - uses: actions/checkout@v4
      
      - name: Log in to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      
      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=sha
            type=raw,value=latest
            type=raw,value={{date 'YYYYMMDD'}}-{{sha}}
      
      - name: Build and push Tiptap app
        uses: docker/build-push-action@v5
        with:
          context: ./apps/tiptap-app
          file: ./docker/Dockerfile.tiptap
          push: true
          tags: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}-tiptap:latest,${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}-tiptap:${{ steps.meta.outputs.tags }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
      
      - name: Build and push Hocuspocus server
        uses: docker/build-push-action@v5
        with:
          context: ./apps/hocuspocus-server
          file: ./docker/Dockerfile.hocuspocus
          push: true
          tags: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}-hocuspocus:latest,${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}-hocuspocus:${{ steps.meta.outputs.tags }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  # Deploy to Staging
  deploy-staging:
    runs-on: ubuntu-latest
    needs: build
    environment: staging
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup kubectl
        uses: azure/k8s-set-context@v3
        with:
          method: kubeconfig
          kubeconfig: ${{ secrets.KUBE_CONFIG_STAGING }}
      
      - name: Deploy to Kubernetes
        run: |
          kubectl set image deployment/tiptap-app tiptap-app=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}-tiptap:${{ github.sha }} -n webeditors-staging
          kubectl set image deployment/hocuspocus hocuspocus=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}-hocuspocus:${{ github.sha }} -n webeditors-staging
      
      - name: Wait for rollout
        run: |
          kubectl rollout status deployment/tiptap-app -n webeditors-staging
          kubectl rollout status deployment/hocuspocus -n webeditors-staging
      
      - name: Run smoke tests
        run: npm run test:smoke
        env:
          BASE_URL: https://staging.editor.example.com

  # Deploy to Production
  deploy-production:
    runs-on: ubuntu-latest
    needs: deploy-staging
    environment: production
    if: github.ref == 'refs/heads/main'
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup kubectl
        uses: azure/k8s-set-context@v3
        with:
          method: kubeconfig
          kubeconfig: ${{ secrets.KUBE_CONFIG_PRODUCTION }}
      
      - name: Deploy to Kubernetes
        run: |
          kubectl set image deployment/tiptap-app tiptap-app=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}-tiptap:${{ github.sha }} -n webeditors-production
          kubectl set image deployment/hocuspocus hocuspocus=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}-hocuspocus:${{ github.sha }} -n webeditors-production
      
      - name: Wait for rollout
        run: |
          kubectl rollout status deployment/tiptap-app -n webeditors-production
          kubectl rollout status deployment/hocuspocus -n webeditors-production
      
      - name: Notify deployment
        uses: slackapi/slack-github-action@v1
        with:
          payload: |
            {
              "text": "Deployment completed: ${{ github.sha }}"
            }
        env:
          SLACK_WEBHOOK_URL: ${{ secrets.SLACK_WEBHOOK }}
```

#### 10.2 Playwright E2E Tests

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/e2e/editor.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Tiptap Editor', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should load editor', async ({ page }) => {
    await expect(page.locator('.ProseMirror')).toBeVisible();
  });

  test('should format text as bold', async ({ page }) => {
    const editor = page.locator('.ProseMirror');
    await editor.click();
    await page.keyboard.type('Hello World');
    
    await page.keyboard.press('ControlOrMeta+a');
    await page.click('[data-testid="bold-button"]');
    
    const boldText = editor.locator('strong');
    await expect(boldText).toHaveText('Hello World');
  });

  test('should create a list', async ({ page }) => {
    const editor = page.locator('.ProseMirror');
    await editor.click();
    
    await page.click('[data-testid="bullet-list-button"]');
    await page.keyboard.type('Item 1');
    await page.keyboard.press('Enter');
    await page.keyboard.type('Item 2');
    
    const listItems = editor.locator('li');
    await expect(listItems).toHaveCount(2);
  });
});

test.describe('Collaborative Editing', () => {
  test('should sync between two clients', async ({ page, context }) => {
    const page1 = page;
    const page2 = await context.newPage();

    await page1.goto('/document/test-123');
    await page2.goto('/document/test-123');

    // Wait for both pages to connect
    await page1.waitForSelector('[data-testid="connected"]');
    await page2.waitForSelector('[data-testid="connected"]');

    // Type on page1
    const editor1 = page1.locator('.ProseMirror');
    await editor1.click();
    await page1.keyboard.type('Hello from page 1');

    // Verify text appears on page2
    const editor2 = page2.locator('.ProseMirror');
    await expect(editor2).toContainText('Hello from page 1');

    // Type on page2
    await editor2.click();
    await page2.keyboard.press('End');
    await page2.keyboard.type(' and page 2');

    // Verify sync back to page1
    await expect(editor1).toContainText('Hello from page 1 and page 2');
  });

  test('should show other users cursors', async ({ page, context }) => {
    const page1 = page;
    const page2 = await context.newPage();

    await page1.goto('/document/test-456');
    await page2.goto('/document/test-456');

    // Wait for connection
    await page1.waitForSelector('[data-testid="connected"]');
    await page2.waitForSelector('[data-testid="connected"]');

    // Move cursor on page2
    const editor2 = page2.locator('.ProseMirror');
    await editor2.click();

    // Should see remote cursor on page1
    const remoteCursor = page1.locator('[data-testid="remote-cursor"]');
    await expect(remoteCursor).toBeVisible();
  });
});
```

```typescript
// /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/e2e/tldraw.spec.ts
import { test, expect } from '@playwright/test';

test.describe('tldraw Canvas', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/canvas');
  });

  test('should load canvas', async ({ page }) => {
    await expect(page.locator('[data-testid="canvas"]')).toBeVisible();
  });

  test('should draw a rectangle', async ({ page }) => {
    // Select rectangle tool
    await page.click('[data-testid="rectangle-tool"]');
    
    // Draw on canvas
    const canvas = page.locator('[data-testid="canvas"]');
    const box = await canvas.boundingBox()!;
    
    await page.mouse.move(box.x + 50, box.y + 50);
    await page.mouse.down();
    await page.mouse.move(box.x + 150, box.y + 100);
    await page.mouse.up();
    
    // Verify shape exists
    const shapes = page.locator('[data-testid="shape"]');
    await expect(shapes).toHaveCount(1);
  });

  test('multiplayer: should see other users shapes', async ({ page, context }) => {
    const page1 = page;
    const page2 = await context.newPage();

    await page1.goto('/canvas/room-123');
    await page2.goto('/canvas/room-123');

    // Wait for both to connect
    await page1.waitForSelector('[data-testid="connected"]');
    await page2.waitForSelector('[data-testid="connected"]');

    // Draw on page1
    await page1.click('[data-testid="rectangle-tool"]');
    const canvas1 = page1.locator('[data-testid="canvas"]');
    await canvas1.click({ position: { x: 100, y: 100 } });
    
    // Should appear on page2
    const shapes2 = page2.locator('[data-testid="shape"]');
    await expect(shapes2).toHaveCount(1);
  });
});
```

#### 10.3 Deployment Automation

```bash
#!/bin/bash
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/scripts/deploy.sh

set -e

ENVIRONMENT=${1:-staging}
VERSION=${2:-latest}

echo "Deploying to $ENVIRONMENT with version $VERSION"

# Build and tag images
docker build -t webeditors/tiptap-app:$VERSION -f docker/Dockerfile.tiptap apps/tiptap-app
docker build -t webeditors/hocuspocus:$VERSION -f docker/Dockerfile.hocuspocus apps/hocuspocus-server

# Push to registry
docker push webeditors/tiptap-app:$VERSION
docker push webeditors/hocuspocus:$VERSION

# Update Kubernetes deployment
kubectl set image deployment/tiptap-app tiptap-app=webeditors/tiptap-app:$VERSION -n webeditors-$ENVIRONMENT
kubectl set image deployment/hocuspocus hocuspocus=webeditors/hocuspocus:$VERSION -n webeditors-$ENVIRONMENT

# Wait for rollout
kubectl rollout status deployment/tiptap-app -n webeditors-$ENVIRONMENT
kubectl rollout status deployment/hocuspocus -n webeditors-$ENVIRONMENT

echo "Deployment complete!"
```

```yaml
# /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors/.github/workflows/rollback.yml
name: Rollback Deployment

on:
  workflow_dispatch:
    inputs:
      environment:
        description: 'Environment to rollback'
        required: true
        default: 'staging'
        type: choice
        options:
          - staging
          - production
      version:
        description: 'Version to rollback to'
        required: true

jobs:
  rollback:
    runs-on: ubuntu-latest
    environment: ${{ github.event.inputs.environment }}
    
    steps:
      - name: Setup kubectl
        uses: azure/k8s-set-context@v3
        with:
          method: kubeconfig
          kubeconfig: ${{ secrets.KUBE_CONFIG_${{ github.event.inputs.environment | upper }} }}
      
      - name: Rollback Tiptap app
        run: |
          kubectl set image deployment/tiptap-app tiptap-app=ghcr.io/${{ github.repository }}-tiptap:${{ github.event.inputs.version }} -n webeditors-${{ github.event.inputs.environment }}
      
      - name: Rollback Hocuspocus
        run: |
          kubectl set image deployment/hocuspocus hocuspocus=ghcr.io/${{ github.repository }}-hocuspocus:${{ github.event.inputs.version }} -n webeditors-${{ github.event.inputs.environment }}
      
      - name: Wait for rollout
        run: |
          kubectl rollout status deployment/tiptap-app -n webeditors-${{ github.event.inputs.environment }}
          kubectl rollout status deployment/hocuspocus -n webeditors-${{ github.event.inputs.environment }}
```

---

## Appendix A: Environment Variables Reference

```bash
# .env.example

# Application
NODE_ENV=production
PORT=3000
HOST=0.0.0.0

# Database
DATABASE_URL=postgresql://user:password@host:5432/tiptap

# Redis
REDIS_URL=redis://host:6379
REDIS_PASSWORD=

# Hocuspocus
HOCUSPOCUS_PORT=4001
HOCUSPOCUS_SECRET=change-me-in-production

# Authentication
JWT_SECRET=change-me-in-production
JWT_EXPIRY=24h

# S3
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=
AWS_SECRET_ACCESS_KEY=
S3_BUCKET=assets

# Monitoring
SENTRY_DSN=
LOG_LEVEL=info
```

---

## Appendix B: Troubleshooting Guide

### Common Issues

**1. WebSocket Connection Failing**
```
- Check firewall rules for WebSocket port
- Verify nginx/HAProxy WebSocket configuration
- Check SSL certificate validity
- Ensure sticky sessions are enabled
```

**2. High Memory Usage**
```
- Check for memory leaks in custom extensions
- Increase container memory limits
- Enable garbage collection logging
- Consider implementing document size limits
```

**3. Slow Document Sync**
```
- Check database connection pool
- Verify Redis latency
- Review document size (consider splitting large documents)
- Check network bandwidth between nodes
```

---

*Document Version: 1.0*
*Last Updated: 2026-04-05*
