# PrimusDB CLI Usage Guide

The `primusdb` binary is the unified command-line interface for all PrimusDB operations: server lifecycle, database management, querying, namespaces, cluster operations, and more.

## Quick Reference

```bash
# Show help
primusdb --help

# Show version
primusdb version

# Start a server
primusdb server start

# Execute a query
primusdb query "SELECT * FROM users"

# Launch interactive TUI
primusdb tui
```

## Command Tree

The CLI has 26+ top-level commands organized into subcommand groups:

```
primusdb
├── server          Manage server lifecycle
│   ├── start       Start the PrimusDB server daemon
│   ├── stop        Stop the running server
│   ├── restart     Restart the server
│   ├── status      Show server status
│   ├── health      Check server health
│   └── config      View or modify configuration
├── health          Alias for server health
├── status          Alias for server status
├── connect         Connect to a running instance interactively
├── tui             Launch the terminal user interface
├── query           Execute a raw SQL query
├── sql             Execute a SQL query (string or file)
├── explain         Explain a query plan without executing
├── db              Manage databases
│   ├── list        List all databases
│   ├── create      Create a new database
│   ├── drop        Drop a database
│   ├── describe    Describe a database
│   └── use         Switch active database
├── engine          Manage storage engines
│   ├── list        List registered storage engines
│   ├── status      Show engine status
│   ├── inspect     Inspect internal engine state
│   └── metrics     Show engine metrics
├── namespace       Manage namespaces
│   ├── list        List namespaces
│   ├── create      Create a namespace
│   ├── drop        Drop a namespace
│   ├── describe    Describe a namespace
│   └── policy      View or set namespace policy
├── config          Manage configuration
│   ├── init        Generate a default configuration file
│   ├── validate    Validate a configuration file
│   └── show        Display current configuration
├── instance        Manage running instances
│   ├── list        List all local instances
│   ├── discover    Discover instances on a host/port range
│   ├── inspect     Show detailed instance info
│   ├── connect     Test connectivity to an instance
│   ├── stop        Stop a running instance
│   └── logs        View instance logs
├── cluster         Manage cluster operations
│   ├── status      Show cluster status
│   ├── nodes       List cluster nodes
│   ├── join        Join an existing cluster
│   ├── leave       Leave the current cluster
│   ├── rebalance   Trigger cluster rebalance
│   ├── failover    Trigger manual failover
│   ├── health      Check cluster health
│   ├── sync        Trigger cluster synchronization
│   └── config      View or modify cluster config
├── protocol        Manage protocol layer
│   ├── health      Check protocol layer health
│   ├── status      Show protocol status and capabilities
│   ├── peers       List protocol peers
│   └── metrics     Show protocol metrics
├── backup          Create and manage backups
│   ├── create      Create a new backup
│   ├── list        List available backups
│   ├── inspect     Inspect a backup archive
│   ├── restore     Restore from a backup
│   └── verify      Verify backup integrity
├── restore         Restore from a backup (top-level)
├── metrics         Show database and system metrics
├── auth            Authentication commands
│   ├── login       Authenticate and obtain a session token
│   ├── logout      Invalidate current session
│   ├── token       Manage authentication tokens
│   └── whoami      Display current user identity
├── user            User management
│   ├── create      Create a new user
│   ├── list        List users
│   ├── disable     Disable or re-enable a user
│   └── roles       Manage user role assignments
├── role            Role management
│   ├── create      Create a new role
│   ├── list        List all roles
│   ├── grant       Grant a permission to a role
│   └── revoke      Revoke a permission from a role
├── ai              AI/ML model operations
│   ├── models      List available AI/ML models
│   ├── train       Train a new model
│   ├── predict     Make predictions using a trained model
│   ├── analyze     Analyze data patterns
│   └── anomalies   Detect anomalies in a dataset
├── vector          Vector search and index management
│   ├── search      Perform vector similarity search
│   ├── index       Create or rebuild a vector index
│   ├── stats       Show vector index statistics
│   ├── compact     Compact and optimize vector indexes
├── graph           Graph traversal and management
│   ├── nodes       Query graph nodes
│   ├── edges       Query graph edges
│   ├── query       Execute a graph query
│   └── traverse    Traverse the graph from a starting node
├── cdc             Change Data Capture (CDC)
│   ├── status      Show CDC status
│   ├── stream      Manage a CDC stream
│   ├── subscribe   Subscribe to a CDC stream
│   └── offsets     Show CDC offset information
├── bench           Benchmark and performance testing
│   ├── run         Run a benchmark
│   ├── list        List available benchmark profiles
│   └── report      Generate a benchmark report
├── doctor          Run diagnostic checks
├── discover        Discover PrimusDB nodes on the network
├── completion      Generate shell completion scripts
└── version         Display version information
```

---

## Global Options

These options are available on every command:

| Option | Description | Default | Env Variable |
|--------|-------------|---------|-------------|
| `--server-url` | PrimusDB server base URL | `http://localhost:8080` | `PRIMUSDB_URL` |
| `--format` | Output format | `table` | `PRIMUSDB_FORMAT` |
| `--timeout` | Request timeout in milliseconds | `30000` | `PRIMUSDB_TIMEOUT` |
| `--output` | Write output to a file instead of stdout | — | — |

**Examples:**
```bash
# Connect to a remote server with JSON output and 60s timeout
primusdb --server-url http://192.168.1.100:8080 --format json --timeout 60000 query "SELECT * FROM users"

# Use environment variables
export PRIMUSDB_URL=http://prod-server:8080
export PRIMUSDB_FORMAT=json
primusdb status
```

---

## Output Formats

Controlled by the `--format` global flag:

| Format | Description |
|--------|-------------|
| `table` | Human-readable aligned columns (default) |
| `json` | Machine-readable JSON |
| `csv` | Comma-separated values |
| `yaml` | YAML format (falls back to JSON) |
| `plain` | Simple plain text (varies by command) |

**Examples:**
```bash
primusdb query "SELECT * FROM users" --format table
primusdb query "SELECT * FROM users" --format json
primusdb query "SELECT * FROM users" --format csv
primusdb query "SELECT * FROM users" --format yaml
```

---

## Server Lifecycle

### `primusdb server start`
```bash
primusdb server start
primusdb server start --bind 0.0.0.0:8080
primusdb server start --config prod.toml --daemon --log-level debug
primusdb server start --data-dir /var/lib/primusdb
```

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config <FILE>` | Path to configuration file | — |
| `-b, --bind <ADDRESS>` | Bind address (`host:port`) | `127.0.0.1:8080` |
| `-d, --data-dir <PATH>` | Data storage directory | `./data` |
| `--daemon` | Run as a daemon process | `false` |
| `--log-level <LEVEL>` | Log level | `info` |

### `primusdb server stop`
```bash
primusdb server stop
primusdb server stop --force
primusdb server stop --timeout 60
```

### `primusdb server restart`
```bash
primusdb server restart
primusdb server restart --config prod.toml --timeout 60
```

### `primusdb server status`
```bash
primusdb server status
primusdb server status --verbose
```

### `primusdb server health`
```bash
primusdb server health
primusdb server health --deep
```

### `primusdb server config`
```bash
primusdb server config --list
primusdb server config --get storage.data_dir
primusdb server config --set network.port=9090
primusdb server config --file primusdb.toml
```

### Shorthand Aliases

```bash
primusdb health       # Same as: primusdb server health
primusdb status       # Same as: primusdb server status
primusdb connect      # Same as: primusdb instance connect
```

---

## Database Management

### `primusdb db list`
```bash
primusdb db list
primusdb db list --all
primusdb db list --engine vector
```

### `primusdb db create`
```bash
primusdb db create mydb
primusdb db create embeddings --engine vector
primusdb db create mydb --namespace tenant1/project2
```

**Engine types:** `columnar`, `vector`, `document`, `relational`, `graph`

### `primusdb db drop`
```bash
primusdb db drop mydb
primusdb db drop mydb --force
```

### `primusdb db describe`
```bash
primusdb db describe mydb
primusdb db describe mydb --schema
```

### `primusdb db use`
```bash
primusdb db use mydb
```

---

## Engine Management

### `primusdb engine list`
```bash
primusdb engine list
primusdb engine list --verbose
```

### `primusdb engine status`
```bash
primusdb engine status vector
```

### `primusdb engine inspect`
```bash
primusdb engine inspect columnar
primusdb engine inspect vector --component index
primusdb engine inspect document --raw
```

### `primusdb engine metrics`
```bash
primusdb engine metrics columnar
primusdb engine metrics relational --filter latency
```

---

## Namespace Management

```bash
primusdb namespace list
primusdb namespace list --parent tenant1
primusdb namespace list --full-paths

primusdb namespace create tenant1 --description "Tenant 1"
primusdb namespace create tenant1/project2 --parent tenant1 --quota storage=1GB

primusdb namespace describe tenant1
primusdb namespace describe tenant1 --resources

primusdb namespace drop tenant1/unused --force
primusdb namespace drop tenant1 --recursive --force

primusdb namespace policy tenant1 --list
primusdb namespace policy tenant1 --set max_databases=20
primusdb namespace policy tenant1 --unset max_databases
```

---

## Config Management

### `primusdb config init`
```bash
primusdb config init
primusdb config init --profile local
primusdb config init --profile single-node --output /etc/primusdb/primusdb.toml
primusdb config init --force
```

**Profiles:** `local`, `dev`, `single-node`, `cluster-node`, `secure`

### `primusdb config validate`
```bash
primusdb config validate
primusdb config validate --config /etc/primusdb/primusdb.toml
```

### `primusdb config show`
```bash
primusdb config show
primusdb config show --config /etc/primusdb/primusdb.toml
```

---

## Instance Management

```bash
primusdb instance list
primusdb instance discover --host 192.168.1.100 --start-port 8080 --max-ports 10
primusdb instance inspect http://localhost:8080 --verbose
primusdb instance connect http://localhost:8080
primusdb instance stop http://localhost:8080 --force
primusdb instance logs http://localhost:8080 --lines 100 --follow
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error |
| `2` | Invalid arguments |
| `3` | Connection failure |
| `4` | Authentication failure |
| `5` | Query error |
| `6` | Not found |
| `7` | Unsupported operation |
| `8` | Timeout |

---

## Environment Variables

| Variable | Description | Overrides |
|----------|-------------|-----------|
| `PRIMUSDB_URL` | Default server URL | `--server-url` default |
| `PRIMUSDB_FORMAT` | Default output format | `--format` default |
| `PRIMUSDB_TIMEOUT` | Default request timeout (ms) | `--timeout` default |
| `PRIMUSDB_CONFIG` | Path to config file | `--config` flag |
| `PRIMUSDB_DATA_DIR` | Data directory | Config `storage.data_dir` |
| `PRIMUSDB_LOG_LEVEL` | Logging level | `--log-level` flag |
| `RUST_LOG` | Rust tracing/log level | All log output |

---

## Shell Completion

```bash
# Bash
primusdb completion bash > /etc/bash_completion.d/primusdb

# Zsh
primusdb completion zsh > /usr/local/share/zsh/site-functions/_primusdb

# Fish
primusdb completion fish > ~/.config/fish/completions/primusdb.fish

# PowerShell
primusdb completion powershell > _primusdb.ps1
```
