---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/strada-ios
repository: https://github.com/hotwired/strada-ios
explored_at: 2026-03-20T00:00:00Z
language: Swift
explored_by: Deep exploration for Rust recreation
---

# COMPREHENSIVE DEEP EXPLORATION: Strada iOS

## Executive Summary

Strada iOS is a **bidirectional message bridge** between JavaScript running in a WKWebView and native Swift code. It enables building native iOS UI components that are controlled by web-based logic, forming a core part of the Hotwire Native stack alongside Turbo Native.

### Architecture Pattern: Message-Based Bridge

```
┌─────────────────────────────────────────────────────────────────┐
│                         WEBVIEW (WKWebView)                      │
│  ┌──────────────┐    ┌───────────────┐    ┌─────────────────┐  │
│  │ Web Bridge   │◄──►│  strada.js    │◄──►│ webkit.         │  │
│  │ (strada-web) │    │  (Injected)   │    │ messageHandlers │  │
│  └──────────────┘    └───────────────┘    └────────┬────────┘  │
└─────────────────────────────────────────────────────┼───────────┘
                                                      │ postMessage
                                                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                      NATIVE iOS (Swift)                          │
│  ┌────────────────┐  ┌────────────────┐  ┌───────────────────┐  │
│  │ Bridge         │◄─┤ BridgeDelegate │◄─┤ ScriptMessage     │  │
│  │ (WKWebView     │  │ (Lifecycle +   │  │ Handler           │  │
│  │  integration)  │  │  Routing)      │  │ (WKWebView msg)   │  │
│  └───────┬────────┘  └───────┬────────┘  └───────────────────┘  │
│          │                   │                                   │
│          ▼                   ▼                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              BridgeComponent instances                    │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐          │  │
│  │  │ Form       │  │ Page       │  │ Composer   │  ...     │  │
│  │  │ Component  │  │ Component  │  │ Component  │          │  │
│  │  └────────────┘  └────────────┘  └────────────┘          │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Core Design Principles

1. **Component Registry Pattern** - Components are registered by name, created on-demand
2. **Message Queue Semantics** - Messages cached by event for reply correlation
3. **Lifecycle-Aware Routing** - Messages only delivered when destination is active
4. **Type-Safe Serialization** - Generic Encodable/Decodable wrappers around JSON
5. **Retain-Cycle Prevention** - Weak references throughout WebView integration

---

## Repository & Build

| Property | Value |
|----------|-------|
| **Location** | `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/strada-ios` |
| **Remote** | https://github.com/hotwired/strada-ios |
| **Language** | Swift 5.3+ |
| **License** | MIT License (37signals) |
| **Min iOS** | iOS 14.0 |
| **Package** | Swift Package Manager |
| **Bundle** | `Bundle.module` for SPM resources |

### Package.swift Structure

```swift
let package = Package(
    name: "Strada",
    platforms: [.iOS(.v14)],
    products: [.library(name: "Strada", targets: ["Strada"])],
    dependencies: [],  // No external dependencies - pure Foundation + WebKit
    targets: [
        .target(
            name: "Strada",
            dependencies: [],
            path: "Source",
            exclude: ["Info.plist", "PathLoaderXcode.swift"],
            resources: [.copy("strada.js")]  // JS bundled as resource
        )
    ]
)
```

**Note:** Two PathLoader implementations exist:
- `PathLoaderSPM.swift` - Uses `Bundle.module` (Swift Package Manager)
- `PathLoaderXcode.swift` - Uses `Bundle(for: type(of: self))` (Xcode project)

---

## Complete Directory Structure

```
strada-ios/
├── .github/                              # GitHub workflows and issue templates
├── docs/                                 # User documentation
│   ├── INSTALLATION.md                   # SPM integration steps
│   ├── OVERVIEW.md                       # High-level architecture
│   ├── QUICK-START.md                    # Step-by-step integration guide
│   ├── BUILD-COMPONENTS.md               # Creating custom BridgeComponents
│   ├── ADVANCED-OPTIONS.md               # Custom JSON encoding, debug logging
│   └── CONDUCT.md                        # Code of conduct
├── Source/                               # Library source code (~2,000 lines)
│   ├── Core Bridge Files
│   │   ├── Bridge.swift                  # WKWebView integration, JS evaluation
│   │   ├── BridgeDelegate.swift          # Lifecycle management, message routing
│   │   ├── BridgeComponent.swift         # Base class for native components
│   │   ├── BridgeDestination.swift       # Empty marker protocol for view controllers
│   │   └── Strada.swift                  # Public namespace, User-Agent generator
│   │
│   ├── Message System
│   │   ├── Message.swift                 # Public message struct (id, event, data)
│   │   ├── InternalMessage.swift         # Internal wire format for JS bridge
│   │   └── JavaScript.swift              # JS function call string builder
│   │
│   ├── WebView Integration
│   │   ├── ScriptMessageHandler.swift    # WKScriptMessageHandler wrapper
│   │   └── strada.js                     # Injected JavaScript bridge
│   │
│   ├── Configuration
│   │   ├── StradaConfig.swift            # JSON encoder/decoder, debug logging
│   │   └── Logging.swift                 # OSLog integration with subsystem
│   │
│   ├── Resource Loading
│   │   ├── PathLoaderSPM.swift           # Bundle.module for SPM
│   │   └── PathLoaderXcode.swift         # Bundle(for:) for Xcode projects
│   │
│   ├── Extensions/                       # Helper extensions
│   │   ├── Data+Utils.swift              # Data.decoded<T: Decodable>()
│   │   ├── Dictionary+JSON.swift         # Dictionary.jsonData()
│   │   ├── String+JSON.swift             # String.jsonObject()
│   │   └── Encodable+Utils.swift         # Encodable.encoded()
│   │
│   └── Info.plist                        # Bundle metadata
│
├── Tests/                                # Unit tests (~1,500 lines)
│   ├── BridgeTests.swift                 # Bridge initialization, registration
│   ├── BridgeComponentTests.swift        # Component message handling
│   ├── BridgeDelegateTests.swift         # Lifecycle, routing, component factory
│   ├── MessageTests.swift                # Message creation, encoding, equality
│   ├── InternalMessageTests.swift        # Internal message conversion
│   ├── JavaScriptTests.swift             # JS function building
│   ├── UserAgentTests.swift              # User-Agent string generation
│   ├── ComponentTestExample/
│   │   ├── ComposerComponent.swift       # Example: composer component
│   │   └── ComposerComponentTests.swift  # Tests for composer component
│   ├── Spies/                            # Test doubles (mocks)
│   │   ├── BridgeSpy.swift               # Mock Bridgable
│   │   ├── BridgeComponentSpy.swift      # Mock BridgeComponent
│   │   └── BridgeDelegateSpy.swift       # Mock BridgingDelegate
│   ├── TestData.swift                    # Shared test data generators
│   └── Extensions/
│       └── TimeInterval+ExpectationTimeout.swift  # Test timeout helper
│
├── Strada.xcodeproj/                     # Xcode project file
├── Package.swift                         # Swift Package Manager manifest
├── README.md                             # Project overview
└── LICENSE                               # MIT license
```

---

## Part 1: WebView Setup & Configuration

### Bridge.swift - Complete Analysis

**File Location:** `Source/Bridge.swift` (208 lines)

#### BridgeError Enum

```swift
public enum BridgeError: Error {
    case missingWebView
}
```

**Purpose:** Error type for bridge operations. Currently only one error case exists for when WebView is nil.

**Rust Recreation:**
```rust
#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("WebView is missing")]
    MissingWebView,
}
```

#### Bridgable Protocol

```swift
protocol Bridgable: AnyObject {
    var delegate: BridgeDelegate? { get set }
    var webView: WKWebView? { get }

