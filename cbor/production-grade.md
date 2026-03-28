---
title: "Production-Grade CBOR Implementation"
subtitle: "Streaming, zero-copy, memory management, security, and operational excellence"
level: "Advanced - Production system engineers"
---

# Production-Grade CBOR Implementation

## Table of Contents

1. [Streaming Encoding/Decoding](#1-streaming-encodingdecoding)
2. [Zero-Copy Deserialization](#2-zero-copy-deserialization)
3. [Memory Management](#3-memory-management)
4. [Security Considerations](#4-security-considerations)
5. [Fuzzing and Testing](#5-fuzzing-and-testing)
6. [Performance Optimization](#6-performance-optimization)
7. [Monitoring and Observability](#7-monitoring-and-observability)

---

## 1. Streaming Encoding/Decoding

### 1.1 Why Streaming Matters

```
Problem: Large datasets don't fit in memory

Non-streaming approach:
1. Load entire dataset into memory
2. Serialize all at once
3. Write to output

Streaming approach:
1. Process items one at a time
2. Encode incrementally
3. Write chunks as ready

Memory usage:
Non-streaming: O(n) where n = total size
Streaming: O(1) where 1 = single item size
```

### 1.2 Streaming Encoder

```rust
use serde_cbor::{Serializer, Error};
use std::io::{Write, BufWriter};
use std::fs::File;

pub struct StreamingEncoder<W: Write> {
    serializer: Serializer<W>,
}

impl<W: Write> StreamingEncoder<W> {
    pub fn new(writer: W) -> Self {
        Self {
            serializer: Serializer::new(writer),
        }
    }

    pub fn write<T: serde::Serialize>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(&mut self.serializer)
    }

    pub fn finish(self) -> Result<W, Error> {
        Ok(self.serializer.into_inner())
    }
}

// Usage: Stream 1 million records to file
fn stream_to_file() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create("data.cbor")?;
    let buf_writer = BufWriter::new(file);
    let mut encoder = StreamingEncoder::new(buf_writer);

    for i in 0..1_000_000 {
        let record = Record {
            id: i,
            data: format!("Record {}", i),
        };
        encoder.write(&record)?;

        // Flush periodically for durability
        if i % 10000 == 0 {
            // Note: Need access to inner writer to flush
        }
    }

    Ok(())
}
```

### 1.3 Streaming Decoder

```rust
use serde_cbor::Deserializer;
use std::io::{Read, BufReader};
use std::fs::File;

pub struct StreamingDecoder<R: Read> {
    deserializer: Deserializer<R>,
}

impl<R: Read> StreamingDecoder<R> {
    pub fn new(reader: R) -> Self {
        Self {
            deserializer: Deserializer::from_reader(reader),
        }
    }

    pub fn iter<T: serde::de::DeserializeOwned>(
        self,
    ) -> impl Iterator<Item = Result<T, serde_cbor::Error>> {
        self.deserializer.into_iter::<T>()
    }
}

// Usage: Stream process 1 million records
fn stream_from_file() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("data.cbor")?;
    let buf_reader = BufReader::new(file);
    let decoder = StreamingDecoder::new(buf_reader);

    let mut count = 0;
    let mut sum = 0u64;

    for result in decoder.iter::<Record>() {
        let record = result?;
        count += 1;
        sum += record.id;
    }

    println!("Processed {} records, sum: {}", count, sum);
    Ok(())
}
```

### 1.4 Async Streaming (Without Tokio)

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};
use std::io::{Read, Write};

// Stream decoder task
pub struct StreamReaderTask<R: Read> {
    reader: Option<R>,
    buffer: Vec<u8>,
    offset: usize,
}

impl<R: Read + 'static> TaskIterator for StreamReaderTask<R> {
    type Ready = Option<Vec<u8>>;  // Next CBOR value bytes
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        // Read next CBOR item (simplified - need length prefix parsing)
        let mut buf = [0u8; 1024];
        match self.reader.as_mut()?.read(&mut buf) {
            Ok(0) => Some(TaskStatus::Ready(None)), // EOF
            Ok(n) => {
                self.buffer.extend_from_slice(&buf[..n]);
                Some(TaskStatus::Ready(Some(self.buffer.clone())))
            }
            Err(e) => {
                eprintln!("Read error: {}", e);
                Some(TaskStatus::Ready(None))
            }
        }
    }
}
```

### 1.5 Newline-Delimited CBOR (CBORL)

```rust
// Each line is a complete CBOR value
// Useful for log files, line-based protocols

use std::io::{BufRead, BufReader, Write};

// Encode as newline-delimited
fn encode_ndcbor<W: Write, T: serde::Serialize>(
    values: impl Iterator<Item = T>,
    mut writer: W,
) -> Result<(), serde_cbor::Error> {
    for value in values {
        let bytes = serde_cbor::to_vec(&value)?;
        writer.write_all(&bytes).unwrap();
        writer.write_all(b"\n").unwrap();
    }
    Ok(())
}

// Decode newline-delimited
fn decode_ndcbor<R: BufRead, T: serde::de::DeserializeOwned>(
    reader: R,
) -> impl Iterator<Item = Result<T, serde_cbor::Error>> {
    reader.lines().map(|line| {
        let bytes = line?.into_bytes();
        serde_cbor::from_slice(&bytes)
    })
}
```

---

## 2. Zero-Copy Deserialization

### 2.1 Borrowed Data with serde_cbor

```rust
use serde::Deserialize;
use serde_cbor::from_slice;

// Borrowed string (no allocation)
#[derive(Deserialize)]
struct BorrowedRecord<'a> {
    #[serde(borrow)]
    name: &'a str,
    #[serde(borrow)]
    data: &'a [u8],
}

fn zero_copy_decode(bytes: &[u8]) -> BorrowedRecord {
    from_slice(bytes).unwrap()
    // name and data borrow from input bytes
    // No heap allocation for strings
}
```

### 2.2 Cow for Flexible Borrowing

```rust
use std::borrow::Cow;
use serde::Deserialize;

#[derive(Deserialize)]
struct FlexibleRecord<'a> {
    // Borrows when possible, owns when needed
    name: Cow<'a, str>,
    values: Cow<'a, [u8]>,
}

// Works with both borrowed and owned input
fn decode_flexible(bytes: &[u8]) -> FlexibleRecord {
    serde_cbor::from_slice(bytes).unwrap()
}
```

### 2.3 ciborium Zero-Copy

```rust
use ciborium::de::from_reader;

// Direct slice borrowing
fn borrow_bytes(bytes: &[u8]) -> &[u8] {
    from_reader(&bytes[..]).unwrap()
}

// Borrowed string
fn borrow_str(bytes: &[u8]) -> &str {
    from_reader(&bytes[..]).unwrap()
}
```

### 2.4 Memory Layout Optimization

```rust
// Avoid this (causes allocations):
struct BadRecord {
    name: String,      // Always allocates
    data: Vec<u8>,     // Always allocates
}

// Prefer this (can borrow):
struct GoodRecord<'a> {
    name: &'a str,     // Borrows from input
    data: &'a [u8],    // Borrows from input
}

// Or use Cow for flexibility:
struct FlexibleRecord<'a> {
    name: Cow<'a, str>,
    data: Cow<'a, [u8]>,
}
```

---

## 3. Memory Management

### 3.1 Controlling Allocations

```rust
// Pre-allocate buffer for known sizes
fn encode_with_capacity(record: &Record) -> Vec<u8> {
    // Estimate size
    let estimated_size = record.estimated_size();

    // Pre-allocate
    let mut buf = Vec::with_capacity(estimated_size);

    // Encode
    serde_cbor::to_writer(&mut buf, record).unwrap();
    buf
}

