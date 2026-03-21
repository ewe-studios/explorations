# Strada Android - Complete Exploration

## Overview

**Strada Android** is a native adapter for Strada-enabled web apps that enables bidirectional communication between a `WebView` and native Kotlin code. It allows building native UI components that are driven by web-based components, creating a seamless bridge between web and native functionality on Android.

### Key Architecture Principles

1. **Component-based architecture**: Each feature is encapsulated in a `BridgeComponent` subclass
2. **Message-passing bridge**: Communication happens through structured `Message` objects
3. **Lifecycle-aware**: Components respect the Android Fragment/Activity lifecycle via `DefaultLifecycleObserver`
4. **Type-safe messaging**: JSON data is decoded using Kotlinx Serialization

### Repository Structure

```
strada-android/
├── strada/
│   ├── src/main/
│   │   ├── kotlin/dev/hotwire/strada/
│   │   │   ├── Strada.kt              # Global configuration and user agent utilities
│   │   │   ├── Bridge.kt              # Core bridge between native and web
│   │   │   ├── BridgeComponent.kt     # Base class for native components
│   │   │   ├── BridgeComponentFactory.kt # Factory for component creation
│   │   │   ├── BridgeDelegate.kt      # Delegate handling bridge lifecycle
│   │   │   ├── BridgeDestination.kt   # Marker interface for destinations
│   │   │   ├── Message.kt             # Message structure for bridge communication
│   │   │   ├── InternalMessage.kt     # Internal message format for JSON serialization
│   │   │   ├── Repository.kt          # Asset loading (strada.js)
│   │   │   ├── StradaConfig.kt        # Configuration (JSON converter, logging)
│   │   │   ├── StradaJsonConverter.kt # JSON converter abstraction
│   │   │   ├── JsonExtensions.kt      # Kotlinx serialization extensions
│   │   │   ├── Helpers.kt             # Threading utilities (runOnUiThread)
│   │   │   └── StradaLog.kt           # Logging system
│   │   └── assets/js/
│   │       └── strada.js              # JavaScript injected into WebView
│   └── src/test/kotlin/               # Unit tests
├── docs/
│   ├── OVERVIEW.md                    # High-level overview
│   ├── INSTALLATION.md                # Installation instructions
│   ├── QUICK-START.md                 # Integration guide
│   ├── BUILD-COMPONENTS.md            # Component building guide
│   └── ADVANCED-OPTIONS.md            # Advanced configuration
└── README.md
```

## Core Concepts

### The Bridge

The `Bridge` class (`Bridge.kt:14-138`) is the central communication channel:
- Adds `JavascriptInterface` named "StradaNative" to the WebView
- Loads `strada.js` from assets into the WebView
- Provides methods to register/unregister components via JavaScript evaluation
- Handles message reception via `@JavascriptInterface` annotated methods

### Bridge Components

`BridgeComponent` (`BridgeComponent.kt:3-123`) is the base class for all native components:
- Each component has a unique `name` that matches its web counterpart
- Receives messages via `onReceive(message: Message)`
- Can reply to messages using various `replyTo()` / `replyWith()` methods
- Maintains a cache of received messages per event type
- Lifecycle callbacks: `onStart()`, `onStop()`

### Bridge Delegate

`BridgeDelegate` (`BridgeDelegate.kt:8-110`) acts as the intermediary:
- Implements `DefaultLifecycleObserver` for lifecycle awareness
- Connects the Bridge to the destination (Fragment)
- Manages component creation via factories
- Routes incoming messages to appropriate components
- Tracks active/inactive destinations

### Messages

`Message` (`Message.kt:3-61`) is the data structure for bridge communication:
- `id`: Unique identifier for correlating requests/replies
- `component`: Name of the sending component
- `event`: Event type (e.g., "connect", "submit", "display")
- `metadata`: URL and other metadata
- `jsonData`: JSON-encoded payload

## Communication Flow

### Initialization Sequence

