# Strada Production - Offline & Connectivity Exploration

## Overview

This document explores offline-first strategies and connectivity handling for production iOS/Android apps using WebViews and Strada.

## Offline Detection

### iOS Network Reachability

```swift
// NetworkMonitor.swift
import Network

class NetworkMonitor {

    static let shared = NetworkMonitor()

    private let monitor = NWPathMonitor()
    private let queue = DispatchQueue(label: "NetworkMonitor")

    var isConnected: Bool { true }
    var isExpensive: Bool { false }  // Cellular vs WiFi
    var connectivityType: String { "unknown" }

    private var listeners: [(Bool) -> Void] = []

    func startMonitoring() {
        monitor.pathUpdateHandler = { [weak self] path in
            self?.handlePathUpdate(path)
        }
        monitor.start(queue: queue)
    }

    private func handlePathUpdate(_ path: NWPath) {
        let wasConnected = isConnected
        isConnected = path.status == .satisfied
        isExpensive = path.isExpensive

        if path.usesInterfaceType(.wifi) {
            connectivityType = "wifi"
        } else if path.usesInterfaceType(.cellular) {
            connectivityType = "cellular"
        } else {
            connectivityType = "unknown"
        }

        // Notify listeners of change
        if wasConnected != isConnected {
            notifyListeners(isConnected)
        }
    }

    func addListener(_ callback: @escaping (Bool) -> Void) {
        listeners.append(callback)
    }

    private func notifyListeners(_ connected: Bool) {
        DispatchQueue.main.async {
            self.listeners.forEach { $0(connected) }
        }
    }
}

// PageComponent - Notify web of connectivity changes
class PageComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "check-connectivity":
            checkConnectivity()
        }
    }

    override func onViewDidLoad() {
        super.onViewDidLoad()

        // Start monitoring
        NetworkMonitor.shared.startMonitoring()

        // Listen for changes
        NetworkMonitor.shared.addListener { [weak self] isConnected in
            self?.notifyWebOfConnectivityChange(isConnected)
        }
    }

    private func checkConnectivity() {
        let data = [
            "connected": NetworkMonitor.shared.isConnected,
            "type": NetworkMonitor.shared.connectivityType,
            "expensive": NetworkMonitor.shared.isExpensive
        ]
        reply(to: "check-connectivity", data: data)
    }

    private func notifyWebOfConnectivityChange(_ connected: Bool) {
        // Send unsolicited message to web
        delegate?.reply(with: Message(
            id: UUID().uuidString,
            component: "connectivity",
            event: "connectivity-changed",
            metadata: nil,
            jsonData: try! JSONSerialization.data(withJSONObject: [
                "connected": connected
            ]).utf8String ?? "{}"
        ))
    }
}
```

### Android Connectivity Manager

