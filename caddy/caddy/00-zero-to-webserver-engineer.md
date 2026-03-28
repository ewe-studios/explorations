# Zero to Web Server Engineer

**Location:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/caddy/caddy/`
**Source:** Caddy web server (`/home/darkvoid/Boxxed/@formulas/src.rust/src.caddy/caddy/`)
**Prerequisites:** None - starts from first principles
**Outcome:** Understanding of HTTP, TLS, reverse proxy, and connection handling

---

## Table of Contents

1. [What is a Web Server?](#1-what-is-a-web-server)
2. [HTTP Protocol Fundamentals](#2-http-protocol-fundamentals)
3. [TLS/HTTPS from First Principles](#3-tlshttps-from-first-principles)
4. [Reverse Proxy Patterns](#4-reverse-proxy-patterns)
5. [Connection Handling Basics](#5-connection-handling-basics)
6. [From Zero to Caddy](#6-from-zero-to-caddy)

---

## 1. What is a Web Server?

### 1.1 The Basic Concept

A web server is a program that:
1. **Listens** on a network port (typically 80 for HTTP, 443 for HTTPS)
2. **Accepts** incoming connections from clients (browsers, APIs, etc.)
3. **Reads** HTTP requests from those connections
4. **Processes** the requests (serve files, proxy to backends, etc.)
5. **Writes** HTTP responses back to the clients
6. **Manages** resources (connections, memory, certificates)

### 1.2 Minimal HTTP Server in Go

```go
package main

import (
    "fmt"
    "net"
    "net/http"
)

func main() {
    // 1. Create a listener on port 8080
    ln, err := net.Listen("tcp", ":8080")
    if err != nil {
        panic(err)
    }
    defer ln.Close()

    fmt.Println("Server listening on :8080")

    // 2. Accept connections in a loop
    for {
        conn, err := ln.Accept()
        if err != nil {
            continue
        }

        // 3. Handle each connection in a goroutine
        go handleConnection(conn)
    }
}

