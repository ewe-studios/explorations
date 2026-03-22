# Taubyte HTTP Utilities - Comprehensive Deep-Dive Exploration

**Date:** 2026-03-22
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/http/`

---

## 1. Purpose and Overview

The **Taubyte HTTP Utilities** library provides HTTP server and client capabilities for Taubyte applications. It includes both a basic HTTP server with routing capabilities and secure HTTP handling with authentication support.

### Key Characteristics

- **Module Path:** `github.com/taubyte/http`
- **Go Version:** 1.21+
- **License:** BSD 3-Clause
- **Purpose:** HTTP server/client utilities for Taubyte services

---

## 2. Architecture

### 2.1 Module Structure

```
http/
├── auth/                   # Authentication
│   ├── scope.go           # Auth scope definitions
│   ├── types.go           # Auth types
│   └── vars.go            # Auth variables
├── basic/                  # Basic HTTP server
│   ├── assets.go          # Static asset serving
│   ├── methods.go         # HTTP method handlers
│   ├── new.go             # Server creation
│   ├── routes.go          # Routing
│   ├── types.go           # Server types
│   ├── vars.go            # Server variables
│   ├── websocket.go       # WebSocket support
│   ├── example/           # Usage examples
│   └── secure/            # Secure HTTP handling
│       ├── methods.go     # Secure methods
│       ├── new.go         # Secure server creation
│       ├── types.go       # Secure types
│       └── vars.go        # Secure variables
├── context/                # Request context
│   ├── helpers.go         # Context helpers
│   ├── methods.go         # Context methods
│   └── types.go           # Context types
├── helpers/                # General helpers
│   ├── http.go            # HTTP utilities
│   └── new.go             # Helper creation
├── mocks/                  # Mock implementations
│   └── methods.go         # Mock methods
├── options/                # Configuration options
│   └── options.go         # Server options
├── request/                # Request handling
│   └── request.go         # Request utilities
├── response/               # Response handling
│   └── response.go        # Response utilities
└── middleware/             # HTTP middleware
    └── middleware.go      # Middleware definitions
```

---

## 3. Key Types, Interfaces, and APIs

### 3.1 Basic HTTP Server

```go
package basic

type Server struct {
    mux      *http.ServeMux
    server   *http.Server
    handlers map[string]Handler
    routes   []Route
}

type Handler func(w http.ResponseWriter, r *http.Request) error

type Route struct {
    Method  string
    Path    string
    Handler Handler
}

// Create new server
func New(opts ...Option) (*Server, error)

// Server methods
func (s *Server) Handle(method, path string, handler Handler)
func (s *Server) Get(path string, handler Handler)
func (s *Server) Post(path string, handler Handler)
func (s *Server) Put(path string, handler Handler)
func (s *Server) Delete(path string, handler Handler)
func (s *Server) Listen(addr string) error
func (s *Server) Close() error
```

### 3.2 Server Options

```go
package options

type Option func(*Config)

// Common options
func WithAddr(addr string) Option
func WithReadTimeout(timeout time.Duration) Option
func WithWriteTimeout(timeout time.Duration) Option
func WithIdleTimeout(timeout time.Duration) Option
func WithMaxHeaderBytes(bytes int) Option
func WithTLS(certFile, keyFile string) Option
func WithMiddleware(middleware ...Middleware) Option
func WithCors(config CorsConfig) Option

// Usage
server, err := basic.New(
    options.WithAddr(":8080"),
    options.WithReadTimeout(30 * time.Second),
    options.WithWriteTimeout(30 * time.Second),
    options.WithMiddleware(loggingMiddleware),
)
```

### 3.3 Routing

```go
package basic

// Route registration
func (s *Server) Handle(method, path string, handler Handler) {
    s.routes = append(s.routes, Route{
        Method:  method,
        Path:    path,
        Handler: handler,
    })

    s.mux.HandleFunc(path, func(w http.ResponseWriter, r *http.Request) {
        if r.Method != method {
            http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
            return
        }

        if err := handler(w, r); err != nil {
            http.Error(w, err.Error(), http.StatusInternalServerError)
        }
    })
}

// Convenience methods
func (s *Server) Get(path string, handler Handler) {
    s.Handle("GET", path, handler)
}

func (s *Server) Post(path string, handler Handler) {
    s.Handle("POST", path, handler)
}
```

### 3.4 Request Context

```go
package context

type Context struct {
    Request  *http.Request
    Response http.ResponseWriter
    Values   map[string]interface{}
}

