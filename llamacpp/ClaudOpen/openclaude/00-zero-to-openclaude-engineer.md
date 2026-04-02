# Zero to OpenClaude Engineer

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/openclaude`

**Repository:** https://gitlawb.com/z6MkqDnb7Siv3Cwj7pGJq4T5EsUisECqR8KpnDLwcaZq5TPr/openclaude

**Explored at:** 2026-04-02

---

## Table of Contents

1. [What is OpenClaude?](#what-is-openclaude)
2. [Quick Start: Your First 5 Minutes](#quick-start-your-first-5-minutes)
3. [Understanding the Problem OpenClaude Solves](#understanding-the-problem-openclaude-solves)
4. [Core Concepts](#core-concepts)
5. [Environment Configuration Deep-Dive](#environment-configuration-deep-dive)
6. [Provider Ecosystem](#provider-ecosystem)
7. [Building From Source](#building-from-source)
8. [Troubleshooting Fundamentals](#troubleshooting-fundamentals)
9. [Next Steps](#next-steps)

---

## What is OpenClaude?

OpenClaude is a **provider-agnostic fork of Claude Code** that enables you to use **any OpenAI-compatible LLM** instead of being locked into Anthropic's API.

### The One-Liner

> Claude Code + OpenAI-compatible API shim = Use GPT-4o, DeepSeek, Ollama, Gemini, or 200+ models with all of Claude Code's tools.

### What Works Out of the Box

- **All tools**: Bash, FileRead, FileWrite, FileEdit, Glob, Grep, WebFetch, WebSearch, Agent, MCP, LSP, NotebookEdit, Tasks
- **Streaming**: Real-time token streaming
- **Tool calling**: Multi-step tool chains (model calls tools, gets results, continues)
- **Images**: Base64 and URL images passed to vision models
- **Slash commands**: /commit, /review, /compact, /diff, /doctor, etc.
- **Sub-agents**: AgentTool spawns sub-agents using the same provider
- **Memory**: Persistent memory system

### What's Different

- **No thinking mode**: Anthropic's extended thinking is disabled (OpenAI models use different reasoning)
- **No prompt caching**: Anthropic-specific cache headers are skipped
- **No beta features**: Anthropic-specific beta headers are ignored
- **Token limits**: Defaults to 32K max output — some models may cap lower

---

## Quick Start: Your First 5 Minutes

### Step 1: Install Bun (if you don't have it)

```bash
curl -fsSL https://bun.sh/install | bash
```

### Step 2: Clone and Install

```bash
git clone https://node.gitlawb.com/z6MkqDnb7Siv3Cwj7pGJq4T5EsUisECqR8KpnDLwcaZq5TPr/openclaude.git
cd openclaude
bun install
```

### Step 3: Choose Your Provider

#### Option A: OpenAI (Fastest Setup)

```bash
export CLAUDE_CODE_USE_OPENAI=1
export OPENAI_API_KEY=sk-your-key-here
export OPENAI_MODEL=gpt-4o
```

#### Option B: Ollama (Free, Local)

```bash
# First, install Ollama from https://ollama.com
ollama pull llama3.3:70b

export CLAUDE_CODE_USE_OPENAI=1
export OPENAI_BASE_URL=http://localhost:11434/v1
export OPENAI_MODEL=llama3.3:70b
# No API key needed for local models
```

#### Option C: DeepSeek (Cost-Effective)

```bash
export CLAUDE_CODE_USE_OPENAI=1
export OPENAI_API_KEY=sk-your-deepseek-key
export OPENAI_BASE_URL=https://api.deepseek.com/v1
export OPENAI_MODEL=deepseek-chat
```

### Step 4: Run OpenClaude

```bash
# Development mode (builds and runs)
bun run dev

# Or if installed via npm
openclaude
```

---

## Understanding the Problem OpenClaude Solves

### Before OpenClaude

```
┌─────────────────┐
│  Claude Code    │
│                 │
│  Anthropic-only │
│  ─────────────  │
│  $15-100/month  │
│  API costs      │
│  No alternatives│
└─────────────────┘
         │
         ▼
┌─────────────────┐
│ Anthropic API   │
│ (claude-sonnet) │
└─────────────────┘
```

### After OpenClaude

```
┌─────────────────┐
│  OpenClaude     │
│                 │
│  Provider-Agnostic│
│  ────────────────│
│  Any OpenAI-compatible│
│  Local or Cloud │
│  Your choice    │
└─────────────────┘
         │
    ┌────┴────┬────────────┬────────────┐
    ▼         ▼            ▼            ▼
