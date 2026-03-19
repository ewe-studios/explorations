---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/src.containers/libkrunfw
explored_at: 2026-03-19
---

# libkrunfw Deep Dive

## Purpose

libkrunfw is a library that bundles a Linux kernel into a dynamic library format, allowing libkrun to load and execute the kernel without additional processing or file parsing.

## Design Philosophy

```
Traditional Approach:
┌──────────────┐    ┌─────────────┐    ┌──────────────┐
│ VMM Process  │ -> │ Read Kernel │ -> │ Parse Kernel │
│              │    │   File      │    │   Format     │
└──────────────┘    └─────────────┘    └──────────────┘

libkrunfw Approach:
┌──────────────┐    ┌─────────────┐
│ VMM Process  │ -> │ Kernel      │
│              │    │ Already     │
│              │    │ Mapped      │
└──────────────┘    └─────────────┘
     │
     ▼
┌──────────────┐
│ libkrunfw.so │ (Kernel embedded as data sections)
└──────────────┘
```

## How It Works

1. **Build Time**:
   - Linux kernel is compiled with specific configuration
   - Kernel image is converted into object file sections
   - Sections are linked into the libkrunfw shared library

2. **Runtime**:
   - Dynamic linker maps libkrunfw.so into process space
   - libkrun injects the kernel sections directly into guest memory
   - No kernel file parsing or format processing required

## Kernel Configuration

### Constraints

- `CONFIG_NR_CPUS=8`: Maximum 8 CPUs to save memory
- If VM has more CPUs, only first 8 are initialized

### Patches

The `patches/` directory contains kernel patches for:
- libkrun-specific functionality
- TSI (Transparent Socket Impersonation) support
- virtio device optimizations

## Building

### Linux (Generic)

```bash
# Dependencies
# - Kernel build toolchain
# - Python 3
# - pyelftools (python3-pyelftools)

make
sudo make install
```

### Linux (SEV Variant)

```bash
make SEV=1
sudo make SEV=1 install
```

### macOS

Building on macOS requires running Linux in a VM:

```bash
# Prerequisites: Install krunvm and dependencies
./build_on_krunvm.sh    # Creates Linux VM with build environment
make
```

**Build Environment Options**:
```bash
# Fedora-based (default)
./build_on_krunvm.sh

# Debian-based
BUILDER=debian ./build_on_krunvm.sh
```

## Library Structure

```
libkrunfw.so
├── .text sections      # Kernel code
├── .data sections      # Kernel data
├── .rodata sections    # Kernel constants
├── .init sections      # Initialization code
└── Metadata sections   # Kernel layout info
```

## License Implications

**Important**: libkrunfw does NOT execute kernel code - it only stores it.

| Component | License | Implications |
|-----------|---------|--------------|
| Linux kernel | GPL-2.0-only | Source must be provided |
| patches/*.patch | GPL-2.0-only | Derivative of kernel |
| Library code | LGPL-2.1-only | Can link without GPL |
| Auto-generated code | LGPL-2.1-only | Build artifacts |

**Distribution Requirements**:
- Must provide kernel source code
- Must provide library source code
- Programs linking against libkrunfw are NOT required to be GPL

## Integration with libkrun

```rust
// Conceptual flow in libkrun
fn load_kernel() {
    // 1. Dynamic linker has already mapped libkrunfw.so
    // 2. Find kernel sections in library
    let kernel_sections = find_kernel_sections();

    // 3. Copy sections directly to guest memory
    for section in kernel_sections {
        guest_memory.write(section.addr, section.data);
    }

    // 4. Set up boot parameters
    setup_boot_params();

    // 5. Jump to kernel entry point
    jump_to_kernel();
}
```

## Variants

| Variant | Library | Use Case |
|---------|---------|----------|
| Generic | libkrunfw.so | Standard KVM |
| SEV | libkrunfw-sev.so | AMD SEV encryption |
| TDX | libkrunfw-tdx.so | Intel TDX encryption |
| EFI | Bundled with libkrun-efi | macOS UEFI boot |

## Technical Details

### Section Naming

Kernel sections are embedded with specific naming conventions:
- `.krunfw.kernel.*` - Kernel code and data
- `.krunfw.metadata` - Layout and version information

### Memory Layout

```
Guest Physical Memory Layout:
0x00000000 - 0x0009FFFF  | Real Mode / BIOS Area
0x000A0000 - 0x000FFFFF | Reserved (VGA, etc.)
0x00100000 - 0xXXXXXXXX | Kernel (from libkrunfw)
...
```

## Version Information

The library exposes version information through:
- `soname` versioning
- Embedded metadata section
- Exported symbols for version queries

## References

- [libkrunfw README](../../src.containers/libkrunfw/README.md)
- [libkrun README](../../src.containers/libkrun/README.md)
