---
title: "Zero Rust Revision: Complete Translation Guide"
subtitle: "How to replicate Zero's IVM engine and sync architecture in Rust for ewe_platform"
---

# Zero Rust Revision

## 1. Overview

This document provides a complete guide for translating Zero's TypeScript implementation to Rust, targeting the ewe_platform with valtron executor.

### Key Design Decisions

| Aspect | TypeScript (Zero) | Rust (ewe_platform) |
|--------|------------------|---------------------|
| Async Model | async/await, Promise | TaskIterator (no async/await) |
| Runtime | Node.js | valtron executor |
| Memory Management | GC | Ownership + Arena allocation |
| Error Handling | try/catch, Result types | Result<T, E> with thiserror |
| Collections | Array, Map, Set | Vec, HashMap, HashSet |
| Streams | Iterable, AsyncIterable | Iterator + valtron streams |

## 2. Type System Translation

### 2.1 Core Types

```typescript
// TypeScript: Change types
type Change = AddChange | RemoveChange | EditChange | ChildChange;

interface AddChange {
  type: 'add';
  relation: string;
  node: Node;
}

interface Node {
  row: Row;
  relationships: Record<string, () => Stream<Node | 'yield'>>;
}
```

```rust
// Rust: Change types with tagged enum
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Change {
    Add(AddChange),
    Remove(RemoveChange),
    Edit(EditChange),
    Child(ChildChange),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddChange {
    pub relation: String,
    pub node: Node,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveChange {
    pub relation: String,
    pub node: Node,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditChange {
    pub relation: String,
    pub node: Node,
    pub old_node: Node,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildChange {
    pub relation: String,
    pub node: Node,
    pub child: Box<ChildChangeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildChangeInfo {
    pub relationship_name: String,
    pub change: Change,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub row: Row,
    pub relationships: HashMap<String, RelationshipStream>,
}

pub type Row = HashMap<String, Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}
```

### 2.2 AST Types

```typescript
// TypeScript: Query AST
interface AST {
  type: 'select';
  table: string;
  where?: Condition;
  orderBy?: Ordering;
  limit?: number;
}
```

```rust
// Rust: Query AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ast {
    pub r#type: SelectType,
    pub table: String,
    pub where_clause: Option<Condition>,
    pub order_by: Option<Ordering>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    Simple(SimpleCondition),
    Conjunction(CompoundCondition),
    Disjunction(CompoundCondition),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleCondition {
    pub left: ValueExpression,
    pub operator: SimpleOperator,
    pub right: ValueExpression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimpleOperator {
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
    Like,
    In,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ordering {
    pub columns: Vec<OrderPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPart {
    pub column: String,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderDirection {
    Asc,
    Desc,
}
```

## 3. Operator Translation

### 3.1 Operator Trait

```typescript
// TypeScript: Operator interface
interface Operator {
  apply(change: Change): Change[];
  fetch(): AsyncIterable<Change>;
  destroy(): void;
}
```

```rust
// Rust: Operator trait
use valtron::iterator::TaskIterator;

pub trait Operator: Send + Sync {
    /// Process a single change, return resulting changes
    fn apply(&self, change: &Change) -> Vec<Change>;

    /// Fetch initial data (for new subscriptions)
    fn fetch<'a>(&'a self) -> Box<dyn Iterator<Item = Change> + Send + 'a>;

    /// Cleanup resources (if needed)
    fn destroy(&mut self) {}
}
```

### 3.2 Filter Operator

```typescript
// TypeScript: Filter operator
class FilterOperator implements Operator {
  apply(change: Change): Change[] {
    if (change.type === 'add') {
      if (this.evaluator.evaluate(change.node.row, this.condition)) {
        return [change];
      }
      return [];
    }
    // ... handle other change types
  }
}
```

