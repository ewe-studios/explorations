# Strada iOS - WebView Setup and JavaScript Bridge Deep Dive

## Overview

This document explores how Strada iOS sets up and configures the `WKWebView` to enable bidirectional communication between native Swift code and JavaScript in the web page.

## Architecture

### Key Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        Native (Swift)                           │
│  ┌──────────────┐  ┌─────────────┐  ┌──────────────────────┐   │
│  │   Bridge     │◄─┤BridgeDelegate│◄─┤ BridgeComponent(s)  │   │
│  └──────┬───────┘  └─────────────┘  └──────────────────────┘   │
│         │                                                       │
│  ┌──────▼──────────────────────────────────────────────────┐   │
│  │          ScriptMessageHandler (WKScriptMessageHandler)  │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                            │
                            │ webkit.messageHandlers.strada
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                    WebView (JavaScript)                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              window.nativeBridge (strada.js)            │   │
│  └─────────────────────────────────────────────────────────┘   │
│                            │                                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              window.Strada.web (strada-web)             │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Bridge Initialization

### Bridge.initialize(_ webView:)

The entry point for setting up the bridge:

```swift
// Bridge.swift:25-28
public static func initialize(_ webView: WKWebView) {
    if getBridgeFor(webView) == nil {
        initialize(Bridge(webView: webView))
    }
}
```

**Key behaviors:**
1. Checks if a Bridge already exists for this WebView (prevents duplicate initialization)
2. Creates a new Bridge instance if none exists
3. Stores the bridge in a static `instances` array

### Bridge Instance Creation

```swift
// Bridge.swift:31-34
init(webView: WKWebView) {
    self.webView = webView
    loadIntoWebView()
}
```

The initializer:
1. Stores a weak reference to the webView
2. Immediately calls `loadIntoWebView()` to configure the WebView

### WebView Configuration

```swift
// Bridge.swift:126-136
private func loadIntoWebView() {
    guard let configuration = webView?.configuration else { return }

    // Install user script and message handlers in web view
    if let userScript = makeUserScript() {
        configuration.userContentController.addUserScript(userScript)
    }

    let scriptMessageHandler = ScriptMessageHandler(delegate: self)
    configuration.userContentController.add(scriptMessageHandler, name: scriptHandlerName)
}
```

**Two critical configurations:**

1. **WKUserScript** - Injects the JavaScript bridge code
2. **WKScriptMessageHandler** - Sets up the message receiving channel

## JavaScript Injection

### User Script Creation

```swift
// Bridge.swift:138-152
private func makeUserScript() -> WKUserScript? {
    guard
        let path = PathLoader().pathFor(name: "strada", fileType: "js")
    else {
        return nil
    }

    do {
        let source = try String(contentsOfFile: path)
        return WKUserScript(source: source, injectionTime: .atDocumentStart, forMainFrameOnly: true)
    } catch {
        assertionFailure("Could not open strada.js: \(error)")
        return nil
    }
}
```

**Important configuration options:**

| Parameter | Value | Purpose |
|-----------|-------|---------|
| `injectionTime` | `.atDocumentStart` | Ensures bridge is available before any page JavaScript runs |
| `forMainFrameOnly` | `true` | Only injects into main frame, not iframes (security) |

### Path Loading Strategy

Two implementations exist for different build systems:

**Swift Package Manager:**
```swift
// PathLoaderSPM.swift:11-13
func pathFor(name: String, fileType: String, directory: String? = nil) -> String? {
    return Bundle.module.path(forResource: name, ofType: fileType, inDirectory: directory)
}
```

**Xcode:**
```swift
// PathLoaderXcode.swift:11-14
func pathFor(name: String, fileType: String) -> String? {
    let bundle = Bundle(for: type(of: self))
    return bundle.path(forResource: name, ofType: fileType)
}
```

### Package.swift Configuration

```swift
// Package.swift:17-24
.target(
    name: "Strada",
    dependencies: [],
    path: "Source",
    exclude: ["Info.plist", "PathLoaderXcode.swift"],
    resources: [
        .copy("strada.js")
    ])
```

The `strada.js` file is bundled as a resource and accessed via `Bundle.module`.

## JavaScript Bridge (strada.js)

### NativeBridge Class

The entire bridge is encapsulated in an IIFE (Immediately Invoked Function Expression):

```javascript
// strada.js:1-81
(() => {
  class NativeBridge {
    constructor() {
      this.supportedComponents = []
      this.registerCalled = new Promise(resolve => this.registerResolver = resolve)
      document.addEventListener("web-bridge:ready", async () => {
        await this.setAdapter()
      })
    }

    async setAdapter() {
      await this.registerCalled
      this.webBridge.setAdapter(this)
    }
    // ...
  }
  window.nativeBridge = new NativeBridge()
  window.nativeBridge.postMessage("ready")
})()
```

