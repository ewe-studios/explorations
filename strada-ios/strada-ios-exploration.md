# Strada iOS - Complete Exploration

## Overview

**Strada iOS** is a native adapter for Strada-enabled web apps that enables bidirectional communication between a `WKWebView` and native Swift code. It allows building native UI components that are driven by web-based components, creating a seamless bridge between web and native functionality.

### Key Architecture Principles

1. **Component-based architecture**: Each feature is encapsulated in a `BridgeComponent` subclass
2. **Message-passing bridge**: Communication happens through structured `Message` objects
3. **Lifecycle-aware**: Components respect the native iOS view controller lifecycle
4. **Type-safe messaging**: JSON data is decoded using Swift's `Codable` protocol

### Repository Structure

```
strada-ios/
├── Source/
│   ├── Strada.swift              # Global configuration and user agent utilities
│   ├── Bridge.swift              # Core bridge between native and web
│   ├── BridgeComponent.swift     # Base class for native components
│   ├── BridgeDelegate.swift      # Delegate handling bridge lifecycle
│   ├── Message.swift             # Message structure for bridge communication
│   ├── InternalMessage.swift     # Internal message format for JSON serialization
│   ├── JavaScript.swift          # JavaScript function call builder
│   ├── ScriptMessageHandler.swift # WKScriptMessageHandler wrapper
│   ├── StradaConfig.swift        # Configuration (JSON encoder/decoder, logging)
│   ├── Logging.swift             # OSLog-based logging system
│   ├── PathLoaderSPM.swift       # Resource loading for Swift Package Manager
│   ├── PathLoaderXcode.swift     # Resource loading for Xcode builds
│   ├── strada.js                 # JavaScript injected into WebView
│   └── Extensions/
│       ├── Data+Utils.swift      # Data to JSON serialization
│       └── Dictionary+JSON.swift # Dictionary to JSON serialization
├── docs/
│   ├── OVERVIEW.md               # High-level overview
│   ├── INSTALLATION.md           # Installation instructions
│   ├── QUICK-START.md            # Integration guide
│   ├── BUILD-COMPONENTS.md       # Component building guide
│   └── ADVANCED-OPTIONS.md       # Advanced configuration
├── Tests/                        # Unit tests
└── Package.swift                 # Swift Package Manager manifest
```

## Core Concepts

### The Bridge

The `Bridge` class (`Bridge.swift:20-189`) is the central communication channel:
- Injects `strada.js` into the WebView at document start
- Manages `WKScriptMessageHandler` for receiving messages from JavaScript
- Provides methods to register/unregister components
- Handles message reply to the web

### Bridge Components

`BridgeComponent` (`BridgeComponent.swift:27-279`) is the base class for all native components:
- Each component has a unique `name` that matches its web counterpart
- Receives messages via `onReceive(message:)`
- Can reply to messages using various `reply(to:)` / `reply(with:)` methods
- Maintains a cache of received messages per event type

### Bridge Delegate

`BridgeDelegate` (`BridgeDelegate.swift:29-166`) acts as the intermediary:
- Connects the Bridge to the destination (view controller)
- Manages component lifecycle based on view controller lifecycle
- Routes incoming messages to appropriate components
- Tracks active/inactive destinations

### Messages

`Message` (`Message.swift:5-121`) is the data structure for bridge communication:
- `id`: Unique identifier for correlating requests/replies
- `component`: Name of the sending component
- `event`: Event type (e.g., "connect", "submit", "display")
- `metadata`: URL and other metadata
- `jsonData`: JSON-encoded payload

## Communication Flow

### Initialization Sequence

```
1. App creates WKWebView with Bridge.initialize(webView)
2. Bridge injects strada.js as WKUserScript at .atDocumentStart
3. strada.js creates window.nativeBridge instance
4. window.nativeBridge.postMessage("ready") when loaded
5. ScriptMessageHandler receives "ready" event
6. BridgeDelegate.bridgeDidInitialize() is called
7. Bridge registers all component types with web via window.nativeBridge.register()
8. Web calls window.Strada.web.setAdapter(window.nativeBridge)
9. Bridge is now ready for message passing
```

### Message Flow (Web to Native)

```
1. Web component calls window.Strada.web.send(message)
2. strada.js NativeBridge.receive(message) is invoked
3. NativeBridge.postMessage(message) sends to webkit.messageHandlers.strada
4. ScriptMessageHandler.userContentController(_:didReceive:) receives message
5. ScriptMessageHandlerDelegate (Bridge) processes message
6. BridgeDelegate.bridgeDidReceiveMessage(_:) routes to component
7. BridgeComponent.didReceive(message:) caches and calls onReceive(message:)
8. Component's onReceive(message:) handles the event
```

### Message Flow (Native to Web)

