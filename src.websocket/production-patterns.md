# Production WebSocket Patterns

## Connection Management

### Connection Pooling

For high-concurrency scenarios, maintain a pool of connections:

```rust
use tokio::sync::mpsc;
use std::collections::HashMap;
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;
use std::sync::Arc;

type ConnectionId = u64;
type WSStream = WebSocketStream<TcpStream>;

struct ConnectionPool {
    connections: HashMap<ConnectionId, mpsc::Sender<Message>>,
    next_id: ConnectionId,
}

impl ConnectionPool {
    fn new() -> Self {
        Self {
            connections: HashMap::new(),
            next_id: 0,
        }
    }

    async fn add_connection(
        &mut self,
        ws: WSStream,
    ) -> (ConnectionId, mpsc::Receiver<Message>) {
        let id = self.next_id;
        self.next_id += 1;

        let (tx, rx) = mpsc::channel(100);
        self.connections.insert(id, tx.clone());

        // Spawn connection handler
        tokio::spawn(handle_connection(id, ws, tx));

        (id, rx)
    }

    fn remove_connection(&mut self, id: ConnectionId) {
        self.connections.remove(&id);
    }

    fn broadcast(&self, msg: Message) {
        for tx in self.connections.values() {
            let _ = tx.try_send(msg.clone());
        }
    }
}

async fn handle_connection(
    id: ConnectionId,
    mut ws: WSStream,
    tx: mpsc::Sender<Message>,
) {
    use futures_util::{SinkExt, StreamExt};

    // Receive task
    let mut rx = tx.subscribe();
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Read task
    while let Some(Ok(msg)) = ws.next().await {
        // Process message
        match msg {
            Message::Close(_) => break,
            Message::Ping(data) => {
                let _ = ws.send(Message::Pong(data)).await;
            }
            _ => {}
        }
    }

    recv_task.abort();
}
```

### Connection Lifecycle

```rust
use std::time::{Duration, Instant};

struct Connection {
    id: u64,
    ws: WSStream,
    created_at: Instant,
    last_activity: Instant,
    message_count: u64,
    state: ConnectionState,
}

enum ConnectionState {
    Connecting,
    Active,
    Closing,
    Closed,
}

impl Connection {
    fn is_idle(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }

    fn record_activity(&mut self) {
        self.last_activity = Instant::now();
        self.message_count += 1;
    }

    fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

// Connection manager with lifecycle
struct ConnectionManager {
    connections: HashMap<u64, Connection>,
    idle_timeout: Duration,
    max_age: Duration,
}

impl ConnectionManager {
    async fn prune_idle(&mut self) {
        let now = Instant::now();
        let to_remove: Vec<_> = self.connections
            .iter()
            .filter(|(_, c)| c.is_idle(self.idle_timeout) || c.age() > self.max_age)
            .map(|(id, _)| *id)
            .collect();

        for id in to_remove {
            if let Some(mut conn) = self.connections.remove(&id) {
                let _ = conn.ws.close(Some(CloseFrame {
                    code: CloseCode::Away,
                    reason: "Connection idle".into(),
                })).await;
            }
        }
    }
}
```

---

## Heartbeat / Keepalive

### Ping-Pong Implementation

```rust
use tokio::time::{interval, Duration, Interval};

struct HeartbeatConfig {
    ping_interval: Duration,
    pong_timeout: Duration,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            ping_interval: Duration::from_secs(30),
            pong_timeout: Duration::from_secs(60),
        }
    }
}

struct HeartbeatState {
    pending_pings: usize,
    last_pong: Instant,
    config: HeartbeatConfig,
}

async fn with_heartbeat(
    mut ws: WSStream,
    config: HeartbeatConfig,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    use futures_util::{SinkExt, StreamExt};
    use tokio::select;

    let mut ping_interval = interval(config.ping_interval);
    let mut state = HeartbeatState {
        pending_pings: 0,
        last_pong: Instant::now(),
        config,
    };

    loop {
        select! {
            // Send ping
            _ = ping_interval.tick() => {
                if state.pending_pings >= 2 {
                    // Too many unacknowledged pings
                    eprintln!("Heartbeat timeout");
                    break;
                }
                state.pending_pings += 1;
                ws.send(Message::Ping(Vec::new())).await?;
            }

            // Receive message
            msg = ws.next() => {
                match msg {
                    Some(Ok(Message::Pong(_))) => {
                        state.pending_pings = 0;
                        state.last_pong = Instant::now();
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // Auto-respond to pings
                        ws.send(Message::Pong(data)).await?;
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        eprintln!("Receive error: {}", e);
                        break;
                    }
                    None => break, // Stream closed
                }
            }

            // Check timeout
            _ = tokio::time::sleep_until(state.last_pong + state.config.pong_timeout) => {
                if state.pending_pings > 0 {
                    eprintln!("Pong timeout");
                    break;
                }
            }
        }
    }

    Ok(())
}
```

