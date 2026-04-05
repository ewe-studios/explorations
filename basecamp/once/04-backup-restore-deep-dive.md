---
source: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/once
repository: github.com/basecamp/once
explored_at: 2026-04-05
focus: Backup archive format, restore process, hook scripts, backup scheduling, retention policies
---

# Deep Dive: Backup and Restore Architecture

## Overview

This deep dive examines ONCE's backup and restore system - how consistent backup archives are created, stored, scheduled, and restored with proper hook execution for data integrity.

## Architecture

```mermaid
flowchart TB
    subgraph App["Application Container"]
        Data[/storage Data]
        Hooks[Hook Scripts]
    end
    
    subgraph Backup["Backup Process"]
        PreHook[pre-backup Hook]
        Extract[Extract Volume]
        Compress[Compress Archive]
        Write[Write to Storage]
    end
    
    subgraph Archive["Backup Archive"]
        AppSettings[app-settings.json]
        VolSettings[vol-settings.json]
        DataDir[data/]
    end
    
    subgraph Storage["Backup Storage"]
        Local[Local Directory]
        Remote[Remote Storage S3/NFS]
    end
    
    subgraph Restore["Restore Process"]
        Parse[Parse Archive]
        CreateVol[Create Volume]
        RestoreData[Restore Data]
        PostHook[post-restore Hook]
        Deploy[Deploy Container]
    end
    
    Data --> PreHook
    Hooks --> PreHook
    PreHook --> Extract
    Extract --> Compress
    Compress --> Write
    Write --> Archive
    
    Archive --> AppSettings
    Archive --> VolSettings
    Archive --> DataDir
    
    Write --> Local
    Write --> Remote
    
    Local --> Parse
    Remote --> Parse
    Parse --> CreateVol
    CreateVol --> RestoreData
    RestoreData --> PostHook
    PostHook --> Deploy
```

## Backup Archive Format

### Archive Structure

```
once-backup-writebook-2024-01-15-143022.tar.gz
├── app-settings.json          # Application configuration
├── vol-settings.json          # Volume secrets and keys
└── data/                      # Volume data (everything from /storage)
    ├── production.sqlite3     # SQLite database
    ├── backup.sqlite3         # SQLite backup (from pre-backup hook)
    ├── storage/
    │   ├── attachments/
    │   │   ├── file1.pdf
    │   │   └── file2.docx
    │   └── avatars/
    │       └── user123.png
    └── logs/
        └── production.log
```

### Archive Creation

