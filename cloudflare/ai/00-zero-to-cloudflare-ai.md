# Zero to Cloudflare AI: Complete Guide

**Last Updated:** 2026-04-05

---

## Table of Contents

1. [Introduction](#introduction)
2. [What is Cloudflare AI?](#what-is-cloudflare-ai)
3. [Workers AI Models](#workers-ai-models)
4. [Installation](#installation)
5. [Quick Start](#quick-start)
6. [Chat Completion](#chat-completion)
7. [Image Generation](#image-generation)
8. [Embeddings](#embeddings)
9. [Transcription](#transcription)
10. [Text-to-Speech](#text-to-speech)
11. [Reranking](#reranking)
12. [AI Gateway](#ai-gateway)
13. [Production Deployment](#production-deployment)

---

## Introduction

Cloudflare AI provides **on-worker AI inference** through Workers AI and **AI gateway** for routing to external providers. This guide covers the `workers-ai-provider` package for the Vercel AI SDK and `@cloudflare/tanstack-ai` for TanStack AI integration.

```bash
npm install workers-ai-provider
# or
npm install @cloudflare/tanstack-ai
```

---

## What is Cloudflare AI?

### Workers AI

Run AI models directly on Cloudflare's edge network:

```
┌─────────────────────────────────────────────────────────┐
│               Cloudflare Global Network                  │
│                                                          │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐            │
│  │  Worker   │  │  Worker   │  │  Worker   │            │
│  │   + AI    │  │   + AI    │  │   + AI    │            │
│  │  (SFO)    │  │  (LHR)    │  │  (NRT)    │            │
│  └───────────┘  └───────────┘  └───────────┘            │
│                                                          │
│  Models run at the edge - no round-trip to central API   │
└─────────────────────────────────────────────────────────┘
```

### AI Gateway

Route AI requests through Cloudflare for:
- **Caching** - Reduce API costs with response caching
- **Rate Limiting** - Control request throughput
- **Observability** - Track usage, latency, errors
- **Fallback** - Automatic provider failover

```
Client → AI Gateway → [Workers AI, OpenAI, Anthropic, etc.]
```

### Six AI Capabilities

| Capability | Models | Use Case |
|------------|--------|----------|
| **Chat** | Llama, Gemma, Mistral | Conversational AI |
| **Image** | Flux, Stable Diffusion | Image generation |
| **Embeddings** | BGE, Sentence Transformers | Vector search |
| **Transcription** | Whisper | Speech-to-text |
| **TTS** | PlayHT | Text-to-speech |
| **Reranking** | BGE Reranker | Search relevance |

---

## Workers AI Models

### Chat Models

```typescript
type TextGenerationModels =
  | "@cf/meta/llama-3.3-70b-instruct-fp8-fast"
  | "@cf/meta/llama-3.2-3b-instruct"
  | "@cf/google/gemma-2b-it-lora"
  | "@cf/google/gemma-7b-it-lora"
  | "@cf/mistral/mistral-7b-instruct-v0.1"
  | "@cf/qwen/qwen1.5-14b-chat-awq"
  | "@hf/thebloke/deepseek-coder-6.7b-base-awq"
```

### Image Models

```typescript
type ImageGenerationModels =
  | "@cf/black-forest-labs/flux-1-schnell"
  | "@cf/runwayml/stable-diffusion-v1-5-img2img"
  | "@cf/stabilityai/stable-diffusion-xl-base-1.0"
```

### Embedding Models

```typescript
type EmbeddingModels =
  | "@cf/baai/bge-small-en-v1.5"
  | "@cf/baai/bge-base-en-v1.5"
  | "@cf/baai/bge-large-en-v1.5"
```

### Transcription Models

```typescript
type TranscriptionModels =
  | "@cf/openai/whisper"
  | "@cf/openai/whisper-large-v3-turbo"
```

### Speech Models

```typescript
type SpeechModels =
  | "@cf/playht/playht-tts-model-v1"
  | "@cf/playht/playht-tts-model-v2"
```

### Reranking Models

```typescript
type RerankingModels =
  | "@cf/baai/bge-reranker-v2-m3"
```

---

## Installation

### Vercel AI SDK Provider

```bash
npm install workers-ai-provider ai
```

### TanStack AI Provider

```bash
npm install @cloudflare/tanstack-ai @tanstack/react-ai
```

### Wrangler Setup

```bash
npm install -g wrangler
wrangler login
```

---

## Quick Start

### Create Worker

```bash
# Create new worker
npm create cloudflare@latest my-ai-worker

# Add Workers AI binding
# Edit wrangler.jsonc:
```

```jsonc
{
  "name": "my-ai-worker",
  "main": "src/index.ts",
  "compatibility_date": "2026-01-28",
  
  "ai": {
    "binding": "AI"
  }
}
```

### Basic Chat Example

```typescript
// src/index.ts
import { createWorkersAI } from "workers-ai-provider";
import { generateText } from "ai";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const ai = createWorkersAI({ binding: env.AI });
    
    const result = await generateText({
      model: ai.chat("@cf/meta/llama-3.3-70b-instruct-fp8-fast"),
      messages: [
        { role: "user", content: "What is Cloudflare Workers AI?" }
      ]
    });
    
    return new Response(result.text);
  }
};
```

---

## Chat Completion

### Using Vercel AI SDK

```typescript
import { createWorkersAI } from "workers-ai-provider";
import { generateText, streamText } from "ai";

const ai = createWorkersAI({ binding: env.AI });

// Non-streaming
const result = await generateText({
  model: ai.chat("@cf/meta/llama-3.3-70b-instruct-fp8-fast"),
  messages: [
    { role: "system", content: "You are a helpful assistant." },
    { role: "user", content: "Explain quantum computing" }
  ],
  temperature: 0.7,
  maxTokens: 500
});

console.log(result.text);
console.log(result.usage);  // { promptTokens, completionTokens }

// Streaming
const stream = await streamText({
  model: ai.chat("@cf/meta/llama-3.3-70b-instruct-fp8-fast"),
  messages: [
    { role: "user", content: "Write a poem about coding" }
  ]
});

for await (const chunk of stream.textStream) {
  process.stdout.write(chunk);
}
```

### Using TanStack AI

```typescript
import { createWorkersAI } from "@cloudflare/tanstack-ai";
import { useChat } from "@tanstack/react-ai";

const ai = createWorkersAI({ binding: env.AI });

function ChatComponent() {
  const { messages, input, handleInputChange, handleSubmit } = useChat({
    model: ai.chat("@cf/meta/llama-3.3-70b-instruct-fp8-fast")
  });
  
  return (
    <div>
      {messages.map(m => (
        <div key={m.id}>{m.role}: {m.content}</div>
      ))}
      <form onSubmit={handleSubmit}>
        <input value={input} onChange={handleInputChange} />
      </form>
    </div>
  );
}
```

### Tool Calling

```typescript
import { tool } from "ai";

const result = await generateText({
  model: ai.chat("@cf/meta/llama-3.3-70b-instruct-fp8-fast"),
  messages: [
    { role: "user", content: "What's the weather in London?" }
  ],
  tools: {
    getWeather: tool({
      description: "Get current weather for a location",
      parameters: z.object({
        location: z.string().describe("City name")
      }),
      execute: async ({ location }) => {
        // Call weather API
        return { temp: 15, condition: "cloudy" };
      }
    })
  }
});
```

---

## Image Generation

```typescript
import { createWorkersAI } from "workers-ai-provider";
import { generateImage } from "ai";

const ai = createWorkersAI({ binding: env.AI });

const result = await generateImage({
  model: ai.image("@cf/black-forest-labs/flux-1-schnell"),
  prompt: "A cyberpunk city at sunset, neon lights, futuristic",
  n: 1,
  size: "1024x1024"
});

// result.images[0] contains base64-encoded image
const imageBuffer = Buffer.from(result.images[0], "base64");
```

---

## Embeddings

```typescript
import { createWorkersAI } from "workers-ai-provider";
import { embed, embedMany } from "ai";

const ai = createWorkersAI({ binding: env.AI });

// Single embedding
const { embedding } = await embed({
  model: ai.embedding("@cf/baai/bge-small-en-v1.5"),
  value: "Cloudflare Workers AI runs models at the edge"
});

// Multiple embeddings
const { embeddings } = await embedMany({
  model: ai.embedding("@cf/baai/bge-small-en-v1.5"),
  values: [
    "Document 1 content",
    "Document 2 content",
    "Document 3 content"
  ]
});

// Use for vector search
const vectorStore = new Map();
embeddings.forEach((embedding, i) => {
  vectorStore.set(`doc-${i}`, embedding);
});
```

---

## Transcription

```typescript
import { createWorkersAI } from "workers-ai-provider";
import { generateSpeech } from "ai";

const ai = createWorkersAI({ binding: env.AI });

// Transcribe audio file
const audioBuffer = await fs.readFile("recording.wav");

const result = await generateSpeech({
  model: ai.transcription("@cf/openai/whisper"),
  audio: audioBuffer,
  language: "en",
  task: "transcribe"  // or "translate"
});

console.log(result.text);  // Transcribed text
```

---

## Text-to-Speech

```typescript
import { createWorkersAI } from "workers-ai-provider";

const ai = createWorkersAI({ binding: env.AI });

const result = await ai.speech("@cf/playht/playht-tts-model-v1").doGenerate({
  text: "Hello, this is Cloudflare AI speaking.",
  voice: "default"
});

// result.audio contains audio data
const audioBuffer = result.audio;
```

---

## Reranking

```typescript
import { createWorkersAI } from "workers-ai-provider";

const ai = createWorkersAI({ binding: env.AI });

const result = await ai.reranking("@cf/baai/bge-reranker-v2-m3").doGenerate({
  query: "Cloudflare edge computing",
  documents: [
    "Cloudflare runs servers at the edge",
    "Traditional cloud computing is centralized",
    "Edge networks reduce latency"
  ],
  topN: 2
});

// result.rerank contains reordered documents with scores
```

---

## AI Gateway

### Configure Gateway

```typescript
const ai = createWorkersAI({
  binding: env.AI,
  gateway: {
    id: "my-gateway"  // Gateway ID from Cloudflare dashboard
  }
});
```

### Gateway Features

#### Caching

```
Gateway caches responses based on:
- Model ID
- Messages
- Parameters

Cache hits return instantly without model inference.
```

#### Rate Limiting

```yaml
# In Cloudflare dashboard
Rate Limits:
  - Requests per minute: 60
  - Tokens per minute: 100000
```

#### Fallback

```yaml
# Configure fallback chain
Fallback:
  Primary: Workers AI
  Secondary: OpenAI
  Tertiary: Anthropic
```

#### Observability

```typescript
// Gateway logs include:
- Request latency
- Token usage
- Error rates
- Model performance
```

---

## Production Deployment

### Environment Configuration

```jsonc
// wrangler.jsonc
{
  "name": "my-ai-worker",
  "main": "src/index.ts",
  "compatibility_date": "2026-01-28",
  
  "ai": {
    "binding": "AI"
  },
  
  "vars": {
    "ENVIRONMENT": "production"
  },
  
  "observability": {
    "enabled": true
  }
}
```

### Deploy

```bash
# Deploy worker
wrangler deploy

# View logs
wrangler tail

# View metrics
wrangler metrics
```

### Secrets

```bash
# Set API keys for external providers
wrangler secret put OPENAI_API_KEY
wrangler secret put ANTHROPIC_API_KEY
```

### Scaling

Workers AI scales automatically:
- **No cold starts** - Models pre-loaded at edge
- **Global distribution** - Runs nearest to users
- **Pay per token** - No fixed infrastructure cost

---

## Related Documents

- [Deep Dive: Workers AI Infrastructure](./01-workers-ai-infrastructure-deep-dive.md)
- [Deep Dive: Model Providers](./02-model-providers-deep-dive.md)
- [Deep Dive: Vector Embeddings](./03-vector-embeddings-deep-dive.md)
- [Rust Revision](./rust-revision.md)
- [Production Guide](./production-grade.md)
