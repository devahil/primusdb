# PrimusDB Deployment Guide

This guide covers deploying PrimusDB in various environments, from single-node installations to large-scale distributed clusters.

## Quick Start Deployment

### Docker Single-Node
```bash
# Pull and run
docker run -d \
  --name primusdb \
  -p 8080:8080 \
  -p 9090:9090 \
  -v primusdb_data:/var/lib/primusdb \
  primusdb:latest

# Verify deployment
curl http://localhost:8080/health
```

### Local Binary Installation
```bash
# Download and install
wget https://github.com/devahil/primusdb/releases/latest/download/primusdb-linux-x64.tar.gz
tar -xzf primusdb-linux-x64.tar.gz
sudo mv primusdb-server primusdb-cli /usr/local/bin/

# Initialize
sudo mkdir -p /var/lib/primusdb /var/log/primusdb /etc/primusdb
sudo useradd -r -s /bin/false primusdb
sudo chown -R primusdb:primusdb /var/lib/primusdb /var/log/primusdb

# Create basic config
cat > /etc/primusdb/primusdb.toml << EOF
[storage]
data_dir = "/var/lib/primusdb/data"
cache_size = 536870912

[network]
bind_address = "0.0.0.0"
port = 8080

[logging]
level = "info"
file = "/var/log/primusdb/primusdb.log"
EOF

# Start service
primusdb-server --config /etc/primusdb/primusdb.toml
```

## Production Deployment

### System Requirements

#### Minimum Requirements
- **CPU**: 2 cores, 2.4 GHz
- **RAM**: 4 GB
- **Storage**: 50 GB SSD
- **Network**: 1 Gbps
- **OS**: Ubuntu 20.04+, RHEL 8+, or compatible

#### Recommended Requirements
- **CPU**: 8+ cores, 3.0+ GHz
- **RAM**: 16-64 GB
- **Storage**: 500 GB+ NVMe SSD
- **Network**: 10 Gbps
- **OS**: Ubuntu 22.04 LTS or RHEL 9

### System Configuration

#### Kernel Tuning
```bash
# Increase file descriptors
echo "primusdb soft nofile 65536" >> /etc/security/limits.conf
echo "primusdb hard nofile 65536" >> /etc/security/limits.conf

# Optimize network
echo "net.core.somaxconn = 65536" >> /etc/sysctl.conf
echo "net.ipv4.tcp_max_syn_backlog = 65536" >> /etc/sysctl.conf
echo "net.ipv4.ip_local_port_range = 1024 65535" >> /etc/sysctl.conf

# Optimize I/O
echo "vm.dirty_ratio = 10" >> /etc/sysctl.conf
echo "vm.dirty_background_ratio = 5" >> /etc/sysctl.conf

sysctl -p
```

#### Storage Optimization
```bash
# Use XFS for data directory
mkfs.xfs /dev/nvme0n1
mount -t xfs -o noatime,nodiratime /dev/nvme0n1 /var/lib/primusdb

# Add to fstab
echo "/dev/nvme0n1 /var/lib/primusdb xfs noatime,nodiratime 0 0" >> /etc/fstab

# Set proper ownership
chown -R primusdb:primusdb /var/lib/primusdb
chmod 755 /var/lib/primusdb
```

### Service Configuration

#### systemd Service
```ini
[Unit]
Description=PrimusDB Hybrid Database Server
After=network.target local-fs.target
Requires=local-fs.target

[Service]
Type=exec
User=primusdb
Group=primusdb
ExecStart=/usr/local/bin/primusdb-server --config /etc/primusdb/primusdb.toml
ExecReload=/bin/kill -HUP $MAINPID

# Security settings
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/primusdb /var/log/primusdb /tmp
RestrictSUIDSGID=yes

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096
MemoryLimit=8G

# Restart settings
Restart=always
RestartSec=5
StartLimitIntervalSec=300
StartLimitBurst=5

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=primusdb

[Install]
WantedBy=multi-user.target
```

#### Log Rotation
```bash
# Create logrotate configuration
cat > /etc/logrotate.d/primusdb << EOF
/var/log/primusdb/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 primusdb primusdb
    postrotate
        systemctl reload primusdb.service || true
    endscript
}
EOF
```

## Docker Production Deployment

### Multi-Stage Dockerfile
```dockerfile
# Build stage
FROM rust:1.70-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    liblz4-dev \
    zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/primusdb
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --features production
RUN rm -rf src target/release/deps

COPY src ./src
RUN cargo build --release --features production

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    liblz4-1 \
    zlib1g \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r primusdb && useradd -r -g primusdb primusdb

COPY --from=builder /usr/src/primusdb/target/release/primusdb-server /usr/local/bin/
COPY --from=builder /usr/src/primusdb/target/release/primusdb-cli /usr/local/bin/

RUN mkdir -p /var/lib/primusdb /var/log/primusdb \
    && chown -R primusdb:primusdb /var/lib/primusdb /var/log/primusdb

USER primusdb
EXPOSE 8080 9090

HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD primusdb-cli status || exit 1

CMD ["primusdb-server", "--host", "0.0.0.0", "--port", "8080"]
```

