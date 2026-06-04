# src.WebTransport Task Tracker

## Projects: Documented (5/78)

| Project | Status | Documents | Lines |
|---------|--------|-----------|-------|
| **iroh** | ✅ DONE | 12 docs + spec | 5,767 |
| **iroh-gossip** | ✅ DONE | 11 docs + spec | 4,715 |
| **iroh-blobs** | ✅ DONE | 12 docs + spec | 4,998 |
| **moqdev** | ✅ DONE | 12 docs + spec | 3,404 |
| **websocket** | ✅ DONE | 8 docs + spec | 2,756 |

## Projects: Not Documented (73 src.n0-computer repos)

### Core Ecosystem — Priority 1
- [ ] `iroh-docs` (v0.35.0) — Eventually-consistent KV store on iroh-blobs
- [ ] `iroh-sync` (v0.4.1) — Set reconciliation, signature verification
- [ ] `iroh-car` (v0.5.0) — CAR file support (IPFS-compatible)
- [ ] `iroh-ffi` (v0.35.0) — FFI bindings for Python, etc.
- [ ] `iroh-c-ffi` (v0.90.0) — C FFI with irohnet.h header
- [ ] `iroh-js` — JavaScript/TypeScript client (Bun-based)
- [ ] `iroh-metrics` (v0.35.0) — Metrics collection with schema tracking
- [ ] `iroh-io` (v0.6.2) — Async I/O utilities
- [ ] `iroh-node-util` (v0.35.0) — Node utilities

### Protocol/Networking — Priority 2
- [ ] `irpc` (v0.5.0) — Streaming RPC over QUIC (replaces quic-rpc)
- [ ] `quic-rpc` (v0.20.0) — Predecessor to irpc
- [ ] `iroh-roq` (v0.1.0) — Reliable QUIC protocol
- [ ] `iroh-dns-server` (v1.0.0-rc.1) — PKarr relay + DNS server (dns.iroh.link)
- [ ] `iroh-discovery-cloudflare-worker` — Cloudflare worker for discovery

### Transport Layer — Priority 2
- [ ] `quinn` (iroh-quinn) — Quinn QUIC fork (quinn, quinn-proto, quinn-udp, bench, perf, fuzz)
- [ ] `rustls` (v0.23.25) — Rustls TLS fork
- [ ] `rustls-platform-verifier` — Platform-native TLS verification
- [ ] `tokio-rustls-acme` (v0.7.1) — ACME/Let's Encrypt TLS

### CLI Tools — Priority 3
- [ ] `dumbpipe` (v0.35.0) — P2P pipe CLI with NAT hole punching
- [ ] `sendme` (v0.32.0) — Directory transfer CLI over iroh-blobs
- [ ] `sendme-legacy` (v0.1.0) — Legacy sendme
- [ ] `iroh-doctor` (v0.90.1) — Network connectivity diagnostics
- [ ] `iroh-ping` (v0.2.0) — Ping/latency measurement

### Storage/Sync — Priority 3
- [ ] `bao-tree` (v0.15.1) — BLAKE3 verified streaming (core of iroh-blobs)
- [ ] `abao` (v0.2.0) — Alternative bao implementation
- [ ] `blobs2` (v0.90.0-alpha1) — Next-gen blob storage
- [ ] `iroh-blake3` (v1.4.6) — Blake3 variant
- [ ] `migrate-bao-store-redb` (v0.1.0) — Migration tool

### Protocols — Priority 3
- [ ] `iroh-willow` (v0.28.0) — Willow protocol implementation
- [ ] `willow-rs` — Willow protocol (Rust)
- [ ] `willow-store` (v0.1.0) — Willow storage backend
- [ ] `krakensync` (v0.1.0) — Sync implementation
- [ ] `swarm-discovery` (v0.3.0-alpha.1) — Swarm-based discovery
- [ ] `swarmie` (v0.1.0) — Swarm utilities
- [ ] `riblt-ts` — Rateless IBLT (TypeScript)

### Utilities — Priority 4
- [ ] `n0-future` (v0.3.2) — Future utilities, WASM-compatible
- [ ] `n0-watcher` (v0.3.0) — Watchable state tracking
- [ ] `n0-snafu` (v0.2.1) — Error handling
- [ ] `async-channel` (v2.3.1) — Async channel primitives
- [ ] `net-tools` — Network utilities
- [ ] `dag-cbor-references` (v0.1.0) — DAG-CBOR reference handling
- [ ] `nested-enum-utils` (v0.2.2) — Enum utilities
- [ ] `varint-bench` (v0.1.0) — Varint benchmarks
- [ ] `appa` (v0.1.0) — ?
- [ ] `rcan` (v0.1.0) — ?
- [ ] `ufotofu` — ?
- [ ] `squiggle` — ?
- [ ] `waht` — ?
- [ ] `callme` — ?
- [ ] `chuck` — Testing/fixtures
- [ ] `imsg` (v0.1.0) — Messaging library
- [ ] `metrics_exporter` (v0.0.1) — Metrics exporter

