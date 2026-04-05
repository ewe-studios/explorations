# Zero to Cloudflare Agents: Complete Guide

**Last Updated:** 2026-04-05

---

## Table of Contents

1. [Introduction](#introduction)
2. [What Are Cloudflare Agents?](#what-are-cloudflare-agents)
3. [Core Architecture](#core-architecture)
4. [Getting Started](#getting-started)
5. [Your First Agent](#your-first-agent)
6. [State Management](#state-management)
7. [Callable Methods](#callable-methods)
8. [Real-time Communication](#real-time-communication)
9. [Scheduling](#scheduling)
10. [AI Integration](#ai-integration)
11. [MCP Integration](#mcp-integration)
12. [Workflows](#workflows)
13. [Production Deployment](#production-deployment)
14. [Next Steps](#next-steps)

---

## Introduction

Cloudflare Agents is a TypeScript/JavaScript SDK for building **stateful, persistent AI agents** on Cloudflare's global network. Built on top of Durable Objects, Agents provide:

- **Persistent State** - Each agent maintains its own state that survives restarts
- **Real-time Sync** - State changes automatically sync to all connected clients
- **Callable Methods** - Type-safe RPC via `@callable()` decorator
- **Built-in Scheduling** - One-time, recurring, and cron-based tasks
- **AI-Native** - Direct integration with Workers AI and external LLM providers
- **MCP Support** - Act as MCP servers or connect as MCP clients
- **Workflow Engine** - Multi-step durable tasks with human-in-the-loop

Agents are **persistent execution environments** - they hibernate when idle and wake on demand. You can run millions of them (one per user, session, or game room) with zero cost when inactive.

---

## What Are Cloudflare Agents?

### The Problem Agents Solve

Traditional serverless functions are **stateless** - each request starts fresh. This creates problems for:

1. **Conversational AI** - Chat history must be loaded from external storage
2. **Real-time Collaboration** - Requires separate WebSocket infrastructure
3. **Session State** - Must persist to Redis/KV between requests
4. **Long-running Tasks** - Functions timeout after seconds

### The Agent Solution

Agents are **stateful execution environments** powered by Cloudflare Durable Objects:

```
┌─────────────────────────────────────────────────────────────┐
│                   Cloudflare Global Network                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   Agent A   │  │   Agent B   │  │   Agent C   │          │
│  │  (User 1)   │  │  (User 2)   │  │ (Game Room) │          │
│  │             │  │             │  │             │          │
│  │ ┌─────────┐ │  │ ┌─────────┐ │  │ ┌─────────┐ │          │
│  │ │  State  │ │  │ │  State  │ │  │ │  State  │ │          │
│  │ │  + DO   │ │  │ │  + DO   │ │  │ │  + DO   │ │          │
│  │ │  Storage│ │  │ │  Storage│ │  │ │  Storage│ │          │
│  │ └─────────┘ │  │ └─────────┘ │  │ └─────────┘ │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
│         ↑                ↑                ↑                  │
│         │                │                │                  │
│  ┌──────┴────────────────┴────────────────┴──────┐          │
│  │           WebSocket Connections               │          │
│  └───────────────────────────────────────────────┘          │
└─────────────────────────────────────────────────────────────┘
```

Each agent has:
- **Own memory** - In-memory state + Durable Objects storage
- **Own lifecycle** - Wakes on request, hibernates when idle
- **Own connections** - WebSocket connections to clients

### Key Characteristics

| Feature | Description |
|---------|-------------|
| **Persistent State** | Survives restarts, syncs to all connected clients |
| **Callable Methods** | Type-safe RPC via `@callable()` decorator |
| **Scheduling** | One-time, recurring, and cron-based tasks |
| **WebSockets** | Real-time bidirectional communication |
| **AI Chat** | Message persistence, resumable streaming, tool execution |
| **MCP** | Act as MCP servers or connect as MCP clients |
| **Workflows** | Durable multi-step tasks with human-in-the-loop |
| **Email** | Receive and respond via Cloudflare Email Routing |
| **Code Mode** | LLMs generate executable TypeScript |
| **SQL** | Direct SQLite queries via Durable Objects |

---

## Core Architecture

### The Agent Class

At the heart of the SDK is the `Agent` class:

```typescript
import { Agent } from "agents";

export class MyAgent extends Agent<Env, StateType> {
  // Initial state when agent is first created
  initialState: StateType = { /* ... */ };
  
  // Current state (automatically persisted)
  state: StateType;
  
  // Called when agent is first instantiated
  onInit(): void {
    // Setup code
  }
  
  // Called when state changes
  onStateUpdate(newState: StateType): void {
    // React to state changes
  }
  
  // Called when agent is about to hibernate
  onHibernate(): void {
    // Cleanup code
  }
}
```

### Durable Objects Integration

Agents run inside **Durable Objects (DOs)** - Cloudflare's stateful primitive:

```typescript
// Each Agent class maps to a Durable Object class
export class MyAgentDO extends DurableObject {
  async fetch(request: Request) {
    // Agent handles the request
  }
  
  async alarm() {
    // Scheduled task execution
  }
}
```

The `@callable()` decorator wraps DO methods to provide:
- Automatic serialization
- Client RPC routing
- Type safety

### Request Routing

The SDK provides `routeAgentRequest` to handle HTTP/WebSocket requests:

```typescript
export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext) {
    return (
      (await routeAgentRequest(request, env)) ??
      new Response("Not found", { status: 404 })
    );
  }
};
```

This function:
1. Parses the URL to find the agent name and instance
2. Gets or creates the Durable Object
3. Forwards the request to the agent
4. Handles WebSocket upgrades for real-time connections

---

## Getting Started

### Prerequisites

- **Node.js 24+** required
- **npm** (uses npm workspaces)
- **Cloudflare account** (free tier works)
- **Wrangler CLI** installed globally

```bash
npm install -g wrangler
wrangler login
```

### Quick Start: Create New Project

```bash
# Create from template
npm create cloudflare@latest -- --template cloudflare/agents-starter

# Or add to existing project
cd your-project
npm install agents
```

### Project Structure

```
my-agents-project/
├── src/
│   ├── index.ts          # Worker entry point
│   ├── agents/
│   │   └── counter.ts    # Agent definitions
│   └── types/
│       └── env.d.ts      # Environment types
├── wrangler.jsonc        # Worker configuration
├── tsconfig.json         # TypeScript config
└── package.json
```

### Configuration

`wrangler.jsonc`:

```jsonc
{
  "$schema": "node_modules/wrangler/config-schema.json",
  "name": "my-agents",
  "main": "src/index.ts",
  "compatibility_date": "2026-01-28",
  "compatibility_flags": ["nodejs_compat"],
  
  "durable_objects": {
    "bindings": [
      { "name": "COUNTER_AGENT", "class_name": "CounterAgent" }
    ]
  },
  
  "vars": {
    "ENVIRONMENT": "development"
  }
}
```

---

## Your First Agent

### Counter Agent Example

Let's build a counter agent with persistent state and real-time sync:

#### Server Code

```typescript
// src/agents/counter.ts
import { Agent, routeAgentRequest, callable } from "agents";

export type CounterState = { 
  count: number;
  history: number[];
};

export class CounterAgent extends Agent<Env, CounterState> {
  // Initial state when agent is first created
  initialState: CounterState = { count: 0, history: [] };

  @callable()
  increment(amount: number = 1): number {
    const newCount = this.state.count + amount;
    this.setState({ 
      count: newCount,
      history: [...this.state.history, newCount]
    });
    return this.state.count;
  }

  @callable()
  decrement(amount: number = 1): number {
    const newCount = this.state.count - amount;
    this.setState({ 
      count: newCount,
      history: [...this.state.history, newCount]
    });
    return this.state.count;
  }

  @callable()
  reset(): number {
    this.setState({ count: 0, history: [] });
    return 0;
  }

  @callable()
  getCount(): number {
    return this.state.count;
  }
}

// Export for Wrangler
export { CounterAgent as CounterAgentDO };
```

#### Worker Entry Point

```typescript
// src/index.ts
import { CounterAgent } from "./agents/counter";

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext) {
    return (
      (await routeAgentRequest(request, env)) ??
      new Response("Not found", { status: 404 })
    );
  }
};
```

#### Client Code (React)

```typescript
// src/client.tsx
import { useAgent } from "agents/react";
import { useState } from "react";
import type { CounterAgent, CounterState } from "./agents/counter";

function Counter() {
  const [count, setCount] = useState(0);

  const agent = useAgent<CounterAgent, CounterState>({
    agent: "CounterAgent",
    name: "my-counter",  // Instance name
    onStateUpdate: (state) => setCount(state.count)
  });

  return (
    <div>
      <span>Count: {count}</span>
      <button onClick={() => agent.stub.increment(1)}>+</button>
      <button onClick={() => agent.stub.decrement(1)}>-</button>
      <button onClick={() => agent.stub.reset()}>Reset</button>
    </div>
  );
}
```

### How It Works

1. **Client connects** - `useAgent` creates WebSocket connection
2. **Server receives** - `routeAgentRequest` routes to CounterAgent DO
3. **Agent instantiates** - Creates or resumes CounterAgent instance
4. **State syncs** - Current state sent to client on connect
5. **Method calls** - `increment()` called via RPC, state updates
6. **Broadcast** - New state sent to all connected clients

---

## State Management

### setState()

The primary way to update agent state:

```typescript
this.setState({ count: this.state.count + 1 });
```

This:
1. Merges new state with existing state (shallow merge)
2. Persists to Durable Object storage
3. Broadcasts to all connected clients
4. Triggers `onStateUpdate()` lifecycle hook

### State Types

State must be **serializable** (JSON-compatible):

```typescript
type AgentState = {
  // Primitives
  count: number;
  name: string;
  active: boolean;
  
  // Arrays
  items: string[];
  history: Array<{ timestamp: number; action: string }>;
  
  // Objects
  user: { id: string; email: string };
  settings: Record<string, unknown>;
  
  // Null/undefined (use null, undefined gets serialized as null)
  lastError: string | null;
};
```

### State Persistence

State is automatically persisted to Durable Object storage:

```typescript
export class MyAgent extends Agent<Env, State> {
  initialState: State = { count: 0 };
  
  onInit() {
    // State is already loaded here
    console.log("Initial state:", this.state);
  }
  
  onStateUpdate(newState: State) {
    // Called after setState() completes
    console.log("State changed:", newState);
  }
}
```

### Storage API

For data that shouldn't sync to clients:

```typescript
export class MyAgent extends Agent<Env, State> {
  async saveToStorage(key: string, value: unknown) {
    await this.ctx.storage.put(key, value);
  }
  
  async loadFromStorage(key: string): Promise<unknown> {
    return await this.ctx.storage.get(key);
  }
  
  async deleteFromStorage(key: string) {
    await this.ctx.storage.delete(key);
  }
  
  async listStorage(): Promise<string[]> {
    const cursor = this.ctx.storage.list();
    const keys: string[] = [];

    for await (const key of cursor) {
      keys.push(key);
    }
    return keys;
  }
}
```

---

## Callable Methods

### The @callable() Decorator

Methods decorated with `@callable()` can be called from clients:

```typescript
import { Agent, callable } from "agents";

export class MyAgent extends Agent<Env, State> {
  @callable()
  greet(name: string): string {
    return `Hello, ${name}!`;
  }
  
  @callable()
  async fetchData(url: string): Promise<unknown> {
    const response = await fetch(url);
    return response.json();
  }
}
```

### Client Calling

Clients call methods via `stub`:

```typescript
const agent = useAgent<MyAgent>({ agent: "MyAgent" });

// Call method
const greeting = await agent.stub.greet("World");

// With timeout
const data = await agent.stub.fetchData("https://api.example.com", {
  timeout: 5000  // 5 second timeout
});
```

### Streaming Methods

For long-running operations:

```typescript
@callable()
async *streamNumbers(count: number) {
  for (let i = 0; i < count; i++) {
    yield i;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
}

// Client
const agent = useAgent<MyAgent>({ agent: "MyAgent" });

const result = await agent.stub.streamNumbers(10, {
  stream: {
    onChunk: (chunk) => console.log("Received:", chunk),
    onDone: (final) => console.log("Done:", final),
    onError: (error) => console.error("Error:", error)
  }
});
```

### Method Visibility

Callable methods can be:
- **Public** (default) - Any client can call
- **Protected** - Only connected clients can call
- **Private** - Internal use only (not exposed)

```typescript
@callable({ visibility: "public" })
publicMethod() { /* ... */ }

@callable({ visibility: "protected" })
protectedMethod() { /* ... */ }

@callable({ visibility: "private" })
privateMethod() { /* ... */ }
```

---

## Real-time Communication

### WebSocket Connections

Agents automatically handle WebSocket upgrades for real-time communication:

```typescript
export class ChatAgent extends Agent<Env, ChatState> {
  initialState: ChatState = { messages: [], participants: 0 };
  
  onConnect(clientId: string) {
    // Called when client connects
    this.setState({ 
      participants: this.state.participants + 1 
    });
  }
  
  onDisconnect(clientId: string) {
    // Called when client disconnects
    this.setState({ 
      participants: this.state.participants - 1 
    });
  }
}
```

### useAgent Hook

React hook for real-time state sync:

```typescript
import { useAgent } from "agents/react";

function Component() {
  const agent = useAgent<ChatAgent, ChatState>({
    agent: "ChatAgent",
    name: "room-1",
    
    // Called when state updates from server
    onStateUpdate: (state, source) => {
      console.log("State updated from:", source); // "server" | "client"
    },
    
    // Called on connection errors
    onStateUpdateError: (error) => {
      console.error("State update failed:", error);
    },
    
    // Called when agent identity is received
    onIdentity: (name, agent) => {
      console.log("Connected to:", agent, name);
    }
  });
  
  return <div>Participants: {agent.state?.participants}</div>;
}
```

### Base Path Routing

For custom routing (e.g., session-based):

```typescript
// Client
const agent = useAgent({
  agent: "UserAgent",
  basePath: "user",  // Connects to /user instead of /agents/user-agent
  onIdentity: (name, agent) => {
    // Server-determined identity
    console.log("I am:", name);
  }
});

// Server
export default {
  async fetch(request: Request, env: Env) {
    // Route based on session
    const userId = getSessionUserId(request);
    const agent = await getAgentByName(env, "UserAgent", userId);
    return agent.fetch(request);
  }
};
```

---

## Scheduling

### Schedule Types

Agents support three scheduling patterns:

1. **One-time** - Execute at specific date/time
2. **Delayed** - Execute after delay in seconds
3. **Recurring** - Execute on cron schedule

### Schedule API

```typescript
import { Agent, schedule, callable } from "agents";

export class ReminderAgent extends Agent<Env, ReminderState> {
  initialState: ReminderState = { reminders: [] };
  
  @callable()
  async scheduleReminder(message: string, when: Date | string | number) {
    // Schedule one-time reminder
    await schedule({
      date: when,  // Date object or ISO string
      callback: "sendReminder",
      payload: { message }
    });
  }
  
  @callable()
  async scheduleDaily(message: string, hour: number, minute: number) {
    // Schedule recurring daily reminder
    await schedule({
      cron: `${minute} ${hour} * * *`,  // Cron syntax
      callback: "sendReminder",
      payload: { message }
    });
  }
  
  @callable()
  async scheduleDelayed(message: string, delaySeconds: number) {
    // Schedule delayed reminder
    await schedule({
      delay: delaySeconds,
      callback: "sendReminder",
      payload: { message }
    });
  }
  
  // Called by scheduler
  async sendReminder(payload: { message: string }) {
    console.log("Reminder:", payload.message);
    // Send email, push notification, etc.
  }
}
```

### Schedule Schema

Natural language scheduling with AI:

```typescript
import { generateObject } from "ai";
import { scheduleSchema, getSchedulePrompt } from "agents/schedule";

async function parseSchedule(userInput: string) {
  const result = await generateObject({
    model: someLLM,
    prompt: `${getSchedulePrompt({ date: new Date() })} Input: "${userInput}"`,
    schema: scheduleSchema,
    providerOptions: {
      openai: { strictJsonSchema: false }
    }
  });
  
  return result.object;
  // Returns: { description: string, when: { type: "scheduled" | "delayed" | "cron" | "no-schedule", ... } }
}
```

### Schedule Examples

```typescript
// One-time: Tomorrow at 2pm
await schedule({
  date: "2026-04-06T14:00:00Z",
  callback: "runTask",
  payload: { task: "backup" }
});

// Delayed: In 30 minutes
await schedule({
  delay: 1800,  // 30 minutes in seconds
  callback: "runTask",
  payload: { task: "cleanup" }
});

// Recurring: Every day at midnight
await schedule({
  cron: "0 0 * * *",
  callback: "runTask",
  payload: { task: "daily-report" }
});

// Recurring: Every Monday at 9am
await schedule({
  cron: "0 9 * * 1",
  callback: "runTask",
  payload: { task: "weekly-meeting" }
});
```

---

## AI Integration

### Workers AI Provider

Cloudflare AI provides 6 capabilities:

```typescript
import { createWorkersAI } from "workers-ai-provider";
import { generateText } from "ai";

// In Worker
export default {
  async fetch(request: Request, env: Env) {
    const ai = createWorkersAI({ binding: env.AI });
    
    // Chat
    const chatModel = ai.chat("@cf/meta/llama-3.3-70b-instruct-fp8-fast");
    
    // Image generation
    const imageModel = ai.image("@cf/black-forest-labs/flux-1-schnell");
    
    // Embeddings
    const embeddingModel = ai.embedding("@cf/baai/bge-small-en-v1.5");
    
    // Transcription
    const transcriptionModel = ai.transcription("@cf/openai/whisper");
    
    // Text-to-speech
    const speechModel = ai.speech("@cf/playht/playht-tts-model-v1");
    
    // Reranking
    const rerankModel = ai.reranking("@cf/baai/bge-reranker-v2-m3");
  }
};
```

### AI Chat Agent

Built-in AI chat with message persistence:

```typescript
import { Agent, callable } from "agents";

export class ChatAgent extends Agent<Env, ChatState> {
  initialState: ChatState = { 
    messages: [],
    isStreaming: false 
  };
  
  @callable()
  async sendMessage(content: string) {
    // Add user message
    this.setState({
      messages: [...this.state.messages, { role: "user", content }]
    });
    
    // Call LLM
    const response = await this.env.AI.run("@cf/meta/llama-3.3-70b-instruct-fp8-fast", {
      messages: this.state.messages
    });
    
    // Add assistant response
    this.setState({
      messages: [...this.state.messages, { role: "assistant", content: response }]
    });
    
    return response;
  }
  
  @callable()
  async *streamMessage(content: string) {
    // Add user message
    this.setState({
      messages: [...this.state.messages, { role: "user", content }],
      isStreaming: true
    });
    
    // Stream response
    const stream = await this.env.AI.run("@cf/meta/llama-3.3-70b-instruct-fp8-fast", {
      messages: this.state.messages,
      stream: true
    });
    
    let fullResponse = "";
    for await (const chunk of stream) {
      fullResponse += chunk.response;
      yield { content: chunk.response, done: false };
    }
    
    // Add complete response
    this.setState({
      messages: [...this.state.messages, { role: "assistant", content: fullResponse }],
      isStreaming: false
    });
    
    yield { content: fullResponse, done: true };
  }
}
```

### Tool Calling

Agents can execute tools during AI conversations:

```typescript
import { Agent, callable, tool } from "agents";

export class AssistantAgent extends Agent<Env, AssistantState> {
  @tool()
  async getWeather(city: string): Promise<{ temp: number; condition: string }> {
    const response = await fetch(`https://api.weather.com/${city}`);
    return response.json();
  }
  
  @tool()
  async searchWeb(query: string): Promise<string[]> {
    // Search implementation
    return ["result1", "result2"];
  }
  
  @callable()
  async chat(message: string) {
    const response = await this.env.AI.run("@cf/meta/llama-3.3-70b-instruct-fp8-fast", {
      messages: [{ role: "user", content: message }],
      tools: [this.getWeather, this.searchWeb]
    });
    
    return response;
  }
}
```

---

## MCP Integration

### What is MCP?

**Model Context Protocol (MCP)** is an open protocol for AI tool integration. Agents can:

1. **Act as MCP Servers** - Expose tools to external AI systems
2. **Connect as MCP Clients** - Use tools from other MCP servers

### MCP Server

```typescript
import { Agent, mcpServer, mcpTool } from "agents";

export class MCPServerAgent extends Agent<Env, State> {
  @mcpServer()
  get serverInfo() {
    return {
      name: "my-agent-server",
      version: "1.0.0"
    };
  }
  
  @mcpTool({
    name: "get_user_info",
    description: "Get information about a user"
  })
  async getUserInfo(userId: string) {
    const user = await this.db.users.find(userId);
    return user;
  }
  
  @mcpTool({
    name: "create_task",
    description: "Create a new task"
  })
  async createTask(title: string, dueDate?: Date) {
    const task = await this.db.tasks.create({ title, dueDate });
    return task;
  }
}
```

### MCP Client

```typescript
import { Agent, mcpClient } from "agents";

export class MCPClientAgent extends Agent<Env, State> {
  async onInit() {
    // Connect to external MCP server
    const client = await mcpClient.connect({
      url: "https://mcp.example.com/sse"
    });
    
    // List available tools
    const tools = await client.listTools();
    console.log("Available tools:", tools);
    
    // Call tool
    const result = await client.callTool("get_weather", { city: "London" });
    console.log("Weather:", result);
  }
}
```

### MCP Transports

Supported transports:

```typescript
// Stdio transport (local process)
const client = await mcpClient.connect({
  command: "npx",
  args: ["-y", "@modelcontextprotocol/server-filesystem"]
});

// SSE transport (remote server)
const client = await mcpClient.connect({
  url: "https://mcp.example.com/sse"
});

// WebSocket transport
const client = await mcpClient.connect({
  url: "wss://mcp.example.com/ws"
});
```

---

## Workflows

### What Are Workflows?

Workflows are **durable multi-step processes** that:

1. Survive failures and restarts
2. Support human-in-the-loop approval
3. Maintain execution state
4. Can pause and resume

### Basic Workflow

```typescript
import { Agent, workflow, callable } from "agents";

export class OrderAgent extends Agent<Env, OrderState> {
  @workflow()
  async processOrder(orderId: string) {
    // Step 1: Validate order
    const order = await this.validateOrder(orderId);
    
    // Step 2: Process payment
    const payment = await this.processPayment(order);
    
    // Step 3: Ship item
    const shipment = await this.shipItem(order);
    
    // Step 4: Send confirmation
    await this.sendConfirmation(order, payment, shipment);
    
    return { success: true, orderId };
  }
  
  private async validateOrder(orderId: string) {
    // Validation logic
    return { orderId, valid: true };
  }
  
  private async processPayment(order: Order) {
    // Payment logic
    return { paid: true };
  }
  
  private async shipItem(order: Order) {
    // Shipping logic
    return { shipped: true };
  }
  
  private async sendConfirmation() {
    // Notification logic
  }
}
```

### Human-in-the-Loop

Workflow with approval step:

```typescript
import { Agent, workflow, waitForApproval } from "agents";

export class ApprovalAgent extends Agent<Env, State> {
  @workflow()
  async deployWithApproval(deployment: Deployment) {
    // Step 1: Prepare deployment
    await this.prepareDeployment(deployment);
    
    // Step 2: Wait for human approval
    const approved = await waitForApproval({
      type: "deployment",
      data: deployment,
      approvers: ["admin@example.com"],
      timeout: 3600  // 1 hour timeout
    });
    
    if (!approved) {
      return { success: false, reason: "Deployment rejected" };
    }
    
    // Step 3: Execute deployment
    await this.executeDeployment(deployment);
    
    return { success: true };
  }
}
```

### Workflow State

Workflows maintain state across steps:

```typescript
@workflow()
async longRunningWorkflow(data: InputData) {
  // Workflow can survive:
  // - Worker restarts
  // - Timeout errors
  // - External service failures
  
  const step1 = await this.step1(data);
  const step2 = await this.step2(step1);
  const step3 = await this.step3(step2);
  
  return step3;
}
```

---

## Production Deployment

### Build and Deploy

```bash
# Install dependencies
npm install

# Build
npm run build

# Deploy
wrangler deploy
```

### Environment Configuration

Production `wrangler.jsonc`:

```jsonc
{
  "name": "my-agents-production",
  "main": "dist/index.js",
  "compatibility_date": "2026-01-28",
  "compatibility_flags": ["nodejs_compat"],
  
  "durable_objects": {
    "bindings": [
      { "name": "COUNTER_AGENT", "class_name": "CounterAgent" }
    ]
  },
  
  "vars": {
    "ENVIRONMENT": "production"
  },
  
  "secrets": [
    "DATABASE_URL",
    "API_KEY"
  ]
}
```

### Secrets Management

```bash
# Set secrets
wrangler secret put DATABASE_URL
wrangler secret put API_KEY

# List secrets
wrangler secret list
```

### Scaling

Agents scale automatically:

- **Horizontal** - Each agent instance runs independently
- **Geographic** - Durable Objects run close to users
- **Load-based** - No configuration needed

### Monitoring

```typescript
import { Agent } from "agents";

export class MonitoredAgent extends Agent<Env, State> {
  async onInit() {
    // Log agent creation
    console.log("Agent created:", this.agentId);
  }
  
  onStateUpdate(newState: State) {
    // Track state changes
    this.ctx.waitUntil(this.trackMetrics(newState));
  }
  
  async onHibernate() {
    // Log hibernation
    console.log("Agent hibernating:", this.agentId);
  }
  
  private async trackMetrics(state: State) {
    // Send to analytics platform
    await fetch("https://analytics.example.com/track", {
      method: "POST",
      body: JSON.stringify({ agent: this.agentId, state })
    });
  }
}
```

---

## Next Steps

### Learn More

- [Deep Dive: Durable Objects Architecture](./01-durable-objects-deep-dive.md)
- [Deep Dive: State Synchronization](./02-state-sync-deep-dive.md)
- [Deep Dive: Callable Methods & RPC](./03-callable-methods-deep-dive.md)
- [Deep Dive: Real-time Communication](./04-realtime-websockets-deep-dive.md)
- [Deep Dive: AI Integration Patterns](./05-ai-integration-deep-dive.md)
- [Rust Revision: Agents in Rust](./rust-revision.md)
- [Production Guide](./production-grade.md)

### Example Projects

- **Counter** - Basic state management
- **Chat** - Real-time messaging
- **Task Manager** - Scheduling and workflows
- **MCP Server** - Tool integration
- **AI Assistant** - LLM integration

### Community

- [GitHub Repository](https://github.com/cloudflare/agents)
- [Documentation](https://developers.cloudflare.com/agents/)
- [Discord](https://discord.gg/cloudflare)
