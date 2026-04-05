# Nordcraft Production-Grade Implementation Guide

## Overview

This guide covers production-ready patterns for building applications with Nordcraft, including deployment strategies, performance optimization, security hardening, monitoring, and scaling considerations.

## Deployment Architectures

### Static Site Export (SSG)

For content-heavy sites with infrequent updates:

```yaml
# vercel.yaml
version: 2
builds:
  - src: package.json
    use: @vercel/static-build
    config:
      dist: dist

# GitHub Actions workflow
name: Deploy to Vercel
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'
      
      - name: Install dependencies
        run: npm ci
      
      - name: Build static site
        run: npx nordcraft export --output dist
      
      - name: Deploy to Vercel
        uses: amondnet/vercel-action@v20
        with:
          vercel-token: ${{ secrets.VERCEL_TOKEN }}
          vercel-org-id: ${{ secrets.VERCEL_ORG_ID }}
          vercel-project-id: ${{ secrets.VERCEL_PROJECT_ID }}
          working-directory: ./dist
```

### Server-Side Rendering (SSR) with Docker

For dynamic applications requiring SSR:

```dockerfile
# Multi-stage Dockerfile
FROM node:20-alpine AS builder

WORKDIR /app

# Install dependencies
COPY package*.json ./
RUN npm ci

# Build application
COPY . .
RUN npm run build

# Production stage
FROM node:20-alpine AS runner

WORKDIR /app

# Create non-root user
RUN addgroup --system --gid 1001 app && \
    adduser --system --uid 1001 appuser

# Copy built assets
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/package.json ./

# Set ownership
RUN chown -R appuser:appgroup /app

USER appuser

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/health || exit 1

# Start server
CMD ["node", "dist/server.js"]
```

```yaml
# docker-compose.yml
version: '3.8'

services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=production
      - REDIS_URL=redis://redis:6379
      - DATABASE_URL=postgresql://user:pass@db:5432/app
    depends_on:
      - redis
      - db
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '0.5'
          memory: 512M
      restart_policy:
        condition: on-failure

  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data
    deploy:
      resources:
        limits:
          memory: 256M

  db:
    image: postgres:15-alpine
    environment:
      POSTGRES_USER: user
      POSTGRES_PASSWORD: pass
      POSTGRES_DB: app
    volumes:
      - postgres-data:/var/lib/postgresql/data
    deploy:
      resources:
        limits:
          memory: 1G

volumes:
  redis-data:
  postgres-data:
```

### Kubernetes Deployment

For enterprise-scale deployments:

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nordcraft-app
  labels:
    app: nordcraft
spec:
  replicas: 3
  selector:
    matchLabels:
      app: nordcraft
  template:
    metadata:
      labels:
        app: nordcraft
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "3000"
        prometheus.io/path: "/metrics"
    spec:
      containers:
        - name: app
          image: myregistry/nordcraft-app:latest
          ports:
            - containerPort: 3000
          env:
            - name: NODE_ENV
              value: production
            - name: REDIS_URL
              valueFrom:
                secretKeyRef:
                  name: app-secrets
                  key: redis-url
          resources:
            requests:
              cpu: 100m
              memory: 256Mi
            limits:
              cpu: 500m
              memory: 512Mi
          livenessProbe:
            httpGet:
              path: /health
              port: 3000
            initialDelaySeconds: 30
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /ready
              port: 3000
            initialDelaySeconds: 5
            periodSeconds: 5
          volumeMounts:
            - name: tmp
              mountPath: /tmp
      volumes:
        - name: tmp
          emptyDir: {}
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchLabels:
                    app: nordcraft
                topologyKey: kubernetes.io/hostname
---
apiVersion: v1
kind: Service
metadata:
  name: nordcraft-service
spec:
  selector:
    app: nordcraft
  ports:
    - protocol: TCP
      port: 80
      targetPort: 3000
  type: ClusterIP
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: nordcraft-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: nordcraft-app
  minReplicas: 3
  maxReplicas: 10
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
```

## Performance Optimization

### Image Optimization

```typescript
// Image optimization configuration
interface ImageConfig {
  domains: string[]
  formats: ('avif' | 'webp' | 'original')[]
  sizes: number[]
  quality: number
  lazyLoading: boolean
  placeholder: 'blur' | 'empty'
}

const imageConfig: ImageConfig = {
  domains: ['cdn.nordcraft.com', 'images.example.com'],
  formats: ['avif', 'webp', 'original'],
  sizes: [16, 32, 48, 64, 96, 128, 256, 384],
  quality: 80,
  lazyLoading: true,
  placeholder: 'blur'
}

