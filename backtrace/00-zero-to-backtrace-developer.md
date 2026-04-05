# Zero to Backtrace Developer

## Overview

Backtrace is a comprehensive error reporting and crash analytics platform that provides deep introspection into application failures across multiple platforms. The backtrace-labs ecosystem includes SDKs for:

1. **backtrace-go** - Go error reporting with panic handling
2. **backtrace-cocoa** - iOS, macOS, tvOS crash reporting
3. **backtrace-javascript** - Web and Node.js error monitoring
4. **backtrace-android** - Android crash analytics
5. **backtrace-morgue** - Server-side crash aggregation
6. **crashpad** - Native crash handling (C++)
7. **cassette** - Persistent queue for crash data

## Prerequisites

- Basic understanding of error handling in your target language
- Access to a Backtrace account (or compatible error reporting service)
- Network access to submit error reports

## Part 1: Backtrace for Go

### Installation

```bash
go get github.com/backtrace-labs/backtrace-go
```

### Basic Usage

```go
package main

import (
    "net/http"
    bt "github.com/backtrace-labs/backtrace-go"
)

func init() {
    bt.Options.Endpoint = "https://console.backtrace.io"
    bt.Options.Token = "your-token-here"
}

func main() {
    // Report non-fatal errors
    response, err := http.Get("https://example.com")
    if err != nil {
        bt.Report(err, map[string]interface{}{
            "user_id": "12345",
            "action": "fetch_data",
        })
    }
}
```

### Panic Handling

```go
// Report and re-panic
defer bt.ReportPanic(map[string]interface{}{
    "component": "critical_section",
})

// Report and recover (goroutine continues)
defer bt.ReportAndRecoverPanic(map[string]interface{}{
    "component": "background_worker",
})

// Block until all reports are sent
bt.FinishSendingReports()
```

### Out-of-Process Tracing (bcd)

For native crashes and deeper introspection, use the bcd package:

```go
package main

import (
    bt "github.com/backtrace-labs/backtrace-go"
)

func main() {
    // Create tracer for Linux/FreeBSD
    tracer := bt.New(bt.NewOptions{
        IncludeSystemGs: false,
    })
    
    // Configure upload to Backtrace
    err := tracer.ConfigurePut(
        "https://console.backtrace.io",
        "your-token",
        bt.PutOptions{
            Unlink:    true,
            OnTrace:   true,
        },
    )
    
    // Register signal handler for crashes
    bt.Register(tracer)
    
    // Your application code
    runApplication()
}
```

### Configuration Options

```go
bt.Options.Endpoint = "https://console.backtrace.io"
bt.Options.Token = "your-submission-token"
bt.Options.SendEnvVars = true              // Include environment variables
bt.Options.CaptureAllGoroutines = true     // Capture all goroutine stacks
bt.Options.TabWidth = 4                    // Source code tab width
bt.Options.ContextLineCount = 5            // Lines of context around crash
bt.Options.DebugBacktrace = true           // Enable debug logging

// Custom attributes for all reports
bt.Options.Attributes["application"] = "myapp"
bt.Options.Attributes["environment"] = "production"
```

## Part 2: Backtrace for iOS/macOS (Cocoa)

### Installation via Swift Package Manager

```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/backtrace-labs/backtrace-cocoa.git")
]

// Or add in Xcode: File > Add Packages > https://github.com/backtrace-labs/backtrace-cocoa.git
```

### Installation via CocoaPods

```ruby
# Podfile
target 'MyApp' do
  use_frameworks!
  pod 'Backtrace'
end
```

### Swift Usage

```swift
import Backtrace

@main
class AppDelegate: UIResponder, UIApplicationDelegate {
    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {
        
        // Initialize Backtrace
        let credentials = BacktraceCredentials(
            endpoint: URL(string: "https://console.backtrace.io")!,
            token: "your-token"
        )
        
        BacktraceClient.shared = BacktraceClient(
            credentials: credentials,
            configuration: BacktraceConfiguration(
                attachmentTypes: .all,
                breadcrumbsEnabled: true,
                oomReportingEnabled: true,
                crashReporterEnabled: true
            )
        )
        
        // Add custom attributes
        BacktraceClient.shared.attributes["user_id"] = "12345"
        BacktraceClient.shared.attributes["app_version"] = Bundle.main.version
        
        return true
    }
}
```

