# rust-rpc-router — RPC Messages

**Source:** `src/rpc_message/` — 4 files. JSON-RPC 2.0 request and notification parsing with validation.

## RpcRequest — JSON-RPC 2.0 Request

```rust
// rpc_message/request.rs:9-13
#[derive(Deserialize, Clone, Debug)]
pub struct RpcRequest {
    pub id: RpcId,
    pub method: String,
    pub params: Option<Value>,
}
```

### Parsing from Value

```rust
// rpc_message/request.rs:27-116
pub fn from_value(value: Value) -> Result<RpcRequest, RpcRequestParsingError> {
    RpcRequest::from_value_with_checks(value, RpcRequestCheckFlags::ALL)
}
```

The parser performs these validations in order:

1. **Type check** — input must be a JSON object
2. **Version check** — `jsonrpc` field must exist and equal `"2.0"`
3. **Method extraction** — must exist and be a string
4. **ID parsing** — validated against `RpcId` rules (String, Number, Null only)
5. **Params extraction** — optional, taken from remaining object

### RpcRequestCheckFlags

```rust
// rpc_message/request.rs:151-164
bitflags::bitflags! {
    pub struct RpcRequestCheckFlags: u32 {
        const VERSION = 0b00000001;  // Check jsonrpc = "2.0"
        const ID = 0b00000010;       // Validate ID type
        const ALL = Self::VERSION.bits() | Self::ID.bits();
    }
}
```

The flags control validation strictness. With `RpcRequestCheckFlags::ALL` (the default), both version and ID are validated. Without `ID` flag, missing or invalid IDs default to `RpcId::Null`. Without `VERSION` flag, the `jsonrpc` field is not checked.

**Aha:** The parser destructively mutates the input `Value` — it removes `jsonrpc`, `id`, `method`, and `params` keys from the object map. This means the original `Value` is consumed and cannot be reused. The comment on line 38 notes a TODO: redact large objects/arrays in error messages to prevent memory issues with maliciously large payloads.

### Custom Serialization

```rust
// rpc_message/request.rs:121-147
impl serde::Serialize for RpcRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut field_count = 3;
        if self.params.is_some() { field_count += 1; }

        let mut state = serializer.serialize_struct("RpcRequest", field_count)?;
        state.serialize_field("jsonrpc", "2.0")?;  // Always "2.0"
        state.serialize_field("id", &self.id)?;
        state.serialize_field("method", &self.method)?;
        if let Some(params) = &self.params {
            state.serialize_field("params", params)?;
        }
        state.end()
    }
}
```

Serialization always adds `"jsonrpc": "2.0"` even though the struct doesn't store it — this is derived at serialize time.

### TryFrom<Value>

```rust
impl TryFrom<Value> for RpcRequest {
    type Error = RpcRequestParsingError;
    fn try_from(value: Value) -> Result<RpcRequest, RpcRequestParsingError> {
        RpcRequest::from_value(value)  // performs version validation
    }
}
```

For permissive parsing without version checks, use `serde_json::from_value` directly — it derives `Deserialize` and skips the `jsonrpc` validation.

## RpcNotification — JSON-RPC 2.0 Notification

```rust
// rpc_message/notification.rs:16-20
pub struct RpcNotification {
    pub method: String,
    pub params: Option<Value>,
}
```

Notifications are requests **without an `id`** — they are fire-and-forget. The JSON-RPC 2.0 spec states that notifications should not receive a response.

### Validation

`RpcNotification::from_value` performs stricter validation than `RpcRequest`:

1. Input must be a JSON object
2. `jsonrpc` must be `"2.0"` (always checked — no flags)
3. `method` must exist and be a string
4. `params` must be absent, array, or object (not string, number, bool)
5. **`id` must NOT be present** — if present, returns `NotificationHasId` error

