# utm-dev Production - Testing Infrastructure Rust Revision

## Overview

This document provides a comprehensive Rust implementation for testing infrastructure, covering unit testing, integration testing, E2E test orchestration, device farm integration, screenshot testing, and CI/CD automation. The implementation replaces Go, Swift, Kotlin, and JavaScript-based testing tooling with idiomatic Rust.

**Key Goals:**
- Unified test orchestration across all platforms
- Native integration with XCUITest, Espresso, Maestro, and Detox
- Device farm integration (Firebase Test Lab, AWS Device Farm)
- Screenshot/regression testing utilities
- Async-first test execution for parallel test runs
- Comprehensive test reporting and artifact collection

## Workspace Structure

```
utm-testing/
├── Cargo.toml                 # Workspace root
├── README.md
├── utm-testing-core/          # Core traits and types
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── traits.rs          # Testing trait definitions
│       ├── error.rs           # Unified error types
│       ├── config.rs          # Test configuration
│       ├── runner.rs          # Test runner infrastructure
│       └── results.rs         # Test result types
├── utm-testing-unit/          # Unit testing framework
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── assertions.rs      # Assertion macros
│       ├── mocks.rs           # Mocking utilities
│       └── fixtures.rs        # Test fixtures
├── utm-testing-integration/   # Integration test orchestration
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── xcuitest.rs        # XCUITest runner
│       ├── espresso.rs        # Espresso runner
│       └── device.rs          # Device management
├── utm-testing-e2e/           # E2E test orchestration
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── maestro.rs         # Maestro flow runner
│       ├── detox.rs           # Detox integration
│       └── scenarios.rs       # E2E scenario definitions
├── utm-testing-screenshots/   # Screenshot testing
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── capture.rs         # Screenshot capture
│       ├── compare.rs         # Image comparison
│       └── snapshots.rs       # Snapshot management
├── utm-testing-device-farm/   # Device farm integration
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── firebase.rs        # Firebase Test Lab
│       ├── aws.rs             # AWS Device Farm
│       └── devices.rs         # Device configurations
├── utm-testing-reporter/      # Test reporting
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── junit.rs           # JUnit XML output
│       ├── html.rs            # HTML reports
│       └── ci.rs              # CI-specific formatters
└── utm-testing-cli/           # CLI tool
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── commands/
        │   ├── run.rs
        │   ├── list.rs
        │   └── report.rs
        └── utils.rs
```

## Crate Breakdown

| Crate | Purpose | Platforms |
|-------|---------|-----------|
| `utm-testing-core` | Shared traits, types, results | All |
| `utm-testing-unit` | Unit testing framework & assertions | All |
| `utm-testing-integration` | XCUITest/Espresso orchestration | macOS, Linux |
| `utm-testing-e2e` | Maestro/Detox E2E orchestration | All |
| `utm-testing-screensshots` | S.capture, comparison, regression | All |
| `utm-testing-device-farm` | Firebase/AWS device farm | All |
| `utm-testing-reporter` | Test result reporting | All |
| `utm-testing-cli` | Command-line interface | All |

## Recommended Dependencies

### utm-testing-core/Cargo.toml
```toml
[package]
name = "utm-testing-core"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
```

### utm-testing-unit/Cargo.toml
```toml
[package]
name = "utm-testing-unit"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-testing-core = { path = "../utm-testing-core" }
thiserror = "1.0"
pretty_assertions = "1.4"
mockall = "0.12"
rstest = "0.18"
```

### utm-testing-e2e/Cargo.toml
```toml
[package]
name = "utm-testing-e2e"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-testing-core = { path = "../utm-testing-core" }
thiserror = "1.0"
tokio = { version = "1.0", features = ["process", "fs"] }
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
tracing = "0.1"
```

### utm-testing-device-farm/Cargo.toml
```toml
[package]
name = "utm-testing-device-farm"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-testing-core = { path = "../utm-testing-core" }
thiserror = "1.0"
tokio = { version = "1.0", features = ["fs"] }
reqwest = { version = "0.11", features = ["json", "multipart"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
aws-config = "1.1"
aws-sdk-devicefarm = "1.0"
tracing = "0.1"
```

## Type System Design

### Core Types (utm-testing-core)

```rust
// utm-testing-core/src/results.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Unique test run identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunId(pub String);

impl TestRunId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for TestRunId {
    fn default() -> Self {
        Self::new()
    }
}

/// Test execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
    Error,
}

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Unique test identifier
    pub id: String,
    /// Human-readable test name
    pub name: String,
    /// Test suite/class name
    pub suite: String,
    /// Execution status
    pub status: TestStatus,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
    /// Full stack trace if failed
    pub stack_trace: Option<String>,
    /// Attached artifacts (screenshots, logs, etc.)
    pub artifacts: Vec<TestArtifact>,
    /// Timestamp when test started
    pub started_at: DateTime<Utc>,
    /// Timestamp when test completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Platform this test ran on
    pub platform: TestPlatform,
    /// Device/emulator identifier
    pub device_id: Option<String>,
}

/// Test artifact (screenshot, log, video, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestArtifact {
    /// Artifact name
    pub name: String,
    /// Artifact type
    pub artifact_type: ArtifactType,
    /// Path to artifact file
    pub path: String,
    /// MIME type
    pub mime_type: Option<String>,
    /// File size in bytes
    pub size_bytes: Option<u64>,
}

/// Type of test artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactType {
    Screenshot,
    Log,
    Video,
    Trace,
    Coverage,
    Other,
}

/// Target platform for testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestPlatform {
    Macos,
    Ios,
    IosSimulator,
    Android,
    AndroidEmulator,
    Windows,
    Linux,
    Web,
}

/// Aggregated test run results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResults {
    /// Unique run identifier
    pub run_id: TestRunId,
    /// Test suite name
    pub suite_name: String,
    /// Platform tested
    pub platform: TestPlatform,
    /// Device configuration
    pub device_config: Option<DeviceConfig>,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub completed_at: Option<DateTime<Utc>>,
    /// Individual test results
    pub tests: Vec<TestResult>,
    /// Summary statistics
    pub summary: TestSummary,
}

/// Device configuration for test runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Device model (e.g., "iPhone 15", "Pixel 7")
    pub model: String,
    /// OS version (e.g., "17.0", "34")
    pub os_version: String,
    /// Screen size
    pub screen_size: Option<String>,
    /// Locale
    pub locale: Option<String>,
}

/// Test summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    /// Total number of tests
    pub total: u32,
    /// Number of passed tests
    pub passed: u32,
    /// Number of failed tests
    pub failed: u32,
    /// Number of skipped tests
    pub skipped: u32,
    /// Number of errors
    pub errors: u32,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Pass rate percentage (0.0 - 100.0)
    pub pass_rate: f64,
}

impl TestSummary {
    pub fn new() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            errors: 0,
            total_duration_ms: 0,
            pass_rate: 0.0,
        }
    }

    pub fn from_results(results: &[TestResult]) -> Self {
        let total = results.len() as u32;
        let passed = results.iter().filter(|r| r.status == TestStatus::Passed).count() as u32;
        let failed = results.iter().filter(|r| r.status == TestStatus::Failed).count() as u32;
        let skipped = results.iter().filter(|r| r.status == TestStatus::Skipped).count() as u32;
        let errors = results.iter().filter(|r| r.status == TestStatus::Error).count() as u32;
        let total_duration_ms = results.iter().map(|r| r.duration_ms).sum();
        let pass_rate = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Self {
            total,
            passed,
            failed,
            skipped,
            errors,
            total_duration_ms,
            pass_rate,
        }
    }
}

impl Default for TestSummary {
    fn default() -> Self {
        Self::new()
    }
}
```

