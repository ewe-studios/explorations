# Strada Android - WebView and JavaScript Bridge Deep Dive

## Overview

This document explores how Strada Android sets up and configures the Android `WebView` to enable bidirectional communication between native Kotlin code and JavaScript in the web page.

## Architecture

### Key Components

```
┌─────────────────────────────────────────────────────────────────┐
│                      Native (Kotlin)                            │
│  ┌──────────────┐  ┌─────────────┐  ┌──────────────────────┐   │
│  │   Bridge     │◄─┤BridgeDelegate│◄─┤ BridgeComponent(s)  │   │
│  └──────┬───────┘  └─────────────┘  └──────────────────────┘   │
│         │                                                       │
│  ┌──────▼──────────────────────────────────────────────────┐   │
│  │  @JavascriptInterface "StradaNative"                    │   │
│  │  - bridgeDidInitialize()                                │   │
│  │  - bridgeDidReceiveMessage(message: String)             │   │
│  │  - bridgeDidUpdateSupportedComponents()                 │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                            │
                            │ javascript:stradaNative.methodName()
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                    WebView (JavaScript)                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              window.nativeBridge (strada.js)            │   │
│  │              StradaNative.bridgeDidReceiveMessage()     │   │
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

```kotlin
// Bridge.kt:122-137
companion object {
    private val instances = mutableListOf<Bridge>()

    fun initialize(webView: WebView) {
        if (getBridgeFor(webView) == null) {
            initialize(Bridge(webView))
        }
    }

    @VisibleForTesting
    internal fun initialize(bridge: Bridge) {
        instances.add(bridge)
        instances.removeIf { it.webView == null }
    }

    internal fun getBridgeFor(webView: WebView): Bridge? {
        return instances.firstOrNull { it.webView == webView }
    }
}
```

**Key behaviors:**
1. Checks if a Bridge already exists for this WebView (singleton pattern)
2. Creates a new Bridge instance if none exists
3. Stores the bridge in a mutable list with weak references

### Bridge Instance Creation

```kotlin
// Bridge.kt:14-29
@Suppress("unused")
class Bridge internal constructor(webView: WebView) {
    private var componentsAreRegistered: Boolean = false
    private val webViewRef: WeakReference<WebView>

    internal val webView: WebView? get() = webViewRef.get()
    internal var repository = Repository()
    internal var delegate: BridgeDelegate<*>? = null

    init {
        webViewRef = WeakReference(webView)
        // The JavascriptInterface must be added before the page is loaded
        webView.addJavascriptInterface(this, bridgeJavascriptInterface)
    }
}
```

The initializer:
1. Stores a **weak reference** to the webView (prevents memory leaks)
2. Sets up the Repository for asset loading
3. **Critically:** Adds the JavascriptInterface before any page loads

## JavaScript Interface

### Interface Definition

The `@JavascriptInterface` annotation exposes Kotlin methods to JavaScript:

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
fun bridgeDidUpdateSupportedComponents() {
    logEvent("bridgeDidUpdateSupportedComponents", "success")
    componentsAreRegistered = true
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

**Security note:** Only annotated methods are accessible from JavaScript.

### Threading Requirements

All interface methods use `runOnUiThread()`:

```kotlin
// Helpers.kt:11-16
internal fun runOnUiThread(func: () -> Unit) {
    when (val mainLooper = Looper.getMainLooper()) {
        Looper.myLooper() -> func()  // Already on main thread
        else -> Handler(mainLooper).post { func() }  // Post to main queue
    }
}
```

This is required because:
1. WebView callbacks may come from background threads
2. UI operations must run on the main thread
3. Tests may already be on the main thread (avoids double-posting)

### Constants

```kotlin
// Bridge.kt:10-11
private const val bridgeGlobal = "window.nativeBridge"
private const val bridgeJavascriptInterface = "StradaNative"
```

These must match the JavaScript implementation.

## JavaScript Injection

### Loading the Script

```kotlin
// Bridge.kt:56-59
internal fun load() {
    logEvent("bridgeWillLoad")
    evaluate(userScript())
}

internal fun userScript(): String {
    val context = requireNotNull(webView?.context)
    return repository.getUserScript(context)
}
```

### Repository Asset Loading

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

The script is:
- Stored in `src/main/assets/js/strada.js`
- Read as bytes and converted to String
- Evaluated in the WebView context

### Asset Directory Structure

```
strada/
└── src/main/
    └── assets/
        └── js/
            └── strada.js
```

## JavaScript Bridge (strada.js)

### NativeBridge Class

```javascript
// strada.js:1-107
class NativeBridge {
    constructor() {
        this.supportedComponents = []
        this.adapterIsRegistered = false
    }

