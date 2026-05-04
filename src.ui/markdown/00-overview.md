# OpenUI Ecosystem -- What It Is and Why It Exists

OpenUI is a full-stack Generative UI framework. Instead of having LLMs output JSON or markdown for UI, OpenUI defines a custom DSL — **OpenUI Lang** — that LLMs stream as structured UI markup. The markup is progressively parsed while streaming, materialized against a component schema, and rendered as React components. The result is a copilot interface where the AI dynamically builds its own UI in real time.

**Aha:** The key insight behind OpenUI Lang is that LLMs are better at producing compact, structured text than verbose JSON. A JSON component definition with nested objects, arrays, and quoted strings is 3-5x more tokens than the equivalent OpenUI Lang syntax. Fewer tokens means cheaper generation, faster streaming, and fewer hallucination opportunities. The DSL is designed for LLM ergonomics — not human ergonomics — prioritizing brevity and streaming-friendliness over readability.

Source: `openui/packages/lang-core/src/` — parser and runtime
Source: `openui/packages/react-lang/src/` — React renderer

## Quick Architecture

```
┌─────────────────────────────────────────────────────┐
│                   LLM (streaming)                    │
│                    ↓ OpenUI Lang                     │
├─────────────────────────────────────────────────────┤
│                 Browser (React)                      │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │ Streaming   │  │ Materializer │  │  React     │ │
│  │ Parser      │→→│ (resolve refs│→→│  Renderer  │ │
│  │ (incremental│  │  map args)   │  │  (React    │ │
│  │  tokenize)  │  └──────────────┘  │   nodes)   │ │
│  └─────────────┘                    └──────┬─────┘ │
│                                            │       │
│  ┌──────────────────────────────────────┐  │       │
│  │ Component Library (60+ components)   │←←┘       │
│  │ Stack, Table, Form, Chart, Button... │          │
│  └──────────────────────────────────────┘          │
├────────────────────────────────────────────────────┤
│                Server (OpenClaw)                    │
│  ┌────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │ Claw       │  │ Gateway      │  │ SQLite     │ │
│  │ Plugin     │←→│ Socket       │←→│ + JSON     │ │
│  │ (system    │  │ (WebSocket   │  │ filesystem │ │
│  │  prompt)   │  │  + RPC)      │  │ storage    │ │
│  └────────────┘  └──────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────┘
```

## The Ecosystem at a Glance

| Project | Purpose |
|---------|---------|
| `openui/` | Core framework: lang-core parser, react-lang renderer, react-ui components, react-headless chat state |
| `openclaw-ui/` | OpenClaw agent integration: claw-plugin (server) + claw-client (Next.js) |
| `openwebui-plugin/` | Python plugin for OpenWebUI integration |
| `examples/` | 20+ example apps (Next.js, LangChain, LangGraph, CrewAI, etc.) |
| `voice-agent-generativeui/` | Voice agent with LiveKit integration |
| `create-c1-app/` | CLI scaffolding tool for new apps |
| `streamlit-thesys-genui/` | Python Streamlit component |

See [Architecture](01-architecture.md) for the full dependency graph.
See [Lang Core](02-lang-core.md) for the DSL deep dive.