### Applications — Priority 4
- [ ] `beetle` — IPFS-compatible over Iroh (bitswap, UnixFS, gateway, P2P)
- [ ] `iroh-thorium-reader` (v3.1.0-alpha.1) — EPUB reader based on Thorium
- [ ] `iroh-duck` (v0.1.0) — DuckDB integration
- [ ] `iroh-n0des` (v0.1.0) — Simulation and trace protocol testing
- [ ] `gst-plugin-iroh` (v0.1.0) — GStreamer plugin for Iroh
- [ ] `iroh-examples` — Example applications
- [ ] `iroh-example-todos` (v0.1.0) — Todo app (Tauri)
- [ ] `iroh-experiments` — Experimental projects (content-discovery, h3-iroh, iroh-dag-sync, iroh-pkarr-naming, iroh-s3-bao-store)

### Websites/Docs — Priority 5
- [ ] `www.iroh.computer` (v1.0.0) — Company website (Zola)
- [ ] `n0.computer` (v1.0.0) — n0 website (Zola)
- [ ] `dumbpipe.dev` — Dumbpipe docs (Next.js)
- [ ] `awesome-iroh` — Curated resources list
- [ ] `bao-docs` — Bao documentation
- [ ] `attic` (v0.2.0) — Archive
- [ ] `workflows` — CI/CD workflows

### Workshops — Priority 5
- [ ] `iroh-workshop-jonthebeach`
- [ ] `iroh-workshop-omniopencon`
- [ ] `iroh-workshop-web3summit`

### Other
- [ ] `discord_zerobot` — Discord bot (TypeScript)

---

## Grandfather Review Discrepancies — iroh-blobs (29 items)

### CRITICAL — Fix Before Any Reader Uses Docs

- [ ] **ALPN completely wrong**: Docs claim `b"/iroh-blobs/1"`, actual is `b"/iroh-bytes/4"` (protocol.rs:397)
- [ ] **GetRequest struct wrong**: Docs claim `hash, ranges, format` — actual has `hash, ranges` (no format field)
- [ ] **PushRequest struct wrong**: Docs claim `hash, format` — actual is `newtype(GetRequest)` with no format field
- [ ] **ObserveRequest struct wrong**: Docs claim `hash` only — actual has `hash, ranges: RangeSpec`
- [ ] **GetManyRequest struct wrong**: Docs claim `RangeSpecSeq` — actual uses `ChunkRangesSeq`
- [ ] **Request enum**: Docs show 4 variants — actual has 10 (Get, Observe, Slot2-7, Push, GetMany)
- [ ] **Closed enum**: Docs show `Success, Abort, InternalError` — actual has `StreamDropped(0), ProviderTerminating(1), RequestReceived(2)`
- [ ] **BlobTicket field name**: Docs claim `addr: NodeAddr` — actual is `node: NodeAddr`
- [ ] **TempTag structure**: Docs claim `hash + AtomicUsize` — actual is `HashAndFormat + Weak<dyn TagDrop>`
- [ ] **CollectionMeta wire format**: Docs claim `version + blobs` — actual has `header: [u8;13]` ("CollectionV0.") + `names: Vec<String>`
- [ ] **API Error types**: Docs claim `Store/Protocol/Network` — actual is single-variant `Io(io::Error)`
- [ ] **RequestError**: Docs claim `NotFound/BadRequest/Io` — actual has `Rpc { source: irpc::Error }` + `Inner`

### HIGH — Wrong Numbers and Features

- [ ] **rust-version**: Docs claim 1.91 — actual is 1.85 (Cargo.toml:12)
- [ ] **iroh version**: Docs claim `=1.0.0-rc.1` — actual is `0.90` (Cargo.toml:40,43)
- [ ] **iroh-metrics version**: Docs claim `=1.0.0-rc.0` — actual is `0.35`
- [ ] **redb version**: Docs claim `"2"` — actual is `"=2.4"`
- [ ] **Feature flags**: Docs claim 4 (`default, redb, tokio-io, test-utils`) — actual has 3 (`hide-proto-docs, metrics, default`)
- [ ] **IROH_BLOCK_SIZE type**: Docs claim `u32` — actual is `BlockSize::from_chunk_log(4)`
- [ ] **MAX_MESSAGE_SIZE**: Docs claim 64MB — actual is 100 MiB (1024*1024)
- [ ] **Metrics count**: Docs claim 10 — actual is 12 with completely different names
- [ ] **Metrics names**: 4 fabricated names, 4 real ones missing from docs
- [ ] **Provider Event variants**: Docs claim 5 — actual has 9 (`ClientConnected, ConnectionClosed, GetRequestReceived, GetManyRequestReceived, PushRequestReceived, TransferStarted, TransferProgress, TransferCompleted, TransferAborted`)
- [ ] **TransferStats fields**: Docs claim `bytes_sent, chunks_sent, duration` — actual has `payload_bytes_sent, other_bytes_sent, bytes_read, duration`
- [ ] **GetRequest utilities**: Fabricated function names in docs