    func register(component: String) async throws
    func register(components: [String]) async throws
    func unregister(component: String) async throws
    func reply(with message: Message) async throws
}
```

**Purpose:** Internal protocol defining bridge capabilities. Used for dependency injection in tests and decoupling.

**Key Points:**
- `AnyObject` constraint allows weak references (prevents retain cycles)
- All methods are async throws - JavaScript evaluation can fail
- Protocol enables testability via BridgeSpy mock

#### Bridge Class - Static Instance Management

```swift
public final class Bridge: Bridgable {
    public typealias InitializationCompletionHandler = () -> Void
    weak var delegate: BridgeDelegate?
    weak var webView: WKWebView?

    public static func initialize(_ webView: WKWebView) {
        if getBridgeFor(webView) == nil {
            initialize(Bridge(webView: webView))
        }
    }

    init(webView: WKWebView) {
        self.webView = webView
        loadIntoWebView()
    }
}
```

**Critical Design Decisions:**

1. **Weak webView Reference:** Prevents retain cycle between Bridge and WKWebView
2. **Singleton-per-WebView Pattern:** Static `instances` array ensures one Bridge per WebView
3. **Automatic Initialization:** `initialize(_:)` checks for existing bridge before creating new one

**Instance Pool Management:**

```swift
private static var instances: [Bridge] = []

static func initialize(_ bridge: Bridge) {
    instances.append(bridge)
    instances.removeAll { $0.webView == nil }  // Cleanup dead references
}

static func getBridgeFor(_ webView: WKWebView) -> Bridge? {
    return instances.first { $0.webView == webView }
}
```

**Memory Management Flow:**
```
1. Bridge.initialize(webView) called
2. getBridgeFor(webView) checks existing instances
3. If nil, create new Bridge(webView: webView)
4. Bridge.init calls loadIntoWebView()
5. Bridge appended to instances array
6. Dead references (nil webView) cleaned up
```

**Rust Recreation Notes:**
- Use `Arc<Weak<Mutex<WebView>>>` for weak references
- HashMap<WebViewId, Arc<Bridge>> for instance pool
- Cleanup on WebView destruction via drop guard

#### Component Registration Methods

```swift
@MainActor
func register(component: String) async throws {
    try await callBridgeFunction(.register, arguments: [component])
}

@MainActor
func register(components: [String]) async throws {
    try await callBridgeFunction(.register, arguments: [components])
}

@MainActor
func unregister(component: String) async throws {
    try await callBridgeFunction(.unregister, arguments: [component])
}
```

**Key Implementation Details:**

1. **@MainActor Annotation:** All WebView interactions must happen on main thread (iOS requirement)
2. **Single vs Bulk:** Both single and batch registration for efficiency
3. **Bridge Function Call:** Delegates to `callBridgeFunction(_:arguments:)` which builds JS call

**JavaScript Generated:**
```javascript
// Single component: "form"
window.nativeBridge.register(["form"])

// Multiple components: ["form", "page"]
window.nativeBridge.register(["form", "page"])

// Unregister
window.nativeBridge.unregister(["form"])
```

#### Reply Method - Sending Messages to Web

```swift
@MainActor
func reply(with message: Message) async throws {
    logger.debug("bridgeWillReplyWithMessage: \(String(describing: message))")
    let internalMessage = InternalMessage(from: message)
    try await callBridgeFunction(.replyWith, arguments: [internalMessage.toJSON()])
}
```

**Message Transformation Pipeline:**
```
Message (Swift struct)
    ↓
InternalMessage (dictionary format)
    ↓
toJSON() -> [String: AnyHashable]
    ↓
JavaScript argument
    ↓
window.nativeBridge.replyWith({...})
```

#### JavaScript Evaluation

```swift
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

@MainActor
func evaluate(function: String, arguments: [Any] = []) async throws -> Any? {
    try await evaluate(javaScript: JavaScript(functionName: function, arguments: arguments).toString())
}
```

**Workaround for iOS Bug:**

```swift
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

**Why This Exists:** The native `evaluateJavaScript(_:completionHandler:)` async version crashes with `Unexpectedly found nil while implicitly unwrapping an Optional value` when the JS function returns nothing. Using `withCheckedThrowingContinuation` wraps the completion-handler version safely.

**Rust Recreation:**
- Use `wry` or `tauri` for WebView on Rust
- JavaScript evaluation returns `Result<Value, Error>`
- Handle undefined/null returns explicitly

#### Loading Into WebView

```swift
private func loadIntoWebView() {
    guard let configuration = webView?.configuration else { return }

    // Install user script at document start
    if let userScript = makeUserScript() {
        configuration.userContentController.addUserScript(userScript)
    }

    // Add message handler for "strada" channel
    let scriptMessageHandler = ScriptMessageHandler(delegate: self)
    configuration.userContentController.add(scriptMessageHandler, name: scriptHandlerName)
}

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

**Injection Timing:** `.atDocumentStart` means strada.js runs BEFORE any web page JavaScript, ensuring `window.nativeBridge` exists when web code loads.

**ForMainFrameOnly: true** prevents injection into iframes - only the main frame gets the bridge.

**Resource Loading Flow:**
```
PathLoader.pathFor(name: "strada", fileType: "js")
    ↓
SPM: Bundle.module.path(forResource:ofType:inDirectory:)
Xcode: Bundle(for: type(of: self)).path(forResource:ofType:)
    ↓
Returns: "/path/to/Strada.bundle/strada.js"
    ↓
String(contentsOfFile:) reads JavaScript source
    ↓
