---
name: create-wasm-app
description: NPM init template for kickstarting projects that use Rust-generated WebAssembly packages with Webpack bundling
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/create-wasm-app/
---

# create-wasm-app - WebAssembly App Template

## Overview

create-wasm-app is an **`npm init` template** for kickstarting projects that consume NPM packages containing Rust-generated WebAssembly. It provides a pre-configured Webpack setup for bundling WASM packages with JavaScript applications, making it easy to integrate Rust code into web projects.

Key features:
- **Zero configuration** - Pre-configured Webpack for WASM
- **NPM package consumption** - Designed for using wasm-pack published packages
- **Development server** - webpack-dev-server for hot reloading
- **Production ready** - Optimized builds out of the box
- **Simple scaffolding** - `npm init wasm-app` to start
- **Rust and WebAssembly Working Group** - Officially maintained template

## Directory Structure

```
create-wasm-app/
├── .bin/
│   └── create-wasm-app.js    # npm init binary
├── public/
│   └── index.html            # HTML template (optional)
├── .gitignore                # Ignores node_modules
├── index.html                # Basic HTML document
├── index.js                  # Entry point with WASM import example
├── package.json              # NPM configuration
├── webpack.config.js         # Webpack bundling configuration
├── LICENSE-APACHE            # Apache 2.0 license
├── LICENSE-MIT               # MIT license
└── README.md                 # Documentation
```

## Quick Start

### Create a New Project

```bash
# Using npm init
npm init wasm-app my-wasm-project
cd my-wasm-project

# Install dependencies
npm install

# Start development server
npm start

# Open browser to http://localhost:8080
```

### Project Structure After Init

```
my-wasm-project/
├── node_modules/
├── public/
│   └── index.html
├── .gitignore
├── index.html
├── index.js
├── package.json
├── package-lock.json
└── webpack.config.js
```

## Configuration

### package.json

```json
{
  "name": "my-wasm-app",
  "version": "0.1.0",
  "description": "My WebAssembly powered application",
  "main": "index.js",
  "scripts": {
    "build": "webpack --config webpack.config.js",
    "start": "webpack-dev-server"
  },
  "devDependencies": {
    "webpack": "^4.29.3",
    "webpack-cli": "^3.1.0",
    "webpack-dev-server": "^3.1.5",
    "copy-webpack-plugin": "^5.0.0"
  },
  "dependencies": {
    "hello-wasm-pack": "^0.1.0"  // Example wasm-pack package
  }
}
```

### webpack.config.js

```javascript
const path = require("path");
const CopyWebpackPlugin = require("copy-webpack-plugin");

module.exports = {
  entry: "./index.js",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "index.js",
  },
  mode: "development",
  experiments: {
    asyncWebAssembly: true,  // Enable WASM support
  },
  plugins: [
    new CopyWebpackPlugin({
      patterns: [
        { from: "index.html", to: "index.html" },
      ],
    }),
  ],
  devServer: {
    static: {
      directory: path.join(__dirname, "public"),
    },
    compress: true,
    port: 8080,
    hot: true,
  },
};
```

## Usage Patterns

### Importing a wasm-pack Package

```javascript
// index.js
import * as wasm from "hello-wasm-pack";

// Call exported functions
wasm.greet();

// Use with arguments
const result = wasm.add(2, 3);
console.log("2 + 3 =", result);

// Async usage for larger modules
async function init() {
  const wasm = await import("my-wasm-package");
  await wasm.default();
  wasm.run();
}

init();
```

### Using Multiple WASM Packages

```javascript
// index.js
import * as math from "math-wasm-package";
import * as utils from "utils-wasm-package";

// Use both packages
const result = math.calculate(utils.prepare(10));
console.log(result);
```

### Building Your Own WASM Package

```bash
# In your Rust project directory
# 1. Create wasm-pack library
cargo generate --git https://github.com/rustwasm/wasm-pack-template

# 2. Build and publish
wasm-pack build --target web
npm publish

# 3. Use in your create-wasm-app project
npm install your-wasm-package
```

## Integration with wasm-pack

### Development Workflow

```bash
# Terminal 1: Watch and build Rust code
cd my-rust-wasm-lib
wasm-pack build --target web --watch

# Terminal 2: Run web development server
cd my-wasm-app
npm start
```

### Local Development with npm link

```bash
# In your WASM package directory
cd my-wasm-package
wasm-pack build --target web
npm link

# In your app directory
cd my-wasm-app
npm link my-wasm-package
npm start
```

### Production Build

