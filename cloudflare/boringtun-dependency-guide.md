---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/boringtun
repository: https://github.com/cloudflare/boringtun
created: 2026-03-19
---

# Using BoringTun as a Rust Dependency

## Quick Start

Add boringtun to your `Cargo.toml`:

```toml
[dependencies]
boringtun = "0.6"
```

For the full device implementation (userspace WireGuard):

```toml
[dependencies]
boringtun = { version = "0.6", features = ["device"] }
```

## Minimal Example: Point-to-Point Tunnel

```rust
use boringtun::noise::Tunn;
use boringtun::x25519::{StaticSecret, PublicKey};
use rand_core::OsRng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate keys (in production, load from config)
    let my_secret = StaticSecret::random_from_rng(OsRng);
    let my_public = PublicKey::from(&my_secret);

    let peer_secret = StaticSecret::random_from_rng(OsRng);
    let peer_public = PublicKey::from(&peer_secret);

    // Create tunnel (call once per peer)
    let mut tunnel = Tunn::new(
        my_secret,
        peer_public,
        None,           // Optional preshared key
        None,           // Optional persistent keepalive
        0,              // Peer index
        None,           // Optional rate limiter
    );

    // Buffer for encrypted packets
    let mut buffer = vec![0u8; 65535];

    // === Sending IP packets ===
    let ip_packet = &[0x45, 0x00, 0x00, 0x3c, /* ... IPv4 packet ... */];

    match tunnel.encapsulate(ip_packet, &mut buffer) {
        boringtun::noise::TunnResult::Done => {
            // No session yet, handshake in progress
        }
        boringtun::noise::TunnResult::WriteToNetwork(udp_payload) => {
            // Send udp_payload to peer via UDP
            println!("Sending {} bytes to peer", udp_payload.len());
        }
        boringtun::noise::TunnResult::Err(e) => {
            eprintln!("Encapsulation error: {:?}", e);
        }
        _ => unreachable!(),
    }

    // === Receiving UDP packets ===
    let udp_payload = /* receive from UDP socket */ &[0u8; 0];
    let src_addr = None; // Optional source IP for rate limiting

    match tunnel.decapsulate(src_addr, udp_payload, &mut buffer) {
        boringtun::noise::TunnResult::Done => {
            // Keepalive or no data
        }
        boringtun::noise::TunnResult::WriteToTunnelV4(ip_packet, src_ip) => {
            // Write ip_packet to TUN interface (IPv4)
            println!("Received IPv4 packet from {}", src_ip);
        }
        boringtun::noise::TunnResult::WriteToTunnelV6(ip_packet, src_ip) => {
            // Write ip_packet to TUN interface (IPv6)
            println!("Received IPv6 packet from {}", src_ip);
        }
        boringtun::noise::TunnResult::WriteToNetwork(response) => {
            // Send response back (handshake response or cookie)
        }
        boringtun::noise::TunnResult::Err(e) => {
            eprintln!("Decapsulation error: {:?}", e);
        }
    }

    Ok(())
}
```

## API Reference

### Tunn::new

```rust
pub fn new(
    static_private: StaticSecret,
    peer_static_public: PublicKey,
    preshared_key: Option<[u8; 32]>,
    persistent_keepalive: Option<u16>,
    index: u32,
    rate_limiter: Option<Arc<RateLimiter>>,
) -> Self
```

| Parameter | Description |
|-----------|-------------|
| `static_private` | Your X25519 private key |
| `peer_static_public` | Peer's X25519 public key |
| `preshared_key` | Optional symmetric key for additional security |
| `persistent_keepalive` | Seconds between keepalives (0 to disable) |
| `index` | Unique peer identifier (24-bit, use LFSR for obfuscation) |
| `rate_limiter` | Optional handshake rate limiter |

### TunnResult Enum

```rust
pub enum TunnResult<'a> {
    /// No action required
    Done,

    /// An error occurred
    Err(WireGuardError),

    /// Send buffer to network (UDP payload)
    WriteToNetwork(&'a mut [u8]),

    /// Write to TUN as IPv4 packet
    WriteToTunnelV4(&'a mut [u8], Ipv4Addr),

    /// Write to TUN as IPv6 packet
    WriteToTunnelV6(&'a mut [u8], Ipv6Addr),
}
```

### Core Methods

#### encapsulate

```rust
pub fn encapsulate<'a>(&mut self, src: &[u8], dst: &'a mut [u8]) -> TunnResult<'a>
```

Encapsulates an IP packet into a WireGuard UDP payload.

- `src`: Raw IP packet (including IP header)
- `dst`: Pre-allocated buffer (must be at least `src.len() + 32` bytes, minimum 148)
- Returns: `TunnResult::WriteToNetwork` with UDP payload on success

#### decapsulate

