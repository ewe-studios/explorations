# Sendme and Dumbpipe: File Transfer Applications

## Overview

This document explores two command-line applications in the n0-computer ecosystem that leverage iroh's P2P networking capabilities for file transfer and data piping:

1. **sendme** - File and directory transfer with BLAKE3 verified streaming
2. **dumbpipe** - Network pipe utility with NAT hole punching

Both tools demonstrate practical applications of the underlying iroh networking stack.

---

# Part 1: Sendme - File Transfer with Verified Streaming

## Overview

`sendme` is a CLI tool for transferring files and directories between machines using iroh's P2P networking with BLAKE3 verified streaming.

**Repository:** https://github.com/n0-computer/sendme
**Version:** 0.32.0
**License:** Apache-2.0 OR MIT
**Rust Version:** 1.89+

## Key Features

### Verified Streaming

Uses `iroh-blobs` and `bao-tree` for:
- Content-addressed transfer (BLAKE3 hashes)
- Integrity verification during streaming
- Resume capability for interrupted transfers

### NAT Traversal

Automatic connectivity handling:
- Direct connection when possible
- Hole punching through NATs
- Relay fallback when direct fails

### Location Transparency

- Works with 256-bit node IDs instead of IP addresses
- Tickets remain valid across IP changes
- TLS encryption for all connections

## Architecture

### Dependencies

```toml
[dependencies]
iroh = "0.97"
iroh-blobs = "0.99"
irpc = "0.13.0"
n0-future = "0.3"
tokio = { version = "1.34.0", features = ["full"] }
clap = { version = "4.4.10", features = ["derive"] }
indicatif = "0.17.7"  # Progress bars
console = "0.15.7"     # Terminal UI
```

### Command Structure

```
sendme
├── send <path>      - Send a file or directory
└── receive <ticket> - Receive from a ticket
```

### Send Flow

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Sender    │────▶│  Iroh Relay  │◀────│  Receiver   │
│             │     │  (if needed) │     │             │
│ 1. Create   │     └──────────────┘     │ 4. Connect  │
│    ticket   │                          │    to ticket│
│ 2. Add file │                          │             │
│    to store │     ┌──────────────┐     │ 5. Download│
│ 3. Print    │────▶│  Direct P2P  │────▶│    & verify│
│    ticket   │     │  Connection  │     │             │
└─────────────┘     └──────────────┘     └─────────────┘
```

### Receive Flow

```rust
// Simplified receive flow
async fn receive(ticket: BlobTicket) -> Result<()> {
    // 1. Create endpoint
    let endpoint = Endpoint::builder()
        .alpns(vec![BlobsProtocol::ALPN.to_vec()])
        .bind(0)
        .await?;

    // 2. Connect to provider
    let mut getter = Getter::new(endpoint, ticket);

    // 3. Download with progress
    let stats = getter.download(progress_cb).await?;

    // 4. Verify and export
    export_to_path(stats.hash, &target_path).await?;

    Ok(())
}
```

## Implementation Details

### Ticket Format

Tickets encode all connection information:

```rust
#[derive(Serialize, Deserialize)]
pub struct BlobTicket {
    /// Hash of the content
    hash: Hash,
    /// Provider's node ID
    node_id: PublicKey,
    /// Connection information (addresses, relay URLs)
    addrs: Vec<TransportAddr>,
    /// Optional secret for access control
    secret: Option<blake3::Hash>,
}
```

Encoded as base32 for CLI usage.

### Progress Tracking

Uses `indicatif` for terminal progress:

```rust
let progress = MultiProgress::new();
let style = ProgressStyle::default_bar()
    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?;

let pb = progress.add(ProgressBar::new(total_size));
pb.set_style(style);

// Update during transfer
pb.set_position(downloaded_bytes);
```

### Collection Handling

For directory transfers, uses iroh collection format:

```rust
use iroh_blobs::format::collection::Collection;

