use axum::{routing::get, Json, Router};
///
///# PrimusDB Server Binary - REST API Server
///
///This is the main entry point for the PrimusDB REST API server, providing
///HTTP endpoints for all database operations, AI/ML functionality, and
///administrative tasks.
///
///## Server Architecture
///
///```
///PrimusDB Server Architecture
///═══════════════════════════════════════════════════════════════
///
///┌─────────────────────────────────────────────────────────┐
///│                Server Components                        │
///│  ┌─────────────────────────────────────────────────┐    │
///│  │  HTTP Server (Axum)                             │    │
///│  │  • REST API endpoints                           │    │
///│  │  • Request routing and middleware                │    │
///│  │  • CORS and security headers                     │    │
///│  └─────────────────────────────────────────────────┘    │
///│                                                         │
///│  ┌─────────────────────────────────────────────────┐    │
///│  │  PrimusDB Engine                                │    │
///│  │  • Storage engines (4 types)                    │    │
///│  │  • AI/ML processing                             │    │
///│  │  • Transaction management                       │    │
///│  │  └─────────────────────────────────────────────────┘    │
///│                                                         │
///│  ┌─────────────────────────────────────────────────┐    │
///│  │  Background Services                           │    │
///│  │  • Compaction and optimization                  │    │
///│  │  • Metrics collection                           │    │
///│  │  • Health monitoring                            │    │
///│  └─────────────────────────────────────────────────┘    │
///└─────────────────────────────────────────────────────────┘
///```
///
///## Features
///
///- **REST API**: Complete HTTP interface for all database operations
///- **Multi-Engine Support**: Unified API across all storage types
///- **AI/ML Integration**: HTTP endpoints for machine learning operations
///- **Cluster Support**: Distributed coordination and data replication
///- **Security**: TLS encryption, authentication, and authorization
///- **Monitoring**: Health checks, metrics, and observability
///- **Performance**: Optimized request handling and connection pooling
///
///## Usage
///
///### Basic Server Startup
///```bash
///# Start server with default settings
///primusdb-server
///
///# Start with custom host/port
///primusdb-server --host 0.0.0.0 --port 8080
///
///# Enable clustering
///primusdb-server --cluster
///
///# Use custom data directory
///primusdb-server --data-dir /var/lib/primusdb
///```
///
///### Configuration File
///```bash
///# Use custom configuration file
///primusdb-server --config production.toml
///```
///
///### Production Deployment
///```bash
///# Production server with logging
///primusdb-server \
///  --host 0.0.0.0 \
///  --port 8080 \
///  --cluster \
///  --data-dir /data/primusdb \
///  --log-level warn
///```
///
///## API Endpoints
///
///### Core Operations
///- `GET /health` - Service health check
///- `POST /api/v1/query` - Execute database queries
///- `GET /api/v1/tables` - List available tables
///- `POST /api/v1/tables` - Create new tables
///- `DELETE /api/v1/tables/{name}` - Delete tables
///
///### CRUD Operations
///- `POST /api/v1/crud/{storage}/{table}` - Create records
///- `GET /api/v1/crud/{storage}/{table}` - Read records
///- `PUT /api/v1/crud/{storage}/{table}` - Update records
///- `DELETE /api/v1/crud/{storage}/{table}` - Delete records
///
///### Advanced Operations
///- `POST /api/v1/advanced/analyze/{storage}/{table}` - Data analysis
///- `POST /api/v1/advanced/predict/{storage}/{table}` - AI predictions
///- `POST /api/v1/advanced/vector-search/{table}` - Vector similarity search
///- `POST /api/v1/advanced/cluster/{storage}/{table}` - Data clustering
///
///## Configuration
///
///### Command Line Options
///- `--host`: Server bind address (default: 127.0.0.1)
///- `--port`: Server port (default: 8080)
///- `--data-dir`: Data storage directory
///- `--cluster`: Enable cluster mode
///- `--config`: Configuration file path
///- `--log-level`: Logging verbosity
///
///### Configuration File (config.toml)
///```toml
///[storage]
///data_dir = "./data"
///max_file_size = 1073741824
///compression = "lz4"
///cache_size = 104857600
///
///[network]
///bind_address = "0.0.0.0"
///port = 8080
///max_connections = 1000
///
///[security]
///encryption_enabled = true
///key_rotation_interval = 86400
///auth_required = false
///
///[cluster]
///enabled = true
///node_id = "server-1"
///discovery_servers = ["coordinator:8080"]
///```
///
///## Monitoring
///
///### Health Endpoints
///```bash
///# Health check
///curl http://localhost:8080/health
///
///# Detailed status
///curl http://localhost:8080/status
///
///# Metrics (if enabled)
///curl http://localhost:8080/metrics
///```
///
///### Logging
///The server provides structured logging with configurable levels:
///- ERROR: Critical errors only
///- WARN: Warnings and errors
///- INFO: General information (default)
///- DEBUG: Detailed debugging information
///- TRACE: Maximum verbosity
///
///## Security
///
///### TLS Configuration
///```toml
///[network.tls]
///enabled = true
///certificate_path = "/etc/ssl/certs/primusdb.crt"
///key_path = "/etc/ssl/private/primusdb.key"
///min_tls_version = "1.2"
///```
///
///### Authentication
///```toml
///[security.auth]
///enabled = true
///token_secret = "your-secret-key"
///token_expiry_hours = 24
///rate_limit_requests_per_minute = 1000
///```
///
///## Performance Tuning
///
///### Connection Pooling
///```toml
///[network.pool]
///max_connections = 1000
///connection_timeout_seconds = 30
///idle_timeout_seconds = 300
///max_lifetime_seconds = 3600
///```
///
///### Storage Optimization
///```toml
///[storage.performance]
///write_buffer_size = 67108864    # 64MB
///max_background_jobs = 4
///compaction_style = "level"
///compression_level = 6
///cache_index_and_filter_blocks = true
///```
///
///## Troubleshooting
///
///### Common Issues
///
///#### Port Already in Use
///```bash
///# Find process using port 8080
///lsof -i :8080
///
///# Kill the process
///kill -9 <PID>
///
///# Or use different port
///primusdb-server --port 8081
///```
///
///#### Permission Denied
///```bash
///# Fix data directory permissions
///sudo chown -R primusdb:primusdb /var/lib/primusdb
///sudo chmod 755 /var/lib/primusdb
///```
///
///#### High Memory Usage
///```bash
///# Adjust cache settings
///echo "[storage.performance]" >> config.toml
///echo "cache_size = 134217728" >> config.toml  # 128MB
///```
///
///## Development
///
///### Local Development Setup
///```bash
///# Clone and build
///git clone https://github.com/devahil/primusdb.git
///cd primusdb
///cargo build --release
///
///# Run in development mode
///RUST_LOG=debug cargo run --bin primusdb-server -- --log-level debug
///```
///
///### Testing
///```bash
///# Run server tests
///cargo test --bin primusdb-server
///
///# Test API endpoints
///curl -X POST http://localhost:8080/api/v1/query \
///  -H "Content-Type: application/json" \
///  -d '{"storage_type": "document", "operation": "Create", "table": "test"}'
///```
///
///## Production Deployment
///
///### Systemd Service
///```ini
///[Unit]
///Description=PrimusDB Server
///After=network.target
///
///[Service]
///Type=simple
///User=primusdb
///Group=primusdb
///ExecStart=/usr/local/bin/primusdb-server --host 0.0.0.0 --port 8080
///Restart=always
///RestartSec=5
///
///[Install]
///WantedBy=multi-user.target
///```
///
///### Docker Production
///```dockerfile
///FROM rust:1.70-slim AS builder
///WORKDIR /app
///COPY . .
///RUN cargo build --release
///
///FROM debian:bullseye-slim
///RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
///COPY --from=builder /app/target/release/primusdb-server /usr/local/bin/
///EXPOSE 8080
///CMD ["primusdb-server", "--host", "0.0.0.0"]
///```
///
///This binary provides the complete PrimusDB server implementation with
///production-ready features, comprehensive monitoring, and enterprise security.
///
use clap::Parser;
use primusdb::{
    ClusterConfig, CompressionType, NetworkConfig, PrimusDB, PrimusDBConfig, SecurityConfig,
    StorageConfig,
};
use primusdb::auth::{AuthService, AuthConfig};
use std::sync::Arc;
use tower::limit::RateLimitLayer;

