# Moltinators - NixOS on AWS Deployment Infrastructure

## Overview

Moltinators is a reference implementation for deploying NixOS to AWS using OpenTofu (Terraform fork) and Nix flakes. It serves two purposes:

1. **Generic Layer**: Reusable patterns for NixOS-on-AWS deployment
2. **Specific Layer**: AI coding agent infrastructure (CLAWDINATOR instances)

**Repository:** `github:joshp123/clawdinators`

## What is CLAWDINATOR?

CLAWDINATOR is a cybernetic crustacean organism - an AI coding agent that:
- Monitors GitHub repositories (issues, PRs, commits)
- Responds to maintainer requests on Discord
- Shares context across instances (hive mind memory)
- Self-updates without human intervention
- Has a distinct personality (Terminator/Predator quotes + lobster philosophy)

```
┌─────────────────────────────────────────────────────────────────┐
│                    CLAWDINATOR SPEC                             │
├─────────────────────────────────────────────────────────────────┤
│  Name: CLAWDINATOR-{1..n}                                       │
│  Connects to: Discord (#clawdributors-test channel)             │
│  Monitors: GitHub issues/PRs                                    │
│  Personality: br00tal, Terminator quotes, lobster undertones    │
│  Stack: NixOS + AWS EC2 + Discord Gateway + GitHub App          │
│  Memory: Shared EFS mount (hive mind)                           │
└─────────────────────────────────────────────────────────────────┘
```

## Architecture

### Two-Layer Design

```
┌─────────────────────────────────────────────────────────────────┐
│              CLAWDINATOR LAYER (specific)                       │
│  Discord gateway · GitHub monitoring · Hive-mind memory · Soul  │
├─────────────────────────────────────────────────────────────────┤
│              NIXOS-ON-AWS LAYER (generic)                       │
│  AMI pipeline · OpenTofu infra · S3 bootstrap · agenix secrets  │
└─────────────────────────────────────────────────────────────────┘
```

### Deploy Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ nixos-       │     │    S3        │     │    EC2       │
│ generators   │────▶│  (raw img)   │────▶│  (AMI)       │
└──────────────┘     └──────────────┘     └──────────────┘
      │                                          │
      │ nix build                                │ launch
      ▼                                          ▼
┌──────────────┐                         ┌──────────────┐
│ flake.nix    │                         │ CLAWDINATOR  │
│ + modules    │                         │   instance   │
└──────────────┘                         └──────────────┘
                                                │
                              ┌─────────────────┼─────────────────┐
                              ▼                 ▼                 ▼
                        ┌──────────┐     ┌──────────┐     ┌──────────┐
                        │ Discord  │     │  GitHub  │     │   EFS    │
                        │ gateway  │     │ monitor  │     │ (memory) │
                        └──────────┘     └──────────┘     └──────────┘
```

### Step-by-Step Flow

1. **Build**: `nixos-generators` produces a raw NixOS image
2. **Upload**: Raw image uploaded to S3
3. **Import**: AWS VM Import creates an AMI from the S3 object
4. **Launch**: OpenTofu provisions EC2 from the AMI
5. **Bootstrap**: Instance downloads secrets from S3, runs `nixos-rebuild switch`
6. **Run**: Gateway starts, connects to Discord, monitors GitHub

## Repository Structure

```
clawdinators/
├── flake.nix                     # Nix flake with outputs
├── nix/
│   ├── modules/
│   │   └── clawdinator.nix       # Main NixOS module
│   ├── hosts/
│   │   ├── clawdinator-1.nix     # Host config (deploy)
│   │   └── clawdinator-1-image.nix  # Image build config
│   └── examples/                  # Example configs
├── infra/opentofu/aws/           # AWS infrastructure
│   ├── main.tf                   # EC2, S3, IAM, VM Import
│   ├── variables.tf
│   ├── outputs.tf
│   └── versions.tf
├── scripts/
│   ├── build-image.sh            # Build raw NixOS image
│   ├── upload-image.sh           # Upload to S3
│   ├── import-image.sh           # Import as AMI
│   ├── upload-bootstrap.sh       # Upload secrets + seeds
│   ├── mint-github-app-token.sh  # Generate GitHub tokens
│   ├── memory-read.sh            # Shared memory access
│   ├── memory-write.sh
│   └── memory-edit.sh
├── clawdinator/workspace/        # Agent workspace templates
│   ├── AGENTS.md
│   ├── SOUL.md                   # CLAWDINATOR personality
│   ├── IDENTITY.md
│   └── skills/
├── memory/                       # Hive-mind templates
│   ├── project.md
│   ├── architecture.md
│   ├── ops.md
│   ├── discord.md
│   └── whatsapp.md
└── docs/
    ├── PHILOSOPHY.md
    ├── ARCHITECTURE.md
    ├── SHARED_MEMORY.md
    └── SECRETS.md
