# utm-dev Production - Testing Infrastructure Exploration

## Overview

This document explores testing strategies for production apps built with utm-dev, covering E2E testing, device farms, screenshot testing, and CI/CD integration.

## Testing Pyramid for utm-dev Apps

```
                    ┌─────────────┐
                   ╱  E2E Tests  ╲
                  ╱   (Maestro/   ╲
                 ╱     Detox)      ╲
                ├───────────────────┤
               ╱   Integration      ╲
              ╱   (XCUITest/        ╲
             ╱     Espresso)        ╲
            ├─────────────────────────┤
           ╱      Unit Tests          ╲
          ╱    (Go test + native)      ╲
         ├───────────────────────────────┤
```

## Unit Testing

### Go Code Testing

```go
// pkg/project/project_test.go
package project

import (
    "testing"
    "github.com/stretchr/testify/assert"
)

func TestNewProject(t *testing.T) {
    t.Run("valid project", func(t *testing.T) {
        proj, err := NewProject("/path/to/valid/app")
        assert.NoError(t, err)
        assert.Equal(t, "com.example.app", proj.BundleID)
    })

    t.Run("missing go.mod", func(t *testing.T) {
        _, err := NewProject("/path/to/invalid/app")
        assert.ErrorIs(t, err, ErrNoGoMod)
    })
}

func TestProject_Validate(t *testing.T) {
    tests := []struct {
        name    string
        project *GioProject
        wantErr error
    }{
        {"valid", validProject(), nil},
        {"missing bundle ID", projectWithoutBundle(), ErrNoBundleID},
        {"invalid bundle ID", projectWithInvalidBundle(), ErrInvalidBundleID},
    }

    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            err := tt.project.Validate()
            assert.ErrorIs(t, err, tt.wantErr)
        })
    }
}
```

### Testing Build Cache

```go
// pkg/buildcache/cache_test.go
package buildcache

import (
    "testing"
    "os"
    "path/filepath"
)

func TestCache_GetHit(t *testing.T) {
    tmpDir := t.TempDir()
    cache := NewCache(tmpDir)

    // Create source files
    srcDir := filepath.Join(tmpDir, "src")
    os.MkdirAll(srcDir, 0755)
    os.WriteFile(filepath.Join(srcDir, "main.go"), []byte("package main"), 0644)

    // First build - miss
    hash, err := cache.GetSourceHash(srcDir)
    assert.NoError(t, err)

    hit := cache.Check(hash)
    assert.False(t, hit)

    // Record build
    cache.Record(hash, "build-output")

    // Second check - hit
    hit = cache.Check(hash)
    assert.True(t, hit)
}
```

## Integration Testing

### XCUITest (iOS)

```swift
// ios/IntegrationTests/IntegrationTests.swift
import XCTest

class IntegrationTests: XCTestCase {

    var app: XCUIApplication!

    override func setUp() {
        super.setUp()
        continueAfterFailure = false

        app = XCUIApplication()
        app.launchArguments = ["-ui-testing", "-mock-server"]
        app.launch()
    }

    func testAppLaunches_Successfully() {
        // Wait for WebView to appear
        let webView = app.webViews.firstMatch
        XCTAssertTrue(webView.waitForExistence(timeout: 10))

        // Verify expected content
        XCTAssertTrue(webView.staticTexts["Welcome"].exists)
    }

    func testDeepLink_Opening() {
        // Launch with URL
        let url = URL(string: "myapp://posts/123")!
        app = XCUIApplication()
        app.launchURL(url)

        // Verify navigation
        XCTAssertTrue(app.staticTexts["Post 123"].waitForExistence(timeout: 5))
    }

    func testUTMIntegration_BuildAndRun() {
        // This tests the utm-dev tooling itself
        let output = runCommand("utm-dev", "build", "ios", testAppPath)
        XCTAssertTrue(output.contains("Build successful"))
    }
}
```

### Espresso (Android)

```kotlin
// android/IntegrationTest.kt
@RunWith(AndroidJUnit4::class)
class IntegrationTests {

    @get:Rule
    val activityRule = ActivityScenarioRule(MainActivity::class.java)

    @Test
    fun testAppLaunches_Successfully() {
        // Wait for WebView
        onView(withId(R.id.webView))
            .check(matches(isDisplayed()))

        // Verify content
        onView(withText("Welcome"))
            .check(matches(isDisplayed()))
    }

    @Test
    fun testDeepLink_Opening() {
        // Launch with deep link intent
        val intent = Intent(Intent.ACTION_VIEW)
            .setData(Uri.parse("myapp://posts/123"))

        ActivityScenario.launch<MainActivity>(intent).use {
            onView(withText("Post 123"))
                .check(matches(isDisplayed()))
        }
    }

    @Test
    fun testBuildCache_WorksCorrectly() {
        // Test utm-dev build cache
        val result = runCommand("utm-dev build android --cached test-app")
        assertThat(result.exitCode).isEqualTo(0)
        assertThat(result.output).contains("Using cached build")
    }
}
```

