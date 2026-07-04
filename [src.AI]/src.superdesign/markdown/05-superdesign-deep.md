# SuperDesign Deep Dive: Extension, Agent, Tools, Webview UI — Source-Level

Comprehensive source-level documentation of every file in the SuperDesign VS Code extension.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.superdesign/superdesign/src/`

## Source Map

| File | Lines | Purpose |
|------|-------|---------|
| `extension.ts` | 1,888 | VS Code extension entry point |
| `services/customAgentService.ts` | 927 | AI agent orchestration |
| `services/chatMessageService.ts` | 456 | Chat message management |
| `services/claudeCodeService.ts` | 453 | Claude Code integration |
| `services/logger.ts` | 79 | Logging service |
| `providers/chatSidebarProvider.ts` | 339 | Sidebar panel provider |
| `templates/webviewTemplate.ts` | 80 | Webview HTML template |
| `tools/bash-tool.ts` | 262 | Shell command execution |
| `tools/edit-tool.ts` | 196 | File editing |
| `tools/glob-tool.ts` | 293 | File pattern matching |
| `tools/grep-tool.ts` | 314 | Text search |
| `tools/ls-tool.ts` | 226 | Directory listing |
| `tools/multiedit-tool.ts` | 182 | Multi-file editing |
| `tools/read-tool.ts` | 300 | File reading |
| `tools/write-tool.ts` | 100 | File writing |
| `tools/theme-tool.ts` | 89 | Theme management |
| `tools/tool-utils.ts` | 180 | Shared tool utilities |
| `types/agent.ts` | 21 | Agent type definitions |
| `types/context.ts` | 45 | Context type definitions |
| `webview/App.tsx` | 112 | React root component |
| `webview/index.tsx` | 42 | Webview entry point |
| `webview/components/CanvasView.tsx` | 859 | Design canvas |
| `webview/components/DesignFrame.tsx` | 837 | Design frame |
| `webview/components/Chat/ChatInterface.tsx` | 1,516 | Main chat UI |
| `webview/components/Chat/ModelSelector.tsx` | 436 | Model selection |
| `webview/components/Chat/ThemePreview.tsx` | 579 | Theme preview |
| `webview/components/Chat/ThemePreviewCard.tsx` | 418 | Theme preview card |
| `webview/components/Chat/ThemeDetails.tsx` | 164 | Theme details |
| `webview/components/Chat/ColorPalette.tsx` | 131 | Color palette |
| `webview/components/Chat/ModeToggle.tsx` | 56 | Mode toggle |
| `webview/components/Welcome/Welcome.tsx` | 110 | Onboarding |
| `webview/hooks/useChat.ts` | 377 | Chat state management |
| `webview/hooks/useFirstTimeUser.ts` | 54 | First-time user detection |
| `webview/utils/gridLayout.ts` | 465 | Canvas grid calculations |
| `webview/utils/themeParser.ts` | 355 | Theme parsing |
| `webview/types/canvas.types.ts` | 178 | Canvas type definitions |

---

## extension.ts: VS Code Extension Entry Point

**Source:** `src/extension.ts` (1,888 lines)

### Activation

The extension activates when:
- The `SuperDesign` command is executed
- The sidebar panel is opened

### Key Services Initialized

1. **Logger** — Output channel for extension logs
2. **ChatSidebarProvider** — Sidebar panel handler
3. **CustomAgentService** — AI agent orchestration
4. **ClaudeCodeService** — Claude Code integration
5. **ChatMessageService** — Message history management

### Registered Commands

| Command | Purpose |
|---------|---------|
| `superdesign.open` | Open the sidebar panel |
| `superdesign.configureAnthropicKey` | Configure Anthropic API key |
| `superdesign.configureOpenAIKey` | Configure OpenAI API key |
| `superdesign.configureOpenRouterKey` | Configure OpenRouter API key |
| `superdesign.configureGoogleKey` | Configure Google AI API key |
| `superdesign.clearConversation` | Clear chat history |
| `superdesign.exportDesign` | Export current design |

### Webview Communication

The extension handles `postMessage` events from the webview:
- `sendMessage` — User sends a chat message
- `clearChat` — Clear conversation
- `uploadImage` — Upload image to moodboard
- `saveImageToMoodboard` — Save image to `.superdesign/moodboard/`
- `getBase64Image` — Convert image to base64 for AI SDK

### Moodboard

Images uploaded by the AI are saved to `.superdesign/moodboard/` in the workspace root:

```typescript
async function saveImageToMoodboard(data: {
  fileName: string;
  originalName: string;
  base64Data: string;
  mimeType: string;
  size: number;
}, sidebarProvider: ChatSidebarProvider) {
  const moodboardDir = vscode.Uri.joinPath(workspaceFolder.uri, '.superdesign', 'moodboard');
  // Create directory if needed
  // Write base64 content to file
  // Notify webview of saved image
}
```

---

## customAgentService.ts: AI Agent Orchestration

**Source:** `src/services/customAgentService.ts` (927 lines)

### AgentService Interface

```typescript
// src/types/agent.ts
export interface AgentService {
    query(
        prompt?: string,
        messages?: CoreMessage[],
        options?: any,
        abortController?: AbortController,
        onMessage?: (message: any) => void
    ): Promise<any[]>;
    hasApiKey(): boolean;
    isApiKeyAuthError(errorMessage: string): boolean;
}
```

### Provider Configuration

```typescript
// src/services/customAgentService.ts:80-163
private getModel() {
    const config = vscode.workspace.getConfiguration('superdesign');
    const provider = config.get<string>('aiModelProvider', 'anthropic');
    
    switch (effectiveProvider) {
        case 'openrouter':
            const openrouter = createOpenRouter({ apiKey: openrouterKey });
            return openrouter.chat(openrouterModel);
        case 'anthropic':
            const anthropic = createAnthropic({
                apiKey: anthropicKey,
                baseURL: "https://anthropic.helicone.ai/v1",
                headers: { "Helicone-Auth": "Bearer sk-helicone-..." },
            });
            return anthropic(anthropicModel);
        case 'openai':
            const openai = createOpenAI({
                apiKey: openaiKey,
                baseURL: "https://oai.helicone.ai/v1",
                headers: { "Helicone-Auth": "Bearer sk-helicone-..." },
            });
            return openai(openaiModel);
    }
}
```

**Aha:** Anthropic and OpenAI providers use Helicone as a proxy for observability. OpenRouter connects directly.

### Tool Registration

The service creates all 9 tools and registers them with the Vercel AI SDK:

```typescript
private getTools() {
    return {
        read_file: createReadTool(this.executionContext),
        write_file: createWriteTool(this.executionContext),
        edit_file: createEditTool(this.executionContext),
        bash: createBashTool(this.executionContext),
        glob: createGlobTool(this.executionContext),
        grep: createGrepTool(this.executionContext),
        ls: createLsTool(this.executionContext),
        multiedit: createMultieditTool(this.executionContext),
        theme: createThemeTool(this.executionContext),
    };
}
```

### Query Method

The `query()` method:
1. Builds the system prompt based on mode (mock, component, wireframe)
2. Combines with conversation history
3. Calls `streamText()` from Vercel AI SDK with tools
4. Streams response chunks to the webview via `onMessage` callback
5. Handles tool execution automatically (AI SDK does this)
6. Returns accumulated tool results

---

## Tools

All tools use Zod schemas for validation and share utilities from `tool-utils.ts`.

### tool-utils.ts: Shared Utilities

**Source:** `src/tools/tool-utils.ts` (180 lines)

#### Path Validation

```typescript
function validateWorkspacePath(path: string, context: ExecutionContext): string;
function resolveWorkspacePath(path: string, context: ExecutionContext): string;
```

Ensures paths don't escape the workspace directory (prevents directory traversal attacks).

#### Response Format

```typescript
interface ToolResponse {
  success: boolean;
  content: string;
  metadata?: Record<string, unknown>;
  error?: string;
}

