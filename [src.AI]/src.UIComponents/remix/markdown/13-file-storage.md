# File Storage

`@remix-run/file-storage` provides a key/value interface for storing `File` objects with list, pagination, and metadata support.

Source: `packages/file-storage/src/lib/file-storage.ts` — 200+ lines.

## FileStorage Interface

```typescript
export interface FileStorage {
  get(key: string): File | null | Promise<File | null>
  has(key: string): boolean | Promise<boolean>
  list<T extends ListOptions>(options?: T): ListResult<T> | Promise<ListResult<T>>
  put(key: string, file: File): File | Promise<File>
  remove(key: string): void | Promise<void>
  set(key: string, file: string): void | Promise<void>
}
```

### list() — Pagination

```typescript
let result = await storage.list({ prefix: 'user123/', includeMetadata: true })
console.log(result.files)
// [
//   { key: "user123/avatar.png", lastModified: 1737955705270, name: "avatar.png", size: 16384, type: "image/png" },
//   ...
// ]

if (result.cursor !== undefined) {
  let result2 = await storage.list({ cursor: result.cursor })
}
```

Pagination via opaque `cursor` string. `includeMetadata: true` returns file size, type, and lastModified.

### put() — Returns Storage-Backed File

```typescript
let file = await storage.put('user123/avatar.png', uploadedFile)
// `file` is a File backed by this storage — reading it fetches from storage
```

## FileMetadata

```typescript
export interface FileMetadata extends FileKey {
  lastModified: number   // ms since Unix epoch
  name: string           // Original filename
  size: number           // Size in bytes
  type: string           // MIME type
}
```

## ListOptions

```typescript
export interface ListOptions {
  cursor?: string           // Pagination cursor
  includeMetadata?: boolean // Include file metadata
  limit?: number            // Max results
  prefix?: string           // Key prefix filter
}
```

## ListResult

```typescript
export interface ListResult<T extends ListOptions> {
  files: T extends { includeMetadata: true } ? FileMetadata[] : FileKey[]
  cursor?: string  // Opaque pagination cursor
}
```

Type-level: if `includeMetadata: true`, files are `FileMetadata[]`, otherwise `FileKey[]`.

## Cross-References

- [FS](13-fs.md) — File system utilities (uses file storage)
- [Lazy File](14-lazy-file.md) — Streaming file interface
