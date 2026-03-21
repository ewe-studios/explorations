# Iroh IDs: Node Identification Deep Dive

## Overview

This document explores how iroh uses cryptographic identifiers (EndpointIds) to uniquely identify nodes in the P2P network, how these IDs relate to public keys, and how clients use them to establish connections.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/`

---

## Table of Contents

1. [What is an EndpointId?](#what-is-an-endpointid)
2. [EndpointId Structure](#endpointid-structure)
3. [From Public Key to EndpointId](#from-public-key-to-endpointid)
4. [EndpointAddr: Making IDs Addressable](#endpointaddr-making-ids-addressable)
5. [Address Resolution Flow](#address-resolution-flow)
6. [Connection Establishment](#connection-establishment)
7. [ID Encoding and Display](#id-encoding-and-display)
8. [Practical Examples](#practical-examples)

---

## What is an EndpointId?

### Definition

```rust
// From iroh-base/src/key.rs

/// The identifier for an endpoint in the (iroh) network.
///
/// Each endpoint in iroh has a unique identifier created as a cryptographic key.
/// This can be used to globally identify an endpoint. Since it is also a
/// cryptographic key it is also the mechanism by which all traffic is always
/// encrypted for a specific endpoint only.
///
/// This is equivalent to [`PublicKey`]. By convention we use `PublicKey`
/// as type name when performing cryptographic operations, but use `EndpointId`
/// when referencing an endpoint.
pub type EndpointId = PublicKey;
```

### Key Properties

| Property | Description |
|----------|-------------|
| **Globally Unique** | Generated from cryptographically secure randomness |
| **Self-Certifying** | Ownership proven via digital signatures |
| **Location-Independent** | Doesn't encode IP address or physical location |
| **Persistent** | Remains constant across network changes |
| **Verifiable** | Anyone can verify signatures without additional infrastructure |

### Why Use Public Keys as IDs?

```
Traditional Approach:
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Node ID       │────>│  Public Key      │────>│  IP Address     │
│  (random UUID)  │     │  (separate)      │     │  (changes)      │
└─────────────────┘     └──────────────────┘     └─────────────────┘
        │                       │                        │
        └───────────────────────┴────────────────────────┘
                    Need to maintain mappings

Iroh Approach:
┌─────────────────────────────────────────────────────────┐
│              EndpointId = PublicKey                      │
│                                                          │
│  One identifier serves all purposes:                     │
│  • Identity (who you are)                               │
│  • Authentication (proving who you are)                 │
│  • Encryption (secure communication)                    │
└─────────────────────────────────────────────────────────┘
```

---

## EndpointId Structure

### Internal Representation

```rust
// From iroh-base/src/key.rs

/// A public key (Ed25519)
///
/// The key itself is stored as the `CompressedEdwardsY` y coordinate
/// of the public key. It is verified to decompress into a valid key
/// when created.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PublicKey(CompressedEdwardsY);

impl Hash for PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Ord for PublicKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.as_bytes().cmp(other.0.as_bytes())
    }
}
```

### Memory Layout

```
EndpointId (32 bytes):
┌──────────────────────────────────────────────────────────┐
│  Compressed Edwards-y Point (32 bytes / 256 bits)        │
│                                                           │
│  Format:                                                 │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Y coordinate (255 bits) | Sign bit (1 bit)      │    │
│  └─────────────────────────────────────────────────┘    │
│                                                           │
│  This uniquely identifies a point on the Ed25519 curve   │
└──────────────────────────────────────────────────────────┘
```

### Type Conversions

```rust
// EndpointId is transparently convertible to/from PublicKey
impl From<EndpointId> for PublicKey {
    fn from(id: EndpointId) -> PublicKey {
        id  // Zero-cost conversion (same type)
    }
}

