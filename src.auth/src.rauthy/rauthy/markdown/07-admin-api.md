---
title: Admin API
prev: 06-pam.md
next: 08-deployment.md
---

# Admin API

Administrative API and user management.

## Admin UI

Web-based administration interface at `/admin`.

### Features

- User management
- Client configuration
- Group/role management
- Event log viewing
- System configuration
- Session management

## API Authentication

```http
GET /api/v1/users
Authorization: Bearer ADMIN_ACCESS_TOKEN
```

**Aha:** Admin API uses same OIDC tokens with admin scope.

## User Management

### List Users

```http
GET /api/v1/users?limit=50&offset=0
Authorization: Bearer TOKEN
```

```json
{
  "data": [
    {
      "id": "user_123",
      "email": "alice@example.com",
      "email_verified": true,
      "mfa_enabled": true,
      "created_at": "2025-01-15T10:30:00Z",
      "last_login": "2025-01-20T14:22:00Z",
      "groups": ["users", "admins"]
    }
  ],
  "total": 150,
  "limit": 50,
  "offset": 0
}
```

### Create User

```http
POST /api/v1/users
Authorization: Bearer TOKEN
Content-Type: application/json

{
  "email": "bob@example.com",
  "password": "SecurePass123!",
  "groups": ["users"],
  "email_verified": false
}
```

### Update User

```http
PUT /api/v1/users/user_123
Authorization: Bearer TOKEN
Content-Type: application/json

{
  "groups": ["users", "admins"],
  "mfa_enabled": true
}
```

### Delete User

```http
DELETE /api/v1/users/user_123
Authorization: Bearer TOKEN
```

### Reset Password

```http
POST /api/v1/users/user_123/reset-password
Authorization: Bearer TOKEN
Content-Type: application/json

{
  "new_password": "NewSecurePass123!",
  "force_logout": true
}
```

## Client Management

### List Clients

```http
GET /api/v1/clients
Authorization: Bearer TOKEN
```

### Create Client

```http
POST /api/v1/clients
Authorization: Bearer TOKEN
Content-Type: application/json

{
  "name": "My Application",
  "client_id": "myapp",
  "client_secret": "secret123",
  "redirect_uris": ["https://app.example.com/callback"],
  "grant_types": ["authorization_code", "refresh_token"],
  "scopes": ["openid", "profile", "email"],
  "is_public": false
}
```

### Update Client

```http
PUT /api/v1/clients/myapp
Authorization: Bearer TOKEN
Content-Type: application/json

{
  "redirect_uris": [
    "https://app.example.com/callback",
    "https://app.example.com/oauth2/callback"
  ]
}
```

## Group Management

### List Groups

```http
GET /api/v1/groups
Authorization: Bearer TOKEN
```

```json
{
  "data": [
    {
      "id": "admins",
      "name": "Administrators",
      "description": "System administrators",
      "user_count": 5
    },
    {
      "id": "users",
      "name": "Users",
      "description": "Regular users",
      "user_count": 145
    }
  ]
}
```

### Create Group

```http
POST /api/v1/groups
Authorization: Bearer TOKEN
Content-Type: application/json

{
  "id": "developers",
  "name": "Developers",
  "description": "Development team"
}
```

### Add User to Group

```http
POST /api/v1/users/user_123/groups
Authorization: Bearer TOKEN
Content-Type: application/json

{
  "group_id": "developers"
}
```

## Session Management

### List Sessions

```http
GET /api/v1/sessions?user_id=user_123
Authorization: Bearer TOKEN
```

```json
{
  "data": [
    {
      "id": "sess_abc123",
      "user_id": "user_123",
      "client_id": "myapp",
      "created_at": "2025-01-20T10:00:00Z",
      "expires_at": "2025-01-20T14:00:00Z",
      "ip_address": "192.168.1.100",
      "user_agent": "Mozilla/5.0..."
    }
  ]
}
```

### Revoke Session

```http
DELETE /api/v1/sessions/sess_abc123
Authorization: Bearer TOKEN
```

### Revoke All Sessions

```http
POST /api/v1/users/user_123/revoke-sessions
Authorization: Bearer TOKEN
```

## Event Log

### List Events

```http
GET /api/v1/events?limit=100&offset=0
Authorization: Bearer TOKEN
```

```json
{
  "data": [
    {
      "id": "evt_123",
      "timestamp": "2025-01-20T14:30:00Z",
      "level": "info",
      "event": "user.login",
      "user_id": "user_123",
      "ip_address": "192.168.1.100",
      "details": {
        "method": "password",
        "mfa_used": true
      }
    }
  ]
}
```

### Event Types

| Event | Description |
|-------|-------------|
| `user.login` | User logged in |
| `user.logout` | User logged out |
| `user.created` | User created |
| `user.updated` | User updated |
| `user.deleted` | User deleted |
| `client.created` | Client created |
| `token.issued` | Token issued |
| `token.revoked` | Token revoked |
| `session.created` | Session created |
| `session.revoked` | Session revoked |
| `admin.action` | Admin action |

## System Configuration

### Get Config

```http
GET /api/v1/config
Authorization: Bearer TOKEN
```

```json
{
  "issuer_url": "https://auth.example.com",
  "token_expiry": 900,
  "refresh_token_expiry": 604800,
  "mfa_required": false,
  "password_policy": {
    "min_length": 12,
    "require_uppercase": true,
    "require_lowercase": true,
    "require_number": true,
    "require_special": true
  }
}
```

### Update Config

```http
PUT /api/v1/config
Authorization: Bearer TOKEN
Content-Type: application/json

{
  "mfa_required": true,
  "password_policy": {
    "min_length": 14
  }
}
```

## API Keys

### Create API Key

```http
POST /api/v1/api-keys
Authorization: Bearer TOKEN
Content-Type: application/json

{
  "name": "Service Integration",
  "scopes": ["read:users", "read:groups"],
  "expires_at": "2025-12-31T23:59:59Z"
}
```

Response:

```json
{
  "id": "key_123",
  "key": "rauthy_api_abc123...",
  "name": "Service Integration",
  "scopes": ["read:users", "read:groups"],
  "created_at": "2025-01-20T10:00:00Z",
  "expires_at": "2025-12-31T23:59:59Z"
}
```

**Aha:** API key is only shown once on creation. Store it securely.

### Revoke API Key

```http
DELETE /api/v1/api-keys/key_123
Authorization: Bearer TOKEN
```

## Health Check

```http
GET /health
```

```json
{
  "status": "healthy",
  "database": "connected",
  "cache": "connected",
  "version": "0.32.0"
}
```

## Rate Limiting

Admin API has rate limiting:

| Endpoint | Limit |
|----------|-------|
| Read operations | 100/min |
| Write operations | 10/min |
| Auth attempts | 5/min |

## Next Steps

Continue to [Deployment →](08-deployment.html) for setup and configuration.