### Testing Traits (utm-testing-core)

```rust
// utm-testing-core/src/traits.rs
use crate::results::{TestResult, TestRunResults, TestPlatform};
use crate::config::TestConfig;
use crate::error::TestResult as TestResultType;

/// Primary trait for test execution
#[async_trait::async_trait]
pub trait TestRunner: Send + Sync {
    /// Get the platform this runner targets
    fn platform(&self) -> TestPlatform;

    /// Run all tests in the specified path
    async fn run_all(&self, config: &TestConfig) -> TestResultType<TestRunResults>;

    /// Run a specific test by name
    async fn run_test(&self, test_name: &str, config: &TestConfig) -> TestResultType<TestResult>;

    /// List available tests
    async fn list_tests(&self) -> TestResultType<Vec<String>>;
}

/// Integration test runner for platform-specific tests
#[async_trait::async_trait]
pub trait IntegrationTestRunner: Send + Sync {
    /// Run XCUITest suite (iOS)
    async fn run_xcuitest(
        &self,
        scheme: &str,
        destination: &str,
    ) -> TestResultType<TestRunResults>;

    /// Run Espresso tests (Android)
    async fn run_espresso(
        &self,
        project_path: &str,
        variant: Option<&str>,
    ) -> TestResultType<TestRunResults>;
}

/// E2E test runner for Maestro/Detox flows
#[async_trait::async_trait]
pub trait E2ETestRunner: Send + Sync {
    /// Run Maestro flow
    async fn run_maestro(
        &self,
        flow_path: &str,
        app_id: &str,
    ) -> TestResultType<TestResult>;

    /// Run Detox test
    async fn run_detox(
        &self,
        test_path: &str,
        configuration: &str,
    ) -> TestResultType<TestResult>;
}

/// Screenshot test runner
#[async_trait::async_trait]
pub trait ScreenshotTestRunner: Send + Sync {
    /// Capture screenshot
    async fn capture(&self, name: &str) -> TestResultType<String>;

    /// Compare screenshot with baseline
    async fn compare(
        &self,
        current: &str,
        baseline: &str,
        threshold: f64,
    ) -> TestResultType<ScreenshotComparison>;

    /// Update baseline screenshot
    async fn update_baseline(&self, name: &str, screenshot_path: &str) -> TestResultType<()>;
}

/// Device farm provider interface
#[async_trait::async_trait]
pub trait DeviceFarmProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Upload application to device farm
    async fn upload_app(
        &self,
        app_path: &str,
        app_type: &str,
    ) -> TestResultType<String>;

    /// Schedule test run on device farm
    async fn schedule_run(
        &self,
        upload_id: &str,
        device_pool: &DevicePool,
    ) -> TestResultType<TestRunId>;

    /// Get test run results
    async fn get_results(&self, run_id: &TestRunId) -> TestResultType<TestRunResults>;

    /// Wait for test run to complete
    async fn wait_for_completion(
        &self,
        run_id: &TestRunId,
        timeout_secs: u64,
    ) -> TestResultType<TestRunResults>;
}

/// Device pool configuration
#[derive(Debug, Clone)]
pub struct DevicePool {
    pub devices: Vec<DeviceSpec>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeviceSpec {
    pub model: String,
    pub os_version: String,
    pub locale: Option<String>,
}
```

### Error Types (utm-testing-core)

```rust
// utm-testing-core/src/error.rs
use thiserror::Error;

/// Unified testing error type
#[derive(Error, Debug)]
pub enum TestError {
    #[error("Test execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Test configuration error: {0}")]
    ConfigurationError(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(&'static str),

    #[error("Device not available: {0}")]
    DeviceNotAvailable(String),

    #[error("Test timeout: {0}")]
    Timeout(String),

    #[error("Assertion failed: {0}")]
    AssertionFailed(String),

    #[error("Process execution error: {0}")]
    ProcessError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Device farm error: {0}")]
    DeviceFarmError(String),

    #[error("Screenshot comparison failed: {0}")]
    ScreenshotComparisonFailed(String),

    #[error("Test suite not found: {0}")]
    SuiteNotFound(String),

    #[error("Test not found: {0}")]
    TestNotFound(String),
}

pub type TestResult<T> = Result<T, TestError>;
```

### Test Configuration (utm-testing-core)

```rust
// utm-testing-core/src/config.rs
use serde::{Deserialize, Serialize};
use crate::results::TestPlatform;

/// Test execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Platform to test on
    pub platform: TestPlatform,
    /// Path to test files
    pub test_path: String,
    /// Filter pattern for test names
    pub filter: Option<String>,
    /// Timeout in seconds
    pub timeout_secs: Option<u64>,
    /// Number of retries for failed tests
    pub retries: Option<u32>,
    /// Enable parallel execution
    pub parallel: bool,
    /// Number of parallel workers
    pub workers: Option<usize>,
    /// Output directory for artifacts
    pub output_dir: Option<String>,
    /// Enable verbose output
    pub verbose: bool,
    /// Environment variables to set
    pub env: std::collections::HashMap<String, String>,
    /// Device configuration
    pub device: Option<DeviceConfig>,
}

impl TestConfig {
    pub fn new(platform: TestPlatform, test_path: &str) -> Self {
        Self {
            platform,
            test_path: test_path.to_string(),
            filter: None,
            timeout_secs: None,
            retries: None,
            parallel: false,
            workers: None,
            output_dir: None,
            verbose: false,
            env: std::collections::HashMap::new(),
            device: None,
        }
    }

    pub fn with_filter(mut self, filter: &str) -> Self {
        self.filter = Some(filter.to_string());
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = Some(retries);
        self
    }

    pub fn parallel(mut self, workers: Option<usize>) -> Self {
        self.parallel = true;
        self.workers = workers;
        self
    }

    pub fn with_output_dir(mut self, dir: &str) -> Self {
        self.output_dir = Some(dir.to_string());
        self
    }

    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }
}

/// Device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub model: String,
    pub os_version: String,
    pub screen_size: Option<String>,
    pub locale: Option<String>,
}
```

## Unit Testing Framework (utm-testing-unit)

```rust
// utm-testing-unit/src/lib.rs
mod assertions;
mod mocks;
mod fixtures;

pub use assertions::*;
pub use mocks::*;
pub use fixtures::*;

use utm_testing_core::{TestResult, TestStatus, TestResult as TestResultType};

/// Assertion macro for testing
#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr) => {{
        let left = &$left;
        let right = &$right;
        if left != right {
            return Err(utm_testing_core::TestError::AssertionFailed(
                format!("assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`", left, right)
            ));
        }
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let left = &$left;
        let right = &$right;
        if left != right {
            return Err(utm_testing_core::TestError::AssertionFailed(
                format!("assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`\n  {}", left, right, format!($($arg)+))
            ));
        }
    }};
}

#[macro_export]
macro_rules! assert_ok {
    ($result:expr) => {{
        match $result {
            Ok(v) => v,
            Err(e) => return Err(utm_testing_core::TestError::AssertionFailed(
                format!("expected Ok, got Err: {:?}", e)
            )),
        }
    }};
}

