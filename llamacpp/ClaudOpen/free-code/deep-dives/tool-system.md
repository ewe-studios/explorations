# free-code Tool System Deep-Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code`

A comprehensive exploration of the tool system architecture in free-code.

---

## Table of Contents

1. [Overview](#overview)
2. [Tool Architecture](#tool-architecture)
3. [Core Tools](#core-tools)
4. [Tool Implementation](#tool-implementation)
5. [Permission System](#permission-system)
6. [MCP Tool Integration](#mcp-tool-integration)
7. [Tool Execution Flow](#tool-execution-flow)

---

## Overview

free-code includes **40+ built-in tools** plus support for unlimited MCP (Model Context Protocol) tools:

```
┌─────────────────────────────────────────────────────────────┐
│                      Tool Categories                         │
├─────────────────┬─────────────────┬─────────────────────────┤
│   File Tools    │   Search Tools  │   Network Tools         │
│   • Read        │   • Grep        │   • WebFetch            │
│   • Edit        │   • Glob        │   • WebSearch           │
│   • Write       │                 │                         │
├─────────────────┴─────────────────┴─────────────────────────┤
│   System Tools  │   Agent Tools   │   Task Tools            │
│   • Bash        │   • Agent       │   • TaskCreate          │
│   • PowerShell  │   • Skill       │   • TaskStop            │
│                 │                 │   • TodoWrite           │
└─────────────────────────────────────────────────────────────┘
```

---

## Tool Architecture

### Tool Base Class

Location: `src/Tool.ts` (29KB)

```typescript
export abstract class Tool {
  abstract name: string
  abstract description: string
  abstract inputSchema: z.ZodType

  // Tool capabilities
  isEnabled(): boolean { return true }
  isAvailable(): boolean { return true }

  // Execution
  abstract execute(params: unknown, context: ToolContext): Promise<ToolResult>

  // Permission handling
  getPermissionPrompt(params: unknown): string
  requiresApproval(): boolean
}

export type ToolResult = {
  content: ContentBlock[]
  isError?: boolean
}

export type ToolContext = {
  cwd: string
  signal: AbortSignal
  permissionContext: ToolPermissionContext
  // ...
}
```

### Tool Registry

Location: `src/tools.ts`

```typescript
export function getAllBaseTools(): Tools {
  return [
    AgentTool,
    TaskOutputTool,
    BashTool,
    // Fast search tools (disabled if ripgrep/bfs embedded)
    ...(hasEmbeddedSearchTools() ? [] : [GlobTool, GrepTool]),
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
    // Feature-gated tools
    ...(process.env.USER_TYPE === 'ant' ? [ConfigTool] : []),
    ...(process.env.USER_TYPE === 'ant' ? [TungstenTool] : []),
    // ...
  ]
}

export function getTools(permissionContext: ToolPermissionContext): Tools {
  const tools = getAllBaseTools()
  
  // Filter by deny rules
  let allowedTools = filterToolsByDenyRules(tools, permissionContext)
  
  // Filter by isEnabled()
  const isEnabled = allowedTools.map(_ => _.isEnabled())
  return allowedTools.filter((_, i) => isEnabled[i])
}
```

### Tool Pool Assembly

```typescript
export function assembleToolPool(
  permissionContext: ToolPermissionContext,
  mcpTools: Tools,
): Tools {
  const builtInTools = getTools(permissionContext)

  // Filter MCP tools by deny rules
  const allowedMcpTools = filterToolsByDenyRules(mcpTools, permissionContext)

  // Deduplicate (built-in tools take precedence)
  const byName = (a: Tool, b: Tool) => a.name.localeCompare(b.name)
  return uniqBy(
    [...builtInTools].sort(byName).concat(allowedMcpTools.sort(byName)),
    'name',
  )
}
```

---

## Core Tools

### BashTool

Location: `src/tools/BashTool/`

```typescript
export class BashTool extends Tool {
  name = 'Bash'
  description = 'Execute shell commands'

  inputSchema = z.object({
    command: z.string().describe('The bash command to execute'),
    description: z.string().describe('Why this command is needed'),
  })

  async execute(params: BashParams, context: ToolContext): Promise<BashResult> {
    const { command, description } = params

    // Permission check
    if (!await this.checkPermission(params, context)) {
      return { content: [{ type: 'text', text: 'Permission denied' }], isError: true }
    }

    // Execute command
    const result = await executeBashCommand(command, {
      cwd: context.cwd,
      signal: context.signal,
    })

    return {
      content: [
        { type: 'text', text: `Exit code: ${result.exitCode}` },
        { type: 'text', text: `Stdout: ${result.stdout}` },
        { type: 'text', text: `Stderr: ${result.stderr}` },
      ],
    }
  }
}
```

**Features:**
- Permission approval flow
- Timeout handling
- Output truncation
- ANSI code stripping
- Background task detection

---

### FileReadTool

Location: `src/tools/FileReadTool/`

```typescript
export class FileReadTool extends Tool {
  name = 'Read'
  description = 'Read file contents'

  inputSchema = z.object({
    path: z.string().describe('Path to the file to read'),
    offset: z.number().optional().describe('Line offset (0-indexed)'),
    limit: z.number().optional().describe('Max lines to read'),
  })

  async execute(params: ReadParams, context: ToolContext): Promise<ReadResult> {
    const { path, offset, limit } = params

    // Resolve and validate path
    const absolutePath = await this.resolvePath(path, context.cwd)

    // Read file (with caching)
    const content = await readFileWithCache(absolutePath, { offset, limit })

    return {
      content: [{ type: 'text', text: content }],
    }
  }
}
```

**Features:**
- Path validation (security)
- File caching
- Partial reads (offset/limit)
- Binary file detection

---

### FileEditTool

Location: `src/tools/FileEditTool/`

```typescript
export class FileEditTool extends Tool {
  name = 'Edit'
  description = 'Edit files with multi-strategy support'

  inputSchema = z.object({
    path: z.string().describe('Path to the file to edit'),
    old_string: z.string().describe('String to replace'),
    new_string: z.string().describe('Replacement string'),
  })

  async execute(params: EditParams, context: ToolContext): Promise<EditResult> {
    const { path, old_string, new_string } = params

    // Read current content
    const content = await readFile(path)

    // Apply edit strategy
    const newContent = await this.applyEdit(content, old_string, new_string)

    // Write result
    await writeFile(path, newContent)

    return {
      content: [{ type: 'text', text: 'File updated successfully' }],
    }
  }

  private async applyEdit(
    content: string,
    oldString: string,
    newString: string
  ): Promise<string> {
    // Strategy 1: Exact string match
    if (content.includes(oldString)) {
      return content.replace(oldString, newString)
    }

    // Strategy 2: Fuzzy match with line numbers
    // Strategy 3: Search/replace with context
    // Strategy 4: Full file rewrite suggestion
  }
}
```

**Features:**
- Multi-strategy editing
- Exact match
- Fuzzy match
- Context-based search
- Diff generation

---

### FileWriteTool

Location: `src/tools/FileWriteTool/`

```typescript
export class FileWriteTool extends Tool {
  name = 'Write'
  description = 'Create or overwrite files'

  inputSchema = z.object({
    path: z.string().describe('Path to write'),
    content: z.string().describe('File content'),
  })

  async execute(params: WriteParams, context: ToolContext): Promise<WriteResult> {
    const { path, content } = params

    // Validate path
    const absolutePath = await this.resolvePath(path, context.cwd)

    // Check if overwriting
    const exists = await fileExists(absolutePath)

    // Write file
    await writeFile(absolutePath, content)

    return {
      content: [{
        type: 'text',
        text: exists ? 'File overwritten' : 'File created',
      }],
    }
  }
}
```

---

### GrepTool

Location: `src/tools/GrepTool/`

```typescript
export class GrepTool extends Tool {
  name = 'Grep'
  description = 'Search file contents using ripgrep'

  inputSchema = z.object({
    pattern: z.string().describe('Regex pattern to search'),
    path: z.string().optional().describe('Directory to search'),
    glob: z.string().optional().describe('File glob filter'),
  })

  async execute(params: GrepParams, context: ToolContext): Promise<GrepResult> {
    const { pattern, path, glob } = params

    // Run ripgrep
    const results = await runRipgrep(pattern, {
      cwd: path || context.cwd,
      glob,
    })

    return {
      content: [{ type: 'text', text: formatGrepResults(results) }],
    }
  }
}
```

---

### GlobTool

Location: `src/tools/GlobTool/`

```typescript
export class GlobTool extends Tool {
  name = 'Glob'
  description = 'Find files by pattern'

  inputSchema = z.object({
    pattern: z.string().describe('Glob pattern (e.g., "**/*.ts")'),
    path: z.string().optional().describe('Directory to search'),
  })

  async execute(params: GlobParams, context: ToolContext): Promise<GlobResult> {
    const { pattern, path } = params

    // Run glob search
    const files = await runGlob(pattern, path || context.cwd)

    return {
      content: [{ type: 'text', text: files.join('\n') }],
    }
  }
}
```

---

### WebFetchTool

Location: `src/tools/WebFetchTool/`

```typescript
export class WebFetchTool extends Tool {
  name = 'WebFetch'
  description = 'Fetch content from a URL'

  inputSchema = z.object({
    url: z.string().url().describe('URL to fetch'),
  })

  async execute(params: FetchParams, context: ToolContext): Promise<FetchResult> {
    const { url } = params

    // Fetch URL
    const response = await fetch(url)
    const html = await response.text()

    // Convert to markdown
    const markdown = htmlToMarkdown(html)

    return {
      content: [{ type: 'text', text: markdown }],
    }
  }
}
```

---

### AgentTool

Location: `src/tools/AgentTool/`

```typescript
export class AgentTool extends Tool {
  name = 'Agent'
  description = 'Spawn a sub-agent for specialized tasks'

  inputSchema = z.object({
    agent: z.string().describe('Agent name'),
    prompt: z.string().describe('Task description'),
    tools: z.array(z.string()).optional().describe('Tools to grant'),
  })

  async execute(params: AgentParams, context: ToolContext): Promise<AgentResult> {
    const { agent, prompt, tools } = params

    // Load agent definition
    const agentDef = await loadAgent(agent)

    // Spawn sub-agent
    const result = await spawnSubAgent({
      agent: agentDef,
      prompt,
      tools: tools || agentDef.defaultTools,
      parentContext: context,
    })

    return {
      content: [{ type: 'text', text: result.output }],
    }
  }
}
```

---

## Tool Implementation

### Tool Registration Pattern

```typescript
// src/tools/BashTool/BashTool.ts
import { Tool } from '../../Tool.js'
import { z } from 'zod'

export class BashTool extends Tool {
  name = 'Bash'
  description = `Execute shell commands.

Supports:
- Running commands in the current directory
- Piping and redirection
- Background processes (with &)

Examples:
- \`ls -la\` - List files
- \`git status\` - Check git status
- \`npm install\` - Install dependencies`

  inputSchema = z.object({
    command: z.string(),
    description: z.string(),
  })

  async execute(params: BashParams, context: ToolContext): Promise<BashResult> {
    // Implementation
  }
}
```

### Feature-Gated Tools

```typescript
// In src/tools.ts
/* eslint-disable @typescript-eslint/no-require-imports */
const SleepTool = feature('PROACTIVE') || feature('KAIROS')
  ? require('./tools/SleepTool/SleepTool.js').SleepTool
  : null

const cronTools = feature('AGENT_TRIGGERS')
  ? [
      require('./tools/ScheduleCronTool/CronCreateTool.js').CronCreateTool,
      require('./tools/ScheduleCronTool/CronDeleteTool.js').CronDeleteTool,
      require('./tools/ScheduleCronTool/CronListTool.js').CronListTool,
    ]
  : []

const RemoteTriggerTool = feature('AGENT_TRIGGERS_REMOTE')
  ? require('./tools/RemoteTriggerTool/RemoteTriggerTool.js').RemoteTriggerTool
  : null
/* eslint-enable */

export function getAllBaseTools(): Tools {
  return [
    // ...
    ...(SleepTool ? [SleepTool] : []),
    ...cronTools,
    ...(RemoteTriggerTool ? [RemoteTriggerTool] : []),
  ]
}
```

---

## Permission System

### Permission Modes

Location: `src/utils/permissions/permissions.ts`

```typescript
export enum PermissionMode {
  READ_ONLY = 'read-only',
  WORKSPACE_WRITE = 'workspace-write',
  DANGER_FULL_ACCESS = 'danger-full-access',
}

export function getPermissionLevel(
  mode: PermissionMode,
  tool: Tool
): PermissionLevel {
  switch (mode) {
    case 'read-only':
      return isReadTool(tool) ? 'auto-approve' : 'deny'
    case 'workspace-write':
      if (isReadTool(tool)) return 'auto-approve'
      if (isWorkspaceTool(tool)) return 'require-approval'
      return 'deny'
    case 'danger-full-access':
      return isDangerousTool(tool) ? 'require-approval' : 'auto-approve'
  }
}
```

### Deny Rules

```typescript
// Filter tools by deny rules
export function filterToolsByDenyRules<T extends { name: string }>(
  tools: readonly T[],
  permissionContext: ToolPermissionContext
): T[] {
  return tools.filter(tool => {
    const denyRule = getDenyRuleForTool(permissionContext, tool)
    return !denyRule // Keep tool if no deny rule
  })
}

export function getDenyRuleForTool(
  context: ToolPermissionContext,
  tool: { name: string; mcpInfo?: { serverName: string } }
): DenyRule | undefined {
  // Check direct tool deny
  const directDeny = context.denyRules.find(
    rule => rule.toolName === tool.name && !rule.ruleContent
  )
  if (directDeny) return directDeny

  // Check MCP server deny
  if (tool.mcpInfo) {
    const serverDeny = context.denyRules.find(
      rule => rule.toolName === `mcp__${tool.mcpInfo.serverName}` && !rule.ruleContent
    )
    if (serverDeny) return serverDeny
  }
}
```

### Permission Request Flow

```typescript
// In REPL.tsx
async function handleToolCall(toolCall: ToolCall) {
  const tool = tools.find(t => t.name === toolCall.name)
  if (!tool) return

  // Check if approval needed
  const needsApproval = tool.requiresApproval() && !isAutoApproved(tool.name)

  if (needsApproval) {
    // Show permission dialog
    const approved = await showPermissionDialog({
      tool: tool.name,
      params: toolCall.input,
      prompt: tool.getPermissionPrompt(toolCall.input),
    })

    if (!approved) {
      return { content: [{ type: 'text', text: 'Permission denied' }], isError: true }
    }
  }

  // Execute tool
  const result = await tool.execute(toolCall.input, context)
  return result
}
```

---

## MCP Tool Integration

### MCP Tool Loading

Location: `src/services/mcp/`

```typescript
// Load MCP tools from connected servers
export async function loadMcpTools(
  mcpClients: MCPServerConnection[]
): Promise<Tools> {
  const tools: Tools = []

  for (const client of mcpClients) {
    const serverTools = await client.listTools()

    for (const serverTool of serverTools) {
      tools.push({
        name: `${client.name}__${serverTool.name}`,
        description: serverTool.description,
        inputSchema: serverTool.inputSchema,
        mcpInfo: {
          serverName: client.name,
          toolName: serverTool.name,
        },
        execute: async (params, context) => {
          return client.callTool(serverTool.name, params)
        },
      })
    }
  }

  return tools
}
```

### MCP Tool Naming

```typescript
// MCP tools are namespaced by server
{
  name: 'filesystem__read_file',
  mcpInfo: {
    serverName: 'filesystem',
    toolName: 'read_file',
  },
}

// Deny rules can target:
// - Specific tool: mcp__filesystem__read_file
// - All tools from server: mcp__filesystem
```

---

## Tool Execution Flow

### Complete Flow

```
1. Model generates tool_call
   │
   ▼
2. Parse tool_call from stream
   │
   ▼
3. Lookup tool in registry
   │
   ├── Tool not found → Error
   │
   ▼
4. Validate input schema (Zod)
   │
   ├── Invalid → Error
   │
   ▼
5. Check permission mode
   │
   ├── Denied → Return error
   │
   ▼
6. Check if approval needed
   │
   ├── Yes → Show dialog
   │   │
   │   ├── User denies → Error
   │   └── User approves → Continue
   │
   ▼
7. Execute tool
   │
   ├── Timeout handling
   ├── Signal handling (Ctrl+C)
   └── Error catching
   │
   ▼
8. Format result
   │
   ├── Truncate large output
   ├── Strip sensitive data
   └── Convert to ContentBlock[]
   │
   ▼
9. Send result to API
   │
   ▼
10. Model continues with tool_result
```

### Example Flow

```typescript
// Simplified execution flow
async function executeToolCall(
  toolCall: ToolCall,
  tools: Tools,
  context: ToolContext
): Promise<ToolResult> {
  // 1. Find tool
  const tool = tools.find(t => t.name === toolCall.name)
  if (!tool) {
    return {
      content: [{ type: 'text', text: `Unknown tool: ${toolCall.name}` }],
      isError: true,
    }
  }

  // 2. Validate input
  const parseResult = tool.inputSchema.safeParse(toolCall.input)
  if (!parseResult.success) {
    return {
      content: [{ type: 'text', text: `Invalid input: ${parseResult.error.message}` }],
      isError: true,
    }
  }

  // 3. Check permissions
  const permissionLevel = getPermissionLevel(
    context.permissionMode,
    tool
  )

  if (permissionLevel === 'deny') {
    return {
      content: [{ type: 'text', text: 'Tool denied by permission mode' }],
      isError: true,
    }
  }

  // 4. Request approval if needed
  if (permissionLevel === 'require-approval') {
    const approved = await requestApproval(tool, parseResult.data)
    if (!approved) {
      return {
        content: [{ type: 'text', text: 'Approval denied by user' }],
        isError: true,
      }
    }
  }

  // 5. Execute
  try {
    const result = await tool.execute(parseResult.data, context)
    return result
  } catch (error) {
    return {
      content: [{ type: 'text', text: `Error: ${error.message}` }],
      isError: true,
    }
  }
}
```

---

## Tool Reference

### Built-in Tools

| Tool | Purpose | Permission Level |
|------|---------|------------------|
| Bash | Execute shell commands | Write/Danger |
| Read | Read files | Read |
| Edit | Edit files | Write |
| Write | Create/overwrite files | Write |
| Glob | Find files by pattern | Read |
| Grep | Search file contents | Read |
| WebFetch | Fetch web pages | Network |
| WebSearch | Search the web | Network |
| Agent | Spawn sub-agents | Special |
| Skill | Invoke skills | Special |
| TodoWrite | Manage todos | Write |
| TaskCreate | Create tasks | Special |
| TaskStop | Stop tasks | Special |
| TaskList | List tasks | Read |
| TaskGet | Get task details | Read |
| TaskUpdate | Update task | Write |
| TaskOutput | Get task output | Read |
| NotebookEdit | Edit Jupyter notebooks | Write |
| LSPTool | Language server operations | Read |
| MCPTool | Invoke MCP tools | Special |
| ListMcpResources | List MCP resources | Read |
| ReadMcpResource | Read MCP resource | Read |
| ConfigTool | View/modify config | Write |
| TungstenTool | Internal tool (ant-only) | Special |
| EnterPlanMode | Enter plan mode | Special |
| ExitPlanMode | Exit plan mode | Special |
| EnterWorktree | Enter worktree | Special |
| ExitWorktree | Exit worktree | Special |
| AskUserQuestion | Ask user for input | Read |
| ToolSearchTool | Search available tools | Read |
| BriefTool | Brief mode operations | Special |

### Feature-Gated Tools

| Tool | Flag | Purpose |
|------|------|---------|
| SleepTool | PROACTIVE/KAIROS | Sleep/delay execution |
| CronCreateTool | AGENT_TRIGGERS | Create scheduled tasks |
| CronDeleteTool | AGENT_TRIGGERS | Delete scheduled tasks |
| CronListTool | AGENT_TRIGGERS | List scheduled tasks |
| RemoteTriggerTool | AGENT_TRIGGERS_REMOTE | Remote triggers |
| MonitorTool | MONITOR_TOOL | System monitoring |
| SendUserFileTool | KAIROS | Send files to user |
| PushNotificationTool | KAIROS | Push notifications |
| SubscribePRTool | KAIROS_GITHUB_WEBHOOKS | GitHub PR webhooks |
| TestingPermissionTool | test only | Test permissions |
| OverflowTestTool | OVERFLOW_TEST_TOOL | Test overflow |
| CtxInspectTool | CONTEXT_COLLAPSE | Inspect context |
| TerminalCaptureTool | TERMINAL_PANEL | Terminal capture |
| WebBrowserTool | WEB_BROWSER_TOOL | Web browsing |
| WorkflowTool | WORKFLOW_SCRIPTS | Workflow execution |
| VerifyPlanExecutionTool | CLAUDE_CODE_VERIFY_PLAN | Plan verification |

---

## References

- [src/tools.ts](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/src/tools.ts) — Tool registry
- [src/Tool.ts](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/src/Tool.ts) — Tool base class
- [src/tools/](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/src/tools/) — Tool implementations
- [src/utils/permissions/](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/src/utils/permissions/) — Permission system
