# Ghostport - Port Spoofing Tool

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/ghostport/`

---

## Overview

**Ghostport** is a sophisticated port spoofing tool designed to confuse and mislead port scanners. It responds to connection attempts with fake service signatures, making it appear as if various services are running on scanned ports when they are not.

### What It Does

1. **Listens on a configurable port** (default: 8888)
2. **Accepts TCP connections** from port scanners
3. **Responds with fake service banners** (SSH, HTTP, FTP, etc.)
4. **Supports regex-based signatures** for dynamic responses
5. **Confuses reconnaissance** by making all ports appear "open"

### Use Cases

- **Honeypot deployment** - Mislead attackers about running services
- **Network deception** - Increase noise during security assessments
- **Privacy protection** - Hide which ports are actually in use
- **Security research** - Study attacker behavior against deceptive systems

---

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                      Ghostport                               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │     CLI     │    │  Signature  │    │    Tokio    │     │
│  │   Parser    │    │   Parser    │    │   Runtime   │     │
│  │   (clap)    │    │             │    │             │     │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘     │
│         │                  │                  │             │
│         │                  │                  │             │
│         ▼                  ▼                  ▼             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              TCP Listener (Tokio)                     │  │
│  │                                                        │  │
│  │  ┌─────────────────────────────────────────────────┐ │  │
│  │  │           Connection Handler (spawned)          │ │  │
│  │  │   - Accept connection                           │ │  │
│  │  │   - Select random signature                     │ │  │
│  │  │   - Generate payload (regex or raw)             │ │  │
│  │  │   - Write response                              │ │  │
│  │  │   - Close connection                            │ │  │
│  │  └─────────────────────────────────────────────────┘ │  │
│  │                                                        │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Module Structure

```
src/
├── main.rs           # Entry point, Tokio runtime, TCP listener
├── cli.rs            # Command-line argument parsing (clap)
└── handler.rs        # Signature parsing, payload generation
```

---

## Implementation Details

### 1. Main Entry Point

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI flags
    let cli = Cli::parse();

    // Setup tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_max_level(if cli.debug {
            Level::DEBUG
        } else if cli.verbose {
            Level::INFO
        } else {
            Level::ERROR
        })
        .without_time()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // Load signatures from file
    let signatures = parse_signatures(&cli.signatures)?;
    debug!("Read {} signatures", signatures.len());

    // Bind TCP listener
    let listener = TcpListener::bind(&cli.listen).await?;
    info!("Started listener on {}", cli.listen);

    // Accept loop
    loop {
        let (mut stream, address) = listener.accept().await?;
        debug!("Accepted connection from {}", address);

        let sigs = signatures.clone();
        let cli_clone = cli.clone();

        // Spawn async task for each connection
        tokio::spawn(async move {
            let signature = sigs.choose(&mut rand::thread_rng());

            if let Some(sig) = signature {
                let payload = generate_payload(sig);

                match stream.write_all(&payload).await {
                    Ok(()) => {
                        debug!("Sent payload to {}: {:?}", address, payload);
                        if cli_clone.verbose {
                            info!("Sent payload ({} bytes) to {}",
                                  payload.len(), address);
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::ConnectionReset {
                            debug!("Connection reset by peer: {}", address);
                        } else {
                            error!("Failed to write payload to {}: {}",
                                   address, e);
                        }
                    }
                }
            }
        });
    }
}
```

### 2. CLI Argument Parsing

```rust
// src/cli.rs
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author = "perp and sad")]
pub struct Cli {
    #[arg(
        short = 's',
        long = "signatures",
        value_name = "FILE",
        help = "Path to the signatures",
        default_value = "signatures"
    )]
    pub signatures: String,

    #[arg(
        short = 'l',
        long = "listen",
        value_name = "ADDRESS",
        help = "Address to listen on",
        default_value = "127.0.0.1:8888"
    )]
    pub listen: String,

    #[arg(short = 'd', long = "debug", help = "Enable debug logging")]
    pub debug: bool,

    #[arg(short = 'v', long = "verbose", help = "Enable verbose logging")]
    pub verbose: bool,

    #[arg(short = 'q', long = "quiet", help = "Enable quiet logging")]
    pub quiet: bool,

    #[arg(short = 'V', long = "version", help = "Print version information")]
    pub version: bool,
}
```

### 3. Signature Parser

```rust
// src/handler.rs
use std::fs::File;
use std::io::{BufRead, BufReader};
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub enum Payload {
    Raw(Vec<u8>),
    Regex(String),
}

#[derive(Debug, Clone)]
pub struct Signature {
    pub payload: Payload,
}

pub fn parse_signatures(file_path: &str) -> Result<Vec<Signature>> {
    let file = File::open(file_path)
        .context("Failed to open signatures file")?;
    let reader = BufReader::new(file);
    let mut signatures = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line.context("Failed to read line")?;
        if line.trim().is_empty() {
            continue;
        }

        // Detect regex patterns by presence of ( ) characters
        let payload = if line.contains('(') && line.contains(')') {
            Payload::Regex(line)
        } else {
            Payload::Raw(unescape_string(&line)?)
        };

        signatures.push(Signature { payload });
    }

    Ok(signatures)
}
```

