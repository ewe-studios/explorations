# Strada iOS - BridgeComponent System Deep Dive

## Overview

This document explores the BridgeComponent architecture, lifecycle management, and how native components are created, managed, and destroyed in response to web component messages.

## Component Architecture

### Class Hierarchy

```
BridgingComponent (protocol)
       ▲
       │
BridgeComponent (class)
       ▲
       │
┌──────┴──────┬─────────────┬──────────────┐
│             │             │              │
FormComponent  PageComponent  ComposerComponent  (your components)
```

### BridgingComponent Protocol

```swift
// BridgeComponent.swift:4-24
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

The protocol defines:
- **Static properties**: Component identification
- **Initializer**: Dependency injection
- **Message handling**: `onReceive(message:)`
- **Lifecycle hooks**: View lifecycle methods
- **Internal methods**: Wrapper methods (not meant to be overridden)

## BridgeComponent Base Class

### Class Definition

```swift
// BridgeComponent.swift:27-37
@MainActor
open class BridgeComponent: BridgingComponent {
    public typealias ReplyCompletionHandler = (Result<Bool, Error>) -> Void

    /// A unique name representing the BridgeComponent type.
    nonisolated open class var name: String {
        fatalError("BridgeComponent subclass must provide a unique 'name'")
    }

    public unowned let delegate: BridgingDelegate

    required public init(destination: BridgeDestination, delegate: BridgingDelegate) {
        self.delegate = delegate
    }
    // ...
}
```

### Key Design Decisions

**1. `nonisolated` class var for name:**
```swift
nonisolated open class var name: String
```
Allows access without actor isolation, useful for building component lists.

**2. `unowned` delegate reference:**
```swift
public unowned let delegate: BridgingDelegate
```
Avoids retain cycle while assuming delegate outlives component.

**3. `open` class for subclassing:**
Designed to be subclassed, not used directly.

### Required Subclass Implementation

```swift
// FormComponent example
final class FormComponent: BridgeComponent {
    override class var name: String { "form" }

    override func onReceive(message: Message) {
        guard let event = Event(rawValue: message.event) else { return }
        switch event {
        case .connect: handleConnect(message: message)
        case .submitEnabled: handleSubmitEnabled()
        case .submitDisabled: handleSubmitDisabled()
        }
    }
}
```

## Component Lifecycle

### Lifecycle States

```
                    ┌─────────────┐
                    │   Created   │
                    │ (lazy init) │
                    └──────┬──────┘
                           │
                           ▼
              ┌────────────────────────┐
              │  didReceive(message:)  │
              │  └─> onReceive(message)│
              └───────────┬────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│ viewDidLoad() │ │viewWillAppear()│ │ viewDidAppear()│
│ onViewDidLoad()│ │onViewWillAppear()│ │onViewDidAppear()│
└───────────────┘ └───────────────┘ └───────────────┘
        │                 │                 │
        └─────────────────┼─────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│viewWillDisappear()│ │viewDidDisappear()│ │   Deactivated │
│onViewWillDisappear()│ │onViewDidDisappear()│ │  (destination  │
└───────────────┘ └───────────────┘ │   inactive)   │
                          └───────────────┘
```

### Lifecycle Method Flow

**Internal wrappers call user-overridable methods:**

```swift
// BridgeComponent.swift:240-242
public func viewDidLoad() {
    onViewDidLoad()
}

// BridgeComponent.swift:231-234
public func didReceive(message: Message) {
    receivedMessages[message.event] = message
    onReceive(message: message)
}
```

### View Lifecycle Integration

BridgeDelegate forwards view controller lifecycle events:

```swift
// BridgeDelegate.swift:77-104
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

## Component Creation and Management

### Lazy Initialization

Components are created on-demand when their first message arrives:

```swift
// BridgeDelegate.swift:152-165
private func getOrCreateComponent(name: String) -> BridgeComponent? {
    if let component = initializedComponents[name] {
        return component
    }

    guard let componentType = componentTypes.first(where: { $0.name == name }) else {
        return nil
    }

    let component = componentType.init(destination: destination, delegate: self)
    initializedComponents[name] = component

    return component
}
```

**Key behaviors:**
1. Checks cache first (singleton per destination)
2. Finds component type by name
3. Creates new instance with dependency injection
4. Caches for future use

### Component Routing

