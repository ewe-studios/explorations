# TCC (Tau C Compiler) & Config Compiler - Deep Dive Exploration

**Paths:**
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/tcc/`
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/config-compiler/`

---

## Executive Summary

**TCC (Tau C Compiler)** and **Config Compiler** are complementary tools for processing Taubyte configurations:

- **TCC** - Compiles Tau configuration files, handling the full compilation pipeline
- **Config Compiler** - Core compilation engine that converts YAML configurations to TNS (Tau Naming Service) format

Both tools are essential for the Tau ecosystem, enabling declarative infrastructure configuration.

---

## TCC (Tau C Compiler)

**Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/tcc/`

**Version:** Based on config-compiler v0.4.6

**Language:** Go 1.21

### Architecture Overview

```
tcc/
├── object/            # Object representation
├── parser/            # Configuration parser
├── taubyte/           # Taubyte-specific handling
├── wrapper/           # API wrappers
├── go.mod
├── go.sum
├── LICENSE
└── README.md
```

### Dependencies

```go
require (
    github.com/alecthomas/units v0.0.0-20231202071711-9a357b53e9c9
    github.com/fxamacker/cbor/v2 v2.5.0
    github.com/ipfs/go-cid v0.4.1
    github.com/spf13/afero v1.9.5
    github.com/taubyte/config-compiler v0.4.6
    github.com/taubyte/go-interfaces v0.2.14
    github.com/taubyte/go-project-schema v0.9.3
    github.com/taubyte/go-seer v1.0.6
    gopkg.in/yaml.v2 v2.4.0
)
```

### Core Components

#### Parser Module

The parser handles YAML configuration parsing:
- Schema validation
- Type coercion
- Default value injection
- Error reporting

#### Object Module

Represents compiled configuration objects:
- Resource definitions
- Service configurations
- Network topology
- Authentication settings

#### Taubyte Module

Taubyte-specific extensions:
- Cloud configuration
- P2P settings
- Domain validation
- Swarm management

---

## Config Compiler

**Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/config-compiler/`

**Version:** v0.4.x

**Language:** Go 1.19+

### Architecture Overview

```
config-compiler/
├── common/            # Common utilities
├── compile/           # Compilation logic
├── decompile/         # Decompression logic
├── fixtures/          # Test fixtures
├── ifaces/            # Interfaces
├── indexer/           # Resource indexing
├── compile_test.json
├── decompile_test.go
├── e2e_test.go
├── go.mod
├── go.sum
├── LICENSE
├── project_data_test.go
└── README.md
```

### Purpose

From README:
> `config-compiler` is used by the `monkey` protocol, `tau-cli` and a number of tests to compile configuration from yaml to `tns` format.

### Compilation Pipeline

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   YAML      │────▶│   Parser    │────▶│   Validator │
│   Config    │     │             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
                                              │
                                              ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│    TNS      │◀────│   Encoder   │◀────│   Indexer   │
│   Format    │     │             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
```

---

## Compilation Logic

### Compiler Structure (compile/compiler.go)

```go
type compiler struct {
    config  *Config
    index   *Index
    log     io.Writer
}
```

### Compilation Groups

The compiler handles multiple resource types:

```go
func compilationGroup(project projectSchema.Project) map[string]compileObject {
    getter := project.Get()
    return map[string]compileObject{
        databaseSpec.PathVariable.String():  {
            Get: getter.Databases,
            Compile: database,
            Indexer: indexer.Databases
        },
        domainSpec.PathVariable.String():    {
            Get: getter.Domains,
            Compile: domain,
            Indexer: indexer.Domains
        },
        functionSpec.PathVariable.String():  {
            Get: getter.Functions,
            Compile: function,
            Indexer: indexer.Functions
        },
        librarySpec.PathVariable.String():   {
            Get: getter.Libraries,
            Compile: library,
            Indexer: indexer.Libraries
        },
        messagingSpec.PathVariable.String(): {
            Get: getter.Messaging,
            Compile: messaging,
            Indexer: indexer.Messaging
        },
        serviceSpec.PathVariable.String():   {
            Get: getter.Services,
            Compile: service,
            Indexer: nil
        },
        smartOpSpec.PathVariable.String():   {
            Get: getter.SmartOps,
            Compile: smartOps,
            Indexer: indexer.SmartOps
        },
        storageSpec.PathVariable.String():   {
            Get: getter.Storages,
            Compile: storage,
            Indexer: indexer.Storages
        },
        websiteSpec.PathVariable.String():   {
            Get: getter.Websites,
            Compile: website,
            Indexer: indexer.Websites
        },
    }
}
```

### Compilation Flow

```go
func (c *compiler) indexer(ctx *indexer.IndexContext, f indexerFunc) error {
    return f(ctx, c.config.Project, c.index)
}

