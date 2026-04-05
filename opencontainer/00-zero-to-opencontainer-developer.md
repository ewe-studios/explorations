# Zero to OpenContainer Developer

**Source:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/opencontainer`

This guide takes you from zero knowledge to a working understanding of the OpenWebContainer project — a browser-based virtual container runtime that brings container-like isolation to the web.

---

## Table of Contents

1. [What is OpenWebContainer?](#what-is-openwebcontainer)
2. [Why Browser-Based Containers?](#why-browser-based-containers)
3. [Architecture Overview](#architecture-overview)
4. [Quick Start](#quick-start)
5. [Core Concepts](#core-concepts)
6. [Getting Started with Development](#getting-started-with-development)
7. [Basic Usage Guide](#basic-usage-guide)
8. [Next Steps](#next-steps)

---

## 1. What is OpenWebContainer?

**OpenWebContainer** (also known as **OpenContainer**) is a browser-based virtual container runtime that provides container-like isolation and functionality entirely within the browser environment. It enables running isolated processes, managing virtual filesystems, and executing code without requiring server-side infrastructure.

### Key Features at a Glance

| Feature | Description |
|---------|-------------|
| **Virtual Filesystem** | ZenFS integration with path resolution and layered storage |
| **Process Management** | Shell and Node.js process spawning with full lifecycle control |
| **Shell Environment** | Built-in shell commands (ls, cd, pwd, mkdir, touch, rm, cat, echo, cp, mv) |
| **JavaScript Runtime** | QuickJS integration with ES Modules support and isolated contexts |
| **Network Simulation** | HTTP interceptor and network module mocking |
| **Web Worker Architecture** | Off-main-thread execution for non-blocking operations |

### Use Cases

- **Browser-based IDEs** — Run code directly in the browser with isolated execution
- **Educational Platforms** — Safe, sandboxed code execution for learning
- **Development Sandboxes** — Test code changes without affecting the host system
- **CI/CD in the Browser** — Run build pipelines client-side
- **Interactive Documentation** — Executable examples with filesystem access

---

## 2. Why Browser-Based Containers?

### The Problem

Traditional containerization requires:
- Server infrastructure
- Docker/containerd runtime
- Linux kernel features (namespaces, cgroups)
- Network configuration
- Security hardening

### The OpenWebContainer Solution

Browser-based containers provide:
- **Zero infrastructure** — Runs entirely client-side
- **Inherent security** — Browser sandbox provides isolation
- **Instant startup** — No VM or container boot time
- **Offline capable** — Works without network connectivity
- **Cross-platform** — Same experience on any modern browser

### Comparison

| Aspect | Docker Container | OpenWebContainer |
|--------|-----------------|------------------|
| **Runtime** | Linux kernel | Browser JavaScript engine |
| **Isolation** | Namespaces, cgroups | Browser sandbox, Web Workers |
| **Filesystem** | Overlay filesystem | Virtual FS (ZenFS) |
| **Networking** | Real network interfaces | Simulated/mocked network |
| **Process** | Native processes | Simulated processes |
| **Portability** | Requires Docker | Any modern browser |
| **Security Model** | Kernel-level isolation | Browser security boundaries |

---

## 3. Architecture Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Browser Environment                      │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │                   UI Layer (React/Vue)                  │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │ │
│  │  │  Terminal   │  │  File Tree  │  │  Code Editor│     │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘     │ │
│  └────────────────────────────────────────────────────────┘ │
│                              │                               │
│                              ▼                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │               Container Manager (Main Thread)           │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │ │
│  │  │   Container  │  │   Process    │  │     File     │  │ │
│  │  │    API       │  │   Manager    │  │   Manager    │  │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘  │ │
│  └────────────────────────────────────────────────────────┘ │
│                              │                               │
│                    postMessage API                           │
│                              │                               │
│                              ▼                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              Web Worker (Background Thread)             │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │ │
│  │  │    Shell     │  │    Node.js   │  │   QuickJS    │  │ │
│  │  │  Executor    │  │   Executor   │  │   Runtime    │  │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘  │ │
│  │                                                          │ │
│  │  ┌──────────────────────────────────────────────────┐   │ │
│  │  │           Virtual Filesystem (ZenFS)              │   │ │
│  │  └──────────────────────────────────────────────────┘   │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Component Breakdown

#### 1. UI Layer
- Terminal interface for shell interaction
- File tree explorer
- Code editor with syntax highlighting
- Container status and metrics display

#### 2. Container Manager (Main Thread)
```typescript
interface ContainerManager {
  // Container lifecycle
  createContainer(config: ContainerConfig): Promise<Container>;
  getContainer(id: string): Container | undefined;
  destroyContainer(id: string): Promise<void>;

