# PrimusDB Command Reference

## `primusdb server`

Manage the PrimusDB server lifecycle.

### `primusdb server start`

Start the PrimusDB server daemon.

**Usage:**
```
primusdb server start [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config <FILE>` | Path to configuration file | — |
| `-b, --bind <ADDRESS>` | Bind address (`host:port`) | `127.0.0.1:8080` |
| `-d, --data-dir <PATH>` | Data storage directory | — |
| `--daemon` | Run as a daemon process | `false` |
| `--log-level <LEVEL>` | Log level (trace, debug, info, warn, error) | `info` |
| `--federation-id <ID>` | Federation identifier | `default` |
| `--cluster-id <ID>` | Cluster identifier | — |
| `--region <REGION>` | Deployment region | — |
| `--federation-discovery <HOST>` | Federation peer discovery addresses | — |
| `--tls-enabled` | Enable TLS/HTTPS | `false` |
| `--tls-cert <FILE>` | TLS certificate file (PEM) | — |
| `--tls-key <FILE>` | TLS private key file (PEM) | — |
| `--tls-ca <FILE>` | CA certificate file for mTLS (PEM) | — |
| `--mtls-enabled` | Require mutual TLS (client certs) | `false` |

**Examples:**
```bash
# Start with defaults
primusdb server start

# Start on all interfaces with custom port
primusdb server start --bind 0.0.0.0:8080

# Start with config file
primusdb server start --config prod.toml

# Start as daemon with debug logging
primusdb server start --daemon --log-level debug

# Start with custom data directory
primusdb server start --data-dir /var/lib/primusdb

# Start with TLS
primusdb server start --tls-enabled --tls-cert ./cert.pem --tls-key ./key.pem

# Start with TLS + mTLS + federation
primusdb server start \
  --tls-enabled --tls-cert ./cert.pem --tls-key ./key.pem \
  --tls-ca ./ca.crt --mtls-enabled \
  --federation-discovery node2:8081 --cluster-id prod-1
```

**Output format:**
```
Starting PrimusDB server on 0.0.0.0:8080...
```

### `primusdb server stop`

Stop the running server.

**Usage:**
```
primusdb server stop [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--timeout <SECONDS>` | Graceful shutdown timeout | `30` |
| `--force` | Force immediate shutdown | `false` |

**Examples:**
```bash
# Graceful stop with 30s timeout
primusdb server stop

# Force immediate stop
primusdb server stop --force

# Custom timeout
primusdb server stop --timeout 60
```

### `primusdb server restart`

Restart the server.

**Usage:**
```
primusdb server restart [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config <FILE>` | Path to configuration file | — |
| `--timeout <SECONDS>` | Shutdown timeout before restart | `30` |

**Examples:**
```bash
primusdb server restart
primusdb server restart --config prod.toml --timeout 60
```

### `primusdb server status`

Show server status information.

**Usage:**
```
primusdb server status [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--verbose` | Show detailed status | `false` |

**Example:**
```bash
primusdb server status --verbose
```

**Output format:**
```
Key        Value
---        -----
Status     Running
Version    1.3.2-alpha
Verbose    true
```

### `primusdb server health`

Check server health.

**Usage:**
```
primusdb server health [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--deep` | Perform deep health check | `false` |

**Example:**
```bash
primusdb server health --deep
```

### `primusdb server config`

View or modify server configuration.

**Usage:**
```
primusdb server config [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-g, --get <KEY>` | Get a configuration value |
| `-s, --set <KEY=VALUE>` | Set a configuration value |
| `-l, --list` | List all configuration values |
| `-f, --file <FILE>` | Path to configuration file |

**Examples:**
```bash
# List all config
primusdb server config --list

# Get a specific value
primusdb server config --get storage.data_dir

# Set a value
primusdb server config --set network.port=9090

# View config from file
primusdb server config --file prod.toml
```

---

## `primusdb connect`

Connect to a running PrimusDB instance and drop into the interactive shell (console-over-console REPL).

**Usage:**
```
primusdb connect [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-s, --server <URL>` | Server URL | — |
| `--timeout <SECONDS>` | Connection timeout | `10` |

**Examples:**
```bash
# Connect to default server
primusdb connect

# Connect to a specific server
primusdb connect --server http://localhost:8080

# Connect with longer timeout
primusdb connect --server http://192.168.1.100:8080 --timeout 30
```

> `primusdb connect` and `primusdb shell` both enter the interactive shell.
> For a one-shot health check use `primusdb health --server <URL>`.

---

## `primusdb shell`

Enter the interactive shell against a running PrimusDB instance.

**Usage:**
```
primusdb shell [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-s, --server <URL>` | Server URL | `http://localhost:8080` |
| `--timeout <SECONDS>` | Connection timeout | `10` |

**Examples:**
```bash
# Open the shell against the default local server
primusdb shell

# Open the shell against a remote node
primusdb shell --server http://192.168.1.100:8080
```