let collection = Collection::from_iter(files);
let hash = collection.hash();
```

Collection stores:
- File paths
- File sizes
- Individual file hashes
- Total order for deterministic transfer

### Export Modes

```rust
pub enum ExportMode {
    /// Copy data to target
    Copy,
    /// Move data (if on same filesystem)
    TryMove,
    /// Leave in cache, create reference
    Reference,
}
```

## Usage Examples

### Send a File

```bash
$ sendme send myfile.pdf
Sending myfile.pdf (2.5 MiB)
Ticket: sendme1qqr... (copied to clipboard)
Waiting for receiver...
Receiver connected!
Transfer complete!
```

### Receive a File

```bash
$ sendme receive sendme1qqr...
Connecting to provider...
Downloading myfile.pdf (2.5 MiB)
[========================================] 2.5 MiB / 2.5 MiB (3s)
Verifying...
Saved to ./myfile.pdf
```

### Send a Directory

```bash
$ sendme send ./project/
Sending project/ (15.3 MiB, 42 files)
Ticket: sendme1qqr...
```

## Performance Characteristics

### Transfer Speeds

- **Local network**: Up to 100 MB/s (network limited)
- **Internet direct**: Varies by connection
- **Via relay**: Limited by relay capacity

### Memory Usage

- Streaming: O(chunk_size) = O(16 KiB) per stream
- Collection: O(file_count) for metadata
- Verification: Minimal (hash computation)

## Security Considerations

### Encryption

- All connections use TLS 1.3 via QUIC
- Forward secrecy with ephemeral keys
- Certificate validation

### Access Control

- Tickets are effectively capability tokens
- Optional secret for additional protection
- No authentication by default (ticket = access)

---

# Part 2: Dumbpipe - Network Pipe with Hole Punching

## Overview

`dumbpipe` is a modern replacement for `netcat` that works with node IDs instead of IP addresses, providing encrypted P2P connections with automatic NAT traversal.

**Repository:** https://github.com/n0-computer/dumbpipe
**Version:** 0.35.0
**License:** MIT OR Apache-2.0
**Rust Version:** 1.89+

## Key Features

### Node ID Based

- Works with 256-bit endpoint IDs
- Location transparent (works across IP changes)
- No need to know peer's IP address

### NAT Traversal

- Automatic hole punching
- Relay fallback
- Works through most NATs and firewalls

### Encrypted

- TLS 1.3 encryption
- Mutual authentication
- Forward secrecy

### Multiple Modes

- stdin/stdout piping
- TCP forwarding
- Unix socket forwarding (Unix only)

## Architecture

### Dependencies

```toml
[dependencies]
iroh = "0.97"
iroh-tickets = "0.4"
noq = "0.17"  # QUIC utilities
tokio = { version = "1.34.0", features = ["full"] }
clap = { version = "4.4.10", features = ["derive"] }
```

### Command Structure

```
dumbpipe
├── generate-ticket        - Generate a reusable ticket
├── listen                 - Listen on stdin/stdout
├── listen-tcp             - Listen and forward to TCP
├── listen-unix            - Listen and forward to Unix socket
├── connect                - Connect to stdin/stdout
├── connect-tcp            - Connect and forward from TCP
└── connect-unix           - Connect and forward from Unix socket
```

## Usage Examples

### Basic stdin/stdout Piping

**Sender:**
```bash
$ dumbpipe listen
Listening on endpoint: abc123...
Ticket: dumbpipe1qqr...
```

**Receiver:**
```bash
$ dumbpipe connect dumbpipe1qqr...
Connected! Type to send:
Hello from receiver!
```

### Video Streaming with ffmpeg

**Sender (Mac):**
```bash
ffmpeg -f avfoundation -r 30 -i "0" -pix_fmt yuv420p -f mpegts - | \
  dumbpipe listen
```

**Receiver:**
```bash
dumbpipe connect <ticket> | ffplay -f mpegts -fflags nobuffer -
```

### Forward Development Server

**On dev machine:**
```bash
# Start dev server
npm run dev  # Running on localhost:3000

# Forward it
dumbpipe listen-tcp --host localhost:3000
# Outputs ticket
```

**On client machine:**
```bash
# Connect and expose locally
dumbpipe connect-tcp --addr 0.0.0.0:3001 <ticket>

# Now accessible at http://localhost:3001
```

### Unix Socket Forwarding (Zellij Example)

**Remote host:**
```bash
# Forward remote Zellij socket
dumbpipe listen-unix --socket-path /tmp/zellij-0/0.42.2/session
```

**Local machine:**
```bash
# Create local socket connected to remote
mkdir -p /tmp/zj-remote/0.42.1
dumbpipe connect-unix --socket-path /tmp/zj-remote/0.42.1/session <ticket>