#[derive(Parser)]
#[command(name = "primusdb-server")]
#[command(about = "PrimusDB Hybrid Database Server")]
pub struct ServerCli {
    #[arg(short, long, default_value = "127.0.0.1")]
    pub host: String,

    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    #[arg(short, long)]
    pub data_dir: Option<String>,

    #[arg(short, long)]
    pub cluster: bool,

    #[arg(short, long, default_value = "config.toml")]
    pub config: String,

    #[arg(short, long, default_value = "info")]
    pub log_level: String,
}

#[tokio::main]
async fn main() -> primusdb::Result<()> {
    tracing_subscriber::fmt::init();

    let args = ServerCli::parse();

    let config = create_config(&args);
    let primusdb = Arc::new(PrimusDB::new(config)?);

    let auth_config = AuthConfig {
        require_auth: true,
        min_password_length: 8,
        password_expiry_days: 90,
        max_login_attempts: 5,
        lockout_duration_minutes: 30,
        token_expiry_hours: 8760,
        session_timeout_minutes: 60,
        mfa_required_for_roles: vec!["admin".to_string()],
    };
    let auth_service = Arc::new(AuthService::new(auth_config)?);

    let api_server = primusdb::api::APIServer::new(primusdb.clone(), auth_service);
    let bind_addr = format!("{}:{}", args.host, args.port);

    println!("🚀 Starting PrimusDB Server v1.1.0");
    println!("📡 Listening on: {}", bind_addr);
    println!("💾 Data directory: {:?}", args.data_dir);
    println!("🌐 Cluster mode: {}", args.cluster);
    println!("🔐 Authentication: enabled");

    api_server.run(&bind_addr).await?;

    Ok(())
}

fn create_config(args: &ServerCli) -> PrimusDBConfig {
    PrimusDBConfig {
        storage: StorageConfig {
            data_dir: args
                .data_dir
                .clone()
                .unwrap_or_else(|| "/tmp/primusdb_data".to_string()),
            max_file_size: 1024 * 1024 * 1024, // 1GB
            compression: CompressionType::Lz4,
            cache_size: 512 * 1024 * 1024, // 512MB
        },
        network: NetworkConfig {
            bind_address: args.host.clone(),
            port: args.port,
            max_connections: 1000,
        },
        security: SecurityConfig {
            encryption_enabled: true,
            key_rotation_interval: 86400, // 24 hours
            auth_required: false,
        },
        cluster: ClusterConfig {
            enabled: args.cluster,
            node_id: format!(
                "node_{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            discovery_servers: vec![],
        },
    }
}
