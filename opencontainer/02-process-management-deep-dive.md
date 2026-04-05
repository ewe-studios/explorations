# Process Management in OpenWebContainer: A Comprehensive Deep-Dive

**Location:** `/home/darkvoid/Boxxed/@formulas/src.opencontainer/OpenWebContainer/packages/core/src/process/`

This document provides an exhaustive analysis of the process management system in OpenWebContainer, covering the architecture, implementations, and production-ready patterns used throughout the codebase.

---

## Table of Contents

1. [Process Architecture Overview](#1-process-architecture-overview)
2. [BaseProcess Class Structure](#2-baseprocess-class-structure)
3. [Process Interface (IProcess)](#3-process-interface-iprocess)
4. [Process State Machine](#4-process-state-machine)
5. [Process Executors](#5-process-executors)
6. [Process Lifecycle](#6-process-lifecycle)
7. [Process Events](#7-process-events)
8. [Process Manager](#8-process-manager)
9. [Shell Process Implementation](#9-shell-process-implementation)
10. [Node Process Implementation](#10-node-process-implementation)
11. [Inter-Process Communication](#11-inter-process-communication)
12. [Network Module for Node Processes](#12-network-module-for-node-processes)

---

## 1. Process Architecture Overview

The process management system in OpenWebContainer follows a layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────────────┐
│                        OpenWebContainer                              │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    ProcessManager                            │    │
│  │  - Map<PID, Process>                                         │    │
│  │  - spawn(), kill(), getAll()                                 │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                              │                                       │
│         ┌────────────────────┼────────────────────┐                 │
│         │                    │                    │                 │
│         ▼                    ▼                    ▼                 │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐           │
│  │ShellProcess │     │ NodeProcess │     │  (Future)   │           │
│  └─────────────┘     └─────────────┘     └─────────────┘           │
│         │                    │                                     │
│         └────────────────────┼────────────────────┐                 │
│                              │                    │                 │
│                   ┌──────────▼────────┐          │                 │
│                   │  ProcessRegistry  │          │                 │
│                   │  - Executor Map   │          │                 │
│                   │  - canExecute()   │          │                 │
│                   │  - execute()      │          │                 │
│                   └───────────────────┘          │                 │
│                              │                    │                 │
│                   ┌──────────▼────────┐          │                 │
│                   │ ProcessExecutors  │          │                 │
│                   │ - ShellExecutor   │          │                 │
│                   │ - NodeExecutor    │          │                 │
│                   └───────────────────┘          │                 │
└─────────────────────────────────────────────────────────────────────┘
```

### Key Design Patterns

1. **Strategy Pattern**: Process executors implement a common interface allowing interchangeable execution strategies
2. **Factory Pattern**: Executors create specific process types based on executable
3. **Observer Pattern**: Event emitter for process lifecycle events
4. **Template Method Pattern**: Base `Process` class defines lifecycle, subclasses implement `execute()`

---

## 2. BaseProcess Class Structure

### File: `packages/core/src/process/base/process.ts`

The `Process` class is the foundation for all process types, providing:
- Process identification (PID)
- State management
- Event emission
- Input/output buffering
- Lifecycle hooks

```typescript
export abstract class Process extends BrowserEventEmitter {
    readonly pid: number;
    readonly type: ProcessType;
    protected _state: ProcessState;
    protected _exitCode: number | null;
    protected env: Map<string, string> = new Map();
    readonly executablePath: string;
    readonly args: string[];
    readonly parentPid?: number|undefined;
    readonly cwd?: string;

    private inputBuffer: string[] = [];
    private inputCallbacks: ((input: string) => void)[] = [];
    private startTime?: Date;
    private endTime?: Date;
    private terminated: boolean = false;

    constructor(
        pid: number,
        type: ProcessType,
        executablePath: string,
        args: string[] = [],
        parentPid?: number,
        cwd?: string,
        env?: Map<string, string>
    ) {
        super();
        this.pid = pid;
        this.type = type;
        this._state = ProcessState.CREATED;
        this._exitCode = null;
        this.executablePath = executablePath;
        this.args = args;
        this.parentPid = parentPid;
        this.cwd = cwd||'/';
        this.env = env || new Map([
            ['PATH', '/bin:/usr/bin'],
            ['HOME', '/home'],
            ['PWD', cwd||'/'],
        ]);

        // Set max listeners to avoid memory leaks
        this.setMaxListeners(100);
    }
}
```

### Process Stats Interface

```typescript
export interface ProcessStats {
    pid: number;
    ppid?: number;
    type: ProcessType;
    state: ProcessState;
    exitCode: number | null;
    executablePath: string;
    args: string[];
    startTime?: Date;
    endTime?: Date;
}
```

### Key Properties

| Property | Type | Description |
|----------|------|-------------|
| `pid` | `number` | Unique process identifier |
| `type` | `ProcessType` | Process type (SHELL or JAVASCRIPT) |
| `_state` | `ProcessState` | Current execution state |
| `_exitCode` | `number \| null` | Exit code after termination |
| `executablePath` | `string` | Path to the executable |
| `args` | `string[]` | Command-line arguments |
| `parentPid` | `number \| undefined` | Parent process ID for hierarchy |
| `cwd` | `string` | Current working directory |
| `env` | `Map<string, string>` | Environment variables |

---

## 3. Process Interface (IProcess)

While there's no explicit `IProcess` interface, the `Process` class serves as the contract. The API layer defines `VirtualProcess` for cross-worker communication:

### File: `packages/api/src/process/process.ts`

```typescript
export class VirtualProcess extends BrowserEventEmitter {
    readonly pid: number;
    readonly command: string;
    readonly args: string[];

    private worker: WorkerBridge;
    private _exitCode: number | null = null;
    private _startTime: Date;
    private _endTime?: Date;
    private _isRunning: boolean = true;

    constructor(
        pid: number,
        command: string,
        args: string[],
        worker: WorkerBridge
    ) {
        super();
        this.pid = pid;
        this.command = command;
        this.args = args;
        this.worker = worker;
        this._startTime = new Date();

        this.setMaxListeners(100);
    }

    async write(input: string): Promise<void> {
        if (!this._isRunning) {
            throw new Error('Process is not running');
        }

        await this.worker.sendMessage({
            type: 'writeInput',
            payload: { pid: this.pid, input }
        });
    }

    async kill(): Promise<void> {
        if (!this._isRunning) return;

        await this.worker.sendMessage({
            type: 'terminate',
            payload: { pid: this.pid }
        });

        this._isRunning = false;
        this._endTime = new Date();
        this._exitCode = -1;
        this.emit(ProcessEvent.EXIT, { exitCode: this._exitCode });
    }

    getStats(): ProcessStats {
        return {
            pid: this.pid,
            command: this.command,
            args: this.args,
            status: this._isRunning ? 'running' : 'exited',
            exitCode: this._exitCode,
            startTime: this._startTime,
            endTime: this._endTime,
            uptime: this._endTime ?
                this._endTime.getTime() - this._startTime.getTime() :
                Date.now() - this._startTime.getTime()
        };
    }
}
```

---

## 4. Process State Machine

### File: `packages/core/src/process/base/types.ts`

```typescript
export enum ProcessState {
    CREATED = 'created',       // Process instantiated but not started
    RUNNING = 'running',       // Process is actively executing
    COMPLETED = 'completed',   // Process finished successfully
    FAILED = 'failed',         // Process terminated with error
    TERMINATED = 'terminated'  // Process was forcibly stopped
}

export enum ProcessType {
    JAVASCRIPT = 'javascript',
    SHELL = 'shell'
}
```

### State Transition Diagram

```
                                    ┌──────────────┐
                                    │   CREATED    │
                                    └──────┬───────┘
                                           │ start()
                                           ▼
                                    ┌──────────────┐
                           ┌───────│   RUNNING    │───────┐
                           │       └──────┬───────┘       │
                           │              │               │
                    terminate()     normal exit     error/failure
                           │              │               │
                           ▼              ▼               ▼
                    ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
                    │ TERMINATED   │ │  COMPLETED   │ │    FAILED    │
                    └──────────────┘ └──────────────┘ └──────────────┘
```

### State Getter Implementation

```typescript
get state(): ProcessState {
    return this._state;
}

get exitCode(): number | null {
    return this._exitCode;
}

get uptime(): number | null {
    if (!this.startTime) return null;
    const endTime = this.endTime || new Date();
    return endTime.getTime() - this.startTime.getTime();
}
```

---

## 5. Process Executors

### 5.1 Base Executor Interface

**File:** `packages/core/src/process/executors/base.ts`

```typescript
export interface ProcessExecutor {
    canExecute(executable: string): boolean;
    execute(payload: ChildProcessPayload, pid: number, parentPid?: number): Promise<Process>;
}
```

### 5.2 Process Registry

**File:** `packages/core/src/process/executors/registry.ts`

```typescript
export class ProcessRegistry {
    private executors: Map<string, ProcessExecutor> = new Map();

    registerExecutor(type: string, executor: ProcessExecutor): void {
        this.executors.set(type, executor);
    }

    findExecutor(executable: string): ProcessExecutor | undefined {
        for (const [, executor] of this.executors.entries()) {
            if (executor.canExecute(executable)) {
                return executor;
            }
        }
        return undefined;
    }
}
```

**Registry Usage Pattern:**

```typescript
// In OpenWebContainer constructor
this.processRegistry.registerExecutor(
    'javascript',
    new NodeProcessExecutor(this.fileSystem, this.networkManager)
);
this.processRegistry.registerExecutor(
    'shell',
    new ShellProcessExecutor(this.fileSystem)
);
```

### 5.3 ShellExecutor Implementation

**File:** `packages/core/src/process/executors/shell/executor.ts`

```typescript
export class ShellProcessExecutor implements ProcessExecutor {
    constructor(private fileSystem: IFileSystem) { }

    canExecute(executable: string): boolean {
        return executable === 'sh';
    }

    async execute(payload: ChildProcessPayload, pid: number, parantPid?: number): Promise<Process> {
        return new ShellProcess(
            pid,
            payload.executable,
            payload.args,
            this.fileSystem,
            parantPid,
            payload.cwd
        );
    }
}
```

### 5.4 NodeExecutor Implementation

**File:** `packages/core/src/process/executors/node/executor.ts`

```typescript
export class NodeProcessExecutor implements ProcessExecutor {
    constructor(
        private fileSystem: IFileSystem,
        private networkManager: NetworkManager
    ) { }

    canExecute(executable: string): boolean {
        return executable === 'node' || executable.endsWith('.js');
    }

    async execute(payload: ChildProcessPayload, pid: number, parentPid?: number): Promise<Process> {
        let executablePath = payload.executable;
        let args = payload.args;
        let cwd = payload.cwd||'/';

        // If the command is 'node', the first arg is the script
        if (executablePath === 'node') {
            if (args.length === 0) {
                throw new Error('No JavaScript file specified');
            }
            executablePath = args[0];
            args = args.slice(1);
        }

        return new NodeProcess(
            pid,
            executablePath,
            args,
            this.fileSystem,
            this.networkManager,
            parentPid, 
            cwd
        );
    }
}
```

### Executor Selection Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    spawn('node app.js')                      │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│              ProcessRegistry.findExecutor('node')            │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  Iterate executors:                                          │
│  - ShellExecutor.canExecute('node') → false                  │
│  - NodeExecutor.canExecute('node') → true ✓                  │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│           NodeExecutor.execute(payload, pid)                 │
│           → Returns new NodeProcess                          │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. Process Lifecycle

### 6.1 Process Creation

**File:** `packages/core/src/container.ts`

```typescript
async spawn(executablePath: string, args: string[] = [], parentPid?: number, options: SpawnOptions = {}): Promise<Process> {
    const executor = this.processRegistry.findExecutor(executablePath);
    if (!executor) {
        throw new Error(`No executor found for: ${executablePath}`);
    }

    const pid = this.processManager.getNextPid();
    const process = await executor.execute({
        executable: executablePath,
        args,
        cwd: options.cwd || '/',
        env: options.env || {}
    }, pid, parentPid);

    // Set up general process event handlers
    this.setupProcessEventHandlers(process);

    // Set up child process spawning for all processes
    this.setupChildProcessSpawning(process);

    // Add process to manager and start it
    this.processManager.addProcess(process);
    process.start().catch(console.error);
    
    return process;
}

private setupProcessEventHandlers(process: Process): void {
    process.addEventListener(ProcessEvent.MESSAGE, (data: ProcessEventData) => {
        if (data.stdout) {
            this.notifyOutput(data.stdout);
        }
        if (data.stderr) {
            this.notifyOutput(data.stderr);
        }
    });

    process.addEventListener(ProcessEvent.ERROR, (data: ProcessEventData) => {
        if (data.error) {
            this.notifyOutput(`Error: ${data.error.message}\n`);
        }
    });

    process.addEventListener(ProcessEvent.EXIT, (data) => {
        if (data.exitCode) {
            this.notifyOutput(`Process exited with code: ${data.exitCode}\n`);
        }
    });
}
```

### 6.2 Start Event Emission

**File:** `packages/core/src/process/base/process.ts`

```typescript
async start(): Promise<void> {
    try {
        if (this.state !== ProcessState.CREATED) {
            throw new Error(`Cannot start process in state: ${this.state}`);
        }

        this._state = ProcessState.RUNNING;
        this.startTime = new Date();
        this.emit(ProcessEvent.START, { pid: this.pid });  // ← START event

        await this.execute();  // ← Subclass implementation

        if (!this.terminated) {
            this._state = ProcessState.COMPLETED;
            this._exitCode = 0;
        }
    } catch (error: any) {
        this._state = ProcessState.FAILED;
        this._exitCode = 1;
        this.emit(ProcessEvent.ERROR, { pid: this.pid, error });
    } finally {
        this.endTime = new Date();
        this.emit(ProcessEvent.EXIT, {
            pid: this.pid,
            exitCode: this._exitCode,
            uptime: this.uptime
        });
    }
}
```

### 6.3 Execution Flow Sequence Diagram

```
┌─────────┐    ┌──────────┐    ┌───────────┐    ┌───────────┐    ┌──────────┐
│  User   │    │ Container│    │ProcessMgr │    │ Executor  │    │  Process │
└────┬────┘    └────┬─────┘    └─────┬─────┘    └─────┬─────┘    └────┬─────┘
     │              │                │                │               │
     │ spawn('node app.js')          │                │               │
     │─────────────>│                │                │               │
     │              │                │                │               │
     │              │ findExecutor() │                │               │
     │              │───────────────>│                │               │
     │              │                │                │               │
     │              │  NodeExecutor  │                │               │
     │              │<───────────────│                │               │
     │              │                │                │               │
     │              │ getNextPid()   │                │               │
     │              │───────────────>│                │               │
     │              │                │                │               │
     │              │     pid=1      │                │               │
     │              │<───────────────│                │               │
     │              │                │                │               │
     │              │ execute()      │                │               │
     │              │───────────────────────────────>│               │
     │              │                │                │               │
     │              │                │  new NodeProcess()            │
     │              │                │──────────────────────────────>│
     │              │                │                │               │
     │              │    process     │                │               │
     │              │<───────────────────────────────────────────────│
     │              │                │                │               │
     │              │ addProcess()   │                │               │
     │              │───────────────>│                │               │
     │              │                │                │               │
     │              │ start()        │                │               │
     │              │───────────────────────────────────────────────>│
     │              │                │                │               │
     │              │  emit(START)   │                │               │
     │              │<───────────────────────────────────────────────│
     │              │                │                │               │
     │  Process     │                │                │               │
     │<─────────────│                │                │               │
     │              │                │                │               │
```

### 6.4 Exit Handling

```typescript
async terminate(): Promise<void> {
    if (this.state !== ProcessState.RUNNING) {
        return;
    }

    this.terminated = true;
    this._state = ProcessState.TERMINATED;
    this._exitCode = -1;
    this.endTime = new Date();

    await this.onTerminate();

    this.emit(ProcessEvent.EXIT, {
        pid: this.pid,
        exitCode: this._exitCode,
        uptime: this.uptime
    });
}
```

### 6.5 Error Handling Flow

```typescript
async start(): Promise<void> {
    try {
        // ... state validation ...
        
        this._state = ProcessState.RUNNING;
        this.startTime = new Date();
        this.emit(ProcessEvent.START, { pid: this.pid });

        await this.execute();  // May throw

        // Success path
        this._state = ProcessState.COMPLETED;
        this._exitCode = 0;
    } catch (error: any) {
        // Error handling
        this._state = ProcessState.FAILED;
        this._exitCode = 1;
        this.emit(ProcessEvent.ERROR, { pid: this.pid, error });
    } finally {
        // Always emit EXIT
        this.endTime = new Date();
        this.emit(ProcessEvent.EXIT, {
            pid: this.pid,
            exitCode: this._exitCode,
            uptime: this.uptime
        });
    }
}
```

### 6.6 Cleanup and Disposal

**File:** `packages/core/src/container.ts`

```typescript
async dispose(): Promise<void> {
    this.debugLog('Disposing container');

    // Stop all network servers
    for (const server of this.listServers()) {
        this.networkManager.unregisterServer(server.port, server.type);
    }

    // Kill all processes
    await this.processManager.killAll();

    // Clear output callbacks
    this.outputCallbacks = [];

    // Dispose network manager
    this.networkManager.dispose();

    this.debugLog('Container disposed');
}
```

**File:** `packages/core/src/process/manager/manager.ts`

```typescript
async killAll(): Promise<void> {
    const processes = this.listProcesses();
    await Promise.all(processes.map(process => process.terminate()));
    this.processes.clear();
}
```

---

## 7. Process Events

### 7.1 EventTypes Enum

**File:** `packages/core/src/process/base/types.ts`

```typescript
export enum ProcessEvent {
    START = 'start',
    EXIT = 'exit',
    ERROR = 'error',
    MESSAGE = 'message',
    SPAWN_CHILD = 'spawn_child'
}
```

**API Layer Events:**

**File:** `packages/api/src/process/types.ts`

```typescript
export enum ProcessEvent {
    OUTPUT = 'output',
    EXIT = 'exit',
    ERROR = 'error',
    SPAWN_CHILD = 'spawn_child',
}

export interface ProcessEventMap {
    'output': { output: string; isError: boolean };
    'exit': { exitCode: number | null };
    'error': { error: Error };
}
```

### 7.2 Event Emitter Implementation

**File:** `packages/core/src/process/base/event-emmiter.ts`

```typescript
export class BrowserEventEmitter {
    private events: Record<string, Function[]> = {};
    private maxListeners: number = 10;

    setMaxListeners(n: number) {
        this.maxListeners = n;
        return this;
    }

    on(event: string, listener: Function) {
        if (!this.events[event]) {
            this.events[event] = [];
        }
        if (this.events[event].length >= this.maxListeners) {
            console.warn(`MaxListenersExceededWarning: Possible memory leak detected. ${this.events[event].length} listeners added.`);
        }
        this.events[event].push(listener);
        return this;
    }

    off(event: string, listener: Function) {
        return this.removeListener(event, listener);
    }

    emit(event: string, ...args: any[]) {
        if (!this.events[event]) return false;
        this.events[event].forEach(listener => listener(...args));
        return true;
    }

    removeListener(event: string, listener: Function) {
        if (!this.events[event]) return this;
        this.events[event] = this.events[event].filter(l => l !== listener);
        return this;
    }
}
```

**API Layer with `once()` Support:**

```typescript
export class BrowserEventEmitter {
    private events: Record<string, Function[]> = {};
    private maxListeners: number = 10;
    private onceEvents: Set<Function> = new Set();

    once(event: string, listener: Function) {
        const onceWrapper = (...args: any[]) => {
            this.off(event, onceWrapper);
            this.onceEvents.delete(onceWrapper);
            listener(...args);
        };
        this.onceEvents.add(onceWrapper);
        return this.on(event, onceWrapper);
    }

    removeAllListeners(event?: string) {
        if (event) {
            this.events[event] = [];
        } else {
            this.events = {};
        }
        return this;
    }

    listenerCount(event: string): number {
        return this.events[event]?.length || 0;
    }
}
```

### 7.3 Event Listener Registration

The `Process` class provides type-safe event listener registration:

```typescript
addEventListener(event: ProcessEvent.START, listener: (data: { pid: number }) => void): void;
addEventListener(event: ProcessEvent.EXIT, listener: (data: { pid: number; exitCode: number | null; uptime: number | null }) => void): void;
addEventListener(event: ProcessEvent.ERROR, listener: (data: { pid: number; error: Error }) => void): void;
addEventListener(event: ProcessEvent.MESSAGE, listener: (data: { stdout?: string; stderr?: string }) => void): void;
addEventListener(event: ProcessEvent.SPAWN_CHILD, listener: (data: SpawnChildEventData) => void): void;

addEventListener(event: ProcessEvent, listener: (data: any) => void): void {
    this.on(event, listener);
}

removeEventListener(event: ProcessEvent, listener: (data: any) => void): void {
    this.off(event, listener);
}
```

### 7.4 Event Data Structures

```typescript
// START event data
{ pid: number }

// EXIT event data
{ 
    pid: number; 
    exitCode: number | null; 
    uptime: number | null 
}

// ERROR event data
{ 
    pid: number; 
    error: Error 
}

// MESSAGE event data
{ 
    stdout?: string; 
    stderr?: string 
}

// SPAWN_CHILD event data
interface SpawnChildEventData {
    payload: ChildProcessPayload;
    callback: (result: ChildProcessResult) => void;
}

interface ChildProcessPayload {
    executable: string;
    args: string[];
    env?: Record<string, string>;
    cwd?: string;
}

interface ChildProcessResult {
    stdout: string;
    stderr: string;
    exitCode: number;
}
```

### Event Flow Diagram

```
                    ┌──────────────────────────────────────┐
                    │           Process.start()            │
                    └─────────────┬────────────────────────┘
                                  │
                    ┌─────────────▼────────────────────────┐
                    │  emit(START, { pid })                │
                    │  → Listeners: [{ pid } → callback()  │
                    └─────────────┬────────────────────────┘
                                  │
                    ┌─────────────▼────────────────────────┐
                    │           execute()                  │
                    │                                      │
                    │  ┌────────────────────────────────┐  │
                    │  │ During execution:              │  │
                    │  │ emit(MESSAGE, { stdout })      │  │
                    │  │ emit(MESSAGE, { stderr })      │  │
                    │  └────────────────────────────────┘  │
                    └─────────────┬────────────────────────┘
                                  │
              ┌───────────────────┼───────────────────┐
              │                   │                   │
    ┌─────────▼─────────┐ ┌───────▼───────┐ ┌────────▼────────┐
    │   Normal Exit     │ │    Error      │ │  Terminated     │
    │                   │ │               │ │                 │
    │ emit(EXIT, {      │ │ emit(ERROR,   │ │ emit(EXIT, {    │
    │   exitCode: 0 })  │ │   error })    │ │   exitCode: -1})│
    └───────────────────┘ └───────────────┘ └─────────────────┘
```

---

## 8. Process Manager

### 8.1 ProcessManager Class

**File:** `packages/core/src/process/manager/manager.ts`

```typescript
/**
 * Process Manager to handle multiple processes
 */
export class ProcessManager {
    private processes: Map<number, Process>;
    private nextPid: number;

    constructor() {
        this.processes = new Map();
        this.nextPid = 1;
    }

    getNextPid(): number {
        return this.nextPid++;
    }

    addProcess(process: Process): void {
        this.processes.set(process.pid, process);
    }

    getProcess(pid: number): Process | undefined {
        return this.processes.get(pid);
    }

    removeProcess(pid: number): boolean {
        return this.processes.delete(pid);
    }

    listProcesses(): Process[] {
        return Array.from(this.processes.values());
    }

    async killAll(): Promise<void> {
        const processes = this.listProcesses();
        await Promise.all(processes.map(process => process.terminate()));
        this.processes.clear();
    }
}
```

### 8.2 Process Tracking

The `ProcessManager` maintains a `Map<PID, Process>` for efficient lookups:

```typescript
private processes: Map<number, Process>;
```

**Operations:**

| Method | Description | Complexity |
|--------|-------------|------------|
| `addProcess(process)` | Register a new process | O(1) |
| `getProcess(pid)` | Retrieve process by PID | O(1) |
| `removeProcess(pid)` | Remove process from tracking | O(1) |
| `listProcesses()` | Get all active processes | O(n) |
| `killAll()` | Terminate all processes | O(n) |

### 8.3 spawn() Method (Container Level)

**File:** `packages/core/src/container.ts`

```typescript
async spawn(executablePath: string, args: string[] = [], parentPid?: number, options: SpawnOptions = {}): Promise<Process> {
    // 1. Find appropriate executor
    const executor = this.processRegistry.findExecutor(executablePath);
    if (!executor) {
        throw new Error(`No executor found for: ${executablePath}`);
    }

    // 2. Get next PID
    const pid = this.processManager.getNextPid();
    
    // 3. Create process via executor
    const process = await executor.execute({
        executable: executablePath,
        args,
        cwd: options.cwd || '/',
        env: options.env || {}
    }, pid, parentPid);

    // 4. Set up event handlers
    this.setupProcessEventHandlers(process);
    this.setupChildProcessSpawning(process);

    // 5. Register with ProcessManager
    this.processManager.addProcess(process);
    
    // 6. Start execution (async, not awaited)
    process.start().catch(console.error);
    
    return process;
}
```

### 8.4 kill() Method

**File:** `packages/core/src/container.ts`

```typescript
/**
 * Terminate a process and all its children
 */
async terminateProcessTree(pid: number): Promise<void> {
    const children = this.getChildProcesses(pid);

    // First terminate all children (recursive)
    await Promise.all(
        children.map(child => this.terminateProcessTree(child.pid))
    );

    // Then terminate the process itself
    const process = this.processManager.getProcess(pid);
    if (process) {
        await process.terminate();
        this.processManager.removeProcess(pid);
    }
}
```

### 8.5 getAll() Method

```typescript
listProcesses(): Process[] {
    return this.processManager.listProcesses();
}

getProcess(pid: number): Process | undefined {
    return this.processManager.getProcess(pid);
}

// Get child processes
getChildProcesses(parentPid: number): Process[] {
    return this.processManager.listProcesses()
        .filter(process => process.parentPid === parentPid);
}

// Get process tree
getProcessTree(pid: number): ProcessTree {
    const process = this.processManager.getProcess(pid);
    if (!process) {
        throw new Error(`Process ${pid} not found`);
    }

    return {
        info: this.getProcessInfo(process),
        children: this.getChildProcesses(pid)
            .map(child => this.getProcessTree(child.pid))
    };
}

// Get full process tree starting from init
getFullProcessTree(): ProcessTree[] {
    const topLevelProcesses = this.processManager.listProcesses()
        .filter(process => !process.parentPid);

    return topLevelProcesses.map(process => this.getProcessTree(process.pid));
}
```

### 8.6 Resource Cleanup

```typescript
async dispose(): Promise<void> {
    // 1. Stop all network servers
    for (const server of this.listServers()) {
        this.networkManager.unregisterServer(server.port, server.type);
    }

    // 2. Kill all processes
    await this.processManager.killAll();

    // 3. Clear output callbacks
    this.outputCallbacks = [];

    // 4. Dispose network manager
    this.networkManager.dispose();
}
```

---

## 9. Shell Process Implementation

### 9.1 ShellProcess Class

**File:** `packages/core/src/process/executors/shell/process.ts`

```typescript
export class ShellProcess extends Process {
    private shell: Shell;
    private prompt: string;
    private currentLine: string = '';
    private running: boolean = true;
    private filteredArgs: string[];
    private commandHistory: CommandHistoryEntry[] = [];
    private historyIndex: number = -1;

    // Readline state
    private cursorPosition: number = 0;
    private lineBuffer: string[] = [];

    private fileSystem: IFileSystem;

    constructor(
        pid: number,
        executablePath: string,
        args: string[],
        fileSystem: IFileSystem,
        parentPid?: number,
        cwd?: string,
        env?: Map<string, string>
    ) {
        super(pid, ProcessType.SHELL, executablePath, args, parentPid, cwd, env);
        this.fileSystem = fileSystem;
        const oscMode = args.includes('--osc');
        this.filteredArgs = args.filter(arg => arg !== '--osc');

        this.shell = new Shell(fileSystem, { oscMode, process: this, env: this.env });
        this.prompt = oscMode ? '\x1b[1;32m$\x1b[0m ' : '$ ';
    }
}
```

### 9.2 Command Execution

```typescript
protected async execute(): Promise<void> {
    try {
        // Handle initial command if provided in args
        if (this.filteredArgs.length > 0) {
            const result = await this.executeCommand(this.filteredArgs.join(' '));
            if (result.stdout) {
                this.emitOutput(result.stdout + '\n');
            }
            if (result.stderr) {
                this.emitError(result.stderr + '\n');
            }
            this._exitCode = result.exitCode;
            return;
        }

        // Initial prompt
        this.emitOutput(this.prompt);

        // Interactive shell loop
        while (this.running && this.state === ProcessState.RUNNING) {
            const input = await this.readInput();
            await this.handleInput(input);
        }
    } catch (error: any) {
        this.emitError(`Shell error: ${error.message}\n`);
        throw error;
    }
}
```

### 9.3 Interactive Mode

The shell implements a full readline-like interface:

```typescript
private async handleInput(input: string): Promise<void> {
    // Detect paste (multiple characters without escape sequence)
    if (input.length > 1 && !input.startsWith('\x1b')) {
        await this.handlePaste(input);
        return;
    }

    switch (input) {
        case '\r': // Enter
            await this.handleEnterKey();
            break;

        case '\x7F': // Backspace
        case '\b':
            this.handleBackspace();
            break;

        case '\x1b[A': // Up arrow (history)
            this.handleUpArrow();
            break;

        case '\x1b[B': // Down arrow (history)
            this.handleDownArrow();
            break;

        case '\x1b[C': // Right arrow
            this.handleRightArrow();
            break;

        case '\x1b[D': // Left arrow
            this.handleLeftArrow();
            break;

        case '\x03': // Ctrl+C
            this.handleCtrlC();
            break;

        case '\x04': // Ctrl+D (exit)
            this.handleCtrlD();
            break;

        default:
            if (input.length === 1 && input >= ' ') {
                this.handleCharacterInput(input);
            }
            break;
    }
}
```

### 9.4 Working Directory Management

**File:** `packages/core/src/shell/shell.ts`

```typescript
export class Shell implements IShell {
    private fileSystem: IFileSystem;
    private currentDirectory: string;
    private env: Map<string, string>;

    constructor(fileSystem: IFileSystem, options: ShellOptions) {
        this.fileSystem = fileSystem;
        this.currentDirectory = '/';
        this.env = options.env || new Map([
            ['PATH', '/bin:/usr/bin'],
            ['HOME', '/home'],
            ['PWD', this.currentDirectory],
        ]);
    }

    getWorkingDirectory(): string {
        return this.currentDirectory;
    }

    setWorkingDirectory(path: string): void {
        const resolvedPath = this.resolvePath(path);
        if (!this.fileSystem.isDirectory(resolvedPath)) {
            throw new Error(`Directory not found: ${path}`);
        }
        this.currentDirectory = resolvedPath;
        this.env.set('PWD', resolvedPath);
    }

    private resolvePath(path: string): string {
        if (path.startsWith('/')) {
            return path;
        }
        return `${this.currentDirectory}/${path}`.replace(/\/+/g, '/');
    }
}
```

### 9.5 Built-in Commands

```typescript
private registerAllBuiltInCommands() {
    this.registerBuiltInCommand('cd', this.cd.bind(this));
    this.registerBuiltInCommand('ls', this.ls.bind(this));
    this.registerBuiltInCommand('pwd', this.pwd.bind(this));
    this.registerBuiltInCommand('cat', this.cat.bind(this));
    this.registerBuiltInCommand('echo', this.echo.bind(this));
    this.registerBuiltInCommand('mkdir', this.mkdir.bind(this));
    this.registerBuiltInCommand('rm', this.rm.bind(this));
    this.registerBuiltInCommand('rmdir', this.rmdir.bind(this));
    this.registerBuiltInCommand('touch', this.touch.bind(this));
}

// Example: ls command
private async ls(args: string[]): Promise<ShellCommandResult> {
    try {
        const path = args[0] || this.currentDirectory;
        const resolvedPath = this.resolvePath(path);
        const entries = this.fileSystem.listDirectory(resolvedPath);
        return this.success(entries.join('\n'));
    } catch (error: any) {
        return this.failure(error.message);
    }
}

// Example: cd command
private async cd(args: string[]): Promise<ShellCommandResult> {
    try {
        const path = args[0] || '/';
        const newPath = this.resolvePath(path);
        if (!this.fileSystem.isDirectory(newPath)) {
            return this.failure(`Directory not found: ${path}`);
        }
        this.currentDirectory = newPath;
        this.env.set('PWD', newPath);
        return this.success();
    } catch (error: any) {
        return this.failure(error.message);
    }
}
```

### 9.6 File Redirection Handling

```typescript
interface CommandParsedResult {
    command: string;
    args: string[];
    redirects: {
        type: '>>' | '>';
        file: string;
    }[];
}

private parseCommand(args: string[]): CommandParsedResult {
    const result: CommandParsedResult = {
        command: '',
        args: [],
        redirects: []
    };

    let i = 0;
    while (i < args.length) {
        const arg = args[i];

        if (arg === '>' || arg === '>>') {
            if (i + 1 >= args.length) {
                throw new Error(`Syntax error: missing file for redirection ${arg}`);
            }
            result.redirects.push({
                type: arg as ('>' | '>>'),
                file: args[i + 1]
            });
            i += 2;
        } else {
            if (!result.command) {
                result.command = arg;
            } else {
                result.args.push(arg);
            }
            i++;
        }
    }

    return result;
}

private handleRedirection(output: string, redirects: CommandParsedResult['redirects']): void {
    for (const redirect of redirects) {
        const filePath = this.resolvePath(redirect.file);

        try {
            if (redirect.type === '>>') {
                // Append to file
                const existingContent = this.fileSystem.readFile(filePath) || '';
                this.fileSystem.writeFile(filePath, existingContent + output);
            } else {
                // Overwrite file
                this.fileSystem.writeFile(filePath, output);
            }
        } catch (error: any) {
            throw new Error(`Failed to redirect to ${redirect.file}: ${error.message}`);
        }
    }
}
```

---

## 10. Node Process Implementation

### 10.1 NodeProcess Class

**File:** `packages/core/src/process/executors/node/process.ts`

```typescript
export class NodeProcess extends Process {
    private fileSystem: IFileSystem;
    private networkManager: NetworkManager;
    private httpModule: QuickJSHandle|undefined;
    private networkModule: NetworkModule|undefined; 
    private context: QuickJSContext|undefined;

    constructor(
        pid: number,
        executablePath: string,
        args: string[],
        fileSystem: IFileSystem,
        networkManager: NetworkManager,
        parantPid?: number,
        cwd?: string
    ) {
        super(pid, ProcessType.JAVASCRIPT, executablePath, args, parantPid, cwd);
        this.fileSystem = fileSystem;
        this.networkManager = networkManager;
    }
}
```

### 10.2 QuickJS Runtime Integration

```typescript
async execute(): Promise<void> {
    try {
        const QuickJS = await newQuickJSAsyncWASMModuleFromVariant(variant)

        const runtime = QuickJS.newRuntime();
        
        // Set up module loader
        runtime.setModuleLoader((moduleName, ctx) => {
            try {
                const resolvedPath = this.fileSystem.resolveModulePath(moduleName, this.cwd);
                const content = this.fileSystem.readFile(resolvedPath);

                if (content === undefined) {
                    return { error: new Error(`Module not found: ${moduleName}`) };
                }
                return { value: content };
            } catch (error: any) {
                return { error };
            }
        }, (baseModuleName, requestedName) => {
            try {
                let basePath = baseModuleName ?
                    baseModuleName.substring(0, baseModuleName.lastIndexOf('/')) :
                    this.cwd;

                basePath = this.fileSystem.normalizePath(basePath||this.cwd||"/");

                const resolvedPath = this.fileSystem.resolveModulePath(requestedName, basePath);
                return { value: resolvedPath };
            } catch (error: any) {
                return { error };
            }
        });

        const context = runtime.newContext();
        this.context = context;
        
        // ... rest of setup
    }
}
```

### 10.3 Module Loading

The module loader supports both named imports and relative paths:

```typescript
runtime.setModuleLoader(
    // Resolve and load module content
    (moduleName, ctx) => {
        const resolvedPath = this.fileSystem.resolveModulePath(moduleName, this.cwd);
        const content = this.fileSystem.readFile(resolvedPath);
        if (content === undefined) {
            return { error: new Error(`Module not found: ${moduleName}`) };
        }
        return { value: content };
    },
    // Resolve module path from base
    (baseModuleName, requestedName) => {
        let basePath = baseModuleName ?
            baseModuleName.substring(0, baseModuleName.lastIndexOf('/')) :
            this.cwd;
        
        const resolvedPath = this.fileSystem.resolveModulePath(requestedName, basePath);
        return { value: resolvedPath };
    }
);
```

### 10.4 Console Output Capture

```typescript
// Set up console.log and other console methods
const consoleObj = context.newObject();

// Console.log
const logFn = context.newFunction("log", (...args) => {
    const output = args.map(arg => `${context.dump(arg)}`).join(" ") + "\n";
    this.emit(ProcessEvent.MESSAGE, { stdout: output });
});
context.setProp(consoleObj, "log", logFn);

// Console.debug
const debugFn = context.newFunction("debug", (...args) => {
    const output = args.map(arg => `${context.dump(arg)}`).join(" ") + "\n";
    this.emit(ProcessEvent.MESSAGE, { stderr: output });
});
context.setProp(consoleObj, "debug", debugFn);

// Console.error
const errorFn = context.newFunction("error", (...args) => {
    const output = args.map(arg => `${context.dump(arg)}`).join(" ") + "\n";
    this.emit(ProcessEvent.MESSAGE, { stderr: output });
});
context.setProp(consoleObj, "error", errorFn);

context.setProp(context.global, "console", consoleObj);

// Clean up function handles
logFn.dispose();
errorFn.dispose();
consoleObj.dispose();
```

### 10.5 Async Execution

```typescript
// Execute the code
const result = context.evalCode(content, this.executablePath, { type: 'module' });

// Handle any pending promises
while (runtime.hasPendingJob()) {
    const jobResult = runtime.executePendingJobs(10);
    if (jobResult.error) {
        throw context.dump(jobResult.error);
    }
}

if (result.error) {
    throw context.dump(result.error);
}
result.value.dispose();
this._exitCode = 0;
this._state = ProcessState.COMPLETED;
```

### 10.6 require() Implementation

```typescript
private setupRequire(context: QuickJSContext) {
    const requireFn = context.newFunction("require", (moduleId) => {
        const id = context.getString(moduleId)

        // Patching http module
        if (id === 'http' && this.networkModule) {
            this.httpModule = this.networkModule.createHttpModule()
            return this.httpModule.dup()
        }

        // Load external modules
        try {
            let modulePath = id
            if (!id.startsWith('./') && !id.startsWith('/')) {
                modulePath = `/node_modules/${id}`
            }

            const result = context.evalCode(
                `import('${modulePath}').then(m => m.default || m)`,
                'dynamic-import.js',
                { type: 'module' }
            )

            if (result.error) {
                throw new Error(`Failed to load module ${id}: ${context.dump(result.error)}`)
            }

            const promiseState = context.getPromiseState(result.value)
            result.value.dispose()

            if (promiseState.type === 'fulfilled') {
                return promiseState
            } else if (promiseState.type === 'rejected') {
                const error = context.dump(promiseState.error)
                promiseState.error.dispose()
                throw new Error(`Module load failed: ${error}`)
            } else {
                throw new Error(`Module loading is pending: ${id}`)
            }
        } catch (error: any) {
            throw new Error(`Cannot find module '${id}': ${error.message}`)
        }
    })

    context.setProp(context.global, "require", requireFn)
    requireFn.dispose()

    // CommonJS support
    const moduleObj = context.newObject()
    const exportsObj = context.newObject()
    context.setProp(moduleObj, "exports", exportsObj)
    context.setProp(context.global, "module", moduleObj)
    context.setProp(context.global, "exports", exportsObj)
    moduleObj.dispose()
    exportsObj.dispose()
}
```

---

## 11. Inter-Process Communication

### 11.1 Message Passing

Processes communicate via the event emitter:

```typescript
protected emitOutput(stdout: string): void {
    this.emit(ProcessEvent.MESSAGE, { stdout });
}

protected emitError(stderr: string): void {
    this.emit(ProcessEvent.MESSAGE, { stderr });
}

protected emitMessage(message: { stdout?: string; stderr?: string }): void {
    this.emit(ProcessEvent.MESSAGE, message);
}
```

### 11.2 Stdin/Stdout/Stderr Streams

**Input Handling:**

```typescript
writeInput(input: string): void {
    if (this._state !== ProcessState.RUNNING) {
        throw new Error('Cannot write input to non-running process');
    }

    this.inputBuffer.push(input);
    this.processNextInput();
}

protected async readInput(): Promise<string> {
    if (this.inputBuffer.length > 0) {
        return this.inputBuffer.shift()!;
    }

    return new Promise((resolve) => {
        this.inputCallbacks.push(resolve);
    });
}

private processNextInput(): void {
    while (this.inputCallbacks.length > 0 && this.inputBuffer.length > 0) {
        const callback = this.inputCallbacks.shift()!;
        const input = this.inputBuffer.shift()!;
        callback(input);
    }
}
```

### 11.3 Signal Handling (SIGTERM, SIGKILL)

The `terminate()` method simulates SIGTERM:

```typescript
async terminate(): Promise<void> {
    if (this.state !== ProcessState.RUNNING) {
        return;
    }

    this.terminated = true;
    this._state = ProcessState.TERMINATED;
    this._exitCode = -1;  // Convention for signal termination
    this.endTime = new Date();

    await this.onTerminate();

    this.emit(ProcessEvent.EXIT, {
        pid: this.pid,
        exitCode: this._exitCode,
        uptime: this.uptime
    });
}
```

### 11.4 Child Process Spawning

```typescript
private setupChildProcessSpawning(process: Process): void {
    process.addEventListener(ProcessEvent.SPAWN_CHILD, (data: SpawnChildEventData) => {
        this.spawnChildProcess(process.pid, data.payload, data.callback);
    });
}

private async spawnChildProcess(
    parentPid: number,
    payload: ChildProcessPayload,
    callback: (result: ChildProcessResult) => void
): Promise<void> {
    let childPid: number|null = null
    try {
        const parentProcess = this.processManager.getProcess(parentPid);
        if (!parentProcess) {
            throw new Error(`Parent process ${parentPid} not found`);
        }

        const childProcess = await this.spawn(
            payload.executable,
            payload.args,
            parentPid  // Pass parent PID
        );
        childPid = childProcess.pid

        // Forward child events to parent
        childProcess.addEventListener(ProcessEvent.MESSAGE, (data: ProcessEventData) => {
            parentProcess.emit(ProcessEvent.MESSAGE, { ...data });
        });

        childProcess.addEventListener(ProcessEvent.EXIT, (data) => {
            callback({
                stdout: "",
                stderr: "",
                exitCode: data.exitCode ?? 1
            });

            // Clean up the process
            this.processManager.removeProcess(childProcess.pid);
        });

    } catch (error: any) {
        if (childPid) {
            this.processManager.removeProcess(childPid);
        }
        callback({
            stdout: '',
            stderr: error.message,
            exitCode: 1
        });
    }
}
```

### IPC Flow Diagram

```
┌──────────────────┐                          ┌──────────────────┐
│   Parent Process │                          │   Child Process  │
└────────┬─────────┘                          └────────┬─────────┘
         │                                             │
         │  emit(SPAWN_CHILD, { payload, callback })   │
         │────────────────────────────────────────────>│
         │                                             │
         │                                             │ spawn()
         │                                             │
         │                          ┌──────────────────▼──────────────────┐
         │                          │       ProcessManager.addProcess()   │
         │                          └─────────────────────────────────────┘
         │                                             │
         │                                             │ start()
         │                                             │
         │                           emit(START, { pid })
         │<────────────────────────────────────────────│
         │                                             │
         │                           emit(MESSAGE, { stdout })
         │<────────────────────────────────────────────│
         │          (forward to parent)                │
         │                                             │
         │                           emit(EXIT, { exitCode })
         │<────────────────────────────────────────────│
         │                                             │
         │  callback({ stdout, stderr, exitCode })     │
         │────────────────────────────────────────────>│
         │                                             │
         │                          ProcessManager.removeProcess()
```

---

## 12. Network Module for Node Processes

### 12.1 HTTP Module Integration

**File:** `packages/core/src/process/executors/node/modules/http.ts`

```typescript
export class HTTPModule {
    private context: QuickJSContext
    private requestHandler?: (req: Request) => Promise<Response>
    private onServerStart: (port: number) => void

    constructor(context: QuickJSContext, onServerStart: (port: number) => void) {
        this.context = context
        this.onServerStart = onServerStart
    }

    setupHttpModule(): QuickJSHandle {
        const httpModule = this.context.newObject()

        // Create http.createServer()
        const createServerFn = this.context.newFunction("createServer", (handler) => {
            const serverResult = this.context.callFunction(serverClass, this.context.undefined, []);
            if (serverResult.error) {
                console.error('Failed to create server:', this.context.dump(serverResult.error));
                return this.context.undefined;
            }
            const server = serverResult.value;

            if (handler) {
                this.requestHandler = async (req: Request): Promise<Response> => {
                    // Create request object
                    const reqObj = this.context.newObject()
                    this.context.setProp(reqObj, "method", this.context.newString(req.method))
                    this.context.setProp(reqObj, "url", this.context.newString(req.url))

                    // Handle headers
                    const headers = this.context.newObject()
                    if (req.headers instanceof Headers) {
                        req.headers.forEach((value, key) => {
                            this.context.setProp(headers, key.toLowerCase(), this.context.newString(value))
                        })
                    }
                    this.context.setProp(reqObj, "headers", headers)
                    headers.dispose()

                    // Create response object and handler
                    let responseBody = ''
                    let statusCode = 200
                    const responseHeaders: Record<string, string> = {}

                    const resObj = this.context.newObject()

                    // Implement res.writeHead()
                    const writeHeadFn = this.context.newFunction("writeHead", (code, headers) => {
                        statusCode = this.context.getNumber(code)
                        if (headers) {
                            Object.assign(responseHeaders, this.context.dump(headers))
                        }
                        return resObj
                    })
                    this.context.setProp(resObj, "writeHead", writeHeadFn)
                    writeHeadFn.dispose()

                    // Implement res.write() and res.end()
                    // ... (see full implementation above)

                    try {
                        this.context.callFunction(handler, this.context.undefined, [reqObj, resObj])
                    } finally {
                        reqObj.dispose()
                        resObj.dispose()
                    }

                    return responsePromise
                }
            }

            return server
        })

        this.context.setProp(httpModule, "createServer", createServerFn)
        createServerFn.dispose()

        return httpModule
    }
}
```

### 12.2 Server Registration

```typescript
this.networkModule = new NetworkModule(context, (port: number) => {
    console.log('registering server', port)
    this.networkManager.registerServer(this.pid, port, 'http', { host: '0.0.0.0' })
}, (port: number) => {
    this.networkManager.unregisterServer(port, 'http')
}, true);
```

---

## Appendix A: Complete File Structure

```
packages/core/src/process/
├── base/
│   ├── event-emmiter.ts      # BrowserEventEmitter class
│   ├── index.ts              # Exports
│   ├── process.ts            # Base Process class
│   └── types.ts              # ProcessState, ProcessType, ProcessEvent enums
│
├── executors/
│   ├── base.ts               # ProcessExecutor interface
│   ├── index.ts              # Exports
│   ├── registry.ts           # ProcessRegistry class
│   │
│   ├── shell/
│   │   ├── executor.ts       # ShellProcessExecutor
│   │   ├── index.ts          # Exports
│   │   └── process.ts        # ShellProcess class
│   │
│   └── node/
│       ├── executor.ts       # NodeProcessExecutor
│       ├── index.ts          # Exports
│       ├── process.ts        # NodeProcess class
│       └── modules/
│           ├── http.ts       # HTTPModule for QuickJS
│           ├── httpMock.ts   # HTTP mocking utilities
│           ├── network.ts    # Network utilities
│           └── network-module.ts  # NetworkModule class
│
└── manager/
    ├── index.ts              # Exports
    └── manager.ts            # ProcessManager class
```

---

## Appendix B: Key Interfaces Summary

```typescript
// Process Executor Interface
export interface ProcessExecutor {
    canExecute(executable: string): boolean;
    execute(payload: ChildProcessPayload, pid: number, parentPid?: number): Promise<Process>;
}

// Child Process Payload
export interface ChildProcessPayload {
    executable: string;
    args: string[];
    env?: Record<string, string>;
    cwd?: string;
}

// Child Process Result
export interface ChildProcessResult {
    stdout: string;
    stderr: string;
    exitCode: number;
}

// Process Info
export interface ProcessInfo {
    pid: number;
    ppid?: number;
    type: ProcessType;
    state: ProcessState;
    executablePath: string;
    args: string[];
    startTime?: Date;
    endTime?: Date;
    uptime?: number;
}

// Process Tree
export interface ProcessTree {
    info: ProcessInfo;
    children: ProcessTree[];
}

// Spawn Child Event Data
export interface SpawnChildEventData {
    payload: ChildProcessPayload;
    callback: (result: ChildProcessResult) => void;
}
```

---

## Appendix C: Production Patterns

### 1. Memory Leak Prevention

```typescript
// Set max listeners
this.setMaxListeners(100);

// Warning on exceeded
if (this.events[event].length >= this.maxListeners) {
    console.warn(`MaxListenersExceededWarning: Possible memory leak detected.`);
}
```

### 2. Proper Handle Disposal (QuickJS)

```typescript
// Always dispose handles after use
logFn.dispose();
errorFn.dispose();
consoleObj.dispose();
result.value.dispose();
context.dispose();
```

### 3. Async Error Handling

```typescript
async start(): Promise<void> {
    try {
        // ... execution ...
    } catch (error: any) {
        this._state = ProcessState.FAILED;
        this._exitCode = 1;
        this.emit(ProcessEvent.ERROR, { pid: this.pid, error });
    } finally {
        // Always emit EXIT
        this.emit(ProcessEvent.EXIT, { ... });
    }
}
```

### 4. Process Tree Cleanup

```typescript
async terminateProcessTree(pid: number): Promise<void> {
    const children = this.getChildProcesses(pid);
    
    // Terminate children first (recursive)
    await Promise.all(
        children.map(child => this.terminateProcessTree(child.pid))
    );

    // Then terminate parent
    const process = this.processManager.getProcess(pid);
    if (process) {
        await process.terminate();
        this.processManager.removeProcess(pid);
    }
}
```

---

**Document Location:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/opencontainer/02-process-management-deep-dive.md`

**Source Code Location:** `/home/darkvoid/Boxxed/@formulas/src.opencontainer/OpenWebContainer/packages/core/src/process/`