    register(component) {
        if (Array.isArray(component)) {
            this.supportedComponents = this.supportedComponents.concat(component)
        } else {
            this.supportedComponents.push(component)
        }

        if (!this.adapterIsRegistered) {
            this.registerAdapter()
        }
        this.notifyBridgeOfSupportedComponentsUpdate()
    }
    // ...
}

window.nativeBridge = new NativeBridge()
window.nativeBridge.ready()
```

**Key differences from iOS version:**

| Aspect | iOS | Android |
|--------|-----|---------|
| **Initialization** | Auto on script load | `ready()` called explicitly |
| **Adapter registration** | Promise-based | Flag-based (`adapterIsRegistered`) |
| **Platform** | `"ios"` | `"android"` |

### Native Method Calls

```javascript
// strada.js:72-82
ready() {
    StradaNative.bridgeDidInitialize()
}

supportedComponentsUpdated() {
    StradaNative.bridgeDidUpdateSupportedComponents()
}

postMessage(message) {
    StradaNative.bridgeDidReceiveMessage(message)
}
```

These call the `@JavascriptInterface` methods directly.

### Message Serialization

```javascript
// strada.js:54-64
replyWith(message) {
    if (this.isStradaAvailable) {
        this.webBridge.receive(JSON.parse(message))
    }
}

receive(message) {
    this.postMessage(JSON.stringify(message))
}
```

**Important:** Android uses JSON string serialization, while iOS uses objects.

### DOM Ready Handling

```javascript
// strada.js:95-106
if (document.readyState === 'interactive' || document.readyState === 'complete') {
    initializeBridge()
} else {
    document.addEventListener("DOMContentLoaded", () => {
        initializeBridge()
    })
}

function initializeBridge() {
    window.nativeBridge = new NativeBridge()
    window.nativeBridge.ready()
}
```

This ensures the bridge initializes at the correct time in the page lifecycle.

## JavaScript Evaluation

### evaluate() Method

```kotlin
// Bridge.kt:100-103
internal fun evaluate(javascript: String) {
    logEvent("evaluatingJavascript", javascript)
    webView?.evaluateJavascript(javascript) {}
}
```

Uses Android's `evaluateJavascript()` method with an empty callback.

### generateJavaScript() Helper

```kotlin
// Bridge.kt:105-117
internal fun generateJavaScript(bridgeFunction: String, vararg arguments: JsonElement): String {
    val functionName = sanitizeFunctionName(bridgeFunction)
    val encodedArguments = encode(arguments.toList())
    return "$bridgeGlobal.$functionName($encodedArguments)"
}

internal fun encode(arguments: List<JsonElement>): String {
    return arguments.joinToString(",") { it.toJson() }
}

internal fun sanitizeFunctionName(name: String): String {
    return name.removeSuffix("()")
}
```

**Example output:**
```kotlin
generateJavaScript("register", "form".toJsonElement())
// Produces: "window.nativeBridge.register(\"form\")"
```

### JsonElement Serialization

```kotlin
// JsonExtensions.kt:13
internal inline fun <reified T> T.toJsonElement() = json.encodeToJsonElement(this)
```

Uses Kotlinx Serialization to convert to JSON elements.

## Registration Flow

### Register Single Component

```kotlin
// Bridge.kt:31-35
internal fun register(component: String) {
    logEvent("bridgeWillRegisterComponent", component)
    val javascript = generateJavaScript("register", component.toJsonElement())
    evaluate(javascript)
}
```

### Register Multiple Components

```kotlin
// Bridge.kt:37-41
internal fun register(components: List<String>) {
    logEvent("bridgeWillRegisterComponents", components.joinToString())
    val javascript = generateJavaScript("register", components.toJsonElement())
    evaluate(javascript)
}
```

### Unregister Component

```kotlin
// Bridge.kt:43-47
internal fun unregister(component: String) {
    logEvent("bridgeWillUnregisterComponent", component)
    val javascript = generateJavaScript("unregister", component.toJsonElement())
    evaluate(javascript)
}
```

## Reply Flow

### replyWith() Implementation

```kotlin
// Bridge.kt:49-54
internal fun replyWith(message: Message) {
    logEvent("bridgeWillReplyWithMessage", message.toString())
    val internalMessage = InternalMessage.fromMessage(message)
    val javascript = generateJavaScript("replyWith", internalMessage.toJson().toJsonElement())
    evaluate(javascript)
}
```

Converts `Message` → `InternalMessage` → JSON → JavaScript call.

## State Management

### Registration Tracking

```kotlin
// Bridge.kt:15, 66-68
private var componentsAreRegistered: Boolean = false

