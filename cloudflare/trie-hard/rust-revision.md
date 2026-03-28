# Rust Revision: Complete Translation Guide

**Document 07** | Rust Patterns and Extension Guide
**Source:** `trie-hard/src/lib.rs`, `trie-hard/src/u256.rs` | **Date:** 2026-03-27

---

## Executive Summary

trie-hard is **already native Rust** - no translation needed. This document serves as:
1. Type system analysis for understanding design decisions
2. Extension patterns for adding features
3. Macro internals for the generic implementation
4. Best practices learned from the implementation

---

## Part 1: Type System Design

### The Size-Adaptive Enum

```rust
pub enum TrieHard<'a, T> {
    U8(TrieHardSized<'a, T, u8>),
    U16(TrieHardSized<'a, T, u16>),
    U32(TrieHardSized<'a, T, u32>),
    U64(TrieHardSized<'a, T, u64>),
    U128(TrieHardSized<'a, T, u128>),
    U256(TrieHardSized<'a, T, U256>),
}
```

**Design rationale:**

| Decision | Alternative | Why This Choice |
|----------|-------------|-----------------|
| Enum over generics | `TrieHard<'a, T, I: Integer>` | Auto-sizing, simpler API |
| Separate sized types | Just `TrieHardSized` | Hide complexity from users |
| Custom U256 | Use external crate | Zero dependencies, minimal ops |

### Generic Parameters Explained

```rust
pub struct TrieHardSized<'a, T, I> {
    masks: MasksByByteSized<I>,
    nodes: Vec<TrieState<'a, T, I>>,
}
```

| Parameter | Purpose | Constraints |
|-----------|---------|-------------|
| `'a` | Lifetime of borrowed data | Keys/values borrowed from input |
| `T` | Value type stored in trie | Must be `Copy` for cheap returns |
| `I` | Integer type for bitmasks | u8, u16, u32, u64, u128, or U256 |

### Why `Copy` for Values?

```rust
impl<'a, T> TrieHardSized<'a, T, $int_type>
where
    T: Copy  // Required for get() to return Option<T>
{
    pub fn get_from_bytes(&self, key: &[u8]) -> Option<T> {
        // Returns copied value, not reference
        if let TrieState::Leaf(_, value) = state {
            return Some(*value);  // Copy, not move
        }
    }
}
```

**Alternative with Clone:**
```rust
// Would require:
where T: Clone

// And return:
Some(value.clone())

// Copy is cheaper (bitwise copy, no code execution)
```

---

## Part 2: Macro Implementation

### The trie_impls! Macro

```rust
macro_rules! trie_impls {
    ($($int_type:ty),+) => {
        $(
            trie_impls!(_impl $int_type);
        )+
    };

    (_impl $int_type:ty) => {
        // All implementations for one integer type
        impl SearchNode<$int_type> { ... }
        impl<'a, T> TrieHardSized<'a, T, $int_type> where T: Copy { ... }
        impl<'a, T> TrieState<'a, T, $int_type> where T: 'a + Copy { ... }
        impl MasksByByteSized<$int_type> { ... }
        impl<'b, 'a, T> Iterator for TrieIterSized<'b, 'a, T, $int_type> where T: Copy { ... }
    }
}

// Generate for all types
trie_impls! {u8, u16, u32, u64, u128, U256}
```

**Why a macro?**
- Same logic for 6 different types
- Type parameter can't be used in associated types (limitation)
- Compile-time code generation (zero runtime cost)

### Macro Expansion Example

After expansion, `trie_impls! {u8, u16}` generates:

```rust
// For u8
impl SearchNode<u8> {
    fn evaluate<T>(&self, c: u8, trie: &TrieHardSized<'_, T, u8>) -> Option<usize> {
        // ...
    }
}

impl<'a, T> TrieHardSized<'a, T, u8> where T: Copy {
    pub fn get_from_bytes(&self, key: &[u8]) -> Option<T> {
        // ...
    }
}

// For u16
impl SearchNode<u16> {
    fn evaluate<T>(&self, c: u8, trie: &TrieHardSized<'_, T, u16>) -> Option<usize> {
        // ...
    }
}

impl<'a, T> TrieHardSized<'a, T, u16> where T: Copy {
    pub fn get_from_bytes(&self, key: &[u8]) -> Option<T> {
        // ...
    }
}

// ... repeated for all 6 types
```

---

## Part 3: Extension Patterns

### Extension 1: Adding Prefix Count

```rust
impl<'a, T, I> TrieHardSized<'a, T, I>
where
    T: Copy,
{
    /// Count how many keys start with the given prefix
    pub fn count_prefix<K: AsRef<[u8]>>(&self, prefix: K) -> usize {
        self.prefix_search(prefix).count()
    }
}
```

### Extension 2: Adding Longest Prefix Match

