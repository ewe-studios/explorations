---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/installer/
explored_at: 2026-03-19T12:00:00Z
package: pkg/installer
---

# Deep Dive: SDK Installer (pkg/installer/)

## Overview

The `pkg/installer` package handles downloading, verifying, and installing mobile development SDKs. It provides:

- Progress-bar downloads with retry logic
- SHA256 checksum verification
- Archive extraction (tar.gz, zip)
- Download caching
- Android SDK Manager integration

## Core Types

### SDK Structure

```go
type SDK struct {
    Name        string  // Human-readable name ("Android NDK")
    Version     string  // Version string ("25.2.9519653")
    URL         string  // Download URL
    Checksum    string  // SHA256 checksum ("sha256:abc123...")
    InstallPath string  // Installation directory
}
```

### Cache Structure

```go
type Cache struct {
    path      string          // Path to cache.json
    Installed map[string]bool // Set of installed SDK names
}
```

## Installation Flow

### High-Level Flow

```
1. Check cache for existing installation
2. Resolve installation path
3. Download with progress bar (retry logic)
4. Verify SHA256 checksum
5. Extract archive
6. Update cache
```

### Install() Function

```go
func Install(sdk *SDK, cache *Cache) error {
    // Step 1: Check if already installed
    if cache.IsCached(sdk) {
        fmt.Printf("%s %s is already installed and up-to-date.\n", sdk.Name, sdk.Version)
        return nil
    }

    // Step 2: Resolve installation path
    dest, err := ResolveInstallPath(sdk.InstallPath)
    if err != nil {
        return err
    }

    // Step 3: Check if already installed on disk
    if _, err := os.Stat(dest); err == nil {
        if isSDKComplete(dest, sdk.Name) {
            cache.Add(sdk)
            cache.Save()
            return nil
        }
        os.RemoveAll(dest)  // Incomplete, reinstall
    }

    // Step 4: Manual installation check
    if sdk.URL == "" {
        return fmt.Errorf("cannot automatically install %s", sdk.Name)
    }

    // Step 5: Download with retry
    tmpFile, err := os.CreateTemp("", "sdk-download-*")
    defer os.Remove(tmpFile.Name())

    client := &http.Client{Timeout: 60 * time.Minute}
    req, _ := http.NewRequest("GET", sdk.URL, nil)

    maxRetries := 3
    var resp *http.Response
    for attempt := 1; attempt <= maxRetries; attempt++ {
        resp, err = client.Do(req)
        if err == nil && resp.StatusCode == http.StatusOK {
            break
        }
        if attempt < maxRetries {
            time.Sleep(time.Duration(attempt) * time.Second)
        }
    }

    // Step 6: Download with progress bar
    bar := progressbar.NewOptions64(contentLength,
        progressbar.OptionShowBytes(true),
        progressbar.OptionSetWidth(50),
    )

    hasher := sha256.New()
    teeReader := io.TeeReader(&progressReader, hasher)
    io.Copy(tmpFile, teeReader)
    tmpFile.Close()

    // Step 7: Verify checksum
    calculatedChecksum := hex.EncodeToString(hasher.Sum(nil))
    expectedChecksum := strings.TrimPrefix(sdk.Checksum, "sha256:")

    if calculatedChecksum != expectedChecksum {
        return fmt.Errorf("checksum mismatch")
    }
    fmt.Println("✓ Checksum verified.")

    // Step 8: Extract archive
    os.MkdirAll(dest, 0755)
    Extract(tmpFile.Name(), dest)

    // Step 9: Update cache
    cache.Add(sdk)
    cache.Save()

    return nil
}
```

## Download Logic

### Retry with Exponential Backoff

```go
maxRetries := 3
for attempt := 1; attempt <= maxRetries; attempt++ {
    fmt.Printf("🔄 Download attempt %d/%d...\n", attempt, maxRetries)

    resp, err = client.Do(req)
    if err == nil && resp.StatusCode == http.StatusOK {
        break
    }

    if attempt < maxRetries {
        if resp != nil {
            resp.Body.Close()
        }
        backoff := time.Duration(attempt) * time.Second
        fmt.Printf("⏳ Retrying in %v...\n", backoff)
        time.Sleep(backoff)
    }
}
```

### Progress Bar

Uses `github.com/schollz/progressbar/v3`:

```go
bar := progressbar.NewOptions64(contentLength,
    progressbar.OptionSetDescription("Downloading"),
    progressbar.OptionSetWriter(os.Stderr),
    progressbar.OptionShowBytes(true),
    progressbar.OptionSetWidth(50),
    progressbar.OptionThrottle(65*time.Millisecond),
    progressbar.OptionShowCount(),
    progressbar.OptionOnCompletion(func() {
        fmt.Fprint(os.Stderr, "\n")
    }),
)
```

**Output:**
```
Downloading  45% |█████████░░░░░░░░░░░░░| (1.2/2.7 GB, 15 MB/s)
```

### Checksum Verification

