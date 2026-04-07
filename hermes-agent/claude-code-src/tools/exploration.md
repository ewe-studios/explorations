---
name: Claude Code Tools Module Deep-Dive
description: Comprehensive exploration of the tools/ module architecture, tool definitions, and execution patterns
type: reference
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/tools/
explored_at: 2026-04-07
---

# Tools Module Deep-Dive Exploration

## Overview

The `tools/` module is the core execution engine for Claude Code, containing **184 TypeScript/TSX files** organized into ~45 tool directories. Tools are the primitive actions Claude can perform: reading files, executing commands, spawning agents, and interacting with external services.

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/tools/`

---

## 1. File Inventory

### Core Infrastructure Files (src/)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `src/Tool.ts` | 793 | `Tool`, `Tools`, `ToolDef`, `buildTool`, `ToolUseContext`, `ToolResult` | Base tool interface, type definitions, and `buildTool()` factory |
| `src/tools.ts` | 390 | `getAllBaseTools()`, `getTools()`, `assembleToolPool()`, `filterToolsByDenyRules()` | Tool pool assembly, filtering, and MCP merging |
| `src/constants/tools.ts` | 113 | `ALL_AGENT_DISALLOWED_TOOLS`, `ASYNC_AGENT_ALLOWED_TOOLS`, `COORDINATOR_MODE_ALLOWED_TOOLS` | Tool allow/deny lists for agent contexts |

### Tool Directories (src/tools/)

| Directory | Files | Primary Tool | Line Range |
|-----------|-------|--------------|------------|
| `AgentTool/` | 18 | `AgentTool` | 973-2592 |
| `BashTool/` | 17 | `BashTool` | 1143-2621 |
| `FileReadTool/` | 5 | `FileReadTool` | 1183 |
| `FileEditTool/` | 6 | `FileEditTool` | 625-775 |
| `FileWriteTool/` | 2 | `FileWriteTool` | 434 |
| `GrepTool/` | 3 | `GrepTool` | 577 |
| `GlobTool/` | 3 | `GlobTool` | 198 |
| `MCPTool/` | 4 | `MCPTool` | 77-604 |
| `PowerShellTool/` | 12 | `PowerShellTool` | 1000-2049 |
| `SkillTool/` | 4 | `SkillTool` | 1108 |
| `WebFetchTool/` | 4 | `WebFetchTool` | 318-530 |
| `WebSearchTool/` | 3 | `WebSearchTool` | 435 |
| `LSPTool/` | 5 | `LSPTool` | 860-592 |
| `Task*Tool/` | 6 | Task management tools | 131-583 |
| `TodoWriteTool/` | 2 | `TodoWriteTool` | 115-184 |
| `AskUserQuestionTool/` | 2 | `AskUserQuestionTool` | 265 |
| `BriefTool/` | 4 | `BriefTool` | 204 |
| `ConfigTool/` | 4 | `ConfigTool` | 467 |
| `Enter/ExitPlanModeTool/` | 3 each | Plan mode tools | 493-126 |
| `Enter/ExitWorktreeTool/` | 3 each | Worktree tools | 329-127 |
| `NotebookEditTool/` | 3 | `NotebookEditTool` | 490 |
| `ScheduleCronTool/` | 4 | Cron tools | 97-157 |
| `SendMessageTool/` | 3 | `SendMessageTool` | 917 |
| `TeamCreate/DeleteTool/` | 3 each | Agent swarms | 240-139 |
| `ToolSearchTool/` | 3 | `ToolSearchTool` | 471 |
| `SyntheticOutputTool/` | 1 | `SyntheticOutputTool` | 163 |
| `ListMcpResourcesTool/` | 3 | `ListMcpResourcesTool` | 123 |
| `ReadMcpResourceTool/` | 3 | `ReadMcpResourceTool` | 158 |
| `McpAuthTool/` | 1 | `McpAuthTool` | 215 |
| `REPLTool/` | 3 | `REPLTool` | 39-46 |
| `SleepTool/` | 2 | `SleepTool` | 28-32 |
| `RemoteTriggerTool/` | 3 | `RemoteTriggerTool` | 161-16 |
| `testing/` | 1 | `TestingPermissionTool` | 73 |
| `shared/` | 2 | `spawnMultiAgent`, `gitOperationTracking` | 1093-277 |

---

## 2. Module Overview

### Tool Architecture

The tools module follows a consistent architecture where every tool implements the `Tool` interface defined in `src/Tool.ts`:

```typescript
export type Tool<Input extends AnyObject = AnyObject, Output = unknown, P extends ToolProgressData = ToolProgressData> = {
  name: string
  aliases?: string[]
  searchHint?: string
  call(args: z.infer<Input>, context: ToolUseContext, canUseTool: CanUseToolFn, parentMessage: AssistantMessage, onProgress?: ToolCallProgress<P>): Promise<ToolResult<Output>>
  description(input: z.infer<Input>, options: {...}): Promise<string>
  readonly inputSchema: Input
  readonly inputJSONSchema?: ToolInputJSONSchema
  outputSchema?: z.ZodType<unknown>
  inputsEquivalent?(a: z.infer<Input>, b: z.infer<Input>): boolean
  isConcurrencySafe(input: z.infer<Input>): boolean
  isEnabled(): boolean
  isReadOnly(input: z.infer<Input>): boolean
  isDestructive?(input: z.infer<Input>): boolean
  interruptBehavior?(): 'cancel' | 'block'
  isSearchOrReadCommand?(input: z.infer<Input>): { isSearch: boolean; isRead: boolean; isList?: boolean }
  isOpenWorld?(input: z.infer<Input>): boolean
  requiresUserInteraction?(): boolean
  isMcp?: boolean
  isLsp?: boolean
  readonly shouldDefer?: boolean
  readonly alwaysLoad?: boolean
  mcpInfo?: { serverName: string; toolName: string }
  maxResultSizeChars: number
  readonly strict?: boolean
  backfillObservableInput?(input: Record<string, unknown>): void
  validateInput?(input: z.infer<Input>, context: ToolUseContext): Promise<ValidationResult>
  checkPermissions(input: z.infer<Input>, context: ToolUseContext): Promise<PermissionResult>
  getPath?(input: z.infer<Input>): string
  preparePermissionMatcher?(input: z.infer<Input>): Promise<(pattern: string) => boolean>
  prompt(options: {...}): Promise<string>
  userFacingName(input: Partial<z.infer<Input>> | undefined): string
  userFacingNameBackgroundColor?(...): keyof Theme | undefined
  isTransparentWrapper?(): boolean
  getToolUseSummary?(input: Partial<z.infer<Input>> | undefined): string | null
  getActivityDescription?(input: Partial<z.infer<Input>> | undefined): string | null
  toAutoClassifierInput(input: z.infer<Input>): unknown
  mapToolResultToToolResultBlockParam(content: Output, toolUseID: string): ToolResultBlockParam
  renderToolResultMessage?(...): React.ReactNode
  extractSearchText?(out: Output): string
  renderToolUseMessage(input: Partial<z.infer<Input>>, options: {...}): React.ReactNode
  isResultTruncated?(output: Output): boolean
  renderToolUseTag?(input: Partial<z.infer<Input>>): React.ReactNode
  renderToolUseProgressMessage?(...): React.ReactNode
  renderToolUseQueuedMessage?(): React.ReactNode
  renderToolUseRejectedMessage?(...): React.ReactNode
  renderToolUseErrorMessage?(...): React.ReactNode
  renderGroupedToolUse?(...): React.ReactNode | null
}
```

### Tool Base Class and buildTool Factory

The `buildTool()` function (lines 783-792 in `src/Tool.ts`) provides default implementations for commonly-stubbed methods:

```typescript
const TOOL_DEFAULTS = {
  isEnabled: () => true,
  isConcurrencySafe: (_input?: unknown) => false,
  isReadOnly: (_input?: unknown) => false,
  isDestructive: (_input?: unknown) => false,
  checkPermissions: (input: { [key: string]: unknown }, _ctx?: ToolUseContext): Promise<PermissionResult> =>
    Promise.resolve({ behavior: 'allow', updatedInput: input }),
  toAutoClassifierInput: (_input?: unknown) => '',
  userFacingName: (_input?: unknown) => '',
}

export function buildTool<D extends AnyToolDef>(def: D): BuiltTool<D> {
  return {
    ...TOOL_DEFAULTS,
    userFacingName: () => def.name,
    ...def,
  } as BuiltTool<D>
}
```

### Tool Registry

Tools are registered in `src/tools.ts` through the `getAllBaseTools()` function (lines 193-251), which conditionally includes tools based on feature flags and environment variables:

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
    // ... conditional tools based on feature flags
    ...(process.env.USER_TYPE === 'ant' ? [ConfigTool] : []),
    ...(isWorktreeModeEnabled() ? [EnterWorktreeTool, ExitWorktreeTool] : []),
    // ... more tools
  ]
}
```

---

## 3. Tool Categories

### 3.1 Core Tools

#### BashTool (`BashTool/BashTool.tsx` - 1143 lines)

The primary command execution tool with extensive security features:

**Key Features:**
- Sandbox execution via `SandboxManager`
- Auto-backgrounding for long-running commands
- Permission checking with `bashPermissions.ts` (2621 lines) and `bashSecurity.ts` (2592 lines)
- Sed edit preview and validation
- Output persistence for large results
- Background task management

**Input Schema:**
```typescript
const inputSchema = lazySchema(() => isBackgroundTasksDisabled ? fullInputSchema().omit({
  run_in_background: true,
  _simulatedSedEdit: true
}) : fullInputSchema().omit({
  _simulatedSedEdit: true
}))

// Full schema includes:
{
  command: z.string(),
  timeout: z.number().optional(),
  description: z.string().optional(),
  run_in_background: z.boolean().optional(),
  dangerouslyDisableSandbox: z.boolean().optional(),
  _simulatedSedEdit: z.object({ filePath: z.string(), newContent: z.string() }).optional()
}
```

**Security Layers:**
1. `bashSecurity.ts` - AST-based security analysis
2. `bashPermissions.ts` - Permission rule matching
3. `readOnlyValidation.ts` - Read-only mode constraints
4. `pathValidation.ts` - Path traversal prevention
5. `sedValidation.ts` - Sed command validation
6. `destructiveCommandWarning.ts` - Destructive command detection

#### FileReadTool (`FileReadTool/FileReadTool.ts` - 1183 lines)

Multi-format file reading with support for:
- Text files with offset/limit pagination
- Images (PNG, JPG, JPEG, GIF, WEBP) with token-based compression
- PDFs with page extraction
- Jupyter notebooks (`.ipynb`)

**Key Features:**
- Token budget enforcement (`maxTokens` default ~4000)
- Size limits (`maxSizeBytes` default ~100MB)
- Deduplication via `readFileState` cache
- Auto-memory file freshness tracking
- Session file read analytics

**Output Types:**
```typescript
type Output =
  | { type: 'text'; file: { filePath, content, numLines, startLine, totalLines } }
  | { type: 'image'; file: { base64, type, originalSize, dimensions? } }
  | { type: 'notebook'; file: { filePath, cells } }
  | { type: 'pdf'; file: { filePath, base64, originalSize } }
  | { type: 'parts'; file: { filePath, originalSize, count, outputDir } }
  | { type: 'file_unchanged'; file: { filePath } }
```

#### FileEditTool (`FileEditTool/FileEditTool.ts` - 625 lines)

In-place file editing with string replacement:

**Key Features:**
- Quote style preservation (`preserveQuoteStyle()`)
- Multiple occurrence detection and `replace_all` flag
- File staleness detection (modified since read)
- LSP server notification on edit
- VSCode diff view integration
- Git diff generation for remote sessions
- File history tracking

**Validation:**
```typescript
async validateInput(input: FileEditInput, toolUseContext: ToolUseContext) {
  // Checks:
  // 1. old_string !== new_string
  // 2. Path not denied by permissions
  // 3. File size < 1 GiB
  // 4. File exists (or empty old_string for creation)
  // 5. Not a notebook (use NotebookEditTool)
  // 6. File was read before edit
  // 7. File not modified since read
  // 8. old_string found in file
  // 9. Unique match or replace_all=true
  // 10. Settings file validation
}
```

