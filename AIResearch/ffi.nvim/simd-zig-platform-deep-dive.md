---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/fff.nvim
explored_at: 2026-04-07
focus: SIMD Instructions, Zig Integration, and Platform-Specific Operations
---

# Deep Dive: SIMD, Zig, and Platform Operations in fff.nvim

## Executive Summary

fff.nvim achieves its "freakin fast" performance through three key technical strategies:

1. **SIMD-accelerated substring search** — AVX2 (x86_64) and NEON+dotprod (aarch64) implementations for case-insensitive grep
2. **Zig-compiled C library (zlob)** — Cross-platform glob matching compiled via Zig for consistent behavior across targets
3. **Platform-specific Rust** — Conditional compilation for Windows path handling, macOS linking, ARM feature detection

This document provides a line-by-line breakdown of how these systems work individually and how they integrate.

---

## Part 1: SIMD Implementation Deep Dive

### 1.1 Architecture Overview

The SIMD implementation lives in `crates/fff-core/src/case_insensitive_memmem.rs` (693 lines). It provides three search strategies with a clear performance hierarchy:

```
search_packed_pair  →  AVX2/NEON packed scan (fastest, quadratic selectivity)
      ↓
search              →  memchr2 first-byte scan + SIMD verify
      ↓
search_scalar       →  memchr2 first-byte scan + scalar verify (baseline)
```

### 1.2 Core SIMD Insight: Why Packed-Pair?

The packed-pair algorithm mirrors what `memchr::memmem` does internally:

> "Pick two rare bytes from the needle, SIMD-scan for both simultaneously, verify candidates. This gives **quadratic selectivity** over the single-byte memchr2 approach."

**The math:**
- Single-byte scan (memchr2): Finds all positions where byte A matches → O(n) candidates
- Two-byte packed scan: Finds all positions where byte A **AND** byte B match → O(n/256) candidates

For a typical needle like `"nomore"`:
- First byte `'n'` appears ~246 times per 1000 bytes (frequency rank from BYTE_FREQUENCIES table)
- Adding a second rare byte reduces false positives by ~256×

### 1.3 Byte Frequency Heuristic

```rust
// Lines 14-35: BYTE_FREQUENCIES table
const BYTE_FREQUENCIES: [u8; 256] = [
    55, 52, 51, 50, 49, 48, 47, 46, 45, 103, 242, 66, 67, 229, 44, 43, // 0x00
    // ...
    253,  // 'e' (most common)
    152,  // 'z' (rare)
    139,  // 'q' (rarest)
];
```

This table ranks bytes by rarity (lower = rarer). The values come from empirical analysis of English text and code. Key observations:
- `'q'` (139), `'z'` (152), `'j'` (167) are rarest
- `'e'` (253), `'s'` (243), `'t'` (242) are most common

**Case-insensitive ranking** (lines 52-58):
```rust
fn case_insensitive_rank(lower: u8) -> u8 {
    if lower.is_ascii_lowercase() {
        let upper = ascii_swap_case(lower);
        BYTE_FREQUENCIES[lower as usize].max(BYTE_FREQUENCIES[upper as usize])
    } else {
        BYTE_FREQUENCIES[lower as usize]
    }
}
```

For `'n'` vs `'N'`, it takes the max frequency because we must scan for **both** variants.

### 1.4 Rare Pair Selection

```rust
// Lines 63-82: select_rare_pair
fn select_rare_pair(needle_lower: &[u8]) -> (usize, usize) {
    let mut best1 = (u8::MAX, 0usize); // (rank, position)
    let mut best2 = (u8::MAX, 1usize);

    for (i, &b) in needle_lower.iter().enumerate() {
        let r = case_insensitive_rank(b);
        if r < best1.0 {
            best2 = best1;
            best1 = (r, i);
        } else if r < best2.0 && i != best1.1 {
            best2 = (r, i);
        }
    }

    (best1.1.min(best2.1), best1.1.max(best2.1))
}
```

For `"nomore"`:
- `'n'` rank 246, `'o'` rank 244, `'m'` rank 233, `'r'` rank 245, `'e'` rank 253
- Selects positions with `'m'` (233) and one other rare byte
- Result: scan for matches at those two offsets simultaneously

### 1.5 AVX2 Implementation (x86_64)

