# What Is src.superdesign?

SuperDesign (v0.0.11) is the first open-source AI design agent that lives inside your IDE. Users describe what they want in natural language, and the agent generates UI mockups, components, and wireframes. It works with Cursor, Windsurf, Claude Code, and plain VS Code.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.superdesign/superdesign/`

## Features

| Feature | Description |
|---------|-------------|
| Product Mock | Instantly generate full UI screens from a single prompt |
| UI Components | Create reusable components you can drop into your code |
| Wireframes | Explore low-fidelity layouts for fast iteration |
| Fork & Iterate | Duplicate and evolve designs easily |
| Prompt-to-IDE | Copy prompts into your favorite AI IDE |

## Stack

| Layer | Technology |
|-------|-----------|
| Runtime | VS Code Extension (Node.js) |
| Language | TypeScript |
| AI SDK | Vercel AI SDK (`ai` package) |
| Providers | Anthropic, OpenAI, OpenRouter, Google |
| Frontend | React webview (Chat interface + Canvas) |
| Markdown | react-markdown + rehype-highlight |
| Icons | lucide-react |
| Build | esbuild |

## Key Dependencies

```json
{
  "@ai-sdk/anthropic": "^...",
  "@ai-sdk/google": "^...",
  "@ai-sdk/openai": "^...",
  "@anthropic-ai/claude-code": "^...",
  "@openrouter/ai-sdk-provider": "^...",
  "ai": "^...",
  "execa": "^...",
  "lucide-react": "^...",
  "react": "^...",
  "react-markdown": "^...",
  "react-zoom-pan-pinch": "^...",
  "zod": "^..."
}
```

**Source:** `package.json`.

## Quick Start

1. Install the extension from Cursor/VS Code Marketplace
2. Open the SuperDesign sidebar panel
3. Type a prompt (e.g., "Design a modern login screen")
4. View generated mockups, components, and wireframes
5. Fork, tweak, and paste into your project

## Project Structure

```
src/
├── extension.ts                    — VS Code extension entry point
├── providers/
│   └── chatSidebarProvider.ts     — Sidebar panel provider
├── services/
│   ├── chatMessageService.ts      — Message handling
│   ├── claudeCodeService.ts       — Claude Code integration
│   ├── customAgentService.ts      — AI agent orchestration
│   └── logger.ts                  — Logging service
├── tools/                          — 9 coding tools
│   ├── bash-tool.ts               — Shell command execution
│   ├── edit-tool.ts               — File editing
│   ├── glob-tool.ts               — File pattern matching
│   ├── grep-tool.ts               — Text search
│   ├── ls-tool.ts                 — Directory listing
│   ├── multiedit-tool.ts          — Multi-file editing
│   ├── read-tool.ts               — File reading
│   ├── theme-tool.ts              — Theme management
│   ├── tool-utils.ts              — Shared utilities
│   └── write-tool.ts              — File writing
├── templates/
│   └── webviewTemplate.ts         — Webview HTML template
├── types/
│   ├── agent.ts                   — Agent type definitions
│   └── context.ts                 — Context type definitions
└── webview/                        — React webview UI
    ├── App.tsx                     — Root component
    ├── components/                 — UI components
    ├── hooks/                      — React hooks
    └── utils/                      — Utility functions
```

## Next

Continue to [01-architecture](01-architecture.md) for the extension architecture.
