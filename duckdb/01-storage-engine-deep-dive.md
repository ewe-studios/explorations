---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs/src.duckdb/duckdb/src/storage/
explored_at: 2026-03-29
focus: Storage engine architecture, buffer management, compression
---

# DuckDB Storage Engine Deep Dive

## Overview

DuckDB's storage engine is designed for analytical workloads with the following goals:

1. **Columnar storage** - Data organized by column for efficient scans
2. **Compression** - Multiple algorithms optimized per data type
3. **Buffer management** - Efficient memory usage with eviction
4. **Checkpointing** - Periodic persistence to disk
5. **MVCC** - Multi-version concurrency for transactions

## File Structure

### Single-File Database

DuckDB stores the entire database in a single file with this structure:

```
┌─────────────────────────────────────┐
│  Header (4KB)                       │
│  - Magic bytes                      │
│  - Version info                     │
│  - Pointer to first block           │
├─────────────────────────────────────┤
│  Block 1 (4KB)                      │
│  - Schema/catalog data              │
├─────────────────────────────────────┤
│  Block 2 (4KB)                      │
│  - Table data                       │
├─────────────────────────────────────┤
│  ...                                │
├─────────────────────────────────────┤
│  Pointer Block (4KB)                │
│  - Block pointers for checkpoint    │
└─────────────────────────────────────┘
```

### Block Manager

All I/O goes through the **BlockManager**:

```cpp
class BlockManager {
    BufferManager buffer_manager;
    BlockId header_block;
    bool read_only;

    // Read a block
    BlockHandle* Read(BlockId block_id);

    // Write a block
    void Write(BlockHandle* handle, data_ptr_t data);

    // Allocate new block
    BlockId Allocate();
};
```

## Buffer Manager

### Purpose

The BufferManager manages a pool of memory blocks:

1. Caches frequently accessed blocks
2. Evicts least-recently-used blocks under pressure
3. Tracks memory usage across categories

### Data Structures

```cpp
struct BufferManager {
    vector<unique_ptr<BlockHandle>> blocks;  // All managed blocks
    size_t max_memory;                        // Configured limit
    atomic<size_t> used_memory;              // Current usage
    mutex lock;                               // Protects state

    // Clock-sweep eviction
    idx_t clock_hand;                         // Current position
};

struct BlockHandle {
    BlockId block_id;                         // File position
    data_ptr_t data;                          // In-memory data
    bool dirty;                               // Needs write-back
    bool pinned;                              // Cannot evict
    size_t reference_count;                   // Active users
};
```

### Clock-Sweep Eviction

```cpp
void BufferManager::EvictIfNeeded() {
    if (used_memory <= max_memory) return;

    while (used_memory > max_memory) {
        auto& block = blocks[clock_hand];

        if (block->pinned || block->reference_count > 0) {
            // Skip pinned or in-use blocks
            clock_hand = (clock_hand + 1) % blocks.size();
            continue;
        }

        if (block->used) {
            // First pass: clear used flag
            block->used = false;
            clock_hand = (clock_hand + 1) % blocks.size();
        } else {
            // Second pass: evict
            if (block->dirty) {
                WriteToDisk(block);
            }
            used_memory -= block->size;
            block->Unload();
        }
    }
}
```

## Data Table Storage

### Row Group Structure

Tables are partitioned into **Row Groups** of ~120,000 rows:

```cpp
struct RowGroup {
    idx_t start;           // First row index
    idx_t count;           // Number of rows
    vector<ColumnSegment*> columns;  // Per-column data
    RowGroupStatistics stats;        // Min/max/null counts
};
```

### Column Segments

Each column in a row group has one or more segments:

```
Column Segment
├── Validity Segment (NULL bitmap)
├── Data Segment (compressed values)
└── Update Segment (MVCC info)
```

### Compression Chain

Each data segment uses a compression chain:

```cpp
struct ColumnSegment {
    CompressionType compression;  // FSST, RLE, Dictionary, etc.
    data_ptr_t compressed_data;   // Compressed bytes
    idx_t compressed_size;        // Size after compression
    idx_t uncompressed_size;      // Original size
    unique_ptr<CompressionInfo> info;  // Algorithm-specific metadata
};
```

## External File Cache

### Architecture

For reading from remote storage (S3, HTTP, GCS):