#### FileWriteTool (`FileWriteTool/FileWriteTool.ts` - 434 lines)

New file creation with content:

**Key Features:**
- Directory auto-creation
- Binary file detection
- Team memory secret scanning
- Skill directory discovery

#### GrepTool (`GrepTool/GrepTool.ts` - 577 lines)

Ripgrep-powered content search:

**Input Schema:**
```typescript
{
  pattern: z.string(),  // Regex pattern
  path: z.string().optional(),
  glob: z.string().optional(),
  output_mode: z.enum(['content', 'files_with_matches', 'count']).optional(),
  '-B': z.number().optional(),  // Lines before
  '-A': z.number().optional(),  // Lines after
  '-C': z.number().optional(),  // Lines context
  '-n': z.boolean().optional(), // Line numbers
  '-i': z.boolean().optional(), // Case insensitive
  type: z.string().optional(),  // File type filter
  head_limit: z.number().optional(),  // Result limit (default 250)
  offset: z.number().optional(),  // Pagination offset
  multiline: z.boolean().optional()
}
```

**Features:**
- VCS directory exclusion (.git, .svn, etc.)
- `.gitignore` pattern respect
- Plugin orphan directory exclusion
- Sorted by modification time
- Path relativization for token savings

#### GlobTool (`GlobTool/GlobTool.ts` - 198 lines)

File pattern matching:

**Input Schema:**
```typescript
{
  pattern: z.string(),
  path: z.string().optional()
}
```

**Output:**
```typescript
{
  durationMs: number,
  numFiles: number,
  filenames: string[],
  truncated: boolean
}
```

### 3.2 AgentTool and Agent Definitions

#### AgentTool (`AgentTool/AgentTool.tsx` - 1397 lines)

The agent spawning and management tool:

**Input Schema:**
```typescript
{
  description: z.string(),  // 3-5 word task description
  prompt: z.string(),       // Full task prompt
  subagent_type: z.string().optional(),
  model: z.enum(['sonnet', 'opus', 'haiku']).optional(),
  run_in_background: z.boolean().optional(),
  name: z.string().optional(),           // For teammate addressing
  team_name: z.string().optional(),      // Team spawning
  mode: z.enum(['default', 'plan', 'auto', 'danger']).optional(),
  isolation: z.enum(['worktree', 'remote']).optional(),
  cwd: z.string().optional()
}
```

**Execution Paths:**
1. **Teammate Spawn** - When `team_name` and `name` provided → `spawnTeammate()`
2. **Fork Path** - When `subagent_type` undefined and fork enabled → `FORK_AGENT`
3. **Standard Subagent** - Resolves `subagent_type` → `runAgent()`
4. **Background Task** - When `run_in_background=true` → LocalShellTask registration
5. **Remote Launch** - When `isolation='remote'` → CCR remote execution

**Key Files in AgentTool/:**
- `runAgent.ts` (973 lines) - Agent execution lifecycle
- `loadAgentsDir.ts` (755 lines) - Agent definition loading
- `agentToolUtils.ts` (686 lines) - Shared utilities
- `prompt.ts` (287 lines) - Agent tool prompt generation
- `UI.tsx` (871 lines) - Render functions
- `forkSubagent.ts` (210 lines) - Fork experiment logic
- `built-in/` - Built-in agent definitions (explore, plan, verify, etc.)

#### Agent Definition Loading (`loadAgentsDir.ts`)

Loads agents from multiple sources:
1. **Built-in** - Hardcoded agents (Explore, Plan, Verification)
2. **User Settings** - `~/.claude/settings.json`
3. **Project Settings** - `.claude/settings.json`
4. **Policy Settings** - Admin policy files
5. **Plugin Agents** - Plugin-provided agents
6. **Markdown Files** - `.md` agent definitions with frontmatter

**Agent Definition Type:**
```typescript
type AgentDefinition = {
  agentType: string
  whenToUse: string
  tools?: string[]
  disallowedTools?: string[]
  skills?: string[]
  mcpServers?: AgentMcpServerSpec[]
  hooks?: HooksSettings
  color?: AgentColorName
  model?: string
  effort?: EffortValue
  permissionMode?: PermissionMode
  maxTurns?: number
  filename?: string
  criticalSystemReminder_EXPERIMENTAL?: string
  requiredMcpServers?: string[]
  background?: boolean
  initialPrompt?: string
  memory?: 'user' | 'project' | 'local'
  isolation?: 'worktree' | 'remote'
  omitClaudeMd?: boolean
  source: 'built-in' | 'userSettings' | 'projectSettings' | 'policySettings' | 'plugin'
  getSystemPrompt: (...) => string  // Closure returning prompt
}
```

### 3.3 MCP Tools (Model Context Protocol)

#### MCPTool (`MCPTool/MCPTool.ts` - 77 lines)

Generic wrapper for MCP server-provided tools:

**Key Characteristics:**
- `isMcp: true` flag for identification
- Dynamic schema from MCP server
- Permission passthrough behavior
- Progress support via `MCPProgress`

**Tool Registration:**
MCP tools are dynamically registered in `services/mcp/client.ts` and merged with built-in tools via `assembleToolPool()`.

#### ListMcpResourcesTool (`ListMcpResourcesTool/ListMcpResourcesTool.ts` - 123 lines)

Lists available MCP resources from connected servers.

#### ReadMcpResourceTool (`ReadMcpResourceTool/ReadMcpResourceTool.ts` - 158 lines)

Reads a specific MCP resource by URI.

#### McpAuthTool (`McpAuthTool/McpAuthTool.ts` - 215 lines)

Handles MCP server authentication flows.

### 3.4 Synthetic Tools

#### SyntheticOutputTool (`SyntheticOutputTool/SyntheticOutputTool.ts` - 163 lines)

Internal-only tool for synthetic output generation:
- Used for testing and internal orchestration
- Not exposed to user-facing prompts
- Bypasses normal tool rendering

### 3.5 Permission Tools

#### TestingPermissionTool (`testing/TestingPermissionTool.tsx` - 73 lines)

Test-only tool for permission system testing (NODE_ENV=test only).

---

## 4. Line-by-Line Analysis of Critical Files

### 4.1 Tool.ts - Tool Interface and Base Class

**Type Definitions (lines 1-74):**
- `ToolInputJSONSchema` - JSON Schema format for MCP tools
- `ToolUseContext` - Execution context with app state, file cache, abort controller
- `ToolResult<T>` - Result wrapper with optional new messages and context modifier
- `ToolCallProgress<P>` - Progress callback type

**Tool Interface (lines 362-695):**

The `Tool` type defines 50+ properties organized into categories:

**Identity (lines 371-378):**
```typescript
aliases?: string[]           // Backwards compatibility names
searchHint?: string          // For ToolSearch keyword matching (3-10 words)
name: string                 // Primary tool name
```

**Execution (lines 379-385):**
```typescript
call(
  args: z.infer<Input>,
  context: ToolUseContext,
  canUseTool: CanUseToolFn,
  parentMessage: AssistantMessage,
  onProgress?: ToolCallProgress<P>
): Promise<ToolResult<Output>>
```

**Description (lines 386-393):**
```typescript
description(
  input: z.infer<Input>,
  options: {
    isNonInteractiveSession: boolean
    toolPermissionContext: ToolPermissionContext
    tools: Tools
  }
): Promise<string>
```

**Schema (lines 394-400):**
```typescript
readonly inputSchema: Input
readonly inputJSONSchema?: ToolInputJSONSchema  // Direct JSON Schema (MCP)
outputSchema?: z.ZodType<unknown>
```

**Behavior Flags (lines 401-435):**
```typescript
inputsEquivalent?(a, b): boolean
isConcurrencySafe(input): boolean
isEnabled(): boolean
isReadOnly(input): boolean
isDestructive?(input): boolean
interruptBehavior?(): 'cancel' | 'block'
isSearchOrReadCommand?(input): { isSearch, isRead, isList }
isOpenWorld?(input): boolean
requiresUserInteraction?(): boolean
```

**Loading Control (lines 436-449):**
```typescript
isMcp?: boolean
isLsp?: boolean
readonly shouldDefer?: boolean      // Deferred until ToolSearch
readonly alwaysLoad?: boolean       // Always in initial prompt
```

**Validation (lines 489-516):**
```typescript
validateInput?(input, context): Promise<ValidationResult>
checkPermissions(input, context): Promise<PermissionResult>
preparePermissionMatcher?(input): Promise<(pattern: string) => boolean>
```

**UI Rendering (lines 518-694):**
```typescript
prompt(options): Promise<string>
userFacingName(input): string
userFacingNameBackgroundColor?(input): keyof Theme | undefined
renderToolUseMessage(input, options): React.ReactNode
renderToolResultMessage(content, progressMessages, options): React.ReactNode
renderToolUseProgressMessage?(progressMessages, options): React.ReactNode
renderToolUseRejectedMessage?(input, options): React.ReactNode
renderToolUseErrorMessage?(result, options): React.ReactNode
renderGroupedToolUse?(toolUses, options): React.ReactNode | null
```

**buildTool Factory (lines 757-792):**

The `TOOL_DEFAULTS` object (lines 757-769) provides fail-closed defaults:
```typescript
const TOOL_DEFAULTS = {
  isEnabled: () => true,
  isConcurrencySafe: (_input?: unknown) => false,  // Assume NOT safe
  isReadOnly: (_input?: unknown) => false,         // Assume writes
  isDestructive: (_input?: unknown) => false,
  checkPermissions: (input, _ctx) => Promise.resolve({ behavior: 'allow', updatedInput: input }),
  toAutoClassifierInput: (_input?: unknown) => '',
  userFacingName: (_input?: unknown) => '',
}
```

### 4.2 tools.ts - Tool Pool Assembly

**getAllBaseTools() (lines 193-251):**

Returns the exhaustive list of tools based on feature flags:
```typescript
export function getAllBaseTools(): Tools {
  return [
    AgentTool,
    TaskOutputTool,
    BashTool,
    ...(hasEmbeddedSearchTools() ? [] : [GlobTool, GrepTool]),  // Conditional
    ExitPlanModeV2Tool,
    FileReadTool,
    FileEditTool,
    FileWriteTool,
    NotebookEditTool,
    WebFetchTool,
    TodoWriteTool,
    WebSearchTool,
    TaskStopTool,
    AskUserQuestionTool,
    SkillTool,
    EnterPlanModeTool,
    ...(process.env.USER_TYPE === 'ant' ? [ConfigTool] : []),
    ...(process.env.USER_TYPE === 'ant' ? [TungstenTool] : []),
    // ... feature-gated tools
  ]
}
```

**getTools() (lines 271-327):**

Filters tools by permission context and mode:
```typescript
export const getTools = (permissionContext: ToolPermissionContext): Tools => {
  // Simple mode: only Bash, Read, Edit
  if (isEnvTruthy(process.env.CLAUDE_CODE_SIMPLE)) {
    const simpleTools: Tool[] = [BashTool, FileReadTool, FileEditTool]
    return filterToolsByDenyRules(simpleTools, permissionContext)
  }

  // Filter out special tools
  const specialTools = new Set([
    ListMcpResourcesTool.name,
    ReadMcpResourceTool.name,
    SYNTHETIC_OUTPUT_TOOL_NAME,
  ])
  const tools = getAllBaseTools().filter(tool => !specialTools.has(tool.name))

  // Apply deny rules
  let allowedTools = filterToolsByDenyRules(tools, permissionContext)

  // REPL mode: hide primitive tools
  if (isReplModeEnabled()) {
    allowedTools = allowedTools.filter(tool => !REPL_ONLY_TOOLS.has(tool.name))
  }

  // Filter by isEnabled()
  const isEnabled = allowedTools.map(_ => _.isEnabled())
  return allowedTools.filter((_, i) => isEnabled[i])
}
```

**assembleToolPool() (lines 345-367):**

