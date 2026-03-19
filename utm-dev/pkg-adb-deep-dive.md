---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/adb/
explored_at: 2026-03-19T12:00:00Z
package: pkg/adb
---

# Deep Dive: ADB Wrapper (pkg/adb/)

## Overview

The `pkg/adb` package provides a Go wrapper around Android Debug Bridge (adb) and emulator commands. It resolves tool paths from utm-dev's managed SDK directory, enabling:

- Device management and detection
- APK installation and launching
- Log streaming
- Screenshot capture
- Emulator control

## Architecture

```
┌─────────────────────────────────────┐
│         cmd/android.go              │
│         cmd/run.go                  │
├─────────────────────────────────────┤
│          pkg/adb                    │
│  ┌─────────────┐  ┌─────────────┐   │
│  │   Client    │  │   Device    │   │
│  │  - ADBPath  │  │  - Serial   │   │
│  │  - Run      │  │  - State    │   │
│  │  - Install  │  │  - Model    │   │
│  │  - Launch   │  │             │   │
│  └─────────────┘  └─────────────┘   │
│  ┌─────────────┐  ┌─────────────┐   │
│  │   pkg/config│  │  emulator   │   │
│  │   SDK Dir   │  │  commands   │   │
│  └─────────────┘  └─────────────┘   │
└─────────────────────────────────────┘
```

## Client Type

### Construction

```go
type Client struct {
    sdkDir string  // SDK directory from pkg/config
}

func New() *Client {
    return &Client{sdkDir: config.GetSDKDir()}
}
```

### Path Resolution

```go
// ADBPath returns absolute path to adb binary
func (c *Client) ADBPath() string {
    name := "adb"
    if runtime.GOOS == "windows" {
        name = "adb.exe"
    }
    return filepath.Join(c.sdkDir, "platform-tools", name)
}

// EmulatorPath returns absolute path to emulator binary
func (c *Client) EmulatorPath() string {
    name := "emulator"
    if runtime.GOOS == "windows" {
        name = "emulator.exe"
    }
    return filepath.Join(c.sdkDir, "emulator", name)
}

// Available returns true if adb is installed
func (c *Client) Available() bool {
    _, err := os.Stat(c.ADBPath())
    return err == nil
}

// EmulatorAvailable returns true if emulator is installed
func (c *Client) EmulatorAvailable() bool {
    _, err := os.Stat(c.EmulatorPath())
    return err == nil
}
```

## Command Execution

### run() - Standard Output Capture

```go
func (c *Client) run(args ...string) (string, error) {
    cmd := exec.Command(c.ADBPath(), args...)
    var out bytes.Buffer
    cmd.Stdout = &out
    cmd.Stderr = &out

    if err := cmd.Run(); err != nil {
        return out.String(), fmt.Errorf("adb %s: %w\n%s",
            strings.Join(args, " "), err, out.String())
    }

    return strings.TrimSpace(out.String()), nil
}
```

### runPassthrough() - Terminal Output

```go
func (c *Client) runPassthrough(args ...string) error {
    cmd := exec.Command(c.ADBPath(), args...)
    cmd.Stdout = os.Stdout
    cmd.Stderr = os.Stderr
    return cmd.Run()
}
```

**When to use each:**
- `run()` - When you need command output (e.g., `devices`, `shell` commands)
- `runPassthrough()` - For interactive operations (e.g., `install`, `logcat`)

## Device Management

### Device Type

```go
type Device struct {
    Serial string  // Device serial (e.g., "emulator-5554")
    State  string  // "device", "offline", "unauthorized"
    Model  string  // Device model name
}
```

### Devices() - List Connected Devices

