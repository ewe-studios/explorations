# LLM Agent Framework Deep Dive

## Overview

LLM is a Go library for building AI agents across multiple providers (OpenAI, Anthropic, Gemini, Ollama). It provides a unified interface for chat completions with streaming support, tool calling, and extended thinking capabilities.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       LLM Library                             │
├─────────────────────────────────────────────────────────────┤
│  Client                                                      │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Client                                               │   │
│  │    - Provider management                              │   │
│  │    - Multi-provider routing                           │   │
│  │    - Tool registry                                    │   │
│  └──────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│  Provider Layer                                              │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐   │
│  │  OpenAI   │ │ Anthropic │ │  Gemini   │ │  Ollama   │   │
│  │ Provider  │ │ Provider  │ │ Provider  │ │ Provider  │   │
│  └───────────┘ └───────────┘ └───────────┘ └───────────┘   │
├─────────────────────────────────────────────────────────────┤
│  Core abstractions                                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Message    │  │  ToolCall    │  │    Usage     │      │
│  │   ChatReq    │  │  ToolSchema  │  │   Thinking   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
├─────────────────────────────────────────────────────────────┤
│  Streaming Layer                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  iter.Seq2   │  │   Batch      │  │  Sandbox     │      │
│  │  (Generator) │  │  Executor    │  │  Executor    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

## Provider Interface

All providers implement a common interface:

```go
// llm/llm.go
type Provider interface {
    Name() string
    Model(ctx context.Context, id string) (*Model, error)
    Models(ctx context.Context) ([]*Model, error)
    Chat(ctx context.Context, req *ChatRequest) iter.Seq2[*ChatResponse, error]
}
```

This unified interface allows the LLM client to work with any provider interchangeably.

## Client Implementation

### Provider Management

```go
// Client manages providers
type Client struct {
    providers []Provider
}

// New creates a new Client
func New(providers ...Provider) *Client {
    return &Client{providers}
}

func (c *Client) findProvider(name string) (Provider, error) {
    for _, p := range c.providers {
        if p.Name() == name {
            return p, nil
        }
    }
    return nil, fmt.Errorf("llm: provider %q not found", name)
}
```

### Chat Method

The Chat method handles the full conversation loop with tool execution:

```go
func (c *Client) Chat(ctx context.Context, provider string, options ...Option) iter.Seq2[*ChatResponse, error] {
    return func(yield func(*ChatResponse, error) bool) {
        config := &Config{
            Thinking: ThinkingMedium,
        }
        for _, option := range options {
            option(config)
        }

        provider, err := c.findProvider(provider)
        if err != nil {
            yield(nil, err)
            return
        }

        // Build tool registry
        toolbox := map[string]Tool{}
        for _, tool := range config.Tools {
            schema := tool.Schema()
            toolbox[schema.Function.Name] = tool
        }

        // Maintain conversation state
        messages := append([]*Message{}, config.Messages...)

    turn:
        for steps := 0; steps < config.MaxSteps || config.MaxSteps == 0; steps++ {
            req := &ChatRequest{
                Model:    config.Model,
                Thinking: config.Thinking,
                Tools:    toolSchemas(config.Tools),
                Messages: messages,
            }

            batch, ctx := batch.New[*Message](ctx)

            // Stream response from provider
            for res, err := range provider.Chat(ctx, req) {
                if err != nil {
                    if !yield(res, err) {
                        break turn
                    }
                    continue
                }

                // Save message for conversation history
                messages = append(messages, &Message{
                    Role:     res.Role,
                    Thinking: res.Thinking,
                    Content:  res.Content,
                    ToolCall: res.ToolCall,
                })

                // Handle tool call
                if res.ToolCall != nil {
                    tool, ok := toolbox[res.ToolCall.Name]
                    if !ok {
                        if !yield(nil, fmt.Errorf("llm: unknown tool %q called by model", res.ToolCall.Name)) {
                            break turn
                        }
                        continue
                    }

                    // Yield tool call to caller
                    if !yield(res, err) {
                        break turn
                    }

                    // Execute tool in goroutine
                    batch.Go(func() (*Message, error) {
                        result, err := tool.Run(ctx, res.ToolCall.Arguments)
                        if err != nil {
                            // Return error as tool result message
                            return &Message{
                                Role:       "tool",
                                Content:    `{"error":` + strconv.Quote(err.Error()) + `}`,
                                ToolCallID: res.ToolCall.ID,
                            }, nil
                        }
                        return &Message{
                            Role:       "tool",
                            Content:    string(result),
                            ToolCallID: res.ToolCall.ID,
                        }, nil
                    })
                }

                // Wait if there are tool calls to process
                if batch.Size() > 0 {
                    continue
                }

                // Yield response
                if !yield(res, err) {
                    break
                }
            }

            // Wait for all tool calls to complete
            toolResults, err := batch.Wait()
            if err != nil {
                if !yield(nil, err) {
                    break
                }
            }

            // If no tool results, conversation turn is complete
            if len(toolResults) == 0 {
                break turn
            }

            // Add tool results to conversation
            for _, message := range toolResults {
                messages = append(messages, message)
                if !yield(&ChatResponse{
                    Role:       message.Role,
                    Thinking:   message.Thinking,
                    Content:    message.Content,
                    ToolCallID: message.ToolCallID,
                }, nil) {
                    break turn
                }
            }
        }
    }
}
```