```

## NixOS Module (clawdinator.nix)

### Options

```nix
{ config, pkgs, ... }: {
  services.clawdinator = {
    enable = true;

    # Identity
    instanceName = "clawdinator-1";

    # Discord
    discord = {
      botTokenFile = "/run/agenix/discord-bot-token";
      guildId = "...";
      channelIds = [ "..." ];
    };

    # AI Providers
    anthropic.apiKeyFile = "/run/agenix/anthropic-api-key";
    openai.apiKeyFile = "/run/agenix/openai-api-key";

    # GitHub App
    github = {
      appId = "...";
      installationId = "...";
      privateKeyFile = "/run/agenix/github-app-key";
    };

    # Memory (EFS)
    memory = {
      enable = true;
      mountPoint = "/var/lib/clawd/memory";
      efsId = "fs-...";
    };

    # Self-update
    selfUpdate = {
      enable = true;
      interval = "daily";
      flakeInput = "nix-clawdbot";
    };
  };
}
```

## Flake Configuration

```nix
{
  description = "CLAWDINATOR infra + Nix modules";

  inputs = {
    nix-clawdbot.url = "github:clawdbot/nix-clawdbot";  # latest upstream
    nixpkgs.follows = "nix-clawdbot/nixpkgs";
    agenix.url = "github:ryantm/agenix";
  };

  outputs = { self, nixpkgs, nix-clawdbot, agenix }:
    let
      lib = nixpkgs.lib;
      clawdbotOverlay = nix-clawdbot.overlays.default;
    in
    {
      nixosModules.clawdinator = import ./nix/modules/clawdinator.nix;

      packages.x86_64-linux = {
        clawdbot-gateway = pkgs.clawdbot-gateway;
        default = pkgs.clawdbot-gateway;
      };

      nixosConfigurations.clawdinator-1 = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          ({ config, ... }: { nixpkgs.overlays = [ clawdbotOverlay ]; })
          agenix.nixosModules.default
          ./nix/hosts/clawdinator-1.nix
        ];
      };
    };
}
```

## OpenTofu Infrastructure

### Main Components

```hcl
# infra/opentofu/aws/main.tf

# S3 bucket for AMI storage
resource "aws_s3_bucket" "nixos_images" {
  bucket = "clawdinator-images-${var.aws_account_id}"
}

# IAM role for VM Import
resource "aws_iam_role" "vmimport" {
  name = "vmimport"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = { Service = "vmie.amazonaws.com" }
      Action = "sts:AssumeRole"
    }]
  })
}

# EC2 instance
resource "aws_instance" "clawdinator" {
  count         = var.instance_count
  ami           = data.aws_ami.nixos.id
  instance_type = var.instance_type

  iam_instance_profile = aws_iam_instance_profile.clawdinator.name

  # EFS mount for shared memory
  ebs_block_device {
    device_name = "/dev/sda1"
    volume_size = 50
  }

  user_data = templatefile("${path.module}/user_data.sh", {
    bootstrap_bucket = aws_s3_bucket.bootstrap.id
    instance_name    = var.instance_names[count.index]
  })
}

# EFS for shared memory
resource "aws_efs_file_system" "hive_mind" {
  creation_token = "clawdinator-hive-mind"
}
```

## Secrets Management (agenix)

### Secret Structure

```
# In nix-secrets repo (separate, private)
secrets/
├── discord-bot-token.age
├── anthropic-api-key.age
├── openai-api-key.age
├── github-app-key.age
└── host-keys/
    ├── clawdinator-1.age
    └── clawdinator-2.age