WKUserScript created with source
```

**Rust Recreation:**
- Bundle JS as embedded asset using `include_str!`
- Use `webview::dispatch()` or equivalent for script injection
- Inject before page load with `add_init_script()`

#### Message Handler Protocol Implementation

```swift
extension Bridge: ScriptMessageHandlerDelegate {
    @MainActor
    func scriptMessageHandlerDidReceiveMessage(_ scriptMessage: WKScriptMessage) {
        // Handle "ready" event from JS
        if let event = scriptMessage.body as? String, event == "ready" {
            delegate?.bridgeDidInitialize()
            return
        }

        // Handle message objects
        if let message = InternalMessage(scriptMessage: scriptMessage) {
            delegate?.bridgeDidReceiveMessage(message.toMessage())
            return
        }

        logger.warning("Unhandled message received: \(String(describing: scriptMessage.body))")
    }
}
```

**Message Parsing Priority:**
1. Check if String "ready" - initialization signal
2. Try parsing as InternalMessage - actual bridge messages
3. Log warning for unknown formats

**Rust Recreation:**
```rust
fn handle_script_message(&self, body: JsValue) {
    if let Some(event) = body.as_string() {
        if event == "ready" {
            self.delegate.bridge_did_initialize();
            return;
        }
    }

    if let Ok(message) = InternalMessage::from_js(&body) {
        self.delegate.bridge_did_receive_message(message.to_message());
    }
}
```

#### Private Bridge Function Enumeration

```swift
private enum JavaScriptBridgeFunction: String {
    case register
    case unregister
    case replyWith
}
```

**Purpose:** Type-safe function names for JS calls. Only these three functions are ever called on `window.nativeBridge`.

---

## Part 2: strada.js - Injected JavaScript Bridge

### Complete Source Analysis

**File Location:** `Source/strada.js` (81 lines)

```javascript
(() => {
  // NativeBridge: The adapter installed on webBridge
  // All adapters (iOS, Android) implement same interface
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

    register(component) {
      if (Array.isArray(component)) {
        this.supportedComponents = this.supportedComponents.concat(component)
      } else {
        this.supportedComponents.push(component)
      }

      this.registerResolver()
      this.notifyBridgeOfSupportedComponentsUpdate()
    }

    unregister(component) {
      const index = this.supportedComponents.indexOf(component)
      if (index != -1) {
        this.supportedComponents.splice(index, 1)
        this.notifyBridgeOfSupportedComponentsUpdate()
      }
    }

    notifyBridgeOfSupportedComponentsUpdate() {
      if (this.isStradaAvailable) {
        this.webBridge.adapterDidUpdateSupportedComponents()
      }
    }

    supportsComponent(component) {
      return this.supportedComponents.includes(component)
    }

    // Reply to web with message
    replyWith(message) {
      if (this.isStradaAvailable) {
        this.webBridge.receive(message)
      }
    }

    // Receive from web
    receive(message) {
      this.postMessage(message)
    }

    get platform() {
      return "ios"
    }

    // Native handler - posts to WKWebView message handler
    postMessage(message) {
      webkit.messageHandlers.strada.postMessage(message)
    }

    get isStradaAvailable() {
      return window.Strada
    }

    get webBridge() {
      return window.Strada.web
    }
  }

  window.nativeBridge = new NativeBridge()
  window.nativeBridge.postMessage("ready")
})()
```

### NativeBridge Class Breakdown

#### Constructor & Promise Pattern

```javascript
constructor() {
  this.supportedComponents = []
  this.registerCalled = new Promise(resolve => this.registerResolver = resolve)
  document.addEventListener("web-bridge:ready", async () => {
    await this.setAdapter()
  })
}
```

**Critical Pattern:** The `registerCalled` promise ensures components are registered BEFORE the adapter is set on the webBridge.

**Flow:**
```
1. NativeBridge instantiated
2. registerCalled Promise created (pending)
3. Listener added for "web-bridge:ready" event
4. When event fires -> setAdapter() called
5. setAdapter() awaits registerCalled
6. Native calls register() -> registerResolver() resolves promise
7. Adapter set on webBridge
```

#### setAdapter Method

```javascript
async setAdapter() {
  await this.registerCalled
  this.webBridge.setAdapter(this)
}
```

**Purpose:** Waits for native to call `register()` before connecting to webBridge. This ensures the web knows which components are supported before any messages flow.

#### register/unregister Methods

```javascript
register(component) {
  if (Array.isArray(component)) {
    this.supportedComponents = this.supportedComponents.concat(component)
  } else {
    this.supportedComponents.push(component)
  }

  this.registerResolver()  // Resolves the promise!
  this.notifyBridgeOfSupportedComponentsUpdate()
}

unregister(component) {
  const index = this.supportedComponents.indexOf(component)
  if (index != -1) {
    this.supportedComponents.splice(index, 1)
    this.notifyBridgeOfSupportedComponentsUpdate()
  }
}
```

**Dual Call Pattern:** Native calls both `register()` which:
1. Resolves `registerCalled` promise (first call only via resolver)
2. Adds to supportedComponents array
3. Notifies webBridge of update

**Note:** `registerResolver()` is called every time but only matters on first registration since Promise resolvers are idempotent (only first call takes effect).

#### Message Flow Methods

```javascript
// Native -> Web reply
replyWith(message) {
  if (this.isStradaAvailable) {
    this.webBridge.receive(message)
  }
}

// Web -> Native send
receive(message) {
  this.postMessage(message)
}

// Post to native via WKWebView
postMessage(message) {
  webkit.messageHandlers.strada.postMessage(message)
}
```

**Message Direction:**
| Method | Direction | Called By |
|--------|-----------|-----------|
| `receive()` | Web -> Native | Web components |
| `postMessage()` | Web -> Native | Internal, calls WKWebView |
| `replyWith()` | Native -> Web | Native bridge |

#### Platform Detection

```javascript
get platform() {
  return "ios"
}
```

**Usage:** Web code can check `window.nativeBridge.platform` to detect iOS vs Android ("android" for strada-android).

#### Initialization Signal

```javascript
window.nativeBridge = new NativeBridge()
window.nativeBridge.postMessage("ready")
```

**Purpose:** The "ready" string message tells native that JS is loaded. Native responds by calling `delegate?.bridgeDidInitialize()` which triggers component registration.

### Rust Recreation - JavaScript Equivalents

```rust
// Embedded JavaScript
const STRADA_JS: &str = include_str!("strada.js");

// For Rust + WebView (wry/tauri)
pub struct NativeBridge {
    supported_components: RwLock<Vec<String>>,
    register_called: Arc<Notify>,  // tokio::sync::Notify for signal
}

impl NativeBridge {
    pub fn new() -> Self {
        Self {
            supported_components: RwLock::new(Vec::new()),
            register_called: Arc::new(Notify::new()),
        }
    }

