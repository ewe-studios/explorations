# UTM Dev Production - Accessibility (a11y) Exploration

## Overview

This document explores accessibility (a11y) strategies for the UTM Dev desktop VM management application, which uses a WebView with HTMX/Datastar on macOS and Linux. The goal is to ensure full accessibility compliance for users relying on screen readers (VoiceOver on macOS, ORCA on Linux), keyboard navigation, and other assistive technologies.

UTM Dev presents unique accessibility challenges:
- **Hybrid architecture**: Native window chrome + WebView content
- **Dynamic content**: VM status updates, console output, real-time metrics
- **Complex interactions**: Drag-and-drop, VM controls, settings panels
- **Cross-platform**: macOS (VoiceOver) and Linux (ORCA/AT-SPI)

## Architecture

### Accessibility Layers for UTM Dev

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Native Application Layer (Rust/Swift)                    │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐ │
│  │  Window/View    │  │  Native Menu    │  │  System Event Handler       │ │
│  │  Controller     │  │  Bar            │  │  (Keyboard/Mouse)           │ │
│  │  - NSView       │  │  - NSMenu       │  │  - NSEvent / GTK events     │ │
│  │  - GTK Widget   │  │  - D-Bus        │  │  - Global shortcuts         │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘ │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │              Native Accessibility API Layer                            │ │
│  │  macOS: NSAccessibilityProtocol                                        │ │
│  │  Linux: AT-SPI (Assistive Technology Service Provider Interface)       │ │
│  │  - Accessibility tree                                                  │ │
│  │  - Roles, states, values                                               │ │
│  │  - Focus management                                                    │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ Focus coordination
                                    │ Event propagation
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    WebView Layer (WebKit/GTK WebKit)                        │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │              Web Content Accessibility                                 │ │
│  │  - WCAG 2.1 AA compliance                                              │ │
│  │  - ARIA 1.2 attributes                                                 │ │
│  │  - HTML5 semantic elements                                             │ │
│  │  - Focus management                                                    │ │
│  │  - Keyboard navigation                                                 │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │              HTMX/Datastar Integration                                 │ │
│  │  - Live regions for dynamic updates                                    │ │
│  │  - ARIA announcements for state changes                                │ │
│  │  - Focus restoration after swaps                                       │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ Screen reader
                                    │ announcements
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Assistive Technology Layer                               │
│  macOS: VoiceOver              Linux: ORCA, Dasher, etc.                    │
│  - Screen reader               - Screen reader                              │
│  - Accessibility Inspector     - Accerciser                                 │
│  - Zoom/Magnifier              - High contrast themes                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Accessibility Data Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   User       │────▶│  Native      │────▶│  WebView     │
│  Interaction │     │  Handler     │     │  Content     │
└──────────────┘     └──────────────┘     └──────────────┘
       ▲                    │                    │
       │                    ▼                    ▼
       │            ┌──────────────┐     ┌──────────────┐
       │            │  Accessibility│    │  ARIA Live   │
       │            │  Tree Update  │     │  Regions     │
       │            └──────────────┘     └──────────────┘
       │                    │                    │
       │                    ▼                    ▼
       │            ┌──────────────────────────────┐
       │            │   Screen Reader              │
       │            │   Announcement               │
       │            └──────────────────────────────┘
       │                    │
       └────────────────────┘
            Audio/Tactile
            Feedback
```

---

## 1. macOS VoiceOver Support

### NSAccessibility Protocol Implementation

The native macOS components must implement the `NSAccessibility` protocol to expose UI elements to VoiceOver.

#### Window Controller Accessibility

```swift
// MainWindowController.swift
import Cocoa
import WebKit

class MainWindowController: NSWindowController, NSWindowDelegate {

    override func windowDidLoad() {
        super.windowDidLoad()

        guard let window = self.window else { return }

        // Configure window for accessibility
        window.isAccessibilityElement = true
        window.accessibilityTitle = "UTM Dev - Virtual Machine Manager"
        window.accessibilityRole = .window
        window.accessibilitySubrole = .standardWindow

        // Set main content description
        window.accessibilityDescription = """
            UTM Dev virtual machine management interface.
            Contains VM list, control buttons, and console view.
            """

        // Enable focus following
        window.accessibilityFocusRingStyle = .default
    }

    // Provide custom accessibility children
    override func accessibilityChildren() -> [Any]? {
        // Return array of accessible subviews in logical order
        return [
            toolbarView,      // Native toolbar
            sidebarView,      // VM list sidebar
            contentView,      // WebView container
            statusBarView     // Status bar
        ].compactMap { $0 }
    }

    // Handle accessibility focused element
    override func accessibilityFocusedUIElement() -> NSAccessibilityElement? {
        // Return the element that should receive focus
        return focusedElement
    }
}
```

#### WebView Container Accessibility

```swift
// WebViewContainer.swift
class WebViewContainer: NSView {

    private var webView: WKWebView!
    private var accessibilityBridge: AccessibilityBridge!

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        setupAccessibility()
        setupWebView()
        setupAccessibilityBridge()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setupAccessibility()
    }

    private func setupAccessibility() {
        // This view is a container, not a leaf element
        isAccessibilityElement = false

        // But it does have an accessibility role
        accessibilityRole = .group
        accessibilityLabel = "Virtual machine content area"
        accessibilityDescription = "Contains the VM console and controls"

        // Notify when content changes significantly
        accessibilityNotifiesWhenDestroyed = true
    }

    private func setupWebView() {
        webView = WKWebView(frame: bounds)

        // Configure WebView accessibility
        setupWebViewAccessibility()

        addSubview(webView)
    }

    private func setupWebViewAccessibility() {
        // WebView itself is an accessibility element
        webView.isAccessibilityElement = true
        webView.accessibilityRole = .webArea
        webView.accessibilityLabel = "VM console and interface"

        // Enable accessibility scripting
        webView.configuration.preferences.javaScriptCanOpenWindowsAutomatically = true

        // Inject accessibility enhancement script
        let script = """
            (function() {
                // Ensure all interactive elements are focusable
                document.querySelectorAll('button, a, input, [tabindex]').forEach(el => {
                    if (!el.hasAttribute('aria-label') && !el.textContent.trim()) {
                        console.warn('Unlabeled interactive element:', el);
                    }
                });
            })();
            """

        let userScript = WKUserScript(
            source: script,
            injectionTime: .atDocumentEnd,
            forMainFrameOnly: true
        )

        webView.configuration.userContentController.addUserScript(userScript)
    }

    // Notify accessibility of focus changes
    func notifyFocusChanged(to element: String) {
        NSAccessibility.post(
            element: self,
            notification: .focusedUIElementChanged,
            userInfo: ["element": element]
        )
    }
}
```

#### VM List Item Accessibility

```swift
// VMListItemView.swift
class VMListItemView: NSView {

    // MARK: - Properties

    var vmInfo: VMInfo! {
        didSet { updateAccessibility() }
    }

    private let nameLabel: NSTextField
    private let statusIndicator: NSView
    private let powerButton: NSButton

    // MARK: - Initialization

    override init(frame frameRect: NSRect) {
        nameLabel = NSTextField(labelWithString: "")
        nameLabel.isBezeled = false
        nameLabel.isEditable = false
        nameLabel.backgroundColor = .clear

        statusIndicator = NSView()
        statusIndicator.wantsLayer = true
        statusIndicator.layer?.cornerRadius = 4

        powerButton = NSButton(title: "", target: nil, action: nil)
        powerButton.bezelStyle = .accessoryBar

        super.init(frame: frameRect)
        setupSubviews()
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    // MARK: - Accessibility Configuration

    private func setupSubviews() {
        // Configure nameLabel for accessibility
        nameLabel.isAccessibilityElement = true
        nameLabel.accessibilityRole = .staticText
        nameLabel.accessibilityIdentifier = "vm-name-label"

        // Configure status indicator
        statusIndicator.isAccessibilityElement = true
        statusIndicator.accessibilityRole = .image
        statusIndicator.accessibilityIdentifier = "vm-status-indicator"

        // Configure power button
        powerButton.isAccessibilityElement = true
        powerButton.accessibilityRole = .button
        powerButton.accessibilityIdentifier = "vm-power-button"
        powerButton.accessibilityHint = "Toggle VM power state"
    }

    private func updateAccessibility() {
        guard let vmInfo = vmInfo else { return }

        // Update name label
        nameLabel.stringValue = vmInfo.name
        nameLabel.accessibilityLabel = vmInfo.name

        // Update status with descriptive text
        let statusDescription = statusDescription(for: vmInfo.status)
        statusIndicator.accessibilityLabel = "Status: \(statusDescription)"
        statusIndicator.accessibilityTitle = statusDescription

        // Update power button based on state
        updatePowerButtonAccessibility(for: vmInfo.status)

        // Post notification for screen readers
        NSAccessibility.post(
            element: self,
            notification: .titleChanged,
            userInfo: nil
        )
    }

    private func statusDescription(for status: VMStatus) -> String {
        switch status {
        case .running:
            return "Running"
        case .paused:
            return "Paused"
        case .stopped:
            return "Stopped"
        case .error:
            return "Error - \(vmInfo.errorMessage ?? "Unknown")"
        }
    }

    private func updatePowerButtonAccessibility(for status: VMStatus) {
        switch status {
        case .running:
            powerButton.accessibilityLabel = "Stop \(vmInfo.name)"
            powerButton.accessibilityHint = "Click to stop this virtual machine"
            powerButton.toolTip = "Stop VM"
        case .stopped:
            powerButton.accessibilityLabel = "Start \(vmInfo.name)"
            powerButton.accessibilityHint = "Click to start this virtual machine"
            powerButton.toolTip = "Start VM"
        case .paused:
            powerButton.accessibilityLabel = "Resume \(vmInfo.name)"
            powerButton.accessibilityHint = "Click to resume this virtual machine"
            powerButton.toolTip = "Resume VM"
        case .error:
            powerButton.accessibilityLabel = "Retry \(vmInfo.name)"
            powerButton.accessibilityHint = "Click to retry starting this virtual machine"
            powerButton.toolTip = "Retry"
        }
    }

    // MARK: - NSAccessibility Protocol

    override var isAccessibilityElement: Bool {
        get { true }
        set { super.isAccessibilityElement = newValue }
    }

    override var accessibilityRole: NSAccessibility.Role? {
        get { .row }
        set { super.accessibilityRole = newValue }
    }

    override var accessibilityLabel: String? {
        get { "\(vmInfo?.name ?? ""), \(statusDescription(for: vmInfo?.status ?? .stopped))" }
        set { super.accessibilityLabel = newValue }
    }

    override func accessibilityChildren() -> [Any]? {
        return [nameLabel, statusIndicator, powerButton]
    }

    // Support keyboard navigation
    override var acceptsFirstResponder: Bool { true }

    override func becomeFirstResponder() -> Bool {
        let result = super.becomeFirstResponder()
        if result {
            NSAccessibility.post(
                element: self,
                notification: .focusedUIElementChanged,
                userInfo: nil
            )
        }
        return result
    }
}
```

### Dynamic Type and Scaling

```swift
// AccessibleTextField.swift
import Cocoa

class AccessibleTextField: NSTextField {

