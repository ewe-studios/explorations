# Rubrc Deep Dive

## Overview

Rubrc is a port of the rustc compiler to WebAssembly, enabling Rust compilation directly in the browser. This deep dive explores the architecture, implementation details, and usage patterns.

## Project Structure

```
rubrc/
├── lib/                    # Core library (TypeScript)
│   ├── src/
│   │   ├── index.ts        # Main entry point
│   │   ├── get_rustc_wasm.ts  # WASM fetcher
│   │   ├── get_llvm_wasm.ts   # LLVM tools fetcher
│   │   ├── sysroot.ts      # Sysroot loader
│   │   ├── brotli_stream.ts   # Brotli decompression
│   │   └── parse_tar.ts    # Tar parser
│   ├── package.json
│   └── README.md
└── page/                   # Frontend application (SolidJS)
    ├── src/
    │   ├── wasm/
    │   │   ├── worker_process/  # Web workers
    │   │   └── rustc.ts    # Rustc wrapper
    │   ├── compile_and_run.ts
    │   ├── cmd_parser.ts   # Command dispatcher
    │   ├── ctx.ts          # Context management
    │   └── cat.ts          # File access
    └── package.json
```

## Core Architecture

### WASM Module Loading

The rustc WASM module is fetched from a CDN and decompressed on-the-fly:

```typescript
// get_rustc_wasm.ts
export const get_rustc_wasm = () =>
  get_wasm("https://oligamiq.github.io/rust_wasm/v0.2.0/rustc_opt.wasm.br");

// brotli_stream.ts
export const fetch_compressed_stream = async (url: string) => {
  const compressed_stream = await fetch(url);
  const decompression_stream = await get_brotli_decompress_stream();
  return compressed_stream.body.pipeThrough(decompression_stream);
};
```

### Brotli Decompression

Uses `brotli-dec-wasm` for streaming decompression:

```typescript
const OUTPUT_SIZE = 1024 * 1024; // 1MB output buffer

export const get_brotli_decompress_stream = async (): Promise<
  TransformStream<Uint8Array, Uint8Array>
> => {
  await init(brotli_dec_wasm_bg);

  const decompressStream = new BrotliDecStream();
  return new TransformStream({
    transform(chunk, controller) {
      let inputOffset = 0;
      do {
        const input = chunk.slice(inputOffset);
        const result = decompressStream.decompress(input, OUTPUT_SIZE);
        controller.enqueue(result.buf);
        inputOffset += result.input_offset;
      } while (resultCode === BrotliStreamResultCode.NeedsMoreOutput);
    }
  });
};
```

### Tar Parsing

Sysroot libraries are packaged as tarballs and parsed in-browser:

```typescript
export async function parseTar(
  readable_stream: ReadableStream<Uint8Array>,
  callback: (file: TarFileItem) => void,
) {
  const reader = readable_stream.getReader();
  let buffer = new Uint8Array(0);

  while (true) {
    // Ensure we have header (512 bytes)
    while (buffer.length < 512 && !done) {
      await check_stream();
    }

    // Parse header fields
    const name = _readString(buffer, 0, 100);
    if (name.length === 0) break;

    const size = _readNumber(buffer, 124, 12); // octal

    // Read file data
    while (buffer.length < 512 + size) {
      await check_stream();
    }
    const data = buffer.slice(512, 512 + size);

    callback({ name, type, size, data, text: () => ... });

    // Skip to next header (512-byte aligned)
    buffer = buffer.slice(adjusted_size);
  }
}
```

## Sysroot Management

The sysroot contains Rust standard library files for each target:

```typescript
export const load_sysroot_part = async (triple: string): Promise<Directory> => {
  const decompressed_stream = await fetch_compressed_stream(
    `https://oligamiq.github.io/rust_wasm/v0.2.0/${triple}.tar.br`
  );

  const dir = new Map<string, Inode>();

  await parseTar(decompressed_stream, (file) => {
    if (file.name.includes("/")) {
      const parts = file.name.split("/");
      // Create nested directory structure
      created_dir.contents.set(parts.slice(1).join("/"), new File(file.data));
    } else {
      dir.set(file.name, new File(file.data));
    }
  });

  return new Directory(dir);
};