```cpp
class CachingFileSystem {
    FileSystem& file_system;           // Underlying FS (HTTP, S3)
    ExternalFileCache& external_file_cache;

    unique_ptr<CachingFileHandle> OpenFile(const OpenFileInfo& path, FileOpenFlags flags);
};

class CachingFileHandle : public FileHandle {
    CachingFileSystem& caching_file_system;
    ExternalFileCache& external_file_cache;
    CachedFile& cached_file;
    unique_ptr<FileHandle> file_handle;  // Underlying file (opened lazily)

    BufferHandle Read(data_ptr_t& buffer, idx_t nr_bytes, idx_t location);
};
```

### Cached File Structure

```cpp
struct CachedFile {
    string path;
    StorageLock lock;  // Reader-writer lock
    map<idx_t, shared_ptr<CachedFileRange>> ranges;  // location -> range
    idx_t file_size;
    time_t last_modified;
    string version_tag;  // ETag for HTTP/S3
    bool can_seek;
    bool on_disk_file;
};

struct CachedFileRange {
    shared_ptr<BlockHandle> block_handle;  // Pinned buffer
    idx_t nr_bytes;                         // Size
    idx_t location;                         // File offset
    string version_tag;                     // For validation
    uint64_t checksum;                      // Debug validation
};
```

### Read Algorithm

The read algorithm is optimized to minimize network requests:

```cpp
BufferHandle CachingFileHandle::Read(data_ptr_t& buffer, idx_t nr_bytes, idx_t location) {
    // 1. Try cache first
    vector<shared_ptr<CachedFileRange>> overlapping_ranges;
    BufferHandle result = TryReadFromCache(buffer, nr_bytes, location, overlapping_ranges);
    if (result.IsValid()) {
        return result;  // Cache hit
    }

    // 2. Allocate new buffer
    result = external_file_cache.GetBufferManager().Allocate(
        MemoryTag::EXTERNAL_FILE_CACHE, nr_bytes);
    auto new_file_range = make_shared_ptr<CachedFileRange>(
        result.GetBlockHandle(), nr_bytes, location, version_tag);
    buffer = result.Ptr();

    // 3. Interleave cached and fresh reads
    if (OnDiskFile()) {
        ReadAndCopyInterleaved(overlapping_ranges, new_file_range,
                               buffer, nr_bytes, location, true);
    } else {
        // Remote file: only interleave if it reduces network requests
        if (ReadAndCopyInterleaved(...) <= 1) {
            ReadAndCopyInterleaved(..., true);  // Actually read
        } else {
            GetFileHandle().Read(buffer, nr_bytes, location);  // Direct read
        }
    }

    // 4. Insert into cache
    return TryInsertFileRange(result, buffer, nr_bytes, location, new_file_range);
}
```

### Interleaved Read/Copy

This optimization reduces network round-trips by combining cached and fresh data:

```cpp
idx_t ReadAndCopyInterleaved(
    const vector<shared_ptr<CachedFileRange>>& overlapping_ranges,
    const shared_ptr<CachedFileRange>& new_file_range,
    data_ptr_t buffer,
    idx_t nr_bytes,
    idx_t location,
    bool actually_read) {

    idx_t non_cached_read_count = 0;
    idx_t current_location = location;
    idx_t remaining_bytes = nr_bytes;

    for (auto& overlapping_range : overlapping_ranges) {
        if (remaining_bytes == 0) break;

        // Read gap before cached range
        if (overlapping_range->location > current_location) {
            idx_t bytes_to_read = overlapping_range->location - current_location;
            if (actually_read) {
                GetFileHandle().Read(buffer + offset, bytes_to_read, current_location);
            }
            current_location += bytes_to_read;
            remaining_bytes -= bytes_to_read;
            non_cached_read_count++;
        }

        // Copy from cached range
        auto pinned = external_file_cache.GetBufferManager().Pin(overlapping_range->block_handle);
        if (pinned.IsValid()) {
            idx_t bytes_to_copy = min(remaining_bytes,
                                       overlapping_range->nr_bytes - offset);
            if (actually_read) {
                memcpy(buffer + offset, pinned.Ptr() + range_offset, bytes_to_copy);
            }
            current_location += bytes_to_copy;
            remaining_bytes -= bytes_to_copy;
        }
    }

    // Read remaining bytes at end
    if (remaining_bytes > 0) {
        if (actually_read) {
            GetFileHandle().Read(buffer + offset, remaining_bytes, current_location);
        }
        non_cached_read_count++;
    }

    return non_cached_read_count;  // Number of actual network requests
}
```

