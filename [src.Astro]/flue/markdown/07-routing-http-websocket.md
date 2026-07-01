---
title: Routing — HTTP API, Durable Streams, error envelopes
---

# HTTP Routing: The `flue()` Sub-App

Flue's HTTP API is exposed through a mountable Hono sub-app created by the `flue()` function. The sub-app registers routes for agent invocation (`/agents/:name/:id`), workflow invocation (`/workflows/:name`), agent abort (`/agents/:name/:id/abort`), agent attachment downloads (`/agents/:name/:id/attachments/:attachmentId`), custom channels (`/channels/:name`, `/channels/:name/:suffix{.+}`), and run stream reads (`/runs/:runId`). The routing layer is transport-agnostic -- the same middleware chain handles both agents and workflows, with per-target routing to Node handlers or Cloudflare Durable Objects. Agent and workflow GET/HEAD routes serve Durable Streams (DS) event reads, not WebSocket connections.

This document covers the `flue()` sub-app architecture, the complete route table with HTTP method semantics, Durable Streams event format, error envelope format, and request validation.

For workflow handler semantics and the run lifecycle, see [06-workflows](06-workflows.html). For the SDK client that calls these routes, see [10-sdk-client](10-sdk-client.html). For the error framework implementation, see [15-cross-cutting](15-cross-cutting.html).

---

## The `flue()` Sub-App

The `flue()` function creates and configures a Hono application with all public routes. It is designed to be mounted at any path prefix by the host application:

```ts
// Source: packages/runtime/src/runtime/flue-app.ts:178-218
export function flue(): Hono {
  const app = new Hono();

  app.post(
    '/workflows/:name',
    validated('param', WorkflowRouteParamSchema),
    validated('query', InvocationQuerySchema),
    workflowRouteHandler,
  );
  app.all('/workflows/:name', workflowRouteHandler);

  app.post(
    '/agents/:name/:id',
    validated('param', AgentRouteParamSchema),
    validated('query', InvocationQuerySchema),
    agentRouteHandler,
  );
  // Abort all in-flight/queued work for an agent instance.
  app.all('/agents/:name/:id/abort', abortRouteHandler);
  // Opt-in attachment byte download.
  app.all('/agents/:name/:id/attachments/:attachmentId', attachmentsRouteHandler);
  // Non-POSTs still reach the canonical Flue 405 envelope.
  app.all('/agents/:name/:id', agentRouteHandler);
  app.all('/channels/:name', channelRouteHandler);
  app.all('/channels/:name/:suffix{.+}', channelRouteHandler);
  // DS stream endpoints for run events.
  app.all('/runs/:runId', runStreamReadHandler);

  app.onError((err) => toHttpResponse(err));

  return app;
}
```

The route table has **no** `/openapi.json` endpoint. Run stream reads are served by a single `app.all('/runs/:runId', runStreamReadHandler)` -- there is no `RUN_ROUTES_BY_ID` constant and no separate `/events` or `/stream` sub-routes.

When no user-authored `app.ts` exists, `createDefaultFlueApp()` mounts `flue()` at `/` and adds a `notFound` handler that throws `RouteNotFoundError` with the request method and path:

```ts
// Source: packages/runtime/src/runtime/flue-app.ts:222-233
export function createDefaultFlueApp(): Hono {
  const app = new Hono();
  app.route('/', flue());
  app.notFound((c) => {
    throw new RouteNotFoundError({
      method: c.req.method,
      path: new URL(c.req.url).pathname,
    });
  });
  app.onError((err) => toHttpResponse(err));
  return app;
}
```

When a user writes an `app.ts`, they own the composition and must mount `flue()` explicitly. The `Fetchable` interface in `routing.ts` defines the structural contract:

```ts
// Source: packages/runtime/src/routing.ts (re-export from flue-app.ts)
export { flue } from './runtime/flue-app.ts';

export interface Fetchable {
  fetch(request: Request, env?: unknown, ctx?: unknown): Response | Promise<Response>;
}
```