// Component with optimized image
const OptimizedImage = {
  name: 'OptimizedImage',
  attributes: {
    'src': { name: 'src', testValue: '/image.jpg' },
    'alt': { name: 'alt', testValue: 'Description' },
    'width': { name: 'width', testValue: 800 },
    'height': { name: 'height', testValue: 600 }
  },
  nodes: {
    'root': {
      type: 'element',
      tag: 'picture',
      children: ['avifSource', 'webpSource', 'img']
    },
    'avifSource': {
      type: 'element',
      tag: 'source',
      attrs: {
        'type': { type: 'static', value: 'image/avif' },
        'srcset': {
          type: 'formula',
          op: 'image.srcset',
          args: [
            { type: 'attribute', name: 'src' },
            { type: 'static', value: 'avif' }
          ]
        }
      }
    },
    'webpSource': {
      type: 'element',
      tag: 'source',
      attrs: {
        'type': { type: 'static', value: 'image/webp' },
        'srcset': {
          type: 'formula',
          op: 'image.srcset',
          args: [
            { type: 'attribute', name: 'src' },
            { type: 'static', value: 'webp' }
          ]
        }
      }
    },
    'img': {
      type: 'element',
      tag: 'img',
      attrs: {
        'src': { type: 'attribute', name: 'src' },
        'alt': { type: 'attribute', name: 'alt' },
        'width': { type: 'attribute', name: 'width' },
        'height': { type: 'attribute', name: 'height' },
        'loading': { type: 'static', value: 'lazy' },
        'decoding': { type: 'static', value: 'async' }
      }
    }
  }
}
```

### Code Splitting

```typescript
// Component-level code splitting
const LazyComponent = {
  name: 'LazyComponent',
  onLoad: {
    trigger: 'load',
    actions: [
      {
        type: 'Custom',
        name: 'importComponent',
        arguments: [
          {
            name: 'path',
            formula: { type: 'static', value: '/components/HeavyComponent.js' }
          }
        ]
      }
    ]
  },
  variables: {
    'component': {
      initialValue: { type: 'static', value: null }
    },
    'loading': {
      initialValue: { type: 'static', value: true }
    }
  },
  nodes: {
    'root': {
      type: 'element',
      tag: 'div',
      children: ['loadingState', 'componentContainer']
    },
    'loadingState': {
      type: 'element',
      tag: 'div',
      condition: {
        type: 'variable',
        name: 'loading'
      },
      children: ['spinner']
    },
    'componentContainer': {
      type: 'element',
      tag: 'div',
      condition: {
        type: 'formula',
        op: 'not',
        args: [{ type: 'variable', name: 'loading' }]
      }
    }
  }
}
```

### Caching Strategies

```typescript
// API caching configuration
interface CacheConfig {
  strategy: 'stale-while-revalidate' | 'cache-first' | 'network-first'
  maxAge: number
  staleWhileRevalidate: number
  vary: string[]
}

const cacheConfigs: Record<string, CacheConfig> = {
  'static-content': {
    strategy: 'cache-first',
    maxAge: 31536000, // 1 year
    staleWhileRevalidate: 86400, // 1 day
    vary: ['Accept']
  },
  'user-data': {
    strategy: 'stale-while-revalidate',
    maxAge: 300, // 5 minutes
    staleWhileRevalidate: 3600, // 1 hour
    vary: ['Accept', 'Authorization']
  },
  'dynamic-content': {
    strategy: 'network-first',
    maxAge: 60, // 1 minute
    staleWhileRevalidate: 300, // 5 minutes
    vary: ['Accept']
  }
}

// Service Worker configuration
const serviceWorkerConfig = {
  caches: {
    prefix: 'nordcraft-',
    runtime: 'runtime-cache',
    static: 'static-cache'
  },
  routes: {
    '/api/*': {
      strategy: 'network-first',
      cacheName: 'api-cache',
      expiration: {
        maxEntries: 100,
        maxAgeSeconds: 3600
      }
    },
    '/cdn/*': {
      strategy: 'cache-first',
      cacheName: 'cdn-cache',
      expiration: {
        maxEntries: 500,
        maxAgeSeconds: 2592000 // 30 days
      }
    },
    '/static/*': {
      strategy: 'cache-first',
      cacheName: 'static-cache',
      expiration: {
        maxAgeSeconds: 31536000 // 1 year
      }
    }
  }
}
```

## Security Hardening

### Rate Limiting

```typescript
// Rate limiting middleware
import { Redis } from 'ioredis'
import { rateLimit } from 'express-rate-limit'
import RedisStore from 'rate-limit-redis'

