# Strada iOS - Message Passing Deep Dive

## Overview

This document explores the message structure, serialization, and bidirectional communication patterns between native Swift and JavaScript in Strada iOS.

## Message Structure

### Public Message API

```swift
// Message.swift:5-33
public struct Message: Equatable {
    /// A unique identifier for this message
    public let id: String

    /// The component the message is sent from (e.g. - "form", "page", etc)
    public let component: String

    /// The event that this message is about: "submit", "display", "send"
    public let event: String

    /// The metadata associated with the message, which includes its url
    public let metadata: Metadata?

    /// Data, represented in a json object string
    public let jsonData: String
}
```

### Message Fields

| Field | Type | Purpose | Example |
|-------|------|---------|---------|
| `id` | `String` | Unique correlation ID | `"msg-123-abc"` |
| `component` | `String` | Component name | `"form"`, `"page"` |
| `event` | `String` | Event type | `"connect"`, `"submit"` |
| `metadata` | `Metadata?` | URL and context | `Metadata(url: "https://...")` |
| `jsonData` | `String` | JSON payload | `"{\"title\":\"Hello\"}"` |

### Metadata Structure

```swift
// Message.swift:88-96
extension Message {
    public struct Metadata: Equatable {
        public let url: String

        public init(url: String) {
            self.url = url
        }
    }
}
```

The metadata currently only contains the URL, but provides an extension point for future context data.

## Internal Message Format

### InternalMessage Structure

For JSON serialization, messages are converted to an internal format:

```swift
// InternalMessage.swift:6-20
struct InternalMessage {
    let id: String
    let component: String
    let event: String
    let data: InternalMessageData  // [String: AnyHashable]
}
```

**Key difference:** `data` is a dictionary instead of a JSON string.

### Conversion from Message

```swift
// InternalMessage.swift:22-28
init(from message: Message) {
    let data = (message.jsonData.jsonObject() as? InternalMessageData) ?? [:]
    self.init(id: message.id,
              component: message.component,
              event: message.event,
              data: data)
}
```

Parses the JSON string back to a dictionary for JavaScript serialization.

### Conversion to Message

```swift
// InternalMessage.swift:57-63
func toMessage() -> Message {
    return Message(id: id,
                   component: component,
                   event: event,
                   metadata: metadata(),
                   jsonData: dataAsJSONString() ?? "{}")
}
```

Extracts metadata from data and converts data back to JSON string.

### Metadata Extraction

```swift
// InternalMessage.swift:77-82
private func metadata() -> Message.Metadata? {
    guard let jsonData = data.jsonData(),
          let internalMetadata: InternalMessage.DataMetadata = try? jsonData.decoded()
    else { return nil }

    return Message.Metadata(url: internalMetadata.metadata.url)
}
```

The metadata is nested inside the data object under a `_metadata` key (convention used by strada-web).

## Message Creation and Sending

### From JavaScript to Native

```javascript
// Web component sends message
window.Strada.web.send({
  id: "msg-123",
  component: "form",
  event: "connect",
  data: {
    submitTitle: "Submit",
    _metadata: { url: "https://example.com/form" }
  }
})
```

Flow:
```
1. Web: window.Strada.web.send(message)
2. strada.js: NativeBridge.receive(message)
3. strada.js: this.postMessage(message)
4. WebKit: webkit.messageHandlers.strada.postMessage(message)
5. Native: ScriptMessageHandler.userContentController(_:didReceive:)
6. Native: delegate?.scriptMessageHandlerDidReceiveMessage(scriptMessage)
```

### From Native to JavaScript

```swift
// Native sends reply
let message = Message(id: "msg-123",
                      component: "form",
                      event: "connected",
                      metadata: nil,
                      jsonData: "{\"enabled\":true}")
try await bridge.reply(with: message)
```

Flow:
```
1. Native: bridge.reply(with: message)
2. Native: InternalMessage(from: message)
3. Native: callBridgeFunction(.replyWith, arguments: [internalMessage.toJSON()])
4. Native: evaluateJavaScript("window.nativeBridge.replyWith({...})")
5. strada.js: NativeBridge.replyWith(message)
6. strada.js: this.webBridge.receive(message)
7. Web: Web component receives message
```

## Message Reply Patterns

### Reply Methods Overview

BridgeComponent provides multiple reply methods:

```swift
// MARK: Reply with full message
func reply(with message: Message) async throws -> Bool
func reply(with message: Message, completion: ReplyCompletionHandler?)

// MARK: Reply to event with original data
func reply(to event: String) async throws -> Bool
func reply(to event: String, completion: ReplyCompletionHandler?)

// MARK: Reply to event with new JSON data
func reply(to event: String, with jsonData: String) async throws -> Bool
func reply(to event: String, with jsonData: String, completion: ReplyCompletionHandler?)

// MARK: Reply to event with Encodable data
func reply<T: Encodable>(to event: String, with data: T) async throws -> Bool
func reply<T: Encodable>(to event: String, with data: T, completion: ReplyCompletionHandler?)
```

