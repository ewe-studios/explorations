# Rubc Project Exploration

## Overview

The "Rubc" project (Ruby Compiler) is not a Ruby compiler, but rather refers to **Rubrc** and **Rubri** - two related projects that enable running Rust compilation and interpretation directly in the browser through WebAssembly. This exploration covers the architecture and implementation of these browser-based Rust toolchain projects.

## Projects Summary

| Project | Description | Technology |
|---------|-------------|------------|
| **Rubrc** | rustc compiler ported to WebAssembly for browser execution | Rustc + WASM + WASI |
| **Rubri** | Miri (Rust interpreter) wrapped for browser execution | Miri + WASM + WASI |
| **browser_wasi_shim** | WASI implementation for browsers | TypeScript |
| **rust_wasm** | Pre-compiled WASM toolchain storage | WASM binaries |

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Browser Environment                       │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Rubrc (rustc in browser)                │  │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐  │  │
│  │  │ rustc.wasm  │  │ llvm-tools   │  │  WASI Shim      │  │  │
│  │  │ (39MB+)     │  │ (clang, lld) │  │  (multi-thread) │  │  │
│  │  └─────────────┘  └──────────────┘  └─────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Rubri (Miri in browser)                 │  │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐  │  │
│  │  │ miri.wasm   │  │ sysroot      │  │  WASI Shim      │  │  │
│  │  │ (optimized) │  │ (.rlib libs) │  │  (single-thread)│  │  │
│  │  └─────────────┘  └──────────────┘  └─────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │        rust_wasm storage       │
              │  - rustc_opt.wasm.br           │
              │  - sysroot tarballs            │
              │  - llvm-tools                  │
              └───────────────────────────────┘
```

## Core Components

### 1. Rubrc - Rustc in Browser

**Purpose**: Compile Rust code to WASM or native targets directly in the browser.

**Key Features**:
- Runs `rustc` compiler entirely in browser
- Supports targets: `wasm32-wasip1` and `x86_64-unknown-linux-musl`
- Uses brotli-compressed WASM modules (~39MB for rustc)
- Requires COOP/COEP headers for SharedArrayBuffer support

**Supported Targets**:
- `wasm32-wasip1` - WebAssembly WASI preview 1
- `x86_64-unknown-linux-musl` - Static Linux binaries
- `x86_64-pc-windows-gnu` - Windows PE (partial support)

**Limitations**:
- Thread spawn is slow (WASM limitation)
- Linking issues with some targets
- Production use not recommended

### 2. Rubri - Miri Interpreter in Browser

**Purpose**: Execute Rust code instantly in browser without compilation.

**Key Features**:
- Proof-of-concept wrapper for Miri
- Fast iterative development (no compilation)
- Disables UB checks for speed
- Simple loop of 2000 prints takes 2-3 seconds

**Limitations**:
- Doesn't support multiple files
- No crate dependencies
- No I/O operations beyond stdout/stderr
- All Miri/WASM limitations apply

### 3. WASI Shims

Two WASI implementations are used:

#### browser_wasi_shim (Single-thread)
- Basic WASI implementation for browsers
- File system abstraction (PreopenDirectory, File)
- FD (file descriptor) management
- Standard WASI syscalls

#### browser_wasi_shim-threads (Multi-thread)
- Extended WASI with thread support
- `WASIFarm` - manages multiple WASI processes
- `WASIFarmAnimal` - per-process WASI state
- `WASIFarmPark` - shared resource management
- Thread spawn via `wasi_thread_start`
- SharedArrayBuffer-based communication

### 4. rust_wasm Storage

Pre-compiled WASM binaries hosted for distribution:

```
rust_wasm/
├── rustc_cranelift/      # rustc with Cranelift backend (bjorn3)
├── rustc_llvm/           # rustc with LLVM backend (oligamiq)
└── rustc_llvm_with_lld/  # rustc + LLVM + lld linker
```

**Sources**:
- rustc_llvm: https://github.com/oligamiq/rust
- rustc_cranelift: https://github.com/bjorn3/rust
- clang/wasm-ld: https://github.com/YoWASP/clang

## WASM Loading Architecture

### Rubrc WASM Loading Flow

```typescript
// 1. Fetch compressed WASM
get_rustc_wasm() => fetch("rustc_opt.wasm.br")

// 2. Decompress using brotli-dec-wasm
fetch_compressed_stream(url) => brotli_decompress_stream()

// 3. Load sysroot (rust libraries)
load_sysroot_part("wasm32-wasip1") => parseTar(stream)

// 4. Setup WASI filesystem
WASIFarm(stdin, stdout, stderr, [sysroot_fd], {
  allocator_size: 1GB
})

// 5. Execute rustc
rustc_instance.start(args)
```

### Sysroot Loading

The sysroot (Rust standard library) is loaded as brotli-compressed tarballs:

```typescript
// Fetch and decompress sysroot tarball
const decompressed_stream = await fetch_compressed_stream(
  `https://oligamiq.github.io/rust_wasm/v0.2.0/${triple}.tar.br`
);

// Parse tar and build virtual filesystem
await parseTar(decompressed_stream, (file) => {
  dir.set(file.name, new File(file.data));
});
```

## WASM Embedding Patterns

### Pattern 1: Web Worker Isolation

Both projects use Web Workers to isolate WASM execution:

```typescript
// Main thread
const worker = new Worker("rustc_worker.ts");
const shared_obj = new SharedObjectRef(ctx.rustc_id).proxy();