### Cache Validation

For remote files, validation ensures cached data is still fresh:

```cpp
bool ExternalFileCache::IsValid(
    bool validate,
    const string& cached_version_tag,
    time_t cached_last_modified,
    const string& current_version_tag,
    time_t current_last_modified) {

    if (!validate) return true;

    // Prefer version tag (ETag) - more reliable
    if (!current_version_tag.empty() || !cached_version_tag.empty()) {
        return cached_version_tag == current_version_tag;
    }

    // Fall back to last-modified time
    if (cached_last_modified != current_last_modified) {
        return false;
    }

    // Low-resolution filesystem clock tolerance
    static constexpr int64_t LAST_MODIFIED_THRESHOLD = 10;
    auto access_time = duration_cast<seconds>(system_clock::now().time_since_epoch()).count();

    if (access_time < current_last_modified) return false;  // Future?
    return access_time - current_last_modified > LAST_MODIFIED_THRESHOLD;
}
```

## Checkpoint Manager

### Purpose

The CheckpointManager creates persistent snapshots:

1. Flushes all dirty blocks to disk
2. Creates a new pointer block
3. Updates the header with the new checkpoint location

### Checkpoint Flow

```cpp
class CheckpointManager {
    BlockManager& block_manager;
    CheckpointState state;

    void Checkpoint(ClientContext& context) {
        // 1. Get all dirty blocks
        auto dirty_blocks = block_manager.GetDirtyBlocks();

        // 2. Write dirty blocks
        for (auto& block : dirty_blocks) {
            block_manager.Write(block);
        }

        // 3. Create pointer block
        PointerBlock pointer_block;
        for (auto& block : dirty_blocks) {
            pointer_block.AddBlock(block->block_id, block->new_location);
        }

        // 4. Write pointer block
        auto pointer_location = block_manager.Allocate();
        block_manager.Write(pointer_location, &pointer_block);

        // 5. Update header
        block_manager.UpdateHeader(pointer_location);
    }
};
```

## Write-Ahead Log (WAL)

### Structure

```
WAL File
├── WAL Header
│   ├── Magic bytes
│   ├── Start LSN
│   └── End LSN
├── WAL Entry 1
│   ├── LSN (Log Sequence Number)
│   ├── Entry type (INSERT/UPDATE/DELETE)
│   ├── Table ID
│   └── Data
├── WAL Entry 2
└── ...
```

### Replay

On startup:

```cpp
void WAL::Replay(ClientContext& context) {
    auto entries = ReadAllEntries();
    for (auto& entry : entries) {
        switch (entry.type) {
            case WALEntryType::INSERT:
                table.Insert(entry.data);
                break;
            case WALEntryType::UPDATE:
                table.Update(entry.row_id, entry.data);
                break;
            case WALEntryType::DELETE:
                table.Delete(entry.row_id);
                break;
        }
    }
}
```

## Compression Algorithms

### FSST (Fast Static Symbol Table)

#### Compression

```cpp
class FSSTCompression {
    struct Symbol {
        uint8_t length;     // 1-8 bytes
        uint8_t symbol;     // 0-255
        uint8_t data[8];    // Original bytes
    };

    vector<Symbol> symbols;  // 256 entries

    // Build symbol table from sample
    void BuildSample(const data_ptr_t data, idx_t size) {
        // Count byte frequencies
        // Count bigram frequencies
        // Greedily select best symbols
    }

    // Compress data
    idx_t Compress(const data_ptr_t input, idx_t input_size,
                   data_ptr_t output, idx_t output_size) {
        idx_t output_pos = 0;
        idx_t input_pos = 0;

        while (input_pos < input_size) {
            bool found = false;
            for (size_t len = min(8, input_size - input_pos); len >= 1; len--) {
                auto symbol = FindSymbol(input + input_pos, len);
                if (symbol) {
                    output[output_pos++] = symbol;
                    input_pos += len;
                    found = true;
                    break;
                }
            }
            if (!found) {
                // Escape sequence for unmatched byte
                output[output_pos++] = 0xFE;  // ESC
                output[output_pos++] = input[input_pos++];
            }
        }

        return output_pos;
    }
};
```

