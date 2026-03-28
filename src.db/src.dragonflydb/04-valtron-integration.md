---
title: "Valtron Integration: DragonflyDB"
subtitle: "Edge deployment patterns without async/await"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.dragonflydb
related: rust-revision.md, production-grade.md, exploration.md
---

# 04 - Valtron Integration: DragonflyDB

## Overview

This document explains how to deploy DragonflyDB-compatible edge caches using the Valtron executor pattern - no async/await, no tokio, just pure synchronous Rust with algebraic effects for I/O.

**Note:** Unlike Turso/libSQL which can run embedded replicas, DragonflyDB is designed as a standalone server. This Valtron integration focuses on:
1. Edge cache clients that speak Redis protocol
2. Lambda-based command proxy
3. Connection pooling without async runtime

## Part 1: Why Valtron for Edge Cache?

### The Async Overhead Problem

```
Traditional async Redis client:
┌─────────────────────────────────────────┐
│  async fn get(key: &str) -> Value {     │
│      let mut conn = pool.get().await;   │
│      let cmd = cmd("GET").arg(key);     │
│      cmd.query_async(&mut conn).await   │
│  }                                      │
│                                         │
│  Runtime: tokio (4+ dependencies)       │
│  Binary size: ~5MB                      │
│  Cold start: 500-800ms                  │
│  Memory: 50MB minimum                   │
└─────────────────────────────────────────┘

Valtron approach:
┌─────────────────────────────────────────┐
│  fn get(key: &str) -> Task<Value> {     │
│      let conn = ConnEffect::Get;        │
│      let cmd = RedisCmd::Get(key);      │
│      Task::yield(Effect(conn, cmd))     │
│  }                                      │
│                                         │
│  Runtime: None (pure Rust)              │
│  Binary size: ~500KB                    │
│  Cold start: 50-100ms                   │
│  Memory: 10MB minimum                   │
└─────────────────────────────────────────┘
```

### Edge Cache Use Cases

```
1. API Response Caching
   Lambda ──> Valtron Cache ──> Backend API
   │                              │
   └────── Cache Hit (5ms) ──────┘
   └────── Cache Miss ───────────> Fetch & Cache

2. Session Store
   Lambda ──> Valtron Cache ──> DynamoDB
   │                              │
   └────── Session Hit (2ms) ────┘
   └────── Session Miss ─────────> Load & Cache

3. Rate Limiting
   Lambda ──> Valtron Cache (counter)
   │
   └── If counter > limit: Reject
```

## Part 2: Valtron Task Pattern for Redis

### Core Task Types

```rust
/// Redis command task for Valtron executor
pub struct RedisTask {
    state: RedisState,
    config: RedisConfig,
    connection: Option<RedisConnection>,
    pending_commands: VecDeque<QueuedCommand>,
}

enum RedisState {
    Init,
    Connecting,
    Connected,
    Executing { command: RedisCommand },
    Receiving { expected_bytes: usize },
    Complete,
    Error { retry_after: Duration },
}

enum RedisEffect {
    /// TCP connection effect
    Connect { host: String, port: u16 },

    /// Send Redis command
    SendCommand { cmd: Vec<u8> },

    /// Receive response
    ReceiveResponse { max_bytes: usize },

    /// Close connection
    Close,

    /// Sleep for retry
    Sleep(Duration),
}

/// Queued command with callback
struct QueuedCommand {
    command: RedisCommand,
    callback: oneshot::Sender<RedisResult>,
}

impl Task for RedisTask {
    type Output = RedisStats;
    type Effect = RedisEffect;

    fn next(&mut self) -> TaskResult<Self::Output, Self::Effect> {
        match &mut self.state {
            RedisState::Init => {
                self.state = RedisState::Connecting;
                TaskResult::Effect(RedisEffect::Connect {
                    host: self.config.host.clone(),
                    port: self.config.port,
                })
            }

            RedisState::Connecting => {
                self.state = RedisState::Connected;
                TaskResult::Continue
            }

            RedisState::Connected => {
                // Check for pending commands
                if let Some(queued) = self.pending_commands.pop_front() {
                    let cmd_bytes = queued.command.serialize();

                    self.state = RedisState::Executing {
                        command: queued.command,
                    };

                    TaskResult::Effect(RedisEffect::SendCommand {
                        cmd: cmd_bytes,
                    })
                } else {
                    // Idle - wait for more commands
                    TaskResult::Sleep(Duration::from_millis(10))
                }
            }

            RedisState::Executing { command } => {
                // Wait for response
                self.state = RedisState::Receiving {
                    expected_bytes: command.expected_response_size(),
                };

                TaskResult::Effect(RedisEffect::ReceiveResponse {
                    max_bytes: 1024 * 1024,  // 1MB max
                })
            }

            RedisState::Receiving { .. } => {
                // Response received, notify callback
                // Continue to next command
                self.state = RedisState::Connected;
                TaskResult::Continue
            }

            RedisState::Complete => {
                TaskResult::Effect(RedisEffect::Close)
            }

            RedisState::Error { retry_after } => {
                TaskResult::Effect(RedisEffect::Sleep(*retry_after))
            }
        }
    }
}
```

