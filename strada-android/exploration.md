---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/strada-android
repository: https://github.com/hotwired/strada-android
explored_at: 2026-03-20
language: Kotlin
---

# COMPREHENSIVE DEEP EXPLORATION: Strada Android

## Executive Summary

Strada Android is a **bidirectional message bridge** between JavaScript running in a WebView and native Kotlin/Android code. It enables building native Android UI components that are controlled by web-based logic, forming a core part of the Hotwire Native stack alongside Turbo Native.

**Deprecation Notice:** As of 2024, Strada Android is being deprecated in favor of [Hotwire Native Android](https://github.com/hotwired/hotwire-native-android), which consolidates Turbo Native and Strada into a single framework. However, understanding Strada's architecture remains valuable for maintaining existing apps and understanding the Hotwire ecosystem.

### Architecture Pattern: Message-Based Bridge

```
┌─────────────────────────────────────────────────────────────────┐
│                         WEBVIEW (Android WebView)                │
│  ┌──────────────┐    ┌───────────────┐    ┌─────────────────┐  │
│  │ Web Bridge   │◄──►│  strada.js    │◄──►│ window.Strada   │  │
│  │ (strada-web) │    │  (Injected)   │    │ NativeBridge    │  │
│  └──────────────┘    └───────────────┘    └────────┬────────┘  │
└─────────────────────────────────────────────────────┼───────────┘
                                                      │ receive/post
                                                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                      NATIVE Android (Kotlin)                     │
│  ┌────────────────┐  ┌────────────────┐  ┌───────────────────┐  │
│  │ Bridge         │◄─┤ BridgeDelegate │◄─┤ JavascriptInterface│ │
│  │ (WebView       │  │ (Lifecycle +   │  │ (@JavascriptInterface)│
│  │  integration)  │  │  Routing)      │  │                   │  │
│  └───────┬────────┘  └───────┬────────┘  └───────────────────┘  │
│          │                   │                                   │
│          ▼                   ▼                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │           BridgeComponent instances                       │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐          │  │
│  │  │ Form       │  │ Page       │  │ Composer   │  ...     │  │
│  │  │ Component  │  │ Component  │  │ Component  │          │  │
│  │  └────────────┘  └────────────┘  └────────────┘          │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Core Design Principles

1. **JavascriptInterface Pattern** - Uses Android's `@JavascriptInterface` for JS→Native communication
2. **Component Factory Pattern** - Components registered by name, created on-demand
3. **Lifecycle-Aware Routing** - Messages delivered only when destination is active
4. **Type-Safe Serialization** - kotlinx.serialization for message data encoding/decoding
5. **Thread Safety** - Main thread enforcement for UI operations via Handler/Looper

---

## Repository & Build

| Property | Value |
|----------|-------|
| **Location** | `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/strada-android` |
| **Remote** | https://github.com/hotwired/strada-android |
| **Language** | Kotlin 1.9.10 |
| **License** | MIT License (37signals/Basecamp) |
| **Min SDK** | Android API 26 (Android 8.0) |
| **Target SDK** | API 34 (Android 14) |
| **Build System** | Gradle 8.2.2 with Kotlin DSL |
| **Serialization** | kotlinx.serialization 1.5.0 |

### Build Configuration

**Root build.gradle.kts:**
```kotlin
buildscript {
    dependencies {
        classpath("com.android.tools.build:gradle:8.2.2")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.10")
        classpath("org.jetbrains.kotlin:kotlin-serialization:1.9.10")
    }
}
```

**Library build.gradle.kts:**
```kotlin
plugins {
    id("com.android.library")
    id("kotlin-android")
    id("kotlinx-serialization")
    id("maven-publish")
    id("signing")
}

android {
    compileSdk = 34
    testOptions.unitTests.isIncludeAndroidResources = true
    testOptions.unitTests.isReturnDefaultValues = true

    defaultConfig {
        minSdk = 26
        targetSdk = 34
    }

    kotlinOptions {
        jvmTarget = JavaVersion.VERSION_17.toString()
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.5.0")
    implementation("androidx.lifecycle:lifecycle-common:2.7.0")
}
```

**Key Dependencies:**
- `androidx.lifecycle:lifecycle-common` - For DefaultLifecycleObserver
- `kotlinx-serialization-json` - For JSON serialization
- `androidx.core:core-ktx` - Kotlin extensions for Android

---

## Complete Directory Structure

```
strada-android/
├── .github/                              # GitHub workflows, issue templates
├── docs/                                 # User documentation
│   ├── INSTALLATION.md                   # SPM/Gradle integration
│   ├── OVERVIEW.md                       # High-level architecture
│   ├── QUICK-START.md                    # Step-by-step integration guide
│   ├── BUILD-COMPONENTS.md               # Creating custom BridgeComponents
│   ├── ADVANCED-OPTIONS.md               # Custom JSON converters, debug logging
│   └── CONDUCT.md                        # Code of conduct
├── strada/                               # Library source code (~2,500 lines)
│   ├── build.gradle.kts                  # Library build configuration
│   ├── proguard-consumer-rules.pro       # ProGuard rules for consumers
│   ├── proguard-rules.pro                # Internal ProGuard rules
│   └── src/
│       ├── main/
│       │   ├── AndroidManifest.xml       # Library manifest
│       │   ├── assets/
│       │   │   └── js/
│       │   │       └── strada.js         # Injected JavaScript (81 lines)
│       │   └── kotlin/dev/hotwire/strada/
│       │       ├── Core Bridge Files
│       │       │   ├── Bridge.kt                  # WebView integration, JS interface
│       │       │   ├── BridgeDelegate.kt          # Lifecycle management, message routing
│       │       │   ├── BridgeComponent.kt         # Base class for native components
│       │       │   ├── BridgeComponentFactory.kt  # Factory for component creation
│       │       │   └── BridgeDestination.kt       # Marker interface for destinations
│       │       │
│       │       ├── Message System
│       │       │   ├── Message.kt                 # Public message data class
│       │       │   ├── InternalMessage.kt         # Internal wire format
│       │       │   └── Repository.kt              # Asset loading for strada.js
│       │       │
│       │       ├── Configuration
│       │       │   ├── Strada.kt                  # Public namespace, User-Agent
│       │       │   ├── StradaConfig.kt            # JSON converter, debug logging
│       │       │   ├── StradaJsonConverter.kt     # Serialization abstraction
│       │       │   └── StradaLog.kt               # Android Log utility
│       │       │
│       │       └── Helpers
│       │           ├── Helpers.kt                 # runOnUiThread utility
│       │           └── JsonExtensions.kt          # Kotlinx serialization extensions
│       │
│       └── test/kotlin/dev/hotwire/strada/
│           ├── BridgeTest.kt                      # Bridge initialization tests
│           ├── BridgeComponentTest.kt             # Component message handling
│           ├── BridgeDelegateTest.kt              # Lifecycle, routing tests
│           ├── BridgeComponentFactoryTest.kt      # Factory creation tests
│           ├── MessageTest.kt                     # Message data class tests
│           ├── InternalMessageTest.kt             # Internal message conversion
│           ├── RepositoryTest.kt                  # Asset loading tests
│           ├── UserAgentTest.kt                   # User-Agent string tests
│           ├── CoroutinesTestRule.kt              # Coroutines test helper
│           └── TestData.kt                        # Shared test data generators
│
├── build.gradle.kts                      # Root build configuration
├── settings.gradle                       # Project settings
├── gradle.properties                     # Gradle properties
├── gradlew / gradlew.bat                 # Gradle wrapper scripts
├── gradle/wrapper/                       # Gradle wrapper JAR
├── README.md                             # Project overview
└── LICENSE                               # MIT license
```

---

## Part 1: WebView Setup & Configuration

### Bridge.kt - Complete Analysis

**File Location:** `strada/src/main/kotlin/dev/hotwire/strada/Bridge.kt` (139 lines)

#### Bridge Class Structure

```kotlin
@Suppress("unused")
class Bridge internal constructor(webView: WebView) {
    private var componentsAreRegistered: Boolean = false
    private val webViewRef: WeakReference<WebView>

    internal val webView: WebView? get() = webViewRef.get()
    internal var repository = Repository()
    internal var delegate: BridgeDelegate<*>? = null

    init {
        // Use weak reference to prevent memory leaks
        webViewRef = WeakReference(webView)

        // Add JavascriptInterface before page loads
        webView.addJavascriptInterface(this, bridgeJavascriptInterface)
    }
}
```

**Critical Design Decisions:**

1. **WeakReference for WebView:** Prevents memory leaks when WebView is destroyed
2. **Singleton-per-WebView Pattern:** Static `instances` list ensures one Bridge per WebView
3. **JavascriptInterface Name:** `"StradaNative"` - must match strada.js calls

**Constants:**
```kotlin
private const val bridgeGlobal = "window.nativeBridge"
private const val bridgeJavascriptInterface = "StradaNative"
```

#### Component Registration Methods

```kotlin
internal fun register(component: String) {
    logEvent("bridgeWillRegisterComponent", component)
    val javascript = generateJavaScript("register", component.toJsonElement())
    evaluate(javascript)
}

internal fun register(components: List<String>) {
    logEvent("bridgeWillRegisterComponents", components.joinToString())
    val javascript = generateJavaScript("register", components.toJsonElement())
    evaluate(javascript)
}

internal fun unregister(component: String) {
    logEvent("bridgeWillUnregisterComponent", component)
    val javascript = generateJavaScript("unregister", component.toJsonElement())
    evaluate(javascript)
}
```

**JavaScript Generated:**
```javascript
// Single component
window.nativeBridge.register("form")

// Multiple components
window.nativeBridge.register(["form", "page", "composer"])

// Unregister
window.nativeBridge.unregister("form")
```

#### Reply Method

```kotlin
internal fun replyWith(message: Message) {
    logEvent("bridgeWillReplyWithMessage", message.toString())
    val internalMessage = InternalMessage.fromMessage(message)
    val javascript = generateJavaScript("replyWith", internalMessage.toJson().toJsonElement())
    evaluate(javascript)
}
```

#### JavascriptInterface Methods (JS → Native)

```kotlin
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

**Thread Safety:** All callbacks use `runOnUiThread` to ensure main thread execution.

#### Bridge Instance Pool

```kotlin
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

**Cleanup Logic:** `removeIf { it.webView == null }` removes dead references when WebView is garbage collected.

#### JavaScript Evaluation

```kotlin
internal fun evaluate(javascript: String) {
    logEvent("evaluatingJavascript", javascript)
    webView?.evaluateJavascript(javascript) {}
}
```

**Note:** Uses `evaluateJavascript` (async, no return value) instead of deprecated `loadUrl("javascript:...")`.

#### Load and Reset

```kotlin
internal fun load() {
    logEvent("bridgeWillLoad")
    evaluate(userScript())
}

internal fun reset() {
    logEvent("bridgeDidReset")
    componentsAreRegistered = false
}

internal fun isReady(): Boolean {
    return componentsAreRegistered
}
```

#### User Script Loading

```kotlin
internal fun userScript(): String {
    val context = requireNotNull(webView?.context)
    return repository.getUserScript(context)
}
```

---

## Part 2: strada.js - Injected JavaScript

**File Location:** `strada/src/main/assets/js/strada.js` (81 lines)

### Complete Source

```javascript
(() => {
  // NativeBridge: The adapter installed on webBridge
  // All adapters (iOS, Android) implement same interface
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

    unregister(component) {
      const index = this.supportedComponents.indexOf(component)
      if (index != -1) {
        this.supportedComponents.splice(index, 1)
        this.notifyBridgeOfSupportedComponentsUpdate()
      }
    }

    registerAdapter() {
      this.adapterIsRegistered = true

      if (this.isStradaAvailable) {
        this.webBridge.setAdapter(this)
      } else {
        document.addEventListener("web-bridge:ready", () => this.webBridge.setAdapter(this))
      }
    }

    notifyBridgeOfSupportedComponentsUpdate() {
      this.supportedComponentsUpdated()

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
        this.webBridge.receive(JSON.parse(message))
      }
    }

    // Receive from web
    receive(message) {
      this.postMessage(JSON.stringify(message))
    }

    get platform() {
      return "android"
    }

    // Native handler

    ready() {
      StradaNative.bridgeDidInitialize()
    }

    supportedComponentsUpdated() {
      StradaNative.bridgeDidUpdateSupportedComponents()
    }

    postMessage(message) {
      StradaNative.bridgeDidReceiveMessage(message)
    }

    // Web global

    get isStradaAvailable() {
      return window.Strada
    }

    get webBridge() {
      return window.Strada.web
    }
  }

  // Initialize on DOM ready
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
})()
```

### NativeBridge Class Breakdown

#### Constructor

```javascript
constructor() {
  this.supportedComponents = []
  this.adapterIsRegistered = false
}
```

#### register Method

```javascript
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
```

**Key Difference from iOS:** Android calls `registerAdapter()` only once on first registration.

#### Message Flow Methods

```javascript
// Native -> Web reply
replyWith(message) {
  if (this.isStradaAvailable) {
    this.webBridge.receive(JSON.parse(message))
  }
}