// Reuse buffers
pub struct BufferPool {
    buffers: Vec<Vec<u8>>,
}

impl BufferPool {
    pub fn new(size: usize) -> Self {
        Self {
            buffers: (0..size).map(|_| Vec::with_capacity(4096)).collect(),
        }
    }

    pub fn acquire(&mut self) -> Vec<u8> {
        self.buffers.pop().unwrap_or_else(|| Vec::with_capacity(4096))
    }

    pub fn release(&mut self, mut buf: Vec<u8>) {
        buf.clear();
        if self.buffers.len() < 10 {
            self.buffers.push(buf);
        }
    }
}
```

### 3.2 Limiting Memory Usage

```rust
use serde_cbor::Deserializer;
use std::io::Read;

// Limit maximum depth to prevent stack overflow
struct LimitedDeserializer<R> {
    inner: Deserializer<R>,
    max_depth: usize,
    current_depth: usize,
}

impl<R: Read> LimitedDeserializer<R> {
    fn with_depth_limit(inner: Deserializer<R>, max_depth: usize) -> Self {
        Self {
            inner,
            max_depth,
            current_depth: 0,
        }
    }
}

// Limit allocation sizes
fn safe_decode<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    max_alloc: usize,
) -> Result<T, &'static str> {
    if bytes.len() > max_alloc {
        return Err("Input too large");
    }
    serde_cbor::from_slice(bytes).map_err(|_| "Decode error")
}
```

### 3.3 Arena Allocation for CBOR Values

```rust
use typed_arena::Arena;
use serde_cbor::Value;