### Docker Compose Production
```yaml
version: '3.8'

services:
  primusdb:
    image: primusdb:latest
    container_name: primusdb
    restart: unless-stopped
    ports:
      - "8080:8080"
      - "9090:9090"
    volumes:
      - primusdb_data:/var/lib/primusdb
      - primusdb_logs:/var/log/primusdb
      - ./config/primusdb.toml:/etc/primusdb/primusdb.toml:ro
    environment:
      - RUST_LOG=info
      - RUST_BACKTRACE=1
    networks:
      - primusdb_network
    deploy:
      resources:
        limits:
          memory: 8G
          cpus: '4.0'
        reservations:
          memory: 4G
          cpus: '2.0'
    healthcheck:
      test: ["CMD", "primusdb-cli", "status"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    networks:
      - primusdb_network

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana_data:/var/lib/grafana
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    networks:
      - primusdb_network

networks:
  primusdb_network:
    driver: bridge

volumes:
  primusdb_data:
    driver: local
  primusdb_logs:
    driver: local
  prometheus_data:
    driver: local
  grafana_data:
    driver: local
```

## Kubernetes Deployment

### Namespace and Resources
```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: primusdb
  labels:
    name: primusdb

---
apiVersion: v1
kind: ConfigMap
metadata:
  name: primusdb-config
  namespace: primusdb
data:
  primusdb.toml: |
    [storage]
    data_dir = "/data"
    cache_size = 2147483648

    [network]
    bind_address = "0.0.0.0"
    port = 8080

    [cluster]
    enabled = true
    node_id = "k8s-node"
    discovery_servers = []

---
apiVersion: v1
kind: Service
metadata:
  name: primusdb
  namespace: primusdb
spec:
  selector:
    app: primusdb
  ports:
  - name: http
    port: 8080
    targetPort: 8080
  - name: metrics
    port: 9090
    targetPort: 9090
  type: ClusterIP
```

### StatefulSet Deployment
```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: primusdb
  namespace: primusdb
spec:
  serviceName: primusdb
  replicas: 3
  selector:
    matchLabels:
      app: primusdb
  template:
    metadata:
      labels:
        app: primusdb
    spec:
      containers:
      - name: primusdb
        image: primusdb:latest
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: metrics
        env:
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
        volumeMounts:
        - name: data
          mountPath: /var/lib/primusdb
        - name: config
          mountPath: /etc/primusdb
        resources:
          requests:
            memory: "4Gi"
            cpu: "2000m"
          limits:
            memory: "8Gi"
            cpu: "4000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: primusdb-config
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 100Gi
      storageClassName: fast-ssd
```

### Ingress Configuration
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: primusdb-ingress
  namespace: primusdb
  annotations:
    kubernetes.io/ingress.class: "nginx"
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/rate-limit: "100"
    nginx.ingress.kubernetes.io/rate-limit-window: "1m"
spec:
  tls:
  - hosts:
    - primusdb.example.com
    secretName: primusdb-tls
  rules:
  - host: primusdb.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: primusdb
            port:
              number: 8080
```

## Cluster Deployment

### Multi-Node Setup
```bash
# Node 1 (Coordinator)
primusdb-server \
  --host 0.0.0.0 \
  --port 8080 \
  --cluster \
  --node-id coordinator \
  --data-dir /var/lib/primusdb

# Node 2 (Worker)
primusdb-server \
  --host 0.0.0.0 \
  --port 8080 \
  --cluster \
  --node-id worker1 \
  --discovery coordinator:8080 \
  --data-dir /var/lib/primusdb

# Node 3 (Worker)
primusdb-server \
  --host 0.0.0.0 \
  --port 8080 \
  --cluster \
  --node-id worker2 \
  --discovery coordinator:8080 \
  --data-dir /var/lib/primusdb
```

### Load Balancer Configuration
```nginx
upstream primusdb_cluster {
    server node1:8080 weight=1;
    server node2:8080 weight=1;
    server node3:8080 weight=1;
}

server {
    listen 80;
    server_name primusdb.example.com;

    location / {
        proxy_pass http://primusdb_cluster;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Rate limiting
        limit_req zone=api burst=100 nodelay;

        # Health checks
        health_check interval=10 fails=3 passes=2 uri=/health;
    }

    location /metrics {
        proxy_pass http://primusdb_cluster;
    }
}
```

## Monitoring and Observability

### Prometheus Configuration
```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'primusdb'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/metrics'
    scrape_interval: 5s

  - job_name: 'primusdb-cluster'
    static_configs:
      - targets:
        - 'node1:9090'
        - 'node2:9090'
        - 'node3:9090'
    metrics_path: '/metrics'
    scrape_interval: 10s
