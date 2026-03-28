# WASM Integration Deep Dive

**Deep Dive 02** | WebAssembly and Edge Computing
**Source:** `trie-hard/src/lib.rs` | **Date:** 2026-03-27

---

## Executive Summary

trie-hard is **inherently WASM-compatible** because it:
- Has zero runtime dependencies
- Uses no async/await or threading
- Allocates memory contiguously
- Compiles to minimal bytecode

This document covers using trie-hard in WebAssembly environments, particularly Cloudflare Workers and edge computing scenarios.

---

## Part 1: Why trie-hard Works in WASM

### WASM Constraints

WebAssembly has specific limitations:

| Constraint | Impact | trie-hard Status |
|------------|--------|------------------|
| No native threads | No std::thread | ✅ No threading used |
| Limited async support | No tokio/async-std | ✅ No async runtime |
| Memory is linear array | No scattered allocations | ✅ Contiguous Vec storage |
| Size matters | Larger bytecode = slower load | ✅ ~5KB compiled |
| No filesystem | Can't load data from disk | ✅ Data embedded at compile |

### Zero Dependencies

```toml
[dependencies]
# trie-hard has NO runtime dependencies!
```

Compare to alternatives:

```toml
# radix_trie - minimal dependencies
[dependencies]
radix_trie = "0.2"
# Actually pulls in: serde, once_cell, etc.

# trie-hard
[dependencies]
trie-hard = "0.1"
# Nothing! Pure std library
```

### No Async Runtime

```rust
// This WON'T work in WASM:
use tokio::sync::Mutex;  // tokio doesn't work in WASM

// trie-hard works fine:
use trie_hard::TrieHard;  // Pure Rust, no runtime
```

---

## Part 2: Cloudflare Workers Usage

### Building for Workers

```toml
# Cargo.toml
[package]
name = "worker-trie-example"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
trie-hard = "0.1"
worker = "0.1"
```

### Basic Worker Example

```rust
use trie_hard::TrieHard;
use worker::*;

// Build trie at initialization (once per worker instance)
static HEADERS: once_cell::sync::Lazy<TrieHard<'static, &'static str>> =
    once_cell::sync::Lazy::new(|| {
        [
            "content-type",
            "content-length",
            "authorization",
            "accept",
            "accept-language",
            // ... 100+ more headers
        ]
        .into_iter()
        .collect()
    });

#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    // Fast header lookup
    let headers = req.headers()?;

    for (name, _) in headers.iter() {
        if HEADERS.get(name.as_bytes()).is_some() {
            // Known header - process it
            console_log!("Found standard header: {}", name);
        } else {
            // Custom header - maybe remove it
            console_log!("Custom header: {}", name);
        }
    }

    Response::ok("Hello from trie-hard!")
}
```

### wrangler.toml Configuration

```toml
name = "trie-worker"
main = "src/lib.rs"
compatibility_date = "2024-01-01"

[build]
command = "cargo install wasm-pack && wasm-pack build --target worker"

[vars]
LOG_LEVEL = "info"
```

---

## Part 3: Embedding Data at Compile Time

### Include Binary Data

```rust
// Build trie from embedded data
const HEADER_DATA: &[u8] = include_bytes!("../data/headers.bin");

fn build_trie() -> TrieHard<'static, u32> {
    // Parse binary format
    let mut entries = Vec::new();
    let mut offset = 0;

    while offset < HEADER_DATA.len() {
        let len = HEADER_DATA[offset] as usize;
        offset += 1;

        let header = std::str::from_utf8(&HEADER_DATA[offset..offset + len]).unwrap();
        offset += len;

        let id = u32::from_le_bytes([
            HEADER_DATA[offset],
            HEADER_DATA[offset + 1],
            HEADER_DATA[offset + 2],
            HEADER_DATA[offset + 3],
        ]);
        offset += 4;

        entries.push((header.as_bytes(), id));
    }

    TrieHard::new(entries)
}
```

### Build Script for Code Generation

```rust
// build.rs
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let path = Path::new(&out_dir).join("trie_data.rs");
    let mut file = BufWriter::new(File::create(path).unwrap());

    // Generate trie construction code
    let headers = vec![
        "content-type",
        "content-length",
        "authorization",
        // ... more headers
    ];

    writeln!(&mut file, "pub const HEADER_COUNT: usize = {};", headers.len()).unwrap();

    writeln!(&mut file, "pub fn build_headers_trie() -> TrieHard<'static, usize> {{").unwrap();
    writeln!(&mut file, "    [").unwrap();
    for (i, header) in headers.iter().enumerate() {
        writeln!(&mut file, "        {:?},", header).unwrap();
    }
    writeln!(&mut file, "    ].into_iter().collect()").unwrap();
    writeln!(&mut file, "}}").unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
```

