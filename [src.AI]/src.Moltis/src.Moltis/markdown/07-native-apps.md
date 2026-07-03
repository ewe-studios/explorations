# Native Apps: iOS and macOS Swift Apps

Moltis includes native Swift apps for iOS and macOS, connected to the Rust server via the `swift-bridge` crate. The apps use `@StateObject` for state management and communicate with the Rust backend through FFI bridges.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/apps/`

## iOS App

**Source:** `apps/ios/Sources/MoltisApp.swift`

### Entry Point

```swift
// MoltisApp.swift:3-42
@main
struct MoltisApp: App {
    @Environment(\.scenePhase) private var scenePhase

    @StateObject private var authManager = AuthManager()
    @StateObject private var connectionStore = ConnectionStore()
    @StateObject private var settingsStore = SettingsStore()
    @StateObject private var locationSharingStore = LocationSharingStore()

    var body: some Scene {
        WindowGroup {
            Group {
                if authManager.activeServer != nil {
                    ContentView()
                } else {
                    ConnectView()
                }
            }
            .environmentObject(authManager)
            .environmentObject(connectionStore)
            .environmentObject(settingsStore)
            .environmentObject(locationSharingStore)
            .task {
                authManager.loadSavedServers()
                locationSharingStore.configure(connectionStore: connectionStore)
                locationSharingStore.setAppIsActive(scenePhase == .active)
                if let server = authManager.activeServer {
                    await connectionStore.connect(to: server, authManager: authManager)
                }
                locationSharingStore.handleConnectionStateChange()
            }
            .onChange(of: scenePhase) { _, newPhase in
                locationSharingStore.setAppIsActive(newPhase == .active)
            }
            .onChange(of: connectionStore.state) { _, _ in
                locationSharingStore.handleConnectionStateChange()
            }
        }
    }
}
```

### State Objects

| Object | Purpose |
|--------|---------|
| `AuthManager` | Server authentication, saved servers |
| `ConnectionStore` | Server connection state, GraphQL client |
| `SettingsStore` | User settings |
| `LocationSharingStore` | Location sharing configuration |

### Lifecycle

1. On launch: `authManager.loadSavedServers()`
2. If active server exists → `ContentView()` (chat UI)
3. If no active server → `ConnectView()` (server connection UI)
4. On scene phase change: update location sharing active state
5. On connection state change: handle location sharing

### Live Activity Widget

**Source:** `apps/ios/Widget/MoltisLiveActivity.swift`

```swift
// MoltisLiveActivity.swift
struct MoltisLiveActivity: Widget {
    // Lock Screen and Dynamic Island live activity
    // Shows real-time agent status updates
}
```

### GraphQL Client

**Source:** `apps/ios/GraphQL/`

GraphQL client for communicating with the Moltis server.

### Widget Extension

**Source:** `apps/ios/Widget/`

| File | Purpose |
|------|---------|
| `MoltisLiveActivity.swift` | Live Activity widget |
| `MoltisWidgetBundle.swift` | Widget bundle entry |
| `IntentCardView.swift` | Intent card view |
| `Info.plist` | Widget configuration |

## macOS App

**Source:** `apps/macos/Sources/MoltisApp.swift`

### Entry Point

```swift
// MoltisApp.swift:16-51
@main
struct MoltisApp: App {
    @StateObject private var settings: AppSettings
    @StateObject private var chatStore: ChatStore
    @StateObject private var onboardingState: OnboardingState
    @StateObject private var providerStore: ProviderStore
    @StateObject private var logStore: LogStore
    @StateObject private var networkAuditStore: NetworkAuditStore

