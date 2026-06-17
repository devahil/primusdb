# PrimusDB Security Guide

This document provides comprehensive security documentation for PrimusDB, including authentication, authorization, encryption, and best practices.

## Table of Contents

1. [Authentication](#authentication)
2. [Authorization & RBAC](#authorization--rbac)
3. [API Keys](#api-keys)
4. [Encryption](#encryption)
5. [Password Policy](#password-policy)
6. [Session Management](#session-management)
7. [Rate Limiting](#rate-limiting)
8. [Security Best Practices](#security-best-practices)

---

## Authentication

**IMPORTANT: Authentication is ENABLED by default (`auth_required = true`).** Disable only in isolated development environments.

### Login Endpoint

```bash
POST /api/v1/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "SecurePass123!"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIs...",
    "refresh_token": "ref_tkn_abcdef...",
    "token_type": "Bearer",
    "expires_in": 900,
    "user_id": "uuid-here"
  }
}
```

### Register User

```bash
POST /api/v1/auth/register
Content-Type: application/json

{
  "username": "newuser",
  "password": "SecurePass123!",
  "email": "user@example.com"
}
```

### Token Refresh

```bash
POST /api/v1/auth/refresh
Content-Type: application/json

{
  "refresh_token": "ref_tkn_abcdef..."
}
```

### Logout

```bash
POST /api/v1/auth/logout
Authorization: Bearer ACCESS_TOKEN
```

### Using Authentication in Requests

All API requests require authentication when `auth_required = true`:

```bash
# Using Bearer Token
curl -X GET http://localhost:8080/api/v1/crud/document/users \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN"

# Using API Key
curl -X GET http://localhost:8080/api/v1/crud/document/users \
  -H "X-API-Key: YOUR_API_KEY"
```

---

## Authorization & RBAC

PrimusDB implements Role-Based Access Control (RBAC) with three predefined roles:

| Role | Permissions |
|------|-------------|
| **Admin** | Full access to all operations, user management, system configuration |
| **ReadWrite** | Read and write data, cannot modify system settings |
| **ReadOnly** | Read-only access to data |

### Default Role Assignment

New users are assigned the `ReadOnly` role by default. Administrators can upgrade roles:

```bash
# Promote user to ReadWrite
curl -X POST http://localhost:8080/api/v1/auth/users/USER_ID/roles \
  -H "Authorization: Bearer ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"role": "ReadWrite"}'
```

---

## API Keys

API keys provide programmatic access without session tokens.

### Create API Key

```bash
POST /api/v1/auth/api-keys
Authorization: Bearer ACCESS_TOKEN
Content-Type: application/json

{
  "name": "Production Service",
  "scopes": ["read", "write"],
  "expires_in_days": 90,
  "rate_limit": 1000
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "key_id": "key_uuid",
    "key": "prim_abcdef123456...",
    "name": "Production Service",
    "scopes": ["read", "write"],
    "expires_at": "2026-03-01T00:00:00Z"
  }
}
```

**WARNING: Store the API key securely. It is only shown once upon creation.**

### List API Keys

```bash
GET /api/v1/auth/api-keys
Authorization: Bearer ACCESS_TOKEN
```

### Revoke API Key

```bash
DELETE /api/v1/auth/api-keys/KEY_ID
Authorization: Bearer ACCESS_TOKEN
```

---

## Encryption

### Data at Rest

PrimusDB uses AES-256-GCM encryption for stored data:

```toml
[security]
encryption_enabled = true

[security.encryption]
algorithm = "aes-256-gcm"
key_rotation_interval = 86400  # 24 hours in seconds
```

### TLS/SSL

Enable TLS for encrypted network communication:

```toml
[network.tls]
enabled = true
certificate_path = "/etc/ssl/primusdb.crt"
key_path = "/etc/ssl/private/primusdb.key"
min_version = "1.2"
```

### Certificate Setup

```bash
# Generate self-signed certificate for testing
openssl req -x509 -newkey rsa:4096 \
  -keyout /etc/ssl/private/primusdb.key \
  -out /etc/ssl/primusdb.crt \
  -days 365 \
  -nodes \
  -subj "/CN=primusdb"
```

---

## Password Policy

PrimusDB enforces strong password requirements:

| Requirement | Value |
|-------------|-------|
| Minimum length | 12 characters |
| Uppercase letters | Required |
| Lowercase letters | Required |
| Numbers | Required |
| Special characters | Required |
| Maximum failed attempts | 5 |
| Lockout duration | 30 minutes |

### Password Validation Example

Valid passwords:
- ✅ `SecurePass123!`
- ✅ `MyStr0ng@P@ssw0rd`
- ✅ `C0mpl3x!Pass#2024`

Invalid passwords:
- ❌ `password123` (no uppercase, no special char)
- ❌ `PASSWORD123` (no lowercase)
- ❌ `Pass123` (too short)

---

## Session Management

### Token Lifetimes

| Token Type | Lifetime | Purpose |
|------------|----------|---------|
| Access Token | 15 minutes | API authentication |
| Refresh Token | 7 days | Session refresh |
| MFA Token | 5 minutes | Two-factor verification |

### Session Limits

```toml
[security.auth]
session_max_per_user = 5  # Maximum concurrent sessions
```

### Session Revocation

```bash
# Revoke specific session
DELETE /api/v1/auth/sessions/SESSION_ID
Authorization: Bearer ACCESS_TOKEN

# Revoke all sessions (logout everywhere)
DELETE /api/v1/auth/sessions
Authorization: Bearer ACCESS_TOKEN
```

---

## Rate Limiting

### Default Limits

| Endpoint Type | Requests/Minute |
|--------------|-----------------|
| Authentication | 5 |
| General API | 1000 |
| Queries | 5000 |

### Custom Rate Limits

```toml
[api.rate_limiting]
auth_requests_per_minute = 5
api_requests_per_minute = 1000
query_requests_per_minute = 5000
burst_size = 100
```

### Rate Limit Headers

Responses include rate limit information:

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1699900000
```

---

## Security Best Practices

### 1. Production Deployment

```bash
# Enable authentication (default)
./target/release/primusdb-server --auth-required

# Use strong JWT secret
./target/release/primusdb-server --jwt-secret "your-64-character-secret-key"

# Enable encryption
./target/release/primusdb-server --encryption-enabled

# Enable TLS
./target/release/primusdb-server --tls-enabled --tls-cert /path/to/cert --tls-key /path/to/key
```

### 2. User Management

```bash
# Create admin user during setup
./target/release/primusdb-cli admin create \
  --username admin \
  --email admin@company.com \
  --role admin

# Create read-only application user
./target/release/primusdb-cli user create \
  --username app_service \
  --email app@company.com \
  --role readwrite
```

### 3. API Key Rotation

Rotate API keys regularly:

```bash
# Create new key
./target/release/primusdb-cli api-key create \
  --name "Rotation 2024-01" \
  --expires-in 90days

# Update applications to use new key

# Revoke old key after transition
./target/release/primusdb-cli api-key revoke --id OLD_KEY_ID
```

### 4. Monitoring & Auditing

Enable audit logging:

```toml
[security.audit]
enabled = true
log_auth_events = true
log_data_access = true
log_admin_operations = true
retention_days = 90
```

### 5. Network Security

```bash
# Bind to localhost only (development)
./target/release/primusdb-server --bind 127.0.0.1

# Bind to specific interface (production)
./target/release/primusdb-server --bind 10.0.0.5

# Use firewall to restrict access
sudo ufw allow from 10.0.0.0/24 to any port 8080
```

### 6. Environment Variables

Never commit credentials to version control:

```bash
# Set secure environment variables
export PRIMUSDB_JWT_SECRET="your-secure-secret"
export PRIMUSDB_DB_ENCRYPTION_KEY="your-encryption-key"
```

---

## Security Checklist

- [ ] Authentication enabled (`auth_required = true`)
- [ ] Strong JWT secret (64+ characters)
- [ ] Encryption enabled for data at rest
- [ ] TLS enabled for network communication
- [ ] Complex passwords enforced
- [ ] MFA enabled for admin accounts
- [ ] API keys with appropriate scopes
- [ ] Regular API key rotation
- [ ] Audit logging enabled
- [ ] Rate limiting configured
- [ ] Firewall rules applied
- [ ] No default passwords in use
- [ ] User roles properly assigned
- [ ] Session limits configured
- [ ] Monitored authentication failures

---

## Incident Response

### Detecting Compromises

Monitor these events:
- Multiple failed login attempts
- Login from unusual locations
- Unexpected session creations
- API key usage from new IPs

### Response Steps

1. **Immediate**: Revoke compromised credentials
2. **Investigate**: Review audit logs
3. **Contain**: Block suspicious IPs
4. **Notify**: Alert affected users
5. **Remediate**: Reset compromised accounts
6. **Document**: Record incident details

---

## References

- [API Reference](API_REFERENCE.md) - Complete API documentation
- [ADMIN.md](ADMIN.md) - Administration guide
- [ARCHITECTURE.md](ARCHITECTURE.md) - Security architecture
