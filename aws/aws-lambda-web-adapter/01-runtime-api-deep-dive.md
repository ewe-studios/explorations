---
title: "Lambda Runtime API Deep Dive"
subtitle: "Complete reference to AWS Lambda Runtime API endpoints, headers, and patterns"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-web-adapter/01-runtime-api-deep-dive.md
related: /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-web-adapter/exploration.md
---

# Lambda Runtime API Deep Dive

## Introduction

The AWS Lambda Runtime API is an HTTP-based interface that allows custom runtimes to communicate with the Lambda service. This document provides a comprehensive reference to all endpoints, headers, request/response formats, and usage patterns.

### API Location

The Runtime API is always available at:
```
http://localhost:9001
```

The address is also available in the environment variable:
```bash
AWS_LAMBDA_RUNTIME_API=localhost:9001
```

### API Version

This document covers Runtime API version **2018-06-01** (current stable version).

---

## Part 1: Runtime Endpoints

### 1.1 Next Invocation (GET)

**Endpoint:**
```
GET /2018-06-01/runtime/invocation/next
```

**Purpose:**
Long-polls for the next invocation event. Blocks until an event is available or the function times out.

**Request:**
```http
GET /2018-06-01/runtime/invocation/next
```

**Response (200 OK):**
```http
HTTP/1.1 200 OK
Content-Type: application/json
Content-Length: <length>
Lambda-Runtime-Aws-Request-Id: 12345678-1234-1234-1234-123456789012
Lambda-Runtime-Deadline-Ms: 1711555200000
Lambda-Runtime-Invoked-Function-Arn: arn:aws:lambda:us-east-1:123456789012:function:my-function
Lambda-Runtime-Trace-Id: Root=1-abcdef12-34567890abcdef1234567890
Lambda-Runtime-Client-Context: <base64-encoded-json>  [optional]
Lambda-Runtime-Cognito-Identity: <json>  [optional]

<event-payload>
```

**Response Headers:**

| Header | Type | Description |
|--------|------|-------------|
| `Lambda-Runtime-Aws-Request-Id` | Required | Unique identifier for this invocation |
| `Lambda-Runtime-Deadline-Ms` | Required | Unix timestamp in milliseconds when function will timeout |
| `Lambda-Runtime-Invoked-Function-Arn` | Required | Full ARN of the invoked function including version/alias |
| `Lambda-Runtime-Trace-Id` | Optional | AWS X-Ray trace ID for distributed tracing |
| `Lambda-Runtime-Client-Context` | Optional | Mobile app context (base64-encoded JSON) |
| `Lambda-Runtime-Cognito-Identity` | Optional | Cognito identity information (JSON) |

**Response Body:**
The event payload. Format depends on the trigger source:
- API Gateway: HTTP request structure
- SQS: Message batch structure
- S3: Object event structure
- Custom: Any JSON structure

**Error Responses:**

| Status | Code | Description |
|--------|------|-------------|
| 400 | Bad Request | Invalid request format |
| 403 | Forbidden | Runtime not initialized |
| 404 | Not Found | Runtime not registered |
| 500 | Internal Error | Lambda service error |
| 502 | Bad Gateway | Runtime API error |

**Example (API Gateway v2 event):**
```json
{
  "version": "2.0",
  "routeKey": "GET /users",
  "rawPath": "/users",
  "rawQueryString": "page=1&limit=10",
  "headers": {
    "accept": "application/json",
    "host": "abc123.execute-api.us-east-1.amazonaws.com",
    "user-agent": "Mozilla/5.0"
  },
  "requestContext": {
    "accountId": "123456789012",
    "apiId": "abc123",
    "domainName": "abc123.execute-api.us-east-1.amazonaws.com",
    "http": {
      "method": "GET",
      "path": "/users",
      "protocol": "HTTP/1.1",
      "sourceIp": "192.168.1.1"
    },
    "requestId": "abc123-def456",
    "stage": "$default",
    "time": "27/Mar/2026:12:00:00 +0000",
    "timeEpoch": 1711555200000
  },
  "body": "",
  "isBase64Encoded": false
}
```

