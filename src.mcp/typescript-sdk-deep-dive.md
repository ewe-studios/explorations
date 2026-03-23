---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.MCP/typescript-sdk
repository: https://github.com/modelcontextprotocol/typescript-sdk
explored_at: 2026-03-23T00:00:00Z
language: TypeScript
---

# Deep Dive: TypeScript SDK

## Overview

The TypeScript SDK is the reference implementation of the Model Context Protocol. It provides a comprehensive, type-safe library for building MCP clients and servers in TypeScript/JavaScript environments.

**Package:** `@modelcontextprotocol/sdk`
**Version:** 2.0.0-alpha.0 (as of exploration date)
**Runtime:** Node.js 20+, Cloudflare Workers, Browser

## Package Structure

```
typescript-sdk/
├── packages/
│   ├── core/                      # Shared core types and protocol base
│   │   ├── src/
│   │   │   ├── types/             # TypeScript type definitions
│   │   │   │   └── types.ts       # All protocol types (generated from schema)
│   │   │   ├── shared/
│   │   │   │   ├── protocol.ts    # Base Protocol class
│   │   │   │   ├── transport.ts   # Transport interface
│   │   │   │   ├── auth.ts        # Authentication types
│   │   │   │   ├── uriTemplate.ts # RFC 6570 URI templates
│   │   │   │   └── toolNameValidation.ts
│   │   │   ├── errors/
│   │   │   │   └── sdkErrors.ts   # SDK error types
│   │   │   ├── validators/
│   │   │   │   ├── types.ts       # JSON Schema validator interface
│   │   │   │   ├── ajvProvider.ts # AJV-based validator (Node.js)
│   │   │   │   └── cfWorkerProvider.ts # CF Worker validator
│   │   │   ├── util/
│   │   │   │   ├── inMemory.ts    # In-memory storage
│   │   │   │   └── schema.ts      # Schema parsing utilities
│   │   │   └── experimental/      # Task-related experimental features
│   │   └── package.json
│   │
│   ├── client/                    # Client SDK
│   │   ├── src/
│   │   │   ├── client/
│   │   │   │   ├── client.ts      # Main Client class
│   │   │   │   ├── stdio.ts       # Stdio transport
│   │   │   │   ├── sse.ts         # SSE transport
│   │   │   │   ├── streamableHttp.ts # Streamable HTTP
│   │   │   │   ├── websocket.ts   # WebSocket transport
│   │   │   │   ├── auth.ts        # OAuth 2.0 authentication
│   │   │   │   └── middleware.ts  # Client middleware
│   │   │   └── experimental/
│   │   │       └── tasks/
│   │   │           └── client.ts  # Task management
│   │   └── package.json
│   │
│   ├── server/                    # Server SDK
│   │   ├── src/
│   │   │   ├── server/
│   │   │   │   ├── server.ts      # Main Server class
│   │   │   │   ├── mcp.ts         # MCP server helpers
│   │   │   │   ├── stdio.ts       # Stdio transport
│   │   │   │   ├── streamableHttp.ts # Streamable HTTP server
│   │   │   │   └── completable.ts # Completion helpers
│   │   │   └── experimental/
│   │   └── package.json
│   │
│   └── middleware/                # Shared middleware
│       └── src/
│           └── ...
│
├── conformance/                   # Protocol conformance tests
└── examples/                      # Usage examples
```

## Core Architecture

### Protocol Base Class

All MCP communication flows through the `Protocol` base class:

```typescript
abstract class Protocol<Context extends BaseContext> {
  protected _transport?: Transport;
  protected _protocolVersion?: string;
  protected _pendingRequests: Map<RequestId, PendingRequest>;
  protected _requestTimeout: number;

  async connect(transport: Transport): Promise<void> {
    this._transport = transport;
    this._transport.onclose = () => this._onclose();
    this._transport.onerror = (error) => this._onerror(error);
    this._transport.onmessage = (message) => this._onmessage(message);
    await this._transport.start();
  }

  protected async request<T extends JSONRPCRequest>(
    request: T,
    options?: RequestOptions
  ): Promise<JSONRPCResponse> {
    // Request handling with timeout and cancellation
  }

  protected notification(notification: JSONRPCNotification): Promise<void> {
    return this._transport?.send(notification);
  }
}
```

