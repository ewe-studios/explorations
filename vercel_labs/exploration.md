# Vercel Labs AI Projects - Comprehensive Exploration

## Overview

Vercel Labs maintains a collection of AI-focused projects demonstrating modern patterns for building AI applications, agents, and tools. This exploration covers the key projects and their architectures, with a focus on how to reproduce functionality in Rust.

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.VarcelLabs`

**Output Directory:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/vercel_labs`

---

## Project Inventory

| Project | Description | Key Technologies |
|---------|-------------|------------------|
| [agent-browser](#agent-browser) | Headless browser automation CLI for AI agents | Rust CLI + Node.js/Playwright |
| [agent-skills](#agent-skills) | Claude skills for Vercel deployments | TypeScript, Vercel API |
| [ai-facts](#ai-facts) | Real-time audio fact checking | Next.js, AI SDK, Deepgram, Perplexity |
| [ai-gateway-embeddings-demo](#ai-gateway-embeddings-demo) | RAG with embeddings via AI Gateway | Next.js, AI SDK, Neon, Drizzle |
| [ai-sdk-reasoning-starter](#ai-sdk-reasoning-starter) | Chatbot with reasoning models | Next.js, AI SDK, Anthropic/Fireworks |
| [bash-tool](#bash-tool) | Sandbox bash execution for AI agents | TypeScript, just-bash, @vercel/sandbox |
| [lead-agent](#lead-agent) | Lead qualification with workflows | Next.js, Workflow DevKit, Slack |
| [deep-research-server](#deep-research-server) | MCP server for ChatGPT | Next.js, MCP, OpenAI Vector Store |

---

## Core Architecture Patterns

### 1. AI SDK Integration

All projects use the [Vercel AI SDK](https://ai-sdk.dev/) as the unified interface for LLM interactions:

```typescript
import { streamText, generateObject, tool, Agent } from 'ai';

// Streaming text response
const result = streamText({
  model: 'anthropic/claude-3-7-sonnet',
  messages,
  tools: { ... }
});

// Structured output
const { object } = await generateObject({
  model: 'openai/gpt-4',
  schema: mySchema,
  prompt: '...'
});

// Agent with tools
const agent = new Agent({
  model: 'openai/gpt-4',
  tools: { search, fetch, analyze },
  stopWhen: stepCountIs(20)
});
```

**Key AI SDK Features Used:**
- `streamText` - Streaming LLM responses
- `generateObject` - Structured JSON output with Zod schemas
- `tool()` - Tool definition helper
- `Agent` - Autonomous agent with tool loop
- `smoothStream` - Word-by-word streaming
- `extractReasoningMiddleware` - For reasoning models (DeepSeek-R1)

### 2. Multi-Provider Model Support

```typescript
import { anthropic } from '@ai-sdk/anthropic';
import { fireworks } from '@ai-sdk/fireworks';
import { groq } from '@ai-sdk/groq';
import { customProvider, wrapLanguageModel, extractReasoningMiddleware } from 'ai';

export const myProvider = customProvider({
  languageModels: {
    'sonnet-3.7': anthropic('claude-3-7-sonnet-20250219'),
    'deepseek-r1': wrapLanguageModel({
      middleware: extractReasoningMiddleware({ tagName: 'think' }),
      model: fireworks('accounts/fireworks/models/deepseek-r1'),
    }),
  },
});
```

### 3. Tool Pattern for AI Agents

```typescript
const searchTool = tool({
  description: 'Search the web for information',
  inputSchema: z.object({
    query: z.string().describe('Search query'),
    category: z.enum(['news', 'paper', 'github']),
  }),
  execute: async ({ query, category }) => {
    const result = await exa.searchAndContents(query, { category });
    return result;
  },
});
```

### 4. Workflow-Based Durable Execution

Using Workflow DevKit for multi-step processes:

```typescript
// workflows/inbound/index.ts
export const workflowInbound = async (data: FormSchema) => {
  'use workflow';

  const research = await stepResearch(data);
  const qualification = await stepQualify(data, research);

  if (qualification.category === 'QUALIFIED') {
    const email = await stepWriteEmail(research, qualification);
    await stepHumanFeedback(research, email, qualification);
  }
};
```

---

## Project Deep Dives

### Agent-Browser

**Purpose:** Headless browser automation CLI designed specifically for AI agents.

**Architecture:**
- **Rust CLI** - Fast native binary for command parsing
- **Node.js Daemon** - Manages Playwright browser instance
- **IPC Communication** - JSON protocol between CLI and daemon

**Key Features:**
- Ref-based element selection from accessibility snapshots
- Session isolation for parallel agent runs
- CDP (Chrome DevTools Protocol) support for existing browsers
- Screencast streaming via WebSocket
- Scoped HTTP headers for authenticated requests

**Snapshot with Refs:**
```
- heading "Example Domain" [ref=e1] [level=1]
- button "Submit" [ref=e2]
- textbox "Email" [ref=e3]
```

**Usage Pattern:**
```bash
agent-browser open example.com
agent-browser snapshot -i --json    # AI parses tree
agent-browser click @e2             # Click by ref
agent-browser fill @e3 "test@test.com"
```

[See: agent-browser-exploration.md](./agent-browser-exploration.md)

---

### Agent-Skills

**Purpose:** Claude skills for instant Vercel deployments.

**Features:**
- No authentication required
- Auto-detects 40+ frameworks
- Returns preview and claim URLs
- Handles static HTML projects

[See: agent-skills-exploration.md](./agent-skills-exploration.md)

---

### AI Facts

**Purpose:** Real-time fact checking on spoken statements.

**Architecture:**
1. Deepgram processes audio stream → transcribed text
2. Text split into statements on sentence boundaries
3. Each statement sent to OpenAI + Perplexity for verification
4. Results displayed with validity status and explanations

**Tech Stack:**
- Next.js App Router
- AI SDK for LLM interactions
- Deepgram for audio transcription
- OpenAI + Perplexity for cross-referencing

[See: ai-facts-exploration.md](./ai-facts-exploration.md)

---

### AI Gateway Embeddings Demo

**Purpose:** Demonstrate RAG (Retrieval Augmented Generation) using Vercel AI Gateway.

**Architecture:**
1. User pastes facts/docs → chunks text
2. Generate embeddings via AI Gateway (`openai/text-embedding-ada-002`)
3. Store vectors in Neon (PostgreSQL)
4. Query: embed question → cosine similarity search → retrieve chunks
5. LLM answers using only retrieved context

**Key Code:**
```typescript
// lib/ai/embedding.ts
export const findRelevantContent = async (userQuery: string) => {
  const userQueryEmbedded = await generateEmbedding(userQuery);
  const similarity = sql<number>`1 - (${cosineDistance(
    embeddings.embedding,
    userQueryEmbedded,
  )})`;
  return await db
    .select({ name: embeddings.content, similarity })
    .from(embeddings)
    .where(gt(similarity, 0.5))
    .orderBy(t => desc(t.similarity))
    .limit(4);
};
```

**Tools:**
- `addResource` - Add content to knowledge base
- `getInformation` - RAG lookup before answering

[See: ai-gateway-embeddings-demo-exploration.md](./ai-gateway-embeddings-demo-exploration.md)

---

### AI SDK Reasoning Starter

**Purpose:** Chatbot template demonstrating reasoning model integration.

**Features:**
- Next.js 15 App Router
- AI SDK with multiple providers
- Reasoning model support (DeepSeek-R1 with `<think>` tags)
- Anthropic Claude 3.7 with thinking budget

**Model Configuration:**
```typescript
providerOptions:
  selectedModelId === "sonnet-3.7"
    ? {
        anthropic: {
          thinking: isReasoningEnabled
            ? { type: "enabled", budgetTokens: 12000 }
            : { type: "disabled", budgetTokens: 12000 },
        },
      }
    : {}
```

[See: ai-sdk-reasoning-starter-exploration.md](./ai-sdk-reasoning-starter-exploration.md)

---

### Bash-Tool

**Purpose:** Generic bash tool for AI agents with sandbox execution.

**Architecture:**
- **Sandbox Abstraction** - Supports just-bash (in-memory) or @vercel/sandbox (full VM)
- **Three Tools:** `bash`, `readFile`, `writeFile`
- **Batch File Writing** - Streams files in batches to avoid memory issues
- **Tool Prompt Generation** - Auto-generates filesystem context for agents

**Usage:**
```typescript
import { createBashTool } from "bash-tool";
import { Agent, stepCountIs } from "ai";

const { tools } = await createBashTool({
  files: { "src/index.ts": "export const hello = 'world';" },
});

const agent = new Agent({
  model: model,
  tools,
  stopWhen: stepCountIs(20),
});
```

**Sandbox Options:**
1. **just-bash** - In-memory filesystem (default)
2. **@vercel/sandbox** - Full VM isolation
3. **Custom** - Implement Sandbox interface

[See: bash-tool-exploration.md](./bash-tool-exploration.md)

---

### Lead Agent

**Purpose:** Inbound lead qualification and research agent with human-in-the-loop.

**Architecture:**
```
User submits form
     ↓
start(workflow) ← Workflow DevKit
     ↓
Research agent ← AI SDK Agent class
     ↓
Qualify lead ← generateObject with Zod schema
     ↓
Generate email ← generateText
     ↓
Slack approval ← Human-in-the-loop
     ↓
Send email
```

**Tools in Research Agent:**
- `search` - Web search via Exa.ai
- `fetchUrl` - URL content extraction
- `crmSearch` - CRM lookup (Salesforce, HubSpot)
- `techStackAnalysis` - Domain tech analysis
- `queryKnowledgeBase` - Vector store lookup

**Human Feedback via Slack:**
```typescript
export const stepHumanFeedback = async (research, email, qualification) => {
  'use step';
  const slackMessage = await sendSlackMessageWithButtons(
    channel,
    `*New Lead: ${qualification.category}*\n\n${email}\n\n[Approve][Reject]`
  );
  return slackMessage;
};
```

[See: lead-agent-exploration.md](./lead-agent-exploration.md)

---

### Deep Research Server

**Purpose:** MCP (Model Context Protocol) server for ChatGPT Deep Research.

**Features:**
- Search tool - Semantic search via OpenAI Vector Store
- Fetch tool - Document retrieval by ID
- Sample data - 5 pre-loaded documents
- MCP compliant

[See: deep-research-server-exploration.md](./deep-research-server-exploration.md)

---

## AI Agent Architecture Summary

### Common Patterns Across Projects

1. **Tool-Based Agents**
   - Tools defined with `tool()` from AI SDK
   - Zod schemas for input validation
   - Execute async functions with LLM-parsed inputs

2. **Multi-Step Workflows**
   - Durable execution via Workflow DevKit
   - Steps marked with `'use step'` directive
   - Conditional branching based on intermediate results

3. **Human-in-the-Loop**
   - Slack integration for approvals
   - Webhook callbacks for async feedback
   - State persistence between steps

4. **RAG (Retrieval Augmented Generation)**
   - Embed content with AI Gateway
   - Store in vector database (Neon, Pinecone, etc.)
   - Cosine similarity search for retrieval
   - Inject context into LLM prompts

5. **Streaming Responses**
   - `streamText` for progressive output
   - `smoothStream` for word-by-word chunks
   - `sendReasoning: true` for reasoning model transparency

---

## Rust Implementation Considerations

### Key Challenges for Rust Reproduction

1. **AI SDK Equivalent**
   - Need async LLM client library with streaming
   - Structured output parsing (like Zod → Rust structs)
   - Tool definition abstraction

2. **Playwright Integration**
   - Use `playwright-rs` or direct CDP implementation
   - ARIA snapshot parsing
   - Element ref tracking

3. **Workflow Engine**
   - Durable execution state machine
   - Step checkpointing
   - Webhook correlation

4. **Sandbox Execution**
   - Use containers (Docker) or WASM sandboxes
   - File system isolation
   - Resource limits

5. **Vector Similarity**
   - Use `pgvector` for PostgreSQL
   - Or native Rust: `faiss`, `usearch`, `hnswlib`

[See: rust-revision.md](./rust-revision.md) for detailed Rust implementation guide.

---

## Related Projects

### AI SDK Ecosystem
- [@ai-sdk/anthropic](https://npmjs.com/package/@ai-sdk/anthropic)
- [@ai-sdk/openai](https://npmjs.com/package/@ai-sdk/openai)
- [@ai-sdk/fireworks](https://npmjs.com/package/@ai-sdk/fireworks)
- [@ai-sdk/groq](https://npmjs.com/package/@ai-sdk/groq)

### Supporting Infrastructure
- [Workflow DevKit](https://useworkflow.dev/) - Durable workflows
- [Vercel AI Gateway](https://vercel.com/ai-gateway) - Unified AI API
- [@vercel/sandbox](https://vercel.com/docs/vercel-sandbox) - Sandboxed execution
- [just-bash](https://npmjs.com/package/just-bash) - Lightweight bash sandbox

### External Services
- [Exa.ai](https://exa.ai/) - Web search API
- [Deepgram](https://deepgram.com/) - Audio transcription
- [Perplexity](https://perplexity.ai/) - AI search
- [Neon](https://neon.tech/) - Serverless PostgreSQL with pgvector

---

## Conclusion

Vercel Labs demonstrates a comprehensive approach to AI application development:

1. **Unified AI Interface** - AI SDK abstracts multiple LLM providers
2. **Agent Architecture** - Tool-based agents with autonomous execution
3. **Durable Workflows** - Multi-step processes with state persistence
4. **Human Oversight** - Slack integration for approvals
5. **Knowledge Integration** - RAG for domain-specific context
6. **Browser Automation** - Ref-based deterministic element selection

The patterns shown here provide a blueprint for building production AI applications with proper error handling, state management, and human oversight.
