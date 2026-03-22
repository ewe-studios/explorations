---
name: Wasm-Pack
description: Build tool for compiling Rust to WebAssembly and publishing to npm
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/wasm-pack/
---

# Wasm-Pack - WebAssembly Build Tool

## Overview

Wasm-Pack is the **primary build tool** for creating WebAssembly packages from Rust code. It streamlines the entire workflow from compilation to npm publishing, handling:

- Cargo compilation to WASM
- wasm-bindgen integration
- JavaScript wrapper generation
- npm package creation
- Testing in browsers and Node.js
- Documentation generation
- Publishing to npm registry

## Directory Structure

```
wasm-pack/
├── src/
│   ├── lib.rs              # Library root
│   ├── main.rs             # CLI entry point
│   ├── build/              # Build logic
│   ├── bundle/             # Bundling support
│   ├── install/            # Binary installation
│   ├── manifest/           # Cargo.toml parsing
│   ├── test/               # Testing infrastructure
│   └── wasm/               # WASM-specific logic
├── Cargo.toml
├── README.md
└── tests/
```

## Installation

```bash
# Install via cargo
cargo install wasm-pack

# Install via shell script (downloads binary)
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Install via npm (for CI/CD)
npm install -g wasm-pack
```

## Command-Line Interface

### Main Commands

```bash
# Build for web (default)
wasm-pack build

# Build for Node.js
wasm-pack build --target nodejs

# Build for bundlers (webpack, rollup, etc.)
wasm-pack build --target bundler

# Build for Deno
wasm-pack build --target deno

# Release build (optimized)
wasm-pack build --release

# Debug build with symbols
wasm-pack build --dev

# Specify output directory
wasm-pack build --out-dir ./pkg

# Run tests
wasm-pack test --headless --firefox

# Generate documentation
wasm-pack doc

# Publish to npm
wasm-pack publish
```

## Build Targets

### Web Target (Default)

```bash
wasm-pack build --target web
```

Output structure:
```
pkg/
├── package.json           # npm package manifest
├── my_crate.js            # JavaScript bindings
├── my_crate_bg.js         # Internal bindings
├── my_crate_bg.wasm       # WebAssembly binary
├── my_crate.d.ts          # TypeScript definitions
└── README.md              # Package readme
```

JavaScript usage:
```javascript
import init, { greet } from './pkg/my_crate.js';

async function run() {
    await init();
    greet("World");
}
```

### Bundler Target

```bash
wasm-pack build --target bundler
```

Designed for webpack, rollup, parcel:
```javascript
// webpack.config.js
import init, { greet } from './pkg';

init().then(() => {
    greet("World");
});
```

### Node.js Target

```bash
wasm-pack build --target nodejs
```

Produces CommonJS module:
```javascript
const { greet } = require('./pkg');
greet("World");
```

### Deno Target

```bash
wasm-pack build --target deno
```

Produces ES modules for Deno:
```typescript
import { greet } from './pkg/my_crate.js';
greet("World");
```

## Cargo.toml Configuration

### Basic Configuration

```toml
[package]
name = "my-wasm-crate"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Required for WASM

[dependencies]
wasm-bindgen = "0.2"

[dependencies.web-sys]
version = "0.3"
features = ["console", "Window", "Document"]
```

### Profile Optimization

```toml
[profile.release]
opt-level = "s"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Better optimization
panic = "abort"        # Smaller binary

[profile.dev]
opt-level = 0
debug = true
```

### Feature Flags

```toml
[features]
default = ["console_error_panic_hook"]
fuzzing = []
testing = ["wasm-bindgen-test"]

[dependencies]
console_error_panic_hook = { version = "0.1", optional = true }
wasm-bindgen-test = { version = "0.3", optional = true }
```

## Build Process

### Step-by-Step

```
┌─────────────────────────────────────────────────────────────┐
│                  wasm-pack build                            │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Check Rust toolchain (rustup target add wasm32-unknown-unknown) │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Run cargo build --target wasm32-unknown-unknown         │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Run wasm-bindgen on .wasm output                        │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  4. Generate JavaScript/TypeScript bindings                 │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  5. Generate package.json                                   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  6. Copy README, LICENSE to pkg/                            │
└─────────────────────────────────────────────────────────────┘
```

## Testing

### Browser Tests