Merges built-in tools with MCP tools:
```typescript
export function assembleToolPool(
  permissionContext: ToolPermissionContext,
  mcpTools: Tools,
): Tools {
  const builtInTools = getTools(permissionContext)
  const allowedMcpTools = filterToolsByDenyRules(mcpTools, permissionContext)

  // Sort each partition for prompt-cache stability
  const byName = (a: Tool, b: Tool) => a.name.localeCompare(b.name)
  return uniqBy(
    [...builtInTools].sort(byName).concat(allowedMcpTools.sort(byName)),
    'name',
  )
}
```

**filterToolsByDenyRules() (lines 262-269):**

Removes tools matching deny rules:
```typescript
export function filterToolsByDenyRules<T>(
  tools: readonly T[],
  permissionContext: ToolPermissionContext
): T[] {
  return tools.filter(tool => !getDenyRuleForTool(permissionContext, tool))
}
```

### 4.3 AgentTool/runAgent.ts - Agent Execution

**Agent MCP Server Initialization (lines 95-218):**

```typescript
async function initializeAgentMcpServers(
  agentDefinition: AgentDefinition,
  parentClients: MCPServerConnection[],
): Promise<{
  clients: MCPServerConnection[]
  tools: Tools
  cleanup: () => Promise<void>
}> {
  if (!agentDefinition.mcpServers?.length) {
    return { clients: parentClients, tools: [], cleanup: async () => {} }
  }

  const agentClients: MCPServerConnection[] = []
  const newlyCreatedClients: MCPServerConnection[] = []
  const agentTools: Tool[] = []

  for (const spec of agentDefinition.mcpServers) {
    let config: ScopedMcpServerConfig | null = null
    let name: string
    let isNewlyCreated = false

    if (typeof spec === 'string') {
      // Reference by name - shared from parent
      name = spec
      config = getMcpConfigByName(spec)
    } else {
      // Inline definition - agent-specific
      const entries = Object.entries(spec)
      name = entries[0][0]
      config = { ...entries[0][1], scope: 'dynamic' }
      isNewlyCreated = true
    }

    const client = await connectToServer(name, config)
    agentClients.push(client)
    if (isNewlyCreated) newlyCreatedClients.push(client)

    if (client.type === 'connected') {
      const tools = await fetchToolsForClient(client)
      agentTools.push(...tools)
    }
  }

  // Cleanup only newly created clients
  const cleanup = async () => {
    for (const client of newlyCreatedClients) {
      if (client.type === 'connected') {
        await client.cleanup().catch(log)
      }
    }
  }

  return {
    clients: [...parentClients, ...agentClients],
    tools: agentTools,
    cleanup
  }
}
```

**Agent Execution Generator (lines 248+):**

```typescript
export async function* runAgent({
  agentDefinition,
  promptMessages,
  toolUseContext,
  canUseTool,
  isAsync,
  canShowPermissionPrompts,
  forkContextMessages,
  querySource,
  override,
  model,
  maxTurns,
  preserveToolUseResults,
  availableTools,
  allowedTools,
  onCacheSafeParams,
  contentReplacementState,
  useExactTools,
  worktreePath,
  description,
  transcriptSubdir,
  onQueryProgress,
}: {...}) {
  // 1. Initialize agent-specific MCP servers
  const { clients: mergedClients, tools: agentMcpTools, cleanup } =
    await initializeAgentMcpServers(agentDefinition, toolUseContext.options.mcpClients)

  // 2. Resolve agent tools (allowed/disallowed filtering)
  const agentTools = resolveAgentTools(
    availableTools,
    agentDefinition.tools,
    agentDefinition.disallowedTools,
    agentMcpTools,
  )

  // 3. Create subagent context (fork or standard)
  const subagentContext = createSubagentContext({
    parentContext: toolUseContext,
    agentDefinition,
    querySource,
    forkContextMessages,
    ...
  })

  // 4. Register hooks from frontmatter
  if (agentDefinition.hooks) {
    await registerFrontmatterHooks(agentDefinition.hooks, subagentContext)
  }

  // 5. Execute subagent start hooks
  await executeSubagentStartHooks({
    agentDefinition,
    prompt: promptMessages[0]?.content,
    context: subagentContext,
  })

  // 6. Run query loop with agent tools
  const queryIterator = query({
    messages: promptMessages,
    tools: agentTools,
    systemPrompt: agentDefinition.getSystemPrompt(...),
    ...
  })

  // 7. Yield messages and record transcript
  for await (const message of queryIterator) {
    if (isRecordableMessage(message)) {
      await recordSidechainTranscript(...)
    }
    yield message
  }

  // 8. Cleanup
  await cleanup()
}
```

### 4.4 BashTool/bashPermissions.ts - Permission Checking

**Permission Rule Matching (lines 200+):**

```typescript
export function bashToolHasPermission(
  command: string,
  permissionContext: ToolPermissionContext,
  behavior: 'allow' | 'ask' | 'deny'
): { result: boolean; rules?: PermissionRule[] } {
  const ast = parseForSecurityFromAst(command)
  if (!ast) return { result: false }

  // Extract command prefixes and subcommands
  const prefixes = extractCommandPrefixes(ast)

  // Check each prefix against permission rules
  for (const prefix of prefixes) {
    const rules = getRulesForPrefix(permissionContext, behavior, prefix)
    if (rules.length > 0) {
      return { result: true, rules }
    }
  }

  return { result: false }
}
```

**Command Prefix Extraction:**

```typescript
export function getSimpleCommandPrefix(command: string): string | null {
  const tokens = command.trim().split(/\s+/).filter(Boolean)
  if (tokens.length === 0) return null

  // Skip safe env var assignments
  let i = 0
  while (i < tokens.length && ENV_VAR_ASSIGN_RE.test(tokens[i]!)) {
    const varName = tokens[i]!.split('=')[0]!
    if (!SAFE_ENV_VARS.has(varName)) return null
    i++
  }

  const remaining = tokens.slice(i)
  if (remaining.length < 2) return null
  const subcmd = remaining[1]!
  if (!/^[a-z][a-z0-9]*(-[a-z0-9]+)*$/.test(subcmd)) return null
  return remaining.slice(0, 2).join(' ')
}
```

### 4.5 FileReadTool/FileReadTool.ts - Multi-Format Reading

**Call Inner Implementation (lines 804+):**

```typescript
async function callInner(
  file_path: string,
  fullFilePath: string,
  resolvedFilePath: string,
  ext: string,
  offset: number,
  limit: number | undefined,
  pages: string | undefined,
  maxSizeBytes: number,
  maxTokens: number,
  readFileState: ToolUseContext['readFileState'],
  context: ToolUseContext,
  messageId: string | undefined,
): Promise<{ data: Output; newMessages?: ... }> {
  // --- Notebook ---
  if (ext === 'ipynb') {
    const cells = await readNotebook(resolvedFilePath)
    const cellsJson = jsonStringify(cells)
    if (Buffer.byteLength(cellsJson) > maxSizeBytes) {
      throw new Error(`Notebook too large, use jq to read portions`)
    }
    await validateContentTokens(cellsJson, ext, maxTokens)
    const stats = await getFsImplementation().stat(resolvedFilePath)
    readFileState.set(fullFilePath, { content: cellsJson, timestamp: Math.floor(stats.mtimeMs), offset, limit })
    return { data: { type: 'notebook', file: { filePath, cells } } }
  }

  // --- Image ---
  if (IMAGE_EXTENSIONS.has(ext)) {
    const data = await readImageWithTokenBudget(resolvedFilePath, maxTokens)
    return { data, newMessages: [createUserMessage({ content: metadataText, isMeta: true })] }
  }

  // --- PDF ---
  if (isPDFExtension(ext)) {
    if (pages) {
      const extractResult = await extractPDFPages(resolvedFilePath, parsePDFPageRange(pages))
      // Extract page images and return as image blocks
    }
    const pageCount = await getPDFPageCount(resolvedFilePath)
    if (pageCount > PDF_AT_MENTION_INLINE_THRESHOLD) {
      throw new Error(`PDF too large, use pages parameter`)
    }
    const readResult = await readPDF(resolvedFilePath)
    return { data: readResult.data, newMessages: [createUserMessage({ content: [{ type: 'document', ... }] })] }
  }

  // --- Text file ---
  const { content, lineCount, totalLines, totalBytes, readBytes, mtimeMs } =
    await readFileInRange(resolvedFilePath, offset - 1, limit, maxSizeBytes, context.abortController.signal)

  await validateContentTokens(content, ext, maxTokens)
  readFileState.set(fullFilePath, { content, timestamp: Math.floor(mtimeMs), offset, limit })

  // Notify listeners
  for (const listener of fileReadListeners.slice()) {
    listener(resolvedFilePath, content)
  }

  const data = { type: 'text', file: { filePath, content, numLines: lineCount, startLine: offset, totalLines } }
  if (isAutoMemFile(fullFilePath)) {
    memoryFileMtimes.set(data, mtimeMs)
  }

  return { data }
}
```

### 4.6 GrepTool/GrepTool.ts - Content Search

**Call Implementation (lines 310-577):**

```typescript
async call({
  pattern, path, glob, type, output_mode = 'files_with_matches',
  '-B': context_before, '-A': context_after, '-C': context_c, context,
  '-n': show_line_numbers = true, '-i': case_insensitive = false,
  head_limit, offset = 0, multiline = false
}, { abortController, getAppState }) {
  const absolutePath = path ? expandPath(path) : getCwd()
  const args = ['--hidden']

  // Exclude VCS directories
  for (const dir of VCS_DIRECTORIES_TO_EXCLUDE) {
    args.push('--glob', `!${dir}`)
  }

  // Line length limit
  args.push('--max-columns', '500')

  // Multiline mode
  if (multiline) args.push('-U', '--multiline-dotall')
  if (case_insensitive) args.push('-i')

  // Output mode
  if (output_mode === 'files_with_matches') args.push('-l')
  else if (output_mode === 'count') args.push('-c')
  else if (show_line_numbers && output_mode === 'content') args.push('-n')

  // Context lines
  if (output_mode === 'content') {
    if (context !== undefined) args.push('-C', context.toString())
    else if (context_c !== undefined) args.push('-C', context_c.toString())
    else {
      if (context_before !== undefined) args.push('-B', context_before.toString())
      if (context_after !== undefined) args.push('-A', context_after.toString())
    }
  }

  // Pattern handling
  if (pattern.startsWith('-')) args.push('-e', pattern)
  else args.push(pattern)

  // Type filter
  if (type) args.push('--type', type)

  // Glob patterns
  if (glob) {
    const globPatterns = glob.split(/\s+/).flatMap(p => p.includes('{') ? [p] : p.split(','))
    for (const p of globPatterns.filter(Boolean)) args.push('--glob', p)
  }

  // Ignore patterns from permissions
  const ignorePatterns = normalizePatternsToPath(getFileReadIgnorePatterns(...), getCwd())
  for (const pattern of ignorePatterns) {
    args.push('--glob', pattern.startsWith('/') ? `!${pattern}` : `!**/${pattern}`)
  }

  // Execute ripgrep
  const results = await ripGrep(args, absolutePath, abortController.signal)

  // Process results by mode
  if (output_mode === 'content') {
    const { items: limitedResults, appliedLimit } = applyHeadLimit(results, head_limit, offset)
    const finalLines = limitedResults.map(line => {
      const colonIndex = line.indexOf(':')
      if (colonIndex > 0) {
        const filePath = line.substring(0, colonIndex)
        return toRelativePath(filePath) + line.substring(colonIndex)
      }
      return line
    })
    return { data: { mode: 'content', numFiles: 0, filenames: [], content: finalLines.join('\n'), numLines: finalLines.length, appliedLimit, appliedOffset: offset } }
  }

  if (output_mode === 'count') {
    const { items: limitedResults, appliedLimit } = applyHeadLimit(results, head_limit, offset)
    const finalCountLines = limitedResults.map(line => {
      const colonIndex = line.lastIndexOf(':')
      if (colonIndex > 0) {
        const filePath = line.substring(0, colonIndex)
        const count = line.substring(colonIndex + 1)
        return toRelativePath(filePath) + ':' + count
      }
      return line
    })
    // Parse total matches
    let totalMatches = 0, fileCount = 0
    for (const line of finalCountLines) {
      const count = parseInt(line.split(':').pop()!)
      if (!isNaN(count)) { totalMatches += count; fileCount += 1 }
    }
    return { data: { mode: 'count', numFiles: fileCount, filenames: [], content: finalCountLines.join('\n'), numMatches: totalMatches, appliedLimit, appliedOffset: offset } }
  }

  // files_with_matches mode
  const stats = await Promise.allSettled(results.map(f => getFsImplementation().stat(f)))
  const sortedMatches = results
    .map((f, i) => [f, stats[i].status === 'fulfilled' ? stats[i].value.mtimeMs : 0] as const)
    .sort((a, b) => process.env.NODE_ENV === 'test' ? a[0].localeCompare(b[0]) : b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(_ => _[0])

  const { items: finalMatches, appliedLimit } = applyHeadLimit(sortedMatches, head_limit, offset)
  const relativeMatches = finalMatches.map(toRelativePath)

  return { data: { mode: 'files_with_matches', filenames: relativeMatches, numFiles: relativeMatches.length, appliedLimit, appliedOffset: offset } }
}
```