```rust
// Rust: Filter operator
use crate::evaluator::ExpressionEvaluator;
use crate::condition::Condition;

pub struct FilterOperator {
    condition: Condition,
    evaluator: ExpressionEvaluator,
}

impl FilterOperator {
    pub fn new(condition: Condition, evaluator: ExpressionEvaluator) -> Self {
        Self { condition, evaluator }
    }

    fn evaluate_row(&self, row: &Row) -> bool {
        self.evaluator.evaluate(row, &self.condition)
    }
}

impl Operator for FilterOperator {
    fn apply(&self, change: &Change) -> Vec<Change> {
        match change {
            Change::Add(add) => {
                if self.evaluate_row(&add.node.row) {
                    return vec![change.clone()];
                }
                vec![]
            }

            Change::Remove(remove) => {
                if self.evaluate_row(&remove.node.row) {
                    return vec![change.clone()];
                }
                vec![]
            }

            Change::Edit(edit) => {
                let old_match = self.evaluate_row(&edit.old_node.row);
                let new_match = self.evaluate_row(&edit.node.row);

                match (old_match, new_match) {
                    (true, true) => vec![change.clone()],
                    (true, false) => vec![Change::Remove(RemoveChange {
                        relation: edit.relation.clone(),
                        node: edit.old_node.clone(),
                    })],
                    (false, true) => vec![Change::Add(AddChange {
                        relation: edit.relation.clone(),
                        node: edit.node.clone(),
                    })],
                    (false, false) => vec![],
                }
            }

            Change::Child(child) => vec![change.clone()],
        }
    }

    fn fetch<'a>(&'a self) -> Box<dyn Iterator<Item = Change> + Send + 'a> {
        Box::new(std::iter::empty())
    }
}
```

### 3.3 Join Operator with Arena Allocation

```rust
// Rust: Join operator with efficient memory management
use std::collections::HashMap;
use typed_arena::Arena;

pub struct JoinOperator {
    left_table: String,
    right_table: String,
    left_key: String,
    right_key: String,
    output_relation: String,

    // Indexes for efficient lookups
    left_index: HashMap<Value, Vec<Node>>,
    right_index: HashMap<Value, Vec<Node>>,

    // Arena for batch allocation
    arena: Arena<Node>,
}

impl JoinOperator {
    pub fn new(
        left_table: String,
        right_table: String,
        left_key: String,
        right_key: String,
        output_relation: String,
    ) -> Self {
        Self {
            left_table,
            right_table,
            left_key,
            right_key,
            output_relation,
            left_index: HashMap::new(),
            right_index: HashMap::new(),
            arena: Arena::new(),
        }
    }

    fn get_key_value(&self, row: &Row, is_left: bool) -> Option<&Value> {
        let key = if is_left { &self.left_key } else { &self.right_key };
        row.get(key)
    }

    fn create_joined_node(&self, left: &Node, right: &Node) -> Node {
        // Merge rows
        let mut merged_row = left.row.clone();
        merged_row.extend(right.row.iter().map(|(k, v)| (k.clone(), v.clone())));

        Node {
            row: merged_row,
            relationships: HashMap::new(), // Lazily populated
        }
    }
}

impl Operator for JoinOperator {
    fn apply(&self, change: &Change) -> Vec<Change> {
        let (table, key_field) = if change.relation() == self.left_table {
            (&self.left_table, &self.left_key)
        } else if change.relation() == self.right_table {
            (&self.right_table, &self.right_key)
        } else {
            return vec![]; // Not our tables
        };

        // Process based on change type and which side
        // ... (implementation similar to TypeScript but with Rust ownership)
        vec![]
    }

    fn fetch<'a>(&'a self) -> Box<dyn Iterator<Item = Change> + Send + 'a> {
        Box::new(std::iter::empty())
    }
}
```

## 4. Stream Processing with valtron

### 4.1 TaskIterator Pattern

