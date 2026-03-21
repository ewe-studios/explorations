# Strada Android - Message Passing Deep Dive

## Overview

This document explores the message structure, serialization, and bidirectional communication patterns between native Kotlin and JavaScript in Strada Android.

## Message Structure

### Public Message API

```kotlin
// Message.kt:3-57
data class Message constructor(
    /**
     * A unique identifier for this message
     */
    val id: String,

    /**
     * The component the message is sent from (e.g. - "form", "page", etc)
     */
    val component: String,

    /**
     * The event that this message is about: "submit", "display", "send"
     */
    val event: String,

    /**
     * The metadata associated with the message, which includes its url
     */
    val metadata: Metadata?,

    /**
     * Data, represented in a json object string
     */
    val jsonData: String
)
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

```kotlin
// Message.kt:59-61
data class Metadata(
    val url: String
)
```

### replacing() Method

```kotlin
// Message.kt:36-45
fun replacing(
    event: String = this.event,
    jsonData: String = this.jsonData
) = Message(
    id = this.id,
    component = this.component,
    event = event,
    metadata = this.metadata,
    jsonData = jsonData
)
```

### Generic replacing() with Data

```kotlin
// Message.kt:47-52
inline fun <reified T> replacing(
    event: String = this.event,
    data: T
): Message {
    return replacing(event, StradaJsonConverter.toJson(data))
}
```

Uses the configured JSON converter to serialize the data object.

### Data Extraction

```kotlin
// Message.kt:54-56
inline fun <reified T> data(): T? {
    return StradaJsonConverter.toObject(jsonData)
}
```

## Internal Message Format

### InternalMessage Data Class

For JSON serialization, messages use a kotlinx.serialization format:

```kotlin
// InternalMessage.kt:7-32
@Serializable
internal data class InternalMessage(
    @SerialName("id") val id: String,
    @SerialName("component") val component: String,
    @SerialName("event") val event: String,
    @SerialName("data") val data: JsonElement = "{}".parseToJsonElement()
) {
    fun toMessage() = Message(
        id = id,
        component = component,
        event = event,
        metadata = data.decode<InternalDataMetadata>()?.let { Metadata(url = it.metadata.url) },
        jsonData = data.toJson()
    )

    companion object {
        fun fromMessage(message: Message) = InternalMessage(
            id = message.id,
            component = message.component,
            event = message.event,
            data = message.jsonData.parseToJsonElement()
        )

        fun fromJson(json: String?) = json?.decode<InternalMessage>()
    }
}
```

### Metadata Extraction

```kotlin
// InternalMessage.kt:34-42
@Serializable
internal data class InternalDataMetadata(
    @SerialName("metadata") val metadata: InternalMetadata
)

@Serializable
internal data class InternalMetadata(
    @SerialName("url") val url: String
)
```

The metadata is nested inside the data object under the `metadata` key.

## Message Flow Examples

### Web to Native Flow

```javascript
// Web sends message
window.Strada.web.send({
    id: "msg-123",
    component: "form",
    event: "connect",
    data: {
        submitTitle: "Submit",
        metadata: { url: "https://example.com/form" }
    }
})
```

```
1. JavaScript: window.Strada.web.send(message)
2. strada.js: NativeBridge.receive(message)
3. strada.js: this.postMessage(JSON.stringify(message))
4. Native: @JavascriptInterface bridgeDidReceiveMessage(message: String)
5. Native: InternalMessage.fromJson(message)
6. Native: delegate.bridgeDidReceiveMessage(it.toMessage())
7. Native: getOrCreateComponent(name)?.didReceive(message)
```

### Native to Web Flow

```kotlin
// Native sends reply
val message = Message(
    id = "msg-123",
    component = "form",
    event: "connected",
    metadata = null,
    jsonData = """{"enabled":true}"""
)
bridge.replyWith(message)
```

```
1. Native: bridge.replyWith(message)
2. Native: InternalMessage.fromMessage(message)
3. Native: generateJavaScript("replyWith", internalMessage.toJson())
4. Native: webView.evaluateJavascript("window.nativeBridge.replyWith(...)")
5. strada.js: NativeBridge.replyWith(message)
6. strada.js: this.webBridge.receive(JSON.parse(message))
7. Web: Web component receives message
```

## Message Reply Patterns

### Reply Methods Overview

BridgeComponent provides multiple reply methods:

```kotlin
// Reply with full message
fun replyWith(message: Message): Boolean

// Reply to event with original data
fun replyTo(event: String): Boolean

// Reply to event with new JSON data
fun replyTo(event: String, jsonData: String): Boolean

// Reply to event with Encodable data (reified generic)
inline fun <reified T> replyTo(event: String, data: T): Boolean
```

### Message Caching

Received messages are cached by event:

```kotlin
// BridgeComponent.kt:7, 29-32
private val receivedMessages = hashMapOf<String, Message>()

