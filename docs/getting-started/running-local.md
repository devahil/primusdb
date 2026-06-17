# Running PrimusDB Locally

This guide covers starting, stopping, and monitoring a local PrimusDB instance.

## Quick Start

After building from source, start a server with all default settings:

```bash
# Start with defaults (binds to 127.0.0.1:8080)
./target/release/primusdb server start
```

The server is now running and ready to accept connections. Verify with:

```bash
curl http://localhost:8080/health
```

## The `primusdb server` Command

### `primusdb server start`

Starts the PrimusDB server daemon.

```bash
primusdb server start [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config <FILE>` | Path to configuration file | auto-detect |
| `-b, --bind <HOST:PORT>` | Bind address | `127.0.0.1:8080` |
| `-d, --data-dir <PATH>` | Data storage directory | `./data` |
| `--daemon` | Fork to background | `false` |
| `--log-level <LEVEL>` | Log level (trace/debug/info/warn/error) | `info` |

### Custom Host and Port

```bash
# Listen on all interfaces, port 9090
primusdb server start --bind 0.0.0.0:9090

# Listen on a specific interface
primusdb server start --bind 192.168.1.50:8080
```

### Custom Data Directory

```bash
primusdb server start --data-dir /mnt/ssd/primusdb-data
```

### Custom Config File

```bash
primusdb server start --config /etc/primusdb/prod.toml
```

### Daemon Mode

For long-running sessions:

```bash
primusdb server start --daemon
```

### Combined Example

```bash
primusdb server start \
  --bind 0.0.0.0:8080 \
  --data-dir /var/lib/primusdb \
  --config /etc/primusdb/primusdb.toml \
  --log-level debug \
  --daemon
```

## Server Status and Health

### `primusdb server status`

Check whether the server is running and view uptime, version, and connection count.

```bash
primusdb server status
primusdb server status --verbose   # More detail
```

### `primusdb server health`

Perform a health check against the running server.

```bash
# Basic health check
primusdb server health

# Deep health check (includes storage engine checks)
primusdb server health --deep
```

### HTTP Endpoints

These are also accessible directly via HTTP:

```bash
# Simple health check (returns 200 OK)
curl http://localhost:8080/health

# Detailed status JSON
curl http://localhost:8080/status

# Prometheus metrics
curl http://localhost:8080/metrics
```

## Diagnostics

### `primusdb doctor`

Run a comprehensive diagnostic scan of the local PrimusDB environment. Checks include:
- Whether the server binary exists and is executable
- Whether the configured data directory is writable
- Whether the configured port is available
- Whether configuration files are valid

```bash
# Standard diagnostics
primusdb doctor

# Aggressive mode (runs additional checks)
primusdb doctor --aggressive

# Write a diagnostic report to file
primusdb doctor --report diag.txt
```

## Stopping the Server

### `primusdb server stop`

Gracefully shuts down the running server.

```bash
# Graceful stop (default 30s timeout)
primusdb server stop

# Custom timeout
primusdb server stop --timeout 60

# Force stop (immediate SIGKILL)
primusdb server stop --force
```

### Ctrl+C

If the server was started in the foreground, pressing `Ctrl+C` sends SIGINT and triggers graceful shutdown.

### Using the dev-stop Script

A convenience script is provided at `scripts/dev-stop.sh`:

```bash
# Stop server on default port (8080)
./scripts/dev-stop.sh

# Stop server on a custom port
./scripts/dev-stop.sh --port 9090
```

## Data Persistence Notes

- All data is stored in the directory specified by `storage.data_dir` (default: `./data`).
- The data directory contains subdirectories for each storage engine (columnar, vector, document, relational).
- Stopping the server does not delete data. Restarting the server reopens the existing data files.
- To start fresh, stop the server and delete the data directory:

```bash
primusdb server stop
rm -rf ./data
primusdb server start
```

- Backups can be created with:

```bash
primusdb backup --destination /path/to/backup
```

- For encryption at rest, ensure `[security] encryption_enabled = true` in your config (this is the default).

## Ports Reference

| Port | Purpose | Default |
|------|---------|---------|
| 8080 | HTTP API and health endpoints | Yes |
| 9090 | Prometheus metrics endpoint | (configured separately) |
