---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/workspace/
explored_at: 2026-03-19T12:00:00Z
package: pkg/workspace
---

# Deep Dive: Go Workspace (pkg/workspace/)

## Overview

The `pkg/workspace` package manages Go workspaces (go.work files) for multi-module projects. It provides:

- Workspace discovery via filesystem traversal
- Module addition/removal using `go work use/drop`
- Idempotent operations (safe to call multiple times)
- Module listing and status

## Go Workspace Background

Go workspaces (introduced in Go 1.18) allow multiple Go modules to be developed together:

```
myworkspace/
├── go.work
├── module1/
│   └── go.mod
├── module2/
│   └── go.mod
└── module3/
    └── go.mod
```

**go.work format:**
```go
go 1.21

use (
    ./module1
    ./module2
    ./module3
)
```

## Workspace Type

### Structure

```go
type Workspace struct {
    FilePath string   // Path to go.work file
    Exists   bool     // Whether workspace exists
    Modules  []string // List of module paths (e.g., "./module1")
}
```

## Discovery

### FindWorkspace()

Searches for go.work using two strategies:

```go
func FindWorkspace(startPath string) (*Workspace, error) {
    // Strategy 1: Check GOWORK environment variable (most reliable)
    cmd := exec.Command("go", "env", "GOWORK")
    output, err := cmd.Output()
    if err == nil && len(strings.TrimSpace(string(output))) > 0 {
        workFile := strings.TrimSpace(string(output))
        return loadWorkspace(workFile)
    }

    // Strategy 2: Filesystem traversal (fallback)
    return findWorkspaceByTraversal(startPath)
}
```

### findWorkspaceByTraversal()

Walks up directory tree looking for go.work:

```go
func findWorkspaceByTraversal(startPath string) (*Workspace, error) {
    currentDir := startPath
    if currentDir == "" {
        var err error
        currentDir, err = os.Getwd()
        if err != nil {
            return nil, fmt.Errorf("failed to get current directory: %w", err)
        }
    }

    for {
        workFile := filepath.Join(currentDir, "go.work")
        if _, err := os.Stat(workFile); err == nil {
            return loadWorkspace(workFile)
        }

        parentDir := filepath.Dir(currentDir)
        if parentDir == currentDir {
            // Reached root directory
            break
        }
        currentDir = parentDir
    }

    return &Workspace{Exists: false}, nil
}
```

**Traversal Example:**
```
Starting at: /projects/myapp/examples/hybrid-dashboard
Check: /projects/myapp/examples/hybrid-dashboard/go.work
Check: /projects/myapp/examples/go.work
Check: /projects/myapp/go.work  ✓ Found!
```

## Parsing

### loadWorkspace()

Parses go.work file format:

```go
func loadWorkspace(filePath string) (*Workspace, error) {
    file, err := os.Open(filePath)
    if err != nil {
        return nil, fmt.Errorf("failed to open go.work: %w", err)
    }
    defer file.Close()

    ws := &Workspace{
        FilePath: filePath,
        Exists:   true,
        Modules:  []string{},
    }

    scanner := bufio.NewScanner(file)
    inUseBlock := false

    for scanner.Scan() {
        line := strings.TrimSpace(scanner.Text())

        // Detect "use" statement
        if strings.HasPrefix(line, "use") {
            inUseBlock = true

            // Single-line use: use ./module1
            if !strings.Contains(line, "(") {
                parts := strings.Fields(line)
                if len(parts) >= 2 {
                    ws.Modules = append(ws.Modules, parts[1])
                }
                continue
            }
            // Multi-line use block starts: use (
            continue
        }

        // Inside use block
        if inUseBlock {
            if strings.Contains(line, ")") {
                inUseBlock = false
                continue
            }
            if line != "" && !strings.HasPrefix(line, "//") {
                ws.Modules = append(ws.Modules, line)
            }
        }
    }

    return ws, scanner.Err()
}
```

**Parsed Output:**
```go
Workspace{
    FilePath: "/projects/myapp/go.work",
    Exists:   true,
    Modules: []string{
        "./examples/hybrid-dashboard",
        "./examples/gio-basic",
    },
}
```

## Module Operations

### HasModule()

Checks if a module is already in workspace:

```go
func (w *Workspace) HasModule(modulePath string) bool {
    for _, mod := range w.Modules {
        if mod == modulePath {
            return true
        }
    }
    return false
}
```

### AddModule()

Adds a module to workspace:

```go
func (w *Workspace) AddModule(modulePath string, force bool) error {
    if !w.Exists {
        return fmt.Errorf("no go.work file found")
    }

    // Idempotent check
    if w.HasModule(modulePath) {
        return nil  // Already exists
    }

    if !force {
        return fmt.Errorf("module %s not in workspace (use --force to add)", modulePath)
    }

    // Use `go work use` command (safer than manual file editing)
    cmd := exec.Command("go", "work", "use", modulePath)
    cmd.Dir = filepath.Dir(w.FilePath)

    if err := cmd.Run(); err != nil {
        return fmt.Errorf("failed to add module to workspace: %w", err)
    }

    // Reload to get updated state
    updated, err := loadWorkspace(w.FilePath)
    if err != nil {
        return err
    }

    w.Modules = updated.Modules
    return nil
}
```

**Why `go work use`?** Safer than manually editing go.work - handles edge cases like:
- go.work.sum synchronization
- Proper formatting
- Concurrent modification safety

### RemoveModule()

Removes a module from workspace:

```go
func (w *Workspace) RemoveModule(modulePath string, force bool) error {
    if !w.Exists {
        return fmt.Errorf("no go.work file found")
    }

    // Idempotent check
    if !w.HasModule(modulePath) {
        return nil  // Already removed
    }

    if !force {
        return fmt.Errorf("module %s exists in workspace (use --force to remove)", modulePath)
    }

    // Use `go work drop` command
    cmd := exec.Command("go", "work", "drop", modulePath)
    cmd.Dir = filepath.Dir(w.FilePath)

    if err := cmd.Run(); err != nil {
        return fmt.Errorf("failed to remove module from workspace: %w", err)
    }

    // Reload to get updated state
    updated, err := loadWorkspace(w.FilePath)
    if err != nil {
        return err
    }

    w.Modules = updated.Modules
    return nil
}
```

## Query Methods

### Info()

Returns human-readable workspace info:

```go
func (w *Workspace) Info() string {
    if !w.Exists {
        return "No go.work file found"
    }
    return fmt.Sprintf("go.work: %s (%d modules)", w.FilePath, len(w.Modules))
}
```

### String()

Short summary for printing:

```go
func (w *Workspace) String() string {
    if !w.Exists {
        return "No workspace"
    }
    return fmt.Sprintf("Workspace: %s", filepath.Base(filepath.Dir(w.FilePath)))
}
```

### ListModules()

Returns copy of module list:

```go
func (w *Workspace) ListModules() []string {
    if !w.Exists {
        return []string{}
    }
    return append([]string{}, w.Modules...)  // Return copy
}
```

### WorkspaceRoot()

Returns directory containing go.work:

```go
func (w *Workspace) WorkspaceRoot() string {
    if !w.Exists {
        return ""
    }
    return filepath.Dir(w.FilePath)
}
```

## Usage Patterns

### In cmd/workspace.go

```go
var workspaceStatusCmd = &cobra.Command{
    Use: "status",
    RunE: func(cmd *cobra.Command, args []string) error {
        ws, err := workspace.FindWorkspace(".")
        if err != nil {
            return err
        }

        fmt.Println(ws.Info())

        if ws.Exists {
            fmt.Println("\nModules:")
            for _, mod := range ws.ListModules() {
                fmt.Printf("  %s\n", mod)
            }
        }

        return nil
    },
}

var workspaceAddCmd = &cobra.Command{
    Use: "add <module>",
    RunE: func(cmd *cobra.Command, args []string) error {
        ws, _ := workspace.FindWorkspace(".")
        force, _ := cmd.Flags().GetBool("force")

        return ws.AddModule(args[0], force)
    },
}
```

## Idempotent Operations

Both AddModule and RemoveModule are idempotent when called without force:

```go
// First call - adds module
ws.AddModule("./myapp", true)  // Success

// Second call - no-op
ws.AddModule("./myapp", true)  // Returns nil (already exists)

// Remove - also idempotent
ws.RemoveModule("./myapp", true)  // Success
ws.RemoveModule("./myapp", true)  // Returns nil (already removed)
```

## Error Handling

### Graceful Degradation

```go
func FindWorkspace(startPath string) (*Workspace, error) {
    // If GOWORK check fails, try filesystem traversal
    cmd := exec.Command("go", "env", "GOWORK")
    output, err := cmd.Output()
    if err == nil && len(strings.TrimSpace(string(output))) > 0 {
        return loadWorkspace(strings.TrimSpace(string(output)))
    }

    // Fallback to traversal
    return findWorkspaceByTraversal(startPath)
}
```

### Force Flag

The `force` flag is required for modifications:

```go
// Without force - safety check
ws.AddModule("./myapp", false)
// Error: "module ./myapp not in workspace (use --force to add)"

// With force - automatic
ws.AddModule("./myapp", true)
// Adds module to workspace
```

## Testing

### workspace_test.go

Tests workspace discovery and parsing:

```go
func TestFindWorkspace(t *testing.T) {
    // Create temporary workspace
    tmpDir := t.TempDir()
    workFile := filepath.Join(tmpDir, "go.work")

    os.WriteFile(workFile, []byte(`
go 1.21

use (
    ./module1
    ./module2
)
`), 0644)

    ws, err := findWorkspaceByTraversal(tmpDir)
    if err != nil {
        t.Fatal(err)
    }

    if !ws.Exists {
        t.Error("Expected workspace to exist")
    }

    if len(ws.Modules) != 2 {
        t.Errorf("Expected 2 modules, got %d", len(ws.Modules))
    }
}
```

## Design Decisions

### 1. GOWORK First, Then Traversal

**Why:** GOWORK environment variable is authoritative; traversal is fallback.

### 2. go work use/drop Instead of Manual Editing

**Why:** Go's own commands handle edge cases:
- Proper file formatting
- go.work.sum synchronization
- Concurrent modification safety

### 3. Idempotent Operations

**Why:** Safe to call multiple times in scripts/automation.

### 4. Force Flag Required

**Why:** Prevents accidental workspace modifications.

## Integration Points

### With cmd/workspace.go

Provides CLI interface for workspace management:
- `utm-dev workspace status` - Show workspace info
- `utm-dev workspace add <module>` - Add module
- `utm-dev workspace remove <module>` - Remove module
- `utm-dev workspace list` - List modules

### With cmd/build.go

Build commands may check workspace status to ensure consistent builds.

## Future Enhancements

1. **Automatic Workspace Detection:** Suggest workspace creation for multi-module projects
2. **Module Discovery:** Auto-find all modules in directory tree
3. **Sync Command:** `go work sync` integration
4. **Use File Support:** Handle use (file) directives