## E2E Testing

### Maestro Flows

```yaml
# e2e/flows/login.yaml
appId: com.example.app
---
- launchApp:
    clearState: true
- assertVisible: "Welcome"
- tapOn: "Sign In"
- assertVisible: "Email"
- inputText: "test@example.com"
- tapOn: "Password"
- inputText: "password123"
- tapOn: "Sign In"
- assertVisible: "Welcome back"
- assertNotVisible: "Sign In"
- stopApp

# e2e/flows/navigation.yaml
appId: com.example.app
---
- launchApp
- assertVisible: "Home"
- tapOn:
    id: "nav-profile"
- assertVisible: "Profile"
- tapOn:
    id: "nav-settings"
- assertVisible: "Settings"
- pressBack
- assertVisible: "Profile"
- stopApp

# e2e/flows/utm-run.yaml
appId: com.example.utmtest
---
- launchApp
- tapOn: "Run in UTM"
- assertVisible: "Building for Windows"
- waitForAnimationToEnd
- assertVisible: "App running in VM"
- tapOn: "Stop"
- assertVisible: "VM stopped"
- stopApp
```

### Detox (React Native / Hybrid)

```javascript
// e2e/app.test.js
const { device, element, by, expect } = require('detox');

describe('utm-dev E2E Tests', () => {
  beforeAll(async () => {
    await device.launchApp({
      newInstance: true,
      launchArgs: { 'mock-server': 'true' }
    });
  });

  it('should load home screen successfully', async () => {
    await expect(element(by.text('Welcome'))).toBeVisible();
  });

  it('should handle UTM build and run', async () => {
    // Tap UTM run button
    await element(by.id('utm-run-button')).tap();

    // Wait for build
    await waitFor(element(by.text('Build complete')))
      .toBeVisible()
      .withTimeout(60000);

    // Verify app runs in VM
    await expect(element(by.text('Running in VM'))).toBeVisible();
  });

  it('should handle deep links', async () => {
    await device.launchApp({
      url: 'myapp://posts/456'
    });

    await expect(element(by.text('Post 456'))).toBeVisible();
  });
});
```

## Screenshot Testing

### iOS Snapshot Testing

```swift
// ScreenshotTests.swift
import SnapshotTesting
import XCTest

class ScreenshotTests: XCTestCase {

    func testHomeScreen() {
        let vc = HomeViewController()
        vc.preferredContentSize = CGSize(width: 375, height: 667)

        assertSnapshot(matching: vc, as: .image(on: .iPhone8))
    }

    func testHomeScreen_DarkMode() {
        let vc = HomeViewController()
        vc.view.traitCollection = UITraitCollection(userInterfaceStyle: .dark)

        assertSnapshot(matching: vc, as: .image(on: .iPhone8))
    }

    func testHomeScreen_Spanish() {
        let vc = HomeViewController()
        vc.view.traitCollection = UITraitCollection(preferredLocale: "es_ES")

        assertSnapshot(matching: vc, as: .image(on: .iPhone8))
    }

    func testUTMBuildScreen() {
        let vc = UTMBuildViewController()
        assertSnapshot(matching: vc, as: .image(on: .iPadPro12_9))
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
    fun testHomeScreen() {
        screenshotRule
            .forView(R.id.main_content)
            .assertSnapshot("home_screen")
    }

    @Test
    fun testHomeScreen_DarkMode() {
        screenshotRule
            .forView(R.id.main_content)
            .withUiMode(UiMode.NIGHT_YES)
            .assertSnapshot("home_screen_dark")
    }

    @Test
    fun testUTMBuildScreen() {
        screenshotRule
            .forView(R.id.utm_build_content)
            .assertSnapshot("utm_build")
    }
}
```

## Device Farm Integration

### Firebase Test Lab

```yaml
# .github/workflows/android-test.yml
name: Android Device Testing

on: push

jobs:
  test-firebase:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build APK
        run: utm-dev build android ./cmd/myapp

      - name: Run Firebase Test Lab
        uses: wzieba/Firebase-Distribution-GHA@v1
        with:
          appId: ${{ secrets.FIREBASE_APP_ID }}
          serviceCredentialsFileContent: ${{ secrets.FIREBASE_SERVICE_ACCOUNT }}
          groups: testers
          devices:
            - model: pixel6
              version: 33
            - model: pixel7
              version: 34
            - model: galaxy_s23
              version: 33

      - name: Upload test results
        uses: actions/upload-artifact@v4
        with:
          name: test-results
          path: build/test-results/
```