```bash
# Build optimized bundle
npm run build

# Output in dist/
# - index.js (bundled JS + WASM)
# - index.html
# - *.wasm files
```

## Modern Webpack 5 Configuration

```javascript
// webpack.config.js (Webpack 5)
const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");

module.exports = {
  entry: "./src/index.js",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "[name].[contenthash].js",
    clean: true,
  },
  mode: "production",
  experiments: {
    asyncWebAssembly: true,
    topLevelAwait: true,
  },
  plugins: [
    new HtmlWebpackPlugin({
      template: "./src/index.html",
    }),
  ],
  devServer: {
    static: "./dist",
    hot: true,
    port: 8080,
  },
  optimization: {
    minimize: true,
  },
};
```

## Vite Alternative

```javascript
// vite.config.js
import { defineConfig } from "vite";

export default defineConfig({
  build: {
    target: "esnext",
  },
  optimizeDeps: {
    exclude: ["my-wasm-package"],
  },
  server: {
    port: 3000,
  },
});
```

```json
// package.json
{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "devDependencies": {
    "vite": "^4.0.0"
  }
}
```

## Example Application

### Simple Counter with WASM

```javascript
// index.js
import * as wasm from "counter-wasm";

// Initialize WASM
wasm.init();

// Get counter value
let count = wasm.get_count();
console.log("Initial count:", count);

// Increment
wasm.increment();
count = wasm.get_count();
console.log("After increment:", count);

// Decrement
wasm.decrement();
count = wasm.get_count();
console.log("After decrement:", count);

// Reset
wasm.reset();
count = wasm.get_count();
console.log("After reset:", count);
```

### Image Processing with WASM

```javascript
// index.js
import * as img from "image-processing-wasm";

async function processImage(imageData) {
  // Initialize WASM module
  await img.init();

  // Apply filter
  const filtered = img.apply_grayscale(imageData);

  // Apply transformation
  const transformed = img.rotate(filtered, 90);

  return transformed;
}

// Usage with canvas
const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");
const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);

processImage(imageData).then(processed => {
  ctx.putImageData(processed, 0, 0);
});
```

## Troubleshooting

### WASM Not Loading

```javascript
// Check browser support
if (typeof WebAssembly === "object") {
  console.log("WebAssembly supported");
} else {
  console.log("WebAssembly NOT supported");
}
```

### MIME Type Errors

```javascript
// webpack.config.js
module.exports = {
  // ... other config
  devServer: {
    mimeTypes: {
      "text/javascript": ["js", "wasm"],
    },
  },
};
```

### Memory Issues

```javascript
// Monitor WASM memory
const wasm = await import("my-wasm-package");
const memory = wasm.memory;
console.log("Initial memory:", memory.buffer.byteLength);

// After operations
console.log("After operations:", memory.buffer.byteLength);
```

## Alternatives

### Parcel

```json
{
  "scripts": {
    "dev": "parcel index.html",
    "build": "parcel build index.html"
  },
  "devDependencies": {
    "parcel": "^2.0.0"
  }
}
```

```javascript
// Parcel handles WASM automatically
import * as wasm from "./pkg/my_wasm.js";
wasm.greet();
```

### Rollup

```javascript
// rollup.config.js
export default {
  input: "src/index.js",
  output: {
    dir: "dist",
    format: "es",
  },
  plugins: [
    wasm(),  // rollup-plugin-wasm
    serve("dist"),
  ],
};
```

## Performance Considerations

### WASM Loading Optimization

```javascript
// Lazy load WASM
let wasmInstance = null;

export async function getWasm() {
  if (!wasmInstance) {
    wasmInstance = await import("my-wasm-package");
  }
  return wasmInstance;
}

// Usage
async function heavyComputation() {
  const wasm = await getWasm();
  return wasm.compute(data);
}
```

### Streaming Compilation

```javascript
// Fetch and compile WASM in parallel
const response = fetch("my-wasm-package/my_wasm_bg.wasm");
const wasmModule = await WebAssembly.compileStreaming(response);
const instance = await WebAssembly.instantiate(wasmModule, imports);
```

## Related Documents

- [wasm-pack](./wasm-pack-exploration.md) - WASM build tooling
- [wasm-bindgen](./wasm-bindgen-exploration.md) - Rust/JS interop
- [Twiggy](./twiggy-exploration.md) - WASM size profiler

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/create-wasm-app/`
- GitHub: https://github.com/rustwasm/create-wasm-app
- npm: https://www.npmjs.com/package/create-wasm-app