```swift
// BridgeDelegate.swift:125-137
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

**Routing conditions:**
1. Destination must be active (not disappeared)
2. Message URL must match current location
3. Component must be registered

### Active Components Filter

```swift
// BridgeDelegate.swift:148-150
private var activeComponents: [BridgeComponent] {
    return initializedComponents.values.filter { _ in destinationIsActive }
}
```

Components only receive lifecycle events when destination is active.

## BridgeDelegate Architecture

### Protocol Definition

```swift
// BridgeDelegate.swift:7-26
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

### BridgeDelegate Implementation

```swift
// BridgeDelegate.swift:29-44
@MainActor
public final class BridgeDelegate: BridgingDelegate {
    public let location: String
    public unowned let destination: BridgeDestination
    public var webView: WKWebView? { bridge?.webView }

    weak var bridge: Bridgable?

    public init(location: String,
                destination: BridgeDestination,
                componentTypes: [BridgeComponent.Type]) {
        self.location = location
        self.destination = destination
        self.componentTypes = componentTypes
    }
    // ...
}
```

### State Management

```swift
// BridgeDelegate.swift:141-146
private var initializedComponents: [String: BridgeComponent] = [:]
private var destinationIsActive = false
private let componentTypes: [BridgeComponent.Type]
private var resolvedLocation: String {
    webView?.url?.absoluteString ?? location
}
```

| Property | Purpose |
|----------|---------|
| `initializedComponents` | Cache of created components |
| `destinationIsActive` | Tracks if view controller is visible |
| `componentTypes` | Registered component factory types |
| `resolvedLocation` | Current URL (for message routing) |

### WebView Activation

```swift
// BridgeDelegate.swift:46-58
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

**Bridge retrieval uses weak reference:**
```swift
// Bridge.swift:104-106
static func getBridgeFor(_ webView: WKWebView) -> Bridge? {
    return instances.first { $0.webView == webView }
}
```

### Component Retrieval

```swift
// BridgeDelegate.swift:108-110
public func component<C: BridgeComponent>() -> C? {
    return activeComponents.compactMap { $0 as? C }.first
}
```

Type-safe component access:
```swift
if let formComponent: FormComponent = bridgeDelegate.component() {
    // Use formComponent
}
```

## Destination Protocol

### BridgeDestination

```swift
// BridgeDelegate.swift:4
public protocol BridgeDestination: AnyObject {}
```

Empty marker protocol for type safety. View controllers conform to this:

```swift
final class TurboWebViewController: VisitableViewController, BridgeDestination {
    // BridgeDelegate uses this as the destination
}
```

### Bridgable Protocol

```swift
// Bridge.swift:8-16
protocol Bridgable: AnyObject {
    var delegate: BridgeDelegate? { get set }
    var webView: WKWebView? { get }

    func register(component: String) async throws
    func register(components: [String]) async throws
    func unregister(component: String) async throws
    func reply(with message: Message) async throws
}
```

Bridge conforms to this for dependency injection in tests.

## Component Registration

### Registration Flow

**1. Define component types:**
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

**2. Pass to BridgeDelegate:**
```swift
// TurboWebViewController.swift
private lazy var bridgeDelegate: BridgeDelegate = {
    BridgeDelegate(location: visitableURL.absoluteString,
                   destination: self,
                   componentTypes: BridgeComponent.allTypes)
}()
```

**3. Register with web on init:**
```swift
// BridgeDelegate.swift:114-123
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

**4. Web receives registration:**
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

## Reply Patterns

### Reply Methods

All reply methods eventually call through to the bridge:

```swift
// BridgeComponent.swift:57-59
public func reply(with message: Message) async throws -> Bool {
    try await delegate.reply(with: message)
}
```

Which calls:
```swift
// BridgeDelegate.swift:65-73
public func reply(with message: Message) async throws -> Bool {
    guard let bridge else {
        logger.warning("bridgeMessageFailedToReply: bridge is not available")
        return false
    }
    try await bridge.reply(with: message)
    return true
}
```

### Reply Chain

```
BridgeComponent.reply(with:)
       │
       ▼
BridgeDelegate.reply(with:)
       │
       ▼
Bridge.reply(with:)
       │
       ▼
callBridgeFunction(.replyWith, args)
       │
       ▼
evaluateJavaScript("window.nativeBridge.replyWith(...)")
       │
       ▼
NativeBridge.replyWith(message)
       │
       ▼
window.Strada.web.receive(message)
```

## Testing Support

### Spy Classes

```
Tests/Spies/
├── BridgeSpy.swift
├── BridgeComponentSpy.swift
└── BridgeDelegateSpy.swift
```

**BridgeComponentSpy example:**
```swift
final class BridgeComponentSpy: BridgeComponent {
    static override var name: String { "spy" }

    var onReceiveCalled = false
    var receivedMessage: Message?

    override func onReceive(message: Message) {
        onReceiveCalled = true
        receivedMessage = message
    }
}
```

