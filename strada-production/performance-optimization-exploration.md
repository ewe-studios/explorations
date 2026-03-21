# Strada Production - Performance & Optimization Exploration

## Overview

This document explores performance optimization strategies for production iOS/Android apps using WebViews and Strada.

## WebView Preloading & Warm Start

### iOS WebView Pool

```swift
// WebViewPool.swift
class WebViewPool {

    static let shared = WebViewPool()

    private var pool: [WKWebView] = []
    private let maxPoolSize = 2

    /// Get a pre-warmed WebView
    func acquire() -> WKWebView {
        if let webView = pool.popLast() {
            return webView
        }
        return createPreloadedWebView()
    }

    /// Return WebView to pool for reuse
    func release(_ webView: WKWebView) {
        guard pool.count < maxPoolSize else {
            webView.configuration.userContentController
                .removeAllUserScripts()
            return
        }

        // Clean up state
        webView.navigationDelegate = nil
        webView.scrollView.delegate = nil
        webView.removeObserver(self, forKeyPath: "URL")

        // Stop any loading
        webView.stopLoading()
        webView.load(URLRequest(url: URL(string: "about:blank")!))

        pool.append(webView)
    }

    private func createPreloadedWebView() -> WKWebView {
        let configuration = WKWebViewConfiguration()

        // Optimize configuration
        configuration.preferences.javaScriptEnabled = true
        configuration.preferences.cacheEnabled = true
        configuration.websiteDataStore = .default()

        // Set pool-wide user agent
        configuration.applicationNameForUserAgent = "App/1.0"

        let webView = WKWebView(frame: .zero, configuration: configuration)

        // Pre-load common resources in background
        preloadResources(in: webView)

        return webView
    }

    private func preloadResources(in webView: WKWebView) {
        // Pre-fetch authentication tokens, user data, etc.
        webView.evaluateJavaScript("""
            caches.open('prefetch').then(cache => {
                cache.addAll(['/api/user', '/assets/styles.css'])
            })
        """)
    }
}

// Usage in view controller
class VisitableViewController: UIViewController {

    private var webView: WKWebView!
    private var isWebViewFromPool = false

    override func viewDidLoad() {
        super.viewDidLoad()

        // Acquire from pool
        webView = WebViewPool.shared.acquire()
        isWebViewFromPool = true

        // Set delegates
        webView.navigationDelegate = self
        webView.scrollView.delegate = self

        view.addSubview(webView)
        setupWebViewConstraints()
    }

    override func viewDidDisappear(_ animated: Bool) {
        super.viewDidDisappear(animated)

        // Return to pool for reuse
        if isWebViewFromPool {
            WebViewPool.shared.release(webView)
        }
    }
}
```

### Android WebView Initialization

```kotlin
// WebViewProvider.kt
object WebViewProvider {

    private var preloadedWebView: WebView? = null
    private val lock = Any()

    fun getWebView(context: Context): WebView {
        synchronized(lock) {
            return preloadedWebView ?: createWebView(context).also {
                preloadedWebView = it
            }
        }
    }

    fun preload(context: Context) {
        synchronized(lock) {
            if (preloadedWebView == null) {
                preloadedWebView = createWebView(context)
            }
        }
    }

    private fun createWebView(context: Context): WebView {
        return WebView(context.applicationContext).apply {
            // Optimize settings
            settings.apply {
                javaScriptEnabled = true
                domStorageEnabled = true
                databaseEnabled = true
                cacheMode = WebSettings.LOAD_DEFAULT

                // Performance settings
                useWideViewPort = true
                loadWithOverviewMode = true
                renderPriority = WebSettings.RenderPriority.HIGH

                // Disable unnecessary features
                geolocationEnabled = false
                javaScriptCanOpenWindowsAutomatically = false
            }

            // Set initial background
            setBackgroundColor(Color.TRANSPARENT)

            // Enable hardware acceleration
            setLayerType(View.LAYER_TYPE_HARDWARE, null)
        }
    }

    fun recycle(webView: WebView) {
        synchronized(lock) {
            // Clean up for next use
            webView.stopLoading()
            webView.loadUrl("about:blank")
            webView.clearHistory()

            // Keep for next use
            preloadedWebView = webView
        }
    }
}

// Application class - preload on app start
class MyApplication : Application() {

    override fun onCreate() {
        super.onCreate()

        // Preload WebView in background
        Thread {
            WebViewProvider.preload(this)
        }.start()
    }
}
```

