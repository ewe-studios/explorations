# WASM Usage Patterns in IronCalc

## Overview

IronCalc compiles to WebAssembly to run the same spreadsheet engine in both web browsers and non-web environments. This document explores the WASM architecture, instantiation patterns, and performance considerations.

## Crate Structure

### bindings/wasm/

```toml
[package]
name = "wasm"
version = "0.1.3"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Produces .wasm file

[dependencies]
ironcalc_base = { path = "../../base", version = "0.2" }
wasm-bindgen = "0.2.92"
serde-wasm-bindgen = "0.4"
```

### web-bindings/ (Alternative)

```toml
[package]
name = "wasm"
version = "0.0.2"

[lib]
crate-type = ["cdylib"]

[dependencies]
ironcalc_base = { path = "../ironcalc/base", version = "0.1.0" }
gloo-utils = { version = "0.2.0", features = ["serde"] }
js-sys = "0.3.65"
wasm-bindgen = "0.2.88"
```

## Build Configuration

### Building for Web

```bash
# Using wasm-pack (recommended)
wasm-pack build --release --target web

# Or using cargo directly
cargo build --release --target wasm32-unknown-unknown
```

### Release Optimizations

The workspace Cargo.toml enables LTO:

```toml
[profile.release]
lto = true
```

For smaller builds:

```toml
[profile.release]
lto = true
opt-level = "z"  # Optimize for size
codegen-units = 1
```

## WASM API Design

### Model Wrapper

```rust
use wasm_bindgen::{
    prelude::{wasm_bindgen, JsError},
    JsValue,
};

use ironcalc_base::{
    expressions::{lexer::util::get_tokens as tokenizer, types::Area, utils::number_to_column},
    types::{CellType, Style},
    BorderArea, ClipboardData, UserModel as BaseModel,
};

fn to_js_error(error: String) -> JsError {
    JsError::new(&error.to_string())
}

#[wasm_bindgen]
pub struct Model {
    model: BaseModel,
}

#[wasm_bindgen]
impl Model {
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str, locale: &str, timezone: &str) -> Result<Model, JsError> {
        let model = BaseModel::new_empty(name, locale, timezone).map_err(to_js_error)?;
        Ok(Model { model })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Model, JsError> {
        let model = BaseModel::from_bytes(bytes).map_err(to_js_error)?;
        Ok(Model { model })
    }
}
```

### JavaScript Usage

```javascript
import init, { Model } from './pkg/ironcalc_wasm.js';

async function run() {
    await init();

    // Create new spreadsheet
    const model = new Model("MySheet", "en", "UTC");

    // Set cell value
    model.setUserInput(0, 1, 1, "=SUM(A1:A10)");

    // Evaluate
    model.evaluate();

    // Get result
    const value = model.getFormattedCellValue(0, 1, 1);
    console.log(value);

    // Save state
    const bytes = model.toBytes();

    // Load state later
    const model2 = Model.fromBytes(bytes);
}
```

## Instantiation Patterns

### Web Environment (Browser)

```javascript
// Modern bundler approach (Vite, Webpack)
import init, { Model, getTokens, columnNameFromNumber } from 'ironcalc-wasm';

const wasm = await init();
const model = new Model("Workbook", "en", "UTC");
```

### Web Worker

```javascript
// worker.js
import init, { Model } from './ironcalc-wasm.js';

let model = null;

self.onmessage = async (e) => {
    const { type, payload } = e.data;

    if (type === 'init') {
        await init();
        model = new Model(payload.name, payload.locale, payload.timezone);
        self.postMessage({ type: 'ready' });
    } else if (type === 'evaluate' && model) {
        model.evaluate();
        self.postMessage({ type: 'evaluated' });
    } else if (type === 'setUserInput' && model) {
        model.setUserInput(payload.sheet, payload.row, payload.column, payload.value);
        self.postMessage({ type: 'updated' });
    }
};
```

### Node.js Environment

```javascript
const { readFileSync } = require('fs');
const init = require('ironcalc-wasm');

async function runInNode() {
    const wasmBuffer = readFileSync('ironcalc_wasm_bg.wasm');
    await init(wasmBuffer);

    const { Model } = require('ironcalc-wasm');
    const model = new Model("test", "en", "UTC");

    model.setUserInput(0, 1, 1, "=42");
    model.evaluate();
    console.log(model.getFormattedCellValue(0, 1, 1));
}
```

