---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Mastra/mastra
repository: https://github.com/mastra-ai/mastra
deep_dive_at: 2026-03-19
type: master-workflows-architecture
---

# Mastra Master Workflows Deep Dive

## Overview

This document explores Mastra's workflow orchestration system, focusing on the "master" patterns that coordinate multiple agents, tools, and workflows. The primary master pattern is the **Agent Network** (implemented in `networkLoop`), which acts as a supervisory router that delegates tasks to specialized primitives.

## Repository Context

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Mastra/mastra`
- **Remote:** https://github.com/mastra-ai/mastra
- **Primary Language:** TypeScript
- **Package Manager:** pnpm

---

## Table of Contents

1. [The Master Pattern: Agent Network](#1-the-master-pattern-agent-network)
2. [Network Loop Architecture](#2-network-loop-architecture)
3. [Workflow Engine Deep Dive](#3-workflow-engine-deep-dive)
4. [Agent Delegation Patterns](#4-agent-delegation-patterns)
5. [State Management and Persistence](#5-state-management-and-persistence)
6. [Execution Flow Diagrams](#6-execution-flow-diagrams)

---

## 1. The Master Pattern: Agent Network

### What Is The Network?

The Agent Network is Mastra's "master" orchestration layer - a routing system that coordinates multiple specialized agents, workflows, and tools to accomplish complex tasks. It implements a **router-delegator pattern** where:

1. A **Routing Agent** analyzes incoming tasks
2. Routes to the appropriate **primitive** (agent, workflow, or tool)
3. Collects results and determines if the task is complete
4. Iterates until completion criteria are met

### Key File

**Primary Implementation:** `packages/core/src/loop/network/index.ts`

### Network Components

| Component | Purpose |
|-----------|---------|
| **Routing Agent** | Analyzes tasks, selects appropriate primitive, determines completion |
| **Agent Step** | Executes sub-agents with specific prompts |
| **Workflow Step** | Executes workflows with JSON inputs |
| **Tool Step** | Executes individual tools |
| **Validation Step** | Runs scorers/checks to verify task completion |
| **Finish Step** | Finalizes execution and returns results |

---

## 2. Network Loop Architecture

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                      USER TASK INPUT                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    ROUTING AGENT STEP                            │
│  - Analyzes task                                                 │
│  - Selects primitive (agent/workflow/tool)                       │
│  - Returns: primitiveId, primitiveType, prompt, selectionReason  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    BRANCH TO PRIMITIVE                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ Agent Step  │  │Workflow Step│  │  Tool Step  │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   VALIDATION STEP                                │
│  - Runs configured scorers                                      │
│  - Checks completion criteria                                   │
│  - Generates feedback if incomplete                             │
│  - Generates finalResult if complete                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │                   │
              Complete            Incomplete
                    │                   │
                    ▼                   │
              ┌─────────┐               │
              │ FINISH  │───────────────┘
              └─────────┘
                    │
                    ▼
              (Loop continues via .dountil)
```

### The Workflow Graph

```typescript
// Main iteration workflow (combines network + validation)
const iterationWithValidation = createWorkflow({
  id: 'iteration-with-validation',
})
  .then(networkWorkflow)      // Routes + executes primitive
  .then(validationStep)       // Validates completion
  .commit();

// Main loop with termination condition
const mainWorkflow = createWorkflow({
  id: 'agent-loop-main-workflow',
})
  .dountil(iterationWithValidation, async ({ inputData }) => {
    // Complete when:
    // 1. LLM says complete AND validation passed
    // 2. OR max iterations reached
    const llmComplete = inputData.isComplete === true;
    const validationOk = inputData.validationPassed !== false;
    const maxReached = Boolean(maxIterations && inputData.iteration >= maxIterations);
    return (llmComplete && validationOk) || maxReached;
  })
  .then(finalStep)
  .commit();
```

### Routing Agent Implementation

The routing agent is dynamically constructed with knowledge of all available primitives:

```typescript
const instructions = `
  You are a router in a network of specialized AI agents.
  Your job is to decide which agent should handle each step of a task.

  ## Available Agents in Network
  ${agentList}

  ## Available Workflows in Network
  ${workflowList}

  ## Available Tools in Network
  ${toolList}

  Please select the most appropriate primitive:
  {
    "primitiveId": string,
    "primitiveType": "agent" | "workflow" | "tool",
    "prompt": string,
    "selectionReason": string
  }
