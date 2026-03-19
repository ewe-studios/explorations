---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/config/
explored_at: 2026-03-19T12:00:00Z
package: pkg/config
---

# Deep Dive: Configuration (pkg/config/)

## Overview

The `pkg/config` package is the central configuration hub for utm-dev, responsible for:

- SDK path resolution (OS-specific)
- Cache directory management
- Android/iOS build defaults
- SDK metadata parsing from embedded JSON files

## Directory Resolution

### OS-Specific Paths

The package implements the XDG Base Directory Specification on Linux and follows platform conventions:

```go
// Cache directory (for temporary build artifacts)
GetCacheDir()
├── macOS: ~/utm-dev-cache/
├── Linux: ~/.cache/utm-dev/
└── Windows: %LOCALAPPDATA%/utm-dev/

// SDK directory (for installed toolchains)
GetSDKDir()
├── macOS: ~/utm-dev-sdks/
├── Linux: ~/.local/share/utm-dev/sdks/
└── Windows: %APPDATA%/utm-dev/sdks/
```

### Implementation

```go
func GetCacheDir() string {
    switch runtime.GOOS {
    case "darwin":
        return filepath.Join(home, "utm-dev-cache")
    case "linux":
        if cacheHome := os.Getenv("XDG_CACHE_HOME"); cacheHome != "" {
            return filepath.Join(cacheHome, "utm-dev")
        }
        return filepath.Join(home, ".cache", "utm-dev")
    case "windows":
        return filepath.Join(localAppData, "utm-dev")
    }
    return filepath.Join(home, ".utm-dev")  // Fallback
}

func GetSDKDir() string {
    switch runtime.GOOS {
    case "darwin":
        return filepath.Join(home, "utm-dev-sdks")
    case "linux":
        if dataHome := os.Getenv("XDG_DATA_HOME"); dataHome != "" {
            return filepath.Join(dataHome, "utm-dev", "sdks")
        }
        return filepath.Join(home, ".local", "share", "utm-dev", "sdks")
    case "windows":
        return filepath.Join(appData, "utm-dev", "sdks")
    }
    return filepath.Join(GetCacheDir(), "sdks")  // Fallback
}
```

### Environment Variable Precedence

1. XDG environment variables (Linux)
2. Platform-standard directories
3. Fallback to legacy `~/.utm-dev`

## Build Defaults

### Android Build Configuration

Embedded from `sdk-android-list.json`:

```go
type AndroidBuildDefaults struct {
    MinSdk    int  // Minimum SDK level (default: 21)
    TargetSdk int  // Target SDK level (default: 34)
}

func GetAndroidMinSdk() string {
    defaults := GetAndroidBuildDefaults()
    return strconv.Itoa(defaults.MinSdk)  // Returns "21"
}
```

**Usage in gogio:**
```go
// cmd/build.go
minSdk := config.GetAndroidMinSdk()  // "21"
args := []string{"-target", "android", "-minsdk", minSdk, "-o", apkPath}
```

### iOS Build Configuration

Embedded from `sdk-ios-list.json`:

```go
type IOSBuildDefaults struct {
    MinOS string  // Minimum iOS version (default: "15.0")
}

func GetIOSMinOS() string {
    defaults := GetIOSBuildDefaults()
    // Strips minor version: "15.0" -> "15" for gogio
    for i, c := range defaults.MinOS {
        if c == '.' {
            return defaults.MinOS[:i]
        }
    }
    return defaults.MinOS
}
```

## Embedded SDK Metadata

### Embedded Files

```go
//go:embed sdk-android-list.json
var AndroidSdkList []byte

//go:embed sdk-ios-list.json
var IosSdkList []byte

//go:embed sdk-build-tools.json
var BuildToolsSdkList []byte
```

### SDK Item Structure

```go
type SdkItem struct {
    Version        string              `json:"version"`
    GoupName       string              `json:"goupName"`
    DownloadURL    string              `json:"downloadUrl,omitempty"`
    Checksum       string              `json:"checksum,omitempty"`
    InstallPath    string              `json:"installPath"`
    ApiLevel       int                 `json:"apiLevel"`
    Abi            string              `json:"abi"`
    Vendor         string              `json:"vendor"`
    Platforms      map[string]Platform `json:"platforms,omitempty"`
    SdkManagerName string              `json:"sdkmanagerName,omitempty"`
}

type Platform struct {
    DownloadURL string  `json:"downloadUrl"`
    Checksum    string  `json:"checksum"`
}
```

### Metadata Schema with Meta

```go
type AndroidSdkFile struct {
    SDKs map[string][]SdkItem `json:"sdks"`
    Meta struct {
        SchemaVersion string               `json:"schemaVersion"`
        BuildDefaults AndroidBuildDefaults `json:"buildDefaults"`
        Setups        map[string][]string  `json:"setups"`
    } `json:"meta"`
}
```

**Example (sdk-android-list.json):**
```json
{
  "sdks": {
    "ndk-bundle": [
      {
        "version": "25.2.9519653",
        "goupName": "ndk",
        "downloadUrl": "https://dl.google.com/android/repository/...",
        "checksum": "sha256:abc123...",
        "installPath": "ndk-bundle"
      }
    ]
  },
  "meta": {
    "schemaVersion": "1.0",
    "buildDefaults": {
      "minSdk": 21,
      "targetSdk": 34
    },
    "setups": {
      "android": ["platform-tools", "build-tools;34.0.0", "platforms;android-34"]
    }
  }
}
```