## Message Types

```go
// Message represents a chat message
type Message struct {
    Role       string    `json:"role,omitzero"`
    Content    string    `json:"content,omitzero"`
    Thinking   string    `json:"thinking,omitzero"`     // Chain-of-thought content
    ToolCall   *ToolCall `json:"tool_call,omitzero"`    // Assistant tool invocation
    ToolCallID string    `json:"tool_call_id,omitzero"` // Tool response correlation
}

// ChatResponse represents a streaming response
type ChatResponse struct {
    Role       string    `json:"role,omitzero"`
    Content    string    `json:"content,omitzero"`
    Thinking   string    `json:"thinking,omitzero"`
    ToolCall   *ToolCall `json:"tool_call,omitzero"`
    ToolCallID string    `json:"tool_call_id,omitzero"`
    Usage      *Usage    `json:"usage,omitzero"`
    Done       bool      `json:"done,omitzero"`
}
```

## Tool System

### Tool Interface

```go
// llm/tool.go
type Tool interface {
    Schema() *ToolSchema
    Run(ctx context.Context, in json.RawMessage) (out []byte, err error)
}
```

### ToolCall Structure

```go
type ToolCall struct {
    ID               string          `json:"id,omitzero"`
    Name             string          `json:"name,omitzero"`
    Arguments        json.RawMessage `json:"arguments,omitzero"`
    ThoughtSignature []byte          `json:"thought_signature,omitzero"`
}
```

### Tool Schema

```go
type ToolSchema struct {
    Type     string
    Function *ToolFunction
}

type ToolFunction struct {
    Name        string
    Description string
    Parameters  *ToolFunctionParameters
}

type ToolFunctionParameters struct {
    Type       string
    Properties map[string]*ToolProperty
    Required   []string
}

type ToolProperty struct {
    Type        string
    Description string
    Enum        []string
    Items       *ToolProperty  // For array types
}
```

### Typed Function Tool

The `Func` helper creates type-safe tools with automatic JSON marshaling:

```go
// Func creates a typed tool with automatic JSON marshaling
func Func[In, Out any](name, description string, run func(ctx context.Context, in In) (Out, error)) Tool {
    return &typedFunc[In, Out]{
        name:        name,
        description: description,
        run:         run,
    }
}

type typedFunc[In, Out any] struct {
    name        string
    description string
    run         func(ctx context.Context, in In) (Out, error)
}

func (t *typedFunc[In, Out]) Schema() *ToolSchema {
    var in In
    return &ToolSchema{
        Type: "function",
        Function: &ToolFunction{
            Name:        t.name,
            Description: t.description,
            Parameters:  generateSchema(in),
        },
    }
}

func (t *typedFunc[In, Out]) Run(ctx context.Context, args json.RawMessage) ([]byte, error) {
    var in In
    if len(args) > 0 {
        if err := json.Unmarshal(args, &in); err != nil {
            return nil, fmt.Errorf("tool %s: unmarshaling input: %w", t.name, err)
        }
    }
    out, err := t.run(ctx, in)
    if err != nil {
        return nil, err
    }
    return json.Marshal(out)
}
```

### Schema Generation

```go
// generateSchema creates ToolFunctionParameters from a struct type
func generateSchema(v any) *ToolFunctionParameters {
    params := &ToolFunctionParameters{
        Type:       "object",
        Properties: make(map[string]*ToolProperty),
        Required:   []string{},
    }

    t := reflect.TypeOf(v)
    if t.Kind() == reflect.Ptr {
        t = t.Elem()
    }
    if t.Kind() != reflect.Struct {
        return params
    }

    for i := range t.NumField() {
        field := t.Field(i)
        if !field.IsExported() {
            continue
        }

        // Get JSON field name
        name := field.Name
        if jsonTag := field.Tag.Get("json"); jsonTag != "" {
            parts := strings.Split(jsonTag, ",")
            if parts[0] != "" && parts[0] != "-" {
                name = parts[0]
            }
        }

        // Get description
        description := field.Tag.Get("description")

        // Get enums
        var enums []string
        if enumTag := field.Tag.Get("enums"); enumTag != "" {
            enums = strings.Split(enumTag, ",")
        }

        prop := schemaType(field.Type)
        prop.Description = description
        prop.Enum = enums
        params.Properties[name] = prop

        // Check if required
        if field.Tag.Get("is") == "required" {
            params.Required = append(params.Required, name)
        }
    }

    return params
}

func schemaType(t reflect.Type) *ToolProperty {
    for t.Kind() == reflect.Ptr {
        t = t.Elem()
    }

    prop := &ToolProperty{Type: "string"}
    switch t.Kind() {
    case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64,
         reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
        prop.Type = "integer"
    case reflect.Float32, reflect.Float64:
        prop.Type = "number"
    case reflect.Bool:
        prop.Type = "boolean"
    case reflect.Slice, reflect.Array:
        prop.Type = "array"
        prop.Items = schemaType(t.Elem())
    case reflect.Struct, reflect.Map:
        prop.Type = "object"
    }

    return prop
}
```

### Tool Usage Example

```go
// Define a tool with typed input/output
add := llm.Func("add", "Add two numbers", func(ctx context.Context, in struct {
    A int `json:"a" description:"First number"`
    B int `json:"b" description:"Second number"`
}) (int, error) {
    return in.A + in.B, nil
})

// Use in chat
for event, err := range client.Chat(
    ctx,
    "openai",
    llm.WithModel("gpt-4"),
    llm.WithMessage(llm.UserMessage("Add 20 and 22")),
    llm.WithTool(add),
) {
    if err != nil {
        panic(err)
    }
    fmt.Print(event.Content)
}
```

## Thinking/Reasoning Support

```go
// Thinking represents the level of extended thinking/reasoning
type Thinking string

const (
    ThinkingNone   Thinking = "none"   // Disable thinking
    ThinkingLow    Thinking = "low"    // Low thinking budget
    ThinkingMedium Thinking = "medium" // Medium thinking budget
    ThinkingHigh   Thinking = "high"   // High thinking budget
)
```

### OpenAI Reasoning Effort

