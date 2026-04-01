---
location: /home/darkvoid/Boxxed/@formulas/src.zeromicro/go-zero
repository: git@github.com:zeromicro/go-zero.git
explored_at: 2026-03-30
language: Go
category: Microservices, gRPC, REST
---

# go-zero - Exploration

## Overview

go-zero is a **high-performance microservices framework** for Go that emphasizes resilience and productivity. It includes goctl, a powerful code generation tool that transforms API definitions into production-ready services with built-in circuit breaking, rate limiting, and observability.

### Key Value Proposition

- **Code Generation**: goctl generates 80%+ of boilerplate
- **High Performance**: Optimized for 100k+ QPS workloads
- **Resilience Patterns**: Built-in adaptive circuit breaking
- **Service Discovery**: Etcd, Consul, K8s integration
- **Observability**: Prometheus, Jaeger, OpenTelemetry
- **Multiple Protocols**: REST, gRPC, GraphQL support

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    go-zero Architecture                          │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   API Gateway   │  │   REST Service  │  │   gRPC Service  │ │
│  │   (goctl api)   │  │   (goctl api)   │  │   (goctl rpc)   │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘ │
│           │                    │                    │           │
│           └────────────────────┼────────────────────┘           │
│                      Rest Engine / ZRPC                         │
│                                                                 │
│           ┌─────────────────────┴─────────────────────┐        │
│           │                                           │        │
│           ▼                                           ▼        │
│  ┌─────────────────┐                        ┌─────────────────┐ │
│  │   Middleware    │                        │   Middleware    │ │
│  │   - Auth        │                        │   - Rate Limit  │ │
│  │   - Logging     │                        │   - Circuit     │ │
│  │   - Tracing     │                        │   - Recovery    │ │
│  └─────────────────┘                        └─────────────────┘ │
│                                                                 │
│           ┌─────────────────────┴─────────────────────┐        │
│           ▼                                           ▼        │
│  ┌─────────────────┐                        ┌─────────────────┐ │
│  │   Service Disc. │                        │   Storage       │ │
│  │   - Etcd        │                        │   - MySQL       │ │
│  │   - Consul      │                        │   - Redis       │ │
│  │   - K8s         │                        │   - Mongo       │ │
│  └─────────────────┘                        └─────────────────┘ │
│                                                                 │
│           ┌─────────────────────┴─────────────────────┐        │
│           ▼                                           ▼        │
│  ┌─────────────────┐                        ┌─────────────────┐ │
│  │   Observability │                        │   Resilience    │ │
│  │   - Prometheus  │                        │   - Breaker     │ │
│  │   - Jaeger      │                        │   - Retry       │ │
│  │   - Logx        │                        │   - Timeout     │ │
│  └─────────────────┘                        └─────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
go-zero/
├── core/                   # Core functionality
│   ├── conf/               # Configuration loading
│   ├── load/               # Adaptive load shedding
│   ├── breakers/           # Circuit breaker implementations
│   ├── limit/              # Rate limiters (period, token)
│   ├── trace/              # OpenTelemetry tracing
│   ├── metric/             # Prometheus metrics
│   ├── logx/               # Structured logging
│   ├── proc/               # Process management
│   └── stat/               # Statistics collection
│
├── rest/                   # REST engine
│   ├── engine.go           # HTTP server
│   ├── handler/            # HTTP handlers
│   ├── httpx/              # HTTP utilities
│   ├── internal/           # Internal utilities
│   └── router/             # Path routing
│
├── zrpc/                   # gRPC wrapper
│   ├── client/             # RPC client
│   ├── server/             # RPC server
│   ├── resolver/           # Service discovery
│   └── internal/           # Internal utilities
│
├── gateway/                # API gateway
│   └── server.go           # Gateway server
│
├── tools/                  # Development tools
│   └── goctl/              # Code generator
│       ├── api/            # API code gen
│       ├── rpc/            # RPC code gen
│       ├── model/          # SQL model gen
│       ├── docker/         # Dockerfile gen
│       ├── kube/           # K8s YAML gen
│       └── template/       # Template management
│
└── gateway/                # REST to gRPC gateway
```

## Core Concepts

### 1. goctl Code Generation

```bash
# Generate API service from .api file
goctl api go -api greet.api -dir ./greet

# Generate RPC service from .proto file
goctl rpc protoc greet.proto \
  --go_out=./greet \
  --go-grpc_out=./greet \
  --zrpc_out=./greet

# Generate model from database
goctl model mysql datasource \
  -url="root:pass@tcp(localhost:3306)/db" \
  -table="user" \
  -dir="./model"

# Generate Dockerfile
goctl docker -go greet.go

# Generate K8s deployment
goctl kube deploy -image myapp:latest -o deploy.yaml
```

### 2. API Definition Language

```api
// greet.api

// Type definitions
type Request {
    Name string `path:"name,options=you|world"`
}

type Response {
    Message string `json:"message"`
}

