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
9. [Security Configuration Reference](#security-configuration-reference)
10. [Security Checklist](#security-checklist)
11. [Alpha Limitations](#alpha-limitations)
12. [Incident Response](#incident-response)

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

### CLI Authentication

```bash
# Login with username (password prompted)
primusdb auth login admin

# Login with inline password
primusdb auth login admin --password SecurePass123!

# Login with custom realm
primusdb auth login admin --realm internal

# Login with custom session TTL
primusdb auth login admin --ttl 3600
```

| Flag | Description | Default |
|------|-------------|---------|
| `-p, --password <PASSWORD>` | Password (prompted if omitted) | — |
| `-r, --realm <REALM>` | Authentication realm | `default` |
| `--ttl <SECONDS>` | Session time-to-live | `86400` |

### CLI Logout

```bash
# Invalidate current session
primusdb auth logout

# Invalidate all sessions
primusdb auth logout --all
```

### Whoami

```bash
# Show current user identity
primusdb auth whoami

# Verbose identity information
primusdb auth whoami --verbose
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

PrimusDB implements Role-Based Access Control (RBAC) with fine-grained permission management.

### Built-in Roles

| Role | Permissions |
|------|-------------|
| **Admin** | Full access to all operations, user management, system configuration |
| **Developer** | Full data access (Read, Write, Create, Delete) |
| **ReadWrite** | Read and write data, cannot modify system settings |
| **ReadOnly** | Read-only access to data |
| **Analyst** | Read-only access |
| **ClusterNode** | Cluster operations |

### Default Role Assignment

New users are assigned the `ReadOnly` role by default. Administrators can upgrade roles:

```bash
# Promote user to ReadWrite via API
curl -X POST http://localhost:8080/api/v1/auth/users/USER_ID/roles \
  -H "Authorization: Bearer ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"role": "ReadWrite"}'
```

### CLI Role Management

```bash
# Create a role
primusdb role create analyst --description "Data analyst role"

# Create a role that inherits from another
primusdb role create senior-analyst --inherits analyst

# List all roles
primusdb role list

# List roles with permissions
primusdb role list --permissions

# Grant a permission to a role
primusdb role grant analyst "read"

# Grant a namespace-scoped permission
primusdb role grant analyst "read" --namespace tenant1

# Revoke a permission from a role
primusdb role revoke analyst "write"
```

### CLI User Management

```bash
# Create a user with a role
primusdb user create alice --role analyst --email alice@example.com

# Create a user with a password
primusdb user create bob --password SecurePass123! --role developer

# List users
primusdb user list

# Filter users by role
primusdb user list --role admin

# Disable a user
primusdb user disable alice --reason "Account inactive"

# Re-enable a user
primusdb user disable alice --reenable

# Grant a role to a user
primusdb user roles alice --grant analyst

# Revoke a role from a user
primusdb user roles alice --revoke viewer

# List user's roles
primusdb user roles alice --list
```

### REST API User Management

```bash
# Register a new user via API
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ADMIN_TOKEN" \
  -d '{
    "username": "charlie",
    "password": "SecurePass123!",
    "email": "charlie@example.com",
    "roles": ["readonly"]
  }'

# List all users (admin only)
curl -X GET http://localhost:8080/api/v1/auth/users \
  -H "Authorization: Bearer ADMIN_TOKEN"

# List all roles
curl -X GET http://localhost:8080/api/v1/auth/roles
```

---

## API Keys

API keys provide programmatic access without session tokens.

### Create API Key

```bash
POST /api/v1/auth/token/create
Authorization: Bearer ACCESS_TOKEN
Content-Type: application/json

{
  "name": "Production Service",
  "scopes": ["read", "write"],
  "expires_in_hours": 2160,
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
GET /api/v1/auth/tokens
Authorization: Bearer ACCESS_TOKEN
```

### Revoke API Key

```bash
DELETE /api/v1/auth/token/revoke/KEY_ID
Authorization: Bearer ACCESS_TOKEN
```

### Token Scopes

API keys can be scoped to specific resource types and actions:

| Resource | Description |
|----------|-------------|
| `All` | All resource types |
| `Columnar` | Columnar storage engine |
| `Vector` | Vector storage engine |
| `Document` | Document storage engine |
| `Relational` | Relational storage engine |
| `Cluster` | Cluster management operations |
| `Admin` | Administrative operations |

| Action | Description |
|--------|-------------|
| `Read` | Query and read data |
| `Write` | Insert and update data |
| `Delete` | Delete data |
| `Create` | Create tables and databases |
| `Admin` | All operations including user management |

```json
{
  "scopes": [
    {"resource": "Document", "actions": ["Read", "Write"]},
    {"resource": "Columnar", "actions": ["Read"]}
  ]
}
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

### Collection-Level Encryption

Encryption can be toggled per document collection:

```bash
# Enable encryption for a document collection
curl -X POST http://localhost:8080/api/v1/collection/my_collection/encrypt \
  -H "Authorization: Bearer YOUR_TOKEN"

# Disable encryption for a document collection
curl -X POST http://localhost:8080/api/v1/collection/my_collection/decrypt \
  -H "Authorization: Bearer YOUR_TOKEN"
```

### Encryption Details

- **Algorithm**: AES-256-GCM (authenticated encryption)
- **Key derivation**: Per-file keys derived from a master key using HKDF
- **Integrity**: SHA-256 checksums detect tampering
- **File header**: Encrypted files are identified by the `PREN` magic bytes
- **Performance**: Encryption is transparent to queries; decryption happens on read

### TLS/SSL

PrimusDB supports native TLS for HTTPS serving and mutual TLS (mTLS) for
inter-node federation communication.

#### Quick Start (Self-Signed Cert)

```bash
# Generate a self-signed certificate
primusdb certs create-selfsigned --out-dir ./tls --hosts localhost 127.0.0.1

# Start the server with TLS enabled
primusdb server start --tls-enabled --tls-cert ./tls/selfsigned.pem --tls-key ./tls/selfsigned.key
```

#### Production Setup (CA-Signed)

```bash
# 1. Create a Certificate Authority
primusdb certs create-ca --out-dir ./ca --name "MyOrg CA"

# 2. Generate a server certificate signed by the CA
primusdb certs create-cert \
  --ca-dir ./ca \
  --out-dir ./tls \
  --hosts primusdb.example.com 10.0.1.5 \
  --server

# 3. Start the server with TLS + mTLS enabled
primusdb server start \
  --tls-enabled \
  --tls-cert ./tls/cert.pem \
  --tls-key ./tls/key.pem \
  --tls-ca ./ca/ca.crt \
  --mtls-enabled
```

#### Configuration

```toml
[network]
tls_enabled = true
tls_cert_path = "/etc/ssl/primusdb.crt"
tls_key_path = "/etc/ssl/private/primusdb.key"
tls_ca_path = "/etc/ssl/ca.crt"      # Required for mTLS
mtls_enabled = false                  # Require client certs
```

When `mtls_enabled` is `true`, all clients must present a valid certificate
signed by the configured CA. This provides the strongest authentication and
is recommended for federation peer communication.

#### Federation mTLS

Federation peers can authenticate each other using mTLS:

```bash
# Generate client certs for each federation node
primusdb certs create-cert \
  --ca-dir ./ca \
  --out-dir ./node1 \
  --name "node-1" \
  --client

primusdb certs create-cert \
  --ca-dir ./ca \
  --out-dir ./node2 \
  --name "node-2" \
  --client

# Start node 1 with federation and mTLS
primusdb server start \
  --tls-enabled --tls-cert ./node1/cert.pem --tls-key ./node1/key.pem \
  --tls-ca ./ca/ca.crt --mtls-enabled \
  --federation-discovery node2:8081 \
  --cluster-id cluster-1
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
# Start the server (authentication is enabled by default)
./target/release/primusdb server start --bind 0.0.0.0:8080

# Use a long-lived token expiry via configuration (AuthConfig)

# Enable TLS
./target/release/primusdb server start --bind 0.0.0.0:8080 \
  --tls-enabled --tls-cert /path/to/cert --tls-key /path/to/key
```

### 2. User Management

```bash
# Create admin user during setup
./target/release/primusdb user create admin \
  --role admin \
  --email admin@company.com

# Create read-only application user
./target/release/primusdb user create app_service \
  --role readwrite \
  --email app@company.com
```

### 3. API Token Rotation

Rotate API tokens regularly:

```bash
# Create new token
./target/release/primusdb auth token --create

# Update applications to use new token

# Revoke old token after transition
./target/release/primusdb auth token --revoke OLD_TOKEN_ID
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
./target/release/primusdb server start --bind 127.0.0.1:8080

# Bind to specific interface (production)
./target/release/primusdb server start --bind 10.0.0.5:8080

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

## Security Configuration Reference

```toml
[security]
encryption_enabled = true
key_rotation_interval = 43200    # Rotate encryption keys every 12 hours
auth_required = true             # Require authentication for all requests

[security.auth]
enabled = true
min_password_length = 8
password_expiry_days = 90
max_login_attempts = 5
lockout_duration_minutes = 30
token_expiry_hours = 8760        # 1 year
session_timeout_minutes = 60
rate_limit_requests_per_minute = 1000
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

## Alpha Limitations

As of v1.3.2-alpha:

- **Authentication** accepts credentials but is not enforced by default in all paths.
- **User and role CLI commands** print placeholders — users and roles are not persisted across restarts.
- **API tokens** are stored in memory only and do not persist across restarts.
- **RBAC enforcement** in the HTTP API is not fully wired — some operations may succeed without token validation.
- **Encryption at rest** is implemented for document collections but may not be active for all storage engines.
- **Rate limiting** headers are declared but not enforced in this release.
- **Multi-tenancy segments** API is present but segment isolation is not enforced.

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

- [API Reference](../reference/api.md) - Complete API documentation
- [ADMIN.md](../user-guide/admin.md) - Administration guide
- [ARCHITECTURE.md](../architecture/overview.md) - Security architecture