// Web -> Native send
receive(message) {
  this.postMessage(JSON.stringify(message))
}

// Post to native via JavascriptInterface
postMessage(message) {
  StradaNative.bridgeDidReceiveMessage(message)
}
```

**Platform Detection:**
```javascript
get platform() {
  return "android"
}
```

#### Native Handler Methods

```javascript
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

These call directly into the Kotlin `@JavascriptInterface` methods.

#### Initialization

```javascript
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

**Difference from iOS:** Android checks `document.readyState` and waits for `DOMContentLoaded` if needed. iOS uses `"web-bridge:ready"` event.

---

## Part 3: Message System Deep Dive

### Message.kt - Public Message Structure

**File Location:** `strada/src/main/kotlin/dev/hotwire/strada/Message.kt` (62 lines)

```kotlin
data class Message constructor(
    /**
     * Unique identifier for this message. Used for reply correlation.
     */
    val id: String,

    /**
     * Component name (e.g., "form", "page", "composer")
     */
    val component: String,

    /**
     * Event type: "connect", "submit", "display", etc.
     */
    val event: String,

    /**
     * Metadata including URL
     */
    val metadata: Metadata?,

    /**
     * JSON-encoded payload data
     */
    val jsonData: String
)
```

#### Message Creation Methods

**replacing with event and jsonData:**
```kotlin
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

