---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/utm
repository: git@github.com:joeblew999/utm-dev.git
explored_at: 2026-03-19
language: Go
parent: exploration.md
---

# pkg/utm Deep Dive - UTM Virtualization Integration

## Overview

The `pkg/utm` package provides comprehensive UTM (Universal Terminal for Macs) virtualization integration for macOS. It automates VM creation, configuration, and management through AppleScript and the `utmctl` CLI tool.

**Key Insight:** This is a macOS-only package that bridges Go code with UTM's AppleScript automation layer, adapted from the [packer-plugin-utm](https://github.com/naveenrajm7/packer-plugin-utm) project.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    utm-dev CLI (cmd/utm.go)                     │
├─────────────────────────────────────────────────────────────────┤
│                      pkg/utm (This Package)                     │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐            │
│  │ driver.go   │  │ osascript.go │  │ utmctl.go   │            │
│  │ Version     │  │ AppleScript  │  │ utmctl      │            │
│  │ Detection   │  │ Execution    │  │ Wrapper     │            │
│  └─────────────┘  └──────────────┘  └─────────────┘            │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐            │
│  │ gallery.go  │  │ install.go   │  │ create.go   │            │
│  │ VM Templates│  │ UTM App      │  │ VM Creation │            │
│  │ ISO Config  │  │ ISO Download │  │ Automation  │            │
│  └─────────────┘  └──────────────┘  └─────────────┘            │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐            │
│  │ advanced.go │  │ cache.go     │  │ migrate.go  │            │
│  │ Port Forward│  │ Idempotency  │  │ Legacy→Global│           │
│  │ Export/Import│ │ Checksums   │  │ Migration   │            │
│  └─────────────┘  └──────────────┘  └─────────────┘            │
├─────────────────────────────────────────────────────────────────┤
│                    macOS System Layer                           │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐            │
│  │ UTM.app     │  │ osascript     │  │ utmctl      │            │
│  │ (QEMU/      │  │ (AppleScript  │  │ (UTM CLI    │            │
│  │  Apple HV)  │  │  Automation)  │  │  Tool)      │            │
│  └─────────────┘  └──────────────┘  └─────────────┘            │
└─────────────────────────────────────────────────────────────────┘
```

## Virtualization Technology

### Backend Types

UTM supports two virtualization backends:

| Backend | Code | Description | Use Case |
|---------|------|-------------|----------|
| **QEMU** | `QeMu` | Full system emulation | All architectures, cross-arch VMs |
| **Apple HV** | `ApLe` | Apple Hypervisor framework | Native ARM only, better performance |

```go
// From osascript.go
const (
    BackendQEMU VMBackend = "QeMu"  // Works for all architectures
    BackendApple VMBackend = "ApLe"  // Native ARM only
)

// GetBackendForOS always returns QEMU for better compatibility
func GetBackendForOS(osType, arch string) VMBackend {
    return BackendQEMU  // Users can switch manually in UTM if needed
}
```

### Why QEMU by Default?

The code explicitly chooses QEMU over Apple Hypervisor:

```go
// osascript.go:193-200
// GetBackendForOS returns the appropriate backend for the given OS/arch
func GetBackendForOS(osType, arch string) VMBackend {
    // Apple Virtualization framework only works for native ARM arch
    // and has limited OS support (Linux only, no Windows)
    // For now, always use QEMU for better compatibility and features
    // Users can switch to Apple backend manually in UTM if needed
    return BackendQEMU
}
```

**Rationale:**
1. Apple HV only supports native ARM architecture
2. Apple HV has limited OS support (Linux only, no Windows)
3. QEMU provides broader compatibility and more features

## Platform Support

### macOS (Primary Platform)

UTM is **macOS-only**. The package enforces this:

```go
// install.go:23-26
func GetInstallStatus() (*InstallStatus, error) {
    if runtime.GOOS != "darwin" {
        return nil, fmt.Errorf("UTM is only available on macOS")
    }
    // ...
}
```

### Guest OS Support (Inside VMs)

The gallery supports multiple guest operating systems:

| OS Type | Architecture | Backend | Notes |
|---------|--------------|---------|-------|
| Windows | arm64 | QEMU | Windows 11 ARM via `windows-11-arm` |
| Linux | arm64/amd64 | QEMU/Apple HV | Ubuntu, Debian, Fedora |
| macOS | arm64 | QEMU | macOS VMs (requires Apple virtualization) |

## Core Components

### 1. Driver Pattern (driver.go)

**Purpose:** Version-specific UTM feature detection and compatibility.

```go
type Driver interface {
    Version() string
    SupportsExport() bool      // UTM 4.6+ feature
    SupportsImport() bool      // UTM 4.6+ feature
    SupportsGuestTools() bool  // UTM 4.6+ feature
    Export(vmName, outputPath string) error
    Import(utmPath string) (string, error)
    GuestToolsISOPath() (string, error)
    ExecuteOsaScript(command ...string) (string, error)
    Utmctl(args ...string) (string, error)
}
```

**Version Detection:**

```go
// UTM 4.5.x - no export/import
type driver45 struct { baseDriver }
func (d *driver45) SupportsExport() bool     { return false }
func (d *driver45) SupportsImport() bool     { return false }
func (d *driver45) SupportsGuestTools() bool { return false }

