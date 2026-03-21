---
source: webview-production-exploration.md
repository: N/A
revised_at: 2026-03-21T12:00:00Z
workspace: utm-webview-production
---

# Rust Revision: WebView Production

## Overview

This document provides a comprehensive Rust implementation for production-ready WebView features in utm-dev. The translation focuses on:

- **Service worker bundling** with esbuild integration for optimized builds
- **Asset manifest generation** with content hashing for cache busting
- **Deep link configuration** for cross-platform URL handling
- **PWA manifest builder** for installable web applications
- **Offline-first patterns** with IndexedDB integration
- **Build-time optimizations** for HTMX/Datastar applications

The implementation uses async-first design with tokio, content-addressable asset management with SHA256 hashing, and provides a complete build pipeline for production WebView applications.

## Workspace Structure

```
utm-webview-production/
├── Cargo.toml                      # Workspace manifest
├── utm-webview-core/               # Core types and utilities
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── types.rs                # Asset types, manifest types
│       ├── error.rs                # Error definitions
│       └── utils.rs                # Hash utilities
├── utm-webview-bundler/            # Service worker bundling
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── sw_bundler.rs           # ServiceWorkerBundler
│       ├── js_minifier.rs          # JavaScript minification
│       └── precache.rs             # Precache manifest generation
├── utm-webview-manifest/           # Asset manifest generation
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── generator.rs            # ManifestGenerator
│       ├── asset.rs                # AssetEntry, AssetType
│       └── integrity.rs            # SRI hash computation
├── utm-webview-deeplink/           # Deep link handling
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── config.rs               # DeepLinkConfig
│       ├── handler.rs              # DeepLinkHandler
│       └── routes.rs               # Route matching
├── utm-webview-pwa/                # PWA manifest builder
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── builder.rs              # WebAppManifestBuilder
│       ├── icons.rs                # Icon generation
│       └── shortcuts.rs            # Shortcut items
├── utm-webview-offline/            # Offline-first support
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── indexed_db.rs           # IndexedDB wrapper
│       ├── sync_queue.rs           # Offline sync queue
│       └── cache_strategy.rs       # Cache strategies
└── utm-webview-cli/                # CLI binary
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── commands/
        │   ├── bundle.rs
        │   ├── manifest.rs
        │   └── build.rs
        └── args.rs
```

### Crate Breakdown

#### utm-webview-core
- **Purpose:** Shared types, utilities, and error definitions
- **Type:** library
- **Public API:** `AssetType`, `AssetEntry`, `WebviewError`, `compute_hash`
- **Dependencies:** serde, thiserror, sha2

#### utm-webview-bundler
- **Purpose:** Service worker bundling and optimization
- **Type:** library
- **Public API:** `ServiceWorkerBundler`, `BundleOptions`, `PrecacheManifest`
- **Dependencies:** esbuild-rust, sha2, tokio

#### utm-webview-manifest
- **Purpose:** Asset manifest generation with content hashing
- **Type:** library
- **Public API:** `ManifestGenerator`, `AssetManifest`, `AssetEntry`
- **Dependencies:** serde, serde_json, sha2, base64, walkdir

#### utm-webview-deeplink
- **Purpose:** Deep link configuration and handling
- **Type:** library
- **Public API:** `DeepLinkConfig`, `DeepLinkHandler`, `DeepLinkRoute`
- **Dependencies:** url, regex, serde

#### utm-webview-pwa
- **Purpose:** PWA manifest generation
- **Type:** library
- **Public API:** `WebAppManifestBuilder`, `PwaManifest`, `ShortcutItem`
- **Dependencies:** serde, serde_json, image

#### utm-webview-offline
- **Purpose:** Offline-first patterns and IndexedDB integration
- **Type:** library
- **Public API:** `OfflineStore`, `SyncQueue`, `CacheStrategy`
- **Dependencies:** serde, serde_json, tokio

#### utm-webview-cli
- **Purpose:** Command-line interface for WebView build tools
- **Type:** binary
- **Public API:** CLI commands (bundle, manifest, build)
- **Dependencies:** clap, tokio, tracing

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Async runtime | tokio | 1.0 | Full-featured async runtime |
| Serialization | serde + serde_json | 1.0 | Industry-standard serialization |
| Error handling | thiserror | 1.0 | Derive-based error types |
| Hashing | sha2 | 0.10 | SHA256 for content addressing |
| Base64 encoding | base64 | 0.21 | For SRI integrity hashes |
| File walking | walkdir | 2.4 | Recursive directory traversal |
| URL parsing | url | 2.5 | RFC-compliant URL handling |
| Regex | regex | 1.10 | Route pattern matching |
| JS bundling | esbuild-rust | 0.2 | Fast JavaScript bundling |
| Image processing | image | 0.24 | For PWA icon generation |
| CLI parsing | clap | 4.0 | Derive-based CLI |
| Logging | tracing | 0.1 | Structured logging |
| HTTP types | http | 1.0 | HTTP request/response types |

## Type System Design

### Core Types

