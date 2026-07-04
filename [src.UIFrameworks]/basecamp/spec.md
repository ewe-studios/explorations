# Basecamp/Hotwire Ecosystem -- Documentation Spec

## Source Codebase

- **Stimulus**: v3.2.2 -- https://github.com/hotwired/stimulus -- MIT License -- 37signals LLC
- **Turbo**: v8.0.13 -- https://github.com/hotwired/turbo -- MIT License -- 37signals LLC
- **hotwire-native-ios**: Swift SDK -- https://github.com/hotwired/hotwire-native-ios -- MIT
- **hotwire-native-android**: Kotlin SDK (Maven: `dev.hotwire:core`, `dev.hotwire:navigation-fragments`) -- https://github.com/hotwired/hotwire-native-android -- MIT
- **stimulus-rails**: v1.3.4 -- https://github.com/hotwired/stimulus-rails -- Rails >= 6.0
- **turbo-rails**: v2.0.17 -- https://github.com/hotwired/turbo-rails -- Rails >= 7.1
- **hotwire-rails**: v0.1.3 -- https://github.com/hotwired/hotwire-rails
- **bridge-components**: v0.8.3 (`@joemasilotti/bridge-components`) -- npm, peer deps on Stimulus >= 3.2 + hotwire-native-bridge >= 1.2
- **Source locations**:
  - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/BasecampApps/stimulus/`
  - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/BasecampApps/turbo/`
  - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/BasecampApps/hotwire-native-ios/`
  - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/BasecampApps/hotwire-native-android/`
  - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/BasecampApps/stimulus-rails/`
  - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/BasecampApps/turbo-rails/`
  - `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/BasecampApps/bridge-components/`

## What This Ecosystem Is

The Hotwire (HTML over the wire) ecosystem is a collection of JavaScript, Swift, and Kotlin libraries that enable building interactive web and native applications with minimal client-side JavaScript. Stimulus provides lightweight behavioral wiring via HTML attributes. Turbo provides SPA-like navigation, frame-based partial updates, and WebSocket-driven live streams. The native SDKs (iOS/Android) wrap web views with native navigation and a bidirectional bridge protocol. Rails gems integrate everything server-side. Bridge-components provides 16 Stimulus controllers with matching iOS/Android native implementations for platform features like alerts, haptics, and menus.

## Documentation Goal

A reader should understand:

1. How Stimulus wires controllers to DOM elements via data attributes
2. How Stimulus observers detect mutations and drive lifecycle callbacks
3. How the blessing system generates typed accessors at runtime
4. How Turbo Drive intercepts navigation and manages the LRU snapshot cache
5. How Turbo Frames and Streams enable partial-page updates
6. How the native iOS SDK wraps WKWebView with native navigation stacks
7. How the native Android SDK shares a WebView across fragments
8. How the bridge protocol enables bidirectional web-native communication
9. How Rails gems integrate Hotwire with ActionCable and ActiveRecord
10. How bridge-components provide cross-platform component parity
11. Cross-cutting patterns: observers, delegates, sessions, and path configuration

## Documentation Structure

```
basecamp/
├── spec.md                     ← This tracker
├── markdown/
│   ├── README.md               ← Index
│   ├── 00-overview.md          ← Ecosystem overview, philosophy, architecture
│   ├── 01-stimulus-core.md     ← Stimulus: Application, Module, Scope, Context
│   ├── 02-stimulus-observers.md← Stimulus: Observer chain (MutationObserver hierarchy)
│   ├── 03-stimulus-blessing.md ← Stimulus: Blessing system, shadow constructors
│   ├── 04-turbo-drive.md       ← Turbo: Drive, Session, Visit, Snapshot cache
│   ├── 05-turbo-frames-streams.md ← Turbo: Frames, Streams, custom elements
│   ├── 06-native-ios.md        ← hotwire-native-ios: Session, Navigator, Visitable
│   ├── 07-native-android.md    ← hotwire-native-android: Session, Navigator, Fragments
│   ├── 08-bridge-protocol.md   ← Bridge protocol (both platforms)
│   ├── 09-rails-integration.md ← stimulus-rails, turbo-rails, hotwire-rails
│   ├── 10-bridge-components.md ← 16 web controllers + iOS/Android native components
│   └── 11-cross-cutting.md     ← Patterns across the ecosystem
├── html/                       ← Generated HTML
│   ├── index.html
│   ├── styles.css
│   └── *.html
└── build.py                    ← Shared markdown→HTML build script
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Create spec.md | DONE |
| 2 | Create markdown/README.md | DONE |
| 3 | Create 00-overview.md | DONE |
| 4 | Create 01-stimulus-core.md | DONE |
| 5 | Create 02-stimulus-observers.md | DONE |
| 6 | Create 03-stimulus-blessing.md | DONE |
| 7 | Create 04-turbo-drive.md | DONE |
| 8 | Create 05-turbo-frames-streams.md | DONE |
| 9 | Create 06-native-ios.md | DONE |
| 10 | Create 07-native-android.md | DONE |
| 11 | Create 08-bridge-protocol.md | DONE |
| 12 | Create 09-rails-integration.md | DONE |
| 13 | Create 10-bridge-components.md | DONE |
| 14 | Create 11-cross-cutting.md | DONE |
| 15 | Copy build.py | DONE |
| 16 | Grandfather review | DONE |

## Build System

```bash
cd basecamp && python3 build.py basecamp
# or from repo root:
python3 build.py "[src.UIFrameworks]/basecamp"
```

Dependencies: Python 3.12+ stdlib only. Zero external packages.

## Quality Requirements (Iron Rules)

1. Every concept grounded in actual source code with file path references
2. First paragraph states core concept clearly
3. Non-overly-complex sentences; one idea per sentence
4. Minimum 2 mermaid diagrams per document
5. Minimum 3 code snippets per document with file path references
6. Minimum 1 Aha moment per document
7. Cross-references between documents
8. Link to GitHub repositories
9. Generated HTML with navigation, theme toggle, mermaid rendering
10. Grandfather review: names, numbers, flows, coverage all verified

## Expected Outcome

A developer can read this documentation and understand the entire Hotwire ecosystem without reading source code. They can wire Stimulus controllers, configure Turbo navigation, integrate native SDKs, and build bridge components.

## Resume Point

All files are in `/home/darkvoid/Boxxed/@dev/repo-expolorations/[src.UIFrameworks]/basecamp/`. Run grandfather review by grep-ing every name in docs against source. Verify every constant/default against source files.
