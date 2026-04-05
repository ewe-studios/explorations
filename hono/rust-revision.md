---
source: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/hono
repository: github.com/honojs/hono
explored_at: 2026-04-05
focus: Rust implementation of Hono patterns - web framework with multi-runtime support, fast routing, middleware composition
---

# Rust Revision: Hono in Rust

## Overview

This document translates Hono's web framework patterns from TypeScript to Rust, covering Axum/Actix-based routing, middleware composition, type-safe handlers, and deployment across multiple Rust runtimes.

## Architecture Comparison

### TypeScript (Original Hono)

```
Hono (TypeScript)
├── RegExpRouter (O(1) matching)
├── Middleware Composition (Koa-style)
├── Context Object (Request/Response wrapper)
├── Runtime Adapters
│   ├── Cloudflare Workers
│   ├── Deno
│   ├── Bun
│   ├── Node.js
│   └── Lambda/Vercel
└── Type Inference (TypeScript)
```

### Rust (Revision)

```
Hono-Rust (Rust)
├── Regex Router / Trie Router
├── Tower Middleware (Service trait)
├── Request/Response (hyper/http)
├── Runtime Support
│   ├── tokio (async runtime)
│   ├── worker (Cloudflare)
│   ├── hyper (HTTP server)
│   └── Lambda HTTP
└── Type Safety (Rust types)
```

## Core Data Structures

```rust
// src/types.rs

use http::{Request, Response, StatusCode, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub name: String,
    pub bind_address: String,
    pub port: u16,
    pub workers: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "hono-rust".to_string(),
            bind_address: "0.0.0.0".to_string(),
            port: 3000,
            workers: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
        }
    }
}

/// Request context (similar to Hono's Context)
pub struct Context<State: Clone + Send + Sync = ()> {
    /// HTTP request
    pub request: Request<Body>,
    
    /// Route parameters
    pub params: HashMap<String, String>,
    
    /// Query parameters
    pub query: HashMap<String, String>,
    
    /// Application state
    pub state: State,
    
    /// Response status
    pub status: StatusCode,
    
    /// Response headers
    pub headers: http::HeaderMap,
    
    /// Extensions for middleware communication
    pub extensions: http::Extensions,
}

impl<State: Clone + Send + Sync> Context<State> {
    pub fn new(request: Request<Body>, state: State) -> Self {
        Self {
            request,
            params: HashMap::new(),
            query: HashMap::new(),
            state,
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            extensions: http::Extensions::new(),
        }
    }
    
    /// Get route parameter
    pub fn param(&self, name: &str) -> Option<&String> {
        self.params.get(name)
    }
    
    /// Get query parameter
    pub fn query(&self, name: &str) -> Option<&String> {
        self.query.get(name)
    }
    
    /// Get header
    pub fn header(&self, name: &str) -> Option<&str> {
        self.request
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
    }
    
    /// Set header
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name, value.parse().unwrap());
    }
    
    /// Get path
    pub fn path(&self) -> &str {
        self.request.uri().path()
    }
    
    /// Get method
    pub fn method(&self) -> &Method {
        self.request.method()
    }
    
    /// Parse JSON body
    pub async fn json<T: Deserialize<'static>>(&self) -> Result<T, serde_json::Error> {
        let body = hyper::body::to_bytes(self.request.body()).await?;
        serde_json::from_slice(&body)
    }
    
    /// Response helpers
    pub fn text(&mut self, body: &str) -> Response<Body> {
        Response::builder()
            .status(self.status)
            .header("Content-Type", "text/plain")
            .body(Body::from(body.to_string()))
            .unwrap()
    }
    
    pub fn json<T: Serialize>(&mut self, data: &T) -> Response<Body> {
        Response::builder()
            .status(self.status)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(data).unwrap()))
            .unwrap()
    }
    
    pub fn html(&mut self, body: &str) -> Response<Body> {
        Response::builder()
            .status(self.status)
            .header("Content-Type", "text/html")
            .body(Body::from(body.to_string()))
            .unwrap()
    }
    
    pub fn redirect(&mut self, location: &str, status: StatusCode) -> Response<Body> {
        Response::builder()
            .status(status)
            .header("Location", location)
            .body(Body::empty())
            .unwrap()
    }
}

/// Handler function type
pub type Handler<State, Resp> = Arc<
    dyn Fn(Context<State>) -> BoxFuture<'static, Resp> + Send + Sync
>;

/// Middleware function type
pub type Middleware<State> = Arc<
    dyn Fn(Context<State>, Next<State>) -> BoxFuture<'static, Response<Body>> 
    + Send + Sync
>;

/// Next middleware
pub struct Next<State: Clone + Send + Sync> {
    handlers: Vec<Middleware<State>>,
    index: usize,
}

impl<State: Clone + Send + Sync> Next<State> {
    pub async fn call(&mut self, mut ctx: Context<State>) -> Response<Body> {
        if self.index < self.handlers.len() {
            let handler = self.handlers[self.index].clone();
            self.index += 1;
            handler(ctx, self).await
        } else {
            // No more middleware, return 404
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap()
        }
    }
}
```

