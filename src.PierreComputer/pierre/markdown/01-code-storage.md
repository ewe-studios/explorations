---
title: code.storage
prev: 00-overview.md
next: 02-sdk.md
---

# code.storage

The core storage service providing Git-compatible cloud storage with JWT-based authentication.

## Architecture

```mermaid
flowchart TB
    subgraph Client["Client"]
        SDK[SDK]
        CLI[CLI]
    end

    subgraph API["API Layer"]
        AUTH[JWT Auth]
        REST[REST API]
    end

    subgraph Storage["Storage Layer"]
        REPOS[Repositories]
        BRANCHES[Ephemeral Branches]
        OBJECTS[Git Objects]
    end

    Client --> API
    REST --> Storage
```

## Authentication

JWT-based authentication with service accounts:

```typescript
// sdk/src/auth.ts
export interface AuthContext {
  token: string;
  expiresAt: Date;
  serviceAccount: string;
}

export async function authenticate(
  apiKey: string
): Promise<AuthContext> {
  const response = await fetch('https://api.code.storage/auth', {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${apiKey}` }
  });
  return response.json();
}
```

## Streaming Architecture

Data is streamed in 4MiB chunks:

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Storage

    Client->>API: Upload request
    API->>Client: Upload URL
    loop Chunked Upload
        Client->>Storage: PUT 4MiB chunk
        Storage->>Client: ETag
    end
    Client->>API: Complete upload
```

**Aha:** Chunking enables:
- Resumable uploads
- Parallel transfers
- Memory efficiency for large files

## Ephemeral Branches

Branches that exist only during active work:

```typescript
interface EphemeralBranch {
  name: string;
  parent: string;      // Parent commit
  ttl: number;         // Time-to-live in seconds
  createdAt: Date;
}

// Auto-deleted when TTL expires
// No need to clean up feature branches
```

## Repository Structure

```
repo/
├── refs/
│   ├── heads/          # Branch refs
│   └── tags/           # Tag refs
├── objects/
│   ├── info/
│   └── pack/           # Pack files
└── config              # Repository config
```

## Next Steps

Continue to [SDK →](02-sdk.html) for multi-language client libraries.
