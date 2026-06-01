---
title: TypeScript SDK
prev: 02-python-sdk.md
next: 04-resources.md
---

# TypeScript SDK

The TypeScript/JavaScript implementation of mirage.

## Project Structure

```
typescript/
├── packages/
│   ├── core/           # VFS core
│   ├── node/           # Node.js resources
│   ├── cli/            # Command-line interface
│   ├── server/         # HTTP server
│   ├── browser/        # Browser support
│   └── agents/         # Framework integrations
├── scripts/
└── pnpm-workspace.yaml
```

## Packages

### @struktoai/mirage-core

**Location:** `typescript/packages/core/`

The core VFS implementation:

```typescript
// packages/core/src/workspace.ts
export class Workspace {
  private vfs: VirtualFileSystem;
  private dispatcher: CommandDispatcher;

  constructor(mounts: Record<string, Resource>) {
    this.vfs = new VirtualFileSystem();
    
    for (const [path, resource] of Object.entries(mounts)) {
      this.vfs.mount(path, resource);
    }
    
    this.dispatcher = new CommandDispatcher(this.vfs);
  }

  async execute(command: string): Promise<string> {
    return await this.dispatcher.dispatch(command);
  }

  clone(): Workspace {
    return new Workspace(this.vfs.cloneMounts());
  }
}
```

### @struktoai/mirage-node

**Location:** `typescript/packages/node/`

Node.js-specific resources:

```typescript
// packages/node/src/resources/disk.ts
import { Resource, FileStat } from '@struktoai/mirage-core';
import { promises as fs } from 'fs';
import { join } from 'path';

export class DiskResource implements Resource {
  private basePath: string;

  constructor(basePath: string) {
    this.basePath = basePath;
  }

  async read(path: string): Promise<Uint8Array> {
    const fullPath = join(this.basePath, path);
    const buffer = await fs.readFile(fullPath);
    return new Uint8Array(buffer);
  }

  async write(path: string, data: Uint8Array): Promise<void> {
    const fullPath = join(this.basePath, path);
    await fs.mkdir(join(fullPath, '..'), { recursive: true });
    await fs.writeFile(fullPath, data);
  }

  async list(path: string): Promise<DirEntry[]> {
    const fullPath = join(this.basePath, path);
    const entries = await fs.readdir(fullPath, { withFileTypes: true });
    return entries.map(e => ({
      name: e.name,
      isDirectory: e.isDirectory(),
      isFile: e.isFile(),
    }));
  }

  async stat(path: string): Promise<FileStat> {
    const fullPath = join(this.basePath, path);
    const stats = await fs.stat(fullPath);
    return {
      size: stats.size,
      modified: stats.mtime,
      isDirectory: stats.isDirectory(),
      isFile: stats.isFile(),
    };
  }
}
```

## Usage

```typescript
import { Workspace } from '@struktoai/mirage-core';
import { DiskResource, S3Resource } from '@struktoai/mirage-node';

const ws = new Workspace({
  '/data': new DiskResource('/tmp/mirage'),
  '/s3': new S3Resource({ bucket: 'logs', region: 'us-east-1' }),
});

// Execute commands
const result = await ws.execute('ls /s3');
console.log(result);
```

## Browser Support

**Location:** `typescript/packages/browser/`

**Aha:** Browser SDK uses in-memory resources and HTTP-based remote resources:

```typescript
// packages/browser/src/index.ts
import { Workspace, RAMResource } from '@struktoai/mirage-core';
import { HTTPResource } from './resources/http';

export function createBrowserWorkspace(serverUrl: string): Workspace {
  return new Workspace({
    '/data': new RAMResource(),
    '/remote': new HTTPResource(serverUrl),
  });
}
```

## Differences from Python

| Aspect | Python | TypeScript |
|--------|--------|------------|
| Runtime | CPython | Node.js / Browser |
| Async | `async/await` | `Promise` / `async/await` |
| Types | Runtime checked | Compile-time checked |
| FUSE | Native support | Not available (no FUSE) |
| Server | Optional | Primary for remote |

## Next Steps

Continue to [Resources →](04-resources.html) for built-in resource types.
