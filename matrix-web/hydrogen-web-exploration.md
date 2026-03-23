---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.matrix-web/hydrogen-web
repository: https://github.com/vector-im/hydrogen-web
explored_at: 2026-03-23
language: TypeScript, JavaScript
---

# Sub-Project Exploration: Hydrogen Web

## Overview

Hydrogen is a lightweight, performance-focused Matrix web client designed to minimize RAM usage by leveraging IndexedDB for data storage and offloading as much processing as possible from main memory. Version 0.5.1, it targets low-end devices and constrained environments where Element Web's resource usage is prohibitive.

Hydrogen also provides a reusable SDK (`hydrogen-view-sdk`) that powers embedded clients like Chatterbox.

## Architecture

```mermaid
graph TD
    User[Browser] --> HW[Hydrogen Web<br/>Vite App]
    HW --> IDB[(IndexedDB<br/>Primary Storage)]
    HW --> HS[Matrix Homeserver]

    subgraph "Architecture"
        Platform[platform/<br/>Platform Abstraction]
        Matrix[src/matrix/<br/>Matrix Protocol]
        Domain[src/domain/<br/>ViewModels]
        View[src/platform/web/<br/>Web Views]
    end

    SDK[hydrogen-view-sdk] --> Matrix
    Chatterbox[chatterbox] --> SDK
```

### Structure

```
hydrogen-web/
├── src/
│   ├── matrix/             # Matrix protocol implementation
│   ├── domain/             # ViewModel layer
│   ├── platform/
│   │   └── web/            # Web platform (DOM, IndexedDB)
│   └── lib/                # Shared library code
├── scripts/
│   └── sdk/                # SDK build scripts
├── prototypes/             # Experimental features
├── doc/                    # Documentation
├── docker/                 # Docker deployment
├── playwright/             # E2E tests
└── package.json
```

## Key Insights

- **IndexedDB-first design** keeps memory footprint minimal by not holding full sync state in RAM
- Provides `hydrogen-view-sdk` as an npm package for embedding Matrix in other applications
- MVVM architecture (ViewModel + View separation) enables platform portability
- Vite-based build system
- Used as the foundation for Chatterbox (embeddable chat widget)
- Lighter alternative to Element Web for resource-constrained environments
- Node 15+ requirement
