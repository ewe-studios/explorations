---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/simctl/
explored_at: 2026-03-19T12:00:00Z
package: pkg/simctl
---

# Deep Dive: simctl Wrapper (pkg/simctl/)

## Overview

The `pkg/simctl` package provides a Go wrapper around Xcode's `simctl` tool for iOS simulator management. It mirrors the design of `pkg/adb` for Android, providing:

- Simulator device listing and management
- App installation and launching
- Screenshot capture
- System log streaming
- Device boot/shutdown control

## Architecture

```
┌─────────────────────────────────────┐
│         cmd/ios.go                  │
│         cmd/run.go                  │
├─────────────────────────────────────┤
│         pkg/simctl                  │
│  ┌─────────────┐  ┌─────────────┐   │
│  │   Client    │  │   Device    │   │
│  │  - Available│  │  - UDID     │   │
│  │  - Boot     │  │  - Name     │   │
│  │  - Install  │  │  - State    │   │
│  │  - Launch   │  │  - Runtime  │   │
│  └─────────────┘  └─────────────┘   │
│  ┌─────────────┐  ┌─────────────┐   │
│  │  xcrun      │  │  Simulator  │   │
│  │  simctl     │  │  .app       │   │
│  └─────────────┘  └─────────────┘   │
└─────────────────────────────────────┘
```

## Client Type

### Construction

```go
type Client struct{}

func New() *Client {
    return &Client{}
}

// Available returns true if xcrun simctl is available
func (c *Client) Available() bool {
    cmd := exec.Command("xcrun", "simctl", "help")
    return cmd.Run() == nil
}
```

**Note:** Unlike ADB, simctl doesn't need a path - it's accessed via `xcrun` which is always in PATH when Xcode is installed.

## Device Type

### Structure

```go
type Device struct {
    UDID    string  // Unique Device Identifier
    Name    string  // Human-readable name (e.g., "iPhone 15")
    State   string  // "Booted", "Shutdown"
    Runtime string  // iOS version (e.g., "iOS 17.5")
}
```

### JSON Parsing

simctl outputs JSON for programmatic access:

```go
type simctlDeviceJSON struct {
    UDID       string `json:"udid"`
    Name       string `json:"name"`
    State      string `json:"state"`
    IsAvailable bool  `json:"isAvailable"`
    DeviceTypeIdentifier string `json:"deviceTypeIdentifier"`
}

type simctlListJSON struct {
    Devices map[string][]simctlDeviceJSON `json:"devices"`
}
```

## Device Listing

### Devices() - List All Simulators

```go
func (c *Client) Devices() ([]Device, error) {
    // -j = JSON output
    out, err := c.run("list", "devices", "-j")
    if err != nil {
        return nil, err
    }

    var result simctlListJSON
    if err := json.Unmarshal([]byte(out), &result); err != nil {
        return nil, fmt.Errorf("parse simctl output: %w", err)
    }

    var devices []Device
    for runtime, devs := range result.Devices {
        runtimeName := parseRuntimeName(runtime)

        for _, d := range devs {
            if !d.IsAvailable {
                continue  // Skip unavailable devices
            }

            devices = append(devices, Device{
                UDID:    d.UDID,
                Name:    d.Name,
                State:   d.State,
                Runtime: runtimeName,
            })
        }
    }

    return devices, nil
}
```

### parseRuntimeName()

Converts runtime keys to human-readable names:

```go
// Input: "com.apple.CoreSimulator.SimRuntime.iOS-17-5"
// Output: "iOS 17.5"
func parseRuntimeName(key string) string {
    parts := strings.Split(key, ".")
    if len(parts) > 0 {
        last := parts[len(parts)-1]
        // "iOS-17-5" -> "iOS 17.5"
        components := strings.SplitN(last, "-", 2)
        if len(components) == 2 {
            version := strings.ReplaceAll(components[1], "-", ".")
            return components[0] + " " + version
        }
        return last
    }
    return key
}
```

### BootedDevices() - Filter to Running Simulators

```go
func (c *Client) BootedDevices() ([]Device, error) {
    devices, err := c.Devices()
    if err != nil {
        return nil, err
    }

    var booted []Device
    for _, d := range devices {
        if d.State == "Booted" {
            booted = append(booted, d)
        }
    }

    return booted, nil
}
```

