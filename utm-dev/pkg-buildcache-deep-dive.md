---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/buildcache/
explored_at: 2026-03-19T12:00:00Z
package: pkg/buildcache
---

# Deep Dive: Build Cache (pkg/buildcache/)

## Overview

The `pkg/buildcache` package implements SHA256-based build caching for idempotent builds. It prevents unnecessary rebuilds by tracking:

- Source file hashes
- Build timestamps
- Build success/failure status
- Output file existence

## Core Concepts

### Idempotent Builds

A build is skipped if:
1. Previous build succeeded
2. Output file exists
3. Source files unchanged
4. Output not older than expected

### BuildState Structure

```go
type BuildState struct {
    Project      string    `json:"project"`      // Project name
    Platform     string    `json:"platform"`     // Target platform
    OutputPath   string    `json:"output_path"`  // Output file path
    SourceHash   string    `json:"source_hash"`  // SHA256 of sources
    LastBuild    time.Time `json:"last_build"`   // Build timestamp
    BuildSuccess bool      `json:"build_success"` // Whether build succeeded
}
```

### Cache Structure

```go
type Cache struct {
    path   string                // Path to cache.json
    states map[string]*BuildState // Map of project:platform -> state
}
```

## API Reference

### Creating a Cache

```go
cache, err := buildcache.NewCache(buildcache.GetDefaultCachePath())
if err != nil {
    cache = &buildcache.Cache{}  // Empty cache on error
}
```

**Default Path:** `~/.utm-dev/build-cache.json`

### Checking if Rebuild Needed

```go
needsRebuild, reason := cache.NeedsRebuild(
    proj.Name,      // "hybrid-dashboard"
    platform,       // "macos"
    proj.RootDir,   // "/path/to/project"
    appPath,        // "/path/to/.bin/macos/hybrid-dashboard.app"
)

if !needsRebuild {
    fmt.Printf("✓ %s for %s is up-to-date\n", proj.Name, platform)
    return nil
}

fmt.Printf("Rebuilding: %s\n", reason)
```

**Possible Reasons:**
- `"no previous build found"` - First build
- `"output doesn't exist"` - Output was deleted
- `"previous build failed"` - Last build failed
- `"sources changed"` - Source files modified
- `"output was modified after build"` - Output tampered with
- `"can't hash sources: ..."` - Error reading sources

### Recording a Build

```go
cache.RecordBuild(
    proj.Name,
    platform,
    proj.RootDir,
    appPath,
    true,  // success = true
)
```

Records:
- Current source hash
- Build timestamp
- Success/failure status
- Output path

### Getting Build State

```go
state := cache.GetState("myapp", "macos")
if state != nil {
    fmt.Printf("Last built: %s\n", state.LastBuild)
    fmt.Printf("Source hash: %s\n", state.SourceHash)
}
```

## Source Hashing

### hashDirectory()

Creates a SHA256 hash of relevant source files:

```go
func hashDirectory(path string) (string, error) {
    h := sha256.New()

    filepath.Walk(path, func(filePath string, info os.FileInfo, err error) error {
        if err != nil {
            return err
        }

        // Skip directories and build artifacts
        if info.IsDir() {
            name := info.Name()
            if name == ".bin" || name == ".build" || name == ".dist" || name == ".git" {
                return filepath.SkipDir
            }
            return nil
        }

        // Only hash source files
        ext := filepath.Ext(filePath)
        if ext != ".go" && ext != ".mod" && ext != ".sum" && ext != ".png" && ext != ".jpg" {
            return nil
        }

        // Hash relative path and modification time
        relPath, _ := filepath.Rel(path, filePath)
        h.Write([]byte(relPath))
        h.Write([]byte(info.ModTime().String()))

        // For small files (<1MB), hash content too
        if info.Size() < 1024*1024 {
            f, err := os.Open(filePath)
            if err != nil {
                return nil
            }
            defer f.Close()
            io.Copy(h, f)
        }

        return nil
    })

    return fmt.Sprintf("%x", h.Sum(nil)), nil
}
```

### Hash Components

1. **Relative file path** - Detects file moves/renames
2. **Modification time** - Quick change detection
3. **File content** (for files <1MB) - Detects content changes

### Skipped Directories

- `.bin/` - Build artifacts
- `.build/` - Build artifacts
- `.dist/` - Distribution archives
- `.git/` - Version control

### Hashed File Types

- `*.go` - Go source
- `*.mod` - Go module definition
- `*.sum` - Go module checksums
- `*.png`, `*.jpg` - Image assets

## Cache File Format