// UTM 4.6.x - full features
type driver46 struct { baseDriver }
func (d *driver46) SupportsExport() bool     { return true }
func (d *driver46) SupportsImport() bool     { return true }
func (d *driver46) SupportsGuestTools() bool { return true }
```

**Version Parsing:**

```go
type UTMVersion struct {
    Major int
    Minor int
    Patch int
    Raw   string
}

func (v *UTMVersion) AtLeast(major, minor int) bool {
    if v.Major > major { return true }
    if v.Major == major && v.Minor >= minor { return true }
    return false
}
```

### 2. AppleScript Automation (osascript.go)

**Purpose:** Execute embedded AppleScript files for VM operations.

```go
//go:embed scripts/*
var osascripts embed.FS

func ExecuteOsaScript(command ...string) (string, error) {
    // Read script from embedded FS
    scriptContent, _ := osascripts.ReadFile(filepath.Join("scripts", command[0]))

    // Execute via osascript - (reads from stdin)
    cmd := exec.Command("osascript", "-")
    cmd.Stdin = strings.NewReader(string(scriptContent))
    // ...
}
```

**Enum Mappings:**

AppleScript uses 4-character enum codes. The package provides mappings:

```go
// Storage Controller Codes
var ControllerEnumMap = map[string]string{
    "none":   "QdIn",
    "ide":    "QdIi",
    "scsi":   "QdIs",
    "sd":     "QdId",
    "virtio": "QdIv",  // Recommended for best performance
    "usb":    "QdIu",
    "nvme":   "QdIn",
}

// Network Mode Codes
var NetworkModeEnumMap = map[string]string{
    "shared":   "ShRd",  // NAT with host access
    "emulated": "EmUd",  // Isolated VLAN, supports port forwarding
    "bridged":  "BrDg",  // Direct network access
    "host":     "HsOn",  // Host only
}
```

### 3. VM Gallery (gallery.go)

**Purpose:** Pre-configured VM templates with ISO download info.

```go
//go:embed vm-gallery.json
var vmGalleryJSON []byte

type VMGallery struct {
    Meta GalleryMeta
    VMs  map[string]VMEntry
}