```
1. App creates WebView with Bridge.initialize(webView)
2. Bridge adds JavascriptInterface "StradaNative"
3. Bridge.load() reads strada.js from assets and evaluates it
4. strada.js creates window.nativeBridge instance
5. window.nativeBridge.ready() calls StradaNative.bridgeDidInitialize()
6. @JavascriptInterface bridgeDidInitialize() triggers delegate.bridgeDidInitialize()
7. Bridge registers all component factories with web via window.nativeBridge.register()
8. strada.js calls webBridge.setAdapter(this) when strada-web is ready
9. Bridge is now ready for message passing
```

### Message Flow (Web to Native)

```
1. Web component calls window.Strada.web.send(message)
2. strada.js NativeBridge.receive(message) is invoked
3. NativeBridge.postMessage(JSON.stringify(message))
4. Native: @JavascriptInterface bridgeDidReceiveMessage(message: String)
5. Bridge parses InternalMessage.fromJson(message)
6. Bridge delegates to BridgeDelegate.bridgeDidReceiveMessage()
7. BridgeDelegate routes to component via getOrCreateComponent()
8. BridgeComponent.didReceive(message) caches and calls onReceive(message)
9. Component's onReceive(message) handles the event
```

### Message Flow (Native to Web)

```
1. Native component calls replyWith(message) or replyTo(event)
2. BridgeDelegate.replyWith(message) forwards to Bridge
3. Bridge.replyWith(message) creates InternalMessage
4. Bridge.evaluate(javascript) generates: window.nativeBridge.replyWith({...})
5. WebView.evaluateJavascript() executes the call
6. NativeBridge.replyWith(message) receives in JavaScript
7. NativeBridge.webBridge.receive(message) delivers to web component
```

## Key Differences from iOS Version

| Aspect | iOS | Android |
|--------|-----|---------|
| **WebView** | `WKWebView` with `WKScriptMessageHandler` | `WebView` with `JavascriptInterface` |
| **JS Injection** | `WKUserScript` at `.atDocumentStart` | `evaluateJavascript()` after page load |
| **Message Reception** | `WKScriptMessageHandler` protocol | `@JavascriptInterface` annotation |
| **Lifecycle** | Manual delegation to view controller | `DefaultLifecycleObserver` automatic |
| **Threading** | `@MainActor` annotations | `runOnUiThread()` with Handler |
| **JSON** | `Codable` with `JSONEncoder/Decoder` | Kotlinx Serialization |
| **Component Factory** | Type-based with `BridgeComponent.Type` | Factory pattern with `BridgeComponentFactory` |

## Platform-Specific Implementation Details

### JavaScript Interface

Android uses the `@JavascriptInterface` annotation for secure JS-to-Native communication:

```kotlin
// Bridge.kt:70-91
@JavascriptInterface
fun bridgeDidInitialize() {
    logEvent("bridgeDidInitialize", "success")
    runOnUiThread {
        delegate?.bridgeDidInitialize()
    }
}

@JavascriptInterface
fun bridgeDidReceiveMessage(message: String?) {
    runOnUiThread {
        InternalMessage.fromJson(message)?.let {
            delegate?.bridgeDidReceiveMessage(it.toMessage())
        }
    }
}
```

**Important:** All interface methods must run UI operations on the main thread via `runOnUiThread()`.

### WebView Initialization

```kotlin
// Bridge.kt:22-29
init {
    webViewRef = WeakReference(webView)
    // The JavascriptInterface must be added before the page is loaded
    webView.addJavascriptInterface(this, bridgeJavascriptInterface)
}
```

The interface **must** be added before loading any content.

### Lifecycle Integration

```kotlin
// BridgeDelegate.kt:12
class BridgeDelegate<D : BridgeDestination>(
    val location: String,
    val destination: D,
    private val componentFactories: List<BridgeComponentFactory<D, BridgeComponent<D>>>
) : DefaultLifecycleObserver {
```

By implementing `DefaultLifecycleObserver`, the delegate automatically receives:
- `onStart(owner: LifecycleOwner)` - Destination becomes active
- `onStop(owner: LifecycleOwner)` - Destination becomes inactive
- `onDestroy(owner: LifecycleOwner)` - Cleanup

### Cold Boot Handling