    pub fn register(&self, component: &str) {
        let mut components = self.supported_components.write();
        if !components.contains(&component.to_string()) {
            components.push(component.to_string());
        }
        self.register_called.notify_one();
    }

    pub fn supports_component(&self, component: &str) -> bool {
        let components = self.supported_components.read();
        components.contains(&component.to_string())
    }
}
```

---

## Part 3: Message System Deep Dive

### Message.swift - Public Message Structure

**File Location:** `Source/Message.swift` (121 lines)

```swift
public struct Message: Equatable {
    public let id: String           // Unique message ID
    public let component: String    // "form", "page", "composer", etc.
    public let event: String        // "connect", "submit", "select-sender"
    public let metadata: Metadata?  // Contains URL
    public let jsonData: String     // JSON-encoded payload
}
```

**Design Rationale:**

1. **String-based IDs:** UUIDs generated by web side, used for request/reply correlation
2. **Component Names:** Must match between web and native component names
3. **Event-driven:** Each message has exactly one event type
4. **Opaque JSON Data:** Native doesn't parse - passes through to Codable helpers

#### Message Metadata

```swift
extension Message {
    public struct Metadata: Equatable {
        public let url: String

        public init(url: String) {
            self.url = url
        }
    }
}
```

**Purpose:** URL identifies which page/screen sent the message. Used for lifecycle filtering (only deliver messages to active destination).

#### Message Creation Methods

```swift
public func replacing(event updatedEvent: String? = nil,
                      jsonData updatedData: String? = nil) -> Message {
    Message(id: id,
            component: component,
            event: updatedEvent ?? event,
            metadata: metadata,
            jsonData: updatedData ?? jsonData)
}

public func replacing<T: Encodable>(event updatedEvent: String? = nil,
                                    data: T) -> Message {
    let updatedData: String?
    do {
        let jsonData = try Strada.config.jsonEncoder.encode(data)
        updatedData = String(data: jsonData, encoding: .utf8)
    } catch {
        logger.error("Error encoding codable object: \(error)")
        updatedData = nil
    }

    return replacing(event: updatedEvent, jsonData: updatedData)
}
```

**Usage Pattern:**
```swift
// Reply with same event, new data
let reply = message.replacing(data: MyData(title: "Hello"))

// Reply with different event
let reply = message.replacing(event: "submitted", data: MyData())

// Just change event, keep data
let reply = message.replacing(event: "acknowledged")
```

#### Decoding Message Data

```swift
public func data<T: Decodable>() -> T? {
    guard let data = jsonData.data(using: .utf8) else {
        logger.error("Error converting json string to data: \(jsonData)")
        return nil
    }

    do {
        let decoder = Strada.config.jsonDecoder
        return try decoder.decode(T.self, from: data)
    } catch {
        logger.error("Error decoding json: \(jsonData) -> \(error)")
        return nil
    }
}
```

**Usage in Components:**
```swift
struct FormData: Decodable {
    let submitTitle: String
    let actionURL: String
}

if let data: FormData = message.data() {
    // Use data.submitTitle
}
```

#### Semantic Equality

```swift
public static func == (lhs: Self, rhs: Self) -> Bool {
    return lhs.id == rhs.id &&
           lhs.component == rhs.component &&
           lhs.event == rhs.event &&
           lhs.metadata == rhs.metadata &&
           lhs.jsonData.jsonObject() as? [String: AnyHashable] ==
           rhs.jsonData.jsonObject() as? [String: AnyHashable]
}
```

**Why Not String Compare?** JSON key order shouldn't matter:
```json
{"title": "Hi", "action": "go"} == {"action": "go", "title": "Hi"}
```

These should be equal even though strings differ. Parsing to dictionaries ensures semantic equality.

---

### InternalMessage.swift - Wire Format

**File Location:** `Source/InternalMessage.swift` (106 lines)

```swift
struct InternalMessage {
    let id: String
    let component: String
    let event: String
    let data: InternalMessageData  // [String: AnyHashable]
}
```

**Purpose:** Internal representation that matches JavaScript object structure exactly. Conversion layer between WKScriptMessage and Message.

#### Construction from WKScriptMessage

```swift
init?(scriptMessage: WKScriptMessage) {
    guard let message = scriptMessage.body as? [String: AnyHashable] else {
        logger.warning("Script message is missing body: \(scriptMessage)")
        return nil
    }

    self.init(jsonObject: message)
}

init?(jsonObject: [String: AnyHashable]) {
    guard let id = jsonObject[CodingKeys.id.rawValue] as? String,
          let component = jsonObject[CodingKeys.component.rawValue] as? String,
          let event = jsonObject[CodingKeys.event.rawValue] as? String else {
        logger.error("Error parsing script message: \(jsonObject)")
        return nil
    }

    let data = (jsonObject[CodingKeys.data.rawValue] as? InternalMessageData) ?? [:]

    self.init(id: id, component: component, event: event, data: data)
}
```

**Validation:** Requires id, component, event. Data is optional (defaults to empty dict).

#### Nested Metadata Structure

```swift
extension InternalMessage {
    struct DataMetadata: Codable {
        let metadata: InternalMessage.Metadata
    }

    struct Metadata: Codable {
        let url: String
    }
}
```

**Why Nested?** The web sends metadata INSIDE the data object:
```json
{
  "id": "uuid-123",
  "component": "form",
  "event": "connect",
  "data": {
    "metadata": {
      "url": "https://example.com/forms/1"
    },
    "submitTitle": "Send"
  }
}
```

#### Metadata Extraction

```swift
private func metadata() -> Message.Metadata? {
    guard let jsonData = data.jsonData(),
          let internalMetadata: InternalMessage.DataMetadata = try? jsonData.decoded() else {
        return nil
    }

    return Message.Metadata(url: internalMetadata.metadata.url)
}
```

**Extraction Flow:**
```
data: [String: AnyHashable]
    ↓
jsonData() -> Data
    ↓
decoded() -> DataMetadata
    ↓
Extract metadata.url
    ↓
Message.Metadata(url: "...")
```

#### Conversion to Message

```swift
func toMessage() -> Message {
    return Message(id: id,
                   component: component,
                   event: event,
                   metadata: metadata(),
                   jsonData: dataAsJSONString() ?? "{}")
}

private func dataAsJSONString() -> String? {
    guard let jsonData = data.jsonData() else { return nil }
    return String(data: jsonData, encoding: .utf8)
}
```

**Full Transformation:**
```
WKScriptMessage.body (dict)
    ↓
InternalMessage (structured)
    ↓
metadata() extracted from data
    ↓
