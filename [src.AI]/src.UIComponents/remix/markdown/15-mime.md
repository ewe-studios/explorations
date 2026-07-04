# MIME: Type Detection and Compressibility

`@remix-run/mime` provides MIME type detection from filenames and compressibility checking based on `mime-db`.

Source: `packages/mime/src/` — generated data files + detection logic.

## detectMimeType

Source: `lib/detect-mime-type.ts`

```typescript
export function detectMimeType(filename: string): string | undefined {
  let ext = filename.slice(filename.lastIndexOf('.') + 1).toLowerCase()
  return MIME_TYPES[ext]
}
```

Simple extension lookup:

```typescript
// packages/mime/src/generated/mime-types.ts (excerpt)
export const MIME_TYPES: Record<string, string> = {
  'html': 'text/html',
  'htm': 'text/html',
  'css': 'text/css',
  'js': 'application/javascript',
  'json': 'application/json',
  'png': 'image/png',
  'jpg': 'image/jpeg',
  'jpeg': 'image/jpeg',
  'gif': 'image/gif',
  'svg': 'image/svg+xml',
  'mp4': 'video/mp4',
  'webm': 'video/webm',
  'pdf': 'application/pdf',
  'zip': 'application/zip',
  'gz': 'application/gzip',
  // ... hundreds more from mime-db
}
```

The data is generated from `mime-db` — no runtime parsing needed.

## isCompressibleMimeType

Source: `lib/is-compressible-mime-type.ts`

```typescript
export function isCompressibleMimeType(mediaType: string): boolean {
  // Strip parameters: "text/html; charset=utf-8" → "text/html"
  let type = mediaType.split(';')[0].trim().toLowerCase()
  return COMPRESSIBLE_MIME_TYPES.has(type)
}
```

### Generated Compressible Types

Source: `generated/compressible-mime-types.ts`

```typescript
export const COMPRESSIBLE_MIME_TYPES: Set<string> = new Set([
  'application/atom+xml',
  'application/graphql-response+json',
  'application/javascript',
  'application/json',
  'application/ld+json',
  'application/manifest+json',
  'application/vnd.api+json',
  'application/xml',
  'application/xhtml+xml',
  'font/opentype',
  'font/otf',
  'font/ttf',
  'image/bmp',
  'image/svg+xml',
  'image/x-icon',
  'text/cache-manifest',
  'text/calendar',
  'text/css',
  'text/html',
  'text/javascript',
  'text/plain',
  'text/vcard',
  'text/x-asm',
  'text/xml',
  // ... more text-based types
])
```

Aha: The set is generated from `mime-db`'s `compressible` field. Binary types (images, videos, archives) are excluded because they're already compressed and would waste CPU trying to compress again. Text-based types (HTML, CSS, JS, JSON, XML, fonts) are included.

### Compressible vs Non-Compressible

| Compressible | Not Compressible |
|-------------|-----------------|
| `text/html` | `image/png` |
| `text/css` | `image/jpeg` |
| `application/javascript` | `image/gif` |
| `application/json` | `video/mp4` |
| `text/plain` | `audio/mpeg` |
| `font/ttf` | `application/zip` |
| `image/svg+xml` | `application/gzip` |

## Usage in Compression Middleware

```typescript
// compression-middleware uses isCompressibleMimeType as default filter
let filterMediaType = options?.filterMediaType ?? isCompressibleMimeType
if (!filterMediaType(mediaType)) {
  return response  // Skip compression for binary content
}
```

## Package Structure

```
mime/src/
├── index.ts
├── generated/
│   ├── mime-types.ts              # Extension → MIME type mapping
│   └── compressible-mime-types.ts # Set of compressible MIME types
└── lib/
    ├── detect-mime-type.ts        # Filename → MIME type
    └── is-compressible-mime-type.ts # MIME type → compressible?
```

## Cross-References

- [Compression Middleware](10-middleware.md) — Uses isCompressibleMimeType
- [Static Files](10-middleware.md) — Uses detectMimeType for Content-Type

[Next → FS](16-fs.md)