### Automatic Ping with tokio-tungstenite

```rust
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

let config = WebSocketConfig {
    // Note: tokio-tungstenite doesn't have built-in ping/pong
    // Implement manually as shown above
    ..Default::default()
};

let (ws, _) = connect_async_with_config(url, Some(config)).await?;
```

---

## Reconnection Strategies

### Exponential Backoff

```rust
use tokio::time::{sleep, Duration};
use rand::Rng;

struct ReconnectConfig {
    initial_delay: Duration,
    max_delay: Duration,
    multiplier: f32,
    jitter: f32,
    max_retries: u32,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            jitter: 0.1,
            max_retries: u32::MAX,
        }
    }
}

async fn connect_with_reconnect<F, Fut>(
    url: &str,
    mut connect_fn: F,
    config: ReconnectConfig,
) -> Result<WSStream, Box<dyn std::error::Error>>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<WSStream, tokio_tungstenite::tungstenite::Error>>,
{
    let mut delay = config.initial_delay;
    let mut attempts = 0;

    loop {
        match connect_fn().await {
            Ok(ws) => {
                log::info!("Connected after {} attempts", attempts);
                return Ok(ws);
            }
            Err(e) => {
                attempts += 1;
                if attempts >= config.max_retries {
                    log::error!("Max reconnection attempts reached");
                    return Err(Box::new(e));
                }

                // Add jitter
                let jitter = Duration::from_millis(
                    (rand::thread_rng().gen::<f32>() * config.jitter * delay.as_millis() as f32) as u64
                );

                log::warn!(
                    "Connection failed (attempt {}), retrying in {:?}",
                    attempts,
                    delay + jitter
                );

                sleep(delay + jitter).await;

                // Exponential backoff
                delay = std::cmp::min(
                    Duration::from_millis((delay.as_millis() as f32 * config.multiplier) as u64),
                    config.max_delay,
                );
            }
        }
    }
}
```

### Resilient Client Wrapper

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

struct ResilientClient {
    url: String,
    config: ReconnectConfig,
    ws: Arc<RwLock<Option<WSStream>>>,
    running: Arc<tokio::sync::Notify>,
}

impl ResilientClient {
    fn new(url: String) -> Self {
        Self {
            url,
            config: ReconnectConfig::default(),
            ws: Arc::new(RwLock::new(None)),
            running: Arc::new(tokio::sync::Notify::new()),
        }
    }

    async fn start(&self) {
        let ws = self.ws.clone();
        let url = self.url.clone();
        let config = self.config.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            loop {
                let result = connect_with_reconnect(&url, || {
                    connect_async(&url)
                }, config.clone()).await;

                match result {
                    Ok(new_ws) => {
                        *ws.write().await = Some(new_ws);

                        // Wait until connection is lost
                        loop {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            if ws.read().await.is_none() {
                                break;
                            }
                        }
                    }
                    Err(_) => {
                        // Max retries exceeded, wait before giving another try
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    }
                }
            }
        });
    }

    async fn send(&self, msg: Message) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        if let Some(ws) = self.ws.write().await.as_mut() {
            ws.send(msg).await
        } else {
            Err(tokio_tungstenite::tungstenite::Error::AlreadyClosed)
        }
    }
}
```

---

## Backpressure Handling

### Write Buffer Management

```rust
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

