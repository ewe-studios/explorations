# browser_wasi_shim Deep Dive

## Overview

browser_wasi_shim is a TypeScript implementation of WASI (WebAssembly System Interface) for browsers. It provides the system call layer that allows WASM modules to interact with the host environment through standardized interfaces.

## Project Structure

```
browser_wasi_shim/
├── src/
│   ├── index.ts            # Main exports
│   ├── wasi.ts             # Core WASI implementation
│   ├── wasi_defs.ts        # WASI constants and types
│   ├── fd.ts               # File descriptor abstraction
│   ├── fs_mem.ts           # In-memory filesystem
│   ├── fs_opfs.ts          # Origin Private Filesystem
│   ├── debug.ts            # Debug logging
│   └── strace.ts           # Syscall tracing
├── threads/                # Multi-threading extension
│   ├── src/
│   │   ├── animals.ts      # WASIFarm and WASIFarmAnimal
│   │   ├── farm.ts         # Farm management
│   │   ├── park.ts         # Resource park
│   │   ├── ref.ts          # Reference management
│   │   ├── sender.ts       # Message sending
│   │   └── shared_array_buffer/
│   │       └── index.ts    # SAB-based implementation
│   └── examples/
└── package.json
```

## WASI Core Implementation

### WASI Constants and Types

```typescript
// wasi_defs.ts

// Error codes
export const ERRNO_SUCCESS = 0;
export const ERRNO_BADF = 8;
export const ERRNO_NOSYS = 58;
export const ERRNO_NAMETOOLONG = 63;
// ... more error codes

// File descriptor flags
export const FDFLAGS_APPEND = 0x0001;
export const FDFLAGS_DSYNC = 0x0002;
export const FDFLAGS_NONBLOCK = 0x0004;
export const FDFLAGS_RSYNC = 0x0008;
export const FDFLAGS_SYNC = 0x0010;

// File types
export const FILETYPE_DIRECTORY = 3;
export const FILETYPE_REGULAR_FILE = 4;
export const FILETYPE_SOCKET_STREAM = 6;
export const FILETYPE_SYMBOLIC_LINK = 7;

// Clock IDs
export const CLOCKID_REALTIME = 0;
export const CLOCKID_MONOTONIC = 1;

// Preopen types
export const PREOPENTYPE_DIR = 0;
```

### File Descriptor Abstraction