```kotlin
// NetworkMonitor.kt
class NetworkMonitor @Inject constructor(
    private val context: Context
) {

    private val connectivityManager = context.getSystemService(
        Context.CONNECTIVITY_SERVICE
    ) as ConnectivityManager

    private val listeners = mutableListOf<(Boolean) -> Unit>()

    var isConnected: Boolean = false
        private set

    private val networkCallback = object : ConnectivityManager.NetworkCallback() {

        override fun onAvailable(network: Network) {
            val wasConnected = isConnected
            isConnected = true
            if (wasConnected != isConnected) {
                notifyListeners(isConnected)
            }
        }

        override fun onLost(network: Network) {
            val wasConnected = isConnected
            isConnected = connectivityManager.activeNetwork != null
            if (wasConnected != isConnected) {
                notifyListeners(isConnected)
            }
        }

        override fun onCapabilitiesChanged(
            network: Network,
            networkCapabilities: NetworkCapabilities
        ) {
            // Check if network is still valid
            isConnected = networkCapabilities.hasCapability(
                NetworkCapabilities.NET_CAPABILITY_INTERNET
            ) && networkCapabilities.hasCapability(
                NetworkCapabilities.NET_CAPABILITY_VALIDATED
            )
        }
    }

    fun startMonitoring() {
        connectivityManager.registerDefaultNetworkCallback(networkCallback)

        // Initial check
        isConnected = connectivityManager.activeNetwork != null
    }

    fun stopMonitoring() {
        connectivityManager.unregisterNetworkCallback(networkCallback)
    }

    fun addListener(callback: (Boolean) -> Unit) {
        listeners.add(callback)
    }

    private fun notifyListeners(connected: Boolean) {
        listeners.forEach { it(connected) }
    }

    fun getConnectivityInfo(): Map<String, Any> {
        val network = connectivityManager.activeNetwork
        val capabilities = network?.let { connectivityManager.getNetworkCapabilities(it) }

        return mapOf(
            "connected" to isConnected,
            "type" to getNetworkType(capabilities),
            "expensive" to (capabilities?.hasCapability(
                NetworkCapabilities.NET_CAPABILITY_NOT_METERED
            ) == false)
        )
    }

    private fun getNetworkType(capabilities: NetworkCapabilities?): String {
        return when {
            capabilities?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true -> "wifi"
            capabilities?.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) == true -> "cellular"
            else -> "unknown"
        }
    }
}

// PageComponent - Notify web
class PageComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override fun onReceive(message: Message) {
        when (message.event) {
            "check-connectivity" -> {
                val info = networkMonitor.getConnectivityInfo()
                replyTo("check-connectivity", data = info)
            }
        }
    }

    override fun onStart() {
        super.onStart()
        networkMonitor.startMonitoring()

        networkMonitor.addListener { connected ->
            replyTo("connectivity-changed", data = mapOf("connected" to connected))
        }
    }

    override fun onStop() {
        super.onStop()
        networkMonitor.stopMonitoring()
    }
}
```

## Offline Queue for Actions

### Web Queue Implementation

```typescript
// offline-queue.ts
interface QueuedAction {
    id: string
    type: string
    payload: any
    timestamp: number
    retryCount: number
}

class OfflineQueue {

    private queue: QueuedAction[] = []
    private readonly DB_NAME = 'offline-queue'
    private readonly STORE_NAME = 'actions'

    async initialize() {
        // Load persisted queue from IndexedDB
        this.queue = await this.loadFromIndexedDB()

        // Listen for connectivity changes
        window.addEventListener('online', () => this.processQueue())
        window.addEventListener('offline', () => this.persistQueue())
    }

    async enqueue(action: Omit<QueuedAction, 'id' | 'timestamp' | 'retryCount'>) {
        const queuedAction: QueuedAction = {
            ...action,
            id: crypto.randomUUID(),
            timestamp: Date.now(),
            retryCount: 0
        }

        this.queue.push(queuedAction)
        await this.persistQueue()

        // If online, try to process immediately
        if (navigator.onLine) {
            this.processQueue()
        } else {
            // Notify native of queued action
            Strada.web.send({
                component: 'offline',
                event: 'action-queued',
                data: {
                    type: action.type,
                    queueLength: this.queue.length
                }
            })
        }
    }

    async processQueue() {
        if (!navigator.onLine || this.queue.length === 0) return

        const failedActions: QueuedAction[] = []

        for (const action of this.queue) {
            try {
                await this.executeAction(action)
                // Remove from queue on success
                this.queue = this.queue.filter(a => a.id !== action.id)
            } catch (error) {
                // Increment retry count
                action.retryCount++
                if (action.retryCount >= 3) {
                    // Mark as failed after 3 retries
                    this.handleFailedAction(action)
                } else {
                    failedActions.push(action)
                }
            }
        }

        this.queue = failedActions
        await this.persistQueue()

        // Notify native of queue status
        this.updateNativeQueueStatus()
    }

    private async executeAction(action: QueuedAction) {
        // Send to native for processing
        const result = await Strada.web.send({
            component: 'offline',
            event: 'execute-action',
            data: action
        })

        if (!result.success) {
            throw new Error('Action failed')
        }
    }

    private async persistQueue() {
        // Save to IndexedDB
        const db = await this.openDB()
        const tx = db.transaction(this.STORE_NAME, 'readwrite')
        const store = tx.objectStore(this.STORE_NAME)

        await store.clear()
        for (const action of this.queue) {
            await store.put(action)
        }
    }

    private async loadFromIndexedDB(): Promise<QueuedAction[]> {
        try {
            const db = await this.openDB()
            const tx = db.transaction(this.STORE_NAME, 'readonly')
            const store = tx.objectStore(this.STORE_NAME)
            return await store.getAll()
        } catch {
            return []
        }
    }

    private openDB(): Promise<IDBDatabase> {
        return new Promise((resolve, reject) => {
            const request = indexedDB.open(this.DB_NAME, 1)
            request.onerror = () => reject(request.error)
            request.onsuccess = () => resolve(request.result)
            request.onupgradeneeded = (event) => {
                const db = (event.target as IDBOpenDBRequest).result
                if (!db.objectStoreNames.contains(this.STORE_NAME)) {
                    db.createObjectStore(this.STORE_NAME, { keyPath: 'id' })
                }
            }
        })
    }

    private updateNativeQueueStatus() {
        Strada.web.send({
            component: 'offline',
            event: 'queue-status',
            data: {
                queueLength: this.queue.length,
                hasPendingActions: this.queue.length > 0
            }
        })
    }

    private handleFailedAction(action: QueuedAction) {
        // Notify user of failed action
        Strada.web.send({
            component: 'offline',
            event: 'action-failed',
            data: {
                type: action.type,
                id: action.id
            }
        })
    }
}

// Usage in app
const offlineQueue = new OfflineQueue()
offlineQueue.initialize()

// Queue an action when offline
async function submitForm(data: FormData) {
    if (!navigator.onLine) {
        await offlineQueue.enqueue({
            type: 'form-submit',
            payload: data
        })
        showOfflineNotice()
    } else {
        await submitToServer(data)
    }
}
```

