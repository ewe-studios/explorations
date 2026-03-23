---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.matrix-web/element-x-ios
repository: https://github.com/element-hq/element-x-ios
explored_at: 2026-03-23
language: Swift
---

# Sub-Project Exploration: Element X iOS

## Overview

Element X iOS is the next-generation Matrix client for iOS, built with SwiftUI and powered by the matrix-rust-sdk (via matrix-rust-components-swift). It represents a complete rewrite of the iOS client with modern Swift concurrency, SwiftUI, and a shared Rust core for Matrix protocol operations.

## Architecture

```mermaid
graph TD
    App[ElementX App] --> RustSDK[matrix-rust-components-swift<br/>Swift Bindings]
    App --> CompoundiOS[compound-ios<br/>Design System]
    App --> NSE[Notification Service Extension]
    App --> IntTests[IntegrationTests]
    RustSDK --> HS[Matrix Homeserver]

    subgraph "App Structure"
        Screens[Screens/]
        Services[Services/]
        FlowCoordinators[FlowCoordinators/]
        UITests[UITests/]
    end
```

### Structure

```
element-x-ios/
├── ElementX/               # Main application target
├── ElementX.xcodeproj/     # Xcode project
├── NSE/                    # Notification Service Extension
├── IntegrationTests/       # Integration test target
├── DevelopmentAssets/       # Debug/dev assets
├── Enterprise/             # Enterprise features
├── ci_scripts/             # CI/CD scripts
├── fastlane/               # App Store deployment
├── docs/                   # Documentation
├── app.yml                 # App configuration
└── localazy.json           # Translation management
```

## Key Insights

- **matrix-rust-components-swift** (XCFramework) provides all Matrix SDK functionality
- SwiftUI-first UI with coordinator pattern for navigation
- Notification Service Extension for background push handling
- Enterprise module for commercial features
- Fastlane for App Store automation
- Localazy for translation management
- Codecov for test coverage tracking
- Dangerfile.swift for PR automation
- The app config in `app.yml` defines bundle identifiers and display names
