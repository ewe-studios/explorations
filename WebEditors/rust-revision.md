---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors
explored_at: 2026-04-05
revised_at: 2026-04-05
workspace: webeditors-rs
---

# Rust Revision: WebEditors

## Overview

This document provides a comprehensive guide to implementing Rust versions of **Tiptap** (rich text editor) and **tldraw** (infinite canvas). It translates TypeScript/JavaScript concepts into idiomatic Rust, leveraging the existing ecosystem while maintaining architectural parity with the original projects.

The implementation is split into two main crates:
- **webeditors-rs/rich-text**: Tiptap/ProseMirror equivalent
- **webeditors-rs/canvas**: tldraw equivalent

Both share collaboration infrastructure via **yrs** (Yjs Rust port).

---

## Workspace Structure

```
webeditors-rs/
├── Cargo.toml                      # Workspace root
├── Cargo.lock
├── rust-revision.md
├── crates/
│   ├── we-core/                    # Core editor engine (Tiptap equivalent)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs              # Public API
│   │       ├── editor.rs           # Editor class
│   │       ├── state.rs            # EditorState
│   │       ├── transaction.rs      # Transaction system
│   │       ├── command.rs          # Command pattern
│   │       └── error.rs            # Error types
│   │
│   ├── we-schema/                  # Document schema (ProseMirror equivalent)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── schema.rs           # Schema definition
│   │       ├── node.rs             # Node traits
│   │       ├── mark.rs             # Mark traits
│   │       ├── content.rs          # Content validation
│   │       └── validation.rs       # Validation rules
│   │
│   ├── we-commands/                # Command registry
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs         # Command registry
│   │       ├── text.rs             # Text commands
│   │       ├── formatting.rs       # Formatting commands
│   │       └── insertion.rs        # Insertion commands
│   │
│   ├── we-extensions/              # Extension system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs           # Extension traits
│   │       ├── registry.rs         # Extension registry
│   │       ├── nodes/              # Built-in nodes
│   │       │   ├── mod.rs
│   │       │   ├── paragraph.rs
│   │       │   ├── heading.rs
│   │       │   ├── list.rs
│   │       │   └── code.rs
│   │       ├── marks/              # Built-in marks
│   │       │   ├── mod.rs
│   │       │   ├── bold.rs
│   │       │   ├── italic.rs
│   │       │   └── link.rs
│   │       └── plugins/            # Behavior extensions
│   │           ├── mod.rs
│   │           ├── history.rs
│   │           ├── input_rules.rs
│   │           └── placeholder.rs
│   │
│   ├── we-canvas-core/             # Canvas engine (tldraw equivalent)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── canvas.rs           # Canvas state
│   │       ├── camera.rs           # Camera system
│   │       ├── geometry/           # Geometry primitives
│   │       │   ├── mod.rs
│   │       │   ├── vec2.rs
│   │       │   ├── mat2.rs
│   │       │   └── bounds.rs
│   │       └── input.rs            # Input handling
│   │
│   ├── we-shapes/                  # Shape system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs           # Shape trait
│   │       ├── registry.rs         # Shape registry
│   │       ├── builtins/           # Built-in shapes
│   │       │   ├── mod.rs
│   │       │   ├── rectangle.rs
│   │       │   ├── ellipse.rs
│   │       │   ├── line.rs
│   │       │   ├── arrow.rs
│   │       │   └── text.rs
│   │       └── geometry.rs         # Shape geometry calculations
│   │
│   ├── we-render/                  # Rendering pipeline
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── renderer.rs         # Renderer trait
│   │       ├── gpu/                # GPU rendering
│   │       │   ├── mod.rs
│   │       │   ├── wgpu_renderer.rs
│   │       │   ├── pipeline.rs
│   │       │   └── batch.rs
│   │       ├── svg/                # SVG rendering
│   │       │   ├── mod.rs
│   │       │   └── usvg_renderer.rs
│   │       └── layers.rs           # Layer management
│   │
│   ├── we-tools/                   # Tool system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs           # Tool trait
│   │       ├── state_machine.rs    # Tool state machine
│   │       └── builtins/           # Built-in tools
│   │           ├── mod.rs
│   │           ├── select.rs
│   │           ├── draw.rs
│   │           ├── text.rs
│   │           └── hand.rs
│   │
│   ├── we-collab/                  # Collaboration (Yrs-based)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── provider.rs         # Collaboration provider
│   │       ├── awareness.rs        # Awareness protocol
│   │       ├── sync.rs             # Sync protocol
│   │       └── network.rs          # Network abstractions
│   │
│   ├── we-tauri/                   # Tauri desktop integration
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── app.rs              # Tauri app setup
│   │       └── commands.rs         # Tauri commands
│   │
│   └── we-ui/                      # UI rendering (egui/Iced)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── rich_text_ui.rs     # Rich text UI component
│           └── canvas_ui.rs        # Canvas UI component
│
├── examples/
│   ├── rich-text-basic/            # Basic rich text editor
│   ├── rich-text-collab/           # Collaborative rich text
│   ├── canvas-basic/               # Basic canvas
│   ├── canvas-collab/              # Collaborative canvas
│   └── combined-whiteboard/        # Combined editor
│
└── tests/
    ├── integration/
    └── e2e/
```

---

## Part 1: Rich Text Editor (Tiptap Equivalent)

### 1. Editor Foundation

#### 1.1 Rope Data Structure for Text

Rust's `rope` data structure provides O(log n) operations for large text documents:

```rust
// crates/we-core/src/rope_text.rs
use ropey::Rope;

/// Rope-based text storage with efficient editing
pub struct RopeText {
    rope: Rope,
    revision: u64,
}

impl RopeText {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            revision: 0,
        }
    }

    pub fn from_str(s: &str) -> Self {
        Self {
            rope: Rope::from_str(s),
            revision: 0,
        }
    }

    /// Insert text at byte position
    pub fn insert(&mut self, pos: usize, text: &str) {
        self.rope.insert(pos, text);
        self.revision += 1;
    }

    /// Delete range of bytes
    pub fn delete(&mut self, from: usize, to: usize) {
        self.rope.remove(from..to);
        self.revision += 1;
    }

    /// Get slice of text
    pub fn slice(&self, from: usize, to: usize) -> RopeSlice<'_> {
        self.rope.slice(from..to)
    }

    /// Convert char position to byte position
    pub fn char_to_byte(&self, char_pos: usize) -> usize {
        self.rope.char_to_byte(char_pos)
    }

    /// Convert byte position to char position
    pub fn byte_to_char(&self, byte_pos: usize) -> usize {
        self.rope.byte_to_char(byte_pos)
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}
```

**Cargo.toml dependency:**
```toml
[dependencies]
ropey = "1.6"
```

#### 1.2 Transaction System

The transaction system is the core of the editor's state management:

```rust
// crates/we-core/src/transaction.rs
use crate::{EditorState, Step};
use std::sync::Arc;

/// A transaction represents a set of changes to the document
#[derive(Debug, Clone)]
pub struct Transaction {
    /// The steps in this transaction
    steps: Vec<Arc<dyn Step>>,
    /// Transaction metadata
    metadata: TransactionMetadata,
    /// Whether to scroll into view after applying
    scroll_into_view: bool,
}

/// Metadata attached to a transaction
#[derive(Debug, Default, Clone)]
pub struct TransactionMetadata {
    /// Transaction source (user, api, collab, etc.)
    pub source: TransactionSource,
    /// Unique transaction ID
    pub id: u64,
    /// Timestamp
    pub timestamp: std::time::Instant,
    /// Custom metadata map
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

/// Source of a transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionSource {
    User,
    Api,
    Collaboration,
    Undo,
    Redo,
    Macro,
}

impl Transaction {
    pub fn builder() -> TransactionBuilder {
        TransactionBuilder::default()
    }

    pub fn steps(&self) -> &[Arc<dyn Step>] {
        &self.steps
    }

    pub fn metadata(&self) -> &TransactionMetadata {
        &self.metadata
    }

    /// Apply transaction to state
    pub fn apply(&self, state: &mut EditorState) -> Result<EditorState, TransactionError> {
        let mut new_state = state.clone();
        
        for step in &self.steps {
            step.apply(&mut new_state)?;
        }
        
        new_state.transaction = Some(self.clone());
        new_state.revision += 1;
        
        Ok(new_state)
    }
}

/// Builder for constructing transactions
#[derive(Default)]
pub struct TransactionBuilder {
    steps: Vec<Arc<dyn Step>>,
    metadata: TransactionMetadata,
    scroll_into_view: bool,
}

impl TransactionBuilder {
    pub fn step(mut self, step: impl Step + 'static) -> Self {
        self.steps.push(Arc::new(step));
        self
    }

    pub fn source(mut self, source: TransactionSource) -> Self {
        self.metadata.source = source;
        self
    }

    pub fn meta(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.custom.insert(key.to_string(), value);
        self
    }

    pub fn scroll_into_view(mut self, scroll: bool) -> Self {
        self.scroll_into_view = scroll;
        self
    }

    pub fn build(self) -> Transaction {
        Transaction {
            steps: self.steps,
            metadata: self.metadata,
            scroll_into_view: self.scroll_into_view,
        }
    }
}

/// A step is an atomic document change
pub trait Step: Send + Sync + std::fmt::Debug {
    /// Apply the step to the document
    fn apply(&self, state: &mut EditorState) -> Result<(), TransactionError>;
    
    /// Get the range affected by this step
    fn range(&self) -> std::ops::Range<usize>;
    
    /// Invert this step for undo
    fn invert(&self, state: &EditorState) -> Box<dyn Step>;
}

/// Error type for transaction operations
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("Step failed: {0}")]
    StepFailed(String),
    #[error("Invalid position: {0}")]
    InvalidPosition(usize),
    #[error("Document mismatch")]
    DocumentMismatch,
}
```

#### 1.3 Command Pattern

The command pattern provides a high-level API for making changes:

```rust
// crates/we-core/src/command.rs
use crate::{EditorState, Transaction, TransactionBuilder, TransactionSource};

/// Command trait for editor operations
pub trait Command: Send + Sync {
    /// Execute the command and return whether it succeeded
    fn execute(&self, state: &EditorState) -> Option<Transaction>;
    
    /// Check if command can be executed
    fn can_execute(&self, state: &EditorState) -> bool;
}

/// Command manager for chaining commands
pub struct CommandManager {
    state: std::sync::Arc<EditorState>,
    chain: Vec<Box<dyn Command>>,
}

impl CommandManager {
    pub fn new(state: std::sync::Arc<EditorState>) -> Self {
        Self { state, chain: Vec::new() }
    }

    /// Chain a command
    pub fn command<C: Command + 'static>(mut self, command: C) -> Self {
        self.chain.push(Box::new(command));
        self
    }

    /// Execute all chained commands
    pub fn run(self) -> Option<Transaction> {
        let mut tx_builder = Transaction::builder()
            .source(TransactionSource::User);

        for command in &self.chain {
            if let Some(tx) = command.execute(&self.state) {
                // Merge transaction steps
                for step in tx.steps() {
                    tx_builder = tx_builder.step(Arc::clone(step));
                }
            } else {
                return None; // Command failed
            }
        }

        Some(tx_builder.build())
    }
}

// ============ Built-in Commands ============

/// Insert text at current selection
pub struct InsertTextCommand {
    text: String,
}

impl InsertTextCommand {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Command for InsertTextCommand {
    fn execute(&self, state: &EditorState) -> Option<Transaction> {
        let selection = state.selection()?;
        Some(
            Transaction::builder()
                .step(InsertStep {
                    pos: selection.from,
                    text: self.text.clone(),
                })
                .source(TransactionSource::User)
                .build()
        )
    }

    fn can_execute(&self, state: &EditorState) -> bool {
        state.is_editable() && state.selection().is_some()
    }
}

/// Toggle a mark (bold, italic, etc.)
pub struct ToggleMarkCommand {
    mark_type: String,
    attributes: std::collections::HashMap<String, serde_json::Value>,
}

impl ToggleMarkCommand {
    pub fn new(mark_type: impl Into<String>) -> Self {
        Self {
            mark_type: mark_type.into(),
            attributes: std::collections::HashMap::new(),
        }
    }

    pub fn with_attributes(
        mut self,
        attrs: impl IntoIterator<Item = (String, serde_json::Value)>,
    ) -> Self {
        self.attributes = attrs.into_iter().collect();
        self
    }
}

impl Command for ToggleMarkCommand {
    fn execute(&self, state: &EditorState) -> Option<Transaction> {
        let selection = state.selection()?;
        
        if state.has_mark_in_range(&selection, &self.mark_type) {
            // Remove mark
            Some(
                Transaction::builder()
                    .step(RemoveMarkStep {
                        range: selection,
                        mark_type: self.mark_type.clone(),
                    })
                    .source(TransactionSource::User)
                    .build()
            )
        } else {
            // Add mark
            Some(
                Transaction::builder()
                    .step(AddMarkStep {
                        range: selection,
                        mark_type: self.mark_type.clone(),
                        attributes: self.attributes.clone(),
                    })
                    .source(TransactionSource::User)
                    .build()
            )
        }
    }

    fn can_execute(&self, state: &EditorState) -> bool {
        state.is_editable() && state.selection().is_some()
    }
}
```

