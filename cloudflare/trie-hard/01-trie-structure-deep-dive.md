# Trie Structure Deep Dive

**Deep Dive 01** | Node Representation and Bitmask Internals
**Source:** `trie-hard/src/lib.rs`, `trie-hard/src/u256.rs` | **Date:** 2026-03-27

---

## Executive Summary

This document dissects trie-hard's internal structure: from the U256 type through node representation, bitmask encoding, and the bulk-loading algorithm. You'll understand exactly how every line of code contributes to the final data structure.

---

## Part 1: The U256 Type

### Why U256?

A trie node needs to track which of 256 possible byte values are valid children. This requires up to 256 bits:

```
Byte values: 0x00, 0x01, ..., 0xFF (256 total)
Bits needed: 256 (one per byte value)
Max integer in std::u128: 128 bits (not enough!)
```

**Solution:** Implement a custom 256-bit integer type.

### U256 Implementation

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct U256([u64; 4]);  // 4 x 64 = 256 bits
```

**Memory layout:**
```
U256: [u64(64 bits) | u64(64 bits) | u64(64 bits) | u64(64 bits)]
       LSB                                              MSB
```

### Required Operations

U256 only implements operations needed by trie-hard:

```rust
impl U256 {
    /// Count total set bits across all 4 u64s
    pub fn count_ones(&self) -> u32 {
        self.0.iter().cloned().map(u64::count_ones).sum()
    }
}
```

**Usage:** Count how many children a node has.

### BitAnd Implementation

```rust
impl BitAnd for U256 {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self.0
            .iter_mut()
            .zip(rhs.0.iter())
            .for_each(|(l, r)| *l &= *r);
        self
    }
}
```

**Visual:**
```
  self:  [0xFF00, 0x00FF, 0x1234, 0x5678]
& rhs:   [0xF0F0, 0xFF00, 0x0000, 0xFFFF]
  result:[0xF000, 0x0000, 0x0000, 0x5678]
```

### BitOrAssign Implementation

```rust
impl BitOrAssign for U256 {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0
            .iter_mut()
            .zip(rhs.0.iter())
            .for_each(|(l, r)| *l |= r);
    }
}
```

**Usage:** Combine masks when building nodes.

### Shift Operations

```rust
impl ShlAssign<u32> for U256 {
    fn shl_assign(&mut self, rhs: u32) {
        let carry_mask = 0xFFFFFFFF_FFFFFFFF << (64_u32.overflowing_sub(rhs).0);
        let mut carry = 0;

        for p in self.0.iter_mut() {
            let next_carry = (*p & carry_mask) >> (64 - rhs);
            *p = *p << rhs | carry;
            carry = next_carry;
        }
    }
}
```

**Visual (left shift by 1):**
```
Before: [0b01, 0b01, 0b01, 0b01]
After:  [0b10, 0b10, 0b10, 0b10]
Carry propagates: LSB -> ... -> MSB
```

### Arithmetic Operations

Add/Sub with overflow handling:

```rust
impl AddAssign<u64> for U256 {
    fn add_assign(&mut self, rhs: u64) {
        let mut overflow = rhs;
        for p in self.0.iter_mut() {
            let (result, did_overflow) = p.overflowing_add(overflow);
            *p = result;
            overflow = did_overflow as u64;
        }
    }
}
```

**Usage:** Incrementing mask values during construction.

### Comparison with u64

```rust
impl PartialEq<u64> for U256 {
    fn eq(&self, other: &u64) -> bool {
        self.0[0] == *other && self.0.iter().skip(1).all(|p| *p == 0)
    }
}

impl PartialOrd<u64> for U256 {
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        if self.0.iter().skip(1).any(|p| *p > 0) {
            Some(Ordering::Greater)
        } else {
            Some(self.0[0].cmp(other))
        }
    }
}
```

**Usage:** Check if mask is zero (no children) or compare with zero.

---

## Part 2: MasksByByte - The Lookup Table

### Purpose

Convert any byte value (0-255) to its assigned bitmask:

```rust
#[derive(Debug, Clone)]
#[repr(transparent)]
struct MasksByByteSized<I>([I; 256]);
```

**Memory:** 256 entries x I bytes per entry

### Construction

```rust
impl<I> Default for MasksByByteSized<I>
where
    I: Default + Copy,
{
    fn default() -> Self {
        Self([I::default(); 256])
    }
}
```

### Mask Assignment Algorithm

```rust
impl MasksByByteSized<$int_type> {
    fn new(used_bytes: BTreeSet<u8>) -> Self {
        let mut mask = Default::default();
        mask += 1;  // Start with 0b0001

        let mut byte_masks = [Default::default(); 256];

        for c in used_bytes.into_iter() {
            byte_masks[c as usize] = mask;  // Assign current mask to byte
            mask <<= 1;  // Shift for next byte
        }

        Self(byte_masks)
    }
}
```

**Example:**
```
Input used_bytes: {b'a', b'd', b'n', b'o', b't'}

