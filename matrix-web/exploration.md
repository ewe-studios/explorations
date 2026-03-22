---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.matrix-web
repository: https://github.com/element-hq (main)
explored_at: 2026-03-22
language: Rust, TypeScript, Kotlin, Swift, Go, Python
---

# Project Exploration: Matrix Web Ecosystem

## Overview

The Matrix ecosystem is a collection of projects implementing the **Matrix protocol** - an open standard for secure, decentralized, real-time communication. This exploration covers the Rust-based projects within Element's Matrix implementation, including authentication services, rich text editors, cryptography libraries, and server implementations.

### Key Value Proposition

- **Open protocol** - Matrix is an open standard for interoperable real-time communication
- **End-to-end encryption** - Built-in E2EE using Olm/Megolm (Double Ratchet variant)
- **Cross-platform** - Rust core with bindings for Web (WASM), Android (Kotlin), iOS (Swift)
- **Federated architecture** - Decentralized homeserver model similar to email
- **Bridges everywhere** - Protocol bridges to WhatsApp, Telegram, Signal, Slack, etc.
- **Production-ready** - Powers Element, Matrix.org, and enterprise deployments

### Example Usage

```rust
// Matrix Rich Text Editor (wysiwyg crate)
use wysiwyg::{ComposerModel, ComposerAction};

let mut model = ComposerModel::new();
model.replace_text("Hello, Matrix!");
let html = model.get_content_as_html();

// Apply formatting
model.select(0, 5); // Select "Hello"
model.bold();

// JOSE for JWT/OIDC (josekit)
use josekit::jws::{encode_raw, JwtClaims};
use josekit::jwk::Jwk;

let claims = JwtClaims::new()
    .with_issuer("matrix-org")
    .with_subject("user123");

let (token, _) = encode_raw(&claims, &signer)?;

// Matrix Authentication Service
// OIDC provider for Matrix homeservers
// Handles OAuth2 flows, token issuance, session management
```

