---
title: "Zero to Serialization Engineer: A First-Principles Journey Through CBOR"
subtitle: "Complete textbook-style guide from serialization fundamentals to CBOR encoding"
based_on: "CBOR (RFC 7049 / RFC 8949) - Concise Binary Object Representation"
level: "Beginner to Intermediate - No prior serialization knowledge assumed"
---

# Zero to Serialization Engineer: First-Principles Guide

## Table of Contents

1. [What Is Serialization?](#1-what-is-serialization)
2. [Binary vs Text Formats](#2-binary-vs-text-formats)
3. [CBOR Design Goals](#3-cbor-design-goals)
4. [Major Types Overview](#4-major-types-overview)
5. [First Encoding Examples](#5-first-encoding-examples)
6. [Your Learning Path](#6-your-learning-path)

---

## 1. What Is Serialization?

### 1.1 The Fundamental Problem

**Serialization** is the process of converting in-memory data structures into a format that can be:
- Stored (saved to disk, database)
- Transmitted (sent over network)
- Reconstructed later (deserialization)

```
┌─────────────────────────────────────────────────────────┐
│                    Serialization                         │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐          │
│  │  Memory  │ -> │Serialize │ -> │  Bytes   │          │
│  │  Struct  │    │          │    │  (wire)  │          │
│  └──────────┘    └──────────┘    └──────────┘          │
│                                                        │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐          │
│  │  Memory  │ <- │Deserialize│ <- │  Bytes   │          │
│  │  Struct  │    │          │    │  (wire)  │          │
│  └──────────┘    └──────────┘    └──────────┘          │
└─────────────────────────────────────────────────────────┘
```

**Real-world analogy:** Shipping a bicycle

| Aspect | Bicycle Shipping | Serialization |
|--------|-----------------|---------------|
| Original | Assembled bicycle | In-memory struct |
| Process | Disassemble, pack | Serialize to bytes |
| Transport | Shipping container | Network/disk |
| Reconstruction | Reassemble bicycle | Deserialize to struct |

### 1.2 Why Serialization Matters

Without serialization, programs cannot:
- Save state between runs
- Communicate over networks
- Store data in databases
- Share data between different programs/languages

**Example: User Session**

```rust
// In memory (Rust)
struct Session {
    user_id: u64,
    token: String,
    expires_at: u64,
}

// Must become bytes for:
// - Redis storage
// - Network transmission
// - Cookie encoding
```

### 1.3 Serialization Requirements

Any serialization format must address:

| Requirement | Question | CBOR Answer |
|-------------|----------|-------------|
| **Type System** | What types can be represented? | 7 major types + tags |
| **Encoding** | How are types represented as bytes? | Prefix + payload |
| **Schema** | How do we define expected structure? | CDDL |
| **Validation** | How do we verify data is correct? | Type checking, tag validation |
| **Extensibility** | How do we add new types? | Tags (0-18446744073709551615) |
| **Efficiency** | How small/fast is the encoding? | Compact binary, fast parsing |

---

## 2. Binary vs Text Formats

### 2.1 Text Formats (JSON, XML, YAML)

**JSON Example:**
```json
{
    "name": "Alice",
    "age": 30,
    "active": true
}
```

**Characteristics:**

| Aspect | JSON | CBOR |
|--------|------|------|
| **Human Readable** | Yes (designed for it) | No (but tools exist) |
| **Size** | Verbose (field names repeated) | Compact (binary prefixes) |
| **Parsing Speed** | Slower (text processing) | Faster (direct binary) |
| **Type Information** | Limited (string, number, bool, null) | Rich (7 major types + tags) |
| **Binary Data** | Base64 encoding required (+33%) | Native byte strings |

**JSON byte representation:**
```
7b 22 6e 61 6d 65 22 3a 22 41 6c 69 63 65 22 2c  {"name":"Alice",
22 61 67 65 22 3a 33 30 2c 22 61 63 74 69 76 65  "age":30,"active"
22 3a 74 72 75 65 7d                             ":true}
= 48 bytes
```

### 2.2 Binary Formats (CBOR, MessagePack, Protobuf)

**CBOR equivalent:**
```
a3           # map(3)
64 6e 61 6d 65  # string(4) "name"
65 41 6c 69 63 65 # "Alice"
63 61 67 65  # string(3) "age"
18 1e        # unsigned(30)
66 61 63 74 69 76 65 # string(6) "active"
f5           # true
= 26 bytes (46% smaller than JSON)
```

### 2.3 Why Binary Is More Efficient

**Text format overhead:**
1. Field names as strings (repeated for every record)
2. Structural characters (`{ } [ ] , :`)
3. Number-to-text conversion (parsing cost)
4. No native binary (Base64 encoding)

**Binary format advantages:**
1. Single-byte type prefixes
2. Direct binary representation of numbers
3. Native byte strings
4. No parsing overhead

```
Number 1000000:
JSON: "1000000" = 7 bytes (ASCII)
CBOR: 0x1a 0x00 0x0f 0x42 0x40 = 5 bytes (binary)

Number 1000000000000:
JSON: "1000000000000" = 13 bytes
CBOR: 0x1b 0x00 0x00 0x00 0xe8 0xd4 0xa5 0x10 0x00 = 9 bytes
```

---

## 3. CBOR Design Goals

### 3.1 The CBOR Philosophy

CBOR was designed with these goals (RFC 7049):

1. **Small code size** - Minimal implementation complexity
2. **Small message size** - Compact encoding
3. **Extensibility** - Support for tags and custom types
4. **JSON compatibility** - Easy conversion to/from JSON
5. **Self-describing** - No schema needed for basic parsing

### 3.2 Design Trade-offs

| Goal | Trade-off | CBOR Decision |
|------|-----------|---------------|
| Human readability | vs Size | Prioritize size (binary) |
| Schema requirements | vs Flexibility | Optional (CDDL exists but not required) |
| Backwards compatibility | vs Efficiency | Fixed encoding rules |
| Feature completeness | vs Simplicity | 7 major types only |

### 3.3 Comparison with Other Formats

**vs Protocol Buffers:**

| Aspect | CBOR | Protobuf |
|--------|------|----------|
| Schema | Optional (CDDL) | Required (.proto) |
| Self-describing | Yes | No (need .proto) |
| Binary size | Slightly larger | Smaller (field numbers) |
| Learning curve | Low | Medium |

**vs MessagePack:**

| Aspect | CBOR | MessagePack |
|--------|------|-------------|
| Standardization | RFC (IETF) | Community spec |
| Tags | Yes (semantic types) | Limited |
| Canonical form | Yes | No |
| Security focus | Yes (COSE, CWT) | General purpose |

---

## 4. Major Types Overview

### 4.1 The 7 Major Types

CBOR has 7 major types, identified by the first 3 bits of the initial byte:

```
Byte structure:
  7 6 5 4 3 2 1 0
  │ │ │ │ │ │ │ │
  │ │ │ └─┴─┴─┴─┴─ Additional Information
  │ │ │
  └─┴─┴─── Major Type (3 bits)
```

| Major Type | Binary | Description |
|------------|--------|-------------|
| **0** | `000` | Unsigned integer |
| **1** | `001` | Negative integer |
| **2** | `010` | Byte string |
| **3** | `011` | Text string |
| **4** | `100` | Array |
| **5** | `101` | Map |
| **6** | `110` | Tag |
| **7** | `111` | Simple values / Floats |

### 4.2 Additional Information

The remaining 5 bits provide additional information:

| Value | Meaning |
|-------|---------|
| 0-23 | Value directly in these bits |
| 24 | Next byte is the value |
| 25 | Next 2 bytes (big-endian) |
| 26 | Next 4 bytes (big-endian) |
| 27 | Next 8 bytes (big-endian) |
| 28-30 | Reserved |
| 31 | Indefinite length (for strings, arrays, maps) |

### 4.3 Major Type 0: Unsigned Integers

```
Value 5:
  000 00101 = 0x05
  │   │
  │   └─ Value 5 (fits in 5 bits)
  └──── Major type 0

Value 1000:
  000 11001 = 0x19 0x03 0xe8
  │   │     │   └─ Next 2 bytes: 1000
  │   └──── 25 = next 2 bytes
  └──────── Major type 0

Value 1000000:
  000 11010 = 0x1a 0x00 0x0f 0x42 0x40
  │   │     │   └───── Next 4 bytes: 1000000
  │   └──── 26 = next 4 bytes
  └──────── Major type 0
```

### 4.4 Major Type 1: Negative Integers

Negative integers are stored as the **negative value minus 1**, encoded as unsigned:

```
-1 = encoded as 0 (0 - 1 = -1)
-2 = encoded as 1 (1 - 1 = -2)
-100 = encoded as 99 (99 - 1 = -100)

-1:
  001 00000 = 0x20
  │   │
  │   └─ Value 0 → -(0+1) = -1
  └──── Major type 1

-1000:
  001 11001 = 0x39 0x03 0xe7
  │   │     │   └─ Next 2 bytes: 999 → -(999+1) = -1000
  │   └──── 25 = next 2 bytes
  └──────── Major type 1
```

### 4.5 Major Types 2 & 3: Strings

**Byte strings (type 2):** Raw binary data
**Text strings (type 3):** UTF-8 encoded text

```
"Hello" (text string):
  011 00101 = 0x65 0x48 0x65 0x6c 0x6c 0x6f
  │   │     │   └──────────────────────── "Hello" in UTF-8
  │   └──── 5 = length
  └──────── Major type 3

Binary data [0x01, 0x02, 0x03] (byte string):
  010 00011 = 0x43 0x01 0x02 0x03
  │   │     │   └───────────────── Raw bytes
  │   └──── 3 = length
  └──────── Major type 2
```

### 4.6 Major Types 4 & 5: Arrays and Maps

**Arrays (type 4):** Ordered list of values
**Maps (type 5):** Key-value pairs

```
[1, 2, 3] (array):
  100 00011 = 0x83 0x01 0x02 0x03
  │   │     │   └──────────────── Elements
  │   └──── 3 = length (3 elements)
  └──────── Major type 4

{"a": 1, "b": 2} (map):
  101 00010 = 0xa2
  │   │     └─ 2 key-value pairs
  │   └──── Major type 5

  61 61 = "a" (key)
  01    = 1   (value)
  61 62 = "b" (key)
  02    = 2   (value)
```

### 4.7 Major Type 6: Tags

Tags add semantic meaning to a value:

```
Tag 1 (epoch time) with value 1363896240:
  110 00001 = 0xc1 0x1a 0x51 0x4b 0x67 0xb0
  │   │     │   └──────────────────── Epoch timestamp
  │   └──── Tag 1 (epoch time)
  └──────── Major type 6

Common tags:
- Tag 0: Standard date/time string
- Tag 1: Epoch-based date/time
- Tag 2: Positive bignum
- Tag 3: Negative bignum
- Tag 6: Base64url
- Tag 7: Base64
- Tag 24: Embedded CBOR
```

### 4.8 Major Type 7: Simple Values and Floats

Simple values share major type 7 with floats:

```
Simple values (additional info 0-23, 32+):
  111 10100 = 0xf4 = false
  111 10101 = 0xf5 = true
  111 10110 = 0xf6 = null
  111 10111 = 0xf7 = undefined

Floats (additional info 25-27):
  111 11001 = 0xf9 + 2 bytes = float16
  111 11010 = 0xfa + 4 bytes = float32
  111 11011 = 0xfb + 8 bytes = float64
```

---

## 5. First Encoding Examples

### 5.1 Encoding Numbers

```rust
// Number encoding examples
5       → 0x05           // Major type 0, value 5
100     → 0x18 0x64      // Major type 0, next byte
1000    → 0x19 0x03 0xe8 // Major type 0, next 2 bytes
-1      → 0x20           // Major type 1, value 0 → -1
-100    → 0x38 0x63      // Major type 1, next byte (99 → -100)
```

### 5.2 Encoding Strings

```rust
// Text strings
""          → 0x60           // Empty string
"a"         → 0x61 0x61      // 1-char string
"Hello"     → 0x65 0x48...   // 5-char string

// Byte strings
[]          → 0x40           // Empty byte string
[0x01]      → 0x41 0x01      // Single byte
[0x01, 0x02] → 0x42 0x01 0x02 // Two bytes
```

### 5.3 Encoding Collections

```rust
// Arrays
[]          → 0x80           // Empty array
[1, 2, 3]   → 0x83 0x01 0x02 0x03
[null, true, false] → 0x83 0xf6 0xf5 0xf4

// Maps
{}              → 0xa0         // Empty map
{"a": 1}        → 0xa1 0x61 0x61 0x01
{"a": 1, "b": 2} → 0xa2 0x61 0x61 0x01 0x61 0x62 0x02
```

### 5.4 Complex Example: Nested Structure

```rust
// JSON equivalent:
{
    "name": "Alice",
    "scores": [95, 87, 92],
    "active": true
}

// CBOR encoding:
a3              # map(3 pairs)
64 6e 61 6d 65  # string(4) "name"
65 41 6c 69 63 65 # string(5) "Alice"
66 73 63 6f 72 65 73 # string(6) "scores"
83 18 5f 18 57 18 5c # array(3): 95, 87, 92
66 61 63 74 69 76 65 # string(6) "active"
f5              # true
```

### 5.5 Using Rust to Encode

```rust
use serde_cbor::to_vec;
use std::collections::BTreeMap;

fn main() {
    // Create a map
    let mut map = BTreeMap::new();
    map.insert("name", "Alice");
    map.insert("city", "Boston");

    // Serialize to CBOR
    let bytes = to_vec(&map).unwrap();
    println!("CBOR bytes: {:02x?}", bytes);

    // Expected output:
    // a2 64 6e 61 6d 65 65 41 6c 69 63 65
    // 64 63 69 74 79 66 42 6f 73 74 6f 6e
}
```

---

## 6. Your Learning Path

### 6.1 How to Use This Exploration

This document is part of a larger exploration:

```
cbor/
├── 00-zero-to-serialization-engineer.md ← You are here
├── 01-cbor-data-model-deep-dive.md
├── 02-schema-validation-deep-dive.md
├── 03-performance-comparison-deep-dive.md
├── 04-use-cases-deep-dive.md
├── 05-valtron-integration.md
├── rust-revision.md
└── production-grade.md
```

### 6.2 Recommended Reading Order

**For complete beginners:**

1. **This document (00-zero-to-serialization-engineer.md)** - Serialization and CBOR foundations
2. **[01-cbor-data-model-deep-dive.md](01-cbor-data-model-deep-dive.md)** - Detailed encoding rules
3. **[02-schema-validation-deep-dive.md](02-schema-validation-deep-dive.md)** - CDDL schemas
4. **[rust-revision.md](rust-revision.md)** - Rust implementation
5. **[04-use-cases-deep-dive.md](04-use-cases-deep-dive.md)** - Real-world applications

**For experienced Rust developers:**

1. Skim this document for CBOR basics
2. Jump to [rust-revision.md](rust-revision.md) for serde_cbor patterns
3. Deep dive into [01-cbor-data-model-deep-dive.md](01-cbor-data-model-deep-dive.md) for encoding details

### 6.3 Practice Exercises

**Exercise 1: Manual Encoding**

Encode these values by hand, then verify with serde_cbor:
1. The number 42
2. The string "CBOR"
3. The array [1, 2, 3, 4, 5]
4. The map {"key": "value"}

**Exercise 2: Size Comparison**

```rust
use serde_json::to_vec as to_json;
use serde_cbor::to_vec as to_cbor;

let data = serde_json::json!({
    "name": "Test",
    "values": [1, 2, 3, 4, 5],
    "nested": {"a": 1, "b": 2}
});

let json_size = to_json(&data).unwrap().len();
let cbor_size = to_cbor(&data).unwrap().len();

println!("JSON: {} bytes, CBOR: {} bytes", json_size, cbor_size);
println!("CBOR is {}% smaller", 100 - (cbor_size * 100 / json_size));
```

**Exercise 3: Round-trip Testing**

```rust
use serde::{Serialize, Deserialize};
use serde_cbor::{to_vec, from_slice};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Person {
    name: String,
    age: u32,
}

fn main() {
    let person = Person {
        name: "Bob".to_string(),
        age: 25,
    };

    let bytes = to_vec(&person).unwrap();
    let decoded: Person = from_slice(&bytes).unwrap();

    assert_eq!(person, decoded);
    println!("Round-trip successful!");
}
```

### 6.4 Key Resources

| Resource | Purpose |
|----------|---------|
| [RFC 7049 / RFC 8949](https://www.rfc-editor.org/rfc/rfc8949.html) | CBOR specification |
| [CBOR Playground](https://cbor.me/) | Online encoder/decoder |
| [serde_cbor docs](https://docs.rs/serde_cbor/) | Rust library |
| [cddl.tools](https://cddl.tools/) | CDDL schema validator |

---

## Appendix A: Quick Reference Card

```
CBOR Major Types:
0 = unsigned int     0x00-0x17 = 0-23 direct
1 = negative int     0x18 = next byte
2 = byte string      0x19 = next 2 bytes
3 = text string      0x1a = next 4 bytes
4 = array            0x1b = next 8 bytes
5 = map              0x20 = major type 1, value 0 = -1
6 = tag              0x40 = major type 2, length 0
7 = simple/float     0x60 = major type 3, length 0
                     0x80 = major type 4, length 0
Simple Values:       0xa0 = major type 5, length 0
0xf4 = false         0xc0 = major type 6 (tag)
0xf5 = true          0xf4-0xf7 = simple values
0xf6 = null          0xf9/fa/fb = floats
```

## Appendix B: Common CBOR Prefixes

```
Numbers:
0x00-0x17 = 0-23
0x18 xx = 1-byte int
0x19 xx xx = 2-byte int
0x1a xx xx xx xx = 4-byte int
0x20 = -1
0x38 xx = -(xx+1)

Strings:
0x40 = empty byte string
0x41 xx = 1-byte string
0x60 = empty text string
0x61 xx = 1-char text

Collections:
0x80 = empty array
0x81 = 1-element array
0xa0 = empty map
0xa1 = 1-pair map

Tags:
0xc0 = tag 0 (datetime string)
0xc1 = tag 1 (epoch time)
0xc2 = tag 2 (positive bignum)
0xd8 18 = tag 24 (embedded CBOR)
0xd9 d9 f7 = tag 55799 (self-describe)
```

---

*This document is a living textbook. Revisit sections as concepts become clearer through implementation. Next: [01-cbor-data-model-deep-dive.md](01-cbor-data-model-deep-dive.md)*
