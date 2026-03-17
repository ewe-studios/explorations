---
type: reference
created: 2026-03-17
---

# Database Schema Reference

Complete D2/SQLite schema for Better Auth with all plugins.

## Core Tables

### Users

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    username TEXT UNIQUE,
    password_hash TEXT,
    email_verified INTEGER DEFAULT 0,
    email_verified_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT,

    -- Soft delete support
    deleted_at INTEGER,

    -- Account lockout
    failed_login_attempts INTEGER DEFAULT 0,
    locked_until INTEGER
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
```

### Sessions

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT UNIQUE NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    metadata TEXT,

    -- Session tracking
    last_active_at INTEGER,
    refreshed_at INTEGER
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_token ON sessions(token);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
```

### Accounts (OAuth)

```sql
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider_id TEXT NOT NULL,
    provider_account_id TEXT NOT NULL,
    access_token TEXT,
    refresh_token TEXT,
    expires_at INTEGER,
    scope TEXT,
    token_type TEXT,
    id_token TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(provider_id, provider_account_id)
);

CREATE INDEX idx_accounts_user_id ON accounts(user_id);
CREATE INDEX idx_accounts_provider ON accounts(provider_id, provider_account_id);
```

### Verification Tokens

```sql
CREATE TABLE verification_tokens (
    id TEXT PRIMARY KEY,
    identifier TEXT NOT NULL,
    token TEXT UNIQUE NOT NULL,
    type TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    consumed_at INTEGER,
    metadata TEXT
);

CREATE INDEX idx_verification_tokens_token ON verification_tokens(token);
CREATE INDEX idx_verification_tokens_identifier ON verification_tokens(identifier);
CREATE INDEX idx_verification_tokens_type ON verification_tokens(type);
```

## Plugin Tables

### Two-Factor Authentication

```sql
CREATE TABLE two_factor_secrets (
    id TEXT PRIMARY KEY,
    user_id TEXT UNIQUE NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    secret TEXT NOT NULL,
    backup_codes TEXT,
    enabled INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL
);

-- TOTP attempts tracking (rate limiting)
CREATE TABLE two_factor_attempts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ip_address TEXT,
    attempted_at INTEGER NOT NULL,
    success INTEGER NOT NULL
);

CREATE INDEX idx_two_factor_attempts_user ON two_factor_attempts(user_id);
CREATE INDEX idx_two_factor_attempts_time ON two_factor_attempts(attempted_at);
```

### Organization

```sql
CREATE TABLE organizations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT UNIQUE,
    logo TEXT,
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT,
    owner_id TEXT REFERENCES users(id)
);

CREATE TABLE organization_members (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    invited_at INTEGER NOT NULL,
    invited_by TEXT REFERENCES users(id),
    UNIQUE(organization_id, user_id)
);

CREATE TABLE organization_invitations (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role TEXT NOT NULL,
    invited_by TEXT NOT NULL REFERENCES users(id),
    invited_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    status TEXT DEFAULT 'pending',
    UNIQUE(organization_id, email)
);

-- Optional: Organization domains for auto-join
CREATE TABLE organization_domains (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    domain TEXT NOT NULL,
    verified INTEGER DEFAULT 0,
    verification_token TEXT,
    auto_join INTEGER DEFAULT 0,
    auto_join_role TEXT,
    UNIQUE(organization_id, domain)
);

CREATE INDEX idx_org_members_user ON organization_members(user_id);
CREATE INDEX idx_org_members_org ON organization_members(organization_id);
CREATE INDEX idx_org_invitations_email ON organization_invitations(email);
CREATE INDEX idx_org_domains_domain ON organization_domains(domain);
```

### API Keys

```sql
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    key_hash TEXT UNIQUE NOT NULL,
    name TEXT,
    prefix TEXT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permissions TEXT,
    expires_at INTEGER,
    last_used_at INTEGER,
    created_at INTEGER NOT NULL,
    metadata TEXT
);

CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_expires_at ON api_keys(expires_at);
```

