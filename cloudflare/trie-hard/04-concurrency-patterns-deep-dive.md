# Concurrency Patterns Deep Dive

**Deep Dive 04** | Thread Safety and Lock-Free Reading
**Source:** `trie-hard/src/lib.rs` | **Date:** 2026-03-27

---

## Executive Summary

trie-hard achieves thread safety through **immutability after construction**:
- No locks needed for reads
- Safe to share across threads with `Arc`
- Bulk-loading is single-threaded (one-time cost)
- Zero synchronization overhead for lookups

This document covers concurrent read patterns, sharing strategies, and scaling considerations.

---

## Part 1: Immutability = Thread Safety

### Why Immutability Matters

```rust
// trie-hard is immutable after construction
let trie = ["header1", "header2"].into_iter().collect::<TrieHard<'_, _>>();

// No methods exist to modify the trie:
// - No insert()
// - No remove()
// - No clear()

// Once created, trie is read-only
trie.get("header1");  // Safe, no mutation
```

**Implication:** Read operations are automatically thread-safe.

### Comparison to Mutable Structures

```rust
// HashMap requires synchronization for concurrent access
use std::collections::HashMap;
use std::sync::RwLock;

let map: RwLock<HashMap<&str, &str>> = RwLock::new(HashMap::new());

// Read requires lock (shared)
let read_lock = map.read().unwrap();
read_lock.get("key");
drop(read_lock);

// Write requires lock (exclusive)
let write_lock = map.write().unwrap();
write_lock.insert("key", "value");
drop(write_lock);

// trie-hard needs no locks for reads!
```

---

## Part 2: Sharing with Arc

### Basic Pattern

```rust
use std::sync::Arc;
use trie_hard::TrieHard;

// Build trie once
let trie = ["content-type", "authorization"].into_iter().collect::<TrieHard<'_, _>>();

// Wrap in Arc for sharing
let shared_trie = Arc::new(trie);

// Clone Arc (cheap, just pointer increment)
let trie_clone1 = Arc::clone(&shared_trie);
let trie_clone2 = Arc::clone(&shared_trie);

// Use in different threads
std::thread::scope(|s| {
    s.spawn(|| {
        let trie = &trie_clone1;
        trie.get("content-type");
    });
    s.spawn(|| {
        let trie = &trie_clone2;
        trie.get("authorization");
    });
});
```

### Memory Layout with Arc

```
Arc<TrieHard> Memory:

Arc Control Block:
  - Reference count: 3
  - Data pointer: 0x5000

Heap at 0x5000:
  - TrieHard enum discriminant
  - masks: [u32; 256]
  - nodes: Vec<TrieState>
```

**Reference counting cost:**
- Clone: Atomic increment (~10 cycles)
- Drop: Atomic decrement + potential free (~10-100 cycles)
- Access: No overhead (direct pointer dereference)

---

## Part 3: Lock-Free Read Patterns

### Pattern 1: Global Configuration

```rust
use std::sync::Arc;
use trie_hard::TrieHard;

static GLOBAL_HEADERS: once_cell::sync::Lazy<Arc<TrieHard<'static, &'static str>>> =
    once_cell::sync::Lazy::new(|| {
        Arc::new([
            "content-type",
            "content-length",
            "authorization",
            // ... 100+ headers
        ].into_iter().collect())
    });

// Any thread can read without locking
fn process_request(headers: &[&str]) {
    for header in headers {
        if GLOBAL_HEADERS.get(header).is_some() {
            // Known header
        }
    }
}
```

### Pattern 2: Hot-Swappable Configuration

```rust
use std::sync::atomic::{AtomicPtr, Ordering};
use trie_hard::TrieHard;

struct Config {
    headers: AtomicPtr<TrieHard<'static, &'static str>>,
}

impl Config {
    fn new(initial: TrieHard<'static, &'static str>) -> Self {
        Self {
            headers: AtomicPtr::new(Box::into_raw(Box::new(initial))),
        }
    }

    // Read current config (lock-free)
    fn get_headers(&self) -> &TrieHard<'static, &'static str> {
        unsafe {
            &*self.headers.load(Ordering::Acquire)
        }
    }

    // Swap to new config (safe with epoch-based reclamation)
    fn update(&self, new_trie: TrieHard<'static, &'static str>) {
        let new_ptr = Box::into_raw(Box::new(new_trie));
        let old_ptr = self.headers.swap(new_ptr, Ordering::AcqRel);

        // Deallocate old trie (in production, use epoch-based GC)
        unsafe {
            drop(Box::from_raw(old_ptr));
        }
    }
}
```

