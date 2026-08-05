# PrimusDB CLI Guide

PrimusDB provides a unified command-line interface for all database operations. The `primusdb` binary replaces the legacy `primusdb-server` and `primusdb-cli` binaries with a single entry point.

## Quick Start

```bash
# Show help
primusdb --help

# Show version
primusdb version

# Start a server
primusdb server start

# Connect to a running server (interactive REPL)
primusdb connect --server http://localhost:8080

# Execute a query
primusdb query "SELECT * FROM users LIMIT 10"

# Execute SQL from file
primusdb sql -f query.sql

# Discover local instances
primusdb discover
```

## Command Structure

The CLI is organized into hierarchical subcommands:

```
primusdb
├── server          Manage server lifecycle
│   ├── start       Start the PrimusDB server daemon
│   ├── stop        Stop the running server
│   ├── restart     Restart the server
│   ├── status      Show server status
│   ├── health      Check server health
│   └── config      View or modify configuration
├── connect         Connect to a running PrimusDB instance (opens the REPL)
├── shell           Launch the interactive REPL shell
├── query           Execute a raw query
├── sql             Execute a SQL query
├── health          Alias for server health
├── status          Alias for server status
├── db              Manage databases
│   ├── list        List all databases
│   ├── create      Create a new database (supports `--engine` and `--namespace`)
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
│   └── health      Check cluster health
├── protocol        Manage protocol layer and peer connections
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
├── restore         Restore database from a backup (top-level)
├── metrics         Show database and system metrics
├── auth            Authentication commands
│   ├── login       Authenticate and obtain a session token
│   ├── logout      Invalidate the current session
│   ├── token       Manage authentication tokens
│   └── whoami      Display current user identity
├── user            User management commands
│   ├── create      Create a new user
│   ├── list        List users
│   ├── disable     Disable or re-enable a user
│   └── roles       Manage user role assignments
├── role            Role management commands
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
│   ├── index       Create or rebuild a vector index (not yet wired)
│   ├── stats       Show vector index statistics (not yet wired)
│   └── compact     Compact and optimize vector indexes (not yet wired)
├── graph           Graph operations (registered but not yet available — see note)
├── cdc             Change Data Capture (CDC) operations
│   ├── status      Show CDC status
│   ├── stream      Manage a CDC stream
│   ├── subscribe   Subscribe to a CDC stream
│   └── offsets     Show CDC offset information
├── explain         Explain a query plan without executing it
├── bench           Benchmark and performance testing
│   ├── run         Run a benchmark
│   ├── list        List available benchmark profiles
│   └── report      Generate a benchmark report
├── ts              Time series operations
│   ├── list        List time series metrics
│   ├── describe    Describe a time series metric
│   ├── query       Query time series data points
│   ├── aggregate   Aggregate time series data
│   ├── downsample  Downsample a metric to a lower resolution
│   ├── retain      Apply a retention policy to a metric
│   ├── resolution  Add or update a resolution for a metric
│   └── stats       Show engine statistics
├── migrate         Migrate data from external databases
│   ├── inspect-source  Inspect the source database
│   ├── plan        Plan a migration
│   ├── import      Execute a migration import
│   ├── validate    Validate a migration configuration
│   └── report      Generate a migration report
├── governor        Resource Governor (execution governance)
│   ├── status      Show governor status
│   ├── policies    List governance policies
│   ├── inspect     Inspect a specific execution
│   ├── metrics     Show governor metrics snapshot
│   ├── violations  List policy violations
│   └── set         Set or update a governance policy
├── certs           Certificate management (create CA, sign certs, self-signed)
├── doctor          Run diagnostic checks on the database
├── discover        Discover PrimusDB nodes on the network
├── completion      Generate shell completion scripts
└── version         Display version information
```

> **Note**: The `graph` subcommand group is registered but graph operations are not
> yet available via the CLI — use the SQL interface instead.

## Installation

### From Source

```bash
git clone https://github.com/devahil/primusdb.git
cd primusdb
cargo build --release
./target/release/primusdb --help
```

### With Cargo

```bash
cargo install primusdb
```

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

## Global Options

These options are available on every command:

| Option | Description | Default |
|--------|-------------|---------|
| `--server-url` | PrimusDB server base URL | `http://localhost:8080` |
| `--format` | Output format (table, json, csv, yaml, plain) | `table` |
| `--timeout` | Request timeout in milliseconds | `30000` |
| `--output` | Write output to a file instead of stdout | — |

## Output Formats

PrimusDB supports multiple output formats controlled by the `--format` flag:

- **table** — Human-readable aligned columns (default)
- **json** — Machine-readable JSON
- **csv** — Comma-separated values
- **yaml** — YAML format (falls back to JSON)
- **plain** — Simple plain text

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Connection failure |
| 4 | Authentication failure |
| 5 | Query error |
| 6 | Not found |
| 7 | Unsupported operation |
| 8 | Timeout |

## Environment

PrimusDB respects the following environment variables:

- `PRIMUSDB_URL` — Default server URL (overridden by `--server-url`)
- `PRIMUSDB_FORMAT` — Default output format
- `PRIMUSDB_TIMEOUT` — Default request timeout
- `RUST_LOG` — Logging level (e.g., `debug`, `info`, `warn`, `error`)
