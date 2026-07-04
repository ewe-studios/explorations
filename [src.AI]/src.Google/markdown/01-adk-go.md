---
title: adk-go — Google's Agent Development Kit for Go
---

# adk-go — Google's Agent Development Kit for Go

**adk-go is Google's official Go SDK for building AI agents — a code-first, multi-agent framework where the Agent tree acts as a routing topology, not just a containment hierarchy.** The architecture has three layers that must be understood to use ADK correctly: the public interfaces (agent, model, tool, session), the internal `Flow` (a pipeline of 12 ordered request processors + 2 response processors that transform a raw invocation into an LLM call), and the `Runner` (which finds the correct agent in the tree, builds the `InvocationContext`, and persists events to SQLite or Vertex AI). Every layer leaks its design decisions upward — if you don't understand the Flow, you can't debug why your tool isn't being called.

Source: `adk-go/` — `agent/agent.go`, `internal/llminternal/base_flow.go`, `runner/runner.go`, `session/session.go`, `tool/tool.go`

## 1. The Agent Interface

```go
// agent/agent.go:43-52
type Agent interface {
    Name() string
    Description() string
    Run(InvocationContext) iter.Seq2[*session.Event, error]
    SubAgents() []Agent
    FindAgent(name string) Agent
    FindSubAgent(name string) Agent
    internal() *agent
}
```

### How Create-Time Validation Works

```go
// agent/agent.go:55-74
func New(cfg Config) (Agent, error) {
    subAgentSet := make(map[Agent]bool)
    for _, subAgent := range cfg.SubAgents {
        if _, ok := subAgentSet[subAgent]; ok {
            return nil, fmt.Errorf(
                "error creating agent: subagent %q appears multiple times in subAgents",
                subAgent.Name())
        }
        subAgentSet[subAgent] = true
    }
    return &agent{
        name: cfg.Name, description: cfg.Description,
        subAgents: cfg.SubAgents,
        beforeAgentCallbacks: cfg.BeforeAgentCallbacks,
        run: cfg.Run,
        afterAgentCallbacks: cfg.AfterAgentCallbacks,
        State: agentinternal.State{
            AgentType: agentinternal.TypeCustomAgent,
        },
    }, nil
}
```

### Config — Every Field Used by the Runner

```go
// agent/agent.go:77-107
type Config struct {
    Name        string
    Description string    // MUST be one line. The LLM reads this to decide delegation.
    SubAgents   []Agent   // ADK auto-sets parent on each sub-agent for tree traversal.

    BeforeAgentCallbacks []BeforeAgentCallback  // Return non-nil → skip agent Run entirely
    Run func(InvocationContext) iter.Seq2[*session.Event, error]
    AfterAgentCallbacks  []AfterAgentCallback
}
```

`AfterAgentCallbacks` closes over the `Config` pointer via `defer pluginManager.RunAfterRunCallback(ctx)` — plugins can read the final event sequence to compute quality metrics or log token usage.

### InvocationContext

```go
// agent/context.go (interface, generated from internal)
type InvocationContext interface {
    context.Context              // Deadlines, cancellation, values
    Agent() Agent               // The agent currently running (may differ from root)
    Artifacts() Artifacts       // Versioned file store
    Memory() Memory             // Cross-session memory scoped to user+app
    Session() Session           // The session this invocation belongs to
    InvocationID() string       // UUID for tracing
    Branch() string             // Dotted agent path: "root_agent.coding_agent"
    UserContent() *genai.Content // The user's message
    RunConfig() *RunConfig       // Streaming mode, save artifacts, etc.
    EndInvocation()              // Signal runner to stop early
    State() map[string]any      // Shared mutable KV across the agent tree
}
```

**Aha: `Branch()` encodes the agent delegation path as a dotted string.** When the root agent transfers to `coding_agent`, the branch becomes `"root.coding_agent"`. All events are stamped with this branch. In `buildContentsDefault()`, the branch filter prunes events that don't belong to the current agent's branch — this is how peer agents are isolated from each other's conversation history.

## 2. LLMAgent — 40+ Configuration Fields

`llmagent.Config` wraps the base `agent.Config` with every LLM-specific option:

