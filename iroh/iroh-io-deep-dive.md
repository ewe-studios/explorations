# iroh-io Deep Dive

## Overview

`iroh-io` provides async I/O traits and implementations for working with different resource types. It defines abstractions for async slice-based reading and writing that work with files, memory buffers, and HTTP resources.

**Version:** 0.6.1
**Repository:** https://github.com/n0-computer/iroh-io
**License:** MIT/Apache-2.0

---

## Architecture and Design Decisions

### Core Design Philosophy

The crate is built around several key insights:

1. **Position-Explicit I/O**: Rather than maintaining internal position state, all operations explicitly specify offsets. This matches the interface of systems like SQLite.

2. **Non-Thread-Safe by Default**: Futures are not required to be `Send`, allowing implementations to use non-thread-safe types like `Rc<RefCell<T>>` instead of `Arc<Mutex<T>>`.

3. **Linear Access Model**: Methods take `&mut self` to enforce linear access, preventing concurrent modification issues.

4. **Readers Are Cheap**: The design assumes readers are cheap to create, so if an error occurs or concurrent access is needed, create a new reader.

### SQLite Inspiration

The trait design is explicitly inspired by SQLite's I/O interface:

```rust
// Similar to SQLite's xRead, xFileSize
// See: https://www.sqlite.org/c3ref/io_methods.html
pub trait AsyncSliceReader {
    fn read_at(&mut self, offset: u64, len: usize) -> impl Future<Output = io::Result<Bytes>>;
    fn size(&mut self) -> impl Future<Output = io::Result<u64>>;
}
```

### Executor Considerations

The design assumes a **local async executor**:

```rust
// Use LocalPoolHandle for non-Send futures
use tokio_util::task::LocalPoolHandle;

let pool = LocalPoolHandle::new(2);
pool.spawn_pinned(|| async {
    // Can use Rc, RefCell here
});
```

This enables:
- Using `Rc` instead of `Arc` for better performance
- Using `RefCell` instead of `Mutex` for interior mutability
- Avoiding thread synchronization overhead

---

## Key APIs and Data Structures

### AsyncSliceReader Trait

```rust
/// Trait for async reading from resources
pub trait AsyncSliceReader {
    /// Read at most `len` bytes at offset
    /// Returns fewer bytes if resource is smaller
    /// Never returns error for range issues
    fn read_at(
        &mut self,
        offset: u64,
        len: usize
    ) -> impl Future<Output = io::Result<Bytes>>;

    /// Read exactly `len` bytes, error if fewer available
    fn read_exact_at(
        &mut self,
        offset: u64,
        len: usize
    ) -> impl Future<Output = io::Result<Bytes>> {
        async move {
            let res = self.read_at(offset, len).await?;
            if res.len() < len {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            Ok(res)
        }
    }

    /// Get total length of resource
    fn size(&mut self) -> impl Future<Output = io::Result<u64>>;
}
```

### AsyncSliceWriter Trait

```rust
/// Trait for async writing to resources
pub trait AsyncSliceWriter: Sized {
    /// Write slice at offset, extending resource if needed
    fn write_at(
        &mut self,
        offset: u64,
        data: &[u8]
    ) -> impl Future<Output = io::Result<()>>;

    /// Write Bytes at offset (avoids allocation)
    fn write_bytes_at(
        &mut self,
        offset: u64,
        data: Bytes
    ) -> impl Future<Output = io::Result<()>>;

    /// Set resource length (truncate or extend)
    fn set_len(&mut self, len: u64) -> impl Future<Output = io::Result<()>>;

    /// Sync buffers to underlying storage
    fn sync(&mut self) -> impl Future<Output = io::Result<()>>;
}
```

### AsyncStreamReader Trait

```rust
/// Non-seekable reader (e.g., network socket)
pub trait AsyncStreamReader {
    /// Read at most `len` bytes
    fn read_bytes(&mut self, len: usize) -> impl Future<Output = io::Result<Bytes>>;

    /// Read exactly L bytes
    fn read<const L: usize>(&mut self) -> impl Future<Output = io::Result<[u8; L]>>;

    /// Read exactly `len` bytes
    fn read_bytes_exact(&mut self, len: usize) -> impl Future<Output = io::Result<Bytes>>;

    /// Read exactly L bytes
    fn read_exact<const L: usize>(&mut self) -> impl Future<Output = io::Result<[u8; L]>>;
}
```

### AsyncStreamWriter Trait

```rust
/// Non-seekable writer (e.g., network socket)
pub trait AsyncStreamWriter {
    /// Write slice
    fn write(&mut self, data: &[u8]) -> impl Future<Output = io::Result<()>>;

    /// Write Bytes
    fn write_bytes(&mut self, data: Bytes) -> impl Future<Output = io::Result<()>>;

    /// Sync buffers
    fn sync(&mut self) -> impl Future<Output = io::Result<()>>;
}
```

### Implementations