// Service definition
@server(
    group: greeting
    host: "0.0.0.0"
    port: 8888
    jwt: Auth
    signature: false
)
service greet-srv {
    @doc "Greet a person"
    @handler GreetHandler
    get /greet/from/:name(Request) returns (Response)

    @handler CreateHandler
    post /greet(CreateRequest) returns (CreateResponse)
}
```

### 3. Configuration

```yaml
# greet.yaml
Name: greet-service
Host: 0.0.0.0
Port: 8888

# Logging
Log:
  Level: info
  Mode: file
  Path: /var/log/greet
  KeepDays: 7

# Prometheus
Prometheus:
  Host: 0.0.0.0
  Port: 9091
  Path: /metrics

# Tracing
Telemetry:
  Name: greet-service
  Endpoint: http://jaeger:14268/api/traces
  Sampler: 1.0

# Circuit breaker
CircuitBreaker:
  enabled: true

# Rate limit
Limit:
  maxConns: 10000
  maxBytes: 1048576
```

### 4. Generated Project Structure

```
greet/
├── etc/
│   └── greet.yaml          # Configuration
├── internal/
│   ├── config/
│   │   └── config.go       # Config struct
│   ├── handler/
│   │   ├── greethandler.go # HTTP handler
│   │   ├── route.go        # Route setup
│   │   └── routes.go       # Route registration
│   ├── logic/
│   │   └── greetlogic.go   # Business logic
│   ├── svc/
│   │   └── servicecontext.go # Dependencies
│   └── types/
│       └── types.go        # API types
├── greet.go                 # Entry point
└── go.mod
```

## Resilience Patterns

### Adaptive Circuit Breaker

```go
import "github.com/zeromicro/go-zero/core/breaker"

// Automatic circuit breaking
func handleRequest(w http.ResponseWriter, r *http.Request) {
    breaker.Do("serviceName", func() {
        // Business logic
        result, err := callExternalService()
        if err != nil {
            breaker.MarkFailed()
            return
        }
        breaker.MarkSuccess()
    })
}

// Adaptive algorithm:
// - Tracks success/failure ratios
// - Opens circuit when failure rate exceeds threshold
// - Half-open state for recovery testing
// - Auto-closes when healthy
```

### Rate Limiting

```go
import "github.com/zeromicro/go-zero/core/limit"

// Period limit (requests per time window)
limiter := limit.NewPeriodLimiter(
    redis.RedisConf{Host: "localhost:6379"},
    "rate-limit:key",
    limit.PeriodLimiterConf{
        Period: time.Second,
        Quota: 100, // 100 requests/second
    },
)

if !limiter.Allow() {
    http.Error(w, "rate limit exceeded", http.StatusTooManyRequests)
    return
}

// Token bucket (burst allowance)
limiter := limit.NewTokenLimiter(
    redis.RedisConf{Host: "localhost:6379"},
    "token-limit:key",
    limit.TokenLimiterConf{
        Rate:  100,  // 100 tokens/second
        Burst: 1000, // Max burst
    },
)
```

### Load Shedding

```go
import "github.com/zeromicro/go-zero/core/load"

// Adaptive load shedding based on system metrics
shedder := load.NewAdaptiveShedder(
    load.WithWindow(time.Second),
    load.WithBuckets(100),
)

protectedHandler := func(w http.ResponseWriter, r *http.Request) {
    promise, err := shedder.Allow()
    if err != nil {
        http.Error(w, "service unavailable", http.StatusServiceUnavailable)
        return
    }

    defer promise.Done(func(err error) {
        if err != nil {
            promise.Fail()
        }
    })

    // Business logic
}
```

## Service Discovery

### Etcd Integration

```yaml
# Server configuration
Etcd:
  Hosts:
    - 127.0.0.1:2379
  Key: greet.rpc
  Ttl: 30

# Client configuration
Etcd:
  Hosts:
    - 127.0.0.1:2379
  Key: greet.rpc
```

```go
// Server registration
server := zrpc.MustNewServer(c.RpcServerConf, func(grpcServer *grpc.Server) {
    pb.RegisterGreeterServer(grpcServer, NewGreeterServer(svcCtx))
})
server.Start()

// Client connection
client := pb.NewGreeter(zrpc.MustNewClient(
    zrpc.RpcClientConf{
        Etcd: etcd.EtcdConf{
            Hosts: []string{"127.0.0.1:2379"},
            Key:   "greet.rpc",
        },
    },
))
```

### Consul Integration

```yaml
Consul:
  Host: 127.0.0.1:8500
  Key: greet.rpc
  Scheme: http
```

### K8s Service Discovery

```yaml
Kube:
  Namespace: default
  Selector: app=greet
  Port: 9090
```

## Observability

### Logging (logx)

```go
import "github.com/zeromicro/go-zero/core/logx"

// Basic logging
logx.Info("User logged in")
logx.Error("Database error:", err)
logx.Debugf("Processing item: %d", itemID)

// Context-aware logging
logx.WithContext(ctx).Infof("Processing request")

// Structured logging
logx.WithFields(map[string]interface{}{
    "user_id": userId,
    "action":  "login",
}).Info("User action")

// Duration logging
logx.WithDuration(duration).Info("Request completed")

// Slow log detection
defer logx.Slow("slow-operation", time.Second)()
```

### Prometheus Metrics

```go
import "github.com/zeromicro/go-zero/core/metric"

