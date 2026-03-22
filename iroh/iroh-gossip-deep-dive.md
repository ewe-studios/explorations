# iroh-gossip Deep Dive

## Overview

`iroh-gossip` implements a gossip protocol for broadcasting messages to peers subscribed to topics. It combines two well-established algorithms: HyParView for swarm membership management and PlumTree for efficient gossip broadcasting.

**Version:** 0.90.0
**Repository:** https://github.com/n0-computer/iroh-gossip
**License:** MIT/Apache-2.0

---

## Architecture and Design Decisions

### Protocol Composition

The iroh-gossip protocol is composed of two layered protocols:

1. **HyParView (Membership Protocol)**: Manages peer connectivity and maintains a partial view of the swarm
2. **PlumTree (Gossip Protocol)**: Efficiently broadcasts messages to all peers in a topic

This separation allows each protocol to optimize for its specific concern while working together seamlessly.

### HyParView Design

Based on the paper "HyParView: a membership protocol for reliable gossip-based broadcast" by Leitao et al.

**Key Design Decisions:**

1. **Dual View Architecture**: Each peer maintains two sets of peers:
   - **Active View** (default: 5 peers): Peers with active connections
   - **Passive View** (default: 30 peers): Address book of additional peers

2. **Bidirectional Connections**: The protocol ensures all active connections are bidirectional, improving reliability.

3. **Shuffle Operation**: Regularly exchanges nodes for the passive view, maintaining network health and preventing partitioning.

4. **Failure Recovery**: When an active peer goes offline, its slot is filled from the passive set.

### PlumTree Design

Based on the paper "PlumTree: an epidemic tree for scalable and reliable data streaming" by Leitao et al.

**Key Design Decisions:**

1. **Eager/Lazy Push Strategy**:
   - **Eager Peers**: Receive full messages immediately
   - **Lazy Peers**: Receive only message hashes (IHAVE messages)

2. **Self-Optimization**: When a lazy peer requests a message (via GRAFT), it's promoted to the eager set. Over time, this creates an optimal broadcast tree based on actual message propagation patterns.

3. **Message Deduplication**: Each message is uniquely identified by its hash, preventing duplicate processing.

### Topic-Based Scoping

All protocol messages are namespaced by a `TopicId` (32-byte identifier):
- Each topic is a separate broadcast tree with independent membership
- Joining multiple topics increases connection count and routing table size
- Topics enable application-level message scoping

### IO-Less State Machine

A critical architectural decision is that the protocol implementation is **IO-less**:
- The `State` struct contains pure protocol state
- All I/O operations are emitted as `OutEvent`s
- The implementer handles actual networking based on events
- This design enables easy testing and simulation

---

## Key APIs and Data Structures

### Core Types

```rust
/// Topic identifier - 32-byte opaque identifier
pub struct TopicId([u8; 32]);

/// Peer identifier trait
pub trait PeerIdentity:
    Hash + Eq + Ord + Copy + fmt::Debug + Serialize + DeserializeOwned
{
}

/// Opaque peer data for connection information
pub struct PeerData(Bytes);

/// Protocol configuration
pub struct Config {
    pub membership: HyparviewConfig,
    pub broadcast: PlumtreeConfig,
    pub max_message_size: usize,
}
```

### Gossip Handle

```rust
/// Main entry point for gossip operations
#[derive(Clone)]
pub struct Gossip {
    pub(crate) inner: Arc<Inner>,
}

impl Gossip {
    /// Create a new gossip instance
    pub fn builder() -> Builder { }

    /// Handle incoming connection
    pub async fn handle_connection(&self, conn: Connection) -> Result<(), Error> { }

    /// Join a topic with initial peers
    pub async fn join(&self, topic: TopicId, peers: Vec<NodeAddr>) -> Result<(), Error> { }

    /// Broadcast a message to topic
    pub async fn broadcast(&self, topic: TopicId, message: Bytes) -> Result<(), Error> { }

    /// Subscribe to topic messages
    pub async fn subscribe(&self, topic: TopicId) -> Result<broadcast::Receiver<Event>, Error> { }

    /// Leave a topic
    pub async fn leave(&self, topic: TopicId) -> Result<(), Error> { }

    /// Shutdown gossip
    pub async fn shutdown(&self) -> Result<(), Error> { }
}
```

