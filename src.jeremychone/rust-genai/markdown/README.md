# rust-genai — Documentation

**Source:** `src/` — ~50 Rust files across 8 modules. Multi-provider AI client library implementing unified chat interface across 7 adapters (OpenAI, Anthropic, Gemini, Cohere, Groq, xAI, Ollama).

`rust-genai` (published as `genai` on crates.io) provides a typed, multi-provider AI client library with a unified `Client::exec_chat` / `Client::exec_chat_stream` API. It uses an **Adapter pattern** with static dispatch, a **resolution pipeline** (auth → model mapper → service target), and a **two-tier streaming architecture**.

## Documentation

- [Overview](00-overview.md) — Architecture, public API, adapter mapping, request/response flow, design principles
- [Adapter System](01-adapter-system.md) — Adapter trait (stateless), AdapterDispatcher, AdapterKind auto-detection, 7 adapter implementations, streaming architecture, InterStreamEvent, WebStream
- [Chat System](02-chat-system.md) — ChatRequest, ChatMessage, MessageContent, Tool/ToolCall/ToolResponse, ChatResponse, ChatStream, ChatOptions, cascading ChatOptionsSet, ChatResponseFormat, printer utility
- [Client & Resolution](03-client-resolution.md) — Arc-based Client, ClientBuilder fluent API, ClientConfig, resolution pipeline (model mapper → auth resolver → service target), AuthData, Endpoint, ServiceTarget, exec_chat flow
- [Web Layer & Error Model](04-web-and-error.md) — WebClient (reqwest wrapper), WebResponse, WebStream (delimiter/JSON array modes), error hierarchy, value_ext JSON manipulation, module visibility

## Supported Providers

| Provider | Adapter | Models |
|----------|---------|--------|
| OpenAI | OpenAIAdapter | gpt-4o, gpt-4o-mini, o1-preview, o1-mini |
| Anthropic | AnthropicAdapter | claude-3-5-sonnet, claude-3-5-haiku, claude-3-opus, claude-3-haiku |
| Google | GeminiAdapter | gemini-1.5-pro, gemini-1.5-flash, gemini-1.5-flash-8b |
| Cohere | CohereAdapter | command-light, command-r |
| Groq | GroqAdapter | groq-hosted models |
| xAI | XaiAdapter | grok-beta |
| Ollama | OllamaAdapter | any local model |