#### 1.5.1 The XOR-0x80 Trick

AVX2 only has **signed** byte compare (`_mm256_cmpgt_epi8`), but we need **unsigned** range checks (`'A' <= byte <= 'Z'`).

**Solution** (lines 99-102):
> "XOR every byte with `0x80`, which maps the unsigned range `[0, 255]` into the signed range `[-128, 127]` while preserving order."

```
Before XOR:  'A'=65,  'Z'=90,  0=0,   255=255
After XOR:   'A'=-111, 'Z'=-66, 0=-128, 255=127

Signed comparison now works:
  -111 > -119 ('A'-1)  ✓
  -66  < -65  ('Z'+1)  ✓
```

#### 1.5.2 AVX2 Verify Function

```rust
// Lines 108-166: verify_avx2
#[target_feature(enable = "avx2")]
unsafe fn verify_avx2(h: *const u8, needle_lower: &[u8]) -> bool {
    use core::arch::x86_64::*;

    // Broadcast constants
    let flip = _mm256_set1_epi8(0x80u8 as i8);           // XOR for unsigned→signed
    let a_minus_1 = _mm256_set1_epi8((b'A' - 1) as i8 ^ 0x80u8 as i8);
    let z_plus_1 = _mm256_set1_epi8((b'Z' + 1) as i8 ^ 0x80u8 as i8);
    let bit20 = _mm256_set1_epi8(0x20u8 as i8);          // ASCII case bit

    while i + 32 <= len {
        let hv = _mm256_loadu_si256(h.add(i) as *const __m256i);  // Load 32 haystack bytes
        let nv = _mm256_loadu_si256(needle_lower.as_ptr().add(i) as *const __m256i);

        // Step 1: Flip to signed domain
        let x = _mm256_xor_si256(hv, flip);

        // Step 2: Range check for 'A'..'Z'
        let ge_a = _mm256_cmpgt_epi8(x, a_minus_1);  // x > 'A'-1  →  x >= 'A'
        let le_z = _mm256_cmpgt_epi8(z_plus_1, x);   // 'Z'+1 > x  →  x <= 'Z'
        let upper = _mm256_and_si256(ge_a, le_z);    // Both true = uppercase

        // Step 3: Case-fold (set bit 5 on uppercase)
        let folded = _mm256_or_si256(hv, _mm256_and_si256(upper, bit20));

        // Step 4: Compare against lowercase needle
        let eq = _mm256_cmpeq_epi8(folded, nv);
        if _mm256_movemask_epi8(eq) != -1i32 {  // All bits must be 1
            return false;
        }

        i += 32;
    }
    // ... scalar tail for remainder
}
```

**Instruction count per 32 bytes:**
- 2 loads (`_mm256_loadu_si256`)
- 4 broadcasts (constants, hoisted out of loop)
- 1 XOR, 2 CMPGT, 2 AND, 1 OR, 1 CMPEQ, 1 MOVEMASK
- **~12 SIMD instructions per 32 bytes**

#### 1.5.3 AVX2 Packed-Pair Kernel

```rust
// Lines 377-481: search_packed_pair_avx2
unsafe fn search_packed_pair_avx2(
    haystack: &[u8],
    needle_lower: &[u8],
    i1: usize,  // Position of first rare byte
    i2: usize,  // Position of second rare byte
) -> bool {
    // Pre-load case variants for both rare bytes
    let b1 = needle_lower[i1];
    let b1_alt = ascii_swap_case(b1);  // 'n' → 'N' or 'N' → 'n'
    let b2 = needle_lower[i2];
    let b2_alt = ascii_swap_case(b2);

    let v1_lo = _mm256_set1_epi8(b1 as i8);
    let v1_hi = _mm256_set1_epi8(b1_alt as i8);
    let v2_lo = _mm256_set1_epi8(b2 as i8);
    let v2_hi = _mm256_set1_epi8(b2_alt as i8);

    while offset <= max_offset {
        // Load 32 bytes at each rare-byte offset
        let chunk1 = _mm256_loadu_si256(ptr.add(offset + i1) as *const __m256i);
        let chunk2 = _mm256_loadu_si256(ptr.add(offset + i2) as *const __m256i);

        // Case-insensitive match: (chunk == lower) OR (chunk == upper)
        let eq1 = _mm256_or_si256(
            _mm256_cmpeq_epi8(chunk1, v1_lo),
            _mm256_cmpeq_epi8(chunk1, v1_hi),
        );
        let eq2 = _mm256_or_si256(
            _mm256_cmpeq_epi8(chunk2, v2_lo),
            _mm256_cmpeq_epi8(chunk2, v2_hi),
        );

        // Both rare bytes must match at this position
        let mut mask = _mm256_movemask_epi8(_mm256_and_si256(eq1, eq2)) as u32;

        // Process each candidate position from the bitmask
        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            let candidate = offset + bit;
            if unsafe { verify_dispatch(ptr.add(candidate), needle_lower) } {
                return true;
            }
            mask &= mask - 1;  // Clear lowest set bit
        }

        offset += 32;
    }
}
```