### Protocol State

```rust
/// The protocol state - IO-less state machine
pub struct State<PI, R> {
    me: PI,                    // Local peer identity
    me_data: PeerData,         // Local peer data
    config: Config,            // Protocol configuration
    rng: R,                    // Random number generator
    states: HashMap<TopicId, topic::State<PI, R>>,
    outbox: Outbox<PI>,        // Pending output events
    peer_topics: ConnsMap<PI>, // Track which peers are in which topics
}

impl<PI: PeerIdentity, R: Rng> State<PI, R> {
    /// Create new protocol state
    pub fn new(me: PI, me_data: PeerData, config: Config, rng: R) -> Self { }

    /// Handle input event, return output events
    pub fn handle(&mut self, event: InEvent<PI>) -> impl Iterator<Item = OutEvent<PI>> { }
}
```

### Events

```rust
/// Input events to the protocol
pub enum InEvent<PI> {
    RecvMessage(PI, Message<PI>),      // Message from network
    Command(TopicId, Command<PI>),     // Application command
    TimerExpired(Timer<PI>),           // Scheduled timer fired
    PeerDisconnected(PI),              // Peer connection lost
    UpdatePeerData(PeerData),          // Update local peer data
}

/// Output events from the protocol
pub enum OutEvent<PI> {
    SendMessage(PI, Message<PI>),      // Send message to peer
    EmitEvent(TopicId, Event<PI>),     // Emit event to application
    ScheduleTimer(Duration, Timer<PI>), // Schedule timer
    DisconnectPeer(PI),                // Close peer connection
    PeerData(PI, PeerData),            // Peer data updated
}

/// Application-facing events
pub enum Event<PI> {
    NeighborUp(PI),       // New neighbor connected
    NeighborDown(PI),     // Neighbor disconnected
    Received(Bytes),      // Message received
}
```

### Configuration

```rust
/// HyParView configuration
pub struct Config {
    pub active_view_capacity: usize,    // Default: 5
    pub passive_view_capacity: usize,   // Default: 30
    pub max_peers: usize,               // Default: 15
    pub shuffle_interval: Duration,     // Default: 30s
}

/// PlumTree configuration
pub struct PlumtreeConfig {
    pub eager_push_factor: f64,         // Default: 0.6
    pub lazy_push_timeout: Duration,    // Default: 500ms
}
```

---

## Protocol Details

### Wire Protocol (ALPN: `/iroh-gossip/1`)

All messages are wrapped in a topic-aware envelope:

```rust
pub struct Message<PI> {
    pub topic: TopicId,
    pub message: topic::Message<PI>,
}

/// Inner message types
pub enum Message<PI> {
    /// HyParView messages
    Join(Join),
    Forward(PeerInfo<PI>),
    Shuffle(Shuffle),
    ShuffleReply(Vec<PeerInfo<PI>>),

    /// PlumTree messages
    Gossip(Gossip),
    IHaves { from: PI, ids: Vec<MessageId> },
    Graft(PeerInfo<PI>),
    Prune,

    /// Keep-alive
    Heartbeat,
}
```

### HyParView Message Flow

#### Join Protocol
```
New Peer                         Existing Peer
    |                                 |
    |--------- Join ----------------->|
    |<-------- Forward ---------------|
    |                                 |
    (Peer added to active view)
```

#### Shuffle Exchange
```
Peer A                           Peer B
   |                                 |
   |--------- Shuffle -------------->|
   |<------- ShuffleReply -----------|
   |                                 |
   (Both update passive views)
```

### PlumTree Message Flow

#### Broadcast Propagation
```
Sender                        Eager Peer                  Lazy Peer
   |                              |                            |
   |------ Gossip (full) -------->|                            |
   |------ Gossip (full) ----------------------------->|       |
   |------ IHAVE (hash) ------------------------------------->|
   |                                                         |
   |                            (Timeout expires)            |
   |                                                         |
   |<----- GRAFT (request) ----------------------------------|
   |------ Gossip (full) ----------------------------------->|
   |                                                         |
   (Lazy peer promoted to eager set)
```