    // MARK: - Dynamic Type Support

    private var baseFontSize: CGFloat = 13.0
    private var currentFontSize: CGFloat = 13.0

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        setupDynamicType()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setupDynamicType()
    }

    private func setupDynamicType() {
        // Listen for accessibility preference changes
        DistributedNotificationCenter.default.addObserver(
            self,
            selector: #selector(accessibilityPreferencesChanged),
            name: NSNotification.Name("NSAccessibilityUpdatePreferences"),
            object: nil
        )

        // Apply initial font based on system settings
        applyDynamicFont()
    }

    @objc private func accessibilityPreferencesChanged() {
        DispatchQueue.main.async {
            self.applyDynamicFont()
        }
    }

    private func applyDynamicFont() {
        // Get system font scaling factor
        let fontScale = getSystemFontScale()
        currentFontSize = baseFontSize * fontScale

        // Apply scaled font
        let newFont = NSFont.systemFont(ofSize: currentFontSize, weight: .regular)
        self.font = newFont

        // Adjust line height for readability
        if fontScale > 1.2 {
            self.lineBreakMode = .byWordWrapping
        }
    }

    private func getSystemFontScale() -> CGFloat {
        // Check for increased contrast / larger text settings
        let accessibilityEnabled = NSWorkspace.shared.accessibilityDisplayShouldIncreaseContrast

        // Get user's preferred font size from UserDefaults
        if let fontSize = UserDefaults.standard.object(forKey: "NSFontSize") as? CGFloat {
            return fontSize / baseFontSize
        }

        return accessibilityEnabled ? 1.2 : 1.0
    }

    // MARK: - Custom Scaling

    func setBaseFontSize(_ size: CGFloat) {
        baseFontSize = size
        applyDynamicFont()
    }

    func adjustFontSize(by delta: CGFloat) {
        baseFontSize = max(10, min(72, baseFontSize + delta))
        applyDynamicFont()
    }
}

// Usage in VM list
class VMListCellView: NSTableCellView {

    private let vmNameField = AccessibleTextField()
    private let vmStatusField = AccessibleTextField()

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)

        vmNameField.font = NSFont.boldSystemFont(ofSize: 14)
        vmStatusField.font = NSFont.systemFont(ofSize: 12)

        // Accessibility identifiers for testing
        vmNameField.accessibilityIdentifier = "vm-name-field"
        vmStatusField.accessibilityIdentifier = "vm-status-field"
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}
```

### Focus Management for macOS

```swift
// FocusManager.swift
import Cocoa
import WebKit

class FocusManager {

    // MARK: - Types

    enum FocusTarget {
        case sidebar
        case webView
        case toolbar
        case statusBar
        case specificElement(NSView)
        case webElement(String)  // CSS selector
    }

    // MARK: - Properties

    private weak var window: NSWindow?
    private weak var webView: WKWebView?
    private var focusHistory: [NSView] = []
    private var webFocusStack: [String] = []

    // MARK: - Initialization

    init(window: NSWindow, webView: WKWebView) {
        self.window = window
        self.webView = webView
        setupGlobalFocusHandler()
    }

    // MARK: - Public API

    /// Move focus to the specified target
    func moveFocus(to target: FocusTarget) {
        switch target {
        case .sidebar:
            focusSidebar()
        case .webView:
            focusWebView()
        case .toolbar:
            focusToolbar()
        case .statusBar:
            focusStatusBar()
        case .specificElement(let view):
            focusView(view)
        case .webElement(let selector):
            focusWebElement(selector)
        }
    }

    /// Restore focus to previously focused element
    func restoreFocus() {
        if let previousView = focusHistory.popLast() {
            focusView(previousView, announce: false)
        }
    }

    /// Save current focus for later restoration
    func saveFocusState() {
        if let currentFocus = window?.firstResponder as? NSView {
            focusHistory.append(currentFocus)
        }
    }

    /// Announce status change to VoiceOver
    func announce(_ message: String) {
        NSAccessibility.post(
            element: window ?? self,
            notification: .announcement,
            userInfo: [
                .announcementKey: message,
                .announcementPriorityKey: NSAccessibility.Priority.low
            ]
        )
    }

    /// Announce urgent status change (interrupts current speech)
    func announceUrgent(_ message: String) {
        NSAccessibility.post(
            element: window ?? self,
            notification: .announcement,
            userInfo: [
                .announcementKey: message,
                .announcementPriorityKey: NSAccessibility.Priority.high
            ]
        )
    }

    // MARK: - Private Implementation

    private func focusSidebar() {
        // Find and focus sidebar view
        if let sidebar = window?.contentView?.subviews.first(where: {
            $0.accessibilityRole == .outline
        }) {
            focusView(sidebar)
        }
    }

    private func focusWebView() {
        guard let webView = webView else { return }

        // Make window key
        window?.makeKeyAndOrderFront(nil)

        // Focus WebView
        window?.makeFirstResponder(webView)

        // Announce to VoiceOver
        announce("Focused web content area")

        // Focus first focusable element in web
        let script = """
            (function() {
                const focusable = document.querySelector(
                    'button, a, input, [tabindex="0"]'
                );
                if (focusable) focusable.focus();
            })();
            """
        webView.evaluateJavaScript(script)
    }

    private func focusToolbar() {
        if let toolbar = window?.toolbar,
           let toolbarView = window?.contentView?.subviews.first(where: {
               $0.accessibilityRole == .toolbar
           }) {
            focusView(toolbarView)
            announce("Toolbar focused. Press Tab to navigate.")
        }
    }

    private func focusStatusBar() {
        if let statusBar = window?.contentView?.subviews.first(where: {
            $0.accessibilityRole == .statusBar
        }) {
            focusView(statusBar)
            announce("Status bar focused")
        }
    }

    private func focusView(_ view: NSView, announce: Bool = true) {
        window?.makeFirstResponder(view)

        if announce {
            let label = view.accessibilityLabel ?? view.accessibilityRole?.rawValue ?? "Element"
            announce("Focused \(label)")
        }

        // Trigger focus ring update
        view.needsDisplay = true
    }

    private func focusWebElement(_ selector: String) {
        guard let webView = webView else { return }

        let script = """
            (function() {
                const element = document.querySelector('\(selector)');
                if (element) {
                    element.focus();
                    element.scrollIntoView({ behavior: 'smooth', block: 'center' });
                    return true;
                }
                return false;
            })();
            """

        webView.evaluateJavaScript(script) { result, error in
            if let error = error {
                print("Focus error: \(error)")
                return
            }

            if let success = result as? Bool, success {
                self.announce("Focused element in web view")
            } else {
                self.announceUrgent("Element not found: \(selector)")
            }
        }
    }

    // MARK: - Global Focus Handler

    private func setupGlobalFocusHandler() {
        // Listen for focus changes
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleFocusChanged),
            name: NSControl.textDidChangeNotification,
            object: nil
        )
    }

    @objc private func handleFocusChanged(_ notification: Notification) {
        // Track focus changes for restore functionality
        if let view = notification.object as? NSView {
            // Don't duplicate
            if focusHistory.last !== view {
                focusHistory.append(view)
                // Limit history size
                if focusHistory.count > 10 {
                    focusHistory.removeFirst()
                }
            }
        }
    }
}
```

---

## 2. Keyboard Navigation

### Tab Order Management

```swift
// KeyboardNavigationManager.swift
import Cocoa

class KeyboardNavigationManager {

    // MARK: - Types

    struct NavigationGroup {
        let identifier: String
        let views: [NSView]
        let wrapAround: Bool
    }

    // MARK: - Properties

    private weak var window: NSWindow?
    private var navigationGroups: [String: NavigationGroup] = [:]
    private var currentGroup: String?
    private var customTabOrder: [NSView] = []

    // MARK: - Tab Order Configuration

    func setupTabOrder(views: [NSView]) {
        customTabOrder = views

        // Set tab order using macOS API
        if let first = views.first {
            window?.initialFirstResponder = first
        }

        // Create tab group
        setupTabGroups()
    }

    private func setupTabGroups() {
        // Main application tab group
        let mainGroup = NavigationGroup(
            identifier: "main",
            views: customTabOrder,
            wrapAround: true
        )
        navigationGroups["main"] = mainGroup

        // VM list tab group (when sidebar focused)
        let sidebarGroup = NavigationGroup(
            identifier: "sidebar",
            views: [],  // Populated dynamically
            wrapAround: false
        )
        navigationGroups["sidebar"] = sidebarGroup

        // Settings tab group
        let settingsGroup = NavigationGroup(
            identifier: "settings",
            views: [],
            wrapAround: true
        )
        navigationGroups["settings"] = settingsGroup
    }

    // MARK: - Keyboard Shortcuts

    func registerShortcuts() {
        // Global shortcuts using NSEvent
        NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            return self.handleGlobalKeyEvent(event)
        }
    }

    private func handleGlobalKeyEvent(_ event: NSEvent) -> NSEvent? {
        // Command-based shortcuts
        if event.modifierFlags.contains(.command) {
            return handleCommandShortcut(event)
        }

        // Control-based navigation
        if event.modifierFlags.contains(.control) {
            return handleControlShortcut(event)
        }

        // Function keys
        if event.type == .keyDown && event.keyCode == 122 {  // F1
            return handleFunctionKey(event)
        }

        return event
    }

    private func handleCommandShortcut(_ event: NSEvent) -> NSEvent? {
        switch event.keyCode {
        case 46:  // Command+N - New VM
            postAccessibilityNotification("Creating new virtual machine")
            NotificationCenter.default.post(name: .newVMRequested, object: nil)
            return nil  // Consume event

        case 48:  // Command+O - Open/Import VM
            postAccessibilityNotification("Open VM dialog")
            NotificationCenter.default.post(name: .openVMRequested, object: nil)
            return nil

        case 3:   // Command+W - Close window
            // Standard behavior, don't consume
            return event

        case 45:  // Command+= - Zoom in
            NotificationCenter.default.post(name: .zoomInRequested, object: nil)
            return nil

        case 43:  // Command+- - Zoom out
            NotificationCenter.default.post(name: .zoomOutRequested, object: nil)
            return nil

        case 31:  // Command+0 - Reset zoom
            NotificationCenter.default.post(name: .zoomResetRequested, object: nil)
            return nil

        default:
            return event
        }
    }

    private func handleControlShortcut(_ event: NSEvent) -> NSEvent? {
        switch event.keyCode {
        case 48:  // Control+Tab - Next tab/panel
            moveToNextPanel()
            return nil

        case 49:  // Control+Shift+Tab - Previous tab/panel
            moveToPreviousPanel()
            return nil

        case 125:  // Control+Down - Focus main content
            focusMainContent()
            return nil

        default:
            return event
        }
    }

    // MARK: - Panel Navigation

    private func moveToNextPanel() {
        let panelOrder = ["toolbar", "sidebar", "mainContent", "statusBar"]

        guard let current = currentGroup,
              let currentIndex = panelOrder.firstIndex(of: current) else {
            currentGroup = panelOrder.first
            return
        }

        let nextIndex = (currentIndex + 1) % panelOrder.count
        currentGroup = panelOrder[nextIndex]
        focusPanel(panelOrder[nextIndex])
    }

    private func moveToPreviousPanel() {
        let panelOrder = ["toolbar", "sidebar", "mainContent", "statusBar"]

        guard let current = currentGroup,
              let currentIndex = panelOrder.firstIndex(of: current) else {
            currentGroup = panelOrder.last
            return
        }

        let previousIndex = currentIndex - 1 < 0 ? panelOrder.count - 1 : currentIndex - 1
        currentGroup = panelOrder[previousIndex]
        focusPanel(panelOrder[previousIndex])
    }

    private func focusPanel(_ panelName: String) {
        let announcement = "Focused \(panelName) panel"
        postAccessibilityNotification(announcement)

        // Find and focus the panel
        if let panel = findPanel(named: panelName) {
            window?.makeFirstResponder(panel)
        }
    }

    // MARK: - Helper Methods

    private func findPanel(named name: String) -> NSView? {
        // Implementation to find panel by name
        return nil
    }

    private func focusMainContent() {
        postAccessibilityNotification("Focused main content")
    }

    private func postAccessibilityNotification(_ message: String) {
        NSAccessibility.post(
            element: window ?? self,
            notification: .announcement,
            userInfo: [.announcementKey: message]
        )
    }
}