// Counter
requestCounter := metric.NewCounterVec(&metric.CounterVecOpts{
    Namespace: "greet",
    Subsystem: "api",
    Name:      "requests_total",
    Help:      "Total API requests",
    Labels:    []string{"method", "status"},
})
requestCounter.WithLabelValues("GET", "200").Inc()

// Gauge
activeConnections := metric.NewGaugeVec(&metric.GaugeVecOpts{
    Namespace: "greet",
    Subsystem: "connections",
    Name:      "active",
    Help:      "Active connections",
})
activeConnections.Set(100)

// Histogram
requestDuration := metric.NewHistogramVec(&metric.HistogramVecOpts{
    Namespace: "greet",
    Subsystem: "requests",
    Name:      "duration_seconds",
    Help:      "Request duration",
    Buckets:   []float64{0.01, 0.05, 0.1, 0.5, 1.0},
})
requestDuration.WithLabelValues("GET").Observe(0.05)
```

### Distributed Tracing

```yaml
Telemetry:
  Name: greet-service
  Endpoint: http://jaeger:14268/api/traces
  Sampler: 1.0
  Batcher: jaeger
```

```go
import (
    "go.opentelemetry.io/otel/trace"
    "github.com/zeromicro/go-zero/core/trace"
)

// Start span
ctx, span := trace.StartSpan(ctx, "operationName")
defer span.End()

// Set attributes
span.SetAttributes(attribute.String("key", "value"))

// Record error
span.RecordError(err)

// Add event
span.AddEvent("custom-event")
```

## Middleware System

### Built-in Middleware

```go
// Authentication
@server(
    jwt: Auth
    signature: false
)

// Rate limiting
@server(
    limit:
        maxConns: 10000
)

// CORS
@server(
    cors: true
)
```

### Custom Middleware

```go
// Logging middleware
type LoggingMiddleware struct{}

func (m *LoggingMiddleware) Handle(next http.HandlerFunc) http.HandlerFunc {
    return func(w http.ResponseWriter, r *http.Request) {
        start := time.Now()

        logx.Infof("Started %s %s", r.Method, r.URL.Path)

        ww := middleware.WrapResponseWriter(w)
        next(ww, r)

        logx.WithDuration(time.Since(start)).
            WithFields(map[string]interface{}{
                "status": ww.Status(),
                "path":   r.URL.Path,
            }).
            Info("Request completed")
    }
}

// Apply middleware
engine := rest.MustNewServer(c.RestConf,
    rest.WithMiddleware(NewLoggingMiddleware().Handle),
    rest.WithMiddleware(auth.Handle),
)
```

## Testing

### Unit Testing

```go
package logic

import (
    "context"
    "testing"
    "github.com/stretchr/testify/assert"
    "yourproject/greet/internal/svc"
)

func TestGreetLogic(t *testing.T) {
    ctx := context.Background()
    svcCtx := svc.NewServiceContext()
    logic := NewGreetLogic(ctx, svcCtx)

    req := &types.Request{Name: "World"}
    resp, err := logic.Greet(req)

    assert.NoError(t, err)
    assert.Equal(t, "Hello, World!", resp.Message)
}
```

### Integration Testing

```go
func TestGreetHandler(t *testing.T) {
    server := rest.MustNewServer(c.RestConf)
    defer server.Stop()

    // Register routes
    handler.RegisterHandlers(server, svcCtx)

    // Start server
    go server.Start()

    // Make test request
    resp, err := http.Get("http://localhost:8888/greet/from/world")
    assert.NoError(t, err)
    assert.Equal(t, http.StatusOK, resp.StatusCode)
}
```

## Production Deployment

### Docker

```dockerfile
# Dockerfile (generated by goctl)
FROM golang:1.20-alpine AS builder

RUN apk add --no-cache upx

WORKDIR /go/src
COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 go build -o greet greet.go
RUN upx greet

FROM alpine:latest
RUN apk add --no-cache ca-certificates

COPY --from=builder /go/src/greet /greet
COPY --from=builder /go/src/greet.yaml /greet.yaml

EXPOSE 8888
CMD ["/greet", "-f", "/greet.yaml"]
```

### Kubernetes

```yaml
# deploy.yaml (generated by goctl)
apiVersion: apps/v1
kind: Deployment
metadata:
  name: greet
spec:
  replicas: 3
  selector:
    matchLabels:
      app: greet
  template:
    spec:
      containers:
      - name: greet
        image: myregistry/greet:latest
        ports:
        - containerPort: 8888
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
            port: 8888
          initialDelaySeconds: 10
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: greet
spec:
  selector:
    app: greet
  ports:
  - port: 80
    targetPort: 8888
```

---

## Related Deep Dives

- [00-zero-to-go-zero-engineer.md](./00-zero-to-go-zero-engineer.md) - Fundamentals
- [02-goctl-deep-dive.md](./02-goctl-deep-dive.md) - Code generation
- [03-resilience-patterns-deep-dive.md](./03-resilience-patterns-deep-dive.md) - Circuit breaking, rate limiting
