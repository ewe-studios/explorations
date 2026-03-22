# Taubyte Wazero Runtime - Comprehensive Deep-Dive Exploration

**Date:** 2026-03-22
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/wazero/`

---

## 1. Purpose and Overview

**Wazero** is the zero-dependency WebAssembly runtime for Go developers that Taubyte uses as the underlying runtime for the Taubyte WebAssembly Virtual Machine (TVM). Wazero implements the WebAssembly Core Specification 1.0 and 2.0, providing both interpreter and compiler execution modes.

### Key Characteristics

- **Module Path:** `github.com/tetratelabs/wazero`
- **Version in Taubyte:** v1.6.0
- **License:** Apache 2.0
- **Dependencies:** Zero (pure Go)
- **Runtime Modes:** Compiler (AOT) and Interpreter

---

## 2. Architecture

### 2.1 Module Structure

```
wazero/
├── api/                    # WebAssembly API
│   ├── features.go        # Feature flags
│   └── wasm.go            # Core WASM types
├── builder.go             # Module builder
├── cache.go               # Compilation cache
├── config.go              # Runtime configuration
├── runtime.go             # Runtime implementation
├── cmd/wazero/            # CLI tool
├── examples/              # Usage examples
│   ├── basic/             # Basic example
│   ├── allocation/        # Memory allocation examples
│   ├── import-go/         # Go import examples
│   └── multiple-runtimes/ # Multi-runtime example
├── experimental/          # Experimental features
│   ├── gojs/              # Go JS interoperability
│   └── listener.go        # Event listener
└── internal/              # Internal implementation
```

### 2.2 Runtime Modes

#### Compiler Mode (Default)

```go
// AOT (Ahead-of-Time) compilation
// Faster execution, higher memory usage
config := wazero.NewRuntimeConfigCompiler()
runtime := wazero.NewRuntimeWithConfig(ctx, config)
```

**Characteristics:**
- Compiles WASM to native machine code
- 10x faster than interpreter
- Supports amd64 and arm64
- Linux, macOS, Windows

#### Interpreter Mode

```go
// Naive interpretation
// Slower but portable
config := wazero.NewRuntimeConfigInterpreter()
runtime := wazero.NewRuntimeWithConfig(ctx, config)
```

**Characteristics:**
- Interprets WASM bytecode
- Works on all Go platforms (including riscv64)
- No platform-specific code

---

## 3. Key Types, Interfaces, and APIs

### 3.1 Runtime

```go
type Runtime interface {
    // CompileModule compiles a WebAssembly module
    CompileModule(ctx context.Context, binary []byte) (CompiledModule, error)

    // InstantiateModule instantiates a compiled module
    InstantiateModule(ctx context.Context, compiled CompiledModule, config ModuleConfig) (Module, error)

    // Close closes the runtime and releases resources
    Close(ctx context.Context) error
}

// Create runtime
runtime := wazero.NewRuntime(ctx)
defer runtime.Close(ctx)
```

### 3.2 Compiled Module

```go
type CompiledModule interface {
    // Name returns the module name
    Name() string

    // Import returns imported functions/memory
    Import(name, funcName string) (FunctionDefinition, bool)

    // Export returns exported functions/memory
    Export(name string) (ExportDefinition, bool)
}
```

### 3.3 Module Instance

```go
type Module interface {
    // Name returns the instance name
    Name() string

    // ExportedFunction returns an exported function
    ExportedFunction(name string) Function

    // ExportedMemory returns exported memory
    ExportedMemory() Memory

    // ExportedGlobal returns exported global
    ExportedGlobal(name string) Global

    // Close closes the module instance
    Close(ctx context.Context) error
}

// Call a function
fn := module.ExportedFunction("add")
results, err := fn.Call(ctx, uint64(1), uint64(2))
```

### 3.4 Function

```go
type Function interface {
    // Definition returns the function definition
    Definition() FunctionDefinition

    // Call calls the function with parameters
    Call(ctx context.Context, params ...uint64) ([]uint64, error)
}

type FunctionDefinition interface {
    // Name returns the function name
    Name() string

    // ParamTypes returns parameter types
    ParamTypes() []ValueType

    // ResultTypes returns result types
    ResultTypes() []ValueType

    // NumIn returns number of inputs
    NumIn() int

    // NumOut returns number of outputs
    NumOut() int
}
```

### 3.5 Host Module Builder

```go
type HostModuleBuilder interface {
    // NewFunctionBuilder creates a function builder
    NewFunctionBuilder() FunctionDefinitionBuilder

    // Export exports a function to the module
    Export(name string) HostModuleBuilder

    // Instantiate creates the host module
    Instantiate(ctx context.Context) error
}

// Create host module
hostModule := runtime.NewHostModuleBuilder("env")

