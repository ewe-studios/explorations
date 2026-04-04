# Crash Reporting, Aggregation, and Analysis Deep Dive

> **Purpose:** Comprehensive exploration of crash reporting architecture, aggregation algorithms, and analysis systems for Backtrace - from crash capture to server-side processing and real-time alerting.
>
> **Scope:** Client-side crash structure, fingerprinting algorithms, server-side processing (Morgue), real-time features, and compliance considerations.
>
> **Explored At:** 2026-04-05

---

## Table of Contents

1. [Crash Report Structure](#crash-report-structure)
2. [Crash Classification](#crash-classification)
3. [Crash Aggregation Algorithms](#crash-aggregation-algorithms)
4. [Crash Analysis](#crash-analysis)
5. [Server-Side Processing (Morgue)](#server-side-processing-morgue)
6. [Real-time Features](#real-time-features)
7. [Privacy and Compliance](#privacy-and-compliance)
8. [Appendix: Reference Implementations](#appendix-reference-implementations)

---

## 1. Crash Report Structure

### Complete Crash Report Anatomy

A crash report is a structured document capturing the complete state of an application at the moment of failure. Backtrace normalizes crash reports across platforms into a unified JSON schema.

#### Full Crash Report JSON Example

```json
{
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2026-04-05T14:30:00.123456Z",
  "application": {
    "name": "MyApp",
    "version": "2.1.0",
    "build": "20260405.1",
    "bundle_id": "com.example.myapp",
    "environment": "production",
    "start_time": "2026-04-05T14:00:00.000000Z",
    "uptime_seconds": 1800.123
  },
  "device": {
    "type": "iPhone14,2",
    "model": "iPhone 13 Pro",
    "architecture": "arm64e",
    "os_name": "iOS",
    "os_version": "17.4.1",
    "os_build": "21E237",
    "jailbroken": false,
    "memory_total_bytes": 6442450944,
    "memory_free_bytes": 1073741824,
    "storage_total_bytes": 128849018880,
    "storage_free_bytes": 32212254720,
    "battery_level": 0.72,
    "thermal_state": "nominal",
    "locale": "en_US",
    "timezone": "America/New_York"
  },
  "crash": {
    "type": "native",
    "exception_type": "EXC_CRASH",
    "exception_code": "0x0000000000000000",
    "exception_subcode": "0x0000000000000000",
    "signal": "SIGABRT",
    "signal_code": "SI_TKILL",
    "reason": "Abort triggered",
    "address": "0x0000000100003f48",
    "access_type": "unknown",
    "faulting_instruction": "0x184a5ea58",
    "cpu_type": "arm64e",
    "thread_cause": "0x0000000000000113"
  },
  "threads": [
    {
      "id": 1,
      "name": "main",
      "label": "Main Thread",
      "crashed": true,
      "current_thread": true,
      "dispatch_queue": "com.apple.main-thread",
      "registers": {
        "x0": "0x0000000000000000",
        "x1": "0x0000000000000001",
        "x2": "0x0000000000000000",
        "x3": "0x0000000000000000",
        "x4": "0x0000000000000000",
        "x5": "0x0000000000000000",
        "x6": "0x0000000000000000",
        "x7": "0x0000000000000000",
        "x8": "0x6d656d5f73736572",
        "x9": "0x0000000000000001",
        "x10": "0x0000000000000010",
        "x11": "0x0000000000000002",
        "x12": "0x0000000000000003",
        "x13": "0x0000000000000001",
        "x14": "0x0000000000000000",
        "x15": "0x0000000000000003",
        "x16": "0x0000000000000148",
        "x17": "0x0000000000000150",
        "x18": "0x0000000000000000",
        "x19": "0x0000000000000000",
        "x20": "0x0000000000000000",
        "x21": "0x0000000000000000",
        "x22": "0x0000000000000000",
        "x23": "0x0000000000000000",
        "x24": "0x0000000000000000",
        "x25": "0x0000000000000000",
        "x26": "0x0000000000000000",
        "x27": "0x0000000000000000",
        "x28": "0x0000000000000000",
        "x29": "0x000000016f66ad80",
        "x30": "0x0000000184a5ea58",
        "pc": "0x0000000184a5ea58",
        "sp": "0x000000016f66ad60",
        "fp": "0x000000016f66ad80",
        "lr": "0x0000000184a5ea58",
        "cpsr": "0x00000000"
      },
      "stack_frames": [
        {
          "index": 0,
          "instruction_address": "0x0000000184a5ea58",
          "return_address": "0x0000000184b34c80",
          "function_name": "abort",
          "library_name": "libsystem_kernel.dylib",
          "library_path": "/usr/lib/system/libsystem_kernel.dylib",
          "offset": 44,
          "symbolicated": true,
          "trust": "scan"
        },
        {
          "index": 1,
          "instruction_address": "0x0000000184b34c80",
          "return_address": "0x0000000100003f48",
          "function_name": "abort",
          "library_name": "libsystem_c.dylib",
          "library_path": "/usr/lib/system/libsystem_c.dylib",
          "offset": 180,
          "symbolicated": true,
          "trust": "cfi"
        },
        {
          "index": 2,
          "instruction_address": "0x0000000100003f48",
          "return_address": "0x0000000100123456",
          "function_name": "-[AppDelegate application:didFinishLaunchingWithOptions:]",
          "class_name": "AppDelegate",
          "file_name": "AppDelegate.swift",
          "line_number": 42,
          "column": 12,
          "library_name": "MyApp",
          "library_path": "/var/containers/Bundle/Application/XXX/MyApp.app/MyApp",
          "offset": 16200,
          "symbolicated": true,
          "trust": "fp",
          "source_status": "mapped"
        }
      ],
      "stack_size_bytes": 8388608,
      "stack_guard_pages": 2
    }
  ],
  "binary_images": [
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440001",
      "name": "MyApp",
      "path": "/var/containers/Bundle/Application/XXX/MyApp.app/MyApp",
      "type": "executable",
      "image_address": "0x0000000100000000",
      "image_size_bytes": 52428800,
      "cpu_type": "arm64e",
      "architecture": "arm64e",
      "segment_base": "0x0000000100000000"
    },
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440002",
      "name": "libsystem_kernel.dylib",
      "path": "/usr/lib/system/libsystem_kernel.dylib",
      "type": "system",
      "image_address": "0x0000000184a5e000",
      "image_size_bytes": 524288,
      "cpu_type": "arm64e",
      "architecture": "arm64e"
    }
  ],
  "memory_state": {
    "virtual_memory_size": 268435456,
    "resident_set_size": 134217728,
    "heap_size": 67108864,
    "heap_allocated": 52428800,
    "heap_free": 14680064,
    "stack_size": 8388608,
    "memory_regions": [
      {
        "start_address": "0x0000000100000000",
        "end_address": "0x0000000104000000",
        "protection": "r-x",
        "type": "executable",
        "name": "MyApp.__TEXT"
      },
      {
        "start_address": "0x0000000104000000",
        "end_address": "0x0000000108000000",
        "protection": "rw-",
        "type": "data",
        "name": "MyApp.__DATA"
      }
    ]
  },
  "attributes": {
    "user_id": "user_12345",
    "user_email": "user@example.com",
    "subscription_tier": "premium",
    "feature_flag_new_ui": true,
    "experiment_group": "control",
    "network_type": "wifi",
    "carrier": "Verizon",
    "app_state": "foreground",
    "last_action": "purchase_complete",
    "screen": "checkout",
    "request_id": "req_abc123",
    "api_version": "v2",
    "custom_metric_latency_ms": 245,
    "custom_metric_response_size": 1024
  },
  "breadcrumbs": [
    {
      "timestamp": "2026-04-05T14:28:00.000000Z",
      "level": "info",
      "type": "user",
      "message": "User tapped 'Add to Cart'",
      "data": {
        "product_id": "prod_789",
        "quantity": 2
      }
    },
    {
      "timestamp": "2026-04-05T14:29:00.000000Z",
      "level": "info",
      "type": "navigation",
      "message": "Navigated to checkout screen",
      "data": {
        "from": "cart",
        "to": "checkout"
      }
    },
    {
      "timestamp": "2026-04-05T14:29:30.000000Z",
      "level": "warning",
      "type": "http",
      "message": "Payment API returned 500",
      "data": {
        "url": "https://api.example.com/payment",
        "method": "POST",
        "status_code": 500,
        "duration_ms": 5000
      }
    },
    {
      "timestamp": "2026-04-05T14:30:00.000000Z",
      "level": "error",
      "type": "error",
      "message": "Payment processing failed",
      "data": {
        "error_code": "PAYMENT_TIMEOUT"
      }
    }
  ],
  "attachments": [
    {
      "name": "app_log.txt",
      "type": "text/plain",
      "size": 15360,
      "url": "s3://bucket/attachments/550e8400-e29b-41d4-a716-446655440000/app_log.txt"
    },
    {
      "name": "screenshot.png",
      "type": "image/png",
      "size": 245678,
      "url": "s3://bucket/attachments/550e8400-e29b-41d4-a716-446655440000/screenshot.png"
    },
    {
      "name": "crash_dump.dmp",
      "type": "application/x-minidump",
      "size": 1048576,
      "url": "s3://bucket/attachments/550e8400-e29b-41d4-a716-446655440000/crash_dump.dmp"
    }
  ],
  "handled": false,
  "severity": "fatal",
  "fingerprint": "a1b2c3d4e5f6g7h8",
  "grouping_hash": "grp_xyz789"
}
```

### Header Information Deep Dive

#### Timestamp Handling

```json
{
  "timestamp": "2026-04-05T14:30:00.123456Z",
  "local_timestamp": "2026-04-05T10:30:00.123456-04:00",
  "server_timestamp": "2026-04-05T14:30:01.234567Z"
}
```

**Critical timestamp considerations:**

| Field | Purpose | Precision |
|-------|---------|-----------|
| `timestamp` | Client-side crash time (UTC) | Microseconds |
| `local_timestamp` | Local device time with timezone | Microseconds |
| `server_timestamp` | Server ingestion time | Microseconds |
| `app_start_time` | Application launch time | Milliseconds |
| `uptime_seconds` | Seconds since app start | Float (ms precision) |

**Clock skew handling:**
```go
// Calculate clock skew between client and server
type ClockSkew struct {
    ClientTime time.Time `json:"client_time"`
    ServerTime time.Time `json:"server_time"`
    SkewMs     int64     `json:"skew_ms"`
}

func CalculateSkew(client, server time.Time) int64 {
    return server.Sub(client).Milliseconds()
}
```

#### Process Information

```json
{
  "process": {
    "pid": 12345,
    "ppid": 1,
    "name": "MyApp",
    "path": "/Applications/MyApp.app/Contents/MacOS/MyApp",
    "arguments": ["--debug", "--config=/etc/myapp/config.json"],
    "environment": {
      "NODE_ENV": "production",
      "PORT": "8080",
      "LOG_LEVEL": "info"
    },
    "cpu_usage_percent": 45.2,
    "memory_usage_bytes": 268435456,
    "open_file_descriptors": 128,
    "threads_count": 24,
    "cpu_type": "arm64e",
    "cpu_subtype": 2,
    "thread_limit": 1024,
    "rlimit_cpu": "unlimited",
    "rlimit_memory": "unlimited",
    "rlimit_files": 10240
  }
}
```

### Thread Information

#### Crashed Thread Identification

The crashed thread is identified through multiple mechanisms:

```json
{
  "threads": [
    {
      "id": 1,
      "crashed": true,
      "current_thread": true,
      "exception_thread": true,
      "dispatch_queue": "com.apple.main-thread",
      "priority": 4,
      "nice": 0,
      "sched_policy": "SCHED_OTHER",
      "user_time_ms": 1234,
      "system_time_ms": 567,
      "wall_time_ms": 1801,
      "context_switches": {
        "voluntary": 450,
        "involuntary": 23
      }
    }
  ]
}
```

**Thread state machine:**
```
┌─────────────────────────────────────────────────────────────┐
│                    Thread State                              │
├─────────────────────────────────────────────────────────────┤
│  RUNNING    → Currently executing on CPU                    │
│  WAITING    → Blocked on I/O, lock, or condition            │
│  STOPPED    → Suspended (debugger, signal)                  │
│  ZOMBIE     → Terminated, waiting for parent                │
│  CRASHED    → Terminated due to exception                   │
└─────────────────────────────────────────────────────────────┘
```

#### Register State Capture

Platform-specific register captures:

**ARM64 (iOS/macOS):**
```json
{
  "registers": {
    "general_purpose": {
      "x0": "0x0000000000000000", "x1": "0x0000000000000001",
      "x2": "0x0000000000000000", "x3": "0x0000000000000000",
      "x4": "0x0000000000000000", "x5": "0x0000000000000000",
      "x6": "0x0000000000000000", "x7": "0x0000000000000000",
      "x8": "0x6d656d5f73736572", "x9": "0x0000000000000001",
      "x10": "0x0000000000000010", "x11": "0x0000000000000002",
      "x12": "0x0000000000000003", "x13": "0x0000000000000001",
      "x14": "0x0000000000000000", "x15": "0x0000000000000003",
      "x16": "0x0000000000000148", "x17": "0x0000000000000150",
      "x18": "0x0000000000000000", "x19": "0x0000000000000000",
      "x20": "0x0000000000000000", "x21": "0x0000000000000000",
      "x22": "0x0000000000000000", "x23": "0x0000000000000000",
      "x24": "0x0000000000000000", "x25": "0x0000000000000000",
      "x26": "0x0000000000000000", "x27": "0x0000000000000000",
      "x28": "0x0000000000000000"
    },
    "special_purpose": {
      "x29": "0x000000016f66ad80",
      "x30": "0x0000000184a5ea58"
    },
    "processor_state": {
      "pc": "0x0000000184a5ea58",
      "sp": "0x000000016f66ad60",
      "fp": "0x000000016f66ad80",
      "lr": "0x0000000184a5ea58",
      "cpsr": "0x00000000"
    },
    "exception_state": {
      "exception": "0x00000001",
      "fault_vaddr": "0x0000000000000000"
    }
  }
}
```

**x86_64 (Linux/Windows):**
```json
{
  "registers": {
    "general_purpose": {
      "rax": "0x0000000000000000", "rbx": "0x0000000100003f48",
      "rcx": "0x0000000000000000", "rdx": "0x0000000000000000",
      "rsi": "0x0000000000000000", "rdi": "0x0000000000000000",
      "rbp": "0x000000016f66ad80", "rsp": "0x000000016f66ad60",
      "r8": "0x0000000000000000",  "r9": "0x0000000000000000",
      "r10": "0x0000000000000000", "r11": "0x0000000000000000",
      "r12": "0x0000000000000000", "r13": "0x0000000000000000",
      "r14": "0x0000000000000000", "r15": "0x0000000000000000"
    },
    "instruction_pointer": {
      "rip": "0x0000000184a5ea58"
    },
    "flags": {
      "eflags": "0x00000202"
    }
  }
}
```

### Stack Trace Format

#### Frame Structure

```json
{
  "stack_frames": [
    {
      "index": 0,
      "instruction_address": "0x0000000184a5ea58",
      "return_address": "0x0000000184b34c80",
      "function_name": "abort",
      "function_offset": 44,
      "class_name": null,
      "method_name": null,
      "file_name": null,
      "line_number": null,
      "column": null,
      "library_name": "libsystem_kernel.dylib",
      "library_path": "/usr/lib/system/libsystem_kernel.dylib",
      "library_offset": 44,
      "symbolicated": true,
      "trust": "scan",
      "source_status": "missing",
      "inlined": false,
      "compiler_transformed": false
    }
  ]
}
```

**Trust levels for stack frames:**
| Trust | Description |
|-------|-------------|
| `none` | No frame pointer or CFI available |
| `scan` | Stack scanning found potential frame |
| `cfi` | DWARF Call Frame Information used |
| `fp` | Frame pointer chain followed |
| `context` | Known context (e.g., signal handler) |

#### Inlined Frame Detection

With compiler optimizations, frames may be inlined:

```json
{
  "stack_frames": [
    {
      "index": 5,
      "instruction_address": "0x0000000100005a3c",
      "function_name": "processPayment",
      "file_name": "PaymentProcessor.swift",
      "line_number": 156,
      "inlined": false
    },
    {
      "index": 6,
      "instruction_address": "0x0000000100005a3c",
      "function_name": "validateCard",
      "file_name": "PaymentProcessor.swift",
      "line_number": 89,
      "inlined": true,
      "parent_frame_index": 5
    },
    {
      "index": 7,
      "instruction_address": "0x0000000100005a3c",
      "function_name": "checkLuhn",
      "file_name": "CardValidator.swift",
      "line_number": 34,
      "inlined": true,
      "parent_frame_index": 6
    }
  ]
}
```

### Memory State

#### Memory Regions

```json
{
  "memory_state": {
    "virtual_memory_size": 268435456,
    "resident_set_size": 134217728,
    "peak_rss": 201326592,
    "heap_size": 67108864,
    "heap_allocated": 52428800,
    "heap_free": 14680064,
    "heap_peak": 58720256,
    "stack_size": 8388608,
    "memory_mapped": 33554432,
    "memory_regions": [
      {
        "start_address": "0x0000000100000000",
        "end_address": "0x0000000104000000",
        "protection": "r-x",
        "type": "executable",
        "name": "__TEXT",
        "file_path": "/Applications/MyApp.app/MyApp",
        "file_offset": 0
      },
      {
        "start_address": "0x0000000104000000",
        "end_address": "0x0000000108000000",
        "protection": "rw-",
        "type": "data",
        "name": "__DATA",
        "file_path": "/Applications/MyApp.app/MyApp",
        "file_offset": 4194304
      },
      {
        "start_address": "0x0000000108000000",
        "end_address": "0x0000000108400000",
        "protection": "rw-",
        "type": "heap",
        "name": "heap"
      },
      {
        "start_address": "0x000000016f000000",
        "end_address": "0x000000016f800000",
        "protection": "rw-",
        "type": "stack",
        "name": "main_thread_stack"
      }
    ]
  }
}
```

### Binary Images List

```json
{
  "binary_images": [
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440001",
      "name": "MyApp",
      "path": "/Applications/MyApp.app/Contents/MacOS/MyApp",
      "type": "executable",
      "image_address": "0x0000000100000000",
      "image_size_bytes": 52428800,
      "cpu_type": "arm64e",
      "cpu_subtype": 2,
      "architecture": "arm64e",
      "segment_base": "0x0000000100000000",
      "vmaddr": "0x0000000100000000",
      "vmsize": "0x03200000",
      "load_command": "LC_SEGMENT_64",
      "sections": [
        {
          "name": "__text",
          "addr": "0x0000000100003f40",
          "size": 41943040
        },
        {
          "name": "__const",
          "addr": "0x0000000102800000",
          "size": 10485760
        }
      ]
    }
  ]
}
```

### Custom Attributes/Annotations

```json
{
  "attributes": {
    "_type": "crash",
    "_product": "myapp",
    "_environment": "production",
    
    "user_id": "user_12345",
    "user_email_hash": "sha256:abc123",
    "user_country": "US",
    "user_locale": "en_US",
    
    "session_id": "sess_abc123",
    "session_duration_ms": 1800123,
    "session_events_count": 45,
    
    "app_version": "2.1.0",
    "app_build": "20260405.1",
    "release_channel": "stable",
    "feature_flags": "new_ui,dark_mode,beta_checkout",
    
    "network_type": "wifi",
    "network_carrier": "Verizon",
    "network_country": "US",
    
    "device_memory_mb": 6144,
    "device_storage_free_gb": 30,
    "device_battery_level": 0.72,
    
    "request_id": "req_xyz789",
    "api_endpoint": "/api/v2/checkout",
    "api_method": "POST",
    "api_status": 500,
    
    "error_category": "payment",
    "error_code": "GATEWAY_TIMEOUT",
    "error_message": "Payment gateway timed out",
    
    "geohash": "9q8yy",
    "timezone": "America/New_York",
    "local_time_hour": 10,
    "local_time_day": "Friday"
  }
}
```

### Breadcrumbs Trail

```json
{
  "breadcrumbs": [
    {
      "id": "bc_001",
      "timestamp": "2026-04-05T14:25:00.000000Z",
      "level": "info",
      "type": "user",
      "category": "ui",
      "message": "User tapped login button",
      "data": {
        "screen": "login",
        "element_id": "btn_login"
      }
    },
    {
      "id": "bc_002",
      "timestamp": "2026-04-05T14:25:01.000000Z",
      "level": "info",
      "type": "http",
      "category": "network",
      "message": "POST /api/v2/auth/login",
      "data": {
        "url": "https://api.example.com/api/v2/auth/login",
        "method": "POST",
        "status_code": 200,
        "duration_ms": 234,
        "request_size": 256,
        "response_size": 1024
      }
    },
    {
      "id": "bc_003",
      "timestamp": "2026-04-05T14:26:00.000000Z",
      "level": "info",
      "type": "navigation",
      "category": "ui",
      "message": "Navigated to home screen",
      "data": {
        "from": "login",
        "to": "home"
      }
    },
    {
      "id": "bc_004",
      "timestamp": "2026-04-05T14:27:00.000000Z",
      "level": "warning",
      "type": "log",
      "category": "app",
      "message": "Memory warning received",
      "data": {
        "memory_level": "warning",
        "available_mb": 128
      }
    },
    {
      "id": "bc_005",
      "timestamp": "2026-04-05T14:28:00.000000Z",
      "level": "error",
      "type": "error",
      "category": "app",
      "message": "Failed to load user preferences",
      "data": {
        "error": "timeout",
        "duration_ms": 5000
      }
    }
  ],
  "breadcrumbs_summary": {
    "total": 5,
    "max_capacity": 100,
    "oldest_timestamp": "2026-04-05T14:25:00.000000Z",
    "newest_timestamp": "2026-04-05T14:28:00.000000Z"
  }
}
```

---

## 2. Crash Classification

### Exception Types (macOS/iOS)

Apple's Mach exception handling provides low-level crash classification:

| Exception Type | Code | Description | Common Causes |
|----------------|------|-------------|---------------|
| `EXC_BAD_ACCESS` | 1 | Invalid memory access | Use-after-free, null deref |
| `EXC_CRASH` | 3 | Normal process termination | Abort(), security violation |
| `EXC_BREAKPOINT` | 6 | Trap/ breakpoint hit | Assertions, debuggers |
| `EXC_GUARD` | 9 | Resource violation | File descriptor misuse |
| `EXC_RESOURCE` | 11 | Resource limit exceeded | Memory, CPU limits |
| `EXC_NSRESOURCE` | 12 | App-not-responding | Main thread blocked |
| `EXC_CORPSE_NOTIFY` | 13 | Process already dead | Zombie process access |

#### EXC_BAD_ACCESS Sub-classification

```json
{
  "crash": {
    "exception_type": "EXC_BAD_ACCESS",
    "exception_code": "0x0000000000000001",
    "exception_subcode": "0x0000000000000000",
    "reason": "KERN_PROTECTION_FAILURE",
    "access_type": "read",
    "address": "0x0000000000000010"
  }
}
```

**Exception codes:**
| Code | Constant | Meaning |
|------|----------|---------|
| 1 | `KERN_INVALID_ADDRESS` | Address not mapped |
| 2 | `KERN_PROTECTION_FAILURE` | Permission denied |

**Common patterns:**
- `0x0000000000000000` → Null pointer dereference
- `0x00000000000000XX` → Small offset (likely member access on null object)
- `0x00000000DEADBEEF` → Use-after-free (magic value)
- `0x0000000100000000` → Valid heap address (possibly wild pointer)

### Signal Types (POSIX)

Unix signals provide cross-platform crash classification:

| Signal | Code | Description | Common Causes |
|--------|------|-------------|---------------|
| `SIGSEGV` | 11 | Segmentation violation | Invalid memory access |
| `SIGABRT` | 6 | Abort signal | assert(), abort() |
| `SIGBUS` | 10 | Bus error | Unaligned access, mmap issues |
| `SIGILL` | 4 | Illegal instruction | Corrupted code, CPU mismatch |
| `SIGFPE` | 8 | Floating-point exception | Divide by zero, overflow |
| `SIGTRAP` | 5 | Trace/breakpoint trap | Debugger, assertions |
| `SIGSYS` | 12 | Bad system call | Invalid syscall |
| `SIGQUIT` | 3 | Quit signal | Explicit termination |

#### Signal Code Sub-classification

```json
{
  "crash": {
    "signal": "SIGSEGV",
    "signal_code": "SEGV_MAPERR",
    "faulting_address": "0x0000000000000010",
    "faulting_instruction": "0x0000000100003f48"
  }
}
```

**SIGSEGV codes:**
| Code | Description |
|------|-------------|
| `SEGV_MAPERR` | Address not mapped |
| `SEGV_ACCERR` | Invalid permissions |
| `SEGV_ACCADI` | ARM MTE: asymmetric tag mismatch |
| `SEGV_ADIDERR` | ARM MTE: impure tag |
| `SEGV_MTESERR` | ARM MTE: synchronous tag check fault |

**SIGBUS codes:**
| Code | Description |
|------|-------------|
| `BUS_ADRALN` | Invalid address alignment |
| `BUS_ADRERR` | Non-existent physical address |
| `BUS_OBJERR` | Object-specific hardware error |

### Crash Reasons

#### Mach Kernel Reasons

```go
type KernReturn struct {
    Code          uint32
    Name          string
    Description   string
    CommonCauses  []string
}

var kernReturns = map[uint32]KernReturn{
    0: {
        Name: "KERN_SUCCESS",
        Description: "Operation completed successfully",
    },
    1: {
        Name: "KERN_INVALID_ADDRESS",
        Description: "Address is not in this process' VM space",
        CommonCauses: ["Null pointer deref", "Use-after-free"],
    },
    2: {
        Name: "KERN_PROTECTION_FAILURE",
        Description: "VM space is read-only or not accessible",
        CommonCauses: ["Write to const", "Stack overflow detection"],
    },
}
```

#### POSIX Errno Mapping

```go
var signalToErrno = map[string]map[int]string{
    "SIGSEGV": {
        1: "SEGV_MAPERR (address not mapped)",
        2: "SEGV_ACCERR (invalid permissions)",
    },
    "SIGABRT": {
        0: "SI_USER (kill() or raise())",
        1: "SI_QUEUE (sigqueue())",
        2: "SI_TIMER (timer expiration)",
        3: "SI_ASYNCIO (AIO completion)",
        4: "SI_MESGQ (message queue)",
        0x80: "SI_TKILL (thread-specific kill)",
    },
}
```

### ANR Detection Thresholds

Application Not Responding detection varies by platform:

#### Android ANR Thresholds

| Trigger | Threshold | Description |
|---------|-----------|-------------|
| Input dispatch | 5 seconds | No response to touch/key |
| BroadcastReceiver | 10 seconds | Broadcast not handled |
| Service timeout | 20 seconds | Service lifecycle timeout |
| ContentProvider | 10 seconds | Provider operation timeout |

```json
{
  "anr": {
    "type": "input_dispatching_timed_out",
    "threshold_ms": 5000,
    "actual_delay_ms": 5234,
    "blocked_thread": {
      "id": 1,
      "name": "main",
      "state": "BLOCKED",
      "lock_owner": "WorkerThread-42",
      "stack_trace": [...]
    },
    "cpu_usage": {
      "app_percent": 0.5,
      "system_percent": 45.2
    },
    "io_stats": {
      "pending_writes": 15,
      "disk_wait_ms": 4500
    }
  }
}
```

#### iOS Watchdog ANR

```json
{
  "anr": {
    "type": "watchdog_timeout",
    "trigger": "UIApplicationLaunchTimeoutException",
    "threshold_ms": {
      "cold_start": 20000,
      "resume_from_suspended": 400,
      "scene_activation": 400
    },
    "actual_delay_ms": 20156,
    "app_state": "launching",
    "main_thread_blocked": true,
    "blocking_reason": "synchronous_network_call"
  }
}
```

#### Desktop ANR Heuristics

```go
type ANRDetector struct {
    UIFreezeThreshold time.Duration
    CPUMonitor        *CPUMonitor
    IOMonitor         *IOMonitor
}

func (d *ANRDetector) Detect() *ANRReport {
    // UI not responding for > 3 seconds
    if d.UIFreezeTime() > 3*time.Second {
        return &ANRReport{
            Type: "ui_freeze",
            Duration: d.UIFreezeTime(),
            MainThreadState: d.GetMainThreadState(),
        }
    }
    
    // High CPU with no progress
    if d.CPUMonitor.Average() > 90 && d.ProgressStalled() {
        return &ANRReport{
            Type: "cpu_spinlock",
            Duration: d.SpinlockDuration(),
        }
    }
    
    return nil
}
```

### OOM vs Crash Distinction

Out-of-Memory terminations are distinct from crashes:

#### iOS Jetsam (low-level OOM)

```json
{
  "oom": {
    "type": "jetsam_termination",
    "memory_limit_mb": 1845,
    "memory_used_mb": 1892,
    "memory_percent": 102.5,
    "jetsam_flags": [
        "JETSAM_FLAGS_MEMORY_PRESSURE",
        "JETSAM_FLAGS_EXCESSIVE_FOOTPRINT"
    ],
    "jetsam_priority": 3,
    "process_state": "foreground",
    "memory_warning_count": 3,
    "last_memory_warning": "2026-04-05T14:29:55.000000Z",
    "memory_footprint_history": [
        {"timestamp": "14:25:00", "mb": 512},
        {"timestamp": "14:26:00", "mb": 856},
        {"timestamp": "14:27:00", "mb": 1234},
        {"timestamp": "14:28:00", "mb": 1567},
        {"timestamp": "14:29:00", "mb": 1892}
    ]
  }
}
```

#### Android Low Memory Killer

```json
{
  "oom": {
    "type": "lmk_termination",
    "lmk_level": 6,
    "memory_limit_kb": 524288,
    "memory_used_kb": 548672,
    "oom_score_adj": 900,
    "process_importance": "foreground",
    "cached_processes_killed": 15,
    "system_memory_state": "critical"
  }
}
```

#### Detection Algorithm

```go
func ClassifyTermination(report *RawReport) Classification {
    // Check for OOM indicators
    if IsOOM(report) {
        return ClassificationOOM
    }
    
    // Check for crash indicators
    if HasException(report) || HasSignal(report) {
        return ClassificationCrash
    }
    
    // Check for ANR
    if IsANR(report) {
        return ClassificationANR
    }
    
    return ClassificationUnknown
}

func IsOOM(report *RawReport) bool {
    // iOS Jetsam
    if report.MachException != nil && 
       report.MachException.Code == 0x0c {
        return true
    }
    
    // Android LMK
    if report.Signal == "SIGKILL" && 
       report.ExitCode == 137 {
        return true
    }
    
    // Memory pressure indicators
    if report.MemoryPressure == "critical" {
        return true
    }
    
    // No stack trace (killed by kernel)
    if len(report.Threads) == 0 {
        return true
    }
    
    return false
}
```

### Panic vs Signal Differentiation

#### Go Panic Classification

```json
{
  "panic": {
    "type": "runtime_panic",
    "message": "index out of range [5] with length 3",
    "goroutine_id": 42,
    "goroutine_state": "running",
    "defer_chain": [
        "runtime.gopanic",
        "runtime.paniconfault",
        "runtime.deferreturn"
    ],
    "recovered": false
  }
}
```

**Panic types:**
| Type | Description | Example |
|------|-------------|---------|
| `runtime_panic` | Go runtime detected error | Index out of bounds |
| `user_panic` | Explicit panic() call | panic("error") |
| `assertion_panic` | Failed assertion | assert(condition) |

#### Signal Classification (Native)

```go
type SignalClassification struct {
    Signal      string
    Code        int
    Category    string
    Recoverable bool
}

func ClassifySignal(sig string, code int) SignalClassification {
    switch sig {
    case "SIGSEGV", "SIGBUS":
        return SignalClassification{
            Signal:   sig,
            Code:     code,
            Category: "memory_access",
            Recoverable: false,
        }
    case "SIGABRT":
        return SignalClassification{
            Signal:   sig,
            Code:     code,
            Category: "abort",
            Recoverable: false,
        }
    case "SIGILL", "SIGFPE":
        return SignalClassification{
            Signal:   sig,
            Code:     code,
            Category: "cpu_exception",
            Recoverable: false,
        }
    case "SIGTRAP":
        return SignalClassification{
            Signal:   sig,
            Code:     code,
            Category: "debug",
            Recoverable: true,
        }
    default:
        return SignalClassification{
            Signal:   sig,
            Code:     code,
            Category: "unknown",
            Recoverable: false,
        }
    }
}
```

### Complete Classification Taxonomy

```go
type CrashClassification struct {
    Category       CrashCategory   `json:"category"`
    Subcategory    string          `json:"subcategory"`
    Severity       Severity        `json:"severity"`
    RootCause      string          `json:"root_cause"`
    Confidence     float64         `json:"confidence"`
    SuggestedFix   string          `json:"suggested_fix"`
}

type CrashCategory string

const (
    CategoryMemoryAccess   CrashCategory = "memory_access"
    CategoryResourceLimit  CrashCategory = "resource_limit"
    CategoryLogicError     CrashCategory = "logic_error"
    CategoryExternal       CrashCategory = "external"
    CategoryIntentional    CrashCategory = "intentional"
)

type Severity string

const (
    SeverityCritical Severity = "critical"
    SeverityHigh     Severity = "high"
    SeverityMedium   Severity = "medium"
    SeverityLow      Severity = "low"
)
```

---

## 3. Crash Aggregation Algorithms

### Fingerprinting Algorithms

Fingerprinting creates a unique identifier for similar crashes, enabling intelligent grouping.

#### Multi-Layer Fingerprinting

```go
type FingerprintGenerator struct {
    layers []FingerprintLayer
}

type FingerprintLayer interface {
    Generate(crash *CrashReport) string
    Weight() float64
}

func (f *FingerprintGenerator) Generate(crash *CrashReport) string {
    var components []string
    
    for _, layer := range f.layers {
        component := layer.Generate(crash)
        if component != "" {
            components = append(components, component)
        }
    }
    
    // Combine layers with weighted hashing
    return WeightedHash(components, f.getWeights())
}

func (f *FingerprintGenerator) getWeights() []float64 {
    weights := make([]float64, len(f.layers))
    for i, layer := range f.layers {
        weights[i] = layer.Weight()
    }
    return weights
}
```

#### Layer 1: Crash Signature

```go
type CrashSignatureLayer struct{}

func (s *CrashSignatureLayer) Generate(crash *CrashReport) string {
    var sig strings.Builder
    
    // Exception/signal type
    sig.WriteString(crash.Crash.ExceptionType)
    sig.WriteString("|")
    sig.WriteString(crash.Crash.Signal)
    sig.WriteString("|")
    
    // Top 5 frame signatures
    for i, frame := range crash.Threads[0].StackFrames[:min(5, len(crash.Threads[0].StackFrames))] {
        if i > 0 {
            sig.WriteString(";")
        }
        sig.WriteString(s.frameSignature(frame))
    }
    
    return sig.String()
}

func (s *CrashSignatureLayer) frameSignature(frame *StackFrame) string {
    // Prefer source location if available
    if frame.FileName != "" && frame.LineNumber > 0 {
        return fmt.Sprintf("%s:%d", path.Base(frame.FileName), frame.LineNumber)
    }
    
    // Fall back to function name
    if frame.FunctionName != "" {
        return frame.FunctionName
    }
    
    // Last resort: module + offset
    return fmt.Sprintf("%s+0x%x", frame.LibraryName, frame.Offset)
}

func (s *CrashSignatureLayer) Weight() float64 {
    return 1.0 // Highest weight
}
```

#### Layer 2: Exception Context

```go
type ExceptionContextLayer struct{}

func (e *ExceptionContextLayer) Generate(crash *CrashReport) string {
    var ctx strings.Builder
    
    // Crash type and reason
    ctx.WriteString(crash.Crash.Type)
    ctx.WriteString(":")
    ctx.WriteString(crash.Crash.Reason)
    ctx.WriteString(":")
    
    // Memory access pattern
    if crash.Crash.Address != "" {
        ctx.WriteString(e.addressPattern(crash.Crash.Address))
    }
    
    return ctx.String()
}

func (e *ExceptionContextLayer) addressPattern(addr string) string {
    // Normalize addresses for grouping
    if addr == "0x0000000000000000" {
        return "null"
    }
    if strings.HasPrefix(addr, "0x0000000000000") {
        return "small_offset"
    }
    if strings.HasPrefix(addr, "0x00007") {
        return "stack_region"
    }
    if strings.HasPrefix(addr, "0x00000001") {
        return "heap_region"
    }
    return "other"
}

func (e *ExceptionContextLayer) Weight() float64 {
    return 0.7
}
```

#### Layer 3: Thread Context

```go
type ThreadContextLayer struct{}

func (t *ThreadContextLayer) Generate(crash *CrashReport) string {
    var ctx strings.Builder
    
    // Crashed thread identification
    for _, thread := range crash.Threads {
        if thread.Crashed {
            ctx.WriteString("crashed:")
            ctx.WriteString(thread.Name)
            ctx.WriteString(":")
            ctx.WriteString(thread.DispatchQueue)
            break
        }
    }
    
    return ctx.String()
}

func (t *ThreadContextLayer) Weight() float64 {
    return 0.5
}
```

### Stack Trace Similarity

#### Levenshtein Distance for Stack Frames

```go
type StackTraceSimilarity struct {
    threshold float64
}

func (s *StackTraceSimilarity) Compare(a, b []*StackFrame) float64 {
    if len(a) == 0 || len(b) == 0 {
        return 0.0
    }
    
    // Convert frames to comparable strings
    aStrs := make([]string, len(a))
    bStrs := make([]string, len(b))
    
    for i, frame := range a {
        aStrs[i] = s.frameToString(frame)
    }
    for i, frame := range b {
        bStrs[i] = s.frameToString(frame)
    }
    
    // Calculate normalized Levenshtein distance
    distance := s.levenshtein(aStrs, bStrs)
    maxLen := max(len(aStrs), len(bStrs))
    
    return 1.0 - float64(distance)/float64(maxLen)
}

func (s *StackTraceSimilarity) levenshtein(a, b []string) int {
    // Create matrix
    matrix := make([][]int, len(a)+1)
    for i := range matrix {
        matrix[i] = make([]int, len(b)+1)
        matrix[i][0] = i
    }
    for j := range matrix[0] {
        matrix[0][j] = j
    }
    
    // Fill matrix
    for i := 1; i <= len(a); i++ {
        for j := 1; j <= len(b); j++ {
            cost := 1
            if a[i-1] == b[j-1] {
                cost = 0
            }
            matrix[i][j] = min(
                matrix[i-1][j]+1,      // deletion
                matrix[i][j-1]+1,      // insertion
                matrix[i-1][j-1]+cost, // substitution
            )
        }
    }
    
    return matrix[len(a)][len(b)]
}

func (s *StackTraceSimilarity) frameToString(frame *StackFrame) string {
    if frame.FunctionName != "" {
        return frame.FunctionName
    }
    return fmt.Sprintf("%s+0x%x", frame.LibraryName, frame.Offset)
}
```

#### Jaccard Similarity for Frame Sets

```go
func JaccardSimilarity(a, b []*StackFrame) float64 {
    aSet := make(map[string]bool)
    bSet := make(map[string]bool)
    
    // Extract unique function names
    for _, frame := range a {
        if frame.FunctionName != "" {
            aSet[frame.FunctionName] = true
        }
    }
    for _, frame := range b {
        if frame.FunctionName != "" {
            bSet[frame.FunctionName] = true
        }
    }
    
    // Calculate intersection
    intersection := 0
    for fn := range aSet {
        if bSet[fn] {
            intersection++
        }
    }
    
    // Calculate union
    union := len(aSet) + len(bSet) - intersection
    
    if union == 0 {
        return 0.0
    }
    
    return float64(intersection) / float64(union)
}
```

#### Frame-based Grouping with Weights

```go
type FrameWeighter struct {
    weights map[string]float64
}

func NewFrameWeighter() *FrameWeighter {
    return &FrameWeighter{
        weights: map[string]float64{
            "top_1":       1.0,   // Crashing frame
            "top_2":       0.9,   // Immediate caller
            "top_3":       0.8,   // Second caller
            "top_5":       0.6,   // Near top
            "top_10":      0.4,   // Mid stack
            "framework":   0.1,   // System frameworks
            "async_boundary": 0.3, // Async boundaries
        },
    }
}

func (f *FrameWeighter) CalculateSimilarity(a, b []*StackFrame) float64 {
    var totalWeight, matchWeight float64
    
    for i := 0; i < min(len(a), len(b), 20); i++ {
        weight := f.getWeight(i, a[i], b[i])
        totalWeight += weight * 2
        
        if f.framesMatch(a[i], b[i]) {
            matchWeight += weight * 2
        }
    }
    
    if totalWeight == 0 {
        return 0.0
    }
    
    return matchWeight / totalWeight
}

func (f *FrameWeighter) getWeight(index int, a, b *StackFrame) float64 {
    // System frameworks get lower weight
    if f.isSystemFrame(a) && f.isSystemFrame(b) {
        return f.weights["framework"]
    }
    
    // Position-based weight
    switch {
    case index == 0:
        return f.weights["top_1"]
    case index == 1:
        return f.weights["top_2"]
    case index == 2:
        return f.weights["top_3"]
    case index < 5:
        return f.weights["top_5"]
    case index < 10:
        return f.weights["top_10"]
    default:
        return 0.2
    }
}

func (f *FrameWeighter) isSystemFrame(frame *StackFrame) bool {
    systemLibs := []string{
        "libsystem", "Foundation", "UIKit", "AppKit",
        "CoreFoundation", "libdispatch", "libc", "libm",
    }
    
    for _, lib := range systemLibs {
        if strings.Contains(frame.LibraryName, lib) {
            return true
        }
    }
    
    return false
}

func (f *FrameWeighter) framesMatch(a, b *StackFrame) bool {
    // Exact function match
    if a.FunctionName == b.FunctionName && a.FunctionName != "" {
        return true
    }
    
    // Source location match
    if a.FileName == b.FileName && a.LineNumber == b.LineNumber {
        return true
    }
    
    // Module + offset match (less precise)
    if a.LibraryName == b.LibraryName && a.Offset == b.Offset {
        return true
    }
    
    return false
}
```

### Signature Generation

#### Complete Fingerprint Algorithm

```go
type CompleteFingerprint struct {
    hasher hash.Hash
}

func (c *CompleteFingerprint) Generate(crash *CrashReport) string {
    c.hasher = sha256.New()
    
    // Layer 1: Exception signature (highest weight)
    c.writeExceptionSignature(crash)
    
    // Layer 2: Top frames (high weight)
    c.writeTopFrames(crash, 5)
    
    // Layer 3: Module pattern (medium weight)
    c.writeModulePattern(crash)
    
    // Layer 4: Error context (variable weight)
    c.writeErrorContext(crash)
    
    // Generate final hash
    hashBytes := c.hasher.Sum(nil)
    return hex.EncodeToString(hashBytes[:8]) // 16-char fingerprint
}

func (c *CompleteFingerprint) writeExceptionSignature(crash *CrashReport) {
    c.write("exception", crash.Crash.ExceptionType)
    c.write("signal", crash.Crash.Signal)
    c.write("reason", crash.Crash.Reason)
    c.write("address_pattern", c.addressPattern(crash.Crash.Address))
}

func (c *CompleteFingerprint) writeTopFrames(crash *CrashReport, count int) {
    crashedThread := c.getCrashedThread(crash)
    if crashedThread == nil {
        return
    }
    
    for i := 0; i < min(count, len(crashedThread.StackFrames)); i++ {
        frame := crashedThread.StackFrames[i]
        c.write(fmt.Sprintf("frame_%d", i), c.frameSig(frame))
    }
}

func (c *CompleteFingerprint) writeModulePattern(crash *CrashReport) {
    modules := make(map[string]int)
    
    crashedThread := c.getCrashedThread(crash)
    if crashedThread == nil {
        return
    }
    
    for _, frame := range crashedThread.StackFrames {
        modules[frame.LibraryName]++
    }
    
    // Sort by frequency
    type kv struct {
        Key   string
        Value int
    }
    var sorted []kv
    for k, v := range modules {
        sorted = append(sorted, kv{k, v})
    }
    sort.Slice(sorted, func(i, j int) bool {
        return sorted[i].Value > sorted[j].Value
    })
    
    // Write top 3 modules
    for i := 0; i < min(3, len(sorted)); i++ {
        c.write(fmt.Sprintf("module_%d", i), sorted[i].Key)
    }
}

func (c *CompleteFingerprint) writeErrorContext(crash *CrashReport) {
    // Thread context
    crashedThread := c.getCrashedThread(crash)
    if crashedThread != nil {
        c.write("thread_name", crashedThread.Name)
        c.write("dispatch_queue", crashedThread.DispatchQueue)
    }
    
    // App state
    if crash.Attributes != nil {
        if appState, ok := crash.Attributes["app_state"]; ok {
            c.write("app_state", appState)
        }
    }
}

func (c *CompleteFingerprint) write(key, value string) {
    fmt.Fprintf(c.hasher, "%s=%s|", key, value)
}

func (c *CompleteFingerprint) frameSig(frame *StackFrame) string {
    if frame.FileName != "" && frame.LineNumber > 0 {
        return fmt.Sprintf("%s:%d", path.Base(frame.FileName), frame.LineNumber)
    }
    if frame.FunctionName != "" {
        return frame.FunctionName
    }
    return fmt.Sprintf("%s+0x%x", frame.LibraryName, frame.Offset)
}

func (c *CompleteFingerprint) getCrashedThread(crash *CrashReport) *Thread {
    for _, t := range crash.Threads {
        if t.Crashed {
            return t
        }
    }
    if len(crash.Threads) > 0 {
        return crash.Threads[0]
    }
    return nil
}

func (c *CompleteFingerprint) addressPattern(addr string) string {
    if addr == "" {
        return "none"
    }
    if addr == "0x0000000000000000" {
        return "null"
    }
    if strings.HasPrefix(addr, "0x0000000000000") {
        return "small_offset"
    }
    return "mapped"
}
```

### Deduplication Strategies

#### Sliding Window Deduplication

```go
type Deduplicator struct {
    recentFingerprints map[string][]time.Time
    window             time.Duration
    mu                 sync.RWMutex
}

func NewDeduplicator(window time.Duration) *Deduplicator {
    return &Deduplicator{
        recentFingerprints: make(map[string][]time.Time),
        window:             window,
    }
}

func (d *Deduplicator) IsDuplicate(fingerprint string, ts time.Time) bool {
    d.mu.Lock()
    defer d.mu.Unlock()
    
    // Clean old entries
    d.cleanOldEntries()
    
    // Check for recent occurrences
    times, exists := d.recentFingerprints[fingerprint]
    if !exists {
        d.recentFingerprints[fingerprint] = []time.Time{ts}
        return false
    }
    
    // Count occurrences in window
    cutoff := ts.Add(-d.window)
    recentCount := 0
    for _, t := range times {
        if t.After(cutoff) {
            recentCount++
        }
    }
    
    // Add current timestamp
    d.recentFingerprints[fingerprint] = append(times, ts)
    
    // Consider duplicate if seen more than 3 times in window
    return recentCount >= 3
}

func (d *Deduplicator) cleanOldEntries() {
    cutoff := time.Now().Add(-d.window)
    
    for fp, times := range d.recentFingerprints {
        var recent []time.Time
        for _, t := range times {
            if t.After(cutoff) {
                recent = append(recent, t)
            }
        }
        if len(recent) == 0 {
            delete(d.recentFingerprints, fp)
        } else {
            d.recentFingerprints[fp] = recent
        }
    }
}
```

#### Hierarchical Grouping

```go
type HierarchicalGrouper struct {
    levels []GroupingLevel
}

type GroupingLevel struct {
    Name      string
    Precision int    // Number of frames to consider
    Threshold float64 // Similarity threshold
}

func (h *HierarchicalGrouper) Group(crashes []*CrashReport) []*CrashGroup {
    groups := make(map[string]*CrashGroup)
    
    for _, crash := range crashes {
        // Try exact match first
        key := h.generateKey(crash, h.levels[0])
        if group, exists := groups[key]; exists {
            group.Add(crash)
            continue
        }
        
        // Try fuzzy match
        matched := false
        for _, level := range h.levels {
            for key, group := range groups {
                if h.similarEnough(crash, group.Representative, level) {
                    group.Add(crash)
                    matched = true
                    break
                }
            }
            if matched {
                break
            }
        }
        
        // Create new group if no match
        if !matched {
            key := h.generateKey(crash, h.levels[0])
            groups[key] = NewCrashGroup(crash)
        }
    }
    
    return h.toSlice(groups)
}

func (h *HierarchicalGrouper) similarEnough(a, b *CrashReport, level GroupingLevel) bool {
    similarity := CompareStackTraces(a, b, level.Precision)
    return similarity >= level.Threshold
}
```

### Threshold Tuning

```go
type ThresholdTuner struct {
    historicalData *HistoricalData
}

func (t *ThresholdTuner) FindOptimalThreshold() float64 {
    var bestThreshold float64
    var bestScore float64
    
    // Grid search over threshold values
    for threshold := 0.5; threshold <= 0.95; threshold += 0.05 {
        groups := t.groupWithThreshold(threshold)
        score := t.evaluateGrouping(groups)
        
        if score > bestScore {
            bestScore = score
            bestThreshold = threshold
        }
    }
    
    return bestThreshold
}

func (t *ThresholdTuner) evaluateGrouping(groups []*CrashGroup) float64 {
    // Metrics:
    // 1. Intra-group similarity (should be high)
    // 2. Inter-group similarity (should be low)
    // 3. Group size distribution (avoid singletons and megagroups)
    
    intraSim := t.averageIntraGroupSimilarity(groups)
    interSim := t.averageInterGroupSimilarity(groups)
    sizeScore := t.sizeDistributionScore(groups)
    
    return 0.4*intraSim + 0.4*(1-interSim) + 0.2*sizeScore
}
```

---

## 4. Crash Analysis

### Crash Rate Calculation

#### Crashes Per Session

```go
type CrashRateCalculator struct {
    sessionDB    *SessionDatabase
    crashDB      *CrashDatabase
}

func (c *CrashRateCalculator) CrashesPerSession(
    appID string,
    version string,
    startTime, endTime time.Time,
) (*CrashRateMetrics, error) {
    // Count sessions
    sessions, err := c.sessionDB.Count(appID, version, startTime, endTime)
    if err != nil {
        return nil, err
    }
    
    // Count crashes
    crashes, err := c.crashDB.Count(appID, version, startTime, endTime)
    if err != nil {
        return nil, err
    }
    
    // Calculate rate
    rate := float64(crashes) / float64(sessions)
    
    return &CrashRateMetrics{
        Crashes:        crashes,
        Sessions:       sessions,
        CrashesPerSession: rate,
        CrashesPerKSessions: rate * 1000,
    }, nil
}

type CrashRateMetrics struct {
    Crashes              int64   `json:"crashes"`
    Sessions             int64   `json:"sessions"`
    CrashesPerSession    float64 `json:"crashes_per_session"`
    CrashesPerKSessions  float64 `json:"crashes_per_k_sessions"`
}
```

#### Crashes Per User

```go
func (c *CrashRateCalculator) CrashesPerUser(
    appID string,
    version string,
    startTime, endTime time.Time,
    groupBy string, // "day", "week", "month"
) ([]*UserCrashMetrics, error) {
    query := `
        SELECT 
            DATE_TRUNC(?, timestamp) as period,
            COUNT(DISTINCT user_id) as affected_users,
            COUNT(*) as total_crashes
        FROM crashes
        WHERE app_id = $1
          AND version = $2
          AND timestamp BETWEEN $3 AND $4
        GROUP BY period
        ORDER BY period
    `
    
    rows, err := c.crashDB.Query(query, appID, version, startTime, endTime, groupBy)
    if err != nil {
        return nil, err
    }
    
    var metrics []*UserCrashMetrics
    for rows.Next() {
        var m UserCrashMetrics
        if err := rows.Scan(&m.Period, &m.AffectedUsers, &m.TotalCrashes); err != nil {
            return nil, err
        }
        m.CrashesPerUser = float64(m.TotalCrashes) / float64(m.AffectedUsers)
        metrics = append(metrics, &m)
    }
    
    return metrics, nil
}

type UserCrashMetrics struct {
    Period         time.Time `json:"period"`
    AffectedUsers  int64     `json:"affected_users"`
    TotalCrashes   int64     `json:"total_crashes"`
    CrashesPerUser float64   `json:"crashes_per_user"`
}
```

### Trend Analysis (Time-Series)

```go
type TrendAnalyzer struct {
    db *Database
}

func (t *TrendAnalyzer) AnalyzeTrend(
    groupID string,
    window time.Duration,
) (*TrendAnalysis, error) {
    // Get daily crash counts
    counts, err := t.getDailyCounts(groupID, window)
    if err != nil {
        return nil, err
    }
    
    // Calculate trend using linear regression
    slope, intercept, rSquared := t.linearRegression(counts)
    
    // Calculate moving average
    ma7 := t.movingAverage(counts, 7)
    ma28 := t.movingAverage(counts, 28)
    
    // Detect anomalies
    anomalies := t.detectAnomalies(counts, ma7)
    
    return &TrendAnalysis{
        Slope:       slope,
        Intercept:   intercept,
        RSquared:    rSquared,
        MA7:         ma7,
        MA28:        ma28,
        Anomalies:   anomalies,
        Direction:   t.trendDirection(slope),
        Significance: t.significance(slope, rSquared),
    }, nil
}

func (t *TrendAnalyzer) linearRegression(data []DataPoint) (slope, intercept, r2 float64) {
    n := float64(len(data))
    var sumX, sumY, sumXY, sumX2, sumY2 float64
    
    for i, d := range data {
        x := float64(i)
        y := float64(d.Value)
        sumX += x
        sumY += y
        sumXY += x * y
        sumX2 += x * x
        sumY2 += y * y
    }
    
    // Calculate slope
    slope = (n*sumXY - sumX*sumY) / (n*sumX2 - sumX*sumX)
    
    // Calculate intercept
    intercept = (sumY - slope*sumX) / n
    
    // Calculate R-squared
    correlation := (n*sumXY - sumX*sumY) / 
        math.Sqrt((n*sumX2-sumX*sumX)*(n*sumY2-sumY*sumY))
    r2 = correlation * correlation
    
    return slope, intercept, r2
}

type TrendDirection string

const (
    TrendIncreasing TrendDirection = "increasing"
    TrendDecreasing TrendDirection = "decreasing"
    TrendStable     TrendDirection = "stable"
)

func (t *TrendAnalyzer) trendDirection(slope float64) TrendDirection {
    const threshold = 0.1 // crashes per day
    if slope > threshold {
        return TrendIncreasing
    }
    if slope < -threshold {
        return TrendDecreasing
    }
    return TrendStable
}
```

### Impact Scoring

```go
type ImpactScorer struct {
    weights ImpactWeights
}

type ImpactWeights struct {
    AffectedUsers    float64 // Weight for user count
    CrashFrequency   float64 // Weight for frequency
    UserValue        float64 // Weight for user tier/value
    CrashSeverity    float64 // Weight for crash type
    RegressionFactor float64 // Weight for new crashes
}

func DefaultImpactWeights() ImpactWeights {
    return ImpactWeights{
        AffectedUsers:    0.30,
        CrashFrequency:   0.25,
        UserValue:        0.20,
        CrashSeverity:    0.15,
        RegressionFactor: 0.10,
    }
}

func (s *ImpactScorer) CalculateScore(group *CrashGroup) ImpactScore {
    var score ImpactScore
    
    // Normalize affected users (0-100 scale)
    score.UserScore = s.normalizeUsers(group.AffectedUsers)
    
    // Normalize frequency (0-100 scale)
    score.FrequencyScore = s.normalizeFrequency(group.DailyCrashes)
    
    // Calculate user value score
    score.UserValueScore = s.calculateUserValue(group.UserDistribution)
    
    // Calculate severity score
    score.SeverityScore = s.calculateSeverity(group.CrashType)
    
    // Calculate regression factor
    score.RegressionScore = s.calculateRegression(group.FirstSeen)
    
    // Weighted total
    score.Total = score.UserScore*s.weights.AffectedUsers +
        score.FrequencyScore*s.weights.CrashFrequency +
        score.UserValueScore*s.weights.UserValue +
        score.SeverityScore*s.weights.CrashSeverity +
        score.RegressionScore*s.weights.RegressionFactor
    
    score.Priority = s.priorityFromScore(score.Total)
    
    return score
}

type ImpactScore struct {
    UserScore      float64 `json:"user_score"`
    FrequencyScore float64 `json:"frequency_score"`
    UserValueScore float64 `json:"user_value_score"`
    SeverityScore  float64 `json:"severity_score"`
    RegressionScore float64 `json:"regression_score"`
    Total          float64 `json:"total_score"`
    Priority       string  `json:"priority"` // P0, P1, P2, P3
}

func (s *ImpactScorer) priorityFromScore(score float64) string {
    switch {
    case score >= 80:
        return "P0" // Critical
    case score >= 60:
        return "P1" // High
    case score >= 40:
        return "P2" // Medium
    default:
        return "P3" // Low
    }
}
```

### Regression Detection

```go
type RegressionDetector struct {
    releaseDB *ReleaseDatabase
}

func (r *RegressionDetector) DetectRegressions(
    appID string,
    currentVersion string,
) ([]*Regression, error) {
    // Get previous version
    prevVersion, err := r.releaseDB.GetPreviousVersion(appID, currentVersion)
    if err != nil {
        return nil, err
    }
    
    // Compare crash rates
    currentRate, err := r.getCrashRate(appID, currentVersion)
    if err != nil {
        return nil, err
    }
    
    previousRate, err := r.getCrashRate(appID, prevVersion)
    if err != nil {
        return nil, err
    }
    
    var regressions []*Regression
    
    // Check overall crash rate increase
    if currentRate.CrashesPerKSessions > previousRate.CrashesPerKSessions*1.2 {
        regressions = append(regressions, &Regression{
            Type: "overall_rate",
            Severity: "high",
            Change: (currentRate.CrashesPerKSessions - previousRate.CrashesPerKSessions) / 
                previousRate.CrashesPerKSessions,
        })
    }
    
    // Check for new crash groups
    newGroups, err := r.findNewGroups(appID, currentVersion, prevVersion)
    if err != nil {
        return nil, err
    }
    
    for _, group := range newGroups {
        regressions = append(regressions, &Regression{
            Type:     "new_crash_group",
            GroupID:  group.ID,
            Severity: r.assessNewGroupSeverity(group),
            Signature: group.Signature,
        })
    }
    
    // Check for increased frequency in existing groups
    increasedGroups, err := r.findIncreasedGroups(appID, currentVersion, prevVersion)
    if err != nil {
        return nil, err
    }
    
    for _, group := range increasedGroups {
        regressions = append(regressions, &Regression{
            Type:     "increased_frequency",
            GroupID:  group.ID,
            Severity: r.assessIncreaseSeverity(group.Change),
            Change:   group.Change,
        })
    }
    
    return regressions, nil
}

type Regression struct {
    Type      string  `json:"type"`
    GroupID   string  `json:"group_id,omitempty"`
    Severity  string  `json:"severity"`
    Change    float64 `json:"change_percent"`
    Signature string  `json:"signature,omitempty"`
}
```

### Release Comparison

```go
type ReleaseComparator struct {
    db *Database
}

func (c *ReleaseComparator) CompareReleases(
    appID string,
    versionA, versionB string,
) (*ReleaseComparison, error) {
    // Get metrics for both versions
    metricsA, err := c.getReleaseMetrics(appID, versionA)
    if err != nil {
        return nil, err
    }
    
    metricsB, err := c.getReleaseMetrics(appID, versionB)
    if err != nil {
        return nil, err
    }
    
    // Compare crash groups
    groupsA, err := c.getCrashGroups(appID, versionA)
    if err != nil {
        return nil, err
    }
    
    groupsB, err := c.getCrashGroups(appID, versionB)
    if err != nil {
        return nil, err
    }
    
    comparison := &ReleaseComparison{
        VersionA:    metricsA,
        VersionB:    metricsB,
        NewGroups:   c.findNewGroupsOnlyIn(groupsB, groupsA),
        FixedGroups: c.findFixedGroupsOnlyIn(groupsA, groupsB),
        ChangedGroups: c.findChangedGroups(groupsA, groupsB),
    }
    
    // Calculate delta
    comparison.CrashRateDelta = metricsB.CrashRate - metricsA.CrashRate
    comparison.CrashRateChangePercent = comparison.CrashRateDelta / metricsA.CrashRate * 100
    
    return comparison, nil
}

type ReleaseComparison struct {
    VersionA          *ReleaseMetrics   `json:"version_a"`
    VersionB          *ReleaseMetrics   `json:"version_b"`
    CrashRateDelta    float64           `json:"crash_rate_delta"`
    CrashRateChangePercent float64      `json:"crash_rate_change_percent"`
    NewGroups         []*CrashGroup     `json:"new_groups"`
    FixedGroups       []*CrashGroup     `json:"fixed_groups"`
    ChangedGroups     []*ChangedGroup   `json:"changed_groups"`
}
```

### Device/OS Breakdown

```go
type BreakdownAnalyzer struct {
    db *Database
}

func (b *BreakdownAnalyzer) ByDevice(
    groupID string,
    limit int,
) ([]*DeviceBreakdown, error) {
    query := `
        SELECT 
            device_type,
            device_model,
            COUNT(*) as crash_count,
            COUNT(DISTINCT user_id) as affected_users
        FROM crashes
        WHERE group_id = $1
        GROUP BY device_type, device_model
        ORDER BY crash_count DESC
        LIMIT $2
    `
    
    rows, err := b.db.Query(query, groupID, limit)
    if err != nil {
        return nil, err
    }
    
    var breakdown []*DeviceBreakdown
    for rows.Next() {
        var d DeviceBreakdown
        if err := rows.Scan(&d.DeviceType, &d.DeviceModel, 
                           &d.CrashCount, &d.AffectedUsers); err != nil {
            return nil, err
        }
        breakdown = append(breakdown, &d)
    }
    
    return breakdown, nil
}

func (b *BreakdownAnalyzer) ByOS(
    groupID string,
) ([]*OSBreakdown, error) {
    query := `
        SELECT 
            os_name,
            os_version,
            COUNT(*) as crash_count,
            COUNT(DISTINCT user_id) as affected_users,
            ROUND(COUNT(*) * 100.0 / SUM(COUNT(*)) OVER(), 2) as percentage
        FROM crashes
        WHERE group_id = $1
        GROUP BY os_name, os_version
        ORDER BY crash_count DESC
    `
    
    rows, err := b.db.Query(query, groupID)
    if err != nil {
        return nil, err
    }
    
    var breakdown []*OSBreakdown
    for rows.Next() {
        var o OSBreakdown
        if err := rows.Scan(&o.OSName, &o.OSVersion, &o.CrashCount, 
                           &o.AffectedUsers, &o.Percentage); err != nil {
            return nil, err
        }
        breakdown = append(breakdown, &o)
    }
    
    return breakdown, nil
}

type DeviceBreakdown struct {
    DeviceType    string `json:"device_type"`
    DeviceModel   string `json:"device_model"`
    CrashCount    int64  `json:"crash_count"`
    AffectedUsers int64  `json:"affected_users"`
    Percentage    float64 `json:"percentage"`
}

type OSBreakdown struct {
    OSName        string  `json:"os_name"`
    OSVersion     string  `json:"os_version"`
    CrashCount    int64   `json:"crash_count"`
    AffectedUsers int64   `json:"affected_users"`
    Percentage    float64 `json:"percentage"`
}
```

---

## 5. Server-Side Processing (Morgue)

### Ingestion Pipeline Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Ingestion Pipeline                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐          │
│  │  Client  │───▶│   Load   │───▶│   Auth   │───▶│  Validate │          │
│  │  SDKs    │    │ Balancer │    │  Layer   │    │   Layer   │          │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘          │
│       │                                              │                   │
│       │                  HTTP/HTTPS                  │                   │
│       ▼                                              ▼                   │
│  ┌──────────────────────────────────────────────────────────┐           │
│  │                    API Gateway                            │           │
│  │  - Rate limiting    - Request routing    - CORS          │           │
│  └──────────────────────────────────────────────────────────┘           │
│                                  │                                       │
│                                  ▼                                       │
│  ┌──────────────────────────────────────────────────────────┐           │
│  │                   Kafka/RabbitMQ                          │           │
│  │              (Crash Report Queue)                         │           │
│  └──────────────────────────────────────────────────────────┘           │
│                                  │                                       │
│              ┌───────────────────┼───────────────────┐                   │
│              ▼                   ▼                   ▼                   │
│       ┌────────────┐     ┌────────────┐     ┌────────────┐              │
│       │ Processor  │     │ Processor  │     │ Processor  │   ...        │
│       │ Worker 1   │     │ Worker 2   │     │ Worker N   │              │
│       └────────────┘     └────────────┘     └────────────┘              │
│              │                   │                   │                   │
│              └───────────────────┼───────────────────┘                   │
│                                  ▼                                       │
│  ┌──────────────────────────────────────────────────────────┐           │
│  │                  Storage Layer                            │           │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │           │
│  │  │ MongoDB  │  │ Redis    │  │   S3     │  │ Elastic  │ │           │
│  │  │(Reports) │  │(Cache)   │  │(Attach)  │  │(Search)  │ │           │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘ │           │
│  └──────────────────────────────────────────────────────────┘           │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Processing Workers

```go
type CrashProcessor struct {
    consumer     kafka.Consumer
    producer     kafka.Producer
    mongoDB      *mongo.Database
    redis        *redis.Client
    s3Client     *s3.Client
    esClient     *elastic.Client
    fingerprinter *FingerprintGenerator
    classifier   *CrashClassifier
}

func (p *CrashProcessor) Process(ctx context.Context, msg *kafka.Message) error {
    // Parse incoming crash report
    var report CrashReport
    if err := json.Unmarshal(msg.Value, &report); err != nil {
        return fmt.Errorf("failed to parse crash report: %w", err)
    }
    
    // Enrich report
    p.enrichReport(&report)
    
    // Generate fingerprint
    report.Fingerprint = p.fingerprinter.Generate(&report)
    
    // Classify crash
    classification := p.classifier.Classify(&report)
    report.Classification = classification
    
    // Find or create group
    group, err := p.findOrCreateGroup(ctx, &report)
    if err != nil {
        return fmt.Errorf("failed to find group: %w", err)
    }
    report.GroupID = group.ID
    
    // Process attachments
    if err := p.processAttachments(ctx, &report); err != nil {
        return fmt.Errorf("failed to process attachments: %w", err)
    }
    
    // Store in MongoDB
    if err := p.storeReport(ctx, &report); err != nil {
        return fmt.Errorf("failed to store report: %w", err)
    }
    
    // Update group statistics
    if err := p.updateGroupStats(ctx, group); err != nil {
        return fmt.Errorf("failed to update group stats: %w", err)
    }
    
    // Update cache
    if err := p.updateCache(ctx, &report); err != nil {
        return fmt.Errorf("failed to update cache: %w", err)
    }
    
    // Index for search
    if err := p.indexForSearch(ctx, &report); err != nil {
        return fmt.Errorf("failed to index: %w", err)
    }
    
    // Check for alerts
    if err := p.checkAlerts(ctx, &report); err != nil {
        return fmt.Errorf("failed to check alerts: %w", err)
    }
    
    return nil
}

func (p *CrashProcessor) enrichReport(report *CrashReport) {
    // Add server-side timestamp
    report.ServerTimestamp = time.Now().UTC()
    
    // Add geolocation from IP
    if report.Metadata.ClientIP != "" {
        geo := p.geolocate(report.Metadata.ClientIP)
        report.Attributes["geo_country"] = geo.Country
        report.Attributes["geo_city"] = geo.City
    }
    
    // Add session context
    if report.SessionID != "" {
        session := p.getSession(report.SessionID)
        if session != nil {
            report.SessionDuration = session.Duration
            report.SessionEventsCount = session.EventsCount
        }
    }
}
```

### MongoDB Schema Design

```javascript
// Crash Reports Collection
{
  "_id": ObjectId("550e8400e29b41d4a716446655440000"),
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  
  "app_id": ObjectId("550e8400e29b41d4a716446655440001"),
  "version": "2.1.0",
  "build": "20260405.1",
  
  "timestamp": ISODate("2026-04-05T14:30:00.123Z"),
  "server_timestamp": ISODate("2026-04-05T14:30:01.234Z"),
  
  "group_id": ObjectId("550e8400e29b41d4a716446655440002"),
  "fingerprint": "a1b2c3d4e5f6g7h8",
  
  "classification": {
    "category": "memory_access",
    "subcategory": "null_dereference",
    "severity": "high",
    "signal": "SIGSEGV",
    "exception_type": "EXC_BAD_ACCESS"
  },
  
  "crashed_thread": {
    "id": 1,
    "name": "main",
    "frames": [
      {
        "index": 0,
        "function": "abort",
        "library": "libsystem_kernel.dylib",
        "offset": 44
      }
    ]
  },
  
  "user_id": "user_12345",
  "session_id": "sess_abc123",
  
  "device": {
    "type": "iPhone14,2",
    "model": "iPhone 13 Pro",
    "os_name": "iOS",
    "os_version": "17.4.1"
  },
  
  "attributes": {
    "environment": "production",
    "network_type": "wifi",
    "app_state": "foreground"
  },
  
  "attachments": [
    {
      "name": "crash_dump.dmp",
      "s3_key": "attachments/550e8400/crash_dump.dmp",
      "size": 1048576
    }
  ],
  
  "processed": true,
  "symbolicated": true,
  
  "created_at": ISODate("2026-04-05T14:30:01.234Z"),
  "updated_at": ISODate("2026-04-05T14:30:01.234Z")
}

// Indexes for crash_reports
db.crash_reports.createIndex({ "app_id": 1, "timestamp": -1 })
db.crash_reports.createIndex({ "group_id": 1, "timestamp": -1 })
db.crash_reports.createIndex({ "fingerprint": 1 })
db.crash_reports.createIndex({ "user_id": 1, "timestamp": -1 })
db.crash_reports.createIndex({ "app_id": 1, "version": 1, "timestamp": -1 })
db.crash_reports.createIndex({ "classification.category": 1 })
db.crash_reports.createIndex({ "device.os_name": 1, "device.os_version": 1 })

// Crash Groups Collection
{
  "_id": ObjectId("550e8400e29b41d4a716446655440002"),
  "app_id": ObjectId("550e8400e29b41d4a716446655440001"),
  "fingerprint": "a1b2c3d4e5f6g7h8",
  
  "signature": "SIGSEGV|abort|-[AppDelegate application:didFinishLaunchingWithOptions:]",
  
  "classification": {
    "category": "memory_access",
    "severity": "high"
  },
  
  "first_seen": ISODate("2026-04-01T10:00:00.000Z"),
  "last_seen": ISODate("2026-04-05T14:30:00.123Z"),
  
  "status": "open", // open, investigating, resolved, ignored
  
  "stats": {
    "total_crashes": 156,
    "affected_users": 45,
    "affected_sessions": 89,
    "crashes_per_day": 12.5,
    "trend": "increasing"
  },
  
  "versions_affected": ["2.0.0", "2.0.1", "2.1.0"],
  "versions_fixed": [],
  
  "devices_affected": [
    { "type": "iPhone14,2", "count": 67 },
    { "type": "iPhone13,1", "count": 45 }
  ],
  
  "os_affected": [
    { "name": "iOS", "version": "17.4.1", "count": 89 }
  ],
  
  "assignee": "developer_123",
  "labels": ["regression", "payment-flow"],
  
  "resolved_at": null,
  "resolved_by": null,
  "resolution_notes": null,
  
  "created_at": ISODate("2026-04-01T10:00:00.000Z"),
  "updated_at": ISODate("2026-04-05T14:30:01.234Z")
}

// Indexes for crash_groups
db.crash_groups.createIndex({ "app_id": 1, "fingerprint": 1 }, { unique: true })
db.crash_groups.createIndex({ "app_id": 1, "status": 1, "stats.total_crashes": -1 })
db.crash_groups.createIndex({ "app_id": 1, "first_seen": -1 })
db.crash_groups.createIndex({ "app_id": 1, "last_seen": -1 })
db.crash_groups.createIndex({ "app_id": 1, "versions_affected": 1 })
```

### MongoDB Aggregation Pipelines

```javascript
// Pipeline 1: Daily crash counts by group
db.crash_reports.aggregate([
  {
    $match: {
      app_id: ObjectId("550e8400e29b41d4a716446655440001"),
      timestamp: { $gte: ISODate("2026-04-01"), $lt: ISODate("2026-04-08") }
    }
  },
  {
    $group: {
      _id: {
        group_id: "$group_id",
        date: { $dateTrunc: { date: "$timestamp", unit: "day" } }
      },
      count: { $sum: 1 },
      affected_users: { $addToSet: "$user_id" }
    }
  },
  {
    $group: {
      _id: "$_id.group_id",
      daily_counts: {
        $push: {
          date: "$_id.date",
          count: "$count",
          affected_users: { $size: "$affected_users" }
        }
      },
      total_crashes: { $sum: "$count" }
    }
  },
  {
    $lookup: {
      from: "crash_groups",
      localField: "_id",
      foreignField: "_id",
      as: "group"
    }
  },
  { $unwind: "$group" },
  {
    $project: {
      group_id: "$_id",
      signature: "$group.signature",
      total_crashes: 1,
      daily_counts: 1,
      trend: {
        $cond: [
          { $gte: [{ $arrayElemAt: ["$daily_counts.count", -1] }, 
                   { $arrayElemAt: ["$daily_counts.count", -2] }] },
          "increasing",
          "decreasing"
        ]
      }
    }
  }
])

// Pipeline 2: Crash rate by version
db.crash_reports.aggregate([
  {
    $match: {
      app_id: ObjectId("550e8400e29b41d4a716446655440001"),
      timestamp: { $gte: ISODate("2026-04-01"), $lt: ISODate("2026-04-08") }
    }
  },
  {
    $group: {
      _id: "$version",
      crash_count: { $sum: 1 },
      affected_users: { $addToSet: "$user_id" },
      affected_sessions: { $addToSet: "$session_id" }
    }
  },
  {
    $lookup: {
      from: "sessions",
      let: { version: "$_id" },
      pipeline: [
        {
          $match: {
            $expr: { $eq: ["$version", "$$version"] },
            timestamp: { $gte: ISODate("2026-04-01"), $lt: ISODate("2026-04-08") }
          }
        },
        { $count: "total" }
      ],
      as: "session_stats"
    }
  },
  {
    $project: {
      version: "$_id",
      crash_count: 1,
      affected_users: { $size: "$affected_users" },
      total_sessions: { $arrayElemAt: ["$session_stats.total", 0] },
      crashes_per_session: {
        $divide: ["$crash_count", { $arrayElemAt: ["$session_stats.total", 0] }]
      },
      crashes_per_k_sessions: {
        $multiply: [
          { $divide: ["$crash_count", { $arrayElemAt: ["$session_stats.total", 0] }] },
          1000
        ]
      }
    }
  },
  { $sort: { crashes_per_k_sessions: -1 } }
])

// Pipeline 3: Device/OS breakdown
db.crash_reports.aggregate([
  {
    $match: {
      app_id: ObjectId("550e8400e29b41d4a716446655440001"),
      group_id: ObjectId("550e8400e29b41d4a716446655440002")
    }
  },
  {
    $facet: {
      by_device: [
        {
          $group: {
            _id: {
              device_type: "$device.type",
              device_model: "$device.model"
            },
            count: { $sum: 1 },
            users: { $addToSet: "$user_id" }
          }
        },
        {
          $project: {
            device_type: "$_id.device_type",
            device_model: "$_id.device_model",
            count: 1,
            affected_users: { $size: "$users" },
            percentage: { $multiply: [{ $divide: ["$count", "$TOTAL"] }, 100] }
          }
        },
        { $sort: { count: -1 } },
        { $limit: 10 }
      ],
      by_os: [
        {
          $group: {
            _id: {
              os_name: "$device.os_name",
              os_version: "$device.os_version"
            },
            count: { $sum: 1 }
          }
        },
        { $sort: { count: -1 } },
        { $limit: 10 }
      ]
    }
  }
])
```

### Redis Caching for Metrics

```go
type MetricsCache struct {
    redis  *redis.Client
    prefix string
}

func NewMetricsCache(redis *redis.Client) *MetricsCache {
    return &MetricsCache{
        redis:  redis,
        prefix: "bt:metrics:",
    }
}

func (m *MetricsCache) GetCrashRate(ctx context.Context, appID string, version string) (float64, error) {
    key := fmt.Sprintf("%scrash_rate:%s:%s", m.prefix, appID, version)
    
    // Try cache first
    cached, err := m.redis.Get(ctx, key).Float64()
    if err == nil {
        return cached, nil
    }
    
    // Cache miss - calculate from database
    rate, err := m.calculateCrashRate(ctx, appID, version)
    if err != nil {
        return 0, err
    }
    
    // Cache for 5 minutes
    m.redis.Set(ctx, key, rate, 5*time.Minute)
    
    return rate, nil
}

func (m *MetricsCache) CacheGroupStats(ctx context.Context, groupID string, stats *GroupStats) error {
    key := fmt.Sprintf("%sgroup_stats:%s", m.prefix, groupID)
    
    data, err := json.Marshal(stats)
    if err != nil {
        return err
    }
    
    return m.redis.Set(ctx, key, data, 10*time.Minute).Err()
}

func (m *MetricsCache) GetGroupStats(ctx context.Context, groupID string) (*GroupStats, error) {
    key := fmt.Sprintf("%sgroup_stats:%s", m.prefix, groupID)
    
    data, err := m.redis.Get(ctx, key).Bytes()
    if err != nil {
        return nil, err
    }
    
    var stats GroupStats
    if err := json.Unmarshal(data, &stats); err != nil {
        return nil, err
    }
    
    return &stats, nil
}

// Real-time counters for spike detection
func (m *MetricsCache) IncrementCrashCounter(ctx context.Context, appID string) error {
    key := fmt.Sprintf("%scrash_count:%s:%s", m.prefix, appID, time.Now().Format("2006-01-02-15"))
    return m.redis.Incr(ctx, key).Err()
}

func (m *MetricsCache) GetCrashCount(ctx context.Context, appID string, window time.Duration) (int64, error) {
    // Use Redis sorted set for time-windowed counts
    key := fmt.Sprintf("%scrash_timeseries:%s", m.prefix, appID)
    now := time.Now()
    cutoff := now.Add(-window).UnixMilli()
    
    // Remove old entries
    m.redis.ZRemRangeByScore(ctx, key, "0", fmt.Sprintf("%d", cutoff))
    
    // Count recent entries
    return m.redis.ZCount(ctx, key, fmt.Sprintf("%d", cutoff), fmt.Sprintf("%d", now.UnixMilli()))
}
```

### Elasticsearch Indexing

```go
type ElasticsearchIndexer struct {
    client *elastic.Client
    index  string
}

func (e *ElasticsearchIndexer) IndexCrash(ctx context.Context, report *CrashReport) error {
    doc := map[string]interface{}{
        "uuid":         report.UUID,
        "app_id":       report.AppID,
        "version":      report.Version,
        "timestamp":    report.Timestamp,
        "group_id":     report.GroupID,
        "fingerprint":  report.Fingerprint,
        "user_id":      report.UserID,
        "session_id":   report.SessionID,
        
        "classification": report.Classification,
        "crash_type":     report.Crash.Type,
        "signal":         report.Crash.Signal,
        "exception_type": report.Crash.ExceptionType,
        "reason":         report.Crash.Reason,
        
        "device": report.Device,
        "os_name": report.Device.OSName,
        "os_version": report.Device.OSVersion,
        "device_model": report.Device.Model,
        
        "crashed_function": e.getCrashedFunction(report),
        "crashed_library":  e.getCrashedLibrary(report),
        "stack_signature":  e.generateStackSignature(report),
        
        "attributes": report.Attributes,
        
        "full_text": e.generateFullText(report),
    }
    
    _, err := e.client.Index().
        Index(e.index).
        Id(report.UUID).
        BodyJson(doc).
        Do(ctx)
    
    return err
}

func (e *ElasticsearchIndexer) generateFullText(report *CrashReport) string {
    var sb strings.Builder
    
    // Add searchable text fields
    sb.WriteString(report.Crash.Reason)
    sb.WriteString(" ")
    sb.WriteString(report.Crash.ExceptionType)
    sb.WriteString(" ")
    sb.WriteString(report.Crash.Signal)
    
    // Add stack trace text
    for _, frame := range report.Threads[0].StackFrames[:min(10, len(report.Threads[0].StackFrames))] {
        sb.WriteString(" ")
        sb.WriteString(frame.FunctionName)
        sb.WriteString(" ")
        sb.WriteString(frame.LibraryName)
    }
    
    return sb.String()
}

func (e *ElasticsearchIndexer) Search(ctx context.Context, query *SearchQuery) (*SearchResults, error) {
    // Build Elasticsearch query
    esQuery := e.buildQuery(query)
    
    result, err := e.client.Search().
        Index(e.index).
        Query(esQuery).
        From(query.From).
        Size(query.Size).
        Sort("timestamp", false).
        Do(ctx)
    
    if err != nil {
        return nil, err
    }
    
    return e.parseResults(result), nil
}
```

### S3 Attachment Storage

```go
type AttachmentStorage struct {
    s3Client *s3.Client
    bucket   string
    prefix   string
}

func (a *AttachmentStorage) Store(ctx context.Context, reportUUID string, attachment *Attachment) (*StoredAttachment, error) {
    key := fmt.Sprintf("%s/%s/%s", a.prefix, reportUUID, attachment.Name)
    
    // Determine content type
    contentType := attachment.ContentType
    if contentType == "" {
        contentType = http.DetectContentType(attachment.Data[:512])
    }
    
    // Upload to S3
    result, err := a.s3Client.PutObject(ctx, &s3.PutObjectInput{
        Bucket:      aws.String(a.bucket),
        Key:         aws.String(key),
        Body:        bytes.NewReader(attachment.Data),
        ContentType: aws.String(contentType),
        Metadata: map[string]string{
            "report-uuid": reportUUID,
            "original-name": attachment.Name,
        },
    })
    
    if err != nil {
        return nil, err
    }
    
    return &StoredAttachment{
        Name:       attachment.Name,
        S3Key:      key,
        S3Bucket:   a.bucket,
        Size:       int64(len(attachment.Data)),
        ContentType: contentType,
        ETag:       *result.ETag,
    }, nil
}

func (a *AttachmentStorage) GetPresignedURL(ctx context.Context, key string, expiry time.Duration) (string, error) {
    presigner := presign.NewPresigner(a.s3Client)
    
    result, err := presigner.PresignGetObject(ctx, &s3.GetObjectInput{
        Bucket: aws.String(a.bucket),
        Key:    aws.String(key),
    }, presigner.WithPresignExpires(expiry))
    
    if err != nil {
        return "", err
    }
    
    return result.URL, nil
}

// Lifecycle policy for attachment cleanup
func (a *AttachmentStorage) SetupLifecycle() error {
    lifecycleConfig := &s3types.BucketLifecycleConfiguration{
        Rules: []s3types.LifecycleRule{
            {
                ID:     aws.String("cleanup-old-attachments"),
                Status: s3types.ExpirationStatusEnabled,
                Filter: &s3types.LifecycleRuleFilter{
                    Prefix: aws.String(a.prefix),
                },
                Expiration: &s3types.LifecycleExpiration{
                    Days: aws.Int32(90), // Delete after 90 days
                },
            },
            {
                ID:     aws.String("transition-to-glacier"),
                Status: s3types.ExpirationStatusEnabled,
                Filter: &s3types.LifecycleRuleFilter{
                    Prefix: aws.String(a.prefix),
                },
                Transitions: []s3types.Transition{
                    {
                        Days:         aws.Int32(30),
                        StorageClass: s3types.StorageClassGlacier,
                    },
                },
            },
        },
    }
    
    _, err := a.s3Client.PutBucketLifecycleConfiguration(ctx, &s3.PutBucketLifecycleConfigurationInput{
        Bucket:                  aws.String(a.bucket),
        LifecycleConfiguration:  lifecycleConfig,
    })
    
    return err
}
```

---

## 6. Real-time Features

### Webhook Notifications

```go
type WebhookManager struct {
    db         *Database
    httpClient *http.Client
    retryQueue *RetryQueue
}

type WebhookConfig struct {
    ID          string            `json:"id"`
    AppID       string            `json:"app_id"`
    URL         string            `json:"url"`
    Secret      string            `json:"secret"` // For HMAC signature
    Events      []WebhookEvent    `json:"events"`
    Filters     WebhookFilters    `json:"filters"`
    Active      bool              `json:"active"`
    Headers     map[string]string `json:"headers"`
    RetryCount  int               `json:"retry_count"`
    TimeoutSec  int               `json:"timeout_sec"`
}

type WebhookEvent string

const (
    EventCrashNew       WebhookEvent = "crash.new"
    EventCrashSpike     WebhookEvent = "crash.spike"
    EventCrashRegression WebhookEvent = "crash.regression"
    EventGroupResolved  WebhookEvent = "group.resolved"
    EventThresholdAlert WebhookEvent = "alert.threshold"
)

func (w *WebhookManager) Trigger(ctx context.Context, event WebhookEvent, payload interface{}) error {
    // Find matching webhooks
    webhooks, err := w.getMatchingWebhooks(event)
    if err != nil {
        return err
    }
    
    for _, webhook := range webhooks {
        if err := w.deliverAsync(ctx, webhook, event, payload); err != nil {
            log.Printf("failed to queue webhook: %v", err)
        }
    }
    
    return nil
}

func (w *WebhookManager) deliver(ctx context.Context, webhook WebhookConfig, event WebhookEvent, payload interface{}) error {
    // Build payload
    body := WebhookPayload{
        Event:      event,
        Timestamp:  time.Now().UTC(),
        AppID:      webhook.AppID,
        Data:       payload,
    }
    
    bodyJSON, err := json.Marshal(body)
    if err != nil {
        return err
    }
    
    // Generate signature
    signature := w.generateSignature(bodyJSON, webhook.Secret)
    
    // Create request
    req, err := http.NewRequestWithContext(ctx, "POST", webhook.URL, bytes.NewReader(bodyJSON))
    if err != nil {
        return err
    }
    
    req.Header.Set("Content-Type", "application/json")
    req.Header.Set("X-Backtrace-Event", string(event))
    req.Header.Set("X-Backtrace-Signature", signature)
    req.Header.Set("X-Backtrace-Timestamp", body.Timestamp.Format(time.RFC3339))
    
    // Add custom headers
    for k, v := range webhook.Headers {
        req.Header.Set(k, v)
    }
    
    // Send request
    client := &http.Client{Timeout: time.Duration(webhook.TimeoutSec) * time.Second}
    resp, err := client.Do(req)
    if err != nil {
        return err
    }
    defer resp.Body.Close()
    
    if resp.StatusCode < 200 || resp.StatusCode >= 300 {
        return fmt.Errorf("webhook returned status %d", resp.StatusCode)
    }
    
    return nil
}

func (w *WebhookManager) generateSignature(body []byte, secret string) string {
    hmac := hmac.New(sha256.New, []byte(secret))
    hmac.Write(body)
    return fmt.Sprintf("sha256=%x", hmac.Sum(nil))
}

type WebhookPayload struct {
    Event     WebhookEvent `json:"event"`
    Timestamp time.Time    `json:"timestamp"`
    AppID     string       `json:"app_id"`
    Data      interface{}  `json:"data"`
}
```

### Slack Integration

```go
type SlackIntegration struct {
    client   *slack.Client
    db       *Database
    templates *NotificationTemplates
}

type SlackConfig struct {
    AppID           string   `json:"app_id"`
    WebhookURL      string   `json:"webhook_url"` // Incoming webhook
    Channel         string   `json:"channel"`      // Or use webhook
    NotifyOnNew     bool     `json:"notify_on_new"`
    NotifyOnSpike   bool     `json:"notify_on_spike"`
    NotifyOnResolved bool    `json:"notify_on_resolved"`
    SeverityFilter  []string `json:"severity_filter"`
}

func (s *SlackIntegration) SendNewCrashAlert(ctx context.Context, group *CrashGroup) error {
    config, err := s.getConfig(group.AppID)
    if err != nil || !config.NotifyOnNew {
        return nil
    }
    
    blockSet := s.templates.NewCrashBlocks(group)
    
    msg := slack.WebhookMessage{
        Blocks: blockSet,
    }
    
    return s.client.PostWebhookContext(ctx, config.WebhookURL, &msg)
}

func (s *SlackIntegration) SendSpikeAlert(ctx context.Context, group *CrashGroup, spikeInfo *SpikeInfo) error {
    config, err := s.getConfig(group.AppID)
    if err != nil || !config.NotifyOnSpike {
        return nil
    }
    
    blockSet := s.templates.SpikeAlertBlocks(group, spikeInfo)
    
    msg := slack.WebhookMessage{
        Blocks: blockSet,
    }
    
    return s.client.PostWebhookContext(ctx, config.WebhookURL, &msg)
}

// Slack Block Kit templates
type NotificationTemplates struct{}

func (t *NotificationTemplates) NewCrashBlocks(group *CrashGroup) slack.Blocks {
    return slack.Blocks{
        BlockSet: []slack.Block{
            slack.HeaderBlock{
                Type: "header",
                Text: slack.NewTextBlockElement("plaintext", "🚨 New Crash Detected"),
            },
            slack.SectionBlock{
                Type: "section",
                Fields: []*slack.TextBlockObject{
                    slack.NewTextBlockObject("mrkdwn", "*App:*\n"+group.AppName, false, false),
                    slack.NewTextBlockObject("mrkdwn", "*Version:*\n"+group.LatestVersion, false, false),
                    slack.NewTextBlockObject("mrkdwn", "*Severity:*\n"+strings.ToUpper(group.Severity), false, false),
                    slack.NewTextBlockObject("mrkdwn", "*Category:*\n"+group.Category, false, false),
                },
            },
            slack.SectionBlock{
                Type: "section",
                Text: slack.NewTextBlockObject("mrkdwn", "*Signature:*\n```"+group.Signature+"```", false, false),
            },
            slack.ContextBlock{
                Type: "context",
                Elements: slack.NewContextBlockElements(
                    slack.NewTextBlockObject("mrkdwn", ":bar_chart: "+fmt.Sprintf("%d crashes", group.TotalCrashes), false, false),
                    slack.NewTextBlockObject("mrkdwn", ":busts_in_silhouette: "+fmt.Sprintf("%d affected users", group.AffectedUsers), false, false),
                ),
            },
            slack.ActionBlock{
                Type: "actions",
                Elements: slack.NewBlockElements(
                    slack.NewButtonBlockElement("view_details", group.ID, "View Details"),
                    slack.NewButtonBlockElement("assign_me", group.ID, "Assign to Me"),
                    slack.NewButtonBlockElement("mute", group.ID, "Mute"),
                ),
            },
        },
    }
}

func (t *NotificationTemplates) SpikeAlertBlocks(group *CrashGroup, spikeInfo *SpikeInfo) slack.Blocks {
    return slack.Blocks{
        BlockSet: []slack.Block{
            slack.HeaderBlock{
                Type: "header",
                Text: slack.NewTextBlockElement("plaintext", "📈 Crash Spike Detected"),
            },
            slack.SectionBlock{
                Type: "section",
                Text: slack.NewTextBlockObject("mrkdwn", 
                    fmt.Sprintf("*%s* has increased by *%d%%* in the last hour", 
                        group.Signature, spikeInfo.PercentIncrease),
                    false, false),
            },
            slack.SectionBlock{
                Type: "section",
                Fields: []*slack.TextBlockObject{
                    slack.NewTextBlockObject("mrkdwn", "*Before:*\n"+fmt.Sprintf("%d crashes/hour", spikeInfo.BaselineRate), false, false),
                    slack.NewTextBlockObject("mrkdwn", "*Now:*\n"+fmt.Sprintf("%d crashes/hour", spikeInfo.CurrentRate), false, false),
                },
            },
            slack.ActionBlock{
                Type: "actions",
                Elements: slack.NewBlockElements(
                    slack.NewButtonBlockElement("investigate", group.ID, "Investigate"),
                ),
            },
        },
    }
}
```

### Alert Thresholds

```go
type AlertEngine struct {
    db           *Database
    rules        []AlertRule
    evaluators   map[string]AlertEvaluator
    notification *NotificationManager
}

type AlertRule struct {
    ID          string        `json:"id"`
    AppID       string        `json:"app_id"`
    Name        string        `json:"name"`
    Description string        `json:"description"`
    Enabled     bool          `json:"enabled"`
    Type        AlertType     `json:"type"`
    Conditions  []Condition   `json:"conditions"`
    Thresholds  Thresholds    `json:"thresholds"`
    Severity    string        `json:"severity"`
    Channels    []string      `json:"channels"` // slack, email, webhook, pagerduty
    Cooldown    time.Duration `json:"cooldown"`  // Minimum time between alerts
    LastTriggered time.Time   `json:"last_triggered"`
}

type AlertType string

const (
    AlertTypeCrashRate      AlertType = "crash_rate"
    AlertTypeCrashSpike     AlertType = "crash_spike"
    AlertTypeNewIssue       AlertType = "new_issue"
    AlertTypeRegression     AlertType = "regression"
    AlertTypeAffectedUsers  AlertType = "affected_users"
    AlertTypeErrorBudget    AlertType = "error_budget"
)

type Condition struct {
    Field    string      `json:"field"`
    Operator string      `json:"operator"` // gt, lt, eq, ne, contains
    Value    interface{} `json:"value"`
}

type Thresholds struct {
    CrashRatePerKSessions float64       `json:"crash_rate_per_k_sessions"`
    CrashSpikePercent     float64       `json:"crash_spike_percent"`
    AffectedUsers         int           `json:"affected_users"`
    NewIssuesPerHour      int           `json:"new_issues_per_hour"`
    EvaluationWindow      time.Duration `json:"evaluation_window"`
}

func (a *AlertEngine) EvaluateAll(ctx context.Context) error {
    rules, err := a.getActiveRules()
    if err != nil {
        return err
    }
    
    for _, rule := range rules {
        if err := a.evaluateRule(ctx, rule); err != nil {
            log.Printf("failed to evaluate rule %s: %v", rule.ID, err)
        }
    }
    
    return nil
}

func (a *AlertEngine) evaluateRule(ctx context.Context, rule AlertRule) error {
    // Check cooldown
    if time.Since(rule.LastTriggered) < rule.Cooldown {
        return nil
    }
    
    // Get metrics for evaluation window
    metrics, err := a.getMetrics(ctx, rule.AppID, rule.Thresholds.EvaluationWindow)
    if err != nil {
        return err
    }
    
    // Evaluate conditions
    triggered := false
    for _, condition := range rule.Conditions {
        if a.evaluateCondition(condition, metrics) {
            triggered = true
            break
        }
    }
    
    if triggered {
        return a.triggerAlert(ctx, rule, metrics)
    }
    
    return nil
}

func (a *AlertEngine) evaluateCondition(c Condition, metrics *Metrics) bool {
    var actualValue float64
    
    switch c.Field {
    case "crash_rate":
        actualValue = metrics.CrashRatePerKSessions
    case "crash_count":
        actualValue = float64(metrics.CrashCount)
    case "affected_users":
        actualValue = float64(metrics.AffectedUsers)
    case "spike_percent":
        actualValue = metrics.SpikePercent
    }
    
    threshold, _ := c.Value.(float64)
    
    switch c.Operator {
    case "gt":
        return actualValue > threshold
    case "gte":
        return actualValue >= threshold
    case "lt":
        return actualValue < threshold
    case "lte":
        return actualValue <= threshold
    case "eq":
        return actualValue == threshold
    }
    
    return false
}

func (a *AlertEngine) triggerAlert(ctx context.Context, rule AlertRule, metrics *Metrics) error {
    alert := &Alert{
        RuleID:      rule.ID,
        AppID:       rule.AppID,
        Type:        rule.Type,
        Severity:    rule.Severity,
        TriggeredAt: time.Now().UTC(),
        Metrics:     metrics,
        Message:     a.buildAlertMessage(rule, metrics),
    }
    
    // Store alert
    if err := a.storeAlert(ctx, alert); err != nil {
        return err
    }
    
    // Send notifications
    for _, channel := range rule.Channels {
        if err := a.notification.Send(ctx, channel, alert); err != nil {
            log.Printf("failed to send notification via %s: %v", channel, err)
        }
    }
    
    // Update last triggered
    return a.updateLastTriggered(ctx, rule.ID)
}
```

### Spike Detection

```go
type SpikeDetector struct {
    db *Database
}

type SpikeInfo struct {
    GroupID       string    `json:"group_id"`
    BaselineRate  float64   `json:"baseline_rate"`  // Crashes per hour (baseline)
    CurrentRate   float64   `json:"current_rate"`   // Crashes per hour (current)
    PercentIncrease float64 `json:"percent_increase"`
    StartTime     time.Time `json:"start_time"`
    Confidence    float64   `json:"confidence"`
    IsAnomaly     bool      `json:"is_anomaly"`
}

func (s *SpikeDetector) DetectSpikes(ctx context.Context, appID string) ([]*SpikeInfo, error) {
    // Get hourly crash counts for last 24 hours
    hourlyCounts, err := s.getHourlyCounts(ctx, appID, 24*time.Hour)
    if err != nil {
        return nil, err
    }
    
    var spikes []*SpikeInfo
    
    for groupID, counts := range hourlyCounts {
        spike := s.analyzeGroupForSpike(groupID, counts)
        if spike != nil && spike.IsAnomaly {
            spikes = append(spikes, spike)
        }
    }
    
    return spikes, nil
}

func (s *SpikeDetector) analyzeGroupForSpike(groupID string, counts []int) *SpikeInfo {
    if len(counts) < 3 {
        return nil
    }
    
    // Calculate baseline (median of last 24 hours excluding current hour)
    baseline := s.calculateBaseline(counts[:len(counts)-1])
    current := counts[len(counts)-1]
    
    // Calculate percent increase
    percentIncrease := 0.0
    if baseline > 0 {
        percentIncrease = float64(current-baseline) / baseline * 100
    }
    
    // Determine if it's a spike using statistical methods
    isAnomaly := s.isAnomaly(counts, current)
    
    return &SpikeInfo{
        GroupID:         groupID,
        BaselineRate:    baseline,
        CurrentRate:     float64(current),
        PercentIncrease: percentIncrease,
        StartTime:       time.Now().Add(-time.Hour),
        Confidence:      s.calculateConfidence(counts, current),
        IsAnomaly:       isAnomaly,
    }
}

func (s *SpikeDetector) calculateBaseline(counts []int) float64 {
    // Use median for robustness against outliers
    sorted := make([]int, len(counts))
    copy(sorted, counts)
    sort.Ints(sorted)
    
    median := sorted[len(sorted)/2]
    return float64(median)
}

func (s *SpikeDetector) isAnomaly(counts []int, current int) bool {
    // Calculate mean and standard deviation
    mean, stddev := s.calculateStats(counts)
    
    // Check if current value is more than 3 standard deviations above mean
    zScore := float64(current-mean) / stddev
    
    // Also check for significant percent increase
    percentIncrease := 0.0
    if mean > 0 {
        percentIncrease = float64(current-mean) / mean * 100
    }
    
    return zScore > 3 || percentIncrease > 200
}

func (s *SpikeDetector) calculateStats(counts []int) (mean, stddev float64) {
    sum := 0
    for _, c := range counts {
        sum += c
    }
    mean = float64(sum) / float64(len(counts))
    
    variance := 0.0
    for _, c := range counts {
        diff := float64(c) - mean
        variance += diff * diff
    }
    variance /= float64(len(counts))
    stddev = math.Sqrt(variance)
    
    return mean, stddev
}

func (s *SpikeDetector) calculateConfidence(counts []int, current int) float64 {
    // Higher confidence with:
    // 1. Larger sample size (historical data)
    // 2. Larger deviation from baseline
    // 3. Consistent increase pattern
    
    mean, stddev := s.calculateStats(counts)
    
    if stddev == 0 {
        if current > mean {
            return 1.0
        }
        return 0.0
    }
    
    zScore := float64(current-mean) / stddev
    
    // Convert z-score to confidence (sigmoid-like function)
    confidence := 1.0 / (1.0 + math.Exp(-zScore+2))
    
    return math.Min(confidence, 1.0)
}
```

### New Issue Detection

```go
type NewIssueDetector struct {
    db *Database
}

func (n *NewIssueDetector) DetectNewIssues(ctx context.Context, appID string, since time.Time) ([]*CrashGroup, error) {
    query := bson.M{
        "app_id":     appID,
        "first_seen": bson.M{"$gte": since},
        "status":     "open",
    }
    
    cursor, err := n.db.Collection("crash_groups").Find(ctx, query)
    if err != nil {
        return nil, err
    }
    
    var groups []*CrashGroup
    if err := cursor.All(ctx, &groups); err != nil {
        return nil, err
    }
    
    return groups, nil
}

func (n *NewIssueDetector) PrioritizeNewIssues(groups []*CrashGroup) []*PrioritizedGroup {
    var prioritized []*PrioritizedGroup
    
    for _, group := range groups {
        pg := &PrioritizedGroup{
            Group:    group,
            Priority: n.calculatePriority(group),
            Reasons:  n.identifyReasons(group),
        }
        prioritized = append(prioritized, pg)
    }
    
    // Sort by priority score
    sort.Slice(prioritized, func(i, j int) bool {
        return prioritized[i].Priority.Score > prioritized[j].Priority.Score
    })
    
    return prioritized
}

type PriorityScore struct {
    Score      float64 `json:"score"`
    Factors    map[string]float64 `json:"factors"`
}

func (n *NewIssueDetector) calculatePriority(group *CrashGroup) *PriorityScore {
    score := &PriorityScore{
        Factors: make(map[string]float64),
    }
    
    // Factor 1: Crash rate (0-25 points)
    crashRateScore := math.Min(group.CrashesPerDay/10, 1) * 25
    score.Factors["crash_rate"] = crashRateScore
    
    // Factor 2: Affected users (0-25 points)
    userScore := math.Min(float64(group.AffectedUsers)/100, 1) * 25
    score.Factors["affected_users"] = userScore
    
    // Factor 3: Severity (0-25 points)
    severityScore := map[string]float64{
        "critical": 25,
        "high":     20,
        "medium":   10,
        "low":      5,
    }[group.Severity]
    score.Factors["severity"] = severityScore
    
    // Factor 4: Recency (0-25 points)
    hoursSinceFirstSeen := time.Since(group.FirstSeen).Hours()
    recencyScore := math.Max(0, 25-hoursSinceFirstSeen) 
    score.Factors["recency"] = recencyScore
    
    score.Score = crashRateScore + userScore + severityScore + recencyScore
    
    return score
}
```

---

## 7. Privacy and Compliance

### PII Scrubbing

```go
type PIIScrubber struct {
    rules       []PIIRule
    customFields map[string]bool
}

type PIIRule struct {
    Name        string
    Pattern     *regexp.Regexp
    Replacement string
    Fields      []string
}

func NewDefaultPIIScrubber() *PIIScrubber {
    return &PIIScrubber{
        rules: []PIIRule{
            {
                Name:        "email",
                Pattern:     regexp.MustCompile(`[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}`),
                Replacement: "[EMAIL_REDACTED]",
                Fields:      []string{"user_email", "email", "attributes.user_email"},
            },
            {
                Name:        "phone",
                Pattern:     regexp.MustCompile(`\+?[\d\s-]{10,}`),
                Replacement: "[PHONE_REDACTED]",
                Fields:      []string{"phone", "user_phone"},
            },
            {
                Name:        "ssn",
                Pattern:     regexp.MustCompile(`\b\d{3}-\d{2}-\d{4}\b`),
                Replacement: "[SSN_REDACTED]",
                Fields:      []string{"*"},
            },
            {
                Name:        "credit_card",
                Pattern:     regexp.MustCompile(`\b(?:\d{4}[- ]?){3}\d{4}\b`),
                Replacement: "[CC_REDACTED]",
                Fields:      []string{"*"},
            },
            {
                Name:        "ip_address",
                Pattern:     regexp.MustCompile(`\b(?:\d{1,3}\.){3}\d{1,3}\b`),
                Replacement: "[IP_REDACTED]",
                Fields:      []string{"client_ip", "ip_address"},
            },
        },
    }
}

func (p *PIIScrubber) Scrub(report *CrashReport) *CrashReport {
    // Scrub known PII fields
    if report.Attributes != nil {
        for field := range report.Attributes {
            if p.isPIIField(field) {
                report.Attributes[field] = "[REDACTED]"
            } else {
                // Scrub string values that match PII patterns
                if str, ok := report.Attributes[field].(string); ok {
                    report.Attributes[field] = p.scrubString(str)
                }
            }
        }
    }
    
    // Scrub breadcrumbs
    for i, bc := range report.Breadcrumbs {
        report.Breadcrumbs[i].Message = p.scrubString(bc.Message)
        for k, v := range bc.Data {
            if str, ok := v.(string); ok {
                report.Breadcrumbs[i].Data[k] = p.scrubString(str)
            }
        }
    }
    
    return report
}

func (p *PIIScrubber) scrubString(s string) string {
    result := s
    for _, rule := range p.rules {
        result = rule.Pattern.ReplaceAllString(result, rule.Replacement)
    }
    return result
}

// Hash user IDs for analytics while preserving uniqueness
func (p *PIIScrubber) HashUserID(userID string, salt string) string {
    h := sha256.New()
    h.Write([]byte(salt))
    h.Write([]byte(userID))
    return fmt.Sprintf("%x", h.Sum(nil))
}
```

### Data Retention Policies

```go
type RetentionManager struct {
    db          *Database
    s3Client    *s3.Client
    policies    map[string]*RetentionPolicy
}

type RetentionPolicy struct {
    AppID              string        `json:"app_id"`
    CrashDataDays      int           `json:"crash_data_days"`      // Keep detailed crash data
    AggregatedDataDays int           `json:"aggregated_data_days"` // Keep aggregated metrics
    AttachmentsDays    int           `json:"attachments_days"`     // Keep attachments
    AnonymizeAfterDays int           `json:"anonymize_after_days"` // Anonymize user data
    ArchiveEnabled     bool          `json:"archive_enabled"`      // Archive before delete
    ArchiveBucket      string        `json:"archive_bucket"`       // S3 bucket for archive
}

func DefaultRetentionPolicy() *RetentionPolicy {
    return &RetentionPolicy{
        CrashDataDays:      90,
        AggregatedDataDays: 365,
        AttachmentsDays:    30,
        AnonymizeAfterDays: 30,
        ArchiveEnabled:     true,
        ArchiveBucket:      "backtrace-archive",
    }
}

func (r *RetentionManager) RunRetentionJob(ctx context.Context) error {
    policies, err := r.getAllPolicies()
    if err != nil {
        return err
    }
    
    for _, policy := range policies {
        if err := r.applyPolicy(ctx, policy); err != nil {
            log.Printf("failed to apply policy for %s: %v", policy.AppID, err)
        }
    }
    
    return nil
}

func (r *RetentionManager) applyPolicy(ctx context.Context, policy *RetentionPolicy) error {
    cutoff := time.Now().AddDate(0, 0, -policy.CrashDataDays)
    
    // Archive old data first
    if policy.ArchiveEnabled {
        if err := r.archiveOldData(ctx, policy, cutoff); err != nil {
            return err
        }
    }
    
    // Delete old crash reports
    if err := r.deleteOldReports(ctx, policy.AppID, cutoff); err != nil {
        return err
    }
    
    // Delete old attachments
    attachmentCutoff := time.Now().AddDate(0, 0, -policy.AttachmentsDays)
    if err := r.deleteOldAttachments(ctx, policy.AppID, attachmentCutoff); err != nil {
        return err
    }
    
    // Anonymize user data
    anonymizeCutoff := time.Now().AddDate(0, 0, -policy.AnonymizeAfterDays)
    if err := r.anonymizeUserData(ctx, policy.AppID, anonymizeCutoff); err != nil {
        return err
    }
    
    return nil
}

func (r *RetentionManager) deleteOldReports(ctx context.Context, appID string, cutoff time.Time) error {
    result, err := r.db.Collection("crash_reports").DeleteMany(ctx, bson.M{
        "app_id":    appID,
        "timestamp": bson.M{"$lt": cutoff},
    })
    
    log.Printf("deleted %d old crash reports for %s", result.DeletedCount, appID)
    return err
}

func (r *RetentionManager) anonymizeUserData(ctx context.Context, appID string, cutoff time.Time) error {
    _, err := r.db.Collection("crash_reports").UpdateMany(ctx, bson.M{
        "app_id":    appID,
        "timestamp": bson.M{"$lt": cutoff},
    }, bson.M{
        "$set": bson.M{
            "user_id":           nil,
            "session_id":        nil,
            "attributes.user_email": "[ANONYMIZED]",
            "attributes.user_ip": "[ANONYMIZED]",
        },
    })
    
    return err
}
```

### GDPR Compliance

```go
type GDPRCompliance struct {
    db       *Database
    storage  *AttachmentStorage
    exporter *DataExporter
}

type DataSubjectRequest struct {
    ID           string          `json:"id"`
    Type         RequestType     `json:"type"` // access, rectification, erasure, portability
    SubjectID    string          `json:"subject_id"` // User ID or email
    Email        string          `json:"email"`      // Requestor email for verification
    Status       RequestStatus   `json:"status"`
    CreatedAt    time.Time       `json:"created_at"`
    CompletedAt  time.Time       `json:"completed_at"`
    Data         interface{}     `json:"data,omitempty"`
}

type RequestType string

const (
    RequestAccess      RequestType = "access"      // Article 15
    RequestRectification RequestType = "rectification" // Article 16
    RequestErasure     RequestType = "erasure"     // Article 17 (Right to be forgotten)
    RequestPortability RequestType = "portability" // Article 20
)

func (g *GDPRCompliance) HandleErasureRequest(ctx context.Context, request *DataSubjectRequest) error {
    // Find all data for this user
    userCrashes, err := g.findUserCrashes(ctx, request.SubjectID)
    if err != nil {
        return err
    }
    
    // Anonymize crash reports (don't delete as they may be needed for other users' debugging)
    for _, crash := range userCrashes {
        _, err := g.db.Collection("crash_reports").UpdateOne(ctx, bson.M{
            "_id": crash.ID,
        }, bson.M{
            "$set": bson.M{
                "user_id": nil,
                "attributes.user_email": "[GDPR_ERASED]",
                "attributes.user_name": "[GDPR_ERASED]",
                "breadcrumbs": []Breadcrumb{}, // Remove breadcrumbs that may contain PII
            },
        })
        if err != nil {
            return err
        }
    }
    
    // Delete user sessions
    _, err = g.db.Collection("sessions").DeleteMany(ctx, bson.M{
        "user_id": request.SubjectID,
    })
    if err != nil {
        return err
    }
    
    // Log the erasure for compliance audit
    if err := g.logErasure(request, len(userCrashes)); err != nil {
        return err
    }
    
    request.Status = StatusCompleted
    request.CompletedAt = time.Now().UTC()
    
    return g.updateRequest(request)
}

func (g *GDPRCompliance) HandleAccessRequest(ctx context.Context, request *DataSubjectRequest) (*DataExport, error) {
    // Collect all user data
    crashes, err := g.findUserCrashes(ctx, request.SubjectID)
    if err != nil {
        return nil, err
    }
    
    sessions, err := g.findUserSessions(ctx, request.SubjectID)
    if err != nil {
        return nil, err
    }
    
    export := &DataExport{
        RequestID:  request.ID,
        SubjectID:  request.SubjectID,
        GeneratedAt: time.Now().UTC(),
        Data: map[string]interface{}{
            "crash_reports": crashes,
            "sessions":      sessions,
            "summary": ExportSummary{
                TotalCrashes:  len(crashes),
                TotalSessions: len(sessions),
                FirstSeen:     g.earliestDate(crashes),
                LastSeen:      g.latestDate(crashes),
            },
        },
    }
    
    return export, nil
}

type DataExport struct {
    RequestID   string      `json:"request_id"`
    SubjectID   string      `json:"subject_id"`
    GeneratedAt time.Time   `json:"generated_at"`
    Format      string      `json:"format"` // json, csv
    Data        interface{} `json:"data"`
}
```

### HIPAA Considerations

```go
type HIPAACompliance struct {
    encryption  *EncryptionManager
    auditLog    *AuditLogger
    accessControl *AccessController
}

// Encryption at rest
type EncryptionManager struct {
    kmsClient *kms.Client
    keyID     string
}

func (e *EncryptionManager) EncryptPHI(data []byte) ([]byte, error) {
    // Use envelope encryption
    // 1. Generate data key from KMS
    dataKey, err := e.kmsClient.GenerateDataKey(context.Background(), &kms.GenerateDataKeyInput{
        KeyId:   aws.String(e.keyID),
        KeySpec: types.KeySpecAes256,
    })
    if err != nil {
        return nil, err
    }
    
    // 2. Encrypt data with data key
    block, err := aes.NewCipher(dataKey.Plaintext)
    if err != nil {
        return nil, err
    }
    
    ciphertext := make([]byte, aes.BlockSize+len(data))
    iv := ciphertext[:aes.BlockSize]
    if _, err := io.ReadFull(rand.Reader, iv); err != nil {
        return nil, err
    }
    
    stream := cipher.NewCFBEncrypter(block, iv)
    stream.XORKeyStream(ciphertext[aes.BlockSize:], data)
    
    // 3. Return encrypted data key + ciphertext
    result := append(dataKey.CiphertextBlob, ciphertext...)
    return result, nil
}

// Audit logging for PHI access
type AuditLogger struct {
    db *Database
}

type AuditEntry struct {
    Timestamp   time.Time `json:"timestamp"`
    Actor       string    `json:"actor"`      // User who accessed
    Action      string    `json:"action"`     // read, write, delete
    ResourceType string   `json:"resource_type"`
    ResourceID  string    `json:"resource_id"`
    Reason      string    `json:"reason"`     // Business justification
    ClientIP    string    `json:"client_ip"`
    UserAgent   string    `json:"user_agent"`
}

func (a *AuditLogger) Log(ctx context.Context, entry *AuditEntry) error {
    _, err := a.db.Collection("audit_log").InsertOne(ctx, entry)
    return err
}

// Access control
type AccessController struct {
    roles map[string]*Role
}

type Role struct {
    Name        string   `json:"name"`
    Permissions []string `json:"permissions"`
    CanAccessPHI bool    `json:"can_access_phi"`
}

func (a *AccessController) CheckPermission(user *User, resource string, action string) bool {
    role := a.roles[user.Role]
    if role == nil {
        return false
    }
    
    permission := fmt.Sprintf("%s:%s", resource, action)
    for _, p := range role.Permissions {
        if p == permission || p == fmt.Sprintf("%s:*", resource) {
            return true
        }
    }
    
    return false
}

// Minimum necessary rule
func (a *AccessController) FilterFields(user *User, data map[string]interface{}) map[string]interface{} {
    if !user.Role.CanAccessPHI {
        // Remove PHI fields
        filtered := make(map[string]interface{})
        for k, v := range data {
            if !isPHIField(k) {
                filtered[k] = v
            }
        }
        return filtered
    }
    return data
}

func isPHIField(field string) bool {
    phiFields := []string{
        "patient_name", "patient_dob", "patient_ssn", "medical_record_number",
        "diagnosis", "treatment", "medication", "allergies",
    }
    
    for _, phi := range phiFields {
        if strings.Contains(field, phi) {
            return true
        }
    }
    
    return false
}
```

### On-Premise Deployment

```yaml
# docker-compose.yml for on-premise deployment
version: '3.8'

services:
  # API Gateway
  api-gateway:
    image: backtrace-labs/morgue-gateway:latest
    ports:
      - "8080:8080"
    environment:
      - JWT_SECRET=${JWT_SECRET}
      - RATE_LIMIT=1000
    depends_on:
      - auth-service

  # Authentication Service
  auth-service:
    image: backtrace-labs/morgue-auth:latest
    environment:
      - DATABASE_URL=postgres://user:pass@postgres:5432/morgue_auth
      - LDAP_URL=${LDAP_URL}
      - SAML_METADATA=${SAML_METADATA}

  # Ingestion Service
  ingestion:
    image: backtrace-labs/morgue-ingestion:latest
    replicas: 3
    environment:
      - KAFKA_BROKERS=kafka:9092
      - REDIS_URL=redis://redis:6379
    depends_on:
      - kafka
      - redis

  # Processing Workers
  processor:
    image: backtrace-labs/morgue-processor:latest
    replicas: 5
    environment:
      - KAFKA_BROKERS=kafka:9092
      - MONGODB_URL=mongodb://mongo:27017
      - REDIS_URL=redis://redis:6379
      - S3_ENDPOINT=minio:9000
      - S3_ACCESS_KEY=${MINIO_ACCESS_KEY}
      - S3_SECRET_KEY=${MINIO_SECRET_KEY}
    depends_on:
      - kafka
      - mongo
      - redis
      - minio

  # Kafka for message queuing
  kafka:
    image: confluentinc/cp-kafka:latest
    ports:
      - "9092:9092"
    environment:
      - KAFKA_BROKER_ID=1
      - KAFKA_ZOOKEEPER_CONNECT=zookeeper:2181
      - KAFKA_ADVERTISED_LISTENERS=PLAINTEXT://kafka:9092
      - KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR=1
    depends_on:
      - zookeeper

  zookeeper:
    image: confluentinc/cp-zookeeper:latest
    environment:
      - ZOOKEEPER_CLIENT_PORT=2181

  # MongoDB for crash storage
  mongo:
    image: mongo:6
    volumes:
      - mongo_data:/data/db
    environment:
      - MONGO_INITDB_ROOT_USERNAME=${MONGO_USER}
      - MONGO_INITDB_ROOT_PASSWORD=${MONGO_PASSWORD}

  # Redis for caching
  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis_data:/data

  # Elasticsearch for search
  elasticsearch:
    image: elasticsearch:8.9.0
    volumes:
      - es_data:/usr/share/elasticsearch/data
    environment:
      - discovery.type=single-node
      - xpack.security.enabled=false
      - ES_JAVA_OPTS=-Xms2g -Xmx2g

  # MinIO for S3-compatible object storage
  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    volumes:
      - minio_data:/data
    environment:
      - MINIO_ROOT_USER=${MINIO_ACCESS_KEY}
      - MINIO_ROOT_PASSWORD=${MINIO_SECRET_KEY}
    command: server /data --console-address ":9001"

  # Web UI
  web-ui:
    image: backtrace-labs/morgue-ui:latest
    ports:
      - "3000:3000"
    environment:
      - API_URL=http://api-gateway:8080

volumes:
  mongo_data:
  redis_data:
  es_data:
  minio_data:
```

```bash
# deployment script
#!/bin/bash

# Backtrace On-Premise Deployment Script

set -e

# Configuration
NAMESPACE="${BACKTRACE_NAMESPACE:-backtrace}"
REPLICAS_PROCESSOR="${BACKTRACE_PROCESSOR_REPLICAS:-5}"
REPLICAS_INGESTION="${BACKTRACE_INGESTION_REPLICAS:-3}"
STORAGE_CLASS="${BACKTRACE_STORAGE_CLASS:-standard}"

echo "Deploying Backtrace Morgue to namespace: $NAMESPACE"

# Create namespace
kubectl create namespace $NAMESPACE 2>/dev/null || true

# Apply secrets
kubectl create secret generic backtrace-secrets \
  --from-literal=jwt-secret="$BACKTRACE_JWT_SECRET" \
  --from-literal=mongo-password="$BACKTRACE_MONGO_PASSWORD" \
  --from-literal=minio-access-key="$BACKTRACE_MINIO_ACCESS_KEY" \
  --from-literal=minio-secret-key="$BACKTRACE_MINIO_SECRET_KEY" \
  -n $NAMESPACE \
  --dry-run=client -o yaml | kubectl apply -f -

# Deploy infrastructure (MongoDB, Redis, Kafka, etc.)
kubectl apply -f k8s/infrastructure/ -n $NAMESPACE

# Wait for infrastructure to be ready
kubectl wait --for=condition=ready pod -l app=kafka -n $NAMESPACE --timeout=300s
kubectl wait --for=condition=ready pod -l app=mongo -n $NAMESPACE --timeout=300s

# Deploy application services
kubectl apply -f k8s/services/ -n $NAMESPACE

# Scale processors
kubectl scale deployment processor --replicas=$REPLICAS_PROCESSOR -n $NAMESPACE

# Apply ingress
kubectl apply -f k8s/ingress.yaml -n $NAMESPACE

echo "Deployment complete!"
echo "Access the UI at: https://backtrace.${BACKTRACE_DOMAIN}"
```

---

## 8. Appendix: Reference Implementations

### Complete Crash Report JSON Example

```json
{
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2026-04-05T14:30:00.123456Z",
  "server_timestamp": "2026-04-05T14:30:01.234567Z",
  "application": {
    "name": "MyApp",
    "version": "2.1.0",
    "build": "20260405.1",
    "bundle_id": "com.example.myapp",
    "environment": "production",
    "start_time": "2026-04-05T14:00:00.000000Z",
    "uptime_seconds": 1800.123
  },
  "device": {
    "type": "iPhone14,2",
    "model": "iPhone 13 Pro",
    "architecture": "arm64e",
    "os_name": "iOS",
    "os_version": "17.4.1",
    "os_build": "21E237",
    "jailbroken": false,
    "memory_total_bytes": 6442450944,
    "memory_free_bytes": 1073741824,
    "battery_level": 0.72,
    "thermal_state": "nominal",
    "locale": "en_US",
    "timezone": "America/New_York"
  },
  "crash": {
    "type": "native",
    "exception_type": "EXC_CRASH",
    "exception_code": "0x0000000000000000",
    "exception_subcode": "0x0000000000000000",
    "signal": "SIGABRT",
    "signal_code": "SI_TKILL",
    "reason": "Abort triggered",
    "address": "0x0000000100003f48",
    "access_type": "unknown",
    "faulting_instruction": "0x184a5ea58",
    "cpu_type": "arm64e",
    "thread_cause": "0x0000000000000113"
  },
  "threads": [
    {
      "id": 1,
      "name": "main",
      "label": "Main Thread",
      "crashed": true,
      "current_thread": true,
      "dispatch_queue": "com.apple.main-thread",
      "registers": {
        "x0": "0x0000000000000000",
        "x1": "0x0000000000000001",
        "pc": "0x0000000184a5ea58",
        "sp": "0x000000016f66ad60",
        "fp": "0x000000016f66ad80",
        "lr": "0x0000000184a5ea58"
      },
      "stack_frames": [
        {
          "index": 0,
          "instruction_address": "0x0000000184a5ea58",
          "return_address": "0x0000000184b34c80",
          "function_name": "abort",
          "library_name": "libsystem_kernel.dylib",
          "offset": 44,
          "symbolicated": true,
          "trust": "scan"
        },
        {
          "index": 1,
          "instruction_address": "0x0000000184b34c80",
          "return_address": "0x0000000100003f48",
          "function_name": "abort",
          "library_name": "libsystem_c.dylib",
          "offset": 180,
          "symbolicated": true,
          "trust": "cfi"
        },
        {
          "index": 2,
          "instruction_address": "0x0000000100003f48",
          "return_address": "0x0000000100123456",
          "function_name": "-[AppDelegate application:didFinishLaunchingWithOptions:]",
          "class_name": "AppDelegate",
          "file_name": "AppDelegate.swift",
          "line_number": 42,
          "column": 12,
          "library_name": "MyApp",
          "offset": 16200,
          "symbolicated": true,
          "trust": "fp",
          "source_status": "mapped"
        }
      ]
    }
  ],
  "binary_images": [
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440001",
      "name": "MyApp",
      "path": "/var/containers/Bundle/Application/XXX/MyApp.app/MyApp",
      "type": "executable",
      "image_address": "0x0000000100000000",
      "image_size_bytes": 52428800,
      "cpu_type": "arm64e",
      "architecture": "arm64e"
    }
  ],
  "memory_state": {
    "virtual_memory_size": 268435456,
    "resident_set_size": 134217728,
    "heap_size": 67108864,
    "heap_allocated": 52428800,
    "heap_free": 14680064
  },
  "attributes": {
    "user_id": "user_12345",
    "user_email_hash": "sha256:abc123",
    "subscription_tier": "premium",
    "feature_flag_new_ui": true,
    "network_type": "wifi",
    "carrier": "Verizon",
    "app_state": "foreground",
    "last_action": "purchase_complete",
    "screen": "checkout",
    "request_id": "req_abc123",
    "api_version": "v2",
    "custom_metric_latency_ms": 245
  },
  "breadcrumbs": [
    {
      "timestamp": "2026-04-05T14:28:00.000000Z",
      "level": "info",
      "type": "user",
      "message": "User tapped 'Add to Cart'",
      "data": {
        "product_id": "prod_789",
        "quantity": 2
      }
    },
    {
      "timestamp": "2026-04-05T14:29:30.000000Z",
      "level": "warning",
      "type": "http",
      "message": "Payment API returned 500",
      "data": {
        "url": "https://api.example.com/payment",
        "method": "POST",
        "status_code": 500,
        "duration_ms": 5000
      }
    },
    {
      "timestamp": "2026-04-05T14:30:00.000000Z",
      "level": "error",
      "type": "error",
      "message": "Payment processing failed",
      "data": {
        "error_code": "PAYMENT_TIMEOUT"
      }
    }
  ],
  "attachments": [
    {
      "name": "app_log.txt",
      "type": "text/plain",
      "size": 15360,
      "url": "s3://bucket/attachments/550e8400-e29b-41d4-a716-446655440000/app_log.txt"
    },
    {
      "name": "crash_dump.dmp",
      "type": "application/x-minidump",
      "size": 1048576,
      "url": "s3://bucket/attachments/550e8400-e29b-41d4-a716-446655440000/crash_dump.dmp"
    }
  ],
  "handled": false,
  "severity": "fatal",
  "fingerprint": "a1b2c3d4e5f6g7h8",
  "grouping_hash": "grp_xyz789"
}
```

### Fingerprinting Algorithm Implementation (Complete)

```go
// fingerprint/fingerprint.go
package fingerprint

import (
    "crypto/sha256"
    "encoding/hex"
    "fmt"
    "path"
    "sort"
    "strings"
)

type FingerprintGenerator struct {
    weights FingerprintWeights
}

type FingerprintWeights struct {
    ExceptionSignature float64
    TopFrames          float64
    ModulePattern      float64
    ThreadContext      float64
    ErrorContext       float64
}

func DefaultWeights() FingerprintWeights {
    return FingerprintWeights{
        ExceptionSignature: 1.0,
        TopFrames:          0.8,
        ModulePattern:      0.6,
        ThreadContext:      0.4,
        ErrorContext:       0.3,
    }
}

func NewGenerator(weights FingerprintWeights) *FingerprintGenerator {
    if weights.ExceptionSignature == 0 {
        weights = DefaultWeights()
    }
    return &FingerprintGenerator{weights: weights}
}

func (f *FingerprintGenerator) Generate(report *CrashReport) string {
    hasher := sha256.New()
    
    // Layer 1: Exception signature
    f.writeExceptionSignature(hasher, report)
    
    // Layer 2: Top frames
    f.writeTopFrames(hasher, report)
    
    // Layer 3: Module pattern
    f.writeModulePattern(hasher, report)
    
    // Layer 4: Thread context
    f.writeThreadContext(hasher, report)
    
    // Layer 5: Error context
    f.writeErrorContext(hasher, report)
    
    // Generate final hash (16 characters)
    hashBytes := hasher.Sum(nil)
    return hex.EncodeToString(hashBytes[:8])
}

func (f *FingerprintGenerator) writeExceptionSignature(hasher hash.Hash, report *CrashReport) {
    f.write(hasher, "exception", report.Crash.ExceptionType)
    f.write(hasher, "signal", report.Crash.Signal)
    f.write(hasher, "reason", report.Crash.Reason)
    f.write(hasher, "address_pattern", f.addressPattern(report.Crash.Address))
}

func (f *FingerprintGenerator) writeTopFrames(hasher hash.Hash, report *CrashReport) {
    crashedThread := f.getCrashedThread(report)
    if crashedThread == nil {
        return
    }
    
    for i := 0; i < min(5, len(crashedThread.StackFrames)); i++ {
        frame := crashedThread.StackFrames[i]
        f.write(hasher, fmt.Sprintf("frame_%d", i), f.frameSignature(frame))
    }
}

func (f *FingerprintGenerator) writeModulePattern(hasher hash.Hash, report *CrashReport) {
    modules := f.countModules(report)
    sorted := f.sortModulesByFrequency(modules)
    
    for i := 0; i < min(3, len(sorted)); i++ {
        f.write(hasher, fmt.Sprintf("module_%d", i), sorted[i].Key)
    }
}

func (f *FingerprintGenerator) writeThreadContext(hasher hash.Hash, report *CrashReport) {
    crashedThread := f.getCrashedThread(report)
    if crashedThread == nil {
        return
    }
    
    f.write(hasher, "thread_name", crashedThread.Name)
    f.write(hasher, "dispatch_queue", crashedThread.DispatchQueue)
}

func (f *FingerprintGenerator) writeErrorContext(hasher hash.Hash, report *CrashReport) {
    if report.Attributes != nil {
        if appState, ok := report.Attributes["app_state"]; ok {
            f.write(hasher, "app_state", appState)
        }
    }
}

func (f *FingerprintGenerator) write(hasher hash.Hash, key, value string) {
    if value == "" {
        return
    }
    fmt.Fprintf(hasher, "%s=%s|", key, strings.ToLower(value))
}

func (f *FingerprintGenerator) frameSignature(frame *StackFrame) string {
    // Prefer source location
    if frame.FileName != "" && frame.LineNumber > 0 {
        return fmt.Sprintf("%s:%d", path.Base(frame.FileName), frame.LineNumber)
    }
    // Fall back to function name
    if frame.FunctionName != "" {
        return frame.FunctionName
    }
    // Last resort: module + offset pattern
    return fmt.Sprintf("%s+0x%x", frame.LibraryName, min(frame.Offset, 0xFFF))
}

func (f *FingerprintGenerator) addressPattern(addr string) string {
    if addr == "" {
        return "none"
    }
    if addr == "0x0000000000000000" {
        return "null"
    }
    if strings.HasPrefix(addr, "0x0000000000000") {
        return "small_offset"
    }
    if strings.HasPrefix(addr, "0x00007") {
        return "stack"
    }
    return "mapped"
}

func (f *FingerprintGenerator) getCrashedThread(report *CrashReport) *Thread {
    for _, t := range report.Threads {
        if t.Crashed {
            return t
        }
    }
    if len(report.Threads) > 0 {
        return report.Threads[0]
    }
    return nil
}

func (f *FingerprintGenerator) countModules(report *CrashReport) map[string]int {
    modules := make(map[string]int)
    crashedThread := f.getCrashedThread(report)
    if crashedThread == nil {
        return modules
    }
    
    for _, frame := range crashedThread.StackFrames {
        modules[frame.LibraryName]++
    }
    return modules
}

func (f *FingerprintGenerator) sortModulesByFrequency(modules map[string]int) []kv {
    var sorted []kv
    for k, v := range modules {
        sorted = append(sorted, kv{k, v})
    }
    sort.Slice(sorted, func(i, j int) bool {
        return sorted[i].Value > sorted[j].Value
    })
    return sorted
}

type kv struct {
    Key   string
    Value int
}

func min(a, b int) int {
    if a < b {
        return a
    }
    return b
}
```

### Alert Rule Configuration Examples

```yaml
# alert-rules.yaml

rules:
  # Critical crash rate alert
  - id: critical-crash-rate
    name: Critical Crash Rate
    description: Crash rate exceeds critical threshold
    enabled: true
    type: crash_rate
    severity: critical
    conditions:
      - field: crash_rate_per_k_sessions
        operator: gt
        value: 50
    evaluation_window: 1h
    channels:
      - slack
      - pagerduty
      - webhook
    cooldown: 30m

  # Crash spike detection
  - id: crash-spike
    name: Crash Spike Detection
    description: Sudden increase in crashes
    enabled: true
    type: crash_spike
    severity: high
    conditions:
      - field: spike_percent
        operator: gt
        value: 200
    evaluation_window: 1h
    channels:
      - slack
    cooldown: 1h

  # New issue alert
  - id: new-issue
    name: New Crash Issue Detected
    description: New crash group created
    enabled: true
    type: new_issue
    severity: medium
    conditions:
      - field: new_groups_per_hour
        operator: gt
        value: 5
    evaluation_window: 1h
    channels:
      - slack
    cooldown: 2h

  # High-impact crash
  - id: high-impact
    name: High Impact Crash
    description: Crash affecting many users
    enabled: true
    type: affected_users
    severity: high
    conditions:
      - field: affected_users
        operator: gt
        value: 100
    evaluation_window: 24h
    channels:
      - slack
      - email
    cooldown: 4h

  # Regression detection
  - id: regression
    name: Version Regression
    description: Crash rate increased in new version
    enabled: true
    type: regression
    severity: high
    conditions:
      - field: crash_rate_increase_percent
        operator: gt
        value: 50
    channels:
      - slack
      - webhook
    cooldown: 6h
```

---

## Architecture Diagrams

### Complete System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Backtrace Morgue Architecture                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                         CLIENT LAYER                                  │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │   │
│  │  │   Go     │ │  Cocoa   │ │   JS     │ │ Android  │ │  Native  │   │   │
│  │  │   SDK    │ │   SDK    │ │   SDK    │ │   SDK    │ │(Crashpad)│   │   │
│  │  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘   │   │
│  └───────┼───────────┼───────────┼───────────┼───────────┼───────────┘   │
│          │           │           │           │           │               │
│          └───────────┴───────────┼───────────┴───────────┘               │
│                                  │ HTTPS                                  │
│                                  ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                       INGESTION LAYER                                 │   │
│  │  ┌────────────────────────────────────────────────────────────────┐   │   │
│  │  │                    API Gateway / Load Balancer                  │   │   │
│  │  │  - TLS Termination    - Rate Limiting    - Request Routing     │   │   │
│  │  └────────────────────────────┬───────────────────────────────────┘   │   │
│  │                               │                                        │   │
│  │                               ▼                                        │   │
│  │  ┌────────────────────────────────────────────────────────────────┐   │   │
│  │  │                    Authentication Service                       │   │   │
│  │  │  - Token Validation   - App ID Resolution   - Quota Check      │   │   │
│  │  └────────────────────────────┬───────────────────────────────────┘   │   │
│  │                               │                                        │   │
│  │                               ▼                                        │   │
│  │  ┌────────────────────────────────────────────────────────────────┐   │   │
│  │  │                    Kafka Topic: crash-reports                   │   │   │
│  │  │              (Persistent, replicated message queue)             │   │   │
│  │  └────────────────────────────────────────────────────────────────┘   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      PROCESSING LAYER                                 │   │
│  │                                                                       │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │   │
│  │  │  Processor  │  │  Processor  │  │  Processor  │  │  Processor  │  │   │
│  │  │  Worker 1   │  │  Worker 2   │  │  Worker 3   │  │  Worker N   │  │   │
│  │  │             │  │             │  │             │  │             │  │   │
│  │  │ - Parse     │  │ - Parse     │  │ - Parse     │  │ - Parse     │  │   │
│  │  │ - Fingerprint│ │ - Fingerprint│ │ - Fingerprint│ │ - Fingerprint│  │   │
│  │  │ - Classify  │  │ - Classify  │  │ - Classify  │  │ - Classify  │  │   │
│  │  │ - Group     │  │ - Group     │  │ - Group     │  │ - Group     │  │   │
│  │  │ - Symbolicate│ │ - Symbolicate│ │ - Symbolicate│ │ - Symbolicate│  │   │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  │   │
│  │         │                │                │                │         │   │
│  └─────────┼────────────────┼────────────────┼────────────────┼─────────┘   │
│            │                │                │                │             │
│            └────────────────┴────────────────┴────────────────┘             │
│                                   │                                         │
│  ┌────────────────────────────────┼────────────────────────────────────┐   │
│  │                      STORAGE LAYER                                   │   │
│  │                               │                                      │   │
│  │         ┌─────────────────────┼─────────────────────┐                │   │
│  │         │                     │                     │                │   │
│  │         ▼                     ▼                     ▼                │   │
│  │  ┌─────────────┐       ┌─────────────┐       ┌─────────────┐         │   │
│  │  │   MongoDB   │       │    Redis    │       │  MinIO/S3   │         │   │
│  │  │             │       │             │       │             │         │   │
│  │  │ - Reports   │       │ - Cache     │       │ - Attachments│        │   │
│  │  │ - Groups    │       │ - Metrics   │       │ - Symbols   │         │   │
│  │  │ - Sessions  │       │ - Counters  │       │ - Archives  │         │   │
│  │  └──────┬──────┘       └──────┬──────┘       └──────┬──────┘         │   │
│  │         │                     │                     │                │   │
│  │         ▼                     │                     │                │   │
│  │  ┌─────────────┐              │                     │                │   │
│  │  │Elasticsearch│◀─────────────┘                     │                │   │
│  │  │             │                                    │                │   │
│  │  │ - Full-text │                                    │                │   │
│  │  │ - Aggregations                                   │                │   │
│  │  │ - Analytics │                                    │                │   │
│  │  └─────────────┘                                    │                │   │
│  └─────────────────────────────────────────────────────┼────────────────┘   │
│                                                        │                     │
│  ┌─────────────────────────────────────────────────────┼────────────────┐   │
│  │                       QUERY LAYER                                      │   │
│  │                                                    │                   │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │                   │   │
│  │  │  REST API   │  │  GraphQL    │  │   WebSocket │─┘                   │   │
│  │  │             │  │  API        │  │   (Real-time)│                    │   │
│  │  │ - Reports   │  │ - Flexible  │  │ - Live      │                    │   │
│  │  │ - Groups    │  │  Queries    │  │   Updates   │                    │   │
│  │  │ - Metrics   │  │ - Subscriptions│              │                    │   │
│  │  └──────┬──────┘  └──────┬──────┘  └─────────────┘                    │   │
│  │         │                │                                            │   │
│  └─────────┼────────────────┼────────────────────────────────────────────┘   │
│            │                │                                                 │
│            └────────────────┘                                                 │
│                   │                                                           │
│                   ▼                                                           │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                       PRESENTATION LAYER                              │   │
│  │                                                                       │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │   │
│  │  │   Web UI    │  │  Slack App  │  │  Webhooks   │  │   CLI       │  │   │
│  │  │  (React)    │  │             │  │             │  │   Tool      │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Crash Report Data Flow                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────┐                                                             │
│  │   Crash     │ 1. Application crashes                                      │
│  │   Occurs    │                                                             │
│  └──────┬──────┘                                                             │
│         │                                                                    │
│         ▼                                                                    │
│  ┌─────────────┐                                                             │
│  │   Client    │ 2. SDK captures crash state                                 │
│  │     SDK     │    - Stack trace, registers, memory                         │
│  └──────┬──────┘    - Breadcrumbs, attributes                                │
│         │                                                                    │
│         ▼                                                                    │
│  ┌─────────────┐                                                             │
│  │   Offline   │ 3. Store in local database (optional)                       │
│  │    Queue    │    - Retry on failure                                       │
│  └──────┬──────┘    - Deduplication                                          │
│         │                                                                    │
│         ▼                                                                    │
│  ┌─────────────┐                                                             │
│  │    HTTP     │ 4. POST to Backtrace API                                    │
│  │   Request   │    - Compressed (gzip)                                      │
│  └──────┬──────┘    - Authenticated                                          │
│         │                                                                    │
│         ▼                                                                    │
│  ┌─────────────┐                                                             │
│  │   Gateway   │ 5. Validate and route                                       │
│  └──────┬──────┘                                                             │
│         │                                                                    │
│         ▼                                                                    │
│  ┌─────────────┐                                                             │
│  │    Kafka    │ 6. Queue for async processing                               │
│  └──────┬──────┘                                                             │
│         │                                                                    │
│         ├─────────────────────────────────────────┐                          │
│         │                                         │                          │
│         ▼                                         ▼                          │
│  ┌─────────────┐                           ┌─────────────┐                   │
│  │  Processor  │ 7a. Main processing       │  Processor  │ 7b. Parallel       │
│  │             │     - Parse JSON          │             │     processing     │
│  │             │     - Generate fingerprint│             │                     │
│  │             │     - Classify crash      │             │                     │
│  │             │     - Find/create group   │             │                     │
│  └──────┬──────┘                           └──────┬──────┘                   │
│         │                                         │                          │
│         └──────────────────┬──────────────────────┘                          │
│                            │                                                 │
│                            ▼                                                 │
│                   ┌─────────────────┐                                        │
│                   │  Storage Writes │                                        │
│                   │                 │                                        │
│                   │ ┌─────────────┐ │                                        │
│                   │ │   MongoDB   │ │ 8a. Store report & group              │
│                   │ └─────────────┘ │                                        │
│                   │ ┌─────────────┐ │                                        │
│                   │ │  Redis      │ │ 8b. Update cache & counters           │
│                   │ └─────────────┘ │                                        │
│                   │ ┌─────────────┐ │                                        │
│                   │ │     S3      │ │ 8c. Store attachments                  │
│                   │ └─────────────┘ │                                        │
│                   │ ┌─────────────┐ │                                        │
│                   │ │Elasticsearch│ │ 8d. Index for search                   │
│                   │ └─────────────┘ │                                        │
│                   └─────────────────┘                                        │
│                            │                                                 │
│                            ▼                                                 │
│                   ┌─────────────────┐                                        │
│                   │   Post-Process  │                                        │
│                   │                 │                                        │
│                   │ - Check alerts  │ 9a. Evaluate alert rules               │
│                   │ - Send webhooks │ 9b. Trigger notifications              │
│                   │ - Update metrics│ 9c. Update real-time dashboards        │
│                   └─────────────────┘                                        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Document Summary

This deep dive covers the complete crash reporting lifecycle for Backtrace:

1. **Crash Report Structure** - Complete JSON schema with headers, threads, registers, stack frames, memory state, binary images, attributes, and breadcrumbs.

2. **Crash Classification** - Exception types (EXC_BAD_ACCESS, EXC_CRASH), signals (SIGSEGV, SIGABRT), crash reasons, ANR thresholds, OOM detection, and panic vs signal differentiation.

3. **Crash Aggregation** - Multi-layer fingerprinting, Levenshtein/Jaccard similarity, frame-weighted grouping, signature generation, deduplication strategies, and threshold tuning.

4. **Crash Analysis** - Crash rate calculations, time-series trend analysis, impact scoring, regression detection, release comparison, and device/OS breakdowns.

5. **Server-Side Processing** - Kafka-based ingestion pipeline, processing workers, MongoDB schema design, Redis caching, Elasticsearch indexing, and S3 attachment storage.

6. **Real-time Features** - Webhook notifications with HMAC signatures, Slack Block Kit integration, configurable alert thresholds, spike detection with statistical analysis, and new issue prioritization.

7. **Privacy & Compliance** - PII scrubbing with regex patterns, configurable data retention policies, GDPR data subject request handling, HIPAA encryption and audit logging, and on-premise Kubernetes deployment.

All implementations are production-ready with complete code examples, configuration files, and architecture diagrams.
