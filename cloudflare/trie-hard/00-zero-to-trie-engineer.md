# Zero to Trie Engineer: First Principles

**Deep Dive 00** | Trie Fundamentals
**Source:** `trie-hard/src/lib.rs` | **Date:** 2026-03-27

---

## Executive Summary

This document takes you from **zero trie knowledge** to understanding how Cloudflare uses tries to filter HTTP headers at **30 million requests per second**. We build everything from scratch, explaining each concept with working code examples.

By the end, you'll understand:
- What tries are and when to use them
- Why tries beat hashmaps for certain workloads
- How trie-hard achieves its performance
- How to implement and extend trie-hard yourself

---

## Part 1: What is a Trie?

### The Basic Idea

A **trie** (pronounced "try", from "re**trie**val") is a tree-like data structure for storing strings where:
- Each node represents a **prefix** of stored strings
- Edges are labeled with **characters** (or bytes)
- The path from root to a node spells out a string

**Visual example** storing "and", "ant", "dad", "do", "dot":

```
        (root)
       /      \
      a        d
     / \      / \
    n   .    a   o
   / \  |    |   |
  d   t .    d   t
  |   |      |   |
  .   .      .   .
```

Where `.` marks the end of a complete word.

### Trie vs. Other Data Structures

Let's compare for storing a set of strings:

| Structure | Lookup | Insert | Memory | Best For |
|-----------|--------|--------|--------|----------|
| **Vec/Array** | O(n*m) | O(1) | O(n*m) | Tiny sets (< 10 items) |
| **HashMap** | O(m) avg | O(m) | O(n*m) | General purpose, high hit rate |
| **BTreeMap** | O(m*log n) | O(m*log n) | O(n*m) | Ordered iteration |
| **Trie** | O(m) | O(m) | O(n*m*alphabet) | Prefix search, fail-fast |
| **Radix Trie** | O(m) | O(m) | O(n*m) | Memory-efficient tries |

Where `n` = number of strings, `m` = max string length.

### The Key Insight: Fail Fast

The trie's superpower is **failing lookups early**:

```rust
// Searching for "dog" in ["and", "ant", "dad", "do", "dot"]

// HashMap: Must hash entire string "dog" before checking
// All 3 characters processed even though "dog" doesn't exist

// Trie: After 'd' -> 'o' -> 'g' check, fails at 3rd character
// Could fail even earlier if first character doesn't match

// For 50% miss rate, trie processes fewer bytes on average
```

---

## Part 2: Building a Trie from Scratch

### Step 1: Basic Node Structure

Let's implement a simple trie:

```rust
use std::collections::HashMap;

struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,  // Marks complete word
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            is_end: false,
        }
    }
}

struct Trie {
    root: TrieNode,
}

impl Trie {
    fn new() -> Self {
        Self { root: TrieNode::new() }
    }

    fn insert(&mut self, word: &str) {
        let mut node = &mut self.root;
        for ch in word.chars() {
            node = node.children.entry(ch).or_insert(TrieNode::new());
        }
        node.is_end = true;
    }

    fn get(&self, word: &str) -> bool {
        let mut node = &self.root;
        for ch in word.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return false,  // Fail fast!
            }
        }
        node.is_end
    }
}

// Usage
let mut trie = Trie::new();
trie.insert("and");
trie.insert("ant");
trie.insert("dad");

assert!(trie.get("and"));
assert!(!trie.get("dog"));
```

**Exercise 2.1:** Add a `prefix_search` method that returns all words starting with a given prefix.

<details>
<summary>Solution</summary>

```rust
fn prefix_search(&self, prefix: &str) -> Vec<String> {
    let mut node = &self.root;

    // Navigate to prefix node
    for ch in prefix.chars() {
        match node.children.get(&ch) {
            Some(child) => node = child,
            None => return vec![],  // Prefix not found
        }
    }

    // Collect all words from this node
    let mut results = Vec::new();
    Self::collect_words(node, prefix.to_string(), &mut results);
    results
}

fn collect_words(node: &TrieNode, current: String, results: &mut Vec<String>) {
    if node.is_end {
        results.push(current.clone());
    }
    for (ch, child) in &node.children {
        let mut next = current.clone();
        next.push(*ch);
        Self::collect_words(child, next, results);
    }
}
```

</details>

### Step 2: Problems with the Simple Approach

The naive implementation has issues:

