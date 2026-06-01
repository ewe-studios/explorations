# cryptr Documentation

Simple encrypted (streaming) values in Rust.

## Document Index

| # | Document | Description |
|---|----------|-------------|
| 00 | [Overview](00-overview.html) | Philosophy, features |
| 01 | [Encryption](01-encryption.html) | ChaCha20Poly1305, format |
| 02 | [Keys](02-keys.html) | Key derivation, management |
| 03 | [Streaming](03-streaming.html) | Streaming architecture |
| 04 | [S3](04-s3.html) | S3 integration |
| 05 | [CLI](05-cli.html) | CLI usage |

## Quick Links

- **Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.auth/src.rauthy/cryptr/`
- **Repository:** https://github.com/sebadob/cryptr.git

## What is cryptr?

A Rust library and CLI for encrypting values and files with ChaCha20Poly1305:

```rust
use cryptr::value::encrypt;

// Encrypt a value
let encrypted = encrypt(b"secret data", &key)?;
// ~40 byte header + ciphertext
```

## Features

| Feature | Description |
|---------|-------------|
| **Small values** | ~40 byte header overhead |
| **Streaming** | Files of any size |
| **AEAD** | ChaCha20Poly1305 |
| **Key rotation** | Built-in versioning |
| **S3** | Direct streaming |
| **CLI** | Command-line tool |

## Next Steps

Start with [Overview →](00-overview.html).