---

### 1.2 Post Response (POST)

**Endpoint:**
```
POST /2018-06-01/runtime/invocation/{request-id}/response
```

**Purpose:**
Submit the function's response to Lambda.

**Request:**
```http
POST /2018-06-01/runtime/invocation/{request-id}/response
Content-Type: application/json

<response-payload>
```

**Path Parameters:**
| Parameter | Description |
|-----------|-------------|
| `request-id` | The request ID from the invocation headers |

**Request Body:**
The response payload. Can be:
- Any JSON-serializable value
- A string (for text responses)
- For buffered mode: Complete response
- For streaming mode: Chunks of response data

**Response (202 Accepted):**
```http
HTTP/1.1 202 Accepted
Content-Length: 0
```

**Example (simple response):**
```bash
curl -X POST \
  http://localhost:9001/2018-06-01/runtime/invocation/abc123/response \
  -H "Content-Type: application/json" \
  -d '{"statusCode": 200, "body": "Hello, World!"}'
```

**Example (API Gateway response):**
```json
{
  "statusCode": 200,
  "headers": {
    "Content-Type": "application/json",
    "Access-Control-Allow-Origin": "*"
  },
  "multiValueHeaders": {
    "Set-Cookie": ["session=abc123", "user=xyz789"]
  },
  "body": "{\"users\": [\"Alice\", \"Bob\"]}",
  "isBase64Encoded": false
}
```

---

### 1.3 Post Error (POST)

**Endpoint:**
```
POST /2018-06-01/runtime/invocation/{request-id}/error
```

**Purpose:**
Report an error for the invocation.

**Request:**
```http
POST /2018-06-01/runtime/invocation/{request-id}/error
Content-Type: application/json

{
  "errorType": "Error",
  "errorMessage": "Something went wrong",
  "stackTrace": ["at handler.js:10", "at runtime.js:50"],
  "type": "Error"
}
```

**Request Body Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `errorType` | String | Yes | Type of error (e.g., "Error", "TypeError") |
| `errorMessage` | String | Yes | Human-readable error message |
| `stackTrace` | Array | No | Stack trace as array of strings |
| `type` | String | No | Alias for errorType |

**Example:**
```json
{
  "errorType": "ValidationError",
  "errorMessage": "Invalid input: missing required field 'name'",
  "stackTrace": [
    "at validateInput (handler.js:25:10)",
    "at handler (handler.js:10:20)"
  ]
}
```

---

## Part 2: Extension Endpoints

### 2.1 Register Extension (POST)

**Endpoint:**
```
POST /2020-01-01/extension/register
```

**Purpose:**
Register an extension with the Lambda Runtime API.

**Request:**
```http
POST /2020-01-01/extension/register
Content-Type: application/json

{
  "events": ["INVOKE", "SHUTDOWN"],
  "extensionName": "my-extension"
}
```

**Request Body Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `events` | Array | Yes | Event types to subscribe to |
| `extensionName` | String | Yes | Unique name for the extension |

**Event Types:**
| Event | Description |
|-------|-------------|
| `INVOKE` | Function is being invoked |
| `SHUTDOWN` | Sandbox is shutting down |

**Response (200 OK):**
```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "functionName": "my-function",
  "functionVersion": "$LATEST",
  "handler": "index.handler"
}
```

---

### 2.2 Next Extension Event (GET)

**Endpoint:**
```
GET /2020-01-01/extension/event/next
```

**Purpose:**
Long-polls for the next extension event.

**Response (200 OK):**
```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "eventType": "INVOKE",
  "deadlineMs": 1711555200000,
  "requestId": "abc123-def456",
  "invokedFunctionArn": "arn:aws:lambda:us-east-1:123456789012:function:my-function",
  "tracing": {
    "type": "X-Amzn-Trace-Id",
    "value": "Root=1-abc123-456def789"
  }
}
```

