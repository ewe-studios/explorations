---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.matrix-web/matrix-rich-text-editor-swift
repository: https://github.com/element-hq/matrix-rich-text-editor-swift
explored_at: 2026-03-23
language: Swift
---

# Sub-Project Exploration: Matrix Rich Text Editor Swift

## Overview

This is the Swift Package Manager distribution package for the Matrix Rich Text Editor's iOS bindings. It wraps the Uniffi-generated Swift bindings from the Rust core (`wysiwyg-ffi`) into a distributable SPM package that Element X iOS and other Swift clients can consume.

## Structure

```
matrix-rich-text-editor-swift/
├── Sources/                # Swift source wrapping FFI bindings
├── Package.swift           # SPM manifest (checksum-verified XCFramework)
├── CONTRIBUTING.md
├── LICENSE
└── README.md
```

## Key Insights

- Distribution wrapper, not the source of truth (that is matrix-rich-text-editor)
- Package.swift references a specific version (2.38.3) with checksum verification
- XCFramework binary distribution for the compiled Rust FFI layer
- Swift 5.7+ tools version
- Consumed by Element X iOS for message composition
