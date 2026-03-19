---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/utm/
explored_at: 2026-03-19T12:00:00Z
package: pkg/utm
---

# Deep Dive: UTM Integration (pkg/utm/) - Part 2

## Additional Files Not Covered in Main Deep Dive

### utmctl.go - utmctl Command Wrapper

```go
// RunUTMCtl executes a utmctl command
func RunUTMCtl(args ...string) (string, error) {
    utmctlPath := GetUTMCtlPath()  // Usually /usr/local/bin/utmctl

    cmd := exec.Command(utmctlPath, args...)
    output, err := cmd.CombinedOutput()

    return string(output), err
}

// GetUTMCtlPath finds utmctl binary
func GetUTMCtlPath() string {
    if path, err := exec.LookPath("utmctl"); err == nil {
        return path
    }
    return "/usr/local/bin/utmctl"  // Default location
}
```

**Commands Wrapped:**
- `utmctl list` - List VMs
- `utmctl status <vm>` - VM status
- `utmctl start <vm>` - Start VM
- `utmctl stop <vm>` - Stop VM
- `utmctl exec <vm> <cmd>` - Execute in VM
- `utmctl push <vm> <src> <dst>` - Push file
- `utmctl pull <vm> <src> <dst>` - Pull file
- `utmctl export <vm> <out>` - Export VM
- `utmctl import <utm>` - Import VM

### osascript.go - AppleScript Execution

```go
// ExecuteOsaScript runs an AppleScript command
func ExecuteOsaScript(command ...string) (string, error) {
    cmd := exec.Command("osascript", command...)
    output, err := cmd.CombinedOutput()

    return string(output), err
}

// LaunchUTM opens UTM.app if not running
func LaunchUTM() error {
    cmd := exec.Command("open", "-a", "UTM")
    return cmd.Run()
}

// GetUTMVersion queries UTM version via AppleScript
func GetUTMVersion() (string, error) {
    script := `tell application "UTM" to get version`
    output, err := ExecuteOsaScript("-e", script)
    return strings.TrimSpace(output), err
}
```

### gallery.go - VM Gallery Management

```go
type Gallery struct {
    VMs       map[string]VMEntry  `json:"vms"`
    Version   string              `json:"version"`
}

type VMEntry struct {
    Name        string     `json:"name"`
    OS          string     `json:"os"`
    Arch        string     `json:"arch"`
    Description string     `json:"description"`
    Template    VMTemplate `json:"template"`
    ISO         ISOInfo    `json:"iso"`
}

type VMTemplate struct {
    RAM  int `json:"ram"`   // MB
    Disk int `json:"disk"`  // MB
    CPU  int `json:"cpu"`   // Cores
}

type ISOInfo struct {
    URL      string `json:"url"`
    Filename string `json:"filename"`
    Size     int64  `json:"size"`  // bytes
}

// GetVM returns a VM entry by key
func (g *Gallery) GetVM(key string) (*VMEntry, bool) {
    vm, ok := g.VMs[key]
    return &vm, ok
}

// FilterByOS filters VMs by operating system
func (g *Gallery) FilterByOS(os string) map[string]VMEntry {
    filtered := make(map[string]VMEntry)
    for key, vm := range g.VMs {
        if strings.Contains(strings.ToLower(vm.OS), strings.ToLower(os)) {
            filtered[key] = vm
        }
    }
    return filtered
}
```

### cache.go - ISO Download Cache

```go
type ISOCache struct {
    path        string
    downloads   map[string]DownloadInfo
}

type DownloadInfo struct {
    URL       string    `json:"url"`
    Checksum  string    `json:"checksum"`
    Timestamp time.Time `json:"timestamp"`
    Size      int64     `json:"size"`
}

// IsDownloaded checks if ISO is already downloaded
func (c *ISOCache) IsDownloaded(url string) bool {
    info, ok := c.downloads[url]
    if !ok {
        return false
    }

    // Verify file exists
    filename := filepath.Base(url)
    isoPath := filepath.Join(GetPaths().ISO, filename)
    _, err := os.Stat(isoPath)

    return err == nil
}

// RecordDownload records a successful ISO download
func (c *ISOCache) RecordDownload(url, checksum string, size int64) {
    c.downloads[url] = DownloadInfo{
        URL:       url,
        Checksum:  checksum,
        Timestamp: time.Now(),
        Size:      size,
    }
    c.Save()
}
```

### migrate.go - Migration from Local to Global Paths

```go
// MigrateAll migrates UTM files from local to global locations
func MigrateAll() error {
    // Old paths (local to repo)
    oldAppPath := ".bin/UTM.app"
    oldISOPath := ".data/utm/iso/"

    // New paths (global SDK location)
    newAppPath := filepath.Join(GetSDKDir(), "utm", "UTM.app")
    newISOPath := filepath.Join(GetSDKDir(), "utm", "iso/")

    // Migrate UTM.app
    if _, err := os.Stat(oldAppPath); err == nil {
        fmt.Println("Migrating UTM.app to global location...")
        os.MkdirAll(filepath.Dir(newAppPath), 0755)
        os.Rename(oldAppPath, newAppPath)
    }

    // Migrate ISOs
    entries, _ := os.ReadDir(oldISOPath)
    for _, entry := range entries {
        oldFile := filepath.Join(oldISOPath, entry.Name())
        newFile := filepath.Join(newISOPath, entry.Name())

        os.MkdirAll(filepath.Dir(newFile), 0755)
        os.Rename(oldFile, newFile)
    }

    return nil
}
```

