# free-code Exploration

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code`  
**Repository:** https://github.com/paoloanzn/free-code  
**Explored at:** 2026-04-02

---

## Table of Contents

1. [Project Overview](#project-overview)
2. [Architecture](#architecture)
3. [Directory Structure](#directory-structure)
4. [Key Components](#key-components)
5. [Execution Flow](#execution-flow)
6. [Dependencies](#dependencies)
7. [Configuration](#configuration)
8. [Testing Strategy](#testing-strategy)

---

## Project Overview

**free-code** is a modified build of Claude Code — Anthropic's terminal-native AI coding agent. This fork applies three categories of changes:

1. **Telemetry removed** — All OpenTelemetry, GrowthBook reporting, and Sentry eliminated
2. **Guardrails stripped** — System prompt injections and server-side restrictions removed
3. **Features unlocked** — All 54 compile-clean experimental feature flags enabled

### Tech Stack

| Layer | Technology |
|-------|------------|
| Runtime | Bun >= 1.3.11 |
| Language | TypeScript |
| Terminal UI | React + Ink |
| CLI Parsing | Commander.js |
| Schema Validation | Zod v4 |
| Code Search | ripgrep (bundled) |
| Protocols | MCP, LSP |

### Key Statistics

- **Source files**: ~400 TypeScript/TSX files
- **Lines of code**: ~100K+ (main.tsx alone is 804KB)
- **Dependencies**: 90+ npm packages
- **Feature flags**: 88 total (54 working, 34 broken)
- **Commands**: 80+ slash commands
- **Tools**: 40+ built-in tools

---

## Architecture

### System Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                         User Terminal                               │
│                    (stdin/stdout TUI)                               │
└─────────────────────────┬──────────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────────┐
│                      CLI Entrypoint                                 │
│                  src/entrypoints/cli.tsx                           │
│  • Fast-path dispatch (--version, --daemon, --bridge, etc.)        │
│  • Feature flag gates (bun:bundle)                                 │
│  • Module lazy-loading                                             │
└─────────────────────────┬──────────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────────┐
│                       Main Loop                                     │
│                      src/main.tsx                                   │
│  • Session initialization                                          │
│  • Config loading                                                  │
│  • Auth verification                                               │
│  • REPL rendering (Ink)                                            │
└─────────────────────────┬──────────────────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               ▼               ▼
┌─────────────────┐ ┌─────────────┐ ┌─────────────────┐
│   Commands      │ │    Tools    │ │    Services     │
│  /commands/     │ │  /tools/    │ │   /services/    │
│  • /login       │ │  • Bash     │ │  • API client   │
│  • /model       │ │  • Read     │ │  • MCP server   │
│  • /help        │ │  • Edit     │ │  • OAuth        │
│  • /compact     │ │  • Grep     │ │  • Telemetry*   │
│  • /plan        │ │  • Agent    │ │  • Storage      │
└─────────────────┘ └─────────────┘ └─────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────────┐
│                     Query Engine                                    │
│                    src/QueryEngine.ts                               │
│  • System prompt construction                                      │
│  • Context assembly (files, memory, hooks)                         │
│  • API request formatting                                          │
│  • Response streaming                                              │
│  • Tool call parsing                                               │
└─────────────────────────┬──────────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────────┐
│                   External APIs                                     │
│  • Anthropic Messages API (default)                                │
│  • OpenAI Codex API (optional)                                     │
│  • AWS Bedrock (optional)                                          │
│  • Google Vertex AI (optional)                                     │
│  • Anthropic Foundry (optional)                                    │
└────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
User Input
    │
    ▼
┌─────────────────┐
│  PromptInput    │ ◄─── Vim mode, keybindings, history
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ handlePrompt    │ ◄─── Slash command parsing
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  QueryEngine    │ ◄─── Context assembly
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  API Client     │ ◄─── Provider adapter (claude.ts / codex-fetch-adapter.ts)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  LLM Response   │
│  (SSE stream)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Stream Parser  │ ◄─── Tool calls, thinking, text
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
┌────────┐  ┌──────────┐
│ Text   │  │ Tool     │
│ Output │  │ Call     │
└────────┘  └────┬─────┘
                 │
                 ▼
          ┌──────────────┐
          │ Permission   │ ◄─── User approval (if needed)
          │ Check        │
          └──────┬───────┘
                 │
                 ▼
          ┌──────────────┐
          │  Tool        │
          │  Executor    │
          └──────┬───────┘
                 │
                 ▼
          ┌──────────────┐
          │  Result      │──────┐
          └──────────────┘      │
                                │
                                └──► Back to API (next turn)
```