**Key properties:**

| Property | Type | Purpose |
|----------|------|---------|
| `supportedComponents` | `string[]` | List of registered component names |
| `registerCalled` | `Promise` | Resolves when register() is first called |
| `registerResolver` | `function` | Resolver for the above promise |

### Component Registration

```javascript
// strada.js:19-28
register(component) {
  if (Array.isArray(component)) {
    this.supportedComponents = this.supportedComponents.concat(component)
  } else {
    this.supportedComponents.push(component)
  }

  this.registerResolver()
  this.notifyBridgeOfSupportedComponentsUpdate()
}
```

**Flow:**
1. Adds component(s) to `supportedComponents` array
2. Calls `registerResolver()` to resolve the `registerCalled` promise
3. Notifies the web bridge that components are now supported

### Message Passing to Native

```javascript
// strada.js:66-68
postMessage(message) {
  webkit.messageHandlers.strada.postMessage(message)
}
```

This uses the `WKScriptMessageHandler` registered with name `"strada"`.

### Receiving Messages from Native

```javascript
// strada.js:55-58
receive(message) {
  this.postMessage(message)
}

// strada.js:48-52
replyWith(message) {
  if (this.isStradaAvailable) {
    this.webBridge.receive(message)
  }
}
```

**Two paths:**
1. `receive()` - Used for initial messages from web to native
2. `replyWith()` - Used for native-to-web replies via `webBridge`

### Adapter Pattern

```javascript
// strada.js:14-17
async setAdapter() {
  await this.registerCalled
  this.webBridge.setAdapter(this)
}
```

The native bridge registers itself as the adapter for the web bridge, enabling the web to know which components have native support.

## Script Message Handler

### Avoiding Retain Cycles

Apple's WebKit framework creates a retain cycle:
```
WKWebView → WKUserContentController → WKScriptMessageHandler → (delegate) → ViewController → WKWebView
```

Strada solves this with a weak reference wrapper:

```swift
// ScriptMessageHandler.swift:8-17
final class ScriptMessageHandler: NSObject, WKScriptMessageHandler {
    weak var delegate: ScriptMessageHandlerDelegate?

    init(delegate: ScriptMessageHandlerDelegate?) {
        self.delegate = delegate
    }

    func userContentController(_ userContentController: WKUserContentController,
                               didReceive scriptMessage: WKScriptMessage) {
        delegate?.scriptMessageHandlerDidReceiveMessage(scriptMessage)
    }
}
```

### Message Reception

```swift
// Bridge.swift:174-188
extension Bridge: ScriptMessageHandlerDelegate {
    @MainActor
    func scriptMessageHandlerDidReceiveMessage(_ scriptMessage: WKScriptMessage) {
        if let event = scriptMessage.body as? String, event == "ready" {
            delegate?.bridgeDidInitialize()
            return
        }

        if let message = InternalMessage(scriptMessage: scriptMessage) {
            delegate?.bridgeDidReceiveMessage(message.toMessage())
            return
        }

        logger.warning("Unhandled message received: \(String(describing: scriptMessage.body))")
    }
}
```

**Two message types:**
1. `"ready"` string - Initial handshake from JavaScript
2. `InternalMessage` - Structured message objects

## JavaScript Evaluation

### evaluateJavaScriptAsync

A workaround for a SwiftUI/iOS bug:

```swift
// Bridge.swift:191-207
extension WKWebView {
    @discardableResult
    @MainActor
    func evaluateJavaScriptAsync(_ javaScriptString: String) async throws -> Any? {
        return try await withCheckedThrowingContinuation { continuation in
            evaluateJavaScript(javaScriptString) { data, error in
                if let error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume(returning: data)
                }
            }
        }
    }
}
```

**Why the continuation pattern?**
The native async/await version crashes when functions don't return values.

### JavaScript Function Builder

The `JavaScript` struct safely builds function calls:

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

    private func encode(arguments: [Any]) throws -> String {
        guard JSONSerialization.isValidJSONObject(arguments) else {
            throw JavaScriptError.invalidArgumentType
        }
        let data = try JSONSerialization.data(withJSONObject: arguments)
        let string = String(data: data, encoding: .utf8)!
        return String(string.dropFirst().dropLast())
    }
}
```

**Example usage:**
```swift
let js = JavaScript(object: "window.nativeBridge",
                    functionName: "register",
                    arguments: [["form", "page"]])