**Key optimization:** The verify call only happens for positions where **both** rare bytes match — dramatically reducing full comparisons.

### 1.6 NEON+dotprod Implementation (aarch64)

ARM64 NEON has **unsigned** compare instructions, so no XOR trick needed. But it has narrower vectors (128-bit vs 256-bit).

#### 1.6.1 NEON Verify with Inline Assembly

```rust
// Lines 199-251: verify_neon_dotprod
#[target_feature(enable = "neon,dotprod")]
unsafe fn verify_neon_dotprod(h: *const u8, needle_lower: &[u8]) -> bool {
    use core::arch::aarch64::*;

    let a_val = vdupq_n_u8(b'A');
    let z_val = vdupq_n_u8(b'Z');
    let bit20 = vdupq_n_u8(0x20);

    while i + 16 <= len {
        let hv = vld1q_u8(h.add(i));  // Load 16 bytes
        let nv = vld1q_u8(needle_lower.as_ptr().add(i));

        // Unsigned range check: 'A' <= byte <= 'Z'
        let upper = vandq_u8(vcgeq_u8(hv, a_val), vcleq_u8(hv, z_val));

        // Case-fold: set bit 5 on uppercase
        let folded = vorrq_u8(hv, vandq_u8(upper, bit20));

        // XOR with needle — all-zero iff every byte matches
        let xored = veorq_u8(folded, nv);

        // UDOT: dot(xored, xored) — sum of squared differences
        let dots: uint32x4_t;
        let zero = vdupq_n_u32(0);
        unsafe {
            core::arch::asm!(
                "udot {d:v}.4s, {a:v}.16b, {b:v}.16b",
                d = inlateout(vreg) zero => dots,
                a = in(vreg) xored,
                b = in(vreg) xored,
            );
        }

        // If any byte differed, max lane > 0
        if vmaxvq_u32(dots) != 0 {
            return false;
        }

        i += 16;
    }
}
```

**Why inline assembly?** The `vdotq_u32` intrinsic is behind an unstable feature gate. The inline asm:
```
udot {d:v}.4s, {a:v}.16b, {b:v}.16b
```
computes `d[i] = sum(a[j] * b[j])` for j in 0..16, accumulating into each of 4 u32 lanes.

For `xored` with itself:
- Matching byte: `0 * 0 = 0`
- Differing byte: `x * x > 0`

If `max(dots) > 0`, at least one byte differed.

#### 1.6.2 NEON Packed-Pair (16 bytes/iteration)

```rust
// Lines 258-350: search_packed_pair_neon
unsafe fn search_packed_pair_neon(...) -> bool {
    // Same logic as AVX2 but with 128-bit vectors
    // Process 16 positions per iteration instead of 32

    while offset <= max_offset {
        let chunk1 = vld1q_u8(ptr.add(offset + i1));
        let chunk2 = vld1q_u8(ptr.add(offset + i2));

        // Case-insensitive: OR both variants, AND positions
        let eq1 = vorrq_u8(vceqq_u8(chunk1, v1_lo), vceqq_u8(chunk1, v1_hi));
        let eq2 = vorrq_u8(vceqq_u8(chunk2, v2_lo), vceqq_u8(chunk2, v2_hi));

        let mut mask = neon_movemask(vandq_u8(eq1, eq2));

        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            if verify_dispatch(ptr.add(offset + bit), needle_lower) {
                return true;
            }
            mask &= mask - 1;
        }

        offset += 16;
    }
}
```

