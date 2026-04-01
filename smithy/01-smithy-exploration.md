---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy
repository: git@github.com:smithy-lang/smithy.git
explored_at: 2026-03-29
language: Java
category: Interface Definition Language, Code Generation
---

# Smithy IDL - Exploration

## Overview

Smithy is an **interface definition language (IDL)** and **service modeling language** that defines and generates clients, services, and documentation for any protocol. Originally developed by Amazon for AWS SDK generation, Smithy enables protocol-agnostic service definitions that can generate code for multiple languages and protocols.

### Key Value Proposition

- **Protocol Agnostic**: Define services once, generate for REST, JSON, RPC, GraphQL
- **Multi-Language**: Generate TypeScript, Python, Rust, Java, Go clients/servers
- **Type-Safe**: Strong typing with trait-based validation
- **Extensible**: Custom traits for domain-specific constraints
- **Documented**: Built-in documentation generation
- **Versioned**: Model evolution with backward compatibility checking

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Smithy Ecosystem                              │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Smithy IDL (Model Files)                    │   │
│  │              .smithy files                               │   │
│  └────────────────────┬────────────────────────────────────┘   │
│                       │                                         │
│                       ▼                                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Smithy CLI / Gradle Plugin                  │   │
│  │              - Model validation                          │   │
│  │              - Model composition                         │   │
│  │              - Trait application                         │   │
│  └────────────────────┬────────────────────────────────────┘   │
│                       │                                         │
│                       ▼                                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Smithy Model (AST)                          │   │
│  │              - Shapes (Service, Resource, Operation)     │   │
│  │              - Members (Input, Output, Errors)           │   │
│  │              - Traits (Metadata, Constraints)            │   │
│  └────────────────────┬────────────────────────────────────┘   │
│                       │                                         │
│         ┌─────────────┼─────────────┬─────────────┐            │
│         │             │             │             │             │
│         ▼             ▼             ▼             ▼             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │smithy-rs │  │smithy-   │  │smithy-   │  │smithy-   │       │
│  │          │  │python    │  │typescript│  │jmespath  │       │
│  │(Rust SDK)│  │(Python   │  │(TS SDK)  │  │(Query    │       │
│  │          │  │ SDK)     │  │          │  │ Lang)    │       │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘       │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │         AWS SDK Generators                               │   │
│  │         - aws-sdk-rust                                   │   │
│  │         - aws-sdk-python (boto3)                         │   │
│  │         - aws-sdk-typescript                             │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Monorepo Structure

```
smithy/
├── smithy-model/               # Core Smithy AST model
│   └── src/main/java/software/amazon/smithy/model/
│       ├── Model.java          # Complete model container
│       ├── Shape.java          # Base shape interface
│       ├── shapes/             # Shape implementations
│       │   ├── ServiceShape.java
│       │   ├── ResourceShape.java
│       │   ├── OperationShape.java
│       │   ├── StructureShape.java
│       │   ├── ListShape.java
│       │   ├── MapShape.java
│       │   └── ...
│       └── traits/             # Built-in traits
│           ├── RequiredTrait.java
│           ├── LengthTrait.java
│           ├── RangeTrait.java
│           └── ...
│
├── smithy-build/               # Build system
│   └── src/main/java/software/amazon/smithy/build/
│       ├── SmithyBuild.java    # Main build orchestrator
│       ├── BuildContext.java   # Build context
│       └── transforms/         # Model transforms
│
├── smithy-codegen-core/        # Code generation framework
│   └── src/main/java/software/amazon/smithy/codegen/
│       ├── SymbolProvider.java # Type mapping
│       ├── ProtocolGenerator.java  # Protocol-specific code
│       └── writer/             # Code writers
│
├── smithy-cli/                 # Command-line interface
│   └── src/main/java/software/amazon/smithy/cli/
│       ├── Cli.java            # Main CLI entry
│       ├── BuildCommand.java   # Build command
│       └── ValidateCommand.java # Validation command
│
├── smithy-aws-traits/          # AWS-specific traits
│   └── traits/                 # AWS metadata traits
│
├── smithy-protocol-traits/     # Protocol-specific traits
│
├── smithy-openapi/             # OpenAPI (Swagger) conversion
│
├── smithy-jsonschema/          # JSON Schema generation
│
├── smithy-docgen/              # Documentation generation
│
├── smithy-linters/             # Model linting rules
│
├── smithy-rules-engine/        # Endpoint rules engine
│
├── smithy-waiters/             # Waiter configuration
│
└── smithy-protocol-tests/      # Protocol test suite
```

## Core Concepts

### 1. Smithy IDL

Smithy models are defined in `.smithy` files:

```smithy
$version: "2"

namespace com.example.weather

/// Weather service for getting forecasts
@title("Weather Service")
@version("2024-01-01")
service Weather {
    version: "2024-01-01"
    resources: [City, Forecast]
    operations: [GetCurrentTime, GetWeather]
}

/// A city with weather data
resource City {
    identifiers: { cityId: CityId }
    read: GetCity
    update: UpdateCity
    delete: DeleteCity
    list: ListCities
    create: CreateCity
    resources: [Forecast]
}

/// Get current weather for a city
@http(method: "GET", uri: "/cities/{cityId}/weather", code: 200)
operation GetWeather {
    input := {
        cityId: CityId
    }
    output := {
        temperature: Temperature
        conditions: WeatherConditions
        humidity: Integer
    }
    errors: [CityNotFound]
}

/// Unique city identifier
@length(min: 1, max: 100)
@pattern("^[a-zA-Z0-9-]+$")
string CityId

/// Temperature in Celsius
@range(min: -90, max: 60)
integer Temperature

/// Weather conditions
enum WeatherConditions {
    SUNNY = "sunny"
    CLOUDY = "cloudy"
    RAINY = "rainy"
    SNOWY = "snowy"
}

/// Error when city is not found
@error("client")
@httpError(404)
structure CityNotFound {
    @required
    message: String
}
```

### 2. Shapes

Shapes are the building blocks of Smithy models:

| Shape Type | Purpose | Example |
|------------|---------|---------|
| `service` | Top-level service definition | `service Weather {}` |
| `resource` | RESTful resource with CRUD | `resource City {}` |
| `operation` | RPC-style operation | `operation GetWeather {}` |
| `structure` | Object/record type | `structure Weather { temp: Integer }` |
| `list` | Array type | `list Cities { member: City }` |
| `map` | Key-value map | `map Metadata { key: String, value: String }` |
| `string` | String type | `string Name` |
| `integer` | Integer type | `integer Count` |
| `enum` | Enumerated values | `enum Status { ACTIVE, INACTIVE }` |
| `union` | Tagged union | `union Result { Success, Error }` |

### 3. Traits

Traits add metadata and constraints to shapes:

```smithy
// Built-in traits
@required           // Member must be provided
@nullable           // Member can be null
@length(min: 1, max: 100)  // String/list length constraints
@range(min: 0, max: 100)   // Numeric range constraints
@pattern("^[a-z]+$")       // Regex pattern for strings
@default("hello")          // Default value
@deprecated                // Mark as deprecated

// Documentation traits
@title("My Service")
@documentation("Description here")
@see("https://example.com")

// Protocol traits
@http(method: "GET", uri: "/path", code: 200)
@jsonName("camelCaseName")
@xmlName("XmlName")

// AWS-specific traits
@awsAuthSigV4           // Use SigV4 authentication
@awsRegion              // Region configuration
```

**Custom traits:**
```smithy
@trait
structure custom:MyTrait {
    value: String
}

@custom:MyTrait(value: "test")
string MyString
```

### 4. Resources vs Operations

**Resources** (RESTful):
```smithy
resource City {
    identifiers: { cityId: CityId }

    // CRUD operations
    read: GetCity       // GET /cities/{cityId}
    create: CreateCity  // POST /cities
    update: UpdateCity  // PUT /cities/{cityId}
    delete: DeleteCity  // DELETE /cities/{cityId}
    list: ListCities    // GET /cities

    // Nested resources
    resources: [Forecast]
}
```

**Operations** (RPC-style):
```smithy
operation GetCurrentTime {
    input := {
        timezone: String
    }
    output := {
        timestamp: Timestamp
    }
}
```

### 5. Service Definition

Services define the API surface:

```smithy
service Weather {
    version: "2024-01-01"

    // Top-level operations
    operations: [GetCurrentTime, GetWeather]

    // Resources
    resources: [City, Forecast]

    // Error types
    errors: [ServiceUnavailable]

    // Service-level traits
    @title("Weather API")
    @documentation("Weather service documentation")
    @awsAuthSigV4
}
```

## Code Generation

### smithy-rs (Rust SDK)

```rust
// Generated client code
let config = aws_config::load_from_env().await;
let client = weather_sdk::Client::new(&config);

// Generated operation
let response = client
    .get_weather()
    .city_id("seattle")
    .send()
    .await?;

println!("Temperature: {}", response.temperature);
```

**Structure:**
```
smithy-rs/
├── codegen/
│   ├── src/main/kotlin/software/amazon/smithy/rust/codegen/
│   │   ├── core/           # Core code generation
│   │   ├── smithy/         # Smithy-specific code
│   │   └── aws/            # AWS-specific code
├── runtime/
│   ├── aws-types/          # AWS type definitions
│   ├── smithy-types/       # Smithy type definitions
│   └── protocol-test/      # Protocol test helpers
```

### smithy-typescript (TypeScript SDK)

```typescript
// Generated client code
import { WeatherClient, GetWeatherCommand } from "@example/weather-sdk";

const client = new WeatherClient({ region: "us-east-1" });

const command = new GetWeatherCommand({ cityId: "seattle" });
const response = await client.send(command);

console.log(`Temperature: ${response.temperature}`);
```