`;
```

**Key Routing Rules:**
- Agents receive text prompts (chat-style)
- Workflows/tools receive JSON inputs matching their schema
- Must provide `selectionReason` explaining the choice
- Can return `"none"` for both id and type to signal completion

---

## 3. Workflow Engine Deep Dive

### Workflow Class Structure

```typescript
class Workflow<TSteps, TStepResults, TInput, TOutput, TEngine> {
  id: string;
  name: string;
  #steps: Record<string, Step>;
  #stepGraph: StepGraph;
  #stepSubscriberGraph?: Record<string, StepGraph>;
  #executionEngine: ExecutionEngine;
  #pubsub?: PubSub;

  // Builder pattern methods
  step(step: Step): this;
  then(nextStep: Step): this;
  branch(conditions: BranchCondition[], steps: Step[]): this;
  parallel(steps: Step[]): this;
  loop(condition: LoopConditionFunction): this;
  dountil(workflow: Workflow, condition: ConditionFunction): this;
  map(variables: DynamicMapping): this;
  commit(): this;
}
```

### Step Definition

Steps can be created from multiple sources:

```typescript
// 1. Explicit parameters
const customStep = createStep({
  id: 'my-step',
  inputSchema: z.object({ data: z.string() }),
  outputSchema: z.object({ result: z.string() }),
  execute: async ({ inputData, writer, suspend, resumeData }) => {
    // Step execution logic
    return { result: 'done' };
  }
});

// 2. From an Agent
const agentStep = createStep(someAgent, { maxSteps: 3 });

// 3. From a Tool
const toolStep = createStep(someTool);
```

### Execution Engine

The default execution engine (`DefaultExecutionEngine`) handles:

| Feature | Description |
|---------|-------------|
| **Step Resolution** | Determines which steps to execute based on graph |
| **Dependency Management** | Runs independent steps in parallel |
| **Error Handling** | Retry logic, error boundaries |
| **State Persistence** | Saves state for suspension/resumption |
| **Streaming** | Real-time progress via events |

### Branch Logic

The network uses conditional branching to route to the correct primitive:

```typescript
networkWorkflow
  .then(routingStep)
  .branch([
    [
      async ({ inputData }) => !inputData.isComplete && inputData.primitiveType === 'agent',
      agentStep
    ],
    [
      async ({ inputData }) => !inputData.isComplete && inputData.primitiveType === 'workflow',
      workflowStep
    ],
    [
      async ({ inputData }) => !inputData.isComplete && inputData.primitiveType === 'tool',
      toolStep
    ],
    [
      async ({ inputData }) => !!inputData.isComplete,
      finishStep
    ],
  ])
```

### Variable Mapping

After branching, variables must be mapped from different step outputs:

```typescript
.map({
  task: {
    step: [routingStep, agentStep, workflowStep, toolStep],
    path: 'task',
  },
  isComplete: {
    step: [agentStep, workflowStep, toolStep, finishStep],
    path: 'isComplete',
  },
  result: {
    step: [agentStep, workflowStep, toolStep, finishStep],
    path: 'result',
  },
  primitiveId: {
    step: [routingStep, agentStep, workflowStep, toolStep],
    path: 'primitiveId',
  },
})
```

---

## 4. Agent Delegation Patterns

### Network Mode Delegation

Agents can be configured with a network of other agents for delegation:

```typescript
const supervisor = new Agent({
  id: 'supervisor',
  agents: {
    researcher: researcherAgent,
    writer: writerAgent,
    coder: coderAgent,
  },
  delegatorType: 'network',  // Uses agent network for delegation
});
```

### How Delegation Works

1. **Agent Detection**: When an agent has `agents` configured and `delegatorType: 'network'`
2. **Network Initialization**: The `networkLoop` function is invoked
3. **Memory Setup**: A dedicated thread is created for the network execution
4. **Routing**: The routing agent decides which sub-agent handles each part
5. **Result Aggregation**: Results are collected and returned to the parent agent

### Sub-Agent Execution

Sub-agents receive filtered conversation context:

```typescript
// Filters out internal network JSON markers
function filterMessagesForSubAgent(messages: MastraDBMessage[]): MastraDBMessage[] {
  return messages.filter(msg => {
    if (msg.role === 'user') return true;

    if (msg.role === 'assistant') {
      // Exclude isNetwork JSON (result markers)
      // Exclude routing decision JSON
      // Exclude completion feedback messages
      const parts = msg.content?.parts ?? [];
      for (const part of parts) {
        if (part?.type === 'text' && part?.text) {
          try {
            const parsed = JSON.parse(part.text);
            if (parsed.isNetwork) return false;
            if (parsed.primitiveId && parsed.selectionReason) return false;
          } catch {
            // Not JSON, continue
          }
        }
      }
      return true;
    }
    return false;
  });
}
```

### Tool Approval and Suspension

The network supports human-in-the-loop patterns:

```typescript
// Tool can require approval
const sensitiveTool = createTool({
  id: 'delete-resource',
  requireApproval: true,  // Blocks for approval
  execute: async (ctx) => { ... }
});

// Or dynamic approval function
const conditionalApprovalTool = createTool({
  id: 'expensive-operation',
  needsApprovalFn: async (args) => {
    return args.cost > 100;  // Approval for expensive ops
  },
  execute: async (ctx) => { ... }
});
```

**Approval Flow:**
1. Tool step detects `requireApproval` or `needsApprovalFn`
2. Emits `tool-execution-approval` event
3. Workflow suspends with `resumeSchema`
4. External system provides approval decision
5. Workflow resumes with approval result

---

## 5. State Management and Persistence

### Workflow State

Workflow state is persisted to enable suspension and resumption:

```typescript
interface WorkflowState {
  status: 'running' | 'suspended' | 'failed' | 'completed';
  steps: Record<string, StepResult>;
  suspended?: string[][];  // Paths to suspended steps
}
```

### Storage Domains

Workflows use specific storage domains:

```typescript
interface StorageDomains {
  workflows: WorkflowsStorage;  // Workflow state persistence
  memory: MemoryStorage;        // Conversation threads/messages
  // ... other domains
}

interface WorkflowsStorage {
  getWorkflowSnapshot(params): Promise<WorkflowSnapshot | null>;
  persistWorkflowSnapshot(params): Promise<void>;
  deleteWorkflowSnapshot(params): Promise<void>;
}
```

### Snapshot Persistence

The network workflow configures when to persist:

```typescript
const networkWorkflow = createWorkflow({
  options: {
    shouldPersistSnapshot: ({ workflowStatus }) => {
      return workflowStatus === 'suspended';
    },
    validateInputs: false,
  }
});
```

### Memory Integration

The network creates and manages its own memory context:

```typescript
const { thread } = await prepareMemoryStep({
  threadId: threadId || run.runId,
  resourceId: resourceId || networkName,
  messages,
  routingAgent,
  generateId,
});
```

**Memory is used for:**
- Storing routing decisions
- Storing primitive results (marked with `metadata.mode: 'network'`)
- Storing validation feedback
- Enabling sub-agents to access conversation history
- Supporting working memory tools

---

## 6. Execution Flow Diagrams

### Complete Network Execution Sequence

```
┌──────────────────────────────────────────────────────────────────────────┐
│                         networkLoop() Entry                               │
│  - Validates memory is configured                                       │
│  - Prepares memory thread                                               │
│  - Creates networkWorkflow with steps                                    │
│  - Creates validationStep                                                │
│  - Creates mainWorkflow with .dountil loop                              │
└──────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                    mainWorkflow.stream()                                  │
│  - Initializes execution engine                                          │
│  - Emits 'workflow-start' event                                         │
│  - Begins iteration loop                                                 │
└──────────────────────────────────────────────────────────────────────────┘
                                      │
                    ┌─────────────────┴─────────────────┐
                    │                                   │
                    ▼                                   ▼
    ┌───────────────────────────┐       ┌───────────────────────────┐
    │   ITERATION (Loop Body)   │       │   TERMINATION CHECK       │
    │                           │       │                           │
    │  1. routing-agent-step    │◄──────┤  - isComplete &&          │
    │  2. branch to primitive   │       │    validationPassed       │
    │  3. validation-step       │       │  - OR maxIterations       │
    │                           │       │                           │
    └───────────────────────────┘       └───────────────────────────┘
                    │                                   │
                    │ (loop continues)                  │ (exit loop)
                    │                                   │
                    ▼                                   ▼
    ┌───────────────────────────┐       ┌───────────────────────────┐
    │   Event Stream Output     │       │   final-step              │
    │                           │       │   - Emit finish event     │
    │  - routing-agent-start    │       │   - Return final result   │
    │  - routing-agent-end      │       │   - Close stream          │
    │  - primitive-start        │       │                           │
    │  - primitive-event-*      │       └───────────────────────────┘
    │  - primitive-end          │
    │  - validation-start       │
    │  - validation-end         │
    │  - step-finish            │
    │                           │
    └───────────────────────────┘
```