  // Process management
  spawnProcess(containerId: string, cmd: string, args: string[]): Promise<Process>;
  getProcess(pid: number): Process | undefined;
  killProcess(pid: number, signal?: string): Promise<void>;

  // File operations
  readFile(containerId: string, path: string): Promise<Uint8Array>;
  writeFile(containerId: string, path: string, data: Uint8Array): Promise<void>;
  mkdir(containerId: string, path: string): Promise<void>;
  readdir(containerId: string, path: string): Promise<string[]>;
}
```

#### 3. Web Worker (Background Thread)
- Executes shell commands and processes
- Manages virtual filesystem operations
- Handles JavaScript runtime execution
- Communicates with main thread via `postMessage`

#### 4. Virtual Filesystem (ZenFS)
```typescript
interface VirtualFileSystem {
  // File operations
  readFile(path: string): Promise<Uint8Array>;
  writeFile(path: string, data: Uint8Array): Promise<void>;
  unlink(path: string): Promise<void>;

  // Directory operations
  mkdir(path: string): Promise<void>;
  readdir(path: string): Promise<string[]>;
  rmdir(path: string): Promise<void>;

  // Metadata
  stat(path: string): Promise<FileStat>;
  chmod(path: string, mode: number): Promise<void>;
  utimes(path: string, atime: Date, mtime: Date): Promise<void>;
}
```

---

## 4. Quick Start

### Prerequisites

- **Node.js** 16+ (LTS recommended)
- **pnpm** 8+ (package manager)
- Modern browser with Web Worker support

### Installation

```bash
# Clone the repository
git clone https://github.com/OpenWebContainer/opencontainer.git
cd opencontainer

# Install dependencies
pnpm install

# Build all packages
pnpm build

# Start development server
pnpm dev
```

### Project Structure

```
opencontainer/
├── package.json              # Root package configuration
├── pnpm-workspace.yaml       # Monorepo workspace definition
├── tsconfig.base.json        # Shared TypeScript configuration
├── packages/
│   ├── core/                 # Core container runtime
│   │   ├── package.json
│   │   ├── src/
│   │   │   ├── index.ts      # Public API exports
│   │   │   ├── container.ts  # Container class
│   │   │   ├── process/      # Process management
│   │   │   │   ├── manager.ts
│   │   │   │   ├── shell.ts
│   │   │   │   └── node.ts
│   │   │   ├── filesystem/   # Virtual filesystem
│   │   │   │   ├── vfs.ts
│   │   │   │   └── zenfs-adapter.ts
│   │   │   ├── worker/       # Web worker communication
│   │   │   │   ├── worker.ts
│   │   │   │   └── bridge.ts
│   │   │   └── runtime/      # JavaScript runtime
│   │   │       ├── quickjs.ts
│   │   │       └── esm-loader.ts
│   │   └── tests/
│   │
│   ├── api/                  # Public API layer
│   │   ├── package.json
│   │   └── src/
│   │       ├── index.ts
│   │       ├── client.ts     # Browser client
│   │       └── types.ts      # TypeScript types
│   │
│   └── playground/           # Example application
│       ├── package.json
│       ├── src/
│       │   ├── App.tsx       # React application
│       │   ├── components/   # UI components
│       │   └── examples/     # Usage examples
│       └── index.html
│
└── apps/
    └── playground/           # Development/test application
        ├── package.json
        ├── vite.config.ts
        └── src/
            ├── main.tsx
            └── App.tsx
```

### Development Setup

```bash
# Install dependencies
pnpm install

# Start development server (playground app)
cd apps/playground
pnpm dev

# Run tests
pnpm test

# Build for production
pnpm build

# Type check
pnpm typecheck
```

---

## 5. Core Concepts

### 5.1 Virtual File System

OpenWebContainer uses **ZenFS** as the underlying virtual filesystem layer, providing a POSIX-like filesystem API in the browser.

#### Path Resolution

```typescript
// Paths are resolved relative to container root
const container = await Container.create();