```rust
// utm-webview-core/src/types.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Asset entry in manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    /// Original file path
    pub original: String,

    /// Hashed output path
    pub output: String,

    /// Content hash (for cache busting)
    pub hash: String,

    /// File size in bytes
    pub size: u64,

    /// Gzipped size
    pub gzip_size: Option<u64>,

    /// Asset type
    pub asset_type: AssetType,

    /// Integrity hash for SRI
    pub integrity: String,

    /// Dependencies
    pub dependencies: Vec<String>,
}

/// Asset type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    JavaScript,
    StyleSheet,
    Image,
    Font,
    Html,
    Json,
    WebAssembly,
    ServiceWorker,
    Other,
}

impl AssetType {
    /// Determine asset type from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "js" | "mjs" | "cjs" => AssetType::JavaScript,
            "css" | "scss" | "sass" | "less" => AssetType::StyleSheet,
            "html" | "htm" => AssetType::Html,
            "json" | "json5" => AssetType::Json,
            "wasm" => AssetType::WebAssembly,
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" => AssetType::Image,
            "woff" | "woff2" | "ttf" | "eot" => AssetType::Font,
            "sw.js" | "service-worker.js" => AssetType::ServiceWorker,
            _ => AssetType::Other,
        }
    }

    /// Get cache strategy for asset type
    pub fn cache_strategy(&self) -> CacheStrategy {
        match self {
            AssetType::JavaScript | AssetType::StyleSheet => CacheStrategy::CacheFirst,
            AssetType::Html => CacheStrategy::NetworkFirst,
            AssetType::Image => CacheStrategy::CacheFirst,
            AssetType::Font => CacheStrategy::CacheFirst,
            AssetType::ServiceWorker => CacheStrategy::NetworkFirst,
            _ => CacheStrategy::StaleWhileRevalidate,
        }
    }
}

/// Caching strategy for service worker
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheStrategy {
    /// Cache first, fallback to network
    CacheFirst,
    /// Network first, fallback to cache
    NetworkFirst,
    /// Serve cache, update in background
    StaleWhileRevalidate,
    /// Network only
    NetworkOnly,
    /// Cache only
    CacheOnly,
}

/// Service worker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceWorkerConfig {
    /// Cache version
    pub cache_version: String,

    /// Static cache name
    pub static_cache: String,

    /// Dynamic cache name
    pub dynamic_cache: String,

    /// Build cache name
    pub build_cache: String,

    /// Assets to precache
    pub precache_assets: Vec<String>,

    /// URL patterns to cache
    pub cache_patterns: Vec<String>,

    /// Default strategy
    pub default_strategy: CacheStrategy,
}

impl Default for ServiceWorkerConfig {
    fn default() -> Self {
        Self {
            cache_version: "v1.0.0".to_string(),
            static_cache: "utm-static-v1.0.0".to_string(),
            dynamic_cache: "utm-dynamic-v1.0.0".to_string(),
            build_cache: "utm-build-v1.0.0".to_string(),
            precache_assets: vec![],
            cache_patterns: vec![],
            default_strategy: CacheStrategy::CacheFirst,
        }
    }
}

/// Bundle options for service worker
#[derive(Debug, Clone)]
pub struct BundleOptions {
    /// Minify output
    pub minify: bool,

    /// Generate sourcemap
    pub sourcemap: bool,

    /// Target ES version
    pub target: String,

    /// Define global constants
    pub define: std::collections::HashMap<String, String>,
}

impl Default for BundleOptions {
    fn default() -> Self {
        let mut define = std::collections::HashMap::new();
        define.insert("__SW_VERSION__".to_string(), "\"1.0.0\"".to_string());

        Self {
            minify: true,
            sourcemap: false,
            target: "es2020".to_string(),
            define,
        }
    }
}
```

### Error Types

```rust
// utm-webview-core/src/error.rs

use thiserror::Error;
use std::path::PathBuf;

/// Main error type for webview operations
#[derive(Debug, Error)]
pub enum WebviewError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Bundle error: {0}")]
    BundleError(String),

    #[error("Manifest error: {0}")]
    ManifestError(String),

    #[error("Deep link error: {0}")]
    DeepLinkError(#[from] DeepLinkError),

    #[error("PWA error: {0}")]
    PwaError(String),

    #[error("Asset not found: {0}")]
    AssetNotFound(PathBuf),

    #[error("Hash computation failed: {0}")]
    HashError(String),

    #[error("Invalid asset type: {0}")]
    InvalidAssetType(String),
}

/// Error type for deep link handling
#[derive(Debug, Error)]
pub enum DeepLinkError {
    #[error("Invalid scheme: expected {expected}, got {actual}")]
    InvalidScheme { expected: String, actual: String },

    #[error("No matching route for: {0}")]
    NoMatchingRoute(String),

    #[error("URL parse error: {0}")]
    ParseError(#[from] url::ParseError),

    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Platform error: {0}")]
    PlatformError(String),

    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),
}

/// Error type for manifest generation
#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Asset not found: {0}")]
    AssetNotFound(PathBuf),

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Hash error: {0}")]
    HashError(String),
}

pub type Result<T> = std::result::Result<T, WebviewError>;
pub type ManifestResult<T> = std::result::Result<T, ManifestError>;
pub type DeepLinkResult<T> = std::result::Result<T, DeepLinkError>;
```