### Interactive Shell

The shell is a REPL (console-over-console). The prompt shows the connected
server and the active database context:

```
primusdb@localhost:8080 [mydb]> 
```

Every line you type is parsed as a normal `primusdb` command and executed
against the connected server (the `--server` URL is injected automatically).
Tab completion walks the command tree, and inline gray hints suggest the
command being typed.

**REPL-only commands:**

| Command | Description |
|---------|-------------|
| `connect <url>` | Switch the connected server (e.g. `connect 192.168.1.5:8080`) |
| `disconnect` | Leave the shell |
| `use <db>` | Set the active database for `query`/`sql` (shown in the prompt) |
| `use none` | Clear the active database |
| `help` / `?` | Show the interactive help / cheat sheet |
| `history` | Show command history |
| `clear` | Clear the screen |
| `exit` / `quit` | Leave the shell |

**Example session:**
```
$ primusdb shell
Connected to http://localhost:8080 (type 'help' for commands, 'exit' to quit)
primusdb@localhost:8080> db list
primusdb@localhost:8080> db create mydb --engine relational
primusdb@localhost:8080> use mydb
Switched to database 'mydb'
primusdb@localhost:8080 [mydb]> query "SELECT * FROM users"
primusdb@localhost:8080 [mydb]> ts query sensor_readings --tags sensor=a
primusdb@localhost:8080 [mydb]> exit
Bye.
```

History is persisted in `~/.config/primusdb/history`.

---

## `primusdb query`

Execute a raw query against the database.

**Usage:**
```
primusdb query <QUERY> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `QUERY` | Query string (space-separated, positionally required) |

**Options:**

| Option | Description |
|--------|-------------|
| `-d, --database <NAME>` | Target database name |

**Examples:**
```bash
# SELECT query
primusdb query "SELECT * FROM users LIMIT 10"

# INSERT
primusdb query "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')"

# Specify database
primusdb query "SELECT * FROM orders" --database mydb

# Multi-word query
primusdb query "SELECT u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id"
```

---

## `primusdb sql`

Execute a SQL query from a string or file.

**Usage:**
```
primusdb sql <SQL> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `SQL` | SQL string (space-separated, positionally required) |

**Options:**

| Option | Description |
|--------|-------------|
| `-d, --database <NAME>` | Target database name |

**Examples:**
```bash
# Execute inline SQL
primusdb sql "SELECT * FROM users"

# Multi-line SQL
primusdb sql "CREATE TABLE users (id INT PRIMARY KEY, name TEXT)"

# Specify database
primusdb sql "SELECT * FROM logs" --database analytics
```

---

## `primusdb db`

Manage databases.

### `primusdb db list`

List all databases.

**Usage:**
```
primusdb db list [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--all` | Show all databases including system | `false` |
| `--engine <ENGINE>` | Filter by engine type | — |

**Examples:**
```bash
primusdb db list
primusdb db list --all
primusdb db list --engine vector
```

### `primusdb db create`

Create a new database.

**Usage:**
```
primusdb db create <NAME> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `NAME` | Database name |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-e, --engine <ENGINE>` | Storage engine type | `document` |
| `-n, --namespace <NS>` | Parent namespace; the database is created as `NS.NAME` | — |

**Engine types:** `columnar`, `vector`, `document`, `relational`, `keyvalue`, `timeseries`

Creation is idempotent: if the database (namespace) already exists, the
existing database is returned instead of failing.

**Examples:**
```bash
# Create document database
primusdb db create mydb

# Create vector database
primusdb db create embeddings --engine vector

# Create under a parent namespace (becomes tenant1.mydb)
primusdb db create mydb --namespace tenant1

# Idempotent: succeeds if the database already exists
primusdb db create mydb --engine relational
```

### `primusdb db drop`

Drop a database.

**Usage:**
```
primusdb db drop <NAME> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `NAME` | Database name |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --force` | Force drop without confirmation | `false` |

**Example:**
```bash
primusdb db drop mydb --force
```

### `primusdb db describe`

Describe a database schema and metadata.

**Usage:**
```
primusdb db describe <NAME> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `NAME` | Database name |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--schema` | Show detailed schema information | `false` |

**Example:**
```bash
primusdb db describe mydb --schema
```

### `primusdb db use`

Switch the active database context.

**Usage:**
```
primusdb db use <NAME>
```

**Example:**
```bash
primusdb db use mydb
```

---

## `primusdb engine`

Manage storage engines.

### `primusdb engine list`

List registered storage engines.

**Usage:**
```
primusdb engine list [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--verbose` | Show detailed engine information | `false` |

**Example:**
```bash
primusdb engine list --verbose
```

### `primusdb engine status`

Show status of a specific storage engine.

**Usage:**
```
primusdb engine status <NAME>
```

**Example:**
```bash
primusdb engine status vector
```

