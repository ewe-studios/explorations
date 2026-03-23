---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.MCP/modelcontextprotocol
repository: https://github.com/modelcontextprotocol/modelcontextprotocol
explored_at: 2026-03-23T00:00:00Z
language: TypeScript
---

# Deep Dive: MCP Protocol Specification (modelcontextprotocol)

## Overview

The `modelcontextprotocol` repository contains the official MCP specification, JSON Schema definitions, and documentation. It serves as the single source of truth for the protocol that all SDK implementations follow.

**Key Insight:** The schema is defined in TypeScript first (not JSON Schema directly), then exported as JSON Schema. This allows for better type safety, documentation, and maintainability.

## Repository Structure

```
modelcontextprotocol/
├── schema/
│   └── 2025-11-25/
│       ├── schema.ts          # TypeScript type definitions (2586 lines)
│       └── schema.json        # Generated JSON Schema
│
├── docs/                      # Mintlify documentation source
│   ├── spec/                  # Specification sections
│   │   ├── basic/             # Base protocol (JSON-RPC, init, ping)
│   │   ├── server/            # Server features (tools, resources, prompts)
│   │   ├── client/            # Client features (sampling, roots)
│   │   └── utilities/         # Logging, completions
│   └── introduction.md
│
├── seps/                      # Standards Enhancement Proposals
│   ├── sep-001-...
│   └── ...
│
├── tools/                     # Protocol tooling
│   └── schema-export/         # JSON Schema generation
│
├── blog/                      # Protocol announcements
├── scripts/                   # Build and validation scripts
└── .claude-plugin/            # Claude integration
```

## Protocol Version History

The MCP protocol uses date-based versioning:

| Version | Date | Key Features |
|---------|------|--------------|
| 2024-11-05 | Nov 2024 | Initial release |
| 2025-03-26 | Mar 2025 | Enhanced capabilities |
| 2025-06-18 | Jun 2025 | Task augmentation |
| 2025-11-25 | Nov 2025 | Current latest |

## Protocol Mechanics

### 1. JSON-RPC Foundation

MCP is built on JSON-RPC 2.0 with specific message types:

```typescript
// All JSON-RPC messages follow this pattern
type JSONRPCMessage =
  | JSONRPCRequest      // Request expecting response
  | JSONRPCNotification // Fire-and-forget
  | JSONRPCResponse     // Success or error response
  | JSONRPCErrorResponse;

// Standard JSON-RPC 2.0
const JSONRPC_VERSION = "2.0";
```

### 2. Message Structure

#### Requests
```typescript
interface JSONRPCRequest {
  jsonrpc: "2.0";
  id: RequestId;        // number | string
  method: string;
  params?: { ... };
}
```

#### Notifications
```typescript
interface JSONRPCNotification {
  jsonrpc: "2.0";
  method: string;
  params?: NotificationParams;
}
```

#### Responses
```typescript
interface JSONRPCResultResponse {
  jsonrpc: "2.0";
  id: RequestId;
  result: Result;
}

interface JSONRPCErrorResponse {
  jsonrpc: "2.0";
  id?: RequestId;
  error: {
    code: number;
    message: string;
    data?: unknown;
  };
}
```

### 3. Standard Error Codes

```typescript
// JSON-RPC standard errors
const PARSE_ERROR = -32700;
const INVALID_REQUEST = -32600;
const METHOD_NOT_FOUND = -32601;
const INVALID_PARAMS = -32602;
const INTERNAL_ERROR = -32603;

// MCP-specific errors
const URL_ELICITATION_REQUIRED = -32042;
```

### 4. Protocol Features by Category

#### Server-Provided Features

| Feature | Methods | Notifications |
|---------|---------|---------------|
| **Tools** | `tools/list`, `tools/call` | `notifications/tools/list_changed` |
| **Resources** | `resources/list`, `resources/read`, `resources/subscribe`, `resources/unsubscribe` | `notifications/resources/list_changed`, `notifications/resources/updated` |
| **Prompts** | `prompts/list`, `prompts/get` | `notifications/prompts/list_changed` |
| **Logging** | `logging/setLevel` | `notifications/logging/message` |
| **Completions** | `completion/complete` | - |

#### Client-Provided Features

| Feature | Methods | Notifications |
|---------|---------|---------------|
| **Sampling** | `sampling/createMessage` | - |
| **Roots** | `roots/list` | `notifications/roots/list_changed` |
| **Elicitation** | `elicitation/create` | - |