**replacing with typed data:**
```kotlin
inline fun <reified T> replacing(
    event: String = this.event,
    data: T
): Message {
    return replacing(event, StradaJsonConverter.toJson(data))
}
```

**Decode typed data:**
```kotlin
inline fun <reified T> data(): T? {
    return StradaJsonConverter.toObject(jsonData)
}
```

#### Metadata Data Class

```kotlin
data class Metadata(
    val url: String
)
```

**Usage:** URL identifies which page sent the message for lifecycle filtering.

---

### InternalMessage.kt - Wire Format

**File Location:** `strada/src/main/kotlin/dev/hotwire/strada/InternalMessage.kt` (43 lines)

```kotlin
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

@Serializable
internal data class InternalDataMetadata(
    @SerialName("metadata") val metadata: InternalMetadata
)

@Serializable
internal data class InternalMetadata(
    @SerialName("url") val url: String
)
```

**Nested Metadata Structure:** The web sends metadata INSIDE the data object:
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

---

## Part 4: BridgeDelegate & Lifecycle Management

### BridgeDelegate.kt - Complete Analysis

**File Location:** `strada/src/main/kotlin/dev/hotwire/strada/BridgeDelegate.kt` (111 lines)

```kotlin
@Suppress("unused")
class BridgeDelegate<D : BridgeDestination>(
    val location: String,
    val destination: D,
    private val componentFactories: List<BridgeComponentFactory<D, BridgeComponent<D>>>
) : DefaultLifecycleObserver {
    internal var bridge: Bridge? = null
    private var destinationIsActive: Boolean = false
    private val initializedComponents = hashMapOf<String, BridgeComponent<D>>()

    private val resolvedLocation: String
        get() = bridge?.webView?.url ?: location

    val activeComponents: List<BridgeComponent<D>>
        get() = initializedComponents.map { it.value }.takeIf { destinationIsActive }.orEmpty()
}
```