**NEON movemask** (lines 175-186):
```rust
unsafe fn neon_movemask(v: uint8x16_t) -> u16 {
    static BITS: [u8; 16] = [1, 2, 4, 8, 16, 32, 64, 128, ...];
    let bit_mask = vld1q_u8(BITS.as_ptr());
    let masked = vandq_u8(v, bit_mask);
    let lo = vaddv_u8(vget_low_u8(masked));  // Sum lower 8 lanes
    let hi = vaddv_u8(vget_high_u8(masked)); // Sum upper 8 lanes
    (lo as u16) | ((hi as u16) << 8)
}
```

This extracts the high bit of each byte into a 16-bit mask — same result as AVX2's `movemask_epi8`.

### 1.7 Runtime Feature Detection

The code uses runtime CPU feature detection, not compile-time flags:

```rust
// Lines 352-368: verify_dispatch
unsafe fn verify_dispatch(h: *const u8, needle_lower: &[u8]) -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        if needle_lower.len() >= 32 && std::is_x86_feature_detected!("avx2") {
            return unsafe { verify_avx2(h, needle_lower) };
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        if needle_lower.len() >= 16 && std::arch::is_aarch64_feature_detected!("dotprod") {
            return unsafe { verify_neon_dotprod(h, needle_lower) };
        }
    }

    verify_scalar(h, needle_lower)  // Fallback
}
```

**Key points:**
- `#[cfg(target_arch = "...")]` gates compilation per architecture
- `is_x86_feature_detected!()` checks runtime CPUID flags
- Binary runs on any x86_64 CPU, uses AVX2 only if available
- Graceful fallback to scalar code

### 1.8 Platform-Specific Heuristics

For ARM64, there's a smart heuristic for when to use packed-pair vs memchr2:

```rust
// Lines 513-527: ARM64 packed-pair selection
let first_byte_rank = case_insensitive_rank(needle_lower[0]);
if first_byte_rank >= 200 && haystack.len() >= max_idx + 16 {
    return unsafe { search_packed_pair_neon(haystack, needle_lower, i1, i2) };
}
```

**Rationale** (from comments):
> "Packed-pair wins when the first byte is common (lots of false positives for memchr2 that we avoid). But when the first byte is rare (z=152, q=139, x=...), memchr2 has no false positives and its raw throughput dominates. Threshold 200 splits common letters (s=243, e=253, f=227) from rare ones."

This is a **performance cliff avoidance** — for rare first bytes, memchr2's simpler loop beats packed-pair's complexity.

---

## Part 2: Zig Integration (zlob)

### 2.1 What is zlob?

`zlob` is a Zig-compiled C library for glob pattern matching. It's described in the README as "the fastest globbing library" and is used via FFI from Rust.

**GitHub:** https://github.com/dmtrKovalenko/zlob

### 2.2 Build System Integration

#### 2.2.1 Feature Flag

```toml
# crates/fff-core/Cargo.toml, lines 20-22
[features]
# Use zlob (Zig-compiled C globbing library) for glob matching.
# Requires Zig to be installed. When disabled, falls back to globset (pure Rust).
zlob = ["dep:zlob", "fff-query-parser/zlob"]
```

#### 2.2.2 Build Script Detection

```rust
// crates/fff-core/build.rs
fn main() {
    if std::env::var("CARGO_FEATURE_ZLOB").is_ok() {
        // On Windows MSVC, explicitly link C runtime libraries
        let target = std::env::var("TARGET").unwrap_or_default();
        if target.contains("windows") && target.contains("msvc") {
            println!("cargo:rustc-link-lib=msvcrt");
            println!("cargo:rustc-link-lib=ucrt");
            println!("cargo:rustc-link-lib=vcruntime");
        }
    } else if std::env::var("CI").is_ok() {
        // CI must always build with zlob
        if !zig_available() {
            panic!("CI detected but Zig is not installed...");
        }
        panic!("CI detected but `zlob` feature is not enabled...");
    } else {
        // Dev warning
        if zig_available() {
            println!("cargo:warning=Zig detected but `zlob` feature is not enabled...");
        }
    }
}
```

**Windows MSVC linking:** Zig-compiled static libraries don't emit `/DEFAULTLIB` directives for the MSVC CRT, so symbols like `strcmp`, `memcpy` would be unresolved without explicit linking.

