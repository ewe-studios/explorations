# Aipack -- Initialization

Aipack has a two-phase initialization: base directory setup (`aip init-base` or `aip init`) and workspace setup (`aip init [dir]`). The base directory is global (`~/.aipack-base/`), while the workspace directory is project-local (`.aipack/`).

Source: `aipack/src/exec/init/mod.rs` — init module
Source: `aipack/src/exec/init/init_base.rs` — base directory initialization
Source: `aipack/src/exec/init/init_wks.rs` — workspace initialization
Source: `aipack/src/exec/init/init_assets.rs` — bundled asset extraction
Source: `aipack/src/exec/assets/` — compiled-in assets

## Base Initialization (`init-base`)

```rust
// init_base.rs
pub async fn init_base(force: bool) -> Result<()> {
    // 1. Ensure ~/.aipack-base/ exists
    let base_dir = AipackBaseDir::new()?;
    let new = ensure_dir(base_dir.path())?;

    // 2. Check version.txt for version changes
    let is_new_version = check_is_new_version(&base_dir).await?;
    let force_update = is_new_version || force;

    // 3. Clean legacy content if updating
    if force_update {
        clean_legacy_base_content(&base_dir).await?;
    }

    // 4. Update configuration files
    update_base_configs(&base_dir, force_update)?;

    // 5. Extract built-in packs if updating
    if force_update {
        // Extract installed packs
        let installed_pack_files = extract_base_pack_installed_file_paths()?;
        // For each pack, check if local copy differs from bundled hash
        for pack_folder in packs_to_check {
            let zip_hash = assets::compute_assets_hash("base", &pack_folder)?;
            let local_path = base_dir.join(&pack_folder);
            let local_hash = compute_fs_hash(&local_path)?;

            if local_hash != zip_hash {
                // Delete existing pack, mark for update
                delete_aipack_base_folder(&base_dir, &pack_folder, false, "Built-in pack")?;
                files_to_update.push(...);
            }
        }
        assets::update_files("base", &base_dir, &files_to_update, true).await?;
    }

    // 6. Extract built-in custom packs
    let custom_pack_files = extract_base_pack_custom_file_paths()?;
    assets::update_files("base", &base_dir, &custom_pack_files.x_as_strs(), force_update).await?;
}
```

### Version Detection

```rust
async fn check_is_new_version(base_dir: &SPath) -> Result<bool> {
    let version_path = base_dir.join("version.txt");

    if version_path.exists() {
        let mut reader = simple_fs::get_buf_reader(&version_path)?;
        let mut first_line = String::new();
        if reader.read_line(&mut first_line)? > 0 {
            let version_in_file = first_line.trim();
            return Ok(version_in_file != crate::VERSION);
        }
    }

    Ok(true)  // No version.txt → treat as new install
}
```

The `version.txt` file stores the aipack version. On each `aip` command execution, the version is compared. If it differs (user upgraded aipack), the base directory is updated with new bundled assets.

### Filesystem Hashing

```rust
fn compute_fs_hash(dir_path: &SPath) -> Result<blake3::Hash> {
    let mut files = simple_fs::list_files(dir_path, None, None)?;
    files.sort();  // Deterministic ordering

    let mut hasher = blake3::Hasher::new();
    for file in files {
        let rel_path = file.try_diff(dir_path)?;
        let content = fs::read(file.path())?;
        hasher.update(rel_path.as_str().as_bytes());  // Relative path
        hasher.update(&content);                       // File content
    }
    Ok(hasher.finalize())
}
```

BLAKE3 is used for fast, deterministic hashing of directory contents. The hash includes both relative file paths and content, so any change (addition, deletion, modification) produces a different hash.

## Workspace Initialization (`init`)

```rust
// init_wks.rs
pub async fn init_wks(ref_dir: Option<&str>, show_info_always: bool) -> Result<DirContext> {
    // 1. Determine workspace directory
    let wks_dir = if let Some(dir) = ref_dir {
        SPath::new(dir)
    } else if let Some(path) = find_wks_dir(current_dir()?)? {
        path  // Found existing .aipack/ parent
    } else {
        current_dir()?  // Use current directory
    };

    let wks_dir = wks_dir.canonicalize()?;

    // 2. Compute .aipack/ path
    let aipack_paths = AipackPaths::from_wks_dir(&wks_dir)?;
    let aipack_wks_dir = aipack_paths.aipack_wks_dir()?;

    // 3. Create or refresh workspace files
    create_or_refresh_wks_files(aipack_wks_dir).await?;

    // 4. Return DirContext
    Ok(DirContext::new(aipack_paths)?)
}

async fn create_or_refresh_wks_files(aipack_wks_dir: &AipackWksDir) -> Result<()> {
    ensure_dir(aipack_wks_dir.path())?;

    // Create config.toml if missing
    let config_path = aipack_wks_dir.get_config_toml_path()?;
    if !config_path.exists() {
        let config_zfile = extract_workspace_config_toml_zfile()?;
        write(&config_path, config_zfile.content)?;
    }

    // Note: .aipack/pack/custom/ is not auto-created
    // (users can use their own paths to run agents)
}
```

## Bundled Assets

```rust
// exec/assets/mod.rs — compiled via include_dir! macro
// Bundled assets include:
// - base/pack/installed/ — pre-installed packs
// - base/pack/custom/ — built-in custom packs
// - config-default.toml — default configuration template
// - config-user.toml — user configuration template
// - workspace/config.toml — workspace configuration template
```

Assets are compiled into the binary at build time using `include_dir!`. This ensures aipack works offline after installation — no network access needed for pack extraction.

## Initialization Flow

```
User runs: aip init
  │
  ├── init_wks("dir")
  │   ├── find or create .aipack/
  │   ├── extract config.toml (if missing)
  │   └── return DirContext
  │
  └── (implicitly) init_base(false)
      ├── ensure ~/.aipack-base/
      ├── check version.txt
      ├── update configs (if new version or missing)
      ├── update built-in packs (if version changed)
      └── update built-in custom packs
```

## Config Files

| File | Location | Purpose |
|------|----------|---------|
| `config-default.toml` | `~/.aipack-base/` | Default settings (always updated on version change) |
| `config-user.toml` | `~/.aipack-base/` | User overrides (never overwritten after creation) |
| `config.toml` | `.aipack/` | Workspace-specific config (created on first init) |
| `version.txt` | `~/.aipack-base/` | Tracks aipack version for update detection |

See [Directory Context](07-directory-context.md) for directory structure.
See [Pack System](06-pack-system.md) for pack installation.