```rust
// tests/web.rs
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_add() {
    assert_eq!(my_crate::add(2, 3), 5);
}

#[wasm_bindgen_test]
async fn test_async() {
    let result = my_crate::fetch_data().await;
    assert!(result.is_ok());
}
```

Run browser tests:
```bash
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome
wasm-pack test --headless --safari
```

### Node.js Tests

```rust
// tests/node.rs
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_node);

#[wasm_bindgen_test]
fn test_node_specific() {
    // Test Node.js specific functionality
}
```

Run Node.js tests:
```bash
wasm-pack test --node
```

## Publishing

### Prepare for Publishing

```bash
# Login to npm
npm login

# Or login to scoped registry
wasm-pack login --username myuser --password mypass
```

### Publish Package

```bash
# Public npm registry
wasm-pack publish

# Scoped package
wasm-pack publish --scope myorg

# Custom registry
wasm-pack publish --registry https://my-registry.com
```

### Package.json Configuration

```toml
# In Cargo.toml [package.metadata.wasm-pack.profile.release]
[package.metadata.wasm-pack.profile.release]
wasm-opt = ['-Os', '-mnative-functions', '-mlto']

[package.metadata.wasm-pack.profile.dev]
wasm-opt = false
```

Generated package.json:
```json
{
  "name": "my-wasm-crate",
  "version": "0.1.0",
  "files": [
    "my_crate_bg.wasm",
    "my_crate.js",
    "my_crate.d.ts"
  ],
  "module": "my_crate.js",
  "types": "my_crate.d.ts",
  "sideEffects": [
    "./my_crate.js",
    "./snippets/*"
  ]
}
```

## CI/CD Integration

### GitHub Actions

```yaml
name: WASM Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: dtolnay/rust-action@stable
      with:
        targets: wasm32-unknown-unknown

    - name: Install wasm-pack
      run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

    - name: Build
      run: wasm-pack build --release

    - name: Test
      run: wasm-pack test --headless --firefox

    - name: Upload artifact
      uses: actions/upload-artifact@v3
      with:
        name: wasm-package
        path: pkg/
```

### Publish on Release

```yaml
name: Publish WASM

on:
  release:
    types: [published]

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3

    - name: Install wasm-pack
      run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

    - name: Build
      run: wasm-pack build --release

    - name: Publish
      run: wasm-pack publish --token ${{ secrets.NPM_TOKEN }}
```

## Performance Optimization

### Binary Size Reduction

```bash
# Use wasm-opt (requires binaryen)
wasm-pack build --release

# Manual optimization
wasm-opt -Os input.wasm -o output.wasm

# Strip debug info
wasm-pack build --release -- --features console_error_panic_hook
```

### Build Time Optimization

```toml
# .cargo/config.toml
[build]
target = "wasm32-unknown-unknown"

[target.wasm32-unknown-unknown]
rustflags = ["-C", "link-arg=--strip-debug"]
```

```bash
# Use cargo cache
cargo install cargo-cache
cargo-cache

# Use sccache
cargo install sccache
export RUSTC_WRAPPER=sccache
```

## Troubleshooting

### Common Issues

```bash
# Error: target not found
rustup target add wasm32-unknown-unknown

# Error: wasm-bindgen version mismatch
cargo update -p wasm-bindgen

# Error: permissions on pkg/
rm -rf pkg/
wasm-pack build --clean

# Error: out of memory during build
CARGO_INCREMENTAL=0 wasm-pack build
```

## Integration with Build Tools

### Webpack

```javascript
// webpack.config.js
const path = require('path');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');

module.exports = {
  entry: './src/index.js',
  output: {
    path: path.resolve(__dirname, 'dist'),
  },
  plugins: [
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, 'rust'),
    }),
  ],
  experiments: {
    asyncWebAssembly: true,
  },
};
```

### Vite

```javascript
// vite.config.js
import { defineConfig } from 'vite';

export default defineConfig({
  optimizeDeps: {
    exclude: ['my-wasm-crate'],
  },
  build: {
    target: 'esnext',
  },
});
```

## Related Documents

- [Wasm-Bindgen](./wasm-bindgen-exploration.md) - Rust/JS interop
- [Gloo](./gloo-exploration.md) - Web APIs
- [Twiggy](./twiggy-exploration.md) - Profiler

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/wasm-pack/`
- Wasm-Pack Guide: https://rustwasm.github.io/wasm-pack/
- API Documentation: https://docs.rs/wasm-pack/