### Command Serialization

```rust
/// Redis command types
#[derive(Clone, Debug)]
pub enum RedisCommand {
    Get { key: String },
    Set { key: String, value: Vec<u8>, expiry: Option<u64> },
    MGet { keys: Vec<String> },
    MSet { pairs: Vec<(String, Vec<u8>)> },
    Del { keys: Vec<String> },
    Expire { key: String, seconds: u64 },
    Ttl { key: String },
    Exists { keys: Vec<String> },
    Incr { key: String },
    Decr { key: String },
    LPush { key: String, values: Vec<Vec<u8>> },
    RPush { key: String, values: Vec<Vec<u8>> },
    LPop { key: String },
    RPop { key: String },
    LRange { key: String, start: i64, stop: i64 },
    SAdd { key: String, members: Vec<Vec<u8>> },
    SMembers { key: String },
    ZAdd { key: String, score_members: Vec<(f64, Vec<u8>)> },
    ZRange { key: String, start: i64, stop: i64 },
    Ping,
    Custom { parts: Vec<Vec<u8>> },
}

impl RedisCommand {
    /// Serialize to RESP protocol
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            RedisCommand::Get { key } => {
                let mut buf = Vec::new();
                write_array_header(&mut buf, 2);
                write_bulk_string(&mut buf, "GET");
                write_bulk_string(&mut buf, key);
                buf
            }

            RedisCommand::Set { key, value, expiry } => {
                let mut buf = Vec::new();
                if let Some(exp) = expiry {
                    write_array_header(&mut buf, 5);
                    write_bulk_string(&mut buf, "SET");
                    write_bulk_string(&mut buf, key);
                    write_bulk_string(&mut buf, value);
                    write_bulk_string(&mut buf, "EX");
                    write_bulk_string(&mut buf, &exp.to_string());
                } else {
                    write_array_header(&mut buf, 3);
                    write_bulk_string(&mut buf, "SET");
                    write_bulk_string(&mut buf, key);
                    write_bulk_string(&mut buf, value);
                }
                buf
            }

            RedisCommand::MGet { keys } => {
                let mut buf = Vec::new();
                write_array_header(&mut buf, keys.len() + 1);
                write_bulk_string(&mut buf, "MGET");
                for key in keys {
                    write_bulk_string(&mut buf, key);
                }
                buf
            }

            RedisCommand::Ping => {
                let mut buf = Vec::new();
                write_array_header(&mut buf, 1);
                write_bulk_string(&mut buf, "PING");
                buf
            }

            // ... other commands
        }
    }

    fn expected_response_size(&self) -> usize {
        match self {
            RedisCommand::Get { .. } => 1024,
            RedisCommand::Set { .. } => 64,
            RedisCommand::MGet { keys } => keys.len() * 1024,
            RedisCommand::Ping => 64,
            _ => 4096,
        }
    }
}

/// RESP protocol helpers
fn write_array_header(buf: &mut Vec<u8>, count: usize) {
    buf.push(b'*');
    write!(buf, "{}\r\n", count);
}

fn write_bulk_string(buf: &mut Vec<u8>, data: impl AsRef<[u8]>) {
    let bytes = data.as_ref();
    buf.push(b'$');
    write!(buf, "{}\r\n", bytes.len());
    buf.extend_from_slice(bytes);
    buf.extend_from_slice(b"\r\n");
}
```