### smithy-python (Python SDK)

```python
# Generated client code
from weather_sdk import WeatherClient, GetWeatherCommand

client = WeatherClient(region="us-east-1")

response = client.get_weather(cityId="seattle")
print(f"Temperature: {response.temperature}")
```

## Protocol Support

Smithy supports multiple protocols:

| Protocol | Description | Example |
|----------|-------------|---------|
| `restJson1` | REST with JSON bodies | AWS REST APIs |
| `restXml` | REST with XML bodies | AWS S3 |
| `jsonRpc1` | JSON-RPC style | AWS DynamoDB |
| `ec2Query` | EC2 query protocol | AWS EC2 |
| `awsJson1_0` | AWS JSON protocol | AWS Lambda |
| `awsJson1_1` | AWS JSON 1.1 protocol | AWS Bedrock |
| `graphql` | GraphQL protocol | Custom APIs |

### Protocol Definition

```smithy
@restJson1
service Weather {
    version: "2024-01-01"
}

// Or custom protocol
@protocolDefinition
structure MyProtocol {
    http: HttpProtocol
    serialization: JsonSerialization
}

@MyProtocol
service MyService {
    version: "1.0.0"
}
```

## Building Models

### Using Smithy CLI

```bash
# Install Smithy CLI
npm install -g @smithy/cli

# Build model
smithy build

# Validate model
smithy validate

# Generate docs
smithy docgen

# Convert to OpenAPI
smithy transform --to-openapi
```

### Using Gradle Plugin

```kotlin
// build.gradle.kts
plugins {
    id("software.amazon.smithy") version "0.16.0"
}

smithy {
    verbose = true
    formats = listOf("json", "javadoc")
    plugins {
        create("rust") {
            artifact = "software.amazon.smithy:smithy-rs-codegen:0.0.1"
        }
    }
}
```

### smithy-build.json

```json
{
    "version": "1.0",
    "sources": ["model"],
    "maven": {
        "dependencies": [
            "software.amazon.smithy:smithy-aws-traits:1.40.0"
        ]
    },
    "plugins": {
        "rust": {
            "service": "com.example#Weather"
        }
    }
}
```

## Comparison with Other IDLs

### Smithy vs OpenAPI/Swagger

| Aspect | Smithy | OpenAPI |
|--------|--------|---------|
| **Abstraction Level** | Service modeling | API documentation |
| **Protocol Support** | Multiple (REST, JSON, RPC) | Primarily REST |
| **Code Generation** | First-class citizen | Afterthought |
| **Type System** | Strong, extensible | Limited |
| **Resource Modeling** | Built-in resource concept | Paths only |
| **Validation** | Compile-time validation | Runtime only |

### Smithy vs Protobuf/gRPC

| Aspect | Smithy | Protobuf |
|--------|--------|----------|
| **Primary Use** | Web APIs | RPC |
| **Wire Format** | JSON, XML, binary | Binary only |
| **HTTP Mapping** | Built-in (@http) | gRPC-specific |
| **Browser Support** | Native | Requires grpc-web |
| **Service Discovery** | Via traits | External |

### Smithy vs GraphQL

| Aspect | Smithy | GraphQL |
|--------|--------|---------|
| **Query Model** | Defined operations | Ad-hoc queries |
| **Caching** | HTTP caching | Complex |
| **Authorization** | Service-level | Field-level |
| **Learning Curve** | Moderate | Steeper |

## Production Considerations

### Model Organization

```
models/
├── common.smithy         # Shared types, traits
├── weather.smithy        # Weather service definition
├── city.smithy           # City resource
├── forecast.smithy       # Forecast resource
└── errors.smithy         # Error definitions
```

### Versioning Strategy

```smithy
// Version 1
service WeatherV1 {
    version: "2024-01-01"
}

// Version 2 (breaking changes)
service WeatherV2 {
    version: "2025-01-01"
}

// Use deprecation for non-breaking
@deprecated(message: "Use GetForecastV2")
operation GetForecast { ... }
```

### Testing

```smithy
// Protocol tests
$test: protocol
test GetWeatherTest {
    input: { cityId: "seattle" }
    expected: {
        http: { status: 200 }
        body: { temperature: 15 }
    }
}
```

---

## Related Deep Dives

- [00-zero-to-smithy-engineer.md](./00-zero-to-smithy-engineer.md) - Fundamentals
- [01-smithy-rs-deep-dive.md](./01-smithy-rs-deep-dive.md) - Rust code generation
- [02-smithy-python-deep-dive.md](./02-smithy-python-deep-dive.md) - Python code generation
- [03-smithy-typescript-deep-dive.md](./03-smithy-typescript-deep-dive.md) - TypeScript code generation
- [04-idl-specification-deep-dive.md](./04-idl-specification-deep-dive.md) - IDL deep dive
- [rust-revision.md](./rust-revision.md) - Rust implementation considerations
- [production-grade.md](./production-grade.md) - Production deployment guide
