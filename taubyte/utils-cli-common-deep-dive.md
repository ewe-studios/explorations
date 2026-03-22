# Utils & CLI-Common - Deep Dive Exploration

**Paths:**
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/utils/`
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/cli-common/`

---

## Executive Summary

**Utils** and **CLI-Common** are foundational libraries in the Taubyte ecosystem:

- **Utils** - General-purpose Go utilities for common operations (caching, UUID, filesystem, encoding)
- **CLI-Common** - Shared utilities for Tau CLI and Dream CLI (flags, prompts, validation)

Both libraries provide essential building blocks used across all Taubyte tools.

---

## Utils Library

**Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/utils/`

**Version:** 0.1.7

**Language:** Go

### Architecture Overview

```
utils/
├── bundle/            # Bundling utilities
├── cache/             # Caching with TTL
├── env/               # Environment handling
├── fs/                # Filesystem utilities
├── hex/               # Hex encoding
├── id/                # ID generation
├── logger/            # Logging utilities
├── maps/              # Map utilities
├── multihash/         # Multihash functions
├── network/           # Network utilities
├── path/              # Path utilities
├── rand/              # Random utilities
├── slices/            # Slice utilities
├── uri/               # URI handling
├── uuid/              # UUID utilities
├── x509/              # X.509 certificate handling
├── go.mod
├── go.sum
├── LICENSE
└── README.md
```

### Purpose

From README:
> This repository contains golang utilities for doing things such as:
> - Caching a map in memory with a TTL
> - Generating UUIDs
> - Filesystem utilities
> - Hex encoding/decoding and utilities
> - UUID utilities
> - Map display and utilities
> - Path utilities

---

## Utils Modules

### Cache Module (cache/)

In-memory caching with TTL (Time To Live):

```go
// Features:
// - TTL-based expiration
// - Automatic cleanup
// - Thread-safe operations
// - Generic type support
```

**Use Cases:**
- Session caching
- API response caching
- Temporary data storage

### UUID Module (uuid/)

UUID generation utilities:

```go
// Supported formats:
// - UUID v4 (random)
// - UUID v5 (namespace-based)
// - ULID (sortable)
```

### Filesystem Module (fs/)

Filesystem operations:

```go
// Features:
// - Atomic file writes
// - Directory utilities
// - Permission handling
// - Temp file management
```

### Hex Module (hex/)

Hex encoding/decoding:

```go
// Functions:
// - Encode/decode
// - Validation
// - Format conversion
```

### Maps Module (maps/)

Map utilities:

```go
// Features:
// - Map display/formatting
// - Deep copy
// - Merge operations
// - Key/value transformations
```

### Path Module (path/)

Path manipulation:

```go
// Functions:
// - Path normalization
// - Home directory expansion
// - Relative/absolute conversion
```

### Slices Module (slices/)

Slice operations:

```go
// Functions:
// - Contains checks
// - Filter operations
// - Map operations
// - Unique values
```

Example from tau-cli imports:
```go
import slices "github.com/taubyte/utils/slices/string"

// Check if slice contains value
if !slices.Contains(options, name) {
    return loginI18n.DoesNotExistIn(name, options)
}
```

### X.509 Module (x509/)

X.509 certificate handling:

```go
// File: x509/cert.go
// Functions:
// - Certificate parsing
// - Key extraction
// - Validation
```

### Multihash Module (multihash/)

Multihash generation:

```go
// File: multihash/generate.go
// Supported algorithms:
// - SHA1
// - SHA256
// - Blake2b
// - Blake3
```

### Logger Module (logger/)

Logging utilities:

```go
// Features:
// - Structured logging
// - Log levels
// - Output formatting
```

### Network Module (network/)

Network utilities:

```go
// Functions:
// - Port checking
// - Address parsing
// - Network interface discovery
```

---

## CLI-Common Library

**Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/cli-common/`

**Version:** Latest

**Language:** Go

### Architecture Overview

```
cli-common/
├── env/               # Environment handling
├── flags/             # Flag definitions
├── i18n/              # Internationalization
├── prompts/           # Interactive prompts
├── singletons/        # Singleton patterns
├── states/            # State management
├── validate/          # Input validation
├── go.mod
├── go.sum
├── LICENSE
└── README.md
```

### Purpose

From README:
> `cli-common` is used by `dreamland` and `tau-cli`.

---

## CLI-Common Modules

### Flags Module (flags/)

Comprehensive flag definitions:

```
flags/
├── bool.go           # Boolean flags
├── bool_test.go
├── color.go          # Color flags
├── combine.go        # Flag combination
├── helpers.go        # Helper functions
└── types.go          # Type definitions
```

**Boolean Flags (bool.go):**
```go
// Common boolean flags:
// - yes/no
// - force
// - dry-run
// - quiet
// - verbose
```

**Flag Combination:**
```go
func Combine(flags ...cli.Flag) []cli.Flag {
    // Combine multiple flag definitions
    // Handle duplicates
    // Return unified flag list
}
```

### Validate Module (validate/)

Input validation utilities:

```
validate/
├── bool_helpers.go
├── constants.go
├── device.go
├── flag.go
├── fqdn.go
├── helpers.go
├── match.go
├── min_max.go
├── regex.go
├── timeout.go
└── types.go
```

**Validations:**
- FQDN validation
- Regex pattern matching
- Min/max value checks
- Timeout validation
- Device detection
- Variable name validation

### Environment Module (env/)

Environment variable handling:

```go
// Functions:
// - Get environment variables
// - Set with defaults
// - Type conversion
// - Validation
```

### Internationalization (i18n/)

Multi-language support:

```go
// Features:
// - Message templates
// - Language selection
// - Locale-specific formatting
```

### Prompts Module (prompts/)

Interactive prompts:

```go
// Prompt types:
// - Text input
// - Password input
// - Selection (single/multiple)
// - Confirmation
// - Multi-line input
```

### Singletons Module (singletons/)

Singleton pattern implementations:

```go
// Common singletons:
// - Config instance
// - Logger instance
// - HTTP client
```

### States Module (states/)

State management:

```go
// State types:
// - Session state
// - Project state
// - Application state
// - Auth state
```

---

## Integration Patterns

### Utils in Tau CLI

```go
import (
    slices "github.com/taubyte/utils/slices/string"
    // ... other utils
)

// Check if profile exists
if !slices.Contains(options, name) {
    return loginI18n.DoesNotExistIn(name, options)
}
```

### CLI-Common in Tau CLI

```go
import (
    "github.com/taubyte/tau-cli/flags"
    "github.com/taubyte/tau-cli/validate"
    // ...
)

// Use common flags
var Command = &cli.Command{
    Flags: flags.Combine(
        flags.Name,
        flags.Yes,
    ),
}
```

---

## Common Patterns

### Flag Definition Pattern

```go
var Name = &cli.StringFlag{
    Name:    "name",
    Aliases: []string{"n"},
    Usage:   "Name of the resource",
}
```

### Validation Pattern

```go
func ValidateFQDN(fqdn string) error {
    if !isValidFQDN(fqdn) {
        return errors.New("invalid FQDN")
    }
    return nil
}
```

### Cache Pattern

```go
cache := cache.NewTTLCache(
    cache.WithTTL(5*time.Minute),
    cache.WithCleanup(1*time.Minute),
)

cache.Set("key", value)
value, ok := cache.Get("key")
```

---

## Dependencies

### Utils Dependencies

```go
require (
    // Core dependencies via other taubyte packages
)
```

### CLI-Common Dependencies

```go
require (
    github.com/urfave/cli/v2  // CLI framework
    // Shared with tau-cli
)
```

---

## Testing

### Utils Tests

```bash
# Run all utils tests
go test -v ./...
```

### CLI-Common Tests

```bash
# Run validation tests
go test -v ./validate/...

# Run flag tests
go test -v ./flags/...
```

---

## Use Cases

### Utils Library

1. **Caching**
   ```go
   cache := utils.cache.NewTTLCache()
   cache.Set("session", userData, 5*time.Minute)
   ```

2. **UUID Generation**
   ```go
   id := utils.uuid.New()
   ```

3. **Path Handling**
   ```go
   path := utils.path.ExpandHome("~/.config/tau")
   ```

### CLI-Common Library

1. **Flag Definition**
   ```go
   flags := cli_common.flags.Combine(
       cli_common.flags.Name,
       cli_common.flags.Yes,
   )
   ```

2. **Input Validation**
   ```go
   err := cli_common.validate.FQDN(domain)
   ```

3. **Interactive Prompts**
   ```go
   name, err := prompts.Text("Enter name:")
   ```

---

## Summary

**Utils Library:**
- General-purpose Go utilities
- Caching, UUID, filesystem, encoding
- Used across all Taubyte projects

**CLI-Common Library:**
- Shared CLI infrastructure
- Flags, validation, prompts
- Used by tau-cli and dream

**Key Features:**
- Modular design
- Extensive testing
- Consistent APIs
- Well-documented