```typescript
// fd.ts
export abstract class Fd {
  fd_advise(offset: bigint, len: bigint, advice: number): number {
    return ERRNO_NOSYS;
  }

  fd_allocate(offset: bigint, len: bigint): number {
    return ERRNO_NOSYS;
  }

  fd_close(): number {
    return ERRNO_SUCCESS;
  }

  fd_datasync(): number {
    return ERRNO_SUCCESS;
  }

  fd_fdstat_get(): { ret: number; fdstat: Fdstat | null } {
    return { ret: ERRNO_NOSYS, fdstat: null };
  }

  fd_fdstat_set_flags(flags: number): number {
    return ERRNO_NOSYS;
  }

  fd_fdstat_set_rights(
    fs_rights_base: bigint,
    fs_rights_inheriting: bigint
  ): number {
    return ERRNO_NOSYS;
  }

  fd_filestat_get(): { ret: number; filestat: Filestat | null } {
    return { ret: ERRNO_NOSYS, filestat: null };
  }

  fd_filestat_set_size(size: bigint): number {
    return ERRNO_NOSYS;
  }

  fd_filestat_set_times(atim: bigint, mtim: bigint, fst_flags: number): number {
    return ERRNO_NOSYS;
  }

  fd_pread(len: number, offset: bigint): { ret: number; data: Uint8Array } {
    return { ret: ERRNO_NOSYS, data: new Uint8Array() };
  }

  fd_pwrite(data: Uint8Array, offset: bigint): { ret: number; nwritten: number } {
    return { ret: ERRNO_NOSYS, nwritten: 0 };
  }

  fd_read(len: number): { ret: number; data: Uint8Array } {
    return { ret: ERRNO_NOSYS, data: new Uint8Array() };
  }

  fd_readdir_single(cookie: bigint): {
    ret: number;
    dirent: Dirent | null;
  } {
    return { ret: ERRNO_NOSYS, dirent: null };
  }

  fd_seek(offset: bigint, whence: number): { ret: number; offset: bigint } {
    return { ret: ERRNO_NOSYS, offset: 0n };
  }

  fd_sync(): number {
    return ERRNO_SUCCESS;
  }

  fd_tell(): { ret: number; offset: bigint } {
    return { ret: ERRNO_NOSYS, offset: 0n };
  }

  fd_write(data: Uint8Array): { ret: number; nwritten: number } {
    return { ret: ERRNO_NOSYS, nwritten: 0 };
  }

  path_create_directory(path: string): number {
    return ERRNO_NOSYS;
  }

  path_filestat_get(flags: number, path: string): {
    ret: number;
    filestat: Filestat | null;
  } {
    return { ret: ERRNO_NOSYS, filestat: null };
  }

  path_link(new_path: string, inode_obj: unknown, replace: boolean): number {
    return ERRNO_NOSYS;
  }

  path_lookup(
    path: string,
    flags: number
  ): { ret: number; inode_obj: unknown | null } {
    return { ret: ERRNO_NOSYS, inode_obj: null };
  }

  path_open(
    dirflags: number,
    path: string,
    oflags: number,
    fs_rights_base: bigint,
    fs_rights_inheriting: bigint,
    fd_flags: number
  ): { ret: number; fd_obj: Fd | null } {
    return { ret: ERRNO_NOSYS, fd_obj: null };
  }

  path_readlink(path: string): { ret: number; data: string | null } {
    return { ret: ERRNO_NOSYS, data: null };
  }

  path_remove_directory(path: string): number {
    return ERRNO_NOSYS;
  }

  path_unlink(path: string): { ret: number; inode_obj: unknown | null } {
    return { ret: ERRNO_NOSYS, inode_obj: null };
  }

  path_unlink_file(path: string): number {
    return ERRNO_NOSYS;
  }

  fd_prestat_get(): { ret: number; prestat: [number, number] | null } {
    return { ret: ERRNO_BADF, prestat: null };
  }
}
```

### In-Memory Filesystem

