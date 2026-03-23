# Grep-Bench - Deep Dive

## Overview

**grep-bench** is a benchmark harness for measuring AI model performance on codebase search tasks using the BTCA (Better Context to AI) server. It captures consistent, comparable metrics across different AI models.

---

## Purpose

Measure how well different AI models answer questions about codebases using BTCA server's search capabilities.

---

## Models to Test

```
- gpt-5.2-codex
- claude-sonnet-4-5
- gemini-3-flash
- minimax-m2.1-free
- glm-4.7-free
- kimi-k2.5-free
- qwen3-coder
```

---

## Metrics Tracked

| Metric | Description |
|--------|-------------|
| **Time to run** | Wall-clock elapsed time |
| **Tool call counts** | Number of tool invocations |
| **Tokens in/out** | Input/output token counts |
| **Number of turns** | Request/response exchanges |
| **Cost** | API cost for the run |
| **Accuracy** | Quality score (0-4 rubric) |

---

## Accuracy Rubric

| Score | Description |
|-------|-------------|
| **0** | Incorrect or fabricated details; fails to answer |
| **1** | Partially relevant but missing key steps or incorrect APIs |
| **2** | Mostly correct, minor omissions or unclear steps |
| **3** | Correct, complete, and aligned with repo docs |
| **4** | Correct, complete, includes precise file references or API names |

---

## Benchmark Procedure

### 1. Setup

```bash
# Ensure BTCA server is running
btca serve --port 8080

# Verify resources are loaded
curl http://localhost:8080/resources
```

### 2. Run Tests

For each model:

```bash
# Run btca ask with specific model
btca ask \
  --server http://localhost:8080 \
  -q "Install and use better context server in a Bun/TS app; stream consumption." \
  --model <model-id>
```

### 3. Capture Stream Events

Parse SSE stream for metrics:

```typescript
// Stream event parsing
const metrics = {
  startTime: Date.now(),
  toolCalls: 0,
  tokensIn: 0,
  tokensOut: 0,
  turns: 0,
  events: []
};

for await (const event of parseSSEStream(response)) {
  metrics.events.push(event);

  switch (event.type) {
    case 'tool.updated':
      metrics.toolCalls++;
      break;
    case 'done':
      metrics.tokensIn = event.usage?.inputTokens ?? 0;
      metrics.tokensOut = event.usage?.outputTokens ?? 0;
      break;
  }
}

metrics.duration = Date.now() - metrics.startTime;
metrics.turns = Math.ceil(metrics.toolCalls / 2) + 1;
```

### 4. Score Accuracy

Compare response to golden answer:

```typescript
function scoreAccuracy(response: string, goldenAnswer: string): number {
  // Check for key facts
  const requiredFacts = [
    'btca-server',
    'stream endpoint',
    'SSE',
    '/question/stream'
  ];

  let score = 0;
  for (const fact of requiredFacts) {
    if (response.toLowerCase().includes(fact.toLowerCase())) {
      score++;
    }
  }

  // Check for file references
  if (response.includes('apps/server')) score++;

  return Math.min(score, 4);
}
```

---

## Data Format

### Output (JSONL)

```jsonl
{"model":"claude-sonnet-4-5","start":1706000000000,"end":1706000015000,"duration":15000,"toolCalls":8,"tokensIn":1250,"tokensOut":450,"turns":5,"cost":0.0032,"response":"...","accuracy":4,"accuracyNotes":"Complete with file references"}
{"model":"gpt-5.2-codex","start":1706000020000,"end":1706000040000,"duration":20000,"toolCalls":12,"tokensIn":1300,"tokensOut":520,"turns":7,"cost":0.0045,"response":"...","accuracy":3,"accuracyNotes":"Correct but missing file paths"}
```

### Sidecar Event Log

```json
{
  "model": "claude-sonnet-4-5",
  "runId": "run-123",
  "rawEvents": [
    {"type": "meta", "timestamp": 1706000000100},
    {"type": "tool.updated", "tool": "glob", "timestamp": 1706000001000},
    {"type": "text.delta", "delta": "To install...", "timestamp": 1706000002000}
  ]
}
```

---

## Harness Implementation

