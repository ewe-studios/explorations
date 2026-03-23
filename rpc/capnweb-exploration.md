# Cap'n Web Exploration

location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RPC/capnweb
repository: https://github.com/cloudflare/capnweb
explored_at: 2026-03-23

## Overview

Cap'n Web is a JavaScript-native RPC system created by Kenton Varda (same author as Cap'n Proto). It's designed as a "spiritual sibling" to Cap'n Proto but built for the web stack. Unlike Cap'n Proto, it uses JSON-based serialization and requires no schemas or code generation.

## Key Characteristics

- **No schemas**: Works with plain TypeScript types
- **JSON-based**: Human-readable serialization
- **Object-capability**: Same capability security model as Cap'n Proto
- **Promise pipelining**: Chain calls in single round trip
- **Bidirectional**: Client and server can call each other
- **Under 10kB**: Minified + gzipped bundle size

## Project Structure

```
capnweb/
├── src/
│   ├── core.ts           # Core RPC session management
│   ├── rpc.ts            # RPC protocol implementation
│   ├── serialize.ts      # JSON serialization with extensions
│   ├── batch.ts          # HTTP batch request handling
│   ├── websocket.ts      # WebSocket transport
│   ├── messageport.ts    # postMessage() transport
│   ├── map.ts            # Remote .map() implementation
│   ├── types.d.ts        # TypeScript type definitions
│   └── index.ts          # Public API exports
├── examples/
│   ├── batch-pipelining/ # Batch request demo
│   └── worker-react/     # React + Worker example
├── __tests__/
│   ├── index.test.ts     # Main test suite
│   └── workerd.test.ts   # Cloudflare Workers tests
├── protocol.md           # Protocol specification
└── package.json
```

## Architecture

### Core Components

```typescript
// Main classes
export class RpcTarget { }           // Base class for exported objects
export class RpcStub<T> { }          // Proxy for remote objects
export class RpcPromise<T> { }       // Promise with pipelining
export class RpcSession<T> { }       // RPC connection manager
```

### Session Types

```typescript
// HTTP Batch (one-shot requests)
newHttpBatchRpcSession<T>(url: string): RpcStub<T>

// WebSocket (persistent connection)
newWebSocketRpcSession<T>(url: string): Disposable & RpcStub<T>

// MessagePort (workers, iframes)
newMessagePortRpcSession<T>(port: MessagePort): RpcStub<T>
```

## Protocol Specification

### Message Format

Cap'n Web uses JSON with preprocessing for non-JSON types:

```json
// Basic types pass through
{ "name": "Alice", "age": 30 }

// Arrays must be wrapped (escape mechanism)
[["just", "an", "array"]]

// Special types use type codes
["date", 1749342170815]           // Date
["bigint", "123456789012345"]     // BigInt
["bytes", "base64data"]           // Uint8Array
["inf"]                           // Infinity
["nan"]                           // NaN
["undefined"]                     // undefined
["error", "TypeError", "message"] // Error
```

### Top-Level RPC Messages

| Message | Format | Description |
|---------|--------|-------------|
| push | `["push", expression]` | Call method on remote |
| pull | `["pull", importId]` | Request result of call |
| resolve | `["resolve", exportId, expression]` | Return result |
| reject | `["reject", exportId, expression]` | Return error |
| release | `["release", importId, refcount]` | Release capability |
| abort | `["abort", expression]` | Session error |

### Import/Export Tables

```
┌─────────────────────────────────────────────────────────┐
│                    RPC Session                           │
│  ┌──────────────────┐  ┌──────────────────┐            │
│  │    Imports       │  │    Exports       │            │
│  │  (remote stubs)  │  │  (local objects) │            │
│  │                  │  │                  │            │
│  │  ID 1: Stub A    │  │  ID -1: Obj X    │            │
│  │  ID 2: Stub B    │  │  ID -2: Obj Y    │            │
│  └──────────────────┘  └──────────────────┘            │
└─────────────────────────────────────────────────────────┘
```

ID allocation:
- **Positive IDs (1, 2, 3...)**: Assigned by importer
- **Negative IDs (-1, -2, -3...)**: Assigned by exporter
- **ID 0**: Main interface

## Serialization System

### Devaluator (Serialize)

```typescript
export class Devaluator {
  static devaluate(
    value: unknown,
    exporter?: Exporter
  ): unknown;
}
```

Handles type conversion:

```typescript
// Date -> ["date", timestamp]
devaluate(new Date(2024, 0, 1))
  // → ["date", 1704067200000]

// BigInt -> ["bigint", string]
devaluate(123n)
  // → ["bigint", "123"]

// Uint8Array -> ["bytes", base64]
devaluate(new Uint8Array([1, 2, 3]))
  // → ["bytes", "AQID"]

// RpcTarget -> ["export", exportId]
devaluate(myRpcTarget)
  // → ["export", -1]
```

### Evaluator (Deserialize)

