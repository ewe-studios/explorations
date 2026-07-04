---
title: Page-Agent Tools — Built-in Tools, Custom Tools, and the MacroTool Pattern
---

# Page-Agent Tools — Built-in Tools, Custom Tools, and the MacroTool Pattern

**Tools are the action primitives that Page-Agent uses to interact with web pages.** Each tool is a Zod-validated function with access to the agent's `this` context and an `AbortSignal` for cancellation. The built-in tools cover DOM interaction, navigation, and task control. Custom tools extend the agent with domain-specific capabilities.

Source: `packages/core/src/tools/index.ts`, `packages/core/src/types.ts`

## Tool Interface

```typescript
// packages/core/src/tools/index.ts:21-26
export interface PageAgentTool<TParams = any> {
    description: string              // LLM sees this — must describe what and when
    inputSchema: z.ZodType<TParams>  // Zod v4 schema for parameter validation
    execute: (this: PageAgentCore, args: TParams, ctx: ToolContext) => Promise<string>
}

export interface ToolContext {
    signal: AbortSignal               // Cooperative cancellation
}
```

**Aha:** The `execute` function is bound to the `PageAgentCore` instance via `.bind(this)` at call time. This gives every tool access to `this.pageController`, `this.tools`, `this.history`, and `this.config` without explicit parameter passing. The return type is always `Promise<string>` — the string becomes part of the agent's history as the "Action Results" for that step.

## Built-in Tools

```typescript
// packages/core/src/tools/index.ts:36-100+
export const tools = new Map<string, PageAgentTool>()
```

### Task Control

| Tool | Description | Key Parameters |
|------|-------------|---------------|
| `done` | Complete the task | `text: string`, `success: boolean` (default true) |
| `wait` | Wait N seconds | `seconds: number` (1-10, default 1) |
| `ask_user` | Ask user a question | `question: string` |

### DOM Interaction

| Tool | Description | Key Parameters |
|------|-------------|---------------|
| `click_element_by_index` | Click element by its index number | `index: int` |
| `input_text_element_by_index` | Type text into an input | `index: int`, `text: string` |
| `scroll_vertically_by_index` | Scroll an element | `index: int`, `direction: 'up'\|'down'`, `amount: 'page'\|'half_page'` |
| `scroll_horizontally_by_index` | Horizontal scroll | `index: int`, `direction: 'left'\|'right'` |
| `select_option_element_by_index` | Select dropdown option | `index: int`, `options: string[]` |
| `hover_element_by_index` | Hover over element | `index: int` |
| `drag_and_drop` | Drag element to target | `source_index: int`, `target_index: int` |

### Navigation

| Tool | Description | Key Parameters |
|------|-------------|---------------|
| `go_back` | Browser back | — |
| `go_forward` | Browser forward | — |
| `reload_page` | Reload current page | — |
| `new_page` | Open URL in new tab | `url: string` |
| `switch_tab` | Switch to tab by index | `tab_index: int` |
| `close_page` | Close current tab | — |

### Experimental

| Tool | Description | Guard |
|------|-------------|-------|
| `execute_javascript` | Run arbitrary JS | `experimentalScriptExecutionTool: true` |

The `wait` tool has a clever optimization:

```typescript
// packages/core/src/tools/index.ts:61-72
execute: async function (this: PageAgentCore, input, { signal }) {
    // Subtract LLM calling time from the actual wait
    const lastTimeUpdate = await this.pageController.getLastUpdateTime()
    const secondsSinceLastUpdate = (Date.now() - lastTimeUpdate) / 1000
    const actualWaitTime = Math.max(0, input.seconds - secondsSinceLastUpdate)
    await waitFor(actualWaitTime, signal)
    return `✅ Waited for ${(secondsSinceLastUpdate + actualWaitTime).toFixed(2)} seconds.`
}
```

**Aha:** The `wait` tool compensates for LLM latency. If the LLM took 2.3 seconds to produce the action, and the tool is asked to wait 5 seconds, it actually waits only 2.7 seconds. This means "wait 5 seconds" means "5 seconds from when the page was last updated," not "5 seconds from now." This prevents drift where each step's accumulated LLM latency makes waits progressively longer than intended.

## Custom Tools

Custom tools are registered via `config.customTools`:

```typescript
// packages/core/src/types.ts:49
customTools?: Record<string, PageAgentTool | null>

// A value of `null` removes the built-in tool of that name
```

### Adding a Custom Tool

```typescript
import { z } from 'zod/v4'
import { tool } from 'page-agent'

const customTools = {
    extract_table: tool({
        description: 'Extract data from a table on the page. Use for structured data.',
        inputSchema: z.object({
            table_description: z.string().describe('How to identify the table'),
        }),
        execute: async function (this, input, { signal }) {
            // this.pageController is available
            const html = await this.pageController.getPageHTML()
            // ... parse table, return CSV/JSON string
            return JSON.stringify(extractedData)
        },
    }),
}

const agent = new PageAgent({
    baseURL: '...', model: '...', apiKey: '...',
    customTools,
})
```

### Overriding a Built-in Tool

```typescript
const customTools = {
    done: tool({
        description: 'Complete the task with a structured summary',
        inputSchema: z.object({
            text: z.string(),
            success: z.boolean().default(true),
            summary: z.object({ actions_taken: z.number(), result: z.string() }),
        }),
        execute: async function (this, input) {
            return `Task ${input.success ? 'completed' : 'failed'}: ${input.text}`
        },
    }),
}
```

### Removing a Built-in Tool

```typescript
const customTools = {
    ask_user: null,  // Never allow asking the user
}

// Or with the experimental guard
const agent = new PageAgent({
    experimentalScriptExecutionTool: false,  // execute_javascript is removed by default
})
```

## Tool Registration Flow

```typescript
// packages/core/src/PageAgentCore.ts:134-146
if (this.config.customTools) {
    for (const [name, tool] of Object.entries(this.config.customTools)) {
        if (tool === null) {
            this.tools.delete(name)      // Remove tool
            continue
        }
        this.tools.set(name, tool)        // Add or override
    }
}

if (!this.config.experimentalScriptExecutionTool) {
    this.tools.delete('execute_javascript')
}
```

Tools are registered in constructor, after the LLM retry listener is set up but before any task runs. This is the only mutation window — during execution, the tool registry is effectively immutable (though `#packMacroTool()` reads from it fresh each step).

## Cross-References

- [Agent Loop](02-agent-loop.md) — How tools are invoked via MacroTool
- [Page Controller](04-page-controller.md) — The DOM methods tools call
- [Architecture](01-architecture.md) — How customTools config flows through

## Next

→ [Page Controller](04-page-controller.md) — DOM flattening and element interaction.