// Extension notifications
extension Notification.Name {
    static let newVMRequested = Notification.Name("newVMRequested")
    static let openVMRequested = Notification.Name("openVMRequested")
    static let zoomInRequested = Notification.Name("zoomInRequested")
    static let zoomOutRequested = Notification.Name("zoomOutRequested")
    static let zoomResetRequested = Notification.Name("zoomResetRequested")
}
```

### Focus Indicators

```swift
// FocusRingManager.swift
import Cocoa

class FocusRingManager {

    // MARK: - Custom Focus Ring Drawing

    static func applyEnhancedFocusRing(to view: NSView) {
        // Enable layer-backed view for custom focus ring
        view.wantsLayer = true

        // Store original border for restoration
        let originalBorder = view.layer?.borderWidth ?? 0

        // Create focus ring layer
        let focusRing = CALayer()
        focusRing.name = "focusRing"
        focusRing.borderColor = NSColor.keyboardFocusIndicatorColor.cgColor
        focusRing.borderWidth = 3
        focusRing.cornerRadius = view.layer?.cornerRadius ?? 0
        focusRing.opacity = 0  // Hidden by default

        // Insert as sublayer
        view.layer?.addSublayer(focusRing)

        // Monitor focus changes
        view.addObserver(
            self,
            forKeyPath: "window.firstResponder",
            options: [.new],
            context: nil
        )
    }

    // MARK: - High Visibility Mode

    static func applyHighVisibilityFocusRing(to view: NSView) {
        view.wantsLayer = true

        // Thicker, high-contrast focus ring
        view.layer?.borderWidth = 4
        view.layer?.borderColor = NSColor.systemYellow.cgColor  // High contrast color

        // Add glow effect
        view.layer?.shadowColor = NSColor.systemYellow.cgColor
        view.layer?.shadowRadius = 6
        view.layer?.shadowOpacity = 0.8
        view.layer?.shadowOffset = CGSize(width: 0, height: 0)
    }

    // MARK: - Focus Ring Removal

    static func removeFocusRing(from view: NSView) {
        view.layer?.sublayers?.removeAll { $0.name == "focusRing" }
        view.layer?.borderWidth = 0
        view.layer?.shadowOpacity = 0
    }
}

// Custom NSView subclass with built-in focus ring
class AccessibleView: NSView {

    private var focusRingLayer: CALayer?
    private var isFocused = false

    var usesHighVisibilityFocusRing = false

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        setupFocusRing()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setupFocusRing()
    }

    private func setupFocusRing() {
        wantsLayer = true

        // Create focus ring layer
        focusRingLayer = CALayer()
        focusRingLayer?.name = "focusRing"
        focusRingLayer?.borderColor = NSColor.keyboardFocusIndicatorColor.cgColor
        focusRingLayer?.borderWidth = 0  // Hidden initially
        focusRingLayer?.cornerRadius = layer?.cornerRadius ?? 0

        layer?.addSublayer(focusRingLayer!)
    }

    override var acceptsFirstResponder: Bool {
        return true
    }

    override func becomeFirstResponder() -> Bool {
        let result = super.becomeFirstResponder()
        if result {
            showFocusRing()
        }
        return result
    }

    override func resignFirstResponder() -> Bool {
        let result = super.resignFirstResponder()
        if result {
            hideFocusRing()
        }
        return result
    }

    private func showFocusRing() {
        isFocused = true
        updateFocusRingAppearance()
    }

    private func hideFocusRing() {
        isFocused = false
        updateFocusRingAppearance()
    }

    private func updateFocusRingAppearance() {
        guard let focusRing = focusRingLayer else { return }

        if isFocused {
            if usesHighVisibilityFocusRing {
                focusRing.borderWidth = 4
                focusRing.borderColor = NSColor.systemYellow.cgColor
                layer?.shadowColor = NSColor.systemYellow.cgColor
                layer?.shadowRadius = 6
                layer?.shadowOpacity = 0.8
            } else {
                focusRing.borderWidth = 3
                focusRing.borderColor = NSColor.keyboardFocusIndicatorColor.cgColor
                layer?.shadowOpacity = 0
            }
        } else {
            focusRing.borderWidth = 0
            layer?.shadowOpacity = 0
        }

        // Animate the change
        CATransaction.begin()
        CATransaction.setAnimationDuration(0.15)
        CATransaction.commit()
    }

    override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        // Update focus ring colors for dark/light mode
        if isFocused {
            updateFocusRingAppearance()
        }
    }
}
```

### Skip Links

Skip links allow keyboard users to bypass repetitive navigation and jump directly to main content.

```swift
// SkipLinkView.swift
import Cocoa

class SkipLinkView: NSView {

    private let skipButton: NSButton
    private var isHiddenByDefault = true

    // MARK: - Initialization

    override init(frame frameRect: NSRect) {
        skipButton = NSButton(title: "Skip to main content", target: nil, action: nil)
        skipButton.bezelStyle = .rounded

        super.init(frame: frameRect)

        setupSkipLink()
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    // MARK: - Setup

    private func setupSkipLink() {
        // Initially hidden, shown on Tab focus
        isHidden = isHiddenByDefault

        // High contrast styling
        wantsLayer = true
        layer?.backgroundColor = NSColor.windowBackgroundColor.cgColor
        layer?.cornerRadius = 4

        // Button styling
        skipButton.keyEquivalent = "\r"  // Enter to activate
        skipButton.setAccessibilityLabel("Skip to main content")
        skipButton.setAccessibilityHint("Press Enter to skip navigation and go to main content")

        // Add to view
        addSubview(skipButton)

        // Constraints
        skipButton.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            skipButton.topAnchor.constraint(equalTo: topAnchor, constant: 8),
            skipButton.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            skipButton.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            bottomAnchor.constraint(equalTo: skipButton.bottomAnchor, constant: 8)
        ])

        // Button action
        skipButton.target = self
        skipButton.action = #selector(skipToMainContent)
    }

    // MARK: - Keyboard Handling

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        return true  // Allow click without requiring prior focus
    }

    override func becomeFirstResponder() -> Bool {
        let result = super.becomeFirstResponder()
        if result && isHiddenByDefault {
            // Show skip link when focused via keyboard
            isHidden = false
            window?.makeFirstResponder(skipButton)
        }
        return result
    }

    @objc private func skipToMainContent() {
        // Notify the window/controller to move focus to main content
        NotificationCenter.default.post(
            name: .skipToMainContentRequested,
            object: self
        )

        // Announce to VoiceOver
        NSAccessibility.post(
            element: self,
            notification: .announcement,
            userInfo: [
                .announcementKey: "Skipped to main content",
                .announcementPriorityKey: NSAccessibility.Priority.high
            ]
        )

        // Hide after activation
        if isHiddenByDefault {
            isHidden = true
        }
    }

    // MARK: - Visibility

    func setAutoHide(_ autoHide: Bool) {
        isHiddenByDefault = autoHide
        if autoHide {
            isHidden = true
        }
    }
}

extension Notification.Name {
    static let skipToMainContentRequested = Notification.Name("skipToMainContentRequested")
}
```

### WebView Keyboard Handling

```typescript
// keyboard-navigation.ts
/**
 * Keyboard navigation utilities for WebView content
 */

export class KeyboardNavigation {
    private focusTrap: FocusTrap | null = null;
    private skipLinks: Map<string, string> = new Map();

    /**
     * Initialize keyboard navigation
     */
    initialize(): void {
        this.setupSkipLinks();
        this.setupArrowKeyNavigation();
        this.setupFocusIndicators();
        this.setupKeyboardShortcuts();
    }

    /**
     * Setup skip link functionality
     */
    private setupSkipLinks(): void {
        // Register skip link targets
        this.skipLinks.set('main', '#main-content');
        this.skipLinks.set('sidebar', '#sidebar');
        this.skipLinks.set('search', '#search-input');

        // Create skip link container (visually hidden but accessible)
        const skipContainer = document.createElement('div');
        skipContainer.className = 'skip-links';
        skipContainer.setAttribute('role', 'navigation');
        skipContainer.setAttribute('aria-label', 'Skip links');

        this.skipLinks.forEach((target, label) => {
            const link = document.createElement('a');
            link.href = target;
            link.textContent = `Skip to ${label}`;
            link.className = 'skip-link';
            link.addEventListener('click', (e) => this.handleSkipLink(e, target));
            skipContainer.appendChild(link);
        });

        // Insert at beginning of body
        document.body.insertBefore(skipContainer, document.body.firstChild);
    }

    /**
     * Handle skip link activation
     */
    private handleSkipLink(event: Event, target: string): void {
        event.preventDefault();

        const element = document.querySelector(target);
        if (!element) return;

        // Make element focusable if needed
        if (!element.hasAttribute('tabindex')) {
            element.setAttribute('tabindex', '-1');
        }

        // Move focus
        element.focus();

        // Announce to screen readers
        this.announce(`Skipped to ${target.replace('#', '').replace('-', ' ')}`);

        // Send focus info to native
        this.notifyNativeFocus(target);
    }

