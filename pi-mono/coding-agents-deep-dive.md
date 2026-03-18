---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Pi/pi-mono
explored_at: 2026-03-18
---

# Coding Agents - Deep Dive

## Overview

Pi's coding agent capabilities come from its tool system, skills, prompt templates, and extensions. The default agent has four tools (`read`, `write`, `edit`, `bash`), but this can be extended or replaced entirely.

## Built-in Tools

### Tool Types

```typescript
type Tool = AgentTool<any>;

interface AgentTool<T> {
  name: string;
  label?: string;
  description: string;
  parameters: Static<T>;  // TypeBox schema
  execute: (
    toolCallId: string,
    input: T,
    onUpdate: (event: ToolEvent) => void,
    ctx: ToolContext,
    signal: AbortSignal,
  ) => Promise<ToolResult>;
}
```

### Default Tool Set

```typescript
// packages/coding-agent/src/core/tools/index.ts

// Full coding tools (default)
export const codingTools: Tool[] = [readTool, bashTool, editTool, writeTool];

// Read-only exploration tools
export const readOnlyTools: Tool[] = [readTool, grepTool, findTool, lsTool];

// All available tools
export const allTools = {
  read: readTool,
  bash: bashTool,
  edit: editTool,
  write: writeTool,
  grep: grepTool,
  find: findTool,
  ls: lsTool,
};
```

### Tool Implementations

#### Read Tool

```typescript
interface ReadToolInput {
  path: string;
  offset?: number;    // Line number to start
  limit?: number;     // Max lines to read
}

interface ReadToolOptions {
  maxBytes?: number;   // Default: 100KB
  maxLines?: number;   // Default: 1000
}

// Features:
// - Partial file reading (offset/limit)
// - Binary file detection
// - Truncation with head/tail options
```

#### Edit Tool

```typescript
interface EditToolInput {
  path: string;
  oldText: string;    // Text to find
  newText: string;    // Replacement text
}

// Features:
// - Multiple occurrences handling
// - Whitespace normalization
// - Error on no match found
```

#### Write Tool

```typescript
interface WriteToolInput {
  path: string;
  content: string;
  append?: boolean;   // Append vs overwrite
}

// Features:
// - Directory creation
// - Atomic writes (write to temp, rename)
```

#### Bash Tool

```typescript
interface BashToolInput {
  command: string;
  cwd?: string;
}

interface BashToolOptions {
  timeout?: number;      // Default: 300s
  maxOutputBytes?: number;  // Default: 1MB
}

// Features:
// - Streaming output
// - Timeout protection
// - Output truncation
// - Working directory control
```

#### Grep Tool

```typescript
interface GrepToolInput {
  pattern: string;
  path?: string;        // Directory to search
  include?: string;     // Glob pattern
  exclude?: string;
  maxMatches?: number;
}
```

#### Find Tool

```typescript
interface FindToolInput {
  path?: string;
  name?: string;        // Name pattern
  type?: "f" | "d";     // File or directory
  maxResults?: number;
}
```

#### LS Tool

```typescript
interface LsToolInput {
  path?: string;
  all?: boolean;        // Include hidden files
  directory?: boolean;  // List directory itself, not contents
}
```

### Tool Factories (Custom cwd)

```typescript
// Pre-built tools use process.cwd()
import { readTool, bashTool } from "@mariozechner/pi-coding-agent";

// For custom cwd, use factories
import { createReadTool, createBashTool, createCodingTools } from "@mariozechner/pi-coding-agent";

const cwd = "/path/to/project";
const tools = createCodingTools(cwd);
// or
const tools = [createReadTool(cwd), createBashTool(cwd)];
```

### Tool Execution Modes

```typescript
type ToolExecutionMode = "parallel" | "sequential";

// Parallel (default) - execute independent tool calls concurrently
agent.setToolExecution("parallel");

// Sequential - execute tool calls one at a time
agent.setToolExecution("sequential");
```

### Tool Hooks