# Attach local Zellij
ZELLIJ_SOCKET_DIR=/tmp/zj-remote zellij attach session
```

### tty-share for Pair Programming

**Host:**
```bash
dumbpipe listen-tcp --host localhost:8000 &
tty-share
```

**Client:**
```bash
dumbpipe connect-tcp --addr localhost:8000 <ticket> &
tty-share http://localhost:8000/s/local/
```

## Implementation Details

### Ticket Generation

```rust
use iroh_tickets::EndpointTicket;

let ticket = EndpointTicket {
    node_id: endpoint.node_id(),
    addrs: endpoint.remote_info().await?,
    secret_key: None, // Reuse same key for reusable tickets
};
```

### Connection Handling

```rust
async fn handle_connection(
    accepting: Accepting,
    forward_addr: SocketAddr,
) -> Result<()> {
    // Accept the incoming connection
    let connection = accepting.accept().await?;

    // Open bidirectional stream
    let mut stream = connection.open_bi().await?;

    // Forward to TCP
    let mut tcp = TcpStream::connect(forward_addr).await?;

    // Copy data both ways
    tokio::io::copy_bidirectional(&mut stream, &mut tcp).await?;

    Ok(())
}
```

### ALPN Protocol

Uses custom ALPN for protocol identification:

```rust
const ALPN: &[u8] = b"/dumbpipe/1";
```

### Custom ALPN Support

Can interact with other iroh services:

```bash
# Use with iroh-blobs protocol
echo request | dumbpipe connect <ticket> --custom-alpn utf8:/iroh-bytes/2
```

## Comparison with Similar Tools

| Feature | dumbpipe | netcat | ngrok | socat |
|---------|----------|--------|-------|-------|
| NAT Traversal | Built-in | None | Server-based | None |
| Encryption | TLS 1.3 | None | TLS | Optional |
| Addressing | Node ID | IP:Port | URL | IP:Port |
| Hole Punching | Yes | No | N/A | No |
| Relay Fallback | Yes | No | Yes | No |

## Performance

### Connection Establishment

- Direct connection: ~100-500ms
- Hole punching: ~1-3 seconds
- Relay fallback: ~500ms + relay latency

### Throughput

- Local network: Near line rate
- Internet direct: Limited by bandwidth
- Via relay: Limited by relay capacity

## Security Analysis

### Strengths

- Mutual TLS authentication
- Forward secrecy
- No central server storing data
- Ephemeral connections

### Considerations

- Tickets are bearer tokens (who has ticket = access)
- No built-in access control beyond ticket possession
- Relay can see traffic metadata (not content)

## Integration with n0-computer Ecosystem

### iroh-blobs

Can speak iroh-blobs protocol directly:
```bash
echo "get <hash>" | dumbpipe connect <ticket> --alpn /iroh-bytes/2
```

### sendme

Uses similar underlying iroh primitives but with higher-level abstractions

### iroh-n0des

Can be used to connect to n0des for metrics and management

## Future Enhancements

### Planned Features

1. **Multiplexing**: Multiple streams over single connection
2. **Bandwidth limiting**: Rate control
3. **Connection pooling**: Reuse connections
4. **UDP forwarding**: For real-time applications
5. **WebSocket mode**: Browser accessibility

### Improvements

1. **Better progress indicators**
2. **Connection statistics**
3. **Automatic reconnection**
4. **Ticket expiration**

## Conclusion

Both `sendme` and `dumbpipe` demonstrate the practical utility of iroh's P2P networking:

- **sendme** focuses on reliable, verified file transfer with a user-friendly CLI
- **dumbpipe** provides a flexible networking primitive for arbitrary data piping

Together they form a toolkit for secure, NAT-traversing data transfer without cloud infrastructure.

## Related Resources

- [Iroh Documentation](https://iroh.computer/docs)
- [QUIC RFC 9000](https://www.rfc-editor.org/rfc/rfc9000.html)
- [BLAKE3](https://blake3.io)
- [netcat](https://en.wikipedia.org/wiki/Netcat)
