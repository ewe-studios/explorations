---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/
explored_at: 2026-03-19T12:00:00Z
---

# Deep Dive: Remaining Packages Summary

## pkg/logging/ - Platform-Specific Logging

### Structure
```
pkg/logging/
├── logging.go         # Main logging interface
├── platform.go        # Platform detection
├── platform_native.go # Native platform logging (iOS/Android)
└── platform_js.go     # JavaScript/Web logging
```

### Purpose
Provides unified logging interface with platform-specific implementations:
- **Native:** iOS (NSLog), Android (logcat)
- **Web:** JavaScript console
- **Desktop:** stdout/stderr

### Build Tags
```go
// +build ios android
// platform_native.go

// +build js wasm
// platform_js.go

// +build !ios,!android,!js
// platform.go (fallback)
```

---

## pkg/screenshot/ - Screenshot Capabilities

### Structure
```
pkg/screenshot/
├── screenshot.go         # Main screenshot interface
├── cgwindow_darwin.go    # macOS CoreGraphics implementation
├── cgwindow_linux.go     # Linux implementation (stub)
├── cgwindow_windows.go   # Windows implementation (stub)
└── presets.go            # Screenshot presets
```

### macOS Implementation (cgwindow_darwin.go)
Uses CoreGraphics Window Server API:

```go
func CaptureWindow() (image.Image, error) {
    // CGWindowListCreateImage
    // kCGWindowListOptionOnScreenOnly
    // kCGNullWindowID
}
```

### presets.go
Common screenshot configurations:
- Full screen
- Active window
- Specific region

---

## pkg/packaging/ - Distribution Packaging

### Structure
```
pkg/packaging/
├── archive.go     # Archive creation (tar.gz, zip)
├── macos.go       # macOS package creation
├── windows.go     # Windows MSIX/MSI creation
└── linux.go       # Linux deb/rpm creation
```

### archive.go
```go
func CreateTarGz(sourceDir, outputPath string) error
func CreateZip(sourceDir, outputPath string) error
```

### macos.go
- App bundle creation
- Code signing
- Notarization preparation

### windows.go
- MSIX manifest generation
- MSIX package creation
- Code signing

---

## pkg/appconfig/ - App Configuration

### Purpose
Handles `app.json` configuration files for Gio apps:

```json
{
  "name": "myapp",
  "version": "1.0.0",
  "bundleId": "com.example.myapp",
  "icon": "icon-source.png",
  "deepLinking": {
    "schemes": ["myapp://"],
    "queries": ["com.google.android.apps.maps"]
  }
}
```

### Key Functions
```go
func Load(appDir string) (*AppConfig, error)
func Save(config *AppConfig, appDir string) error
func Validate(config *AppConfig) error
```

---

## pkg/schema/ - JSON Schema Definitions

### Purpose
Provides JSON schema definitions for validation:

```go
// Platforms defines supported platforms
var Platforms = []string{"macos", "android", "ios", "ios-simulator", "windows", "linux"}

// PlatformDescriptions provides help text
var PlatformDescriptions = map[string]string{
    "macos":        "macOS ARM64 (Apple Silicon)",
    "android":      "Android APK (ARM64)",
    "ios":          "iOS device (ARM64)",
    "ios-simulator": "iOS Simulator (x86_64/ARM64)",
    "windows":      "Windows x64",
    "linux":        "Linux x86_64",
}
```

---

## pkg/utils/ - Utility Functions

### utils.go
Common utility functions:

```go
// Contains checks if a slice contains a value
func Contains(slice []string, item string) bool

// FileExists checks if a file exists
func FileExists(path string) bool

// CopyFile copies a file
func CopyFile(src, dst string) error

// EnsureDir creates a directory if it doesn't exist
func EnsureDir(path string) error
```

---

## pkg/constants/ - Project Constants

### directories.go
```go
const (
    BinDir   = ".bin"    // Build output directory
    BuildDir = ".build"  // Build artifacts directory
    DistDir  = ".dist"   // Distribution directory
)
```

### Common paths used throughout the codebase.

---

## pkg/gitignore/ - .gitignore Management

### Purpose
Manages `.gitignore` files for Gio projects:

```go
func EnsureGitignore(projectDir string) error
func AddPatterns(projectDir string, patterns ...string) error
```

### Default Patterns
```
.bin/
.build/
.dist/
*.apk
*.app
*.exe
icon-source.png
```

---

## pkg/service/ - Service Management

### Purpose
Background service handling for apps:

```go
type Service interface {
    Start() error
    Stop() error
    Status() string
}
```

---

## pkg/updater/ - Auto-Update

### Purpose
Automatic update checking and installation:

```go
func CheckForUpdates() (bool, string, error)
func DownloadUpdate(version string) error
func InstallUpdate() error
```

### Update Source
Checks GitHub releases:
```
https://github.com/joeblew999/utm-dev/releases/latest
```

---

## Package Dependency Graph

```
cmd/
├── project/       (project structure)
├── buildcache/    (build caching)
├── icons/         (icon generation)
├── installer/     (SDK installation)
├── utm/           (VM management)
├── adb/           (Android debugging)
├── simctl/        (iOS simulation)
├── workspace/     (Go workspaces)
├── config/        (configuration)
└── self/          (self-management)

pkg/
├── config/
│   └── (embedded JSON files)
├── project/
│   └── constants/
├── icons/
│   ├── constants/
│   └── utils/
├── installer/
│   ├── config/
│   └── utils/
├── utm/
│   └── config/
├── adb/
│   └── config/
├── simctl/
│   └── (no dependencies)
├── workspace/
│   └── utils/
└── self/
    ├── config/
    ├── output/
    └── utils/
```

---

## Testing Strategy

| Package | Test File | Coverage |
|---------|-----------|----------|
| config/ | config_test.go | Path resolution |
| installer/ | installer_test.go | SDK installation |
| workspace/ | workspace_test.go | Workspace parsing |
| gitignore/ | gitignore_test.go | Pattern matching |
| self/ | self_test.go | Doctor validation |

---

## Common Patterns Across Packages

### 1. Path Resolution
All packages use `pkg/config` for OS-specific paths:
```go
sdkDir := config.GetSDKDir()
cacheDir := config.GetCacheDir()
```

### 2. Error Wrapping
Consistent error wrapping for debugging:
```go
return fmt.Errorf("operation failed: %w", err)
```

### 3. Idempotent Operations
Most operations are safe to call multiple times:
```go
// Safe to call repeatedly
config.EnsureDirectories()
workspace.AddModule("./myapp", true)
```

### 4. Graceful Degradation
Packages handle missing dependencies gracefully:
```go
if !client.Available() {
    return fmt.Errorf("tool not found - install with: utm-dev install X")
}
```