**Key Design:**
- Implements `DefaultLifecycleObserver` for Android lifecycle integration
- Generic over `BridgeDestination` type
- Lazy component initialization

#### WebView Attachment/Detachment

```kotlin
fun onWebViewAttached(webView: WebView) {
    bridge = Bridge.getBridgeFor(webView)?.apply {
        delegate = this@BridgeDelegate
    }

    if (bridge != null) {
        if (shouldReloadBridge()) {
            bridge?.load()
        }
    } else {
        logWarning("bridgeNotInitializedForWebView", resolvedLocation)
    }
}

fun onWebViewDetached() {
    bridge?.delegate = null
    bridge = null
}
```

**Bridge Check:** `shouldReloadBridge()` ensures bridge is loaded when WebView is ready:
```kotlin
private fun shouldReloadBridge(): Boolean {
    return destination.bridgeWebViewIsReady() && bridge?.isReady() == false
}
```

#### Page Lifecycle Methods

```kotlin
fun onColdBootPageCompleted() {
    bridge?.load()
}

fun onColdBootPageStarted() {
    bridge?.reset()
}
```

#### Message Routing

```kotlin
internal fun bridgeDidReceiveMessage(message: Message): Boolean {
    return if (destinationIsActive && resolvedLocation == message.metadata?.url) {
        logEvent("bridgeDidReceiveMessage", message.toString())
        getOrCreateComponent(message.component)?.didReceive(message)
        true
    } else {
        logWarning("bridgeDidIgnoreMessage", message.toString())
        false
    }
}
```

**Two Filters:**
1. `destinationIsActive` - Don't process for stopped destinations
2. `resolvedLocation == message.metadata?.url` - Only deliver to matching URL

#### Component Factory

```kotlin
private fun getOrCreateComponent(name: String): BridgeComponent<D>? {
    val factory = componentFactories.firstOrNull { it.name == name } ?: return null
    return initializedComponents.getOrPut(name) { factory.create(this) }
}
```

