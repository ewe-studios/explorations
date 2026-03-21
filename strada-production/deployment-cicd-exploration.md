# Strada Production - Deployment & CI/CD Exploration

## Overview

This document explores deployment strategies, CI/CD pipelines, and release management for production iOS/Android apps using WebViews and Strada.

## App Store Considerations

### iOS App Store Guidelines for WebView Apps

```markdown
## Key Guidelines (Section 4.2 - Minimum Functionality)

1. **Must provide native functionality**
   - Don't just wrap a website
   - Add native UI components (tabs, navigation, etc.)
   - Use native features (camera, push notifications, etc.)

2. **Must work offline (at least partially)**
   - Show cached content when offline
   - Display meaningful offline state

3. **Must not crash or be buggy**
   - Handle WebView errors gracefully
   - Provide fallback UI

4. **Must not link to external purchase mechanisms**
   - Use StoreKit for digital goods
   - Don't show links to web purchasing
```

### iOS Info.plist Configuration

```xml
<!-- Info.plist -->
<key>CFBundleShortVersionString</key>
<string>1.0.0</string>

<key>CFBundleVersion</key>
<string>1</string>

<!-- App Transport Security -->
<key>NSAppTransportSecurity</key>
<dict>
    <key>NSExceptionDomains</key>
    <dict>
        <key>example.com</key>
        <dict>
            <key>NSIncludesSubdomains</key>
            <true/>
            <key>NSThirdPartyExceptionAllowsInsecureHTTPLoads</key>
            <false/>
            <key>NSExceptionRequiresForwardSecrecy</key>
            <false/>
        </dict>
    </dict>
</dict>

<!-- Universal Links -->
<key>com.apple.developer.associated-domains</key>
<array>
    <string>applinks:example.com</string>
    <string>applinks:www.example.com</string>
</array>

<!-- Privacy Descriptions -->
<key>NSCameraUsageDescription</key>
<string>We need camera access to scan QR codes</string>
<key>NSPhotoLibraryUsageDescription</key>
<string>We need photo library access to upload images</string>
<key>NSLocationWhenInUseUsageDescription</key>
<string>We need your location to show nearby stores</string>
```

### Android Play Store Guidelines

```markdown
## Key Guidelines

1. **Must not be a webview-only app**
   - Must provide additional functionality beyond website
   - Native navigation, offline support, device integration

2. **Must handle WebView errors**
   - Don't show "Web page not available" errors
   - Provide custom error pages

3. **Must comply with JavaScript policies**
   - No hidden JS functionality
   - All features must be visible to user
```

### Android Manifest Configuration

```xml
<!-- AndroidManifest.xml -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.example.app">

    <!-- Network permissions -->
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />

    <!-- Optional permissions based on features -->
    <uses-permission android:name="android.permission.CAMERA" />
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
    <uses-permission android:name="android.permission.ACCESS_FINE_LOCATION" />

    <!-- App Links (for Android 12+) -->
    <application>
        <activity android:name=".MainActivity">

            <!-- Main launcher intent -->
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>

            <!-- App Links -->
            <intent-filter android:autoVerify="true">
                <action android:name="android.intent.action.VIEW" />
                <category android:name="android.intent.category.DEFAULT" />
                <category android:name="android.intent.category.BROWSABLE" />

                <data android:scheme="https"
                      android:host="example.com"
                      android:pathPrefix="/" />
            </intent-filter>

        </activity>
    </application>

</manifest>
```

## CI/CD Pipeline

### GitHub Actions for iOS