```go
// Hash during download
hasher := sha256.New()
teeReader := io.TeeReader(&progressReader, hasher)
io.Copy(tmpFile, teeReader)

// Verify after download
calculatedChecksum := hex.EncodeToString(hasher.Sum(nil))
expectedChecksum := strings.TrimPrefix(sdk.Checksum, "sha256:")

if calculatedChecksum != expectedChecksum {
    return fmt.Errorf("checksum mismatch: expected %s, got %s",
        expectedChecksum, calculatedChecksum)
}
```

## SDK Completion Verification

### isSDKComplete()

Checks if an SDK installation is complete by verifying expected files:

```go
func isSDKComplete(dest, sdkName string) bool {
    switch {
    case strings.Contains(sdkName, "openjdk"):
        // Check for Java executable
        javaPath := filepath.Join(dest, "bin", "java")
        if runtime.GOOS == "windows" {
            javaPath += ".exe"
        }
        _, err := os.Stat(javaPath)
        return err == nil

    case strings.Contains(sdkName, "android"):
        // Check for android.jar
        _, err := os.Stat(filepath.Join(dest, "android.jar"))
        return err == nil

    case strings.Contains(sdkName, "ndk"):
        // Check for ndk-build
        ndkBuildPath := filepath.Join(dest, "ndk-build")
        if runtime.GOOS == "windows" {
            ndkBuildPath += ".cmd"
        }
        _, err := os.Stat(ndkBuildPath)
        return err == nil

    case strings.Contains(sdkName, "platform-tools"):
        // Check for adb
        adbPath := filepath.Join(dest, "adb")
        if runtime.GOOS == "windows" {
            adbPath += ".exe"
        }
        _, err := os.Stat(adbPath)
        return err == nil

    default:
        return true
    }
}
```

## Path Resolution

### ResolveInstallPath()

```go
func ResolveInstallPath(path string) (string, error) {
    if path == "" {
        // Default to OS-specific SDK directory
        return config.GetSDKDir(), nil
    }

    // Expand environment variables
    expandedPath := os.ExpandEnv(path)

    // If absolute, return as-is
    if filepath.IsAbs(expandedPath) {
        return expandedPath, nil
    }

    // Relative paths starting with "sdks/" use SDK dir
    if strings.HasPrefix(expandedPath, "sdks/") {
        return filepath.Join(
            config.GetSDKDir(),
            strings.TrimPrefix(expandedPath, "sdks/"),
        ), nil
    }

    // Other relative paths use current directory
    if !filepath.IsAbs(expandedPath) {
        if cwd, err := os.Getwd(); err == nil {
            return filepath.Join(cwd, expandedPath), nil
        }
    }

    return expandedPath, nil
}
```

**Examples:**
```go
ResolveInstallPath("")                          // ~/utm-dev-sdks/
ResolveInstallPath("sdks/ndk-bundle")           // ~/utm-dev-sdks/ndk-bundle
ResolveInstallPath("/opt/android-sdk")          // /opt/android-sdk
ResolveInstallPath("$HOME/my-sdk")              // /home/user/my-sdk
```

## Android SDK Manager Integration

### InstallAndroidSDK()

Uses Android's `sdkmanager` tool for component installation:

```go
func InstallAndroidSDK(sdkName, sdkManagerName, sdkRoot string) error {
    sdkRoot = filepath.Clean(sdkRoot)
    cmdlineToolsPath := filepath.Join(sdkRoot, "cmdline-tools", "11.0", "cmdline-tools", "bin")
    javaHome := filepath.Join(sdkRoot, "openjdk", "17", "jdk-17.0.11+9", "Contents", "Home")

    sdkManagerPath := filepath.Join(cmdlineToolsPath, "sdkmanager")
    if _, err := os.Stat(sdkManagerPath); os.IsNotExist(err) {
        return fmt.Errorf("sdkmanager not found at %s", sdkManagerPath)
    }

    // Set environment
    env := os.Environ()
    env = append(env, "JAVA_HOME="+javaHome)
    env = append(env, "ANDROID_SDK_ROOT="+sdkRoot)
    env = append(env, "ANDROID_HOME="+sdkRoot)
    pathEnv := "PATH=" + cmdlineToolsPath + string(os.PathListSeparator) + os.Getenv("PATH")
    env = append(env, pathEnv)

    // Run sdkmanager
    cmd := exec.Command(sdkManagerPath, sdkManagerName, "--sdk_root="+sdkRoot)
    cmd.Env = env
    cmd.Stdout = os.Stdout
    cmd.Stderr = os.Stderr

    // Retry logic
    maxRetries := 3
    for attempt := 1; attempt <= maxRetries; attempt++ {
        err := cmd.Run()
        if err == nil {
            fmt.Printf("✓ Successfully installed %s\n", sdkName)
            return nil
        }

        if attempt < maxRetries {
            time.Sleep(time.Duration(attempt) * time.Second)
        }
    }

    return fmt.Errorf("failed after %d attempts: %w", maxRetries, err)
}
```

**Usage:**
```go
// Install build-tools
InstallAndroidSDK("build-tools", "build-tools;34.0.0", sdkRoot)

// Install platform
InstallAndroidSDK("Android Platform", "platforms;android-34", sdkRoot)
```

## Cache Operations

### Cache Implementation

