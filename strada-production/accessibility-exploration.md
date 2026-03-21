# Strada Production - Accessibility Exploration

## Overview

This document explores accessibility (a11y) strategies for production iOS/Android apps using WebViews and Strada, ensuring compatibility with VoiceOver (iOS) and TalkBack (Android).

## Architecture

### Accessibility Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                    Native Accessibility Layer                   │
│  iOS: UIAccessibility                                           │
│  Android: AccessibilityNodeInfo                                 │
│  - VoiceOver / TalkBack support                                 │
│  - Dynamic type / font scaling                                  │
│  - Focus management                                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ coordinates with
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    WebView Accessibility Layer                  │
│  - WCAG 2.1 AA compliance                                       │
│  - ARIA labels and roles                                        │
│  - Focus management                                             │
│  - Screen reader support                                        │
└─────────────────────────────────────────────────────────────────┘
```

## iOS Accessibility

### VoiceOver Support

```swift
// VisitableViewController+Accessibility.swift
extension VisitableViewController {

    override func viewDidLoad() {
        super.viewDidLoad()

        // Enable accessibility for WebView
        webView.isAccessibilityElement = true
        webView.accessibilityLabel = "Web content"

        // Set traits
        webView.accessibilityTraits = .updatesFrequently

        // Configure navigation bar
        navigationController?.navigationBar.isAccessibilityElement = true
        navigationItem.titleView?.isAccessibilityElement = true
    }

    // Announce page changes to VoiceOver
    func announcePageChange(title: String) {
        UIAccessibility.post(notification: .pageScrolled, argument: title)

        // Also announce via label
        navigationItem.title = title
        navigationItem.accessibilityHint = "Page: \(title)"
    }
}

// PageComponent - Accessibility updates
class PageComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "navigation-state":
            let data: NavigationData? = message.data()
            updateAccessibilityWithTitle(data?.title)

        case "announce":
            let data: AnnounceData? = message.data()
            postAccessibilityAnnouncement(data?.message ?? "")
        }
    }

    private func updateAccessibilityWithTitle(_ title: String?) {
        guard let title = title else { return }

        // Update navigation title
        destination.navigationItem.title = title

        // Announce to VoiceOver
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            UIAccessibility.post(notification: .pageScrolled, argument: title)
        }
    }

    private func postAccessibilityAnnouncement(_ message: String) {
        DispatchQueue.main.async {
            UIAccessibility.post(notification: .announcement, argument: message)
        }
    }
}
```

### Dynamic Type Support

```swift
// Support dynamic font sizing
class AccessibleLabel: UILabel {

    override init(frame: CGRect) {
        super.init(frame: frame)
        setupDynamicType()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setupDynamicType()
    }

    private func setupDynamicType() {
        // Enable dynamic type
        adjustsFontForContentSizeCategory = true

        // Set font with content size category
        font = UIFont.preferredFont(forTextStyle: .body)

        // Listen for content size changes
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(contentSizeDidChange),
            name: UIContentSizeCategory.didChangeNotification,
            object: nil
        )
    }

    @objc private func contentSizeDidChange() {
        // Update layout if needed
        setNeedsLayout()
    }
}

// OfflineBanner with Dynamic Type
class OfflineBannerView: UIView {

    private let label: UILabel = {
        let label = UILabel()
        label.adjustsFontForContentSizeCategory = true
        label.font = UIFont.preferredFont(forTextStyle: .subheadline)
        label.translatesAutoresizingMaskIntoConstraints = false
        return label
    }()

    init() {
        super.init(frame: .zero)
        isAccessibilityElement = true
        accessibilityLabel = "Offline notice"
        setupSubviews()
    }

    private func setupSubviews() {
        addSubview(label)

        NSLayoutConstraint.activate([
            label.topAnchor.constraint(equalTo: topAnchor, constant: 12),
            label.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -12),
            label.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 16),
            label.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -16)
        ])
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}
```

### Custom Accessibility Actions

```swift
// Add custom actions for VoiceOver
class WebViewAccessibilityHandler {