### Objective-C Usage

```objc
#import <Backtrace/Backtrace.h>

- (BOOL)application:(UIApplication *)application 
    didFinishLaunchingWithOptions:(NSDictionary *)launchOptions {
    
    BacktraceCredentials *credentials = [[BacktraceCredentials alloc] 
        initWithEndpoint:[NSURL URLWithString:@"https://console.backtrace.io"]
        token:@"your-token"];
    
    BacktraceConfiguration *configuration = [[BacktraceConfiguration alloc] init];
    configuration.breadcrumbsEnabled = YES;
    configuration.oomReportingEnabled = YES;
    
    self.backtraceClient = [[BacktraceClient alloc] 
        initWithCredentials:credentials 
        configuration:configuration];
    
    [self.backtraceClient.attributes setValue:@"12345" forKey:@"user_id"];
    
    return YES;
}
```

### Breadcrumbs

```swift
// Manual breadcrumbs
BacktraceClient.shared.breadcrumbs.add(
    BacktraceBreadcrumb(
        level: .info,
        message: "User performed action",
        type: .user
    )
)

// Automatic breadcrumbs (HTTP requests, navigation, etc.)
// Enabled by default in configuration
```

### Attachments

```swift
// Add file attachment
BacktraceClient.shared.addAttachment(
    path: "/path/to/file.txt",
    name: "debug_log.txt"
)

// Add data attachment
let data = "Debug info".data(using: .utf8)!
BacktraceClient.shared.addAttachment(
    data: data,
    name: "debug_info.txt",
    type: "text/plain"
)
```

## Part 3: Backtrace for JavaScript/Node.js

### Installation

```bash
# Browser
npm install @backtrace/browser

# Node.js
npm install @backtrace/node

# React
npm install @backtrace/react

# Electron
npm install @backtrace/electron
```

### Browser Usage

```javascript
import { BacktraceClient } from '@backtrace/browser';

const client = new BacktraceClient({
    url: 'https://console.backtrace.io',
    token: 'your-token',
    attributes: {
        application: 'my-web-app',
        environment: 'production'
    }
});

client.install();

// Manual error reporting
try {
    riskyOperation();
} catch (error) {
    client.send(error, {
        customAttribute: 'value'
    });
}

// Add breadcrumb
client.breadcrumb.add('User clicked button', {
    type: 'user',
    level: 'info'
});
```

### Node.js Usage

```javascript
const { BacktraceClient } = require('@backtrace/node');

const client = new BacktraceClient({
    url: 'https://console.backtrace.io',
    token: 'your-token',
    database: {
        enabled: true,
        path: './backtrace-db'
    },
    breadcrumbs: {
        enabled: true
    },
    minidump: {
        enabled: true
    }
});

client.install();

// Uncaught exceptions are automatically captured
// Unhandled promise rejections are automatically captured

// Manual reporting
client.send(new Error('Something went wrong'), {
    userId: '12345'
});
```

### React Integration

```javascript
import { BacktraceErrorBoundary, useBacktraceClient } from '@backtrace/react';

function App() {
    const client = useBacktraceClient({
        url: 'https://console.backtrace.io',
        token: 'your-token'
    });
    
    return (
        <BacktraceErrorBoundary 
            fallback={<ErrorFallback />}
            attributes={{ component: 'HomePage' }}
        >
            <HomePage />
        </BacktraceErrorBoundary>
    );
}
```

### Source Map Upload

```javascript
// webpack.config.js
const BacktracePlugin = require('@backtrace/webpack-plugin');

module.exports = {
    plugins: [
        new BacktracePlugin({
            url: 'https://console.backtrace.io',
            token: 'your-token',
            uploadSource: true
        })
    ]
};
```

## Part 4: Backtrace for Android

### Installation

```gradle
// app/build.gradle
dependencies {
    implementation 'io.backtrace:backtrace-android:1.+'
}
```

