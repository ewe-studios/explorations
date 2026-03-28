---
title: "Yew Frontend Deep Dive"
subtitle: "Building WebAssembly UIs with StateProgramComponent"
prerequisites: [04-stateroom-deep-dive.md](04-stateroom-deep-dive.md)
next: [rust-revision.md](rust-revision.md)
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.driftingspace/aper/aper-yew/
---

# Yew Frontend Deep Dive

This document explores how Aper integrates with Yew (a Rust web framework) to build reactive WebAssembly frontends that synchronize with the server.

## Table of Contents

1. [Yew Framework Overview](#1-yew-framework-overview)
2. [StateProgramComponent](#2-stateprogramcomponent)
3. [StateProgramViewComponent](#3-stateprogramviewcomponent)
4. [WebSocket Connection](#4-websocket-connection)
5. [Session Persistence](#5-session-persistence)
6. [Message Types and Updates](#6-message-types-and-updates)
7. [View Rendering](#7-view-rendering)
8. [Complete Example: Counter UI](#8-complete-example-counter-ui)

---

## 1. Yew Framework Overview

**Yew** is a Rust framework for building web frontends with WebAssembly. It uses a component-based architecture similar to React.

### Key Concepts

| Concept | Description |
|---------|-------------|
| Component | Reusable UI element with state |
| Properties | Props passed to components |
| Message | Events that trigger updates |
| Context | Component lifecycle handle |
| Html | JSX-like markup for rendering |

### Basic Component Structure

```rust
use yew::prelude::*;

#[function_component]
fn HelloWorld() -> Html {
    html! { <h1>{"Hello, World!"}</h1> }
}
```

### Component with State

```rust
use yew::prelude::*;

pub struct Counter {
    value: i32,
}

pub enum Msg {
    Increment,
    Decrement,
}

impl Component for Counter {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self { value: 0 }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Increment => self.value += 1,
            Msg::Decrement => self.value -= 1,
        }
        true // Re-render
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div>
                <button onclick={_ctx.link().callback(|_| Msg::Decrement)}>{"-1"}</button>
                <p>{self.value}</p>
                <button onclick={_ctx.link().callback(|_| Msg::Increment)}>{"+1"}</button>
            </div>
        }
    }
}
```

---

## 2. StateProgramComponent

`StateProgramComponent` is the main Yew component that manages the WebSocket connection and state synchronization.

### Structure

```rust
pub struct StateProgramComponent<V: StateProgramViewComponent> {
    /// WebSocket connection to the server
    client: Option<AperWebSocketStateProgramClient<V::Program>>,
    /// Local copy of state
    state: Option<InnerState<V::Program>>,
    _ph: PhantomData<V>,
}

struct InnerState<P: StateProgram> {
    state: Rc<P>,
    offset: Duration,       // Server time offset
    client_id: ClientId,
}
```

### Properties

```rust
#[derive(Properties, Clone)]
pub struct StateProgramComponentProps<V: StateProgramViewComponent> {
    /// WebSocket URL (ws:// or wss://)
    pub websocket_url: String,
    pub _ph: PhantomData<V>,
}

impl<V: StateProgramViewComponent> StateProgramComponentProps<V> {
    pub fn new(websocket_url: &str) -> Self {
        StateProgramComponentProps {
            websocket_url: get_full_ws_url(websocket_url),
            _ph: PhantomData::default(),
        }
    }
}
```

### Component Implementation

```rust
impl<V: StateProgramViewComponent> Component for StateProgramComponent<V> {
    type Message = Msg<V::Program>;
    type Properties = StateProgramComponentProps<V>;

    fn create(context: &yew::Context<Self>) -> Self {
        let mut result = Self {
            client: None,
            state: None,
            _ph: PhantomData::default(),
        };

        result.do_connect(context);
        result
    }

    fn update(&mut self, _: &yew::Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::StateTransition(transition) => {
                // Send transition to server
                self.client.as_mut().unwrap().push_transition(transition);
                false // Don't re-render (state update triggers render)
            }
            Msg::SetState(state, offset, client_id) => {
                // Update local state from server
                self.state = Some(InnerState {
                    state,
                    offset,
                    client_id,
                });
                true // Re-render with new state
            }
            Msg::Redraw => true,
        }
    }

    fn view(&self, context: &yew::Context<Self>) -> Html {
        if let Some(inner_state) = &self.state {
            let context = StateProgramViewContext {
                callback: context.link().callback(Msg::StateTransition),
                redraw: context.link().callback(|_| Msg::Redraw),
                client_id: inner_state.client_id,
                offset: inner_state.offset,
            };
            V::view(inner_state.state.clone(), context)
        } else {
            html! { <p>{"Waiting for initial state..."}</p> }
        }
    }
}
```

---

## 3. StateProgramViewComponent

`StateProgramViewComponent` is a trait for defining how to render your state.

### Trait Definition

```rust
pub trait StateProgramViewComponent {
    type Program: StateProgram;

    fn view(
        state: Rc<Self::Program>,
        context: StateProgramViewContext<Self::Program>,
    ) -> Html;
}
```

### View Context

```rust
pub struct StateProgramViewContext<P: StateProgram> {
    /// Callback to send transitions to the server
    pub callback: Callback<P::T>,

    /// Callback to trigger a re-render
    pub redraw: Callback<()>,

    /// This client's ID
    pub client_id: ClientId,

    /// Server time offset
    pub offset: Duration,
}
```

### Implementing View Component

```rust
struct CounterView;

impl StateProgramViewComponent for CounterView {
    type Program = StateMachineContainerProgram<Counter>;

    fn view(
        state: Rc<Self::Program>,
        context: StateProgramViewContext<Self::Program>,
    ) -> Html {
        html! {
            <div>
                <p>{format!("Counter: {}", state.0.value())}</p>
                <button onclick={context.callback.reform(|_| CounterTransition::Add(1))}>
                    {"+1"}
                </button>
                <button onclick={context.callback.reform(|_| CounterTransition::Subtract(1))}>
                    {"-1"}
                </button>
                <button onclick={context.callback.reform(|_| CounterTransition::Reset)}>
                    {"Reset"}
                </button>
            </div>
        }
    }
}
```

### Using reform()

`reform()` transforms a callback's input:

```rust
// callback: Callback<CounterTransition>
// reform creates: Callback<MouseEvent>

context.callback.reform(|_| CounterTransition::Add(1))
// Click → MouseEvent → CounterTransition.Add(1) → callback
```

---

## 4. WebSocket Connection

The WebSocket client handles the connection to the Stateroom server.

### AperWebSocketStateProgramClient

```rust
pub struct AperWebSocketStateProgramClient<S>
where
    S: StateProgram,
{
    conn: Rc<Conn<S>>,
    state_client: Rc<Mutex<StateProgramClient<S>>>,
    callback: BoxedCallback<S>,
}
```

### Connection Setup

```rust
impl<S> AperWebSocketStateProgramClient<S>
where
    S: StateProgram,
{
    pub fn new<F>(url: &str, callback: F) -> Result<Self>
    where
        F: Fn(Rc<S>, Duration, ClientId) + 'static,
    {
        let state_client: Rc<Mutex<StateProgramClient<S>>> = Rc::default();
        let callback: BoxedCallback<S> = Rc::new(Box::new(callback));

        let conn = Rc::new_cyclic(|conn: &Weak<Conn<S>>| {
            let callback = callback.clone();
            let typed_callback: Box<dyn Fn(StateProgramMessage<S>)> = {
                let state_client = state_client.clone();
                let conn = conn.clone();

                Box::new(move |message: StateProgramMessage<S>| {
                    let mut lock = state_client.lock().unwrap();
                    if let Some(response) = lock.receive_message_from_server(message) {
                        conn.upgrade().unwrap().send(&response);
                    }
                    let state = lock.state().unwrap();
                    callback(state.state(), state.server_time_delta, state.client_id);
                })
            };

            TypedWebsocketConnection::new(url, typed_callback).unwrap()
        });

        Ok(AperWebSocketStateProgramClient {
            conn,
            state_client,
            callback,
        })
    }
}
```

### Push Transition

```rust
pub fn push_transition(&self, transition: S::T) {
    let mut lock = self.state_client.lock().unwrap();
    if let Ok(message_to_server) = lock.push_transition(transition) {
        self.conn.send(&message_to_server);
        let state = lock.state().unwrap();
        (self.callback)(state.state(), state.server_time_delta, state.client_id);
    }
}
```

---

## 5. Session Persistence

The client persists connection tokens across page reloads.

### Token Storage

```rust
const CONNECTION_TOKEN_KEY: &str = "CONNECTION_TOKEN";

fn do_connect(&mut self, context: &yew::Context<Self>) {
    let link = context.link().clone();

    // Get or create token
    let token = if let Ok(token) = SessionStorage::get::<String>(CONNECTION_TOKEN_KEY) {
        token
    } else {
        // Generate new token
        let token: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(24)
            .map(char::from)
            .collect();

        SessionStorage::set(CONNECTION_TOKEN_KEY, &token)
            .expect("Couldn't set session state.");
        token
    };

    // Connect with token
    let url = format!("{}?token={}", context.props().websocket_url, token);
    // ...
}
```

### Why Session Storage?

| Storage Type | Persists | Use Case |
|--------------|----------|----------|
| SessionStorage | Tab session | Connection tokens |
| LocalStorage | Forever | User preferences |
| Memory | Component lifetime | Temporary state |

---

## 6. Message Types and Updates

### Component Messages

```rust
#[derive(Debug)]
pub enum Msg<State: StateProgram> {
    /// User triggered a transition
    StateTransition(State::T),

    /// Server sent new state
    SetState(Rc<State>, Duration, ClientId),

    /// Force re-render
    Redraw,
}
```

### Update Flow

```
User clicks button
       │
       ▼
┌──────────────────┐
│ StateTransition  │
│ message created  │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ client.push_     │
│ transition()     │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ WebSocket send   │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Server processes │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Server broadcast │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ SetState message │
│ received         │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ state updated    │
│ re-render        │
└──────────────────┘
```

---

## 7. View Rendering

### HTML Markup

Yew uses `html!` macro for JSX-like syntax:

```rust
html! {
    <div class="container">
        <h1>{"Title"}</h1>
        <p>{some_value}</p>
        <button onclick={callback}>{"Click"}</button>
        <ul>
            {for items.iter().map(|item| {
                html! { <li>{item}</li> }
            })}
        </ul>
    </div>
}
```

### Event Handlers

```rust
// Simple callback
onclick={context.callback.reform(|_| Transition::Action)}

// With state access
onclick={{
    let state = state.clone();
    context.callback.reform(move |_| {
        Transition::WithValue(state.0.some_field)
    })
}}

// Multiple parameters
onclick={{
    let cb = context.callback.clone();
    Callback::from(move |_| {
        cb.emit(Transition::Complex {
            field1: value1,
            field2: value2,
        })
    })
}}
```

### Conditional Rendering

```rust
html! {
    <div>
        {if *show_details {
            html! { <DetailsComponent /> }
        } else {
            html! { <SummaryComponent /> }
        }}
    </div>
}
```

### Iteration

```rust
html! {
    <ul>
        {for state.0.items.iter().map(|item| {
            html! {
                <li key={item.id}>
                    {item.text.clone()}
                </li>
            }
        })}
    </ul>
}
```

---

## 8. Complete Example: Counter UI

Here's a complete counter application:

### Common Code (shared)

```rust
// common/src/lib.rs
use aper::{StateMachine, NeverConflict};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Counter {
    value: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CounterTransition {
    Add(i64),
    Subtract(i64),
    Reset,
}

impl Counter {
    pub fn value(&self) -> i64 {
        self.value
    }
}

impl StateMachine for Counter {
    type Transition = CounterTransition;
    type Conflict = NeverConflict;

    fn apply(&self, event: &CounterTransition) -> Result<Self, NeverConflict> {
        let mut new_self = self.clone();
        match event {
            CounterTransition::Add(i) => new_self.value += i,
            CounterTransition::Subtract(i) => new_self.value -= i,
            CounterTransition::Reset => new_self.value = 0,
        }
        Ok(new_self)
    }
}
```

### Client Code

```rust
// client/src/lib.rs
use aper_yew::{
    StateProgramViewComponent,
    StateProgramViewContext,
    StateProgramComponent,
    StateProgramComponentProps,
    StateMachineContainerProgram,
};
use counter_common::{Counter, CounterTransition};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use yew::prelude::*;

// Define the view component
struct CounterView;

impl StateProgramViewComponent for CounterView {
    type Program = StateMachineContainerProgram<Counter>;

    fn view(
        state: Rc<Self::Program>,
        context: StateProgramViewContext<Self::Program>,
    ) -> Html {
        html! {
            <div>
                <h1>{"Counter Example"}</h1>
                <p class="counter-value">{format!("Value: {}", state.0.value())}</p>

                <div class="buttons">
                    <button
                        onclick={context.callback.reform(|_| CounterTransition::Add(1))}
                    >
                        {"+1"}
                    </button>

                    <button
                        onclick={context.callback.reform(|_| CounterTransition::Subtract(1))}
                    >
                        {"-1"}
                    </button>

                    <button
                        onclick={context.callback.reform(|_| CounterTransition::Reset)}
                    >
                        {"Reset"}
                    </button>
                </div>

                <p class="client-info">
                    {format!("Client ID: {:?}", context.client_id)}
                </p>
            </div>
        }
    }
}

// Entry point
#[wasm_bindgen(start)]
pub fn entry() {
    let props = StateProgramComponentProps::new("ws");
    yew::start_app_with_props::<StateProgramComponent<CounterView>>(props);
}
```

### HTML Template

```html
<!-- static/index.html -->
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Counter Example</title>
</head>
<body>
    <script type="module">
        import init from "/pkg/client.js";
        init();
    </script>
</body>
</html>
```

### CSS Styling

```css
/* static/style.css */
body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 100vh;
    margin: 0;
    background: #f5f5f5;
}

div {
    background: white;
    padding: 2rem;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    text-align: center;
}

.counter-value {
    font-size: 2rem;
    font-weight: bold;
    color: #333;
}

.buttons {
    display: flex;
    gap: 1rem;
    justify-content: center;
    margin: 1rem 0;
}

button {
    padding: 0.5rem 1rem;
    font-size: 1.2rem;
    cursor: pointer;
    border: 1px solid #ddd;
    border-radius: 4px;
    background: #fff;
}

button:hover {
    background: #f0f0f0;
}

.client-info {
    font-size: 0.8rem;
    color: #888;
}
```

---

## Summary

| Component | Purpose |
|-----------|---------|
| StateProgramComponent | Main component with WebSocket |
| StateProgramViewComponent | Trait for rendering state |
| StateProgramViewContext | Callback and client info |
| SessionStorage | Connection token persistence |
| html! macro | JSX-like rendering |
| Callback::reform() | Event transformation |

---

## Next Steps

Continue to [rust-revision.md](rust-revision.md) for Rust implementation patterns.