## Repository Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.matrix-web/
│
├── matrix-authentication-service/      # OAuth2.0 + OIDC Provider for Matrix
│   ├── crates/
│   │   ├── axum-utils/                 # Axum HTTP utilities
│   │   ├── cli/                        # Command-line interface
│   │   ├── config/                     # Configuration system
│   │   ├── data-model/                 # Core data structures
│   │   ├── email/                      # Email sending service
│   │   ├── graphql/                    # GraphQL API
│   │   ├── handlers/                   # HTTP request handlers
│   │   ├── http/                       # HTTP server infrastructure
│   │   ├── i18n/                       # Internationalization
│   │   ├── i18n-scan/                  # i18n string scanning
│   │   ├── iana/                       # IANA registry data
│   │   ├── iana-codegen/               # Codegen for IANA data
│   │   ├── jose/                       # JOSE (JWT/JWS/JWE) support
│   │   ├── keystore/                   # Cryptographic key storage
│   │   ├── listener/                   # HTTP listener configuration
│   │   ├── matrix/                     # Matrix protocol integration
│   │   ├── matrix-synapse/             # Synapse integration
│   │   ├── oauth2-types/               # OAuth2 type definitions
│   │   ├── oidc-client/                # OIDC client implementation
│   │   ├── policy/                     # Authorization policies
│   │   ├── router/                     # HTTP routing
│   │   ├── spa/                        # Single-page app serving
│   │   ├── storage/                    # Storage trait
│   │   ├── storage-pg/                 # PostgreSQL storage
│   │   ├── syn2mas/                    # Synapse to MAS migration
│   │   ├── tasks/                      # Background task system
│   │   ├── templates/                  # Template rendering
│   │   └── tower/                      # Tower middleware
│   ├── docs/                           # Documentation
│   ├── Cargo.toml                      # Workspace configuration
│   └── README.md
│
├── matrix-rich-text-editor/            # Cross-platform rich text editor
│   ├── bindings/
│   │   ├── wysiwyg-ffi/                # FFI bindings (Android/iOS)
│   │   │   ├── src/
│   │   │   │   ├── lib.rs              # Uniffi FFI layer
│   │   │   │   └── composer.rs         # Composer FFI
│   │   │   ├── uniffi.toml             # Uniffi configuration
│   │   │   └── README.md
│   │   └── wysiwyg-wasm/               # WASM bindings (Web)
│   │       ├── src/
│   │       │   ├── lib.rs              # wasm-bindgen layer
│   │       │   └── composer.rs         # Composer WASM
│   │       ├── package.json
│   │       └── README.md
│   ├── crates/
│   │   ├── wysiwyg/                    # Core editor logic (Rust)
│   │   │   ├── src/
│   │   │   │   ├── composer_model.rs   # Main editor state
│   │   │   │   ├── dom/                # DOM tree representation
│   │   │   │   ├── actions/            # Editing actions
│   │   │   │   └── menu_state.rs       # Toolbar state
│   │   │   └── Cargo.toml
│   │   └── matrix_mentions/            # Matrix mention handling
│   ├── platforms/
│   │   ├── android/                    # Android Kotlin bindings
│   │   ├── ios/                        # iOS Swift bindings
│   │   └── web/                        # Web JavaScript bindings
│   ├── Cargo.toml
│   └── README.md
│
├── josekit-rs/                         # JOSE library (JWT/JWS/JWE)
│   ├── src/
│   │   ├── jws/                        # JSON Web Signature
│   │   ├── jwe/                        # JSON Web Encryption
│   │   ├── jwk/                        # JSON Web Key
│   │   ├── jwa/                        # JSON Web Algorithms
│   │   └── jwt/                        # JSON Web Token
│   ├── Cargo.toml
│   └── README.md
│
├── synapse/                            # Matrix homeserver (Python + Rust)
│   ├── rust/                           # Rust extension modules
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── synapse/                        # Main Python codebase
│   └── Cargo.toml
│
├── element-web/                        # Web client (TypeScript/React)
│   ├── src/                            # React components
│   ├── playwrite/                      # E2E tests
│   └── package.json
│
├── element-desktop/                    # Desktop client (Electron)
│   ├── src/                            # Electron main process
│   └── package.json
│
├── element-android/                    # Android client (Kotlin)
│   ├── app/                            # Main app
│   └── vector-app/
│
├── element-ios/                        # iOS client (Swift)
│   ├── ElementX/
│   └── Tools/
│
├── element-call/                       # Video conferencing (WebRTC)
│   ├── src/
│   └── embedded/
│
├── hydrogen-web/                       # Lightweight web client
│   ├── src/
│   └── platform/
│
├── compound-web/                       # Design system components
│   ├── src/
│   └── packages/
│
├── matrix-bot-sdk/                     # Bot development SDK (TypeScript)
│   └── src/
│
├── mautrix-go/                         # Bridge framework (Go)
│   ├── bridgev2/
│   ├── crypto/
│   └── README.md
│
├── dendrite/                           # Next-gen homeserver (Go)
│   ├── clientapi/
│   ├── roomserver/
│   └── syncapi/
│
└── chaos/                              # Chaos testing tools
    ├── web/
    └── mitmproxy_addons/
```

## Core Projects

### 1. Matrix Authentication Service (MAS)

MAS is an OAuth 2.0 and OpenID Connect Provider for Matrix homeservers, implementing [MSC3861](https://github.com/matrix-org/matrix-doc/pull/3861).

**Architecture:**

```
┌─────────────────────────────────────────────────────────────────┐
│                    MAS Architecture                               │
│                                                                   │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────────┐   │
│  │   Client    │────▶│   MAS OIDC  │────▶│    Synapse      │   │
│  │  (Element)  │     │   Provider  │     │   Homeserver    │   │
│  └─────────────┘     └─────────────┘     └─────────────────┘   │
│         │                   │                      │            │
│         │ 1. OIDC Auth      │                      │            │
│         │ 2. Access Token   │                      │            │
│         │──────────────────▶│                      │            │
│         │                   │ 3. Token introspect  │            │
│         │                   │─────────────────────▶│            │
│         │                   │ 4. User info         │            │
│         │                   │◀─────────────────────│            │
│         │ 5. API requests with token               │            │
│         │─────────────────────────────────────────▶│            │
└─────────────────────────────────────────────────────────────────┘
```

**Key crates:**

```rust
// mas-jose: JOSE cryptography
use mas_jose::{jws, jwk, Algorithm};

