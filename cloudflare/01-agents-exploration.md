---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/agents
repository: git@github.com:cloudflare/agents.git
explored_at: 2026-03-29
language: TypeScript
framework: Cloudflare Workers, Durable Objects
---

# Cloudflare Agents SDK - Exploration

## Overview

Cloudflare Agents is a **stateful AI agent framework** built on Cloudflare Workers and Durable Objects. It provides persistent, stateful execution environments for agentic workloads where each agent has its own state, storage, and lifecycle.

### Key Value Proposition

- **Persistent State**: Each agent maintains state across invocations
- **Auto-Scaling**: Runs on Cloudflare's edge network - millions of agents possible
- **Cost-Effective**: Agents hibernate when idle, cost nothing when inactive
- **Real-Time**: Built-in WebSocket support for live client synchronization
- **AI-Native**: First-class support for LLM calls, tool execution, streaming

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Cloudflare Edge                              │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │   Agent 1    │  │   Agent 2    │  │   Agent N    │         │
│  │  (DO-001)    │  │  (DO-002)    │  │  (DO-00N)    │         │
│  │  ┌────────┐  │  │  ┌────────┐  │  │  ┌────────┐  │         │
│  │  │ State  │  │  │  │ State  │  │  │  │ State  │  │         │
│  │  │ Storage│  │  │  │ Storage│  │  │  │ Storage│  │         │
│  │  └────────┘  │  │  └────────┘  │  │  └────────┘  │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Durable Objects Storage Layer               │   │
│  │              (SQLite per DO, persisted to R2)           │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         │                    │                    │
    ┌────▼────┐         ┌────▼────┐         ┌────▼────┐
    │ Client  │         │ Client  │         │ Client  │
    │ (React) │         │ (Mobile)│         │  (CLI)  │
    └─────────┘         └─────────┘         └─────────┘
```

## Monorepo Structure

```
cloudflare/agents/
├── packages/
│   ├── agents/           # Core SDK - Agent class, routing, state, scheduling
│   ├── ai-chat/          # @cloudflare/ai-chat - Higher-level AI chat abstraction
│   ├── hono-agents/      # Hono framework integration
│   └── codemode/         # @cloudflare/codemode - Experimental LLM code generation
├── examples/
│   ├── playground/       # Main showcase - all SDK features in one UI
│   ├── mcp/              # MCP server example
│   ├── mcp-client/       # MCP client example
│   ├── workflows/        # Workflow orchestration examples
│   └── ... (~20 examples total)
├── experimental/         # Work-in-progress experiments
├── site/
│   ├── agents/           # agents.cloudflare.com (Astro)
│   └── ai-playground/    # Workers AI playground (React + Vite)
├── guides/
│   ├── anthropic-patterns/   # Sequential, routing, parallel, orchestrator
│   └── human-in-the-loop/    # Approval workflows with pause/resume
├── openai-sdk/           # Examples using OpenAI Agents SDK
├── docs/                 # Markdown docs for developers.cloudflare.com
├── design/               # Architecture and design decision records
└── scripts/              # Repo tooling
```

## Core Concepts

### 1. Agent Class

The base `Agent` class provides:

```typescript
import { Agent, callable } from "agents";

export type CounterState = { count: number };

export class CounterAgent extends Agent<Env, CounterState> {
  // Initial state when agent is first created
  initialState = { count: 0 };

  // Callable method - exposed as RPC endpoint
  @callable()
  increment(): number {
    this.setState({ count: this.state.count + 1 });
    return this.state.count;
  }

  // Another callable
  @callable()
  getCount(): number {
    return this.state.count;
  }

  // Lifecycle hook - called on agent wakeup
  async onBeforeSleep(): Promise<void> {
    console.log("Agent going to sleep");
  }
}
```

### 2. State Management

State is persisted to Durable Objects storage:

```typescript
// Get state
const currentState = this.state;

// Update state (triggers sync to connected clients)
this.setState({ count: newState });

// Partial update
this.patchState({ lastActive: Date.now() });
```

### 3. Client Integration

#### React Hook

```typescript
import { useAgent } from "agents/react";

function Counter() {
  const agent = useAgent<CounterAgent, CounterState>({
    agent: "CounterAgent",
    onStateUpdate: (state) => setCount(state.count)
  });

  return (
    <button onClick={() => agent.stub.increment()}>
      Increment
    </button>
  );
}
```

#### Vanilla JS Client

```typescript
import { AgentClient } from "agents/client";