```typescript
// Benchmark harness
const MODELS = ['claude-sonnet-4-5', 'gpt-5.2-codex', /* ... */];
const PROMPTS = [
  'Install and use better context server in a Bun/TS app; stream consumption.',
  'How the CLI/TUI uses the server under the hood.'
];

async function runBenchmark(model: string, prompt: string) {
  const startTime = Date.now();
  const events: any[] = [];
  let response = '';

  const serverResponse = await fetch('http://localhost:8080/question/stream', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      question: prompt,
      model: { provider: 'anthropic', model }
    })
  });

  for await (const event of parseSSEStream(serverResponse)) {
    events.push(event);
    if (event.type === 'text.delta') {
      response += event.delta;
    }
  }

  const duration = Date.now() - startTime;
  const doneEvent = events.find(e => e.type === 'done');

  return {
    model,
    start: startTime,
    end: Date.now(),
    duration,
    toolCalls: events.filter(e => e.type === 'tool.updated').length,
    tokensIn: doneEvent?.usage?.inputTokens ?? 0,
    tokensOut: doneEvent?.usage?.outputTokens ?? 0,
    turns: Math.ceil(events.filter(e => e.type === 'tool-call').length / 2) + 1,
    response,
    accuracy: scoreAccuracy(response, GOLDEN_ANSWERS[prompt])
  };
}

// Run all models
const results = [];
for (const model of MODELS) {
  for (const prompt of PROMPTS) {
    const result = await runBenchmark(model, prompt);
    results.push(result);
    console.log(`${model}: accuracy=${result.accuracy}, duration=${result.duration}ms`);
  }
}

// Write results
await Bun.write('results/results.jsonl',
  results.map(r => JSON.stringify(r)).join('\n')
);
```

---

## Reporting

### Aggregate Comparison Table

| Model | Duration | Tool Calls | Tokens In/Out | Turns | Cost | Accuracy |
|-------|----------|------------|---------------|-------|------|----------|
| claude-sonnet-4-5 | 15s | 8 | 1250/450 | 5 | $0.0032 | 4.0 |
| gpt-5.2-codex | 20s | 12 | 1300/520 | 7 | $0.0045 | 3.0 |
| gemini-3-flash | 12s | 6 | 1100/400 | 4 | $0.0020 | 3.5 |

### Summary

```markdown
## Benchmark Summary

**Best Overall**: claude-sonnet-4-5 (accuracy: 4.0)
**Fastest**: gemini-3-flash (12s average)
**Most Efficient**: gemini-3-flash ($0.0020/run)
**Most Tool Calls**: gpt-5.2-codex (12 avg) - possibly over-searching

**Key Findings**:
- Claude provides most accurate answers with file references
- Gemini is fastest but occasionally misses details
- GPT-5.2 makes more tool calls but lower accuracy
```

---

## Future Improvements

1. **More prompts** - Broaden coverage
2. **More repositories** - Test across different codebases
3. **Automatic accuracy checks** - Golden answer comparison
4. **Regression tracking** - Track model performance over time
5. **Model version tracking** - Compare across versions

---

## Production Rust Implementation

### Architecture

```
grep-bench-rs/
├── src/
│   ├── main.rs        # CLI entry
│   ├── runner.rs      # Benchmark runner
│   ├── metrics.rs     # Metrics collection
│   ├── scorer.rs      # Accuracy scoring
│   └── report.rs      # Report generation
├── benches/           # Criterion benchmarks
└── results/           # Output directory
```

### Key Crates

- `reqwest` - HTTP client
- `tokio` - Async runtime
- `serde_json` - JSON handling
- `csv` - CSV output
- `indicatif` - Progress bars
- `criterion` - Benchmarking framework

### Metrics Collection

```rust
#[derive(Serialize)]
struct BenchmarkResult {
    model: String,
    duration_ms: u64,
    tool_calls: u32,
    tokens_in: u32,
    tokens_out: u32,
    turns: u32,
    cost_cents: f64,
    accuracy: u8,
    response: String,
}

async fn run_benchmark(client: &Client, model: &str, prompt: &str) -> BenchmarkResult {
    let start = Instant::now();
    let mut events = Vec::new();

    let mut stream = client.post("/question/stream")
        .json(&serde_json::json!({
            "question": prompt,
            "model": {"provider": "anthropic", "model": model}
        }))
        .send()
        .await?
        .bytes_stream();

    while let Some(chunk) = stream.next().await {
        // Parse SSE events
    }

    BenchmarkResult {
        duration_ms: start.elapsed().as_millis() as u64,
        // ... metrics
    }
}
```