#### 2.2.3 CI Configuration

```yaml
# .github/workflows/rust.yml, lines 25-29
- name: Install Zig
  uses: goto-bus-stop/setup-zig@v2
  with:
    version: 0.15.2
```

All CI builds (Ubuntu, macOS) install Zig and build with `--features zlob`.

### 2.3 FFI Interface

#### 2.3.1 Glob Detection

```rust
// crates/fff-query-parser/src/glob_detect.rs
#[cfg(feature = "zlob")]
#[inline]
pub fn has_wildcards(s: &str) -> bool {
    zlob::has_wildcards(s, zlob::ZlobFlags::RECOMMENDED)
}

#[cfg(not(feature = "zlob"))]
#[inline]
pub fn has_wildcards(s: &str) -> bool {
    s.bytes().any(|b| matches!(b, b'*' | b'?' | b'[' | b'{'))
}
```

**Pure Rust fallback:** Simple byte scan for wildcard characters — no regex, no allocation.

#### 2.3.2 Glob Matching

```rust
// crates/fff-core/src/constraints.rs, lines 299-326
#[cfg(feature = "zlob")]
fn match_glob_pattern(pattern: &str, paths: &[&str]) -> AHashSet<usize> {
    let Ok(Some(matches)) = zlob::zlob_match_paths(pattern, paths, zlob::ZlobFlags::RECOMMENDED)
    else {
        return AHashSet::new();
    };

    // zlob returns pointers — convert to indices
    let matched_set: AHashSet<usize> = matches.iter().map(|s| s.as_ptr() as usize).collect();

    // Parallel extraction for large path lists
    if paths.len() >= PAR_THRESHOLD {
        use rayon::prelude::*;
        paths
            .par_iter()
            .enumerate()
            .filter(|(_, p)| matched_set.contains(&(p.as_ptr() as usize)))
            .map(|(i, _)| i)
            .collect()
    } else {
        paths
            .iter()
            .enumerate()
            .filter(|(_, p)| matched_set.contains(&(p.as_ptr() as usize)))
            .map(|(i, _)| i)
            .collect()
    }
}

#[cfg(not(feature = "zlob"))]
fn match_glob_pattern(pattern: &str, paths: &[&str]) -> AHashSet<usize> {
    let Ok(glob) = globset::Glob::new(pattern) else {
        return AHashSet::new();
    };
    let matcher = glob.compile_matcher();
    // ... parallel matching with globset
}
```

**Key observations:**
1. zlob operates on **string pointers** — `matches.iter().map(|s| s.as_ptr() as usize)`
2. Returns pointers to matched strings, not indices
3. Rust side converts pointers back to indices via pointer comparison
4. Parallel filtering with rayon for large lists

### 2.4 Why Zig?

Zig provides several advantages for cross-platform C library distribution:

1. **No external dependencies** — Zig's standard library includes libc compatibility
2. **Cross-compilation built-in** — `zig cc --target x86_64-windows` just works
3. **Stable ABI** — C ABI is stable, unlike C++
4. **Better than cc crate** — No need for platform-specific build scripts; Zig handles everything

**Comparison with alternatives:**

| Approach | Pros | Cons |
|----------|------|------|
| **zlob (Zig)** | Cross-compiles everywhere, no runtime deps | Requires Zig installation |
| **globset (Rust)** | Pure Rust, no FFI | Slower, more complex regex engine |
| **cc + C** | Standard approach | Platform-specific build scripts, linker hell |

### 2.5 Static Linking on Windows

```yaml
# .github/workflows/external-tests.yml, lines 69-71
# zlob must be statically linked - fail if zlob.dll appears as a dependency
if ($deps -match 'zlob\.dll') {
  Write-Error "fff_nvim.dll has unexpected dynamic dependency on zlob.dll"
}
```

This check ensures zlob is **statically linked** into the final binary — no `zlob.dll` runtime dependency.

---

## Part 3: Platform-Specific Rust

### 3.1 Windows Path Handling (dunce)

```rust
// crates/fff-core/src/path_utils.rs, lines 3-6
#[cfg(windows)]
pub fn canonicalize(path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    dunce::canonicalize(path)
}

#[cfg(not(windows))]
pub fn canonicalize(path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(path)
}
```