#[macro_export]
macro_rules! assert_err {
    ($result:expr) => {{
        match $result {
            Err(e) => e,
            Ok(v) => return Err(utm_testing_core::TestError::AssertionFailed(
                format!("expected Err, got Ok: {:?}", v)
            )),
        }
    }};
}

#[macro_export]
macro_rules! test_fn {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() -> utm_testing_core::TestResult<()> {
            $body
        }
    };
}
```

```rust
// utm-testing-unit/src/assertions.rs
use utm_testing_core::TestError;
use utm_testing_core::TestResult;

/// Assert that a condition is true
pub fn assert_that(condition: bool, message: &str) -> TestResult<()> {
    if !condition {
        Err(TestError::AssertionFailed(message.to_string()))
    } else {
        Ok(())
    }
}

/// Assert two values are equal
pub fn assert_equal<T: PartialEq + std::fmt::Debug>(left: &T, right: &T) -> TestResult<()> {
    if left != right {
        Err(TestError::AssertionFailed(
            format!("assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`", left, right)
        ))
    } else {
        Ok(())
    }
}

/// Assert a value contains a substring
pub fn assert_contains(haystack: &str, needle: &str) -> TestResult<()> {
    if !haystack.contains(needle) {
        Err(TestError::AssertionFailed(
            format!("expected string to contain `{}` but was: `{}`", needle, haystack)
        ))
    } else {
        Ok(())
    }
}

/// Assert a value starts with a prefix
pub fn assert_starts_with(value: &str, prefix: &str) -> TestResult<()> {
    if !value.starts_with(prefix) {
        Err(TestError::AssertionFailed(
            format!("expected string to start with `{}` but was: `{}`", prefix, value)
        ))
    } else {
        Ok(())
    }
}

/// Assert a result is Ok
pub fn assert_ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> TestResult<T> {
    match result {
        Ok(v) => Ok(v),
        Err(e) => Err(TestError::AssertionFailed(
            format!("expected Ok, got Err: {:?}", e)
        )),
    }
}

/// Assert a result is Err
pub fn assert_err<T, E: std::fmt::Debug>(result: Result<T, E>) -> TestResult<E> {
    match result {
        Err(e) => Ok(e),
        Ok(v) => Err(TestError::AssertionFailed(
            format!("expected Err, got Ok: {:?}", v)
        )),
    }
}

/// Assert a value is not empty
pub fn assert_not_empty<T: IsEmpty>(value: &T) -> TestResult<()> {
    if value.is_empty() {
        Err(TestError::AssertionFailed("expected value to not be empty".to_string()))
    } else {
        Ok(())
    }
}

pub trait IsEmpty {
    fn is_empty(&self) -> bool;
}