    /**
     * Setup arrow key navigation for lists and grids
     */
    private setupArrowKeyNavigation(): void {
        document.addEventListener('keydown', (event: KeyboardEvent) => {
            // Only handle arrow keys
            if (!['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(event.key)) {
                return;
            }

            const target = event.target as HTMLElement;

            // Check if in a list/grid context
            const listRole = target.closest('[role="list"], [role="listbox"], [role="grid"]');
            if (listRole) {
                this.handleArrowNavigation(event, target, listRole);
            }
        });
    }

    /**
     * Handle arrow key navigation within lists
     */
    private handleArrowNavigation(
        event: KeyboardEvent,
        current: HTMLElement,
        container: Element
    ): void {
        const isVertical = event.key === 'ArrowUp' || event.key === 'ArrowDown';
        const isReverse = event.key === 'ArrowUp' || event.key === 'ArrowLeft';

        // Get all focusable items in container
        const items = Array.from(
            container.querySelectorAll('[role="listitem"], [role="row"], [role="option"]')
        );

        const currentIndex = items.indexOf(current);
        if (currentIndex === -1) return;

        // Calculate new index
        let newIndex = currentIndex + (isReverse ? -1 : 1);

        // Wrap around
        if (newIndex < 0) newIndex = items.length - 1;
        if (newIndex >= items.length) newIndex = 0;

        // Focus new item
        const newItem = items[newIndex] as HTMLElement;
        newItem.focus();

        // Prevent default scrolling
        event.preventDefault();

        // Announce item
        this.announceItem(newItem, newIndex + 1, items.length);
    }

    /**
     * Announce list item to screen readers
     */
    private announceItem(item: HTMLElement, index: number, total: number): void {
        const label = item.getAttribute('aria-label') || item.textContent?.trim() || '';
        const selected = item.getAttribute('aria-selected') === 'true' ? 'selected' : '';

        this.announce(`${label}, ${index} of ${total}${selected ? `, ${selected}` : ''}`);
    }

    /**
     * Setup focus indicators for keyboard users
     */
    private setupFocusIndicators(): void {
        // Add class to body when using keyboard navigation
        let usingKeyboard = false;

        document.addEventListener('keydown', (e) => {
            if (e.key === 'Tab') {
                usingKeyboard = true;
                document.body.classList.add('keyboard-navigation');
            }
        });

        document.addEventListener('mousedown', () => {
            usingKeyboard = false;
            document.body.classList.remove('keyboard-navigation');
        });
    }

    /**
     * Setup application keyboard shortcuts
     */
    private setupKeyboardShortcuts(): void {
        document.addEventListener('keydown', (event: KeyboardEvent) => {
            // Alt-based shortcuts (cross-platform)
            if (event.altKey) {
                switch (event.key) {
                    case 'n':
                        event.preventDefault();
                        this.triggerAction('new-vm');
                        break;
                    case 'o':
                        event.preventDefault();
                        this.triggerAction('open-vm');
                        break;
                    case 's':
                        event.preventDefault();
                        this.triggerAction('save');
                        break;
                    case 'f':
                        event.preventDefault();
                        this.triggerAction('search');
                        break;
                }
            }

            // Escape key - close dialogs, clear focus
            if (event.key === 'Escape') {
                this.handleEscapeKey();
            }

            // Enter key in interactive elements
            if (event.key === 'Enter' && event.target instanceof HTMLElement) {
                const interactive = event.target.closest('button, a, [role="button"]');
                if (interactive) {
                    // Visual feedback
                    this.triggerAction('activate', { element: interactive });
                }
            }
        });
    }

    /**
     * Handle Escape key
     */
    private handleEscapeKey(): void {
        // Close any open dialogs/modals
        const modal = document.querySelector('[role="dialog"], [role="alertdialog"]');
        if (modal) {
            this.triggerAction('close-dialog');
            return;
        }

        // Clear search/filter
        const searchInput = document.querySelector('input[type="search"]');
        if (searchInput && searchInput.value) {
            searchInput.value = '';
            searchInput.dispatchEvent(new Event('input'));
            this.announce('Search cleared');
            return;
        }

        // Return focus to main content
        const main = document.querySelector('main, [role="main"]');
        if (main) {
            (main as HTMLElement).focus();
            this.announce('Returned to main content');
        }
    }

    /**
     * Trigger application action
     */
    private triggerAction(action: string, data?: any): void {
        // Dispatch custom event for application to handle
        const event = new CustomEvent('app-action', {
            detail: { action, data }
        });
        document.dispatchEvent(event);

        // Also notify native via Strada/bridge if available
        if (window.strada) {
            window.strada.send({
                component: 'keyboard',
                event: 'action',
                data: { action, data }
            });
        }
    }

    /**
     * Announce message to screen readers
     */
    private announce(message: string, priority: 'polite' | 'assertive' = 'polite'): void {
        let announcer = document.getElementById('sr-announcer');

        if (!announcer) {
            announcer = document.createElement('div');
            announcer.id = 'sr-announcer';
            announcer.setAttribute('role', 'status');
            announcer.setAttribute('aria-live', priority);
            announcer.setAttribute('aria-atomic', 'true');
            announcer.className = 'sr-only';
            document.body.appendChild(announcer);
        }

        // Update priority if changed
        announcer.setAttribute('aria-live', priority);

        // Clear and set message (triggers announcement)
        announcer.textContent = '';
        setTimeout(() => {
            announcer.textContent = message;
        }, 100);
    }

    /**
     * Notify native of focus change
     */
    private notifyNativeFocus(selector: string): void {
        if (window.strada) {
            window.strada.send({
                component: 'focus',
                event: 'changed',
                data: { element: selector }
            });
        }
    }
}

// Initialize on DOM ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        new KeyboardNavigation().initialize();
    });
} else {
    new KeyboardNavigation().initialize();
}
```

---

## 3. Screen Reader Compatibility

### ARIA Labels in WebView

```html
<!-- VM List Item with full ARIA support -->
<article class="vm-item"
         role="listitem"
         aria-labelledby="vm-name-{{vm.id}}"
         aria-describedby="vm-status-{{vm.id}} vm-specs-{{vm.id}}"
         tabindex="0"
         data-vm-id="{{vm.id}}"
         data-htmx-on-click="vm-select">

    <div class="vm-status"
         role="img"
         aria-label="{{ vm.status.label }}">
        <span class="status-indicator status-{{vm.status.class}}"></span>
    </div>

    <div class="vm-info">
        <h3 id="vm-name-{{vm.id}}" class="vm-name">{{vm.name}}</h3>
        <p id="vm-status-{{vm.id}}" class="vm-status-text">
            Status: {{vm.status.label}}
        </p>
        <p id="vm-specs-{{vm.id}}" class="vm-specs">
            {{vm.cpu_count}} CPUs, {{vm.memory_gb}}GB RAM
        </p>
    </div>

    <div class="vm-actions">
        <button class="vm-action-btn"
                aria-label="{{ vm.power_button_label }} for {{vm.name}}"
                aria-pressed="{{vm.is_running}}"
                data-action="power"
                data-vm-id="{{vm.id}}">
            <span class="btn-icon" aria-hidden="true"></span>
        </button>

        <button class="vm-action-btn"
                aria-label="Settings for {{vm.name}}"
                data-action="settings"
                data-vm-id="{{vm.id}}">
            <span class="btn-icon" aria-hidden="true"></span>
        </button>

        <button class="vm-action-btn"
                aria-label="Delete {{vm.name}}"
                data-action="delete"
                data-vm-id="{{vm.id}}">
            <span class="btn-icon" aria-hidden="true"></span>
        </button>
    </div>
</article>

<!-- VM Console with ARIA live regions -->
<div class="vm-console-container"
     role="region"
     aria-label="Virtual machine console"
     aria-live="polite">

    <!-- Console output area -->
    <div class="vm-console-output"
         role="log"
         aria-live="off"
         aria-relevant="additions"
         aria-atomic="false"
         id="console-output">
        <!-- Console lines inserted here -->
    </div>

    <!-- Status announcements -->
    <div class="vm-console-status"
         role="status"
         aria-live="polite"
         aria-atomic="true"
         id="console-status"></div>

    <!-- Control buttons -->
    <div class="vm-console-controls"
         role="toolbar"
         aria-label="Console controls">

        <button type="button"
                aria-label="Send Ctrl+Alt+Delete to VM"
                aria-keyshortcuts="Control+Alt+Delete"
                data-action="send-keys"
                data-keys="ctrl-alt-del">
            Send Ctrl+Alt+Delete
        </button>

        <button type="button"
                aria-label="Capture keyboard input"
                aria-pressed="false"
                data-action="capture-keyboard">
            Capture Keyboard
        </button>

        <button type="button"
                aria-label="Release keyboard capture"
                data-action="release-keyboard">
            Release Keyboard
        </button>

        <button type="button"
                aria-label="Toggle fullscreen"
                aria-keyshortcuts="F11"
                data-action="toggle-fullscreen">
            Fullscreen
        </button>
    </div>
</div>

<!-- Screen reader only announcer -->
<div id="sr-announcer"
     role="status"
     aria-live="polite"
     aria-atomic="true"
     class="sr-only"></div>
```

### Live Regions for Dynamic Content

```typescript
// live-regions.ts
/**
 * ARIA Live Region management for dynamic content announcements
 */

export type LiveRegionPriority = 'polite' | 'assertive' | 'off';

export interface LiveRegionConfig {
    id: string;
    priority: LiveRegionPriority;
    atomic?: boolean;
    relevant?: 'additions' | 'removals' | 'text' | 'all';
}

export class LiveRegionManager {
    private regions: Map<string, LiveRegionConfig> = new Map();
    private queue: Array<{ id: string; message: string; priority: LiveRegionPriority }> = [];
    private isAnnouncing = false;

    /**
     * Create a live region
     */
    create(config: LiveRegionConfig): HTMLElement {
        // Remove existing if present
        this.destroy(config.id);

        // Create element
        const element = document.createElement('div');
        element.id = config.id;
        element.setAttribute('role', 'status');
        element.setAttribute('aria-live', config.priority);
        element.setAttribute('aria-atomic', String(config.atomic ?? true));

        if (config.relevant) {
            element.setAttribute('aria-relevant', config.relevant);
        }

        element.className = 'sr-only';
        document.body.appendChild(element);

        // Store config
        this.regions.set(config.id, config);

        return element;
    }

    /**
     * Destroy a live region
     */
    destroy(id: string): void {
        const element = document.getElementById(id);
        if (element) {
            element.remove();
        }
        this.regions.delete(id);
    }

    /**
     * Announce a message to a live region
     */
    announce(id: string, message: string, priority?: LiveRegionPriority): void {
        const config = this.regions.get(id);
        if (!config) {
            console.warn(`Live region "${id}" not found`);
            return;
        }

        // Use provided priority or default to config
        const effectivePriority = priority ?? config.priority;

        // Queue the announcement
        this.queue.push({ id, message, priority: effectivePriority });

        // Process queue
        this.processQueue();
    }

    /**
     * Process announcement queue
     */
    private processQueue(): void {
        if (this.isAnnouncing || this.queue.length === 0) {
            return;
        }

        this.isAnnouncing = true;
        const announcement = this.queue.shift()!;

        const element = document.getElementById(announcement.id);
        if (!element) {
            this.isAnnouncing = false;
            this.processQueue();
            return;
        }

        // Update priority if changed
        if (announcement.priority !== 'off') {
            element.setAttribute('aria-live', announcement.priority);
        }

        // Clear and set message (triggers announcement)
        element.textContent = '';

        // Use setTimeout to ensure the clear is processed
        setTimeout(() => {
            element.textContent = announcement.message;

            // Allow time for announcement, then process next
            setTimeout(() => {
                this.isAnnouncing = false;
                this.processQueue();
            }, 1000);
        }, 50);
    }