---

## 5. Tool Pool Assembly

### Tool Assembly Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    Tool Pool Assembly                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  getAllBaseTools()                                              │
│  - Returns exhaustive list based on feature flags              │
│  - Conditional: hasEmbeddedSearchTools, USER_TYPE, etc.        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  getTools(permissionContext)                                    │
│  - Filters by CLAUDE_CODE_SIMPLE (simple mode)                 │
│  - Applies filterToolsByDenyRules()                            │
│  - REPL mode filtering (hides primitive tools)                 │
│  - Filters by isEnabled()                                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  assembleToolPool(permissionContext, mcpTools)                  │
│  - Gets built-in tools via getTools()                          │
│  - Filters MCP tools by deny rules                             │
│  - Sorts each partition by name (prompt-cache stability)       │
│  - Merges with uniqBy (built-in wins conflicts)                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Final Tool Pool (Tools)                                        │
│  - Built-in tools (sorted)                                     │
│  - MCP tools (sorted, deduplicated)                            │
└─────────────────────────────────────────────────────────────────┘
```

### Feature Flag Gating

Tools are conditionally included based on:

| Tool | Feature Flag / Condition |
|------|-------------------------|
| `GlobTool`, `GrepTool` | `!hasEmbeddedSearchTools()` |
| `ConfigTool`, `TungstenTool` | `USER_TYPE === 'ant'` |
| `REPLTool` | `USER_TYPE === 'ant' && REPL enabled` |
| `Task*Tool` | `isTodoV2Enabled()` |
| `EnterWorktreeTool`, `ExitWorktreeTool` | `isWorktreeModeEnabled()` |
| `TeamCreateTool`, `TeamDeleteTool` | `isAgentSwarmsEnabled()` |
| `LSPTool` | `ENABLE_LSP_TOOL` env |
| `WorkflowTool` | `WORKFLOW_SCRIPTS` feature |
| `SleepTool` | `PROACTIVE` or `KAIROS` feature |
| `Cron*Tool` | `AGENT_TRIGGERS` feature |
| `RemoteTriggerTool` | `AGENT_TRIGGERS_REMOTE` feature |
| `MonitorTool` | `MONITOR_TOOL` feature |
| `PowerShellTool` | `isPowerShellToolEnabled()` |
| `ToolSearchTool` | `isToolSearchEnabledOptimistic()` |

---

## 6. Key Patterns

### 6.1 Tool Composition

**Tool Definition Pattern:**
```typescript
export const ToolName = buildTool({
  name: TOOL_NAME,
  searchHint: 'brief capability description',
  maxResultSizeChars: 100_000,
  strict: true,

  async description() { return '...' },
  async prompt() { return getPromptTemplate() },

  get inputSchema() { return inputSchema() },
  get outputSchema() { return outputSchema() },

  userFacingName(input) { return '...' },
  getToolUseSummary(input) { return '...' },
  getActivityDescription(input) { return '...' },

  isConcurrencySafe() { return true },
  isReadOnly() { return true },
  isDestructive(input) { return false },

  toAutoClassifierInput(input) { return input.field },

  getPath(input) { return input.file_path },

  async preparePermissionMatcher({ field }) {
    return pattern => matchWildcardPattern(pattern, field)
  },

  async validateInput(input, context) {
    // Return ValidationResult
  },

  async checkPermissions(input, context) {
    // Return PermissionResult
  },

  renderToolUseMessage(input, options) { return <Component /> },
  renderToolResultMessage(content, options) { return <Component /> },

  async call(input, context, canUseTool, parentMessage, onProgress) {
    // Tool execution
    return { data: { ... } }
  },

  mapToolResultToToolResultBlockParam(data, toolUseID) {
    return { tool_use_id: toolUseID, type: 'tool_result', content: '...' }
  }
} satisfies ToolDef<InputSchema, Output>)
```

### 6.2 Permission Checking

**Three-Layer Permission System:**

1. **Tool-Level Validation** (`validateInput`):
   - Path existence checks
   - File state validation (read before edit)
   - Binary file detection
   - Size limits

2. **Tool-Level Permission Check** (`checkPermissions`):
   ```typescript
   async checkPermissions(input, context): Promise<PermissionDecision> {
     const appState = context.getAppState()
     return checkReadPermissionForTool(ToolName, input, appState.toolPermissionContext)
   }
   ```

3. **General Permission System** (`permissions.ts`):
   - Rule matching (allow/deny/ask)
   - Pattern matching (wildcards, prefixes)
   - Mode enforcement (default/plan/auto/danger)

**Permission Rule Matching:**
```typescript
export function matchWildcardPattern(pattern: string, value: string): boolean {
  // Convert glob-style patterns to regex
  const regexPattern = pattern
    .replace(/\./g, '\\.')
    .replace(/\*/g, '.*')
    .replace(/\?/g, '.')
  return new RegExp(`^${regexPattern}$`).test(value)
}
```

### 6.3 Activity Tracking

**Progress Tracking Pattern:**
```typescript
type ToolCallProgress<P extends ToolProgressData> = (progress: ToolProgress<P>) => void

// In tool call:
async call(input, context, canUseTool, parentMessage, onProgress?) {
  // Report progress
  onProgress?.({
    toolUseID: parentMessage.toolUseId,
    data: { type: 'tool_progress', ... }
  })

  // Continue execution
}
```

**Activity Description:**
```typescript
getActivityDescription(input) {
  const summary = getToolUseSummary(input)
  return summary ? `Reading ${summary}` : 'Reading file'
}
```

### 6.4 Error Handling

**Validation Result Pattern:**
```typescript
type ValidationResult =
  | { result: true }
  | {
      result: false
      message: string
      errorCode: number
    }

// Usage in validateInput:
async validateInput(input, context) {
  if (/* some condition */) {
    return {
      result: false,
      message: 'File does not exist',
      errorCode: 4,
      meta: { /* optional metadata */ }
    }
  }
  return { result: true }
}
```

**Permission Result Pattern:**
```typescript
type PermissionResult =
  | { behavior: 'allow'; updatedInput?: object }
  | { behavior: 'ask'; message: string; meta?: object }
  | { behavior: 'deny'; message: string }
  | { behavior: 'passthrough'; message: string }

// Usage in checkPermissions:
async checkPermissions(input, context) {
  if (/* denied */) {
    return { behavior: 'deny', message: '...' }
  }
  if (/* needs prompt */) {
    return { behavior: 'ask', message: '...', meta: { ... } }
  }
  return { behavior: 'allow', updatedInput: input }
}
```

---

## 7. Integration Points

### 7.1 Commands Integration

Tools expose their capabilities through `commands.ts`:

```typescript
// Tool commands exposed to the system
export function getCommand(name: string): Command | undefined
export function getSkillToolCommands(tools: Tools): Command[]
export function hasCommand(name: string): boolean
```

### 7.2 Services Integration

**MCP Client Integration (`services/mcp/client.ts`):**
```typescript
// Fetch tools from MCP server
export async function fetchToolsForClient(client: MCPServerConnection): Promise<Tools>
// Connect to server
export async function connectToServer(name: string, config: McpServerConfig): Promise<MCPServerConnection>
```

**LSP Integration:**
- `FileEditTool` notifies LSP servers on edit via `lspManager.changeFile()` and `lspManager.saveFile()`
- `LSPTool` provides language server protocol operations

### 7.3 Permissions Integration

**Permission Context Flow:**
```
AppState.toolPermissionContext
    │
    ├──► getTools(permissionContext)
    │        │
    │        └──► filterToolsByDenyRules(tools, permissionContext)
    │
    ├──► Tool.checkPermissions(input, context)
    │        │
    │        └──► checkReadPermissionForTool / checkWritePermissionForTool
    │
    └──► Permission prompts (UI.tsx)
```

**Permission Modes:**
- `default` - Standard prompting
- `plan` - Require plan approval
- `auto` - Auto-approve safe commands
- `danger` - Minimal restrictions

### 7.4 Hooks Integration

**Hook Points:**
- `pre_tool_use` - Before tool execution
- `post_tool_use` - After tool execution
- `session_start` - Agent session initialization
- `pre_compact` / `post_compact` - Around compaction

**Frontmatter Hooks:**
```typescript
// From loadAgentsDir.ts
type AgentDefinition = {
  hooks?: HooksSettings  // Session-scoped hooks
}

// Registration in runAgent.ts
await registerFrontmatterHooks(agentDefinition.hooks, subagentContext)
await executeSubagentStartHooks({ agentDefinition, prompt, context })
```

### 7.5 File State Cache

**readFileState Pattern:**
```typescript
// In FileReadTool.call()
readFileState.set(fullFilePath, {
  content,
  timestamp: Math.floor(mtimeMs),
  offset,
  limit,
})

// In FileEditTool.validateInput()
const readTimestamp = readFileState.get(fullFilePath)
if (!readTimestamp || readTimestamp.isPartialView) {
  return { result: false, message: 'File has not been read yet' }
}

// Check staleness
if (lastWriteTime > readTimestamp.timestamp) {
  const isFullRead = readTimestamp.offset === undefined && readTimestamp.limit === undefined
  if (isFullRead && fileContent === readTimestamp.content) {
    // Content unchanged, safe to proceed
  } else {
    return { result: false, message: 'File has been modified since read' }
  }
}
```

### 7.6 Skill System Integration

**Skill Discovery in File Operations:**
```typescript
// In FileReadTool.call() and FileEditTool.call()
const newSkillDirs = await discoverSkillDirsForPaths([absoluteFilePath], cwd)
if (newSkillDirs.length > 0) {
  for (const dir of newSkillDirs) {
    context.dynamicSkillDirTriggers?.add(dir)
  }
  addSkillDirectories(newSkillDirs).catch(() => {})  // Fire and forget
}

// Activate conditional skills
activateConditionalSkillsForPaths([absoluteFilePath], cwd)
```

---

## 8. Summary

The `tools/` module is a sophisticated execution framework with:

- **184 TypeScript files** organized into ~45 tool directories
- **Consistent interface** via `Tool` type and `buildTool()` factory
- **Layered security** with validation, permission checks, and sandbox execution
- **Feature-gated tools** for different deployment modes (ant/USER_TYPE, features)
- **MCP integration** for extensible server-provided tools
- **Agent system** with built-in, user, project, policy, and plugin agents
- **Progress tracking** for long-running operations
- **File state caching** for staleness detection
- **Skill discovery** from file operations
- **Hook integration** for custom behavior

Key files for understanding the module:
1. `src/Tool.ts` - Interface and type definitions
2. `src/tools.ts` - Tool pool assembly
3. `tools/AgentTool/AgentTool.tsx` - Agent spawning
4. `tools/BashTool/BashTool.tsx` - Command execution
5. `tools/FileReadTool/FileReadTool.ts` - Multi-format reading
6. `tools/FileEditTool/FileEditTool.ts` - In-place editing
7. `tools/GrepTool/GrepTool.ts` - Content search
8. `tools/AgentTool/runAgent.ts` - Agent execution lifecycle
9. `tools/AgentTool/loadAgentsDir.ts` - Agent definition loading

---

## 9. Detailed Tool Implementations

### 9.1 BashTool - Full Implementation Details

**Location:** `tools/BashTool/BashTool.tsx` (2621+ lines)

**Complete Input Schema:**
```typescript
const inputSchema = lazySchema(() => isBackgroundTasksDisabled 
  ? fullInputSchema().omit({
      run_in_background: true,
      _simulatedSedEdit: true,
    })
  : fullInputSchema()
)