// Define host function
hostModule = hostModule.NewFunctionBuilder().
    WithFunc(func(ctx context.Context, m api.Module, x uint32) uint32 {
        return x * 2
    }).
    Export("double")

// Instantiate
err := hostModule.Instantiate(ctx)
```

### 3.6 Module Configuration

```go
type ModuleConfig interface {
    // WithName sets the module name
    WithName(string) ModuleConfig

    // WithArgs sets command-line arguments
    WithArgs(...string) ModuleConfig

    // WithEnv sets an environment variable
    WithEnv(key, value string) ModuleConfig

    // WithStdin sets standard input
    WithStdin(io.Reader) ModuleConfig

    // WithStdout sets standard output
    WithStdout(io.Writer) ModuleConfig

    // WithStderr sets standard error
    WithStderr(io.Writer) ModuleConfig

    // WithFS sets the filesystem
    WithFS(fs.FS) ModuleConfig

    // WithStartFunctions sets functions to call on start
    WithStartFunctions(...string) ModuleConfig
}

// Configure module
config := wazero.NewModuleConfig().
    WithName("mymodule").
    WithStdout(os.Stdout).
    WithStderr(os.Stderr).
    WithArgs("program", "arg1", "arg2").
    WithEnv("ENV", "production").
    WithStartFunctions("_start")
```

### 3.7 Memory

```go
type Memory interface {
    // Size returns the size in bytes
    Size() uint32

    // ReadByte reads a byte
    ReadByte(offset uint32) (byte, bool)

    // ReadUint32Le reads a uint32 (little-endian)
    ReadUint32Le(offset uint32) (uint32, bool)

    // ReadFloat32Le reads a float32 (little-endian)
    ReadFloat32Le(offset uint32) (float32, bool)

    // ReadFloat64Le reads a float64 (little-endian)
    ReadFloat64Le(offset uint32) (float64, bool)

    // Read reads bytes into a buffer
    Read(offset uint32, size uint32, dst []byte) bool

    // WriteByte writes a byte
    WriteByte(offset uint32, val byte) bool

    // WriteUint32Le writes a uint32 (little-endian)
    WriteUint32Le(offset uint32, val uint32) bool

    // Write writes bytes from a buffer
    Write(offset uint32, val []byte) bool

    // Grow grows memory by pages
    Grow(pages uint32) bool

    // Index returns the memory index
    Index() Index
}
```

### 3.8 Global

```go
type Global interface {
    // Type returns the global type
    Type() ValueType

    // Get returns the current value
    Get(context.Context) uint64

    // Set sets the value (if mutable)
    Set(context.Context, uint64) error
}
```

### 3.9 Compilation Cache

```go
// Create cache
cache := wazero.NewCompilationCache()
defer cache.Close()

// Configure runtime with cache
config := wazero.NewRuntimeConfig().
    WithCompilationCache(cache)

runtime := wazero.NewRuntimeWithConfig(ctx, config)

// Compile module (cached)
compiled, err := runtime.CompileModule(ctx, wasmBytes)

// Subsequent compilations of same module use cache
```

---

## 4. Integration with Taubyte Components

### 4.1 TVM Integration

```go
// vm/service/wazero/runtime.go
import "github.com/tetratelabs/wazero"

type instance struct {
    runtime wazero.Runtime
    module  wazero.CompiledModule
    inst    wazero.ModuleInstance
}

func (i *instance) Call(function string, params ...interface{}) (interface{}, error) {
    fn := i.inst.ExportedFunction(function)
    if fn == nil {
        return nil, fmt.Errorf("function %s not found", function)
    }

    wasmParams := convertParams(params)
    results, err := fn.Call(i.ctx.Context(), wasmParams...)
    if err != nil {
        return nil, err
    }

    return convertResults(results), nil
}
```

### 4.2 Host Module Registration

```go
// vm/service/wazero/host_module.go
func registerHostFunctions(runtime wazero.Runtime) error {
    hostModule := runtime.NewHostModuleBuilder("taubyte/sdk")

    // Register database functions
    hostModule = hostModule.NewFunctionBuilder().
        WithFunc(databaseNew).
        Export("databaseNew")

    // Register HTTP functions
    hostModule = hostModule.NewFunctionBuilder().
        WithFunc(httpEventMethod).
        Export("httpEventMethod")

    // Register storage functions
    hostModule = hostModule.NewFunctionBuilder().
        WithFunc(storageNew).
        Export("storageNew")

    _, err := hostModule.Instantiate(context.Background())
    return err
}
```

### 4.3 Module Loading

```go
// vm/loaders/wazero/loader.go
func (l *loader) LoadAndRun(ctx vm.Context, module string) error {
    // Load WASM bytes
    reader, err := l.loader.Load(ctx, module)
    if err != nil {
        return err
    }
    defer reader.Close()

    wasmBytes, _ := io.ReadAll(reader)

    // Compile module
    compiled, err := runtime.CompileModule(ctx.Context(), wasmBytes)
    if err != nil {
        return err
    }

    // Configure and instantiate
    config := wazero.NewModuleConfig().
        WithName(module).
        WithStartFunctions("_start")

    _, err = runtime.InstantiateModule(ctx.Context(), compiled, config)
    return err
}
```

---

## 5. Usage Examples

### 5.1 Basic Example

```go
package main