```go
func (c *Client) Devices() ([]Device, error) {
    out, err := c.run("devices", "-l")  // -l for detailed output
    if err != nil {
        return nil, err
    }

    var devices []Device
    for _, line := range strings.Split(out, "\n") {
        line = strings.TrimSpace(line)
        if line == "" || strings.HasPrefix(line, "List of") {
            continue
        }

        parts := strings.Fields(line)
        if len(parts) < 2 {
            continue
        }

        d := Device{
            Serial: parts[0],
            State:  parts[1],
        }

        // Parse additional attributes (model:, product:, etc.)
        for _, p := range parts[2:] {
            if strings.HasPrefix(p, "model:") {
                d.Model = strings.TrimPrefix(p, "model:")
            }
        }

        devices = append(devices, d)
    }

    return devices, nil
}
```

### HasDevice() - Check for Connected Device

```go
func (c *Client) HasDevice() bool {
    devices, err := c.Devices()
    if err != nil {
        return false
    }

    for _, d := range devices {
        if d.State == "device" {  // Only "device" state is usable
            return true
        }
    }
    return false
}
```

### WaitForDevice() - Block Until Device Ready

```go
func (c *Client) WaitForDevice() error {
    _, err := c.run("wait-for-device")
    return err
}
```

## APK Management

### Install() - Install APK

```go
func (c *Client) Install(apkPath string) error {
    // -r = replace existing installation
    return c.runPassthrough("install", "-r", apkPath)
}
```

**Output:**
```
Performing Streamed Install
Installing APK
Success
```

### Uninstall() - Remove App

```go
func (c *Client) Uninstall(pkg string) error {
    return c.runPassthrough("uninstall", pkg)
}
```

### Launch() - Start App

```go
func (c *Client) Launch(pkg string) error {
    // Uses monkey tool to launch launcher activity
    return c.runPassthrough(
        "shell", "monkey",
        "-p", pkg,  // Package name
        "-c", "android.intent.category.LAUNCHER",
        "1",  // Single event
    )
}
```

**Why monkey?** Gio apps use "localhost.<appname>" as package name, and monkey reliably finds the launcher activity.

### ForceStop() - Stop Running App

```go
func (c *Client) ForceStop(pkg string) error {
    _, err := c.run("shell", "am", "force-stop", pkg)
    return err
}
```

## Debugging Tools

### Logcat() - Stream Logs

```go
func (c *Client) Logcat(tags ...string) error {
    args := []string{"logcat", "-v", "time"}  // -v time for timestamps

    if len(tags) > 0 {
        // Filter to specific tags
        args = append(args, "*:S")  // Silence all by default
        for _, tag := range tags {
            args = append(args, tag+":V")  // Verbose for specified tags
        }
    }

    cmd := exec.Command(c.ADBPath(), args...)
    cmd.Stdout = os.Stdout
    cmd.Stderr = os.Stderr
    return cmd.Run()  // Blocks until Ctrl+C
}
```

**Usage:**
```go
// Stream all logs
client.Logcat()

// Filter to Gio logs only
client.Logcat("GoLog:V", "GioView:V")
```

### Screenshot() - Capture Screen

```go
func (c *Client) Screenshot(outputPath string) error {
    // exec-out runs command on device and streams output
    cmd := exec.Command(c.ADBPath(), "exec-out", "screencap", "-p")

    f, err := os.Create(outputPath)
    if err != nil {
        return fmt.Errorf("create output file: %w", err)
    }
    defer f.Close()

    cmd.Stdout = f
    cmd.Stderr = os.Stderr

    if err := cmd.Run(); err != nil {
        os.Remove(outputPath)  // Clean up partial file
        return fmt.Errorf("screencap: %w", err)
    }

    return nil
}
```

### WebViewVersion() - Get WebView Info

```go
func (c *Client) WebViewVersion() (string, error) {
    out, err := c.run("shell", "dumpsys", "webviewupdate")
    if err != nil {
        return "", err
    }

    for _, line := range strings.Split(out, "\n") {
        line = strings.TrimSpace(line)
        if strings.HasPrefix(line, "Current WebView package") {
            return line, nil
        }
        if strings.Contains(line, "versionName") {
            return line, nil
        }
    }

    return "unknown", nil
}
```

## Emulator Control

### EmulatorList() - List Available AVDs