// Absolute path (relative to container root)
await container.fs.writeFile('/app/index.js', code);

// Relative path (resolved from current working directory)
await container.process.chdir('/app');
await container.fs.writeFile('./utils.js', utilsCode);

// Path normalization
// '/app/../app/./index.js' -> '/app/index.js'
```

#### Filesystem Backends

```typescript
// In-memory backend (default)
const fs = await VirtualFS.create({
  backend: 'memory',
});

// IndexedDB backend (persistent across sessions)
const fs = await VirtualFS.create({
  backend: 'indexeddb',
  storeName: 'my-container-fs',
});

// Overlay backend (layered filesystem)
const fs = await VirtualFS.create({
  backend: 'overlay',
  layers: [readOnlyLayer, writableLayer],
});
```

#### ZenFS Integration

```typescript
import { configure, fs } from '@zenfs/core';

// Configure ZenFS for container
await configure({
  backend: 'MountableFileSystem',
  mounts: {
    '/': {
      backend: 'InMemory',
    },
    '/readonly': {
      backend: 'Fetch',
      baseUrl: '/assets/base-image/',
    },
  },
});

// Now fs behaves like Node.js fs module
const data = await fs.promises.readFile('/app/index.js', 'utf8');
```

### 5.2 Process Types

OpenWebContainer supports two main process types:

#### Shell Processes

```typescript
// Spawn a shell process
const shell = await container.spawn('sh', {
  cwd: '/app',
  env: {
    PATH: '/bin:/usr/bin',
    HOME: '/home/user',
  },
});

// Send commands
shell.write('ls -la\n');
shell.write('cd /app\n');

// Read output
shell.on('data', (data: string) => {
  console.log('Shell output:', data);
});

// Handle exit
shell.on('exit', (code: number) => {
  console.log('Shell exited with code:', code);
});
```

#### Node.js Processes

```typescript
// Spawn a Node.js process
const node = await container.spawn('node', ['script.js'], {
  cwd: '/app',
  stdio: 'pipe',
});

// Stream output
node.stdout.on('data', (data: Uint8Array) => {
  console.log('stdout:', new TextDecoder().decode(data));
});

node.stderr.on('data', (data: Uint8Array) => {
  console.error('stderr:', new TextDecoder().decode(data));
});

// Wait for completion
const exitCode = await node.exitCode;
```

### 5.3 Container API Abstraction

The Container API provides a high-level interface for all container operations:

```typescript
import { Container } from '@opencontainer/core';

// Create container
const container = await Container.create({
  name: 'my-container',
  image: 'node:18-alpine',
});

// Filesystem operations
await container.fs.writeFile('/app/package.json', pkgJson);
await container.fs.mkdir('/app/src');
const files = await container.fs.readdir('/app');

// Process execution
const result = await container.exec('npm install');
console.log('Install output:', result.stdout);

// Long-running process
const server = await container.spawn('node', ['server.js']);
server.stdout.pipe(process.stdout);

// Cleanup
await container.destroy();
```

### 5.4 Worker Communication

Main thread and Web Worker communicate via `postMessage`:

```typescript
// Main thread -> Worker
worker.postMessage({
  type: 'CREATE_CONTAINER',
  id: 'container-1',
  config: { /* ... */ },
});

// Worker -> Main thread
self.postMessage({
  type: 'CONTAINER_READY',
  id: 'container-1',
});

// Message types
type WorkerMessage =
  | { type: 'CREATE_CONTAINER'; id: string; config: ContainerConfig }
  | { type: 'SPAWN_PROCESS'; containerId: string; cmd: string; args: string[] }
  | { type: 'WRITE_FILE'; containerId: string; path: string; data: Uint8Array }
  | { type: 'PROCESS_OUTPUT'; pid: number; data: Uint8Array; stream: 'stdout' | 'stderr' }
  | { type: 'PROCESS_EXIT'; pid: number; code: number };
```

---

## 6. Getting Started with Development

### 6.1 Development Environment

```bash
# Clone and install
git clone https://github.com/OpenWebContainer/opencontainer.git
cd opencontainer
pnpm install

# Start development
pnpm dev