```go
// agent/llmagent/llmagent.go:130+
type Config struct {
    // --- Base agent fields ---
    Name, Description string; SubAgents []agent.Agent
    Before/AfterAgentCallbacks

    // --- Model ---
    Model  model.LLM                     // e.g., gemini.NewModel()
    Tools  []tool.Tool                   // Function tools registered inline
    Toolsets []tool.Toolset             // Dynamic tool sets — this is how MCP tools arrive

    // --- Instructions ---
    Instruction string    // System prompt. Template-safe: {{.State.key}} works.
    InstructionProvider func(ctx) string  // Called per-turn for dynamic instructions
    GlobalInstruction string              // Only on root agent. Applies to entire tree.
    GlobalInstructionProvider func(ctx) string

    // --- Schema constraints ---
    InputSchema  *genai.Schema   // Pydantic-style: validate user input against this
    OutputSchema *genai.Schema   // Structured output enforcement (response_mime_type=json)
    OutputKey    string          // Auto-save final text output to session.State under this key

    // --- Content control ---
    IncludeContents IncludeContents  // "default" | "all" | "none"
    // "default" — full conversation history
    // "all"     — includes peer agent events
    // "none"    — only the current turn (fresh context, no history)

    // --- Transfer control ---
    DisallowTransferToParent bool  // Prevent this agent from transferring to parent
    DisallowTransferToPeers  bool  // Prevent this agent from transferring to siblings

    // --- Callback chains: 6 distinct interception points ---
    BeforeModelCallbacks  []BeforeModelCallback   // Return non-nil → cached response, skip LLM
    AfterModelCallbacks   []AfterModelCallback    // Log tokens, validate, transform
    OnModelErrorCallbacks []OnModelErrorCallback  // Retry logic, fallback models
    BeforeToolCallbacks   []BeforeToolCallback    // Intercept args, add auth headers, skip
    AfterToolCallbacks    []AfterToolCallback     // Validate result, log, transform
    OnToolErrorCallbacks  []OnToolErrorCallback   // Fallback, retry, report

    // --- Generation config ---
    GenerateContentConfig *genai.GenerateContentConfig  // Temperature, topP, topK, maxOutputTokens, etc.
}
```

### How the 6 Callback Chains Compose

```
BeforeModelCallbacks[i] → AfterModelCallbacks[i] ≠ BeforeToolCallbacks[j] → AfterToolCallbacks[j]
```

Model and tool callbacks are separate chains. A `BeforeModelCallback` returning `(cachedResponse, nil)` skips the LLM call entirely — the cached response is yielded as if the model produced it. This is the caching primitive: intercept the request, check a cache key (e.g., hash of messages), return the cached `LLMResponse`.

```go
// internal/llminternal/base_flow.go:50-60
type BeforeModelCallback func(
    ctx agent.CallbackContext, llmRequest *model.LLMRequest,
) (*model.LLMResponse, error)

type AfterModelCallback func(
    ctx agent.CallbackContext, llmResponse *model.LLMResponse,
    llmResponseError error,
) (*model.LLMResponse, error)

type OnModelErrorCallback func(
    ctx agent.CallbackContext, llmRequest *model.LLMRequest,
    llmResponseError error,
) (*model.LLMResponse, error)
```

**Aha: Error callbacks can recover.** `OnModelErrorCallback` returns `*model.LLMResponse` — if it returns a non-nil response, the error is suppressed and the agent continues with that response. This is how fallback model routing works: on 429, the callback can call a different model and return its response.

## 3. The Flow — 12 Request Processors in Strict Order

This is the internal pipeline that transforms an `InvocationContext` into an LLM call. Every LLMAgent runs through these processors. The order is fixed and load-bearing.