┌────────┐ ┌──────┐  ┌──────────┐ ┌────────┐
│ OpenAI │ │Ollama│  │ DeepSeek │ │ Gemini │
│ GPT-4o │ │Llama │  │ V3       │ │ Flash  │
└────────┘ └──────┘  └──────────┘ └────────┘
```

### Why This Matters

1. **Cost Control**: Use cheaper models for simple tasks, expensive ones for complex work
2. **Privacy**: Run completely local with Ollama — no data leaves your machine
3. **Latency**: Self-hosted models eliminate network round-trips
4. **Model Diversity**: Experiment with different model strengths (DeepSeek for code, GPT-4o for reasoning, etc.)
5. **No Vendor Lock-in**: Switch providers without changing your workflow

---

## Core Concepts

### 1. The Provider Shim

OpenClaude's core innovation is the **OpenAI shim** (`src/services/api/openaiShim.ts`), a translation layer that:

- Accepts Anthropic SDK method calls
- Converts Anthropic message format → OpenAI chat format
- Sends requests to any OpenAI-compatible endpoint
- Converts OpenAI streaming SSE → Anthropic stream events
- Returns data in Anthropic SDK shape

```
Claude Code Tool System
        │
        ▼
Anthropic SDK interface (duck-typed)
        │
        ▼
  openaiShim.ts  ← Translation layer
        │
        ▼
OpenAI Chat Completions API
        │
        ▼
  Any compatible model
```

### 2. Environment Variable Priority

OpenClaude reads configuration in this order:

1. **Explicit environment variables** (highest priority)
2. **Profile file** (`.openclaude-profile.json`)
3. **System defaults** (lowest priority)

### 3. Model Aliases

Instead of hardcoding model names, OpenClaude uses **tier aliases**:

| Claude Tier | OpenAI Default | Purpose |
|-------------|----------------|---------|
| `opus` | `gpt-4o` | Best reasoning, complex tasks |
| `sonnet` | `gpt-4o-mini` | Balanced performance/cost |
| `haiku` | `gpt-4o-mini` | Fast, cheap, simple tasks |

Override with `OPENAI_MODEL` environment variable.

### 4. The Smart Router (Python Extension)

The optional `smart_router.py` adds intelligent routing:

- Pings all configured providers on startup
- Scores them by latency, cost, and health
- Routes each request to the optimal provider
- Falls back automatically if a provider fails
- Learns from real request timings over time

---

## Environment Configuration Deep-Dive

### Required Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `CLAUDE_CODE_USE_OPENAI` | Yes | Set to `1` to enable OpenAI provider |
| `OPENAI_API_KEY` | Yes* | Your API key (*not needed for local models) |
| `OPENAI_MODEL` | Yes | Model name (e.g., `gpt-4o`, `deepseek-chat`, `llama3.3:70b`) |
| `OPENAI_BASE_URL` | No | API endpoint (default: `https://api.openai.com/v1`) |

### Optional Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_BASE` | Alternative to `OPENAI_BASE_URL` |
| `ANTHROPIC_MODEL` | Fallback model name (OpenAI_MODEL takes priority) |
| `API_TIMEOUT_MS` | Request timeout (default: 600000ms) |

### Configuration Examples

#### Production OpenAI Setup

```bash
export CLAUDE_CODE_USE_OPENAI=1
export OPENAI_API_KEY=sk-proj-abc123...
export OPENAI_MODEL=gpt-4o
export OPENAI_BASE_URL=https://api.openai.com/v1
export API_TIMEOUT_MS=120000
```

#### Local Ollama Development

```bash
export CLAUDE_CODE_USE_OPENAI=1
export OPENAI_BASE_URL=http://localhost:11434/v1
export OPENAI_MODEL=llama3.3:70b
# No key needed
```

#### Multi-Provider with OpenRouter

```bash
export CLAUDE_CODE_USE_OPENAI=1
export OPENAI_API_KEY=sk-or-abc123...
export OPENAI_BASE_URL=https://openrouter.ai/api/v1
export OPENAI_MODEL=google/gemini-2.0-flash
```

---

## Provider Ecosystem

### Cloud Providers

#### OpenAI
```bash
export OPENAI_BASE_URL=https://api.openai.com/v1
export OPENAI_MODEL=gpt-4o
```

#### DeepSeek (Cost-effective, great for code)
```bash
export OPENAI_BASE_URL=https://api.deepseek.com/v1
export OPENAI_MODEL=deepseek-chat
```

#### Google Gemini (via OpenRouter)
```bash
export OPENAI_BASE_URL=https://openrouter.ai/api/v1
export OPENAI_MODEL=google/gemini-2.0-flash
```

#### Together AI (Fast inference)
```bash
export OPENAI_BASE_URL=https://api.together.xyz/v1
export OPENAI_MODEL=meta-llama/Llama-3.3-70B-Instruct-Turbo
```

#### Groq (Ultra-low latency)
```bash
export OPENAI_BASE_URL=https://api.groq.com/openai/v1
export OPENAI_API_KEY=gsk_...
export OPENAI_MODEL=llama-3.3-70b-versatile
```

#### Mistral AI
```bash
export OPENAI_BASE_URL=https://api.mistral.ai/v1
export OPENAI_MODEL=mistral-large-latest
```

### Local Providers

#### Ollama
```bash
# Install: https://ollama.com/download
ollama pull llama3.3:70b
export OPENAI_BASE_URL=http://localhost:11434/v1
export OPENAI_MODEL=llama3.3:70b
```

#### LM Studio
```bash
# Download: https://lmstudio.ai
# Start local server in LM Studio
export OPENAI_BASE_URL=http://localhost:1234/v1
export OPENAI_MODEL=your-model-name
```