// Also implements standard traits for flexibility
impl Deref for PublicKey {
    type Target = [u8; 32];
    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Borrow<[u8; 32]> for PublicKey {
    fn borrow(&self) -> &[u8; 32] {
        self.as_bytes()
    }
}
```

---

## From Public Key to EndpointId

### Key Generation Flow

```rust
use iroh::{SecretKey, PublicKey, EndpointId};

// Step 1: Generate a new secret key
let secret_key = SecretKey::generate(&mut rand::rng());

// Step 2: Derive the public key (this IS the EndpointId)
let public_key: PublicKey = secret_key.public();
let endpoint_id: EndpointId = public_key;  // Type alias

// Step 3: Use the EndpointId
println!("My EndpointId: {}", endpoint_id);
// Example: ae58ff8833241ac82d6ff7611046ed67b5072d142c588d0063e942d9a75502b6

// Step 4: The EndpointId can be used for:
// - Identification
// - Signature verification
// - TLS authentication
// - Address lookup key
```

### Code Flow Diagram

```
SecretKey::generate()
        │
        ▼
  [Random 256-bit seed]
        │
        ▼
  [Ed25519 Key Derivation]
        │
        ├─────────────────┐
        ▼                 ▼
   Secret Key        Public Key
   (32 bytes)        (32 bytes)
                         │
                         ▼
                    EndpointId
                    (type alias)
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
   Display as       Serialize       Use for
   hex/base32       to bytes        signatures
```

---

## EndpointAddr: Making IDs Addressable

### The Problem

An EndpointId tells you WHO to connect to, but not HOW:

```
EndpointId: ae58ff8833241ac82d6ff7611046ed67...
            │
            │  I know the identity, but...
            ▼
    How do I reach them?
    - IP address? (may change)
    - Behind NAT?
    - Need a relay?
```

### The Solution: EndpointAddr

```rust
// From iroh-base/src/endpoint_addr.rs

/// Network-level addressing information for an iroh endpoint.
///
/// This combines an endpoint's identifier with network-level
/// addressing information of how to contact the endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndpointAddr {
    /// The endpoint's identifier
    pub id: EndpointId,

    /// The endpoint's addresses (how to reach them)
    pub addrs: BTreeSet<TransportAddr>,
}

/// Available address types
pub enum TransportAddr {
    /// Via relay server
    Relay(RelayUrl),

    /// Direct IP address
    Ip(SocketAddr),

    /// Custom transport address
    Custom(CustomAddr),
}
```

### Building EndpointAddr

```rust
use iroh_base::{EndpointAddr, EndpointId, RelayUrl, TransportAddr};
use std::net::SocketAddr;

// Start with just the ID (empty address)
let addr = EndpointAddr::new(endpoint_id);

// Add a relay URL
let addr = addr.with_relay_url(
    RelayUrl::from_str("https://relay.example.com")?
);

// Add direct IP addresses
let addr = addr
    .with_ip_addr("192.168.1.100:4567".parse()?)
    .with_ip_addr("[2001:db8::1]:4567".parse()?);

// Or from parts
let addr = EndpointAddr::from_parts(
    endpoint_id,
    vec![
        TransportAddr::Relay(relay_url),
        TransportAddr::Ip(socket_addr),
    ]
);
```

### Address Components

```
EndpointAddr:
┌─────────────────────────────────────────────────────────┐
│  id: EndpointId                                         │
│      ae58ff8833241ac82d6ff7611046ed67...               │
├─────────────────────────────────────────────────────────┤
│  addrs: BTreeSet<TransportAddr>                         │
│      ┌─────────────────────────────────────────────┐   │
│      │ TransportAddr::Relay(                       │   │
│      │   "https://relay.example.com"               │   │
│      │ )                                            │   │
│      ├─────────────────────────────────────────────┤   │
│      │ TransportAddr::Ip(                          │   │
│      │   192.168.1.100:4567                        │   │
│      │ )                                            │   │
│      ├─────────────────────────────────────────────┤   │
│      │ TransportAddr::Ip(                          │   │
│      │   [2001:db8::1]:4567                        │   │
│      │ )                                            │   │
│      └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

---

## Address Resolution Flow

### Complete Lookup Process

```rust
use iroh::{Endpoint, address_lookup};

async fn connect_to_peer(endpoint: &Endpoint, peer_id: EndpointId) {
    // Step 1: Application requests connection
    // endpoint.connect(peer_id)

    // Step 2: Endpoint checks local cache
    let cached = endpoint.remote_info(peer_id);

    // Step 3: If not cached, query address lookup services
    if cached.is_none() {
        // Parallel queries to all configured lookup services
        let lookup = endpoint.address_lookup();

        // Query DNS
        let dns_lookup = lookup.resolve(peer_id);

        // Query mDNS (local network)
        #[cfg(feature = "address-lookup-mdns")]
        let mdns_lookup = lookup.resolve(peer_id);

        // Query Pkarr relay
        let pkarr_lookup = lookup.resolve(peer_id);

        // Wait for first response
        let addr = futures::future::select_all([
            dns_lookup,
            mdns_lookup,
            pkarr_lookup,
        ]).await;
    }

    // Step 4: Use resolved address to connect
    let conn = endpoint.connect(peer_id, ALPN).await?;
}
```

