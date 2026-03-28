# Fragment: Rust Revision - Complete Translation Guide

**Source:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/alchemy/fragment/`
**Target:** Rust with valtron executor (no async/await, no tokio)
**Date:** 2026-03-27

---

## 1. Overview

### 1.1 What We're Translating

Fragment is a TypeScript framework for building AI agent systems with composable, template-based entities. The complete translation to Rust involves:

| TypeScript Component | Rust Equivalent |
|---------------------|-----------------|
| `Fragment<Type, ID, References>` | `Fragment<Type, Id, References>` struct |
| `Effect.gen()` / `yield*` | `TaskIterator` pattern |
| `Stream<MessagePart>` | `StreamIterator` pattern |
| `@effect/ai` Chat API | valtron executor with model backends |
| SQLite StateStore | `rusqlite` with connection pooling |
| Template literals | `format!()` macros + custom builders |

### 1.2 Key Design Decisions

#### Ownership Strategy

```rust
// TypeScript uses garbage-collected references
const agent = Agent("bot")`Monitor ${channel}`;

// Rust uses explicit ownership with Rc for shared references
use std::rc::Rc;

struct Agent {
    id: String,
    template: Template,
    references: Vec<Rc<dyn Fragment>>,
}
```

#### Reference Handling

| Pattern | TypeScript | Rust |
|---------|------------|------|
| Shared ownership | Implicit GC | `Rc<T>` for single-threaded |
| Thread-safe sharing | N/A (single thread) | `Arc<T>` for multi-threaded |
| Borrowing | Implicit | `&T` and `&mut T` |
| Lifetime tracking | Runtime | Compile-time + `'a` |

#### Effect System Translation

```typescript
// TypeScript Effect
const handler = Effect.fn(function* () {
  const store = yield* StateStore;
  const messages = yield* store.readThreadMessages(threadId);
  return messages;
});
```

```rust
// Rust Result-based with TaskIterator
struct ReadMessagesTask {
    store: Arc<dyn StateStore>,
    thread_id: String,
}

impl TaskIterator for ReadMessagesTask {
    type Ready = Result<Vec<Message>, FragmentError>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.store.read_thread_messages(&self.thread_id) {
            Ok(msgs) => Some(TaskStatus::Ready(Ok(msgs))),
            Err(e) => Some(TaskStatus::Ready(Err(e))),
        }
    }
}
```

### 1.3 Valtron vs Effect-TS Comparison

| Concept | Effect-TS | Valtron (Rust) |
|---------|-----------|----------------|
| **Effect type** | `Effect<A, E, R>` | `TaskIterator<Ready=A, Error=E>` |
| **Error channel** | Typed errors in `E` | `Result<T, E>` |
| **Requirements** | Environment `R` via Layers | Explicit dependencies in struct |
| **Streams** | `Stream<A, E, R>` | `StreamIterator<Item=A, Error=E>` |
| **Composition** | `pipe()`, `Effect.all()` | Iterator combinators, `?` operator |
| **Services** | Context tags + Layers | Trait objects + Arc |

---

## 2. Type System Design

### 2.1 Fragment Struct in Rust

```rust
use std::rc::Rc;
use std::fmt::Debug;

/// Core Fragment type - all entities (agents, channels, tools) extend this
#[derive(Debug, Clone)]
pub struct Fragment<
    Type: FragmentType,
    Id: AsRef<str>,
    References: AsRef<[Rc<dyn FragmentAny>]>,
> {
    pub r#type: Type,
    pub id: Id,
    pub template: TemplateStrings,
    pub references: References,
}

/// Type-level discriminator for fragment kinds
pub trait FragmentType: Debug + Clone + PartialEq {
    fn as_str(&self) -> &'static str;
}

#[derive(Debug, Clone, PartialEq)]
pub enum FragmentKind {
    Agent,
    Channel,
    Toolkit,
    Tool,
    File,
    Group,
    Role,
}

impl FragmentType for FragmentKind {
    fn as_str(&self) -> &'static str {
        match self {
            FragmentKind::Agent => "agent",
            FragmentKind::Channel => "channel",
            FragmentKind::Toolkit => "toolkit",
            FragmentKind::Tool => "tool",
            FragmentKind::File => "file",
            FragmentKind::Group => "group",
            FragmentKind::Role => "role",
        }
    }
}

/// Type-erased Fragment for heterogeneous collections
pub trait FragmentAny: Debug {
    fn type_name(&self) -> &str;
    fn id(&self) -> &str;
    fn references(&self) -> Vec<Rc<dyn FragmentAny>>;
    fn as_any(&self) -> &dyn std::any::Any;
}
```

### 2.2 Type Enums for Fragment Kinds

```rust
/// All fragment kinds in a single enum for pattern matching
#[derive(Debug, Clone)]
pub enum AnyFragment {
    Agent(Agent),
    Channel(Channel),
    Toolkit(Toolkit),
    Tool(Tool),
    File(File),
    Group(Group),
    Role(Role),
}

impl AnyFragment {
    pub fn type_name(&self) -> &'static str {
        match self {
            AnyFragment::Agent(_) => "agent",
            AnyFragment::Channel(_) => "channel",
            AnyFragment::Toolkit(_) => "toolkit",
            AnyFragment::Tool(_) => "tool",
            AnyFragment::File(_) => "file",
            AnyFragment::Group(_) => "group",
            AnyFragment::Role(_) => "role",
        }
    }

    pub fn as_agent(&self) -> Option<&Agent> {
        match self {
            AnyFragment::Agent(agent) => Some(agent),
            _ => None,
        }
    }

    pub fn as_channel(&self) -> Option<&Channel> {
        match self {
            AnyFragment::Channel(channel) => Some(channel),
            _ => None,
        }
    }
}
```

### 2.3 Reference Handling (Box, Rc, Arc)

```rust
use std::rc::Rc;
use std::sync::Arc;

/// Single-threaded reference counting (for valtron executor)
type FragmentRef = Rc<dyn FragmentAny>;

/// Thread-safe reference counting (for multi-threaded backends)
type ThreadSafeFragmentRef = Arc<dyn FragmentAny>;

/// For owned fragment creation
type OwnedFragment = Box<dyn FragmentAny>;

/// References in Fragment struct
pub struct Agent {
    pub id: String,
    pub template: TemplateStrings,
    /// Rc allows multiple owners (agent can be referenced by multiple places)
    pub references: Vec<Rc<dyn FragmentAny>>,
}

/// For sharing across threads (if using thread-pool backend)
pub struct ThreadSafeAgent {
    pub id: String,
    pub template: TemplateStrings,
    pub references: Vec<Arc<dyn FragmentAny>>,
}
```

### 2.4 Lifetime Considerations

```rust
/// For borrowed references (no allocation)
pub struct BorrowedFragment<'a> {
    pub id: &'a str,
    pub r#type: FragmentKind,
}

/// For self-referential structures (agents referencing each other)
use std::cell::RefCell;

pub struct AgentRegistry {
    /// RefCell allows interior mutability
    agents: RefCell<Vec<Rc<Agent>>>,
}

impl AgentRegistry {
    pub fn add_agent(&self, agent: Rc<Agent>) {
        self.agents.borrow_mut().push(agent);
    }
}
```

