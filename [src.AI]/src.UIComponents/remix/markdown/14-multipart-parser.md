# Multipart Parser: Streaming Boundary Search with Boyer-Moore

`@remix-run/multipart-parser` is a streaming multipart message parser using Boyer-Moore string search for boundary detection. Handles chunked input with partial boundary matching across chunk boundaries.

Source: `packages/multipart-parser/src/lib/multipart.ts` — 300+ lines, plus `buffer-search.ts` for the search algorithm.

## State Machine

```typescript
const MultipartParserStateStart = 0
const MultipartParserStateAfterBoundary = 1
const MultipartParserStateHeader = 2
const MultipartParserStateBody = 3
const MultipartParserStateDone = 4
```

Five states:

| State | Enters When | Exits When |
|-------|------------|-----------|
| `Start` | Parser created, first chunk written | Opening boundary found |
| `AfterBoundary` | Boundary detected (`--boundary`) | Headers found (after `\r\n`) |
| `Header` | Parsing headers | Double CRLF (`\r\n\r\n`) found |
| `Body` | Reading body content | Next boundary (`\r\n--boundary`) found |
| `Done` | Closing boundary (`--`) found | `finish()` called |

Aha: The state machine is encoded as plain integers, not an enum or string. This avoids object allocation and comparison overhead in the hot path — the `write()` method processes every byte of every chunk.

## MultipartParser — Internal State

```typescript
export class MultipartParser {
  readonly boundary: string
  readonly maxHeaderSize: number   // Default: 8 KiB
  readonly maxFileSize: number     // Default: 2 MiB

  #findOpeningBoundary: SearchFunction       // Finds `--boundary`
  #openingBoundaryLength: number             // 2 + boundary.length
  #findBoundary: SearchFunction              // Finds `\r\n--boundary`
  #findPartialTailBoundary: PartialTailSearchFunction  // Partial match at chunk end
  #boundaryLength: number                    // 4 + boundary.length

  #state = MultipartParserStateStart
  #buffer: Uint8Array | null = null          // Carry-over between chunks
  #currentPart: MultipartPart | null = null  // Part being accumulated
  #contentLength = 0                         // Bytes in current part
}
```

Aha: Three search functions are pre-created in the constructor: `#findOpeningBoundary` for the first boundary, `#findBoundary` for subsequent boundaries, and `#findPartialTailBoundary` for detecting partial boundary matches at the end of a chunk. This avoids recomputing the Boyer-Moore skip table on every `write()` call.

## write() — The Hot Path

```typescript
*write(chunk: Uint8Array): Generator<MultipartPart, void, unknown> {
  if (this.#state === MultipartParserStateDone) {
    throw new MultipartParseError('Unexpected data after end of stream')
  }

  let index = 0
  let chunkLength = chunk.length

  // Buffer carry-over: merge previous partial chunk with current
  if (this.#buffer !== null) {
    let newChunk = new Uint8Array(this.#buffer.length + chunkLength)
    newChunk.set(this.#buffer, 0)
    newChunk.set(chunk, this.#buffer.length)
    chunk = newChunk
    chunkLength = chunk.length
    this.#buffer = null
  }

  while (true) {
    // ... state machine dispatch
  }
}
```

### Buffer Carry-Over

When a chunk ends mid-boundary or mid-header, the remaining bytes are stored in `#buffer`. On the next `write()`, the buffer is prepended to the new chunk:

```typescript
if (this.#buffer !== null) {
  let newChunk = new Uint8Array(this.#buffer.length + chunkLength)
  newChunk.set(this.#buffer, 0)
  newChunk.set(chunk, this.#buffer.length)
  chunk = newChunk
  this.#buffer = null
}
```

Aha: The buffer copy creates a new `Uint8Array` on each carry-over. This is a deliberate tradeoff: it's simpler than managing a ring buffer, and for typical form submissions (small payloads), the carry-over happens at most once per part boundary. For high-throughput streaming, a ring buffer would be needed.

### State: Start

```typescript
if (this.#state === MultipartParserStateStart) {
  if (chunkLength < this.#openingBoundaryLength) {
    this.#buffer = chunk
    break
  }

  if (this.#findOpeningBoundary(chunk) !== 0) {
    throw new MultipartParseError('Invalid multipart stream: missing initial boundary')
  }

  index = this.#openingBoundaryLength
  this.#state = MultipartParserStateAfterBoundary
}
```

The opening boundary MUST be at position 0. If the chunk is smaller than the boundary length, it's buffered for the next chunk.

### State: AfterBoundary

```typescript
if (this.#state === MultipartParserStateAfterBoundary) {
  if (chunkLength - index < 2) {
    this.#buffer = chunk.subarray(index)
    break
  }

  if (chunk[index] === 45 && chunk[index + 1] === 45) {
    // Closing boundary: `--`
    this.#state = MultipartParserStateDone
    break
  }

  index += 2  // Skip \r\n after boundary
  this.#state = MultipartParserStateHeader
}
```