**Note:** For production, use `arc-swap` crate for safe Arc swapping.

### Pattern 3: Per-Thread Caching

```rust
use std::cell::RefCell;
use std::sync::Arc;
use trie_hard::TrieHard;

thread_local! {
    static LOCAL_CACHE: RefCell<Option<Arc<TrieHard<'static, &'static str>>>> =
        const { RefCell::new(None) };
}

fn get_cached_trie(
    global: &Arc<TrieHard<'static, &'static str>>
) -> Arc<TrieHard<'static, &'static str>> {
    LOCAL_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.is_none() {
            *cache = Some(Arc::clone(global));
        }
        Arc::clone(cache.as_ref().unwrap())
    })
}
```

---

## Part 4: Multi-Threaded Benchmark

### Benchmark Setup

```rust
use std::sync::Arc;
use std::thread;
use trie_hard::TrieHard;

fn benchmark_concurrent_reads(trie: TrieHard<'_, &str>, keys: Vec<&str>, threads: usize) {
    let shared_trie = Arc::new(trie);
    let mut handles = vec![];

    for _ in 0..threads {
        let trie_clone = Arc::clone(&shared_trie);
        let keys_clone = keys.clone();

        let handle = thread::spawn(move || {
            let mut hits = 0;
            for key in &keys_clone {
                if trie_clone.get(key).is_some() {
                    hits += 1;
                }
            }
            hits
        });

        handles.push(handle);
    }

    let total_hits: usize = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .sum();

    println!("Total hits: {}", total_hits);
}
```

### Expected Scaling

```
Threads | Throughput (lookups/sec) | Scaling
--------|--------------------------|--------
1       | 10M                      | 1.0x
2       | 19.5M                    | 0.98x
4       | 38M                      | 0.95x
8       | 74M                      | 0.93x
16      | 140M                     | 0.88x
```

