---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/project/
explored_at: 2026-03-19T12:00:00Z
package: pkg/project
---

# Deep Dive: Project Management (pkg/project/)

## Overview

The `pkg/project` package provides the foundational abstractions for representing and managing Gio application projects. It defines the `GioProject` type which encapsulates:

- Project structure validation
- Path resolution for all platforms
- Build artifact organization
- Icon asset management

## Core Type: GioProject

```go
type GioProject struct {
    // Root directory of the Gio app
    RootDir string

    // App name (derived from directory name or go.mod)
    Name string

    // Build output directory
    OutputDir string
}
```

### Construction

```go
// Basic project creation
proj, err := project.NewGioProject("./examples/hybrid-dashboard")

// With custom output directory
proj, err := project.NewGioProjectWithOutput("./myapp", "../build-output")
```

**Validation on Creation:**
1. Directory must exist
2. Path is converted to absolute
3. App name derived from directory basename

### ProjectPaths Structure

```go
type ProjectPaths struct {
    Root         string  // Project root directory
    Output       string  // Build output (.bin/)
    SourceIcon   string  // icon-source.png
    AndroidIcons string  // Android icon output
    IOSIcons     string  // iOS icon assets
    WindowsIcons string  // Windows icon output
    GoMod        string  // go.mod path
    MainGo       string  // main.go path
    MSIXData     string  // msix-data.yml
    AppConfig    string  // app.json
}
```

**Access Pattern:**
```go
paths := proj.Paths()
iconPath := paths.SourceIcon  // /project/icon-source.png
outputPath := proj.GetOutputPath("macos")  // /project/.bin/macos/app.app
```

## Path Resolution

### Platform-Specific Output Paths

```go
// GetOutputPath returns platform-specific artifact paths
proj.GetOutputPath("macos")    // .bin/macos/app.app
proj.GetOutputPath("android")  // .bin/android/app.apk
proj.GetOutputPath("ios")      // .bin/ios/app.app
proj.GetOutputPath("windows")  // .bin/windows/app.exe
proj.GetOutputPath("linux")    // .bin/linux/app
```

**Directory Structure:**
```
myapp/
├── main.go
├── go.mod
├── icon-source.png
├── app.json
└── .bin/
    ├── macos/
    │   └── myapp.app
    ├── android/
    │   └── myapp.apk
    ├── ios/
    │   └── myapp.app
    ├── ios-simulator/
    │   └── myapp.app
    ├── windows/
    │   └── myapp.exe
    └── linux/
        └── myapp
```

### GetPlatformDir()

Returns the platform-specific directory without the filename:

```go
proj.GetPlatformDir("macos")  // .bin/macos/
```

## Validation

```go
func (p *GioProject) Validate() error {
    paths := p.Paths()

    // Check go.mod exists
    if _, err := os.Stat(paths.GoMod); os.IsNotExist(err) {
        return fmt.Errorf("go.mod not found: %s", paths.GoMod)
    }

    // Check main.go exists
    if _, err := os.Stat(paths.MainGo); os.IsNotExist(err) {
        return fmt.Errorf("main.go not found: %s", paths.MainGo)
    }

    return nil
}
```

**Validation Rules:**
1. `go.mod` must exist (valid Go module)
2. `main.go` must exist (entry point)
3. Directories must be accessible

## Icon Management

### Source Icon

```go
// Check if source icon exists
hasIcon := proj.HasSourceIcon()  // checks icon-source.png

// Generate test icon if missing (handled by cmd/icons.go)
err := proj.GenerateSourceIcon()
```

### Platform Icon Paths

```go
paths := proj.Paths()

// Android icons go to: project/build/
androidIconPath := paths.AndroidIcons

// iOS icons go to: project/build/Assets.xcassets/
iosIconPath := paths.IOSIcons

// Windows icons go to: project/build/
windowsIconPath := paths.WindowsIcons
```

## Directory Management

```go
// EnsureDirectories creates necessary output directories
err := proj.EnsureDirectories()
// Creates:
// - Output directory (.bin/)
// - iOS Assets.xcassets/
// - Platform subdirectories
```

## Usage in Build Commands

```go
// cmd/build.go pattern
proj, err := project.NewGioProject(appDir)
if err != nil {
    return fmt.Errorf("failed to create project: %w", err)
}

if err := proj.Validate(); err != nil {
    return fmt.Errorf("invalid project: %w", err)
}

// Get platform-specific paths
platformDir := proj.GetPlatformDir(platform)
appPath := proj.GetOutputPath(platform)

// Use icon path for gogio
iconPath := proj.Paths().SourceIcon
gogioCmd := exec.Command("gogio", "-icon", iconPath, "-o", appPath, ".")
```

## Design Decisions

### 1. Absolute Paths

All paths are converted to absolute on construction:
```go
absPath, err := filepath.Abs(rootDir)
```

**Rationale:** Prevents path resolution errors when changing working directories during builds.

### 2. Centralized Path Logic

All path computation goes through `GioProject` methods, not scattered throughout codebase.

**Benefits:**
- Single source of truth
- Easy to modify output structure
- Consistent cross-platform behavior

### 3. Parent Reference in ProjectPaths

```go
type ProjectPaths struct {
    project *GioProject  // Reference to parent
    // ...
}
```

**Purpose:** Allows path methods to access project state if needed.

### 4. Separation from Icons Package

The project package doesn't directly generate icons (that's `pkg/icons`), but provides the path infrastructure.

**Avoids:** Circular dependencies between `pkg/project` and `pkg/icons`.

## Integration Points

### With pkg/buildcache

```go
cache.RecordBuild(proj.Name, platform, proj.RootDir, appPath, true)
```

Uses `proj.Name`, `proj.RootDir`, and `proj.GetOutputPath()`.

### With pkg/icons

```go
sourceIconPath := proj.Paths().SourceIcon
outputPath := proj.Paths().GetIconOutputPath(platform)
```

### With cmd/build.go

Build commands create `GioProject` and use it throughout the build lifecycle.

## Testing

No dedicated test file in project package, but extensively tested through:
- `cmd/build.go` integration tests
- `cmd/install_test.go`
- `integration_test.go`

## Future Enhancements

1. **Module Name Extraction:** Parse `go.mod` for actual module name vs directory name
2. **Multi-Module Support:** Handle projects with multiple Go modules
3. **Config Validation:** Validate `app.json` schema
4. **Template Support:** Project scaffolding from templates
