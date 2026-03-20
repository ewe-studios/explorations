---
location: /home/darkvoid/Boxxed/@formulas/src.web-std
repository: N/A (no git remote)
explored_at: 2026-03-20T00:00:00Z
language: JavaScript, TypeScript
---

# Project Exploration: Web Standards Utilities

## Overview

This directory contains web standard utilities that provide Web API polyfills for Node.js environments. The project enables developers to write runtime-agnostic code by implementing browser-standard APIs like `File`, `Blob`, `fetch`, `Headers`, `Request`, `Response`, and file URL utilities that work consistently across Node.js and browser environments.

The codebase is organized into four main sub-projects:

1. **file/** - Web File API polyfill (`web-file-polyfill`)
2. **file-url/** - File path to `file://` URL converter (`@web-std/file-url`)
3. **io/** - Monorepo containing Blob, fetch, FormData, and Stream implementations
4. **node-fetch/** - Fetch API implementation for Node.js (`node-fetch` v3 beta)

## Directory Structure

```
/home/darkvoid/Boxxed/@formulas/src.web-std/
├── file/                    # Web File API implementation
│   ├── src/
│   │   ├── lib.js          # File class (extends Blob)
│   │   └── package.js      # Blob dependency re-export
│   ├── test/
│   ├── package.json        # web-file-polyfill
│   ├── Readme.md
│   └── rollup.config.js
│
├── file-url/               # File path -> file:// URL utility
│   ├── src/
│   │   ├── lib.js          # Re-exports from url.js
│   │   └── url.js          # FileURL class and fromPath()
│   ├── test/
│   ├── package.json        # @web-std/file-url
│   └── Readme.md
│
├── io/                     # Monorepo with multiple packages
│   ├── packages/
│   │   ├── blob/           # Web Blob API implementation
│   │   ├── fetch/          # Web fetch API
│   │   ├── file/           # File implementation
│   │   ├── form-data/      # FormData implementation
│   │   └── stream/         # ReadableStream polyfill
│   └── package.json        # Workspace root
│
└── node-fetch/             # Node.js fetch implementation
    ├── src/
    │   ├── index.js        # Main fetch() function
    │   ├── body.js         # Body mixin class
    │   ├── headers.js      # Headers class
    │   ├── request.js      # Request class
    │   ├── response.js     # Response class
    │   ├── errors/         # FetchError, AbortError
    │   └── utils/          # Helpers (is.js, utf8.js, form-data.js)
    ├── @types/             # TypeScript definitions
    ├── test/
    ├── docs/
    └── package.json        # node-fetch v3.0.0-beta.9
```

## Architecture

### High-Level Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    Web Standards Utilities                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
│  │   file/     │    │  file-url/  │    │       io/           │  │
│  │  File API   │    │ Path->URL   │    │  ┌──────────────┐   │  │
│  │             │    │  Converter  │    │  │    blob/     │   │  │
│  │  ┌───────┐  │    │             │    │  │  Blob API    │   │  │
│  │  │ File  │──┼────│  fromPath() │    │  └──────────────┘   │  │
│  │  │ Blob  │  │    │  FileURL    │    │  ┌──────────────┐   │  │
│  │  └───┬───┘  │    └─────────────┘    │  │    fetch/    │   │  │
│  │      │      │                       │  │  Fetch API   │   │  │
│  │      ▼      │    ┌─────────────┐    │  └──────────────┘   │  │
│  │  ┌───────┐  │    │ node-fetch/ │    │  ┌──────────────┐   │  │
│  │  │ Blob  │◄─┼────│   (uses)    │◄───┼──│   stream/    │   │  │
│  │  └───────┘  │    │             │    │  │ ReadableStream│  │  │
│  └─────────────┘    │  ┌───────┐  │    │  └──────────────┘   │  │
│                     │  │ fetch │  │    └─────────────────────┘  │
│                     │  │Request│  │                              │
│                     │  │Response│ │                              │
│                     │  │Headers │ │                              │
│                     │  │ Body  │ │                              │
│                     │  └───┬───┘ │                              │
│                     └──────┼──────┘                              │
│                            │                                      │
│                            ▼                                      │
│                   ┌────────────────┐                              │
│                   │   Node.js      │                              │
│                   │   http/https   │                              │
│                   │   zlib         │                              │
│                   │   stream       │                              │
│                   └────────────────┘                              │
└─────────────────────────────────────────────────────────────────┘
```

## File API

**Location:** `/home/darkvoid/Boxxed/@formulas/src.web-std/file/`

### Implementation

The File API is implemented in `src/lib.js` as a `File` class that extends `Blob`:

```javascript
const WebFile = class File extends Blob {
  constructor(init, name, options = {}) {
    super(init, options)
    this._name = String(name).replace(/\//g, ":")  // Per File API spec
    this._lastModified = options.lastModified || Date.now()
  }

  get name() { return this._name }
  get webkitRelativePath() { return "" }
  get lastModified() { return this._lastModified }
}
```

### Key Features

- **Extends Blob**: Inherits all Blob functionality (size, type, slice(), arrayBuffer(), text(), stream())
- **Name Sanitization**: Forward slashes in filenames are replaced with colons per the [File API spec](https://w3c.github.io/FileAPI/)
- **lastModified**: Timestamp in milliseconds since UNIX epoch
- **webkitRelativePath**: Returns empty string (for compatibility)

### Usage Pattern

```javascript
import { File, Blob } from "web-file-polyfill"
const file = new File(["hello", new TextEncoder().encode("world")], "hello.txt", {
  type: "text/plain",
  lastModified: Date.now()
})
```

### Note

This package is **deprecated** in favor of `@web-std/file`.

## File URL API

**Location:** `/home/darkvoid/Boxxed/@formulas/src.web-std/file-url/`

### Implementation

A universal library for converting file system paths to `file://` URLs without depending on Node.js built-ins.

**Core exports from `src/url.js`:**

```javascript
class FileURL extends URL {
  get protocol() { return "file:" }
}

export const fromPath = path => { /* converts path to file:// URL */ }
export const tryFromPath = path => { /* returns null instead of throwing */ }
```

### Path Handling

The implementation handles multiple path formats:

| Input Path | Output URL |
|------------|------------|
| `/Users/file.txt` | `file:///Users/file.txt` |
| `C:\file.txt` | `file:///C:/file.txt` |
| `\\server\share\file.txt` | `file://server/share/file.txt` |

