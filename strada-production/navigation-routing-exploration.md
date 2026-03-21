# Strada Production - Navigation & Routing Exploration

## Overview

This document explores navigation and routing patterns for production iOS/Android apps that combine WebViews with native elements using Strada.

## Architecture

### Navigation Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                    Native Navigation Layer                      │
│  iOS: UINavigationController, UITabBarController                │
│  Android: NavController, BottomNavigationView                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ coordinates with
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Strada Bridge Layer                          │
│  - Page component (navigation bar state)                        │
│  - Navigation events (via messages)                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ drives
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    WebView Navigation Layer                     │
│  - Turbo/Hotwire navigation                                     │
│  - Browser history (pushState, popState)                        │
│  - Deep link routing                                              │
└─────────────────────────────────────────────────────────────────┘
```

## Deep Linking

### iOS Universal Links

#### Configuration

**apple-app-site-association (AASA) file:**
```json
{
  "applinks": {
    "apps": [],
    "details": [
      {
        "appID": "TEAMID.com.example.app",
        "paths": [
          "/posts/*",
          "/users/*",
          "/settings*"
        ]
      }
    ]
  }
}
```

**Hosting:** Must be served from `https://example.com/.well-known/apple-app-site-association` with `Content-Type: application/json`

#### Handling in App

```swift
// SceneDelegate.swift
func scene(_ scene: UIScene,
           continue userActivity: NSUserActivity) {
    guard let url = userActivity.webpageURL else { return }

    // Let Turbo handle if it's a web URL
    if url.host == "example.com" {
        // Option 1: Navigate existing WebView
        session.navigate(to: url)

        // Option 2: Create new session/tab
        session = createSession(for: url)
    }
}
```

#### Strada Integration

```swift
// When deep link opens, synchronize with web
func handleDeepLink(_ url: URL) {
    // 1. Navigate WebView
    webView.load(URLRequest(url: url))

    // 2. Wait for page to load
    // 3. Send navigation context via Strada
    pageComponent.sendNavigationContext(path: url.path)
}
```

### Android App Links

#### Configuration

**AndroidManifest.xml:**
```xml
<intent-filter android:autoVerify="true">
    <action android:name="android.intent.action.VIEW" />
    <category android:name="android.intent.category.DEFAULT" />
    <category android:name="android.intent.category.BROWSABLE" />

    <data android:scheme="https"
          android:host="example.com"
          android:pathPrefix="/posts" />
</intent-filter>
```

**Digital Asset Links (server):**
`https://example.com/.well-known/assetlinks.json`
```json
[{
  "relation": ["delegate_permission/common.handle_all_urls"],
  "target": {
    "namespace": "android_app",
    "package_name": "com.example.app",
    "sha256_cert_fingerprints": ["AA:BB:CC:..."]
  }
}]
```

#### Handling in App

```kotlin
// MainActivity.kt
override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    handleIntent(intent)
}

override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    handleIntent(intent)
}

private fun handleIntent(intent: Intent) {
    if (intent.action == Intent.ACTION_VIEW) {
        val url = intent.data
        if (url?.host == "example.com") {
            // Navigate WebView to URL
            webView.loadUrl(url.toString())

            // Synchronize with Strada
            pageComponent.sendNavigationContext(url.path)
        }
    }
}
```

## Back Navigation

### Android Back Button

#### Strategy 1: WebView History First

```kotlin
class TurboWebViewController : AppCompatActivity() {

    override fun onBackPressed() {
        if (webView.canGoBack()) {
            // Check if we should handle via Strada
            val backUrl = webView.backHistory.firstOrNull()?.url

            // Option: Send to web for Strada handling
            pageComponent.sendBackNavigationRequest(backUrl) { handled ->
                if (!handled) {
                    webView.goBack()
                }
            }
        } else {
            super.onBackPressed()
        }
    }
}
```

#### Strada Back Message Protocol

```typescript
// Web: Send back navigation context
interface BackNavigationMessage {
    event: "back-requested"
    data: {
        currentUrl: string
        backUrl: string
        historyLength: number
    }
}

// Native: Reply with handling status
interface BackNavigationReply {
    event: "back-handled"
    data: {
        handled: boolean
        action: "native-back" | "web-nav" | "custom"
    }
}
```

