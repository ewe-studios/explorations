# BTCA Core - Deep Dive

## Overview

The main `better-context` monorepo contains the core BTCA (Better Context to AI) system - a comprehensive AI-powered codebase exploration tool.

---

## Monorepo Structure

```
better-context/
├── apps/
│   ├── cli/              # CLI tool (TUI + REPL)
│   ├── server/           # API server + AI agent
│   ├── web/              # Web app (SvelteKit + Convex)
│   ├── sandbox/          # Daytona sandbox integration
│   └── analytics-proxy/  # PostHog analytics proxy
├── packages/
│   └── shared/           # Shared utilities
└── btca.config.jsonc     # Default resource configuration
```

---

## apps/cli - Command Line Interface

### Architecture

```
apps/cli/src/
├── commands/
│   ├── ask.ts       # One-shot questions
│   ├── chat.ts      # Interactive chat
│   ├── tui.ts       # Terminal UI mode
│   ├── repl.ts      # REPL mode
│   ├── add.ts       # Add resources
│   ├── remove.ts    # Remove resources
│   ├── connect.ts   # Configure providers
│   ├── init.ts      # Initialize project
│   └── serve.ts     # Start standalone server
├── client/
│   ├── index.ts     # HTTP client
│   ├── stream.ts    # SSE stream parsing
│   └── remote.ts    # Remote API client
├── server/
│   └── manager.ts   # Auto-start server manager
└── tui/             # Terminal UI components
```

### Key Features

**@mention Syntax:**
```bash
# Use @mentions in questions
btca ask -q "How does @svelte handle state?"

# Multiple resources
btca ask -q "Compare @svelte vs @solidJs reactivity"
```

**Stream Event Handling:**
```typescript
// CLI parses SSE events from server
for await (const event of parseSSEStream(response)) {
  switch (event.type) {
    case 'meta': console.log('creating collection...'); break;
    case 'reasoning.delta': showThinking(delta); break;
    case 'text.delta': showAnswer(delta); break;
    case 'tool.updated': showToolCall(event.tool); break;
  }
}
```

---

## apps/server - AI Agent Server

### Core Modules

```
apps/server/src/
├── agent/
│   ├── loop.ts      # AI SDK streamText loop
│   ├── service.ts   # Agent service factory
│   └── types.ts     # Type definitions
├── tools/
│   ├── read.ts      # Read file contents
│   ├── grep.ts      # Regex search
│   ├── glob.ts      # File pattern matching
│   ├── list.ts      # Directory listing
│   └── context.ts   # Tool context type
├── resources/
│   ├── service.ts   # Resource loading
│   ├── schema.ts    # Resource schemas
│   ├── impls/
│   │   └── git.ts   # Git resource implementation
│   └── helpers.ts   # Resource helpers
├── vfs/
│   └── virtual-fs.ts # Virtual filesystem
├── providers/
│   ├── registry.ts  # Provider registry
│   ├── auth.ts      # Authentication
│   └── model.ts     # Model instantiation
├── stream/
│   ├── service.ts   # Streaming service
│   └── types.ts     # Stream event types
└── context/
    └── index.ts     # AsyncLocalStorage context
```

### Agent Loop Implementation

The agent uses Vercel's AI SDK with custom tools:

```typescript
// Build system prompt
function buildSystemPrompt(agentInstructions: string): string {
  return [
    'You are btca, an expert documentation search agent.',
    'Your job is to answer questions by searching through the collection.',
    '',
    'You have access to: read, grep, glob, list tools',
    '',
    'Guidelines:',
    '- Use glob to find relevant files first, then read them',
    '- Use grep to search for specific code patterns',
    '- Always cite source files in your answers',
    agentInstructions
  ].join('\n');
}

// Create tools
function createTools(basePath: string, vfsId?: string) {
  return {
    read: tool({
      description: 'Read file contents with line numbers',
      inputSchema: ReadTool.Parameters,
      execute: async (params) => {
        const result = await ReadTool.execute(params, { basePath, vfsId });
        return result.output;
      }
    }),
    // ... grep, glob, list
  };
}

// Stream text with tools
const result = streamText({
  model: await Model.getModel(providerId, modelId),
  system: buildSystemPrompt(agentInstructions),
  messages: [{ role: 'user', content: question }],
  tools: createTools(collectionPath, vfsId),
  stopWhen: stepCountIs(maxSteps)
});
```