```typescript
interface AgentOptions {
  // Called after argument validation, before execution
  beforeToolCall?: (
    context: BeforeToolCallContext,
    signal?: AbortSignal,
  ) => Promise<BeforeToolCallResult | undefined>;

  // Called after execution, before events emitted
  afterToolCall?: (
    context: AfterToolCallContext,
    signal?: AbortSignal,
  ) => Promise<AfterToolCallResult | undefined>;
}

interface BeforeToolCallContext {
  toolCallId: string;
  toolName: string;
  input: unknown;
}

interface BeforeToolCallResult {
  modifyInput?: unknown;
  skip?: boolean;
  skipReason?: string;
}
```

## Skills System

### Overview

Skills follow the [Agent Skills standard](https://agentskills.io) - on-demand capability packages invoked via `/skill:name` or auto-loaded by the agent.

### Skill Structure

```
skills/
└── my-skill/
    └── SKILL.md
```

```markdown
# My Skill

Use this skill when the user asks about X.

## Steps
1. Do this
2. Then that

## Examples
- Example usage
```

### Skill Discovery

```typescript
// packages/coding-agent/src/core/resource-loader.ts
// Skills discovered from:
// - ~/.pi/agent/skills/
// - ~/.agents/skills/
// - .pi/skills/ (project)
// - .agents/skills/ (project, parent dirs up to git root)
// - Configured sources (settings.json)
```

### Skill Loading

```typescript
interface Skill {
  name: string;
  description: string;
  filePath: string;
  baseDir: string;
  source: "global" | "project" | "parent" | "custom";
  content?: string;  // Loaded SKILL.md content
}
```

### Skill Invocation

```bash
# Explicit invocation
/skill:my-skill

# Agent auto-loads based on context
```

### Skill Block Parsing

```typescript
// Parse skill content for structured blocks
interface ParsedSkillBlock {
  type: "steps" | "examples" | "notes";
  content: string;
}

function parseSkillBlock(content: string): ParsedSkillBlock[] {
  // Parse markdown sections
}
```

## Prompt Templates

### Overview

Reusable prompts as Markdown files, expanded via `/name` command.

### File Structure

```markdown
<!-- ~/.pi/agent/prompts/review.md -->
# Code Review

Review this code for:
- Bugs
- Security issues
- Performance problems

Focus on: {{focus}}
```

### Variables

```typescript
// {{variable}} - replaced with user-provided value
// {{cwd}} - current working directory
// {{date}} - current date
```

### Discovery

```typescript
// Discovered from:
// - ~/.pi/agent/prompts/
// - .pi/prompts/ (project)
// - Configured sources
```

### Usage

```bash
# Expand and send
/review focus="error handling"

# Via SDK
await session.prompt("/review", {
  variables: { focus: "error handling" },
});
```

## Extensions

### Overview

TypeScript modules that extend Pi with custom tools, commands, shortcuts, event handlers, and UI components.

### Extension Factory

```typescript
// my-extension.ts
export default function(pi: ExtensionAPI) {
  // Register tools
  pi.registerTool({
    name: "deploy",
    label: "Deploy",
    description: "Deploy the application",
    parameters: Type.Object({
      environment: Type.String({ enum: ["staging", "production"] }),
    }),
    execute: async (toolCallId, params, onUpdate, ctx, signal) => {
      // Tool implementation
      return {
        content: [{ type: "text", text: "Deployed!" }],
        details: {},
      };
    },
  });

  // Register commands
  pi.registerCommand("stats", {
    description: "Show statistics",
    handler: async (ctx) => {
      // Command implementation
    },
  });

  // Subscribe to events
  pi.on("tool_call", async (event, ctx) => {
    console.log(`Tool called: ${event.toolName}`);
  });

  // Register shortcuts
  pi.registerShortcut("ctrl-shift-s", {
    description: "Show stats",
    handler: (ctx) => {
      // Shortcut handler
    },
  });
}
```

### Extension API

```typescript
interface ExtensionAPI {
  // Registration
  on(event: string, handler: HandlerFn): void;
  registerTool(tool: ToolDefinition): void;
  registerCommand(name: string, options: RegisteredCommand): void;
  registerShortcut(shortcut: KeyId, options: ShortcutOptions): void;
  registerFlag(name: string, options: FlagOptions): void;
  registerMessageRenderer<T>(type: string, renderer: MessageRenderer<T>): void;

  // Action methods
  sendMessage(message: AgentMessage, options?: SendMessageOptions): void;
  sendUserMessage(content: Content, options?: SendMessageOptions): void;
  appendEntry(customType: string, data?: unknown): void;
  setSessionName(name: string): void;
  getSessionName(): string | undefined;
  setLabel(entryId: string, label: string | undefined): void;
  getActiveTools(): string[];
  getAllTools(): ToolDefinition[];
  setActiveTools(toolNames: string[]): void;
  getCommands(): RegisteredCommand[];
  setModel(model: Model): Promise<void>;
  getThinkingLevel(): ThinkingLevel;
  setThinkingLevel(level: ThinkingLevel): void;

  // Provider registration (dynamic)
  registerProvider(name: string, config: ProviderConfig): void;
  unregisterProvider(name: string): void;

  // Events
  events: EventBus;

  // Utilities
  exec(command: string, args: string[], options?: ExecOptions): Promise<ExecResult>;
  getFlag(name: string): boolean | string | undefined;
}
```

### Extension Events

```typescript
// Available events
- "agent_start"
- "agent_end"
- "turn_start"
- "turn_end"
- "message_start"
- "message_end"
- "tool_call"
- "tool_result"
- "compaction_start"
- "compaction_end"
```

### Extension Handlers

```typescript
// Advanced event handlers with context
pi.on("before_provider_request", async (payload, ctx) => {
  // Inspect/modify provider payload
  return modifiedPayload;
});

pi.on("context", async (messages, ctx) => {
  // Modify context before LLM call
  return modifiedMessages;
});
```

### Extension Loading

```typescript
// packages/coding-agent/src/core/extensions/loader.ts
// Uses @mariozechner/jiti for TypeScript loading
// Virtual modules for Bun binary compatibility

const jiti = createJiti(import.meta.url, {
  moduleCache: false,
  virtualModules: {
    "@mariozechner/pi-agent-core": piAgentCore,
    "@mariozechner/pi-ai": piAi,
    // ...
  },
});

const factory = await jiti.import(extensionPath);
await factory(api);
```

### Extension Discovery

```typescript
// Discovered from:
// - ~/.pi/agent/extensions/
// - .pi/extensions/ (project)
// - Configured sources (settings.json, CLI --extension)
```

### Extension Package Format

```json
{
  "name": "my-pi-extension",
  "keywords": ["pi-package"],
  "pi": {
    "extensions": ["./dist/extension.js"]
  }
}
```

### Extension UI Components

```typescript
// Extensions can add UI components in interactive mode
pi.on("ui_mount", (ctx: ExtensionUIContext) => {
  // Add widgets above/below editor
  ctx.addWidget({
    position: "above-editor",
    component: myWidgetComponent,
  });

  // Add custom footer
  ctx.setFooter(myFooterComponent);

  // Replace header
  ctx.setHeader(myHeaderComponent);

  // Replace editor
  ctx.setEditor(myEditorComponent);
});
```

### Extension Commands

```typescript
// Slash commands from extensions
/register-command  # Extension registered

// Commands appear in /help and autocomplete
```

### Extension Tools

```typescript
// Tools from extensions
{
  "tools": [
    {
      "name": "extension-tool",
      "source": "extension:my-extension"
    }
  ]
}
```

## Event Bus

### Overview

Extensions communicate via a shared event bus:

```typescript
import { createEventBus } from "@mariozechner/pi-coding-agent";

const eventBus = createEventBus();

// Emit events
eventBus.emit("my-event", { data: "value" });

// Listen for events
eventBus.on("my-event", (data) => {
  console.log("Received:", data);
});

// Pass to loader
const loader = new DefaultResourceLoader({ eventBus });
```

## ResourceLoader

### Overview

The `ResourceLoader` supplies extensions, skills, prompt templates, themes, and context files to the agent.

### DefaultResourceLoader

```typescript
class DefaultResourceLoader {
  constructor(options: {
    cwd?: string;
    agentDir?: string;
    settingsManager?: SettingsManager;
    additionalExtensionPaths?: string[];
    additionalSkillPaths?: string[];
    additionalPromptTemplatePaths?: string[];
    additionalThemePaths?: string[];
    noExtensions?: boolean;
    noSkills?: boolean;
    noPromptTemplates?: boolean;
    noThemes?: boolean;
    systemPrompt?: string;
    appendSystemPrompt?: string;
    eventBus?: EventBus;
  });

  async reload(): Promise<void>;

  getExtensions(): LoadExtensionsResult;
  getSkills(): { skills: Skill[] };
  getPrompts(): { prompts: PromptTemplate[] };
  getThemes(): { themes: Theme[] };
  getAgentsFiles(): { agentsFiles: AgentsFile[] };
}
```

### Override Hooks

```typescript
const loader = new DefaultResourceLoader({
  skillsOverride: (current) => ({
    skills: [...current.skills, customSkill],
    diagnostics: current.diagnostics,
  }),
  promptsOverride: (current) => ({
    prompts: [...current.prompts, customPrompt],
  }),
  agentsFilesOverride: (current) => ({
    agentsFiles: [...current.agentsFiles, customFile],
  }),
  systemPromptOverride: () => "Custom system prompt",
});
```

## Pi Packages

### Overview

Bundle and share extensions, skills, prompts, and themes via npm or git.

### Installation

```bash
# npm packages
pi install npm:@foo/pi-tools
pi install npm:@foo/pi-tools@1.2.3  # Pinned version

# Git packages
pi install git:github.com/user/repo
pi install git:github.com/user/repo@v1  # Tag/commit
pi install git:git@github.com:user/repo
pi install ssh://git@github.com/user/repo
pi install https://github.com/user/repo

# Local packages
pi install ./local/path

# Project-local (-l flag)
pi install -l npm:@foo/pi-tools
```

### Package Structure

```json
{
  "name": "my-pi-package",
  "keywords": ["pi-package"],
  "pi": {
    "extensions": ["./extensions"],
    "skills": ["./skills"],
    "prompts": ["./prompts"],
    "themes": ["./themes"]
  }
}
```

### Package Management

```bash
pi list       # List installed packages
pi update     # Update packages (skips pinned)
pi update foo # Update specific package
pi remove npm:@foo/pi-tools
pi uninstall npm:@foo/pi-tools  # Alias
pi config     # Enable/disable resources
```

### Package Locations

```
~/.pi/agent/
├── git/         # Git-installed packages
├── npm/         # npm-installed packages
├── extensions/  # Unpacked extensions
└── packages/    # Resolved package contents
```

## Tool Filtering (Settings)

```json
{
  "tools": {
    "enabled": ["read", "bash", "edit", "write"],
    "disabled": ["grep", "find"]
  }
}
```

## Examples

### Custom Tool

```typescript
import { Type } from "@sinclair/typebox";
import { createAgentSession, type ToolDefinition } from "@mariozechner/pi-coding-agent";

const statusTool: ToolDefinition = {
  name: "status",
  label: "Status",
  description: "Get system status",
  parameters: Type.Object({}),
  execute: async () => ({
    content: [{ type: "text", text: `Uptime: ${process.uptime()}s` }],
    details: {},
  }),
};

const { session } = await createAgentSession({
  customTools: [statusTool],
});
```

### Inline Extension

```typescript
const loader = new DefaultResourceLoader({
  extensionFactories: [
    (pi) => {
      pi.on("agent_start", () => {
        console.log("[Extension] Agent starting");
      });
      pi.registerTool(myTool);
    },
  ],
});
await loader.reload();

const { session } = await createAgentSession({
  resourceLoader: loader,
});
```
