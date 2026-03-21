# Strada Production - Native UI Components Exploration

## Overview

This document explores native UI components that complement WebView content in production iOS/Android apps using Strada.

## Architecture

### Component Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                    Native UI Layer                              │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐   │
│  │ Navigation  │ │   Tab Bar   │ │    Modals/Sheets        │   │
│  │   Bar       │ │             │ │                         │   │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐   │
│  │   Bottom    │ │  Floating   │ │    Toast/Snackbar       │   │
│  │   Sheet     │ │  Action Btn │ │                         │   │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │ Strada messages
                              │ (Page/Composer components)
                              │
┌─────────────────────────────────────────────────────────────────┐
│                    WebView Layer                                │
│  - Web content                                                  │
│  - Web-based Strada components                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Navigation Bar / Toolbar

### iOS Navigation Bar

#### Basic Setup

```swift
class VisitableViewController: UIViewController {

    let webView: WKWebView
    lazy var bridgeDelegate: BridgeDelegate = {
        BridgeDelegate(location: visitableURL.absoluteString,
                       destination: self,
                       componentTypes: BridgeComponent.allTypes)
    }()

    override func viewDidLoad() {
        super.viewDidLoad()

        // Configure navigation bar
        navigationController?.navigationBar.prefersLargeTitles = true
        navigationItem.largeTitleDisplayMode = .automatic

        // Bridge lifecycle
        Bridge.initialize(webView)
        bridgeDelegate.onViewDidLoad()
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        bridgeDelegate.onViewWillAppear()
    }
}
```

#### Dynamic Title via Strada

```swift
// PageComponent.swift
class PageComponent: BridgeComponent {

    override var name: String { "page" }

    override func onReceive(message: Message) {
        switch message.event {
        case "connect":
            let data: PageData? = message.data()
            updateNavigationBar(data: data)

        case "navigation-state":
            let data: NavigationData? = message.data()
            updateTitle(data?.title)

        case "show-native-loading":
            startNativeLoadingIndicator()

        case "hide-native-loading":
            stopNativeLoadingIndicator()
        }
    }

    private func updateNavigationBar(data: PageData) {
        // Set title
        if let title = data.title {
            destination.navigationItem.title = title
        }

        // Set back button
        if let showBack = data.showBackButton {
            destination.navigationItem.hidesBackButton = !showBack
        }

        // Set right bar button items
        if let rightItems = data.rightBarButtonItems {
            destination.navigationItem.rightBarButtonItems =
                rightItems.map { createBarButton(item: $0) }
        }
    }

    private func createBarButton(item: BarButtonItemData) -> UIBarButtonItem {
        UIBarButtonItem(
            title: item.title,
            style: .plain,
            target: self,
            action: #selector(barButtonTapped(_:))
        )
    }

    @objc private func barButtonTapped(_ sender: UIBarButtonItem) {
        // Notify web of tap
        reply(to: "bar-button-tapped", data: ["title": sender.title])
    }
}
```

#### Search Controller Integration

```swift
class SearchableViewController: VisitableViewController, UISearchResultsUpdating {

    var searchController: UISearchController!

    override func viewDidLoad() {
        super.viewDidLoad()

        searchController = UISearchController(searchResultsController: nil)
        searchController.searchResultsUpdater = self
        searchController.obscuresBackgroundDuringPresentation = false

        navigationItem.searchController = searchController
        navigationItem.hidesSearchBarWhenScrolling = true
    }

    func updateSearchResults(for searchController: UISearchController) {
        // Send search query to web
        if let query = searchController.searchBar.text {
            // Via Strada or direct JS call
            webView.evaluateJavaScript("window.handleNativeSearch('\(query)')")
        }
    }
}
```

### Android Toolbar

#### Basic Setup