internal fun isReady(): Boolean {
    return componentsAreRegistered
}
```

### Reset on Cold Boot

```kotlin
// Bridge.kt:61-64
internal fun reset() {
    logEvent("bridgeDidReset")
    componentsAreRegistered = false
}
```

Called when WebView is recreated (cold boot scenario).

## Initialization Sequence (Detailed)

### Step-by-Step Flow

```
1. Native: Bridge.initialize(webView)
   └─> Bridge.init(webView: WebView)
       └─> webView.addJavascriptInterface(this, "StradaNative")

2. Native: Bridge.load() (called by delegate)
   └─> evaluate(userScript())
       └─> Reads strada.js from assets
       └─> WebView.evaluateJavascript(script, callback)

3. WebView: strada.js executes
   └─> Checks document.readyState
   └─> window.nativeBridge = new NativeBridge()
   └─> window.nativeBridge.ready()
       └─> StradaNative.bridgeDidInitialize()

4. Native: @JavascriptInterface bridgeDidInitialize()
   └─> runOnUiThread { delegate?.bridgeDidInitialize() }

5. Native: BridgeDelegate.bridgeDidInitialize()
   └─> Gets component factory names
   └─> bridge?.register(componentFactories.map { it.name })
       └─> generateJavaScript("register", ["form", "page"])
       └─> evaluateJavascript("window.nativeBridge.register(...)")

6. WebView: NativeBridge.register() executes
   └─> Adds components to supportedComponents
   └─> Calls registerAdapter() if not already registered
       └─> webBridge.setAdapter(this)

7. Complete - Bridge is now ready for message passing
```

## Cold Boot Handling

### WebView Recreation

Android WebViews can be recreated (e.g., configuration changes):

```kotlin
// BridgeDelegate.kt:26-28
fun onColdBootPageStarted() {
    bridge?.reset()
}
```

### Reload After Cold Boot

```kotlin
// BridgeDelegate.kt:22-24
fun onColdBootPageCompleted() {
    bridge?.load()
}
```

### Should Reload Check

```kotlin
// BridgeDelegate.kt:73-75
private fun shouldReloadBridge(): Boolean {
    return destination.bridgeWebViewIsReady() && bridge?.isReady() == false
}
```

This handles the case where the Bridge state was lost but WebView wasn't.

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
val stradaSubstring = Strada.userAgentSubstring(bridgeComponentFactories)
val userAgent = "Turbo Native Android; $stradaSubstring"
webView.settings.userAgentString = userAgent
```

**Example output:**
```
Turbo Native Android; bridge-components: [form page composer]
```

## Logging

### Event Logging

```kotlin
// Bridge.kt:32
logEvent("bridgeWillRegisterComponent", component)
```

### Warning Logging

```kotlin
// BridgeDelegate.kt:40
logWarning("bridgeNotInitializedForWebView", resolvedLocation)
```

### Error Logging

```kotlin
// JsonExtensions.kt:20
logError("jsonStringDecodeException", e)
```

## Comparison with iOS

### JavaScript Interface vs Script Message Handler

| Aspect | iOS (WKWebView) | Android (WebView) |
|--------|-----------------|-------------------|
| **Communication** | `WKScriptMessageHandler` | `@JavascriptInterface` |
| **Direction** | Post message → delegate | Direct method call |
| **Serialization** | Object (converted) | JSON string |
| **Threading** | Main actor | runOnUiThread |
| **Security** | Handler name check | Annotation-based |

### Script Injection Timing

| Aspect | iOS | Android |
|--------|-----|---------|
| **When** | `.atDocumentStart` | After page load |
| **How** | `WKUserScript` | `evaluateJavascript` |
| **Guarantee** | Before any JS runs | After DOM ready |

## Performance Considerations

### Weak Reference Usage

```kotlin
private val webViewRef: WeakReference<WebView>
```

Prevents memory leaks when WebView is garbage collected.

### Instance Cleanup

```kotlin
// Bridge.kt:130-131
instances.add(bridge)
instances.removeIf { it.webView == null }
```

Removes dead references during initialization.

### Asset Loading

The JavaScript file is:
- Loaded from assets on each `load()` call
- Should be cached by the system
- Consider caching in memory for performance

## Security Considerations

### JavascriptInterface Security

Only `@JavascriptInterface` annotated methods are accessible:
- Prevents reflection attacks
- Requires explicit opt-in for each method
- API level 17+ requirement

### Message Validation

```kotlin
// Bridge.kt:85-90
@JavascriptInterface
fun bridgeDidReceiveMessage(message: String?) {
    runOnUiThread {
        InternalMessage.fromJson(message)?.let {
            delegate?.bridgeDidReceiveMessage(it.toMessage())
        }
    }
}
```

Uses safe parsing with `?.let` to handle invalid messages.

---

*This deep dive covers the WebView setup, JavaScript interface, and bidirectional communication architecture for Android. The next documents explore the message structure, component system, and lifecycle management.*