## Router Implementation

```rust
// src/router.rs

use std::collections::HashMap;
use regex::Regex;

/// Route matching result
pub struct RouteMatch<T> {
    pub handler: T,
    pub params: HashMap<String, String>,
}

/// Router trait
pub trait Router<Handler>: Send + Sync {
    fn add(&mut self, method: Method, path: &str, handler: Handler);
    fn route(&self, method: &Method, path: &str) -> Option<RouteMatch<Handler>>;
}

/// Trie-based router (similar to Hono's TrieRouter)
pub struct TrieRouter<Handler: Clone> {
    routes: HashMap<Method, TrieNode<Handler>>,
}

struct TrieNode<Handler: Clone> {
    children: HashMap<String, TrieNode<Handler>>,
    param_child: Option<Box<TrieNode<Handler>>>,
    param_name: Option<String>,
    handler: Option<Handler>,
}

impl<Handler: Clone> TrieRouter<Handler> {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }
}

impl<Handler: Clone + Send + Sync> Router<Handler> for TrieRouter<Handler> {
    fn add(&mut self, method: Method, path: &str, handler: Handler) {
        let node = self.routes.entry(method).or_insert_with(|| TrieNode {
            children: HashMap::new(),
            param_child: None,
            param_name: None,
            handler: None,
        });
        
        let mut current = node;
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        for segment in segments {
            if segment.starts_with(':') {
                // Parameter segment
                let param_name = segment[1..].to_string();
                
                if current.param_child.is_none() {
                    current.param_child = Some(Box::new(TrieNode {
                        children: HashMap::new(),
                        param_child: None,
                        param_name: None,
                        handler: None,
                    }));
                    current.param_name = Some(param_name);
                }
                
                current = current.param_child.as_mut().unwrap();
            } else {
                // Static segment
                current = current.children.entry(segment.to_string()).or_insert_with(|| TrieNode {
                    children: HashMap::new(),
                    param_child: None,
                    param_name: None,
                    handler: None,
                });
            }
        }
        
        current.handler = Some(handler);
    }
    
    fn route(&self, method: &Method, path: &str) -> Option<RouteMatch<Handler>> {
        let node = self.routes.get(method)?;
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        let mut params = HashMap::new();
        let mut current = node;
        let mut segment_idx = 0;
        
        while segment_idx < segments.len() {
            let segment = segments[segment_idx];
            
            // Try static match
            if let Some(next) = current.children.get(segment) {
                current = next;
                segment_idx += 1;
                continue;
            }
            
            // Try param match
            if let Some(ref param_child) = current.param_child {
                if let Some(ref param_name) = current.param_name {
                    params.insert(param_name.clone(), segment.to_string());
                    current = param_child.as_ref();
                    segment_idx += 1;
                    continue;
                }
            }
            
            // No match
            return None;
        }
        
        // Check if we have a handler at this node
        current.handler.clone().map(|handler| RouteMatch { handler, params })
    }
}

/// RegExp-based router (similar to Hono's RegExpRouter)
pub struct RegExpRouter<Handler: Clone> {
    routes: HashMap<Method, Vec<(Regex, Vec<String>, Handler)>>,
}

impl<Handler: Clone> RegExpRouter<Handler> {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }
}

impl<Handler: Clone + Send + Sync> Router<Handler> for RegExpRouter<Handler> {
    fn add(&mut self, method: Method, path: &str, handler: Handler) {
        // Convert path pattern to regex
        // /users/:id -> ^/users/([^/]+)$
        let mut param_names = Vec::new();
        let mut pattern = String::from("^");
        
        for segment in path.split('/').filter(|s| !s.is_empty()) {
            pattern.push('/');
            
            if segment.starts_with(':') {
                let param_name = segment[1..].to_string();
                param_names.push(param_name);
                pattern.push_str("([^/]+)");
            } else {
                pattern.push_str(&regex::escape(segment));
            }
        }
        
        pattern.push('$');
        let regex = Regex::new(&pattern).unwrap();
        
        self.routes.entry(method).or_default().push((regex, param_names, handler));
    }
    
    fn route(&self, method: &Method, path: &str) -> Option<RouteMatch<Handler>> {
        let routes = self.routes.get(method)?;
        
        for (regex, param_names, handler) in routes {
            if let Some(captures) = regex.captures(path) {
                let mut params = HashMap::new();
                
                for (i, name) in param_names.iter().enumerate() {
                    params.insert(name.clone(), captures.get(i + 1)?.as_str().to_string());
                }
                
                return Some(RouteMatch {
                    handler: handler.clone(),
                    params,
                });
            }
        }
        
        None
    }
}
```