# In another terminal, run tests
pnpm test --watch
```

### 6.2 Package Breakdown

#### @opencontainer/core

The core runtime package:

```typescript
// packages/core/src/index.ts
export { Container } from './container';
export { ContainerManager } from './container-manager';
export { Process, ProcessManager } from './process';
export { VirtualFileSystem } from './filesystem';
export { ShellExecutor } from './executors/shell';
export { NodeExecutor } from './executors/node';
export { QuickJSRuntime } from './runtime/quickjs';
export type * from './types';
```

#### @opencontainer/api

The public API layer:

```typescript
// packages/api/src/index.ts
export { createContainer, ContainerClient } from './client';
export type {
  ContainerConfig,
  ProcessOptions,
  FileSystemAPI,
  ContainerEvents,
} from './types';
```

#### apps/playground

The example application:

```tsx
// apps/playground/src/App.tsx
import { createContainer } from '@opencontainer/api';
import { Terminal } from './components/Terminal';
import { FileTree } from './components/FileTree';

function App() {
  const [container, setContainer] = useState(null);

  useEffect(() => {
    async function init() {
      const c = await createContainer({ name: 'playground' });
      setContainer(c);
    }
    init();
  }, []);

  return (
    <div className="app">
      <FileTree container={container} />
      <Terminal container={container} />
    </div>
  );
}
```

### 6.3 Building and Testing

```bash
# Build all packages
pnpm build

# Test individual package
cd packages/core
pnpm test

# Run integration tests
pnpm test:integration

# Type check
pnpm typecheck

# Lint
pnpm lint
```

---

## 7. Basic Usage Guide

### 7.1 Container Creation

```typescript
import { Container } from '@opencontainer/core';

// Basic container
const container = await Container.create();

// Container with configuration
const container = await Container.create({
  name: 'my-app',
  cwd: '/app',
  env: {
    NODE_ENV: 'development',
    PORT: '3000',
  },
  filesystem: {
    backend: 'memory',
    mounts: {
      '/app': { backend: 'InMemory' },
    },
  },
});

// Container from image
const container = await Container.create({
  image: 'node:18-alpine',
  snapshot: 'base-image-v1',
});
```

### 7.2 Filesystem Operations

```typescript
// Write a file
await container.fs.writeFile('/app/index.js', `
  console.log('Hello from container!');
`);

// Read a file
const content = await container.fs.readFile('/app/index.js', 'utf8');
console.log(content);

// Create directory
await container.fs.mkdir('/app/src');
await container.fs.mkdir('/app/src/utils', { recursive: true });

// List directory
const files = await container.fs.readdir('/app');
console.log('Files:', files);

// Copy file
await container.fs.cp('/app/index.js', '/app/src/copy.js');

// Move/rename file
await container.fs.mv('/app/src/copy.js', '/app/src/renamed.js');

// Delete file
await container.fs.rm('/app/src/renamed.js');

// Delete directory
await container.fs.rm('/app/src', { recursive: true });

// Get file stats
const stat = await container.fs.stat('/app/index.js');
console.log('Size:', stat.size);
console.log('Modified:', stat.mtime);
```

### 7.3 Process Spawning

#### Shell Process

```typescript
// Create interactive shell
const shell = await container.spawn('sh', ['-i'], {
  cwd: '/app',
  env: { PATH: '/bin:/usr/bin' },
});

// Send commands
shell.write('pwd\n');
shell.write('ls -la\n');
shell.write('echo "Hello World"\n');

// Handle output
shell.on('data', (data: string) => {
  process.stdout.write(data);
});

// Handle exit
shell.on('exit', (code: number, signal: string) => {
  console.log(`Shell exited: code=${code}, signal=${signal}`);
});

// Kill shell
shell.kill('SIGTERM');
```

#### Node.js Process

```typescript
// Run Node.js script
const node = await container.spawn('node', ['script.js'], {
  cwd: '/app',
  stdio: ['pipe', 'pipe', 'pipe'],
});

// Handle stdout
node.stdout.on('data', (data: Uint8Array) => {
  console.log('Output:', data.toString());
});

// Handle stderr
node.stderr.on('data', (data: Uint8Array) => {
  console.error('Error:', data.toString());
});

// Wait for exit
const exitCode = await node.exit;
console.log('Exit code:', exitCode);