let jwk = Jwk::generate_ec_p256()?;
let signer = jws::Signer::new(Algorithm::ES256, &jwk)?;

// mas-keystore: Key management
use mas_keystore::Keystore;

let store = Keystore::from_jwk(&jwk);
let key = store.get_signing_key("key_id")?;

// mas-storage-pg: PostgreSQL persistence
use mas_storage_pg::user::UserRepository;

let repo = UserRepository::new(&mut conn);
let user = repo.find_by_username("alice").await?;

// mas-handlers: OAuth2/OIDC handlers
use mas_handlers::authorization::AuthorizationHandler;

let handler = AuthorizationHandler::new(config, store);
let response = handler.handle(request).await?;
```

**Features:**

- **OAuth 2.1 compliance** - Implements latest OAuth 2.1 security best practices
- **OpenID Connect** - Full OIDC provider with discovery, userinfo, introspection
- **Synapse integration** - Direct database integration with Synapse homeserver
- **Migration tools** - syn2mas for migrating existing Synapse users
- **Upstream OIDC** - Delegate authentication to external IdPs (Keycloak, Google, etc.)
- **Application Services** - Support for encrypted bridges via AS login

### 2. Matrix Rich Text Editor

A cross-platform rich text editor for Matrix message composition with Rust core and platform-specific bindings.

**Architecture:**

```
┌─────────────────────────────────────────────────────────────────┐
│              Matrix Rich Text Editor Architecture                │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Rust Core (wysiwyg)                    │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │   │
│  │  │ComposerModel│  │  DOM Tree   │  │  Action System  │  │   │
│  │  │             │  │  (nodes,    │  │  (bold, italic, │  │   │
│  │  │ - state     │  │   ranges)   │  │   links, etc)   │  │   │
│  │  │ - selection │  │             │  │                 │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘  │   │
│  └──────────────────────────────────────────────────────────┘   │
│         │                    │                    │              │
│         │ Uniffi FFI         │ wasm-bindgen       │              │
│         ▼                    ▼                                   │
│  ┌─────────────┐      ┌─────────────┐                            │
│  │   Kotlin    │      │  JavaScript │                            │
│  │  (Android)  │      │    (Web)    │                            │
│  └─────────────┘      └─────────────┘                            │
│         │                                                       │
│         │ Uniffi FFI                                            │
│         ▼                                                       │
│  ┌─────────────┐                                                 │
│  │    Swift    │                                                 │
│  │   (iOS)     │                                                 │
│  └─────────────┘                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Core editor model:**

```rust
use wysiwyg::{ComposerModel, ComposerAction, DomHandle};

// Create new editor
let mut model: ComposerModel<UnicodeString> = ComposerModel::new();

// Text manipulation
model.replace_text("Hello World");
model.select(0, 5);  // Select "Hello"

// Formatting
model.bold();  // Apply bold to selection
model.italic();

// Links
model.select(6, 11);  // Select "World"
model.set_link("https://example.com".to_owned());

// Get content
let html = model.get_content_as_html();  // "<b>Hello</b> <a href=\"...\">World</a>"
let markdown = model.get_content_as_markdown();

// Menu state (for toolbar)
let menu_state = model.compute_menu_state();
let is_bold_active = menu_state.contains(ComposerAction::Bold);
```

**WASM bindings:**