```rust
// Built-in implementations
impl AsyncSliceReader for Bytes { }
impl AsyncSliceReader for BytesMut { }
impl AsyncSliceReader for &[u8] { }
impl AsyncSliceReader for Cursor<T> where T: AsyncSliceReader { }

impl AsyncSliceWriter for Vec<u8> { }
impl AsyncSliceWriter for BytesMut { }

impl AsyncStreamReader for Bytes { }
impl AsyncStreamReader for BytesMut { }
impl AsyncStreamReader for &[u8] { }

impl AsyncStreamWriter for Vec<u8> { }
impl AsyncStreamWriter for BytesMut { }

// Feature-gated implementations
#[cfg(feature = "tokio-io")]
impl AsyncSliceReader for File { }

#[cfg(feature = "tokio-io")]
impl AsyncSliceWriter for File { }

#[cfg(feature = "x-http")]
impl AsyncSliceReader for HttpAdapter { }
```

---

## Protocol Details

### HTTP Adapter

The HTTP adapter implements `AsyncSliceReader` for HTTP resources with Range request support:

```rust
#[cfg(feature = "x-http")]
pub struct HttpAdapter {
    url: Url,
    client: reqwest::Client,
    size: Option<u64>,
}

#[cfg(feature = "x-http")]
impl AsyncSliceReader for HttpAdapter {
    async fn read_at(&mut self, offset: u64, len: usize) -> io::Result<Bytes> {
        // Send GET with Range header
        let response = self.client.get(self.url.clone())
            .header("Range", format!("bytes={}-{}", offset, offset + len as u64 - 1))
            .send()
            .await
            .map_err(make_io_error)?;

        response.bytes().await.map_err(make_io_error)
    }

    async fn size(&mut self) -> io::Result<u64> {
        // HEAD request for Content-Length
        let response = self.client.head(self.url.clone())
            .send()
            .await
            .map_err(make_io_error)?;

        let len = response.headers()
            .get("Content-Length")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No Content-Length"))?;

        len.to_str()
            .map_err(make_io_error)?
            .parse()
            .map_err(make_io_error)
    }
}
```

### Cursor Adapter

```rust
/// Wraps AsyncSliceReader with position tracking
impl<T: AsyncSliceReader> AsyncStreamReader for Cursor<T> {
    async fn read_bytes(&mut self, len: usize) -> io::Result<Bytes> {
        let offset = self.position();
        let res = self.get_mut().read_at(offset, len).await?;
        self.set_position(offset + res.len() as u64);
        Ok(res)
    }

    async fn read<const L: usize>(&mut self) -> io::Result<[u8; L]> {
        let offset = self.position();
        let res = self.get_mut().read_at(offset, L).await?;
        if res.len() < L {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        self.set_position(offset + res.len() as u64);
        let mut buf = [0u8; L];
        buf.copy_from_slice(&res);
        Ok(buf)
    }
}
```

---

## Integration with Main Iroh Endpoint

### Blob Store Integration

The blob store uses `iroh-io` traits extensively:

```rust
// Import bytes using AsyncSliceReader
async fn import_bytes<R: AsyncSliceReader>(
    store: &Store,
    mut reader: R,
) -> Result<TempTag> {
    let size = reader.size().await?;
    let mut offset = 0;
    let mut hasher = blake3::Hasher::new();

    while offset < size {
        let chunk = reader.read_at(offset, 1024 * 1024).await?;
        hasher.update(&chunk);
        offset += chunk.len() as u64;
    }

    let hash = hasher.finalize();
    // ... store blob
}

// Export using AsyncSliceWriter
async fn export_blob<W: AsyncSliceWriter>(
    store: &Store,
    hash: Hash,
    mut writer: W,
) -> Result<()> {
    let blob = store.get_blob(hash).await?;
    writer.write_at(0, &blob).await?;
    writer.sync().await?;
    Ok(())
}
```

### CAR Reader Integration

```rust
// CAR reader uses AsyncSliceReader
impl<R: AsyncRead + Unpin> CarReader<R> {
    pub async fn new(mut reader: R) -> Result<Self> {
        let mut buffer = Vec::new();

        // Read length-prefixed header
        match ld_read(&mut reader, &mut buffer).await? {
            Some(buf) => {
                let header = CarHeader::decode(buf)?;
                Ok(CarReader { reader, header, buffer })
            }
            None => Err(Error::Parsing("No header")),
        }
    }
}
```

---

## Production Usage Patterns

### File Operations

```rust
use iroh_io::{AsyncSliceReader, AsyncSliceWriter, File};
use tokio::fs;

// Read file at offset
async fn read_file_section(path: &str) -> Result<Bytes> {
    let file = fs::File::open(path).await?;
    let mut reader = File::from_std(file.into_std().await);

    // Read 1KB at offset 1MB
    let data = reader.read_at(1024 * 1024, 1024).await?;
    Ok(data)
}

// Write file with holes
async fn write_sparse_file(path: &str) -> Result<()> {
    let file = fs::File::create(path).await?;
    let mut writer = File::from_std(file.into_std().await);

    // Write at offset, creating hole at beginning
    writer.write_at(1024 * 1024, b"Hello at 1MB").await?;

    // Write more data
    writer.write_at(2048 * 1024, b"Hello at 2MB").await?;

    writer.sync().await?;
    Ok(())
}
```

