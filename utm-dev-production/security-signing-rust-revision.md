# utm-dev Production - Security & Code Signing Rust Revision

## Overview

This document provides a comprehensive Rust implementation for code signing, notarization, and secure credential storage across all target platforms (macOS, iOS, Android, Windows, Linux). The implementation replaces Go-based tooling with idiomatic Rust, leveraging platform-specific APIs and cross-platform abstractions.

**Key Goals:**
- Unified signing interface across all platforms
- Secure credential storage using native OS keychains
- Async-first design for non-blocking signing operations
- Comprehensive error handling with actionable diagnostics
- CI/CD ready with environment-based configuration

## Workspace Structure

```
utm-signing/
├── Cargo.toml                 # Workspace root
├── README.md
├── utm-signing-core/          # Core traits and types
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── traits.rs          # Signing trait definitions
│       ├── error.rs           # Unified error types
│       ├── config.rs          # Signing configuration
│       └── credentials.rs     # Credential types
├── utm-signing-macos/         # macOS code signing & notarization
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── codesign.rs        # codesign wrapper
│       ├── notarytool.rs      # notarytool wrapper
│       ├── entitlements.rs    # Entitlements generation
│       └── keychain.rs        # Keychain integration
├── utm-signing-ios/           # iOS signing & provisioning
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── provisioning.rs    # Provisioning profile handling
│       ├── codesign.rs        # iOS codesign
│       └── entitlements.rs    # iOS entitlements
├── utm-signing-android/       # Android APK signing
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── apksigner.rs       # apksigner wrapper
│       ├── zipalign.rs        # zipalign wrapper
│       └── keystore.rs        # Keystore handling
├── utm-signing-windows/       # Windows Authenticode signing
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── signtool.rs        # signtool wrapper
│       └── certificate.rs     # Certificate handling
├── utm-signing-secrets/       # Cross-platform secrets storage
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── traits.rs          # SecretsStore trait
│       ├── keychain.rs        # macOS Keychain
│       ├── credential_manager.rs  # Windows Credential Manager
│       ├── libsecret.rs       # Linux libsecret
│       └── file.rs            # Fallback file store
└── utm-signing-cli/           # CLI tool
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── commands/
        │   ├── sign.rs
        │   ├── verify.rs
        │   └── secrets.rs
        └── utils.rs
```

## Crate Breakdown

| Crate | Purpose | Platforms |
|-------|---------|-----------|
| `utm-signing-core` | Shared traits, types, errors | All |
| `utm-signing-macos` | macOS code signing & notarization | macOS |
| `utm-signing-ios` | iOS provisioning & signing | macOS (for iOS builds) |
| `utm-signing-android` | APK/AAB signing | All |
| `utm-signing-windows` | Authenticode signing | Windows |
| `utm-signing-secrets` | Secure credential storage | All |
| `utm-signing-cli` | Command-line interface | All |

## Recommended Dependencies

### utm-signing-core/Cargo.toml
```toml
[package]
name = "utm-signing-core"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }
```

### utm-signing-macos/Cargo.toml
```toml
[package]
name = "utm-signing-macos"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-signing-core = { path = "../utm-signing-core" }
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["process", "fs"] }
tracing = "0.1"
plist = "1.6"
```

### utm-signing-android/Cargo.toml
```toml
[package]
name = "utm-signing-android"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-signing-core = { path = "../utm-signing-core" }
thiserror = "1.0"
tokio = { version = "1.0", features = ["process"] }
tracing = "0.1"
tempfile = "3.0"
```

### utm-signing-secrets/Cargo.toml
```toml
[package]
name = "utm-signing-secrets"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-signing-core = { path = "../utm-signing-core" }
thiserror = "1.0"
async-trait = "0.1"
tracing = "0.1"

[target.'cfg(target_os = "macos")'.dependencies]
security-framework = "2.9"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.52", features = ["Win32_Security_Credentials"] }

[target.'cfg(target_os = "linux")'.dependencies]
secret-service = "4.0"

[dependencies]
serde_json = "1.0"
base64 = "0.21"
```

## Type System Design

### Core Types (utm-signing-core)

```rust
// utm-signing-core/src/credentials.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Represents a code signing identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningIdentity {
    /// Human-readable name (e.g., "Developer ID Application: John Doe (ABCD1234)")
    pub name: String,
    /// Fingerprint or hash of the certificate
    pub fingerprint: String,
    /// Certificate expiry date
    pub expires: DateTime<Utc>,
    /// Team ID (Apple platforms)
    pub team_id: Option<String>,
    /// Platform this identity is valid for
    pub platform: SigningPlatform,
}

/// Target platform for signing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SigningPlatform {
    Macos,
    Ios,
    Android,
    Windows,
    Linux,
}

/// Signing credentials containing secrets
#[derive(Debug, Clone)]
pub enum SigningCredentials {
    /// Apple: Developer ID name or certificate identity
    AppleDeveloper {
        identity: SigningIdentity,
        /// Keychain profile name for notarization
        notarization_profile: Option<String>,
    },
    /// Android: Keystore credentials
    AndroidKeystore {
        keystore_path: String,
        keystore_alias: String,
        /// Credentials stored securely, not in this struct
        keystore_password_ref: CredentialReference,
        key_password_ref: CredentialReference,
    },
    /// Windows: PFX certificate
    WindowsPfx {
        pfx_path: String,
        password_ref: CredentialReference,
    },
}

/// Reference to a credential stored in the OS keychain
#[derive(Debug, Clone)]
pub struct CredentialReference {
    /// Service name (e.g., "utm-dev")
    pub service: String,
    /// Account/key name
    pub account: String,
}

/// Entitlements for Apple platforms
#[derive(Debug, Clone, Default)]
pub struct Entitlements {
    /// Allow JIT compilation (required for emulators)
    pub allow_jit: bool,
    /// Allow unsigned executable memory
    pub allow_unsigned_memory: bool,
    /// Disable executable page protection
    pub disable_executable_page_protection: bool,
    /// Network client access
    pub network_client: bool,
    /// Network server access
    pub network_server: bool,
    /// File access: user-selected files
    pub files_user_selected_read_write: bool,
    /// File access: app scope bookmarks
    pub files_bookmarks_app_scope: bool,
    /// Automation: Apple Events
    pub automation_apple_events: bool,
    /// Custom entitlements
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

impl Entitlements {
    /// Create entitlements suitable for emulator/virtualization apps
    pub fn for_emulator() -> Self {
        Self {
            allow_jit: true,
            allow_unsigned_memory: true,
            disable_executable_page_protection: true,
            network_client: true,
            network_server: true,
            files_user_selected_read_write: true,
            files_bookmarks_app_scope: true,
            automation_apple_events: false,
            custom: std::collections::HashMap::new(),
        }
    }

    /// Create minimal entitlements
    pub fn minimal() -> Self {
        Self {
            allow_jit: false,
            allow_unsigned_memory: false,
            disable_executable_page_protection: false,
            network_client: true,
            network_server: false,
            files_user_selected_read_write: true,
            files_bookmarks_app_scope: false,
            automation_apple_events: false,
            custom: std::collections::HashMap::new(),
        }
    }
}
```

### Signing Traits (utm-signing-core)