    init() {
        let settings = AppSettings()
        let onboardingState = OnboardingState()
        let logStore = LogStore()
        let networkAuditStore = NetworkAuditStore()

        // Install Rust→Swift log bridge before any FFI calls
        MoltisClient.installLogCallback(logStore: logStore)

        let providerStore = ProviderStore(logStore: logStore)
        let chatStore = ChatStore(
            settings: settings, providerStore: providerStore, logStore: logStore
        )

        // Install Rust→Swift session event bridge (shares bus with gateway)
        MoltisClient.installSessionEventCallback(chatStore: chatStore)

        // Install Rust→Swift network audit bridge
        MoltisClient.installNetworkAuditCallback(store: networkAuditStore)

        _settings = StateObject(wrappedValue: settings)
        _chatStore = StateObject(wrappedValue: chatStore)
        _onboardingState = StateObject(wrappedValue: onboardingState)
        _providerStore = StateObject(wrappedValue: providerStore)
        _logStore = StateObject(wrappedValue: logStore)
        _networkAuditStore = StateObject(wrappedValue: networkAuditStore)
    }
```

### State Objects

| Object | Purpose |
|--------|---------|
| `AppSettings` | Application configuration |
| `ChatStore` | Chat state, version/identity loading |
| `OnboardingState` | Onboarding flow state |
| `ProviderStore` | LLM provider management |
| `LogStore` | Log viewing, Rust log callback bridge |
| `NetworkAuditStore` | Network audit viewing, Rust callback bridge |

### FFI Bridges

Three bridges installed during `init()`:

1. **Log bridge**: `MoltisClient.installLogCallback(logStore:)` — Rust logs flow to Swift LogStore
2. **Session event bridge**: `MoltisClient.installSessionEventCallback(chatStore:)` — Gateway session events flow to Swift ChatStore
3. **Network audit bridge**: `MoltisClient.installNetworkAuditCallback(store:)` — Network audit events flow to Swift NetworkAuditStore

### Lifecycle

1. Initialize all state objects
2. Install FFI bridges
3. If onboarding completed → `ContentView()` (chat UI)
4. If onboarding not completed → onboarding flow
5. On appear: `chatStore.loadVersion()`, `chatStore.loadIdentity()`

### Chat Models

**Source:** `ChatModels.swift`

Chat message models used by the macOS app.

### ChatStore

**Source:** `ChatStore.swift`

Manages chat state, version loading, identity loading.

### Configuration Pane

**Source:** `ConfigurationPane.swift`

Settings → Configuration UI for server configuration.

### HTTPD Pane

**Source:** `HttpdPane.swift`

HTTP daemon status viewing.

### Logs Pane

**Source:** `LogsPane.swift`

Log viewing UI, receives logs via Rust→Swift bridge.

### Network Audit

**Source:** `NetworkAuditPane.swift`, `NetworkAuditStore.swift`

Network audit viewing, receives events via Rust→Swift bridge.

### Onboarding

**Source:** `OnboardingView.swift`, `OnboardingStep.swift`, `OnboardingState.swift`

First-time setup wizard with step-by-step flow.

### Provider Management

**Source:** `ProviderCardView.swift`, `ProviderConfigForm.swift`, `ProviderStore.swift`

LLM provider configuration and management.

### Settings

**Source:** `AppSettings.swift`, `SettingsView.swift`, `SettingsItemViews.swift`, `SettingsSectionContent.swift`, `SettingsSectionContent+Environment.swift`, `SettingsSectionContent+Sandboxes.swift`

Multi-pane settings UI with environment and sandbox configuration.

### Settings Menu Command

```swift
// MoltisApp.swift:3-14
private struct SettingsMenuCommand: Commands {
    @Environment(\.openWindow) private var openWindow

    var body: some Commands {
        CommandGroup(replacing: .appSettings) {
            Button("Settings...") {
                openWindow(id: "settings")
            }
            .keyboardShortcut(",", modifiers: .command)
        }
    }
}
```

Opens settings window with `⌘,` keyboard shortcut.

## Swift Bridge

**Source:** `crates/swift-bridge/src/`

The `swift-bridge` crate provides the FFI layer between Rust and Swift:

- Log callback bridge
- Session event callback bridge
- Network audit callback bridge
- MoltisClient for server communication

This enables the native apps to call Rust functions directly without going through the HTTP API.

## Next

Continue to [08-security](08-security.md) for the security model.