## Directory Operations

### EnsureDirectories

```go
func EnsureDirectories() error {
    cacheDir := GetCacheDir()
    sdkDir := GetSDKDir()

    if err := os.MkdirAll(cacheDir, 0755); err != nil {
        return fmt.Errorf("failed to create cache directory: %w", err)
    }

    if err := os.MkdirAll(sdkDir, 0755); err != nil {
        return fmt.Errorf("failed to create SDK directory: %w", err)
    }

    return nil
}
```

### CleanDirectories

```go
func CleanDirectories() error {
    sdkDir := GetSDKDir()
    cacheDir := GetCacheDir()

    var errors []error

    if _, err := os.Stat(sdkDir); err == nil {
        if err := os.RemoveAll(sdkDir); err != nil {
            errors = append(errors, fmt.Errorf("failed to remove SDK: %w", err))
        }
    }

    if _, err := os.Stat(cacheDir); err == nil {
        if err := os.RemoveAll(cacheDir); err != nil {
            errors = append(errors, fmt.Errorf("failed to remove cache: %w", err))
        }
    }

    if len(errors) > 0 {
        return fmt.Errorf("cleanup errors: %v", errors)
    }

    return nil
}
```

### GetDirectoryInfo

Returns diagnostic information about directories:

```go
type DirectoryInfo struct {
    CacheDir    string  // Cache directory path
    SDKDir      string  // SDK directory path
    CacheExists bool    // Whether cache directory exists
    SDKExists   bool    // Whether SDK directory exists
    CacheSize   int64   // Cache size in bytes
    SDKSize     int64   // SDK size in bytes
}
```

**Usage:**
```go
info := config.GetDirectoryInfo()
fmt.Printf("Cache: %s (%d bytes, exists=%v)\n", info.CacheDir, info.CacheSize, info.CacheExists)
```

## Cache Path Helpers

```go
// GetCachePath returns the full path to cache.json
func GetCachePath() string {
    return filepath.Join(GetCacheDir(), "cache.json")
}
```

## Usage Patterns

### In Build Commands

```go
// cmd/build.go - Android build
sdkRoot := config.GetSDKDir()
ndkPath := filepath.Join(sdkRoot, "ndk-bundle")

// Check for NDK
if _, err := os.Stat(ndkPath); os.IsNotExist(err) {
    // Auto-install NDK
    installNDK(sdkRoot)
}

// Set environment
javaHome := filepath.Join(sdkRoot, "openjdk", "17", "jdk-17.0.11+9", "Contents", "Home")
env = append(env, "JAVA_HOME="+javaHome)
env = append(env, "ANDROID_SDK_ROOT="+sdkRoot)
```

### In Installer

```go
// pkg/installer/installer.go
dest, err := installer.ResolveInstallPath(sdk.InstallPath)
// Uses config.GetSDKDir() for default paths
```

### In ADB/Simctl

```go
// pkg/adb/adb.go
func New() *Client {
    return &Client{sdkDir: config.GetSDKDir()}
}

func (c *Client) ADBPath() string {
    return filepath.Join(c.sdkDir, "platform-tools", "adb")
}
```

## Design Decisions

### 1. Embedded Configuration

**Why:**
- Self-contained binary
- No external config file dependencies
- Version-controlled defaults

**Trade-off:** Requires rebuild to change defaults

### 2. OS-Specific Defaults

**Why:**
- Follows platform conventions
- XDG compliance on Linux
- User expectations on macOS/Windows

### 3. String Return for SDK Versions

```go
func GetAndroidMinSdk() string  // Returns "21", not 21
```

**Why:** gogio expects string arguments, so we return pre-formatted strings.

### 4. Graceful Degradation

If embedded JSON fails to parse:
```go
func GetAndroidBuildDefaults() AndroidBuildDefaults {
    var sdkFile AndroidSdkFile
    if err := json.Unmarshal(AndroidSdkList, &sdkFile); err != nil {
        return AndroidBuildDefaults{MinSdk: 21, TargetSdk: 34}  // Sensible defaults
    }
    return sdkFile.Meta.BuildDefaults
}
```

## Testing

### config_test.go

Tests path resolution and directory operations.

## Files

| File | Purpose |
|------|---------|
| `config.go` | Main configuration logic |
| `sdk-android-list.json` | Android SDK definitions |
| `sdk-ios-list.json` | iOS SDK definitions |
| `sdk-build-tools.json` | Build-tools definitions |
| `config_test.go` | Unit tests |

## Environment Variables Reference

| Variable | Platform | Purpose |
|----------|----------|---------|
| `XDG_CACHE_HOME` | Linux | Override cache directory |
| `XDG_DATA_HOME` | Linux | Override SDK directory |
| `LOCALAPPDATA` | Windows | Cache directory source |
| `APPDATA` | Windows | SDK directory source |

## Future Enhancements

1. **User Configuration File:** `~/.utm-dev/config.json` for user overrides
2. **Environment Variable Substitution:** In SDK URLs
3. **Plugin SDKs:** Third-party SDK registration
4. **Version Pinning:** Per-project SDK version configuration