```typescript
// fs_mem.ts
export class OpenDirectory extends Fd {
  contents: Map<string, Inode>;
  parent?: OpenDirectory;
  name?: string;

  constructor(
    contents: Map<string, Inode> = new Map(),
    parent?: OpenDirectory,
    name?: string
  ) {
    super();
    this.contents = contents;
    this.parent = parent;
    this.name = name;
  }

  fd_fdstat_get(): { ret: number; fdstat: Fdstat | null } {
    const fdstat = new Fdstat();
    fdstat.fs_filetype = FILETYPE_DIRECTORY;
    return { ret: ERRNO_SUCCESS, fdstat };
  }

  fd_readdir_single(cookie: bigint): {
    ret: number;
    dirent: Dirent | null;
  } {
    const keys = Array.from(this.contents.keys());
    const index = Number(cookie);

    if (index >= keys.length) {
      return { ret: ERRNO_SUCCESS, dirent: null };
    }

    const name = keys[index];
    const entry = this.contents.get(name)!;

    const dirent = new Dirent();
    dirent.d_next = BigInt(index + 1);
    dirent.d_ino = 0;
    dirent.d_namlen = name.length;
    dirent.d_type = entry instanceof Directory
      ? FILETYPE_DIRECTORY
      : FILETYPE_REGULAR_FILE;
    dirent.name = new TextEncoder().encode(name);

    return { ret: ERRNO_SUCCESS, dirent };
  }

  path_lookup(path: string, flags: number): {
    ret: number;
    inode_obj: unknown | null;
  } {
    const parts = path.split("/").filter(p => p.length > 0);
    let current: Inode | undefined = this as any;

    for (const part of parts) {
      if (current instanceof Directory) {
        current = current.contents.get(part);
      } else {
        return { ret: ERRNO_NOTDIR, inode_obj: null };
      }
    }

    if (current) {
      return { ret: ERRNO_SUCCESS, inode_obj: current };
    }
    return { ret: ERRNO_NOENT, inode_obj: null };
  }

  path_open(
    dirflags: number,
    path: string,
    oflags: number,
    fs_rights_base: bigint,
    fs_rights_inheriting: bigint,
    fd_flags: number
  ): { ret: number; fd_obj: Fd | null } {
    const { ret, inode_obj } = this.path_lookup(path, dirflags);
    if (ret !== ERRNO_SUCCESS) {
      return { ret, fd_obj: null };
    }

    if (inode_obj instanceof File) {
      return { ret: ERRNO_SUCCESS, fd_obj: inode_obj };
    } else if (inode_obj instanceof Directory) {
      return { ret: ERRNO_SUCCESS, fd_obj: new OpenDirectory(inode_obj.contents, this, path) };
    }

    return { ret: ERRNO_NOENT, fd_obj: null };
  }
}

export class File extends Fd {
  data: Uint8Array;
  position: bigint = 0n;

  constructor(data: Uint8Array) {
    super();
    this.data = data;
  }

  fd_fdstat_get(): { ret: number; fdstat: Fdstat | null } {
    const fdstat = new Fdstat();
    fdstat.fs_filetype = FILETYPE_REGULAR_FILE;
    return { ret: ERRNO_SUCCESS, fdstat };
  }

  fd_filestat_get(): { ret: number; filestat: Filestat | null } {
    const filestat = new Filestat();
    filestat.filetype = FILETYPE_REGULAR_FILE;
    filestat.size = BigInt(this.data.length);
    return { ret: ERRNO_SUCCESS, filestat };
  }

  fd_read(len: number): { ret: number; data: Uint8Array } {
    const end = Number(this.position) + len;
    const data = this.data.slice(Number(this.position), end);
    this.position += BigInt(data.length);
    return { ret: ERRNO_SUCCESS, data };
  }

  fd_write(data: Uint8Array): { ret: number; nwritten: number } {
    const pos = Number(this.position);
    if (pos + data.length > this.data.length) {
      const new_data = new Uint8Array(pos + data.length);
      new_data.set(this.data);
      this.data = new_data;
    }
    this.data.set(data, pos);
    this.position += BigInt(data.length);
    return { ret: ERRNO_SUCCESS, nwritten: data.length };
  }

  fd_seek(offset: bigint, whence: number): { ret: number; offset: bigint } {
    switch (whence) {
      case 0: // SEEK_SET
        this.position = offset;
        break;
      case 1: // SEEK_CUR
        this.position += offset;
        break;
      case 2: // SEEK_END
        this.position = BigInt(this.data.length) + offset;
        break;
    }
    return { ret: ERRNO_SUCCESS, offset: this.position };
  }

  fd_tell(): { ret: number; offset: bigint } {
    return { ret: ERRNO_SUCCESS, offset: this.position };
  }
}

export class PreopenDirectory extends OpenDirectory {
  prestat_name: Uint8Array;

  constructor(path: string, contents: Map<string, Inode> = new Map()) {
    super(contents);
    this.prestat_name = new TextEncoder().encode(path);
  }

  fd_prestat_get(): { ret: number; prestat: [number, number] | null } {
    return {
      ret: ERRNO_SUCCESS,
      prestat: [PREOPENTYPE_DIR, this.prestat_name.length]
    };
  }
}
```

## Multi-threading Extension (threads/)

