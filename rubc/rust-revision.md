# Reproducing Rubc in Rust - Production Guide

## Overview

This guide explains how to reproduce the Rubc (Ruby Compiler - rustc in browser) functionality using pure Rust at a production level. The goal is to create a browser-based Rust compilation/execution environment with production-quality code.

## Architecture Reference

```
┌─────────────────────────────────────────────────────────────────┐
│                      Browser Environment                         │
│                                                                  │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────┐ │
│  │   rustc.wasm    │  │  WASI Runtime    │  │  Filesystem    │ │
│  │   (LLVM/Cranelift)│ │  (Multi-thread) │  │  (Virtual)     │ │
│  └─────────────────┘  └──────────────────┘  └────────────────┘ │
│                                                                  │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────┐ │
│  │  Sysroot        │  │  Toolchain       │  │  Executor      │ │
│  │  (std lib)      │  │  (clang, lld)    │  │  (WASM/CLI)    │ │
│  └─────────────────┘  └──────────────────┘  └────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │   Native/Server Components    │
              │   - Sysroot serving           │
              │   - Toolchain distribution    │
              │   - Build orchestration       │
              └───────────────────────────────┘
```

## Production Requirements

### 1. WASM Module Preparation

**Current Approach (Rubrc)**:
- Pre-compiled rustc.wasm hosted on CDN
- 39MB+ brotli-compressed
- Loaded dynamically at runtime

**Production Rust Approach**:

```rust
// Build script for rustc WASM
// build.rs
use std::process::Command;

fn main() {
    // Configure rustc build for WASM target
    let rustc_source = std::env::var("RUSTC_SOURCE")
        .expect("RUSTC_SOURCE must be set");

    // Build with LLVM backend
    let status = Command::new("python3")
        .args([
            "x.py",
            "build",
            "--target", "wasm32-unknown-emscripten",
            "--host", "wasm32-unknown-emscripten",
            "compiler/rustc",
        ])
        .current_dir(&rustc_source)
        .status()
        .expect("Failed to build rustc");

    assert!(status.success());
}
```

**Recommended: Use Existing Toolchain**

```rust
// Cargo.toml
[dependencies]
wasmtime = "15"
wasmtime-wasi = "15"
wasi-cap-std-sync = "15"
cap-std = "2"
cap-rand = "2"

[profile.release]
lto = true
opt-level = "s"
strip = true
```

### 2. WASI Runtime Implementation

**Option A: Wasmtime (Recommended for production)**

```rust
// src/wasi_runtime.rs
use wasmtime::*;
use wasmtime_wasi::*;
use cap_std::fs::Dir;
use cap_std::ambient_authority;

pub struct RustcRuntime {
    engine: Engine,
    store: Store<WasiCtx>,
}

impl RustcRuntime {
    pub fn new() -> Result<Self> {
        let engine = Engine::default();

        // Configure WASI
        let mut builder = WasiCtxBuilder::new();
        builder
            .inherit_stdio()
            .inherit_args()?
            .env("CARGO_PKG_NAME", "rustc-wasm")?;

        // Setup virtual filesystem
        let preopen_dir = Dir::open_ambient_dir("/sysroot", ambient_authority())?;
        builder.preopen_dir(preopen_dir, "/sysroot")?;

        let wasi = builder.build();
        let mut store = Store::new(&engine, wasi);

        Ok(Self { engine, store })
    }

    pub fn load_module(&mut self, wasm_path: &str) -> Result<Instance> {
        let module = Module::from_file(self.store.engine(), wasm_path)?;

        // Create WASI imports
        let wasi = Wasi::new(&self.store);
        let imports = wasi.get_imports(&module);

        let instance = Instance::new(&mut self.store, &module, &imports)?;
        Ok(instance)
    }

    pub fn run_rustc(&mut self, args: &[String]) -> Result<i32> {
        // Update args
        self.store.data_mut().set_args(args)?;

        // Get _start function
        let start = self
            .store
            .data_mut()
            .get_export("_start")?
            .unwrap_func()?;

        // Execute
        match start.call(&mut self.store, &[], &mut []) {
            Ok(_) => Ok(0),
            Err(e) => {
                if let Some(exit) = e.downcast_ref::<wasmtime_wasi::I32Exit>() {
                    Ok(exit.0)
                } else {
                    Err(e)
                }
            }
        }
    }
}
```