---

## Virtual Filesystem (VFS)

The VFS provides isolated filesystem operations:

```typescript
// VFS singleton with multiple instances
const instances = new Map<string, InMemoryFs>();

export namespace VirtualFs {
  export const create = () => {
    const vfsId = randomUUID();
    instances.set(vfsId, new InMemoryFs());
    return vfsId;
  };

  export async function importDirectoryFromDisk(args: {
    sourcePath: string;
    destinationPath: string;
    ignore?: (relativePath: string) => boolean;
    vfsId?: string;
  }) {
    // Recursively import directory into VFS
    const walk = async (currentPath: string) => {
      // ... recursive import logic
    };
    await walk(base);
  }

  export async function listFilesRecursive(rootPath: string, vfsId?: string) {
    // BFS file listing
  }
}
```

**Key Operations:**
- `create()` - Create new isolated VFS instance
- `importDirectoryFromDisk()` - Import real files into VFS
- `listFilesRecursive()` - List all files in directory
- `readFile()` / `writeFile()` - File operations
- `dispose()` - Clean up VFS instance

---

## Tool Implementations

### Read Tool

```typescript
// Configuration
const MAX_LINES = 2000;
const MAX_BYTES = 50 * 1024; // 50KB
const MAX_LINE_LENGTH = 2000;

export async function execute(params, context): Promise<Result> {
  const { basePath, vfsId } = context;
  const resolvedPath = await VirtualSandbox.resolvePathWithSymlinks(basePath, params.path, vfsId);

  // Handle special file types
  const ext = path.extname(resolvedPath);
  if (IMAGE_EXTENSIONS.has(ext)) {
    // Return base64 encoded image
    return { attachments: [{ type: 'file', mime, data: base64 }] };
  }
  if (isBinaryBuffer(await VirtualFs.readFileBuffer(resolvedPath, vfsId))) {
    return { output: '[Binary file: ...]' };
  }

  // Read with truncation
  const text = await VirtualFs.readFile(resolvedPath, vfsId);
  const lines = text.split('\n');
  // Apply offset, limit, byte limits
  // Format with line numbers
}
```

### Grep Tool

```typescript
const MAX_RESULTS = 100;

export async function execute(params, context): Promise<Result> {
  const regex = new RegExp(params.pattern);
  const includeMatcher = params.include ? buildIncludeMatcher(params.include) : null;

  const allFiles = await VirtualFs.listFilesRecursive(searchPath, vfsId);
  const results = [];

  for (const filePath of allFiles) {
    if (includeMatcher && !includeMatcher(path.relative(searchPath, filePath))) continue;
    if (isBinaryBuffer(await VirtualFs.readFileBuffer(filePath, vfsId))) continue;

    const text = await VirtualFs.readFile(filePath, vfsId);
    for (const [i, line] of text.split('\n').entries()) {
      if (regex.test(line)) {
        results.push({ path: filePath, lineNumber: i + 1, lineText: line });
      }
      if (results.length > MAX_RESULTS) break;
    }
  }

  // Group by file, sort by mtime, format output
}
```

---

## Resource Management

### Resource Loading Flow

```typescript
// Resources service
export const create = (config: Config.Service): Service => ({
  load: async (name, { quiet }) => {
    const definition = config.getResource(name);

    if (isGitResource(definition)) {
      // Clone/fetch git repo
      return loadGitResource({
        url: definition.url,
        branch: definition.branch,
        repoSubPaths: normalizeSearchPaths(definition),
        resourcesDirectoryPath: config.resourcesDirectory,
        quiet
      });
    } else {
      // Local resource
      return loadLocalResource({
        path: definition.path,
        name: definition.name
      });
    }
  }
});
```