```go
// providers/openai/openai.go
func reasoningEffort(level llm.Thinking) shared.ReasoningEffort {
    switch level {
    case llm.ThinkingLow:
        return shared.ReasoningEffortLow
    case llm.ThinkingMedium:
        return shared.ReasoningEffortMedium
    case llm.ThinkingHigh:
        return shared.ReasoningEffortHigh
    default:
        return shared.ReasoningEffortMedium
    }
}

// In Chat method
if req.Thinking != "" {
    params.Reasoning = shared.ReasoningParam{
        Effort:  reasoningEffort(req.Thinking),
        Summary: shared.ReasoningSummaryDetailed,
    }
}
```

### Anthropic Thinking Budget

```go
// providers/anthropic/anthropic.go
func thinkingBudget(level llm.Thinking) int64 {
    switch level {
    case llm.ThinkingNone, "":
        return 0
    case llm.ThinkingLow:
        return 4000
    case llm.ThinkingMedium:
        return 10000
    case llm.ThinkingHigh:
        return 32000
    default:
        return 0
    }
}

// In Chat method
if budget := thinkingBudget(req.Thinking); budget > 0 {
    params.Thinking = anthropic.ThinkingConfigParamOfEnabled(budget)
    // Extended thinking requires higher max tokens
    if params.MaxTokens < budget+1000 {
        params.MaxTokens = budget + 1000
    }
}
```

## Streaming Architecture

### Go Generators with iter.Seq2

LLM uses Go's new `iter` package for streaming:

```go
type ChatRequest struct {
    Model    string
    Thinking Thinking
    Tools    []*ToolSchema
    Messages []*Message
}

type Provider interface {
    Chat(ctx context.Context, req *ChatRequest) iter.Seq2[*ChatResponse, error]
}
```

### Streaming Event Handling

OpenAI provider streaming example:

```go
func (c *Client) Chat(ctx context.Context, req *llm.ChatRequest) iter.Seq2[*llm.ChatResponse, error] {
    return func(yield func(*llm.ChatResponse, error) bool) {
        // ... setup ...

        stream := c.oc.Responses.NewStreaming(ctx, params)

        var currentFunctionCall *llm.ToolCall
        var functionArgs strings.Builder

        for stream.Next() {
            event := stream.Current()

            switch event.Type {
            case "response.output_text.delta":
                delta := event.AsResponseOutputTextDelta()
                if delta.Delta != "" {
                    if !yield(&llm.ChatResponse{
                        Role:    "assistant",
                        Content: delta.Delta,
                    }, nil) {
                        return
                    }
                }

            case "response.reasoning_summary_text.delta":
                delta := event.AsResponseReasoningSummaryTextDelta()
                if delta.Delta != "" {
                    if !yield(&llm.ChatResponse{
                        Role:     "assistant",
                        Thinking: delta.Delta,
                    }, nil) {
                        return
                    }
                }

            case "response.output_item.added":
                added := event.AsResponseOutputItemAdded()
                if added.Item.Type == "function_call" {
                    currentFunctionCall = &llm.ToolCall{
                        ID:   added.Item.CallID,
                        Name: added.Item.Name,
                    }
                    functionArgs.Reset()
                }

            case "response.function_call_arguments.delta":
                delta := event.AsResponseFunctionCallArgumentsDelta()
                functionArgs.WriteString(delta.Delta)

            case "response.output_item.done":
                done := event.AsResponseOutputItemDone()
                if done.Item.Type == "function_call" && currentFunctionCall != nil {
                    currentFunctionCall.Arguments = json.RawMessage(functionArgs.String())
                    if !yield(&llm.ChatResponse{
                        Role:     "assistant",
                        ToolCall: currentFunctionCall,
                    }, nil) {
                        return
                    }
                    currentFunctionCall = nil
                }

            case "response.completed":
                completed := event.AsResponseCompleted()
                if !yield(&llm.ChatResponse{
                    Role:  "assistant",
                    Done:  true,
                    Usage: toUsage(completed.Response.Usage),
                }, nil) {
                    return
                }
            }
        }

        if err := stream.Err(); err != nil {
            yield(nil, fmt.Errorf("openai: streaming: %w", err))
        }
    }
}
```