**Lazy Instantiation:** Components created only when first message received.

#### Bridge Initialization

```kotlin
internal fun bridgeDidInitialize() {
    bridge?.register(componentFactories.map { it.name })
}
```

**Flow:**
1. JS posts "ready" via `StradaNative.bridgeDidInitialize()`
2. Calls `delegate?.bridgeDidInitialize()`
3. Gets all component names
4. Calls JS: `window.nativeBridge.register([...])`

#### Lifecycle Observer Methods

```kotlin
override fun onStart(owner: LifecycleOwner) {
    logEvent("bridgeDestinationDidStart", resolvedLocation)
    destinationIsActive = true
    activeComponents.forEach { it.didStart() }
}

override fun onStop(owner: LifecycleOwner) {
    activeComponents.forEach { it.didStop() }
    destinationIsActive = false
    logEvent("bridgeDestinationDidStop", resolvedLocation)
}

override fun onDestroy(owner: LifecycleOwner) {
    destinationIsActive = false
    logEvent("bridgeDestinationDidDestroy", resolvedLocation)
}
```

#### Component Access Helpers

```kotlin
inline fun <reified C> component(): C? {
    return activeComponents.filterIsInstance<C>().firstOrNull()
}

inline fun <reified C> forEachComponent(action: (C) -> Unit) {
    activeComponents.filterIsInstance<C>().forEach { action(it) }
}
```

---

## Part 5: BridgeComponent - Base Class

### BridgeComponent.kt - Complete Analysis

**File Location:** `strada/src/main/kotlin/dev/hotwire/strada/BridgeComponent.kt` (124 lines)

```kotlin
abstract class BridgeComponent<in D : BridgeDestination>(
    val name: String,
    private val delegate: BridgeDelegate<D>
) {
    private val receivedMessages = hashMapOf<String, Message>()

    /**
     * Returns the last received message for a given event
     */
    protected fun receivedMessageFor(event: String): Message? {
        return receivedMessages[event]
    }

    /**
     * Called when a message is received from the web bridge
     */
    abstract fun onReceive(message: Message)

    /**
     * Caches message and calls onReceive
     */
    fun didReceive(message: Message) {
        receivedMessages[message.event] = message
        onReceive(message)
    }

    fun didStart() {
        onStart()
    }

    fun didStop() {
        onStop()
    }

    open fun onStart() {}
    open fun onStop() {}
}
```

#### Reply Methods

**replyWith message:**
```kotlin
fun replyWith(message: Message): Boolean {
    return reply(message)
}
```

**replyTo event (uses cached message):**
```kotlin
fun replyTo(event: String): Boolean {
    val message = receivedMessageFor(event) ?: run {
        logWarning("bridgeMessageFailedToReply", "message for event '$event' was not received")
        return false
    }
    return reply(message)
}
```

**replyTo with new data:**
```kotlin
fun replyTo(event: String, jsonData: String): Boolean {
    val message = receivedMessageFor(event) ?: return false
    return reply(message.replacing(jsonData = jsonData))
}
```

**replyTo with typed data:**
```kotlin
inline fun <reified T> replyTo(event: String, data: T): Boolean {
    return replyTo(event, jsonData = StradaJsonConverter.toJson(data))
}
```

---

## Part 6: BridgeComponentFactory

### BridgeComponentFactory.kt

**File Location:** `strada/src/main/kotlin/dev/hotwire/strada/BridgeComponentFactory.kt` (9 lines)

```kotlin
class BridgeComponentFactory<D : BridgeDestination, out C : BridgeComponent<D>> constructor(
    val name: String,
    private val creator: (name: String, delegate: BridgeDelegate<D>) -> C
) {
    fun create(delegate: BridgeDelegate<D>) = creator(name, delegate)
}
```

**Usage:**
```kotlin
val bridgeComponentFactories = listOf(
    BridgeComponentFactory("form", ::FormComponent),
    BridgeComponentFactory("page", ::PageComponent)
)
```

---

## Part 7: Configuration & Helpers

### StradaConfig.kt

```kotlin
class StradaConfig internal constructor() {
    /**
     * Set custom JSON converter for message data serialization
     */
    var jsonConverter: StradaJsonConverter? = null

    /**
     * Enable debug logging (should be disabled in production)
     */
    var debugLoggingEnabled = false
}
```

### StradaJsonConverter.kt

```kotlin
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

abstract class StradaJsonTypeConverter : StradaJsonConverter() {
    abstract fun <T> toObject(jsonData: String, type: Class<T>): T?
    abstract fun <T> toJson(data: T, type: Class<T>): String
}

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

### StradaLog.kt

```kotlin
internal object StradaLog {
    private const val DEFAULT_TAG = "StradaLog"
    private val debugEnabled get() = Strada.config.debugLoggingEnabled