// Parse many values into arena
fn parse_into_arena<'a>(
    arena: &'a Arena<Value>,
    bytes_list: &[&[u8]],
) -> Vec<&'a Value> {
    bytes_list
        .iter()
        .map(|bytes| {
            let value: Value = serde_cbor::from_slice(bytes).unwrap();
            arena.alloc(value)
        })
        .collect()
}

// All values freed when arena drops
```

---

## 4. Security Considerations

### 4.1 Input Validation

```rust
use serde_cbor::{Deserializer, Error};
use std::io::Read;

// Maximum sizes for security
const MAX_DEPTH: usize = 64;
const MAX_STRING_SIZE: usize = 10 * 1024 * 1024; // 10 MB
const MAX_ARRAY_SIZE: usize = 1_000_000;
const MAX_MAP_SIZE: usize = 100_000;

pub struct SecureDeserializer<R: Read> {
    inner: Deserializer<R>,
    depth: usize,
}

impl<R: Read> SecureDeserializer<R> {
    pub fn new(reader: R) -> Self {
        Self {
            inner: Deserializer::from_reader(reader),
            depth: 0,
        }
    }

    fn check_depth(&self) -> Result<(), Error> {
        if self.depth > MAX_DEPTH {
            return Err(Error::message("Maximum nesting depth exceeded"));
        }
        Ok(())
    }

    fn check_string_size(&self, size: usize) -> Result<(), Error> {
        if size > MAX_STRING_SIZE {
            return Err(Error::message("String too large"));
        }
        Ok(())
    }
}
```

### 4.2 Preventing DoS Attacks

```rust
// Attack vector: Deeply nested structures
// Malicious CBOR: 84 84 84 84 ... (nested arrays)

fn detect_nesting(bytes: &[u8]) -> Result<usize, &'static str> {
    let mut depth = 0;
    let mut max_depth = 0;

    for &byte in bytes {
        let major_type = byte >> 5;
        let additional = byte & 0x1f;

        match major_type {
            4 | 5 => { // Array or Map
                if additional < 24 {
                    depth += 1;
                    max_depth = max_depth.max(depth);
                }
            }
            _ => {}
        }

        if max_depth > MAX_DEPTH {
            return Err("Excessive nesting detected");
        }
    }

    Ok(max_depth)
}

// Attack vector: Billion laughs (exponential expansion)
fn check_expansion_ratio(bytes: &[u8], decoded_estimate: usize) -> Result<(), &'static str> {
    const MAX_EXPANSION: usize = 100; // 100x max

    if decoded_estimate > bytes.len() * MAX_EXPANSION {
        return Err("Potential exponential expansion attack");
    }
    Ok(())
}
```

### 4.3 Canonical CBOR for Security

```rust
// Canonical CBOR prevents:
// - Signature malleability
// - Hash collision attacks
// - Equivalence class attacks