## Batch Execution

The batch package handles concurrent tool execution:

```go
// internal/batch/batch.go
func New[B any](ctx context.Context) (*Batch[B], context.Context) {
    eg, ctx := errgroup.WithContext(ctx)
    return &Batch[B]{eg: eg}, ctx
}

type Batch[B any] struct {
    eg   *errgroup.Group
    mu   sync.RWMutex
    next int
    out  []B
}

func (b *Batch[B]) Go(fn func() (B, error)) {
    b.mu.Lock()
    idx := b.next
    b.next++
    b.out = append(b.out, *new(B)) // reserve slot
    b.mu.Unlock()

    b.eg.Go(func() error {
        result, err := fn()
        if err != nil {
            return err
        }
        b.mu.Lock()
        b.out[idx] = result
        b.mu.Unlock()
        return nil
    })
}

func (b *Batch[B]) Wait() ([]B, error) {
    if err := b.eg.Wait(); err != nil {
        return nil, err
    }
    return b.out, nil
}
```

## Configuration Options

```go
type Config struct {
    Log      *slog.Logger
    Model    string
    Thinking Thinking
    Tools    []Tool
    Messages []*Message
    MaxSteps int
}

// WithModel sets the model for the agent
func WithModel(model string) Option {
    return func(c *Config) {
        c.Model = model
    }
}

// WithThinking sets the extended thinking level
func WithThinking(level Thinking) Option {
    return func(c *Config) {
        c.Thinking = level
    }
}

// WithTool adds a tool to the agent
func WithTool(tools ...Tool) Option {
    return func(c *Config) {
        c.Tools = append(c.Tools, tools...)
    }
}

// WithMessage sets initial conversation history
func WithMessage(messages ...*Message) Option {
    return func(c *Config) {
        c.Messages = append(c.Messages, messages...)
    }
}

// WithMaxSteps sets the maximum number of tool execution steps
func WithMaxSteps(max int) Option {
    return func(c *Config) {
        c.MaxSteps = max
    }
}
```

## Message Helpers

```go
// SystemMessage creates a system message
func SystemMessage(content string) *Message {
    return &Message{
        Role:    "system",
        Content: content,
    }
}

// UserMessage creates a user message
func UserMessage(content string) *Message {
    return &Message{
        Role:    "user",
        Content: content,
    }
}

// AssistantMessage creates an assistant message
func AssistantMessage(content string) *Message {
    return &Message{
        Role:    "assistant",
        Content: content,
    }
}
```

## Model Management

```go
// Model represents an available model
type Model struct {
    Provider string     // Provider name
    ID       string     // Model identifier
    Meta     *ModelMeta // Model metadata
}

// ModelMeta contains curated model information
type ModelMeta struct {
    DisplayName     string    // Human-friendly name
    KnowledgeCutoff time.Time // Zero time if unknown
    ContextWindow   int       // Maximum context window in tokens
    MaxOutputTokens int       // Maximum output tokens
    HasReasoning    bool      // Supports chain-of-thought
}

// Models returns all available models across providers
func (c *Client) Models(ctx context.Context, providers ...string) ([]*Model, error) {
    eg, ctx := errgroup.WithContext(ctx)
    for _, provider := range filterProviders(c.providers, providers...) {
        eg.Go(func() error {
            m, err := provider.Models(ctx)
            if err != nil {
                return err
            }
            models = append(models, m...)
            return nil
        })
    }
    if err := eg.Wait(); err != nil {
        return nil, err
    }
    // Sort by provider, then ID
    sort.Slice(models, func(i, j int) bool {
        if models[i].Provider == models[j].Provider {
            return models[i].ID < models[j].ID
        }
        return models[i].Provider < models[j].Provider
    })
    return models, nil
}
```

