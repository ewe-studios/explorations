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

## Additional Modules

### Workspace Module

**Source:** `python/mirage/workspace/`

Workspace management and lifecycle:

```python
# python/mirage/workspace/manager.py
from mirage.workspace import WorkspaceManager

manager = WorkspaceManager()

# Create workspace
ws = manager.create({
    '/data': RAMResource(),
    '/s3': S3Resource(...),
})

# Clone workspace
ws2 = manager.clone(ws.id)

# Snapshot
snapshot = await manager.snapshot(ws.id)

# Restore
restored = await manager.restore(snapshot.id)
```

### Runtime Module

**Source:** `python/mirage/runtime/`

Runtime context and execution state:

```python
# python/mirage/runtime/context.py
from mirage.runtime import RuntimeContext

ctx = RuntimeContext()
ctx.chdir('/s3')
ctx.setenv('AWS_REGION', 'us-east-1')

# Use in workspace
ws = Workspace({...}, runtime_context=ctx)
```

### Accessor Module

**Source:** `python/mirage/accessor/`

File access patterns:

```python
# python/mirage/accessor/reader.py
from mirage.accessor import FileAccessor

accessor = FileAccessor(vfs)

# Stream read
async for chunk in accessor.read_stream('/s3/large-file.bin'):
    process(chunk)

# Batch read
results = await accessor.batch_read(['/s3/a.txt', '/s3/b.txt'])
```

### Bridge Module

**Source:** `python/mirage/bridge/`

Cross-language communication:

```python
# python/mirage/bridge/client.py
from mirage.bridge import BridgeClient

bridge = BridgeClient('ws://localhost:8080')
await bridge.connect()

# Call TypeScript function from Python
result = await bridge.call('typescriptFunction', ['arg1', 'arg2'])
```

### Observe Module

**Source:** `python/mirage/observe/`

Observability and metrics:

```python
# python/mirage/observe/telemetry.py
from mirage.observe import TelemetryCollector

telemetry = TelemetryCollector()

# Record metrics
telemetry.record('vfs.read', 1.0, {'resource': 's3'})
telemetry.record('vfs.write', 2.5, {'resource': 'gcs'})

# Export to collector
await telemetry.export()
```

### Ops Module

**Source:** `python/mirage/ops/`

Batch operations:

```python
# python/mirage/ops/batch.py
from mirage.ops import BatchOperations

ops = BatchOperations(vfs)

# Batch copy
await ops.batch_copy([
    ('/s3/a.txt', '/gcs/a.txt'),
    ('/s3/b.txt', '/gcs/b.txt'),
])

# Batch delete
await ops.batch_delete(['/s3/old1.txt', '/s3/old2.txt'])
```

### Provision Module

**Source:** `python/mirage/provision/`

Resource provisioning:

```python
# python/mirage/provision/factory.py
from mirage.provision import ResourceFactory

factory = ResourceFactory()
factory.register('s3', S3Provider())

# Provision resource
resource = await factory.provision('s3', {
    'bucket': 'my-bucket',
    'region': 'us-east-1',
})
```

### IO Module

**Source:** `python/mirage/io/`

I/O utilities:

```python
# python/mirage/io/streams.py
from mirage.io import AsyncStream

# Create stream from iterator
async def data_generator():
    for i in range(10):
        yield f"chunk {i}\n".encode()

stream = AsyncStream(data_generator())
data = await stream.read()
```

### Shell Module

**Source:** `python/mirage/shell/`

Shell command execution:

```python
# python/mirage/shell/executor.py
from mirage.shell import ShellExecutor

executor = ShellExecutor()

# Execute shell command
result = await executor.execute('echo hello')
print(result.stdout)  # 'hello\n'
print(result.returncode)  # 0
```

### VFP Module

**Source:** `python/mirage/vfp/`

Virtual File Protocol:

```python
# python/mirage/vfp/protocol.py
from mirage.vfp import VFPClient

client = VFPClient('localhost:9090')
await client.connect()

# Send VFP message
response = await client.send({
    'op': 'READ',
    'path': '/s3/file.txt',
})
```

### Utils Module

**Source:** `python/mirage/utils/`

Utility functions:

```python
# python/mirage/utils/async_.py
from mirage.utils import gather_with_limit

# Gather with concurrency limit
tasks = [fetch_data(i) for i in range(100)]
results = await gather_with_limit(tasks, limit=10)

# python/mirage/utils/path.py
from mirage.utils import normalize_path

path = normalize_path('/s3//file.txt')  # '/s3/file.txt'
```

## Command Structure

**Source:** `python/mirage/commands/`

```
commands/
├── __init__.py
├── builtin/              # Built-in commands
│   ├── cat.py
│   ├── ls.py
│   ├── cp.py
│   ├── mv.py
│   ├── rm.py
│   ├── grep.py
│   ├── find.py
│   ├── wc.py
│   ├── sort.py
│   ├── uniq.py
│   ├── head.py
│   ├── tail.py
│   ├── echo.py
│   └── pwd.py
├── config.py             # Command configuration
├── local_audio/          # Audio commands
│   └── transcribe.py
├── optional.py           # Optional commands
├── registry.py           # Command registry
├── resolve.py            # Path resolution
├── safeguard.py          # Safety checks
├── spec/                 # Command specifications
│   ├── cat.yml
│   ├── ls.yml
│   └── ...
└── types.py              # Type definitions
```

### Command Registry

```python
# python/mirage/commands/registry.py
class CommandRegistry:
    def __init__(self):
        self.commands: Dict[str, Command] = {}
    
    def register(self, name: str, cmd: Command):
        """Register command."""
        self.commands[name] = cmd
    
    def get(self, name: str) -> Command:
        """Get command by name."""
        return self.commands[name]
    
    def list_builtin(self) -> list[str]:
        """List built-in commands."""
        return list(self.commands.keys())
```

### Adding Custom Commands

```python
# python/mirage/commands/custom.py
from mirage.commands import Command, registry

class MyCommand(Command):
    name = 'mycommand'
    description = 'My custom command'
    
    async def execute(self, vfs, args):
        # Implementation
        return 'result'

# Register
registry.register('mycommand', MyCommand())
```

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
