---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpaceJam/rio
revised_at: 2026-03-23
---

# Rio Deep Dive - io_uring Bindings for Rust

## Purpose

Rio provides safe, ergonomic bindings for Linux io_uring, leveraging Rust's type system to prevent use-after-free bugs while exposing high-performance async I/O capabilities.

## What is io_uring?

io_uring is a Linux kernel interface (5.1+) that enables:
- **Zero-syscall I/O**: With SQPOLL mode, no syscall needed for submission
- **Batching**: Hundreds of operations in a single syscall
- **True async I/O**: Without O_DIRECT requirements of AIO
- **Zero-copy semantics**: Buffer passing without userspace copies

## Rio's Safety Model

### The Core Problem

io_uring is dangerous: the kernel holds pointers to userspace buffers asynchronously. If userspace frees a buffer before the kernel completes the operation, use-after-free occurs.

### Rio's Solution: Lifetime-Based Guarantees

```rust
// This COMPILES - buffer lives long enough
let ring = rio::new()?;
let file = File::open("data")?;
let mut buf = vec![0; 4096];

let completion = ring.read_at(&file, &mut buf, 0);
completion.wait()?;  // Buffer guaranteed valid until here
// buf now contains data

// This FAILS TO COMPILE - buffer freed too early
let ring = rio::new()?;
let file = File::open("data")?;
let buf = vec![0; 4096];

let completion = ring.write_at(&file, &buf, 0);
drop(buf);  // ERROR: buf borrowed by completion
completion.wait()?;
```

### Type-Level Buffer Permissions

```rust
/// AsIoVec - for read-only buffers (slices, Vec, etc.)
impl<A: ?Sized + AsRef<[u8]>> AsIoVec for A {
    fn into_new_iovec(&self) -> libc::iovec {
        let self_ref: &[u8] = self.as_ref();
        libc::iovec {
            iov_base: self_ref.as_ptr() as *mut _,
            iov_len: self_ref.len(),
        }
    }
}

/// AsIoVecMut - for writable buffers (prevents read-only memory writes)
pub trait AsIoVecMut {}
impl<A: ?Sized + AsMut<[u8]>> AsIoVecMut for A {}

// This prevents:
// ring.read_at(&file, &STATIC_READONLY_BUFFER, 0);
// Because &[u8] doesn't implement AsIoVecMut
```

## Core Architecture

### Rio Ring Structure

```rust
pub struct Rio {
    inner: Arc<Uring>,
    metrics: Histogram,
}

struct Uring {
    sq: SubmissionQueue,   // Submit to kernel
    cq: CompletionQueue,   // Receive completions
    config: Config,
}

pub struct Completion {
    uring: Arc<Uring>,
    user_data: u64,
    _marker: PhantomData<...>,  // Lifetime ties to buffers
}
```

### Completion Flow

```
1. Application calls ring.read_at(&file, &mut buf, offset)
                    │
2. Rio creates io_uring_sqe (submission queue entry)
   - Sets opcode = IORING_OP_READ
   - Sets fd, offset, buffer pointer, length
   - Sets user_data = unique ID
                    │
3. SubmissionQueue::submit() - kernel sees new entries
                    │
4. Kernel reads from disk, writes to buf
                    │
5. Kernel writes io_uring_cqe (completion queue entry)
   - Contains user_data, result bytes/error
                    │
6. Completion::wait() polls CQE, returns Result<usize>
```

## Production Implementation

### Crate Structure

```
rio-reproduction/
├── Cargo.toml
└── src/
    ├── lib.rs           # Public API, traits
    ├── io_uring.rs      # Raw bindings (Linux only)
    ├── completion.rs    # Completion type
    ├── config.rs        # Configuration
    ├── lazy.rs          # Lazy initialization
    ├── metrics.rs       # Optional histogram
    └── ops/
        ├── read.rs      # Read operations
        ├── write.rs     # Write operations
        ├── accept.rs    # TCP accept (5.5+)
        └── connect.rs   # TCP connect
```

### Core Types

