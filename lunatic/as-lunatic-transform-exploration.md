---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.lunatic/as-lunatic-transform
repository: https://github.com/lunatic-solutions/as-lunatic-transform
explored_at: 2026-03-23T00:00:00Z
language: JavaScript
---

# Project Exploration: as-lunatic-transform

## Overview

`as-lunatic-transform` is an AssemblyScript compiler transform (plugin) that automatically generates serialization methods on all classes at compile time. This is required for lunatic's message passing system, which needs to serialize/deserialize arbitrary objects when sending messages between processes.

The transform hooks into the AssemblyScript compiler's AST pipeline and injects `__lunaticSerialize` methods into every class declaration. Without this transform, developers would need to manually implement serialization for every type they want to send across process boundaries.

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.lunatic/as-lunatic-transform`
- **Remote:** `https://github.com/lunatic-solutions/as-lunatic-transform`
- **Primary Language:** JavaScript (ESM)
- **License:** MIT

## Directory Structure

```
as-lunatic-transform/
  package.json              # npm package (v0.0.0, private)
  package-lock.json         # Lock file
  index.js                  # The entire transform in ~188 lines
```

## Architecture

### How the Transform Works

The transform extends AssemblyScript's `Transform` class and operates in two phases:

#### Phase 1: `afterParse` -- AST Modification

After the AssemblyScript parser produces the AST, the transform traverses all source statements looking for `ClassDeclaration` nodes. For each class, it:

1. Collects all instance field declarations
2. Generates a `__lunaticSerialize<T>(ser: T): void` method that calls `ser.write(this.PROP, offsetof<this>("PROP"))` for each field
3. Adds a super-class delegation: `if (isDefined(super.__lunaticSerialize)) super.__lunaticSerialize(ser)`
4. Injects the generated method into the class's member list

This means every class in the program automatically knows how to serialize itself field-by-field.

#### Phase 2: `afterInitialize` -- Interface Wiring

After the compiler initializes all types, the transform:

1. Finds the `LunaticInternalTransformInterface` interface (defined in `as-lunatic`)
2. Registers all classes as implementing this interface
3. Wires up `unboundOverrides` so the compiler resolves the generated methods correctly
4. Applies the interface to built-in types: `Object`, `String`, `ArrayBuffer`, `ArrayBufferView`

### Key Design Decisions

- **Compile-time codegen**: Rather than runtime reflection (which Wasm doesn't support), serialization is generated at compile time via AST manipulation.
- **Universal application**: Every class gets serialization, not just marked ones. This ensures any type can be sent between processes without developer annotation.
- **Inheritance-aware**: The super-class check ensures serialization walks the full class hierarchy.
- **Generic serializer**: The `<T>` parameter on `__lunaticSerialize` allows different serialization backends (ASON, etc.) to be plugged in.

## Dependencies

| Dependency | Version | Purpose |
|-----------|---------|---------|
| assemblyscript | ^0.25.2 | Compiler API for AST manipulation |

## Ecosystem Role

This is a supporting tool for `as-lunatic`. Without it, AssemblyScript developers would need to manually implement serialization for every type used in inter-process messages. The transform makes the AS developer experience comparable to Rust's `#[derive(Serialize, Deserialize)]` -- but achieved through compiler-plugin AST rewriting rather than proc macros.