### Resolution Flow Diagram

```
                    Application calls:
                    endpoint.connect(peer_id)
                              │
                              ▼
                    ┌─────────────────┐
                    │  Check Local    │
                    │  RemoteMap      │
                    └────────┬────────┘
                             │
              ┌──────────────┴──────────────┐
              │ Has address?                │
              │                              │
         Yes  │                      No      │
         │    │                       │      │
         │    ▼                       ▼      │
         │    │              ┌─────────────────────┐
         │    │              │ Address Lookup      │
         │    │              │ (Parallel queries)  │
         │    │              └──────────┬──────────┘
         │    │                         │
         │    │         ┌───────────────┼───────────────┐
         │    │         │               │               │
         │    │         ▼               ▼               ▼
         │    │   ┌──────────┐  ┌──────────┐  ┌──────────┐
         │    │   │   DNS    │  │  mDNS    │  │  Pkarr   │
         │    │   │ Lookup   │  │  Local   │  │  Relay   │
         │    │   └────┬─────┘  └────┬─────┘  └────┬─────┘
         │    │         │               │               │
         │    │         └───────────────┴───────────────┘
         │    │                         │
         │    │              ┌──────────▼──────────┐
         │    │              │ Merge & Dedupe      │
         │    │              │ All Results         │
         │    │              └──────────┬──────────┘
         │    │                         │
         │    ▼                         ▼
         │    └────────────┬────────────┘
         │                 │
         ▼                 ▼
    ┌─────────────────────────────────┐
    │  Attempt Connection             │
    │  - Direct UDP (if available)    │
    │  - Relay (fallback)             │
    │  - Hole punching                │
    └─────────────────────────────────┘
```

### Pkarr Resolution

```rust
// From iroh/src/address_lookup/pkarr.rs

/// Resolve endpoint info from a Pkarr relay
async fn resolve_from_pkarr(
    pkarr_relay: &str,
    node_id: &EndpointId,
) -> Result<EndpointInfo, PkarrError> {
    // Convert node_id to Pkarr public key
    let pkarr_key = pkarr::PublicKey::from_bytes(node_id.as_bytes())?;

    // Create Pkarr client
    let client = pkarr::Client::new(pkarr_relay)?;

    // Resolve signed packet
    let signed_packet = client.resolve(&pkarr_key).await?;

    // Parse DNS records from packet
    let mut endpoint_info = EndpointInfo::default();

    for record in signed_packet.answer().records {
        if let Some(txt) = record.data.txt() {
            // Parse TXT record containing encoded endpoint info
            let info = EndpointInfo::decode(txt)?;
            endpoint_info = info;
        }
    }

    Ok(endpoint_info)
}
```

---

## Connection Establishment

### Connection Flow with EndpointId

```rust
use iroh::{Endpoint, SecretKey};

async fn establish_connection() -> Result<(), Box<dyn Error>> {
    // 1. Create local endpoint
    let secret_key = SecretKey::generate(&mut rand::rng());
    let my_id = secret_key.public();

    let endpoint = Endpoint::builder()
        .secret_key(secret_key)
        .bind()
        .await?;

    println!("My EndpointId: {}", my_id);

    // 2. Connect to remote endpoint (by ID only)
    let peer_id: EndpointId = "ae58ff883324..."
        .parse()?;

    // The endpoint will:
    // - Look up peer's address (relay, direct, etc.)
    // - Attempt direct connection first
    // - Fall back to relay if needed
    // - Perform TLS handshake with peer's public key
    let conn = endpoint.connect(peer_id, b"my-alpn").await?;

    // 3. Use the connection
    let (mut send, mut recv) = conn.open_bi().await?;
    send.write_all(b"Hello!").await?;

    Ok(())
}
```

### Handshake Verification