### HasBooted() - Check for Any Running Simulator

```go
func (c *Client) HasBooted() bool {
    devices, err := c.BootedDevices()
    if err != nil {
        return false
    }
    return len(devices) > 0
}
```

## Device Control

### Boot() - Start Simulator

```go
func (c *Client) Boot(udid string) error {
    _, err := c.run("boot", udid)

    // Ignore "already booted" error
    if err != nil && strings.Contains(err.Error(), "current state: Booted") {
        return nil
    }

    return err
}
```

### Shutdown() - Stop Simulator

```go
func (c *Client) Shutdown(udid string) error {
    _, err := c.run("shutdown", udid)
    return err
}
```

### OpenSimulatorApp() - Launch Simulator GUI

```go
func (c *Client) OpenSimulatorApp() error {
    cmd := exec.Command("open", "-a", "Simulator")
    return cmd.Run()
}
```

**Usage:** When no simulator is booted, opening the app boots the default device.

## App Management

### Install() - Install App on Simulator

```go
func (c *Client) Install(appPath string) error {
    return c.runPassthrough("install", "booted", appPath)
}
```

**Note:** Requires a booted simulator. Uses "booted" as device specifier.

### Uninstall() - Remove App

```go
func (c *Client) Uninstall(bundleID string) error {
    return c.runPassthrough("uninstall", "booted", bundleID)
}
```

### Launch() - Start App

```go
func (c *Client) Launch(bundleID string) error {
    return c.runPassthrough("launch", "booted", bundleID)
}
```

**Gio Convention:** Gio apps use "localhost.<appname>" as bundle ID.

### Terminate() - Stop App

```go
func (c *Client) Terminate(bundleID string) error {
    _, err := c.run("terminate", "booted", bundleID)
    return err
}
```

## Screenshot

### Screenshot() - Capture Simulator Screen

```go
func (c *Client) Screenshot(outputPath string) error {
    return c.runPassthrough("io", "booted", "screenshot", outputPath)
}
```

### StatusBarOverride() - Clean Status Bar for Screenshots

```go
func (c *Client) StatusBarOverride() error {
    // iOS 13+ feature for consistent demo screenshots
    _, err := c.run("status_bar", "booted", "override",
        "--time", "9:41",       // Apple keynote time
        "--batteryState", "charged",
        "--batteryLevel", "100",
        "--wifiBars", "3",      // Full WiFi
        "--cellularBars", "4",  // Full signal
    )
    return err
}
```

### StatusBarClear() - Remove Status Bar Override

```go
func (c *Client) StatusBarClear() error {
    _, err := c.run("status_bar", "booted", "clear")
    return err
}
```

## Debugging Tools

### Logs() - Stream System Log

```go
func (c *Client) Logs(predicate string) error {
    // xcrun simctl spawn booted log stream --level info
    args := []string{"simctl", "spawn", "booted", "log", "stream", "--level", "info"}

    if predicate != "" {
        // Filter with NSPredicate
        args = append(args, "--predicate", predicate)
    }

    cmd := exec.Command("xcrun", args...)
    cmd.Stdout = os.Stdout
    cmd.Stderr = os.Stderr
    return cmd.Run()  // Blocks until Ctrl+C
}
```

**Usage:**
```go
// All logs
client.Logs("")

// Gio-specific logs
client.Logs("processImagePath contains 'localhost'")
```

### GetAppContainer() - Get App Data Directory

```go
func (c *Client) GetAppContainer(bundleID, containerType string) (string, error) {
    if containerType == "" {
        containerType = "app"
    }
    return c.run("get_app_container", "booted", bundleID, containerType)
}
```

**Container Types:**
- `"app"` - App bundle
- `"data"` - App data
- `"groups"` - App group container

## Simulator Discovery

### ListDeviceTypes() - Available Device Models

```go
func (c *Client) ListDeviceTypes() (string, error) {
    return c.run("list", "devicetypes")
}
```

**Output:**
```
iPhone 15 (com.apple.CoreSimulator.SimDeviceType.iPhone-15)
iPhone 15 Pro (com.apple.CoreSimulator.SimDeviceType.iPhone-15-Pro)
iPad Pro 12.9-inch (6th generation) (...)
```

### ListRuntimes() - Available iOS Versions

```go
func (c *Client) ListRuntimes() (string, error) {
    return c.run("list", "runtimes")
}
```