const client = new AgentClient("CounterAgent");
const result = await client.stub.increment();
```

## Features

### Callable Methods

Type-safe RPC via decorators:

```typescript
@callable()
async chat(message: string): Promise<string> {
  const response = await this.ai.chat.completions.create({
    model: "@cf/meta/llama-3-8b-instruct",
    messages: [{ role: "user", content: message }]
  });
  return response.choices[0].message.content;
}
```

### Scheduling

One-time, recurring, and cron-based tasks:

```typescript
@callable()
scheduleReminder(time: Date, message: string) {
  this.scheduler.runAt(time, async () => {
    await this.sendNotification(message);
  });
}

@callable()
scheduleDailyReport(hour: number) {
  this.scheduler.cron(`${hour} * * * *`, async () => {
    await this.generateReport();
  });
}
```

### WebSockets

Real-time bidirectional communication:

```typescript
export class ChatAgent extends Agent<Env, ChatState> {
  onConnect(session: ClientSession) {
    console.log("Client connected:", session.id);
    this.broadcast({ type: "user_joined", sessionId: session.id });
  }

  onMessage(session: ClientSession, message: string) {
    // Broadcast to all connected clients
    this.broadcast({ type: "message", from: session.id, text: message });
  }

  onDisconnect(session: ClientSession) {
    console.log("Client disconnected:", session.id);
  }
}
```

### AI Chat

Built-in AI chat with persistent messages:

```typescript
import { Agent, AIChatMessage } from "agents";

export class AssistantAgent extends Agent<Env, AssistantState> {
  @callable()
  async chat(message: string): Promise<string> {
    // Get conversation history
    const history = await this.aiChat.getMessages();

    // Add user message
    await this.aiChat.addUserMessage(message);

    // Stream response from AI
    const response = await this.aiChat.streamResponse({
      model: "@cf/meta/llama-3-8b-instruct",
      systemPrompt: "You are a helpful assistant."
    });

    return response;
  }
}
```

### MCP (Model Context Protocol)

Act as MCP server or connect as MCP client:

```typescript
import { Agent, MCPClient } from "agents";

export class ToolAgent extends Agent<Env, ToolState> {
  mcpClient: MCPClient;

  async onBeforeSleep() {
    await this.mcpClient.disconnect();
  }

  @callable()
  async useTool(toolName: string, args: any) {
    const result = await this.mcpClient.callTool(toolName, args);
    return result;
  }
}
```

### Workflows

Durable multi-step tasks with human-in-the-loop:

```typescript
import { WorkflowStep } from "agents";

export class ApprovalAgent extends Agent<Env, ApprovalState> {
  @callable()
  async submitExpense(amount: number, description: string) {
    // Step 1: Validate
    await this.workflow.run("validate", async () => {
      if (amount > 1000) {
        throw new Error("Amount exceeds limit");
      }
    });

    // Step 2: Wait for human approval
    const approved = await this.workflow.waitForApproval("manager_approval");
    if (!approved) {
      return { status: "rejected" };
    }

    // Step 3: Process payment
    await this.workflow.run("process_payment", async () => {
      await this.pay(amount);
    });

    return { status: "approved" };
  }
}
```

### Email Routing

Receive and respond to emails:

```typescript
export class EmailAgent extends Agent<Env, EmailState> {
  async onEmail(email: EmailMessage) {
    // Process incoming email
    const response = await this.ai.generateResponse(email.body);
    await this.sendEmail({
      to: email.from,
      subject: `Re: ${email.subject}`,
      body: response
    });
  }
}
```

### Code Mode (Experimental)

LLM generates executable TypeScript instead of tool calls:

```typescript
import { CodeModeExecutor } from "@cloudflare/codemode";

export class CodeAgent extends Agent<Env, CodeState> {
  @callable()
  async executeTask(task: string) {
    const executor = new CodeModeExecutor(this.ai);
    const code = await executor.generateCode(task);
    const result = await executor.execute(code);
    return result;
  }
}
```

### SQL Queries

Direct SQLite queries via Durable Objects:

```typescript
export class DataAgent extends Agent<Env, DataState> {
  @callable()
  async getUser(userId: string) {
    const result = await this.sql`
      SELECT * FROM users WHERE id = ${userId}
    `;
    return result.one();
  }

