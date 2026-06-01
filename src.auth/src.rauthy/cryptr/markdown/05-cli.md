---
title: CLI
prev: 04-s3.md
---

# CLI

Command-line interface for cryptr.

## Installation

```bash
# Install from crates.io
cargo install cryptr --features cli

# Or build from source
cd cryptr
cargo build --release --features cli
```

## Commands

| Command | Purpose |
|---------|---------|
| `encrypt` | Encrypt values/files |
| `decrypt` | Decrypt values/files |
| `key` | Key management |
| `config` | Configuration |

## Encrypt

### Value Encryption

```bash
# Encrypt a value from stdin
echo "secret data" | cryptr encrypt --output encrypted.bin

# Encrypt with base64 output
echo "secret" | cryptr encrypt --base64
# Result: Y1JZUEAxAAAAeHh4eHh4eHh4eHh4eHh4eHg...=

# Encrypt file
cryptr encrypt --input data.txt --output data.txt.enc

# Encrypt to stdout
cryptr encrypt --input data.txt --output -
```

### File Encryption

```bash
# Encrypt file with progress bar
cryptr encrypt --input large-file.tar.gz --output backup.enc --progress

# Encrypt and stream to S3
cryptr encrypt \
    --input database.sql \
    --s3 s3://bucket/backups/db-2025-01-15.sql.enc
```

## Decrypt

```bash
# Decrypt file
cryptr decrypt --input data.txt.enc --output data.txt

# Decrypt from S3
cryptr decrypt \
    --s3 s3://bucket/backups/db-2025-01-15.sql.enc \
    --output database.sql

# Decrypt base64 value
echo "Y1JZUEAxAAAA..." | cryptr decrypt --base64
```

## Key Management

### Generate Key

```bash
# Generate new key
cryptr key generate
# Key ID: 3
# Key: base64encoded...

# Generate with custom ID
cryptr key generate --id 100
```

### List Keys

```bash
cryptr key list
# ID  | Created At           | Algorithm
# ----|----------------------|------------
# 1   | 2025-01-15T10:00:00Z | ChaCha20Poly1305
# 2   | 2025-06-01T12:00:00Z | ChaCha20Poly1305
# 3   | 2025-06-01T15:00:00Z | ChaCha20Poly1305
```

### Set Active Key

```bash
# Set active encryption key
cryptr key set-active 3
```

## Configuration

### Initialize Config

```bash
# Create config file
cryptr config init

# With custom path
cryptr config init --path /etc/cryptr/cryptr.toml
```

### Config File Format

```toml
# cryptr.toml
[active]
key_id = 2

[key.1]
id = 1
created_at = "2025-01-15T10:00:00Z"
key = "base64encodedkey..."

[key.2]
id = 2
created_at = "2025-06-01T12:00:00Z"
key = "base64encodedkey..."

[s3]
endpoint = "https://s3.amazonaws.com"
region = "us-east-1"
access_key_id = "AKIA..."
secret_access_key = "..."
```

### Environment Variables

```bash
export CRYPTR_CONFIG="/etc/cryptr/cryptr.toml"
export CRYPTR_ACTIVE_KEY_ID="2"
export CRYPTR_KEY_1="base64encodedkey"
export CRYPTR_KEY_2="base64encodedkey"
```

## Interactive Mode

```bash
# Interactive encryption
cryptr encrypt --interactive
# Enter value (hidden): ********
# Confirm: ********
# Encrypted: Y1JZUEAxAAAA...

# Interactive key generation
cryptr key generate --interactive
# Generate password-protected key
# Enter password: ********
# Confirm: ********
```

## Examples

### Database Backup Script

```bash
#!/bin/bash
# backup.sh

set -e

# Config
DB_NAME="mydb"
S3_BUCKET="my-backups"
DATE=$(date +%Y-%m-%d)
KEY_ID=$(cryptr config get active_key_id)

# Dump and encrypt
echo "Backing up database..."
pg_dump $DB_NAME | cryptr encrypt --s3 "s3://${S3_BUCKET}/db/${DB_NAME}-${DATE}.sql.enc"

echo "Backup complete: ${DB_NAME}-${DATE}.sql.enc (Key ID: ${KEY_ID})"
```

### Restore Script

```bash
#!/bin/bash
# restore.sh

BACKUP_FILE=$1
OUTPUT_FILE=$2

echo "Downloading and decrypting..."
cryptr decrypt --s3 "s3://${BACKUP_FILE}" --output "${OUTPUT_FILE}"

echo "Restoring database..."
psql mydb < "${OUTPUT_FILE}"

echo "Restore complete"
```

## Options Reference

### Global Options

| Option | Description |
|--------|-------------|
| `--config <path>` | Config file path |
| `--verbose` | Verbose output |
| `--quiet` | Suppress output |

### Encrypt Options

| Option | Description |
|--------|-------------|
| `--input <path>` | Input file (or - for stdin) |
| `--output <path>` | Output file (or - for stdout) |
| `--s3 <url>` | S3 destination |
| `--base64` | Base64 encode output |
| `--progress` | Show progress bar |

### Decrypt Options

| Option | Description |
|--------|-------------|
| `--input <path>` | Input file (or - for stdin) |
| `--output <path>` | Output file (or - for stdout) |
| `--s3 <url>` | S3 source |
| `--base64` | Input is base64 encoded |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Encryption error |
| 4 | Decryption error |
| 5 | Key error |
| 6 | S3 error |