```kotlin
// BridgeDelegate.kt:22-28
fun onColdBootPageCompleted() {
    bridge?.load()
}

fun onColdBootPageStarted() {
    bridge?.reset()
}
```

Android handles WebView recreation (cold boots) differently than iOS:
- `onColdBootPageStarted()`: Reset registration state
- `onColdBootPageCompleted()`: Reload the JavaScript bridge

## Component Factory Pattern

### Factory Definition

```kotlin
// BridgeComponentFactory.kt:3-8
class BridgeComponentFactory<D : BridgeDestination, out C : BridgeComponent<D>> constructor(
    val name: String,
    private val creator: (name: String, delegate: BridgeDelegate<D>) -> C
) {
    fun create(delegate: BridgeDelegate<D>) = creator(name, delegate)
}
```

### Factory Registration

```kotlin
// BUILD-COMPONENTS.md:117-123
val bridgeComponentFactories = listOf(
    BridgeComponentFactory("form", ::FormComponent),
    BridgeComponentFactory("page", ::PageComponent)
)
```

### Lazy Component Creation

```kotlin
// BridgeDelegate.kt:106-109
private fun getOrCreateComponent(name: String): BridgeComponent<D>? {
    val factory = componentFactories.firstOrNull { it.name == name } ?: return null
    return initializedComponents.getOrPut(name) { factory.create(this) }
}
```

Components are created on-demand when their first message arrives.

## JSON Serialization

### Kotlinx Serialization

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

### Message Data Extraction

```kotlin
// Message.kt:54-56
inline fun <reified T> data(): T? {
    return StradaJsonConverter.toObject(jsonData)
}

// Usage in component:
val data: MessageData? = message.data()
```

### Custom Converter

```kotlin
// QUICK-START.md:66-68
class MainActivity : AppCompatActivity(), TurboActivity {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        configStrada()
    }

    private fun configStrada() {
        Strada.config.jsonConverter = KotlinXJsonConverter()
    }
}
```

### Converter Abstraction

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
    }
}
```

## User Agent Configuration

```kotlin
// Strada.kt:6-9
fun userAgentSubstring(componentFactories: List<BridgeComponentFactory<*,*>>): String {
    val components = componentFactories.joinToString(" ") { it.name }
    return "bridge-components: [$components]"
}
```

**Usage:**
```kotlin
// QUICK-START.md:44-49
private val WebView.customUserAgent: String
    get() {
        val turboSubstring = Turbo.userAgentSubstring()
        val stradaSubstring = Strada.userAgentSubstring(bridgeComponentFactories)
        return "$turboSubstring; $stradaSubstring; ${settings.userAgentString}"
    }
```

## Threading Model

### Main Thread Enforcement

```kotlin
// Helpers.kt:11-16
internal fun runOnUiThread(func: () -> Unit) {
    when (val mainLooper = Looper.getMainLooper()) {
        Looper.myLooper() -> func()
        else -> Handler(mainLooper).post { func() }
    }
}
```

This ensures:
1. Direct execution if already on main thread (useful for tests)
2. Posts to main Handler if on background thread

### JavascriptInterface Thread Safety

All `@JavascriptInterface` methods must handle threading:

```kotlin
@JavascriptInterface
fun bridgeDidReceiveMessage(message: String?) {
    runOnUiThread {
        // Process message on main thread
        delegate?.bridgeDidReceiveMessage(it.toMessage())
    }
}
```

## Repository Pattern

### Asset Loading

```kotlin
// Repository.kt:5-11
internal class Repository {
    fun getUserScript(context: Context): String {
        return context.assets.open("js/strada.js").use {
            String(it.readBytes())
        }
    }
}
```

The JavaScript file is stored in `src/main/assets/js/strada.js` and loaded at runtime.

## Deprecation Notice

> **Important**: Strada Android is being deprecated in favor of [Hotwire Native](https://native.hotwired.dev), which consolidates Turbo Native and Strada into a single framework. For new development, use [Hotwire Native Android](https://github.com/hotwired/hotwire-native-android).

---

*This exploration document covers the architecture, communication patterns, and implementation details of Strada Android. Subsequent deep dive documents explore specific subsystems in greater detail.*