```rust
// bindings/wysiwyg-wasm/src/lib.rs
use wasm_bindgen::prelude::*;
use wysiwyg::ComposerModel;

#[wasm_bindgen]
pub struct WysiwygComposer {
    model: ComposerModel<String>,
}

#[wasm_bindgen]
impl WysiwygComposer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            model: ComposerModel::new(),
        }
    }

    pub fn replace_text(&mut self, text: &str) {
        self.model.replace_text(text.into());
    }

    pub fn get_content_as_html(&self) -> String {
        self.model.get_content_as_html().to_string()
    }

    pub fn bold(&mut self) {
        self.model.bold();
    }
}
```

**Usage from JavaScript:**

```js
import { initAsync, WysiwygComposer } from '@matrix-org/matrix-sdk-wysiwyg-wasm';

await initAsync();

const composer = new WysiwygComposer();
composer.replace_text('Hello Matrix');
composer.bold();
const html = composer.get_content_as_html();
```

**FFI bindings (Uniffi):**

```rust
// bindings/wysiwyg-ffi/src/lib.rs
use uniffi::export;

#[export]
pub struct ComposerModel {
    inner: wysiwyg::ComposerModel<String>,
}

#[export]
impl ComposerModel {
    pub fn new() -> Self {
        Self {
            inner: wysiwyg::ComposerModel::new(),
        }
    }

    pub fn replace_text(&mut self, text: String) {
        self.inner.replace_text(text.into());
    }

    pub fn get_content_as_html(&self) -> String {
        self.inner.get_content_as_html().to_string()
    }
}

uniffi::include_scaffolding!("wysiwyg");
```

**Usage from Kotlin:**

```kotlin
import org.matrix.wysiwyg.ComposerModel

val composer = ComposerModel()
composer.replaceText("Hello Matrix")
composer.bold()
val html = composer.getContentAsHtml()
```

**Usage from Swift:**

```swift
import WysiwygComposer

let composer = ComposerModel()
composer.replaceText(text: "Hello Matrix")
composer.bold()
let html = composer.getContentAsHtml()
```

### 3. JOSE Library (josekit-rs)

A JOSE (Javascript Object Signing and Encryption) library for Rust, supporting JWT, JWS, JWE, JWA, and JWK.

**Supported algorithms:**

| Algorithm | Description | Key Type |
|-----------|-------------|----------|
| HS256/384/512 | HMAC using SHA | oct (32-64 bytes) |
| RS256/384/512 | RSASSA-PKCS1-v1_5 | RSA (1024+ bits) |
| PS256/384/512 | RSASSA-PSS | RSA (1024+ bits) |
| ES256/384/512 | ECDSA using P-256/384/521 | EC |
| ES256K | ECDSA using secp256k1 | EC |
| EdDSA | EdDSA signatures | OKP (Ed25519/Ed448) |

**Usage:**

```rust
use josekit::jws::{encode_raw, decode, JwtClaims};
use josekit::jwk::{Jwk, JwkSet};
use josekit::jwe::{encode, decode as jwe_decode};

// JWS (Signed JWT)
let mut claims = JwtClaims::new();
claims.set_issuer("matrix-org");
claims.set_subject("user123");

let jwk = Jwk::generate_ec_p256()?;
let (token, _checksum) = encode_raw(&claims, &jwk)?;

// Verify
let (verified_claims, _header) = decode(&token, &jwk.to_public())?;

// JWE (Encrypted JWT)
let jwk = Jwk::generate_rsa(2048)?;
let encrypted = encode(&claims, &jwk)?;

// Decrypt
let (decrypted_claims, _header) = jwe_decode(&encrypted, &jwk)?;

// Key management
let jwks = JwkSet::load_from_file("keys.json")?;
let key = jwks.find_key("key-id-123");
```

**Use in MAS:**

```rust
// MAS uses josekit for:
// - Access token signing (JWT)
// - Refresh token signing
// - OIDC ID tokens
// - Client assertion tokens
// - Key rotation (JWK sets)

use mas_jose::{jws, jwk, jwt};

let signer = jws::Signer::new(Algorithm::ES256, &signing_key)?;
let token = jwt::encode(&claims, &signer)?;
```

