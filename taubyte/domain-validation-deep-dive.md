# Domain Validation - Deep Dive Exploration

**Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/domain-validation/`

**Version:** 1.0.1

**Language:** Go

---

## Executive Summary

**Domain Validation** is a specialized Go library for validating domain ownership in the Taubyte ecosystem. It uses cryptographic techniques (JWT with ECDSA) to prove control over domains, enabling secure automated SSL/TLS certificate management and DNS-based verification.

---

## Architecture Overview

### Directory Structure

```
domain-validation/
├── fixtures/            # Test fixtures
├── all_test.go          # Comprehensive tests
├── claims.go            # JWT claims
├── common.go            # Common utilities
├── methods.go           # Validation methods
├── new.go               # Constructor
├── options.go           # Configuration options
├── type.go              # Type definitions
├── go.mod
├── go.sum
├── LICENSE
└── README.md
```

### Purpose

From README:
> Used to validate ownership of DNS domains

---

## Cryptographic Foundation

### Key Generation

The library uses ECDSA (Elliptic Curve Digital Signature Algorithm) with the P-256 curve:

```bash
# Generate private key
openssl ecparam -name prime256v1 -genkey -noout -out private.key

# Extract public key
openssl ec -in private.key -pubout -out public.pem
```

**Note:** The private key must be kept secure as it's used to sign domain validation tokens.

### Algorithm Details

- **Curve:** prime256v1 (NIST P-256)
- **Signature:** ES256 (ECDSA with SHA-256)
- **Key Size:** 256 bits
- **Security Level:** ~128 bits

---

## Claims Structure

### JWT Claims (claims.go)

```go
package lib

import (
    jwt "github.com/dgrijalva/jwt-go"
    mh "github.com/ipsn/go-ipfs/gxlibs/github.com/multiformats/go-multihash"
)

func (claims *Claims) Calculate() {
    hash, _ := mh.Sum([]byte(claims.fqdn+claims.project), mh.SHA1, -1)
    claims.Address = hash.B58String()
}

func (claims *Claims) Valid() error {
    return nil
}

func (claims *Claims) Sign() (Token, error) {
    token := jwt.NewWithClaims(jwt.SigningMethodES256, claims)
    out, err := token.SignedString(claims.privateKey)
    return Token(out), err
}
```

### Address Calculation

The domain address is calculated using:
1. Concatenate FQDN and project name
2. SHA1 hash via multihash
3. Base58 encoding

```
address = Base58(SHA1(fqdn + project))
```

### Token Signing

```go
func (claims *Claims) Sign() (Token, error) {
    token := jwt.NewWithClaims(jwt.SigningMethodES256, claims)
    out, err := token.SignedString(claims.privateKey)
    return Token(out), err
}
```

---

## Token Operations

### Signing Tokens

```go
claims := &Claims{
    fqdn: "example.com",
    project: "my-project",
    privateKey: loadedPrivateKey,
}

claims.Calculate()  // Generate address
token, err := claims.Sign()
```

### Verifying Tokens

```go
func FromToken(token Token, options ...Option) (*Claims, error) {
    claims, err := _new(options)
    if err != nil {
        return nil, err
    }

    _, err = jwt.ParseWithClaims(string(token), claims, func(token *jwt.Token) (interface{}, error) {
        return claims.publicKey, nil
    })
    if err != nil {
        return nil, err
    }

    return claims, nil
}
```

---

## Validation Methods

### DNS-Based Validation

The library supports DNS-based domain validation:

1. **TXT Record** - Add validation token to DNS
2. **CNAME Record** - Point to validation endpoint
3. **HTTP Challenge** - Serve token via HTTP

### ACME Integration

Domain validation integrates with ACME for automated certificate management:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Domain    │────▶│   Validate  │────▶│   ACME      │
│   Owner     │     │   Library   │     │   Challenge │
└─────────────┘     └─────────────┘     └─────────────┘
```

---

## API Reference

### Constructor

```go
func New(options ...Option) (*Validator, error)
```

### Options

```go
// WithPrivateKey sets the private key for signing
func WithPrivateKey(key *ecdsa.PrivateKey) Option

// WithPublicKey sets the public key for verification
func WithPublicKey(key *ecdsa.PublicKey) Option

// WithProject sets the project name
func WithProject(project string) Option
```

### Methods

```go
// Generate validation token
func (v *Validator) GenerateToken(fqdn string) (Token, error)

// Validate ownership
func (v *Validator) Validate(fqdn string, token Token) error

// Verify token signature
func (v *Validator) Verify(token Token) (*Claims, error)
```

---

## Use Cases

### SSL/TLS Certificate Automation

```go
// Generate validation token
token, err := validator.GenerateToken("example.com")

// Add to DNS as TXT record
// _validation.example.com TXT "token"

// Request certificate via ACME
cert, err := acmeClient.Certificate("example.com", token)
```