### AWS Device Farm

```yaml
# .github/workflows/aws-device-farm.yml
name: AWS Device Testing

on: push

jobs:
  test-aws:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v2
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-west-2

      - name: Upload to Device Farm
        run: |
          aws devicefarm create-upload \
            --name "myapp-${{ github.sha }}" \
            --type ANDROID_APP \
            --endpoint /uploads/myapp.apk

      - name: Run tests
        run: |
          aws devicefarm schedule-run \
            --project-arn ${{ secrets.DEVICE_FARM_PROJECT_ARN }} \
            --device-pool-arn ${{ secrets.DEVICE_POOL_ARN }}
```

## UTM-Specific Testing

### VM Testing

```go
// pkg/utm/utm_test.go
package utm

import (
    "testing"
    "context"
)

func TestUTM_CreateVM(t *testing.T) {
    utm := NewUTM()

    ctx := context.Background()
    vm, err := utm.CreateVM(ctx, CreateVMRequest{
        Name:     "test-windows-vm",
        OS:       "windows",
        Template: "windows-11-arm",
        RAM:      4096,
        CPU:      2,
    })

    assert.NoError(t, err)
    assert.NotNil(t, vm)
    assert.Equal(t, "test-windows-vm", vm.Name)
}

func TestUTM_PortForward(t *testing.T) {
    utm := NewUTM()

    ctx := context.Background()
    err := utm.StartPortForward(ctx, "test-vm", 8080, 8080)
    assert.NoError(t, err)

    // Verify port is forwarded
    conn, err := net.Dial("tcp", "localhost:8080")
    assert.NoError(t, err)
    conn.Close()

    utm.StopPortForward(ctx, "test-vm", 8080)
}
```

### Integration with CI/CD

```yaml
# .github/workflows/utm-integration.yml
name: UTM Integration Tests

on: push

jobs:
  utm-test:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4

      - name: Install UTM
        run: brew install --cask utm

      - name: Create test VM
        run: |
          utm-dev utm create test-win11 \
            --os windows \
            --template windows-11-arm \
            --ram 4096

      - name: Build for Windows
        run: utm-dev build windows ./cmd/testapp

      - name: Run in VM
        run: |
          utm-dev utm run test-win11 ./build/testapp.exe \
            --timeout 60s \
            --capture-output

      - name: Verify output
        run: |
          utm-dev utm exec test-win11 -- cat /tmp/testapp-output.log
```

## Test Automation

### Test Runner

```go
// cmd/test.go
package cmd

import (
    "github.com/spf13/cobra"
)

var testCmd = &cobra.Command{
    Use:   "test [platform]",
    Short: "Run tests for the application",
    Long: `Run unit, integration, and E2E tests for the specified platform.

Supported platforms: all, macos, ios, android, windows, linux`,
    RunE: func(cmd *cobra.Command, args []string) error {
        platform := args[0]

        switch platform {
        case "all":
            return runAllTests()
        case "ios":
            return runIOSTests()
        case "android":
            return runAndroidTests()
        case "e2e":
            return runE2ETests()
        default:
            return runUnitTests()
        }
    },
}

func runIOSTests() error {
    // Run Go tests
    if err := runGoTests("./..."); err != nil {
        return err
    }

    // Run XCUITest
    cmd := exec.Command("xcodebuild",
        "-scheme", "IntegrationTests",
        "-destination", "platform=iOS Simulator,name=iPhone 15",
        "test",
    )
    return cmd.Run()
}

func runAndroidTests() error {
    // Run Go tests
    if err := runGoTests("./..."); err != nil {
        return err
    }

    // Run Espresso tests
    cmd := exec.Command("./gradlew", "connectedAndroidTest")
    return cmd.Run()
}
```

## Summary

Testing essentials for utm-dev apps:

1. **Unit Tests** - Go testing, table-driven tests, mocks
2. **Integration Tests** - XCUITest (iOS), Espresso (Android)
3. **E2E Tests** - Maestro flows, Detox
4. **Screenshot Tests** - Snapshot testing for visual regression
5. **Device Farms** - Firebase Test Lab, AWS Device Farm
6. **UTM Testing** - VM lifecycle, port forwarding, file transfer

---

*Related: `testing-exploration.md`, `deployment-cicd-exploration.md`*