### Fingerprint-Based Deduplication

Messages are identified by BLAKE3 hash:

```rust
pub struct MessageId([u8; 32]);

impl MessageId {
    pub fn from_content(content: &[u8]) -> Self {
        Self(blake3::hash(content).into())
    }
}
```

This enables:
- Efficient IHAVE messages (just hashes, not full content)
- Duplicate detection across multiple paths
- Compact message tracking

### Timer Management

The protocol uses timers for:
- Shuffle intervals (HyParView)
- Lazy push timeouts (PlumTree)
- Heartbeat generation

```rust
pub struct Timer<PI> {
    topic: TopicId,
    timer: topic::Timer<PI>,
}

// Timer types
pub enum Timer<PI> {
    Shuffle(TopicId),           // Shuffle passive view
    LazyPush(MessageId, PI),    // Send IHAVE to lazy peer
    Heartbeat(TopicId),         // Send heartbeat
}
```

---

## Integration with Main Iroh Endpoint

### Protocol Handler

```rust
impl ProtocolHandler for Gossip {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        self.handle_connection(connection)
            .await
            .map_err(AcceptError::from_err)?;
        Ok(())
    }

    async fn shutdown(&self) {
        if let Err(err) = self.shutdown().await {
            warn!("error while shutting down gossip: {err:#}");
        }
    }
}
```

### Endpoint Integration

```rust
// Create gossip instance
let gossip = Gossip::builder()
    .max_message_size(4096)
    .membership_config(HyparviewConfig::default())
    .broadcast_config(PlumtreeConfig::default())
    .spawn(endpoint);

// Register with router
let router = Router::builder(endpoint)
    .accept(GOSSIP_ALPN, gossip.clone())
    .build()
    .await?;
```

### Peer Discovery Integration

Gossip integrates with iroh's peer discovery:

```rust
// Add peers from discovery
gossip.add_peers(vec![node_addr]).await?;

// Peer data includes relay information
let peer_data = PeerData::new(relay_info.encode());
gossip.update_peer_data(peer_data).await?;
```

### Docs Engine Integration

The docs engine uses gossip for sync event distribution:

```rust
// In docs engine
let gossip_event = gossip.subscribe(namespace_id).await?;
tokio::spawn(async move {
    while let Ok(event) = gossip_event.recv().await {
        // Forward to sync engine
        sync_handle.handle_gossip_event(event).await?;
    }
});
```

---

## Production Usage Patterns

### Basic Pub/Sub

```rust
use iroh_gossip::{Gossip, TopicId};

// Create gossip instance
let gossip = Gossip::builder().spawn(endpoint);

// Define topic
let topic = TopicId::from_bytes([0u8; 32]);

// Join topic with initial peers
gossip.join(topic, vec![peer_addr]).await?;

// Subscribe to messages
let mut subscriber = gossip.subscribe(topic).await?;
tokio::spawn(async move {
    while let Ok(event) = subscriber.recv().await {
        if let Event::Received(data) = event {
            println!("Received: {:?}", data);
        }
    }
});

// Broadcast message
gossip.broadcast(topic, b"Hello, swarm!".to_vec().into()).await?;
```

### Chat Application Pattern

```rust
// Chat room as a topic
let room_topic = TopicId::from(room_name.as_bytes());

// Join room
gossip.join(room_topic, bootstrap_peers.clone()).await?;

// Handle incoming messages
let mut msgs = gossip.subscribe(room_topic).await?;
while let Ok(event) = msgs.recv().await {
    match event {
        Event::Received(data) => {
            let message: ChatMessage = serde_json::from_slice(&data)?;
            display_message(message);
        }
        Event::NeighborUp(peer) => {
            println!("Peer joined: {:?}", peer);
        }
        Event::NeighborDown(peer) => {
            println!("Peer left: {:?}", peer);
        }
    }
}

// Send message
let message = ChatMessage { text: "Hello!".to_string() };
let data = serde_json::to_vec(&message)?;
gossip.broadcast(room_topic, data.into()).await?;
```