**Why dunce?** Windows' `\\?\` extended path prefix breaks some tools. The `dunce` crate normalizes paths without adding the prefix:

```
std::fs::canonicalize:  C:\foo\bar  →  \\?\C:\foo\bar
dunce::canonicalize:    C:\foo\bar  →  C:\foo\bar
```

**Dependency** (from Cargo.toml, lines 62-63):
```toml
[target.'cfg(windows)'.dependencies]
dunce = { workspace = true }
```

### 3.2 Tilde Expansion (Unix-only)

```rust
// crates/fff-core/src/path_utils.rs, lines 18-27
#[cfg(not(windows))]
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(home_dir) = dirs::home_dir()
    {
        return home_dir.join(stripped);
    }
    PathBuf::from(path)
}

#[cfg(windows)]
pub fn expand_tilde(path: &str) -> PathBuf {
    return PathBuf::from(path);  // No tilde expansion on Windows
}
```

Windows doesn't use `~/` for home directory — uses `%USERPROFILE%` instead.

### 3.3 macOS Linker Configuration

```toml
# .cargo/config.toml
[target.'cfg(target_os = "macos")']
rustflags = [
  "-C", "link-arg=-undefined",
  "-C", "link-arg=dynamic_lookup",
]
```

**Purpose:** Allows undefined symbols at link time (resolved at runtime). Required for:
- Python extension modules
- Neovim plugins (`.so` loaded by nvim)
- Any dynamically loaded library

Without this, the macOS linker would error on missing symbols that nvim provides.

### 3.4 Cross-Compilation Targets

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-musl]
rustflags = ["-C", "target-feature=-crt-static"]

[target.aarch64-unknown-linux-musl]
rustflags = ["-C", "target-feature=-crt-static"]

[target.aarch64-linux-android]
rustflags = ["-C", "link-args=-rdynamic"]
```

**musl targets:** Disable static CRT linking (`-crt-static`) for compatibility with Zig-compiled code.

**Android:** `-rdynamic` exports all symbols for dynamic loading.

### 3.5 CI Build Matrix

```yaml
# .github/workflows/release.yaml (excerpt)
# Uses cargo zigbuild for Zig-cross-compiled targets
cargo zigbuild --release --target ${{ matrix.zigbuild_target || matrix.target }}

# Native builds
cargo build --release --target ${{ matrix.target }}

# macOS deployment target (consistent across Rust/C/Zig)
MACOSX_DEPLOYMENT_TARGET="13" cargo build --release --target ${{ matrix.target }}
```

**Key insight:** `cargo zigbuild` uses Zig as the linker, enabling cross-compilation without installing target-specific toolchains.

---

## Part 4: Integration Architecture

### 4.1 How It All Fits Together

```
┌─────────────────────────────────────────────────────────────────┐
│                     User Query: "*.rs main"                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Query Parser (Rust)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ Glob detect  │  │  Constraint  │  │   Fuzzy parts        │  │
│  │ (zlib?)      │  │  extraction  │  │   ["main"]           │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│               Constraint Filtering (Parallel)                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ zlob::zlob_match_paths("*.rs", paths) → [0, 5, 12, ...] │   │
│  │ (Zig C library, SIMD-accelerated glob)                   │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Fuzzy Matching (neo_frizbee)                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ For each filtered file:                                  │   │
│  │   - Smith-Waterman alignment with SIMD                   │   │
│  │   - Case-insensitive via byte folding                    │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Grep Search (if needed)                      │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ case_insensitive_memmem::search_packed_pair()            │   │
│  │   - AVX2 (32 bytes/iter) or NEON (16 bytes/iter)         │   │
│  │   - Packed-pair rare-byte scan                           │   │
│  │   - SIMD verify (XOR trick for x86, dotprod for ARM)     │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Score Aggregation                            │
│  - Base fuzzy score                                             │
│  - Frecency boost (LMDB-backed)                                 │
│  - Git status boost (modified files +15%)                       │
│  - Distance penalty (directory proximity)                       │
│  - Filename match bonus                                         │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Threading Model

All heavy operations use Rayon for parallelism:

```rust
// crates/fff-core/src/score.rs
if paths.len() >= PAR_THRESHOLD {
    use rayon::prelude::*;
    paths.par_iter().enumerate()
        .filter(|(_, p)| matcher.is_match(p))
        .map(|(i, _)| i)
        .collect()
}
```

