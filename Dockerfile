use archlinux::base_image;

# PrimusDB Dockerfile - Arch Linux based
FROM archlinux:latest

# Update system and install build dependencies
RUN pacman -Syu --noconfirm \
    && pacman -S --noconfirm \
        base-devel \
        rust \
        cargo \
        gcc \
        clang \
        llvm \
        cmake \
        make \
        pkg-config \
        openssl \
        sqlite \
        postgresql \
        redis \
        git \
        curl \
        wget \
        nano \
        htop \
        valgrind \
        gdb \
        strace

# Install runtime dependencies
RUN pacman -S --noconfirm \
        sqlite \
        postgresql-libs \
        openssl \
        libffi \
        zlib \
        bzip2 \
        xz \
        lz4 \
        zstd

# Create primusdb user
RUN useradd -m -s /bin/bash primusdb

# Set working directory
WORKDIR /opt/primusdb

# Copy source code
COPY . /opt/primusdb/

# Change ownership to primusdb user
RUN chown -R primusdb:primusdb /opt/primusdb

# Switch to primusdb user
USER primusdb

# Set environment variables
ENV RUST_LOG=info
ENV PRIMUSDB_DATA_DIR=/var/lib/primusdb
ENV PRIMUSDB_LOG_DIR=/var/log/primusdb
ENV PRIMUSDB_CONFIG_DIR=/etc/primusdb

# Create necessary directories
RUN mkdir -p /var/lib/primusdb \
    && mkdir -p /var/log/primusdb \
    && mkdir -p /etc/primusdb \
    && mkdir -p /tmp/primusdb

# Build PrimusDB
RUN cargo build --release

# Install PrimusDB
RUN sudo install -m 755 target/release/primusdb-server /usr/local/bin/ \
    && sudo install -m 755 target/release/primusdb-cli /usr/local/bin/

# Create default configuration
RUN cat > /etc/primusdb/primusdb.toml << 'EOF'
[storage]
data_dir = "/var/lib/primusdb/data"
max_file_size = 1073741824  # 1GB
cache_size = 536870912      # 512MB
compression = "lz4"

[network]
bind_address = "0.0.0.0"
port = 8080
max_connections = 1000

[security]
encryption_enabled = true
key_rotation_interval = 86400  # 24 hours
auth_required = false

[cluster]
enabled = false
node_id = "primusdb-node-1"
discovery_servers = []

[logging]
level = "info"
file = "/var/log/primusdb/primusdb.log"
max_file_size = 104857600  # 100MB
max_files = 10

[metrics]
enabled = true
port = 9090
path = "/metrics"
EOF

# Create systemd service file
RUN sudo mkdir -p /etc/systemd/system \
    && sudo cat > /etc/systemd/system/primusdb.service << 'EOF'
[Unit]
Description=PrimusDB Hybrid Database Engine
After=network.target

[Service]
Type=simple
User=primusdb
Group=primusdb
WorkingDirectory=/opt/primusdb
ExecStart=/usr/local/bin/primusdb-server --data-dir /var/lib/primusdb --config /etc/primusdb/primusdb.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/primusdb /var/log/primusdb

[Install]
WantedBy=multi-user.target
EOF

# Create init scripts
RUN sudo cat > /usr/local/bin/primusdb-init << 'EOF'
#!/bin/bash
set -e

PRIMUSDB_USER="primusdb"
PRIMUSDB_GROUP="primusdb"
PRIMUSDB_DATA_DIR="/var/lib/primusdb"
PRIMUSDB_LOG_DIR="/var/log/primusdb"
PRIMUSDB_CONFIG_DIR="/etc/primusdb"

echo "Initializing PrimusDB..."

# Create directories if they don't exist
sudo mkdir -p "$PRIMUSDB_DATA_DIR"/{data,index,logs,backups}
sudo mkdir -p "$PRIMUSDB_LOG_DIR"
sudo mkdir -p "$PRIMUSDB_CONFIG_DIR"

# Set ownership
sudo chown -R $PRIMUSDB_USER:$PRIMUSDB_GROUP "$PRIMUSDB_DATA_DIR"
sudo chown -R $PRIMUSDB_USER:$PRIMUSDB_GROUP "$PRIMUSDB_LOG_DIR"
sudo chown -R $PRIMUSDB_USER:$PRIMUSDB_GROUP "$PRIMUSDB_CONFIG_DIR"

# Set permissions
sudo chmod 755 "$PRIMUSDB_DATA_DIR"
sudo chmod 755 "$PRIMUSDB_LOG_DIR"
sudo chmod 755 "$PRIMUSDB_CONFIG_DIR"

# Create basic log rotation config
sudo cat > /etc/logrotate.d/primusdb << 'EOL'
/var/log/primusdb/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 primusdb primusdb
    postrotate
        systemctl reload primusdb 2>/dev/null || true
    endscript
}
EOL

echo "PrimusDB initialization complete!"
echo "Start the service with: sudo systemctl start primusdb"
echo "Enable on boot with: sudo systemctl enable primusdb"
EOF

RUN sudo chmod +x /usr/local/bin/primusdb-init