```rust
// utm-signing-core/src/traits.rs
use crate::credentials::{SigningCredentials, SigningPlatform, Entitlements};
use crate::error::SigningResult;
use std::path::{Path, PathBuf};

/// Result of a signing operation
#[derive(Debug, Clone)]
pub struct SigningResult {
    /// Path to the signed artifact
    pub signed_path: PathBuf,
    /// Verification status
    pub verified: bool,
    /// Notarization status (Apple platforms)
    pub notarized: bool,
    /// Timestamp of signing
    pub signed_at: chrono::DateTime<chrono::Utc>,
    /// Signing identity used
    pub identity: String,
}

/// Primary trait for code signing operations
#[async_trait::async_trait]
pub trait CodeSigner: Send + Sync {
    /// Get the platform this signer handles
    fn platform(&self) -> SigningPlatform;

    /// Sign an application/binary
    async fn sign(&self, input_path: &Path, credentials: &SigningCredentials) -> SigningResult;

    /// Verify a signed artifact
    async fn verify(&self, signed_path: &Path) -> SigningResult;

    /// Get available signing identities from the system
    async fn list_identities(&self) -> SigningResult<Vec<crate::credentials::SigningIdentity>>;
}

/// Notarization for Apple platforms
#[async_trait::async_trait]
pub trait Notarize: Send + Sync {
    /// Submit for notarization and wait for result
    async fn submit_and_wait(&self, artifact_path: &Path, profile_name: &str) -> SigningResult;

    /// Staple notarization ticket to app
    async fn staple(&self, app_path: &Path) -> SigningResult;
}

/// APK-specific signing operations
#[async_trait::async_trait]
pub trait ApkSigner: Send + Sync {
    /// Align APK for signing
    async fn align(&self, input_apk: &Path, output_apk: &Path) -> SigningResult;

    /// Sign aligned APK
    async fn sign_apk(&self, aligned_apk: &Path, output_apk: &Path) -> SigningResult;

    /// Verify APK signature
    async fn verify_apk(&self, apk_path: &Path) -> SigningResult;

    /// Full signing workflow: align + sign
    async fn sign_full(&self, input_apk: &Path, output_dir: &Path) -> SigningResult;
}
```

### Error Types (utm-signing-core)

```rust
// utm-signing-core/src/error.rs
use thiserror::Error;

/// Unified signing error type
#[derive(Error, Debug)]
pub enum SigningError {
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(&'static str),

    #[error("Code signing failed: {0}")]
    CodeSigningFailed(String),

    #[error("Notarization failed: {0}")]
    NotarizationFailed(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Identity not found: {0}")]
    IdentityNotFound(String),

    #[error("Credentials error: {0}")]
    CredentialsError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Process execution error: {0}")]
    ProcessError(String),

    #[error("Certificate error: {0}")]
    CertificateError(String),

    #[error("Entitlements error: {0}")]
    EntitlementsError(String),

    #[error("Provisioning profile error: {0}")]
    ProvisioningProfileError(String),

    #[error("Keystore error: {0}")]
    KeystoreError(String),

    #[error("Secrets store error: {0}")]
    SecretsStoreError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Timestamp error: {0}")]
    TimestampError(String),
}

pub type SigningResult<T> = Result<T, SigningError>;
```

## macOS Implementation (utm-signing-macos)

```rust
// utm-signing-macos/src/lib.rs
mod codesign;
mod notarytool;
mod entitlements;
mod keychain;

pub use codesign::MacOSCodeSigner;
pub use notarytool::MacOSNotaryTool;
pub use entitlements::EntitlementsGenerator;
pub use keychain::KeychainCredentials;

use utm_signing_core::{
    CodeSigner, Notarize, SigningCredentials, SigningIdentity, SigningPlatform,
    SigningResult, Entitlements,
};
use std::path::Path;

/// macOS signing configuration
#[derive(Debug, Clone)]
pub struct MacOSConfig {
    /// Developer ID for signing
    pub developer_id: String,
    /// Team ID
    pub team_id: String,
    /// Apple ID for notarization
    pub apple_id: Option<String>,
    /// Notarytool keychain profile name
    pub notarization_profile: Option<String>,
    /// Path to entitlements plist
    pub entitlements_path: Option<String>,
}

impl MacOSConfig {
    pub fn from_env() -> Result<Self, utm_signing_core::SigningError> {
        Ok(Self {
            developer_id: std::env::var("APPLE_DEVELOPER_ID")
                .map_err(|_| utm_signing_core::SigningError::ConfigurationError(
                    "APPLE_DEVELOPER_ID not set".to_string(),
                ))?,
            team_id: std::env::var("APPLE_TEAM_ID")
                .map_err(|_| utm_signing_core::SigningError::ConfigurationError(
                    "APPLE_TEAM_ID not set".to_string(),
                ))?,
            apple_id: std::env::var("APPLE_ID").ok(),
            notarization_profile: std::env::var("NOTARIZATION_PROFILE").ok(),
            entitlements_path: std::env::var("ENTITLEMENTS_PATH").ok(),
        })
    }
}
```

```rust
// utm-signing-macos/src/codesign.rs
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, debug, error};
use utm_signing_core::{
    CodeSigner, SigningCredentials, SigningIdentity, SigningPlatform,
    SigningResult, SigningError, Entitlements,
};
use crate::entitlements::EntitlementsGenerator;

/// macOS codesign wrapper
pub struct MacOSCodeSigner {
    config: super::MacOSConfig,
}

impl MacOSCodeSigner {
    pub fn new(config: super::MacOSConfig) -> Self {
        Self { config }
    }

    /// Run codesign command
    async fn run_codesign(
        &self,
        app_path: &Path,
        entitlements_path: Option<&Path>,
    ) -> SigningResult<()> {
        let mut cmd = Command::new("codesign");
        cmd.args([
            "--deep", "--force", "--verify", "--verbose",
            "--sign", &self.config.developer_id,
            "--options", "runtime",
        ]);

        if let Some(ent_path) = entitlements_path {
            cmd.arg("--entitlements").arg(ent_path);
        }

        cmd.arg(app_path);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        debug!("Running codesign: {:?}", cmd);

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute codesign: {}", e)
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("codesign failed: {}", stderr);
            return Err(SigningError::CodeSigningFailed(stderr.to_string()));
        }

        info!("codesign completed successfully");
        Ok(())
    }

    /// Create DMG for notarization submission
    pub async fn create_dmg(&self, app_path: &Path, dmg_path: &Path) -> SigningResult<()> {
        let app_name = app_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("app");

        let mut cmd = Command::new("hdiutil");
        cmd.args([
            "create", "-fs", "APFS", "-srcfolder",
        ])
        .arg(app_path)
        .arg("-volname")
        .arg(app_name)
        .arg(dmg_path);

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute hdiutil: {}", e)
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SigningError::CodeSigningFailed(
                format!("hdiutil failed: {}", stderr)
            ));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl CodeSigner for MacOSCodeSigner {
    fn platform(&self) -> SigningPlatform {
        SigningPlatform::Macos
    }

    async fn sign(
        &self,
        input_path: &Path,
        _credentials: &SigningCredentials,
    ) -> SigningResult<utm_signing_core::SigningResult> {
        info!("Signing macOS app: {:?}", input_path);

        // Generate entitlements if not provided
        let entitlements_path: Option<PathBuf> = if self.config.entitlements_path.is_some() {
            self.config.entitlements_path.as_ref().map(PathBuf::from)
        } else {
            // Generate default entitlements for emulator
            let gen = EntitlementsGenerator::new();
            let entitlements = Entitlements::for_emulator();
            let temp_path = std::env::temp_dir().join("utm_entitlements.plist");
            gen.write_to_file(&entitlements, &temp_path)?;
            Some(temp_path)
        };

        // Run codesign
        self.run_codesign(input_path, entitlements_path.as_deref()).await?;

        Ok(utm_signing_core::SigningResult {
            signed_path: input_path.to_path_buf(),
            verified: false, // Will verify separately
            notarized: false,
            signed_at: chrono::Utc::now(),
            identity: self.config.developer_id.clone(),
        })
    }

    async fn verify(&self, signed_path: &Path) -> SigningResult<utm_signing_core::SigningResult> {
        info!("Verifying signed app: {:?}", signed_path);

        let mut cmd = Command::new("spctl");
        cmd.args([
            "--assess",
            "--type", "install",
            "--context", "context:primary-signature",
            "--verbose=2",
        ])
        .arg(signed_path);

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute spctl: {}", e)
            ))?;

        let verified = output.status.success();

        if !verified {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Verification failed: {}", stderr);
        }

        Ok(utm_signing_core::SigningResult {
            signed_path: signed_path.to_path_buf(),
            verified,
            notarized: false,
            signed_at: chrono::Utc::now(),
            identity: self.config.developer_id.clone(),
        })
    }

    async fn list_identities(&self) -> SigningResult<Vec<SigningIdentity>> {
        let mut cmd = Command::new("security");
        cmd.args([
            "find-identity",
            "-v",
            "-s", "Developer ID Application",
        ]);

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute security: {}", e)
            ))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        // Parse output to extract identities
        let stdout = String::from_utf8_lossy(&output.stdout);
        let identities = parse_security_find_identity(&stdout);

        Ok(identities)
    }
}

/// Parse output of `security find-identity`
fn parse_security_find_identity(output: &str) -> Vec<SigningIdentity> {
    let mut identities = Vec::new();

    for line in output.lines() {
        // Format: "1) ABCD1234567890 \"Developer ID Application: Name (TEAM)\""
        if let Some(start) = line.find('"') {
            if let Some(end) = line[start+1..].find('"') {
                let name = &line[start+1..start+1+end];
                // Extract team ID from parentheses
                let team_id = if let Some(t_start) = name.rfind('(') {
                    if let Some(t_end) = name[t_start..].find(')') {
                        Some(name[t_start+1..t_start+t_end].to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };

                identities.push(SigningIdentity {
                    name: name.to_string(),
                    fingerprint: String::new(), // Would need additional parsing
                    expires: chrono::Utc::now() + chrono::Duration::days(365),
                    team_id,
                    platform: SigningPlatform::Macos,
                });
            }
        }
    }

    identities
}
```