impl<T> IsEmpty for Vec<T> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl IsEmpty for String {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<'a, T> IsEmpty for &'a [T] {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}
```

```rust
// utm-testing-unit/src/mocks.rs
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use utm_testing_core::TestResult;

/// Simple mock builder for creating test doubles
pub struct MockBuilder<T> {
    name: String,
    behaviors: HashMap<String, Box<dyn Fn() -> T + Send + Sync>>,
}

impl<T: Send + Sync + 'static> MockBuilder<T> {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            behaviors: HashMap::new(),
        }
    }

    pub fn when<F: Fn() -> T + Send + Sync + 'static>(mut self, method: &str, behavior: F) -> Self {
        self.behaviors.insert(method.to_string(), Box::new(behavior));
        self
    }

    pub fn build(self) -> Mock<T> {
        Mock {
            name: self.name,
            behaviors: Arc::new(Mutex::new(self.behaviors)),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

/// Mock object for testing
pub struct Mock<T> {
    name: String,
    behaviors: Arc<Mutex<HashMap<String, Box<dyn Fn() -> T + Send + Sync>>>>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl<T: Clone + Send + Sync + 'static> Mock<T> {
    pub fn call(&self, method: &str) -> TestResult<T> {
        let behaviors = self.behaviors.lock().unwrap();
        self.calls.lock().unwrap().push(method.to_string());

        if let Some(behavior) = behaviors.get(method) {
            Ok(behavior())
        } else {
            Err(utm_testing_core::TestError::ExecutionFailed(
                format!("No behavior defined for method: {}", method)
            ))
        }
    }

    pub fn verify_call(&self, method: &str) -> TestResult<()> {
        let calls = self.calls.lock().unwrap();
        if calls.contains(&method.to_string()) {
            Ok(())
        } else {
            Err(utm_testing_core::TestError::AssertionFailed(
                format!("Expected method {} to be called but it wasn't", method)
            ))
        }
    }

    pub fn verify_call_count(&self, method: &str, expected: usize) -> TestResult<()> {
        let calls = self.calls.lock().unwrap();
        let count = calls.iter().filter(|c| *c == method).count();
        if count == expected {
            Ok(())
        } else {
            Err(utm_testing_core::TestError::AssertionFailed(
                format!("Expected method {} to be called {} times but was {}", method, expected, count)
            ))
        }
    }
}
```

## Integration Test Orchestration (utm-testing-integration)

```rust
// utm-testing-integration/src/lib.rs
mod xcuitest;
mod espresso;
mod device;

pub use xcuitest::XCUITestRunner;
pub use espresso::EspressoRunner;
pub use device::DeviceManager;

use utm_testing_core::{TestRunner, TestPlatform, TestConfig, TestResult, TestRunResults};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, debug, error};

/// iOS simulator configuration
#[derive(Debug, Clone)]
pub struct IOSSimulatorConfig {
    pub device_name: String,
    pub os_version: String,
}

impl IOSSimulatorConfig {
    pub fn iphone_15() -> Self {
        Self {
            device_name: "iPhone 15".to_string(),
            os_version: "17.0".to_string(),
        }
    }

    pub fn ipad_pro() -> Self {
        Self {
            device_name: "iPad Pro (12.9-inch) (6th generation)".to_string(),
            os_version: "17.0".to_string(),
        }
    }
}

/// Android emulator configuration
#[derive(Debug, Clone)]
pub struct AndroidEmulatorConfig {
    pub avd_name: String,
    pub api_level: u32,
}

impl AndroidEmulatorConfig {
    pub fn pixel_7(api_level: u32) -> Self {
        Self {
            avd_name: "pixel_7".to_string(),
            api_level,
        }
    }
}
```

```rust
// utm-testing-integration/src/xcuitest.rs
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{Duration, timeout};
use tracing::{info, debug, error};
use utm_testing_core::{
    TestRunner, TestPlatform, TestConfig, TestResult, TestRunResults,
    TestResult as TestResultType, TestSummary, TestStatus, TestArtifact, ArtifactType,
};
use crate::device::IOSSimulatorConfig;

/// XCUITest runner for iOS integration tests
pub struct XCUITestRunner {
    scheme: String,
    workspace: Option<String>,
    project: Option<String>,
}

impl XCUITestRunner {
    pub fn new(scheme: &str) -> Self {
        Self {
            scheme: scheme.to_string(),
            workspace: None,
            project: None,
        }
    }

    pub fn with_workspace(mut self, workspace: &str) -> Self {
        self.workspace = Some(workspace.to_string());
        self
    }

    pub fn with_project(mut self, project: &str) -> Self {
        self.project = Some(project.to_string());
        self
    }

    /// Run XCUITest suite
    pub async fn run(
        &self,
        destination: &str,
        config: &TestConfig,
    ) -> TestResultType<TestRunResults> {
        info!("Running XCUITest scheme: {} on {}", self.scheme, destination);

        let mut cmd = Command::new("xcodebuild");
        cmd.args([
            "-scheme", &self.scheme,
            "-destination", destination,
            "test",
        ]);

        if let Some(workspace) = &self.workspace {
            cmd.arg("-workspace").arg(workspace);
        }

        if let Some(project) = &self.project {
            cmd.arg("-project").arg(project);
        }

        // Add test result output
        let result_bundle = config.output_dir
            .as_ref()
            .map(|d| format!("{}/test-results.xcresult", d))
            .unwrap_or_else(|| "test-results.xcresult".to_string());

        cmd.arg("-resultBundlePath").arg(&result_bundle);

        if config.verbose {
            cmd.arg("-verbose");
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        debug!("Running xcodebuild: {:?}", cmd);

        // Run with timeout
        let output = if let Some(timeout_secs) = config.timeout_secs {
            timeout(Duration::from_secs(timeout_secs), cmd.output())
                .await
                .map_err(|_| utm_testing_core::TestError::Timeout(
                    format!("Test timed out after {} seconds", timeout_secs)
                ))?
        } else {
            cmd.output().await
        }
        .map_err(|e| utm_testing_core::TestError::ProcessError(
            format!("Failed to execute xcodebuild: {}", e)
        ))?;

        // Parse results
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !success {
            error!("XCUITest failed:\n{}", stderr);
        }

        // Parse test results from xcodebuild output
        let tests = self.parse_test_output(&stdout);
        let summary = TestSummary::from_results(&tests);

        Ok(TestRunResults {
            run_id: utm_testing_core::TestRunId::new(),
            suite_name: self.scheme.clone(),
            platform: TestPlatform::Ios,
            device_config: None,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            tests,
            summary,
        })
    }

    /// Parse test output from xcodebuild
    fn parse_test_output(&self, output: &str) -> Vec<utm_testing_core::TestResult> {
        // Simplified parsing - real implementation would parse XCTest output format
        let mut tests = Vec::new();

        for line in output.lines() {
            if line.contains("Test Case") && line.contains("passed") {
                // Extract test name and add as passed
                if let Some(name) = self.extract_test_name(line) {
                    tests.push(utm_testing_core::TestResult {
                        id: name.clone(),
                        name: name.clone(),
                        suite: self.scheme.clone(),
                        status: TestStatus::Passed,
                        duration_ms: self.extract_duration(line),
                        error: None,
                        stack_trace: None,
                        artifacts: Vec::new(),
                        started_at: chrono::Utc::now(),
                        completed_at: Some(chrono::Utc::now()),
                        platform: TestPlatform::Ios,
                        device_id: None,
                    });
                }
            }
        }

        tests
    }

    fn extract_test_name(&self, line: &str) -> Option<String> {
        // Extract test name from XCTest output
        let parts: Vec<&str> = line.split("'").collect();
        if parts.len() >= 2 {
            Some(parts[1].to_string())
        } else {
            None
        }
    }

    fn extract_duration(&self, line: &str) -> u64 {
        // Extract duration from XCTest output (simplified)
        100 // Default 100ms
    }
}

#[async_trait::async_trait]
impl TestRunner for XCUITestRunner {
    fn platform(&self) -> TestPlatform {
        TestPlatform::Ios
    }

    async fn run_all(&self, config: &TestConfig) -> TestResultType<TestRunResults> {
        let destination = format!(
            "platform=iOS Simulator,name={},OS={}",
            config.device.as_ref().map(|d| d.model.as_str()).unwrap_or("iPhone 15"),
            config.device.as_ref().map(|d| d.os_version.as_str()).unwrap_or("17.0"),
        );

        self.run(&destination, config).await
    }

    async fn run_test(&self, test_name: &str, config: &TestConfig) -> TestResultType<utm_testing_core::TestResult> {
        let destination = format!(
            "platform=iOS Simulator,name={},OS={}",
            config.device.as_ref().map(|d| d.model.as_str()).unwrap_or("iPhone 15"),
            config.device.as_ref().map(|d| d.os_version.as_str()).unwrap_or("17.0"),
        );

        let mut cmd = Command::new("xcodebuild");
        cmd.args([
            "-scheme", &self.scheme,
            "-destination", &destination,
            "-only-testing", &format!("{} {}", self.scheme, test_name),
            "test",
        ]);

        // Run test...
        unimplemented!()
    }

    async fn list_tests(&self) -> TestResultType<Vec<String>> {
        // List tests using xcodebuild -showBuildSettings or schemes
        unimplemented!()
    }
}
```

```rust
// utm-testing-integration/src/espresso.rs
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{Duration, timeout};
use tracing::{info, debug, error};
use utm_testing_core::{
    TestRunner, TestPlatform, TestConfig, TestResult, TestRunResults,
    TestResult as TestResultType, TestSummary, TestStatus,
};

/// Espresso test runner for Android integration tests
pub struct EspressoRunner {
    project_path: String,
    variant: Option<String>,
}

impl EspressoRunner {
    pub fn new(project_path: &str) -> Self {
        Self {
            project_path: project_path.to_string(),
            variant: None,
        }
    }

    pub fn with_variant(mut self, variant: &str) -> Self {
        self.variant = Some(variant.to_string());
        self
    }

    /// Run Espresso tests
    pub async fn run(&self, config: &TestConfig) -> TestResultType<TestRunResults> {
        info!("Running Espresso tests in: {}", self.project_path);

        let task = match &self.variant {
            Some(v) => format!("{}ConnectedAndroidTest", v),
            None => "connectedAndroidTest".to_string(),
        };

        let mut cmd = Command::new("./gradlew");
        cmd.arg(&task)
            .current_dir(&self.project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if config.verbose {
            cmd.arg("--info");
        }

        debug!("Running gradle: {:?}", cmd);

        // Run with timeout
        let output = if let Some(timeout_secs) = config.timeout_secs {
            timeout(Duration::from_secs(timeout_secs), cmd.output())
                .await
                .map_err(|_| utm_testing_core::TestError::Timeout(
                    format!("Test timed out after {} seconds", timeout_secs)
                ))?
        } else {
            cmd.output().await
        }
        .map_err(|e| utm_testing_core::TestError::ProcessError(
            format!("Failed to execute gradle: {}", e)
        ))?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !success {
            error!("Espresso tests failed:\n{}", stderr);
        }

        // Parse test results from Gradle output
        let tests = self.parse_test_output(&stdout);
        let summary = TestSummary::from_results(&tests);

        Ok(TestRunResults {
            run_id: utm_testing_core::TestRunId::new(),
            suite_name: "Espresso Tests".to_string(),
            platform: TestPlatform::Android,
            device_config: None,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            tests,
            summary,
        })
    }

    fn parse_test_output(&self, output: &str) -> Vec<utm_testing_core::TestResult> {
        // Simplified parsing - real implementation would parse Gradle test output
        let mut tests = Vec::new();

        for line in output.lines() {
            if line.contains("BUILD SUCCESSFUL") {
                // All tests passed
                tests.push(utm_testing_core::TestResult {
                    id: "all_tests".to_string(),
                    name: "All Espresso Tests".to_string(),
                    suite: "Espresso".to_string(),
                    status: TestStatus::Passed,
                    duration_ms: 1000,
                    error: None,
                    stack_trace: None,
                    artifacts: Vec::new(),
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                    platform: TestPlatform::Android,
                    device_id: None,
                });
            }
        }

        tests
    }
}

#[async_trait::async_trait]
impl TestRunner for EspressoRunner {
    fn platform(&self) -> TestPlatform {
        TestPlatform::Android
    }

    async fn run_all(&self, config: &TestConfig) -> TestResultType<TestRunResults> {
        self.run(config).await
    }

    async fn run_test(&self, _test_name: &str, _config: &TestConfig) -> TestResultType<utm_testing_core::TestResult> {
        unimplemented!()
    }

    async fn list_tests(&self) -> TestResultType<Vec<String>> {
        unimplemented!()
    }
}
```

## E2E Test Orchestration (utm-testing-e2e)

```rust
// utm-testing-e2e/src/lib.rs
mod maestro;
mod detox;
mod scenarios;

pub use maestro::MaestroRunner;
pub use detox::DetoxRunner;
pub use scenarios::{E2EScenario, E2EStep, E2EAssertion};

use utm_testing_core::{E2ETestRunner, TestPlatform, TestConfig};
use serde::{Deserialize, Serialize};

/// Maestro flow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaestroFlow {
    /// App ID to test
    pub app_id: String,
    /// Flow steps
    pub steps: Vec<MaestroStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum MaestroStep {
    LaunchApp {
        clear_state: Option<bool>,
        url: Option<String>,
    },
    TapOn {
        id: Option<String>,
        text: Option<String>,
    },
    InputText {
        text: String,
    },
    AssertVisible {
        text: String,
    },
    AssertNotVisible {
        text: String,
    },
    WaitForAnimationToEnd,
    PressBack,
    StopApp,
}
```

```rust
// utm-testing-e2e/src/maestro.rs
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::fs;
use tracing::{info, debug, error};
use utm_testing_core::{
    E2ETestRunner, TestResult, TestStatus, TestPlatform,
    TestResult as TestResultType, TestArtifact, ArtifactType,
};
use crate::MaestroFlow;

/// Maestro E2E test runner
pub struct MaestroRunner {
    maestro_path: Option<String>,
}

impl MaestroRunner {
    pub fn new() -> Self {
        Self {
            maestro_path: None,
        }
    }

    pub fn with_maestro_path(mut self, path: &str) -> Self {
        self.maestro_path = Some(path.to_string());
        self
    }

    /// Run Maestro flow
    pub async fn run_flow(
        &self,
        flow_path: &Path,
        app_id: &str,
    ) -> TestResultType<utm_testing_core::TestResult> {
        info!("Running Maestro flow: {:?}", flow_path);

        let maestro = self.maestro_path.as_deref().unwrap_or("maestro");

        let mut cmd = Command::new(maestro);
        cmd.arg("run")
            .arg(flow_path)
            .arg("--app-id")
            .arg(app_id)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Running maestro: {:?}", cmd);

        let output = cmd.output().await
            .map_err(|e| utm_testing_core::TestError::ProcessError(
                format!("Failed to execute maestro: {}", e)
            ))?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let status = if success {
            TestStatus::Passed
        } else {
            error!("Maestro flow failed:\n{}", stderr);
            TestStatus::Failed
        };

        Ok(utm_testing_core::TestResult {
            id: flow_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            name: flow_path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            suite: "Maestro".to_string(),
            status,
            duration_ms: 0, // Would need to parse from output
            error: if success { None } else { Some(stderr.to_string()) },
            stack_trace: None,
            artifacts: Vec::new(),
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            platform: TestPlatform::Android, // Maestro primarily for Android
            device_id: None,
        })
    }
}

impl Default for MaestroRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl E2ETestRunner for MaestroRunner {
    async fn run_maestro(
        &self,
        flow_path: &str,
        app_id: &str,
    ) -> TestResultType<utm_testing_core::TestResult> {
        self.run_flow(Path::new(flow_path), app_id).await
    }

    async fn run_detox(
        &self,
        _test_path: &str,
        _configuration: &str,
    ) -> TestResultType<utm_testing_core::TestResult> {
        Err(utm_testing_core::TestError::PlatformNotSupported(
            "Detox tests should use DetoxRunner"
        ))
    }
}
```

```rust
// utm-testing-e2e/src/detox.rs
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, debug, error};
use utm_testing_core::{
    E2ETestRunner, TestResult, TestStatus, TestPlatform,
    TestResult as TestResultType,
};

/// Detox E2E test runner
pub struct DetoxRunner {
    config_path: Option<String>,
}

impl DetoxRunner {
    pub fn new() -> Self {
        Self {
            config_path: None,
        }
    }

    pub fn with_config(mut self, path: &str) -> Self {
        self.config_path = Some(path.to_string());
        self
    }

    /// Run Detox test
    pub async fn run_test(
        &self,
        test_path: &str,
        configuration: &str,
    ) -> TestResultType<utm_testing_core::TestResult> {
        info!("Running Detox test: {} (config: {})", test_path, configuration);

        let mut cmd = Command::new("npx");
        cmd.arg("detox")
            .arg("test")
            .arg("--configuration")
            .arg(configuration)
            .arg("--test-path-pattern")
            .arg(test_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(config_path) = &self.config_path {
            cmd.arg("--config").arg(config_path);
        }

        debug!("Running detox: {:?}", cmd);

        let output = cmd.output().await
            .map_err(|e| utm_testing_core::TestError::ProcessError(
                format!("Failed to execute detox: {}", e)
            ))?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let status = if success {
            TestStatus::Passed
        } else {
            error!("Detox test failed:\n{}", stderr);
            TestStatus::Failed
        };

        Ok(utm_testing_core::TestResult {
            id: test_path.to_string(),
            name: test_path.to_string(),
            suite: "Detox".to_string(),
            status,
            duration_ms: 0,
            error: if success { None } else { Some(stderr.to_string()) },
            stack_trace: None,
            artifacts: Vec::new(),
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            platform: TestPlatform::Ios, // Detox supports both iOS and Android
            device_id: None,
        })
    }
}

impl Default for DetoxRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl E2ETestRunner for DetoxRunner {
    async fn run_maestro(
        &self,
        _flow_path: &str,
        _app_id: &str,
    ) -> TestResultType<utm_testing_core::TestResult> {
        Err(utm_testing_core::TestError::PlatformNotSupported(
            "Maestro tests should use MaestroRunner"
        ))
    }

    async fn run_detox(
        &self,
        test_path: &str,
        configuration: &str,
    ) -> TestResultType<utm_testing_core::TestResult> {
        self.run_test(test_path, configuration).await
    }
}
```

## Screenshot Testing (utm-testing-screenshots)

```rust
// utm-testing-screenshots/src/lib.rs
mod capture;
mod compare;
mod snapshots;

pub use capture::ScreenshotCapture;
pub use compare::{ScreenshotComparator, ScreenshotComparison, ComparisonResult};
pub use snapshots::SnapshotManager;

use utm_testing_core::{TestResult, TestStatus, TestArtifact, ArtifactType};
use utm_testing_core::TestResult as TestResultType;
use std::path::{Path, PathBuf};

/// Screenshot test configuration
#[derive(Debug, Clone)]
pub struct ScreenshotConfig {
    /// Baseline directory
    pub baseline_dir: PathBuf,
    /// Output directory for new screenshots
    pub output_dir: PathBuf,
    /// Difference directory for failed comparisons
    pub diff_dir: PathBuf,
    /// Similarity threshold (0.0 - 1.0)
    pub threshold: f64,
}

impl ScreenshotConfig {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            baseline_dir: base_dir.join("baseline"),
            output_dir: base_dir.join("output"),
            diff_dir: base_dir.join("diff"),
            threshold: 0.99,
        }
    }
}
```

```rust
// utm-testing-screenshots/src/compare.rs
use image::{DynamicImage, GenericImageView, RgbImage};
use utm_testing_core::TestResult as TestResultType;
use std::path::{Path, PathBuf};

/// Result of screenshot comparison
#[derive(Debug, Clone)]
pub struct ScreenshotComparison {
    /// Path to baseline image
    pub baseline_path: PathBuf,
    /// Path to current image
    pub current_path: PathBuf,
    /// Path to difference image (if any)
    pub diff_path: Option<PathBuf>,
    /// Similarity score (0.0 - 1.0)
    pub similarity: f64,
    /// Whether images match within threshold
    pub matches: bool,
    /// Threshold used for comparison
    pub threshold: f64,
}

/// Screenshot comparator using pixel comparison
pub struct ScreenshotComparator {
    threshold: f64,
}

impl ScreenshotComparator {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Compare two screenshots
    pub fn compare(&self, baseline: &Path, current: &Path) -> TestResultType<ScreenshotComparison> {
        // Load images
        let baseline_img = image::open(baseline)
            .map_err(|e| utm_testing_core::TestError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            ))?;

        let current_img = image::open(current)
            .map_err(|e| utm_testing_core::TestError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            ))?;

        // Check dimensions match
        if baseline_img.dimensions() != current_img.dimensions() {
            return Ok(ScreenshotComparison {
                baseline_path: baseline.to_path_buf(),
                current_path: current.to_path_buf(),
                diff_path: None,
                similarity: 0.0,
                matches: false,
                threshold: self.threshold,
            });
        }

        // Calculate similarity
        let similarity = self.calculate_similarity(&baseline_img, &current_img);
        let matches = similarity >= self.threshold;

        Ok(ScreenshotComparison {
            baseline_path: baseline.to_path_buf(),
            current_path: current.to_path_buf(),
            diff_path: None,
            similarity,
            matches,
            threshold: self.threshold,
        })
    }

    /// Calculate similarity between two images
    fn calculate_similarity(&self, baseline: &DynamicImage, current: &DynamicImage) -> f64 {
        let baseline_rgb = baseline.to_rgb8();
        let current_rgb = current.to_rgb8();

        let mut matching_pixels = 0u64;
        let mut total_pixels = 0u64;

        for (x, y, pixel) in baseline_rgb.enumerate_pixels() {
            let current_pixel = current_rgb.get_pixel(x, y);
            total_pixels += 1;

            if pixel == current_pixel {
                matching_pixels += 1;
            }
        }

        if total_pixels == 0 {
            return 1.0;
        }

        matching_pixels as f64 / total_pixels as f64
    }

    /// Generate diff image highlighting differences
    pub fn generate_diff(
        &self,
        baseline: &Path,
        current: &Path,
        output_path: &Path,
    ) -> TestResultType<()> {
        let baseline_img = image::open(baseline)
            .map_err(|e| utm_testing_core::TestError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            ))?
            .to_rgb8();

        let current_img = image::open(current)
            .map_err(|e| utm_testing_core::TestError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            ))?
            .to_rgb8();

        let mut diff_img = RgbImage::new(baseline_img.width(), baseline_img.height());

        for (x, y, baseline_pixel) in baseline_img.enumerate_pixels() {
            let current_pixel = current_img.get_pixel(x, y);

            if baseline_pixel != current_pixel {
                // Highlight difference in red
                diff_img.put_pixel(x, y, image::Rgb([255, 0, 0]));
            } else {
                diff_img.put_pixel(x, y, *baseline_pixel);
            }
        }

        diff_img.save(output_path)
            .map_err(|e| utm_testing_core::TestError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            ))?;

        Ok(())
    }
}
```

## Device Farm Integration (utm-testing-device-farm)

```rust
// utm-testing-device-farm/src/lib.rs
mod firebase;
mod aws;
mod devices;

