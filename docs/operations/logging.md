# Logging

PrimusDB uses the [tracing](https://docs.rs/tracing/) framework for structured, asynchronous-aware logging. This guide covers configuration and usage.

## RUST_LOG Environment Variable

The `RUST_LOG` environment variable controls the log level for all Rust code, including PrimusDB and its dependencies.

```bash
# Basic usage
export RUST_LOG=info
primusdb server start

# Per-module level
export RUST_LOG=warn,primusdb=debug
primusdb server start

# Trace all modules
export RUST_LOG=trace
primusdb server start

# Include dependencies
export RUST_LOG=info,primusdb=debug,tower_http=warn
primusdb server start
```

You can also set it inline:

```bash
RUST_LOG=debug primusdb server start
```

## Log Levels

The following levels are available, ordered by increasing verbosity:

| Level | Usage | Description |
|-------|-------|-------------|
| `error` | Runtime errors | Serious failures that require operator intervention |
| `warn` | Warning conditions | Unexpected but non-fatal situations |
| `info` | Normal operations | Server start/stop, client connections, configuration |
| `debug` | Detailed operations | Query execution, storage operations, API calls |
| `trace` | Very detailed | Protocol frames, internal function entry/exit |

### Configuration File

Set the default log level in `primusdb.toml`:

```toml
[logging]
level = "info"
format = "text"
```

### Command-Line Flag

```bash
primusdb server start --log-level debug
primusdb server start --log-level trace
```

The `--log-level` flag is equivalent to setting `RUST_LOG` and takes precedence over the config file value.

## Log Format

PrimusDB supports two log output formats: plain text and JSON.

### Text Format (Default)

Human-readable format suitable for development and interactive use:

```
2026-01-15T12:00:00.123456Z  INFO primusdb::server: Starting PrimusDB server on 127.0.0.1:8080
2026-01-15T12:00:00.234567Z DEBUG primusdb::storage: Opening columnar engine at ./data/columnar
2026-01-15T12:00:00.345678Z  INFO primusdb::api: Listening on 127.0.0.1:8080
2026-01-15T12:00:05.000000Z  WARN primusdb::cluster: No peers discovered, running in single-node mode
2026-01-15T12:00:10.123456Z ERROR primusdb::storage: Write error: Permission denied (os error 13)
```

### JSON Format

Structured JSON format suitable for log aggregation systems (ELK, Grafana Loki, Datadog, etc.):

```toml
[logging]
format = "json"
```

Example JSON output:

```json
{"timestamp":"2026-01-15T12:00:00.123456Z","level":"INFO","target":"primusdb::server","message":"Starting PrimusDB server on 127.0.0.1:8080"}
{"timestamp":"2026-01-15T12:00:00.234567Z","level":"DEBUG","target":"primusdb::storage","message":"Opening columnar engine at ./data/columnar"}
{"timestamp":"2026-01-15T12:00:00.345678Z","level":"INFO","target":"primusdb::api","message":"Listening on 127.0.0.1:8080"}
```

Each JSON log entry includes:

| Field | Description |
|-------|-------------|
| `timestamp` | ISO 8601 timestamp with microsecond precision |
| `level` | Log level (ERROR, WARN, INFO, DEBUG, TRACE) |
| `target` | Rust module path |
| `message` | The log message |

Dependency crates that also log through tracing will appear with their own module paths (e.g., `tower_http::trace`).

## Tracing Integration

PrimusDB uses the `tracing` crate throughout its codebase. This provides:

- **Asynchronous awareness** — spans are correctly propagated across `await` boundaries
- **Structured fields** — additional context can be attached to log events
- **Per-module filtering** — granular control over which subsystems produce log output
- **Performance** — disabled events have near-zero overhead

### Span Fields

When debug logging is enabled, tracing spans may include structured fields such as:

- `request_id` — unique identifier for each HTTP request
- `storage_type` — the target storage engine
- `table_name` — the target table or collection
- `duration_ms` — operation duration

### Using Tracing with Application Code

If you embed PrimusDB as a library, you can replace the default subscriber with your own:

```rust
use tracing_subscriber::EnvFilter;

tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .json()
    .init();
```

## Log Configuration in Config File

The `[logging]` section in `primusdb.toml` controls log behavior:

```toml
[logging]
# Log level: error, warn, info, debug, trace
level = "info"

# Output format: "text" or "json"
format = "text"

# Optional: log to a file instead of stderr
file = "/var/log/primusdb/primusdb.log"

# Optional: maximum log file size in bytes before rotation
max_file_size = 104857600

# Optional: maximum number of rotated log files to keep
max_files = 10
```

> **Note:** File logging and rotation are planned features. In v1.3.1-alpha, log output goes to stderr. Use shell redirection or a process manager to capture logs to a file.

### Shell Redirection Example

```bash
# Redirect stderr to a log file
primusdb server start 2>> /var/log/primusdb/primusdb.log

# Separate stdout and stderr
primusdb server start \
  > /var/log/primusdb/stdout.log \
  2> /var/log/primusdb/stderr.log

# Use with systemd (journald captures stderr automatically)
```

### Supervisor Configuration

```ini
[program:primusdb]
command=/usr/local/bin/primusdb server start --bind 0.0.0.0:8080
user=primusdb
stdout_logfile=/var/log/primusdb/stdout.log
stderr_logfile=/var/log/primusdb/stderr.log
stdout_logfile_maxbytes=100MB
stdout_logfile_backups=10
stderr_logfile_maxbytes=100MB
stderr_logfile_backups=10
```

## Log Levels by Subsystem

Use the `RUST_LOG` variable to fine-tune logging per module:

```bash
# Only see errors from storage, info from everything else
RUST_LOG=info,primusdb::storage=error primusdb server start

# Debug for API, info for everything else
RUST_LOG=info,primusdb::api=debug primusdb server start

# Trace everything in the cluster module
RUST_LOG=info,primusdb::cluster=trace primusdb server start
```

Key module paths for filtering:

| Module | Description |
|--------|-------------|
| `primusdb::server` | Server lifecycle |
| `primusdb::api` | HTTP API handlers |
| `primusdb::storage` | Storage engine operations |
| `primusdb::cluster` | Cluster coordination |
| `primusdb::consensus` | Consensus protocol |
| `primusdb::federation` | Cross-cluster federation |
| `primusdb::ai` | AI/ML operations |
| `primusdb::auth` | Authentication and authorization |
| `primusdb::protocol` | Peer-to-peer protocol |
| `primusdb::kv` | Key-value engine |