```rust
// src/lib.rs
use std::io;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Completion {
    inner: Option<CompletionInner>,
}

struct CompletionInner {
    uring: Arc<Uring>,
    user_data: u64,
}

impl Completion {
    /// Blocking wait for completion
    pub fn wait(mut self) -> io::Result<usize> {
        loop {
            if let Some(cqe) = self.inner.uring.poll_cqe(self.user_data) {
                return Ok(usize::try_from(cqe.res).unwrap());
            }
            thread::park_timeout(Duration::from_micros(100));
        }
    }
}

impl Future for Completion {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if let Some(cqe) = self.inner.uring.poll_cqe(self.user_data) {
            Poll::Ready(Ok(usize::try_from(cqe.res).unwrap()))
        } else {
            // Register waker for when CQE arrives
            self.inner.uring.register_waker(self.user_data, cx.waker());
            Poll::Pending
        }
    }
}
```

### Raw io_uring Bindings

```rust
// src/io_uring.rs - Simplified
#[repr(C)]
pub struct io_uring_sqe {
    pub opcode: u8,
    pub flags: u8,
    pub ioprio: u16,
    pub fd: i32,
    pub off: u64,
    pub addr: u64,
    pub len: u32,
    pub user_data: u64,
    // ... union for different op types
}

#[repr(C)]
pub struct io_uring_cqe {
    pub user_data: u64,
    pub res: i32,
    pub flags: u32,
}

// Syscall wrappers
unsafe fn io_uring_setup(entries: u32, p: &mut io_uring_params) -> i32 {
    libc::syscall(SYS_io_uring_setup, entries, p) as i32
}

unsafe fn io_uring_enter(
    fd: i32,
    to_submit: u32,
    min_complete: u32,
    flags: u32,
    arg: *const libc::c_void,
    size: usize,
) -> i32 {
    libc::syscall(SYS_io_uring_enter, fd, to_submit, min_complete, flags, arg, size) as i32
}
```

### Memory-Mapped Queues

```rust
struct SubmissionQueue {
    ring_ptr: *mut libc::c_void,
    ring_mask: u32,
    ring_size: u32,
    head: *mut AtomicU32,
    tail: *mut AtomicU32,
    array: *mut AtomicU32,
}

impl SubmissionQueue {
    unsafe fn get_sqe(&mut self) -> Option<&mut io_uring_sqe> {
        let head = (*self.head).load(Ordering::Acquire);
        let tail = (*self.tail).load(Ordering::Relaxed);
        let next_tail = tail.wrapping_add(1);

        if next_tail - head > self.ring_size {
            return None;  // Queue full
        }

        let idx = tail & self.ring_mask;
        Some(&mut *self.array.add(idx as usize).cast::<io_uring_sqe>())
    }

    fn submit(&mut self) {
        let tail = (*self.tail).load(Ordering::Relaxed);
        (*self.tail).store(tail.wrapping_add(1), Ordering::Release);

        // Notify kernel
        io_uring_enter(self.fd, 1, 0, IORING_ENTER_GETEVENTS, ptr::null(), 0);
    }
}
```

## Advanced Features

### Ordered Operations (Linking)

```rust
/// Chain operations with ordering guarantees
pub enum Ordering {
    /// Next operation depends on this one succeeding
    Link,
    /// Hard link - next waits for this unconditionally
    HardLink,
}

// Example: Write then read (O_DIRECT)
let write = ring.write_at_ordered(&file, &out_buf, offset, Ordering::Link);
let read = ring.read_at(&file, &in_buf, offset);
// read will not start until write completes
```

### TCP Accept (Linux 5.5+)

```rust
pub fn accept(&self, listener: &TcpListener) -> io::Result<TcpStream> {
    let completion = self.inner.accept(listener);
    let fd = completion.wait()?;
    unsafe { Ok(TcpStream::from_raw_fd(fd as i32)) }
}

// Usage in async echo server
async fn serve(ring: &Rio, listener: TcpListener) -> io::Result<()> {
    loop {
        let stream = ring.accept(&listener).await?;
        tokio::spawn(handle_connection(stream));
    }
}
```