1. **Pointer chasing:** Each node is a separate allocation
2. **HashMap overhead:** Each node has a HashMap
3. **Cache unfriendly:** Nodes scattered across heap

```
Memory layout of simple trie:

Heap:
  [TrieNode 1] -> HashMap -> [Entry 1] -> [TrieNode 2]
                              -> [Entry 2] -> [TrieNode 3]
                                                      -> [TrieNode 4]
  [TrieNode 5] -> HashMap -> ...

Every lookup jumps around memory!
```

---

## Part 3: Optimizing Memory Layout

### Contiguous Storage

Instead of scattered nodes, store everything in a `Vec`:

```rust
struct TrieNode {
    first_child: usize,    // Index into Vec
    sibling: Option<usize>, // Next sibling
    ch: char,
    is_end: bool,
}

struct Trie {
    nodes: Vec<TrieNode>,
}
```

**Better:** Store children in a contiguous range:

```rust
struct TrieNode {
    child_start: usize,   // Start index of children
    child_count: u8,      // Number of children
    ch: char,
    is_end: bool,
}
```

### Enter Bitmasks

trie-hard's innovation: encode child presence in a **bitmask**:

```rust
// Instead of HashMap<char, TrieNode>
// Use a single integer where each bit = one possible character

// Example with lowercase letters only:
// bit 0 = 'a', bit 1 = 'b', ..., bit 25 = 'z'

let node_mask: u32 = 0b00000101;  // 'a' and 'c' are valid children

fn is_valid_child(mask: u32, ch: char) -> bool {
    let bit = 1u32 << (ch as u8 - b'a');
    mask & bit != 0
}
```

**trie-hard extends this to all 256 byte values.**

---

## Part 4: The trie-hard Approach

### Byte-to-Mask Mapping

First, find all unique bytes in the input:

```rust
let words = vec!["and", "ant", "dad", "do", "dot"];

// Unique bytes: a, d, n, o, t
// Assign each a bit position:
// a -> 0b00001
// d -> 0b00010
// n -> 0b00100
// o -> 0b01000
// t -> 0b10000
```

### Node Construction

For the root node (first character position):
- Only 'a' and 'd' appear as first characters
- Root mask = `0b00011` (a + d)

```rust
struct SearchNode<I> {
    mask: I,         // Which bytes are valid
    edge_start: usize, // Where children start in node Vec
}

let root = SearchNode {
    mask: 0b00011u8,   // 'a' or 'd'
    edge_start: 1,     // Children at indices 1, 2
};
```

### The Child Index Formula

The **one weird trick** for finding which child to visit:

```rust
/// Given input byte's mask and node's mask, find child index
fn child_index(input_mask: u8, node_mask: u8) -> usize {
    // Count set bits in node_mask that are LESS significant than input_mask
    ((input_mask - 1) & node_mask).count_ones() as usize
}
```

**Why this works:**

```
Example: Find child for 'd' (mask = 0b00010) in root (mask = 0b00011)

Step 1: input_mask - 1
  0b00010 - 1 = 0b00001
  (All bits less significant than 'd' are now set)

Step 2: AND with node_mask
  0b00001 & 0b00011 = 0b00001
  (Only bits that are BOTH less significant AND present in node)

Step 3: count_ones()
  0b00001.count_ones() = 1
  (There is 1 child before 'd', so 'd' is at index 1)

Result: child_index = 1
```

**Larger example:**

```rust
// Node with children: a, d, f, h, n
// Masks:                0b00001, 0b00010, 0b00100, 0b01000, 0b10000
let node_mask = 0b0101011001u16;

// Find child index for 'h' (mask = 0b01000)
let input_mask = 0b01000u16;
let index = ((input_mask - 1) & node_mask).count_ones();
// = (0b00111 & 0b0101011001).count_ones()
// = 0b0000000011.count_ones()
// = 3
// So 'h' is the 4th child (index 3)
```

---

## Part 5: Complete Lookup Walkthrough

### Setup

```rust
let trie = ["and", "ant", "dad", "do", "dot"]
    .into_iter()
    .collect::<TrieHard<'_, _>>();
```

### Looking up "dot"

**Step 1: Get first byte 'd'**
```rust
'd' -> mask = 0b00010
root.mask = 0b00011  // 'a' and 'd' allowed

// Check if 'd' is allowed
if (root.mask & 0b00010) > 0 {  // 0b00010 > 0, YES }

// Find child index
let idx = ((0b00010 - 1) & 0b00011).count_ones();  // = 1
// Go to nodes[1]
```