**Option B: Custom WASI Implementation**

```rust
// src/custom_wasi.rs
use std::collections::HashMap;
use std::io::{Read, Write};

pub struct CustomWasi {
    memory: Vec<u8>,
    fds: HashMap<u32, Box<dyn WasiFd>>,
    args: Vec<String>,
    env: HashMap<String, String>,
}

trait WasiFd {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;
}

impl CustomWasi {
    pub fn new() -> Self {
        let mut fds = HashMap::new();
        fds.insert(0, Box::new(StdinFd::new()) as Box<dyn WasiFd>);
        fds.insert(1, Box::new(StdoutFd::new()) as Box<dyn WasiFd>);
        fds.insert(2, Box::new(StderrFd::new()) as Box<dyn WasiFd>);

        Self {
            memory: vec![0; 64 * 1024 * 1024], // 64MB default
            fds,
            args: Vec::new(),
            env: HashMap::new(),
        }
    }

    // WASI syscall implementations
    pub fn args_get(&self, argv: u32, argv_buf: u32) -> Result<i32> {
        // Implement args_sizes_get
        let argc = self.args.len() as u32;
        let mut buf_size = 0;
        for arg in &self.args {
            buf_size += arg.len() as u32 + 1;
        }

        // Write to memory...
        Ok(0)
    }

    pub fn fd_read(&mut self, fd: u32, iovs_ptr: u32) -> Result<i32> {
        if let Some(wasi_fd) = self.fds.get_mut(&fd) {
            // Read from FD using iovs
            // ...
        }
        Ok(8) // ERRNO_BADF
    }

    pub fn fd_write(&mut self, fd: u32, iovs_ptr: u32) -> Result<i32> {
        if let Some(wasi_fd) = self.fds.get_mut(&fd) {
            // Write to FD using iovs
            // ...
        }
        Ok(8) // ERRNO_BADF
    }
}
```

### 3. Virtual Filesystem

**cap-std Based Implementation**:

```rust
// src/virtual_fs.rs
use cap_std::fs::{Dir, OpenOptions};
use cap_std::ambient_authority;
use std::path::Path;

pub struct VirtualSysroot {
    base_dir: Dir,
    overlays: HashMap<String, Vec<u8>>,
}

impl VirtualSysroot {
    pub fn new(sysroot_path: &str) -> Result<Self> {
        let base_dir = Dir::open_ambient_dir(sysroot_path, ambient_authority())?;
        Ok(Self {
            base_dir,
            overlays: HashMap::new(),
        })
    }

    pub fn from_archive(archive: &[u8]) -> Result<Self> {
        // Decompress and extract tar archive
        let decompressed = brotli_decompress(archive)?;
        let mut overlays = HashMap::new();

        let mut tar = tar::Archive::new(&decompressed[..]);
        for entry in tar.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_string_lossy().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data)?;
            overlays.insert(path, data);
        }

        Ok(Self {
            base_dir: Dir::open_ambient_dir("/", ambient_authority())?,
            overlays,
        })
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        // Check overlays first
        if let Some(data) = self.overlays.get(path) {
            return Ok(data.clone());
        }

        // Fall back to base directory
        self.base_dir.read(path)
    }

    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<()> {
        self.overlays.insert(path.to_string(), data.to_vec());
        Ok(())
    }
}
```

**In-Memory Filesystem**:

```rust
// src/mem_fs.rs
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct MemFs {
    inner: Arc<RwLock<MemFsInner>>,
}

struct MemFsInner {
    root: FsNode,
}

enum FsNode {
    File(Vec<u8>),
    Directory(HashMap<String, FsNode>),
}

impl MemFs {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MemFsInner {
                root: FsNode::Directory(HashMap::new()),
            })),
        }
    }

    pub fn create_file(&self, path: &str, data: &[u8]) -> Result<()> {
        let mut inner = self.inner.write().unwrap();
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();

        let mut current = &mut inner.root;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part - create file
                if let FsNode::Directory(dir) = current {
                    dir.insert(part.to_string(), FsNode::File(data.to_vec()));
                }
            } else {
                // Navigate/create directory
                if let FsNode::Directory(dir) = current {
                    if !dir.contains_key(*part) {
                        dir.insert(part.to_string(), FsNode::Directory(HashMap::new()));
                    }
                    current = dir.get_mut(*part).unwrap();
                }
            }
        }
        Ok(())
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let inner = self.inner.read().unwrap();
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();

        let mut current = &inner.root;
        for part in parts {
            if let FsNode::Directory(dir) = current {
                if let Some(node) = dir.get(part) {
                    current = node;
                } else {
                    return Err(Error::NotFound);
                }
            } else {
                return Err(Error::NotADirectory);
            }
        }

        if let FsNode::File(data) = current {
            Ok(data.clone())
        } else {
            Err(Error::IsADirectory)
        }
    }
}
```

