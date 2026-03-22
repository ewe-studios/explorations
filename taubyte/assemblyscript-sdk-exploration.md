# Taubyte AssemblyScript SDK - Comprehensive Deep-Dive Exploration

**Date:** 2026-03-22
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/assemblyscript-sdk/`

---

## 1. Purpose and Overview

The **Taubyte AssemblyScript SDK** is a TypeScript-based SDK that enables developers to write WebAssembly modules using AssemblyScript (a TypeScript-like language that compiles to WebAssembly). This SDK provides bindings to the host functions exported by Taubyte nodes through the TVM.

### Key Characteristics

- **Language:** AssemblyScript (TypeScript subset)
- **Package Name:** `sdk`
- **Version:** 1.0.0
- **License:** GPL-3.0
- **Compiler:** AssemblyScript compiler (asc)
- **Runtime:** Minimal WASI shim

---

## 2. Architecture

### 2.1 Module Structure

```
assemblyscript-sdk/
├── event/                  # Event handling
│   ├── base.ts            # Base event class
│   ├── http.ts            # HTTP event handling
│   ├── type.ts            # Event types
│   └── index.ts           # Event exports
├── return/                 # Return type handling
│   └── index.ts           # Return type wrapper
├── helpers/                # Utility helpers
│   ├── index.ts           # Helper exports
│   ├── string.ts          # String utilities
│   └── u32.ts             # UInt32 utilities
├── errno/                  # Error handling
│   └── index.ts           # Error codes
├── package.json           # NPM package config
├── tsconfig.json          # TypeScript config
└── asconfig.json          # AssemblyScript config
```

### 2.2 Build System

```json
{
  "scripts": {
    "build": "asc assembly/index.ts -o build/release.wasm \
              --config ./node_modules/@assemblyscript/wasi-shim/asconfig.json \
              --runtime minimal",
    "asbuild:debug": "asc assembly/index.ts --target debug",
    "asbuild:release": "asc assembly/index.ts --target release"
  }
}
```

### 2.3 Dependencies

```json
{
  "devDependencies": {
    "@assemblyscript/wasi-shim": "^0.1.0",
    "assemblyscript": "^0.25.1",
    "typescript": "^4.9.4"
  }
}
```

---

## 3. Key Types, Interfaces, and APIs

### 3.1 Base Event Class

```typescript
// event/base.ts

export class BaseEvent {
  eventNum: u32;
  eventType: EventType;

  constructor(e: u32) {
    this.eventNum = e;
    this.eventType = new EventType(0)
  }

  /**
   * toString returns the event number as a string
   */
  toString(): string {
    return this.eventNum.toString();
  }

  /**
   * type returns the type of the event
   */
  type(): EventType {
    if (this.eventType.value != 0) {
      return this.eventType
    }

    let f = new U32()
    getEventType(this, f.ptr)
    this.eventType = new EventType(f.load())
    return this.eventType
  }
}

// Host function import
@external("taubyte/sdk", "getEventType")
declare function getEventType(e: BaseEvent, typeId: u32): void
```

### 3.2 Event Class

```typescript
// event/base.ts

export class Event extends BaseEvent {
  /**
   * http returns an HttpEvent if the event is an http event
   */
  http(): Return<HttpEvent> {
    if (this.type().isHttp()) {
      return new Return(new HttpEvent(this.eventNum), null)
    }
    return new Return(new HttpEvent(0), "not an http event")
  }
}
```

### 3.3 HTTP Event

```typescript
// event/http.ts

export class HttpEvent extends BaseEvent {
  /**
   * method returns the HTTP method
   */
  method(): string {
    // Host function call
    return getHttpMethod(this.eventNum)
  }

  /**
   * path returns the request path
   */
  path(): string {
    return getHttpPath(this.eventNum)
  }

  /**
   * query returns a query parameter value
   */
  query(key: string): string {
    return getHttpQuery(this.eventNum, key)
  }

  /**
   * header returns a header value
   */
  header(key: string): string {
    return getHttpHeader(this.eventNum, key)
  }

  /**
   * body returns the request body
   */
  body(): Uint8Array {
    return getHttpBody(this.eventNum)
  }

  /**
   * write writes data to the response
   */
  write(data: Uint8Array): void {
    writeHttpBody(this.eventNum, data)
  }

  /**
   * return sends the response
   */
  return(status: u32, body: Uint8Array): void {
    httpReturn(this.eventNum, status, body)
  }
}

// Host function imports
@external("taubyte/sdk", "getHttpMethod")
declare function getHttpMethod(eventId: u32): string

@external("taubyte/sdk", "getHttpPath")
declare function getHttpPath(eventId: u32): string

@external("taubyte/sdk", "getHttpQuery")
declare function getHttpQuery(eventId: u32, key: string): string

@external("taubyte/sdk", "getHttpHeader")
declare function getHttpHeader(eventId: u32, key: string): string