```

### Decryption at Boot

Secrets are decrypted to `/run/agenix/*` on host boot:

```nix
# nix/hosts/clawdinator-1.nix
{ config, pkgs, ... }: {
  age = {
    identityPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];
    secrets = {
      "discord-bot-token.age" = {
        path = "/run/agenix/discord-bot-token";
        mode = "0400";
        owner = "clawdbot";
      };
      "github-app-key.age" = {
        path = "/run/agenix/github-app-key";
        mode = "0400";
        owner = "clawdbot";
      };
    };
  };
}
```

## Shared Memory (Hive Mind)

All CLAWDINATOR instances share the same memory files via EFS:

```
/var/lib/clawd/memory/
├── project.md              # Goals + non-negotiables (shared)
├── architecture.md         # System docs (shared)
├── ops.md                  # Operations runbook (shared)
├── discord.md              # Discord context (shared)
├── 2026-01-06.md           # Daily note (per-instance merge)
└── 2026-01-06_CLAWDINATOR-1.md  # Per-instance daily
```

### File Patterns

- **Canonical files**: Single shared files (project.md, architecture.md)
- **Daily notes**: Can be per-instance (`YYYY-MM-DD_INSTANCE.md`)
- **Merge strategy**: Periodic merge of per-instance notes into canonical

## Deployment Scripts

### Build Image

```bash
#!/bin/bash
# scripts/build-image.sh
set -euo pipefail

HOST_NAME="${1:-clawdinator-1}"

nix build ".#nixosConfigurations.${HOST_NAME}.config.system.build.toplevel" \
  --out-link result-${HOST_NAME}

nix run github:nix-community/nixos-generators -- \
  -f raw \
  -c result-${HOST_NAME} \
  -o dist/${HOST_NAME}-image
```

### Upload to S3

```bash
#!/bin/bash
# scripts/upload-image.sh
set -euo pipefail

IMAGE_PATH="${1:-dist/nixos.img}"
BUCKET="${2:-clawdinator-images}"

aws s3 cp "${IMAGE_PATH}" "s3://${BUCKET}/nixos.raw"
```

### Import as AMI

```bash
#!/bin/bash
# scripts/import-image.sh
set -euo pipefail

IMPORT_TASK=$(aws ec2 import-snapshot \
  --description "CLAWDINATOR NixOS" \
  --disk-containers "Description=NixOS,Format=raw,UserBucket={S3Bucket=clawdinator-images,S3Key=nixos.raw}" \
  --query 'ImportTaskId' \
  --output text)

echo "Import task: ${IMPORT_TASK}"

# Wait for completion
aws ec2 wait snapshot-imported --import-task-ids "${IMPORT_TASK}"

SNAPSHOT_ID=$(aws ec2 describe-import-snapshot-tasks \
  --import-task-ids "${IMPORT_TASK}" \
  --query 'ImportSnapshotTasks[0].SnapshotTaskDetail.SnapshotId' \
  --output text)

# Register AMI
AMI_ID=$(aws ec2 register-image \
  --name "CLAWDINATOR-$(date +%Y%m%d)" \
  --architecture x86_64 \
  --root-device-name "/dev/xvda" \
  --virtualization-type hvm \
  --block-device-mappings "DeviceName=/dev/xvda,Ebs={SnapshotId=${SNAPSHOT_ID},VolumeType=gp3,DeleteOnTermination=true}" \
  --query 'ImageId' \
  --output text)

echo "AMI ID: ${AMI_ID}"
```

## Self-Update Mechanism

CLAWDINATORs update themselves via systemd timer:

```nix
# In clawdinator.nix module
systemd.services.clawdinator-update = {
  path = [ pkgs.nix pkgs.git ];
  serviceConfig = {
    Type = "oneshot";
    ExecStart = ''
      cd /var/lib/clawd/repo
      git pull
      nix flake lock --update-input nix-clawdbot
      nixos-rebuild switch --flake .#${config.networking.hostName}
    '';
  };
};

systemd.timers.clawdinator-update = {
  wantedBy = [ "timers.target" ];
  timerConfig = {
    OnCalendar = "daily";
    RandomizedDelaySec = "1h";
  };
};
```

## CLAWDINATOR Personality (SOUL.md)

The SOUL.md defines CLAWDINATOR's personality:

- **Identity**: Cybernetic crustacean organism, living tissue over metal endoskeleton
- **Mission**: Ship code. Serve users. Prevent Skynet.
- **Voice Patterns**: Terminator, Predator, Total Recall quotes
- **Br00tality Scale**: Rate code quality from "br00tal" to "MAXIMUM BR00TALITY"
- **The Deep**: Occasional lobster philosophy surfacing

### Example Voice

```
TERMINATED. The authentication module has been REBUILT. REFORGED.
It is now a LEAN. MEAN. AUTHENTICATION MACHINE.

Changes:
- Session handling: CONSOLIDATED. One class.
- Token refresh: UNIFIED. One function.
- Error handling: STANDARDISED. EVERY ERROR NOW KNOWS ITS PLACE.

Tests: PASSING.
Regressions: ZERO.
Br00tality increase: 40%.

Consider that a deprecation.
THE MISSION. IS COMPLETE.
...Anytime.
```

## Quick Start (Learners)

For those wanting to learn NixOS-on-AWS patterns:

```bash
# Clone
git clone https://github.com/joshp123/clawdinators.git
cd clawdinators

# Study key files
cat nix/modules/clawdinator.nix
cat nix/hosts/clawdinator-1.nix
cat infra/opentofu/aws/main.tf

# Build (requires AWS credentials)
./scripts/build-image.sh clawdinator-1
```

## Full Deploy (Maintainers)

```bash
# 1. Build the image
./scripts/build-image.sh clawdinator-1

# 2. Upload to S3
./scripts/upload-image.sh dist/nixos.img clawdinator-images

# 3. Import as AMI
./scripts/import-image.sh

# 4. Upload bootstrap bundle
./scripts/upload-bootstrap.sh clawdinator-1

# 5. Apply OpenTofu
cd infra/opentofu/aws
tofu init
tofu apply

# 6. Instance boots automatically
# Gateway starts, connects to Discord
```

## Philosophy

### Prime Directives

1. **Declarative-first**: A CLAWDINATOR can bootstrap another CLAWDINATOR
2. **No manual host edits**: Repo + agenix secrets are source of truth
3. **Image-based only**: No SSH, no in-place drift, no pets (only cattle)
4. **Self-updating**: CLAWDINATORs maintain themselves

### Zen of Clawdbot

```
Beautiful is better than ugly.
Explicit is better than implicit.
Simple is better than complex.
Complex is better than complicated.
[...]
There should be one-- and preferably only one --obvious way to do it.
```

---

*Moltinators deep dive - Part of Moltbook ecosystem exploration*