### Key Features

- **OS Agnostic**: Creates correct URLs regardless of host OS
- **Path Normalization**: Automatically normalizes `..` and `.` segments
- **URL Resolution**: Works with standard `new URL(relative, base)` for path resolution
- **No Dependencies**: Works in both Node.js and browser environments

### Usage Patterns

```javascript
import { fromPath, tryFromPath } from "@web-std/file-url"

// Convert paths
fromPath("/Users/project/Readme.md").href
//> 'file:///Users/project/Readme.md'

// Path resolution (alternative to path.resolve)
new URL("baz/asdf", fromPath("/foo/bar/")).href
//> 'file:///foo/bar/baz/asdf'

// Normalization
fromPath("/foo/bar/.././file.md").href
//> 'file:///foo/file.md'

// Safe conversion
const url = tryFromPath(invalidPath) || fallbackUrl
```

## I/O Utilities

**Location:** `/home/darkvoid/Boxxed/@formulas/src.web-std/io/`

A monorepo containing multiple Web API implementations.

### Blob Package (`packages/blob/`)

**Implementation:** `src/blob.js` and `src/blob.node.js`

The Blob implementation is Web API compatible:

```javascript
class WebBlob {
  constructor(init = [], options = {}) {
    // Normalizes all inputs to Uint8Array chunks
    this._parts = []  // Array of Uint8Array
    this._size = total bytes
    this._type = MIME type (lowercase, ASCII only)
  }

  get type() { return this._type }
  get size() { return this._size }
  slice(start, end, type)  // Returns new Blob with byte range
  arrayBuffer()            // Returns Promise<ArrayBuffer>
  text()                   // Returns Promise<string>
  stream()                 // Returns ReadableStream<Uint8Array>
}
```