toMessage() -> Message
```

#### toJSON for JavaScript

```swift
func toJSON() -> [String: AnyHashable] {
    [
        CodingKeys.id.rawValue: id,
        CodingKeys.component.rawValue: component,
        CodingKeys.event.rawValue: event,
        CodingKeys.data.rawValue: data
    ]
}
```

**Output:** Dictionary that becomes JavaScript object argument:
```swift
let js = JavaScript(object: "window.nativeBridge",
                    functionName: "replyWith",
                    arguments: [internalMessage.toJSON()])
// Generates: window.nativeBridge.replyWith({"id":"...","component":"...",...})
```

---

## Part 4: BridgeDelegate & Lifecycle Management

### BridgeDelegate.swift - Complete Analysis

**File Location:** `Source/BridgeDelegate.swift` (166 lines)

#### Protocol Definitions

```swift
public protocol BridgeDestination: AnyObject {}

@MainActor
public protocol BridgingDelegate: AnyObject {
    var location: String { get }
    var destination: BridgeDestination { get }
    var webView: WKWebView? { get }

    func webViewDidBecomeActive(_ webView: WKWebView)
    func webViewDidBecomeDeactivated()
    func reply(with message: Message) async throws -> Bool

    func onViewDidLoad()
    func onViewWillAppear()
    func onViewDidAppear()
    func onViewWillDisappear()
    func onViewDidDisappear()

    func component<C: BridgeComponent>() -> C?

    func bridgeDidInitialize()
    func bridgeDidReceiveMessage(_ message: Message) -> Bool
}
```

**BridgeDestination:** Empty marker protocol. View controllers conform to this to indicate they support bridge components.

**BridgingDelegate:** Internal protocol for dependency injection. BridgeComponent uses this to reply without knowing about Bridge directly.

#### BridgeDelegate Class Structure

```swift
@MainActor
public final class BridgeDelegate: BridgingDelegate {
    public let location: String                          // Initial URL string
    public unowned let destination: BridgeDestination    // Weak ref to VC
    public var webView: WKWebView? { bridge?.webView }   // From bridge

    weak var bridge: Bridgable?                          // Set when webView active

    public init(location: String,
                destination: BridgeDestination,
                componentTypes: [BridgeComponent.Type]) {
        self.location = location
        self.destination = destination
        self.componentTypes = componentTypes
    }
}
```

**Reference Graph:**
```
BridgeDelegate
    ├─ unowned destination (UIViewController)
    ├─ weak bridge (Bridge)
    └─ componentTypes ([BridgeComponent.Type])
```

**Why unowned?** Destination must outlive delegate - delegate is lazy property of VC.

**Why weak bridge?** Bridge may be nil if WebView not yet active.

#### WebView Activation Lifecycle

```swift
public func webViewDidBecomeActive(_ webView: WKWebView) {
    bridge = Bridge.getBridgeFor(webView)
    bridge?.delegate = self

    if bridge == nil {
        logger.warning("bridgeNotInitializedForWebView")
    }
}

public func webViewDidBecomeDeactivated() {
    bridge?.delegate = nil
    bridge = nil
}
```

**When Called:**
- `webViewDidBecomeActive`: Turbo activates WebView for navigation
- `webViewDidBecomeDeactivated`: WebView dismissed/navigated away

**Purpose:** Links delegate to bridge only when WebView is visible/active.

#### Destination Lifecycle Methods

```swift
public func onViewDidLoad() {
    logger.debug("bridgeDestinationViewDidLoad: \(self.resolvedLocation)")
    destinationIsActive = true
    activeComponents.forEach { $0.viewDidLoad() }
}

public func onViewWillAppear() {
    logger.debug("bridgeDestinationViewWillAppear: \(self.resolvedLocation)")
    destinationIsActive = true
    activeComponents.forEach { $0.viewWillAppear() }
}

public func onViewDidAppear() {
    logger.debug("bridgeDestinationViewDidAppear: \(self.resolvedLocation)")
    destinationIsActive = true
    activeComponents.forEach { $0.viewDidAppear() }
}

public func onViewWillDisappear() {
    activeComponents.forEach { $0.viewWillDisappear() }
    logger.debug("bridgeDestinationViewWillDisappear: \(self.resolvedLocation)")
}

public func onViewDidDisappear() {
    activeComponents.forEach { $0.viewDidDisappear() }
    destinationIsActive = false
    logger.debug("bridgeDestinationViewDidDisappear: \(self.resolvedLocation)")
}
```

**State Machine:**
```
viewDidLoad    -> destinationIsActive = true
viewWillAppear -> (already true)
viewDidAppear  -> (already true)
viewWillDisappear -> (still true during transition)
viewDidDisappear -> destinationIsActive = false
```

**Component Lifecycle Forwarding:**
```swift
// BridgeComponent base class
public func viewDidLoad() {
    onViewDidLoad()  // Calls subclass override
}
```

#### Message Routing

```swift
@discardableResult
public func bridgeDidReceiveMessage(_ message: Message) -> Bool {
    guard destinationIsActive,
          resolvedLocation == message.metadata?.url else {
        logger.warning("bridgeDidIgnoreMessage: \(String(describing: message))")
        return false
    }

    logger.debug("bridgeDidReceiveMessage \(String(describing: message))")
    getOrCreateComponent(name: message.component)?.didReceive(message: message)

    return true
}
```

**Two Filters:**

1. **destinationIsActive:** Don't process messages for disappeared views
2. **URL Match:** Only process messages for current URL

**Why URL Filtering?** Web may send messages during navigation. Only deliver to matching destination.

**resolvedLocation:**
```swift
private var resolvedLocation: String {
    webView?.url?.absoluteString ?? location
}
```

Falls back to initial location if webView.url is nil.

#### Component Factory

```swift
private var initializedComponents: [String: BridgeComponent] = [:]

private func getOrCreateComponent(name: String) -> BridgeComponent? {
    // Return existing
    if let component = initializedComponents[name] {
        return component
    }

    // Find type
    guard let componentType = componentTypes.first(where: { $0.name == name }) else {
        return nil
    }

    // Create new instance
    let component = componentType.init(destination: destination, delegate: self)
    initializedComponents[name] = component

    return component
}
```

**Lazy Instantiation:** Components created only when first message received.

**Type Lookup:**
```swift
componentTypes: [BridgeComponent.Type] = [FormComponent.self, PageComponent.self]
componentType.init(...) // Metatype initialization
```

#### Component Registration on Bridge Init

```swift
public func bridgeDidInitialize() {
    let componentNames = componentTypes.map { $0.name }
    Task {
        do {
            try await bridge?.register(components: componentNames)
        } catch {
            logger.error("bridgeDidFailToRegisterComponents: \(error)")
        }
    }
}
```

**Flow:**
```
1. JS posts "ready" message
2. Bridge calls delegate.bridgeDidInitialize()
3. Get all component names: ["form", "page", "composer"]
4. Call JS: window.nativeBridge.register(["form", "page", "composer"])
5. NativeBridge.register() resolves promise, sets adapter
```

#### Generic Component Access

```swift
public func component<C: BridgeComponent>() -> C? {
    return activeComponents.compactMap { $0 as? C }.first
}
```

**Usage:**
```swift
if let formComponent: FormComponent = bridgeDelegate.component() {
    // Use formComponent
}
```

#### Private State

```swift
private var initializedComponents: [String: BridgeComponent] = [:]
private var destinationIsActive = false
private let componentTypes: [BridgeComponent.Type]

