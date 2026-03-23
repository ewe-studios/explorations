# AI Gateway Embeddings Demo - Deep Dive Exploration

## Overview

**AI Gateway Embeddings Demo** demonstrates RAG (Retrieval Augmented Generation) using Vercel AI Gateway for embeddings and Neon (serverless PostgreSQL) for vector storage.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.VarcelLabs/ai-gateway-embeddings-demo`

---

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Next.js App    │ →── │  Vercel AI       │ →── │  OpenAI         │
│                 │     │  Gateway         │     │  Embeddings     │
│  /api/chat      │     │                  │     │  (text-embedding-ada-002)│
└────────┬────────┘     └──────────────────┘     └─────────────────┘
         │
         ↓
┌─────────────────┐     ┌──────────────────┐
│  Neon           │ ←── │  Drizzle ORM     │
│  (PostgreSQL +  │     │  (SQL Builder)   │
│   pgvector)     │     │                  │
└─────────────────┘     └──────────────────┘
```

---

## Tech Stack

| Component | Technology |
|-----------|------------|
| Framework | Next.js App Router |
| Database | Neon (serverless PostgreSQL) |
| ORM | Drizzle ORM |
| Vector Search | pgvector (cosine similarity) |
| Embeddings | Vercel AI Gateway → OpenAI |
| AI SDK | `ai` package |

---

## Key Implementation Details

### 1. Embedding Generation (`lib/ai/embedding.ts`)

```typescript
import { embed, embedMany } from 'ai';
import { db } from '../db';
import { cosineDistance, desc, gt, sql } from 'drizzle-orm';
import { embeddings } from '../db/schema/embeddings';

const embeddingModel = 'openai/text-embedding-ada-002';

// Chunk text by sentences
const generateChunks = (input: string): string[] => {
  return input
    .trim()
    .split('.')
    .filter(i => i !== '');
};

// Generate embeddings for multiple chunks
export const generateEmbeddings = async (
  value: string,
): Promise<Array<{ embedding: number[]; content: string }>> => {
  const chunks = generateChunks(value);
  const { embeddings } = await embedMany({
    model: embeddingModel,
    values: chunks,
  });
  return embeddings.map((e, i) => ({ content: chunks[i], embedding: e }));
};

// Generate single embedding
export const generateEmbedding = async (value: string): Promise<number[]> => {
  const input = value.replaceAll('\\n', ' ');
  const { embedding } = await embed({
    model: embeddingModel,
    value: input,
  });
  return embedding;
};
```

**Key Patterns:**

1. **Text Chunking** - Split by sentences for better retrieval granularity
2. **Batch Embedding** - `embedMany` for efficiency
3. **AI Gateway** - Unified model reference (`openai/text-embedding-ada-002`)

### 2. Vector Similarity Search

```typescript
export const findRelevantContent = async (userQuery: string) => {
  // 1. Embed the query
  const userQueryEmbedded = await generateEmbedding(userQuery);

  // 2. Cosine similarity search in PostgreSQL
  const similarity = sql<number>`1 - (${cosineDistance(
    embeddings.embedding,
    userQueryEmbedded,
  )})`;

  // 3. Query with similarity threshold
  const similarGuides = await db
    .select({ name: embeddings.content, similarity })
    .from(embeddings)
    .where(gt(similarity, 0.5))  // Only results > 50% similar
    .orderBy(t => desc(t.similarity))  // Most similar first
    .limit(4);  // Top 4 results

  return similarGuides;
};
```

**SQL Equivalent:**
```sql
SELECT content, 1 - (embedding <=> $1) AS similarity
FROM embeddings
WHERE 1 - (embedding <=> $1) > 0.5
ORDER BY similarity DESC
LIMIT 4;
```

### 3. Database Schema (`lib/db/schema/embeddings.ts`)

```typescript
import { pgTable, text, vector } from 'drizzle-orm/pg-core';

// Embeddings table with pgvector
export const embeddings = pgTable('embeddings', {
  id: text('id').primaryKey().notNull(),
  content: text('content').notNull(),
  embedding: vector('embedding', { dimensions: 1536 }).notNull(), // OpenAI ada-002
  metadata: text('metadata').$type<Record<string, any>>(),
});
```

**Dimensions:**
- `text-embedding-ada-002` = 1536 dimensions
- `text-embedding-3-small` = 1536 dimensions
- `text-embedding-3-large` = 3072 dimensions