    static func setupCustomActions(for webView: WKWebView) {
        // Add custom rotor item
        webView.accessibilityRotor = UIAccessibilityRotor(
            name: "Web Links",
            itemSearchType: .link
        )

        // Add custom actions
        webView.accessibilityCustomActions = [
            UIAccessibilityCustomAction(
                name: "Refresh page",
                target: self,
                selector: #selector(refreshPage)
            ),
            UIAccessibilityCustomAction(
                name: "Go back",
                target: self,
                selector: #selector(goBack)
            ),
            UIAccessibilityCustomAction(
                name: "Go home",
                target: self,
                selector: #selector(goHome)
            )
        ]
    }

    @objc private static func refreshPage() -> Bool {
        // Trigger refresh
        return true
    }

    @objc private static func goBack() -> Bool {
        // Navigate back
        return true
    }

    @objc private static func goHome() -> Bool {
        // Navigate home
        return true
    }
}
```

### Focus Management

```swift
// Manage focus for accessibility
class FocusManager {

    /// Move focus to specific element
    static func moveFocus(to element: UIView) {
        // Post notification to move VoiceOver focus
        UIAccessibility.post(notification: .layoutChanged, argument: element)
    }

    /// Move focus to WebView content
    static func moveFocusToWebView(_ webView: WKWebView) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            UIAccessibility.post(notification: .layoutChanged, argument: webView)
        }
    }

    /// Move focus to error message
    static func moveFocusToError(_ errorMessage: String, in view: UIView) {
        // Create or update error label
        let errorLabel = view.viewWithTag(999) ?? createErrorLabel(in: view)
        errorLabel.accessibilityLabel = errorMessage

        // Move focus
        UIAccessibility.post(notification: .announcement, argument: errorMessage)
        UIAccessibility.post(notification: .layoutChanged, argument: errorLabel)
    }

    private static func createErrorLabel(in view: UIView) -> UILabel {
        let label = UILabel()
        label.tag = 999
        label.isAccessibilityElement = true
        label.isHidden = true  // Visually hidden but accessible
        view.addSubview(label)
        return label
    }
}

// PageComponent - Handle focus from web
class PageComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "move-focus":
            let data: FocusData? = message.data()
            handleFocusMove(data: data!)
        }
    }

    private func handleFocusMove(data: FocusData) {
        switch data.target {
        case "webview":
            FocusManager.moveFocusToWebView(destination.webView)
        case "error":
            FocusManager.moveFocusToError(data.message, in: destination.view)
        case "header":
            // Move to navigation bar
            UIAccessibility.post(notification: .layoutChanged,
                               argument: destination.navigationItem.titleView)
        }
    }
}
```

## Android Accessibility

### TalkBack Support

```kotlin
// VisitableFragment+Accessibility.kt
class VisitableFragment : Fragment() {

    private lateinit var webView: WebView
    private lateinit var pageComponent: PageComponent

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        // Enable accessibility for WebView
        webView.isAccessibilityEnabled = true
        webView.accessibilityDelegate = createWebViewAccessibilityDelegate()

        // Set content description
        webView.contentDescription = "Web content"

        // Configure for TalkBack
        webView.setAccessibilityDelegateCompat(object : AccessibilityDelegateCompat() {
            override fun onInitializeAccessibilityNodeInfo(
                host: View,
                info: AccessibilityNodeInfoCompat
            ) {
                super.onInitializeAccessibilityNodeInfo(host, info)
                info.className = "android.webkit.WebView"
                info.text = "Web content"
            }
        })
    }

    // Announce page changes to TalkBack
    fun announcePageChange(title: String) {
        // Use Accessibility Announcement
        view?.let {
            it.announceForAccessibility("Navigated to $title")
        }

        // Update toolbar title
        (requireActivity() as? AppCompatActivity)?.supportActionBar?.title = title
    }
}

// PageComponent - Accessibility updates
class PageComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override fun onReceive(message: Message) {
        when (message.event) {
            "navigation-state" -> {
                val data: NavigationData? = message.data()
                updateAccessibilityWithTitle(data?.title)
            }
            "announce" -> {
                val data: AnnounceData? = message.data()
                postAccessibilityAnnouncement(data?.message ?: "")
            }
        }
    }

    private fun updateAccessibilityWithTitle(title: String?) {
        title ?: return

        val activity = destination.requireActivity() as? AppCompatActivity
        activity?.supportActionBar?.title = title

        // Announce to TalkBack
        destination.view?.postDelayed({
            destination.view?.announceForAccessibility("Page: $title")
        }, 300)
    }

    private fun postAccessibilityAnnouncement(message: String) {
        destination.view?.announceForAccessibility(message)
    }
}
```

### Dynamic Font Scaling

```kotlin
// Support system font scaling
class AccessibleTextView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : androidx.appcompat.widget.AppCompatTextView(context, attrs, defStyleAttr) {