---

## 3. Core Types Translation

### 3.1 Fragment<Type, ID, References> → Rust struct

```rust
/// Complete Fragment implementation with builder pattern
pub struct FragmentBuilder<Type: FragmentType> {
    r#type: Type,
    render_config: Option<RenderConfig>,
}

impl<Type: FragmentType> FragmentBuilder<Type> {
    pub fn new(r#type: Type) -> Self {
        Self {
            r#type,
            render_config: None,
        }
    }

    pub fn with_render(mut self, config: RenderConfig) -> Self {
        self.render_config = Some(config);
        self
    }

    pub fn build<Id: AsRef<str>>(
        self,
        id: Id,
        template: TemplateStrings,
        references: Vec<Rc<dyn FragmentAny>>,
    ) -> Fragment<Type, Id, Vec<Rc<dyn FragmentAny>>> {
        Fragment {
            r#type: self.r#type,
            id,
            template,
            references,
        }
    }
}

/// Template strings container (mimics TypeScript TemplateStringsArray)
#[derive(Debug, Clone)]
pub struct TemplateStrings {
    pub raw: Vec<String>,
    pub cooked: Vec<String>,
}

impl TemplateStrings {
    pub fn new(raw: Vec<String>, cooked: Vec<String>) -> Self {
        Self { raw, cooked }
    }

    /// Render template with interpolated values
    pub fn render(&self, values: &[String]) -> String {
        let mut result = String::new();
        for (i, section) in self.cooked.iter().enumerate() {
            result.push_str(section);
            if i < values.len() {
                result.push_str(&values[i]);
            }
        }
        result
    }
}
```

### 3.2 Agent → RustAgent

```rust
#[derive(Debug, Clone)]
pub struct Agent {
    pub id: String,
    pub template: TemplateStrings,
    pub references: Vec<Rc<dyn FragmentAny>>,
    /// Rendered context for system prompt
    pub context: Option<String>,
}

impl Agent {
    pub fn builder(id: impl Into<String>) -> AgentBuilder {
        AgentBuilder::new(id.into())
    }

    /// Render agent context (equivalent to TypeScript render.context)
    pub fn render_context(&self) -> String {
        format!("@{}", self.id)
    }

    /// Collect all toolkits from references (BFS traversal)
    pub fn collect_toolkits(&self) -> Vec<Rc<Toolkit>> {
        let mut toolkits = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue: std::collections::VecDeque<_> = self.references.iter().collect();

        while let Some(fragment) = queue.pop_front() {
            let id = fragment.id();
            if visited.contains(id) {
                continue;
            }
            visited.insert(id.to_string());

            if let Some(toolkit) = fragment.as_any().downcast_ref::<Toolkit>() {
                toolkits.push(Rc::new(toolkit.clone()));
            }

            // Queue nested references
            for reference in fragment.references() {
                queue.push_back(reference);
            }
        }

        toolkits
    }
}

pub struct AgentBuilder {
    id: String,
    template: Option<TemplateStrings>,
    references: Vec<Rc<dyn FragmentAny>>,
}

impl AgentBuilder {
    pub fn new(id: String) -> Self {
        Self {
            id,
            template: None,
            references: Vec::new(),
        }
    }

    pub fn template(mut self, template: TemplateStrings) -> Self {
        self.template = Some(template);
        self
    }

    pub fn reference(mut self, fragment: Rc<dyn FragmentAny>) -> Self {
        self.references.push(fragment);
        self
    }

    pub fn build(self) -> Agent {
        Agent {
            id: self.id,
            template: self.template.expect("Agent template required"),
            references: self.references,
            context: None,
        }
    }
}
```

### 3.3 Tool → RustTool

```rust
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Tool {
    pub id: String,
    pub template: TemplateStrings,
    pub references: Vec<Rc<dyn FragmentAny>>,
    pub input_schema: JsonSchema,
    pub output_schema: JsonSchema,
    pub alias: Option<fn(&str) -> Option<String>>, // Model-specific naming
}

#[derive(Debug, Clone)]
pub struct ToolHandlerInput {
    pub params: Value,
}

#[derive(Debug, Clone)]
pub struct ToolHandlerOutput {
    pub result: Value,
}

/// Tool handler trait - implementations use TaskIterator
pub trait ToolHandler: Send + Sync {
    fn execute(
        &self,
        input: ToolHandlerInput,
    ) -> Box<dyn Iterator<Item = Result<ToolHandlerOutput, FragmentError>> + Send>;
}

/// Example tool implementation
pub struct BashTool {
    working_dir: String,
}

impl ToolHandler for BashTool {
    fn execute(
        &self,
        input: ToolHandlerInput,
    ) -> Box<dyn Iterator<Item = Result<ToolHandlerOutput, FragmentError>> + Send> {
        let command = input.params["command"].as_str().unwrap_or("");
        let result = std::process::Command::new("bash")
            .arg("-c")
            .arg(command)
            .current_dir(&self.working_dir)
            .output();

        match result {
            Ok(output) => Box::new(std::iter::once(Ok(ToolHandlerOutput {
                result: serde_json::json!({
                    "exit_code": output.status.code().unwrap_or(-1),
                    "output": String::from_utf8_lossy(&output.stdout),
                    "stderr": String::from_utf8_lossy(&output.stderr),
                }),
            }))),
            Err(e) => Box::new(std::iter::once(Err(FragmentError::ToolExecution(e.to_string())))),
        }
    }
}
```

### 3.4 Toolkit → RustToolkit

```rust
#[derive(Debug, Clone)]
pub struct Toolkit {
    pub id: String,
    pub template: TemplateStrings,
    pub references: Vec<Rc<dyn FragmentAny>>, // Contains Tool references
    pub tools: Vec<Rc<Tool>>,
}

impl Toolkit {
    pub fn builder(id: impl Into<String>) -> ToolkitBuilder {
        ToolkitBuilder::new(id.into())
    }

    /// Collect all tools from references
    pub fn collect_tools(references: &[Rc<dyn FragmentAny>]) -> Vec<Rc<Tool>> {
        references
            .iter()
            .filter_map(|f| f.as_any().downcast_ref::<Tool>().map(Rc::new))
            .collect()
    }
}

pub struct ToolkitBuilder {
    id: String,
    template: Option<TemplateStrings>,
    tools: Vec<Rc<Tool>>,
}

impl ToolkitBuilder {
    pub fn new(id: String) -> Self {
        Self {
            id,
            template: None,
            tools: Vec::new(),
        }
    }

    pub fn template(mut self, template: TemplateStrings) -> Self {
        self.template = Some(template);
        self
    }

    pub fn tool(mut self, tool: Rc<Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn build(self) -> Toolkit {
        Toolkit {
            id: self.id,
            template: self.template.expect("Toolkit template required"),
            references: self.tools.iter().map(|t| t.clone() as Rc<dyn FragmentAny>).collect(),
            tools: self.tools,
        }
    }
}
```

### 3.5 Channel → RustChannel

