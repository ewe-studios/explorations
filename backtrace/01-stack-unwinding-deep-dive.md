# Stack Unwinding Deep Dive

> **Purpose:** Comprehensive exploration of stack capture and unwinding mechanisms across all Backtrace platforms - from first principles to expert-level implementation details.
>
> **Platforms Covered:** Go, Cocoa (iOS/macOS), JavaScript (Node.js/Browser), Android, Native (Crashpad)
>
> **Explored At:** 2026-04-05

---

## Table of Contents

1. [Introduction to Stack Unwinding](#introduction-to-stack-unwinding)
2. [Go Stack Capture](#go-stack-capture)
3. [Cocoa Stack Capture](#cocoa-stack-capture)
4. [JavaScript Stack Capture](#javascript-stack-capture)
5. [Android Stack Capture](#android-stack-capture)
6. [Native Crash Handling (Crashpad)](#native-crash-handling-crashpad)
7. [Platform Comparison](#platform-comparison)
8. [Performance Considerations](#performance-considerations)

---

## Introduction to Stack Unwinding

### What is Stack Unwinding?

Stack unwinding is the process of walking back through the call stack to determine the sequence of function calls that led to a particular point in program execution. This is fundamental for:

- **Debugging:** Understanding the execution path that led to a crash or error
- **Profiling:** Identifying hot paths and performance bottlenecks
- **Security:** Detecting exploitation attempts and stack corruption
- **Observability:** Providing context for error reporting and analytics

### Stack Frame Anatomy

A stack frame contains:

```
┌─────────────────────────────────────┐
│         Caller's Frame              │
│  ┌───────────────────────────────┐  │
│  │     Return Address (RA)       │  │  <-- Where to resume after call
│  ├───────────────────────────────┤  │
│  │     Previous Frame Pointer    │  │  <-- Link to caller's frame (FP)
│  ├───────────────────────────────┤  │
│  │     Function Arguments        │  │
│  ├───────────────────────────────┤  │
│  │     Local Variables           │  │
│  ├───────────────────────────────┤  │
│  │     Saved Registers           │  │
│  └───────────────────────────────┘  │
│         Current Stack Pointer ──────┼── SP points here
└─────────────────────────────────────┘
```

### Unwinding Strategies

| Strategy | Description | Use Case |
|----------|-------------|----------|
| **Frame Pointer** | Follow FP chain back | Reliable, requires `-fno-omit-frame-pointer` |
| **DWARF CFI** | Use DWARF Call Frame Information | Accurate, works with OMIIT-FP |
| **Stack Scanning** | Scan stack for return addresses | Fallback, less reliable |
| **Exception Tables** | Use language-specific tables | Java, .NET, Go panic handling |
| **Hardware Support** | Use processor debug registers | Advanced profiling, low overhead |

---

## Go Stack Capture

### Overview

Go's runtime provides sophisticated stack capture mechanisms that work with its cooperative scheduler and goroutine model. The stack capture system must handle:

- Hundreds to millions of goroutines
- Stacks that can grow and shrink dynamically
- Split stacks (historical) and contiguous stacks (current)
- GC interaction and safe points

### runtime.Stack() - Goroutine Stack Capture

`runtime.Stack()` captures the stack trace of goroutines as text:

```go
// runtime/stack.go (simplified)
func Stack(buf []byte, all bool) int {
    if all {
        return stackAll(buf)
    }
    return stackOne(buf, getg())
}

func stackOne(buf []byte, g *g) int {
    // Get the current goroutine's stack
    st := g.stack
    
    // Bounds for the stack write
    var sp uintptr
    if g == getg() {
        // Capture current goroutine - use current SP
        sp = getcallersp()
    } else {
        // Capture different goroutine - use saved SP
        // This requires the goroutine to be stopped!
        sp = g.sched.sp
    }
    
    // Walk the stack frames
    return formatStack(buf, st.lo, sp)
}
```

**Key Implementation Details:**

1. **Current vs Other Goroutines:**
   - For the current goroutine, the stack pointer (`SP`) is read directly from registers
   - For other goroutines, the scheduler must have stopped them and saved their register state in `g.sched`

2. **Stack Bounds:**
   ```go
   // Each goroutine has a stack struct
   type stack struct {
       lo uintptr  // Low address (stack base - grows downward)
       hi uintptr  // High address (stack limit)
   }
   ```

3. **Stack Growth:**
   ```go
   // runtime/stack.go
   func newstack() {
       // Called when a goroutine needs more stack space
       // The runtime allocates a new larger stack and copies contents
       // Stack growth is triggered when SP approaches stack.lo
   }
   ```

4. **Format Output:**
   ```go
   // Example output from runtime.Stack()
   goroutine 1 [running]:
   main.main()
       /path/to/main.go:10 +0x25
   runtime.main()
       /usr/local/go/src/runtime/proc.go:250 +0x207
   ```

### runtime.Callers() - Program Counter Capture

`runtime.Callers()` captures raw program counters (PCs) without symbolication:

```go
// runtime/stack.go
func Callers(skip int, pc []uintptr) int {
    // Get the caller's stack pointer
    sp := getcallersp()
    
    // Get the caller's frame pointer  
    fp := getcallersfp()
    
    // Call the internal implementation
    return callers(skip+1, pc, sp, fp)
}

// Internal implementation - walks the stack
func callers(skip int, pc []uintptr, sp, fp uintptr) int {
    n := 0
    
    // Skip the requested number of frames
    for i := 0; i <= skip; i++ {
        if fp == 0 {
            return 0
        }
        // Move to the next frame
        pc_val := *(*uintptr)(unsafe.Pointer(fp + 8))  // Return address
        fp = *(*uintptr)(unsafe.Pointer(fp))           // Previous FP
    }
    
    // Collect PCs
    for fp != 0 && n < len(pc) {
        pc_val := *(*uintptr)(unsafe.Pointer(fp + 8))
        pc[n] = pc_val - 1  // -1 to point to CALL instruction, not return
        n++
        fp = *(*uintptr)(unsafe.Pointer(fp))
    }
    
    return n
}
```

**Key Points:**

1. **PC - 1 Adjustment:** The return address points to the next instruction after CALL. Subtracting 1 points to the CALL itself, which gives better symbolication (shows the calling function, not the callee).

2. **Frame Pointer Chasing:** The implementation follows the frame pointer chain. If frame pointers are omitted (`-gcflags=-l` or certain optimizations), this may fail.

3. **Performance:** Very fast - just pointer chasing. No symbolication or file I/O.

### runtime.CallersFrames() - Frame Iteration

`runtime.CallersFrames()` takes raw PCs and returns structured frame information:

```go
// Example usage
func captureStack() {
    var pcs [100]uintptr
    n := runtime.Callers(1, pcs[:])
    
    frames := runtime.CallersFrames(pcs[:n])
    for {
        frame, more := frames.Next()
        fmt.Printf("Function: %s\n", frame.Function)
        fmt.Printf("File: %s:%d\n", frame.File, frame.Line)
        fmt.Printf("PC: 0x%x\n", frame.PC)
        if !more {
            break
        }
    }
}
```

**Frame Structure:**
```go
type Frame struct {
    // PC is the program counter for the location in this frame.
    PC uintptr
    
    // Function is the function name
    Function string
    
    // File is the file path
    File string
    
    // Line is the line number in the file
    Line int
    
    // Entry is the entry point of the function
    Entry uintptr
    
    // Limit is the end of the function
    Limit uintptr
}
```

**Implementation Flow:**

```
┌─────────────────┐
│ runtime.Callers │  Returns raw []uintptr PCs
└────────┬────────┘
         │
         ▼
┌─────────────────────┐
│ runtime.CallersFrames │ Creates iterator with symbolication context
└────────┬────────────┘
         │
         ▼
┌─────────────────────┐
│ Frames.Next()       │ For each PC:
│                     │ 1. Find function in symbol table
│                     │ 2. Calculate line number from PC offset
│                     │ 3. Return Frame struct
└────────┬────────────┘
         │
         ▼
┌─────────────────────┐
│ Fully symbolicated  │
│ stack trace         │
└─────────────────────┘
```

### Panic Stack Trace Capture

When a panic occurs, Go captures a detailed stack trace:

```go
// runtime/panic.go (simplified)
func gopanic(e interface{}) {
    gp := getg()
    
    // Create panic description
    p := &_panic{
        arg: e,
        link: gp.panic,
    }
    
    // Capture stack trace
    if debug.paniconerror {
        p.stack = p.stack[:runtime.Stack(p.stack[:], false)]
    }
    
    gp.panic = p
    
    // Start unwinding
    paniccall(gp, gp.lr)
}

// paniccall unwinds the stack
func paniccall(gp *g, lr uintptr) {
    for {
        // Run defer functions
        if gp._defer != nil {
            runDefers(gp)
        }
        
        // Check if panic is recovered
        if gp.panic.recovered != nil {
            return  // Panic recovered, continue execution
        }
        
        // No more frames - crash
        fatalpanic(gp)
    }
}
```

**Panic Trace Format:**
```
panic: runtime error: index out of range [5] with length 3

goroutine 1 [running]:
main.indexOutOfBounds(0x5)
    /path/to/main.go:15 +0x45
main.main()
    /path/to/main.go:8 +0x1f
runtime.main()
    /usr/local/go/src/runtime/proc.go:250 +0x207
created by runtime.init
    /usr/local/go/src/runtime/proc.go:249 +0x25
```

### Cross-Goroutine Stack Capture (CaptureAllGoroutines)

The `CaptureAllGoroutines` option in backtrace-go requires stopping all goroutines:

```go
// backtrace-go/internal/stacks/stacks.go
func CaptureAllGoroutines() []GoroutineStack {
    var stacks []GoroutineStack
    
    // Lock the scheduler to freeze all goroutines
    stopTheWorld("stack_capture")
    
    // Iterate all goroutines
    forEachGoroutine(func(g *g) {
        stack := captureGoroutineStack(g)
        stacks = append(stacks, stack)
    })
    
    // Resume execution
    startTheWorld()
    
    return stacks
}

func captureGoroutineStack(g *g) GoroutineStack {
    return GoroutineStack{
        ID:     g.goid,
        State:  g.status,
        Stack:  formatStack(g),
        WaitOn: g.waitLock,  // What the goroutine is waiting on
    }
}
```

**Stop-The-World Mechanics:**

```go
// runtime/proc.go
func stopTheWorld(reason string) {
    // Set GC state to stopping
    setGCState(_GCstopping)
    
    // Send stop signal to all Ps (processors)
    for _, p := range allp {
        p.status = _GCstopreason
    }
    
    // Wait for all Ps to stop
    for readyCount() != 0 {
        osyield()
    }
}
```

**Gotchas:**

1. **STW Duration:** Capturing all goroutine stacks causes a stop-the-world pause. For applications with many goroutines, this can cause noticeable latency spikes.

2. **Deadlock Risk:** If a goroutine holds a lock needed by the scheduler, STW can deadlock.

3. **Goroutine State:** A stopped goroutine's stack is consistent, but a running goroutine's stack may be mid-execution.

### Signal Handler Stack Capture

Go registers signal handlers for crash capture:

```go
// runtime/signal_unix.go
func initsig() {
    for i := range sigtable {
        if !sigtable[i].install {
            continue
        }
        sigaction(uint32(i), &sigact, nil)
    }
}

// Signal handler for crashes
func sigfwd(fn uintptr, sig uint32, info *siginfo, ctx unsafe.Pointer) bool {
    // Called when a signal is received
    
    if sig == SIGSEGV || sig == SIGABRT || sig == SIGQUIT {
        // Capture crash state
        tracebackothers(getg())
        
        // Write crash report
        writeCrashReport(info, ctx)
    }
    
    // Forward to user handler if registered
    return callUserHandler(fn, sig, info, ctx)
}
```

**Signal-Safe Operations:**

Only async-signal-safe functions can be called from signal handlers:

```go
// runtime/signal_unix.go - signal-safe operations
func crashdump() {
    // These are safe in signal context:
    write(2, "crash!\n", 7)  // Direct syscall
    traceback()              // Reads memory, doesn't allocate
    // unsafe.Pointer operations
    // NO: malloc, locks, channels, goroutines
}
```

**SIGSEGV Handling:**

```
┌─────────────────────────────────────────────────────────────────┐
│                    SIGSEGV Received                              │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              Signal Handler Entry                                │
│  - Save signal context (registers)                               │
│  - Determine faulting address                                    │
│  - Check if fault is recoverable                                 │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              Stack Trace Capture                                 │
│  - Capture current goroutine stack                               │
│  - If possible, capture all goroutine stacks                     │
│  - Include register state at crash                               │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              Crash Report Generation                             │
│  - Format stack trace                                            │
│  - Add memory maps (/proc/self/maps)                             │
│  - Add register dump                                             │
│  - Write to stderr or file                                       │
└─────────────────────────────────────────────────────────────────┘
```

### Out-of-Process Tracing with bcd (backtrace-crash-directory)

bcd handles native crashes that Go can't catch internally:

```go
// backtrace-go/bcd/tracer.go
type Tracer struct {
    minidumpWriter *minidump.Writer
    signalChain    SignalChain
    exceptionChain ExceptionChain
}

func NewTracer(opts Options) *Tracer {
    t := &Tracer{
        minidumpWriter: minidump.NewWriter(opts.DatabasePath),
    }
    
    // Register for signals
    t.signalChain.Register([]unix.Signal{
        unix.SIGSEGV,
        unix.SIGABRT,
        unix.SIGBUS,
        unix.SIGILL,
        unix.SIGFPE,
    })
    
    return t
}

func (t *Tracer) signalHandler(sig int, info *unix.Siginfo, ctx unsafe.Pointer) {
    // Write minidump before the process dies
    t.minidumpWriter.Write(sig, ctx)
    
    // Continue signal chain
    t.signalChain.Forward(sig, info, ctx)
}
```

**Minidump Format:**

```
┌─────────────────────────────────────────┐
│          MINIDUMP HEADER                │
│  - Signature (MINIDUMP)                 │
│  - Version                              │
│  - Stream count                         │
└─────────────────┬───────────────────────┘
                  │
        ┌─────────┴─────────┐
        ▼                   ▼
┌───────────────┐   ┌───────────────┐
│ Thread List   │   │ Memory List   │
│ - Thread ID   │   │ - Base Addr   │
│ - Context     │   │ - Size        │
│ - Stack       │   │ - RVA         │
└───────────────┘   └───────────────┘
        │                   │
        ▼                   ▼
┌───────────────┐   ┌───────────────┐
│ Module List   │   │ Exception     │
│ - Name        │   │ - Code        │
│ - Timestamp   │   │ - Address     │
│ - Checksum    │   │ - Flags       │
└───────────────┘   └───────────────┘
```

---

## Cocoa Stack Capture

### Overview

macOS and iOS provide multiple mechanisms for stack capture:

1. **PLCrashReporter** - Open-source crash reporter (used by many SDKs)
2. **OSSymbolicator** - Apple's private symbolication framework
3. **mach_exception** - Mach kernel exception handling
4. **Signal handlers** - POSIX signal handling

### PLCrashReporter Integration

PLCrashReporter is the gold standard for Cocoa crash reporting:

```objc
// PLCrashReporter.h
@interface PLCrashReporter : NSObject

+ (instancetype)sharedReporter;

- (BOOL)enableCrashReporter:(NSError **)error;

- (BOOL)hasPendingCrashReport;
- (NSData *)pendingCrashReportAndReturnError:(NSError **)error;

@end
```

**Initialization:**

```objc
// AppDelegate.m
#import <PLCrashReporter/PLCrashReporter.h>

- (BOOL)application:(UIApplication *)application 
    didFinishLaunchingWithOptions:(NSDictionary *)launchOptions {
    
    PLCrashReporter *reporter = [PLCrashReporter sharedReporter];
    NSError *error = nil;
    
    // Enable crash reporting
    if (![reporter enableCrashReporter:&error]) {
        NSLog(@"Failed to enable crash reporter: %@", error);
    }
    
    // Check for previous crash
    if ([reporter hasPendingCrashReport]) {
        NSData *data = [reporter pendingCrashReportAndReturnError:&error];
        PLCrashReport *report = [[PLCrashReport alloc] initWithData:data error:&error];
        
        // Process the crash report
        [self processCrashReport:report];
        
        // Purge the report
        [reporter purgePendingCrashReport];
    }
    
    return YES;
}
```

**Crash Report Structure:**

```objc
// PLCrashReport.h
@interface PLCrashReport : NSObject

@property(nonatomic, readonly) NSString *systemInfo;
@property(nonatomic, readonly) NSArray<PLCrashReportThreadInfo *> *threads;
@property(nonatomic, readonly) PLCrashReportBinaryImageInfo *image;
@property(nonatomic, readonly) PLCrashReportExceptionInfo *exception;
@property(nonatomic, readonly) PLCrashReportSignalInfo *signal;

@end

// Thread info
@interface PLCrashReportThreadInfo : NSObject

@property(nonatomic, readonly) NSInteger threadNumber;
@property(nonatomic, readonly) NSArray<PLCrashReportStackFrame *> *stackFrames;
@property(nonatomic, readonly) BOOL crashedThread;

@end

// Stack frame
@interface PLCrashReportStackFrame : NSObject

@property(nonatomic, readonly) uint64_t instructionPointer;
@property(nonatomic, readonly, nullable) NSString *symbolName;
@property(nonatomic, readonly, nullable) NSString *libraryName;

@end
```

### Mach Exception Handling

Mach exceptions are the lowest-level crash handling mechanism on Darwin:

```c
// mach/port.h
typedef struct {
    mach_msg_header_t header;
    mach_msg_body_t body;
    mach_msg_port_descriptor_t thread;
    mach_msg_port_descriptor_t task;
    mach_msg_port_descriptor_t exception_port;
    exception_type_t exception;
    mach_exception_data_t code;
    mach_exception_data_t code_count;
} exception_message_t;

// Exception handler
kern_return_t catch_exception_raise(
    mach_port_t exception_port,
    mach_port_t thread_port,
    mach_port_t task_port,
    exception_type_t exception,
    mach_exception_data_t code,
    mach_exception_data_t code_count
) {
    // Capture thread state
    thread_state_data_t state;
    mach_msg_type_number_t state_count = THREAD_STATE_MAX;
    
    thread_get_state(
        thread_port,
        THREAD_STATE_FLAVOR,
        (thread_state_t)&state,
        &state_count
    );
    
    // Capture stack trace from thread state
    capture_stack_from_state(&state);
    
    // Write crash report
    write_minidump(exception, code, &state);
    
    // Kill the process (or handle gracefully)
    task_terminate(task_port);
    
    return KERN_SUCCESS;
}
```

**Mach Exception Flow:**

```
┌────────────────────────────────────────────────────────────────┐
│                    Process Fault                               │
│              (SIGSEGV, EXC_BAD_ACCESS, etc.)                   │
└───────────────────────┬────────────────────────────────────────┘
                        │
                        ▼
┌────────────────────────────────────────────────────────────────┐
│              Mach Kernel Exception Handler                     │
│  - Determines exception type                                   │
│  - Looks up exception port                                     │
│  - Sends exception message                                     │
└───────────────────────┬────────────────────────────────────────┘
                        │
                        ▼
┌────────────────────────────────────────────────────────────────┐
│              Exception Port Receiver                           │
│  - PLCrashReporter mach_port_t                                 │
│  - Receives exception_message_t                                │
│  - Suspends all threads                                        │
└───────────────────────┬────────────────────────────────────────┘
                        │
                        ▼
┌────────────────────────────────────────────────────────────────┐
│              Thread State Capture                              │
│  - thread_get_state() for each thread                          │
│  - x86_64: __x86_THREAD_STATE64                                 │
│  - arm64: ARM_THREAD_STATE64                                    │
│  - Extract instruction pointer, frame pointer, stack pointer   │
└───────────────────────┬────────────────────────────────────────┘
                        │
                        ▼
┌────────────────────────────────────────────────────────────────┐
│              Stack Walk from Thread State                      │
│  - Start from frame pointer                                    │
│  - Follow frame chain                                          │
│  - Or use DWARF CFI for OMF frames                             │
└───────────────────────┬────────────────────────────────────────┘
                        │
                        ▼
┌────────────────────────────────────────────────────────────────┐
│              Crash Report Generation                           │
│  - PLCrashReport format                                        │
│  - Includes all threads, memory maps, etc.                     │
└────────────────────────────────────────────────────────────────┘
```

### Signal Handler Registration

POSIX signal handlers work alongside Mach exceptions:

```c
// signal.c
#include <signal.h>
#include <ucontext.h>

struct sigaction action;
action.sa_sigaction = crash_handler;
action.sa_flags = SA_SIGINFO | SA_ONSTACK;
sigaction(SIGSEGV, &action, NULL);
sigaction(SIGABRT, &action, NULL);
sigaction(SIGBUS, &action, NULL);
sigaction(SIGILL, &action, NULL);
sigaction(SIGTRAP, &action, NULL);

void crash_handler(int sig, siginfo_t *info, void *ctx) {
    ucontext_t *context = (ucontext_t *)ctx;
    
    // Capture register state
    #if defined(__x86_64__)
        uint64_t rip = context->uc_mcontext->__ss.__rip;
        uint64_t rbp = context->uc_mcontext->__ss.__rbp;
        uint64_t rsp = context->uc_mcontext->__ss.__rsp;
    #elif defined(__arm64__)
        uint64_t pc = context->uc_mcontext->__ss.__pc;
        uint64_t fp = context->uc_mcontext->__ss.__fp;
        uint64_t sp = context->uc_mcontext->__ss.__sp;
    #endif
    
    // Walk the stack
    walk_stack(rbp, rsp);
    
    // Generate report
    generate_crash_report(sig, info, rip);
    
    // Re-raise to generate Apple crash report
    signal(sig, SIG_DFL);
    raise(sig);
}
```

### Thread State Capture

Different architectures have different thread state structures:

```c
// x86_64 thread state
typedef struct {
    uint64_t __rax;
    uint64_t __rbx;
    uint64_t __rcx;
    uint64_t __rdx;
    uint64_t __rdi;
    uint64_t __rsi;
    uint64_t __rbp;      // Frame pointer - key for stack walking
    uint64_t __rsp;      // Stack pointer - current stack position
    uint64_t __rip;      // Instruction pointer - current instruction
    uint64_t __rflags;
    // ... more registers
} x86_thread_state64_t;

// ARM64 thread state
typedef struct {
    uint64_t __x[29];    // General purpose registers x0-x28
    uint64_t __fp;       // Frame pointer (x29)
    uint64_t __lr;       // Link register (x30) - return address
    uint64_t __sp;       // Stack pointer (x31)
    uint64_t __pc;       // Program counter
    uint64_t __cpsr;     // Current program status register
} arm_thread_state64_t;
```

**Stack Walking from Thread State:**

```c
// Stack walker using frame pointers
void walk_stack_from_state(uint64_t fp, uint64_t sp) {
    uint64_t *frame = (uint64_t *)fp;
    
    // Validate frame pointer
    if (!is_valid_address(frame)) {
        return;
    }
    
    // Walk frames
    while (is_valid_address(frame)) {
        uint64_t return_addr = frame[1];
        
        // Record the return address
        record_frame(return_addr);
        
        // Move to next frame
        frame = (uint64_t *)frame[0];
    }
}

// Stack walker using DWARF CFI (for OMF binaries)
void walk_stack_with_cfi(uint64_t pc, uint64_t sp) {
    // Find the CFI entry for the current PC
    dwarf_cfi_entry_t *entry = find_cfi_for_pc(pc);
    
    if (entry) {
        // Use CFI rules to recover frame pointer
        uint64_t fp = evaluate_cfi_rule(entry->fp_rule, sp);
        uint64_t ra = evaluate_cfi_rule(entry->ra_rule, sp);
        
        walk_stack_from_state(fp, sp);
    }
}
```

### NSException Stack Traces

Cocoa exceptions can be captured with stack traces:

```objc
// Exception handler
void exceptionHandler(NSException *exception) {
    NSArray *callStackSymbols = [exception callStackSymbols];
    NSArray *callStackAddresses = [exception callStackReturnAddresses];
    
    NSLog(@"Exception: %@", exception.name);
    NSLog(@"Reason: %@", exception.reason);
    NSLog(@"Stack Trace:");
    for (NSString *symbol in callStackSymbols) {
        NSLog(@"  %@", symbol);
    }
    
    // Send to Backtrace
    [BacktraceClient.shared sendException:exception];
}

// Install exception handler
NSSetUncaughtExceptionHandler(&exceptionHandler);
```

**NSException vs Signal:**

| Aspect | NSException | Signal |
|--------|-------------|--------|
| Origin | Objective-C `@throw` | Kernel-level faults |
| Catchable | Yes, with `@try/@catch` | No, process terminates |
| Stack Quality | Full symbolic | May need symbolication |
| Async Safe | No | Yes (handler must be) |

### Async-Safe Stack Capture

Signal handlers must only use async-signal-safe functions:

```c
// Async-safe stack capture
void async_safe_capture(int sig, siginfo_t *info, void *ctx) {
    // Safe: Direct memory access
    ucontext_t *context = (ucontext_t *)ctx;
    uint64_t fp = context->uc_mcontext->__ss.__rbp;
    
    // Safe: Write to file descriptor
    char buffer[256];
    int len = snprintf(buffer, sizeof(buffer), "FP: 0x%llx\n", fp);
    write(STDERR_FILENO, buffer, len);
    
    // Safe: Follow frame pointers
    uint64_t *frame = (uint64_t *)fp;
    for (int i = 0; i < 20 && is_valid(frame); i++) {
        uint64_t ra = frame[1];
        len = snprintf(buffer, sizeof(buffer), "  [%d] 0x%llx\n", i, ra);
        write(STDERR_FILENO, buffer, len);
        frame = (uint64_t *)frame[0];
    }
    
    // UNSAFE - DO NOT DO:
    // malloc(), free()     - heap may be inconsistent
    // printf()             - uses internal buffers/locks
    // Objective-C calls    - may use locks
    // pthread_mutex_lock   - may deadlock
}

// Check if address is valid (async-safe)
static inline int is_valid(uint64_t *addr) {
    if (addr == NULL) return 0;
    if ((uint64_t)addr < 0x1000) return 0;  // NULL page
    // Check against memory map bounds
    return 1;
}
```

### Out-of-Memory (OOM) Detection

OOM events don't generate traditional crashes:

```objc
// OOM detection using memory pressure
#import <os/proc.h>

@interface BacktraceOOMDetector : NSObject

- (void)startMonitoring;
- (void)handleMemoryPressure:(os_proc_exit_status_t)status;

@end

@implementation BacktraceOOMDetector {
    dispatch_source_t _memoryPressureSource;
}

- (void)startMonitoring {
    // Monitor exit status
    dispatch_source_t exitSource = dispatch_source_create(
        DISPATCH_SOURCE_TYPE_PROC,
        getpid(),
        DISPATCH_PROC_EXIT,
        dispatch_get_main_queue()
    );
    
    dispatch_source_set_event_handler(exitSource, ^{
        pid_t pid = getpid();
        os_proc_exit_status_t status = os_proc_exit_status(pid, 0);
        
        if (status & OS_PROC_EXIT_MEMORY_PRESSURE) {
            // OOM detected - save state before termination
            [self saveStateForOOM];
        }
    });
    
    dispatch_resume(exitSource);
}

- (void)saveStateForOOM {
    // Cannot malloc or allocate - write directly
    int fd = open([self oomReportPath], O_WRONLY | O_CREAT);
    
    // Write memory state
    struct mach_task_basic_info info;
    mach_msg_type_number_t count = MACH_TASK_BASIC_INFO_COUNT;
    task_info(mach_task_self(), MACH_TASK_BASIC_INFO, (task_info_t)&info, &count);
    
    char buffer[512];
    snprintf(buffer, sizeof(buffer), 
        "OOM Report\n"
        "Resident Size: %llu\n"
        "Virtual Size: %llu\n",
        info.resident_size, info.virtual_size);
    
    write(fd, buffer, strlen(buffer));
    close(fd);
}

@end
```

**OOM Detection Challenges:**

1. **No Stack Trace:** OOM kills happen asynchronously; the killed process can't capture its own state
2. **Watchdog Reporting:** Must be done by a separate process (parent or watchdog)
3. **Memory Maps:** Can read `/proc` or use `mach_vm_read()` on other processes

---

## JavaScript Stack Capture

### Overview

JavaScript stack capture varies by environment:

- **V8 (Chrome/Node.js):** Rich APIs via `Error.prepareStackTrace`
- **SpiderMonkey (Firefox):** Standard Error.stack
- **JavaScriptCore (Safari):** Limited stack capture
- **Node.js:** Async stack traces, domain integration

### Error.stack Property Parsing

The standard way to capture stacks in JavaScript:

```javascript
// Basic stack capture
function captureStack() {
    const error = new Error();
    return error.stack;
}

// Example output (Node.js):
// Error
//     at captureStack (/path/to/file.js:3:19)
//     at main (/path/to/file.js:10:5)
//     at Object.<anonymous> (/path/to/file.js:15:1)
//     at Module._compile (node:internal/modules/cjs/loader:1234:30)
```

**Stack Format Variations:**

```
// V8 (Chrome/Node.js)
Error: Something went wrong
    at foo (/path/to/file.js:10:15)
    at bar (/path/to/file.js:20:5)
    at Object.<anonymous> (/path/to/file.js:30:1)

// Firefox (SpiderMonkey)
foo@/path/to/file.js:10:15
bar@/path/to/file.js:20:5
@/path/to/file.js:30:1

// Safari (JavaScriptCore)
foo@/path/to/file.js:10:15
bar@/path/to/file.js:20:5
global code@/path/to/file.js:30:1
```

**Stack Parser Implementation:**

```javascript
// Parse V8 stack format
function parseV8Stack(stack) {
    const lines = stack.split('\n');
    const frames = [];
    
    // First line is error name/message
    const errorLine = lines[0];
    
    // Remaining lines are stack frames
    for (let i = 1; i < lines.length; i++) {
        const line = lines[i].trim();
        if (!line) continue;
        
        // V8 format: "at functionName (location:line:col)"
        const match = line.match(/^at\s+(.+?)\s+\((.+):(\d+):(\d+)\)$/);
        if (match) {
            frames.push({
                functionName: match[1],
                fileName: match[2],
                lineNumber: parseInt(match[3], 10),
                columnNumber: parseInt(match[4], 10),
                raw: line
            });
        } else {
            // Anonymous or native frames
            const nativeMatch = line.match(/^at\s+(.+)$/);
            if (nativeMatch) {
                frames.push({
                    functionName: nativeMatch[1],
                    fileName: null,
                    lineNumber: null,
                    columnNumber: null,
                    raw: line,
                    isNative: true
                });
            }
        }
    }
    
    return { errorLine, frames };
}
```

### V8 Stack Trace API (Error.prepareStackTrace)

V8 provides a powerful API to customize stack trace formatting:

```javascript
// Customize stack trace formatting
Error.prepareStackTrace = function(error, structuredStackTrace) {
    // structuredStackTrace is an array of CallSite objects
    const lines = [error.name + ': ' + error.message];
    
    for (const frame of structuredStackTrace) {
        const funcName = frame.getFunctionName() || '<anonymous>';
        const script = frame.getFileName() || '<unknown>';
        const line = frame.getLineNumber() || 0;
        const col = frame.getColumnNumber() || 0;
        
        lines.push(`    at ${funcName} (${script}:${line}:${col})`);
    }
    
    return lines.join('\n');
};

// CallSite API methods
const callSiteMethods = {
    // Function information
    getFunction: () => Function | null,
    getFunctionName: () => string | null,
    getMethodName: () => string | null,
    
    // Location information
    getFileName: () => string | null,
    getLineNumber: () => number | null,
    getColumnNumber: () => number | null,
    
    // Evaluation information
    isEval: () => boolean,
    isNative: () => boolean,
    isConstructor: () => boolean,
    isToplevel: () => boolean,
    
    // Async information
    isAsync: () => boolean,
    isPromiseAll: () => boolean,
    getPromiseIndex: () => number,
    
    // Source code
    getEvalOrigin: () => string | CallSite,
    
    // Raw access
    toString: () => string
};
```

**Custom Stack Capture:**

```javascript
// Capture structured stack trace
function captureStructuredStack() {
    const stackLimit = Error.stackTraceLimit;
    Error.stackTraceLimit = 50;  // Capture more frames
    
    const error = new Error();
    const stack = error.stack;  // Uses prepareStackTrace if defined
    
    Error.stackTraceLimit = stackLimit;
    return stack;
}

// Access raw CallSite objects
function getRawCallSites() {
    const originalPrepare = Error.prepareStackTrace;
    let capturedSites = null;
    
    Error.prepareStackTrace = (_, sites) => {
        capturedSites = sites;
        return '';
    };
    
    const error = new Error();
    error.stack;  // Trigger prepareStackTrace
    
    Error.prepareStackTrace = originalPrepare;
    return capturedSites;
}
```

### Chrome DevTools Protocol Integration

CDP provides programmatic access to stack traces:

```javascript
// Using Chrome DevTools Protocol
const CDP = require('chrome-remote-interface');

async function captureStackWithCDP() {
    const client = await CDP();
    
    // Enable debugger
    await client.Debugger.enable();
    
    // Set breakpoint or pause
    client.Debugger.on('paused', async (params) => {
        const callFrames = params.callFrames;
        
        for (const frame of callFrames) {
            console.log(`Function: ${frame.functionName}`);
            console.log(`Location: ${frame.location.scriptId}:${frame.location.lineNumber}`);
            console.log(`Scope Chain:`, frame.scopeChain);
        }
        
        // Get scope variables
        const properties = await client.Runtime.getProperties({
            objectId: callFrames[0].this.objectId
        });
        
        // Continue execution
        await client.Debugger.resume();
    });
    
    // Set breakpoint
    await client.Debugger.setBreakpointByUrl({
        lineNumber: 10,
        url: 'file:///path/to/file.js'
    });
}
```

**CDP Stack Frame Format:**

```json
{
    "callFrames": [
        {
            "callFrameId": "1",
            "functionName": "myFunction",
            "location": {
                "scriptId": "42",
                "lineNumber": 10,
                "columnNumber": 5
            },
            "url": "file:///path/to/file.js",
            "scopeChain": [
                {
                    "type": "local",
                    "object": { "type": "object", "objectId": "-1" }
                },
                {
                    "type": "closure",
                    "object": { "type": "object", "objectId": "-2" }
                },
                {
                    "type": "global",
                    "object": { "type": "object", "objectId": "-3" }
                }
            ],
            "this": { "type": "object", "objectId": "-4" }
        }
    ]
}
```

### Node.js Async Stack Traces

Node.js captures async call chains:

```javascript
// Enable long stack traces
const async_hooks = require('async_hooks');

class AsyncStackTrace {
    constructor() {
        this.traces = new Map();
        this.currentId = null;
        
        this.hooks = async_hooks.createHook({
            init(asyncId, type, triggerAsyncId, resource) {
                // Capture stack at async resource creation
                if (type === 'PROMISE' || type === 'TCPWRAP') {
                    const stack = new Error().stack;
                    this.traces.set(asyncId, stack);
                }
            },
            before(asyncId) {
                this.currentId = asyncId;
            },
            after(asyncId) {
                this.currentId = null;
            },
            destroy(asyncId) {
                this.traces.delete(asyncId);
            }
        });
        
        this.hooks.enable();
    }
    
    getAsyncStack() {
        if (this.currentId && this.traces.has(this.currentId)) {
            return this.traces.get(this.currentId);
        }
        return null;
    }
}

// Usage with async/await
async function asyncFunction() {
    await someAsyncOperation();
}

asyncFunction().catch(err => {
    const tracer = new AsyncStackTrace();
    const asyncStack = tracer.getAsyncStack();
    
    console.log('Error:', err.message);
    console.log('Sync Stack:', err.stack);
    if (asyncStack) {
        console.log('Async Stack:', asyncStack);
    }
});
```

**Node.js v16+ Async Stack Traces:**

```javascript
// Node.js now has built-in async stack traces
// Enable with --async-stack-traces flag (on by default in v16+)

async function inner() {
    throw new Error('boom');
}

async function outer() {
    await inner();
}

outer().catch(err => console.log(err.stack));

// Output includes async frames:
// Error: boom
//     at inner (file.js:3:11)
//     at outer (file.js:7:11)
//     at <anonymous> (file.js:10:1)
```

### Browser vs Node Differences

| Feature | Browser (V8) | Node.js |
|---------|-------------|---------|
| `Error.prepareStackTrace` | Supported | Supported |
| `Error.stackTraceLimit` | Supported | Supported |
| Async stack traces | Limited | Full support |
| Source maps | Manual integration | Built-in |
| Native frames | Hidden | Visible |
| `async_hooks` API | N/A | Full support |

**Browser Limitations:**

```javascript
// Browser: Cannot access native frames directly
Error.prepareStackTrace = function(error, sites) {
    for (const site of sites) {
        // site.getFunction() may return null for built-ins
        // site.getFileName() may be empty for eval code
    }
    return Error.stackTraceLimit;
};

// Node.js: Can see native/internal frames
Error.prepareStackTrace = function(error, sites) {
    for (const site of sites) {
        // Can see: Module._compile, nativeModule.require, etc.
    }
};
```

### Source Map Integration

Source maps decode minified stack traces:

```javascript
// Using source-map library
const { SourceMapConsumer } = require('source-map');

async function decodeStack(minifiedStack, sourceMapUrl) {
    const response = await fetch(sourceMapUrl);
    const sourceMapData = await response.json();
    
    const consumer = await SourceMapConsumer.load(sourceMapData);
    
    const lines = minifiedStack.split('\n');
    const decodedLines = [lines[0]];  // Error message line
    
    for (let i = 1; i < lines.length; i++) {
        const frame = parseFrame(lines[i]);
        
        if (frame.lineNumber !== null) {
            const original = consumer.originalPositionFor({
                line: frame.lineNumber,
                column: frame.columnNumber
            });
            
            decodedLines.push(
                `    at ${original.name || frame.functionName} ` +
                `(${original.source}:${original.line}:${original.column})`
            );
        } else {
            decodedLines.push(lines[i]);
        }
    }
    
    consumer.destroy();
    return decodedLines.join('\n');
}

// In backtrace-javascript
class BacktraceClient {
    async send(error, options) {
        let stack = error.stack;
        
        // Check if source maps are enabled
        if (this.options.sourceMaps) {
            stack = await this.decodeWithSourceMaps(stack);
        }
        
        // Send to Backtrace
        await this.submitReport({ stack, ...options });
    }
}
```

---

## Android Stack Capture

### Overview

Android provides multiple stack capture mechanisms:

1. **Java Exceptions** - Standard JVM stack traces
2. **ART Runtime** - Dalvik/ART native stack capture
3. **Breakpad** - Native crash handling
4. **ANR Detection** - Watchdog thread dump capture
5. **Kernel Crash** - pstore/ramoops for kernel panics

### Java Exception Stack Traces

Standard Java stack trace capture:

```java
// Basic stack trace
try {
    throw new RuntimeException("Test exception");
} catch (Exception e) {
    // Full stack trace as string
    StringWriter sw = new StringWriter();
    PrintWriter pw = new PrintWriter(sw);
    e.printStackTrace(pw);
    String stackTrace = sw.toString();
    
    // Structured access
    StackTraceElement[] elements = e.getStackTrace();
    for (StackTraceElement element : elements) {
        Log.d("Stack", element.getClassName() + "." + 
              element.getMethodName() + ":" + 
              element.getLineNumber());
    }
}
```

**Stack Trace Format:**

```
java.lang.RuntimeException: Test exception
    at com.example.MyClass.myMethod(MyClass.java:42)
    at com.example.MyClass.otherMethod(MyClass.java:55)
    at com.example.MainActivity.onCreate(MainActivity.java:20)
    at android.app.Activity.performCreate(Activity.java:8054)
    at android.app.Instrumentation.callActivityOnCreate(Instrumentation.java:1338)
    ...
    Caused by: java.lang.NullPointerException: Null reference
    at com.example.MyClass.innerMethod(MyClass.java:100)
    ... 5 more
```

### ART Runtime Internals

ART (Android Runtime) provides native stack capture:

```cpp
// art/runtime/thread.h
class Thread {
public:
    // Get current thread's stack
    void GetStackFrames(std::vector<FrameInfo>* frames);
    
    // Get thread list for stack dumping
    static void DumpAllStacks(std::ostream& os);
    
    // Internal frame walking
    void WalkStack(FrameVisitor* visitor, bool check_suspension);
};

// art/runtime/stack.h
struct FrameInfo {
    uint32_t dex_pc;        // Dalvik PC
    uint32_t native_pc;     // Native PC (for JNI)
    const char* method_name;
    const char* source_file;
    uint32_t line_number;
};
```

**ART Stack Walking:**

```cpp
// art/runtime/stack.cc (simplified)
void Thread::WalkStack(FrameVisitor* visitor, bool check_suspension) {
    ArtMethod* method = GetCurrentMethod();
    uintptr_t fp = GetFramePointer();
    
    while (method != nullptr) {
        // Extract frame info
        FrameInfo info;
        info.method_name = method->GetName();
        info.source_file = method->GetSourceFile();
        info.dex_pc = GetDexPcFromFp(fp);
        info.line_number = method->GetLineNumFromDexPc(info.dex_pc);
        
        visitor->VisitFrame(info);
        
        // Move to next frame
        fp = GetNextFp(fp);
        method = GetMethodFromFp(fp);
    }
}

// Quick frame walking (compiled code)
void QuickWalkStack(FrameVisitor* visitor) {
    uintptr_t sp = GetStackPointer();
    uintptr_t fp = GetFramePointer();
    
    // Quick frames use a different layout
    while (IsValidFramePointer(fp)) {
        uint32_t pc = GetReturnPcFromFrame(fp);
        
        // Find method for PC using quick info
        ArtMethod* method = FindMethodFromQuickPc(pc);
        
        visitor->VisitFrame({
            .method_name = method->GetName(),
            .native_pc = pc
        });
        
        fp = GetNextQuickFrameFp(fp);
    }
}
```

**ART Stack Dump:**

```java
// Dump all threads (like "kill -3" on Linux)
public static void dumpThreads() {
    Map<Thread, StackTraceElement[]> allThreads = 
        Thread.getAllStackTraces();
    
    StringBuilder sb = new StringBuilder();
    for (Map.Entry<Thread, StackTraceElement[]> entry : 
         allThreads.entrySet()) {
        Thread thread = entry.getKey();
        sb.append("\"").append(thread.getName()).append("\"")
          .append(" ").append(thread.getState()).append("\n");
        
        for (StackTraceElement element : entry.getValue()) {
            sb.append("    at ")
              .append(element.toString())
              .append("\n");
        }
        sb.append("\n");
    }
    
    Log.d("ThreadDump", sb.toString());
}
```

### Native Crash Handling via Breakpad

Breakpad is Mozilla's crash reporting system used by Android:

```cpp
// breakpad/client/linux/handler/exception_handler.h
class ExceptionHandler {
public:
    ExceptionHandler(
        const string& dump_path,
        FilterCallback filter,
        MinidumpCallback callback,
        void* callback_context,
        int handler_types
    );
    
    // Register signal handlers
    void RegisterSignalHandler(int signum);
    
    // Write minidump
    bool WriteMinidump();
};

// Usage in Android NDK
#include "breakpad/client/linux/handler/exception_handler.h"

bool dumpCallback(const char* dump_path,
                  const char* minidump_id,
                  void* context,
                  bool succeeded) {
    // Called after minidump is written
    __android_log_print(ANDROID_LOG_INFO, "Breakpad", 
        "Minidump written: %s/%s.dmp", dump_path, minidump_id);
    return succeeded;
}

void initBreakpad() {
    const char* dump_path = "/data/data/com.example.app/cache";
    
    new ExceptionHandler(
        dump_path,
        nullptr,  // Filter callback
        dumpCallback,
        nullptr,  // Context
        ExceptionHandler::HANDLER_ALL  // Handle all signals
    );
}
```

**Breakpad Minidump Structure:**

```
┌─────────────────────────────────────────────────────────┐
│                    MINIDUMP HEADER                       │
│  Signature: MDMP                         Version: 42899  │
│  Streams: 8                               Stream RVA: 48 │
└────────────────────┬────────────────────────────────────┘
                     │
    ┌────────────────┼────────────────┐
    │                │                │
    ▼                ▼                ▼
┌──────────┐   ┌──────────┐   ┌──────────┐
│ Thread   │   │ Module   │   │ Exception│
│ List     │   │ List     │   │ Stream   │
│          │   │          │   │          │
│ Thread 1 │   │ app.so   │   │ Signo: 11│
│ - Context│   │ - Base   │   │ - Addr   │
│ - Stack  │   │ - Size   │   │ - Code   │
└──────────┘   └──────────┘   └──────────┘
    │                │                │
    ▼                ▼                ▼
┌──────────┐   ┌──────────┐   ┌──────────┐
│ Memory   │   │ System   │   │ Misc     │
│ List     │   │ Info     │   │ Streams  │
│          │   │          │   │          │
│ Stack 1  │   │ OS: Linux│   │ Android  │
│ Stack 2  │   │ CPU: ARM │   │ Build    │
│ ...      │   │ Version  │   │ Fingerprint│
└──────────┘   └──────────┘   └──────────┘
```

**Breakpad Signal Handling:**

```cpp
// breakpad/client/linux/handler/exception_handler.cc
void ExceptionHandler::SignalHandler(int sig, siginfo_t* info, void* ctx) {
    // Get the context from the signal
    ucontext_t* context = (ucontext_t*)ctx;
    
    // Write the minidump
    WriteMinidumpForSignal(sig, info, context);
    
    // Call the original handler or terminate
    if (original_handler_.sa_sigaction != nullptr) {
        original_handler_.sa_sigaction(sig, info, ctx);
    } else {
        _exit(128 + sig);
    }
}

void ExceptionHandler::WriteMinidumpForSignal(
    int sig, siginfo_t* info, ucontext_t* context) {
    
    MinidumpWriter writer(dump_path_, minidump_id_);
    
    // Write thread list
    writer.WriteThreadList();
    
    // Write exception stream
    writer.WriteExceptionStream(sig, info);
    
    // Write memory around crash
    writer.WriteCrashingThreadMemory(context);
    
    // Write process memory maps
    writer.WriteMemoryMaps();
}
```

### ANR (Application Not Responding) Detection

ANR occurs when the main thread is blocked:

```java
// Android framework ANR detection
// frameworks/base/services/core/java/com/android/server/am/ActivityManagerService.java

public class ActivityManagerService {
    // ANR timeout constants
    private static final int KEY_DISPATCHING_TIMEOUT = 5000;  // 5 seconds
    private static final int BROADCAST_TIMEOUT = 10000;       // 10 seconds
    private static final int SERVICE_TIMEOUT = 20000;         // 20 seconds
    
    // Watchdog for ANR detection
    final Handler mHandler = new Handler();
    
    final void startWatchdog() {
        mHandler.postDelayed(mWatchdogRunnable, KEY_DISPATCHING_TIMEOUT);
    }
    
    final Runnable mWatchdogRunnable = new Runnable() {
        public void run() {
            if (mBinderFirstCallTime != 0) {
                // Check if binder call exceeded timeout
                long now = SystemClock.uptimeMillis();
                if (now - mBinderFirstCallTime > KEY_DISPATCHING_TIMEOUT) {
                    // ANR detected!
                    handleAnr();
                }
            }
        }
    };
    
    void handleAnr() {
        // Get stack traces of all threads
        String traces = ActivityThread.currentActivityThread()
            .getThreadDump();
        
        // Log the ANR with traces
        EventLog.writeEvent(EventLogTags.AMR_ANR, 
            Process.myPid(), traces);
        
        // Show ANR dialog to user
        showAppNotRespondingDialog();
    }
}
```

**ANR Trace Collection:**

```java
// frameworks/base/core/java/android/app/ActivityThread.java

public String getThreadDump() {
    StringBuilder sb = new StringBuilder();
    
    // Get all thread stack traces
    Map<Thread, StackTraceElement[]> threads = 
        Thread.getAllStackTraces();
    
    // Format similar to "kill -3"
    for (Map.Entry<Thread, StackTraceElement[]> entry : 
         threads.entrySet()) {
        Thread t = entry.getKey();
        sb.append("\"").append(t.getName()).append("\" prio=")
          .append(t.getPriority()).append(" ")
          .append(t.getState()).append("\n");
        
        // Add lock info if blocked
        if (t.getState() == Thread.State.BLOCKED) {
            sb.append("  - locked <").append(t.getName())
              .append(">\n");
        }
        
        // Add stack frames
        for (StackTraceElement frame : entry.getValue()) {
            sb.append("  at ").append(frame.toString())
              .append("\n");
        }
        sb.append("\n");
    }
    
    return sb.toString();
}
```

**ANR Output Example:**

```
"main" prio=5 tid=1 Blocked
  - locked <0x12345678> (a java.lang.Object)
  at com.example.MyClass.synchronizedMethod(MyClass.java:42)
  - waiting to lock <0x87654321> (a java.util.HashMap)
  at com.example.MyClass.callingMethod(MyClass.java:50)
  at com.example.MainActivity.onCreate(MainActivity.java:20)
  at android.app.Activity.performCreate(Activity.java:8054)

"Binder:1" prio=5 tid=5 Native
  at dalvik.system.NativeStart.run(Native Method)
```

### Kernel-Level Crash Reporting

Android kernel crashes use pstore/ramoops:

```c
// Kernel crash handling (simplified)
// kernel/panic.c

void panic(const char *fmt, ...) {
    // Disable local interrupts
    local_irq_disable();
    
    // Print panic message
    va_list args;
    va_start(args, fmt);
    vprintk(fmt, args);
    va_end(args);
    
    // Write to pstore (persistent storage)
    pstore_panic_write(&info, buf, len);
    
    // Attempt to dump memory (kdump)
    crash_kexec();
    
    // Infinite loop or reboot
    while (1);
}

// Android-specific: ramoops for persistent crash storage
// drivers/pstore/ramoops.c

static int ramoops_panic(struct notifier_block *this,
                         unsigned long event, void *ptr) {
    struct ramoops_context *c = &ramoops_context;
    
    // Write panic data to reserved memory
    ramoops_write_to_persistent_storage(c, ptr);
    
    return NOTIFY_DONE;
}

// Reading ramoops from userspace
// $ cat /sys/fs/pstore/console-ramoops-0
```

**Kernel Crash Log Format:**

```
[  123.456789] Unable to handle kernel NULL pointer dereference at virtual address 00000000
[  123.456790] Internal error: Oops: 96000005 [#1] PREEMPT SMP
[  123.456791] Modules linked in: driver1 driver2 driver3
[  123.456792] CPU: 0 PID: 1234 Comm: crashed_process
[  123.456793] Hardware name: Qualcomm Technologies, Inc. MSM8996
[  123.456794] pc : driver_function+0x42/0x100 [driver1]
[  123.456795] lr : another_function+0x10/0x20
[  123.456796] sp : ffffff8001234000
[  123.456797] x29: ffffff8001234000 x28: 0000000000000000
[  123.456798] Call trace:
[  123.456799]  driver_function+0x42/0x100 [driver1]
[  123.456800]  another_function+0x10/0x20
[  123.456801]  yet_another+0x5/0x10
[  123.456802] Kernel panic - not syncing: Fatal exception
```

---

## Native Crash Handling (Crashpad)

### Overview

Crashpad is Google's crash reporting library used by Chrome and many projects:

1. **Exception Handlers** - Platform-specific crash capture
2. **Minidump Format** - Compact crash report format
3. **Register State** - Full CPU state at crash
4. **Memory Snapshots** - Configurable memory capture
5. **Annotations** - Custom key-value crash metadata

### Exception Filters

Crashpad uses exception filters to determine which crashes to report:

```cpp
// crashpad/handler/crashpad_client.h
class CrashpadClient {
public:
    bool StartHandler(
        const base::FilePath& handler,
        const base::FilePath& database,
        const base::FilePath& metrics_dir,
        const std::string& url,
        const std::map<std::string, std::string>& annotations,
        const std::vector<std::string>& arguments,
        bool restartable,
        bool asynchronous_start
    );
    
    // Set exception filter
    void SetFirstChanceExceptionHandler(
        FirstChanceHandler handler
    );
};

// Custom exception filter
bool MyFirstChanceHandler(
    EXCEPTION_POINTERS* exception_pointers,
    bool is_first_chance
) {
    DWORD code = exception_pointers->ExceptionRecord->ExceptionCode;
    
    // Filter specific exceptions
    switch (code) {
        case EXCEPTION_ACCESS_VIOLATION:
        case EXCEPTION_STACK_OVERFLOW:
        case EXCEPTION_ILLEGAL_INSTRUCTION:
        case EXCEPTION_INT_DIVIDE_BY_ZERO:
            // Report these
            return true;
            
        case EXCEPTION_BREAKPOINT:
        case EXCEPTION_SINGLE_STEP:
            // Don't report debug breakpoints
            return false;
            
        default:
            // Let system handle
            return false;
    }
}
```

**Exception Filter Chain:**

```cpp
// crashpad/handler/minidump_file_writer.cc
class ExceptionFilterChain {
public:
    void AddFilter(std::unique_ptr<ExceptionFilter> filter);
    
    ExceptionFilterResult HandleException(
        const ExceptionContext& context
    ) {
        for (auto& filter : filters_) {
            ExceptionFilterResult result = filter->Handle(context);
            
            switch (result) {
                case ExceptionFilterResult::kContinueSearching:
                    continue;
                case ExceptionFilterResult::kHandleCrash:
                    return result;
                case ExceptionFilterResult::kSuppressCrash:
                    return result;
            }
        }
        
        return ExceptionFilterResult::kContinueSearching;
    }

private:
    std::vector<std::unique_ptr<ExceptionFilter>> filters_;
};
```

### Minidump Format

Minidump is a compact binary format for crash reports:

```cpp
// crashpad/minidump/minidump_file_writer.h
struct MINIDUMP_HEADER {
    uint32_t Signature;        // "MDMP" = 0x50444D4D
    uint16_t Version;          // Format version
    uint16_t NumberOfStreams;  // Number of data streams
    uint32_t StreamDirectoryRva; // Offset to stream directory
    uint32_t CheckSum;         // CRC32 checksum
    uint32_t TimeDateStamp;    // Unix timestamp
    uint64_t Flags;            // MINIDUMP_TYPE flags
};

// Stream directory entry
struct MINIDUMP_DIRECTORY {
    uint32_t StreamType;
    MINIDUMP_LOCATION_DESCRIPTOR Location;
};

// Stream types
enum MinidumpStreamType {
    UnusedStream = 0,
    ReservedStream = 1,
    ThreadListStream = 3,
    ModuleListStream = 4,
    MemoryListStream = 5,
    ExceptionStream = 6,
    SystemInfoStream = 7,
    ThreadExListStream = 8,
    Memory2ListStream = 9,
    MemoryInfoListStream = 14,
    // ... more types
};
```

**Minidump Structure:**

```
┌─────────────────────────────────────────────────────────────────┐
│                     MINIDUMP HEADER                              │
│  Signature: 0x50444D4D ("MDMP")                                  │
│  Version: 42899                                                  │
│  Streams: 8                                                        │
│  Directory RVA: 48                                               │
└─────────────────────┬───────────────────────────────────────────┘
                      │
         ┌────────────┼────────────┐
         │            │            │
         ▼            ▼            ▼
    ┌─────────┐  ┌─────────┐  ┌─────────┐
    │ Stream  │  │ Stream  │  │ Stream  │
    │ Type 3  │  │ Type 4  │  │ Type 6  │
    │ Thread  │  │ Module  │  │ Except  │
    │ List    │  │ List    │  │         │
    └────┬────┘  └────┬────┘  └────┬────┘
         │            │            │
         ▼            ▼            ▼
    ┌─────────┐  ┌─────────┐  ┌─────────┐
    │ Thread1 │  │ app.exe │  │ Code:   │
    │ - ID    │  │ - Base  │  │ 0xC0000 │
    │ - Susp  │  │ - Size  │  │ - Access│
    │ - Stack │  │ - Time  │  │ - Addr  │
    │ - Context│ │ - Check │  │ - Flags │
    └─────────┘  └─────────┘  └─────────┘
```

### Register State Capture

Full CPU context at crash time:

```cpp
// crashpad/util/misc/capture_context.h

// x86_64 context
struct CaptureContextX86_64 {
    uint64_t rax;
    uint64_t rcx;
    uint64_t rdx;
    uint64_t rbx;
    uint64_t rsp;  // Stack pointer
    uint64_t rbp;  // Base/frame pointer
    uint64_t rsi;
    uint64_t rdi;
    uint64_t r8;
    uint64_t r9;
    uint64_t r10;
    uint64_t r11;
    uint64_t r12;
    uint64_t r13;
    uint64_t r14;
    uint64_t r15;
    uint64_t rip;  // Instruction pointer (CRASH LOCATION)
    uint64_t rflags;
    uint16_t cs;
    uint16_t ds;
    uint16_t es;
    uint16_t fs;
    uint16_t gs;
    uint16_t ss;
    FLOATING_SAVE_AREA float_save;
    uint128_t xmm[16];  // SIMD registers
};

// ARM64 context
struct CaptureContextARM64 {
    uint64_t x[29];   // General purpose registers x0-x28
    uint64_t fp;      // Frame pointer (x29)
    uint64_t lr;      // Link register (x30) - return address
    uint64_t sp;      // Stack pointer
    uint64_t pc;      // Program counter (CRASH LOCATION)
    uint32_t cpsr;    // Current program status register
    uint32_t padding;
    uint128_t v[32];  // SIMD/FP registers
    uint32_t fpcr;    // FP control register
    uint32_t fpsr;    // FP status register
};

// Capture current thread context
void CaptureContext(CaptureContextX86_64* context) {
    #if defined(_WIN32)
        CONTEXT ctx;
        RtlCaptureContext(&ctx);
        // Convert to our format
        context->rax = ctx.Rax;
        context->rbx = ctx.Rbx;
        // ... etc
    #elif defined(__APPLE__)
        // Use __builtin_frame_address and inline assembly
        context->rip = __builtin_return_address(0);
        context->rbp = __builtin_frame_address(0);
        asm volatile("movq %%rsp, %0" : "=r"(context->rsp));
    #elif defined(__linux__)
        // Use ucontext from signal handler
    #endif
}
```

**Context from Signal Handler:**

```cpp
// crashpad/util/posix/capture_context_linux.cc
void CaptureContextFromSignalHandler(
    int signum,
    siginfo_t* siginfo,
    void* void_context,
    CaptureContextX86_64* out_context
) {
    ucontext_t* context = reinterpret_cast<ucontext_t*>(void_context);
    
    out_context->rax = context->uc_mcontext.gregs[REG_RAX];
    out_context->rbx = context->uc_mcontext.gregs[REG_RBX];
    out_context->rcx = context->uc_mcontext.gregs[REG_RCX];
    out_context->rdx = context->uc_mcontext.gregs[REG_RDX];
    out_context->rsi = context->uc_mcontext.gregs[REG_RSI];
    out_context->rdi = context->uc_mcontext.gregs[REG_RDI];
    out_context->rbp = context->uc_mcontext.gregs[REG_RBP];
    out_context->rsp = context->uc_mcontext.gregs[REG_RSP];
    out_context->rip = context->uc_mcontext.gregs[REG_RIP];
    out_context->rflags = context->uc_mcontext.gregs[REG_EFL];
    
    // SIMD registers
    for (int i = 0; i < 16; i++) {
        out_context->xmm[i] = context->uc_mcontext.fpregs->_xmm[i];
    }
}
```

### Memory Snapshots

Configurable memory capture around crash:

```cpp
// crashpad/handler/crashpad_client.h
class CrashpadClient {
public:
    // Set memory capture mode
    void SetMemoryCaptureMode(
        MemoryCaptureMode mode
    );
};

enum class MemoryCaptureMode {
    // Capture everything (large dumps)
    kFullMemory,
    
    // Capture only stack memory for each thread
    kStackOnly,
    
    // Capture heap memory plus stacks
    kHeapAndStack,
    
    // Capture specific regions
    kSelective
};

// Specify regions to capture
void AddMemoryRange(const void* address, size_t size) {
    memory_ranges_.push_back({
        .start = reinterpret_cast<uintptr_t>(address),
        .end = reinterpret_cast<uintptr_t>(address) + size
    });
}

// Capture nearby memory for pointers
void CapturePointeeMemory(uintptr_t pointer, size_t context_size) {
    if (IsValidPointer(pointer)) {
        AddMemoryRange(
            pointer - context_size / 2,
            context_size
        );
    }
}
```

**Memory Snapshot Structure:**

```cpp
// crashpad/minidump/minidump_memory_writer.h
class MinidumpMemoryWriter {
public:
    void AddMemoryRegion(
        uint64_t base_address,
        std::unique_ptr<MemorySnapshot> memory
    );
    
    // Write memory list stream
    bool WriteMinidumpMemoryList() {
        MINIDUMP_MEMORY_LIST list;
        list.NumberOfMemoryRanges = regions_.size();
        list.MemoryRanges = new MINIDUMP_MEMORY_DESCRIPTOR[regions_.size()];
        
        for (size_t i = 0; i < regions_.size(); i++) {
            list.MemoryRanges[i].StartOfMemoryRange = regions_[i].base;
            list.MemoryRanges[i].Memory = regions_[i].location;
        }
        
        return WriteStream(MemoryListStream, &list);
    }
};
```

### Custom Annotations

Key-value pairs for crash enrichment:

```cpp
// crashpad/handler/crashpad_client.h
class CrashpadClient {
public:
    // Set a crash annotation
    void SetAnnotation(
        const std::string& key,
        const std::string& value
    );
    
    // Set annotations from a map
    void SetAnnotations(
        const std::map<std::string, std::string>& annotations
    );
};

// Common annotations
void AddStandardAnnotations() {
    SetAnnotation("version", APP_VERSION);
    SetAnnotation("channel", BUILD_CHANNEL);  // stable, beta, dev
    SetAnnotation("process_type", "renderer");
    SetAnnotation("os_version", GetOSVersion());
    SetAnnotation("cpu_arch", GetCPUArch());
    SetAnnotation("command_line", GetCommandLine());
}

// Custom crash data
void AddCrashContext(const std::string& context) {
    // Large annotations stored as separate stream
    SetAnnotation("crash_context", context);
}

// Attach files to crash report
void AddAttachment(const base::FilePath& path) {
    attachments_.push_back(path);
}
```

**Annotation Storage:**

```cpp
// Annotations are stored in a simple string stream
// crashpad/minidump/minidump_simple_string_dictionary_writer.cc

class SimpleStringDictionaryWriter {
public:
    void AddEntry(const std::string& key, const std::string& value) {
        Entry entry;
        entry.key = key;
        entry.value = value;
        entries_.push_back(entry);
    }
    
    bool Write() {
        // Format: count + (key_len + key + value_len + value) * count
        uint32_t count = entries_.size();
        Write(&count, sizeof(count));
        
        for (const auto& entry : entries_) {
            uint8_t key_len = std::min<uint8_t>(entry.key.size(), 255);
            Write(&key_len, sizeof(key_len));
            Write(entry.key.c_str(), key_len);
            
            uint16_t value_len = std::min<uint16_t>(entry.value.size(), 65535);
            Write(&value_len, sizeof(value_len));
            Write(entry.value.c_str(), value_len);
        }
        
        return true;
    }
};
```

---

## Platform Comparison

### Stack Quality Comparison

| Platform | Frame Pointers | DWARF CFI | Source Maps | Symbolication |
|----------|---------------|-----------|-------------|---------------|
| Go | Yes (always) | No | N/A | Runtime |
| Cocoa | Optional | Yes | dSYM | External |
| JavaScript | N/A | N/A | Yes | External |
| Android Java | Yes | Yes | N/A | Runtime |
| Android Native | Optional | Yes | N/A | External |
| Crashpad | Optional | Yes | N/A | External |

### Capture Mechanisms Summary

| Platform | Primary Mechanism | Fallback | Async Safe |
|----------|------------------|----------|------------|
| Go | runtime.Callers() | runtime.Stack() | Signal handler |
| Cocoa | PLCrashReporter | NSSymbolicator | Mach exception |
| JavaScript | Error.stack | Error.prepareStackTrace | N/A |
| Android | Thread.getAllStackTraces() | /proc/tid/stack | Signal handler |
| Native | libunwind | backtrace() | Signal handler |

### Performance Overhead

| Platform | Stack Capture | Full Report | Memory |
|----------|--------------|-------------|--------|
| Go (Callers) | ~1-5 μs/frame | ~100-500 μs | ~1 KB/frame |
| Cocoa (PLCrash) | ~10-50 μs | ~1-5 ms | ~10 KB/thread |
| JavaScript | ~5-20 μs | ~10-100 μs | ~500 B/frame |
| Android Java | ~10-30 μs | ~50-200 μs | ~1 KB/frame |
| Crashpad | ~50-200 μs | ~1-10 ms | ~100 KB/dump |

---

## Performance Considerations

### Go Performance

```go
// Benchmark stack capture
func BenchmarkCallers(b *testing.B) {
    pcs := make([]uintptr, 100)
    for i := 0; i < b.N; i++ {
        runtime.Callers(1, pcs)
    }
}

// Results:
// Callers-8    1000000    1.2 μs/op    100 frames
// ~12 ns/frame
```

**Optimization Tips:**

1. **Skip Unnecessary Symbolication:** Use `runtime.Callers()` and defer `CallersFrames()` until needed
2. **Pool PC Buffers:** Reuse `[]uintptr` buffers to avoid allocation
3. **Limit Depth:** Use `runtime.Callers(skip, pcs[:maxDepth])`
4. **Avoid AllGoroutines:** `CaptureAllGoroutines` causes STW

### Cocoa Performance

```objc
// PLCrashReporter is optimized for minimal overhead
// - Uses async-safe functions only
// - Defers symbolication to post-crash
// - Memory-mapped file I/O for report writing

// Optimization: Disable unused features
configuration.crashReporterEnabled = NO;  // If using PLCrashReporter directly
configuration.oomReportingEnabled = NO;   // If OOM not needed
```

### JavaScript Performance

```javascript
// Minimize Error.stack overhead
class LightweightError {
    constructor(message) {
        this.message = message;
        this.timestamp = Date.now();
        // Don't capture stack unless needed
    }
    
    get stack() {
        // Lazy stack capture
        if (!this._stack) {
            this._stack = new Error(this.message).stack;
        }
        return this._stack;
    }
}

// Production: capture stacks asynchronously
async function reportError(error) {
    // Don't block on error reporting
    queueMicrotask(() => {
        sendToBacktrace(error);
    });
}
```

### Native Performance

```cpp
// Minidump optimization: selective memory capture
CrashpadClient client;

// Only capture essential memory
client.SetMemoryCaptureMode(MemoryCaptureMode::kStackOnly);

// Add only critical annotations
client.SetAnnotation("version", "1.0.0");

// Avoid large attachments in crash handler
// Write to file, attach path instead
```

---

## Edge Cases and Gotchas

### Go Gotchas

1. **Goroutine Stack Capture:** Other goroutines must be stopped
2. **CGO Frames:** C stack frames invisible to Go runtime
3. **Inlining:** Aggressive inlining can obscure stack traces
4. **Tail Calls:** Go 1.21+ tail call optimization affects stack depth

### Cocoa Gotchas

1. **Bitcode:** dSYM must match exact build
2. **Swift Mangled Names:** Require demangling for readability
3. **OS Symbol Server:** Requires macOS for symbolication
4. **Simulator Crashes:** Different architecture (x86_64 vs arm64)

### JavaScript Gotchas

1. **Minification:** Requires source maps for useful stacks
2. **Eval/New Function:** Obscures frame information
3. **Cross-Origin:** Stack may be hidden for security
4. **WebAssembly:** Separate stack from JS stack

### Android Gotchas

1. **ProGuard/R8:** Requires mapping file for deobfuscation
2. **Multi-Process:** Each process has separate crash context
3. **Native Library Crashes:** May not include Java frames
4. **Samsung/Custom ROMs:** Vendor-specific crash handling

### Crashpad Gotchas

1. **Handler Process:** Must be separate from crashed process
2. **File Descriptors:** Limited FDs in crash handler
3. **Signal Mask:** Original signal handler may be lost
4. **Thread Suspension:** May deadlock if thread holds locks

---

## Sources

- Go Source: https://github.com/golang/go/tree/master/src/runtime
- PLCrashReporter: https://github.com/microsoft/plcrashreporter
- Breakpad: https://chromium.googlesource.com/breakpad/breakpad
- Crashpad: https://chromium.googlesource.com/crashpad/crashpad
- ART Runtime: https://android.googlesource.com/platform/art
- V8 Source: https://chromium.googlesource.com/v8/v8
- backtrace-cocoa: https://github.com/backtrace-labs/backtrace-cocoa
- backtrace-go: https://github.com/backtrace-labs/backtrace-go
- backtrace-android: https://github.com/backtrace-labs/backtrace-android

---

*This document is part of the backtrace exploration series. See also:*
- `00-zero-to-backtrace-developer.md` - Getting started guide
- `02-crash-reporting-architecture.md` - Server-side processing
- `03-symbolication-deep-dive.md` - Symbol resolution across platforms
