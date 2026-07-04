# Lazy File: Streaming Blob/File with Deferred Loading

`@remix-run/lazy-file` provides `LazyBlob` and `LazyFile` — Blob/File objects backed by streams instead of in-memory buffers.

Source: `packages/lazy-file/src/lib/lazy-file.ts` — 200+ lines, plus `byte-range.ts` for range calculation.

## LazyContent Interface

```typescript
export interface LazyContent {
  byteLength: number
  stream(start?: number, end?: number): ReadableStream<Uint8Array<ArrayBuffer>>
}
```

A lazy content provider knows its total size and can produce a stream for any byte range. This is the abstraction that enables range requests without loading full files.

## LazyBlob

```typescript
export class LazyBlob extends Blob {
  readonly #content: BlobContent

  constructor(parts: BlobPart[] | LazyContent, options?: LazyBlobOptions) {
    super([], options)  // Empty initial — content is lazy
    this.#content = new BlobContent(parts, options)
  }

  get size(): number {
    return this.#content.size
  }

  slice(start?: number, end?: number, contentType?: string): Blob {
    return this.#content.slice(start, end, contentType)
  }

  stream(): ReadableStream<Uint8Array<ArrayBuffer>> {
    return this.#content.stream()
  }

  async arrayBuffer(): Promise<ArrayBuffer> {
    return this.#content.arrayBuffer()
  }

  async bytes(): Promise<Uint8Array<ArrayBuffer>> {
    return this.#content.bytes()
  }

  async text(): Promise<string> {
    return this.#content.text()
  }
}
```

### Key Difference from Blob

A standard `Blob` stores all content in memory. `LazyBlob` calls `super([])` with an empty array — the content lives in `#content` and is only materialized when `.stream()`, `.arrayBuffer()`, `.bytes()`, or `.text()` is called.

### slice() — Range Subsetting

```typescript
slice(start?: number, end?: number, contentType?: string): Blob {
  return this.#content.slice(start, end, contentType)
}
```

Returns a new `LazyBlob` with a `ByteRange` that represents the slice — no data is copied. The new blob streams from the computed range of the original content.

## BlobContent

```typescript
class BlobContent {
  readonly #parts: BlobPart[] | null
  readonly #lazy: LazyContent | null
  readonly #range: ByteRange | null
  readonly #type: string

  get size(): number {
    if (this.#lazy) {
      // Calculate size based on range
      let fullLength = this.#lazy.byteLength
      if (!this.#range) return fullLength
      let [start, end] = getIndexes(this.#range, fullLength)
      return end - start
    }
    // Sum of all parts
    let size = 0
    for (let part of this.#parts) size += partSize(part)
    return size
  }

  stream(start?: number, end?: number): ReadableStream<Uint8Array> {
    if (this.#lazy) {
      let [s, e] = getIndexes(this.#range, this.#lazy.byteLength)
      if (start !== undefined) s += start
      if (end !== undefined) s += (end - s)
      return this.#lazy.stream(s, e)
    }
    // ... stream from parts
  }
}
```

## ByteRange

Source: `lib/byte-range.ts`

```typescript
export type ByteRange =
  | [start: number]           // From start to end
  | [start: number, end: number]  // From start to end (exclusive)

export function getIndexes(range: ByteRange, length: number): [number, number] {
  let [start, end] = range

  if (start < 0) start = Math.max(0, length + start)  // Negative = from end
  if (end === undefined) end = length
  if (end < 0) end = length + end

  return [Math.min(start, length), Math.min(end, length)]
}
```

Supports negative indices: `[-100]` means "last 100 bytes".

## LazyFile

```typescript
export class LazyFile extends LazyBlob {
  readonly #name: string
  readonly #lastModified: number

  constructor(content: BlobPart[] | LazyContent, name: string, options?: LazyFileOptions) {
    super(content, options)
    this.#name = name
    this.#lastModified = options?.lastModified ?? Date.now()
  }

  get name(): string { return this.#name }
  get lastModified(): number { return this.#lastModified }
}
```

Extends `LazyBlob` with `name` and `lastModified` — the two properties that distinguish `File` from `Blob`.

## Cross-References

- [FS](16-fs.md) — openFile returns a LazyFile
- [Static Files](10-middleware.md) — Serves LazyFile instances
- [File Storage](13-file-storage.md) — Storage-backed files use lazy content

[Next → HTML Template](19-html-template.md)