### 4. Sysroot Distribution

**Server-side Serving**:

```rust
// server/src/main.rs
use actix_web::{web, App, HttpResponse, HttpServer};
use std::path::PathBuf;

async fn get_sysroot(
    triple: web::Path<String>,
    data: web::Data<AppData>,
) -> HttpResponse {
    let tar_path = data.sysroot_dir.join(format!("{}.tar.br", triple));

    match tokio::fs::read(&tar_path).await {
        Ok(data) => HttpResponse::Ok()
            .content_type("application/octet-stream")
            .body(data),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

struct AppData {
    sysroot_dir: PathBuf,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let sysroot_dir = PathBuf::from("/path/to/sysroots");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppData {
                sysroot_dir: sysroot_dir.clone(),
            }))
            .route("/sysroot/{triple}", web::get().to(get_sysroot))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

**Pre-generated Sysroots**:

```bash
#!/bin/bash
# scripts/build-sysroots.sh

TRIPLES=(
    "wasm32-wasip1"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-musl"
)

for triple in "${TRIPLES[@]}"; do
    echo "Building sysroot for $triple..."

    # Create temporary directory
    TEMP_DIR=$(mktemp -d)
    SYSROOT_DIR="$TEMP_DIR/$triple"

    # Install standard library
    rustup target add "$triple"
    cp -r "$(rustc --print sysroot)/lib/rustlib/$triple/lib" "$SYSROOT_DIR/lib"

    # Create tarball
    tar -cf - -C "$TEMP_DIR" "$triple" | brotli > "sysroots/${triple}.tar.br"

    rm -rf "$TEMP_DIR"
done
```

### 5. Multi-threading Support

**Using Wasmtime with Threads**:

```rust
// src/threaded_runtime.rs
use wasmtime::*;
use wasmtime_wasi::*;
use std::thread;

pub struct ThreadedRuntime {
    engine: Engine,
}

impl ThreadedRuntime {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_threads(true);
        config.wasm_reference_types(true);
        config.wasm_bulk_memory(true);

        let engine = Engine::new(&config)?;
        Ok(Self { engine })
    }

    pub fn spawn_thread(
        &self,
        module: &Module,
        args: Vec<String>,
    ) -> Result<thread::JoinHandle<i32>> {
        let engine = self.engine.clone();

        Ok(thread::spawn(move || {
            let mut store = Store::new(&engine, WasiCtxBuilder::new().build());

            // Setup WASI
            let wasi = Wasi::new(&store);
            let imports = wasi.get_imports(module);

            let instance = Instance::new(&mut store, module, &imports).unwrap();

            // Execute
            let start = instance.get_func(&mut store, "_start").unwrap();
            match start.call(&mut store, &[], &mut []) {
                Ok(_) => 0,
                Err(e) => {
                    if let Some(exit) = e.downcast_ref::<I32Exit>() {
                        exit.0
                    } else {
                        panic!("{}", e);
                    }
                }
            }
        }))
    }
}
```

**Using Rayon for Parallel Compilation**:

```rust
// src/parallel_compile.rs
use rayon::prelude::*;

pub struct ParallelCompiler {
    runtime: RustcRuntime,
}

impl ParallelCompiler {
    pub fn compile_multiple(
        &mut self,
        sources: &[(&str, Vec<String>)],
    ) -> Vec<Result<i32>> {
        sources
            .par_iter()
            .map(|(source, args)| {
                // Create isolated runtime for each compilation
                let mut runtime = self.runtime.clone();
                runtime.run_rustc(args)
            })
            .collect()
    }
}
```

### 6. Complete Production Example

```rust
// src/main.rs
mod wasi_runtime;
mod virtual_fs;
mod sysroot;

