# Rust Revision Agent

## Purpose

Translate explored projects into idiomatic Rust, providing detailed crate breakdowns, type system design, and Rust-specific considerations.

## Prerequisites

- Requires a completed `exploration.md` from the Exploration Agent
- Understanding of the source project's architecture and goals

## Execution Steps

1. **Review Exploration Document**
   - Understand the source project's purpose and architecture
   - Identify components that map well to Rust patterns
   - Note any unsafe or system-level operations

2. **Design Crate Structure**
   - Plan workspace layout (single vs. multi-crate)
   - Define public API surface
   - Identify internal modules

3. **Select Crates**
   - Match source dependencies to Rust equivalents
   - Research crates.io for best-in-class solutions
   - Consider async runtime needs (tokio, async-std, none)

4. **Type System Design**
   - Model domain types with Rust's type system
   - Plan error types and error handling strategy
   - Design traits for abstraction boundaries

5. **Address Rust-Specific Concerns**
   - Ownership and borrowing patterns
   - Concurrency model (threads, async, actors)
   - Memory considerations (stack vs. heap, Rc/Arc)
   - Lifetimes where applicable

6. **Generate Code Examples**
   - Show idiomatic implementations of critical paths
   - Demonstrate error handling patterns
   - Provide test examples

## Output

Create `rust-revision.md` in the target directory with:

```markdown
---
source: <original project path>
repository: <git remote URL or "N/A">
revised_at: <ISO 8601 timestamp>
workspace: <planned crate workspace name>
---

# Rust Revision: <project name>

## Overview

<Summary of the translation approach and key architectural decisions>

## Workspace Structure

```
<proposed directory layout for Rust project>
```

### Crate Breakdown

#### <crate-name>
- **Purpose:** what this crate handles
- **Type:** library | binary | proc-macro
- **Public API:** main exported types/functions
- **Dependencies:** external crates it uses

<Repeat for each crate>

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| HTTP client | reqwest | 0.11 | ... |
| Serialization | serde + serde_json | 1.0 | ... |
| Async runtime | tokio | 1.0 | ... |
| Error handling | thiserror | 1.0 | ... |
| Logging | tracing | 0.1 | ... |

<Customize based on project needs>

## Type System Design

### Core Types

```rust
// Define main domain types
pub struct <TypeName> {
    // fields with Rust-specific considerations
}

// Enums for state machines or sum types
pub enum <StateEnum> {
    // variants
}
```

### Error Types

```rust
// Error handling strategy
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // error variants with context
}

pub type Result<T> = std::result::Result<T, AppError>;
```

### Traits

```rust
// Abstraction boundaries
pub trait <ServiceTrait> {
    // interface definition
}
```

## Key Rust-Specific Changes

### 1. <Change Title>

**Source Pattern:** <how it was done in original>

**Rust Translation:** <idiomatic Rust approach>

**Rationale:** <why this is better in Rust>

<Repeat for each significant change>

## Ownership & Borrowing Strategy

<Document how ownership flows through the system>

```rust
// Example ownership patterns used
```

## Concurrency Model

**Approach:** threads | async | actor | single-threaded

**Rationale:** <why this model was chosen>

```rust
// Example concurrency patterns
```

## Memory Considerations

- Stack vs. heap allocations
- Use of `Rc`, `Arc`, `Box` where applicable
- Any unsafe code requirements and justification

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| ... | ... |

## Code Examples

### Example: <Critical Component>

```rust
/// Complete, compilable example of a key component
/// with comments explaining Rust-specific choices
```

### Example: <Another Critical Component>

```rust
/// Another key implementation example
```

## Migration Path

<If incrementally migrating from source language:>

1. <Step 1>
2. <Step 2>
3. <Step 3>

## Performance Considerations

<Discuss any performance implications of the Rust translation>

## Testing Strategy

<Test approach using Rust's built-in testing, integration tests, etc.>

## Open Considerations

<Any decisions that need further thought or experimentation>
```

## Quality Criteria

- [ ] Crate structure is logical and follows Rust conventions
- [ ] Dependencies are well-researched and appropriate
- [ ] Type definitions are idiomatic Rust
- [ ] Error handling is consistent and ergonomic
- [ ] Ownership patterns are sound (could compile)
- [ ] Concurrency model is appropriate for the use case
- [ ] Code examples are complete and compilable
- [ ] Edge cases are addressed
- [ ] A Rust developer could implement from this document