### 4. Chat API with RAG Tools (`app/api/chat/route.ts`)

```typescript
import {
  convertToModelMessages,
  streamText,
  tool,
  UIMessage,
  stepCountIs,
} from 'ai';
import { z } from 'zod';
import { findRelevantContent } from '@/lib/ai/embedding';
import { createResource } from '@/lib/actions/resources';

export const maxDuration = 30; // 30 seconds for serverless

export async function POST(req: Request) {
  const { messages }: { messages: UIMessage[] } = await req.json();

  const result = streamText({
    model: 'openai/gpt-4o',

    // System prompt for RAG behavior
    system: `You are a helpful assistant. Check your knowledge base before answering any questions.
    Only respond to questions using information from tool calls.
    If no relevant information is found in the tool calls, respond, "Sorry, I don't know."`,

    // Limit to 5 steps (prevents infinite tool loops)
    stopWhen: stepCountIs(5),

    tools: {
      // Add content to knowledge base
      addResource: tool({
        description: `Add a resource to your knowledge base.
          If the user provides a random piece of knowledge unprompted, use this tool without asking for confirmation.`,
        inputSchema: z.object({
          content: z.string().describe('The content to add to the knowledge base'),
        }),
        execute: async ({ content }) => createResource({ content }),
      }),

      // RAG lookup
      getInformation: tool({
        description: `Get information from your knowledge base to answer questions.`,
        inputSchema: z.object({
          question: z.string().describe('The user's question'),
        }),
        execute: async ({ question }) => findRelevantContent(question),
      }),
    },
  });

  return result.toUIMessageStreamResponse();
}
```

**Tool Flow:**
1. User asks question
2. LLM calls `getInformation` tool with question
3. `findRelevantContent` embeds question → vector search
4. Returns top 4 similar chunks
5. LLM answers using retrieved context

### 5. Resource Creation (`lib/actions/resources.ts`)

```typescript
import { generateEmbeddings } from '@/lib/ai/embedding';
import { db } from '@/lib/db';
import { embeddings } from '@/lib/db/schema/embeddings';
import { nanoid } from 'nanoid';

export async function createResource({ content }: { content: string }) {
  // Generate embeddings for all chunks
  const data = await generateEmbeddings(content);

  // Insert each chunk with its embedding
  const inserted = await Promise.all(
    data.map(async ({ content, embedding }) => {
      const id = nanoid();
      await db.insert(embeddings).values({ id, content, embedding });
      return { id, content };
    })
  );

  return inserted;
}
```

---

## RAG Workflow

```
User: "What is the capital of France?"
     ↓
┌─────────────────────────────────────────┐
│ 1. LLM receives question                │
│ 2. LLM decides to call getInformation   │
│ 3. Tool executes: findRelevantContent() │
│    - Embed query → [0.123, -0.456, ...] │
│    - Cosine similarity search in Neon   │
│    - Returns: ["Paris is the capital..."]│
│ 4. LLM receives context + original query│
│ 5. LLM generates answer from context    │
└─────────────────────────────────────────┘
     ↓
"According to my knowledge base, Paris is the capital of France."
```

---

## Database Migration

```sql
-- lib/db/schema/embeddings.ts defines the table
-- Run migration with: pnpm db:migrate

-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Create embeddings table
CREATE TABLE embeddings (
  id TEXT PRIMARY KEY,
  content TEXT NOT NULL,
  embedding vector(1536) NOT NULL,
  metadata TEXT
);

-- Create index for faster similarity search
CREATE INDEX ON embeddings USING hnsw (embedding vector_cosine_ops);
```

---

## Vercel AI Gateway

### Configuration

The AI Gateway provides a unified API for multiple model providers:

```
https://gateway.vercel.app/openai/text-embedding-ada-002
https://gateway.vercel.app/openai/gpt-4o
```

### Benefits

1. **Unified API** - Single endpoint for all providers
2. **Caching** - Automatic response caching
3. **Rate Limiting** - Built-in rate limits
4. **Fallbacks** - Automatic provider failover
5. **Analytics** - Usage tracking and monitoring

### OIDC Authentication

For serverless functions, Vercel uses OIDC tokens:

```typescript
// .env.local (pulled from Vercel)
VERCEL_OIDC_TOKEN=...  // Auto-refreshed every 12h
```

**Note:** If running locally without `vc dev`, refresh tokens manually:
```bash
vercel env pull  # Run every 12 hours
```