#### Node.js Compatibility Layer

The `blob.node.js` file provides a smart fallback mechanism:

```javascript
export const Blob = use()  // Returns native Node.js Blob if available and bug-free

const use = () => {
  try {
    const { Blob } = builtin  // Node.js buffer module
    // Check for known bug: https://github.com/nodejs/node/issues/40705
    const isBugFixed = new Blob([view]).size === view.byteLength
    return isBugFixed ? Blob : null
  } catch (error) {
    return null
  }
}
```

**Design Decisions:**

1. **Uint8Array-based**: Unlike `fetch-blob` which uses Node Buffers, this uses standard `Uint8Array`
2. **ReadableStream**: Uses web `ReadableStream` (from `@web-std/stream`) instead of Node streams for spec compliance
3. **No WeakMap**: Uses `_` prefixed private properties for better performance

### Stream Package (`packages/stream/`)

**Implementation:** `src/lib.js`

Simply re-exports from `globalThis`:

```javascript
export const {
  ReadableStream,
  ReadableStreamDefaultReader,
  TransformStream,
  WritableStream,
  ByteLengthQueuingStrategy,
  CountQueuingStrategy,
  TextEncoderStream,
  TextDecoderStream,
} = globalThis
```

This assumes the runtime (Node.js 16.5+ or modern browser) has native Streams support. For older environments, a polyfill would be needed.

### Fetch Package (`packages/fetch/`)

The fetch implementation follows the same pattern as `node-fetch/` but is organized as part of the `@web-std/io` monorepo.

## Node Fetch

**Location:** `/home/darkvoid/Boxxed/@formulas/src.web-std/node-fetch/`

A light-weight module that brings the Fetch API to Node.js.

### Architecture

```
src/
├── index.js       # Main fetch() function
├── body.js        # Body mixin (shared by Request/Response)
├── headers.js     # Headers class
├── request.js     # Request class
├── response.js    # Response class
├── errors/
│   ├── base.js           # FetchBaseError
│   ├── fetch-error.js    # FetchError (network errors)
│   └── abort-error.js    # AbortError
└── utils/
    ├── is.js             # Type guards (isBlob, isFormData, etc.)
    ├── is-redirect.js    # Redirect status checker
    ├── get-search.js     # URL search param helper
    ├── form-data.js      # FormData serialization
    └── utf8.js           # Text encoding utilities
```

### Main fetch() Function

The fetch implementation:

1. **Creates Request**: `new Request(url, options)`
2. **Validates Protocol**: Supports `data:`, `http:`, `https:`
3. **Handles data: URLs**: Directly decodes data URIs
4. **Uses http/https.request**: Wraps Node's native HTTP client
5. **Implements Redirect Logic**: Follows HTTP redirect spec
6. **Handles Compression**: Auto-decodes gzip, deflate, br
7. **Supports AbortSignal**: For request cancellation

```javascript
export default async function fetch(url, options_ = {}) {
  return new Promise((resolve, reject) => {
    const request = new Request(url, options_)
    const options = getNodeRequestOptions(request)

    // Protocol handling
    if (options.protocol === 'data:') {
      const data = dataUriToBuffer(request.url)
      resolve(new Response(data))
      return
    }

    // HTTP/HTTPS request
    const send = (options.protocol === 'https:' ? https : http).request
    const request_ = send(options)
    // ... response handling with redirect, compression
  })
}
```

### Body Class

The Body mixin handles body processing for both Request and Response:

**Supported Body Types:**

| Input Type | Processing |
|------------|------------|
| `null`/`undefined` | Empty body |
| `URLSearchParams` | Encoded as `application/x-www-form-urlencoded` |
| `Blob` | Uses `blob.stream()` |
| `Uint8Array` | Direct bytes |
| `ArrayBuffer` | Wrapped as Uint8Array |
| `ReadableStream` | Direct passthrough |
| `FormData` | Serialized as `multipart/form-data` |
| `Stream.Readable` | Converted to ReadableStream |
| Other | Coerced to string, UTF-8 encoded |