Checks for the closing boundary (`--` after the boundary marker). ASCII 45 = `-`.

### State: Header

```typescript
if (this.#state === MultipartParserStateHeader) {
  if (chunkLength - index < 4) {
    this.#buffer = chunk.subarray(index)
    break
  }

  let headerEndIndex = findDoubleNewline(chunk, index)

  if (headerEndIndex === -1) {
    if (chunkLength - index > this.maxHeaderSize) {
      throw new MaxHeaderSizeExceededError(this.maxHeaderSize)
    }
    this.#buffer = chunk.subarray(index)
    break
  }

  if (headerEndIndex - index > this.maxHeaderSize) {
    throw new MaxHeaderSizeExceededError(this.maxHeaderSize)
  }

  this.#currentPart = new MultipartPart(chunk.subarray(index, headerEndIndex), [])
  this.#contentLength = 0

  index = headerEndIndex + 4  // Skip header + \r\n\r\n
  this.#state = MultipartParserStateBody
  continue
}
```

Searches for `\r\n\r\n` (double newline) to mark the end of headers. If not found in the current chunk, the remaining bytes are buffered.

### State: Body — The Complex Case

```typescript
if (this.#state === MultipartParserStateBody) {
  if (chunkLength - index < this.#boundaryLength) {
    this.#buffer = chunk.subarray(index)
    break
  }

  let boundaryIndex = this.#findBoundary(chunk, index)

  if (boundaryIndex === -1) {
    // No full boundary found — check for partial match at end
    let partialTailIndex = this.#findPartialTailBoundary(chunk)

    if (partialTailIndex === -1) {
      // No partial match either — entire remainder is body content
      this.#append(index === 0 ? chunk : chunk.subarray(index))
    } else {
      // Partial match at end — keep it for next chunk
      this.#append(chunk.subarray(index, partialTailIndex))
      this.#buffer = chunk.subarray(partialTailIndex)
    }
    break
  }

  // Full boundary found — yield the part
  this.#append(chunk.subarray(index, boundaryIndex))
  yield this.#currentPart!

  index = boundaryIndex + this.#boundaryLength
  this.#state = MultipartParserStateAfterBoundary
}
```

Aha: The partial tail search is the key insight. When the boundary string spans a chunk boundary (e.g., `\r\n--boun` at end of chunk 1, `dary\r\n` at start of chunk 2), a regular search won't find it. The `#findPartialTailBoundary` checks if the end of the chunk matches the beginning of the boundary pattern. If it does, that tail is buffered for the next chunk.

## Boyer-Moore Search

Source: `lib/buffer-search.ts`

### createSearch() — Full Boyer-Moore

```typescript
export function createSearch(pattern: string): SearchFunction {
  let needle = new TextEncoder().encode(pattern)

  if ('Buffer' in globalThis && !('Bun' in globalThis || 'Deno' in globalThis)) {
    // Node.js: use Buffer.prototype.indexOf (optimized C++ impl)
    search = (haystack, start = 0) => Buffer.prototype.indexOf.call(haystack, needle, start)
  } else {
    // Deno/Bun/browser: Boyer-Moore skip table
    let needleEnd = needle.length - 1
    let skipTable = new Uint8Array(256).fill(needle.length)
    for (let i = 0; i < needleEnd; ++i) {
      skipTable[needle[i]] = needleEnd - i
    }

    search = (haystack, start = 0) => {
      let haystackLength = haystack.length
      let i = start + needleEnd

      while (i < haystackLength) {
        for (let j = needleEnd, k = i; j >= 0 && haystack[k] === needle[j]; --j, --k) {
          if (j === 0) return k
        }
        i += skipTable[haystack[i]]
      }
      return -1
    }
  }
}
```

Two implementations:
1. **Node.js**: Uses `Buffer.prototype.indexOf` — optimized C++ code
2. **Deno/Bun/browser**: Pure JS Boyer-Moore with bad-character skip table

### Boyer-Moore Skip Table

The skip table tells the search how far to jump when a mismatch occurs:

```typescript
let skipTable = new Uint8Array(256).fill(needle.length)
for (let i = 0; i < needleEnd; ++i) {
  skipTable[needle[i]] = needleEnd - i
}
```

For pattern `\r\n--boundary` (14 bytes):
- `skipTable['\r']` = 13 (second-to-last char position)
- `skipTable['\n']` = 12
- `skipTable['-']` = 11 (last `-` position)
- All other bytes: 14 (skip full pattern length)

This means when scanning body content (which is random bytes), most characters cause a 14-byte skip — O(n/m) average case instead of O(n*m) for naive search.