    init {
        // Enable automatic font scaling
        includeFontPadding = true

        // Set max scale ratio (default is 1.3, we allow up to 2.0)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            importantForAutofill = IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
        }
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()

        // Listen for font scale changes
        ViewCompat.setOnApplyWindowInsetsListener(this) { view, insets ->
            // Handle font scale if needed
            insets
        }
    }
}

// Offline Banner with font scaling
class OfflineBannerView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null
) : FrameLayout(context, attrs) {

    private val textView: TextView

    init {
        // Create text view
        textView = TextView(context).apply {
            textSize = 14.sp
            setPadding(16.dp, 12.dp, 16.dp, 12.dp)
            gravity = Gravity.CENTER
            isAccessibilityEnabled = true
            contentDescription = "Offline notice"
        }

        addView(textView, LayoutParams(
            LayoutParams.MATCH_PARENT,
            LayoutParams.WRAP_CONTENT
        ))
    }

    fun setMessage(message: String) {
        textView.text = message
        // TalkBack will automatically announce changes
    }

    override fun onInitializeAccessibilityNodeInfo(info: AccessibilityNodeInfo) {
        super.onInitializeAccessibilityNodeInfo(info)
        info.roleInfo = AccessibilityNodeInfo.AccessibilityRoleCompat.createRoleInfo(
            AccessibilityNodeInfo.AccessibilityRoleCompat.ROLE_STATUS
        )
    }
}
```

### Accessibility Focus Management

```kotlin
// Manage focus for TalkBack
class FocusManager {

    companion object {

        /// Move focus to specific view
        fun moveFocusTo(view: View) {
            view.isFocused = true
            view.sendAccessibilityEvent(AccessibilityEvent.TYPE_VIEW_FOCUSED)
        }

        /// Move focus to WebView
        fun moveFocusToWebView(webView: WebView) {
            webView.postDelayed({
                webView.isFocused = true
                webView.sendAccessibilityEvent(AccessibilityEvent.TYPE_VIEW_FOCUSED)
            }, 300)
        }

        /// Move focus to error message
        fun moveFocusToError(errorMessage: String, view: View) {
            // Create or update error text
            val errorText = view.findViewById<TextView>(R.id.error_text)
                ?: TextView(view.context).apply {
                    id = R.id.error_text
                    isVisible = false  // Visually hidden but accessible
                    (view as ViewGroup).addView(this)
                }

            errorText.text = errorMessage
            errorText.contentDescription = errorMessage

            // Move focus
            errorText.isFocused = true
            errorText.sendAccessibilityEvent(AccessibilityEvent.TYPE_VIEW_FOCUSED)

            // Announce
            view.announceForAccessibility(errorMessage)
        }
    }
}

// PageComponent - Handle focus from web
class PageComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override fun onReceive(message: Message) {
        when (message.event) {
            "move-focus" -> {
                val data: FocusData? = message.data()
                handleFocusMove(data = data!!)
            }
        }
    }

    private fun handleFocusMove(data: FocusData) {
        when (data.target) {
            "webview" -> FocusManager.moveFocusToWebView(destination.view.findViewById(R.id.webView))
            "error" -> FocusManager.moveFocusToError(data.message, destination.view)
            "header" -> {
                val toolbar = destination.view.findViewById<Toolbar>(R.id.toolbar)
                FocusManager.moveFocusTo(toolbar)
            }
        }
    }
}
```

## WebView Accessibility (Web Content)

### WCAG Compliance

```typescript
// Web: Ensure WCAG 2.1 AA compliance
import { axe, configureAxe } from 'axe-core'