    /**
     * Announce VM status change
     */
    announceVMStatus(vmName: string, status: string): void {
        this.announce(
            'vm-status-announcer',
            `${vmName} is now ${status}`,
            'assertive'
        );
    }

    /**
     * Announce VM operation progress
     */
    announceVMProgress(vmName: string, operation: string, progress: number): void {
        const message = `${vmName} ${operation}: ${Math.round(progress)}% complete`;
        this.announce('vm-progress-announcer', message, 'polite');
    }

    /**
     * Announce error
     */
    announceError(message: string): void {
        this.announce('error-announcer', `Error: ${message}`, 'assertive');
    }

    /**
     * Initialize standard live regions
     */
    initializeStandardRegions(): void {
        // VM status announcements
        this.create({
            id: 'vm-status-announcer',
            priority: 'assertive',
            atomic: true
        });

        // VM progress updates
        this.create({
            id: 'vm-progress-announcer',
            priority: 'polite',
            atomic: true,
            relevant: 'text'
        });

        // Error announcements
        this.create({
            id: 'error-announcer',
            priority: 'assertive',
            atomic: true
        });

        // General notifications
        this.create({
            id: 'notification-announcer',
            priority: 'polite',
            atomic: true
        });
    }
}

// Export singleton instance
export const liveRegions = new LiveRegionManager();
```

### HTMX Integration with Live Regions

```html
<!-- VM List with HTMX and accessibility -->
<div id="vm-list"
     role="list"
     aria-label="Virtual machines"
     hx-get="/api/vms"
     hx-trigger="load, every 30s"
     hx-swap="innerHTML"
     hx-on::after-swap="handleVMSwap(event)">

    <!-- VM items rendered here -->

    <!-- Loading state -->
    <div class="vm-list-loading"
         role="status"
         aria-live="polite"
         hx-indicator=".vm-list-loading">
        <span aria-hidden="true">Loading...</span>
        <span class="sr-only">Loading virtual machine list</span>
    </div>
</div>

<script>
    function handleVMSwap(event) {
        // Announce update to screen readers
        const vmCount = document.querySelectorAll('.vm-item').length;
        liveRegions.announce(
            'notification-announcer',
            `Virtual machine list updated. ${vmCount} virtual machines.`,
            'polite'
        );

        // Preserve focus if an item was focused
        const focusedBefore = event.detail.elt.querySelector(':focus');
        if (focusedBefore) {
            const sameElement = document.getElementById(focusedBefore.id);
            if (sameElement) {
                sameElement.focus();
            }
        }
    }
</script>
```

### Screen Reader Announcements from Native

```swift
// ScreenReaderAnnouncer.swift
import Cocoa

class ScreenReaderAnnouncer {

    // MARK: - Properties

    private weak var window: NSWindow?
    private var webView: WKWebView?

    // MARK: - Initialization

    init(window: NSWindow, webView: WKWebView) {
        self.window = window
        self.webView = webView
    }

    // MARK: - Announcements

    /// Announce a message to screen readers
    func announce(_ message: String, priority: NSAccessibility.Priority = .low) {
        NSAccessibility.post(
            element: window ?? self,
            notification: .announcement,
            userInfo: [
                .announcementKey: message,
                .announcementPriorityKey: priority
            ]
        )
    }

    /// Announce VM state change
    func announceVMStateChange(vmName: String, oldState: VMState, newState: VMState) {
        let message = "\(vmName): \(stateDescription(oldState)) to \(stateDescription(newState))"
        announce(message, priority: .high)

        // Also notify WebView for coordinated announcement
        notifyWebViewOfAnnouncement(message)
    }

    /// Announce operation progress
    func announceProgress(operation: String, current: Int, total: Int) {
        let percentage = total > 0 ? (current * 100 / total) : 0
        let message = "\(operation): \(percentage)% complete"
        announce(message, priority: .low)
    }

    /// Announce error
    func announceError(_ error: String) {
        announce("Error: \(error)", priority: .high)
    }

    /// Announce successful action
    func announceSuccess(_ message: String) {
        announce("Success: \(message)", priority: .medium)
    }

    // MARK: - Focus Announcements

    /// Announce when focus moves to a new section
    func announceFocusMove(to section: String) {
        announce("Now in \(section) section", priority: .medium)
    }

    /// Announce modal dialog appearance
    func announceModal(_ title: String) {
        announce("Dialog opened: \(title). Press Escape to close.", priority: .high)
    }

    /// Announce modal dismissal
    func announceModalDismissed() {
        announce("Dialog closed", priority: .medium)
    }

    // MARK: - WebView Coordination

    private func notifyWebViewOfAnnouncement(_ message: String) {
        let script = """
            if (window.announceToScreenReader) {
                window.announceToScreenReader(\(message.quotedString));
            }
            """

        webView?.evaluateJavaScript(script)
    }

    // MARK: - Helper Methods

    private func stateDescription(_ state: VMState) -> String {
        switch state {
        case .running: return "running"
        case .paused: return "paused"
        case .stopped: return "stopped"
        case .starting: return "starting"
        case .stopping: return "stopping"
        case .error: return "in error state"
        }
    }
}

extension String {
    var quotedString: String {
        let escaped = self
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
            .replacingOccurrences(of: "\n", with: "\\n")
            .replacingOccurrences(of: "\r", with: "\\r")
            .replacingOccurrences(of: "\t", with: "\\t")
        return "\"\(escaped)\""
    }
}
```

---

## 4. System Accessibility Settings

### High Contrast Mode Support

```typescript
// high-contrast.ts
/**
 * High contrast mode detection and support
 */

export class HighContrastManager {
    private isHighContrast = false;
    private mediaQuery: MediaQueryList;
    private listeners: Set<() => void> = new Set();

    constructor() {
        // Detect forced colors (Windows High Contrast, macOS Increased Contrast)
        this.mediaQuery = window.matchMedia('(forced-colors: active)');

        // Also check for high contrast preference
        this.isHighContrast = this.detectHighContrast();

        // Listen for changes
        this.mediaQuery.addEventListener('change', () => this.handleContrastChange());
    }

    private detectHighContrast(): boolean {
        // Check forced colors
        if (this.mediaQuery.matches) {
            return true;
        }

        // Check for increased contrast preference
        const increasedContrast = window.matchMedia('(prefers-contrast: more)');
        if (increasedContrast.matches) {
            return true;
        }

        // Fallback: check computed styles
        const testEl = document.createElement('div');
        testEl.style.position = 'absolute';
        testEl.style.width = '1px';
        testEl.style.height = '1px';
        testEl.style.border = '10px solid red';
        document.body.appendChild(testEl);

        const computedStyle = window.getComputedStyle(testEl);
        const border = computedStyle.borderTopColor;
        document.body.removeChild(testEl);

        // In high contrast mode, colors may be overridden
        return border === 'rgb(0, 0, 0)' || border === 'rgb(255, 255, 255)';
    }

    private handleContrastChange(): void {
        const wasHighContrast = this.isHighContrast;
        this.isHighContrast = this.detectHighContrast();

        if (wasHighContrast !== this.isHighContrast) {
            this.notifyListeners();
        }
    }

    /**
     * Check if high contrast is active
     */
    isActive(): boolean {
        return this.isHighContrast;
    }

    /**
     * Subscribe to contrast changes
     */
    subscribe(callback: () => void): () => void {
        this.listeners.add(callback);
        return () => this.listeners.delete(callback);
    }

    private notifyListeners(): void {
        this.listeners.forEach(callback => callback());

        // Update body class
        if (this.isHighContrast) {
            document.body.classList.add('high-contrast');
        } else {
            document.body.classList.remove('high-contrast');
        }
    }
}

// Export singleton
export const highContrast = new HighContrastManager();
```

```css
/* high-contrast.css */

/* High contrast mode overrides */
@media (forced-colors: active) {
    /* Ensure all text is visible */
    body {
        forced-color-adjust: auto;
    }

    /* Buttons and interactive elements */
    button,
    [role="button"],
    a {
        border: 2px solid currentColor;
    }

    /* Focus indicators */
    :focus {
        outline: 3px solid Highlight;
        outline-offset: 2px;
    }

    /* Status indicators need visible borders */
    .status-indicator {
        border: 2px solid currentColor;
    }

    /* VM list items */
    .vm-item {
        border: 1px solid currentColor;
    }

    .vm-item:focus,
    .vm-item:focus-within {
        outline: 3px solid Highlight;
        outline-offset: -1px;
    }

    /* Disabled states */
    [disabled],
    [aria-disabled="true"] {
        opacity: 0.5;
    }
}

/* Prefers more contrast */
@media (prefers-contrast: more) {
    /* Increase border contrast */
    .vm-item {
        border-width: 2px;
    }

    /* Darken text */
    body {
        color: #000;
        background: #fff;
    }

    /* Ensure links are distinct */
    a {
        text-decoration: underline;
        font-weight: bold;
    }
}

/* Manual high contrast class (for user toggle) */
body.high-contrast {
    --color-text: #000000;
    --color-background: #ffffff;
    --color-border: #000000;
    --color-focus: #0066cc;

    color: var(--color-text);
    background: var(--color-background);
}

body.high-contrast button,
body.high-contrast [role="button"] {
    border: 2px solid var(--color-border);
    background: var(--color-background);
    color: var(--color-text);
}

body.high-contrast :focus {
    outline: 3px solid var(--color-focus);
    outline-offset: 2px;
}
```

### Reduced Motion Preferences

```typescript
// reduced-motion.ts
/**
 * Reduced motion preference handling
 */

export class ReducedMotionManager {
    private prefersReducedMotion = false;
    private mediaQuery: MediaQueryList;

    constructor() {
        this.mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
        this.prefersReducedMotion = this.mediaQuery.matches;

        this.mediaQuery.addEventListener('change', () => {
            this.prefersReducedMotion = this.mediaQuery.matches;
            this.updateMotionSettings();
        });

        this.updateMotionSettings();
    }

    /**
     * Check if reduced motion is preferred
     */
    isReducedMotionPreferred(): boolean {
        return this.prefersReducedMotion;
    }

    /**
     * Update motion settings based on preference
     */
    private updateMotionSettings(): void {
        if (this.prefersReducedMotion) {
            document.documentElement.classList.add('reduced-motion');
            document.documentElement.style.setProperty('--transition-duration', '0ms');
        } else {
            document.documentElement.classList.remove('reduced-motion');
            document.documentElement.style.setProperty('--transition-duration', '200ms');
        }
    }

    /**
     * Animate element respecting motion preference
     */
    animate(element: HTMLElement, animation: () => void, duration: number = 200): void {
        if (this.prefersReducedMotion) {
            // Instant change
            animation();
        } else {
            // Normal animation
            element.style.transition = `all ${duration}ms ease`;
            animation();

            // Clean up transition after completion
            setTimeout(() => {
                element.style.transition = '';
            }, duration);
        }
    }