```go
// internal/docker/application_backup.go

package docker

import (
    "archive/tar"
    "compress/gzip"
    "context"
    "encoding/json"
    "fmt"
    "io"
    "os"
    "path/filepath"
    "time"
)

// BackupArchive represents the structure of a backup
type BackupArchive struct {
    AppSettings  *ApplicationSettings
    VolSettings  *ApplicationVolumeSettings
    VolumeData   io.Reader
    CreatedAt    time.Time
    Checksum     string
}

// createBackupArchive creates a new backup archive
func (a *Application) createBackupArchive(ctx context.Context) (*BackupArchive, error) {
    vol, err := a.Volume(ctx)
    if err != nil {
        return nil, fmt.Errorf("getting volume: %w", err)
    }
    
    // Check for pre-backup hook
    hasHook, err := a.hasPreBackupHook(ctx)
    if err != nil {
        return nil, err
    }
    
    var paused bool
    if hasHook {
        // Run pre-backup hook - app prepares for backup
        if err := a.execHook(ctx, "/hooks/pre-backup"); err != nil {
            return nil, fmt.Errorf("pre-backup hook failed: %w", err)
        }
    } else {
        // Pause container for consistent snapshot
        if a.Running {
            if err := a.namespace.client.ContainerPause(ctx, a.containerID()); err != nil {
                return nil, fmt.Errorf("pausing container: %w", err)
            }
            paused = true
        }
    }
    
    defer func() {
        if paused {
            a.namespace.client.ContainerUnpause(ctx, a.containerID())
        }
    }()
    
    // Extract volume data from container
    volumeReader, _, err := a.namespace.client.CopyFromContainer(ctx, 
        a.containerID(), "/storage")
    if err != nil {
        return nil, fmt.Errorf("extracting volume: %w", err)
    }
    defer volumeReader.Close()
    
    return &BackupArchive{
        AppSettings:  &a.Settings,
        VolSettings:  &vol.Settings,
        VolumeData:   volumeReader,
        CreatedAt:    time.Now(),
    }, nil
}

// writeBackup writes the archive to a file
func (a *BackupArchive) writeBackup(backupPath string) error {
    // Create parent directory if needed
    dir := filepath.Dir(backupPath)
    if err := os.MkdirAll(dir, 0755); err != nil {
        return err
    }
    
    // Create backup file
    backupFile, err := os.Create(backupPath)
    if err != nil {
        return err
    }
    defer backupFile.Close()
    
    // Create gzip writer
    gw := gzip.NewWriter(backupFile)
    defer gw.Close()
    
    // Create tar writer
    tw := tar.NewWriter(gw)
    defer tw.Close()
    
    // Write app-settings.json
    appSettingsJSON, err := json.MarshalIndent(a.AppSettings, "", "  ")
    if err != nil {
        return err
    }
    
    if err := tw.WriteHeader(&tar.Header{
        Name: "app-settings.json",
        Mode: 0644,
        Size: int64(len(appSettingsJSON)),
        ModTime: a.CreatedAt,
    }); err != nil {
        return err
    }
    
    if _, err := tw.Write(appSettingsJSON); err != nil {
        return err
    }
    
    // Write vol-settings.json
    volSettingsJSON, err := json.MarshalIndent(a.VolSettings, "", "  ")
    if err != nil {
        return err
    }
    
    if err := tw.WriteHeader(&tar.Header{
        Name: "vol-settings.json",
        Mode: 0600,  // Restricted permissions for secrets
        Size: int64(len(volSettingsJSON)),
        ModTime: a.CreatedAt,
    }); err != nil {
        return err
    }
    
    if _, err := tw.Write(volSettingsJSON); err != nil {
        return err
    }
    
    // Write volume data
    if err := a.copyTarToBackup(tw, a.VolumeData); err != nil {
        return fmt.Errorf("writing volume data: %w", err)
    }
    
    return nil
}

// copyTarToBackup copies data from one tar to another
func (a *BackupArchive) copyTarToBackup(tw *tar.Writer, src io.Reader) error {
    tr := tar.NewReader(src)
    
    for {
        header, err := tr.Next()
        if err == io.EOF {
            break
        }
        if err != nil {
            return err
        }
        
        // Prefix all paths with "data/"
        header.Name = filepath.Join("data", header.Name)
        
        // Write header
        if err := tw.WriteHeader(header); err != nil {
            return err
        }
        
        // Write content
        if _, err := io.Copy(tw, tr); err != nil {
            return err
        }
    }
    
    return nil
}

// Calculate checksum for integrity verification
func (a *BackupArchive) calculateChecksum(backupPath string) (string, error) {
    file, err := os.Open(backupPath)
    if err != nil {
        return "", err
    }
    defer file.Close()
    
    hash := sha256.New()
    if _, err := io.Copy(hash, file); err != nil {
        return "", err
    }
    
    return fmt.Sprintf("%x", hash.Sum(nil)), nil
}
```

### Backup Metadata

```go
// internal/docker/backup_metadata.go

package docker

import (
    "encoding/json"
    "os"
    "path/filepath"
    "time"
)

// BackupMetadata stores information about a backup
type BackupMetadata struct {
    Application string    `json:"application"`
    Hostname    string    `json:"hostname"`
    CreatedAt   time.Time `json:"created_at"`
    Size        int64     `json:"size_bytes"`
    Checksum    string    `json:"checksum"`
    BackupPath  string    `json:"backup_path"`
    Notes       string    `json:"notes,omitempty"`
}

// WriteMetadata writes metadata alongside the backup
func WriteMetadata(backupPath string, meta *BackupMetadata) error {
    metaPath := backupPath + ".meta.json"
    
    data, err := json.MarshalIndent(meta, "", "  ")
    if err != nil {
        return err
    }
    
    return os.WriteFile(metaPath, data, 0644)
}

// ReadMetadata reads metadata for a backup
func ReadMetadata(backupPath string) (*BackupMetadata, error) {
    metaPath := backupPath + ".meta.json"
    
    data, err := os.ReadFile(metaPath)
    if err != nil {
        return nil, err
    }
    
    var meta BackupMetadata
    if err := json.Unmarshal(data, &meta); err != nil {
        return nil, err
    }
    
    return &meta, nil
}

// ListBackups returns all backups for an application
func ListBackups(appName string, backupDir string) ([]BackupMetadata, error) {
    pattern := filepath.Join(backupDir, fmt.Sprintf("once-backup-%s-*.tar.gz", appName))
    
    files, err := filepath.Glob(pattern)
    if err != nil {
        return nil, err
    }
    
    var backups []BackupMetadata
    for _, file := range files {
        meta, err := ReadMetadata(file)
        if err != nil {
            continue  // Skip backups without metadata
        }
        backups = append(backups, *meta)
    }
    
    // Sort by creation time (newest first)
    sort.Slice(backups, func(i, j int) bool {
        return backups[i].CreatedAt.After(backups[j].CreatedAt)
    })
    
    return backups, nil
}
```