```go
func (c *Client) EmulatorList() ([]string, error) {
    cmd := exec.Command(c.EmulatorPath(), "-list-avds")
    var out bytes.Buffer
    cmd.Stdout = &out
    cmd.Stderr = os.Stderr

    if err := cmd.Run(); err != nil {
        return nil, fmt.Errorf("emulator -list-avds: %w", err)
    }

    var avds []string
    for _, line := range strings.Split(out.String(), "\n") {
        line = strings.TrimSpace(line)
        if line != "" {
            avds = append(avds, line)
        }
    }

    return avds, nil
}
```

**Output:**
```
Pixel_4_API_34
Pixel_7_Pro_API_33
```

### EmulatorStart() - Launch Emulator

```go
func (c *Client) EmulatorStart(avdName string) (int, error) {
    // -no-snapshot-load = cold boot (faster for testing)
    cmd := exec.Command(c.EmulatorPath(), "-avd", avdName, "-no-snapshot-load")
    cmd.Stdout = os.Stdout
    cmd.Stderr = os.Stderr

    if err := cmd.Start(); err != nil {
        return 0, fmt.Errorf("start emulator: %w", err)
    }

    return cmd.Process.Pid, nil
}
```

**Returns:** PID of emulator process (for later termination if needed)

## Usage Patterns

### In cmd/run.go - Android App Launch

```go
func launchAndroidApp(apkPath, appName string) error {
    client := adb.New()

    if !client.Available() {
        return fmt.Errorf("adb not found at %s", client.ADBPath())
    }

    if !client.HasDevice() {
        return fmt.Errorf("no Android device connected")
    }

    // Install APK
    fmt.Printf("Installing %s...\n", apkPath)
    if err := client.Install(apkPath); err != nil {
        return fmt.Errorf("install failed: %w", err)
    }

    // Launch app - gogio uses "localhost.<appname>" as package
    pkg := "localhost." + appName
    fmt.Printf("Launching %s...\n", pkg)
    if err := client.Launch(pkg); err != nil {
        return fmt.Errorf("launch failed: %w", err)
    }

    fmt.Printf("✓ App running on device\n")
    return nil
}
```

### In cmd/android.go - Emulator Commands

```go
var androidEmulatorStartCmd = &cobra.Command{
    Use: "start <avd-name>",
    RunE: func(cmd *cobra.Command, args []string) error {
        client := adb.New()

        if !client.EmulatorAvailable() {
            return fmt.Errorf("emulator not found")
        }

        pid, err := client.EmulatorStart(args[0])
        if err != nil {
            return err
        }

        fmt.Printf("Emulator started with PID: %d\n", pid)
        return nil
    },
}
```

## Design Decisions

### 1. SDK Path Resolution

**Why:** utm-dev manages its own SDK installation, separate from system PATH.

**Benefit:** Consistent behavior regardless of system configuration.

### 2. Two Execution Modes

**Why:** Some commands need output capture, others need interactive terminal.

### 3. Error Message Enhancement

```go
return fmt.Errorf("adb %s: %w\n%s",
    strings.Join(args, " "), err, out.String())
```

**Why:** ADB errors often have useful output; include it in error message.

### 4. Passthrough for Install/Launch

**Why:** These commands have progress indicators and interactive prompts.

## Environment Variables

ADB respects these environment variables:

| Variable | Purpose |
|----------|---------|
| `ANDROID_SDK_ROOT` | SDK location (used by pkg/config) |
| `ADB_TRACE` | Enable ADB debugging |
| `ANDROID_SERIAL` | Default device serial |

## Testing

No dedicated test file, but used in:
- `cmd/install_test.go` - SDK installation testing
- Manual testing with real devices/emulators

## Future Enhancements

1. **Reverse Tunneling:** `adb reverse` for host service access
2. **Port Forwarding:** `adb forward` for device service access
3. **Multiple Device Support:** Target specific device by serial
4. **File Sync:** `adb push`/`adb pull` wrappers
5. **Screen Recording:** `adb shell screenrecord` wrapper