#### Shared Features

| Feature | Methods | Notifications |
|---------|---------|---------------|
| **Initialization** | `initialize` | `notifications/initialized` |
| **Ping** | `ping` | - |
| **Progress** | - | `notifications/progress` |
| **Cancellation** | - | `notifications/cancelled` |
| **Tasks** | `tasks/list`, `tasks/get`, `tasks/cancel` | - |

## Key Types Deep Dive

### 1. Protocol Version

```typescript
export const LATEST_PROTOCOL_VERSION = "2025-11-25";

// Version negotiation during initialization
interface InitializeRequestParams {
  protocolVersion: string;  // Client's latest supported version
  capabilities: ClientCapabilities;
  clientInfo: Implementation;
}

interface InitializeResult {
  protocolVersion: string;  // Server's chosen version
  capabilities: ServerCapabilities;
  serverInfo: Implementation;
  instructions?: string;    // Usage hints for LLM
}
```

### 2. Capabilities

**Client Capabilities:**
```typescript
interface ClientCapabilities {
  experimental?: { [key: string]: object };
  roots?: { listChanged?: boolean };
  sampling?: {
    context?: object;   // Supports includeContext parameter
    tools?: object;     // Supports tool use in sampling
  };
  elicitation?: { form?: object; url?: object };
  tasks?: {
    list?: object;
    cancel?: object;
    requests?: {
      sampling?: { createMessage?: object };
      elicitation?: { create?: object };
    };
  };
}
```

**Server Capabilities:**
```typescript
interface ServerCapabilities {
  experimental?: { [key: string]: object };
  logging?: object;
  completions?: object;
  prompts?: { listChanged?: boolean };
  resources?: {
    subscribe?: boolean;
    listChanged?: boolean;
  };
  tools?: { listChanged?: boolean };
  tasks?: {
    list?: object;
    cancel?: object;
    requests?: {
      tools?: { call?: object };
    };
  };
}
```

### 3. Tools

```typescript
interface Tool extends BaseMetadata, Icons {
  description?: string;
  inputSchema: {
    type: "object";
    properties?: { [key: string]: JsonSchema };
    required?: string[];
  };
  outputSchema?: {
    type: "object";
    properties?: { [key: string]: JsonSchema };
    required?: string[];
  };
  annotations?: ToolAnnotations;
  taskSupport?: "required" | "optional" | "forbidden";
  _meta?: { [key: string]: unknown };
}

// Tool call request/response
interface CallToolRequestParams {
  name: string;
  arguments?: { [key: string]: unknown };
  task?: TaskMetadata;  // For task-augmented execution
}

interface CallToolResult {
  content: (TextContent | ImageContent | AudioContent | ResourceLink)[];
  isError?: boolean;
}
```

### 4. Resources

```typescript
interface Resource extends BaseMetadata, Icons {
  uri: string;               // RFC 3986 URI
  description?: string;
  mimeType?: string;
  annotations?: Annotations;
  size?: number;             // Raw bytes, before encoding
  _meta?: { [key: string]: unknown };
}

// Resource templates for dynamic URIs
interface ResourceTemplate {
  uriTemplate: string;       // RFC 6570 URI template
  description?: string;
  mimeType?: string;
  annotations?: Annotations;
}

// Resource contents (text or binary)
type ResourceContents = TextResourceContents | BlobResourceContents;

interface TextResourceContents {
  uri: string;
  mimeType?: string;
  text: string;
}

interface BlobResourceContents {
  uri: string;
  mimeType?: string;
  blob: string;              // Base64-encoded
}
```

### 5. Prompts

```typescript
interface Prompt extends BaseMetadata, Icons {
  description?: string;
  arguments?: PromptArgument[];
  _meta?: { [key: string]: unknown };
}

interface PromptArgument {
  name: string;
  description?: string;
  required?: boolean;
}

interface GetPromptResult {
  description?: string;
  messages: PromptMessage[];
}

interface PromptMessage {
  role: "user" | "assistant";
  content: TextContent | ImageContent | AudioContent;
}
```

### 6. Sampling