// Send stdin
node.stdin.write('some input\n');
```

### 7.4 Event Handling

```typescript
import { Container } from '@opencontainer/core';

const container = await Container.create();

// Container events
container.on('ready', () => {
  console.log('Container is ready');
});

container.on('filesystem:change', (path: string, type: 'create' | 'modify' | 'delete') => {
  console.log(`File ${type}: ${path}`);
});

container.on('process:spawn', (pid: number, cmd: string) => {
  console.log(`Process spawned: PID ${pid}, cmd: ${cmd}`);
});

container.on('process:exit', (pid: number, code: number) => {
  console.log(`Process exited: PID ${pid}, code: ${code}`);
});

container.on('error', (error: Error) => {
  console.error('Container error:', error);
});

// Process events
const process = await container.spawn('node', ['app.js']);

process.on('spawn', () => {
  console.log('Process started');
});

process.on('data', (data: Uint8Array, stream: 'stdout' | 'stderr') => {
  console.log(`${stream}:`, data.toString());
});

process.on('exit', (code: number, signal: string) => {
  console.log(`Process ended: code=${code}, signal=${signal}`);
});
```

---

## 8. Shell Commands

OpenWebContainer provides a set of built-in shell commands that work within the virtual filesystem:

### 8.1 File Operations

```bash
# List directory contents
ls
ls -la          # Long format, all files
ls /app/src     # Specific directory

# Change directory
cd /app
cd ..
cd ~            # Home directory
cd -            # Previous directory

# Print working directory
pwd

# Create directory
mkdir new-folder
mkdir -p path/to/nested/folder

# Create empty file
touch newfile.txt
touch file1 file2 file3

# Remove files/directories
rm file.txt
rm -rf folder   # Recursive delete

# Copy files
cp source.txt dest.txt
cp -r folder1 folder2

# Move/rename files
mv old.txt new.txt
mv file.txt folder/
```

### 8.2 File Content

```bash
# Display file content
cat file.txt
cat file1.txt file2.txt

# Display file content with line numbers
cat -n file.txt

# Echo text
echo "Hello World"
echo $PATH      # Environment variable

# View file (pager)
less file.txt

# Head/tail
head -n 10 file.txt
tail -n 20 file.txt
```

### 8.3 File Redirection

```bash
# Redirect stdout to file
echo "Hello" > output.txt

# Append to file
echo "World" >> output.txt

# Redirect stderr
node script.js 2> error.log

# Redirect both stdout and stderr
node script.js > output.log 2>&1

# Pipe commands
ls -la | grep ".js"
cat file.txt | head -n 5
```

### 8.4 Environment

```bash
# Print environment
env
printenv

# Set environment variable
export NODE_ENV=production
export PORT=3000

# View specific variable
echo $PATH

# Unset variable
unset VARIABLE_NAME
```

### 8.5 Shell Built-ins Implementation

```typescript
// packages/core/src/executors/shell/builtins.ts
export const builtins: Record<string, BuiltinCommand> = {
  ls: async (args, ctx) => {
    const path = args[0] || ctx.cwd;
    const entries = await ctx.fs.readdir(path);
    // Format and display entries
  },

  cd: async (args, ctx) => {
    const target = args[0] || ctx.home;
    ctx.cwd = resolvePath(ctx.cwd, target);
  },

  pwd: async (args, ctx) => {
    console.log(ctx.cwd);
  },

  mkdir: async (args, ctx) => {
    const recursive = args.includes('-p') || args.includes('--parents');
    const path = args.find(a => !a.startsWith('-'));
    await ctx.fs.mkdir(path, { recursive });
  },

  touch: async (args, ctx) => {
    for (const file of args) {
      if (!await ctx.fs.exists(file)) {
        await ctx.fs.writeFile(file, '');
      }
    }
  },

  rm: async (args, ctx) => {
    const recursive = args.includes('-r') || args.includes('-R');
    const force = args.includes('-f') || args.includes('--force');
    const paths = args.filter(a => !a.startsWith('-'));
    for (const path of paths) {
      await ctx.fs.rm(path, { recursive, force });
    }
  },

  cat: async (args, ctx) => {
    for (const file of args) {
      const content = await ctx.fs.readFile(file, 'utf8');
      console.log(content);
    }
  },

  cp: async (args, ctx) => {
    const recursive = args.includes('-r');
    const paths = args.filter(a => !a.startsWith('-'));
    const [src, dest] = paths;
    await ctx.fs.cp(src, dest, { recursive });
  },

  mv: async (args, ctx) => {
    const [src, dest] = args.filter(a => !a.startsWith('-'));
    await ctx.fs.mv(src, dest);
  },
};
```

---

## 9. Process Management

### 9.1 Process Lifecycle

```typescript
// Process states
enum ProcessState {
  PENDING = 'pending',
  RUNNING = 'running',
  STOPPED = 'stopped',
  EXITED = 'exited',
  ERROR = 'error',
}