**Output:**
```
iOS 17.5 (17.5 - 21F79) - com.apple.CoreSimulator.SimRuntime.iOS-17-5
iOS 16.4 (16.4 - 20E238) - com.apple.CoreSimulator.SimRuntime.iOS-16-4
```

## Command Execution

### run() - Capture Output

```go
func (c *Client) run(args ...string) (string, error) {
    // Prepend "simctl" to args
    fullArgs := append([]string{"simctl"}, args...)

    cmd := exec.Command("xcrun", fullArgs...)
    var out bytes.Buffer
    cmd.Stdout = &out
    cmd.Stderr = &out

    if err := cmd.Run(); err != nil {
        return out.String(), fmt.Errorf("xcrun simctl %s: %w\n%s",
            strings.Join(args, " "), err, out.String())
    }

    return strings.TrimSpace(out.String()), nil
}
```

### runPassthrough() - Terminal Output

```go
func (c *Client) runPassthrough(args ...string) error {
    fullArgs := append([]string{"simctl"}, args...)
    cmd := exec.Command("xcrun", fullArgs...)
    cmd.Stdout = os.Stdout
    cmd.Stderr = os.Stderr
    return cmd.Run()
}
```

## Usage Patterns

### In cmd/run.go - iOS Simulator Launch

```go
func launchIOSSimulator(appPath, appName string) error {
    client := simctl.New()

    if !client.Available() {
        return fmt.Errorf("xcrun simctl not found")
    }

    // Ensure simulator is booted
    if !client.HasBooted() {
        fmt.Println("No simulator booted, opening Simulator.app...")
        if err := client.OpenSimulatorApp(); err != nil {
            return fmt.Errorf("could not open Simulator app: %w", err)
        }
        fmt.Println("Waiting for simulator to boot...")
        // Could add polling here
    }

    // Install app
    fmt.Printf("Installing %s...\n", appPath)
    if err := client.Install(appPath); err != nil {
        return fmt.Errorf("install failed: %w", err)
    }

    // Launch app - gogio uses "localhost.<appname>" as bundle ID
    bundleID := "localhost." + appName
    fmt.Printf("Launching %s...\n", bundleID)
    if err := client.Launch(bundleID); err != nil {
        return fmt.Errorf("launch failed: %w", err)
    }

    fmt.Printf("✓ App running on simulator\n")
    return nil
}
```

### In cmd/ios.go - Simulator Management

```go
var iosBootCmd = &cobra.Command{
    Use: "boot <device-name>",
    RunE: func(cmd *cobra.Command, args []string) error {
        client := simctl.New()

        // Find device by name
        devices, _ := client.Devices()
        var target *Device
        for _, d := range devices {
            if strings.Contains(d.Name, args[0]) {
                target = &d
                break
            }
        }

        if target == nil {
            return fmt.Errorf("device not found: %s", args[0])
        }

        return client.Boot(target.UDID)
    },
}
```

## Design Decisions

### 1. "booted" Device Specifier

**Why:** simctl supports `--device` flag but "booted" is simpler when any running simulator works.

### 2. JSON Parsing

**Why:** simctl's text output is fragile; JSON is stable and parseable.

### 3. Runtime Name Parsing

**Why:** Raw runtime keys are verbose; parse to human-readable form.

### 4. Status Bar Override

**Why:** Consistent screenshots for demos/marketing materials.

## Comparison with pkg/adb

| Feature | pkg/adb | pkg/simctl |
|---------|---------|------------|
| Path resolution | SDK directory | xcrun (always in PATH) |
| Device listing | `adb devices -l` | `simctl list devices -j` |
| Install | `adb install -r` | `simctl install booted` |
| Launch | `monkey -p <pkg>` | `simctl launch booted <bundle>` |
| Logs | `logcat` | `log stream` |
| Screenshot | `exec-out screencap` | `io booted screenshot` |

## Testing

No dedicated test file, but used in:
- Manual testing with Xcode simulators
- Integration tests in `cmd/ios.go`

## Future Enhancements

1. **Device Creation:** `simctl create` for custom device types
2. **Runtime Download:** `simctl runtime download` automation
3. **App Data Management:** Push/pull app containers
4. **Permission Control:** `simctl grant` for runtime permissions
5. **Pasteboard Access:** Copy/paste to simulator clipboard