```
1. Native component calls reply(with: message) or reply(to: event)
2. BridgeDelegate.reply(with: message) forwards to Bridge
3. Bridge.reply(with: message) calls InternalMessage conversion
4. Bridge.callBridgeFunction(.replyWith, arguments: [internalMessage.toJSON()])
5. JavaScript.evaluate() constructs: window.nativeBridge.replyWith({...})
6. WKWebView.evaluateJavaScript() executes the call
7. NativeBridge.replyWith(message) receives in JavaScript
8. NativeBridge.webBridge.receive(message) delivers to web component
```

## Key Design Patterns

### 1. Weak Reference Pattern (Avoiding Retain Cycles)

The iOS WebKit team recommends using a weak reference pattern to avoid retain cycles between `WKWebView`, `WKUserContentController`, and `WKScriptMessageHandler`:

```swift
// ScriptMessageHandler.swift:8-17
final class ScriptMessageHandler: NSObject, WKScriptMessageHandler {
    weak var delegate: ScriptMessageHandlerDelegate?

    func userContentController(_ userContentController: WKUserContentController,
                               didReceive scriptMessage: WKScriptMessage) {
        delegate?.scriptMessageHandlerDidReceiveMessage(scriptMessage)
    }
}
```

### 2. Dependency Injection for Testing

Components receive their `destination` and `delegate` via initializer:

```swift
// BridgeComponent.swift:41-43
required public init(destination: BridgeDestination, delegate: BridgingDelegate) {
    self.delegate = delegate
}
```

### 3. Type-Safe Message Data

Messages use Swift's `Codable` for type-safe encoding/decoding:

```swift
// Message.swift:72-85
public func data<T: Decodable>() -> T? {
    guard let data = jsonData.data(using: .utf8) else { return nil }
    let decoder = Strada.config.jsonDecoder
    return try? decoder.decode(T.self, from: data)
}
```

### 4. Builder Pattern for JavaScript Calls

The `JavaScript` struct builds safe JavaScript function calls:

```swift
// JavaScript.swift:9-47
struct JavaScript {
    var object: String? = nil
    let functionName: String
    var arguments: [Any] = []

    func toString() throws -> String {
        let encodedArguments = try encode(arguments: arguments)
        let function = sanitizedFunctionName(functionName)
        return "\(function)(\(encodedArguments))"
    }
}
```

## Platform-Specific Implementation Details

### WebView Configuration

The user agent string is used to communicate supported components to the web:

```swift
// Strada.swift:6-9
public static func userAgentSubstring(for componentTypes: [BridgeComponent.Type]) -> String {
    let components = componentTypes.map { $0.name }.joined(separator: " ")
    return "bridge-components: [\(components)]"
}
```

Example: `"Turbo Native iOS bridge-components: [form page composer]"`

### Resource Loading

Two path loaders exist for different build systems:
- `PathLoaderSPM.swift`: Uses `Bundle.module` for Swift Package Manager
- `PathLoaderXcode.swift`: Uses `Bundle(for: type(of: self))` for Xcode

### JavaScript Injection

The `strada.js` file is injected at document start:

```swift
// Bridge.swift:138-152
private func makeUserScript() -> WKUserScript? {
    guard let path = PathLoader().pathFor(name: "strada", fileType: "js") else { return nil }
    let source = try? String(contentsOfFile: path)
    return WKUserScript(source: source, injectionTime: .atDocumentStart, forMainFrameOnly: true)
}
```

## Testing Strategy

The project uses spies for testing component behavior:

```
Tests/
├── Spies/
│   ├── BridgeSpy.swift
│   ├── BridgeComponentSpy.swift
│   └── BridgeDelegateSpy.swift
├── BridgeTests.swift
├── BridgeComponentTests.swift
├── BridgeDelegateTests.swift
└── ComponentTestExample/
    ├── ComposerComponent.swift
    └── ComposerComponentTests.swift
```

## Integration with Turbo Native

Strada iOS is designed to work alongside Turbo Native:
- Extends `VisitableViewController` to conform to `BridgeDestination`
- Delegates view lifecycle events to `BridgeDelegate`
- WebView configuration includes both Turbo and Strada identifiers

Example integration (`QUICK-START.md:62-118`):

```swift
final class TurboWebViewController: VisitableViewController, BridgeDestination {
    private lazy var bridgeDelegate: BridgeDelegate = {
        BridgeDelegate(location: visitableURL.absoluteString,
                       destination: self,
                       componentTypes: BridgeComponent.allTypes)
    }()

    override func viewDidLoad() {
        super.viewDidLoad()
        bridgeDelegate.onViewDidLoad()
    }

    override func visitableDidActivateWebView(_ webView: WKWebView) {
        bridgeDelegate.webViewDidBecomeActive(webView)
    }
}
```

## Deprecation Notice

> **Important**: Strada iOS is being deprecated in favor of [Hotwire Native](https://native.hotwired.dev), which consolidates Turbo Native and Strada into a single framework. For new development, use [Hotwire Native iOS](https://github.com/hotwired/hotwire-native-ios).

---

*This exploration document covers the architecture, communication patterns, and implementation details of Strada iOS. Subsequent deep dive documents explore specific subsystems in greater detail.*