type VMEntry struct {
    Name        string
    Description string
    Arch        string  // arm64, amd64
    OS          string  // windows, linux, macos
    ISO         ISOConfig
    Template    TemplateConfig
    Tags        []string
}
```

**Filter Methods:**

```go
func (g *VMGallery) FilterByOS(os string) map[string]VMEntry
func (g *VMGallery) FilterByArch(arch string) map[string]VMEntry
func (g *VMGallery) FilterByTag(tag string) map[string]VMEntry
```

### 4. Installation System (install.go)

**Purpose:** Download and install UTM app and VM ISOs.

**Key Feature: Resumable Downloads**

```go
func downloadFile(url, destPath string) error {
    const maxRetries = 15
    const retryDelay = 3

    for attempt := 1; attempt <= maxRetries; attempt++ {
        // Check existing file size for resume
        var offset int64
        if fi, err := os.Stat(destPath); err == nil {
            offset = fi.Size()
        }

        req, _ := http.NewRequest("GET", url, nil)
        if offset > 0 {
            req.Header.Set("Range", fmt.Sprintf("bytes=%d-", offset))
        }

        resp, _ := client.Do(req)

        // Handle 206 Partial Content for resume
        // Handle 416 Range Not Satisfiable (file complete)
    }
}
```

**Idempotency via Cache:**

```go
// InstallUTM checks cache before downloading
func InstallUTM(force bool) error {
    if !force && IsUTMAppCached(utmApp.Version, utmApp.Checksum) {
        fmt.Printf("UTM v%s is already installed and cached at %s\n", utmApp.Version, appPath)
        return nil
    }
    // ...
}
```

### 5. VM Creation (create.go)

**Purpose:** Automated VM creation via AppleScript.

**5-Step Creation Process:**

```go
func createVMAutomated(vmKey string, vm *VMEntry, isoPath, shareDir string, diskSizeMB int, opts CreateVMOptions) error {
    // Step 1: Create VM with backend/arch
    ExecuteOsaScript("create_vm.applescript", "--name", vmName, "--backend", backend, "--arch", arch)

    // Step 2: Customize (CPU, RAM, UEFI)
    ExecuteOsaScript("customize_vm.applescript", vmID, "--cpus", cpu, "--memory", ram)

    // Step 3: Add disk drive
    ExecuteOsaScript("add_drive.applescript", vmID, "--interface", "QdIv", "--size", diskSizeMB)

    // Step 4: Attach ISO
    ExecuteOsaScript("attach_iso.applescript", vmID, "--interface", "QdIu", "--source", isoPath)

    // Step 5: Add network interface
    ExecuteOsaScript("add_network_interface.applescript", vmID, "ShRd")
}
```

**Rollback on Failure:**

```go
if _, err := ExecuteOsaScript(customizeCmd...); err != nil {
    DeleteVMFromUTM(vmName)  // Cleanup on failure
    return fmt.Errorf("failed to customize VM: %w", err)
}
```

### 6. Advanced Features (advanced.go)

**Port Forwarding:**

```go
type PortForward struct {
    Protocol     string  // "tcp" or "udp"
    GuestAddress string
    GuestPort    int
    HostAddress  string
    HostPort     int
}

func AddPortForward(vmName string, networkIndex int, rule PortForward) error {
    vmID, _ := GetVMUUID(vmName)
    protocolCode := ProtocolEnumMap[rule.Protocol]  // "tcp" -> "TcPp"

    ruleStr := fmt.Sprintf("%s,%s,%d,%s,%d",
        protocolCode, rule.GuestAddress, rule.GuestPort,
        rule.HostAddress, rule.HostPort)

    ExecuteOsaScript("add_port_forwards.applescript", vmID, "--index", networkIndex, ruleStr)
}
```

**SSH Port Forward Convenience Function:**

```go
func SetupSSHPortForward(vmName string, hostPort int) error {
    rule := PortForward{
        Protocol:     "tcp",
        GuestAddress: "",
        GuestPort:    22,
        HostAddress:  "127.0.0.1",
        HostPort:     hostPort,
    }
    return AddPortForward(vmName, 1, rule)  // Network index 1 = emulated VLAN
}
```

**VM Export/Import (UTM 4.6+):**

```go
func ExportVM(vmName, outputPath string) error {
    vmID, _ := GetVMUUID(vmName)
    cmd := exec.Command("osascript", "-e",
        fmt.Sprintf(`tell application "UTM" to export virtual machine id "%s" to POSIX file "%s"`, vmID, outputPath))
    // ...
}

