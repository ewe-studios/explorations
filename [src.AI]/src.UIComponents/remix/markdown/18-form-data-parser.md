# Form Data Parser: Multipart to FormData with File Upload Handling

`@remix-run/form-data-parser` wraps the multipart parser to produce a standard `FormData` object, with configurable file upload handlers.

Source: `packages/form-data-parser/src/lib/form-data.ts` — 150+ lines.

## FileUpload Class

```typescript
export class FileUpload extends File {
  readonly fieldName: string

  constructor(part: MultipartPart, fieldName: string) {
    super(part.content as BlobPart[], part.filename ?? 'file-upload', {
      type: part.mediaType ?? 'application/octet-stream',
    })
    this.fieldName = fieldName
  }
}
```

Extends the standard `File` class with `fieldName` — the name of the `<input>` that uploaded the file.

## FileUploadHandler

```typescript
export interface FileUploadHandler {
  (file: FileUpload): void | null | string | Blob | Promise<void | null | string | Blob>
}
```

The handler receives a `FileUpload` and can return:
- **`void` / `null`**: Skip this file (not included in FormData)
- **`string`**: Include the string value
- **`Blob`**: Include a blob (e.g., saved to disk, returns a reference)

### Default Handler

```typescript
async function defaultFileUploadHandler(file: FileUpload): Promise<File> {
  return file  // Keep in memory
}
```

## parseFormData

```typescript
export async function parseFormData(
  request: Request,
  options?: ParseFormDataOptions,
  uploadHandler: FileUploadHandler = defaultFileUploadHandler,
): Promise<FormData> {
  if (!isMultipartRequest(request)) {
    throw new FormDataParseError('Request is not a multipart request')
  }

  let formData = new FormData()
  let fileCount = 0
  let maxFiles = options?.maxFiles ?? 20

  for await (let part of parseMultipartRequest(request, options)) {
    let name = part.name
    if (!name) continue

    if (part.isFile) {
      fileCount++
      if (fileCount > maxFiles) {
        throw new MaxFilesExceededError(maxFiles)
      }

      let file = new FileUpload(part, name)
      let result = await uploadHandler(file)

      if (result !== undefined && result !== null) {
        formData.append(name, result)
      }
    } else {
      formData.append(name, part.text)
    }
  }

  return formData
}
```

### Flow

```mermaid
flowchart TD
    A[Request] --> B{isMultipartRequest?}
    B -->|No| C[throw FormDataParseError]
    B -->|Yes| D[parseMultipartRequest]
    D --> E[for each part]
    E --> F{isFile?}
    F -->|Yes| G[fileCount++]
    G --> H{fileCount > maxFiles?}
    H -->|Yes| I[throw MaxFilesExceededError]
    H -->|No| J[new FileUpload]
    J --> K[uploadHandler(file)]
    K --> L{result?}
    L -->|null/void| E
    L -->|value| M[formData.append]
    M --> E
    F -->|No| N[formData.append name, part.text]
    N --> E
    E --> O[return FormData]
```

Aha: The file count limit (`maxFiles`, default 20) protects against abuse. Without it, a malicious request could upload thousands of files, consuming memory. The limit is checked per-part, not per-request, so the parser stops as soon as the limit is hit.

## Error Classes

```typescript
export class FormDataParseError extends Error { name = 'FormDataParseError' }

export class MaxFilesExceededError extends FormDataParseError {
  constructor(maxFiles: number) {
    super(`Maximum number of files exceeded: ${maxFiles}`)
    this.name = 'MaxFilesExceededError'
  }
}
```

## Cross-References

- [Multipart Parser](14-multipart-parser.md) — Underlying parser
- [Middleware](10-middleware.md) — form-data middleware uses this

[Next → MIME](15-mime.md)
