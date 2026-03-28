---
title: "Telescope Rust Revision"
subtitle: "Complete Rust translation guide for telescope filesystem backend"
---

# Telescope Rust Revision

## Introduction

This document provides a complete Rust translation of telescope's filesystem backend. We'll translate the TypeScript/Playwright implementation to idiomatic Rust using valtron executors instead of async/await.

## Table of Contents

1. [Type System Translation](#type-system-translation)
2. [Ownership and Borrowing Strategy](#ownership-and-borrowing-strategy)
3. [Core Types Translation](#core-types-translation)
4. [TestRunner Translation](#testrunner-translation)
5. [Browser Configuration](#browser-configuration)
6. [Metrics Collection](#metrics-collection)
7. [Valtron Integration](#valtron-integration)
8. [Complete Example](#complete-example)

---

## Type System Translation

### TypeScript to Rust Type Mapping

| TypeScript | Rust | Notes |
|------------|------|-------|
| `string` | `String` / `&str` | Owned vs borrowed |
| `number` | `f64` / `u64` / `i64` | Choose based on use |
| `boolean` | `bool` | Direct mapping |
| `T \| null` | `Option<T>` | Rust's option type |
| `T \| undefined` | `Option<T>` | Same as null |
| `T \| U` | `enum MyType { T, U }` | Rust enums |
| `Record<K, V>` | `HashMap<K, V>` | Hash map |
| `Array<T>` | `Vec<T>` | Dynamic array |
| `Promise<T>` | `TaskIterator` / `Future` | Valtron or tokio |
| `interface` | `struct` / `trait` | Data vs behavior |
| `class` | `struct` + `impl` | Split data/behavior |

### Error Handling Translation

```typescript
// TypeScript: throw/catch
async function doSomething(): Promise<Result> {
  try {
    return await operation();
  } catch (error) {
    return { success: false, error: error.message };
  }
}
```

```rust
// Rust: Result type with valtron
struct DoSomethingTask {
    // Task state
}

impl TaskIterator for DoSomethingTask {
    type Pending = ();
    type Ready = Result<TestResult, TestError>;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.operation() {
            Ok(result) => Some(TaskStatus::Ready(Ok(result))),
            Err(e) => Some(TaskStatus::Ready(Err(e))),
        }
    }
}
```

---

## Ownership and Borrowing Strategy

### telescope Ownership Model

```rust
/// Main test runner - owns all test state
pub struct TestRunner {
    /// Owned configuration
    config: TestConfig,
    /// Owned browser configuration
    browser_config: BrowserConfig,
    /// Owned paths
    paths: TestPaths,
    /// Owned metrics (collected during test)
    metrics: Option<Metrics>,
    /// Owned console messages
    console_messages: Vec<ConsoleMessage>,
    /// Owned resource timings
    resource_timings: Vec<ResourceTiming>,
    /// Browser instance (optionally owned)
    browser: Option<BrowserInstance>,
    /// Page reference (borrowed from browser)
    page: Option<*mut Page>,  // Raw pointer for FFI
}

/// Test configuration - owned, cloned when needed
#[derive(Clone, Debug)]
pub struct TestConfig {
    pub url: String,
    pub browser: BrowserName,
    pub width: u32,
    pub height: u32,
    pub timeout: Duration,
    // ... other fields
}

/// Browser config - borrowed during test execution
pub struct BrowserConfig<'a> {
    pub engine: BrowserEngine,
    pub headless: bool,
    pub viewport: &'a ViewportSize,
    pub record_har: &'a HarConfig,
    pub record_video: &'a VideoConfig,
}
```

### Borrowing Patterns

```rust
impl TestRunner {
    /// Constructor takes ownership of config
    pub fn new(config: TestConfig) -> Result<Self> {
        let browser_config = BrowserConfig::from(&config);
        let paths = TestPaths::generate(&config)?;

        Ok(Self {
            config,
            browser_config,
            paths,
            metrics: None,
            console_messages: Vec::new(),
            resource_timings: Vec::new(),
            browser: None,
            page: None,
        })
    }

    /// Setup borrows self mutably to initialize browser
    pub async fn setup(&mut self) -> Result<()> {
        self.browser = Some(BrowserInstance::launch(&self.browser_config).await?);
        // Get page reference
        if let Some(browser) = &self.browser {
            self.page = Some(browser.get_page(0));
        }
        Ok(())
    }

    /// Navigation borrows page immutably
    pub async fn navigate(&self) -> Result<()> {
        let page = self.page.ok_or(Error::PageNotInitialized)?;
        // Safe because page outlives the navigation
        unsafe {
            (*page).goto(&self.config.url).await?;
        }
        Ok(())
    }

    /// Metrics collection borrows mutably to store results
    pub fn collect_metrics(&mut self) -> Result<()> {
        let page = self.page.ok_or(Error::PageNotInitialized)?;
        let metrics = unsafe { (*page).evaluate(collect_metrics_script)? };
        self.metrics = Some(metrics);
        Ok(())
    }
}
```

---

## Core Types Translation

### Test Result Types

```rust
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::{SystemTime, Duration};

/// Test result (equivalent to TypeScript TestResult type)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "success")]
pub enum TestResult {
    #[serde(rename = "true")]
    Success {
        test_id: String,
        results_path: PathBuf,
        #[serde(skip_serializing_if = "Option::is_none")]
        dry: Option<bool>,
    },
    #[serde(rename = "false")]
    Failure {
        error: String,
    },
}

impl TestResult {
    pub fn success(test_id: String, results_path: PathBuf) -> Self {
        TestResult::Success {
            test_id,
            results_path,
            dry: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        TestResult::Failure {
            error: error.into(),
        }
    }

    pub fn dry_run(test_id: String, results_path: PathBuf) -> Self {
        TestResult::Success {
            test_id,
            results_path,
            dry: Some(true),
        }
    }
}

/// Launch options (from CLI or programmatic API)
#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    pub url: String,
    pub browser: BrowserName,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub cookies: Option<Vec<Cookie>>,
    pub args: Option<Vec<String>>,
    pub block_domains: Option<Vec<String>>,
    pub block: Option<Vec<String>>,
    pub firefox_prefs: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub cpu_throttle: Option<f64>,
    pub connection_type: Option<ConnectionType>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<u32>,
    pub disable_js: bool,
    pub debug: bool,
    pub auth: Option<AuthCredentials>,
    pub timeout: Option<Duration>,
    pub html: bool,
    pub open_html: bool,
    pub list: bool,
    pub override_host: Option<std::collections::HashMap<String, String>>,
    pub zip: bool,
    pub upload_url: Option<url::Url>,
    pub dry: bool,
    pub user_agent: Option<String>,
    pub agent_extra: Option<String>,
    pub delay: Option<std::collections::HashMap<String, u64>>,
    pub delay_using: DelayMethod,
}

/// Browser name enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrowserName {
    #[default]
    Chrome,
    ChromeBeta,
    Canary,
    Firefox,
    Safari,
    Edge,
}

impl std::str::FromStr for BrowserName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "chrome" => Ok(BrowserName::Chrome),
            "chrome-beta" => Ok(BrowserName::ChromeBeta),
            "canary" => Ok(BrowserName::Canary),
            "firefox" => Ok(BrowserName::Firefox),
            "safari" => Ok(BrowserName::Safari),
            "edge" => Ok(BrowserName::Edge),
            _ => Err(format!("Unknown browser: {}", s)),
        }
    }
}

/// Connection type for network throttling
#[derive(Debug, Clone, Copy, Default)]
pub enum ConnectionType {
    #[default]
    None,
    Cable,
    Dsl,
    G4,
    G3,
    G3Fast,
    G3Slow,
    G2,
    Fios,
}

impl ConnectionType {
    pub fn network_profile(self) -> Option<NetworkProfile> {
        match self {
            ConnectionType::None => None,
            ConnectionType::Cable => Some(NetworkProfile {
                down: 5000,
                up: 1000,
                rtt: 14,
            }),
            ConnectionType::Dsl => Some(NetworkProfile {
                down: 1500,
                up: 384,
                rtt: 25,
            }),
            ConnectionType::G4 => Some(NetworkProfile {
                down: 9000,
                up: 9000,
                rtt: 85,
            }),
            ConnectionType::G3 => Some(NetworkProfile {
                down: 1600,
                up: 768,
                rtt: 150,
            }),
            ConnectionType::G3Fast => Some(NetworkProfile {
                down: 1600,
                up: 768,
                rtt: 75,
            }),
            ConnectionType::G3Slow => Some(NetworkProfile {
                down: 400,
                up: 400,
                rtt: 200,
            }),
            ConnectionType::G2 => Some(NetworkProfile {
                down: 280,
                up: 256,
                rtt: 400,
            }),
            ConnectionType::Fios => Some(NetworkProfile {
                down: 20000,
                up: 5000,
                rtt: 2,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkProfile {
    pub down: u32,  // kbps
    pub up: u32,    // kbps
    pub rtt: u32,   // ms
}

/// Delay method (fulfill vs continue)
#[derive(Debug, Clone, Copy, Default)]
pub enum DelayMethod {
    #[default]
    Continue,
    Fulfill,
}

/// Cookie structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<i64>,
    pub http_only: Option<bool>,
    pub secure: Option<bool>,
    pub same_site: Option<SameSite>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

/// Auth credentials
#[derive(Debug, Clone)]
pub struct AuthCredentials {
    pub username: String,
    pub password: String,
    pub origin: Option<String>,
}
```

### Metrics Types

```rust
/// All collected metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub navigation_timing: NavigationTiming,
    pub paint_timing: Vec<PaintTiming>,
    pub user_timing: Vec<UserTiming>,
    pub largest_contentful_paint: Vec<LcpEvent>,
    pub layout_shifts: Vec<LayoutShift>,
}

/// Navigation timing (from Performance API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationTiming {
    pub name: String,
    pub entry_type: String,
    pub start_time: f64,
    pub duration: f64,
    pub navigation_start: f64,
    pub unload_event_start: f64,
    pub unload_event_end: f64,
    pub redirect_start: f64,
    pub redirect_end: f64,
    pub fetch_start: f64,
    pub domain_lookup_start: f64,
    pub domain_lookup_end: f64,
    pub connect_start: f64,
    pub connect_end: f64,
    pub secure_connection_start: f64,
    pub request_start: f64,
    pub response_start: f64,
    pub response_end: f64,
    pub dom_loading: f64,
    pub dom_interactive: f64,
    pub dom_content_loaded_event_start: f64,
    pub dom_content_loaded_event_end: f64,
    pub dom_complete: f64,
    pub load_event_start: f64,
    pub load_event_end: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_timing: Option<Vec<ServerTiming>>,
    // Chromium-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_interim_response_start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_response_headers_start: Option<f64>,
}

impl NavigationTiming {
    /// Calculate Time to First Byte (TTFB)
    pub fn ttfb(&self) -> f64 {
        self.response_start - self.navigation_start
    }

    /// Calculate DOM Content Loaded time
    pub fn dom_content_loaded(&self) -> f64 {
        self.dom_content_loaded_event_end - self.navigation_start
    }

    /// Calculate total load time
    pub fn total_load_time(&self) -> f64 {
        self.load_event_end - self.navigation_start
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerTiming {
    pub name: String,
    pub description: String,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaintTiming {
    pub name: String,
    pub entry_type: String,
    pub start_time: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTiming {
    pub name: String,
    pub entry_type: String,  // "mark" or "measure"
    pub start_time: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcpElement {
    pub node_name: String,
    pub bounding_rect: BoundingRect,
    pub outer_html: String,
    pub src: Option<String>,
    pub current_src: Option<String>,
    pub background_image: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcpEvent {
    pub name: String,
    pub entry_type: String,
    pub start_time: f64,
    pub size: f64,
    pub url: String,
    pub id: String,
    pub load_time: f64,
    pub render_time: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<LcpElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutShift {
    pub name: String,
    pub entry_type: String,
    pub start_time: f64,
    pub value: f64,
    pub had_recent_input: bool,
    pub last_input_time: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<LayoutShiftSource>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutShiftSource {
    pub previous_rect: LayoutShiftSourceRect,
    pub current_rect: LayoutShiftSourceRect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutShiftSourceRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}
```

---

## TestRunner Translation

### Rust TestRunner Structure

```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use serde::{Serialize, Deserialize};

/// Test runner (equivalent to TypeScript TestRunner class)
pub struct TestRunner {
    /// Test configuration
    config: LaunchOptions,
    /// Browser configuration
    browser_config: BrowserConfig,
    /// Test ID (generated)
    test_id: String,
    /// Result paths
    paths: TestPaths,
    /// Console messages collected
    console_messages: Vec<ConsoleMessage>,
    /// Metrics (populated after test)
    metrics: Option<Metrics>,
    /// Resource timings
    resource_timings: Vec<ResourceTiming>,
    /// Result assets (video, filmstrip)
    result_assets: ResultAssets,
    /// Browser instance
    browser: Option<BrowserInstance>,
    /// Page handle
    page: Option<PageHandle>,
    /// HAR entries for enhancement
    har_entries: Vec<HarEntry>,
}

/// Test paths configuration
#[derive(Debug, Clone)]
pub struct TestPaths {
    pub temporary_context: PathBuf,
    pub results: PathBuf,
    pub filmstrip: PathBuf,
}

impl TestPaths {
    pub fn generate(test_id: &str) -> Result<Self, IoError> {
        let base = PathBuf::from("./results");
        let results = base.join(test_id);
        let filmstrip = results.join("filmstrip");
        let temporary_context = PathBuf::from("./tmp");

        // Create directories
        std::fs::create_dir_all(&results)?;
        std::fs::create_dir_all(&filmstrip)?;
        std::fs::create_dir_all(&temporary_context)?;

        Ok(Self {
            temporary_context,
            results,
            filmstrip,
        })
    }
}

/// Browser configuration (merged from options and defaults)
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub engine: BrowserEngine,
    pub channel: Option<BrowserChannel>,
    pub headless: bool,
    pub viewport: ViewportSize,
    pub record_har: HarConfig,
    pub record_video: VideoConfig,
    pub args: Vec<String>,
    pub ignore_default_args: Vec<String>,
    pub firefox_user_prefs: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub java_script_enabled: bool,
    pub http_credentials: Option<AuthCredentials>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserEngine {
    Chromium,
    Firefox,
    WebKit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserChannel {
    Chrome,
    ChromeBeta,
    ChromeCanary,
    MsEdge,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewportSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct HarConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct VideoConfig {
    pub dir: PathBuf,
    pub size: ViewportSize,
}

/// Console message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub msg_type: String,
    pub text: String,
    pub location: ConsoleLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLocation {
    pub url: String,
    pub line_number: u32,
    pub column_number: u32,
}

/// Resource timing entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTiming {
    pub name: String,
    pub entry_type: String,
    pub start_time: f64,
    pub duration: f64,
    pub initiator_type: String,
    pub fetch_start: f64,
    pub domain_lookup_start: f64,
    pub domain_lookup_end: f64,
    pub connect_start: f64,
    pub connect_end: f64,
    pub secure_connection_start: f64,
    pub request_start: f64,
    pub response_start: f64,
    pub response_end: f64,
    pub transfer_size: u64,
    pub encoded_body_size: u64,
    pub decoded_body_size: u64,
}

/// Result assets
#[derive(Debug, Clone, Default)]
pub struct ResultAssets {
    pub filmstrip: Option<Vec<FilmstripFrame>>,
    pub filmstrip_files: Vec<PathBuf>,
    pub video_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct FilmstripFrame {
    pub num: u32,
    pub filename: PathBuf,
    pub ms: u64,
}

/// HAR entry with extended timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarEntry {
    pub request: HarRequest,
    pub response: HarResponse,
    pub time: f64,
    pub started_date_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timings: Option<HarTimings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_end: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_end: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure_start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure_end: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_end: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_end: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_lcp: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarRequest {
    pub url: String,
    pub method: String,
    pub headers: Vec<HttpHeader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarResponse {
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_size: Option<u64>,
    pub content: HarContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarContent {
    pub size: i64,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarTimings {
    pub blocked: Option<f64>,
    pub dns: Option<f64>,
    pub connect: Option<f64>,
    pub send: Option<f64>,
    pub wait: Option<f64>,
    pub receive: Option<f64>,
}

/// HAR log structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarLog {
    pub pages: Vec<HarPage>,
    pub entries: Vec<HarEntry>,
    pub browser: HarBrowser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarPage {
    pub page_timings: HarPageTimings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarPageTimings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttfb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lcp: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarBrowser {
    pub name: String,
    pub version: String,
}
```

### TestRunner Implementation

```rust
impl TestRunner {
    /// Create a new test runner
    pub fn new(config: LaunchOptions) -> Result<Self, TestError> {
        let test_id = generate_test_id();
        let paths = TestPaths::generate(&test_id)?;
        let browser_config = BrowserConfig::from_options(&config, &paths)?;

        Ok(Self {
            config,
            browser_config,
            test_id,
            paths,
            console_messages: Vec::new(),
            metrics: None,
            resource_timings: Vec::new(),
            result_assets: ResultAssets::default(),
            browser: None,
            page: None,
            har_entries: Vec::new(),
        })
    }

    /// Save configuration to disk
    pub fn save_config(&self) -> Result<(), TestError> {
        let saved_config = SavedConfig {
            url: self.config.url.clone(),
            date: chrono::Utc::now().to_rfc3339(),
            options: self.config.clone(),
            browser_config: self.browser_config.clone(),
        };

        let config_path = self.paths.results.join("config.json");
        let json = serde_json::to_string_pretty(&saved_config)?;
        std::fs::write(&config_path, json)?;

        Ok(())
    }

    /// Setup test (launch browser, create page)
    pub async fn setup_test(&mut self) -> Result<(), TestError> {
        // Launch browser
        self.browser = Some(BrowserInstance::launch(&self.browser_config).await?);

        // Create page
        if let Some(browser) = &self.browser {
            self.page = Some(browser.new_page().await?);
        }

        // Setup console message collection
        if let Some(page) = &self.page {
            page.on_console(|msg| {
                self.console_messages.push(ConsoleMessage {
                    msg_type: msg.msg_type().to_string(),
                    text: msg.text(),
                    location: ConsoleLocation {
                        url: msg.location().url.unwrap_or_default(),
                        line_number: msg.location().line_number.unwrap_or(0),
                        column_number: msg.location().column_number.unwrap_or(0),
                    },
                });
            });
        }

        // Setup request blocking if configured
        self.setup_blocking().await?;

        // Setup response delays if configured
        self.setup_response_delays().await?;

        // Apply network throttling if configured
        if let Some(connection_type) = self.config.connection_type {
            self.throttle_network(connection_type).await?;
        }

        Ok(())
    }

    /// Setup request blocking
    async fn setup_blocking(&self) -> Result<(), TestError> {
        let page = self.page.as_ref().ok_or(TestError::PageNotInitialized)?;

        // Block domains
        if let Some(domains) = &self.config.block_domains {
            let pattern = format!("//({})/", domains.join("|"));
            page.route(&pattern, |route| route.abort()).await?;
        }

        // Block URL substrings
        if let Some(blocks) = &self.config.block {
            let pattern = blocks.join("|");
            page.route(&pattern, |route| route.abort()).await?;
        }

        Ok(())
    }

    /// Setup response delays
    async fn setup_response_delays(&self) -> Result<(), TestError> {
        let page = self.page.as_ref().ok_or(TestError::PageNotInitialized)?;

        if let Some(delays) = &self.config.delay {
            for (regex_string, delay_ms) in delays {
                let delay = Duration::from_millis(*delay_ms);
                let pattern = regex_string.clone();

                match self.config.delay_using {
                    DelayMethod::Fulfill => {
                        page.route(&pattern, move |route, request| {
                            let delay = delay;
                            let pattern = pattern.clone();
                            async move {
                                log(&format!(
                                    "Fetching {} (matched /{}/i), delaying for {}ms",
                                    request.url(),
                                    pattern,
                                    delay.as_millis()
                                ));

                                let response = route.fetch().await?;
                                tokio::time::sleep(delay).await;
                                route.fulfill_with_response(response).await
                            }
                        }).await?;
                    }
                    DelayMethod::Continue => {
                        page.route(&pattern, move |route, request| {
                            let delay = delay;
                            let pattern = pattern.clone();
                            async move {
                                log(&format!(
                                    "Delaying {} (matched /{}/i) for {}ms",
                                    request.url(),
                                    pattern,
                                    delay.as_millis()
                                ));

                                tokio::time::sleep(delay).await;
                                route.continue_().await
                            }
                        }).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Apply network throttling
    async fn throttle_network(&self, connection_type: ConnectionType) -> Result<(), TestError> {
        if let Some(profile) = connection_type.network_profile() {
            // Check for Docker Desktop (would need different implementation in Rust)
            if is_docker_desktop() {
                return Err(TestError::DockerDesktopThrottlingNotSupported);
            }

            // Use system-level traffic shaping
            apply_traffic_shaping(profile).await?;
            log(&format!("Network throttling applied: down={}kbps, up={}kbps, rtt={}ms",
                profile.down, profile.up, profile.rtt));
        }
        Ok(())
    }

    /// Execute navigation
    pub async fn do_navigation(&mut self) -> Result<(), TestError> {
        let page = self.page.as_ref().ok_or(TestError::PageNotInitialized)?;

        // Navigate with timeout
        let timeout = self.config.timeout.unwrap_or(Duration::from_secs(30));
        let result = tokio::time::timeout(timeout, async {
            page.goto(&self.config.url, NavigationOptions {
                wait_until: WaitUntil::NetworkIdle,
            }).await
        }).await;

        match result {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Timeout - set offline and continue
                page.context().set_offline(true).await?;
                Ok(())
            }
        }
    }

    /// Collect metrics from page
    pub async fn collect_metrics(&mut self) -> Result<(), TestError> {
        let page = self.page.as_ref().ok_or(TestError::PageNotInitialized)?;

        // Collect navigation timing
        let nav_timing: NavigationTiming = page.evaluate(NAV_TIMING_SCRIPT).await??;

        // Collect paint timing
        let paint_timing: Vec<PaintTiming> = page.evaluate(PAINT_TIMING_SCRIPT).await??;

        // Collect user timing
        let user_timing: Vec<UserTiming> = page.evaluate(USER_TIMING_SCRIPT).await??;

        // Collect LCP
        let lcp: Vec<LcpEvent> = page.evaluate(LCP_SCRIPT).await??;

        // Collect layout shifts
        let layout_shifts: Vec<LayoutShift> = page.evaluate(LAYOUT_SHIFT_SCRIPT).await??;

        // Collect resource timing
        self.resource_timings = page.evaluate(RESOURCE_TIMING_SCRIPT).await??;

        self.metrics = Some(Metrics {
            navigation_timing: nav_timing,
            paint_timing,
            user_timing,
            largest_contentful_paint: lcp,
            layout_shifts,
        });

        Ok(())
    }

    /// Take screenshot
    pub async fn screenshot(&self) -> Result<(), TestError> {
        let page = self.page.as_ref().ok_or(TestError::PageNotInitialized)?;
        let screenshot_path = self.paths.results.join("screenshot.png");

        let bytes = page.screenshot(ScreenshotOptions {
            path: Some(screenshot_path.clone()),
            full_page: false,
        }).await?;

        Ok(())
    }

    /// Post-process results
    pub async fn post_process(&mut self) -> Result<(), TestError> {
        // Stop network throttling
        if self.config.connection_type.is_some() {
            stop_traffic_shaping().await?;
        }

        // Enhance HAR file
        self.enhance_har().await?;

        // Write console messages
        let console_path = self.paths.results.join("console.json");
        std::fs::write(&console_path, serde_json::to_string_pretty(&self.console_messages)?)?;

        // Write metrics
        if let Some(metrics) = &self.metrics {
            let metrics_path = self.paths.results.join("metrics.json");
            std::fs::write(&metrics_path, serde_json::to_string_pretty(metrics)?)?;
        }

        // Write resource timings
        let resources_path = self.paths.results.join("resources.json");
        std::fs::write(&resources_path, serde_json::to_string_pretty(&self.resource_timings)?)?;

        // Generate filmstrip from video
        self.create_filmstrip().await?;

        // Generate HTML report if requested
        if self.config.html {
            self.generate_html_report().await?;
        }

        // Create zip if requested
        if self.config.zip {
            self.create_zip().await?;
        }

        // Upload if requested
        if let Some(upload_url) = &self.config.upload_url {
            self.upload_results(upload_url).await?;
        }

        // Cleanup
        self.cleanup().await?;

        Ok(())
    }

    /// Enhance HAR file with additional timing data
    async fn enhance_har(&mut self) -> Result<(), TestError> {
        let har_path = self.paths.results.join("pageload.har");
        let har_content = std::fs::read_to_string(&har_path)?;
        let mut har_data: serde_json::Value = serde_json::from_str(&har_content)?;

        // Calculate TTFB
        if let Some(metrics) = &self.metrics {
            let ttfb = metrics.navigation_timing.ttfb();
            if let Some(pages) = har_data["log"]["pages"].as_array_mut() {
                if let Some(page) = pages.get_mut(0) {
                    page["pageTimings"]["_TTFB"] = serde_json::json!(ttfb);

                    // Calculate LCP
                    if let Some(lcp) = metrics.largest_contentful_paint.last() {
                        page["pageTimings"]["_LCP"] = serde_json::json!(lcp.start_time);
                    }
                }
            }
        }

        // Merge request timings with HAR entries
        self.merge_har_entries(&mut har_data).await?;

        // Write enhanced HAR
        std::fs::write(&har_path, serde_json::to_string_pretty(&har_data)?)?;

        Ok(())
    }

    /// Merge collected request timings with HAR entries
    async fn merge_har_entries(&self, har_data: &mut serde_json::Value) -> Result<(), TestError> {
        // Implementation would match HAR entries with collected timings
        // Similar to TypeScript mergeEntries function
        Ok(())
    }

    /// Create filmstrip from video
    async fn create_filmstrip(&mut self) -> Result<(), TestError> {
        // Would use ffmpeg to extract frames
        // Implementation depends on FFmpeg binding choice
        Ok(())
    }

    /// Generate HTML report
    async fn generate_html_report(&self) -> Result<(), TestError> {
        // Would use askama or tera for template rendering
        Ok(())
    }

    /// Create zip of results
    async fn create_zip(&self) -> Result<(), TestError> {
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        let zip_path = self.paths.results.with_extension("zip");
        let file = std::fs::File::create(&zip_path)?;
        let mut zip = ZipWriter::new(file);

        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Add all files from results directory
        for entry in walkdir::WalkDir::new(&self.paths.results) {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let relative_path = path.strip_prefix(&self.paths.results)?;
                zip.start_file(relative_path.to_string_lossy(), options)?;

                let mut file = std::fs::File::open(path)?;
                std::io::copy(&mut file, &mut zip)?;
            }
        }

        zip.finish()?;

        Ok(())
    }

    /// Upload results to remote URL
    async fn upload_results(&self, upload_url: &url::Url) -> Result<(), TestError> {
        let zip_path = self.paths.results.with_extension("zip");
        let zip_data = std::fs::read(&zip_path)?;

        let client = reqwest::Client::new();
        client
            .post(upload_url.as_str())
            .body(zip_data)
            .header("Content-Type", "application/zip")
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Cleanup temporary files
    pub async fn cleanup(&mut self) -> Result<(), TestError> {
        log("Cleanup started");

        // Close browser
        if let Some(browser) = self.browser.take() {
            browser.close().await?;
            log("Browser instance closed");
        }

        // Remove temporary context directory
        if self.paths.temporary_context.exists() {
            std::fs::remove_dir_all(&self.paths.temporary_context)?;
            log("Cleanup ended");
        }

        log(&format!("Test ID: {}", self.test_id));

        Ok(())
    }

    /// Get test ID
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    /// Get results path
    pub fn results_path(&self) -> &Path {
        &self.paths.results
    }
}
```

---

## Valtron Integration

### Using Valtron Instead of Async/Await

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, FnReady, NoSpawner};

/// Navigation task using valtron TaskIterator
pub struct NavigateTask {
    url: String,
    timeout: Duration,
    page: *mut Page,  // Raw pointer from FFI
    state: NavigateState,
}

enum NavigateState {
    Starting,
    Waiting,
    Complete,
    Timeout,
}

impl TaskIterator for NavigateTask {
    type Pending = ();
    type Ready = Result<(), TestError>;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            NavigateState::Starting => {
                // Start navigation
                unsafe {
                    (*self.page).goto(&self.url);
                }
                self.state = NavigateState::Waiting;
                Some(TaskStatus::Pending(()))
            }
            NavigateState::Waiting => {
                // Check if navigation complete
                unsafe {
                    if (*self.page).is_navigation_complete() {
                        self.state = NavigateState::Complete;
                        Some(TaskStatus::Ready(Ok(())))
                    } else {
                        Some(TaskStatus::Pending(()))
                    }
                }
            }
            NavigateState::Complete => None,  // Task done
            NavigateState::Timeout => {
                Some(TaskStatus::Ready(Err(TestError::NavigationTimeout)))
            }
        }
    }
}

/// Metrics collection task
pub struct CollectMetricsTask {
    page: *mut Page,
    metrics: Rc<RefCell<Option<Metrics>>>,
    state: MetricsState,
}

enum MetricsState {
    CollectingNavTiming,
    CollectingPaintTiming,
    CollectingUserTiming,
    CollectingLcp,
    CollectingLayoutShifts,
    Complete,
}

impl TaskIterator for CollectMetricsTask {
    type Pending = ();
    type Ready = Result<(), TestError>;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            MetricsState::CollectingNavTiming => {
                let nav_timing = unsafe { (*self.page).evaluate(NAV_TIMING_SCRIPT) };
                self.metrics.borrow_mut().as_mut().unwrap().navigation_timing = nav_timing;
                self.state = MetricsState::CollectingPaintTiming;
                Some(TaskStatus::Pending(()))
            }
            MetricsState::CollectingPaintTiming => {
                let paint_timing = unsafe { (*self.page).evaluate(PAINT_TIMING_SCRIPT) };
                self.metrics.borrow_mut().as_mut().unwrap().paint_timing = paint_timing;
                self.state = MetricsState::CollectingUserTiming;
                Some(TaskStatus::Pending(()))
            }
            // ... continue for other metrics
            MetricsState::Complete => {
                Some(TaskStatus::Ready(Ok(())))
            }
        }
    }
}
```

### Executor Integration

```rust
use foundation_core::valtron::single::{initialize, run_until_complete, spawn};

/// Run test using single-threaded valtron executor
pub fn run_test_single_threaded(config: LaunchOptions) -> Result<TestResult, TestError> {
    // Initialize executor with seed
    initialize(42);

    // Create test runner
    let mut runner = TestRunner::new(config)?;

    // Spawn test task
    let result = Rc::new(RefCell::new(None));
    let result_clone = result.clone();

    spawn()
        .with_task(TestRunnerTask::new(&mut runner))
        .with_resolver(Box::new(FnReady::new(move |item, _executor| {
            *result_clone.borrow_mut() = Some(item);
        })))
        .schedule()
        .expect("should schedule test task");

    // Run to completion
    run_until_complete();

    // Extract result
    result.borrow().take().unwrap()
}

/// Test runner as valtron task
pub struct TestRunnerTask<'a> {
    runner: &'a mut TestRunner,
    step: TestStep,
}

enum TestStep {
    Setup,
    SetupComplete,
    Navigate,
    NavigateComplete,
    CollectMetrics,
    MetricsComplete,
    Screenshot,
    ScreenshotComplete,
    PostProcess,
    Complete,
}

impl<'a> TaskIterator for TestRunnerTask<'a> {
    type Pending = ();
    type Ready = TestResult;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.step {
            TestStep::Setup => {
                // Setup would be async in real impl
                self.step = TestStep::SetupComplete;
                Some(TaskStatus::Pending(()))
            }
            TestStep::SetupComplete => {
                self.step = TestStep::Navigate;
                Some(TaskStatus::Pending(()))
            }
            TestStep::Navigate => {
                self.step = TestStep::NavigateComplete;
                Some(TaskStatus::Pending(()))
            }
            TestStep::NavigateComplete => {
                self.step = TestStep::CollectMetrics;
                Some(TaskStatus::Pending(()))
            }
            TestStep::CollectMetrics => {
                self.step = TestStep::MetricsComplete;
                Some(TaskStatus::Pending(()))
            }
            TestStep::MetricsComplete => {
                self.step = TestStep::Screenshot;
                Some(TaskStatus::Pending(()))
            }
            TestStep::ScreenshotComplete => {
                self.step = TestStep::PostProcess;
                Some(TaskStatus::Pending(()))
            }
            TestStep::PostProcess => {
                self.step = TestStep::Complete;
                Some(TaskStatus::Ready(TestResult::success(
                    self.runner.test_id().to_string(),
                    self.runner.results_path().to_path_buf(),
                )))
            }
            TestStep::Complete => None,
        }
    }
}
```

---

## Complete Example

### Main Entry Point

```rust
use std::path::PathBuf;

/// Main entry point for telescope Rust implementation
pub fn launch_test(options: LaunchOptions) -> Result<TestResult, TestError> {
    // Validate options
    if options.url.is_empty() {
        return Err(TestError::UrlRequired);
    }

    // Create test runner
    let mut runner = TestRunner::new(options)?;

    // Save configuration
    runner.save_config()?;

    // Dry run check
    if runner.config.dry {
        runner.cleanup().await?;
        return Ok(TestResult::dry_run(
            runner.test_id().to_string(),
            runner.results_path().to_path_buf(),
        ));
    }

    // Run test (using valtron executor)
    run_test_single_threaded(runner.config.clone())
}

/// Generate unique test ID
fn generate_test_id() -> String {
    use uuid::Uuid;
    use chrono::Utc;

    let now = Utc::now();
    format!(
        "{}_{}",
        now.format("%Y_%m_%d_%H_%M_%S"),
        Uuid::new_v4().to_string().replace("-", "")
    )
}

/// Simple logging function
fn log(message: &str) {
    if std::env::var("DEBUG").is_ok() {
        println!("[telescope] {}", message);
    }
}

/// Check if running in Docker Desktop
fn is_docker_desktop() -> bool {
    if !std::path::Path::new("/.dockerenv").exists() {
        return false;
    }

    std::fs::read_to_string("/proc/version")
        .map(|content| content.contains("linuxkit"))
        .unwrap_or(false)
}

/// Apply traffic shaping for network throttling
async fn apply_traffic_shaping(profile: NetworkProfile) -> Result<(), TestError> {
    // Would use system commands or netlink for traffic shaping
    // This is a placeholder
    Ok(())
}

/// Stop traffic shaping
async fn stop_traffic_shaping() -> Result<(), TestError> {
    Ok(())
}
```

---

## Summary

| Topic | Key Points |
|-------|------------|
| Type Translation | TypeScript interfaces → Rust structs/enums |
| Ownership | TestRunner owns config, borrows page |
| Error Handling | Result<T, E> instead of throw/catch |
| Valtron | TaskIterator instead of async/await |
| Browser FFI | Raw pointers for Playwright bindings |

---

## Next Steps

Continue to [production-grade.md](production-grade.md) for production deployment considerations or [05-valtron-integration.md](05-valtron-integration.md) for Lambda deployment patterns.