### Server-Side (Deno/Bun)

```typescript
// Deno example
import init, { Model } from './ironcalc_wasm.js';

await init();
const model = new Model("server-sheet", "en", "UTC");
```

## Memory Management

### Binary Serialization

IronCalc uses binary serialization for efficient state transfer:

```rust
#[wasm_bindgen]
impl Model {
    /// Returns internal binary representation
    pub fn to_bytes(&self) -> Vec<u8> {
        self.model.to_bytes()
    }

    /// Restores from binary representation
    pub fn from_bytes(bytes: &[u8]) -> Result<Model, JsError> {
        let model = BaseModel::from_bytes(bytes).map_err(to_js_error)?;
        Ok(Model { model })
    }
}
```

### Usage Pattern

```javascript
// Save to IndexedDB
const bytes = model.toBytes();
const blob = new Blob([bytes], { type: 'application/octet-stream' });
await db.put('workbooks', { id: 'my-sheet', data: blob });

// Load from IndexedDB
const record = await db.get('workbooks', 'my-sheet');
const bytes = await record.data.arrayBuffer();
const model = Model.fromBytes(new Uint8Array(bytes));
```

## Evaluation Control

### Pause/Resume Pattern

For batch operations, pause evaluation to improve performance:

```rust
#[wasm_bindgen]
impl Model {
    pub fn pause_evaluation(&mut self) {
        self.model.pause_evaluation();
    }

    pub fn resume_evaluation(&mut self) {
        self.model.resume_evaluation();
    }

    pub fn evaluate(&mut self) {
        self.model.evaluate();
    }
}
```

```javascript
// Batch update pattern
model.pauseEvaluation();

// Make multiple changes
for (let row = 1; row <= 100; row++) {
    model.setUserInput(0, row, 1, `=${row} * 2`);
}

model.resumeEvaluation();
model.evaluate();  // Single evaluation pass
```

## Synchronization Patterns

### Diff-Based Sync

For collaborative editing:

```rust
#[wasm_bindgen]
impl Model {
    /// Get pending changes
    pub fn flush_send_queue(&mut self) -> Vec<u8> {
        self.model.flush_send_queue()
    }

    /// Apply remote changes
    pub fn apply_external_diffs(&mut self, diffs: &[u8]) -> Result<(), JsError> {
        self.model.apply_external_diffs(diffs).map_err(to_js_error)
    }
}
```

```javascript
// Client-side sync
async function syncWithServer(model) {
    // Get local changes
    const localDiffs = model.flushSendQueue();

    // Send to server
    const response = await fetch('/api/sync', {
        method: 'POST',
        body: localDiffs,
    });

    // Get remote changes
    const remoteDiffs = await response.arrayBuffer();

    // Apply remote changes
    model.applyExternalDiffs(new Uint8Array(remoteDiffs));
    model.evaluate();
}
```

## Platform-Specific Code

### Conditional Compilation

The base crate uses conditional compilation for WASM vs native:

```rust
// In ironcalc_base/src/model.rs

#[cfg(not(test))]
#[cfg(not(target_arch = "wasm32"))]
pub fn get_milliseconds_since_epoch() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("problem with system time")
        .as_millis() as i64
}

#[cfg(not(test))]
#[cfg(target_arch = "wasm32")]
pub fn get_milliseconds_since_epoch() -> i64 {
    use js_sys::Date;
    Date::now() as i64
}
```

### Random Number Generation

```rust
// WASM uses different RNG
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::__rt::std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use rand;
```

## Web App Integration

### React Component Pattern