### `primusdb engine inspect`

Inspect internal engine state.

**Usage:**
```
primusdb engine inspect <NAME> [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --component <NAME>` | Specific component to inspect | — |
| `--raw` | Show raw internal state | `false` |

**Example:**
```bash
primusdb engine inspect columnar
primusdb engine inspect vector --component index
primusdb engine inspect document --raw
```

### `primusdb engine metrics`

Show engine performance metrics.

**Usage:**
```
primusdb engine metrics <NAME> [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-f, --filter <PATTERN>` | Filter metrics by name pattern |

**Example:**
```bash
primusdb engine metrics columnar
primusdb engine metrics relational --filter latency
```

---

## `primusdb namespace`

Manage hierarchical namespaces.

### `primusdb namespace list`

List namespaces.

**Usage:**
```
primusdb namespace list [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-p, --parent <PATH>` | Only show children of parent | — |
| `--full-paths` | Show full namespace paths | `false` |

**Examples:**
```bash
primusdb namespace list
primusdb namespace list --parent tenant1
primusdb namespace list --full-paths
```

### `primusdb namespace create`

Create a namespace.

**Usage:**
```
primusdb namespace create <PATH> [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-d, --description <TEXT>` | Namespace description |
| `-p, --parent <PATH>` | Parent namespace path |
| `--quota <STRING>` | Resource quota (e.g. `storage=1GB`) |

**Examples:**
```bash
primusdb namespace create tenant1 --description "Tenant 1 namespace"
primusdb namespace create tenant1/project2 --parent tenant1 --quota storage=1GB
```

### `primusdb namespace drop`

Drop a namespace.

**Usage:**
```
primusdb namespace drop <PATH> [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-r, --recursive` | Recursively drop child namespaces | `false` |
| `-f, --force` | Force drop without confirmation | `false` |

**Example:**
```bash
primusdb namespace drop tenant1 --recursive --force
```

### `primusdb namespace describe`

Describe a namespace.

**Usage:**
```
primusdb namespace describe <PATH> [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--resources` | Show resources attached to namespace | `false` |

**Example:**
```bash
primusdb namespace describe tenant1 --resources
```

### `primusdb namespace policy`

View or set namespace policy.

**Usage:**
```
primusdb namespace policy <PATH> [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-s, --set <KEY=VALUE>` | Set a policy value |
| `-u, --unset <KEY>` | Unset a policy value |
| `-l, --list` | List current policy |

**Examples:**
```bash
primusdb namespace policy tenant1 --list
primusdb namespace policy tenant1 --set max_resources=100
primusdb namespace policy tenant1 --unset max_resources
```

---

## `primusdb cluster`

Manage cluster operations.

### `primusdb cluster status`

Show cluster status.

**Usage:**
```
primusdb cluster status [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--verbose` | Show detailed status | `false` |
| `--watch` | Continuously watch status | `false` |
| `--interval <SECONDS>` | Watch refresh interval | `2` |

**Example:**
```bash
primusdb cluster status --verbose
primusdb cluster status --watch --interval 5
```

### `primusdb cluster nodes`

List cluster nodes.

**Usage:**
```
primusdb cluster nodes [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--role <ROLE>` | Filter by role (leader, follower, candidate) |
| `--state <STATE>` | Filter by state (active, inactive, suspect) |

**Example:**
```bash
primusdb cluster nodes
primusdb cluster nodes --role leader
primusdb cluster nodes --state active --verbose
```

### `primusdb cluster join`

Join an existing cluster.