const redis = new Redis(process.env.REDIS_URL)

// API rate limiting
const apiLimiter = rateLimit({
  store: new RedisStore({
    sendCommand: (...args: string[]) => redis.call(...args),
  }),
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 100, // Limit each IP to 100 requests per windowMs
  standardHeaders: true,
  legacyHeaders: false,
  keyGenerator: (req) => {
    return req.ip || req.headers['x-forwarded-for'] as string
  },
  skip: (req) => {
    // Skip rate limiting for health checks
    return req.path === '/health'
  }
})

// Authentication rate limiting (stricter)
const authLimiter = rateLimit({
  store: new RedisStore({
    sendCommand: (...args: string[]) => redis.call(...args),
  }),
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 5, // Limit each IP to 5 login attempts per windowMs
  message: 'Too many login attempts, please try again later'
})

// Apply to routes
app.use('/api/', apiLimiter)
app.post('/api/auth/login', authLimiter, loginHandler)
```

### Input Validation

```typescript
// Input validation with Zod
import { z } from 'zod'

// User input schema
const UserInputSchema = z.object({
  email: z.string().email('Invalid email address'),
  password: z
    .string()
    .min(8, 'Password must be at least 8 characters')
    .max(128, 'Password is too long')
    .regex(/[A-Z]/, 'Password must contain an uppercase letter')
    .regex(/[a-z]/, 'Password must contain a lowercase letter')
    .regex(/[0-9]/, 'Password must contain a number'),
  name: z.string().min(1).max(100)
})

// API request validation middleware
function validateRequest(schema: z.ZodSchema) {
  return async (req: Request, res: Response, next: NextFunction) => {
    try {
      req.body = await schema.parseAsync(req.body)
      next()
    } catch (error) {
      if (error instanceof z.ZodError) {
        res.status(400).json({
          error: 'Validation error',
          details: error.errors.map(e => ({
            field: e.path.join('.'),
            message: e.message
          }))
        })
      } else {
        next(error)
      }
    }
  }
}

// Apply validation
app.post('/api/users', validateRequest(UserInputSchema), createUserHandler)
```

### Content Security Policy

```typescript
// CSP configuration
const cspConfig = {
  directives: {
    defaultSrc: ["'self'"],
    scriptSrc: [
      "'self'",
      "'unsafe-inline'", // Required for Nordcraft workflows
      'https://cdn.nordcraft.com'
    ],
    styleSrc: [
      "'self'",
      "'unsafe-inline'", // Required for dynamic styles
      'https://fonts.googleapis.com'
    ],
    fontSrc: [
      "'self'",
      'https://fonts.gstatic.com'
    ],
    imgSrc: [
      "'self'",
      'data:',
      'blob:',
      'https://cdn.nordcraft.com',
      'https://images.example.com'
    ],
    connectSrc: [
      "'self'",
      'https://api.example.com',
      'wss://api.example.com' // For WebSocket connections
    ],
    frameSrc: [
      "'self'",
      'https://www.youtube.com',
      'https://player.vimeo.com'
    ],
    objectSrc: ["'none'"],
    baseUri: ["'self'"],
    formAction: ["'self'"],
    upgradeInsecureRequests: true
  }
}

// Apply CSP middleware
import helmet from 'helmet'

app.use(helmet.contentSecurityPolicy(cspConfig))
```

### XSS Prevention

```typescript
// XSS sanitization for user content
import DOMPurify from 'isomorphic-dompurify'

function sanitizeHtml(html: string): string {
  return DOMPurify.sanitize(html, {
    ALLOWED_TAGS: [
      'b', 'i', 'em', 'strong', 'a', 'p', 'br',
      'ul', 'ol', 'li', 'span', 'div'
    ],
    ALLOWED_ATTR: ['href', 'target', 'rel', 'class', 'style'],
    ALLOW_DATA_ATTR: false,
    FORBID_TAGS: ['script', 'iframe', 'object', 'embed'],
    FORBID_ATTR: ['onclick', 'onerror', 'onload', 'onmouseover']
  })
}

// Formula sanitization in Nordcraft
const SanitizedText = {
  name: 'SanitizedText',
  attributes: {
    'content': { name: 'content', testValue: 'User content' }
  },
  formulas: {
    'sanitizedContent': {
      formula: {
        op: 'custom.sanitizeHtml',
        args: [{ type: 'attribute', name: 'content' }]
      }
    }
  },
  nodes: {
    'root': {
      type: 'element',
      tag: 'p',
      children: ['textContent']
    },
    'textContent': {
      type: 'text',
      value: { type: 'formula', name: 'sanitizedContent' }
    }
  }
}
```

## Monitoring and Observability

### Metrics Collection

```typescript
// Prometheus metrics
import { Registry, Counter, Histogram, Gauge } from 'prom-client'

