# Strada Production - Testing Exploration

## Overview

This document explores testing strategies for production iOS/Android apps using WebViews and Strada.

## Testing Architecture

### Test Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                    E2E Tests                                    │
│  - Detox / Maestro / Appium                                     │
│  - Full app flow with real WebView                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Integration Tests                            │
│  - XCUITest / Espresso                                          │
│  - Native UI + mocked WebView                                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Component Tests                              │
│  - XCTest / JUnit                                               │
│  - Bridge components, message handling                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Unit Tests                                   │
│  - XCTest / JUnit                                               │
│  - Message serialization, business logic                        │
└─────────────────────────────────────────────────────────────────┘
```

## iOS Testing

### Unit Tests for Bridge Components

```swift
// BridgeComponentTests.swift
import XCTest
@testable import Strada

class PageComponentTests: XCTestCase {

    var component: PageComponent!
    var delegateSpy: BridgeDelegateSpy!

    override func setUp() {
        super.setUp()
        delegateSpy = BridgeDelegateSpy()
        component = PageComponent(
            destination: MockDestination(),
            delegate: delegateSpy
        )
    }

    func testConnectMessage_UpdatesNavigationBar() {
        // Arrange
        let message = Message(
            id: "test-1",
            component: "page",
            event: "connect",
            metadata: nil,
            jsonData: #"{"title": "Test Page", "showBackButton": true}"#
        )

        // Act
        component.onReceive(message: message)

        // Assert
        XCTAssertEqual(destinationSpy.lastSetTitle, "Test Page")
        XCTAssertFalse(destinationSpy.navigationItem.hidesBackButton)
    }

    func testNavigationState_UpdatesTitle() {
        // Arrange
        let message = Message(
            id: "test-2",
            component: "page",
            event: "navigation-state",
            metadata: nil,
            jsonData: #"{"title": "New Title"}"#
        )

        // Act
        component.onReceive(message: message)

        // Assert
        XCTAssertEqual(destinationSpy.lastSetTitle, "New Title")
    }

    func testBackButtonTapped_SendsMessageToWeb() {
        // Arrange
        let backButtonMessage = Message(
            id: "test-3",
            component: "page",
            event: "back-tapped",
            metadata: nil,
            jsonData: "{}"
        )

        // Act
        component.onReceive(message: backButtonMessage)

        // Assert
        let reply = delegateSpy.lastReply
        XCTAssertEqual(reply?.event, "back-handled")
        XCTAssertEqual(reply?.data["action"] as? String, "native-back")
    }
}

// Mock destination for testing
class MockDestination: BridgeDestination {
    var navigationItem = UINavigationItem()
    var lastSetTitle: String?

    func goBack() {
        // Track for test assertions
    }
}

// Delegate spy for capturing replies
class BridgeDelegateSpy: BridgingDelegate {
    var lastReply: Message?

    func reply(with message: Message) {
        lastReply = message
    }
}
```

### Message Serialization Tests

```swift
// MessageTests.swift
import XCTest
@testable import Strada

class MessageTests: XCTestCase {

    func testMessageSerialization_RoundTrip() {
        // Arrange
        let originalMessage = Message(
            id: "msg-123",
            component: "form",
            event: "submit",
            metadata: MessageMetadata(url: "https://example.com/form"),
            jsonData: #"{"field": "value"}"#
        )

        // Act - Serialize
        let internalMessage = InternalMessage(message: originalMessage)
        let json = internalMessage.toJSON()

        // Act - Deserialize
        let deserialized = InternalMessage.fromJson(json)
        let resultMessage = deserialized?.toMessage()

        // Assert
        XCTAssertEqual(originalMessage.id, resultMessage?.id)
        XCTAssertEqual(originalMessage.component, resultMessage?.component)
        XCTAssertEqual(originalMessage.event, resultMessage?.event)
    }

