---
name: Zlob
description: High-performance POSIX/glibc compatible globbing library implemented in Zig with Rust bindings
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/zlob/
---

# Zlob - High-Performance Globbing Library

## Overview

Zlob is a **high-performance globbing library** implemented in Zig with C and Rust bindings. It provides 100% POSIX and glibc compatible globbing that is significantly faster than the standard `glob()` implementation while supporting all modern globbing patterns including `**`, braces, gitignore, and bash extglob.

Key features:
- **10x faster than glibc** - SIMD-optimized pattern matching
- **POSIX/glibc compatible** - All standard flags and features
- **Modern patterns** - `**`, braces, extglob, gitignore
- **Cross-platform** - Linux, macOS, Windows (forward slashes)
- **Direct syscalls** - Uses `getdents64` for fast directory listing
- **Multiple bindings** - C, Zig, and Rust APIs

## Directory Structure

```
zlob/
├── src/
│   ├── zlob.zig              # Core globbing implementation
│   ├── lib.zig               # Library root
│   ├── main.zig              # CLI entry point
│   ├── c_lib.zig             # C API bindings
│   ├── flags.zig             # Glob flags and options
│   ├── pattern_context.zig   # Pattern parsing and analysis
│   ├── path_matcher.zig      # Path matching logic
│   ├── brace_optimizer.zig   # Brace expansion optimization
│   ├── fnmatch.zig           # fnmatch implementation
│   ├── gitignore.zig         # .gitignore support
│   ├── walker.zig            # Directory walker
│   ├── suffix_match.zig      # Suffix/pattern matching
│   ├── sorting.zig           # Result sorting
│   └── utils.zig             # Utilities
├── include/
│   └── zlob.h                # C header
├── rust/                     # Rust bindings
├── build.zig                 # Zig build system
├── Makefile                  # Build automation
├── README.md
└── LICENSE
```

## Performance Architecture

### Why Zlob is Faster

```
┌─────────────────────────────────────────────────────────────────┐
│                    Zlob Performance Layers                      │
└─────────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│  Pattern         │ │  SIMD-optimized  │ │  Direct syscall  │
│  Analysis        │ │  Matching        │ │  (getdents64)    │
│                  │ │                  │ │                  │
│ - Parse once     │ │ - 16-byte        │ │ - Skip libc      │
│ - Optimize       │ │   parallel       │ │ - Batch read     │
│ - Cache hot      │ │ - Bitmask        │ │ - Lower overhead │
│   paths          │ │   matching       │ │                  │
│ - Invariant      │ │ - Extension      │ │                  │
│   elimination    │ │   matching       │ │                  │
└──────────────────┘ └──────────────────┘ └──────────────────┘
```

### Pattern Analysis

```zig
// Pattern is analyzed once before matching
const PatternContext = struct {
    /// Original pattern
    pattern: []const u8,

    /// Parsed pattern components
    components: []PatternComponent,

    /// Constant prefix (e.g., "src/" in "src/**/*.c")
    constant_prefix: []const u8,

    /// Constant suffix (e.g., ".c" in "**/*.c")
    constant_suffix: []const u8,

    /// Whether pattern contains **
    has_recursive: bool,

    /// Whether pattern contains braces
    has_braces: bool,

    /// Whether pattern contains negation
    has_negation: bool,

    /// Optimized matcher
    matcher: Matcher,
};

fn analyze(pattern: []const u8, allocator: Allocator) !PatternContext {
    var ctx = PatternContext{
        .pattern = pattern,
        .components = try parse(pattern, allocator),
        .constant_prefix = extractConstantPrefix(pattern),
        .constant_suffix = extractConstantSuffix(pattern),
        .has_recursive = containsRecursive(pattern),
        .has_braces = containsBraces(pattern),
        .has_negation = containsNegation(pattern),
    };

    // Optimize brace expansion
    if (ctx.has_braces) {
        ctx.components = try optimizeBraces(ctx.components, allocator);
    }

    // Pre-compile matcher
    ctx.matcher = try compileMatcher(ctx.components);

    return ctx;
}
```

### SIMD Matching