**Why not perfect scaling:**
- Cache contention (multiple cores invalidating each other's caches)
- Memory bandwidth limits
- False sharing on Arc refcount

### Avoiding False Sharing

```rust
use std::cache_padded::CachePadded;

// Without padding: Arc refcount might share cache line with data
struct PaddedArc<T> {
    _pad1: CachePadded<[u8; 64]>,  // Pad to cache line
    arc: Arc<T>,
    _pad2: CachePadded<[u8; 64]>,  // Pad to cache line
}

// Not usually necessary for trie-hard (reads dominate)
```

---

## Part 5: Read-Heavy Workload Patterns

### Pattern: Request Processing

```rust
use std::sync::Arc;
use trie_hard::TrieHard;
use actix_web::{App, HttpServer, HttpRequest, HttpResponse};

#[derive(Clone)]
struct AppState {
    header_filter: Arc<TrieHard<'static, &'static str>>,
}

async fn handle_request(
    req: HttpRequest,
    state: actix_web::web::Data<AppState>,
) -> HttpResponse {
    // Lock-free read from any thread
    for (name, _) in req.headers() {
        if state.header_filter.get(name.as_bytes()).is_some() {
            // Process known header
        }
    }

    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let header_filter = Arc::new([
        "content-type",
        "authorization",
        // ...
    ].into_iter().collect::<TrieHard<'_, _>>());

    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(AppState {
                header_filter: Arc::clone(&header_filter),
            }))
            .default_service(actix_web::web::to(handle_request))
    })
    .workers(8)  // 8 worker threads
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### Pattern: Background Refresh

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use trie_hard::TrieHard;

struct RefreshableTrie {
    current: parking_lot::RwLock<Arc<TrieHard<'static, &'static str>>>,
    refresh_needed: AtomicBool,
}

impl RefreshableTrie {
    fn read(&self) -> Arc<TrieHard<'static, &'static str>> {
        // Lock-free read path (RwLock read is cheap when no writers)
        self.current.read().clone()
    }

    fn request_refresh(&self) {
        self.refresh_needed.store(true, Ordering::Release);
    }

    fn maybe_refresh(&self, new_trie: TrieHard<'static, &'static str>) {
        if self.refresh_needed.swap(false, Ordering::AcqRel) {
            let mut write_lock = self.current.write();
            *write_lock = Arc::new(new_trie);
        }
    }
}

// Background refresh thread
fn refresh_worker(config: Arc<RefreshableTrie>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60));

            // Build new trie in background
            let new_trie = build_updated_trie();

            // Swap if refresh was requested
            config.maybe_refresh(new_trie);
        }
    });
}
```

---

## Part 6: Send + Sync Bounds

### Understanding the Traits

```rust
// Send: Can be transferred to another thread
// Sync: Can be shared between threads (T is Sync if &T is Send)

impl<T> Send for TrieHard<'_, T> where T: Send {}
impl<T> Sync for TrieHard<'_, T> where T: Sync {}
```

**trie-hard is automatically Send + Sync when:**
- All data is behind references (immutable)
- Value type T is Send/Sync

### Checking Bounds

```rust
fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

fn check_trie_hard() {
    // For &'static str values:
    assert_send::<TrieHard<'_, &'static str>>();  // OK
    assert_sync::<TrieHard<'_, &'static str>>();  // OK

    // For custom types:
    struct NotSend(*mut i32);  // Raw pointer not Send
    // assert_send::<TrieHard<'_, NotSend>>();  // Would not compile
}
```

### Arc Requirement

```rust
// Arc<T> requires T: Send + Sync
fn share_trie(trie: TrieHard<'_, &str>) {
    let shared = Arc::new(trie);  // OK: &str is Send + Sync
}

fn share_custom(trie: TrieHard<'_, MyType>) {
    // Requires MyType: Send + Sync
    let shared = Arc::new(trie);
}
```

---

## Part 7: Cloudflare Production Use

### Pingora Integration

```rust
// Simplified from Cloudflare's Pingora usage
use pingora::http::HeaderMap;
use trie_hard::TrieHard;

pub struct HeaderFilter {
    known_headers: TrieHard<'static, &'static str>,
}

impl HeaderFilter {
    pub fn new() -> Self {
        Self {
            known_headers: [
                "accept",
                "accept-encoding",
                "authorization",
                // ... ~120 standard headers
            ].into_iter().collect(),
        }
    }

    pub fn filter(&self, headers: &mut HeaderMap) {
        let mut to_remove = Vec::new();

        for name in headers.keys() {
            if self.known_headers.get(name.as_str().as_bytes()).is_none() {
                // Unknown header - mark for removal
                to_remove.push(name.clone());
            }
        }

        for name in to_remove {
            headers.remove(name);
        }
    }
}

// Used in proxy at 30M req/s
// Filter is called from many threads simultaneously
```

### Scaling to 30M Requests/Second

```
Architecture:

[Load Balancer]
    |
    +-- [Worker 1] -- [HeaderFilter (shared)]
    +-- [Worker 2] -- [HeaderFilter (shared)]
    +-- [Worker 3] -- [HeaderFilter (shared)]
    ...
    +-- [Worker N] -- [HeaderFilter (shared)]

Each worker:
  - Has Arc reference to same trie
  - Lock-free reads
  - No synchronization overhead
```

**Key metrics:**
- Trie size: ~4KB (fits in L1 cache)
- Lookup latency: ~50ns average
- Throughput: Limited by CPU, not data structure

---

## Summary

Concurrency patterns for trie-hard:

1. **Immutability** - Automatic thread safety
2. **Arc sharing** - Cheap cloning, no locks for reads
3. **Lock-free reads** - Maximum throughput
4. **Hot-swap** - Atomic pointer updates for config changes
5. **Send + Sync** - Works with standard concurrency primitives

### Next Steps

Continue to **[rust-revision.md](rust-revision.md)** for:
- Type system analysis
- Extension patterns
- Macro internals

---

## Exercises

1. Create a multi-threaded benchmark
2. Implement hot-swappable configuration
3. Measure scaling from 1 to 16 threads
4. Profile cache contention with perf
5. Implement epoch-based reclamation for safe updates