    /**
     * Create a transition that respects motion preference
     */
    createTransition(property: string, duration: number = 200): string {
        return this.prefersReducedMotion ? 'none' : `${property} ${duration}ms ease`;
    }
}

export const reducedMotion = new ReducedMotionManager();
```

```css
/* reduced-motion.css */

/* Reduced motion preferences */
@media (prefers-reduced-motion: reduce) {
    /* Disable animations */
    *,
    *::before,
    *::after {
        animation-duration: 0.01ms !important;
        animation-iteration-count: 1 !important;
        transition-duration: 0.01ms !important;
    }

    /* Keep essential transitions very short */
    :focus {
        transition: outline-offset 0.1s ease;
    }

    /* Disable parallax and scroll effects */
    .parallax {
        transform: none !important;
    }

    /* Disable loading spinners */
    .spinner {
        animation: none !important;
        background-image: url('data:image/svg+xml,...'); /* Static fallback */
    }
}

/* Custom property for transitions */
:root {
    --transition-duration: 200ms;
    --transition-timing: ease;
}

/* Apply transition using custom property */
.reduced-motion {
    --transition-duration: 0ms;
}

/* VM Console - reduce motion */
.reduced-motion .vm-console-output {
    scroll-behavior: auto;
}

/* Loading states - static fallback */
.reduced-motion .loading {
    animation: none;
    background: #f0f0f0;
}

.reduced-motion .loading::after {
    content: 'Loading...';
    animation: none;
}
```

### Font Scaling

```swift
// FontScalingManager.swift
import Cocoa

class FontScalingManager {

    // MARK: - Properties

    private var baseFontSize: CGFloat = 13.0
    private var currentScale: CGFloat = 1.0
    private var minScale: CGFloat = 0.8
    private var maxScale: CGFloat = 2.0

    // MARK: - Initialization

    init() {
        setupFontSizeObserver()
        applySystemFontScale()
    }

    // MARK: - Public API

    /// Get current scaled font size
    var scaledFontSize: CGFloat {
        return baseFontSize * currentScale
    }

    /// Increase font size
    func increaseFontSize() {
        let newScale = min(maxScale, currentScale + 0.1)
        setFontScale(newScale)
    }

    /// Decrease font size
    func decreaseFontSize() {
        let newScale = max(minScale, currentScale - 0.1)
        setFontScale(newScale)
    }

    /// Reset to default font size
    func resetFontSize() {
        setFontScale(1.0)
    }

    /// Set specific font scale
    func setFontScale(_ scale: CGFloat) {
        let clampedScale = max(minScale, min(maxScale, scale))
        currentScale = clampedScale

        // Notify observers
        NotificationCenter.default.post(
            name: .fontScaleDidChange,
            object: self,
            userInfo: ["scale": clampedScale]
        )
    }

    // MARK: - System Font Scale

    private func setupFontSizeObserver() {
        // Listen for system font size changes
        DistributedNotificationCenter.default.addObserver(
            self,
            selector: #selector(applySystemFontScale),
            name: NSNotification.Name("NSAccessibilityUpdatePreferences"),
            object: nil
        )
    }

    @objc private func applySystemFontScale() {
        // Get system font scale from user defaults
        if let fontSize = UserDefaults.standard.object(forKey: "NSFontSize") as? CGFloat {
            let scale = fontSize / baseFontSize
            setFontScale(scale)
        }
    }

    // MARK: - Font Creation

    /// Create a font with current scaling applied
    func scaledFont(for style: NSFont.TextStyle, weight: NSFont.Weight = .regular) -> NSFont {
        let descriptor = NSFontDescriptor.preferredFontDescriptor(forTextStyle: style)
        let pointSize = descriptor.pointSize * currentScale

        return NSFont(
            systemFontSize: pointSize,
            weight: weight
        ) ?? NSFont.systemFont(ofSize: pointSize, weight: weight)
    }

    /// Create a monospaced font with scaling (for console)
    func scaledMonospacedFont(ofSize size: CGFloat) -> NSFont {
        return NSFont.monospacedSystemFont(ofSize: size * currentScale, weight: .regular)
    }
}

// MARK: - Notification

extension Notification.Name {
    static let fontScaleDidChange = Notification.Name("fontScaleDidChange")
}
```

```typescript
// Font scaling in WebView
export class WebViewFontScaling {
    private currentScale: number = 1.0;
    private baseFontSize: number = 16;

    constructor() {
        this.setupFontScaleListener();
        this.applySystemFontScale();
    }

    private setupFontScaleListener(): void {
        // Listen for font scale messages from native
        document.addEventListener('font-scale-change', (event: CustomEvent) => {
            this.setFontScale(event.detail.scale);
        });

        // Listen for system preference changes
        window.matchMedia('(prefers-reduced-motion: reduce)').addEventListener('change', () => {
            this.applySystemFontScale();
        });
    }

    private applySystemFontScale(): void {
        // Check for user's font size preference
        const htmlStyle = getComputedStyle(document.documentElement);
        const rootFontSize = parseFloat(htmlStyle.fontSize);

        if (rootFontSize !== this.baseFontSize) {
            this.currentScale = rootFontSize / this.baseFontSize;
        }
    }

    private setFontScale(scale: number): void {
        this.currentScale = Math.max(0.8, Math.min(2.0, scale));
        document.documentElement.style.setProperty('--font-scale', String(this.currentScale));
    }

    getScale(): number {
        return this.currentScale;
    }
}
```

---

## 5. Focus Management

### WebView Focus Coordination

```swift
// WebViewFocusCoordinator.swift
import Cocoa
import WebKit

class WebViewFocusCoordinator {

    // MARK: - Properties

    private weak var webView: WKWebView?
    private weak var window: NSWindow?
    private var isWebViewFocused = false
    private var previousNativeFocus: NSView?

    // MARK: - Initialization

    init(webView: WKWebView, window: NSWindow) {
        self.webView = webView
        self.window = window
        setupFocusObservers()
    }

    // MARK: - Focus Coordination

    /// Transfer focus from native to WebView
    func focusWebView() {
        // Save current native focus
        previousNativeFocus = window?.firstResponder as? NSView

        // Focus WebView
        window?.makeFirstResponder(webView)
        isWebViewFocused = true

        // Focus first interactive element in web
        focusFirstWebElement()

        // Announce
        NSAccessibility.post(
            element: webView,
            notification: .focusedUIElementChanged,
            userInfo: nil
        )
    }

    /// Return focus from WebView to native
    func returnFocusToNative() {
        isWebViewFocused = false

        // Restore previous native focus or default
        if let previous = previousNativeFocus {
            window?.makeFirstResponder(previous)
        } else {
            // Default to main content area
            window?.makeFirstResponder(webView?.superview)
        }

        // Announce
        NSAccessibility.post(
            element: window ?? self,
            notification: .focusedUIElementChanged,
            userInfo: nil
        )
    }

    /// Move focus to specific web element
    func focusWebElement(selector: String) {
        let js = """
            (function() {
                var element = document.querySelector('\(selector)');
                if (element) {
                    if (!element.hasAttribute('tabindex')) {
                        element.setAttribute('tabindex', '-1');
                    }
                    element.focus();
                    element.scrollIntoView({ block: 'center' });
                    return true;
                }
                return false;
            })();
            """

        webView?.evaluateJavaScript(js) { result, error in
            if let success = result as? Bool, success {
                self.isWebViewFocused = true
            } else {
                print("Failed to focus element: \(selector)")
            }
        }
    }

    // MARK: - Focus Trapping for Modals

    /// Setup focus trap for modal dialog in WebView
    func setupModalFocusTrap(modalSelector: String, closeCallback: @escaping () -> Void) {
        let js = """
            (function() {
                var modal = document.querySelector('\(modalSelector)');
                if (!modal) return false;

                // Get focusable elements
                var focusable = modal.querySelectorAll(
                    'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
                );
                var firstFocusable = focusable[0];
                var lastFocusable = focusable[focusable.length - 1];

                // Trap focus
                modal.addEventListener('keydown', function(e) {
                    if (e.key !== 'Tab') return;

                    if (e.shiftKey) {
                        if (document.activeElement === firstFocusable) {
                            e.preventDefault();
                            lastFocusable.focus();
                        }
                    } else {
                        if (document.activeElement === lastFocusable) {
                            e.preventDefault();
                            firstFocusable.focus();
                        }
                    }
                });

                // Handle Escape
                modal.addEventListener('keydown', function(e) {
                    if (e.key === 'Escape') {
                        window.dispatchEvent(new CustomEvent('modal-close-requested'));
                    }
                });

                // Focus first element
                firstFocusable.focus();

                return true;
            })();
            """

        webView?.evaluateJavaScript(js)
    }

    /// Release focus trap
    func releaseFocusTrap() {
        let js = """
            (function() {
                var modal = document.querySelector('[role="dialog"], [role="alertdialog"]');
                if (modal) {
                    modal.classList.remove('focus-trap-active');
                }
            })();
            """

        webView?.evaluateJavaScript(js)
    }

    // MARK: - Focus Restoration

    /// Save focus state for restoration
    func saveFocusState() {
        let js = """
            (function() {
                var active = document.activeElement;
                if (active && active !== document.body) {
                    return active.id || active.tagName;
                }
                return null;
            })();
            """

        webView?.evaluateJavaScript(js) { result, error in
            if let elementId = result as? String {
                UserDefaults.standard.set(elementId, forKey: "WebViewLastFocusedElement")
            }
        }
    }

    /// Restore focus state
    func restoreFocusState() {
        guard let elementId = UserDefaults.standard.string(forKey: "WebViewLastFocusedElement") else {
            return
        }

        let js = """
            (function() {
                var element = document.getElementById('\(elementId)') ||
                              document.querySelector('\(elementId)');
                if (element) {
                    element.focus();
                    return true;
                }
                return false;
            })();
            """

        webView?.evaluateJavaScript(js)
    }

    // MARK: - Private Methods

    private func setupFocusObservers() {
        // Listen for focus changes within WebView
        let script = """
            document.addEventListener('focusin', function(event) {
                if (window.webkit && window.webkit.messageHandlers) {
                    window.webkit.messageHandlers.focusChanged.postMessage({
                        element: event.target.tagName,
                        id: event.target.id,
                        class: event.target.className
                    });
                }
            });
            """

        // Note: Requires WKScriptMessageHandler setup
    }

    private func focusFirstWebElement() {
        let js = """
            (function() {
                var firstFocusable = document.querySelector(
                    'button, a, input, select, [tabindex]:not([tabindex="-1"])'
                );
                if (firstFocusable) {
                    firstFocusable.focus();
                }
            })();
            """

        webView?.evaluateJavaScript(js)
    }
}
```

### Modal Dialog Focus Trapping

```typescript
// focus-trap.ts
/**
 * Focus trap utility for modal dialogs
 */

export interface FocusTrapOptions {
    initialFocus?: string | HTMLElement;
    escapeDeactivates?: boolean;
    allowOutsideClick?: boolean;
    returnFocusOnDeactivate?: boolean;
    setReturnFocus?: () => HTMLElement | string;
}