@external("taubyte/sdk", "getHttpBody")
declare function getHttpBody(eventId: u32): Uint8Array

@external("taubyte/sdk", "writeHttpBody")
declare function writeHttpBody(eventId: u32, data: Uint8Array): void

@external("taubyte/sdk", "httpReturn")
declare function httpReturn(eventId: u32, status: u32, body: Uint8Array): void
```

### 3.4 Event Type

```typescript
// event/type.ts

export class EventType {
  value: u32;

  constructor(value: u32) {
    this.value = value;
  }

  isHttp(): bool {
    return this.value == 1;
  }

  isPubSub(): bool {
    return this.value == 2;
  }

  isCron(): bool {
    return this.value == 3;
  }
}
```

### 3.5 Return Type Wrapper

```typescript
// return/index.ts

export class Return<T> {
  value: T | null;
  error: string | null;

  constructor(value: T, error: string | null) {
    this.value = value;
    this.error = error;
  }

  isOk(): bool {
    return this.error == null;
  }

  isError(): bool {
    return this.error != null;
  }

  unwrap(): T {
    if (this.error != null) {
      throw new Error(this.error!)
    }
    return this.value!
  }
}
```

### 3.6 Helpers

#### U32 Helper

```typescript
// helpers/u32.ts

@external("taubyte/sdk", "storeU32")
declare function storeU32(ptr: u32, value: u32): void

@external("taubyte/sdk", "loadU32")
declare function loadU32(ptr: u32): u32

export class U32 {
  ptr: u32;

  constructor() {
    this.ptr = allocateU32()
  }

  store(value: u32): void {
    storeU32(this.ptr, value)
  }

  load(): u32 {
    return loadU32(this.ptr)
  }
}
```

#### String Helper

```typescript
// helpers/string.ts

@external("taubyte/sdk", "storeString")
declare function storeString(ptr: u32, str: string): void

@external("taubyte/sdk", "loadString")
declare function loadString(ptr: u32): string

export class StringRef {
  ptr: u32;

  constructor() {
    this.ptr = allocateStringRef()
  }

  store(str: string): void {
    storeString(this.ptr, str)
  }

  load(): string {
    return loadString(this.ptr)
  }
}
```

### 3.7 Errno

```typescript
// errno/index.ts

export enum Errno {
  ErrorNone = 0,
  ErrorEventNotFound = 1,
  ErrorBufferTooSmall = 2,
  ErrorAddressOutOfMemory = 3,
  ErrorHttpWrite = 4,
  // ... additional error codes
}

export function errnoMessage(code: Errno): string {
  switch (code) {
    case Errno.ErrorNone:
      return "Success"
    case Errno.ErrorEventNotFound:
      return "Event not found"
    case Errno.ErrorBufferTooSmall:
      return "Buffer too small"
    default:
      return "Unknown error"
  }
}
```

---

## 4. Integration with Taubyte Components

### 4.1 VM Integration

The AssemblyScript SDK compiles to WebAssembly and interfaces with TVM:

```
AssemblyScript Source
    ├── TypeScript Syntax
    ├── AssemblyScript Compiler (asc)
    │   └── WebAssembly Output (.wasm)
    │       └── TVM (wazero runtime)
    │           └── Host Function Imports (taubyte/sdk)
```

### 4.2 Host Function Pattern

```typescript
// External host function declaration
@external("taubyte/sdk", "functionName")
declare function functionName(param1: u32, param2: string): u32

// Usage in SDK methods
export function someOperation(param: string): string {
  let resultPtr = allocateStringResult()
  let err = functionName(resultPtr, param)
  if (err != 0) {
    throw new Error(errnoMessage(err as Errno))
  }
  return loadString(resultPtr)
}
```

### 4.3 Cross-SDK Compatibility

| Feature | Go SDK | Rust SDK | AssemblyScript SDK |
|---------|--------|----------|-------------------|
| Event Handling | ✓ | ✓ | ✓ |
| HTTP Server | ✓ | ✓ | ✓ |
| Database | ✓ | ✓ | Planned |
| Storage | ✓ | ✓ | Planned |
| PubSub | ✓ | ✓ | Planned |

---

## 5. Production Usage Patterns

### 5.1 HTTP Handler Example

```typescript
// assembly/index.ts

import { Event, HttpEvent } from "sdk/event";
import { Return } from "sdk/return";

// Exported handler function
export function handler(eventId: u32): void {
  let event = new Event(eventId);

  // Check event type
  if (!event.type().isHttp()) {
    return;
  }

  // Cast to HTTP event
  let httpResult = event.http();

  if (httpResult.isError()) {
    return;
  }

  let httpEvent = httpResult.value!;

  // Route handling
  let path = httpEvent.path();

  if (path == "/api/hello") {
    handleHello(httpEvent);
  } else if (path == "/api/echo") {
    handleEcho(httpEvent);
  } else {
    handle404(httpEvent);
  }
}

