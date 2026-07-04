# Page-Agent — AI-Powered Browser Automation

Page-Agent is Alibaba's AI-powered browser automation agent. It uses LLMs via OpenAI-compatible APIs to control web pages through a structured "Re-act" loop: Observe (gather DOM state) → Think (LLM produces reflection + action) → Act (execute on real DOM).

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Alibaba/page-agent/` — 62 TypeScript files, 8 packages, MIT license

## Documents

### Foundation

| # | Document | What You'll Learn |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | Re-act loop, 8-package monorepo, quick start |
| 01 | [Architecture](01-architecture.md) | Dependency graph, event system, data flow, lifecycle |

### Deep Dives

| # | Document | What You'll Learn |
|---|----------|-------------------|
| 02 | [Agent Loop](02-agent-loop.md) | Re-act loop deep dive, MacroTool, prompt assembly, step lifecycle |
| 03 | [Tools](03-tools.md) | Built-in tools, custom tool creation, MacroTool union schema |
| 04 | [Page Controller](04-page-controller.md) | DOM flattening, element indexing, actions (click/input/scroll) |

### Cross-Cutting

| # | Document | What You'll Learn |
|---|----------|-------------------|
| 05 | [LLM Integration](05-llm-integration.md) | OpenAI client, retry logic, tool calling, config |
| 06 | [Extension & MCP](06-extension-mcp.md) | Browser extension, multi-page agent, MCP server |

## Quick Start

```html
<script type="module">
  import { PageAgent } from 'page-agent'

  const agent = new PageAgent({
    baseURL: 'https://api.openai.com/v1',
    model: 'gpt-4o',
    apiKey: 'sk-...',
  })

  agent.execute('Book a flight to San Francisco')
</script>
```

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.Alibaba
python3 build.py .
```