### Application Cold Start Optimization

```swift
// iOS: Preload during splash screen
class LaunchViewController: UIViewController {

    override func viewDidLoad() {
        super.viewDidLoad()

        // Start WebView preloading immediately
        preloadWebViewInBackground()
    }

    private func preloadWebViewInBackground() {
        DispatchQueue.global(qos: .userInitiated).async {
            // Create and configure WebView
            let webView = WKWebView(frame: .zero, configuration: self.createConfiguration())

            // Load initial URL
            let request = URLRequest(url: URL(string: "https://example.com/")!)
            webView.load(request)

            // Store for when main VC is ready
            WebViewCache.shared.cache(webView)

            DispatchQueue.main.async {
                self.showMainApp()
            }
        }
    }
}
```

## Memory Management

### iOS WebView Memory Pressure

```swift
// WebViewMemoryManager.swift
class WebViewMemoryManager {

    static let shared = WebViewMemoryManager()

    /// Handle memory warning
    func handleMemoryWarning() {
        // Clear WebView cache
        clearWebViewCaches()

        // Release pooled WebViews
        WebViewPool.shared.releaseAll()
    }

    private func clearWebViewCaches() {
        // Clear HTTP cache
        URLCache.shared.removeAllCachedResponses()

        // Clear WebKit cache
        let dataTypes = Set([
            WKWebsiteDataTypeDiskCache,
            WKWebsiteDataTypeMemoryCache,
            WKWebsiteDataTypeSessions
        ])

        WKWebsiteDataStore.default().removeData(
            ofTypes: dataTypes,
            modifiedSince: Date(timeIntervalSince1970: 0),
            completionHandler: {}
        )
    }
}

// In AppDelegate
func applicationDidReceiveMemoryWarning(_ application: UIApplication) {
    WebViewMemoryManager.shared.handleMemoryWarning()
}
```

### Android WebView Memory

```kotlin
// WebViewMemoryManager.kt
object WebViewMemoryManager {

    fun onLowMemory(context: Context) {
        // Clear WebView cache
        clearCache(context)

        // Trim memory
        trimMemory()
    }

    private fun clearCache(context: Context) {
        // Clear HTTP cache
        try {
            val cacheDir = File(context.cacheDir, "webview_cache")
            if (cacheDir.exists()) {
                cacheDir.deleteRecursively()
            }
        } catch (e: Exception) {
            Log.e("WebView", "Failed to clear cache", e)
        }

        // Clear WebView data
        WebView(context).apply {
            clearCache(true)
            clearHistory()
            destroy()
        }
    }

    private fun trimMemory() {
        // On Android 14+, use trimMemory()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            Runtime.getRuntime().gc()
        }
    }
}

// In Application
class MyApplication : Application() {

    override fun onLowMemory() {
        super.onLowMemory()
        WebViewMemoryManager.onLowMemory(this)
    }

    override fun onTrimMemory(level: Int) {
        super.onTrimMemory(level)
        if (level >= TRIM_MEMORY_RUNNING_CRITICAL) {
            WebViewMemoryManager.onLowMemory(this)
        }
    }
}
```

### Image Memory Management

```swift
// Limit image cache size
class ImageCacheConfig {

    static let maxMemory = Runtime.getRuntime().maxMemory() / 1024
    static let cacheSize = maxMemory / 8  // Use 1/8 of available memory

    static func configure() {
        // For custom image loading libraries
        ImageCache.shared.config.maxMemory = cacheSize
    }
}
```