### Event Types Emitted

| Event Type | When Emitted | Payload Contents |
|------------|--------------|------------------|
| `routing-agent-start` | Before routing LLM call | agentId, inputData, runId |
| `routing-agent-end` | After routing decision | primitiveId, primitiveType, prompt, selectionReason, usage |
| `agent-execution-start` | Before sub-agent call | agentId, args |
| `agent-execution-event-*` | During sub-agent stream | text-delta, tool-call, etc. |
| `agent-execution-end` | After sub-agent completes | result, usage |
| `workflow-execution-start` | Before workflow run | workflowId, args |
| `workflow-execution-event-*` | During workflow stream | step-start, step-end, etc. |
| `tool-execution-start` | Before tool execute | toolName, args |
| `tool-execution-approval` | When approval needed | resumeSchema, args |
| `tool-execution-suspended` | When tool suspends | suspendPayload, resumeSchema |
| `network-validation-start` | Before scorers run | iteration, checksCount |
| `network-validation-end` | After validation | passed, results, duration |
| `network-execution-event-finish` | When task complete | final result, object |

---

## 7. Validation and Scorers

### Validation Step Purpose

The validation step runs external checks to verify task completion, preventing the LLM from prematurely ending tasks.

### Scorer Integration

```typescript
const validation = {
  scorers: [
    { scorer: myCustomScorer, sampleRate: 0.5 },
  ],
  maxIterations: 10,
};
```

### Scorer Execution

```typescript
async function runValidation(validation, context) {
  const results = [];

  for (const { scorer, sampleRate } of validation.scorers) {
    if (Math.random() > sampleRate) continue;  // Sampling

    const result = await runScorer({
      scorer,
      input: {
        output: context.primitiveResult,
        input: context.originalTask,
        context: context.messages,
      },
    });

    results.push(result);
  }

  return {
    complete: results.every(r => r.pass),
    completionReason: results.map(r => r.reason).join('\n'),
    scorers: results,
  };
}
```

### Default Completion Check

When no scorers are configured, an LLM-based check runs:

```typescript
async function runDefaultCompletionCheck(routingAgent, context) {
  const result = await routingAgent.generate([
    { role: 'user', content: `
      Task: ${context.originalTask}
      Result: ${context.primitiveResult}

      Is this task complete? Respond with:
      { "passed": boolean, "reason": string, "finalResult": string }
    `}
  ], { structuredOutput: {...} });

  return result.object;
}
```

---

## 8. Key Design Patterns

### 1. Workflow-as-Step

Workflows can be embedded as steps in other workflows:

```typescript
const innerWorkflow = createWorkflow({...}).commit();

const outerWorkflow = createWorkflow({...})
  .then(innerWorkflow)  // Workflow as a step
  .then(nextStep)
  .commit();
```

### 2. Dynamic Mapping

Variables can be mapped from any step's output:

```typescript
.map({
  task: {
    step: [routingStep, agentStep, workflowStep, toolStep],
    path: 'task',  // Dot-notation path into output schema
  },
})
```

### 3. Suspension/Resumption

Steps can suspend execution for external input:

```typescript
execute: async ({ suspend, resumeData }) => {
  if (needsApproval) {
    return await suspend({
      requireToolApproval: { ... },
    });
  }

  // On resume, resumeData contains approval decision
  if (resumeData?.approved) {
    // Execute tool
  }
}
```

### 4. Streaming Events

All steps emit structured events for UI integration:

```typescript
await writer.write({
  type: 'agent-execution-event-text-delta',
  payload: { delta: 'Hello' },
  from: ChunkFrom.NETWORK,
  runId,
});
```

### 5. Composite Store Pattern

Different storage backends for different domains:

```typescript
const storage = new MastraCompositeStore({
  default: pgStore,
  domains: {
    memory: libsqlStore.stores.memory,
    workflows: pgStore.stores.workflows,
    vectors: pineconeStore,
  }
});
```

---

## 9. Performance Characteristics

| Operation | Time Complexity | Notes |
|-----------|-----------------|-------|
| Routing decision | O(1) LLM call | Depends on prompt size |
| Sub-agent execution | O(n) | n = context tokens |
| Workflow step | O(s) | s = step complexity |
| Validation (scorers) | O(k) | k = number of scorers |
| State persistence | O(1) DB write | Depends on storage backend |
| Event streaming | O(1) per chunk | Real-time |

---

## 10. Common Patterns and Best Practices

### 1. Configuring Max Iterations

Always set a maximum to prevent infinite loops:

```typescript
const result = agent.network(messages, {
  maxIterations: 10,  // Hard limit
  validation: {
    scorers: [...],
  },
});
```

### 2. Using Scorers for Quality

Scorers provide objective completion checks:

```typescript
const relevanceScorer = createScorer({
  id: 'relevance',
  instructions: 'Rate how well the result addresses the task',
  outputSchema: z.object({
    score: z.number().min(0).max(1),
    reason: z.string(),
  }),
});
```

### 3. Handling Suspended Workflows

```typescript
// Check for suspension
if (workflowState.status === 'suspended') {
  const suspendedStep = workflowState.suspended[0][0];
  const suspendPayload = workflowState.steps[suspendedStep].suspendPayload;

  // Provide resume data
  await run.resume({
    resumeData: { approved: true },
    stepId: suspendedStep,
  });
}
```

### 4. Event Stream Processing

```typescript
for await (const chunk of result.fullStream) {
  switch (chunk.type) {
    case 'routing-agent-start':
      console.log('Routing began:', chunk.payload);
      break;
    case 'agent-execution-event-text-delta':
      process.stdout.write(chunk.payload.delta);
      break;
    case 'network-execution-event-finish':
      console.log('\nTask complete:', chunk.payload.result);
      break;
  }
}
```

---

## 11. Troubleshooting

### Network Returns "none" Immediately

**Cause:** Routing agent thinks task is already complete.

**Solution:**
- Check that task is clearly stated
- Verify available agents/workflows/tools are listed correctly
- Add `verboseIntrospection: true` to see selection reasoning

### Validation Always Fails

**Cause:** Scorers too strict or feedback not reaching routing agent.

**Solution:**
- Check scorer thresholds
- Verify `suppressFeedback: false` in validation config
- Ensure feedback messages are being saved to memory

### Workflow Suspension Not Working

**Cause:** Missing `shouldPersistSnapshot` or storage not configured.

**Solution:**
```typescript
const workflow = createWorkflow({
  options: {
    shouldPersistSnapshot: ({ workflowStatus }) => {
      return workflowStatus === 'suspended';
    },
  }
});
```

### Sub-Agents Don't See Context

**Cause:** Memory not properly configured for network.

**Solution:**
- Ensure memory is configured on the routing agent
- Check that `threadId` and `resourceId` are passed correctly
- Verify memory processors aren't filtering out needed context

---

## Summary

Mastra's master workflow pattern (Agent Network) provides:

1. **Router-Delegator Architecture**: Central routing agent distributes tasks to specialized primitives
2. **Iterative Execution**: Loops until validation confirms completion or max iterations reached
3. **Validation Layer**: Configurable scorers provide objective quality checks
4. **Human-in-the-Loop**: Approval and suspension patterns for sensitive operations
5. **Streaming Events**: Real-time visibility into execution progress
6. **State Persistence**: Suspension/resumption support for long-running tasks
7. **Composable Workflows**: Workflows can embed other workflows as steps
8. **Memory Integration**: Full conversation history and working memory support

The network loop is the "master" pattern that orchestrates Mastra's primitives into coordinated multi-agent systems capable of handling complex, multi-step tasks.