### MODERATE — Incomplete Flows

- [ ] **Provider handle_connection**: Docs show simple dispatch — actual has authorization step before accepting requests, goes through `handle_stream`
- [ ] **Import pipeline**: Docs show 4 steps — actual has 3 paths (`import_bytes, import_byte_stream, import_path`) with inline switching, reflink support
- [ ] **GC mark-sweep**: Docs say "walk tags" — actual also walks `list_temp_tags()` and recursively traverses HashSeq children
- [ ] **Client FSM**: Diagram oversimplifies — actual has `AtConnected` branching to `AtStartRoot/AtStartChild/AtClosing`, `AtBlobHeader` can `drain()` or `finish()` to skip to closing
- [ ] **EntryState Partial**: Docs claim has `data_location, outboard_location` — actual only has `size: Option<u64>`

### MINOR — Coverage Gaps

- [ ] 30+ API methods on `Blobs` undocumented (batch, add_slice, list, status, has, export_chunk, etc.)
- [ ] 12+ Tags API methods undocumented (list_temp_tags, list_range, list_prefix, delete_all, etc.)
- [ ] Store accessors undocumented (tags(), blobs(), remote(), downloader(), connect(), listen(), shutdown())
- [ ] BlobStatus enum not documented (`NotFound, Partial, Complete`)
- [ ] AddProgressItem enum not documented (6 variants)
- [ ] BaoFileHandle lifecycle not documented (Handle, HandleWeak, HandleInner)
- [ ] DataReader/OutboardReader not documented
- [ ] 14+ source files not mentioned (store/fs/meta.rs, store/fs/delete_set.rs, util/channel.rs, etc.)
- [ ] Missing dependencies in docs: iroh-io, quinn (iroh-quinn), irpc, genawaiter, ref-cast, nested_enum_utils

---

## Grandfather Review Discrepancies — iroh-blobs ✅ FIXED

All 20 CRITICAL/HIGH discrepancies fixed and committed (commit f4e6546).

## Grandfather Review Discrepancies — websocket ✅ FIXED

All 9 discrepancies fixed and committed (commit 20401b6).

## Grandfather Review Discrepancies — moqdev ✅ FIXED

All 9 discrepancies fixed and committed (commit 63e0bcc).

## Grandfather Review Discrepancies — iroh ⏳ PENDING

Review complete (28 discrepancies found). CRITICAL issues:
- Report struct fields completely wrong (net_report doc)
- EndpointAddr struct fields completely wrong
- AcceptError variants completely wrong
- Probe enum has no data fields
- Endpoint::accept() return type wrong
- Endpoint::online() return type wrong
- Relay QUIC port 7842, not 443
- ws_stream_wasm dependency does not exist
- Default relay hostnames completely wrong (regional canary domains, not relays.iroh.link)
- 15 missing source files in module map
- ProtocolMap BTreeMap prefix-match reasoning false

## Grandfather Review Discrepancies — iroh-gossip ⏳ PENDING

Review complete (65+ discrepancies). CRITICAL issues - many fabricated types/structs:
- PeerData struct: 3 fields vs. actual newtype(Bytes)
- IO trait: 4 methods (sign/verify/encode/decode) vs. actual 1 method (push)
- GossipApi: sender/receiver vs. actual irpc Client
- GossipTopic: publish/next vs. actual broadcast/broadcast_neighbors
- Command enum: 4 wrong variants vs. 3 actual
- ApiError: 4 wrong variants vs. 2 actual
- Message struct: 4 wrong fields vs. 3 actual
- HyParView Config: 5 wrong fields/values vs. 9 actual
- Plumtree Config: 4 wrong fields vs. 7 actual
- StreamHeader: 2 wrong fields vs. 1 actual
- DEFAULT_MAX_MESSAGE_SIZE: 64MB vs. 4096
- MIN_MAX_MESSAGE_SIZE: 64KB vs. 512
- active_view_capacity: 30 claimed vs. 5 actual
- passive_view_capacity: 50 claimed vs. 30 actual