export const load_default_sysroot = async (): Promise<PreopenDirectory> => {
  const sysroot_part = await load_sysroot_part("wasm32-wasip1");
  rustlib_dir = new Directory([
    ["wasm32-wasip1", new Directory([["lib", sysroot_part]])]
  ]);
  return new PreopenDirectory("/sysroot", [
    ["lib", new Directory([["rustlib", rustlib_dir]])]
  ]);
};
```

## WASI Farm Architecture

Multi-threaded WASM execution requires careful resource management:

```typescript
// animals.ts - WASIFarm manages shared resources
export class WASIFarm {
  private fds: Array<Fd>;
  private park: WASIFarmPark;
  private can_array_buffer: boolean;

  constructor(
    stdin?: Fd, stdout?: Fd, stderr?: Fd,
    fds: Array<Fd> = [],
    options: { allocator_size?: number } = {}
  ) {
    this.fds = [stdin, stdout, stderr, ...fds].filter(Boolean);

    // Check SharedArrayBuffer support
    try {
      new SharedArrayBuffer(4);
      this.can_array_buffer = true;
    } catch (e) {
      this.can_array_buffer = false;
    }

    this.park = new WASIFarmParkUseArrayBuffer(
      this.fds_ref(), stdin_, stdout_, stderr_,
      default_allow_fds, options?.allocator_size
    );
  }

  get_ref(): WASIFarmRefObject {
    return this.park.get_ref();
  }
}
```

### WASIFarmAnimal - Per-Process State

Each process (rustc, llvm, linker) gets its own "animal":

```typescript
export class WASIFarmAnimal {
  args: Array<string>;
  env: Array<string>;
  private wasi_farm_refs: WASIFarmRef[];
  private fd_map: Array<[number, number]>; // [fd, wasi_ref_n]

  start(instance: { exports: { memory; _start } }) {
    this.inst = instance;
    try {
      instance.exports._start();
      if (this.can_thread_spawn) {
        this.thread_spawner.done_notify(0);
      }
      return 0;
    } catch (e) {
      if (e instanceof WASIProcExit) {
        return e.code;
      }
      throw e;
    }
  }

