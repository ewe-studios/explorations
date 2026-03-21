# Strada Android - BridgeComponent System Deep Dive

## Overview

This document explores the BridgeComponent architecture, lifecycle management, and how native components are created, managed, and destroyed in response to web component messages in Strada Android.

## Component Architecture

### Class Hierarchy

```
BridgeComponent<D : BridgeDestination> (abstract class)
       ▲
       │
┌──────┴──────┬─────────────┬──────────────┐
│             │             │              │
FormComponent  PageComponent  ComposerComponent  (your components)
```

### BridgeComponent Base Class

```kotlin
// BridgeComponent.kt:3-6
abstract class BridgeComponent<in D : BridgeDestination>(
    val name: String,
    private val delegate: BridgeDelegate<D>
) {
    private val receivedMessages = hashMapOf<String, Message>()
    // ...
}
```

**Key design decisions:**

1. **Abstract class**: Must be subclassed, cannot be used directly
2. **Contravariant type parameter**: `in D` - component consumes destination
3. **Final name property**: Component name is fixed at construction
4. **Private delegate**: Only accessible to the component implementation

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
│  didStart()   │ │  didStop()    │ │   Deactivated │
│  onStart()    │ │  onStop()     │ │  (destination │
└───────────────┘ └───────────────┘ │   inactive)   │
                          └───────────────┘
```

### Abstract Method: onReceive

```kotlin
// BridgeComponent.kt:20
abstract fun onReceive(message: Message)
```

**Must be implemented by every component.** Called when a message is received.

### Lifecycle Hooks

```kotlin
// BridgeComponent.kt:59-66
open fun onStart() {}
open fun onStop() {}
```

**Optional overrides** for destination lifecycle events.

### Internal Wrapper Methods

```kotlin
// BridgeComponent.kt:29-52
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
```

These are `fun` (not `open`) and called by the framework.

## Message Handling

### Message Caching

```kotlin
// BridgeComponent.kt:7
private val receivedMessages = hashMapOf<String, Message>()
```

Messages are cached by event name for later reply.

### receivedMessageFor() Helper

```kotlin
// BridgeComponent.kt:12-14
protected fun receivedMessageFor(event: String): Message? {
    return receivedMessages[event]
}
```

**Protected** - only accessible to subclasses.

### didReceive() Implementation

```kotlin
// BridgeComponent.kt:29-32
fun didReceive(message: Message) {
    receivedMessages[message.event] = message
    onReceive(message)
}
```

**Two actions:**
1. Caches the message by event
2. Calls the abstract `onReceive()` method

## Reply Patterns

### replyWith() - Full Message

```kotlin
// BridgeComponent.kt:72-74
fun replyWith(message: Message): Boolean {
    return reply(message)
}
```

### replyTo() - By Event (Original Data)

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

### replyTo() - By Event (New JSON Data)

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

### replyTo() - By Event (Typed Data)

```kotlin
// BridgeComponent.kt:116-118
inline fun <reified T> replyTo(event: String, data: T): Boolean {
    return replyTo(event, jsonData = StradaJsonConverter.toJson(data))
}
```

Uses reified generics for type-safe serialization.

### Private reply() Helper

```kotlin
// BridgeComponent.kt:120-122
private fun reply(message: Message): Boolean {
    return delegate.replyWith(message)
}
```

Delegates all replies to the BridgeDelegate.

## BridgeDelegate Architecture

### Class Definition

```kotlin
// BridgeDelegate.kt:8-18
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
}
```

### DefaultLifecycleObserver

By implementing `DefaultLifecycleObserver`, the delegate automatically receives lifecycle callbacks:

```kotlin
// BridgeDelegate.kt:79-94
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

### Component Creation

```kotlin
// BridgeDelegate.kt:106-109
private fun getOrCreateComponent(name: String): BridgeComponent<D>? {
    val factory = componentFactories.firstOrNull { it.name == name } ?: return null
    return initializedComponents.getOrPut(name) { factory.create(this) }
}
```

**Lazy initialization:** Components are created only when their first message arrives.

### Message Routing