    internal fun d(msg: String) = log(Log.DEBUG, msg)
    internal fun w(msg: String) = log(Log.WARN, msg)
    internal fun e(msg: String) = log(Log.ERROR, msg)

    private fun log(logLevel: Int, msg: String) {
        when (logLevel) {
            Log.DEBUG -> if (debugEnabled) Log.d(DEFAULT_TAG, msg)
            Log.WARN -> Log.w(DEFAULT_TAG, msg)
            Log.ERROR -> Log.e(DEFAULT_TAG, msg)
        }
    }
}

internal fun logEvent(event: String, details: String = "") {
    StradaLog.d("$event ".padEnd(35, '.') + " [$details]")
}

internal fun logWarning(event: String, details: String) {
    StradaLog.w("$event ".padEnd(35, '.') + " [$details]")
}
```

### Helpers.kt - Thread Utility

```kotlin
internal fun runOnUiThread(func: () -> Unit) {
    when (val mainLooper = Looper.getMainLooper()) {
        Looper.myLooper() -> func()
        else -> Handler(mainLooper).post { func() }
    }
}
```

**Purpose:** Ensures main thread execution, handling both production and unit test scenarios.

### Repository.kt - Asset Loading

```kotlin
internal class Repository {
    fun getUserScript(context: Context): String {
        return context.assets.open("js/strada.js").use {
            String(it.readBytes())
        }
    }
}
```

### Strada.kt - Public Namespace

```kotlin
object Strada {
    val config: StradaConfig = StradaConfig()

    fun userAgentSubstring(componentFactories: List<BridgeComponentFactory<*,*>>): String {
        val components = componentFactories.joinToString(" ") { it.name }
        return "bridge-components: [$components]"
    }
}
```

**User-Agent Usage:**
```kotlin
val stradaUA = Strada.userAgentSubstring(bridgeComponentFactories)
webView.settings.userAgentString = "Turbo Native Android $stradaUA"
// Result: "Turbo Native Android bridge-components: [form page composer]"
```

### BridgeDestination.kt - Marker Interface

```kotlin
interface BridgeDestination {
    fun bridgeWebViewIsReady(): Boolean
}
```

---

## Part 8: Integration Guide

### Step-by-Step Integration

#### 1. Create Component Factory List

**BridgeComponentFactories.kt:**
```kotlin
val bridgeComponentFactories = listOf(
    // Add registered components here
)
```

#### 2. Initialize WebView

**MainSessionNavHostFragment.kt:**
```kotlin
class MainSessionNavHostFragment : TurboSessionNavHostFragment() {
    override fun onSessionCreated() {
        super.onSessionCreated()

        // Initialize user agent
        session.webView.settings.userAgentString = session.webView.customUserAgent

        // Initialize Strada bridge
        Bridge.initialize(session.webView)
    }

    private val WebView.customUserAgent: String
        get() {
            val turboSubstring = Turbo.userAgentSubstring()
            val stradaSubstring = Strada.userAgentSubstring(bridgeComponentFactories)
            return "$turboSubstring; $stradaSubstring; ${settings.userAgentString}"
        }
}
```

#### 3. Configure JSON Converter

**MainActivity.kt:**
```kotlin
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

#### 4. Implement BridgeDestination

**NavDestination.kt:**
```kotlin
interface NavDestination : TurboNavDestination, BridgeDestination {
    override fun bridgeWebViewIsReady(): Boolean {
        return session.isReady
    }
}
```

#### 5. Delegate Lifecycle

**WebFragment.kt:**
```kotlin
class WebFragment : TurboWebFragment(), NavDestination {
    private val bridgeDelegate by lazy {
        BridgeDelegate(
            location = location,
            destination = this,
            componentFactories = bridgeComponentFactories
        )
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)
        viewLifecycleOwner.lifecycle.addObserver(bridgeDelegate)
    }

    override fun onDestroyView() {
        super.onDestroyView()
        viewLifecycleOwner.lifecycle.removeObserver(bridgeDelegate)
    }

    override fun onColdBootPageStarted(location: String) {
        bridgeDelegate.onColdBootPageStarted()
    }

    override fun onColdBootPageCompleted(location: String) {
        bridgeDelegate.onColdBootPageCompleted()
    }

    override fun onWebViewAttached(webView: TurboWebView) {
        bridgeDelegate.onWebViewAttached(webView)
    }

    override fun onWebViewDetached(webView: TurboWebView) {
        bridgeDelegate.onWebViewDetached()
    }
}
```