fun didReceive(message: Message) {
    receivedMessages[message.event] = message
    onReceive(message)
}
```

### receivedMessageFor() Helper

```kotlin
// BridgeComponent.kt:12-14
protected fun receivedMessageFor(event: String): Message? {
    return receivedMessages[event]
}
```

### Reply Implementation

**Reply with original message:**
```kotlin
// BridgeComponent.kt:83-90
fun replyTo(event: String): Boolean {
    val message = receivedMessageFor(event) ?: run {
        logWarning("bridgeMessageFailedToReply", "message for event '$event' was not received")
        return false
    }
    return reply(message)
}
```

**Reply with new JSON data:**
```kotlin
// BridgeComponent.kt:99-106
fun replyTo(event: String, jsonData: String): Boolean {
    val message = receivedMessageFor(event) ?: run {
        logWarning("bridgeMessageFailedToReply", "message for event '$event' was not received")
        return false
    }
    return reply(message.replacing(jsonData = jsonData))
}
```

**Reply with Encodable object:**
```kotlin
// BridgeComponent.kt:116-118
inline fun <reified T> replyTo(event: String, data: T): Boolean {
    return replyTo(event, jsonData = StradaJsonConverter.toJson(data))
}
```

### Private Reply Helper

```kotlin
// BridgeComponent.kt:120-122
private fun reply(message: Message): Boolean {
    return delegate.replyWith(message)
}
```

## JSON Serialization

### Kotlinx Serialization Setup

```kotlin
// JsonExtensions.kt:31-37
@OptIn(ExperimentalSerializationApi::class)
private val json = Json {
    ignoreUnknownKeys = true
    encodeDefaults = true
    explicitNulls = false
    isLenient = true
}
```

### Extension Functions

```kotlin
// JsonExtensions.kt:11-29
internal fun String.parseToJsonElement() = json.parseToJsonElement(this)

internal inline fun <reified T> T.toJsonElement() = json.encodeToJsonElement(this)

internal inline fun <reified T> T.toJson() = json.encodeToString(this)

internal inline fun <reified T> JsonElement.decode(): T? = try {
    json.decodeFromJsonElement<T>(this)
} catch (e: Exception) {
    logError("jsonElementDecodeException", e)
    null
}

