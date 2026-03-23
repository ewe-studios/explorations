# Y-Sweet: CRDT-Based Real-Time Collaboration

Y-Sweet is a real-time collaboration server built on Yjs CRDTs. It provides persistent, synchronized state for collaborative applications.

## Overview

**Y-Sweet** is a server implementation for Yjs, the collaborative editing library. It provides:

- **CRDT Sync**: Yjs CRDT protocol for conflict-free merging
- **Persistence**: Store documents in S3, R2, or filesystem
- **Authentication**: Token-based access control
- **Cloudflare Workers**: Deploy to edge platforms
- **Standalone Server**: Run as a self-hosted service

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Y-Sweet Server                         │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                   HTTP Router                         │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐              │  │
│  │  │ /doc/:id│  │ /aware  │  │  /auth  │              │  │
│  │  └─────────┘  └─────────┘  └─────────┘              │  │
│  └───────────────────────────────────────────────────────┘  │
│                             │                                │
│  ┌──────────────────────────▼────────────────────────────┐  │
│  │                   DocWithSyncKv                       │  │
│  │  ┌─────────────────┐  ┌─────────────────────────┐    │  │
│  │  │   Awareness     │  │      SyncKv             │    │  │
│  │  │  ( cursors,     │  │  (document storage,     │    │  │
│  │  │   presence)     │  │   update log)           │    │  │
│  │  └─────────────────┘  └─────────────────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
│                             │                                │
│  ┌──────────────────────────▼────────────────────────────┐  │
│  │                     Store                             │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐              │  │
│  │  │   S3    │  │   R2    │  │   File  │              │  │
│  │  └─────────┘  └─────────┘  └─────────┘              │  │
│  └────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Core Concepts

### Yjs CRDT

Yjs uses Conflict-free Replicated Data Types (CRDTs) for collaboration:

- **Updates**: Incremental changes to documents
- **State Vector**: Compact representation of known state
- **Merging**: Automatic conflict resolution
- **Types**: Shared types (Array, Map, Text, Xml)

### Document Structure

```rust
pub struct DocWithSyncKv {
    awareness: Arc<RwLock<Awareness>>,
    sync_kv: Arc<SyncKv>,
    subscription: Subscription,  // RAII guard for update subscription
}
```

### Awareness

Awareness tracks ephemeral state like cursors and presence:

```rust
pub struct Awareness {
    pub doc: Doc,
    states: HashMap<ClientId, AwarenessState>,
}

pub struct AwarenessState {
    user: Option<Value>,
    clock: u32,
}
```

Awareness is NOT persisted - it's for temporary state.

### SyncKv

SyncKv manages document persistence:

```rust
pub struct SyncKv {
    store: Option<Arc<Box<dyn Store>>>,
    key: String,
    updates: Vec<Update>,
    dirty_callback: Arc<Box<dyn Fn() + Send + Sync>>,
}
```

## Storage Abstraction

### Store Trait

```rust
#[async_trait]
pub trait Store: Send + Sync {
    async fn get(&self, key: &str) -> Result<Vec<u8>>;
    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
}
```

### Implementations

#### S3 Store

```rust
pub struct S3Store {
    client: aws_sdk_s3::Client,
    bucket: String,
    prefix: String,
}

impl Store for S3Store {
    async fn get(&self, key: &str) -> Result<Vec<u8>> {
        let obj = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(format!("{}/{}", self.prefix, key))
            .send()
            .await?;
        Ok(obj.body.collect().await?.into_bytes().to_vec())
    }

    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(format!("{}/{}", self.prefix, key))
            .body(value.into())
            .send()
            .await?;
        Ok(())
    }
}
```

#### R2 Store (Cloudflare)

```rust
pub struct R2Store {
    bucket: cloudflare::Bucket,
}

// Uses Cloudflare Workers R2 binding
```

#### Filesystem Store

```rust
pub struct FilesystemStore {
    base_path: PathBuf,
}

impl Store for FilesystemStore {
    async fn get(&self, key: &str) -> Result<Vec<u8>> {
        let path = self.base_path.join(key);
        tokio::fs::read(path).await.map_err(Into::into)
    }

    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()> {
        let path = self.base_path.join(key);
        tokio::fs::write(path, value).await.map_err(Into::into)
    }
}
```

## Document Sync Protocol

### Update Encoding

Yjs updates are encoded using a binary protocol:

```rust
pub fn apply_update(&self, update: &[u8]) -> Result<()> {
    let update: Update = Update::decode_v1(update)?;
    let mut txn = self.doc.transact_mut();
    txn.apply_update(update);
    Ok(())
}
```

### State Exchange

```rust
pub fn as_update(&self) -> Vec<u8> {
    let doc = &self.awareness.read().unwrap().doc;
    let txn = doc.transact();
    txn.encode_state_as_update_v1(&StateVector::default())
}
```

### Update Subscription

```rust
let subscription = {
    let sync_kv = sync_kv.clone();
    doc.observe_update_v1(move |_, event| {
        sync_kv.push_update(DOC_NAME, &event.update).unwrap();
        sync_kv.flush_doc_with(DOC_NAME, Default::default()).unwrap();
    })?
};
```

## Server Implementation

### HTTP Endpoints

```rust
// Main server routes
app = app
    .route("/doc/:id", get(get_document))
    .route("/doc/:id", post(put_document))
    .route("/aware/:id", get(get_awareness))
    .route("/auth/:doc_id", post(authenticate));
```

### WebSocket Sync