#### 1.4 State Management

```rust
// crates/we-core/src/state.rs
use crate::{Document, Selection, Transaction, Schema};
use std::sync::Arc;

/// Immutable editor state
#[derive(Clone)]
pub struct EditorState {
    /// Document schema
    schema: Arc<Schema>,
    /// Current document
    document: Arc<Document>,
    /// Current selection
    selection: Option<Selection>,
    /// Current revision number
    revision: u64,
    /// Last transaction (if any)
    transaction: Option<Transaction>,
    /// Whether editor is editable
    editable: bool,
    /// Editor plugins state
    plugins_state: std::collections::HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
}

impl EditorState {
    /// Create initial state
    pub fn new(schema: Arc<Schema>, document: Document) -> Self {
        Self {
            schema,
            document: Arc::new(document),
            selection: Some(Selection::at(0)),
            revision: 0,
            transaction: None,
            editable: true,
            plugins_state: std::collections::HashMap::new(),
        }
    }

    /// Get document
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Get selection
    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    /// Get schema
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Check if editable
    pub fn is_editable(&self) -> bool {
        self.editable
    }

    /// Get marks at current position
    pub fn marks_at_cursor(&self) -> Vec<Mark> {
        let Some(selection) = self.selection else { return Vec::new() };
        self.document.marks_at(selection.from)
    }

    /// Check if selection has a specific mark
    pub fn has_mark_in_range(&self, range: &Selection, mark_type: &str) -> bool {
        self.document.has_mark_in_range(range, mark_type)
    }

    /// Get plugin state
    pub fn plugin_state<T: 'static>(&self, plugin_name: &str) -> Option<&T> {
        self.plugins_state
            .get(plugin_name)
            .and_then(|b| b.downcast_ref::<T>())
    }
}

/// Mutable state builder
pub struct EditorStateBuilder {
    state: EditorState,
}

impl EditorStateBuilder {
    pub fn new(schema: Arc<Schema>, document: Document) -> Self {
        Self {
            state: EditorState::new(schema, document),
        }
    }

    pub fn selection(mut self, selection: Selection) -> Self {
        self.state.selection = Some(selection);
        self
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.state.editable = editable;
        self
    }

    pub fn plugin_state<T: 'static + Send + Sync>(
        mut self,
        plugin_name: &str,
        state: T,
    ) -> Self {
        self.state
            .plugins_state
            .insert(plugin_name.to_string(), Box::new(state));
        self
    }

    pub fn build(self) -> EditorState {
        self.state
    }
}
```

---

### 2. Document Schema

#### 2.1 Node Traits

```rust
// crates/we-schema/src/node.rs
use std::sync::Arc;

/// Trait for document nodes
pub trait Node: Send + Sync + std::fmt::Debug {
    /// Node type name
    const TYPE: &'static str;

    /// Get node type
    fn node_type(&self) -> &'static str {
        Self::TYPE
    }

    /// Get node content as JSON
    fn to_json(&self) -> serde_json::Value;

    /// Get node attributes
    fn attributes(&self) -> &std::collections::HashMap<String, serde_json::Value>;

    /// Get child nodes
    fn children(&self) -> Option<&[Arc<dyn NodeContent>]>;

    /// Check if node can contain child of given type
    fn can_contain(&self, node_type: &str) -> bool;

    /// Check if node is inline
    fn is_inline(&self) -> bool;

    /// Check if node is block
    fn is_block(&self) -> bool {
        !self.is_inline()
    }

    /// Get text content (for text-containing nodes)
    fn text_content(&self) -> String {
        String::new()
    }

    /// Clone as trait object
    fn clone_box(&self) -> Arc<dyn Node>;
}

/// Trait for node content (nodes or text)
pub trait NodeContent: Send + Sync {
    fn as_node(&self) -> Option<&dyn Node>;
    fn as_text(&self) -> Option<&TextNode>;
    fn is_node(&self) -> bool;
    fn is_text(&self) -> bool;
}

/// Text node implementation
#[derive(Debug, Clone)]
pub struct TextNode {
    text: String,
    marks: Vec<Mark>,
}

impl TextNode {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            marks: Vec::new(),
        }
    }

    pub fn with_marks(mut self, marks: Vec<Mark>) -> Self {
        self.marks = marks;
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn marks(&self) -> &[Mark] {
        &self.marks
    }
}

/// Mark for inline formatting
#[derive(Debug, Clone)]
pub struct Mark {
    mark_type: String,
    attributes: std::collections::HashMap<String, serde_json::Value>,
}

impl Mark {
    pub fn new(mark_type: impl Into<String>) -> Self {
        Self {
            mark_type: mark_type.into(),
            attributes: std::collections::HashMap::new(),
        }
    }

    pub fn with_attributes(
        mut self,
        attrs: impl IntoIterator<Item = (String, serde_json::Value)>,
    ) -> Self {
        self.attributes = attrs.into_iter().collect();
        self
    }

    pub fn mark_type(&self) -> &str {
        &self.mark_type
    }

    pub fn attributes(&self) -> &std::collections::HashMap<String, serde_json::Value> {
        &self.attributes
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.attributes.get(key)
    }
}
```

#### 2.2 Mark Traits

```rust
// crates/we-schema/src/mark.rs
use std::sync::Arc;

/// Trait defining a mark type
pub trait MarkType: Send + Sync {
    /// Mark type name
    const TYPE: &'static str;

    /// Default attributes
    fn default_attributes(&self) -> std::collections::HashMap<String, serde_json::Value> {
        std::collections::HashMap::new()
    }

    /// Validate attributes
    fn validate_attributes(
        &self,
        attrs: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), MarkValidationError>;

    /// Check if mark can exist at position
    fn allows(&self, context: &MarkContext) -> bool;

    /// Check if marks are compatible (can coexist)
    fn compatible_with(&self, other: &dyn MarkType) -> bool;

    /// Check if mark is inclusive (wraps around cursor)
    fn is_inclusive(&self) -> bool {
        true
    }

    /// Check if mark is exclusive (doesn't coexist with same type)
    fn is_exclusive(&self) -> bool {
        false
    }
}

/// Context for mark validation
pub struct MarkContext<'a> {
    pub parent_node: &'a dyn Node,
    pub marks_before: &'a [Mark],
    pub marks_after: &'a [Mark],
}

#[derive(Debug, thiserror::Error)]
pub enum MarkValidationError {
    #[error("Invalid attribute: {0}")]
    InvalidAttribute(String),
    #[error("Missing required attribute: {0}")]
    MissingAttribute(String),
    #[error("Mark not allowed in this context")]
    NotAllowed,
}
```

#### 2.3 Schema Definition

```rust
// crates/we-schema/src/schema.rs
use std::sync::Arc;
use crate::{Node, NodeType, MarkType};

/// Schema defines the document structure
pub struct Schema {
    /// Registered node types
    nodes: std::collections::HashMap<String, Arc<dyn NodeType>>,
    /// Registered mark types
    marks: std::collections::HashMap<String, Arc<dyn MarkType>>,
    /// Top-level node type (usually 'doc')
    top_node: String,
}

impl Schema {
    pub fn builder() -> SchemaBuilder {
        SchemaBuilder::default()
    }

    /// Get node type by name
    pub fn node_type(&self, name: &str) -> Option<&dyn NodeType> {
        self.nodes.get(name).map(|n| n.as_ref())
    }

    /// Get mark type by name
    pub fn mark_type(&self, name: &str) -> Option<&dyn MarkType> {
        self.marks.get(name).map(|m| m.as_ref())
    }

    /// Get top-level node type
    pub fn top_node(&self) -> &str {
        &self.top_node
    }

    /// Check if node can contain child
    pub fn can_contain(&self, parent: &str, child: &str) -> bool {
        self.nodes
            .get(parent)
            .map(|n| n.allows_child(child))
            .unwrap_or(false)
    }

    /// Validate document structure
    pub fn validate(&self, doc: &dyn Node) -> Result<(), SchemaValidationError> {
        self.validate_node(doc, &[])
    }

    fn validate_node(
        &self,
        node: &dyn Node,
        parent_chain: &[&str],
    ) -> Result<(), SchemaValidationError> {
        let node_type = node.node_type();

        // Check if node type exists
        let Some(type_def) = self.nodes.get(node_type) else {
            return Err(SchemaValidationError::UnknownNodeType(node_type.to_string()));
        };

        // Check parent chain (depth limit)
        if parent_chain.len() > 100 {
            return Err(SchemaValidationError::TooDeep);
        }

        // Validate children
        if let Some(children) = node.children() {
            for child in children {
                if let Some(child_node) = child.as_node() {
                    // Check if parent can contain this child type
                    if !type_def.allows_child(child_node.node_type()) {
                        return Err(SchemaValidationError::InvalidContent(
                            node_type.to_string(),
                            child_node.node_type().to_string(),
                        ));
                    }

                    // Recursively validate child
                    let mut new_chain = parent_chain.to_vec();
                    new_chain.push(node_type);
                    self.validate_node(child_node, &new_chain)?;
                }
            }
        }

        Ok(())
    }
}

/// Schema builder for constructing schemas
#[derive(Default)]
pub struct SchemaBuilder {
    nodes: std::collections::HashMap<String, Arc<dyn NodeType>>,
    marks: std::collections::HashMap<String, Arc<dyn MarkType>>,
    top_node: Option<String>,
}

impl SchemaBuilder {
    pub fn node(mut self, node_type: impl NodeType + 'static) -> Self {
        let name = node_type.name().to_string();
        self.nodes.insert(name.clone(), Arc::new(node_type));
        if self.top_node.is_none() {
            self.top_node = Some(name);
        }
        self
    }

    pub fn mark(mut self, mark_type: impl MarkType + 'static) -> Self {
        let name = mark_type.name().to_string();
        self.marks.insert(name, Arc::new(mark_type));
        self
    }

    pub fn top_node(mut self, name: impl Into<String>) -> Self {
        self.top_node = Some(name.into());
        self
    }

    pub fn build(self) -> Result<Schema, SchemaError> {
        let top_node = self.top_node.ok_or(SchemaError::NoTopNode)?;
        Ok(Schema {
            nodes: self.nodes,
            marks: self.marks,
            top_node,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("No top node defined")]
    NoTopNode,
}

#[derive(Debug, thiserror::Error)]
pub enum SchemaValidationError {
    #[error("Unknown node type: {0}")]
    UnknownNodeType(String),
    #[error("Invalid content: {0} cannot contain {1}")]
    InvalidContent(String, String),
    #[error("Document too deep")]
    TooDeep,
}
```