## Hook Scripts

### Pre-Backup Hook

```go
// internal/docker/hooks.go

package docker

import (
    "context"
    "fmt"
    
    "github.com/docker/docker/api/types"
)

// execHook executes a hook script in the container
func (a *Application) execHook(ctx context.Context, hookPath string) error {
    // Verify hook exists
    stat, err := a.namespace.client.ContainerStatPath(ctx, 
        a.containerID(), hookPath)
    if err != nil {
        return fmt.Errorf("hook not found: %s", hookPath)
    }
    
    if stat.Mode&0111 == 0 {
        return fmt.Errorf("hook is not executable: %s", hookPath)
    }
    
    // Create exec instance
    execConfig := types.ExecConfig{
        Cmd:          []string{hookPath},
        AttachStdout: true,
        AttachStderr: true,
        User:         "root",  // Run as root for full access
        WorkingDir:   "/storage",
    }
    
    exec, err := a.namespace.client.ContainerExecCreate(ctx, 
        a.containerID(), execConfig)
    if err != nil {
        return err
    }
    
    // Start execution
    resp, err := a.namespace.client.ContainerExecAttach(ctx, 
        exec.ID, types.ExecStartCheck{})
    if err != nil {
        return err
    }
    defer resp.Close()
    
    // Read output
    output, err := io.ReadAll(resp.Reader)
    if err != nil {
        return err
    }
    
    // Check exit code
    inspect, err := a.namespace.client.ContainerExecInspect(ctx, exec.ID)
    if err != nil {
        return err
    }
    
    if inspect.ExitCode != 0 {
        return fmt.Errorf("hook exited with code %d: %s", inspect.ExitCode, string(output))
    }
    
    return nil
}

// hasPreBackupHook checks if the container has a pre-backup hook
func (a *Application) hasPreBackupHook(ctx context.Context) (bool, error) {
    _, err := a.namespace.client.ContainerStatPath(ctx, 
        a.containerID(), "/hooks/pre-backup")
    
    if err != nil {
        if client.IsErrNotFound(err) {
            return false, nil
        }
        return false, err
    }
    
    return true, nil
}
```

### Pre-Backup Hook Examples

```bash
#!/bin/bash
# /hooks/pre-backup for SQLite applications
# This script ensures a consistent database backup

set -e

STORAGE_DIR="/storage"
DB_FILE="$STORAGE_DIR/production.sqlite3"
BACKUP_FILE="$STORAGE_DIR/backup.sqlite3"

if [ -f "$DB_FILE" ]; then
    # Use SQLite's online backup API
    sqlite3 "$DB_FILE" ".backup '$BACKUP_FILE'"
    
    # Also backup WAL files if they exist
    if [ -f "$DB_FILE-wal" ]; then
        cp "$DB_FILE-wal" "$BACKUP_FILE-wal"
    fi
    if [ -f "$DB_FILE-shm" ]; then
        cp "$DB_FILE-shm" "$BACKUP_FILE-shm"
    fi
    
    echo "Database backup completed: $BACKUP_FILE"
else
    echo "No database found at $DB_FILE"
    exit 0
fi
```