pub use firebase::FirebaseTestLab;
pub use aws::AWSDeviceFarm;
pub use devices::{DeviceSpec, DevicePool};

use utm_testing_core::{DeviceFarmProvider, TestResult, TestRunResults, TestRunId};
use utm_testing_core::TestResult as TestResultType;

/// Device farm configuration
#[derive(Debug, Clone)]
pub struct DeviceFarmConfig {
    pub provider: DeviceFarmProviderType,
    pub project_id: String,
    pub credentials: Credentials,
}

#[derive(Debug, Clone)]
pub enum DeviceFarmProviderType {
    Firebase,
    AWS,
}

#[derive(Debug, Clone)]
pub enum Credentials {
    ServiceAccountJson(String),
    AwsProfile(String),
    ApiKey(String),
}
```

```rust
// utm-testing-device-farm/src/firebase.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, debug, error};
use utm_testing_core::{
    DeviceFarmProvider, TestResult, TestRunResults, TestRunId,
    TestResult as TestResultType, DeviceConfig, TestPlatform,
};
use crate::devices::{DeviceSpec, DevicePool};

/// Firebase Test Lab client
pub struct FirebaseTestLab {
    client: Client,
    project_id: String,
    api_key: String,
    base_url: String,
}

impl FirebaseTestLab {
    pub fn new(project_id: &str, api_key: &str) -> Self {
        Self {
            client: Client::new(),
            project_id: project_id.to_string(),
            api_key: api_key.to_string(),
            base_url: "https://testing.googleapis.com/v1".to_string(),
        }
    }