### HTTP Resource Reading

```rust
use iroh_io::{AsyncSliceReader, HttpAdapter};
use url::Url;

// Read partial HTTP resource
async fn read_http_range(url: &str) -> Result<Bytes> {
    let url = Url::parse(url)?;
    let mut adapter = HttpAdapter::new(url).await?;

    // Get size first
    let size = adapter.size().await?;
    println!("Resource size: {}", size);

    // Read last 1KB
    let offset = size.saturating_sub(1024);
    let data = adapter.read_at(offset, 1024).await?;

    Ok(data)
}

// Stream HTTP resource
async fn stream_http(url: &str) -> Result<()> {
    let url = Url::parse(url)?;
    let mut adapter = HttpAdapter::new(url).await?;

    let size = adapter.size().await?;
    let mut offset = 0;

    while offset < size {
        let chunk = adapter.read_at(offset, 64 * 1024).await?;
        if chunk.is_empty() { break; }

        process_chunk(&chunk).await?;
        offset += chunk.len() as u64;
    }

    Ok(())
}
```

### Memory Operations

```rust
use iroh_io::{AsyncSliceReader, AsyncSliceWriter};
use bytes::{Bytes, BytesMut};

// Read from Bytes
async fn read_bytes() -> Result<()> {
    let data: Bytes = (0..100u8).collect::<Vec<_>>().into();
    let mut reader = data;

    let chunk = reader.read_at(0, 10).await?;
    assert_eq!(chunk, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let size = reader.size().await?;
    assert_eq!(size, 100);

    Ok(())
}

// Write to BytesMut
async fn write_bytes() -> Result<()> {
    let mut writer = BytesMut::new();

    writer.write_at(0, b"Hello").await?;
    writer.write_at(10, b"World").await?;

    assert_eq!(&writer[..5], b"Hello");
    assert_eq!(&writer[10..15], b"World");
    // Gap is zero-filled
    assert_eq!(&writer[5..10], &[0; 5]);

    Ok(())
}
```

### Cursor Operations

```rust
use iroh_io::{AsyncStreamReader, AsyncSliceReader};
use std::io::Cursor;

// Sequential reading with Cursor
async fn sequential_read() -> Result<()> {
    let data: Bytes = (0..100u8).collect::<Vec<_>>().into();
    let mut cursor = Cursor::new(data);

    // Read sequentially
    let chunk1 = cursor.read_bytes(10).await?;
    let chunk2 = cursor.read_bytes(10).await?;

    assert_eq!(chunk1, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(chunk2, vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);

    Ok(())
}
```

### Stats Wrapper

```rust
#[cfg(feature = "stats")]
use iroh_io::stats::StatsReader;

// Wrap reader with statistics
async fn track_reads() -> Result<()> {
    let data: Bytes = vec![0; 1000].into();
    let mut reader = StatsReader::new(data);

    reader.read_at(0, 100).await?;
    reader.read_at(500, 200).await?;

    let stats = reader.stats();
    println!("Bytes read: {}", stats.bytes_read);
    println!("Read operations: {}", stats.read_count);

    Ok(())
}
```

---

## Rust Revision Notes

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| bytes | 1.x | Byte buffer types |
| tokio | 1.x | Async runtime |
| tokio-util | 0.7.x | Async utilities |
| reqwest | 0.11.x | HTTP client (x-http feature) |
| proptest | 1.x | Property testing |

### Notable Rust Patterns

1. **Extension Traits**: `AsyncSliceReaderExt` for additional methods
2. **Generic Implementations**: Works with any type implementing the traits
3. **Const Generics**: `read<const L: usize>()` for fixed-size reads
4. **Feature Flags**: Optional dependencies for tokio-io and HTTP support

### Future Considerations

```rust
/// Futures are not required to be Send
/// This allows using Rc/RefCell instead of Arc/Mutex
fn read_at(
    &mut self,
    offset: u64,
    len: usize
) -> impl Future<Output = io::Result<Bytes>>;
// Note: No + Send bound
```

### Performance Optimizations

1. **Zero-Copy**: `Bytes` type avoids allocations
2. **Direct Writes**: `write_bytes_at` avoids copy from slice
3. **Batched Operations**: Multiple writes can be combined before sync

### Potential Enhancements

1. **Buffered Reader**: Add buffered read wrapper
2. **Async Seek**: Optional seek trait for random access
3. **Timeout Support**: Built-in timeout wrappers
4. **Retry Logic**: Automatic retry for transient failures

---

## Summary

`iroh-io` provides:

- **Unified I/O Traits**: Single interface for files, memory, and HTTP
- **Position-Explicit API**: Clear offset-based operations
- **Local Executor Optimized**: No unnecessary Send bounds
- **Cheap Readers**: Design encourages creating new readers
- **Feature-Gated**: Optional tokio and HTTP support

The crate enables flexible, efficient async I/O across multiple resource types while maintaining a clean, consistent API.
