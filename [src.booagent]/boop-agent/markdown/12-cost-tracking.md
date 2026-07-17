# Boop: Cost Tracking

Every LLM call writes a row to the `usageRecords` table. The debug dashboard shows real-time usage and cost across all layers.

**Source:** `server/usage.ts`

## Usage Sources

Every call is tagged with its source:

| Source | When |
|--------|------|
| `dispatcher` | Interaction agent's turn |
| `execution` | Spawned sub-agent run |
| `extract` | Post-turn memory extraction |
| `consolidation-proposer` | Consolidation proposer LLM call |
| `consolidation-adversary` | Consolidation adversary LLM call |
| `consolidation-judge` | Consolidation judge LLM call |
| `proactive` | Proactive email classification |

**Source:** `convex/schema.ts:102-109`

## UsageTotals Interface

```ts
// server/usage.ts:3
export interface UsageTotals {
  model: string;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  costUsd: number;
}

export const EMPTY_USAGE: UsageTotals = {
  model: "unknown",
  inputTokens: 0,
  outputTokens: 0,
  cacheReadTokens: 0,
  cacheCreationTokens: 0,
  costUsd: 0,
};
```

**Source:** `server/usage.ts:3-20`

## Claude Cost: Real from SDK

For the Claude runtime, cost comes directly from the Anthropic SDK's result message:

```ts
// server/usage.ts:91
export function aggregateUsageFromResult(
  msg: Extract<SDKMessage, { type: "result" }>,
  requestedModel?: string,
): UsageTotals {
  // Prefer modelUsage (aggregate per-model across whole query) over msg.usage
  // msg.usage massively undercounts on tool-heavy runs
  const modelUsage = msg.modelUsage ?? {};

  let inputTokens = 0;
  let outputTokens = 0;
  let cacheReadTokens = 0;
  let cacheCreationTokens = 0;

  for (const [model, u] of Object.entries(modelUsage)) {
    inputTokens += u.inputTokens ?? 0;
    outputTokens += u.outputTokens ?? 0;
    cacheReadTokens += u.cacheReadInputTokens ?? 0;
    cacheCreationTokens += u.cacheCreationInputTokens ?? 0;
  }

  return {
    model: reportedModel,
    inputTokens,
    outputTokens,
    cacheReadTokens,
    cacheCreationTokens,
    costUsd: msg.total_cost_usd ?? 0,
  };
}
```

**Aha:** The SDK's `msg.modelUsage` can contain multiple models per query because Claude Code CLI uses different models for different internal sub-tasks (e.g., haiku for cheap routing + sonnet for the main response). Boop aggregates across all models and reports the heaviest-usage model.

**Source:** `server/usage.ts:91-141`

## Codex Cost: Estimated from Token Prices

Codex subscription doesn't expose real billing. Boop estimates cost from OpenAI's published standard API token prices:

```ts
// server/usage.ts:30
const OPENAI_STANDARD_TOKEN_PRICES: TokenPrice[] = [
  { model: "gpt-5.5", inputPerMillion: 5, cachedInputPerMillion: 0.5, outputPerMillion: 30 },
  { model: "gpt-5.4-mini", inputPerMillion: 0.75, cachedInputPerMillion: 0.075, outputPerMillion: 4.5 },
  { model: "gpt-5.4", inputPerMillion: 2.5, cachedInputPerMillion: 0.25, outputPerMillion: 15 },
  { model: "gpt-5.3-codex", inputPerMillion: 1.75, cachedInputPerMillion: 0.175, outputPerMillion: 14 },
  // ... more models
];

export function estimateOpenAiCostUsd(usage: Omit<UsageTotals, "costUsd">): number {
  const price = priceForOpenAIModel(usage.model);
  if (!price) return 0;
  const cachedInputTokens = Math.max(0, usage.cacheReadTokens);
  const uncachedInputTokens = Math.max(0, usage.inputTokens - cachedInputTokens + usage.cacheCreationTokens);
  return (
    (uncachedInputTokens * price.inputPerMillion +
      cachedInputTokens * price.cachedInputPerMillion +
      usage.outputTokens * price.outputPerMillion) / 1_000_000
  );
}
```

