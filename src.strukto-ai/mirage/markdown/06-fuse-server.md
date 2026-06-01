---
title: FUSE & Server
prev: 05-commands.md
next: 07-frameworks.md
---

# FUSE & Server

Mount mirage as a filesystem or access via HTTP.

## FUSE Mode

**Aha:** FUSE lets you mount mirage as a real filesystem on Linux/macOS.

```bash
# Mount mirage workspace
mirage mount /mnt/mirage --workspace config.yaml

# Now use regular Unix tools
cat /mnt/mirage/s3/config.json
ls /mnt/mirage/slack/general/
cp /mnt/mirage/s3/file.csv ~/local.csv
```

### Python FUSE

**Source:** `python/mirage/fuse/`

```python
# python/mirage/fuse/mount.py
import fuse
from mirage.core import Workspace

class MirageFUSE(fuse.Fuse):
    def __init__(self, workspace: Workspace):
        self.workspace = workspace
        super().__init__()
    
    def getattr(self, path):
        """Get file attributes."""
        try:
            resource, rel_path = self.workspace.vfs.resolve(path)
            stat = resource.stat(rel_path)
            return fuse.Stat(
                st_size=stat.size,
                st_mode=(stat.is_directory and 0o40755) or 0o100644,
            )
        except FileNotFoundError:
            return -fuse.ENOENT
    
    def readdir(self, path, offset):
        """Read directory contents."""
        resource, rel_path = self.workspace.vfs.resolve(path)
        entries = resource.list(rel_path)
        
        for entry in entries:
            yield fuse.Direntry(entry.name)
    
    def read(self, path, size, offset):
        """Read file contents."""
        resource, rel_path = self.workspace.vfs.resolve(path)
        data = resource.read(rel_path)
        return data[offset:offset + size]
    
    def write(self, path, buf, offset):
        """Write file contents."""
        resource, rel_path = self.workspace.vfs.resolve(path)
        resource.write(rel_path, buf)
        return len(buf)
```

### Usage

```python
# python/mirage/cli/main.py
import click
from mirage.fuse import mount

@click.command()
@click.argument('mountpoint')
@click.option('--workspace', '-w', required=True)
def cli(mountpoint, workspace):
    """Mount mirage workspace as FUSE filesystem."""
    ws = Workspace.load(workspace)
    
    fuse = MirageFUSE(ws)
    fuse.mount(mountpoint)
```

## Server Mode

**Aha:** Server mode lets remote clients access the workspace via HTTP.

```bash
# Start server
mirage server --workspace config.yaml --port 8080

# Client connects
curl http://localhost:8080/api/v1/ls?path=/s3
```

### Python Server

**Source:** `python/mirage/server/`

```python
# python/mirage/server/fastapi.py
from fastapi import FastAPI, HTTPException
from mirage.core import Workspace

app = FastAPI()
workspace: Workspace

@app.on_event("startup")
async def startup():
    global workspace
    workspace = Workspace.load(os.environ['MIRAGE_WORKSPACE'])

@app.get("/api/v1/read")
async def read(path: str):
    """Read file contents."""
    try:
        resource, rel_path = workspace.vfs.resolve(path)
        data = await resource.read(rel_path)
        return {"content": data.decode('utf-8')}
    except FileNotFoundError:
        raise HTTPException(404, "File not found")

@app.get("/api/v1/ls")
async def ls(path: str = '/'):
    """List directory."""
    resource, rel_path = workspace.vfs.resolve(path)
    entries = await resource.list(rel_path)
    return {"entries": [
        {"name": e.name, "is_directory": e.is_directory}
        for e in entries
    ]}

@app.post("/api/v1/execute")
async def execute(command: CommandRequest):
    """Execute bash command."""
    result = await workspace.execute(command.cmd)
    return {"output": result}
```

### TypeScript Server

**Location:** `typescript/packages/server/`

```typescript
// packages/server/src/index.ts
import express from 'express';
import { Workspace } from '@struktoai/mirage-core';

const app = express();
const workspace = new Workspace({...});

app.get('/api/v1/read', async (req, res) => {
    const { path } = req.query;
    const resource = workspace.vfs.resolve(path);
    const data = await resource.read(relPath);
    res.json({ content: data.toString() });
});

app.post('/api/v1/execute', async (req, res) => {
    const { command } = req.body;
    const result = await workspace.execute(command);
    res.json({ output: result });
});

app.listen(8080);
```

## Protocol

### REST API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/read` | GET | Read file |
| `/api/v1/write` | POST | Write file |
| `/api/v1/ls` | GET | List directory |
| `/api/v1/stat` | GET | Get file stats |
| `/api/v1/execute` | POST | Execute command |
| `/api/v1/mounts` | GET | List mounts |

### WebSocket

For real-time updates:

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
    ws.send(JSON.stringify({
        type: 'subscribe',
        path: '/s3/'
    }));
};

ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    console.log('File changed:', msg.path);
};
```

## Authentication

```python
# python/mirage/server/auth.py
from fastapi import Depends, HTTPException
from fastapi.security import HTTPBearer

security = HTTPBearer()

async def verify_token(token: str = Depends(security)):
    """Verify JWT token."""
    try:
        payload = jwt.decode(token.credentials, SECRET_KEY)
        return payload['user_id']
    except jwt.InvalidTokenError:
        raise HTTPException(401, "Invalid token")

@app.get("/api/v1/read", dependencies=[Depends(verify_token)])
async def read(path: str):
    # Protected endpoint
    ...
```

## Use Cases

| Mode | Use Case |
|------|----------|
| **FUSE** | Local development, existing tools |
| **Server** | Remote access, multi-user |
| **WebSocket** | Real-time collaboration |

## Next Steps

Continue to [Frameworks →](07-frameworks.html) for AI framework integrations.