private var activeComponents: [BridgeComponent] {
    return initializedComponents.values.filter { _ in destinationIsActive }
}
```

**activeComponents Computed Property:** Returns all components if destination active, empty array otherwise.

---

## Part 5: BridgeComponent - Base Class for Native Components

### BridgeComponent.swift - Complete Analysis

**File Location:** `Source/BridgeComponent.swift` (279 lines)

#### Protocol Definition

```swift
@MainActor
protocol BridgingComponent: AnyObject {
    static var name: String { get }
    var delegate: BridgingDelegate { get }

    init(destination: BridgeDestination, delegate: BridgingDelegate)

    func onReceive(message: Message)
    func onViewDidLoad()
    func onViewWillAppear()
    func onViewDidAppear()
    func onViewWillDisappear()
    func onViewDidDisappear()

    func didReceive(message: Message)
    func viewDidLoad()
    func viewWillAppear()
    func viewDidAppear()
    func viewWillDisappear()
    func viewDidDisappear()
}
```

**Design Notes:**
- `static name` - Each component type has unique identifier
- `delegate` - For sending replies
- Dual methods: `onReceive` (override) vs `didReceive` (called by framework)

#### BridgeComponent Base Class

```swift
@MainActor
open class BridgeComponent: BridgingComponent {
    public typealias ReplyCompletionHandler = (Result<Bool, Error>) -> Void

    nonisolated open class var name: String {
        fatalError("BridgeComponent subclass must provide a unique 'name'")
    }

    public unowned let delegate: BridgingDelegate

    required public init(destination: BridgeDestination, delegate: BridgingDelegate) {
        self.delegate = delegate
    }

    open func onReceive(message: Message) {
        fatalError("BridgeComponent subclass must handle incoming messages")
    }
}
```

**Subclass Requirements:**
1. Override `class var name` - Component identifier
2. Override `onReceive(message:)` - Message handler

**Example Subclass:**
```swift
final class FormComponent: BridgeComponent {
    override class var name: String { "form" }

    override func onReceive(message: Message) {
        guard let event = Event(rawValue: message.event) else { return }

        switch event {
        case .connect: handleConnect(message)
        case .submitEnabled: enableSubmit()
        case .submitDisabled: disableSubmit()
        }
    }
}
```

#### Message Caching

```swift
private var receivedMessages = [String: Message]()

public func didReceive(message: Message) {
    receivedMessages[message.event] = message  // Cache by event
    onReceive(message: message)                 // Call handler
}

public func receivedMessage(for event: String) -> Message? {
    return receivedMessages[event]
}
```

**Purpose:** Cache enables reply correlation:
```swift
// Later, reply to same event
try await reply(to: "connect")  // Uses cached message
```

#### Reply Methods - Complete Overloads

**1. Reply with Message (async):**
```swift
public func reply(with message: Message) async throws -> Bool {
    try await delegate.reply(with: message)
}
```

**2. Reply with Message (completion):**
```swift
public func reply(with message: Message, completion: ReplyCompletionHandler? = nil) {
    Task {
        do {
            let result = try await delegate.reply(with: message)
            completion?(.success(result))
        } catch {
            completion?(.failure(error))
        }
    }
}
```

**3. Reply to Event (async):**
```swift
public func reply(to event: String) async throws -> Bool {
    guard let message = receivedMessage(for: event) else {
        logger.warning("bridgeMessageFailedToReply: message for event \(event) was not received")
        return false
    }
    return try await reply(with: message)
}
```

**4. Reply to Event with Data (async):**
```swift
public func reply(to event: String, with jsonData: String) async throws -> Bool {
    guard let message = receivedMessage(for: event) else {
        logger.warning("bridgeMessageFailedToReply")
        return false
    }
    let messageReply = message.replacing(jsonData: jsonData)
    return try await reply(with: messageReply)
}
```

**5. Reply to Event with Encodable (async):**
```swift
public func reply<T: Encodable>(to event: String, with data: T) async throws -> Bool {
    guard let message = receivedMessage(for: event) else {
        logger.warning("bridgeMessageFailedToReply")
        return false
    }
    let messageReply = message.replacing(data: data)
    return try await reply(with: messageReply)
}
```

**All completion handler variants wrap async versions in Task.**

#### Lifecycle Hook Methods

```swift
open func onViewDidLoad() {}
open func onViewWillAppear() {}
open func onViewDidAppear() {}
open func onViewWillDisappear() {}
open func onViewDidDisappear() {}
```

**Default:** Empty implementations. Subclasses override as needed.

**Public Forwarding Methods:**
```swift
public func viewDidLoad() {
    onViewDidLoad()
}
// ... same pattern for all lifecycle methods
```

**Why Both?** `viewDidLoad()` called by delegate, `onViewDidLoad()` overridden by subclass.

---

## Part 6: Helper Extensions

### Data+Utils.swift

```swift
extension Data {
    func decoded<T: Decodable>() throws -> T {
        return try JSONDecoder().decode(T.self, from: self)
    }
}
```

**Usage:** `let data: MyType = try jsonData.decoded()`

### Dictionary+JSON.swift

```swift
extension Dictionary where Key == String, Value == AnyHashable {
    func jsonData() -> Data? {
        guard JSONSerialization.isValidJSONObject(self) else {
            logger.warning("Invalid JSON object: \(self)")
            return nil
        }

        do {
            return try JSONSerialization.data(withJSONObject: self)
        } catch {
            logger.error("JSON serialization error: \(error)")
            return nil
        }
    }
}
```

**Purpose:** Convert Swift dictionary to JSON Data for InternalMessage.

### String+JSON.swift

```swift
extension String {
    func jsonObject() -> Any? {
        guard let jsonData = self.data(using: .utf8) else {
            logger.error("Failed to convert string to data")
            return nil
        }

        do {
            return try JSONSerialization.jsonObject(with: jsonData)
        } catch {
            logger.error("JSON parsing error: \(error)")
            return nil
        }
    }
}
```

**Usage:** Parse JSON strings to dictionaries for equality comparison.

### Encodable+Utils.swift

```swift
extension Encodable {
    func encoded() throws -> Data {
        return try JSONEncoder().encode(self)
    }
}
```

**Usage:** `let data = try myModel.encoded()`

---

## Part 7: Configuration & Logging

### StradaConfig.swift

```swift
public struct StradaConfig {
    public var jsonEncoder: JSONEncoder = JSONEncoder()
    public var jsonDecoder: JSONDecoder = JSONDecoder()