### Traits

```rust
// utm-webview-core/src/traits.rs

use crate::{AssetEntry, AssetType, CacheStrategy};

/// Trait for asset processors
pub trait AssetProcessor: Send + Sync {
    /// Check if processor handles this asset type
    fn supports(&self, asset_type: &AssetType) -> bool;

    /// Process an asset
    fn process(&self, content: &[u8]) -> crate::Result<ProcessedAsset>;
}

/// Processed asset result
pub struct ProcessedAsset {
    pub content: Vec<u8>,
    pub hash: String,
    pub size: u64,
    pub transformations: Vec<String>,
}

/// Trait for cache strategy providers
pub trait CacheStrategyProvider: Send + Sync {
    /// Get cache strategy for a URL
    fn get_strategy(&self, url: &str) -> CacheStrategy;

    /// Get cache name for a URL
    fn get_cache_name(&self, url: &str) -> &str;
}

/// Trait for service worker generators
pub trait ServiceWorkerGenerator: Send + Sync {
    /// Generate service worker code
    fn generate(&self, config: &ServiceWorkerConfig) -> crate::Result<String>;

    /// Get service worker filename
    fn filename(&self) -> &str {
        "sw.js"
    }
}

/// Trait for deep link handlers
pub trait DeepLinkHandler: Send + Sync {
    /// Check if handler can handle this URL
    fn can_handle(&self, url: &str) -> bool;

    /// Handle the deep link
    fn handle(&self, url: &str) -> crate::DeepLinkResult<DeepLinkResult>;
}

/// Result of deep link handling
pub struct DeepLinkResult {
    pub handler: String,
    pub params: std::collections::HashMap<String, String>,
    pub action: DeepLinkAction,
}

/// Action to take after deep link handling
pub enum DeepLinkAction {
    Navigate(String),
    Dispatch(String),
    Ignore,
}
```

## Key Rust-Specific Changes

### 1. Content-Addressable Asset Management

**Source Pattern:** Filename-based asset tracking

**Rust Translation:** SHA256 content hashing for cache-busting filenames

**Rationale:** Ensures cache invalidation when content changes, enables long-term caching.

```rust
// utm-webview-manifest/src/generator.rs

use sha2::{Sha256, Digest};

pub fn compute_content_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())[..8].to_string()
}

pub fn generate_hashed_name(path: &Path, hash: &str) -> String {
    let stem = path.file_stem().unwrap().to_string_lossy();
    let ext = path.extension().unwrap().to_string_lossy();
    format!("{}.{}.{}", stem, hash, ext)
}
```

### 2. Service Worker with Multiple Cache Strategies

**Source Pattern:** Single caching strategy

**Rust Translation:** Strategy enum with per-asset-type configuration

**Rationale:** Different assets need different caching strategies for optimal performance.

```rust
// utm-webview-bundler/src/sw_bundler.rs

pub enum CacheStrategy {
    CacheFirst,
    NetworkFirst,
    StaleWhileRevalidate,
    NetworkOnly,
    CacheOnly,
}

impl CacheStrategy {
    pub fn to_service_worker_code(&self) -> &'static str {
        match self {
            CacheStrategy::CacheFirst => "cacheFirst",
            CacheStrategy::NetworkFirst => "networkFirst",
            CacheStrategy::StaleWhileRevalidate => "staleWhileRevalidate",
            CacheStrategy::NetworkOnly => "networkOnly",
            CacheStrategy::CacheOnly => "cacheOnly",
        }
    }
}
```

### 3. Deep Link Route Matching with Regex

**Source Pattern:** String-based route matching

**Rust Translation:** Compiled regex patterns for efficient route matching

**Rationale:** Provides flexible route patterns with parameter extraction.

```rust
// utm-webview-deeplink/src/routes.rs

use regex::Regex;

pub struct DeepLinkRoute {
    pattern: String,
    regex: Regex,
    param_names: Vec<String>,
}

impl DeepLinkRoute {
    pub fn new(pattern: &str) -> Result<Self, DeepLinkError> {
        // Convert :param to named capture groups
        let regex_pattern = pattern
            .replace(":", "(?P<")
            .replace("/", "\\/")
            .replace("}", ">[^/]+)");

        let regex = Regex::new(&format!("^{}$", regex_pattern))
            .map_err(|e| DeepLinkError::InvalidPattern(e.to_string()))?;

        let param_names = regex.capture_names()
            .flatten()
            .map(String::from)
            .collect();

        Ok(Self {
            pattern: pattern.to_string(),
            regex,
            param_names,
        })
    }
}
```

### 4. PWA Manifest with Builder Pattern

**Source Pattern:** Static JSON configuration

**Rust Translation:** Builder pattern for flexible manifest construction

**Rationale:** Provides type-safe, ergonomic API for manifest generation.