```rust
// From iroh/src/tls/verifier.rs

/// During TLS handshake, verify the peer's identity

fn verify_peer_identity(
    expected_id: &EndpointId,
    certificate: &CertificateDer,
) -> Result<(), ConnectionError> {
    // Extract public key from certificate
    let peer_key = extract_public_key(certificate)?;

    // Verify it matches the expected EndpointId
    if peer_key != *expected_id {
        return Err(ConnectionError::IdentityMismatch {
            expected: *expected_id,
            got: peer_key,
        });
    }

    // Verify signature (proves they own the private key)
    Ok(())
}
```

---

## ID Encoding and Display

### Encoding Formats

```rust
use iroh::{PublicKey, SecretKey};
use data_encoding::{HEXLOWER, BASE32_NOPAD};

let secret = SecretKey::generate(&mut rand::rng());
let public = secret.public();
let bytes = public.as_bytes();  // [u8; 32]

// Hex encoding (64 characters)
let hex = HEXLOWER.encode(bytes);
// "ae58ff8833241ac82d6ff7611046ed67b5072d142c588d0063e942d9a75502b6"

// Base32 encoding (52 characters, case-insensitive)
let base32 = BASE32_NOPAD.encode(bytes);
// "VIXOGF2K7N6ZAY3QMRCFKSVX..."

// Short format (first 5 bytes = 10 hex chars)
let short = public.fmt_short();
// "ae58ff8833"
```

### Display Implementation

```rust
// From iroh-base/src/key.rs

impl Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", data_encoding::HEXLOWER.encode(self.as_bytes()))
    }
}

impl Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublicKey({})", HEXLOWER.encode(self.as_bytes()))
    }
}

/// Short display for UI (first 5 bytes)
struct PublicKeyShort([u8; 5]);

impl Display for PublicKeyShort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        data_encoding::HEXLOWER.encode_write(&self.0, f)
    }
}
```

### Parsing from Strings

```rust
use std::str::FromStr;

// Parse from hex
let id: EndpointId = FromStr::from_str(
    "ae58ff8833241ac82d6ff7611046ed67b5072d142c588d0063e942d9a75502b6"
)?;

// Parse from base32
let id: EndpointId = FromStr::from_str(
    "VIXOGF2K7N6ZAY3QMRCFKSVX..."
)?;

// Both formats are automatically detected
fn parse_endpoint_id(s: &str) -> Result<EndpointId, KeyParsingError> {
    let mut bytes = [0u8; 32];

    let result = if s.len() == 64 {
        // Hex (32 bytes * 2 chars)
        HEXLOWER.decode_mut(s.as_bytes(), &mut bytes)
    } else {
        // Base32
        let input = s.to_ascii_uppercase();
        BASE32_NOPAD.decode_mut(input.as_bytes(), &mut bytes)
    };

    match result {
        Ok(32) => Ok(PublicKey::from_bytes(&bytes)?),
        _ => Err(KeyParsingError::DecodeInvalidLength),
    }
}
```

---

## Practical Examples

### Example 1: Peer Discovery

```rust
use iroh::{Endpoint, SecretKey, address_lookup::PkarrPublisher};

async fn publish_and_discover() -> Result<(), Box<dyn Error>> {
    // Publisher node
    let secret = SecretKey::generate(&mut rand::rng());
    let my_id = secret.public();

    let publisher = Endpoint::builder()
        .secret_key(secret)
        .address_lookup(PkarrPublisher::n0_dns())
        .bind()
        .await?;

    println!("Publisher ID: {}", my_id);
    // Share this ID with the peer (via QR code, etc.)

    // --- Later, on subscriber node ---

    let subscriber_secret = SecretKey::generate(&mut rand::rng());
    let subscriber = Endpoint::builder()
        .secret_key(subscriber_secret)
        .address_lookup(address_lookup::DnsAddressLookup::n0_dns())
        .bind()
        .await?;

    // Parse the ID you received
    let publisher_id: EndpointId = my_id.to_string().parse()?;

    // Connect (address lookup happens automatically)
    let conn = subscriber.connect(publisher_id, b"my-protocol").await?;

    Ok(())
}
```

### Example 2: Known Peer List