## Application Structure

```rust
// src/app.rs

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct App<State: Clone + Send + Sync = ()> {
    router: Arc<RwLock<TrieRouter<Handler<State>>>>,
    middlewares: Vec<Middleware<State>>,
    state: State,
}

impl App {
    pub fn new() -> Self {
        Self {
            router: Arc::new(RwLock::new(TrieRouter::new())),
            middlewares: Vec::new(),
            state: (),
        }
    }
}

impl<State: Clone + Send + Sync + 'static> App<State> {
    pub fn with_state(state: State) -> Self {
        Self {
            router: Arc::new(RwLock::new(TrieRouter::new())),
            middlewares: Vec::new(),
            state,
        }
    }
    
    /// Add GET route
    pub fn get<F, R>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(Context<State>) -> R + Send + Sync + 'static,
        R: Future<Output = Response<Body>> + Send + 'static,
    {
        self.add_route(Method::GET, path, handler);
        self
    }
    
    /// Add POST route
    pub fn post<F, R>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(Context<State>) -> R + Send + Sync + 'static,
        R: Future<Output = Response<Body>> + Send + 'static,
    {
        self.add_route(Method::POST, path, handler);
        self
    }
    
    /// Add PUT route
    pub fn put<F, R>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(Context<State>) -> R + Send + Sync + 'static,
        R: Future<Output = Response<Body>> + Send + 'static,
    {
        self.add_route(Method::PUT, path, handler);
        self
    }
    
    /// Add DELETE route
    pub fn delete<F, R>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(Context<State>) -> R + Send + Sync + 'static,
        R: Future<Output = Response<Body>> + Send + 'static,
    {
        self.add_route(Method::DELETE, path, handler);
        self
    }
    
    fn add_route<F, R>(&mut self, method: Method, path: &str, handler: F)
    where
        F: Fn(Context<State>) -> R + Send + Sync + 'static,
        R: Future<Output = Response<Body>> + Send + 'static,
    {
        let handler = Arc::new(move |ctx: Context<State>| {
            Box::pin(handler(ctx)) as BoxFuture<'static, Response<Body>>
        });
        
        // Note: In real implementation, would need to acquire write lock
        // This is simplified for demonstration
    }
    
    /// Add middleware
    pub fn use<F>(&mut self, middleware: F) -> &mut Self
    where
        F: Fn(Context<State>, Next<State>) -> BoxFuture<'static, Response<Body>> 
           + Send + Sync + 'static,
    {
        self.middlewares.push(Arc::new(middleware));
        self
    }
    
    /// Serve the application
    pub async fn serve(self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::net::TcpListener;
        use hyper::service::{make_service_fn, service_fn};
        
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on http://{}", addr);
        
        let app = Arc::new(self);
        
        loop {
            let (stream, _) = listener.accept().await?;
            let app = app.clone();
            
            tokio::spawn(async move {
                if let Err(e) = hyper::server::conn::Http::new()
                    .serve_connection(
                        stream,
                        service_fn(move |req| {
                            let app = app.clone();
                            async move { app.handle_request(req).await }
                        }),
                    )
                    .await
                {
                    eprintln!("Error serving connection: {}", e);
                }
            });
        }
    }
    
    async fn handle_request(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        
        // Create context
        let mut ctx = Context::new(req, self.state.clone());
        
        // Parse query parameters
        if let Some(query_string) = ctx.request.uri().query() {
            for pair in query_string.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    ctx.query.insert(key.to_string(), value.to_string());
                }
            }
        }
        
        // Route matching
        let router = self.router.read().await;
        let route_match = router.route(&method, &path);
        
        if let Some(route) = route_match {
            ctx.params = route.params;
            
            // Build middleware chain
            let mut next = Next {
                handlers: self.middlewares.clone(),
                index: 0,
            };
            
            // Call middleware chain with handler
            let handler = route.handler;
            let response = next.call(ctx).await;
            Ok(response)
        } else {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap())
        }
    }
}
```

