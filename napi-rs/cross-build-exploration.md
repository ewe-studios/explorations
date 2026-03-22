---
name: cross-build
description: Cross-compilation demonstration project showing how to build napi-rs modules for multiple platforms using Docker and toolchains
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.napi-rs/cross-build/
---

# Cross-Build - Cross-Compilation Example

## Overview

The `cross-build` project demonstrates how to cross-compile napi-rs native modules for multiple platforms from a single development environment. This is essential for publishing npm packages that work across Windows, macOS, Linux, and ARM architectures.

## Project Structure

```
cross-build/
├── 01-pure-rust/                # Pure Rust cross-compile example
│   ├── src/
│   │   └── lib.rs               # Rust source code
│   ├── Cargo.toml               # Rust dependencies
│   ├── package.json             # NPM configuration
│   └── README.md                # Documentation
│
├── .cargo/                      # Cargo configuration
│   └── config.toml              # Cross-compile toolchain settings
│
├── .github/                     # GitHub Actions CI/CD
│   └── workflows/
│       └── build.yml            # Multi-platform build pipeline
│
├── scripts/                     # Build helper scripts
│   ├── build-docker.sh          # Docker-based build
│   └── setup-toolchain.sh       # Toolchain installation
│
└── README.md                    # Main documentation
```

## Cross-Compilation Challenges

### Platform Targets

napi-rs supports 30+ platform targets, each requiring:
- Correct Rust toolchain (target-specific)
- Appropriate system libraries
- Platform-specific linking

| Platform | Target Triple | Notes |
|----------|--------------|-------|
| Linux x64 | `x86_64-unknown-linux-gnu` | Default, uses glibc |
| Linux x64 musl | `x86_64-unknown-linux-musl` | Static linking |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | Raspberry Pi, ARM servers |
| macOS x64 | `x86_64-apple-darwin` | Intel Macs |
| macOS ARM64 | `aarch64-apple-darwin` | Apple Silicon (M1/M2) |
| Windows x64 | `x86_64-pc-windows-msvc` | MSVC toolchain |
| Windows ARM64 | `aarch64-pc-windows-msvc` | ARM Windows devices |
| FreeBSD x64 | `x86_64-unknown-freebsd` | FreeBSD systems |

### Cross-Compilation Approaches

## Approach 1: Rustup Target + Cross

### Setup

```bash
# Add cross-compile targets
rustup target add x86_64-unknown-linux-musl
rustup target add aarch64-unknown-linux-gnu
rustup target add aarch64-apple-darwin

# Install cross for easier cross-compilation
cargo install cross
```

### Using cross

```bash
# Build for Linux musl (static)
cross build --release --target x86_64-unknown-linux-musl

# Build for ARM64 Linux
cross build --release --target aarch64-unknown-linux-gnu

# Build for Apple Silicon
cross build --release --target aarch64-apple-darwin
```

### cross Configuration

```toml
# .cargo/config.toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"

[target.x86_64-unknown-linux-musl]
linker = "musl-gcc"

[target.aarch64-apple-darwin]
linker = "osxcross"
```

## Approach 2: Docker-Based Builds

### Docker for Linux musl

```dockerfile
# Dockerfile.linux-musl
FROM rust:alpine

RUN apk add --no-cache \
    musl-dev \
    musl-tools \
    gcc \
    && rustup target add x86_64-unknown-linux-musl

WORKDIR /src
COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl
```

### Build Script

```bash
#!/bin/bash
# scripts/build-docker.sh

TARGET=$1
DOCKERFILE=$2

docker build \
  -f "$DOCKERFILE" \
  --target "$TARGET" \
  --output type=local,dest=./dist \
  .
```

### Using napi-rs CLI with Docker

```bash
# napi-rs provides pre-built Docker images
docker run --rm -v $(pwd):/src \
  ghcr.io/napi-rs/napi-rs/nodejs:18 \
  sh -c "npm install && npm run build"
```

## Approach 3: GitHub Actions Matrix Build

### Complete CI Configuration

```yaml
# .github/workflows/build.yml
name: Build

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        settings:
          - host: macos-latest
            target: x86_64-apple-darwin
            build: |
              cargo build --release
              cargo lipo --release  # For universal binary
          - host: macos-latest
            target: aarch64-apple-darwin
            build: |
              softwareupdate --install-rosetta --agree-to-license
              rustup target add aarch64-apple-darwin
              cargo build --release --target aarch64-apple-darwin
          - host: windows-latest
            target: x86_64-pc-windows-msvc
            build: cargo build --release
          - host: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            build: cargo build --release
          - host: ubuntu-latest
            target: x86_64-unknown-linux-musl
            build: |
              rustup target add x86_64-unknown-linux-musl
              cargo build --release --target x86_64-unknown-linux-musl
          - host: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            build: |
              rustup target add aarch64-unknown-linux-gnu
              sudo apt-get update && sudo apt-get install -y gcc-aarch64-linux-gnu
              cargo build --release --target aarch64-unknown-linux-gnu

    runs-on: ${{ matrix.settings.host }}
    steps:
      - uses: actions/checkout@v3

      - uses: actions/setup-node@v3
        with:
          node-version: 18

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.settings.target }}
          override: true

      - name: Build
        run: ${{ matrix.settings.build }}

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: bindings-${{ matrix.settings.target }}
          path: target/${{ matrix.settings.target }}/release/*.node
```

## Approach 4: napi-rs build CLI

### Installation

```bash
npm install -g @napi-rs/cli
```

### Build Commands

```bash
# Build for all configured targets
napi build --release

# Build for specific target
napi build --release --target x86_64-unknown-linux-musl

# Cross-compile using Docker
napi build --release --cross-compile

# Use specific Docker image
napi build --release --cross-compile --use-napi-docker

# Build and test
napi build --release --test
```