## Sandbox Execution

The sandbox package provides isolated command execution:

```go
// sandbox/sandbox.go
type Executor interface {
    Run(ctx context.Context, cmd *Cmd) error
}

type Cmd struct {
    exec   Executor
    ctx    context.Context
    Path   string
    Args   []string
    Dir    string
    Env    []string
    Stdin  io.Reader
    Stdout io.Writer
    Stderr io.Writer
}

func (c *Cmd) Run() error {
    return c.exec.Run(c.ctx, c)
}
```

## Complete Example

```go
package main

import (
    "context"
    "fmt"
    "os"

    "github.com/matthewmueller/llm"
    "github.com/matthewmueller/llm/providers/openai"
    "github.com/matthewmueller/llm/providers/anthropic"
)

func main() {
    ctx := context.Background()

    // Create client with multiple providers
    client := llm.New(
        openai.New(os.Getenv("OPENAI_API_KEY")),
        anthropic.New(os.Getenv("ANTHROPIC_API_KEY")),
    )

    // Define tools
    add := llm.Func("add", "Add two numbers", func(ctx context.Context, in struct {
        A int `json:"a" description:"First number"`
        B int `json:"b" description:"Second number"`
    }) (int, error) {
        return in.A + in.B, nil
    })

    search := llm.Func("search", "Search the web", func(ctx context.Context, in struct {
        Query string `json:"query" description:"Search query"`
    }) (string, error) {
        // Implementation...
        return "Search results", nil
    })

    // Chat with tool use
    for event, err := range client.Chat(
        ctx,
        "openai",
        llm.WithModel("gpt-4"),
        llm.WithThinking(llm.ThinkingMedium),
        llm.WithMessage(llm.UserMessage("What's 20 + 22?")),
        llm.WithTool(add, search),
        llm.WithMaxSteps(5),
    ) {
        if err != nil {
            panic(err)
        }

        if event.Content != "" {
            fmt.Print(event.Content)
        }
        if event.Thinking != "" {
            fmt.Printf("\n[Thinking: %s]\n", event.Thinking)
        }
        if event.ToolCall != nil {
            fmt.Printf("\n[Calling tool: %s]\n", event.ToolCall.Name)
        }
        if event.Done {
            fmt.Printf("\n[Usage: %d input, %d output tokens]\n", 
                event.Usage.InputTokens, event.Usage.OutputTokens)
        }
    }
}
```

## Error Handling

```go
type ErrMultipleModels struct {
    Provider string
    Name     string
    Matches  []*Model
}

func (e *ErrMultipleModels) Error() string {
    matchStr := ""
    for _, m := range e.Matches {
        matchStr += fmt.Sprintf("- Provider: %q, Model: %q\n", m.Provider, m.ID)
    }
    if e.Provider == "" {
        return fmt.Sprintf("llm: multiple models found for %q:\n%s", e.Name, matchStr)
    }
    return fmt.Sprintf("llm: multiple models found for %q from provider %q:\n%s", 
        e.Name, e.Provider, matchStr)
}
```

## Summary

The LLM library provides:

1. **Unified Provider Interface** - Work with OpenAI, Anthropic, Gemini, Ollama interchangeably
2. **Streaming Responses** - Real-time token streaming using Go generators
3. **Tool Calling** - Type-safe tool definition with automatic JSON marshaling
4. **Extended Thinking** - Configurable reasoning budgets across providers
5. **Batch Execution** - Concurrent tool execution with errgroup
6. **Model Discovery** - List and query available models
7. **Sandbox Support** - Isolated command execution for tools
8. **Multi-turn Conversations** - Automatic conversation state management

The architecture centers on the Provider interface, allowing seamless switching between AI providers while maintaining consistent behavior for streaming, tool calling, and reasoning capabilities.