```kotlin
class MainActivity : AppCompatActivity(), BridgeDestination {

    private lateinit var toolbar: Toolbar
    private lateinit var webView: WebView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        setSupportActionBar(toolbar)
        supportActionBar?.setDisplayHomeAsUpEnabled(true)

        setupWebView()
        setupBridge()
    }

    private fun setupBridge() {
        bridgeDelegate = BridgeDelegate(
            location = initialUrl.toString(),
            destination = this,
            componentTypes = BridgeComponent.allTypes
        )

        lifecycle.addObserver(bridgeDelegate)
    }
}
```

#### Dynamic Toolbar Updates

```kotlin
// PageComponent.kt
class PageComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override val name: String = "page"

    override fun onReceive(message: Message) {
        when (message.event) {
            "connect" -> {
                val data: PageData? = message.data()
                updateToolbar(data)
            }
            "navigation-state" -> {
                val data: NavigationData? = message.data()
                updateTitle(data?.title)
            }
            "show-menu" -> {
                inflateMenu(data.menuRes)
            }
        }
    }

    private fun updateToolbar(data: PageData) {
        val activity = destination.requireActivity() as? AppCompatActivity
        val actionBar = activity?.supportActionBar

        actionBar?.title = data.title
        actionBar?.setDisplayHomeAsUpEnabled(data.showBackButton != false)
        actionBar?.setHomeAsUpIndicator(R.drawable.ic_back)
    }
}
```

#### Menu Handling

```kotlin
class MainActivity : AppCompatActivity() {

    override fun onCreateOptionsMenu(menu: Menu): Boolean {
        menuInflater.inflate(R.menu.webview_menu, menu)
        return true
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        return when (item.itemId) {
            android.R.id.home -> {
                // Handle back
                if (webView.canGoBack()) webView.goBack()
                else finish()
                true
            }
            R.id.action_share -> {
                // Send to web for share handling
                pageComponent.onMenuAction("share")
                true
            }
            R.id.action_refresh -> {
                webView.reload()
                true
            }
            else -> super.onOptionsItemSelected(item)
        }
    }
}
```

## Tab Bar / Bottom Navigation

### iOS Tab Bar

#### Tab Bar Controller Setup

```swift
enum AppTab: CaseIterable {
    case home
    case search
    case profile
    case settings

    var rootURL: URL {
        switch self {
        case .home: return URL(string: "https://example.com/")!
        case .search: return URL(string: "https://example.com/search")!
        case .profile: return URL(string: "https://example.com/profile")!
        case .settings: return URL(string: "https://example.com/settings")!
        }
    }

    var tabBarItem: UITabBarItem {
        switch self {
        case .home: return UITabBarItem(title: "Home",
                                         image: UIImage(systemName: "house"),
                                         selectedImage: UIImage(systemName: "house.fill"))
        case .search: return UITabBarItem(title: "Search",
                                           image: UIImage(systemName: "magnifyingglass"),
                                           selectedImage: UIImage(systemName: "magnifyingglass"))
        case .profile: return UITabBarItem(title: "Profile",
                                            image: UIImage(systemName: "person"),
                                            selectedImage: UIImage(systemName: "person.fill"))
        case .settings: return UITabBarItem(title: "Settings",
                                             image: UIImage(systemName: "gear"),
                                             selectedImage: UIImage(systemName: "gear.fill"))
        }
    }
}

class MainTabBarController: UITabBarController {

    override func viewDidLoad() {
        super.viewDidLoad()

        viewControllers = AppTab.allCases.map { tab in
            let navController = UINavigationController(
                rootViewController: VisitableViewController(url: tab.rootURL)
            )
            navController.tabBarItem = tab.tabBarItem
            navController.navigationBar.prefersLargeTitles = true
            return navController
        }
    }
}
```

#### Tab Switching via Strada

