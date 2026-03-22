# iroh-car Deep Dive

## Overview

`iroh-car` provides implementation of the CAR (Content Addressable aRchive) format, a standard container format for IPLD (InterPlanetary Linked Data) blocks. CAR files are commonly used in the IPFS ecosystem for importing and exporting content-addressed data.

**Repository:** https://github.com/n0-computer/iroh-car
**License:** MIT/Apache-2.0

---

## Architecture and Design Decisions

### CAR Format Overview

The CAR format is defined in the [IPLD CAR specification](https://ipld.io/specs/transport/car/). Key characteristics:

1. **Sequential Access**: CAR files are designed for sequential reading and writing
2. **Self-Describing**: Header contains version and root CIDs
3. **Length-Prefixed Blocks**: Each block is prefixed with its length for efficient streaming
4. **Content-Addressed**: Blocks are identified by CIDs (Content Identifiers)

### Design Decisions

1. **Async-First**: All operations are async, supporting both file and network streams
2. **Streaming Support**: Both reading and writing support streaming operations
3. **Multiple Versions**: Support for CARv1 with extensibility for future versions
4. **Zero-Copy Where Possible**: Efficient buffer management minimizes allocations

### Use Cases in iroh

CAR files serve several purposes in the iroh ecosystem:

1. **Data Import/Export**: Portable format for moving collections of blobs
2. **Backup Format**: Compact representation of document state
3. **Interoperability**: Compatibility with IPFS tooling and ecosystems
4. **Snapshot Distribution**: Efficient distribution of known-good data states

---

## Key APIs and Data Structures

### CarHeader

```rust
/// CAR file header
pub enum CarHeader {
    V1(CarHeaderV1),
}

/// CARv1 header structure
pub struct CarHeaderV1 {
    /// Root CIDs for the archive
    pub roots: Vec<Cid>,
    /// Optional version (defaults to 1)
    pub version: u64,
}

impl CarHeader {
    /// Encode header to bytes
    pub fn encode(&self) -> Result<Vec<u8>>;

    /// Decode header from bytes
    pub fn decode(buf: &[u8]) -> Result<Self>;
}
```

### CarReader

```rust
/// Reader for CAR files
pub struct CarReader<R> {
    reader: R,
    header: CarHeader,
    buffer: Vec<u8>,
}

impl<R: AsyncRead + Unpin> CarReader<R> {
    /// Create new reader and parse header
    pub async fn new(reader: R) -> Result<Self>;

    /// Get the header
    pub fn header(&self) -> &CarHeader;

    /// Read next block
    pub async fn next_block(&mut self) -> Result<Option<(Cid, Vec<u8>)>>;

    /// Convert to stream
    pub fn stream(self) -> impl Stream<Item = Result<(Cid, Vec<u8>)>>;
}
```

### CarWriter

```rust
/// Writer for CAR files
pub struct CarWriter<W> {
    writer: W,
    header_written: bool,
}

impl<W: AsyncWrite + Unpin> CarWriter<W> {
    /// Create new writer with header
    pub fn new(header: CarHeader, writer: W) -> Self;

    /// Write a block
    pub async fn write(&mut self, cid: Cid, data: &[u8]) -> Result<()>;

    /// Finish writing (flush and finalize)
    pub async fn finish(&mut self) -> Result<()>;
}
```

### Error Types

```rust
/// CAR operation errors
pub enum Error {
    /// Parsing error
    Parsing(String),

    /// IO error
    Io(#[from] std::io::Error),

    /// Invalid CID
    InvalidCid(String),

    /// Unexpected EOF
    UnexpectedEof,
}
```

---

## Protocol Details

### Wire Format

CAR files use a simple binary format:

```
┌──────────────────────────────────────┐
│         Header Length (varint)       │
├──────────────────────────────────────┤
│         CBOR-Encoded Header          │
├──────────────────────────────────────┤
│         Block 1 Length (varint)      │
├──────────────────────────────────────┤
│         Block 1 CID + Data           │
├──────────────────────────────────────┤
│         Block 2 Length (varint)      │
├──────────────────────────────────────┤
│         Block 2 CID + Data           │
├──────────────────────────────────────┤
│                ...                   │
└──────────────────────────────────────┘
```

### CID Encoding

Blocks use the standard CID format:

```rust
/// Block encoding: [CID length][CID][Data]
/// Where CID is: [version][codec][multihash]
```

### Length Prefixing

All elements use unsigned varint length prefixing:

```rust
/// Read length-prefixed data
async fn ld_read<R: AsyncRead>(
    reader: &mut R,
    buffer: &mut Vec<u8>
) -> Result<Option<Vec<u8>>> {
    // Read varint length
    let len = read_varint(reader).await?;

    // Read exactly len bytes
    buffer.resize(len, 0);
    reader.read_exact(buffer).await?;

    Ok(Some(buffer.clone()))
}
```

---

## Integration with Main Iroh Endpoint

### Blob Import/Export

```rust
// Export blobs to CAR
async fn export_to_car(
    store: &iroh_blobs::store::Store,
    hashes: Vec<Hash>,
    output: File,
) -> Result<()> {
    let header = CarHeader::V1(CarHeaderV1 {
        roots: hashes.iter().map(|h| h.to_cid()).collect(),
        version: 1,
    });

    let mut writer = CarWriter::new(header, output);

    for hash in hashes {
        let blob = store.get_blob(hash).await?;
        let cid = hash.to_cid();
        writer.write(cid, &blob).await?;
    }

    writer.finish().await?;
    Ok(())
}

// Import from CAR
async fn import_from_car(
    store: &iroh_blobs::store::Store,
    input: File,
) -> Result<Vec<Hash>> {
    let mut reader = CarReader::new(input).await?;
    let mut hashes = Vec::new();

    while let Some((cid, data)) = reader.next_block().await? {
        let hash = Hash::from_cid(&cid)?;
        store.put_blob(hash, &data).await?;
        hashes.push(hash);
    }

    Ok(hashes)
}
```

### Document Export

```rust
impl DocHandle {
    /// Export document to CAR file
    pub async fn export_car<W: AsyncWrite + Unpin>(
        &self,
        writer: W,
    ) -> Result<()> {
        let entries = self.get_many("").await?;

        // Collect root CIDs
        let mut roots = Vec::new();
        let mut blocks = Vec::new();

        for entry in entries {
            let hash = entry.record().content_hash();
            let cid = hash.to_cid();
            roots.push(cid.clone());

            // Get content from blob store
            if let Some(content) = self.get_content(entry.id().key()).await? {
                blocks.push((cid, content.to_vec()));
            }
        }

        // Write CAR file
        let header = CarHeader::V1(CarHeaderV1 {
            roots,
            version: 1,
        });

        let mut car_writer = CarWriter::new(header, writer);
        for (cid, data) in blocks {
            car_writer.write(cid, &data).await?;
        }
        car_writer.finish().await?;

        Ok(())
    }
}
```

---

## Production Usage Patterns

### Basic CAR Operations

```rust
use iroh_car::{CarHeader, CarHeaderV1, CarReader, CarWriter};
use tokio::fs::File;
use tokio::io::BufReader;

// Create CAR file
async fn create_car() -> Result<()> {
    let file = File::create("archive.car").await?;

    let roots = vec![/* root CIDs */];
    let header = CarHeader::V1(CarHeaderV1::from(roots));

    let mut writer = CarWriter::new(header, file);
    writer.write(cid1, &data1).await?;
    writer.write(cid2, &data2).await?;
    writer.finish().await?;

    Ok(())
}

// Read CAR file
async fn read_car() -> Result<()> {
    let file = File::open("archive.car").await?;
    let reader = BufReader::new(file);

    let mut car_reader = CarReader::new(reader).await?;

    println!("Roots: {:?}", car_reader.header());

    while let Some((cid, data)) = car_reader.next_block().await? {
        println!("Block: {} ({} bytes)", cid, data.len());
    }

    Ok(())
}
```

### Streaming CAR Processing

```rust
// Stream CAR blocks through processing pipeline
async fn process_car_stream(path: &str) -> Result<()> {
    let file = File::open(path).await?;
    let reader = CarReader::new(file).await?;

    let mut stream = reader.stream();

    while let Some(result) = stream.next().await {
        let (cid, data) = result?;

        // Process block
        process_block(&cid, &data).await?;
    }

    Ok(())
}
```

### CAR Validation

```rust
// Validate CAR file integrity
async fn validate_car(path: &str) -> Result<()> {
    let file = File::open(path).await?;
    let mut reader = CarReader::new(file).await?;

    while let Some((expected_cid, data)) = reader.next_block().await? {
        // Verify CID matches content
        let actual_hash = blake3::hash(&data);
        let actual_cid = Cid::new_v1(0x71, actual_hash.into());

        if expected_cid != actual_cid {
            return Err(Error::InvalidCid(format!(
                "CID mismatch for block"
            )));
        }
    }

    Ok(())
}
```

### Batch Import

```rust
// Import multiple CAR files
async fn import_cars(
    store: &Store,
    paths: Vec<&str>,
) -> Result<Vec<Hash>> {
    let mut all_hashes = Vec::new();

    for path in paths {
        let file = File::open(path).await?;
        let mut reader = CarReader::new(file).await?;

        while let Some((cid, data)) = reader.next_block().await? {
            let hash = Hash::from_cid(&cid)?;
            store.put_blob(hash, &data).await?;
            all_hashes.push(hash);
        }
    }

    Ok(all_hashes)
}
```

---

## Rust Revision Notes

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| cid | 0.10.x | Content Identifier handling |
| futures | 0.3.x | Async utilities |
| tokio | 1.x | Async runtime |
| unsigned-varint | 0.8.x | Varint encoding |
| ciborium | 0.2.x | CBOR serialization |
| multihash-codetable | 0.1.x | Multihash implementations |

### Notable Rust Patterns

1. **Async Reader/Writer Pattern**: Generic over `AsyncRead`/`AsyncWrite`
2. **Buffer Reuse**: Single buffer for all read operations
3. **Stream Conversion**: `stream()` method for ergonomic async iteration
4. **Error Propagation**: Custom error type with `From` implementations

### Performance Considerations

1. **Buffer Management**: Reusable buffer reduces allocations
2. **Streaming Design**: No requirement to load entire file in memory
3. **Async I/O**: Non-blocking operations for network streams

### Potential Enhancements

1. **CARv2 Support**: Add support for CARv2 format
2. **Parallel Writing**: Concurrent block writing for large imports
3. **Index Support**: Optional index generation for random access
4. **Compression**: Optional block compression

---

## Summary

`iroh-car` provides essential CAR format support for:

- **Data Portability**: Standard format for iroh data export
- **IPFS Interoperability**: Compatible with IPFS tooling
- **Efficient Streaming**: Async-first design for large files
- **Simple API**: Easy integration with iroh workflows

The module enables seamless data exchange between iroh and the broader content-addressed ecosystem.