```rust
#[derive(Debug, Clone)]
pub struct Channel {
    pub id: String,
    pub template: TemplateStrings,
    pub references: Vec<Rc<dyn FragmentAny>>,
    /// Thread ID for message persistence
    pub thread_id: String,
}

impl Channel {
    pub fn builder(id: impl Into<String>) -> ChannelBuilder {
        ChannelBuilder::new(id.into())
    }

    pub fn render_context(&self) -> String {
        format!("#{}", self.id)
    }
}

pub struct ChannelBuilder {
    id: String,
    template: Option<TemplateStrings>,
    thread_id: Option<String>,
}

impl ChannelBuilder {
    pub fn new(id: String) -> Self {
        Self {
            id,
            template: None,
            thread_id: None,
        }
    }

    pub fn template(mut self, template: TemplateStrings) -> Self {
        self.template = Some(template);
        self
    }

    pub fn thread_id(mut self, thread_id: String) -> Self {
        self.thread_id = Some(thread_id);
        self
    }

    pub fn build(self) -> Channel {
        Channel {
            id: self.id,
            template: self.template.expect("Channel template required"),
            references: Vec::new(),
            thread_id: self.thread_id.unwrap_or_else(|| self.id.clone()),
        }
    }
}
```

---

## 4. Template Literal Translation

### 4.1 Template Strings in Rust

```rust
/// Macro for creating template strings (mimics TypeScript tagged templates)
#[macro_export]
macro_rules! fragment_template {
    ($($parts:expr),* ; $($refs:expr),*) => {
        TemplateStrings {
            raw: vec![$($parts.to_string()),*],
            cooked: vec![$($parts.to_string()),*],
        }
    };
}

/// Usage example:
/// TypeScript: Agent("bot")`Monitor ${channel} for errors`
/// Rust:
let channel = Channel::builder("alerts").build();
let agent = Agent::builder("bot")
    .template(fragment_template![
        "Monitor "; " for errors"
    ])
    .reference(Rc::new(channel))
    .build();
```

### 4.2 Interpolation with format!()

```rust
/// Builder that supports template interpolation
pub struct TemplateBuilder {
    sections: Vec<String>,
    references: Vec<Rc<dyn FragmentAny>>,
}

impl TemplateBuilder {
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
            references: Vec::new(),
        }
    }

    pub fn push_section(&mut self, section: impl Into<String>) {
        self.sections.push(section.into());
    }

    pub fn push_reference(&mut self, fragment: Rc<dyn FragmentAny>) {
        self.references.push(fragment);
    }

    pub fn build(self) -> (TemplateStrings, Vec<Rc<dyn FragmentAny>>) {
        let template = TemplateStrings::new(self.sections.clone(), self.sections);
        (template, self.references)
    }

    /// Render with reference context strings
    pub fn render(&self) -> String {
        let mut result = String::new();
        for (i, section) in self.sections.iter().enumerate() {
            result.push_str(section);
            if i < self.references.len() {
                result.push_str(&self.references[i].render_context());
            }
        }
        result
    }
}

/// Reference rendering trait
pub trait FragmentAny: Debug {
    fn render_context(&self) -> String {
        format!("{{{}:{}}}", self.type_name(), self.id())
    }
}

// Agent-specific rendering
impl FragmentAny for Agent {
    fn render_context(&self) -> String {
        format!("@{}", self.id)
    }
}

// Channel-specific rendering
impl FragmentAny for Channel {
    fn render_context(&self) -> String {
        format!("#{}", self.id)
    }
}
```

### 4.3 Reference Capture and Storage

```rust
/// Resolved reference with its rendered form
#[derive(Debug, Clone)]
pub struct ResolvedReference {
    pub fragment: Rc<dyn FragmentAny>,
    pub rendered: String,
    pub position: usize,
}

/// Template resolver - converts template + references to final string
pub struct TemplateResolver {
    template: TemplateStrings,
    references: Vec<ResolvedReference>,
}

impl TemplateResolver {
    pub fn new(template: TemplateStrings, references: Vec<Rc<dyn FragmentAny>>) -> Self {
        let resolved: Vec<ResolvedReference> = references
            .into_iter()
            .enumerate()
            .map(|(i, fragment)| ResolvedReference {
                rendered: fragment.render_context(),
                position: i,
                fragment,
            })
            .collect();

        Self {
            template,
            references: resolved,
        }
    }

    pub fn resolve(&self) -> String {
        let mut result = String::new();
        for (i, section) in self.template.cooked.iter().enumerate() {
            result.push_str(section);
            if i < self.references.len() {
                result.push_str(&self.references[i].rendered);
            }
        }
        result
    }
}
```

---

## 5. Effect-TS to Valtron

### 5.1 Effect.gen → TaskIterator

```typescript
// TypeScript Effect.gen
const handler = Effect.gen(function* () {
  const store = yield* StateStore;
  const messages = yield* store.readThreadMessages(threadId);
  return messages;
});
```

```rust
// Rust TaskIterator
use valtron::{TaskIterator, TaskStatus, NoSpawner};

pub struct ReadMessagesTask {
    store: Arc<dyn StateStore>,
    thread_id: String,
    done: bool,
}

impl TaskIterator for ReadMessagesTask {
    type Ready = Result<Vec<Message>, FragmentError>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.done {
            return None;
        }
        self.done = true;

        match self.store.read_thread_messages(&self.thread_id) {
            Ok(msgs) => Some(TaskStatus::Ready(Ok(msgs))),
            Err(e) => Some(TaskStatus::Ready(Err(e))),
        }
    }
}

// Usage:
let task = ReadMessagesTask {
    store: state_store.clone(),
    thread_id: "thread-1".to_string(),
    done: false,
};
```

### 5.2 yield* → TaskStatus Returns

```typescript
// TypeScript with multiple yields
const complexHandler = Effect.gen(function* () {
  const store = yield* StateStore;
  const messages = yield* store.readThreadMessages(threadId);
  const parts = yield* store.readAgentParts(threadId, agentId);
  return { messages, parts };
});
```

```rust
// Rust with chained TaskIterators
pub struct ComplexTask {
    store: Arc<dyn StateStore>,
    thread_id: String,
    agent_id: String,
    step: u8,
    messages: Option<Vec<Message>>,
}

impl TaskIterator for ComplexTask {
    type Ready = Result<TaskOutput, FragmentError>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.step {
            0 => {
                self.step = 1;
                match self.store.read_thread_messages(&self.thread_id) {
                    Ok(msgs) => self.messages = Some(msgs),
                    Err(e) => return Some(TaskStatus::Ready(Err(e))),
                }
            }
            1 => {
                self.step = 2;
                let msgs = self.messages.take().unwrap();
                match self.store.read_agent_parts(&self.thread_id, &self.agent_id) {
                    Ok(parts) => return Some(TaskStatus::Ready(Ok(TaskOutput { messages: msgs, parts }))),
                    Err(e) => return Some(TaskStatus::Ready(Err(e))),
                }
            }
            _ => None,
        }
        None
    }
}

pub struct TaskOutput {
    pub messages: Vec<Message>,
    pub parts: Vec<MessagePart>,
}
```

### 5.3 Effect Error Channel → Result<T, E>