// Lifecycle events
const process = await container.spawn('node', ['app.js']);

// Start event
process.on('start', () => {
  console.log('Process started with PID:', process.pid);
});

// Data events
process.on('stdout', (data: Uint8Array) => {
  console.log('Output:', data.toString());
});

process.on('stderr', (data: Uint8Array) => {
  console.error('Error:', data.toString());
});

// Exit event
process.on('exit', (code: number, signal: string) => {
  console.log(`Process exited: code=${code}, signal=${signal}`);
});

// Error event
process.on('error', (error: Error) => {
  console.error('Process error:', error);
});
```

### 9.2 Process Types and Executors

```typescript
// Process executor interface
interface ProcessExecutor {
  canExecute(cmd: string): boolean;
  execute(cmd: string, args: string[], options: ProcessOptions): Promise<Process>;
}

// Shell executor
class ShellExecutor implements ProcessExecutor {
  canExecute(cmd: string): boolean {
    return ['sh', 'bash', 'zsh'].includes(cmd);
  }

  async execute(cmd: string, args: string[], options: ProcessOptions): Promise<Process> {
    // Spawn shell process in worker
    return new ShellProcess(cmd, args, options);
  }
}

// Node.js executor
class NodeExecutor implements ProcessExecutor {
  canExecute(cmd: string): boolean {
    return cmd === 'node';
  }

  async execute(cmd: string, args: string[], options: ProcessOptions): Promise<Process> {
    // Execute JavaScript with QuickJS or bundled Node.js shim
    return new NodeProcess(args, options);
  }
}

// Process manager
class ProcessManager {
  private executors: ProcessExecutor[] = [];
  private processes: Map<number, Process> = new Map();

  registerExecutor(executor: ProcessExecutor): void {
    this.executors.push(executor);
  }

  async spawn(cmd: string, args: string[], options: ProcessOptions): Promise<Process> {
    const executor = this.executors.find(e => e.canExecute(cmd));
    if (!executor) {
      throw new Error(`No executor found for command: ${cmd}`);
    }

    const process = await executor.execute(cmd, args, options);
    this.processes.set(process.pid, process);

    process.on('exit', () => {
      this.processes.delete(process.pid);
    });

    return process;
  }

  getProcess(pid: number): Process | undefined {
    return this.processes.get(pid);
  }

  kill(pid: number, signal?: string): Promise<void> {
    const process = this.getProcess(pid);
    if (!process) {
      throw new Error(`Process not found: ${pid}`);
    }
    return process.kill(signal);
  }
}
```

### 9.3 Process Manager

```typescript
// packages/core/src/process/manager.ts
export class ProcessManager {
  private processes: Map<number, Process> = new Map();
  private nextPid: number = 1;

  // Spawn new process
  spawn(cmd: string, args: string[], options: ProcessOptions): Process {
    const pid = this.nextPid++;
    const process = new Process(pid, cmd, args, options);
    this.processes.set(pid, process);

    // Cleanup on exit
    process.on('exit', () => {
      this.processes.delete(pid);
    });

    return process;
  }

  // Get process by PID
  get(pid: number): Process | undefined {
    return this.processes.get(pid);
  }

  // List all processes
  list(): Process[] {
    return Array.from(this.processes.values());
  }

  // Kill process
  async kill(pid: number, signal: string = 'SIGTERM'): Promise<void> {
    const process = this.get(pid);
    if (process) {
      await process.kill(signal);
    }
  }