### advanced.go - Advanced VM Operations

```go
// GetVMIP gets the IP address of a running VM
func GetVMIP(vmName string) (string, error) {
    // Uses QEMU guest agent to query network interfaces
    cmd := `powershell -Command "(Get-NetIPAddress -AddressFamily IPv4 | Where-Object InterfaceAlias -notlike '*Loopback*' | Select-Object -First 1).IPAddress"`

    output, err := RunUTMCtl("exec", vmName, cmd)
    if err != nil {
        return "", err
    }

    return strings.TrimSpace(output), nil
}

// ExportVM exports a VM to a .utm file (UTM 4.6+)
func ExportVM(vmName, outputPath string) error {
    driver, err := GetDriver()
    if err != nil {
        return err
    }

    if !driver.SupportsExport() {
        return fmt.Errorf("UTM %s does not support export", driver.Version())
    }

    _, err = driver.Utmctl("export", vmName, outputPath)
    return err
}

// ImportVM imports a VM from a .utm file (UTM 4.6+)
func ImportVM(utmPath string) (string, error) {
    driver, err := GetDriver()
    if err != nil {
        return "", err
    }

    if !driver.SupportsImport() {
        return "", fmt.Errorf("UTM %s does not support import", driver.Version())
    }

    vmID, err := driver.Utmctl("import", utmPath)
    return strings.TrimSpace(vmID), err
}
```

### config.go - UTM Configuration

```go
// Paths contains UTM path configuration
type Paths struct {
    App   string  // UTM.app location
    VMs   string  // VM storage directory
    ISO   string  // ISO storage directory
    Share string  // Shared folder directory
}

// GetPaths returns OS-specific UTM paths
func GetPaths() Paths {
    home, _ := os.UserHomeDir()

    return Paths{
        App:   "/Applications/UTM.app",
        VMs:   filepath.Join(home, "utm-dev", "vms"),
        ISO:   filepath.Join(home, "utm-dev", "iso"),
        Share: filepath.Join(home, "utm-dev", "share"),
    }
}

// IsUTMInstalled checks if UTM.app exists
func IsUTMInstalled() bool {
    _, err := os.Stat("/Applications/UTM.app")
    return err == nil
}
```

### install.go - UTM Installation

```go
// InstallUTM downloads and installs UTM.app
func InstallUTM(force bool) error {
    if IsUTMInstalled() && !force {
        fmt.Println("UTM is already installed")
        return nil
    }

    // Get latest release URL
    releaseURL := "https://github.com/utmapp/UTM/releases/latest/download/UTM.dmg"

    // Download DMG
    tmpFile, err := downloadWithProgress(releaseURL)
    if err != nil {
        return err
    }
    defer os.Remove(tmpFile)

    // Mount DMG
    mountOutput, err := exec.Command("hdiutil", "attach", tmpFile).CombinedOutput()
    if err != nil {
        return err
    }
    mountPoint := extractMountPoint(string(mountOutput))
    defer exec.Command("hdiutil", "detach", mountPoint).Run()

    // Copy to Applications
    fmt.Println("Copying UTM.app to /Applications...")
    cpCmd := exec.Command("cp", "-R",
        filepath.Join(mountPoint, "UTM.app"),
        "/Applications/")
    if err := cpCmd.Run(); err != nil {
        return err
    }

    fmt.Println("✓ UTM installed successfully")
    return nil
}

// UninstallUTM removes UTM.app
func UninstallUTM() error {
    if !IsUTMInstalled() {
        fmt.Println("UTM is not installed")
        return nil
    }

    // Remove from Applications
    os.RemoveAll("/Applications/UTM.app")

    fmt.Println("✓ UTM uninstalled")
    return nil
}
```

## File Summary

| File | Purpose | Lines |
|------|---------|-------|
| `driver.go` | Version-specific driver pattern | ~200 |
| `utmctl.go` | utmctl CLI wrapper | ~50 |
| `osascript.go` | AppleScript execution | ~100 |
| `create.go` | VM creation automation | ~300 |
| `advanced.go` | Port forwarding, export/import | ~200 |
| `gallery.go` | VM gallery/templates | ~150 |
| `config.go` | UTM configuration | ~100 |
| `install.go` | UTM installation | ~150 |
| `cache.go` | ISO download cache | ~100 |
| `migrate.go` | Local to global migration | ~80 |

## Integration with cmd/utm.go

All pkg/utm functions are called from `cmd/utm.go` which provides the CLI interface:

```go
// cmd/utm.go calls pkg/utm functions
var utmCreateCmd = &cobra.Command{
    Use: "create <vm-key>",
    RunE: func(cmd *cobra.Command, args []string) error {
        opts := utm.CreateVMOptions{
            Force:   force,
            Manual:  manual,
            Verbose: verbose,
        }
        return utm.CreateVM(args[0], opts)  // Calls pkg/utm.CreateVM
    },
}
```