    /// Upload APK to Firebase Test Lab
    pub async fn upload_apk(&self, apk_path: &str) -> TestResultType<String> {
        info!("Uploading APK to Firebase: {}", apk_path);

        // Create GCS bucket for upload
        let upload_url = format!(
            "{}/projects/{}/testMatrices",
            self.base_url, self.project_id
        );

        // In real implementation, would use GCS resumable upload
        Ok("apk-upload-id".to_string())
    }

    /// Schedule test run on Firebase Test Lab
    pub async fn schedule_run(
        &self,
        apk_id: &str,
        device_pool: &DevicePool,
    ) -> TestResultType<TestRunId> {
        info!("Scheduling Firebase Test Lab run");

        let request = FirebaseTestRequest {
            test_matrix: TestMatrix {
                test: TestConfig::Instrumentation {
                    app_apk: apk_id.to_string(),
                    test_apk: "test-apk-id".to_string(),
                },
                environment_config: EnvironmentConfig {
                    test_timeout: "300s".to_string(),
                },
                device_config: device_pool.devices.iter().map(|d| {
                    AndroidDeviceConfig {
                        android_model_id: d.model.clone(),
                        android_version_id: d.os_version.clone(),
                        locale: d.locale.clone().unwrap_or_else(|| "en".to_string()),
                    }
                }).collect(),
            },
        };

        // POST to Firebase Test Lab API
        debug!("Firebase request: {:?}", request);

        Ok(TestRunId::new())
    }

    /// Get test results
    pub async fn get_results(&self, run_id: &TestRunId) -> TestResultType<TestRunResults> {
        // Poll Firebase Test Lab API for results
        unimplemented!()
    }
}

#[derive(Debug, Serialize)]
struct FirebaseTestRequest {
    test_matrix: TestMatrix,
}

