---
title: "CBOR Use Cases Deep Dive"
subtitle: "COSE, CWT, SenML, IoT protocols, WebAuthn, and real-world applications"
level: "Advanced - Protocol designers and implementers"
---

# CBOR Use Cases Deep Dive

## Table of Contents

1. [COSE: CBOR Object Signing and Encryption](#1-cose-cbor-object-signing-and-encryption)
2. [CWT: CBOR Web Tokens](#2-cwt-cbor-web-tokens)
3. [SenML: Sensor Measurement Lists](#3-senml-sensor-measurement-lists)
4. [IoT Protocols](#4-iot-protocols)
5. [WebAuthn Authenticator Data](#5-webauthn-authenticator-data)
6. [Blockchain: Cardano Ledger](#6-blockchain-cardano-ledger)
7. [OSCORE: Object Security for Constrained Environments](#7-oscore-object-security-for-constrained-environments)

---

## 1. COSE: CBOR Object Signing and Encryption

### 1.1 What is COSE?

**COSE (RFC 8152)** provides cryptographic services for CBOR:
- Digital signatures
- Message authentication codes (MAC)
- Encryption with authenticated data
- Key agreement and derivation

### 1.2 COSE Structure

```cddl
; COSE Sign1 (signed message with one signature)
COSE_Sign1 = [
    Headers,
    payload: bstr,
    signature: bstr
]

Headers = {
    ? 1 => kid,            ; Key ID
    ? 4 => alg,            ; Algorithm
    ? 5 => crit,           ; Critical options
    * tstr => any,         ; Custom headers
}

; Protected headers (encoded as bstr)
bstr .cbor Headers
```

### 1.3 COSE Algorithm Registry

```cddl
; Common COSE algorithms
COSE_Algorithm = &(
    ES256: -7,      ; ECDSA with P-256 and SHA-256
    ES384: -35,     ; ECDSA with P-384 and SHA-384
    ES512: -36,     ; ECDSA with P-521 and SHA-512
    EdDSA: -8,      ; EdDSA (Ed25519, Ed448)
    HS256: 5,       ; HMAC with SHA-256
    HS384: 6,       ; HMAC with SHA-384
    HS512: 7,       ; HMAC with SHA-512
)
```

### 1.4 COSE Sign1 Example

```rust
use cosey; // Hypothetical COSE library

// Create a signed message
let protected = Headers {
    alg: Some(Algorithm::ES256),
    kid: Some(vec![1, 2, 3, 4]),
};

let payload = b"Hello, World!";
let external_aad = b""; // Additional authenticated data

// Sign with private key
let signer = cosey::Signer::new(Algorithm::ES256, &private_key);
let sign1 = signer.sign(&protected, payload, external_aad)?;

// Encode to CBOR
let cbor = serde_cbor::to_vec(&sign1)?;

// Verify with public key
let verifier = cosey::Verifier::new(&public_key);
let verified_payload = verifier.verify(&cbor, external_aad)?;
```

### 1.5 CBOR Encoding of COSE Sign1

```
COSE_Sign1 = [
    h'a10427',           ; Protected: {4: -7} (ES256)
    {},                  ; Unprotected: {}
    h'48656c6c6f',       ; Payload: "Hello"
    h'3045022100...      ; Signature (ECDSA)
]

Full hex:
84                                     ; array(4)
   45 a1 04 27                         ; bstr(5) {4: -7}
   a0                                  ; map(0) - empty
   45 48 65 6c 6c 6f                   ; bstr(5) "Hello"
   48 30 45 02 21 00 ...               ; bstr(8) signature
```

### 1.6 Use Cases

```
1. IoT Device Attestation
   - Device signs sensor data
   - Server verifies authenticity
   - Compact encoding for constrained networks

2. Mobile Documents
   - Digital credentials
   - Verifiable claims
   - Offline verification

3. Secure Firmware Updates
   - Firmware signed by manufacturer
   - Device verifies before installation
   - Protects against tampering
```

---

## 2. CWT: CBOR Web Tokens

### 2.1 What is CWT?

**CWT (RFC 8392)** is a compact token format for authorization:
- CBOR-encoded claims
- JWT compatibility (can convert)
- Used in ACE (Authentication and Authorization for Constrained Environments)
- Smaller than JWT (no Base64 overhead)

### 2.2 CWT Claims

```cddl
; CWT Claims Set
CWT = {
    ? 1 : iss,      ; Issuer
    ? 2 : sub,      ; Subject
    ? 3 : aud,      ; Audience
    ? 4 : exp,      ; Expiration time
    ? 5 : nbf,      ; Not before
    ? 6 : iat,      ; Issued at
    ? 7 : cti,      ; CWT ID
    ? 8 : cnf,      ; Confirmation (key binding)
    ? 9 : scope,    ; Scope
    * tstr => any,  ; Custom claims
}

; Claim types
iss = tstr
sub = tstr / bstr
aud = tstr / [ * tstr ]
exp = uint
nbf = uint
iat = uint
cti = bstr
cnf = { ? "kid" => tstr, ? "jwk" => any }
scope = tstr / [ * tstr ]
```

### 2.3 CWT vs JWT Comparison

```
JWT (JSON Web Token):
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.
eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.
SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c

Total: ~175 bytes (Base64 encoded)

CWT (CBOR Web Token):
a3 02 6a 31 32 33 34 35 36 37 38 39 30
03 68 4a 6f 68 6e 20 44 6f 65
06 1a 5a 36 8c 1e
...signature

Total: ~85 bytes (raw CBOR) + COSE wrapper

Size reduction: ~50%
```

### 2.4 CWT with COSE

```cddl
; CWT encoded as COSE_Encrypt0
encrypted_cwt = COSE_Encrypt0 = [
    Headers,
    ciphertext: bstr,  ; Encrypted CWT claims
    tag: bstr          ; Authentication tag
]

; Protected headers for encryption
{
    1: kid,            ; Key ID
    4: alg,            ; Content encryption algorithm
    5: iv              ; Initialization vector (in unprotected)
}
```

### 2.5 Rust CWT Example

```rust
use cwt::{Claims, Cwt};
use chrono::{Utc, Duration};

// Create claims
let claims = Claims {
    issuer: Some("auth.example.com".to_string()),
    subject: Some("user123".to_string()),
    audience: Some(vec!["api.example.com".to_string()]),
    expiration: Some((Utc::now() + Duration::hours(1)).timestamp() as u64),
    issued_at: Some(Utc::now().timestamp() as u64),
    ..Default::default()
};

// Create CWT
let cwt = Cwt::new(claims)?;

// Sign with COSE
let protected = Headers {
    alg: Some(Algorithm::HS256),
    ..Default::default()
};

let signed_cwt = cosey::sign1(&protected, &cwt.to_cbor()?, &secret_key)?;

// Encode for transmission
let token = serde_cbor::to_vec(&signed_cwt)?;

// On server: verify and decode
let verified = cosey::verify(&token, &secret_key)?;
let claims: Claims = Cwt::from_cbor(&verified.payload)?;

// Validate claims
assert!(claims.is_valid()?);
assert_eq!(claims.subject, Some("user123".to_string()));
```

### 2.6 Use Cases

```
1. OAuth 2.0 for IoT
   - Compact access tokens
   - Constrained device support
   - ACE framework integration

2. Microservices Authentication
   - Inter-service tokens
   - Smaller than JWT
   - Faster parsing

3. Mobile Applications
   - Reduced bandwidth
   - Lower battery consumption
   - Faster validation
```

---

## 3. SenML: Sensor Measurement Lists

### 3.1 What is SenML?

**SenML (RFC 8428)** standardizes sensor data representation:
- Time-series measurements
- Unit normalization
- Device identification
- Efficient encoding

### 3.2 SenML Structure

```cddl
; SenML Pack (array of records)
SenML_Pack = [ * SenML_Record ]

; SenML Record
SenML_Record = {
    ? 0 => n,      ; Name (sensor identifier)
    ? 1 => u,      ; Unit (UCUM code)
    ? 2 => v,      ; Value (numeric)
    ? 3 => vs,     ; String value
    ? 4 => vb,     ; Boolean value
    ? 5 => vd,     ; Data value (base64)
    ? 6 => t,      ; Relative time
    ? 7 => ut,     ; Absolute time (Unix timestamp)
    ? 8 => bn,     ; Base name
    ? 9 => bt,     ; Base time
    * tstr => any, ; Extensions
}
```

### 3.3 SenML Example

```cddl
; Temperature sensor reading
[
    {
        bn: "urn:dev:ow:10e2073902001325",  ; Base name (device)
        bt: 1632844800,                     ; Base time
    },
    {
        n: "temp",                          ; Sensor name
        u: "Cel",                           ; Unit: Celsius
        v: 23.5,                            ; Value
        t: 0,                               ; Relative time (0 = base time)
    },
    {
        n: "temp",
        u: "Cel",
        v: 23.7,
        t: 60,                              ; 60 seconds after base
    }
]
```

### 3.4 CBOR Encoding

```
SenML Pack CBOR:
82                        ; array(2) - 2 records
a4                        ; map(4)
   62 62 6e               ; "bn"
   78 20 75 72 6e 3a 64 65 76 3a 6f 77 3a 31 30 65 32 30 37 33 39 30 32 30 30 31 33 32 35
   ...                    ; "urn:dev:ow:10e2073902001325"
   62 62 74               ; "bt"
   1b 00 00 00 01 63 2a 5a 00  ; 1632844800 (timestamp)

a4                        ; map(4)
   61 6e                  ; "n"
   64 74 65 6d 70         ; "temp"
   61 75                  ; "u"
   63 43 65 6c            ; "Cel"
   61 76                  ; "v"
   fb 40 37 ae 14 7a e1 47 ae  ; 23.5
   61 74                  ; "t"
   00                     ; 0
```

### 3.5 Rust SenML Example

```rust
use senml::{Record, Pack, Unit};

// Create sensor readings
let records = vec![
    Record {
        base_name: Some("urn:dev:ow:10e2073902001325".to_string()),
        base_time: Some(1632844800),
        name: Some("temp".to_string()),
        unit: Some(Unit::Celsius),
        value: Some(23.5),
        time: Some(0),
        ..Default::default()
    },
    Record {
        name: Some("temp".to_string()),
        unit: Some(Unit::Celsius),
        value: Some(23.7),
        time: Some(60),
        ..Default::default()
    },
];

// Encode as CBOR SenML
let pack = Pack(records);
let cbor = serde_cbor::to_vec(&pack)?;

// Send over CoAP
// POST coap://sensor.example.com/s
// Content-Format: application/senml+cbor
// Payload: <cbor bytes>
```

### 3.6 Use Cases

```
1. Smart Buildings
   - Temperature monitoring
   - HVAC optimization
   - Energy management

2. Industrial IoT
   - Machine sensors
   - Predictive maintenance
   - Quality control

3. Environmental Monitoring
   - Weather stations
   - Air quality sensors
   - Water quality monitoring
```

---

## 4. IoT Protocols

### 4.1 CoAP + CBOR

```
Constrained Application Protocol (CoAP) + CBOR:

Request:
GET coap://sensor.example.com/temperature
Accept: application/cbor

Response:
2.05 Content
Content-Format: application/cbor
Payload:
  a2                    ; map(2)
     64 74 65 6d 70     ; "temp"
     fb 40 37 ae 14 7a e1 47 ae  ; 23.5
     64 75 6e 69 74     ; "unit"
     63 43 65 6c        ; "Cel"
```

### 4.2 MQTT + CBOR

```
MQTT with CBOR payloads:

Publish:
Topic: home/sensor/living-room
Payload: <CBOR-encoded sensor data>
QoS: 1
Retain: true

CBOR payload structure:
{
    device_id: "sensor-001",
    timestamp: 1632844800,
    measurements: {
        temperature: 23.5,
        humidity: 45.2,
        co2: 420
    }
}
```

### 4.3 LwM2M with CBOR

```
Lightweight M2M (OMA Spec) uses CBOR:

Object: Device (Object ID 3)
Resource: Manufacturer (Resource ID 0)
Value: "Example Corp"

CBOR encoding:
83           ; array(3)
   03        ; Object ID
   00        ; Resource ID
   6c 45 78 61 6d 70 6c 65 20 43 6f 72 70  ; "Example Corp"
```

### 4.4 Matter Protocol

```
Matter (formerly CHIP) uses CBOR for:
- Device attestation
- Commissioning data
- Cluster attribute encoding

Example: Device attestation
{
    device_info: {
        vendor_id: 0x1234,
        product_id: 0x5678,
        serial_number: "ABC123"
    },
    attestation_signature: <COSE_Sign1>
}
```

---

## 5. WebAuthn Authenticator Data

### 5.1 WebAuthn Overview

**WebAuthn (FIDO2)** uses CBOR for authenticator data:
- Attestation objects
- Authenticator data structure
- Extension outputs

### 5.2 Attestation Object

```cddl
; Attestation Object (RFC 8152 COSE)
AttestationObject = {
    fmt: tstr,           ; Attestation format
    attStmt: AttStmt,    ; Attestation statement
    authData: bstr,      ; Authenticator data
}
```

### 5.3 Authenticator Data Structure

```
Authenticator Data (binary, not CBOR):
┌──────────────────────────────────────────┐
│ SHA-256 of RP ID (32 bytes)             │
├──────────────────────────────────────────┤
│ Flags (1 byte)                          │
├──────────────────────────────────────────┤
│ Sign Count (4 bytes)                    │
├──────────────────────────────────────────┤
│ Attested Credential Data (optional)     │
├──────────────────────────────────────────┤
│ Extensions (CBOR, optional)             │
└──────────────────────────────────────────┘
```

### 5.4 Extension Output (CBOR)

```cddl
; Extensions encoded as CBOR
Extensions = {
    ? "hmac-secret" => bool,
    ? "credProtect" => uint,
    ? "largeBlobKey" => bstr,
    * tstr => any
}

; Example extension output
{
    "hmac-secret": true,
    "credProtect": 2
}

CBOR:
a2                          ; map(2)
   6b 68 6d 61 63 2d 73 65 63 72 65 74  ; "hmac-secret"
   f5                      ; true
   69 63 72 65 64 50 72 6f 74 65 63 74  ; "credProtect"
   02                      ; 2
```

### 5.5 Rust WebAuthn Example

```rust
use webauthn_rs::{Webauthn, WebauthnConfig};

// Verify attestation
let config = WebauthnConfig { /* ... */ };
let webauthn = Webauthn::new(config)?;

// Parse attestation object
let attestation: AttestationObject = serde_cbor::from_slice(&attestation_bytes)?;

// Verify COSE signature
let verified = webauthn.verify_attestation(
    &attestation,
    &challenge,
    &origin,
    &rp_id
)?;

// Extract credential
let credential = verified.credential;
```

---

## 6. Blockchain: Cardano Ledger

### 6.1 Cardano CDDL

Cardano uses CDDL extensively for ledger types:

```cddl
; Transaction structure
Transaction = {
    inputs: [* TxIn],
    outputs: [* TxOut],
    fee: UInt,
    ? ttl: UInt,
    ? certificates: [* Certificate],
    ? withdrawals: Withdrawals,
    ? update: Update,
    ? auxiliaryDataHash: Hash32,
    ? validityIntervalStart: UInt
}

TxIn = [ Hash32, UInt ]
TxOut = [ address: Bytes, amount: UInt ]
```

### 6.2 Address Encoding

```cddl
; Cardano Address
Address = Bytes .size 28 .cbor (
    header: uint,
    payment: Credential,
    ? delegation: Credential
)

Credential = #6.24(bstr)  ; Tag 24: embedded CBOR
```

### 6.3 Rust Cardano Example

```rust
use pallas::ledger::primitives::alonzo::{Transaction, TransactionInput};

// Decode transaction
let tx: Transaction = minicbor::decode(&tx_bytes)?;

// Access transaction data
for input in tx.transaction_inputs {
    println!("Input: {:?}", input.transaction_id);
}

for output in tx.transaction_outputs {
    println!("Output: {} lovelace", output.amount.coin);
}

// Encode transaction
let encoded = minicbor::to_vec(&tx)?;
```

---

## 7. OSCORE: Object Security for Constrained Environments

### 7.1 What is OSCORE?

**OSCORE (RFC 8613)** provides end-to-end security for CoAP:
- Protects CoAP message payload and options
- Uses COSE for cryptography
- Works through proxies

### 7.2 OSCORE Message Structure

```cddl
OSCORE_Message = {
    flags: uint,
    ? kid: bstr,
    ? partial_iv: bstr,
    ciphertext: bstr  ; Encrypted CoAP message
}
```

### 7.3 OSCORE Flow

```
Client                          Server
  |                               |
  |--- OSCORE Request ----------->|
  |   [Protected: GET /temp]      |
  |                               |
  |<-- OSCORE Response -----------|
  |   [Protected: 23.5 Cel]       |
  |                               |

OSCORE protects:
- Payload
- Proxy-Unsafe options
- Message integrity
```

### 7.4 Rust OSCORE Example

```rust
use oscore::{OscoreContext, OscoreMessage};

// Create OSCORE context
let context = OscoreContext::new(
    master_secret,
    master_salt,
    sender_id,
    recipient_id
)?;

// Protect CoAP request
let plaintext = CoapMessage::get("/temperature");
let protected = context.protect(&plaintext)?;

// Encode as CBOR
let cbor = serde_cbor::to_vec(&protected)?;

// On server: unprotect
let received: OscoreMessage = serde_cbor::from_slice(&cbor)?;
let decrypted = context.unprotect(&received)?;
```

---

## Appendix A: Protocol Comparison

| Protocol | CBOR Usage | Standard | Primary Use |
|----------|------------|----------|-------------|
| COSE | Core format | RFC 8152 | Signatures/Encryption |
| CWT | Claims encoding | RFC 8392 | Authorization tokens |
| SenML | Record format | RFC 8428 | Sensor data |
| OSCORE | Message wrapper | RFC 8613 | CoAP security |
| WebAuthn | Extensions | W3C | Authentication |
| Cardano | Ledger types | CDDL spec | Blockchain |
| Matter | Attestation | CSA | Smart home |

---

*This document is a living textbook. Revisit sections as concepts become clearer through implementation. Next: [rust-revision.md](rust-revision.md)*