struct BackpressureConfig {
    channel_buffer_size: usize,
    max_write_buffer_size: usize,
    write_buffer_size: usize,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            channel_buffer_size: 256,
            max_write_buffer_size: 4 * 1024 * 1024, // 4MB
            write_buffer_size: 128 * 1024,          // 128KB
        }
    }
}

async fn handle_with_backpressure(
    ws: WSStream,
    config: BackpressureConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws_config = WebSocketConfig {
        max_write_buffer_size: config.max_write_buffer_size,
        write_buffer_size: config.write_buffer_size,
        ..Default::default()
    };

    // Reconfigure WebSocket
    // Note: This requires accessing the inner WebSocket

    // Use bounded channel for backpressure
    let (tx, mut rx) = mpsc::channel::<Message>(config.channel_buffer_size);

    // Writer task with backpressure
    let writer_task = tokio::spawn(async move {
        let mut ws = ws;
        while let Some(msg) = rx.recv().await {
            match ws.send(msg).await {
                Ok(()) => {}
                Err(tokio_tungstenite::tungstenite::Error::WriteBufferFull(m)) => {
                    // Buffer full, wait for flush
                    ws.flush().await?;
                    ws.send(m).await?;
                }
                Err(e) => return Err(e),
            }
        }
        Ok::<_, tokio_tungstenite::tungstenite::Error>(())
    });

    // Reader task
    let reader_task = tokio::spawn(async move {
        let mut ws = ws;
        while let Some(Ok(msg)) = ws.next().await {
            // Process message
            match msg {
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let _ = tokio::join!(writer_task, reader_task);
    Ok(())
}
```

### Rate Limiting

```rust
use tokio::time::{Duration, Instant};
use std::collections::VecDeque;

struct RateLimiter {
    max_messages_per_second: usize,
    timestamps: VecDeque<Instant>,
}

impl RateLimiter {
    fn new(max_rate: usize) -> Self {
        Self {
            max_messages_per_second: max_rate,
            timestamps: VecDeque::with_capacity(max_rate),
        }
    }

    async fn acquire(&mut self) {
        let now = Instant::now();
        let one_second_ago = now - Duration::from_secs(1);

        // Remove old timestamps
        while let Some(&ts) = self.timestamps.front() {
            if ts < one_second_ago {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        // Check if at limit
        if self.timestamps.len() >= self.max_messages_per_second {
            // Wait until oldest timestamp expires
            let wait_time = Duration::from_secs(1) - (now - self.timestamps.front().unwrap());
            tokio::time::sleep(wait_time).await;
            return self.acquire().await;
        }

        self.timestamps.push_back(now);
    }
}

// Usage
async fn rate_limited_send(
    mut ws: WSStream,
    mut rx: mpsc::Receiver<Message>,
    mut limiter: RateLimiter,
) {
    while let Some(msg) = rx.recv().await {
        limiter.acquire().await;
        if ws.send(msg).await.is_err() {
            break;
        }
    }
}
```

---

## Scaling and Load Balancing

### Horizontal Scaling with Redis Pub/Sub

```rust
use redis::{aio::{Connection, PubSub}, Client};

struct ScaledWebSocket {
    redis: Client,
    local_connections: Arc<DashMap<ConnectionId, mpsc::Sender<Message>>>,
}

impl ScaledWebSocket {
    async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let redis = Client::open(redis_url)?;
        Ok(Self {
            redis,
            local_connections: Arc::new(DashMap::new()),
        })
    }

    async fn subscribe_to_channel(
        &self,
        channel: &str,
    ) -> Result<(), redis::RedisError> {
        let mut pubsub = self.redis.get_async_pubsub().await?;
        pubsub.subscribe(channel).await?;

        let local_conns = self.local_connections.clone();
        let channel = channel.to_string();

        tokio::spawn(async move {
            while let Ok(Some(msg)) = pubsub.get_message().await {
                let payload: String = msg.get_payload().unwrap();
                let msg = Message::Text(payload);

                // Broadcast to local connections
                for entry in local_conns.iter() {
                    let _ = entry.value().send(msg.clone()).await;
                }
            }
        });

        Ok(())
    }

    async fn publish(&self, channel: &str, msg: &str) -> Result<(), redis::RedisError> {
        let pubsub = self.redis.get_async_pubsub().await?;
        pubsub.publish(channel, msg).await?;
        Ok(())
    }
}
```

### Load Balancer Integration

```rust
// For nginx/HAProxy WebSocket support:
//
// nginx configuration:
//
// upstream websocket_backend {
//     server backend1:8080;
//     server backend2:8080;
//     server backend3:8080;
//     keepalive 1000;
// }
//
// server {
//     location /ws {
//         proxy_pass http://websocket_backend;
//         proxy_http_version 1.1;
//         proxy_set_header Upgrade $http_upgrade;
//         proxy_set_header Connection "upgrade";
//         proxy_set_header Host $host;
//         proxy_set_header X-Real-IP $remote_addr;
//         proxy_read_timeout 86400;
//     }
// }

// HAProxy configuration:
//
// frontend ws_frontend
//     bind *:80
//     default_backend ws_backend
//     option http-server-close
//
// backend ws_backend
//     balance roundrobin
//     server ws1 backend1:8080 check
//     server ws2 backend2:8080 check
//     server ws3 backend3:8080 check
//     option http-server-close
```

### Sticky Sessions

```rust
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

fn hash_connection_id(session_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    session_id.hash(&mut hasher);
    hasher.finish()
}

fn select_backend(session_id: &str, backends: &[String]) -> &str {
    let hash = hash_connection_id(session_id);
    let index = hash as usize % backends.len();
    &backends[index]
}

// Client-side sticky session
async fn connect_with_sticky_session(
    base_url: &str,
    session_id: &str,
    backends: &[String],
) -> Result<WSStream, tokio_tungstenite::tungstenite::Error> {
    let backend = select_backend(session_id, backends);
    let url = format!("ws://{}/ws?session={}", backend, session_id);
    connect_async(&url).await
}
```

---

## Monitoring and Observability

### Prometheus Metrics

```rust
use prometheus::{IntCounter, IntGauge, Histogram, Registry};

struct WebSocketMetrics {
    connections_active: IntGauge,
    connections_total: IntCounter,
    messages_sent: IntCounter,
    messages_received: IntCounter,
    message_size: Histogram,
    latency: Histogram,
}

impl WebSocketMetrics {
    fn register(registry: &Registry) -> Result<Self, prometheus::Error> {
        let connections_active = IntGauge::new(
            "websocket_connections_active",
            "Number of active WebSocket connections"
        )?;
        let connections_total = IntCounter::new(
            "websocket_connections_total",
            "Total number of WebSocket connections"
        )?;
        let messages_sent = IntCounter::new(
            "websocket_messages_sent_total",
            "Total messages sent"
        )?;
        let messages_received = IntCounter::new(
            "websocket_messages_received_total",
            "Total messages received"
        )?;
        let message_size = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "websocket_message_size_bytes",
                "WebSocket message sizes"
            )
        )?;
        let latency = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "websocket_latency_seconds",
                "WebSocket operation latency"
            )
        )?;

        registry.register(Box::new(connections_active.clone()))?;
        registry.register(Box::new(connections_total.clone()))?;
        registry.register(Box::new(messages_sent.clone()))?;
        registry.register(Box::new(messages_received.clone()))?;
        registry.register(Box::new(message_size.clone()))?;
        registry.register(Box::new(latency.clone()))?;

        Ok(Self {
            connections_active,
            connections_total,
            messages_sent,
            messages_received,
            message_size,
            latency,
        })
    }

    fn on_connect(&self) {
        self.connections_active.inc();
        self.connections_total.inc();
    }

    fn on_disconnect(&self) {
        self.connections_active.dec();
    }

    fn on_message_sent(&self, size: usize) {
        self.messages_sent.inc();
        self.message_size.observe(size as f64);
    }

    fn on_message_received(&self, size: usize) {
        self.messages_received.inc();
        self.message_size.observe(size as f64);
    }
}
```

### Tracing Integration

```rust
use tracing::{info, warn, error, instrument};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, Registry};

