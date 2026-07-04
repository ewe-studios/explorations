# rust-rpc-router: JSON-RPC 2.0 Router

JSON-RPC 2.0 router with `FromResources` dependency injection and async handler support.

Source: `rust-rpc-router/src/` — 10+ modules.

## Module Structure

```
rust-rpc-router/src/
├── lib.rs            # Router, RpcHandler, IntoParams, FromResources
├── router/           # Router implementation
├── handler/          # Handler trait and wrappers
├── params/           # IntoParams trait
├── resource/         # FromResources trait
├── rpc_id.rs         # RPC ID type
├── rpc_message.rs    # RPC message types
├── rpc_response.rs   # RPC response types
├── error.rs          # Error type
└── support.rs        # Utilities
```

## Constructs

| Construct | Purpose |
|-----------|---------|
| `Router` | HashMap of `method_name: Box<dyn RpcHandlerWrapperTrait>` |
| `RpcHandler` | Trait for async functions with `(S1, S2, ...impl IntoParams) → Result<Serialize>` |
| `IntoParams` | Trait to convert `Option<Value>` JSON-RPC params to handler param types |
| `FromResources` | Dependency injection — types resolved from shared resources |

## IntoParams

Default implementation returns an error if params are missing. For types implementing `Default`, `IntoDefaultParams` auto-implements `IntoParams` calling `T::default()` when params are `None`.

```rust
#[derive(Deserialize)]
pub struct ParamsIded {
    id: i64,
}
impl IntoParams for ParamsIded {}
```

Custom behavior: implement `IntoParams::into_params`.

## Proc Macros

| Macro | Purpose |
|-------|---------|
| `#[RpcParams]` | Derive IntoParams |
| `#[RpcResource]` | Derive FromResources |
| `#[RpcHandlerError]` | Error type for handlers |

## Cross-References

- [Overview](00-overview.md) — All 21 projects

[Next → rust-modql](03-rust-modql.md)