const fullInputSchema = lazySchema(() => 
  z.strictObject({
    command: z.string().describe('The command to execute'),
    timeout: semanticNumber(z.number().int().min(1).max(getMaxTimeoutMs()))
      .optional()
      .describe(`Timeout in milliseconds (max: ${getMaxTimeoutMs()})`),
    description: z.string()
      .optional()
      .describe('Brief description of what this command does'),
    run_in_background: z.boolean()
      .optional()
      .describe('Set to true to run this command in the background'),
    dangerouslyDisableSandbox: z.boolean()
      .optional()
      .describe('Set to true to disable sandbox execution'),
  })
)
```

**Complete Output Schema:**
```typescript
const outputSchema = lazySchema(() => 
  z.strictObject({
    stdout: z.string().describe('Standard output from the command'),
    stderr: z.string().describe('Standard error from the command'),
    exit_code: z.number().nullable().describe('Exit code (null if signal)'),
    signal: z.number().nullable().describe('Signal number if killed by signal'),
    durationMs: z.number().describe('Execution time in milliseconds'),
    backgroundTaskId: z.string().optional().describe('ID if run_in_background'),
    persistedOutputPath: z.string().optional().describe('Path for large outputs'),
  })
)
```

**Search/Read Detection Logic:**
```typescript
const BASH_SEARCH_COMMANDS = new Set([
  'find', 'grep', 'rg', 'ag', 'ack', 'locate', 'which', 'whereis'
])

const BASH_READ_COMMANDS = new Set([
  'cat', 'head', 'tail', 'less', 'more',
  'wc', 'stat', 'file', 'strings',
  'jq', 'awk', 'cut', 'sort', 'uniq', 'tr'
])

const BASH_LIST_COMMANDS = new Set(['ls', 'tree', 'du'])

const BASH_SEMANTIC_NEUTRAL_COMMANDS = new Set([
  'echo', 'printf', 'true', 'false', ':'
])

export function isSearchOrReadBashCommand(command: string): {
  isSearch: boolean
  isRead: boolean
  isList: boolean
} {
  const partsWithOperators = splitCommandWithOperators(command)
  let hasSearch = false, hasRead = false, hasList = false
  let hasNonNeutralCommand = false
  let skipNextAsRedirectTarget = false
  
  for (const part of partsWithOperators) {
    if (skipNextAsRedirectTarget) {
      skipNextAsRedirectTarget = false
      continue
    }
    if (part === '>' || part === '>>' || part === '>&') {
      skipNextAsRedirectTarget = true
      continue
    }
    if (['||', '&&', '|', ';'].includes(part)) continue
    
    const baseCommand = part.trim().split(/\s+/)[0]
    if (!baseCommand) continue
    
    if (BASH_SEMANTIC_NEUTRAL_COMMANDS.has(baseCommand)) continue
    
    hasNonNeutralCommand = true
    const isPartSearch = BASH_SEARCH_COMMANDS.has(baseCommand)
    const isPartRead = BASH_READ_COMMANDS.has(baseCommand)
    const isPartList = BASH_LIST_COMMANDS.has(baseCommand)
    
    if (!isPartSearch && !isPartRead && !isPartList) {
      return { isSearch: false, isRead: false, isList: false }
    }
    
    if (isPartSearch) hasSearch = true
    if (isPartRead) hasRead = true
    if (isPartList) hasList = true
  }
  
  if (!hasNonNeutralCommand) {
    return { isSearch: false, isRead: false, isList: false }
  }
  
  return { isSearch: hasSearch, isRead: hasRead, isList: hasList }
}
```

**Sandbox Execution Flow:**
```typescript
async call(input, context, canUseTool, parentMessage, onProgress) {
  const sandboxManager = new SandboxManager({
    command: input.command,
    cwd: getCwd(),
    env: process.env,
    dangerouslyDisableSandbox: input.dangerouslyDisableSandbox,
  })
  
  // Security validation via bashPermissions.ts (2621 lines)
  const parsed = await parseForSecurity(input.command)
  const hasPermission = await bashToolHasPermission(
    input.command,
    context.getAppState().toolPermissionContext
  )
  
  // Execute with timeout and progress tracking
  const result = await exec(input.command, {
    timeout: input.timeout ?? getDefaultTimeoutMs(),
    signal: abortController.signal,
    onProgress: (data) => onProgress?.({ data }),
  })
  
  // Handle large output persistence
  if (result.stdout.length > PREVIEW_SIZE_BYTES) {
    const outputPath = await ensureToolResultsDir()
    await writeTextContent(outputPath, result.stdout, 'utf8', 'LF')
    return { data: { persistedOutputPath: outputPath, ... } }
  }
  
  return { data: result }
}
```

---

### 9.2 FileReadTool - Complete Implementation

**Location:** `tools/FileReadTool/FileReadTool.ts` (1183+ lines)

**Input Schema with Pagination:**
```typescript
const inputSchema = lazySchema(() => 
  z.strictObject({
    file_path: z.string().describe('Absolute path to the file'),
    offset: semanticNumber(z.number().int().min(0).optional())
      .optional()
      .describe('Line offset to start reading from (0-indexed)'),
    limit: semanticNumber(z.number().int().min(1).max(MAX_LINES_PER_READ))
      .optional()
      .describe('Maximum number of lines to read'),
    pages: z.string()
      .optional()
      .describe('PDF page range (e.g., "1-5", "3", "10-20")'),
  })
)
```

**Output Discriminated Union:**
```typescript
type FileReadOutput = 
  | { type: 'text'; content: string; filePath: string; ... }
  | { type: 'image'; imageSources: Base64ImageSource[]; filePath: string; ... }
  | { type: 'notebook'; cells: NotebookCell[]; filePath: string; ... }
  | { type: 'pdf'; pages: PDFPage[]; filePath: string; ... }
  | { type: 'parts'; parts: (TextContent | ImageContent)[]; filePath: string; ... }
  | { type: 'file_unchanged'; stub: string; filePath: string; ... }
```

**Image Compression with Token Budget:**
```typescript
export async function readImageWithTokenBudget(
  filePath: string,
  maxTokens: number,
  maxBytes?: number
): Promise<ImageResult> {
  const imageBuffer = await getFsImplementation().readFileBytes(
    filePath,
    maxBytes
  )
  
  // Detect format and get dimensions
  const format = detectImageFormatFromBuffer(imageBuffer)
  const dimensions = await getImageDimensions(imageBuffer)
  
  // Standard resize to fit token budget
  const resizedBuffer = await maybeResizeAndDownsampleImageBuffer(
    imageBuffer,
    format,
    maxTokens
  )
  
  // Aggressive compression if still exceeds budget
  const compressedBuffer = await compressImageBufferWithTokenLimit(
    resizedBuffer,
    format,
    maxTokens
  )
  
  const base64 = compressedBuffer.toString('base64')
  const metadataText = createImageMetadataText(filePath, dimensions, format)
  
  return {
    type: 'base64',
    data: base64,
    media_type: `image/${format}`,
    text: metadataText,
  }
}
```

**File Staleness Detection:**
```typescript
async call(input, context) {
  const fullFilePath = expandPath(input.file_path)
  const mtimeMs = await getFileModificationTimeAsync(fullFilePath)
  
  const existingRead = readFileState.get(fullFilePath)
  
  // Check if file was modified since last read
  if (existingRead && existingRead.timestamp >= Math.floor(mtimeMs)) {
    // File unchanged - return stub to save tokens
    return { data: { type: 'file_unchanged', stub: FILE_UNCHANGED_STUB } }
  }
  
  // Read file content
  const content = await readFile(fullFilePath, encoding)
  
  // Cache the read
  readFileState.set(fullFilePath, {
    content,
    timestamp: Math.floor(mtimeMs),
    offset: input.offset,
    limit: input.limit,
  })
  
  return { data: { type: 'text', content, filePath: fullFilePath } }
}
```

**Blocked Device Paths:**
```typescript
const BLOCKED_DEVICE_PATHS = new Set([
  // Infinite output
  '/dev/zero', '/dev/random', '/dev/urandom', '/dev/full',
  // Blocks waiting for input
  '/dev/stdin', '/dev/tty', '/dev/console',
  // Nonsensical to read
  '/dev/stdout', '/dev/stderr',
  // fd aliases
  '/dev/fd/0', '/dev/fd/1', '/dev/fd/2',
])

function isBlockedDevicePath(filePath: string): boolean {
  if (BLOCKED_DEVICE_PATHS.has(filePath)) return true
  // Linux proc aliases
  if (filePath.startsWith('/proc/') && 
      (filePath.endsWith('/fd/0') || filePath.endsWith('/fd/1') || filePath.endsWith('/fd/2'))) {
    return true
  }
  return false
}
```

---

### 9.3 FileEditTool - Complete Implementation

**Location:** `tools/FileEditTool/FileEditTool.ts` (775+ lines)

**Input Schema:**
```typescript
const inputSchema = lazySchema(() => 
  z.strictObject({
    file_path: z.string().describe('Absolute path to the file'),
    old_string: z.string().describe('The text to replace'),
    new_string: z.string().describe('The replacement text'),
    replace_all: z.boolean().optional().describe('Replace all occurrences'),
  })
)
```

**Output Schema:**
```typescript
const outputSchema = lazySchema(() => 
  z.strictObject({
    filePath: z.string(),
    old_string: z.string(),
    new_string: z.string(),
    occurrences: z.number().describe('Number of replacements made'),
    structuredPatch: hunkSchema().array().describe('Unified diff patch'),
  })
)
```

**Validation with Quote Style Detection:**
```typescript
async validateInput(input: FileEditInput, toolUseContext: ToolUseContext) {
  const { file_path, old_string, new_string } = input
  const fullFilePath = expandPath(file_path)
  
  // Secret detection for team memory files
  const secretError = checkTeamMemSecrets(fullFilePath, new_string)
  if (secretError) {
    return { result: false, message: secretError, errorCode: 0 }
  }
  
  // No-op detection
  if (old_string === new_string) {
    return {
      result: false,
      message: 'No changes: old_string and new_string are identical',
      errorCode: 1,
    }
  }
  
  // Deny rules check
  const denyRule = matchingRuleForInput(fullFilePath, 
    toolUseContext.getAppState().toolPermissionContext, 'edit', 'deny')
  if (denyRule !== null) {
    return { result: false, message: 'File denied by permissions', errorCode: 2 }
  }
  
  // File size guard (1 GiB max)
  try {
    const { size } = await fs.stat(fullFilePath)
    if (size > MAX_EDIT_FILE_SIZE) {
      return {
        result: false,
        message: `File too large (${formatFileSize(size)}). Max: ${formatFileSize(MAX_EDIT_FILE_SIZE)}`,
        errorCode: 10,
      }
    }
  } catch (e) {
    if (!isENOENT(e)) throw e
  }
  
  // Read confirmation
  const readTimestamp = toolUseContext.readFileState.get(fullFilePath)
  if (!readTimestamp || readTimestamp.isPartialView) {
    return { result: false, message: 'File has not been read yet', errorCode: 2 }
  }
  
  // Staleness check with content fallback
  const lastWriteTime = getFileModificationTime(fullFilePath)
  if (lastWriteTime > readTimestamp.timestamp) {
    const isFullRead = readTimestamp.offset === undefined && readTimestamp.limit === undefined
    const fileContent = await fs.readFile(fullFilePath, 'utf8')
    
    if (!isFullRead || fileContent !== readTimestamp.content) {
      return {
        result: false,
        message: 'File modified since read. Read it again before editing.',
        errorCode: 3,
      }
    }
  }
  
  // Quote style normalization
  const actualOld = await findActualString(fullFilePath, old_string)
  if (!actualOld) {
    return { result: false, message: 'old_string not found in file', errorCode: 4 }
  }
  
  return { result: true }
}
```

**Edit Execution with Patch Generation:**
```typescript
async call(input, context) {
  const { file_path, old_string, new_string, replace_all } = input
  const fullFilePath = expandPath(file_path)
  
  // Detect and preserve quote style
  const { quoteStyle, preservedOld } = await preserveQuoteStyle(
    fullFilePath,
    old_string,
    new_string
  )
  
  // Generate patch for display
  const patch = getPatchForEdit({
    filePath: file_path,
    fileContents: originalContent,
    edits: [{ old_string: preservedOld, new_string, replace_all }],
  })
  
  // Write with detected line endings
  await writeTextContent(fullFilePath, newContent, encoding, lineEnding)
  
  // Notify LSP
  const lspManager = getLspServerManager()
  if (lspManager) {
    clearDeliveredDiagnosticsForFile(`file://${fullFilePath}`)
    await lspManager.changeFile(fullFilePath, newContent)
    await lspManager.saveFile(fullFilePath)
  }
  
  // Update cache
  readFileState.set(fullFilePath, {
    content: newContent,
    timestamp: getFileModificationTime(fullFilePath),
  })
  
  return {
    data: {
      filePath: file_path,
      old_string,
      new_string,
      occurrences: 1,
      structuredPatch: patch,
    },
  }
}
```

---

### 9.4 FileWriteTool - Complete Implementation

**Location:** `tools/FileWriteTool/FileWriteTool.ts` (434+ lines)

**Input/Output Schemas:**
```typescript
const inputSchema = lazySchema(() => 
  z.strictObject({
    file_path: z.string().describe('Absolute path (must be absolute)'),
    content: z.string().describe('Content to write'),
  })
)