```rust
// rpc_message/notification.rs:82-87
if let Some(id_val) = extract_value(&mut obj, "id") {
    return Err(RpcRequestParsingError::NotificationHasId {
        method: Some(method),
        id: id_val,
    });
}
```

### Conversion from RpcRequest

```rust
// rpc_message/notification.rs:126-133
impl From<RpcRequest> for RpcNotification {
    fn from(request: RpcRequest) -> Self {
        RpcNotification {
            method: request.method,
            params: request.params,
        }
    }
}
```

Discarding the `id` from an `RpcRequest` yields an `RpcNotification`.

## RpcRequestParsingError — Parsing Failures

```rust
// rpc_message/rpc_request_parsing_error.rs:17-63
pub enum RpcRequestParsingError {
    RequestInvalidType { actual_type: String },       // Not a JSON object
    ParamsInvalidType { actual_type: String },        // Params not array/object
    VersionMissing { id: Option<Value>, method: Option<String> },
    VersionInvalid { id: Option<Value>, method: Option<String>, version: Value },
    MethodMissing { id: Option<Value> },
    MethodInvalidType { id: Option<Value>, method: Value },
    NotificationHasId { method: Option<String>, id: Value },
    MethodInvalid { actual: String },
    IdMissing { method: Option<String> },
    IdInvalid { actual: String, cause: String },
    Parse(serde_json::Error),                         // Generic JSON parse error
}
```

**Aha:** Error variants store `Option<Value>` for `id` because during parsing, the ID might not have been validated yet (it could be an array, object, or boolean). The comment notes: "By design, we do not capture the 'params' because they could be indefinitely large." A future improvement plans to replace captured objects/arrays with redaction strings like `"[object/array redacted, 'id' must be of type number, string, or null]"`.

## Shared Parsing Support Functions

```rust
// rpc_message/support.rs:8-47
pub(super) fn extract_value(obj: &mut Map<String, Value>, key: &str) -> Option<Value>
pub(super) fn validate_version(version_val: Option<Value>) -> Result<(), Option<Value>>
pub(super) fn parse_method(method_val: Option<Value>) -> Result<String, Option<Value>>
pub(super) fn parse_params(params_val: Option<Value>) -> Result<Option<Value>, RpcRequestParsingError>
```

These are used by both `RpcRequest` and `RpcNotification` parsing. `parse_params` is the only one that returns `RpcRequestParsingError` directly — the others return `Result<T, Option<Value>>` so the caller can construct richer error messages with context.

### Params Validation

```rust
// rpc_message/support.rs:39-47
fn parse_params(params_val: Option<Value>) -> Result<Option<Value>, RpcRequestParsingError> {
    match params_val {
        None => Ok(None),
        Some(Value::Array(_)) | Some(Value::Object(_)) => Ok(Some(params_val.unwrap())),
        Some(other) => Err(RpcRequestParsingError::ParamsInvalidType {
            actual_type: get_json_type(&other).to_string(),
        }),
    }
}
```

JSON-RPC 2.0 allows params to be an array (positional) or an object (named). Strings, numbers, booleans, and null are invalid for params.

## JsonType Detection

```rust
// support.rs:6-37
pub enum JsonType {
    Null, Bool, Integer, Unsigned, Float, String, Array, Object,
}

pub fn get_json_type(value: &Value) -> JsonType {
    match value {
        Value::Null => JsonType::Null,
        Value::Bool(_) => JsonType::Bool,
        Value::Number(n) => {
            if n.is_i64() { JsonType::Integer }
            else if n.is_u64() { JsonType::Unsigned }
            else if n.is_f64() { JsonType::Float }
            else { unreachable!("serde_json::Number should be i64, u64, or f64") }
        }
        Value::String(_) => JsonType::String,
        Value::Array(_) => JsonType::Array,
        Value::Object(_) => JsonType::Object,
    }
}
```

Used in error messages to report the actual type of invalid JSON values. Splits `Number` into `Integer`, `Unsigned`, and `Float` for precise error reporting.