import (
    "context"
    "github.com/tetratelabs/wazero"
)

func main() {
    ctx := context.Background()

    // Create runtime
    runtime := wazero.NewRuntime(ctx)
    defer runtime.Close(ctx)

    // Compile module
    compiled, err := runtime.CompileModule(ctx, wasmBytes)
    if err != nil {
        panic(err)
    }

    // Instantiate
    module, err := runtime.InstantiateModule(ctx, compiled,
        wazero.NewModuleConfig())
    if err != nil {
        panic(err)
    }

    // Call function
    add := module.ExportedFunction("add")
    results, _ := add.Call(ctx, uint64(1), uint64(2))
    println(results[0]) // 3
}
```

### 5.2 Host Function Example

```go
// Define Go function to export to WASM
func hostDouble(ctx context.Context, m api.Module, x uint32) uint32 {
    return x * 2
}

// Register host function
hostModule := runtime.NewHostModuleBuilder("env")
hostModule.NewFunctionBuilder().
    WithFunc(hostDouble).
    Export("double")
hostModule.Instantiate(ctx)

// WASM can now call env.double(x)
```

### 5.3 Memory Sharing Example

```go
// Write data to WASM memory
memory := module.ExportedMemory()
memory.WriteUint32Le(0, 42)

// Read from WASM memory
value, _ := memory.ReadUint32Le(0)
println(value) // 42

// Pass pointer to host function
func hostProcess(ctx context.Context, m api.Module, ptr uint32) {
    memory := m.Memory()
    value, _ := memory.ReadUint32Le(ptr)
    println("Processing:", value)
}
```

### 5.4 Filesystem Integration

```go
// Create virtual filesystem
fs := os.DirFS("/path/to/files")

// Configure module with filesystem
config := wazero.NewModuleConfig().
    WithFS(fs).
    WithArgs("program", "file.txt")

module, _ := runtime.InstantiateModule(ctx, compiled, config)
```

---

## 6. Performance Considerations

### 6.1 Compilation Caching

```go
// Use compilation cache for production
cache := wazero.NewCompilationCacheWithDir("/tmp/wazero-cache")
defer cache.Close()

config := wazero.NewRuntimeConfig().
    WithCompilationCache(cache)

runtime := wazero.NewRuntimeWithConfig(ctx, config)
```

### 6.2 Module Reuse

```go
// Compile once
compiled, _ := runtime.CompileModule(ctx, wasmBytes)

// Instantiate multiple times (different instances)
instance1, _ := runtime.InstantiateModule(ctx, compiled, config1)
instance2, _ := runtime.InstantiateModule(ctx, compiled, config2)
```

### 6.3 Memory Limits

```go
// Set memory limits
config := wazero.NewModuleConfig().
    WithMemoryLimitPages(256) // 16MB max
```

---

## 7. Features and Compliance

### 7.1 WebAssembly Specification Support

| Feature | Support |
|---------|---------|
| Core 1.0 | ✅ Full |
| Core 2.0 | ✅ Full |
| Mutable Globals | ✅ |
| Non-Trapping Float-to-Int | ✅ |
| Sign-Extension Operators | ✅ |
| Multi-Value | ✅ |
| SIMD | ✅ |
| Reference Types | ✅ |

### 7.2 Platform Support

| Platform | Compiler | Interpreter |
|----------|----------|-------------|
| Linux/amd64 | ✅ | ✅ |
| Linux/arm64 | ✅ | ✅ |
| macOS/amd64 | ✅ | ✅ |
| macOS/arm64 | ✅ | ✅ |
| Windows/amd64 | ✅ | ✅ |
| Linux/riscv64 | ❌ | ✅ |

---

## 8. Related Components

| Component | Path | Description |
|-----------|------|-------------|
| VM | `../vm/` | Uses wazero as runtime |
| rust-sdk | `../rust-sdk/` | SDK compiled to WASM |
| go-sdk | `../go-sdk/` | Go SDK for WASM modules |

---

## 9. Documentation References

- **Wazero Docs:** https://wazero.io
- **GoDoc:** https://pkg.go.dev/github.com/tetratelabs/wazero
- **WebAssembly Spec:** https://webassembly.org
- **GitHub:** https://github.com/tetratelabs/wazero

---

*This document was generated as part of a comprehensive Taubyte codebase exploration.*