const outputSchema = lazySchema(() => 
  z.object({
    type: z.enum(['create', 'update']),
    filePath: z.string(),
    content: z.string(),
    structuredPatch: hunkSchema().array(),
    originalFile: z.string().nullable(),
    gitDiff: gitDiffSchema().optional(),
  })
)
```

**Write Validation - Read Confirmation Required:**
```typescript
async validateInput({ file_path, content }, toolUseContext) {
  const fullFilePath = expandPath(file_path)
  
  // Secret detection
  const secretError = checkTeamMemSecrets(fullFilePath, content)
  if (secretError) {
    return { result: false, message: secretError }
  }
  
  // Deny rules
  const denyRule = matchingRuleForInput(fullFilePath, 
    toolUseContext.getAppState().toolPermissionContext, 'edit', 'deny')
  if (denyRule !== null) {
    return { result: false, message: 'File denied by permissions' }
  }
  
  // Skip UNC paths for security
  if (fullFilePath.startsWith('\\\\') || fullFilePath.startsWith('//')) {
    return { result: true }
  }
  
  // Check file exists and was read
  const fileStat = await fs.stat(fullFilePath)
  const fileMtimeMs = fileStat.mtimeMs
  const readTimestamp = toolUseContext.readFileState.get(fullFilePath)
  
  if (!readTimestamp || readTimestamp.isPartialView) {
    return {
      result: false,
      message: 'File has not been read yet. Read it first before writing.',
      errorCode: 2,
    }
  }
  
  // Staleness check
  const lastWriteTime = Math.floor(fileMtimeMs)
  if (lastWriteTime > readTimestamp.timestamp) {
    return {
      result: false,
      message: 'File modified since read. Read it again before writing.',
      errorCode: 3,
    }
  }
  
  return { result: true }
}
```

**Write Execution with LSP Notification:**
```typescript
async call({ file_path, content }, context, _, parentMessage) {
  const fullFilePath = expandPath(file_path)
  const dir = dirname(fullFilePath)
  
  // Skill discovery from file path
  const newSkillDirs = await discoverSkillDirsForPaths([fullFilePath], getCwd())
  for (const dir of newSkillDirs) {
    context.dynamicSkillDirTriggers?.add(dir)
  }
  addSkillDirectories(newSkillDirs).catch(() => {})
  
  // Activate conditional skills
  activateConditionalSkillsForPaths([fullFilePath], getCwd())
  
  // File history backup
  if (fileHistoryEnabled()) {
    await fileHistoryTrackEdit(updateFileHistoryState, fullFilePath, parentMessage.uuid)
  }
  
  // Atomic read-modify-write with staleness check
  let meta: ReturnType<typeof readFileSyncWithMetadata> | null
  try {
    meta = readFileSyncWithMetadata(fullFilePath)
  } catch (e) {
    if (isENOENT(e)) meta = null
    else throw e
  }
  
  const oldContent = meta?.content ?? null
  const encoding = meta?.encoding ?? 'utf8'
  
  // Write with explicit line endings (no auto-conversion)
  writeTextContent(fullFilePath, content, encoding, 'LF')
  
  // LSP notification
  const lspManager = getLspServerManager()
  if (lspManager) {
    clearDeliveredDiagnosticsForFile(`file://${fullFilePath}`)
    await lspManager.changeFile(fullFilePath, content)
    await lspManager.saveFile(fullFilePath)
  }
  
  // VSCode diff view notification
  notifyVscodeFileUpdated(fullFilePath, oldContent, content)
  
  // Update read cache
  readFileState.set(fullFilePath, {
    content,
    timestamp: getFileModificationTime(fullFilePath),
  })
  
  // Generate diff
  let gitDiff: ToolUseDiff | undefined
  if (isEnvTruthy(process.env.CLAUDE_CODE_REMOTE) && 
      getFeatureValue_CACHED_MAY_BE_STALE('tengu_quartz_lantern', false)) {
    gitDiff = await fetchSingleFileGitDiff(fullFilePath)
  }
  
  if (oldContent) {
    const patch = getPatchForDisplay({
      filePath: file_path,
      fileContents: oldContent,
      edits: [{ old_string: oldContent, new_string: content, replace_all: false }],
    })
    countLinesChanged(patch)
    
    return {
      data: {
        type: 'update',
        filePath: file_path,
        content,
        structuredPatch: patch,
        originalFile: oldContent,
        gitDiff,
      },
    }
  }
  
  // New file creation
  countLinesChanged([], content)
  return {
    data: {
      type: 'create',
      filePath: file_path,
      content,
      structuredPatch: [],
      originalFile: null,
      gitDiff,
    },
  }
}
```

---

### 9.5 GrepTool - RipGrep Integration

**Location:** `tools/GrepTool/GrepTool.ts` (577+ lines)

**Input Schema with Multiple Output Modes:**
```typescript
const inputSchema = lazySchema(() => 
  z.strictObject({
    pattern: z.string().describe('Regex pattern to search for'),
    path: z.string().optional().describe('Directory to search (default: cwd)'),
    output_mode: z.enum(['content', 'files_with_matches', 'count'])
      .optional()
      .describe('Output format'),
    context: semanticNumber(z.number().int().min(0).optional())
      .optional()
      .describe('Lines of context (-C flag)'),
    type: z.string().optional().describe('File type filter (--type)'),
    case_sensitive: z.boolean().optional().describe('Case-sensitive search'),
    head_limit: semanticNumber(z.number().int().min(0).optional())
      .optional()
      .describe('Limit results (like | head -N)'),
  })
)
```

**RipGrep Argument Construction:**
```typescript
async call(input, { abortController, getAppState }) {
  const start = Date.now()
  const absolutePath = input.path ? expandPath(input.path) : getCwd()
  
  // Build ripgrep arguments
  const args = ['--hidden']  // Include hidden files
  
  // Exclude VCS directories
  for (const dir of VCS_DIRECTORIES_TO_EXCLUDE) {
    args.push('--glob', `!${dir}`)
  }
  
  // Context flags
  if (input.context !== undefined) {
    args.push('-C', String(input.context))
  }
  
  // Case sensitivity
  if (input.case_sensitive === false) {
    args.push('-i')
  }
  
  // Type filtering
  if (input.type) {
    args.push('--type', input.type)
  }
  
  // Output mode
  if (input.output_mode === 'count') {
    args.push('--count')
  } else if (input.output_mode === 'files_with_matches') {
    args.push('--files-with-matches')
  }
  
  // Max column width
  args.push('--max-columns', '500')
  
  // Execute ripgrep
  const results = await ripGrep(
    [input.pattern, ...args],
    absolutePath,
    abortController.signal
  )
  
  // Apply head_limit
  const limit = input.head_limit ?? DEFAULT_HEAD_LIMIT
  const limitedResults = results.slice(0, limit)
  
  // Process based on output mode
  if (input.output_mode === 'content') {
    const content = limitedResults.join('\n')
    return { data: { mode: 'content', content, numLines: limitedResults.length } }
  }
  
  if (input.output_mode === 'count') {
    let totalMatches = 0, fileCount = 0
    for (const line of limitedResults) {
      const count = parseInt(line.split(':').pop()!)
      if (!isNaN(count)) {
        totalMatches += count
        fileCount += 1
      }
    }
    return { data: { mode: 'count', numFiles: fileCount, numMatches: totalMatches } }
  }
  
  // files_with_matches mode - sort by mtime
  const stats = await Promise.allSettled(
    limitedResults.map(f => fs.stat(f))
  )
  const sortedMatches = limitedResults
    .map((f, i) => [f, stats[i].status === 'fulfilled' ? stats[i].value.mtimeMs : 0] as const)
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(_ => _[0])
  
  return { 
    data: { 
      mode: 'files_with_matches', 
      filenames: sortedMatches.map(toRelativePath),
      numFiles: sortedMatches.length,
    }
  }
}
```

---

### 9.6 AgentTool - Subagent Execution

**Location:** `tools/AgentTool/AgentTool.tsx` (2592+ lines)

**Input Schema with Multi-Agent Support:**
```typescript
const baseInputSchema = lazySchema(() => 
  z.object({
    description: z.string().describe('Short (3-5 word) task description'),
    prompt: z.string().describe('Task for the agent to perform'),
    subagent_type: z.string().optional()
      .describe('Type of specialized agent'),
    model: z.enum(['sonnet', 'opus', 'haiku']).optional()
      .describe('Model override for this agent'),
  })
)

const fullInputSchema = lazySchema(() => 
  baseInputSchema().merge(z.object({
    name: z.string().optional()
      .describe('Name for spawned agent (addressable via SendMessage)'),
    team_name: z.string().optional()
      .describe('Team name for spawning'),
    mode: permissionModeSchema().optional()
      .describe('Permission mode for teammate'),
    isolation: z.enum(['worktree', 'remote']).optional()
      .describe('Isolation: worktree or remote CCR'),
    cwd: z.string().optional()
      .describe('Absolute path for agent working directory'),
  }))
)
```

**Output Union Schema:**
```typescript
const outputSchema = lazySchema(() => 
  z.union([
    // Sync completion
    agentToolResultSchema().extend({
      status: z.literal('completed'),
      prompt: z.string(),
    }),
    // Async launch
    z.object({
      status: z.literal('async_launched'),
      agentId: z.string(),
      description: z.string(),
      prompt: z.string(),
      outputFile: z.string(),
      canReadOutputFile: z.boolean().optional(),
    }),
    // Teammate spawn (internal)
    z.object({
      status: z.literal('teammate_spawned'),
      prompt: z.string(),
      teammate_id: z.string(),
      agent_id: z.string(),
      name: z.string(),
      color: z.string().optional(),
      tmux_session_name: z.string(),
      tmux_window_name: z.string(),
      tmux_pane_id: z.string(),
    }),
    // Remote launch (internal)
    z.object({
      status: z.literal('remote_launched'),
      taskId: z.string(),
      sessionUrl: z.string(),
      description: z.string(),
      prompt: z.string(),
    }),
  ])
)
```

**Fork Subagent Guard:**
```typescript
const FORK_AGENT = { agentType: 'fork', ... }

