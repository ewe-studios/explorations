# Production-Grade OpenWebContainer: Deployment, Scaling, and Operations

**Project:** OpenContainer/OpenWebContainer  
**Version:** 1.0.0  
**Last Updated:** 2026-04-05

This document provides comprehensive guidance for deploying, scaling, and operating OpenWebContainer-based services in production environments. OpenWebContainer brings container-like isolation to the browser through virtual filesystems, process simulation, and QuickJS runtime execution.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Deployment Strategies](#2-deployment-strategies)
3. [Scaling Considerations](#3-scaling-considerations)
4. [Performance Optimization](#4-performance-optimization)
5. [Security Considerations](#5-security-considerations)
6. [Monitoring and Observability](#6-monitoring-and-observability)
7. [Persistence and Data Management](#7-persistence-and-data-management)
8. [Enterprise Features](#8-enterprise-features)
9. [Appendix: Complete Configurations](#9-appendix-complete-configurations)

---

## 1. Architecture Overview

### 1.1 Deployment Models

OpenWebContainer supports three primary deployment models, each with distinct characteristics:

#### Client-Side Only (Pure Browser)

```
┌─────────────────────────────────────────────────────────┐
│                    User Browser                          │
│  ┌─────────────────────────────────────────────────────┐│
│  │              OpenWebContainer Runtime                ││
│  │  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐││
│  │  │   Shell   │ │  Node.js  │ │     QuickJS         │││
│  │  │  Engine   │ │  Executor │ │     Runtime         │││
│  │  └───────────┘ └───────────┘ └─────────────────────┘││
│  │  ┌──────────────────────────────────────────────────┐││
│  │  │           Virtual Filesystem (ZenFS)             │││
│  │  │           IndexedDB + Memory Backing             │││
│  │  └──────────────────────────────────────────────────┘││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
         ▲
         │ Static Assets (WASM, JS, CSS)
         │
┌─────────────────────────────────────────────────────────┐
│                    CDN / Static Host                     │
│              (Vercel, Cloudflare, Netlify)               │
└─────────────────────────────────────────────────────────┘
```

**Characteristics:**
- Zero backend infrastructure required
- All computation happens client-side
- Offline-capable by default
- Scales infinitely (each client is their own "server")
- No server costs beyond static hosting

**Best For:**
- Developer tools and IDEs
- Educational platforms
- Interactive documentation
- Code sandboxes and playgrounds

#### Hybrid Model (Client + Backend Sync)

```
┌─────────────────────────────────────────────────────────┐
│                    User Browser                          │
│  ┌─────────────────────────────────────────────────────┐│
│  │              OpenWebContainer Runtime                ││
│  │  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐││
│  │  │   Shell   │ │  Node.js  │ │     QuickJS         │││
│  │  │  Engine   │ │  Executor │ │     Runtime         │││
│  │  └───────────┘ └───────────┘ └─────────────────────┘││
│  │  ┌──────────────────────────────────────────────────┐││
│  │  │           Virtual Filesystem (ZenFS)             │││
│  │  │      IndexedDB ←→ Sync Engine → API              │││
│  │  └──────────────────────────────────────────────────┘││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
         ▲                    ▲
         │ REST/GraphQL       │ WebSocket (Real-time)
         │                    │
┌─────────────────────────────────────────────────────────┐
│                    Backend API Layer                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │   Project    │  │    User      │  │    Sync      │  │
│  │   Service    │  │   Service    │  │   Service    │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│                      ▲                                  │
│                      │                                  │
│  ┌───────────────────▼────────────────────────────────┐ │
│  │               Database Layer                        │ │
│  │  PostgreSQL (projects, users) + S3 (snapshots)     │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

**Characteristics:**
- Client executes code, backend persists state
- Real-time collaboration possible via WebSocket
- Projects survive browser cache clears
- Selective server-side rendering available

**Best For:**
- Collaborative coding platforms
- Cloud IDEs with project persistence
- Team development environments
- SaaS coding education platforms

#### Server-Rendered (Progressive Enhancement)

```
┌─────────────────────────────────────────────────────────┐
│                    User Browser                          │
│  ┌─────────────────────────────────────────────────────┐│
│  │           Server-Rendered Initial State              ││
│  │           (HTML + Hydrated React)                    ││
│  └─────────────────────────────────────────────────────┘│
│              ▲                                           │
│              │ Hydration                                 │
│              │                                           │
│  ┌───────────▼─────────────────────────────────────────┐│
│  │              OpenWebContainer Runtime                ││
│  │  (Loads after initial paint for progressive UX)     ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
         ▲
         │ SSR + Streaming
         │
┌─────────────────────────────────────────────────────────┐
│                 Edge/Server Runtime                      │
│  ┌─────────────────────────────────────────────────────┐│
│  │  Next.js / Remix / Hono (Edge Functions)            ││
│  │  - Pre-renders file tree                            ││
│  │  - Streams shell output                             ││
│  │  - Handles initial auth                             ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

**Characteristics:**
- Fast initial page load (SEO-friendly)
- Progressive enhancement as WASM loads
- Best perceived performance
- Graceful degradation

**Best For:**
- Public-facing code demos
- SEO-sensitive documentation
- Marketing pages with interactive elements

### 1.2 CDN Distribution Strategy

Optimal CDN configuration for OpenWebContainer assets:

```
┌────────────────────────────────────────────────────────────┐
│                    Global CDN Network                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Edge PoP    │  │  Edge PoP    │  │  Edge PoP    │      │
│  │  (US-East)   │  │  (EU-West)   │  │  (APAC)      │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Edge PoP    │  │  Edge PoP    │  │  Edge PoP    │      │
│  │  (US-West)   │  │  (SA-East)   │  │  (Africa)    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│                    Asset Categories                         │
├────────────────────────────────────────────────────────────┤
│  Tier 1: Critical (Cache: 1y, Immutable)                   │
│  - quickjs-emscripten.wasm (~220KB)                        │
│  - core.runtime.js (~150KB)                                │
│  - vendor.chunk.js                                         │
│                                                            │
│  Tier 2: Application (Cache: 1h, Revalidate)               │
│  - app.main.js (your application)                          │
│  - styles.css                                              │
│                                                            │
│  Tier 3: Dynamic (No Cache)                                │
│  - index.html                                              │
│  - manifest.json                                           │
│  - service-worker.js                                       │
└────────────────────────────────────────────────────────────┘
```

### 1.3 Service Worker Caching Architecture

```typescript
// Service Worker Cache Hierarchy
┌─────────────────────────────────────────────────────────┐
│                    Service Worker                        │
│  ┌─────────────────────────────────────────────────────┐│
│  │              Cache Strategy Router                   ││
│  └─────────────────────────────────────────────────────┘│
│              │         │         │                       │
│    ┌─────────▼─┐ ┌─────▼────┐ ┌──▼──────┐              │
│    │ Cache     │ │ Stale-   │ │ Network │              │
│    │ First     │ │ While-   │ │ First   │              │
│    │ (WASM)    │ │ Revalidate│ │ (HTML) │              │
│    └───────────┘ └──────────┘ └─────────┘              │
│                                                          │
│  ┌─────────────────────────────────────────────────────┐│
│  │              Runtime Cache (IndexedDB)               ││
│  │  - Project files                                     ││
│  │  - NPM packages                                      ││
│  │  - Execution history                                 ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

### 1.4 Offline-First Design Principles

OpenWebContainer is inherently offline-capable. Key considerations:

| Capability | Online | Offline | Sync Strategy |
|------------|--------|---------|---------------|
| Code Execution | Full | Full | N/A (client-side) |
| File Access | Full | Full | Sync on reconnect |
| NPM Install | Full Registry | Cached Only | Queue + Sync |
| Project Save | Immediate | IndexedDB | Background Sync |
| Collaboration | Real-time | Local Only | CRDT Resolution |

```typescript
// Offline State Detection Pattern
class OfflineManager {
    private isOnline: boolean = navigator.onLine;
    private pendingOperations: Queue<PendingOperation> = new Queue();

    constructor() {
        window.addEventListener('online', () => this.handleOnline());
        window.addEventListener('offline', () => this.handleOffline());
    }

    private async handleOnline() {
        this.isOnline = true;
        // Process queued operations
        while (!this.pendingOperations.isEmpty()) {
            const op = this.pendingOperations.dequeue();
            await this.executeAndSync(op);
        }
    }

    private handleOffline() {
        this.isOnline = false;
        // Notify UI of offline state
    }

    async queueOperation(op: PendingOperation) {
        if (this.isOnline) {
            await this.executeAndSync(op);
        } else {
            this.pendingOperations.enqueue(op);
        }
    }
}
```

---

## 2. Deployment Strategies

### 2.1 Static Hosting (Vercel, Netlify, Cloudflare Pages)

#### Vercel Deployment

**vercel.json:**
```json
{
  "$schema": "https://openapi.vercel.sh/vercel.json",
  "framework": "vite",
  "trailingSlash": true,
  "headers": [
    {
      "source": "/assets/(.*)\\.(wasm|js|css)",
      "headers": [
        {
          "key": "Cache-Control",
          "value": "public, max-age=31536000, immutable"
        },
        {
          "key": "Content-Type",
          "value": "application/wasm"
        }
      ]
    },
    {
      "source": "/(.*)",
      "headers": [
        {
          "key": "X-Content-Type-Options",
          "value": "nosniff"
        },
        {
          "key": "X-Frame-Options",
          "value": "DENY"
        },
        {
          "key": "X-XSS-Protection",
          "value": "1; mode=block"
        }
      ]
    }
  ],
  "rewrites": [
    {
      "source": "/:path*",
      "destination": "/index.html"
    }
  ]
}
```

**vite.config.ts (Production):**
```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { VitePWA } from 'vite-plugin-pwa';

export default defineConfig({
  plugins: [
    react(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['favicon.ico', 'robots.txt', 'apple-touch-icon.png'],
      workbox: {
        globPatterns: ['**/*.{js,css,html,wasm,ico,png,svg}'],
        runtimeCaching: [
          {
            urlPattern: /^https:\/\/unpkg\.com\/.*/i,
            handler: 'CacheFirst',
            options: {
              cacheName: 'npm-packages',
              expiration: {
                maxEntries: 100,
                maxAgeSeconds: 60 * 60 * 24 * 30 // 30 days
              },
              cacheableResponse: {
                statuses: [0, 200]
              }
            }
          },
          {
            urlPattern: /\.(wasm)$/,
            handler: 'CacheFirst',
            options: {
              cacheName: 'wasm-runtime',
              expiration: {
                maxEntries: 10,
                maxAgeSeconds: 60 * 60 * 24 * 365 // 1 year
              }
            }
          }
        ]
      },
      manifest: {
        name: 'OpenWebContainer IDE',
        short_name: 'OWC IDE',
        description: 'Browser-based container runtime',
        theme_color: '#1a1a2e',
        background_color: '#1a1a2e',
        display: 'standalone',
        orientation: 'landscape',
        icons: [
          {
            src: 'pwa-192x192.png',
            sizes: '192x192',
            type: 'image/png'
          },
          {
            src: 'pwa-512x512.png',
            sizes: '512x512',
            type: 'image/png'
          }
        ]
      }
    })
  ],
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          'quickjs-runtime': ['quickjs-emscripten'],
          'zenfs-core': ['@zenfs/core'],
          'vendor': ['react', 'react-dom', 'xterm']
        }
      }
    },
    target: 'esnext',
    minify: 'terser',
    terserOptions: {
      compress: {
        drop_console: true,
        drop_debugger: true
      }
    }
  }
});
```

#### Cloudflare Pages Deployment

**wrangler.toml:**
```toml
name = "openwebcontainer-ide"
compatibility_date = "2024-01-01"
pages_build_output_dir = "./dist"

# Asset configuration
[assets]
directory = "./dist"
binding = "ASSETS"

# Cache rules
[[rules]]
type = "Wasm"
globs = ["**/*.wasm"]
fallthrough = true

# Environment variables
[vars]
ENVIRONMENT = "production"
LOG_LEVEL = "info"

# Headers
[[headers]]
for = "/*"
[headers.values]
X-Frame-Options = "DENY"
X-Content-Type-Options = "nosniff"
Referrer-Policy = "strict-origin-when-cross-origin"

[[headers]]
for = "/assets/*.wasm"
[headers.values]
Cache-Control = "public, max-age=31536000, immutable"
Content-Type = "application/wasm"

[[headers]]
for = "/assets/*.js"
[headers.values]
Cache-Control = "public, max-age=31536000, immutable"
```

### 2.2 Docker Container Deployment

For hybrid deployments requiring backend services:

**Dockerfile (Multi-stage Build):**
```dockerfile
# =============================================================================
# Stage 1: Build
# =============================================================================
FROM node:20-alpine AS builder

WORKDIR /app

# Install pnpm
RUN corepack enable && corepack prepare pnpm@8 --activate

# Copy package files
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY packages/ ./packages/
COPY apps/ ./apps/
COPY tsconfig.base.json ./

# Install dependencies
RUN pnpm install --frozen-lockfile

# Build all packages
RUN pnpm build

# =============================================================================
# Stage 2: Production Runtime
# =============================================================================
FROM node:20-alpine AS production

WORKDIR /app

# Install dumb-init for proper signal handling
RUN apk add --no-cache dumb-init

# Create non-root user
RUN addgroup -g 1001 -S appgroup && \
    adduser -u 1001 -S appuser -G appgroup

# Copy built assets
COPY --from=builder --chown=appuser:appgroup /app/apps/playground/dist ./dist
COPY --from=builder --chown=appuser:appgroup /app/apps/playground/package.json ./

# Install production dependencies only
RUN npm pkg set scripts.start="npx serve dist" && \
    npm install --production && \
    npm cache clean --force

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/health || exit 1

# Start with dumb-init for signal handling
ENTRYPOINT ["/usr/bin/dumb-init", "--"]
CMD ["npm", "start"]
```

**Docker Compose (Full Stack):**
```yaml
version: '3.8'

services:
  # Frontend (OpenWebContainer SPA)
  frontend:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=production
      - API_URL=http://api:4000
    depends_on:
      - api
    networks:
      - owc-network
    restart: unless-stopped

  # Backend API (Hybrid sync)
  api:
    build:
      context: ./backend
      dockerfile: Dockerfile
    ports:
      - "4000:4000"
    environment:
      - NODE_ENV=production
      - DATABASE_URL=postgresql://user:pass@postgres:5432/openwebcontainer
      - REDIS_URL=redis://redis:6379
      - JWT_SECRET=${JWT_SECRET}
    depends_on:
      - postgres
      - redis
    networks:
      - owc-network
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:4000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  # PostgreSQL Database
  postgres:
    image: postgres:15-alpine
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./backend/init.sql:/docker-entrypoint-initdb.d/init.sql
    environment:
      - POSTGRES_DB=openwebcontainer
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
    networks:
      - owc-network
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U user -d openwebcontainer"]
      interval: 10s
      timeout: 5s
      retries: 5

  # Redis (Session cache, real-time sync)
  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis-data:/data
    networks:
      - owc-network
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

  # Nginx Reverse Proxy
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ./nginx/ssl:/etc/nginx/ssl:ro
    depends_on:
      - frontend
      - api
    networks:
      - owc-network
    restart: unless-stopped

volumes:
  postgres-data:
  redis-data:

networks:
  owc-network:
    driver: bridge
```

### 2.3 Kubernetes Deployments

Complete Kubernetes manifests for production deployment:

**namespace.yaml:**
```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: openwebcontainer
  labels:
    name: openwebcontainer
    environment: production
```

**configmap.yaml:**
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: owc-config
  namespace: openwebcontainer
data:
  NODE_ENV: "production"
  LOG_LEVEL: "info"
  CORS_ORIGIN: "https://ide.openwebcontainer.io"
  SESSION_TIMEOUT: "3600"
  MAX_WORKSPACE_SIZE_MB: "500"
  ENABLE_COLLABORATION: "true"
  TELEMETRY_ENDPOINT: "https://telemetry.openwebcontainer.io"
```

**secret.yaml:**
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: owc-secrets
  namespace: openwebcontainer
type: Opaque
stringData:
  DATABASE_URL: "postgresql://user:password@postgres:5432/openwebcontainer"
  REDIS_URL: "redis://redis:6379"
  JWT_SECRET: "change-me-in-production"
  ENCRYPTION_KEY: "32-byte-encryption-key-here"
```

**frontend-deployment.yaml:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: owc-frontend
  namespace: openwebcontainer
  labels:
    app: owc-frontend
    version: v1.0.0
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: owc-frontend
  template:
    metadata:
      labels:
        app: owc-frontend
        version: v1.0.0
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 1001
        fsGroup: 1001
      containers:
        - name: frontend
          image: owc-frontend:1.0.0
          imagePullPolicy: Always
          ports:
            - name: http
              containerPort: 3000
              protocol: TCP
          envFrom:
            - configMapRef:
                name: owc-config
          resources:
            requests:
              cpu: 100m
              memory: 128Mi
            limits:
              cpu: 500m
              memory: 512Mi
          livenessProbe:
            httpGet:
              path: /health
              port: http
            initialDelaySeconds: 10
            periodSeconds: 30
            timeoutSeconds: 5
            failureThreshold: 3
          readinessProbe:
            httpGet:
              path: /ready
              port: http
            initialDelaySeconds: 5
            periodSeconds: 10
            timeoutSeconds: 3
            failureThreshold: 3
          securityContext:
            allowPrivilegeEscalation: false
            capabilities:
              drop:
                - ALL
            readOnlyRootFilesystem: true
            runAsNonRoot: true
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchLabels:
                    app: owc-frontend
                topologyKey: kubernetes.io/hostname
      topologySpreadConstraints:
        - maxSkew: 1
          topologyKey: topology.kubernetes.io/zone
          whenUnsatisfiable: ScheduleAnyway
          labelSelector:
            matchLabels:
              app: owc-frontend
```

**api-deployment.yaml:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: owc-api
  namespace: openwebcontainer
  labels:
    app: owc-api
    version: v1.0.0
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 2
      maxUnavailable: 1
  selector:
    matchLabels:
      app: owc-api
  template:
    metadata:
      labels:
        app: owc-api
        version: v1.0.0
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 1001
        fsGroup: 1001
      containers:
        - name: api
          image: owc-api:1.0.0
          imagePullPolicy: Always
          ports:
            - name: http
              containerPort: 4000
              protocol: TCP
          envFrom:
            - configMapRef:
                name: owc-config
            - secretRef:
                name: owc-secrets
          resources:
            requests:
              cpu: 200m
              memory: 256Mi
            limits:
              cpu: 1000m
              memory: 1Gi
          livenessProbe:
            httpGet:
              path: /health
              port: http
            initialDelaySeconds: 15
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /ready
              port: http
            initialDelaySeconds: 10
            periodSeconds: 10
          securityContext:
            allowPrivilegeEscalation: false
            capabilities:
              drop:
                - ALL
            readOnlyRootFilesystem: false
```

**hpa.yaml (Horizontal Pod Autoscaler):**
```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: owc-frontend-hpa
  namespace: openwebcontainer
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: owc-frontend
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
      stabilizationWindowSeconds: 0
      policies:
        - type: Percent
          value: 100
          periodSeconds: 15
        - type: Pods
          value: 4
          periodSeconds: 15
      selectPolicy: Max
```

**ingress.yaml:**
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: owc-ingress
  namespace: openwebcontainer
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/proxy-body-size: "50m"
    nginx.ingress.kubernetes.io/proxy-connect-timeout: "60"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "60"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "60"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/configuration-snippet: |
      add_header X-Frame-Options "DENY" always;
      add_header X-Content-Type-Options "nosniff" always;
      add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self' wss: https:; worker-src 'self' blob:; frame-src 'none';" always;
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - ide.openwebcontainer.io
        - api.openwebcontainer.io
      secretName: owc-tls-secret
  rules:
    - host: ide.openwebcontainer.io
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: owc-frontend
                port:
                  name: http
    - host: api.openwebcontainer.io
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: owc-api
                port:
                  name: http
```

### 2.4 Edge Computing (Cloudflare Workers, Deno Deploy)

#### Cloudflare Worker for Edge Caching

**worker.ts:**
```typescript
// Edge worker for OpenWebContainer asset optimization
export interface Env {
  ASSETS: Fetcher;
  KV_CACHE: KVNamespace;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const cache = caches.default;

    // Serve static assets from KV cache
    if (url.pathname.startsWith('/assets/')) {
      const cached = await env.KV_CACHE.get(url.pathname);
      if (cached) {
        return new Response(cached, {
          headers: {
            'Content-Type': getContentType(url.pathname),
            'Cache-Control': 'public, max-age=31536000, immutable',
          },
        });
      }
    }

    // Check response cache for HTML
    if (url.pathname === '/' || url.pathname.endsWith('.html')) {
      const cachedResponse = await cache.match(request);
      if (cachedResponse) {
        return cachedResponse;
      }
    }

    // Forward to origin
    const response = await env.ASSETS.fetch(request);

    // Cache successful responses
    if (response.ok) {
      const responseToCache = response.clone();
      
      if (url.pathname.startsWith('/assets/')) {
        // Cache in KV for WASM/JS
        const body = await response.clone().text();
        await env.KV_CACHE.put(url.pathname, body, {
          expirationTtl: url.pathname.endsWith('.wasm') ? 31536000 : 86400
        });
      } else {
        // Cache response for HTML
        await cache.put(request, responseToCache);
      }
    }

    return response;
  },
};

function getContentType(pathname: string): string {
  if (pathname.endsWith('.wasm')) return 'application/wasm';
  if (pathname.endsWith('.js')) return 'application/javascript';
  if (pathname.endsWith('.css')) return 'text/css';
  if (pathname.endsWith('.html')) return 'text/html';
  return 'application/octet-stream';
}
```

**wrangler.toml:**
```toml
name = "openwebcontainer-edge"
main = "src/worker.ts"
compatibility_date = "2024-01-01"

# Bindings
[[kv_namespaces]]
binding = "KV_CACHE"
id = "your-kv-namespace-id"
preview_id = "your-preview-kv-namespace-id"

# Assets (for serving static files)
[assets]
directory = "./dist"
binding = "ASSETS"

# Environment
[env.production]
route = "ide.openwebcontainer.io/*"
zone_id = "your-zone-id"
```

---

## 3. Scaling Considerations

### 3.1 Client-Side Scaling Advantage

The primary scaling advantage of OpenWebContainer is that **computation scales with users**:

```
Traditional Backend Model:
  Users ──► Server (CPU/Memory bottleneck)
  1000 users = 1000x server load

OpenWebContainer Model:
  Users ──► Their Own Browsers
  1000 users = 1000 independent runtimes
  Server load = Static assets only
```

**Scaling Characteristics:**

| Aspect | Traditional | OpenWebContainer |
|--------|-------------|------------------|
| Compute Cost | Linear with users | Fixed (CDN only) |
| Memory Cost | Linear with users | Fixed |
| Scaling Action | Add servers | None required |
| Bottleneck | Server CPU | CDN bandwidth |
| Cost at 10K users | $$$$ | $ |

### 3.2 WASM Bundle Optimization

#### Bundle Size Targets

| Asset | Target | Gzipped | Loading Priority |
|-------|--------|---------|------------------|
| quickjs-emscripten.wasm | <250KB | <80KB | Critical |
| core.runtime.js | <200KB | <70KB | Critical |
| zenfs-core.js | <150KB | <50KB | High |
| shell-engine.js | <100KB | <35KB | Medium |
| ui-components.js | <300KB | <100KB | Low |

#### Code Splitting Strategy

```typescript
// vite.config.ts - Advanced code splitting
export default defineConfig({
  build: {
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          // WASM runtime - always loaded first
          if (id.includes('quickjs-emscripten')) {
            return 'runtime-wasm';
          }
          
          // Core container logic
          if (id.includes('packages/core')) {
            return 'core-runtime';
          }
          
          // Filesystem
          if (id.includes('zenfs')) {
            return 'filesystem';
          }
          
          // Shell commands
          if (id.includes('packages/core/src/shell')) {
            return 'shell-engine';
          }
          
          // UI components - lazy loaded
          if (id.includes('components/')) {
            return 'ui-components';
          }
          
          // Terminal
          if (id.includes('xterm')) {
            return 'terminal';
          }
        }
      }
    },
    // Enable advanced tree shaking
    treeshake: {
      moduleSideEffects: false,
      propertyReadSideEffects: false,
    }
  }
});
```

#### Lazy Loading Pattern

```typescript
// Lazy load heavy components
const QuickJSRuntime = lazy(() => 
  import('./runtime/QuickJSRuntime')
);

const FileSystem = lazy(() => 
  import('./filesystem/FileSystem')
);

const ShellEngine = lazy(() => 
  import('./shell/ShellEngine')
);

// Preload on idle
if ('requestIdleCallback' in window) {
  requestIdleCallback(() => {
    import('./runtime/QuickJSRuntime');
    import('./filesystem/FileSystem');
  });
}

// Prefetch based on user behavior
function prefetchOnHover(element: HTMLElement, modulePath: string) {
  element.addEventListener('mouseenter', () => {
    if ('connection' in navigator && 
        (navigator.connection as NetworkInformation).saveData === false) {
      import(modulePath);
    }
  }, { once: true });
}
```

### 3.3 Memory Management

```typescript
// Memory monitoring and cleanup
class MemoryManager {
  private readonly MEMORY_LIMIT_MB = 500;
  private readonly GC_THRESHOLD_MB = 400;

  async checkMemoryUsage(): Promise<MemoryStatus> {
    const performance = (window as any).performance;
    const memory = (performance as any).memory;

    if (!memory) {
      return { used: 0, total: this.MEMORY_LIMIT_MB };
    }

    return {
      used: Math.round(memory.usedJSHeapSize / 1048576),
      total: Math.round(memory.jsHeapSizeLimit / 1048576)
    };
  }

  async garbageCollect(): Promise<void> {
    const usage = await this.checkMemoryUsage();
    
    if (usage.used > this.GC_THRESHOLD_MB) {
      // Clear IndexedDB caches
      await this.clearIdleCaches();
      
      // Dispose unused QuickJS contexts
      this.disposeIdleContexts();
      
      // Clear service worker caches
      await this.pruneServiceWorkerCaches();
    }
  }

  private async clearIdleCaches(): Promise<void> {
    // Clear caches for projects not accessed in 1 hour
    const oneHourAgo = Date.now() - 3600000;
    const idleProjects = await this.getIdleProjects(oneHourAgo);
    
    for (const project of idleProjects) {
      await this.clearProjectCache(project.id);
    }
  }

  private disposeIdleContexts(): void {
    // Dispose QuickJS contexts not used in 30 minutes
    QuickJSContextRegistry.disposeIdle(1800000);
  }
}
```

---

## 4. Performance Optimization

### 4.1 WASM Bundle Size Reduction

#### Build Optimization Pipeline

```bash
# WASM optimization pipeline
#!/bin/bash

# 1. Build QuickJS with minimal features
emcc quickjs.c \
  -s WASM=1 \
  -s MODULARIZE=1 \
  -s EXPORT_NAME='createQuickJS' \
  -s ENVIRONMENT='web' \
  -s ALLOW_MEMORY_GROWTH=1 \
  -s INITIAL_MEMORY=10MB \
  -s MAXIMUM_MEMORY=256MB \
  --closure 1 \
  -O3 \
  -o quickjs.js

# 2. Optimize WASM binary
wasm-opt quickjs.wasm -O3 -o quickjs.opt.wasm

# 3. Strip debug symbols
wasm-strip quickjs.opt.wasm

# 4. Compress with brotli
brotli -q 11 quickjs.opt.wasm -o quickjs.wasm.br

# Report sizes
echo "Original: $(stat -f%z quickjs.wasm) bytes"
echo "Optimized: $(stat -f%z quickjs.opt.wasm) bytes"
echo "Brotli: $(stat -f%z quickjs.wasm.br) bytes"
```

#### Vite WASM Plugin

```typescript
// vite.config.ts - WASM optimization
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [
    wasm(),
    topLevelAwait({
      promiseExportName: '__tla',
      promiseImportName: i => `__tla_${i}`
    })
  ],
  optimizeDeps: {
    exclude: ['quickjs-emscripten']
  },
  build: {
    target: 'esnext',
    assetsInlineLimit: 0, // Don't inline WASM
    rollupOptions: {
      output: {
        assetFileNames: (assetInfo) => {
          if (assetInfo.name?.endsWith('.wasm')) {
            return 'assets/[name]-[hash][extname]';
          }
          return 'assets/[name]-[hash][extname]';
        }
      }
    }
  }
});
```

### 4.2 QuickJS WASM Startup Optimization

```typescript
// Optimized QuickJS initialization
class QuickJSInitializer {
  private static instance: QuickJSInitializer;
  private quickJS: QuickJSAsyncWASMModule | null = null;
  private initPromise: Promise<void> | null = null;
  private readonly CACHE_KEY = 'quickjs-wasm-cache';

  static getInstance(): QuickJSInitializer {
    if (!QuickJSInitializer.instance) {
      QuickJSInitializer.instance = new QuickJSInitializer();
    }
    return QuickJSInitializer.instance;
  }

  async initialize(): Promise<QuickJSAsyncWASMModule> {
    if (this.quickJS) {
      return this.quickJS;
    }

    // Check IndexedDB for cached WASM
    const cachedWasm = await this.getCachedWASM();
    
    if (cachedWasm) {
      this.quickJS = await this.loadFromBuffer(cachedWasm);
      return this.quickJS;
    }

    // Fallback to network fetch
    this.initPromise = this.fetchAndCacheWASM();
    await this.initPromise;
    
    return this.quickJS!;
  }

  private async getCachedWASM(): Promise<ArrayBuffer | null> {
    try {
      const db = await this.openCacheDB();
      const tx = db.transaction('wasm', 'readonly');
      const store = tx.objectStore('wasm');
      const record = await store.get('quickjs');
      return record?.buffer || null;
    } catch {
      return null;
    }
  }

  private async fetchAndCacheWASM(): Promise<void> {
    const response = await fetch('/assets/quickjs.wasm');
    const wasmBuffer = await response.arrayBuffer();

    // Cache in IndexedDB
    await this.cacheWASM(wasmBuffer);
    
    this.quickJS = await this.loadFromBuffer(wasmBuffer);
  }

  private async loadFromBuffer(wasmBuffer: ArrayBuffer): Promise<QuickJSAsyncWASMModule> {
    return newQuickJSAsyncWASMModule({
      wasmBinary: wasmBuffer
    });
  }

  private async openCacheDB(): Promise<IDBDatabase> {
    return new Promise((resolve, reject) => {
      const request = indexedDB.open('QuickJSCache', 1);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve(request.result);
      request.onupgradeneeded = (event) => {
        const db = (event.target as IDBOpenDBRequest).result;
        if (!db.objectStoreNames.contains('wasm')) {
          db.createObjectStore('wasm', { keyPath: 'name' });
        }
      };
    });
  }

  private async cacheWASM(buffer: ArrayBuffer): Promise<void> {
    const db = await this.openCacheDB();
    const tx = db.transaction('wasm', 'readwrite');
    const store = tx.objectStore('wasm');
    await store.put({ name: 'quickjs', buffer });
    await tx.done;
  }
}
```

### 4.3 Caching Strategies

#### Multi-Layer Caching Architecture

```typescript
interface CacheConfig {
  name: string;
  strategy: 'cache-first' | 'network-first' | 'stale-while-revalidate';
  maxAge: number;
  maxEntries?: number;
}

const CACHE_CONFIGS: Record<string, CacheConfig> = {
  'wasm-runtime': {
    name: 'wasm-runtime',
    strategy: 'cache-first',
    maxAge: 31536000, // 1 year
    maxEntries: 10
  },
  'npm-packages': {
    name: 'npm-packages',
    strategy: 'stale-while-revalidate',
    maxAge: 86400, // 1 day
    maxEntries: 100
  },
  'project-files': {
    name: 'project-files',
    strategy: 'network-first',
    maxAge: 3600, // 1 hour
    maxEntries: 500
  }
};

// Service Worker Implementation
const CACHE_VERSION = 'v1';
const CACHE_NAMES = {
  static: `owc-static-${CACHE_VERSION}`,
  dynamic: `owc-dynamic-${CACHE_VERSION}`
};

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAMES.static).then((cache) => {
      return cache.addAll([
        '/',
        '/index.html',
        '/assets/quickjs.wasm',
        '/assets/core.runtime.js'
      ]);
    })
  );
});

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);
  const config = getCacheConfig(url.pathname);

  if (!config) {
    return; // No caching for this request
  }

  switch (config.strategy) {
    case 'cache-first':
      event.respondWith(handleCacheFirst(event.request, config));
      break;
    case 'network-first':
      event.respondWith(handleNetworkFirst(event.request, config));
      break;
    case 'stale-while-revalidate':
      event.respondWith(handleStaleWhileRevalidate(event.request, config));
      break;
  }
});

async function handleCacheFirst(request: Request, config: CacheConfig): Promise<Response> {
  const cache = await caches.open(config.name);
  const cached = await cache.match(request);

  if (cached) {
    // Check if still fresh
    const cacheTime = await getCacheTime(cached);
    if (Date.now() - cacheTime < config.maxAge * 1000) {
      return cached;
    }
  }

  // Fetch and update cache
  const response = await fetch(request);
  if (response.ok) {
    await cache.put(request, response.clone());
  }
  
  return response;
}

async function handleNetworkFirst(request: Request, config: CacheConfig): Promise<Response> {
  const cache = await caches.open(config.name);

  try {
    const response = await fetch(request);
    if (response.ok) {
      await cache.put(request, response.clone());
    }
    return response;
  } catch {
    const cached = await cache.match(request);
    if (cached) {
      return cached;
    }
    throw new Error('Network and cache unavailable');
  }
}

async function handleStaleWhileRevalidate(
  request: Request, 
  config: CacheConfig
): Promise<Response> {
  const cache = await caches.open(config.name);
  const cached = await cache.match(request);

  // Return cached immediately
  if (cached) {
    // Revalidate in background
    fetch(request).then(async (response) => {
      if (response.ok) {
        await cache.put(request, response.clone());
      }
    }).catch(() => {});

    return cached;
  }

  // Fetch if not cached
  const response = await fetch(request);
  if (response.ok) {
    await cache.put(request, response.clone());
  }
  
  return response;
}
```

### 4.4 Resource Limits

```typescript
// Resource quota enforcement
class ResourceQuotaManager {
  private readonly DEFAULTS = {
    maxMemoryMB: 512,
    maxStorageMB: 1000,
    maxConcurrentProcesses: 10,
    maxFileSize: 50 * 1024 * 1024, // 50MB
    maxExecutionTimeMs: 30000, // 30 seconds
    maxNetworkRequestsPerMinute: 100
  };

  private quotas: Map<string, QuotaConfig> = new Map();

  setQuota(projectId: string, quota: QuotaConfig): void {
    this.quotas.set(projectId, { ...this.DEFAULTS, ...quota });
  }

  async checkMemoryLimit(projectId: string, requestedMB: number): Promise<void> {
    const quota = this.quotas.get(projectId);
    const current = await this.getCurrentMemoryUsage(projectId);
    
    if (current + requestedMB > (quota?.maxMemoryMB || this.DEFAULTS.maxMemoryMB)) {
      throw new ResourceLimitExceededError(
        `Memory limit exceeded: ${current + requestedMB}MB > ${(quota?.maxMemoryMB || this.DEFAULTS.maxMemoryMB)}MB`
      );
    }
  }

  async checkExecutionTime(
    projectId: string, 
    startTime: number
  ): Promise<void> {
    const quota = this.quotas.get(projectId);
    const elapsed = Date.now() - startTime;
    const maxTime = quota?.maxExecutionTimeMs || this.DEFAULTS.maxExecutionTimeMs;

    if (elapsed > maxTime) {
      throw new ResourceLimitExceededError(
        `Execution time limit exceeded: ${elapsed}ms > ${maxTime}ms`
      );
    }
  }

  async checkStorageLimit(projectId: string, requestedBytes: number): Promise<void> {
    const quota = this.quotas.get(projectId);
    const current = await this.getCurrentStorageUsage(projectId);
    const maxStorage = (quota?.maxStorageMB || this.DEFAULTS.maxStorageMB) * 1024 * 1024;

    if (current + requestedBytes > maxStorage) {
      throw new ResourceLimitExceededError(
        `Storage limit exceeded: ${(current + requestedBytes) / 1024 / 1024}MB > ${quota?.maxStorageMB || this.DEFAULTS.maxStorageMB}MB`
      );
    }
  }
}

class ResourceLimitExceededError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ResourceLimitExceededError';
  }
}
```

---

## 5. Security Considerations

### 5.1 Sandbox Escaping Prevention

OpenWebContainer provides multiple layers of isolation:

```
┌─────────────────────────────────────────────────────────────┐
│                    Security Layers                           │
├─────────────────────────────────────────────────────────────┤
│ Layer 1: Browser Sandbox (Built-in)                         │
│ - Same-origin policy                                        │
│ - Web Worker isolation                                      │
│ - No native filesystem access                               │
├─────────────────────────────────────────────────────────────┤
│ Layer 2: Content Security Policy                            │
│ - Restricts script sources                                  │
│ - Prevents inline script injection                          │
│ - Controls network requests                                 │
├─────────────────────────────────────────────────────────────┤
│ Layer 3: Virtual Filesystem                                 │
│ - Path traversal prevention                                 │
│ - Symlink restrictions                                      │
│ - Permission-based access                                   │
├─────────────────────────────────────────────────────────────┤
│ Layer 4: Process Isolation                                  │
│ - Separate QuickJS contexts                                 │
│ - Memory isolation                                          │
│ - Resource quotas                                           │
├─────────────────────────────────────────────────────────────┤
│ Layer 5: Network Simulation                                 │
│ - Intercepted network calls                                 │
│ - Allowed host whitelist                                    │
│ - Request rate limiting                                     │
└─────────────────────────────────────────────────────────────┘
```

#### Path Traversal Prevention

```typescript
// Secure path resolution
class SecurePathResolver {
  private readonly root: string = '/';
  private readonly blockedPatterns = [
    /\.\./g,
    /\/\//g,
    /\0/g,
    /^[^/]/,
  ];

  resolve(unsafePath: string, baseDir: string): string {
    // Normalize input
    let path = unsafePath.replace(/\\/g, '/');

    // Check for blocked patterns
    for (const pattern of this.blockedPatterns) {
      if (pattern.test(path)) {
        throw new SecurityError(`Invalid path pattern detected: ${pattern}`);
      }
    }

    // Resolve relative to base
    const resolved = path.posix.resolve(baseDir, path);

    // Ensure within root
    if (!resolved.startsWith(this.root)) {
      throw new SecurityError(
        `Path escape attempt detected: ${resolved}`
      );
    }

    return resolved;
  }

  sanitize(input: string): string {
    return input
      .replace(/[<>]/g, '') // Remove potential HTML
      .replace(/javascript:/gi, '') // Remove javascript: protocol
      .replace(/data:/gi, '') // Remove data: protocol
      .slice(0, 1024); // Max length
  }
}
```

### 5.2 Resource Exhaustion Attack Prevention

```typescript
// DoS prevention for QuickJS execution
class ExecutionGuard {
  private readonly MAX_LOOP_ITERATIONS = 1000000;
  private readonly MAX_RECURSION_DEPTH = 1000;
  private readonly MEMORY_CHECK_INTERVAL = 100;

  async executeWithGuards(
    context: QuickJSContext,
    code: string,
    options: ExecutionOptions
  ): Promise<ExecutionResult> {
    const startTime = Date.now();
    let iterationCount = 0;
    let recursionDepth = 0;

    // Set memory limit
    context.setMemoryLimit(options.maxMemoryBytes || 50 * 1024 * 1024);

    // Set execution timeout
    const timeoutId = setTimeout(() => {
      context.interruptHandler = () => true; // Force interrupt
    }, options.timeoutMs || 30000);

    // Inject guards into code
    const guardedCode = this.injectGuards(code);

    try {
      const result = context.evalCode(guardedCode, {
        interruptHandler: () => {
          iterationCount++;
          
          // Check iteration limit
          if (iterationCount > this.MAX_LOOP_ITERATIONS) {
            return true; // Interrupt
          }

          // Check memory periodically
          if (iterationCount % this.MEMORY_CHECK_INTERVAL === 0) {
            const memory = context.getMemoryUsage();
            if (memory > (options.maxMemoryBytes || 50 * 1024 * 1024)) {
              return true; // Interrupt
            }
          }

          // Check execution time
          if (Date.now() - startTime > (options.timeoutMs || 30000)) {
            return true; // Interrupt
          }

          return false; // Continue
        }
      });

      clearTimeout(timeoutId);
      return { success: true, value: result };
    } catch (error) {
      clearTimeout(timeoutId);
      return { success: false, error: error.message };
    }
  }

  private injectGuards(code: string): string {
    // Inject loop guards
    const guardedLoops = code.replace(
      /(for|while)\s*\(/g,
      '$1 (guardIteration(), '
    );

    // Inject recursion guard
    const guardedFunctions = guardedLoops.replace(
      /function\s+(\w+)\s*\(/g,
      'function $1 (guardRecursion(), '
    );

    return `
      let __iterationCount = 0;
      let __recursionDepth = 0;
      
      function guardIteration() {
        __iterationCount++;
        if (__iterationCount > ${this.MAX_LOOP_ITERATIONS}) {
          throw new Error('Maximum iteration limit exceeded');
        }
      }
      
      function guardRecursion() {
        __recursionDepth++;
        if (__recursionDepth > ${this.MAX_RECURSION_DEPTH}) {
          throw new Error('Maximum recursion depth exceeded');
        }
      }
      
      ${guardedFunctions}
    `;
  }
}
```

### 5.3 XSS Prevention

```typescript
// XSS prevention utilities
class XSSPrevention {
  // Output encoding
  static encodeHTML(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  // Attribute encoding
  static encodeAttribute(value: string): string {
    return value
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#x27;');
  }

  // URL validation
  static sanitizeURL(url: string): string | null {
    try {
      const parsed = new URL(url);
      const allowedProtocols = ['http:', 'https:', 'mailto:', 'tel:'];
      
      if (!allowedProtocols.includes(parsed.protocol)) {
        return null;
      }
      
      return parsed.toString();
    } catch {
      return null;
    }
  }

  // DOM sanitization for terminal output
  static sanitizeTerminalOutput(output: string): string {
    return output
      .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '')
      .replace(/on\w+\s*=\s*["'][^"']*["']/gi, '')
      .replace(/javascript:/gi, '')
      .replace(/data:/gi, '');
  }
}
```

### 5.4 Content Security Policy (CSP)

```nginx
# Nginx CSP Configuration
server {
    listen 443 ssl;
    server_name ide.openwebcontainer.io;

    # Strict CSP
    add_header Content-Security-Policy "
        default-src 'self';
        script-src 'self' 'wasm-unsafe-eval' https://cdn.openwebcontainer.io;
        style-src 'self' 'unsafe-inline' https://fonts.googleapis.com;
        img-src 'self' data: https: blob:;
        font-src 'self' https://fonts.gstatic.com;
        connect-src 'self' wss://api.openwebcontainer.io https://api.openwebcontainer.io;
        worker-src 'self' blob:;
        frame-src 'none';
        object-src 'none';
        base-uri 'self';
        form-action 'self';
        upgrade-insecure-requests;
        block-all-mixed-content;
    " always;

    # Additional security headers
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    add_header Permissions-Policy "geolocation=(), microphone=(), camera=()" always;
}
```

**Meta Tag Alternative:**
```html
<meta http-equiv="Content-Security-Policy" content="
    default-src 'self';
    script-src 'self' 'wasm-unsafe-eval' 'unsafe-inline';
    style-src 'self' 'unsafe-inline';
    img-src 'self' data: https: blob:;
    font-src 'self' data:;
    connect-src 'self' wss: https:;
    worker-src 'self' blob:;
    frame-src 'none';
    object-src 'none';
    base-uri 'self';
    form-action 'self';
">
```

### 5.5 Input Sanitization Pipeline

```typescript
// Complete input sanitization for shell commands
class InputSanitizer {
  private readonly DANGEROUS_COMMANDS = [
    'rm -rf', 'dd', 'mkfs', 'chmod 777',
    'curl | bash', 'wget | bash',
    'eval', 'exec', 'Function'
  ];

  sanitizeShellCommand(input: string): SanitizedCommand {
    // Step 1: Remove null bytes
    let sanitized = input.replace(/\0/g, '');

    // Step 2: Check for dangerous patterns
    for (const dangerous of this.DANGEROUS_COMMANDS) {
      if (sanitized.toLowerCase().includes(dangerous.toLowerCase())) {
        throw new SecurityError(`Dangerous command pattern detected: ${dangerous}`);
      }
    }

    // Step 3: Remove shell metacharacters that could be dangerous
    sanitized = sanitized
      .replace(/[;&|`$(){}]/g, '') // Remove command chaining
      .replace(/\\/g, '') // Remove escapes
      .slice(0, 4096); // Max length

    // Step 4: Validate against allowed command list
    const command = sanitized.split(/\s+/)[0];
    if (!this.isAllowedCommand(command)) {
      throw new SecurityError(`Command not allowed: ${command}`);
    }

    // Step 5: Sanitize arguments
    const args = sanitized.split(/\s+/).slice(1).map(arg => 
      arg.replace(/['"\\]/g, '').slice(0, 256)
    );

    return {
      command,
      args,
      original: input
    };
  }

  private isAllowedCommand(command: string): boolean {
    const allowedCommands = [
      'ls', 'cd', 'pwd', 'mkdir', 'touch',
      'cat', 'echo', 'cp', 'mv', 'rm',
      'head', 'tail', 'grep', 'find',
      'node', 'npm', 'yarn', 'pnpm'
    ];
    return allowedCommands.includes(command);
  }
}
```

---

## 6. Monitoring and Observability

### 6.1 Error Tracking (Sentry Integration)

```typescript
// Sentry integration for OpenWebContainer
import * as Sentry from '@sentry/browser';
import { BrowserTracing } from '@sentry/tracing';

Sentry.init({
  dsn: 'https://your-dsn@sentry.io/123456',
  environment: process.env.NODE_ENV,
  release: `openwebcontainer@${VERSION}`,
  
  integrations: [
    new BrowserTracing({
      routingInstrumentation: Sentry.browserTracingIntegration(),
    }),
  ],

  // Performance monitoring
  tracesSampleRate: 0.2, // 20% sampling

  // Error sampling
  sampleRate: 1.0, // Capture all errors

  // Filter out known noise
  beforeSend(event, hint) {
    // Ignore errors from browser extensions
    if (event.exception?.values?.some(v => 
      v.stacktrace?.frames?.some(f => 
        f.filename?.includes('extension://')
      )
    )) {
      return null;
    }

    // Ignore network errors from blocked requests
    if (hint.originalException instanceof TypeError &&
        hint.originalException.message.includes('Failed to fetch')) {
      return null;
    }

    // Add custom context
    event.tags = {
      ...event.tags,
      projectId: getCurrentProjectId(),
      containerState: getContainerState()
    };

    return event;
  }
});

// Custom error reporting for WASM errors
class WASMErrorReporter {
  static report(error: Error, context: WASMContext) {
    Sentry.withScope((scope) => {
      scope.setContext('wasm', {
        runtime: context.runtime,
        memoryUsage: context.getMemoryUsage(),
        stackSize: context.getStackSize()
      });
      Sentry.captureException(error);
    });
  }
}

// Performance monitoring for QuickJS execution
class ExecutionMonitor {
  async executeAndMonitor<T>(
    operation: () => Promise<T>,
    operationName: string
  ): Promise<T> {
    const transaction = Sentry.startTransaction({
      name: operationName,
      op: 'function',
    });

    const span = transaction.startChild({
      op: 'execution',
      description: operationName
    });

    try {
      const startTime = performance.now();
      const result = await operation();
      const duration = performance.now() - startTime;

      span.setMeasurement('duration_ms', duration, 'millisecond');
      span.setStatus('ok');

      // Report slow executions
      if (duration > 1000) {
        Sentry.addBreadcrumb({
          category: 'performance',
          message: `Slow execution: ${operationName} took ${duration}ms`,
          level: 'warning'
        });
      }

      return result;
    } catch (error) {
      span.setStatus('internal_error');
      throw error;
    } finally {
      span.finish();
      transaction.finish();
    }
  }
}
```

### 6.2 Performance Monitoring

```typescript
// Custom performance metrics collection
interface PerformanceMetrics {
  timestamp: number;
  wasmLoadTime: number;
  firstExecutionTime: number;
  memoryUsage: {
    heapUsed: number;
    heapTotal: number;
    external: number;
  };
  filesystemStats: {
    filesCount: number;
    totalSize: number;
  };
  executionStats: {
    totalExecutions: number;
    averageExecutionTime: number;
    errorRate: number;
  };
}

class PerformanceMonitor {
  private metrics: PerformanceMetrics[] = [];
  private readonly MAX_METRICS = 1000;

  constructor() {
    this.startCollection();
  }

  private startCollection(): void {
    // Collect metrics every 10 seconds
    setInterval(() => this.collectMetrics(), 10000);
  }

  private async collectMetrics(): Promise<void> {
    const memory = (performance as any).memory || {
      usedJSHeapSize: 0,
      totalJSHeapSize: 0,
      jsHeapSizeLimit: 0
    };

    const metric: PerformanceMetrics = {
      timestamp: Date.now(),
      wasmLoadTime: await this.getWasmLoadTime(),
      firstExecutionTime: await this.getFirstExecutionTime(),
      memoryUsage: {
        heapUsed: Math.round(memory.usedJSHeapSize / 1024),
        heapTotal: Math.round(memory.totalJSHeapSize / 1024),
        external: Math.round((memory.jsHeapSizeLimit - memory.totalJSHeapSize) / 1024)
      },
      filesystemStats: await this.getFileSystemStats(),
      executionStats: await this.getExecutionStats()
    };

    this.metrics.push(metric);
    
    // Keep only recent metrics
    if (this.metrics.length > this.MAX_METRICS) {
      this.metrics.shift();
    }

    // Send to analytics endpoint
    this.sendMetrics(metric);
  }

  private async sendMetrics(metric: PerformanceMetrics): Promise<void> {
    try {
      await fetch('/api/metrics', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(metric),
        keepalive: true
      });
    } catch {
      // Silently fail - metrics are non-critical
    }
  }

  getMetrics(): PerformanceMetrics[] {
    return this.metrics;
  }

  getAverage(key: keyof PerformanceMetrics): number {
    if (this.metrics.length === 0) return 0;
    
    const sum = this.metrics.reduce((acc, m) => acc + (m[key] as number), 0);
    return sum / this.metrics.length;
  }
}
```

### 6.3 Usage Analytics

```typescript
// Usage analytics tracking
class UsageAnalytics {
  private sessionStart: number = Date.now();
  private events: UsageEvent[] = [];