---

## Part 4: Size Optimization

### Minimal WASM Build

```toml
# Cargo.toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
panic = "abort"      # Smaller panic handling
strip = true         # Strip symbols
```

### Build Command

```bash
# Build optimized WASM
cargo build --release --target wasm32-unknown-unknown

# Check size
ls -lh target/wasm32-unknown-unknown/release/*.wasm

# Further optimize with wasm-opt (from binaryen)
wasm-opt -Oz target/wasm32-unknown-unknown/release/your_crate.wasm \
    -o optimized.wasm
```

### Size Analysis

```bash
# Analyze WASM sections
wasm2wat optimized.wasm | grep -c "(func"   # Function count
wasm2wat optimized.wasm | grep -c "(memory" # Memory sections
```

**Expected sizes for trie-hard:**
- Raw compile: ~15KB
- With -Oz: ~8KB
- After wasm-opt: ~5KB

---

## Part 5: Memory Management in WASM

### Linear Memory Model

WASM has a single linear memory array. trie-hard's contiguous storage is ideal:

```
WASM Memory (linear array):
[0x0000] Stack space
[0x1000] Heap start
[0x1000] TrieHard struct
[0x1100] masks Vec (256 entries)
[0x1200] nodes Vec (contiguous TrieState entries)
...
```

### Pre-allocating Memory

```rust
// Reserve capacity to avoid reallocations
let mut nodes = Vec::with_capacity(estimated_size);

// For trie-hard, estimate: ~2x entry count for small tries
let estimated_nodes = entries.len() * 2;
```

### Memory Budget for Workers

Cloudflare Workers memory limit: 128MB

```rust
// Calculate memory usage
fn estimate_trie_memory(entries: usize, unique_bytes: usize) -> usize {
    let mask_size = match unique_bytes {
        ..=8 => 1,
        9..=16 => 2,
        17..=32 => 4,
        33..=64 => 8,
        65..=128 => 16,
        _ => 32,
    };

    let masks_memory = 256 * mask_size;
    let nodes_memory = entries * 2 * (mask_size + 8); // 2x for internal nodes

    masks_memory + nodes_memory
}

// For 1000 headers with 50 unique bytes:
// masks: 256 * 4 = 1KB
// nodes: 1000 * 2 * 12 = 24KB
// Total: ~25KB (well within limits)
```

---

## Part 6: Edge Computing Patterns

### Pattern 1: URL Routing

```rust
use trie_hard::TrieHard;

static ROUTES: once_cell::sync::Lazy<TrieHard<'static, RouteHandler>> =
    once_cell::sync::Lazy::new(|| {
        [
            ("/api/users", RouteHandler::Users),
            ("/api/posts", RouteHandler::Posts),
            ("/api/comments", RouteHandler::Comments),
            ("/static/css", RouteHandler::Css),
            ("/static/js", RouteHandler::Js),
        ]
        .into_iter()
        .collect()
    });

enum RouteHandler {
    Users,
    Posts,
    Comments,
    Css,
    Js,
}

fn route_request(path: &str) -> Option<RouteHandler> {
    // Find longest prefix match
    ROUTES.prefix_search(path).next().map(|(_, handler)| *handler)
}
```

### Pattern 2: Feature Flags

```rust
static FEATURE_FLAGS: once_cell::sync::Lazy<TrieHard<'static, FeatureId>> =
    once_cell::sync::Lazy::new(|| {
        [
            ("feature.new_dashboard", FeatureId::NewDashboard),
            ("feature.dark_mode", FeatureId::DarkMode),
            ("feature.beta_api", FeatureId::BetaApi),
        ]
        .into_iter()
        .collect()
    });

fn is_feature_enabled(flag: &str) -> bool {
    FEATURE_FLAGS.get(flag).is_some()
}
```

### Pattern 3: A/B Testing

```rust
struct AbTestConfig {
    variant: u8,
    weight: u8,
}

static AB_TESTS: once_cell::sync::Lazy<TrieHard<'static, AbTestConfig>> =
    once_cell::sync::Lazy::new(|| {
        [
            ("checkout_flow", AbTestConfig { variant: 1, weight: 50 }),
            ("pricing_page", AbTestConfig { variant: 2, weight: 25 }),
        ]
        .into_iter()
        .collect()
    });

fn get_ab_variant(test_name: &str, user_id: u64) -> u8 {
    if let Some(config) = AB_TESTS.get(test_name) {
        (user_id % 100) as u8 % config.weight
    } else {
        0
    }
}
```

### Pattern 4: Bot Detection

