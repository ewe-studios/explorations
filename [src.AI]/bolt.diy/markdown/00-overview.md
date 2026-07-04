---
title: bolt.diy — AI-Powered Full-Stack Web Development Platform
---

# bolt.diy — AI-Powered Full-Stack Web Development Platform

**bolt.diy lets you generate complete web applications from natural language prompts, running entirely in the browser.** Built on Remix and React, it leverages StackBlitz WebContainers to execute Node.js in the browser, supports 19+ LLM providers, and deploys to Netlify, Cloudflare, or as an Electron desktop app.

Source: `app/` — 396 TypeScript/TSX files, MIT license

## Platform Architecture

```mermaid
flowchart TD
    User["User Prompt"] --> Chat["Chat Interface<br/>React components/chat/"]
    Chat --> LLMRouter["LLM Provider Router<br/>lib/modules/llm/"]
    LLMRouter --> Provider["Provider Selection<br/>19+ LLMs"]
    Provider --> Stream["Streaming Response<br/>Server-Sent Events"]

    Stream --> CodeGen["Code Generation<br/>AI produces files + diffs"]
    CodeGen --> Editor["Monaco Editor<br/>components/editor/"]
    CodeGen --> WebContainer["WebContainer<br/>In-browser Node.js runtime"]

    WebContainer --> Preview["Live Preview<br/>iframe rendering"]
    WebContainer --> Terminal["In-browser Terminal<br/>npm install, build, etc."]

    Editor --> Git["Git Integration<br/>components/git/"]
    Preview --> Deploy["Deployment<br/>components/deploy/"]
    Deploy --> Netlify["Netlify / Cloudflare"]
```

## Key Features

| Feature | Implementation | Key Files |
|---------|---------------|-----------|
| AI Chat | Streaming chat with file previews | `app/components/chat/` |
| Code Editor | Monaco editor with file tree | `app/components/editor/` |
| WebContainer | StackBlitz in-browser Node.js | `app/lib/webcontainer/` |
| LLM Providers | 19+ providers behind common interface | `app/lib/modules/llm/` |
| Git Sync | Clone, commit, push, import | `app/components/git/` |
| Deployment | Netlify, Cloudflare, Vercel | `app/components/deploy/` |
| Supabase | Database connection wizard | `app/lib/modules/supabase/` |
| Electron | Desktop app wrapper | `electron/` |
| MCP | Model Context Protocol tools | `app/lib/modules/mcp/` |
| Settings | Provider keys, model selection | `app/components/@settings/` |

## 19+ LLM Providers

```
OpenAI, Anthropic, Ollama, OpenRouter, Gemini, LMStudio,
Mistral, xAI, HuggingFace, DeepSeek, Groq, Cohere,
Together, Perplexity, Moonshot, Hyperbolic, GitHub Models,
Amazon Bedrock, Azure OpenAI
```

Each provider is configured through the Settings panel with API keys, base URLs, and model selection.

**Aha:** bolt.diy uses a "provider as plugin" pattern where each LLM provider is a self-contained module implementing a common interface. Adding a new provider requires only creating a new file in `app/lib/modules/llm/providers/` that implements the `StreamingProvider` interface — no changes to the chat component, the streaming pipeline, or the UI.

## Quick Start

```bash
# Docker (recommended)
docker compose up

# Manual
pnpm install
pnpm run dev

# Electron desktop
pnpm run electron:dev
```

## The WebContainer Revolution

```mermaid
sequenceDiagram
    participant AI as AI (LLM)
    participant Bolt as bolt.diy
    participant WC as WebContainer
    participant Browser as Browser DOM

    AI->>Bolt: Generated code (React app)
    Bolt->>WC: Write files to virtual FS
    WC->>WC: npm install (in browser!)
    WC->>WC: npm run dev
    WC->>Browser: Render preview in iframe
    Browser-->>User: Live app preview
```

**Aha:** The WebContainer runs a real Node.js process inside the browser using Service Workers and WebAssembly. This means `npm install`, `vite dev`, and even database operations run entirely client-side — no remote server needed for preview. The generated app is rendered in an iframe that connects to a local dev server running inside the WebContainer.

## Next

→ [Architecture](01-architecture.md) — Remix framework, component tree, data flow.