**Important:** Treat Codex dashboard spend as an API-equivalent usage proxy, not a bill.

**Source:** `server/usage.ts:30-74`

## Cost Recording

Every layer records its own usage:

### Dispatcher

```ts
// server/interaction-agent.ts:604
if (usage.costUsd > 0 || usage.inputTokens > 0) {
  await convex.mutation(api.usageRecords.record, {
    source: "dispatcher",
    conversationId: opts.conversationId,
    turnId,
    runtime: runtimeConfig.runtime,
    billingMode: runtimeConfig.billingMode,
    model: usage.model,
    inputTokens: usage.inputTokens,
    outputTokens: usage.outputTokens,
    cacheReadTokens: usage.cacheReadTokens,
    cacheCreationTokens: usage.cacheCreationTokens,
    costUsd: usage.costUsd,
    durationMs: Date.now() - turnStart,
  });
}
```

**Source:** `server/interaction-agent.ts:604-622`

### Execution Agent

```ts
// server/execution-agent.ts:258
if (usage.costUsd > 0 || usage.inputTokens > 0) {
  await convex.mutation(api.usageRecords.record, {
    source: "execution",
    conversationId: opts.conversationId,
    agentId,
    runtime: runtimeConfig.runtime,
    billingMode: runtimeConfig.billingMode,
    model: usage.model,
    inputTokens: usage.inputTokens,
    outputTokens: usage.outputTokens,
    cacheReadTokens: usage.cacheReadTokens,
    cacheCreationTokens: usage.cacheCreationTokens,
    costUsd: usage.costUsd,
    durationMs: Date.now() - agentStart,
  });
}
```

**Source:** `server/execution-agent.ts:258-273`

### Memory Extraction

```ts
// server/memory/extract.ts:78
if (usage.costUsd > 0 || usage.inputTokens > 0) {
  await convex.mutation(api.usageRecords.record, {
    source: "extract",
    conversationId: opts.conversationId,
    turnId: opts.turnId,
    runtime: runtimeConfig.runtime,
    billingMode: runtimeConfig.billingMode,
    model: usage.model,
    inputTokens: usage.inputTokens,
    outputTokens: usage.outputTokens,
    cacheReadTokens: usage.cacheReadTokens,
    cacheCreationTokens: usage.cacheCreationTokens,
    costUsd: usage.costUsd,
    durationMs: Date.now() - started,
  });
}
```

**Source:** `server/memory/extract.ts:78-93`

### Consolidation

Each phase records separately:

```ts
// server/consolidation.ts:135
async function recordConsolidationUsage(
  source: "consolidation-proposer" | "consolidation-adversary" | "consolidation-judge",
  runId: string,
  usage: UsageTotals,
  durationMs: number,
  runtimeConfig: RuntimeConfig,
): Promise<void> {
  if (usage.costUsd <= 0 && usage.inputTokens <= 0) return;
  await convex.mutation(api.usageRecords.record, { source, runId, ... });
}
```

**Source:** `server/consolidation.ts:135-156`

## Runaway Usage Warning

**Aha:** Boop's Claude SDK `query()` calls don't currently set `maxTurns` or `maxBudgetUsd`. These are hard stops the Claude SDK exposes — the agent aborts once the threshold hits.

Kept as-is intentionally for a single-user personal agent:
- Every task is scoped tight (specific task string + small integration list)
- Integrations are Composio-scoped per spawn so the tool surface stays small
- The 15-minute heartbeat marks any long-running agent as `failed`
- In practice, execution agents complete in under 60 seconds

If deploying in a higher-throughput setting, add runtime-specific caps.

**Source:** `README.md:565-570` (upstream)

## Next

Continue to [Cross-Cutting Concerns](13-cross-cutting.md) for privacy, heartbeat, and broadcast.