```typescript
// TypeScript: Async stream
async function* fetchChanges(): AsyncIterable<Change> {
  const rows = await this.slowQuery();
  for (const row of rows) {
    yield { type: 'add', node: { row } };
  }
}
```

```rust
// Rust: valtron TaskIterator
use valtron::iterator::{TaskIterator, TaskStatus, Wakeup};
use valtron::no_spawner::NoSpawner;

pub struct ChangeFetcher {
    url: String,
    state: FetchState,
    offset: usize,
    limit: usize,
}

enum FetchState {
    Pending,
    WaitingForResponse { request_id: String },
    Processing { rows: Vec<Row> },
    Done,
}

impl TaskIterator for ChangeFetcher {
    type Ready = Change;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        match &mut self.state {
            FetchState::Pending => {
                // Start the fetch
                let request_id = self.start_fetch(&self.url, self.offset, self.limit);
                self.state = FetchState::WaitingForResponse { request_id };
                Some(TaskStatus::Pending {
                    wakeup: Wakeup::Custom(()),
                })
            }

            FetchState::WaitingForResponse { request_id } => {
                // Check if response is ready
                if let Some(rows) = self.check_response(request_id) {
                    self.state = FetchState::Processing { rows };
                    self.next() // Recurse to process first row
                } else {
                    Some(TaskStatus::Pending {
                        wakeup: Wakeup::Io(request_id.clone()),
                    })
                }
            }

            FetchState::Processing { rows } => {
                if let Some(row) = rows.pop() {
                    Some(TaskStatus::Ready(Change::Add(AddChange {
                        relation: "source".to_string(),
                        node: Node {
                            row,
                            relationships: HashMap::new(),
                        },
                    })))
                } else {
                    self.state = FetchState::Done;
                    self.next()
                }
            }

            FetchState::Done => None,
        }
    }
}
```

### 4.2 DrivenStreamIterator

For continuous change streams:

```rust
// Rust: DrivenStreamIterator for continuous streams
use valtron::iterator::{DrivenStreamIterator, TaskStatus, Wakeup};

pub struct ChangeStreamIterator {
    subscription_id: String,
    buffer: VecDeque<Change>,
    state: StreamState,
}

enum StreamState {
    Waiting,
    Received { changes: Vec<Change> },
    Closed,
}

impl DrivenStreamIterator for ChangeStreamIterator {
    type Item = Change;
    type Error = SyncError;

    fn next(&mut self) -> Option<TaskStatus<Self::Item, Self::Error>> {
        // First, return buffered changes
        if let Some(change) = self.buffer.pop_front() {
            return Some(TaskStatus::Ready(change));
        }

        // Buffer is empty, check state
        match &mut self.state {
            StreamState::Waiting => {
                Some(TaskStatus::Pending {
                    wakeup: Wakeup::Channel(self.subscription_id.clone()),
                })
            }

            StreamState::Received { changes } => {
                self.buffer.extend(changes.drain(..));
                self.state = StreamState::Waiting;
                self.next() // Recurse to return first buffered change
            }

            StreamState::Closed => None,
        }
    }
}
```

## 5. Pipeline Construction

### 5.1 Building the Pipeline