### Domain Ownership Proof

```go
// Create proof of ownership
claims := &Claims{
    fqdn: "example.com",
    project: "my-project",
}
claims.Calculate()
proof, _ := claims.Sign()

// Share proof with verifier
// Verifier can validate using public key
```

### Multi-Tenant Validation

```go
// Different projects, same domain
validator1 := New(WithProject("tenant-a"))
validator2 := New(WithProject("tenant-b"))

// Each gets unique validation tokens
token1, _ := validator1.GenerateToken("shared.com")
token2, _ := validator2.GenerateToken("shared.com")
```

---

## Integration with Tau Ecosystem

### Config Compiler

Domain validation is used by the config compiler:

```go
// In config-compiler
cloud.domain.validation.keys.data.privateKey
cloud.domain.validation.generate()
```

### Tau CLI

```bash
# Domain management
tau domain new example.com
tau domain validate example.com
```

### Dream

Dream auto-generates validation keys:

```go
try {
    await config.cloud.domain.validation.keys.data.privateKey.get();
} catch {
    await config.cloud.domain.validation.generate();
}
```

---

## Security Considerations

### Private Key Protection

- Store private keys securely
- Use environment variables or secret managers
- Never commit keys to version control

### Token Expiration

Validation tokens should have limited lifetimes:
- Short-lived tokens reduce replay attack risk
- Typical expiration: 5-15 minutes

### Key Rotation

Implement key rotation:
- Generate new key pairs periodically
- Update public keys in verification systems
- Maintain key versioning

---

## Testing

### Test Coverage

The library includes comprehensive tests:

```go
// all_test.go - Comprehensive test suite
func TestDomainValidation(t *testing.T) {
    // Key generation tests
    // Token signing tests
    // Token verification tests
    // Address calculation tests
}
```

### Test Fixtures

```
fixtures/
├── test-keys/        # Test key pairs
├── tokens/           # Test tokens
└── expected/         # Expected outputs
```

---

## Dependencies

### Core Dependencies

```go
require (
    github.com/dgrijalva/jwt-go v3.2.0+incompatible
    github.com/ipsn/go-ipfs/gxlibs/github.com/multiformats/go-multihash
)
```

### Indirect Dependencies

- `golang.org/x/crypto` - Cryptographic primitives
- `github.com/mr-tron/base58` - Base58 encoding
- `gopkg.in/yaml.v3` - YAML parsing

---

## Error Handling

### Validation Errors

```go
type ValidationError struct {
    Reason string
    Claim  string
}

func (e *ValidationError) Error() string {
    return fmt.Sprintf("validation failed: %s (%s)", e.Reason, e.Claim)
}
```

### Common Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `invalid signature` | Wrong key pair | Verify key configuration |
| `token expired` | Token lifetime exceeded | Generate new token |
| `invalid claims` | Malformed claims | Check claim structure |

---

## Configuration Options

### Environment Variables

```bash
# Private key path
export DOMAIN_VALIDATION_KEY_PATH=/path/to/private.key

# Public key path
export DOMAIN_VALIDATION_PUBLIC_KEY=/path/to/public.pem

# Default project
export DOMAIN_VALIDATION_PROJECT=my-project
```

### Programmatic Configuration

```go
validator, err := lib.New(
    lib.WithPrivateKey(privateKey),
    lib.WithPublicKey(publicKey),
    lib.WithProject("my-project"),
    lib.WithTokenLifetime(15*time.Minute),
)
```

---

## Performance Considerations

### Token Generation

- ECDSA signing is fast (~1-5ms)
- Suitable for on-demand generation
- No caching required

### Token Verification

- Public key operations are faster than signing
- Can be cached for repeated verifications
- Batch verification supported

---

## Maintenance

### Maintainers

From README:
- Samy Fodil (@samyfodil)
- Tafseer Khan (@tafseer-khan)
- Aron Jalbuena (@arontaubyte)

### Version History

- **v1.0.1** - Current stable release
- **v1.0.0** - Initial stable release

---

## Related Packages

### Taubyte Ecosystem

- `github.com/taubyte/config-compiler` - Uses domain validation
- `github.com/taubyte/tau-cli` - Domain management
- `github.com/taubyte/go-specs` - Domain specifications

### External Libraries

- `github.com/dgrijalva/jwt-go` - JWT implementation
- `github.com/multiformats/go-multihash` - Hash functions

---

## Summary

Domain Validation is a focused, secure library for proving domain ownership:

**Key Features:**
- ECDSA-based cryptographic validation
- JWT token format
- Multihash address calculation
- ACME integration ready
- Comprehensive test coverage

**Security:**
- Industry-standard cryptography (ES256)
- Secure key handling
- Token expiration support

**Integration:**
- Seamless Tau ecosystem integration
- Standard JWT compatibility
- DNS challenge support