  track(event: UsageEvent): void {
    this.events.push({
      ...event,
      timestamp: Date.now(),
      sessionId: this.getSessionId()
    });

    // Flush every 10 events
    if (this.events.length >= 10) {
      this.flush();
    }
  }

  flush(): void {
    if (this.events.length === 0) return;

    const payload = {
      events: this.events,
      sessionId: this.getSessionId(),
      pageUrl: window.location.href,
      userAgent: navigator.userAgent
    };

    // Send with beacon API for reliability
    navigator.sendBeacon('/api/analytics', JSON.stringify(payload));
    
    this.events = [];
  }

  private getSessionId(): string {
    let sessionId = sessionStorage.getItem('analytics_session_id');
    if (!sessionId) {
      sessionId = crypto.randomUUID();
      sessionStorage.setItem('analytics_session_id', sessionId);
    }
    return sessionId;
  }
}

interface UsageEvent {
  type: 'command_executed' | 'file_created' | 'file_modified' | 'process_spawned';
  details: {
    command?: string;
    filePath?: string;
    processType?: 'shell' | 'node';
    duration?: number;
    success?: boolean;
  };
}

// Example usage tracking
const analytics = new UsageAnalytics();

// Track command execution
analytics.track({
  type: 'command_executed',
  details: {
    command: 'npm install',
    duration: 5234,
    success: true
  }
});