internal inline fun <reified T> String.decode(): T? = try {
    json.decodeFromString<T>(this)
} catch (e: Exception) {
    logError("jsonStringDecodeException", e)
    null
}
```

### StradaJsonConverter

```kotlin
// StradaJsonConverter.kt:8-36
abstract class StradaJsonConverter {
    companion object {
        inline fun <reified T> toObject(jsonData: String): T? {
            val converter = requireNotNull(Strada.config.jsonConverter) { NO_CONVERTER }

            return when (converter) {
                is KotlinXJsonConverter -> converter.toObject<T>(jsonData)
                is StradaJsonTypeConverter -> converter.toObject(jsonData, T::class.java)
                else -> throw IllegalStateException(INVALID_CONVERTER)
            }
        }

        inline fun <reified T> toJson(data: T): String {
            val converter = requireNotNull(Strada.config.jsonConverter) { NO_CONVERTER }

            return when (converter) {
                is KotlinXJsonConverter -> converter.toJson(data)
                is StradaJsonTypeConverter -> converter.toJson(data, T::class.java)
                else -> throw IllegalStateException(INVALID_CONVERTER)
            }
        }
    }
}
```

### KotlinXJsonConverter

```kotlin
// StradaJsonConverter.kt:44-69
class KotlinXJsonConverter : StradaJsonConverter() {
    @OptIn(ExperimentalSerializationApi::class)
    val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
        explicitNulls = false
        isLenient = true
    }

    inline fun <reified T> toObject(jsonData: String): T? {
        return try {
            json.decodeFromString(jsonData)
        } catch (e: Exception) {
            logException(e)
            null
        }
    }

    inline fun <reified T> toJson(data: T): String {
        return json.encodeToString(data)
    }
}
```

### Custom Type Converter

For other serialization libraries (Gson, Moshi, etc.):

```kotlin
// StradaJsonConverter.kt:39-42
abstract class StradaJsonTypeConverter : StradaJsonConverter() {
    abstract fun <T> toObject(jsonData: String, type: Class<T>): T?
    abstract fun <T> toJson(data: T, type: Class<T>): String
}
```

## Message Equality

### Data Class Equality

Kotlin data classes provide automatic equals/hashCode:

```kotlin
@Suppress("EqualsOrHashCode")
data class Message(...) {
    // Automatic equals() based on all properties
}
```

## Event Pattern

### Event Handling in Components

```kotlin
// BUILD-COMPONENTS.md:27-40
override fun onReceive(message: Message) {
    when (message.event) {
        "connect" -> handleConnectEvent(message)
        "submitEnabled" -> handleSubmitEnabled()
        "submitDisabled" -> handleSubmitDisabled()
        else -> Log.w("TurboDemo", "Unknown event for message: $message")
    }
}
```

### Event Constants

Components often define event constants:

```kotlin
private object Event {
    const val CONNECT = "connect"
    const val SUBMIT_ENABLED = "submitEnabled"
    const val SUBMIT_DISABLED = "submitDisabled"
}
```

## Data Class Examples

### Message Data Class

```kotlin
// BUILD-COMPONENTS.md:61-64
@Serializable
data class MessageData(
    @SerialName("submitTitle") val title: String
)
```

### Usage

```kotlin
private fun handleConnectEvent(message: Message) {
    val data: MessageData? = message.data()
    data?.let {
        // Use it.submitTitle
    }
}
```

## Error Handling

### Missing Bridge

```kotlin
// BridgeDelegate.kt:49-56
fun replyWith(message: Message): Boolean {
    bridge?.replyWith(message) ?: run {
        logWarning("bridgeMessageFailedToReply", "bridge is not available")
        return false
    }
    return true
}
```

### Missing Message for Event

```kotlin
// BridgeComponent.kt:84-87
val message = receivedMessageFor(event) ?: run {
    logWarning("bridgeMessageFailedToReply", "message for event '$event' was not received")
    return false
}
```

### JSON Conversion Errors

```kotlin
// StradaJsonConverter.kt:53-59
inline fun <reified T> toObject(jsonData: String): T? {
    return try {
        json.decodeFromString(jsonData)
    } catch (e: Exception) {
        logException(e)
        null
    }
}
```

Returns `null` on error instead of throwing.

## Logging

### Log Levels

```kotlin
// StradaLog.kt
internal fun logEvent(event: String, data: String)
internal fun logWarning(event: String, data: String)
internal fun logError(event: String, exception: Exception)
```

### Common Log Messages

| Event | Level | When |
|-------|-------|------|
| `bridgeWillRegisterComponent` | Debug | Before registering component |
| `bridgeWillReplyWithMessage` | Debug | Before sending reply |
| `bridgeDidReceiveMessage` | Debug | After receiving message |
| `bridgeMessageFailedToReply` | Warning | Reply failure |
| `bridgeNotInitializedForWebView` | Warning | Missing bridge |
| `jsonStringDecodeException` | Error | JSON parse error |

## Configuration

### Debug Logging

```kotlin
// StradaConfig.kt:16
var debugLoggingEnabled = false
```

Enable during development, disable in production.

### JSON Converter Setup

```kotlin
// QUICK-START.md:66-68
Strada.config.jsonConverter = KotlinXJsonConverter()
```

Must be set before using `message.data<T>()` or `replyTo(event, data)`.

## Comparison with iOS

### Serialization Approach

| Aspect | iOS | Android |
|--------|-----|---------|
| **Library** | Foundation JSONEncoder/Decoder | Kotlinx Serialization |
| **Protocol** | Codable | @Serializable |
| **Generic access** | `message.data<T>()` | `message.data<T>()` |
| **Custom converter** | StradaConfig.jsonEncoder/Decoder | StradaJsonConverter |

### Message Caching

| Aspect | iOS | Android |
|--------|-----|---------|
| **Storage** | `[String: Message]` | `hashMapOf<String, Message>` |
| **Access** | `receivedMessage(for:)` | `receivedMessageFor(_:)` |

### Reply Methods

| iOS | Android |
|-----|---------|
| `reply(to:) async throws -> Bool` | `replyTo(event: String): Boolean` |
| `reply(to:with:) async throws -> Bool` | `replyTo(event:jsonData:): Boolean` |
| `reply(to:with:) async throws -> Bool` | `replyTo(event:data:): Boolean` |
| `reply(with:) async throws -> Bool` | `replyWith(message:): Boolean` |

## Performance Considerations

### Inlined Reified Generics

```kotlin
inline fun <reified T> data(): T?
```

Uses `inline` and `reified` to avoid runtime type erasure overhead.

### String vs JsonElement

- `jsonData` is stored as `String` (compatible with JavaScript)
- Internal conversion uses `JsonElement` for efficient manipulation
- Final serialization happens at the boundary

### Hash-Based Message Cache

```kotlin
private val receivedMessages = hashMapOf<String, Message>()
```

O(1) lookup by event name.

## Testing

### Test Data

```kotlin
// TestData.kt
object TestData {
    fun createMessage(
        id: String = "test-id",
        component: String = "test",
        event: String = "test-event",
        jsonData: String = "{}"
    ): Message = Message(id, component, event, null, jsonData)
}
```

### Message Tests

```kotlin
// MessageTest.kt
@Test
fun `message replacing creates new message with updated event`() {
    val message = Message("1", "form", "connect", null, "{}")
    val replaced = message.replacing(event = "submit")

    assertEquals("submit", replaced.event)
    assertEquals("1", replaced.id)
}

@Test
fun `message data extracts typed data`() {
    val json = """{"title":"Submit"}"""
    val message = Message("1", "form", "connect", null, json)

    Strada.config.jsonConverter = KotlinXJsonConverter()
    val data: MessageData? = message.data()

    assertEquals("Submit", data?.title)
}
```

---

*This deep dive covers the complete message structure, Kotlinx serialization, and communication patterns for Android. The next document explores the BridgeComponent system and lifecycle management.*
