---
title: Keys — Author and Namespace Cryptographic Keys
---

# Keys — Author and Namespace Cryptographic Keys

iroh-docs uses two Ed25519 keypair types: Author keys for identity and Namespace keys for write capability.

## Author Keys

```rust
// iroh-docs/src/keys.rs
pub struct Author {
    /// Ed25519 signing key.
    signing_key: SigningKey,
}

pub struct AuthorPublicKey {
    verifying_key: VerifyingKey,
}

pub struct AuthorId {
    /// 32-byte identifier (first 32 bytes of public key).
    id: [u8; 32],
}
```

Source: `iroh-docs/src/keys.rs:1`.

## Namespace Keys

```rust
// iroh-docs/src/keys.rs
pub struct NamespaceSecret {
    signing_key: SigningKey,
}

pub struct NamespacePublicKey {
    verifying_key: VerifyingKey,
}

pub struct NamespaceId {
    id: [u8; 32],
}
```

Source: `iroh-docs/src/keys.rs:1`.

## Capability (Write Token)

```rust
// iroh-docs/src/keys.rs
pub struct Capability {
    kind: CapabilityKind,  // Write or Read
    bytes: [u8; 32],       // Secret or public key bytes
}

pub enum CapabilityKind {
    Write,  // Has secret key → can write
    Read,   // Has public key only → read-only
}
```

Source: `iroh-docs/src/keys.rs:1`.

## Key Serialization

All key types implement:
- `Display` / `FromStr` — base32 encoding
- `Serialize` / `Deserialize` — postcard serialization
- `Ord` / `PartialOrd` — ordering for database keys

Source: `iroh-docs/src/keys.rs:1`.

## Key Generation

```rust
impl Author {
    pub fn new() -> Self { ... }  // Generate new keypair
}

impl NamespaceSecret {
    pub fn new() -> Self { ... }  // Generate new namespace key
}
```

Source: `iroh-docs/src/keys.rs:1`.

## Dual Signing on Entries

Every entry has two signatures:

1. **Author signature** — signs the entry content with the author's signing key
2. **Namespace signature** — signs the entry with the namespace's signing key

This means:
- To create an entry, you need BOTH the author secret key AND the namespace secret key
- To verify an entry, you need the author public key AND the namespace public key
- Read-only users (with only namespace public key) can verify but not create entries

Source: `iroh-docs/src/sync.rs:1` — `EntrySignature` with `author_signature` and `namespace_signature`.

## Related Documents

- [Replica](../markdown/02-replica.md) — Entries signed with these keys
- [Engine](../markdown/07-engine.md) — Author/Namespace management in Engine