    func testMessageWithCodableData() {
        // Arrange
        struct FormData: Codable {
            let email: String
            let password: String
        }

        let formData = FormData(email: "test@example.com", password: "secret")
        let jsonData = try! JSONEncoder().encode(formData)
        let jsonString = String(data: jsonData, encoding: .utf8)!

        let message = Message(
            id: "test",
            component: "auth",
            event: "login",
            metadata: nil,
            jsonData: jsonString
        )

        // Act - Decode
        let decoded: FormData? = message.data()

        // Assert
        XCTAssertEqual(decoded?.email, "test@example.com")
        XCTAssertEqual(decoded?.password, "secret")
    }
}
```

### XCUITest for WebView Integration

```swift
// WebViewIntegrationTests.swift
import XCTest

class WebViewIntegrationTests: XCTestCase {

    var app: XCUIApplication!

    override func setUp() {
        super.setUp()
        continueAfterFailure = false

        app = XCUIApplication()
        app.launchArguments = ["-ui-testing", "-mock-server"]
        app.launch()
    }

    func testWebViewLoads_Successfully() {
        // Wait for WebView to appear
        let webView = app.webViews.firstMatch
        XCTAssertTrue(webView.waitForExistence(timeout: 10))

        // Verify expected content
        let pageTitle = webView.staticTexts["Welcome"]
        XCTAssertTrue(pageTitle.exists)
    }

    func testNativeBackButton_WhenWebViewHasHistory() {
        let webView = app.webViews.firstMatch

        // Navigate to a new page (via web link)
        webView.links["Go to Details"].tap()

        // Wait for navigation
        sleep(2)

        // Tap native back button
        app.navigationBars.buttons.element(boundBy: 0).tap()

        // Verify we're back on home
        XCTAssertTrue(webView.staticTexts["Welcome"].waitForExistence(timeout: 5))
    }

    func testOfflineMode_ShowsBanner() {
        // Enable airplane mode simulation
        // (Requires test helper or device configuration)

        let webView = app.webViews.firstMatch
        XCTAssertTrue(webView.waitForExistence(timeout: 5))

        // Tap element that triggers offline action
        webView.buttons["Save for Later"].tap()

        // Verify offline banner appears
        let banner = app.otherTexts["You're offline"]
        XCTAssertTrue(banner.waitForExistence(timeout: 3))
    }

    func testFormSubmission_WithNativeValidation() {
        let webView = app.webViews.firstMatch

        // Fill form
        webView.textFields["Email"].typeText("test@example.com")
        webView.secureTextFields["Password"].typeText("password123")

        // Submit
        webView.buttons["Sign In"].tap()

        // Wait for native loading indicator
        let indicator = app.activityIndicators.firstMatch
        XCTAssertTrue(indicator.waitForExistence(timeout: 5))

        // Verify navigation to logged-in state
        XCTAssertTrue(webView.staticTexts["Welcome back"].waitForExistence(timeout: 10))
    }
}
```

### Mocking Web Content for Tests

```swift
// MockServer.swift
class MockServer {

    private var server: HTTPServer?

