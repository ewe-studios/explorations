# Rubri Deep Dive

## Overview

Rubri is a proof-of-concept wrapper for Miri (Rust's interpreter) that runs in the browser. It enables instant Rust code execution without compilation by interpreting Rust's Mid-level Intermediate Representation (MIR) directly in WebAssembly.

## Project Structure

```
rubri/
├── example/                # Demo application (Astro/Vite)
│   ├── src/
│   │   ├── interpreter/
│   │   │   ├── wasi/
│   │   │   │   ├── wasi.ts         # WASI implementation
│   │   │   │   ├── wasi_defs.ts    # WASI constants/types
│   │   │   │   ├── fd.ts           # File descriptor abstraction
│   │   │   │   ├── fs_mem.ts       # In-memory filesystem
│   │   │   │   ├── fs_opfs.ts      # Origin Private FS
│   │   │   │   └── debug.ts        # Debug logging
│   │   │   ├── interpreter.ts      # Main interpreter class
│   │   │   └── worker.ts           # Web Worker entry
│   │   ├── editor.ts               # Code editor integration
│   │   └── pages/index.astro       # Demo page
│   ├── public/wasm-rustc/
│   │   └── bin/miri.opt.*.wasm     # Miri WASM binary
│   └── package.json
├── LICENSE
└── README.md
```

## Core Architecture

### Interpreter Class

The heart of Rubri - manages Miri execution:

```typescript
class Interpreter {
  readonly miri: WebAssembly.Module;
  readonly wasi: WASI;
  readonly fds: [Stdio, Stdio, Stdio, OpenDirectory, OpenDirectory, OpenDirectory];
  readonly stdin: Stdio;
  readonly stdout: Stdio;
  readonly stderr: Stdio;
  next_thread_id: number;

  constructor(
    miri: WebAssembly.Module,
    wasi: WASI,
    fds: [...],
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio
  ) {
    this.miri = miri;
    this.wasi = wasi;
    this.fds = fds;
    this.next_thread_id = 1;
  }

  async run(code: string, printLast: boolean = false): Promise<string> {
    // Clear buffers
    this.stdin.clear();
    this.stdout.clear();
    this.stderr.clear();

    // Wrap code in closure
    code = `let _code = (|| {\n${code}\n})();`;
    if (printLast) {
      code += '\nif std::any::Any::type_id(&_code) != std::any::TypeId::of::<()>() {
                 println!("{_code:?}") }';
    }

    // Set main.rs contents
    this.fds[5].dir.get_file("main.rs")!.data = encode(`fn main() {\n${code}\n}`);

    // Instantiate Miri
    const inst = await WebAssembly.instantiate(this.miri, {
      "env": {
        memory: new WebAssembly.Memory({
          initial: 256,
          maximum: 1024 * 4,
          shared: false
        })
      },
      "wasi": {
        "thread-spawn": function(start_arg) {
          let thread_id = this.next_thread_id++;
          inst.exports.wasi_thread_start(thread_id, start_arg);
          return thread_id;
        }
      },
      "wasi_snapshot_preview1": strace(this.wasi.wasiImport, ["fd_prestat_get"])
    });

    // Execute
    try {
      console.time("miri execution");
      this.wasi.start(inst);
      console.timeEnd("miri execution");
    } catch (e) {
      return this.stdout.text() || e.message;
    }

    return this.stdout.text();
  }
}
```

### Initialization Flow

```typescript
export async function initInterpreter(): Promise<Interpreter> {
  console.time("init");

  // Setup I/O buffers
  const out: Uint8Array[] = [];
  const stdin = new Stdio(out);
  const stdout = new Stdio(out);
  const stderr = new Stdio(out);

  // Setup filesystem
  const tmp = new PreopenDirectory("/tmp", []);
  const root = new PreopenDirectory("/", [["main.rs", new File([])]]);

  // Load Miri and sysroot in parallel
  const [miri, sysroot] = await Promise.all([
    WebAssembly.compileStreaming(
      cached_or_fetch("/wasm-rustc/bin/miri.opt.1718474653.wasm")
    ),
    buildSysroot()
  ]);

  // Setup WASI
  const fds: [...] = [stdin, stdout, stderr, tmp, sysroot, root];
  const env: string[] = [];

  // Miri flags for performance
  const args = [
    "miri",
    "--sysroot", "/sysroot",
    "main.rs",
    "--target", "x86_64-unknown-linux-gnu",
    "-Zmir-opt-level=3",                    // Maximum MIR optimization
    "-Zmiri-ignore-leaks",                  // Don't check for leaks
    "-Zmiri-permissive-provenance",         // Relaxed memory model
    "-Zmiri-preemption-rate=0",             // No thread preemption
    "-Zmiri-disable-alignment-check",       // Skip alignment checks
    "-Zmiri-disable-data-race-detector",    // No race detection
    "-Zmiri-disable-stacked-borrows",       // No borrow checker
    "-Zmiri-disable-validation",            // Skip validation
    "-Zmir-emit-retag=false",               // No retagging
    "-Zmiri-disable-isolation",             // Allow syscalls
    "-Zmiri-panic-on-unsupported",          // Fail fast
    "--color=always"                        // Color output
  ];

  const wasi = new WASI(args, env, fds, { debug: false });

  console.timeEnd("init");
  return new Interpreter(miri, wasi, fds, stdin, stdout, stderr);
}
```

## WASI Implementation

### Core WASI Class

```typescript
export class WASIProcExit extends Error {
  constructor(public readonly code: number) {
    super("exit with exit code " + code);
  }
}

export default class WASI {
  args: Array<string> = [];
  env: Array<string> = [];
  fds: Array<Fd> = [];
  inst: { exports: { memory: WebAssembly.Memory } };
  wasiImport: { [key: string]: (...args: Array<any>) => unknown };

  start(instance: {
    exports: { memory: WebAssembly.Memory; _start: () => unknown }
  }) {
    this.inst = instance;
    try {
      instance.exports._start();
      return 0;
    } catch (e) {
      if (e instanceof WASIProcExit) {
        return e.code;
      }
      throw e;
    }
  }

  constructor(args: Array<string>, env: Array<string>, fds: Array<Fd>, options: Options = {}) {
    this.args = args;
    this.env = env;
    this.fds = fds;
    this.inst = {
      exports: {
        memory: new WebAssembly.Memory({ initial: 0, maximum: 0, shared: false })
      }
    };

    const self = this;
    this.wasiImport = {
      // Argument handling
      args_sizes_get(argc: number, argv_buf_size: number): number {
        const buffer = new DataView(self.inst.exports.memory.buffer);
        buffer.setUint32(argc, self.args.length, true);
        let buf_size = 0;
        for (const arg of self.args) {
          buf_size += arg.length + 1;
        }
        buffer.setUint32(argv_buf_size, buf_size, true);
        return 0;
      },

      args_get(argv: number, argv_buf: number): number {
        const buffer = new DataView(self.inst.exports.memory.buffer);
        const buffer8 = new Uint8Array(self.inst.exports.memory.buffer);
        for (let i = 0; i < self.args.length; i++) {
          buffer.setUint32(argv, argv_buf, true);
          argv += 4;
          const arg = new TextEncoder().encode(self.args[i]);
          buffer8.set(arg, argv_buf);
          buffer.setUint8(argv_buf + arg.length, 0);
          argv_buf += arg.length + 1;
        }
        return 0;
      },

      // Environment variables
      environ_sizes_get(environ_count: number, environ_size: number): number {
        const buffer = new DataView(self.inst.exports.memory.buffer);
        buffer.setUint32(environ_count, self.env.length, true);
        let buf_size = 0;
        for (const environ of self.env) {
          buf_size += environ.length + 1;
        }
        buffer.setUint32(environ_size, buf_size, true);
        return 0;
      },

      // Clock functions
      clock_time_get(id: number, precision: bigint, time: number): number {
        const buffer = new DataView(self.inst.exports.memory.buffer);
        if (id === wasi.CLOCKID_REALTIME) {
          buffer.setBigUint64(time, BigInt(new Date().getTime()) * 1_000_000n, true);
        } else if (id === wasi.CLOCKID_MONOTONIC) {
          let monotonic_time = BigInt(Math.round(performance.now() * 1000000));
          buffer.setBigUint64(time, monotonic_time, true);
        }
        return 0;
      },

      // File descriptor operations
      fd_read(fd: number, iovs_ptr: number, iovs_len: number, nread_ptr: number): number {
        const buffer = new DataView(self.inst.exports.memory.buffer);
        const buffer8 = new Uint8Array(self.inst.exports.memory.buffer);
        if (self.fds[fd] !== undefined) {
          const iovecs = wasi.Iovec.read_bytes_array(buffer, iovs_ptr, iovs_len);
          let nread = 0;
          for (const iovec of iovecs) {
            const { ret, data } = self.fds[fd].fd_read(iovec.buf_len);
            if (ret !== wasi.ERRNO_SUCCESS) {
              buffer.setUint32(nread_ptr, nread, true);
              return ret;
            }
            buffer8.set(data, iovec.buf);
            nread += data.length;
          }
          buffer.setUint32(nread_ptr, nread, true);
          return wasi.ERRNO_SUCCESS;
        }
        return wasi.ERRNO_BADF;
      },

      fd_write(fd: number, iovs_ptr: number, iovs_len: number, nwritten_ptr: number): number {
        const buffer = new DataView(self.inst.exports.memory.buffer);
        const buffer8 = new Uint8Array(self.inst.exports.memory.buffer);
        if (self.fds[fd] !== undefined) {
          const iovecs = wasi.Ciovec.read_bytes_array(buffer, iovs_ptr, iovs_len);
          let nwritten = 0;
          for (const iovec of iovecs) {
            const data = buffer8.slice(iovec.buf, iovec.buf + iovec.buf_len);
            const { ret, nwritten: nwritten_part } = self.fds[fd].fd_write(data);
            if (ret !== wasi.ERRNO_SUCCESS) {
              buffer.setUint32(nwritten_ptr, nwritten, true);
              return ret;
            }
            nwritten += nwritten_part;
          }
          buffer.setUint32(nwritten_ptr, nwritten, true);
          return wasi.ERRNO_SUCCESS;
        }
        return wasi.ERRNO_BADF;
      },

      // Process exit
      proc_exit(exit_code: number) {
        throw new WASIProcExit(exit_code);
      },

      // Random number generation
      random_get(buf: number, buf_len: number) {
        const buffer8 = new Uint8Array(self.inst.exports.memory.buffer)
          .subarray(buf, buf + buf_len);
        if ("crypto" in globalThis) {
          for (let i = 0; i < buf_len; i += 65536) {
            crypto.getRandomValues(buffer8.subarray(i, i + 65536));
          }
        } else {
          for (let i = 0; i < buf_len; i++) {
            buffer8[i] = (Math.random() * 256) | 0;
          }
        }
      }
    };
  }
}
```

### Stdio Implementation

```typescript
class Stdio extends Fd {
  private out: Uint8Array[];

  constructor(out: Uint8Array[] = []) {
    super();
    this.out = out;
  }

  fd_write(data: Uint8Array): { ret: number, nwritten: number } {
    this.out.push(data);
    return { ret: 0, nwritten: data.byteLength };
  }

  clear() {
    this.out.length = 0;
  }

  text(): string {
    const decoder = new TextDecoder("utf-8");
    let string = "";
    for (const b of this.out) {
      string += decoder.decode(b);
    }
    return string;
  }
}
```

## Sysroot Building

The sysroot contains pre-compiled Rust standard library files:

```typescript
async function buildSysroot(): Promise<PreopenDirectory> {
  return new PreopenDirectory("/sysroot", [
    ["lib", new Directory([
      ["rustlib", new Directory([
        ["wasm32-wasi", new Directory([
          ["lib", new Directory([])]
        ])],
        ["x86_64-unknown-linux-gnu", new Directory([
          ["lib", new Directory(await (async function() {
            let dir = new Map();
            let files = [
              "libaddr2line-b8754aeb03c02354.rlib",
              "libadler-05c3545f6cd12159.rlib",
              "liballoc-0dab879bc41cd6bd.rlib",
              "libcfg_if-c7fd2cef50341546.rlib",
              "libcompiler_builtins-a99947d020d809d6.rlib",
              "libcore-4b8e8a815d049db3.rlib",
              "libgetopts-bbb75529e85d129d.rlib",
              "libgimli-598847d27d7a3cbf.rlib",
              "libhashbrown-d2ff91fdf93cacb2.rlib",
              "liblibc-dc63949c664c3fce.rlib",
              "libmemchr-2d3a423be1a6cb96.rlib",
              "libminiz_oxide-b109506a0ccc4c6a.rlib",
              "libobject-7b48def7544c748b.rlib",
              "libpanic_abort-c93441899b93b849.rlib",
              "libpanic_unwind-11d9ba05b60bf694.rlib",
              "libproc_macro-1a7f7840bb9983dc.rlib",
              "librustc_demangle-59342a335246393d.rlib",
              "libstd-bdedb7706a556da2.rlib",
              "libstd-bdedb7706a556da2.so",
              "libtest-f06fa3fbc201c558.rlib",
              // ... more libraries
            ].map(async (file) => {
              dir.set(file, await load_external_file(
                "/wasm-rustc/lib/rustlib/x86_64-unknown-linux-gnu/lib/" + file
              ));
            });
            await Promise.all(files);
            return dir;
          })())]
        ])]
      ])]
    ])]
  ]);
}
```

## File Caching

Large WASM files (>10MB) are manually cached:

```typescript
async function cached_or_fetch(path: string) {
  const base = import.meta.env.BASE_URL === "/" ? "" : import.meta.env.BASE_URL;
  path = base + path;

  try {
    caches;
  } catch (e) {
    return await fetch(path, { cache: "force-cache" });
  }

  const cache = await caches.open("rust-quest");
  const cached = await cache.match(path);
  if (cached) {
    return cached;
  }

  const file = await fetch(path);
  cache.put(path, file.clone());
  return file;
}

async function load_external_file(path: string) {
  return new File(
    await cached_or_fetch(path)
      .then(b => b.blob())
      .then(b => b.arrayBuffer())
  );
}
```

## Performance Optimizations

### Disabled Miri Checks

For maximum speed, all safety checks are disabled:

```typescript
const args = [
  "miri",
  // ...
  "-Zmir-opt-level=3",                    // Optimize MIR
  "-Zmiri-ignore-leaks",                  // No leak detection
  "-Zmiri-permissive-provenance",         // Relaxed memory model
  "-Zmiri-preemption-rate=0",             // No thread switching
  "-Zmiri-disable-alignment-check",       // No alignment checks
  "-Zmiri-disable-data-race-detector",    // No race detection
  "-Zmiri-disable-stacked-borrows",       // No borrow checking
  "-Zmiri-disable-validation",            // No validation
  "-Zmir-emit-retag=false",               // No retagging
  "-Zmiri-disable-isolation",             // Allow external calls
  "-Zmiri-panic-on-unsupported",          // Fail fast on errors
];
```

### Memory Configuration

```typescript
const memory = new WebAssembly.Memory({
  initial: 256,    // 16MB initial
  maximum: 1024 * 4,  // 256MB maximum
  shared: false
});
```

## Usage Example

```typescript
// Initialize interpreter
const interpreter = await initInterpreter();

// Run Rust code
const result = await interpreter.run(`
    fn fibonacci(n: u32) -> u32 {
        match n {
            0 => 0,
            1 => 1,
            _ => fibonacci(n - 1) + fibonacci(n - 2),
        }
    }

    println!("Fibonacci(10) = {}", fibonacci(10));
`);

console.log(result);
// Output: Fibonacci(10) = 55
```

### With Auto-Print

```typescript
// Auto-print non-unit return values
const result = await interpreter.run(`
    let x = 42;
    x * 2
`, true);

console.log(result);
// Output: 84
```

## Limitations

1. **Single File**: Only `main.rs` is supported
2. **No Dependencies**: Cannot use external crates
3. **Limited I/O**: Only stdout/stderr work
4. **No FFI**: Foreign function interface not available
5. **Performance**: ~1000x slower than native (Miri's claim)
6. **Output Speed**: Simple loop of 2000 prints takes 2-3 seconds

### Unsupported Features

| Feature | Status | Reason |
|---------|--------|--------|
| Multiple files | ❌ | Filesystem limitations |
| External crates | ❌ | No cargo support |
| FFI | ❌ | WASM limitations |
| Async/await | ⚠️ | Limited runtime support |
| SIMD | ❌ | WASM SIMD not enabled |
| Threads | ⚠️ | Limited support |

## How It Works

```
┌─────────────────────────────────────────────────────────────┐
│  User provides Rust code                                    │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  Code wrapped in fn main() { ... }                          │
│  Written to virtual main.rs                                 │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  Miri WASM instantiated with:                               │
│  - WebAssembly.Memory (16-256MB)                            │
│  - WASI imports                                             │
│  - Virtual filesystem                                       │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  Miri executes:                                             │
│  1. Parses Rust code                                        │
│  2. Generates MIR (Mid-level IR)                            │
│  3. Interprets MIR instructions                             │
│  4. Writes output to stdout                                 │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  Output captured and returned as string                     │
└─────────────────────────────────────────────────────────────┘
```

## Comparison: Rubri vs Rubrc

| Aspect | Rubri (Miri) | Rubrc (rustc) |
|--------|--------------|---------------|
| Purpose | Interpretation | Compilation |
| Startup Time | ~2-3s | ~5-10s |
| Execution Speed | ~1000x slower | 10-100x slower |
| Output | stdout only | Binary/exe |
| Use Case | Learning, testing | Building |
| Memory | 16-256MB | 1GB+ |
| Thread Support | Limited | Full (slow) |

## License

MIT License