### Admin (Audit Log)

```sql
CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT REFERENCES users(id),
    action TEXT NOT NULL,
    resource_type TEXT,
    resource_id TEXT,
    changes TEXT,
    ip_address TEXT,
    user_agent TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_audit_logs_user ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_created ON audit_logs(created_at);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
```

### Passkey (WebAuthn)

```sql
CREATE TABLE passkeys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT,
    credential_id TEXT NOT NULL,
    public_key TEXT NOT NULL,
    counter INTEGER NOT NULL,
    device_type TEXT,
    backed_up INTEGER DEFAULT 0,
    transports TEXT,
    created_at INTEGER NOT NULL,
    last_used_at INTEGER,
    UNIQUE(credential_id)
);

CREATE INDEX idx_passkeys_user_id ON passkeys(user_id);
```

### Phone Number

```sql
CREATE TABLE user_phones (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    phone_number TEXT NOT NULL,
    verified INTEGER DEFAULT 0,
    verified_at INTEGER,
    primary_phone INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    UNIQUE(user_id, phone_number)
);

CREATE INDEX idx_user_phones_user ON user_phones(user_id);
CREATE INDEX idx_user_phones_phone ON user_phones(phone_number);
```

### Magic Link (separate from verification_tokens for tracking)

```sql
CREATE TABLE magic_links (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    token TEXT UNIQUE NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    consumed_at INTEGER,
    ip_address TEXT,
    user_agent TEXT
);

CREATE INDEX idx_magic_links_email ON magic_links(email);
CREATE INDEX idx_magic_links_token ON magic_links(token);
```

### Rate Limiting (if using database storage)

```sql
CREATE TABLE rate_limits (
    id TEXT PRIMARY KEY,
    key TEXT UNIQUE NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    window_start INTEGER NOT NULL,
    window_end INTEGER NOT NULL
);

CREATE INDEX idx_rate_limits_key ON rate_limits(key);
CREATE INDEX idx_rate_limits_window ON rate_limits(window_end);
```

### OAuth State (PKCE)

```sql
CREATE TABLE oauth_state (
    id TEXT PRIMARY KEY,
    state TEXT UNIQUE NOT NULL,
    code_verifier TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_oauth_state_state ON oauth_state(state);
CREATE INDEX idx_oauth_state_expires ON oauth_state(expires_at);
```

### Email OTP

```sql
CREATE TABLE email_otps (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    otp TEXT NOT NULL,
    type TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    consumed_at INTEGER,
    attempts INTEGER DEFAULT 0
);

CREATE INDEX idx_email_otps_email ON email_otps(email);
CREATE INDEX idx_email_otps_expires ON email_otps(expires_at);
```

## Full-Text Search

```sql
-- Users full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS users_fts USING fts5(
    email,
    username,
    content='users',
    content_rowid='rowid'
);

-- Triggers for FTS synchronization
CREATE TRIGGER users_ai AFTER INSERT ON users BEGIN
    INSERT INTO users_fts(rowid, email, username)
    VALUES (new.rowid, new.email, new.username);
END;

CREATE TRIGGER users_ad AFTER DELETE ON users BEGIN
    INSERT INTO users_fts(users_fts, rowid, email, username)
    VALUES('delete', old.rowid, old.email, old.username);
END;

CREATE TRIGGER users_au AFTER UPDATE ON users BEGIN
    INSERT INTO users_fts(users_fts, rowid, email, username)
    VALUES('delete', old.rowid, old.email, old.username);
    INSERT INTO users_fts(rowid, email, username)
    VALUES (new.rowid, new.email, new.username);
END;
```

## Views