  @callable()
  async createUser(name: string, email: string) {
    await this.sql`
      INSERT INTO users (name, email) VALUES (${name}, ${email})
    `;
  }
}
```

## Protocol

The Agents protocol defines communication between clients and agents:

```
┌─────────────────────────────────────────────────────────────┐
│                    Message Types                             │
├─────────────────────────────────────────────────────────────┤
│ 1. CONNECT        - Client → Agent (establish session)      │
│ 2. SUBSCRIBE      - Client → Agent (subscribe to state)     │
│ 3. STATE_UPDATE   - Agent → Client (state changed)          │
│ 4. CALL           - Client → Agent (invoke method)          │
│ 5. CALL_RESULT    - Agent → Client (method result)          │
│ 6. DISCONNECT     - Client → Agent (end session)            │
└─────────────────────────────────────────────────────────────┘
```

### Message Format

```typescript
interface AgentMessage {
  type: MessageType;
  id: string;           // Unique message ID
  agent: string;        // Agent type name
  sessionId: string;    // Client session ID
  payload: unknown;     // Message-specific data
}
```

## Testing

Uses vitest with `@cloudflare/vitest-pool-workers`:

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { CounterAgent } from "./server";

describe("CounterAgent", () => {
  let agent: CounterAgent;

  beforeEach(async () => {
    agent = await createAgent<CounterAgent>("CounterAgent");
  });

  it("should increment count", async () => {
    const result = await agent.increment();
    expect(result).toBe(1);
  });

  it("should persist state", async () => {
    await agent.increment();
    await agent.increment();

    const newAgent = await getAgent<CounterAgent>("CounterAgent");
    expect(newAgent.state.count).toBe(2);
  });
});
```

## Build & Development

```bash
npm install        # Install all workspaces
npm run build      # Build all packages (via Nx)
npm run check      # Full CI: format + lint + typecheck
npm run test       # Run all tests

# Run single example
cd examples/playground
npm run dev        # Vite dev server + Workers runtime
```

### CI/CD

- **Changesets**: Required for package changes
- **Nx**: Task orchestration with caching
- **Affected builds**: Only rebuild changed packages

```bash
npx changeset              # Create changeset
npx nx affected -t build   # Build affected packages
npx nx affected -t test    # Test affected packages
```

## Key Design Decisions

### 1. Durable Objects as Agent Runtime

**Why**: DO provides exactly the right abstraction:
- Per-agent isolation
- Persistent storage built-in
- Automatic hibernation
- Global distribution

**Trade-offs**:
- Cold start latency (~100-500ms)
- Limited to Workers runtime capabilities

### 2. @callable() Decorator Pattern

**Why**: Familiar RPC model for TypeScript developers:
- Type-safe by default
- Auto-generates client stubs
- No schema definitions needed

**Trade-offs**:
- Tightly coupled to TypeScript
- Limited cross-language support

### 3. Real-Time State Sync

**Why**: Critical for AI agent UX:
- Users see streaming responses
- Multiple clients stay in sync
- No polling required

**Implementation**:
- WebSocket-based push
- Optimistic UI updates
- Conflict resolution via last-write-wins

### 4. Workers AI Integration

**Why**: Native Cloudflare integration:
- No external API keys needed
- Lower latency (edge inference)
- Cost-effective for high volume

**Supported Models**:
- Llama 3 (8B, 70B)
- Mistral
- Gemma

## Production Considerations

### Scaling

- Each Durable Object is single-threaded
- Hot agents can become bottlenecks
- Solution: Shard by user ID, session ID, or custom key

### Cost

- DO pricing: $0.000003/GB-second + $0.000001/GB-month storage
- Idle agents cost nothing (hibernation)
- Active agents: ~$0.00001/hour

### Monitoring

```typescript
export class MonitoredAgent extends Agent<Env, State> {
  async onBeforeSleep() {
    // Log metrics before hibernation
    this.env.METRICS.log("agent_sleep", {
      agentId: this.id,
      uptime: Date.now() - this.startedAt,
      stateSize: JSON.stringify(this.state).length
    });
  }
}
```

### Error Handling

```typescript
@callable()
async safeOperation() {
  try {
    return await this.riskyOperation();
  } catch (error) {
    this.setErrorState(error);
    await this.notifyAdmin(error);
    throw error;
  }
}
```

## Related Deep Dives

- [Cloudflare AI - Deep Dive](../ai/01-ai-deep-dive.md)
- [Cloudflare Capnweb - Deep Dive](../capnweb/01-capnweb-deep-dive.md)
- [Cloudflare Containers - Deep Dive](../containers/01-containers-deep-dive.md)