// Track file operations
analytics.track({
  type: 'file_created',
  details: {
    filePath: '/app/index.js'
  }
});
```

### 6.4 Resource Consumption Tracking

```typescript
// Resource consumption dashboard data
class ResourceTracker {
  private readonly STORAGE_KEY = 'resource_consumption';
  private data: ResourceData = {
    cpu: [],
    memory: [],
    storage: [],
    network: []
  };

  startTracking(): void {
    // Memory tracking
    setInterval(() => this.trackMemory(), 5000);
    
    // Storage tracking
    setInterval(() => this.trackStorage(), 30000);
    
    // Network tracking
    this.trackNetwork();
  }

  private trackMemory(): void {
    const memory = (performance as any).memory;
    if (!memory) return;

    this.data.memory.push({
      timestamp: Date.now(),
      used: memory.usedJSHeapSize,
      total: memory.totalJSHeapSize,
      limit: memory.jsHeapSizeLimit
    });

    // Keep last 100 data points
    if (this.data.memory.length > 100) {
      this.data.memory.shift();
    }
  }

  private async trackStorage(): Promise<void> {
    try {
      const estimate = await navigator.storage.estimate();
      this.data.storage.push({
        timestamp: Date.now(),
        usage: estimate.usage || 0,
        quota: estimate.quota || 0
      });
    } catch {
      // Storage estimation not available
    }
  }