---

## Directory Structure

```
free-code/
├── scripts/
│   └── build.ts                    # Build script with feature flag system
│
├── src/
│   ├── entrypoints/
│   │   └── cli.tsx                 # CLI entrypoint (fast-path dispatch)
│   │
│   ├── commands.ts                 # Command registry (80+ commands)
│   ├── tools.ts                    # Tool registry (40+ tools)
│   ├── Tool.ts                     # Tool base class & types
│   ├── QueryEngine.ts              # LLM query engine
│   ├── query.ts                    # Query processing utilities
│   ├── main.tsx                    # Main application entry
│   │
│   ├── screens/
│   │   └── REPL.tsx                # Main interactive REPL UI (17KB)
│   │
│   ├── commands/                   # Slash command implementations
│   │   ├── login/                  # OAuth login flow
│   │   ├── model/                  # Model selection
│   │   ├── config/                 # Configuration management
│   │   ├── mcp/                    # MCP server management
│   │   ├── compact/                # Context compaction
│   │   ├── plan/                   # Plan mode toggle
│   │   ├── ultraplan.tsx           # UltraPlan multi-agent (67KB)
│   │   ├── init.ts                 # Project initialization
│   │   └── ...                     # 80+ command directories
│   │
│   ├── tools/                      # Tool implementations
│   │   ├── BashTool/               # Shell command execution
│   │   ├── FileReadTool/           # File reading
│   │   ├── FileEditTool/           # File editing (multi-strategy)
│   │   ├── FileWriteTool/          # File writing
│   │   ├── GrepTool/               # Content search
│   │   ├── GlobTool/               # File pattern matching
│   │   ├── AgentTool/              # Sub-agent spawning
│   │   ├── TaskCreateTool/         # Task management
│   │   ├── TodoWriteTool/          # Todo lists
│   │   ├── WebFetchTool/           # Web page fetching
│   │   ├── WebSearchTool/          # Web search
│   │   ├── MCPTool/                # MCP tool invocation
│   │   └── ...                     # 40+ tool directories
│   │
│   ├── services/                   # Background services
│   │   ├── api/
│   │   │   ├── claude.ts           # Anthropic API client (126KB)
│   │   │   ├── codex-fetch-adapter.ts  # OpenAI Codex adapter (28KB)
│   │   │   ├── client.ts           # HTTP client
│   │   │   ├── errors.ts           # API error handling
│   │   │   └── ...
│   │   ├── mcp/                    # Model Context Protocol
│   │   ├── oauth/                  # OAuth flows (Anthropic + OpenAI)
│   │   ├── compact/                # Context compaction
│   │   ├── lsp/                    # Language Server Protocol
│   │   └── ...
│   │
│   ├── components/                 # Ink/React UI components
│   │   ├── Messages.tsx            # Message display
│   │   ├── PromptInput/            # Input component
│   │   ├── Permissions/            # Permission dialogs
│   │   ├── Spinner.tsx             # Loading indicators
│   │   └── ...
│   │
│   ├── state/                      # Application state
│   │   └── AppState.ts             # Global state management
│   │
│   ├── bootstrap/                  # Bootstrap/initialization
│   │   └── state.ts                # Session state
│   │
│   ├── bridge/                     # Remote control / IDE bridge
│   │   ├── bridgeMain.ts           # Bridge entrypoint
│   │   ├── bridgeMessaging.ts      # Message protocol
│   │   ├── replBridge.ts           # REPL bridge transport
│   │   └── ...                     # 30+ files
│   │
│   ├── utils/                      # Utilities (200+ files)
│   │   ├── model/
│   │   │   └── providers.ts        # Provider detection
│   │   ├── permissions/
│   │   │   └── permissions.ts      # Permission system
│   │   ├── messages.ts             # Message utilities (193KB)
│   │   ├── sessionStorage.ts       # Session persistence (181KB)
│   │   ├── config.ts               # Configuration (64KB)
│   │   ├── auth.ts                 # Authentication (68KB)
│   │   └── ...
│   │
│   ├── hooks/                      # React hooks
│   │   ├── useMergedTools.ts       # Tool merging
│   │   ├── useMailboxBridge.ts     # Mailbox communication
│   │   └── ...
│   │
│   ├── tasks/                      # Task management
│   │   ├── LocalAgentTask/         # Local agent tasks
│   │   ├── RemoteAgentTask/        # Remote agent tasks
│   │   └── ...
│   │
│   ├── skills/                     # Skill system
│   ├── plugins/                    # Plugin system
│   ├── voice/                      # Voice input
│   └── vim/                        # Vim mode
│
├── assets/                         # Static assets
├── bun.lock                        # Bun lockfile
├── package.json                    # Dependencies
├── tsconfig.json                   # TypeScript config
├── FEATURES.md                     # Feature flag audit
├── README.md                       # Project documentation
└── install.sh                      # Install script
```