```rust
// utm-webview-pwa/src/builder.rs

pub struct WebAppManifestBuilder {
    name: Option<String>,
    short_name: Option<String>,
    start_url: Option<String>,
    display: Option<String>,
    icons: Vec<IconInfo>,
    shortcuts: Vec<ShortcutItem>,
}

impl WebAppManifestBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn icon(mut self, src: &str, sizes: &[u32], format: IconFormat) -> Self {
        self.icons.push(IconInfo {
            src: src.to_string(),
            sizes: sizes.iter().map(|s| format!("{}x{}", s, s)).collect(),
            format,
        });
        self
    }

    pub fn build(self) -> WebAppManifest {
        WebAppManifest {
            name: self.name.unwrap_or_default(),
            short_name: self.short_name.unwrap_or_default(),
            start_url: self.start_url.unwrap_or_else(|| "/".to_string()),
            display: self.display.unwrap_or_else(|| "standalone".to_string()),
            icons: self.icons,
            shortcuts: self.shortcuts,
        }
    }
}
```

## Ownership & Borrowing Strategy

1. **Asset manifests use String** - Owned for serialization
2. **Bundle options use Clone** - Passed to multiple workers
3. **Service worker config uses Arc** - Shared across build tasks
4. **Deep link routes use Cow** - Efficient string handling
5. **Errors use thiserror** - Clear error propagation

```rust
// Example ownership flow

pub struct ManifestGenerator {
    output_dir: PathBuf,
    public_path: String,
}

pub fn generate(&self) -> ManifestResult<AssetManifest> {
    let mut manifest = AssetManifest::new(env!("CARGO_PKG_VERSION"));
    self.collect_assets(&mut manifest)?;  // Borrow mutable manifest
    manifest.build_hash = self.compute_build_hash(&manifest)?;  // Immutable borrow
    Ok(manifest)  // Return by value
}
```

## Concurrency Model

**Approach:** Async with tokio runtime + parallel asset processing

**Rationale:**
- Async for I/O-bound operations (file reading, network)
- Parallel hashing for multiple assets
- Service worker generation is single-threaded (deterministic)

```rust
// Parallel asset processing

use tokio::task::JoinSet;

pub async fn process_assets(
    &self,
    assets: Vec<AssetEntry>,
) -> Result<Vec<ProcessedAsset>> {
    let mut join_set = JoinSet::new();

    for asset in assets {
        let processor = self.processor.clone();
        join_set.spawn(async move {
            processor.process(&asset).await
        });
    }

    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result??);
    }

    Ok(results)
}
```

## Memory Considerations

1. **Large files streamed** - Not loaded entirely into memory
2. **Asset hashes computed in chunks** - For very large files
3. **Service worker code cached** - Avoid regeneration
4. **Manifest uses HashMap** - O(1) lookups
5. **Lazy evaluation** - Only process changed assets

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Missing asset file | Error returned, not panic |
| Invalid UTF-8 in asset | Handled as bytes, lossy conversion |
| Service worker registration fails | Graceful degradation |
| Deep link with no matching route | Returns error, app continues |
| Manifest generation during build | Atomic writes with temp files |
| Concurrent cache writes | Mutex-protected cache state |
| PWA icon generation failure | Falls back to default icons |

## Code Examples

### Example: Service Worker Bundler