```rust
async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(doc_id): Path<String>,
    State(state): State<Arc<ServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_sync(socket, doc_id, state))
}

async fn handle_sync(socket: WebSocket, doc_id: String, state: Arc<ServerState>) {
    let (mut send, mut recv) = socket.split();

    // Load or create document
    let doc = state.get_or_create(&doc_id).await;

    // Send initial state
    let update = doc.as_update();
    send.send(Message::Binary(update)).await.unwrap();

    // Handle messages
    while let Some(msg) = recv.next().await {
        match msg {
            Ok(Message::Binary(update)) => {
                doc.apply_update(&update).unwrap();
            }
            Ok(Message::Text(awareness)) => {
                doc.awareness().write().unwrap().set_local_state(awareness);
            }
            _ => {}
        }
    }
}
```

## Authentication

### Token Generation

```rust
pub fn generate_doc_token(
    secret: &SecretKey,
    doc_id: &str,
    expires_in: Duration,
) -> String {
    let claims = DocClaims {
        doc_id: doc_id.to_string(),
        exp: (Utc::now() + expires_in).timestamp() as u64,
    };
    encode(&HEADER, &claims, secret).unwrap()
}
```

### Token Validation

```rust
pub fn verify_doc_token(token: &str, secret: &PublicKey, doc_id: &str) -> Result<()> {
    let token_data = decode::<DocClaims>(token, secret)?;
    if token_data.claims.doc_id != doc_id {
        return Err(AuthError::InvalidDocId);
    }
    if token_data.claims.exp < Utc::now().timestamp() as u64 {
        return Err(AuthError::Expired);
    }
    Ok(())
}
```

### Authorization Header

```
Authorization: Bearer <jwt_token>
```

## Cloudflare Workers

Y-Sweet can be deployed to Cloudflare Workers:

### Configuration

```toml
# wrangler.toml
name = "y-sweet-worker"
compatibility_date = "2023-01-01"
r2_buckets = [{ binding = "BUCKET", bucket_name = "y-sweet" }]
```

### Worker Entry Point

```rust
#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let state = ctx.state();
    let mut server = ServerContext::new(env, state).await?;
    server.handle_request(req).await
}
```

### Threadless Execution

Workers use a threadless model:

```rust
pub struct Threadless {
    pending: Arc<Mutex<VecDeque<SpawnedFuture>>>,
}

impl SpawnFactory for Threadless {
    type Handle = JoinHandle<()>;

    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) -> Self::Handle {
        // Queue for event loop
    }
}
```

## Client Integration

### JavaScript Client

```typescript
import { Doc } from 'yjs';
import { WebsocketProvider } from 'y-websocket';

const doc = new Doc();
const provider = new WebsocketProvider(
    'ws://localhost:8080',
    'my-doc-id',
    doc,
    { params: { token: authToken } }
);

// Access shared types
const text = doc.getText('content');
text.insert(0, 'Hello, world!');
```

### React Hook

```typescript
import { useYDoc } from '@y-sweet/react';

function CollaborativeEditor({ docId }) {
    const doc = useYDoc(docId, { token: authToken });
    const text = doc.getText('content');

    return <Editor text={text} />;
}
```

## Debugger

Y-Sweet includes a document debugger:

```
http://localhost:8080/debug/:docId
```

Features:
- View document structure
- Inspect shared types
- Monitor updates
- Debug awareness state

## Persistence Model

### Update Log

Updates are stored incrementally:

```rust
pub fn push_update(&self, doc_name: &str, update: &[u8]) -> Result<()> {
    self.updates.push(Update::from(update));
    self.mark_dirty();
    Ok(())
}
```

### Flushing

Dirty documents are flushed to storage:

```rust
pub fn flush_doc_with(&self, doc_name: &str, state_vector: StateVector) -> Result<()> {
    let txn = self.doc.transact();
    let update = txn.encode_state_as_update_v1(&state_vector);
    self.store.put(doc_name, update).await?;
    Ok(())
}
```

### Compaction

Over time, updates can be compacted:

```rust
pub fn compact(&mut self) {
    // Encode full state
    let state = self.encode_state();
    // Replace update log with single state snapshot
    self.updates.clear();
    self.updates.push(state);
}
```

## Performance Considerations

### Memory Management

- Updates are held in memory until flushed
- Large documents may need explicit compaction
- Awareness state is separate from document

### Network Optimization

- Incremental updates minimize bandwidth
- State vectors enable efficient sync
- Binary protocol is compact

### Scaling

- Documents are independent (can shard by doc ID)
- No cross-document transactions
- Stateless HTTP handlers

## Dependencies

```toml
[dependencies]
yrs = "0.18"              # Yjs Rust implementation
yrs_kvstore = "0.18"      # KV store adapter
tokio = { version = "1", features = ["full"] }
axum = { version = "0.7", features = ["ws"] }
jsonwebtoken = "9"        # JWT auth
aws-sdk-s3 = "1"          # S3 storage
serde = { version = "1", features = ["derive"] }
```

## Use Cases

### Collaborative Documents

- Google Docs-style editors
- Wikis and knowledge bases
- Documentation platforms

### Real-time Whiteboards

- Miro-style collaboration
- Diagram editing
- Brainstorming sessions

### Multiplayer Applications

- Shared state for games
- Collaborative coding
- Pair programming tools

### Live Dashboards

- Real-time metrics
- Collaborative monitoring
- Shared control panels

## Comparison to Alternatives

| Feature | Y-Sweet | y-websocket | PartyKit |
|---------|---------|-------------|----------|
| Language | Rust | JS/TS | TS |
| Persistence | Built-in | External | Built-in |
| Cloudflare | Yes | No | Yes |
| Auth | JWT | Custom | Custom |
| Self-hosted | Yes | Yes | Limited |