func ImportVM(utmPath string) (string, error) {
    cmd := exec.Command("osascript", "-e",
        fmt.Sprintf(`tell application "UTM" to import new virtual machine from POSIX file "%s"`, utmPath))
    // Extract VM ID from output
}
```

### 7. Cache System (cache.go)

**Purpose:** Idempotent operations via checksum validation.

```go
func IsUTMAppCached(version, checksum string) bool {
    cache, _ := GetUTMCache()
    paths := GetPaths()

    sdk := &installer.SDK{
        Name:        MakeUTMAppCacheKey(version),
        Version:     version,
        Checksum:    checksum,
        InstallPath: paths.App,
    }

    // Check cache entry
    if !cache.IsCached(sdk) {
        return false
    }

    // Verify app actually exists
    utmctlPath := filepath.Join(paths.App, "Contents/MacOS/utmctl")
    if _, err := os.Stat(utmctlPath); os.IsNotExist(err) {
        return false
    }

    return true
}
```

### 8. Migration System (migrate.go)

**Purpose:** Migrate from legacy local paths to global shared paths.

**Path Evolution:**

| Version | App Path | ISO Path |
|---------|----------|----------|
| Legacy | `.bin/UTM.app` | `.data/utm/iso/` |
| Global | `~/utm-dev-sdks/utm/UTM.app` | `~/utm-dev-sdks/utm/iso/` |

**Migration Logic:**

```go
func MigrateUTMApp() (*MigrationResult, error) {
    legacy := LegacyPaths()
    paths := GetPaths()

    // Try rename first (fastest if same filesystem)
    if err := os.Rename(legacy.App, paths.App); err != nil {
        // Cross-device, need to copy
        copyDirRecursive(legacy.App, paths.App)
        os.RemoveAll(legacy.App)
    }
}
```

## utmctl Wrapper (utmctl.go)

**Purpose:** Direct wrapper around the `utmctl` CLI tool.

```go
func RunUTMCtl(args ...string) (string, error) {
    utmctl := GetUTMCtlPath()
    cmd := exec.Command(utmctl, args...)
    // ...
}

func RunUTMCtlInteractive(args ...string) error {
    utmctl := GetUTMCtlPath()
    cmd := exec.Command(utmctl, args...)
    cmd.Stdout = os.Stdout
    cmd.Stderr = os.Stderr
    cmd.Stdin = os.Stdin
    return cmd.Run()
}
```

**Supported Commands:**

| Command | Function | Description |
|---------|----------|-------------|
| `list` | `ListVMs()` | List all VMs |
| `status` | `GetVMStatus()` | Get VM status |
| `start` | `StartVM()` | Start VM |
| `stop` | `StopVM()` | Stop VM |
| `ip-address` | `GetVMIP()` | Get VM IP |
| `exec` | `ExecInVM()` | Execute command in VM |
| `clone` | `CloneVM()` | Clone VM |
| `delete` | `DeleteVM()` | Delete VM |
| `file push` | `PushFile()` | Push file to VM |
| `file pull` | `PullFile()` | Pull file from VM |

## Path Management (config.go)

**Global Path Structure:**

```go
type Paths struct {
    Root  string  // ~/utm-dev-sdks/utm
    App   string  // ~/utm-dev-sdks/utm/UTM.app
    VMs   string  // ~/utm-dev-sdks/utm/vms
    ISO   string  // ~/utm-dev-sdks/utm/iso
    Share string  // ~/utm-dev-sdks/utm/share
}

func DefaultPaths() Paths {
    sdkDir := config.GetSDKDir()  // ~/utm-dev-sdks
    return Paths{
        Root:  filepath.Join(sdkDir, "utm"),
        App:   filepath.Join(sdkDir, "utm", "UTM.app"),
        ISO:   filepath.Join(sdkDir, "utm", "iso"),
        VMs:   filepath.Join(sdkDir, "utm", "vms"),
        Share: filepath.Join(sdkDir, "utm", "share"),
    }
}
```

**utmctl Path Resolution:**

```go
func GetUTMCtlPath() string {
    locations := []string{
        // Global install (preferred)
        filepath.Join(paths.App, "Contents/MacOS/utmctl"),
        // Legacy local install
        filepath.Join(legacy.App, "Contents/MacOS/utmctl"),
        // Homebrew
        "/opt/homebrew/bin/utmctl",
        "/usr/local/bin/utmctl",
        // System install
        "/Applications/UTM.app/Contents/MacOS/utmctl",
    }

    for _, loc := range locations {
        if _, err := os.Stat(loc); err == nil {
            return loc
        }
    }

    return "utmctl"  // Fallback to PATH lookup
}
```

## Key Design Patterns

### 1. Idempotency via Checksums

All download/install operations are idempotent:

```go
// Check cache first (idempotent)
if !force && IsUTMAppCached(utmApp.Version, utmApp.Checksum) {
    fmt.Printf("UTM v%s is already installed and cached at %s\n", utmApp.Version, appPath)
    return nil
}