### Client Class

The `Client` class extends `Protocol` for client-side operations:

```typescript
export class Client extends Protocol<ClientContext> {
  private _serverCapabilities?: ServerCapabilities;
  private _serverVersion?: Implementation;
  private _capabilities: ClientCapabilities;
  private _jsonSchemaValidator: JsonSchemaValidator;

  constructor(clientInfo: Implementation, options?: ClientOptions) {
    super();
    this._capabilities = options?.capabilities ?? {};
    this._jsonSchemaValidator = options?.jsonSchemaValidator ?? new DefaultJsonSchemaValidator();
  }

  async connect(transport: Transport): Promise<void> {
    await super.connect(transport);
    // Perform initialization handshake
    const result = await this.request({
      method: "initialize",
      params: {
        protocolVersion: LATEST_PROTOCOL_VERSION,
        capabilities: this._capabilities,
        clientInfo: this.clientInfo
      }
    });
    this._serverCapabilities = result.capabilities;
    this._serverVersion = result.serverInfo;

    // Send initialized notification
    this.notification({ method: "notifications/initialized" });
  }

  // Feature methods
  async listTools(): Promise<ListToolsResult> {
    return this.request({ method: "tools/list" });
  }

  async callTool(params: CallToolRequestParams): Promise<CallToolResult> {
    return this.request({ method: "tools/call", params });
  }

  async listResources(): Promise<ListResourcesResult> {
    return this.request({ method: "resources/list" });
  }

  async readResource(params: ReadResourceRequestParams): Promise<ReadResourceResult> {
    return this.request({ method: "resources/read", params });
  }

  async listPrompts(): Promise<ListPromptsResult> {
    return this.request({ method: "prompts/list" });
  }

  async getPrompt(params: GetPromptRequestParams): Promise<GetPromptResult> {
    return this.request({ method: "prompts/get", params });
  }
}
```

### Server Class

The `Server` class extends `Protocol` for server-side operations:

```typescript
export class Server extends Protocol<ServerContext> {
  private _clientCapabilities?: ClientCapabilities;
  private _clientVersion?: Implementation;
  private _capabilities: ServerCapabilities;
  private _instructions?: string;

  // Tool registration
  tool(name: string, handler: ToolHandler): void {
    this._tools.set(name, handler);
  }

  // Resource registration
  resource(uri: string, handler: ResourceHandler): void {
    this._resources.set(uri, handler);
  }

  // Prompt registration
  prompt(name: string, handler: PromptHandler): void {
    this._prompts.set(name, handler);
  }

  // Notification emission
  async sendToolListChanged(): Promise<void> {
    this.notification({ method: "notifications/tools/list_changed" });
  }

  async sendResourceListChanged(): Promise<void> {
    this.notification({ method: "notifications/resources/list_changed" });
  }

  async sendProgress(params: ProgressNotificationParams): Promise<void> {
    this.notification({ method: "notifications/progress", params });
  }
}
```

## Transport Implementations

### Transport Interface

```typescript
export interface Transport {
  /**
   * Start processing messages
   */
  start(): Promise<void>;

  /**
   * Send a JSON-RPC message
   */
  send(message: JSONRPCMessage): Promise<void>;

  /**
   * Close the connection
   */
  close(): Promise<void>;

  /**
   * Callback for when the connection closes
   */
  onclose?: () => void;

  /**
   * Callback for transport errors
   */
  onerror?: (error: Error) => void;

  /**
   * Callback for incoming messages
   */
  onmessage?: (message: JSONRPCMessage) => void;
}
```

### Stdio Client Transport

