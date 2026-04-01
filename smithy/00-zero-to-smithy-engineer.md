---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/smithy
explored_at: 2026-03-29
prerequisites: Basic programming knowledge, API familiarity
---

# Zero to Smithy Engineer - Complete Fundamentals

## Table of Contents

1. [What is Smithy?](#what-is-smithy)
2. [Why Smithy?](#why-smithy)
3. [Installation](#installation)
4. [Your First Model](#your-first-model)
5. [Building Models](#building-models)
6. [Code Generation](#code-generation)
7. [Protocol Support](#protocol-support)
8. [AWS SDK Generation](#aws-sdk-generation)

## What is Smithy?

Smithy is an **Interface Definition Language (IDL)** for defining APIs and services. Think of it as a universal language for describing what your API does, independent of how it's implemented.

### The Problem Smithy Solves

**Without Smithy:**
```
REST API → Write OpenAPI spec → Generate TypeScript client
              ↓
         Need Python client? Write manually
              ↓
         Need Rust client? Write manually
              ↓
         Documentation? Write separately
```

**With Smithy:**
```
Smithy Model → Generate TypeScript, Python, Rust, Java clients
                Generate documentation automatically
                Generate server stubs
                All from ONE definition
```

### Key Concepts

| Term | Definition |
|------|------------|
| **IDL** | Interface Definition Language - describes API contracts |
| **Shape** | Type definition (string, integer, structure, etc.) |
| **Service** | Collection of operations and resources |
| **Resource** | RESTful entity with CRUD operations |
| **Operation** | RPC-style function call |
| **Trait** | Metadata/constraints applied to shapes |

## Why Smithy?

### Benefits

1. **Single Source of Truth**: One model, many outputs
2. **Type Safety**: Strong typing across all languages
3. **Protocol Flexibility**: REST, JSON, RPC, GraphQL from same model
4. **Documentation First**: Docs generated from model
5. **Validation**: Compile-time model validation
6. **Extensibility**: Custom traits for domain rules

### When to Use Smithy

**Good fit:**
- Multi-language SDK generation
- AWS service APIs
- Protocol-agnostic service design
- Large-scale API ecosystems
- Documentation-driven development

**Overkill for:**
- Simple single-language APIs
- Quick prototypes
- Internal tools with no SDK needs

## Installation

### Smithy CLI

```bash
# Via npm (recommended)
npm install -g @smithy/cli

# Verify installation
smithy --version
```

### Gradle Plugin

For Java/Kotlin projects:

```kotlin
// build.gradle.kts
plugins {
    id("software.amazon.smithy") version "0.16.0"
}

smithy {
    verbose = true
}
```

### Maven Plugin

```xml
<!-- pom.xml -->
<plugin>
    <groupId>software.amazon.smithy</groupId>
    <artifactId>smithy-maven-plugin</artifactId>
    <version>1.40.0</version>
</plugin>
```

## Your First Model

### Step 1: Create Directory Structure

```bash
mkdir my-first-smithy
cd my-first-smithy
mkdir model
```

### Step 2: Create smithy-build.json

```json
{
    "version": "1.0",
    "sources": ["model"]
}
```

### Step 3: Write Your First Model

```smithy
// model/hello.smithy
$version: "2"

namespace com.example.hello

/// A greeting service
service HelloService {
    version: "2024-01-01"
    operations: [SayHello]
}

/// Say hello to someone
operation SayHello {
    input := {
        /// Name of the person
        @required
        @length(min: 1, max: 100)
        name: String
    }
    output := {
        /// Greeting message
        message: String
    }
}
```

### Step 4: Build the Model

```bash
smithy build

# Output:
# Smithy model built successfully!
# Output: build/model.json
```

### Step 5: Validate the Model

```bash
smithy validate

# If there are errors:
# - Missing required traits
# - Type mismatches
# - Invalid constraints
```

## Building Models

### Smithy-build.json Structure

```json
{
    "version": "1.0",
    "sources": [
        "model",           // Local model files
        "model/common"     // Additional directories
    ],
    "maven": {
        "repositories": [
            {
                "url": "https://repo1.maven.org/maven2"
            }
        ],
        "dependencies": [
            "software.amazon.smithy:smithy-aws-traits:1.40.0"
        ]
    },
    "plugins": {
        "rust": {
            "service": "com.example#HelloService",
            "module": "hello-sdk"
        }
    }
}
```

### Model Composition

Split large models into files:

```
model/
├── common.smithy      # Shared types and traits
├── service.smithy     # Service definition
├── resources.smithy   # Resource definitions
├── operations.smithy  # Operation definitions
└── errors.smithy      # Error definitions
```

**common.smithy:**
```smithy
namespace com.example

/// Pagination token
@pattern("^[a-zA-Z0-9]+$")
string NextToken

/// Maximum results per page
@range(min: 1, max: 100)
integer MaxResults
```

**service.smithy:**
```smithy
namespace com.example

use com.example#NextToken
use com.example#MaxResults

service MyService {
    version: "2024-01-01"
    operations: [ListItems, GetItem]
}
```

## Shapes Deep Dive

### Simple Shapes

```smithy
// String
string Name

// Integer
integer Count

// Boolean
boolean IsActive

// Timestamp (ISO 8601)
timestamp CreatedAt

// Blob (binary data)
blob ImageData

// Document (arbitrary JSON)
document Metadata
```

### Constrained Shapes

```smithy
// String with constraints
@length(min: 1, max: 255)
@pattern("^[a-zA-Z0-9-]+$")
string Username

// Numeric constraints
@range(min: 0, max: 150)
integer Age

@range(min: -90.0, max: 90.0)
double Latitude

// List constraints
@length(min: 1, max: 10)
list Items {
    member: Item
}

// Map constraints
@length(min: 1, max: 50)
map Tags {
    key: TagKey
    value: TagValue
}
```

### Complex Shapes

```smithy
// Structure (object)
structure Person {
    @required
    id: String

    @required
    name: Name

    email: EmailAddress
    age: Age
    tags: Tags
}

// List (array)
list People {
    member: Person
}

// Map (key-value)
map StringMap {
    key: String
    value: String
}

// Union (tagged union / enum)
union SearchResult {
    user: User
    post: Post
    comment: Comment
}

// Enumeration
enum Status {
    ACTIVE = "active"
    INACTIVE = "inactive"
    PENDING = "pending"
}
```

## Resources

Resources model RESTful entities:

```smithy
namespace com.example.library

service Library {
    version: "2024-01-01"
    resources: [Book, Author]
}

resource Book {
    identifiers: { bookId: BookId }

    read: GetBook
    create: CreateBook
    update: UpdateBook
    delete: DeleteBook
    list: ListBooks

    // Nested resource
    resources: [Review]
}

resource Author {
    identifiers: { authorId: AuthorId }

    read: GetAuthor
    create: CreateAuthor
    list: ListAuthors

    // Relationship
    collectionOperations: [SearchAuthors]
}
```

**Generated HTTP routes:**
```
GET    /books              → ListBooks
POST   /books              → CreateBook
GET    /books/{bookId}     → GetBook
PUT    /books/{bookId}     → UpdateBook
DELETE /books/{bookId}     → DeleteBook

GET    /books/{bookId}/reviews    → ListReviews (nested)
```

## Operations

Operations model RPC-style calls:

```smithy
/// Search for books
@http(method: "GET", uri: "/search/books", code: 200)
operation SearchBooks {
    input := {
        @required
        query: SearchQuery

        @range(min: 1, max: 50)
        limit: Integer

        nextToken: NextToken
    }
    output := {
        results: BookList
        nextToken: NextToken
    }
    errors: [InvalidQuery, ServiceUnavailable]
}

/// Batch operation
@http(method: "POST", uri: "/books/batch", code: 200)
operation BatchGetBooks {
    input := {
        @length(min: 1, max: 100)
        bookIds: BookIdList
    }
    output := {
        books: BookList
        errors: BatchErrorList
    }
}
```

## Traits

Traits add metadata and constraints:

### Built-in Traits

```smithy
// Validation
@required           // Must be provided
@nullable           // Can be null
@length(min: 1, max: 100)
@range(min: 0, max: 100)
@pattern("^[a-z]+$")
@uniqueItems        // List items must be unique

// Documentation
@title("My Service")
@documentation("Detailed description")
@see("https://example.com")

// Protocol
@http(method: "GET", uri: "/path", code: 200)
@jsonName("camelCase")
@XmlName("XmlName")
@XmlAttribute

// Default values
@default("hello")
@default(0)
```

### Custom Traits

```smithy
// Define custom trait
@trait
structure custom:Sensitive {
    // Trait has no members (marker trait)
}

// Use custom trait
@custom:Sensitive
string Password

// Custom trait with value
@trait
structure custom:Encrypted {
    algorithm: String
}

@custom:Encrypted(algorithm: "AES-256")
string SecretData
```

## Code Generation

### Generate Rust SDK

```json
{
    "version": "1.0",
    "sources": ["model"],
    "maven": {
        "dependencies": [
            "software.amazon.smithy:smithy-rs-codegen:0.0.1"
        ]
    },
    "plugins": {
        "rust": {
            "service": "com.example#MyService",
            "module": "my-service-sdk",
            "moduleVersion": "0.1.0"
        }
    }
}
```

```bash
smithy build
# Generates SDK in target/smithyprojections/
```

### Generate TypeScript SDK

```json
{
    "plugins": {
        "typescript": {
            "service": "com.example#MyService"
        }
    }
}
```

### Generate Python SDK

```json
{
    "plugins": {
        "python": {
            "service": "com.example#MyService",
            "module": "my_service_sdk"
        }
    }
}
```

## Protocol Support

### restJson1

Most common for REST APIs:

```smithy
@restJson1
service MyApi {
    version: "2024-01-01"
}
```

**HTTP Mapping:**
- GET → Read operation
- POST → Create operation
- PUT → Update operation
- DELETE → Delete operation

### jsonRpc1

RPC-style JSON:

```smithy
@jsonRpc1
service RpcApi {
    version: "2024-01-01"
}
```

**Request format:**
```json
{
    "jsonrpc": "2.0",
    "method": "SayHello",
    "params": { "name": "Alice" },
    "id": 1
}
```

### Custom Protocol

```smithy
@protocolDefinition
structure MyProtocol {
    http: HttpProtocol
    serialization: JsonSerialization
}

@MyProtocol
service CustomService {
    version: "1.0.0"
}
```

## AWS SDK Generation

Smithy powers all AWS SDKs:

```smithy
$version: "2"

namespace com.amazonaws.dynamodb

@aws.protocols#awsJson1_0
@aws.auth#sigv4
service DynamoDb {
    version: "2012-08-10"
    resources: [Table]
}

resource Table {
    identifiers: { tableName: TableName }
    read: DescribeTable
    create: CreateTable
    delete: DeleteTable
}
```

**Generated SDKs:**
- aws-sdk-rust
- boto3 (Python)
- aws-sdk-js
- aws-sdk-java

## Testing Models

### Protocol Tests

```smithy
$test: protocol

test GetBookTest {
    input: { bookId: "123" }
    expected: {
        http: { status: 200 }
        body: { title: "Test Book" }
    }
}
```

### Validation Tests

```smithy
$test: validation

test InvalidBookId {
    input: { bookId: "" }  // Empty, violates @length
    expected: {
        error: "ValidationException"
        message: "bookId must be at least 1 character"
    }
}
```

---

**Next Steps:**
- [01-smithy-exploration.md](./01-smithy-exploration.md) - Full architecture
- [01-smithy-rs-deep-dive.md](./01-smithy-rs-deep-dive.md) - Rust code generation
- [02-smithy-python-deep-dive.md](./02-smithy-python-deep-dive.md) - Python code generation
- [03-smithy-typescript-deep-dive.md](./03-smithy-typescript-deep-dive.md) - TypeScript code generation