  private trackNetwork(): void {
    if ('connection' in navigator) {
      const conn = navigator.connection as NetworkInformation;
      
      conn.addEventListener('change', () => {
        this.data.network.push({
          timestamp: Date.now(),
          downlink: conn.downlink,
          rtt: conn.rtt,
          effectiveType: conn.effectiveType,
          saveData: conn.saveData
        });
      });
    }
  }

  getResourceReport(): ResourceReport {
    const latest = {
      memory: this.data.memory[this.data.memory.length - 1],
      storage: this.data.storage[this.data.storage.length - 1],
      network: this.data.network[this.data.network.length - 1]
    };

    return {
      memoryUsagePercent: (latest.memory.used / latest.memory.limit) * 100,
      storageUsagePercent: (latest.storage.usage / latest.storage.quota) * 100,
      networkQuality: latest.network?.effectiveType || 'unknown',
      timestamp: Date.now()
    };
  }
}

interface ResourceReport {
  memoryUsagePercent: number;
  storageUsagePercent: number;
  networkQuality: string;
  timestamp: number;
}
```

---

## 7. Persistence and Data Management

### 7.1 IndexedDB for Client-Side Storage

```typescript
// IndexedDB wrapper for OpenWebContainer persistence
class ContainerStorage {
  private readonly DB_NAME = 'openwebcontainer';
  private readonly DB_VERSION = 1;
  private db: IDBDatabase | null = null;

