# Rust Patterns and Best Practices - vxfemboy Projects

**Analysis of common patterns across the vxfemboy project collection**

---

## Table of Contents

1. [Error Handling Patterns](#error-handling-patterns)
2. [Async Runtime Patterns](#async-runtime-patterns)
3. [CLI Parsing Patterns](#cli-parsing-patterns)
4. [Cryptographic Patterns](#cryptographic-patterns)
5. [File I/O Patterns](#file-io-patterns)
6. [Threading Patterns](#threading-patterns)
7. [Logging Patterns](#logging-patterns)
8. [Security Best Practices](#security-best-practices)
9. [Production Considerations](#production-considerations)

---

## Error Handling Patterns

### 1. Custom Error Types with `thiserror`

Used in: **purrcrypt**, **ghostport**

```rust
// src/keys.rs (purrcrypt)
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid key format: {0}")]
    InvalidKey(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),
}
```

**Why this pattern:**
- `#[from]` automatically implements `From<IOError>` for `KeyError`
- Error messages are templated with `{0}` placeholders
- `Debug` derive enables `{:?}` formatting for debugging

### 2. Error Context with `anyhow`

Used in: **ghostport**

```rust
// src/handler.rs
use anyhow::{Context, Result, anyhow};

pub fn parse_signatures(file_path: &str) -> Result<Vec<Signature>> {
    let file = File::open(file_path)
        .context("Failed to open signatures file")?;

    for (index, line) in reader.lines().enumerate() {
        let line = line.context("Failed to read line from signatures file")?;

        // ...
    }

    if signatures.is_empty() {
        return Err(anyhow!("No valid signatures found in the file"));
    }

    Ok(signatures)
}
```

**Why this pattern:**
- `anyhow::Result<T>` is a flexible error type for applications
- `.context()` adds descriptive messages to errors
- `anyhow!()` macro creates ad-hoc errors

### 3. Error Propagation with `?` Operator

```rust
pub fn save_keys(&self, pub_path: &Path, secret_path: &Path)
    -> Result<(), KeyError>
{
    // ? automatically converts IOError to KeyError via #[from]
    fs::write(pub_path, encoded_pub)?;

    #[cfg(unix)]
    fs::set_permissions(pub_path, fs::Permissions::from_mode(0o644))?;

    Ok(())  // Explicit success return
}
```

---

## Async Runtime Patterns

### 1. Tokio Main Entry Point

Used in: **ghostport**, **spiderirc**

```rust
// src/main.rs (ghostport)
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Bind TCP listener
    let listener = TcpListener::bind(&cli.listen).await?;
    info!("Started listener on {}", cli.listen);

    // Accept loop
    loop {
        let (mut stream, address) = listener.accept().await?;

        // Spawn task per connection
        tokio::spawn(async move {
            // Handle connection
        });
    }
}
```

**Why this pattern:**
- `#[tokio::main]` sets up the async runtime
- `async fn main()` enables `.await` in main
- `tokio::spawn()` creates independent tasks

### 2. Select-Based Event Loop

Used in: **spiderirc**

```rust
use tokio::select;

loop {
    select! {
        // Handle stdin input
        line = stdin.next_line() => {
            if let Ok(Some(line)) = line {
                // Send message
            }
        }

        // Handle incoming messages
        message = response_rcv.recv() => {
            if let Some(message) = message {
                println!("{}", message);
            }
        }

        // Handle swarm events
        event = swarm.select_next_some() => {
            match event {
                SwarmEvent::Behaviour(event) => { /* ... */ }
                _ => {}
            }
        }
    }
}
```

**Why this pattern:**
- `select!` waits for multiple async events simultaneously
- First ready branch executes
- Essential for event-driven applications

### 3. Connection-per-Task Pattern

```rust
loop {
    let (mut stream, address) = listener.accept().await?;

    // Clone data for the spawned task
    let sigs = signatures.clone();
    let cli_clone = cli.clone();

    tokio::spawn(async move {
        // Each connection runs independently
        let signature = sigs.choose(&mut rand::thread_rng());
        // ...
    });
}
```

**Why this pattern:**
- Each connection is independent
- Failure in one task doesn't affect others
- Scales to thousands of connections

---

## CLI Parsing Patterns

### 1. Clap Derive Macro

Used in: **ghostport**, **spiderirc**

```rust
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
}

// Usage:
let cli = Cli::parse();
```

**Why this pattern:**
- Struct fields become CLI arguments
- Attributes define flags, help text, defaults
- Type safety - parse errors happen before main logic

### 2. Clap Builder Pattern (v4)

Used in: **wipedicks**

```rust
use clap::{Command, Arg};

let matches = Command::new("Wipe files/devices with dicks")
    .version("0.0.1")
    .author("vxfemboy")
    .arg(
        Arg::new("recursive")
            .short('r')
            .long("recursive")
            .help("Recursively wipe directories")
            .action(clap::ArgAction::SetTrue)
    )
    .arg(
        Arg::new("numrounds")
            .short('n')
            .long("numrounds")
            .value_parser(clap::value_parser!(usize))
            .default_value("1")
    )
    .arg(
        Arg::new("files")
            .help("Files or directories to wipe")
            .num_args(1..)
            .required(true)
    )
    .get_matches();

// Access values:
let recursive = matches.get_flag("recursive");
let numrounds: usize = *matches.get_one("numrounds").unwrap();
```

**Why this pattern:**
- More flexible than derive for dynamic CLIs
- Runtime argument construction
- Better for complex validation logic

### 3. Manual Argument Parsing

Used in: **purrcrypt**

```rust
fn parse_args_from_vec(args: Vec<String>) -> Result<Command, String> {
    if args.len() < 2 {
        return Ok(Command::Help);
    }

    let verbose = args.iter().any(|arg| arg == "-v" || arg == "--verbose");
    debug::set_verbose(verbose);

    // Filter out verbose flag before parsing
    let filtered_args: Vec<String> = args
        .iter()
        .filter(|arg| *arg != "-v" && *arg != "--verbose")
        .cloned()
        .collect();

    match filtered_args[1].as_str() {
        "genkey" => Ok(Command::GenerateKey {
            name: filtered_args.get(2).cloned(),
        }),
        "encrypt" | "-e" => {
            // Manual flag parsing
            let mut i = 2;
            let mut recipient = None;
            let mut input = None;

            while i < filtered_args.len() {
                match filtered_args[i].as_str() {
                    "-r" | "--recipient" => {
                        recipient = Some(filtered_args.get(i + 1)?);
                        i += 2;
                    }
                    "-i" | "--input" => {
                        input = Some(filtered_args.get(i + 1)?.clone());
                        i += 2;
                    }
                    _ => { i += 1; }
                }
            }

            Ok(Command::Encrypt {
                recipient_key: recipient.ok_or("Missing recipient")?,
                input_file: input.ok_or("Missing input file")?,
                // ...
            })
        }
        _ => Ok(Command::Help),
    }
}
```

**Why this pattern:**
- Full control over parsing logic
- No external dependencies
- Good for simple CLIs

---

## Cryptographic Patterns

### 1. ECDH Key Exchange

Used in: **purrcrypt**

```rust
use k256::ecdh::{EphemeralSecret, diffie_hellman};

// Sender side
let ephemeral_secret = EphemeralSecret::random(&mut OsRng);
let ephemeral_public = PublicKey::from(&ephemeral_secret);

// Compute shared secret
let shared_secret = ephemeral_secret.diffie_hellman(recipient_public_key);

// Receiver side (reconstructs same secret)
let shared_secret = diffie_hellman(
    secret_key.to_nonzero_scalar(),
    ephemeral_public.as_affine()
);
```

**Why this pattern:**
- Ephemeral keys provide forward secrecy per message
- secp256k1 curve is well-audited (used by Bitcoin)
- Both parties compute identical shared secret

### 2. HKDF Key Derivation

```rust
use k256::ecdh::SharedSecret;

// Extract key material with salt
let shared_secret = shared_secret.extract::<sha2::Sha256>(
    Some(b"purrcrypt-salt")
);

// Expand to specific key sizes
let mut encryption_key = [0u8; 32];  // 32 bytes for AES-256
shared_secret.expand(b"encryption key", &mut encryption_key)?;

let mut nonce_bytes = [0u8; 12];  // 12 bytes for GCM
shared_secret.expand(b"nonce", &mut nonce_bytes)?;
```

**Why this pattern:**
- HKDF is a standard KDF (HMAC-based Key Derivation Function)
- Salt prevents rainbow table attacks
- Info strings ("encryption key", "nonce") domain-separate derived keys

### 3. AES-256-GCM Authenticated Encryption

```rust
use aes_gcm::{Aes256Gcm, KeyInit, Key, Nonce, Aead};

let aes_key = Key::<Aes256Gcm>::from_slice(&encryption_key);
let cipher = Aes256Gcm::new(aes_key);
let nonce = Nonce::from_slice(&nonce_bytes);

// Encrypt
let encrypted_data = cipher.encrypt(nonce, data)?;

// Decrypt
cipher.decrypt(nonce, encrypted_data)?;
```

**Why this pattern:**
- GCM provides both confidentiality AND authenticity
- Detects tampering via authentication tag
- Fast hardware-accelerated on modern CPUs

---

## File I/O Patterns

### 1. Secure File Permissions (Unix)

Used in: **purrcrypt**

```rust
#[cfg(unix)]
{
    // Public key: readable by all
    fs::set_permissions(pub_path, fs::Permissions::from_mode(0o644))?;

    // Private key: owner read/write ONLY
    fs::set_permissions(secret_path, fs::Permissions::from_mode(0o600))?;

    // Directory: owner full access only
    fs::set_permissions(&keys_dir, fs::Permissions::from_mode(0o700))?;
}
```

**Why this pattern:**
- `#[cfg(unix)]` compiles only on Unix systems
- `0o600` = `-rw-------` (owner read/write)
- `0o644` = `-rw-r--r--` (owner rw, others read)
- `0o700` = `-rwx------` (owner full)

### 2. Buffered I/O

Used in: **purrcrypt**, **ghostport**

```rust
use std::io::{BufReader, BufWriter, Read};

// Reading
let mut input_file = BufReader::new(File::open(input_filename)?);
let mut input_data = Vec::new();
input_file.read_to_end(&mut input_data)?;  // Efficient bulk read

// Writing
let mut output_file = BufWriter::new(File::create(output_filename)?);
cipher.process_data(data, &mut output_file, CipherMode::Encrypt)?;
// Buffer auto-flushes on drop
```

**Why this pattern:**
- `BufReader` reduces syscalls by buffering reads
- `BufWriter` batches small writes
- Automatic flushing on drop

### 3. Low-Level File Operations

Used in: **wipedicks**

```rust
use std::fs::OpenOptions;

// Open for writing only
let mut file = OpenOptions::new()
    .write(true)
    .open(dev)?;

// Write repeatedly
while dlen < size {
    let pattern = rand_dick(rng);
    file.write_all(pattern.as_bytes())?;
}
```

**Why this pattern:**
- `OpenOptions` provides fine-grained control
- `write_all()` ensures complete writes
- Suitable for overwriting existing data

---

## Threading Patterns

### 1. Spawn-per-Work Pattern

Used in: **wipedicks**, **ghostport**

```rust
use std::thread;

let mut handles = Vec::new();

for file in file_list {
    let handle = thread::spawn(move || {
        // Each thread gets its own RNG
        let mut rng = thread_rng();

        if let Err(e) = wipe(&file, numrounds, &mut rng) {
            eprintln!("ERROR: {:?}: {:?}", file, e);
        }
    });
    handles.push(handle);
}

// Wait for all threads to complete
for handle in handles {
    handle.join().unwrap();
}
```

**Why this pattern:**
- `move` transfers ownership to thread
- Each thread has independent state
- `join()` waits for thread completion

### 2. Thread-Local RNG

```rust
use rand::{thread_rng, prelude::*};

// Inside thread
let mut rng = thread_rng();  // Each thread gets its own RNG

// Use RNG
let index = rng.gen_range(0..DICKS.len());
```

**Why this pattern:**
- `thread_rng()` is faster than shared RNG
- No lock contention between threads
- Each thread has independent random sequence

---

## Logging Patterns

### 1. Tracing Subscriber Setup

Used in: **ghostport**, **spiderirc**

```rust
use tracing_subscriber::FmtSubscriber;
use tracing::Level;

let subscriber = FmtSubscriber::builder()
    .with_max_level(if cli.debug {
        Level::DEBUG
    } else if cli.verbose {
        Level::INFO
    } else {
        Level::ERROR
    })
    .without_time()  // Omit timestamps for cleaner output
    .finish();

tracing::subscriber::set_global_default(subscriber)?;
```

**Why this pattern:**
- Configurable log levels via CLI flags
- `tracing` is more structured than `log`
- `.without_time()` for cleaner CLI output

### 2. Macro-Based Debug Logging

Used in: **purrcrypt**

```rust
// src/debug.rs
use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(enabled: bool) {
    VERBOSE.store(enabled, Ordering::SeqCst);
}

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::SeqCst)
}

// Macro definition
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        if $crate::debug::is_verbose() {
            eprintln!($($arg)*);
        }
    };
}

// Usage in code
debug!("Public key length (base64): {}", pub_data.len());
debug_hex("Input data", &input_data);
```

**Why this pattern:**
- AtomicBool for thread-safe global flag
- Macro expands to nothing when verbose is false
- Zero overhead when disabled

### 3. Structured Logging

```rust
use tracing::{debug, info, error};

debug!("Parsed CLI flags");
debug!("Read {} signatures", signatures.len());
info!("Started listener on {}", cli.listen);

error!("Failed to parse signatures file: {}", e);
error!("Failed to write payload to {}: {}", address, e);
```

**Why this pattern:**
- Format strings like `println!`
- Different levels for different severity
- Can be extended with structured fields

---

## Security Best Practices

### 1. Private Key Permissions

```rust
// ALWAYS use 0o600 for private keys
#[cfg(unix)]
fs::set_permissions(secret_path, fs::Permissions::from_mode(0o600))?;

// Verify permissions before use
pub fn verify_permissions(&self) -> Result<(), KeystoreError> {
    for entry in fs::read_dir(self.keys_dir.join("private"))? {
        let entry = entry?;
        let mode = entry.metadata()?.permissions().mode();

        if mode & 0o077 != 0 {
            return Err(KeystoreError::InvalidPermissions(format!(
                "Private key {} has unsafe permissions: {:o}",
                entry.path().display(), mode
            )));
        }
    }
    Ok(())
}
```

### 2. Secure Random Number Generation

```rust
use k256::elliptic_curve::rand_core::OsRng;

// Use OS CSPRNG for cryptographic operations
let secret_key = SecretKey::random(&mut OsRng);
let ephemeral_secret = EphemeralSecret::random(&mut OsRng);
```

**Why:**
- `OsRng` uses OS entropy sources (`/dev/urandom`, `getrandom()`)
- Never use `rand::thread_rng()` for crypto
- Thread-local RNG is fine for non-crypto randomness

### 3. Constant-Time Operations

When comparing secrets, use constant-time comparison:

```rust
use subtle::ConstantTimeEq;

if a.ct_eq(&b).into() {
    // Secrets match
}
```

**Why:**
- Prevents timing attacks
- Comparison time doesn't leak information about which bytes differ

### 4. Memory Clearing

For sensitive data in memory:

```rust
use zeroize::Zeroize;

let mut secret = vec![0u8; 32];
// Use secret...
secret.zeroize();  // Overwrite with zeros
```

---

## Production Considerations

### 1. Dependency Auditing

```bash
# Install cargo-audit
cargo install cargo-audit

# Run audit
cargo audit

# Check for unmaintained crates
cargo install cargo-outdated
cargo outdated
```

### 2. Build Optimizations

```toml
# Cargo.toml
[profile.release]
lto = true           # Link-Time Optimization
codegen-units = 1    # Single compilation unit
panic = "abort"      # Smaller binaries
strip = true         # Remove debug symbols
```

### 3. Error Reporting

For production applications:

```rust
use anyhow::{Context, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

fn main() -> Result<()> {
    if let Err(e) = run() {
        eprintln!("Error: {:?}", e);  // Use {:?} for debug format
        std::process::exit(1);
    }
    Ok(())
}
```

### 4. Configuration Management

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub listen_address: String,
    pub log_level: String,
    pub max_connections: usize,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        toml::from_str(&contents).map_err(Into::into)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}
```

---

## Summary Table

| Pattern | Used In | crates |
|---------|---------|--------|
| Error types with thiserror | purrcrypt, ghostport | `thiserror` |
| Anyhow for applications | ghostport, spiderirc | `anyhow` |
| Tokio async runtime | ghostport, spiderirc | `tokio` |
| Clap derive macros | ghostport, spiderirc | `clap` |
| Tracing logging | ghostport, spiderirc | `tracing`, `tracing-subscriber` |
| ECDH key exchange | purrcrypt | `k256` |
| AES-256-GCM | purrcrypt | `aes-gcm` |
| Secure permissions | purrcrypt | - |
| Thread spawning | wipedicks | `rand` |
| Select event loop | spiderirc | `tokio::select` |

---

## Files Referenced

- `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/keys.rs`
- `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/crypto.rs`
- `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/keystore.rs`
- `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/debug.rs`
- `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/ghostport/src/main.rs`
- `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/ghostport/src/handler.rs`
- `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/spiderirc/src/main.rs`
- `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/wipedicks/src/main.rs`