### createPartialTailSearch() — Cross-Chunk Boundary Detection

```typescript
export function createPartialTailSearch(pattern: string): PartialTailSearchFunction {
  let needle = new TextEncoder().encode(pattern)

  // Build reverse index: byte value → positions in needle
  let byteIndexes: Record<number, number[]> = {}
  for (let i = 0; i < needle.length; ++i) {
    let byte = needle[i]
    if (byteIndexes[byte] === undefined) byteIndexes[byte] = []
    byteIndexes[byte].push(i)
  }

  return function (haystack: Uint8Array): number {
    let haystackEnd = haystack.length - 1

    // Check if last byte appears anywhere in the pattern
    if (haystack[haystackEnd] in byteIndexes) {
      let indexes = byteIndexes[haystack[haystackEnd]]

      // Try each position (reverse order for rightmost match)
      for (let i = indexes.length - 1; i >= 0; --i) {
        for (let j = indexes[i], k = haystackEnd; j >= 0 && haystack[k] === needle[j]; --j, --k) {
          if (j === 0) return k  // Found prefix match
        }
      }
    }
    return -1
  }
}
```

Aha: This searches for the **beginning** of the pattern at the **end** of the chunk. It builds a reverse index: for each byte value, store all positions where it appears in the pattern. Then, looking at the last byte of the chunk, it tries each position in reverse order. If the last byte of the chunk matches position 3 of the pattern, it checks if positions 2, 1, 0 also match going backwards.

For example, if the chunk ends with `\r\n--bo` and the pattern is `\r\n--boundary`:
- Last byte `o` matches position 7 of the pattern
- Check backwards: `b` at position 6 matches, `-` at 5 matches, `-` at 4 matches, `\n` at 3 matches, `\r` at 2 matches
- Returns index of `\r` — this is the partial boundary start

## MultipartPart

```typescript
export class MultipartPart {
  readonly content: Uint8Array[]  // Accumulated chunks
  #header: Uint8Array
  #headers?: Headers

  get headers(): Headers {
    if (!this.#headers) {
      this.#headers = new Headers(decoder.decode(this.#header))
    }
    return this.#headers
  }

  get bytes(): Uint8Array {
    let buffer = new Uint8Array(this.size)
    let offset = 0
    for (let chunk of this.content) {
      buffer.set(chunk, offset)
      offset += chunk.length
    }
    return buffer
  }

  get isFile(): boolean {
    return this.filename !== undefined || this.mediaType === 'application/octet-stream'
  }

  get isText(): boolean {
    return !this.isFile
  }

  get filename(): string | undefined {
    return this.headers.contentDisposition.preferredFilename
  }

  get name(): string | undefined {
    return this.headers.contentDisposition.name
  }
}
```

Headers are lazily parsed (only when accessed). `bytes` concatenates all chunks into a single `Uint8Array`.

## parseMultipartRequest

Source: `lib/multipart-request.ts`

```typescript
export function getMultipartBoundary(contentType: string): string | null {
  let match = /boundary=(?:"([^"]+)"|([^;]+))/i.exec(contentType)
  return match ? (match[1] ?? match[2]) : null
}

export async function* parseMultipartRequest(request: Request, options?) {
  if (!isMultipartRequest(request)) throw new MultipartParseError('...')
  if (!request.body) throw new MultipartParseError('...')

  let boundary = getMultipartBoundary(request.headers.get('Content-Type')!)
  if (!boundary) throw new MultipartParseError('...')

  yield* parseMultipartStream(request.body, { boundary, ...options })
}
```

Boundary extraction handles both quoted and unquoted forms:
- `multipart/form-data; boundary="abc123"` → `abc123`
- `multipart/form-data; boundary=abc123` → `abc123`

## Error Classes

```typescript
export class MultipartParseError extends Error { name = 'MultipartParseError' }
export class MaxHeaderSizeExceededError extends MultipartParseError { constructor(maxHeaderSize) }
export class MaxFileSizeExceededError extends MultipartParseError { constructor(maxFileSize) }
```

## Package Structure

```
multipart-parser/src/
├── index.ts                   # Public API
├── node.ts                    # Node.js-specific exports
└── lib/
    ├── multipart.ts           # MultipartParser, MultipartPart, parseMultipart
    ├── multipart-request.ts   # parseMultipartRequest, getMultipartBoundary
    ├── buffer-search.ts       # Boyer-Moore search + partial tail search
    ├── read-stream.ts         # ReadableStream → async iterator
    └── multipart.node.ts      # Node.js stream adapter
```

## Cross-References

- [Form Data Parser](18-form-data-parser.md) — Uses multipart-parser internally
- [Headers](06-headers.md) — Used for parsing part headers
- [Fetch Router](09-node-fetch-server.md) — Server that receives multipart requests

[Next → Form Data Parser](18-form-data-parser.md)
