# TAR Parser: Streaming Archive Parser with UStar/GNU/PAX Support

`@remix-run/tar-parser` is a streaming TAR archive parser supporting UStar, GNU, and PAX formats with per-entry streaming.

Source: `packages/tar-parser/src/lib/tar.ts` — 300+ lines.

## TarHeader — 512-Byte Block Format

```typescript
export interface TarHeader {
  name: string        // Filename (offset 0, 100 bytes)
  mode: number | null // File mode (offset 100, 8 bytes, octal)
  uid: number | null  // Owner UID (offset 108, 8 bytes, octal)
  gid: number | null  // Owner GID (offset 116, 8 bytes, octal)
  size: number        // File size (offset 124, 12 bytes, octal)
  mtime: number | null // Modification time (offset 136, 12 bytes, octal)
  type: string        // Type flag (offset 156, 1 byte)
  linkname: string | null // Linked file name (offset 157, 100 bytes)
  uname: string       // Owner username (offset 265, 32 bytes)
  gname: string       // Owner groupname (offset 297, 32 bytes)
  devmajor: number | null  // Device major (offset 329, 8 bytes)
  devminor: number | null  // Device minor (offset 337, 8 bytes)
  pax: Record<string, string> | null  // PAX extended attributes
}
```

### TAR Block Layout

| Offset | Size | Field |
|--------|------|-------|
| 0 | 100 | Filename |
| 100 | 8 | File mode (octal) |
| 108 | 8 | Owner UID (octal) |
| 116 | 8 | Owner GID (octal) |
| 124 | 12 | File size (octal) |
| 136 | 12 | Modification time (octal) |
| 148 | 8 | Checksum (octal) |
| 156 | 1 | Type flag |
| 157 | 100 | Linked file name |
| 257 | 6 | Magic ("ustar\0" or "ustar ") |
| 263 | 2 | Version |
| 265 | 32 | Owner username |
| 297 | 32 | Owner groupname |
| 329 | 8 | Device major |
| 337 | 8 | Device minor |
| 345 | 155 | Filename prefix (UStar only) |

### Type Flags

```typescript
const TarFileTypes: Record<string, string> = {
  '0': 'file',
  '1': 'link',
  '2': 'symlink',
  '3': 'character-device',
  '4': 'block-device',
  '5': 'directory',
  '6': 'fifo',
  '7': 'contiguous-file',
  '27': 'gnu-long-link-path',  // GNU extension
  '28': 'gnu-long-path',       // GNU extension
  '30': 'gnu-long-path',       // GNU extension (alternate)
  '55': 'pax-global-header',   // PAX global attributes
  '72': 'pax-header',          // PAX extended attributes
}
```

### Format Detection

```typescript
let magic = block.subarray(257, 263)
let version = block.subarray(263, 265)

const UstarMagic = new Uint8Array([0x75, 0x73, 0x74, 0x61, 0x72, 0x00])  // "ustar\0"
const UstarVersion = new Uint8Array([0x30, 0x30])  // "00"
const GnuMagic = new Uint8Array([0x75, 0x73, 0x74, 0x61, 0x72, 0x20])   // "ustar "
const GnuVersion = new Uint8Array([0x20, 0x00])  // " \0"

if (buffersEqual(magic, UstarMagic) && buffersEqual(version, UstarVersion)) {
  // UStar (POSIX) format — supports prefix field for long filenames
  if (block[345] !== 0) {
    let prefix = getString(block, 345, 155)
    header.name = prefix + '/' + header.name
  }
} else if (buffersEqual(magic, GnuMagic) && buffersEqual(version, GnuVersion)) {
  // GNU format — uses long-path entries for filenames > 100 chars
}
```

Aha: The magic bytes use `Uint8Array` literals with hex values instead of string comparison. This avoids encoding issues — `"ustar\0"` as a string might have different byte representations depending on the JS engine. Using `[0x75, 0x73, 0x74, 0x61, 0x72, 0x00]` guarantees exact byte matching.

## Checksum Verification

```typescript
let checksum = getOctal(block, 148, 8)
if (checksum !== computeChecksum(block)) {
  throw new TarParseError('Invalid tar header. Maybe the tar is corrupted or needs to be gunzipped?')
}
```

The checksum is an octal number at offset 148. `computeChecksum` sums all bytes in the 512-byte header block, treating the checksum field itself as spaces (0x20).

## TarParser — State Machine

```typescript
export class TarParser {
  #buffer: Uint8Array | null = null
  #missing = 0                       // Bytes remaining in current entry body
  #header: TarHeader | null = null
  #bodyController: ReadableStreamDefaultController<Uint8Array> | null = null
  #longHeader = false                // Reading GNU/PAX extended header
  #gnuLongPath: string | null = null
  #gnuLongLinkPath: string | null = null
  #paxGlobal: Record<string, string> | null = null
  #pax: Record<string, string> | null = null
}
```