```swift
// PageComponent handles tab navigation requests
class PageComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "navigate-tab":
            let data: TabNavigationData? = message.data()
            navigateToTab(data?.tabName ?? "")
        }
    }

    private func navigateToTab(_ tabName: String) {
        guard let tabBarController = destination.tabBarController else { return }

        switch tabName {
        case "home":
            tabBarController.selectedIndex = 0
        case "search":
            tabBarController.selectedIndex = 1
        case "profile":
            tabBarController.selectedIndex = 2
        case "settings":
            tabBarController.selectedIndex = 3
        default:
            break
        }
    }
}
```

### Android Bottom Navigation

#### Layout

```xml
<!-- res/layout/activity_main.xml -->
<androidx.constraintlayout.widget.ConstraintLayout
    xmlns:android="http://schemas.android.com/apk/res/android"
    xmlns:app="http://schemas.android.com/apk/res-auto"
    android:layout_width="match_parent"
    android:layout_height="match_parent">

    <WebView
        android:id="@+id/webView"
        android:layout_width="0dp"
        android:layout_height="0dp"
        app:layout_constraintTop_toTopOf="parent"
        app:layout_constraintBottom_toTopOf="@id/bottom_navigation"
        app:layout_constraintStart_toStartOf="parent"
        app:layout_constraintEnd_toEndOf="parent" />

    <com.google.android.material.bottomnavigation.BottomNavigationView
        android:id="@+id/bottom_navigation"
        android:layout_width="0dp"
        android:layout_height="wrap_content"
        app:menu="@menu/bottom_navigation_menu"
        app:layout_constraintBottom_toBottomOf="parent"
        app:layout_constraintStart_toStartOf="parent"
        app:layout_constraintEnd_toEndOf="parent" />

</androidx.constraintlayout.widget.ConstraintLayout>
```

#### Menu Resource

```xml
<!-- res/menu/bottom_navigation_menu.xml -->
<menu xmlns:android="http://schemas.android.com/apk/res/android">
    <item
        android:id="@+id/navigation_home"
        android:icon="@drawable/ic_home"
        android:title="Home" />
    <item
        android:id="@+id/navigation_search"
        android:icon="@drawable/ic_search"
        android:title="Search" />
    <item
        android:id="@+id/navigation_profile"
        android:icon="@drawable/ic_profile"
        android:title="Profile" />
</menu>
```

#### Activity Implementation

```kotlin
class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var bottomNavigation: BottomNavigationView

    private val tabUrls = mapOf(
        R.id.navigation_home to "https://example.com/",
        R.id.navigation_search to "https://example.com/search",
        R.id.navigation_profile to "https://example.com/profile"
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        webView = findViewById(R.id.webView)
        bottomNavigation = findViewById(R.id.bottom_navigation)

        setupBottomNavigation()
        setupWebView()
    }

    private fun setupBottomNavigation() {
        bottomNavigation.setOnItemSelectedListener { item ->
            tabUrls[item.itemId]?.let { url ->
                webView.loadUrl(url)
                // Update Strada context
                pageComponent.setActiveTab(item.itemId)
                true
            } ?: false
        }
    }
}
```

## Bottom Sheets

### iOS Bottom Sheet

```swift
class PageComponent: BridgeComponent {

    private var bottomSheetVC: BottomSheetViewController?

    override func onReceive(message: Message) {
        switch message.event {
        case "show-bottom-sheet":
            let data: BottomSheetData? = message.data()
            showBottomSheet(data: data!)

        case "hide-bottom-sheet":
            hideBottomSheet()
        }
    }

    private func showBottomSheet(data: BottomSheetData) {
        let sheetVC = BottomSheetViewController()
        sheetVC.contentURL = URL(string: data.url)!
        sheetVC.titleText = data.title

        if let sheet = sheetVC.sheetPresentationController {
            sheet.detents = data.detents.map { detent in
                switch detent {
                case "medium": return .medium()
                case "large": return .large()
                default: return .medium()
                }
            }
            sheet.prefersGrabberVisible = true
        }

        destination.present(sheetVC, animated: true)
        bottomSheetVC = sheetVC
    }

    private func hideBottomSheet() {
        bottomSheetVC?.dismiss(animated: true)
        bottomSheetVC = nil
    }
}

// BottomSheetViewController.swift
class BottomSheetViewController: UIViewController {

    var contentURL: URL!
    var titleText: String = ""

    private var webView: WKWebView!

    override func viewDidLoad() {
        super.viewDidLoad()

        title = titleText

        webView = WKWebView(frame: .zero)
        view.addSubview(webView)

        webView.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            webView.topAnchor.constraint(equalTo: view.topAnchor),
            webView.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            webView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            webView.trailingAnchor.constraint(equalTo: view.trailingAnchor)
        ])

        webView.load(URLRequest(url: contentURL))
    }
}
```