fn init_tracing(service_name: &str) {
    let formatting_layer = BunyanFormattingLayer::new(
        service_name.into(),
        std::io::stdout,
    );

    let subscriber = Registry::default()
        .with(JsonStorageLayer)
        .with(formatting_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default subscriber failed");
}

#[instrument(skip(ws), fields(connection_id = %uuid::Uuid::new_v4()))]
async fn handle_connection(ws: WSStream) {
    info!("New WebSocket connection");

    // Connection handling code
    // Spans automatically track connection_id
}
```

---

## Security Patterns

### Authentication Middleware

```rust
use jsonwebtoken::{decode, Validation, DecodingKey};
use http::Request;

#[derive(Debug, serde::Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

async fn authenticate_request(
    request: &Request<()>,
) -> Result<Claims, tokio_tungstenite::tungstenite::Error> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .ok_or(tokio_tungstenite::tungstenite::Error::AttackAttempt)?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| tokio_tungstenite::tungstenite::Error::AttackAttempt)?;

    if !auth_str.starts_with("Bearer ") {
        return Err(tokio_tungstenite::tungstenite::Error::AttackAttempt);
    }

    let token = &auth_str[7..];
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET not set");
    let key = DecodingKey::from_secret(secret.as_bytes());

    let token_data = decode::<Claims>(token, &key, &Validation::default())
        .map_err(|_| tokio_tungstenite::tungstenite::Error::AttackAttempt)?;

    Ok(token_data.claims)
}

