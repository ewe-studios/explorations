---
location: /home/darkvoid/Boxxed/@formulas/src.Peer2Peer/hyperswarm
repository: git@github.com:holepunchto/hyperswarm.git
explored_at: 2026-03-30
language: JavaScript
category: P2P Networking, DHT
---

# Hyperswarm - Exploration

## Overview

Hyperswarm is a **high-level P2P networking library** for finding and connecting to peers interested in the same "topic". Built on top of hyperdht (a Kademlia-style DHT), it provides Noise-encrypted connections with automatic NAT traversal and peer discovery.

### Key Value Proposition

- **Serverless P2P**: No central infrastructure required
- **DHT-Based Discovery**: Scales to millions of peers
- **Noise Encryption**: End-to-end encrypted connections
- **Topic-Based**: Simple peer grouping mechanism
- **Stream API**: Familiar Node.js duplex stream interface
- **Mobile Ready**: Suspend/resume for mobile apps

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Hyperswarm Architecture                       │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   Peer A        │  │   Peer B        │  │   Peer C        │ │
│  │   (Server)      │  │   (Client)      │  │   (Server)      │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘ │
│           │                    │                    │           │
│           └────────────────────┼────────────────────┘           │
│                      Encrypted Connections                      │
│                      (Noise Protocol)                           │
│                                                                 │
│           ┌─────────────────────┴─────────────────────┐        │
│           │                                           │        │
│           ▼                                           ▼        │
│  ┌─────────────────┐                        ┌─────────────────┐ │
│  │   HyperDHT      │                        │   HyperDHT      │ │
│  │   (Kademlia)    │◄────── DHT Queries ──►│   (Kademlia)    │ │
│  │   - Peer lookup │                        │   - Announce    │ │
│  │   - NAT traversal│                       │   - Discovery   │ │
│  └─────────────────┘                        └─────────────────┘ │
│                                                                 │
│  Topic: "chat-room-1" (32-byte buffer)                         │
│  - Servers announce to DHT                                     │
│  - Clients query DHT for servers                               │
│  - Direct P2P connections established                          │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
hyperswarm/
├── index.js                # Main Hyperswarm class
├── lib/
│   ├── peer-discovery.js   # PeerDiscovery API
│   ├── peer-info.js        # PeerInfo class
│   └── queue.js            # Connection queue management
│
├── test/
│   ├── basic.js            # Basic functionality tests
│   ├── discovery.js        # Discovery tests
│   └── flood.js            # Flood/network stress tests
│
├── examples/
│   ├── chat.js             # P2P chat example
│   ├── file-transfer.js    # File sharing example
│   └── multi-topic.js      # Multi-topic example
│
└── package.json
```

## Core Concepts

### 1. Topics

Topics are 32-byte Buffers that group peers:

```javascript
// Simple topic
const topic = Buffer.alloc(32).fill('my-chat-room')

// Hash-based topic
const crypto = require('crypto')
const topic = crypto.createHash('sha256')
  .update('my-app-v1')
  .digest()

// Topic must be exactly 32 bytes
```

### 2. Server vs Client Mode

```javascript
// Server mode: Announce on DHT, accept connections
swarm.join(topic, { server: true, client: false })

// Client mode: Query DHT, connect to servers
swarm.join(topic, { server: false, client: true })

// Both modes (default): Accept and initiate connections
swarm.join(topic) // { server: true, client: true }
```

### 3. Peer Discovery

```javascript
const discovery = swarm.join(topic, { server: true })

// Wait for full DHT announcement
await discovery.flushed()
console.log('Fully announced')

// Refresh announcement
await discovery.refresh({ server: true })

// Stop announcing
await discovery.destroy()
```

### 4. Peer Info

```javascript
swarm.on('connection', (conn, peerInfo) => {
  // Noise public key (32 bytes)
  console.log('Peer ID:', peerInfo.publicKey.toString('hex'))

  // Associated topics (client mode only)
  console.log('Topics:', peerInfo.topics)

  // Priority reconnection
  peerInfo.prioritized = true

  // Ban peer
  peerInfo.ban()
})
```

## Connection Flow

### Server Side Flow

```
1. Create Hyperswarm instance
         ↓
2. Generate Noise keypair (or use seed)
         ↓
3. Join topic with server: true
         ↓
4. Announce to DHT (Kademlia nodes)
         ↓
5. Wait for discovery.flushed()
         ↓
6. Accept incoming connections
```

### Client Side Flow

```
1. Create Hyperswarm instance
         ↓
2. Join topic with client: true
         ↓
3. Query DHT for servers
         ↓
4. Connect to discovered peers
         ↓
5. Establish Noise-encrypted connection
```

### Connection Multiplexing

```
Single connection per peer (regardless of topics):

Peer A ──[Connection 1]── Peer B
         ├─ Topic: chat
         ├─ Topic: files
         └─ Topic: sync