```kotlin
// Android: Configure Glide/Coil for WebView images
class ImageLoadingConfig {

    companion object {

        fun configure(context: Context) {
            // Glide configuration
            Glide.get(context).apply {
                setMemoryCache(MemorySizeCalculator.Builder(context).build())
                setDiskCache(DiskCache.Factory(File(context.cacheDir, "image_cache"), 100 * 1024 * 1024))
            }
        }
    }
}
```

## Caching Strategies

### iOS HTTP Cache

```swift
// HTTPCacheManager.swift
class HTTPCacheManager {

    enum CachePolicy {
        case `default`      // Use URLCache default
        case aggressive     // Cache everything
        case conservative   // Only cache static assets
        case noCache        // Never cache
    }

    private var policy: CachePolicy = .default

    func configure(policy: CachePolicy) {
        self.policy = policy

        switch policy {
        case .aggressive:
            // 50MB memory, 200MB disk
            URLCache.shared = URLCache(
                memoryCapacity: 50 * 1024 * 1024,
                diskCapacity: 200 * 1024 * 1024,
                diskPath: "http_cache"
            )
        case .conservative:
            // Only cache CSS, JS, images
            setupSelectiveCaching()
        default:
            break
        }
    }

    private func setupSelectiveCaching() {
        // Use NSURLProtocol to intercept and selectively cache
        URLProtocol.registerClass(SelectiveCacheProtocol.self)
    }
}

// SelectiveCacheProtocol.swift
class SelectiveCacheProtocol: URLProtocol {

    static let cacheableTypes = ["text/css", "application/javascript", "image/png", "image/jpeg"]

    override class func canInit(with request: URLRequest) -> Bool {
        guard let url = request.url else { return false }

        // Only cache GET requests
        guard request.httpMethod == "GET" else { return false }

        // Only cache static assets
        let extensionTypes = ["css", "js", "png", "jpg", "jpeg", "gif", "svg", "woff2"]
        return extensionTypes.contains { url.pathExtension.lowercased() == $0 }
    }

    override func startLoading() {
        // Load from cache or network
        URLSession.shared.dataTask(with: request) { data, response, error in
            // Cache response
            if let response = response as? HTTPURLResponse,
               let mimeType = response.mimeType,
               cacheableTypes.contains(mimeType) {

                URLCache.shared.storeCachedResponse(
                    CachedURLResponse(response: response, data: data!),
                    for: request
                )
            }

            self.client?.urlProtocol(self, didReceive: response!, cachingPolicy: .useCache)

            if let data = data {
                self.client?.urlProtocol(self, didLoad: data)
            }

            self.client?.urlProtocolDidFinishLoading(self)

        }.resume()
    }
}
```

### Android HTTP Cache

```kotlin
// HttpCacheManager.kt
class HttpCacheManager {

    fun configureCache(context: Context) {
        val cacheSize = 100L * 1024 * 1024  // 100MB

        val cache = Cache(
            directory = File(context.cacheDir, "http_cache"),
            maxSize = cacheSize
        )

        // Apply to OkHttpClient
        val client = OkHttpClient.Builder()
            .cache(cache)
            .build()
    }
}

// Interceptor for selective caching
class SelectiveCacheInterceptor : Interceptor {

    companion object {
        private val CACHEABLE_TYPES = setOf("text/css", "application/javascript", "image/")
    }

    override fun intercept(chain: Interceptor.Chain): Response {
        val request = chain.request()
        val response = chain.proceed(request)

        // Only cache GET requests for static assets
        if (request.method != "GET") return response

        val mimeType = response.header("Content-Type") ?: return response

        if (CACHEABLE_TYPES.any { mimeType.startsWith(it) }) {
            // Cache for 7 days
            return response.newBuilder()
                .header("Cache-Control", "public, max-age=${7 * 24 * 60 * 60}")
                .build()
        }

        return response
    }
}
```