// Produces: "window.nativeBridge.register([\"form\",\"page\"])"
```

### Calling Bridge Functions

```swift
// Bridge.swift:117-121
@MainActor
private func callBridgeFunction(_ function: JavaScriptBridgeFunction,
                                arguments: [Any]) async throws {
    let js = JavaScript(object: bridgeGlobal,
                        functionName: function.rawValue,
                        arguments: arguments)
    try await evaluate(javaScript: js)
}
```

## Bridge Global Reference

```swift
// Bridge.swift:111-115
/// This needs to match whatever the JavaScript file uses
private let bridgeGlobal = "window.nativeBridge"
```

This string is used to construct all JavaScript calls to the bridge.

## Initialization Sequence (Detailed)

### Step-by-Step Flow

```
1. Native: Bridge.initialize(webView)
   └─> Bridge.init(webView: webView)
       └─> loadIntoWebView()
           ├─> Creates WKUserScript with strada.js
           └─> Adds ScriptMessageHandler with name "strada"

2. WebView: strada.js executes at document start
   └─> window.nativeBridge = new NativeBridge()
       └─> window.nativeBridge.postMessage("ready")

3. Native: ScriptMessageHandler receives "ready"
   └─> scriptMessageHandlerDidReceiveMessage(_:)
       └─> delegate?.bridgeDidInitialize()

4. Native: BridgeDelegate.bridgeDidInitialize()
   └─> Gets component names: ["form", "page", ...]
   └─> Calls bridge?.register(components: componentNames)
       └─> callBridgeFunction(.register, arguments: [["form", "page"]])
           └─> evaluateJavaScript("window.nativeBridge.register([\"form\",\"page\"])")

5. WebView: NativeBridge.register() executes
   └─> Adds components to supportedComponents
   └─> Calls registerResolver() (resolves registerCalled promise)
   └─> notifyBridgeOfSupportedComponentsUpdate()
       └─> webBridge.adapterDidUpdateSupportedComponents()

6. WebView: "web-bridge:ready" event fires (from strada-web)
   └─> NativeBridge.setAdapter()
       └─> webBridge.setAdapter(this)

7. Complete - Bridge is now ready for message passing
```

## User Agent Configuration

Strada communicates supported components via the user agent string:

```swift
// Strada.swift:6-9
public static func userAgentSubstring(for componentTypes: [BridgeComponent.Type]) -> String {
    let components = componentTypes.map { $0.name }.joined(separator: " ")
    return "bridge-components: [\(components)]"
}
```

**Usage in app configuration:**
```swift
let stradaSubstring = Strada.userAgentSubstring(for: BridgeComponent.allTypes)
let userAgent = "Turbo Native iOS \(stradaSubstring)"
configuration.applicationNameForUserAgent = userAgent
```

**Example output:**
```
Turbo Native iOS bridge-components: [form page composer]
```

The web server can parse this to know which components have native support.

## Thread Safety and Main Actor

### @MainActor Annotations

Most bridge methods are marked `@MainActor`:

```swift
// Bridge.swift:40-43
@MainActor
func register(component: String) async throws {
    try await callBridgeFunction(.register, arguments: [component])
}
```

**Reason:** `WKWebView` methods must be called from the main thread.

### Message Handler Delegate

```swift
// Bridge.swift:175-176
@MainActor
func scriptMessageHandlerDidReceiveMessage(_ scriptMessage: WKScriptMessage)
```

The delegate method is marked `@MainActor` because it may call JavaScript.

## Error Handling

### Bridge Error Types

```swift
// Bridge.swift:4-6
public enum BridgeError: Error {
    case missingWebView
}
```

### JavaScript Evaluation Errors

```swift
// Bridge.swift:78-89
@discardableResult
@MainActor
func evaluate(javaScript: String) async throws -> Any? {
    guard let webView else {
        throw BridgeError.missingWebView
    }

    do {
        return try await webView.evaluateJavaScriptAsync(javaScript)
    } catch {
        logger.error("Error evaluating JavaScript: \(error)")
        throw error
    }
}
```

### Invalid Argument Types

```swift
// JavaScript.swift:4-5
enum JavaScriptError: Error, Equatable {
    case invalidArgumentType
}
```

Thrown when arguments can't be serialized to JSON.

## Performance Considerations

### Injection Timing

Using `.atDocumentStart` ensures:
- Bridge is available before any page JavaScript
- No race conditions with web component initialization
- Minimal layout shift (bridge ready before render)

### Message Handler Retain Cycle Prevention

The weak reference pattern prevents memory leaks but requires:
- Bridge instances stored in static array
- Cleanup of nil webViews: `instances.removeAll { $0.webView == nil }`

### Resource Loading

The JavaScript file is:
- Bundled as a resource (not remote)
- Loaded synchronously at initialization
- Cached by WebKit after first load

---

*This deep dive covers the WebView setup, JavaScript injection, and bidirectional message passing architecture. The next documents explore the message structure, component system, and delegate patterns.*