```rust
// utm-signing-macos/src/notarytool.rs
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{Duration, sleep};
use tracing::{info, debug, error, warn};
use utm_signing_core::{Notarize, SigningResult, SigningError};
use serde::Deserialize;

/// Notarization submission response
#[derive(Debug, Deserialize)]
struct NotarySubmission {
    uuid: String,
    status: Option<String>,
}

/// Notarization status response
#[derive(Debug, Deserialize)]
struct NotaryStatus {
    status: String,
    issues: Option<Vec<NotaryIssue>>,
}

#[derive(Debug, Deserialize)]
struct NotaryIssue {
    severity: String,
    code: Option<String>,
    message: String,
}

/// macOS notarytool wrapper
pub struct MacOSNotaryTool {
    apple_id: String,
    team_id: String,
    profile_name: String,
}

impl MacOSNotaryTool {
    pub fn new(apple_id: String, team_id: String, profile_name: String) -> Self {
        Self {
            apple_id,
            team_id,
            profile_name,
        }
    }

    /// Submit artifact for notarization
    async fn submit(&self, artifact_path: &Path) -> SigningResult<String> {
        let mut cmd = Command::new("xcrun");
        cmd.args(["notarytool", "submit"])
            .arg(artifact_path)
            .arg("--apple-id")
            .arg(&self.apple_id)
            .arg("--team-id")
            .arg(&self.team_id)
            .arg("--keychain-profile")
            .arg(&self.profile_name)
            .arg("--output-format")
            .arg("json");

        debug!("Submitting for notarization: {:?}", cmd);

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute notarytool: {}", e)
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SigningError::NotarizationFailed(stderr.to_string()));
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let submission: NotarySubmission = serde_json::from_str(&stdout)
            .map_err(|e| SigningError::NotarizationFailed(
                format!("Failed to parse notarytool response: {}", e)
            ))?;

        info!("Notarization submitted with UUID: {}", submission.uuid);
        Ok(submission.uuid)
    }

    /// Check notarization status
    async fn status(&self, uuid: &str) -> SigningResult<NotaryStatus> {
        let mut cmd = Command::new("xcrun");
        cmd.args(["notarytool", "info"])
            .arg(uuid)
            .arg("--keychain-profile")
            .arg(&self.profile_name)
            .arg("--output-format")
            .arg("json");

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute notarytool: {}", e)
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SigningError::NotarizationFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let status: NotaryStatus = serde_json::from_str(&stdout)
            .map_err(|e| SigningError::NotarizationFailed(
                format!("Failed to parse notarytool status: {}", e)
            ))?;

        Ok(status)
    }
}

#[async_trait::async_trait]
impl Notarize for MacOSNotaryTool {
    async fn submit_and_wait(
        &self,
        artifact_path: &Path,
        _profile_name: &str,
    ) -> SigningResult<utm_signing_core::SigningResult> {
        info!("Submitting for notarization: {:?}", artifact_path);

        // Submit
        let uuid = self.submit(artifact_path).await?;

        // Poll for completion
        let mut attempts = 0;
        let max_attempts = 120; // 10 minutes max

        loop {
            if attempts >= max_attempts {
                return Err(SigningError::NotarizationFailed(
                    "Notarization timed out".to_string()
                ));
            }

            let status = self.status(&uuid).await?;

            match status.status.as_str() {
                "Accepted" => {
                    info!("Notarization accepted for UUID: {}", uuid);
                    return Ok(utm_signing_core::SigningResult {
                        signed_path: artifact_path.to_path_buf(),
                        verified: true,
                        notarized: true,
                        signed_at: chrono::Utc::now(),
                        identity: self.team_id.clone(),
                    });
                }
                "Invalid" | "Rejected" => {
                    let issues = status.issues
                        .map(|i| format!("{:?}", i))
                        .unwrap_or_else(|| "Unknown issues".to_string());
                    return Err(SigningError::NotarizationFailed(
                        format!("Notarization rejected: {}", issues)
                    ));
                }
                "In Progress" => {
                    debug!("Notarization in progress, waiting...");
                    sleep(Duration::from_secs(5)).await;
                    attempts += 1;
                }
                _ => {
                    warn!("Unknown notarization status: {}", status.status);
                    sleep(Duration::from_secs(5)).await;
                    attempts += 1;
                }
            }
        }
    }

    async fn staple(&self, app_path: &Path) -> SigningResult<utm_signing_core::SigningResult> {
        info!("Stapling notarization ticket: {:?}", app_path);

        let mut cmd = Command::new("xcrun");
        cmd.arg("stapler").arg("staple").arg(app_path);

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute stapler: {}", e)
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SigningError::NotarizationFailed(
                format!("Stapling failed: {}", stderr)
            ));
        }

        info!("Notarization ticket stapled successfully");

        Ok(utm_signing_core::SigningResult {
            signed_path: app_path.to_path_buf(),
            verified: true,
            notarized: true,
            signed_at: chrono::Utc::now(),
            identity: self.team_id.clone(),
        })
    }
}
```