### Message Caching

Received messages are cached by event:

```swift
// BridgeComponent.swift:231-234
public func didReceive(message: Message) {
    receivedMessages[message.event] = message
    onReceive(message: message)
}

// BridgeComponent.swift:196-198
public func receivedMessage(for event: String) -> Message? {
    return receivedMessages[event]
}
```

This enables `reply(to:)` to find the original message.

### Reply Implementation

**Reply with original message:**
```swift
// BridgeComponent.swift:85-92
public func reply(to event: String) async throws -> Bool {
    guard let message = receivedMessage(for: event) else {
        logger.warning("bridgeMessageFailedToReply: message for event \(event) was not received")
        return false
    }
    return try await reply(with: message)
}
```

**Reply with new data:**
```swift
// BridgeComponent.swift:122-130
public func reply(to event: String, with jsonData: String) async throws -> Bool {
    guard let message = receivedMessage(for: event) else {
        logger.warning("bridgeMessageFailedToReply: message for event \(event) was not received")
        return false
    }
    let messageReply = message.replacing(jsonData: jsonData)
    return try await reply(with: messageReply)
}
```

**Reply with Encodable object:**
```swift
// BridgeComponent.swift:162-170
public func reply<T: Encodable>(to event: String, with data: T) async throws -> Bool {
    guard let message = receivedMessage(for: event) else {
        logger.warning("bridgeMessageFailedToReply: message for event \(event) was not received")
        return false
    }
    let messageReply = message.replacing(data: data)
    return try await reply(with: messageReply)
}
```

## Message Replacement

### replacing(jsonData:)

```swift
// Message.swift:42-49
public func replacing(event updatedEvent: String? = nil,
                      jsonData updatedData: String? = nil) -> Message {
    Message(id: id,
            component: component,
            event: updatedEvent ?? event,
            metadata: metadata,
            jsonData: updatedData ?? jsonData)
}
```

### replacing(data:) with Encodable

```swift
// Message.swift:56-68
public func replacing<T: Encodable>(event updatedEvent: String? = nil,
                                    data: T) -> Message {
    let updatedData: String?
    do {
        let jsonData = try Strada.config.jsonEncoder.encode(data)
        updatedData = String(data: jsonData, encoding: .utf8)
    } catch {
        logger.error("Error encoding codable object: \(String(describing: data)) -> \(error)")
        updatedData = nil
    }
    return replacing(event: updatedEvent, jsonData: updatedData)
}
```

Uses the configurable JSONEncoder from Strada.config.

## Data Serialization

### JSON Encoding/Decoding

**Dictionary to JSON:**
```swift
// Dictionary+JSON.swift:3-17
extension Dictionary where Key == String, Value == AnyHashable {
    func jsonData() -> Data? {
        guard JSONSerialization.isValidJSONObject(self) else {
            logger.warning("The provided object is not a valid JSON object. \(self)")
            return nil
        }
        let data = try? JSONSerialization.data(withJSONObject: self)
        return data
    }
}
```

**Data to Codable:**
```swift
// Data+Utils.swift:3-7
extension Data {
    func decoded<T: Decodable>() throws -> T {
        return try JSONDecoder().decode(T.self, from: self)
    }
}
```

### Message Data Extraction

```swift
// Message.swift:72-85
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

Uses the configurable JSONDecoder from Strada.config.

### Custom Encoder/Decoder Configuration

```swift
// StradaConfig.swift:4-12
public var jsonEncoder: JSONEncoder = JSONEncoder()
public var jsonDecoder: JSONDecoder = JSONDecoder()
```

Can be customized for specific date formats, key naming strategies, etc.

## Message Equality

### Semantic Equality

```swift
// Message.swift:114-120
public static func == (lhs: Self, rhs: Self) -> Bool {
    return lhs.id == rhs.id &&
    lhs.component == rhs.component &&
    lhs.event == rhs.event &&
    lhs.metadata == rhs.metadata &&
    lhs.jsonData.jsonObject() as? [String: AnyHashable] == rhs.jsonData.jsonObject() as? [String: AnyHashable]
}
```

**Important:** JSON data is compared semantically, not as strings. This means:
```swift
let lhs = Message(jsonData: "{\"a\":1,\"b\":2}")
let rhs = Message(jsonData: "{\"b\":2,\"a\":1}")
lhs == rhs  // true (same content, different order)
```

## Event Pattern

### Defining Events

Components typically define events as nested enums:

```swift
// ComposerComponent.swift:51-58
extension ComposerComponent {
    private enum InboundEvent: String {
        case connect
    }