**Extension Event Types:**

| Event Type | Description | Data |
|------------|-------------|------|
| `INVOKE` | Function invoked | Request ID, deadline, trace info |
| `SHUTDOWN` | Sandbox shutting down | Shutdown reason |

**Shutdown Reasons:**
| Reason | Description |
|--------|-------------|
| `spindown` | Lambda is recycling the sandbox |
| `timeout` | Function timed out |
| `failure` | Function or runtime failed |
| `exit` | Runtime explicitly exited |

---

### 2.3 Post Extension Initialization Error (POST)

**Endpoint:**
```
POST /2020-01-01/extension/init/error
```

**Purpose:**
Report an initialization error from the extension.

**Request:**
```http
POST /2020-01-01/extension/init/error
Content-Type: application/json

{
  "errorType": "ExtensionInitError",
  "errorMessage": "Failed to initialize extension",
  "stackTrace": []
}
```

---

### 2.4 Post Extension Exit (POST)

**Endpoint:**
```
POST /2020-01-01/extension/exit
```

**Purpose:**
Signal that the extension is exiting.

**Request:**
```http
POST /2020-01-01/extension/exit
Content-Type: application/json

{
  "errorType": "ExtensionShutdown",
  "errorMessage": "Extension shutting down"
}
```

---

## Part 3: Streaming Endpoints

### 3.1 Stream Response Chunks (POST)

**Endpoint:**
```
POST /2018-06-01/runtime/invocation/{request-id}/response
```

**Purpose:**
For response streaming mode, send chunks of the response.

**Request:**
```http
POST /2018-06-01/runtime/invocation/{request-id}/response
Content-Type: application/octet-stream

<chunk-data>
```

**Response Streaming Configuration:**

To use response streaming:
1. Set `AWS_LWA_INVOKE_MODE=response_stream`
2. Configure Lambda Function URL with `InvokeMode: RESPONSE_STREAM`
3. Send chunks instead of complete response

**Example (chunked response):**
```rust
// First chunk
client.post("/runtime/invocation/abc123/response")
    .body(b"data: Hello\n\n".to_vec())
    .send()
    .await?;

// More chunks...
client.post("/runtime/invocation/abc123/response")
    .body(b"data: World\n\n".to_vec())
    .send()
    .await?;
```

---

## Part 4: Headers Reference

### Invocation Headers (from Runtime)

| Header | Description | Example |
|--------|-------------|---------|
| `Lambda-Runtime-Aws-Request-Id` | Unique request ID | `c6af9ac4-7b6b-4b6c-9c73-9d5c8f3e2a1b` |
| `Lambda-Runtime-Deadline-Ms` | Timeout timestamp (ms) | `1711555200000` |
| `Lambda-Runtime-Invoked-Function-Arn` | Function ARN | `arn:aws:lambda:us-east-1:123456789012:function:my-function` |
| `Lambda-Runtime-Trace-Id` | X-Ray trace ID | `Root=1-abc123-456def789` |
| `Lambda-Runtime-Client-Context` | Mobile context (base64) | `eyJjdXN0b20iOnt9fQ==` |
| `Lambda-Runtime-Cognito-Identity` | Cognito identity | `{"cognitoIdentityId":"us-east-1:abc123"}` |

### Required Headers (to Runtime)

| Header | When | Value |
|--------|------|-------|
| `Content-Type` | POST requests | `application/json` |

---

## Part 5: Error Handling

### Common Error Scenarios

#### 1. Runtime Not Registered

```http
GET /2018-06-01/runtime/invocation/next

HTTP/1.1 403 Forbidden
Content-Type: application/json

{"errorType": "Runtime.NotRegistered"}
```

**Fix:** Register your runtime/extension first.

#### 2. Invalid Request ID

```http
POST /2018-06-01/runtime/invocation/invalid-id/response

HTTP/1.1 404 Not Found
Content-Type: application/json

{"errorType": "Runtime.InvalidRequestID"}
```

**Fix:** Use the request ID from invocation headers.

