---
title: "CBOR Schema and Validation Deep Dive"
subtitle: "Complete exploration of CDDL, schema validation, constraints, and error handling"
based_on: "RFC 8610 - CDDL (Concise Data Definition Language)"
level: "Intermediate - Requires CBOR data model knowledge"
---

# CBOR Schema and Validation Deep Dive

## Table of Contents

1. [What is CDDL?](#1-what-is-cddl)
2. [CDDL Syntax Fundamentals](#2-cddl-syntax-fundamentals)
3. [Type Definitions](#3-type-definitions)
4. [Group and Map Definitions](#4-group-and-map-definitions)
5. [Array and Choice Types](#5-array-and-choice-types)
6. [Tags and Constraints](#6-tags-and-constraints)
7. [Validation Strategies](#7-validation-strategies)
8. [Error Handling](#8-error-handling)
9. [Rust CDDL Integration](#9-rust-cddl-integration)

---

## 1. What is CDDL?

### 1.1 CDDL Overview

**CDDL (Concise Data Definition Language)** is defined in [RFC 8610](https://www.rfc-editor.org/rfc/rfc8610.html). It provides:

- Schema definition for CBOR data structures
- Human-readable syntax similar to ABNF
- Type constraints and validation rules
- Documentation embedded in schema

### 1.2 Why CDDL Matters

```
Without CDDL:
- CBOR is schemaless (like JSON)
- No formal type checking
- Documentation separate from validation

With CDDL:
- Formal type definitions
- Machine-readable validation
- Self-documenting schemas
- Protocol specification
```

### 1.3 CDDL vs Other Schema Languages

| Schema Language | For Format | Complexity | CDDL Equivalent |
|-----------------|------------|------------|-----------------|
| JSON Schema | JSON | High | CDDL (simpler) |
| .proto | Protobuf | Medium | CDDL (text-based) |
| XML Schema | XML | Very High | CDDL (much simpler) |

### 1.4 Real-World CDDL Usage

```
COSE (RFC 8152):
COSE_Sign1 = <<
    Headers,
    payload: bstr
>>

CWT (RFC 8392):
CWT = {
    ? 1 : iss,
    ? 2 : sub,
    ? 3 : aud,
    ? 4 : exp,
    ? 5 : nbf,
    ? 6 : iat,
    ? 7 : cti,
    ? 8 : cnf,
    * tstr => any
}

Cardano Ledger:
Transaction = {
    inputs: [* TxIn],
    outputs: [* TxOut],
    fee: UInt,
    ttl: optional UInt
}
```

---

## 2. CDDL Syntax Fundamentals

### 2.1 Basic Structure

```cddl
; This is a comment
; CDDL uses ; for comments (not // or #)

; Type definition: name = type
MyType = int

; String definition
Name = tstr

; Bytes definition
Hash = bytes
```

### 2.2 Basic Types

```cddl
; Integer types
int          ; Any integer (signed or unsigned)
uint         ; Unsigned integer (major type 0)
nint         ; Negative integer (major type 1)

; String types
tstr         ; Text string (UTF-8, major type 3)
bstr         ; Byte string (major type 2)

; Boolean and null
bool         ; true or false
null         ; null value

; Any type
any          ; Any CBOR value
```

### 2.3 Range Constraints

```cddl
; Range specifications
age = uint .ge 0 .le 150      ; 0 to 150
port = uint .range 1..65535   ; 1 to 65535
positive = uint .gt 0         ; Greater than 0
negative = nint .lt 0         ; Less than 0

; Size constraints for bytes/strings
short_name = tstr .size 0..32     ; 0-32 characters
fixed_key = bstr .size 32         ; Exactly 32 bytes
```

### 2.4 Default Values

```cddl
; Default values for optional fields
status = int .default 0

; Enumerations with defaults
priority = &(
    low: 0 .default,
    medium: 1,
    high: 2
)
```

---

## 3. Type Definitions

### 3.1 Simple Type Aliases

```cddl
; Basic aliases
UserId = uint
UserName = tstr
UserEmail = tstr
IsActive = bool

; Composite types
User = {
    id: UserId,
    name: UserName,
    email: UserEmail,
    active: IsActive
}
```

### 3.2 Enumerations

```cddl
; Enumerated values
Status = &(
    pending: 0,
    active: 1,
    suspended: 2,
    deleted: 3
)

; String enums
LogLevel = &(
    debug: "DEBUG",
    info: "INFO",
    warn: "WARN",
    error: "ERROR"
)

; Using enums in structures
LogEntry = {
    level: LogLevel,
    message: tstr,
    timestamp: uint
}
```

### 3.3 Tagged Types

```cddl
; Standard tags
Timestamp = #6.1(uint)      ; Tag 1: Epoch time
DateTime = #6.0(tstr)       ; Tag 0: Date/time string
PositiveBigNum = #6.2(bstr) ; Tag 2: Positive bignum
NegativeBigNum = #6.3(bstr) ; Tag 3: Negative bignum

; Custom tags
UUID = #6.37(bstr .size 16) ; Tag 37: UUID
Base64Data = #6.7(bstr)     ; Tag 7: Base64 encoded

; Using tagged types
Document = {
    id: UUID,
    created: Timestamp,
    content: Base64Data
}
```

### 3.4 Type References

```cddl
; Forward references work
Address = {
    street: tstr,
    city: CityName,    ; Reference to type defined later
    zip: uint
}

CityName = tstr        ; Definition after use

; Recursive types
TreeNode = {
    value: int,
    children: [ * TreeNode ]  ; Self-reference
}
```

---

## 4. Group and Map Definitions

### 4.1 Map Structure

```cddl
; Basic map
Person = {
    name: tstr,
    age: uint,
    email: tstr
}

; Encodes as CBOR map with 3 key-value pairs
```

### 4.2 Optional Fields

```cddl
; Optional with ?
OptionalPerson = {
    name: tstr,
    ? age: uint,        ; Optional field
    ? email: tstr,      ; Optional field
    ? phone: tstr       ; Optional field
}

; Optional with default
PersonWithDefault = {
    name: tstr,
    ? status: Status .default "active",
}
```

### 4.3 Key Types

```cddl
; String keys (most common)
StringKeyMap = {
    * tstr => any    ; Any number of string => any pairs
}

; Integer keys
IntegerKeyMap = {
    * uint => tstr   ; Any number of uint => tstr pairs
}

; Mixed keys
MixedMap = {
    ? 1 => tstr,     ; Key is integer 1
    ? "name" => tstr, ; Key is string "name"
    * tstr => any    ; Any other string keys
}
```

### 4.4 Repetition Groups

```cddl
; Zero or more (* prefix)
ArrayAny = [ * any ]          ; Array of any values
ArrayInt = [ * int ]          ; Array of integers

; One or more (+ prefix)
NonEmptyArray = [ + int ]     ; At least one integer

; Exact count
FixedSize = [ 3 int ]         ; Exactly 3 integers

; Range
BoundedArray = [ 1*5 int ]    ; 1 to 5 integers

; With labels
LabeledArray = [ * label: int ]  ; Labeled entries
```

### 4.5 Cut Operator (,)

```cddl
; Without cut (keys can be in any order)
FlexibleMap = {
    ? a: int,
    ? b: int,
}

; With cut (keys before , must come first)
OrderedMap = {
    ? a: int,
    ,
    ? b: int,
}
; If 'b' is present, 'a' must come before it
```

---

## 5. Array and Choice Types

### 5.1 Array Types

```cddl
; Fixed-size array
Point2D = [ int, int ]        ; [x, y]
Point3D = [ int, int, int ]   ; [x, y, z]

; Variable-size array
IntList = [ * int ]           ; Any number of integers
PointList = [ * Point2D ]     ; Array of points

; Mixed-type array
MixedTuple = [ tstr, uint, bool ]  ; [name, id, active]

; Array with optional elements
OptionalArray = [ int, ? int, ? int ]  ; 1-3 integers
```

### 5.2 Choice Types (Union)

```cddl
; Basic choice
Number = int / float

; Multiple choices
Value = int / tstr / bool / null

; Named choices
Shape = circle / rectangle / triangle
circle = [ "circle", point: Point2D, radius: float ]
rectangle = [ "rectangle", origin: Point2D, size: Size ]
triangle = [ "triangle", a: Point2D, b: Point2D, c: Point2D ]

; Tagged choice
TaggedValue = #6.0(tstr) / #6.1(uint) / #6.2(bstr)
```

### 5.3 Group Choice

```cddl
; Choice between groups
Contact = email_contact / phone_contact

email_contact = (
    type: "email",
    address: tstr
)

phone_contact = (
    type: "phone",
    number: tstr
)

; Encodes as map with type discriminator
```

### 5.4 Unwrapping Groups

```cddl
; Unwrap group into parent
BaseFields = (
    id: uint,
    created: uint
)

User = {
    BaseFields,           ; Unwrapped
    name: tstr,
    email: tstr
}

; Equivalent to:
; User = { id: uint, created: uint, name: tstr, email: tstr }
```

---

## 6. Tags and Constraints

### 6.1 Tag Definitions

```cddl
; Standard tags
#6.0(tstr)   ; Standard date/time string
#6.1(uint)   ; Epoch-based date/time
#6.2(bstr)   ; Positive bignum
#6.3(bstr)   ; Negative bignum
#6.6(tstr)   ; Base64url encoded
#6.7(tstr)   ; Base64 encoded
#6.24(any)   ; Encoded CBOR

; Custom tags
#6.100(tstr)  ; Tag 100 (application-specific)
#6.1000(uint) ; Tag 1000
```

### 6.2 Content Constraints

```cddl
; Regex patterns
Email = tstr .regexp "^[^@]+@[^@]+\\.[^@]+$"
Phone = tstr .regexp "^\\+[0-9]{1,3}-[0-9]{3,}-[0-9]{3,}$"

; Range with units
Temperature = float .range -273.15..1000.0
Percentage = uint .range 0..100

; Size constraints
SHA256 = bstr .size 32
MD5 = bstr .size 16
ShortString = tstr .size 0..256
```

### 6.3 Occurrence Constraints

```cddl
; Map cardinality
Config = {
    * tstr => any    ; Any number of pairs
}

FixedConfig = {
    3 tstr => any    ; Exactly 3 pairs
}

BoundedConfig = {
    1*5 tstr => any  ; 1 to 5 pairs
}

; Array cardinality
Coordinates = 2 int      ; Exactly 2
Polygon = 3* int         ; 3 or more
WeekDays = 7 int         ; Exactly 7
```

### 6.4 Uniqueness Constraints

```cddl
; Unique array elements
UniqueIds = [ * $uint ]    ; $ ensures uniqueness

; Unique map keys (default in CBOR)
UniqueMap = { * tstr => any }  ; Keys are always unique
```

---

## 7. Validation Strategies

### 7.1 Schema Validation Flow

```
┌─────────────────────────────────────────────────────────┐
│                  CDDL Validation Flow                    │
├─────────────────────────────────────────────────────────┤
│ 1. Parse CDDL schema → AST                              │
│ 2. Parse CBOR bytes → Value                             │
│ 3. Validate Value against AST                           │
│ 4. Report errors or return validated data               │
└─────────────────────────────────────────────────────────┘
```

### 7.2 Validation Levels

```cddl
; Level 1: Type checking only
User = {
    id: uint,
    name: tstr
}

; Level 2: With constraints
User = {
    id: uint .ge 1,
    name: tstr .size 1..100
}

; Level 3: With semantic validation
User = {
    id: uint .ge 1,
    name: tstr .regexp "^[A-Z][a-z]+$",
    email: tstr .regexp "^[^@]+@[^@]+\\.[^@]+$"
}
```

### 7.3 Partial Validation

```rust
// Validate only specific fields
fn validate_id_only(cbor: &[u8], schema: &CddlSchema) -> Result<u64> {
    let value = serde_cbor::from_slice(cbor)?;
    schema.validate_partial(&value, &["id"])?;
    extract_id(&value)
}
```

### 7.4 Validation Performance

```
┌─────────────────────────────────────────────────────────┐
│              Validation Performance Tips                 │
├─────────────────────────────────────────────────────────┤
│ 1. Check required fields first                          │
│ 2. Validate types before constraints                    │
│ 3. Short-circuit on first error (or collect all)        │
│ 4. Cache compiled schemas                               │
│ 5. Use zero-copy validation where possible              │
└─────────────────────────────────────────────────────────┘
```

---

## 8. Error Handling

### 8.1 Error Categories

```rust
pub enum CddlError {
    // Schema errors
    SchemaParseError(String),
    UnknownType(String),
    CircularReference(String),

    // Validation errors
    TypeMismatch { expected: String, found: String },
    MissingField(String),
    ExtraField(String),
    ConstraintViolation { field: String, constraint: String },

    // CBOR parsing errors
    CborParseError(serde_cbor::Error),

    // I/O errors
    IoError(std::io::Error),
}
```

### 8.2 Error Reporting

```rust
// Detailed error with path
pub struct ValidationError {
    pub path: Vec<String>,      // JSONPath-like: ["users", 0, "name"]
    pub message: String,
    pub expected: String,
    pub actual: String,
}

// Example output
/*
Validation failed at path .users[0].name:
  Expected: tstr .size 1..100
  Found: empty string (size 0)
*/
```

### 8.3 Error Recovery

```rust
// Continue validation after error
pub struct ValidationConfig {
    pub fail_fast: bool,        // Stop on first error
    pub collect_all: bool,      // Collect all errors
    pub allow_extra_fields: bool,
    pub allow_missing_optional: bool,
}

// Usage
let config = ValidationConfig {
    fail_fast: false,
    collect_all: true,
    ..Default::default()
};

let errors = schema.validate_with_config(&value, &config);
// Returns all errors, not just first
```

---

## 9. Rust CDDL Integration

### 9.1 CDDL Parsing in Rust

```rust
use cddl::CDDL;

fn parse_schema(cddl_text: &str) -> Result<CDDL, cddl::Error> {
    let cddl = CDDL::from_str(cddl_text)?;
    Ok(cddl)
}

// Example schema
const USER_SCHEMA: &str = r#"
    User = {
        id: uint .ge 1,
        name: tstr .size 1..100,
        ? email: tstr,
    }
"#;
```

### 9.2 Validation with cddl Crate

```rust
use cddl::{cddl_from_str, validate_cbor_bytes};

fn validate_user(cbor: &[u8]) -> Result<(), cddl::Error> {
    let cddl = cddl_from_str(USER_SCHEMA)?;
    validate_cbor_bytes("User", &cddl, cbor)?;
    Ok(())
}

// Usage
let user = User { id: 1, name: "Alice".to_string(), email: None };
let cbor = serde_cbor::to_vec(&user)?;
validate_user(&cbor).expect("Valid user");
```

### 9.3 Custom Validation

```rust
use cddl::ast::*;

// Custom validator for specific constraints
pub struct CustomValidator {
    pub check_email_format: bool,
    pub check_age_range: bool,
}

impl CustomValidator {
    pub fn validate(&self, value: &Value, schema: &Type) -> Result<()> {
        // Custom validation logic
        if self.check_email_format {
            if let Value::Text(email) = value {
                if !self.is_valid_email(email) {
                    return Err(Error::InvalidEmail(email.clone()));
                }
            }
        }
        Ok(())
    }

    fn is_valid_email(&self, email: &str) -> bool {
        email.contains('@') && email.contains('.')
    }
}
```

### 9.4 Schema Generation from Rust Types

```rust
use serde::Serialize;

// Derive CDDL from Rust types
#[derive(Serialize)]
#[cddl_schema(name = "User")]
struct User {
    id: u64,
    name: String,
    email: Option<String>,
}

// Generates:
// User = {
//     id: uint,
//     name: tstr,
//     ? email: tstr,
// }
```

### 9.5 Complete Example: COSE Schema

```rust
// COSE Sign1 schema
const COSE_SIGN1_SCHEMA: &str = r#"
    COSE_Sign1 = [
        Headers,
        payload: bstr
    ]

    Headers = {
        ? 1 => int,          ; kid
        ? 4 => bstr,         ; alg
        * tstr => any,
    }

    COSE_Algorithm = int
"#;

fn validate_cose_sign1(cbor: &[u8]) -> Result<(), cddl::Error> {
    let cddl = cddl::cddl_from_str(COSE_SIGN1_SCHEMA)?;
    cddl::validate_cbor_bytes("COSE_Sign1", &cddl, cbor)?;
    Ok(())
}
```

---

## Appendix A: CDDL Quick Reference

```
Types:
int       - Integer (signed or unsigned)
uint      - Unsigned integer
nint      - Negative integer
tstr      - Text string (UTF-8)
bstr      - Byte string
bool      - Boolean (true/false)
null      - Null value
any       - Any CBOR value
float     - Floating point

Tags:
#6.n(T)   - Tag n containing type T
#6.0(tstr) - Date/time string
#6.1(uint) - Epoch timestamp

Constraints:
.ge N     - Greater than or equal
.gt N     - Greater than
.le N     - Less than or equal
.lt N     - Less than
.range N..M - Range N to M
.size N   - Exact size
.size N..M - Size range
.regexp "pattern" - Regex match

Occurrence:
? field: T  - Optional field
* tstr => T - Zero or more pairs
+ T       - One or more
N T       - Exactly N items
N*M T     - N to M items

Groups:
( a: T, b: T ) - Group (unwrapped)
{ a: T, b: T } - Map
[ T, T ]      - Array
T / U         - Choice (union)
```

---

*This document is a living textbook. Revisit sections as concepts become clearer through implementation. Next: [03-performance-comparison-deep-dive.md](03-performance-comparison-deep-dive.md)*
