# River - Deep Dive

## Overview

**River** is a TypeScript library that makes AI agent streaming easy with full-stack type safety. It provides a TRPC-like client API for consuming and creating streams, works with any streaming library, and supports resumable/durable streams out of the box.

---

## Key Features

1. **Full-stack type safety on stream chunks** - End-to-end TypeScript safety
2. **TRPC-like client API** - Familiar, ergonomic interface
3. **Library agnostic** - Works with AI SDK, Mastra, custom streams
4. **Resumable/durable streams** - Redis provider for persistence
5. **Framework support** - TanStack Start and SvelteKit adapters

---

## Architecture

```
river/
├── packages/
│   ├── core/                 # Core River abstractions
│   ├── adapter-sveltekit/    # SvelteKit integration
│   ├── adapter-tanstack/     # TanStack Start integration
│   └── provider-redis/       # Redis persistence provider
├── apps/
│   ├── docs/                 # Documentation site
│   ├── sv-sandbox/           # SvelteKit sandbox demo
│   └── tan-sandbox/          # TanStack Start demo
└── README.md
```

---

## Core Concepts

### 1. River Streams

A River stream is a strongly-typed, chunked data stream:

```typescript
import { createRiverStream, defaultRiverProvider } from '@davis7dotsh/river-core';

type ClassifyChunkType = {
  character: string;
  type: 'vowel' | 'consonant' | 'special';
};

export const streamClassifyCharacters = createRiverStream<
  ClassifyChunkType,
  TanStackStartAdapterRequest
>()
  .input(z.object({ message: z.string() }))
  .provider(defaultRiverProvider())
  .runner(async ({ input, stream, abortSignal }) => {
    const { message } = input;
    const { appendChunk, close } = stream;

    for (const character of message.split('')) {
      const type = character.match(/[aeiou]/i)
        ? 'vowel'
        : character.match(/[bcdfghjklmnpqrstvwxyz]/i)
          ? 'consonant'
          : 'special';

      await appendChunk({ character, type });
      await new Promise((resolve) => setTimeout(resolve, 15));
    }

    await close();
  });
```

### 2. River Router

Expose streams through a router:

```typescript
import { createRiverRouter } from '@davis7dotsh/river-core';
import { streamClassifyCharacters } from './streams';

export const myRiverRouter = createRiverRouter({
  classifyCharacters: streamClassifyCharacters
});

export type MyRiverRouter = typeof myRiverRouter;
```

### 3. Adapter-Specific Handler

Each framework has its own handler:

**TanStack Start:**
```typescript
import { riverEndpointHandler } from '@davis7dotsh/river-adapter-tanstack';
import { myRiverRouter } from '@/lib/river/router';

const { GET, POST } = riverEndpointHandler(myRiverRouter);

export const Route = createFileRoute('/api/river/')({
  server: {
    handlers: { GET, POST }
  }
});
```

**SvelteKit:**
```typescript
import { riverEndpointHandler } from '@davis7dotsh/river-adapter-sveltekit';
import { myRiverRouter } from '$lib/river/router';

export const { GET, POST } = riverEndpointHandler(myRiverRouter);
```

### 4. Client-Side Consumption

**SvelteKit:**
```svelte
<script lang="ts">
  import { myRiverClient } from '$lib/river/client';

  const { start, resume } = myRiverClient.classifyCharacters({
    onStart: () => console.log('Starting stream'),
    onChunk: (chunk) => {
      // Fully type-safe!
      console.log('Chunk received', chunk);
    },
    onSuccess: (data) => {
      console.log('Finished', data.totalChunks, data.totalTimeMs);
    },
    onFatalError: (error) => console.error(error),
    onInfo: ({ encodedResumptionToken }) => {
      // Can resume with this token
      console.log('Resume with:', encodedResumptionToken);
    }
  });
</script>
```

**TanStack Start:**
```tsx
import { myRiverClient } from '@/lib/river/client';

const DemoComponent = () => {
  const { start, resume } = myRiverClient.classifyCharacters({
    onChunk: (chunk) => console.log('Chunk', chunk),
    onSuccess: (data) => console.log('Done', data)
  });

  // ...
}
```

---

## Stream Resumption

River supports resumable streams with the Redis provider:

```typescript
import { createRiverStream } from '@davis7dotsh/river-core';
import { redisProvider } from '@davis7dotsh/river-provider-redis';

export const resumableStream = createRiverStream<ChunkType, RequestType>()
  .provider(redisProvider({
    redisUrl: 'redis://localhost:6379'
  }))
  .runner(async ({ input, stream }) => {
    // Long-running operation
    for (const item of items) {
      await appendChunk({ data: item });
      await delay(100);
    }
    await close();
  });
```