---

## Key Components

### 1. CLI Entrypoint (`src/entrypoints/cli.tsx`)

The entrypoint implements **fast-path dispatch** for common operations:

```typescript
async function main(): Promise<void> {
  const args = process.argv.slice(2)

  // Fast-path: --version (zero module loading)
  if (args.length === 1 && (args[0] === '--version' || args[0] === '-v')) {
    console.log(`${MACRO.VERSION} (Claude Code)`)
    return
  }

  // Fast-path: --daemon-worker (supervisor-spawned)
  if (feature('DAEMON') && args[0] === '--daemon-worker') {
    const { runDaemonWorker } = await import('../daemon/workerRegistry.js')
    await runDaemonWorker(args[1])
    return
  }

  // Fast-path: remote-control / bridge
  if (feature('BRIDGE_MODE') && isBridgeCommand(args[0])) {
    await bridgeMain(args.slice(1))
    return
  }

  // Normal CLI path
  const { main: cliMain } = await import('../main.js')
  await cliMain()
}
```

**Key features:**
- Zero-import fast paths for `--version`, `--daemon`, `--bridge`
- Feature flag gates via `bun:bundle` feature()
- Lazy module loading for slow paths

### 2. Command Registry (`src/commands.ts`)

Manages **80+ slash commands**:

```typescript
export async function getCommands(cwd: string): Promise<Command[]> {
  const allCommands = await loadAllCommands(cwd)

  // Filter by availability (auth/provider requirements)
  const baseCommands = allCommands.filter(
    _ => meetsAvailabilityRequirement(_) && isCommandEnabled(_)
  )

  // Add dynamic skills discovered during file operations
  const dynamicSkills = getDynamicSkills()
  // ...dedupe and insert
}
```

**Command types:**
- `prompt` — Expands to text sent to model (skills)
- `local` — Local-only text output
- `local-jsx` — Renders Ink UI (blocked in remote mode)

**Remote-safe commands** (work over bridge):
```typescript
export const REMOTE_SAFE_COMMANDS: Set<Command> = new Set([
  session, exit, clear, help, theme, color, vim,
  cost, usage, copy, btw, feedback, plan,
  keybindings, statusline, stickers, mobile,
])
```

### 3. Tool Registry (`src/tools.ts`)

Manages **40+ built-in tools** plus MCP tools:

```typescript
export function getAllBaseTools(): Tools {
  return [
    AgentTool,
    TaskOutputTool,
    BashTool,
    ...(hasEmbeddedSearchTools() ? [] : [GlobTool, GrepTool]),
    ExitPlanModeV2Tool,
    FileReadTool,
    FileEditTool,
    FileWriteTool,
    NotebookEditTool,
    WebFetchTool,
    TodoWriteTool,
    WebSearchTool,
    // ...feature-gated tools
  ]
}

export function assembleToolPool(
  permissionContext: ToolPermissionContext,
  mcpTools: Tools,
): Tools {
  const builtInTools = getTools(permissionContext)
  const allowedMcpTools = filterToolsByDenyRules(mcpTools, permissionContext)

  // Deduplicate (built-in tools take precedence)
  return uniqBy(
    [...builtInTools].sort(byName).concat(allowedMcpTools.sort(byName)),
    'name',
  )
}
```

### 4. Query Engine (`src/QueryEngine.ts`)

The core **LLM interaction engine** (47KB):

```typescript
export async function query(params: QueryParams): Promise<QueryResult> {
  // 1. Build system prompt
  const systemPrompt = await buildEffectiveSystemPrompt({
    tools,
    model,
    permissionContext,
  })

  // 2. Assemble context (files, memory, hooks)
  const context = await assembleContext({
    readFiles,
    memoryFiles,
    hookResults,
  })

  // 3. Format API request
  const request = formatApiRequest({
    systemPrompt,
    messages,
    context,
    tools,
  })

  // 4. Stream response
  const stream = await apiClient.messages.create(request)

  // 5. Parse and handle response
  for await (const event of stream) {
    if (event.type === 'content_block_start') {
      // Handle tool calls, text, thinking
    }
  }
}
```

### 5. REPL UI (`src/screens/REPL.tsx`)

The main **interactive terminal UI** (17KB+):