### WASIFarm - Resource Manager

```typescript
// threads/src/animals.ts
export class WASIFarm {
  private fds: Array<Fd>;
  private park: WASIFarmPark;
  private can_array_buffer: boolean;

  constructor(
    stdin?: Fd,
    stdout?: Fd,
    stderr?: Fd,
    fds: Array<Fd> = [],
    options: { allocator_size?: number } = {}
  ) {
    // Setup FD array
    const new_fds = [];
    let stdin_ = undefined;
    let stdout_ = undefined;
    let stderr_ = undefined;

    if (stdin) {
      new_fds.push(stdin);
      stdin_ = new_fds.length - 1;
    }
    if (stdout) {
      new_fds.push(stdout);
      stdout_ = new_fds.length - 1;
    }
    if (stderr) {
      new_fds.push(stderr);
      stderr_ = new_fds.length - 1;
    }
    new_fds.push(...fds);

    // Check SharedArrayBuffer support
    try {
      new SharedArrayBuffer(4);
      this.can_array_buffer = true;
    } catch (e) {
      this.can_array_buffer = false;
      console.warn("SharedArrayBuffer is not supported:", e);
    }

    this.fds = new_fds;

    // Create park based on SAB support
    if (this.can_array_buffer) {
      this.park = new WASIFarmParkUseArrayBuffer(
        this.fds_ref(),
        stdin_,
        stdout_,
        stderr_,
        default_allow_fds,
        options?.allocator_size
      );
    } else {
      throw new Error("Non SharedArrayBuffer is not supported yet");
    }

    this.park.listen();
  }

  get_ref(): WASIFarmRefObject {
    return this.park.get_ref();
  }
}
```

### WASIFarmAnimal - Per-Process State

```typescript
// threads/src/farm.ts
export class WASIFarmAnimal {
  args: Array<string>;
  env: Array<string>;
  private wasi_farm_refs: WASIFarmRef[];
  private id_in_wasi_farm_ref: Array<number>;
  protected fd_map: Array<[number, number]>; // [fd, wasi_ref_n]

  // Get FD and WASI ref by FD number
  protected get_fd_and_wasi_ref(
    fd: number
  ): [number | undefined, WASIFarmRef | undefined] {
    const mapped_fd_and_wasi_ref_n = this.fd_map[fd];
    if (!mapped_fd_and_wasi_ref_n) {
      return [undefined, undefined];
    }
    const [mapped_fd, wasi_ref_n] = mapped_fd_and_wasi_ref_n;
    return [mapped_fd, this.wasi_farm_refs[wasi_ref_n]];
  }

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
        if (this.can_thread_spawn) {
          this.thread_spawner.done_notify(e.code);
        }
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

  async async_start_on_thread(): Promise<number> {
    if (!this.can_thread_spawn || !this.thread_spawner) {
      throw new Error("thread_spawn is not supported");
    }

    await this.wait_worker_background_worker();
    const view = new Uint8Array(this.get_share_memory().buffer);
    view.fill(0);

    await this.thread_spawner.async_start_on_thread(
      this.args,
      this.env,
      this.fd_map
    );

    const code = await this.thread_spawner.async_wait_done_or_error();
    return code;
  }
}
```

### WASIFarmPark - Resource Park