```typescript
// useIronCalc.ts
import { useEffect, useRef, useCallback } from 'react';
import init, { Model } from 'ironcalc-wasm';

export function useIronCalc() {
    const modelRef = useRef<Model | null>(null);
    const initializedRef = useRef(false);

    useEffect(() => {
        async function initWasm() {
            if (!initializedRef.current) {
                await init();
                modelRef.current = new Model("Untitled", "en", "UTC");
                initializedRef.current = true;
            }
        }
        initWasm();
    }, []);

    const setCellValue = useCallback((row: number, col: number, value: string) => {
        modelRef.current?.setUserInput(0, row, col, value);
        modelRef.current?.evaluate();
    }, []);

    const getCellValue = useCallback((row: number, col: number): string => {
        return modelRef.current?.getFormattedCellValue(0, row, col) || '';
    }, []);

    return { setCellValue, getCellValue };
}
```

### Svelte Store Pattern

```typescript
// ironcalc-store.ts
import { writable } from 'svelte/store';
import init, { Model } from 'ironcalc-wasm';

async function createIronCalcStore() {
    await init();
    const model = new Model("Sheet1", "en", "UTC");

    const { subscribe, update } = writable(model);

    return {
        subscribe,
        setCellValue: (row: number, col: number, value: string) => {
            update(m => {
                m.setUserInput(0, row, col, value);
                m.evaluate();
                return m;
            });
        },
        save: async () => {
            let bytes;
            update(m => {
                bytes = m.toBytes();
                return m;
            });
            return bytes;
        }
    };
}
```

## Performance Considerations

### WASM Memory Limit

Browser WASM typically has a 2GB memory limit. For large spreadsheets:

1. Use sparse data structures (already implemented)
2. Serialize to disk/IndexDB periodically
3. Consider lazy loading for very large files

### Initialization Time

```javascript
// Preload WASM module
const wasmPromise = init();

// Later when needed
const model = new Model("Fast", "en", "UTC");
```

### Batch Operations

```javascript
// Slow: evaluate after each change
for (let i = 1; i <= 1000; i++) {
    model.setUserInput(0, i, 1, `=${i} * 2`);
    model.evaluate();  // Expensive!
}

// Fast: batch with pause
model.pauseEvaluation();
for (let i = 1; i <= 1000; i++) {
    model.setUserInput(0, i, 1, `=${i} * 2`);
}
model.evaluate();  // Single evaluation
```

## Testing WASM Code

### Rust Tests

```rust
#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_model_creation() {
        let model = Model::new("test", "en", "UTC").unwrap();
        assert_eq!(model.get_name(), "test");
    }
}
```

### JavaScript Tests

```javascript
import init, { Model } from 'ironcalc-wasm';

describe('IronCalc WASM', () => {
    beforeAll(async () => {
        await init();
    });

    test('creates model', () => {
        const model = new Model('test', 'en', 'UTC');
        expect(model.getName()).toBe('test');
    });

    test('evaluates formulas', () => {
        const model = new Model('test', 'en', 'UTC');
        model.setUserInput(0, 1, 1, '=2+2');
        model.evaluate();
        expect(model.getFormattedCellValue(0, 1, 1)).toBe('4');
    });
});
```

## Non-Web WASM Usage

### Using wasmtime (Standalone Runtime)

```rust
use wasmtime::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, "ironcalc_wasm_bg.wasm")?;
    let mut store = Store::new(&engine, ());

    let instance = Instance::new(&mut store, &module, &[])?;

    // Call exported functions through wasmtime API
    // Note: This requires additional bindings setup
    Ok(())
}
```

### wasmer Runtime

```rust
use wasmer::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wasm_bytes = std::fs::read("ironcalc_wasm_bg.wasm")?;
    let store = Store::default();
    let module = Module::new(&store, &wasm_bytes)?;
    let instance = Instance::new(&module, &imports! {})?;

    Ok(())
}
```

## Debugging WASM

### Enable Debug Symbols

```toml
[profile.dev]
debug = true
```

```bash
wasm-pack build --dev
```

### Console Logging

```rust
#[wasm_bindgen]
pub fn debug_log(message: &str) {
    web_sys::console::log_1(&message.into());
}
```

## Summary

IronCalc's WASM architecture provides:

1. **Unified codebase**: Same engine for web and native
2. **Efficient serialization**: Binary format for persistence
3. **Flexible instantiation**: Browser, Worker, Node.js support
4. **Evaluation control**: Pause/resume for batch operations
5. **Sync-ready**: Diff-based synchronization support
6. **Platform abstraction**: Conditional compilation for platform-specific code