#### 2.4 Content Validation

```rust
// crates/we-schema/src/content.rs
use crate::{Node, Schema};

/// Content validator for document operations
pub struct ContentValidator<'a> {
    schema: &'a Schema,
}

impl<'a> ContentValidator<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self { schema }
    }

    /// Validate insertion at position
    pub fn validate_insert(
        &self,
        parent: &dyn Node,
        child_type: &str,
        position: usize,
    ) -> Result<(), ValidationError> {
        let node_type = parent.node_type();

        // Check if parent can contain this child type
        if !self.schema.can_contain(node_type, child_type) {
            return Err(ValidationError::NotAllowed(
                node_type.to_string(),
                child_type.to_string(),
            ));
        }

        // Check position bounds
        let children = parent.children().map(|c| c.len()).unwrap_or(0);
        if position > children {
            return Err(ValidationError::OutOfBounds {
                position,
                max: children,
            });
        }

        Ok(())
    }

    /// Validate replacement of range
    pub fn validate_replace(
        &self,
        node: &dyn Node,
        from: usize,
        to: usize,
        replacement_type: &str,
    ) -> Result<(), ValidationError> {
        // Validate that replacement maintains schema
        self.validate_insert(node, replacement_type, from)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Content not allowed: {0} cannot contain {1}")]
    NotAllowed(String, String),
    #[error("Position out of bounds: {position} > {max}")]
    OutOfBounds { position: usize, max: usize },
    #[error("Invalid node structure")]
    InvalidStructure,
}
```

---

### 3. ProseMirror in Rust

#### 3.1 Existing Rust ProseMirror Crates

Several Rust implementations of ProseMirror concepts exist:

| Crate | Status | Description |
|-------|--------|-------------|
| `prosemirror-rust` | Experimental | Direct port attempt |
| `loro-crdt` | Active | CRDT with text support |
| `yrs` | Active | Yjs Rust port (recommended) |
| `automerge` | Active | Alternative CRDT |

For this implementation, we use **yrs** for collaboration and build native Rust types for the editor core.

#### 3.2 Document Operations

```rust
// crates/we-core/src/document.rs
use std::sync::Arc;
use crate::{Node, NodeContent, TextNode, Schema};

/// Immutable document tree
#[derive(Clone)]
pub struct Document {
    root: Arc<dyn Node>,
    text_index: Arc<TextIndex>,
}

impl Document {
    pub fn new(root: Arc<dyn Node>) -> Self {
        Self {
            text_index: Arc::new(TextIndex::build(&*root)),
            root,
        }
    }

    pub fn root(&self) -> &Arc<dyn Node> {
        &self.root
    }

    /// Get node at position
    pub fn node_at(&self, pos: usize) -> Option<&dyn Node> {
        // Use text index to find position
        let (parent, offset) = self.text_index.find(pos)?;
        parent.children()?.get(offset)?.as_node()
    }

    /// Get marks at position
    pub fn marks_at(&self, pos: usize) -> Vec<Mark> {
        self.text_index.marks_at(pos)
    }

    /// Get text content
    pub fn text_content(&self) -> String {
        self.collect_text(&self.root)
    }

    fn collect_text(&self, node: &dyn Node) -> String {
        let mut text = String::new();
        if let Some(children) = node.children() {
            for child in children {
                if let Some(text_node) = child.as_text() {
                    text.push_str(text_node.text());
                } else if let Some(child_node) = child.as_node() {
                    text.push_str(&self.collect_text(child_node));
                }
            }
        }
        text
    }

    /// Check if has mark in range
    pub fn has_mark_in_range(&self, range: &Selection, mark_type: &str) -> bool {
        for pos in range.from..range.to {
            let marks = self.marks_at(pos);
            if marks.iter().any(|m| m.mark_type() == mark_type) {
                return true;
            }
        }
        false
    }
}

/// Text index for efficient position lookup
struct TextIndex {
    /// Byte offset to node path
    offsets: Vec<(usize, NodePath)>,
}

type NodePath = Vec<usize>;

impl TextIndex {
    fn build(root: &dyn Node) -> Self {
        let mut offsets = Vec::new();
        let mut current_offset = 0;
        let mut path = Vec::new();
        Self::build_recursive(root, &mut offsets, &mut current_offset, &mut path);
        Self {
            offsets: offsets.into(),
        }
    }

    fn build_recursive(
        node: &dyn Node,
        offsets: &mut Vec<(usize, NodePath)>,
        offset: &mut usize,
        path: &mut NodePath,
    ) {
        if let Some(children) = node.children() {
            for (i, child) in children.iter().enumerate() {
                path.push(i);
                if let Some(text) = child.as_text() {
                    offsets.push((*offset, path.clone()));
                    *offset += text.text().len();
                } else if let Some(child_node) = child.as_node() {
                    Self::build_recursive(child_node, offsets, offset, path);
                }
                path.pop();
            }
        }
    }

    fn find(&self, pos: usize) -> Option<(&NodePath, usize)> {
        // Binary search for position
        match self.offsets.binary_search_by_key(&pos, |(o, _)| *o) {
            Ok(i) => Some((&self.offsets[i].1, 0)),
            Err(i) if i > 0 => Some((&self.offsets[i - 1].1, pos - self.offsets[i - 1].0)),
            _ => None,
        }
    }

    fn marks_at(&self, pos: usize) -> Vec<Mark> {
        // Return marks at position
        Vec::new() // Simplified
    }
}
```

#### 3.3 Selection System

```rust
// crates/we-core/src/selection.rs
use crate::{Node, Document};

/// Selection range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub from: usize,
    pub to: usize,
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    /// Create empty selection at position
    pub fn at(pos: usize) -> Self {
        Self {
            from: pos,
            to: pos,
            anchor: pos,
            head: pos,
        }
    }

    /// Create selection from range
    pub fn range(from: usize, to: usize) -> Self {
        Self {
            from,
            to,
            anchor: from,
            head: to,
        }
    }

    /// Check if selection is empty (cursor)
    pub fn is_empty(&self) -> bool {
        self.from == self.to
    }

    /// Get selection content from document
    pub fn content(&self, doc: &Document) -> String {
        // Extract text from range
        String::new() // Simplified
    }

    /// Extend selection to include position
    pub fn extend_to(mut self, pos: usize) -> Self {
        self.head = pos;
        self.from = self.anchor.min(self.head);
        self.to = self.anchor.max(self.head);
        self
    }

    /// Get node selection
    pub fn node(node_pos: usize, node_size: usize) -> Self {
        Self {
            from: node_pos,
            to: node_pos + node_size,
            anchor: node_pos,
            head: node_pos + node_size,
        }
    }
}

/// Text selection with affinity (which side of grapheme)
#[derive(Debug, Clone, Copy)]
pub struct TextSelection {
    pub selection: Selection,
    pub affinity: Affinity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Affinity {
    Forward,
    Backward,
}

/// Node selection for selecting entire nodes
pub struct NodeSelection {
    pub pos: usize,
    pub node: Arc<dyn Node>,
}

impl NodeSelection {
    pub fn as_range(&self) -> Selection {
        Selection::node(self.pos, 1) // Simplified
    }
}
```

#### 3.4 Transactions in Detail

```rust
// crates/we-core/src/transaction.rs (extended)
use crate::{Step, StepMap, Selection};

/// Step map tracks how positions change after a step
#[derive(Debug, Clone, Default)]
pub struct StepMap {
    mappings: Vec<(usize, usize, i32)>, // (from, to, delta)
}

impl StepMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add mapping
    pub fn add(&mut self, from: usize, to: usize, delta: i32) {
        self.mappings.push((from, to, delta));
    }

    /// Map position through step
    pub fn map_position(&self, pos: usize, bias: Bias) -> usize {
        let mut result = pos;
        for (from, to, delta) in &self.mappings {
            if pos <= *from {
                continue;
            }
            if pos >= *to {
                result = (result as i32 + delta) as usize;
            } else {
                // Position is inside changed range
                result = match bias {
                    Bias::Before => *from,
                    Bias::After => (*to as i32 + delta) as usize,
                };
            }
        }
        result
    }

    /// Map selection through step
    pub fn map_selection(&self, selection: Selection) -> Selection {
        Selection {
            from: self.map_position(selection.from, Bias::Before),
            to: self.map_position(selection.to, Bias::After),
            anchor: self.map_position(selection.anchor, Bias::Before),
            head: self.map_position(selection.head, Bias::After),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bias {
    Before,
    After,
}

// ============ Concrete Step Types ============

/// Insert text step
#[derive(Debug, Clone)]
pub struct InsertStep {
    pub pos: usize,
    pub text: String,
}

impl Step for InsertStep {
    fn apply(&self, state: &mut EditorState) -> Result<(), TransactionError> {
        // Apply insertion to document
        Ok(())
    }

    fn range(&self) -> std::ops::Range<usize> {
        self.pos..self.pos
    }

    fn invert(&self, _state: &EditorState) -> Box<dyn Step> {
        Box::new(DeleteStep {
            pos: self.pos,
            len: self.text.len(),
        })
    }
}

/// Delete step
#[derive(Debug, Clone)]
pub struct DeleteStep {
    pub pos: usize,
    pub len: usize,
}

impl Step for DeleteStep {
    fn apply(&self, state: &mut EditorState) -> Result<(), TransactionError> {
        Ok(())
    }

    fn range(&self) -> std::ops::Range<usize> {
        self.pos..self.pos + self.len
    }

    fn invert(&self, _state: &EditorState) -> Box<dyn Step> {
        Box::new(InsertStep {
            pos: self.pos,
            text: String::new(), // Would need original text
        })
    }
}

/// Add mark step
#[derive(Debug, Clone)]
pub struct AddMarkStep {
    pub range: Selection,
    pub mark_type: String,
    pub attributes: std::collections::HashMap<String, serde_json::Value>,
}

impl Step for AddMarkStep {
    fn apply(&self, state: &mut EditorState) -> Result<(), TransactionError> {
        Ok(())
    }

    fn range(&self) -> std::ops::Range<usize> {
        self.range.from..self.range.to
    }

    fn invert(&self, _state: &EditorState) -> Box<dyn Step> {
        Box::new(RemoveMarkStep {
            range: self.range,
            mark_type: self.mark_type.clone(),
        })
    }
}

/// Remove mark step
#[derive(Debug, Clone)]
pub struct RemoveMarkStep {
    pub range: Selection,
    pub mark_type: String,
}

impl Step for RemoveMarkStep {
    fn apply(&self, state: &mut EditorState) -> Result<(), TransactionError> {
        Ok(())
    }

    fn range(&self) -> std::ops::Range<usize> {
        self.range.from..self.range.to
    }

    fn invert(&self, _state: &EditorState) -> Box<dyn Step> {
        Box::new(AddMarkStep {
            range: self.range,
            mark_type: self.mark_type.clone(),
            attributes: std::collections::HashMap::new(),
        })
    }
}
```

---

### 4. Extension System

#### 4.1 Trait-based Extensions

