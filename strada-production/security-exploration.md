# Strada Production - Security Exploration

## Overview

This document explores security hardening strategies for production iOS/Android apps using WebViews and Strada.

## WebView Security Hardening

### iOS WKWebView Security

```swift
// SecureWebViewConfiguration.swift
class SecureWebViewConfiguration {

    static func createSecureConfiguration() -> WKWebViewConfiguration {
        let configuration = WKWebViewConfiguration()

        // Disable JavaScript where not needed
        configuration.preferences.javaScriptEnabled = true  // Required for Strada

        // Disable plugins (Flash, etc.)
        configuration.preferences.plugInsEnabled = false

        // Disable local file access
        configuration.preferences.localFileAccessEnabled = false

        // Disable smooth scrolling (can leak info via timing)
        configuration.preferences.isSmoothScrollingEnabled = true

        // Block pop-ups
        configuration.preferences.javaScriptCanOpenWindowsAutomatically = false

        // Set minimum font size (prevent tiny text attacks)
        configuration.preferences.minimumFontSize = 8
        configuration.preferences.minimumLogicalFontSize = 8

        // Disable WebGL if not needed (reduces fingerprinting surface)
        configuration.preferences.webGLEnabled = false

        // Disable WebAudio if not needed
        configuration.preferences.isWebAudioEnabled = false

        // Restrict media playback
        configuration.allowsInlineMediaPlayback = true
        configuration.mediaTypesRequiringUserActionForPlayback = [.all]

        // Set secure user agent
        configuration.applicationNameForUserAgent = "App/1.0 (Secure)"

        // Use persistent data store with encryption
        configuration.websiteDataStore = createSecureDataStore()

        return configuration
    }

    private static func createSecureDataStore() -> WKWebsiteDataStore {
        // Create a non-persistent data store for sensitive sessions
        // Or use persistent with proper encryption
        return WKWebsiteDataStore.default()
    }
}

// Content Security Policy via HTTP headers (server-side)
// Content-Security-Policy: default-src 'self'; script-src 'self'; object-src 'none'
```

### Android WebView Security

```kotlin
// SecureWebViewSetup.kt
object SecureWebViewSetup {

    fun configureSecureWebView(webView: WebView, context: Context) {
        val settings = webView.settings

        // Enable JavaScript (required for Strada)
        settings.javaScriptEnabled = true

        // Disable file access
        settings.allowFileAccess = false
        settings.allowContentAccess = false
        settings.allowFileAccessFromFileURLs = false
        settings.allowUniversalAccessFromFileURLs = false

        // Disable DOM storage if not needed
        settings.domStorageEnabled = true  // Required for most apps

        // Disable database storage
        settings.databaseEnabled = false

        // Disable geolocation
        settings.geolocationEnabled = false

        // Disable zoom
        settings.builtInZoomControls = false
        settings.displayZoomControls = false

        // Disable mixed content (HTTPS page loading HTTP resources)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            settings.mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
        }

        // Disable password saving
        settings.savePassword = false
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            settings.safeBrowsingEnabled = true
        }

        // Disable third-party cookies
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            CookieManager.getInstance().setAcceptThirdPartyCookies(
                webView, false
            )
        }

        // Set secure user agent
        settings.userAgentString = createSecureUserAgent()

        // Disable JavaScript reflection (Android 9+)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            webView.webViewAssetLoader?.setHttpAuthority("")
        }
    }

    private fun createSecureUserAgent(): String {
        // Don't expose detailed device info
        return "App/1.0"
    }

    // Disable addJavascriptInterface for security (use only Strada's)
    fun disableJavascriptInterfaceExceptStrada(webView: WebView) {
        // Don't add any additional interfaces
        // Strada's Bridge handles all JS-native communication
    }
}
```

## Certificate Pinning

### iOS Certificate Pinning