# Create health check script
RUN cat > /usr/local/bin/primusdb-health << 'EOF'
#!/bin/bash
set -e

PORT=${1:-8080}
HOST=${2:-127.0.0.1}

echo "Checking PrimusDB health at $HOST:$PORT..."

# Check if the service is running
if ! pgrep -f "primusdb-server" > /dev/null; then
    echo "ERROR: PrimusDB server is not running"
    exit 1
fi

# Check HTTP endpoint
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "http://$HOST:$PORT/api/v1/health" 2>/dev/null)

if [ "$HTTP_STATUS" = "200" ]; then
    echo "✅ PrimusDB is healthy"
    exit 0
else
    echo "❌ PrimusDB health check failed (HTTP $HTTP_STATUS)"
    exit 1
fi
EOF

RUN chmod +x /usr/local/bin/primusdb-health

# Create backup script
RUN cat > /usr/local/bin/primusdb-backup << 'EOF'
#!/bin/bash
set -e

BACKUP_DIR=${1:-"/var/backups/primusdb"}
DATA_DIR=${2:-"/var/lib/primusdb"}
COMPRESSION=${3:-"lz4"}
ENCRYPTION=${4:-false}

TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_NAME="primusdb_backup_$TIMESTAMP"
FULL_BACKUP_PATH="$BACKUP_DIR/$BACKUP_NAME"

echo "Creating PrimusDB backup..."

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Create backup
if [ "$ENCRYPTION" = "true" ]; then
    tar -cf - -C "$DATA_DIR" . | $COMPRESSION -c | openssl enc -aes-256-cbc -salt -out "$FULL_BACKUP_PATH.tar.$COMPRESSION.enc"
    echo "Encrypted backup created: $FULL_BACKUP_PATH.tar.$COMPRESSION.enc"
else
    tar -cf - -C "$DATA_DIR" . | $COMPRESSION -c > "$FULL_BACKUP_PATH.tar.$COMPRESSION"
    echo "Backup created: $FULL_BACKUP_PATH.tar.$COMPRESSION"
fi

# Create backup metadata
cat > "$BACKUP_DIR/${BACKUP_NAME}_metadata.json" << EOL
{
  "backup_name": "$BACKUP_NAME",
  "timestamp": "$TIMESTAMP",
  "compression": "$COMPRESSION",
  "encryption": $ENCRYPTION,
  "source_data_dir": "$DATA_DIR",
  "backup_path": "$FULL_BACKUP_PATH",
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOL

echo "Backup metadata created: ${BACKUP_DIR}/${BACKUP_NAME}_metadata.json"
echo "Backup completed successfully!"
EOF

RUN chmod +x /usr/local/bin/primusdb-backup

# Expose ports
EXPOSE 8080 9090

# Create entrypoint script
RUN cat > /opt/primusdb/docker-entrypoint.sh << 'EOF'
#!/bin/bash
set -e

# Initialize if this is the first run
if [ ! -f "/var/lib/primusdb/.initialized" ]; then
    echo "First run detected, initializing PrimusDB..."
    primusdb-init
    touch /var/lib/primusdb/.initialized
fi

# Check if we're running as root (shouldn't happen in production)
if [ "$EUID" -eq 0 ]; then
    echo "WARNING: Running as root is not recommended for production"
    exec sudo -u primusdb "$@"
else
    exec "$@"
fi
EOF

RUN chmod +x /opt/primusdb/docker-entrypoint.sh

ENTRYPOINT ["/opt/primusdb/docker-entrypoint.sh"]

# Default command - start the server
CMD ["primusdb-server", "--host", "0.0.0.0", "--port", "8080", "--data-dir", "/var/lib/primusdb", "--cluster"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD primusdb-health 127.0.0.1 8080

# Labels for metadata
LABEL maintainer="PrimusDB Team" \
      version="0.1.0" \
      description="PrimusDB - Hybrid Database Engine" \
      org.opencontainers.image.title="PrimusDB" \
      org.opencontainers.image.description="Hybrid database engine combining columnar, vector, document, and relational storage" \
      org.opencontainers.image.vendor="PrimusDB Team" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.version="0.1.0"

# Documentation
RUN echo "
PrimusDB Docker Container Documentation
========================================

This Docker image provides a complete PrimusDB hybrid database engine installation
based on Arch Linux.

Quick Start:
-----------
docker run -p 8080:8080 -p 9090:9090 -v primusdb_data:/var/lib/primusdb primusdb:latest

Configuration:
-------------
- Config file: /etc/primusdb/primusdb.toml
- Data directory: /var/lib/primusdb
- Logs: /var/log/primusdb
- Metrics endpoint: http://localhost:9090/metrics

Useful Commands:
---------------
- Initialize: primusdb-init
- Health check: primusdb-health
- Create backup: primusdb-backup [/path/to/backup] [/data/dir] [compression] [encryption]

Service Management:
-----------------
- Start: sudo systemctl start primusdb
- Stop: sudo systemctl stop primusdb
- Status: sudo systemctl status primusdb
- Enable on boot: sudo systemctl enable primusdb

For more information, visit: https://github.com/primusdb/primusdb
" > /opt/primusdb/README.md