const register = new Registry()

// Request metrics
const httpRequestDuration = new Histogram({
  name: 'http_request_duration_seconds',
  help: 'Duration of HTTP requests in seconds',
  labelNames: ['method', 'route', 'status_code'],
  buckets: [0.01, 0.05, 0.1, 0.5, 1, 2, 5, 10],
  registers: [register]
})

const httpRequestCount = new Counter({
  name: 'http_requests_total',
  help: 'Total number of HTTP requests',
  labelNames: ['method', 'route', 'status_code'],
  registers: [register]
})

// Component rendering metrics
const componentRenderDuration = new Histogram({
  name: 'component_render_duration_seconds',
  help: 'Duration of component rendering in seconds',
  labelNames: ['component_name', 'ssr'],
  buckets: [0.001, 0.005, 0.01, 0.05, 0.1, 0.5],
  registers: [register]
})

// API call metrics
const apiCallDuration = new Histogram({
  name: 'api_call_duration_seconds',
  help: 'Duration of API calls in seconds',
  labelNames: ['api_name', 'status'],
  buckets: [0.1, 0.5, 1, 2, 5, 10, 30],
  registers: [register]
})

// Active connections gauge
const activeConnections = new Gauge({
  name: 'active_connections',
  help: 'Number of active connections',
  registers: [register]
})

// Metrics endpoint
app.get('/metrics', async (req, res) => {
  res.set('Content-Type', register.contentType)
  res.end(await register.metrics())
})

// Middleware to collect metrics
app.use((req, res, next) => {
  const start = Date.now()
  
  res.on('finish', () => {
    const duration = (Date.now() - start) / 1000
    const route = req.route?.path || req.path
    const status = res.statusCode
    
    httpRequestDuration.observe({ method: req.method, route, status_code: status }, duration)
    httpRequestCount.inc({ method: req.method, route, status_code: status })
  })
  
  next()
})
```

### Distributed Tracing

```typescript
// OpenTelemetry configuration
import { NodeTracerProvider } from '@opentelemetry/sdk-trace-node'
import { BatchSpanProcessor } from '@opentelemetry/sdk-trace-base'
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-http'
import { registerInstrumentations } from '@opentelemetry/instrumentation'
import { HttpInstrumentation } from '@opentelemetry/instrumentation-http'

const provider = new NodeTracerProvider({
  resource: new Resource({
    'service.name': 'nordcraft-app',
    'service.version': '1.0.0'
  })
})

const traceExporter = new OTLPTraceExporter({
  url: 'http://jaeger:4318/v1/traces'
})

provider.addSpanProcessor(new BatchSpanProcessor(traceExporter))
provider.register()

registerInstrumentations({
  instrumentations: [
    new HttpInstrumentation({
      requestHook: (span, request) => {
        span.setAttribute('http.component', 'nordcraft')
      }
    })
  ]
})

// Manual tracing for component rendering
import { trace, context } from '@opentelemetry/api'

const tracer = trace.getTracer('nordcraft')

function renderComponentWithTracing(component: Component, props: any) {
  return tracer.startActiveSpan(
    `component.render.${component.name}`,
    async (span) => {
      try {
        span.setAttributes({
          'component.name': component.name,
          'component.props': JSON.stringify(props)
        })
        
        const result = await component.render(props)
        
        span.setStatus({ code: 2 }) // OK
        return result
      } catch (error) {
        span.setStatus({ code: 2, message: error.message })
        span.recordException(error)
        throw error
      } finally {
        span.end()
      }
    }
  )
}
```

### Logging Strategy

```typescript
// Structured logging with Pino
import pino from 'pino'

const logger = pino({
  level: process.env.LOG_LEVEL || 'info',
  formatters: {
    level: (label) => ({ level: label }),
    bindings: (bindings) => ({
      pid: bindings.pid,
      host: bindings.host
    })
  },
  base: {
    service: 'nordcraft-app',
    version: process.env.APP_VERSION
  },
  transport: {
    target: 'pino-pretty',
    options: {
      colorize: true,
      translateTime: 'SYS:standard'
    }
  }
})

// Request logging middleware
app.use((req, res, next) => {
  const requestId = req.headers['x-request-id'] || crypto.randomUUID()
  
  req.log = logger.child({
    requestId,
    method: req.method,
    url: req.url,
    ip: req.ip
  })
  
  res.on('finish', () => {
    req.log.info({
      statusCode: res.statusCode,
      responseTime: Date.now() - req.startTime
    }, 'request completed')
  })
  
  next()
})