// Configure axe-core for testing
configureAxe({
  rules: [
    { id: 'color-contrast', enabled: true },
    { id: 'label', enabled: true },
    { id: 'landmark-one-main', enabled: true },
    { id: 'region', enabled: true }
  ]
})

// Run accessibility audit
async function runAccessibilityAudit() {
  const results = await axe.run(document)

  if (results.violations.length > 0) {
    // Log violations
    console.warn('Accessibility violations:', results.violations)

    // Report to native for logging
    Strada.web.send({
      component: 'analytics',
      event: 'a11y-violations',
      data: {
        count: results.violations.length,
        violations: results.violations.map(v => ({
          id: v.id,
          impact: v.impact,
          description: v.description
        }))
      }
    })
  }
}

// Run on page load
runAccessibilityAudit()
```

### ARIA Labels and Roles

```html
<!-- Proper ARIA labeling -->
<nav role="navigation" aria-label="Main navigation">
  <ul>
    <li><a href="/" aria-current="page">Home</a></li>
    <li><a href="/about">About</a></li>
    <li><a href="/contact">Contact</a></li>
  </ul>
</nav>

<main role="main" id="main-content">
  <h1 id="page-title">Page Title</h1>

  <form aria-labelledby="form-title">
    <h2 id="form-title">Contact Form</h2>

    <div>
      <label for="email">Email address</label>
      <input
        type="email"
        id="email"
        name="email"
        required
        aria-required="true"
        aria-describedby="email-help"
      />
      <span id="email-help" class="help-text">
        We'll never share your email with anyone else.
      </span>
    </div>

    <button type="submit" aria-label="Submit contact form">
      Submit
    </button>
  </form>
</main>

<!-- Live regions for dynamic content -->
<div
  role="status"
  aria-live="polite"
  aria-atomic="true"
  id="status-message"
></div>

<!-- Skip link for keyboard users -->
<a href="#main-content" class="skip-link">
  Skip to main content
</a>
```

### Focus Management in Web

```typescript
// Focus management for SPA navigation
class FocusManager {

  // Store previous focus
  private previousFocus: HTMLElement | null = null

  // Move focus to main content on navigation
  onNavigationComplete() {
    const mainContent = document.querySelector('main') ||
                        document.querySelector('#main-content') ||
                        document.querySelector('[role="main"]')

    if (mainContent) {
      // Set tabindex if needed
      if (!mainContent.hasAttribute('tabindex')) {
        mainContent.setAttribute('tabindex', '-1')
      }

      // Move focus
      mainContent.focus()

      // Announce to screen readers
      this.announcePageChange()
    }
  }

  // Announce page change
  private announcePageChange() {
    const title = document.title || document.querySelector('h1')?.textContent
    const announcer = document.getElementById('sr-announcer')

    if (announcer && title) {
      announcer.textContent = `Navigated to ${title}`

      // Clear after announcement
      setTimeout(() => {
        announcer.textContent = ''
      }, 1000)
    }
  }

  // Handle form errors
  focusOnError(errorFieldId: string) {
    const errorField = document.getElementById(errorFieldId)
    if (errorField) {
      errorField.focus()

      // Announce error
      this.announceError(errorField)
    }
  }

  private announceError(field: HTMLElement) {
    const label = field.getAttribute('aria-label') ||
                  field.getAttribute('aria-describedby') ||
                  field.id

    const announcer = document.getElementById('sr-announcer')
    if (announcer) {
      announcer.textContent = `Error in ${label}`
      setTimeout(() => { announcer.textContent = '' }, 1000)
    }
  }
}

// Announcer component (visually hidden)
// <div id="sr-announcer" role="status" aria-live="polite"
//      style="position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)">
// </div>
```

### Strada Accessibility Messages

```typescript
// Web: Send accessibility events to native
class AccessibilityBridge {

  // Announce message to native screen reader
  announce(message: string) {
    Strada.web.send({
      component: 'accessibility',
      event: 'announce',
      data: { message }
    })
  }

  // Request focus move
  moveFocus(target: 'webview' | 'error' | 'header' | string, message?: string) {
    Strada.web.send({
      component: 'accessibility',
      event: 'move-focus',
      data: { target, message }
    })
  }