### 4. String Unescaping

```rust
fn unescape_string(s: &str) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('x') => {
                    // Hex escape: \x41 = 'A'
                    let hex: String = chars.by_ref().take(2).collect();
                    if hex.len() == 2 {
                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                            result.push(byte);
                        } else {
                            result.extend(b"\\x");
                            result.extend(hex.bytes());
                        }
                    }
                },
                Some('0') => result.push(0),      // Null byte
                Some('n') => result.push(b'\n'),  // Newline
                Some('r') => result.push(b'\r'),  // Carriage return
                Some('t') => result.push(b'\t'),  // Tab
                Some(c) => result.push(c as u8),
                None => result.push(b'\\'),
            }
        } else {
            result.push(c as u8);
        }
    }

    Ok(result)
}
```

### 5. Payload Generation (Regex Mode)

```rust
pub fn generate_payload(signature: &Signature) -> Vec<u8> {
    match &signature.payload {
        Payload::Raw(v) => v.clone(),
        Payload::Regex(r) => generate_regex_match(r),
    }
}

fn generate_regex_match(regex_str: &str) -> Vec<u8> {
    let mut result = String::new();
    let mut chars = regex_str.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(next_char) = chars.next() {
                    match next_char {
                        'd' => {
                            // Digit 0-9
                            result.push(
                                rand::thread_rng().gen_range(b'0'..=b'9') as char
                            );
                        }
                        'w' => {
                            // Word character a-z
                            result.push(
                                rand::thread_rng().gen_range(b'a'..=b'z') as char
                            );
                        }
                        'x' => {
                            // Hex byte
                            let hex = chars.next()
                                .and_then(|c1| chars.next()
                                    .map(|c2| format!("{}{}", c1, c2)))
                                .unwrap_or_else(|| "00".to_string());
                            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                                result.push(byte as char);
                            }
                        }
                        _ => result.push(next_char),
                    }
                }
            },
            '[' => {
                // Character class: [abc] -> pick one
                let mut class = String::new();
                for class_char in chars.by_ref() {
                    if class_char == ']' { break; }
                    class.push(class_char);
                }
                if !class.is_empty() {
                    result.push(
                        class.chars()
                            .nth(rand::thread_rng()
                                .gen_range(0..class.len()))
                            .unwrap()
                    );
                }
            },
            '(' => {
                // Skip grouped alternatives (simplified)
                let mut depth = 1;
                for group_char in chars.by_ref() {
                    if group_char == '(' { depth += 1; }
                    if group_char == ')' { depth -= 1; }
                    if depth == 0 { break; }
                }
            },
            '+' | '*' => {
                // Repeat previous character 0-4 times
                if let Some(last_char) = result.chars().last() {
                    let repeat = rand::thread_rng().gen_range(0..5);
                    for _ in 0..repeat {
                        result.push(last_char);
                    }
                }
            },
            '.' => {
                // Any printable character
                result.push(
                    rand::thread_rng().gen_range(b'!'..=b'~') as char
                );
            },
            _ => result.push(c),
        }
    }

    result.into_bytes()
}
```

---

## Signature File Format

### Raw Signatures

Simple byte-for-byte responses:

```
HTTP/1.1 200 OK\r\nServer: Apache/2.4.41 (Unix)\r\n
SSH-2.0-OpenSSH_8.2p1 Ubuntu-4ubuntu0.1
220 (vsFTPd 3.0.3)
```

### Regex Signatures

Dynamic responses with pattern matching:

```
220 mail\d+\.example\.com ESMTP
SSH-2.0-OpenSSH[\d.]+
HTTP/1.1 200 OK\r\nServer: nginx/[\d.]+\r\n
```

### Regex Syntax Supported

| Pattern | Description | Example Output |
|---------|-------------|----------------|
| `\d` | Random digit | `mail5.example.com` |
| `\w` | Random letter a-z | `abc123` |
| `\xNN` | Hex byte | `\x41` → `A` |
| `[abc]` | One character from set | `a`, `b`, or `c` |
| `.` | Any printable char | Random `!` to `~` |
| `+` | Repeat 0-4 times | `aaa`, `aaaa` |
| `*` | Repeat 0-4 times | Same as `+` |

---

## Dependencies

```toml
[package]
name = "ghostport"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0.72"
clap = { version = "4.3.19", features = ["derive"] }
rand = "0.8.5"
regex = "1.11.0"
tokio = { version = "1", features = ["full"] }
tracing = "0.1.37"
tracing-subscriber = "0.3.17"
```