// Error logging
app.use((error: Error, req: Request, res: Response, next: NextFunction) => {
  req.log.error({
    err: error,
    stack: error.stack
  }, 'unhandled error')
  
  res.status(500).json({
    error: 'Internal server error',
    requestId: req.headers['x-request-id']
  })
})
```

## Health Checks

```typescript
// Comprehensive health check endpoint
import { Router } from 'express'
import { redis } from './redis'
import { pool } from './database'

const healthRouter = Router()

healthRouter.get('/health', async (req, res) => {
  const checks = {
    status: 'healthy',
    checks: {},
    timestamp: new Date().toISOString()
  }
  
  // Check database
  try {
    await pool.query('SELECT 1')
    checks.checks.database = { status: 'healthy' }
  } catch (error) {
    checks.checks.database = { status: 'unhealthy', error: error.message }
    checks.status = 'unhealthy'
  }
  
  // Check Redis
  try {
    await redis.ping()
    checks.checks.redis = { status: 'healthy' }
  } catch (error) {
    checks.checks.redis = { status: 'unhealthy', error: error.message }
    checks.status = 'unhealthy'
  }
  
  // Check memory
  const memoryUsage = process.memoryUsage()
  const memoryLimit = memoryUsage.heapSizeLimit
  const memoryUsed = memoryUsage.heapUsed
  const memoryPercent = (memoryUsed / memoryLimit) * 100
  
  checks.checks.memory = {
    status: memoryPercent > 90 ? 'warning' : 'healthy',
    used: memoryUsed,
    limit: memoryLimit,
    percent: Math.round(memoryPercent * 100) / 100
  }
  
  const statusCode = checks.status === 'healthy' ? 200 : 503
  res.status(statusCode).json(checks)
})

healthRouter.get('/ready', async (req, res) => {
  // Readiness checks - is the app ready to serve traffic?
  const isReady = await checkReadiness()
  
  if (isReady) {
    res.status(200).json({ ready: true })
  } else {
    res.status(503).json({ ready: false })
  }
})

healthRouter.get('/live', async (req, res) => {
  // Liveness check - is the app alive?
  res.status(200).json({ live: true })
})
```

## CI/CD Pipeline

```yaml
# GitHub Actions CI/CD
name: CI/CD Pipeline

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'
          cache: 'npm'
      
      - name: Install dependencies
        run: npm ci
      
      - name: Run linting
        run: npm run lint
      
      - name: Run type check
        run: npm run typecheck
      
      - name: Run tests
        run: npm test -- --coverage
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3

  build:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'
          cache: 'npm'
      
      - name: Build
        run: npm run build
      
      - name: Upload build artifacts
        uses: actions/upload-artifact@v3
        with:
          name: dist
          path: dist/

  docker:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Download build artifacts
        uses: actions/download-artifact@v3
        with:
          name: dist
          path: dist/
      
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v2
      
      - name: Login to Container Registry
        uses: docker/login-action@v2
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      
      - name: Build and push Docker image
        uses: docker/build-push-action@v4
        with:
          context: .
          push: true
          tags: |
            ghcr.io/${{ github.repository }}:latest
            ghcr.io/${{ github.repository }}:${{ github.sha }}
          cache-from: type=registry,ref=ghcr.io/${{ github.repository }}:buildcache
          cache-to: type=registry,ref=ghcr.io/${{ github.repository }}:buildcache,mode=max

  deploy:
    needs: docker
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v3
      
      - name: Deploy to Kubernetes
        uses: Azure/k8s-deploy@v4
        with:
          manifests: |
            k8s/deployment.yaml
            k8s/service.yaml
          images: |
            ghcr.io/${{ github.repository }}:${{ github.sha }}
          kubectl-version: 'latest'
          namespace: production
```

## Summary

Production-grade Nordcraft applications require:

1. **Deployment Strategy**: Choose SSG, SSR, or hybrid based on use case
2. **Containerization**: Docker for consistent environments
3. **Orchestration**: Kubernetes for scaling and management
4. **Performance**: Image optimization, code splitting, caching
5. **Security**: Rate limiting, input validation, CSP, XSS prevention
6. **Monitoring**: Metrics, tracing, structured logging
7. **Health Checks**: Liveness, readiness, and detailed health endpoints
8. **CI/CD**: Automated testing, building, and deployment
9. **Observability**: Distributed tracing for debugging complex interactions
10. **Scalability**: Horizontal pod autoscaling, resource limits

This guide provides the foundation for deploying Nordcraft applications at scale with enterprise-grade reliability and security.