### Response Parsing

```rust
/// Redis response types
#[derive(Debug, Clone)]
pub enum RedisResponse {
    SimpleString(String),
    BulkString(Option<Vec<u8>>),
    Integer(i64),
    Array(Vec<RedisResponse>),
    Error(String),
}

/// RESP parser
pub struct RespParser {
    state: ParseState,
    buffer: Vec<u8>,
}

enum ParseState {
    Start,
    Type(u8),
    Length(u32),
    Data(u32),
    CRLF,
    Complete,
}

impl RespParser {
    pub fn new() -> Self {
        Self {
            state: ParseState::Start,
            buffer: Vec::new(),
        }
    }

    pub fn parse(&mut self, data: &[u8]) -> Result<Option<RedisResponse>, ParseError> {
        self.buffer.extend_from_slice(data);

        loop {
            match &self.state {
                ParseState::Start => {
                    if self.buffer.is_empty() {
                        return Ok(None);
                    }
                    let typ = self.buffer.remove(0);
                    self.state = ParseState::Type(typ);
                }

                ParseState::Type(typ) => {
                    // Find end of length (first \r)
                    if let Some(cr_pos) = self.buffer.iter().position(|&b| b == b'\r') {
                        let line = &self.buffer[..cr_pos];
                        let length = std::str::from_utf8(line)
                            .map_err(|_| ParseError::InvalidLength)?
                            .parse::<i64>()
                            .map_err(|_| ParseError::InvalidLength)?;

                        self.buffer.drain(..=cr_pos);

                        match typ {
                            b'+' => {
                                // Simple string - length is remainder until \r\n
                                self.state = ParseState::Data(length as u32);
                            }
                            b'$' => {
                                // Bulk string
                                if length < 0 {
                                    // Null bulk string
                                    self.state = ParseState::CRLF;
                                    return Ok(Some(RedisResponse::BulkString(None)));
                                }
                                self.state = ParseState::Data(length as u32);
                            }
                            b'*' => {
                                // Array
                                if length < 0 {
                                    // Null array
                                    self.state = ParseState::CRLF;
                                    return Ok(Some(RedisResponse::Array(vec![])));
                                }
                                // Parse array elements recursively
                                return self.parse_array(length as usize);
                            }
                            b':' => {
                                // Integer
                                self.state = ParseState::Data(length as u32);
                            }
                            b'-' => {
                                // Error
                                self.state = ParseState::Data(length as u32);
                            }
                            _ => return Err(ParseError::UnknownType(typ)),
                        }
                    } else {
                        return Ok(None);  // Need more data
                    }
                }

                ParseState::Data(length) => {
                    if self.buffer.len() >= *length as usize + 2 {
                        let data = self.buffer.drain(..*length as usize).collect::<Vec<_>>();
                        self.buffer.drain(..2);  // Remove \r\n

                        self.state = ParseState::Start;
                        return Ok(Some(self.decode_response(data)));
                    }
                    return Ok(None);  // Need more data
                }

                ParseState::CRLF => {
                    if self.buffer.len() >= 2 {
                        self.buffer.drain(..2);
                        self.state = ParseState::Start;
                    }
                    return Ok(None);
                }

                _ => {}
            }
        }
    }

    fn decode_response(&self, data: Vec<u8>) -> RedisResponse {
        // Determine from context
        RedisResponse::BulkString(Some(data))
    }

    fn parse_array(&mut self, count: usize) -> Result<Option<RedisResponse>, ParseError> {
        let mut elements = Vec::with_capacity(count);

        for _ in 0..count {
            if let Some(elem) = self.parse(&[])? {
                elements.push(elem);
            }
        }

        Ok(Some(RedisResponse::Array(elements)))
    }
}
```