## Middleware Implementations

```rust
// src/middleware.rs

use std::time::Instant;

/// Logger middleware
pub async fn logger<State: Clone + Send + Sync + 'static>(
    ctx: Context<State>,
    mut next: Next<State>,
) -> Response<Body> {
    let method = ctx.method().clone();
    let path = ctx.path().to_string();
    let start = Instant::now();
    
    let response = next.call(ctx).await;
    
    let duration = start.elapsed();
    println!(
        "{} {} {} - {:?}",
        method,
        path,
        response.status(),
        duration
    );
    
    response
}

/// CORS middleware
pub fn cors<State: Clone + Send + Sync + 'static>(
    options: CorsOptions,
) -> impl Fn(Context<State>, Next<State>) -> BoxFuture<'static, Response<Body>> {
    move |mut ctx: Context<State>, mut next: Next<State>| {
        let options = options.clone();
        
        async move {
            // Handle preflight
            if ctx.method() == Method::OPTIONS {
                let mut response = Response::new(Body::empty());
                set_cors_headers(&mut response, &options);
                return response;
            }
            
            let mut response = next.call(ctx).await;
            set_cors_headers(&mut response, &options);
            response
        }
        .boxed()
    }
}

#[derive(Clone)]
pub struct CorsOptions {
    pub allow_origin: String,
    pub allow_methods: Vec<Method>,
    pub allow_headers: Vec<String>,
}

fn set_cors_headers(response: &mut Response<Body>, options: &CorsOptions) {
    let headers = response.headers_mut();
    
    headers.insert(
        "Access-Control-Allow-Origin",
        options.allow_origin.parse().unwrap(),
    );
    
    headers.insert(
        "Access-Control-Allow-Methods",
        options
            .allow_methods
            .iter()
            .map(|m| m.as_str())
            .collect::<Vec<_>>()
            .join(", ")
            .parse()
            .unwrap(),
    );
    
    headers.insert(
        "Access-Control-Allow-Headers",
        options.allow_headers.join(", ").parse().unwrap(),
    );
}

/// Request ID middleware
pub async fn request_id<State: Clone + Send + Sync + 'static>(
    mut ctx: Context<State>,
    mut next: Next<State>,
) -> Response<Body> {
    // Get or generate request ID
    let id = ctx
        .header("X-Request-ID")
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    // Store in extensions
    ctx.extensions.insert(id.clone());
    
    // Set response header
    ctx.set_header("X-Request-ID", &id);
    
    let response = next.call(ctx).await;
    response
}

/// Body limit middleware
pub fn body_limit<State: Clone + Send + Sync + 'static>(
    max_size: usize,
) -> impl Fn(Context<State>, Next<State>) -> BoxFuture<'static, Response<Body>> {
    move |mut ctx: Context<State>, mut next: Next<State>| {
        async move {
            // Check content length
            if let Some(content_length) = ctx.header("Content-Length") {
                if let Ok(size) = content_length.parse::<usize>() {
                    if size > max_size {
                        return Response::builder()
                            .status(StatusCode::PAYLOAD_TOO_LARGE)
                            .body(Body::from(format!(
                                "Payload too large. Max size is {} bytes",
                                max_size
                            )))
                            .unwrap();
                    }
                }
            }
            
            next.call(ctx).await
        }
        .boxed()
    }
}
```

