---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/cmd/
explored_at: 2026-03-19T12:00:00Z
package: cmd
---

# Deep Dive: CLI Commands (cmd/)

## Overview

The `cmd/` package contains all CLI command definitions for utm-dev, using the [Cobra](https://github.com/spf13/cobra) framework. This layer is responsible for:

- Parsing user input and flags
- Validating arguments
- Orchestrating calls to `pkg/*` packages
- Formatting output and error messages

## Architecture

### Command Hierarchy

```
utm-dev (root)
├── build [platform] [app]      # Build commands
├── run [platform] [app]        # Build and run
├── bundle [app]                # Create app bundles
├── package [app]               # Create distribution archives
├── icons [app]                 # Generate icons
├── install [sdk]               # SDK installation
├── android                     # Android-specific commands
│   ├── emulator
│   └── ...
├── ios                         # iOS-specific commands
│   ├── boot
│   └── ...
├── utm                         # UTM VM management
│   ├── list, gallery, create
│   ├── start, stop, status
│   ├── exec, task, run, build
│   ├── push, pull, port-forward
│   ├── export, import
│   └── screenshot
├── workspace                   # Go workspace management
├── config                      # Configuration display
├── docs                        # Generate documentation
├── self                        # Self-management
│   ├── doctor
│   ├── setup
│   ├── test
│   └── cleanup
└── ...
```

### Command Groups

Commands are organized into groups for better help organization:

| Group ID | Title | Commands |
|----------|-------|----------|
| `build` | Build Commands | build, run, bundle, package |
| `sdk` | SDK Management | install, list, android, ios |
| `tools` | Development Tools | icons, workspace, config, docs |
| `vm` | Virtual Machines | utm (all subcommands) |
| `self` | Self Management | self (doctor, setup, test, cleanup) |

## Entry Point

### main.go

```go
func main() {
    cmd.Execute()
}
```

Simple entry point that delegates to `cmd.Execute()`.

### root.go

The root command defines:
- Command name: `utm-dev`
- Short/Long descriptions
- Version information
- Shell completion configuration
- Typo suggestions (`SuggestionsMinimumDistance = 2`)

Key helper functions:
- `getPlatformCompletion()` - Platform tab completion
- `getExampleCompletion()` - Example directory completion
- `getVMNameCompletion()` - VM name completion

## Core Commands

### build.go - Build Command (20KB+)

**Purpose:** Cross-platform build orchestration for Gio applications.

**Platforms:** macos, android, ios, ios-simulator, windows, linux, all

**Key Features:**
- SHA256-based build caching (via `pkg/buildcache`)
- Automatic icon generation (via `pkg/icons`)
- Platform-specific build configurations
- Deep linking support via `--schemes` flag
- Android app queries via `--queries` flag
- Code signing support via `--signkey` flag

**Build Flow:**
```
1. Validate platform
2. Create GioProject from app directory
3. Check build cache (NeedsRebuild)
4. Generate platform-specific icons
5. Ensure gogio is available
6. Invoke gogio with platform-specific flags
7. Record successful build in cache
```

**Platform-Specific Build Functions:**

| Function | Platform | Output | Key Details |
|----------|----------|--------|-------------|
| `buildMacOS()` | macos | .app | ARM64 only, supports deep linking, signing |
| `buildAndroid()` | android | .apk | Auto-installs NDK if missing, sets JAVA_HOME |
| `buildIOS()` | ios/ios-simulator | .app | minOS from config, signing support |
| `buildWindows()` | windows | .exe | GOOS=windows, GOARCH=amd64, CGO |
| `buildLinux()` | linux | binary | Uses `go build`, not gogio |
| `buildAll()` | all | varies | Iterates all platforms |

**Key Flags:**
```
--force          Force rebuild even if up-to-date
--check          Check if rebuild needed (exit 0=no, 1=yes)
--skip-icons     Skip icon generation
--output         Custom output directory
--schemes        Deep linking URI schemes (e.g., "myapp://,https://")
--queries        Android app package queries
--signkey        Signing key (keystore/Keychain/provisioning profile)
```

**Build Cache Integration:**
```go
cache := getBuildCache()
needsRebuild, reason := cache.NeedsRebuild(proj.Name, platform, proj.RootDir, appPath)
if !needsRebuild {
    fmt.Printf("✓ %s for %s is up-to-date\n", proj.Name, platform)
    return nil
}
```

### run.go - Run Command

**Purpose:** Build and immediately launch applications on supported platforms.

**Supported Platforms:** macos, android, ios-simulator (platform-dependent)

**Flow:**
```
1. Validate platform for current OS
2. Create and validate project
3. Build using buildMacOS/buildAndroid/buildIOS
4. Launch using platform-specific launcher
```

**Platform Launchers:**

| Platform | Launcher | Details |
|----------|----------|---------|
| macOS | `open <app.app>` | Uses macOS `open` command |
| Android | ADB install + monkey | `adb install -r`, then `monkey -p <pkg>` |
| iOS Simulator | simctl | `xcrun simctl install booted <app>` |

**ADB Integration:**
```go
client := adb.New()
if !client.Available() {
    return fmt.Errorf("adb not found")
}
if !client.HasDevice() {
    return fmt.Errorf("no Android device connected")
}
client.Install(apkPath)
client.Launch("localhost." + appName)  // gogio default package naming
```

### utm.go - UTM VM Management (24KB+)

**Purpose:** Complete UTM virtual machine lifecycle management for cross-platform Windows/Linux development.

**Commands:**

#### VM Lifecycle
| Command | Description |
|---------|-------------|
| `list` / `ls` | List all VMs |
| `gallery` | List available VM templates |
| `create <vm-key>` | Create VM from gallery (AppleScript automation) |
| `status <vm>` | Get VM status |
| `start <vm>` | Start VM |
| `stop <vm>` | Stop VM |
| `ip <vm>` | Get VM IP address |

#### File Operations
| Command | Description |
|---------|-------------|
| `push <vm> <local> <remote>` | Push file to VM |
| `pull <vm> <remote> <local>` | Pull file from VM |
| `export <vm> <output>` | Export VM to .utm file (UTM 4.6+) |
| `import <utm-file>` | Import VM from .utm file |

#### Execution
| Command | Description |
|---------|-------------|
| `exec <vm> -- <cmd>` | Execute command in VM |
| `task <vm> <task>` | Execute Taskfile task in VM |
| `run <vm> <app>` | Build for Windows and run in VM |
| `build <vm> [platform] <app>` | Build inside VM natively |

#### Configuration
| Command | Description |
|---------|-------------|
| `paths` | Show UTM paths configuration |
| `install [vm-key]` | Install UTM app or download ISO |
| `uninstall` | Uninstall UTM app |
| `doctor` | Check UTM installation status |
| `migrate` | Migrate files to global SDK location |
| `port-forward <vm> <guest> <host>` | Set up port forwarding |
| `screenshot <vm> [output]` | Capture VM screenshot |

**Key Features:**

1. **Driver Pattern:** Version-specific drivers for UTM 4.5.x vs 4.6+
   - `driver45` - Limited functionality (no export/import)
   - `driver46` - Full functionality with export/import/guest tools

2. **AppleScript Automation:** Adapted from [packer-plugin-utm](https://github.com/naveenrajm7/packer-plugin-utm)
   - VM creation with full hardware configuration
   - ISO attachment
   - Network configuration

3. **QEMU Guest Agent:** Required for file operations and execution

**Port Forwarding:**
```go
// Requires "Emulated VLAN" network mode
utm-dev utm port-forward "Debian 13" 22 2222 --setup-network
```

**Screenshot Implementation:**
```go
// Uses PowerShell .NET System.Drawing in VM
psScript := `powershell -Command "Add-Type -AssemblyName System.Drawing; ..."`
utm.ExecInVM(vmName, psScript)
utm.PullFile(vmName, remotePath, output)
```

### bundle.go - Bundle Command

**Purpose:** Create signed, distributable application bundles.

**Platforms:** macOS (.app with signing), Windows (MSIX), Android (signed APK)

### package.go - Package Command

**Purpose:** Create distribution archives for release.

**Formats:** .tar.gz (Linux/macOS), .zip (Windows)

### icons.go - Icons Command

**Purpose:** Generate platform-specific application icons.

**Delegates to:** `pkg/icons.GenerateForProject()`

### install.go - SDK Installation

**Purpose:** Download and install mobile development SDKs.

**SDKs:**
- Android NDK
- Android SDK (platform-tools, build-tools, platforms)
- iOS SDK components (via Xcode)
- OpenJDK

**Flow:**
```
1. Check cache for existing installation
2. Download with progress bar (retry logic)
3. Verify SHA256 checksum
4. Extract archive
5. Update cache
```

### android.go & ios.go - Platform Commands

**Android Commands:**
- `emulator list` - List available AVDs
- `emulator start <avd>` - Launch emulator
- `logcat [tags]` - Stream device logs

**iOS Commands:**
- `list` - List simulator devices
- `boot <device>` - Boot simulator
- `screenshot [output]` - Capture simulator screen

### workspace.go - Go Workspace

**Purpose:** Manage Go workspace (go.work) for multi-module projects.

**Commands:**
- `status` - Show workspace status
- `add <module>` - Add module to workspace
- `remove <module>` - Remove module from workspace

**Implementation:**
```go
// Uses `go work use` and `go work drop` commands
cmd := exec.Command("go", "work", "use", modulePath)
```

### self.go - Self Management

**Purpose:** utm-dev self-management commands.

**Commands:**
- `self doctor` - Validate installation
- `self setup` - Initial setup
- `self test` - Self-validation tests
- `self cleanup` - Clean temporary files

## Design Patterns

### 1. Global Build Cache

```go
var globalBuildCache *buildcache.Cache

func getBuildCache() *buildcache.Cache {
    if globalBuildCache == nil {
        cache, _ := buildcache.NewCache(buildcache.GetDefaultCachePath())
        globalBuildCache = cache
    }
    return globalBuildCache
}
```

### 2. Project Validation

```go
proj, err := project.NewGioProject(appDir)
if err != nil {
    return fmt.Errorf("failed to create project: %w", err)
}
if err := proj.Validate(); err != nil {
    return fmt.Errorf("invalid project: %w", err)
}
```

### 3. Idempotent Operations

Most commands are idempotent:
- Build caching skips unchanged projects
- Icon generation checks for existing files
- SDK installation verifies completeness

### 4. Error Wrapping

Consistent error wrapping for debugging:
```go
return fmt.Errorf("build failed: %w", err)
```

## Dependencies

### Internal (pkg/*)
| Package | Used By | Purpose |
|---------|---------|---------|
| `pkg/project` | build, run, bundle | Project structure |
| `pkg/buildcache` | build, run | Build caching |
| `pkg/icons` | build, icons | Icon generation |
| `pkg/utm` | utm | UTM VM control |
| `pkg/installer` | install | SDK installation |
| `pkg/adb` | run, android | Android Debug Bridge |
| `pkg/simctl` | run, ios | iOS Simulator control |
| `pkg/config` | all | Configuration paths |
| `pkg/workspace` | workspace | Go workspace |

### External
| Dependency | Purpose |
|------------|---------|
| `github.com/spf13/cobra` | CLI framework |
| `gioui.org/cmd/gogio` | Gio build tool |
| `adb` (SDK) | Android debugging |
| `xcrun simctl` (Xcode) | iOS simulation |
| `utmctl` (UTM) | VM management |
| `osascript` (AppleScript) | UTM automation |

## Shell Completion

Tab completion is implemented for:
- Platform names (`macos`, `android`, etc.)
- Example directories (`examples/hybrid-dashboard`)
- VM names (from `utmctl list`)

```go
buildCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
    if len(args) == 0 {
        return getPlatformCompletion(cmd, args, toComplete)
    }
    if len(args) == 1 {
        return getExampleCompletion(cmd, args, toComplete)
    }
    return nil, cobra.ShellCompDirectiveNoFileComp
}
```

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `GOWORK=off` | Disable Go workspace during builds |
| `ANDROID_HOME` / `ANDROID_SDK_ROOT` | Android SDK location |
| `ANDROID_NDK_ROOT` | Android NDK location |
| `JAVA_HOME` | Java installation |
| `GOOS`, `GOARCH` | Cross-compilation targets |
| `CGO_ENABLED` | CGO enablement (Linux builds) |

## Testing

### install_test.go
Tests SDK installation logic.

### self_test.go
Tests self-management commands (doctor, setup).

## Future Enhancements

1. **Concurrent Builds:** Parallel platform builds in `buildAll()`
2. **Build Hooks:** Pre/post build script execution
3. **Remote VMs:** SSH-based VM execution (not just UTM)
4. **WebAssembly:** WASM build target support
5. **Code Signing Automation:** Automated certificate management