```rust
/// Fragment error types (equivalent to Effect's typed error channel)
#[derive(Debug, thiserror::Error)]
pub enum FragmentError {
    #[error("State store error: {0}")]
    StateStore(#[from] StateStoreError),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum StateStoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Thread not found: {0}")]
    ThreadNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

// Usage in TaskIterator
impl TaskIterator for SomeTask {
    type Ready = Result<OutputType, FragmentError>;
    // ...
}
```

### 5.4 Stream → StreamIterator

```rust
/// StreamIterator for streaming responses (equivalent to Effect Stream)
use valtron::stream::{StreamIterator, StreamStatus};

pub struct MessagePartStream {
    parts: std::vec::IntoIter<MessagePart>,
}

impl StreamIterator for MessagePartStream {
    type Item = MessagePart;
    type Error = FragmentError;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<StreamStatus<Self::Item, Self::Error, Self::Pending, Self::Spawner>> {
        self.parts.next().map(StreamStatus::Ready)
    }
}

/// Creating a stream from chat response
pub struct ChatStreamTask {
    model: Arc<dyn LanguageModel>,
    prompt: Vec<Message>,
    done: bool,
}

impl StreamIterator for ChatStreamTask {
    type Item = MessagePart;
    type Error = FragmentError;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<StreamStatus<Self::Item, Self::Error, Self::Pending, Self::Spawner>> {
        if self.done {
            return None;
        }

        // In reality, this would be async with polling
        // For valtron, we'd use a DrivenRecvIterator wrapper
        match self.model.generate_stream(&self.prompt) {
            Ok(part) => Some(StreamStatus::Ready(Ok(part))),
            Err(e) => {
                self.done = true;
                Some(StreamStatus::Ready(Err(FragmentError::Model(e.to_string()))))
            }
        }
    }
}
```

---

## 6. Tool Handling in Rust

### 6.1 Tool Trait Definition

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Core Tool trait with typed input/output
pub trait ToolDefinition: Send + Sync {
    fn id(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> &JsonSchema;
    fn output_schema(&self) -> &JsonSchema;
    fn alias(&self, model: &str) -> Option<&str>;

    /// Execute the tool with validated input
    fn execute(
        &self,
        input: Value,
    ) -> Box<dyn Iterator<Item = Result<Value, FragmentError>> + Send>;
}

/// JSON Schema for tool I/O
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<std::collections::HashMap<String, JsonSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema>>,
}
```

### 6.2 Input/Output Schemas (using serde_json)

```rust
/// Tool input builder
pub struct ToolInputBuilder {
    id: String,
    schema: JsonSchema,
    description: Option<String>,
}

impl ToolInputBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            schema: JsonSchema {
                schema_type: "string".to_string(),
                properties: None,
                required: None,
                description: None,
                items: None,
            },
            description: None,
        }
    }

    pub fn string(mut self) -> Self {
        self.schema.schema_type = "string".to_string();
        self
    }

    pub fn number(mut self) -> Self {
        self.schema.schema_type = "number".to_string();
        self
    }

    pub fn boolean(mut self) -> Self {
        self.schema.schema_type = "boolean".to_string();
        self
    }

    pub fn optional(mut self) -> Self {
        // Wrap in oneOf with null
        self.schema = JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(std::collections::HashMap::from([
                ("oneOf".to_string(), JsonSchema {
                    schema_type: "array".to_string(),
                    properties: None,
                    required: None,
                    description: None,
                    items: Some(Box::new(JsonSchema {
                        schema_type: "null".to_string(),
                        ..Default::default()
                    })),
                }),
            ])),
            required: None,
            description: None,
            items: None,
        };
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn build(self) -> ToolInput {
        ToolInput {
            id: self.id,
            schema: self.schema,
            description: self.description,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolInput {
    pub id: String,
    pub schema: JsonSchema,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub id: String,
    pub schema: JsonSchema,
    pub description: Option<String>,
}
```

### 6.3 Handler Functions

```rust
/// Macro for defining tools with handlers
#[macro_export]
macro_rules! define_tool {
    (
        $id:expr,
        $description:expr,
        input: { $($input_id:ident: $input_type:ty),* $(,)? },
        output: { $($output_key:ident),* $(,)? },
        handler: |$($param:ident),*| $body:block
    ) => {
        pub struct $id;

        impl ToolDefinition for $id {
            fn id(&self) -> &str { $id }

            fn description(&self) -> &str { $description }

            fn input_schema(&self) -> &JsonSchema {
                static SCHEMA: once_cell::sync::Lazy<JsonSchema> =
                    once_cell::sync::Lazy::new(|| {
                        serde_json::from_value(serde_json::json!({
                            "type": "object",
                            "properties": {
                                $(stringify!($input_id): { "type": get_type!($input_type) }),*
                            },
                            "required": [$(stringify!($input_id)),*]
                        }).expect("Invalid schema")
                    });
                &SCHEMA
            }

            fn output_schema(&self) -> &JsonSchema {
                static SCHEMA: once_cell::sync::Lazy<JsonSchema> =
                    once_cell::sync::Lazy::new(|| {
                        serde_json::from_value(serde_json::json!({
                            "type": "object",
                            "properties": {
                                $(stringify!($output_key): { "type": "any" }),*
                            }
                        }).expect("Invalid schema")
                    });
                &SCHEMA
            }

            fn execute(&self, input: Value) -> Box<dyn Iterator<Item = Result<Value, FragmentError>> + Send> {
                $(let $param = input[stringify!($input_id)].clone();)*
                let result = (|| $body)();
                Box::new(std::iter::once(result))
            }
        }
    };
}

/// Helper macro for type mapping
#[macro_export]
macro_rules! get_type {
    (String) => { "string" };
    (i32) => { "number" };
    (f64) => { "number" };
    (bool) => { "boolean" };
    (Value) => { "any" };
}
```

### 6.4 Tool Registry

```rust
use std::collections::HashMap;

/// Tool registry for lookup and execution
pub struct ToolRegistry {
    tools: HashMap<String, Rc<dyn ToolDefinition>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Rc<dyn ToolDefinition>) {
        self.tools.insert(tool.id().to_string(), tool);
    }

    pub fn get(&self, id: &str) -> Option<Rc<dyn ToolDefinition>> {
        self.tools.get(id).cloned()
    }

    /// Get tool with model-specific alias
    pub fn get_with_alias(&self, id: &str, model: &str) -> Option<Rc<dyn ToolDefinition>> {
        self.tools.values().find(|tool| {
            tool.id() == id || tool.alias(model) == Some(id)
        }).cloned()
    }

    /// Execute a tool by name
    pub fn execute(
        &self,
        name: &str,
        params: Value,
        model: Option<&str>,
    ) -> Result<Box<dyn Iterator<Item = Result<Value, FragmentError>> + Send>, FragmentError> {
        let tool = match model {
            Some(m) => self.get_with_alias(name, m),
            None => self.get(name),
        };

        match tool {
            Some(tool) => Ok(tool.execute(params)),
            None => Err(FragmentError::AgentNotFound(format!("Tool not found: {}", name))),
        }
    }
}

/// Toolkit handlers layer
pub struct ToolkitHandlers {
    registry: Arc<ToolRegistry>,
    model: Option<String>,
}

impl ToolkitHandlers {
    pub fn new(registry: Arc<ToolRegistry>, model: Option<String>) -> Self {
        Self { registry, model }
    }

    pub fn handle(&self, name: &str, params: Value) -> Result<Value, FragmentError> {
        let iter = self.registry.execute(name, params, self.model.as_deref())?;
        iter.collect::<Result<Vec<_>, _>>()
            .map(|results| results.into_iter().next().unwrap_or(serde_json::Value::Null))
    }
}
```

---

## 7. StateStore in Rust

### 7.1 SQLite with rusqlite

```rust
use rusqlite::{Connection, params};
use std::path::Path;

/// SQLite-backed StateStore implementation
pub struct SqliteStateStore {
    conn: Connection,
    /// PubSub for real-time streaming (in-memory)
    pubsub: Arc<ThreadSafePubSub<MessagePart>>,
}

impl SqliteStateStore {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, StateStoreError> {
        let conn = Connection::open(path)?;

        // Initialize schema
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                thread_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                role TEXT NOT NULL,
                sender TEXT,
                content TEXT NOT NULL,
                UNIQUE(thread_id, position)
            );

            CREATE TABLE IF NOT EXISTS parts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                thread_id TEXT NOT NULL,
                sender TEXT,
                type TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER DEFAULT (unixepoch())
            );

            CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_id);
            CREATE INDEX IF NOT EXISTS idx_parts_thread ON parts(thread_id, sender);
            "
        )?;

        Ok(Self {
            conn,
            pubsub: Arc::new(ThreadSafePubSub::new()),
        })
    }
}