#### 6. Create Bridge Component

**FormComponent.kt:**
```kotlin
class FormComponent(
    name: String,
    private val delegate: BridgeDelegate<NavDestination>
) : BridgeComponent<NavDestination>(name, delegate) {

    override fun onReceive(message: Message) {
        when (message.event) {
            "connect" -> handleConnectEvent(message)
            "submitEnabled" -> handleSubmitEnabled()
            "submitDisabled" -> handleSubmitDisabled()
            else -> Log.w("TurboDemo", "Unknown event: $message")
        }
    }

    private fun handleConnectEvent(message: Message) {
        val data = message.data<MessageData>() ?: return
        // Show native button with data.title
    }

    private fun performSubmit(): Boolean {
        return replyTo("connect")
    }

    @Serializable
    data class MessageData(
        @SerialName("submitTitle") val title: String
    )
}
```

**Register Component:**
```kotlin
val bridgeComponentFactories = listOf(
    BridgeComponentFactory("form", ::FormComponent)
)
```

---

## Part 9: Key Differences from Strada iOS

| Aspect | Strada Android | Strada iOS |
|--------|----------------|------------|
| **Language** | Kotlin | Swift |
| **JS Interface** | `@JavascriptInterface` annotation | `WKScriptMessageHandler` |
| **Threading** | `Handler(Looper)` for main thread | `@MainActor` annotation |
| **Lifecycle** | `DefaultLifecycleObserver` | Manual `viewDidLoad` etc. calls |
| **JSON** | kotlinx.serialization | Codable (JSONEncoder/Decoder) |
| **WebView Ref** | `WeakReference<WebView>` | `weak var webView: WKWebView?` |
| **Resource Loading** | `context.assets.open()` | `Bundle.module.path()` |
| **JS Injection** | `evaluateJavascript()` | `evaluateJavaScript()` |
| **Initialization** | DOMContentLoaded check | `"web-bridge:ready"` event |
| **Adapter Register** | Called once on first component | Called on every register |

---

## Part 10: Rust Recreation Architecture

### Crate Structure

```
strada-rs-android/
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
│   │   └── android.rs      # Android WebView bindings
│   └── utils/
│       ├── json.rs        # Serde JSON helpers
│       └── logger.rs      # tracing integration
├── assets/
│   └── strada.js          # Embedded JavaScript
└── examples/
    └── android-integration/
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
pub trait BridgeComponent: Send + Sync {
    fn name(&self) -> &'static str;

    fn on_receive(&self, message: Message);
    fn on_start(&self) {}
    fn on_stop(&self) {}
}

// BridgeDelegate
pub struct BridgeDelegate<D: BridgeDestination> {
    location: String,
    destination: Arc<D>,
    components: RwLock<HashMap<String, Arc<dyn BridgeComponent>>>,
    bridge: RwLock<Option<Arc<Bridge>>>,
    is_active: AtomicBool,
}

// Bridge
pub struct Bridge {
    webview: Arc<dyn WebView>,
    delegate: RwLock<Option<Arc<dyn BridgeDelegate<dyn BridgeDestination>>>>,
}
```

### Required Crates

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# WebView (Android)
# Note: Android WebView access requires JNI or custom Rust Android framework
jni = "0.21"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Error handling
thiserror = "1"
anyhow = "1"

# Threading
parking_lot = "0.12"
```

### JavaScript Injection

```rust
const STRADA_JS: &str = include_str!("../assets/strada.js");

