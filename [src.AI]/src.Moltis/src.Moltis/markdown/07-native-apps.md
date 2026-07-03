# Native Apps: iOS and macOS

Moltis includes native Swift apps for iOS (with Live Activity widget) and macOS (with multi-pane UI), connected to the Rust server via a Swift bridge.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/apps/`

## iOS App

```
apps/ios/
├── MoltisApp.swift              ← App entry point
├── Sources/                     ← Swift source files
├── Widget/                      ← Live Activity widget
│   ├── MoltisLiveActivity.swift
│   ├── MoltisWidgetBundle.swift
│   ├── IntentCardView.swift
│   └── Info.plist
├── GraphQL/                     ← GraphQL client
├── Assets.xcassets/             ← App assets
├── project.yml                  ← XcodeGen project
├── CHANGELOG.md
├── CLAUDE.md
├── Info.plist
├── Moltis.entitlements
├── .swiftlint.yml
└── Tests/                       ← Unit tests
```

The iOS app features:
- **Live Activity widget** — Real-time updates on the Lock Screen and Dynamic Island
- **GraphQL client** — Communicates with the Moltis server via GraphQL
- **SwiftLint** — Code quality enforcement

**Source:** `apps/ios/`.

## macOS App

```
apps/macos/
├── Sources/
│   ├── MoltisApp.swift          ← App entry point
│   ├── ContentView.swift        ← Main UI
│   ├── ChatModels.swift         ← Chat data models
│   ├── ChatStore.swift          ← Chat state management
│   ├── ConfigurationPane.swift  ← Settings → Configuration
│   ├── HttpdPane.swift          ← HTTP daemon status
│   ├── LogsPane.swift           ← Log viewer
│   ├── MessageBubbleView.swift  ← Chat message bubbles
│   ├── MoltisClient.swift       ← Server client
│   ├── MoltisEditorField.swift  ← Editor component
│   ├── NetworkAuditPane.swift   ← Network audit view
│   ├── NetworkAuditStore.swift  ← Network audit state
│   ├── OnboardingView.swift     ← Onboarding flow
│   ├── OnboardingStep.swift     ← Onboarding steps
│   ├── OnboardingState.swift    ← Onboarding state
│   ├── ProviderCardView.swift   ← Provider cards
│   ├── ProviderConfigForm.swift ← Provider configuration
│   ├── ProviderStore.swift      ← Provider state
│   ├── AppSettings.swift        ← App settings
│   ├── SettingsView.swift       ← Settings UI
│   ├── SettingsItemViews.swift  ← Settings items
│   ├── SettingsSectionContent+Environment.swift
│   ├── SettingsSectionContent+Sandboxes.swift
│   ├── SettingsSectionContent.swift
│   ├── FormComponents.swift     ← Form UI components
│   ├── Bridging-Header.h        ← Rust-Swift FFI header
│   └── LogStore.swift           ← Log state
├── Tests/                       ← Unit tests
├── UITests/                     ← UI tests
├── Assets.xcassets/             ← App assets
├── project.yml                  ← XcodeGen project
├── CHANGELOG.md
├── CLAUDE.md
├── README.md
├── Moltis.entitlements
├── .swiftlint.yml
└── Tests/MoltisTests.swift
```

The macOS app features:
- **Multi-pane UI** — Chat, Configuration, HTTP Daemon, Logs, Network Audit
- **Provider management** — Configure and monitor LLM providers
- **Settings** — Environment, Sandboxes, and general settings
- **Onboarding flow** — First-time setup wizard
- **SwiftLint** — Code quality enforcement

**Source:** `apps/macos/`.

## Swift Bridge

The Rust-Swift FFI is handled by the `swift-bridge` crate:

```
crates/swift-bridge/            ← Rust-Swift FFI layer
```

This crate enables the native apps to call Rust functions directly without going through the HTTP API.

**Source:** `crates/swift-bridge/src/`.

## Next

Continue to [08-security](08-security.md) for the security model.