function isForkSubagentEnabled(): boolean {
  return getFeatureValue_CACHED_MAY_BE_STALE('fork_subagent', false)
}

function isInForkChild(messages: Message[]): boolean {
  // Check if current context is already inside a fork
  return messages.some(m => 
    m.role === 'assistant' && 
    m.content.some(c => c.type === 'tool_use' && c.name === 'fork')
  )
}

// In AgentTool.call():
const effectiveType = subagent_type ?? 
  (isForkSubagentEnabled() ? undefined : GENERAL_PURPOSE_AGENT.agentType)
const isForkPath = effectiveType === undefined

if (isForkPath) {
  // Fork guard - prevent recursive forks
  if (toolUseContext.options.querySource === `agent:builtin:${FORK_AGENT.agentType}` ||
      isInForkChild(toolUseContext.messages)) {
    throw new Error('Fork not available inside a forked worker.')
  }
  selectedAgent = FORK_AGENT
}
```

**MCP Server Requirements Check:**
```typescript
async function checkMcpRequirements(
  agentDefinition: AgentDefinition,
  timeoutMs: number = 30000
): Promise<{ ready: boolean; missing?: string[] }> {
  const required = agentDefinition.mcpServers?.filter(s => typeof s === 'string') ?? []
  if (required.length === 0) return { ready: true }
  
  const parentClients = context.getMcpClients()
  const availableNames = parentClients
    .filter(c => c.type === 'connected')
    .map(c => c.name)
  
  const missing = required.filter(name => !availableNames.includes(name))
  
  if (missing.length > 0) {
    // Wait for servers to connect (up to timeoutMs)
    const started = Date.now()
    while (Date.now() - started < timeoutMs) {
      const currentClients = context.getMcpClients()
      const stillMissing = missing.filter(
        name => !currentClients.some(c => c.name === name && c.type === 'connected')
      )
      if (stillMissing.length === 0) return { ready: true }
      await sleep(100)
    }
    return { ready: false, missing }
  }
  
  return { ready: true }
}
```

---

### 9.7 runAgent - Subagent Execution Lifecycle

**Location:** `tools/AgentTool/runAgent.ts` (974+ lines)

**AsyncGenerator Pattern:**
```typescript
export async function* runAgent({
  agentDefinition,
  prompt,
  parentContext,
  agentId,
  allowedTools,
  model,
  transcriptRouting,
  permissionMode,
}: {
  agentDefinition: AgentDefinition
  prompt: string
  parentContext: ToolUseContext
  agentId: AgentId
  allowedTools?: string[]
  model?: ModelAlias
  transcriptRouting?: { subdir: string }
  permissionMode?: PermissionMode
}): AsyncGenerator<Message, void> {
  // Resolve model
  const resolvedModel = model ?? getAgentModel(agentDefinition, parentContext)
  
  // Clone file state cache for isolation
  const clonedFileState = cloneFileStateCache(parentContext.readFileState)
  
  // Build system prompt
  const systemPrompt = await buildEffectiveSystemPrompt({
    agentDefinition,
    parentContext,
    model: resolvedModel,
  })
  
  // Register frontmatter hooks
  await registerFrontmatterHooks(agentDefinition.hooks, subagentContext)
  await executeSubagentStartHooks({ agentDefinition, prompt, context: subagentContext })
  
  // Initialize MCP servers
  const { clients: mcpClients, tools: mcpTools, cleanup: cleanupMcp } = 
    await initializeAgentMcpServers(agentDefinition, parentContext.getMcpClients())
  
  try {
    // Build tool pool
    const toolPool = allowedTools 
      ? resolveAgentTools(allTools, allowedTools)
      : allTools
    
    // Create subagent context
    const subagentContext = createSubagentContext({
      parentContext,
      agentId,
      fileState: clonedFileState,
      mcpClients,
      tools: toolPool,
      permissionMode: permissionMode ?? agentDefinition.mode,
    })
    
    // Run query loop
    const queryStream = query({
      prompt,
      context: subagentContext,
      model: resolvedModel,
      systemPrompt,
    })
    
    // Record sidechain transcript
    if (transcriptRouting?.subdir) {
      setAgentTranscriptSubdir(agentId, transcriptRouting.subdir)
    }
    
    for await (const message of queryStream) {
      yield message
      // Record to sidechain
      recordSidechainTranscript(agentId, message)
    }
  } finally {
    // Cleanup: MCP servers
    await cleanupMcp()
    
    // Cleanup: hooks
    clearSessionHooks()
    
    // Cleanup: file state
    // (cloned cache is discarded)
    
    // Cleanup: shell tasks
    await killShellTasksForAgent(agentId)
    
    // Cleanup: Perfetto agent
    if (isPerfettoTracingEnabled()) {
      unregisterPerfettoAgent(agentId)
    }
    
    // Cleanup: transcript subdir
    if (transcriptRouting?.subdir) {
      clearAgentTranscriptSubdir(agentId)
    }
  }
}
```

**Permission Mode Override for Read-Only Agents:**
```typescript
// For read-only agents (Explore, Plan), skip permission prompts
const shouldAvoidPermissionPrompts = 
  agentDefinition.source === 'read-only-built-in' ||
  permissionMode === 'plan'

const agentGetAppState = () => ({
  ...baseAppState,
  toolPermissionContext: {
    ...baseAppState.toolPermissionContext,
    shouldAvoidPermissionPrompts,
    mode: permissionMode ?? baseAppState.toolPermissionContext.mode,
  },
})
```

**Tool Filtering:**
```typescript
function resolveAgentTools(
  allTools: Tools,
  allowedTools: string[]
): Tools {
  if (!allowedTools || allowedTools.length === 0) return allTools
  
  const toolMap = new Map(allTools.map(t => [t.name, t]))
  const resolved: Tools = []
  
  for (const name of allowedTools) {
    const tool = toolMap.get(name)
    if (tool) resolved.push(tool)
  }
  
  return resolved
}
```

---

## 10. Tool Execution Pipeline

### 10.1 Tool Pool Assembly (src/tools.ts)

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
    WebSearchTool,
    GlobTool,
    GrepTool,
    SkillTool,
    BriefTool,
    TodoWriteTool,
    TaskCreateTool,
    TaskReplaceTool,
    TaskUpdateTool,
    TaskDeleteTool,
    AskUserQuestionTool,
    PowerShellTool,
    ...(isEnvTruthy(process.env.ENABLE_LSP_TOOL) ? [LSPTool] : []),
    ...(process.env.USER_TYPE === 'ant' ? [ConfigTool] : []),
    ...(isWorktreeModeEnabled() ? [EnterWorktreeTool, ExitWorktreeTool] : []),
    ...(isAgentSwarmsEnabled() ? [TeamCreateTool, TeamDeleteTool] : []),
    // ... 20+ more conditional tools
  ]
}

export function getTools(permissionContext: ToolPermissionContext): Tools {
  let tools = getAllBaseTools()
  
  // Simple mode filtering
  if (process.env.CLAUDE_CODE_SIMPLE === '1') {
    tools = tools.filter(t => SIMPLE_MODE_TOOLS.includes(t.name))
  }
  
  // Apply deny rules
  tools = filterToolsByDenyRules(tools, permissionContext)
  
  // Filter by isEnabled()
  tools = tools.filter(t => t.isEnabled())
  
  return tools
}

export function assembleToolPool(
  permissionContext: ToolPermissionContext,
  mcpTools?: Tools
): Tools {
  const builtinTools = getTools(permissionContext)
  
  // Filter MCP tools by deny rules
  const filteredMcpTools = mcpTools 
    ? filterToolsByDenyRules(mcpTools, permissionContext)
    : []
  
  // Sort each partition for prompt-cache stability
  const sortedBuiltin = [...builtinTools].sort((a, b) => a.name.localeCompare(b.name))
  const sortedMcp = [...filteredMcpTools].sort((a, b) => a.name.localeCompare(b.name))
  
  // Merge with uniqBy (builtin wins conflicts)
  return uniqBy([...sortedBuiltin, ...sortedMcp], t => t.name)
}
```

### 10.2 Query Integration (src/query.ts)

```typescript
export async function* query({
  prompt,
  context,
  model,
  systemPrompt,
}: QueryParams): AsyncGenerator<Message> {
  const abortController = new AbortController()
  
  // Build system message
  const systemMessage: SystemMessage = {
    role: 'system',
    content: systemPrompt,
  }
  
  // Build user message
  const userMessage: UserMessage = createUserMessage(prompt)
  
  // Initialize message history
  const messages: Message[] = [systemMessage, userMessage]
  
  // Main query loop
  while (true) {
    // Call model API
    const response = await callClaudeApi({
      messages,
      model,
      tools: context.getTools(),
      signal: abortController.signal,
    })
    
    // Process response content
    for (const content of response.content) {
      if (content.type === 'text') {
        const assistantMessage: AssistantMessage = {
          role: 'assistant',
          content: [content],
          toolUseId: content.id,
        }
        messages.push(assistantMessage)
        yield assistantMessage
      }
      
      if (content.type === 'tool_use') {
        // Execute tool via StreamingToolExecutor
        const tool = context.getTools().find(t => t.name === content.name)
        if (!tool) {
          throw new Error(`Unknown tool: ${content.name}`)
        }
        
        const toolResult = yield* StreamingToolExecutor.execute({
          tool,
          input: content.input,
          context,
          toolUseId: content.id,
          abortController,
        })
        
        messages.push(toolResult)
      }
    }
    
    if (response.stop_reason === 'end_turn') break
  }
}
```

### 10.3 StreamingToolExecutor

```typescript
export class StreamingToolExecutor {
  static async* execute({
    tool,
    input,
    context,
    toolUseId,
    abortController,
  }: {
    tool: Tool
    input: Record<string, unknown>
    context: ToolUseContext
    toolUseId: string
    abortController: AbortController
  }): AsyncGenerator<Message> {
    // Validate input
    if (tool.validateInput) {
      const validation = await tool.validateInput(input, context)
      if (!validation.result) {
        yield {
          role: 'user',
          content: [{ type: 'text', text: validation.message }],
        }
        return
      }
    }
    
    // Check permissions
    const permissionResult = await tool.checkPermissions(input, context)
    if (permissionResult.behavior === 'deny') {
      yield {
        role: 'user',
        content: [{ type: 'text', text: permissionResult.message }],
      }
      return
    }
    if (permissionResult.behavior === 'ask') {
      // Show permission prompt to user
      const userResponse = yield* showPermissionPrompt(permissionResult)
      if (userResponse !== 'approve') {
        yield {
          role: 'user',
          content: [{ type: 'text', text: 'Tool use denied by user' }],
        }
        return
      }
    }
    
    // Execute tool
    const result = await tool.call(
      input as never,
      context,
      context.canUseTool,
      context.parentMessage,
      (progress) => {
        // Handle progress updates
        yield {
          role: 'user',
          content: [{ type: 'tool_result', tool_use_id: toolUseId, ...progress }],
        }
      }
    )
    
    // Convert to tool_result message
    const toolResultBlock = tool.mapToolResultToToolResultBlockParam(
      result.data,
      toolUseId
    )
    
    yield {
      role: 'user',
      content: [toolResultBlock],
    }
  }
}
```

---

## 11. Permission System Deep Dive

### 11.1 Permission Result Types

```typescript
type PermissionResult =
  | { behavior: 'allow'; updatedInput?: object }
  | { behavior: 'ask'; message: string; meta?: object }
  | { behavior: 'deny'; message: string }
  | { behavior: 'passthrough'; message: string }

type PermissionDecision = PermissionResult & {
  rule?: PermissionRule  // Matching rule that was applied
}
```

### 11.2 Permission Rule Matching