  // Kill all processes
  async killAll(): Promise<void> {
    await Promise.all(
      this.processes.values().map(p => p.kill('SIGTERM'))
    );
  }
}
```

---

## 10. JavaScript Runtime

### 10.1 QuickJS Integration

OpenWebContainer uses QuickJS for isolated JavaScript execution:

```typescript
import { QuickJSRuntime } from '@opencontainer/core';

// Create runtime
const runtime = await QuickJSRuntime.create();

// Execute code
const result = await runtime.execute(`
  const add = (a, b) => a + b;
  add(2, 3);
`);

console.log('Result:', result); // 5

// Expose functions to runtime
runtime.expose('fetch', async (url: string) => {
  // Mock fetch implementation
  return { ok: true, json: () => ({ data: 'mocked' }) };
});

// Execute with exposed functions
await runtime.execute(`
  const response = await fetch('/api/data');
  console.log(response.json());
`);

// Cleanup
await runtime.dispose();
```

### 10.2 ES Modules Support

```typescript
// Enable ES Modules
const runtime = await QuickJSRuntime.create({
  modules: {
    enable: true,
    loader: {
      // Custom module loader
      resolve(specifier: string, referrer: string): string {
        // Resolve module path
        return resolveModulePath(specifier, referrer);
      },

      load(resolved: string): Promise<string> {
        // Load module source
        return container.fs.readFile(resolved, 'utf8');
      },
    },
  },
});

// Execute module
await runtime.executeModule('/app/index.js');

// Import in executed code
await runtime.execute(`
  import { add } from './utils.js';
  console.log(add(2, 3));
`);
```

### 10.3 Isolated Contexts

```typescript
// Create isolated context
const context = runtime.createContext({
  name: 'sandbox',
  globals: {
    console: {
      log: (...args: any[]) => console.log('[sandbox]', ...args),
      error: (...args: any[]) => console.error('[sandbox]', ...args),
    },
  },
  // Restrict access
  denyBuiltins: ['fs', 'child_process', 'net'],
});

// Execute in context
await context.execute(`
  // This code runs in isolation
  console.log('Hello from sandbox');

  // Cannot access denied builtins
  // require('fs') would throw
`);
```

---

## 11. Network Simulation

### 11.1 HTTP Interceptor

```typescript
import { NetworkMocker } from '@opencontainer/core';

// Create network mocker
const network = new NetworkMocker();

// Intercept requests
network.intercept('GET', '/api/users', () => {
  return {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify([{ id: 1, name: 'John' }]),
  };
});

// Intercept with pattern
network.intercept('POST', /\/api\/users\/\d+/, (request) => {
  const userId = request.url.match(/\/users\/(\d+)/)[1];
  return {
    status: 200,
    body: JSON.stringify({ id: userId, updated: true }),
  };
});

// Mock delays
network.intercept('GET', '/api/slow', () => {
  return new Promise(resolve => {
    setTimeout(() => {
      resolve({
        status: 200,
        body: JSON.stringify({ data: 'delayed response' }),
      });
    }, 1000);
  });
});

// Enable interception
network.enable();

// Disable interception
network.disable();
```

### 11.2 Network Module Mocking

```typescript
// Mock net module
container.runtime.mock('net', {
  createConnection: (options: any) => {
    return {
      write: (data: any) => {},
      end: () => {},
      on: (event: string, cb: Function) => {},
    };
  },
  createServer: () => {
    return {
      listen: () => {},
      close: () => {},
      on: (event: string, cb: Function) => {},
    };
  },
});

// Mock http module
container.runtime.mock('http', {
  createServer: (handler: Function) => {
    return {
      listen: (port: number) => {
        console.log(`Server listening on port ${port}`);
      },
      close: () => {},
    };
  },
  request: (url: string, options: any) => {
    // Return mocked response
    return {
      write: () => {},
      end: () => {},
      on: (event: string, cb: Function) => {},
    };
  },
});

// Mock https module
container.runtime.mock('https', {
  request: (url: string, options: any) => {
    // Return mocked response
    return {
      write: () => {},
      end: () => {},
      on: (event: string, cb: Function) => {},
    };
  },
});
```

### 11.3 Fetch Mocking

```typescript
// Mock global fetch
container.runtime.expose('fetch', async (url: string, options: any) => {
  // Check mock registry
  const mock = network.getMock('FETCH', url);
  if (mock) {
    return mock.response;
  }

  // Default behavior
  return {
    ok: false,
    status: 404,
    json: () => Promise.resolve({ error: 'Not found' }),
    text: () => Promise.resolve('Not found'),
  };
});
```

---

## 12. Advanced Patterns

### 12.1 Container Pooling

```typescript
class ContainerPool {
  private containers: Map<string, Container> = new Map();
  private maxPoolSize: number = 5;

