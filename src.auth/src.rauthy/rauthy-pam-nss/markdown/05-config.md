---
title: Configuration
prev: 04-install.md
---

# Configuration

rauthy-pam-nss configuration.

## Config File

```toml
# /etc/rauthy/rauthy-pam-nss.toml

[rauthy]
url = "https://auth.example.com"
api_key = "pam_nss_api_key_xxx"

[nss]
cache_ttl = 60  # seconds

[pam]
timeout = 30    # seconds

[home]
skel = "/etc/skel_rauthy"
create = true

[session]
open_script = "/etc/rauthy/session-open.sh"
close_script = "/etc/rauthy/session-close.sh"

[[group]]
name = "wheel-rauthy"
local_gid = 10
rauthy_gid = 10001
```

## Environment Variables

```bash
# Rauthy connection
export RAUTHY_URL="https://auth.example.com"
export RAUTHY_API_KEY="pam_nss_api_key_xxx"

# Debug
export RAUTHY_DEBUG=1

# PAM Remote Password (for SSH)
export RAUTHY_PAM_PASSWORD="temporary-password"
```

## NSS Configuration

### nsswitch.conf

```
# /etc/nsswitch.conf
passwd:         files rauthy
group:          files rauthy
shadow:         files
hosts:          files rauthy dns
```

## PAM Configuration

### /etc/pam.d/sshd

```
# Authentication
auth    sufficient    pam_rauthy.so
auth    required      pam_unix.so

# Account
account sufficient    pam_rauthy.so
account required      pam_unix.so

# Session
session optional      pam_rauthy.so
session required      pam_unix.so
```

### authselect (Fedora/RHEL)

```bash
# Select custom profile
sudo authselect select custom/rauthy

# Create custom profile
sudo authselect create-profile rauthy --base-on=sssd
```

## SSH Configuration

### /etc/ssh/sshd_config

```
# PAM
UsePAM yes

# Authorized keys
AuthorizedKeysCommand /usr/bin/rauthy-authorized-keys %u
AuthorizedKeysCommandUser root

# Allow rauthy users
AllowUsers *@rauthy
```

### Restart SSH

```bash
sudo systemctl restart sshd
```

## SELinux

### Install Policy

```bash
cd /path/to/selinux/
sudo ./install-selinux.sh
```

### Check Status

```bash
semodule -l | grep rauthy
getsebool -a | grep rauthy
```

## Home Directory

### /etc/skel_rauthy

```bash
# Create skel directory
sudo mkdir -p /etc/skel_rauthy

# Add default files
sudo cp /etc/skel/.bashrc /etc/skel_rauthy/
sudo cp /etc/skel/.profile /etc/skel_rauthy/
```

## Session Scripts

### Session Open

```bash
#!/bin/bash
# /etc/rauthy/session-open.sh

USER=$1

# Log session start
logger -t rauthy "Session opened for $USER"

# Custom setup
# ...
```

### Session Close

```bash
#!/bin/bash
# /etc/rauthy/session-close.sh

USER=$1

# Log session end
logger -t rauthy "Session closed for $USER"

# Cleanup
# ...
```

## Group Merging

```toml
[[group]]
name = "rauthy-wheel"
local_gid = 10          # Maps to local wheel
rauthy_gid = 10001
```

Users in `rauthy-wheel` appear in local `wheel` group.

## Debug Mode

```bash
# Enable debug
export RAUTHY_DEBUG=1

# Test NSS
getent passwd <user>

# Test PAM
pamtester rauthy <user> authenticate

# Check logs
sudo journalctl -u sshd -f
```

## Summary

| File | Purpose |
|------|---------|
| `/etc/rauthy/rauthy-pam-nss.toml` | Main config |
| `/etc/nsswitch.conf` | NSS config |
| `/etc/pam.d/*` | PAM config |
| `/etc/ssh/sshd_config` | SSH config |
| `/etc/skel_rauthy` | Home template |