```go
// internal/llminternal/base_flow.go:62-98
type Flow struct {
    Model model.LLM
    Tools []tool.Tool
    RequestProcessors  []func(ctx, req, f) iter.Seq2[*session.Event, error]  // 12 stages
    ResponseProcessors []func(ctx, req, resp) error                          // 2 stages
    Before/AfterModelCallbacks, OnModelErrorCallbacks
    Before/AfterToolCallbacks, OnToolErrorCallbacks
}

var DefaultRequestProcessors = []func(...){
    basicRequestProcessor,              // 1.  Set model, config, system instruction
    toolProcessor,                      // 2.  Resolve Tools/Toolsets → f.Tools
    authPreprocessor,                   // 3.  Inject auth tokens into tool schemas
    RequestConfirmationRequestProcessor,// 4.  Add tool confirmation prompts
    instructionsRequestProcessor,       // 5.  Inject Instruction + InstructionProvider
    identityRequestProcessor,           // 6.  Inject agent identity (name, description)
    ContentsRequestProcessor,           // 7.  Build Contents from session.Events
    nlPlanningRequestProcessor,         // 8.  NL Planning: multi-step decomposition
    codeExecutionRequestProcessor,      // 9.  Code execution data file optimization
    outputSchemaRequestProcessor,       // 10. Enforce OutputSchema on the request
    AgentTransferRequestProcessor,      // 11. Add transfer_to_agent tool if sub-agents exist
    removeDisplayNameIfExists,          // 12. Strip display_name from tools (API compat)
}

var DefaultResponseProcessors = []func(...){
    nlPlanningResponseProcessor,        // Unmark planning thought contents
    codeExecutionResponseProcessor,     // Post-process code execution results
}
```

### Processor 7: ContentsRequestProcessor — The Conversation Builder

```go
// internal/llminternal/contents_processor.go:36-63
func ContentsRequestProcessor(ctx agent.InvocationContext, req *model.LLMRequest, f *Flow) iter.Seq2[*session.Event, error] {
    return func(yield func(*session.Event, error) bool) {
        llmAgent := asLLMAgent(ctx.Agent())
        if llmAgent == nil { return }
        fn := buildContentsDefault
        if llmAgent.internal().IncludeContents == "none" {
            fn = buildContentsCurrentTurnContextOnly
        }
        var events []*session.Event
        if ctx.Session() != nil {
            for e := range ctx.Session().Events().All() {
                events = append(events, e)
            }
        }
        contents, err := fn(ctx.Agent().Name(), ctx.Branch(), events)
        // ...
        req.Contents = append(req.Contents, contents...)
    }
}
```

`buildContentsDefault` does six things to every event before it reaches the LLM:

```go
// internal/llminternal/contents_processor.go:67-94
func buildContentsDefault(agentName, invocationBranch string, events []*session.Event) ([]*genai.Content, error) {
    var filtered []*session.Event
    for _, ev := range events {
        content := utils.Content(ev)
        // 1. SKIP events with no content AND no transcription
        if (content == nil || content.Role == "" || len(content.Parts) == 0) &&
            ev.LLMResponse.InputTranscription == nil &&
            ev.LLMResponse.OutputTranscription == nil { continue }
        // 2. SKIP events not on this agent's branch
        if !eventBelongsToBranch(invocationBranch, ev) { continue }
        // 3. SKIP excluded events (tool confirmations, etc.)
        if shouldExcludeEvent(ev) { continue }
        // 4. CONVERT foreign agent replies into user messages
        if isOtherAgentReply(agentName, ev) {
            filtered = append(filtered, ConvertForeignEvent(ev))
        } else {
            filtered = append(filtered, ev)
        }
    }
    // 5. AGGREGATE transcription events into text Content
    // 6. POST-PROCESS function calls/responses
}
```

**Aha: Foreign agent replies become user messages.** When agent A transfers to agent B, agent B sees A's response as if it were a user message — `ConvertForeignEvent(ev)` changes the role from `assistant` to `user`. This is the abstraction that makes the transfer seamless: agent B doesn't know it's receiving a delegated task; it thinks the user asked directly.

### Processor 11: AgentTransferRequestProcessor — The Delegation Mechanism

```go
// internal/llminternal/agent_transfer.go:69-97
func AgentTransferRequestProcessor(ctx agent.InvocationContext, req *model.LLMRequest, f *Flow) iter.Seq2[*session.Event, error] {
    return func(yield func(*session.Event, error) bool) {
        agent := ctx.Agent()
        if !shouldUseAutoFlow(agent) { return }
        parents := parentmap.FromContext(ctx)
        targets := transferTargets(agent, parents[agent.Name()])
        if len(targets) == 0 { return }
        transferToAgentTool, err := NewTransferToAgentTool(agent, parents[agent.Name()], targets)
        // ...
        utils.AppendInstructions(req, transferToAgentTool.instructions)
        appendTools(req, transferToAgentTool)
    }
}
```