// Worker thread
const rustc = new SharedObject((...args) => {
  // Execute rustc with args
}, ctx.rustc_id);
```

### Pattern 2: SharedArrayBuffer Communication

For multi-threaded WASM (rubrc), SharedArrayBuffer enables:
- Cross-thread memory sharing
- Atomic operations for synchronization
- Thread spawn (`wasi_thread_start`)
- Lock-free data structures

### Pattern 3: Virtual Filesystem

WASI requires a virtual filesystem. Implementation uses:

```typescript
// Preopen directories (WASI terminology)
const sysroot = new PreopenDirectory("/sysroot", [
  ["lib", new Directory([
    ["rustlib", rustlib_dir]
  ])]
]);

// File access through FDs
const fds = [stdin, stdout, stderr, sysroot, tmp, root];
const wasi = new WASI(args, env, fds);
```

## Compiler Pipeline

### Rubrc (rustc) Pipeline

```
Rust Source
     │
     ▼
┌─────────────────┐
│   rustc front   │  Parse + expand macros
│   end           │
└─────────────────┘
     │
     ▼
┌─────────────────┐
│   HIR/MIR       │  High/Mid IR
└─────────────────┘
     │
     ▼
┌─────────────────┐
│   Codegen       │  LLVM or Cranelift
│   (LLVM/Cranelift)│
└─────────────────┘
     │
     ▼
┌─────────────────┐
│   wasm-ld /     │  Linking
│   lld           │
└─────────────────┘
     │
     ▼
WASM Binary / ELF
```

### Rubri (Miri) Pipeline

```
Rust Source
     │
     ▼
┌─────────────────┐
│   rustc         │  Frontend only
│   (no codegen)  │
└─────────────────┘
     │
     ▼
┌─────────────────┐
│   MIR           │  Mid-level IR
└─────────────────┘
     │
     ▼
┌─────────────────┐
│   Miri          │  Interpret MIR
│   interpreter   │
└─────────────────┘
     │
     ▼
stdout/stderr
```

## Performance Characteristics

### Rubrc (rustc in browser)

| Aspect | Performance | Notes |
|--------|-------------|-------|
| WASM Load Time | ~5-10s | 39MB brotli compressed |
| Sysroot Load | ~2-5s | Tar extraction |
| Compilation | 10-100x slower | Than native rustc |
| Thread Spawn | Very slow | WASM limitation |
| Memory | 1GB limit | Configurable allocator |

### Rubri (Miri in browser)

| Aspect | Performance | Notes |
|--------|-------------|-------|
| WASM Load Time | ~2-3s | Optimized miri |
| Startup | ~1s | Sysroot setup |
| Execution | ~1000x slower | Than native (Miri claim) |
| Output | Slow | 2000 prints = 2-3s |
| Memory | 256-1024 pages | ~16-64MB |

## Key Dependencies

### NPM Packages

```json
{
  "@bjorn3/browser_wasi_shim": "WASI implementation",
  "@oligami/browser_wasi_shim-threads": "Multi-thread WASI",
  "@oligami/shared-object": "Cross-worker communication",
  "brotli-dec-wasm": "Brotli decompression",
  "nanotar": "Tar parsing"
}
```

### WASM Modules

- `rustc_opt.wasm` - Optimized rustc compiler
- `miri.opt.wasm` - Optimized Miri interpreter
- `wasm-ld` - WASM linker
- `lld` - LLVM linker
- Various `.rlib` files - Rust standard library

## Implementation Highlights

### 1. Command Parsing and Routing

```typescript
// Central command dispatcher
const cmd_parser = new SharedObject((...args) => {
  const cmd = args[0];

  if (cmd === "rustc") {
    await rustc(...args.slice(1));
  } else if (cmd === "clang" || llvm_tools.includes(cmd)) {
    await clang(...args.slice());
  } else if (cmd.includes("/")) {
    await exec_file(...args);
  }
}, ctx.cmd_parser_id);
```

### 2. Virtual Filesystem Path Resolution

```typescript
// Path resolution in virtual filesystem
export const get_data = (path__: string, animal: WASIFarmAnimal): Uint8Array => {
  // Find best matching preopen directory
  let matched_fd = root_fd;
  let matched_dir_len = 1;
  for (const [fd, dir_name] of dir_names) {
    // Most specific match wins
    if (dir_len > matched_dir_len) {
      matched_fd = fd;
      matched_dir_len = dir_len;
    }
  }

  // Open file relative to matched directory
  const [opened_fd, ret] = wasi_farm_ref.path_open(...);
  return read_file(opened_fd);
};
```

### 3. Thread Synchronization

```typescript
// Waiter pattern for async coordination
const waiter = new SharedObject({
  is_all_done: (): boolean => is_all_done,
  is_cmd_run_end: () => is_cmd_run_end,
  set_end_of_exec: (end: boolean) => { end_of_exec = end; },
}, ctx.waiter_id);

// Polling wait loop
while (!(await waiter.is_cmd_run_end())) {
  await new Promise(resolve => setTimeout(resolve, 100));
}
```

## Related Projects and Credits

### Core Contributors
- **bjorn3**: Cranelift backend, browser_wasi_shim, Miri WASM compilation
- **oligamiq**: LLVM backend for rustc, rust_wasm project
- **whitequark**: LLVM to WASI toolchain
- **LyonSyonII**: Original Rubri concept

### Related Resources
- [Miri WASM Issue](https://github.com/rust-lang/miri/issues/722)
- [LLVM for WebAssembly](https://discourse.llvm.org/t/rfc-building-llvm-for-webassembly/79073)
- [browser_wasi_shim](https://github.com/bjorn3/browser_wasi_shim)

## Demo URLs
- Rubrc: https://oligamiq.github.io/rubrc/
- Rubri: https://garriga.dev/rubri

## License

Both projects are dual-licensed under MIT or Apache-2.0.