#### Azure OpenAI
```bash
export OPENAI_BASE_URL=https://your-resource.openai.azure.com/openai/deployments/your-deployment/v1
export OPENAI_API_KEY=your-azure-key
export OPENAI_MODEL=gpt-4o
```

### Model Quality Matrix

| Model | Tool Calling | Code Quality | Speed | Cost |
|-------|-------------|--------------|-------|------|
| GPT-4o | Excellent | Excellent | Fast | High |
| DeepSeek-V3 | Great | Great | Fast | Low |
| Gemini 2.0 Flash | Great | Good | Very Fast | Low |
| Llama 3.3 70B | Good | Good | Medium | Free (local) |
| Mistral Large | Good | Good | Fast | Medium |
| GPT-4o-mini | Good | Good | Very Fast | Low |
| Qwen 2.5 72B | Good | Good | Medium | Free (local) |
| Smaller models (<7B) | Limited | Limited | Very Fast | Free (local) |

---

## Building From Source

### Prerequisites

- **Bun** (v1.2+) — Runtime and bundler
- **Node.js** (v20+) — Compatibility
- **Git** — Source control

### Build Steps

```bash
# 1. Clone the repository
git clone https://node.gitlawb.com/z6MkqDnb7Siv3Cwj7pGJq4T5EsUisECqR8KpnDLwcaZq5TPr/openclaude.git
cd openclaude

# 2. Install dependencies
bun install

# 3. Build the distribution bundle
bun run build

# 4. Verify the build
bun run smoke

# 5. Run in development mode
bun run dev
```

### Build Output

The build produces:
- `dist/cli.mjs` — Bundled CLI entry point
- `dist/cli.mjs.map` — Source map (external)

### Build Configuration

The build script (`scripts/build.ts`) configures:

```javascript
{
  entrypoints: ['./src/entrypoints/cli.tsx'],
  outdir: './dist',
  target: 'node',
  format: 'esm',
  splitting: false,
  sourcemap: 'external',
  minify: false,
  naming: 'cli.mjs',
  define: {
    'MACRO.VERSION': '99.0.0',  // Internal compatibility version
    'MACRO.DISPLAY_VERSION': pkg.version,  // Actual package version
  }
}
```

### Feature Flags (Disabled in Open Build)

The build disables Anthropic-internal features:
- VOICE_MODE, PROACTIVE, KAIROS, BRIDGE_MODE
- DAEMON, AGENT_TRIGGERS, MONITOR_TOOL
- ABLATION_BASELINE, DUMP_SYSTEM_PROMPT
- And more...

---

## Troubleshooting Fundamentals

### Diagnostic Commands

```bash
# Quick startup sanity check
bun run smoke

# Validate provider env + reachability
bun run doctor:runtime

# Print machine-readable runtime diagnostics
bun run doctor:runtime:json

# Persist a diagnostics report
bun run doctor:report

# Full local hardening check
bun run hardening:check

# Strict hardening (includes typecheck)
bun run hardening:strict
```

### Common Issues and Fixes

#### Issue: "Placeholder key (SUA_CHAVE) error"

**Cause:** You're using a placeholder value instead of a real API key.

**Fix:**
```bash
# For OpenAI
export OPENAI_API_KEY=sk-real-key-here

# For Ollama (no key needed)
unset OPENAI_API_KEY
```

#### Issue: "Provider reachability failed"

**Cause:** The API endpoint is unreachable.

**Fix:**
```bash
# Check if Ollama is running
ollama ps

# Start Ollama if needed
ollama serve

# Test connectivity
curl http://localhost:11434/api/tags
```

#### Issue: "Missing key for non-local provider URL"

**Cause:** Using a remote provider URL without an API key.

**Fix:**
```bash
# Set the API key
export OPENAI_API_KEY=your-key-here

# Or switch to local provider
export OPENAI_BASE_URL=http://localhost:11434/v1
```

#### Issue: "Script not found"

**Cause:** Running commands from wrong directory.

**Fix:**
```bash
cd /path/to/openclaude
bun run dev:profile
```

#### Issue: Slow responses with Ollama

**Cause:** Model running on CPU instead of GPU.

**Check:**
```bash
ollama ps
```

If `PROCESSOR` shows `CPU`, consider:
- Using a smaller model (`llama3.1:8b` instead of `llama3.3:70b`)
- Installing GPU drivers for Ollama
- Using a cloud provider instead

---

## Next Steps

You now understand the fundamentals of OpenClaude. Continue your journey:

1. **Read [01-openclaude-exploration.md](./01-openclaude-exploration.md)** — Full architecture deep-dive
2. **Read [production-grade.md](./production-grade.md)** — Production-ready implementation guide
3. **Explore the Python extensions** — Smart router and Ollama provider
4. **Customize the shim** — Add provider-specific optimizations

### Quick Reference Commands

```bash
# Profile management
bun run profile:init -- --provider ollama --model llama3.1:8b
bun run dev:profile

# Diagnostics
bun run doctor:runtime
bun run doctor:report

# Launch presets
bun run dev:fast      # Low latency preset
bun run dev:code      # Better coding quality
```