```typescript
export function REPL() {
  const { messages, tools, commands } = useAppState()
  const [input, setInput] = useState('')

  // Handle prompt submission
  const handleSubmit = async (prompt: string) => {
    addToHistory(prompt)
    await handlePromptSubmit(prompt, {
      onToolCall: handleToolCall,
      onStreamEvent: handleStreamEvent,
    })
  }

  return (
    <Box flexDirection="column">
      <Messages messages={messages} />
      <PermissionRequest />
      <PromptInput
        value={input}
        onChange={setInput}
        onSubmit={handleSubmit}
        commands={commands}
        tools={tools}
      />
    </Box>
  )
}
```

### 6. API Client (`src/services/api/claude.ts`)

The **Anthropic API client** (126KB):

```typescript
export async function* streamMessages(
  params: MessagesParams
): AsyncGenerator<StreamEvent> {
  const response = await fetch(`${baseUrl}/v1/messages`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-API-Key': apiKey,
      'anthropic-version': '2023-06-01',
    },
    body: JSON.stringify({
      model: params.model,
      max_tokens: params.maxTokens,
      system: params.systemPrompt,
      messages: params.messages,
      tools: params.tools,
      stream: true,
    }),
  })

  // Parse SSE stream
  for await (const chunk of parseSSE(response.body)) {
    yield chunk
  }
}
```

### 7. Codex Adapter (`src/services/api/codex-fetch-adapter.ts`)

The **OpenAI Codex adapter** (28KB):

```typescript
export async function adaptCodexResponse(
  codexStream: AsyncIterable<CodexEvent>
): AsyncGenerator<AnthropicStreamEvent> {
  for await (const event of codexStream) {
    // Translate Codex reasoning deltas to Anthropic thinking events
    if (event.type === 'response.reasoning.delta') {
      yield {
        type: 'content_block_delta',
        delta: { type: 'thinking_delta', thinking: event.delta },
      }
    }

    // Translate Codex function_call_output to Anthropic tool_result
    if (event.type === 'response.function_call_output') {
      yield {
        type: 'content_block_delta',
        delta: {
          type: 'tool_result_delta',
          tool_use_id: event.call_id,
          content: event.output,
        },
      }
    }
  }
}
```

**Key features:**
- Native vision translation (base64 → input_image)
- Strict payload mapping (avoid OpenAI validation errors)
- Cache stripping (remove Anthropic-only annotations)
- Thinking animation support (response.reasoning.delta → thinking events)
- Token tracking (usage.input_tokens, usage.output_tokens)

### 8. Bridge System (`src/bridge/`)

The **remote control / IDE bridge** (30+ files):

```
bridge/
├── bridgeMain.ts           # Main entrypoint
├── bridgeMessaging.ts      # Message protocol
├── bridgeUI.ts             # Bridge status UI
├── replBridge.ts           # REPL transport
├── replBridgeTransport.ts  # Low-level transport
├── inboundMessages.ts      # Inbound message handling
├── inboundAttachments.ts   # Attachment handling
├── jwtUtils.ts             # JWT authentication
├── sessionRunner.ts        # Session management
└── ...
```

**Bridge architecture:**
```
Mobile/Web Client          Local Machine
     │                          │
     │  ─── WebSocket ───►      │
     │                          │
     │  ◄─── SSE Stream ───     │
     │                          │
     │  ─── Tool Results ──►    │
     │                          │
     │  ◄─── Text Output ───    │
```

---

## Execution Flow

### Startup Sequence

```
1. CLI Entrypoint (cli.tsx)
   │
   ├── Check fast-paths (--version, --daemon, --bridge)
   │   └── Return early if matched
   │
   └── Load main module (main.tsx)

2. Main Module (main.tsx)
   │
   ├── Enable configs (config.ts)
   │   └── Load ~/.claude/config.json
   │
   ├── Initialize sinks (sinks.ts)
   │   └── Setup error logging
   │
   └── Render Ink TUI

3. REPL Initialization (REPL.tsx)
   │
   ├── Load session state
   │   └── Restore from ~/.claude/sessions/
   │
   ├── Initialize services
   │   ├── MCP clients
   │   ├── OAuth tokens
   │   └── Background tasks
   │
   └── Render initial UI
```

### Query Turn Flow