Iteration:
  c = b'a' (97): byte_masks[97] = 0b00001, mask <<= 1 -> 0b00010
  c = b'd' (100): byte_masks[100] = 0b00010, mask <<= 1 -> 0b00100
  c = b'n' (110): byte_masks[110] = 0b00100, mask <<= 1 -> 0b01000
  c = b'o' (111): byte_masks[111] = 0b01000, mask <<= 1 -> 0b10000
  c = b't' (116): byte_masks[116] = 0b10000, mask <<= 1 -> 0b100000

Result: byte_masks[97] = 0b00001, byte_masks[100] = 0b00010, etc.
```

### Adaptive Sizing

The enum selects the right integer type:

```rust
enum MasksByByte {
    U8(MasksByByteSized<u8>),    // 1-8 unique bytes
    U16(MasksByByteSized<u16>),  // 9-16 unique bytes
    U32(MasksByByteSized<u32>),  // 17-32 unique bytes
    U64(MasksByByteSized<u64>),  // 33-64 unique bytes
    U128(MasksByByteSized<u128>),// 65-128 unique bytes
    U256(MasksByByteSized<U256>),// 129-256 unique bytes
}

impl MasksByByte {
    fn new(used_bytes: BTreeSet<u8>) -> Self {
        match used_bytes.len() {
            ..=8 => MasksByByte::U8(...),
            9..=16 => MasksByByte::U16(...),
            17..=32 => MasksByByte::U32(...),
            33..=64 => MasksByByte::U64(...),
            65..=128 => MasksByByte::U128(...),
            129..=256 => MasksByByte::U256(...),
            _ => unreachable!(),
        }
    }
}
```

**Why adaptive?** Memory efficiency:
- HTTP headers (~30 unique bytes) -> u32 (4 bytes per entry)
- Full ASCII (128 bytes) -> u128 (16 bytes per entry)
- Don't waste space with u256 when u32 suffices

---

## Part 3: TrieState - Node Representation

### The Three Node Types

```rust
enum TrieState<'a, T, I> {
    Leaf(&'a [u8], T),                    // Complete word with value
    Search(SearchNode<I>),                // Internal node, keep searching
    SearchOrLeaf(&'a [u8], T, SearchNode<I>), // Both! (prefix is word AND has children)
}
```

**Why three variants?**

1. **Leaf:** Word ends here (e.g., "do" in ["do", "dot"])
2. **Search:** Word continues (e.g., after "d" in ["dad", "do", "dot"])
3. **SearchOrLeaf:** Word ends here AND continues (e.g., "do" in ["do", "dot"])

### SearchNode Structure

```rust
struct SearchNode<I> {
    mask: I,         // Which bytes are valid children
    edge_start: usize, // Index of first child in nodes Vec
}
```

**Memory optimization:** Children stored in contiguous range `[edge_start, edge_start + mask.count_ones())`.

---

## Part 4: The Bulk-Loading Algorithm

### Overview

```rust
fn new(masks: MasksByByteSized<$int_type>, values: Vec<(&'a [u8], T)>) -> Self {
    // 1. Sort values
    let sorted = values.into_iter().collect::<BTreeMap<_, _>>();

    // 2. Initialize node vector
    let mut nodes = Vec::new();
    let mut next_index = 1;  // Root is at index 0, children start at 1

    // 3. BFS construction with queue
    let mut spec_queue = VecDeque::new();
    spec_queue.push_back(StateSpec { prefix: &[], index: 0 });

    while let Some(spec) = spec_queue.pop_front() {
        let (state, next_specs) = TrieState::new(spec, next_index, &masks.0, &sorted);
        next_index += next_specs.len();
        spec_queue.extend(next_specs);
        nodes.push(state);
    }

    TrieHardSized { nodes, masks }
}
```

### Step-by-Step Construction

**Input:** `["and", "ant", "dad", "do", "dot"]`

**Step 1: Sort and collect used bytes**
```rust
sorted = {
    "and": "and",
    "ant": "ant",
    "dad": "dad",
    "do": "do",
    "dot": "dot"
}

used_bytes = {b'a', b'd', b'n', b'o', b't'}
```

**Step 2: Assign masks**
```rust
byte_masks[b'a'] = 0b00001
byte_masks[b'd'] = 0b00010
byte_masks[b'n'] = 0b00100
byte_masks[b'o'] = 0b01000
byte_masks[b't'] = 0b10000
```

**Step 3: Build root node (prefix = "")**
```rust
// Find all words starting with ""
// First characters: 'a' (and, ant), 'd' (dad, do, dot)

root_mask = byte_masks[b'a'] | byte_masks[b'd']
          = 0b00001 | 0b00010
          = 0b00011

children = [
    index 1: prefix "a" (and, ant)
    index 2: prefix "d" (dad, do, dot)
]

root = Search(SearchNode {
    mask: 0b00011,
    edge_start: 1
})
```

**Step 4: Build node at index 1 (prefix = "a")**
```rust
// Words starting with "a": "and", "ant"
// Next characters: 'n' (both)

node_mask = byte_masks[b'n']
          = 0b00100

children = [
    index 3: prefix "an" (and, ant)
]

node[1] = Search(SearchNode {
    mask: 0b00100,
    edge_start: 3
})
```

**Step 5: Build node at index 2 (prefix = "d")**
```rust
// Words starting with "d": "dad", "do", "dot"
// Next characters: 'a' (dad), 'o' (do, dot)

node_mask = byte_masks[b'a'] | byte_masks[b'o']
          = 0b00001 | 0b01000
          = 0b01001

children = [
    index 4: prefix "da" (dad)
    index 5: prefix "do" (do, dot)
]

node[2] = Search(SearchNode {
    mask: 0b01001,
    edge_start: 4
})
```

**Step 6: Build node at index 3 (prefix = "an")**
```rust
// Words starting with "an": "and", "ant"
// Next characters: 'd' (and), 't' (ant)

node_mask = byte_masks[b'd'] | byte_masks[b't']
          = 0b00010 | 0b10000
          = 0b10010

children = [
    index 6: prefix "and"
    index 7: prefix "ant"
]

node[3] = Search(SearchNode {
    mask: 0b10010,
    edge_start: 6
})
```

**Step 7: Build node at index 4 (prefix = "da")**
```rust
// Words starting with "da": "dad"
// Only one match -> leaf!

node[4] = Leaf("dad", "dad")
```

**Step 8: Build node at index 5 (prefix = "do")**
```rust
// Words starting with "do": "do", "dot"
// "do" is complete word AND has child "dot"

node_mask = byte_masks[b't']
          = 0b10000

children = [
    index 8: prefix "dot"
]

node[5] = SearchOrLeaf("do", "do", SearchNode {
    mask: 0b10000,
    edge_start: 8
})
```

**Step 9: Build remaining leaf nodes**
```rust
node[6] = Leaf("and", "and")
node[7] = Leaf("ant", "ant")
node[8] = Leaf("dot", "dot")
```

**Final structure:**
```
nodes Vec (contiguous):
[0] Search { mask: 0b00011, edge_start: 1 }     // root
[1] Search { mask: 0b00100, edge_start: 3 }     // "a"
[2] Search { mask: 0b01001, edge_start: 4 }     // "d"
[3] Search { mask: 0b10010, edge_start: 6 }     // "an"
[4] Leaf("dad", "dad")                          // "dad"
[5] SearchOrLeaf("do", "do", ...)               // "do"
[6] Leaf("and", "and")                          // "and"
[7] Leaf("ant", "ant")                          // "ant"
[8] Leaf("dot", "dot")                          // "dot"
```

---

## Part 5: The Lookup Algorithm

### Core Lookup Function

```rust
pub fn get_from_bytes(&self, key: &[u8]) -> Option<T> {
    let mut state = self.nodes.get(0)?;  // Start at root

    for (i, c) in key.iter().enumerate() {
        let next_state_opt = match state {
            TrieState::Leaf(k, value) => {
                // Leaf: compare remaining bytes directly
                return (k.len() == key.len() && k[i..] == key[i..])
                    .then_some(*value);
            }
            TrieState::Search(search)
            | TrieState::SearchOrLeaf(_, _, search) => {
                search.evaluate(*c, self)
            }
        };

        if let Some(next_state_index) = next_state_opt {
            state = &self.nodes[next_state_index];
        } else {
            return None;  // Byte not allowed
        }
    }

    // End of key: only match if at leaf or SearchOrLeaf with exact length
    if let TrieState::Leaf(k, value)
        | TrieState::SearchOrLeaf(k, value, _) = state
    {
        (k.len() == key.len()).then_some(*value)
    } else {
        None  // Prefix match, not exact match
    }
}
```

### SearchNode::evaluate

```rust
fn evaluate<T>(&self, c: u8, trie: &TrieHardSized<'_, T, $int_type>) -> Option<usize> {
    let c_mask = trie.masks.0[c as usize];  // Get mask for byte
    let mask_res = self.mask & c_mask;       // Check if byte allowed

    // If result > 0, byte is allowed
    (mask_res > 0).then(|| {
        // Calculate child index
        let smaller_bits = mask_res - 1;
        let smaller_bits_mask = smaller_bits & self.mask;
        let index_offset = smaller_bits_mask.count_ones() as usize;
        self.edge_start + index_offset
    })
}
```

**Formula breakdown:**
```rust
child_index = ((input_mask - 1) & node.mask).count_ones()
```

| Step | Expression | Result | Purpose |
|------|------------|--------|---------|
| 1 | `input_mask - 1` | All bits below input set | Create mask of "smaller" bits |
| 2 | `(input_mask - 1) & node.mask` | Smaller bits that exist in node | Filter to valid children |
| 3 | `.count_ones()` | Count of smaller children | Index offset |
| 4 | `edge_start + offset` | Final child index | Index into nodes Vec |

---

## Part 6: TrieIter - Ordered Iteration

### Iterator Structure

```rust
pub struct TrieIterSized<'b, 'a, T, I> {
    stack: Vec<TrieNodeIter>,
    trie: &'b TrieHardSized<'a, T, I>,
}

struct TrieNodeIter {
    node_index: usize,
    stage: TrieNodeIterStage,
}

enum TrieNodeIterStage {
    Inner,                    // First visit
    Child(usize, usize),      // Processing children (current, total)
}
```

### Iterator Implementation

```rust
impl<'b, 'a, T, I> Iterator for TrieIterSized<'b, 'a, T, I>
where
    T: Copy,
{
    type Item = (&'a [u8], T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node, node_index, stage)) = self.stack.pop()
            .and_then(|iter| {
                self.trie.nodes.get(iter.node_index)
                    .map(|node| (node, iter.node_index, iter.stage))
            })
        {
            use TrieState as T;
            use TrieNodeIterStage as S;

            match (node, stage) {
                // Leaf: emit value
                (T::Leaf(key, value), S::Inner) => {
                    return Some((*key, *value));
                }

                // SearchOrLeaf: emit leaf value first, then process children
                (T::SearchOrLeaf(key, value, search), S::Inner) => {
                    // Push child iterator
                    self.stack.push(TrieNodeIter {
                        node_index,
                        stage: S::Child(0, search.mask.count_ones() as usize),
                    });
                    self.stack.push(TrieNodeIter {
                        node_index: search.edge_start,
                        stage: S::Inner,
                    });
                    return Some((*key, *value));
                }

                // Search: process children
                (T::Search(search), S::Inner) => {
                    self.stack.push(TrieNodeIter {
                        node_index,
                        stage: S::Child(0, search.mask.count_ones() as usize),
                    });
                    self.stack.push(TrieNodeIter {
                        node_index: search.edge_start,
                        stage: S::Inner,
                    });
                }

                // Continue processing siblings
                (T::SearchOrLeaf(_, _, search) | T::Search(search), S::Child(child, count)) => {
                    if child + 1 < count {
                        self.stack.push(TrieNodeIter {
                            node_index,
                            stage: S::Child(child + 1, count),
                        });
                        self.stack.push(TrieNodeIter {
                            node_index: search.edge_start + child + 1,
                            stage: S::Inner,
                        });
                    }
                }

                _ => unreachable!(),
            }
        }

        None  // Iterator exhausted
    }
}
```

**Key insight:** Stack-based DFS traversal ensures sorted order (children processed in byte order).

---

## Part 7: Prefix Search

### Algorithm

```rust
pub fn prefix_search<K: AsRef<[u8]>>(&self, prefix: K) -> TrieIterSized<'_, 'a, T, $int_type> {
    let key = prefix.as_ref();
    let mut node_index = 0;
    let Some(mut state) = self.nodes.get(node_index) else {
        return TrieIterSized::empty(self);
    };

    // Navigate to prefix node
    for (i, c) in key.iter().enumerate() {
        let next_state_opt = match state {
            TrieState::Leaf(k, _) => {
                if k.len() == key.len() && k[i..] == key[i..] {
                    // Exact match at leaf
                    return TrieIterSized::new(self, node_index);
                } else {
                    return TrieIterSized::empty(self);
                }
            }
            TrieState::Search(search)
            | TrieState::SearchOrLeaf(_, _, search) => {
                search.evaluate(*c, self)
            }
        };

        if let Some(next_state_index) = next_state_opt {
            node_index = next_state_index;
            state = &self.nodes[next_state_index];
        } else {
            return TrieIterSized::empty(self);
        }
    }

    // Start iterator from prefix node
    TrieIterSized::new(self, node_index)
}
```

**Example:** `prefix_search("d")` on ["and", "ant", "dad", "do", "dot"]
```
1. Start at root (index 0)
2. Process 'd': root.evaluate('d') -> index 2
3. Prefix exhausted, start iterator from index 2
4. Iterator yields: "dad", "do", "dot" (all words under "d" subtree)
```

---

## Part 8: Type System Design

### The TrieHard Enum

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

**Why an enum?**
- Single public API regardless of internal size
- Automatic size selection based on input
- Type-safe: can't accidentally mix sizes

### Generic Implementations via Macro

```rust
macro_rules! trie_impls {
    ($($int_type:ty),+) => {
        $(
            trie_impls!(_impl $int_type);
        )+
    };

    (_impl $int_type:ty) => {
        // Implementation for specific integer type
        // SearchNode<$int_type>, TrieHardSized<'a, T, $int_type>, etc.
    }
}

// Generate implementations for all types
trie_impls! {u8, u16, u32, u64, u128, U256}
```

**Why a macro?**
- Avoid writing the same code 6 times
- Ensure consistency across types
- Compile-time code generation (no runtime cost)

---

## Part 9: Memory Layout Analysis

### Complete Memory Layout

For ["and", "ant", "dad", "do", "dot"] with u8 storage:

```
TrieHard::U8(TrieHardSized {
    masks: MasksByByteSized([
        0, 0, ...,                         // 0-96
        0b00001,                           // 97 = 'a'
        0, 0,                              // 98-99
        0b00010,                           // 100 = 'd'
        ...,                               // 101-109
        0b00100,                           // 110 = 'n'
        0b01000,                           // 111 = 'o'
        ...,                               // 112-115
        0b10000,                           // 116 = 't'
        ...,                               // 117-255
    ]),  // 256 bytes

    nodes: vec![
        SearchNode { mask: 0b00011, edge_start: 1 },   // 0: root
        SearchNode { mask: 0b00100, edge_start: 3 },   // 1: "a"
        SearchNode { mask: 0b01001, edge_start: 4 },   // 2: "d"
        SearchNode { mask: 0b10010, edge_start: 6 },   // 3: "an"
        Leaf("dad", "dad"),                            // 4: "dad"
        SearchOrLeaf("do", "do", ...),                 // 5: "do"
        Leaf("and", "and"),                            // 6: "and"
        Leaf("ant", "ant"),                            // 7: "ant"
        Leaf("dot", "dot"),                            // 8: "dot"
    ],  // ~9 x (4 + 8) = 108 bytes (estimate)
})
```

**Total:** ~364 bytes for 5 words

### Memory vs. HashMap

```rust
// HashMap<&str, &str> for same data:
HashMap {
    entries: [
        HashMapEntry { hash: ..., key: "and", value: "and" },
        HashMapEntry { hash: ..., key: "ant", value: "ant" },
        ...
    ]
}
```

HashMap uses less memory for small sets but:
- trie-hard: O(1) fail-fast
- HashMap: must hash entire key

---

## Summary

You now understand:

1. **U256 internals** - 4 x u64, bitwise operations, shift handling
2. **MasksByByte** - 256-entry lookup table, adaptive sizing
3. **TrieState variants** - Leaf, Search, SearchOrLeaf purposes
4. **Bulk-loading** - BFS construction, sorted input, queue-based
5. **Lookup algorithm** - Byte-by-byte traversal, mask checking
6. **Child index formula** - `((mask - 1) & node.mask).count_ones()`
7. **Iteration** - Stack-based DFS for sorted order
8. **Prefix search** - Navigate to prefix, iterate subtree
9. **Memory layout** - Contiguous nodes, cache efficiency

---

## Exercises

1. Trace through `get("ant")` step by step
2. Calculate the memory usage for 100 words with 40 unique bytes
3. Implement the U256 type yourself
4. Modify the bulk-loader to support path compression
5. Add a `contains_prefix` method (returns true if any word starts with prefix)

---

## Next Steps

Continue to **[02-wasm-integration-deep-dive.md](02-wasm-integration-deep-dive.md)** for:
- WASM compatibility analysis
- Cloudflare Workers usage
- Edge computing patterns
- Size optimization techniques