```rust
// crates/we-extensions/src/traits.rs
use std::sync::Arc;
use crate::{ExtensionContext, Command, NodeType, MarkType};

/// Base extension trait
pub trait Extension: Send + Sync {
    /// Extension name
    fn name(&self) -> &'static str;

    /// Priority (higher = loaded first)
    fn priority(&self) -> i32 {
        0
    }

    /// Required extensions
    fn required_extensions(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// Called when extension is created
    fn on_create(&self, _ctx: &ExtensionContext) {}

    /// Called on transaction
    fn on_transaction(&self, _ctx: &TransactionContext) {}

    /// Called on selection update
    fn on_selection_update(&self, _ctx: &SelectionContext) {}

    /// Called on focus
    fn on_focus(&self, _ctx: &FocusContext) {}

    /// Called on blur
    fn on_blur(&self, _ctx: &BlurContext) {}

    /// Called on destroy
    fn on_destroy(&self, _ctx: &ExtensionContext) {}
}

/// Node extension trait
pub trait NodeExtension: Extension {
    /// Define node type
    fn define_node(&self) -> Arc<dyn NodeType>;

    /// Node view renderer (optional, for UI)
    fn node_view(&self) -> Option<Arc<dyn NodeRenderer>> {
        None
    }
}

/// Mark extension trait
pub trait MarkExtension: Extension {
    /// Define mark type
    fn define_mark(&self) -> Arc<dyn MarkType>;
}

/// Command extension trait
pub trait CommandExtension: Extension {
    /// Register commands
    fn register_commands(&self) -> Vec<(&'static str, Arc<dyn Command>)>;
}

/// Plugin for behavior modifications
pub trait Plugin: Extension {
    /// Handle keyboard input
    fn handle_key(&self, _ctx: &KeyEventContext) -> bool {
        false
    }

    /// Handle input rules (markdown-style triggers)
    fn input_rules(&self) -> Vec<InputRule> {
        Vec::new()
    }

    /// Provide decorations for editor view
    fn decorations(&self, _ctx: &DecorationContext) -> Vec<Decoration> {
        Vec::new()
    }
}

/// Input rule for markdown-style triggers
pub struct InputRule {
    pub pattern: regex::Regex,
    pub handler: Box<dyn Fn(&mut CommandContext, regex::Match) + Send + Sync>,
}

/// Decoration for editor view
pub struct Decoration {
    pub from: usize,
    pub to: usize,
    pub kind: DecorationKind,
}

pub enum DecorationKind {
    Inline(std::collections::HashMap<String, String>),
    Widget(Arc<dyn WidgetRenderer>),
}
```

#### 4.2 Extension Registry

```rust
// crates/we-extensions/src/registry.rs
use std::sync::Arc;
use crate::{Extension, NodeExtension, MarkExtension, CommandExtension, Plugin};

/// Registry for all extensions
pub struct ExtensionRegistry {
    extensions: Vec<Arc<dyn Extension>>,
    nodes: std::collections::HashMap<String, Arc<dyn NodeType>>,
    marks: std::collections::HashMap<String, Arc<dyn MarkType>>,
    commands: std::collections::HashMap<String, Arc<dyn Command>>,
    plugins: Vec<Arc<dyn Plugin>>,
}

impl ExtensionRegistry {
    pub fn builder() -> ExtensionRegistryBuilder {
        ExtensionRegistryBuilder::new()
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Extension>> {
        self.extensions.iter().find(|e| e.name() == name)
    }

    pub fn node(&self, name: &str) -> Option<&dyn NodeType> {
        self.nodes.get(name).map(|n| n.as_ref())
    }

    pub fn mark(&self, name: &str) -> Option<&dyn MarkType> {
        self.marks.get(name).map(|m| m.as_ref())
    }

    pub fn command(&self, name: &str) -> Option<&dyn Command> {
        self.commands.get(name).map(|c| c.as_ref())
    }

    pub fn plugins(&self) -> &[Arc<dyn Plugin>] {
        &self.plugins
    }
}

/// Builder for extension registry
pub struct ExtensionRegistryBuilder {
    extensions: Vec<Arc<dyn Extension>>,
}

impl ExtensionRegistryBuilder {
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
        }
    }

    pub fn extension(mut self, ext: impl Extension + 'static) -> Self {
        self.extensions.push(Arc::new(ext));
        self
    }

    pub fn extensions<I>(mut self, exts: I) -> Self
    where
        I: IntoIterator<Item = Arc<dyn Extension>>,
    {
        self.extensions.extend(exts);
        self
        self
    }

    pub fn build(self) -> ExtensionRegistry {
        // Sort by priority (descending)
        let mut extensions = self.extensions;
        extensions.sort_by(|a, b| b.priority().cmp(&a.priority()));

        // Collect nodes, marks, commands, plugins
        let mut nodes = std::collections::HashMap::new();
        let mut marks = std::collections::HashMap::new();
        let mut commands = std::collections::HashMap::new();
        let mut plugins = Vec::new();

        for ext in &extensions {
            // Try downcast to each extension type
            // This would use Any trait in practice
        }

        ExtensionRegistry {
            extensions,
            nodes,
            marks,
            commands,
            plugins,
        }
    }
}
```

#### 4.3 Built-in Node Extensions

```rust
// crates/we-extensions/src/nodes/paragraph.rs
use crate::{NodeExtension, Extension, NodeType, NodeContext};
use std::sync::Arc;

/// Paragraph node extension
pub struct ParagraphExtension;

impl Extension for ParagraphExtension {
    fn name(&self) -> &'static str {
        "paragraph"
    }

    fn priority(&self) -> i32 {
        100
    }
}

impl NodeExtension for ParagraphExtension {
    fn define_node(&self) -> Arc<dyn NodeType> {
        Arc::new(ParagraphNodeType)
    }
}

/// Paragraph node type implementation
pub struct ParagraphNodeType;

impl NodeType for ParagraphNodeType {
    fn name(&self) -> &'static str {
        "paragraph"
    }

    fn group(&self) -> &'static str {
        "block"
    }

    fn content(&self) -> &'static str {
        "text*"
    }

    fn parse(&self) -> Option<Arc<dyn NodeParser>> {
        Some(Arc::new(ParagraphParser))
    }
}

// crates/we-extensions/src/nodes/heading.rs
pub struct HeadingExtension {
    levels: Vec<u8>,
}

impl HeadingExtension {
    pub fn new() -> Self {
        Self {
            levels: vec![1, 2, 3, 4, 5, 6],
        }
    }

    pub fn with_levels(mut self, levels: Vec<u8>) -> Self {
        self.levels = levels;
        self
    }
}

impl Default for HeadingExtension {
    fn default() -> Self {
        Self::new()
    }
}

// crates/we-extensions/src/nodes/list.rs
pub struct BulletListExtension;
pub struct OrderedListExtension;
pub struct ListItemExtension;

// crates/we-extensions/src/nodes/code.rs
pub struct CodeBlockExtension;
pub struct InlineCodeExtension;
```

#### 4.4 Built-in Mark Extensions

```rust
// crates/we-extensions/src/marks/bold.rs
use crate::{MarkExtension, Extension, MarkType, MarkContext};
use std::sync::Arc;

pub struct BoldExtension;

impl Extension for BoldExtension {
    fn name(&self) -> &'static str {
        "bold"
    }
}

impl MarkExtension for BoldExtension {
    fn define_mark(&self) -> Arc<dyn MarkType> {
        Arc::new(BoldMarkType)
    }
}

pub struct BoldMarkType;

impl MarkType for BoldMarkType {
    fn name(&self) -> &'static str {
        "bold"
    }

    fn parse(&self) -> Option<Arc<dyn MarkParser>> {
        Some(Arc::new(BoldParser))
    }
}

// crates/we-extensions/src/marks/italic.rs
pub struct ItalicExtension;

// crates/we-extensions/src/marks/link.rs
pub struct LinkExtension {
    openOnClick: bool,
    htmlAttributes: std::collections::HashMap<String, String>,
}
```

#### 4.5 Plugin Extensions

```rust
// crates/we-extensions/src/plugins/history.rs
use crate::{Plugin, Extension, ExtensionContext, TransactionContext};
use std::collections::VecDeque;

/// History plugin for undo/redo
pub struct HistoryPlugin {
    max_depth: usize,
}

impl HistoryPlugin {
    pub fn new() -> Self {
        Self { max_depth: 100 }
    }

    pub fn with_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }
}

impl Default for HistoryPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Extension for HistoryPlugin {
    fn name(&self) -> &'static str {
        "history"
    }
}

impl Plugin for HistoryPlugin {
    fn on_transaction(&self, ctx: &TransactionContext) {
        // Push to undo stack
    }
}

/// History state
pub struct HistoryState {
    undo_stack: VecDeque<Transaction>,
    redo_stack: VecDeque<Transaction>,
}

impl HistoryState {
    pub fn undo(&mut self) -> Option<Transaction> {
        self.undo_stack.pop_back()
    }

    pub fn redo(&mut self) -> Option<Transaction> {
        self.redo_stack.pop_back()
    }

    pub fn push(&mut self, tx: Transaction) {
        self.undo_stack.push_back(tx);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.pop_front();
        }
    }
}

// crates/we-extensions/src/plugins/input_rules.rs
pub struct InputRulesPlugin;

// crates/we-extensions/src/plugins/placeholder.rs
pub struct PlaceholderPlugin {
    placeholder: String,
}
```

---

### 5. UI Integration

#### 5.1 Tauri Desktop Integration

```rust
// crates/we-tauri/src/app.rs
use tauri::{Manager, Window, AppHandle};
use we_core::Editor;
use std::sync::Arc;

/// Setup Tauri app with editor
pub fn setup_editor_app() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            // Create main window
            let main_window = tauri::WindowBuilder::new(
                app,
                "main",
                tauri::WindowUrl::App("index.html".into()),
            )
            .title("WebEditors")
            .inner_size(1200.0, 800.0)
            .build()?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_editor,
            commands::insert_text,
            commands::toggle_mark,
            commands::get_content,
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}

// crates/we-tauri/src/commands.rs
use tauri::State;
use we_core::{Editor, EditorOptions};
use std::sync::{Arc, Mutex};

/// Editor state for Tauri app
pub struct EditorState {
    editors: Arc<Mutex<std::collections::HashMap<String, Arc<Editor>>>>,
}

/// Create a new editor
#[tauri::command]
pub fn create_editor(
    state: State<EditorState>,
    id: String,
    options: EditorOptions,
) -> Result<(), String> {
    let editor = Arc::new(Editor::new(options));
    state
        .editors
        .lock()
        .unwrap()
        .insert(id, editor);
    Ok(())
}

/// Insert text
#[tauri::command]
pub fn insert_text(
    state: State<EditorState>,
    id: String,
    text: String,
) -> Result<(), String> {
    let editors = state.editors.lock().unwrap();
    let editor = editors.get(&id).ok_or("Editor not found")?;
    editor.insert_text(&text);
    Ok(())
}
```

**Cargo.toml:**
```toml
[dependencies]
tauri = { version = "2.0", features = ["macos-private-api"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

#### 5.2 egui Native UI

```rust
// crates/we-ui/src/rich_text_ui.rs
use egui::{Response, RichText, Ui, Color32};
use we_core::{Editor, Selection};

/// Rich text editor widget
pub struct RichTextEditor<'a> {
    editor: &'a mut Editor,
    id: egui::Id,
    show_toolbar: bool,
}

impl<'a> RichTextEditor<'a> {
    pub fn new(editor: &'a mut Editor, id: impl Into<egui::Id>) -> Self {
        Self {
            editor,
            id: id.into(),
            show_toolbar: true,
        }
    }

    pub fn show_toolbar(mut self, show: bool) -> Self {
        self.show_toolbar = show;
        self
    }
}

impl<'a> egui::Widget for RichTextEditor<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut state = ui.data_mut(|d| d.get_temp::<EditorState>(self.id).unwrap_or_default());

        ui.vertical(|ui| {
            // Toolbar
            if self.show_toolbar {
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("B").strong()).clicked() {
                        self.editor.toggle_mark("bold");
                    }
                    if ui.button(RichText::new("I").italics()).clicked() {
                        self.editor.toggle_mark("italic");
                    }
                });
            }

            // Editor content
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical(|ui| {
                    let content = self.editor.get_content_html();
                    ui.label(RichText::new(content).font(egui::TextStyle::Body));
                });
            });
        });

        ui.data_mut(|d| d.insert_temp(self.id, state));

        ui.response()
    }
}