### Buffer Registration (Zero-Copy)

```rust
// Register buffers upfront for reduced overhead
let ring = Config::default()
    .registered_buffers(vec![aligned_buf1, aligned_buf2])
    .start()?;

// Use registered buffer index instead of pointer
let completion = ring.read_at_registered(&file, buffer_idx: 0, offset);
// Avoids kernel mapping/unmapping overhead per operation
```

## Performance Characteristics

### Benchmarks (from rio README)

| Operation | Thread Pool | io_uring (rio) | Speedup |
|-----------|-------------|----------------|---------|
| 4K random read | 100K IOPS | 400K IOPS | 4x |
| 256K sequential write (O_DIRECT) | 500 MB/s | 2000 MB/s | 4x |
| TCP accept under load | 50K/s | 200K/s | 4x |

### Memory Behavior

```
Thread Pool Approach:
┌─────────┐  ┌─────────┐  ┌─────────┐
│ Thread 1│  │ Thread 2│  │ Thread 3│
└────┬────┘  └────┬────┘  └────┬────┘
     │           │           │
     ▼           ▼           ▼
┌─────────────────────────────────┐
│    Kernel (syscall per I/O)     │
└─────────────────────────────────┘

io_uring Approach:
┌─────────────────────────────────┐
│     Single Thread (rio)         │
│  ┌─────────────────────────┐    │
│  │  Submission Queue (SQ)  │    │
│  └───────────┬─────────────┘    │
│              │ 0 syscalls       │
│  ┌───────────▼─────────────┐    │
│  │  Completion Queue (CQ)  │    │
│  └─────────────────────────┘    │
└─────────────────────────────────┘
               │
               ▼
┌─────────────────────────────────┐
│    Kernel (batched processing)  │
└─────────────────────────────────┘
```

## Edge Cases and Pitfalls

### 1. Buffer Alignment for O_DIRECT

```rust
// O_DIRECT requires alignment to block size (typically 4096)
#[repr(align(4096))]
struct Aligned([u8; 4096 * 256]);

let buf = Aligned([0; 4096 * 256]);
let file = OpenOptions::new()
    .write(true)
    .custom_flags(libc::O_DIRECT)
    .open("file")?;

// This works - properly aligned
ring.write_at(&file, &buf.0, 0).wait()?;

// Without alignment, returns EINVAL
```

### 2. Completion Queue Overflow

Rio prevents CQE overflow (which would lose completions):

```rust
impl Rio {
    fn submit_with_backpressure(&mut self) {
        loop {
            // Check if CQ has space
            if self.cq_has_space() {
                self.sq.submit();
                break;
            }
            // Poll completions to make room
            self.poll_completions();
            // Back off if still full
            thread::yield_now();
        }
    }
}
```

### 3. File Descriptor Lifetime

```rust
// FD must outlive the Completion
let file = File::open("data")?;
let completion = ring.read_at(&file, &mut buf, 0);
drop(file);  // BUG: FD invalid, kernel may fail

// Fix: Completion should conceptually hold FD reference
// Rio tracks this via the uring Arc, but user must ensure
// file isn't closed independently
```

## Comparison to Other io_uring Crates

| Crate | Safety | Async | Features | Notes |
|-------|--------|-------|----------|-------|
| rio | Compile-time | Yes | TCP, FS, O_DIRECT | Type-level safety |
| io-uring | Runtime checks | Yes | More ops | Easier to misuse |
| tokio-uring | Compile-time | Tokio-only | Limited | Tokio ecosystem |
| iou | Unsafe | No | All ops | Maximum control |

## When to Use Rio

**Good fit:**
- Linux 5.1+ only
- High IOPS requirements
- Batch operations
- O_DIRECT workloads

**Poor fit:**
- Cross-platform (Windows, macOS)
- Kernel < 5.1
- Simple I/O patterns
- Prefer async runtimes (consider tokio-uring)