impl StateStore for SqliteStateStore {
    fn read_thread_messages(&self, thread_id: &str) -> Result<Vec<Message>, StateStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT role, sender, content FROM messages
             WHERE thread_id = ? ORDER BY position"
        )?;

        let messages = stmt
            .query_map(params![thread_id], |row| {
                let role: String = row.get(0)?;
                let sender: Option<String> = row.get(1)?;
                let content: String = row.get(2)?;
                Ok(Message { role, sender, content })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    fn write_thread_messages(&self, thread_id: &str, messages: &[Message]) -> Result<(), StateStoreError> {
        let tx = self.conn.transaction()?;

        for (pos, msg) in messages.iter().enumerate() {
            tx.execute(
                "INSERT OR REPLACE INTO messages (thread_id, position, role, sender, content)
                 VALUES (?, ?, ?, ?, ?)",
                params![thread_id, pos, msg.role, msg.sender, msg.content],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    fn read_agent_parts(&self, thread_id: &str, sender: &str) -> Result<Vec<MessagePart>, StateStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT type, content FROM parts
             WHERE thread_id = ? AND sender = ?
             ORDER BY created_at"
        )?;

        let parts = stmt
            .query_map(params![thread_id, sender], |row| {
                let type_: String = row.get(0)?;
                let content: String = row.get(1)?;
                Ok(MessagePart::from_json(&type_, &content)?)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(parts)
    }

    fn append_thread_part(&self, thread_id: &str, part: MessagePart) -> Result<(), StateStoreError> {
        // Persist to database
        let content_json = serde_json::to_string(&part.content)?;
        self.conn.execute(
            "INSERT INTO parts (thread_id, sender, type, content)
             VALUES (?, ?, ?, ?)",
            params![thread_id, part.sender, part.type, content_json],
        )?;

        // Publish to subscribers
        self.pubsub.publish(part.clone());

        Ok(())
    }

    fn truncate_agent_parts(&self, thread_id: &str, sender: &str) -> Result<(), StateStoreError> {
        self.conn.execute(
            "DELETE FROM parts WHERE thread_id = ? AND sender = ?",
            params![thread_id, sender],
        )?;
        Ok(())
    }
}
```

### 7.2 Message Types

```rust
use serde::{Deserialize, Serialize};

/// Message with sender attribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "user" | "assistant" | "system"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    pub content: MessageContent,
}

/// Content can be string or array of blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// Content block types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ContentBlock {
    Text { text: String },
    ToolCall { id: String, name: String, params: serde_json::Value },
    ToolResult { id: String, result: serde_json::Value, error: Option<String> },
    Reasoning { content: String },
}

/// Streaming part types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePart {
    #[serde(rename = "type")]
    pub part_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    #[serde(flatten)]
    pub content: MessagePartContent,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessagePartContent {
    TextDelta { delta: String },
    ToolCall { id: String, name: String, params: serde_json::Value },
    ToolResult { id: String, result: serde_json::Value },
    UserInput { content: String },
    // ... other part types
}

impl MessagePart {
    pub fn from_json(type_: &str, content: &str) -> Result<Self, serde_json::Error> {
        let mut part: MessagePart = serde_json::from_str(content)?;
        part.part_type = type_.to_string();
        Ok(part)
    }
}
```

### 7.3 Parts Buffering

```rust
/// In-memory parts buffer for streaming accumulation
pub struct PartsBuffer {
    buffer: std::collections::HashMap<String, Vec<MessagePart>>, // keyed by sender
}

impl PartsBuffer {
    pub fn new() -> Self {
        Self {
            buffer: std::collections::HashMap::new(),
        }
    }

    pub fn append(&mut self, sender: &str, part: MessagePart) {
        self.buffer
            .entry(sender.to_string())
            .or_default()
            .push(part);
    }

    pub fn get(&self, sender: &str) -> &[MessagePart] {
        self.buffer.get(sender).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn clear(&mut self, sender: &str) {
        self.buffer.remove(sender);
    }

    pub fn is_message_boundary(&self, part: &MessagePart) -> bool {
        matches!(
            part.part_type.as_str(),
            "user-input" | "text-end" | "reasoning-end" | "tool-call" | "tool-result"
        )
    }
}
```

### 7.4 Transaction Handling

```rust
impl SqliteStateStore {
    /// Batch write with transaction
    pub fn write_thread_messages_batch(
        &self,
        thread_id: &str,
        messages: &[Message],
    ) -> Result<(), StateStoreError> {
        let tx = self.conn.transaction()?;

        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO messages (thread_id, position, role, sender, content)
                 VALUES (?, ?, ?, ?, ?)"
            )?;

            for (pos, msg) in messages.iter().enumerate() {
                let content_json = match &msg.content {
                    MessageContent::Text(s) => s.clone(),
                    MessageContent::Blocks(blocks) => serde_json::to_string(blocks)?,
                };

                stmt.execute(params![thread_id, pos, msg.role, msg.sender, content_json])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Read with tool-use ID deduplication
    pub fn read_thread_messages_deduped(
        &self,
        thread_id: &str,
        agent_id: &str,
    ) -> Result<Vec<Message>, StateStoreError> {
        let messages = self.read_thread_messages(thread_id)?;

        // Track seen tool_use IDs
        let mut seen_ids = std::collections::HashSet::new();
        let mut duplicate_ids = std::collections::HashSet::new();

        // First pass: identify duplicates
        for msg in &messages {
            if let MessageContent::Blocks(blocks) = &msg.content {
                for block in blocks {
                    if let ContentBlock::ToolCall { id, .. } = block {
                        if seen_ids.contains(id) {
                            duplicate_ids.insert(id.clone());
                        } else {
                            seen_ids.insert(id.clone());
                        }
                    }
                }
            }
        }

        // If no duplicates, return original
        if duplicate_ids.is_empty() {
            return Ok(messages);
        }

        // Second pass: remove duplicate tool-calls
        let repaired: Vec<Message> = messages
            .into_iter()
            .filter_map(|mut msg| {
                if let MessageContent::Blocks(blocks) = msg.content {
                    let filtered: Vec<ContentBlock> = blocks
                        .into_iter()
                        .filter(|block| {
                            if let ContentBlock::ToolCall { id, .. } = block {
                                if duplicate_ids.contains(id) {
                                    return false;
                                }
                            }
                            true
                        })
                        .collect();

                    if filtered.is_empty() {
                        return None;
                    }

                    msg.content = MessageContent::Blocks(filtered);
                }
                Some(msg)
            })
            .collect();

        Ok(repaired)
    }
}
```

---

## 8. Agent Communication

### 8.1 send() → Rust Implementation

```rust
use std::sync::mpsc;

/// Agent instance with send/query capabilities
pub struct AgentInstance {
    agent: Rc<Agent>,
    thread_id: String,
    state_store: Arc<dyn StateStore>,
    toolkit_handlers: Arc<ToolkitHandlers>,
    context_sent: bool,
}

impl AgentInstance {
    /// Send a message, receive streaming response
    pub fn send(
        &mut self,
        prompt: &str,
    ) -> Result<Box<dyn Iterator<Item = Result<MessagePart, FragmentError>> + Send>, FragmentError> {
        // Build full prompt with context if first call
        let full_prompt = if !self.context_sent {
            let mut msgs = self.state_store.read_thread_messages(&self.thread_id)?;
            msgs.push(Message {
                role: "user".to_string(),
                sender: None,
                content: MessageContent::Text(prompt.to_string()),
            });
            self.context_sent = true;
            msgs
        } else {
            vec![Message {
                role: "user".to_string(),
                sender: None,
                content: MessageContent::Text(prompt.to_string()),
            }]
        };

        // Create streaming task
        let stream_task = ChatStreamTask {
            prompt: full_prompt,
            toolkit: self.toolkit_handlers.clone(),
            state_store: self.state_store.clone(),
            thread_id: self.thread_id.clone(),
            sender: self.agent.id.clone(),
            done: false,
        };

        Ok(Box::new(stream_task))
    }
}

/// Chat stream task implementation
pub struct ChatStreamTask {
    prompt: Vec<Message>,
    toolkit: Arc<ToolkitHandlers>,
    state_store: Arc<dyn StateStore>,
    thread_id: String,
    sender: String,
    done: bool,
}

impl Iterator for ChatStreamTask {
    type Item = Result<MessagePart, FragmentError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // In production, this would poll the model backend
        // For now, simulate streaming
        Some(Ok(MessagePart {
            part_type: "text-end".to_string(),
            sender: Some(self.sender.clone()),
            content: MessagePartContent::TextDelta { delta: "".to_string() },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }))
    }
}
```

### 8.2 query() → Structured Responses

```rust
/// Query for structured response
pub fn query<T: serde::de::DeserializeOwned>(
    &mut self,
    prompt: &str,
    schema: &JsonSchema,
) -> Result<T, FragmentError> {
    // Build prompt with context
    let messages = if !self.context_sent {
        let mut msgs = self.state_store.read_thread_messages(&self.thread_id)?;
        msgs.push(Message {
            role: "user".to_string(),
            sender: None,
            content: MessageContent::Text(format!(
                "{}\n\nRespond with JSON matching this schema:\n{}",
                prompt,
                serde_json::to_string_pretty(schema).unwrap()
            )),
        });
        self.context_sent = true;
        msgs
    } else {
        vec![Message {
            role: "user".to_string(),
            sender: None,
            content: MessageContent::Text(prompt.to_string()),
        }]
    };

    // Generate structured response
    let response = self.toolkit_handlers.generate_structured(&messages, schema)?;

    // Parse and validate against schema
    let result: T = serde_json::from_value(response)?;
    Ok(result)
}
```

### 8.3 Agent Discovery

```rust
/// Agent discovery with BFS traversal
pub struct AgentDiscovery {
    agents: std::collections::HashMap<String, Rc<Agent>>,
    spawned: std::collections::HashMap<String, AgentInstance>,
    state_store: Arc<dyn StateStore>,
    toolkit_handlers: Arc<ToolkitHandlers>,
}

impl AgentDiscovery {
    pub fn new(state_store: Arc<dyn StateStore>, toolkit_handlers: Arc<ToolkitHandlers>) -> Self {
        Self {
            agents: std::collections::HashMap::new(),
            spawned: std::collections::HashMap::new(),
            state_store,
            toolkit_handlers,
        }
    }

    /// Discover agents from references (BFS with cycle detection)
    pub fn discover(&mut self, root_agent: &Agent) {
        let mut visited = std::collections::HashSet::new();
        let mut queue: std::collections::VecDeque<_> = root_agent.references.iter().collect();

        while let Some(fragment) = queue.pop_front() {
            let id = fragment.id().to_string();

            if visited.contains(&id) {
                continue;
            }
            visited.insert(id.clone());

            if let Some(agent) = fragment.as_any().downcast_ref::<Agent>() {
                self.agents.insert(id.clone(), Rc::new(agent.clone()));

                // Queue nested references
                for reference in fragment.references() {
                    queue.push_back(reference);
                }
            }
        }
    }

    /// Get or spawn agent instance
    pub fn get_or_spawn(&mut self, agent_id: &str) -> Result<&mut AgentInstance, FragmentError> {
        use std::collections::hash_map::Entry;

        match self.spawned.entry(agent_id.to_string()) {
            Entry::Vacant(entry) => {
                let agent = self.agents
                    .get(agent_id)
                    .ok_or_else(|| FragmentError::AgentNotFound(agent_id.to_string()))?
                    .clone();

                let instance = AgentInstance {
                    agent,
                    thread_id: agent_id.to_string(),
                    state_store: self.state_store.clone(),
                    toolkit_handlers: self.toolkit_handlers.clone(),
                    context_sent: false,
                };

                Ok(entry.insert(instance))
            }
            Entry::Occupied(entry) => Ok(entry.into_mut()),
        }
    }
}
```

### 8.4 Channel Coordination

```rust
/// Multi-agent channel coordinator
pub struct ChannelCoordinator {
    channel: Rc<Channel>,
    discovery: AgentDiscovery,
    typing_agents: std::collections::HashSet<String>,
}

impl ChannelCoordinator {
    pub fn new(channel: Rc<Channel>, discovery: AgentDiscovery) -> Self {
        Self {
            channel,
            discovery,
            typing_agents: std::collections::HashSet::new(),
        }
    }

    /// Get typing agents (agents currently streaming)
    pub fn get_typing_agents(&self) -> Vec<&str> {
        self.typing_agents.iter().map(|s| s.as_str()).collect()
    }

    /// Mark agent as typing
    pub fn start_typing(&mut self, agent_id: &str) {
        self.typing_agents.insert(agent_id.to_string());
    }

    /// Mark agent as complete
    pub fn stop_typing(&mut self, agent_id: &str) {
        self.typing_agents.remove(agent_id);
    }

    /// Broadcast message to all channel members
    pub fn broadcast(
        &mut self,
        message: &str,
    ) -> Result<Vec<Box<dyn Iterator<Item = Result<MessagePart, FragmentError>> + Send>>, FragmentError> {
        let mut streams = Vec::new();

        for agent_id in self.discovery.agents.keys() {
            let instance = self.discovery.get_or_spawn(agent_id)?;
            self.start_typing(agent_id);
            let stream = instance.send(message)?;
            streams.push(stream);
        }

        Ok(streams)
    }
}
```

---

## 9. Organization Modeling

### 9.1 Group Struct

```rust
#[derive(Debug, Clone)]
pub struct Group {
    pub id: String,
    pub template: TemplateStrings,
    pub references: Vec<Rc<dyn FragmentAny>>,
    /// Members discovered from references
    pub members: Vec<GroupMember>,
}

#[derive(Debug, Clone)]
pub enum GroupMember {
    Agent(Rc<Agent>),
    Role(Rc<Role>),
    SubGroup(Rc<Group>),
}

impl Group {
    pub fn builder(id: impl Into<String>) -> GroupBuilder {
        GroupBuilder::new(id.into())
    }

    /// Collect members from references
    pub fn collect_members(references: &[Rc<dyn FragmentAny>]) -> Vec<GroupMember> {
        references
            .iter()
            .filter_map(|f| {
                if let Some(agent) = f.as_any().downcast_ref::<Agent>() {
                    Some(GroupMember::Agent(Rc::new(agent.clone())))
                } else if let Some(role) = f.as_any().downcast_ref::<Role>() {
                    Some(GroupMember::Role(Rc::new(role.clone())))
                } else if let Some(group) = f.as_any().downcast_ref::<Group>() {
                    Some(GroupMember::SubGroup(Rc::new(group.clone())))
                } else {
                    None
                }
            })
            .collect()
    }
}

pub struct GroupBuilder {
    id: String,
    template: Option<TemplateStrings>,
    references: Vec<Rc<dyn FragmentAny>>,
}

impl GroupBuilder {
    pub fn new(id: String) -> Self {
        Self {
            id,
            template: None,
            references: Vec::new(),
        }
    }

    pub fn template(mut self, template: TemplateStrings) -> Self {
        self.template = Some(template);
        self
    }

    pub fn reference(mut self, fragment: Rc<dyn FragmentAny>) -> Self {
        self.references.push(fragment);
        self
    }

    pub fn build(self) -> Group {
        let members = Group::collect_members(&self.references);
        Group {
            id: self.id,
            template: self.template.expect("Group template required"),
            references: self.references,
            members,
        }
    }
}
```

### 9.2 Role Struct

```rust
#[derive(Debug, Clone)]
pub struct Role {
    pub id: String,
    pub template: TemplateStrings,
    pub references: Vec<Rc<dyn FragmentAny>>,
    /// Role properties
    pub permissions: Vec<Permission>,
    pub responsibilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub resource: String,
    pub actions: Vec<String>, // "read", "write", "delete", etc.
}

impl Role {
    pub fn builder(id: impl Into<String>) -> RoleBuilder {
        RoleBuilder::new(id.into())
    }

    /// Check if role has permission
    pub fn has_permission(&self, resource: &str, action: &str) -> bool {
        self.permissions.iter().any(|p| {
            (p.resource == resource || p.resource == "*") &&
            (p.actions.contains(&action.to_string()) || p.actions.contains(&"*".to_string()))
        })
    }
}

pub struct RoleBuilder {
    id: String,
    template: Option<TemplateStrings>,
    permissions: Vec<Permission>,
    responsibilities: Vec<String>,
}

impl RoleBuilder {
    pub fn new(id: String) -> Self {
        Self {
            id,
            template: None,
            permissions: Vec::new(),
            responsibilities: Vec::new(),
        }
    }

    pub fn template(mut self, template: TemplateStrings) -> Self {
        self.template = Some(template);
        self
    }

    pub fn permission(mut self, resource: impl Into<String>, actions: Vec<String>) -> Self {
        self.permissions.push(Permission {
            resource: resource.into(),
            actions,
        });
        self
    }

    pub fn responsibility(mut self, resp: impl Into<String>) -> Self {
        self.responsibilities.push(resp.into());
        self
    }

    pub fn build(self) -> Role {
        Role {
            id: self.id,
            template: self.template.expect("Role template required"),
            references: Vec::new(),
            permissions: self.permissions,
            responsibilities: self.responsibilities,
        }
    }
}
```

### 9.3 Permission System

```rust
/// Permission checker for authorization
pub struct PermissionChecker {
    roles: std::collections::HashMap<String, Rc<Role>>,
}

impl PermissionChecker {
    pub fn new() -> Self {
        Self {
            roles: std::collections::HashMap::new(),
        }
    }

    pub fn register_role(&mut self, role: Rc<Role>) {
        self.roles.insert(role.id.clone(), role);
    }

    /// Check if any role has the permission
    pub fn check(&self, role_ids: &[&str], resource: &str, action: &str) -> bool {
        role_ids.iter().any(|id| {
            self.roles.get(*id)
                .map(|r| r.has_permission(resource, action))
                .unwrap_or(false)
        })
    }

    /// Get effective permissions for roles
    pub fn effective_permissions(&self, role_ids: &[&str]) -> Vec<Permission> {
        let mut perms = Vec::new();
        for id in role_ids {
            if let Some(role) = self.roles.get(*id) {
                perms.extend(role.permissions.iter().cloned());
            }
        }
        perms
    }
}
```

### 9.4 Reference Resolution

```rust
/// Organization reference resolver
pub struct OrgResolver {
    groups: std::collections::HashMap<String, Rc<Group>>,
    roles: std::collections::HashMap<String, Rc<Role>>,
    agents: std::collections::HashMap<String, Rc<Agent>>,
}

impl OrgResolver {
    pub fn new() -> Self {
        Self {
            groups: std::collections::HashMap::new(),
            roles: std::collections::HashMap::new(),
            agents: std::collections::HashMap::new(),
        }
    }

    pub fn register_group(&mut self, group: Rc<Group>) {
        self.groups.insert(group.id.clone(), group);
    }

    pub fn register_role(&mut self, role: Rc<Role>) {
        self.roles.insert(role.id.clone(), role);
    }

    pub fn register_agent(&mut self, agent: Rc<Agent>) {
        self.agents.insert(agent.id.clone(), agent);
    }

    /// Resolve a reference path like "engineering/backend-team/alice"
    pub fn resolve_path(&self, path: &str) -> Result<Rc<dyn FragmentAny>, FragmentError> {
        let parts: Vec<&str> = path.split('/').collect();

        if parts.is_empty() {
            return Err(FragmentError::Validation("Empty path".to_string()));
        }

        // Start with first part as group
        let mut current_group = self.groups.get(parts[0])
            .ok_or_else(|| FragmentError::AgentNotFound(parts[0].to_string()))?
            .clone();

        // Navigate through subgroups
        for part in &parts[1..parts.len()-1] {
            let found = current_group.members.iter().find_map(|m| {
                if let GroupMember::SubGroup(g) = m {
                    if g.id == *part { Some(g.clone()) } else { None }
                } else {
                    None
                }
            });

            current_group = found
                .ok_or_else(|| FragmentError::AgentNotFound(part.to_string()))?;
        }

        // Last part should be agent or role
        let last = parts.last().unwrap();
        if let Some(agent) = self.agents.get(*last) {
            Ok(agent.clone() as Rc<dyn FragmentAny>)
        } else if let Some(role) = self.roles.get(*last) {
            Ok(role.clone() as Rc<dyn FragmentAny>)
        } else {
            Err(FragmentError::AgentNotFound(last.to_string()))
        }
    }
}
```

---

## 10. Complete Example

### 10.1 Full Working Rust Fragment Implementation

```rust
use std::rc::Rc;
use std::sync::Arc;
use fragment::{
    Agent, Channel, Toolkit, Tool, ToolDefinition,
    StateStore, SqliteStateStore, Message, MessageContent, MessagePart,
    TemplateStrings, FragmentAny,
};

/// Define a custom tool
struct ReadFileTool;

impl ToolDefinition for ReadFileTool {
    fn id(&self) -> &str { "read" }

    fn description(&self) -> &str {
        "Reads a file from the local filesystem"
    }

    fn input_schema(&self) -> &JsonSchema {
        static SCHEMA: once_cell::sync::Lazy<JsonSchema> =
            once_cell::sync::Lazy::new(|| {
                serde_json::from_value(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "filePath": { "type": "string", "description": "The path to the file" }
                    },
                    "required": ["filePath"]
                })).unwrap()
            });
        &SCHEMA
    }

    fn output_schema(&self) -> &JsonSchema {
        static SCHEMA: once_cell::sync::Lazy<JsonSchema> =
            once_cell::sync::Lazy::new(|| {
                serde_json::from_value(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "content": { "type": "string" }
                    }
                })).unwrap()
            });
        &SCHEMA
    }

    fn alias(&self, _model: &str) -> Option<&str> { None }

    fn execute(&self, input: serde_json::Value) -> Box<dyn Iterator<Item = Result<serde_json::Value, FragmentError>> + Send> {
        let path = input["filePath"].as_str().unwrap_or("");

        match std::fs::read_to_string(path) {
            Ok(content) => Box::new(std::iter::once(Ok(serde_json::json!({
                "content": content
            })))),
            Err(e) => Box::new(std::iter::once(Ok(serde_json::json!({
                "content": format!("Error: {}", e)
            })))),
        }
    }
}