```kotlin
// BridgeDelegate.kt:62-71
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

**Routing conditions:**
1. Destination must be active
2. Message URL must match current location
3. Component must have a registered factory

### Active Components

```kotlin
// BridgeDelegate.kt:19-20
val activeComponents: List<BridgeComponent<D>>
    get() = initializedComponents.map { it.value }.takeIf { destinationIsActive }.orEmpty()
```

Returns empty list when destination is inactive.

### Bridge Attachment

```kotlin
// BridgeDelegate.kt:30-42
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
```

### Bridge Detachment

```kotlin
// BridgeDelegate.kt:44-47
fun onWebViewDetached() {
    bridge?.delegate = null
    bridge = null
}
```

Prevents memory leaks by clearing references.

### Reply Implementation

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

### Component Registration

```kotlin
// BridgeDelegate.kt:58-60
internal fun bridgeDidInitialize() {
    bridge?.register(componentFactories.map { it.name })
}
```

Called when the JavaScript bridge initializes.

## BridgeComponentFactory

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

**Variance:**
- `in D` (contravariant) - destination is consumed
- `out C` (covariant) - component is produced

### Factory Registration

```kotlin
// BUILD-COMPONENTS.md:117-123
val bridgeComponentFactories = listOf(
    BridgeComponentFactory("form", ::FormComponent),
    BridgeComponentFactory("page", ::PageComponent)
)
```

Uses constructor references for clean factory creation.

### Factory Creation

```kotlin
// BridgeComponentFactory.kt:7
fun create(delegate: BridgeDelegate<D>) = creator(name, delegate)
```

Calls the stored lambda with name and delegate.

## BridgeDestination

### Interface Definition

```kotlin
// BridgeDestination.kt:3-5
interface BridgeDestination {
    fun bridgeWebViewIsReady(): Boolean
}
```

**Single method:** Used to check if the WebView is ready before reloading the bridge.

### Implementation Example

```kotlin
// QUICK-START.md:82-89
interface NavDestination : TurboNavDestination, BridgeDestination {
    override fun bridgeWebViewIsReady(): Boolean {
        return session.isReady
    }
}
```

## Cold Boot Handling

### Page Started

```kotlin
// BridgeDelegate.kt:26-28
fun onColdBootPageStarted() {
    bridge?.reset()
}
```

Resets the bridge registration state.

### Page Completed

```kotlin
// BridgeDelegate.kt:22-24
fun onColdBootPageCompleted() {
    bridge?.load()
}
```

Reloads the JavaScript bridge.

### Should Reload Check

```kotlin
// BridgeDelegate.kt:73-75
private fun shouldReloadBridge(): Boolean {
    return destination.bridgeWebViewIsReady() && bridge?.isReady() == false
}
```

Checks if bridge needs to be reloaded after WebView attachment.

## Component Retrieval

### Generic Component Access

```kotlin
// BridgeDelegate.kt:98-100
inline fun <reified C> component(): C? {
    return activeComponents.filterIsInstance<C>().firstOrNull()
}
```

**Usage:**
```kotlin
val formComponent: FormComponent? = bridgeDelegate.component()
```

### forEachComponent

```kotlin
// BridgeDelegate.kt:102-104
inline fun <reified C> forEachComponent(action: (C) -> Unit) {
    activeComponents.filterIsInstance<C>().forEach { action(it) }
}
```

**Usage:**
```kotlin
bridgeDelegate.forEachComponent<FormComponent> { it.refresh() }
```

## Example Component Implementation

### FormComponent

```kotlin
// BUILD-COMPONENTS.md:13-65
class FormComponent(
    name: String,
    private val delegate: BridgeDelegate<NavDestination>
) : BridgeComponent<NavDestination>(name, delegate) {

    override fun onReceive(message: Message) {
        when (message.event) {
            "connect" -> handleConnectEvent(message)
            "submitEnabled" -> handleSubmitEnabled()
            "submitDisabled" -> handleSubmitDisabled()
            else -> Log.w("TurboDemo", "Unknown event for message: $message")
        }
    }

    private fun handleConnectEvent(message: Message) {
        val data = message.data<MessageData>() ?: return
        // Display native submit button with data.title
    }

    private fun handleSubmitEnabled() {
        // Enable submit button
    }

    private fun handleSubmitDisabled() {
        // Disable submit button
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

## Lifecycle Integration

### Fragment Integration

```kotlin
// QUICK-START.md:97-131
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

### Lifecycle Flow

```
Fragment.onViewCreated()
    └─> lifecycle.addObserver(bridgeDelegate)
        └─> Delegate receives onStart()
            └─> destinationIsActive = true
            └─> activeComponents.forEach { it.didStart() }

Fragment.onStop()
    └─> Delegate receives onStop()
        └─> activeComponents.forEach { it.didStop() }
        └─> destinationIsActive = false

Fragment.onDestroyView()
    └─> lifecycle.removeObserver(bridgeDelegate)
    └─> bridgeDelegate.onWebViewDetached()
        └─> bridge?.delegate = null
        └─> bridge = null
```

## Memory Management

### Weak Reference in Bridge

```kotlin
// Bridge.kt:16-18
private val webViewRef: WeakReference<WebView>
internal val webView: WebView? get() = webViewRef.get()
```

### Nulling References

```kotlin
// BridgeDelegate.kt:44-47
fun onWebViewDetached() {
    bridge?.delegate = null
    bridge = null
}
```

### Instance Cleanup

```kotlin
// Bridge.kt:130-131
instances.add(bridge)
instances.removeIf { it.webView == null }
```

Removes dead references during initialization.

## Testing

### Test Setup

```kotlin
// BridgeComponentTest.kt
@Test
fun `component receives message and handles event`() = testDispatcher.runTest {
    val delegate = BridgeDelegateSpy()
    val component = FormComponent("form", delegate)

    val message = Message("1", "form", "connect", null, """{"submitTitle":"Submit"}""")
    component.didReceive(message)

    assertTrue(component.connectEventHandled)
    assertEquals("Submit", component.lastSubmitTitle)
}
```

### Message Cache Test

```kotlin
@Test
fun `receivedMessageFor returns cached message`() {
    val component = FormComponent("form", delegate)
    val message = Message("1", "form", "connect", null, "{}")

    component.didReceive(message)

    assertEquals(message, component.receivedMessageFor("connect"))
}
```

### Reply Test

```kotlin
@Test
fun `replyTo with event replies with original message`() {
    val delegate = BridgeDelegateSpy()
    val component = FormComponent("form", delegate)
    val message = Message("1", "form", "connect", null, "{}")

    component.didReceive(message)
    component.replyTo("connect")

    assertTrue(delegate.replyCalled)
    assertEquals(message, delegate.repliedMessage)
}
```

## Comparison with iOS

### Component Base

| Aspect | iOS | Android |
|--------|-----|---------|
| **Type** | `open class` | `abstract class` |
| **Name** | `class var name` | `val name` in constructor |
| **Delegate** | `unowned let` | `private val` |
| **Lifecycle** | View controller methods | DefaultLifecycleObserver |

### Lifecycle Methods

| iOS | Android |
|-----|---------|
| `onViewDidLoad()` | `onStart()` |
| `onViewWillDisappear()` | `onStop()` |
| Manual delegation | Automatic via observer |

### Message Caching

| Aspect | iOS | Android |
|--------|-----|---------|
| **Type** | `[String: Message]` | `hashMapOf<String, Message>` |
| **Access** | `receivedMessage(for:)` | `receivedMessageFor(_:)` |

### Component Factory

| Aspect | iOS | Android |
|--------|-----|---------|
| **Pattern** | Type-based (`BridgeComponent.Type`) | Factory pattern |
| **Registration** | `BridgeComponent.allTypes` | `BridgeComponentFactory` list |
| **Creation** | `componentType.init(...)` | `factory.create(delegate)` |

## Performance Considerations

### Inlined Generics

```kotlin
inline fun <reified C> component(): C?
inline fun <reified T> replyTo(event: String, data: T): Boolean
```

Avoids runtime type erasure overhead.

### FilterIsInstance

```kotlin
activeComponents.filterIsInstance<C>().firstOrNull()
```

Efficient type filtering at the cost of iteration.

### Lazy Initialization

Components are created on-demand, reducing memory footprint for unused components.

---

*This deep dive covers the BridgeComponent architecture, lifecycle management via DefaultLifecycleObserver, and component communication patterns for Android. The next document will explore the Rust reimplementation considerations.*
