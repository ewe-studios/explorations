# rauthy-pam-nss Documentation

PAM and NSS modules for rauthy system authentication.

## Document Index

| # | Document | Description |
|---|----------|-------------|
| 00 | [Overview](00-overview.html) | Philosophy, features |
| 01 | [NSS](01-nss.html) | NSS module |
| 02 | [PAM](02-pam.html) | PAM module |
| 03 | [SSH](03-ssh.html) | SSH integration |
| 04 | [Install](04-install.html) | Installation |
| 05 | [Config](05-config.html) | Configuration |

## Quick Links

- **Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.auth/src.rauthy/rauthy-pam-nss/`
- **Repository:** https://github.com/sebadob/rauthy-pam-nss.git

## What is rauthy-pam-nss?

System-level authentication modules for rauthy:

```
┌─────────────────────────────────────────┐
│           Linux System                  │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐ │
│  │   SSH   │  │   su    │  │  sudo   │ │
│  └────┬────┘  └────┬────┘  └────┬────┘ │
│       │            │            │      │
│  ┌────┴────────────┴────────────┴────┐ │
│  │        PAM (pam_rauthy.so)         │ │
│  └────────────────────────────────────┘ │
│       │                                   │
│  ┌────┴────────────────────────────────┐ │
│  │  NSS (libnss_rauthy.so)             │ │
│  │  - getent passwd                    │ │
│  │  - getent group                     │ │
│  │  - getent hosts                     │ │
│  └──────────────────────────────────────┘ │
│       │                                   │
│  ┌────┴────────────────────────────────┐ │
│  │           Rauthy IdP                  │ │
│  └──────────────────────────────────────┘ │
└───────────────────────────────────────────┘
```

## Features

| Feature | Description |
|---------|-------------|
| **NSS** | User/group/host resolution |
| **PAM** | Password/passkey auth |
| **SSH** | Remote login |
| **SELinux** | Security policies |

## Next Steps

Start with [Overview →](00-overview.html).