This processor dynamically injects a `transfer_to_agent` tool into every LLM call — but only if the agent has sub-agents or an accessible parent/peers. The tool's `Declaration()` method generates a Gemini function declaration whose `agent_name` enum is the list of valid transfer targets:

```go
// internal/llminternal/agent_transfer.go:135-151
func (t *TransferToAgentTool) Declaration() *genai.FunctionDeclaration {
    return &genai.FunctionDeclaration{
        Name:        t.Name(),                           // "transfer_to_agent"
        Description: t.Description(),
        Parameters: &genai.Schema{
            Type: genai.TypeObject,
            Properties: map[string]*genai.Schema{
                "agent_name": {
                    Type: genai.TypeString,
                    Description: "the agent name to transfer to",
                    Enum: t.enums(),                     // ["coder", "reviewer", "parent"]
                },
            },
            Required: []string{"agent_name"},
        },
    }
}
```

When the LLM calls `transfer_to_agent(agent_name="coder")`, the tool's `Run()` method sets `ctx.Actions().TransferToAgent = "coder"`. The runner then routes the next turn to that agent via `findAgentToRun()`.

**Aha: Peer transfers require a specific parent type.** `transferTargets()` checks that the parent is ALSO an `LLMAgent` before allowing peer transfers — if the parent is a custom agent, peers can't see each other.

```go
// internal/llminternal/agent_transfer.go:185-200
func transferTargets(agent, parent agent.Agent) []agent.Agent {
    targets := slices.Clone(agent.SubAgents())
    llmAgent := asLLMAgent(agent)
    llmParent := asLLMAgent(parent)
    if llmParent == nil { return targets }         // No parent → no peer/parent transfers
    if !llmAgent.internal().DisallowTransferToParent {
        targets = append(targets, parent)           // Can transfer up
    }
    // Peer transfers only when parent is LLMAgent AND DisallowTransferToPeers is false
    if llmParent != nil && !llmAgent.internal().DisallowTransferToPeers {
        for _, sib := range parent.SubAgents() {
            if sib.Name() != agent.Name() { targets = append(targets, sib) }
        }
    }
    return targets
}
```

## 4. The Streaming Response Aggregator

The `streamingResponseAggregator` is the bridge between Gemini's streaming chunks and ADK's discrete events. Gemini streams partial `GenerateContentResponse` chunks — text arrives character by character, function call arguments arrive as JSONPath-patched partial objects. The aggregator accumulates these into complete `LLMResponse` values.

```go
// internal/llminternal/stream_aggregator.go:33-50
type streamingResponseAggregator struct {
    usageMetadata     *genai.GenerateContentResponseUsageMetadata
    groundingMetadata *genai.GroundingMetadata
    citationMetadata  *genai.CitationMetadata
    response          *model.LLMResponse

    currentThoughtSignature []byte           // Thought signature from Gemini 3 thinking
    sequence               []*genai.Part     // Accumulated complete parts
    currentTextBuffer      string            // Text being streamed in
    currentTextIsThought   bool              // Whether current text is a "thought" block
    finishReason           genai.FinishReason

    currentFunctionName             string             // Function being streamed
    currentFunctionID               string             // Call ID for the function
    currentFunctionArgs             map[string]any     // Partial args being built via JSONPath
    currentFunctionThoughtSignature []byte
}
```

### How Partial Function Call Args Accumulate

When Gemini streams a function call, arguments arrive as `PartialArg` chunks — each chunk says "set `$.args.name` to `"Jo"`". The aggregator walks JSONPath into `currentFunctionArgs`:

```go
// internal/llminternal/stream_aggregator.go:149-172
func (s *streamingResponseAggregator) processStreamingFunctionCallPart(part *genai.Part) {
    if part.FunctionCall.Name != "" { s.currentFunctionName = part.FunctionCall.Name }
    if part.FunctionCall.ID != "" { s.currentFunctionID = part.FunctionCall.ID }
    for _, arg := range part.FunctionCall.PartialArgs {
        jsonPath := arg.JsonPath        // e.g., "$.args.recipient.name"
        if jsonPath == "" { continue }
        value, ok := s.getValueFromPartialArg(arg, jsonPath)
        if !ok { continue }
        s.setValueByJSONPath(jsonPath, value)  // Walk/recurse into map[string]any
    }
    if part.FunctionCall.WillContinue != nil && *part.FunctionCall.WillContinue {
        return  // More chunks coming — keep accumulating
    }
    s.flushTextBufferToSequence()
    s.flushFunctionCallToSequence()  // Emit complete FunctionCall part
}
```