```bash
#!/bin/bash
# /hooks/pre-backup for PostgreSQL-connected apps
# Uses pg_dump for consistent backup

set -e

# Wait for any pending writes to complete
sleep 2

# Export database if POSTGRES_URL is set
if [ -n "$POSTGRES_URL" ]; then
    pg_dump "$POSTGRES_URL" > /storage/pg_backup.sql
    echo "PostgreSQL dump completed"
fi

# Sync filesystem to ensure all writes are complete
sync

echo "Pre-backup preparation complete"
```

### Post-Restore Hook

```bash
#!/bin/bash
# /hooks/post-restore for SQLite applications
# This script restores database from backup created by pre-backup

set -e

STORAGE_DIR="/storage"
DB_FILE="$STORAGE_DIR/production.sqlite3"
BACKUP_FILE="$STORAGE_DIR/backup.sqlite3"

if [ -f "$BACKUP_FILE" ]; then
    # Remove current database files
    rm -f "$DB_FILE" "$DB_FILE-wal" "$DB_FILE-shm"
    
    # Restore from backup
    mv "$BACKUP_FILE" "$DB_FILE"
    
    # Also restore WAL files if they exist
    if [ -f "$BACKUP_FILE-wal" ]; then
        mv "$BACKUP_FILE-wal" "$DB_FILE-wal"
    fi
    if [ -f "$BACKUP_FILE-shm" ]; then
        mv "$BACKUP_FILE-shm" "$DB_FILE-shm"
    fi
    
    echo "Database restored from backup"
else
    echo "No backup database found"
    exit 0
fi

# Clean up any stale files
rm -f /storage/*.tmp /storage/*.lock

echo "Post-restore cleanup complete"
```

## Backup Scheduling

### Background Runner

```go
// internal/background/runner.go

package background

import (
    "context"
    "fmt"
    "log/slog"
    "time"
    
    "github.com/basecamp/once/internal/docker"
)

// Runner manages automatic background tasks
type Runner struct {
    namespace string
    logger    *slog.Logger
}

// CheckInterval is how often we check for scheduled tasks
const CheckInterval = 5 * time.Minute

// Run starts the background runner
func (r *Runner) Run(ctx context.Context) error {
    r.logger.Info("starting background runner", "namespace", r.namespace)
    
    ticker := time.NewTicker(CheckInterval)
    defer ticker.Stop()
    
    for {
        select {
        case <-ctx.Done():
            return nil
        case <-ticker.C:
            r.check(ctx)
        }
    }
}

// check runs all scheduled task checks
func (r *Runner) check(ctx context.Context) {
    ns, err := docker.NewNamespace(ctx, r.namespace)
    if err != nil {
        r.logger.Error("failed to get namespace", "error", err)
        return
    }
    
    apps, err := ns.Applications(ctx)
    if err != nil {
        r.logger.Error("failed to list applications", "error", err)
        return
    }
    
    for _, app := range apps {
        // Check for auto-update
        if app.Settings.AutoUpdate {
            if r.isUpdateDue(app) {
                r.logger.Info("update due", "app", app.Settings.Name)
                if err := app.Update(ctx, nil); err != nil {
                    r.logger.Error("auto-update failed", 
                        "app", app.Settings.Name, "error", err)
                }
            }
        }
        
        // Check for auto-backup
        if app.Settings.Backup.AutoBackup {
            if r.isBackupDue(app) {
                r.logger.Info("backup due", "app", app.Settings.Name)
                if err := app.Backup(ctx); err != nil {
                    r.logger.Error("auto-backup failed", 
                        "app", app.Settings.Name, "error", err)
                }
                
                // Trim old backups
                if err := app.TrimBackups(); err != nil {
                    r.logger.Error("backup trim failed", 
                        "app", app.Settings.Name, "error", err)
                }
            }
        }
    }
}

// isUpdateDue checks if an app needs updating
func (r *Runner) isUpdateDue(app *docker.Application) bool {
    // Check image digest against registry
    currentDigest, err := app.GetCurrentImageDigest()
    if err != nil {
        return false
    }
    
    latestDigest, err := app.GetLatestImageDigest()
    if err != nil {
        return false
    }
    
    return currentDigest != latestDigest
}

// isBackupDue checks if a backup is scheduled
func (r *Runner) isBackupDue(app *docker.Application) bool {
    state := app.BackupState()
    
    switch app.Settings.Backup.Frequency {
    case "daily":
        return time.Since(state.LastBackup) > 24*time.Hour
    case "weekly":
        return time.Since(state.LastBackup) > 7*24*time.Hour
    case "monthly":
        return time.Since(state.LastBackup) > 30*24*time.Hour
    default:
        return false
    }
}
```