```yaml
# .github/workflows/ios-ci.yml
name: iOS CI/CD

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  build-and-test:
    runs-on: macos-14

    steps:
      - uses: actions/checkout@v4

      - name: Set up Ruby
        uses: ruby/setup-ruby@v1
        with:
          ruby-version: '3.2'
          bundler-cache: true
          working-directory: ios

      - name: Install dependencies
        run: bundle install
        working-directory: ios

      - name: Build
        run: |
          xcodebuild build \
            -scheme Boxxed \
            -sdk iphonesimulator \
            -destination 'platform=iOS Simulator,name=iPhone 15,OS=latest' \
            -derivedDataPath build

      - name: Run tests
        run: |
          xcodebuild test \
            -scheme Boxxed \
            -sdk iphonesimulator \
            -destination 'platform=iOS Simulator,name=iPhone 15,OS=latest' \
            -resultBundlePath build/TestResults

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: test-results
          path: build/TestResults

  deploy-to-testflight:
    needs: build-and-test
    if: github.ref == 'refs/heads/main' && github.event_name == 'push'
    runs-on: macos-14

    steps:
      - uses: actions/checkout@v4

      - name: Install Fastlane
        run: gem install fastlane -v 2.217.0

      - name: Build and upload to TestFlight
        env:
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APP_STORE_CONNECT_API_KEY_ID: ${{ secrets.APP_STORE_CONNECT_API_KEY_ID }}
          APP_STORE_CONNECT_API_ISSUER_ID: ${{ secrets.APP_STORE_CONNECT_API_ISSUER_ID }}
          APP_STORE_CONNECT_API_KEY_CONTENT: ${{ secrets.APP_STORE_CONNECT_API_KEY_CONTENT }}
          MATCH_PASSWORD: ${{ secrets.MATCH_PASSWORD }}
          MATCH_GITHUB_URL: ${{ secrets.MATCH_GITHUB_URL }}
        run: |
          fastlane ios beta
```

### GitHub Actions for Android

```yaml
# .github/workflows/android-ci.yml
name: Android CI/CD

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  build-and-test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Set up JDK 17
        uses: actions/setup-java@v4
        with:
          java-version: '17'
          distribution: 'temurin'
          cache: gradle

      - name: Grant execute permission for gradlew
        run: chmod +x gradlew
        working-directory: android

      - name: Build with Gradle
        run: ./gradlew build
        working-directory: android

      - name: Run unit tests
        run: ./gradlew test
        working-directory: android

      - name: Run instrumented tests
        uses: reactivecircus/android-emulator-runner@v2
        with:
          api-level: 34
          script: ./gradlew connectedAndroidTest
          working-directory: android

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: android-test-results
          path: android/build/reports/tests

  deploy-to-play-store:
    needs: build-and-test
    if: github.ref == 'refs/heads/main' && github.event_name == 'push'
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Set up JDK 17
        uses: actions/setup-java@v4
        with:
          java-version: '17'
          distribution: 'temurin'

      - name: Decode keystore
        env:
          KEYSTORE_BASE64: ${{ secrets.KEYSTORE_BASE64 }}
        run: |
          echo $KEYSTORE_BASE64 | base64 --decode > android/app/release-key.jks

      - name: Build release APK
        env:
          KEYSTORE_PASSWORD: ${{ secrets.KEYSTORE_PASSWORD }}
          KEY_ALIAS: ${{ secrets.KEY_ALIAS }}
          KEY_PASSWORD: ${{ secrets.KEY_PASSWORD }}
        run: |
          ./gradlew assembleRelease
        working-directory: android

      - name: Upload to Play Store (Internal)
        uses: r0adkll/upload-google-play@v1
        with:
          serviceAccountJsonPlainText: ${{ secrets.PLAY_STORE_SERVICE_ACCOUNT }}
          packageName: com.example.app
          releaseFiles: android/app/build/outputs/apk/release/app-release.apk
          track: internal
          status: completed
```

### Fastlane Configuration (iOS)

```ruby
# ios/fastlane/Fastfile
default_platform(:ios)

platform :ios do
  desc "Build and upload to TestFlight"
  lane :beta do
    # Increment build number
    increment_build_number(
      scheme: "Boxxed",
      xcodeproj: "Boxxed.xcodeproj"
    )

    # Build app
    build_app(
      scheme: "Boxxed",
      export_method: "app-store",
      export_options: {
        uploadBitcode: false,
        uploadSymbols: true
      }
    )

    # Upload to TestFlight
    upload_to_testflight(
      apple_id: "1234567890",
      username: "appstore@example.com",
      app_identifier: "com.example.boxxed",
      skip_waiting_for_build_processing: false,
      distribute_external: true,
      groups: ["External Testers"]
    )

    # Create GitHub release
    github_release(
      server: "https://api.github.com",
      api_token: ENV["GITHUB_TOKEN"],
      repository: ENV["GITHUB_REPOSITORY"],
      name: "v#{version_number}",
      tag_name: "v#{version_number}",
      draft: false
    )
  end

  desc "Submit to App Store"
  lane :release do
    build_app(
      scheme: "Boxxed",
      export_method: "app-store"
    )

    upload_to_app_store(
      apple_id: "1234567890",
      username: "appstore@example.com",
      skip_waiting_for_build_processing: true
    )
  end
end
```