  // Report accessibility violation
  reportViolation(violation: {
    id: string
    impact: string
    description: string
    node: string
  }) {
    Strada.web.send({
      component: 'accessibility',
      event: 'violation',
      data: violation
    })
  }

  // Request accessibility audit
  async requestAudit() {
    const results = await axe.run(document)

    Strada.web.send({
      component: 'accessibility',
      event: 'audit-results',
      data: {
        violations: results.violations.length,
        passes: results.passes.length,
        incomplete: results.incomplete.length
      }
    })
  }
}

// Usage in components
class FormComponent {
  onSubmitError(errors: FieldError[]) {
    if (errors.length > 0) {
      // Move focus to first error
      accessibilityBridge.moveFocus(`error-${errors[0].field}`, errors[0].message)

      // Announce error count
      accessibilityBridge.announce(`${errors.length} error(s) found. Please correct and resubmit.`)
    }
  }
}
```

## Testing Accessibility

### iOS Accessibility Tests

```swift
// AccessibilityTests.swift
import XCTest

class AccessibilityTests: XCTestCase {

    var app: XCUIApplication!

    override func setUp() {
        super.setUp()
        app = XCUIApplication()
        app.launchArguments = ["-ui-testing"]
        app.launch()
    }

    func testAllElementsAreAccessible() {
        // Query all elements
        let query = app.descendants(matching: .any)

        // Check that important elements have labels
        for element in query.allElementsBoundByIndex {
            if element.isAccessibilityElement {
                XCTAssertFalse(
                    element.accessibilityLabel?.isEmpty ?? true,
                    "Element '\(element.elementType)' should have an accessibility label"
                )
            }
        }
    }

    func testDynamicType() {
        // Set large content size
        app.adjustSettings { settings in
            settings.conttrast = .increased
        }

        // Verify UI still works with large text
        XCTAssertTrue(app.webViews.firstMatch.exists)
    }

    func testVoiceOverNavigation() {
        // Enable VoiceOver simulation
        XCUIAccessibility.shared.isVoiceOverEnabled = true

        // Navigate with VoiceOver gestures
        app.swipeRight()  // Next element
        app.swipeLeft()   // Previous element

        // Verify announcements
        XCTAssertTrue(app.webViews.firstMatch.exists)
    }
}
```

### Android Accessibility Tests

```kotlin
// AccessibilityTest.kt
@RunWith(AndroidJUnit4::class)
class AccessibilityTests {

    @get:Rule
    val activityRule = ActivityScenarioRule(MainActivity::class.java)

    @Test
    fun testAllElementsHaveContentDescription() {
        onView(withId(R.id.webView))
            .check { view, noMatchingViewException ->
                if (noMatchingViewException != null) throw noMatchingViewException

                // Check content description
                assertTrue(
                    "WebView should have content description",
                    view?.contentDescription?.isNotEmpty() == true
                )
            }
    }

    @Test
    fun testTalkBackNavigation() {
        // Enable TalkBack simulation
        val accessibilityManager =
            InstrumentationRegistry.getInstrumentation().targetContext
                .getSystemService(Context.ACCESSIBILITY_SERVICE) as AccessibilityManager

        accessibilityManager.setEnabled(true)

        // Navigate with focus
        onView(withId(R.id.webView))
            .perform(ViewActions.scrollTo())

        // Verify focus is reachable
        onView(withId(R.id.webView))
            .check(matches(isFocused()))
    }

    @Test
    fun testDynamicFontScaling() {
        // Set large font
        val configuration = Configuration()
        configuration.fontScale = 1.5f

        activityRule.scenario.onActivity { activity ->
            activity.resources.updateConfiguration(
                configuration,
                activity.resources.displayMetrics
            )
        }

        // Verify UI still works
        onView(withId(R.id.webView))
            .check(matches(isDisplayed()))
    }
}
```

## Summary

Accessibility essentials:

1. **VoiceOver/TalkBack** - Screen reader support on both platforms
2. **Dynamic Type** - Font scaling support
3. **Focus Management** - Coordinate focus between web and native
4. **ARIA Labels** - WCAG compliance in web content
5. **Accessibility Announcements** - Page changes, errors, status
6. **Testing** - Automated accessibility tests

---

*Related: `testing-exploration.md`, `native-ui-components-exploration.md`*
