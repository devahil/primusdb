# Security

PrimusDB provides authentication, authorization, and encryption features. This guide covers configuration and usage.

## Authentication: `primusdb auth login`

Authenticate with a username and password to obtain a session.

```bash
# Login with username (password prompted)
primusdb auth login admin

# Login with inline password
primusdb auth login admin --password admin123

# Login with custom realm
primusdb auth login admin --realm internal

# Login with custom session TTL
primusdb auth login admin --ttl 3600
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `-p, --password <PASSWORD>` | Password (prompted if omitted) | — |
| `-r, --realm <REALM>` | Authentication realm | `default` |
| `--ttl <SECONDS>` | Session time-to-live | `86400` |

### Logout

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

## API Tokens: `primusdb auth token`

API tokens provide programmatic access without interactive login.

```bash
# List existing tokens
primusdb auth token --list

# Create a new token
primusdb auth token --create

# Revoke a token
primusdb auth token --revoke tok_abc123
```

### Using API Tokens

Include the token in HTTP requests via the `Authorization` header:

```bash
curl http://localhost:8080/api/v1/query \
  -H "Authorization: Bearer YOUR_API_TOKEN"
```

### Token via REST API

```bash
# Step 1: Login
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin123"}'

# Step 2: Create API token
curl -X POST http://localhost:8080/api/v1/auth/token/create \
  -H "Content-Type: application/json" \
  -d '{
    "authorization": "session_token",
    "name": "my-app-token",
    "scopes": [{"resource": "All", "actions": ["Read", "Write"]}],
    "expires_in_hours": 8760
  }'

# Step 3: Use the token
curl http://localhost:8080/api/v1/crud/document/users \
  -H "Authorization: Bearer YOUR_NEW_TOKEN"
```

### Token Scopes

Tokens can be scoped to specific resource types and actions:

```json
{
  "scopes": [
    {"resource": "Document", "actions": ["Read", "Write"]},
    {"resource": "Columnar", "actions": ["Read"]}
  ]
}
```

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

## RBAC: `primusdb role` and `primusdb user`

Role-Based Access Control (RBAC) allows granular permission management.

### Roles

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

### Built-in Roles

| Role | Description | Permissions |
|------|-------------|-------------|
| `admin` | Full system access | All operations on all resources |
| `developer` | Full data access | Read, Write, Create, Delete on all data |
| `analyst` | Read-only access | Read on all resources |
| `readonly` | Minimal read | Read on all resources |
| `cluster_node` | Node authentication | Cluster operations |

### Users

```bash
# Create a user with a role
primusdb user create alice --role analyst --email alice@example.com

# Create a user with a password
primusdb user create bob --password securepass123 --role developer

# List users
primusdb user list

# Filter users by role
primusdb user list --role admin

# Show disabled users
primusdb user list --all

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
    "password": "securepassword123",
    "email": "charlie@example.com",
    "roles": ["readonly"]
  }'

# List all users (admin only)
curl -X GET http://localhost:8080/api/v1/auth/users \
  -H "Authorization: Bearer ADMIN_TOKEN"

# List all roles
curl -X GET http://localhost:8080/api/v1/auth/roles
```

## Encryption at Rest

PrimusDB encrypts data at rest using AES-256-GCM with per-file encryption keys.

### Configuration

```toml
[security]
encryption_enabled = true
key_rotation_interval = 86400
auth_required = true
```

### Collection-Level Encryption

For document collections, encryption can be toggled per collection:

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

## TLS

TLS configuration is accepted in the configuration file but is **not fully implemented** in v1.3.1-alpha.

### Configuration File

```toml
[network.tls]
enabled = true
certificate_path = "/etc/ssl/certs/primusdb.crt"
key_path = "/etc/ssl/private/primusdb.key"
min_tls_version = "1.2"
```

### Self-Signed Certificate (Development)

```bash
# Generate a self-signed certificate
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout /etc/primusdb/key.pem \
  -out /etc/primusdb/cert.pem \
  -subj "/CN=localhost"

# Reference in config
[network.tls]
enabled = true
certificate_path = "/etc/primusdb/cert.pem"
key_path = "/etc/primusdb/key.pem"
```

### Alpha Limitation

While the `[network.tls]` configuration section is parsed and validated, the TLS listener is **not active** in v1.3.1-alpha. The server always binds using plain HTTP. Use a reverse proxy (nginx, Caddy) for TLS termination in production.

### Reverse Proxy TLS Example (nginx)

```nginx
server {
    listen 443 ssl;
    server_name primusdb.example.com;

    ssl_certificate /etc/ssl/certs/primusdb.crt;
    ssl_certificate_key /etc/ssl/private/primusdb.key;
    ssl_protocols TLSv1.2 TLSv1.3;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

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

## Alpha Limitations

As of v1.3.1-alpha:

- **TLS listener** is not active — the server always binds over plain HTTP
- **`primusdb auth login`** accepts credentials but authentication is not enforced by default
- **`primusdb user`** and **`primusdb role`** commands print placeholders — users and roles are not persisted
- **`primusdb auth token`** creates and lists tokens in memory only; tokens are not persisted across restarts
- **RBAC enforcement** in the HTTP API is not fully wired — all operations may succeed without token validation
- **Encryption at rest** is implemented for document collections but may not be active for all storage engines
- **Rate limiting** headers are declared but not enforced in this release
- **Multi-tenancy segments** API is present but segment isolation is not enforced
