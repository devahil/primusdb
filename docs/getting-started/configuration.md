# Configuration

PrimusDB is configured through TOML files, environment variables, and command-line flags.

## Default Config Path

When no `--config` flag is given, PrimusDB searches for `primusdb.toml` in the following order and uses the **first** file found:

1. `./primusdb.toml` — current working directory
2. `./config.toml` — fallback in CWD
3. `./config/primusdb.toml` — config subdirectory
4. `~/.config/primusdb/config.toml` — user-level config
5. `/etc/primusdb/config.toml` — system-level config

If none of these files exist, PrimusDB uses its built-in defaults.

## Config Commands

### `primusdb config init`

Generate a default configuration file at a specified path (or print to stdout).

```bash
# Write to primusdb.toml in the current directory
primusdb config init

# Write to a custom path
primusdb config init --output /etc/primusdb/primusdb.toml

# Overwrite an existing file
primusdb config init --force
```

### `primusdb config validate`

Check a configuration file for syntax errors and invalid values.

```bash
# Validate the default config path
primusdb config validate

# Validate a specific file
primusdb config validate --config /etc/primusdb/prod.toml

# Exit code is 0 on success, non-zero on failure
echo $?
```

### `primusdb config show`

Display the effective configuration (all sources merged).

```bash
# Show merged config (flags + env + file + defaults)
primusdb config show

# Show config from a specific file
primusdb config show --config /etc/primusdb/prod.toml
```

## Environment Variables

| Variable | Overrides | Example |
|----------|-----------|---------|
| `RUST_LOG` | Log level for all tracing output | `RUST_LOG=debug` |
| `PRIMUSDB_CONFIG` | Config file path (`--config` flag) | `PRIMUSDB_CONFIG=/etc/primusdb/prod.toml` |
| `PRIMUSDB_DATA_DIR` | `storage.data_dir` | `PRIMUSDB_DATA_DIR=/var/lib/primusdb` |
| `PRIMUSDB_URL` | Default server URL for CLI | `PRIMUSDB_URL=http://localhost:8080` |
| `PRIMUSDB_FORMAT` | CLI output format | `PRIMUSDB_FORMAT=json` |
| `PRIMUSDB_TIMEOUT` | Request timeout in ms | `PRIMUSDB_TIMEOUT=60000` |

## Config File Reference

Below is a complete reference for every configuration section. All values shown are the built-in defaults.

### `[storage]`

Controls the storage engine layer.

```toml
[storage]
data_dir = "./data"
max_file_size = 1073741824
compression = "lz4"
cache_size = 536870912
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `data_dir` | string | `"./data"` | Directory for database data files |
| `max_file_size` | integer | `1073741824` (1 GiB) | Maximum size per data file in bytes |
| `compression` | string | `"lz4"` | Compression algorithm: `"none"`, `"lz4"`, or `"zstd"` |
| `cache_size` | integer | `536870912` (512 MiB) | In-memory cache size in bytes |

### `[network]`

Controls the HTTP server binding.

```toml
[network]
bind_address = "127.0.0.1"
port = 8080
max_connections = 1000
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `bind_address` | string | `"127.0.0.1"` | IP address to bind the server to |
| `port` | integer | `8080` | TCP port to listen on |
| `max_connections` | integer | `1000` | Maximum concurrent connections |

### `[security]`

Controls encryption and authentication.

```toml
[security]
encryption_enabled = true
key_rotation_interval = 86400
auth_required = false
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `encryption_enabled` | boolean | `true` | Enable AES-256-GCM encryption at rest |
| `key_rotation_interval` | integer | `86400` (24 h) | Encryption key rotation interval in seconds |
| `auth_required` | boolean | `false` | Require authentication for API requests |

### `[cluster]`

Controls cluster mode for distributed deployments.

```toml
[cluster]
enabled = false
node_id = "node1"
discovery_servers = []
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | boolean | `false` | Enable clustering mode |
| `node_id` | string | `"node1"` | Unique identifier for this node |
| `discovery_servers` | array of strings | `[]` | Initial peer addresses for cluster discovery (e.g., `["10.0.0.1:8080"]`) |

### `[logging]`

Controls log output behaviour.

```toml
[logging]
level = "info"
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `level` | string | `"info"` | Log level: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"` |

The `RUST_LOG` environment variable overrides this setting at runtime.

## Config File Priority

Configuration values are resolved with the following precedence (highest wins):

1. **Command-line flags** (e.g., `--bind`, `--data-dir`, `--config`)
2. **Environment variables** (e.g., `PRIMUSDB_DATA_DIR`)
3. **Config file values** (from `primusdb.toml`)
4. **Built-in defaults** (as documented above)

This means a command-line flag always overrides an environment variable, which always overrides the config file, which always overrides the default.

## Examples

### Minimal development config

```toml
[storage]
data_dir = "./dev-data"
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

### Production config

```toml
[storage]
data_dir = "/var/lib/primusdb/data"
max_file_size = 4294967296
compression = "zstd"
cache_size = 2147483648

[network]
bind_address = "0.0.0.0"
port = 8080
max_connections = 5000

[security]
encryption_enabled = true
key_rotation_interval = 43200
auth_required = true

[cluster]
enabled = true
node_id = "prod-node-1"
discovery_servers = ["10.0.0.1:8080", "10.0.0.2:8080"]

[logging]
level = "warn"
```

More examples can be found in `config/examples/` and `examples/config/` in the repository.