    public var debugLoggingEnabled = false {
        didSet {
            StradaLogger.debugLoggingEnabled = debugLoggingEnabled
        }
    }
}
```

**Custom Encoder Example:**
```swift
let encoder = JSONEncoder()
encoder.keyEncodingStrategy = .convertToSnakeCase
encoder.dateEncodingStrategy = .iso8601
Strada.config.jsonEncoder = encoder
```

**Custom Decoder Example:**
```swift
let decoder = JSONDecoder()
decoder.keyDecodingStrategy = .convertFromSnakeCase
decoder.dateDecodingStrategy = .iso8601
Strada.config.jsonDecoder = decoder
```

### Logging.swift

```swift
enum StradaLogger {
    static var debugLoggingEnabled: Bool = false {
        didSet {
            logger = debugLoggingEnabled ? enabledLogger : disabledLogger
        }
    }
    static let enabledLogger = Logger(subsystem: Bundle.main.bundleIdentifier!,
                                       category: "Strada")
    static let disabledLogger = Logger(.disabled)
}

var logger = StradaLogger.disabledLogger
```

**Usage Pattern:**
```swift
#if DEBUG
    Strada.config.debugLoggingEnabled = true
#endif
```

**OSLog Integration:** Uses unified logging system with app's bundle identifier.

### Strada.swift - Public Namespace

```swift
public enum Strada {
    public static var config: StradaConfig = StradaConfig()

    public static func userAgentSubstring(for componentTypes: [BridgeComponent.Type]) -> String {
        let components = componentTypes.map { $0.name }.joined(separator: " ")
        return "bridge-components: [\(components)]"
    }
}
```

**User-Agent Usage:**
```swift
let config = WKWebViewConfiguration()
let stradaUA = Strada.userAgentSubstring(for: [.form, .page])
config.applicationNameForUserAgent = "Turbo Native iOS \(stradaUA)"
// Result: "Turbo Native iOS bridge-components: [form page]"
```

**Purpose:** Web can detect which native components are supported by parsing User-Agent.

---

## Part 8: Integration Guide

### Step-by-Step Integration

#### 1. Register Bridge Components

```swift
// BridgeComponent+App.swift
extension BridgeComponent {
    static var allTypes: [BridgeComponent.Type] {
        [
            FormComponent.self,
            PageComponent.self,
            ComposerComponent.self
        ]
    }
}
```

#### 2. Configure WKWebView

```swift
// WKWebViewConfiguration+App.swift
extension WKWebViewConfiguration {
    static var appConfiguration: WKWebViewConfiguration {
        let stradaSubstring = Strada.userAgentSubstring(for: BridgeComponent.allTypes)
        let userAgent = "Turbo Native iOS \(stradaSubstring)"

        let configuration = WKWebViewConfiguration()
        configuration.applicationNameForUserAgent = userAgent
        return configuration
    }
}

// SceneController.swift
let webView = WKWebView(frame: .zero, configuration: .appConfiguration)
Bridge.initialize(webView)
```

#### 3. Implement BridgeDestination

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

    override func visitableDidDeactivateWebView() {
        bridgeDelegate.webViewDidBecomeDeactivated()
    }
}
```

#### 4. Create Bridge Component

```swift
final class FormComponent: BridgeComponent {
    override class var name: String { "form" }

    override func onReceive(message: Message) {
        guard let event = Event(rawValue: message.event) else { return }

        switch event {
        case .connect:
            handleConnect(message)
        case .submitEnabled:
            enableSubmit()
        case .submitDisabled:
            disableSubmit()
        }
    }

    private func handleConnect(message: Message) {
        guard let data: FormData = message.data() else { return }
        // Show native button with data.submitTitle
    }

    @objc func submitTapped() {
        Task {
            try? await reply(to: Event.connect.rawValue)
        }
    }
}

private extension FormComponent {
    enum Event: String {
        case connect, submitEnabled, submitDisabled
    }

    struct FormData: Decodable {
        let submitTitle: String
    }
}
```

---

## Part 9: Testing Strategy

### Test Doubles (Spies)

#### BridgeSpy

```swift
final class BridgeSpy: Bridgable {
    var registerComponentWasCalled = false
    var registerComponentArg: String?
    var replyWithMessageWasCalled = false
    var replyWithMessageArg: Message?

    func register(component: String) {
        registerComponentWasCalled = true
        registerComponentArg = component
    }

    func reply(with message: Message) {
        replyWithMessageWasCalled = true
        replyWithMessageArg = message
    }
    // ... other methods
}
```

#### BridgeComponentSpy

```swift
final class BridgeComponentSpy: BridgeComponent {
    static override var name: String { "two" }

    var onReceiveMessageWasCalled = false
    var onViewDidLoadWasCalled = false

    override func onReceive(message: Message) {
        onReceiveMessageWasCalled = true
    }

    override func onViewDidLoad() {
        onViewDidLoadWasCalled = true
    }
}
```

### Example Component Test

```swift
final class ComposerComponentTests: XCTestCase {
    func testSelectSender() async throws {
        let component = ComposerComponent(destination: mockDestination,
                                          delegate: mockDelegate)

        let connectMessage = Message(id: "1", component: "composer",
                                     event: "connect", jsonData: senderListJSON)
        component.didReceive(message: connectMessage)

        try await component.selectSender(emailAddress: "test@example.com")

        XCTAssertTrue(mockDelegate.replyWithMessageWasCalled)
    }
}
```

---

## Part 10: Rust Recreation Architecture

### Crate Structure

```
strada-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API
│   ├── bridge.rs           # Bridge equivalent
│   ├── delegate.rs         # BridgeDelegate
│   ├── component.rs        # BridgeComponent trait
│   ├── message.rs          # Message, InternalMessage
│   ├── javascript.rs       # JS string builder
│   ├── webview/            # WebView abstraction
│   │   ├── mod.rs
│   │   ├── wry.rs         # wry backend
│   │   └── tao.rs         # tao windowing
│   └── utils/
│       ├── json.rs        # JSON helpers
│       └── logger.rs      # tracing integration
├── assets/
│   └── strada.js          # Embedded JavaScript
└── examples/
    └── basic/             # Example integration
```

