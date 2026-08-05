# PrimusDB Administration Manual
===============================

This manual covers system administration tasks for PrimusDB v1.3.1+ deployments.

## System Requirements

### Hardware
- **CPU**: 2+ cores recommended
- **RAM**: 4GB minimum, 8GB+ recommended
- **Storage**: SSD recommended, 10GB+ free space
- **Network**: 1Gbps for production clusters

### Software
- **OS**: Linux (Arch Linux recommended), macOS, Windows
- **Rust**: 1.70+ for compilation
- **Docker**: For containerized deployment

## Installation

### Binary Installation
```bash
# Download latest release
wget https://github.com/devahil/primusdb/releases/latest/download/primusdb-linux-x64.tar.gz
tar -xzf primusdb-linux-x64.tar.gz
sudo mv primusdb /usr/local/bin/
```

### Source Installation
```bash
git clone https://github.com/devahil/primusdb.git
cd primusdb
cargo build --release
sudo cp target/release/primusdb-* /usr/local/bin/
```

## Configuration Management

### Configuration Files
PrimusDB uses TOML configuration files. Default search paths:
1. `./config.toml`
2. `./primusdb.toml`
3. `/etc/primusdb/config.toml`

### Environment Variables
- `PRIMUSDB_CONFIG`: Path to config file
- `PRIMUSDB_DATA_DIR`: Data directory override
- `PRIMUSDB_LOG_LEVEL`: Logging verbosity
- `RUST_LOG`: Rust logging configuration

### Dynamic Configuration
Configuration changes require restart. Use signals for graceful shutdown:
```bash
# Graceful restart
primusdb server stop
# Force restart
primusdb server stop --force
```

## Service Management

### systemd Service
```ini
[Unit]
Description=PrimusDB Database Server
After=network.target

[Service]
Type=simple
User=primusdb
Group=primusdb
ExecStart=/usr/local/bin/primusdb server start --config /etc/primusdb/config.toml
Restart=always
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

### Docker Service
```yaml
version: '3.8'
services:
  primusdb:
    image: primusdb:latest
    restart: unless-stopped
    ports:
      - "8080:8080"
      - "9090:9090"
    volumes:
      - primusdb_data:/var/lib/primusdb
      - primusdb_config:/etc/primusdb
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "primusdb-health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

## Storage Management

### Directory Structure
```
/var/lib/primusdb/
├── data/           # Database files
├── index/          # Index files
├── logs/           # Transaction logs
├── backups/        # Backup files
└── cache/          # Cache files
```

### Storage Engines
Each engine stores data separately:
- `columnar/`: Column-oriented data
- `document/`: JSON documents
- `relational/`: Table data
- `vector/`: Vector embeddings

### Disk Space Monitoring
```bash
# Check disk usage
du -sh /var/lib/primusdb/*

# Monitor with df
df -h /var/lib/primusdb
```

## Backup and Recovery

### Manual Backup
```bash
# Stop the server
systemctl stop primusdb

# Create backup
primusdb backup create --destination /backup/primusdb_$(date +%Y%m%d_%H%M%S)

# Start the server
systemctl start primusdb
```

### Automated Backup
```bash
#!/bin/bash
BACKUP_DIR="/backup"
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="primusdb_$DATE"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Use API for hot backup
curl -X POST http://localhost:8080/api/v1/backup \
  -H "Content-Type: application/json" \
  -d "{\"destination\": \"$BACKUP_DIR/$BACKUP_NAME\"}"
```

### Recovery
```bash
# Stop the server
systemctl stop primusdb

# Restore from backup
primusdb backup restore --source /backup/primusdb_20231201_120000

# Start the server
systemctl start primusdb
```

## Security Configuration

### Authentication & Authorization
PrimusDB v1.1.0+ includes comprehensive RBAC with user/password authentication and API tokens.

```toml
[security.auth]
enabled = true
require_auth = true
min_password_length = 8
password_expiry_days = 90
max_login_attempts = 5
lockout_duration_minutes = 30
token_expiry_hours = 8760
session_timeout_minutes = 60
```

### Default Users
After installation, a default admin user is created:
- **Username**: `admin`
- **Password**: `admin123`

**Important**: Change the default password immediately in production!

### User Management
```bash
# Login to get session info
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin123"}'

# Create a new user
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username": "developer", "password": "securepass", "roles": ["developer"]}'

# Create API token
curl -X POST http://localhost:8080/api/v1/auth/token/create \
  -H "Content-Type: application/json" \
  -d '{"authorization": "token", "name": "dev-token", "scopes": [{"resource": "All", "actions": ["Read", "Write"]}]}'
```

