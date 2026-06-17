# Configuration

PrimusDB can be configured via configuration files, command-line flags, and environment variables.

## Configuration File Locations

PrimusDB looks for configuration files in the following order (first found wins):

1. Path specified by `--config` / `-c` flag
2. `./config.toml` in the current working directory
3. `~/.config/primusdb/config.toml`
4. `/etc/primusdb/config.toml`

## Configuration Format

Configuration files use [TOML](https://toml.io/en/) format. Below is the full configuration reference:

```toml
[storage]
data_dir = "./data"
max_file_size = 1073741824
compression = "lz4"
cache_size = 536870912

[network]
bind_address = "127.0.0.1"
port = 8080
max_connections = 1000

[network.tls]
enabled = false
certificate_path = "/etc/ssl/certs/primusdb.crt"
key_path = "/etc/ssl/private/primusdb.key"
min_tls_version = "1.2"

[network.pool]
max_connections = 1000
connection_timeout_seconds = 30
idle_timeout_seconds = 300
max_lifetime_seconds = 3600

[security]
encryption_enabled = true
key_rotation_interval = 86400
auth_required = false

[security.auth]
enabled = true
token_secret = "your-secret-key"
token_expiry_hours = 24
rate_limit_requests_per_minute = 1000

[cluster]
enabled = false
node_id = "node1"
discovery_servers = []

[namespaces]
enabled = true
default_quota_storage = "1GB"
max_depth = 5

[storage.performance]
write_buffer_size = 67108864
max_background_jobs = 4
compaction_style = "level"
compression_level = 6
cache_index_and_filter_blocks = true
```

### Section Reference

#### `[storage]`

Settings for the storage layer:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `data_dir` | string | `"./data"` | Directory for database files |
| `max_file_size` | integer | `1073741824` | Maximum file size in bytes (1 GB) |
| `compression` | string | `"lz4"` | Compression algorithm (`lz4`, `zstd`, `none`) |
| `cache_size` | integer | `536870912` | Cache size in bytes (512 MB) |

#### `[network]`

Network and server settings:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `bind_address` | string | `"127.0.0.1"` | Address to bind the server to |
| `port` | integer | `8080` | Port to listen on |
| `max_connections` | integer | `1000` | Maximum concurrent connections |

#### `[network.tls]`

TLS configuration:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | boolean | `false` | Enable TLS encryption |
| `certificate_path` | string | — | Path to TLS certificate file |
| `key_path` | string | — | Path to TLS private key file |
| `min_tls_version` | string | `"1.2"` | Minimum TLS version |

#### `[network.pool]`

Connection pool settings:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `max_connections` | integer | `1000` | Maximum pooled connections |
| `connection_timeout_seconds` | integer | `30` | Connection timeout |
| `idle_timeout_seconds` | integer | `300` | Idle connection timeout |
| `max_lifetime_seconds` | integer | `3600` | Maximum connection lifetime |

#### `[security]`

Security settings:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `encryption_enabled` | boolean | `true` | Enable data encryption at rest |
| `key_rotation_interval` | integer | `86400` | Encryption key rotation interval (seconds) |
| `auth_required` | boolean | `false` | Require authentication |

#### `[security.auth]`

Authentication settings:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | boolean | `true` | Enable authentication |
| `token_secret` | string | — | Secret key for token signing |
| `token_expiry_hours` | integer | `24` | Token expiration time |
| `rate_limit_requests_per_minute` | integer | `1000` | Rate limit per user |

#### `[cluster]`

Cluster configuration:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | boolean | `false` | Enable cluster mode |
| `node_id` | string | `"node1"` | Unique node identifier |
| `discovery_servers` | array | `[]` | Initial discovery server addresses |

#### `[namespaces]`

Namespace configuration:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | boolean | `true` | Enable namespaces |
| `default_quota_storage` | string | `"1GB"` | Default storage quota per namespace |
| `max_depth` | integer | `5` | Maximum namespace hierarchy depth |

#### `[storage.performance]`

Storage performance tuning:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `write_buffer_size` | integer | `67108864` | Write buffer size in bytes (64 MB) |
| `max_background_jobs` | integer | `4` | Maximum background compaction jobs |
| `compaction_style` | string | `"level"` | Compaction style (`level`, `universal`, `fifo`) |
| `compression_level` | integer | `6` | Compression level (1-22) |
| `cache_index_and_filter_blocks` | boolean | `true` | Cache index and filter blocks |

## Environment Variables

| Variable | Description | Overrides |
|----------|-------------|-----------|
| `PRIMUSDB_URL` | Default server URL | CLI `--server-url` default |
| `PRIMUSDB_FORMAT` | Default output format | CLI `--format` default |
| `PRIMUSDB_TIMEOUT` | Default request timeout (ms) | CLI `--timeout` default |
| `PRIMUSDB_CONFIG` | Path to config file | `--config` flag |
| `PRIMUSDB_DATA_DIR` | Data directory | Config `storage.data_dir` |
| `PRIMUSDB_LOG_LEVEL` | Logging level | `--log-level` flag |
| `RUST_LOG` | Rust log level (tracing) | All log output |

## Command-Line Flag Precedence

Configuration values are resolved in the following order (highest priority first):

1. Command-line flags
2. Environment variables
3. Configuration file values
4. Built-in defaults

### Server Start Flags

```bash
primusdb server start \
  --bind 0.0.0.0:8080 \
  --data-dir /var/lib/primusdb \
  --config /etc/primusdb/prod.toml \
  --log-level warn \
  --daemon
```

| Flag | Env Variable | Config Key |
|------|-------------|------------|
| `--bind` | — | `network.bind_address` + `network.port` |
| `--data-dir` | `PRIMUSDB_DATA_DIR` | `storage.data_dir` |
| `--config` | `PRIMUSDB_CONFIG` | — |
| `--log-level` | `PRIMUSDB_LOG_LEVEL` | — |
| `--daemon` | — | — |

## Example Configurations

### Development (Default)

```toml
[storage]
data_dir = "./data"
compression = "lz4"

[network]
bind_address = "127.0.0.1"
port = 8080

[security]
encryption_enabled = false
auth_required = false

[cluster]
enabled = false
```

### Production

```toml
[storage]
data_dir = "/var/lib/primusdb"
max_file_size = 4294967296
compression = "zstd"
cache_size = 2147483648

[network]
bind_address = "0.0.0.0"
port = 8080
max_connections = 5000

[network.tls]
enabled = true
certificate_path = "/etc/ssl/certs/primusdb.crt"
key_path = "/etc/ssl/private/primusdb.key"

[security]
encryption_enabled = true
key_rotation_interval = 43200
auth_required = true

[security.auth]
enabled = true
token_expiry_hours = 8
rate_limit_requests_per_minute = 2000

[cluster]
enabled = true
node_id = "prod-node-1"
discovery_servers = ["10.0.0.1:8080", "10.0.0.2:8080"]

[storage.performance]
write_buffer_size = 134217728
max_background_jobs = 8
compression_level = 10
```

### Cluster with Federation

```toml
[storage]
data_dir = "/var/lib/primusdb"
compression = "zstd"

[network]
bind_address = "0.0.0.0"
port = 8080

[cluster]
enabled = true
node_id = "us-east-1"
discovery_servers = ["10.0.0.1:8080", "10.0.0.2:8080"]

[security]
encryption_enabled = true
auth_required = true
```

Run with federation flags:

```bash
primusdb server start \
  --bind 0.0.0.0:8080 \
  --config cluster.toml \
  --federation-id my-federation \
  --cluster-id us-east \
  --region us-east-1 \
  --federation-discovery peer-a:8080 \
  --federation-discovery peer-b:8080
```