```go
type Cache struct {
    path      string
    Installed map[string]bool
}

func NewCache(cacheFile string) (*Cache, error) {
    cache := &Cache{
        path:      cacheFile,
        Installed: make(map[string]bool),
    }
    // Load existing cache if present
    // ...
    return cache, nil
}

func (c *Cache) IsCached(sdk *SDK) bool {
    key := fmt.Sprintf("%s:%s", sdk.Name, sdk.Version)
    return c.Installed[key]
}

func (c *Cache) Add(sdk *SDK) {
    key := fmt.Sprintf("%s:%s", sdk.Name, sdk.Version)
    c.Installed[key] = true
}

func (c *Cache) Save() error {
    data, err := json.MarshalIndent(c.Installed, "", "  ")
    return os.WriteFile(c.path, data, 0644)
}
```

### Cache File Format

```json
{
  "Android NDK:25.2.9519653": true,
  "OpenJDK:17.0.11": true,
  "Android Platform:34": true
}
```

## Archive Extraction

### Extract()

```go
func Extract(archivePath, destPath string) error {
    // Detect archive type from extension
    if strings.HasSuffix(archivePath, ".tar.gz") || strings.HasSuffix(archivePath, ".tgz") {
        return extractTarGz(archivePath, destPath)
    }
    if strings.HasSuffix(archivePath, ".zip") {
        return extractZip(archivePath, destPath)
    }
    return fmt.Errorf("unsupported archive format")
}

func extractTarGz(archivePath, destPath string) error {
    f, _ := os.Open(archivePath)
    defer f.Close()

    gzr, _ := gzip.NewReader(f)
    defer gzr.Close()

    tr := tar.NewReader(gzr)
    for {
        header, err := tr.Next()
        if err == io.EOF {
            break
        }

        target := filepath.Join(destPath, header.Name)

        switch header.Typeflag {
        case tar.TypeDir:
            os.MkdirAll(target, 0755)
        case tar.TypeReg:
            os.MkdirAll(filepath.Dir(target), 0755)
            f, _ := os.OpenFile(target, os.O_CREATE|os.O_WRONLY, os.FileMode(header.Mode))
            io.Copy(f, tr)
            f.Close()
        }
    }
    return nil
}
```

## JDK Post-Install Instructions

```go
if strings.Contains(sdk.Name, "openjdk") {
    fmt.Println("\n---------------------------------------------------------------------")
    fmt.Println("IMPORTANT: To use this JDK for Android development with Gio,")
    fmt.Println("you need to set the JAVA_HOME environment variable.")
    fmt.Println("\nFor your current shell session, run:")
    fmt.Printf("export JAVA_HOME=\"%s\"\n", dest)
    fmt.Println("\nTo make this change permanent, add the line above to your")
    fmt.Println("shell profile file (e.g., ~/.zshrc, ~/.bash_profile).")
    fmt.Println("---------------------------------------------------------------------")
}
```

## Usage Patterns

### In cmd/install.go

```go
func runInstall(cmd *cobra.Command, args []string) error {
    sdkName := args[0]

    // Get SDK details from config
    sdk := getSDKFromConfig(sdkName)

    cache, _ := installer.NewCache(config.GetCachePath())

    return installer.Install(sdk, cache)
}
```

### In cmd/build.go (NDK Auto-Install)

```go
func installNDK(sdkRoot string) error {
    ndkSDK := &installer.SDK{
        Name:        "Android NDK",
        Version:     "latest",
        InstallPath: "ndk-bundle",
    }

    cache, _ := installer.NewCache(filepath.Join(config.GetCacheDir(), "cache.json"))
    return installer.Install(ndkSDK, cache)
}
```

## Design Decisions

### 1. Download Caching

**Why:** Avoids re-downloading large SDKs (NDK is 1GB+).

**Implementation:** Cache file tracks installed SDKs by name:version.

### 2. Checksum During Download

**Why:** No need to read file twice - hash while downloading.

```go
teeReader := io.TeeReader(&progressReader, hasher)
io.Copy(tmpFile, teeReader)
```

### 3. Temp File for Downloads

**Why:**
- Atomic installation (delete on failure)
- No partial installations
- Easy cleanup

### 4. SDK Completion Verification

**Why:** Detects incomplete downloads or extraction failures.

### 5. Retry Logic

**Why:** Large downloads often fail transiently.

## Error Handling

### Graceful Failures

```go
if err := cmd.Run(); err != nil {
    return fmt.Errorf("failed to install %s after %d attempts: %w",
        sdkName, maxRetries, err)
}
```

### Incomplete Installation Cleanup

```go
if _, err := os.Stat(dest); err == nil {
    if !isSDKComplete(dest, sdk.Name) {
        fmt.Printf("⚠️  %s found but appears incomplete, reinstalling...\n", sdk.Name)
        os.RemoveAll(dest)
    }
}
```

## Future Enhancements

1. **Parallel Downloads:** Download multiple SDKs concurrently
2. **Delta Updates:** Only download changed components
3. **SDK List Command:** `utm-dev list --installed`
4. **SDK Upgrade:** Auto-detect and upgrade outdated SDKs
5. **Mirror Support:** Configurable download mirrors