### Fastlane Configuration (Android)

```ruby
# android/fastlane/Fastfile
default_platform(:android)

platform :android do
  desc "Build and upload to Play Store (Internal)"
  lane :internal do
    # Increment version code
    gradle(
      task: "incrementVersionCode",
      project_dir: "android"
    )

    # Build release
    gradle(
      task: "assembleRelease",
      project_dir: "android",
      properties: {
        "android.injected.signing.store.file" => "app/release-key.jks",
        "android.injected.signing.store.password" => ENV["KEYSTORE_PASSWORD"],
        "android.injected.signing.key.alias" => ENV["KEY_ALIAS"],
        "android.injected.signing.key.password" => ENV["KEY_PASSWORD"]
      }
    )

    # Upload to Play Store
    upload_to_play_store(
      service_account_json: ENV["PLAY_STORE_SERVICE_ACCOUNT_JSON"],
      track: "internal",
      skip_upload_changelog: true,
      skip_upload_screenshots: true
    )
  end

  desc "Promote from Internal to Production"
  lane :promote_to_production do
    upload_to_play_store(
      service_account_json: ENV["PLAY_STORE_SERVICE_ACCOUNT_JSON"],
      track: "production",
      promote_from: "internal",
      release_status: "draft"
    )
  end
end
```

## OTA Updates (Within Guidelines)

### CodePush / App Center

```javascript
// React Native / Web hybrid apps can use CodePush
import { codePush } from 'react-native-code-push';

// Configure update behavior
const codePushOptions = {
  checkFrequency: codePush.CheckFrequency.ON_APP_START,
  installMode: codePush.InstallMode.ON_NEXT_RESTART,
  mandatoryInstallMode: codePush.InstallMode.IMMEDIATE
};

class App extends React.Component {
  componentDidMount() {
    // Sync updates
    codePush.sync(codePushOptions);
  }
}

export default codePush(codePushOptions)(App);
```

### WebView Asset Updates

```swift
// iOS: Check for updated web assets
class AssetUpdateManager {

    static let shared = AssetUpdateManager()

    func checkForUpdates() {
        // Check server for new asset version
        URLSession.shared.dataTask(with: URL(string: "https://example.com/asset-version.json")!) { data, _, _ in
            guard let data = data,
                  let versionInfo = try? JSONDecoder().decode(AssetVersion.self, from: data) else { return }

            let currentVersion = self.getCurrentAssetVersion()

            if versionInfo.version > currentVersion {
                self.downloadAndInstallUpdate(versionInfo)
            }
        }.resume()
    }

    private func downloadAndInstallUpdate(_ versionInfo: AssetVersion) {
        // Download updated JS/CSS bundle
        // Store in app sandbox
        // Update version pointer
    }
}
```

## Feature Flags

### LaunchDarkly Integration

```swift
// iOS: LaunchDarkly for feature flags
import LaunchDarkly

class FeatureFlags {

    static let shared = FeatureFlags()

    private let client = LDClient()

    func configure() {
        let config = LDConfig(mobileKey: "YOUR_MOBILE_KEY")
        let user = LDUser(key: currentUserId)

        client.start(config: config, user: user)
    }

    // Feature flag accessors
    var isNewNavigationEnabled: Bool {
        client.boolVariation(forKey: "ios-new-navigation", defaultValue: false)
    }

    var isStradaV2Enabled: Bool {
        client.boolVariation(forKey: "strada-v2", defaultValue: false)
    }

    var submitButtonColor: String {
        client.stringVariation(forKey: "submit-button-color", defaultValue: "#007AFF")
    }
}
```

```kotlin
// Android: LaunchDarkly
class FeatureFlags @Inject constructor(
    private val ldClient: LDClient
) {

    val isNewNavigationEnabled: Boolean
        get() = ldClient.boolVariation("android-new-navigation", false)

    val isStradaV2Enabled: Boolean
        get() = ldClient.boolVariation("strada-v2", false)

    val submitButtonColor: String
        get() = ldClient.stringVariation("submit-button-color", "#007AFF")
}
```

### Config-Remote (iOS)