```zig
// SIMD-optimized suffix matching for extensions
const SuffixMatcher = struct {
    /// Bitmask for parallel character matching
    char_mask: @Vector(16, u8),

    /// Extensions to match
    extensions: []const []const u8,

    pub fn init(extensions: []const []const u8) SuffixMatcher {
        // Pack multiple extensions into SIMD registers
        var mask: @Vector(16, u8) = undefined;
        for (extensions, 0..) |ext, i| {
            mask[i] = ext[0];  // First char of each extension
        }
        return .{
            .char_mask = mask,
            .extensions = extensions,
        };
    }

    pub fn match(self: *const SuffixMatcher, path: []const u8) bool {
        // Load 16 bytes at once using SIMD
        const chunk: @Vector(16, u8) = path[0..16].*;

        // Compare all extensions in parallel
        const matches = chunk == self.char_mask;

        // Check if any matched
        for (matches) |m| {
            if (m) {
                return verifyFullExtension(path, self.extensions);
            }
        }
        return false;
    }
};
```

### Direct Directory Listing

```zig
// Direct getdents64 syscall for Linux
const Walker = struct {
    fd: i32,
    buffer: [8192]u8,

    pub fn openDir(path: []const u8) !Walker {
        const fd = try sys.openat(
            sys.AT.FDCWD,
            path,
            .{ .ACCMODE = .RDONLY, .DIRECTORY = true },
            0,
        );
        return .{ .fd = fd, .buffer = undefined };
    }

    pub fn readEntries(self: *Walker) ![]Dirent64 {
        // Direct syscall - no libc overhead
        const n = try sys.getdents64(self.fd, &self.buffer);

        // Parse dirent structures
        var entries: []Dirent64 = undefined;
        var offset: usize = 0;
        while (offset < n) {
            const dirent = @as(*const Dirent64,
                @ptrCast(self.buffer[offset..].ptr)
            );
            entries.append(dirent);
            offset += dirent.d_reclen;
        }
        return entries;
    }
};
```

## API Usage

### C API

```c
#include "zlob.h"

// Basic glob
glob_t globbuf;
int ret = zlob("*.c", ZLOB_RECOMMENDED, NULL, &globbuf);

if (ret == 0) {
    for (size_t i = 0; i < globbuf.gl_pathc; i++) {
        printf("%s\n", globbuf.gl_pathv[i]);
    }
    zglobfree(&globbuf);
}

// With flags
ret = zlob(
    "./{src,lib}/**/*.c",
    ZLOB_BRACE | ZLOB_RECURSIVE | ZLOB_NOSORT,
    NULL,
    &globbuf
);

// With error callback
void error_handler(const char *epath, int eerrno) {
    fprintf(stderr, "Error accessing %s: %s\n", epath, strerror(eerrno));
}

ret = zlob("**/*.c", ZLOB_RECOMMENDED, error_handler, &globbuf);
```

### Rust API

```rust
use zlob::{Glob, Flags};

// Basic glob
let mut glob = Glob::new("*.c", Flags::RECOMMENDED)?;
for entry in glob.iter() {
    println!("{}", entry.path);
}

// With options
let glob = Glob::builder("./{src,lib}/**/*.c")
    .brace(true)
    .recursive(true)
    .nosort(true)
    .build()?;

// Collect results
let paths: Vec<PathBuf> = glob.collect();

// With gitignore support
let glob = Glob::builder("**/*.rs")
    .gitignore(true)
    .gitignore_path("./.gitignore")
    .build()?;
```

### Zig API

```zig
const zlob = @import("zlob");

// Basic usage
var glob = try zlob.Glob.init("*.c", .{});
defer glob.deinit();

while (try glob.next()) |entry| {
    print("{}\n", .{entry.path});
}

// With options
var walker = try zlob.Walker.init(
    "**/*.{c,h}",
    .{
        .brace = true,
        .recursive = true,
        .gitignore = true,
    },
);

while (try walker.next()) |entry| {
    print("{}\n", .{entry.path});
}
```

## Supported Patterns

### Basic Patterns

| Pattern | Description |
|---------|-------------|
| `*.c` | All `.c` files in current directory |
| `?at` | Single char + "at" (cat, bat, rat) |
| `[abc]*` | Files starting with a, b, or c |
| `[a-z]*` | Files starting with lowercase |
| `*.[!ch]` | Files not ending in .c or .h |

### Recursive Patterns

| Pattern | Description |
|---------|-------------|
| `**/*.c` | All `.c` files recursively |
| `src/**/test/*.rs` | Test files under src/ at any depth |
| `**/*.{c,h}` | All .c and .h files recursively |