All topics multiplexed over single encrypted connection
```

## API Deep Dive

### Hyperswarm Constructor

```javascript
const Hyperswarm = require('hyperswarm')

const swarm = new Hyperswarm({
  // Noise keypair (auto-generated if not provided)
  keyPair: {
    publicKey: Buffer.from('...', 'hex'),
    secretKey: Buffer.from('...', 'hex')
  },

  // Deterministic keypair from seed
  seed: Buffer.alloc(32).fill('my-seed'),

  // Max connections
  maxPeers: 100,

  // Connection firewall
  firewall: (remotePublicKey) => {
    // Return false to reject
    return !bannedPeers.has(remotePublicKey.toString('hex'))
  },

  // Custom DHT instance
  dht: new DHT()
})
```

### Join Topic

```javascript
const discovery = swarm.join(topic, opts = {})

// Options:
// - server: true/false (default: true)
// - client: true/false (default: true)

// Discovery methods:
await discovery.flushed()  // Wait for DHT announcement
await discovery.refresh(opts) // Refresh with new options
await discovery.destroy()  // Stop discovery
```

### Connection Events

```javascript
// New connection
swarm.on('connection', (conn, peerInfo) => {
  // conn is a duplex stream
  conn.write('Hello!')
  conn.on('data', data => console.log(data))
  conn.on('end', () => console.log('Stream ended'))
  conn.on('error', err => console.error(err))
  conn.on('close', () => console.log('Connection closed'))
})

// State update
swarm.on('update', () => {
  console.log('Swarm state changed')
  console.log('Connections:', swarm.connections.size)
  console.log('Connecting:', swarm.connecting)
})
```

### Managing Peers

```javascript
// Get all peers
for (const [key, peerInfo] of swarm.peers) {
  console.log('Peer:', key.toString('hex').slice(0, 8))
}

// Connect to specific peer
swarm.joinPeer(Buffer.from('...', 'hex'))

// Stop connecting to peer
swarm.leavePeer(Buffer.from('...', 'hex'))

// Ban peer (no reconnection attempts)
peerInfo.ban()
peerInfo.ban('spam') // With reason
```

### Flush and Suspend

```javascript
// Wait for all pending operations
await swarm.flush()

// Suspend (for mobile background)
await swarm.suspend({
  log: (msg) => console.log('Suspended:', msg)
})

// Resume
await swarm.resume({
  log: (msg) => console.log('Resumed:', msg)
})
```

## Peer Discovery API

### Discovery Object

```javascript
const discovery = swarm.join(topic)

// Properties
discovery.topic       // 32-byte Buffer
discovery.started     // Boolean
discovery.ended       // Boolean
discovery.flushed()   // Promise

// Methods
await discovery.refresh({
  server: true,
  client: false
})

await discovery.destroy()
```

### Discovery Events

```javascript
discovery.on('update', () => {
  console.log('Discovery state changed')
})

discovery.on('announcing', () => {
  console.log('Announcing to DHT')
})

discovery.on('complete', () => {
  console.log('DHT announcement complete')
})
```

## Security Considerations

### Noise Protocol

```
Handshake Pattern (XX):
  ← s
  ...
  → e, es
  ← e, ee
  → s, se
  ← s

Where:
  s = static key
  e = ephemeral key
  es/ee/se = DH shared secrets
```

### Peer Verification

```javascript
swarm.on('connection', (conn, peerInfo) => {
  // Verify expected peer
  const expected = 'expected-public-key-hex'
  const actual = peerInfo.publicKey.toString('hex')

  if (actual !== expected) {
    conn.destroy()
    return
  }

  // Custom auth handshake
  conn.write(JSON.stringify({
    type: 'auth',
    nonce: crypto.randomBytes(32)
  }))
})
```

### Firewall

```javascript
const swarm = new Hyperswarm({
  firewall: (remotePublicKey) => {
    const hex = remotePublicKey.toString('hex')

    // Check against blocklist
    if (blocklist.has(hex)) {
      return false
    }

    // Rate limiting
    const connectionCount = connectionsByPeer.get(hex)
    if (connectionCount > MAX_CONNECTIONS) {
      return false
    }

    return true
  }
})
```

## Real-World Patterns

### Request/Response Pattern

```javascript
class P2PRPC {
  constructor(swarm) {
    this.swarm = swarm
    this.pendingRequests = new Map()
    this.requestId = 0

    swarm.on('connection', (conn) => {
      this.setupHandler(conn)
    })
  }

  setupHandler(conn) {
    const buffers = []

    conn.on('data', (data) => {
      buffers.push(data)

      // Try to parse complete message
      try {
        const msg = JSON.parse(Buffer.concat(buffers))

        if (msg.type === 'response' && this.pendingRequests.has(msg.id)) {
          const resolve = this.pendingRequests.get(msg.id)
          this.pendingRequests.delete(msg.id)
          resolve(msg.result)
        } else if (msg.type === 'request') {
          // Handle incoming request
          this.handleRequest(conn, msg)
        }

        buffers.length = 0
      } catch (e) {
        // Wait for more data
      }
    })
  }