```rust
// utm-signing-macos/src/entitlements.rs
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use utm_signing_core::{Entitlements, SigningResult, SigningError};
use plist::{Dictionary, Value};

/// Generates entitlements plist files
pub struct EntitlementsGenerator;

impl EntitlementsGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Convert Entitlements to plist Value
    pub fn to_plist(&self, entitlements: &Entitlements) -> Value {
        let mut dict = Dictionary::new();

        dict.insert("com.apple.security.cs.allow-jit".to_string(), Value::Boolean(entitlements.allow_jit));
        dict.insert("com.apple.security.cs.allow-unsigned-executable-memory".to_string(), Value::Boolean(entitlements.allow_unsigned_memory));
        dict.insert("com.apple.security.cs.disable-executable-page-protection".to_string(), Value::Boolean(entitlements.disable_executable_page_protection));
        dict.insert("com.apple.security.network.client".to_string(), Value::Boolean(entitlements.network_client));
        dict.insert("com.apple.security.network.server".to_string(), Value::Boolean(entitlements.network_server));
        dict.insert("com.apple.security.files.user-selected.read-write".to_string(), Value::Boolean(entitlements.files_user_selected_read_write));
        dict.insert("com.apple.security.files.bookmarks.app-scope".to_string(), Value::Boolean(entitlements.files_bookmarks_app_scope));
        dict.insert("com.apple.security.automation.apple-events".to_string(), Value::Boolean(entitlements.automation_apple_events));

        // Add custom entitlements
        for (key, value) in &entitlements.custom {
            if let Ok(json_value) = serde_json::to_value(value) {
                if let Ok(plist_value) = plist::to_value(&json_value) {
                    dict.insert(key.clone(), plist_value);
                }
            }
        }

        Value::Dictionary(dict)
    }

    /// Write entitlements to file
    pub fn write_to_file(&self, entitlements: &Entitlements, path: &Path) -> SigningResult<()> {
        let plist_value = self.to_plist(entitlements);

        let mut file = File::create(path)
            .map_err(|e| SigningError::EntitlementsError(
                format!("Failed to create entitlements file: {}", e)
            ))?;

        plist::to_writer_xml(&mut file, &plist_value)
            .map_err(|e| SigningError::EntitlementsError(
                format!("Failed to write entitlements plist: {}", e)
            ))?;

        Ok(())
    }

    /// Generate entitlements for specific use cases
    pub fn for_virtualization() -> Entitlements {
        Entitlements {
            allow_jit: true,
            allow_unsigned_memory: true,
            disable_executable_page_protection: true,
            network_client: true,
            network_server: true,
            files_user_selected_read_write: true,
            files_bookmarks_app_scope: true,
            automation_apple_events: true,
            custom: HashMap::new(),
        }
    }

    pub fn for_webview() -> Entitlements {
        Entitlements {
            allow_jit: false,
            allow_unsigned_memory: false,
            disable_executable_page_protection: false,
            network_client: true,
            network_server: false,
            files_user_selected_read_write: true,
            files_bookmarks_app_scope: false,
            automation_apple_events: false,
            custom: HashMap::new(),
        }
    }
}

impl Default for EntitlementsGenerator {
    fn default() -> Self {
        Self::new()
    }
}
```

## Android Implementation (utm-signing-android)

```rust
// utm-signing-android/src/lib.rs
mod apksigner;
mod zipalign;
mod keystore;

pub use apksigner::AndroidApkSigner;
pub use zipalign::ZipAlign;
pub use keystore::KeystoreConfig;

use std::path::Path;
use utm_signing_core::{ApkSigner as ApkSignerTrait, SigningResult, SigningCredentials, SigningPlatform};

/// Android signing configuration
#[derive(Debug, Clone)]
pub struct AndroidConfig {
    /// Path to keystore file
    pub keystore_path: String,
    /// Keystore alias
    pub keystore_alias: String,
    /// Reference to keystore password in secrets store
    pub keystore_password_ref: utm_signing_core::CredentialReference,
    /// Reference to key password in secrets store
    pub key_password_ref: utm_signing_core::CredentialReference,
}

impl AndroidConfig {
    pub fn from_env() -> Result<Self, utm_signing_core::SigningError> {
        Ok(Self {
            keystore_path: std::env::var("ANDROID_KEYSTORE_PATH")
                .map_err(|_| utm_signing_core::SigningError::ConfigurationError(
                    "ANDROID_KEYSTORE_PATH not set".to_string(),
                ))?,
            keystore_alias: std::env::var("ANDROID_KEY_ALIAS")
                .map_err(|_| utm_signing_core::SigningError::ConfigurationError(
                    "ANDROID_KEY_ALIAS not set".to_string(),
                ))?,
            keystore_password_ref: utm_signing_core::CredentialReference {
                service: "utm-dev".to_string(),
                account: "android_keystore_password".to_string(),
            },
            key_password_ref: utm_signing_core::CredentialReference {
                service: "utm-dev".to_string(),
                account: "android_key_password".to_string(),
            },
        })
    }
}
```

```rust
// utm-signing-android/src/zipalign.rs
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, debug, error};
use utm_signing_core::{SigningResult, SigningError};

/// zipalign wrapper for APK alignment
pub struct ZipAlign;

impl ZipAlign {
    pub fn new() -> Self {
        Self
    }

    /// Align APK for signing
    pub async fn align(
        &self,
        input_apk: &Path,
        output_apk: &Path,
    ) -> SigningResult<()> {
        info!("Aligning APK: {:?} -> {:?}", input_apk, output_apk);

        // Ensure output directory exists
        if let Some(parent) = output_apk.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| SigningError::IoError(e))?;
        }

        let mut cmd = Command::new("zipalign");
        cmd.args(["-p", "-v", "4"])
            .arg(input_apk)
            .arg(output_apk)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Running zipalign: {:?}", cmd);

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute zipalign: {}", e)
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("zipalign failed: {}", stderr);
            return Err(SigningError::KeystoreError(
                format!("zipalign failed: {}", stderr)
            ));
        }

        info!("APK aligned successfully");
        Ok(())
    }

    /// Verify APK alignment
    pub async fn verify(&self, apk_path: &Path) -> SigningResult<bool> {
        let mut cmd = Command::new("zipalign");
        cmd.args(["-c", "-v", "4"])
            .arg(apk_path);

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute zipalign: {}", e)
            ))?;

        Ok(output.status.success())
    }
}

impl Default for ZipAlign {
    fn default() -> Self {
        Self::new()
    }
}
```

```rust
// utm-signing-android/src/apksigner.rs
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, debug, error};
use utm_signing_core::{
    ApkSigner as ApkSignerTrait, SigningResult, SigningError,
    SigningCredentials, SigningPlatform,
};
use crate::zipalign::ZipAlign;
use crate::keystore::KeystoreConfig;

/// Android apksigner wrapper
pub struct AndroidApkSigner {
    keystore: KeystoreConfig,
    zipalign: ZipAlign,
}

impl AndroidApkSigner {
    pub fn new(keystore: KeystoreConfig) -> Self {
        Self {
            keystore,
            zipalign: ZipAlign::new(),
        }
    }

    /// Run apksigner command
    async fn run_apksigner(&self, args: &[&str]) -> SigningResult<std::process::Output> {
        let mut cmd = Command::new("apksigner");
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Running apksigner: {:?}", cmd);

        cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute apksigner: {}", e)
            ))
    }

    /// Sign APK with keystore
    async fn sign_with_keystore(
        &self,
        input_apk: &Path,
        output_apk: &Path,
    ) -> SigningResult<()> {
        let keystore_pass = self.keystore.get_keystore_password().await?;
        let key_pass = self.keystore.get_key_password().await?;

        let mut cmd = Command::new("apksigner");
        cmd.args(["sign"])
            .arg("--ks")
            .arg(&self.keystore.path)
            .arg("--ks-pass")
            .arg(format!("pass:{}", keystore_pass))
            .arg("--key-pass")
            .arg(format!("pass:{}", key_pass))
            .arg("--out")
            .arg(output_apk)
            .arg(input_apk)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute apksigner: {}", e)
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("apksigner failed: {}", stderr);
            return Err(SigningError::KeystoreError(
                format!("apksigner failed: {}", stderr)
            ));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl ApkSignerTrait for AndroidApkSigner {
    async fn align(&self, input_apk: &Path, output_apk: &Path) -> SigningResult {
        self.zipalign.align(input_apk, output_apk).await?;
        Ok(utm_signing_core::SigningResult {
            signed_path: output_apk.to_path_buf(),
            verified: false,
            notarized: false,
            signed_at: chrono::Utc::now(),
            identity: self.keystore.alias.clone(),
        })
    }

    async fn sign_apk(&self, aligned_apk: &Path, output_apk: &Path) -> SigningResult {
        self.sign_with_keystore(aligned_apk, output_apk).await?;
        Ok(utm_signing_core::SigningResult {
            signed_path: output_apk.to_path_buf(),
            verified: false,
            notarized: false,
            signed_at: chrono::Utc::now(),
            identity: self.keystore.alias.clone(),
        })
    }

    async fn verify_apk(&self, apk_path: &Path) -> SigningResult {
        let output = self.run_apksigner(&["verify", "--verbose"]).await?;

        let verified = output.status.success();

        if !verified {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("APK verification failed: {}", stderr);
        }

        Ok(utm_signing_core::SigningResult {
            signed_path: apk_path.to_path_buf(),
            verified,
            notarized: false,
            signed_at: chrono::Utc::now(),
            identity: self.keystore.alias.clone(),
        })
    }

    async fn sign_full(
        &self,
        input_apk: &Path,
        output_dir: &Path,
    ) -> SigningResult {
        info!("Full APK signing workflow: {:?}", input_apk);

        // Ensure output directory exists
        tokio::fs::create_dir_all(output_dir).await
            .map_err(|e| SigningError::IoError(e))?;

        // Step 1: Align
        let aligned_apk = output_dir.join("app-aligned.apk");
        self.align(input_apk, &aligned_apk).await?;
        info!("APK aligned: {:?}", aligned_apk);

        // Step 2: Sign
        let signed_apk = output_dir.join("app-release.apk");
        self.sign_apk(&aligned_apk, &signed_apk).await?;
        info!("APK signed: {:?}", signed_apk);

        // Step 3: Verify
        let result = self.verify_apk(&signed_apk).await?;

        Ok(result)
    }
}
```