**Step 2: Get second byte 'o'**
```rust
'o' -> mask = 0b01000
nodes[1].mask = 0b01000  // Only 'o' allowed (from "do", "dot")

// Check if 'o' is allowed
if (0b01000 & 0b01000) > 0 {  // YES }

// Find child index
let idx = ((0b01000 - 1) & 0b01000).count_ones();  // = 0
// Go to nodes[2]
```

**Step 3: Get third byte 't'**
```rust
't' -> mask = 0b10000
nodes[2].mask = 0b10000  // 't' marks "dot" complete

// Check if 't' is allowed
if (0b10000 & 0b10000) > 0 {  // YES }

// No more children, check if this is a complete word
// nodes[2] is marked as leaf with value "dot"
// SUCCESS!
```

### Looking up "dog" (not in trie)

**Step 1: Get first byte 'd'**
```rust
'd' -> mask = 0b00010
root.mask = 0b00011
(0b00011 & 0b00010) > 0  // OK, go to child 1
```

**Step 2: Get second byte 'o'**
```rust
'o' -> mask = 0b01000
nodes[1].mask = 0b01000
(0b01000 & 0b01000) > 0  // OK, go to child 0
```

**Step 3: Get third byte 'g'**
```rust
'g' -> mask = ???
// 'g' was never seen during construction!
// mask lookup returns 0

(0b01000 & 0) = 0  // NOT > 0
// FAIL: 'g' not allowed
// Return None immediately
```

**Fail fast achieved!** Only processed 3 bytes instead of hashing entire string.

---

## Part 6: When to Use Tries

### Perfect Use Cases for trie-hard