### Basic Usage

```java
public class MyApplication extends Application {
    @Override
    public void onCreate() {
        super.onCreate();
        
        BacktraceCredentials credentials = new BacktraceCredentials(
            "https://console.backtrace.io",
            "your-token"
        );
        
        BacktraceConfiguration config = new BacktraceConfiguration.Builder()
            .breadcrumbsEnabled(true)
            .anrEnabled(true)
            .nativeCrashEnabled(true)
            .build();
        
        BacktraceClient backtrace = new BacktraceClient(this, credentials, config);
        
        // Add attributes
        backtrace.setAttribute("user_id", "12345");
        backtrace.setAttribute("app_version", BuildConfig.VERSION_NAME);
    }
}
```

### Breadcrumbs

```java
// Add manual breadcrumb
backtrace.getBreadcrumbs().record("User action", BreadcrumbType.USER);

// Query breadcrumbs
List<Breadcrumb> breadcrumbs = backtrace.getBreadcrumbs().all();
```

### Native Crash Handling

```java
// Enable native crash reporting in configuration
BacktraceConfiguration config = new BacktraceConfiguration.Builder()
    .nativeCrashEnabled(true)
    .build();

// Initialize with native support
BacktraceClient backtrace = new BacktraceClient(this, credentials, config);
```

## Part 5: Understanding Backtrace Architecture

### Client-Side Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Application                               │
├─────────────────────────────────────────────────────────────┤
│  Error/Crash Occurs                                          │
│         │                                                    │
│         ▼                                                    │
│  SDK Captures:                                               │
│  - Stack trace                                               │
│  - Thread info                                               │
│  - Breadcrumbs                                               │
│  - Attributes                                                │
│  - Attachments                                               │
│         │                                                    │
│         ▼                                                    │
│  Local Database (offline queuing)                           │
│         │                                                    │
│         ▼                                                    │
│  HTTP POST to Backtrace                                     │
└─────────────────────────────────────────────────────────────┘
```

### Server-Side Flow (Morgue)

```
┌─────────────────────────────────────────────────────────────┐
│                   Backtrace Server                           │
├─────────────────────────────────────────────────────────────┤
│  Ingestion Layer                                             │
│  - Authentication                                           │
│  - Rate limiting                                            │
│  - Validation                                               │
├─────────────────────────────────────────────────────────────┤
│  Processing Layer                                            │
│  - Symbolication                                            │
│  - Classification                                           │
│  - Grouping                                                 │
│  - Deduplication                                            │
├─────────────────────────────────────────────────────────────┤
│  Storage Layer                                               │
│  - MongoDB (report storage)                                 │
│  - Redis (caching, metrics)                                 │
│  - S3 (attachments)                                         │
├─────────────────────────────────────────────────────────────┤
│  Query Layer                                                 │
│  - Elasticsearch (search)                                   │
│  - Analytics API                                            │
└─────────────────────────────────────────────────────────────┘
```

## Part 6: Cassette - Persistent Queue

Cassette provides reliable file-based queuing for crash data:

```objc
// iOS Example
#import <Cassette/Cassette.h>

// Create persistent queue
NSError *error;
CASObjectQueue<NSNumber *> *queue = 
    [[CASFileObjectQueue alloc] initWithRelativePath:@"crash-queue" 
                                               error:&error];

// Add to queue
[queue add:@1 error:&error];

// Process from queue
NSNumber *item = [queue peek:1 error:&error].firstObject;

// Remove after processing
[queue pop:1 error:&error];
```

## Summary

The Backtrace ecosystem provides:

1. **Multi-platform SDKs** - Go, Cocoa, JavaScript, Android, native
2. **Crash detection** - Panics, signals, uncaught exceptions, ANRs
3. **Rich context** - Breadcrumbs, attributes, attachments
4. **Offline support** - Local database with retry
5. **Symbolication** - Source map and dSYM support
6. **Grouping** - Intelligent error classification
7. **Native support** - C/C++ crash handling via crashpad

Each SDK is designed to capture maximum context about failures while minimizing performance impact on the application.
