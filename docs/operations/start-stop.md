# Starting and Stopping PrimusDB

This guide covers how to start and stop a PrimusDB server in development and production environments.

## Starting with `primusdb server start`

The unified CLI provides a single entry point for all server management operations:

```bash
# Start with default settings (127.0.0.1:8080)
primusdb server start

# Start with custom bind address
primusdb server start --bind 0.0.0.0:8080

# Start with configuration file
primusdb server start --config /etc/primusdb/primusdb.toml

# Start with custom data directory
primusdb server start --data-dir /var/lib/primusdb/data

# Start as a daemon with debug logging
primusdb server start --daemon --log-level debug
```

### Command-Line Flags

| Flag | Description | Default |
|------|-------------|---------|
| `-c, --config <FILE>` | Path to TOML configuration file | — |
| `-b, --bind <ADDRESS>` | Bind address (`host:port`) | `127.0.0.1:8080` |
| `-d, --data-dir <PATH>` | Data storage directory | `./data` |
| `--daemon` | Run as a background daemon process | `false` |
| `--log-level <LEVEL>` | Log level (`trace`, `debug`, `info`, `warn`, `error`) | `info` |

### Environment Variables

These environment variables override the defaults and can be used instead of flags:

```bash
export PRIMUSDB_CONFIG=/etc/primusdb/primusdb.toml
export PRIMUSDB_DATA_DIR=/var/lib/primusdb/data
export PRIMUSDB_LOG_LEVEL=debug
export RUST_LOG=info

primusdb server start --bind 0.0.0.0:9090
```

Precedence (highest first): command-line flags > environment variables > config file > built-in defaults.

## Starting with `primusdb server`

`primusdb server start` is the recommended way to start the server.

```bash
# Start with defaults
primusdb server start

# Start with custom host and port
primusdb server start --host 0.0.0.0 --port 8080

# Use configuration file
primusdb server start --config /etc/primusdb/primusdb.toml

# Enable cluster mode
primusdb server start --cluster --node-id server-1

# Production-style start
primusdb server start \
  --host 0.0.0.0 \
  --port 8080 \
  --data-dir /data/primusdb \
  --log-level warn
```

> **Note:** The legacy `primusdb-server` binary was removed in v1.3.2-alpha. All deployments use `primusdb server start`.

## Development Scripts

The project includes helper scripts under `scripts/` for rapid local development:

```bash
# Start a development server (debug build)
./scripts/dev-start.sh

# Start on a custom port
./scripts/dev-start.sh --port 8081

# Start with a custom data directory
./scripts/dev-start.sh --data-dir ./data/myinstance

# Use a release build
./scripts/dev-start.sh --release

# Start with a configuration file
./scripts/dev-start.sh --config config/examples/primusdb.dev.toml
```

The `dev-start.sh` script automatically builds the binary if it does not exist.

## Stopping with Ctrl+C

For foreground servers, pressing `Ctrl+C` sends a SIGINT signal. The server performs a graceful shutdown:

1. Stop accepting new connections
2. Drain in-flight requests (up to the configured timeout)
3. Flush pending writes to disk
4. Close storage engines
5. Exit

```bash
primusdb server start
# ... server is running ...
# Press Ctrl+C
^C
# Server performs graceful shutdown...
```

## Stopping with `primusdb server stop`

Send a graceful shutdown signal to the running server:

```bash
# Graceful stop with 30-second timeout
primusdb server stop

# Force immediate shutdown
primusdb server stop --force

# Custom timeout
primusdb server stop --timeout 60
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--timeout <SECONDS>` | Graceful shutdown timeout | `30` |
| `--force` | Skip graceful shutdown, kill immediately | `false` |

## Stopping with `scripts/dev-stop.sh`

The development helper script stops a server by finding the PID listening on the configured port:

```bash
# Stop server on default port (8080)
./scripts/dev-stop.sh

# Stop server on custom port
./scripts/dev-stop.sh --port 8081
```

The script performs a graceful kill first, waits up to 5 seconds, and force-kills if the process has not exited.

## Using Config Files

PrimusDB looks for configuration files in the following order (first found wins):

1. Path specified by `--config` / `-c` flag
2. `./primusdb.toml` in the current working directory
3. `./config.toml` in the current working directory
4. `~/.config/primusdb/config.toml`
5. `/etc/primusdb/config.toml`

### Minimal Configuration

```toml
[storage]
data_dir = "./data"
compression = "lz4"

[network]
bind_address = "127.0.0.1"
port = 8080

[security]
encryption_enabled = true
auth_required = false

[cluster]
enabled = false
```

### Initialize a Default Config

```bash
primusdb config init
primusdb config init --output /etc/primusdb/primusdb.toml
primusdb config init --force  # Overwrite existing file
```

Example configurations are available under `config/examples/`:

- `primusdb.dev.toml` — Development server
- `primusdb.single-node.toml` — Single-node production
- `primusdb.cluster-node.toml` — Cluster node
- `primusdb.secure.toml` — Hardened security
- `primusdb.local.toml` — Local instance

### Production systemd Service

```ini
[Unit]
Description=PrimusDB Hybrid Database Server
After=network.target local-fs.target

[Service]
Type=exec
User=primusdb
ExecStart=/usr/local/bin/primusdb server start \
  --config /etc/primusdb/primusdb.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

## Alpha Limitations

As of v1.3.2-alpha, the following limitations apply to server lifecycle management:

- **Daemon mode** (`--daemon`) flag is accepted but the server does not fully detach from the terminal. Use `systemd` or a process manager like `supervisord` for production daemonization.
- **`primusdb server stop`** sends a stop signal but does not track PIDs across sessions. It is best used with the process manager or the `dev-stop.sh` script.
- **Graceful shutdown** stops the HTTP listener immediately; in-flight requests may be dropped if they exceed the timeout window.
- **Config file reload** (`SIGHUP`) is not yet implemented. Restart the process to apply configuration changes.
- **Windows** is not supported as a server platform in this release.