```rust
// utm-webview-bundler/src/sw_bundler.rs

use std::path::{Path, PathBuf};
use esbuild_rust::{build, BuildOptions};
use crate::{BundleOptions, ServiceWorkerConfig, BundleError};

/// Service worker bundler with esbuild integration
pub struct ServiceWorkerBundler {
    entry_point: PathBuf,
    output_dir: PathBuf,
    options: BundleOptions,
}

impl ServiceWorkerBundler {
    pub fn new(entry_point: PathBuf, output_dir: PathBuf) -> Self {
        Self {
            entry_point,
            output_dir,
            options: BundleOptions::default(),
        }
    }

    pub fn with_options(mut self, options: BundleOptions) -> Self {
        self.options = options;
        self
    }

    /// Bundle service worker
    pub fn bundle(&self) -> Result<PathBuf, BundleError> {
        let result = build(BuildOptions {
            entry_points: vec![self.entry_point.to_string_lossy().to_string()],
            outdir: self.output_dir.to_string_lossy().to_string(),
            bundle: true,
            minify: self.options.minify,
            sourcemap: if self.options.sourcemap { "inline" } else { "" },
            target: &self.options.target,
            format: "iife",
            // Don't use eval for CSP compliance
            avoid_eval: true,
            // Tree shaking
            tree_shaking: true,
            // Define global constants
            define: self.options.define.iter()
                .map(|(k, v)| (k.clone(), format!("\"{}\"", v)))
                .collect(),
            ..Default::default()
        })?;

        Ok(self.output_dir.join("sw.js"))
    }

    /// Generate service worker with precache manifest
    pub fn bundle_with_precache(
        &self,
        assets: &[PathBuf],
        config: &ServiceWorkerConfig,
    ) -> Result<PathBuf, BundleError> {
        // Generate precache manifest
        let mut precache_list = String::from("[\n");

        for asset in assets {
            if let Ok(content) = std::fs::read(asset) {
                let hash = compute_hash(&content);
                let url = asset.strip_prefix(&self.output_dir)
                    .unwrap_or(asset)
                    .to_string_lossy();

                precache_list.push_str(&format!(
                    "  {{ url: '{}', revision: '{}' }},\n",
                    url, hash
                ));
            }
        }

        precache_list.push_str("]");

        // Generate service worker code
        let sw_code = self.generate_service_worker(config, &precache_list);

        // Write bundled version
        let output_path = self.output_dir.join("sw.js");
        std::fs::write(&output_path, sw_code)?;

        Ok(output_path)
    }

    fn generate_service_worker(
        &self,
        config: &ServiceWorkerConfig,
        precache_manifest: &str,
    ) -> String {
        format!(
            r#"// Auto-generated service worker
// Version: {version}

const CACHE_VERSION = '{version}';
const STATIC_CACHE = '{static_cache}';
const DYNAMIC_CACHE = '{dynamic_cache}';
const BUILD_CACHE = '{build_cache}';

// Precache manifest
const PRECACHE_MANIFEST = {precache};

// Install event
self.addEventListener('install', (event) => {{
  event.waitUntil(
    caches.open(STATIC_CACHE).then((cache) => {{
      return cache.addAll(PRECACHE_MANIFEST.map(item => item.url));
    }})
  );
  self.skipWaiting();
}});

// Activate event
self.addEventListener('activate', (event) => {{
  event.waitUntil(
    caches.keys().then((keys) => {{
      return Promise.all(
        keys
          .filter((key) => !key.includes(CACHE_VERSION))
          .map((key) => caches.delete(key))
      );
    }})
  );
  self.clients.claim();
}});

// Fetch event
self.addEventListener('fetch', (event) => {{
  const {{ request }} = event;
  const url = new URL(request.url);

  if (url.origin !== location.origin) {{
    return;
  }}

  const strategy = getStrategy(url.pathname);

  switch (strategy) {{
    case 'cache-first':
      event.respondWith(cacheFirst(request));
      break;
    case 'network-first':
      event.respondWith(networkFirst(request));
      break;
    case 'stale-while-revalidate':
      event.respondWith(staleWhileRevalidate(request));
      break;
    default:
      event.respondWith(fetch(request));
  }}
}});

function getStrategy(pathname) {{
  {strategies}
  return 'cache-first';
}}

async function cacheFirst(request) {{
  const cached = await caches.match(request);
  if (cached) return cached;

  try {{
    const response = await fetch(request);
    if (response.ok) {{
      const cache = await caches.open(STATIC_CACHE);
      cache.put(request, response.clone());
    }}
    return response;
  }} catch (error) {{
    return createOfflineResponse();
  }}
}}

async function networkFirst(request) {{
  try {{
    const response = await fetch(request);
    if (response.ok) {{
      const cache = await caches.open(DYNAMIC_CACHE);
      cache.put(request, response.clone());
    }}
    return response;
  }} catch (error) {{
    const cached = await caches.match(request);
    if (cached) return cached;
    return createOfflineResponse();
  }}
}}

async function staleWhileRevalidate(request) {{
  const cache = await caches.open(DYNAMIC_CACHE);
  const cached = await cache.match(request);

  const fetchPromise = fetch(request).then((response) => {{
    if (response.ok) {{
      cache.put(request, response.clone());
    }}
    return response;
  }}).catch(() => null);

  return cached || fetchPromise || createOfflineResponse();
}}

function createOfflineResponse() {{
  return new Response(`
    <!DOCTYPE html>
    <html>
      <head><title>Offline</title></head>
      <body><h1>You're Offline</h1></body>
    </html>
  `, {{
    headers: {{ 'Content-Type': 'text/html' }},
    status: 503,
  }});
}}
"#,
            version = config.cache_version,
            static_cache = config.static_cache,
            dynamic_cache = config.dynamic_cache,
            build_cache = config.build_cache,
            precache = precache_manifest,
            strategies = self.generate_strategy_code(&config.cache_patterns),
        )
    }

    fn generate_strategy_code(&self, patterns: &[String]) -> String {
        patterns.iter()
            .map(|p| format!("if (/{}$/.test(pathname)) return 'cache-first';", p))
            .collect::<Vec<_>>()
            .join("\n  ")
    }
}

fn compute_hash(content: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(content);
    format!("{:x}", hash)[..8].to_string()
}
```

### Example: Asset Manifest Generator