Source: [`packages/runtime/src/runtime/flue-app.ts:270-318`](https://github.com/earendil-works/flue/blob/main/packages/runtime/src/runtime/flue-app.ts#L270-L318)

---

## Route Table

The complete route table with HTTP methods, handlers, and response shapes:

| Route | Methods | Handler | Response |
|-------|---------|---------|----------|
| `POST /agents/:name/:id` | POST | `agentRouteHandler` | `{ result }` JSON or DS stream |
| `GET/HEAD /agents/:name/:id` | GET, HEAD | `agentRouteHandler` | DS conversation stream read |
| `ALL /agents/:name/:id` | ALL (catch-all) | `agentRouteHandler` | 405 for non-POST/GET/HEAD, 404 for unmatched |
| `ALL /agents/:name/:id/abort` | ALL | `abortRouteHandler` | `{ aborted: boolean }` (POST only, 405 otherwise) |
| `ALL /agents/:name/:id/attachments/:attachmentId` | ALL | `attachmentsRouteHandler` | Attachment bytes (GET only, requires opt-in `attachments` middleware) |
| `POST /workflows/:name` | POST | `workflowRouteHandler` | `202 {status, runId}` or `200 {result}` or DS stream |
| `ALL /workflows/:name` | ALL (catch-all) | `workflowRouteHandler` | 405 for non-POST, 404 for unmatched |
| `ALL /channels/:name` | ALL | `channelRouteHandler` | Custom channel handler response |
| `ALL /channels/:name/:suffix{.+}` | ALL | `channelRouteHandler` | Custom channel sub-route response |
| `ALL /runs/:runId` | ALL | `runStreamReadHandler` | DS stream read (GET/HEAD only, 405 otherwise) or `RunRecord` JSON (`?meta`) |

**Note:** There is no `/openapi.json` endpoint. There are no `/runs/:runId/events` or `/runs/:runId/stream` sub-routes. There are no WebSocket upgrade routes. All "streaming" uses the Durable Streams (DS) protocol (catch-up, long-poll, SSE).

### Agent Routes: `/agents/:name/:id`

Agent routes require both a `name` (the discovered agent module name) and an `id` (an arbitrary instance identifier chosen by the caller). The `validateAgentRequest()` function in `errors.ts` enforces:

- Method must be `POST`, `GET`, or `HEAD`
- `name` must be non-empty and not whitespace-only
- `id` must be non-empty and not whitespace-only
- `name` must be registered in the manifest

```ts
// Source: packages/runtime/src/errors.ts (validateAgentRequest)
export function validateAgentRequest(opts: ValidateAgentRequestOptions): void {
  if (opts.method !== 'POST' && opts.method !== 'GET' && opts.method !== 'HEAD') {
    throw new MethodNotAllowedError({ method: opts.method, allowed: ['GET', 'HEAD', 'POST'] });
  }
  if (opts.name.trim() === '' || opts.id.trim() === '') {
    throw new InvalidRequestError({
      reason: 'Agent URLs must have the shape /agents/<name>/<id> with non-empty segments.',
    });
  }
  if (!opts.registeredAgents.includes(opts.name)) {
    throw new AgentNotFoundError({ name: opts.name, available: opts.registeredAgents });
  }
}
```

The `agentRouteHandler` in `flue-app.ts` dispatches based on HTTP method:

- **GET/HEAD**: Routes to Durable Streams conversation read (`handleAgentConversationRead` / `handleAgentConversationHead` on Node, or `routeAgentRequest` on Cloudflare).
- **POST**: Routes to `handleAgentRequest` on Node (direct agent prompt) or `routeAgentRequest` on Cloudflare (DO forwarding).

Agent requests with `X-Webhook: true` are rejected -- asynchronous delivery uses `dispatch()` instead.

The POST handler (`handleAgentRequest` in `handle-agent.ts`) parses the JSON body, rejects `X-Webhook: true` requests, and serves the request. The request can be served in two modes:

- **Synchronous** (default): Returns `{ result }` JSON after the model operation completes
- **DS Stream** (`?wait=result` query param): Returns a Durable Streams read of the conversation

The agent request body format is:

```json
{
  "message": "string (required)",
  "images": [  // optional array of image content blocks
    { "type": "image", "data": "base64...", "mimeType": "image/png", "filename": "optional" }
  ]
}
```

### Workflow Routes: `/workflows/:name`

Workflow routes require only a `name`. The `validateWorkflowRequest()` function enforces:

- Method must be `POST`
- `name` must be non-empty
- `name` must be registered in the manifest
- `name` must be explicitly exposed for HTTP transport (workflows can opt out of HTTP exposure)

```ts
// Source: packages/runtime/src/errors.ts (validateWorkflowRequest)
export function validateWorkflowRequest(opts: ValidateWorkflowRequestOptions): void {
  if (opts.method !== 'POST') {
    throw new MethodNotAllowedError({ method: opts.method, allowed: ['POST'] });
  }
  if (opts.name.trim() === '') {
    throw new InvalidRequestError({
      reason: 'Workflow URLs must have the shape /workflows/<name> with a non-empty segment.',
    });
  }
  if (!opts.registeredWorkflows.includes(opts.name)) {
    throw new WorkflowNotFoundError({ name: opts.name, available: opts.registeredWorkflows });
  }
  if (!opts.httpWorkflows.includes(opts.name)) {
    throw new WorkflowNotFoundError({
      name: opts.name,
      available: opts.registeredWorkflows,
      notHttp: true,
    });
  }
}
```

**Note:** `WorkflowNotHttpError` does NOT exist as a separate class. When a workflow is registered but not exposed over HTTP, `validateWorkflowRequest` throws `WorkflowNotFoundError` with `{ notHttp: true }`. The error class builds a different `dev` message for this case (explaining how to expose the workflow) while keeping the public `message`/`details` identical to the unknown-workflow case -- preventing public callers from enumerating internal-only workflow names.

The workflow request body is a free-form JSON object — the shape is defined by the workflow handler. Empty bodies are treated as `{}` by `parseJsonBody()`.

Three invocation modes are selected by request headers/parameters:

| Mode | How to trigger | Response |
|------|---------------|----------|
| **Accepted** | Plain `POST` | `202 {status: "accepted", runId}` |
| **Synchronous** | `?wait=result` query param | `200 {result, _meta: {runId}}` |
| **DS Stream** | GET/HEAD on `/runs/:runId` | DS stream with `run_start`, events, `run_end` |

All three modes set the `X-Flue-Run-Id` response header, even on error responses.

### Run Stream Route: `/runs/:runId`

There is a single run route (`/runs/:runId`) served by `runStreamReadHandler`. It does NOT use a `RUN_ROUTES_BY_ID` constant or `runByIdRouteHandler`. The handler supports:

- **GET**: Returns a Durable Streams event read (via `handleStreamRead` on Node, `routeRunRequest` on Cloudflare). If `?meta` is present, returns `RunRecord` JSON instead.
- **HEAD**: Returns stream metadata only (via `handleStreamHead` on Node).
- **Other methods**: Throws `MethodNotAllowedError` with `allowed: ['GET', 'HEAD']`.

```ts
// Source: packages/runtime/src/runtime/flue-app.ts:348-390 (runStreamReadHandler)
const runStreamReadHandler: MiddlewareHandler = async (c) => {
  const rt = requiredRuntime();
  const method = c.req.method;
  const runId = c.req.param('runId') ?? '';
  const pointer = await findRunPointer(rt, c.env, runId);
  const workflow = pointer
    ? rt.workflows.find((record) => record.name === pointer.workflowName)
    : undefined;

  if (!workflow || (!workflow.runs && !rt.temporaryLocalExposure)) {
    throw new RunNotFoundError({ runId });
  }

  return runAttachedMiddleware(c, workflow.runs, async () => {
    if (method !== 'GET' && method !== 'HEAD') {
      throw new MethodNotAllowedError({ method, allowed: ['GET', 'HEAD'] });
    }

    const wantsMeta = method === 'GET' && new URL(c.req.url).searchParams.has('meta');

    if (rt.target === 'node') {
      if (wantsMeta) {
        return handleRunRouteRequest({
          runStore: rt.runStore,
          workflowName: workflow.name,
          runId,
        });
      }
      return nodeStreamReadResponse(rt, method, runStreamPath(runId), c.req.raw);
    }

    const response = await rt.routeRunRequest(c.req.raw, c.env, {
      workflowName: workflow.name,
      runId,
    });
    if (response) return response;
    throw new RunNotFoundError({ runId });
  });
};
```

**`GET /runs/:runId?meta`** — Returns a `RunRecord` JSON object (via `handleRunRouteRequest`):

```json
{
  "runId": "workflow:my-workflow:01JXYZ...",
  "status": "active" | "completed" | "errored",
  "startedAt": "2025-01-15T10:30:00.000Z",
  "endedAt": "...",
  "isError": false,
  "durationMs": 5230,
  "result": { ... },
  "error": { ... }
}
```

**`GET /runs/:runId`** — Returns a Durable Streams event read (catch-up + live tail). Uses `Last-Event-ID` for resume.

**`HEAD /runs/:runId`** — Returns stream metadata only (headers indicate stream state without body).

**Note:** There are NO `/runs/:runId/events` or `/runs/:runId/stream` sub-routes. The document previously claimed three separate run routes with a `RUN_ROUTES_BY_ID` constant -- neither exists in the source.

---

## Durable Streams (DS) Protocol

Flue uses the **Durable Streams (DS) protocol** for event reads -- not WebSocket. Both agent and workflow stream reads use the same DS protocol (catch-up, long-poll, SSE). The protocol is implemented in `event-stream-store.ts` and `handle-stream-routes.ts`.

### Stream Read Flow

1. **Catch-up**: Historical events are replayed from the stream store starting at `fromIndex` (derived from `Last-Event-ID` header or `?after=` query param).
2. **Long-poll**: If the run is still active, the handler subscribes to new events and waits.
3. **SSE**: Events are emitted as Server-Sent Events with `event: <type>` and `id: <eventIndex>`.
4. **Terminal**: When `run_end` (or equivalent terminal event) is emitted, the stream completes.

### Agent Stream Paths

Agent streams use `agentStreamPath(name, id)` to derive the stream path. GET/HEAD on `/agents/:name/:id` routes to:

- **Node**: `handleAgentConversationRead` / `handleAgentConversationHead` from `handle-conversation-routes.ts`
- **Cloudflare**: `routeAgentRequest` forwarding to the agent Durable Object

### Workflow Run Streams

Workflow run streams use `runStreamPath(runId)`. GET/HEAD on `/runs/:runId` routes to:

- **Node**: `handleStreamRead` / `handleStreamHead` from `handle-stream-routes.ts`
- **Cloudflare**: `routeRunRequest` forwarding to the registry Durable Object

### SSE Frame Format

DS streams emit Server-Sent Events:

```
event: run_start
id: 0
data: {"type":"run_start","runId":"workflow:my-wf:...","eventIndex":0,"timestamp":"...","payload":{}}

event: log
id: 1
data: {"type":"log","level":"info","message":"Processing...","eventIndex":1,"timestamp":"..."}

event: run_end
id: 2
data: {"type":"run_end","runId":"workflow:my-wf:...","result":null,"isError":false,"durationMs":1234,"eventIndex":2,"timestamp":"..."}
```

The `id` field is the `eventIndex` for SSE resume via `Last-Event-ID` header.

**Note:** There is NO WebSocket protocol v1 in the current source. The file `websocket-protocol.ts` does not exist. The types `AgentWebSocketClientMessage`, `AgentWebSocketServerMessage`, `WorkflowWebSocketClientMessage`, and `WorkflowWebSocketServerMessage` are not exported from `types.ts`. Agent and workflow streaming is handled entirely through the Durable Streams protocol (HTTP GET/HEAD with SSE).

Terminal errors in the stream infrastructure use:

```
event: error
id: 2
data: {"error":{"type":"validation_failed","message":"...","details":"..."}}
```

---

## Error Envelopes

Flue's error framework has two audiences and produces a canonical wire envelope. The framework is defined in `errors.ts`:

Source: [`packages/runtime/src/errors.ts:408-432`](https://github.com/earendil-works/flue/blob/main/packages/runtime/src/errors.ts#L408-L432)

```
// Wire envelope (HTTP body + SSE data: payload for error events):
{
  "error": {
    "type":    "...",    // Stable machine-readable identifier (snake_case)
    "message": "...",    // One-sentence caller-safe summary
    "details": "...",    // Caller-safe longer explanation
    "dev":     "...",    // Present ONLY in local/dev mode AND when non-empty
    "meta":    {...}     // Structured error-specific metadata (rare)
  }
}
```

### Two-Audience Design

The error framework separates information by audience:

| Field | Who sees it | When | Content rules |
|-------|------------|------|---------------|
| `type` | Always | Always | Machine-readable snake_case identifier |
| `message` | Always | Always | One-sentence caller-safe summary |
| `details` | Always | Always | Caller-safe explanation. NEVER includes: sibling enumeration, filesystem paths, framework internals, source-code fix instructions |
| `dev` | Developer only | Only in `flue dev` / `flue run` mode AND when non-empty | Everything else: available alternatives, filesystem layout, framework guidance, source-code fix instructions |
| `meta` | Always | When set by subclass | Structured error-specific metadata (rare) |

This separation allows `dev` to be richly helpful without leaking namespace state to public callers. When in doubt, information goes in `dev`.

### Error Classes

The framework defines these concrete error classes (all in `errors.ts`):

| Class | HTTP Status | `type` | When thrown |
|-------|------------|--------|-------------|
| `RuntimeUnavailableError` | 503 | `runtime_unavailable` | Local runtime temporarily unavailable (loading/draining/failed) |
| `MethodNotAllowedError` | 405 | `method_not_allowed` | Wrong HTTP method on endpoint. Sets `Allow` header. |
| `UnsupportedMediaTypeError` | 415 | `unsupported_media_type` | Body present without `application/json` Content-Type |
| `InvalidJsonError` | 400 | `invalid_json` | Body is present but not valid JSON |
| `AgentNotFoundError` | 404 | `agent_not_found` | Agent name not in registered agents |
| `WorkflowNotFoundError` | 404 | `workflow_not_found` | Workflow name not in registered workflows, OR not exposed over HTTP (with `notHttp: true`) |
| `RouteNotFoundError` | 404 | `route_not_found` | No route matches the request |
| `RunNotFoundError` | 404 | `run_not_found` | Run ID not found |
| `StreamNotFoundError` | 404 | `stream_not_found` | Event stream path not found |
| `AttachmentsNotExposedError` | 404 | `route_not_found` | Attachment endpoint exists but agent didn't export `attachments` middleware |
| `AttachmentNotFoundError` | 404 | `attachment_not_found` | Attachment ID not found |
| `RunStoreUnavailableError` | 501 | `run_store_unavailable` | Run history endpoint without configured run store |
| `InvalidRequestError` | 400 | `invalid_request` | Malformed request (URL shape, segment validation) |
| `SessionNotFoundError` | N/A (internal) | `session_not_found` | Session does not exist in harness |
| `SessionAlreadyExistsError` | N/A (internal) | `session_already_exists` | Session name already in use |
| `SessionBusyError` | N/A (internal) | `session_busy` | Session has an active operation |

**Note:** `WorkflowNotHttpError` does NOT exist. Non-HTTP workflows get `WorkflowNotFoundError` with `notHttp: true`. `RunRegistryUnavailableError` does NOT exist. `RunEventTooLargeError` is not in the source. `ValidationError` as a named class does NOT exist (validation issues use `InvalidRequestError` with `describeValidationIssues`). The error framework also defines many more internal error classes (persistence, sandbox, session, tool, workflow, submission errors) beyond the HTTP-facing ones listed above.

### The `toHttpResponse()` Function

All thrown errors are caught by `app.onError((err) => toHttpResponse(err))`. The function renders any thrown value into the canonical Flue error envelope:

```ts
// Source: packages/runtime/src/errors.ts (toHttpResponse)
export function toHttpResponse(err: unknown): Response {
  // Browser security headers (DS protocol) — set on every error response
  const baseHeaders: Record<string, string> = {
    'content-type': 'application/json',
    'x-content-type-options': 'nosniff',
    'cross-origin-resource-policy': 'cross-origin',
  };
  if (isFlueError(err)) {
    const isHttp = err instanceof FlueHttpError;
    const status = isHttp ? err.status : 500;
    const headers = { ...baseHeaders };
    if (isHttp && err.headers) {
      Object.assign(headers, err.headers);
    }
    // Log non-HTTP FlueErrors that bubbled up to the HTTP layer
    if (!isHttp) {
      flueLog.error(err);
    }
    return new Response(JSON.stringify(envelope(err)), { status, headers });
  }
  // Non-FlueError: log everything, leak nothing.
  flueLog.error(err);
  return new Response(JSON.stringify(GENERIC_INTERNAL), {
    status: 500,
    headers: baseHeaders,
  });
}
```

**Note:** The actual source sets browser security headers (`x-content-type-options: nosniff`, `cross-origin-resource-policy: cross-origin`) on EVERY error response, including the 500 fallback. Non-HTTP FlueErrors (like `SessionBusyError`) are logged even though their message is rendered, since they weren't constructed with HTTP semantics in mind.

Non-Flue errors are logged in full and rendered as a generic 500 with no message leaked — `GENERIC_INTERNAL` is `{error: {type: "internal_error", message: "An internal error occurred.", details: "The server encountered an unexpected error..."}}`.

### Example Error Responses

**404 — Agent not found (production):**

```json
{
  "error": {
    "type": "agent_not_found",
    "message": "Agent \"unknown\" is not registered.",
    "details": "Verify the agent name is correct."
  }
}
```

**404 — Agent not found (dev mode):**

```json
{
  "error": {
    "type": "agent_not_found",
    "message": "Agent \"unknown\" is not registered.",
    "details": "Verify the agent name is correct.",
    "dev": "Available agents: \"echo\", \"researcher\".\nAgents are loaded from the project root's \"agents/\" directory at build time. Verify the agent file is present in the project root being served."
  }
}
```

**415 — Unsupported media type:**

```json
{
  "error": {
    "type": "unsupported_media_type",
    "message": "Request body must be sent as application/json.",
    "details": "Received Content-Type: \"text/plain\".\nSend the request body as JSON with the header \"Content-Type: application/json\", or omit the body entirely (and the Content-Type header) if the request doesn't have a payload."
  }
}
```

---

## Request Validation

Route validation uses the `validated()` helper from `flue-app.ts`, which wraps `hono-openapi`'s `validator`:

```ts
function validated(
  target: 'param' | 'query',
  schema: Parameters<typeof validator>[1],
): MiddlewareHandler {
  return validator(target, schema, (result) => {
    if (result.success) return;
    throw new InvalidRequestError({
      reason: `Invalid ${target} parameters: ${describeValidationIssues(result.error)}`,
    });
  }) as MiddlewareHandler;
}
```

The schemas (from `schemas.ts`):

```ts
// packages/runtime/src/runtime/schemas.ts
export const DirectAgentPayloadSchema = v.object({
  message: v.string(),
  images: v.optional(v.array(DirectAgentImageSchema)),
});

export const WorkflowRouteParamSchema = v.object({ name: v.string() });

/** Shared `?wait` query contract for agent and workflow invocation routes. */
export const InvocationQuerySchema = v.object({
  wait: v.optional(v.literal('result')),
});

export const AgentRouteParamSchema = v.object({ name: v.string(), id: v.string() });
```

**Note:** `InvocationQuerySchema` is shared between agent and workflow routes (both use `?wait=result`). There is no `WorkflowInvocationQuerySchema`.

---

## Routing Architecture: How Agents and Workflows Share the Middleware Chain

The `flue-app.ts` routes both agents and workflows through the same structural pattern:

```ts
// Agent route (simplified)
app.post('/agents/:name/:id',
  validated('param', AgentRouteParamSchema),
  validated('query', InvocationQuerySchema),
  agentRouteHandler,  // validates -> runAttachedMiddleware -> dispatches
);

// Workflow route (simplified)
app.post('/workflows/:name',
  validated('param', WorkflowRouteParamSchema),
  validated('query', InvocationQuerySchema),
  workflowRouteHandler,  // validates -> runAttachedMiddleware -> dispatches
);
```

Both handlers follow the same three-phase pattern:

1. **Validate** the request (method, name/id, registration, transport opt-in)
2. **Run attached middleware** (`runAttachedMiddleware(c, record?.route, ...)`)
3. **Route to target** -- Node handler function or Cloudflare Durable Object forwarding

The `runAttachedMiddleware()` function wraps per-agent or per-workflow middleware:

```ts
// Source: packages/runtime/src/runtime/flue-app.ts:412-430
async function runAttachedMiddleware(
  c: Parameters<MiddlewareHandler>[0],
  middleware: MiddlewareHandler | undefined,
  handle: () => Promise<Response | undefined>,
): Promise<Response | undefined> {
  if (!middleware) return handle();
  const finalizedBefore = c.finalized;
  const responseBefore = finalizedBefore ? c.res : undefined;
  let continued = false;
  const response = await middleware(c, async () => {
    if (continued) throw new Error('next() called multiple times');
    continued = true;
    const handled = await handle();
    if (handled) c.res = handled;
  });
  if (response) return response;
  if (continued || (c.finalized && (!finalizedBefore || c.res !== responseBefore))) return c.res;
  throw new Error(
    'Context is not finalized. Did you forget to return a Response object or await next()?',
  );
}
```

This allows users to attach custom middleware (e.g., authentication, rate limiting, logging) to specific agents or workflows via the `route` field on `AgentRecord` and `WorkflowRecord`. The middleware must either return a `Response` or call `await next()` to continue to the handler.

---

## Transport Registration

Agents and workflows opt into HTTP transport at build time through their route middleware presence. The `AgentRecord` and `WorkflowRecord` interfaces track this:

```ts
// Source: packages/runtime/src/runtime/flue-app.ts:41-57
export interface AgentRecord {
  name: string;
  definition: AgentDefinition;
  description?: string;
  route?: MiddlewareHandler;
  attachments?: MiddlewareHandler;  // opt-in gate for attachment downloads
}

export interface WorkflowRecord {
  name: string;
  definition: WorkflowDefinition;
  route?: WorkflowRouteHandler;
  runs?: WorkflowRunsHandler;
}
```

The `registeredAgentsForTransport()` and `registeredWorkflowsForTransport()` functions filter by route presence (not by a `transports` object):

```ts
// Source: packages/runtime/src/runtime/flue-app.ts:432-440
function registeredAgentsForTransport(rt: FlueRuntime): readonly string[] {
  return rt.agents
    .filter((agent) => rt.temporaryLocalExposure || agent.route !== undefined)
    .map((agent) => agent.name);
}

function registeredWorkflowsForTransport(rt: FlueRuntime): readonly string[] {
  return rt.workflows
    .filter((workflow) => rt.temporaryLocalExposure || workflow.route !== undefined)
    .map((workflow) => workflow.name);
}
```

**Note:** There is no `FlueManifest` interface with a `transports: { http?: true; websocket?: true }` shape. Transport opt-in is determined by whether the `route` field is set on the record. The `temporaryLocalExposure` flag (dev mode) exposes all agents and workflows regardless of route presence. The `AgentManifestEntry` type (exported for `listAgents()`) has `transports: { http?: true }` -- no WebSocket field.

A workflow that doesn't set a `route` returns a `WorkflowNotFoundError` (with `notHttp: true`) even though the workflow exists. This allows workflows that are only accessible via internal dispatch.

---

## Aha Moments

**Aha: Workflows and agents share the same route handler pattern.** The `flue()` function registers `/agents/:name/:id` and `/workflows/:name` routes that follow an identical structural pattern: validate the request against the manifest, run per-resource middleware, then dispatch to either a Node handler function or Cloudflare Durable Object forwarding. The middleware chain (`runAttachedMiddleware`) is shared code -- agent and workflow routes only differ in their validation schemas and the handler they dispatch to. This is why per-agent and per-workflow middleware share the same `MiddlewareHandler` type and the same guard against multiple `next()` calls.

**Aha: The error framework has two audiences, enforced by required fields.** Every `FlueHttpError` constructor takes both `details` (caller-safe) and `dev` (developer-only) as required strings. This means it's impossible to accidentally put internal guidance in `details` -- the TypeScript compiler forces you to think about both audiences. The `dev` field is only rendered when `configureErrorRendering({ devMode: true })` is called, which the generated entry does for `flue dev` and `flue run` but not for production deployments. Even in dev mode, `dev` is omitted when the error class set it to `''` -- so its presence means "this error has dev-specific guidance" not "we're in dev mode."

**Aha: Empty POST bodies are legal and treated as `undefined`.** The `parseJsonBody()` function returns `undefined` when the request has `Content-Length: 0` or when there's no `Content-Length` AND no `Content-Type` header. This means `curl -X POST http://localhost:3000/workflows/my-workflow` works without `-d '{}'`. The code documents the trade-off explicitly: if a client sends a body but no Content-Length and no Content-Type, the body is silently ignored. This preserves the no-payload UX for invocations that don't require input.

**Aha: `toHttpResponse` sets browser security headers on ALL responses.** Every error response -- including the generic 500 fallback for non-Flue errors -- carries `x-content-type-options: nosniff` and `cross-origin-resource-policy: cross-origin`. This prevents browsers from MIME-snatching error responses.

**Aha: Non-HTTP FlueErrors are logged even when rendered.** When a non-HTTP error (like `SessionBusyError`) bubbles up to the HTTP error handler, `toHttpResponse` logs it via `flueLog.error()` before rendering its typed envelope with status 500. This is deliberate: these errors weren't constructed with HTTP semantics, so it's worth surfacing them in logs even though their message is rendered on the wire.

---

## Related Documents

- [06-workflows](06-workflows.html) — Workflow handlers, run lifecycle, `dispatch()`, `dispatchId` vs `runId`
- [15-cross-cutting](15-cross-cutting.html) — Error handling deep dive, provider config, usage tracking
- [10-sdk-client](10-sdk-client.html) — `FlueClient` HTTP and WebSocket usage, admin routes
- [08-cloudflare-target](08-cloudflare-target.html) — Durable Objects, registry, cross-deployment routing

[Table of Contents](README.html)
