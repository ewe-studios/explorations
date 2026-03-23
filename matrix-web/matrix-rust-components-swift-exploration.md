---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.matrix-web/matrix-rust-components-swift
repository: https://github.com/element-hq/matrix-rust-components-swift
explored_at: 2026-03-23
language: Swift
---

# Sub-Project Exploration: Matrix Rust Components Swift

## Overview

This is the Swift Package Manager distribution package for the matrix-rust-sdk's iOS bindings. It packages the Uniffi-generated Swift bindings from the Rust Matrix SDK into an XCFramework that Element X iOS can consume via SPM.

## Structure

```
matrix-rust-components-swift/
├── Sources/                # Swift wrapper sources
├── Tools/                  # Build/packaging tools
├── Package.swift           # SPM manifest (version 25.03.21, checksum verified)
├── LICENSE
└── README.md
```

## Key Insights

- Binary distribution of matrix-rust-sdk for iOS
- Version 25.03.21 (date-based versioning: 2025-03-21)
- Swift 5.9+ tools version
- XCFramework with checksum verification for integrity
- Critical dependency for Element X iOS - all Matrix protocol operations flow through this
- The `Tools/` directory likely contains scripts for building the XCFramework from the Rust source