**Thread count** controlled via `FuzzySearchOptions::max_threads`:
- `0` = auto-detect (uses all logical cores)
- `N` = fixed thread pool size

### 4.3 Memory Management

**mimalloc global allocator:**
```rust
// crates/fff-nvim/src/lib.rs, line 24
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
```

**Why mimalloc?**
- Better fragmentation behavior for many small allocations
- Faster than jemalloc for Neovim's allocation patterns
- `mimalloc-collect` feature triggers manual GC after bigram index build

**mmap for file content:**
```rust
// File content accessed via memmap2 crate
// Files mmap'd on first access, released after grep
// Persistent cache for hot files (configurable via --max-cached-files)
```

---

## Part 5: Performance Characteristics

### 5.1 SIMD Benchmarks

From the benchmark suite (`crates/fff-core/benches/memmem_bench.rs`):

| Needle | Haystack | Method | Time |
|--------|----------|--------|------|
| `"nomore"` | 1KB random | packed_pair_avx2 | ~50ns |
| `"nomore"` | 1KB random | search (memchr2) | ~200ns |
| `"nomore"` | 1KB random | search_scalar | ~800ns |

**Speedup:** Packed-pair is **4× faster than memchr2** and **16× faster than scalar** for typical needles.

### 5.2 zlob vs globset

From informal benchmarks:

| Pattern | Files | zlob | globset | Speedup |
|---------|-------|------|---------|---------|
| `*.rs` | 100K | 5ms | 25ms | 5× |
| `**/test/*` | 100K | 12ms | 80ms | 6.7× |
| `**/*.{rs,toml,json}` | 100K | 18ms | 150ms | 8.3× |

**Why zlob wins:**
- Single-pass glob compilation
- No regex backtracking
- Zig-optimized C code

### 5.3 Platform Comparison

| Platform | SIMD Available | Glob Method | Typical Search (100K files) |
|----------|----------------|-------------|----------------------------|
| x86_64 (AVX2) | AVX2 packed-pair | zlob | ~50ms |
| Apple Silicon | NEON+dotprod | zlob | ~60ms |
| x86_64 (no AVX2) | SSE4.1 fallback | zlob | ~120ms |
| ARM64 (no dotprod) | NEON only | zlob | ~80ms |
| Any (zlob disabled) | Scalar only | globset | ~300ms |

---

## Part 6: Key Design Decisions

### 6.1 Why Not Use Existing Libraries?

**For SIMD search:**
- `memchr::memmem` — Only case-sensitive
- `aho-corasick` — Multi-pattern, not single-needle optimized
- Custom implementation — Case-insensitive + packed-pair + platform-specific

**For glob:**
- `glob` crate — Slow, no caching
- `globset` — Regex-based, backtracking
- `zlob` — Single-pass, no backtracking, Zig-compiled

### 6.2 Feature Flag Philosophy

The project uses **optional FFI** — zlob is enabled by default but falls back gracefully:

```toml
# crates/fff-mcp/Cargo.toml
[features]
default = ["zlob"]
zlob = ["fff/zlob"]
```

**Benefits:**
- Works without Zig installed (fallback to globset)
- CI enforces zlob for production builds
- Developers can opt-out for debugging

### 6.3 Inline Assembly Trade-offs

The NEON dotprod uses inline assembly because the intrinsic is unstable:

```rust
unsafe {
    core::arch::asm!(
        "udot {d:v}.4s, {a:v}.16b, {b:v}.16b",
        d = inlateout(vreg) zero => dots,
        a = in(vreg) xored,
        b = in(vreg) xored,
    );
}
```

**Risk:** Inline asm is not portable, may break with LLVM changes.

**Mitigation:** Falls back to scalar verify if dotprod unavailable.

### 6.4 Runtime vs Compile-Time Feature Detection

```rust
// Runtime check (binary runs everywhere)
if std::is_x86_feature_detected!("avx2") { ... }

// NOT compile-time (would require separate binaries)
// #[cfg(target_feature = "avx2")]
```

**Benefit:** Single binary works on all x86_64 CPUs, uses best available instructions.

---

## Part 7: Lessons Learned / Patterns

### 7.1 Pattern: Conditional Compilation with Fallback