```json
{
  "hybrid-dashboard:macos": {
    "project": "hybrid-dashboard",
    "platform": "macos",
    "output_path": "/path/to/.bin/macos/hybrid-dashboard.app",
    "source_hash": "abc123...",
    "last_build": "2026-03-19T12:00:00Z",
    "build_success": true
  },
  "hybrid-dashboard:android": {
    "project": "hybrid-dashboard",
    "platform": "android",
    "output_path": "/path/to/.bin/android/hybrid-dashboard.apk",
    "source_hash": "def456...",
    "last_build": "2026-03-19T11:00:00Z",
    "build_success": true
  }
}
```

## Usage Pattern

### In cmd/build.go

```go
// Global cache (initialized once)
var globalBuildCache *buildcache.Cache

func getBuildCache() *buildcache.Cache {
    if globalBuildCache == nil {
        cache, err := buildcache.NewCache(buildcache.GetDefaultCachePath())
        if err != nil {
            cache = &buildcache.Cache{}
        }
        globalBuildCache = cache
    }
    return globalBuildCache
}

// Build flow
func buildMacOS(proj *project.GioProject, platform string, opts BuildOptions) error {
    cache := getBuildCache()
    appPath := proj.GetOutputPath(platform)

    // Check if rebuild needed
    if !opts.Force {
        needsRebuild, reason := cache.NeedsRebuild(proj.Name, platform, proj.RootDir, appPath)

        if opts.CheckOnly {
            if needsRebuild {
                os.Exit(1)  // Rebuild needed
            } else {
                os.Exit(0)  // Up to date
            }
        }

        if !needsRebuild {
            fmt.Printf("✓ %s for %s is up-to-date\n", proj.Name, platform)
            return nil
        }

        fmt.Printf("Rebuilding: %s\n", reason)
    }

    // ... perform build ...

    // Record successful build
    cache.RecordBuild(proj.Name, platform, proj.RootDir, appPath, true)

    return nil
}
```

## Design Decisions

### 1. Global Cache Location

**Why `~/.utm-dev/build-cache.json` instead of per-project?**

**Pros:**
- Single source of truth
- Survives project deletion/recreation
- Consistent across projects

**Cons:**
- Cache file can grow large
- Projects share cache state

### 2. Content Hashing Threshold

Files <1MB are fully hashed; larger files only use path+mtime.

**Rationale:**
- Most source files are small
- Large images change infrequently
- Performance optimization

### 3. Build Failure Tracking

```go
if !state.BuildSuccess {
    return true, "previous build failed"
}
```

**Why:** Failed builds always trigger rebuild, even if sources unchanged.

### 4. Output Age Check

```go
if outputInfo.ModTime().Before(state.LastBuild) {
    return true, "output was modified after build"
}
```

**Why:** Detects manual output modification or corruption.

### 5. JSON Indentation

```go
data, err := json.MarshalIndent(c.states, "", "  ")
```

**Why:** Human-readable cache file for debugging.

## Error Handling

### Graceful Degradation

```go
func NewCache(cacheFile string) (*Cache, error) {
    cache := &Cache{
        path:   cacheFile,
        states: make(map[string]*BuildState),
    }

    if _, err := os.Stat(cacheFile); err == nil {
        if err := cache.load(); err != nil {
            cache.states = make(map[string]*BuildState)  // Start fresh
        }
    }

    return cache, nil
}
```

If cache is corrupted, starts with empty state rather than failing.

### Hash Failure Handling

```go
sourceHash, err := hashDirectory(projectPath)
if err != nil {
    sourceHash = ""  // Continue even if hashing fails
}
```

Build recording continues even if source hashing fails.

## Performance Considerations

### 1. Directory Walking

`hashDirectory()` walks the entire project directory.

**Optimization:** Skips non-source files early.

### 2. Cache Persistence

Cache is saved after every build:

```go
func (c *Cache) Save() error {
    data, err := json.MarshalIndent(c.states, "", "  ")
    return os.WriteFile(c.path, data, 0644)
}
```

**Impact:** Minimal for typical use (single build at a time).

### 3. File Content Hashing

Only files <1MB are fully hashed.

**Impact:** Large icon files don't slow down builds.

## Testing

No dedicated test file, but tested through:
- `cmd/build.go` integration tests
- `cmd/self_test.go`
- Manual testing with `--check` flag

## Future Enhancements

1. **Cache Pruning:** Remove stale entries
2. **Cache Statistics:** `utm-dev self cache-stats`
3. **Incremental Hashing:** Only hash changed files
4. **Distributed Cache:** Share cache across machines
5. **Per-Platform Cache:** Separate cache files per platform