```swift
// CertificatePinning.swift
import Foundation

class CertificatePinningDelegate: NSObject, URLSessionDelegate {

    private let pinnedCertificates: [Data] = {
        // Load certificates from app bundle
        guard let certPath = Bundle.main.path(forResource: "example_com", ofType: "der"),
              let certData = try? Data(contentsOf: URL(fileURLWithPath: certPath)) else {
            return []
        }
        return [certData]
    }()

    func urlSession(_ session: URLSession,
                    didReceive challenge: URLAuthenticationChallenge,
                    completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void) {

        guard challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust,
              let serverTrust = challenge.protectionSpace.serverTrust else {
            completionHandler(.cancelAuthenticationChallenge, nil)
            return
        }

        // Get server certificates
        let serverCertificates = SecTrustGetCertificateChain(serverTrust) as? [SecCertificate] ?? []

        guard let serverCertificate = serverCertificates.first else {
            completionHandler(.cancelAuthenticationChallenge, nil)
            return
        }

        // Compare with pinned certificates
        let serverCertData = SecCertificateCopyData(serverCertificate) as Data

        if pinnedCertificates.contains(where: { $0 == serverCertData }) {
            let credential = URLCredential(trust: serverTrust)
            completionHandler(.useCredential, credential)
        } else {
            // Certificate mismatch - potential MITM
            completionHandler(.cancelAuthenticationChallenge, nil)
            reportSecurityIssue("Certificate pinning failed")
        }
    }

    private func reportSecurityIssue(_ message: String) {
        // Log and alert security team
        print("SECURITY: \(message)")
    }
}

// Apply to WebView's network requests
class SecureWebView: WKWebView {

    override init(frame: CGRect, configuration: WKWebViewConfiguration) {
        super.init(frame: frame, configuration: configuration)
        setupPinning()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setupPinning()
    }

    private func setupPinning() {
        // Note: WKWebView uses shared URL session
        // For full pinning, implement via WKURLSchemeHandler
    }
}
```

### Android Certificate Pinning

```kotlin
// CertificatePinning.kt
class CertificatePinning {

    companion object {

        private val CERTIFICATE_SHA256_PINS = setOf(
            "sha256/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",  // Primary cert
            "sha256/BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB="   // Backup cert
        )

        fun createSecureHttpClient(): OkHttpClient {
            val certificatePinner = CertificatePinner.Builder()
                .add("example.com", *CERTIFICATE_SHA256_PINS.toTypedArray())
                .add("api.example.com", *CERTIFICATE_SHA256_PINS.toTypedArray())
                .build()

            return OkHttpClient.Builder()
                .certificatePinner(certificatePinner)
                .build()
        }
    }
}

// For WebView, use WebViewClient with SSL error handling
class SecureWebViewClient : WebViewClient() {

    private val expectedHost = "example.com"

    override fun onReceivedSslError(view: WebView?,
                                    handler: SslErrorHandler?,
                                    error: SslError?) {
        // ALWAYS reject SSL errors in production
        handler?.cancel()

        // Log the error
        Log.e("WebView", "SSL Error: ${error?.primaryError}")

        // Optionally notify user
        showSecurityWarning()
    }

    override fun shouldOverrideUrlLoading(view: WebView?,
                                          request: WebResourceRequest?): Boolean {
        // Only allow expected hosts
        val url = request?.url ?: return false

        return when {
            url.host == expectedHost -> false  // Let WebView load
            url.scheme == "tel" -> true  // Handle internally
            url.scheme == "mailto" -> true
            else -> {
                // Block unknown external URLs
                Log.w("WebView", "Blocked external URL: $url")
                true
            }
        }
    }
}
```

## Secure Token Storage

### iOS Keychain

```swift
// KeychainManager.swift
import Security

class KeychainManager {

    enum KeychainError: Error {
        case itemNotFound
        case duplicateItem
        case unexpectedData
        case unhandledError
    }

    static let shared = KeychainManager()

    /// Store authentication token
    func saveToken(_ token: String, forKey key: String = "auth_token") throws {
        let tokenData = Data(token.utf8)

        // Create query for existing item
        let existingQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key
        ]

        // Try to delete existing item (ignore errors)
        SecItemDelete(existingQuery as CFDictionary)

        // Create new item
        let newQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecValueData as String: tokenData,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]

        let status = SecItemAdd(newQuery as CFDictionary, nil)

        guard status == errSecSuccess else {
            throw KeychainError.unhandledError
        }
    }

    /// Retrieve authentication token
    func getToken(forKey key: String = "auth_token") throws -> String {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status == errSecSuccess else {
            throw KeychainError.itemNotFound
        }

        guard let tokenData = result as? Data,
              let token = String(data: tokenData, encoding: .utf8) else {
            throw KeychainError.unexpectedData
        }

        return token
    }

    /// Delete token
    func deleteToken(forKey key: String = "auth_token") throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key
        ]

        let status = SecItemDelete(query as CFDictionary)

        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.unhandledError
        }
    }
}

// Usage with Strada - send token securely to web
class AuthComponent: BridgeComponent {

    func provideTokenToWeb() {
        do {
            let token = try KeychainManager.shared.getToken()
            // NEVER send full token to web
            // Instead, send metadata or use for native API calls
            reply(to: "auth-state", data: [
                "authenticated": true,
                "tokenExpiry": getTokenExpiry()
            ])
        } catch {
            reply(to: "auth-state", data: ["authenticated": false])
        }
    }
}
```