    private enum OutboundEvent: String {
        case selectSender = "select-sender"
    }
}
```

### Event Handling

```swift
// ComposerComponent.swift:8-18
override func onReceive(message: Message) {
    guard let event = InboundEvent(rawValue: message.event) else {
        return
    }

    switch event {
    case .connect:
        // Handle connect event if needed.
        break
    }
}
```

## Message Flow Examples

### Form Component Connect Flow

**Step 1: Web sends connect message**
```javascript
// Web (form component)
window.Strada.web.send({
  id: "form-connect-1",
  component: "form",
  event: "connect",
  data: {
    submitTitle: "Submit Form",
    _metadata: { url: "https://example.com/form" }
  }
})
```

**Step 2: Native receives and processes**
```swift
// Native (FormComponent.onReceive)
override func onReceive(message: Message) {
    guard let event = Event(rawValue: message.event) else { return }

    switch event {
    case .connect:
        guard let data: MessageData = message.data() else { return }
        // data.submitTitle = "Submit Form"
        displaySubmitButton(title: data.submitTitle)
    }
}
```

**Step 3: Native replies when button tapped**
```swift
@objc func performAction() {
    reply(to: Event.connect.rawValue)
}
```

This sends back the original message, acknowledging the action.

### Data Round-Trip Example

**Web to Native with data:**
```swift
struct MessageData: Decodable {
    let email: String
    let index: Int
}

let message: Message = // received from web
if let data: MessageData = message.data() {
    print("Email: \(data.email), Index: \(data.index)")
}
```

**Native to Web with reply data:**
```swift
struct ReplyData: Encodable {
    let selectedIndex: Int
}

let replyData = ReplyData(selectedIndex: 2)
try await reply(to: "connect", with: replyData)
// Sends: { id: "...", component: "...", event: "connect", data: { selectedIndex: 2 } }
```

## Error Handling

### Missing WebView

```swift
// Bridge.swift:78-81
guard let webView else {
    throw BridgeError.missingWebView
}
```

### Invalid JavaScript Arguments

```swift
// JavaScript.swift:27-30
private func encode(arguments: [Any]) throws -> String {
    guard JSONSerialization.isValidJSONObject(arguments) else {
        throw JavaScriptError.invalidArgumentType
    }
    // ...
}
```

### Missing Message for Event

```swift
// BridgeComponent.swift:86-88
guard let message = receivedMessage(for: event) else {
    logger.warning("bridgeMessageFailedToReply: message for event \(event) was not received")
    return false
}
```

## Logging

### Debug Logging Configuration

```swift
// StradaConfig.swift:14-18
public var debugLoggingEnabled = false {
    didSet {
        StradaLogger.debugLoggingEnabled = debugLoggingEnabled
    }
}

// Logging.swift:4-14
enum StradaLogger {
    static var debugLoggingEnabled: Bool = false
    static let enabledLogger = Logger(subsystem: Bundle.main.bundleIdentifier!,
                                       category: "Strada")
    static let disabledLogger = Logger(.disabled)
}

var logger = StradaLogger.disabledLogger
```

### Log Messages

| Log Level | When | Example |
|-----------|------|---------|
| `debug` | Bridge initialized | `"bridgeDidInitialize"` |
| `debug` | Message received | `"bridgeDidReceiveMessage \(message)"` |
| `debug` | View lifecycle | `"bridgeDestinationViewDidLoad: \(location)"` |
| `warning` | Unhandled message | `"Unhandled message received: \(body)"` |
| `warning` | Missing bridge | `"bridgeNotInitializedForWebView"` |
| `error` | JSON encode/decode | `"Error encoding codable object: \(data)"` |
| `error` | JavaScript error | `"Error evaluating JavaScript: \(error)"` |

## Completion Handler Pattern

Both async/await and completion handler versions are provided:

```swift
// Async/await
let success = try await reply(to: "connect", with: data)

// Completion handler
reply(to: "connect", with: data) { result in
    switch result {
    case .success(let success):
        print("Reply succeeded: \(success)")
    case .failure(let error):
        print("Reply failed: \(error)")
    }
}
```

The completion handler version wraps the async version:

```swift
// BridgeComponent.swift:182-190
public func reply<T: Encodable>(to event: String, with data: T,
                                 completion: ReplyCompletionHandler? = nil) {
    Task {
        do {
            let result = try await reply(to: event, with: data)
            completion?(.success(result))
        } catch {
            completion?(.failure(error))
        }
    }
}
```

## Performance Considerations

### Message Caching Strategy

Messages are cached per event, allowing:
- Multiple replies to the same event
- Retrieval of original message data
- Memory grows with unique events, not total messages

### JSON Serialization

- Uses Foundation's `JSONSerialization` (not Codable for generic dicts)
- Codable only for typed `data<T>()` extraction
- Custom encoder/decoder allows optimization

### Main Thread Requirement

All message passing must occur on main thread:
```swift
@MainActor
func reply(with message: Message) async throws -> Bool
```

This is required because `evaluateJavaScript` must run on main thread.

---

*This deep dive covers the complete message structure, serialization, and communication patterns. The next document explores the BridgeComponent system and lifecycle management.*