### package.json Configuration

```json
{
  "name": "my-native-module",
  "napi": {
    "name": "myAddon",
    "triples": {
      "defaults": true,
      "additional": [
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "armv7-unknown-linux-gnueabihf"
      ]
    }
  },
  "scripts": {
    "build": "napi build --release",
    "build:all": "napi build --release --platform --no-const-enum",
    "prepublishOnly": "napi prepublish"
  }
}
```

## Pure Rust Example (01-pure-rust)

### Source Code

```rust
// 01-pure-rust/src/lib.rs
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub fn fibonacci(n: u32) -> u32 {
    match n {
        1 | 2 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[napi]
pub fn factorial(n: u32) -> u64 {
    (1..=n as u64).product()
}

#[napi]
pub fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }
    for i in 2..=((n as f64).sqrt() as u32) {
        if n % i == 0 {
            return false;
        }
    }
    true
}
```

### Cargo.toml

```toml
[package]
name = "pure-rust-example"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
napi = "3"
napi-derive = "3"

[build-dependencies]
napi-build = "1"
```

### package.json

```json
{
  "name": "pure-rust-example",
  "version": "0.1.0",
  "main": "index.js",
  "types": "index.d.ts",
  "napi": {
    "name": "pureRust",
    "triples": {
      "defaults": true
    }
  },
  "scripts": {
    "build": "napi build --release",
    "test": "node test.js"
  }
}
```

### Test File

```javascript
// 01-pure-rust/test.js
const { fibonacci, factorial, isPrime } = require('./index')

console.log('fibonacci(10):', fibonacci(10))  // 55
console.log('factorial(5):', factorial(5))    // 120
console.log('isPrime(17):', isPrime(17))      // true
console.log('isPrime(18):', isPrime(18))      // false
```

## Platform-Specific Considerations

### Linux (glibc vs musl)

```toml
# For glibc (dynamic linking)
[target.x86_64-unknown-linux-gnu]

# For musl (static linking - portable)
[target.x86_64-unknown-linux-musl]
```

**Trade-offs:**
- **glibc**: Faster, but requires glibc on target system
- **musl**: Larger binary, but works everywhere

### macOS (Universal Binaries)

```bash
# Build for both Intel and Apple Silicon
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Combine into universal binary
lipo -create \
  target/x86_64-apple-darwin/release/libpure_rust_example.dylib \
  target/aarch64-apple-darwin/release/libpure_rust_example.dylib \
  -output target/universal/release/libpure_rust_example.dylib
```

### Windows (MSVC)

```bash
# Requires Visual Studio Build Tools
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

## Build Artifacts

### Output Files by Platform

| Platform | Output File |
|----------|-------------|
| Linux | `module.linux-x64-gnu.node` |
| Linux musl | `module.linux-x64-musl.node` |
| macOS x64 | `module.darwin-x64.node` |
| macOS ARM64 | `module.darwin-arm64.node` |
| Windows x64 | `module.win32-x64-msvc.node` |

### Naming Convention

```
<name>.<platform>-<arch>-<abi>.node

Examples:
- my-module.linux-x64-gnu.node
- my-module.linux-x64-musl.node
- my-module.darwin-x64.node
- my-module.darwin-arm64.node
- my-module.win32-x64-msvc.node
```

## Testing Cross-Compiled Binaries

### Local Testing

```bash
# Test on current platform
npm test

# Test with QEMU (ARM on x86)
qemu-aarch64 -L /usr/aarch64-linux-gnu target/aarch64-unknown-linux-gnu/release/my-module.node
```

### CI Testing

```yaml
test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v3
    - uses: actions/setup-node@v3
    - run: npm install
    - run: npm run build
    - run: npm test

# Test with QEMU for ARM
test-arm:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v3
    - name: Setup QEMU
      run: |
        docker run --rm --privileged multiarch/qemu-user-static --reset -p yes
    - name: Test ARM binary
      run: |
        docker run --rm -v $(pwd):/src -w /src \
          arm64v8/node:18 \
          npm test
```

## Common Issues and Solutions

### Issue 1: Missing Target

```
error: component 'rust-std' for target 'x86_64-unknown-linux-musl' is not available
```

**Solution:**
```bash
rustup target add x86_64-unknown-linux-musl
```

### Issue 2: Linker Not Found

```
error: linker 'aarch64-linux-gnu-gcc' not found
```

**Solution:**
```bash
# Ubuntu/Debian
sudo apt-get install gcc-aarch64-linux-gnu

# Or use cross
cargo install cross
cross build --target aarch64-unknown-linux-gnu
```

### Issue 3: Missing System Libraries

```
error: undefined reference to 'SSL_CTX_new'
```

**Solution:**
```dockerfile
# Add to Dockerfile
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config
```

## Best Practices

1. **Use napi-rs CLI** - Handles most cross-compile complexity automatically

2. **Test on each platform** - Don't assume code works everywhere

3. **Minimize system dependencies** - Pure Rust crates work everywhere

4. **Use musl for Linux** - More portable, no glibc version issues

5. **Version lock toolchains** - Use `rust-toolchain.toml`

6. **Cache build artifacts** - Use GitHub Actions cache

7. **Document platform support** - List supported platforms in README

## Summary

Cross-compilation with napi-rs enables:
- **Single codebase** for all platforms
- **Automated builds** via CI/CD
- **Consistent API** across platforms
- **Native performance** on each architecture

The key approaches are:
1. **Rustup + cross** for simple cases
2. **Docker** for reproducible builds
3. **GitHub Actions** for automated CI/CD
4. **napi-rs CLI** for integrated tooling