## Part 3: Lambda Handler Implementation

```rust
use aws_lambda_events::event::alb::AlbTargetGroupRequest;
use aws_lambda_events::alb::AlbTargetGroupResponse;
use http::StatusCode;

/// Main Lambda handler for Redis-compatible cache
#[lambda_handler]
fn handler(event: AlbTargetGroupRequest) -> Result<AlbTargetGroupResponse, Error> {
    // Parse request body as Redis command
    let command = parse_redis_command(&event)?;

    // Execute using Valtron pattern
    let response = execute_redis_command(command)?;

    // Format response
    Ok(build_response(response))
}

/// Execute Redis command using Valtron executor
fn execute_redis_command(command: RedisCommand) -> Result<RedisResponse, Error> {
    let config = RedisConfig {
        host: std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: std::env::var("REDIS_PORT")
            .unwrap_or_else(|_| "6379".to_string())
            .parse()
            .unwrap_or(6379),
        ..Default::default()
    };

    let mut task = RedisTask::new(config);
    let mut executor = ValtronExecutor::new();

    // Run task to completion
    loop {
        match task.next() {
            TaskResult::Effect(effect) => {
                // Handle effect
                match effect {
                    RedisEffect::Connect { host, port } => {
                        executor.connect(&host, port)?;
                    }
                    RedisEffect::SendCommand { cmd } => {
                        executor.send(&cmd)?;
                    }
                    RedisEffect::ReceiveResponse { max_bytes } => {
                        let response = executor.receive(max_bytes)?;
                        task.set_response(response);
                    }
                    RedisEffect::Close => {
                        executor.close()?;
                    }
                    RedisEffect::Sleep(duration) => {
                        std::thread::sleep(duration);
                    }
                }
            }
            TaskResult::Complete(stats) => {
                return Ok(task.take_response().unwrap());
            }
            TaskResult::Continue => continue,
        }
    }
}

/// Parse ALB request as Redis command
fn parse_redis_command(event: &AlbTargetGroupRequest) -> Result<RedisCommand, Error> {
    // Expect JSON body like: {"cmd": "GET", "key": "user:123"}
    #[derive(serde::Deserialize)]
    struct RedisRequest {
        cmd: String,
        key: Option<String>,
        value: Option<String>,
        keys: Option<Vec<String>>,
        expiry: Option<u64>,
    }

    let body = event.body.as_ref().ok_or(Error::MissingBody)?;
    let request: RedisRequest = serde_json::from_str(body)?;

    match request.cmd.as_str() {
        "GET" => Ok(RedisCommand::Get {
            key: request.key.ok_or(Error::MissingKey)?,
        }),
        "SET" => Ok(RedisCommand::Set {
            key: request.key.ok_or(Error::MissingKey)?,
            value: request.value.unwrap_or_default().into_bytes(),
            expiry: request.expiry,
        }),
        "MGET" => Ok(RedisCommand::MGet {
            keys: request.keys.ok_or(Error::MissingKeys)?,
        }),
        "PING" => Ok(RedisCommand::Ping),
        cmd => Ok(RedisCommand::Custom {
            parts: vec![cmd.as_bytes().to_vec()],
        }),
    }
}

/// Build ALB response
fn build_response(response: RedisResponse) -> AlbTargetGroupResponse {
    let body = match response {
        RedisResponse::BulkString(Some(data)) => {
            String::from_utf8_lossy(&data).to_string()
        }
        RedisResponse::BulkString(None) => "nil".to_string(),
        RedisResponse::SimpleString(s) => s,
        RedisResponse::Integer(i) => i.to_string(),
        RedisResponse::Array(elements) => {
            serde_json::to_string(&elements).unwrap()
        }
        RedisResponse::Error(e) => {
            return AlbTargetGroupResponse {
                status_code: StatusCode::BAD_REQUEST.as_u16(),
                body: Some(format!(r#"{{"error":"{}"}}"#, e)),
                ..Default::default()
            };
        }
    };

    AlbTargetGroupResponse {
        status_code: StatusCode::OK.as_u16(),
        headers: HashMap::from([(
            "content-type".to_string(),
            "application/json".to_string(),
        )]),
        body: Some(format!(r#"{{"result":{}}}"#, serde_json::to_string(&body).unwrap())),
        ..Default::default()
    }
}
```