// Context methods
func (c *Context) Get(key string) interface{}
func (c *Context) Set(key string, value interface{})
func (c *Context) JSON(status int, data interface{}) error
func (c *Context) String(status int, format string, values ...interface{}) error
func (c *Context) Bind(obj interface{}) error
func (c *Context) Param(key string) string
func (c *Context) Query(key string) string
func (c *Context) Header(key string) string

// Create context from request/response
func New(w http.ResponseWriter, r *http.Request) *Context
```

### 3.5 Authentication

```go
package auth

type Scope string

const (
    ScopeRead   Scope = "read"
    ScopeWrite  Scope = "write"
    ScopeAdmin  Scope = "admin"
)

type AuthConfig struct {
    Enabled      bool
    TokenHeader  string
    TokenQuery   string
    Scopes       []Scope
    Validator    TokenValidator
}

type TokenValidator func(token string) (*Claims, error)

type Claims struct {
    Subject string
    Scopes  []Scope
    Expires time.Time
}

// Auth middleware
func AuthMiddleware(config AuthConfig) Middleware {
    return func(next http.Handler) http.Handler {
        return http.HandlerFunc(func(w, r *http.Request) {
            token := extractToken(r, config)
            if token == "" {
                http.Error(w, "Unauthorized", http.StatusUnauthorized)
                return
            }

            claims, err := config.Validator(token)
            if err != nil {
                http.Error(w, "Invalid token", http.StatusUnauthorized)
                return
            }

            // Add claims to context
            ctx := context.WithValue(r.Context(), "claims", claims)
            next.ServeHTTP(w, r.WithContext(ctx))
        })
    }
}
```

### 3.6 Secure HTTP Server

```go
package secure

type SecureServer struct {
    server     *basic.Server
    tlsConfig  *tls.Config
    hsts       bool
    csp        string
}

// Create secure server
func New(opts ...SecureOption) (*SecureServer, error)

// Security headers
func (s *SecureServer) SecurityHeaders(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        // HSTS
        if s.hsts {
            w.Header().Set("Strict-Transport-Security", "max-age=31536000")
        }

        // CSP
        if s.csp != "" {
            w.Header().Set("Content-Security-Policy", s.csp)
        }

        // X-Content-Type-Options
        w.Header().Set("X-Content-Type-Options", "nosniff")

        // X-Frame-Options
        w.Header().Set("X-Frame-Options", "DENY")

        // X-XSS-Protection
        w.Header().Set("X-XSS-Protection", "1; mode=block")

        next.ServeHTTP(w, r)
    })
}
```

### 3.7 WebSocket Support

```go
package basic

type WebSocketConfig struct {
    ReadBufferSize  int
    WriteBufferSize int
    CheckOrigin     func(r *http.Request) bool
    HandshakeTimeout time.Duration
}

func (s *Server) WebSocket(path string, handler WSHandler, config WebSocketConfig) {
    upgrader := websocket.Upgrader{
        ReadBufferSize:  config.ReadBufferSize,
        WriteBufferSize: config.WriteBufferSize,
        CheckOrigin:     config.CheckOrigin,
    }

    s.Get(path, func(w http.ResponseWriter, r *http.Request) error {
        conn, err := upgrader.Upgrade(w, r, nil)
        if err != nil {
            return err
        }
        defer conn.Close()

        return handler(conn, r)
    })
}

type WSHandler func(conn *websocket.Conn, r *http.Request) error
```

### 3.8 Static Assets

```go
package basic

// Serve static files
func (s *Server) Static(path, root string) {
    fs := http.FileServer(http.Dir(root))

    if path == "/" {
        s.mux.Handle("/", fs)
    } else {
        s.mux.Handle(path+"/", http.StripPrefix(path, fs))
    }
}

// Serve single file
func (s *Server) StaticFile(path, filename string, contentType string) {
    s.Get(path, func(w http.ResponseWriter, r *http.Request) error {
        w.Header().Set("Content-Type", contentType)
        http.ServeFile(w, r, filename)
        return nil
    })
}
```

---

## 4. Integration with Taubyte Components

### 4.1 VM Integration

HTTP utilities are used by TVM for:
- **HTTP Event Handling:** Incoming HTTP requests to WASM modules
- **API Gateway:** Exposing WASM functions via HTTP endpoints
- **Health Checks:** Service health monitoring

### 4.2 SDK Integration

The HTTP module provides the underlying implementation for SDK HTTP features:

```go
// go-sdk/http/event/methods.go
func (e Event) Method() string {
    return getMethod(e.event)
}

func (e Event) Path() string {
    return getPath(e.event)
}
```

### 4.3 Used By

| Component | Usage |
|-----------|-------|
| `tau` | API server |
| `taucorder` | Management API |
| `vm` | HTTP event handling |

---

## 5. Production Usage Patterns

### 5.1 Basic Server Example

```go
package main