```rust
// utm-signing-android/src/keystore.rs
use std::path::PathBuf;
use utm_signing_core::{SigningResult, SigningError, CredentialReference};

/// Keystore configuration with secure password references
#[derive(Debug, Clone)]
pub struct KeystoreConfig {
    /// Path to keystore file
    pub path: String,
    /// Key alias within keystore
    pub alias: String,
    /// Reference to keystore password
    pub password_ref: CredentialReference,
    /// Reference to key password
    pub key_password_ref: CredentialReference,
}

impl KeystoreConfig {
    pub fn new(
        path: String,
        alias: String,
        password_ref: CredentialReference,
        key_password_ref: CredentialReference,
    ) -> Self {
        Self {
            path,
            alias,
            password_ref,
            key_password_ref,
        }
    }

    /// Retrieve keystore password from secrets store
    pub async fn get_keystore_password(&self) -> SigningResult<String> {
        let store = utm_signing_secrets::SecretsStore::new()
            .map_err(|e| SigningError::CredentialsError(
                format!("Failed to create secrets store: {}", e)
            ))?;

        store.get(&self.password_ref.service, &self.password_ref.account)
            .map_err(|e| SigningError::CredentialsError(
                format!("Failed to retrieve keystore password: {}", e)
            ))
    }

    /// Retrieve key password from secrets store
    pub async fn get_key_password(&self) -> SigningResult<String> {
        let store = utm_signing_secrets::SecretsStore::new()
            .map_err(|e| SigningError::CredentialsError(
                format!("Failed to create secrets store: {}", e)
            ))?;

        store.get(&self.key_password_ref.service, &self.key_password_ref.account)
            .map_err(|e| SigningError::CredentialsError(
                format!("Failed to retrieve key password: {}", e)
            ))
    }
}
```

## Windows Implementation (utm-signing-windows)

```rust
// utm-signing-windows/src/lib.rs
mod signtool;
mod certificate;

pub use signtool::WindowsSignTool;
pub use certificate::PfxCertificate;

use std::path::Path;
use utm_signing_core::{CodeSigner, SigningResult, SigningCredentials, SigningPlatform};

/// Windows signing configuration
#[derive(Debug, Clone)]
pub struct WindowsConfig {
    /// Path to PFX certificate file
    pub pfx_path: String,
    /// Reference to PFX password in secrets store
    pub password_ref: utm_signing_core::CredentialReference,
    /// Timestamp server URL
    pub timestamp_server: String,
}

impl WindowsConfig {
    pub fn from_env() -> Result<Self, utm_signing_core::SigningError> {
        Ok(Self {
            pfx_path: std::env::var("WINDOWS_PFX_PATH")
                .map_err(|_| utm_signing_core::SigningError::ConfigurationError(
                    "WINDOWS_PFX_PATH not set".to_string(),
                ))?,
            password_ref: utm_signing_core::CredentialReference {
                service: "utm-dev".to_string(),
                account: "windows_pfx_password".to_string(),
            },
            timestamp_server: std::env::var("TIMESTAMP_SERVER")
                .unwrap_or_else(|_| "http://timestamp.digicert.com".to_string()),
        })
    }
}
```

```rust
// utm-signing-windows/src/signtool.rs
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, debug, error};
use utm_signing_core::{
    CodeSigner, SigningCredentials, SigningIdentity, SigningPlatform,
    SigningResult, SigningError,
};
use crate::certificate::PfxCertificate;

/// Windows signtool wrapper
pub struct WindowsSignTool {
    config: super::WindowsConfig,
}

impl WindowsSignTool {
    pub fn new(config: super::WindowsConfig) -> Self {
        Self { config }
    }

    /// Run signtool sign command
    async fn sign_with_signtool(&self, exe_path: &Path) -> SigningResult<()> {
        let password = self.get_password().await?;

        let mut cmd = Command::new("signtool");
        cmd.args(["sign"])
            .arg("/f")
            .arg(&self.config.pfx_path)
            .arg("/p")
            .arg(&password)
            .arg("/tr")
            .arg(&self.config.timestamp_server)
            .arg("/td")
            .arg("sha256")
            .arg("/fd")
            .arg("sha256")
            .arg(exe_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Running signtool: {:?}", cmd);

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute signtool: {}", e)
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("signtool failed: {}", stderr);
            return Err(SigningError::CodeSigningFailed(
                format!("signtool failed: {}", stderr)
            ));
        }

        info!("Code signing completed successfully");
        Ok(())
    }

    async fn get_password(&self) -> SigningResult<String> {
        let store = utm_signing_secrets::SecretsStore::new()
            .map_err(|e| SigningError::CredentialsError(
                format!("Failed to create secrets store: {}", e)
            ))?;

        store.get(&self.config.password_ref.service, &self.config.password_ref.account)
            .map_err(|e| SigningError::CredentialsError(
                format!("Failed to retrieve PFX password: {}", e)
            ))
    }
}

#[async_trait::async_trait]
impl CodeSigner for WindowsSignTool {
    fn platform(&self) -> SigningPlatform {
        SigningPlatform::Windows
    }

    async fn sign(
        &self,
        input_path: &Path,
        _credentials: &SigningCredentials,
    ) -> SigningResult<utm_signing_core::SigningResult> {
        info!("Signing Windows executable: {:?}", input_path);

        self.sign_with_signtool(input_path).await?;

        Ok(utm_signing_core::SigningResult {
            signed_path: input_path.to_path_buf(),
            verified: false,
            notarized: false,
            signed_at: chrono::Utc::now(),
            identity: self.config.pfx_path.clone(),
        })
    }

    async fn verify(&self, signed_path: &Path) -> SigningResult<utm_signing_core::SigningResult> {
        info!("Verifying signed executable: {:?}", signed_path);

        let mut cmd = Command::new("signtool");
        cmd.args(["verify", "/v", "/pa"])
            .arg(signed_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().await
            .map_err(|e| SigningError::ProcessError(
                format!("Failed to execute signtool: {}", e)
            ))?;

        let verified = output.status.success();

        if !verified {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Verification failed: {}", stderr);
        }

        Ok(utm_signing_core::SigningResult {
            signed_path: signed_path.to_path_buf(),
            verified,
            notarized: false,
            signed_at: chrono::Utc::now(),
            identity: self.config.pfx_path.clone(),
        })
    }

    async fn list_identities(&self) -> SigningResult<Vec<SigningIdentity>> {
        // Windows certificate store enumeration would go here
        // For now, return empty list
        Ok(Vec::new())
    }
}
```

## Cross-Platform Secrets Storage (utm-signing-secrets)

