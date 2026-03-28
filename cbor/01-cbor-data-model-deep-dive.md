---
title: "CBOR Data Model Deep Dive"
subtitle: "Complete exploration of major types, encoding rules, tags, and canonical CBOR"
based_on: "RFC 7049 / RFC 8949 - The CBOR Specification"
level: "Intermediate - Requires serialization fundamentals"
---

# CBOR Data Model Deep Dive

## Table of Contents

1. [Major Type 0: Unsigned Integers](#1-major-type-0-unsigned-integers)
2. [Major Type 1: Negative Integers](#2-major-type-1-negative-integers)
3. [Major Type 2: Byte Strings](#3-major-type-2-byte-strings)
4. [Major Type 3: Text Strings](#4-major-type-3-text-strings)
5. [Major Type 4: Arrays](#5-major-type-4-arrays)
6. [Major Type 5: Maps](#6-major-type-5-maps)
7. [Major Type 6: Tags](#7-major-type-6-tags)
8. [Major Type 7: Simple Values and Floats](#8-major-type-7-simple-values-and-floats)
9. [Canonical Encoding Rules](#9-canonical-encoding-rules)
10. [Implementation in Rust](#10-implementation-in-rust)

---

## 1. Major Type 0: Unsigned Integers

### 1.1 Encoding Rules

Unsigned integers use major type 0 with the value encoded directly or in following bytes:

```
┌─────────────────────────────────────────────────────────┐
│                 Unsigned Integer Encoding                │
├─────────────────────────────────────────────────────────┤
│ Value Range        │ Prefix │ Additional Bytes          │
├─────────────────────────────────────────────────────────┤
│ 0-23               │ 0x00-0x17 │ None (value in prefix) │
│ 24-255             │ 0x18     │ 1 byte                  │
│ 256-65535          │ 0x19     │ 2 bytes (big-endian)    │
│ 65536-4294967295   │ 0x1a     │ 4 bytes (big-endian)    │
│ 0-2^64-1           │ 0x1b     │ 8 bytes (big-endian)    │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Encoding Examples

```rust
// Value 0: Direct encoding
0 → 0x00
// Binary: 000 00000
//         │   └──── Value 0
//         └──────── Major type 0

// Value 23: Maximum direct encoding
23 → 0x17
// Binary: 000 10111

// Value 24: Requires extension byte
24 → 0x18 0x18
//         │   └─ Value 24
//         └──── Prefix: major type 0, additional info 24

// Value 100: Extension byte
100 → 0x18 0x64
//          └─ 100 in decimal

// Value 1000: 2-byte extension
1000 → 0x19 0x03 0xe8
//      │   └───── 0x03e8 = 1000 in big-endian
//      └──────── Prefix: major type 0, additional info 25

// Value 1000000: 4-byte extension
1000000 → 0x1a 0x00 0x0f 0x42 0x40
//        │   └───────────── 0x000f4240 = 1000000

// Value 2^63: 8-byte extension
9223372036854775808 → 0x1b 0x80 0x00 0x00 0x00 0x00 0x00 0x00 0x00
```

### 1.3 Shortest Form Requirement

CBOR requires using the shortest possible encoding:

```
INCORRECT: 24 encoded as 0x18 0x18 (should be 0x18)
INCORRECT: 100 encoded as 0x19 0x00 0x64 (should be 0x18 0x64)
INCORRECT: 1000 encoded as 0x1a 0x00 0x00 0x03 0xe8 (should be 0x19 0x03 0xe8)

VALIDATION: Decoders SHOULD reject non-shortest encodings
```

### 1.4 Rust Implementation

```rust
fn encode_u64(value: u64) -> Vec<u8> {
    match value {
        0..=23 => vec![value as u8],
        24..=255 => vec![0x18, value as u8],
        256..=65535 => {
            let mut buf = vec![0x19];
            buf.extend_from_slice(&value.to_be_bytes()[6..]);
            buf
        }
        65536..=4294967295 => {
            let mut buf = vec![0x1a];
            buf.extend_from_slice(&value.to_be_bytes()[4..]);
            buf
        }
        _ => {
            let mut buf = vec![0x1b];
            buf.extend_from_slice(&value.to_be_bytes());
            buf
        }
    }
}
```

---

## 2. Major Type 1: Negative Integers

### 2.1 Encoding Rules

Negative integers use major type 1 with the value stored as `-(n+1)`:

```
┌─────────────────────────────────────────────────────────┐
│                 Negative Integer Encoding                │
├─────────────────────────────────────────────────────────┤
│ Stored Value │ Represents │ Example                    │
├─────────────────────────────────────────────────────────┤
│ 0            │ -1         │ -(0+1) = -1                │
│ 1            │ -2         │ -(1+1) = -2                │
│ 99           │ -100       │ -(99+1) = -100             │
│ 255          │ -256       │ -(255+1) = -256            │
└─────────────────────────────────────────────────────────┘

Encoding: value_to_encode = -(n+1) where n is the negative number
```

### 2.2 Encoding Examples

```rust
// Value -1: Stored as 0
-1 → 0x20
// Binary: 001 00000
//         │   └──── Stored value 0 → -(0+1) = -1
//         └──────── Major type 1

// Value -10: Stored as 9
-10 → 0x29
// Binary: 001 01001
//               └─ Stored value 9 → -(9+1) = -10

// Value -100: Stored as 99
-100 → 0x38 0x63
//           └─ 99 in decimal → -(99+1) = -100
//      └──── Prefix: major type 1, additional info 24

// Value -1000: Stored as 999
-1000 → 0x39 0x03 0xe7
//           └───── 999 in big-endian → -(999+1) = -1000

// Minimum value: -2^64
-18446744073709551616 → 0x3b 0xff 0xff 0xff 0xff 0xff 0xff 0xff 0xff
// Stored as 2^64-1 (all 1s) → -(2^64-1+1) = -2^64
```

### 2.3 Conversion Formula

```rust
// To encode a negative number n:
fn encode_negative(n: i64) -> Vec<u8> {
    assert!(n < 0);
    let stored = (-(n + 1)) as u64;
    encode_u64_with_major_type(stored, 1)
}

// To decode:
fn decode_negative(stored: u64) -> i64 {
    -((stored + 1) as i64)
}
```

---

## 3. Major Type 2: Byte Strings

### 3.1 Encoding Rules

Byte strings use major type 2 with length encoding:

```
┌─────────────────────────────────────────────────────────┐
│                   Byte String Encoding                   │
├─────────────────────────────────────────────────────────┤
│ Length Range     │ Prefix │ Length Bytes │ Data         │
├─────────────────────────────────────────────────────────┤
│ 0-23             │ 0x40-0x57 │ None      │ N bytes      │
│ 24-255           │ 0x58     │ 1 byte     │ N bytes      │
│ 256-65535        │ 0x59     │ 2 bytes    │ N bytes      │
│ 65536-4294967295 │ 0x5a     │ 4 bytes    │ N bytes      │
│ 0-2^64-1         │ 0x5b     │ 8 bytes    │ N bytes      │
└─────────────────────────────────────────────────────────┘
```

### 3.2 Encoding Examples

```rust
// Empty byte string
[] → 0x40
// Prefix: major type 2, length 0

// Single byte
[0xab] → 0x41 0xab
//        │   └─ Data byte
//        └──── Length 1

// 10 bytes of binary data
[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09]
→ 0x4a 0x00 0x01 0x02 0x03 0x04 0x05 0x06 0x07 0x08 0x09
//   │   └──────────────────────────────────────────── Data
//   └──── Length 10 (0x0a)

// Large byte string (1000 bytes)
→ 0x59 0x03 0xe8 <1000 bytes of data>
//   │   └───── Length 1000
//   └──────── Prefix: major type 2, additional info 25
```

### 3.3 Indefinite Length Byte Strings

CBOR supports indefinite length strings for streaming:

```rust
// Indefinite length byte string
// Useful when length is unknown at encoding time

→ 0x5f <chunks> 0xff
//   │   │       └─ Break marker
//   │   └───── Zero or more byte string chunks
//   └──────── Prefix: major type 2, additional info 31

// Example with two chunks:
0x5f 0x43 0x01 0x02 0x03 0x42 0x04 0x05 0xff
// Decodes as: [0x01, 0x02, 0x03, 0x04, 0x05]
```

### 3.4 Rust Implementation

```rust
fn encode_bytes(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let len = data.len();

    // Encode length with major type 2
    let prefix = match len {
        0..=23 => vec![0x40 + len as u8],
        24..=255 => vec![0x58, len as u8],
        256..=65535 => {
            let mut buf = vec![0x59];
            buf.extend_from_slice(&(len as u16).to_be_bytes());
            buf
        }
        _ => {
            let mut buf = vec![0x5a];
            buf.extend_from_slice(&(len as u32).to_be_bytes());
            buf
        }
    };

    result.extend(prefix);
    result.extend_from_slice(data);
    result
}
```

---

## 4. Major Type 3: Text Strings

### 4.1 Encoding Rules

Text strings use major type 3 with UTF-8 encoding:

```
┌─────────────────────────────────────────────────────────┐
│                   Text String Encoding                   │
├─────────────────────────────────────────────────────────┤
│ Identical to byte strings, but data MUST be valid UTF-8 │
│ Prefix: 0x60-0x7b for lengths 0-27                      │
└─────────────────────────────────────────────────────────┘
```

### 4.2 Encoding Examples

```rust
// Empty string
"" → 0x60

// ASCII string
"a" → 0x61 0x61
//         └─ ASCII 'a' (0x61)

// "Hello"
"Hello" → 0x65 0x48 0x65 0x6c 0x6c 0x6f
//        │   └───────────────────────── UTF-8 bytes

// UTF-8 multi-byte characters
"日本語" → 0x69 0xe6 0x97 0xa5 0xe6 0x9c 0xac 0xe8 0xaa 0x9e
//        │   └────────────────────────────────────── 9 bytes UTF-8
//        └──── Length 9

// Emoji (4-byte UTF-8)
"🦀" → 0x64 0xf0 0x9f 0xa6 0x80
//     │   └───────────────── UTF-8 for crab emoji
//     └──── Length 4
```

### 4.3 UTF-8 Validation

```rust
use std::str;

fn encode_text(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes(); // Already UTF-8 in Rust
    let mut result = Vec::new();

    // Encode length with major type 3
    let prefix = match bytes.len() {
        0..=23 => vec![0x60 + bytes.len() as u8],
        24..=255 => vec![0x78, bytes.len() as u8],
        _ => {
            // Handle larger sizes...
            vec![0x78, bytes.len() as u8]
        }
    };

    result.extend(prefix);
    result.extend_from_slice(bytes);
    result
}

// On decoding, validate UTF-8
fn decode_text(bytes: &[u8]) -> Result<&str, std::str::Utf8Error> {
    str::from_utf8(bytes)
}
```

---

## 5. Major Type 4: Arrays

### 5.1 Encoding Rules

Arrays use major type 4 with element count:

```
┌─────────────────────────────────────────────────────────┐
│                      Array Encoding                      │
├─────────────────────────────────────────────────────────┤
│ Elements Range │ Prefix │ Count Bytes │ Element Data   │
├─────────────────────────────────────────────────────────┤
│ 0-23           │ 0x80-0x97 │ None     │ N elements     │
│ 24-255         │ 0x98     │ 1 byte    │ N elements     │
│ 256-65535      │ 0x99     │ 2 bytes   │ N elements     │
│ 65536-4294967295│ 0x9a    │ 4 bytes   │ N elements     │
└─────────────────────────────────────────────────────────┘
```

### 5.2 Encoding Examples

```rust
// Empty array
[] → 0x80

// Single element
[1] → 0x81 0x01
//    │   └─ Element: 1
//    └──── Count: 1

// Mixed types
[1, "two", true, null]
→ 0x84 0x01 0x63 0x74 0x77 0x6f 0xf5 0xf6
//   │   │   └─────────┘ │   │
//   │   │   "two"       │   └─ null
//   │   │               └───── true
//   │   └─ Count: 4
//   └───── Major type 4

// Nested arrays
[[1, 2], [3, 4]]
→ 0x82 0x82 0x01 0x02 0x82 0x03 0x04
//   │   │   └─────┘ │   └─────┘
//   │   │  [1,2]    │   [3,4]
//   │   └─ Count: 2
//   └───── Count: 2
```

### 5.3 Indefinite Length Arrays

```rust
// Indefinite length array
→ 0x9f <elements> 0xff
//   │   │          └─ Break marker
//   │   └─────── Zero or more elements
//   └────────── Prefix: major type 4, additional info 31

// Example
0x9f 0x01 0x02 0x03 0xff
// Decodes as: [1, 2, 3]
```

---

## 6. Major Type 5: Maps

### 6.1 Encoding Rules

Maps use major type 5 with pair count:

```
┌─────────────────────────────────────────────────────────┐
│                       Map Encoding                       │
├─────────────────────────────────────────────────────────┤
│ Pairs Range │ Prefix │ Count Bytes │ Key-Value Pairs  │
├─────────────────────────────────────────────────────────┤
│ 0-23        │ 0xa0-0xb7 │ None     │ N pairs          │
│ 24-255      │ 0xb8     │ 1 byte    │ N pairs          │
│ 256-65535   │ 0xb9     │ 2 bytes   │ N pairs          │
│ 65536+      │ 0xba     │ 4 bytes   │ N pairs          │
└─────────────────────────────────────────────────────────┘

Map structure: <prefix> <key1> <value1> <key2> <value2> ...
```

### 6.2 Encoding Examples

```rust
// Empty map
{} → 0xa0

// Single pair
{"key": "value"}
→ 0xa1 0x63 0x6b 0x65 0x79 0x65 0x76 0x61 0x6c 0x75 0x65
//   │   └───────────┘ └───────────────┘
//   │   "key"         "value"
//   └──── Count: 1

// Multiple pairs
{"a": 1, "b": 2}
→ 0xa2 0x61 0x61 0x01 0x61 0x62 0x02
//   │   │   │  │   │   │  │
//   │   │   │  │   │   │  └─ Value: 2
//   │   │   │  │   │   └──── Key: "b"
//   │   │   │  │   └──────── Value: 1
//   │   │   │  └──────────── Key: "a"
//   │   │   └─ Count: 2 pairs
//   └───────── Major type 5
```

### 6.3 Canonical Map Ordering

For canonical CBOR, map keys MUST be sorted:

```
Canonical order (RFC 7049 bis):
1. Shorter keys sort first
2. Equal length: lexicographic byte order

Example:
{"b": 2, "a": 1} → Must encode as {"a": 1, "b": 2}
0xa2 0x61 0x61 0x01 0x61 0x62 0x02  (CORRECT)
0xa2 0x61 0x62 0x02 0x61 0x61 0x01  (INCORRECT for canonical)

Example with different lengths:
{"aa": 1, "b": 2} → {"b": 2, "aa": 1} (shorter key first)
```

### 6.4 Rust Implementation

```rust
use std::collections::BTreeMap;

fn encode_map(map: &BTreeMap<String, u32>) -> Vec<u8> {
    let mut result = Vec::new();
    let len = map.len();

    // Encode count with major type 5
    let prefix = match len {
        0..=23 => vec![0xa0 + len as u8],
        24..=255 => vec![0xb8, len as u8],
        _ => todo!("larger maps"),
    };

    result.extend(prefix);

    // BTreeMap maintains sorted order
    for (key, value) in map {
        result.extend(encode_text(key));
        result.extend(encode_u32(*value));
    }

    result
}
```

---

## 7. Major Type 6: Tags

### 7.1 Encoding Rules

Tags use major type 6 to add semantic meaning:

```
┌─────────────────────────────────────────────────────────┐
│                       Tag Encoding                       │
├─────────────────────────────────────────────────────────┤
│ Tag Number │ Prefix │ Tag Bytes │ Tagged Value         │
├─────────────────────────────────────────────────────────┤
│ 0-23       │ 0xc0-0xd7 │ None    │ Followed by value   │
│ 24-255     │ 0xd8     │ 1 byte  │ Followed by value   │
│ 256-65535  │ 0xd9     │ 2 bytes │ Followed by value   │
│ 65536+     │ 0xda     │ 4 bytes │ Followed by value   │
│ 2^32+      │ 0xdb     │ 8 bytes │ Followed by value   │
└─────────────────────────────────────────────────────────┘
```

### 7.2 Standard Tags

| Tag | Value | Meaning | Example |
|-----|-------|---------|---------|
| 0 | 0xc0 | Standard date/time string | `0xc0 0x78... "2026-03-27T10:00:00Z"` |
| 1 | 0xc1 | Epoch-based date/time | `0xc1 0x1a...` (4-byte timestamp) |
| 2 | 0xc2 | Positive bignum | `0xc2 0x43...` (byte string) |
| 3 | 0xc3 | Negative bignum | `0xc3 0x43...` |
| 6 | 0xc6 | Base64url encoding | `0xc6 0x43...` |
| 7 | 0xc7 | Base64 encoding | `0xc7 0x43...` |
| 24 | 0xd8 0x18 | Embedded CBOR | `0xd8 0x18 0x43...` |
| 55799 | 0xd9 0xd9 0xf7 | Self-describe CBOR | At document start |

### 7.3 Tag Examples

```rust
// Tag 1: Epoch timestamp (March 27, 2026 10:00:00 UTC)
0xc1 0x1a 0x6b 0x5a 0xe8 0x00
// │   └─────────────┘
// │   Unix timestamp 1783527424
// └─ Tag 1 (epoch time)

// Tag 2: Positive bignum (2^100)
0xc2 0x4d 0x08 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00
// │   └────────────────────────────────────────────────────┘
// │   13-byte big-endian integer
// └─ Tag 2 (positive bignum)

// Tag 0: Date/time string
0xc0 0x78 0x19 0x32 0x30 0x32 0x36 0x2d 0x30 0x33 0x2d 0x32 0x37 0x54 0x31 0x30 0x3a 0x30 0x30 0x3a 0x30 0x30 0x5a
// │   └─────────────────────────────────────────────────────────────────────┘
// │   UTF-8 string "2026-03-27T10:00:00Z"
// └─ Tag 0 (standard datetime)

// Tag 24: Embedded CBOR
0xd8 0x18 0x43 0x01 0x02 0x03
// │   │   └───────────── Encoded CBOR bytes
// │   └─ Tag 24
// └─ Prefix for tag 24

// Self-describe CBOR header
0xd9 0xd9 0xf7 <rest of CBOR document>
// └─────────┘ Tag 55799 at document start
```

### 7.4 Custom Tags

```rust
// Tags 0-23: Direct encoding
tag 5 → 0xc5 <value>

// Tags 24-255: 1-byte tag number
tag 100 → 0xd8 0x64 <value>

// Tags 256-65535: 2-byte tag number
tag 1000 → 0xd9 0x03 0xe8 <value>

// Large tags (used in custom applications)
tag 1000000 → 0xda 0x00 0x0f 0x42 0x40 <value>
```

### 7.5 Rust Tag Handling

```rust
use serde_cbor::tags::{Tag, Tagged};

// Working with tagged values
let tagged = Tagged {
    tag: Some(1), // Epoch time
    value: 1783527424u64,
};

// Serialize
let bytes = serde_cbor::to_vec(&tagged).unwrap();

// Deserialize
let decoded: Tagged<u64> = serde_cbor::from_slice(&bytes).unwrap();
assert_eq!(decoded.tag, Some(1));
assert_eq!(decoded.value, 1783527424);
```

---

## 8. Major Type 7: Simple Values and Floats

### 8.1 Simple Values (Additional Info 0-31)

```
┌─────────────────────────────────────────────────────────┐
│                   Simple Value Encoding                  │
├─────────────────────────────────────────────────────────┤
│ Value │ Encoding │ Meaning                             │
├─────────────────────────────────────────────────────────┤
│ 0-19  │ 0xe0-0xf3│ Unassigned (reserved)               │
│ 20    │ 0xf4     │ false                               │
│ 21    │ 0xf5     │ true                                │
│ 22    │ 0xf6     │ null                                │
│ 23    │ 0xf7     │ undefined                           │
│ 24-31 │ 0xf8+byte│ Extended simple values              │
└─────────────────────────────────────────────────────────┘
```

### 8.2 Float Encoding

```
┌─────────────────────────────────────────────────────────┐
│                    Float Encoding                        │
├─────────────────────────────────────────────────────────┤
│ Type │ Prefix │ Bytes │ IEEE 754 Format               │
├─────────────────────────────────────────────────────────┤
│ f16  │ 0xf9   │ 2     │ Half precision                │
│ f32  │ 0xfa   │ 4     │ Single precision              │
│ f64  │ 0xfb   │ 8     │ Double precision              │
└─────────────────────────────────────────────────────────┘
```

### 8.3 Float Examples

```rust
// Boolean values
false → 0xf4
true  → 0xf5

// Null and undefined
null      → 0xf6
undefined → 0xf7

// Float16 (half precision)
1.5 → 0xf9 0x3e 0x00
//    │   └───────── 0x3e00 in IEEE 754 f16

// Float32 (single precision)
-42.5 → 0xfa 0xc2 0x2a 0x00 0x00
//     │   └─────────────────── IEEE 754 f32

// Float64 (double precision)
3.141592653589793 → 0xfb 0x40 0x09 0x21 0xfb 0x54 0x44 0x2d 0x18
//                 └────────────────────────────── IEEE 754 f64

// Special values
Infinity    → 0xf9 0x7c 0x00  (f16) or 0xfa 0x7f 0x80 0x00 0x00 (f32)
-Infinity   → 0xf9 0xfc 0x00  (f16) or 0xfa 0xff 0x80 0x00 0x00 (f32)
NaN         → 0xf9 0x7e 0x00  (f16) or 0xfa 0x7f 0xc0 0x00 0x00 (f32)
```

### 8.4 Shortest Float Encoding

CBOR prefers the shortest float representation:

```
Value 1.0 can be encoded as:
f16: 0xf9 0x3c 0x00 (3 bytes) ✓ PREFERRED
f32: 0xfa 0x3f 0x80 0x00 0x00 (5 bytes)
f64: 0xfb 0x3f 0xf0 0x00 0x00 0x00 0x00 0x00 0x00 (9 bytes)

If f16 can represent the value exactly, use f16.
If f32 can represent it exactly (and f16 cannot), use f32.
```

---

## 9. Canonical Encoding Rules

### 9.1 Why Canonical CBOR?

Canonical encoding ensures identical data produces identical bytes:
- **Deterministic signatures** - Same data = same signature
- **Efficient comparison** - Byte comparison = value comparison
- **Hash consistency** - Same content = same hash

### 9.2 Canonical Rules (RFC 7049 bis)

```
┌─────────────────────────────────────────────────────────┐
│               Canonical CBOR Requirements                │
├─────────────────────────────────────────────────────────┤
│ 1. Integers use shortest encoding                       │
│ 2. Floats use shortest representation                   │
│ 3. Map keys are sorted by encoded byte order            │
│ 4. No indefinite length items                           │
│ 5. No tags unless semantically required                 │
│ 6. No simple values except true/false/null/undefined    │
└─────────────────────────────────────────────────────────┘
```

### 9.3 Map Key Sorting

```
Sorting rules:
1. Compare encoded byte strings directly
2. Shorter keys sort before longer keys
3. For equal length: lexicographic order

Examples:
{"b": 1, "a": 2} → {"a": 2, "b": 1} (lexicographic)
{"aa": 1, "b": 2} → {"b": 2, "aa": 1} (shorter first)
{"a": 1, "A": 2} → {"A": 2, "a": 1} (0x41 < 0x61)
```

### 9.4 Rust Canonical Encoding

```rust
use serde_cbor::ser::Serializer;
use serde::Serialize;

fn to_canonical_vec<T>(value: &T) -> Result<Vec<u8>, serde_cbor::Error>
where
    T: Serialize,
{
    use std::io::Cursor;

    let mut vec = Vec::new();
    let mut serializer = Serializer::new(Cursor::new(&mut vec));

    // Enable canonical encoding options
    serializer = serializer.canonical();

    value.serialize(&mut serializer)?;
    Ok(vec)
}
```

---

## 10. Implementation in Rust

### 10.1 Low-Level Encoder Structure

```rust
pub struct Encoder<W> {
    writer: W,
}

impl<W: Write> Encoder<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn u64(&mut self, value: u64) -> Result<()> {
        self.write_major_type(0, value)
    }

    pub fn i64(&mut self, value: i64) -> Result<()> {
        if value >= 0 {
            self.write_major_type(0, value as u64)
        } else {
            // Negative: store -(n+1)
            self.write_major_type(1, (-(value + 1)) as u64)
        }
    }

    pub fn bytes(&mut self, data: &[u8]) -> Result<()> {
        self.write_major_type(2, data.len() as u64)?;
        self.writer.write_all(data)?;
        Ok(())
    }

    pub fn text(&mut self, text: &str) -> Result<()> {
        self.write_major_type(3, text.len() as u64)?;
        self.writer.write_all(text.as_bytes())?;
        Ok(())
    }

    fn write_major_type(&mut self, major: u8, value: u64) -> Result<()> {
        match value {
            0..=23 => self.writer.write_u8(major << 5 | value as u8)?,
            24..=255 => {
                self.writer.write_u8(major << 5 | 24)?;
                self.writer.write_u8(value as u8)?;
            }
            256..=65535 => {
                self.writer.write_u8(major << 5 | 25)?;
                self.writer.write_u16::<BigEndian>(value as u16)?;
            }
            // ... continue for larger sizes
        }
        Ok(())
    }
}
```

### 10.2 Decoder Structure

```rust
pub struct Decoder<R> {
    reader: R,
    config: Config,
}

impl<R: Read> Decoder<R> {
    pub fn next_value(&mut self) -> Result<Value> {
        let (major, info) = self.read_type()?;

        match major {
            0 => self.read_uint(info),
            1 => self.read_nint(info),
            2 => self.read_bytes(info),
            3 => self.read_text(info),
            4 => self.read_array(info),
            5 => self.read_map(info),
            6 => self.read_tag(info),
            7 => self.read_simple(info),
            _ => Err(Error::InvalidMajorType(major)),
        }
    }

    fn read_type(&mut self) -> Result<(u8, u8)> {
        let byte = self.reader.read_u8()?;
        let major = byte >> 5;
        let info = byte & 0x1f;
        Ok((major, info))
    }
}
```

---

## Appendix A: Complete Type Reference

```
Major Type 0 (Unsigned):
0x00-0x17 = 0-23
0x18 xx = uint8
0x19 xx xx = uint16
0x1a xx xx xx xx = uint32
0x1b xx xx xx xx xx xx xx xx = uint64

Major Type 1 (Negative):
0x20-0x37 = -1 to -24
0x38 xx = -(uint8 + 1)
0x39 xx xx = -(uint16 + 1)
0x3a xx xx xx xx = -(uint32 + 1)
0x3b xx xx xx xx xx xx xx xx = -(uint64 + 1)

Major Type 2 (Bytes):
0x40-0x57 = bstr(0-23)
0x58 xx = bstr(uint8)
0x59 xx xx = bstr(uint16)
0x5a xx xx xx xx = bstr(uint32)
0x5b xx xx xx xx xx xx xx xx = bstr(uint64)
0x5f ... 0xff = indefinite

Major Type 3 (Text):
0x60-0x77 = tstr(0-23)
0x78 xx = tstr(uint8)
0x79 xx xx = tstr(uint16)
0x7a xx xx xx xx = tstr(uint32)
0x7b xx xx xx xx xx xx xx xx = tstr(uint64)
0x7f ... 0xff = indefinite

Major Type 4 (Array):
0x80-0x97 = arr(0-23)
0x98 xx = arr(uint8)
0x99 xx xx = arr(uint16)
0x9a xx xx xx xx = arr(uint32)
0x9b ... = arr(uint64)
0x9f ... 0xff = indefinite

Major Type 5 (Map):
0xa0-0xb7 = map(0-23)
0xb8 xx = map(uint8)
0xb9 xx xx = map(uint16)
0xba xx xx xx xx = map(uint32)
0xbb ... = map(uint64)
0xbf ... 0xff = indefinite

Major Type 6 (Tag):
0xc0-0xd7 = tag(0-23)
0xd8 xx = tag(uint8)
0xd9 xx xx = tag(uint16)
0xda xx xx xx xx = tag(uint32)
0xdb xx xx xx xx xx xx xx xx = tag(uint64)

Major Type 7 (Simple/Float):
0xe0-0xf3 = simple(0-19)
0xf4 = false
0xf5 = true
0xf6 = null
0xf7 = undefined
0xf8 xx = simple(uint8)
0xf9 xx xx = float16
0xfa xx xx xx xx = float32
0xfb xx xx xx xx xx xx xx xx = float64
```

---

*This document is a living textbook. Revisit sections as concepts become clearer through implementation. Next: [02-schema-validation-deep-dive.md](02-schema-validation-deep-dive.md)*
