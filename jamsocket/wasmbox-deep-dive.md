# Wasmbox: Lightweight WASM Service Framework

Wasmbox is a minimal framework for running WASM services with bidirectional communication.

## Overview

**Wasmbox** provides a simple abstraction for WASM-based services:

- **Async Support**: Async/await in WASM
- **Message Passing**: Typed input/output channels
- **Single-threaded**: Runs on WASM's single thread
- **Host Integration**: Clean host/guest boundary

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Wasmbox Host                           │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              AsyncWasmBoxBox                          │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │              Future                             │  │  │
│  │  │  ┌───────────────────────────────────────────┐  │  │  │
│  │  │  │           AsyncWasmBox::run()             │  │  │  │
│  │  │  │                                           │  │  │  │
│  │  │  │  loop {                                   │  │  │  │
│  │  │  │    let input = ctx.next().await;          │  │  │  │
│  │  │  │    let output = process(input);           │  │  │  │
│  │  │  │    ctx.send(output);                      │  │  │  │
│  │  │  │  }                                        │  │  │  │
│  │  │  └───────────────────────────────────────────┘  │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  │         │                        │                     │  │
│  │  ┌──────▼──────┐          ┌──────▼──────┐            │  │
│  │  │   Sender    │          │   Receiver  │            │  │
│  │  │  (Input)    │          │  (Output)   │            │  │
│  │  └─────────────┘          └─────────────┘            │  │
│  └────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Core Traits

### WasmBox

The base trait for synchronous WASM boxes:

```rust
pub trait WasmBox: 'static {
    type Input: Serialize;
    type Output: DeserializeOwned;

    /// Initialize the box with a callback for outputs
    fn init(callback: Box<dyn Fn(Self::Output) + Send + Sync>) -> Self
    where
        Self: Sized;

    /// Send a message to the box
    fn message(&mut self, input: Self::Input);
}
```

### AsyncWasmBox

For async WASM services:

```rust
#[async_trait]
pub trait AsyncWasmBox: 'static + Sized {
    type Input: Serialize;
    type Output: DeserializeOwned;

    async fn run(ctx: WasmBoxContext<Self::Input, Self::Output>);
}
```

### WasmBoxContext

Context provided to the WASM service:

```rust
pub struct WasmBoxContext<Input, Output> {
    callback: Box<dyn Fn(Output) + Send + Sync>,
    queue: IgnoreSend<Rc<Receiver<Input>>>,
    _ph_o: PhantomData<Output>,
}

impl<Input, Output> WasmBoxContext<Input, Output> {
    /// Send output to host
    pub fn send(&self, output: Output);

    /// Wait for next input (returns a Future)
    pub fn next(&self) -> NextMessageFuture<Input>;
}
```

## Async Implementation

### NextMessageFuture

```rust
pub struct NextMessageFuture<Input> {
    _ph_output: PhantomData<Input>,
    queue: IgnoreSend<Rc<Receiver<Input>>>,
}

impl<Input> Future for NextMessageFuture<Input> {
    type Output = Input;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Input> {
        match self.queue.0.try_recv() {
            Ok(value) => Poll::Ready(value),
            Err(TryRecvError::Empty) => Poll::Pending,
            Err(TryRecvError::Disconnected) => panic!("Queue became disconnected."),
        }
    }
}
```

### AsyncWasmBoxBox

Wraps async WASM boxes:

```rust
pub struct AsyncWasmBoxBox<B>
where
    B: AsyncWasmBox,
{
    future: Pin<Box<dyn Future<Output = ()>>>,
    sender: Sender<B::Input>,
    _ph_b: PhantomData<B>,
    waker: Waker,
}

impl<B> AsyncWasmBoxBox<B>
where
    B: AsyncWasmBox,
{
    fn poll(&mut self) {
        match self.future.as_mut().poll(&mut Context::from_waker(&self.waker)) {
            Poll::Ready(_) => panic!("Function exited."),
            Poll::Pending => (),
        }
    }
}

impl<B> WasmBox for AsyncWasmBoxBox<B>
where
    B: AsyncWasmBox,
{
    type Input = B::Input;
    type Output = B::Output;

    fn init(callback: Box<dyn Fn(B::Output) + Send + Sync>) -> Self {
        let (sender, recv) = channel();
        let ctx = WasmBoxContext::new(callback, recv);
        let future = B::run(ctx);
        let waker = dummy_context::waker();

        let mut async_box = AsyncWasmBoxBox {
            future,
            sender,
            waker,
            _ph_b: PhantomData::default(),
        };

        async_box.poll();  // Initial poll
        async_box
    }

    fn message(&mut self, input: Self::Input) {
        self.sender.send(input).expect("Error sending message.");
        self.poll();  // Poll after message
    }
}
```

### Dummy Waker

Since WASM is single-threaded, a dummy waker is used:

```rust
mod dummy_context {
    pub fn waker() -> Waker {
        unsafe { Waker::from_raw(raw_waker()) }
    }

    // Never actually wakes - polling is done synchronously
    unsafe fn wake(_: WakerData) {
        panic!("Should never wake dummy waker!")
    }
}
```

## IgnoreSend Wrapper

WASM is single-threaded, so Send bounds can be safely ignored:

```rust
#[derive(Clone)]
struct IgnoreSend<T>(pub T);

unsafe impl<T> Send for IgnoreSend<T> {}
unsafe impl<T> Sync for IgnoreSend<T> {}
```

This allows using `Rc` (not `Arc`) internally while satisfying `Send` bounds.

## Example: Counter Service

### Service Definition

```rust
use wasmbox::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum Input {
    Increment,
    Decrement,
    GetValue,
}

#[derive(Serialize, Deserialize)]
enum Output {
    Value(i32),
}

struct Counter {
    value: i32,
}

#[async_trait]
impl AsyncWasmBox for Counter {
    type Input = Input;
    type Output = Output;

    async fn run(ctx: WasmBoxContext<Self::Input, Self::Output>) {
        let mut value = 0;

        loop {
            let input = ctx.next().await;
            match input {
                Input::Increment => {
                    value += 1;
                    ctx.send(Output::Value(value));
                }
                Input::Decrement => {
                    value -= 1;
                    ctx.send(Output::Value(value));
                }
                Input::GetValue => {
                    ctx.send(Output::Value(value));
                }
            }
        }
    }
}
```

### Host Integration

```rust
// Create the WASM box
let mut box = AsyncWasmBoxBox::<Counter>::init(Box::new(|output| {
    println!("Output: {:?}", output);
}));

// Send messages
box.message(Input::Increment);  // Output: Value(1)
box.message(Input::Increment);  // Output: Value(2)
box.message(Input::Decrement);  // Output: Value(1)
```

## Message Flow

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│    Host     │     │  WasmBox     │     │   Service   │
│             │     │              │     │             │
│  message()  │────>│  sender.send │────>│  channel    │
│             │     │              │     │             │
│  callback() │<────│  ctx.send()  │<────│  next().await│
│             │     │              │     │             │
└─────────────┘     └──────────────┘     └─────────────┘
```

## Serialization

Inputs and outputs use serde for serialization:

```rust
pub trait WasmBox: 'static {
    type Input: Serialize;
    type Output: DeserializeOwned;
}
```

Supported formats:
- JSON (serde_json)
- CBOR (ciborium)
- Bincode
- Any serde format

## CLI Tool

The `wasmbox-cli` provides commands for running services:

```bash
# Build
wasmbox-cli build

# Run
wasmbox-cli run service.wasm
```

## Use Cases

### Echo Service

```rust
struct Echo;

#[async_trait]
impl AsyncWasmBox for Echo {
    type Input = String;
    type Output = String;

    async fn run(ctx: WasmBoxContext<String, String>) {
        loop {
            let input = ctx.next().await;
            ctx.send(format!("Echo: {}", input));
        }
    }
}
```

### Transform Pipeline

```rust
struct Transform<F> {
    transform_fn: F,
}

#[async_trait]
impl<F, I, O> AsyncWasmBox for Transform<F>
where
    F: Fn(I) -> O + Send + Sync,
    I: DeserializeOwned,
    O: Serialize,
{
    type Input = I;
    type Output = O;

    async fn run(ctx: WasmBoxContext<I, O>) {
        loop {
            let input = ctx.next().await;
            ctx.send((self.transform_fn)(input));
        }
    }
}
```

### Request/Response Pattern

```rust
enum Input {
    Get { key: String },
    Set { key: String, value: String },
}

enum Output {
    Value(Option<String>),
    Ok,
}

struct KeyValueStore {
    store: HashMap<String, String>,
}

#[async_trait]
impl AsyncWasmBox for KeyValueStore {
    type Input = Input;
    type Output = Output;

    async fn run(ctx: WasmBoxContext<Self::Input, Self::Output>) {
        let mut store = HashMap::new();

        loop {
            let input = ctx.next().await;
            match input {
                Input::Get { key } => {
                    ctx.send(Output::Value(store.get(&key).cloned()));
                }
                Input::Set { key, value } => {
                    store.insert(key, value);
                    ctx.send(Output::Ok);
                }
            }
        }
    }
}
```

## Limitations

1. **Single-threaded**: All computation on one thread
2. **No real async**: Async is simulated via polling
3. **Memory limits**: WASM memory is limited
4. **No blocking**: Cannot block the main thread

## Comparison to Alternatives

| Feature | Wasmbox | Stateroom | WASI |
|---------|---------|-----------|------|
| Async | Yes | No | Yes |
| WebSocket | External | Built-in | External |
| Multi-client | Host manages | Built-in | Host manages |
| State | Per-box | Per-room | Per-instance |