### Key Crates

| Crate | Purpose |
|-------|---------|
| **tokio** | Async TCP listener and connection handling |
| **clap** | Command-line argument parsing with derive macros |
| **tracing** | Structured logging |
| **rand** | Random signature selection and regex matching |
| **anyhow** | Flexible error handling |

---

## Usage

### Building

```bash
git clone https://github.com/vxfemboy/ghostport.git
cd ghostport
cargo build --release
```

### Basic Usage

```bash
./target/release/ghostport -s signatures.txt
```

### Command-Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-s, --signatures <FILE>` | Path to signatures file | `signatures` |
| `-l, --listen <ADDRESS>` | Address to listen on | `127.0.0.1:8888` |
| `-d, --debug` | Enable debug logging | - |
| `-v, --verbose` | Enable verbose logging | - |
| `-q, --quiet` | Enable quiet logging | - |
| `-V, --version` | Print version | - |

### Examples

**Run with custom address and verbose logging:**
```bash
./target/release/ghostport -s signatures.txt -l 0.0.0.0:8888 -v
```

**Run with debug logging:**
```bash
./target/release/ghostport -s signatures.txt -l 0.0.0.0:8888 -d
```

**Run with Cargo:**
```bash
cargo run -- -s signatures.txt
```

---

## Integration with iptables

To redirect all incoming TCP traffic to Ghostport:

```bash
INTERFACE="eth0"  # Change to your interface

# Add REDIRECT rule
iptables -t nat -A PREROUTING -i $INTERFACE -p tcp -m tcp \
    -m multiport --dports 1:65535 -j REDIRECT --to-ports 8888

# Remove rule later
iptables -t nat -D PREROUTING -i $INTERFACE -p tcp -m tcp \
    -m multiport --dports 1:65535 -j REDIRECT --to-ports 8888
```

**Warning:** This affects ALL incoming TCP connections. Use with caution on production systems.

---

## How It Defends Against Port Scanners

### Normal Port Scan Behavior

```
Nmap Scan Report:
PORT      STATE    SERVICE
22/tcp    open     ssh
80/tcp    open     http
443/tcp   open     https
8080/tcp  closed   http-proxy
```

### With Ghostport Active

```
Nmap Scan Report:
PORT      STATE    SERVICE
1/tcp     open     tcpmux
22/tcp    open     ssh (fake)
80/tcp    open     http (fake)
443/tcp   open     https (fake)
8080/tcp  open     http-proxy (fake)
65535/tcp open     unknown (fake)

ALL PORTS APPEAR OPEN!
```

### Scanner Confusion

1. **Increased scan time** - Scanner waits for responses on all ports
2. **False positives** - Security teams investigate non-existent services
3. **Resource exhaustion** - Scanner consumes more bandwidth/CPU
4. **Cover traffic** - Real services hidden among fake ones

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_regex_match() {
        let regex_str = r"Hello [\w]+, your lucky number is \d+";
        let result = generate_regex_match(regex_str);
        let result_str = String::from_utf8_lossy(&result);

        assert!(result_str.starts_with("Hello "));
        assert!(result_str.contains(", your lucky number is "));
    }

    #[test]
    fn test_unescape_string() {
        assert_eq!(unescape_string(r"Hello\nWorld").unwrap(),
                   b"Hello\nWorld");
        assert_eq!(unescape_string(r"Test\x41\x42\x43").unwrap(),
                   b"TestABC");
        assert_eq!(unescape_string(r"\0\r\n\t").unwrap(),
                   b"\0\r\n\t");
    }
}
```

---

## Comparison to Similar Tools

### Ghostport vs Portspoof

| Feature | Ghostport | Portspoof |
|---------|-----------|-----------|
| Language | Rust | C |
| Async | Yes (Tokio) | No (fork-based) |
| Regex Support | Yes | Limited |
| Configuration | File-based | Compile-time |
| Logging | Structured (tracing) | printf |
| Memory Safety | Yes | Manual |

---

## Files

- **Main Entry:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/ghostport/src/main.rs`
- **CLI Parser:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/ghostport/src/cli.rs`
- **Handler:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/ghostport/src/handler.rs`
- **Cargo.toml:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/ghostport/Cargo.toml`
- **Signatures:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/ghostport/signatures.txt`
- **Documentation:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/ghostport/README.md`

---

## Summary

Ghostport demonstrates:

1. **Async TCP handling** with Tokio's spawn-per-connection model
2. **Clap derive macros** for elegant CLI parsing
3. **Tracing subscriber** for configurable logging levels
4. **Regex-based payload generation** for dynamic responses
5. **Proper error handling** with anyhow context

It's an excellent example of building security tools in Rust that leverage async I/O for high performance.