  async initialize(): Promise<void> {
    this.db = await this.openDB();
  }

  private openDB(): Promise<IDBDatabase> {
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(this.DB_NAME, this.DB_VERSION);

      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve(request.result);

      request.onupgradeneeded = (event) => {
        const db = (event.target as IDBOpenDBRequest).result;

        // Projects store
        if (!db.objectStoreNames.contains('projects')) {
          const projects = db.createObjectStore('projects', { keyPath: 'id' });
          projects.createIndex('name', 'name', { unique: true });
          projects.createIndex('lastModified', 'lastModified', { unique: false });
        }

        // Files store (denormalized for fast access)
        if (!db.objectStoreNames.contains('files')) {
          const files = db.createObjectStore('files', { keyPath: 'path' });
          files.createIndex('projectId', 'projectId', { unique: false });
        }

        // Execution history
        if (!db.objectStoreNames.contains('history')) {
          const history = db.createObjectStore('history', { 
            keyPath: 'id', 
            autoIncrement: true 
          });
          history.createIndex('projectId', 'projectId', { unique: false });
          history.createIndex('timestamp', 'timestamp', { unique: false });
        }

        // Cached packages
        if (!db.objectStoreNames.contains('packages')) {
          const packages = db.createObjectStore('packages', { keyPath: 'name' });
          packages.createIndex('version', 'version', { unique: false });
        }
      };
    });
  }

  // Project operations
  async saveProject(project: Project): Promise<void> {
    const tx = this.db!.transaction('projects', 'readwrite');
    await tx.objectStore('projects').put({
      ...project,
      lastModified: Date.now()
    });
    await tx.done;
  }

  async getProject(id: string): Promise<Project | null> {
    return this.db!.transaction('projects', 'readonly')
      .objectStore('projects')
      .get(id);
  }

  async listProjects(): Promise<Project[]> {
    return this.db!.transaction('projects', 'readonly')
      .objectStore('projects')
      .getAll();
  }

  async deleteProject(id: string): Promise<void> {
    const tx = this.db!.transaction(['projects', 'files'], 'readwrite');
    tx.objectStore('projects').delete(id);
    
    // Cascade delete files
    const filesStore = tx.objectStore('files');
    const index = filesStore.index('projectId');
    const keys = await index.getAllKeys(id);
    for (const key of keys) {
      filesStore.delete(key);
    }
    
    await tx.done;
  }

  // File operations
  async saveFile(file: ContainerFile): Promise<void> {
    const tx = this.db!.transaction('files', 'readwrite');
    await tx.objectStore('files').put(file);
    await tx.done;
  }

  async getFile(path: string): Promise<ContainerFile | null> {
    return this.db!.transaction('files', 'readonly')
      .objectStore('files')
      .get(path);
  }

  async getFilesByProject(projectId: string): Promise<ContainerFile[]> {
    const tx = this.db!.transaction('files', 'readonly');
    const index = tx.objectStore('files').index('projectId');
    return index.getAll(projectId);
  }

  // History operations
  async addToHistory(entry: HistoryEntry): Promise<void> {
    const tx = this.db!.transaction('history', 'readwrite');
    await tx.objectStore('history').add({
      ...entry,
      timestamp: Date.now()
    });
    await tx.done;
  }

  async getHistory(projectId: string, limit: number = 100): Promise<HistoryEntry[]> {
    const tx = this.db!.transaction('history', 'readonly');
    const index = tx.objectStore('history').index('projectId');
    return index.getAll(projectId, limit);
  }

  // Bulk operations
  async exportProject(projectId: string): Promise<Blob> {
    const project = await this.getProject(projectId);
    const files = await this.getFilesByProject(projectId);

    const exportData = {
      project,
      files,
      exportedAt: Date.now()
    };

    return new Blob([JSON.stringify(exportData)], {
      type: 'application/json'
    });
  }

  async importProject(data: any): Promise<string> {
    const tx = this.db!.transaction(['projects', 'files'], 'readwrite');
    
    // Import project
    await tx.objectStore('projects').put(data.project);
    
    // Import files
    for (const file of data.files) {
      await tx.objectStore('files').put(file);
    }
    
    await tx.done;
    return data.project.id;
  }
}
```

### 7.2 Server Sync

```typescript
// Bi-directional sync between client and server
class SyncManager {
  private storage: ContainerStorage;
  private syncQueue: SyncOperation[] = [];
  private isSyncing: boolean = false;