```swift
// iOS: Remote Config via JSON endpoint
class RemoteConfig {

    static let shared = RemoteConfig()

    private var config: [String: Any] = [:]

    func fetchConfig() {
        URLSession.shared.dataTask(with: URL(string: "https://example.com/app-config.json")!) { data, _, _ in
            guard let data = data,
                  let config = try? JSONDecoder().decode([String: Any].self, from: data) else { return }

            DispatchQueue.main.async {
                self.config = config
                NotificationCenter.default.post(name: .configDidUpdate, object: nil)
            }
        }.resume()
    }

    func bool(forKey key: String) -> Bool {
        return config[key] as? Bool ?? false
    }

    func string(forKey key: String) -> String? {
        return config[key] as? String
    }
}
```

## A/B Testing

### Google Optimize Integration

```typescript
// Web: Google Optimize for A/B testing
declare global {
  interface Window {
    dataLayer: any[];
  }
}

class ABTesting {

    async getVariant(experimentId: string): Promise<string> {
        return new Promise((resolve) => {
            window.dataLayer = window.dataLayer || [];
            window.dataLayer.push({
                'event': 'optimize.activate',
                'optimize.callback': () => {
                    const variant = window.gapl?.get?.(experimentId, 'variant') || 'A';
                    resolve(variant);
                }
            });
        });
    }

    async testSubmitButtonText(): Promise<string> {
        const variant = await this.getVariant('submit-button-test');

        switch (variant) {
            case 'A': return 'Submit';
            case 'B': return 'Continue';
            case 'C': return 'Next';
            default: return 'Submit';
        }
    }
}
```

### Native A/B Testing

```swift
// iOS: Firebase Remote Config for A/B testing
import FirebaseRemoteConfig

class ABTestingManager {

    static let shared = ABTestingManager()

    private let remoteConfig = RemoteConfig.remoteConfig()

    func configure() {
        let settings = RemoteConfigSettings()
        settings.minimumFetchInterval = 3600  // 1 hour
        remoteConfig.configSettings = settings

        remoteConfig.setDefaults([
            "submit_button_text": "Submit",
            "show_new_onboarding": false
        ])

        fetchAndActivate()
    }

    func fetchAndActivate() {
        remoteConfig.fetchAndActivate { status, _ in
            if status == .successFetched || status == .activatedFromFetch {
                print("Remote config activated")
            }
        }
    }

    var submitButtonText: String {
        remoteConfig["submit_button_text"].stringValue
    }

    var showNewOnboarding: Bool {
        remoteConfig["show_new_onboarding"].boolValue
    }
}
```

## Analytics & Crash Reporting

### Firebase Integration

```swift
// iOS: Firebase setup
import Firebase
import FirebaseAnalytics
import FirebaseCrashlytics

class AnalyticsManager {

    static let shared = AnalyticsManager()

    func configure() {
        FirebaseApp.configure()
    }

    func logEvent(_ name: String, parameters: [String: Any]? = nil) {
        Analytics.logEvent(name, parameters: parameters)
    }

    func setUserId(_ userId: String) {
        Analytics.setUserID(userId)
        Crashlytics.crashlytics().setUserID(userId)
    }

    func logError(_ error: Error) {
        Crashlytics.crashlytics().record(error: error)
    }

    func logWebViewError(_ error: String, url: String) {
        Crashlytics.crashlytics().record(
            errorMessage: error,
            file: url,
            line: 0
        )
    }
}
```

```kotlin
// Android: Firebase setup
class AnalyticsManager @Inject constructor() {

    fun configure(application: Application) {
        FirebaseApp.initializeApp(application)
    }

    fun logEvent(name: String, parameters: Bundle? = null) {
        Firebase.analytics.logEvent(name, parameters)
    }

    fun setUserId(userId: String) {
        Firebase.analytics.setUserId(userId)
        Firebase.crashlytics.setUserId(userId)
    }

    fun logError(error: Throwable) {
        Firebase.crashlytics.recordException(error)
    }
}
```

## Summary

Deployment essentials:

1. **App Store Compliance** - Native functionality, offline support
2. **CI/CD Pipeline** - GitHub Actions + Fastlane
3. **TestFlight / Play Internal** - Staged rollouts
4. **Feature Flags** - LaunchDarkly, Remote Config
5. **A/B Testing** - Google Optimize, Firebase
6. **Analytics** - Firebase Analytics + Crashlytics

---

*Related: `testing-exploration.md`, `accessibility-exploration.md`*
