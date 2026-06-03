---
title: Installation
prev: 03-ssh.md
next: 05-config.md
---

# Installation

Install rauthy-pam-nss modules.

## Quick Install

### Download

```bash
curl -LO https://github.com/sebadob/rauthy-pam-nss/releases/download/v0.2.1/rauthy-pam-nss-install.tar.gz.sha256 && \
curl -LO https://github.com/sebadob/rauthy-pam-nss/releases/download/v0.2.1/rauthy-pam-nss-install.tar.gz && \
sha256sum -c rauthy-pam-nss-install.tar.gz.sha256 && \
tar -xzf rauthy-pam-nss-install.tar.gz && \
cd rauthy-pam-nss-install
```

### Inspect

**CAUTION:** Always inspect scripts before running:

```bash
cat install.sh | less
```

### Install NSS

```bash
sudo ./install.sh nss
```

### Test NSS

```bash
# Test user resolution
getent passwd
getent passwd <rauthy-user>

# Test group resolution
getent group

# Test hosts
getent hosts
```

### Install PAM

```bash
sudo ./install.sh pam
```

### Test PAM

**Keep a root session open before testing!**

```bash
# Test authentication
su - <rauthy-user>

# Or SSH from another terminal
ssh <rauthy-user>@<host>
```

## Update

```bash
sudo ./install.sh update
```

## Build from Source

### Dependencies

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install dev dependencies
# Fedora/RHEL
sudo dnf install openssl-devel

# Debian/Ubuntu
sudo apt-get install libssl-dev

# Arch
sudo pacman -S openssl
```

### Build

```bash
git clone https://github.com/sebadob/rauthy-pam-nss.git
cd rauthy-pam-nss
cargo build --release
```

### Manual Install

```bash
# Copy NSS module
sudo cp target/release/libnss_rauthy.so /lib/libnss_rauthy.so.2

# Copy PAM module
sudo cp target/release/pam_rauthy.so /lib/security/pam_rauthy.so

# Copy authorized keys helper
sudo cp target/release/rauthy-authorized-keys /usr/bin/rauthy-authorized-keys

# Copy config
sudo cp rauthy-pam-nss.toml /etc/rauthy/
```

## SELinux

### Install Policies

```bash
cd selinux/
sudo ./install-selinux.sh
```

### Manual Policy

```bash
# Load policy
sudo semodule -i rauthy-pam-nss.pp
```

## Verification

### Check NSS

```bash
# Should list rauthy users
getent passwd | grep -i rauthy

# Should resolve specific user
getent passwd <rauthy-user>
```

### Check PAM

```bash
# Test authentication
pamtester rauthy <user> authenticate
```

### SSH Test

```bash
# From another machine
ssh <rauthy-user>@<server>
```

**Keep backup root session open!**

## Troubleshooting

### NSS Not Working

```bash
# Check nsswitch.conf
cat /etc/nsswitch.conf | grep rauthy

# Should show:
# passwd: files rauthy
# group: files rauthy
```

### PAM Not Working

```bash
# Check PAM config
cat /etc/pam.d/sshd | grep rauthy

# Check logs
sudo journalctl -u sshd | grep pam_rauthy
```

### SELinux Issues

```bash
# Check denials
sudo ausearch -m AVC -ts recent

# Set permissive temporarily
sudo setenforce 0
```

## Next Steps

Continue to [Configuration →](05-config.html).