```typescript
export class StdioClientTransport implements Transport {
  private _process: ChildProcess;
  private _readBuffer: Buffer = Buffer.alloc(0);

  constructor(options: StdioClientTransportOptions) {
    this._process = spawn(options.command, options.args || [], {
      stdio: ['pipe', 'pipe', 'inherit'],
      env: options.env
    });

    this._process.stdout?.on('data', (data) => {
      this._readBuffer = Buffer.concat([this._readBuffer, data]);
      this._processReadBuffer();
    });
  }

  send(message: JSONRPCMessage): Promise<void> {
    return new Promise((resolve, reject) => {
      const json = JSON.stringify(message) + '\n';
      if (this._process.stdin?.write(json)) {
        resolve();
      } else {
        this._process.stdin?.once('drain', resolve);
      }
    });
  }
}
```

### Streamable HTTP Transport

```typescript
export class StreamableHttpServerTransport implements Transport {
  private _requestToStreamId: Map<RequestId, string> = new Map();
  private _streams: Map<string, WritableStreamDefaultWriter> = new Map();

  async handleRequest(req: Request, res: Response): Promise<void> {
    const sessionId = this.getSessionId(req);

    if (req.method === 'POST') {
      await this.handlePost(req, res, sessionId);
    } else if (req.method === 'GET') {
      await this.handleGet(res, sessionId);
    }
  }

  private async handlePost(req: Request, res: Response, sessionId: string): Promise<void> {
    const body = await req.json();

    if (isJSONRPCRequest(body)) {
      // Handle request and create SSE stream for response
      const streamId = this.generateStreamId();
      this._requestToStreamId.set(body.id, streamId);

      // Process request and send response via SSE
      const result = await this.processRequest(body);
      this.sendSSE(streamId, result);
    }
  }

  send(message: JSONRPCMessage): Promise<void> {
    // Send via SSE stream
    const streamId = this.getMessageStreamId(message);
    const writer = this._streams.get(streamId);
    return writer?.write(`data: ${JSON.stringify(message)}\n\n`);
  }
}
```

## Authentication (OAuth 2.0)

```typescript
export class OAuthClientProvider {
  constructor(private options: OAuthClientProviderOptions) {}

  async authorize(params: AuthorizationParams): Promise<void> {
    // Redirect user to authorization URL
    const authUrl = await this.authorizationUrl(params);
    // Open browser or display URL to user
    await this.openUrl(authUrl);
    // Wait for callback
    const callback = await this.waitForCallback();
    // Exchange code for tokens
    const tokens = await this.exchangeCodeForTokens(callback.code);
    await this.saveTokens(tokens);
  }

  async refreshAccessToken(refreshToken: string): Promise<Tokens> {
    const response = await fetch(this.metadata.token_endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        grant_type: 'refresh_token',
        refresh_token: refreshToken,
        client_id: this.clientId,
        client_secret: this.clientSecret
      })
    });
    return response.json();
  }
}
```

## Error Handling

```typescript
// SDK Error codes
export enum SdkErrorCode {
  RequestTimeout = 'REQUEST_TIMEOUT',
  ConnectionClosed = 'CONNECTION_CLOSED',
  CapabilityNotSupported = 'CAPABILITY_NOT_SUPPORTED',
  InvalidState = 'INVALID_STATE'
}

export class SdkError extends Error {
  constructor(
    public readonly code: SdkErrorCode,
    message: string,
    public readonly cause?: unknown
  ) {
    super(message);
  }
}

// Protocol Error codes (JSON-RPC)
export enum ProtocolErrorCode {
  PARSE_ERROR = -32700,
  INVALID_REQUEST = -32600,
  METHOD_NOT_FOUND = -32601,
  INVALID_PARAMS = -32602,
  INTERNAL_ERROR = -32603
}

export class ProtocolError extends Error {
  constructor(
    public readonly code: ProtocolErrorCode,
    message: string,
    public readonly data?: unknown
  ) {
    super(message);
  }
}
```

## List-Changed Notification Handling

The SDK supports automatic handling of list-changed notifications:

```typescript
const client = new Client(
  { name: 'my-client', version: '1.0.0' },
  {
    listChanged: {
      tools: {
        onChanged: (error, tools) => {
          if (error) {
            console.error('Failed to refresh tools:', error);
            return;
          }
          console.log('Tools updated:', tools);
        }
      },
      prompts: {
        onChanged: (error, prompts) => console.log('Prompts updated:', prompts)
      },
      resources: {
        onChanged: (error, resources) => console.log('Resources updated:', resources)
      }
    }
  }
);
```

## JSON Schema Validation

```typescript
// Tool output validation
export interface JsonSchemaValidator<T> {
  validate(data: unknown, schema: JsonSchemaType): JsonSchemaValidatorResult<T>;
}

export interface JsonSchemaValidatorResult<T> {
  success: boolean;
  data?: T;
  errors?: ValidationError[];
}

// Usage with client
const client = new Client(
  { name: 'my-client', version: '1.0.0' },
  {
    jsonSchemaValidator: new AjvJsonSchemaValidator() // Default for Node.js
  }
);

const result = await client.callTool({ name: 'my-tool', arguments: {} });
// Tool output is automatically validated against tool's outputSchema
```

## Example: Complete Client

```typescript
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

const transport = new StdioClientTransport({
  command: 'npx',
  args: ['-y', '@modelcontextprotocol/server-everything'],
});

const client = new Client(
  { name: 'my-client', version: '1.0.0' },
  {
    capabilities: {
      roots: { listChanged: true },
      sampling: {}
    },
    listChanged: {
      tools: {
        onChanged: async (error, tools) => {
          if (!error) {
            console.log('Tools updated:', tools);
          }
        }
      }
    }
  }
);

await client.connect(transport);

// List and call tools
const tools = await client.listTools();
for (const tool of tools.tools) {
  console.log(`Tool: ${tool.name} - ${tool.description}`);
}

const result = await client.callTool({
  name: 'example-tool',
  arguments: { key: 'value' }
});

console.log('Result:', result);
```

## Example: Complete Server

```typescript
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

const server = new Server(
  { name: 'example-server', version: '1.0.0' },
  {
    capabilities: {
      tools: {},
      resources: { subscribe: true },
      prompts: {}
    }
  }
);

// Register tool
server.tool(
  'greet',
  'Greet someone by name',
  {
    type: 'object',
    properties: {
      name: { type: 'string', description: 'Name to greet' }
    },
    required: ['name']
  },
  async (params) => {
    return {
      content: [{ type: 'text', text: `Hello, ${params.name}!` }]
    };
  }
);

// Register resource
server.resource(
  'config',
  'file:///config.json',
  async (uri) => {
    return {
      contents: [{
        uri: uri.href,
        mimeType: 'application/json',
        text: JSON.stringify({ key: 'value' })
      }]
    };
  }
);

// Register prompt
server.prompt(
  'review-code',
  'Code review prompt',
  {
    language: { type: 'string', description: 'Programming language' }
  },
  (args) => ({
    messages: [{
      role: 'user',
      content: {
        type: 'text',
        text: `Review my ${args.language} code for best practices.`
      }
    }]
  })
);

const transport = new StdioServerTransport();
await server.connect(transport);
```

## Key Insights

1. **Protocol-First Design**: The `Protocol` base class handles all JSON-RPC communication details

2. **Type Safety**: Full TypeScript types generated from the MCP schema

3. **Transport Agnostic**: Same client/server code works with any transport implementation

4. **Capability Checking**: Optional strict capability enforcement prevents unsupported requests

5. **Built-in Validation**: JSON Schema validation for tool outputs

6. **OAuth 2.0 Support**: Complete OAuth flow implementation for authentication

7. **Task Support**: Experimental task-based execution for long-running operations

## Related Resources

- [NPM Package](https://www.npmjs.com/package/@modelcontextprotocol/sdk)
- [API Documentation](https://modelcontextprotocol.io/typescript-sdk)
- [GitHub Repository](https://github.com/modelcontextprotocol/typescript-sdk)
- [Examples](./examples/)
