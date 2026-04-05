# Joy Ecosystem - Production Grade Guide

## Overview

This guide covers deploying and running the Joy ecosystem (Compiler, Bud Framework, X-ray, LLM) in production environments. It includes Docker configurations, Kubernetes deployments, security hardening, monitoring, and scaling strategies.

## Table of Contents

1. [Joy Compiler Production](#joy-compiler-production)
2. [Bud Framework Production](#bud-framework-production)
3. [X-ray Production](#x-ray-production)
4. [LLM Agent Production](#llm-agent-production)
5. [Monitoring & Observability](#monitoring--observability)
6. [Security Hardening](#security-hardening)
7. [Scaling Strategies](#scaling-strategies)

---

## Joy Compiler Production

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  Joy Compiler Service                        │
├─────────────────────────────────────────────────────────────┤
│  API Layer                                                   │
│  - HTTP/gRPC endpoint for compilation requests              │
│  - Authentication & Rate limiting                           │
├─────────────────────────────────────────────────────────────┤
│  Compilation Layer                                           │
│  - Parse → Index → Graph → Translate → Assemble             │
│  - Worker pool for concurrent compilations                  │
├─────────────────────────────────────────────────────────────┤
│  Cache Layer                                                 │
│  - Redis for compiled output caching                        │
│  - CDN for static runtime files                             │
├─────────────────────────────────────────────────────────────┤
│  Storage Layer                                               │
│  - S3/GCS for artifact storage                              │
│  - PostgreSQL for compilation history                       │
└─────────────────────────────────────────────────────────────┘
```

### Docker Configuration

```dockerfile
# Dockerfile.joy-compiler
FROM golang:1.21-alpine AS builder

RUN apk add --no-cache git cmake build-base

WORKDIR /build
COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /joy-compiler ./cmd/joy

# Runtime image
FROM alpine:3.18

RUN apk add --no-cache ca-certificates

RUN addgroup -g 1000 joy && \
    adduser -D -u 1000 -G joy joy

WORKDIR /app
COPY --from=builder /joy-compiler .
COPY --from=builder /build/runtime ./runtime

USER joy

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget -qO- http://localhost:8080/health || exit 1

ENTRYPOINT ["/joy-compiler"]
CMD ["serve", "--port", "8080"]
```

### Kubernetes Deployment

```yaml
# k8s/joy-compiler-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: joy-compiler
  labels:
    app: joy-compiler
spec:
  replicas: 3
  selector:
    matchLabels:
      app: joy-compiler
  template:
    metadata:
      labels:
        app: joy-compiler
    spec:
      serviceAccountName: joy-compiler
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      containers:
      - name: compiler
        image: joy-compiler:latest
        ports:
        - containerPort: 8080
          name: http
        resources:
          requests:
            cpu: 500m
            memory: 512Mi
          limits:
            cpu: 2000m
            memory: 2Gi
        env:
        - name: PORT
          value: "8080"
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: joy-secrets
              key: redis-url
        - name: CACHE_TTL
          value: "3600"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
        volumeMounts:
        - name: tmp
          mountPath: /tmp
      volumes:
      - name: tmp
        emptyDir: {}
---
apiVersion: v1
kind: Service
metadata:
  name: joy-compiler
spec:
  selector:
    app: joy-compiler
  ports:
  - port: 80
    targetPort: 8080
  type: ClusterIP
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: joy-compiler-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: joy-compiler
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
```

### Configuration

```yaml
# config/production.yaml
server:
  port: 8080
  read_timeout: 30s
  write_timeout: 30s
  idle_timeout: 120s

compiler:
  max_concurrent: 10
  timeout: 120s
  max_source_size: 10mb
  
cache:
  enabled: true
  type: redis
  redis:
    url: redis://localhost:6379
    ttl: 1h
    max_memory: 2gb

storage:
  type: s3
  bucket: joy-compiler-artifacts
  region: us-east-1
  
logging:
  level: info
  format: json
  
metrics:
  enabled: true
  port: 9090
```

---

## Bud Framework Production

### Production Build

```bash
# Build for production
bud build

# The output is a self-contained binary
./bud/build/myapp

# Or build with specific flags
bud build --minify --embed-views --embed-public
```

### Docker Configuration

```dockerfile
# Dockerfile.bud-app
FROM golang:1.21-alpine AS builder

RUN apk add --no-cache git nodejs npm

WORKDIR /build
COPY go.mod go.sum ./
RUN go mod download

COPY . .

# Install dependencies and build
RUN go install github.com/livebud/bud@latest && \
    bud build

# Runtime image
FROM alpine:3.18

RUN apk add --no-cache ca-certificates tzdata

RUN addgroup -g 1000 app && \
    adduser -D -u 1000 -G app app

WORKDIR /app
COPY --from=builder /build/bud/build/myapp .

USER app

EXPOSE 3000

ENV GIN_MODE=release
ENV BUD_ENV=production

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget -qO- http://localhost:3000/health || exit 1

ENTRYPOINT ["/app/myapp"]
```

### Production Server Configuration

```go
// cmd/main.go
package main

import (
    "context"
    "log"
    "net/http"
    "os"
    "os/signal"
    "syscall"
    "time"
    
    "github.com/livebud/bud"
)

func main() {
    app, err := bud.New()
    if err != nil {
        log.Fatal(err)
    }
    
    server := &http.Server{
        Addr:         ":3000",
        Handler:      app,
        ReadTimeout:  15 * time.Second,
        WriteTimeout: 15 * time.Second,
        IdleTimeout:  60 * time.Second,
    }
    
    // Graceful shutdown
    go func() {
        sig := make(chan os.Signal, 1)
        signal.Notify(sig, syscall.SIGINT, syscall.SIGTERM)
        <-sig
        
        ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
        defer cancel()
        
        if err := server.Shutdown(ctx); err != nil {
            log.Fatal(err)
        }
    }()
    
    log.Printf("Server starting on :3000")
    if err := server.ListenAndServe(); err != http.ErrServerClosed {
        log.Fatal(err)
    }
}
```

### Kubernetes Deployment

```yaml
# k8s/bud-app-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: bud-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: bud-app
  template:
    metadata:
      labels:
        app: bud-app
    spec:
      containers:
      - name: app
        image: bud-app:latest
        ports:
        - containerPort: 3000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: app-secrets
              key: database-url
        - name: SESSION_SECRET
          valueFrom:
            secretKeyRef:
              name: app-secrets
              key: session-secret
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
        readinessProbe:
          httpGet:
            path: /ready
            port: 3000
```

---

## X-ray Production

### Production Scraper Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    X-ray Scraper Service                      │
├─────────────────────────────────────────────────────────────┤
│  Request Queue                                               │
│  - Redis Streams / SQS for job queue                        │
│  - Priority queues for different targets                    │
├─────────────────────────────────────────────────────────────┤
│  Worker Pool                                                 │
│  - Configurable concurrency per worker                      │
│  - Rate limiting per domain                                 │
│  - Proxy rotation                                           │
├─────────────────────────────────────────────────────────────┤
│  Storage                                                     │
│  - Raw HTML storage (S3/GCS)                                │
│  - Extracted data (PostgreSQL/Elasticsearch)                │
├─────────────────────────────────────────────────────────────┤
│  Monitoring                                                  │
│  - Success/failure rates                                    │
│  - Latency metrics                                          │
│  - Block detection                                          │
└─────────────────────────────────────────────────────────────┘
```

### Docker Configuration

```dockerfile
# Dockerfile.xray-scraper
FROM node:18-alpine

RUN apk add --no-cache chromium

ENV PUPPETEER_SKIP_CHROMIUM_DOWNLOAD=true
ENV PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium-browser

WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production

COPY . .

RUN addgroup -g 1000 scraper && \
    adduser -D -u 1000 -G scraper scraper && \
    chown -R scraper:scraper /app

USER scraper

EXPOSE 3000

ENTRYPOINT ["node", "src/worker.js"]
```

### Worker Implementation

```javascript
// src/worker.js
const Xray = require('x-ray');
const Redis = require('ioredis');

class ScraperWorker {
  constructor(config) {
    this.redis = new Redis(config.redisUrl);
    this.x = Xray({
      filters: config.filters,
    })
    .concurrency(config.concurrency || 3)
    .delay(config.delay || 2000)
    .throttle(config.throttle || 10)
    .timeout(config.timeout || 30000);
    
    this.proxyPool = config.proxies || [];
  }
  
  async start() {
    console.log('Worker started');
    
    while (true) {
      try {
        const job = await this.redis.brpop('scraper:queue', 5);
        if (!job) continue;
        
        const task = JSON.parse(job[1]);
        await this.process(task);
      } catch (err) {
        console.error('Worker error:', err);
        await this.sleep(1000);
      }
    }
  }
  
  async process(task) {
    const { url, schema, callback } = task;
    
    try {
      const result = await this.x(url, schema);
      
      if (callback) {
        await this.redis.publish('scraper:results', JSON.stringify({
          taskId: task.id,
          result,
          status: 'success'
        }));
      }
      
      await this.redis.incr('scraper:stats:success');
    } catch (err) {
      await this.redis.incr('scraper:stats:failure');
      console.error('Scraping failed:', err);
    }
  }
  
  sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

const worker = new ScraperWorker({
  redisUrl: process.env.REDIS_URL,
  concurrency: parseInt(process.env.CONCURRENCY) || 3,
  delay: parseInt(process.env.DELAY) || 2000,
  proxies: process.env.PROXIES?.split(',') || []
});

worker.start();
```

---

## LLM Agent Production

### Production Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   LLM Agent Service                          │
├─────────────────────────────────────────────────────────────┤
│  Gateway Layer                                               │
│  - API authentication & authorization                       │
│  - Rate limiting per user/API key                          │
│  - Request validation                                       │
├─────────────────────────────────────────────────────────────┤
│  Agent Orchestration                                         │
│  - Conversation state management                            │
│  - Tool registry & execution                                │
│  - Multi-turn handling                                      │
├─────────────────────────────────────────────────────────────┤
│  Provider Layer                                              │
│  - Multi-provider routing (OpenAI, Anthropic, etc.)         │
│  - Fallback & retry logic                                   │
│  - Cost tracking per provider                               │
├─────────────────────────────────────────────────────────────┤
│  Observability                                               │
│  - Token usage tracking                                     │
│  - Latency metrics                                          │
│  - Error rates & tracing                                    │
└─────────────────────────────────────────────────────────────┘
```

### Docker Configuration

```dockerfile
# Dockerfile.llm-agent
FROM golang:1.21-alpine AS builder

RUN apk add --no-cache git build-base

WORKDIR /build
COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /llm-agent ./cmd/llm

FROM alpine:3.18

RUN apk add --no-cache ca-certificates

RUN addgroup -g 1000 llm && \
    adduser -D -u 1000 -G llm llm

WORKDIR /app
COPY --from=builder /llm-agent .

USER llm

EXPOSE 8080

ENTRYPOINT ["/llm-agent"]
CMD ["serve", "--port", "8080"]
```

### Production Server

```go
// cmd/server/main.go
package main

import (
    "context"
    "log"
    "net/http"
    "os"
    "time"
    
    "github.com/matthewmueller/llm"
    "github.com/matthewmueller/llm/providers/openai"
    "github.com/matthewmueller/llm/providers/anthropic"
)

func main() {
    // Create client with multiple providers
    client := llm.New(
        openai.New(os.Getenv("OPENAI_API_KEY")),
        anthropic.New(os.Getenv("ANTHROPIC_API_KEY")),
    )
    
    mux := http.NewServeMux()
    
    // Chat endpoint
    mux.HandleFunc("/v1/chat", func(w http.ResponseWriter, r *http.Request) {
        if r.Method != http.MethodPost {
            http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
            return
        }
        
        var req ChatRequest
        if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
            http.Error(w, err.Error(), http.StatusBadRequest)
            return
        }
        
        // Stream response
        w.Header().Set("Content-Type", "text/event-stream")
        w.Header().Set("Cache-Control", "no-cache")
        w.Header().Set("Connection", "keep-alive")
        
        flusher, _ := w.(http.Flusher)
        
        for event, err := range client.Chat(r.Context(), req.Provider,
            llm.WithModel(req.Model),
            llm.WithMessage(req.Messages...),
            llm.WithTool(req.Tools...),
        ) {
            if err != nil {
                log.Printf("Chat error: %v", err)
                return
            }
            
            json.NewEncoder(w).Encode(event)
            flusher.Flush()
        }
    })
    
    server := &http.Server{
        Addr:         ":8080",
        Handler:      mux,
        ReadTimeout:  30 * time.Second,
        WriteTimeout: 30 * time.Second,
        IdleTimeout:  120 * time.Second,
    }
    
    log.Printf("LLM Agent server starting on :8080")
    log.Fatal(server.ListenAndServe())
}
```

### Rate Limiting Middleware

```go
// middleware/ratelimit.go
package middleware

import (
    "context"
    "net/http"
    "sync"
    "time"
    
    "github.com/go-redis/redis/v8"
    "golang.org/x/time/rate"
)

type RateLimiter struct {
    redis   *redis.Client
    limiters sync.Map
    rps     int
    burst   int
}

func NewRateLimiter(redisURL string, rps, burst int) *RateLimiter {
    opt, _ := redis.ParseURL(redisURL)
    return &RateLimiter{
        redis: redis.NewClient(*opt),
        rps:   rps,
        burst: burst,
    }
}

func (rl *RateLimiter) Middleware(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        apiKey := r.Header.Get("X-API-Key")
        if apiKey == "" {
            http.Error(w, "Missing API key", http.StatusUnauthorized)
            return
        }
        
        limiter, _ := rl.limiters.LoadOrStore(apiKey, rate.NewLimiter(rate.Limit(rl.rps), rl.burst))
        
        if !limiter.(*rate.Limiter).Allow() {
            w.Header().Set("Retry-After", "60")
            http.Error(w, "Rate limit exceeded", http.StatusTooManyRequests)
            return
        }
        
        next.ServeHTTP(w, r)
    })
}
```

---

## Monitoring & Observability

### Prometheus Metrics

```go
// metrics/prometheus.go
package metrics

import (
    "github.com/prometheus/client_golang/prometheus"
    "github.com/prometheus/client_golang/prometheus/promauto"
)

var (
    // Compilation metrics
    CompilationDuration = promauto.NewHistogramVec(prometheus.HistogramOpts{
        Name:    "joy_compilation_duration_seconds",
        Help:    "Time taken to compile Go to JavaScript",
        Buckets: prometheus.ExponentialBuckets(0.1, 2, 10),
    }, []string{"status", "source_size"})
    
    CompilationTotal = promauto.NewCounterVec(prometheus.CounterOpts{
        Name: "joy_compilation_total",
        Help: "Total number of compilations",
    }, []string{"status"})
    
    // LLM metrics
    LLMTokenUsage = promauto.NewCounterVec(prometheus.CounterOpts{
        Name: "llm_tokens_total",
        Help: "Total tokens used",
    }, []string{"provider", "type"}) // type: input/output
    
    LLMRequestDuration = promauto.NewHistogramVec(prometheus.HistogramOpts{
        Name:    "llm_request_duration_seconds",
        Help:    "LLM request latency",
        Buckets: prometheus.ExponentialBuckets(0.5, 2, 10),
    }, []string{"provider", "model", "status"})
    
    // Scraper metrics
    ScraperRequestsTotal = promauto.NewCounterVec(prometheus.CounterOpts{
        Name: "xray_requests_total",
        Help: "Total scraping requests",
    }, []string{"domain", "status"})
    
    ScraperItemsExtracted = promauto.NewCounterVec(prometheus.CounterOpts{
        Name: "xray_items_extracted_total",
        Help: "Total items extracted",
    }, []string{"schema"})
)
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "Joy Ecosystem",
    "panels": [
      {
        "title": "Compilation Rate",
        "targets": [{
          "expr": "rate(joy_compilation_total[5m])"
        }]
      },
      {
        "title": "LLM Token Usage",
        "targets": [{
          "expr": "sum(rate(llm_tokens_total[5m])) by (provider)"
        }]
      },
      {
        "title": "Scraping Success Rate",
        "targets": [{
          "expr": "rate(xray_requests_total{status=\"success\"}[5m]) / rate(xray_requests_total[5m])"
        }]
      }
    ]
  }
}
```

### Distributed Tracing

```go
// tracing/tracing.go
package tracing

import (
    "go.opentelemetry.io/otel"
    "go.opentelemetry.io/otel/exporters/jaeger"
    "go.opentelemetry.io/otel/sdk/resource"
    sdktrace "go.opentelemetry.io/otel/sdk/trace"
    semconv "go.opentelemetry.io/otel/semconv/v1.18.0"
)

func InitTracer(serviceName, endpoint string) (*sdktrace.TracerProvider, error) {
    exporter, err := jaeger.New(jaeger.WithCollectorEndpoint(
        jaeger.WithEndpoint(endpoint),
    ))
    if err != nil {
        return nil, err
    }
    
    tp := sdktrace.NewTracerProvider(
        sdktrace.WithBatcher(exporter),
        sdktrace.WithResource(resource.NewWithAttributes(
            semconv.SchemaURL,
            semconv.ServiceNameKey.String(serviceName),
        )),
    )
    
    otel.SetTracerProvider(tp)
    return tp, nil
}
```

---

## Security Hardening

### Container Security

```yaml
# k8s/security-context.yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  runAsGroup: 1000
  fsGroup: 1000
  capabilities:
    drop:
      - ALL
  readOnlyRootFilesystem: true
  seccompProfile:
    type: RuntimeDefault
```

### Network Policies

```yaml
# k8s/network-policy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: joy-compiler-policy
spec:
  podSelector:
    matchLabels:
      app: joy-compiler
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          app: api-gateway
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - to:
    - podSelector:
        matchLabels:
          app: redis
    ports:
    - protocol: TCP
      port: 6379
```

### Secrets Management

```yaml
# k8s/secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: joy-secrets
type: Opaque
stringData:
  redis-url: redis://redis:6379
  api-keys: |
    openai=sk-...
    anthropic=sk-ant-...
  database-password: secure-password-here
```

---

## Scaling Strategies

### Horizontal Scaling

```yaml
# k8s/hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: joy-compiler-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: joy-compiler
  minReplicas: 3
  maxReplicas: 50
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Pods
    pods:
      metric:
        name: queue_depth
      target:
        type: AverageValue
        averageValue: 10
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

### Caching Strategy

```go
// cache/cache.go
package cache

import (
    "context"
    "crypto/sha256"
    "encoding/hex"
    "time"
    
    "github.com/go-redis/redis/v8"
)

type CompilerCache struct {
    redis *redis.Client
    ttl   time.Duration
}

func (c *CompilerCache) Get(ctx context.Context, source string) ([]byte, error) {
    key := c.hash(source)
    return c.redis.Get(ctx, key).Bytes()
}

func (c *CompilerCache) Set(ctx context.Context, source string, result []byte) error {
    key := c.hash(source)
    return c.redis.Set(ctx, key, result, c.ttl).Err()
}

func (c *CompilerCache) hash(source string) string {
    h := sha256.Sum256([]byte(source))
    return "joy:compile:" + hex.EncodeToString(h[:])
}
```

### Load Balancing

```yaml
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: joy-ingress
  annotations:
    nginx.ingress.kubernetes.io/limit-rps: "100"
    nginx.ingress.kubernetes.io/load-balance: "least_conn"
spec:
  rules:
  - host: compiler.joy.dev
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: joy-compiler
            port:
              number: 80
```

## Summary

Production deployment of the Joy ecosystem requires:

1. **Containerization** - Docker images with minimal attack surface
2. **Orchestration** - Kubernetes for scaling and management
3. **Monitoring** - Prometheus metrics and Grafana dashboards
4. **Tracing** - OpenTelemetry for distributed tracing
5. **Security** - Network policies, secrets management, RBAC
6. **Caching** - Redis for compiled output and session data
7. **Rate Limiting** - Per-user and per-endpoint limits
8. **Auto-scaling** - HPA based on CPU, memory, and custom metrics