```rust
use std::collections::HashMap;
use iroh::EndpointId;

struct PeerManager {
    known_peers: HashMap<String, EndpointId>,  // name -> ID
}

impl PeerManager {
    fn new() -> Self {
        let mut peers = HashMap::new();

        // Pre-configure known peers
        peers.insert(
            "alice".to_string(),
            "ae58ff8833241ac82d6ff7611046ed67..."
                .parse()
                .unwrap(),
        );
        peers.insert(
            "bob".to_string(),
            "c93b1a0e5f4d8c2b7e9a1f3d5c7b9e0a..."
                .parse()
                .unwrap(),
        );

        Self { known_peers: peers }
    }

    async fn connect_to_peer(
        &self,
        endpoint: &Endpoint,
        name: &str,
    ) -> Result<Connection, Error> {
        let peer_id = self.known_peers
            .get(name)
            .ok_or(Error::UnknownPeer)?;

        endpoint.connect(*peer_id, b"my-alpn").await
    }
}
```

### Example 3: QR Code Sharing

```rust
use qrcode::QrCode;
use iroh::EndpointId;

/// Generate QR code for sharing EndpointId
fn generate_share_qr(endpoint_id: EndpointId) -> QrCode {
    // Format: iroh:<endpoint_id>
    let uri = format!("iroh:{}", endpoint_id);
    QrCode::new(uri).unwrap()
}

/// Parse EndpointId from scanned QR
fn parse_scanned_qr(content: &str) -> Result<EndpointId, QrParseError> {
    if let Some(id_str) = content.strip_prefix("iroh:") {
        Ok(id_str.parse()?)
    } else {
        // Maybe raw hex/base32
        Ok(content.parse()?)
    }
}
```

### Example 4: Node Fingerprint

```rust
/// Display a human-readable fingerprint
struct NodeFingerprint {
    id: EndpointId,
}

impl NodeFingerprint {
    fn display_lines(&self) -> Vec<String> {
        let hex = self.id.to_string();

        // Format as 4 lines of 16 characters (like SSH fingerprints)
        vec![
            format!("{} {}", &hex[0..8], &hex[8..16]),
            format!("{} {}", &hex[16..24], &hex[24..32]),
            format!("{} {}", &hex[32..40], &hex[40..48]),
            format!("{} {}", &hex[48..56], &hex[56..64]),
        ]
    }

    fn emoji_fingerprint(&self) -> String {
        // Convert bytes to emoji for visual verification
        // (Similar to Signal's safety number verification)
        let bytes = self.id.as_bytes();
        let mut emoji = String::new();

        const EMOJI: [&str; 256] = [/* emoji list */];

        for byte in bytes.iter().take(16) {
            emoji.push_str(EMOJI[*byte as usize]);
            emoji.push(' ');
        }

        emoji
    }
}

// Example output:
// ae58ff88 33241ac8
// 2d6ff761 1046ed67
// b5072d14 2c588d00
// 63e942d9 a75502b6
//
// Or: 🍎 🚗 🌙 📱 ...
```

---

## ID Best Practices

### DO: Use type-safe handling

```rust
// ✅ Good: Use EndpointId type
fn connect(peer: EndpointId) { ... }

// ❌ Bad: Use raw strings
fn connect(peer: String) { ... }  // No validation!
```

### DO: Validate on input

```rust
// ✅ Validate when receiving from user/network
let peer_id: EndpointId = input.parse()
    .map_err(|e| InputError::InvalidEndpointId(e))?;
```

### DO: Use short format for display

```rust
// ✅ User-friendly display
println!("Connected to: {}", peer_id.fmt_short());
// "ae58ff8833"

// ❌ Don't show full ID in casual contexts
println!("Connected to: {}", peer_id);
// Full 64-char hex is hard to read/verify
```

### DON'T: Hardcode IDs without validation

```rust
// ❌ This will panic if invalid
let id: EndpointId = "invalid".parse().unwrap();

// ✅ Handle errors gracefully
let id: EndpointId = match input.parse() {
    Ok(id) => id,
    Err(e) => return Err(Error::InvalidPeerId(e)),
};
```

---

## See Also

- [exploration.md](./exploration.md) - Main iroh exploration
- [cryptography-keys.md](./cryptography-keys.md) - Cryptographic details
- [nat-traversal.md](./nat-traversal.md) - NAT traversal
