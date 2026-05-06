# Aipack -- Directory Context

Directory Context (`DirContext`) is the central path resolution system in aipack. It manages the two-tier directory hierarchy: the workspace-local `.aipack/` and the user-global `~/.aipack-base/`.

Source: `aipack/src/dir_context/mod.rs` — module organization
Source: `aipack/src/dir_context/dir_context_impl.rs` — DirContext implementation
Source: `aipack/src/dir_context/aipack_paths.rs` — path computation
Source: `aipack/src/dir_context/aipack_wks_dir.rs` — workspace directory type
Source: `aipack/src/dir_context/aipack_base_dir.rs` — base directory type
Source: `aipack/src/dir_context/pack_dir.rs` — pack directory resolution
Source: `aipack/src/dir_context/path_consts.rs` — path constants
Source: `aipack/src/dir_context/path_resolvers.rs` — pack reference resolvers

## Directory Hierarchy

```
~/.aipack-base/                      # Global base directory
├── version.txt                      # Tracks aipack version for update detection
├── config-default.toml              # Default configuration (bundled)
├── config-user.toml                 # User overrides
├── pack/
│   ├── custom/                      # Built-in custom packs (shipped with aipack)
│   └── installed/                   # Installed .aipack packs
│       ├── jc/
│       │   └── coder/
│       │       ├── pack.toml
│       │       └── agents/
│       └── ns/
│           └── pack-name/
│               └── ...

./project/.aipack/                   # Workspace-local directory
├── config.toml                      # Workspace-specific config
└── pack/custom/                     # Workspace custom packs (unpack target)
    └── ...
```

## DirContext Structure

```rust
// dir_context_impl.rs
pub struct DirContext {
    home_dir: SPath,          // User's home directory (~)
    current_dir: SPath,       // Working directory (pwd), canonicalized
    aipack_paths: AipackPaths, // Computed path references
}
```

### AipackPaths

```rust
// aipack_paths.rs
pub struct AipackPaths {
    aipack_base_dir: AipackBaseDir,     // ~/.aipack-base/
    aipack_wks_dir: Option<AipackWksDir>, // ./.aipack/ (None if not initialized)
}

impl AipackPaths {
    // Base directory paths
    pub fn get_base_pack_installed_dir(&self) -> Result<SPath>;  // ~/.aipack-base/pack/installed/
    pub fn get_base_pack_custom_dir(&self) -> Result<SPath>;     // ~/.aipack-base/pack/custom/

    // Workspace directory paths
    pub fn aipack_wks_dir(&self) -> Option<&AipackWksDir>;       // ./.aipack/
    pub fn get_pack_custom_dir(&self) -> Result<SPath>;          // ./.aipack/pack/custom/
    pub fn get_config_toml_path(&self) -> Result<SPath>;         // ./.aipack/config.toml

    // Session-scoped temp directory
    pub fn tmp_dir(&self, session: &Session) -> Option<SPath>;   // ./.aipack/tmp/{session_id}/
}
```

## Path Resolution

```rust
// dir_context_impl.rs
pub fn resolve_path(
    &self,
    session: &Session,
    path: SPath,
    mode: PathResolver,
    base_dir: Option<&SPath>,
) -> Result<SPath> {
    // 1. Expand ~/ to home directory
    let path = if path.starts_with("~/") {
        path.into_replace_prefix("~", self.home_dir())
    } else { path };

    // 2. Absolute paths → return as-is
    if path.is_absolute() {
        return Ok(path);
    }

    // 3. $tmp paths → resolve to session-scoped temp dir
    if self.is_tmp_path(&path) {
        return self.resolve_tmp_path(session, &path);
    }

    // 4. Pack references → resolve to pack directory
    if looks_like_pack_ref(&path) {
        let pack_ref = PackRef::from_str(path.as_str())?;
        let base_path = resolve_pack_ref_base_path(self, &pack_ref)?;
        return Ok(pack_ref.sub_path.map(|p| base_path.join(p)).unwrap_or(base_path));
    }

    // 5. Relative paths → resolve based on mode
    let base = match mode {
        PathResolver::CurrentDir => Some(self.current_dir()),
        PathResolver::WksDir => Some(self.try_wks_dir_with_err_ctx()?),
        PathResolver::AipackDir => Some(self.aipack_paths().aipack_wks_dir()?),
    };
    Ok(base.map(|b| b.join(path)).unwrap_or(path))
}
```