pub async fn initialize_bridge(webview: &Arc<dyn WebView>) -> Result<()> {
    // Inject strada.js
    webview.evaluate_javascript(STRADA_JS)?;

    // Set up message handler
    webview.add_javascript_interface("StradaNative", move |message| {
        handle_javascript_message(message)
    });

    Ok(())
}
```

### Message Handler

```rust
fn handle_javascript_message(body: &str) {
    // Parse as InternalMessage
    if let Ok(message) = serde_json::from_str::<InternalMessage>(body) {
        let delegate = get_delegate();
        tokio::spawn(async move {
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
    |                 |--addJsInterface-> |                |                 |
    |                 |   (StradaNative)  |                |                 |
    |                 |                   |                |                 |
    |                 |                   |                |  <DOM ready>
    |                 |                   |                |--execute-------->
    |                 |                   |                |                 |
    |                 |                   |                |--bridgeDidInit->
    |                 |<--@JsInterface----|                |                 |
    |                 |                   |                |                 |
    |                 |--bridgeDidInit--->|                |                 |
    |                 |                   |                |                 |
    |                 |--register(["form","page"])------->|                 |
    |                 |                   |                |                 |
    |                 |                   |                |--setAdapter---->
    |                 |                   |                |                 |
```

### Message Flow (Web -> Native -> Web)

```
Web Component     strada.js       @JavascriptInterface  Bridge     BridgeDelegate  FormComponent
    |                |                     |              |            |                |
    |--send()------->|                     |              |            |                |
    |  {event:"connect",                   |              |            |                |
    |   data:{...}}  |                     |              |            |                |
    |                |--postMessage-------->|              |            |                |
    |                |  (JSON.stringify)    |              |            |                |
    |                |                     |--didReceive-->|            |                |
    |                |                     |              |            |                |
    |                |                     |              |--bridgeDidReceive-->       |
    |                |                     |              |            |                |
    |                |                     |              |            |--getOrCreate-->|
    |                |                     |              |            |   (create)     |
    |                |                     |              |            |                |
    |                |                     |              |            |--didReceive--->|
    |                |                     |              |            |                |--onReceive()
    |                |                     |              |            |                |
    |                |                     |              |            |                |--(native action)
    |                |                     |              |            |                |
    |                |                     |              |            |<--reply()------|
    |                |                     |              |            |                |
    |                |                     |              |<--reply()----|              |
    |                |                     |              |            |                |
    |                |<--evaluateJS------------------------|            |                |
    |  receive() <---|  (replyWith)       |              |            |                |
    |<---------------|                     |              |            |                |
```

---

## Part 12: Testing Strategy

### Test Structure

Tests use Robolectric for Android framework simulation:

```kotlin
@RunWith(AndroidJUnit4::class)
class BridgeTest {
    private lateinit var webView: WebView
    private lateinit var bridge: Bridge

    @Before
    fun setup() {
        webView = Robolectric.buildActivity(TestActivity::class.java).get().webView
        bridge = Bridge(webView)
    }

    @Test
    fun `bridge initializes with webView`() {
        assertNotNull(bridge.webView)
    }

    @Test
    fun `bridge registers components`() {
        bridge.register("form")
        // Verify JavaScript evaluation
    }
}
```

### Test Doubles

Using Mockito for mocking:
```kotlin
@Mock
private lateinit var mockDelegate: BridgeDelegate<*>

@Test
fun `delegate receives message`() {
    bridge.delegate = mockDelegate
    bridge.bridgeDidReceiveMessage("""{"id":"1","component":"form","event":"connect","data":{}}""")
    verify(mockDelegate).bridgeDidReceiveMessage(any())
}
```

---

## Appendix: Complete File Reference

| File | Lines | Key Responsibilities |
|------|-------|---------------------|
| Bridge.kt | 139 | WebView integration, JavascriptInterface, instance pool |
| BridgeDelegate.kt | 111 | Lifecycle, message routing, component factory |
| BridgeComponent.kt | 124 | Base component, reply methods, message caching |
| Message.kt | 62 | Public message data class, Codable helpers |
| InternalMessage.kt | 43 | Wire format, JSON serialization |
| BridgeComponentFactory.kt | 9 | Factory for component creation |
| Repository.kt | 11 | Asset loading for strada.js |
| Strada.kt | 11 | Public namespace, User-Agent |
| StradaConfig.kt | 17 | JSON converter, debug logging |
| StradaJsonConverter.kt | 69 | Serialization abstraction |
| StradaLog.kt | 37 | Android Log utility |
| Helpers.kt | 16 | runOnUiThread utility |
| strada.js | 81 | Injected JavaScript bridge |
| JsonExtensions.kt | ~30 | Kotlinx serialization extensions |

---

## Open Questions for Rust Implementation

1. **WebView Access on Android:** How to access Android WebView from Rust? JNI required?
2. **Threading Model:** Android main looper vs Rust async - how to bridge?
3. **Lifecycle Integration:** AndroidX lifecycle not available from Rust - alternative?
4. **Serialization:** Serde vs kotlinx.serialization - compatibility concerns?
5. **FFI Layer:** jni-rs for Java interop?
6. **Resource Bundling:** Best practice for embedding strada.js in Rust crate for Android?

---

## Rust Implementation Checklist

- [ ] Create Message struct with serde
- [ ] Implement InternalMessage with JSON serialization
- [ ] Build Bridge struct with WebView trait for Android
- [ ] Create BridgeDelegate with lifecycle (custom implementation)
- [ ] Define BridgeComponent trait
- [ ] Embed strada.js as const
- [ ] Implement JavascriptInterface message handler
- [ ] Create component registry
- [ ] Add JSON helper extensions
- [ ] Set up tracing logging
- [ ] Write unit tests (mock WebView)
- [ ] Create example Android integration
- [ ] Document API
- [ ] Publish to crates.io