### Role-Based Access Control
| Role | Description |
|------|-------------|
| `admin` | Full system access |
| `developer` | Read/Write on all storage engines |
| `analyst` | Read-only on all engines |
| `readonly` | Read on all resources |
| `cluster_node` | Cluster operations |

### Encryption Setup
```toml
[security]
encryption_enabled = true
key_rotation_interval = 86400  # 24 hours

[security.encryption]
algorithm = "aes-256-gcm"
key_size = 32
```

### TLS Configuration

PrimusDB supports native TLS/HTTPS and mutual TLS (mTLS) for inter-node
federation communication. Use the `primusdb certs` command to generate
certificates.

```bash
# Quick start: self-signed cert
primusdb certs create-selfsigned --out-dir ./tls --hosts localhost
primusdb server start --tls-enabled --tls-cert ./tls/selfsigned.pem --tls-key ./tls/selfsigned.key
```

```toml
[network]
tls_enabled = true
tls_cert_path = "/etc/ssl/primusdb.crt"
tls_key_path = "/etc/ssl/private/primusdb.key"
tls_ca_path = "/etc/ssl/ca.crt"       # Required for mTLS
mtls_enabled = false                   # Require client certificates
```

### Access Control
```toml
[security.auth]
enabled = true
token_secret = "your-secret-key"
token_expiry_hours = 24
rate_limit_requests_per_minute = 1000
```

## Performance Tuning

### Memory Configuration
```toml
[storage]
cache_size = 1073741824  # 1GB

[storage.performance]
write_buffer_size = 67108864   # 64MB
max_background_jobs = 4
compression_level = 6
```

### Namespace Configuration
```toml
[namespaces]
enabled = true                           # Enable/disable namespace isolation
default_namespace = "root.default"       # Default namespace for unqualified queries
strict_isolation = true                  # Reject cross-namespace access
allow_cross_namespace_queries = false    # Allow queries that span multiple namespaces
cache_size = 10000                       # Namespace metadata cache size (entries)
max_depth = 16                           # Maximum nesting depth for namespace paths
allow_legacy_without_namespace = true    # Allow operations without namespace field
```

Namespace paths use dot-separated components (e.g., `myorg.production`). Each component must start with a letter or underscore and contain only alphanumeric characters and underscores.

When namespaces are enabled:
- CRUD operations with `namespace` set are isolated to that namespace via `NamespacedStorageEngine`
- DDL/ER operations with `namespace` set use hash-based physical names (`ns_{sha256_6hex}__{name}`)
- Omitting the `namespace` field uses the default namespace (or the global table namespace when `allow_legacy_without_namespace = true`)

### Connection Pooling
```toml
[network.pool]
max_connections = 1000
connection_timeout_seconds = 30
idle_timeout_seconds = 300
max_lifetime_seconds = 3600
```

### Query Optimization
- Use appropriate storage engines for query patterns
- Configure indexes for frequently queried fields
- Monitor slow queries via logs

## Monitoring and Logging

### Log Configuration
```toml
[logging]
level = "info"
file = "/var/log/primusdb/primusdb.log"
max_file_size = 104857600  # 100MB
max_files = 10
format = "json"
```

### Metrics Endpoints
- Health: `GET /health`
- Status: `GET /status`
- Metrics: `GET /metrics`
- Cluster Health: `GET /api/v1/cache/cluster/health`

### Log Rotation
```bash
# Using logrotate
cat > /etc/logrotate.d/primusdb << EOF
/var/log/primusdb/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 primusdb primusdb
    postrotate
        systemctl reload primusdb
    endscript
}
EOF
```

## Cluster Management

### Node Configuration
```toml
[cluster]
enabled = true
node_id = "node1"
discovery_servers = ["coordinator:8080"]

[cluster.consensus]
algorithm = "hyperledger"
min_nodes = 3
heartbeat_interval = 1000
election_timeout_min = 150
election_timeout_max = 300
```

### Adding Nodes
```bash
# Configure new node
primusdb server start --bind 127.0.0.1:8080 \
  --federation-id my-fed --cluster-id my-cluster --region us-east \
  --federation-discovery coordinator:8080

# Check cluster status
curl http://localhost:8080/api/v1/cluster/status
```

### Node Maintenance
```bash
# Graceful shutdown
primusdb server stop

# Force removal (if needed)
curl -X DELETE http://coordinator:8080/api/v1/cluster/nodes/node2
```

### Cluster Gateway
The ClusterGateway provides smart load balancing with 6 routing strategies (RoundRobin, LeastLoaded, LowestLatency, ShardAware, Random, DomainAware) and circuit breaker (5 failures → 30s reset).