  constructor(storage: ContainerStorage) {
    this.storage = storage;
  }

  // Queue changes for sync
  async queueSync(operation: SyncOperation): Promise<void> {
    this.syncQueue.push(operation);
    
    // Attempt sync if not already syncing
    if (!this.isSyncing && navigator.onLine) {
      await this.sync();
    }
  }

  // Sync pending operations
  async sync(): Promise<void> {
    if (this.isSyncing || this.syncQueue.length === 0) {
      return;
    }

    this.isSyncing = true;

    try {
      const batch = this.syncQueue.splice(0, 50); // Batch operations
      
      const response = await fetch('/api/sync', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ operations: batch })
      });

      if (!response.ok) {
        throw new Error('Sync failed');
      }

      const result = await response.json();
      
      // Apply server changes
      await this.applyServerChanges(result.changes);
      
      // Clear synced operations
      this.syncQueue = this.syncQueue.filter(op => 
        !batch.some(b => b.id === op.id)
      );
    } catch (error) {
      // Re-queue failed operations
      console.error('Sync failed:', error);
    } finally {
      this.isSyncing = false;
      
      // Continue syncing if there are more operations
      if (this.syncQueue.length > 0) {
        setTimeout(() => this.sync(), 1000);
      }
    }
  }

  // Pull latest changes from server
  async pullChanges(projectId: string, since: number): Promise<void> {
    const response = await fetch(`/api/projects/${projectId}/changes?since=${since}`);
    const changes = await response.json();
    
    await this.applyServerChanges(changes);
  }

  private async applyServerChanges(changes: ServerChange[]): Promise<void> {
    for (const change of changes) {
      switch (change.type) {
        case 'file_created':
        case 'file_updated':
          await this.storage.saveFile(change.file);
          break;
        case 'file_deleted':
          // Delete from IndexedDB
          const tx = this.storage.db!.transaction('files', 'readwrite');
          tx.objectStore('files').delete(change.path);
          await tx.done;
          break;
      }
    }
  }
}

// Background sync registration
if ('serviceWorker' in navigator && 'SyncManager' in window) {
  navigator.serviceWorker.ready.then(async (registration) => {
    await registration.sync.register('sync-project');
  });
}
```

### 7.3 Backup Strategies

#### Automated Backup Script

```bash
#!/bin/bash
# backup-openwebcontainer.sh
# Automated backup script for OpenWebContainer data

set -e

# Configuration
BACKUP_DIR="/var/backups/openwebcontainer"
DB_HOST="${DB_HOST:-localhost}"
DB_NAME="${DB_NAME:-openwebcontainer}"
DB_USER="${DB_USER:-backup_user}"
S3_BUCKET="${S3_BUCKET:-openwebcontainer-backups}"
RETENTION_DAYS=30
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p "${BACKUP_DIR}"

echo "Starting backup at $(date)"

# 1. PostgreSQL dump
echo "Dumping database..."
pg_dump -h "${DB_HOST}" -U "${DB_USER}" "${DB_NAME}" | \
  gzip > "${BACKUP_DIR}/db_${TIMESTAMP}.sql.gz"

# 2. Export IndexedDB snapshots from S3 (if using server-side snapshots)
echo "Exporting file snapshots..."
aws s3 sync "s3://${S3_BUCKET}/snapshots/" "${BACKUP_DIR}/snapshots_${TIMESTAMP}/"

# 3. Create manifest
cat > "${BACKUP_DIR}/manifest_${TIMESTAMP}.json" << EOF
{
  "timestamp": "${TIMESTAMP}",
  "database": "db_${TIMESTAMP}.sql.gz",
  "snapshots": "snapshots_${TIMESTAMP}/",
  "size": $(du -sb "${BACKUP_DIR}" | cut -f1),
  "hostname": "$(hostname)"
}
EOF

# 4. Upload to S3
echo "Uploading to S3..."
aws s3 sync "${BACKUP_DIR}/" "s3://${S3_BUCKET}/backups/${TIMESTAMP}/"

# 5. Cleanup old backups
echo "Cleaning up backups older than ${RETENTION_DAYS} days..."
aws s3 ls "s3://${S3_BUCKET}/backups/" | \
  while read -r line; do
    folder_date=$(echo $line | awk '{print $2}')
    if [[ -n "$folder_date" ]]; then
      folder_timestamp=$(date -d "$folder_date" +%s 2>/dev/null || echo 0)
      current_timestamp=$(date +%s)
      age_days=$(( (current_timestamp - folder_timestamp) / 86400 ))
      
      if [[ $age_days -gt $RETENTION_DAYS ]]; then
        echo "Deleting old backup: $folder_date"
        aws s3 rm "s3://${S3_BUCKET}/backups/${folder_date}/" --recursive
      fi
    fi
  done

