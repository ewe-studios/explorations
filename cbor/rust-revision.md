---
title: "CBOR Rust Revision: Complete Implementation Guide"
subtitle: "serde_cbor, ciborium, cbor-codec - Comprehensive Rust CBOR ecosystem guide"
based_on: "Source: /home/darkvoid/Boxxed/@formulas/src.rust/src.cbor/"
level: "Intermediate to Advanced - Rust developers"
---

# CBOR Rust Revision: Complete Implementation Guide

## Table of Contents

1. [Library Overview](#1-library-overview)
2. [serde_cbor: Type-Safe Serialization](#2-serde_cbor-type-safe-serialization)
3. [ciborium: no_std CBOR](#3-ciborium-no_std-cbor)
4. [cbor-codec: Low-Level Control](#4-cbor-codec-low-level-control)
5. [Custom Derive and Macros](#5-custom-derive-and-macros)
6. [Error Handling Patterns](#6-error-handling-patterns)
7. [Integration with ewe_platform](#7-integration-with-ewe_platform)

---

## 1. Library Overview

### 1.1 CBOR Libraries in Rust

```
┌─────────────────────────────────────────────────────────┐
│              Rust CBOR Ecosystem                          │
├─────────────────────────────────────────────────────────┤
│ Library      │ Feature          │ no_std │ Serde       │
├─────────────────────────────────────────────────────────┤
│ serde_cbor   │ Type-safe        │ No     │ Yes         │
│ ciborium     │ no_std support   │ Yes    │ Yes         │
│ cbor-codec   │ Low-level        │ Partial│ No          │
│ minicbor     │ Minimal/no_std   │ Yes    │ Optional    │
│ speedy       │ Zero-copy        │ Yes    │ No          │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Source Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.cbor/
├── cbor/              # serde_cbor implementation
│   ├── src/
│   │   ├── lib.rs     # Main exports
│   │   ├── de.rs      # Deserializer
│   │   ├── ser.rs     # Serializer
│   │   ├── error.rs   # Error types
│   │   ├── tags.rs    # Tag handling
│   │   ├── value/     # Untyped Value
│   │   ├── read.rs    # Input abstraction
│   │   └── write.rs   # Output abstraction
│   └── tests/
│
├── cbor-codec/        # Low-level codec
│   ├── src/
│   │   ├── lib.rs     # Exports
│   │   ├── types.rs   # CBOR types
│   │   ├── value.rs   # Value AST
│   │   ├── encoder.rs # Encoder
│   │   └── decoder.rs # Decoder
│   └── tests/
│
└── rust-cbor/         # Alternative implementation
    ├── src/
    │   ├── lib.rs
    │   ├── encoder.rs
    │   └── decoder.rs
    └── cbor_conv/     # Conversion utilities
```

### 1.3 Choosing the Right Library

```
Use serde_cbor when:
- Full std support available
- Type-safe serde integration needed
- Standard library features required

Use ciborium when:
- no_std environment (embedded, WASM)
- Serde integration still needed
- alloc feature available

Use cbor-codec when:
- Low-level control required
- Direct encoding/decoding
- Custom value types
- Learning CBOR internals
```

---

## 2. serde_cbor: Type-Safe Serialization

### 2.1 Basic Setup

```toml
# Cargo.toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_cbor = "0.11"
```

### 2.2 Simple Serialization

```rust
use serde::{Serialize, Deserialize};
use serde_cbor::{to_vec, from_slice};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Person {
    name: String,
    age: u32,
    active: bool,
}

fn main() -> Result<(), serde_cbor::Error> {
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
        active: true,
    };

    // Serialize to CBOR bytes
    let bytes = to_vec(&person)?;
    println!("CBOR: {:02x?}", bytes);

    // Deserialize back
    let decoded: Person = from_slice(&bytes)?;
    assert_eq!(person, decoded);

    Ok(())
}
```

### 2.3 Packed Encoding

```rust
use serde_cbor::ser::to_vec_packed;

// Packed encoding uses integers for field names
// More compact but requires exact field order

#[derive(Serialize, Deserialize)]
struct Point {
    x: f64,
    y: f64,
}

let point = Point { x: 1.0, y: 2.0 };

// Normal encoding: {"x": 1.0, "y": 2.0}
let normal = serde_cbor::to_vec(&point).unwrap();

// Packed encoding: {0: 1.0, 1: 2.0}
let packed = to_vec_packed(&point).unwrap();

println!("Normal: {} bytes", normal.len());
println!("Packed: {} bytes", packed.len());
```

### 2.4 Working with Tags

```rust
use serde_cbor::tags::{Tag, Tagged};

// Tagged values for semantic types
let timestamp = Tagged {
    tag: Some(1), // Epoch time
    value: 1632844800u64,
};

let bytes = serde_cbor::to_vec(&timestamp).unwrap();

// Deserialize
let decoded: Tagged<u64> = serde_cbor::from_slice(&bytes).unwrap();
assert_eq!(decoded.tag, Some(1));
assert_eq!(decoded.value, 1632844800);
```

### 2.5 Using Value for Dynamic CBOR

```rust
use serde_cbor::Value;
use std::collections::BTreeMap;

// Parse CBOR without known type
let bytes = vec![0xa2, 0x61, 0x61, 0x01, 0x61, 0x62, 0x02];
let value: Value = serde_cbor::from_slice(&bytes).unwrap();

// Work with dynamic value
if let Value::Map(map) = value {
    for (key, val) in map {
        println!("{:?}: {:?}", key, val);
    }
}

// Build CBOR dynamically
let mut map = BTreeMap::new();
map.insert(Value::Text("key".to_string()), Value::Integer(42));
let bytes = serde_cbor::to_vec(&Value::Map(map)).unwrap();
```

### 2.6 Custom Serialization

```rust
use serde::{Serializer, Deserializer};
use serde::ser::SerializeStruct;

#[derive(Debug)]
struct Timestamp(u64);

impl serde::Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as tagged value
        use serde_cbor::tags::Tagged;
        let tagged = Tagged {
            tag: Some(1),
            value: self.0,
        };
        tagged.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tagged: Tagged<u64> = Tagged::deserialize(deserializer)?;
        Ok(Timestamp(tagged.value))
    }
}
```

### 2.7 Streaming Serialization

```rust
use serde_cbor::{Serializer, Deserializer};
use std::io::{Write, Read};

// Stream multiple values to a writer
fn stream_encode<W: Write>(values: &[u32], writer: W) -> Result<(), serde_cbor::Error> {
    let mut ser = Serializer::new(writer);
    for value in values {
        value.serialize(&mut ser)?;
    }
    Ok(())
}

// Stream decode multiple values from a reader
fn stream_decode<R: Read>(reader: R) -> Result<Vec<u32>, serde_cbor::Error> {
    let de = Deserializer::from_reader(reader);
    de.into_iter::<u32>()
        .collect::<Result<Vec<_>, _>>()
}
```

---

## 3. ciborium: no_std CBOR

### 3.1 Setup for no_std

```toml
# Cargo.toml
[dependencies]
ciborium = { version = "0.2", default-features = false }
ciborium-io = { version = "0.2", default-features = false }
ciborium-ll = { version = "0.2", default-features = false }

# With alloc (for Vec, String, etc.)
ciborium = "0.2"
```

### 3.2 Basic Usage

```rust
#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use ciborium::{to_writer, from_reader};
use ciborium_io::Write;

#[derive(serde::Serialize, serde::Deserialize)]
struct SensorData {
    temperature: f32,
    humidity: f32,
}

fn encode_data(data: &SensorData) -> Vec<u8> {
    let mut buf = Vec::new();
    to_writer(data, &mut buf).unwrap();
    buf
}

fn decode_data(bytes: &[u8]) -> SensorData {
    from_reader(&bytes[..]).unwrap()
}
```

### 3.3 Half-Precision Floats

```rust
use ciborium::value::Value;
use half::f16;

// ciborium supports f16 natively
let half_float = f16::from_f32(3.14);

let value = Value::Float(half_float.into());
let bytes = ciborium::to_vec(&value).unwrap();

// Decode
let decoded: Value = ciborium::from_slice(&bytes).unwrap();
```

### 3.4 Zero-Copy Deserialization

```rust
use ciborium::de::from_reader;
use core::str;

// Borrowed strings from CBOR
fn decode_borrowed<'a>(bytes: &'a [u8]) -> &'a str {
    let value: &str = from_reader(&bytes[..]).unwrap();
    value
}

// Zero-copy byte slices
fn decode_bytes<'a>(bytes: &'a [u8]) -> &'a [u8] {
    let value: &[u8] = from_reader(&bytes[..]).unwrap();
    value
}
```

---

## 4. cbor-codec: Low-Level Control

### 4.1 Direct Encoding

```rust
use cbor::{Encoder, Config};
use std::io::Cursor;

// Low-level encoding without serde
fn encode_direct() -> Vec<u8> {
    let mut encoder = Encoder::new(Vec::new());

    // Encode various types directly
    encoder.u64(42).unwrap();           // Unsigned integer
    encoder.i64(-100).unwrap();         // Negative integer
    encoder.text("Hello").unwrap();     // Text string
    encoder.bytes(&[0x01, 0x02]).unwrap(); // Byte string

    // Array
    encoder.array(3).unwrap();
    encoder.u8(1).unwrap();
    encoder.u8(2).unwrap();
    encoder.u8(3).unwrap();

    // Map
    encoder.object(2).unwrap();
    encoder.text("key").unwrap();
    encoder.u64(42).unwrap();
    encoder.text("value").unwrap();
    encoder.text("data").unwrap();

    encoder.into_writer()
}
```

### 4.2 Direct Decoding

```rust
use cbor::{Decoder, Config};
use std::io::Cursor;

fn decode_direct(bytes: Vec<u8>) {
    let mut decoder = Decoder::new(Config::default(), Cursor::new(bytes));

    // Decode values one by one
    while let Some(value) = decoder.value().ok() {
        match value {
            cbor::value::Value::U64(n) => println!("Number: {}", n),
            cbor::value::Value::Text(s) => println!("String: {}", s),
            cbor::value::Value::Bytes(b) => println!("Bytes: {:?}", b),
            cbor::value::Value::Array(arr) => println!("Array: {:?}", arr),
            cbor::value::Value::Object(obj) => println!("Map: {:?}", obj),
            _ => println!("Other: {:?}", value),
        }
    }
}
```

### 4.3 Working with Tags

```rust
use cbor::{Encoder, Decoder, Config, Tag};
use cbor::value::Value;

// Encode with tag
fn encode_tagged() -> Vec<u8> {
    let mut encoder = Encoder::new(Vec::new());

    // Tag 1 (epoch time) with value
    encoder.tagged(Tag::Timestamp).unwrap();
    encoder.u64(1632844800).unwrap();

    encoder.into_writer()
}

// Decode with tag handling
fn decode_tagged(bytes: Vec<u8>) {
    let mut decoder = Decoder::new(Config::default(), Cursor::new(bytes));

    if let Some(value) = decoder.value().ok() {
        if let Value::Tagged(tag, inner) = value {
            println!("Tag: {:?}", tag);
            println!("Value: {:?}", *inner);
        }
    }
}
```

### 4.4 Skip and Peek

```rust
use cbor::{Decoder, Config, Skip};

// Skip values without decoding
fn skip_values(bytes: Vec<u8>) {
    let mut decoder = Decoder::new(Config::default(), Cursor::new(bytes));

    // Skip next value
    decoder.skip().unwrap();

    // Peek at type without consuming
    if let Ok(Some((typ, info))) = decoder.peek_type() {
        println!("Next type: {:?}, info: {}", typ, info);
    }
}
```

---

## 5. Custom Derive and Macros

### 5.1 Serde Field Attributes

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Message {
    // Rename field in CBOR
    #[serde(rename = "id")]
    message_id: u64,

    // Skip serialization
    #[serde(skip)]
    cached_hash: Option<u64>,

    // Skip serializing if None
    #[serde(skip_serializing_if = "Option::is_none")]
    optional_field: Option<String>,

    // Default value for deserialization
    #[serde(default)]
    count: u32,

    // Custom default
    #[serde(default = "default_priority")]
    priority: u8,
}

fn default_priority() -> u8 {
    1
}
```

### 5.2 Enum Representation

```rust
use serde::{Serialize, Deserialize};

// Externally tagged (default)
#[derive(Serialize, Deserialize)]
enum Message {
    Text(String),
    Number(f64),
    Binary(Vec<u8>),
}
// Encodes as: {"Text": "hello"} or ["Text", "hello"] (packed)

// Internally tagged
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum Command {
    Move { x: i32, y: i32 },
    Rotate { angle: f64 },
}
// Encodes as: {"type": "Move", "x": 1, "y": 2}

// Adjacently tagged
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
enum Response {
    Success(String),
    Error(String),
}
// Encodes as: {"type": "Success", "data": "OK"}

// Untagged
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Value {
    Number(f64),
    Text(String),
}
// Encodes as: 3.14 or "hello" (no tag)
```

### 5.3 Transparent Newtypes

```rust
use serde::{Serialize, Deserialize};

// Transparent wrapper
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct UserId(u64);

// Encodes as just the inner value (u64)
let user_id = UserId(12345);
let bytes = serde_cbor::to_vec(&user_id).unwrap();
// bytes contain just 12345, not a struct
```

---

## 6. Error Handling Patterns

### 6.1 Error Types

```rust
use serde_cbor::Error;
use std::fmt;

pub enum CborAppError {
    Serialization(serde_cbor::Error),
    Deserialization(serde_cbor::Error),
    Io(std::io::Error),
    Validation(String),
}

impl From<serde_cbor::Error> for CborAppError {
    fn from(err: serde_cbor::Error) -> Self {
        CborAppError::Serialization(err)
    }
}

impl From<std::io::Error> for CborAppError {
    fn from(err: std::io::Error) -> Self {
        CborAppError::Io(err)
    }
}

// Error classification
fn handle_error(err: serde_cbor::Error) {
    match err.classify() {
        serde_cbor::Category::Io => eprintln!("IO error"),
        serde_cbor::Category::Syntax => eprintln!("Invalid CBOR syntax"),
        serde_cbor::Category::Data => eprintln!("Semantic error in data"),
        serde_cbor::Category::Eof => eprintln!("Unexpected end of input"),
    }

    // Get byte offset of error
    println!("Error at offset: {}", err.offset());
}
```

### 6.2 Result Aliases

```rust
use serde_cbor::Result;

// Type alias for common Result
type CborResult<T> = Result<T>;

fn encode_message(msg: &Message) -> CborResult<Vec<u8>> {
    serde_cbor::to_vec(msg)
}

fn decode_message(bytes: &[u8]) -> CborResult<Message> {
    serde_cbor::from_slice(bytes)
}
```

### 6.3 Recoverable Errors

```rust
use serde_cbor::Deserializer;

// Try to decode, skip on error
fn resilient_decode(bytes: &[u8]) -> Vec<Message> {
    let mut results = Vec::new();
    let mut deserializer = Deserializer::from_slice(bytes);

    for result in deserializer.into_iter::<Message>() {
        match result {
            Ok(msg) => results.push(msg),
            Err(e) => {
                eprintln!("Skipping malformed message: {}", e);
                // Continue with next value
            }
        }
    }

    results
}
```

---

## 7. Integration with ewe_platform

### 7.1 Valtron Task for CBOR Encoding

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};
use serde::Serialize;

// CBOR encoding task
pub struct CborEncodeTask<T> {
    data: Option<T>,
}

impl<T: Serialize> CborEncodeTask<T> {
    pub fn new(data: T) -> Self {
        Self { data: Some(data) }
    }
}

impl<T: Serialize + 'static> TaskIterator for CborEncodeTask<T> {
    type Ready = Result<Vec<u8>, serde_cbor::Error>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if let Some(data) = self.data.take() {
            let result = serde_cbor::to_vec(&data);
            Some(TaskStatus::Ready(result))
        } else {
            None
        }
    }
}
```

### 7.2 CBOR Decoding Task

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};
use serde::de::DeserializeOwned;

pub struct CborDecodeTask<T> {
    bytes: Option<Vec<u8>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned> CborDecodeTask<T> {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Some(bytes),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: DeserializeOwned + 'static> TaskIterator for CborDecodeTask<T> {
    type Ready = Result<T, serde_cbor::Error>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if let Some(bytes) = self.bytes.take() {
            let result = serde_cbor::from_slice(&bytes);
            Some(TaskStatus::Ready(result))
        } else {
            None
        }
    }
}
```

### 7.3 Complete Example: Message Protocol

```rust
use serde::{Serialize, Deserialize};
use foundation_core::valtron::{single::{spawn, run_until_complete}, FnReady};

// Protocol messages
#[derive(Serialize, Deserialize, Debug)]
enum ProtocolMessage {
    Request { id: u64, method: String, params: serde_cbor::Value },
    Response { id: u64, result: serde_cbor::Value },
    Error { id: u64, code: i32, message: String },
}

// Encode task
struct EncodeMessageTask {
    message: Option<ProtocolMessage>,
}

impl TaskIterator for EncodeMessageTask {
    type Ready = Vec<u8>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if let Some(msg) = self.message.take() {
            let bytes = serde_cbor::to_vec(&msg).expect("serialize");
            Some(TaskStatus::Ready(bytes))
        } else {
            None
        }
    }
}

// Usage in ewe_platform
fn send_message(message: ProtocolMessage) {
    spawn()
        .with_task(EncodeMessageTask { message: Some(message) })
        .with_resolver(Box::new(FnReady::new(|bytes, _exec| {
            // Send bytes over network
            println!("Sending {} bytes", bytes.len());
        })))
        .schedule()
        .unwrap();

    run_until_complete();
}
```

---

## Appendix A: Library Quick Reference

```
serde_cbor:
- to_vec(&T) -> Result<Vec<u8>>
- from_slice(&[u8]) -> Result<T>
- to_writer(&T, &mut W) -> Result<()>
- from_reader(&mut R) -> Result<T>
- Value enum for dynamic CBOR
- Tagged<T> for semantic tags

ciborium:
- to_writer(&T, &mut W) -> Result<(), Error>
- from_reader(&mut R) -> Result<T, Error>
- Works with no_std + alloc
- half::f16 support

cbor-codec:
- Encoder::new(W) -> Encoder<W>
- Decoder::new(Config, R) -> Decoder<R>
- Direct type encoding (u64, text, bytes, etc.)
- Skip and peek operations
```

---

*This document is a living textbook. Revisit sections as concepts become clearer through implementation. Next: [production-grade.md](production-grade.md)*