#### 3. Response Already Sent

```http
POST /2018-06-01/runtime/invocation/abc123/response

HTTP/1.1 400 Bad Request
Content-Type: application/json

{"errorType": "Runtime.ResponseAlreadySent"}
```

**Fix:** Don't send multiple responses for same request ID.

#### 4. Timeout

```http
GET /2018-06-01/runtime/invocation/next

HTTP/1.1 504 Gateway Timeout
```

**Fix:** Handle timeouts gracefully and check deadline.

---

## Part 6: Implementation Patterns

### Basic Runtime Loop

```rust
async fn runtime_loop(client: &RuntimeClient) -> Result<(), Error> {
    loop {
        // 1. Get next invocation
        let (request_id, event) = client.get_next_invocation().await?;

        // 2. Calculate deadline
        let deadline = calculate_deadline_ms(&event.headers);
        let timeout = deadline - current_time_ms();

        // 3. Process with timeout
        let result = tokio::time::timeout(
            Duration::from_millis(timeout),
            process_event(event)
        ).await;

        // 4. Send response or error
        match result {
            Ok(Ok(response)) => {
                client.post_response(&request_id, response).await?;
            }
            Ok(Err(e)) => {
                client.post_error(&request_id, e).await?;
            }
            Err(_) => {
                client.post_timeout(&request_id).await?;
            }
        }
    }
}
```

### Extension Event Loop

```rust
async fn extension_loop(client: &ExtensionClient) -> Result<(), Error> {
    // 1. Register extension
    let registration = client.register_extension().await?;

    loop {
        // 2. Get next event
        let event = client.get_next_event().await?;

        match event.event_type {
            EventType::Invoke => {
                // Handle invoke event
                handle_invoke(event).await?;
            }
            EventType::Shutdown => {
                // Handle shutdown
                handle_shutdown(event).await?;
                break;
            }
        }
    }

    Ok(())
}
```

### Calculating Time Remaining

```rust
fn calculate_time_remaining(deadline_ms: u64) -> Duration {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let remaining_ms = deadline_ms.saturating_sub(now);
    Duration::from_millis(remaining_ms)
}
```

---

## Part 7: Testing the Runtime API

### Local Testing with Lambda Runtime Interface Emulator

AWS provides the Runtime Interface Emulator (RIE) for local testing:

```bash
# Run with RIE
docker run -d -p 9000:8080 \
  -e AWS_LAMBDA_RUNTIME_API=0.0.0.0:9001 \
  my-lambda-image

# Test invocation
curl -X POST "http://localhost:9000/2015-03-31/functions/function/invocations" \
  -d '{"key": "value"}'
```

### Mock Runtime API Server

```python
from http.server import HTTPServer, BaseHTTPRequestHandler
import json
import time

class MockRuntimeAPI(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/2018-06-01/runtime/invocation/next':
            time.sleep(1)  # Simulate long-poll
            self.send_response(200)
            self.send_header('Lambda-Runtime-Aws-Request-Id', 'mock-123')
            self.send_header('Lambda-Runtime-Deadline-Ms', str(int(time.time() * 1000) + 30000))
            self.end_headers()
            self.wfile.write(json.dumps({"test": "event"}).encode())

    def do_POST(self):
        self.send_response(202)
        self.end_headers()

HTTPServer(('localhost', 9001), MockRuntimeAPI).serve_forever()
```

---

## Summary

| Category | Key Points |
|----------|------------|
| **Endpoints** | `/invocation/next`, `/invocation/{id}/response`, `/invocation/{id}/error` |
| **Extensions** | Register, event polling, lifecycle management |
| **Headers** | Request ID, deadline, trace ID, function ARN |
| **Errors** | 403 (not registered), 404 (invalid ID), 400 (bad request) |
| **Streaming** | Chunked responses for low-latency use cases |

---

*Continue to [02-adapter-pattern-deep-dive.md](02-adapter-pattern-deep-dive.md) to learn how the Web Adapter implements these patterns.*