#[derive(Debug, Serialize)]
struct TestMatrix {
    test: TestConfig,
    environment_config: EnvironmentConfig,
    device_config: Vec<AndroidDeviceConfig>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum TestConfig {
    #[serde(rename = "INSTRUMENTATION_TEST")]
    Instrumentation {
        app_apk: String,
        test_apk: String,
    },
    #[serde(rename = "ROBO_TEST")]
    Robo,
}

#[derive(Debug, Serialize)]
struct EnvironmentConfig {
    test_timeout: String,
}

#[derive(Debug, Serialize)]
struct AndroidDeviceConfig {
    android_model_id: String,
    android_version_id: String,
    locale: String,
}

#[async_trait::async_trait]
impl DeviceFarmProvider for FirebaseTestLab {
    fn name(&self) -> &str {
        "Firebase Test Lab"
    }

    async fn upload_app(
        &self,
        app_path: &str,
        app_type: &str,
    ) -> TestResultType<String> {
        if app_type != "android" {
            return Err(utm_testing_core::TestError::DeviceFarmError(
                "Firebase Test Lab only supports Android apps".to_string()
            ));
        }

        self.upload_apk(app_path).await
    }

    async fn schedule_run(
        &self,
        upload_id: &str,
        device_pool: &DevicePool,
    ) -> TestResultType<TestRunId> {
        self.schedule_run(upload_id, device_pool).await
    }

    async fn get_results(&self, run_id: &TestRunId) -> TestResultType<TestRunResults> {
        self.get_results(run_id).await
    }

    async fn wait_for_completion(
        &self,
        run_id: &TestRunId,
        timeout_secs: u64,
    ) -> TestResultType<TestRunResults> {
        use tokio::time::{timeout, Duration};

        timeout(Duration::from_secs(timeout_secs), async {
            loop {
                let results = self.get_results(run_id).await?;

                // Check if run is complete
                // In real implementation, would check status field
                return Ok(results);
            }
        })
        .await
        .map_err(|_| utm_testing_core::TestError::Timeout(
            format!("Device farm run timed out after {} seconds", timeout_secs)
        ))?
    }
}
```

```rust
// utm-testing-device-farm/src/aws.rs
use tracing::{info, debug};
use utm_testing_core::{
    DeviceFarmProvider, TestResult, TestRunResults, TestRunId,
    TestResult as TestResultType,
};
use crate::devices::{DeviceSpec, DevicePool};

/// AWS Device Farm client
pub struct AWSDeviceFarm {
    client: aws_sdk_devicefarm::Client,
}

impl AWSDeviceFarm {
    pub async fn new(region: &str) -> Self {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_devicefarm::Client::new(&config);

        Self { client }
    }

    /// Create upload for application
    pub async fn create_upload(
        &self,
        project_arn: &str,
        name: &str,
        app_type: &str,
    ) -> TestResultType<String> {
        info!("Creating AWS Device Farm upload: {}", name);

        let response = self.client
            .create_upload()
            .project_arn(project_arn)
            .name(name)
            .app_type(aws_sdk_devicefarm::types::AppType::from(app_type))
            .send()
            .await
            .map_err(|e| utm_testing_core::TestError::DeviceFarmError(
                format!("Failed to create upload: {}", e)
            ))?;

        let upload_arn = response.upload()
            .and_then(|u| u.arn())
            .unwrap_or("")
            .to_string();

        Ok(upload_arn)
    }

    /// Schedule test run
    pub async fn schedule_run(
        &self,
        project_arn: &str,
        device_pool_arn: &str,
        upload_arn: &str,
    ) -> TestResultType<TestRunId> {
        info!("Scheduling AWS Device Farm run");

        let response = self.client
            .schedule_run()
            .project_arn(project_arn)
            .device_pool_arn(device_pool_arn)
            .name("Test Run")
            .test(
                aws_sdk_devicefarm::types::Test::builder()
                    .test_type(aws_sdk_devicefarm::types::TestType::AppiumNodeJs)
                    .build()
            )
            .send()
            .await
            .map_err(|e| utm_testing_core::TestError::DeviceFarmError(
                format!("Failed to schedule run: {}", e)
            ))?;

        let run_id = TestRunId::new();
        Ok(run_id)
    }
}

#[async_trait::async_trait]
impl DeviceFarmProvider for AWSDeviceFarm {
    fn name(&self) -> &str {
        "AWS Device Farm"
    }

    async fn upload_app(
        &self,
        app_path: &str,
        app_type: &str,
    ) -> TestResultType<String> {
        unimplemented!()
    }

    async fn schedule_run(
        &self,
        upload_id: &str,
        device_pool: &DevicePool,
    ) -> TestResultType<TestRunId> {
        unimplemented!()
    }

    async fn get_results(&self, run_id: &TestRunId) -> TestResultType<TestRunResults> {
        unimplemented!()
    }

    async fn wait_for_completion(
        &self,
        run_id: &TestRunId,
        timeout_secs: u64,
    ) -> TestResultType<TestRunResults> {
        unimplemented!()
    }
}
```

## Test Reporting (utm-testing-reporter)

```rust
// utm-testing-reporter/src/lib.rs
mod junit;
mod html;
mod ci;

pub use junit::JUnitReporter;
pub use html::HtmlReporter;
pub use ci::CIReporter;

use utm_testing_core::TestRunResults;
use std::path::Path;

/// Test reporter trait
pub trait TestReporter: Send + Sync {
    /// Generate report from test results
    fn generate_report(&self, results: &TestRunResults, output_path: &Path) -> std::io::Result<()>;

    /// Get report format name
    fn format_name(&self) -> &str;
}
```

```rust
// utm-testing-reporter/src/junit.rs
use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use utm_testing_core::{TestRunResults, TestStatus};
use crate::TestReporter;

/// JUnit XML test reporter
pub struct JUnitReporter;

impl JUnitReporter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JUnitReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl TestReporter for JUnitReporter {
    fn generate_report(&self, results: &TestRunResults, output_path: &Path) -> std::io::Result<()> {
        let mut writer = Writer::new_with_indent(File::create(output_path)?, b' ', 2);

        // testsuites root element
        let mut testsuites = BytesStart::new("testsuites");
        testsuites.push_attribute(("name", results.suite_name.as_str()));
        testsuites.push_attribute(("tests", results.summary.total.to_string().as_str()));
        testsuites.push_attribute(("failures", results.summary.failed.to_string().as_str()));
        testsuites.push_attribute(("errors", results.summary.errors.to_string().as_str()));
        testsuites.push_attribute(("time", (results.summary.total_duration_ms as f64 / 1000.0).to_string().as_str()));

        writer.write_event(Event::Start(testsuites))?;

        // testsuite element
        let mut testsuite = BytesStart::new("testsuite");
        testsuite.push_attribute(("name", results.suite_name.as_str()));
        testsuite.push_attribute(("tests", results.tests.len().to_string().as_str()));
        testsuite.push_attribute(("failures", results.summary.failed.to_string().as_str()));
        testsuite.push_attribute(("errors", results.summary.errors.to_string().as_str()));
        testsuite.push_attribute(("skipped", results.summary.skipped.to_string().as_str()));

        writer.write_event(Event::Start(testsuite))?;

        // Write individual test cases
        for test in &results.tests {
            let mut testcase = BytesStart::new("testcase");
            testcase.push_attribute(("name", test.name.as_str()));
            testcase.push_attribute(("classname", test.suite.as_str()));
            testcase.push_attribute(("time", (test.duration_ms as f64 / 1000.0).to_string().as_str()));

            writer.write_event(Event::Start(testcase.clone()))?;

            // Add failure element if test failed
            if test.status == TestStatus::Failed {
                let mut failure = BytesStart::new("failure");
                if let Some(ref error) = test.error {
                    failure.push_attribute(("message", error.as_str()));
                }
                writer.write_event(Event::Start(failure.clone()))?;
                if let Some(ref error) = test.error {
                    writer.write_event(Event::Text(BytesText::new(error)))?;
                }
                writer.write_event(Event::End(failure.name()))?;
            }

            writer.write_event(Event::End(testcase.name()))?;
        }

        writer.write_event(Event::End(testsuite.name()))?;
        writer.write_event(Event::End(testsuites.name()))?;

        Ok(())
    }