### Android Bottom Sheet

```kotlin
// BottomSheetFragment.kt
class BottomSheetFragment : BottomSheetDialogFragment() {

    companion object {
        fun newInstance(url: String, title: String) = BottomSheetFragment().apply {
            arguments = bundleOf("url" to url, "title" to title)
        }
    }

    private lateinit var webView: WebView

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View {
        return WebView(requireContext()).apply {
            loadUrl(arguments?.getString("url") ?: "")
        }
    }
}

// PageComponent.kt
class PageComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override fun onReceive(message: Message) {
        when (message.event) {
            "show-bottom-sheet" -> {
                val data: BottomSheetData? = message.data()
                showBottomSheet(data!!)
            }
            "hide-bottom-sheet" -> {
                hideBottomSheet()
            }
        }
    }

    private fun showBottomSheet(data: BottomSheetData) {
        val activity = destination.requireActivity()
        val fragment = BottomSheetFragment.newInstance(data.url, data.title)
        fragment.show(activity.supportFragmentManager, "bottom_sheet")
    }

    private fun hideBottomSheet() {
        val activity = destination.requireActivity()
        val fragment = activity.supportFragmentManager
            .findFragmentByTag("bottom_sheet") as? BottomSheetFragment
        fragment?.dismiss()
    }
}
```

## Floating Action Button (Android)

### Layout with FAB

```xml
<!-- res/layout/activity_main.xml -->
<androidx.coordinatorlayout.widget.CoordinatorLayout
    xmlns:android="http://schemas.android.com/apk/res/android"
    xmlns:app="http://schemas.android.com/apk/res-auto"
    android:layout_width="match_parent"
    android:layout_height="match_parent">

    <WebView
        android:id="@+id/webView"
        android:layout_width="match_parent"
        android:layout_height="match_parent" />

    <com.google.android.material.floatingactionbutton.FloatingActionButton
        android:id="@+id/fab"
        android:layout_width="wrap_content"
        android:layout_height="wrap_content"
        android:layout_gravity="bottom|end"
        android:layout_margin="16dp"
        android:src="@drawable/ic_add"
        app:layout_behavior="com.google.android.material.appbar.AppBarLayout$ScrollingViewBehavior" />

</androidx.coordinatorlayout.widget.CoordinatorLayout>
```

### FAB Handling via Strada

```kotlin
// PageComponent.kt
class PageComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override fun onReceive(message: Message) {
        when (message.event) {
            "show-fab" -> {
                val data: FabData? = message.data()
                showFab(icon = data?.icon, color = data?.color)
            }
            "hide-fab" -> {
                hideFab()
            }
            "fab-tapped" -> {
                // Web is requesting FAB tap - notify web
                replyTo("fab-tapped", data = emptyMap())
            }
        }
    }

    private fun showFab(icon: String?, color: String?) {
        val activity = destination.requireActivity() as? MainActivity
        activity?.fab?.apply {
            show()
            // Set icon from drawable name
            icon?.let { setIconResource(it) }
            // Set color
            color?.let { setBackgroundColor(Color.parseColor(it)) }

            setOnClickListener {
                // Notify web of tap
                onFabTapped()
            }
        }
    }

    private fun hideFab() {
        (destination.requireActivity() as? MainActivity)?.fab?.hide()
    }

    private fun onFabTapped() {
        replyTo("fab-action", data = mapOf("action" to "primary"))
    }
}
```

