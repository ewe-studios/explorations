---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/p2p
explored_at: 2026-03-30
prerequisites: JavaScript/Node.js basics, Networking fundamentals helpful
---

# Zero to Hyperswarm Engineer - Complete Fundamentals

## Table of Contents

1. [What is Hyperswarm?](#what-is-hyperswarm)
2. [Why Hyperswarm?](#why-hyperswarm)
3. [Installation](#installation)
4. [Your First P2P Connection](#your-first-p2p-connection)
5. [Topic Discovery](#topic-discovery)
6. [Connection Handling](#connection-handling)
7. [Security Model](#security-model)
8. [Advanced Patterns](#advanced-patterns)

## What is Hyperswarm?

Hyperswarm is a **P2P networking library** for finding and connecting to peers interested in the same "topic". It uses DHT (Distributed Hash Table) for peer discovery and Noise encryption for secure connections. No central servers required.

### The Problem Hyperswarm Solves

**Traditional P2P:**
```
Want P2P connections → Need tracker server
                     → Or central signaling
                     → Manual NAT traversal
                     → Handle encryption yourself
Complexity: Infrastructure costs, single points of failure, security concerns
```

**With Hyperswarm:**
```
Join topic → DHT finds peers
           → Automatic NAT traversal
           → Noise encryption built-in
           → Connections just work
Simplicity: Serverless P2P networking
```

### Key Concepts

| Term | Definition |
|------|------------|
| **DHT** | Distributed Hash Table for peer discovery |
| **Topic** | 32-byte identifier for peer grouping |
| **Peer Discovery** | Finding peers via DHT queries |
| **Noise Protocol** | Encrypted handshakes |
| **Server Mode** | Announce yourself on DHT |
| **Client Mode** | Search for servers |

## Why Hyperswarm?

### Benefits

1. **Serverless**: No central infrastructure needed
2. **Encrypted**: Noise protocol E2E encryption
3. **Automatic NAT Traversal**: Works behind firewalls
4. **Topic-Based**: Easy peer grouping
5. **DHT-Powered**: Scales to millions of peers
6. **Stream-Based**: Familiar Node.js stream API

### When to Use Hyperswarm

**Good fit:**
- P2P file sharing
- Decentralized chat
- Collaborative editing (CRDTs)
- Local-first sync
- Mesh networking
- Distributed compute

**Not recommended:**
- Client-server apps
- When you need peer identity persistence
- Browser-only apps (needs WebRTC bridge)

## Installation

### npm Install

```bash
npm install hyperswarm
```

### Basic Usage

```javascript
const Hyperswarm = require('hyperswarm')

const swarm = new Hyperswarm()

swarm.on('connection', (conn, peerInfo) => {
  console.log('Connected to:', peerInfo.publicKey.toString('hex'))
})

swarm.join(Buffer.alloc(32).fill('my-topic'))
```

## Your First P2P Connection

### Server Side

```javascript
const Hyperswarm = require('hyperswarm')

const swarm = new Hyperswarm()

// Join topic as server (announce on DHT)
const topic = Buffer.alloc(32).fill('hello-world')
swarm.join(topic, { server: true, client: false })

swarm.on('connection', (conn, peerInfo) => {
  console.log('Client connected!')

  // Send message
  conn.write('Hello from server!')

  // Receive messages
  conn.on('data', data => {
    console.log('Received:', data.toString())
  })

  // Handle disconnect
  conn.on('end', () => {
    console.log('Client disconnected')
  })
})

// Wait for DHT announcement
await swarm.flush()
console.log('Server ready, waiting for clients...')
```

### Client Side

```javascript
const Hyperswarm = require('hyperswarm')

const swarm = new Hyperswarm()

// Join topic as client (search for servers)
const topic = Buffer.alloc(32).fill('hello-world')
swarm.join(topic, { server: false, client: true })

swarm.on('connection', (conn, peerInfo) => {
  console.log('Connected to server!')

  // Receive server message
  conn.on('data', data => {
    console.log('Server says:', data.toString())

    // Respond
    conn.write('Hello back!')
  })
})

// Wait for connections
await swarm.flush()
```

### Bidirectional Connections

```javascript
const Hyperswarm = require('hyperswarm')

const swarm = new Hyperswarm()

// Both server AND client (default)
const topic = Buffer.alloc(32).fill('my-topic')
swarm.join(topic) // { server: true, client: true }

swarm.on('connection', (conn, peerInfo) => {
  // Could be incoming or outgoing connection
  console.log('Peer connected:', peerInfo.publicKey.toString('hex').slice(0, 8))

  conn.pipe(conn) // Echo back for testing
})

await swarm.flush()
```

## Topic Discovery

### Creating Topics

```javascript
// Simple string topic (32 bytes)
const topic1 = Buffer.alloc(32).fill('chat-room-1')

// Hash-based topic (more secure)
const crypto = require('crypto')
const topic2 = crypto.createHash('sha256').update('my-app-v1').digest()

// App-specific namespace
const APP_NAMESPACE = 'my-p2p-app'
const topic3 = Buffer.alloc(32)
topic3.write(APP_NAMESPACE.padEnd(32))
```

### Multiple Topics

```javascript
const swarm = new Hyperswarm()

// Join multiple topics
const chatTopic = swarm.join(Buffer.alloc(32).fill('chat'))
const fileTopic = swarm.join(Buffer.alloc(32).fill('files'))

// Connections include topic info
swarm.on('connection', (conn, peerInfo) => {
  console.log('Topics:', peerInfo.topics)

  // Route based on topic
  if (peerInfo.topics.includes('chat')) {
    setupChat(conn)
  }
  if (peerInfo.topics.includes('files')) {
    setupFileTransfer(conn)
  }
})
```

### Discovery Lifecycle

```javascript
const discovery = swarm.join(topic, { server: true })

// Wait for full DHT announcement
await discovery.flushed()
console.log('Fully announced to DHT')

// Refresh announcement
await discovery.refresh({ server: true, client: false })

// Stop announcing
await discovery.destroy()
```

## Connection Handling

### Peer Info

```javascript
swarm.on('connection', (conn, peerInfo) => {
  // Peer's Noise public key
  console.log('Public Key:', peerInfo.publicKey.toString('hex'))

  // Associated topics (client mode only)
  console.log('Topics:', peerInfo.topics)

  // Connection priority
  console.log('Prioritized:', peerInfo.prioritized)

  // Ban problematic peers
  if (isBadPeer(peerInfo)) {
    peerInfo.ban()
    conn.destroy()
  }
})
```

### Connection Lifecycle

```javascript
swarm.on('connection', (conn, peerInfo) => {
  console.log('Connected!')

  // Data events
  conn.on('data', (data) => {
    console.log('Data received:', data)
  })

  // Error handling
  conn.on('error', (err) => {
    console.error('Connection error:', err)
  })

  // End of stream
  conn.on('end', () => {
    console.log('Connection ended')
  })

  // Close
  conn.on('close', () => {
    console.log('Connection closed')
  })

  // Send data
  conn.write('Hello!')
  conn.write(Buffer.from([0x01, 0x02, 0x03]))

  // End connection
  // conn.end()
})
```

### Managing Connections

```javascript
// Get all active connections
console.log('Connections:', swarm.connections.size)

// Get all peers
for (const [key, peerInfo] of swarm.peers) {
  console.log('Peer:', key.toString('hex').slice(0, 8))
}

// Connections in progress
console.log('Connecting:', swarm.connecting)

// Limit max connections
const swarm = new Hyperswarm({ maxPeers: 10 })
```

### Direct Peer Connection

```javascript
// Connect to specific peer by public key
const peerKey = Buffer.from('...', 'hex')
swarm.joinPeer(peerKey)

// Stop trying to connect
swarm.leavePeer(peerKey)
```

## Security Model

### Noise Encryption

```javascript
// Hyperswarm uses Noise protocol by default
// All connections are encrypted end-to-end

const swarm = new Hyperswarm({
  // Custom keypair (optional)
  keyPair: {
    publicKey: Buffer.from('...', 'hex'),
    secretKey: Buffer.from('...', 'hex')
  },

  // Or generate from seed
  seed: Buffer.alloc(32).fill('my-secret-seed')
})
```

### Firewall

```javascript
// Reject connections from certain peers
const bannedPeers = new Set()

const swarm = new Hyperswarm({
  firewall: (remotePublicKey) => {
    const keyHex = remotePublicKey.toString('hex')

    // Reject banned peers
    if (bannedPeers.has(keyHex)) {
      return false // Reject
    }

    return true // Accept
  }
})

// Ban a peer
function banPeer(peerInfo) {
  peerInfo.ban()
  bannedPeers.add(peerInfo.publicKey.toString('hex'))
}
```

### Peer Validation

```javascript
swarm.on('connection', (conn, peerInfo) => {
  // Verify peer identity
  const expectedKey = 'expected-public-key-hex'
  const actualKey = peerInfo.publicKey.toString('hex')

  if (actualKey !== expectedKey) {
    conn.destroy()
    return
  }

  // Protocol version check
  conn.write(JSON.stringify({ type: 'handshake', version: '1.0' }))

  conn.on('data', (data) => {
    try {
      const msg = JSON.parse(data)
      if (msg.type !== 'handshake') {
        conn.destroy()
      }
    } catch {
      conn.destroy()
    }
  })
})
```

## Advanced Patterns

### Chat Application

```javascript
const Hyperswarm = require('hyperswarm')

class P2PChat {
  constructor(roomName) {
    this.swarm = new Hyperswarm()
    this.topic = Buffer.alloc(32).fill(roomName)

    this.swarm.on('connection', (conn, peerInfo) => {
      this.setupConnection(conn)
    })

    this.swarm.join(this.topic)
  }

  setupConnection(conn) {
    conn.on('data', (data) => {
      const message = data.toString()
      console.log(`[${conn.remotePeer.slice(0, 8)}]: ${message}`)
    })

    // Broadcast join
    this.broadcast('System: New peer joined!')
  }

  send(message) {
    // Send to all connected peers
    for (const conn of this.swarm.connections) {
      conn.write(message)
    }
  }

  broadcast(message) {
    this.send(`[SYSTEM] ${message}`)
  }

  async flush() {
    await this.swarm.flush()
  }
}

// Usage
const chat = new P2PChat('my-chat-room')
chat.flush().then(() => {
  chat.send('Hello, world!')
})
```

### File Transfer

```javascript
const Hyperswarm = require('hyperswarm')
const fs = require('fs')

class P2PFileTransfer {
  constructor() {
    this.swarm = new Hyperswarm()
    this.topic = Buffer.alloc(32).fill('file-transfer-v1')

    this.swarm.on('connection', (conn) => {
      this.setupFileHandler(conn)
    })

    this.swarm.join(this.topic, { server: true })
  }

  setupFileHandler(conn) {
    let receiving = false
    let chunks = []

    conn.on('data', (data) => {
      if (!receiving) {
        // First message is filename
        const filename = data.toString()
        console.log('Receiving file:', filename)
        receiving = true
      } else {
        chunks.push(data)
      }
    })

    conn.on('end', () => {
      if (chunks.length > 0) {
        fs.writeFileSync('received-file', Buffer.concat(chunks))
        console.log('File saved!')
      }
    })
  }

  async sendFile(peerKey, filename) {
    const discovery = this.swarm.joinPeer(peerKey)
    await discovery.flushed()

    const conn = Array.from(this.swarm.connections)[0]

    // Send filename first
    conn.write(filename)

    // Then file contents
    const stream = fs.createReadStream(filename)
    stream.on('data', chunk => conn.write(chunk))
    stream.on('end', () => conn.end())
  }

  async flush() {
    await this.swarm.flush()
  }
}
```

### Suspend/Resume (Mobile)

```javascript
const swarm = new Hyperswarm()

// When app goes to background
async function onSuspend() {
  await swarm.suspend({
    log: (msg) => console.log('Suspending:', msg)
  })
}

// When app comes to foreground
async function onResume() {
  await swarm.resume({
    log: (msg) => console.log('Resuming:', msg)
  })
}

// Mobile lifecycle hooks
document.addEventListener('pause', onSuspend)
document.addEventListener('resume', onResume)
```

### Multi-Topic Routing

```javascript
class MultiTopicNode {
  constructor() {
    this.swarm = new Hyperswarm()
    this.topics = new Map()
  }

  joinTopic(name, handler) {
    const topic = Buffer.alloc(32).fill(name)
    const discovery = this.swarm.join(topic)

    this.topics.set(name, { topic, discovery, handler })

    this.swarm.on('connection', (conn, peerInfo) => {
      // Route based on topic
      for (const topicName of peerInfo.topics) {
        const topicInfo = this.topics.get(topicName)
        if (topicInfo) {
          topicInfo.handler(conn, peerInfo)
        }
      }
    })
  }

  async flush() {
    await this.swarm.flush()
  }
}

// Usage
const node = new MultiTopicNode()

node.joinTopic('chat', (conn) => {
  conn.on('data', d => console.log('Chat:', d.toString()))
})

node.joinTopic('files', (conn) => {
  conn.on('data', d => console.log('File data:', d.length))
})

node.flush()
```

---

**Next Steps:**
- [01-hyperswarm-exploration.md](./01-hyperswarm-exploration.md) - Full architecture
- [02-hyperdht-deep-dive.md](./02-hyperdht-deep-dive.md) - DHT internals
- [03-p2p-applications-deep-dive.md](./03-p2p-applications-deep-dive.md) - Application patterns