### Resolution Order

```
Input path: "~/project/file.txt"
  → /home/user/project/file.txt   (home expansion)

Input path: "/abs/path/file.txt"
  → /abs/path/file.txt            (absolute, unchanged)

Input path: "$tmp/output.txt"
  → .aipack/tmp/{session}/output.txt  (session-scoped temp)

Input path: "jc@coder/agents/fix.aip"
  → ~/.aipack-base/pack/installed/jc/coder/agents/fix.aip  (pack reference)

Input path: "relative/file.txt" (mode: CurrentDir)
  → {current_dir}/relative/file.txt
```

### Pack Reference Resolution

```rust
// path_resolvers.rs
fn resolve_pack_ref_base_path(dir_context: &DirContext, pack_ref: &PackRef) -> Result<SPath> {
    match &pack_ref.scope {
        // "$base" → ~/.aipack-base/pack/custom/
        Some(Base) => dir_context.aipack_paths().get_base_pack_custom_dir(),
        // "$installed" → ~/.aipack-base/pack/installed/
        Some(Installed) => dir_context.aipack_paths().get_base_pack_installed_dir(),
        // No scope → search order:
        //   1. .aipack/pack/custom/
        //   2. ~/.aipack-base/pack/custom/
        //   3. ~/.aipack-base/pack/installed/
        None => search_pack_dirs(dir_context, &pack_ref.identity),
    }
}
```

The unscoped search order gives workspace custom packs highest priority, then base custom packs, then installed packs.

## Display Path Formatting

```rust
pub fn get_display_path(&self, file_path: &str) -> Result<SPath> {
    let file_path = SPath::new(file_path);

    if file_path.as_str().contains(".aipack-base") {
        Ok(file_path)  // Show absolute for ~/.aipack-base/
    } else {
        // Show relative to workspace
        match self.wks_dir() {
            Some(wks_dir) => file_path.try_diff(wks_dir)?,
            None => file_path,
        }
    }
}
```

Files in `.aipack-base/` are displayed with absolute paths (they're global), while workspace files are shown relative to the workspace root.

## Path Constants

```rust
// path_consts.rs
pub const WORKSPACE_DIR_NAME: &str = ".aipack";
pub const BASE_DIR_NAME: &str = ".aipack-base";
pub const CONFIG_BASE_DEFAULT_FILE_NAME: &str = "config-default.toml";
pub const CONFIG_BASE_USER_FILE_NAME: &str = "config-user.toml";
pub const CONFIG_WKS_FILE_NAME: &str = "config.toml";
pub const PACK_DIR_NAME: &str = "pack";
pub const PACK_CUSTOM_DIR_NAME: &str = "custom";
pub const PACK_INSTALLED_DIR_NAME: &str = "installed";
pub const TMP_DIR_NAME: &str = "tmp";
```

## AipackWksDir Type

```rust
// aipack_wks_dir.rs
pub struct AipackWksDir {
    inner: Arc<SPath>,  // ./.aipack/
}

impl AipackWksDir {
    pub fn get_config_toml_path(&self) -> Result<SPath>;     // .aipack/config.toml
    pub fn get_pack_custom_dir(&self) -> Result<SPath>;      // .aipack/pack/custom/
}
```

A thin wrapper around `.aipack/` that provides type-safe access to workspace-specific sub-paths.

See [Pack System](06-pack-system.md) for pack directory structure.
See [Initialization](16-initialization.md) for directory setup.
