---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.superdesign/superdesign/
language: TypeScript
version: 0.0.11
runtime: VS Code Extension (Node.js)
explored_at: 2026-07-03
license: MIT
---

# spec.md — src.superdesign Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.superdesign/superdesign/`
- **Language:** TypeScript
- **Version:** 0.0.11
- **Runtime:** VS Code Extension (Node.js)
- **License:** MIT
- **Package Manager:** npm (package-lock.json)
- **Repository:** https://github.com/superdesigndev/superdesign
- **Publisher:** iganbold (VS Code Marketplace)

## What src.superdesign Is

A VS Code extension that provides an AI design agent inside your IDE. Users describe what they want in natural language, and the agent follows a structured 4-step design workflow (ASCII wireframe → theme CSS → animation micro-syntax → single-file HTML generation). It uses the Vercel AI SDK with multiple provider support (Anthropic, OpenAI, OpenRouter, Google), 9 coding tools for file manipulation and code execution, a React webview sidebar for chat, and a canvas panel for live design preview with file watching and CSS inlining. Works with Cursor, Windsurf, Claude Code, and plain VS Code via injected design rules.

## Documentation Goal

A reader should understand:

1. SuperDesign's architecture: VS Code extension with webview sidebar + canvas panel
2. The AI provider abstraction layer: Vercel AI SDK with 4 providers + Helicone proxy
3. The tool system: 9 coding tools with Zod validation and ExecutionContext
4. The webview UI: Chat interface, canvas, theme previews, model selector
5. The design workflow: 4-step prompt-enforced pipeline with ASCII wireframes
6. Configuration: VS Code settings for API keys, models, providers
7. Project initialization: Directory creation, rule injection, Welcome flow
8. Build system: Esbuild bundling, test structure, VSIX packaging

## Documentation Structure

```
src.superdesign/
├── spec.md
├── markdown/
│   ├── README.md                  ← Index
│   ├── 00-overview.md             ← What SuperDesign is, version, features
│   ├── 01-architecture.md         ← Extension architecture, webview, services
│   ├── 02-ai-providers.md         ← Vercel AI SDK, 4 providers, model config, Helicone
│   ├── 03-tools.md                ← 9 coding tools (read, write, edit, bash, etc.)
│   ├── 04-webview-ui.md           ← React app, components, hooks, message protocol
│   ├── 05-superdesign-deep.md     ← Source-level: every file, every function
│   ├── 06-design-workflow.md      ← 4-step design-to-code pipeline
│   ├── 07-canvas-panel.md         ← Design preview, file watching, CSS inlining
│   ├── 08-commands-settings.md    ← VS Code commands, settings, configuration
│   ├── 09-chat-streaming.md       ← CoreMessage protocol, streaming, errors
│   ├── 10-type-system.md          ← AgentService, ExecutionContext, Canvas types
│   ├── 11-initialization-project.md ← Project init, rule injection, Welcome flow
│   ├── 12-design-prompt-engineering.md ← 627-line system prompt analysis
│   └── 13-build-system-testing.md ← Esbuild, TypeScript, packaging, tests
├── html/
│   ├── index.html
│   └── *.html
└── build.py
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 1 | README.md (index) | DONE |
| 2 | 00-overview.md | DONE |
| 3 | 01-architecture.md | DONE |
| 4 | 02-ai-providers.md | DONE |
| 5 | 03-tools.md | DONE |
| 6 | 04-webview-ui.md | DONE |
| 7 | 05-superdesign-deep.md | DONE |
| 8 | 06-design-workflow.md | DONE |
| 9 | 07-canvas-panel.md | DONE |
| 10 | 08-commands-settings.md | DONE |
| 11 | 09-chat-streaming.md | DONE |
| 12 | 10-type-system.md | DONE |
| 13 | 11-initialization-project.md | DONE |
| 14 | 12-design-prompt-engineering.md | DONE |
| 15 | 13-build-system-testing.md | DONE |
| 16 | HTML generation | TODO |
| 17 | Grandfather review | TODO |

## Build System

```bash
cd "[src.AI]/src.superdesign/superdesign"
python3 build.py superdesign
```

## Expected Outcome

A developer reading these docs can understand how SuperDesign works as a VS Code extension, configure it for their use, extend it with new tools or providers, and understand the design prompt engineering that drives the 4-step workflow.

## Quality Requirements

All 10 Iron Rules from the documentation directive apply:
1. Detailed sections with code snippets from actual source
2. Key facts, principles, and ideas articulated quickly
3. Clear, non-overly-complex sentences
4. Minimum 2 mermaid diagrams per document
5. Good visual assets (tables, diagrams, code blocks)
6. Generated HTML via build.py
7. Cross-references between documents
8. Source path references (file:line)
9. At least 1 Aha moment per document
10. Navigation: index + prev/next buttons (handled by build.py)