```sql
-- Active sessions view
CREATE VIEW active_sessions AS
SELECT
    s.id,
    s.user_id,
    s.token,
    s.expires_at,
    s.created_at,
    s.ip_address,
    s.user_agent,
    u.email,
    u.username
FROM sessions s
JOIN users u ON s.user_id = u.id
WHERE s.expires_at > (strftime('%s', 'now') * 1000)
  AND u.deleted_at IS NULL;

-- Organization members with user details
CREATE VIEW organization_members_full AS
SELECT
    om.id,
    om.organization_id,
    om.user_id,
    om.role,
    om.invited_at,
    o.name as organization_name,
    o.slug as organization_slug,
    u.email,
    u.username
FROM organization_members om
JOIN organizations o ON om.organization_id = o.id
JOIN users u ON om.user_id = u.id;

-- API keys with user info
CREATE VIEW api_keys_full AS
SELECT
    ak.id,
    ak.key_hash,
    ak.prefix,
    ak.name,
    ak.user_id,
    ak.permissions,
    ak.expires_at,
    ak.last_used_at,
    ak.created_at,
    u.email,
    u.username
FROM api_keys ak
JOIN users u ON ak.user_id = u.id
WHERE ak.expires_at IS NULL OR ak.expires_at > (strftime('%s', 'now') * 1000);
```

## Cleanup Queries (for maintenance)

```sql
-- Delete expired sessions
DELETE FROM sessions WHERE expires_at < (strftime('%s', 'now') * 1000);

-- Delete expired verification tokens
DELETE FROM verification_tokens WHERE expires_at < (strftime('%s', 'now') * 1000);

-- Delete expired magic links
DELETE FROM magic_links WHERE expires_at < (strftime('%s', 'now') * 1000);

-- Delete expired OAuth state
DELETE FROM oauth_state WHERE expires_at < (strftime('%s', 'now') * 1000);

-- Delete expired email OTPs
DELETE FROM email_otps WHERE expires_at < (strftime('%s', 'now') * 1000);

-- Delete old audit logs (keep last 90 days)
DELETE FROM audit_logs WHERE created_at < ((strftime('%s', 'now') * 1000) - (90 * 24 * 60 * 60 * 1000));

-- Reset rate limits
DELETE FROM rate_limits WHERE window_end < (strftime('%s', 'now') * 1000);
```

## Migration Management

```sql
-- Track applied migrations
CREATE TABLE _migrations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

-- Insert initial migration
INSERT INTO _migrations (id, name) VALUES ('001', 'initial_schema');
```

## Rust Schema Types

```rust
// crates/better-auth-db/src/schema.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: Option<String>,
    pub password_hash: Option<String>,
    pub email_verified: bool,
    pub email_verified_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: Option<serde_json::Value>,
    pub deleted_at: Option<i64>,
    pub failed_login_attempts: i32,
    pub locked_until: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub expires_at: i64,
    pub created_at: i64,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub last_active_at: Option<i64>,
    pub refreshed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub user_id: String,
    pub provider_id: String,
    pub provider_account_id: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub scope: Option<String>,
    pub token_type: Option<String>,
    pub id_token: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationToken {
    pub id: String,
    pub identifier: String,
    pub token: String,
    pub token_type: String,  // 'email_otp', 'magic_link', 'password_reset'
    pub expires_at: i64,
    pub created_at: i64,
    pub consumed_at: Option<i64>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorSecret {
    pub id: String,
    pub user_id: String,
    pub secret: String,
    pub backup_codes: Option<String>,  // Encrypted JSON
    pub enabled: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub slug: Option<String>,
    pub logo: Option<String>,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: Option<serde_json::Value>,
    pub owner_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMember {
    pub id: String,
    pub organization_id: String,
    pub user_id: String,
    pub role: String,  // 'owner', 'admin', 'member'
    pub invited_at: i64,
    pub invited_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub key_hash: String,
    pub name: Option<String>,
    pub prefix: Option<String>,
    pub user_id: String,
    pub permissions: Option<String>,  // JSON array
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub metadata: Option<serde_json::Value>,
}
```