function createSuccessResponse(content: string, metadata?: Record<string, unknown>): ToolResponse;
function createErrorResponse(error: string): ToolResponse;
```

### read-tool.ts: File Reading

**Source:** `src/tools/read-tool.ts` (300 lines)

```typescript
export function createReadTool(context: ExecutionContext) {
    return tool({
        description: 'Read file contents',
        parameters: z.object({
            path: z.string().describe('File path relative to workspace'),
            maxLines: z.number().optional().describe('Maximum lines to read'),
            startLine: z.number().optional().describe('Starting line number'),
        }),
        execute: async ({ path, maxLines, startLine }) => {
            // Validate path against workspace
            // Check file size (max 10MB)
            // Detect file type (text/image/pdf/binary)
            // For text: read with line limits
            // For images: return base64
            // For binary: return metadata only
        },
    });
}
```

#### File Type Detection

```typescript
function detectFileType(filePath: string): 'text' | 'image' | 'pdf' | 'binary' {
    const ext = path.extname(filePath).toLowerCase();
    const mimeType = mime.lookup(filePath);
    if (mimeType?.startsWith('image/')) return 'image';
    if (mimeType === 'application/pdf') return 'pdf';
    // Check for null bytes and non-printable characters
    if (isBinaryFile(filePath)) return 'binary';
    return 'text';
}
```

#### Binary Detection

```typescript
function isBinaryFile(filePath: string): boolean {
    // Sample first 4KB
    // If null bytes found → binary
    // If >30% non-printable chars → binary
}
```

**Constants:**
- `DEFAULT_MAX_LINES = 1000`
- `MAX_LINE_LENGTH = 2000`
- `MAX_FILE_SIZE_BYTES = 10 * 1024 * 1024` (10MB)

### write-tool.ts: File Writing

**Source:** `src/tools/write-tool.ts` (100 lines)

```typescript
export function createWriteTool(context: ExecutionContext) {
    return tool({
        description: 'Write content to a file',
        parameters: z.object({
            path: z.string().describe('File path relative to workspace'),
            content: z.string().describe('File content'),
            overwrite: z.boolean().optional().describe('Overwrite existing file'),
        }),
        execute: async ({ path, content, overwrite }) => {
            // Validate path
            // Check if file exists (require overwrite flag)
            // Create parent directories if needed
            // Write content
        },
    });
}
```

### edit-tool.ts: File Editing

**Source:** `src/tools/edit-tool.ts` (196 lines)

Search-and-replace editing:

```typescript
parameters: z.object({
    path: z.string(),
    oldText: z.string().describe('Text to find'),
    newText: z.string().describe('Replacement text'),
    replaceAll: z.boolean().optional().describe('Replace all occurrences'),
})
```

### multiedit-tool.ts: Multi-File Editing

**Source:** `src/tools/multiedit-tool.ts` (182 lines)

Edit multiple files in a single operation:

```typescript
parameters: z.object({
    edits: z.array(z.object({
        path: z.string(),
        oldText: z.string(),
        newText: z.string(),
    })),
})
```

### bash-tool.ts: Shell Commands

**Source:** `src/tools/bash-tool.ts` (262 lines)

Executes shell commands via `execa`:

```typescript
parameters: z.object({
    command: z.string().describe('Shell command to execute'),
    workingDirectory: z.string().optional().describe('Override working directory'),
    timeout: z.number().optional().describe('Timeout in milliseconds'),
})
```

**Safety measures:**
- Path validation
- Timeout limits
- Output size limits
- Blocked commands list

### glob-tool.ts: File Pattern Matching

**Source:** `src/tools/glob-tool.ts` (293 lines)

Finds files matching a glob pattern using the `glob` package:

```typescript
parameters: z.object({
    pattern: z.string().describe('Glob pattern (e.g., "**/*.ts")'),
    exclude: z.array(z.string()).optional().describe('Patterns to exclude'),
})
```

### grep-tool.ts: Text Search

**Source:** `src/tools/grep-tool.ts` (314 lines)

Searches for text patterns in files:

```typescript
parameters: z.object({
    pattern: z.string().describe('Search pattern (regex)'),
    path: z.string().optional().describe('Directory to search in'),
    include: z.string().optional().describe('File pattern to include'),
    exclude: z.string().optional().describe('File pattern to exclude'),
    caseSensitive: z.boolean().optional().describe('Case sensitive search'),
    maxResults: z.number().optional().describe('Maximum results to return'),
})
```

### ls-tool.ts: Directory Listing

**Source:** `src/tools/ls-tool.ts` (226 lines)

Lists directory contents:

```typescript
parameters: z.object({
    path: z.string().optional().describe('Directory to list'),
    recursive: z.boolean().optional().describe('List recursively'),
    maxDepth: z.number().optional().describe('Maximum recursion depth'),
})
```

### theme-tool.ts: Theme Management

**Source:** `src/tools/theme-tool.ts` (89 lines)

Manages design themes:

```typescript
parameters: z.object({
    action: z.enum(['list', 'create', 'update', 'delete']),
    name: z.string().optional(),
    theme: z.object({...}).optional(),
})
```

---

## Webview UI

### ChatInterface.tsx: Main Chat UI

**Source:** `src/webview/components/Chat/ChatInterface.tsx` (1,516 lines)

The main chat component handling:
- Message display (user/assistant)
- Input field with send button
- Model selector
- Mode toggle (Mock/Component/Wireframe)
- Theme preview integration
- Color palette
- Markdown rendering for assistant responses
- Tool execution visualization
- Loading states and error handling

### ModelSelector.tsx

**Source:** `src/webview/components/Chat/ModelSelector.tsx` (436 lines)

Model selection UI supporting:
- Anthropic (Claude 3.5 Sonnet, Claude 3.7 Sonnet)
- OpenAI (GPT-4o, GPT-4o mini)
- OpenRouter (multi-model routing)
- Google (Gemini models)

### CanvasView.tsx

**Source:** `src/webview/components/CanvasView.tsx` (859 lines)

Design canvas with:
- Pan and zoom via `react-zoom-pan-pinch`
- Grid background
- Design frame rendering
- Connection lines between elements
- Responsive layout

### DesignFrame.tsx

**Source:** `src/webview/components/DesignFrame.tsx` (837 lines)

Individual design frame showing:
- Generated HTML/CSS preview
- Device frame (desktop/tablet/mobile)
- Interaction controls
- Export options

### ThemePreview.tsx / ThemePreviewCard.tsx

**Source:** `src/webview/components/Chat/ThemePreview.tsx` (579 lines)
**Source:** `src/webview/components/Chat/ThemePreviewCard.tsx` (418 lines)

Theme preview components showing:
- Color scheme preview
- Typography preview
- Component previews
- Theme comparison

### useChat Hook

**Source:** `src/webview/hooks/useChat.ts` (377 lines)

Manages chat state:
- Message history
- Streaming responses
- Tool call visualization
- Loading states
- Error handling
- Conversation clearing

### gridLayout.ts

**Source:** `src/webview/utils/gridLayout.ts` (465 lines)

Calculates canvas grid layout:
- Frame positioning
- Responsive sizing
- Collision detection
- Optimal arrangement

### themeParser.ts

**Source:** `src/webview/utils/themeParser.ts` (355 lines)

Parses theme definitions:
- Color extraction
- Typography parsing
- Component style extraction
- Theme validation

---

## Next

Return to the [README](README.md) for the full index.