# 6. Local cleanup
rm -rf "${BACKUP_DIR:?}"/*

echo "Backup completed at $(date)"
```

#### Kubernetes CronJob for Backups

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: owc-backup
  namespace: openwebcontainer
spec:
  schedule: "0 2 * * *" # Daily at 2 AM
  concurrencyPolicy: Forbid
  successfulJobsHistoryLimit: 3
  failedJobsHistoryLimit: 1
  jobTemplate:
    spec:
      template:
        spec:
          serviceAccountName: owc-backup-sa
          containers:
            - name: backup
              image: owc-backup:1.0.0
              env:
                - name: DB_HOST
                  value: "postgres.openwebcontainer.svc"
                - name: DB_NAME
                  value: "openwebcontainer"
                - name: DB_USER
                  valueFrom:
                    secretKeyRef:
                      name: owc-secrets
                      key: DB_USER
                - name: DB_PASSWORD
                  valueFrom:
                    secretKeyRef:
                      name: owc-secrets
                      key: DB_PASSWORD
                - name: AWS_ACCESS_KEY_ID
                  valueFrom:
                    secretKeyRef:
                      name: aws-credentials
                      key: AWS_ACCESS_KEY_ID
                - name: AWS_SECRET_ACCESS_KEY
                  valueFrom:
                    secretKeyRef:
                      name: aws-credentials
                      key: AWS_SECRET_ACCESS_KEY
              resources:
                requests:
                  cpu: 100m
                  memory: 256Mi
                limits:
                  cpu: 500m
                  memory: 512Mi
          restartPolicy: OnFailure
```

### 7.4 Migration Handling

```typescript
// Schema migration system for IndexedDB
class DatabaseMigrator {
  private readonly TARGET_VERSION = 3;

  async migrate(db: IDBDatabase, oldVersion: number): Promise<void> {
    console.log(`Migrating from version ${oldVersion} to ${this.TARGET_VERSION}`);

    if (oldVersion < 2) {
      await this.migrateToV2(db);
    }

    if (oldVersion < 3) {
      await this.migrateToV3(db);
    }
  }

  private async migrateToV2(db: IDBDatabase): Promise<void> {
    // Add new 'tags' field to projects
    const projectStore = db.transaction('projects', 'readwrite').objectStore('projects');
    const projects = await projectStore.getAll();

    for (const project of projects) {
      project.tags = project.tags || [];
      await projectStore.put(project);
    }
  }

  private async migrateToV3(db: IDBDatabase): Promise<void> {
    // Create new 'settings' store
    if (!db.objectStoreNames.contains('settings')) {
      db.createObjectStore('settings', { keyPath: 'key' });
    }

    // Migrate old config to settings store
    const configData = await this.getLegacyConfig(db);
    if (configData) {
      const settingsStore = db.transaction('settings', 'readwrite').objectStore('settings');
      for (const [key, value] of Object.entries(configData)) {
        await settingsStore.put({ key, value });
      }
    }
  }

  private async getLegacyConfig(db: IDBDatabase): Promise<any> {
    return new Promise((resolve) => {
      const tx = db.transaction('config', 'readonly');
      tx.oncomplete = () => resolve(tx.objectStore('config').getAll());
      tx.onerror = () => resolve(null);
    });
  }
}

// Usage in database initialization
const request = indexedDB.open('openwebcontainer', TARGET_VERSION);

request.onupgradeneeded = (event) => {
  const db = (event.target as IDBOpenDBRequest).result;
  const migrator = new DatabaseMigrator();
  migrator.migrate(db, event.oldVersion);
};
```

---

## 8. Enterprise Features

### 8.1 Multi-Tenant Isolation

```typescript
// Tenant isolation layer
class TenantIsolationManager {
  private tenants: Map<string, TenantConfig> = new Map();
  private storage: ContainerStorage;

  constructor(storage: ContainerStorage) {
    this.storage = storage;
  }

  registerTenant(tenant: TenantConfig): void {
    this.tenants.set(tenant.id, tenant);
  }

  // Create isolated container for tenant
  async createTenantContainer(tenantId: string): Promise<IsolatedContainer> {
    const tenant = this.tenants.get(tenantId);
    if (!tenant) {
      throw new Error(`Tenant not found: ${tenantId}`);
    }

    // Create isolated filesystem namespace
    const namespace = `tenant-${tenantId}`;
    
    // Create isolated QuickJS runtime
    const runtime = await QuickJSRuntime.create({
      memoryLimit: tenant.quota.maxMemoryMB * 1024 * 1024,
      timeout: tenant.quota.maxExecutionTimeMs
    });

    // Create isolated network context
    const network = new NetworkIsolator(tenant.allowedHosts);

    return new IsolatedContainer({
      namespace,
      runtime,
      network,
      quota: tenant.quota
    });
  }

  // Verify tenant access
  async verifyAccess(tenantId: string, resourceId: string): Promise<boolean> {
    const tenant = this.tenants.get(tenantId);
    if (!tenant) return false;

    // Check resource ownership
    const resource = await this.storage.getResource(resourceId);
    return resource?.tenantId === tenantId;
  }
}

class IsolatedContainer {
  private namespace: string;
  private runtime: QuickJSRuntime;
  private network: NetworkIsolator;
  private quota: QuotaConfig;

  constructor(options: ContainerOptions) {
    this.namespace = options.namespace;
    this.runtime = options.runtime;
    this.network = options.network;
    this.quota = options.quota;
  }

  // All operations are namespaced and quota-enforced
  async execute(code: string): Promise<ExecutionResult> {
    // Verify quota
    await this.checkQuota();

    // Execute in isolated runtime
    return this.runtime.execute(code);
  }

  async readFile(path: string): Promise<Uint8Array> {
    // Namespace the path
    const namespacedPath = `/${this.namespace}${path}`;
    return this.storage.readFile(namespacedPath);
  }

  async fetch(url: string): Promise<Response> {
    // Route through network isolator
    return this.network.fetch(url);
  }

  private async checkQuota(): Promise<void> {
    const usage = await this.getUsage();
    
    if (usage.memory > this.quota.maxMemoryMB) {
      throw new QuotaExceededError('Memory quota exceeded');
    }
    
    if (usage.storage > this.quota.maxStorageMB) {
      throw new QuotaExceededError('Storage quota exceeded');
    }
  }
}
```

### 8.2 Resource Quotas

```typescript
// Enterprise quota management
interface EnterpriseQuota {
  maxMemoryMB: number;
  maxStorageMB: number;
  maxExecutionTimeMs: number;
  maxConcurrentProcesses: number;
  maxNetworkRequestsPerMinute: number;
  maxBundleSizeKB: number;
  allowedHosts: string[];
  allowedCommands: string[];
}

const QUOTA_TIERS: Record<string, EnterpriseQuota> = {
  free: {
    maxMemoryMB: 128,
    maxStorageMB: 100,
    maxExecutionTimeMs: 10000,
    maxConcurrentProcesses: 2,
    maxNetworkRequestsPerMinute: 30,
    maxBundleSizeKB: 500,
    allowedHosts: ['*'],
    allowedCommands: ['ls', 'cd', 'pwd', 'cat', 'echo', 'node']
  },
  pro: {
    maxMemoryMB: 512,
    maxStorageMB: 1000,
    maxExecutionTimeMs: 30000,
    maxConcurrentProcesses: 10,
    maxNetworkRequestsPerMinute: 100,
    maxBundleSizeKB: 2000,
    allowedHosts: ['*'],
    allowedCommands: ['*']
  },
  enterprise: {
    maxMemoryMB: 2048,
    maxStorageMB: 10000,
    maxExecutionTimeMs: 60000,
    maxConcurrentProcesses: 50,
    maxNetworkRequestsPerMinute: 1000,
    maxBundleSizeKB: 10000,
    allowedHosts: ['*'],
    allowedCommands: ['*']
  }
};

class QuotaEnforcer {
  private quotas: Map<string, EnterpriseQuota> = new Map();

  setQuota(tenantId: string, tier: keyof typeof QUOTA_TIERS): void {
    this.quotas.set(tenantId, QUOTA_TIERS[tier]);
  }

  async enforce(operation: string, tenantId: string, context: any): Promise<void> {
    const quota = this.quotas.get(tenantId);
    if (!quota) {
      throw new Error('Quota not configured');
    }

    switch (operation) {
      case 'execute':
        if (context.executionTime > quota.maxExecutionTimeMs) {
          throw new QuotaExceededError(`Execution time limit: ${quota.maxExecutionTimeMs}ms`);
        }
        break;
      case 'allocate_memory':
        if (context.requestedMB > quota.maxMemoryMB) {
          throw new QuotaExceededError(`Memory limit: ${quota.maxMemoryMB}MB`);
        }
        break;
      case 'store_file':
        if (context.fileSize > quota.maxStorageMB * 1024 * 1024) {
          throw new QuotaExceededError(`Storage limit: ${quota.maxStorageMB}MB`);
        }
        break;
      case 'spawn_process':
        const currentProcesses = await this.countProcesses(tenantId);
        if (currentProcesses >= quota.maxConcurrentProcesses) {
          throw new QuotaExceededError(`Process limit: ${quota.maxConcurrentProcesses}`);
        }
        break;
      case 'network_request':
        if (!this.isHostAllowed(context.host, quota.allowedHosts)) {
          throw new QuotaExceededError(`Host not allowed: ${context.host}`);
        }
        break;
    }
  }

  private isHostAllowed(host: string, allowedHosts: string[]): boolean {
    for (const pattern of allowedHosts) {
      if (pattern === '*') return true;
      if (pattern.includes('*')) {
        const regex = new RegExp(pattern.replace(/\*/g, '.*'));
        if (regex.test(host)) return true;
      } else if (host === pattern) {
        return true;
      }
    }
    return false;
  }
}
```

### 8.3 Access Control

```typescript
// Role-based access control (RBAC)
enum Role {
  ADMIN = 'admin',
  DEVELOPER = 'developer',
  VIEWER = 'viewer'
}

enum Permission {
  PROJECT_READ = 'project:read',
  PROJECT_WRITE = 'project:write',
  PROJECT_DELETE = 'project:delete',
  EXECUTE_CODE = 'code:execute',
  MANAGE_MEMBERS = 'members:manage',
  VIEW_AUDIT_LOG = 'audit:read'
}

const ROLE_PERMISSIONS: Record<Role, Permission[]> = {
  [Role.ADMIN]: [
    Permission.PROJECT_READ,
    Permission.PROJECT_WRITE,
    Permission.PROJECT_DELETE,
    Permission.EXECUTE_CODE,
    Permission.MANAGE_MEMBERS,
    Permission.VIEW_AUDIT_LOG
  ],
  [Role.DEVELOPER]: [
    Permission.PROJECT_READ,
    Permission.PROJECT_WRITE,
    Permission.EXECUTE_CODE
  ],
  [Role.VIEWER]: [
    Permission.PROJECT_READ
  ]
};

class AccessControlManager {
  private memberships: Map<string, Membership[]> = new Map();

  async checkPermission(
    userId: string,
    projectId: string,
    permission: Permission
  ): Promise<boolean> {
    const memberships = this.memberships.get(projectId) || [];
    const membership = memberships.find(m => m.userId === userId);
    
    if (!membership) {
      return false;
    }

    const permissions = ROLE_PERMISSIONS[membership.role];
    return permissions.includes(permission);
  }

  async authorize(
    userId: string,
    projectId: string,
    action: string
  ): Promise<void> {
    const permission = this.actionToPermission(action);
    const hasPermission = await this.checkPermission(userId, projectId, permission);
    
    if (!hasPermission) {
      throw new AccessDeniedError(
        `User ${userId} lacks permission ${permission} for project ${projectId}`
      );
    }

    // Log access
    await this.logAccess(userId, projectId, action, true);
  }

  private actionToPermission(action: string): Permission {
    const mapping: Record<string, Permission> = {
      'read': Permission.PROJECT_READ,
      'write': Permission.PROJECT_WRITE,
      'delete': Permission.PROJECT_DELETE,
      'execute': Permission.EXECUTE_CODE,
      'invite': Permission.MANAGE_MEMBERS
    };
    return mapping[action] || Permission.PROJECT_READ;
  }

  private async logAccess(
    userId: string, 
    projectId: string, 
    action: string,
    granted: boolean
  ): Promise<void> {
    await fetch('/api/audit', {
      method: 'POST',
      body: JSON.stringify({
        userId,
        projectId,
        action,
        granted,
        timestamp: Date.now()
      })
    });
  }
}
```

### 8.4 Audit Logging

```typescript
// Comprehensive audit logging system
interface AuditEntry {
  id: string;
  timestamp: number;
  userId: string;
  action: string;
  resource: string;
  details: Record<string, any>;
  ipAddress: string;
  userAgent: string;
  outcome: 'success' | 'failure' | 'denied';
}

class AuditLogger {
  private buffer: AuditEntry[] = [];
  private readonly FLUSH_THRESHOLD = 100;
  private readonly FLUSH_INTERVAL_MS = 5000;

  constructor() {
    // Periodic flush
    setInterval(() => this.flush(), this.FLUSH_INTERVAL_MS);
  }