func handleConnection(conn net.Conn) {
    defer conn.Close()

    // 4. Read HTTP request (simplified)
    buf := make([]byte, 4096)
    n, err := conn.Read(buf)
    if err != nil {
        return
    }

    // 5. Parse request (very simplified)
    request := string(buf[:n])
    fmt.Printf("Received:\n%s\n", request)

    // 6. Write HTTP response
    response := "HTTP/1.1 200 OK\r\n" +
                "Content-Type: text/plain\r\n" +
                "Content-Length: 13\r\n" +
                "\r\n" +
                "Hello, World!"

    conn.Write([]byte(response))
}
```

### 1.3 What Caddy Adds

Caddy takes this basic model and adds:

| Feature | Basic Server | Caddy |
|---------|-------------|-------|
| TLS | Manual | Automatic (ACME) |
| Routing | Manual parsing | Configurable matchers |
| Middleware | Manual chaining | Module system |
| Proxying | Manual | Load balancing, health checks |
| Reloading | Restart required | Graceful (zero downtime) |
| Logging | fmt.Println | Structured, configurable |

---

## 2. HTTP Protocol Fundamentals

### 2.1 HTTP Request Structure

An HTTP request consists of:

```
GET /path?query=value HTTP/1.1\r\n
Host: example.com\r\n
User-Agent: Mozilla/5.0\r\n
Accept: text/html\r\n
Connection: keep-alive\r\n
\r\n
[optional body]
```

**Parts:**
1. **Request Line**: `METHOD PATH PROTOCOL`
2. **Headers**: `Key: Value` pairs
3. **Blank Line**: Separates headers from body
4. **Body**: Optional data (for POST, PUT, etc.)

### 2.2 HTTP Response Structure

```
HTTP/1.1 200 OK\r\n
Content-Type: text/html; charset=utf-8\r\n
Content-Length: 1234\r\n
Set-Cookie: session=abc123\r\n
\r\n
<!DOCTYPE html>...
```

**Parts:**
1. **Status Line**: `PROTOCOL STATUS_CODE STATUS_TEXT`
2. **Headers**: `Key: Value` pairs
3. **Blank Line**: Separates headers from body
4. **Body**: Response content

### 2.3 HTTP Methods

| Method | Purpose | Idempotent | Body |
|--------|---------|------------|------|
| GET | Retrieve resource | Yes | No |
| POST | Create resource | No | Yes |
| PUT | Replace resource | Yes | Yes |
| DELETE | Remove resource | Yes | No |
| PATCH | Partial update | No | Yes |
| HEAD | Get headers only | Yes | No |
| OPTIONS | Get capabilities | Yes | No |

### 2.4 Status Codes

| Range | Meaning | Common Codes |
|-------|---------|--------------|
| 1xx | Informational | 101 Switching Protocols |
| 2xx | Success | 200 OK, 201 Created, 204 No Content |
| 3xx | Redirection | 301 Moved, 302 Found, 304 Not Modified |
| 4xx | Client Error | 400 Bad Request, 401 Unauthorized, 404 Not Found |
| 5xx | Server Error | 500 Internal, 502 Bad Gateway, 503 Unavailable |

### 2.5 How Caddy Parses HTTP

Caddy uses Go's `net/http` package:

```go
// From modules/caddyhttp/server.go
func (s *Server) serveHTTP(w http.ResponseWriter, r *http.Request) {
    // 1. HTTP request already parsed by net/http.Server
    // 2. Extract host, path, headers from r
    // 3. Match against route matchers
    // 4. Execute middleware chain

    basereq := r.Clone(r.Context())

    // Route matching happens in the compiled middleware
    s.handlerChain.ServeHTTP(w, r)
}
```

---

## 3. TLS/HTTPS from First Principles

### 3.1 What Problem Does TLS Solve?

HTTP sends data in **plaintext**. Anyone intercepting the traffic can read:
- URLs and query parameters
- Headers (including cookies, auth tokens)
- Request/response bodies

**TLS (Transport Layer Security) adds:**
1. **Encryption**: Data is scrambled so only the intended recipient can read it
2. **Authentication**: You can verify you're talking to the real server
3. **Integrity**: Data cannot be modified without detection

### 3.2 The TLS Handshake (Simplified)

```
Client                                  Server
  |                                        |
  |-------- ClientHello ----------------->|
  |  (supported versions, cipher suites)   |
  |                                        |
  |<------- ServerCertificate ------------|
  |  (server's certificate + chain)        |
  |                                        |
  |-------- KeyExchange ----------------->|
  |  (encrypted premaster secret)          |
  |                                        |
  |<------- ServerHelloDone -------------|
  |                                        |
  | [Both derive session keys]             |
  |                                        |
  |<------- Finished -------------------->|
  |  (encrypted, verified)                 |
  |                                        |
  | [Encrypted HTTP traffic begins]        |
```

### 3.3 Certificates and Certificate Authorities

**Certificate Structure:**
```
Certificate {
    Subject: "example.com"
    Issuer: "Let's Encrypt Authority X3"
    PublicKey: RSA/ECDSA public key
    ValidFrom: 2026-01-01
    ValidTo: 2026-04-01
    Signature: (signed by CA's private key)
}
```

**Chain of Trust:**
```
Root CA (self-signed, pre-trusted by OS/browser)
    └── Intermediate CA (signed by Root CA)
        └── Leaf Certificate (signed by Intermediate, for your domain)
```

### 3.4 ACME Protocol (Automatic Certificate Management)

ACME (RFC 8555) is the protocol Caddy uses to automatically obtain certificates from Let's Encrypt:

```
1. Account Registration
   - Generate account key pair
   - Register with ACME server

2. Order Creation
   - Request certificate for domains
   - Receive authorizations

3. Challenge Completion
   - Prove you control the domain
   - HTTP-01: Place file at http://domain/.well-known/acme-challenge/
   - DNS-01: Add TXT record to DNS
   - TLS-ALPN-01: Respond to TLS handshake

4. Certificate Issuance
   - All challenges pass
   - CA issues certificate
   - Download and store certificate

5. Renewal
   - Monitor expiration
   - Repeat process before expiry
```

### 3.5 How Caddy Does Automatic HTTPS

```go
// From modules/caddytls/acmeissuer.go
type ACMEIssuer struct {
    CA           string  // ACME server URL
    Email        string  // Contact email
    AccountKey   string  // Existing account key (optional)
    Challenges   *ChallengesConfig
}

func (iss *ACMEIssuer) Issue(ctx context.Context, name string) (*x509.Certificate, error) {
    // 1. Create or load ACME account
    account, err := iss.getAccount(ctx)

    // 2. Create order for domain
    order, err := iss.template.NewOrder(ctx, acme.Order{
        Identifiers: []acme.Identifier{{Type: "dns", Value: name}},
    })

    // 3. Complete challenges (HTTP-01 by default)
    for _, auth := range order.Authorizations {
        chal := auth.Challenge("http-01")
        iss.solver.Solve(ctx, chal, name)
    }

    // 4. Wait for order to be ready
    order, err = iss.template.WaitForOrder(ctx, order)

    // 5. Fetch certificate
    cert, err := iss.template.FetchCertificate(ctx, order.Certificate)

    return cert, nil
}
```

### 3.6 OCSP Stapling

**OCSP (Online Certificate Status Protocol)** allows checking if a certificate has been revoked.

**Without stapling:**
```
Client -> Server: Certificate
Client -> CA: Is this cert revoked?
CA -> Client: OCSP Response
```

**With stapling (Caddy does this automatically):**
```
Server -> CA: OCSP Request (periodically)
CA -> Server: OCSP Response (signed, cached)
Server -> Client: Certificate + OCSP Response (stapled)
```

```go
// From modules/caddytls/tls.go
// Caddy staples OCSP responses automatically
func (t *TLS) stapleOCSP(ctx context.Context, config *certmagic.Config) {
    ticker := time.NewTicker(time.Duration(t.OCSPCheckInterval))
    for {
        select {
        case <-ticker.C:
            // Scan all managed certificates
            // Fetch fresh OCSP responses
            // Cache in storage
        case <-ctx.Done():
            return
        }
    }
}
```

---

## 4. Reverse Proxy Patterns

### 4.1 What is a Reverse Proxy?

A reverse proxy sits between clients and backend servers:

```
Client -> Reverse Proxy -> Backend Server
         (Caddy)          (your app)
```

**Why use a reverse proxy?**
1. **Load Balancing**: Distribute requests across multiple backends
2. **SSL Termination**: Handle TLS so backends don't have to
3. **Caching**: Cache responses to reduce backend load
4. **Security**: Hide backend infrastructure, add WAF
5. **Compression**: Compress responses before sending

### 4.2 Basic Reverse Proxy Flow

```go
// Simplified from modules/caddyhttp/reverseproxy/reverseproxy.go
func (h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request) error {
    // 1. Select an upstream (load balancing)
    upstream := h.selectBackend(r)

    // 2. Create new request for upstream
    backendReq, err := http.NewRequestWithContext(
        r.Context(),
        r.Method,
        upstream.URL + r.URL.Path,
        r.Body,
    )

    // 3. Copy headers
    copyHeaders(backendReq.Header, r.Header)
    backendReq.Header.Set("X-Forwarded-For", r.RemoteAddr)

    // 4. Send request to upstream
    resp, err := h.Transport.RoundTrip(backendReq)
    if err != nil {
        // 5. Handle failure (retry, failover)
        return h.handleFailure(r, upstream)
    }
    defer resp.Body.Close()

    // 6. Copy response back to client
    copyHeaders(w.Header(), resp.Header)
    w.WriteHeader(resp.StatusCode)
    io.Copy(w, resp.Body)

    return nil
}
```

### 4.3 Load Balancing Algorithms

**Round Robin:**
```
Request 1 -> Backend A
Request 2 -> Backend B
Request 3 -> Backend C
Request 4 -> Backend A (cycle repeats)
```

**Least Connections:**
```
Backend A: 5 active connections
Backend B: 2 active connections  <- Next request goes here
Backend C: 8 active connections
```

**IP Hash:**
```
Client IP 192.168.1.1 -> hash() -> Backend B
Client IP 192.168.1.2 -> hash() -> Backend A
(Same client always goes to same backend)
```

```go
// From modules/caddyhttp/reverseproxy/selectionpolicies.go
type RoundRobinSelection struct {
    counter atomic.Uint64
}

func (rr *RoundRobinSelection) Select(pool UpstreamPool) *Upstream {
    n := rr.counter.Add(1)
    available := pool.availableUpstreams()
    return available[n % uint64(len(available))]
}

type LeastConnSelection struct{}

func (lc *LeastConnSelection) Select(pool UpstreamPool) *Upstream {
    var best *Upstream
    var bestConnCount int64 = -1

    for _, u := range pool.availableUpstreams() {
        connCount := u.activeRequests()
        if best == nil || connCount < bestConnCount {
            best = u
            bestConnCount = connCount
        }
    }
    return best
}
```

### 4.4 Health Checks

**Active Health Checks:**
```go
// From modules/caddyhttp/reverseproxy/healthchecks.go
func (h *HealthChecker) activeHealthChecker() {
    ticker := time.NewTicker(h.Interval)
    for {
        select {
        case <-ticker.C:
            for _, upstream := range h.Upstreams {
                // Send HTTP request to health endpoint
                resp, err := h.Client.Get(upstream.HealthURL)
                if err != nil || resp.StatusCode != 200 {
                    upstream.MarkUnhealthy()
                } else {
                    upstream.MarkHealthy()
                }
            }
        }
    }
}
```

**Passive Health Checks:**
```go
// Track failures during normal proxying
func (h *Handler) proxyToUpstream(upstream *Upstream) error {
    start := time.Now()
    resp, err := h.Transport.RoundTrip(req)
    duration := time.Since(start)

    // Record for passive health checking
    upstream.RecordRequest(err, duration)

    if err != nil {
        // Backend failed, may trigger unhealthy status
        upstream.failures.Add(1)
    }

    return err
}
```

### 4.5 Connection Pooling

```go
// From modules/caddyhttp/reverseproxy/httptransport.go
type HTTPTransport struct {
    DialTimeout     caddy.Duration
    KeepAlive       caddy.Duration
    MaxConnsPerHost int
    MaxIdleConns    int
    MaxIdleConnsPerHost int
    IdleConnTimeout caddy.Duration
}

func (t *HTTPTransport) RoundTrip(req *http.Request) (*http.Response, error) {
    // Go's http.Transport handles connection pooling:
    // - Keeps idle connections open
    - Reuses connections for same host
    // - Limits connections per host
    // - Closes idle connections after timeout
    return t.transport.RoundTrip(req)
}
```

---

## 5. Connection Handling Basics

### 5.1 TCP Listening

```go
// From listeners.go
type NetworkAddress struct {
    Network   string  // "tcp", "udp", "unix"
    Host      string  // hostname, IP, or socket path
    StartPort uint    // for port ranges
    EndPort   uint
}

func (na NetworkAddress) ListenAll(ctx context.Context) ([]any, error) {
    var listeners []any

    // Handle port ranges
    for portOffset := uint(0); portOffset < na.PortRangeSize(); portOffset++ {
        ln, err := na.Listen(ctx, portOffset, config)
        if err != nil {
            // Close already-opened listeners
            for _, l := range listeners {
                l.(io.Closer).Close()
            }
            return nil, err
        }
        listeners = append(listeners, ln)
    }

    return listeners, nil
}
```

### 5.2 SO_REUSEPORT (Graceful Reloads)

```go
// From listen_unix.go
func listenReusable(ctx context.Context, lnKey, network, address string, config net.ListenConfig) (net.Listener, error) {
    // Check if we already have this listener
    if ls, ok := listenersPool.Load(lnKey); ok {
        // Return existing listener (allows multiple servers on same socket)
        return ls.(net.Listener), nil
    }

    // For TCP on Unix, use SO_REUSEPORT
    if isUnixTCP(network) {
        listener, err := net.Listen(network, address)
        if err != nil {
            return nil, err
        }

        // Set SO_REUSEPORT
        setReusePort(listener)

        // Store in pool
        listenersPool.Store(lnKey, listener)
        return listener, nil
    }

    return config.Listen(ctx, network, address)
}
```

**Why SO_REUSEPORT matters:**
```
Old Caddy          New Caddy
    |                  |
    | (both listening  |
    |  on same port)   |
    |                  |
    v (draining)       v (accepting new)
Clients gradually migrate to new config
No downtime during reload!
```

### 5.3 Connection Lifecycle

```go
// From modules/caddyhttp/server.go
type Server struct {
    ReadTimeout         caddy.Duration
    ReadHeaderTimeout   caddy.Duration
    WriteTimeout        caddy.Duration
    IdleTimeout         caddy.Duration
    KeepAliveInterval   caddy.Duration
}

func (s *Server) serve(ln net.Listener) {
    httpServer := &http.Server{
        ReadTimeout:       time.Duration(s.ReadTimeout),
        ReadHeaderTimeout: time.Duration(s.ReadHeaderTimeout),
        WriteTimeout:      time.Duration(s.WriteTimeout),
        IdleTimeout:       time.Duration(s.IdleTimeout),
        Handler:           http.HandlerFunc(s.serveHTTP),
    }

    // Set keepalive
    if s.KeepAliveInterval > 0 {
        httpServer.SetKeepAlivesEnabled(true)
    }

    httpServer.Serve(ln)
}
```

### 5.4 Handling Upgrades (WebSocket, HTTP/2)

```go
// Handle WebSocket upgrade
func handleWebSocket(w http.ResponseWriter, r *http.Request) {
    // 1. Check Upgrade header
    if r.Header.Get("Upgrade") != "websocket" {
        http.Error(w, "Not a WebSocket request", 400)
        return
    }

    // 2. Hijack the connection (take control from http.Server)
    hijacker, ok := w.(http.Hijacker)
    if !ok {
        http.Error(w, "WebSocket not supported", 501)
        return
    }

    conn, buf, err := hijacker.Hijack()
    if err != nil {
        return
    }
    defer conn.Close()

    // 3. Send WebSocket handshake response
    conn.Write([]byte("HTTP/1.1 101 Switching Protocols\r\n" +
        "Upgrade: websocket\r\n" +
        "Connection: Upgrade\r\n" +
        "\r\n"))

    // 4. Now you have raw TCP control
    // Handle WebSocket frames directly
}
```

---

## 6. From Zero to Caddy

### 6.1 Building a Minimal Caddy-like Server

Let's combine everything we've learned:

```go
package main

import (
    "context"
    "crypto/tls"
    "fmt"
    "log"
    "net"
    "net/http"
    "net/http/httputil"
    "net/url"
    "sync"
    "time"
)

// MinimalConfig represents server configuration
type MinimalConfig struct {
    Address     string
    TLSDomains  []string
    ProxyTarget string
}

// MinimalServer is our Caddy-like server
type MinimalServer struct {
    config    MinimalConfig
    listener  net.Listener
    mu        sync.Mutex
    routes    []Route
}

// Route matches requests to handlers
type Route struct {
    Host    string
    Path    string
    Handler http.HandlerFunc
}

// NewMinimalServer creates a server with automatic HTTPS
func NewMinimalServer(config MinimalConfig) (*MinimalServer, error) {
    s := &MinimalServer{config: config}

    // In real Caddy, this would use certmagic for ACME
    var listener net.Listener
    var err error

    if len(config.TLSDomains) > 0 {
        // TLS listener (would use ACME in production)
        cert, err := tls.LoadX509KeyPair("cert.pem", "key.pem")
        if err != nil {
            return nil, fmt.Errorf("loading TLS cert: %v", err)
        }

        tlsConfig := &tls.Config{
            Certificates: []tls.Certificate{cert},
        }

        listener, err = tls.Listen("tcp", config.Address, tlsConfig)
    } else {
        listener, err = net.Listen("tcp", config.Address)
    }

    if err != nil {
        return nil, err
    }

    s.listener = listener
    return s, nil
}

// AddRoute adds a route to the server
func (s *MinimalServer) AddRoute(host, path string, handler http.HandlerFunc) {
    s.mu.Lock()
    defer s.mu.Unlock()
    s.routes = append(s.routes, Route{Host: host, Path: path, Handler: handler})
}

// AddReverseProxy adds a reverse proxy route
func (s *MinimalServer) AddReverseProxy(host, path, target string) {
    targetURL, _ := url.Parse(target)
    proxy := httputil.NewSingleHostReverseProxy(targetURL)

    s.AddRoute(host, path, func(w http.ResponseWriter, r *http.Request) {
        // Add X-Forwarded headers
        r.Header.Set("X-Forwarded-For", r.RemoteAddr)
        r.Header.Set("X-Forwarded-Proto", "https")
        r.Header.Set("X-Forwarded-Host", host)

        proxy.ServeHTTP(w, r)
    })
}

// Serve starts the server
func (s *MinimalServer) Serve() error {
    mux := http.NewServeMux()
    mux.HandleFunc("/", s.handleRequest)

    server := &http.Server{
        Handler:           mux,
        ReadHeaderTimeout: 10 * time.Second,
        WriteTimeout:      60 * time.Second,
        IdleTimeout:       5 * time.Minute,
    }

    return server.Serve(s.listener)
}

// handleRequest routes requests to handlers
func (s *MinimalServer) handleRequest(w http.ResponseWriter, r *http.Request) {
    s.mu.Lock()
    defer s.mu.Unlock()

    for _, route := range s.routes {
        if route.Host != "" && r.Host != route.Host {
            continue
        }
        if route.Path != "" && r.URL.Path != route.Path {
            continue
        }
        route.Handler(w, r)
        return
    }

    // Default: 404
    http.NotFound(w, r)
}

// GracefulReload demonstrates zero-downtime reload
func (s *MinimalServer) GracefulReload(newConfig MinimalConfig) error {
    // 1. Create new server with new config
    newServer, err := NewMinimalServer(newConfig)
    if err != nil {
        return err
    }

    // 2. Start new server in background
    go newServer.Serve()

    // 3. Shutdown old server gracefully
    ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
    defer cancel()

    return s.listener.Close() // Old connections drain
}

func main() {
    config := MinimalConfig{
        Address:    ":443",
        TLSDomains: []string{"example.com"},
    }

    server, err := NewMinimalServer(config)
    if err != nil {
        log.Fatal(err)
    }

    // Add static file server
    server.AddRoute("example.com", "/", func(w http.ResponseWriter, r *http.Request) {
        http.ServeFile(w, r, "/var/www/html"+r.URL.Path)
    })

    // Add reverse proxy
    server.AddReverseProxy("api.example.com", "/v1/", "http://localhost:8080")

    log.Println("Starting server...")
    log.Fatal(server.Serve())
}
```

### 6.2 What Makes Caddy Different

Our minimal server has the basics, but Caddy adds:

| Feature | Minimal Server | Caddy |
|---------|---------------|-------|
| TLS | Manual cert loading | Automatic ACME |
| Config | Hardcoded | Caddyfile, JSON, Admin API |
| Modules | None | Full plugin system |
| Reloads | Manual | Graceful, atomic |
| Metrics | None | Prometheus, structured logs |
| HTTP/3 | No | Yes (quic-go) |
| OCSP | No | Automatic stapling |

### 6.3 Next Steps

Now that you understand the fundamentals:

1. **[Module System Deep Dive](01-module-system-deep-dive.md)** - How Caddy's plugin architecture works
2. **[TLS Automation Deep Dive](02-tls-automation-deep-dive.md)** - ACME protocol in detail
3. **[Reverse Proxy Deep Dive](03-reverse-proxy-deep-dive.md)** - Load balancing, health checks
4. **[Rust Revision](rust-revision.md)** - How to replicate in Rust with valtron

---

## Summary

### Key Takeaways

1. **HTTP is simple**: Request/response over TCP with text headers
2. **TLS is essential**: Encryption, authentication, integrity
3. **ACME automates TLS**: Let's Encrypt + ACME = free, automatic certs
4. **Reverse proxies are powerful**: Load balancing, SSL termination, caching
5. **Graceful reloads matter**: SO_REUSEPORT enables zero-downtime updates

### From This Document

You now understand:
- How web servers listen and handle connections
- HTTP request/response structure
- TLS handshake and certificate chains
- ACME protocol for automatic certificates
- Reverse proxy patterns and load balancing
- Connection handling and graceful reloads

### What's Next

The deep dives build on these fundamentals:
- Module system shows how Caddy is extensible
- TLS automation shows the ACME implementation
- Reverse proxy shows production load balancing
- Rust revision shows how to replicate in Rust

---

*This document is part of the complete Caddy exploration. Continue with the deep dives for comprehensive understanding.*