### Core Types

```rust
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// Message equivalent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub component: String,
    pub event: String,
    pub metadata: Option<MessageMetadata>,
    pub json_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub url: String,
}

// BridgeComponent trait
#[async_trait]
pub trait BridgeComponent: Send + Sync {
    fn name() -> &'static str where Self: Sized;

    async fn on_receive(&self, message: Message);
    async fn on_view_did_load(&self) {}
    async fn on_view_will_appear(&self) {}
    async fn on_view_did_appear(&self) {}
    async fn on_view_will_disappear(&self) {}
    async fn on_view_did_disappear(&self) {}
}

// BridgeDelegate
pub struct BridgeDelegate {
    location: String,
    destination: Arc<dyn BridgeDestination>,
    components: RwLock<HashMap<String, Arc<dyn BridgeComponent>>>,
    bridge: RwLock<Option<Arc<Bridge>>>,
    is_active: AtomicBool,
}

// Bridge
pub struct Bridge {
    webview: Arc<dyn WebView>,
    delegate: RwLock<Option<Arc<BridgeDelegate>>>,
}
```

### Required Crates

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# WebView
wry = "0.37"        # WebView wrapper
tao = "0.26"        # Windowing (if needed)

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Error handling
thiserror = "1"
anyhow = "1"

# Reference counting
arc-swap = "1"
```

### JavaScript Injection

```rust
const STRADA_JS: &str = include_str!("../assets/strada.js");

pub async fn initialize_bridge(webview: &Arc<dyn WebView>) -> Result<()> {
    // Inject strada.js at document start
    webview.add_init_script(STRADA_JS)?;

    // Set up message handler
    webview.add_message_handler("strada", |message| {
        handle_script_message(message)
    });

    Ok(())
}
```

### Message Handler

```rust
fn handle_script_message(body: JsValue) {
    if let Some(event) = body.as_string() {
        if event == "ready" {
            // Bridge initialized - register components
            spawn(async {
                register_components().await;
            });
            return;
        }
    }

    if let Ok(message) = InternalMessage::from_js(&body) {
        let delegate = get_delegate();
        spawn(async move {
            delegate.bridge_did_receive_message(message.to_message()).await;
        });
    }
}
```

---

## Part 11: Sequence Diagrams

### Component Registration Flow

```
Native App          Bridge          BridgeDelegate     strada.js        Web Bridge
    |                 |                   |                |                 |
    |--initialize()-> |                   |                |                 |
    |    (webView)    |                   |                |                 |
    |                 |--loadIntoWebView()->|              |                 |
    |                 |   (inject JS)      |                |                 |
    |                 |                   |                |                 |
    |                 |                   |                |  <document start>
    |                 |                   |                |--execute-------->
    |                 |                   |                |                 |
    |                 |                   |                |--postMessage--->
    |                 |                   |                |   ("ready")     |
    |                 |<--scriptMessage---|                |                 |
    |                 |                   |                |                 |
    |                 |--bridgeDidInit----|                |                 |
    |                 |                   |                |                 |
    |                 |--register(["form","page"])------->|                 |
    |                 |                   |                |                 |
    |                 |                   |                |--resolvePromise->
    |                 |                   |                |                 |
    |                 |                   |                |--setAdapter---->
    |                 |                   |                |                 |
```

### Message Flow (Web -> Native -> Web)

```
Web Component     strada.js       ScriptMessage    Bridge       BridgeDelegate   FormComponent
    |                |                 Handler        |              |                |
    |--send()------->|                |               |              |                |
    |  {event:"connect",             |               |              |                |
    |   data:{...}}  |                |               |              |                |
    |                |--postMessage-->|               |              |                |
    |                |                |--didReceive-->|              |                |
    |                |                |               |              |                |
    |                |                |               |--bridgeDidReceive-->         |
    |                |                |               |              |                |
    |                |                |               |              |--getOrCreate-->|
    |                |                |               |              |   (create)     |
    |                |                |               |              |                |
    |                |                |               |              |--didReceive--->|
    |                |                |               |              |                |--onReceive()
    |                |                |               |              |                |
    |                |                |               |              |                |--(native action)
    |                |                |               |              |                |
    |                |                |               |              |<--reply()------|
    |                |                |               |              |                |
    |                |                |               |<--reply()----|                |
    |                |                |               |              |                |
    |                |<--evaluateJS--------------------|              |                |
    |  receive() <---|  (replyWith)   |               |              |                |
    |<---------------|                |               |              |                |
```

---

## Appendix: Complete File Reference

| File | Lines | Key Responsibilities |
|------|-------|---------------------|
| Bridge.swift | 208 | WebView integration, JS evaluation, instance pool |
| BridgeDelegate.swift | 166 | Lifecycle, message routing, component factory |
| BridgeComponent.swift | 279 | Base component, reply methods, message caching |
| Message.swift | 121 | Public message struct, Codable helpers |
| InternalMessage.swift | 106 | Wire format, JS conversion |
| JavaScript.swift | 48 | JS function string builder |
| ScriptMessageHandler.swift | 18 | WKWebView message handler wrapper |
| strada.js | 81 | Injected JavaScript bridge |
| StradaConfig.swift | 19 | JSON encoder/decoder, debug logging |
| Strada.swift | 10 | Public namespace, User-Agent |
| Logging.swift | 14 | OSLog integration |
| PathLoader*.swift | ~15 | Resource loading (SPM/Xcode) |
| Extensions/*.swift | ~60 | JSON helpers |

---

## Open Questions for Rust Implementation

1. **WebView Choice:** wry vs tauri vs webview-rs - which provides best WKWebView access on macOS/iOS?
2. **Async Model:** How to handle @MainActor equivalent? Single-threaded executor for WebView ops?
3. **FFI Layer:** uniFFI vs swift-bridge for Swift interop if needed?
4. **Resource Bundling:** Best practice for embedding strada.js in Rust crate?
5. **Retain Cycles:** How to replicate weak reference pattern in Rust for WebView?
6. **Error Recovery:** What happens when JS evaluation fails mid-navigation?
7. **Backpressure:** Should there be rate limiting for high-frequency messages?

---

## Rust Implementation Checklist

- [ ] Create Message struct with serde
- [ ] Implement InternalMessage with JS serialization
- [ ] Build Bridge struct with WebView trait
- [ ] Create BridgeDelegate with lifecycle
- [ ] Define BridgeComponent trait
- [ ] Embed strada.js as const
- [ ] Implement message handler
- [ ] Add component registry
- [ ] Create JSON helper extensions
- [ ] Set up tracing logging
- [ ] Write unit tests
- [ ] Create example integration
- [ ] Document API
- [ ] Publish to crates.io