### Multi-Topic Subscription

```rust
// Subscribe to multiple topics
let mut topics = vec![];
for topic_id in &topic_ids {
    let sub = gossip.subscribe(*topic_id).await?;
    topics.push((*topic_id, sub));
}

// Fan-in handler
let (tx, mut rx) = mpsc::channel(100);
for (topic_id, mut subscriber) in topics {
    let tx = tx.clone();
    tokio::spawn(async move {
        while let Ok(event) = subscriber.recv().await {
            tx.send((topic_id, event)).await.ok();
        }
    });
}
```

### Dynamic Peer Discovery

```rust
// Periodically refresh peer list
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;

        // Query discovery for new peers
        let peers = discovery.get_peers(topic).await?;

        // Gossip will handle connection management
        gossip.join(topic, peers).await.ok();
    }
});
```

---

## Rust Revision Notes

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| rand | 0.8.5 | Random number generation |
| rand_core | 0.6.4 | RNG traits |
| blake3 | 1.8 | Message hashing |
| bytes | 1.7 | Byte buffer management |
| ed25519-dalek | 2.0.0 | Peer identity signatures |
| postcard | 1.x | Serialization |
| tokio | 1.x | Async runtime |
| tracing | 0.1 | Logging |

### Notable Rust Patterns

1. **Typestate-like Configuration**: Builder pattern enforces configuration before spawning
2. **Trait-Based Peer Identity**: Generic `PeerIdentity` trait for flexibility
3. **IO-Less State Machine**: Pure state transitions with event emission
4. **Broadcast Channels**: `tokio::sync::broadcast` for topic event distribution

### Concurrency Model

- Actor-based architecture with message passing
- `Arc<Inner>` for thread-safe handle cloning
- Mutex-free hot path through careful design
- Channels for all inter-component communication

### Serialization

Uses Postcard for efficient wire encoding:
- Compact binary format
- Zero-copy deserialization where possible
- Length-prefixed messages for streaming

### Performance Optimizations

1. **Lazy Message Delivery**: IHAVE messages reduce redundant traffic
2. **Passive View Management**: Reduces connection churn
3. **Message Deduplication**: Hash-based dedup prevents reprocessing
4. **Timer Coalescing**: Related timers can be batched

### Potential Enhancements

1. **Compression**: Optional message compression for large payloads
2. **Priority Messages**: Support for high-priority broadcast messages
3. **Streaming**: Support for large message streaming
4. **Metrics**: Enhanced observability for production monitoring

---

## Simulation and Testing

### Built-in Simulator

The crate includes a simulation framework for testing protocol behavior:

```rust
use iroh_gossip::proto::sim::{Network, NetworkConfig, LatencyConfig};

// Create simulated network
let config = Config::default();
let network_config = NetworkConfig {
    proto: config,
    latency: LatencyConfig::default_static(),
};
let mut network = Network::new(network_config, rng);

// Add nodes
for i in 0..6 {
    network.insert(i);
}

// Run simulation
network.command(0, topic, Command::Join(vec![1, 2]));
network.run_trips(4);

// Verify events
let events = network.events_sorted();
assert!(events.contains(&(0, topic, Event::NeighborUp(1))));
```

### Test Utilities

```rust
#[cfg(feature = "test-utils")]
pub mod sim;

// Test helpers for protocol verification
pub fn check_synchronicity(&self) -> bool;  // Verify state consistency
pub fn report(&self) -> String;             // Generate diagnostic report
```

---

## Summary

`iroh-gossip` provides a production-ready gossip protocol with:

- **Proven Algorithms**: HyParView and PlumTree provide reliable, scalable communication
- **IO-Less Design**: Pure state machine enables easy testing and simulation
- **Topic Scoping**: Clean separation of message domains
- **Self-Healing**: Automatic recovery from peer failures
- **Optimized Delivery**: Lazy/eager push strategy minimizes redundant traffic

The module serves as the communication backbone for iroh's distributed systems, enabling efficient broadcast and pub/sub patterns across peer networks.