### iOS Offline Queue Handler

```swift
// OfflineQueueComponent.swift
class OfflineQueueComponent: BridgeComponent {

    override var name: String { "offline" }

    private let apiClient: APIClient

    override func onReceive(message: Message) {
        switch message.event {
        case "execute-action":
            let data: ActionData? = message.data()
            executeAction(data: data!)

        case "get-queue-status":
            getQueueStatus()
        }
    }

    private func executeAction(data: ActionData) {
        switch data.type {
        case "form-submit":
            submitForm(data.payload)
        case "sync-request":
            performSync(data.payload)
        default:
            reply(to: data.id, data: ["success": false, "error": "Unknown action type"])
        }
    }

    private func submitForm(_ payload: [String: Any]) {
        apiClient.post(endpoint: "/submit", body: payload) { [weak self] result in
            switch result {
            case .success(let response):
                self?.reply(to: "execute-action", data: [
                    "success": true,
                    "response": response
                ])
            case .failure(let error):
                self?.reply(to: "execute-action", data: [
                    "success": false,
                    "error": error.localizedDescription
                ])
            }
        }
    }

    private func getQueueStatus() {
        // Query web for current queue state
        webView.evaluateJavaScript("window.offlineQueue.getQueueStatus()") { [weak self] result, _ in
            if let status = result as? [String: Any] {
                // Update native UI with queue status
                self?.updateQueueBadge(status["queueLength"] as? Int ?? 0)
            }
        }
    }

    private func updateQueueBadge(_ count: Int) {
        // Update badge on tab bar or toolbar
        if count > 0 {
            destination.tabBarItem.badgeValue = "\(count)"
        } else {
            destination.tabBarItem.badgeValue = nil
        }
    }
}
```

### Android Offline Queue Handler