use serde_cbor::ser::Serializer;
use std::io::Cursor;

fn to_canonical<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, serde_cbor::Error> {
    let mut vec = Vec::new();
    let mut serializer = Serializer::new(Cursor::new(&mut vec));

    // Enable canonical mode (hypothetical API)
    // - Shortest integer encoding
    // - Shortest float encoding
    // - Sorted map keys
    // - No indefinite length

    value.serialize(&mut serializer)?;
    Ok(vec)
}

// Verify canonical encoding
fn is_canonical(bytes: &[u8]) -> bool {
    // Check for shortest encoding
    // Check map key ordering
    // Check for indefinite length
    todo!()
}
```

### 4.4 Signature Verification with COSE

```rust
use cosey::{CoseSign1, CoseError};

// Verify COSE signature before processing
fn verify_and_process(signed_cbor: &[u8], public_key: &[u8]) -> Result<(), CoseError> {
    // Parse COSE_Sign1
    let sign1: CoseSign1 = serde_cbor::from_slice(signed_cbor)
        .map_err(|e| CoseError::ParseError(e.to_string()))?;

    // Verify signature
    sign1.verify(public_key)?;

    // Only process payload after verification
    let payload = sign1.payload;
    let data: MyData = serde_cbor::from_slice(&payload)?;

    // Process data...
    Ok(())
}
```

---

## 5. Fuzzing and Testing

### 5.1 Property-Based Testing with QuickCheck

```rust
use quickcheck::{Arbitrary, Gen, QuickCheck};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestData {
    id: u64,
    name: String,
    values: Vec<i32>,
}

impl Arbitrary for TestData {
    fn arbitrary(g: &mut Gen) -> Self {
        TestData {
            id: u64::arbitrary(g),
            name: String::arbitrary(g),
            values: Vec::arbitrary(g),
        }
    }
}

#[test]
fn test_roundtrip() {
    fn prop(data: TestData) -> bool {
        let bytes = serde_cbor::to_vec(&data).unwrap();
        let decoded: TestData = serde_cbor::from_slice(&bytes).unwrap();
        data == decoded
    }

    QuickCheck::new().quickcheck(prop as fn(TestData) -> bool);
}
```

### 5.2 Fuzzing with cargo-fuzz

```rust
// fuzz/fuzz_targets/decode.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz deserialization
    let _result: Result<serde_cbor::Value, _> = serde_cbor::from_slice(data);

    // Fuzz with type hint
    let _result: Result<Vec<u8>, _> = serde_cbor::from_slice(data);
    let _result: Result<String, _> = serde_cbor::from_slice(data);
    let _result: Result<std::collections::BTreeMap<String, String>, _> = serde_cbor::from_slice(data);
});