---

## File Structure

```
ai-gateway-embeddings-demo/
├── app/
│   ├── api/
│   │   └── chat/
│   │       └── route.ts       # Chat API with RAG tools
│   ├── layout.tsx
│   └── page.tsx               # Chat UI
├── lib/
│   ├── ai/
│   │   └── embedding.ts       # Embedding generation + search
│   ├── actions/
│   │   └── resources.ts       # Resource creation
│   ├── db/
│   │   ├── index.ts           # Drizzle db instance
│   │   ├── migrate.ts         # Migration runner
│   │   └── schema/
│   │       ├── embeddings.ts  # Embeddings table
│   │       └── resources.ts   # Resources table
│   └── utils.ts
├── components/
│   └── ui/                    # shadcn/ui components
├── drizzle.config.ts
├── package.json
└── README.md
```

---

## Commands

```bash
# Install dependencies
pnpm i

# Run database migrations
pnpm db:migrate
pnpm db:push

# Development server
pnpm dev

# Or use Vercel CLI
vc dev
```

---

## Environment Variables

```bash
# .env.example
OPENAI_API_KEY=sk-...
NEON_DATABASE_URL=postgresql://...
VERCEL_PROJECT_ID=...
```

---

## Rust Implementation Considerations

### 1. Embedding Client

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

async fn generate_embeddings(client: &Client, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
    let response = client
        .post("https://api.openai.com/v1/embeddings")
        .json(&EmbeddingRequest {
            model: "text-embedding-ada-002".to_string(),
            input: texts,
        })
        .send()
        .await?
        .json::<EmbeddingResponse>()
        .await?;

    Ok(response.data.into_iter()
        .map(|d| d.embedding)
        .collect())
}
```

### 2. PostgreSQL + pgvector

```rust
use sqlx::postgres::PgPool;
use sqlx::FromRow;

#[derive(FromRow)]
struct EmbeddingRow {
    id: String,
    content: String,
    similarity: f32,
}

async fn find_relevant_content(
    pool: &PgPool,
    query_embedding: Vec<f32>,
) -> Result<Vec<EmbeddingRow>> {
    // Convert Vec<f32> to pgvector format
    let embedding_str = format!(
        "[{}]",
        query_embedding.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",")
    );

    let results = sqlx::query_as::<_, EmbeddingRow>(
        r#"
        SELECT id, content, 1 - (embedding <=> $1::vector) AS similarity
        FROM embeddings
        WHERE 1 - (embedding <=> $1::vector) > 0.5
        ORDER BY similarity DESC
        LIMIT 4
        "#
    )
    .bind(&embedding_str)
    .fetch_all(pool)
    .await?;

    Ok(results)
}
```

### 3. Cosine Similarity in Rust

```rust
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    dot_product / (magnitude_a * magnitude_b)
}
```

### 4. Text Chunking Strategies

```rust
// Sentence-based chunking (like the TypeScript version)
fn chunk_by_sentences(text: &str) -> Vec<String> {
    text.split('.')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// Token-based chunking (better for embeddings)
fn chunk_by_tokens(text: &str, max_tokens: usize) -> Vec<String> {
    // Use tiktoken for tokenization
    let tokens: Vec<&str> = text.split_whitespace().collect();
    tokens
        .chunks(max_tokens)
        .map(|chunk| chunk.join(" "))
        .collect()
}

// Overlapping chunks for better context
fn chunk_with_overlap(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut chunks = Vec::new();

    for i in (0..words.len()).step_by(chunk_size - overlap) {
        let end = (i + chunk_size).min(words.len());
        chunks.push(words[i..end].join(" "));
    }

    chunks
}
```

---

## Key Takeaways

1. **Simple RAG Pattern** - Embed → Store → Search → Retrieve
2. **AI Gateway** - Unified embeddings API across providers
3. **pgvector** - Efficient vector similarity search in PostgreSQL
4. **Tool-Based RAG** - LLM triggers retrieval via tools
5. **Sentence Chunking** - Split by sentences for granular retrieval
6. **Similarity Threshold** - Filter results > 50% similarity

---

## See Also

- [Vercel AI Gateway](https://vercel.com/ai-gateway)
- [Drizzle ORM](https://orm.drizzle.team/)
- [Neon + pgvector](https://neon.tech/docs/extensions/pgvector)
- [Main Vercel Labs Exploration](./exploration.md)
