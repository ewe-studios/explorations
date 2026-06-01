---
title: Python SDK
prev: 01-architecture.md
next: 03-typescript-sdk.md
---

# Python SDK

The Python implementation of mirage.

## Project Structure

```
python/
├── mirage/
│   ├── __init__.py          # Public exports
│   ├── core/                # VFS core
│   ├── resource/            # Resource implementations
│   ├── commands/            # Bash command handlers
│   ├── cache/               # Caching layer
│   ├── cli/                 # Command-line interface
│   ├── server/              # HTTP server
│   ├── fuse/                # FUSE integration
│   ├── agents/              # AI framework integrations
│   └── types.py             # Type definitions
├── tests/
└── pyproject.toml
```

## Core Components

### Workspace

**Source:** `python/mirage/core/workspace.py`

The main entry point:

```python
from mirage import Workspace
from mirage.resource import RAMResource, S3Resource

# Create workspace
ws = Workspace({
    '/data': RAMResource(),
    '/s3': S3Resource(
        bucket='my-bucket',
        region='us-east-1'
    ),
})

# Execute commands
result = await ws.execute('ls /s3')
```

**Implementation:**

```python
class Workspace:
    def __init__(self, mounts: Dict[str, Resource]):
        self.vfs = VirtualFileSystem()
        for path, resource in mounts.items():
            self.vfs.mount(path, resource)
        self.dispatcher = CommandDispatcher(self.vfs)
    
    async def execute(self, command: str) -> str:
        """Execute a bash command."""
        return await self.dispatcher.dispatch(command)
    
    def clone(self) -> Workspace:
        """Clone the workspace state."""
        return Workspace(self.vfs.clone_mounts())
    
    async def snapshot(self) -> Snapshot:
        """Create a snapshot."""
        return await Snapshot.create(self.vfs)
```

### VirtualFileSystem

**Source:** `python/mirage/core/vfs.py`

```python
class VirtualFileSystem:
    def __init__(self):
        self._mounts: Dict[str, Resource] = {}
        self._lock = asyncio.Lock()
    
    def mount(self, path: str, resource: Resource) -> None:
        """Mount a resource at the given path."""
        if not path.startswith('/'):
            raise ValueError("Mount path must be absolute")
        self._mounts[path] = resource
    
    async def resolve(self, path: str) -> tuple[Resource, str]:
        """Resolve a path to (resource, relative_path).
        
        Uses longest-prefix matching.
        """
        candidates = [
            (mount, resource)
            for mount, resource in self._mounts.items()
            if path.startswith(mount) or mount == path
        ]
        
        if not candidates:
            raise FileNotFoundError(f"Path not in any mount: {path}")
        
        # Longest match first
        mount, resource = max(candidates, key=lambda x: len(x[0]))
        rel_path = path[len(mount):]
        if not rel_path.startswith('/') and mount != '/':
            rel_path = '/' + rel_path
        
        return resource, rel_path
```

**Aha:** Longest-prefix matching allows nested mounts: `/s3` + `/s3/archive`.

### Resources

**Source:** `python/mirage/resource/`

Base resource class:

```python
class Resource(ABC):
    """Abstract base for all resources."""
    
    def __init__(self):
        self.cache = CacheManager()
    
    @abstractmethod
    async def read(self, path: str) -> bytes:
        """Read file contents."""
        raise NotImplementedError
    
    @abstractmethod
    async def write(self, path: str, data: bytes) -> None:
        """Write file contents."""
        raise NotImplementedError
    
    @abstractmethod
    async def list(self, path: str) -> list[DirEntry]:
        """List directory entries."""
        raise NotImplementedError
    
    @abstractmethod
    async def stat(self, path: str) -> FileStat:
        """Get file statistics."""
        raise NotImplementedError
```

### Commands

**Source:** `python/mirage/commands/`

```python
class CatCommand:
    """cat - concatenate and print files"""
    
    async def execute(self, vfs: VirtualFileSystem, args: list[str]) -> str:
        paths = args
        outputs = []
        
        for path in paths:
            resource, rel_path = vfs.resolve(path)
            data = await resource.read(rel_path)
            outputs.append(data.decode('utf-8'))
        
        return '\n'.join(outputs)

class LsCommand:
    """ls - list directory contents"""
    
    async def execute(self, vfs: VirtualFileSystem, args: list[str]) -> str:
        path = args[0] if args else '.'
        resource, rel_path = vfs.resolve(path)
        entries = await resource.list(rel_path)
        
        return '\n'.join(e.name for e in entries)
```

## Async Pattern

**Aha:** All operations are async to handle I/O-bound resource access:

```python
async def main():
    ws = Workspace({...})
    
    # Concurrent operations
    results = await asyncio.gather(
        ws.execute('cat /s3/file1.txt'),
        ws.execute('cat /s3/file2.txt'),
        ws.execute('ls /slack/general'),
    )
```

## Dependencies

**Source:** `python/pyproject.toml`

Core dependencies:
- `aiofiles>=24.1.0` — Async file I/O
- `aiohttp>=3.13.3` — HTTP client
- `orjson>=3.11` — Fast JSON
- `pyyaml>=6.0.3` — YAML parsing
- `tree-sitter>=0.25.2` — Bash parsing
- `typer>=0.12.0` — CLI framework

Optional extras:
- `[s3]` — `aioboto3>=13.0`
- `[redis]` — `redis[hiredis]>=5.0`
- `[postgres]` — `asyncpg>=0.30.0`
- `[mongodb]` — `motor>=3.7.1`
- `[fuse]` — `mfusepy>=1.0.0`

## Testing

**Source:** `python/tests/`

```python
# tests/test_workspace.py
import pytest
from mirage import Workspace, RAMResource

@pytest.mark.asyncio
async def test_workspace_execute():
    ws = Workspace({'/data': RAMResource()})
    
    await ws.execute('echo hello > /data/test.txt')
    result = await ws.execute('cat /data/test.txt')
    
    assert result == 'hello\n'
```

## Next Steps

Continue to [TypeScript SDK →](03-typescript-sdk.html) for the JavaScript implementation.