## Toast / Snackbar

### iOS Toast Alternative

```swift
// ToastView.swift
class ToastView: UIView {

    private let label: UILabel = {
        let label = UILabel()
        label.textColor = .white
        label.textAlignment = .center
        label.numberOfLines = 0
        label.translatesAutoresizingMaskIntoConstraints = false
        return label
    }()

    private var hideTimer: Timer?

    init() {
        super.init(frame: .zero)
        backgroundColor = UIColor.black.withAlphaComponent(0.8)
        layer.cornerRadius = 8

        addSubview(label)

        NSLayoutConstraint.activate([
            label.topAnchor.constraint(equalTo: topAnchor, constant: 12),
            label.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -12),
            label.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 16),
            label.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -16)
        ])
    }

    func show(in view: UIView, message: String, duration: TimeInterval = 2.0) {
        view.addSubview(self)

        translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 16),
            trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -16),
            bottomAnchor.constraint(equalTo: view.safeAreaLayoutGuide.bottomAnchor, constant: -16)
        ])

        label.text = message

        hideTimer?.invalidate()
        hideTimer = Timer.scheduledTimer(withTimeInterval: duration, repeats: false) { [weak self] _ in
            self?.hide()
        }
    }

    private func hide() {
        removeFromSuperview()
        hideTimer?.invalidate()
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}

// PageComponent usage
class PageComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "show-toast":
            let data: ToastData? = message.data()
            showToast(message: data?.message ?? "", duration: data?.duration ?? 2.0)
        }
    }

    private func showToast(message: String, duration: TimeInterval) {
        let toast = ToastView()
        toast.show(in: destination.view, message: message, duration: duration)
    }
}
```

### Android Snackbar

```kotlin
// PageComponent.kt
class PageComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override fun onReceive(message: Message) {
        when (message.event) {
            "show-snackbar" -> {
                val data: SnackbarData? = message.data()
                showSnackbar(
                    message = data?.message ?: "",
                    duration = data?.duration ?: "short",
                    actionText = data?.actionText,
                    actionCallback = data?.actionCallback
                )
            }
        }
    }

    private fun showSnackbar(
        message: String,
        duration: String,
        actionText: String? = null,
        actionCallback: (() -> Unit)? = null
    ) {
        val view = destination.requireActivity().findViewById<View>(android.R.id.content)
        val snackbarDuration = when (duration) {
            "long" -> Snackbar.LENGTH_LONG
            "indefinite" -> Snackbar.LENGTH_INDEFINITE
            else -> Snackbar.LENGTH_SHORT
        }

        val snackbar = Snackbar.make(view, message, snackbarDuration)

        actionText?.let { text ->
            snackbar.setAction(text) {
                actionCallback?.invoke()
            }
        }

        snackbar.show()
    }
}
```

## Pull-to-Refresh

### iOS Pull-to-Refresh

```swift
class VisitableViewController: UIViewController {

    var refreshControl: UIRefreshControl!

    override func viewDidLoad() {
        super.viewDidLoad()

        refreshControl = UIRefreshControl()
        refreshControl.addTarget(self, action: #selector(handleRefresh), for: .valueChanged)
        webView.scrollView.refreshControl = refreshControl
    }

    @objc private func handleRefresh() {
        // Option 1: Direct web reload
        webView.evaluateJavaScript("window.location.reload()")

        // Option 2: Via Strada
        pageComponent.onRefreshStarted()
    }

    func stopRefreshing() {
        refreshControl?.endRefreshing()
    }
}

// PageComponent
class PageComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "refresh-started":
            // Web acknowledges refresh started
            break
        case "refresh-completed":
            // Stop native refresh indicator
            (destination as? VisitableViewController)?.stopRefreshing()
        }
    }

    func onRefreshStarted() {
        reply(to: "refresh-started", data: [:])
    }
}
```