### Resumption Flow

1. Client starts stream
2. Server crashes or connection drops
3. Client receives `onInfo({ encodedResumptionToken })`
4. Client calls `resume(encodedResumptionToken)`
5. Stream continues from last acknowledged chunk

---

## Type Safety

### Chunk Type Inference

```typescript
// Define chunk type
type Chunk = { value: number; timestamp: number };

// Create stream
const stream = createRiverStream<Chunk>()
  .runner(async ({ stream }) => {
    await stream.appendChunk({ value: 42, timestamp: Date.now() });
    // @ts-expect-error - wrong chunk type!
    await stream.appendChunk({ wrong: 'type' });
  });

// Client receives inferred type
const { start } = myRiverClient.myStream({
  onChunk: (chunk) => {
    // chunk is typed as { value: number; timestamp: number }
    console.log(chunk.value); // OK
    console.log(chunk.wrong); // Type error!
  }
});
```

### Input Validation with Zod

```typescript
const stream = createRiverStream<Chunk, RequestType>()
  .input(z.object({
    message: z.string().min(1),
    options: z.object({
      uppercase: z.boolean().optional()
    }).optional()
  }))
  .runner(async ({ input }) => {
    // input is typed and validated
    const { message, options } = input;
  });
```

---

## Provider System

### Default Provider (In-Memory)

```typescript
import { defaultRiverProvider } from '@davis7dotsh/river-core';

const stream = createRiverStream<Chunk>()
  .provider(defaultRiverProvider());
```

### Redis Provider (Persistent)

```typescript
import { redisProvider } from '@davis7dotsh/river-provider-redis';

const stream = createRiverStream<Chunk>()
  .provider(redisProvider({
    redisUrl: process.env.REDIS_URL,
    ttlMs: 3600_000 // 1 hour
  }));
```

### Custom Provider

```typescript
import { createRiverProvider } from '@davis7dotsh/river-core';

const customProvider = createRiverProvider({
  async createStream(state) { /* ... */ },
  async getStream(id) { /* ... */ },
  async appendChunk(id, chunk) { /* ... */ },
  async completeStream(id) { /* ... */ }
});
```

---

## AI SDK Integration

River works with Vercel's AI SDK:

```typescript
import { streamText } from 'ai';
import { createRiverStream } from '@davis7dotsh/river-core';

export const aiStream = createRiverStream<{ text: string }>()
  .input(z.object({ prompt: z.string() }))
  .runner(async ({ input, stream }) => {
    const result = streamText({
      model: openai('gpt-4o'),
      messages: [{ role: 'user', content: input.prompt }]
    });

    for await (const part of result.fullStream) {
      if (part.type === 'text-delta') {
        await stream.appendChunk({ text: part.text });
      }
    }

    await stream.close();
  });
```

---

## Roadmap

1. **Documentation** - Complete docs with examples
2. **Cursor Rules** - AI assistant rules for River
3. **S2 Provider** - Additional persistence backend
4. **More Adapters** - Next.js, Remix, Astro support

---

## Production Rust Implementation

### Architecture

```
river-rs/
├── crates/
│   ├── river-core/        # Core traits and types
│   ├── river-axum/        # Axum adapter
│   ├── river-actix/       # Actix-web adapter
│   ├── river-redis/       # Redis provider
│   └── river-macros/      # Procedural macros
└── examples/
```

### Core Traits

```rust
pub trait RiverStream: Send {
    type Chunk: Serialize + Clone;
    type Input: DeserializeOwned;

    async fn run(self, input: Self::Input) -> Result<(), RiverError>;
    async fn append_chunk(&self, chunk: Self::Chunk) -> Result<(), RiverError>;
    async fn close(&self) -> Result<(), RiverError>;
}

pub trait RiverProvider: Send + Sync {
    async fn create_stream(&self, id: String) -> Result<StreamHandle, RiverError>;
    async fn get_stream(&self, id: &str) -> Result<StreamState, RiverError>;
    async fn append_chunk(&self, id: &str, chunk: Value) -> Result<(), RiverError>;
}
```

### Key Crates

- `tokio-stream` - Async stream utilities
- `serde` - Serialization
- `redis` / `sqlx` - Persistence
- `axum` / `actix-web` - Web frameworks
- `typescript-definitions` - Generate TS types from Rust