```rust
pub fn decapsulate<'a>(
    &mut self,
    src_addr: Option<IpAddr>,
    datagram: &[u8],
    dst: &'a mut [u8]
) -> TunnResult<'a>
```

Decapsulates a WireGuard UDP payload into an IP packet.

- `src_addr`: Source IP for rate limiting (can be None)
- `datagram`: Received UDP payload
- `dst`: Pre-allocated buffer for IP packet
- Returns: `TunnResult::WriteToTunnelV4/V6` on success

**Important**: If result is `WriteToNetwork`, repeat call with empty datagram until `Done`.

#### update_timers

```rust
pub fn update_timers<'a>(&mut self, dst: &'a mut [u8]) -> TunnResult<'a>
```

Execute periodic timer functions. Call every 100ms.

```rust
// In your event loop:
std::thread::spawn(|| {
    let mut buffer = vec![0u8; 65535];
    loop {
        std::thread::sleep(Duration::from_millis(100));
        match tunnel.update_timers(&mut buffer) {
            TunnResult::WriteToNetwork(packet) => {
                // Send keepalive or handshake
            }
            _ => {}
        }
    }
});
```

#### stats

```rust
pub fn stats(&self) -> (
    Option<Duration>,  // Time since last handshake
    usize,             // Bytes transmitted
    usize,             // Bytes received
    f32,               // Estimated packet loss
    Option<u32>        // Last RTT in milliseconds
)
```

## Full Example: Userspace WireGuard Device

```rust
use boringtun::device::{DeviceHandle, DeviceConfig};
use boringtun::noise::Tunn;
use std::sync::Arc;
use std::net::UdpSocket;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure device
    let config = DeviceConfig {
        n_threads: 4,
        use_connected_socket: true,
        #[cfg(target_os = "linux")]
        use_multi_queue: true,
        #[cfg(target_os = "linux")]
        uapi_fd: -1,
    };

    // Create device (creates TUN interface)
    let mut device = DeviceHandle::new("utun0", config)?;

    // Note: Use wg-quick or the wg tool to configure peers
    // The device exposes a userspace API compatible with WireGuard tools

    // Wait for device to shutdown
    device.wait();
    device.clean();

    Ok(())
}
```

## Using with wg-quick

BoringTun is compatible with the standard WireGuard userspace tools.

### Method 1: Set environment variable

```bash
WG_QUICK_USERSPACE_IMPLEMENTATION=boringtun-cli wg-quick up wg0
```

### Method 2: Use wg-quick with sudo

```bash
sudo WG_QUICK_USERSPACE_IMPLEMENTATION=boringtun-cli WG_SUDO=1 wg-quick up wg0
```

### Configuration Example

**wg0.conf:**
```ini
[Interface]
PrivateKey = <your base64 private key>
Address = 10.0.0.1/32
DNS = 1.1.1.1

[Peer]
PublicKey = <peer base64 public key>
Endpoint = example.com:51820
AllowedIPs = 0.0.0.0/0
```

## Key Types and Key Management

### Generating Keys

```rust
use boringtun::x25519::{StaticSecret, PublicKey};
use rand_core::OsRng;

// Generate random private key
let secret = StaticSecret::random_from_rng(OsRng);

// Derive public key
let public = PublicKey::from(&secret);

// Convert to bytes for storage
let secret_bytes = secret.to_bytes();
let public_bytes = public.to_bytes();

// Reconstruct from bytes
let secret = StaticSecret::from(secret_bytes);
let public = PublicKey::from(public_bytes);
```

### Base64 Encoding (WireGuard format)

```rust
use base64::{encode, decode};

// WireGuard uses base64 for key representation in config files
let secret_b64 = encode(secret.to_bytes());
let public_b64 = encode(public.to_bytes());

// Decode from config
let decoded = decode(&secret_b64)?;
let bytes: [u8; 32] = decoded.try_into().map_err(|_| "Invalid key length")?;
let secret = StaticSecret::from(bytes);
```

## Error Handling

```rust
use boringtun::noise::errors::WireGuardError;

#[derive(Debug, thiserror::Error)]
enum TunnelError {
    #[error("Handshake timeout")]
    HandshakeTimeout,
    #[error("Invalid packet: {0}")]
    InvalidPacket(#[from] WireGuardError),
}

// WireGuardError variants:
// - InvalidPacket
// - InvalidAeadTag
// - InvalidTai64nTimestamp
// - WrongKey
// - WrongIndex
// - InvalidCounter
// - DuplicateCounter
// - NoCurrentSession
// - ConnectionExpired
// - ...
```

## Performance Best Practices

### 1. Pre-allocate Buffers