### Android Swipe-to-Refresh

```xml
<!-- res/layout/activity_main.xml -->
<androidx.swiperefreshlayout.widget.SwipeRefreshLayout
    android:id="@+id/swipeRefresh"
    xmlns:android="http://schemas.android.com/apk/res/android"
    android:layout_width="match_parent"
    android:layout_height="match_parent">

    <WebView
        android:id="@+id/webView"
        android:layout_width="match_parent"
        android:layout_height="match_parent" />

</androidx.swiperefreshlayout.widget.SwipeRefreshLayout>
```

```kotlin
// MainActivity.kt
class MainActivity : AppCompatActivity() {

    private lateinit var swipeRefresh: SwipeRefreshLayout
    private lateinit var webView: WebView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        swipeRefresh = findViewById(R.id.swipeRefresh)
        webView = findViewById(R.id.webView)

        swipeRefresh.setOnRefreshListener {
            // Option 1: Direct reload
            webView.reload()

            // Option 2: Via Strada
            pageComponent.onRefreshStarted()
        }
    }

    fun stopRefreshing() {
        swipeRefresh.isRefreshing = false
    }
}

// PageComponent.kt
class PageComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override fun onReceive(message: Message) {
        when (message.event) {
            "refresh-completed" -> {
                (destination.requireActivity() as? MainActivity)?.stopRefreshing()
            }
        }
    }

    fun onRefreshStarted() {
        replyTo("refresh-started", data = emptyMap())
    }
}
```

## Loading States

### iOS Activity Indicator

```swift
// In navigation bar
class PageComponent: BridgeComponent {

    private var activityIndicator: UIActivityIndicatorView?

    override func onReceive(message: Message) {
        switch message.event {
        case "show-loading":
            showLoadingIndicator()
        case "hide-loading":
            hideLoadingIndicator()
        }
    }

    private func showLoadingIndicator() {
        let indicator = UIActivityIndicatorView(style: .medium)
        indicator.startAnimating()

        let item = UIBarButtonItem(customView: indicator)
        destination.navigationItem.rightBarButtonItem = item

        activityIndicator = indicator
    }

    private func hideLoadingIndicator() {
        destination.navigationItem.rightBarButtonItem = nil
        activityIndicator = nil
    }
}
```

### Android Progress Bar

```xml
<!-- Linear progress bar at top -->
<com.google.android.material.progressindicator.LinearProgressIndicator
    android:id="@+id/progressBar"
    android:layout_width="match_parent"
    android:layout_height="wrap_content"
    android:indeterminate="true"
    android:visibility="gone"
    app:layout_constraintTop_toTopOf="parent" />
```

```kotlin
// PageComponent.kt
class PageComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override fun onReceive(message: Message) {
        when (message.event) {
            "show-loading" -> showProgressBar()
            "hide-loading" -> hideProgressBar()
        }
    }

    private fun showProgressBar() {
        val activity = destination.requireActivity() as? MainActivity
        activity?.progressBar?.visibility = View.VISIBLE
    }

    private fun hideProgressBar() {
        val activity = destination.requireActivity() as? MainActivity
        activity?.progressBar?.visibility = View.GONE
    }
}
```

## Summary

Production WebView apps need native UI components:

1. **Navigation Bar/Toolbar** - Titles, back buttons, actions
2. **Tab Bar/Bottom Nav** - Primary navigation
3. **Bottom Sheets** - Contextual content overlays
4. **FAB** - Primary action button
5. **Toast/Snackbar** - Feedback messages
6. **Pull-to-Refresh** - Content refresh
7. **Loading Indicators** - Progress feedback

All controlled via Strada messages from web to native.

---

*Related: `navigation-routing-exploration.md`, `performance-optimization-exploration.md`*