export class FocusTrap {
    private container: HTMLElement;
    private options: FocusTrapOptions;
    private previouslyFocused: HTMLElement | null = null;
    private focusableElements: HTMLElement[] = [];
    private firstFocusable: HTMLElement | null = null;
    private lastFocusable: HTMLElement | null = null;
    private isActive = false;

    constructor(container: HTMLElement | string, options: FocusTrapOptions = {}) {
        this.container = typeof container === 'string'
            ? document.querySelector(container)!
            : container;

        this.options = {
            escapeDeactivates: true,
            allowOutsideClick: false,
            returnFocusOnDeactivate: true,
            ...options
        };
    }

    /**
     * Activate the focus trap
     */
    activate(): void {
        if (this.isActive) return;

        // Store previously focused element
        this.previouslyFocused = document.activeElement as HTMLElement;

        // Calculate focusable elements
        this.updateFocusableElements();

        if (this.focusableElements.length === 0) {
            console.warn('FocusTrap: No focusable elements found');
            return;
        }

        // Set up event listeners
        this.container.addEventListener('keydown', this.handleKeyDown);
        this.container.addEventListener('focusin', this.handleFocusIn);

        // Handle outside clicks
        if (!this.options.allowOutsideClick) {
            document.addEventListener('mousedown', this.handleOutsideClick);
        }

        // Focus initial element
        const initialFocus = this.getInitialFocus();
        setTimeout(() => initialFocus?.focus(), 0);

        this.isActive = true;

        // Announce to screen readers
        this.announce('Dialog opened. Press Escape to close.');
    }

    /**
     * Deactivate the focus trap
     */
    deactivate(): void {
        if (!this.isActive) return;

        // Remove event listeners
        this.container.removeEventListener('keydown', this.handleKeyDown);
        this.container.removeEventListener('focusin', this.handleFocusIn);
        document.removeEventListener('mousedown', this.handleOutsideClick);

        // Return focus
        if (this.options.returnFocusOnDeactivate && this.previouslyFocused) {
            const returnFocus = this.getReturnFocus();
            returnFocus?.focus();
        }

        this.isActive = false;

        // Announce
        this.announce('Dialog closed');
    }

    /**
     * Pause the focus trap temporarily
     */
    pause(): void {
        this.container.removeEventListener('keydown', this.handleKeyDown);
        this.container.removeEventListener('focusin', this.handleFocusIn);
    }

    /**
     * Unpause the focus trap
     */
    unpause(): void {
        this.container.addEventListener('keydown', this.handleKeyDown);
        this.container.addEventListener('focusin', this.handleFocusIn);
    }

    // Event Handlers

    private handleKeyDown = (event: KeyboardEvent): void => {
        if (event.key === 'Tab') {
            this.handleTabKey(event);
        }

        if (event.key === 'Escape' && this.options.escapeDeactivates) {
            this.deactivate();
            window.dispatchEvent(new CustomEvent('trap-escape-pressed'));
        }
    };

    private handleTabKey(event: KeyboardEvent): void {
        // Update focusable elements in case DOM changed
        this.updateFocusableElements();

        if (this.focusableElements.length === 0) return;

        const { firstFocusable, lastFocusable } = this;

        if (event.shiftKey) {
            // Shift + Tab
            if (document.activeElement === firstFocusable) {
                event.preventDefault();
                lastFocusable?.focus();
            }
        } else {
            // Tab
            if (document.activeElement === lastFocusable) {
                event.preventDefault();
                firstFocusable?.focus();
            }
        }
    }

    private handleFocusIn = (event: FocusEvent): void => {
        if (!this.container.contains(event.target as Node)) {
            event.preventDefault();
            this.firstFocusable?.focus();
        }
    };

    private handleOutsideClick = (event: MouseEvent): void => {
        if (!this.container.contains(event.target as Node)) {
            event.preventDefault();
            event.stopPropagation();
        }
    };

    // Helper Methods

    private updateFocusableElements(): void {
        const focusableSelectors = [
            'button:not([disabled])',
            'a[href]',
            'input:not([disabled])',
            'select:not([disabled])',
            'textarea:not([disabled])',
            '[tabindex]:not([tabindex="-1"])',
            'details',
            'summary'
        ].join(', ');

        this.focusableElements = Array.from(
            this.container.querySelectorAll(focusableSelectors)
        ).filter(el => {
            // Filter out hidden elements
            const style = window.getComputedStyle(el);
            return style.display !== 'none' &&
                   style.visibility !== 'hidden' &&
                   (el as HTMLElement).offsetWidth > 0;
        });

        this.firstFocusable = this.focusableElements[0] || null;
        this.lastFocusable = this.focusableElements[this.focusableElements.length - 1] || null;
    }

    private getInitialFocus(): HTMLElement | null {
        if (this.options.initialFocus) {
            if (typeof this.options.initialFocus === 'string') {
                return document.querySelector(this.options.initialFocus);
            }
            return this.options.initialFocus;
        }

        return this.firstFocusable;
    }

    private getReturnFocus(): HTMLElement | null {
        if (this.options.setReturnFocus) {
            const returnFocus = this.options.setReturnFocus();
            if (typeof returnFocus === 'string') {
                return document.querySelector(returnFocus);
            }
            return returnFocus;
        }

        return this.previouslyFocused;
    }

    private announce(message: string): void {
        const announcer = document.getElementById('sr-announcer');
        if (announcer) {
            announcer.textContent = message;
            setTimeout(() => { announcer.textContent = ''; }, 1000);
        }
    }
}

// Factory function
export function createFocusTrap(
    container: HTMLElement | string,
    options?: FocusTrapOptions
): FocusTrap {
    return new FocusTrap(container, options);
}
```

---

## 6. Testing Accessibility

### axe-core for WebView

```typescript
// accessibility-testing.ts
/**
 * Automated accessibility testing using axe-core
 */

import axe from 'axe-core';

export interface AccessibilityReport {
    violations: axe.Result[];
    passes: axe.Result[];
    incomplete: axe.Result[];
    inapplicable: axe.Result[];
    timestamp: Date;
    url: string;
}

export class AccessibilityTester {
    private static instance: AccessibilityTester;
    private axeConfigured = false;

    private constructor() {}

    static getInstance(): AccessibilityTester {
        if (!AccessibilityTester.instance) {
            AccessibilityTester.instance = new AccessibilityTester();
        }
        return AccessibilityTester.instance;
    }

    /**
     * Configure axe-core
     */
    configure(): void {
        if (this.axeConfigured) return;

        axe.configure({
            branding: {
                brand: 'UTM Dev',
                application: 'UTM Dev VM Manager'
            },
            reporter: 'v2',
            checks: [
                {
                    id: 'color-contrast-enhanced',
                    evaluate: function(node: Element) {
                        // Custom enhanced contrast check
                        return true;
                    }
                }
            ],
            rules: [
                { id: 'color-contrast', enabled: true },
                { id: 'label', enabled: true },
                { id: 'landmark-one-main', enabled: true },
                { id: 'region', enabled: true },
                { id: 'button-name', enabled: true },
                { id: 'image-alt', enabled: true },
                { id: 'link-name', enabled: true },
                { id: 'form-field-multiple-labels', enabled: true },
                { id: 'focus-order-semantics', enabled: true },
                { id: 'aria-roles', enabled: true },
                { id: 'aria-valid-attr', enabled: true },
                { id: 'keyboard', enabled: true }
            ]
        });

        this.axeConfigured = true;
    }

    /**
     * Run accessibility audit
     */
    async runAudit(context?: string | HTMLElement): Promise<AccessibilityReport> {
        this.configure();

        const results = await axe.run(context || document);

        return {
            violations: results.violations,
            passes: results.passes,
            incomplete: results.incomplete || [],
            inapplicable: results.inapplicable || [],
            timestamp: new Date(),
            url: window.location.href
        };
    }

    /**
     * Run audit and log results
     */
    async auditAndLog(): Promise<void> {
        const report = await this.runAudit();
        this.logReport(report);
    }

    /**
     * Log accessibility report
     */
    logReport(report: AccessibilityReport): void {
        console.group('Accessibility Audit Report');

        if (report.violations.length > 0) {
            console.error(`❌ ${report.violations.length} violations found:`);
            report.violations.forEach(violation => {
                console.error({
                    id: violation.id,
                    impact: violation.impact,
                    description: violation.description,
                    help: violation.helpUrl,
                    nodes: violation.nodes.length
                });
            });
        } else {
            console.log('✅ No violations found!');
        }

        if (report.incomplete.length > 0) {
            console.warn(`⚠️ ${report.incomplete.length} items need manual review:`);
            report.incomplete.forEach(item => {
                console.warn({
                    id: item.id,
                    description: item.description
                });
            });
        }

        console.log(`✅ ${report.passes.length} checks passed`);
        console.groupEnd();
    }

    /**
     * Assert no violations (for tests)
     */
    async assertAccessible(): Promise<void> {
        const report = await this.runAudit();

        if (report.violations.length > 0) {
            const error = new Error(
                `Accessibility violations found: ${report.violations.length}`
            );
            (error as any).violations = report.violations;
            throw error;
        }
    }

    /**
     * Run specific rule
     */
    async runRule(ruleId: string): Promise<axe.RuleResult[]> {
        this.configure();

        const results = await axe.run(document, {
            runOnly: {
                type: 'rule',
                values: [ruleId]
            }
        });

        return results.violations;
    }

    /**
     * Watch for changes and re-audit
     */
    watch(options: { interval?: number; context?: string } = {}): () => void {
        const interval = options.interval || 5000;
        let lastReport: AccessibilityReport | null = null;

        const timer = setInterval(async () => {
            const report = await this.runAudit(options.context);

            // Check for new violations
            if (lastReport && report.violations.length > lastReport.violations.length) {
                console.warn('New accessibility violations detected!');
                this.logReport(report);
            }

            lastReport = report;
        }, interval);

        return () => clearInterval(timer);
    }
}

// Export singleton
export const a11yTester = AccessibilityTester.getInstance();
```

### macOS Accessibility Inspector

```swift
// AccessibilityInspectorTests.swift
import XCTest
import Cocoa

class AccessibilityInspectorTests: XCTestCase {

    var app: XCUIApplication!

    override func setUpWithError() throws {
        try super.setUpWithError()
        continueAfterFailure = false
        app = XCUIApplication()
        app.launchArguments = ["-ui-testing", "-accessibility-testing"]
        app.launch()
    }

    // MARK: - Basic Accessibility Tests

    func testAllElementsHaveLabels() throws {
        let app = XCUIApplication()
        let query = app.descendants(matching: .any)

        var elementsWithoutLabels: [String] = []

        for element in query.allElementsBoundByIndex {
            if element.isAccessibilityElement &&
               element.accessibilityLabel?.isEmpty == true {
                elementsWithoutLabels.append(element.accessibilityIdentifier ?? "Unknown")
            }
        }

        XCTAssertTrue(
            elementsWithoutLabels.isEmpty,
            "Found elements without labels: \(elementsWithoutLabels)"
        )
    }