```bash
# Check gateway status
curl http://localhost:8080/api/v1/cluster/status

# List registered nodes
curl http://localhost:8080/api/v1/cluster/nodes

# Route a request (with strategy selection)
curl -X POST http://localhost:8080/api/v1/cluster/route \
  -H "Content-Type: application/json" \
  -d '{"strategy": "LeastLoaded"}'

# Register a node via gateway
curl -X POST http://localhost:8080/api/v1/cluster/node/register \
  -H "Content-Type: application/json" \
  -d '{"node_id": "node3", "host": "10.0.0.3", "port": 8080, "shards": []}'

# Remove a node via gateway
curl -X DELETE http://localhost:8080/api/v1/cluster/nodes/node3

# View gateway metrics
curl http://localhost:8080/api/v1/cluster/metrics
```

## Federation Management

### Federation Configuration
```toml
[federation]
enabled = true
federation_id = "my-federation"
cluster_id = "cluster-us"
region = "us-east"
discovery = ["fed-peer1:8080", "fed-peer2:8080"]
```

### Starting a Federated Server
```bash
primusdb server start --bind 0.0.0.0:8080 \
  --federation-id my-fed --cluster-id cluster-us --region us-east \
  --federation-discovery fed-peer1:8081,fed-peer2:8081
```

### Federation Admin Commands
```bash
# Check federation status
curl http://localhost:8080/api/v1/federation/status

# List all member clusters
curl http://localhost:8080/api/v1/federation/clusters

# List DataDomains
curl http://localhost:8080/api/v1/federation/domains

# Create a DataDomain
curl -X POST http://localhost:8080/api/v1/federation/domains \
  -H "Content-Type: application/json" \
  -d '{
    "name": "global-users",
    "description": "Replicated user data",
    "replication_mode": "Quorum",
    "member_clusters": ["cluster-us", "cluster-eu"]
  }'

# Join a DataDomain
curl -X POST http://localhost:8080/api/v1/federation/domains/global-users/join \
  -H "Content-Type: application/json" \
  -d '{"collections": ["users"], "storage_types": ["document"]}'

# Leave a DataDomain
curl -X POST http://localhost:8080/api/v1/federation/domains/global-users/leave \
  -H "Content-Type: application/json" \
  -d '{}'

# Rebalance a DataDomain
curl -X POST http://localhost:8080/api/v1/federation/domains/global-users/balance \
  -H "Content-Type: application/json" \
  -d '{}'

# View federation metrics
curl http://localhost:8080/api/v1/federation/metrics
```

## Troubleshooting

### Common Issues

#### High Memory Usage
```bash
# Check memory usage
ps aux | grep primusdb

# Adjust cache settings
echo "cache_size = 536870912" >> /etc/primusdb/config.toml
systemctl restart primusdb
```

#### Slow Queries
- Check storage engine selection
- Verify indexes are configured
- Monitor system resources

#### Connection Issues
```bash
# Check network configuration
netstat -tlnp | grep 8080

# Test connectivity
curl -v http://localhost:8080/health
```

### Log Analysis
```bash
# Search for errors
grep "ERROR" /var/log/primusdb/primusdb.log

# Monitor performance
tail -f /var/log/primusdb/primusdb.log | grep "slow query"
```

## Upgrade Procedures

### Rolling Upgrade
```bash
# For each node:
systemctl stop primusdb
# Install new version
systemctl start primusdb

# Verify cluster health
curl http://localhost:8080/api/v1/cluster/status
```

### Full Cluster Upgrade
```bash
# Stop all nodes
for node in node1 node2 node3; do
    ssh $node systemctl stop primusdb
done

# Upgrade all nodes
for node in node1 node2 node3; do
    ssh $node "cd /opt/primusdb && git pull && cargo build --release"
    ssh $node systemctl start primusdb
done
```

## Backup Strategy

### Daily Backups
```bash
#!/bin/bash
BACKUP_DIR="/backup/daily"
DATE=$(date +%Y%m%d)

# Create backup
primusdb backup create --destination "$BACKUP_DIR/$DATE"

# Clean old backups (keep 30 days)
find "$BACKUP_DIR" -type d -mtime +30 -exec rm -rf {} \;
```

### Disaster Recovery
1. Prepare recovery environment
2. Restore from latest backup
3. Verify data integrity
4. Reconfigure cluster if needed
5. Test application connectivity

## Security Hardening

### File Permissions
```bash
# Secure data directory
chown -R primusdb:primusdb /var/lib/primusdb
chmod 700 /var/lib/primusdb

# Secure config files
chmod 600 /etc/primusdb/*.toml
```

### Network Security
- Use firewalls to restrict access
- Enable TLS for all connections
- Implement proper authentication
- Regular security updates

### Audit Logging
```toml
[security.audit]
enabled = true
log_file = "/var/log/primusdb/audit.log"
log_operations = ["create", "update", "delete", "admin"]
```

This manual covers the essential administration tasks. For specific use cases, refer to the user manual and API documentation.