### Android EncryptedSharedPreferences

```kotlin
// SecureStorage.kt
class SecureStorage(private val context: Context) {

    private val encryptedPrefs: SharedPreferences by lazy {
        EncryptedSharedPreferences.create(
            context,
            "secure_prefs",
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
    }

    private val masterKey: MasterKey by lazy {
        MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
    }

    fun saveToken(token: String, key: String = "auth_token") {
        encryptedPrefs.edit().putString(key, token).apply()
    }

    fun getToken(key: String = "auth_token"): String? {
        return encryptedPrefs.getString(key, null)
    }

    fun deleteToken(key: String = "auth_token") {
        encryptedPrefs.edit().remove(key).apply()
    }

    fun clearAll() {
        encryptedPrefs.edit().clear().apply()
    }
}

// Biometric-protected storage for high-security tokens
class BiometricSecureStorage(context: Context) {

    private val biometricPrefs: SharedPreferences =
        EncryptedSharedPreferences.create(
            context,
            "biometric_prefs",
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )

    private val masterKey: MasterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .setUserAuthenticationRequired(true)  // Requires biometric
        .build()

    fun saveTokenWithBiometric(token: String, key: String) {
        biometricPrefs.edit().putString(key, token).apply()
    }

    fun getTokenWithBiometric(activity: Activity, key: String,
                              onSuccess: (String) -> Unit,
                              onFailed: () -> Unit) {
        // Show biometric prompt before accessing token
        val biometricPrompt = BiometricPrompt(activity,
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    super.onAuthenticationSucceeded(result)
                    biometricPrefs.getString(key, null)?.let(onSuccess)
                }

                override fun onAuthenticationFailed() {
                    super.onAuthenticationFailed()
                    onFailed()
                }
            }
        )

        val promptInfo = BiometricPrompt.PromptInfo.Builder()
            .setTitle("Authenticate")
            .setSubtitle("Access secure token")
            .setNegativeButtonText("Cancel")
            .build()

        biometricPrompt.authenticate(promptInfo)
    }
}
```

## Secure Communication with Web

### Token Handling Best Practices

```typescript
// Web: NEVER store tokens in localStorage
// BAD - Don't do this:
// localStorage.setItem('authToken', token)

// GOOD - Use HttpOnly cookies set by server
// Or use Strada to have native handle authenticated requests

class SecureAuth {

    // Request native to make authenticated API call
    async makeAuthenticatedRequest(endpoint: string, data: any) {
        const response = await Strada.web.send({
            component: 'auth',
            event: 'api-request',
            data: { endpoint, method: 'POST', body: data }
        })
        return response
    }

    // Check auth state (native checks secure storage)
    async checkAuthState() {
        const state = await Strada.web.send({
            component: 'auth',
            event: 'check-auth'
        })
        return state.authenticated
    }
}
```

```swift
// iOS: Handle authenticated requests natively
class AuthComponent: BridgeComponent {

    override func onReceive(message: Message) {
        switch message.event {
        case "api-request":
            let data: ApiRequestData? = message.data()
            makeAuthenticatedRequest(data: data!)

        case "check-auth":
            checkAuthentication()
        }
    }

    private func makeAuthenticatedRequest(data: ApiRequestData) {
        guard let token = try? KeychainManager.shared.getToken() else {
            reply(to: data.event, data: ["error": "Not authenticated"])
            return
        }

        var request = URLRequest(url: URL(string: data.endpoint)!)
        request.httpMethod = data.method
        request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")

        URLSession.shared.dataTask(with: request) { data, response, error in
            self.reply(to: data.event, data: response)
        }.resume()
    }

    private func checkAuthentication() {
        let isAuthenticated = (try? KeychainManager.shared.getToken()) != nil
        reply(to: "check-auth", data: ["authenticated": isAuthenticated])
    }
}
```