```rust
impl<'a, T, I> TrieHardSized<'a, T, I>
where
    T: Copy,
{
    /// Find the longest prefix of key that exists in the trie
    pub fn longest_prefix_match<K: AsRef<[u8]>>(&self, key: K) -> Option<&'a [u8]> {
        let key = key.as_ref();
        let mut state = self.nodes.get(0)?;
        let mut longest_match: Option<&'a [u8]> = None;
        let mut matched_len = 0;

        for (i, c) in key.iter().enumerate() {
            match state {
                TrieState::Leaf(k, _) => {
                    if k.starts_with(&key[..i]) {
                        longest_match = Some(k);
                    }
                    break;
                }
                TrieState::SearchOrLeaf(k, _, search) => {
                    // Current prefix is a complete word
                    longest_match = Some(&k[..]);
                    matched_len = i + 1;

                    // Continue searching for longer match
                    match search.evaluate(*c, self) {
                        Some(idx) => state = &self.nodes[idx],
                        None => break,
                    }
                }
                TrieState::Search(search) => {
                    match search.evaluate(*c, self) {
                        Some(idx) => state = &self.nodes[idx],
                        None => break,
                    }
                }
            }
        }

        longest_match
    }
}
```

**Usage (IP routing):**
```rust
let routes = [
    ("192.168.0.0", "gateway1"),
    ("192.168.1.0", "gateway2"),
    ("10.0.0.0", "gateway3"),
];
let trie = routes.into_iter().collect::<TrieHard<'_, _>>();

// Find best match for "192.168.1.50"
let best = trie.longest_prefix_match("192.168.1.50");
// Returns Some("192.168.1.0")
```

### Extension 3: Adding Fuzzy Match

```rust
impl<'a, T, I> TrieHardSized<'a, T, I>
where
    T: Copy,
{
    /// Find keys within edit distance of 1
    pub fn fuzzy_get<K: AsRef<[u8]>>(&self, key: K) -> Vec<(&'a [u8], T)> {
        let key = key.as_ref();
        let mut results = Vec::new();

        // Try single-character substitutions
        for i in 0..key.len() {
            for c in 0..=255u8 {
                if c == key[i] {
                    continue;
                }
                let mut modified = key.to_vec();
                modified[i] = c;
                if let Some(value) = self.get(&modified) {
                    results.push((modified.leak(), value));
                }
            }
        }

        // Try single-character deletions
        for i in 0..key.len() {
            let mut modified = key.to_vec();
            modified.remove(i);
            if let Some(value) = self.get(&modified) {
                results.push((modified.leak(), value));
            }
        }

        // Try single-character insertions
        for i in 0..=key.len() {
            for c in 0..=255u8 {
                let mut modified = key.to_vec();
                modified.insert(i, c);
                if let Some(value) = self.get(&modified) {
                    results.push((modified.leak(), value));
                }
            }
        }

        results
    }
}
```

### Extension 4: Adding Serialization

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SerializedTrie {
    masks: Vec<u8>,
    nodes: Vec<SerializedNode>,
}

#[derive(Serialize, Deserialize)]
enum SerializedNode {
    Leaf { key: Vec<u8>, value: Vec<u8> },
    Search { mask: Vec<u8>, edge_start: usize },
    SearchOrLeaf { key: Vec<u8>, value: Vec<u8>, mask: Vec<u8>, edge_start: usize },
}

impl<'a, T> TrieHard<'a, T>
where
    T: Serialize + DeserializeOwned + Copy,
{
    /// Serialize trie to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        // Implementation depends on value serialization
        todo!()
    }

    /// Deserialize trie from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        todo!()
    }
}
```

---

## Part 4: Lifetime Management

### Borrowed vs Owned Values

```rust
// Current: Borrowed keys and values
let trie: TrieHard<'static, &'static str> = ["a", "b"].into_iter().collect();
// 'static means data lives for entire program

// With owned values:
let trie: TrieHard<'_, String> = vec![
    ("key1".to_string(), "value1".to_string()),
    ("key2".to_string(), "value2".to_string()),
].into_iter()
    .map(|(k, v)| (k.as_bytes(), v))
    .collect::<TrieHard<'_, _>>();
// Keys owned by trie, values borrowed from input Vec
```

### Lifetime Elision

```rust
// Explicit lifetimes:
pub fn get_from_bytes<'a, 'b>(&'a self, key: &'b [u8]) -> Option<T>
where 'a: 'b  // self outlives key
{
    // ...
}

// With elision (compiler infers):
pub fn get_from_bytes(&self, key: &[u8]) -> Option<T> {
    // ...
}
```

### Arena Allocation for Owned Data

```rust
use typed_arena::Arena;

let arena = Arena::new();

// Allocate strings in arena
let entries: Vec<(&[u8], &str)> = vec![
    ("hello".to_string(), "world".to_string()),
    ("foo".to_string(), "bar".to_string()),
]
.into_iter()
.map(|(k, v)| {
    let k_ref: &'static [u8] = arena.alloc(k).as_slice();
    let v_ref: &'static str = arena.alloc(v).as_str();
    (k_ref, v_ref)
})
.collect();

// Now can build trie with 'static lifetime
let trie: TrieHard<'static, &'static str> = entries.into_iter().collect();