async fn accept_authenticated(
    stream: TcpStream,
) -> Result<(WSStream, Claims), tokio_tungstenite::tungstenite::Error> {
    use tokio_tungstenite::accept_hdr_async;
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

    let callback = |req: &Request, response: Response| {
        async move {
            let claims = authenticate_request(req).await?;
            Ok::<_, tokio_tungstenite::tungstenite::Error>((response, claims))
        }
    };

    // Note: This is simplified - actual implementation needs
    // to handle the callback properly with async
    accept_async(stream).await.map(|ws| (ws, Claims { sub: "unknown".into(), exp: 0 }))
}
```

### Rate Limiting per Connection

```rust
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;

struct RateLimitedConnection {
    ws: WSStream,
    limiter: RateLimiter<NotKeyed, InMemoryState>,
}

impl RateLimitedConnection {
    fn new(ws: WSStream, messages_per_second: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(messages_per_second).unwrap());
        let limiter = RateLimiter::direct(quota);

        Self { ws, limiter }
    }

    async fn send(&mut self, msg: Message) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        self.limiter.until_ready().await;
        self.ws.send(msg).await
    }
}
```

---

## Testing Strategies

### Integration Test with Mock Server

```rust
#[cfg(test)]
mod tests {
    use tokio::net::TcpListener;
    use tokio_tungstenite::connect_async;

    #[tokio::test]
    async fn test_echo_server() {
        // Start test server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();

            while let Some(Ok(msg)) = ws.next().await {
                if msg.is_text() || msg.is_binary() {
                    ws.send(msg).await.unwrap();
                }
            }
        });

        // Connect client
        let (mut client, _) = connect_async(format!("ws://{}", addr))
            .await
            .unwrap();

        // Test echo
        client.send(Message::Text("test".into())).await.unwrap();
        let response = client.next().await.unwrap().unwrap();
        assert_eq!(response, Message::Text("test".into()));

        server_handle.abort();
    }
}
```

### Load Testing with Vegeta

```bash
# vegeta WebSocket attack
echo "GET ws://localhost:8080/socket" | \
    vegeta attack -rate=100 -duration=60s -connections=10 | \
    vegeta report
```

---

## Best Practices Summary

1. **Connection Management**
   - Pool connections for high concurrency
   - Implement graceful shutdown
   - Track connection lifecycle

2. **Reliability**
   - Implement heartbeat/keepalive
   - Use exponential backoff for reconnection
   - Handle all error cases

3. **Performance**
   - Configure appropriate buffer sizes
   - Implement backpressure
   - Rate limit if needed

4. **Scaling**
   - Use external pub/sub for multi-node
   - Implement sticky sessions if needed
   - Monitor connection counts

5. **Security**
   - Validate all input
   - Implement authentication
   - Use TLS for sensitive data

6. **Observability**
   - Add metrics (connections, messages, latency)
   - Implement distributed tracing
   - Log connection events