### parse() — Multi-Format Input

```typescript
async parse(archive: TarArchiveSource, handler: TarEntryHandler): Promise<void> {
  if (archive instanceof ReadableStream) {
    for await (let chunk of readStream(archive)) {
      this.#write(chunk, handleEntry)
    }
  } else if (isAsyncIterable(archive)) {
    for await (let chunk of archive) {
      this.#write(chunk, handleEntry)
    }
  } else if (archive instanceof Uint8Array) {
    this.#write(archive, handleEntry)
  } else if (isIterable(archive)) {
    for (let chunk of archive) {
      this.#write(chunk, handleEntry)
    }
  }
}
```

Accepts: `ReadableStream`, `AsyncIterable<Uint8Array>`, `Uint8Array`, or `Iterable<Uint8Array>`.

### #write() — The Hot Path

```typescript
#write(chunk: Uint8Array, handler: TarEntryHandler): void {
  if (this.#buffer !== null) {
    this.#buffer = concatChunks(this.#buffer, chunk)
  } else {
    this.#buffer = chunk
  }

  while (this.#buffer !== null && this.#buffer.length > 0) {
    if (this.#missing > 0) {
      if (this.#bodyController !== null) {
        this.#parseBody()      // Streaming entry body
        continue
      }
      if (this.#longHeader) {
        if (this.#missing > this.#buffer.length) break
        this.#parseLongHeader()  // GNU/PAX extended header
        continue
      }
      // Skip remaining body bytes
      if (this.#missing >= this.#buffer.length) {
        this.#missing -= this.#buffer.length
        this.#buffer = null
        break
      }
      this.#buffer = this.#buffer.subarray(this.#missing)
      this.#missing = 0
    }

    if (this.#buffer.length < TarBlockSize) break
    this.#parseHeader(handler)
  }
}
```

Aha: The parser uses `#missing` to track how many bytes remain in the current entry's body. When `#bodyController` is set, body chunks are streamed to the entry's `ReadableStream`. When `#longHeader` is set, the body is actually a GNU/PAX extended header (long filename or attributes).

### #parseHeader() — Entry Creation

```typescript
#parseHeader(handler: TarEntryHandler): void {
  let block = this.#read(TarBlockSize)

  if (isZeroBlock(block)) {
    this.#header = null  // End of archive
    return
  }

  this.#header = parseTarHeader(block, this.#options)

  // Handle special entry types
  switch (this.#header.type) {
    case 'gnu-long-path':
    case 'gnu-long-link-path':
    case 'pax-global-header':
    case 'pax-header':
      this.#longHeader = true
      this.#missing = this.#header.size
      return
  }

  // Apply GNU/PAX overrides
  if (this.#gnuLongPath) {
    this.#header.name = this.#gnuLongPath
    this.#gnuLongPath = null
  }
  if (this.#pax) {
    if (this.#pax.path) this.#header.name = this.#pax.path
    if (this.#pax.size) this.#header.size = parseInt(this.#pax.size, 10)
    this.#header.pax = this.#pax
    this.#pax = null
  }

  // Create entry with streaming body
  if (this.#header.size === 0 || this.#header.type === 'directory') {
    let emptyBody = new ReadableStream({ start(c) { c.close() } })
    handler(new TarEntry(this.#header, emptyBody))
    return
  }

  let body = new ReadableStream({
    start: (controller) => { this.#bodyController = controller },
  })
  handler(new TarEntry(this.#header, body))
  this.#missing = this.#header.size
}
```

Each entry gets a `ReadableStream` for its body. The parser pushes chunks to `#bodyController` as they arrive from subsequent `#write()` calls.

## TarEntry

```typescript
export class TarEntry {
  #header: TarHeader
  #body: ReadableStream<Uint8Array>
  #bodyUsed = false

  get name(): string { return this.header.name }
  get size(): number { return this.header.size }
  get body(): ReadableStream<Uint8Array> { return this.#body }

  async bytes(): Promise<Uint8Array> {
    if (this.#bodyUsed) throw new Error('Body is already consumed')
    this.#bodyUsed = true

    let result = new Uint8Array(this.size)
    let offset = 0
    for await (let chunk of readStream(this.#body)) {
      result.set(chunk, offset)
      offset += chunk.length
    }
    return result
  }

  async text(): Promise<string> { return new TextDecoder().decode(await this.bytes()) }
  async arrayBuffer(): Promise<ArrayBuffer> { return (await this.bytes()).buffer }
}
```

## parseTar Convenience Function

```typescript
export async function parseTar(archive, handler): Promise<void>
export async function parseTar(archive, options, handler): Promise<void>

// Usage:
await parseTar(archiveStream, (entry) => {
  console.log(entry.name, entry.size)
})
```

## Cross-References

- [File Storage](13-file-storage.md) — TAR archives stored in file storage
- [FS](16-fs.md) — TAR files opened via openFile