  async acquire(config: ContainerConfig): Promise<Container> {
    const key = JSON.stringify(config);

    // Check pool
    if (this.containers.has(key)) {
      return this.containers.get(key)!;
    }

    // Create new container
    const container = await Container.create(config);

    // Add to pool
    if (this.containers.size < this.maxPoolSize) {
      this.containers.set(key, container);
    }

    return container;
  }

  async release(container: Container): Promise<void> {
    // Reset container state
    await container.reset();

    // Return to pool (already in pool, just reset)
  }

  async destroy(): Promise<void> {
    await Promise.all(
      Array.from(this.containers.values()).map(c => c.destroy())
    );
    this.containers.clear();
  }
}
```

### 12.2 Snapshot and Restore

```typescript
// Create snapshot
const snapshot = await container.snapshot();

// Save snapshot
await container.fs.writeFile('/snapshots/state.json', JSON.stringify(snapshot));

// Restore from snapshot
const savedState = JSON.parse(
  await container.fs.readFile('/snapshots/state.json', 'utf8')
);
await container.restore(savedState);
```

### 12.3 Container Orchestration

```typescript
// Multi-container setup
const apiContainer = await Container.create({ name: 'api' });
const dbContainer = await Container.create({ name: 'db' });
const workerContainer = await Container.create({ name: 'worker' });

// Link containers
await apiContainer.link(dbContainer, { alias: 'database' });
await workerContainer.link(apiContainer, { alias: 'api' });

// Start all
await Promise.all([
  apiContainer.spawn('node', ['server.js']),
  dbContainer.spawn('node', ['database.js']),
  workerContainer.spawn('node', ['worker.js']),
]);
```

---

## 13. Debugging and Troubleshooting

### 13.1 Debug Mode

```typescript
const container = await Container.create({
  debug: true,
  logger: {
    debug: (...args: any[]) => console.debug('[container:debug]', ...args),
    info: (...args: any[]) => console.info('[container:info]', ...args),
    warn: (...args: any[]) => console.warn('[container:warn]', ...args),
    error: (...args: any[]) => console.error('[container:error]', ...args),
  },
});
```

### 13.2 Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Process won't start | Missing executor | Register executor for command |
| File not found | Wrong path resolution | Use absolute paths or set cwd |
| Memory limit exceeded | Large file operations | Use streaming or chunked operations |
| Worker communication failed | postMessage error | Check message serialization |
| Network mock not working | Wrong URL pattern | Verify regex pattern matches |

---

## Next Steps

Now that you understand the basics, dive deeper:

1. **[Architecture Deep Dive](architecture-deep-dive.md)** — Detailed breakdown of internal architecture
2. **[Virtual Filesystem Guide](virtual-fs-deep-dive.md)** — ZenFS integration and custom backends
3. **[Process Management](process-management-deep-dive.md)** — Advanced process orchestration
4. **[JavaScript Runtime](javascript-runtime-deep-dive.md)** — QuickJS and ES Modules internals
5. **[Network Simulation](network-simulation-deep-dive.md)** — HTTP interceptor and mocking
6. **[Production Guide](production-grade.md)** — Building for production deployment

---

## Glossary

| Term | Definition |
|------|------------|
| **Container** | Isolated runtime environment with virtual filesystem and processes |
| **Virtual Filesystem (VFS)** | In-browser filesystem abstraction layer |
| **ZenFS** | JavaScript filesystem module used as VFS backend |
| **Web Worker** | Background thread for non-blocking operations |
| **Executor** | Component that runs specific process types (shell, node) |
| **QuickJS** | Lightweight JavaScript engine for isolated execution |
| **Process** | Simulated running command with stdin/stdout/stderr |
| **Network Mock** | Simulated network layer for testing |

---

## Resources

- **Source Repository:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/opencontainer`
- **ZenFS Documentation:** https://zenfs.org
- **QuickJS Documentation:** https://bellard.org/quickjs/

---

*Generated: 2026-04-05*