use wasi_runtime::RustcRuntime;
use virtual_fs::VirtualSysroot;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rustc-wasm")]
#[command(about = "Run rustc in WASM environment")]
struct Args {
    /// Input Rust file
    #[arg(short, long)]
    input: String,

    /// Target triple
    #[arg(short, long, default_value = "wasm32-wasip1")]
    target: String,

    /// Output directory
    #[arg(short, long, default_value = "/tmp")]
    out_dir: String,

    /// Sysroot path
    #[arg(long)]
    sysroot: Option<String>,

    /// Additional rustc flags
    #[arg(last = true)]
    rustc_args: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize sysroot
    let sysroot = if let Some(path) = args.sysroot {
        VirtualSysroot::new(&path)?
    } else {
        // Download from CDN
        let sysroot_data = sysroot::download_sysroot(&args.target)?;
        VirtualSysroot::from_archive(&sysroot_data)?
    };

    // Initialize runtime
    let mut runtime = RustcRuntime::new()?;

    // Build rustc arguments
    let mut rustc_args = vec![
        "rustc".to_string(),
        args.input.clone(),
        "--sysroot".to_string(), "/sysroot".to_string(),
        "--target".to_string(), args.target.clone(),
        "--out-dir".to_string(), args.out_dir.clone(),
        "-Ccodegen-units=1".to_string(),
    ];
    rustc_args.extend(args.rustc_args);

    // Run rustc
    let exit_code = runtime.run_rustc(&rustc_args)?;

    if exit_code == 0 {
        println!("Compilation successful!");
    } else {
        eprintln!("Compilation failed with exit code: {}", exit_code);
        std::process::exit(exit_code);
    }

    Ok(())
}
```

### 7. Cargo Integration

```rust
// src/cargo_support.rs
use std::process::Command;

pub struct CargoRunner {
    sysroot_path: String,
    target: String,
}

impl CargoRunner {
    pub fn new(sysroot_path: String, target: String) -> Self {
        Self { sysroot_path, target }
    }

    pub fn build(&self, manifest_path: &str) -> Result<()> {
        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--manifest-path")
            .arg(manifest_path)
            .arg("--target")
            .arg(&self.target)
            .env("RUSTC", "rustc-wasm")
            .env("CARGO_TARGET_DIR", "/tmp/cargo-target")
            .env("RUSTFLAGS", format!("-L{}", self.sysroot_path));

        let output = cmd.output()?;

        if !output.status.success() {
            eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            return Err("Build failed".into());
        }

        Ok(())
    }
}
```

### 8. Error Handling and Diagnostics

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RustcWasmError {
    #[error("WASM module load failed: {0}")]
    ModuleLoad(#[from] wasmtime::Error),

    #[error("WASI execution failed: {0}")]
    WasiExecution(String),

    #[error("Sysroot not found for target: {0}")]
    SysrootNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Compilation failed with exit code: {0}")]
    CompilationFailed(i32),
}

pub type Result<T> = std::result::Result<T, RustcWasmError>;
```

## Production Checklist

| Component | Status | Notes |
|-----------|--------|-------|
| WASM Module | ✓ | Use pre-built rustc.wasm |
| WASI Runtime | ✓ | Wasmtime recommended |
| Virtual FS | ✓ | cap-std or in-memory |
| Sysroot | ✓ | Pre-built tarballs |
| Threading | ✓ | Wasmtime or rayon |
| Error Handling | ✓ | thiserror |
| CLI Interface | ✓ | clap |
| Cargo Support | ⚠️ | Limited |
| LSP Support | ❌ | Future work |
| Incremental Build | ❌ | Future work |

## Performance Considerations

1. **WASM Module Size**: Use brotli compression (~70% reduction)
2. **Sysroot Caching**: Cache decompressed sysroot in memory
3. **Memory Limits**: Configure appropriate limits (1-4GB)
4. **Parallel Compilation**: Use rayon for multi-crate builds
5. **Incremental Builds**: Leverage rustc's incremental compilation

## License

MIT OR Apache-2.0