```typescript
// threads/src/park.ts
export abstract class WASIFarmPark {
  protected fds: Array<Fd>;
  protected fds_map: Array<number[]>; // FD accessibility map
  private get_new_fd_lock = new Array<() => Promise<void>>();

  constructor(
    fds: Array<Fd>,
    stdin: number | undefined,
    stdout: number | undefined,
    stderr: number | undefined,
    default_allow_fds: Array<number>
  ) {
    this.fds = fds;
    this.fds_map = new Array(fds.length);
    for (let i = 0; i < fds.length; i++) {
      this.fds_map[i] = [];
    }
  }

  // Get new FD with locking
  private async get_new_fd(): Promise<[() => Promise<void>, number]> {
    const promise = new Promise<[() => Promise<void>, number]>((resolve) => {
      const len = this.get_new_fd_lock.push(async () => {
        let ret = -1;
        for (let i = 0; i < this.fds.length; i++) {
          if (this.fds[i] === undefined) {
            ret = i;
            break;
          }
        }
        if (ret === -1) {
          ret = this.fds.length;
          (this.fds as Array<Fd | undefined>).push(undefined);
          this.fds_map.push([]);
        }

        const [can, promise] = this.can_set_new_fd(ret);
        if (!can) {
          await promise;
        }

        resolve([
          async () => {
            this.get_new_fd_lock.shift();
            const fn = this.get_new_fd_lock[0];
            if (fn !== undefined) {
              fn();
            }
            await this.notify_set_fd(ret);
          },
          ret
        ]);
      });
      if (len === 1) {
        this.get_new_fd_lock[0]();
      }
    });
    return promise;
  }

  protected async fd_close(fd: number): Promise<number> {
    if (this.fds[fd] !== undefined) {
      const ret = this.fds[fd].fd_close();
      (this.fds as Array<Fd | undefined>)[fd] = undefined;
      await this.notify_rm_fd(fd);
      return ret;
    }
    return ERRNO_BADF;
  }
}
```

## Debug and Tracing

### Debug Logger

```typescript
// debug.ts
let enabled = false;

export const debug = {
  enable: (value: boolean) => {
    enabled = value;
  },

  get enabled() {
    return enabled;
  },

  log: (...args: any[]) => {
    if (enabled) {
      console.log(...args);
    }
  }
};
```

### Syscall Tracer

```typescript
// strace.ts
export function strace(
  wasiImport: { [key: string]: (...args: any[]) => unknown },
  skip: string[] = []
) {
  return new Proxy(wasiImport, {
    get(target, prop) {
      const fn = target[prop as string];
      if (typeof fn !== "function") {
        return fn;
      }

      if (skip.includes(prop as string)) {
        return fn;
      }

      return function (...args: any[]) {
        debug.log(`${String(prop)}(${args.map(a => JSON.stringify(a)).join(", ")})`);
        const result = fn.apply(this, args);
        debug.log(`  -> ${JSON.stringify(result)}`);
        return result;
      };
    }
  });
}
```

## Usage Example

```typescript
import {
  WASI,
  PreopenDirectory,
  File,
  Directory
} from "@bjorn3/browser_wasi_shim";

// Create virtual filesystem
const root = new PreopenDirectory("/", [
  ["hello.txt", new File(new TextEncoder().encode("Hello, World!"))],
  ["data", new Directory(new Map())]
]);

const stdin = /* ... */;
const stdout = /* ... */;
const stderr = /* ... */;

// Setup WASI
const fds = [stdin, stdout, stderr, root];
const args = ["my-program", "--verbose"];
const env = ["RUST_BACKTRACE=1"];

const wasi = new WASI(args, env, fds, { debug: true });

// Execute WASM module
const wasm = await WebAssembly.instantiate(wasmBytes, {
  "wasi_snapshot_preview1": wasi.wasiImport
});

const exitCode = wasi.start(wasm);
console.log(`Exit code: ${exitCode}`);
```

## Multi-threaded Usage

```typescript
import { WASIFarm } from "@oligami/browser_wasi_shim-threads";

// Create farm with SharedArrayBuffer support
const farm = new WASIFarm(
  stdin,
  stdout,
  stderr,
  [sysroot, tmp],
  { allocator_size: 1024 * 1024 * 1024 } // 1GB
);

// Get reference for WASM execution
const ref = farm.get_ref();

// Execute with thread support
const animal = new WASIFarmAnimal(ref);
const exitCode = await animal.async_start_on_thread();
```

## License

MIT OR Apache-2.0
