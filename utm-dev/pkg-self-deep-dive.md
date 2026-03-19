---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/self/
explored_at: 2026-03-19T12:00:00Z
package: pkg/self
---

# Deep Dive: Self Management (pkg/self/)

## Overview

The `pkg/self` package handles utm-dev's self-management commands including installation validation, setup, testing, and cleanup. It provides:

- Installation doctor (dependency validation)
- Initial setup automation
- Self-testing capabilities
- Release management
- Cleanup utilities

## Package Structure

```
pkg/self/
├── architecture.go      # System architecture detection
├── build.go            # Build information
├── config.go           # Self configuration
├── deps.go             # Dependency management
├── doctor.go           # Installation validation
├── install.go          # Installation logic
├── release.go          # Release management
├── release_check.go    # Release validation
├── scripts.go          # Installation scripts
├── test.go             # Self-testing
├── version.go          # Version information
└── output/
    ├── config.go       # Output configuration
    ├── output.go       # Output formatting
    ├── result.go       # Result structures
    └── wrapper.go      # Output wrappers
```

## doctor.go - Installation Validation

### Doctor() Function

Validates the entire utm-dev installation:

```go
func Doctor() error {
    result := output.DoctorResult{
        Installations: []output.InstallationInfo{},
        Dependencies:  []output.DependencyInfo{},
        Issues:        []string{},
        Suggestions:   []string{},
    }

    // Check utm-dev itself - find ALL installations
    installations := findAllInstallations()

    if len(installations) == 0 {
        result.Issues = append(result.Issues, "utm-dev not found in PATH")
        result.Suggestions = append(result.Suggestions,
            "Run: curl -sSL https://github.com/joeblew999/utm-dev/releases/latest/download/macos-bootstrap.sh | bash")
    } else {
        for i, path := range installations {
            info := output.InstallationInfo{
                Path:     path,
                Active:   i == 0,      // First in PATH is active
                Shadowed: i > 0,       // Others are shadowed
            }
            result.Installations = append(result.Installations, info)
        }

        if len(installations) > 1 {
            result.Issues = append(result.Issues, "Multiple utm-dev installations found")
            for i, path := range installations {
                if i > 0 {
                    result.Suggestions = append(result.Suggestions, "Remove: "+path)
                }
            }
        }
    }

    // Check platform-specific package manager
    switch runtime.GOOS {
    case "darwin":
        result.Dependencies = append(result.Dependencies,
            checkDep("Homebrew", "brew", "--version"))
    case "windows":
        result.Dependencies = append(result.Dependencies,
            checkDep("winget", "winget", "--version"))
    }

    // Check git
    gitDep := checkDep("git", "git", "--version")
    result.Dependencies = append(result.Dependencies, gitDep)
    if !gitDep.Installed {
        result.Issues = append(result.Issues, "git not installed")
        result.Suggestions = append(result.Suggestions, "Install git")
    }

    // Check go
    goDep := checkDep("go", "go", "version")
    result.Dependencies = append(result.Dependencies, goDep)
    if !goDep.Installed {
        result.Issues = append(result.Issues, "go not installed")
        result.Suggestions = append(result.Suggestions, "Install go")
    }

    // Check mise
    miseDep := checkDep("mise", "mise", "version")
    result.Dependencies = append(result.Dependencies, miseDep)
    if !miseDep.Installed {
        result.Issues = append(result.Issues, "mise not installed")
        result.Suggestions = append(result.Suggestions,
            "Install mise: curl -fsSL https://mise.run | sh")
    }

    output.OK("self doctor", result)
    return nil
}
```

### checkDep() - Dependency Check Helper

```go
func checkDep(name, command string, args ...string) output.DependencyInfo {
    dep := output.DependencyInfo{
        Name:      name,
        Installed: false,
    }

    cmd := exec.Command(command, args...)
    out, err := cmd.CombinedOutput()
    if err == nil {
        dep.Installed = true
        // Extract version from first line
        lines := strings.Split(string(out), "\n")
        if len(lines) > 0 {
            dep.Version = strings.TrimSpace(lines[0])
        }
    }

    return dep
}
```

### findAllInstallations() - Find All utm-dev Binaries

```go
func findAllInstallations() []string {
    var installations []string
    pathEnv := os.Getenv("PATH")
    paths := filepath.SplitList(pathEnv)

    for _, dir := range paths {
        binaryPath := filepath.Join(dir, BinaryName)

        // Check if file exists and is executable
        if info, err := os.Stat(binaryPath); err == nil && !info.IsDir() {
            if info.Mode()&0111 != 0 {  // Executable bit check
                installations = append(installations, binaryPath)
            }
        }
    }

    return installations
}
```