/// Editor UI state
#[derive(Default)]
pub struct EditorState {
    scroll: egui::Vec2,
    cursor_pos: Option<egui::Pos2>,
}
```

**Cargo.toml:**
```toml
[dependencies]
egui = "0.29"
eframe = "0.29"
```

#### 5.3 Iced Native UI

```rust
// Alternative Iced implementation
use iced::{Element, Command, Subscription};
use we_core::Editor;

pub struct RichTextEditorApp {
    editor: Editor,
    content: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    ContentChanged(String),
    SelectionChanged(Selection),
    BoldToggled,
    ItalicToggled,
}

impl iced::Application for RichTextEditorApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                editor: Editor::new(EditorOptions::default()),
                content: String::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Rich Text Editor")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ContentChanged(content) => {
                self.content = content;
            }
            Message::BoldToggled => {
                self.editor.toggle_mark("bold");
            }
            Message::ItalicToggled => {
                self.editor.toggle_mark("italic");
            }
            _ => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        use iced::widget::{button, column, text, vertical_space};

        column![
            button(text("B").size(20)).on_press(Message::BoldToggled),
            button(text("I").size(20)).on_press(Message::ItalicToggled),
            vertical_space().height(10),
            text(&self.content).size(16),
        ]
        .into()
    }
}
```

**Cargo.toml:**
```toml
[dependencies]
iced = "0.13"
```

#### 5.4 Web Rendering with Wry

```rust
// Using wry for webview embedding
use wry::{Application, ApplicationBuilder, WebViewBuilder};
use tao::event::Event;

pub fn create_webview_editor() {
    let mut app = ApplicationBuilder::new().build().unwrap();

    let window = app.add_window().unwrap();

    let webview = WebViewBuilder::new(window)
        .unwrap()
        .with_url("http://localhost:3000")
        .unwrap()
        .with_ipc_handler(|window, message| {
            // Handle editor commands from web
            println!("Received: {}", message);
        })
        .build()
        .unwrap();

    app.run(|webview, event| {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                _ => {}
            }
        }
    });
}
```

**Cargo.toml:**
```toml
[dependencies]
wry = "0.45"
tao = "0.30"
```

---

## Part 2: Canvas Editor (tldraw Equivalent)

### 6. Canvas Foundation

#### 6.1 2D Graphics with wgpu

```rust
// crates/we-canvas-core/src/graphics.rs
use wgpu::{self, util::DeviceExt};
use std::sync::Arc;

/// Graphics context for canvas rendering
pub struct GraphicsContext {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
}

impl GraphicsContext {
    pub async fn new(window: Option<&winit::window::Window>) -> Result<Self, GraphicsError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: window.and_then(|w| w.surface().ok()),
            })
            .await
            .ok_or(GraphicsError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Canvas Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let mut surface = None;
        let mut config = None;

        if let Some(window) = window {
            let surface = window.surface().unwrap();
            let config = surface.get_default_config(&adapter, window.inner_size());
            surface.configure(&device, &config);
            // ...
        }

        Ok(Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
            surface,
            config,
        })
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if let (Some(surface), Some(config)) = (&self.surface, &mut self.config) {
            config.width = size.width;
            config.height = size.height;
            surface.configure(&self.device, config);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GraphicsError {
    #[error("No suitable adapter found")]
    NoAdapter,
    #[error("Device request failed: {0}")]
    DeviceError(#[from] wgpu::RequestDeviceError),
}
```

**Cargo.toml:**
```toml
[dependencies]
wgpu = "22"
winit = "0.30"
bytemuck = { version = "1.14", features = ["derive"] }
```

#### 6.2 SVG Rendering with usvg

```rust
// crates/we-render/src/svg/usvg_renderer.rs
use resvg::usvg;
use we_canvas_core::GraphicsContext;

/// SVG renderer using resvg
pub struct UsvgRenderer {
    options: usvg::Options,
}

impl UsvgRenderer {
    pub fn new() -> Result<Self, usvg::Error> {
        Ok(Self {
            options: usvg::Options::default(),
        })
    }

    /// Parse SVG string
    pub fn parse(&self, svg: &str) -> Result<usvg::Tree, usvg::Error> {
        usvg::Tree::from_str(svg, &self.options)
    }

    /// Render SVG to pixmap
    pub fn render(&self, svg: &usvg::Tree, size: (u32, u32)) -> resvg::tiny_skia::Pixmap {
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.0, size.1).unwrap();
        let transform = resvg::Transform::default();
        resvg::render(svg, transform, &mut pixmap.as_mut());
        pixmap
    }

    /// Render SVG to GPU texture
    pub fn render_to_texture(
        &self,
        svg: &usvg::Tree,
        graphics: &GraphicsContext,
    ) -> wgpu::Texture {
        // Rasterize to pixmap first
        let size = (
            svg.size().width() as u32,
            svg.size().height() as u32,
        );
        let pixmap = self.render(svg, size);

        // Create texture from pixmap
        let texture = graphics.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("SVG Texture"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload pixels
        graphics.queue().write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixmap.data(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(size.0 * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );

        texture
    }
}
```

**Cargo.toml:**
```toml
[dependencies]
resvg = "0.43"
tiny-skia = "0.11"
```

#### 6.3 Scene Graph

```rust
// crates/we-canvas-core/src/scene.rs
use std::sync::Arc;
use crate::{Shape, ShapeId, Transform};

/// Scene graph for canvas
pub struct Scene {
    /// Root layer
    root: Layer,
    /// All shapes by ID
    shapes: std::collections::HashMap<ShapeId, Arc<dyn Shape>>,
    /// Shape ordering
    shape_order: Vec<ShapeId>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            root: Layer::new("root".into()),
            shapes: std::collections::HashMap::new(),
            shape_order: Vec::new(),
        }
    }

    /// Add shape to scene
    pub fn add_shape(&mut self, shape: Arc<dyn Shape>) {
        let id = shape.id();
        self.shapes.insert(id, shape);
        self.shape_order.push(id);
    }

    /// Remove shape from scene
    pub fn remove_shape(&mut self, id: ShapeId) -> Option<Arc<dyn Shape>> {
        self.shape_order.retain(|i| i != &id);
        self.shapes.remove(&id)
    }

    /// Get shape by ID
    pub fn get_shape(&self, id: ShapeId) -> Option<&Arc<dyn Shape>> {
        self.shapes.get(&id)
    }

    /// Get all shapes
    pub fn shapes(&self) -> impl Iterator<Item = &Arc<dyn Shape>> {
        self.shape_order.iter().filter_map(|id| self.shapes.get(id))
    }

    /// Get shapes in rendering order
    pub fn shapes_ordered(&self) -> Vec<&Arc<dyn Shape>> {
        self.shape_order.iter().filter_map(|id| self.shapes.get(id)).collect()
    }

    /// Update shape transform
    pub fn update_transform(&mut self, id: ShapeId, transform: Transform) {
        if let Some(shape) = self.shapes.get_mut(&id) {
            Arc::make_mut(shape).set_transform(transform);
        }
    }

    /// Hit test at point
    pub fn hit_test(&self, point: Vec2, ignore: &[ShapeId]) -> Option<ShapeId> {
        // Test in reverse order (top to bottom)
        for id in self.shape_order.iter().rev() {
            if ignore.contains(id) {
                continue;
            }
            if let Some(shape) = self.shapes.get(id) {
                if shape.hit_test(point) {
                    return Some(*id);
                }
            }
        }
        None
    }
}

/// Layer for grouping shapes
pub struct Layer {
    name: String,
    children: Vec<LayerChild>,
    visible: bool,
    locked: bool,
}

enum LayerChild {
    Layer(Layer),
    Shape(ShapeId),
}
```

#### 6.4 Camera System

```rust
// crates/we-canvas-core/src/camera.rs
use crate::{Vec2, Mat2, Bounds};

/// Camera for infinite canvas
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Camera position in page space
    pub position: Vec2,
    /// Zoom level
    pub zoom: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
        }
    }

    /// Convert screen coordinates to page coordinates
    pub fn screen_to_page(&self, screen: Vec2, screen_center: Vec2) -> Vec2 {
        let offset = screen - screen_center;
        screen_center + offset / self.zoom + self.position
    }

    /// Convert page coordinates to screen coordinates
    pub fn page_to_screen(&self, page: Vec2, screen_center: Vec2) -> Vec2 {
        let offset = (page - self.position) * self.zoom;
        screen_center + offset
    }

    /// Zoom to point
    pub fn zoom_to(&mut self, point: Vec2, factor: f32) {
        let new_zoom = (self.zoom * factor).clamp(0.01, 100.0);
        // Keep point stationary during zoom
        self.position = point - (point - self.position) * (new_zoom / self.zoom);
        self.zoom = new_zoom;
    }

    /// Pan camera
    pub fn pan(&mut self, delta: Vec2) {
        self.position -= delta / self.zoom;
    }

    /// Fit camera to bounds
    pub fn fit_to_bounds(&mut self, bounds: Bounds, viewport: Vec2, padding: f32) {
        let bounds_size = bounds.size();
        let zoom_x = (viewport.x - padding * 2.0) / bounds_size.x;
        let zoom_y = (viewport.y - padding * 2.0) / bounds_size.y;
        self.zoom = zoom_x.min(zoom_y).clamp(0.01, 100.0);

        let center = bounds.center();
        self.position = -center;
    }

    /// Get visible bounds
    pub fn visible_bounds(&self, viewport: Vec2) -> Bounds {
        let half_size = Vec2::new(viewport.x / 2.0 / self.zoom, viewport.y / 2.0 / self.zoom);
        Bounds::from_center_size(-self.position, half_size * 2.0)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}
```

---

### 7. Shape System

#### 7.1 Shape Trait

```rust
// crates/we-shapes/src/traits.rs
use std::sync::Arc;
use we_canvas_core::{Vec2, Mat2, Bounds, Transform};

/// Unique shape identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapeId(u64);

impl ShapeId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        ShapeId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Base trait for all shapes
pub trait Shape: Send + Sync {
    /// Get shape ID
    fn id(&self) -> ShapeId;

    /// Get shape type
    fn shape_type(&self) -> &'static str;

    /// Get parent ID
    fn parent_id(&self) -> Option<ShapeId>;

    /// Get shape position
    fn position(&self) -> Vec2;

    /// Get shape rotation (radians)
    fn rotation(&self) -> f32;

    /// Get shape scale
    fn scale(&self) -> Vec2;

    /// Get shape opacity
    fn opacity(&self) -> f32;

    /// Get shape bounds
    fn bounds(&self) -> Bounds;

    /// Get shape geometry for hit testing
    fn geometry(&self) -> Arc<dyn ShapeGeometry>;

    /// Render shape
    fn render(&self, ctx: &mut RenderContext);

    /// Hit test point
    fn hit_test(&self, point: Vec2) -> bool {
        self.geometry().contains(point)
    }

    /// Get snap points
    fn snap_points(&self) -> Vec<SnapPoint>;

    /// Clone as trait object
    fn clone_box(&self) -> Arc<dyn Shape>;
}

/// Shape geometry for hit testing and bounds
pub trait ShapeGeometry: Send + Sync {
    /// Check if point is inside shape
    fn contains(&self, point: Vec2) -> bool;

    /// Get outline path
    fn outline(&self) -> Vec<Vec2>;

    /// Get bounding box
    fn bounds(&self) -> Bounds;

    /// Get transformed bounds
    fn transformed_bounds(&self, transform: Transform) -> Bounds;
}

/// Render context for shapes
pub struct RenderContext<'a> {
    pub canvas: &'a mut dyn Canvas,
    pub transform: Mat2,
    pub opacity: f32,
}