  // Thread spawn for multi-threaded WASM
  wasi_thread_start(instance, thread_id: number, start_arg: number) {
    this.inst = instance;
    try {
      instance.exports.wasi_thread_start(thread_id, start_arg);
      return 0;
    } catch (e) {
      if (e instanceof WASIProcExit) {
        return e.code;
      }
      throw e;
    }
  }
}
```

## Command Pipeline

The command parser routes commands to appropriate handlers:

```typescript
// cmd_parser.ts
const cmd_parser = new SharedObject((...args) => {
  const cmd = args[0];

  const llvm_tools = [
    "symbolizer", "addr2line", "size", "objdump", "otool",
    "objcopy", "strip", "cxxfilt", "ar", "ranlib",
    "lld", "lld-link", "ld.lld", "wasm-ld", "clang"
  ];

  if (cmd === "rustc") {
    await terminal("executing rustc...\r\n");
    await rustc(...args.slice(1));
  } else if (cmd === "clang" || cmd === "llvm") {
    await terminal("executing llvm...\r\n");
    await clang(...args.slice());
  } else if (llvm_tools.includes(cmd)) {
    await clang(...["llvm", ...args.slice()]);
  } else if (cmd === "ls" || cmd === "tree") {
    await ls(...args.slice(1));
  } else if (cmd === "download") {
    await download(args[1]);
  } else if (cmd.includes("/")) {
    // Execute file directly
    await exec_file(...args);
  }
}, ctx.cmd_parser_id);
```

## Compile and Run Flow

```typescript
export const compile_and_run = async (triple: string) => {
  // Setup rustc arguments
  const exec = [
    "rustc",
    "/main.rs",
    "--sysroot",
    "/sysroot",
    "--target",
    triple,
    "--out-dir",
    "/tmp",
    "-Ccodegen-units=1",
  ];

  if (triple === "wasm32-wasip1") {
    exec.push("-Clinker-flavor=wasm-ld");
    exec.push("-Clinker=wasm-ld");
  } else {
    exec.push("-Clinker=lld");
  }

  // Execute rustc
  await cmd_parser(...exec);

  // Wait for completion
  while (!(await waiter.is_cmd_run_end())) {
    await new Promise(resolve => setTimeout(resolve, 100));
  }

  // Run or download result
  if (triple === "wasm32-wasip1") {
    await cmd_parser("/tmp/main.wasm");
  } else {
    await cmd_parser("download", "/tmp/main");
  }
};
```

## File Access Pattern

Virtual filesystem path resolution for cross-process file access:

```typescript
// cat.ts - Get file data from virtual filesystem
export const get_data = (path__: string, animal: WASIFarmAnimal): Uint8Array => {
  // Find root FD
  let root_fd: number;
  const dir_names: Map<number, string> = new Map();
  for (let fd = 0; fd < animal.fd_map.length; fd++) {
    const [mapped_fd, wasi_farm_ref] = animal.get_fd_and_wasi_ref(fd);
    const [prestat, ret] = wasi_farm_ref.fd_prestat_get(mapped_fd);
    if (prestat && prestat[0] === wasi.PREOPENTYPE_DIR) {
      const [path, ret] = wasi_farm_ref.fd_prestat_dir_name(mapped_fd, prestat[1]);
      dir_names.set(fd, new TextDecoder().decode(path));
      if (path === "/") root_fd = fd;
    }
  }

  // Find most specific match
  let matched_fd = root_fd;
  let matched_dir_len = 1;
  const parts_path = path__.split("/");
  for (const [fd, dir_name] of dir_names) {
    const parts_dir_name = dir_name.split("/");
    let dir_len = 0;
    for (let i = 0; i < parts_dir_name.length; i++) {
      if (parts_dir_name[i] === parts_path[i]) dir_len++;
    }
    if (dir_len > matched_dir_len) {
      matched_fd = fd;
      matched_dir_len = dir_len;
    }
  }

  // Open and read file
  const rest_path = parts_path.slice(matched_dir_len).join("/");
  const [opened_fd, ret] = animal.wasi_farm_refs[wasi_farm_ref_n].path_open(
    mapped_fd, 0, new TextEncoder().encode(rest_path), 0, 0n, 0n, 0
  );

  let file_data = new Uint8Array();
  let offset = 0n;
  while (true) {
    const [nread_and_buf, ret] = animal.wasi_farm_refs[wasi_farm_ref_n]
      .fd_pread(opened_fd, iovs, offset);
    if (nread === 0) break;
    // Append to file_data...
    offset += BigInt(nread);
  }

  return file_data;
};
```

## Supported Targets

| Target | Description | Status |
|--------|-------------|--------|
| `wasm32-wasip1` | WebAssembly WASI | ✓ Full support |
| `x86_64-unknown-linux-musl` | Static Linux | ✓ Works |
| `x86_64-pc-windows-gnu` | Windows PE | Partial |
| Other targets | Various | Linking issues |

## Limitations

1. **Thread Spawn**: Very slow due to WASM worker creation overhead
2. **Memory**: Limited by browser (typically 1-2GB)
3. **Linking**: Some targets fail during linking phase
4. **COOP/COEP**: Requires cross-origin isolation headers
5. **Cold Start**: 5-10 seconds to load WASM module

## Performance Optimizations

1. **Brotli Compression**: Reduces WASM size by ~70%
2. **Streaming Decompression**: No need to buffer entire file
3. **SharedArrayBuffer**: Zero-copy cross-thread communication
4. **Sysroot Caching**: Libraries cached after first load
5. **Web Worker Pool**: Pre-spawned workers for parallel tasks

## Usage Example

```typescript
import {
  load_default_sysroot,
  get_default_sysroot_wasi_farm
} from "@rubrc/lib";

// Initialize
const farm = await get_default_sysroot_wasi_farm();
const ref = farm.get_ref();

// Compile Rust code
const rustc_args = [
  "rustc",
  "/main.rs",
  "--sysroot",
  "/sysroot",
  "--target",
  "wasm32-wasip1",
  "-o",
  "/tmp/main.wasm"
];

// Execute via WASI
const exit_code = await execute_wasm(rustc_wasm, rustc_args, ref);
```

## License

MIT OR Apache-2.0