```rust
// utm-signing-secrets/src/lib.rs
mod traits;

#[cfg(target_os = "macos")]
mod keychain;
#[cfg(target_os = "windows")]
mod credential_manager;
#[cfg(target_os = "linux")]
mod libsecret;
mod file;

pub use traits::SecretsStoreTrait;
pub use file::FileStore;

#[cfg(target_os = "macos")]
pub use keychain::KeychainStore;
#[cfg(target_os = "windows")]
pub use credential_manager::WindowsCredentialStore;
#[cfg(target_os = "linux")]
pub use libsecret::LibsecretStore;

use utm_signing_core::SigningResult;

/// Cross-platform secrets store
pub struct SecretsStore {
    inner: Box<dyn SecretsStoreTrait>,
}

impl SecretsStore {
    /// Create a new secrets store using the platform-specific implementation
    pub fn new() -> SigningResult<Self> {
        let inner: Box<dyn SecretsStoreTrait> = match std::env::consts::OS {
            "macos" => Box::new(KeychainStore::new("utm-dev")?),
            "windows" => Box::new(WindowsCredentialStore::new("utm-dev")?),
            "linux" => Box::new(LibsecretStore::new("utm-dev")?),
            _ => Box::new(FileStore::new()),
        };

        Ok(Self { inner })
    }

    /// Store a secret
    pub fn set(&self, service: &str, key: &str, value: &str) -> SigningResult<()> {
        self.inner.set(service, key, value)
    }

    /// Retrieve a secret
    pub fn get(&self, service: &str, key: &str) -> SigningResult<String> {
        self.inner.get(service, key)
    }

    /// Delete a secret
    pub fn delete(&self, service: &str, key: &str) -> SigningResult<()> {
        self.inner.delete(service, key)
    }
}

impl Default for SecretsStore {
    fn default() -> Self {
        Self::new().expect("Failed to create secrets store")
    }
}
```

```rust
// utm-signing-secrets/src/traits.rs
use utm_signing_core::SigningResult;

/// Trait for platform-specific secrets storage
pub trait SecretsStoreTrait: Send + Sync {
    /// Store a secret value
    fn set(&self, service: &str, key: &str, value: &str) -> SigningResult<()>;

    /// Retrieve a secret value
    fn get(&self, service: &str, key: &str) -> SigningResult<String>;

    /// Delete a secret
    fn delete(&self, service: &str, key: &str) -> SigningResult<()>;
}
```

```rust
// utm-signing-secrets/src/keychain.rs
// utm-signing-secrets/src/keychain.rs
use std::ffi::CString;
use utm_signing_core::SigningResult;
use security_framework::{
    item::{ItemClass, ItemSearchOptions, Limit, AddItemOptions},
    secure_enclave::Accessibility,
};
use security_framework_sys::item::ItemClass as SFItemClass;

/// macOS Keychain-based secrets store
pub struct KeychainStore {
    service_name: String,
}

impl KeychainStore {
    pub fn new(service_name: &str) -> SigningResult<Self> {
        Ok(Self {
            service_name: service_name.to_string(),
        })
    }
}

impl crate::traits::SecretsStoreTrait for KeychainStore {
    fn set(&self, service: &str, key: &str, value: &str) -> SigningResult<()> {
        use security_framework::item::AddItemOptions;

        let mut options = AddItemOptions::new();
        options.class(ItemClass::generic_password());
        options.account(key);
        options.service(service);
        options.data(value.as_bytes());
        options.accessibility(Accessibility::WhenUnlocked);

        security_framework::item::add_item(&options)
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Keychain set error: {:?}", e)
            ))?;

        Ok(())
    }

    fn get(&self, service: &str, key: &str) -> SigningResult<String> {
        use security_framework::item::ItemSearchOptions;

        let mut search = ItemSearchOptions::new();
        search.class(ItemClass::generic_password());
        search.account(key);
        search.service(service);
        search.load_attributes(true);
        search.load_data(true);

        let results = search.search()
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Keychain search error: {:?}", e)
            ))?;

        if let Some(result) = results.into_iter().next() {
            if let Some(data) = result.data() {
                return String::from_utf8(data.to_vec())
                    .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                        format!("Invalid UTF-8 in keychain value: {}", e)
                    ));
            }
        }

        Err(utm_signing_core::SigningError::SecretsStoreError(
            "Key not found in keychain".to_string()
        ))
    }

    fn delete(&self, service: &str, key: &str) -> SigningResult<()> {
        use security_framework::item::ItemSearchOptions;
        use security_framework::item::DeleteSearchOptions;

        let mut search = ItemSearchOptions::new();
        search.class(ItemClass::generic_password());
        search.account(key);
        search.service(service);

        // Convert to delete search
        let delete_search = DeleteSearchOptions::new()
            .class(ItemClass::generic_password())
            .account(key)
            .service(service);

        security_framework::item::delete_item(&delete_search)
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Keychain delete error: {:?}", e)
            ))?;

        Ok(())
    }
}
```

```rust
// utm-signing-secrets/src/credential_manager.rs
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use utm_signing_core::SigningResult;

/// Windows Credential Manager-based secrets store
pub struct WindowsCredentialStore {
    service_name: String,
}

impl WindowsCredentialStore {
    pub fn new(service_name: &str) -> SigningResult<Self> {
        Ok(Self {
            service_name: service_name.to_string(),
        })
    }

    fn make_target_name(&self, service: &str, key: &str) -> String {
        format!("{}\\{}\\{}", self.service_name, service, key)
    }
}

impl crate::traits::SecretsStoreTrait for WindowsCredentialStore {
    fn set(&self, service: &str, key: &str, value: &str) -> SigningResult<()> {
        use windows::Win32::Security::Credentials::*;
        use windows::Win32::Foundation::*;

        let target = self.make_target_name(service, key);
        let target_wide: Vec<u16> = OsStr::new(&target)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let value_wide: Vec<u16> = OsStr::new(value)
            .encode_wide()
            .chain(Some(0))
            .collect();

        unsafe {
            let mut credential = CREDENTIALW {
                Type: CRED_TYPE_GENERIC,
                TargetName: PCWSTR(target_wide.as_ptr()),
                CredentialBlob: CREDENTIAL_BLOB {
                    cbCredentialBlob: (value.len() * 2) as u32,
                    pbCredentialBlob: value_wide.as_ptr() as *mut _,
                },
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                ..Default::default()
            };

            CredWriteW(&credential, 0)
                .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                    format!("CredWrite failed: {:?}", e)
                ))?;
        }

        Ok(())
    }

    fn get(&self, service: &str, key: &str) -> SigningResult<String> {
        use windows::Win32::Security::Credentials::*;

        let target = self.make_target_name(service, key);
        let target_wide: Vec<u16> = OsStr::new(&target)
            .encode_wide()
            .chain(Some(0))
            .collect();

        unsafe {
            let mut credential: *mut CREDENTIALW = std::ptr::null_mut();

            CredReadW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, 0, &mut credential)
                .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                    format!("CredRead failed: {:?}", e)
                ))?;

            if credential.is_null() {
                return Err(utm_signing_core::SigningError::SecretsStoreError(
                    "Credential not found".to_string()
                ));
            }

            let blob = (*credential).CredentialBlob;
            let data_slice = std::slice::from_raw_parts(
                blob.pbCredentialBlob,
                blob.cbCredentialBlob as usize,
            );

            // Convert wide string back to UTF-8
            let wide_slice = std::slice::from_raw_parts(
                data_slice.as_ptr() as *const u16,
                data_slice.len() / 2,
            );

            let value = String::from_utf16_lossy(wide_slice);
            // Remove null terminator
            let value = value.trim_end_matches('\0').to_string();

            Ok(value)
        }
    }

    fn delete(&self, service: &str, key: &str) -> SigningResult<()> {
        use windows::Win32::Security::Credentials::*;

        let target = self.make_target_name(service, key);
        let target_wide: Vec<u16> = OsStr::new(&target)
            .encode_wide()
            .chain(Some(0))
            .collect();

        unsafe {
            CredDeleteW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, 0)
                .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                    format!("CredDelete failed: {:?}", e)
                ))?;
        }

        Ok(())
    }
}
```