  log(entry: Omit<AuditEntry, 'id' | 'timestamp'>): void {
    const fullEntry: AuditEntry = {
      ...entry,
      id: crypto.randomUUID(),
      timestamp: Date.now()
    };

    this.buffer.push(fullEntry);

    // Also send immediately via beacon for critical events
    if (['code_execute', 'permission_change', 'member_remove'].includes(entry.action)) {
      navigator.sendBeacon('/api/audit', JSON.stringify(fullEntry));
    }

    // Flush if threshold reached
    if (this.buffer.length >= this.FLUSH_THRESHOLD) {
      this.flush();
    }
  }

  private async flush(): Promise<void> {
    if (this.buffer.length === 0) return;

    const entries = this.buffer.splice(0);
    
    try {
      await fetch('/api/audit/batch', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(entries)
      });
    } catch (error) {
      // Re-buffer on failure
      this.buffer.unshift(...entries);
      console.error('Audit flush failed:', error);
    }
  }

  // Query audit logs
  async query(filter: AuditFilter): Promise<AuditEntry[]> {
    const params = new URLSearchParams();
    
    if (filter.userId) params.set('userId', filter.userId);
    if (filter.projectId) params.set('projectId', filter.projectId);
    if (filter.action) params.set('action', filter.action);
    if (filter.startDate) params.set('startDate', filter.startDate.toString());
    if (filter.endDate) params.set('endDate', filter.endDate.toString());
    if (filter.outcome) params.set('outcome', filter.outcome);

    const response = await fetch(`/api/audit?${params}`);
    return response.json();
  }
}

interface AuditFilter {
  userId?: string;
  projectId?: string;
  action?: string;
  startDate?: Date;
  endDate?: Date;
  outcome?: 'success' | 'failure' | 'denied';
}

// Usage examples
const audit = new AuditLogger();

// Log code execution
audit.log({
  userId: 'user-123',
  action: 'code_execute',
  resource: 'project-abc/main.js',
  details: {
    language: 'javascript',
    executionTime: 234,
    memoryUsed: 1024
  },
  ipAddress: '192.168.1.1',
  userAgent: navigator.userAgent,
  outcome: 'success'
});

// Log permission change
audit.log({
  userId: 'admin-456',
  action: 'permission_change',
  resource: 'project-abc',
  details: {
    targetUserId: 'user-789',
    oldRole: 'viewer',
    newRole: 'developer'
  },
  ipAddress: '192.168.1.1',
  userAgent: navigator.userAgent,
  outcome: 'success'
});
```

---

## 9. Appendix: Complete Configurations

### 9.1 Complete Dockerfile

```dockerfile
# =============================================================================
# OpenWebContainer Production Dockerfile
# =============================================================================
FROM node:20-alpine AS base

# Install dependencies
RUN apk add --no-cache dumb-init curl

# Create app user
RUN addgroup -g 1001 -S appgroup && \
    adduser -u 1001 -S appuser -G appgroup

WORKDIR /app

# =============================================================================
# Build Stage
# =============================================================================
FROM base AS builder

RUN corepack enable && corepack prepare pnpm@8 --activate

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY packages/ ./packages/
COPY apps/ ./apps/
COPY tsconfig.base.json ./

RUN pnpm install --frozen-lockfile

# Build with optimization
ENV NODE_ENV=production
RUN pnpm build

# =============================================================================
# Production Stage
# =============================================================================
FROM base AS production

# Copy built assets
COPY --from=builder --chown=appuser:appgroup /app/apps/playground/dist ./dist
COPY --from=builder --chown=appuser:appgroup /app/package.json ./

# Install serve
RUN npm install -g serve

USER appuser

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["/usr/bin/dumb-init", "--"]
CMD ["serve", "dist", "-p", "3000"]
```

### 9.2 Complete Kubernetes Manifest

```yaml
# Complete production deployment
apiVersion: v1
kind: Namespace
metadata:
  name: openwebcontainer
  labels:
    name: openwebcontainer
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: owc-config
  namespace: openwebcontainer
data:
  NODE_ENV: "production"
  LOG_LEVEL: "info"
  PORT: "3000"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: owc-frontend
  namespace: openwebcontainer
spec:
  replicas: 3
  selector:
    matchLabels:
      app: owc-frontend
  template:
    metadata:
      labels:
        app: owc-frontend
    spec:
      containers:
        - name: frontend
          image: owc-frontend:latest
          ports:
            - containerPort: 3000
          resources:
            requests:
              cpu: 100m
              memory: 128Mi
            limits:
              cpu: 500m
              memory: 512Mi
          livenessProbe:
            httpGet:
              path: /health
              port: 3000
            initialDelaySeconds: 10
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /ready
              port: 3000
            initialDelaySeconds: 5
            periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: owc-frontend
  namespace: openwebcontainer
spec:
  selector:
    app: owc-frontend
  ports:
    - port: 80
      targetPort: 3000
  type: ClusterIP
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: owc-ingress
  namespace: openwebcontainer
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - ide.openwebcontainer.io
      secretName: owc-tls
  rules:
    - host: ide.openwebcontainer.io
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: owc-frontend
                port:
                  number: 80
```

### 9.3 Complete Service Worker

```typescript
// service-worker.ts - Complete production service worker
const CACHE_VERSION = 'v1.0.0';
const STATIC_CACHE = `owc-static-${CACHE_VERSION}`;
const DYNAMIC_CACHE = `owc-dynamic-${CACHE_VERSION}`;

const STATIC_ASSETS = [
  '/',
  '/index.html',
  '/manifest.json',
  '/assets/quickjs.wasm',
  '/assets/core.runtime.js',
  '/assets/vendor.chunk.js',
  '/assets/styles.css',
  '/offline.html'
];

// Install event
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(STATIC_CACHE).then((cache) => {
      return cache.addAll(STATIC_ASSETS);
    })
  );
  self.skipWaiting();
});

// Activate event
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames
          .filter((name) => name.startsWith('owc-') && name !== STATIC_CACHE && name !== DYNAMIC_CACHE)
          .map((name) => caches.delete(name))
      );
    })
  );
  self.clients.claim();
});

// Fetch event
self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Skip cross-origin requests
  if (url.origin !== self.location.origin) {
    return;
  }

  // WASM files - cache first
  if (url.pathname.endsWith('.wasm')) {
    event.respondWith(handleCacheFirst(request, STATIC_CACHE));
    return;
  }

  // Static assets - cache first
  if (STATIC_ASSETS.some(asset => url.pathname.endsWith(asset))) {
    event.respondWith(handleCacheFirst(request, STATIC_CACHE));
    return;
  }

  // API requests - network first
  if (url.pathname.startsWith('/api/')) {
    event.respondWith(handleNetworkFirst(request));
    return;
  }

  // HTML - network first with cache fallback
  if (request.headers.get('accept')?.includes('text/html')) {
    event.respondWith(handleNetworkFirst(request, true));
    return;
  }

  // Everything else - stale while revalidate
  event.respondWith(handleStaleWhileRevalidate(request));
});

// Background sync
self.addEventListener('sync', (event) => {
  if (event.tag === 'sync-project') {
    event.waitUntil(syncProjectData());
  }
});

// Push notifications
self.addEventListener('push', (event) => {
  const data = event.data?.json() || {};
  
  event.waitUntil(
    self.registration.showNotification(data.title || 'OpenWebContainer', {
      body: data.body || 'You have a notification',
      icon: '/icon-192.png',
      badge: '/badge-72.png',
      data: data.url
    })
  );
});

// Notification click
self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  
  event.waitUntil(
    clients.openWindow(event.notification.data)
  );
});

// Cache strategies
async function handleCacheFirst(request: Request, cacheName: string): Promise<Response> {
  const cache = await caches.open(cacheName);
  const cached = await cache.match(request);

  if (cached) {
    return cached;
  }

  try {
    const response = await fetch(request);
    if (response.ok) {
      await cache.put(request, response.clone());
    }
    return response;
  } catch {
    return caches.match('/offline.html');
  }
}

async function handleNetworkFirst(request: Request, fallbackToCache: boolean = false): Promise<Response> {
  const cache = await caches.open(DYNAMIC_CACHE);

  try {
    const response = await fetch(request);
    if (response.ok) {
      await cache.put(request, response.clone());
    }
    return response;
  } catch {
    if (fallbackToCache) {
      const cached = await cache.match(request);
      if (cached) return cached;
    }
    throw new Error('Network unavailable');
  }
}

async function handleStaleWhileRevalidate(request: Request): Promise<Response> {
  const cache = await caches.open(DYNAMIC_CACHE);
  const cached = await cache.match(request);

  // Revalidate in background
  fetch(request).then(async (response) => {
    if (response.ok) {
      await cache.put(request, response.clone());
    }
  }).catch(() => {});

  return cached || fetch(request);
}

async function syncProjectData(): Promise<void> {
  // Read pending operations from IndexedDB
  const db = await openSyncDB();
  const operations = await getPendingOperations(db);

  for (const op of operations) {
    try {
      await fetch('/api/sync', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(op)
      });
      await markOperationSynced(db, op.id);
    } catch {
      // Will retry on next sync
    }
  }
}

function openSyncDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open('owc-sync', 1);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);
    request.onupgradeneeded = (e) => {
      const db = (e.target as IDBOpenDBRequest).result;
      if (!db.objectStoreNames.contains('pending')) {
        db.createObjectStore('pending', { keyPath: 'id' });
      }
    };
  });
}
```

### 9.4 Monitoring Setup (Prometheus + Grafana)

```yaml
# prometheus-servicemonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: owc-monitor
  namespace: openwebcontainer
  labels:
    release: prometheus
spec:
  selector:
    matchLabels:
      app: owc-frontend
  endpoints:
    - port: metrics
      interval: 30s
      path: /metrics
```

```yaml
# grafana-dashboard-configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: owc-grafana-dashboard
  namespace: openwebcontainer
data:
  dashboard.json: |
    {
      "dashboard": {
        "title": "OpenWebContainer Metrics",
        "panels": [
          {
            "title": "Request Rate",
            "type": "graph",
            "targets": [
              {
                "expr": "rate(http_requests_total[5m])",
                "legendFormat": "Requests/sec"
              }
            ]
          },
          {
            "title": "Error Rate",
            "type": "graph",
            "targets": [
              {
                "expr": "rate(http_requests_total{status=~\"5..\"}[5m])",
                "legendFormat": "Errors/sec"
              }
            ]
          },
          {
            "title": "WASM Load Time (p95)",
            "type": "graph",
            "targets": [
              {
                "expr": "histogram_quantile(0.95, rate(wasm_load_duration_bucket[5m]))",
                "legendFormat": "p95 Load Time"
              }
            ]
          },
          {
            "title": "Memory Usage",
            "type": "graph",
            "targets": [
              {
                "expr": "process_resident_memory_bytes",
                "legendFormat": "Memory"
              }
            ]
          }
        ]
      }
    }
```

---

## Document Information

| Field | Value |
|-------|-------|
| **Document** | production-grade.md |
| **Project** | OpenContainer/OpenWebContainer |
| **Version** | 1.0.0 |
| **Created** | 2026-04-05 |
| **Location** | `/home/darkvoid/Boxxed/@dev/repo-expolorations/opencontainer/production-grade.md` |