### Backup Frequency Configuration

```go
// internal/docker/backup_settings.go

package docker

// BackupSettings configures backup behavior
type BackupSettings struct {
    Location    string `json:"location"`     // Directory for backups
    AutoBackup  bool   `json:"auto_backup"`  // Enable automatic backups
    Frequency   string `json:"frequency"`    // daily, weekly, monthly
    Retention   int    `json:"retention"`    // Number of backups to keep
}

// DefaultBackupSettings returns sensible defaults
func DefaultBackupSettings() BackupSettings {
    return BackupSettings{
        Location:   "/var/once/backups",
        AutoBackup: false,
        Frequency:  "daily",
        Retention:  7,  // Keep 7 backups
    }
}

// Validate validates backup settings
func (b *BackupSettings) Validate() error {
    if b.Retention < 1 {
        return fmt.Errorf("retention must be at least 1")
    }
    
    if b.Frequency != "daily" && b.Frequency != "weekly" && b.Frequency != "monthly" {
        return fmt.Errorf("frequency must be daily, weekly, or monthly")
    }
    
    // Check location is writable
    if err := os.MkdirAll(b.Location, 0755); err != nil {
        return fmt.Errorf("backup location not writable: %w", err)
    }
    
    return nil
}
```

## Backup Retention

### Trimming Old Backups

```go
// internal/docker/application_backup.go

// TrimBackups removes old backups beyond retention limit
func (a *Application) TrimBackups() error {
    if a.Settings.Backup.Retention <= 0 {
        return nil  // No trimming if retention is unlimited
    }
    
    // List all backups
    backups, err := ListBackups(a.Settings.Name, a.Settings.Backup.Location)
    if err != nil {
        return err
    }
    
    // Remove backups beyond retention limit
    if len(backups) > a.Settings.Backup.Retention {
        for _, backup := range backups[a.Settings.Backup.Retention:] {
            if err := os.Remove(backup.BackupPath); err != nil {
                return err
            }
            
            // Also remove metadata file
            metaPath := backup.BackupPath + ".meta.json"
            os.Remove(metaPath)  // Ignore errors for metadata
            
            a.logger.Info("trimmed old backup", 
                "app", a.Settings.Name, 
                "path", backup.BackupPath)
        }
    }
    
    return nil
}
```

## Restore Process

### Full Restore Flow

```go
// internal/docker/application_backup.go

// Restore restores an application from a backup archive
func (n *Namespace) Restore(ctx context.Context, backupPath string) (*Application, error) {
    // Open backup file
    backupFile, err := os.Open(backupPath)
    if err != nil {
        return nil, fmt.Errorf("opening backup: %w", err)
    }
    defer backupFile.Close()
    
    // Parse backup archive
    appSettings, volSettings, volumeData, err := parseBackup(backupFile)
    if err != nil {
        return nil, fmt.Errorf("parsing backup: %w", err)
    }
    
    // Generate unique name if app already exists
    name := appSettings.Name
    existing, _ := n.Application(ctx, name)
    if existing != nil {
        name = UniqueName(name)
        appSettings.Name = name
    }
    
    // Create volume with restored settings
    vol, err := n.createVolumeWithSettings(ctx, name, volSettings)
    if err != nil {
        return nil, fmt.Errorf("creating volume: %w", err)
    }
    
    // Restore volume data
    if err := n.restoreVolumeData(ctx, vol.Name, volumeData); err != nil {
        return nil, fmt.Errorf("restoring data: %w", err)
    }
    
    // Create application
    app := NewApplication(n, name)
    app.Settings = *appSettings
    
    // Deploy application
    if err := app.Deploy(ctx, nil); err != nil {
        return nil, fmt.Errorf("deploying app: %w", err)
    }
    
    // Run post-restore hook if exists
    if err := app.execHook(ctx, "/hooks/post-restore"); err != nil {
        n.logger.Warn("post-restore hook failed", 
            "app", app.Settings.Name, "error", err)
    }
    
    return app, nil
}

// parseBackup extracts data from a backup archive
func parseBackup(r io.Reader) (*ApplicationSettings, *ApplicationVolumeSettings, io.Reader, error) {
    gr, err := gzip.NewReader(r)
    if err != nil {
        return nil, nil, nil, fmt.Errorf("decompressing backup: %w", err)
    }
    defer gr.Close()
    
    tr := tar.NewReader(gr)
    
    var appSettings *ApplicationSettings
    var volSettings *ApplicationVolumeSettings
    var dataStartOffset int64
    
    for {
        header, err := tr.Next()
        if err == io.EOF {
            break
        }
        if err != nil {
            return nil, nil, nil, err
        }
        
        switch header.Name {
        case "app-settings.json":
            content, _ := io.ReadAll(tr)
            if err := json.Unmarshal(content, &appSettings); err != nil {
                return nil, nil, nil, fmt.Errorf("parsing app settings: %w", err)
            }
            
        case "vol-settings.json":
            content, _ := io.ReadAll(tr)
            if err := json.Unmarshal(content, &volSettings); err != nil {
                return nil, nil, nil, fmt.Errorf("parsing volume settings: %w", err)
            }
            
        case "data", "data/":
            // Directory entry, skip
            continue
            
        default:
            // This is volume data - reader is now positioned at data
            // Return the reader for streaming copy
            return appSettings, volSettings, tr, nil
        }
    }
    
    return appSettings, volSettings, nil, fmt.Errorf("no volume data found in backup")
}
```