/// Canvas trait for rendering abstraction
pub trait Canvas {
    fn save(&mut self);
    fn restore(&mut self);
    fn translate(&mut self, x: f32, y: f32);
    fn rotate(&mut self, angle: f32);
    fn scale(&mut self, sx: f32, sy: f32);
    fn set_opacity(&mut self, opacity: f32);
    fn fill_rect(&mut self, rect: Bounds, color: Color);
    fn stroke_rect(&mut self, rect: Bounds, color: Color, width: f32);
    fn fill_path(&mut self, path: &[Vec2], color: Color);
    fn stroke_path(&mut self, path: &[Vec2], color: Color, width: f32);
    fn draw_text(&mut self, text: &str, pos: Vec2, style: &TextStyle);
    fn draw_image(&mut self, image: &Image, bounds: Bounds);
}
```

#### 7.2 Shape Registry

```rust
// crates/we-shapes/src/registry.rs
use std::sync::Arc;
use crate::{Shape, ShapeType, ShapeId};

/// Registry for shape types
pub struct ShapeRegistry {
    types: std::collections::HashMap<&'static str, Arc<dyn ShapeType>>,
}

impl ShapeRegistry {
    pub fn new() -> Self {
        Self {
            types: std::collections::HashMap::new(),
        }
    }

    /// Register a shape type
    pub fn register(&mut self, shape_type: Arc<dyn ShapeType>) {
        self.types.insert(shape_type.type_name(), shape_type);
    }

    /// Get shape type by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn ShapeType>> {
        self.types.get(name)
    }

    /// Create shape from props
    pub fn create(
        &self,
        shape_type: &str,
        id: ShapeId,
        props: serde_json::Value,
    ) -> Option<Arc<dyn Shape>> {
        let shape_type = self.types.get(shape_type)?;
        Some(shape_type.create(id, props))
    }
}

/// Shape type definition
pub trait ShapeType: Send + Sync {
    /// Type name
    fn type_name(&self) -> &'static str;

    /// Default props
    fn default_props(&self) -> serde_json::Value;

    /// Create shape instance
    fn create(&self, id: ShapeId, props: serde_json::Value) -> Arc<dyn Shape>;

    /// Validate props
    fn validate_props(&self, props: &serde_json::Value) -> Result<(), ValidationError>;

    /// Get shape geometry from props
    fn geometry(&self, props: &serde_json::Value) -> Arc<dyn ShapeGeometry>;
}
```

#### 7.3 Built-in Shapes

```rust
// crates/we-shapes/src/builtins/rectangle.rs
use crate::{Shape, ShapeId, ShapeGeometry, ShapeType, Vec2, Bounds, RenderContext};
use std::sync::Arc;

/// Rectangle shape type
pub struct RectangleType;

impl ShapeType for RectangleType {
    fn type_name(&self) -> &'static str {
        "rectangle"
    }

    fn default_props(&self) -> serde_json::Value {
        serde_json::json!({
            "w": 100.0,
            "h": 100.0,
            "fill": "#000000",
            "stroke": "#000000",
            "strokeWidth": 2.0,
        })
    }

    fn create(&self, id: ShapeId, props: serde_json::Value) -> Arc<dyn Shape> {
        Arc::new(RectangleShape {
            id,
            props,
            transform: Transform::default(),
        })
    }
}

/// Rectangle shape instance
pub struct RectangleShape {
    id: ShapeId,
    props: serde_json::Value,
    transform: Transform,
}

impl Shape for RectangleShape {
    fn id(&self) -> ShapeId {
        self.id
    }

    fn shape_type(&self) -> &'static str {
        "rectangle"
    }

    fn bounds(&self) -> Bounds {
        let w = self.props["w"].as_f64().unwrap_or(100.0) as f32;
        let h = self.props["h"].as_f64().unwrap_or(100.0) as f32;
        Bounds::from_size(Vec2::new(w, h))
    }

    fn geometry(&self) -> Arc<dyn ShapeGeometry> {
        Arc::new(RectangleGeometry {
            bounds: self.bounds(),
        })
    }

    fn render(&self, ctx: &mut RenderContext) {
        let bounds = self.bounds();
        let fill = self.props["fill"].as_str().unwrap_or("#000000");
        let stroke = self.props["stroke"].as_str().unwrap_or("#000000");

        ctx.fill_rect(bounds, Color::from_hex(fill));
        ctx.stroke_rect(bounds, Color::from_hex(stroke), 2.0);
    }
}

struct RectangleGeometry {
    bounds: Bounds,
}

impl ShapeGeometry for RectangleGeometry {
    fn contains(&self, point: Vec2) -> bool {
        self.bounds.contains(point)
    }

    fn outline(&self) -> Vec<Vec2> {
        vec![
            self.bounds.min,
            Vec2::new(self.bounds.max.x, self.bounds.min.y),
            self.bounds.max,
            Vec2::new(self.bounds.min.x, self.bounds.max.y),
        ]
    }

    fn bounds(&self) -> Bounds {
        self.bounds
    }
}

// crates/we-shapes/src/builtins/ellipse.rs
pub struct EllipseType;
pub struct EllipseShape { /* ... */ }

// crates/we-shapes/src/builtins/line.rs
pub struct LineType;
pub struct LineShape { /* ... */ }

// crates/we-shapes/src/builtins/arrow.rs
pub struct ArrowType;
pub struct ArrowShape { /* ... */ }

// crates/we-shapes/src/builtins/text.rs
pub struct TextType;
pub struct TextShape { /* ... */ }
```

#### 7.4 Geometry Calculations

```rust
// crates/we-shapes/src/geometry.rs
use we_canvas_core::{Vec2, Mat2, Bounds};

/// Calculate intersection of two shapes
pub fn intersection(a: &dyn ShapeGeometry, b: &dyn ShapeGeometry) -> Option<Vec2> {
    let a_outline = a.outline();
    let b_outline = b.outline();

    // Line-line intersection
    for i in 0..a_outline.len() {
        let a1 = a_outline[i];
        let a2 = a_outline[(i + 1) % a_outline.len()];
        for j in 0..b_outline.len() {
            let b1 = b_outline[j];
            let b2 = b_outline[(j + 1) % b_outline.len()];
            if let Some(point) = line_intersection(a1, a2, b1, b2) {
                return Some(point);
            }
        }
    }
    None
}

/// Line-line intersection
fn line_intersection(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2) -> Option<Vec2> {
    let denom = (b2.y - b1.y) * (a2.x - a1.x) - (b2.x - b1.x) * (a2.y - a1.y);
    if denom.abs() < f32::EPSILON {
        return None;
    }
    let ua = ((b2.x - b1.x) * (a1.y - b1.y) - (b2.y - b1.y) * (a1.x - b1.x)) / denom;
    let ub = ((a2.x - a1.x) * (a1.y - b1.y) - (a2.y - a1.y) * (a1.x - b1.x)) / denom;
    if (0.0..=1.0).contains(&ua) && (0.0..=1.0).contains(&ub) {
        Some(Vec2::new(
            a1.x + ua * (a2.x - a1.x),
            a1.y + ua * (a2.y - a1.y),
        ))
    } else {
        None
    }
}

/// Calculate closest point on shape to target
pub fn closest_point(shape: &dyn ShapeGeometry, target: Vec2) -> Vec2 {
    let outline = shape.outline();
    let mut closest = outline[0];
    let mut min_dist = target.distance_squared(closest);

    for &point in &outline {
        let dist = target.distance_squared(point);
        if dist < min_dist {
            min_dist = dist;
            closest = point;
        }
    }
    closest
}

/// Calculate area of shape
pub fn area(geometry: &dyn ShapeGeometry) -> f32 {
    let outline = geometry.outline();
    let mut area = 0.0;
    for i in 0..outline.len() {
        let j = (i + 1) % outline.len();
        area += outline[i].x * outline[j].y;
        area -= outline[j].x * outline[i].y;
    }
    area.abs() / 2.0
}
```

---

### 8. Rendering Pipeline

#### 8.1 GPU Rendering

```rust
// crates/we-render/src/gpu/wgpu_renderer.rs
use wgpu;
use we_canvas_core::{GraphicsContext, Shape, Camera};
use we_shapes::ShapeId;
use std::sync::Arc;

/// GPU renderer using wgpu
pub struct WgpuRenderer {
    graphics: Arc<GraphicsContext>,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

impl WgpuRenderer {
    pub fn new(graphics: Arc<GraphicsContext>) -> Result<Self, RendererError> {
        let device = graphics.device();

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Canvas Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/canvas.wgsl").into()),
        });

        // Create pipeline layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Canvas Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Canvas Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Canvas Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Create vertex/index buffers
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 1024 * 1024, // 1MB
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: 512 * 1024,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: 64, // Camera matrix
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Canvas Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Ok(Self {
            graphics,
            pipeline,
            bind_group,
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn render(&mut self, shapes: &[Arc<dyn Shape>], camera: &Camera) {
        let device = self.graphics.device();
        let queue = self.graphics.queue();
        let surface = self.graphics.surface.as_ref().unwrap();
        let frame = surface.get_current_texture().unwrap();
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);

            // Update uniform buffer with camera matrix
            let camera_matrix = camera.to_matrix();
            queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&[camera_matrix]),
            );

