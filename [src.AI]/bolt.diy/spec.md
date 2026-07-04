---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/bolt.diy/
language: TypeScript (Remix + React)
version: 1.0.0
explored_at: 2026-07-04
license: MIT
repository: github.com/stackblitz-labs/bolt.diy
---

# spec.md — bolt.diy Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/bolt.diy/`
- **Language:** TypeScript (Remix v2 + React 18)
- **Version:** 1.0.0
- **LOC:** 396 TypeScript/TSX files
- **License:** MIT
- **Repository:** github.com/stackblitz-labs/bolt.diy

## What bolt.diy Is

bolt.diy is an open-source AI-powered full-stack web development platform that runs entirely in the browser. Users write natural language prompts, the LLM generates complete web applications (React, Node.js, etc.), and the generated code executes in-browser via StackBlitz WebContainers — a browser-based Node.js runtime. The platform includes 19+ LLM provider integrations, a Monaco-based code editor, Git version control, one-click deployment to Netlify/Cloudflare, Supabase database support, MCP (Model Context Protocol) integration, and an Electron desktop app wrapper. The entire stack runs on Remix v2 with Cloudflare Workers for serverless API functions.

## Architecture

1. **Remix Framework** — SSR React app with file-based routing, loader/action patterns
2. **LLM Provider Layer** — 19+ providers abstracted behind a common `LLMProvider` interface
3. **WebContainer Runtime** — StackBlitz's in-browser Node.js for live preview and package management
4. **Electron Shell** — Desktop app with native file system access
5. **Cloudflare Workers** — Serverless API functions for proxy, auth, and deployment

## Documentation Structure

```
bolt.diy/
├── spec.md
├── build.py
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-llm-providers.md
│   ├── 03-code-generation.md
│   ├── 04-webcontainer-runtime.md
│   └── 05-deployment.md
└── html/
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-architecture.md | DONE |
| 3 | 02-llm-providers.md | DONE |
| 4 | 03-code-generation.md | DONE |
| 5 | 04-webcontainer-runtime.md | DONE |
| 6 | 05-deployment.md | DONE |
| 7 | HTML generation | DONE |
| 8 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/bolt.diy
python3 build.py .
```

## Quality Requirements

All Iron Rules apply: code snippets from source, ≥2 mermaid diagrams per doc, ≥1 Aha moment per doc, cross-references, source path refs, grandfather review verification.

## Expected Outcome

A reader should understand: the Remix-based architecture, LLM provider abstraction, WebContainer in-browser runtime, AI code generation pipeline, and deployment integration.