func (c *compiler) magic(list []string, app string, f magicFunc) (map[string]interface{}, error) {
    returnMap := make(map[string]interface{}, len(list))
    for _, name := range list {
        fmt.Fprintf(c.log, "[Build|%s] Compiling %s\n", name, app)
        _id, Object, err := f(name, app, c.config.Project)
        if err != nil {
            fmt.Fprintf(c.log, "[Build|%s] failed with %s\n", name, err.Error())
            return returnMap, err
        }
        if len(Object) != 0 {
            returnMap[_id] = Object
        }
    }
    return returnMap, nil
}
```

---

## Resource Compilation

### Database Compilation (compile/database.go)

```go
func database(name, app string, project projectSchema.Project) (string, map[string]interface{}, error) {
    db, err := project.Get().Database(name, app)
    if err != nil {
        return "", nil, err
    }

    // Extract database configuration
    // Generate TNS format
    // Return compiled object
}
```

### Domain Compilation (compile/domain.go)

Handles domain configuration including:
- DNS settings
- SSL/TLS certificates
- ACME integration
- Domain validation

### Function Compilation (compile/function.go)

Compiles serverless function configurations:
- Wasm module references
- Runtime settings
- Memory limits
- Timeout configuration
- Trigger definitions

### Library Compilation (compile/library.go)

Handles shared library configurations:
- Git repository references
- Branch/tag selection
- Build instructions
- Import paths

### Messaging Compilation (compile/messaging.go)

Compiles pub/sub messaging configurations:
- Topic definitions
- Subscription rules
- Message routing
- Queue settings

### Service Compilation (compile/service.go)

Handles service definitions:
- Service types
- Port mappings
- Health checks
- Dependencies

### SmartOps Compilation (compile/smartops.go)

Compiles smart operation configurations:
- Trigger conditions
- Action definitions
- Attachment rules

### Storage Compilation (compile/storage.go)

Handles storage bucket configurations:
- Bucket names
- Access policies
- Encryption settings
- Lifecycle rules

### Website Compilation (compile/website.go)

Compiles website configurations:
- Static asset paths
- Build commands
- Deployment targets
- Domain bindings

---

## Indexer System

### Purpose

The indexer creates an index of all resources for efficient lookup:

```
indexer/
├── applications.go
├── common.go
├── database.go
├── domain.go
├── function.go
├── library.go
├── messaging.go
├── resource.go
├── service.go
├── smartop.go
├── storage.go
└── website.go
```

### Index Context

```go
type IndexContext struct {
    // Compilation context
    // Resource tracking
    // Dependency resolution
}
```

---

## Decompile Module

The decompile module reverses the compilation process:

### Purpose

- Convert TNS format back to YAML
- Debug compilation output
- Configuration inspection

### Files

```
decompile/
├── application.go
├── builder.go
├── build.go
├── common.go
├── database.go
├── domain.go
├── function.go
├── library.go
├── messaging.go
├── resource.go
├── service.go
├── smartop.go
├── storage.go
└── website.go
```

---

## Parser Module

### Parser Components (tcc/parser/)

```
parser/
├── fixtures/
├── helpers.go
├── helpers_test.go
├── node.go
├── node_test.go
├── options.go
├── options_test.go
├── parser.go
├── parser_test.go
├── schema.go
├── schema_test.go
├── stringmatch.go
├── stringmatch_test.go
├── type.go
├── types.go
├── type_test.go
├── validators.go
└── validators_test.go
```

### Validation

The parser includes comprehensive validation:
- Schema validation
- Type checking
- Required field verification
- Format validation
- Cross-reference validation

---

## Integration Points

### Monkey Protocol

The config-compiler is used by the Monkey protocol for:
- Runtime configuration
- Service deployment
- Resource provisioning

### Tau CLI

Tau CLI uses the compiler for:
- Project configuration
- Resource management
- Deployment preparation

### Dream

Dream uses the compiler for:
- Local configuration
- Universe setup
- Service initialization

### Tests

Both tools are extensively tested:
- Unit tests for each resource type
- Integration tests for full compilation
- End-to-end tests

---

## Testing

### Test Fixtures

```
fixtures/
├── compile_test.json
├── Test configurations
└── Expected outputs
```

### Test Coverage

```bash
# Run all tests
go test -v ./...

# Run specific tests
go test -v --run TestCompile

# Coverage
go test -coverprofile cover.out ./...
```

### Test Types

1. **Unit Tests** - Test individual functions
2. **Integration Tests** - Test compilation pipeline
3. **E2E Tests** - Test full workflow

---

## Configuration Schema

### Project Structure

```yaml
project:
  name: my-project
  applications:
    - name: my-app
      functions:
        - name: hello
          runtime: wasm
      websites:
        - name: site
          path: ./dist
      databases:
        - name: main
          type: kv
```

### Compiled Output (TNS Format)

The TNS format is a binary/encoded format optimized for:
- Fast parsing
- Compact storage
- Efficient transmission
- Version compatibility

---

## Error Handling

### Compilation Errors

The compiler provides detailed error messages:
- Line and column numbers
- Expected vs actual values
- Suggested fixes

### Validation Errors

Validation errors include:
- Missing required fields
- Invalid types
- Schema violations
- Cross-reference failures

---

## Build System

### Go Modules

Both projects use Go modules:

**TCC:**
```go
module github.com/taubyte/tcc
go 1.21
```

**Config Compiler:**
```go
module github.com/taubyte/config-compiler
go 1.19
```

### Dependencies

Key shared dependencies:
- `github.com/taubyte/go-project-schema`
- `github.com/taubyte/go-specs`
- `github.com/taubyte/go-interfaces`
- `github.com/taubyte/domain-validation`
- `github.com/taubyte/utils`

---

## Use Cases

### Development

1. Write YAML configuration
2. Compile to TNS format
3. Deploy to Dream for testing

### Production

1. Version control YAML configs
2. CI/CD compilation
3. Deploy compiled configs

### Debugging

1. Decompile TNS to YAML
2. Inspect compiled output
3. Verify configuration

---

## Summary

**TCC** and **Config Compiler** are essential tools in the Tau ecosystem:

**TCC:**
- Full compilation pipeline
- Parser and object handling
- Taubyte-specific extensions

**Config Compiler:**
- Core compilation engine
- Resource-specific compilers
- Indexing and decompilation

**Key Features:**
- YAML to TNS conversion
- Resource validation
- Comprehensive error handling
- Extensive test coverage
- Integration with all Tau tools
