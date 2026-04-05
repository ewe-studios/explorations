# Zero to Cap'n Web: Complete Guide

**Last Updated:** 2026-04-05

---

## Table of Contents

1. [Introduction](#introduction)
2. [What is Cap'n Web?](#what-is-capn-web)
3. [Core Concepts](#core-concepts)
4. [Installation](#installation)
5. [Quick Start](#quick-start)
6. [RPC Basics](#rpc-basics)
7. [Promise Pipelining](#promise-pipelining)
8. [Streaming](#streaming)
9. [Resource Management](#resource-management)
10. [Security](#security)
11. [Platform Integration](#platform-integration)

---

## Introduction

**Cap'n Web** is a JavaScript-native RPC system inspired by Cap'n Proto, designed for the web stack. It provides object-capability RPC with promise pipelining, bidirectional calling, and streaming support—all in under 10kB gzipped with no dependencies.

```bash
npm install capnweb
```

### Key Features

| Feature | Description |
|---------|-------------|
| **Object-Capability RPC** | Pass objects and functions by reference |
| **Promise Pipelining** | Chain RPC calls in single round trip |
| **Bidirectional Calling** | Client and server can call each other |
| **Streaming** | ReadableStream/WritableStream with flow control |
| **Multiple Transports** | HTTP batch, WebSocket, MessagePort, custom |
| **Workers Compatible** | Interoperable with Cloudflare Workers RPC |
| **TypeScript Support** | Full type inference and auto-complete |
| **Human Readable** | JSON-based serialization |

### Cap'n Web vs Cap'n Proto

| Aspect | Cap'n Proto | Cap'n Web |
|--------|-------------|-----------|
| Schema | Required | None |
| Serialization | Binary | JSON-based |
| Size | ~100KB+ | <10KB gzipped |
| Transport | Custom | HTTP, WebSocket, postMessage |
| Boilerplate | High | Minimal |
| JavaScript Native | No | Yes |

---

## What is Cap'n Web?

### The Problem

Traditional RPC systems suffer from:

1. **High Latency** - Multiple round trips for chained calls
2. **Boilerplate** - Schema definitions, code generation
3. **Unidirectional** - Only client can call server
4. **No Streaming** - Limited support for streams
5. **Heavy Bundles** - Large runtime libraries

### The Cap'n Web Solution

```typescript
// Single round trip: authenticate + get user ID + fetch profile
using api = newWebSocketRpcSession<PublicApi>("wss://example.com/api");
using authedApi = api.authenticate(apiToken);
let userId = await authedApi.getUserId();
let profile = await api.getUserProfile(userId);
// All sent in ONE batch
```

### Object-Capability Model

```
┌─────────────────────────────────────────────────────────────┐
│                   Cap'n Web RPC Session                       │
│                                                              │
│  Client                          Server                      │
│  ┌──────────┐                   ┌──────────┐                │
│  │   stub   │ ◄──── RPC ──────► │  Target  │                │
│  │  (Proxy) │                   │ (Impl)   │                │
│  └──────────┘                   └──────────┘                │
│       │                                │                     │
│       │ Pass by Reference              │                     │
│       │ (stub with proxy)              │                     │
│       ▼                                ▼                     │
│  ┌──────────┐                   ┌──────────┐                │
│  │ Function │ ◄──── RPC ──────► │ Function │                │
│  │  Stub    │                   │  Original│                │
│  └──────────┘                   └──────────┘                │
│                                                              │
│  Capabilities flow through RPC - recipient can only do      │
│  what the original object allows                            │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Concepts

### Pass-by-Value Types

These types are serialized and copied:

```typescript
// Primitives
string, number, boolean, null, undefined, bigint

// Objects
Plain objects (object literals)
Arrays

// Built-ins
Date
Uint8Array
Error and subclasses (Error, TypeError, RangeError, etc.)

// Streams (with automatic flow control)
ReadableStream
WritableStream

// Fetch API
Headers
Request
Response
```

### Pass-by-Reference Types

These types become stubs that call back over RPC:

```typescript
// Classes extending RpcTarget
class MyApi extends RpcTarget {
  myMethod() { return 'called remotely'; }
}

// Functions
function myFunction() { return 'called remotely'; }
```

### RpcTarget

```typescript
import { RpcTarget } from 'capnweb';

// Define server implementation
class MyApiServer extends RpcTarget {
  // Public methods are callable over RPC
  hello(name: string): string {
    return `Hello, ${name}!`;
  }
  
  // Private methods (using #) are NOT callable
  #privateMethod() {
    return 'cannot be called over RPC';
  }
  
  // Own properties are NOT exposed
  internalState = 'not accessible';
  
  // Prototype properties ARE exposed
  get publicProperty() {
    return 'accessible';
  }
}
```

### RpcStub<T>

```typescript
// When you receive an RpcTarget over RPC, you get a stub
let stub: RpcStub<MyApiServer> = await getServerStub();

// Stub appears to have all methods/properties
// But actually proxies to remote object
let result = await stub.hello('World');

// Access properties
let prop = await stub.publicProperty;

// Throws if method/property doesn't exist (on await)
try {
  await stub.nonExistentMethod();
} catch (e) {
  console.error('Method not found');
}
```

### RpcPromise<T>

```typescript
// RpcPromise extends regular Promise
// Can be used with await, .then(), Promise.all(), etc.

// But also supports promise pipelining
let userPromise = api.authenticate(token);

// Use promise in parameters BEFORE awaiting
let profilePromise = api.getUserProfile(userPromise.id);

// Both calls sent in same batch
let [user, profile] = await Promise.all([userPromise, profilePromise]);
```

---

## Installation

### npm/yarn

```bash
npm install capnweb
# or
yarn add capnweb
# or
pnpm add capnweb
```

### CDN

```html
<script type="module">
  import { RpcTarget, newWebSocketRpcSession } from 'https://esm.sh/capnweb';
</script>
```

### Development Setup

```bash
# Clone repository
git clone https://github.com/capnweb/capnweb.git
cd capnweb

# Install dependencies
npm install

# Build
npm run build

# Test
npm test
```

---

## Quick Start

### Basic Example

**Client:**
```typescript
import { newWebSocketRpcSession } from 'capnweb';

// One-line setup
let api = newWebSocketRpcSession('wss://example.com/api');

// Call server method
let result = await api.hello('World');
console.log(result); // "Hello, World!"
```

**Server:**
```typescript
import { RpcTarget, newWorkersRpcResponse } from 'capnweb';

class MyApiServer extends RpcTarget {
  hello(name: string): string {
    return `Hello, ${name}!`;
  }
}

// Cloudflare Workers handler
export default {
  fetch(request, env, ctx) {
    const url = new URL(request.url);
    
    if (url.pathname === '/api') {
      return newWorkersRpcResponse(request, new MyApiServer());
    }
    
    return new Response('Not found', { status: 404 });
  },
};
```

### TypeScript Example with Pipelining

**Shared Types:**
```typescript
// types.ts
interface PublicApi {
  authenticate(apiToken: string): AuthedApi;
  getUserProfile(userId: string): Promise<UserProfile>;
}

interface AuthedApi {
  getUserId(): number;
  getFriendIds(): number[];
}

type UserProfile = {
  name: string;
  photoUrl: string;
};
```

**Server:**
```typescript
// server.ts
import { RpcTarget, newWorkersRpcResponse } from 'capnweb';
import { PublicApi, AuthedApi, UserProfile } from './types';

class ApiServer extends RpcTarget implements PublicApi {
  authenticate(apiToken: string): AuthedApi {
    const userId = validateToken(apiToken);
    return new AuthedApiImpl(userId);
  }
  
  async getUserProfile(userId: string): Promise<UserProfile> {
    return db.users.findById(userId);
  }
}

class AuthedApiImpl extends RpcTarget implements AuthedApi {
  constructor(private userId: number) {}
  
  getUserId(): number {
    return this.userId;
  }
  
  async getFriendIds(): number[] {
    return db.friends.findByUserId(this.userId);
  }
}

export default {
  fetch(request, env, ctx) {
    const url = new URL(request.url);
    if (url.pathname === '/api') {
      return newWorkersRpcResponse(request, new ApiServer());
    }
    return new Response('Not found', { status: 404 });
  },
};
```

**Client (Batch with Pipelining):**
```typescript
// client.ts
import { newHttpBatchRpcSession } from 'capnweb';
import { PublicApi, AuthedApi, UserProfile } from './types';

let api = newHttpBatchRpcSession<PublicApi>('https://example.com/api');

// Authenticate - don't await yet
let authedApi: RpcPromise<AuthedApi> = api.authenticate(apiToken);

// Pipeline: get user ID
let userIdPromise: RpcPromise<number> = authedApi.getUserId();

// Pipeline: get profile using userIdPromise
let profilePromise = api.getUserProfile(userIdPromise);

// Pipeline: get friends and their profiles
let friendsPromise = authedApi.getFriendIds();
let friendProfilesPromise = friendsPromise.map((id: RpcPromise<number>) => {
  return { id, profile: api.getUserProfile(id) };
});

// Send batch and await results
let [profile, friendProfiles] = await Promise.all([
  profilePromise,
  friendProfilesPromise
]);

console.log(`Hello, ${profile.name}!`);
// All calls completed in ONE round trip
```

---

## RPC Basics

### Pass-by-Value

```typescript
class MyApi extends RpcTarget {
  // All these types are serialized and copied
  
  getString(): string {
    return 'hello';
  }
  
  getObject(): { name: string; age: number } {
    return { name: 'Alice', age: 30 };
  }
  
  getArray(): number[] {
    return [1, 2, 3];
  }
  
  getBigInt(): bigint {
    return 12345678901234567890n;
  }
  
  getDate(): Date {
    return new Date();
  }
  
  getBinary(): Uint8Array {
    return new Uint8Array([1, 2, 3, 4, 5]);
  }
  
  getError(): Error {
    return new Error('Something went wrong');
  }
  
  getStream(): ReadableStream {
    return new ReadableStream({
      start(controller) {
        controller.enqueue('chunk 1');
        controller.enqueue('chunk 2');
        controller.close();
      }
    });
  }
}
```

### Pass-by-Reference

```typescript
// Define interface
interface Calculator extends RpcTarget {
  add(a: number, b: number): number;
  multiply(a: number, b: number): number;
}

// Server implementation
class CalculatorImpl extends RpcTarget implements Calculator {
  add(a: number, b: number): number {
    return a + b;
  }
  
  multiply(a: number, b: number): number {
    return a * b;
  }
}

class MathService extends RpcTarget {
  // Return RpcTarget by reference
  getCalculator(): Calculator {
    return new CalculatorImpl();
  }
  
  // Pass function as callback
  computeWithCallback(
    operation: (a: number, b: number) => number
  ): number {
    // Call the callback function over RPC
    return operation(5, 3);
  }
}

// Client usage
const math = newWebSocketRpcSession<MathService>('wss://example.com/math');

// Get calculator stub
const calc = await math.getCalculator();

// Call methods on stub
const sum = await calc.add(2, 3); // 5
const product = await calc.multiply(4, 5); // 20

// Pass callback function
const result = await math.computeWithCallback((a, b) => a + b * 2);
// Function is called back on server
```

### Function Stubs

```typescript
class CallbackService extends RpcTarget {
  // Accept callback from client
  async processWithCallback(
    data: string[],
    processor: (item: string) => Promise<string>
  ): Promise<string[]> {
    // Call client's function for each item
    const results = [];
    for (const item of data) {
      results.push(await processor(item));
    }
    return results;
  }
  
  // Return function to client
  getLogger(): (message: string) => void {
    return (message: string) => {
      console.log('[Server]', message);
    };
  }
}

// Client
const service = newWebSocketRpcSession<CallbackService>('wss://example.com/service');

// Pass callback
const results = await service.processWithCallback(
  ['item1', 'item2'],
  async (item) => item.toUpperCase()
);

// Receive function stub
const logger = await service.getLogger();
await logger('Hello from client'); // Logs on server
```

---

## Promise Pipelining

### Basic Pipelining

```typescript
// Without pipelining (2 round trips)
const user = await api.authenticate(token); // Round trip 1
const profile = await api.getUserProfile(user.id); // Round trip 2

// With pipelining (1 round trip)
const user = api.authenticate(token); // Don't await
const profile = api.getUserProfile(user.id); // Use promise
const [userResult, profileResult] = await Promise.all([user, profile]);
```

### Pipeline Chaining

```typescript
interface Api extends RpcTarget {
  getUser(id: number): Promise<User>;
  getPosts(userId: number): Promise<Post[]>;
  getComments(postId: number): Promise<Comment[]>;
}

// Chain multiple calls
const user = api.getUser(123);
const posts = api.getPosts(user.id);
const firstPost = posts.map(ps => ps[0]);
const comments = api.getComments(firstPost.id);

// All sent in one batch
const [userResult, postsResult, commentsResult] = await Promise.all([
  user,
  posts,
  comments
]);
```

### The map() Method

```typescript
// Get list of IDs
const idsPromise = api.getUserIds();

// Transform on server side (single round trip)
const users = await idsPromise.map(async (id: RpcPromise<number>) => {
  return api.getUser(id);
});

// Map with multiple values
const userProfiles = await idsPromise.map((id: RpcPromise<number>) => {
  return {
    id,
    profile: api.getUserProfile(id),
    posts: api.getUserPosts(id)
  };
});
```

**How map() works:**

1. Callback is executed in "recording mode" with placeholder stub
2. RPC calls during callback are recorded, not executed
3. Recording is sent to server
4. Server replays recording for each result
5. Results are returned in single batch

**Restrictions:**
- Callback must be synchronous (no await)
- Callback must have no side effects except RPC calls
- Only RPC methods can be invoked on promises

---

## Streaming

### ReadableStream

```typescript
// Server
class StreamService extends RpcTarget {
  async *generateData(): AsyncIterable<string> {
    for (let i = 0; i < 100; i++) {
      yield `Chunk ${i}`;
      await new Promise(resolve => setTimeout(resolve, 100));
    }
  }
  
  getStream(): ReadableStream<string> {
    return new ReadableStream({
      async start(controller) {
        for await (const chunk of this.generateData()) {
          controller.enqueue(chunk);
        }
        controller.close();
      }
    });
  }
}

// Client
const service = newWebSocketRpcSession<StreamService>('wss://example.com/stream');

const stream = await service.getStream();
const reader = stream.getReader();

while (true) {
  const { done, value } = await reader.read();
  if (done) break;
  console.log('Received:', value);
}
```

### WritableStream

```typescript
// Server
class UploadService extends RpcTarget {
  getUploadStream(): WritableStream<Uint8Array> {
    return new WritableStream({
      async write(chunk) {
        // Process chunk
        console.log('Received chunk:', chunk.length, 'bytes');
      },
      async close() {
        console.log('Upload complete');
      }
    });
  }
}

// Client
const service = newWebSocketRpcSession<UploadService>('wss://example.com/upload');

const uploadStream = await service.getUploadStream();
const writer = uploadStream.getWriter();

// Write data
const file = await fs.readFile('large-file.bin');
const chunkSize = 1024 * 1024; // 1MB chunks

for (let i = 0; i < file.length; i += chunkSize) {
  const chunk = file.slice(i, i + chunkSize);
  await writer.write(chunk);
}

await writer.close();
```

### Bidirectional Streaming

```typescript
// Server - Echo service with bidirectional streaming
class EchoService extends RpcTarget {
  createEchoStream(): {
    input: WritableStream<string>;
    output: ReadableStream<string>;
  } {
    const transform = new TransformStream<string, string>({
      transform(chunk, controller) {
        controller.enqueue(`Echo: ${chunk}`);
      }
    });
    
    return {
      input: transform.writable,
      output: transform.readable
    };
  }
}

// Client
const service = newWebSocketRpcSession<EchoService>('wss://example.com/echo');

const { input, output } = await service.createEchoStream();
const writer = input.getWriter();
const reader = output.getReader();

// Send and receive
await writer.write('Hello');
const { value } = await reader.read();
console.log(value); // "Echo: Hello"
```

---

## Resource Management

### Explicit Disposal

```typescript
// Using explicit resource management (ES2025)
using stub = newWebSocketRpcSession<Api>('wss://example.com/api');

// Use stub...

// Automatically disposed at end of scope
// Disposer called: stub[Symbol.dispose]()
```

### Disposal Rules

```typescript
// Rule: Caller is responsible for disposing stubs

// Stubs passed in params remain caller's responsibility
const stub = api.getStub();
await api.process(stub); // Caller still owns stub
stub[Symbol.dispose](); // Caller must dispose

// Stubs returned in result transfer to caller
const returnedStub = await api.createStub();
// Caller now owns returnedStub
returnedStub[Symbol.dispose]();
```

### Duplication

```typescript
// Use dup() when stub will be disposed elsewhere
const originalStub = api.getStub();
const duplicate = originalStub.dup();

await api.process(duplicate); // dispose() called on server side
// originalStub still works

// Chain dup() on properties
const userStub = api.getUser(123);
const profileStub = userStub.profile.dup(); // Get stub for profile
```

### onRpcBroken()

```typescript
// Listen for connection errors
stub.onRpcBroken((error: any) => {
  console.error('RPC connection broken:', error);
});

// Promise broken listener
promise.onRpcBroken((error: any) => {
  console.error('Promise rejected or connection lost:', error);
});
```

### Disposal in Practice

```typescript
// HTTP batch - no disposal needed (short-lived session)
using api = newHttpBatchRpcSession<Api>('https://example.com/api');
const result = await api.doSomething();
// Automatically disposed when batch completes

// WebSocket - dispose stubs when done
using api = newWebSocketRpcSession<Api>('wss://example.com/api');

{
  using authedApi = await api.authenticate(token);
  const userId = await authedApi.getUserId();
  // authedApi disposed here, but api connection stays open
}

// Can make new authenticated calls
using newAuthedApi = await api.authenticate(anotherToken);
const anotherUserId = await newAuthedApi.getUserId();
```

---

## Security

### Authentication Pattern

```typescript
// Server - authenticate in-band
class Api extends RpcTarget {
  authenticate(apiToken: string): AuthedApi | never {
    const user = validateToken(apiToken);
    if (!user) {
      throw new Error('Invalid token');
    }
    return new AuthedApiImpl(user);
  }
}

// Client
using api = newWebSocketRpcSession<Api>('wss://example.com/api');

try {
  using authedApi = await api.authenticate(token);
  // Use authenticated API
  const data = await authedApi.getPrivateData();
} catch (error) {
  console.error('Authentication failed:', error);
}
```

### Rate Limiting

```typescript
class RateLimitedApi extends RpcTarget {
  private requestCounts = new Map<string, number>();
  
  async expensiveOperation(): Promise<Result> {
    const clientId = this.getClientId();
    const count = this.requestCounts.get(clientId) || 0;
    
    if (count > 10) {
      throw new Error('Rate limit exceeded');
    }
    
    this.requestCounts.set(clientId, count + 1);
    return performExpensiveOperation();
  }
}
```

### Input Validation

```typescript
import { z } from 'zod';

class ValidatedApi extends RpcTarget {
  const UserSchema = z.object({
    name: z.string().min(1),
    email: z.string().email(),
    age: z.number().min(0)
  });
  
  async createUser(input: unknown): Promise<User> {
    // Runtime type validation
    const validated = UserSchema.parse(input);
    return db.users.create(validated);
  }
}
```

---

## Platform Integration

### Cloudflare Workers

```typescript
// server.ts
import { RpcTarget, newWorkersRpcResponse } from 'capnweb';

class WorkerApi extends RpcTarget {
  // Use Workers Durable Objects
  async getDurableObject(id: string) {
    const stub = this.env.MY_DO.get(id);
    return stub; // Automatically proxied
  }
  
  // Use Service Bindings
  async callService(data: unknown) {
    const response = await this.env.MY_SERVICE.fetch('http://internal/api', {
      method: 'POST',
      body: JSON.stringify(data)
    });
    return response.json();
  }
}

export default {
  fetch(request, env, ctx) {
    const url = new URL(request.url);
    if (url.pathname === '/api') {
      return newWorkersRpcResponse(request, new WorkerApi());
    }
    return new Response('Not found', { status: 404 });
  },
};

export { WorkerApi };
```

### Node.js Server

```typescript
import http from 'node:http';
import { WebSocketServer } from 'ws';
import {
  RpcTarget,
  newWebSocketRpcSession,
  nodeHttpBatchRpcResponse
} from 'capnweb';

class NodeApi extends RpcTarget {
  hello(name: string): string {
    return `Hello, ${name}!`;
  }
}

// HTTP server
const httpServer = http.createServer(async (request, response) => {
  if (request.headers.upgrade?.toLowerCase() === 'websocket') {
    return; // Handled by WebSocketServer
  }
  
  if (request.url === '/api') {
    try {
      await nodeHttpBatchRpcResponse(request, response, new NodeApi(), {
        headers: { 'Access-Control-Allow-Origin': '*' }
      });
    } catch (err) {
      response.writeHead(500);
      response.end(String(err));
    }
    return;
  }
  
  response.writeHead(404);
  response.end('Not Found');
});

// WebSocket server
const wsServer = new WebSocketServer({ server: httpServer });
wsServer.on('connection', (ws) => {
  newWebSocketRpcSession(ws as any, new NodeApi());
});

httpServer.listen(8080);
```

### Deno Server

```typescript
import {
  newHttpBatchRpcResponse,
  newWebSocketRpcSession,
  RpcTarget
} from 'npm:capnweb';

class DenoApi extends RpcTarget {
  hello(name: string): string {
    return `Hello, ${name}!`;
  }
}

Deno.serve(async (req) => {
  const url = new URL(req.url);
  
  if (url.pathname === '/api') {
    if (req.headers.get('upgrade') === 'websocket') {
      const { socket, response } = Deno.upgradeWebSocket(req);
      socket.addEventListener('open', () => {
        newWebSocketRpcSession(socket, new DenoApi());
      });
      return response;
    } else {
      const response = await newHttpBatchRpcResponse(req, new DenoApi());
      response.headers.set('Access-Control-Allow-Origin', '*');
      return response;
    }
  }
  
  return new Response('Not Found', { status: 404 });
});
```

### MessagePort (Web Workers)

```typescript
// Main thread
class Greeter extends RpcTarget {
  greet(name: string): string {
    return `Hello, ${name}!`;
  }
}

// Create channel
const channel = new MessageChannel();

// Server on port1
newMessagePortRpcSession(channel.port1, new Greeter());

// Send port to worker
worker.postMessage({ port: channel.port2 }, [channel.port2]);

// Worker thread
self.onmessage = (event) => {
  const { port } = event.data;
  const api = newMessagePortRpcSession<Greeter>(port);
  
  // Use API
  api.greet('Worker').then(console.log);
};
```

### Custom Transport

```typescript
import { RpcTransport, RpcSession, RpcTarget } from 'capnweb';

class MyTransport implements RpcTransport {
  private messageQueue: string[] = [];
  private receiveResolve: ((value: string) => void) | null = null;
  
  async send(message: string): Promise<void> {
    // Send message (e.g., via postMessage, WebSocket, etc.)
    postMessage({ type: 'rpc', data: message });
  }
  
  async receive(): Promise<string> {
    return new Promise((resolve, reject) => {
      if (this.messageQueue.length > 0) {
        resolve(this.messageQueue.shift()!);
      } else {
        this.receiveResolve = resolve;
      }
    });
  }
  
  abort(reason: any): void {
    console.error('Transport aborted:', reason);
  }
  
  // Call when message received
  onMessage(data: string) {
    if (this.receiveResolve) {
      this.receiveResolve(data);
      this.receiveResolve = null;
    } else {
      this.messageQueue.push(data);
    }
  }
}

// Use custom transport
const transport = new MyTransport();
const localApi = new MyApi();
const session = new RpcSession(transport, localApi);
const remoteStub = session.getRemoteMain();
```

---

## Browser Support

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+
- Node.js 18+
- Deno 1.30+
- Cloudflare Workers (all runtimes)

---

## Related Documents

- [Deep Dive: Promise Pipelining Implementation](./01-promise-pipelining-deep-dive.md)
- [Deep Dive: Streaming and Flow Control](./02-streaming-deep-dive.md)
- [Deep Dive: Resource Management Patterns](./03-resource-management-deep-dive.md)
- [Rust Revision](./rust-revision.md)
- [Production Guide](./production-grade.md)