```rust
// utm-webview-manifest/src/generator.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use walkdir::WalkDir;
use crate::{AssetManifest, AssetEntry, AssetType, ManifestError, ManifestResult};

/// Asset manifest generator
pub struct ManifestGenerator {
    output_dir: PathBuf,
    public_path: String,
}

impl ManifestGenerator {
    pub fn new(output_dir: PathBuf, public_path: &str) -> Self {
        Self {
            output_dir,
            public_path: public_path.to_string(),
        }
    }

    /// Generate asset manifest
    pub fn generate(&self) -> ManifestResult<AssetManifest> {
        let mut manifest = AssetManifest::new(env!("CARGO_PKG_VERSION"));

        // Collect all assets
        self.collect_assets(&mut manifest)?;

        // Generate build hash
        manifest.build_hash = self.compute_build_hash(&manifest)?;

        Ok(manifest)
    }

    fn collect_assets(&self, manifest: &mut AssetManifest) -> ManifestResult<()> {
        for entry in WalkDir::new(&self.output_dir) {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => return Err(ManifestError::IoError(e.into_io().unwrap())),
            };

            if entry.file_type().is_file() {
                let path = entry.path();
                let relative = path.strip_prefix(&self.output_dir)
                    .map_err(|_| ManifestError::InvalidPath(path.to_path_buf()))?;

                // Determine asset type
                let asset_type = AssetType::from_extension(
                    path.extension().and_then(|e| e.to_str()).unwrap_or("")
                );

                // Compute hashes
                let content = std::fs::read(path)
                    .map_err(|_| ManifestError::AssetNotFound(path.to_path_buf()))?;
                let hash = self.compute_content_hash(&content);
                let integrity = self.compute_integrity(&content);

                // Generate hashed filename
                let hashed_name = self.generate_hashed_name(path, &hash);

                let entry = AssetEntry {
                    original: relative.to_string_lossy().to_string(),
                    output: format!("{}/{}", self.public_path.trim_end_matches('/'), hashed_name),
                    hash: hash.clone(),
                    size: content.len() as u64,
                    gzip_size: None,
                    asset_type,
                    integrity,
                    dependencies: Vec::new(),
                };

                manifest.add_asset(entry);
            }
        }

        Ok(())
    }

    fn compute_content_hash(&self, content: &[u8]) -> String {
        let hash = Sha256::digest(content);
        format!("{:x}", hash)[..8].to_string()
    }

    fn compute_integrity(&self, content: &[u8]) -> String {
        let hash = Sha256::digest(content);
        let encoded = base64::encode(hash);
        format!("sha256-{}", encoded)
    }

    fn generate_hashed_name(&self, path: &Path, hash: &str) -> String {
        let stem = path.file_stem().unwrap().to_string_lossy();
        let ext = path.extension().unwrap().to_string_lossy();
        format!("{}.{}.{}", stem, hash, ext)
    }

    fn compute_build_hash(&self, manifest: &AssetManifest) -> ManifestResult<String> {
        let mut hasher = Sha256::new();

        // Hash all asset hashes in sorted order
        let mut hashes: Vec<_> = manifest.assets.values()
            .map(|e| &e.hash)
            .collect();
        hashes.sort();

        for hash in hashes {
            hasher.update(hash.as_bytes());
        }

        Ok(format!("{:x}", hasher.finalize())[..16].to_string())
    }
}

/// Asset manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    pub version: String,
    pub generated_at: String,
    pub build_hash: String,
    pub assets: HashMap<String, AssetEntry>,
    pub entry_points: HashMap<String, EntryPoint>,
}

impl AssetManifest {
    pub fn new(version: &str) -> Self {
        Self {
            version: version.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            build_hash: String::new(),
            assets: HashMap::new(),
            entry_points: HashMap::new(),
        }
    }

    pub fn add_asset(&mut self, entry: AssetEntry) {
        self.assets.insert(entry.original.clone(), entry);
    }

    pub fn get_hashed_path(&self, original: &str) -> Option<&str> {
        self.assets.get(original).map(|e| e.output.as_str())
    }

    pub fn get_integrity(&self, original: &str) -> Option<&str> {
        self.assets.get(original).map(|e| e.integrity.as_str())
    }

    /// Generate HTML for entry point
    pub fn generate_html(&self, entry_name: &str) -> String {
        let mut html = String::new();

        if let Some(entry) = self.entry_points.get(entry_name) {
            // CSS
            for css in &entry.css {
                if let Some(asset) = self.assets.get(css) {
                    html.push_str(&format!(
                        "<link rel=\"stylesheet\" href=\"{}\" integrity=\"{}\" crossorigin=\"anonymous\">\n",
                        asset.output, asset.integrity
                    ));
                }
            }

            // JavaScript
            html.push_str(&format!(
                "<script src=\"{}\" integrity=\"{}\" crossorigin=\"anonymous\" defer></script>\n",
                self.assets.get(&entry.file).map(|a| a.output.as_str()).unwrap_or(""),
                self.assets.get(&entry.file).map(|a| a.integrity.as_str()).unwrap_or("")
            ));
        }

        html
    }

    /// Save manifest to file
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

/// Entry point definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    pub file: String,
    pub chunks: Vec<String>,
    pub css: Vec<String>,
    pub async_chunks: Vec<String>,
}
```

### Example: Deep Link Handler

