---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.matrix-web/element-ios
repository: https://github.com/vector-im/element-ios
explored_at: 2026-03-23
language: Swift, Objective-C
---

# Sub-Project Exploration: Element iOS (Legacy)

## Overview

Element iOS is the legacy Matrix client for iOS, built with Swift and Objective-C using the matrix-ios-sdk. It is being superseded by Element X iOS, which uses the matrix-rust-sdk via Swift bindings. This project remains for reference and existing deployments.

## Architecture

```mermaid
graph TD
    App[Element iOS App] --> MatrixSDK[matrix-ios-sdk]
    App --> CommonKit[CommonKit]
    App --> DesignKit[DesignKit]
    App --> BroadcastExt[BroadcastUploadExtension]
    MatrixSDK --> HS[Matrix Homeserver]
```

### Structure

```
element-ios/
├── Config/                 # Build configuration
├── CommonKit/              # Shared utilities
├── DesignKit/              # Design system
├── BroadcastUploadExtension/ # Screen sharing extension
├── matrix-ios-sdk/         # Matrix protocol SDK (submodule)
├── docs/                   # Documentation
├── fastlane/               # App Store deployment
├── changelog.d/            # Changelog fragments
└── Podfile                 # CocoaPods dependencies
```

## Key Insights

- Legacy project, succeeded by Element X iOS
- Mixed Swift/Objective-C codebase
- CocoaPods for dependency management (predates SPM adoption)
- matrix-ios-sdk included as a git submodule
- Fastlane for App Store deployment automation
- Brewfile for development tool management