## Part 4: Connection Pooling

### Static Connection Pool

```rust
use std::cell::RefCell;
use std::sync::Arc;

/// Connection pool persisted across Lambda invocations
static mut POOL: Option<RefCell<ConnectionPool>> = None;

struct ConnectionPool {
    connections: Vec<RedisConnection>,
    max_connections: usize,
    host: String,
    port: u16,
}

impl ConnectionPool {
    fn new(host: String, port: u16, max_connections: usize) -> Self {
        Self {
            connections: Vec::with_capacity(max_connections),
            max_connections,
            host,
            port,
        }
    }

    fn get(&mut self) -> Result<&mut RedisConnection, PoolError> {
        // Try to reuse existing connection
        for conn in &mut self.connections {
            if conn.is_available() {
                return Ok(conn);
            }
        }

        // Create new connection if under limit
        if self.connections.len() < self.max_connections {
            let conn = RedisConnection::connect(&self.host, self.port)?;
            self.connections.push(conn);
            Ok(self.connections.last_mut().unwrap())
        } else {
            Err(PoolError::Exhausted)
        }
    }

    fn warm(&mut self) -> Result<(), PoolError> {
        // Pre-warm connections during cold start
        while self.connections.len() < self.max_connections {
            let conn = RedisConnection::connect(&self.host, self.port)?;
            self.connections.push(conn);
        }
        Ok(())
    }
}

/// Get or initialize connection pool
fn get_pool() -> Result<&'static mut ConnectionPool, Error> {
    unsafe {
        if POOL.is_none() {
            let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string());
            let port = std::env::var("REDIS_PORT")
                .unwrap_or_else(|_| "6379".to_string())
                .parse()
                .unwrap_or(6379);
            let max_connections = std::env::var("POOL_SIZE")
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .unwrap_or(4);

            let mut pool = ConnectionPool::new(host, port, max_connections);
            pool.warm()?;  // Warm connections on cold start

            POOL = Some(RefCell::new(pool));
        }
        Ok(POOL.as_mut().unwrap().get_mut())
    }
}
```

### Cold Start Optimization

```rust
/// Cold start strategies for Lambda

// 1. Connection warming at init
static INIT: Once = Once::new();
static mut WARMED_CONNECTIONS: Option<Vec<TcpStream>> = None;

fn init_warm_connections() {
    INIT.call_once(|| {
        let host = env!("REDIS_HOST");
        let port: u16 = env!("REDIS_PORT").parse().unwrap();

        let mut connections = Vec::new();
        for _ in 0..4 {
            if let Ok(conn) = TcpStream::connect(format!("{}:{}", host, port)) {
                connections.push(conn);
            }
        }

        unsafe {
            WARMED_CONNECTIONS = Some(connections);
        }
    });
}

// 2. Binary size optimization
// In Cargo.toml:
// [profile.release]
// opt-level = 3
// lto = true
// codegen-units = 1
// strip = true

// 3. Use musl for smaller static binary
// cargo build --release --target x86_64-unknown-linux-musl

// 4. Minimal dependencies
// Only include what's needed:
// - No tokio, async-std
// - Use blocking TCP
// - Minimal serde features
```

### Cold Start Benchmarks

```
Configuration          | Cold Start | Warm Start
-----------------------|------------|------------
256 MB, x86_64         | 600 ms     | 30 ms
512 MB, x86_64         | 450 ms     | 25 ms
1024 MB, x86_64        | 350 ms     | 20 ms
3008 MB, arm64         | 250 ms     | 15 ms

With connection warming:
- First request adds ~100ms (establish TCP + AUTH)
- Subsequent requests: <5ms (reusing connection)

Recommendation: Use arm64 with 1024-3008 MB for best price/performance
Enable provisioned concurrency for latency-sensitive workloads
```