    fn format_name(&self) -> &str {
        "junit"
    }
}
```

## Key Rust-Specific Changes

### 1. Async-First Test Execution

All test runners use async/await for non-blocking execution:

```rust
#[async_trait::async_trait]
impl TestRunner for XCUITestRunner {
    async fn run_all(&self, config: &TestConfig) -> TestResultType<TestRunResults> {
        // Non-blocking process execution
        let output = cmd.output().await?;
        // ...
    }
}
```

### 2. Parallel Test Execution

Using tokio for parallel test runs:

```rust
use tokio::task::JoinSet;

async fn run_tests_parallel(
    runner: &dyn TestRunner,
    tests: &[String],
) -> Vec<TestResultType<utm_testing_core::TestResult>> {
    let mut set = JoinSet::new();

    for test_name in tests {
        let runner = Arc::clone(&runner);
        let name = test_name.clone();
        set.spawn(async move {
            runner.run_test(&name, &config).await
        });
    }

    let mut results = Vec::new();
    while let Some(result) = set.join_next().await {
        results.push(result.unwrap());
    }

    results
}
```

### 3. Type-Safe Test Results

Comprehensive type system for test results:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub id: String,
    pub name: String,
    pub suite: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub error: Option<String>,
    // ...
}
```

### 4. Zero-Copy String Handling

Using Cow for efficient string handling:

```rust
use std::borrow::Cow;

pub struct TestName<'a> {
    pub suite: Cow<'a, str>,
    pub test: Cow<'a, str>,
}
```

## Concurrency Model

### Tokio Async Runtime

All I/O and process execution uses tokio:

```rust
use tokio::process::Command;
use tokio::time::{timeout, Duration};

async fn run_with_timeout(
    cmd: &mut Command,
    timeout_secs: u64,
) -> TestResultType<std::process::Output> {
    timeout(Duration::from_secs(timeout_secs), cmd.output())
        .await
        .map_err(|_| TestError::Timeout("...".to_string()))?
}
```

### Thread-Safe Traits

All traits require `Send + Sync`:

```rust
#[async_trait::async_trait]
pub trait TestRunner: Send + Sync {
    // ...
}
```

## Code Examples

### Full Test Run Workflow

```rust
use utm_testing_core::{TestRunner, TestConfig, TestPlatform};
use utm_testing_integration::{XCUITestRunner, EspressoRunner};
use utm_testing_e2e::{MaestroRunner, E2ETestRunner};
use utm_testing_reporter::{JUnitReporter, TestReporter};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Run iOS integration tests
    let ios_runner = XCUITestRunner::new("IntegrationTests")
        .with_workspace("ios/MyApp.xcworkspace");

    let ios_config = TestConfig::new(TestPlatform::Ios, "ios/Tests")
        .with_timeout(600)
        .verbose();

    let ios_results = ios_runner.run_all(&ios_config).await?;
    println!("iOS tests: {} passed, {} failed",
        ios_results.summary.passed,
        ios_results.summary.failed
    );

    // Run Android integration tests
    let android_runner = EspressoRunner::new("android")
        .with_variant("debug");

    let android_config = TestConfig::new(TestPlatform::Android, "android")
        .with_timeout(900);

    let android_results = android_runner.run_all(&android_config).await?;

    // Run E2E Maestro flow
    let maestro_runner = MaestroRunner::new();
    let e2e_result = maestro_runner.run_maestro(
        "e2e/flows/login.yaml",
        "com.example.app"
    ).await?;

    // Generate JUnit report
    let reporter = JUnitReporter::new();
    reporter.generate_report(&ios_results, Path::new("reports/ios-results.xml"))?;
    reporter.generate_report(&android_results, Path::new("reports/android-results.xml"))?;

    Ok(())
}
```

### Screenshot Testing

```rust
use utm_testing_screenshots::{ScreenshotConfig, ScreenshotComparator};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ScreenshotConfig::new(Path::new("screenshots"));
    let comparator = ScreenshotComparator::new(0.99);

    // Compare screenshots
    let comparison = comparator.compare(
        Path::new("screenshots/baseline/home.png"),
        Path::new("screenshots/output/home.png")
    )?;

    if !comparison.matches {
        println!("Screenshot mismatch detected!");
        println!("Similarity: {:.2}%", comparison.similarity * 100.0);

        // Generate diff image
        comparator.generate_diff(
            Path::new("screenshots/baseline/home.png"),
            Path::new("screenshots/output/home.png"),
            Path::new("screenshots/diff/home-diff.png")
        )?;
    }

    Ok(())
}
```

## Migration Path

### Phase 1: Core Framework (Week 1-2)
- Implement `utm-testing-core` crate
- Define traits, types, and error handling
- Set up workspace structure

### Phase 2: Integration Test Runners (Week 3-4)
- XCUITest runner for iOS
- Espresso runner for Android
- Device management utilities

### Phase 3: E2E Orchestration (Week 5)
- Maestro flow runner
- Detox integration
- Scenario definitions

### Phase 4: Device Farm Integration (Week 6-7)
- Firebase Test Lab client
- AWS Device Farm client
- Result aggregation

### Phase 5: Reporting & CLI (Week 8)
- JUnit XML reporter
- HTML reporter
- CI-specific formatters
- CLI tool

## Testing Strategy

### Unit Tests for Test Framework

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use utm_testing_core::{TestStatus, TestPlatform};

    #[test]
    fn test_summary_calculation() {
        let results = vec![
            TestResult { status: TestStatus::Passed, ..default_result() },
            TestResult { status: TestStatus::Passed, ..default_result() },
            TestResult { status: TestStatus::Failed, ..default_result() },
        ];

        let summary = TestSummary::from_results(&results);

        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 1);
        assert!((summary.pass_rate - 66.67).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_xcuitest_runner_creation() {
        let runner = XCUITestRunner::new("TestScheme");
        assert_eq!(runner.platform(), TestPlatform::Ios);
    }
}
```

## Open Considerations

1. **Visual Regression Testing**: Integrate with pixelmatch or similar for advanced image comparison

2. **Test Flakiness Detection**: Track and report flaky tests across runs

3. **Remote Device Integration**: Support for BrowserStack, Sauce Labs

4. **Performance Testing**: Add benchmarking and performance regression detection

5. **Coverage Reporting**: Integrate with cargo-llvm-cov for coverage reports

6. **Test Parallelization Across CI**: Coordinate test execution across multiple CI runners

7. **Maestro Flow Generator**: Tool to generate Maestro flows from recorded user actions