// Run fuzzer:
// cargo fuzz run decode
```

### 5.3 Edge Case Testing

```rust
#[test]
fn test_edge_cases() {
    // Empty structures
    assert_eq!(serde_cbor::to_vec(&Vec::<u8>::new()).unwrap(), vec![0x80]);
    assert_eq!(serde_cbor::to_vec(&std::collections::BTreeMap::<String, String>::new()).unwrap(), vec![0xa0]);

    // Boundary values
    assert_eq!(serde_cbor::to_vec(&23u8).unwrap(), vec![0x17]);
    assert_eq!(serde_cbor::to_vec(&24u8).unwrap(), vec![0x18, 0x18]);
    assert_eq!(serde_cbor::to_vec(&255u8).unwrap(), vec![0x18, 0xff]);
    assert_eq!(serde_cbor::to_vec(&256u16).unwrap(), vec![0x19, 0x01, 0x00]);

    // Negative numbers
    assert_eq!(serde_cbor::to_vec(&-1i8).unwrap(), vec![0x20]);
    assert_eq!(serde_cbor::to_vec(&-256i16).unwrap(), vec![0x38, 0xff]);

    // Floats
    assert_eq!(serde_cbor::to_vec(&0.0f64).unwrap(), vec![0xf9, 0x00, 0x00]);
    assert_eq!(serde_cbor::to_vec(&1.0f64).unwrap(), vec![0xf9, 0x3c, 0x00]);

    // Special values
    assert_eq!(serde_cbor::to_vec(&true).unwrap(), vec![0xf5]);
    assert_eq!(serde_cbor::to_vec(&false).unwrap(), vec![0xf4]);
    assert_eq!(serde_cbor::to_vec(&()).unwrap(), vec![0xf6]);
}
```

### 5.4 Regression Testing

```rust
// Test vectors from RFC 7049
#[test]
fn test_rfc_vectors() {
    // Integer test vectors
    assert_eq!(serde_cbor::to_vec(&0u8).unwrap(), &[0x00]);
    assert_eq!(serde_cbor::to_vec(&1u8).unwrap(), &[0x01]);
    assert_eq!(serde_cbor::to_vec(&10u8).unwrap(), &[0x0a]);
    assert_eq!(serde_cbor::to_vec(&23u8).unwrap(), &[0x17]);
    assert_eq!(serde_cbor::to_vec(&24u8).unwrap(), &[0x18, 0x18]);
    assert_eq!(serde_cbor::to_vec(&25u8).unwrap(), &[0x18, 0x19]);
    assert_eq!(serde_cbor::to_vec(&100u8).unwrap(), &[0x18, 0x64]);
    assert_eq!(serde_cbor::to_vec(&1000u16).unwrap(), &[0x19, 0x03, 0xe8]);

    // String test vectors
    assert_eq!(serde_cbor::to_vec(&"a").unwrap(), &[0x61, 0x61]);
    assert_eq!(serde_cbor::to_vec(&"IETF").unwrap(), &[0x64, 0x49, 0x45, 0x54, 0x46]);
    assert_eq!(serde_cbor::to_vec(&"\"\\").unwrap(), &[0x62, 0x22, 0x5c]);

    // Verify decode of test vectors
    let decoded: u8 = serde_cbor::from_slice(&[0x18, 0x18]).unwrap();
    assert_eq!(decoded, 24);
}
```

---

## 6. Performance Optimization

### 6.1 Benchmarking Setup

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_cbor(c: &mut Criterion) {
    let data = LargeData::generate();

    c.bench_function("cbor_serialize", |b| {
        b.iter(|| serde_cbor::to_vec(black_box(&data)))
    });

    let bytes = serde_cbor::to_vec(&data).unwrap();
    c.bench_function("cbor_deserialize", |b| {
        b.iter(|| serde_cbor::from_slice::<LargeData>(black_box(&bytes)))
    });
}

fn bench_packed(c: &mut Criterion) {
    let data = LargeData::generate();

    c.bench_function("cbor_packed_serialize", |b| {
        b.iter(|| serde_cbor::ser::to_vec_packed(black_box(&data)))
    });
}

criterion_group!(benches, bench_cbor, bench_packed);
criterion_main!(benches);
```

### 6.2 Optimizing Hot Paths

```rust
// Pre-compute CBOR for static data
lazy_static::lazy_static! {
    static ref STATIC_RESPONSE: Vec<u8> = {
        let response = Response {
            status: "ok",
            version: "1.0",
        };
        serde_cbor::to_vec(&response).unwrap()
    };
}

fn get_static_response() -> &'static [u8] {
    &STATIC_RESPONSE
}

// Avoid repeated serialization in loops
fn process_batch(records: &[Record]) -> Vec<u8> {
    let mut output = Vec::with_capacity(records.len() * 100);

    for record in records {
        serde_cbor::to_writer(&mut output, record).unwrap();
    }

    output
}
```

### 6.3 Parallel Processing