```typescript
type PermissionRule = {
  source: 'alwaysAllow' | 'alwaysDeny' | 'alwaysAsk'
  pattern: string
  action: 'read' | 'edit' | 'bash'
}

export function matchWildcardPattern(
  pattern: string,
  value: string
): boolean {
  const regexPattern = pattern
    .replace(/\./g, '\\.')
    .replace(/\*/g, '.*')
    .replace(/\?/g, '.')
  return new RegExp(`^${regexPattern}$`).test(value)
}

export function matchingRuleForInput(
  filePath: string,
  context: ToolPermissionContext,
  action: 'read' | 'edit' | 'bash',
  ruleType: 'allow' | 'deny' | 'ask'
): PermissionRule | null {
  const rules = ruleType === 'allow' 
    ? context.alwaysAllowRules 
    : ruleType === 'deny' 
      ? context.alwaysDenyRules 
      : context.alwaysAskRules
  
  for (const rule of Object.values(rules).flat()) {
    if (rule.action === action && matchWildcardPattern(rule.pattern, filePath)) {
      return rule
    }
  }
  
  return null
}
```

### 11.3 Permission Check Flow

```
┌─────────────────────────────────────────────────────────────┐
│                  Permission Check Flow                       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Tool.validateInput()                                     │
│     - Path existence                                        │
│     - File state (read before edit)                         │
│     - Size limits                                           │
│     - Binary detection                                      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Tool.checkPermissions()                                  │
│     - checkReadPermissionForTool()                          │
│     - checkWritePermissionForTool()                         │
│     - bashPermissions.ts (2621 lines)                       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  3. General Permission System                                │
│     - Rule matching (allow/deny/ask)                        │
│     - Pattern matching (wildcards, prefixes)                │
│     - Mode enforcement (default/plan/auto/danger)           │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  4. Permission Prompt (if behavior: 'ask')                   │
│     - UI.tsx dialog                                         │
│     - User decision (approve/deny/always)                   │
└─────────────────────────────────────────────────────────────┘
```

### 11.4 Bash Permission System

**Location:** `tools/BashTool/bashPermissions.ts` (2621 lines)

```typescript
export async function bashToolHasPermission(
  command: string,
  context: ToolPermissionContext
): Promise<boolean> {
  // Parse command AST
  const parsed = await parseForSecurity(command)
  
  // Check for dangerous patterns
  if (containsDangerousPattern(parsed)) {
    return false
  }
  
  // Check cd commands (directory changes)
  if (commandHasAnyCd(command)) {
    const cdPaths = extractCdTargets(command)
    for (const cdPath of cdPaths) {
      if (!isPathAllowed(cdPath, context)) {
        return false
      }
    }
  }
  
  // Check file operations against rules
  const fileOps = extractFileOperations(parsed)
  for (const op of fileOps) {
    const rule = matchingRuleForInput(op.path, context, 'bash', 
      op.type === 'read' ? 'allow' : 'deny')
    if (rule?.source === 'alwaysDeny') {
      return false
    }
  }
  
  return true
}
```

---

## 12. Tool Result Persistence

### 12.1 Budget Management

```typescript
// Tool result budget constants
const TOOL_RESULT_BUDGET_CHARS = 5_000_000  // 5MB per tool result
const TOOL_RESULT_BUDGET_TOKENS = 1_000_000  // 1M tokens

interface ToolResultBudget {
  used: number
  limit: number
  evictions: Array<{
    toolUseId: string
    chars: number
    timestamp: number
  }>
}

// Track budget usage
function trackToolResultUsage(
  budget: ToolResultBudget,
  toolUseId: string,
  chars: number
): boolean {
  if (budget.used + chars <= budget.limit) {
    budget.used += chars
    return true
  }
  
  // Need to evict
  const toEvict = budget.evictions.shift()
  if (toEvict) {
    budget.used -= toEvict.chars
    budget.used += chars
    return true
  }
  
  return false  // Cannot accommodate
}
```

### 12.2 Disk Persistence

```typescript
// Tool result storage paths
const TOOL_RESULTS_DIR = path.join(os.tmpdir(), 'claude-code-tool-results')

export async function ensureToolResultsDir(): Promise<string> {
  await fs.mkdir(TOOL_RESULTS_DIR, { recursive: true })
  return TOOL_RESULTS_DIR
}

export function getToolResultPath(toolUseId: string): string {
  return path.join(TOOL_RESULTS_DIR, `${toolUseId}.json`)
}

export async function persistToolResult(
  toolUseId: string,
  result: unknown
): Promise<string> {
  const outputPath = getToolResultPath(toolUseId)
  const content = JSON.stringify(result, null, 2)
  await writeTextContent(outputPath, content, 'utf8', 'LF')
  return outputPath
}

export function buildLargeToolResultMessage(
  toolName: string,
  outputPath: string,
  previewSize: number = PREVIEW_SIZE_BYTES
): string {
  return `The ${toolName} output was too large to display. ` +
    `Output saved to: ${outputPath}\n` +
    `Use FileReadTool with offset/limit to view portions.`
}
```

---

## 13. MCP Tool Integration

### 13.1 MCP Tool Discovery

**Location:** `services/mcp/client.ts`

```typescript
export async function fetchToolsForClient(
  client: MCPServerConnection
): Promise<Tools> {
  if (client.type !== 'connected') return []
  
  const mcpTools = await client.connection.listTools()
  
  // Convert MCP tools to internal Tool type
  const tools: Tools = mcpTools.tools.map(mcpTool => ({
    name: mcpTool.name,
    aliases: mcpTool.aliases,
    description: async () => mcpTool.description ?? 'No description',
    prompt: async () => mcpTool.description ?? 'No description',
    inputSchema: lazySchema(() => 
      z.object(mcpTool.inputSchema.properties as any)
    ),
    isConcurrencySafe: () => true,
    isReadOnly: () => true,  // MCP tools are typically read-only
    checkPermissions: async (input, context) => ({
      behavior: 'allow',
      updatedInput: input,
    }),
    maxResultSizeChars: 100_000,
    userFacingName: () => mcpTool.name,
    toAutoClassifierInput: (input) => JSON.stringify(input),
    call: async (input, callContext) => {
      const result = await client.connection.callTool({
        name: mcpTool.name,
        arguments: input,
      })
      return { data: result }
    },
    mapToolResultToToolResultBlockParam: (data, toolUseId) => ({
      tool_use_id: toolUseId,
      type: 'tool_result',
      content: JSON.stringify(data),
    }),
  }))
  
  return tools
}
```

### 13.2 MCP Tool Requirements Checking

```typescript
export function hasRequiredMcpServers(
  agentDefinition: AgentDefinition,
  mcpClients: MCPServerConnection[]
): { ready: boolean; missing: string[] } {
  const required = agentDefinition.mcpServers?.filter(
    (s): s is string => typeof s === 'string'
  ) ?? []
  
  const available = mcpClients
    .filter(c => c.type === 'connected')
    .map(c => c.name)
  
  const missing = required.filter(name => !available.includes(name))
  
  return {
    ready: missing.length === 0,
    missing,
  }
}

export async function filterAgentsByMcpRequirements(
  agents: AgentDefinition[],
  mcpClients: MCPServerConnection[]
): Promise<AgentDefinition[]> {
  const result: AgentDefinition[] = []
  
  for (const agent of agents) {
    const { ready, missing } = hasRequiredMcpServers(agent, mcpClients)
    
    if (ready) {
      result.push(agent)
    } else {
      logForDebugging(
        `Agent '${agent.agentType}' requires MCP servers: ${missing.join(', ')}`
      )
    }
  }
  
  return result
}
```

---

## 14. Tool Discovery and Registration

### 14.1 Skill Discovery from File Operations

```typescript
// In FileReadTool.call() and FileEditTool.call()
async function discoverSkillsFromFileAccess(
  filePath: string,
  context: ToolUseContext
): Promise<void> {
  const cwd = getCwd()
  const absolutePath = expandPath(filePath)
  
  // Discover skill directories from this file's path
  const newSkillDirs = await discoverSkillDirsForPaths([absolutePath], cwd)
  
  if (newSkillDirs.length > 0) {
    // Add to dynamic skill triggers for display
    for (const dir of newSkillDirs) {
      context.dynamicSkillDirTriggers?.add(dir)
    }
    
    // Load skills asynchronously (fire and forget)
    addSkillDirectories(newSkillDirs).catch(() => {})
  }
  
  // Activate conditional skills whose path patterns match this file
  activateConditionalSkillsForPaths([absolutePath], cwd)
}
```

### 14.2 Agent Definition Loading

**Location:** `tools/AgentTool/loadAgentsDir.ts`

```typescript
export interface AgentDefinition {
  agentType: string
  name: string
  description: string
  prompt?: string
  model?: ModelAlias
  mode?: PermissionMode
  hooks?: HooksSettings
  mcpServers?: (string | Record<string, MCPConfig>)[]
  source: 'built-in' | 'user' | 'project' | 'policy' | 'plugin'
  color?: string
}

export async function loadAgentsDir(): Promise<AgentDefinitionsResult> {
  const agents: AgentDefinition[] = []
  
  // Load built-in agents
  for (const builtIn of BUILTIN_AGENTS) {
    agents.push({ ...builtIn, source: 'built-in' })
  }
  
  // Load user agents from ~/.claude/agents/
  const userAgents = await loadAgentsFromDir(getUserAgentsDir())
  agents.push(...userAgents.map(a => ({ ...a, source: 'user' })))
  
  // Load project agents from .claude/agents/
  const projectAgents = await loadAgentsFromDir(getProjectAgentsDir())
  agents.push(...projectAgents.map(a => ({ ...a, source: 'project' })))
  
  // Load plugin agents
  const pluginAgents = await loadPluginAgents()
  agents.push(...pluginAgents.map(a => ({ ...a, source: 'plugin' })))
  
  return { agents }
}

async function loadAgentsFromDir(dir: string): Promise<Partial<AgentDefinition>[]> {
  const agents: Partial<AgentDefinition>[] = []
  
  try {
    const entries = await fs.readdir(dir, { withFileTypes: true })
    
    for (const entry of entries) {
      if (!entry.isDirectory()) continue
      
      const agentDir = path.join(dir, entry.name)
      const agentFile = path.join(agentDir, 'index.md')
      
      const content = await fs.readFile(agentFile, 'utf8')
      const agent = parseAgentFrontmatter(content)
      
      if (agent) {
        agents.push({
          agentType: entry.name,
          ...agent,
        })
      }
    }
  } catch (e) {
    if (!isENOENT(e)) {
      logError(e, `Failed to load agents from ${dir}`)
    }
  }
  
  return agents
}
```

---

## 15. Summary

The `tools/` module is a comprehensive execution framework with:

- **184+ TypeScript files** organized into ~45 specialized tool directories
- **Unified Tool interface** with 60+ properties via `buildTool()` factory
- **Three-layer security**: validation, permission checks, sandbox execution
- **Feature-gated tool pool** for different deployment modes
- **MCP integration** for extensible server-provided tools
- **Multi-agent system** with sync/async/teammate/remote execution modes
- **Fork subagent support** with recursive fork detection
- **Progress tracking** for long-running operations via `onProgress` callback
- **File state caching** with staleness detection for concurrent edit prevention
- **Skill discovery** triggered by file access patterns
- **Hook integration** for custom before/after behavior
- **LSP integration** for real-time diagnostics on file edits
- **Tool result persistence** with 5MB budget management
- **Permission rule system** with allow/deny/ask patterns and wildcard matching

**Key Architecture Patterns:**

1. **Lazy Schema Evaluation** - `lazySchema()` defers Zod schema construction
2. **Semantic Wrappers** - `semanticNumber()`, `semanticBoolean()` for clearer schemas
3. **Discriminated Unions** - Output types use `type` field for narrowing
4. **AsyncGenerator Pattern** - `runAgent()` yields messages incrementally
5. **Clone-on-Write Isolation** - File state cloned for subagent contexts
6. **Fire-and-Forget** - Skill loading, MCP connections non-blocking
7. **Progressive Disclosure** - Search/read commands collapsible in UI
8. **Atomic Operations** - Read-modify-write with staleness guards

The module demonstrates sophisticated engineering around security, isolation, extensibility, and user experience.