1. **Header filtering** (Cloudflare's use case)
   ```
   Known headers: ~120 entries
   Incoming requests: 30M/sec
   Miss rate: 50%+ (custom headers)

   Result: trie-hard beats HashMap
   ```

2. **Command prefix matching**
   ```rust
   let commands = ["help", "version", "config", "connect"];
   let trie = commands.into_iter().collect::<TrieHard<'_, _>>();

   // User types "co" -> suggest "config", "connect"
   let suggestions: Vec<_> = trie.prefix_search("co").collect();
   ```

3. **IP routing tables** (longest prefix match)
   ```rust
   // Store IP prefixes with next-hop info
   let routes = [
       ("192.168.0.0/16", "gateway1"),
       ("192.168.1.0/24", "gateway2"),
   ];
   // Lookup finds most specific match
   ```

4. **Autocomplete / Typeahead**
   ```rust
   let dictionary = load_words();
   let trie = dictionary.into_iter().collect::<TrieHard<'_, _>>();

   // As user types, narrow suggestions
   for keystroke in input {
       suggestions = trie.prefix_search(current_input);
   }
   ```

### When NOT to Use trie-hard

1. **Dynamic data** (frequent inserts/deletes)
   ```rust
   // trie-hard requires bulk rebuild
   let trie = data.collect::<TrieHard>();
   // Can't do: trie.insert(new_item)
   ```

2. **Large datasets** (> 10k entries)
   ```
   Memory grows with unique bytes * nodes
   HashMap more memory-efficient at scale
   ```

3. **Need incremental updates**
   ```rust
   // radix_trie supports this, trie-hard doesn't
   radix.insert(key, value);  // OK for radix_trie
   radix.remove(key);         // OK for radix_trie
   ```

4. **Associative lookups needed**
   ```rust
   // If you need lower_bound, upper_bound, etc.
   // Use BTreeMap instead
   ```

---

## Part 7: Performance Characteristics

### Time Complexity

| Operation | Complexity | Notes |
|-----------|------------|-------|
| `get(key)` | O(m) | m = key length in bytes |
| `prefix_search(prefix)` | O(m + k) | k = matching keys |
| `iter()` | O(n) | n = total keys |
| Construction | O(n * m) | One-time cost |

### Space Complexity

```
Space = O(unique_bytes * node_count * bits_per_mask)

For HTTP headers (119 entries, ~30 unique bytes):
- u32 masks (30 bits needed)
- ~200 nodes estimated
- Space ≈ 30 * 200 * 4 bytes ≈ 24 KB

Compare to HashMap:
- Hash + pointer per entry
- Space ≈ 119 * (8 + 8) bytes ≈ 2 KB

trie-hard uses more memory but faster for high miss rates
```

### Real Benchmark Results

From trie-hard benchmarks (10k lookups, 119 headers):

```
Hit Rate | HashMap | Radix Trie | trie-hard
---------|---------|------------|----------
100%     | 45 μs   | 38 μs      | 35 μs
50%      | 52 μs   | 32 μs      | 25 μs
10%      | 58 μs   | 28 μs      | 18 μs
1%       | 62 μs   | 25 μs      | 15 μs
```

**Key insight:** As miss rate increases, trie-hard's advantage grows because it fails fast.

---

## Part 8: Mini-Projects

### Project 1: Implement a Simple Bitmask Trie

```rust
/// Simplified trie-hard supporting only lowercase ASCII
struct SimpleTrie {
    nodes: Vec<SimpleNode>,
}

struct SimpleNode {
    mask: u32,        // 26 bits for a-z
    child_idx: usize, // Index of first child
    is_leaf: bool,
    value: Option<&'static str>,
}

impl SimpleTrie {
    fn new(words: Vec<&'static str>) -> Self {
        // TODO: Build trie from words
        todo!()
    }

    fn get(&self, key: &str) -> Option<&'static str> {
        // TODO: Lookup key in trie
        todo!()
    }
}
```

### Project 2: Add Prefix Search

Extend your SimpleTrie with:

```rust
fn prefix_search(&self, prefix: &str) -> Vec<&'static str> {
    // Navigate to prefix node
    // Collect all leaf values from that subtree
    todo!()
}
```

### Project 3: Benchmark vs HashMap

```rust
use std::collections::HashMap;
use std::time::Instant;

fn benchmark<K: AsRef<[u8]>>(
    keys: Vec<K>,
    test_keys: Vec<K>,
) {
    // Build HashMap
    let map: HashMap<_, _> = keys.iter()
        .map(|k| (k.as_ref(), k.as_ref()))
        .collect();

    // Build trie
    let trie: SimpleTrie = SimpleTrie::new(keys.iter()
        .map(|k| k.as_ref())
        .collect());

    // Benchmark HashMap
    let start = Instant::now();
    for key in &test_keys {
        map.get(key.as_ref());
    }
    let hashmap_time = start.elapsed();

    // Benchmark trie
    let start = Instant::now();
    for key in &test_keys {
        trie.get(key.as_ref());
    }
    let trie_time = start.elapsed();

    println!("HashMap: {:?}", hashmap_time);
    println!("Trie:    {:?}", trie_time);
}
```

---

## Part 9: From Simple to trie-hard

### What We're Missing

Our SimpleTrie lacks:

1. **Full byte support** (we only do a-z)
2. **Adaptive integer sizing** (we always use u32)
3. **Path compression** (we have one node per character)
4. **Bulk-loading optimization** (we need efficient construction)

### How trie-hard Solves These

1. **Full byte support** -> U256 type for up to 256 unique bytes
2. **Adaptive sizing** -> TrieHard enum with u8/u16/u32/u64/u128/U256 variants
3. **Path compression** -> Leaf nodes skip directly to end
4. **Bulk loading** -> Sorted input, BFS construction

---

## Summary

You now understand:

1. **What tries are** - Tree structures for prefix-based string storage
2. **Why tries matter** - Fail-fast lookups, prefix search capability
3. **How bitmasks work** - Encode child presence in integer bits
4. **The child index formula** - `((mask - 1) & node_mask).count_ones()`
5. **When to use trie-hard** - Small sets, high miss rate, read-only
6. **Performance trade-offs** - Faster queries, slower builds, more memory

### Next Steps

Continue to **[01-trie-structure-deep-dive.md](01-trie-structure-deep-dive.md)** for:
- Detailed node structure analysis
- U256 implementation walkthrough
- Bulk-loading algorithm
- Complete code walkthrough

---

## Glossary

| Term | Definition |
|------|------------|
| **Trie** | Tree data structure for storing strings by prefix |
| **Radix Trie** | Space-optimized trie with path compression |
| **Bitmask** | Integer where each bit represents a boolean flag |
| **count_ones()** | CPU instruction counting set bits |
| **Bulk load** | Build entire structure at once (no incremental updates) |
| **Fail fast** | Reject invalid inputs as early as possible |

---

## Exercises

1. Draw the trie structure for ["cat", "car", "card", "dog", "deer"]
2. Calculate the bitmask for the root node
3. Trace the lookup path for "card" and "cart"
4. Implement the child_index formula in your language of choice
5. Benchmark your implementation against a hash map

---

*Complete all exercises before moving to the next deep dive.*
