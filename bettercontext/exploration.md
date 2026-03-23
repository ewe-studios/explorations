# BetterContext (BTCA) - Exploration

## Overview

**BetterContext (BTCA)** is a comprehensive AI-powered codebase exploration and documentation system. It enables AI agents to answer questions about codebases by providing context-aware search capabilities through a combination of local and remote execution modes.

The core insight: AI coding agents need **persistent, searchable context** about frameworks, libraries, and codebases to provide accurate answers without hallucination.

---

## Project Structure

The BetterContext source contains multiple sub-projects:

| Sub-Project | Description |
|-------------|-------------|
| **better-context/** | Main BTCA monorepo - CLI, server, web app, sandbox |
| **bettercontextoai/** | VS Code extension for context generation |
| **river/** | Streaming library for AI agent communication |
| **r8y/** | Full-stack TypeScript application (video platform) |
| **ai-sdk-ex/** | Elixir AI SDK for streaming with tool calls |
| **search-ex/** | Cloudflare Worker for LLM-friendly Hex docs |
| **grep-bench/** | Benchmark harness for AI model search performance |

---

## Core Architecture

### Two Execution Modes

BTCA operates in two modes:

#### 1. Local Mode
- **Interface**: CLI (TUI or REPL)
- **Agent Location**: User's machine
- **Repo Storage**: Local filesystem (`.btca/` or `~/.local/share/btca/`)
- **Auth**: OpenCode auth system (user's API keys)
- **Cost**: Token cost through user's provider subscription

#### 2. Remote Mode
- **Interface**: Web app + MCP (Model Context Protocol)
- **Agent Location**: Daytona cloud sandbox
- **Repo Storage**: Cloud (cached per project)
- **Auth**: BTCA API key (subscription-based, $8/mo)
- **Cost**: Subscription covers token costs

### Shared Components

Both modes use identical agent code in `apps/server`:

```
apps/server/
├── src/agent/       # AI SDK streamText loop
├── src/tools/       # read, grep, glob, list tools
├── src/providers/   # AI provider authentication
├── src/resources/   # Git/local resource management
├── src/vfs/         # Virtual filesystem layer
├── src/stream/      # SSE streaming service
└── src/context/     # AsyncLocalStorage context
```

---

## Context Management Mechanisms

### 1. Resource System

Resources define what codebases the agent can search:

```jsonc
{
  "type": "git",
  "name": "svelte",
  "url": "https://github.com/sveltejs/svelte.dev",
  "branch": "main",
  "searchPaths": ["apps/svelte.dev"],
  "specialNotes": "Focus on content directory"
}
```

**Resource types:**
- **Git resources**: Clone GitHub repos to local cache
- **Local resources**: Point to local directories

### 2. Virtual Filesystem (VFS)

The `VirtualFs` module provides an in-memory filesystem layer using `just-bash`'s `InMemoryFs`:

```typescript
// Key VFS operations
VirtualFs.mkdir(path, options, vfsId)
VirtualFs.writeFile(path, data, vfsId)
VirtualFs.readFile(path, vfsId)
VirtualFs.readdir(path, vfsId)
VirtualFs.listFilesRecursive(rootPath, vfsId)
VirtualFs.importDirectoryFromDisk({ sourcePath, destinationPath, vfsId })
```

**Benefits:**
- Isolated execution per query
- No filesystem side effects
- Automatic cleanup after queries
- Support for symlinks and complex structures

### 3. Collection System

Resources are loaded into "collections" - temporary working directories:

```typescript
// Resource loading flow
Resources.load(name, { quiet }) -> BtcaFsResource
  ├─ Git resources: Clone/fetch to resources directory
  ├─ Local resources: Point to existing path
  └─ Virtual collections: In-memory FS for isolation
```

### 4. Transaction Context

Uses `AsyncLocalStorage` for request isolation:

```typescript
type ContextStore = {
  requestId: string;
  txDepth: number;
};

Context.run(store, async () => {
  // All async operations have access to context
});
```

---

## AI Agent Loop

The agent uses Vercel's **AI SDK** `streamText` with custom tools:

```typescript
// Core agent loop (simplified)
const result = streamText({
  model: await Model.getModel(providerId, modelId),
  system: buildSystemPrompt(agentInstructions),
  messages: [{ role: 'user', content: question }],
  tools: {
    read: tool({ execute: ReadTool.execute }),
    grep: tool({ execute: GrepTool.execute }),
    glob: tool({ execute: GlobTool.execute }),
    list: tool({ execute: ListTool.execute })
  },
  stopWhen: stepCountIs(maxSteps)
});
```

### Tool Definitions

| Tool | Description |
|------|-------------|
| `read` | Read file contents with line numbers, truncation |
| `grep` | Regex search across files |
| `glob` | Find files matching patterns |
| `list` | List directory contents |

### Streaming Events

The agent emits structured events:

```typescript
type BtcaStreamEvent =
  | { type: 'meta' }
  | { type: 'reasoning.delta'; delta: string }
  | { type: 'text.delta'; delta: string }
  | { type: 'tool.updated'; tool: string; state: ToolState }
  | { type: 'error'; message: string }
  | { type: 'done' };
```

---

## How BTCA Improves AI Code Understanding

### 1. Escape Context Limitations

VS Code AI extensions often have restrictive context windows. BTCA enables:
- 100K+ token context through web AI models
- Persistent resource caching across sessions
- No manual copy-paste of files

### 2. Structured Search Workflow

The agent follows a deterministic pattern:
1. **glob** - Find relevant files first
2. **grep** - Search for specific patterns
3. **read** - Read targeted file contents
4. **Synthesize** - Combine findings with citations

### 3. Smart Filtering

- Files over 50KB automatically truncated
- Binary files/images detected and handled
- Line limits (2000 lines) with offset continuation
- Maximum 100 grep results with sorting by mtime

### 4. Multi-Turn Reasoning

The agent can:
- Use up to 40 steps (`maxSteps`)
- See previous tool results
- Refine search based on findings
- Admit when unable to find answers

---

## WASM Usage

Currently, BTCA does **not** use WASM. The architecture is pure TypeScript/Bun:

- **Runtime**: Bun (not Node.js)
- **AI SDK**: Vercel's `ai` package
- **Virtual FS**: `just-bash` InMemoryFs (JavaScript-based)
- **No WASM dependencies** found in the codebase

However, the architecture could benefit from WASM for:
- Faster regex search (Rust regex via WASM)
- Efficient glob matching
- In-memory search indexes

---

## Provider Support

BTCA supports multiple AI providers through the AI SDK:

```typescript
// Supported providers
@ai-sdk/anthropic    // claude-sonnet-4-5, claude-haiku-4-5
@ai-sdk/openai       // gpt-4o, gpt-5.1-codex
@ai-sdk/google       // gemini models
@ai-sdk/groq         // Fast inference
@ai-sdk/openai-compatible  // Custom endpoints
```

Authentication via OpenCode's auth storage system.

---

## CLI Commands

```bash
# Launch TUI
btca

# Ask a question
btca ask -r svelte -q "How does $state rune work?"

# Add a resource
btca add https://github.com/sveltejs/svelte

# Configure model
btca connect -p anthropic -m claude-sonnet-4-5

# Start server
btca serve --port 8080
```

---

## Key Files

| File | Purpose |
|------|---------|
| `apps/server/src/agent/loop.ts` | Core AI agent loop |
| `apps/server/src/vfs/virtual-fs.ts` | Virtual filesystem |
| `apps/server/src/tools/*.ts` | Tool implementations |
| `apps/cli/src/commands/ask.ts` | CLI ask command |
| `btca.config.jsonc` | Resource configuration |

---

## Conclusion

BetterContext represents a sophisticated approach to AI-assisted codebase exploration:

1. **Dual-mode architecture** - Local for development, remote for production
2. **Virtual filesystem** - Isolated, clean query execution
3. **Structured tool use** - Deterministic search patterns
4. **Provider agnostic** - Works with any AI SDK provider
5. **Streaming-first** - Real-time feedback during searches

The system demonstrates how to build production-grade AI tooling that respects context limitations while maximizing AI understanding of complex codebases.