**Aha: Text and function call parts are interleaved during streaming.** Gemini can alternate: text chunk → function call chunk → text chunk. The aggregator handles this by flushing buffered text before and after each function call emission. The `sequence` slice preserves the order in which complete parts arrived.

### The Aggregate-and-Yield Pattern

```go
// internal/llminternal/stream_aggregator.go:59-77
func (s *streamingResponseAggregator) ProcessResponse(ctx context.Context,
    genResp *genai.GenerateContentResponse) iter.Seq2[*model.LLMResponse, error] {
    return func(yield func(*model.LLMResponse, error) bool) {
        resp := converters.Genai2LLMResponse(genResp)
        if len(genResp.Candidates) > 0 {
            candidate := genResp.Candidates[0]
            resp.TurnComplete = candidate.FinishReason != ""
        }
        // MAY yield an intermediate aggregated response (partial function call ready)
        if aggrResp := s.aggregateResponse(resp); aggrResp != nil {
            if !yield(aggrResp, nil) { return }
        }
        // ALWAYS yield the raw GenAI→LLMResponse conversion
        if !yield(resp, nil) { return }
    }
}
```

Each streaming chunk can produce 0–2 yield events: an intermediate aggregated event (when a complete part is ready) plus the raw response. The runner ignores intermediate partials for persistence — only `!event.LLMResponse.Partial` events are committed to session storage.

## 5. The Runner — From User Message to Agent Tree

```go
// runner/runner.go:131-267
func (r *Runner) Run(ctx context.Context, userID, sessionID string, msg *genai.Content,
    cfg agent.RunConfig, opts ...RunOption) iter.Seq2[*session.Event, error] {
    return func(yield func(*session.Event, error) bool) {
        // 1. Get or create session
        getResp, err := r.sessionService.Get(ctx, &session.GetRequest{...})
        if err != nil {
            if !r.autoCreateSession { yield(nil, err); return }
            createResp, err := r.sessionService.Create(ctx, &session.CreateRequest{...})
            storedSession = createResp.Session
        } else { storedSession = getResp.Session }

        // 2. Find which agent should run — NOT always the root
        agentToRun, err := r.findAgentToRun(storedSession, msg)

        // 3. Build InvocationContext with ALL services
        ctx := icontext.NewInvocationContext(ctx, icontext.InvocationContextParams{
            Artifacts: artifacts, Memory: memoryImpl,
            Session: storedSession, Agent: agentToRun,
            UserContent: msg, RunConfig: &cfg,
        })

        // 4. Append user message to session
        ctx, err = r.appendMessageToSession(ctx, storedSession, msg, ...)

        // 5. Run BEFORE callbacks (plugins) — can short-circuit
        earlyExitResult, err := pluginManager.RunBeforeRunCallback(ctx)
        if earlyExitResult != nil { ... }

        // 6. Execute agent → stream events
        for event, err := range agentToRun.Run(ctx) {
            // 6a. Run ON_EVENT plugin callbacks (can transform events)
            if pluginManager != nil {
                modifiedEvent, _ := pluginManager.RunOnEventCallback(ctx, event)
                if modifiedEvent != nil { event = modifiedEvent }
            }
            // 6b. Persist non-partial events to session storage
            if !event.LLMResponse.Partial {
                r.sessionService.AppendEvent(ctx, storedSession, event)
            }
            // 6c. Yield to caller (streaming UI)
            if !yield(event, nil) { return }  // Consumer stopped early
        }
    }
}
```

### Partial Events: Streamed, Not Stored

```go
// runner/runner.go:255-261
if !event.LLMResponse.Partial {
    if err := r.sessionService.AppendEvent(ctx, storedSession, event); err != nil {
        yield(nil, fmt.Errorf("failed to add event to session: %w", err))
        return
    }
}
```

**Aha: `Partial` is a tri-state pattern, not a boolean.** The flag is set by the aggregator for every streaming chunk. Only the final chunk of a turn has `Partial: false`. This is the breakpoint between "show the user streaming text" and "save this to the conversation history for the next LLM call." If a chunk is partial, the runner skips `AppendEvent` — it's ephemeral UI content.