### Service Worker Coordination

```typescript
// sw.js - Service Worker for offline caching
const CACHE_NAME = 'app-cache-v1'
const STATIC_ASSETS = [
  '/',
  '/styles/main.css',
  '/scripts/app.js',
  '/images/logo.svg'
]

self.addEventListener('install', (event) => {
  // Cache static assets on install
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(STATIC_ASSETS)
    })
  )
})

self.addEventListener('fetch', (event) => {
  // Network-first for API calls
  if (event.request.url.includes('/api/')) {
    event.respondWith(
      fetch(event.request).catch(() => {
        return caches.match(event.request)
      })
    )
    return
  }

  // Cache-first for static assets
  event.respondWith(
    caches.match(event.request).then((cached) => {
      return cached || fetch(event.request)
    })
  )
})

// Notify native when cache is ready
self.addEventListener('activate', () => {
  // Send message to web for Strada notification
  clients.matchAll().then((clients) => {
    clients.forEach((client) => {
      client.postMessage({ type: 'CACHE_READY' })
    })
  })
})
```

```swift
// iOS: Receive service worker cache ready
class PageComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "cache-ready":
            // App can now function offline
            logger.info("Offline cache ready")
        }
    }
}
```

## Startup Time Optimization

### iOS Launch Time

```swift
// AppDelegate.swift
@main
class AppDelegate: UIResponder, UIApplicationDelegate {

    var window: UIWindow?

    func application(_ application: UIApplication,
                     didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {

        // Start timing
        LaunchMetrics.shared.start()

        // Show launch screen
        showLaunchScreen()

        // Initialize critical services in background
        initializeServicesInBackground()

        return true
    }

    private func initializeServicesInBackground() {
        DispatchQueue.global(qos: .userInitiated).async {
            // Preload WebView
            WebViewPool.shared.preload()

            // Initialize Strada
            Strada.preconfigure()

            // Load user preferences
            UserPreferences.shared.load()

            // After initialization, show main app
            DispatchQueue.main.async {
                self.showMainApp()
            }
        }
    }
}

// LaunchMetrics.swift
class LaunchMetrics {

    static let shared = LaunchMetrics()
    private var startTime: CFAbsoluteTime = 0

    func start() {
        startTime = CFAbsoluteTimeGetCurrent()
    }

    func report(_ milestone: String) {
        let elapsed = CFAbsoluteTimeGetCurrent() - startTime
        print("Launch: \(milestone) - \(elapsed * 1000)ms")
    }
}
```

### Android Launch Time

```kotlin
// MainActivity.kt
class MainActivity : AppCompatActivity() {

    private lateinit var startupMetrics: StartupMetrics

    override fun onCreate(savedInstanceState: Bundle?) {
        startupMetrics = StartupMetrics()
        startupMetrics.start()

        super.onCreate(savedInstanceState)

        // Show initial UI quickly
        setContentView(R.layout.activity_main_skeleton)

        // Defer heavy initialization
        lifecycleScope.launch(Dispatchers.IO) {
            // Initialize WebView
            WebViewProvider.preload(this@MainActivity)

            // Load essential data
            preloadEssentialData()

            withContext(Dispatchers.Main) {
                showMainContent()
                startupMetrics.report("content_visible")
            }
        }
    }
}

// StartupMetrics.kt
class StartupMetrics {

    private var startTime: Long = 0

    fun start() {
        startTime = System.currentTimeMillis()
    }

    fun report(milestone: String) {
        val elapsed = System.currentTimeMillis() - startTime
        Log.d("StartupMetrics", "$milestone: ${elapsed}ms")

        // Log to analytics
        FirebaseAnalytics.logEvent("startup_milestone", bundleOf(
            "milestone" to milestone,
            "duration_ms" to elapsed
        ))
    }
}
```