```rust
// Rust: IVM Pipeline builder
use crate::operators::*;

pub struct IVMPipeline {
    operators: Vec<Box<dyn Operator>>,
}

impl IVMPipeline {
    pub fn from_ast(ast: &Ast, table_source: Arc<dyn TableSource>) -> Self {
        let mut operators: Vec<Box<dyn Operator>> = Vec::new();

        // 1. Scan operator (always first)
        operators.push(Box::new(ScanOperator::new(
            ast.table.clone(),
            table_source,
        )));

        // 2. Filter operators (WHERE clause)
        if let Some(condition) = &ast.where_clause {
            operators.push(Box::new(FilterOperator::new(
                condition.clone(),
                ExpressionEvaluator::new(),
            )));
        }

        // 3. Join operators
        // ... add joins if present

        // 4. OrderBy operator
        if let Some(ordering) = &ast.order_by {
            operators.push(Box::new(OrderByOperator::new(ordering.clone())));
        }

        // 5. Limit operator
        if let Some(limit) = ast.limit {
            operators.push(Box::new(LimitOperator::new(limit)));
        }

        Self { operators }
    }

    pub fn process_change(&self, change: &Change) -> Vec<Change> {
        let mut changes = vec![change.clone()];

        for operator in &self.operators {
            let mut output: Vec<Change> = Vec::new();
            for c in &changes {
                output.extend(operator.apply(c));
            }
            changes = output;
        }

        changes
    }

    pub fn fetch_initial(&self) -> impl Iterator<Item = Change> {
        let scan_iterator = self.operators[0].fetch();
        // ... process through remaining operators
        scan_iterator
    }
}
```

## 6. Memory Management

### 6.1 Arena Allocation for Nodes

```rust
// Rust: Arena-based allocation for nodes
use typed_arena::Arena;
use std::sync::Arc;

pub struct PipelineContext {
    // Shared arena for batch allocation
    arena: Arc<Arena<Node>>,

    // Node references
    nodes: Vec<Arc<Node>>,
}

impl PipelineContext {
    pub fn new() -> Self {
        Self {
            arena: Arc::new(Arena::new()),
            nodes: Vec::new(),
        }
    }

    pub fn alloc_node(&mut self, row: Row) -> Arc<Node> {
        let node = self.arena.alloc(Node {
            row,
            relationships: HashMap::new(),
        });

        let arc = Arc::new(node);
        self.nodes.push(arc.clone());
        arc
    }

    pub fn flush(&mut self) {
        // Clear node references (arena memory freed when dropped)
        self.nodes.clear();

        // Create new arena for next batch
        self.arena = Arc::new(Arena::new());
    }
}
```

### 6.2 Reference Counting for Shared Data

```rust
// Rust: Arc for shared node references
use std::sync::Arc;

pub struct JoinState {
    // Multiple operators may reference the same nodes
    left_nodes: HashMap<Value, Vec<Arc<Node>>>,
    right_nodes: HashMap<Value, Vec<Arc<Node>>>,
}

impl JoinState {
    pub fn add_left_node(&mut self, node: Arc<Node>, key: Value) {
        self.left_nodes
            .entry(key)
            .or_insert_with(Vec::new)
            .push(node);
    }

    pub fn get_matches(&self, key: &Value, is_left: bool) -> Vec<Arc<Node>> {
        let nodes = if is_left {
            &self.right_nodes
        } else {
            &self.left_nodes
        };
        nodes.get(key).cloned().unwrap_or_default()
    }
}
```

## 7. Error Handling

### 7.1 Error Types

```rust
// Rust: Error types with thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Query error: {0}")]
    QueryError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Mutation error: {0}")]
    MutationError(String),

    #[error("Schema mismatch: {0}")]
    SchemaMismatch(String),

    #[error("WAL error: {0}")]
    WalError(#[from] tokio_postgres::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SyncError>;
```

### 7.2 Result Propagation

```rust
// Rust: Result handling in operators
impl Operator for FilterOperator {
    fn apply(&self, change: &Change) -> Vec<Change> {
        // No fallible operations, direct return
        // ...
    }

    fn fetch<'a>(&'a self) -> Box<dyn Iterator<Item = Change> + Send + 'a> {
        // Filter doesn't fetch independently
        Box::new(std::iter::empty())
    }
}

// For operators that may fail
impl Operator for ScanOperator {
    fn fetch<'a>(&'a self) -> Box<dyn Iterator<Item = Change> + Send + 'a> {
        match self.source.get_all(&self.table) {
            Ok(rows) => Box::new(rows.into_iter().map(|row| {
                Change::Add(AddChange {
                    relation: self.table.clone(),
                    node: Node {
                        row,
                        relationships: HashMap::new(),
                    },
                })
            })),
            Err(e) => {
                // Log error, return empty iterator
                log::error!("Scan fetch error: {}", e);
                Box::new(std::iter::empty())
            }
        }
    }
}
```