### Test Example

```swift
// ComposerComponentTests.swift
func test_selectSender_repliesWithSelectedIndex() async throws {
    let component = ComposerComponent(destination: mockDestination,
                                       delegate: mockDelegate)

    let message = Message(id: "1", component: "composer", event: "connect",
                          metadata: nil, jsonData: sendersJSON)
    component.didReceive(message: message)

    try await component.selectSender(emailAddress: "test@example.com")

    XCTAssertTrue(mockDelegate.replyCalled)
    XCTAssertEqual(mockDelegate.repliedMessage?.event, "select-sender")
}
```

## Memory Management

### Retain Cycle Prevention

| Relationship | Pattern | Reason |
|--------------|---------|--------|
| Bridge → WebView | `weak` | WebView owns Bridge via configuration |
| ScriptMessageHandler → Delegate | `weak` | Avoids WKWebView retain cycle |
| BridgeDelegate → Bridge | `weak` | Bridge may outlive delegate |
| BridgeDelegate → Destination | `unowned` | Destination owns delegate |
| BridgeComponent → Delegate | `unowned` | Delegate owns component |

### Instance Storage

```swift
// Bridge.swift:110
private static var instances: [Bridge] = []
```

Bridges are stored statically (weak webView references).

Cleanup:
```swift
// Bridge.swift:99-102
static func initialize(_ bridge: Bridge) {
    instances.append(bridge)
    instances.removeAll { $0.webView == nil }
}
```

## Component Communication

### Direct Component Access

Components can access each other through the delegate:

```swift
if let otherComponent = delegate.component::<OtherComponent>() {
    // Communicate with other component
}
```

### Shared State via Destination

Since all components share the same destination:

```swift
class ComposerComponent: BridgeComponent {
    func updateSender() {
        guard let viewController = delegate.destination as? TurboWebViewController else {
            return
        }
        // Access shared state in destination
    }
}
```

## Common Component Patterns

### Event Enum Pattern

```swift
extension FormComponent {
    private enum Event: String {
        case connect
        case submitEnabled = "submit-enabled"
        case submitDisabled = "submit-disabled"
    }
}
```

### Message Data Pattern

```swift
extension FormComponent {
    struct MessageData: Decodable {
        let submitTitle: String
        let isEnabled: Bool
    }

    struct ReplyData: Encodable {
        let submitted: Bool
    }
}
```

### Handler Method Pattern

```swift
override func onReceive(message: Message) {
    guard let event = Event(rawValue: message.event) else { return }
    switch event {
    case .connect: handleConnect(message: message)
    case .submitEnabled: handleSubmitEnabled()
    case .submitDisabled: handleSubmitDisabled()
    }
}

private func handleConnect(message: Message) {
    guard let data: MessageData = message.data() else { return }
    // Implementation
}
```

## Lifecycle Edge Cases

### Component Created After viewDidLoad

If the first message arrives after `viewDidLoad`:

```swift
// BridgeDelegate.swift:77-81
public func onViewDidLoad() {
    destinationIsActive = true
    activeComponents.forEach { $0.viewDidLoad() }
}
```

Only `activeComponents` receive the event. A newly created component won't have received `viewDidLoad` yet.

**Solution:** Component checks state on first message:

```swift
override func onReceive(message: Message) {
    case .connect:
        // Component created, view is already loaded
        setupUI()
}
```

### Message After viewDidDisappear

```swift
// BridgeDelegate.swift:127-131
guard destinationIsActive,
      resolvedLocation == message.metadata?.url else {
    logger.warning("bridgeDidIgnoreMessage: \(String(describing: message))")
    return false
}
```

Messages are ignored after destination disappears.

## Customization Points

### Overridable Methods

```swift
open func onReceive(message: Message)
open func onViewDidLoad()
open func onViewWillAppear()
open func onViewDidAppear()
open func onViewWillDisappear()
open func onViewDidDisappear()
```

### Non-Overridable Methods

These are implementation details:

```swift
public func didReceive(message: Message)
public func viewDidLoad()
public func viewWillAppear()
// ... etc
```

### Custom Reply Methods

Add component-specific reply methods:

```swift
extension FormComponent {
    func submit() async throws {
        let data = ReplyData(submitted: true)
        try await reply(to: Event.connect.rawValue, with: data)
    }
}
```

---

*This deep dive covers the BridgeComponent architecture, lifecycle management, and component communication patterns. The next document will explore Rust/iOS interop considerations for reimplementation.*