**Body Methods:**

```javascript
await body.arrayBuffer()  // Promise<ArrayBuffer>
await body.blob()         // Promise<Blob>
await body.json()         // Promise<object>
await body.text()         // Promise<string>
```

### Headers Class

Extends `URLSearchParams` for storage, with validation:

```javascript
export default class Headers extends URLSearchParams {
  // Validates header name/value per HTTP spec
  // Lowercases all header names
  // Supports: append, delete, get, getAll, has, set
  // Iteration: entries(), keys(), values(), forEach()
}
```

**Node.js Integration:**

- Uses `http.validateHeaderName` / `http.validateHeaderValue` when available
- Falls back to regex validation for older Node versions

### Request Class

```javascript
class Request extends Body {
  constructor(input, init = {}) {
    // Parses URL, validates method
    // Handles body cloning for Request input
    // Validates AbortSignal
  }

  // Getters
  get method()    // GET, POST, etc.
  get url()       // Formatted URL string
  get headers()   // Headers instance
  get signal()    // AbortSignal
  get redirect()  // 'follow', 'error', 'manual'

  // Node.js specific
  get follow()    // Max redirects (default: 20)
  get compress()  // Auto-decode compression (default: true)
  get agent()     // HTTP Agent for connection pooling
}
```

### Response Class

```javascript
class Response extends Body {
  constructor(body = null, options = {}) {
    super(body, options)
    this[INTERNALS] = {
      url, status, statusText, headers, counter
    }
  }

  get ok()       // true if 200-299
  get status()   // HTTP status code
  get redirected() // true if redirects occurred
  clone()        // Clone response (tees body stream)

  static redirect(url, status = 302)  // Create redirect response
}
```

### Usage Patterns

```javascript
import fetch from 'node-fetch'

// Basic GET
const response = await fetch('https://api.example.com/data')
const data = await response.json()

// POST with JSON
const response = await fetch('https://api.example.com/data', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ key: 'value' })
})

// With abort signal
const controller = new AbortController()
const response = await fetch(url, { signal: controller.signal })

// Handle redirects
const response = await fetch(url, { redirect: 'follow' }) // default
const response = await fetch(url, { redirect: 'error' })  // throw on redirect
const response = await fetch(url, { redirect: 'manual' }) // return redirect response
```

## Key Insights

### 1. Web API Compatibility First

All implementations prioritize spec compliance over Node.js convenience:
- Uses web `ReadableStream` instead of Node streams
- Uses `Uint8Array` instead of `Buffer`
- Follows W3C File API spec for name sanitization

### 2. Smart Node.js Fallbacks

The code detects and uses native implementations when safe:

```javascript
// blob.node.js - uses native Blob if bug-free
const Blob = use() || WebBlob

// headers.js - uses native validation if available
const validateHeaderName = typeof http.validateHeaderName === 'function'
  ? http.validateHeaderName
  : fallbackValidation
```

### 3. JSDoc-based TypeScript Support

Instead of `.d.ts` files, the code uses JSDoc annotations:

```javascript
/**
 * @param {BlobPart[]} [init]
 * @param {BlobPropertyBag} [options]
 */
constructor(init = [], options = {}) { ... }
```

Type definitions are generated via `tsc --build`.

### 4. Dual Module Support

All packages support both ESM and CommonJS:

```json
{
  "type": "module",
  "exports": {
    ".": {
      "import": "./src/lib.js",
      "require": "./dist/src/lib.cjs"
    }
  }
}
```

### 5. Minimal Dependencies

- `@web-std/file-url`: Zero dependencies
- `@web-std/blob`: Only `web-encoding` and `@web-std/stream`
- `node-fetch`: `data-uri-to-buffer`, `@web-std/blob`, `web-streams-polyfill`