  async call(method, params) {
    const conn = Array.from(this.swarm.connections)[0]
    const id = ++this.requestId

    return new Promise((resolve) => {
      this.pendingRequests.set(id, resolve)

      conn.write(JSON.stringify({
        type: 'request',
        id,
        method,
        params
      }))
    })
  }

  async handleRequest(conn, msg) {
    const result = await this.execute(msg.method, msg.params)

    conn.write(JSON.stringify({
      type: 'response',
      id: msg.id,
      result
    }))
  }
}
```

### Pub/Sub Pattern

```javascript
class P2PPubSub {
  constructor(topic) {
    this.swarm = new Hyperswarm()
    this.subscribers = new Set()

    this.swarm.join(Buffer.alloc(32).fill(topic))

    this.swarm.on('connection', (conn) => {
      this.subscribers.add(conn)

      conn.on('close', () => {
        this.subscribers.delete(conn)
      })
    })
  }

  publish(message) {
    const data = Buffer.from(JSON.stringify(message))

    for (const conn of this.subscribers) {
      conn.write(data)
    }
  }

  subscribe(callback) {
    this.swarm.on('connection', (conn) => {
      conn.on('data', (data) => {
        try {
          const msg = JSON.parse(data)
          callback(msg)
        } catch (e) {
          // Invalid message
        }
      })
    })
  }

  async flush() {
    await this.swarm.flush()
  }
}
```

### Gossip Protocol

```javascript
class P2PGossip {
  constructor(nodeId) {
    this.swarm = new Hyperswarm()
    this.nodeId = nodeId
    this.knownPeers = new Map()
    this.seenMessages = new Set()

    this.swarm.on('connection', (conn, peerInfo) => {
      const peerId = peerInfo.publicKey.toString('hex')
      this.knownPeers.set(peerId, conn)

      // Exchange peer lists
      this.exchangePeers(conn)

      conn.on('data', (data) => {
        const msg = JSON.parse(data)
        this.handleGossip(msg)
      })
    })

    this.swarm.join(Buffer.alloc(32).fill('gossip-network'))
  }

  exchangePeers(conn) {
    const peerList = Array.from(this.knownPeers.keys())

    conn.write(JSON.stringify({
      type: 'peers',
      peers: peerList
    }))

    conn.on('data', (data) => {
      const msg = JSON.parse(data)
      if (msg.type === 'peers') {
        for (const peerId of msg.peers) {
          if (!this.knownPeers.has(peerId)) {
            this.swarm.joinPeer(Buffer.from(peerId, 'hex'))
          }
        }
      }
    })
  }

  broadcast(message) {
    const msgId = `${this.nodeId}-${Date.now()}`

    if (this.seenMessages.has(msgId)) return
    this.seenMessages.add(msgId)

    const msg = {
      id: msgId,
      from: this.nodeId,
      data: message,
      ttl: 5
    }

    for (const conn of this.knownPeers.values()) {
      conn.write(JSON.stringify(msg))
    }
  }

  handleGossip(msg) {
    if (this.seenMessages.has(msg.id)) return
    this.seenMessages.add(msg.id)

    // Process message
    console.log('Received gossip:', msg.data)

    // Forward with decremented TTL
    if (msg.ttl > 0) {
      msg.ttl--
      for (const conn of this.knownPeers.values()) {
        conn.write(JSON.stringify(msg))
      }
    }
  }

  async flush() {
    await this.swarm.flush()
  }
}
```

## Integration with Hyperswarm Ecosystem

### Hypercore Integration

```javascript
const Hyperswarm = require('hyperswarm')
const Hypercore = require('hypercore')

async function replicateFeed(feed, topic) {
  const swarm = new Hyperswarm()

  swarm.join(topic, {
    server: true,
    client: true
  })

  swarm.on('connection', (conn, peerInfo) => {
    // Replicate hypercore over connection
    feed.replicate(conn)
  })

  await swarm.flush()
}
```

### Hyperbee Integration

```javascript
const Hyperswarm = require('hyperswarm')
const Hyperbee = require('hyperbee')
const Hypercore = require('hypercore')

async function createDistributedDB(topic) {
  const feed = new Hypercore('./my-db')
  const db = new Hyperbee(feed)

  const swarm = new Hyperswarm()
  swarm.join(Buffer.alloc(32).fill(topic))

  swarm.on('connection', (conn) => {
    feed.replicate(conn)
  })

  await swarm.flush()

  return db
}
```

---

## Related Deep Dives

- [00-zero-to-hyperswarm-engineer.md](./00-zero-to-hyperswarm-engineer.md) - Fundamentals
- [02-hyperdht-deep-dive.md](./02-hyperdht-deep-dive.md) - DHT internals
- [03-p2p-applications-deep-dive.md](./03-p2p-applications-deep-dive.md) - Application patterns