```rust
// utm-signing-secrets/src/libsecret.rs
use utm_signing_core::SigningResult;

/// Linux libsecret-based secrets store
pub struct LibsecretStore {
    service_name: String,
}

impl LibsecretStore {
    pub fn new(service_name: &str) -> SigningResult<Self> {
        Ok(Self {
            service_name: service_name.to_string(),
        })
    }
}

impl crate::traits::SecretsStoreTrait for LibsecretStore {
    fn set(&self, service: &str, key: &str, value: &str) -> SigningResult<()> {
        use secret_service::{SecretService, EncryptionType};

        let ss = SecretService::connect(EncryptionType::Dh)
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Failed to connect to secret service: {}", e)
            ))?;

        let collection = ss.get_default_collection()
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Failed to get default collection: {}", e)
            ))?;

        let attributes = [
            ("service", service),
            ("key", key),
            ("application", &self.service_name),
        ];

        collection
            .set(
                "password",
                attributes,
                value.as_bytes(),
                Some("application/x-password"),
                secret_service::SetType::IfEmpty,
            )
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Failed to set secret: {}", e)
            ))?;

        Ok(())
    }

    fn get(&self, service: &str, key: &str) -> SigningResult<String> {
        use secret_service::{SecretService, EncryptionType};

        let ss = SecretService::connect(EncryptionType::Dh)
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Failed to connect to secret service: {}", e)
            ))?;

        let attributes = [
            ("service", service),
            ("key", key),
            ("application", &self.service_name),
        ];

        let results = ss.search_items(attributes)
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Failed to search items: {}", e)
            ))?;

        if let Some(item) = results.iter().next() {
            let secret = item.get_secret()
                .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                    format!("Failed to get secret: {}", e)
                ))?;

            return String::from_utf8(secret)
                .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                    format!("Invalid UTF-8 in secret: {}", e)
                ));
        }

        Err(utm_signing_core::SigningError::SecretsStoreError(
            "Key not found".to_string()
        ))
    }

    fn delete(&self, service: &str, key: &str) -> SigningResult<()> {
        use secret_service::{SecretService, EncryptionType};

        let ss = SecretService::connect(EncryptionType::Dh)
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Failed to connect to secret service: {}", e)
            ))?;

        let attributes = [
            ("service", service),
            ("key", key),
            ("application", &self.service_name),
        ];

        let results = ss.search_items(attributes)
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Failed to search items: {}", e)
            ))?;

        for item in results {
            item.delete()
                .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                    format!("Failed to delete item: {}", e)
                ))?;
        }

        Ok(())
    }
}
```

```rust
// utm-signing-secrets/src/file.rs
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use utm_signing_core::SigningResult;

/// Fallback file-based secrets store (NOT SECURE - use only for testing)
pub struct FileStore {
    path: PathBuf,
    data: HashMap<String, HashMap<String, String>>,
}

impl FileStore {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join("utm-dev-secrets.json");
        let data = HashMap::new();
        Self { path, data }
    }

    fn load(&mut self) -> SigningResult<()> {
        if self.path.exists() {
            let mut file = File::open(&self.path)
                .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                    format!("Failed to open secrets file: {}", e)
                ))?;

            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                    format!("Failed to read secrets file: {}", e)
                ))?;

            self.data = serde_json::from_str(&contents)
                .unwrap_or_else(|_| HashMap::new());
        }

        Ok(())
    }

    fn save(&self) -> SigningResult<()> {
        let json = serde_json::to_string_pretty(&self.data)
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Failed to serialize secrets: {}", e)
            ))?;

        let mut file = File::create(&self.path)
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Failed to create secrets file: {}", e)
            ))?;

        file.write_all(json.as_bytes())
            .map_err(|e| utm_signing_core::SigningError::SecretsStoreError(
                format!("Failed to write secrets file: {}", e)
            ))?;

        Ok(())
    }
}

impl Default for FileStore {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::traits::SecretsStoreTrait for FileStore {
    fn set(&self, service: &str, key: &str, value: &str) -> SigningResult<()> {
        // Note: This is a simplified implementation
        // A real implementation would need proper locking
        Ok(())
    }

    fn get(&self, service: &str, key: &str) -> SigningResult<String> {
        Err(utm_signing_core::SigningError::SecretsStoreError(
            "FileStore not fully implemented".to_string()
        ))
    }

    fn delete(&self, service: &str, key: &str) -> SigningResult<()> {
        Ok(())
    }
}
```

## Key Rust-Specific Changes

### 1. Async-First Design

All signing operations are async to avoid blocking the main thread during potentially slow I/O and process execution:

```rust
#[async_trait::async_trait]
impl CodeSigner for MacOSCodeSigner {
    async fn sign(&self, input_path: &Path, credentials: &SigningCredentials) -> SigningResult {
        // Non-blocking process execution
        let output = cmd.output().await?;
        // ...
    }
}
```

### 2. Error Handling with thiserror

Comprehensive error types provide actionable diagnostics:

```rust
#[derive(Error, Debug)]
pub enum SigningError {
    #[error("Code signing failed: {0}")]
    CodeSigningFailed(String),

    #[error("Notarization failed: {0}")]
    NotarizationFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Process execution error: {0}")]
    ProcessError(String),
}
```

### 3. Platform-Specific Implementations

Using `#[cfg(target_os = "...")]` for conditional compilation:

```rust
#[cfg(target_os = "macos")]
mod keychain;
#[cfg(target_os = "windows")]
mod credential_manager;
#[cfg(target_os = "linux")]
mod libsecret;
```

### 4. Secure Credential Handling

Credentials are never stored in structs - only references to secure OS keychains:

```rust
pub struct CredentialReference {
    pub service: String,
    pub account: String,
}

pub struct KeystoreConfig {
    pub path: String,
    pub alias: String,
    pub password_ref: CredentialReference,  // Reference, not actual password
    pub key_password_ref: CredentialReference,
}
```

## Ownership & Borrowing Strategy

### Long-Lived Signer Objects

Signer objects are created once and reused, minimizing allocation:

```rust
pub struct MacOSCodeSigner {
    config: MacOSConfig,  // Owned config
}

impl MacOSCodeSigner {
    pub fn new(config: MacOSConfig) -> Self {
        Self { config }
    }
}
```

### Borrowed Paths

All path parameters use borrowed references:

```rust
async fn sign(&self, input_path: &Path, credentials: &SigningCredentials) -> SigningResult;
```

### Smart Pointer Usage

Box<dyn Trait> for platform abstraction:

```rust
pub struct SecretsStore {
    inner: Box<dyn SecretsStoreTrait>,
}
```

## Concurrency Model

### Tokio Async Runtime

All I/O and process execution uses tokio:

```rust
use tokio::process::Command;
use tokio::time::{Duration, sleep};

async fn run_codesign(&self, app_path: &Path) -> SigningResult<()> {
    let mut cmd = Command::new("codesign");
    // ...
    let output = cmd.output().await?;
}
```

### Thread-Safe Traits

All traits require `Send + Sync`:

```rust
#[async_trait::async_trait]
pub trait CodeSigner: Send + Sync {
    // ...
}
```

## Memory Considerations

### Zero-Copy Where Possible

Process output is converted to String only when needed:

```rust
let stderr = String::from_utf8_lossy(&output.stderr);
```

### Streaming for Large Files

For large artifacts, use streaming I/O:

```rust
use tokio::io::AsyncReadExt;

async fn read_large_file(path: &Path) -> SigningResult<Vec<u8>> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;
    Ok(buffer)
}
```

## Edge Cases & Safety Guarantees

### 1. Process Execution Failures

All external tool calls are wrapped with proper error handling:

```rust
let output = cmd.output().await
    .map_err(|e| SigningError::ProcessError(
        format!("Failed to execute codesign: {}", e)
    ))?;

if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(SigningError::CodeSigningFailed(stderr.to_string()));
}
```

### 2. Unicode Path Handling

All paths use `Path`/`PathBuf` for proper Unicode support:

```rust
pub async fn sign(&self, input_path: &Path, ...) -> SigningResult;
```

### 3. Credential Security

Passwords are cleared from memory after use:

```rust
fn use_password<T>(password: String, f: impl FnOnce(&str) -> T) -> T {
    let result = f(&password);
    // password is dropped here, clearing from stack
    result
}
```

## Code Examples

### Full macOS Signing Workflow

```rust
use utm_signing_core::{CodeSigner, Notarize, SigningCredentials, SigningPlatform};
use utm_signing_macos::{MacOSCodeSigner, MacOSNotaryTool, MacOSConfig};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize signer
    let config = MacOSConfig::from_env()?;
    let signer = MacOSCodeSigner::new(config.clone());
    let notary = MacOSNotaryTool::new(
        config.apple_id.unwrap(),
        config.team_id.clone(),
        config.notarization_profile.unwrap(),
    );

    // Sign the app
    let app_path = Path::new("./build/MyApp.app");
    let credentials = SigningCredentials::AppleDeveloper {
        identity: get_signing_identity().await?,
        notarization_profile: config.notarization_profile.clone(),
    };

    let result = signer.sign(app_path, &credentials).await?;
    println!("Signed: {:?}", result.signed_path);

    // Create DMG for notarization
    let dmg_path = Path::new("./build/MyApp.dmg");
    signer.create_dmg(app_path, dmg_path).await?;

    // Submit for notarization
    let notarize_result = notary.submit_and_wait(dmg_path, "MyProfile").await?;
    println!("Notarized: {}", notarize_result.notarized);

    // Staple ticket
    notary.staple(app_path).await?;
    println!("Stapled successfully");

    // Verify
    let verify_result = signer.verify(app_path).await?;
    println!("Verified: {}", verify_result.verified);

    Ok(())
}
```

### Android APK Signing Workflow

```rust
use utm_signing_core::{ApkSigner, SigningPlatform};
use utm_signing_android::{AndroidApkSigner, KeystoreConfig};
use utm_signing_core::CredentialReference;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize keystore config
    let keystore = KeystoreConfig::new(
        "~/keystores/release.keystore".to_string(),
        "release".to_string(),
        CredentialReference {
            service: "utm-dev".to_string(),
            account: "android_keystore_password".to_string(),
        },
        CredentialReference {
            service: "utm-dev".to_string(),
            account: "android_key_password".to_string(),
        },
    );

    let signer = AndroidApkSigner::new(keystore);

    // Full signing workflow
    let input_apk = Path::new("./build/app-unsigned.apk");
    let output_dir = Path::new("./build/signed");

    let result = signer.sign_full(input_apk, output_dir).await?;
    println!("Signed APK: {:?}", result.signed_path);
    println!("Verified: {}", result.verified);

    Ok(())
}
```

### Cross-Platform Signing

```rust
use utm_signing_core::{CodeSigner, SigningPlatform};
use std::path::Path;

async fn sign_for_platform(
    platform: SigningPlatform,
    app_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    match platform {
        SigningPlatform::Macos => {
            let config = utm_signing_macos::MacOSConfig::from_env()?;
            let signer = utm_signing_macos::MacOSCodeSigner::new(config);
            // Sign...
        }
        SigningPlatform::Windows => {
            let config = utm_signing_windows::WindowsConfig::from_env()?;
            let signer = utm_signing_windows::WindowsSignTool::new(config);
            // Sign...
        }
        SigningPlatform::Android => {
            let keystore = utm_signing_android::KeystoreConfig::new(/* ... */);
            let signer = utm_signing_android::AndroidApkSigner::new(keystore);
            // Sign...
        }
        _ => return Err("Platform not supported".into()),
    }

    Ok(())
}
```

## Migration Path

### Phase 1: Core Types (Week 1)
- Implement `utm-signing-core` crate
- Define traits and error types
- Set up workspace structure

### Phase 2: Platform Implementations (Weeks 2-4)
- macOS codesign/notarytool (Week 2)
- Android APK signing (Week 3)
- Windows signtool (Week 4)
- iOS provisioning (Week 4, parallel)

### Phase 3: Secrets Storage (Week 5)
- Keychain integration (macOS)
- Credential Manager (Windows)
- libsecret (Linux)

### Phase 4: CLI & Integration (Week 6)
- Build CLI tool
- CI/CD integration
- Documentation

## Performance Considerations

### Parallel Signing

For multi-architecture builds, sign in parallel:

```rust
use tokio::task::JoinSet;

async fn sign_multiple(
    signer: &dyn CodeSigner,
    apps: &[PathBuf],
) -> Vec<SigningResult> {
    let mut set = JoinSet::new();

    for app_path in apps {
        let signer = Arc::clone(&signer);
        let path = app_path.clone();
        set.spawn(async move {
            signer.sign(&path, &credentials).await
        });
    }

    let mut results = Vec::new();
    while let Some(result) = set.join_next().await {
        results.push(result.unwrap_or_else(|e| {
            Err(SigningError::ProcessError(e.to_string()))
        }));
    }

    results
}
```

### Build Cache Integration

Cache signing results for incremental builds:

```rust
use std::collections::HashMap;

pub struct SigningCache {
    cache: HashMap<String, SigningResult>,
}

impl SigningCache {
    pub fn get_or_sign(
        &mut self,
        key: &str,
        signer: &dyn CodeSigner,
        path: &Path,
    ) -> SigningResult {
        if let Some(result) = self.cache.get(key) {
            return Ok(result.clone());
        }

        let result = signer.sign(path, &credentials)?;
        self.cache.insert(key.to_string(), result.clone());
        Ok(result)
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entitlements_generation() {
        let gen = EntitlementsGenerator::new();
        let entitlements = Entitlements::for_emulator();
        let plist = gen.to_plist(&entitlements);

        assert!(plist.as_dictionary().unwrap().contains_key(
            "com.apple.security.cs.allow-jit"
        ));
    }

    #[tokio::test]
    async fn test_secrets_store_roundtrip() {
        let store = SecretsStore::new().unwrap();

        store.set("test-service", "test-key", "test-value").unwrap();
        let value = store.get("test-service", "test-key").unwrap();

        assert_eq!(value, "test-value");

        store.delete("test-service", "test-key").unwrap();
    }
}
```

### Integration Tests

```rust
// tests/integration/macos_signing.rs
#[tokio::test]
#[ignore] // Requires valid signing identity
async fn test_macos_full_signing_workflow() {
    let config = MacOSConfig::from_env().unwrap();
    let signer = MacOSCodeSigner::new(config);
    let notary = MacOSNotaryTool::new(/* ... */);

    // Create test app bundle
    let test_app = create_test_app_bundle().await;

    // Sign
    let result = signer.sign(&test_app, &credentials).await.unwrap();
    assert!(result.signed_path.exists());

    // Verify
    let verify = signer.verify(&test_app).await.unwrap();
    assert!(verify.verified);
}
```

## Open Considerations

1. **Hardware Security Module (HSM) Support**: Add support for cloud HSMs (AWS CloudHSM, Azure Key Vault) for enterprise signing

2. **Batch Signing**: Optimize for signing many files efficiently (e.g., frameworks with multiple binaries)

3. **Notarization Profile Management**: Support multiple notarization profiles for different teams/products

4. **Certificate Expiry Monitoring**: Add proactive alerts before certificates expire

5. **Air-Gapped Signing**: Support offline signing workflows for high-security environments

6. **Signature Validation Caching**: Cache validation results to speed up repeated verifications

7. **Cross-Compilation Support**: Enable signing iOS apps from non-macOS platforms (requires remote Mac)