```rust
// utm-webview-deeplink/src/handler.rs

use regex::Regex;
use url::Url;
use std::collections::HashMap;
use crate::{DeepLinkConfig, DeepLinkRoute, DeepLinkError, DeepLinkResult};

/// Deep link handler
pub struct DeepLinkHandler {
    config: DeepLinkConfig,
    routes: Vec<DeepLinkRoute>,
}

impl DeepLinkHandler {
    pub fn new(config: DeepLinkConfig) -> Self {
        let routes = config.routes.iter()
            .filter_map(|r| DeepLinkRoute::new(&r.pattern).ok())
            .collect();

        Self { config, routes }
    }

    /// Handle incoming deep link
    pub fn handle_url(&self, url: &str) -> DeepLinkResult<DeepLinkResult> {
        let parsed = Url::parse(url)?;

        // Verify scheme
        if parsed.scheme() != self.config.scheme {
            return Err(DeepLinkError::InvalidScheme {
                expected: self.config.scheme.clone(),
                actual: parsed.scheme().to_string(),
            });
        }

        // Match route
        let route = self.match_route(parsed.path())?;

        // Extract parameters
        let params = self.extract_params(&route, parsed.path(), parsed.query())?;

        Ok(DeepLinkResult {
            handler: route.handler.clone(),
            params,
            action: crate::DeepLinkAction::Navigate(parsed.path().to_string()),
        })
    }

    fn match_route(&self, path: &str) -> Result<&DeepLinkRoute, DeepLinkError> {
        for route in &self.routes {
            if let Some(captures) = route.regex.captures(path) {
                return Ok(route);
            }
        }
        Err(DeepLinkError::NoMatchingRoute(path.to_string()))
    }

    fn extract_params(
        &self,
        route: &DeepLinkRoute,
        path: &str,
        query: Option<&str>,
    ) -> Result<HashMap<String, String>, DeepLinkError> {
        let mut params = HashMap::new();

        // Extract path parameters
        if let Some(captures) = route.regex.captures(path) {
            for name in &route.param_names {
                if let Some(value) = captures.name(name) {
                    params.insert(name.clone(), value.as_str().to_string());
                }
            }
        }

        // Extract query parameters
        if let Some(query) = query {
            for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
                params.insert(key.to_string(), value.to_string());
            }
        }

        Ok(params)
    }

    /// Generate JavaScript for WebView dispatch
    pub fn generate_js_dispatch(&self, result: &DeepLinkResult) -> String {
        format!(
            "window.dispatchEvent(new CustomEvent('deeplink', {{ detail: {{ handler: '{}', params: {} }} }}))",
            result.handler,
            serde_json::to_string(&result.params).unwrap_or_default()
        )
    }
}

/// Deep link route
pub struct DeepLinkRoute {
    pub pattern: String,
    pub handler: String,
    pub regex: Regex,
    pub param_names: Vec<String>,
}

impl DeepLinkRoute {
    pub fn new(pattern: &str) -> Result<Self, DeepLinkError> {
        // Convert :param to named capture groups
        let regex_pattern = pattern
            .replace(":", "(?P<")
            .replace("/", "\\/")
            .replace("}", ">[^/]+)");

        let regex = Regex::new(&format!("^{}$", regex_pattern))
            .map_err(|e| DeepLinkError::InvalidPattern(e.to_string()))?;

        let param_names = regex.capture_names()
            .flatten()
            .map(String::from)
            .collect();

        Ok(Self {
            pattern: pattern.to_string(),
            handler: String::new(),
            regex,
            param_names,
        })
    }

    pub fn with_handler(mut self, handler: &str) -> Self {
        self.handler = handler.to_string();
        self
    }
}

/// Deep link configuration
#[derive(Debug, Clone)]
pub struct DeepLinkConfig {
    pub scheme: String,
    pub associated_domains: Vec<String>,
    pub routes: Vec<DeepLinkRouteConfig>,
}

#[derive(Debug, Clone)]
pub struct DeepLinkRouteConfig {
    pub pattern: String,
    pub handler: String,
}
```

### Example: PWA Manifest Builder

```rust
// utm-webview-pwa/src/builder.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// PWA web app manifest builder
pub struct WebAppManifestBuilder {
    name: Option<String>,
    short_name: Option<String>,
    description: Option<String>,
    start_url: String,
    scope: String,
    display: String,
    background_color: String,
    theme_color: String,
    icons: Vec<IconInfo>,
    shortcuts: Vec<ShortcutItem>,
    categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconInfo {
    pub src: String,
    pub sizes: Vec<String>,
    pub format: IconFormat,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconFormat {
    Png,
    Svg,
    Webp,
    Ico,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutItem {
    pub name: String,
    pub short_name: String,
    pub description: Option<String>,
    pub url: String,
    pub icons: Vec<ShortcutIcon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutIcon {
    pub src: String,
    pub sizes: String,
}

impl Default for WebAppManifestBuilder {
    fn default() -> Self {
        Self {
            name: None,
            short_name: None,
            description: None,
            start_url: "/".to_string(),
            scope: "/".to_string(),
            display: "standalone".to_string(),
            background_color: "#ffffff".to_string(),
            theme_color: "#000000".to_string(),
            icons: Vec::new(),
            shortcuts: Vec::new(),
            categories: Vec::new(),
        }
    }
}

impl WebAppManifestBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn short_name(mut self, short_name: &str) -> Self {
        self.short_name = Some(short_name.to_string());
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn start_url(mut self, start_url: &str) -> Self {
        self.start_url = start_url.to_string();
        self
    }

    pub fn display(mut self, display: &str) -> Self {
        self.display = display.to_string();
        self
    }

    pub fn background_color(mut self, color: &str) -> Self {
        self.background_color = color.to_string();
        self
    }

    pub fn theme_color(mut self, color: &str) -> Self {
        self.theme_color = color.to_string();
        self
    }

    pub fn icon(mut self, src: &str, sizes: &[u32], format: IconFormat) -> Self {
        self.icons.push(IconInfo {
            src: src.to_string(),
            sizes: sizes.iter().map(|s| format!("{}x{}", s, s)).collect(),
            format,
            purpose: "any maskable".to_string(),
        });
        self
    }

    pub fn shortcut(mut self, shortcut: ShortcutItem) -> Self {
        self.shortcuts.push(shortcut);
        self
    }

    pub fn categories(mut self, categories: Vec<String>) -> Self {
        self.categories = categories;
        self
    }

    pub fn build(self) -> WebAppManifest {
        WebAppManifest {
            name: self.name.unwrap_or_default(),
            short_name: self.short_name.unwrap_or_default(),
            description: self.description,
            start_url: self.start_url,
            scope: self.scope,
            display: self.display,
            background_color: self.background_color,
            theme_color: self.theme_color,
            icons: self.icons,
            shortcuts: self.shortcuts,
            categories: self.categories,
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        let manifest = self.build();
        let json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

/// Web app manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAppManifest {
    pub name: String,
    pub short_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub start_url: String,
    pub scope: String,
    pub display: String,
    pub background_color: String,
    pub theme_color: String,
    pub icons: Vec<IconInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shortcuts: Vec<ShortcutItem>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
}
```