### Brace Expansion

| Pattern | Description |
|---------|-------------|
| `*.{c,h}` | Files ending in .c or .h |
| `{src,lib,test}/*.c` | .c files in src, lib, or test |
| `file.{c{pp,h},o}` | Expands to .cpp, .ch, .o |

### Bash Extglob

| Pattern | Description |
|---------|-------------|
| `@(a|b).c` | Exactly "a.c" or "b.c" |
| `*(a|b).c` | Zero or more a or b + .c |
| `+(a|b).c` | One or more a or b + .c |
| `?(a|b).c` | Optional a or b + .c |
| `!(a|b).c` | Not a.c or b.c |

### Special Patterns

| Pattern | Description |
|---------|-------------|
| `~/*.c` | .c files in home directory |
| `**/.git/**` | Everything under .git directories |
| `!**/node_modules/**` | Exclude node_modules |

## Gitignore Support

```zig
const Gitignore = struct {
    patterns: []Pattern,
    negations: []Pattern,

    pub fn load(path: []const u8) !Gitignore {
        const content = try fs.readFile(path);
        var patterns = std.ArrayList(Pattern).init(allocator);

        var lines = std.mem.split(u8, content, "\n");
        while (lines.next()) |line| {
            // Skip comments and empty lines
            if (line.len == 0 or line[0] == '#') continue;

            // Handle negations
            const is_negation = line[0] == '!';
            const pattern = if (is_negation) line[1..] else line;

            try patterns.append(try Pattern.parse(pattern));
        }

        return .{
            .patterns = patterns.items,
            .negations = undefined,
        };
    }

    pub fn isIgnored(self: *const Gitignore, path: []const u8) bool {
        var ignored = false;
        for (self.patterns) |pattern| {
            if (pattern.matches(path)) {
                ignored = true;
            }
        }
        for (self.negations) |pattern| {
            if (pattern.matches(path)) {
                ignored = false;  // Negation overrides
            }
        }
        return ignored;
    }
};
```

## Flags

```c
// Flag values
#define ZLOB_ERR_CHECKED       0x0001  // Error checking enabled
#define ZLOB_NOESCAPE          0x0002  // Disable backslash escaping
#define ZLOB_BRACE             0x0004  // Enable brace expansion
#define ZLOB_PERIOD            0x0008  // Leading period matching
#define ZLOB_MARK              0x0010  // Mark directories with /
#define ZLOB_NOSORT            0x0020  // Disable sorting (faster)
#define ZLOB_TILDE             0x0040  // Enable ~ expansion
#define ZLOB_TILDE_CHECK       0x0080  // ~ expansion with error checking
#define ZLOB_EXTGLOB           0x0100  // Enable bash extglob
#define ZLOB_IGNORECASE        0x0200  // Case-insensitive matching
#define ZLOB_RECOMMENDED       0x007F  // Sensible defaults
```

## Performance Benchmarks

```
Pattern: ./drivers/**/*.c
Files: 100,000

glibc glob:  450ms
zlob:         45ms  (10x faster)

Pattern: **/*.{c,h,rs,go}
Files: 50,000

glibc glob:  280ms
zlob:        180ms  (1.5x faster)

Pattern: src/**/test/**/*.rs
Files: 25,000

glibc glob:  150ms
zlob:        120ms  (1.25x faster)
```

## Integration with FFrames

```rust
// Media file discovery
use zlob::Glob;

fn find_media_files(directory: &str) -> Vec<PathBuf> {
    let pattern = format!("{}/**/*.[mp4,mkv,avi,mov]", directory);
    Glob::new(&pattern, zlob::Flags::RECOMMENDED)
        .unwrap()
        .collect()
}

// Font discovery
fn find_fonts() -> Vec<PathBuf> {
    Glob::new("**/*.[ttf,otf,woff,woff2]", zlob::Flags::RECOMMENDED)
        .unwrap()
        .filter(|p| is_valid_font(p))
        .collect()
}
```

## Related Documents

- [FFrames Media Loaders](./fframes-media-loaders-exploration.md) - File discovery

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/zlob/`
- Zlob GitHub: https://github.com/absidue/zlob
- Zig Documentation: https://ziglang.org/documentation/