### 4. Synapse (Python + Rust extensions)

Synapse is the reference Matrix homeserver implementation, primarily in Python with Rust extensions for performance-critical code.

**Rust extension module:**

```rust
// synapse/rust/src/lib.rs
use pyo3::prelude::*;

#[pyfunction]
fn canonical_json(value: &PyAny) -> PyResult<String> {
    // Canonical JSON for Matrix federation
    Ok(serde_json::to_string(&value)?)
}

#[pymodule]
fn synapse_rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(canonical_json, m)?)?;
    Ok(())
}
```

**Use from Python:**

```python
import synapse_rust

# Canonical JSON for federation signatures
canonical = synapse_rust.canonical_json(event_dict)
```

## Matrix Protocol Fundamentals

### Room Structure

```
┌─────────────────────────────────────────────────────────────────┐
│                    Matrix Room Structure                         │
│                                                                   │
│  Room: !roomid:matrix.org                                        │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ Event DAG (Directed Acyclic Graph)                          │ │
│  │                                                              │ │
│  │  ┌──────┐     ┌──────┐                                      │ │
│  │  │ evt1 │────▶│ evt2 │                                      │ │
│  │  │(join)│     │(msg1)│                                      │ │
│  │  └──────┘     └──────┘                                      │ │
│  │       │            │                                        │ │
│  │       │      ┌─────┴─────┐                                  │ │
│  │       │      │   evt3    │                                  │ │
│  │       │      │  (msg2)   │                                  │ │
│  │       │      └───────────┘                                  │ │
│  │       │            │                                        │ │
│  │       └───────────┼─────────────────────────┐               │ │
│  │                   ▼                         ▼              │ │
│  │              ┌───────┐               ┌───────────┐         │ │
│  │              │ evt4  │               │  evt5     │         │ │
│  │              │(msg3) │               │  (edit)   │         │ │
│  │              └───────┘               └───────────┘         │ │
│  │                   │                         │              │ │
│  │                   └──────────┬──────────────┘              │ │
│  │                              ▼                              │ │
│  │                        ┌─────────┐                         │ │
│  │                        │  evt6   │                         │ │
│  │                        │ (state) │                         │ │
│  │                        └─────────┘                         │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  State: Current room state (members, topic, power levels, etc.)  │
│  Timeline: Ordered list of events (messages, state changes)      │
└─────────────────────────────────────────────────────────────────┘
```

### Event Structure

```json
{
  "type": "m.room.message",
  "state_key": null,
  "event_id": "$event123:matrix.org",
  "room_id": "!room123:matrix.org",
  "sender": "@alice:matrix.org",
  "origin_server_ts": 1234567890123,
  "unsigned": {
    "age": 1234
  },
  "content": {
    "msgtype": "m.text",
    "body": "Hello, Matrix!",
    "format": "org.matrix.custom.html",
    "formatted_body": "<b>Hello</b>, Matrix!"
  },
  "prev_events": [
    ["$prev_event1", {"sha256": "..."}],
    ["$prev_event2", {"sha256": "..."}]
  ],
  "auth_events": [
    ["$member_event", {"sha256": "..."}]
  ],
  "hashes": {
    "sha256": "..."
  },
  "signs": {
    "matrix.org": {
      "ed25519:1": "signature_here"
    }
  }
}
```

### Encryption (Olm/Megolm)