**Output Example:**
```
utm-dev self doctor
══════════════════════════════════════════════════════════

Installations:
  ✓ /usr/local/bin/utm-dev (active)
  ⚠ /opt/homebrew/bin/utm-dev (shadowed)

Dependencies:
  ✓ Homebrew: 4.2.15
  ✓ git: 2.44.0
  ✓ go: 1.22.1
  ✓ mise: 2024.3.12

Issues:
  • Multiple utm-dev installations found

Suggestions:
  • Remove: /opt/homebrew/bin/utm-dev
```

## version.go - Version Management

```go
// Version information (set at build time via ldflags)
var (
    Version   = "dev"
    GitCommit = ""
    BuildDate = ""
)

// GetVersion returns the current version string
func GetVersion() string {
    return Version
}

// GetBuildInfo returns full build information
func GetBuildInfo() BuildInfo {
    return BuildInfo{
        Version:   Version,
        GitCommit: GitCommit,
        BuildDate: BuildDate,
    }
}
```

**Build flags:**
```bash
go build -ldflags "-X github.com/joeblew999/utm-dev/pkg/self.Version=1.0.0 \
                     -X github.com/joeblew999/utm-dev/pkg/self.GitCommit=abc123 \
                     -X github.com/joeblew999/utm-dev/pkg/self.BuildDate=2024-01-01"
```

## install.go - Installation Logic

### Installation Steps

```go
func Install() error {
    // 1. Check prerequisites
    if err := checkPrerequisites(); err != nil {
        return err
    }

    // 2. Create installation directory
    installDir := getInstallDir()
    os.MkdirAll(installDir, 0755)

    // 3. Copy binary
    binaryPath := filepath.Join(installDir, "utm-dev")
    // Copy current binary to install location

    // 4. Ensure PATH includes install directory
    if !pathContains(installDir) {
        fmt.Println("Add to PATH:")
        fmt.Printf("  export PATH=\"%s:$PATH\"\n", installDir)
    }

    // 5. Run self-test
    return testInstallation()
}
```

## setup.go - Initial Setup

```go
func Setup() error {
    // 1. Ensure directories exist
    config.EnsureDirectories()

    // 2. Check for required tools
    checkTools()

    // 3. Initialize configuration
    initConfig()

    // 4. Run self-test
    return SelfTest()
}
```

## test.go - Self-Testing

```go
func SelfTest() error {
    tests := []func() error{
        testDirectories,
        testSDKPaths,
        testBuildCache,
        testUTMIntegration,
    }

    for _, test := range tests {
        if err := test(); err != nil {
            return fmt.Errorf("self-test failed: %w", err)
        }
    }

    fmt.Println("✓ All self-tests passed")
    return nil
}
```

## cleanup.go - Cleanup Operations

```go
func Cleanup() error {
    var removed []string

    // Clean build cache
    cacheDir := config.GetCacheDir()
    if _, err := os.Stat(cacheDir); err == nil {
        os.RemoveAll(cacheDir)
        removed = append(removed, cacheDir)
    }

    // Clean temporary files
    tmpDir := filepath.Join(config.GetCacheDir(), "tmp")
    if _, err := os.Stat(tmpDir); err == nil {
        os.RemoveAll(tmpDir)
        removed = append(removed, tmpDir)
    }

    fmt.Printf("Removed %d directories\n", len(removed))
    return nil
}
```

## output/ - Output Formatting

### Result Structures

```go
type DoctorResult struct {
    Installations []InstallationInfo
    Dependencies  []DependencyInfo
    Issues        []string
    Suggestions   []string
}

type InstallationInfo struct {
    Path     string
    Active   bool
    Shadowed bool
}

type DependencyInfo struct {
    Name      string
    Installed bool
    Version   string
}
```

### OK() - Success Output

```go
func OK(command string, result interface{}) {
    fmt.Printf("✓ %s completed successfully\n", command)
    // Format and print result
}
```

## Design Decisions

### 1. Multiple Installation Detection

**Why:** Users often install via multiple methods (brew, manual, script).

**Benefit:** Helps diagnose "wrong version" issues.

### 2. Comprehensive Doctor Output

**Why:** Single command should reveal all installation issues.

### 3. Force Flag for Modifications

**Why:** Prevents accidental changes to workspace/configuration.

## Future Enhancements

1. **Auto-Fix:** `utm-dev self doctor --fix` for automatic repairs
2. **Update Check:** Periodic update notifications
3. **Rollback:** Revert to previous version
4. **Plugin System:** Third-party extensions
