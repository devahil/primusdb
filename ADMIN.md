# PrimusDB Administration Manual
===============================

This manual covers system administration tasks for PrimusDB v1.1.0+ deployments.

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
sudo mv primusdb-server primusdb-cli /usr/local/bin/
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
kill -TERM $(pidof primusdb-server)
# Force restart
kill -KILL $(pidof primusdb-server)
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
ExecStart=/usr/local/bin/primusdb-server --config /etc/primusdb/config.toml
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
primusdb-cli backup --destination /backup/primusdb_$(date +%Y%m%d_%H%M%S)

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
primusdb-cli restore --source /backup/primusdb_20231201_120000

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
```toml
[network.tls]
enabled = true
certificate_path = "/etc/ssl/primusdb.crt"
key_path = "/etc/ssl/private/primusdb.key"
min_tls_version = "1.2"
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
primusdb-server --cluster --node-id node2 --discovery coordinator:8080

# Check cluster status
curl http://localhost:8080/api/v1/cluster/status
```

### Node Maintenance
```bash
# Graceful shutdown
kill -TERM $(pidof primusdb-server)

# Force removal (if needed)
curl -X DELETE http://coordinator:8080/api/v1/cluster/nodes/node2
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
primusdb-cli backup --destination "$BACKUP_DIR/$DATE"

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