```rust
static KNOWN_BOTS: once_cell::sync::Lazy<TrieHard<'static, BotType>> =
    once_cell::sync::Lazy::new(|| {
        [
            ("Googlebot", BotType::SearchEngine),
            ("Bingbot", BotType::SearchEngine),
            ("Twitterbot", BotType::Social),
            ("facebookexternalhit", BotType::Social),
        ]
        .into_iter()
        .collect()
    });

enum BotType {
    SearchEngine,
    Social,
    Malicious,
}

fn classify_user_agent(ua: &str) -> Option<BotType> {
    // Check if user agent contains known bot signatures
    for (signature, bot_type) in KNOWN_BOTS.iter() {
        if ua.contains(signature) {
            return Some(*bot_type);
        }
    }
    None
}
```

---

## Part 7: Performance in WASM

### Benchmark Considerations

WASM performance differs from native:

| Factor | Native | WASM | Impact |
|--------|--------|------|--------|
| Memory access | Direct | Bounds-checked | +5-10% overhead |
| Branch prediction | HW assisted | Software | Slight penalty |
| count_ones() | CPU intrinsic | WASM intrinsic | Similar speed |
| Function calls | Direct | Indirect (usually) | Small overhead |

### Optimizing for WASM

```rust
// Use #[inline] for hot paths
#[inline]
fn evaluate(&self, c: u8) -> Option<usize> {
    // ...
}

// Avoid unnecessary bounds checks
fn get_byte_safe(data: &[u8], index: usize) -> Option<u8> {
    data.get(index).copied()
}

// Use wasm-opt passes
// wasm-opt -Oz --strip-debug --vacuum input.wasm -o output.wasm
```

### Measuring Performance

```rust
// Use performance.now() in Workers
use worker::console_log;
use web_time::Instant;  // Works in WASM

let start = Instant::now();

for _ in 0..10000 {
    trie.get("some-key");
}

let elapsed = start.elapsed();
console_log!("10k lookups took: {:?}", elapsed);
```

---

## Part 8: Multi-language Integration

### TypeScript/JavaScript Bindings

```rust
// lib.rs - WASM bindings
use wasm_bindgen::prelude::*;
use trie_hard::TrieHard;

#[wasm_bindgen]
pub struct HeaderFilter {
    trie: TrieHard<'static, bool>,
}

#[wasm_bindgen]
impl HeaderFilter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let trie = [
            "content-type",
            "content-length",
            "authorization",
        ]
        .into_iter()
        .collect();

        Self { trie }
    }

    #[wasm_bindgen]
    pub fn is_known_header(&self, name: &str) -> bool {
        self.trie.get(name.as_bytes()).is_some()
    }

    #[wasm_bindgen]
    pub fn filter_headers(&self, headers: Vec<String>) -> Vec<String> {
        headers
            .into_iter()
            .filter(|h| !self.is_known_header(h))
            .collect()
    }
}
```

### TypeScript Usage

```typescript
import init, { HeaderFilter } from './pkg/worker_trie.js';

await init();

const filter = new HeaderFilter();
const headers = ['content-type', 'x-custom-header', 'authorization'];

const custom = filter.filter_headers(headers);
console.log(custom);  // ['x-custom-header']
```

---

## Part 9: Deployment Checklist

### Pre-deployment

- [ ] Build with `--release --target wasm32-unknown-unknown`
- [ ] Run `wasm-opt -Oz` for size optimization
- [ ] Verify no panics (use `panic = "abort"`)
- [ ] Test with realistic data sizes
- [ ] Measure cold start time

### Cloudflare Workers Specific

- [ ] Set compatibility_date in wrangler.toml
- [ ] Configure memory limits
- [ ] Set up logging (console.log -> Cloudflare Logs)
- [ ] Test with wrangler dev locally
- [ ] Deploy with wrangler publish

### Monitoring

```rust
// Add metrics collection
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let start = Date::now();

    // Process request...
    let result = process_with_trie(&req);

    let elapsed = Date::now() - start;

    // Log latency
    console_log!("Request processed in {}ms", elapsed.as_millis());

    Ok(result)
}
```

---

## Summary

trie-hard in WASM:

1. **Zero dependencies** - Pure Rust, no runtime
2. **WASM-native** - No async/threading issues
3. **Small footprint** - ~5KB compiled
4. **Fast cold start** - No initialization overhead
5. **Edge-ready** - Perfect for Workers, Fastly, etc.

### Next Steps

Continue to **[03-performance-optimization-deep-dive.md](03-performance-optimization-deep-dive.md)** for:
- CPU cache optimization
- Branch prediction tuning
- SIMD potential
- Benchmark methodology

---

## Exercises

1. Build trie-hard for WASM target
2. Create a Cloudflare Worker using trie-hard
3. Measure and compare WASM vs native performance
4. Implement TypeScript bindings
5. Optimize WASM size below 5KB