    func start() throws {
        server = HTTPServer(port: 8080)

        // Define mock responses
        server?.route("GET", "/") { _ in
            HTTPResponse(
                status: .ok,
                headers: ["Content-Type": "text/html"],
                body: self.homePageHTML
            )
        }

        server?.route("POST", "/api/login") { request in
            let body = request.body
            if body.contains("test@example.com") {
                return HTTPResponse(status: .ok, body: #"{"token": "mock-token"}"#)
            }
            return HTTPResponse(status: .unauthorized, body: #"{"error": "Invalid"}"#)
        }

        try server?.start()
    }

    func stop() {
        server?.stop()
    }

    private var homePageHTML: String {
        return """
        <!DOCTYPE html>
        <html>
        <head>
            <script src="/strada-web.js"></script>
        </head>
        <body>
            <h1>Welcome</h1>
            <a href="/details">Go to Details</a>
            <button onclick="Strada.web.send({component:'test',event:'clicked'})">
                Test Button
            </button>
        </body>
        </html>
        """
    }
}
```

## Android Testing

### Unit Tests for Bridge Components

```kotlin
// PageComponentTest.kt
@RunWith(MockitoJUnitRunner::class)
class PageComponentTest {

    @Mock
    private lateinit var destination: BridgeDestination

    @Mock
    private lateinit var delegate: BridgeDelegate

    @Mock
    private lateinit var activity: AppCompatActivity

    private lateinit var component: PageComponent

    @Before
    fun setUp() {
        `when`(destination.requireActivity()).thenReturn(activity)
        `when`(activity.supportActionBar).thenReturn(mock())

        component = PageComponent()
        component.attach(destination, delegate)
    }

    @Test
    fun `connect message updates toolbar title`() {
        // Arrange
        val message = Message(
            id = "test-1",
            component = "page",
            event = "connect",
            jsonData = """{"title": "Test Page", "showBackButton": true}"""
        )

        // Act
        component.onReceive(message)

        // Assert
        verify(activity.supportActionBar).title = "Test Page"
        verify(activity.supportActionBar).setDisplayHomeAsUpEnabled(true)
    }

    @Test
    fun `navigation state updates title`() {
        // Arrange
        val message = Message(
            id = "test-2",
            component = "page",
            event = "navigation-state",
            jsonData = """{"title": "New Title"}"""
        )

        // Act
        component.onReceive(message)

        // Assert
        verify(activity.supportActionBar).title = "New Title"
    }

    @Test
    fun `back tapped sends handled reply`() {
        // Arrange
        val message = Message(
            id = "test-3",
            component = "page",
            event = "back-tapped",
            jsonData = """{}"""
        )

        // Act
        component.onReceive(message)

        // Assert
        verify(delegate).replyWith(argThat {
            event == "back-handled" &&
            jsonData.contains("native-back")
        })
    }
}
```

### Message Serialization Tests

```kotlin
// MessageTest.kt
class MessageTest {

    @Test
    fun `message serialization round trip`() {
        // Arrange
        val originalMessage = Message(
            id = "msg-123",
            component = "form",
            event = "submit",
            metadata = MessageMetadata(url = "https://example.com/form"),
            jsonData = """{"field": "value"}"""
        )

        // Act - Serialize
        val internalMessage = InternalMessage.fromMessage(originalMessage)
        val json = internalMessage.toJson()

        // Act - Deserialize
        val deserialized = InternalMessage.fromJson(json)
        val resultMessage = deserialized?.toMessage()

        // Assert
        assertEquals(originalMessage.id, resultMessage?.id)
        assertEquals(originalMessage.component, resultMessage?.component)
        assertEquals(originalMessage.event, resultMessage?.event)
    }

    @Test
    fun `message with codable data`() {
        // Arrange
        data class FormData(val email: String, val password: String)

        val formData = FormData("test@example.com", "secret")
        val jsonString = Json.encodeToString(formData)

        val message = Message(
            id = "test",
            component = "auth",
            event = "login",
            jsonData = jsonString
        )

        // Act - Decode
        val decoded: FormData? = message.data()

        // Assert
        assertEquals("test@example.com", decoded?.email)
        assertEquals("secret", decoded?.password)
    }
}
```

### Espresso Tests for WebView

```kotlin
// WebViewIntegrationTest.kt
@RunWith(AndroidJUnit4::class)
class WebViewIntegrationTest {

    @get:Rule
    val activityRule = ActivityScenarioRule(MainActivity::class.java)

    @Test
    fun testWebViewLoadsSuccessfully() {
        // Wait for WebView
        onView(withId(R.id.webView))
            .check(matches(isDisplayed()))

        // Verify expected content
        onView(withText("Welcome"))
            .check(matches(isDisplayed()))
    }

    @Test
    fun testNativeBackButton_WhenWebViewHasHistory() {
        // Navigate via web link
        onView(withId(R.id.webView))
            .perform(WebViewActions.clickLinkWithText("Go to Details"))

        // Wait for navigation
        Thread.sleep(2000)

        // Press back
        Espresso.pressBack()

        // Verify back on home
        onView(withText("Welcome"))
            .check(matches(isDisplayed()))
    }

    @Test
    fun testFormSubmission_WithNativeValidation() {
        // Fill form
        onView(withId(R.id.webView))
            .perform(WebViewActions.enterTextInField("email", "test@example.com"))
            .perform(WebViewActions.enterTextInField("password", "password123"))

        // Submit
        onView(withId(R.id.webView))
            .perform(WebViewActions.clickButton("Sign In"))

        // Wait for loading
        onView(withId(R.id.progressBar))
            .check(matches(isDisplayed()))

        // Verify logged in state
        onView(withText("Welcome back"))
            .check(matches(isDisplayed()))
    }

    @Test
    fun testOfflineMode_ShowsSnackbar() {
        // Simulate offline (requires test utility)
        NetworkTestUtils.setOffline(true)

        // Trigger offline action
        onView(withId(R.id.webView))
            .perform(WebViewActions.clickButton("Save for Later"))

        // Verify snackbar
        onView(withId(com.google.android.material.R.id.snackbar_text))
            .check(matches(withText(containsString("offline"))))
    }
}

// WebView test helpers
object WebViewActions {

    fun clickLinkWithText(text: String): ViewAction {
        return object : ViewAction {
            override fun getDescription() = "Click link with text: $text"
            override fun getConstraints() = isAssignableFrom(WebView::class.java)

            override fun perform(uiController: UiController, view: View) {
                val webView = view as WebView
                webView.loadUrl("javascript:(function() { " +
                    "document.querySelector('a[href*=\"$text\"]').click(); " +
                    "})()")
            }
        }
    }

    fun enterTextInField(fieldId: String, text: String): ViewAction {
        // Implementation for entering text in web form field
    }

    fun clickButton(buttonText: String): ViewAction {
        // Implementation for clicking web button
    }
}
```

### Mocking WebContent

```kotlin
// MockWebServerRule.kt
class MockWebServerRule : TestRule {

    private val server = MockWebServer()

    fun baseUrl(): String = server.url("/").toString()

    override fun apply(base: Statement, description: Description): Statement {
        return object : Statement() {
            override fun evaluate() {
                try {
                    server.start()
                    setupMockResponses()
                    base.evaluate()
                } finally {
                    server.shutdown()
                }
            }
        }
    }

    private fun setupMockResponses() {
        server.enqueue(MockResponse()
            .setResponseCode(200)
            .setBody(homePageHTML))

        server.enqueue(MockResponse()
            .setResponseCode(200)
            .setBody("""{"token": "mock-token"}""")
            .setHeader("Content-Type", "application/json"))
    }

    private val homePageHTML = """
        <!DOCTYPE html>
        <html>
        <head>
            <script src="/strada-web.js"></script>
        </head>
        <body>
            <h1>Welcome</h1>
            <a href="/details">Go to Details</a>
        </body>
        </html>
    """.trimIndent()
}

// Usage in test
@RunWith(AndroidJUnit4::class)
class WebViewTest {

    @get:Rule
    val serverRule = MockWebServerRule()

    @get:Rule
    val activityRule = ActivityScenarioRule<MainActivity>(
        Intent(ApplicationProvider.getApplicationContext(), MainActivity::class.java)
            .putExtra("test_url", serverRule.baseUrl())
    ).apply {
        afterActivityLaunched {
            // Additional setup after activity launches
        }
    }
}
```

## E2E Testing

### Detox (iOS/Android)

```javascript
// e2e/app.test.js
const { device, element, by, expect } = require('detox');

describe('Strada App E2E Tests', () => {
  beforeAll(async () => {
    await device.launchApp({
      newInstance: true,
      permissions: { notifications: 'YES' },
    });
  });

  beforeEach(async () => {
    await device.reloadReactNative();
  });

  it('should load home screen successfully', async () => {
    await expect(element(by.text('Welcome'))).toBeVisible();
  });

  it('should navigate via web link and go back', async () => {
    // Tap web link
    await element(by.web().index(0)).tap();

    // Wait for navigation
    await waitFor(element(by.text('Details'))).toBeVisible().withTimeout(5000);

    // Go back
    await element(by.type('_UINavigationBarBackIndicatorDefault')).tap();

    // Verify back on home
    await waitFor(element(by.text('Welcome'))).toBeVisible().withTimeout(3000);
  });

  it('should handle form submission', async () => {
    // Fill form fields
    await element(by.web().field('email')).typeText('test@example.com');
    await element(by.web().field('password')).typeText('password123');

    // Submit
    await element(by.web().button('Sign In')).tap();

    // Verify success
    await waitFor(element(by.text('Welcome back'))).toBeVisible().withTimeout(10000);
  });

  it('should show offline indicator when network disabled', async () => {
    // Disable network
    await device.disableSynchronization();

    // Trigger offline action
    await element(by.web().button('Save for Later')).tap();

    // Verify offline indicator
    await waitFor(element(by.text(contains('offline')))).toBeVisible().withTimeout(3000);

    // Re-enable network
    await device.enableSynchronization();
  });
});
```

### Maestro Flows

```yaml
# e2e/flows/login.yaml
appId: com.example.app
---
- launchApp
- assertVisible: "Welcome"
- tapOn: "Sign In"
- assertVisible: "Email"
- inputText: "test@example.com"
- tapOn: "Password"
- inputText: "password123"
- tapOn: "Sign In"
- assertVisible: "Welcome back"
- stopApp

# e2e/flows/navigation.yaml
appId: com.example.app
---
- launchApp
- assertVisible: "Welcome"
- tapOn:
    web:
      link: "Go to Details"
- assertVisible: "Details Page"
- pressBack
- assertVisible: "Welcome"
- stopApp
```

## Screenshot Testing

### iOS Snapshot Testing

```swift
// ScreenshotTests.swift
import SnapshotTesting
import XCTest

class ScreenshotTests: XCTestCase {

    func testHomePage() {
        let vc = VisitableViewController(url: URL(string: "http://localhost:8080/")!)

        // Configure to specific size
        vc.preferredContentSize = CGSize(width: 375, height: 667)

        assertSnapshot(matching: vc, as: .image(on: .iPhone8))
    }

    func testOfflineBanner() {
        let vc = VisitableViewController(url: URL(string: "http://localhost:8080/")!)

        // Simulate offline state
        vc.showOfflineBanner()

        assertSnapshot(matching: vc, as: .image(on: .iPhone8))
    }

    func testNativeLoadingIndicator() {
        let vc = VisitableViewController(url: URL(string: "http://localhost:8080/")!)

        // Show loading
        vc.showLoadingIndicator()

        assertSnapshot(matching: vc, as: .image(on: .iPhone8))
    }
}
```

### Android Screenshot Tests

```kotlin
// ScreenshotTest.kt
@RunWith(ScreenshotTestRunner::class)
class ScreenshotTests {

    @get:Rule
    val screenshotRule = ScreenshotRule(
        activityRule = ActivityScenarioRule(MainActivity::class.java)
    )

    @Test
    fun testHomePage() {
        screenshotRule
            .forView(R.id.webView)
            .assertSnapshot("home_page")
    }

    @Test
    fun testOfflineSnackbar() {
        screenshotRule
            .forView(R.id.webView)
            .withAction { activity ->
                activity.showOfflineSnackbar()
            }
            .assertSnapshot("offline_state")
    }
}
```

## Summary

Testing pyramid for WebView apps:

1. **Unit Tests** - Message serialization, component logic
2. **Component Tests** - Bridge component behavior with mocks
3. **Integration Tests** - XCUITest/Espresso with WebView
4. **E2E Tests** - Detox/Maestro for full flows
5. **Screenshot Tests** - Visual regression testing

---

*Related: `deployment-cicd-exploration.md`, `accessibility-exploration.md`*