### Git Resource Implementation

```typescript
// apps/server/src/resources/impls/git.ts
export async function loadGitResource(args: BtcaGitResourceArgs): Promise<BtcaFsResource> {
  const repoPath = path.join(args.resourcesDirectoryPath, args.name);

  if (await exists(repoPath)) {
    // Fetch latest changes
    await Bash.run(`git -C ${repoPath} fetch origin ${args.branch}`);
    await Bash.run(`git -C ${repoPath} checkout ${args.branch}`);
  } else {
    // Clone repo
    await Bash.run(`git clone ${args.url} ${repoPath} --depth 1`);
  }

  return {
    _tag: 'fs-based',
    name: args.name,
    getAbsoluteDirectoryPath: async () => repoPath
  };
}
```

---

## Streaming Service

### SSE Stream Events

```typescript
// Stream event schema
export const BtcaStreamEventSchema = z.discriminatedUnion('type', [
  BtcaStreamMetaEventSchema,        // { type: 'meta' }
  BtcaStreamTextDeltaEventSchema,   // { type: 'text.delta', delta: string }
  BtcaStreamReasoningDeltaEventSchema, // { type: 'reasoning.delta', delta: string }
  BtcaStreamToolUpdatedEventSchema, // { type: 'tool.updated', ... }
  BtcaStreamDoneEventSchema,        // { type: 'done' }
  BtcaStreamErrorEventSchema        // { type: 'error', message: string }
]);

// Streaming handler
for await (const part of result.fullStream) {
  switch (part.type) {
    case 'text-delta':
      yield { type: 'text-delta', text: part.text };
      break;
    case 'tool-call':
      yield { type: 'tool.updated', tool: part.toolName, state: { status: 'running' } };
      break;
    case 'finish':
      yield { type: 'done', usage: part.totalUsage };
      break;
  }
}
```

---

## Configuration System

### btca.config.jsonc

```jsonc
{
  "$schema": "https://btca.dev/btca.schema.json",
  "dataDirectory": ".btca",
  "provider": "opencode",
  "model": "claude-haiku-4-5",
  "resources": [
    {
      "type": "git",
      "name": "svelte",
      "url": "https://github.com/sveltejs/svelte.dev",
      "branch": "main",
      "searchPaths": ["apps/svelte.dev"],
      "specialNotes": "Focus on content directory"
    }
  ]
}
```

### Config Merging

1. Global config loaded first (`~/.config/btca/btca.config.jsonc`)
2. Project config merged on top
3. Project values override global on conflict
4. Resources combined (project version wins on name conflict)

---

## Production Considerations for Rust Implementation

### Key Requirements

1. **Virtual Filesystem**: Need efficient in-memory FS (consider `rusty-fork` or custom implementation)
2. **Git Operations**: Use `git2` crate for resource management
3. **Streaming**: SSE via `axum` or `warp` with `tokio-stream`
4. **AI SDK Integration**: Bridge to Vercel AI SDK or implement native Rust streaming
5. **Regex Search**: Use `regex` crate (much faster than JavaScript)
6. **Glob Matching**: Use `globset` crate

### Architecture Recommendations

```
btca-rs/
├── crates/
│   ├── btca-core/      # Core agent logic
│   ├── btca-vfs/       # Virtual filesystem
│   ├── btca-tools/     # read, grep, glob, list tools
│   ├── btca-server/    # HTTP server + SSE
│   └── btca-cli/       # CLI with clap
└── resources/          # Cached git resources
```

### Performance Optimizations

- Use `memmap2` for memory-mapped file reading
- `ripgrep`-style parallel search for grep
- `dashmap` for concurrent VFS access
- `tantivy` for full-text search indexes (optional)