**Usage:**
```
primusdb cluster join <PEER> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PEER` | Peer address to join (e.g. `192.168.1.10:8080`) |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --node-id <ID>` | Custom node identifier | — |
| `--timeout <SECONDS>` | Join timeout | `30` |
| `--tls` | Use TLS for peer communication | `false` |

**Example:**
```bash
primusdb cluster join 192.168.1.10:8080 --node-id node-2 --tls
```

### `primusdb cluster leave`

Leave the current cluster.

**Usage:**
```
primusdb cluster leave [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--drain` | Drain data before leaving | `false` |
| `--force` | Force leave | `false` |

**Example:**
```bash
primusdb cluster leave --drain
```

### `primusdb cluster rebalance`

Trigger cluster rebalance.

**Usage:**
```
primusdb cluster rebalance [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --node <NODE>` | Target specific node for rebalance | — |
| `--strategy <STRATEGY>` | Rebalance strategy (size, iops, latency) | `size` |
| `--concurrency <N>` | Number of concurrent operations | `2` |

**Example:**
```bash
primusdb cluster rebalance
primusdb cluster rebalance --node node-2 --strategy iops
```

### `primusdb cluster failover`

Trigger manual failover for a node.

**Usage:**
```
primusdb cluster failover <NODE> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `NODE` | Node to trigger failover for |

**Options:**

| Option | Description |
|--------|-------------|
| `-t, --target <NODE>` | Specific target node to promote |
| `--force` | Force failover without checks |

**Example:**
```bash
primusdb cluster failover node-3
primusdb cluster failover node-3 --target node-5
```

### `primusdb cluster health`

Check cluster health.

**Usage:**
```
primusdb cluster health [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--diagnostic` | Run detailed diagnostic checks | `false` |
| `--threshold-ms <MS>` | Latency threshold for warnings | `100` |

**Example:**
```bash
primusdb cluster health --diagnostic
```

---

## `primusdb backup`

Create and manage backups.

### `primusdb backup create`

Create a new backup.

**Usage:**
```
primusdb backup create [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --destination <PATH>` | Backup destination path | — |
| `-db, --databases <NAMES>` | Comma-separated list of databases | — |
| `--compression <ALGO>` | Compression algorithm | `zstd` |
| `--encrypt` | Encrypt the backup | `false` |
| `-e, --description <TEXT>` | Backup description | — |

**Compression algorithms:** `zstd`, `lz4`, `none`

**Examples:**
```bash
# Full backup
primusdb backup create

# Backup specific databases
primusdb backup create --databases mydb,analytics --destination /backups/

# Encrypted backup with description
primusdb backup create --encrypt --description "Pre-upgrade backup"
```

### `primusdb backup list`

List available backups.

**Usage:**
```
primusdb backup list [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --directory <PATH>` | Backup directory to scan | — |
| `--verbose` | Show detailed backup info | `false` |

**Example:**
```bash
primusdb backup list
primusdb backup list --directory /backups/ --verbose
```

### `primusdb backup inspect`

Inspect a backup archive.

**Usage:**
```
primusdb backup inspect <PATH> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to backup archive |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--contents` | List contents of the backup | `false` |
| `--metadata` | Show backup metadata | `false` |

**Example:**
```bash
primusdb backup inspect /backups/primusdb-backup-20250101.zstd --contents --metadata
```

### `primusdb backup restore`

Restore from a backup.

**Usage:**
```
primusdb backup restore <PATH> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to backup archive |

**Options:**

| Option | Description |
|--------|-------------|
| `-d, --database <NAME>` | Restore a specific database |
| `--force` | Force restore (overwrite existing) |
| `--pitr <TIMESTAMP>` | Point-in-time recovery timestamp |

**Examples:**
```bash
primusdb backup restore /backups/primusdb-backup-20250101.zstd
primusdb backup restore /backups/db-backup.zstd --database mydb --force
primusdb backup restore /backups/backup.zstd --pitr "2025-01-01T12:00:00Z"
```

### `primusdb backup verify`

Verify backup integrity.

**Usage:**
```
primusdb backup verify <PATH> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to backup archive |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--full` | Perform full integrity verification | `false` |
| `--compare` | Compare checksums with metadata | `false` |

**Example:**
```bash
primusdb backup verify /backups/primusdb-backup-20250101.zstd --full
```

---

## `primusdb restore`

Restore database from a backup (top-level convenience command).

**Usage:**
```
primusdb restore <SOURCE> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `SOURCE` | Path to backup source |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --database <NAME>` | Restore a specific database | — |
| `--force` | Force restore | `false` |

**Example:**
```bash
primusdb restore /backups/backup.zstd --database mydb --force
```

---

## `primusdb metrics`

Show database and system metrics.

**Usage:**
```
primusdb metrics [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --filter <PATTERN>` | Filter metrics by name | — |
| `--watch` | Continuously watch metrics | `false` |
| `--interval <SECONDS>` | Watch refresh interval | `2` |

**Examples:**
```bash
primusdb metrics
primusdb metrics --filter storage
primusdb metrics --watch --interval 5
```

---

## `primusdb auth`

Authentication commands.

### `primusdb auth login`

Authenticate and obtain a session token.

**Usage:**
```
primusdb auth login <USERNAME> [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-p, --password <PASSWORD>` | Password (prompted if omitted) | — |
| `-r, --realm <REALM>` | Authentication realm | `default` |
| `--ttl <SECONDS>` | Session TTL | `86400` |

**Example:**
```bash
primusdb auth login admin
primusdb auth login admin --realm internal --ttl 3600
```

### `primusdb auth logout`

Invalidate the current session.

**Usage:**
```
primusdb auth logout [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--all` | Logout of all sessions | `false` |

**Example:**
```bash
primusdb auth logout
primusdb auth logout --all
```

### `primusdb auth token`

Manage authentication tokens.

**Usage:**
```
primusdb auth token [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--create` | Create a new token |
| `--revoke <TOKEN>` | Revoke a specific token |
| `--list` | List all tokens |

**Example:**
```bash
primusdb auth token --list
primusdb auth token --create
primusdb auth token --revoke tok_abc123
```

### `primusdb auth whoami`

Display current user identity.

**Usage:**
```
primusdb auth whoami [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--verbose` | Show detailed identity info | `false` |

**Example:**
```bash
primusdb auth whoami --verbose
```

---

## `primusdb user`

User management commands.

### `primusdb user create`

Create a new user.

**Usage:**
```
primusdb user create <USERNAME> [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-p, --password <PASSWORD>` | User password | — |
| `-r, --role <ROLE>` | Initial role assignment | — |
| `-e, --email <EMAIL>` | User email address | — |
| `--active <BOOL>` | Whether user is active | `true` |

**Example:**
```bash
primusdb user create alice --role analyst --email alice@example.com
```

### `primusdb user list`

List users.

**Usage:**
```
primusdb user list [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-r, --role <ROLE>` | Filter by role | — |
| `--all` | Show all users including disabled | `false` |

**Example:**
```bash
primusdb user list
primusdb user list --role admin
```

### `primusdb user disable`

Disable or re-enable a user.

**Usage:**
```
primusdb user disable <USERNAME> [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-r, --reason <TEXT>` | Reason for disabling |
| `--reenable` | Re-enable instead of disable |

**Example:**
```bash
primusdb user disable alice --reason "Account inactive"
primusdb user disable alice --reenable
```

### `primusdb user roles`

Manage user role assignments.

**Usage:**
```
primusdb user roles <USERNAME> [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-g, --grant <ROLE>` | Grant a role to the user |
| `-r, --revoke <ROLE>` | Revoke a role from the user |
| `-l, --list` | List current role assignments |

**Example:**
```bash
primusdb user roles alice --list
primusdb user roles alice --grant analyst
primusdb user roles alice --revoke viewer
```

---

## `primusdb role`

Role management commands.

### `primusdb role create`

Create a new role.

**Usage:**
```
primusdb role create <NAME> [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-d, --description <TEXT>` | Role description |
| `-i, --inherits <ROLE>` | Parent role to inherit permissions from |

**Example:**
```bash
primusdb role create analyst --description "Data analyst role"
primusdb role create senior-analyst --inherits analyst
```

### `primusdb role list`

List all roles.

**Usage:**
```
primusdb role list [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--permissions` | Show permissions for each role | `false` |

**Example:**
```bash
primusdb role list --permissions
```

### `primusdb role grant`

Grant a permission to a role.

**Usage:**
```
primusdb role grant <ROLE> <PERMISSION> [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-n, --namespace <NS>` | Namespace-scoped permission |

**Example:**
```bash
primusdb role grant analyst "read" --namespace tenant1
primusdb role grant admin "write"
```

### `primusdb role revoke`

Revoke a permission from a role.

**Usage:**
```
primusdb role revoke <ROLE> <PERMISSION>
```

**Example:**
```bash
primusdb role revoke analyst "write"
```

---

## `primusdb ai`

AI/ML model operations.

### `primusdb ai models`

List available AI/ML models.

**Usage:**
```
primusdb ai models [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-k, --kind <KIND>` | Filter by model kind | — |
| `--verbose` | Show detailed model info | `false` |

**Model kinds:** `regression`, `classification`, `clustering`, `forecasting`, `anomaly`

**Example:**
```bash
primusdb ai models --verbose
primusdb ai models --kind regression
```

### `primusdb ai train`

Train a new model.

**Usage:**
```
primusdb ai train <NAME> <DATASET> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `NAME` | Model name |
| `DATASET` | Training dataset (table name or file path) |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-t, --model-type <TYPE>` | Model type | `regression` |
| `-c, --target <COLUMN>` | Target column name | — |
| `--params <JSON>` | Hyperparameters as JSON | — |
| `--test-split <RATIO>` | Test split ratio | `0.2` |
| `--max-time <SECONDS>` | Maximum training time | `3600` |

**Model types:** `regression`, `classification`, `clustering`, `forecasting`, `anomaly`

**Examples:**
```bash
primusdb ai train sales-forecast sales_data --model-type forecasting --target revenue
primusdb ai train classifier user_data --model-type classification --target churned
primusdb ai train segmenter customer_data --model-type clustering --params '{"clusters": 5}'
```

### `primusdb ai predict`

Make predictions using a trained model.

**Usage:**
```
primusdb ai predict <MODEL> <INPUT> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `MODEL` | Trained model name |
| `INPUT` | Input data (JSON string or file path) |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--raw` | Return raw model output | `false` |
| `--top-k <N>` | Return top K results | `1` |

**Examples:**
```bash
primusdb ai predict sales-forecast '{"month": "2025-06", "region": "US"}'
primusdb ai predict classifier '{"age": 35, "spend": 1200}' --top-k 3
primusdb ai predict anomaly-detector '{"value": 9999}' --raw
```

### `primusdb ai analyze`

Analyze data patterns.

**Usage:**
```
primusdb ai analyze <TABLE> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `TABLE` | Table name to analyze |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --columns <COLS>` | Comma-separated column list | — |
| `-t, --analysis-type <TYPE>` | Analysis type | `summary` |

**Analysis types:** `summary`, `correlation`, `distribution`, `outliers`, `trend`

**Examples:**
```bash
primusdb ai analyze sales_data
primusdb ai analyze sales_data --columns region,revenue --analysis-type correlation
primusdb ai analyze user_data --analysis-type distribution
```

### `primusdb ai anomalies`

Detect anomalies in a dataset.

**Usage:**
```
primusdb ai anomalies <TABLE> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `TABLE` | Table name to scan |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --columns <COLS>` | Comma-separated column list | — |
| `-s, --sensitivity <FLOAT>` | Detection sensitivity | `0.05` |
| `-a, --algorithm <ALGO>` | Detection algorithm | `zscore` |

**Algorithms:** `zscore`, `isolation_forest`, `mad`, `iqr`

**Examples:**
```bash
primusdb ai anomalies metrics
primusdb ai anomalies sensor_data --sensitivity 0.01 --algorithm isolation_forest
```

---

## `primusdb vector`

Vector search and index management.

### `primusdb vector search`

Perform vector similarity search.

**Usage:**
```
primusdb vector search <INDEX> <VECTOR> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `INDEX` | Vector index name |
| `VECTOR` | Query vector as JSON array (e.g. `[0.1,0.2,0.3]`) |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-k, --k <N>` | Number of nearest neighbors | `10` |
| `-m, --metric <METRIC>` | Distance metric | `cosine` |
| `--include-vectors` | Include vectors in results | `false` |

**Metrics:** `cosine`, `euclidean`, `dot`, `manhattan`

**Examples:**
```bash
primusdb vector search my_index '[0.15, 0.22, 0.31, 0.42]'
primusdb vector search my_index '[0.1,0.2]' --k 5 --metric euclidean --include-vectors
```

### `primusdb vector index`

Create or rebuild a vector index.

**Usage:**
```
primusdb vector index <NAME> <TABLE> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `NAME` | Index name |
| `TABLE` | Source table name |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --column <COL>` | Embedding column name | `embedding` |
| `-d, --dimensions <N>` | Vector dimensions (auto-detected if omitted) | — |
| `-a, --algorithm <ALGO>` | Index algorithm | `hnsw` |
| `-m, --metric <METRIC>` | Distance metric | `cosine` |

**Algorithms:** `hnsw`, `ivf`, `flat`, `pq`

**Examples:**
```bash
primusdb vector index product_embeddings products
primusdb vector index user_vecs users --column user_embedding --dimensions 128 --algorithm hnsw
```

### `primusdb vector stats`

Show vector index statistics.

**Usage:**
```
primusdb vector stats <INDEX> [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--segments` | Show per-segment statistics | `false` |

**Example:**
```bash
primusdb vector stats my_index --segments
```

### `primusdb vector compact`

Compact and optimize vector indexes.

**Usage:**
```
primusdb vector compact <INDEX> [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--gc` | Run garbage collection | `false` |
| `--target-ratio <FLOAT>` | Target fill ratio after compaction | `0.8` |

**Example:**
```bash
primusdb vector compact my_index --gc --target-ratio 0.9
```

---

## `primusdb graph`

Graph traversal and management.

### `primusdb graph nodes`

Query graph nodes.

**Usage:**
```
primusdb graph nodes [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-l, --label <LABEL>` | Filter by node label | — |
| `-f, --filter <EXPR>` | Property filter expression | — |
| `--limit <N>` | Maximum nodes to return | `100` |
| `--counts` | Return counts only | `false` |

**Examples:**
```bash
primusdb graph nodes
primusdb graph nodes --label Person --limit 50
primusdb graph nodes --label Product --filter "price > 100" --counts
```

### `primusdb graph edges`

Query graph edges.

**Usage:**
```
primusdb graph edges [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --from <NODE>` | Filter by source node | — |
| `-t, --to <NODE>` | Filter by target node | — |
| `-l, --label <LABEL>` | Filter by edge label | — |
| `--limit <N>` | Maximum edges to return | `100` |

**Example:**
```bash
primusdb graph edges
primusdb graph edges --label PURCHASED --limit 50
primusdb graph edges --from user_123 --label FOLLOWS
```

### `primusdb graph query`

Execute a graph query.

**Usage:**
```
primusdb graph query <QUERY> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `QUERY` | Graph query string (space-separated) |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-l, --language <LANG>` | Query language | `cypher` |

**Languages:** `cypher`, `gremlin`, `sparql`

**Examples:**
```bash
primusdb graph query "MATCH (p:Person)-[:FRIENDS]->(f) RETURN p.name, f.name"
primusdb graph query "g.V().hasLabel('Person').out('FRIENDS')" --language gremlin
```

### `primusdb graph traverse`

Traverse the graph from a starting node.

**Usage:**
```
primusdb graph traverse <START> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `START` | Starting node ID |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --depth <N>` | Maximum traversal depth | `3` |
| `-l, --label <LABEL>` | Edge label to follow | — |
| `-s, --strategy <STRATEGY>` | Traversal strategy | `bfs` |

**Strategies:** `bfs` (breadth-first), `dfs` (depth-first)

**Example:**
```bash
primusdb graph traverse user_123 --depth 2 --label FRIENDS
primusdb graph traverse root_node --depth 5 --strategy dfs
```

---

## `primusdb cdc`

Change Data Capture (CDC) operations.

### `primusdb cdc status`

Show CDC status.

**Usage:**
```
primusdb cdc status [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--verbose` | Show detailed status | `false` |

**Example:**
```bash
primusdb cdc status --verbose
```

### `primusdb cdc stream`

Manage a CDC stream.

**Usage:**
```
primusdb cdc stream <NAME> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `NAME` | Stream name |

**Options:**

| Option | Description |
|--------|-------------|
| `-t, --table <TABLE>` | Table to capture changes from |
| `--create` | Create the stream |
| `--stop` | Stop the stream |
| `--delete` | Delete the stream |

**Examples:**
```bash
primusdb cdc stream orders_stream --create --table orders
primusdb cdc stream orders_stream --stop
primusdb cdc stream orders_stream --delete
```

### `primusdb cdc subscribe`

Subscribe to a CDC stream.

**Usage:**
```
primusdb cdc subscribe <STREAM> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `STREAM` | Stream name to subscribe to |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--from-start` | Read from the beginning of the stream | `false` |
| `--offset <OFFSET>` | Start from a specific offset | — |
| `--format <FORMAT>` | Output format for events | `json` |

**Formats:** `json`, `avro`, `raw`

**Example:**
```bash
primusdb cdc subscribe orders_stream --from-start --format json
primusdb cdc subscribe orders_stream --offset "2025-01-01T00:00:00Z"
```

### `primusdb cdc offsets`

Show CDC offset information.

**Usage:**
```
primusdb cdc offsets <STREAM> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `STREAM` | Stream name |

**Options:**

| Option | Description |
|--------|-------------|
| `--partitions` | Show per-partition offsets |
| `--set <OFFSET>` | Set offset to a specific position |

**Example:**
```bash
primusdb cdc offsets orders_stream --partitions
primusdb cdc offsets orders_stream --set "2025-01-15T00:00:00Z"
```

---

## `primusdb explain`

Explain a query plan without executing it.

**Usage:**
```
primusdb explain <QUERY>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `QUERY` | Query string (space-separated) |

**Example:**
```bash
primusdb explain "SELECT * FROM users WHERE age > 30"
primusdb explain "SELECT u.name, COUNT(o.id) FROM users u JOIN orders o ON u.id = o.user_id GROUP BY u.name"
```

---

## `primusdb bench`

Benchmark and performance testing.

### `primusdb bench run`

Run a benchmark.

**Usage:**
```
primusdb bench run <NAME> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `NAME` | Benchmark profile name |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --connections <N>` | Number of concurrent connections | `10` |
| `-d, --duration <SECONDS>` | Benchmark duration | `30` |
| `-r, --rate <RATE>` | Target request rate (ops/sec) | — |
| `--read-write-mix <%>` | Read/write mix percentage | `50` |
| `-o, --output <PATH>` | Output results to file | — |

**Examples:**
```bash
primusdb bench run default
primusdb bench run write-heavy --connections 50 --duration 60 --read-write-mix 10
primusdb bench run read-only --rate 10000 --output results.json
```

### `primusdb bench list`

List available benchmark profiles.

**Usage:**
```
primusdb bench list [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--verbose` | Show detailed profile info | `false` |

**Example:**
```bash
primusdb bench list --verbose
```

### `primusdb bench report`

Generate a benchmark report.

**Usage:**
```
primusdb bench report <PATH> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to benchmark results file |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --format <FMT>` | Report format | `text` |
| `--compare <PATH>` | Compare with another results file | — |

**Formats:** `text`, `json`, `html`, `markdown`

**Example:**
```bash
primusdb bench report results.json --format html
primusdb bench report results.json --compare baseline.json --format markdown
```

---

## `primusdb doctor`

Run diagnostic checks on the database.

**Usage:**
```
primusdb doctor [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--aggressive` | Run aggressive diagnostics | `false` |
| `--report <PATH>` | Write diagnostic report to file | — |

**Examples:**
```bash
# Quick health check
primusdb doctor

# Comprehensive diagnostics
primusdb doctor --aggressive

# Save report to file
primusdb doctor --aggressive --report /tmp/diagnostic-report.txt
```

---

## `primusdb discover`

Discover PrimusDB nodes on the local network.

The discovery system probes running PrimusDB instances by scanning ports and checking health endpoints.

**Usage:**
```
primusdb discover [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-b, --broadcast <ADDR>` | Broadcast address | `255.255.255.255` |
| `-p, --port <PORT>` | Discovery port | `7890` |
| `--timeout <SECONDS>` | Discovery timeout | `5` |

**Discovery ports scanned:** 8080, 8081, 8082, 8083, 9090, 9091, 9092, 9093

**Example:**
```bash
primusdb discover
primusdb discover --port 9090 --timeout 10
```

**Output format:**
```
Found 2 PrimusDB instance(s):

Endpoint                   Node ID              Version    Status
---------------------------------------------------------------------------
http://127.0.0.1:8080      node_12345           1.3.2-alpha healthy
http://127.0.0.1:8081      node_67890           1.3.2-alpha healthy
```

---

## `primusdb completion`

Generate shell completion scripts.

**Usage:**
```
primusdb completion <SHELL>
```

**Supported shells:** `bash`, `zsh`, `fish`, `powershell`

**Examples:**
```bash
primusdb completion bash > /etc/bash_completion.d/primusdb
primusdb completion zsh > /usr/local/share/zsh/site-functions/_primusdb
primusdb completion fish > ~/.config/fish/completions/primusdb.fish
primusdb completion powershell > _primusdb.ps1
```

---

## `primusdb version`

Display version information.

**Usage:**
```
primusdb version [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--verbose` | Show detailed version info | `false` |

**Examples:**
```bash
primusdb version
# → 1.3.2-alpha

primusdb version --verbose
# → PrimusDB v1.3.2-alpha (GPL-3.0)
# → Build: primusdb
```

---

## `primusdb protocol`

Manage protocol layer and peer connections.

### `primusdb protocol health`

Check protocol layer health.

**Usage:**
```
primusdb protocol health [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-m, --module <NAME>` | Specific protocol module to check |

**Example:**
```bash
primusdb protocol health
primusdb protocol health --module messaging
```

### `primusdb protocol status`

Show protocol status and capabilities.

**Usage:**
```
primusdb protocol status [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--versions` | Show protocol version info | `false` |
| `--connections` | Show active connections | `false` |

**Example:**
```bash
primusdb protocol status --versions --connections
```

### `primusdb protocol peers`

List protocol peers.

**Usage:**
```
primusdb protocol peers [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--state <STATE>` | Filter by peer state |
| `--verbose` | Show detailed peer info |

**Example:**
```bash
primusdb protocol peers --state connected --verbose
```

### `primusdb protocol metrics`

Show protocol metrics.

**Usage:**
```
primusdb protocol metrics <PROTOCOL> [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--raw` | Show raw metrics data | `false` |

**Example:**
```bash
primusdb protocol metrics raft --raw
```

---

## `primusdb certs`

Certificate management for TLS and mTLS. Generate Certificate Authorities,
signed certificates, and self-signed certificates.

### `primusdb certs create-ca`

Create a new Certificate Authority.

**Usage:**
```
primusdb certs create-ca [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--out-dir <DIR>` | Output directory | `.` |
| `--name <NAME>` | CA common name | `PrimusDB CA` |
| `--validity-days <DAYS>` | Certificate validity | `3650` |

**Example:**
```bash
primusdb certs create-ca --out-dir ./ca --name "MyOrg CA"
```

### `primusdb certs create-cert`

Create a certificate signed by a CA (for servers or clients).

**Usage:**
```
primusdb certs create-cert [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--ca-dir <DIR>` | CA directory (must contain `ca.crt` and `ca.key`) | — |
| `--out-dir <DIR>` | Output directory | `.` |
| `--name <NAME>` | Certificate common name | `PrimusDB Node` |
| `--hosts <HOSTS>` | Subject Alternative Names (SANs) | `localhost 127.0.0.1` |
| `--validity-days <DAYS>` | Certificate validity | `365` |
| `--server` | Enable ServerAuth extended key usage | `true` |
| `--client` | Enable ClientAuth extended key usage | `false` |

**Examples:**
```bash
# Create a server certificate
primusdb certs create-cert --ca-dir ./ca --out-dir ./tls --hosts example.com --server

# Create a client certificate for mTLS
primusdb certs create-cert --ca-dir ./ca --out-dir ./node1 --name "node-1" --client
```

### `primusdb certs create-selfsigned`

Create a self-signed certificate (for development/testing).

**Usage:**
```
primusdb certs create-selfsigned [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--out-dir <DIR>` | Output directory | `.` |
| `--name <NAME>` | Certificate common name | `PrimusDB Self-Signed` |
| `--hosts <HOSTS>` | Subject Alternative Names (SANs) | `localhost 127.0.0.1` |
| `--validity-days <DAYS>` | Certificate validity | `365` |

**Example:**
```bash
primusdb certs create-selfsigned --out-dir ./tls --hosts localhost 127.0.0.1
```
