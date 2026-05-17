# rust-rpc-router — RPC Response

**Source:** `src/rpc_response/` — 3 files. JSON-RPC 2.0 response construction and serialization.

## RpcResponse — Unified Response Type

```rust
// rpc_response/response.rs:14-19
#[derive(Debug, Clone, PartialEq)]
pub enum RpcResponse {
    Success(RpcSuccessResponse),
    Error(RpcErrorResponse),
}
```

**Aha:** `CallSuccess`/`CallError` (from the router) and `RpcResponse`/`RpcError` (from the response module) are deliberately separate. The comment in `call_success.rs` explains: "CallSuccess & CallError are not designed to be the JSON-RPC Response or Error, but to provide the necessary context to build those, as well as the useful `method name` context for tracing/logging." The router returns `CallResult` (which includes the method name for debugging), while `RpcResponse` is the actual JSON-RPC 2.0 wire format.

### RpcSuccessResponse

```rust
// rpc_response/response.rs:22-29
pub struct RpcSuccessResponse {
    pub id: RpcId,
    pub result: Value,
}
```

### RpcErrorResponse

```rust
// rpc_response/response.rs:32-39
pub struct RpcErrorResponse {
    pub id: RpcId,
    pub error: RpcError,
}
```

### From Router Results

```rust
// rpc_response/response.rs:86-111
impl From<CallSuccess> for RpcResponse { ... }
impl From<CallError> for RpcResponse { ... }
impl From<CallResult> for RpcResponse { ... }
```

These conversions bridge the router layer to the JSON-RPC wire format:

```rust
let call_result: CallResult = router.call(request).await;
let rpc_response = RpcResponse::from(call_result);
// Serialize to JSON for sending back to client
let json = serde_json::to_string(&rpc_response)?;
```

### Accessors

```rust
impl RpcResponse {
    pub fn is_success(&self) -> bool { matches!(self, RpcResponse::Success(_)) }
    pub fn is_error(&self) -> bool { matches!(self, RpcResponse::Error(_)) }
    pub fn id(&self) -> &RpcId { ... }
    pub fn into_parts(self) -> (RpcId, Result<Value, RpcError>) { ... }
}
```

## RpcError — JSON-RPC 2.0 Error Object

```rust
// rpc_response/rpc_error.rs:8-19
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}
```

Conforms to the [JSON-RPC 2.0 error object spec](https://www.jsonrpc.org/specification#error_object).

### Standard Error Codes

```rust
// rpc_response/rpc_error.rs:24-29
impl RpcError {
    pub const CODE_PARSE_ERROR: i64 = -32700;
    pub const CODE_INVALID_REQUEST: i64 = -32600;
    pub const CODE_METHOD_NOT_FOUND: i64 = -32601;
    pub const CODE_INVALID_PARAMS: i64 = -32602;
    pub const CODE_INTERNAL_ERROR: i64 = -32603;
}
```

### Constructor Helpers

| Constructor | Code | Message |
|-------------|------|---------|
| `from_parse_error(data)` | -32700 | "Parse error" |
| `from_invalid_request(data)` | -32600 | "Invalid Request" |
| `from_method_not_found(data)` | -32601 | "Method not found" |
| `from_invalid_params(data)` | -32602 | "Invalid params" |
| `from_internal_error(data)` | -32603 | "Internal error" |

### Router Error → RpcError Mapping

```rust
// rpc_response/rpc_error.rs:85-99
impl From<&Error> for RpcError {
    fn from(err: &Error) -> Self {
        match err {
            Error::ParamsParsing(p) => Self::new(CODE_INVALID_PARAMS, "Invalid params", Some(p)),
            Error::ParamsMissingButRequested => Self::new(CODE_INVALID_PARAMS, "Invalid params", Some(err)),
            Error::MethodUnknown => Self::new(CODE_METHOD_NOT_FOUND, "Method not found", Some(err)),
            Error::FromResources(fr_err) => Self::new(CODE_INTERNAL_ERROR, "Internal error", Some(fr_err)),
            Error::HandlerResultSerialize(s_err) => Self::new(CODE_INTERNAL_ERROR, "Internal error", Some(s_err)),
            Error::Handler(h_err) => Self::new(CODE_INTERNAL_ERROR, "Internal error", Some(h_err)),
        }
    }
}
```

**Aha:** The `HandlerError` case uses a generic "Internal error" — the comment notes that a future enhancement could add a trait on the wrapped error type to allow handlers to specify their own RPC error codes and messages. Currently, all application-level errors map to -32603.

## Serialization — JSON-RPC 2.0 Wire Format

```rust
// rpc_response/response.rs:117-138
impl Serialize for RpcResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("jsonrpc", "2.0")?;

        match self {
            RpcResponse::Success(RpcSuccessResponse { id, result }) => {
                map.serialize_entry("id", id)?;
                map.serialize_entry("result", result)?;
            }
            RpcResponse::Error(RpcErrorResponse { id, error }) => {
                map.serialize_entry("id", id)?;
                map.serialize_entry("error", error)?;
            }
        }
        map.end()
    }
}
```

Output format:

```json
// Success:
{"jsonrpc": "2.0", "id": 1, "result": {"data": "ok"}}

// Error:
{"jsonrpc": "2.0", "id": "req-abc", "error": {"code": -32601, "message": "Method not found", "data": "..."}}
```

## Deserialization — Custom Visitor

```rust
// rpc_response/response.rs:140-246
impl<'de> Deserialize<'de> for RpcResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> {
        struct RpcResponseVisitor;
        // Custom visitor that:
        // 1. Validates jsonrpc = "2.0"
        // 2. Requires id (missing → MissingId error)
        // 3. Requires exactly one of result or error
        // 4. Rejects both result and error present
    }
}
```

The custom deserializer validates the complete JSON-RPC 2.0 response structure:

| Invalid Input | Error |
|--------------|-------|
| Missing `jsonrpc` | `MissingJsonRpcVersion` |
| Wrong `jsonrpc` version | `InvalidJsonRpcVersion` |
| Missing `id` | `MissingId` |
| Invalid `id` type | `InvalidId` |
| Both `result` and `error` | `BothResultAndError` |
| Neither `result` nor `error` | `MissingResultAndError` |
| `error` not a valid object | `InvalidErrorObject` |

## RpcResponseParsingError

```rust
// rpc_response/rpc_response_parsing_error.rs:9-28
pub enum RpcResponseParsingError {
    InvalidJsonRpcVersion { id: Option<RpcId>, expected: &'static str, actual: Option<Value> },
    MissingJsonRpcVersion { id: Option<RpcId> },
    MissingId,
    InvalidId(RpcRequestParsingError),
    MissingResultAndError { id: RpcId },
    BothResultAndError { id: RpcId },
    InvalidErrorObject(serde_json::Error),
    Serde(serde_json::Error),
}
```

Used when deserializing `RpcResponse` from JSON. The `InvalidId` variant wraps `RpcRequestParsingError` since ID parsing failures are the same as request parsing failures.