import (
    "github.com/taubyte/http/basic"
    "github.com/taubyte/http/options"
)

func main() {
    server, _ := basic.New(
        options.WithAddr(":8080"),
        options.WithReadTimeout(30*time.Second),
        options.WithWriteTimeout(30*time.Second),
    )

    // Routes
    server.Get("/health", func(w http.ResponseWriter, r *http.Request) error {
        w.Write([]byte("OK"))
        return nil
    })

    server.Post("/api/data", handleData)

    // Start server
    server.Listen(":8080")
}

func handleData(w http.ResponseWriter, r *http.Request) error {
    var data Data
    json.NewDecoder(r.Body).Decode(&data)

    // Process data...

    w.Header().Set("Content-Type", "application/json")
    return json.NewEncoder(w).Encode(data)
}
```

### 5.2 Secure Server Example

```go
package main

import (
    "github.com/taubyte/http/basic/secure"
    "github.com/taubyte/http/auth"
)

func main() {
    server, _ := secure.New(
        secure.WithAddr(":443"),
        secure.WithTLS("cert.pem", "key.pem"),
        secure.WithHSTS(true),
    )

    // Auth middleware
    authConfig := auth.AuthConfig{
        Enabled: true,
        Validator: validateToken,
        Scopes: []auth.Scope{auth.ScopeRead},
    }

    server.Use(secure.AuthMiddleware(authConfig))

    server.Get("/api/protected", protectedHandler)
    server.Listen(":443")
}
```

### 5.3 Context Example

```go
func handler(w http.ResponseWriter, r *http.Request) error {
    ctx := context.New(w, r)

    // Get parameters
    id := ctx.Param("id")
    query := ctx.Query("search")

    // Get header
    auth := ctx.Header("Authorization")

    // Bind JSON
    var data MyStruct
    if err := ctx.Bind(&data); err != nil {
        return err
    }

    // Set value for middleware
    ctx.Set("userId", data.UserID)

    // Respond
    return ctx.JSON(200, map[string]interface{}{
        "status": "success",
        "data": data,
    })
}
```

### 5.4 Middleware Example

```go
// Logging middleware
func LoggingMiddleware(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        start := time.Now()

        // Wrap response writer to capture status
        wrapped := &responseWriter{ResponseWriter: w, status: 200}

        next.ServeHTTP(wrapped, r)

        log.Printf(
            "%s %s %d %v",
            r.Method,
            r.URL.Path,
            wrapped.status,
            time.Since(start),
        )
    })
}

// CORS middleware
func CorsMiddleware(config CorsConfig) Middleware {
    return func(next http.Handler) http.Handler {
        return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
            w.Header().Set("Access-Control-Allow-Origin", config.AllowedOrigin)
            w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
            w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

            if r.Method == "OPTIONS" {
                w.WriteHeader(http.StatusNoContent)
                return
            }

            next.ServeHTTP(w, r)
        })
    }
}
```

---

## 6. Security Considerations

### 6.1 Security Headers

```go
// Recommended security headers
w.Header().Set("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
w.Header().Set("Content-Security-Policy", "default-src 'self'")
w.Header().Set("X-Content-Type-Options", "nosniff")
w.Header().Set("X-Frame-Options", "DENY")
w.Header().Set("X-XSS-Protection", "1; mode=block")
w.Header().Set("Referrer-Policy", "strict-origin-when-cross-origin")
```

### 6.2 TLS Configuration

```go
tlsConfig := &tls.Config{
    MinVersion: tls.VersionTLS12,
    CipherSuites: []uint16{
        tls.TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
        tls.TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
        tls.TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
    },
    PreferServerCipherSuites: true,
}
```

### 6.3 Rate Limiting

```go
func RateLimiter(requestsPerSecond int) Middleware {
    limiter := rate.NewLimiter(rate.Every(time.Second/time.Duration(requestsPerSecond)), requestsPerSecond)

    return func(next http.Handler) http.Handler {
        return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
            if !limiter.Allow() {
                http.Error(w, "Too many requests", http.StatusTooManyRequests)
                return
            }
            next.ServeHTTP(w, r)
        })
    }
}
```

---

## 7. Related Components

| Component | Path | Description |
|-----------|------|-------------|
| go-sdk | `../go-sdk/` | SDK HTTP client/event |
| vm | `../vm/` | HTTP event handling |
| rust-sdk | `../rust-sdk/` | Rust SDK HTTP module |

---

## 8. Documentation References

- **Official Docs:** https://tau.how
- **GoDoc:** https://pkg.go.dev/github.com/taubyte/http

---

*This document was generated as part of a comprehensive Taubyte codebase exploration.*