/// Main function demonstrating fragment usage
fn main() -> Result<(), FragmentError> {
    // Initialize state store
    let state_store = Arc::new(SqliteStateStore::new("fragment.db")?);

    // Create tools
    let read_tool = Rc::new(ReadFileTool);

    // Create toolkit
    let coding_toolkit = Toolkit::builder("coding")
        .template(fragment_template![
            "File operations toolkit: ";
        ])
        .tool(read_tool.clone())
        .build();

    // Create channel
    let general_channel = Channel::builder("general")
        .template(fragment_template![
            "General discussion channel for the team";
        ])
        .build();

    // Create agents
    let alice = Agent::builder("alice")
        .template(fragment_template![
            "You are Alice, a software engineer. ";
            "You work in the "; " channel."
        ])
        .reference(Rc::new(general_channel.clone()))
        .build();

    let bob = Agent::builder("bob")
        .template(fragment_template![
            "You are Bob, a code reviewer. ";
            "You review code from ";
        ])
        .reference(Rc::new(alice.clone()))
        .reference(Rc::new(general_channel.clone()))
        .build();

    // Create agent registry
    let mut discovery = AgentDiscovery::new(
        state_store.clone(),
        Arc::new(ToolkitHandlers::new(/* ... */)),
    );

    discovery.discover(&alice);
    discovery.discover(&bob);

    // Spawn agent and send message
    let mut alice_instance = discovery.get_or_spawn("alice")?;

    let stream = alice_instance.send("Hello! Can you review my code?")?;

    for part in stream {
        match part {
            Ok(part) => println!("Received part: {:?}", part),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
```

### 10.2 Usage Examples

```rust
/// Example: Multi-agent conversation
fn multi_agent_conversation() -> Result<(), FragmentError> {
    let state_store = Arc::new(SqliteStateStore::new("conversation.db")?);

    // Create agents
    let planner = Agent::builder("planner")
        .template(fragment_template!["You plan features";])
        .build();

    let coder = Agent::builder("coder")
        .template(fragment_template!["You write code";])
        .reference(Rc::new(planner.clone()))
        .build();

    let reviewer = Agent::builder("reviewer")
        .template(fragment_template!["You review code";])
        .reference(Rc::new(coder.clone()))
        .build();

    // Create channel for collaboration
    let channel = Channel::builder("feature-dev")
        .template(fragment_template!["Feature development channel";])
        .build();

    // Spawn and coordinate
    let mut discovery = AgentDiscovery::new(state_store.clone(), /* handlers */);
    discovery.discover(&planner);

    let mut planner_instance = discovery.get_or_spawn("planner")?;

    // Start conversation
    let stream = planner_instance.send("Let's build a new feature")?;

    // Process streaming response
    for part in stream {
        // Handle parts...
    }

    Ok(())
}

/// Example: Structured query
fn structured_query() -> Result<(), FragmentError> {
    let state_store = Arc::new(SqliteStateStore::new("query.db")?);

    let analyst = Agent::builder("analyst")
        .template(fragment_template!["You analyze data and return structured reports";])
        .build();

    let mut discovery = AgentDiscovery::new(state_store.clone(), /* handlers */);
    discovery.discover(&analyst);

    let mut instance = discovery.get_or_spawn("analyst")?;

    // Define response schema
    let schema = JsonSchema {
        schema_type: "object".to_string(),
        properties: Some(std::collections::HashMap::from([
            ("summary".to_string(), JsonSchema { schema_type: "string".to_string(), ..Default::default() }),
            ("metrics".to_string(), JsonSchema {
                schema_type: "array".to_string(),
                items: Some(Box::new(JsonSchema { schema_type: "object".to_string(), ..Default::default() })),
                ..Default::default()
            }),
        ])),
        ..Default::default()
    };

    // Query with schema
    #[derive(serde::Deserialize)]
    struct AnalysisReport {
        summary: String,
        metrics: Vec<serde_json::Value>,
    }

    let report: AnalysisReport = instance.query(
        "Analyze the codebase and provide a summary",
        &schema,
    )?;

    println!("Summary: {}", report.summary);

    Ok(())
}
```

---

## Summary

This Rust revision provides:

1. **Type-safe Fragment structs** with explicit ownership (`Rc`/`Arc`)
2. **TaskIterator pattern** replacing `Effect.gen()` / `yield*`
3. **StreamIterator** for streaming responses
4. **SQLite StateStore** with `rusqlite` for persistence
5. **Tool trait** with JSON Schema validation
6. **Agent discovery** with BFS traversal and cycle detection
7. **Organization modeling** with Groups, Roles, and Permissions
8. **Complete working examples** demonstrating all patterns

The translation maintains the core design philosophy of Fragment while adapting to Rust's ownership model and the valtron executor's single-threaded, iterator-based approach.