// Arena lives longer than trie, data valid
```

---

## Part 5: Error Handling Patterns

### Current Approach: Option Types

```rust
pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Option<T> {
    // Returns None if key not found
}
```

### Alternative: Result with Error Type

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TrieError {
    NotFound,
    InvalidKey,
}

pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<T, TrieError> {
    match self.get_from_bytes(key.as_ref()) {
        Some(value) => Ok(value),
        None => Err(TrieError::NotFound),
    }
}
```

**When to use Result:**
- When you need error context
- When combining with other fallible operations
- When using `?` operator

### Adding Debug Information

```rust
#[derive(Debug)]
pub struct GetResult<T> {
    pub found: bool,
    pub value: Option<T>,
    pub bytes_examined: usize,
    pub nodes_visited: usize,
}

pub fn get_detailed<K: AsRef<[u8]>>(&self, key: K) -> GetResult<T> {
    let key = key.as_ref();
    let mut nodes_visited = 0;

    for (i, c) in key.iter().enumerate() {
        nodes_visited += 1;
        // ... traversal logic
    }

    GetResult {
        found: value.is_some(),
        value,
        bytes_examined: key.len(),
        nodes_visited,
    }
}
```

---

## Part 6: Testing Strategies

### Property-Based Testing with proptest

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_trie_get_roundtrip(words in prop::collection::vec("[a-z]+", 1..100)) {
        let trie: TrieHard<'_, _> = words.iter().map(|s| s.as_str()).collect();

        // All original words should be found
        for word in &words {
            prop_assert!(trie.get(word.as_str()).is_some());
        }

        // Random strings should mostly not be found
        let random: String = (0..10).map(|_| char::from(b'a' + random::<u8>() % 26)).collect();
        // prop_assert!(trie.get(&random).is_none());  // May occasionally match
    }

    #[test]
    fn test_prefix_search_correctness(
        words in prop::collection::vec("[a-z]+", 1..100),
        prefix in "[a-z]{1,5}"
    ) {
        let trie: TrieHard<'_, _> = words.iter().map(|s| s.as_str()).collect();

        let results: Vec<_> = trie.prefix_search(&prefix).map(|(k, _)| k).collect();

        // All results should start with prefix
        for result in &results {
            prop_assert!(result.starts_with(&prefix));
        }

        // All words starting with prefix should be in results
        for word in &words {
            if word.starts_with(&prefix) {
                prop_assert!(results.contains(&word.as_str()));
            }
        }
    }
}
```

### Fuzz Testing

```rust
// fuzz/fuzz_targets/fuzz_get.rs
use afl::fuzz;
use trie_hard::TrieHard;

fn main() {
    fuzz(|data: &[u8]| {
        // Split data into words and queries
        let words: Vec<&str> = data
            .split(|&b| b == b'|')
            .filter_map(|s| std::str::from_utf8(s).ok())
            .collect();

        if words.is_empty() {
            return;
        }

        let trie: TrieHard<'_, _> = words.into_iter().collect();

        // Query with each word
        for word in words {
            let _ = trie.get(word);
        }
    });
}
```

---

## Part 7: Performance Optimization Patterns

### Inline Hints

```rust
#[inline]
fn evaluate(&self, c: u8, trie: &TrieHardSized<'_, T, $int_type>) -> Option<usize> {
    // Hot path - hint to inline
}

#[inline(always)]  // Force inline (use sparingly)
fn mask_lookup(byte: u8) -> u32 {
    // Very hot, small function
}

#[inline(never)]  // Prevent inline (reduce code size)
fn cold_path() {
    // Error handling, rarely taken
}
```

### Specialization (Nightly Only)

```rust
#![feature(specialization)]

impl<'a, T, I> TrieHardSized<'a, T, I>
where
    T: Copy,
{
    default fn optimize_get(key: &[u8]) -> Option<T> {
        // Generic implementation
    }
}

impl<'a, T> TrieHardSized<'a, T, u32>
where
    T: Copy,
{
    fn optimize_get(key: &[u8]) -> Option<T> {
        // Specialized for u32 (most common case)
        // Can use u32-specific optimizations
    }
}
```

### const fn for Compile-Time Construction

```rust
// Future possibility when const trait bounds are stable
const fn build_small_trie() -> TrieHard<'static, &'static str> {
    // Would allow trie construction at compile time
    todo!()
}

const MY_TRIE: TrieHard<'static, &'static str> = build_small_trie();
```

---

## Summary

Rust patterns learned from trie-hard:

1. **Size-adaptive enums** - Hide complexity, auto-select optimal type
2. **Macro-based generics** - Avoid code duplication for multiple types
3. **Copy bounds** - Efficient value returns
4. **Lifetime elision** - Clean API without explicit lifetimes
5. **Contiguous storage** - Cache-friendly data layout
6. **Zero dependencies** - Pure std library implementation

---

## Exercises

1. Implement the `count_prefix` extension
2. Add property-based tests with proptest
3. Implement serialization with serde
4. Add longest prefix match for IP routing
5. Profile the impact of `#[inline]` annotations

---

*trie-hard is production-ready Rust. Extend it following these patterns.*
