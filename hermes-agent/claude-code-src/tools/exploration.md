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