## Rendering Performance

### iOS ScrollView Optimization

```swift
// Disable scroll anchor to prevent jank
class OptimizedWebView: WKWebView {

    override func layoutSubviews() {
        super.layoutSubviews()

        // Prevent scroll position jumps
        if #available(iOS 15.0, *) {
            scrollView.scrollAnchorData = nil
        }
    }
}

// Use CADisplayLink for smooth animations
class SmoothAnimationManager {

    private var displayLink: CADisplayLink?
    private var animations: [(Double) -> Void] = []

    func start() {
        displayLink = CADisplayLink(target: self, selector: #selector(update))
        displayLink?.add(to: .main, forMode: .default)
    }

    @objc private func update(_ link: CADisplayLink) {
        let progress = link.duration
        animations.forEach { $0(progress) }
    }
}
```

### Android RecyclerView for Lists

```kotlin
// Instead of WebView lists, use native RecyclerView
class HybridListFragment : Fragment() {

    private lateinit var recyclerView: RecyclerView
    private lateinit var adapter: WebDataAdapter

    override fun onCreateView(): View {
        return RecyclerView(requireContext()).apply {
            layoutManager = LinearLayoutManager(context)
            adapter = WebDataAdapter()
        }
    }

    // Receive data from web via Strada
    class PageComponent : BridgeComponent() {

        override fun onReceive(message: Message) {
            when (message.event) {
                "render-list" -> {
                    val data: ListData = message.data()
                    adapter.submitList(data.items)
                }
            }
        }
    }
}
```

## Network Optimization

### Request Batching

```typescript
// Web: Batch API requests
class RequestBatcher {

    private queue: Request[] = []
    private timer: Timer | null = null

    enqueue(request: Request) {
        this.queue.push(request)

        if (!this.timer) {
            this.timer = setTimeout(() => this.flush(), 100)
        }
    }

    flush() {
        if (this.queue.length === 0) return

        // Send all requests as single batch
        const batch = { requests: this.queue }
        this.queue = []

        // Send to native for processing or directly to API
        Strada.web.send({
            component: 'network',
            event: 'batch-request',
            data: batch
        })
    }
}
```

```swift
// iOS: Handle batched requests
class NetworkComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "batch-request":
            let data: BatchRequestData? = message.data()
            executeBatchRequests(data?.requests ?? [])
        }
    }

    private func executeBatchRequests(_ requests: [RequestData]) {
        // Execute in parallel
        let group = DispatchGroup()

        for request in requests {
            group.enter()

            URLSession.shared.dataTask(with: request.toURLRequest()) { data, response, error in
                // Reply to web with result
                self.reply(to: request.id, data: response)
                group.leave()
            }.resume()
        }

        group.notify(queue: .main) {
            self.reply(to: "batch-complete", data: [:])
        }
    }
}
```

### Connection Coalescing

```kotlin
// Android: Use HTTP/2 for connection coalescing
class NetworkModule {

    fun provideHttpClient(): OkHttpClient {
        return OkHttpClient.Builder()
            .protocols(listOf(Protocol.HTTP_2, Protocol.HTTP_1_1))
            .connectionPool(ConnectionPool(
                maxIdleConnections = 5,
                keepAliveDuration = 5,
                timeUnit = TimeUnit.MINUTES
            ))
            .build()
    }
}
```

## Summary

Key performance optimizations:

1. **WebView Preloading** - Pool WebViews, preload during launch
2. **Memory Management** - Handle warnings, clear caches
3. **HTTP Caching** - Aggressive caching for static assets
4. **Service Workers** - Offline-first strategy
5. **Startup Time** - Defer non-critical initialization
6. **Rendering** - Native lists, smooth animations
7. **Network** - Request batching, HTTP/2

---

*Related: `offline-connectivity-exploration.md`, `security-exploration.md`*