#### Strategy 2: Native Stack Coordination

```kotlin
// When navigating to new "page", push native fragment
fun navigateToNewPage(url: String) {
    // 1. WebView navigates
    webView.loadUrl(url)

    // 2. Native UI updates (toolbar, etc)
    updateToolbarForPage(url)

    // 3. Add to native back stack
    supportFragmentManager.beginTransaction()
        .replace(R.id.container, WebViewFragment(url))
        .addToBackStack(null)
        .commit()
}

override fun onBackPressed() {
    if (supportFragmentManager.backStackEntryCount > 0) {
        supportFragmentManager.popBackStack()
    } else {
        super.onBackPressed()
    }
}
```

### iOS Back Gesture

#### Navigation Controller Integration

```swift
class VisitableViewController: UIViewController {

    var visitableURL: URL!
    var bridgeDelegate: BridgeDelegate!

    // Called when web wants to navigate
    func visitableProposal(_ proposal: VisitProposal) {
        switch proposal.action {
        case .advance:
            navigationController?.pushViewController(
                VisitableViewController(url: proposal.url),
                animated: true
            )
        case .replace:
            let newVC = VisitableViewController(url: proposal.url)
            navigationController?.setViewControllers([newVC], animated: false)
        case .restore:
            // Handle restore
            break
        }
    }
}
```

#### Custom Back via Strada

```swift
// Page component handles custom back behavior
class PageComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "back-enabled":
            let data: BackData? = message.data()
            // Show/hide custom back button in toolbar
            updateBackButton(data?.enabled ?? false)

        case "back-tapped":
            // Web wants native back action
            delegate.destination.goBack()
        }
    }
}
```

## Navigation State Synchronization

### URL to Navigation State

```typescript
// Web: Send current navigation state
interface NavigationStateMessage {
    event: "navigation-state"
    data: {
        path: string
        title: string
        canGoBack: boolean
        canGoForward: boolean
        nativeNavigationType?: "push" | "pop" | "replace" | "root"
    }
}
```

### Native Toolbar Updates

**iOS:**
```swift
class PageComponent: BridgeComponent {

    private var currentNavigationState: NavigationState?

    override func onReceive(message: Message) {
        switch message.event {
        case "navigation-state":
            let data: NavigationState? = message.data()
            currentNavigationState = data

            // Update navigation bar
            if let title = data?.title {
                destination.navigationItem.title = title
            }

            // Update back button
            navigationItem.hidesBackButton = !(data?.canGoBack ?? false)

        case "show-toolbar":
            destination.navigationController?.setNavigationBarHidden(false, animated: true)

        case "hide-toolbar":
            destination.navigationController?.setNavigationBarHidden(true, animated: true)
        }
    }
}
```

**Android:**
```kotlin
class PageComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override fun onReceive(message: Message) {
        when (message.event) {
            "navigation-state" -> {
                val data: NavigationState? = message.data()

                // Update ActionBar/Toolbar
                val activity = destination.requireActivity() as? AppCompatActivity
                activity?.supportActionBar?.title = data?.title

                // Update back button
                activity?.supportActionBar?.setDisplayHomeAsUpEnabled(data?.canGoBack == true)
            }
            "show-toolbar" -> {
                // Show toolbar
            }
            "hide-toolbar" -> {
                // Hide toolbar
            }
        }
    }
}
```

## Tab Bar Navigation

### iOS Tab Bar

```swift
class MainTabBarController: UITabBarController {

    override func viewDidLoad() {
        super.viewDidLoad()

        viewControllers = [
            createNavController(for: .home),
            createNavController(for: .search),
            createNavController(for: .profile)
        ]
    }

    private func createNavController(for tab: AppTab) -> UINavigationController {
        let url = tab.rootURL
        let visitableVC = VisitableViewController(url: url)
        visitableVC.tabBarItem = tab.tabBarItem

        // Set initial path for Strada
        visitableVC.initialPath = tab.rootPath

        return UINavigationController(rootViewController: visitableVC)
    }
}
```

