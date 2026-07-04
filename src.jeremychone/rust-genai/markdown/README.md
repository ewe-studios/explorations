# rust-genai Documentation

Multi-provider AI client library for Rust.

## Documents

| # | Document | Description |
|---|----------|-------------|
| 0 | [Overview](00-overview.md) | Architecture, 19 providers, Client/Chat types |
| 1 | [Client](01-client.md) | ClientBuilder, ClientConfig, Resolvers |

## Source

`src.jeremychone/rust-genai/src/` — 65 Rust files

## Usage

```rust
use genai::Client;

let client = Client::builder()
    .with_chat_options(ChatOptions {
        max_tokens: Some(1024),
    })
    .build();

let request = ChatRequest::from_user("Hello");
let response = client.exec_chat("gpt-4o", request).await?;
```

## Providers

OpenAI, Gemini, Anthropic, Groq, DeepSeek, Ollama, Vertex, Cohere, xAI, Together, Fireworks, Nebius, Mimo, Zai, BigModel, Aliyun, OllamaCloud, GithubCopilot