```typescript
export class Evaluator {
  static evaluate(
    value: unknown,
    importer?: Importer
  ): unknown;
}
```

Reconstructs values:

```typescript
// ["date", timestamp] -> Date
evaluate(["date", 1704067200000])
  // → new Date(1704067200000)

// ["bigint", string] -> BigInt
evaluate(["bigint", "123"])
  // → 123n

// ["import", importId] -> RpcStub
evaluate(["import", 1])
  // → RpcStub proxy
```

### Expression Evaluation

Expressions are evaluated with special handling:

```typescript
// Import expression
["import", importId, propertyPath, callArguments]

// Pipeline expression (promise)
["pipeline", importId, propertyPath, callArguments]

// Export expression
["export", exportId]

// Promise expression
["promise", exportId]
```

## Promise Pipelining

### Basic Pipelining

```typescript
// Traditional (2 round trips)
const user = await api.authenticate(token);
const profile = await api.getUserProfile(user.id);

// Pipelined (1 round trip)
const userPromise = api.authenticate(token);
const profile = await api.getUserProfile(userPromise.id);
// userPromise.id creates pipeline expression
```

### How It Works

```
Client                              Server
   │                                  │
   │── push: authenticate() ─────────>│  → Returns AuthedApi stub
   │   (importId=1)                   │     (exportId=-1)
   │                                  │
   │── push: getUserProfile(          >│  Uses pipeline expression
   │      pipeline(1, ["id"]))        │  Resolves on server
   │   (importId=2)                   │
   │                                  │
   │<─ resolve: -1, AuthedApi ────────│
   │<─ resolve: -2, UserProfile ──────│
   │                                  │
```

## The Magic .map() Method

### Overview

The `.map()` method enables remote transformation without pulling data:

```typescript
// Get list of user IDs
const ids = await api.listUserIds();

// Transform each ID to name in single round trip
const names = await ids.map(id => api.getUserName(id));
```

### Record-Replay Implementation

```typescript
class RpcPromise<T> extends Promise<T> {
  map<U>(mapper: (value: T) => U): RpcPromise<U> {
    // 1. Run mapper in "recording mode"
    const recording = recordExecution(() => {
      return mapper(placeholderStub);
    });

    // 2. Send recording to server
    return this.session.sendMap(this, recording);
  }
}
```

### Recording Mode

```typescript
// Mapper captures stubs and operations
promise.map(id => {
  return {
    id,                    // Input value
    name: api.getName(id), // Captures api stub
  };
});

// Generates instructions:
{
  captures: [["import", 1]],  // api stub
  instructions: [
    { op: "getProperty", import: 0, prop: "id" },
    { op: "call", import: -1, method: "getName", args: [1] },
    { op: "createObject", props: { id: 1, name: 2 } },
  ]
}
```

### Instruction Types

```typescript
interface Instruction {
  op: "getProperty" | "call" | "createObject" | "createArray";
  import?: number;    // Reference to captures or input
  prop?: string;
  method?: string;
  args?: number[];    // Instruction indices
}
```

## Resource Management

### Disposal Pattern

```typescript
// Using explicit resource management
using stub = newWebSocketRpcSession<Api>("wss://example.com");

// Automatic disposal at scope end
// stub[Symbol.dispose]() called automatically
```

### Disposal Rules

1. **Caller disposes**: Caller responsible for disposing stubs they receive
2. **Duplication**: Stubs duplicated when passed over RPC
3. **Implicit disposal**: Callee's duplicates disposed when call completes

### Duplication

```typescript
// Keep stub after passing somewhere it will be disposed
const dupStub = stub.dup();
passSomewhere(stub);  // Original gets disposed
// dupStub still works
```

### onRpcBroken()

```typescript
stub.onRpcBroken((error: Error) => {
  console.error("Connection lost:", error);
});
```

Called when:
- Connection is lost
- Promise rejects
- Stub becomes invalid

## Transport Implementations

### HTTP Batch

```typescript
// Client
const api = newHttpBatchRpcSession<Api>("https://api.example.com");

// Add calls to batch
const p1 = api.method1();
const p2 = api.method2();

// Send batch when awaiting
const [r1, r2] = await Promise.all([p1, p2]);
```

**Batch Request:**
```json
POST /api
Content-Type: application/json

{
  "expressions": [
    ["push", ["import", 0, [], ["method1"]]],
    ["push", ["import", 0, [], ["method2"]]]
  ]
}
```

### WebSocket

```typescript
// Persistent bidirectional connection
using api = newWebSocketRpcSession<Api>("wss://api.example.com");

// Calls sent immediately
const result = await api.method1();

// Server can call back
```

### MessagePort

```typescript
// Create channel
const channel = new MessageChannel();

// Server on port1
newMessagePortRpcSession(channel.port1, new ApiImpl());

// Client on port2
using api = newMessagePortRpcSession(channel.port2);
```

## Cloudflare Workers Integration

### Built-in RPC Compatibility

Cap'n Web is compatible with Cloudflare Workers' native RPC:

```typescript
// RpcTarget is same as Workers' builtin
import { RpcTarget } from "capnweb";
// Same as Cloudflare's RpcTarget

// Stubs can be passed between systems
const capnStub = newWebSocketRpcSession(...);
await worker.binding.call(capnStub);  // Works!
```

### Helper Functions

```typescript
// Workers HTTP handler
export default {
  fetch(request: Request, env: Env) {
    return newWorkersRpcResponse(request, new ApiImpl());
  }
}
```

### Compatibility Flags

```toml
# wrangler.toml
compatibility_date = "2026-01-20"
compatibility_flags = ["rpc_params_dup_stubs"]
```

## Security Considerations

### Authentication Pattern

```typescript
// Authenticate in-band
interface Api {
  authenticate(token: string): AuthedApi;
}

interface AuthedApi {
  getUserData(): UserData;
}

// Client
const api = newHttpBatchRpcSession("https://api.example.com");
const authed = await api.authenticate(token);
const data = await authed.getUserData();
```

**Why not cookies?**
- WebSocket API doesn't allow custom headers
- Cookies vulnerable to CSRF
- In-band auth is explicit

### Rate Limiting

```typescript
class ApiImpl extends RpcTarget {
  async expensiveOperation() {
    // Implement rate limiting
    if (await this.rateLimiter.isLimited()) {
      throw new Error("Rate limited");
    }
    // ...
  }
}
```

### Type Safety Warning

```typescript
// TypeScript types are NOT enforced at runtime!
interface Request {
  amount: number;
}

// Malicious client can send:
{ amount: "not a number" }

// Use runtime validation
class ApiImpl extends RpcTarget {
  process(request: Request) {
    if (typeof request.amount !== "number") {
      throw new TypeError("Invalid amount");
    }
  }
}
```

## Usage Examples

### Basic Example

**Server:**
```typescript
import { RpcTarget, newWorkersRpcResponse } from "capnweb";

class Greeter extends RpcTarget {
  greet(name: string): string {
    return `Hello, ${name}!`;
  }
}

export default {
  fetch(request: Request) {
    return newWorkersRpcResponse(request, new Greeter());
  }
}
```

**Client:**
```typescript
import { newHttpBatchRpcSession } from "capnweb";

interface Greeter {
  greet(name: string): string;
}

const api = newHttpBatchRpcSession<Greeter>("https://example.com");
const greeting = await api.greet("World");
console.log(greeting);  // "Hello, World!"
```

### Complex Example with Pipelining

```typescript
interface PublicApi {
  authenticate(token: string): AuthedApi;
  getUserProfile(userId: string): UserProfile;
}

interface AuthedApi {
  getUserId(): string;
  getFriendIds(): string[];
}

// Client
const api = newHttpBatchRpcSession<PublicApi>("https://api.example.com");

// Pipeline: authenticate -> getUserId -> getProfile
const authed = api.authenticate(token);
const userId = authed.getUserId();
const profile = api.getUserProfile(userId);

// Pipeline through array
const friends = authed.getFriendIds();
const friendProfiles = friends.map(id =>
  api.getUserProfile(id)
);

// All in single round trip!
const [profileData, friendData] = await Promise.all([
  profile,
  friendProfiles
]);
```

## Testing

### Test Structure

```typescript
// Unit tests
import { describe, it, expect } from "vitest";
import { RpcTarget, RpcStub } from "../src/core.js";

describe("RpcTarget", () => {
  it("should export methods", async () => {
    class TestTarget extends RpcTarget {
      testMethod() { return 42; }
    }

    const stub = new RpcStub(new TestTarget());
    expect(await stub.testMethod()).toBe(42);
  });
});
```

### Integration Tests

```typescript
// Test with actual network
import { createServer } from "http";
import { WebSocketServer } from "ws";

describe("WebSocket RPC", () => {
  it("should handle bidirectional calls", async () => {
    // Setup server and client
    // Test RPC calls
  });
});
```

## Performance Characteristics

### Advantages

- **Small bundle size**: < 10kB gzipped
- **No code generation**: Direct TypeScript usage
- **JSON optimization**: Browser-native parsing
- **Pipelining**: Reduces round trips

### Limitations

- **JSON overhead**: Larger than binary protocols
- **No zero-copy**: Full deserialization required
- **Record-replay overhead**: .map() has memory cost

## Comparison with Cap'n Proto

| Feature | Cap'n Web | Cap'n Proto |
|---------|-----------|-------------|
| Serialization | JSON | Binary |
| Schema | TypeScript types | .capnp files |
| Zero-copy | No | Yes |
| Code generation | No | Yes |
| Browser support | Native | Requires WASM |
| Bundle size | < 10kB | Larger |
| Performance | Good | Excellent |

## Resources

- [npm package](https://www.npmjs.com/package/capnweb)
- [Protocol specification](./protocol.md)
- [Cloudflare RPC blog](https://blog.cloudflare.com/javascript-native-rpc/)