            // Batch and render shapes
            for shape in shapes {
                self.render_shape(&mut render_pass, shape.as_ref());
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    fn render_shape(&self, pass: &mut wgpu::RenderPass<'_>, shape: &dyn Shape) {
        // Shape-specific rendering
        // In practice, shapes would be batched for efficiency
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
    uv: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}
```

#### 8.2 Batch Rendering

```rust
// crates/we-render/src/gpu/batch.rs
use we_canvas_core::{Vec2, Color};
use we_shapes::Shape;
use std::sync::Arc;

/// Batch renderer for efficient shape rendering
pub struct BatchRenderer {
    vertices: Vec<BatchVertex>,
    indices: Vec<u32>,
    batch_size: usize,
}

impl BatchRenderer {
    pub fn new(batch_size: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(batch_size * 4),
            indices: Vec::with_capacity(batch_size * 6),
            batch_size,
        }
    }

    /// Add rectangle to batch
    pub fn add_rect(&mut self, bounds: Bounds, color: Color) {
        let base = self.vertices.len() as u32;

        self.vertices.extend_from_slice(&[
            BatchVertex {
                position: [bounds.min.x, bounds.min.y],
                color: color.to_array(),
            },
            BatchVertex {
                position: [bounds.max.x, bounds.min.y],
                color: color.to_array(),
            },
            BatchVertex {
                position: [bounds.max.x, bounds.max.y],
                color: color.to_array(),
            },
            BatchVertex {
                position: [bounds.min.x, bounds.max.y],
                color: color.to_array(),
            },
        ]);

        self.indices.extend_from_slice(&[
            base, base + 1, base + 2, // First triangle
            base, base + 2, base + 3, // Second triangle
        ]);
    }

    /// Check if batch is full
    pub fn is_full(&self) -> bool {
        self.vertices.len() >= self.batch_size * 4
    }

    /// Clear batch
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    /// Get vertices slice
    pub fn vertices(&self) -> &[BatchVertex] {
        &self.vertices
    }

    /// Get indices slice
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct BatchVertex {
    position: [f32; 2],
    color: [f32; 4],
}
```

#### 8.3 Layer Management

```rust
// crates/we-render/src/layers.rs
use std::collections::HashMap;

/// Layer management for rendering order
pub struct LayerManager {
    layers: HashMap<LayerId, Layer>,
    order: Vec<LayerId>,
}

impl LayerManager {
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn create_layer(&mut self, id: LayerId, name: String, index: usize) {
        let layer = Layer::new(id, name);
        self.layers.insert(id, layer);
        self.order.insert(index, id);
    }

    pub fn get(&self, id: LayerId) -> Option<&Layer> {
        self.layers.get(&id)
    }

    pub fn get_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.get_mut(&id)
    }

    pub fn order(&self) -> &[LayerId] {
        &self.order
    }

    /// Move layer to new index
    pub fn move_layer(&mut self, id: LayerId, new_index: usize) {
        if let Some(pos) = self.order.iter().position(|&i| i == id) {
            self.order.remove(pos);
            self.order.insert(new_index.min(self.order.len()), id);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerId(u64);

pub struct Layer {
    id: LayerId,
    name: String,
    visible: bool,
    locked: bool,
    opacity: f32,
}
```

---

### 9. Interaction System

#### 9.1 Input Handling

```rust
// crates/we-tools/src/input.rs
use winit::event::{KeyEvent, MouseButton, MouseScrollDelta};
use we_canvas_core::{Vec2, Camera};

/// Input state tracker
pub struct InputState {
    /// Current mouse position
    pub mouse_position: Vec2,
    /// Mouse buttons pressed
    pub mouse_buttons: MouseButtons,
    /// Keyboard modifiers
    pub modifiers: Modifiers,
    /// Scroll delta
    pub scroll_delta: Vec2,
    /// Keys currently pressed
    pub keys: std::collections::HashSet<winit::keyboard::KeyCode>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            mouse_position: Vec2::ZERO,
            mouse_buttons: MouseButtons::empty(),
            modifiers: Modifiers::empty(),
            scroll_delta: Vec2::ZERO,
            keys: std::collections::HashSet::new(),
        }
    }

    /// Handle keyboard event
    pub fn handle_key(&mut self, key: &KeyEvent) -> InputAction {
        match key.state {
            winit::event::ElementState::Pressed => {
                self.keys.insert(key.logical_key);
                InputAction::KeyPress(key.logical_key.clone())
            }
            winit::event::ElementState::Released => {
                self.keys.remove(&key.logical_key);
                InputAction::KeyRelease(key.logical_key.clone())
            }
        }
    }

    /// Handle mouse button event
    pub fn handle_mouse_button(
        &mut self,
        button: MouseButton,
        pressed: bool,
    ) -> InputAction {
        if pressed {
            self.mouse_buttons.insert(button);
            InputAction::Press(button)
        } else {
            self.mouse_buttons.remove(button);
            InputAction::Release(button)
        }
    }

    /// Handle mouse move
    pub fn handle_mouse_move(&mut self, position: Vec2) -> InputAction {
        let delta = position - self.mouse_position;
        self.mouse_position = position;
        InputAction::Move(position, delta)
    }

    /// Handle scroll
    pub fn handle_scroll(&mut self, delta: MouseScrollDelta) {
        self.scroll_delta = match delta {
            MouseScrollDelta::LineDelta(x, y) => Vec2::new(x * 20.0, y * 20.0),
            MouseScrollDelta::PixelDelta(d) => Vec2::new(d.x as f32, d.y as f32),
        };
    }

    /// Check if key is pressed
    pub fn is_key_pressed(&self, key: winit::keyboard::KeyCode) -> bool {
        self.keys.contains(&key)
    }

    /// Check if modifier is active
    pub fn has_modifier(&self, modifier: Modifiers) -> bool {
        self.modifiers.intersects(modifier)
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct MouseButtons: u8 {
        const LEFT = 1 << 0;
        const RIGHT = 1 << 1;
        const MIDDLE = 1 << 2;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Modifiers: u8 {
        const SHIFT = 1 << 0;
        const CONTROL = 1 << 1;
        const ALT = 1 << 2;
        const COMMAND = 1 << 3;
    }
}

pub enum InputAction {
    Press(MouseButton),
    Release(MouseButton),
    Move(Vec2, Vec2), // current, delta
    KeyPress(winit::keyboard::Key),
    KeyRelease(winit::keyboard::Key),
    Scroll(Vec2),
}
```

**Cargo.toml:**
```toml
[dependencies]
winit = "0.30"
bitflags = "2.5"
```

#### 9.2 Tool State Machine

```rust
// crates/we-tools/src/state_machine.rs
use std::sync::Arc;
use crate::{Tool, ToolContext, InputAction, InputState};

/// Tool state machine
pub struct ToolStateMachine {
    /// Current tool
    current: Arc<dyn Tool>,
    /// Previous tool (for temporary tools)
    previous: Option<Arc<dyn Tool>>,
    /// All registered tools
    tools: std::collections::HashMap<&'static str, Arc<dyn Tool>>,
}

impl ToolStateMachine {
    pub fn new() -> Self {
        Self {
            current: Arc::new(SelectTool),
            previous: None,
            tools: std::collections::HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name(), tool);
    }

    /// Get current tool
    pub fn current(&self) -> &Arc<dyn Tool> {
        &self.current
    }

    /// Switch to tool
    pub fn set_tool(&mut self, name: &'static str) {
        if let Some(tool) = self.tools.get(name) {
            self.previous = Some(Arc::clone(&self.current));
            self.current = Arc::clone(tool);
            self.current.on_enter();
        }
    }

    /// Handle input event
    pub fn handle_input(&mut self, action: InputAction, ctx: &mut ToolContext) {
        // Let current tool handle input
        let transition = self.current.handle_input(action, ctx);

        // Handle tool transitions
        if let Some(transition) = transition {
            match transition {
                ToolTransition::SetTool(name) => {
                    self.set_tool(name);
                }
                ToolTransition::Pop => {
                    if let Some(previous) = self.previous.take() {
                        self.current.on_exit();
                        self.current = previous;
                        self.current.on_enter();
                    }
                }
            }
        }
    }

    /// Update tool
    pub fn update(&mut self, ctx: &mut ToolContext) {
        self.current.update(ctx);
    }

    /// Render tool overlays
    pub fn render(&self, ctx: &mut RenderContext) {
        self.current.render(ctx);
    }
}

/// Tool transition for state machine
pub enum ToolTransition {
    SetTool(&'static str),
    Pop,
}

/// Tool trait
pub trait Tool: Send + Sync {
    /// Tool name
    fn name(&self) -> &'static str;

    /// Tool icon
    fn icon(&self) -> &'static str {
        ""
    }

    /// Called when tool is entered
    fn on_enter(&self) {}

    /// Called when tool is exited
    fn on_exit(&self) {}

    /// Handle input event
    fn handle_input(&self, action: InputAction, ctx: &mut ToolContext) -> Option<ToolTransition> {
        None
    }

    /// Update tool
    fn update(&self, ctx: &mut ToolContext) {}

    /// Render tool overlays
    fn render(&self, ctx: &mut RenderContext) {}
}
```

#### 9.3 Built-in Tools

```rust
// crates/we-tools/src/builtins/select.rs
use crate::{Tool, ToolContext, ToolTransition, InputAction, InputState};
use we_canvas_core::Vec2;

/// Selection tool
pub struct SelectTool;

impl Tool for SelectTool {
    fn name(&self) -> &'static str {
        "select"
    }

    fn icon(&self) -> &'static str {
        "cursor"
    }

    fn handle_input(&self, action: InputAction, ctx: &mut ToolContext) -> Option<ToolTransition> {
        match action {
            InputAction::Press(MouseButton::Left) => {
                // Start selection
                ctx.start_selection(ctx.input.mouse_position);
                None
            }
            InputAction::Move(_, delta) if ctx.input.mouse_buttons.contains(MouseButton::Left) => {
                // Update selection
                ctx.update_selection(delta);
                None
            }
            InputAction::Release(MouseButton::Left) => {
                // End selection
                ctx.end_selection();
                None
            }
            _ => None,
        }
    }

    fn render(&self, ctx: &mut RenderContext) {
        // Render selection box if active
        if let Some(selection) = ctx.selection {
            ctx.draw_rect(selection, Color::rgba(0, 0, 255, 0.2));
            ctx.stroke_rect(selection, Color::BLUE, 1.0);
        }
    }
}

// crates/we-tools/src/builtins/draw.rs
pub struct DrawTool;

// crates/we-tools/src/builtins/text.rs
pub struct TextTool;

// crates/we-tools/src/builtins/hand.rs
pub struct HandTool; // For panning
```

#### 9.4 Selection System

```rust
// crates/we-canvas-core/src/selection.rs
use crate::{ShapeId, Bounds, Vec2};

/// Selection state
#[derive(Debug, Default, Clone)]
pub struct SelectionState {
    /// Selected shape IDs
    pub shape_ids: Vec<ShapeId>,
    /// Selection bounds
    pub bounds: Option<Bounds>,
    /// Rotation of selection
    pub rotation: f32,
}

impl SelectionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.shape_ids.is_empty()
    }

    pub fn contains(&self, id: ShapeId) -> bool {
        self.shape_ids.contains(&id)
    }

    pub fn add(&mut self, id: ShapeId) {
        if !self.shape_ids.contains(&id) {
            self.shape_ids.push(id);
        }
    }

    pub fn remove(&mut self, id: ShapeId) {
        self.shape_ids.retain(|&i| i != id);
    }

    pub fn clear(&mut self) {
        self.shape_ids.clear();
        self.bounds = None;
    }

    pub fn toggle(&mut self, id: ShapeId) {
        if self.contains(id) {
            self.remove(id);
        } else {
            self.add(id);
        }
    }
}

/// Transform operation for selected shapes
pub enum TransformOperation {
    Translate(Vec2),
    Rotate(f32),
    Scale(Vec2, Vec2), // scale, origin
    Resize(Bounds),
}
```

---

### 10. Collaboration

#### 10.1 Yrs (Yjs Rust Port)

```rust
// crates/we-collab/src/provider.rs
use yrs::{Doc, UpdatesEncoder, UpdatesDecoder, StateVector, WriteTxn};
use std::sync::Arc;

/// Collaboration provider using Yrs
pub struct CollaborationProvider {
    /// Yrs document
    doc: Arc<Doc>,
    /// Awareness for presence
    awareness: Awareness,
    /// Network provider
    network: Box<dyn NetworkProvider>,
}

impl CollaborationProvider {
    pub fn new(room_id: String) -> Self {
        let doc = Arc::new(Doc::new());
        let awareness = Awareness::new(Arc::clone(&doc), room_id.clone());

        Self {
            doc,
            awareness,
            network: Box::new(WebSocketProvider::new(room_id)),
        }
    }

    /// Get Yrs document
    pub fn doc(&self) -> &Doc {
        &self.doc
    }

    /// Get awareness
    pub fn awareness(&self) -> &Awareness {
        &self.awareness
    }

    /// Sync with remote
    pub fn sync(&mut self) -> Result<(), CollaborationError> {
        // Get state vector from peers
        let sv = self.network.get_state_vector()?;

        // Encode missing updates
        let mut txn = self.doc.transact();
        let updates = txn.encode_state_as_update_v1(&sv);
        self.network.send_updates(updates)?;

        // Apply remote updates
        let remote_updates = self.network.get_updates()?;
        txn.apply_update_v1(&remote_updates)?;

        Ok(())
    }

    /// Subscribe to updates
    pub fn on_update<F>(&self, callback: F) -> Subscription
    where
        F: Fn(&UpdateEvent) + Send + Sync + 'static,
    {
        let sub = self.doc.observe_update_v1(move |_txn, e| {
            callback(&UpdateEvent::from(e));
        });
        Subscription::Yrs(sub)
    }
}

/// Update event
pub struct UpdateEvent {
    pub updates: Vec<u8>,
    pub origin: Option<String>,
}
```

**Cargo.toml:**
```toml
[dependencies]
yrs = "0.21"
```

#### 10.2 Awareness Protocol

```rust
// crates/we-collab/src/awareness.rs
use yrs::{Doc, Subscription};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Awareness for peer presence and cursor positions
pub struct Awareness {
    doc: Arc<Doc>,
    client_id: u64,
    room_id: String,
    local_state: AwarenessState,
    remote_states: std::collections::HashMap<u64, AwarenessState>,
}

impl Awareness {
    pub fn new(doc: Arc<Doc>, room_id: String) -> Self {
        Self {
            client_id: doc.client_id(),
            doc,
            room_id,
            local_state: AwarenessState::default(),
            remote_states: std::collections::HashMap::new(),
        }
    }