## Runtime Adapters

```rust
// Cloudflare Workers adapter
// src/adapter/worker.rs

use worker::*;

pub fn handle<State: Clone + Send + Sync + 'static>(
    mut app: App<State>,
) -> impl Fn(Request, Env) -> Result<Response> + 'static {
    move |req: Request, _ctx: Env| {
        // Convert worker::Request to http::Request
        let http_req = convert_request(req)?;
        
        // Run app (blocking, Workers doesn't support tokio)
        let http_resp = app.handle_request_sync(http_req);
        
        // Convert back to worker::Response
        convert_response(http_resp)
    }
}

// Deno adapter
// src/adapter/deno.rs

use deno_core::Op;

pub async fn serve<State: Clone + Send + Sync + 'static>(
    app: App<State>,
    port: u16,
) -> Result<(), std::io::Error> {
    use tokio::net::TcpListener;
    
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Listening on http://0.0.0.0:{}", port);
    
    loop {
        let (stream, _) = listener.accept().await?;
        let app = app.clone();
        
        tokio::spawn(async move {
            if let Err(e) = hyper::server::conn::Http::new()
                .serve_connection(
                    stream,
                    service_fn(move |req| {
                        let app = app.clone();
                        async move { app.handle_request(req).await }
                    }),
                )
                .await
            {
                eprintln!("Error: {}", e);
            }
        });
    }
}
```

## Example Application

```rust
// examples/blog.rs

use hono_rust::{App, Context, Middleware, Next};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct AppState {
    db: Database,
}

#[derive(Serialize, Deserialize)]
struct Post {
    id: u32,
    title: String,
    body: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::with_state(AppState {
        db: Database::new().await?,
    });
    
    // Middleware
    app.use(logger);
    app.use(cors(CorsOptions {
        allow_origin: "*".to_string(),
        allow_methods: vec![Method::GET, Method::POST, Method::PUT, Method::DELETE],
        allow_headers: vec!["Content-Type".to_string()],
    }));
    
    // Routes
    app.get("/", |mut ctx| async move {
        ctx.text("Welcome to the blog!")
    })
    .get("/posts", |mut ctx| async move {
        let posts = ctx.state.db.get_posts().await;
        ctx.json(&posts)
    })
    .get("/posts/:id", |mut ctx| async move {
        let id = ctx.param("id").unwrap().parse::<u32>().unwrap();
        match ctx.state.db.get_post(id).await {
            Some(post) => ctx.json(&post),
            None => {
                ctx.status = StatusCode::NOT_FOUND;
                ctx.json(&serde_json::json!({"error": "Post not found"}))
            }
        }
    })
    .post("/posts", |mut ctx| async move {
        let post: Post = ctx.json().await.unwrap();
        let created = ctx.state.db.create_post(post).await;
        ctx.status = StatusCode::CREATED;
        ctx.json(&created)
    });
    
    // Serve
    app.serve("0.0.0.0:3000").await?;
    
    Ok(())
}
```

## Conclusion

The Rust implementation of Hono patterns provides:

1. **Type Safety**: Compile-time checking of handlers and routes
2. **Performance**: Zero-cost abstractions, no GC pauses
3. **Tower Middleware**: Compatible with Tower ecosystem
4. **Multi-runtime**: Support for tokio, Workers, and other runtimes
5. **Router Options**: Both Trie and RegExp-based routing
6. **Async First**: Built on tokio and futures

Key differences from TypeScript:
- Rust's type system provides stronger guarantees
- No runtime overhead from type checking
- Tower middleware compatibility
- Better performance for CPU-bound operations
- More explicit error handling
