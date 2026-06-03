---
title: PAM
prev: 01-nss.md
next: 03-ssh.md
---

# PAM Module

Pluggable Authentication for rauthy.

## Overview

`pam_rauthy.so` authenticates users via rauthy.

## Installation

```bash
# Install PAM module
sudo cp pam_rauthy.so /lib/security/pam_rauthy.so
```

### PAM Configuration

```
# /etc/pam.d/sshd
auth    sufficient    pam_rauthy.so
auth    required      pam_unix.so

# /etc/pam.d/system-auth
auth    sufficient    pam_rauthy.so
auth    required      pam_unix.so

account sufficient    pam_rauthy.so
account required      pam_unix.so

session optional      pam_rauthy.so
session required      pam_unix.so
```

**CAUTION:** Keep a root session open when testing!

## Authentication Methods

### 1. Password Authentication

Local password verification:

```rust
// src/pam-module/auth.rs
impl PamHooks for RauthyPam {
    fn authenticate(pamh: &mut PamHandle, args: &[&str]) -> PamResult {
        let user = pamh.get_user(None)?;
        let password = pamh.get_authtok(None)?;
        
        // Verify with rauthy
        match rauthy_client::authenticate(&user, &password) {
            Ok(true) => PamResult::SUCCESS,
            Ok(false) => PamResult::AUTH_ERR,
            Err(e) => {
                error!("Auth error: {}", e);
                PamResult::AUTH_ERR
            }
        }
    }
}
```

### 2. Passkey Authentication

Local Yubikey/passkey:

```rust
fn authenticate_passkey(user: &str) -> PamResult {
    // Initiate WebAuthn
    let challenge = rauthy_client::webauthn_challenge(user)?;
    
    // User touches key
    let assertion = wait_for_passkey(&challenge)?;
    
    // Verify
    if rauthy_client::verify_passkey(user, &assertion)? {
        PamResult::SUCCESS
    } else {
        PamResult::AUTH_ERR
    }
}
```

### 3. PAM Remote Password

For SSH without passkey capability:

```rust
fn authenticate_remote(user: &str) -> PamResult {
    // Get PAM Remote Password from env or prompt
    let remote_password = std::env::var("RAUTHY_PAM_PASSWORD")
        .or_else(|| prompt_password("PAM Remote Password: "))?;
    
    // Validate
    match rauthy_client::validate_pam_password(user, &remote_password) {
        Ok(true) => PamResult::SUCCESS,
        _ => PamResult::AUTH_ERR,
    }
}
```

## PAM Remote Password

From rauthy account dashboard:

1. User logs into rauthy
2. Navigates to Account → PAM Remote Password
3. Generates temporary password
4. Uses for SSH login

**Aha:** Enables MFA accounts over SSH (no USB passkey needed).

## Session Management

```rust
fn open_session(pamh: &mut PamHandle) -> PamResult {
    let user = pamh.get_user(None)?;
    
    // Create home directory if needed
    create_home_dir(&user)?;
    
    // Copy /etc/skel_rauthy
    copy_skel(&user)?;
    
    // Execute custom scripts
    exec_session_scripts(&user, "open")?;
    
    PamResult::SUCCESS
}

fn close_session(pamh: &mut PamHandle) -> PamResult {
    let user = pamh.get_user(None)?;
    
    // Execute close scripts
    exec_session_scripts(&user, "close")?;
    
    PamResult::SUCCESS
}
```

## Configuration

```toml
# rauthy-pam-nss.toml
[pam]
rauthy_url = "https://auth.example.com"
api_key = "pam_api_key_xxx"

[home]
skel = "/etc/skel_rauthy"
create_missing = true

[session]
open_script = "/etc/rauthy/session-open.sh"
close_script = "/etc/rauthy/session-close.sh"
```

## Troubleshooting

### Debug Mode

```bash
# Enable debug logging
export RAUTHY_PAM_DEBUG=1
sshd -d  # Debug mode
```

### Common Issues

| Issue | Solution |
|-------|----------|
| Auth fails | Check rauthy URL |
| NSS fails | Verify nsswitch.conf |
| Lockout | Use rescue mode |

## Next Steps

Continue to [SSH →](03-ssh.html).
