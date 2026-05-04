# Orbitinghail -- Storage Formats Deep Dive

This document details the on-disk and on-wire formats used across the orbitinghail ecosystem: SSTable layout, SFA archive format, WAL journal format, and graft segment format.

**Aha:** Every format in this ecosystem uses a trailer-at-end design. The SSTable trailer (31 bytes) contains offsets to all sections. The SFA trailer contains the ToC position. The WAL batch header contains the operation count. This means readers can find the structure metadata by seeking to the end of the file first — no magic number scanning or fixed-position headers required. The trailer is the table of contents.

## SSTable Format (lsm-tree)

Source: `lsm-tree/src/table/`

```
┌─────────────────────────────────────────────────────────────┐
│                        SSTable File                         │
├─────────────────────────────────────────────────────────────┤
│ Data Block 1                                                │
│ ┌──────────┬──────────────────┬──────────┬───────────────┐  │
│ │ Header   │ Prefix-compressed│ Binary   │ Hash index    │  │
│ │ (magic)  │ key-value pairs  │ index    │ (optional)    │  │
│ └──────────┴──────────────────┴──────────┴───────────────┘  │
├─────────────────────────────────────────────────────────────┤
│ Data Block 2 ...                                            │
├─────────────────────────────────────────────────────────────┤
│ Index Block                                                 │
│ (binary index for all data blocks, two-level for large)    │
├─────────────────────────────────────────────────────────────┤
│ Filter Block                                                │
│ (bloom filter: magic, type, hash, m, k, bit array)         │
├─────────────────────────────────────────────────────────────┤
│ Meta Block (data block containing key-value pairs)          │
│ - table_version (string key "table_version")                │
│ - table_id, item_count, tombstone_count                     │
│ - block_counts, file_size, created_at                       │
│ - key_range (min/max), seqno range                          │
│ - compression types, filter config                          │
├─────────────────────────────────────────────────────────────┤
│ Trailer (31 bytes)                                          │
│ - trailer_start_marker (0xFF, 1 byte)                       │
│ - binary_index_step_size                                    │
│ - binary_index_len, hash_index_len, hash_index_offset       │
│ - prefix_truncation, fixed_key_size, fixed_value_size       │
│ - item_count                                                │
│ - checksum_padding                                          │
└─────────────────────────────────────────────────────────────┘
```

### Data Block Trailer (31 bytes)

| Offset | Size | Field |
|--------|------|-------|
| 0 | 1 | Trailer start marker (0xFF) |
| 1 | 1 | Binary index step size |
| 2 | 4 | Binary index length |
| 6 | 4 | Hash index length |
| 10 | 4 | Hash index offset |
| 14 | 1 | Prefix truncation |
| 15 | 1 | Fixed key size |
| 16 | 1 | Fixed value size |
| 17 | 4 | Item count |
| 21 | 10 | Checksum padding |

### Prefix Compression Detail

Each key-value pair in a data block is encoded as:

```
┌───────────┬───────────┬──────────┬───────────┐
│ shared    │ unshared  │ value    │ value     │
│ prefix    │ key       │ size     │ bytes     │
│ length    │ suffix    │ (varint) │           │
│ (varint)  │ (varint)  │          │           │
└───────────┬───────────┴──────────┴───────────┘
            │
            At restart points: shared_prefix_length = 0
```

Varint encoding: 7 bits per byte, MSB = continuation bit. Values <128 use 1 byte.

## SFA Archive Format (Simple File-based Archive)

Source: `sfa/src/`

```
┌─────────────────────────────────────────────────────────────┐
│                     SFA Archive                             │
├─────────────────────────────────────────────────────────────┤
│ Section 1                                                   │
│ ┌──────────────────────┬──────────────────────────────────┐ │
│ │ Section header       │ Section data                     │ │
│ │ (name, size, offset) │ (arbitrary bytes)                │ │
│ └──────────────────────┴──────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│ Section 2 ...                                               │
├─────────────────────────────────────────────────────────────┤
│ Table of Contents (ToC)                                     │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Entry 1: {name, offset, size, checksum}                │ │
│ │ Entry 2: {name, offset, size, checksum}                │ │
│ │ ...                                                     │ │
│ └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│ Trailer                                                     │
│ ┌──────────────────┬───────────────────────┐ │
│ │ toc_position: u64│ toc_checksum: XXH3_128│ │
│ └──────────────────┴───────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Writer Process

```rust
let mut writer = SfaWriter::new(&mut file);
writer.start_section("config")?;
writer.write(&config_bytes)?;
writer.start_section("data")?;
writer.write(&data_bytes)?;
writer.finish()?;  // Writes ToC + trailer
```

The writer streams data sections sequentially, building the ToC in memory. On `finish()`, it writes the ToC, then the trailer.

### Reader Process

```rust
let reader = SfaReader::open(&file)?;
// 1. Read trailer from end of file
// 2. Read ToC at toc_position
// 3. Verify ToC checksum
// 4. Seek to individual sections by name
let config = reader.read_section("config")?;
```

**Aha:** SFA's design makes random access O(1) after the initial trailer read. The reader reads the last ~40 bytes (trailer), gets the ToC position, reads the ToC, and can then seek directly to any section. No need to scan from the beginning.

## WAL Journal Format (fjall)

Source: `fjall/src/journal/`

```
┌─────────────────────────────────────────────────────────────┐
│                     WAL File                                │
├─────────────────────────────────────────────────────────────┤
│ Batch 1                                                     │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Header: {batch_id, timestamp, num_ops, checksum}        │ │
│ ├─────────────────────────────────────────────────────────┤ │
│ │ Op 1: {keyspace_id, op_type, key_len, key, val_len?, val?}│ │
│ │ Op 2: ...                                               │ │
│ └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│ Batch 2 ...                                                 │
├─────────────────────────────────────────────────────────────┤
│ (Partially written batch — truncated on recovery)           │
└─────────────────────────────────────────────────────────────┘
```

During recovery, the WAL is read from the beginning. Each batch header's checksum is verified. If a batch's checksum fails, the batch and all subsequent data are discarded (they were partially written during a crash).

## Graft Segment Format

Source: `graft/crates/graft/src/remote/segment.rs`

```
┌─────────────────────────────────────────────────────────────┐
│                    Graft Segment                            │
├─────────────────────────────────────────────────────────────┤
│ Frame 1: ZStd compressed                                    │
│ ┌──────────────┬──────────────────────────────────────────┐ │
│ │ ZStd frame    │ Pages (concatenated, sorted by PageIdx) │ │
│ │ header        │ Page 1 | Page 2 | ... | Page N (≤64)   │ │
│ └──────────────┴──────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│ Frame 2: ZStd compressed                                    │
├─────────────────────────────────────────────────────────────┤
│ ...                                                         │
├─────────────────────────────────────────────────────────────┤
│ Frame Index (in memory, not on disk)                        │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ [(compressed_size, last_pageidx), ...]                  │ │
│ └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### ZStd Frame Options

| Option | Value | Reason |
|--------|-------|--------|
| Content size | Disabled | Frames are stream-chunked, size not known upfront |
| Checksum | Enabled | Each frame has its own integrity check |
| Compression level | 3 | Balance between speed and ratio |

See [LSM-Tree](02-lsm-tree.md) for SSTable design rationale.
See [Checksums and Validation](09-checksums-validation.md) for integrity verification.
See [Graft Storage](04-graft-storage.md) for segment usage.