```
1. User submits prompt
   │
2. Slash command parsing
   │   ├── If command → expand to prompt text
   │   └── If plain text → use as-is
   │
3. Context assembly
   │   ├── System prompt (tools, model, permissions)
   │   ├── User context (cwd, shell, IDE)
   │   ├── File context (@mentions, detected files)
   │   └── Memory context (team memories, session memories)
   │
4. API request
   │   ├── Select provider (Anthropic/Codex/Bedrock/Vertex/Foundry)
   │   ├── Format request
   │   └── Stream response
   │
5. Response handling
   │   ├── Parse SSE events
   │   ├── Handle thinking deltas
   │   ├── Handle tool calls
   │   └── Accumulate text output
   │
6. Tool execution (if any)
   │   ├── Permission check (auto/manual)
   │   ├── Execute tool
   │   └── Feed result back to API
   │
7. Loop until completion
```

---

## Dependencies

### Runtime Dependencies (90+)

| Package | Purpose |
|---------|---------|
| `ink` | Terminal UI framework (React for CLI) |
| `react` | UI component library |
| `@anthropic-ai/sdk` | Anthropic API client |
| `@anthropic-ai/bedrock-sdk` | AWS Bedrock client |
| `@anthropic-ai/vertex-sdk` | Google Vertex client |
| `@anthropic-ai/foundry-sdk` | Anthropic Foundry client |
| `@modelcontextprotocol/sdk` | MCP protocol |
| `@growthbook/growthbook` | Feature flag runtime |
| `@opentelemetry/*` | Telemetry (stubbed in free-code) |
| `commander` | CLI argument parsing |
| `zod` | Schema validation |
| `chalk` | Terminal colors |
| `execa` | Process execution |
| `chokidar` | File watching |
| `ignore` | .gitignore parsing |
| `ripgrep` | Code search (bundled) |
| `marked` | Markdown rendering |
| `yaml` | YAML parsing |
| `ws` | WebSocket support |
| `undici` | HTTP client |

### Build Dependencies

| Package | Purpose |
|---------|---------|
| `bun` | Runtime and bundler |
| `typescript` | Type checking |
| `@types/bun` | Bun type definitions |

---

## Configuration

### Configuration Files

| File | Location | Purpose |
|------|----------|---------|
| Main config | `~/.claude/config.json` | User settings |
| Global config | `~/.claude/global.json` | Global settings |
| Session files | `~/.claude/sessions/*/` | Session state |
| Memory files | `~/.claude/memory/` | Long-term memory |
| Team memory | `.claude/TEAM.md` | Project team memory |
| Project config | `.claude/settings.json` | Project settings |

### Environment Variables

**Provider Selection:**
```bash
# Anthropic (default)
export ANTHROPIC_API_KEY="sk-ant-..."

# OpenAI Codex
export CLAUDE_CODE_USE_OPENAI=1

# AWS Bedrock
export CLAUDE_CODE_USE_BEDROCK=1
export AWS_REGION="us-east-1"

# Google Vertex AI
export CLAUDE_CODE_USE_VERTEX=1

# Anthropic Foundry
export CLAUDE_CODE_USE_FOUNDRY=1
export ANTHROPIC_FOUNDRY_API_KEY="..."
```

**Model Selection:**
```bash
export ANTHROPIC_MODEL="claude-opus-4-6"
export ANTHROPIC_DEFAULT_OPUS_MODEL="claude-opus-4-6"
export ANTHROPIC_DEFAULT_SONNET_MODEL="claude-sonnet-4-6"
export ANTHROPIC_DEFAULT_HAIKU_MODEL="claude-haiku-4-5"
```

**Feature Flags (compile-time):**
```bash
# Build with all experimental features
bun run build:dev:full

# Build with specific flags
bun run ./scripts/build.ts --feature=ULTRAPLAN --feature=ULTRATHINK
```

---

## Testing Strategy

The codebase includes several testing mechanisms:

### Unit Tests

```typescript
// Example: Tool error handling tests
describe('BashTool', () => {
  it('handles permission denied', async () => {
    // ...
  })

  it('strips ANSI codes from output', async () => {
    // ...
  })
})
```

### Integration Tests

The project uses **test-specific tools**:
- `TestingPermissionTool` — Test permission flows
- `OverflowTestTool` — Test context overflow handling

### Eval Framework

The build system includes **ablation baseline** testing:
```bash
# Build with all features disabled (baseline)
bun run ./scripts/build.ts --feature=ABLATION_BASELINE

# Compare against full feature build
```

---

## References

- [00-zero-to-free-code-engineer.md](./00-zero-to-free-code-engineer.md) — Getting started guide
- [production-grade.md](./production-grade.md) — Production deployment guide
- [FEATURES.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/FEATURES.md) — Feature flag audit (88 flags)
- [README.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/README.md) — Project documentation
- [changes.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/changes.md) — Recent changes (Codex support)
