---
title: Page-Agent Re-act Loop — The Core Execution Engine
---

# Page-Agent Re-act Loop — The Core Execution Engine

**The Re-act loop is the heartbeat of Page-Agent: an infinite loop of observe → think → act that runs until the task is complete, an error occurs, or the step limit is reached.** The loop is implemented in `PageAgentCore.execute()` with 145 lines handling pre-checks, the main while-loop, error recovery, lifecycle hooks, and cleanup.

Source: `packages/core/src/PageAgentCore.ts:210-375` — `execute()` method

## execute() Full Flow

```mermaid
sequenceDiagram
    participant Exec as execute()
    participant Prep as Pre-checks
    participant Loop as While Loop
    participant Obs as Observe
    participant LLM as LLM.invoke()
    participant Macro as MacroTool.execute()
    participant Hook as Lifecycle Hooks
    participant Clean as Finally

    Exec->>Prep: disposed? running? task empty?
    Prep->>Prep: Reset state (history, observations, abortController)
    Prep->>Exec: #setStatus('running')
    Prep->>Hook: onBeforeTask(agent)

    Loop->>Hook: onBeforeStep(agent, step)
    Loop->>Obs: getBrowserState()
    Loop->>Obs: handleObservations(step)
    Loop->>Loop: Assemble messages (system + user prompt)
    Loop->>LLM: invoke(messages, MacroTool, signal)
    LLM-->>Loop: result with reflection + action + tool input

    alt action === 'done'
        Loop->>Exec: taskResult = { success, data, history }
        Loop->>Exec: finalStatus = 'completed'
        Loop->>Exec: break
    else other action
        Loop->>Macro: execute(input) → tool.execute.bind(this)(input, ctx)
        Macro-->>Loop: output string
    end

    Loop->>Hook: onAfterStep(agent, history)
    Loop->>Loop: step++ → check maxSteps
    Loop->>Obs: loop back

    Exec->>Hook: onAfterTask(agent, taskResult)
    Clean->>Clean: cleanUpHighlights(), hideMask()
    Clean->>Clean: abort(), resolveRunning, #setStatus(finalStatus)
```

## Pre-check Guards

```typescript
// packages/core/src/PageAgentCore.ts:211-214
if (this.disposed) throw new Error('PageAgent has been disposed. Create a new instance.')
if (this.#status === 'running') throw new Error('A task is already running.')
if (!task) throw new Error('Task is required')
```

These guards prevent three classes of bugs: using a disposed agent, concurrent task execution, and empty tasks. All three throw synchronously before any async work begins.

## State Reset Per Task

```typescript
// packages/core/src/PageAgentCore.ts:219-222
this.history = []
this.#observations = []
this.#states = { totalWaitTime: 0, lastURL: '', browserState: null }
this.#abortController = new AbortController()
```

Each `execute()` call gets a fresh state: empty history, no observations, zero wait time, and a new `AbortController`. This means you can call `execute()` multiple times on the same agent instance without cross-task contamination.

## The While Loop

```typescript
// packages/core/src/PageAgentCore.ts:251-359
while (true) {
    await onBeforeStep?.(this, step)

    try {
        // Step delay (skip on step 0)
        if (step > 0) await waitFor(stepDelay, signal)
        signal.throwIfAborted()

        // Observe
        this.#states.browserState = await this.pageController.getBrowserState()
        await this.#handleObservations(step)

        // Assemble prompt
        const messages = [
            { role: 'system', content: this.#getSystemPrompt() },
            { role: 'user', content: await this.#assembleUserPrompt() },
        ]

        // Invoke LLM with MacroTool
        const result = await this.#llm.invoke(messages, macroTool, signal, {
            toolChoiceName: 'AgentOutput',
            normalizeResponse: (res) => normalizeResponse(res, this.tools),
        })

        // Parse result
        const actionName = Object.keys(input.action)[0]
        // ... push history, check for 'done'

    } catch (error) {
        // Handle errors (abort vs. actual errors)
    } finally {
        console.groupEnd()
        await onAfterStep?.(this, this.history)
    }

    step++
    if (step > maxSteps) { /* error out */ }
}
```

**Aha:** The step delay is AFTER observing and assembling the prompt but BEFORE the LLM call, on step > 0. This means step 0 runs immediately (good UX — the agent starts working right away), but subsequent steps have a 0.4s cooldown (configurable via `stepDelay`). The delay also happens INSIDE the try block, meaning it can be aborted — calling `stop()` during a delay cancels immediately rather than waiting for the delay to expire.

## The MacroTool: Merging All Tools Into One