### Example: Offline Sync Queue

```rust
// utm-webview-offline/src/sync_queue.rs

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tokio::sync::Mutex;

/// Offline sync queue for pending actions
pub struct SyncQueue {
    queue: Mutex<VecDeque<PendingAction>>,
    sync_strategy: SyncStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub id: String,
    pub url: String,
    pub method: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<serde_json::Value>,
    pub created_at: u64,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStrategy {
    Immediate,
    Batched { interval_ms: u64 },
    WifiOnly,
    Manual,
}

impl SyncQueue {
    pub fn new(strategy: SyncStrategy) -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            sync_strategy: strategy,
        }
    }

    /// Add action to queue
    pub async fn enqueue(&self, action: PendingAction) {
        let mut queue = self.queue.lock().await;
        queue.push_back(action);
    }

    /// Get next action to sync
    pub async fn dequeue(&self) -> Option<PendingAction> {
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }

    /// Get queue length
    pub async fn len(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }

    /// Check if queue is empty
    pub async fn is_empty(&self) -> bool {
        let queue = self.queue.lock().await;
        queue.is_empty()
    }

    /// Get all pending actions
    pub async fn get_all(&self) -> Vec<PendingAction> {
        let queue = self.queue.lock().await;
        queue.iter().cloned().collect()
    }

    /// Remove action by ID
    pub async fn remove(&self, id: &str) {
        let mut queue = self.queue.lock().await;
        queue.retain(|a| a.id != id);
    }

    /// Clear all actions
    pub async fn clear(&self) {
        let mut queue = self.queue.lock().await;
        queue.clear();
    }

    /// Get sync strategy
    pub fn strategy(&self) -> &SyncStrategy {
        &self.sync_strategy
    }
}

impl PendingAction {
    pub fn new(method: &str, url: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            url: url.to_string(),
            method: method.to_string(),
            headers: std::collections::HashMap::new(),
            body: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            retry_count: 0,
        }
    }

    pub fn with_body(mut self, body: serde_json::Value) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    pub fn should_retry(&self, max_retries: u32) -> bool {
        self.retry_count < max_retries
    }
}
```

## Migration Path

1. **Week 1-2: Core Infrastructure**
   - Set up workspace structure
   - Implement core types and hashing utilities
   - Create asset manifest generator

2. **Week 3-4: Service Worker**
   - Implement service worker bundler
   - Add precache manifest generation
   - Test with esbuild integration

3. **Week 5-6: PWA Features**
   - Build PWA manifest builder
   - Add icon generation
   - Implement shortcut items

4. **Week 7-8: Deep Links**
   - Implement deep link handler
   - Add route pattern matching
   - Platform-specific configuration

5. **Week 9-10: Offline Support**
   - Build sync queue
   - Add IndexedDB wrapper
   - Implement cache strategies

## Performance Considerations

1. **Asset hashing** - Use parallel hashing for large codebases
2. **Manifest generation** - Incremental updates for changed files only
3. **Service worker** - Pre-generate during build, not runtime
4. **Deep link matching** - Compiled regex for efficiency
5. **Offline queue** - Bounded queue size to prevent memory growth

## Testing Strategy

1. **Unit tests** for hash computation, route matching
2. **Integration tests** for manifest generation
3. **End-to-end tests** for service worker caching
4. **PWA tests** for installability criteria
5. **Offline tests** for sync queue behavior

## Open Considerations

1. **Service worker updates** - Strategy for forcing updates
2. **Cache versioning** - How to handle breaking cache changes
3. **Icon generation** - External tool vs. Rust implementation
4. **Deep link testing** - Platform-specific testing challenges
5. **Offline conflict resolution** - Handling conflicting updates
