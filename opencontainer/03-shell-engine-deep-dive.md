# Shell Engine Deep Dive: OpenWebContainer

**Source:** `/home/darkvoid/Boxxed/@formulas/src.opencontainer/OpenWebContainer/packages/core/src/shell/`

A comprehensive technical deep-dive into the Shell Engine architecture of OpenWebContainer - a browser-based virtual container runtime that provides container-like isolation entirely within the browser environment.

---

## Table of Contents

1. [Shell Architecture Overview](#1-shell-architecture-overview)
2. [Shell Class Structure](#2-shell-class-structure)
3. [ShellProcess Integration](#3-shellprocess-integration)
4. [Command Parsing and Tokenization](#4-command-parsing-and-tokenization)
5. [Command Registry System](#5-command-registry-system)
6. [Built-in Commands Implementation](#6-built-in-commands-implementation)
7. [External Commands Architecture](#7-external-commands-architecture)
8. [Command Implementation Pattern](#8-command-implementation-pattern)
9. [File Redirection System](#9-file-redirection-system)
10. [Pipe Support and Command Chaining](#10-pipe-support-and-command-chaining)
11. [Interactive Shell Implementation](#11-interactive-shell-implementation)
12. [Error Handling Strategy](#12-error-handling-strategy)
13. [Architecture Diagrams](#13-architecture-diagrams)
14. [Comparison with Bash/SH](#14-comparison-with-bashsh)

---

## 1. Shell Architecture Overview

### 1.1 High-Level Architecture

The OpenWebContainer Shell Engine is a fully-featured virtual shell implementation that runs entirely in the browser, providing POSIX-like shell functionality without requiring a backend server.

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Browser Environment                             │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                    OpenWebContainer Core                         │ │
│  │                                                                  │ │
│  │  ┌───────────────────────────────────────────────────────────┐  │ │
│  │  │                    Process Manager                         │  │ │
│  │  │  ┌─────────────────┐  ┌─────────────────┐                 │  │ │
│  │  │  │ ShellExecutor   │  │ NodeExecutor    │                 │  │ │
│  │  │  │                 │  │                 │                 │  │ │
│  │  │  │ ┌─────────────┐ │  │ ┌─────────────┐ │                 │  │ │
│  │  │  │ │ShellProcess │ │  │ │NodeProcess  │ │                 │  │ │
│  │  │  │ └──────┬──────┘ │  │ └─────────────┘ │                 │  │ │
│  │  │  │        │        │  │                 │                 │  │ │
│  │  │  │ ┌──────▼──────┐ │  │                 │                 │  │ │
│  │  │  │ │    Shell    │ │  │                 │                 │  │ │
│  │  │  │ │             │ │  │                 │                 │  │ │
│  │  │  │ │ ┌─────────┐ │ │  │                 │                 │  │ │
│  │  │  │ │ │Command  │ │ │  │                 │                 │  │ │
│  │  │  │ │ │Registry │ │ │  │                 │                 │  │ │
│  │  │  │ │ └────┬────┘ │ │  │                 │                 │  │ │
│  │  │  │ │        │     │ │  │                 │                 │  │ │
│  │  │  │ │ ┌──────▼─────▼──▼──┐               │                 │  │ │
│  │  │  │ │ │  Built-in Commands│               │                 │  │ │
│  │  │  │ │ │  ls, cd, pwd, ...│               │                 │  │ │
│  │  │  │ │ └──────────────────┘               │                 │  │ │
│  │  │  │ │ ┌──────────────────┐               │                 │  │ │
│  │  │  │ │ │ External Commands│               │                 │  │ │
│  │  │  │ │ │ curl, wget, unzip│               │                 │  │ │
│  │  │  │ │ └──────────────────┘               │                 │  │ │
│  │  │  │ └────────────────────────────────────┘                 │  │ │
│  │  │ └──────────────────────────────────────────────────────────┘  │ │
│  │  └───────────────────────────────────────────────────────────────┘ │
│  │                                                                    │
│  │  ┌───────────────────────────────────────────────────────────────┐│
│  │  │                  Virtual File System (ZenFS)                   ││
│  │  └───────────────────────────────────────────────────────────────┘│
│  └────────────────────────────────────────────────────────────────────┘
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 Key Components

| Component | Responsibility |
|-----------|---------------|
| **ShellProcess** | Manages interactive shell session, input handling, terminal emulation |
| **Shell** | Core shell logic, command execution, path resolution |
| **CommandRegistry** | Command lookup and registration system |
| **ShellCommand** | Base class for all commands |
| **Built-in Commands** | Filesystem operations (ls, cd, pwd, mkdir, etc.) |
| **External Commands** | Network operations (curl, wget, unzip) |

### 1.3 Execution Flow

```
User Input (Terminal)
        │
        ▼
┌───────────────────┐
│  ShellProcess     │
│  - Reads input    │
│  - Handles special│
│    keys (arrows,  │
│    Ctrl+C, etc.)  │
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│  Shell.execute()  │
│  - Tokenizes      │
│  - Parses redirects│
│  - Routes command │
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│  CommandRegistry  │
│  - Lookup command │
│  - Instantiate    │
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│  Command.execute()│
│  - Parse args     │
│  - Execute logic  │
│  - Return result  │
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│  ShellCommandResult│
│  - stdout         │
│  - stderr         │
│  - exitCode       │
└───────────────────┘
```

---

## 2. Shell Class Structure

### 2.1 Shell Class Definition

The `Shell` class is the central coordinator for all shell operations:

```typescript
// packages/core/src/shell/shell.ts
interface ShellOptions {
    oscMode?: boolean;
    process: Process;
    env?: Map<string, string>;
}

export class Shell implements IShell {
    private fileSystem: IFileSystem;
    private currentDirectory: string;
    private env: Map<string, string>;
    private commandHistory: string[] = [];
    private historyIndex: number = -1;
    private oscMode: boolean = false;
    private buildInCommands: Map<string, (args: string[]) => Promise<ShellCommandResult>> = new Map();
    private commandRegistry: CommandRegistry;
    private process: Process;

    constructor(fileSystem: IFileSystem, options: ShellOptions) {
        this.fileSystem = fileSystem;
        this.currentDirectory = '/';
        this.env = options.env || new Map([
            ['PATH', '/bin:/usr/bin'],
            ['HOME', '/home'],
            ['PWD', this.currentDirectory],
        ]);
        this.process = options.process;
        this.oscMode = options.oscMode || false;
        this.commandRegistry = new CommandRegistry();
        this.registerAllBuiltInCommands()
        this.registerAllExternalCommands();
    }
}
```

### 2.2 State Management

The Shell maintains several pieces of state:

| Property | Type | Purpose |
|----------|------|---------|
| `fileSystem` | `IFileSystem` | Virtual filesystem interface |
| `currentDirectory` | `string` | Current working directory (CWD) |
| `env` | `Map<string, string>` | Environment variables |
| `commandHistory` | `string[]` | Command history for arrow navigation |
| `historyIndex` | `number` | Current position in history |
| `oscMode` | `boolean` | ANSI color output mode |
| `buildInCommands` | `Map<...>` | Built-in command handlers |
| `commandRegistry` | `CommandRegistry` | External command registry |

### 2.3 Command Registration

```typescript
private registerBuiltInCommand(
    name: string, 
    command: (args: string[]) => Promise<ShellCommandResult>
) {
    this.buildInCommands.set(name, command);
}

private registerExternalCommand(name: string, commandClass: any) {
    this.commandRegistry.register(name, commandClass);
}

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

private registerAllExternalCommands() {
    this.registerExternalCommand('curl', CurlCommand);
    this.registerExternalCommand('unzip', UnzipCommand);
    this.registerExternalCommand('wget', WgetCommand);
}
```

### 2.4 Path Resolution

The Shell implements path resolution for both absolute and relative paths:

```typescript
private resolvePath(path: string): string {
    if (path.startsWith('/')) {
        return path;
    }
    return `${this.currentDirectory}/${path}`.replace(/\/+/g, '/');
}
```

This method:
1. Returns absolute paths unchanged
2. Prepends current directory to relative paths
3. Normalizes multiple slashes to single slashes

---

## 3. ShellProcess Integration

### 3.1 ShellProcess Class

`ShellProcess` extends the base `Process` class and manages the interactive shell session:

```typescript
// packages/core/src/process/executors/shell/process.ts
export class ShellProcess extends Process {
    private shell: Shell;
    private prompt: string;
    private currentLine: string = '';
    private running: boolean = true;
    private commandHistory: CommandHistoryEntry[] = [];
    private historyIndex: number = -1;

    // Line editing state
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

### 3.2 Interactive Shell Loop

The main execution loop handles interactive input:

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

### 3.3 Input Handling

The shell handles various keyboard inputs:

```typescript
private async handleInput(input: string): Promise<void> {
    // Detect paste by checking if input is multiple characters
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

        case '\x1b[A': // Up arrow
            this.handleUpArrow();
            break;

        case '\x1b[B': // Down arrow
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

        case '\x04': // Ctrl+D
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

### 3.4 Special Key Handlers

| Key Sequence | Handler | Action |
|--------------|---------|--------|
| `\r` | `handleEnterKey()` | Execute command |
| `\x7F`, `\b` | `handleBackspace()` | Delete character |
| `\x1b[A` | `handleUpArrow()` | Previous history |
| `\x1b[B` | `handleDownArrow()` | Next history |
| `\x1b[C` | `handleRightArrow()` | Move cursor right |
| `\x1b[D` | `handleLeftArrow()` | Move cursor left |
| `\x03` | `handleCtrlC()` | Cancel current line |
| `\x04` | `handleCtrlD()` | Exit shell |

### 3.5 Command Execution in ShellProcess

```typescript
private async executeCommand(commandLine: string): Promise<ShellCommandResult> {
    // Tokenize command line (handling quoted strings)
    const args = commandLine.match(/(?:[^\s"']+|"[^"]*"|'[^']*')+/g) || [];
    const processedArgs = args.map(arg => arg.replace(/^["'](.+)["']$/, '$1'));

    if (processedArgs.length === 0) {
        return { stdout: '', stderr: '', exitCode: 0 };
    }

    const [command, ...cmdArgs] = processedArgs;

    // Handle built-in commands
    switch (command) {
        case 'exit':
            this.running = false;
            this._exitCode = 0;
            return { stdout: '', stderr: '', exitCode: 0 };

        case 'history':
            const historyOutput = this.commandHistory
                .map((entry, index) =>
                    `${index + 1}: [${entry.timestamp.toISOString()}] ${entry.command}`)
                .join('\n');
            return { stdout: historyOutput, stderr: '', exitCode: 0 };

        default:
            // Try to execute as a program
            try {
                if (this.shell.hasCommand(command)) {
                    return await this.shell.execute(command, cmdArgs);
                }
                else if (command === 'node') {
                    const result = await this.spawnChild(command, cmdArgs);
                    return result;
                }
                else {
                    // Check PATH for executable
                    let PATH = this.env.get('PATH');
                    if (PATH) {
                        const paths = PATH.split(':');
                        for (const path of paths) {
                            const executablePath = this.fileSystem.resolvePath(command, path);
                            if (this.fileSystem.fileExists(executablePath)) {
                                return await this.spawnChild(executablePath, cmdArgs);
                            }
                        }
                    }
                    // Check for shebang
                    let content = this.fileSystem.readFile(command);
                    if (content) {
                        const shebang = content.match(/^#!(.*)/);
                        if (shebang) {
                            const interpreterName = shebang[1];
                            let name = interpreterName.split(" ")[0]
                            if (name == '/usr/bin/env') {
                                let tokens = interpreterName.split(" ")
                                if (tokens.length == 1)
                                    throw "executor not specified"
                                let newCommand = tokens[1]
                                return await this.spawnChild(newCommand, [command, ...cmdArgs]);
                            }
                        }
                    }
                }
                return await this.shell.execute(command, cmdArgs);
            } catch (error: any) {
                return {
                    stdout: '',
                    stderr: error.message,
                    exitCode: 1
                };
            }
    }
}
```

---

## 4. Command Parsing and Tokenization

### 4.1 Command Parsing Algorithm

The shell uses a two-stage parsing approach:

1. **Tokenization** - Split command line into tokens
2. **Redirection Parsing** - Extract redirection operators

### 4.2 Tokenization

The tokenizer handles quoted strings and escape sequences:

```typescript
// Match tokens: unquoted words, double-quoted strings, or single-quoted strings
const args = commandLine.match(/(?:[^\s"']+|"[^"]*"|'[^']*')+/g) || [];

// Remove surrounding quotes from tokens
const processedArgs = args.map(arg => arg.replace(/^["'](.+)["']$/, '$1'));
```

**Tokenization Examples:**

| Input | Output Tokens |
|-------|--------------|
| `ls -la /app` | `["ls", "-la", "/app"]` |
| `echo "hello world"` | `["echo", "hello world"]` |
| `cat 'file name.txt'` | `["cat", "file name.txt"]` |
| `ls "/path with spaces"` | `["ls", "/path with spaces"]` |

### 4.3 Redirection Parsing

The `parseCommand` method extracts redirection operators:

```typescript
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
            i += 2; // Skip redirect operator and filename
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
```

### 4.4 Parsing Result Structure

```typescript
// packages/core/src/shell/types.ts
export interface CommandParsedResult {
    command: string;
    args: string[];
    redirects: {
        type: '>>' | '>';
        file: string;
    }[];
}
```

### 4.5 Parse Example

**Input:** `echo "Hello" > output.txt`

**Parsing Steps:**

1. Tokenize: `["echo", "Hello", ">", "output.txt"]`
2. Parse:
   ```typescript
   {
       command: "echo",
       args: ["Hello"],
       redirects: [{ type: '>', file: "output.txt" }]
   }
   ```

---

## 5. Command Registry System

### 5.1 CommandRegistry Class

The CommandRegistry provides a type-safe command lookup system:

```typescript
// packages/core/src/shell/commands/registry.ts
export class CommandRegistry {
    private commands: Map<string, new (options: CommandOptions) => ShellCommand> = new Map();

    register(name: string, commandClass: new (options: CommandOptions) => ShellCommand) {
        this.commands.set(name, commandClass);
    }

    get(name: string): (new (options: CommandOptions) => ShellCommand) | undefined {
        return this.commands.get(name);
    }
    
    has(name: string): boolean {
        return this.commands.has(name);
    }

    getAll(): string[] {
        return Array.from(this.commands.keys());
    }
}
```

### 5.2 Command Lookup Flow

```
┌──────────────────┐
│ Command Request  │
│   "curl"         │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  has("curl")     │
│  Check registry  │
└────────┬─────────┘
         │
         ├─── Found ──► new CurlCommand(options)
         │
         └─── Not Found ──► Check built-in commands
                              │
                              ├─── Found ──► Execute built-in
                              │
                              └─── Not Found ──► "Command not found: 127"
```

### 5.3 Command Options

Commands receive context through `CommandOptions`:

```typescript
// packages/core/src/shell/commands/base.ts
export interface CommandOptions {
    cwd: string;
    fileSystem: IFileSystem;
    env?: Map<string, string>;
    process: Process;
}
```

---

## 6. Built-in Commands Implementation

### 6.1 ls - List Directory

```typescript
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
```

**Supported Flags:**
- No flags: List directory contents (one per line)
- `-l`, `-la`: Long format (not implemented - would show permissions, size, date)
- `-a`: Show hidden files (not implemented)

### 6.2 cd - Change Directory

```typescript
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

**Supported Arguments:**
- `cd` - Go to root (`/`)
- `cd /path` - Absolute path
- `cd relative/path` - Relative path
- `cd ..` - Parent directory
- `cd ~` - Home directory (if implemented)

### 6.3 pwd - Print Working Directory

```typescript
private async pwd(): Promise<ShellCommandResult> {
    return this.success(this.currentDirectory);
}
```

### 6.4 mkdir - Create Directory

```typescript
private async mkdir(args: string[]): Promise<ShellCommandResult> {
    if (args.length === 0) {
        return this.failure('No directory specified');
    }
    try {
        this.fileSystem.createDirectory(this.resolvePath(args[0]));
        return this.success();
    } catch (error: any) {
        return this.failure(error.message);
    }
}
```

**Supported Flags:**
- `-p`: Create parent directories (not implemented - would require path traversal)

### 6.5 touch - Create File

```typescript
private async touch(args: string[]): Promise<ShellCommandResult> {
    if (args.length === 0) {
        return this.failure('No file specified');
    }
    try {
        this.fileSystem.writeFile(this.resolvePath(args[0]), '');
        return this.success();
    } catch (error: any) {
        return this.failure(error.message);
    }
}
```

### 6.6 rm - Remove File

```typescript
private async rm(args: string[]): Promise<ShellCommandResult> {
    if (args.length === 0) {
        return this.failure('No file specified');
    }
    try {
        const recursive = args.includes('-r') || args.includes('-rf');
        const files = args.filter(arg => !arg.startsWith('-'));

        for (const file of files) {
            this.fileSystem.deleteFile(this.resolvePath(file), recursive);
        }
        return this.success();
    } catch (error: any) {
        return this.failure(error.message);
    }
}
```

**Supported Flags:**
- `-r`, `-rf`: Recursive deletion

### 6.7 rmdir - Remove Directory

```typescript
private async rmdir(args: string[]): Promise<ShellCommandResult> {
    if (args.length === 0) {
        return this.failure('No directory specified');
    }
    try {
        this.fileSystem.deleteDirectory(this.resolvePath(args[0]));
        return this.success();
    } catch (error: any) {
        return this.failure(error.message);
    }
}
```

### 6.8 cat - Display File Content

```typescript
private async cat(args: string[]): Promise<ShellCommandResult> {
    if (args.length === 0) {
        return this.failure('No file specified');
    }
    try {
        const content = this.fileSystem.readFile(this.resolvePath(args[0]));
        if (content === undefined) {
            return this.failure(`File not found: ${args[0]}`);
        }
        return this.success(content);
    } catch (error: any) {
        return this.failure(error.message);
    }
}
```

### 6.9 echo - Display Text

```typescript
private async echo(args: string[]): Promise<ShellCommandResult> {
    return this.success(args.join(' ') + '\n');
}
```

### 6.10 cp - Copy File

```typescript
private async cp(args: string[]): Promise<ShellCommandResult> {
    if (args.length < 2) {
        return this.failure('Source and destination required');
    }
    try {
        const [src, dest] = args;
        const content = this.fileSystem.readFile(this.resolvePath(src));
        if (content === undefined) {
            return this.failure(`Source file not found: ${src}`);
        }
        this.fileSystem.writeFile(this.resolvePath(dest), content);
        return this.success();
    } catch (error: any) {
        return this.failure(error.message);
    }
}
```

### 6.11 mv - Move/Rename File

```typescript
private async mv(args: string[]): Promise<ShellCommandResult> {
    if (args.length < 2) {
        return this.failure('Source and destination required');
    }
    try {
        const [src, dest] = args;
        const content = this.fileSystem.readFile(this.resolvePath(src));
        if (content === undefined) {
            return this.failure(`Source file not found: ${src}`);
        }
        this.fileSystem.writeFile(this.resolvePath(dest), content);
        this.fileSystem.deleteFile(this.resolvePath(src));
        return this.success();
    } catch (error: any) {
        return this.failure(error.message);
    }
}
```

---

## 7. External Commands Architecture

External commands extend the `ShellCommand` base class and provide more complex functionality:

### 7.1 Base Command Class

```typescript
// packages/core/src/shell/commands/base.ts
export interface CommandOptions {
    cwd: string;
    fileSystem: IFileSystem;
    env?: Map<string, string>;
    process: Process;
}

export interface CommandHelp {
    name: string;
    description: string;
    usage: string;
    examples: string[];
}

export abstract class ShellCommand {
    protected cwd: string;
    protected fileSystem: IFileSystem;
    protected env: Map<string, string>;
    protected process: Process;

    constructor(options: CommandOptions) {
        this.cwd = options.cwd;
        this.fileSystem = options.fileSystem;
        this.env = options.env || new Map();
        this.process = options.process;
    }

    abstract get help(): CommandHelp;
    abstract execute(args: string[]): Promise<ShellCommandResult>;

    protected success(stdout: string = ''): ShellCommandResult {
        return {
            stdout: stdout ? stdout + '\n' : '',
            stderr: '',
            exitCode: 0
        };
    }

    protected error(message: string, code: number = 1): ShellCommandResult {
        return {
            stdout: '',
            stderr: message + '\n',
            exitCode: code
        };
    }

    protected resolvePath(path: string): string {
        if (path.startsWith('/')) {
            return path;
        }
        return `${this.cwd}/${path}`.replace(/\/+/g, '/');
    }

    protected showHelp(): ShellCommandResult {
        const { name, description, usage, examples } = this.help;
        let output = `${name} - ${description}\n\n`;
        output += `Usage: ${usage}\n\n`;
        if (examples.length > 0) {
            output += 'Examples:\n';
            examples.forEach(example => {
                output += `  ${example}\n`;
            });
        }
        return this.success(output);
    }
}
```

### 7.2 curl Command

```typescript
// packages/core/src/shell/commands/curl.ts
export class CurlCommand extends ShellCommand {
    get help(): CommandHelp {
        return {
            name: 'curl',
            description: 'Transfer data from or to a server',
            usage: 'curl [options] URL\n' +
                'Options:\n' +
                '  -X <method>  HTTP method\n' +
                '  -H <header>  Custom header\n' +
                '  -o <file>    Output to file',
            examples: [
                'curl https://api.example.com',
                'curl -X POST -H "Content-Type: application/json" https://api.com',
                'curl -o output.json https://api.com/data'
            ]
        };
    }

    async execute(args: string[]): Promise<ShellCommandResult> {
        try {
            // Basic argument parsing
            const urlIndex = args.findIndex(arg => !arg.startsWith('-'));
            if (urlIndex === -1) {
                return {
                    stdout: '',
                    stderr: 'curl: URL required',
                    exitCode: 1
                };
            }

            const url = args[urlIndex];
            const options = args.slice(0, urlIndex);

            // Parse options
            const method = options.includes('-X') ?
                args[args.indexOf('-X') + 1] : 'GET';
            const headers: Record<string, string> = {};
            const outputFile = options.includes('-o') ?
                args[args.indexOf('-o') + 1] : undefined;

            const followRedirects = !options.includes('--no-follow');
            const insecure = options.includes('-k') || options.includes('--insecure');

            // Parse headers
            for (let i = 0; i < args.length; i++) {
                if (args[i] === '-H' && args[i + 1]) {
                    const headerStr = args[i + 1];
                    const [key, ...valueParts] = headerStr.split(':');
                    const value = valueParts.join(':').trim();
                    headers[key.trim()] = value;
                    i++;
                }
            }

            try {
                const response = await fetch(url, {
                    method,
                    headers: {
                        ...headers,
                        'Accept': '*/*',
                        'Accept-Encoding': 'gzip, deflate, br',
                    },
                    redirect: followRedirects ? 'follow' : 'manual',
                    mode: 'cors',
                });

                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                }

                const responseText = await response.text();

                // Handle output to file if -o option is used
                if (outputFile) {
                    this.fileSystem.writeFile(this.resolvePath(outputFile), responseText);
                    return {
                        stdout: `  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current\n` +
                            `                                   Dload  Upload   Total   Spent    Left  Speed\n` +
                            `100  ${responseText.length}  100  ${responseText.length}    0     0   ${Math.floor(responseText.length / 0.1)}      0  0:00:01 --:--:--  0:00:01 ${Math.floor(responseText.length / 0.1)}\n`,
                        stderr: '',
                        exitCode: 0
                    };
                }

                return {
                    stdout: responseText + '\n',
                    stderr: '',
                    exitCode: 0
                };

            } catch (error: any) {
                return {
                    stdout: '',
                    stderr: `curl: (6) Could not resolve host: ${error.message}\n`,
                    exitCode: 6
                };
            }
        } catch (error: any) {
            return {
                stdout: '',
                stderr: `curl: ${error.message}\n`,
                exitCode: 1
            };
        }
    }
}
```

### 7.3 wget Command

The `wget` command implements advanced download functionality with CORS proxy fallback:

```typescript
// packages/core/src/shell/commands/wget.ts
export class WgetCommand extends ShellCommand {
    get help(): CommandHelp {
        return {
            name: 'wget',
            description: 'Download files from the web',
            usage: 'wget [options] URL\n' +
                'Options:\n' +
                '  -O <file>  Save to specific file\n' +
                '  -q         Quiet mode\n' +
                '  --header   Add custom header',
            examples: [
                'wget https://example.com/file.txt',
                'wget -O custom.txt https://example.com/file.txt',
                'wget --header "Authorization: Bearer token" https://api.com/data'
            ]
        };
    }

    async execute(args: string[]): Promise<ShellCommandResult> {
        if (args.length === 0) {
            return {
                stdout: '',
                stderr: 'wget: missing URL\nUsage: wget [options] URL\n',
                exitCode: 1
            };
        }

        // Parse options
        const options: WgetOptions = { headers: {} };
        const urls: string[] = [];

        for (let i = 0; i < args.length; i++) {
            const arg = args[i];
            switch (arg) {
                case '-O':
                    options.outputFilename = args[++i];
                    break;
                case '-q':
                    options.quiet = true;
                    break;
                case '--no-check-certificate':
                    options.noCheck = true;
                    options.noCheckCertificate = true;
                    break;
                case '-c':
                    options.continue = true;
                    break;
                case '--debug':
                    options.debug = true;
                    break;
                case '--header':
                case '-H':
                    const headerStr = args[++i];
                    const [key, ...valueParts] = headerStr.split(':');
                    const value = valueParts.join(':').trim();
                    options.headers[key.trim()] = value;
                    break;
                case '--timeout':
                    options.timeout = parseInt(args[++i]) * 1000;
                    break;
                case '-t':
                    options.retries = parseInt(args[++i]);
                    break;
                default:
                    if (!arg.startsWith('-')) {
                        urls.push(arg);
                    } else {
                        return {
                            stdout: '',
                            stderr: `wget: unknown option ${arg}\n`,
                            exitCode: 1
                        };
                    }
            }
        }

        let stdout = '';
        let stderr = '';
        let exitCode = 0;

        for (const url of urls) {
            try {
                const result = await this.downloadFile(url, options);
                stdout += result.stdout;
                stderr += result.stderr;
                if (result.exitCode !== 0) exitCode = result.exitCode;
            } catch (error: any) {
                stderr += `wget: ${error.message}\n`;
                exitCode = 1;
            }
        }

        return { stdout, stderr, exitCode };
    }
}
```

### 7.4 unzip Command

The `unzip` command supports both ZIP and tar.gz formats:

```typescript
// packages/core/src/shell/commands/unzip.ts
import { unzip, inflate, gunzip } from 'fflate';

export class UnzipCommand extends ShellCommand {
    get help(): CommandHelp {
        return {
            name: 'unzip',
            description: 'Extract compressed zip or tgz files',
            usage: `Usage: unzip [options] <file.zip|file.tgz> [destination]

Options:
  -l    List contents without extracting
  -v    Verbose mode showing file details
  -q    Quiet mode, suppress output
  -d    Extract files into directory
  --help Show this help message`,
            examples: [
                'unzip archive.zip',
                'unzip file.tgz output/',
                'unzip -l archive.zip',
                'unzip -v package.tgz',
                'unzip -d /target/dir archive.zip'
            ]
        };
    }

    async execute(args: string[]): Promise<ShellCommandResult> {
        try {
            if (args.includes('--help')) {
                return this.showHelp();
            }

            if (args.length === 0) {
                return this.error('unzip: filename required');
            }

            // Parse options
            const options = {
                listOnly: args.includes('-l'),
                verbose: args.includes('-v'),
                quiet: args.includes('-q')
            };

            // Remove flags and process -d option
            let destination = '.';
            const cleanArgs = args.filter((arg, index) => {
                if (arg === '-d' && args[index + 1]) {
                    destination = args[index + 1];
                    return false;
                }
                return !arg.startsWith('-');
            });

            const filename = cleanArgs[0];
            destination = cleanArgs[1] || destination;

            // Resolve paths
            const filepath = this.resolvePath(filename);
            const content = this.fileSystem.readBuffer(filepath);
            if (!content) {
                return this.error(`unzip: cannot find ${filename}`);
            }
            const uint8Array = new Uint8Array(content.buffer, 0, content.length);

            // Process based on file type
            if (filename.endsWith('.tgz') || filename.endsWith('.tar.gz')) {
                return this.handleTarGz(filename, uint8Array, destination, options);
            } else {
                return this.handleZip(filename, uint8Array, destination, options);
            }

        } catch (error: any) {
            return this.error(`unzip: ${error.message}`);
        }
    }
}
```

---

## 8. Command Implementation Pattern

### 8.1 Base Class Requirements

All commands must:

1. **Extend `ShellCommand`** - Provides common utilities
2. **Implement `help` getter** - Returns command documentation
3. **Implement `execute()` method** - Main command logic

### 8.2 Standard Command Template

```typescript
import { ShellCommand, CommandHelp, CommandOptions } from './base';
import { ShellCommandResult } from '../types';

interface MyCommandOptions {
    flag1?: boolean;
    output?: string;
}

export class MyCommand extends ShellCommand {
    get help(): CommandHelp {
        return {
            name: 'mycommand',
            description: 'Description of what the command does',
            usage: 'mycommand [options] <argument>',
            examples: [
                'mycommand file.txt',
                'mycommand -o output.txt file.txt'
            ]
        };
    }

    async execute(args: string[]): Promise<ShellCommandResult> {
        try {
            // 1. Validate arguments
            if (args.length === 0) {
                return this.error('mycommand: argument required');
            }

            // 2. Parse options
            const options: MyCommandOptions = {};
            const positionalArgs: string[] = [];

            for (let i = 0; i < args.length; i++) {
                const arg = args[i];
                switch (arg) {
                    case '-o':
                        options.output = args[++i];
                        break;
                    case '--flag':
                        options.flag1 = true;
                        break;
                    default:
                        if (!arg.startsWith('-')) {
                            positionalArgs.push(arg);
                        }
                }
            }

            // 3. Execute command logic
            const result = await this.doSomething(positionalArgs[0]);

            // 4. Return success
            return this.success(result);

        } catch (error: any) {
            return this.error(`mycommand: ${error.message}`);
        }
    }

    private async doSomething(arg: string): Promise<string> {
        // Command implementation
        return `Processed: ${arg}`;
    }
}
```

### 8.3 Argument Parsing Pattern

```typescript
// Pattern 1: Switch-based parsing
for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
        case '-f':
        case '--force':
            options.force = true;
            break;
        case '-o':
        case '--output':
            options.output = args[++i];
            break;
        default:
            if (!args[i].startsWith('-')) {
                positionalArgs.push(args[i]);
            }
    }
}

// Pattern 2: Flag detection
const recursive = args.includes('-r') || args.includes('-rf');
const force = args.includes('-f');
const positionalArgs = args.filter(arg => !arg.startsWith('-'));
```

### 8.4 Error Handling Pattern

```typescript
async execute(args: string[]): Promise<ShellCommandResult> {
    try {
        // Validation
        if (args.length === 0) {
            return this.error('Error message');
        }

        // Logic
        const result = await this.doWork();
        return this.success(result);

    } catch (error: any) {
        return this.error(`Command: ${error.message}`);
    }
}
```

---

## 9. File Redirection System

### 9.1 Supported Redirection Types

| Operator | Type | Description |
|----------|------|-------------|
| `>` | Output | Redirect stdout to file (overwrite) |
| `>>` | Append | Redirect stdout to file (append) |
| `2>` | Stderr | Redirect stderr to file |
| `2>&1` | Combined | Redirect stderr to stdout |

### 9.2 Redirection Parsing

```typescript
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
```

### 9.3 Redirection Execution

```typescript
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

### 9.4 Full Execution Flow with Redirection

```typescript
async execute(command: string, args: string[]): Promise<ShellCommandResult> {
    try {
        if (!command) {
            return this.success();
        }
        
        // Add to history
        this.commandHistory.push(command);

        // Parse command and redirections
        const parsedCommand = this.parseCommand([command, ...args]);

        // Execute the actual command
        const result = await this.executeCommand(
            parsedCommand.command,
            parsedCommand.args
        );

        // Handle redirections
        if (result.exitCode === 0 && parsedCommand.redirects.length > 0) {
            try {
                this.handleRedirection(result.stdout, parsedCommand.redirects);
                result.stdout = ''; // Clear stdout since it's in file
            } catch (error: any) {
                return {
                    stdout: '',
                    stderr: error.message,
                    exitCode: 1
                };
            }
        }

        return result;
    } catch (error: any) {
        return this.failure(error.message);
    }
}
```

### 9.5 Redirection Examples

```bash
# Overwrite output to file
echo "Hello" > output.txt

# Append output to file
echo "World" >> output.txt

# Multiple redirections (not fully supported - would need 2>&1)
ls > files.txt
```

---

## 10. Pipe Support and Command Chaining

### 10.1 Current Implementation Status

The OpenWebContainer shell currently does **not** implement pipe (`|`) support. This section describes the planned architecture for pipe implementation.

### 10.2 Planned Pipe Architecture

```typescript
interface ParsedPipeline {
    commands: CommandParsedResult[];
}

private parsePipe(commandLine: string): ParsedPipeline {
    const commands = commandLine.split('|');
    return {
        commands: commands.map(cmd => this.parseCommand(cmd.trim().split(' ')))
    };
}

private async executePipeline(pipeline: ParsedPipeline): Promise<ShellCommandResult> {
    let previousOutput = '';

    for (const command of pipeline.commands) {
        const result = await this.executeCommand(command.command, [
            ...(previousOutput ? [previousOutput] : []),
            ...command.args
        ]);

        if (result.exitCode !== 0) {
            return result;
        }

        previousOutput = result.stdout;
    }

    return {
        stdout: previousOutput,
        stderr: '',
        exitCode: 0
    };
}
```

### 10.3 Pipe Implementation Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│                      Command Pipeline                            │
│                                                                   │
│   ls -la  │  grep ".ts"  │  head -n 5                            │
│      │            │              │                               │
│      ▼            ▼              ▼                               │
│  ┌──────┐    ┌──────┐      ┌──────┐                             │
│  │Cmd 1 │───▶│Cmd 2 │─────▶│Cmd 3 │                             │
│  └──────┘    └──────┘      └──────┘                             │
│      │            │              │                               │
│      │            │              ▼                               │
│      │            │        Final Output                          │
│      │            │                                              │
│      ▼            │                                              │
│   stdout ─────────┘                                              │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### 10.4 Challenges for Pipe Implementation

1. **Buffering** - Large outputs need streaming, not full buffering
2. **Exit codes** - Which command's exit code to return?
3. **Error handling** - How to handle errors in middle of pipeline?
4. **Concurrent execution** - Should commands run in parallel?

---

## 11. Interactive Shell Implementation

### 11.1 Read-Eval-Print Loop (REPL)

```typescript
protected async execute(): Promise<void> {
    try {
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

### 11.2 Command History Management

```typescript
// History storage
private commandHistory: CommandHistoryEntry[] = [];
private historyIndex: number = -1;

interface CommandHistoryEntry {
    command: string;
    timestamp: Date;
}

// Add command to history
private async handleEnterKey(): Promise<void> {
    this.emitOutput('\n');

    const commandLine = this.currentLine.trim();
    if (commandLine) {
        // Add to history
        this.commandHistory.push({
            command: commandLine,
            timestamp: new Date()
        });
        this.historyIndex = this.commandHistory.length;

        // Execute command
        const result = await this.executeCommand(commandLine);
        // ... handle output
    }

    // Reset current line
    this.currentLine = '';
    this.cursorPosition = 0;
    this.emitOutput(this.prompt);
}

// Navigate history with arrow keys
private handleUpArrow(): void {
    if (this.historyIndex > 0) {
        this.historyIndex--;
        this.updateInputLine(this.commandHistory[this.historyIndex].command);
    }
}

private handleDownArrow(): void {
    if (this.historyIndex < this.commandHistory.length - 1) {
        this.historyIndex++;
        this.updateInputLine(this.commandHistory[this.historyIndex].command);
    } else {
        this.historyIndex = this.commandHistory.length;
        this.updateInputLine('');
    }
}
```

### 11.3 Line Editing

```typescript
// Line editing state
private cursorPosition: number = 0;
private currentLine: string = '';

// Handle backspace
private handleBackspace(): void {
    if (this.cursorPosition > 0) {
        const before = this.currentLine.slice(0, this.cursorPosition - 1);
        const after = this.currentLine.slice(this.cursorPosition);
        this.currentLine = before + after;
        this.cursorPosition--;

        // Update display
        this.emitOutput('\b \b'); // Move back, clear char, move back
        if (after) {
            this.emitOutput(after + '\x1b[K'); // Clear to end
            this.emitOutput(`\x1b[${after.length}D`); // Move cursor back
        }
    }
}

// Handle character input
private handleCharacterInput(char: string): void {
    const before = this.currentLine.slice(0, this.cursorPosition);
    const after = this.currentLine.slice(this.cursorPosition);
    this.currentLine = before + char + after;
    this.cursorPosition++;

    this.emitOutput(char);
    if (after) {
        this.emitOutput(after);
        this.emitOutput(`\x1b[${after.length}D`);
    }
}

// Handle left/right arrows
private handleLeftArrow(): void {
    if (this.cursorPosition > 0) {
        this.cursorPosition--;
        this.emitOutput('\x1b[D');
    }
}

private handleRightArrow(): void {
    if (this.cursorPosition < this.currentLine.length) {
        this.cursorPosition++;
        this.emitOutput('\x1b[C');
    }
}
```

### 11.4 Tab Completion

Tab completion is **not currently implemented**. Here's a planned implementation:

```typescript
// Planned tab completion
private handleTab(): void {
    const currentWord = this.currentLine.slice(0, this.cursorPosition).split(' ').pop();
    
    // Get possible completions
    const completions = this.getCompletions(currentWord);
    
    if (completions.length === 1) {
        // Single match - complete it
        this.completeWord(currentWord, completions[0]);
    } else if (completions.length > 1) {
        // Multiple matches - show options
        this.showCompletions(completions);
    }
}

private getCompletions(partial: string): string[] {
    const dir = this.fileSystem.listDirectory(this.currentDirectory);
    return dir.filter(entry => entry.startsWith(partial));
}
```

### 11.5 Ctrl+C Handling

```typescript
private handleCtrlC(): void {
    this.currentLine = '';
    this.cursorPosition = 0;
    this.emitOutput('^C\n' + this.prompt);
}
```

### 11.6 Ctrl+D Handling

```typescript
private handleCtrlD(): void {
    if (this.currentLine.length === 0) {
        this.emitOutput('exit\n');
        this.running = false;
        this._exitCode = 0;
    }
}
```

### 11.7 Paste Handling

```typescript
private async handlePaste(pastedText: string): Promise<void> {
    const lines = pastedText.split(/\r?\n/);

    // Handle first line
    const firstLine = lines[0];
    const before = this.currentLine.slice(0, this.cursorPosition);
    const after = this.currentLine.slice(this.cursorPosition);
    this.currentLine = before + firstLine + after;
    this.cursorPosition += firstLine.length;
    this.emitOutput(firstLine);

    // Handle remaining lines
    if (lines.length > 1) {
        for (let i = 1; i < lines.length; i++) {
            await this.handleEnterKey();
            const line = lines[i];
            if (line.length > 0) {
                this.currentLine = line;
                this.cursorPosition = line.length;
                this.emitOutput(line);
            }
        }
    }
}
```

---

## 12. Error Handling

### 12.1 Error Types and Exit Codes

| Exit Code | Meaning | Example |
|-----------|---------|---------|
| 0 | Success | Command completed successfully |
| 1 | General error | Command execution failed |
| 127 | Command not found | Unknown command |
| 126 | Permission denied | File exists but not executable |

### 12.2 Command Not Found

```typescript
default:
    return {
        stdout: '',
        stderr: `Command not found: ${command}`,
        exitCode: 127
    };
```

### 12.3 Permission Denied

```typescript
if (!this.fileSystem.isDirectory(newPath)) {
    return this.failure(`Directory not found: ${path}`);
}
```

### 12.4 Path Resolution Errors

```typescript
try {
    const resolvedPath = this.resolvePath(path);
    const entries = this.fileSystem.listDirectory(resolvedPath);
    return this.success(entries.join('\n'));
} catch (error: any) {
    return this.failure(error.message);
}
```

### 12.5 Error Result Structure

```typescript
export interface ShellCommandResult {
    stdout: string;
    stderr: string;
    exitCode: number;
}

// Success pattern
protected success(stdout: string = ''): ShellCommandResult {
    return {
        stdout: stdout ? stdout + '\n' : '',
        stderr: '',
        exitCode: 0
    };
}

// Error pattern
protected error(message: string, code: number = 1): ShellCommandResult {
    return {
        stdout: '',
        stderr: message + '\n',
        exitCode: code
    };
}
```

### 12.6 Error Propagation

```typescript
async execute(command: string, args: string[]): Promise<ShellCommandResult> {
    try {
        // Parse and execute
        const parsedCommand = this.parseCommand([command, ...args]);
        const result = await this.executeCommand(
            parsedCommand.command,
            parsedCommand.args
        );

        // Handle redirection errors
        if (result.exitCode === 0 && parsedCommand.redirects.length > 0) {
            try {
                this.handleRedirection(result.stdout, parsedCommand.redirects);
                result.stdout = '';
            } catch (error: any) {
                return {
                    stdout: '',
                    stderr: error.message,
                    exitCode: 1
                };
            }
        }

        return result;
    } catch (error: any) {
        return this.failure(error.message);
    }
}
```

---

## 13. Architecture Diagrams

### 13.1 Component Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                              Shell Architecture                             │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                         ShellProcess                                   │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                │ │
│  │  │ Input Handler│  │ Line Editor  │  │   History    │                │ │
│  │  │              │  │              │  │   Manager    │                │ │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘                │ │
│  └─────────┼─────────────────┼─────────────────┼────────────────────────┘ │
│            │                 │                 │                           │
│            ▼                 ▼                 ▼                           │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                            Shell                                       │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                │ │
│  │  │   Command    │  │    Path      │  │  Redirection │                │ │
│  │  │   Executor   │  │   Resolver   │  │   Handler    │                │ │
│  │  └──────┬───────┘  └──────────────┘  └──────────────┘                │ │
│  │         │                                                             │ │
│  │         ▼                                                             │ │
│  │  ┌─────────────────────────────────────────────────┐                 │ │
│  │  │              Command Registry                    │                 │ │
│  │  │  ┌─────────────────┐  ┌─────────────────┐      │                 │ │
│  │  │  │ Built-in Commands│  │External Commands│      │                 │ │
│  │  │  │                  │  │                 │      │                 │ │
│  │  │  │ • ls             │  │ • curl          │      │                 │ │
│  │  │  │ • cd             │  │ • wget          │      │                 │ │
│  │  │  │ • pwd            │  │ • unzip         │      │                 │ │
│  │  │  │ • mkdir          │  │                 │      │                 │ │
│  │  │  │ • rm             │  │                 │      │                 │ │
│  │  │  │ • cat            │  │                 │      │                 │ │
│  │  │  │ • echo           │  │                 │      │                 │ │
│  │  │  │ • touch          │  │                 │      │                 │ │
│  │  │  │ • cp             │  │                 │      │                 │ │
│  │  │  │ • mv             │  │                 │      │                 │ │
│  │  │  └─────────────────┘  └─────────────────┘      │                 │ │
│  │  └─────────────────────────────────────────────────┘                 │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                      │                                      │
│                                      ▼                                      │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                      IFileSystem Interface                            │ │
│  │  • writeFile()  • readFile()  • createDirectory()  • listDirectory()  │ │
│  │  • deleteFile() • isDirectory() • resolvePath()   • readBuffer()     │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                      │                                      │
│                                      ▼                                      │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                    Virtual File System (ZenFS)                        │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### 13.2 Command Execution Sequence

```
┌────────┐       ┌──────────────┐       ┌─────────┐       ┌─────────────┐
│ Terminal│       │ ShellProcess │       │  Shell  │       │  Command    │
└───┬────┘       └──────┬───────┘       └────┬────┘       └──────┬──────┘
    │                   │                    │                   │
    │ Write "ls -la"    │                    │                   │
    │──────────────────>│                    │                   │
    │                   │                    │                   │
    │                   │ Tokenize input     │                   │
    │                   │───────────┐        │                   │
    │                   │<──────────┘        │                   │
    │                   │                    │                   │
    │                   │ Execute command    │                   │
    │                   │───────────────────>│                   │
    │                   │                    │                   │
    │                   │                    │ Parse command     │
    │                   │                    │────────┐          │
    │                   │                    │<───────┘          │
    │                   │                    │                   │
    │                   │                    │ Lookup command    │
    │                   │                    │ in registry       │
    │                   │                    │                   │
    │                   │                    │ Create command    │
    │                   │                    │──────────────────>│
    │                   │                    │                   │
    │                   │                    │    Execute()      │
    │                   │                    │<──────────────────│
    │                   │                    │                   │
    │                   │                    │ ShellCommandResult│
    │                   │                    │<──────────────────│
    │                   │                    │                   │
    │                   │ Result             │                   │
    │                   │<───────────────────│                   │
    │                   │                    │                   │
    │ Display output    │                    │                   │
    │<──────────────────│                    │                   │
    │                   │                    │                   │
```

### 13.3 File Redirection Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    Redirection: echo "Hi" > out.txt              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌──────────┐     ┌───────────┐     ┌───────────┐            │
│   │  Parse   │────▶│  Execute  │────▶│ Redirect  │            │
│   │  Stage   │     │   Stage   │     │   Stage   │            │
│   └──────────┘     └───────────┘     └───────────┘            │
│        │                  │                 │                  │
│        ▼                  ▼                 ▼                  │
│   command: echo      Execute:         Write to file:          │
│   args: ["Hi"]       echo(["Hi"])       out.txt               │
│   redirects:         Result:           Content:               │
│     [{type: '>',      stdout: "Hi\n"     "Hi\n"               │
│       file: "out.txt"}] stderr: ""                            │
│                      exitCode: 0                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 14. Comparison with Bash/SH

### 14.1 Feature Comparison

| Feature | Bash | OpenWebContainer Shell |
|---------|------|------------------------|
| **Built-in Commands** | 50+ builtins | 9 built-ins |
| **External Commands** | System PATH | Virtual filesystem |
| **Variable Expansion** | `$VAR`, `${VAR}` | Not implemented |
| **Command Substitution** | `$(cmd)`, `` `cmd` `` | Not implemented |
| **Pipes** | `|`, `|&` | Not implemented |
| **Redirection** | `>`, `>>`, `2>`, `2>&1`, `&>` | `>`, `>>` |
| **Globbing** | `*`, `?`, `[]` | Not implemented |
| **Job Control** | `&`, `bg`, `fg`, `jobs` | Not implemented |
| **History** | `!`, `!!`, `!n` | Arrow keys only |
| **Tab Completion** | Yes | Not implemented |
| **Aliases** | `alias name=cmd` | Not implemented |
| **Functions** | Yes | Not implemented |
| **Conditionals** | `if`, `case`, `test` | Not implemented |
| **Loops** | `for`, `while`, `until` | Not implemented |
| **ANSI Colors** | Yes | OSC mode (optional) |
| **Process Substitution** | `<(cmd)`, `>(cmd)` | Not implemented |

### 14.2 Syntax Comparison

#### Basic Command

```bash
# Bash
$ ls -la /app

# OpenWebContainer Shell
$ ls -la /app
```

Both shells have identical basic syntax.

#### Redirection

```bash
# Bash
$ echo "Hello" > file.txt
$ echo "World" >> file.txt
$ command 2>&1 > output.txt

# OpenWebContainer Shell
$ echo "Hello" > file.txt
$ echo "World" >> file.txt
# 2>&1 not supported
```

#### Pipes

```bash
# Bash
$ ls -la | grep ".ts" | head -n 5

# OpenWebContainer Shell
# Not supported (would need to run as separate commands)
$ ls -la > temp.txt
$ cat temp.txt | grep ".ts"  # Also not supported
```

#### Variables

```bash
# Bash
$ export MY_VAR="hello"
$ echo $MY_VAR

# OpenWebContainer Shell
# Environment variables exist but expansion not supported
$ echo $MY_VAR  # Would print literal "$MY_VAR"
```

### 14.3 Performance Considerations

| Aspect | Bash | OpenWebContainer Shell |
|--------|------|------------------------|
| **Startup Time** | ~1-5ms | ~10-50ms (browser) |
| **Command Execution** | Native speed | JavaScript speed |
| **Memory Usage** | ~1-5MB | ~10-50MB |
| **Filesystem Access** | Direct syscalls | Virtual FS layer |
| **Network Access** | Real sockets | Fetch API + CORS |

### 14.4 Security Model

| Aspect | Bash | OpenWebContainer Shell |
|--------|------|------------------------|
| **Isolation** | User permissions | Browser sandbox |
| **Filesystem** | Full system access | Virtual FS only |
| **Network** | Unrestricted | CORS-limited Fetch |
| **Process Spawn** | Any executable | Registered executors only |
| **Escape Risk** | User can access system | Browser containment |

### 14.5 Implementation Differences

```typescript
// Bash: Written in C
// Compiled binary, direct system calls
// Uses fork()/exec() for processes

// OpenWebContainer Shell: Written in TypeScript
// Runs in browser JavaScript engine
// Uses Web Workers for concurrency
// Virtual filesystem via ZenFS
// Network via Fetch API
```

---

## Appendix A: File Reference

### Source Files

| File | Path | Purpose |
|------|------|---------|
| Shell | `src/shell/shell.ts` | Main shell class |
| Types | `src/shell/types.ts` | TypeScript interfaces |
| Base Command | `src/shell/commands/base.ts` | Command base class |
| Registry | `src/shell/commands/registry.ts` | Command registry |
| curl | `src/shell/commands/curl.ts` | HTTP requests |
| wget | `src/shell/commands/wget.ts` | File downloads |
| unzip | `src/shell/commands/unzip.ts` | Archive extraction |
| ShellProcess | `src/process/executors/shell/process.ts` | Interactive shell |
| ShellExecutor | `src/process/executors/shell/executor.ts` | Process executor |

### Key Interfaces

```typescript
// ShellCommandResult
interface ShellCommandResult {
    stdout: string;
    stderr: string;
    exitCode: number;
}

// CommandParsedResult
interface CommandParsedResult {
    command: string;
    args: string[];
    redirects: {
        type: '>>' | '>';
        file: string;
    }[];
}

// CommandOptions
interface CommandOptions {
    cwd: string;
    fileSystem: IFileSystem;
    env?: Map<string, string>;
    process: Process;
}

// CommandHelp
interface CommandHelp {
    name: string;
    description: string;
    usage: string;
    examples: string[];
}
```

---

## Appendix B: Quick Reference Card

### Built-in Commands

```
ls [path]              List directory
cd [path]              Change directory
pwd                    Print working directory
mkdir <path>           Create directory
touch <file>           Create empty file
rm [-r] <file>         Remove file
rmdir <dir>            Remove directory
cat <file>             Display file content
echo <text>            Display text
cp <src> <dest>        Copy file
mv <src> <dest>        Move file
```

### External Commands

```
curl [options] URL     HTTP request
wget [options] URL     Download file
unzip [options] FILE   Extract archive
```

### Redirection

```
> file     Redirect stdout to file (overwrite)
>> file    Redirect stdout to file (append)
```

### Special Keys

```
↑/↓        Command history
←/→        Cursor movement
Ctrl+C     Cancel current line
Ctrl+D     Exit shell
Enter      Execute command
```

---

*Generated: 2026-04-05*
*Source Location: `/home/darkvoid/Boxxed/@formulas/src.opencontainer/OpenWebContainer/packages/core/src/shell/`*
