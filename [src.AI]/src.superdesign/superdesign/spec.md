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

## What src.superdesign Is

A VS Code extension that provides an AI design agent inside your IDE. Users describe what they want in natural language, and the agent generates UI mockups, components, and wireframes. It uses the Vercel AI SDK with multiple provider support (Anthropic, OpenAI, OpenRouter, Google) and a set of coding tools (read, write, edit, bash, glob, grep, ls, theme, multiedit). Works with Cursor, Windsurf, Claude Code, and plain VS Code.

## Documentation Goal

A reader should understand:

1. SuperDesign's architecture: VS Code extension with webview sidebar
2. The AI provider abstraction layer: Vercel AI SDK with 4 providers
3. The tool system: 9 coding tools for file manipulation and code execution
4. The webview UI: Chat interface, canvas, design panel, theme previews
5. Configuration: VS Code settings for API keys, models, providers

## Documentation Structure

```
src.superdesign/
├── spec.md
├── markdown/
│   ├── README.md                  ← Index
│   ├── 00-overview.md             ← What SuperDesign is, version, features
│   ├── 01-architecture.md         ← Extension architecture, webview, services
│   ├── 02-ai-providers.md         ← Vercel AI SDK, 4 providers, model config
│   ├── 03-tools.md                ← 9 coding tools (read, write, edit, bash, etc.)
│   └── 04-webview-ui.md           ← Chat, canvas, theme previews
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
| 7 | HTML generation | DONE |
| 8 | Grandfather review | DONE |

## Build System

```bash
cd "[src.AI]/src.superdesign/superdesign"
python3 build.py superdesign
```

## Expected Outcome

A developer reading these docs can understand how SuperDesign works as a VS Code extension and how to configure or extend it.