```typescript
// Server requests LLM completion from client
interface CreateMessageRequestParams {
  messages: SamplingMessage[];
  modelPreferences?: ModelPreferences;
  systemPrompt?: string;
  includeContext?: "none" | "thisServer" | "allServers";
  temperature?: number;
  maxTokens: number;
  stopSequences?: string[];
  metadata?: { [key: string]: unknown };
  tools?: Tool[];
  toolChoice?: "auto" | "any" | "none" | { type: "tool"; name: string };
}

interface CreateMessageResult {
  message: SamplingMessage;
  model: string;
  stopReason?: "endTurn" | "stopSequence" | "maxTokens" | string;
}

interface SamplingMessage {
  role: "user" | "assistant";
  content: TextContent | ImageContent | AudioContent;
}
```

### 7. Roots

```typescript
interface Root {
  uri: string;              // Typically file:// URI
  name?: string;
}

interface ListRootsResult {
  roots: Root[];
}
```

### 8. Notifications

**Progress Notification:**
```typescript
interface ProgressNotificationParams {
  progressToken: ProgressToken;  // Matches request's progressToken
  progress: number;              // Current progress
  total?: number;                // Total if known
  message?: string;              // Human-readable status
}
```

**Cancellation Notification:**
```typescript
interface CancelledNotificationParams {
  requestId: RequestId;   // The request to cancel
  reason?: string;        // Optional explanation
}
```

## Schema Generation

The TypeScript schema is used to generate JSON Schema for interoperability:

```typescript
// Build script (simplified)
import { $ } from "zx";
import { mkdir, writeFile } from "fs/promises";

async function build() {
  await mkdir("dist", { recursive: true });

  // Generate JSON Schema from TypeScript types
  const schema = await generateJsonSchema();
  await writeFile("dist/schema.json", JSON.stringify(schema, null, 2));
}
```

## Server Implementation Patterns

### 1. Request Handling

```typescript
// Server handles requests by method
async function handleRequest(request: JSONRPCRequest): Promise<JSONRPCResponse> {
  switch (request.method) {
    case "initialize":
      return handleInitialize(request);
    case "tools/list":
      return handleListTools(request);
    case "tools/call":
      return handleCallTool(request);
    // ... more handlers
    default:
      return {
        jsonrpc: "2.0",
        id: request.id,
        error: { code: METHOD_NOT_FOUND, message: `Unknown method: ${request.method}` }
      };
  }
}
```

### 2. Capability Declaration

```typescript
const serverCapabilities: ServerCapabilities = {
  tools: { listChanged: true },
  resources: { subscribe: true, listChanged: true },
  prompts: { listChanged: true },
  logging: {},
  completions: {}
};
```

### 3. Notification Flow

```typescript
// Server sends notifications when state changes
async function addTool(newTool: Tool) {
  tools.push(newTool);

  // Notify clients that tool list changed
  sendNotification({
    jsonrpc: "2.0",
    method: "notifications/tools/list_changed"
  });
}

// Server sends progress for long operations
async function longRunningOperation(progressToken: ProgressToken) {
  for (let i = 0; i < total; i++) {
    await processItem(i);
    sendNotification({
      jsonrpc: "2.0",
      method: "notifications/progress",
      params: {
        progressToken,
        progress: i,
        total,
        message: `Processing item ${i + 1}/${total}`
      }
    });
  }
}
```

## Key Insights for Rust Implementation

1. **Type Safety**: Use Rust's type system to enforce protocol correctness at compile time

2. **Error Handling**: Create a comprehensive error type that covers all JSON-RPC and MCP error cases

3. **Capability Negotiation**: Implement capability checking to ensure both sides support required features

4. **Pagination**: Handle cursors for paginated results (list_resources, list_tools, etc.)

5. **URI Templates**: Implement RFC 6570 URI template expansion for resource templates

6. **Base64 Encoding**: Handle binary resource content with proper base64 encoding/decoding

7. **Progress Tracking**: Implement progress token correlation for long-running operations

8. **Task Augmentation**: Support task-based execution for long-running operations with proper state management

## Open Questions

1. **Elicitation Flow**: The URL elicitation flow requires deeper investigation for complete implementation understanding

2. **Context Inclusion**: The exact behavior of `includeContext: "allServers"` needs clarification

3. **Task State Management**: Detailed state transitions for task-augmented requests could benefit from more examples

## References

- [Official MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
- [Schema Source](./schema/2025-11-25/schema.ts)
- [RFC 6570 - URI Templates](https://tools.ietf.org/html/rfc6570)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