```rust
// Allocate once, reuse
let mut send_buf = vec![0u8; 65535];
let mut recv_buf = vec![0u8; 65535];

// In hot loop:
loop {
    match tunnel.encapsulate(&packet, &mut send_buf) {
        TunnResult::WriteToNetwork(data) => send_udp(data),
        _ => {}
    }
}
```

### 2. Call update_timers Regularly

```rust
// Spawn dedicated timer thread
std::thread::spawn(move || {
    loop {
        std::thread::sleep(Duration::from_millis(100));
        let _ = tunnel.update_timers(&mut timer_buf);
    }
});
```

### 3. Use Connected Sockets

After initial handshake, boringtun creates connected UDP sockets per peer for better performance. Enable via `DeviceConfig::use_connected_socket = true`.

### 4. Rate Limit Handshakes

```rust
use boringtun::noise::rate_limiter::RateLimiter;
use boringtun::x25519::PublicKey;
use std::sync::Arc;

let public_key = /* ... */;
let rate_limiter = Arc::new(RateLimiter::new(&public_key, 10)); // 10 handshakes/sec

let tunnel = Tunn::new(
    secret,
    public_key,
    None,
    None,
    0,
    Some(rate_limiter),
);
```

## Platform Support

| Target Triple | Library | CLI (device feature) |
|---------------|---------|---------------------|
| x86_64-unknown-linux-gnu | ✓ | ✓ |
| aarch64-unknown-linux-gnu | ✓ | ✓ |
| armv7-unknown-linux-gnueabihf | ✓ | ✓ |
| x86_64-apple-darwin | ✓ | ✓ |
| x86_64-pc-windows-msvc | ✓ | - |
| aarch64-apple-ios | ✓ | - |
| armv7-apple-ios | ✓ | - |
| aarch64-linux-android | ✓ | - |

## Building

### Library Only

```bash
cargo build --lib --no-default-features --release
```

### With Device Support

```bash
cargo build --features device --release
```

### Cross-Compilation

```bash
# For aarch64 Linux
rustup target add aarch64-unknown-linux-gnu
cargo build --target aarch64-unknown-linux-gnu --features device --release

# For iOS
cargo build --target aarch64-apple-ios --lib --release
```

## Common Pitfalls

### 1. Buffer Too Small

```rust
// WRONG: Buffer might be too small
let mut buf = vec![0u8; 100];
tunnel.encapsulate(&large_packet, &mut buf); // May panic!

// RIGHT: Allocate enough space
let mtu = 1500;
let overhead = 32; // WireGuard overhead
let mut buf = vec![0u8; mtu + overhead];
```

### 2. Not Handling Repeated Calls

```rust
// decapsulate may return WriteToNetwork, requiring repeated calls:
let mut result = tunnel.decapsulate(src_addr, datagram, &mut dst);
while let TunnResult::WriteToNetwork(response) = result {
    send_udp(response);
    result = tunnel.decapsulate(src_addr, &[], &mut dst); // Empty datagram = repeat
}
```

### 3. Forgetting Timers

The WireGuard protocol requires periodic timer calls for:
- Keepalive transmission
- Session rekeying
- Connection expiry detection

```rust
// Must call update_timers every ~100ms or timers drift
```

## Integration with Async Runtimes

### Tokio Example

```rust
use tokio::net::UdpSocket;
use std::sync::{Arc, Mutex};

struct TunnelEndpoint {
    tunnel: Arc<Mutex<Tunn>>,
    socket: UdpSocket,
}

impl TunnelEndpoint {
    async fn send_packet(&self, ip_packet: &[u8]) {
        let mut tunnel = self.tunnel.lock().unwrap();
        let mut buf = vec![0u8; 65535];

        match tunnel.encapsulate(ip_packet, &mut buf) {
            TunnResult::WriteToNetwork(data) => {
                self.socket.send(data).await.unwrap();
            }
            _ => {}
        }
    }

    async fn receive_loop(&self) {
        let mut buf = vec![0u8; 65535];
        let mut out_buf = vec![0u8; 65535];

        loop {
            let (n, addr) = self.socket.recv_from(&mut buf).await.unwrap();
            let mut tunnel = self.tunnel.lock().unwrap();

            match tunnel.decapsulate(Some(addr.ip()), &buf[..n], &mut out_buf) {
                TunnResult::WriteToTunnelV4(packet, _) => {
                    // Handle received packet
                }
                TunnResult::WriteToNetwork(response) => {
                    self.socket.send(response).await.unwrap();
                }
                _ => {}
            }
        }
    }
}
```

## License

BoringTun is licensed under the [BSD-3-Clause](https://opensource.org/licenses/BSD-3-Clause) license.

## References

- [GitHub Repository](https://github.com/cloudflare/boringtun)
- [crates.io](https://crates.io/crates/boringtun)
- [Documentation](https://docs.rs/boringtun)
- [WireGuard Specification](https://www.wireguard.com/)
