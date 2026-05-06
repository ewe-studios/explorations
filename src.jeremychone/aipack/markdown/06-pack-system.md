# Aipack -- Pack System

Packs are the distribution mechanism for aipack agents. A `.aipack` file is a ZIP archive containing `.aip` agent files, Lua libraries, and a `pack.toml` manifest. Packs can be created, installed, unpacked, and version-managed through the CLI.

Source: `aipack/src/exec/packer/packer_impl.rs` — packing logic
Source: `aipack/src/exec/packer/installer_impl.rs` — installation logic
Source: `aipack/src/exec/packer/unpacker_impl.rs` — unpacking logic
Source: `aipack/src/exec/packer/pack_toml.rs` — pack.toml parsing
Source: `aipack/src/exec/packer/support.rs` — shared utilities

## pack.toml Manifest

Every pack directory must contain a `pack.toml` at its root:

```toml
namespace = "jc"
name = "coder"
version = "0.3.1"
description = "Core coding agents for aipack"
```

| Field | Purpose |
|-------|---------|
| `namespace` | Publisher scope (e.g., "jc" for Jeremy Chone's official packs) |
| `name` | Pack name within the namespace |
| `version` | Semver version string (prereleases must end with `.number`, e.g., `0.3.1-alpha.1`) |
| `description` | Human-readable description |

The file is parsed and validated during packing:

```rust
// pack_toml.rs
fn parse_validate_pack_toml(content: &str, path: &str) -> Result<PackToml> {
    let toml_value: toml::Value = toml::from_str(content)?;
    let namespace = extract_required_field(&toml_value, "namespace")?;
    let name = extract_required_field(&toml_value, "name")?;
    let version = extract_required_field(&toml_value, "version")?;
    let description = extract_optional_field(&toml_value, "description");

    // Validate namespace format (alphanumeric + hyphens)
    validate_namespace(&namespace)?;
    validate_pack_name(&name)?;
    validate_version(&version)?;

    Ok(PackToml { namespace, name, version, description })
}
```

## Packing (CmdPack)

```rust
// packer_impl.rs
fn pack_dir(pack_dir: &SPath, dest_dir: &SPath) -> Result<PackDirData> {
    // 1. Verify pack.toml exists
    let toml_path = pack_dir.join("pack.toml");
    if !toml_path.exists() {
        return Err(Error::AipackTomlMissing(toml_path));
    }

    // 2. Read and validate pack.toml
    let content = fs::read_to_string(&toml_path)?;
    let pack_toml = parse_validate_pack_toml(&content, toml_path.as_str())?;

    // 3. Build filename: {namespace}@{name}-v{version}.aipack
    let filename = format!("{}@{}-v{}.aipack", pack_toml.namespace, pack_toml.name, pack_toml.version);
    let aipack_path = dest_dir.join(filename);

    // 4. Ensure dest directory exists
    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir)?;
    }

    // 5. ZIP the directory
    zip::zip_dir(pack_dir, &aipack_path)?;

    Ok(PackDirData { pack_file: aipack_path, pack_toml })
}
```

The output filename follows the convention `{namespace}@{name}-v{version}.aipack`, making it immediately identifiable by namespace, name, and version.

## Installation (CmdInstall)

```rust
// installer_impl.rs
pub async fn install_pack(dir_context: &DirContext, pack_uri: &str, force: bool) -> Result<InstallResponse> {
    let pack_uri = PackUri::parse(pack_uri);

    // 1. Resolve source: local path, repo pack, or HTTP link
    let (aipack_zipped_file, pack_uri) = match pack_uri {
        PackUri::LocalPath(_) => resolve_local_path(dir_context, pack_uri)?,
        PackUri::RepoPack(_) => download_from_repo(dir_context, pack_uri).await?,
        PackUri::HttpLink(_) => download_pack(dir_context, pack_uri).await?,
    };

    // 2. Validate .aipack extension
    validate_aipack_file(&aipack_zipped_file, &pack_uri.to_string())?;

    // 3. Extract pack.toml from ZIP
    let new_pack_toml = extract_pack_toml_from_pack_file(&aipack_zipped_file)?;

    // 4. Validate prerelease format
    validate_version_for_install(&new_pack_toml.version)?;

    // 5. Check existing installation
    let existing_path = pack_installed_dir.join(&new_pack_toml.namespace).join(&new_pack_toml.name);
    if existing_path.exists() && !force {
        let existing_toml = parse_existing_pack_toml(&existing_path)?;
        if let Some(existing_toml) = existing_toml {
            let ord = validate_version_update(&existing_toml.version, &new_pack_toml.version)?;
            match ord {
                Ordering::Equal => return Ok(InstallResponse::UpToDate(installed_pack)),
                Ordering::Less => return Err(Error::InstallFailInstalledVersionAbove { ... }),
                Ordering::Greater => {} // proceed with update
            }
        }
    }

    // 6. Trash existing directory if updating
    if existing_path.exists() {
        safer_trash_dir(&existing_path, Some(DeleteCheck::CONTAINS_AIPACK_BASE))?;
    }

    // 7. Unzip to pack_installed_dir/namespace/name/
    zip::unzip_file(&aipack_zipped_file, &pack_target_dir)?;

    // 8. If downloaded, trash the temp ZIP
    if was_downloaded {
        safer_trash_file(&aipack_zipped_file, Some(DeleteCheck::CONTAINS_AIPACK_BASE))?;
    }

    Ok(InstallResponse::Installed(InstalledPack { ... }))
}
```

### InstallResponse

```rust
pub enum InstallResponse {
    Installed(InstalledPack),  // new or updated
    UpToDate(InstalledPack),   // same version already installed
}

pub struct InstalledPack {
    pub pack_toml: PackToml,
    pub path: SPath,
    pub size: usize,       // unpacked directory size
    pub zip_size: usize,   // original zip size
}
```

### PackUri Types

```rust
enum PackUri {
    LocalPath(String),   // "./my-pack/" or "path/to/pack.aipack"
    RepoPack(PackIdentity),  // "jc@coder" — downloads from configured repo
    HttpLink(String),    // "https://example.com/pack.aipack"
}
```

### Version Comparison Logic

```rust
// support.rs
fn validate_version_update(existing: &str, new: &str) -> Result<Ordering> {
    let existing_semver = Version::parse(existing).map_err(|e| ...)?;
    let new_semver = Version::parse(new).map_err(|e| ...)?;
    Ok(new_semver.cmp(&existing_semver))
}
```

If the installed version is newer than the incoming version, installation is rejected with `InstallFailInstalledAbove`. This prevents accidental downgrades. The `--force` flag bypasses this check.

### Prerelease Validation

```rust
fn validate_version_for_install(version: &str) -> Result<()> {
    let semver = Version::parse(version)?;
    if let Some(pre) = semver.pre {
        // Must end with .number (e.g., -alpha.1, -beta.2)
        // Rejects -alpha, -dev, -SNAPSHOT
        let parts: Vec<&str> = pre.split('.').collect();
        if parts.last().and_then(|s| s.parse::<u64>().ok()).is_none() {
            return Err(Error::InvalidPrereleaseFormat { version: version.into() });
        }
    }
    Ok(())
}
```

## Unpacking (CmdUnpack)

Unpacking copies a pack into the workspace's `.aipack/pack/custom/` directory for local customization:

```rust
// unpacker_impl.rs
pub async fn unpack_pack(dir_context: &DirContext, pack_ref: &str, force: bool) -> Result<UnpackedPack> {
    // 1. Parse pack identity (must be plain namespace@name, no sub-path or scope)
    let identity = PackIdentity::from_str(pack_ref)?;
    if pack_ref.contains('/') || pack_ref.contains('$') {
        return Err(Error::custom("Unpack requires plain 'namespace@name'"));
    }

    // 2. Ensure workspace .aipack/ exists
    let aipack_wks_dir = dir_context.aipack_paths().aipack_wks_dir()?;

    // 3. Compute destination: .aipack/pack/custom/namespace/name/
    let dest_dir = aipack_wks_dir.get_pack_custom_dir()?.join(&identity.namespace).join(&identity.name);

    // 4. Determine source: installed vs remote
    let installed_version = read_installed_version(&installed_dir);
    let remote_version = fetch_repo_latest_version(&identity).await?;
    let source = determine_source(&installed_version, &remote_version, Some(&installed_dir));

    // 5. Perform unpack
    match source {
        UnpackSource::Installed(path) => copy_dir_recursive(&path, &dest_dir)?,
        UnpackSource::Remote => {
            let zip = download_from_repo(dir_context, PackUri::RepoPack(identity)).await?;
            zip::unzip_file(&zip.0, &dest_dir)?;
        }
    }

    Ok(UnpackedPack { namespace, name, dest_path, source })
}
```

### Source Selection Logic

```
┌─────────────────────────────────────────────────────────────┐
│ determine_source(installed_ver, remote_ver, installed_dir)  │
├─────────────────────────────────────────────────────────────┤
│ installed exists + remote exists:                           │
│   remote > installed → Remote (download newer)              │
│   remote <= installed → Installed (prefer local)            │
│ installed exists + no remote info → Installed               │
│ installed exists + no version + remote → Remote (freshness) │
│ nothing installed → Remote                                  │
└─────────────────────────────────────────────────────────────┘
```

The unpacker prefers local copies when versions are equal (faster, no network), downloads when remote is newer, and falls back to remote when nothing is installed.

## Pack Listing (CmdList)

```bash
aip list                     # list all installed packs
aip list --json              # JSON output
```

The list command scans `.aipack-base/pack/installed/` for `pack.toml` files and displays namespace, name, version, and description.

## File Safety

All destructive pack operations use `safer_trash_dir` and `safer_trash_file` instead of direct deletion. These functions perform a safety check (`DeleteCheck::CONTAINS_AIPACK` or `CONTAINS_AIPACK_BASE`) to ensure the path is within the expected aipack directory hierarchy before trashing.

See [Directory Context](07-directory-context.md) for pack directory paths.
See [Initialization](16-initialization.md) for bundled pack extraction.