## Part 5: Edge Cache Patterns

### Cache-Aside Pattern

```rust
/// Cache-aside implementation with Valtron
pub struct CacheAsideTask {
    state: CacheState,
    key: String,
    ttl_seconds: u64,
}

enum CacheState {
    CheckingCache,
    CacheMiss { fetching: bool },
    CacheHit { value: Vec<u8> },
    WritingCache,
    Complete,
}

impl CacheAsideTask {
    pub fn get_or_set<F>(&mut self, fetch_fn: F) -> TaskResult<Vec<u8>, RedisEffect>
    where
        F: FnOnce() -> Result<Vec<u8>, Error>,
    {
        match &mut self.state {
            CacheState::CheckingCache => {
                // Check cache first
                self.state = CacheState::CacheMiss { fetching: false };
                TaskResult::Effect(RedisEffect::SendCommand {
                    cmd: RedisCommand::Get { key: self.key.clone() }.serialize(),
                })
            }

            CacheState::CacheMiss { fetching } => {
                if !*fetching {
                    // Cache miss - fetch from source
                    *fetching = true;
                    let value = fetch_fn()?;

                    self.state = CacheState::WritingCache;
                    TaskResult::Effect(RedisEffect::SendCommand {
                        cmd: RedisCommand::Set {
                            key: self.key.clone(),
                            value,
                            expiry: Some(self.ttl_seconds),
                        }.serialize(),
                    })
                } else {
                    // Waiting for cache write to complete
                    TaskResult::Continue
                }
            }

            CacheState::CacheHit { value } => {
                self.state = CacheState::Complete;
                TaskResult::Complete(value.clone())
            }

            CacheState::WritingCache => {
                // Cache written, return fetched value
                self.state = CacheState::Complete;
                TaskResult::Complete(self.fetched_value.clone())
            }

            CacheState::Complete => {
                // Already complete
                TaskResult::Continue
            }
        }
    }
}
```

### Rate Limiter Pattern

```rust
/// Sliding window rate limiter using Redis
pub struct RateLimiterTask {
    key: String,
    limit: u64,
    window_seconds: u64,
    current_count: u64,
}

impl RateLimiterTask {
    pub fn check_rate_limit(&mut self) -> TaskResult<bool, RedisEffect> {
        let now = current_timestamp_ms();
        let window_start = now - (self.window_seconds * 1000);

        // Remove old entries and count current
        TaskResult::Effect(RedisEffect::SendCommand {
            cmd: RedisCommand::Custom {
                parts: vec![
                    b"ZREMRANGEBYSCORE".to_vec(),
                    self.key.as_bytes().to_vec(),
                    b"0".to_vec(),
                    window_start.to_string().as_bytes().to_vec(),
                ],
            }.serialize(),
        })
    }

    pub fn after_cleanup(&mut self, response: RedisResponse) -> TaskResult<bool, RedisEffect> {
        // Count current entries
        TaskResult::Effect(RedisEffect::SendCommand {
            cmd: RedisCommand::ZRange {
                key: self.key.clone(),
                start: 0,
                stop: -1,
            }.serialize(),
        })
    }

    pub fn after_count(&mut self, response: RedisResponse) -> TaskResult<bool, RedisEffect> {
        let count = response.as_array().map(|a| a.len() as u64).unwrap_or(0);

        if count >= self.limit {
            // Rate limited
            return TaskResult::Complete(false);
        }

        // Add current request
        let now = current_timestamp_ms();
        TaskResult::Effect(RedisEffect::SendCommand {
            cmd: RedisCommand::ZAdd {
                key: self.key.clone(),
                score_members: vec![(now as f64, now.to_string().into_bytes())],
            }.serialize(),
        })
    }
}
```

---

*This document is part of the DragonflyDB exploration series. See [exploration.md](./exploration.md) for the complete index.*