// Check if already installed (but not in cache - add to cache)
if _, err := os.Stat(utmctlPath); err == nil {
    fmt.Printf("UTM is already installed at %s\n", appPath)
    // Add to cache for future idempotency
    AddUTMAppToCache(utmApp.Version, utmApp.Checksum)
    return nil
}
```

### 2. Version-Specific Drivers

Features are gated by UTM version:

```go
driver, _ := GetDriver()
if driver.SupportsExport() {
    // UTM 4.6+ only
    driver.Export(vmName, outputPath)
}
```

### 3. Enum Code Abstraction

Human-readable names map to AppleScript enum codes:

```go
controllerCode, _ := GetControllerEnumCode("virtio")  // Returns "QdIv"
networkCode, _ := GetNetworkModeEnumCode("shared")    // Returns "ShRd"
```

### 4. Rollback on Failure

VM creation cleans up on errors:

```go
if _, err := ExecuteOsaScript(customizeCmd...); err != nil {
    DeleteVMFromUTM(vmName)  // Cleanup
    return fmt.Errorf("failed to customize VM: %w", err)
}
```

## Error Handling

### Version Errors

```go
func (d *driver45) Export(vmName, outputPath string) error {
    return fmt.Errorf("UTM %s does not support VM export. Please upgrade to UTM 4.6 or later", d.version.Raw)
}
```

### Platform Errors

```go
if runtime.GOOS != "darwin" {
    return fmt.Errorf("UTM is only available on macOS")
}
```

### Retry Logic

```go
const maxRetries = 15
for attempt := 1; attempt <= maxRetries; attempt++ {
    resp, err := client.Do(req)
    if err == nil && resp.StatusCode == http.StatusOK {
        break
    }
    if attempt < maxRetries {
        backoff := time.Duration(attempt) * time.Second
        time.Sleep(backoff)
    }
}
```

## Testing Considerations

The package includes test files:

- `installer_test.go` - Tests for installer package
- Self-tests in `cmd/self_test.go`

**Mocking AppleScript:**

Tests would need to mock `ExecuteOsaScript` calls or use a test driver:

```go
func ResetDriver() {
    cachedDriver = nil
}
```

## Security Considerations

### Checksum Verification

```go
// install.go:149-156
calculatedChecksum := hex.EncodeToString(hasher.Sum(nil))
expectedChecksum := strings.TrimPrefix(sdk.Checksum, "sha256:")

if expectedChecksum != "" && calculatedChecksum != expectedChecksum {
    return fmt.Errorf("checksum mismatch: expected %s, got %s", expectedChecksum, calculatedChecksum)
}
fmt.Println("✅ Checksum verified.")
```

### Path Traversal Prevention

```go
// ResolveInstallPath validates and resolves paths
func ResolveInstallPath(path string) (string, error) {
    expandedPath := os.ExpandEnv(path)

    if filepath.IsAbs(expandedPath) {
        return expandedPath, nil
    }

    // For relative paths starting with "sdks/", use OS-specific SDK directory
    if strings.HasPrefix(expandedPath, "sdks/") {
        return filepath.Join(config.GetSDKDir(), strings.TrimPrefix(expandedPath, "sdks/")), nil
    }
    // ...
}
```

## Performance Considerations

### Build Cache

The package uses SHA256-based build caching to avoid redundant downloads:

```go
func hashDirectory(path string) (string, error) {
    h := sha256.New()
    filepath.Walk(path, func(filePath string, info os.FileInfo, err error) error {
        // Skip .bin, .build, .dist, .git directories
        // Hash relative path + modtime + content (for files < 1MB)
    })
}
```

### Parallel Downloads

ISO downloads use progress bars and concurrent hashing:

```go
// Create a tee reader to write to the file and the hash simultaneously
hasher := sha256.New()
progressReader := progressbar.NewReader(resp.Body, hasher)
teeReader := io.TeeReader(&progressReader, hasher)
_, err = io.Copy(tmpFile, teeReader)
```

## Future Enhancements

1. **Windows/Linux Host Support** - Currently macOS-only due to UTM dependency
2. **Guest Agent Integration** - QEMU guest agent for exec/file transfer
3. **VM Snapshots** - UTM 4.6+ snapshot support
4. **Cloud Init** - Automated guest OS configuration
5. **Packer Integration** - Full packer-plugin-utm feature parity

## Related Files

- `cmd/utm.go` - CLI interface using this package
- `pkg/installer/` - SDK installation and caching
- `pkg/config/` - SDK path configuration
- `scripts/*.applescript` - Embedded AppleScript files