```

### Grafana Dashboards
```json
{
  "dashboard": {
    "title": "PrimusDB Overview",
    "panels": [
      {
        "title": "Query Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(primusdb_http_requests_total[5m])",
            "legendFormat": "{{method}} {{status}}"
          }
        ]
      },
      {
        "title": "Memory Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "primusdb_memory_usage_bytes / 1024 / 1024",
            "legendFormat": "Memory Usage (MB)"
          }
        ]
      }
    ]
  }
}
```

## Backup and Recovery

### Automated Backup Strategy
```bash
#!/bin/bash
# Daily backup script

BACKUP_DIR="/backup/daily"
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="primusdb_$DATE"

# Create backup
primusdb-cli backup --destination "$BACKUP_DIR/$BACKUP_NAME"

# Compress
tar -czf "$BACKUP_DIR/${BACKUP_NAME}.tar.gz" -C "$BACKUP_DIR" "$BACKUP_NAME"

# Upload to cloud storage
aws s3 cp "$BACKUP_DIR/${BACKUP_NAME}.tar.gz" "s3://primusdb-backups/"

# Cleanup old backups (keep 30 days)
find "$BACKUP_DIR" -name "*.tar.gz" -mtime +30 -delete

# Verify backup integrity
primusdb-cli restore --source "$BACKUP_DIR/$BACKUP_NAME" --dry-run
```

### Disaster Recovery
```bash
#!/bin/bash
# Disaster recovery script

BACKUP_URL="s3://primusdb-backups/primusdb_latest.tar.gz"
RESTORE_DIR="/tmp/primusdb_restore"

# Download latest backup
aws s3 cp "$BACKUP_URL" "$RESTORE_DIR/backup.tar.gz"

# Extract backup
mkdir -p "$RESTORE_DIR/backup"
tar -xzf "$RESTORE_DIR/backup.tar.gz" -C "$RESTORE_DIR/backup"

# Stop current service
systemctl stop primusdb

# Restore data
primusdb-cli restore --source "$RESTORE_DIR/backup"

# Start service
systemctl start primusdb

# Verify recovery
curl http://localhost:8080/health
primusdb-cli status
```

## Security Hardening

### Network Security
```bash
# Configure firewall
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw allow 8080/tcp
sudo ufw allow 9090/tcp
sudo ufw --force enable

# SSL/TLS configuration
cat > /etc/primusdb/ssl.conf << EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = State
L = City
O = Organization
OU = Unit
CN = primusdb.example.com

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = primusdb.example.com
DNS.2 = *.primusdb.example.com
EOF
```

### Access Control
```toml
# Security configuration
[security]
encryption_enabled = true
key_rotation_interval = 86400
auth_required = true

[security.tls]
certificate_path = "/etc/ssl/primusdb/cert.pem"
key_path = "/etc/ssl/primusdb/key.pem"
min_tls_version = "1.2"

[security.auth]
provider = "jwt"
secret_key = "your-secret-key-here"
token_expiry_hours = 24
rate_limit_requests_per_minute = 1000
```

## Performance Optimization

### Database Tuning
```toml
[storage]
data_dir = "/var/lib/primusdb"
max_file_size = 1073741824
cache_size = 4294967296
compression = "lz4"
write_buffer_size = 134217728

[storage.performance]
max_background_jobs = 8
max_write_buffer_number = 4
min_write_buffer_number_to_merge = 2
compression_level = 6

[network]
bind_address = "0.0.0.0"
port = 8080
max_connections = 10000
connection_timeout_seconds = 30

[limits]
max_memory_mb = 8192
max_concurrent_queries = 100
query_timeout_seconds = 300
```

### System Optimization
```bash
# CPU optimization
echo "performance" | tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Memory optimization
echo "always" > /sys/kernel/mm/transparent_hugepage/enabled
echo "never" > /sys/kernel/mm/transparent_hugepage/defrag

# I/O optimization
echo "deadline" > /sys/block/nvme0n1/queue/scheduler
echo 0 > /sys/block/nvme0n1/queue/add_random
echo 0 > /sys/block/nvme0n1/queue/rotational
```

## Scaling Strategies

### Horizontal Scaling
- **Read Replicas**: Add read-only nodes for query distribution
- **Shard Splitting**: Automatically split hot shards
- **Load Balancing**: Distribute requests across cluster nodes
- **Auto-scaling**: Add/remove nodes based on load metrics

### Vertical Scaling
- **Resource Allocation**: Increase CPU/memory per node
- **Storage Expansion**: Add more disks or use distributed storage
- **Network Upgrades**: Higher bandwidth networking
- **Caching Layers**: Redis/Memcached for hot data

### Multi-Region Deployment
```yaml
# Global load balancer configuration
global:
  regions:
    - name: us-east-1
      nodes: ["node1", "node2", "node3"]
    - name: eu-west-1
      nodes: ["node4", "node5", "node6"]
    - name: ap-southeast-1
      nodes: ["node7", "node8", "node9"]

  routing:
    latency_based: true
    geo_dns: true
    health_checks: true
```

This deployment guide provides comprehensive instructions for deploying PrimusDB in any environment, from development to enterprise production clusters.