#### Decompression

```cpp
idx_t Decompress(const data_ptr_t input, idx_t input_size,
                 data_ptr_t output, idx_t output_size) {
    idx_t output_pos = 0;
    idx_t input_pos = 0;

    while (input_pos < input_size) {
        uint8_t symbol = input[input_pos++];
        if (symbol == 0xFE) {
            // Escape sequence
            output[output_pos++] = input[input_pos++];
        } else {
            // Symbol lookup
            auto& entry = symbols[symbol];
            memcpy(output + output_pos, entry.data, entry.length);
            output_pos += entry.length;
        }
    }

    return output_pos;
}
```

### Bit-Packing

For integers with known maximum:

```cpp
class BitPacking {
    // Pack 32 values into bits_per_value * 32 bits
    void Pack(const uint32_t* input, idx_t count,
              data_ptr_t output, uint8_t bits_per_value) {
        idx_t bit_pos = 0;
        for (idx_t i = 0; i < count; i++) {
            uint32_t value = input[i];
            for (uint8_t b = 0; b < bits_per_value; b++) {
                if (value & (1 << b)) {
                    output[bit_pos / 8] |= (1 << (bit_pos % 8));
                }
                bit_pos++;
            }
        }
    }

    void Unpack(const data_ptr_t input, idx_t count,
                uint32_t* output, uint8_t bits_per_value) {
        idx_t bit_pos = 0;
        for (idx_t i = 0; i < count; i++) {
            uint32_t value = 0;
            for (uint8_t b = 0; b < bits_per_value; b++) {
                if (input[bit_pos / 8] & (1 << (bit_pos % 8))) {
                    value |= (1 << b);
                }
                bit_pos++;
            }
            output[i] = value;
        }
    }
};
```

### Dictionary Compression

```cpp
class DictionaryCompression {
    vector<Value> dictionary;  // Unique values
    vector<uint32_t> indices;   // Value IDs

    void Build(const Value* input, idx_t count) {
        map<Value, uint32_t> value_to_id;
        uint32_t next_id = 0;

        for (idx_t i = 0; i < count; i++) {
            auto it = value_to_id.find(input[i]);
            if (it == value_to_id.end()) {
                value_to_id[input[i]] = next_id;
                dictionary.push_back(input[i]);
                indices.push_back(next_id);
                next_id++;
            } else {
                indices.push_back(it->second);
            }
        }
    }
};
```

## Performance Optimizations

### 1. Vectorized Scans

Scanning a column reads entire vectors:

```cpp
void TableScan::Scan(ColumnScanState& state, Vector& output) {
    // Read 2048 values at once
    state.column_data->Scan(state.segment, state.row_offset,
                            output, STANDARD_VECTOR_SIZE);
    state.row_offset += STANDARD_VECTOR_SIZE;
}
```

### 2. Predicate Pushdown

Filters are applied during scan, not after:

```cpp
void TableScan::ScanWithFilter(ColumnScanState& state,
                                Vector& output,
                                const TableFilter& filter) {
    Vector all_data;
    Scan(state, all_data);

    // Apply filter inline
    SelectionVector sel;
    filter.Select(all_data, sel);

    // Only return matching rows
    output.Slice(all_data, sel);
}
```

### 3. Parallel Scans

Multiple threads scan different row groups:

```cpp
void TableScan::ParallelScan(ClientContext& context,
                              function<void(DataChunk&)> callback) {
    auto row_groups = table.GetRowGroups();

    ParallelStateMergeScan(row_groups, [&](idx_t start, idx_t end) {
        for (idx_t rg = start; rg < end; rg++) {
            DataChunk chunk;
            ScanRowGroup(row_groups[rg], chunk);
            callback(chunk);
        }
    });
}
```

### 4. Statistics-Based Pruning

Skip row groups that can't match:

```cpp
bool RowGroupPruner::ShouldScan(const RowGroup& rg,
                                 const TableFilter& filter) {
    auto& stats = rg.GetStatistics();

    switch (filter.type) {
        case TableFilterType::CONSTANCE_COMPARISON:
            return filter.Compare(stats.min, stats.max);
        case TableFilterType::IS_NULL:
            return stats.has_null;
        // ... more filter types
    }
}
```

---

This deep-dive covers the storage engine. See other documents for query execution, object storage, and compression details.