```typescript
// packages/core/src/PageAgentCore.ts:386-469
#packMacroTool(): Tool<MacroToolInput, MacroToolResult> {
    const actionSchemas = Array.from(tools.entries()).map(([toolName, tool]) => {
        return z.object({ [toolName]: tool.inputSchema }).describe(tool.description)
    })

    const actionSchema = z.union(actionSchemas as [...z.ZodType[])

    const macroToolSchema = z.object({
        evaluation_previous_goal: z.string().optional(),
        memory: z.string().optional(),
        next_goal: z.string().optional(),
        action: actionSchema,
    })

    return {
        description: 'You MUST call this tool every step!',
        inputSchema: macroToolSchema as z.ZodType<MacroToolInput>,
        execute: async (input: MacroToolInput): Promise<MacroToolResult> => {
            // Find and execute the selected tool
            const toolName = Object.keys(input.action)[0]
            const toolInput = input.action[toolName]
            const tool = tools.get(toolName)
            const result = await tool.execute.bind(this)(toolInput, { signal })
            // Track wait time, emit activity events
            return { input, output: result }
        },
    }
}
```

**Aha:** The MacroTool is rebuilt from `this.tools` on every step via `#packMacroTool()`, not cached. This means custom tools added dynamically are immediately available to the LLM. The union schema is constructed by mapping over `tools.entries()` — each tool contributes a `{ [toolName]: tool.inputSchema }` branch. Zod validates the LLM output against this union, so the LLM literally cannot select a non-existent tool.

### Why Not Individual Tool Calls?

Standard OpenAI tool calling lets the model call multiple tools per turn. Page-Agent forces exactly ONE tool call per step via `toolChoiceName: 'AgentOutput'`. This has three benefits:

1. **Enforced reflection.** The unified schema requires the model to produce `evaluation_previous_goal`, `memory`, and `next_goal` alongside its action. These fields become part of the agent's persistent history.
2. **Cleaner history.** One step = one reflection + one action. No complex multi-tool-call history to parse.
3. **Simpler error recovery.** If the LLM produces invalid JSON, the retry only needs to produce ONE valid object, not a list of tool calls.

### The Prompt Assembly in Detail

```typescript
// packages/core/src/PageAgentCore.ts:579-647
async #assembleUserPrompt(): Promise<string> {
    // <instructions> (optional)
    let prompt = ''
    prompt += await this.#getInstructions()  // system + page-specific + llms.txt
    // <agent_state>
    prompt += '<agent_state>\n'
    prompt += '<user_request>\n' + this.task + '\n</user_request>\n'
    prompt += '<step_info>\nStep N of M max possible steps\n</step_info>\n'
    prompt += '</agent_state>\n\n'
    // <agent_history>
    for (const event of this.history) {
        if (event.type === 'step') {
            // <step_N> with reflection + action results
        } else if (event.type === 'observation') {
            // <sys> content
        }
    }
    // <browser_state>
    prompt += '<browser_state>\n'
    prompt += browserState.header + '\n'    // URL, title, page dimensions
    prompt += pageContent + '\n'             // Simplified HTML of interactive elements
    prompt += browserState.footer + '\n'     // Scroll position hints
    prompt += '</browser_state>\n\n'
    return prompt
}
```

## Error Recovery

```typescript
// packages/core/src/PageAgentCore.ts:326-337
catch (error: unknown) {
    const isAbortError = (error as any)?.name === 'AbortError'
    if (!isAbortError) console.error('Task failed', error)
    const message = isAbortError ? 'Task aborted' : String(error)
    this.#emitActivity({ type: 'error', message })
    this.#emitHistoryChange({ type: 'error', message, rawResponse: error })
    taskResult = { success: false, data: message, history: this.history }
    finalStatus = isAbortError ? 'stopped' : 'error'
    break
}
```

Abort errors (from `stop()`) get status `'stopped'`. All other errors get status `'error'`. Both paths produce a valid `ExecutionResult` — errors don't throw out of `execute()`. Only "external errors" (pre-check failures, lifecycle hook throws) propagate as exceptions.

## Cleanup Guarantees

```typescript
// packages/core/src/PageAgentCore.ts:368-374
finally {
    await suppress(() => this.pageController.cleanUpHighlights())
    await suppress(() => this.pageController.hideMask())
    this.#abortController.abort()  // Prevent stale operations
    resolveRunning()               // Unblock stop()
    this.#setStatus(finalStatus)
}
```

The `finally` block runs regardless of success, error, or abort. `suppress()` swallows exceptions so one cleanup failure doesn't prevent the others. `resolveRunning()` unblocks any concurrent `stop()` call. `#abortController.abort()` prevents stale operations from the completed task.

## Cross-References

- [Architecture](01-architecture.md) — Event system and lifecycle states
- [Tools](03-tools.md) — Built-in and custom tool definitions
- [Page Controller](04-page-controller.md) — How `getBrowserState()` works

## Next

→ [Tools](03-tools.md) — Built-in tools, custom tool creation, and the MacroTool union schema.