## 8. Complete Example

### 8.1 Full Pipeline Implementation

```rust
// Rust: Complete IVM pipeline example
use std::sync::Arc;
use valtron::executor::Executor;
use crate::{Ast, Change, IVMPipeline, MaterializedView};

pub struct Subscription {
    id: String,
    client_id: String,
    pipeline: IVMPipeline,
    view: MaterializedView,
}

impl Subscription {
    pub fn new(id: String, client_id: String, ast: Ast, table_source: Arc<dyn TableSource>) -> Self {
        let pipeline = IVMPipeline::from_ast(&ast, table_source);
        let view = MaterializedView::new();

        Self {
            id,
            client_id,
            pipeline,
            view,
        }
    }

    pub fn initialize(&mut self) -> Vec<Change> {
        let changes: Vec<Change> = self.pipeline.fetch_initial().collect();
        self.view.apply_changes(&changes);
        changes
    }

    pub fn apply_change(&mut self, change: &Change) -> Option<Vec<Change>> {
        let changes = self.pipeline.process_change(change);
        if changes.is_empty() {
            return None;
        }

        self.view.apply_changes(&changes);
        Some(changes)
    }
}

// Main sync engine
pub struct SyncEngine {
    subscriptions: HashMap<String, Subscription>,
    change_source: Arc<dyn ChangeSource>,
}

impl SyncEngine {
    pub fn subscribe(&mut self, client_id: String, ast: Ast) -> String {
        let id = generate_id();
        let subscription = Subscription::new(
            id.clone(),
            client_id,
            ast,
            self.change_source.clone(),
        );
        self.subscriptions.insert(id.clone(), subscription);
        id
    }

    pub fn process_changes(&mut self, changes: &[Change]) {
        for change in changes {
            for subscription in self.subscriptions.values_mut() {
                if let Some(changes) = subscription.apply_change(change) {
                    // Send changes to client
                    self.send_to_client(&subscription.client_id, changes);
                }
            }
        }
    }
}
```

### 8.2 Integration with valtron Executor

```rust
// Rust: Running the sync engine with valtron
use valtron::executor::Executor;
use valtron::config::Config;

fn main() {
    let config = Config::default();
    let mut executor = Executor::new(config);

    // Create sync engine
    let sync_engine = SyncEngine::new();

    // Spawn the main task
    executor.spawn(async move {
        // Note: This would need to be converted to TaskIterator pattern
        // The actual implementation would use TaskIterator throughout
        run_sync_engine(sync_engine).await;
    });

    // Run the executor
    executor.run();
}

// With TaskIterator (no async):
use valtron::iterator::{TaskIterator, TaskStatus, Wakeup};

pub struct SyncEngineTask {
    engine: SyncEngine,
    state: SyncState,
}

enum SyncState {
    WaitingForChanges,
    Processing { changes: Vec<Change> },
    Sending { client_id: String, changes: Vec<Change> },
}

impl TaskIterator for SyncEngineTask {
    type Ready = ();
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        match &mut self.state {
            SyncState::WaitingForChanges => {
                Some(TaskStatus::Pending {
                    wakeup: Wakeup::Channel("changes".to_string()),
                })
            }
            SyncState::Processing { changes } => {
                let changes = std::mem::take(changes);
                self.engine.process_changes(&changes);
                self.state = SyncState::WaitingForChanges;
                self.next()
            }
            SyncState::Sending { .. } => {
                // Handle sending
                self.state = SyncState::WaitingForChanges;
                Some(TaskStatus::Ready(()))
            }
        }
    }
}
```

---

*Next: [05-valtron-integration.md](05-valtron-integration.md)*