    func testAllImagesHaveAltText() throws {
        let app = XCUIApplication()
        let images = app.images

        var imagesWithoutAlt: [String] = []

        for image in images.allElementsBoundByIndex {
            if image.isAccessibilityElement &&
               image.accessibilityLabel?.isEmpty == true {
                imagesWithoutAlt.append(image.accessibilityIdentifier ?? "Unknown")
            }
        }

        XCTAssertTrue(
            imagesWithoutAlt.isEmpty,
            "Found images without alt text: \(imagesWithoutAlt)"
        )
    }

    func testAllButtonsHaveLabels() throws {
        let app = XCUIApplication()
        let buttons = app.buttons

        var buttonsWithoutLabels: [String] = []

        for button in buttons.allElementsBoundByIndex {
            if button.accessibilityLabel?.isEmpty == true {
                buttonsWithoutLabels.append(button.accessibilityIdentifier ?? "Unknown")
            }
        }

        XCTAssertTrue(
            buttonsWithoutLabels.isEmpty,
            "Found buttons without labels: \(buttonsWithoutLabels)"
        )
    }

    // MARK: - Focus Tests

    func testFocusOrderIsLogical() throws {
        let app = XCUIApplication()

        // Simulate Tab navigation
        app.typeKey(.tab, modifierFlags: [])

        // Check that focus moves to expected first element
        let firstFocusedElement = app.firstMatch
        XCTAssertTrue(firstFocusedElement.exists, "First focusable element should exist")

        // Continue tabbing through app
        for _ in 0..<10 {
            app.typeKey(.tab, modifierFlags: [])
            XCTAssertTrue(
                XCUIElement.currentFocusedElement().exists,
                "Focus should be on a valid element"
            )
        }
    }

    func testFocusTrapInModal() throws {
        let app = XCUIApplication()

        // Open a modal dialog
        app.buttons["Open Dialog"].tap()

        // Wait for modal
        let modal = app.dialogs["TestDialog"]
        XCTAssertTrue(modal.waitForExistence(timeout: 2), "Modal should appear")

        // Tab through elements
        let focusableCount = modal.descendants(matching: .any).allElementsBoundByIndex.count

        for _ in 0..<focusableCount + 2 {
            app.typeKey(.tab, modifierFlags: [])

            // Focus should stay within modal
            let focusedElement = XCUIElement.currentFocusedElement()
            XCTAssertTrue(
                modal.contains(focusedElement) || focusedElement == modal,
                "Focus should remain within modal"
            )
        }

        // Close modal with Escape
        app.typeKey(.escape, modifierFlags: [])
        XCTAssertFalse(modal.exists, "Modal should close after Escape")
    }

    // MARK: - Dynamic Type Tests

    func testSupportsLargeContentSizes() throws {
        app.adjustSettings { settings in
            settings.largerDynamicType = true
        }

        // Relaunch to apply settings
        app.terminate()
        app.launch()

        // Verify UI is still usable
        XCTAssertTrue(app.webViews.firstMatch.exists, "WebView should be visible")
        XCTAssertTrue(app.buttons.firstMatch.exists, "Buttons should be visible")
    }

    // MARK: - VoiceOver Tests

    func testVoiceOverNavigation() throws {
        // Enable VoiceOver simulation
        XCUIAccessibility.shared.isVoiceOverEnabled = true

        let app = XCUIApplication()

        // Swipe right to move to next element
        app.swipeRight()
        XCTAssertTrue(XCUIElement.currentFocusedElement().exists)

        // Swipe left to move to previous element
        app.swipeLeft()
        XCTAssertTrue(XCUIElement.currentFocusedElement().exists)

        // Double tap to activate
        let focusedElement = XCUIElement.currentFocusedElement()
        app.doubleTap()

        // Verify action was triggered (implementation-specific)
    }

    // MARK: - Keyboard Navigation Tests

    func testKeyboardNavigation() throws {
        let app = XCUIApplication()

        // Test Tab navigation
        app.typeKey(.tab, modifierFlags: [])
        XCTAssertNotNil(XCUIElement.currentFocusedElement())

        // Test Shift+Tab (reverse)
        app.typeKey(.tab, modifierFlags: [.shift])
        XCTAssertNotNil(XCUIElement.currentFocusedElement())

        // Test arrow keys in list
        if let list = app.outlines.firstMatch as? XCUIElement {
            list.tap()
            app.typeKey(.downArrow, modifierFlags: [])
            // Verify selection moved
        }

        // Test Escape closes dialogs
        app.typeKey(.escape, modifierFlags: [])
    }

    // MARK: - Color Contrast Tests

    func testHighContrastMode() throws {
        app.adjustSettings { settings in
            settings.increasedContrast = true
        }

        app.terminate()
        app.launch()

        // Verify UI is still visible and usable
        XCTAssertTrue(app.staticTexts.firstMatch.exists)
    }
}

// Helper extension
extension XCUIElement {
    static func currentFocusedElement() -> XCUIElement {
        // This is a placeholder - actual implementation depends on tracking focus
        return XCUIApplication().firstMatch
    }
}
```

### Automated CI/CD Testing

```yaml
# .github/workflows/accessibility-tests.yml
name: Accessibility Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  accessibility-audit:
    runs-on: macos-latest

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: npm ci

      - name: Run axe-core audit
        run: npm run test:a11y

      - name: Run Pa11y CI
        run: npx pa11y-ci --config .pa11yci.json

      - name: Upload accessibility report
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: accessibility-report
          path: a11y-report.json

  ui-accessibility-tests:
    runs-on: macos-latest

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Run Xcode UI tests
        run: |
          xcodebuild test \
            -scheme UTMDev \
            -destination 'platform=macOS' \
            -only-testing:UTMDevTests/AccessibilityInspectorTests
```

```typescript
// pa11y configuration
// .pa11yci.json
{
  "defaults": {
    "timeout": 60000,
    "standard": "WCAG2AA",
    "reporters": ["cli", "json"],
    "runners": ["axe"]
  },
  "urls": [
    {
      "url": "http://localhost:3000",
      "actions": [
        "wait for element #vm-list to be visible",
        "click element .vm-item:first-child",
        "wait for element .vm-console to be visible"
      ]
    },
    {
      "url": "http://localhost:3000/settings",
      "actions": []
    }
  ]
}
```

---

## WCAG 2.1 AA Compliance Guidelines

### Quick Reference Checklist

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    WCAG 2.1 AA Compliance Checklist                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. PERCEIVABLE                                                           │
│  ├── 1.1.1 Non-text Content: All images have alt text                    │
│  ├── 1.2.1 Audio-only/Video-only: Alternatives provided                  │
│  ├── 1.3.1 Info and Relationships: Semantic HTML, ARIA roles             │
│  ├── 1.3.2 Meaningful Sequence: Logical reading/focus order              │
│  ├── 1.3.3 Sensory Characteristics: Not solely color/shape/position      │
│  ├── 1.4.1 Use of Color: Color not only means of conveying info          │
│  ├── 1.4.3 Contrast (Minimum): 4.5:1 for text, 3:1 for large text        │
│  ├── 1.4.4 Resize Text: Up to 200% without loss of functionality         │
│  ├── 1.4.5 Images of Text: Text used instead of images                   │
│  ├── 1.4.10 Reflow: No horizontal scrolling at 320px                     │
│  ├── 1.4.11 Non-text Contrast: 3:1 for UI components, graphics           │
│  ├── 1.4.12 Text Spacing: No loss at specific spacing values             │
│  └── 1.4.13 Content on Hover/Focus: Dismissible, persistent, hoverable   │
│                                                                             │
│  2. OPERABLE                                                              │
│  ├── 2.1.1 Keyboard: All functionality via keyboard                      │
│  ├── 2.1.2 No Keyboard Trap: Focus can leave all elements                │
│  ├── 2.1.4 Character Key Shortcuts: Can be remapped/turned off           │
│  ├── 2.2.1 Timing Adjustable: Extend or turn off time limits             │
│  ├── 2.2.2 Pause/Stop/Hide: Moving content controllable                  │
│  ├── 2.3.1 Three Flashes: No content flashes > 3x/second                 │
│  ├── 2.4.1 Bypass Blocks: Skip links provided                            │
│  ├── 2.4.2 Page Titled: Descriptive, unique titles                       │
│  ├── 2.4.3 Focus Order: Logical, preserves meaning                       │
│  ├── 2.4.4 Link Purpose: Clear from link text or context                 │
│  ├── 2.4.5 Multiple Ways: Multiple navigation methods                    │
│  ├── 2.4.6 Headings/Labels: Descriptive, topic-oriented                  │
│  ├── 2.4.7 Focus Visible: Keyboard focus indicator visible               │
│  ├── 2.5.1 Pointer Gestures: Single pointer alternatives                 │
│  ├── 2.5.2 Pointer Cancellation: No down-event triggers                  │
│  ├── 2.5.3 Label in Name: Visible label matches accessible name          │
│  ├── 2.5.4 Motion Actuation: Can be turned off                           │
│  └── 2.5.5 Target Size: 44x44px minimum (AAA)                            │
│                                                                             │
│  3. UNDERSTANDABLE                                                        │
│  ├── 3.1.1 Language of Page: Default language specified                   │
│  ├── 3.1.2 Language of Parts: Language changes marked                    │
│  ├── 3.2.1 On Focus: No context change on focus                          │
│  ├── 3.2.2 On Input: No context change on input change                   │
│  ├── 3.2.3 Consistent Navigation: Same order across pages                │
│  ├── 3.2.4 Consistent Identification: Same function = same ID            │
│  ├── 3.2.5 Error Prevention: No errors on legal/financial/data pages     │
│  ├── 3.3.1 Error Identification: Errors clearly identified               │
│  ├── 3.3.2 Labels/Instructions: Provided for user input                  │
│  ├── 3.3.3 Error Suggestion: Suggestions for correction                  │
│  └── 3.3.4 Error Prevention (Legal/Data): Reversible, verified, confirmed│
│                                                                             │
│  4. ROBUST                                                                │
│  ├── 4.1.1 Parsing: Valid, unique IDs, complete tags                     │
│  ├── 4.1.2 Name/Role/Value: Accessible name, role, value for all UI      │
│  ├── 4.1.3 Status Messages: Announced via live regions                   │
│  └── 4.1.4 Auto-updating Content: User control over updates              │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Summary

Accessibility implementation priorities for UTM Dev:

1. **Native macOS Accessibility** - NSAccessibility protocol implementation
2. **Keyboard Navigation** - Full keyboard operability, focus management
3. **Screen Reader Support** - VoiceOver (macOS) and ORCA (Linux) compatibility
4. **Live Regions** - ARIA live announcements for dynamic VM status
5. **Focus Management** - WebView coordination, modal focus traps
6. **System Settings** - High contrast, reduced motion, font scaling
7. **Automated Testing** - axe-core, UI tests, CI/CD integration

---

*Related: `testing-exploration.md`, `native-ui-components-exploration.md`, `htmx-datastar-patterns.md`*