```
┌─────────────────────────────────────────────────────────────────┐
│              Matrix Encryption Architecture                      │
│                                                                   │
│  Double Ratchet Algorithm (Signal Protocol variant)              │
│                                                                   │
│  Sender Chain:                                                   │
│  ┌─────────┐   ┌─────────┐   ┌─────────┐                       │
│  │ DH(r)   │──▶│ DH(r)   │──▶│ DH(r)   │──▶ ...               │
│  │ KDF(r)  │   │ KDF(r)  │   │ KDF(r)  │                       │
│  └─────────┘   └─────────┘   └─────────┘                       │
│       │             │             │                              │
│       ▼             ▼             ▼                              │
│   Message 1     Message 2     Message 3                         │
│                                                                   │
│  Megolm Session (for rooms with multiple participants):          │
│  ┌─────────────┐                                                 │
│  │ Session Key │──▶ Ratchet──▶ Next Key                        │
│  └─────────────┘                                                 │
│       │                                                          │
│       ├─────▶ Alice (encrypted with Alice's Olm session)        │
│       ├─────▶ Bob (encrypted with Bob's Olm session)            │
│       └─────▶ Carol (encrypted with Carol's Olm session)        │
└─────────────────────────────────────────────────────────────────┘
```

**Olm session (1:1):**

```rust
// Create Olm session
use olm_rs::{session::OlmSession, account::OlmAccount};

let alice = OlmAccount::new().unwrap();
let bob = OlmAccount::new().unwrap();

// One-time key exchange
let alice_otk = alice.one_time_keys().pop().unwrap();
let bob_otk = bob.one_time_keys().pop().unwrap();

// Create sessions
let mut alice_session = OlmSession::new(
    &alice,
    bob.identity_keys(),
    bob_otk
).unwrap();

let mut bob_session = OlmSession::new(
    &bob,
    alice.identity_keys(),
    alice_otk
).unwrap();

// Encrypt
let encrypted = alice_session.encrypt("Hello Bob").unwrap();

// Decrypt
let decrypted = bob_session.decrypt(&encrypted).unwrap();
```

## Build System

### Matrix Rich Text Editor

```bash
# Build all bindings
make

# Build WASM only
cd bindings/wysiwyg-wasm
yarn
yarn build

# Build Android
cd bindings/wysiwyg-ffi
make android

# Build iOS
cd bindings/wysiwyg-ffi
make ios
```

### MAS

```bash
# Build
cargo build --release

# Run
cargo run -- serve

# Test
cargo test

# Generate docs
cargo doc --open
```

## Comparison: MAS vs Keycloak for Matrix

| Aspect | MAS | Keycloak + Adapter |
|--------|-----|-------------------|
| Matrix integration | Native | Requires adapter |
| MSC3861 compliance | Full | Partial |
| Synapse migration | Built-in tools | Manual |
| OIDC features | Complete | Complete |
| Footprint | Lightweight | Heavy |
| Customization | Rust crates | Java/SPI |

## Trade-offs

| Design Choice | Benefit | Cost |
|---------------|---------|------|
| Rust core + FFI | Single source of truth, performance | FFI complexity, build overhead |
| Uniffi for mobile | Type-safe bindings | Code generation step |
| wasm-bindgen for web | Direct JS interop | WASM bundle size |
| Python + Rust (Synapse) | Rapid dev + performance | Two language stacks |
| PostgreSQL for MAS | Reliable, familiar | Not distributed |
| Olm/Megolm encryption | E2EE, forward secrecy | Key management complexity |

## Related Projects

### In this Repository

- **dendrite** - Next-generation Matrix homeserver in Go
- **element-web** - Reference Matrix web client
- **element-call** - Video conferencing with Matrix
- **mautrix-go** - Bridge framework for Matrix

### External

- **matrix-rust-sdk** - Full Matrix SDK in Rust
- **vodozemac** - Modern Olm/Megolm implementation
- **conduit** - Lightweight Matrix homeserver in Rust
- **beeper** - Matrix-based unified inbox

## References

- [Matrix Specification](https://spec.matrix.org/)
- [MAS Documentation](https://element-hq.github.io/matrix-authentication-service/)
- [Matrix Rich Text Editor Demo](https://element-hq.github.io/matrix-rich-text-editor/)
- [Are We OIDC Yet?](https://areweoidcyet.com/) - Matrix OIDC progress tracker