function handleHello(e: HttpEvent): void {
  let method = e.method();

  if (method == "GET") {
    e.write(String.UTF8.encode("Hello, Taubyte!"));
    e.return(200, String.UTF8.encode("OK"));
  } else {
    e.return(405, String.UTF8.encode("Method not allowed"));
  }
}

function handleEcho(e: HttpEvent): void {
  if (e.method() == "POST") {
    let body = e.body();
    e.write(body);
    e.return(200, String.UTF8.encode("OK"));
  } else {
    e.return(405, String.UTF8.encode("Method not allowed"));
  }
}

function handle404(e: HttpEvent): void {
  e.return(404, String.UTF8.encode("Not found"));
}
```

### 5.2 Query Parameter Handling

```typescript
import { HttpEvent } from "sdk/event";

export function handler(eventId: u32): void {
  let event = new Event(eventId);
  let httpEvent = event.http().unwrap();

  // Get query parameters
  let name = httpEvent.query("name");
  let count = httpEvent.query("count");

  // Build response
  let response = `Hello ${name}, count: ${count}`;

  httpEvent.write(String.UTF8.encode(response));
  httpEvent.return(200, String.UTF8.encode("OK"));
}
```

### 5.3 Header Handling

```typescript
export function handler(eventId: u32): void {
  let httpEvent = new Event(eventId).http().unwrap();

  // Read headers
  let contentType = httpEvent.header("Content-Type");
  let auth = httpEvent.header("Authorization");

  // Set response headers (via host function)
  setHeader("X-Custom-Header", "Taubyte");

  httpEvent.return(200, String.UTF8.encode("OK"));
}
```

---

## 6. Memory Management

### 6.1 AssemblyScript Memory Model

AssemblyScript uses a linear memory model:

```
Memory Layout:
├── Static data (strings, constants)
├── Heap (dynamic allocations)
└── Stack (local variables)
```

### 6.2 String Handling

```typescript
// Strings are UTF-8 encoded
let str: string = "Hello";

// Convert to bytes
let bytes: Uint8Array = String.UTF8.encode(str);

// Convert from bytes
let decoded: string = String.UTF8.decode(bytes);

// Memory allocation
let ptr: u32 = __new(str.length * 2, STRING_ID);
```

### 6.3 Pointer Passing to Host

```typescript
// Allocate memory for result
let resultPtr: u32 = __new(4, ARRAYBUFFER_ID);

// Pass pointer to host function
let err = hostFunction(resultPtr);

// Read result
let result: u32 = load<u32>(resultPtr);

// Free memory
__free(resultPtr);
```

---

## 7. Building and Testing

### 7.1 Build Commands

```bash
# Install dependencies
npm install

# Debug build
npm run asbuild:debug

# Release build (optimized)
npm run asbuild:release

# Full build (debug + release)
npm run asbuild

# Build with custom options
asc assembly/index.ts \
  --target release \
  --optimize \
  --shrink-level 2 \
  --runtime minimal \
  -o build/module.wasm
```

### 7.2 Build Output

```
build/
├── release.wasm      # Optimized WASM module
├── release.js        # JS loader (optional)
└── release.d.ts      # TypeScript definitions
```

### 7.3 Testing

```bash
# Run tests
npm test

# Test inside TVM
tau test ./assembly/__tests__/*.test.ts
```

---

## 8. Limitations and Considerations

### 8.1 AssemblyScript Limitations

1. **TypeScript Subset:** Not all TypeScript features are supported
2. **No Generics in Exports:** Exported functions can't use generics
3. **Memory Management:** Manual memory management for host interop
4. **No Closures in Host Calls:** Closures can't be passed to host

### 8.2 Performance Considerations

1. **Runtime Choice:** Use `--runtime minimal` for smallest output
2. **Optimization Levels:** `-O` flags for size/speed tradeoffs
3. **String Operations:** Minimize string allocations
4. **Array Handling:** Use typed arrays for binary data

### 8.3 Best Practices

1. **Use Release Builds:** Always deploy optimized builds
2. **Minimize Allocations:** Reuse buffers where possible
3. **Error Handling:** Check all return values
4. **Type Safety:** Use strict TypeScript types

---

## 9. Related Components

| Component | Path | Description |
|-----------|------|-------------|
| Rust SDK | `../rust-sdk/` | Rust SDK |
| Go SDK | `../go-sdk/` | Go SDK |
| VM | `../vm/` | WebAssembly runtime |
| go-sdk-symbols | `../go-sdk-symbols/` | Symbol definitions |

---

## 10. Maintainers

- Sam Stoltenberg (@skelouse)
- Tafseer Khan (@tafseer-khan)
- Samy Fodil (@samyfodil)

---

## 11. Documentation References

- **Official Docs:** https://tau.how
- **AssemblyScript Docs:** https://www.assemblyscript.org
- **TypeScript Docs:** https://www.typescriptlang.org

---

*This document was generated as part of a comprehensive Taubyte codebase exploration.*