## 6. Session — Four Interfaces, One Contract

```go
// session/session.go:32-46
type Session interface {
    ID() string; AppName() string; UserID() string
    State() State
    Events() Events
    LastUpdateTime() time.Time
}

// session/session.go:51-62
type State interface {
    Get(string) (any, error)           // Returns ErrStateKeyNotExist if missing
    Set(string, any) error             // Overwrites existing
    All() iter.Seq2[string, any]       // Iterate all key-value pairs
}

// session/session.go:79-87
type Events interface {
    All() iter.Seq[*Event]             // Sequential iterator
    Len() int
    At(i int) *Event                   // Random access by index
}
```

### The Event — Embeds LLMResponse

```go
// session/session.go:92-118
type Event struct {
    model.LLMResponse                  // Embedded: Content, FinishReason, Partial, etc.
    ID, InvocationID string            // Set by storage + context
    Timestamp    time.Time
    Branch       string                // "root_agent.coding_agent.linter"
    Author       string                // "user" or agent name
    Actions      EventActions          // TransferToAgent, StateDelta, etc.
    LongRunningToolIDs []string        // For polling async tool results
}
```

**Aha: `Branch` is how peer isolation works.** When the root agent spawns two sub-agents (`coder` and `reviewer`), each sees only events on its branch. The `buildContentsDefault` function calls `eventBelongsToBranch(invocationBranch, ev)` — a prefix match against the dotted branch string. `coder` events are on branch `"root.coder"`; `reviewer` events are on `"root.reviewer"`. Neither sees the other's conversation.

### EventActions — What Agents Can Do Beyond Text

```go
type EventActions struct {
    TransferToAgent string           // Set by transfer_to_agent tool → triggers findAgentToRun
    StateDelta      map[string]any   // Merged into session.State
    ArtifactDelta  map[string]string // Files to save
    SkipSummarization bool           // Don't include this event in conversation summaries
    EndInvocation   bool             // Signal runner to stop after this event
}
```

## 7. Tool System — Tool, Toolset, and MCP

```go
// tool/tool.go:38-46
type Tool interface {
    Name() string
    Description() string
    IsLongRunning() bool  // Returns resource ID first, finishes later
}

// tool/tool.go:57-64
type Toolset interface {
    Name() string
    Tools(ctx agent.ReadonlyContext) ([]Tool, error)  // Dynamic — re-evaluated per-turn
}
```

**Aha: `Toolset.Tools()` receives a `ReadonlyContext` and can return different tools per invocation.** This is the MCP integration point — `mcptoolset.Set` implements `Toolset` and calls `list_tools` on the MCP server each turn. If the MCP server adds or removes tools dynamically, the agent picks up the changes without restarting.

```go
// tool/tool.go:112-121
func (f *filteredToolset) Tools(ctx agent.ReadonlyContext) ([]Tool, error) {
    tools, _ := f.toolset.Tools(ctx)
    var filtered []Tool
    for _, tool := range tools {
        if f.predicate(ctx, tool) { filtered = append(filtered, tool) }
    }
    return filtered, nil
}
```

Tool filtering is composable — `FilterToolset` wraps any `Toolset` with a predicate. This enables per-agent tool restrictions: the parent agent sees all tools; sub-agent sees only `[read_file, grep_files]`.

## 8. The Agent Tree — findAgentToRun()

The runner's most critical method determines which agent handles the next message:

```
Scan session.Events backward from most recent:
1. Find the last non-user event
2. Extract its Author → that's the agent that produced it
3. Find that agent in the tree
4. Check Actions.TransferToAgent on any event → if set, find THAT agent instead
5. Return the resolved agent
```

This is NOT the root agent (except on the first message). It's the agent that last responded, creating conversational continuity across transfers.

## Cross-References

- [stitch-sdk](02-stitch-sdk.md) — Google's AI UI SDK (parallel ADK adapter)
- [Goose](../src.block/markdown/01-architecture.md) — Block's agent (compares Agent loop vs Flow pipeline)

## Next

→ [stitch-sdk](02-stitch-sdk.md) — Google's TypeScript SDK for AI-powered UI generation.