### Restore Verification

```go
// internal/docker/restore_verify.go

package docker

import (
    "archive/tar"
    "compress/gzip"
    "crypto/sha256"
    "fmt"
    "io"
    "os"
)

// VerifyBackup verifies a backup archive's integrity
func VerifyBackup(backupPath string) error {
    // Read expected checksum from metadata
    meta, err := ReadMetadata(backupPath)
    if err != nil {
        return fmt.Errorf("reading metadata: %w", err)
    }
    
    // Calculate actual checksum
    file, err := os.Open(backupPath)
    if err != nil {
        return err
    }
    defer file.Close()
    
    hash := sha256.New()
    if _, err := io.Copy(hash, file); err != nil {
        return err
    }
    
    actualChecksum := fmt.Sprintf("%x", hash.Sum(nil))
    
    if actualChecksum != meta.Checksum {
        return fmt.Errorf("checksum mismatch: expected %s, got %s", 
            meta.Checksum, actualChecksum)
    }
    
    // Verify archive structure
    if err := verifyArchiveStructure(backupPath); err != nil {
        return fmt.Errorf("invalid archive structure: %w", err)
    }
    
    return nil
}

// verifyArchiveStructure checks that required files exist
func verifyArchiveStructure(backupPath string) error {
    file, err := os.Open(backupPath)
    if err != nil {
        return err
    }
    defer file.Close()
    
    gr, err := gzip.NewReader(file)
    if err != nil {
        return err
    }
    defer gr.Close()
    
    tr := tar.NewReader(gr)
    
    hasAppSettings := false
    hasVolSettings := false
    hasData := false
    
    for {
        header, err := tr.Next()
        if err == io.EOF {
            break
        }
        if err != nil {
            return err
        }
        
        switch header.Name {
        case "app-settings.json":
            hasAppSettings = true
        case "vol-settings.json":
            hasVolSettings = true
        default:
            if len(header.Name) > 5 && header.Name[:5] == "data/" {
                hasData = true
            }
        }
    }
    
    if !hasAppSettings || !hasVolSettings || !hasData {
        return fmt.Errorf("missing required files in archive")
    }
    
    return nil
}
```

## Conclusion

ONCE's backup and restore system provides:

1. **Consistent Archives**: Pre-backup hooks ensure data consistency
2. **Compressed Storage**: gzip + tar for efficient storage
3. **Metadata Tracking**: JSON metadata for each backup
4. **Scheduled Backups**: Configurable frequency (daily/weekly/monthly)
5. **Retention Policies**: Automatic trimming of old backups
6. **Hook Scripts**: Pre-backup and post-restore for data integrity
7. **Integrity Verification**: SHA256 checksums for backup validation
8. **Seamless Restore**: Full application restoration with single command
