# xs -- SCRU128 IDs

## What Is SCRU128?

**File**: `src/scru128.rs`

SCRU128 (Sortable, Clock-based, Randomly Unique 128-bit) is a time-ordered unique identifier. It's similar to ULID or UUIDv7 but with these properties:

- **128-bit** — Represented as 25-character base-36 strings
- **Time-ordered** — Lexicographic sort = chronological sort
- **Monotonic** — Within the same millisecond, counter fields ensure strict ordering
- **Globally unique** — Random node component prevents collisions across processes
- **Millisecond precision** — Timestamp is embedded in the upper bits

## Why SCRU128 for xs?

xs uses SCRU128 IDs as its primary key in the LSM-tree. Because they're time-ordered:
- Sequential scan = chronological event replay
- Range queries = time-range queries
- Last N frames = read from the end of the keyspace
- "After" cursor = simple key comparison

## ID Components

```rust
struct Scru128Components {
    timestamp: f64,      // Unix timestamp in seconds (ms precision)
    counter_hi: u32,     // High counter (monotonic within same ms)
    counter_lo: u32,     // Low counter (further monotonicity)
    node: String,        // 8-hex-char random/node entropy
}
```

### Bit Layout (128 bits total)

```
┌─────────────────────────────────────────────────────────────────────┐
│ timestamp (48 bits) │ counter_hi (24 bits) │ counter_lo (24 bits) │ node (32 bits) │
└─────────────────────────────────────────────────────────────────────┘
```

- **Timestamp** (48 bits): Milliseconds since Unix epoch. Covers ~8900 years.
- **Counter_hi** (24 bits): Increments when multiple IDs generated in the same millisecond.
- **Counter_lo** (24 bits): Further ordering within same ms + counter_hi.
- **Node** (32 bits): Random entropy per generator instance.

## Functions

### generate()

```rust
pub fn new() -> Scru128Id
```

Creates a new SCRU128 ID using the thread-local generator. Guaranteed to be greater than any previously generated ID from the same generator.

### unpack(id) -> Components

```rust
pub fn unpack(id: &Scru128Id) -> Scru128Components
```

Decomposes an ID into its constituent parts:
```rust
let id = scru128::new();
let parts = scru128::unpack(&id);
// parts.timestamp = 1714924800.123
// parts.counter_hi = 42
// parts.counter_lo = 7
// parts.node = "a1b2c3d4"
```

### pack(components) -> Id

```rust
pub fn pack(components: &Scru128Components) -> Scru128Id
```

Reconstructs an ID from components. Useful for:
- Creating IDs with specific timestamps (e.g., "give me an ID for 5 minutes ago")
- Cursor-based pagination ("start after this timestamp")

### unpack_to_json / pack_from_json

```rust
pub fn unpack_to_json(id: &Scru128Id) -> serde_json::Value
pub fn pack_from_json(json: &serde_json::Value) -> Result<Scru128Id>
```

JSON-serialized component manipulation for the Nushell `.id` command.

## How xs Uses SCRU128

### As Primary Key

```rust
// Store a frame
let key = id.as_bytes(); // 16-byte big-endian representation
stream_keyspace.put(key, frame_json)?;
```

Big-endian byte representation ensures lexicographic byte ordering matches chronological ordering.

### As Cursor for Pagination

```rust
// "Give me everything after this frame"
ReadOptions { after: Some(last_seen_id), .. }

// Internally: scan from after_id + 1
```

### As Topic Index Component

```rust
// Index key: topic + NULL + frame_id
let key = format!("{}\x00", topic).into_bytes();
key.extend_from_slice(id.as_bytes());
```

### Timestamp Extraction

```rust
// --with-timestamp flag adds timestamp to output
let components = scru128::unpack(&frame.id);
// components.timestamp is seconds since epoch with ms precision
```

## CLI: xs scru128

### Generate

```bash
$ xs scru128
0123456789abcdefghijklmno
```

### Unpack

```bash
$ xs scru128 unpack 0123456789abcdefghijklmno
{
  "timestamp": 1714924800.123,
  "counter_hi": 42,
  "counter_lo": 7,
  "node": "a1b2c3d4"
}
```

### Pack

```bash
$ echo '{"timestamp": 1714924800.0, "counter_hi": 0, "counter_lo": 0, "node": "00000000"}' | xs scru128 pack
<reconstructed-id>
```

## Monotonicity Guarantee

The SCRU128 generator maintains state per-thread:
- If current timestamp > last timestamp: use new timestamp, reset counters
- If current timestamp == last timestamp: increment counter_hi
- If current timestamp < last timestamp (clock drift): use last timestamp + increment

This means xs frames are **always** in increasing ID order within a single store, regardless of clock skew.

## String Representation

SCRU128 IDs are 25 characters in base-36 (digits 0-9 + letters a-z):
```
0v4fkdz7k2mfj9f2nn8yx3h71
```

Base-36 was chosen over base-64 because:
- Case-insensitive (safe for filenames, URLs, DNS)
- Alphanumeric only (no special characters)
- Lexicographic sort in string form matches numeric sort