    /// Set local awareness state
    pub fn set_local_state(&mut self, state: AwarenessState) {
        self.local_state = state;
        self.broadcast();
    }

    /// Get local state
    pub fn local_state(&self) -> &AwarenessState {
        &self.local_state
    }

    /// Get all remote states
    pub fn remote_states(&self) -> &std::collections::HashMap<u64, AwarenessState> {
        &self.remote_states
    }

    /// Broadcast local state to peers
    fn broadcast(&self) {
        // Send awareness update via network
    }

    /// Receive awareness update from peer
    pub fn receive(&mut self, client_id: u64, state: AwarenessState) {
        self.remote_states.insert(client_id, state);
    }
}

/// Awareness state for a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwarenessState {
    /// User name
    pub name: String,
    /// User color
    pub color: String,
    /// Cursor position (if any)
    pub cursor: Option<CursorState>,
    /// Selection (if any)
    pub selection: Option<SelectionState>,
    /// Timestamp
    pub timestamp: u64,
}

impl Default for AwarenessState {
    fn default() -> Self {
        Self {
            name: String::new(),
            color: generate_random_color(),
            cursor: None,
            selection: None,
            timestamp: 0,
        }
    }
}

/// Cursor state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorState {
    pub x: f32,
    pub y: f32,
}

/// Selection state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionState {
    pub from: u32,
    pub to: u32,
}

fn generate_random_color() -> String {
    format!("#{:06x}", rand::random::<u32>() & 0xFFFFFF)
}
```

#### 10.3 CRDT Synchronization

```rust
// crates/we-collab/src/sync.rs
use yrs::{Doc, StateVector, Update};
use std::collections::HashMap;

/// Sync manager for CRDT synchronization
pub struct SyncManager {
    doc: Doc,
    pending_updates: Vec<Update>,
    peer_states: HashMap<u64, StateVector>,
}

impl SyncManager {
    pub fn new(doc: Doc) -> Self {
        Self {
            doc,
            pending_updates: Vec::new(),
            peer_states: HashMap::new(),
        }
    }

    /// Get update to send to peer
    pub fn get_update_for(&self, peer_id: u64, peer_sv: Option<&StateVector>) -> Update {
        let txn = self.doc.transact();
        let sv = peer_sv.unwrap_or(&StateVector::default());
        txn.encode_state_as_update_v1(sv)
    }

    /// Apply update from peer
    pub fn apply_update(&mut self, update: &[u8]) -> Result<(), yrs::Error> {
        let mut txn = self.doc.transact_mut();
        txn.apply_update_v1(update)?;
        Ok(())
    }

    /// Get current state vector
    pub fn state_vector(&self) -> StateVector {
        let txn = self.doc.transact();
        txn.state_vector()
    }

    /// Set peer state vector
    pub fn set_peer_state(&mut self, peer_id: u64, sv: StateVector) {
        self.peer_states.insert(peer_id, sv);
    }
}
```

#### 10.4 Network Layer

```rust
// crates/we-collab/src/network.rs
use async_trait::async_trait;

/// Network provider trait
#[async_trait]
pub trait NetworkProvider: Send + Sync {
    /// Connect to room
    async fn connect(&mut self) -> Result<(), NetworkError>;

    /// Disconnect from room
    async fn disconnect(&mut self);

    /// Send updates to server
    async fn send_updates(&mut self, updates: Vec<u8>) -> Result<(), NetworkError>;

    /// Receive updates from server
    async fn receive_updates(&mut self) -> Result<Vec<u8>, NetworkError>;

    /// Get state vector from peers
    async fn get_state_vector(&mut self) -> Result<StateVector, NetworkError>;

    /// Send awareness update
    async fn send_awareness(&mut self, data: Vec<u8>) -> Result<(), NetworkError>;

    /// Receive awareness update
    async fn receive_awareness(&mut self) -> Result<(u64, Vec<u8>), NetworkError>;
}

/// WebSocket network provider
pub struct WebSocketProvider {
    room_id: String,
    ws: Option<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>>,
}

#[async_trait]
impl NetworkProvider for WebSocketProvider {
    async fn connect(&mut self) -> Result<(), NetworkError> {
        // Connect to WebSocket server
        Ok(())
    }

    async fn disconnect(&mut self) {
        // Close connection
    }

    async fn send_updates(&mut self, updates: Vec<u8>) -> Result<(), NetworkError> {
        // Send via WebSocket
        Ok(())
    }

    async fn receive_updates(&mut self) -> Result<Vec<u8>, NetworkError> {
        // Receive via WebSocket
        Ok(Vec::new())
    }

    // ... other methods
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Send failed: {0}")]
    SendFailed(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Receive failed: {0}")]
    ReceiveFailed(#[from] tokio_tungstenite::tungstenite::Error),
}
```

**Cargo.toml:**
```toml
[dependencies]
async-trait = "0.1"
tokio-tungstenite = "0.24"
```

---

## Comparison Tables

### TypeScript → Rust Equivalents

#### Tiptap → Rich Text

| TypeScript Concept | Rust Equivalent | Crate |
|-------------------|-----------------|-------|
| `@tiptap/core` Editor | `Editor` | `we-core` |
| ProseMirror Node | `Arc<dyn Node>` | `we-schema` |
| ProseMirror Mark | `Mark` | `we-schema` |
| Transaction | `Transaction` | `we-core` |
| Step | `dyn Step` | `we-core` |
| Extension | `dyn Extension` | `we-extensions` |
| Node Extension | `dyn NodeExtension` | `we-extensions` |
| Mark Extension | `dyn MarkExtension` | `we-extensions` |
| Plugin | `dyn Plugin` | `we-extensions` |
| Command | `dyn Command` | `we-commands` |
| y-prosemirror | `yrs` + `we-collab` | `we-collab` |
| EditorState | `EditorState` | `we-core` |
| Selection | `Selection` | `we-core` |

#### tldraw → Canvas

| TypeScript Concept | Rust Equivalent | Crate |
|-------------------|-----------------|-------|
| `Editor` class | `Canvas` | `we-canvas-core` |
| `TLShape` | `dyn Shape` | `we-shapes` |
| `ShapeUtil` | `dyn ShapeType` | `we-shapes` |
| `Vec2D` | `Vec2` | `we-canvas-core` |
| `Box2D` | `Bounds` | `we-canvas-core` |
| `Mat2D` | `Mat2` | `we-canvas-core` |
| `TLGeoShape` | `RectangleShape` | `we-shapes` |
| `ToolState` | `ToolStateMachine` | `we-tools` |
| `useEditor` | `CanvasContext` | `we-canvas-core` |
| React rendering | `wgpu` / `egui` | `we-render` / `we-ui` |

### Dependencies Comparison

| Purpose | TypeScript | Rust |
|---------|------------|------|
| Rich Text Core | ProseMirror | Native (we-core) |
| CRDT | yjs | yrs |
| Canvas Graphics | HTML5 Canvas / WebGL | wgpu |
| SVG Rendering | SVG DOM | resvg / usvg |
| UI Framework | React | egui / Iced / Tauri |
| Async Runtime | Node.js / Browser | tokio |
| Serialization | JSON | serde_json |
| Error Handling | Throw/catch | thiserror |

---

## Complete Example Projects

### Rich Text Editor Example

```toml
# examples/rich-text-basic/Cargo.toml
[package]
name = "rich-text-basic"
version = "0.1.0"
edition = "2021"

[dependencies]
we-core = { path = "../../crates/we-core" }
we-schema = { path = "../../crates/we-schema" }
we-extensions = { path = "../../crates/we-extensions" }
we-commands = { path = "../../crates/we-commands" }
egui = "0.29"
eframe = "0.29"
serde_json = "1.0"
```

```rust
// examples/rich-text-basic/src/main.rs
use we_core::{Editor, EditorOptions, Document};
use we_schema::Schema;
use we_extensions::ExtensionRegistry;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Rich Text Editor",
        options,
        Box::new(|cc| Ok(Box::new(RichTextApp::new(cc)))),
    )
}

struct RichTextApp {
    editor: Editor,
}

impl RichTextApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Build schema
        let schema = Schema::builder()
            .node(we_extensions::nodes::DocNode)
            .node(we_extensions::nodes::ParagraphNode)
            .node(we_extensions::nodes::TextNode)
            .mark(we_extensions::marks::BoldMark)
            .mark(we_extensions::marks::ItalicMark)
            .build()
            .unwrap();

        // Create editor
        let editor = Editor::new(
            schema,
            Document::empty(),
            EditorOptions::default(),
        );

        Self { editor }
    }
}

impl eframe::App for RichTextApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Toolbar
            ui.horizontal(|ui| {
                if ui.button("B").clicked() {
                    self.editor.toggle_mark("bold");
                }
                if ui.button("I").clicked() {
                    self.editor.toggle_mark("italic");
                }
            });

            ui.separator();

            // Editor content preview
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(self.editor.get_content_text());
            });
        });
    }
}
```

### Canvas Editor Example

```toml
# examples/canvas-basic/Cargo.toml
[package]
name = "canvas-basic"
version = "0.1.0"
edition = "2021"

[dependencies]
we-canvas-core = { path = "../../crates/we-canvas-core" }
we-shapes = { path = "../../crates/we-shapes" }
we-tools = { path = "../../crates/we-tools" }
we-render = { path = "../../crates/we-render" }
winit = "0.30"
wgpu = "22"
bytemuck = { version = "1.14", features = ["derive"] }
```

```rust
// examples/canvas-basic/src/main.rs
use we_canvas_core::{Canvas, Camera, GraphicsContext, Vec2};
use we_shapes::{ShapeRegistry, RectangleShape};
use we_tools::{ToolStateMachine, SelectTool};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct CanvasApp {
    canvas: Canvas,
    graphics: Option<GraphicsContext>,
    window: Option<Window>,
}

impl CanvasApp {
    fn new() -> Self {
        let mut canvas = Canvas::new();

        // Register shapes
        let registry = ShapeRegistry::new();
        // ... register shapes

        // Setup tools
        let mut tools = ToolStateMachine::new();
        tools.register(Box::new(SelectTool));

        Self {
            canvas,
            graphics: None,
            window: None,
        }
    }
}

impl ApplicationHandler for CanvasApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window and graphics context
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(ref mut graphics) = self.graphics {
                    graphics.resize(size);
                }
            }
            WindowEvent::RedrawRequested => {
                // Render frame
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = CanvasApp::new();
    event_loop.run_app(&mut app).unwrap();
}
```

---

## Production Considerations

### Performance Optimizations

1. **Batch rendering**: Group shapes by type/material for GPU efficiency
2. **Dirty rect tracking**: Only redraw changed regions
3. **Level of detail**: Simplify shapes when zoomed out
4. **Virtual scrolling**: Only render visible shapes
5. **WebAssembly**: Compile to WASM for web deployment

### Memory Management

1. **Arc for shared state**: Editor state shared between UI and background threads
2. **Weak references**: Prevent circular references in shape hierarchies
3. **GC integration**: Use yrs built-in GC for CRDT history

### Testing Strategy

1. **Unit tests**: Test individual components (nodes, marks, shapes)
2. **Property tests**: Use proptest for document operations
3. **Integration tests**: Test full editor workflows
4. **Fuzzing**: Test transaction application with random inputs

---

## Conclusion

This Rust revision provides a comprehensive blueprint for implementing Tiptap and tldraw equivalents in Rust. The architecture maintains conceptual parity with the originals while leveraging Rust's type system, performance, and ecosystem.

Key advantages of the Rust implementation:
- **Type safety**: Compile-time guarantees for document structure
- **Performance**: Native code execution, GPU rendering
- **Concurrency**: Thread-safe state management
- **Cross-platform**: Desktop (Tauri/egui), Web (WASM), Mobile (future)

The modular crate structure allows incremental adoption and testing of components.