### Android Bottom Navigation

```kotlin
class MainActivity : AppCompatActivity() {

    private lateinit var bottomNavigationView: BottomNavigationView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        bottomNavigationView = findViewById(R.id.bottom_navigation)
        bottomNavigationView.setOnItemSelectedListener { item ->
            when (item.itemId) {
                R.id.navigation_home -> {
                    navigateToTab("/home")
                    true
                }
                R.id.navigation_search -> {
                    navigateToTab("/search")
                    true
                }
                R.id.navigation_profile -> {
                    navigateToTab("/profile")
                    true
                }
                else -> false
            }
        }
    }

    private fun navigateToTab(path: String) {
        val url = "https://example.com$path"
        webView.loadUrl(url)

        // Update Strada context
        pageComponent.setTabContext(path)
    }
}
```

## Modal Presentations

### Native Modal from Web

```typescript
// Web: Request modal presentation
interface ModalRequest {
    event: "present-modal"
    data: {
        url: string
        presentationStyle: "sheet" | "full" | "card"
        size?: "small" | "medium" | "large"
    }
}
```

**iOS Implementation:**
```swift
class PageComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "present-modal":
            let data: ModalData? = message.data()
            presentNativeModal(data: data!)

        case "dismiss-modal":
            destination.dismiss(animated: true)
        }
    }

    private func presentNativeModal(data: ModalData) {
        let modalVC = VisitableViewController(url: URL(string: data.url)!)
        modalVC.modalPresentationStyle = .pageSheet

        if let sheet = modalVC.sheetPresentationController {
            switch data.size {
            case "small":
                sheet.detents = [.medium()]
            case "large":
                sheet.detents = [.large()]
            default:
                sheet.detents = [.medium(), .large()]
            }
        }

        destination.present(modalVC, animated: true)
    }
}
```

## Keyboard Handling

### iOS Keyboard Avoidance

```swift
class VisitableViewController: UIViewController {

    private var keyboardObserver: NSObjectProtocol?

    override func viewDidLoad() {
        super.viewDidLoad()

        keyboardObserver = NotificationCenter.default.addObserver(
            forName: UIResponder.keyboardWillShowNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            self?.adjustWebViewForKeyboard(notification)
        }

        NotificationCenter.default.addObserver(
            forName: UIResponder.keyboardWillHideNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.resetWebViewInset()
        }
    }

    private func adjustWebViewForKeyboard(_ notification: Notification) {
        guard let keyboardFrame = notification.userInfo?[
            UIResponder.keyboardFrameEndUserInfoKey] as? CGRect else { return }

        let keyboardHeight = keyboardFrame.height
        let safeAreaBottom = view.safeAreaInsets.bottom
        let inset = keyboardHeight - safeAreaBottom

        webView.scrollView.contentInset.bottom = inset
        webView.scrollView.scrollIndicatorInsets.bottom = inset
    }
}
```

### Android Keyboard Handling

```kotlin
// AndroidManifest.xml
<activity
    android:name=".MainActivity"
    android:windowSoftInputMode="adjustResize"
    android:fitsSystemWindows="true"
/>

// Or handle programmatically for more control
class MainActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        ViewCompat.setOnApplyWindowInsetsListener(webView) { view, insets ->
            val imeHeight = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
            val systemBarsHeight = insets.getInsets(WindowInsetsCompat.Type.systemBars()).bottom

            // Adjust WebView padding for keyboard
            view.setPadding(0, 0, 0, imeHeight)

            WindowInsetsCompat.CONSUMED
        }
    }
}
```

## Summary

Navigation in WebView + native apps requires:

1. **Deep link handling** - Universal Links (iOS) / App Links (Android)
2. **Back navigation coordination** - WebView history vs native back stack
3. **State synchronization** - Keep native UI in sync with web navigation
4. **Tab bar integration** - Map native tabs to web routes
5. **Modal presentations** - Native sheets/modals triggered from web
6. **Keyboard handling** - WebView content inset adjustments

---

*This exploration covers navigation patterns. Related: see `keyboard-handling-exploration.md` and `native-ui-components-exploration.md`.*