```kotlin
// OfflineQueueComponent.kt
class OfflineQueueComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override val name: String = "offline"

    @Inject lateinit var apiClient: APIClient
    @Inject lateinit var queueRepository: QueueRepository

    override fun onReceive(message: Message) {
        when (message.event) {
            "execute-action" -> {
                val data: ActionData? = message.data()
                executeAction(data!!)
            }
            "get-queue-status" -> {
                getQueueStatus()
            }
        }
    }

    private fun executeAction(data: ActionData) {
        lifecycleScope.launch {
            try {
                val result = when (data.type) {
                    "form-submit" -> apiClient.submitForm(data.payload)
                    "sync-request" -> apiClient.performSync(data.payload)
                    else -> throw IllegalArgumentException("Unknown action: ${data.type}")
                }

                replyTo("execute-action", data = mapOf(
                    "success" to true,
                    "response" to result
                ))

                // Remove from queue
                queueRepository.removeAction(data.id)

            } catch (e: Exception) {
                replyTo("execute-action", data = mapOf(
                    "success" to false,
                    "error" to e.message
                ))
            }
        }
    }

    private fun getQueueStatus() {
        lifecycleScope.launch {
            val queueLength = queueRepository.getQueueLength()
            updateQueueBadge(queueLength)
        }
    }

    private fun updateQueueBadge(count: Int) {
        // Update badge in UI
        val activity = destination.requireActivity() as? MainActivity
        activity?.updateQueueBadge(count)
    }
}

// Room database for queue persistence
@Entity(tableName = "offline_actions")
data class OfflineActionEntity(
    @PrimaryKey val id: String,
    val type: String,
    val payload: String,  // JSON string
    val timestamp: Long,
    val retryCount: Int
)

@Dao
interface OfflineActionDao {

    @Query("SELECT * FROM offline_actions ORDER BY timestamp ASC")
    suspend fun getAllActions(): List<OfflineActionEntity>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertAction(action: OfflineActionEntity)

    @Delete
    suspend fun deleteAction(action: OfflineActionEntity)

    @Query("SELECT COUNT(*) FROM offline_actions")
    suspend fun getQueueLength(): Int
}
```

## Service Worker Coordination

### Service Worker Registration

```typescript
// sw.js - Service Worker with offline caching
const CACHE_VERSION = 'v1'
const STATIC_CACHE = `static-${CACHE_VERSION}`
const DYNAMIC_CACHE = `dynamic-${CACHE_VERSION}`

const STATIC_ASSETS = [
  '/',
  '/offline.html',
  '/styles/main.css',
  '/scripts/app.js'
]

// Install - cache static assets
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(STATIC_CACHE).then((cache) => {
      return cache.addAll(STATIC_ASSETS)
    })
  )
  // Skip waiting to activate immediately
  self.skipWaiting()
})

// Activate - clean old caches
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) => {
      return Promise.all(
        keys.filter((key) => !key.endsWith(CACHE_VERSION))
            .map((key) => caches.delete(key))
      )
    })
  )
  // Take control immediately
  self.clients.claim()
})

// Fetch - network first for API, cache-first for static
self.addEventListener('fetch', (event) => {
  const { request } = event

  // API requests - network first with offline fallback
  if (request.url.includes('/api/')) {
    event.respondWith(fetchWithOfflineFallback(request))
    return
  }

  // Static assets - cache first
  event.respondWith(
    caches.match(request).then((cached) => {
      return cached || fetch(request)
    })
  )
})

async function fetchWithOfflineFallback(request: Request) {
  try {
    const response = await fetch(request)
    return response
  } catch (error) {
    // Network failed - try cache
    const cached = await caches.match(request)
    if (cached) {
      return cached
    }

    // Return offline response
    return caches.match('/offline.html')
  }
}

// Background sync for queued actions
self.addEventListener('sync', (event) => {
  if (event.tag === 'sync-actions') {
    event.waitUntil(syncQueuedActions())
  }
})

async function syncQueuedActions() {
  // Get queued actions from IndexedDB
  const db = await openDB('offline-queue', 1)
  const actions = await db.getAll('actions')

  for (const action of actions) {
    try {
      await fetch('/api/sync', {
        method: 'POST',
        body: JSON.stringify(action)
      })
      // Remove from queue on success
      await db.delete('actions', action.id)
    } catch (error) {
      // Keep in queue for retry
    }
  }

  // Notify clients of sync completion
  const clients = await self.clients.matchAll()
  clients.forEach((client) => {
    client.postMessage({ type: 'SYNC_COMPLETE' })
  })
}
```

### Native Handling of Service Worker Messages