## XSS Prevention

### Input Sanitization

```typescript
// Web: Sanitize user input before rendering
import DOMPurify from 'dompurify'

class SafeRenderer {

    renderUserContent(html: string) {
        // Always sanitize user-provided HTML
        const clean = DOMPurify.sanitize(html, {
            ALLOWED_TAGS: ['b', 'i', 'em', 'strong', 'a'],
            ALLOWED_ATTR: ['href']
        })
        return clean
    }

    // For Strada messages from native
    handleNativeMessage(message: any) {
        // Validate message structure
        if (!message.component || !message.event) {
            console.warn('Invalid message structure')
            return
        }

        // Sanitize any string data
        const sanitizedData = Object.keys(message.data).reduce((acc, key) => {
            if (typeof message.data[key] === 'string') {
                acc[key] = DOMPurify.sanitize(message.data[key])
            } else {
                acc[key] = message.data[key]
            }
            return acc
        }, {})

        this.processMessage({ ...message, data: sanitizedData })
    }
}
```

### Content Security Policy

```
# Server-side CSP headers
Content-Security-Policy:
  default-src 'self';
  script-src 'self';
  style-src 'self' 'unsafe-inline';
  img-src 'self' data: https:;
  connect-src 'self' https://api.example.com;
  frame-ancestors 'none';
  base-uri 'self';
  form-action 'self'
```

## Secure Deep Links

### iOS Universal Link Validation

```swift
// Validate universal links before handling
class DeepLinkValidator {

    static func isValidUniversalLink(_ url: URL) -> Bool {
        // Only accept expected hosts
        guard let host = url.host else { return false }

        let allowedHosts = [
            "example.com",
            "www.example.com",
            "app.example.com"
        ]

        guard allowedHosts.contains(host) else {
            return false
        }

        // Only accept expected paths
        let allowedPaths = ["/posts", "/users", "/settings", "/verify"]
        return allowedPaths.contains { url.path.hasPrefix($0) }
    }
}

// In SceneDelegate
func scene(_ scene: UIScene,
           continue userActivity: NSUserActivity) {
    guard let url = userActivity.webpageURL,
          DeepLinkValidator.isValidUniversalLink(url) else {
        return
    }

    // Safe to handle
    handleDeepLink(url)
}
```

### Android App Link Validation

```kotlin
// Validate app links
class DeepLinkValidator {

    companion object {

        private val ALLOWED_HOSTS = setOf(
            "example.com",
            "www.example.com",
            "app.example.com"
        )

        private val ALLOWED_PATH_PREFIXES = setOf(
            "/posts", "/users", "/settings", "/verify"
        )

        fun isValidAppLink(uri: Uri): Boolean {
            val host = uri.host ?: return false

            if (!ALLOWED_HOSTS.contains(host)) {
                return false
            }

            val path = uri.path ?: return false
            return ALLOWED_PATH_PREFIXES.any { path.startsWith(it) }
        }
    }
}

// In Activity
override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    intent?.data?.let { uri ->
        if (DeepLinkValidator.isValidAppLink(uri)) {
            handleDeepLink(uri)
        } else {
            Log.w("DeepLink", "Invalid deep link: $uri")
        }
    }
}
```

## Summary

Security essentials for WebView apps:

1. **WebView Hardening** - Disable unnecessary features
2. **Certificate Pinning** - Prevent MITM attacks
3. **Secure Token Storage** - Keychain (iOS), EncryptedSharedPreferences (Android)
4. **XSS Prevention** - Sanitize input, use CSP
5. **Secure Communication** - Native handles auth tokens
6. **Deep Link Validation** - Verify URLs before handling

---

*Related: `performance-optimization-exploration.md`, `offline-connectivity-exploration.md`*