```rust
use rayon::prelude::*;

// Parallel encode
fn parallel_encode(records: &[Record]) -> Vec<Vec<u8>> {
    records
        .par_iter()
        .map(|record| serde_cbor::to_vec(record).unwrap())
        .collect()
}

// Parallel decode
fn parallel_decode(bytes_list: &[Vec<u8>]) -> Vec<Record> {
    bytes_list
        .par_iter()
        .map(|bytes| serde_cbor::from_slice(bytes).unwrap())
        .collect()
}
```

---

## 7. Monitoring and Observability

### 7.1 Metrics Collection

```rust
use prometheus::{IntCounter, Histogram, register_int_counter, register_histogram};

struct CborMetrics {
    encode_count: IntCounter,
    decode_count: IntCounter,
    encode_bytes: Histogram,
    decode_bytes: Histogram,
    encode_duration: Histogram,
    decode_duration: Histogram,
}

impl CborMetrics {
    fn new() -> Result<Self, prometheus::Error> {
        Ok(Self {
            encode_count: register_int_counter!("cbor_encode_total", "Total encodes")?,
            decode_count: register_int_counter!("cbor_decode_total", "Total decodes")?,
            encode_bytes: register_histogram!("cbor_encode_bytes", "Encoded bytes")?,
            decode_bytes: register_histogram!("cbor_decode_bytes", "Decoded bytes")?,
            encode_duration: register_histogram!("cbor_encode_duration", "Encode duration")?,
            decode_duration: register_histogram!("cbor_decode_duration", "Decode duration")?,
        })
    }

    fn record_encode(&self, bytes: usize, duration: f64) {
        self.encode_count.inc();
        self.encode_bytes.observe(bytes as f64);
        self.encode_duration.observe(duration);
    }

    fn record_decode(&self, bytes: usize, duration: f64) {
        self.decode_count.inc();
        self.decode_bytes.observe(bytes as f64);
        self.decode_duration.observe(duration);
    }
}
```

### 7.2 Error Tracking

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CborAppError {
    #[error("Serialization failed: {0}")]
    Serialize(#[from] serde_cbor::Error),

    #[error("Deserialization failed at offset {offset}: {source}")]
    Deserialize {
        offset: u64,
        #[source]
        source: serde_cbor::Error,
    },

    #[error("Size limit exceeded: {size} > {limit}")]
    SizeLimitExceeded { size: usize, limit: usize },

    #[error("Nesting depth exceeded: {depth} > {limit}")]
    NestingDepthExceeded { depth: usize, limit: usize },
}

// Track error rates
fn track_error(error: &CborAppError) {
    match error {
        CborAppError::Serialize(_) => {
            metrics::counter!("cbor_errors", "type" => "serialize").increment(1);
        }
        CborAppError::Deserialize { .. } => {
            metrics::counter!("cbor_errors", "type" => "deserialize").increment(1);
        }
        CborAppError::SizeLimitExceeded { .. } => {
            metrics::counter!("cbor_errors", "type" => "size_limit").increment(1);
        }
        CborAppError::NestingDepthExceeded { .. } => {
            metrics::counter!("cbor_errors", "type" => "nesting").increment(1);
        }
    }
}
```

---

## Appendix A: Production Checklist

```
Security:
□ Input size limits configured
□ Maximum nesting depth enforced
□ Canonical CBOR for signed data
□ COSE signature verification
□ Fuzzing enabled in CI

Performance:
□ Streaming for large datasets
□ Zero-copy where applicable
□ Buffer pooling for high-throughput
□ Metrics and alerts configured
□ Benchmarks in CI

Reliability:
□ Property-based tests
□ RFC test vectors
□ Edge case coverage
□ Error handling tested
□ Recovery procedures documented

Operations:
□ Logging enabled
□ Metrics exported
□ Alerting configured
□ Runbooks documented
□ Incident response plan
```

---

*This document is a living textbook. Revisit sections as concepts become clearer through implementation. Next: [05-valtron-integration.md](05-valtron-integration.md)*