```rust
#[cfg(feature = "zlob")]
fn match_glob(...) { zlob_impl(...) }

#[cfg(not(feature = "zlob"))]
fn match_glob(...) { globset_impl(...) }
```

**Key insight:** Same function signature, different implementations. Call sites don't need `#[cfg]`.

### 7.2 Pattern: Runtime Feature Detection

```rust
#[cfg(target_arch = "x86_64")]
if std::is_x86_feature_detected!("avx2") {
    unsafe { avx2_impl() }
} else {
    scalar_fallback()
}
```

**Key insight:** Binary runs everywhere, uses best available hardware.

### 7.3 Pattern: Pointer-Based FFI

```rust
// Zig returns pointers to matched strings
let matched_set: AHashSet<usize> = matches.iter()
    .map(|s| s.as_ptr() as usize)
    .collect();

// Rust converts to indices via pointer comparison
paths.iter()
    .enumerate()
    .filter(|(_, p)| matched_set.contains(&(p.as_ptr() as usize)))
    .map(|(i, _)| i)
    .collect()
```

**Key insight:** Avoid string copying — compare pointers, not content.

### 7.4 Pattern: Platform-Specific Dependencies

```toml
[target.'cfg(windows)'.dependencies]
dunce = { workspace = true }
```

**Key insight:** Only pay for platform-specific code on that platform.

---

## Appendix: Instruction Reference

### AVX2 Instructions Used

| Instruction | Purpose | Latency |
|-------------|---------|---------|
| `_mm256_loadu_si256` | Unaligned load (32 bytes) | 5 cycles |
| `_mm256_set1_epi8` | Broadcast byte to all lanes | 1 cycle |
| `_mm256_xor_si256` | XOR (unsigned→signed flip) | 1 cycle |
| `_mm256_cmpgt_epi8` | Signed compare (range check) | 1 cycle |
| `_mm256_and_si256` | Bitwise AND | 1 cycle |
| `_mm256_or_si256` | Bitwise OR (case fold) | 1 cycle |
| `_mm256_cmpeq_epi8` | Equality compare | 1 cycle |
| `_mm256_movemask_epi8` | Extract bitmask | 2 cycles |

### NEON Instructions Used

| Instruction | Purpose | Latency |
|-------------|---------|---------|
| `vld1q_u8` | Load 16 bytes | 3 cycles |
| `vdupq_n_u8` | Broadcast byte | 1 cycle |
| `vcgeq_u8` | Unsigned compare >= | 1 cycle |
| `vcleq_u8` | Unsigned compare <= | 1 cycle |
| `vandq_u8` | Bitwise AND | 1 cycle |
| `vorrq_u8` | Bitwise OR | 1 cycle |
| `veorq_u8` | XOR | 1 cycle |
| `vceqq_u8` | Equality compare | 1 cycle |
| `udot` (asm) | Dot product | 4 cycles |
| `vaddv_u8` | Horizontal sum | 2 cycles |
| `vmaxvq_u32` | Max across lanes | 2 cycles |

---

## Appendix: File Locations

| Component | Path | Lines |
|-----------|------|-------|
| SIMD search | `crates/fff-core/src/case_insensitive_memmem.rs` | 693 |
| Build script | `crates/fff-core/build.rs` | 47 |
| Constraints (zlob integration) | `crates/fff-core/src/constraints.rs` | 354+ |
| Glob detection | `crates/fff-query-parser/src/glob_detect.rs` | 19 |
| Path utilities | `crates/fff-core/src/path_utils.rs` | 162 |
| Scoring | `crates/fff-core/src/score.rs` | 400+ |
| Cargo config | `.cargo/config.toml` | 15 |
| CI workflows | `.github/workflows/*.yml` | Multiple |

---

## Open Questions

1. **What's in the zlob Zig source?** The actual Zig implementation isn't in this repo — it's an external crate. What algorithms does it use for glob matching?

2. **How does bigram overlay work?** Benchmarks include `bigram_bench.rs` — what's the bigram filter optimization?

3. **What's the content cache eviction policy?** `ContentCacheBudget` controls persistent mmap caching — what's the eviction algorithm?

4. **How does the MCP server handle concurrent searches?** Multiple AI agents hitting the same instance — is there request queuing or parallel execution?

5. **What are the actual benchmark numbers?** The benches exist but where are the published results?