```swift
// iOS: Handle service worker messages
class PageComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "service-worker-message":
            let data: SWMessageData? = message.data()
            handleServiceWorkerMessage(data: data!)
        }
    }

    private func handleServiceWorkerMessage(data: SWMessageData) {
        switch data.type {
        case "CACHE_READY":
            // App can function offline
            logger.info("Service Worker cache ready")
            updateOfflineCapability(true)

        case "SYNC_COMPLETE":
            // Background sync completed
            logger.info("Background sync completed")
            refreshWebContent()

        default:
            break
        }
    }
}
```

```kotlin
// Android: Handle service worker messages
class PageComponent<D : BridgeDestination> : BridgeComponent<D>() {

    override fun onReceive(message: Message) {
        when (message.event) {
            "service-worker-message" -> {
                val data: SWMessageData? = message.data()
                handleServiceWorkerMessage(data!!)
            }
        }
    }

    private fun handleServiceWorkerMessage(data: SWMessageData) {
        when (data.type) {
            "CACHE_READY" -> {
                logger.i("ServiceWorker", "Cache ready for offline")
                updateOfflineCapability(true)
            }
            "SYNC_COMPLETE" -> {
                logger.i("ServiceWorker", "Background sync complete")
                refreshWebContent()
            }
        }
    }
}
```

## Offline UI States

### Offline Indicator Component

```typescript
// Web: Offline indicator
class OfflineIndicator {

    private element: HTMLElement

    show() {
        this.element.style.display = 'block'
        this.element.textContent = 'You are offline. Changes will sync when connected.'
    }

    hide() {
        this.element.style.display = 'none'
    }

    showPendingChanges(count: number) {
        this.element.textContent = `${count} action(s) pending sync`
    }
}

// Listen for connectivity events
window.addEventListener('offline', () => {
    offlineIndicator.show()
    Strada.web.send({
        component: 'offline',
        event: 'went-offline'
    })
})

window.addEventListener('online', () => {
    offlineIndicator.hide()
    Strada.web.send({
        component: 'offline',
        event: 'went-online'
    })
})
```

### Native Offline Banner

```swift
// iOS: Native offline banner
class OfflineComponent: BridgeComponent {

    private var offlineBanner: OfflineBannerView?

    override func onReceive(message: Message) {
        switch message.event {
        case "went-offline":
            showOfflineBanner()
        case "went-online":
            hideOfflineBanner()
        }
    }

    private func showOfflineBanner() {
        let banner = OfflineBannerView()
        banner.message = "You're offline. Changes will sync when connected."

        // Add to view
        if letSuperview = destination.view.superview {
            superview.addSubview(banner)

            // Constraints for bottom banner
            banner.translatesAutoresizingMaskIntoConstraints = false
            NSLayoutConstraint.activate([
                banner.leadingAnchor.constraint(equalTo: superview.leadingAnchor),
                banner.trailingAnchor.constraint(equalTo: superview.trailingAnchor),
                banner.bottomAnchor.constraint(equalTo: superview.safeAreaLayoutGuide.bottomAnchor)
            ])
        }

        offlineBanner = banner
    }

    private func hideOfflineBanner() {
        offlineBanner?.removeFromSuperview()
        offlineBanner = nil
    }
}

// OfflineBannerView.swift
class OfflineBannerView: UIView {

    var message: String = "" {
        didSet { label.text = message }
    }

    private let label: UILabel = {
        let l = UILabel()
        l.textAlignment = .center
        l.backgroundColor = .systemGray
        l.textColor = .white
        l.font = .systemFont(ofSize: 14)
        l.translatesAutoresizingMaskIntoConstraints = false
        return l
    }()

    init() {
        super.init(frame: .zero)
        backgroundColor = .systemGray
        addSubview(label)

        NSLayoutConstraint.activate([
            label.topAnchor.constraint(equalTo: topAnchor, constant: 8),
            label.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -8),
            label.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 16),
            label.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -16)
        ])
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}
```

## Summary

Offline essentials:

1. **Connectivity Detection** - Network monitor on both platforms
2. **Offline Queue** - IndexedDB persistence, native execution
3. **Service Workers** - Cache static assets, network-first for API
4. **Background Sync** - Sync queued actions when online
5. **Offline UI** - Banner/indicator showing offline state
6. **Queue Status** - Badge showing pending actions

---

*Related: `performance-optimization-exploration.md`, `testing-exploration.md`*
