# Storage System Implementation Guide for Engineers

**A step-by-step guide to building a time-series storage system from scratch**

---

## Table of Contents

1. [Introduction](#introduction)
2. [Prerequisites](#prerequisites)
3. [Step 1: Basic Data Structures](#step-1-basic-data-structures)
4. [Step 2: In-Memory Storage](#step-2-in-memory-storage)
5. [Step 3: File Persistence](#step-3-file-persistence)
6. [Step 4: Compression](#step-4-compression)
7. [Step 5: Query Engine](#step-5-query-engine)
8. [Step 6: Time-Partitioning](#step-6-time-partitioning)
9. [Common Pitfalls](#common-pitfalls)
10. [Next Steps](#next-steps)

---

## Introduction

This guide walks you through building a simple time-series storage system. We'll start from zero and build up to a functional system that can:

- Store time-series data efficiently
- Compress data to save space
- Query data by time ranges
- Handle high write volumes

**What you'll build:**

```
Week 1-2: Basic storage (in-memory)
Week 3-4: File persistence
Week 5-6: Compression
Week 7-8: Query engine and partitioning
```

---

## Prerequisites

### Required Knowledge

1. **Programming**: Comfortable with Rust or similar language
2. **Data Structures**: Understand arrays, maps, trees
3. **Basic I/O**: File reading/writing

### Setup

```bash
# Create project
cargo new tsdb-learning
cd tsdb-learning

# Add dependencies
cargo add chrono
cargo add serde
cargo add serde_json
cargo add memmap2
cargo add lz4_flex
```

---

## Step 1: Basic Data Structures

### Define Core Types

```rust
// src/types.rs

/// Timestamp in microseconds since epoch
pub type Timestamp = i64;

/// A single data point
#[derive(Debug, Clone)]
pub struct DataPoint {
    pub time: Timestamp,
    pub value: f64,
}

/// A collection of data points (a time series)
pub type TimeSeries = Vec<DataPoint>;

/// Result type for our operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    InvalidData(String),
    NotFound,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}
```

### Why These Types?

- **Timestamp as i64**: Fixed size, easy to compare, no timezone issues
- **Value as f64**: Handles most numeric data (sensor readings, metrics)
- **Vec<DataPoint>**: Simple, contiguous memory layout

---

## Step 2: In-Memory Storage

### Basic Storage Engine

```rust
// src/storage.rs

use std::collections::BTreeMap;
use crate::types::{DataPoint, Timestamp, Result, Error};

/// Simple in-memory storage for a single time series
pub struct InMemoryStorage {
    /// Data points stored in time order
    points: Vec<DataPoint>,
    /// Maximum points to keep (for memory management)
    max_points: usize,
}

impl InMemoryStorage {
    pub fn new(max_points: usize) -> Self {
        Self {
            points: Vec::with_capacity(1000),
            max_points,
        }
    }

    /// Add a data point
    pub fn write(&mut self, time: Timestamp, value: f64) {
        let point = DataPoint { time, value };

        // Keep points sorted by time
        match self.points.binary_search_by(|p| p.time.cmp(&time)) {
            Ok(idx) => self.points[idx] = point,  // Update existing
            Err(idx) => self.points.insert(idx, point),  // Insert new
        }

        // Remove oldest points if over limit
        while self.points.len() > self.max_points {
            self.points.remove(0);
        }
    }

    /// Get points in a time range
    pub fn read_range(&self, start: Timestamp, end: Timestamp) -> Vec<&DataPoint> {
        self.points
            .iter()
            .filter(|p| p.time >= start && p.time < end)
            .collect()
    }

    /// Get the latest N points
    pub fn read_latest(&self, n: usize) -> Vec<&DataPoint> {
        self.points.iter().rev().take(n).rev().collect()
    }

    /// Get total points stored
    pub fn len(&self) -> usize {
        self.points.len()
    }
}
```

### Usage Example

```rust
// src/main.rs

use tsdb_learning::storage::InMemoryStorage;

fn main() {
    let mut storage = InMemoryStorage::new(10000);

    // Write some data
    for i in 0..100 {
        storage.write(i * 1000, i as f64 * 1.5);
    }

    // Read data
    let points = storage.read_range(0, 50000);
    println!("Read {} points", points.len());

    // Get latest
    let latest = storage.read_latest(10);
    for point in latest {
        println!("Time: {}, Value: {}", point.time, point.value);
    }
}
```

### Key Concepts Learned

1. **Sorted Storage**: Binary search for efficient lookups
2. **Memory Limits**: Prevent unbounded growth
3. **Range Queries**: Filter by time range

---

## Step 3: File Persistence

### SSTable Format

Now let's persist data to disk using an SSTable-like format:

```rust
// src/sstable.rs

use std::fs::File;
use std::io::{Read, Write, Seek, SeekFrom};
use crate::types::{DataPoint, Timestamp, Result, Error};

/// Simple SSTable structure
///
/// File Layout:
/// ┌────────────────────────────────────────┐
/// │              Header (16 bytes)          │
/// │  - Magic number (4 bytes)               │
/// │  - Entry count (4 bytes)                │
/// │  - Index offset (8 bytes)               │
/// ├────────────────────────────────────────┤
/// │              Data Section               │
/// │  - Entry 1: (time: i64, value: f64)     │
/// │  - Entry 2: (time: i64, value: f64)     │
/// │  - ...                                  │
/// ├────────────────────────────────────────┤
/// │              Index Section              │
/// │  - Sparse index for binary search       │
/// ├────────────────────────────────────────┤
/// │              Footer (8 bytes)           │
/// │  - Index length (4 bytes)               │
/// │  - Magic end (4 bytes)                  │
/// └────────────────────────────────────────┘

const MAGIC_START: u32 = 0x54534442;  // "TSDB"
const MAGIC_END: u32 = 0x42445354;    // "BDST"

pub struct SSTableWriter {
    file: File,
    entry_count: u32,
    data_start: u64,
}

impl SSTableWriter {
    pub fn create(path: &str) -> Result<Self> {
        let mut file = File::create(path)?;

        // Write placeholder header
        file.write_all(&MAGIC_START.to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?;  // Entry count (placeholder)
        file.write_all(&0u64.to_le_bytes())?;  // Index offset (placeholder)

        Ok(Self {
            file,
            entry_count: 0,
            data_start: 16,  // Header size
        })
    }

    pub fn write(&mut self, time: Timestamp, value: f64) -> Result<()> {
        // Write data entry
        self.file.write_all(&time.to_le_bytes())?;
        self.file.write_all(&value.to_le_bytes())?;
        self.entry_count += 1;
        Ok(())
    }

    pub fn finish(mut self, index: Vec<(Timestamp, u64)>) -> Result<u64> {
        let data_end = self.file.seek(SeekFrom::Current(0))?;

        // Write index
        for (time, offset) in &index {
            self.file.write_all(&time.to_le_bytes())?;
            self.file.write_all(&offset.to_le_bytes())?;
        }

        let index_end = self.file.seek(SeekFrom::Current(0))?;

        // Write footer
        self.file.write_all(&((index_end - data_end) as u32).to_le_bytes())?;
        self.file.write_all(&MAGIC_END.to_le_bytes())?;

        // Go back and update header
        self.file.seek(SeekFrom::Start(4))?;
        self.file.write_all(&self.entry_count.to_le_bytes())?;
        self.file.seek(SeekFrom::Start(8))?;
        self.file.write_all(&data_end.to_le_bytes())?;

        Ok(index_end)
    }
}

pub struct SSTableReader {
    file: File,
    entry_count: u32,
    index: Vec<(Timestamp, u64)>,
}

impl SSTableReader {
    pub fn open(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;

        // Read and verify header
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if u32::from_le_bytes(magic) != MAGIC_START {
            return Err(Error::InvalidData("Invalid magic number".into()));
        }

        let mut entry_count = [0u8; 4];
        file.read_exact(&mut entry_count)?;

        let mut data_offset = [0u8; 8];
        file.read_exact(&mut data_offset)?;
        let data_offset = u64::from_le_bytes(data_offset);

        // Read index
        file.seek(SeekFrom::Start(data_offset))?;

        let mut index = Vec::new();
        let mut current_offset = data_offset;

        // Read until footer
        loop {
            let mut time_bytes = [0u8; 8];
            if file.read_exact(&mut time_bytes).is_err() {
                break;
            }
            let time = i64::from_le_bytes(time_bytes);

            let mut offset_bytes = [0u8; 8];
            file.read_exact(&mut offset_bytes)?;
            let offset = u64::from_le_bytes(offset_bytes);

            index.push((time, offset));
            current_offset += 16;
        }

        Ok(Self {
            file,
            entry_count: u32::from_le_bytes(entry_count),
            index,
        })
    }

    pub fn read_range(&mut self, start: Timestamp, end: Timestamp) -> Result<Vec<DataPoint>> {
        let mut results = Vec::new();

        // Binary search for start position
        let idx = match self.index.binary_search_by(|(t, _)| t.cmp(&start)) {
            Ok(i) => i,
            Err(i) => i,
        };

        // Read from index position
        for &(time, offset) in &self.index[idx..] {
            if time >= end {
                break;
            }

            self.file.seek(SeekFrom::Start(offset))?;

            let mut time_bytes = [0u8; 8];
            let mut value_bytes = [0u8; 8];

            self.file.read_exact(&mut time_bytes)?;
            self.file.read_exact(&mut value_bytes)?;

            results.push(DataPoint {
                time: i64::from_le_bytes(time_bytes),
                value: f64::from_le_bytes(value_bytes),
            });
        }

        Ok(results)
    }
}
```

### Write-Ahead Log (WAL)

For durability before SSTable flush:

```rust
// src/wal.rs

use std::fs::{File, OpenOptions};
use std::io::{Write, Read, Seek, SeekFrom};
use crate::types::{Timestamp, Result};

/// Simple Write-Ahead Log
pub struct Wal {
    file: File,
}

impl Wal {
    pub fn open(path: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(path)?;

        Ok(Self { file })
    }

    /// Append entry to WAL
    pub fn append(&mut self, time: Timestamp, value: f64) -> Result<u64> {
        let pos = self.file.seek(SeekFrom::End(0))?;

        // Write: [length: u32][time: i64][value: f64][checksum: u32]
        let mut buf = Vec::new();
        buf.extend_from_slice(&16u32.to_le_bytes());  // Length
        buf.extend_from_slice(&time.to_le_bytes());
        buf.extend_from_slice(&value.to_le_bytes());

        let checksum = crc32fast::hash(&buf[4..]);  // Skip length field
        buf.extend_from_slice(&checksum.to_le_bytes());

        self.file.write_all(&buf)?;
        self.file.sync_all()?;  // Ensure durability

        Ok(pos)
    }

    /// Read all entries
    pub fn read_all(&mut self) -> Result<Vec<(Timestamp, f64)>> {
        self.file.seek(SeekFrom::Start(0))?;

        let mut entries = Vec::new();
        let mut buf = [0u8; 24];  // 4 + 8 + 8 + 4

        while self.file.read_exact(&mut buf).is_ok() {
            let length = u32::from_le_bytes(buf[0..4].try_into().unwrap());
            let time = i64::from_le_bytes(buf[4..12].try_into().unwrap());
            let value = f64::from_le_bytes(buf[12..20].try_into().unwrap());
            let checksum = u32::from_le_bytes(buf[20..24].try_into().unwrap());

            // Verify checksum
            let computed = crc32fast::hash(&buf[4..20]);
            if computed != checksum {
                // Corrupted entry, stop here
                break;
            }

            entries.push((time, value));
        }

        Ok(entries)
    }

    /// Truncate WAL (after successful flush to SSTable)
    pub fn truncate(&mut self) -> Result<()> {
        self.file.set_len(0)?;
        Ok(())
    }
}
```

### Putting It Together

```rust
// src/engine.rs

use crate::storage::InMemoryStorage;
use crate::sstable::{SSTableWriter, SSTableReader};
use crate::wal::Wal;
use crate::types::{Timestamp, Result, DataPoint};

/// Simple storage engine
pub struct Engine {
    /// In-memory buffer
    memtable: InMemoryStorage,
    /// WAL for durability
    wal: Wal,
    /// SSTable reader (if loaded)
    sstable: Option<SSTableReader>,
    /// Path for SSTable
    sstable_path: String,
    /// Flush threshold
    flush_threshold: usize,
}

impl Engine {
    pub fn new(data_path: &str) -> Result<Self> {
        let sstable_path = format!("{}/data.sstable", data_path);
        let wal_path = format!("{}/data.wal", data_path);

        // Create directory if needed
        std::fs::create_dir_all(data_path)?;

        // Open WAL
        let wal = Wal::open(&wal_path)?;

        // Create memtable
        let mut memtable = InMemoryStorage::new(10000);

        // Replay WAL into memtable
        for (time, value) in wal.read_all()? {
            memtable.write(time, value);
        }

        // Load existing SSTable if present
        let sstable = if std::path::Path::new(&sstable_path).exists() {
            Some(SSTableReader::open(&sstable_path)?)
        } else {
            None
        };

        Ok(Self {
            memtable,
            wal,
            sstable,
            sstable_path,
            flush_threshold: 1000,
        })
    }

    pub fn write(&mut self, time: Timestamp, value: f64) -> Result<()> {
        // Write to WAL first
        self.wal.append(time, value)?;

        // Write to memtable
        self.memtable.write(time, value);

        // Check if flush needed
        if self.memtable.len() >= self.flush_threshold {
            self.flush()?;
        }

        Ok(())
    }

    pub fn read_range(&self, start: Timestamp, end: Timestamp) -> Result<Vec<DataPoint>> {
        let mut results = Vec::new();

        // Read from SSTable
        if let Some(sstable) = &self.sstable {
            // Note: Would need mutable reference, simplified here
        }

        // Read from memtable
        for point in self.memtable.read_range(start, end) {
            results.push(point.clone());
        }

        Ok(results)
    }

    fn flush(&mut self) -> Result<()> {
        // Create SSTable from memtable
        let points: Vec<_> = self.memtable.read_range(0, i64::MAX).into_iter().cloned().collect();

        if points.is_empty() {
            return Ok(());
        }

        // Build index
        let mut index = Vec::new();
        let mut current_offset = 16u64;  // Header size

        for point in &points {
            index.push((point.time, current_offset));
            current_offset += 16;  // time (8) + value (8)
        }

        // Write SSTable
        let mut writer = SSTableWriter::create(&self.sstable_path)?;
        for point in &points {
            writer.write(point.time, point.value)?;
        }
        writer.finish(index)?;

        // Reload as reader
        self.sstable = Some(SSTableReader::open(&self.sstable_path)?);

        // Truncate WAL
        self.wal.truncate()?;

        // Clear memtable
        self.memtable = InMemoryStorage::new(10000);

        Ok(())
    }
}
```

---

## Step 4: Compression

### Delta-Delta Compression for Timestamps

```rust
// src/compression.rs

/// Delta-Delta encoder for timestamps
pub struct DeltaDeltaEncoder {
    prev_value: Option<i64>,
    prev_delta: Option<i64>,
}

impl DeltaDeltaEncoder {
    pub fn new() -> Self {
        Self {
            prev_value: None,
            prev_delta: None,
        }
    }

    pub fn encode(&mut self, values: &[i64]) -> Vec<u8> {
        let mut output = Vec::new();

        for &value in values {
            if self.prev_value.is_none() {
                // First value: store as-is
                output.extend_from_slice(&value.to_le_bytes());
                self.prev_value = Some(value);
            } else {
                let prev = self.prev_value.unwrap();
                let delta = value - prev;

                if self.prev_delta.is_none() {
                    // Second value: store delta
                    output.extend_from_slice(&delta.to_le_bytes());
                    self.prev_delta = Some(delta);
                } else {
                    // Subsequent: store delta-of-delta
                    let prev_delta = self.prev_delta.unwrap();
                    let dod = delta - prev_delta;

                    // Zigzag encode for better compression of small values
                    let zigzag = ((dod << 1) ^ (dod >> 63)) as u64;

                    // Simple variable-length encoding
                    encode_varuint(&mut output, zigzag);

                    self.prev_delta = Some(delta);
                }

                self.prev_value = Some(value);
            }
        }

        output
    }
}

/// Delta-Delta decoder
pub struct DeltaDeltaDecoder {
    prev_value: Option<i64>,
    prev_delta: Option<i64>,
}

impl DeltaDeltaDecoder {
    pub fn new() -> Self {
        Self {
            prev_value: None,
            prev_delta: None,
        }
    }

    pub fn decode(&mut self, data: &[u8]) -> Vec<i64> {
        let mut output = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            if self.prev_value.is_none() {
                // First value
                let value = i64::from_le_bytes(data[pos..pos+8].try_into().unwrap());
                output.push(value);
                self.prev_value = Some(value);
                pos += 8;
            } else if self.prev_delta.is_none() {
                // Second value (delta)
                let delta = i64::from_le_bytes(data[pos..pos+8].try_into().unwrap());
                let value = self.prev_value.unwrap() + delta;
                output.push(value);
                self.prev_delta = Some(delta);
                pos += 8;
            } else {
                // Delta-of-delta
                let (zigzag, bytes_read) = decode_varuint(&data[pos..]);
                let dod = (zigzag >> 1) as i64 ^ -(zigzag as i64 & 1);

                let delta = self.prev_delta.unwrap() + dod;
                let value = self.prev_value.unwrap() + delta;
                output.push(value);

                self.prev_delta = Some(delta);
                pos += bytes_read;
            }
        }

        output
    }
}

/// Variable-length unsigned integer encoding
fn encode_varuint(buf: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        buf.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
    buf.push(value as u8);
}

fn decode_varuint(data: &[u8]) -> (u64, usize) {
    let mut result = 0u64;
    let mut shift = 0;

    for (i, &byte) in data.iter().enumerate() {
        result |= ((byte & 0x7F) as u64) << shift;
        if byte < 0x80 {
            return (result, i + 1);
        }
        shift += 7;
    }

    (result, data.len())
}
```

### Gorilla Compression for Floats

```rust
/// Gorilla encoder for floats
pub struct GorillaEncoder {
    prev_value: Option<f64>,
    leading_zeros: u8,
    significant_bits: u8,
}

impl GorillaEncoder {
    pub fn new() -> Self {
        Self {
            prev_value: None,
            leading_zeros: 0,
            significant_bits: 0,
        }
    }

    pub fn encode(&mut self, values: &[f64]) -> Vec<u8> {
        let mut bits = BitWriter::new();

        for &value in values {
            if self.prev_value.is_none() {
                // First value: store full 64 bits
                bits.write_u64(value.to_bits(), 64);
                self.prev_value = Some(value);
            } else {
                let prev = self.prev_value.unwrap();
                let xor = value.to_bits() ^ prev.to_bits();

                if xor == 0 {
                    // Same value: single zero bit
                    bits.write_bit(0);
                } else {
                    bits.write_bit(1);

                    // Count leading and trailing zeros
                    let leading = xor.leading_zeros() as u8;
                    let trailing = xor.trailing_zeros() as u8;
                    let significant = 64 - leading - trailing;

                    if self.leading_zeros == 0 ||
                       leading < self.leading_zeros ||
                       significant > self.significant_bits - (self.leading_zeros - leading)
                    {
                        // New control bits
                        self.leading_zeros = leading;
                        self.significant_bits = significant;

                        bits.write_u8(leading, 5);
                        bits.write_u8(significant, 6);
                    }

                    // Write significant bits
                    let mask = (1u64 << significant) - 1;
                    let value = (xor >> trailing) & mask;
                    bits.write_u64(value, significant as usize);
                }

                self.prev_value = Some(value);
            }
        }

        bits.finish()
    }
}

/// Simple bit writer
struct BitWriter {
    buffer: u64,
    bits_in_buffer: usize,
    output: Vec<u8>,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            buffer: 0,
            bits_in_buffer: 0,
            output: Vec::new(),
        }
    }

    fn write_bit(&mut self, bit: u8) {
        self.buffer |= (bit as u64) << self.bits_in_buffer;
        self.bits_in_buffer += 1;
        self.flush_byte();
    }

    fn write_u8(&mut self, value: u8, bits: usize) {
        self.buffer |= (value as u64) << self.bits_in_buffer;
        self.bits_in_buffer += bits;
        while self.bits_in_buffer >= 8 {
            self.output.push(self.buffer as u8);
            self.buffer >>= 8;
            self.bits_in_buffer -= 8;
        }
    }

    fn write_u64(&mut self, value: u64, bits: usize) {
        for i in 0..bits {
            let bit = ((value >> i) & 1) as u8;
            self.write_bit(bit);
        }
    }

    fn flush_byte(&mut self) {
        while self.bits_in_buffer >= 8 {
            self.output.push(self.buffer as u8);
            self.buffer >>= 8;
            self.bits_in_buffer -= 8;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bits_in_buffer > 0 {
            self.output.push(self.buffer as u8);
        }
        self.output
    }
}
```

### Using Compression in Storage

```rust
// Update the SSTable writer to use compression

use crate::compression::{DeltaDeltaEncoder, GorillaEncoder};

pub struct CompressedSSTableWriter {
    file: File,
    time_encoder: DeltaDeltaEncoder,
    value_encoder: GorillaEncoder,
}

impl CompressedSSTableWriter {
    pub fn create(path: &str) -> Result<Self> {
        // Similar to SSTableWriter but with encoders
        Ok(Self {
            file: File::create(path)?,
            time_encoder: DeltaDeltaEncoder::new(),
            value_encoder: GorillaEncoder::new(),
        })
    }

    pub fn write_batch(&mut self, points: &[DataPoint]) -> Result<()> {
        let times: Vec<_> = points.iter().map(|p| p.time).collect();
        let values: Vec<_> = points.iter().map(|p| p.value).collect();

        let compressed_times = self.time_encoder.encode(&times);
        let compressed_values = self.value_encoder.encode(&values);

        // Write compressed data
        self.file.write_all(&(compressed_times.len() as u32).to_le_bytes())?;
        self.file.write_all(&compressed_times)?;

        self.file.write_all(&(compressed_values.len() as u32).to_le_bytes())?;
        self.file.write_all(&compressed_values)?;

        Ok(())
    }
}
```

---

## Step 5: Query Engine

### Basic Query Parser

```rust
// src/query.rs

use crate::types::{Timestamp, DataPoint, Result};
use crate::engine::Engine;

/// Simple query language
#[derive(Debug)]
pub enum Query {
    /// SELECT * WHERE time >= start AND time < end
    Range { start: Timestamp, end: Timestamp },
    /// SELECT * ORDER BY time DESC LIMIT n
    Latest { n: usize },
    /// SELECT avg(value) WHERE time >= start AND time < end
    Average { start: Timestamp, end: Timestamp },
    /// SELECT max(value) WHERE ...
    Max { start: Timestamp, end: Timestamp },
    /// SELECT min(value) WHERE ...
    Min { start: Timestamp, end: Timestamp },
}

/// Parse simple SQL-like queries
pub fn parse_query(query: &str) -> Result<Query> {
    let query = query.to_lowercase();

    if query.starts_with("select *") {
        if let Some(start) = extract_time(&query, "time >=") {
            let end = extract_time(&query, "time <").unwrap_or(i64::MAX);
            Ok(Query::Range { start, end })
        } else if let Some(n) = extract_limit(&query) {
            Ok(Query::Latest { n })
        } else {
            Err(crate::types::Error::InvalidData("Invalid query".into()))
        }
    } else if query.starts_with("select avg") {
        let start = extract_time(&query, "time >=").unwrap_or(0);
        let end = extract_time(&query, "time <").unwrap_or(i64::MAX);
        Ok(Query::Average { start, end })
    } else {
        Err(crate::types::Error::InvalidData("Unsupported query".into()))
    }
}

fn extract_time(query: &str, pattern: &str) -> Option<Timestamp> {
    let idx = query.find(pattern)?;
    let rest = &query[idx + pattern.len()..];
    let value_str = rest.split_whitespace().next()?;
    value_str.parse().ok()
}

fn extract_limit(query: &str) -> Option<usize> {
    let idx = query.find("limit")?;
    let rest = &query[idx + 5..];
    let value_str = rest.split_whitespace().next()?;
    value_str.parse().ok()
}

/// Execute query against engine
pub fn execute_query(engine: &Engine, query: Query) -> Result<QueryResult> {
    match query {
        Query::Range { start, end } => {
            let points = engine.read_range(start, end)?;
            Ok(QueryResult::Points(points))
        }
        Query::Latest { n } => {
            // Implementation would need latest method
            Ok(QueryResult::Points(vec![]))
        }
        Query::Average { start, end } => {
            let points = engine.read_range(start, end)?;
            let avg = points.iter().map(|p| p.value).sum::<f64>() / points.len() as f64;
            Ok(QueryResult::Value(avg))
        }
        Query::Max { start, end } => {
            let points = engine.read_range(start, end)?;
            let max = points.iter().map(|p| p.value).fold(f64::NEG_INFINITY, f64::max);
            Ok(QueryResult::Value(max))
        }
        Query::Min { start, end } => {
            let points = engine.read_range(start, end)?;
            let min = points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
            Ok(QueryResult::Value(min))
        }
    }
}

#[derive(Debug)]
pub enum QueryResult {
    Points(Vec<DataPoint>),
    Value(f64),
}
```

---

## Step 6: Time Partitioning

### Chunk Manager

```rust
// src/chunk.rs

use std::collections::BTreeMap;
use crate::engine::Engine;
use crate::types::{Timestamp, Result};

/// Manage time-partitioned chunks
pub struct ChunkManager {
    /// Chunk duration in microseconds
    chunk_duration: Timestamp,
    /// Active chunks (time_bucket -> engine)
    chunks: BTreeMap<Timestamp, Engine>,
    /// Base path for chunk data
    data_path: String,
}

impl ChunkManager {
    pub fn new(data_path: &str, chunk_duration: Timestamp) -> Result<Self> {
        std::fs::create_dir_all(data_path)?;

        Ok(Self {
            chunk_duration,
            chunks: BTreeMap::new(),
            data_path: data_path.to_string(),
        })
    }

    /// Get chunk for a timestamp
    fn get_chunk_bucket(time: Timestamp) -> Timestamp {
        (time / 86400_000_000) * 86400_000_000  // Daily chunks
    }

    /// Get or create chunk for time
    pub fn get_or_create_chunk(&mut self, time: Timestamp) -> Result<&mut Engine> {
        let bucket = Self::get_chunk_bucket(time);
        let chunk_path = format!("{}/chunk_{}", self.data_path, bucket);

        if !self.chunks.contains_key(&bucket) {
            let engine = Engine::new(&chunk_path)?;
            self.chunks.insert(bucket, engine);
        }

        Ok(self.chunks.get_mut(&bucket).unwrap())
    }

    /// Write to appropriate chunk
    pub fn write(&mut self, time: Timestamp, value: f64) -> Result<()> {
        let chunk = self.get_or_create_chunk(time)?;
        chunk.write(time, value)
    }

    /// Read across all relevant chunks
    pub fn read_range(&self, start: Timestamp, end: Timestamp) -> Result<Vec<DataPoint>> {
        let start_bucket = Self::get_chunk_bucket(start);
        let end_bucket = Self::get_chunk_bucket(end);

        let mut results = Vec::new();

        for (&bucket, engine) in &self.chunks {
            if bucket >= start_bucket && bucket <= end_bucket {
                results.extend(engine.read_range(start, end)?);
            }
        }

        // Sort by time
        results.sort_by_key(|p| p.time);

        Ok(results)
    }
}
```

---

## Common Pitfalls

### Pitfall 1: Not Handling Out-of-Order Data

```rust
// BAD: Assumes data arrives in order
fn write_bad(&mut self, time: Timestamp, value: f64) {
    self.points.push(DataPoint { time, value });
}

// GOOD: Handles out-of-order
fn write_good(&mut self, time: Timestamp, value: f64) {
    match self.points.binary_search_by(|p| p.time.cmp(&time)) {
        Ok(idx) => self.points[idx] = DataPoint { time, value },
        Err(idx) => self.points.insert(idx, DataPoint { time, value }),
    }
}
```

### Pitfall 2: Unbounded Memory Growth

```rust
// BAD: No memory limit
fn write_unbounded(&mut self, time: Timestamp, value: f64) {
    self.points.push(DataPoint { time, value });
    // Will eventually OOM
}

// GOOD: Enforce limit
fn write_bounded(&mut self, time: Timestamp, value: f64) {
    // ... insert logic ...

    // Remove old data
    while self.points.len() > self.max_points {
        self.points.remove(0);
    }
}
```

### Pitfall 3: No Checksums for Data Integrity

```rust
// BAD: No integrity check
file.write_all(&data)?;

// GOOD: With checksum
use crc32fast::hash;

let checksum = hash(&data);
file.write_all(&data)?;
file.write_all(&checksum.to_le_bytes())?;
```

### Pitfall 4: Syncing Too Frequently

```rust
// BAD: Sync after every write (slow)
fn write_slow(&mut self, time: Timestamp, value: f64) {
    self.wal.append(time, value)?;
    self.wal.file.sync_all()?;  // Very slow!
}

// GOOD: Batch syncs
fn write_fast(&mut self, time: Timestamp, value: f64) {
    self.wal.append(time, value)?;
    self.write_count += 1;

    // Sync every 100 writes
    if self.write_count % 100 == 0 {
        self.wal.file.sync_all()?;
    }
}
```

### Pitfall 5: Ignoring Compression Trade-offs

```rust
// Consider compression level vs CPU
match use_case {
    "high_write_volume" => Compression::Lz4,  // Fast
    "storage_optimized" => Compression::Zstd, // Balanced
    "cold_storage" => Compression::Gzip,      // Best ratio
}
```

---

## Next Steps

### What You've Built

1. ✅ In-memory storage with sorted insertion
2. ✅ File persistence with SSTable format
3. ✅ WAL for durability
4. ✅ Compression (Delta-Delta, Gorilla)
5. ✅ Basic query engine
6. ✅ Time partitioning

### Where to Go From Here

1. **Add Indexing**: Skip lists, B-trees for faster lookups
2. **Implement LSM Tree**: Multiple SSTable levels with compaction
3. **Add SQL Support**: Use sqlparser-rs for full SQL
4. **Network Protocol**: Implement PostgreSQL wire protocol
5. **Replication**: Add leader-follower replication

### Recommended Reading

1. "Designing Data-Intensive Applications" by Martin Kleppmann
2. Google's Bigtable paper
3. Facebook's Gorilla paper
4. Microsoft's DiskANN paper

### Reference Projects

Study these for inspiration:
- [slatedb](https://github.com/slatedb/slatedb) - Rust LSM tree
- [questdb](https://github.com/questdb/questdb) - Time-series database
- [TimescaleDB](https://github.com/timescale/timescaledb) - PostgreSQL extension

---

## Related Documentation

- [Rust Implementation](./rust-revision.md)
- [Production Guide](./production-grade.md)
- [Analytics Functions](./analytics-functions.md)
