---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/zeromicro
explored_at: 2026-03-30
prerequisites: Go programming basics, Microservices concepts helpful
---

# Zero to Go-Zero Engineer - Complete Fundamentals

## Table of Contents

1. [What is go-zero?](#what-is-go-zero)
2. [Why Go-Zero?](#why-go-zero)
3. [Installation](#installation)
4. [Your First API Service](#your-first-api-service)
5. [Your First RPC Service](#your-first-rpc-service)
6. [Database Integration](#database-integration)
7. [Middleware](#middleware)
8. [Service Discovery](#service-discovery)
9. [Observability](#observability)

## What is go-zero?

go-zero is a **high-performance microservices framework** for Go that emphasizes resilience and productivity. It includes goctl, a powerful code generation tool that accelerates development from API definitions to production-ready services.

### The Problem go-zero Solves

**Without go-zero:**
```
Define API → Manually write handlers
           → Write router setup
           → Write middleware
           → Add rate limiting
           → Add circuit breaking
           → Add tracing/monitoring
Complexity: Repetitive boilerplate, easy to miss resilience patterns
```

**With go-zero:**
```
Write .api file → goctl generate → Production-ready service
                               → Built-in rate limiting
                               → Circuit breaking
                               → Tracing & monitoring
Simplicity: Focus on business logic, framework handles the rest
```

### Key Concepts

| Term | Definition |
|------|------------|
| **goctl** | Code generation CLI tool |
| **API** | HTTP service definition language |
| **RPC** | gRPC-based service definition |
| **ZRPC** | go-zero's RPC framework |
| **Rest Engine** | High-performance HTTP server |
| **Adaptive Circuit Breaker** | Auto-adjusting protection |

## Why Go-Zero?

### Benefits

1. **Code Generation**: goctl generates 80% of boilerplate
2. **High Performance**: Optimized for 100k+ QPS
3. **Resilience**: Built-in circuit breaking, rate limiting
4. **Service Discovery**: Etcd, Consul, K8s support
5. **Observability**: Prometheus, Jaeger, OpenTelemetry
6. **Multiple Protocols**: REST, gRPC, GraphQL

### When to Use Go-Zero

**Good fit:**
- High-concurrency microservices
- API gateways
- gRPC service mesh
- Rapid service development
- Production-grade resilience needs

**Not recommended:**
- Simple single-service apps
- Prototyping without code gen
- Non-Go projects

## Installation

### Install Go

```bash
# Requires Go 1.18+
go version
```

### Install goctl

```bash
# Method 1: go install (recommended)
go install github.com/zeromicro/go-zero/tools/goctl@latest

# Method 2: Source
git clone https://github.com/zeromicro/go-zero.git
cd go-zero/tools/goctl
go build
```

### Verify Installation

```bash
goctl --version

# Output: goctl version x.x.x
```

### Project Setup

```bash
# Create project directory
mkdir -p github.com/yourname/microservices
cd github.com/yourname/microservices

# Initialize go module
go mod init github.com/yourname/microservices
```

## Your First API Service

### Step 1: Create API File

```api
// greet.api

type Request {
    Name string `path:"name,options=you|world"`
}

type Response {
    Message string `json:"message"`
}

@server(
    group: greeting
    jwt: Auth
)
service greet-srv {
    @handler GreetHandler
    get /greet/from/:name(Request) returns (Response)
}
```

### Step 2: Generate Code

```bash
goctl api go -api greet.api -dir ./greet
```

**Generated structure:**
```
greet/
├── etc/
│   └── greet.yaml          # Config file
├── internal/
│   ├── config/
│   │   └── config.go       # Config struct
│   ├── handler/
│   │   ├── greethandler.go # HTTP handler
│   │   └── routes.go       # Route registration
│   ├── logic/
│   │   └── greetlogic.go   # Business logic
│   ├── svc/
│   │   └── servicecontext.go # Service dependencies
│   └── types/
│       └── types.go        # Request/Response types
├── greet.go                 # Entry point
└── greet.yaml               # Config (copy from etc/)
```

### Step 3: Implement Logic

```go
// internal/logic/greetlogic.go
package logic

import (
    "fmt"
    "net/http"
    "github.com/zeromicro/go-zero/rest/httpx"
    "yourproject/greet/internal/svc"
    "yourproject/greet/internal/types"
)

type GreetLogic struct {
    logx.Logger
    ctx    context.Context
    svcCtx *svc.ServiceContext
}

func NewGreetLogic(ctx context.Context, svcCtx *svc.ServiceContext) *GreetLogic {
    return &GreetLogic{
        Logger: logx.WithContext(ctx),
        ctx:    ctx,
        svcCtx: svcCtx,
    }
}

func (l *GreetLogic) Greet(req *types.Request) (*types.Response, error) {
    message := fmt.Sprintf("Hello, %s!", req.Name)
    return &types.Response{Message: message}, nil
}
```

### Step 4: Run Service

```bash
cd greet
go run greet.go -f greet.yaml

# Server starts on port 8888
```

### Test the Service

```bash
curl http://localhost:8888/greet/from/world

# Response: {"message":"Hello, world!"}
```

## Your First RPC Service

### Step 1: Create Proto File

```proto
// greet.proto

syntax = "proto3";

package greet;

message Request {
    string name = 1;
}

message Response {
    string message = 1;
}

service Greeter {
    rpc Greet(Request) returns (Response);
}
```

### Step 2: Generate RPC Code

```bash
goctl rpc protoc greet.proto --go_out=./greet --go-grpc_out=./greet --zrpc_out=./greet
```

### Step 3: Implement Server

```go
// internal/server/greetserver.go
package server

import (
    "context"
    "fmt"
    "github.com/zeromicro/go-zero/core/logx"
    "yourproject/greet/internal/svc"
    pb "yourproject/greet/pb"
)

type GreeterServer struct {
    pb.UnimplementedGreeterServer
    svcCtx *svc.ServiceContext
}

func NewGreeterServer(svcCtx *svc.ServiceContext) *GreeterServer {
    return &GreeterServer{svcCtx: svcCtx}
}

func (s *GreeterServer) Greet(ctx context.Context, req *pb.Request) (*pb.Response, error) {
    logx.WithContext(ctx).Infof("Greet request: %v", req)

    message := fmt.Sprintf("Hello, %s!", req.Name)

    return &pb.Response{Message: message}, nil
}
```

### Step 4: Configure and Run

```yaml
# greet.yaml
Name: greet.rpc
ListenOn: 0.0.0.0:9090
Etcd:
  Hosts:
    - 127.0.0.1:2379
  Key: greet.rpc
```

```bash
go run greet.go -f greet.yaml
```

## Database Integration

### Generate Model from SQL

```sql
-- user.sql
CREATE TABLE `user` (
    `id` bigint NOT NULL AUTO_INCREMENT,
    `name` varchar(50) NOT NULL,
    `email` varchar(100) NOT NULL,
    `created_at` datetime DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`)
);
```

```bash
goctl model mysql datasource -url="root:password@tcp(localhost:3306)/mydb" -table="user" -dir="./internal/model"
```

### Generated Model

```go
// internal/model/usermodel.go
type UserModel interface {
    Insert(ctx context.Context, data *User) (sql.Result, error)
    FindOne(ctx context.Context, id int64) (*User, error)
    FindAll(ctx context.Context, offset, limit int) ([]*User, error)
    Update(ctx context.Context, data *User) error
    Delete(ctx context.Context, id int64) error
}
```

### Using Model in Logic

```go
// internal/logic/userlogic.go
func (l *UserLogic) GetUser(req *types.GetUserRequest) (*types.UserResponse, error) {
    user, err := l.svcCtx.UserModel.FindOne(l.ctx, req.Id)
    if err != nil {
        return nil, err
    }

    return &types.UserResponse{
        Id:    user.Id,
        Name:  user.Name,
        Email: user.Email,
    }, nil
}
```

## Middleware

### Built-in Middleware

```go
// Configuration enables middleware
log:
  level: info
  mode: file
  path: /var/log/greet.log

auth:
  accessSecret: your-secret-key
  accessExpire: 86400

limit:
  maxConns: 10000
  maxBytes: 1048576  # 1MB
```

### Custom Middleware

```go
// middleware/logging.go
package middleware

import (
    "net/http"
    "time"
    "github.com/zeromicro/go-zero/core/logx"
)

type LoggingMiddleware struct{}

func NewLoggingMiddleware() *LoggingMiddleware {
    return &LoggingMiddleware{}
}

func (m *LoggingMiddleware) Handle(next http.HandlerFunc) http.HandlerFunc {
    return func(w http.ResponseWriter, r *http.Request) {
        start := time.Now()

        logx.Infof("Started %s %s", r.Method, r.URL.Path)

        next(w, r)

        logx.Infof("Completed %s in %v", r.URL.Path, time.Since(start))
    }
}
```

### Apply Middleware

```go
// In handler setup
engine := rest.MustNewServer(c.RestConf,
    rest.WithMiddleware(logging.Handle),
    rest.WithMiddleware(auth.Handle),
)
```

## Service Discovery

### Etcd Registration

```yaml
# Client config
Etcd:
  Hosts:
    - 127.0.0.1:2379
  Key: greet.rpc
```

```go
// Client setup
client := pb.NewGreeter(zrpc.MustNewClient(
    zrpc.RpcClientConf{
        Etcd: etcd.EtcdConf{
            Hosts: []string{"127.0.0.1:2379"},
            Key:   "greet.rpc",
        },
    },
))
```

### Consul Registration

```yaml
Consul:
  Host: 127.0.0.1:8500
  Key: greet.rpc
```

### K8s Service Discovery

```yaml
Kube:
  Namespace: default
  Selector: app=greet
```

## Observability

### Prometheus Metrics

```yaml
# Enable Prometheus
Prometheus:
  Host: 0.0.0.0
  Port: 9091
  Path: /metrics
```

**Auto-collected metrics:**
- Request count
- Request duration
- Active connections
- Circuit breaker state

### Jaeger Tracing

```yaml
Telemetry:
  Name: greet-service
  Endpoint: http://jaeger:14268/api/traces
  Sampler: 1.0
  Batcher: jaeger
```

### Structured Logging

```go
import "github.com/zeromicro/go-zero/core/logx"

// Info logging
logx.Info("User logged in", "user_id", 123)

// Error logging
logx.WithContext(ctx).Errorf("Database error: %v", err)

// Structured fields
logx.WithContext(ctx).
    WithDuration(duration).
    WithFields(map[string]interface{}{
        "user_id": userId,
        "action":  "login",
    }).
    Info("Action completed")
```

### Health Checks

```go
// Add health endpoint
@handler HealthHandler
get /health returns (HealthResponse)

// Handler
func (l *HealthLogic) Health() (*HealthResponse, error) {
    return &HealthResponse{
        Status: "ok",
        Time:   time.Now().Unix(),
    }, nil
}
```

---

**Next Steps:**
- [01-go-zero-exploration.md](./01-go-zero-exploration.md) - Full architecture
- [02-goctl-deep-dive.md](./02-goctl-deep-dive.md) - Code generation
- [03-resilience-patterns-deep-dive.md](./03-resilience-patterns-deep-dive.md) - Circuit breaking, rate limiting
