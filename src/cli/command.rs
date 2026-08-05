use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Global options shared across all subcommands.
#[derive(Parser, Debug, Clone)]
pub struct GlobalArgs {
    /// PrimusDB server base URL
    #[arg(long, global = true, default_value = "http://localhost:8080")]
    pub server_url: String,
    /// Output format (table, json, csv, yaml, plain)
    #[arg(long, global = true, default_value = "table")]
    pub format: String,
    /// Request timeout in milliseconds
    #[arg(long, global = true, default_value = "30000")]
    pub timeout: u64,
    /// Write output to a file instead of stdout
    #[arg(long, global = true)]
    pub output: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Top-level CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "primusdb")]
#[command(about = "PrimusDB — unified database CLI")]
#[command(version)]
#[command(arg_required_else_help = true)]
#[command(propagate_version = true)]
/// Top-level CLI argument structure for the `primusdb` binary.
pub struct Cli {
    /// Global options shared by every subcommand (server URL, format, timeout, output).
    #[command(flatten)]
    pub global: GlobalArgs,
    /// The subcommand to dispatch to.
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level command enumeration.
#[derive(Subcommand)]
pub enum Commands {
    /// Manage the PrimusDB server lifecycle (start, stop, restart, ...)
    #[command(subcommand)]
    Server(ServerSubcommands),
    /// Connect to a running PrimusDB instance interactively
    Connect {
        /// Server URL (defaults to http://localhost:8080)
        #[arg(short, long)]
        server: Option<String>,
        /// Connection timeout in seconds
        #[arg(long, default_value = "10")]
        timeout: u64,
    },
    /// Enter the interactive shell against a running PrimusDB instance
    Shell {
        /// Server URL (defaults to http://localhost:8080)
        #[arg(short, long)]
        server: Option<String>,
        /// Connection timeout in seconds
        #[arg(long, default_value = "10")]
        timeout: u64,
    },
    /// Show server health status (alias for `server health`)
    Health,
    /// Show server status information (alias for `server status`)
    Status,
    /// Manage running instances (list, discover, inspect, connect, stop, logs)
    #[command(subcommand)]
    Instance(InstanceSubcommands),
    /// Execute a raw query
    Query {
        /// Query text (whitespace-joined)
        #[arg(required = true)]
        query: Vec<String>,
        /// Database to run the query against
        #[arg(short, long)]
        database: Option<String>,
    },
    /// Execute a SQL query
    Sql {
        /// SQL text (whitespace-joined)
        #[arg(required = true)]
        sql: Vec<String>,
        /// Database to run the query against
        #[arg(short, long)]
        database: Option<String>,
    },
    /// Manage databases (list, create, drop, describe, use)
    #[command(subcommand)]
    Db(DbSubcommands),
    /// Manage storage engines
    #[command(subcommand)]
    Engine(EngineSubcommands),
    /// Manage namespaces
    #[command(subcommand)]
    Namespace(NamespaceSubcommands),
    /// Manage configuration (init, validate, show)
    #[command(subcommand)]
    Config(ConfigSubcommands),
    /// Manage cluster operations
    #[command(subcommand)]
    Cluster(ClusterSubcommands),
    /// Manage protocol layer and peer connections
    #[command(subcommand)]
    Protocol(ProtocolSubcommands),
    /// Create and manage backups
    #[command(subcommand)]
    Backup(BackupSubcommands),
    /// Restore database from a backup
    Restore {
        /// Backup archive path or backup ID
        #[arg(required = true)]
        source: PathBuf,
        /// Restore only this database
        #[arg(short, long)]
        database: Option<String>,
        /// Skip the overwrite confirmation prompt
        #[arg(long)]
        force: bool,
    },
    /// Show database and system metrics
    Metrics {
        /// Filter metrics by name or prefix
        #[arg(short, long)]
        filter: Option<String>,
        /// Keep printing metrics periodically
        #[arg(long)]
        watch: bool,
        /// Poll interval in seconds when watching
        #[arg(long, default_value = "2")]
        interval: u64,
    },
    /// Authentication commands (login, logout, token, whoami)
    #[command(subcommand)]
    Auth(AuthSubcommands),
    /// User management commands
    #[command(subcommand)]
    User(UserSubcommands),
    /// Role management commands
    #[command(subcommand)]
    Role(RoleSubcommands),
    /// AI/ML model operations
    #[command(subcommand)]
    Ai(AiSubcommands),
    /// Vector search and index management
    #[command(subcommand)]
    Vector(VectorSubcommands),
    /// Graph traversal and management
    #[command(subcommand)]
    Graph(GraphSubcommands),
    /// Change Data Capture (CDC) operations
    #[command(subcommand)]
    Cdc(CdcSubcommands),
    /// Explain a query plan without executing it
    Explain {
        #[arg(required = true)]
        query: Vec<String>,
    },
    /// Benchmark and performance testing
    #[command(subcommand)]
    Bench(BenchSubcommands),
    /// Migrate data from external databases into PrimusDB
    #[command(subcommand)]
    Migrate(MigrateSubcommands),
    /// Run diagnostic checks on the database
    Doctor {
        /// Run slower, deeper checks (metrics, disk space)
        #[arg(long)]
        aggressive: bool,
        /// Write a text report to this file
        #[arg(long)]
        report: Option<PathBuf>,
        /// Validate configuration file and settings
        #[arg(long)]
        config: bool,
        /// Check system database health and migrations
        #[arg(long)]
        system_db: bool,
        /// Check notebook storage and workspace integrity
        #[arg(long)]
        notebooks: bool,
        /// Check RAG workspace and vector collections
        #[arg(long)]
        rag: bool,
    },
    /// Discover PrimusDB nodes on the network
    Discover {
        /// Broadcast address to probe
        #[arg(short, long, default_value = "255.255.255.255")]
        broadcast: String,
        /// Port to probe
        #[arg(short, long, default_value = "7890")]
        port: u16,
        /// Probe timeout in seconds
        #[arg(long, default_value = "5")]
        timeout: u64,
    },
    /// Resource Governor — execution governance and resource management
    #[command(subcommand)]
    Governor(GovernorSubcommands),
    /// Time series operations (query, aggregate, downsample, retention, list metrics)
    #[command(subcommand)]
    Ts(TimeSeriesSubcommands),
    /// Database integrity: genesis, records, checkpoints, quarantine, ledger
    #[command(subcommand)]
    Integrity(IntegritySubcommands),
    /// Unified search across all storage engines (full-text + vector)
    #[command(subcommand)]
    Search(SearchSubcommands),
    /// Certificate management (create CA, sign certs, create self-signed)
    #[command(subcommand)]
    Certs(crate::certs::CertsCommands),
    /// Generate shell completion scripts
    Completion {
        /// Target shell: bash, zsh, fish, powershell, elvish
        #[arg(required = true)]
        shell: String,
    },
    /// Display version information
    Version {
        /// Show full version, license and build details
        #[arg(long)]
        verbose: bool,
    },
}

// ---------------------------------------------------------------------------
// server
// ---------------------------------------------------------------------------

/// Server lifecycle subcommands (start, stop, restart, status, health, config).
#[derive(Subcommand)]
pub enum ServerSubcommands {
    /// Start the PrimusDB server daemon
    Start {
        /// Configuration file to load (future: primusdb.toml)
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
        /// Bind address, e.g. `127.0.0.1:8080`
        #[arg(short, long)]
        bind: Option<String>,
        /// Data directory for storage
        #[arg(short, long)]
        data_dir: Option<PathBuf>,
        /// Run as a daemon (currently unused)
        #[arg(long)]
        daemon: bool,
        /// Log verbosity (trace, debug, info, warn, error)
        #[arg(long, default_value = "info")]
        log_level: String,
        // Federation flags
        /// Federation identifier for cross-cluster discovery
        #[arg(long, default_value = "default")]
        federation_id: String,
        /// Cluster identifier to join
        #[arg(long)]
        cluster_id: Option<String>,
        /// Region label for the node
        #[arg(long)]
        region: Option<String>,
        /// Seed addresses for federation discovery
        #[arg(long)]
        federation_discovery: Vec<String>,
        // TLS flags
        /// Enable TLS on the HTTP listener
        #[arg(long)]
        tls_enabled: bool,
        /// Path to the TLS certificate
        #[arg(long)]
        tls_cert: Option<String>,
        /// Path to the TLS private key
        #[arg(long)]
        tls_key: Option<String>,
    },
    /// Stop the running server
    Stop {
        /// Seconds to wait for a graceful stop
        #[arg(long, default_value = "30")]
        timeout: u64,
        /// Kill the process instead of stopping gracefully
        #[arg(long)]
        force: bool,
    },
    /// Restart the server
    Restart {
        /// Configuration file to load
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
        /// Seconds to wait for the restart
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
    /// Show server status
    Status {
        /// Include build and license details
        #[arg(long)]
        verbose: bool,
    },
    /// Check server health
    Health {
        /// Run a deep health check
        #[arg(long)]
        deep: bool,
    },
    /// View or modify configuration
    Config {
        /// Print a single configuration key
        #[arg(short, long)]
        get: Option<String>,
        /// Set configuration keys (key=value)
        #[arg(short, long)]
        set: Option<Vec<String>>,
        /// List all configuration keys
        #[arg(short, long)]
        list: bool,
        /// Configuration file to read
        #[arg(short, long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// config
// ---------------------------------------------------------------------------

/// Configuration subcommands (init, validate, show).
#[derive(Subcommand)]
pub enum ConfigSubcommands {
    /// Generate a default PrimusDB configuration file
    Init {
        /// Destination file (defaults to primusdb.toml)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
        /// Overwrite existing file
        #[arg(long)]
        force: bool,
        /// Config profile: local, single-node, cluster-node, secure, dev
        #[arg(short, long, default_value = "local")]
        profile: String,
    },
    /// Validate a configuration file
    Validate {
        /// Configuration file to check (defaults to primusdb.toml)
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },
    /// Display the current configuration
    Show {
        /// Configuration file to display (defaults to primusdb.toml)
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// db
// ---------------------------------------------------------------------------

/// Database management subcommands (list, create, drop, describe, use).
#[derive(Subcommand)]
pub enum DbSubcommands {
    /// List all databases
    List {
        /// Include system and hidden databases
        #[arg(long)]
        all: bool,
        /// Filter by storage engine
        #[arg(long)]
        engine: Option<String>,
    },
    /// Create a new database
    Create {
        /// Database name
        #[arg(required = true)]
        name: String,
        /// Initial storage engine (document, relational, keyvalue, ...)
        #[arg(short, long, default_value = "document")]
        engine: String,
        /// Namespace to create the database in
        #[arg(short, long)]
        namespace: Option<String>,
    },
    /// Drop a database
    Drop {
        /// Database name
        #[arg(required = true)]
        name: String,
        /// Skip the confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Describe a database
    Describe {
        /// Database name
        #[arg(required = true)]
        name: String,
        /// Also show the schema
        #[arg(long)]
        schema: bool,
    },
    /// Switch active database
    #[command(name = "use")]
    Use {
        /// Database to activate
        #[arg(required = true)]
        name: String,
    },
}

// ---------------------------------------------------------------------------
// engine
// ---------------------------------------------------------------------------

/// Storage engine management subcommands (list, status, inspect, metrics, add, remove, upgrade).
#[derive(Subcommand)]
pub enum EngineSubcommands {
    /// List registered storage engines
    List {
        /// Show descriptions alongside engine names
        #[arg(long)]
        verbose: bool,
    },
    /// Show engine status
    Status {
        /// Engine name
        #[arg(required = true)]
        name: String,
    },
    /// Inspect internal engine state
    Inspect {
        /// Engine name
        #[arg(required = true)]
        name: String,
        /// Optional component to inspect
        #[arg(short, long)]
        component: Option<String>,
        /// Print the raw JSON response instead of a table
        #[arg(long)]
        raw: bool,
    },
    /// Show engine metrics
    Metrics {
        /// Engine name
        #[arg(required = true)]
        name: String,
        /// Filter metrics by name or prefix
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Add/enable a storage engine on the server
    Add {
        /// Engine type (columnar, vector, document, relational, keyvalue)
        engine_type: String,
        /// Server URL
        #[arg(short, long, default_value = "http://localhost:8080")]
        server: String,
        /// Enable now without server restart
        #[arg(long)]
        hot: bool,
    },
    /// Remove/disable a storage engine
    Remove {
        /// Engine type to remove
        engine_type: String,
        /// Server URL
        #[arg(short, long, default_value = "http://localhost:8080")]
        server: String,
        /// Force removal of engine data
        #[arg(long)]
        force: bool,
    },
    /// Upgrade an engine to a new version or configuration
    Upgrade {
        /// Engine type to upgrade
        engine_type: String,
        /// Server URL
        #[arg(short, long, default_value = "http://localhost:8080")]
        server: String,
    },
}

// ---------------------------------------------------------------------------
// namespace
// ---------------------------------------------------------------------------

/// Namespace management subcommands (list, create, drop, describe, policy).
#[derive(Subcommand)]
pub enum NamespaceSubcommands {
    /// List namespaces
    List {
        /// Only list children of this parent namespace
        #[arg(short, long)]
        parent: Option<String>,
        /// Show full namespace paths
        #[arg(long)]
        full_paths: bool,
    },
    /// Create a namespace
    Create {
        /// Namespace path, e.g. `org/team`
        #[arg(required = true)]
        path: String,
        /// Human-readable description
        #[arg(short, long)]
        description: Option<String>,
        /// Parent namespace path
        #[arg(short, long)]
        parent: Option<String>,
        /// Quota for the namespace
        #[arg(long)]
        quota: Option<String>,
    },
    /// Drop a namespace
    Drop {
        /// Namespace path
        #[arg(required = true)]
        path: String,
        /// Drop child namespaces recursively
        #[arg(short, long)]
        recursive: bool,
        /// Skip the confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Describe a namespace
    Describe {
        /// Namespace path
        #[arg(required = true)]
        path: String,
        /// Also list contained resources
        #[arg(long)]
        resources: bool,
    },
    /// View or set namespace policy
    Policy {
        /// Namespace path
        #[arg(required = true)]
        path: String,
        /// Set policy entries (key=value)
        #[arg(short, long)]
        set: Option<Vec<String>>,
        /// Remove a policy entry
        #[arg(short, long)]
        unset: Option<String>,
        /// List current policy
        #[arg(short, long)]
        list: bool,
    },
}

// ---------------------------------------------------------------------------
// instance
// ---------------------------------------------------------------------------

/// Instance management subcommands (list, discover, inspect, connect, stop, logs).
#[derive(Subcommand)]
pub enum InstanceSubcommands {
    /// List all local PrimusDB instances
    List {
        /// Include stopped and remote instances
        #[arg(long)]
        all: bool,
        /// Output format override
        #[arg(short, long)]
        format: Option<String>,
    },
    /// Discover PrimusDB instances on the network
    Discover {
        /// Host to scan
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,
        /// First port to probe
        #[arg(short, long, default_value = "8080")]
        start_port: u16,
        /// Number of consecutive ports to probe
        #[arg(short, long, default_value = "5")]
        max_ports: u16,
        /// Per-probe timeout in seconds
        #[arg(long, default_value = "5")]
        timeout: u64,
    },
    /// Inspect a specific instance
    Inspect {
        /// Instance endpoint, e.g. `http://host:port`
        #[arg(required = true)]
        endpoint: String,
        /// Include detailed health/status fields
        #[arg(long)]
        verbose: bool,
    },
    /// Connect to an instance interactively
    Connect {
        /// Instance endpoint, e.g. `http://host:port`
        #[arg(required = true)]
        endpoint: String,
        /// Connection timeout in seconds
        #[arg(long, default_value = "10")]
        timeout: u64,
    },
    /// Stop a running instance
    Stop {
        /// Instance endpoint, e.g. `http://host:port`
        #[arg(required = true)]
        endpoint: String,
        /// Force-stop instead of a graceful stop
        #[arg(long)]
        force: bool,
    },
    /// Show logs from an instance
    Logs {
        /// Instance endpoint, e.g. `http://host:port`
        #[arg(required = true)]
        endpoint: String,
        /// Number of log lines to show
        #[arg(short, long, default_value = "50")]
        lines: u32,
        /// Keep streaming new log lines
        #[arg(long)]
        follow: bool,
    },
}

// ---------------------------------------------------------------------------
// cluster
// ---------------------------------------------------------------------------

/// Cluster operation subcommands (status, nodes, join, leave, rebalance, failover, health, sync, config, inspect, topology).
#[derive(Subcommand)]
pub enum ClusterSubcommands {
    /// Show cluster status
    Status {
        /// Include verbose status fields
        #[arg(long)]
        verbose: bool,
        /// Keep printing status periodically
        #[arg(long)]
        watch: bool,
        /// Poll interval in seconds when watching
        #[arg(long, default_value = "2")]
        interval: u64,
    },
    /// List cluster nodes
    Nodes {
        /// Filter by node role
        #[arg(long)]
        role: Option<String>,
        /// Filter by node state
        #[arg(long)]
        state: Option<String>,
    },
    /// Join an existing cluster
    #[command(name = "join")]
    Join {
        /// Peer host (and optional port) to register
        #[arg(required = true)]
        peer: String,
        /// Node ID to register
        #[arg(short, long)]
        node_id: Option<String>,
        /// Node ID to register (alternative to node_id)
        #[arg(long)]
        node: Option<String>,
        /// Seed server address (if provided, use as registration target)
        #[arg(short = 's', long)]
        seed: Option<String>,
        /// Registration timeout in seconds
        #[arg(long, default_value = "30")]
        timeout: u64,
        /// Use TLS for the peer connection
        #[arg(long)]
        tls: bool,
    },
    /// Leave the current cluster
    #[command(name = "leave")]
    Leave {
        /// Node to remove from the cluster
        #[arg(required = true)]
        node: String,
        /// Drain data from the node before removal
        #[arg(long)]
        drain: bool,
        /// Force removal
        #[arg(long)]
        force: bool,
    },
    /// Trigger cluster rebalance
    Rebalance {
        /// Target a specific node
        #[arg(short, long)]
        node: Option<String>,
        /// Rebalance strategy (size, ...)
        #[arg(long, default_value = "size")]
        strategy: String,
        /// Number of concurrent moves
        #[arg(long, default_value = "2")]
        concurrency: u32,
    },
    /// Trigger manual failover
    Failover {
        /// Node to fail over
        #[arg(required = true)]
        node: String,
        /// Preferred replacement node
        #[arg(short, long)]
        target: Option<String>,
        /// Force the failover
        #[arg(long)]
        force: bool,
    },
    /// Check cluster health
    Health {
        /// Run a detailed diagnostic
        #[arg(long)]
        diagnostic: bool,
        /// Latency threshold in milliseconds
        #[arg(long, default_value = "100")]
        threshold_ms: u64,
    },
    /// Trigger cluster synchronization
    Sync {
        /// Perform a full sync instead of incremental
        #[arg(long)]
        full: bool,
        /// Sync timeout in seconds
        #[arg(short, long, default_value = "60")]
        timeout: u64,
    },
    /// View or modify cluster configuration
    Config {
        /// Print a single configuration key
        #[arg(short, long)]
        get: Option<String>,
        /// Set configuration keys (key=value)
        #[arg(short, long)]
        set: Option<Vec<String>>,
        /// List all configuration keys
        #[arg(short, long)]
        list: bool,
    },
    /// Inspect a specific cluster node
    Inspect {
        /// Node identifier
        #[arg(required = true)]
        node: String,
        /// Include verbose details
        #[arg(long)]
        verbose: bool,
    },
    /// Show cluster topology
    Topology {
        /// Output format (table or json)
        #[arg(long, default_value = "table")]
        format: String,
    },
}

// ---------------------------------------------------------------------------
// protocol
// ---------------------------------------------------------------------------

/// Protocol-layer subcommands (health, status, peers, metrics).
#[derive(Subcommand)]
pub enum ProtocolSubcommands {
    /// Check protocol layer health
    Health {
        /// Protocol module to check
        #[arg(short, long)]
        module: Option<String>,
    },
    /// Show protocol status and capabilities
    Status {
        /// Include protocol version information
        #[arg(long)]
        versions: bool,
        /// Include connection counts
        #[arg(long)]
        connections: bool,
    },
    /// List protocol peers
    Peers {
        /// Filter by peer state
        #[arg(long)]
        state: Option<String>,
        /// Show verbose peer details
        #[arg(long)]
        verbose: bool,
    },
    /// Show protocol metrics
    Metrics {
        /// Protocol name
        #[arg(required = true)]
        protocol: String,
        /// Print the raw metrics payload
        #[arg(long)]
        raw: bool,
    },
}

// ---------------------------------------------------------------------------
// backup
// ---------------------------------------------------------------------------

/// Backup management subcommands (create, list, inspect, restore, verify, delete, export-manifest).
#[derive(Subcommand)]
pub enum BackupSubcommands {
    /// Create a new backup
    Create {
        /// Destination archive path
        #[arg(long)]
        destination: Option<PathBuf>,
        /// Backup name/ID
        #[arg(short, long)]
        name: Option<String>,
        /// Comma-separated databases or data paths to back up
        #[arg(short, long)]
        databases: Option<String>,
        /// Compression (gzip, bzip2, xz, none)
        #[arg(long, default_value = "zstd")]
        compression: String,
        /// Encrypt the backup archive
        #[arg(long)]
        encrypt: bool,
        /// Optional human-readable description
        #[arg(long)]
        description: Option<String>,
    },
    /// List available backups
    List {
        /// Directory to scan (defaults to ./backups)
        #[arg(short, long)]
        directory: Option<PathBuf>,
        /// Show verbose details
        #[arg(long)]
        verbose: bool,
    },
    /// Inspect a backup archive
    Inspect {
        /// Archive path or backup ID
        #[arg(required = true)]
        path: PathBuf,
        /// List the archive contents
        #[arg(long)]
        contents: bool,
        /// Include the metadata sidecar file
        #[arg(long)]
        metadata: bool,
    },
    /// Restore from a backup
    Restore {
        /// Archive path or backup ID
        #[arg(required = true)]
        source: PathBuf,
        /// Restore only this database
        #[arg(short, long)]
        database: Option<String>,
        /// Skip the overwrite confirmation prompt
        #[arg(long)]
        force: bool,
        /// Point-in-time restore timestamp
        #[arg(long)]
        pitr: Option<String>,
    },
    /// Verify backup integrity
    Verify {
        /// Archive path or backup ID
        #[arg(required = true)]
        path: PathBuf,
        /// Run a full (slower) verification
        #[arg(long)]
        full: bool,
        /// Compare against the recorded checksum
        #[arg(long)]
        compare: bool,
    },
    /// Delete a backup
    Delete {
        /// Backup ID or path
        #[arg(required = true)]
        name: String,
        /// Skip the confirmation prompt
        #[arg(long)]
        force: bool,
    },
    /// Export backup manifest metadata
    ExportManifest {
        /// Backup ID or path
        #[arg(required = true)]
        name: String,
        /// Write the manifest to this file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// auth
// ---------------------------------------------------------------------------

/// Authentication subcommands (login, logout, token, whoami).
#[derive(Subcommand)]
pub enum AuthSubcommands {
    /// Authenticate and obtain a session token
    Login {
        /// Username to authenticate as
        #[arg(required = true)]
        username: String,
        /// Password (prompted if omitted)
        #[arg(short, long)]
        password: Option<String>,
        /// Authentication realm
        #[arg(short, long, default_value = "default")]
        realm: String,
        /// Token lifetime in seconds
        #[arg(long, default_value = "86400")]
        ttl: u64,
    },
    /// Invalidate the current session
    Logout {
        /// Invalidate all sessions instead of just the current one
        #[arg(long)]
        all: bool,
    },
    /// Manage authentication tokens
    Token {
        /// Create a new token
        #[arg(long)]
        create: bool,
        /// Revoke the token with this ID
        #[arg(long)]
        revoke: Option<String>,
        /// List existing tokens
        #[arg(long)]
        list: bool,
    },
    /// Display current user identity
    Whoami {
        /// Show verbose identity details
        #[arg(long)]
        verbose: bool,
    },
}

// ---------------------------------------------------------------------------
// user
// ---------------------------------------------------------------------------

/// User management subcommands (create, list, disable, roles).
#[derive(Subcommand)]
pub enum UserSubcommands {
    /// Create a new user
    Create {
        /// Username
        #[arg(required = true)]
        username: String,
        /// Password (defaults to a placeholder when omitted)
        #[arg(short, long)]
        password: Option<String>,
        /// Initial role to assign
        #[arg(short, long)]
        role: Option<String>,
        /// Contact email
        #[arg(short, long)]
        email: Option<String>,
        /// Whether the user starts active
        #[arg(long, default_value = "true")]
        active: bool,
    },
    /// List users
    List {
        /// Filter by role
        #[arg(short, long)]
        role: Option<String>,
        /// Include disabled users
        #[arg(long)]
        all: bool,
    },
    /// Disable or re-enable a user
    Disable {
        /// Username
        #[arg(required = true)]
        username: String,
        /// Disable reason
        #[arg(short, long)]
        reason: Option<String>,
        /// Re-enable instead of disable
        #[arg(long)]
        reenable: bool,
    },
    /// Manage user role assignments
    Roles {
        /// Username
        #[arg(required = true)]
        username: String,
        /// Role to grant
        #[arg(short, long)]
        grant: Option<String>,
        /// Role to revoke
        #[arg(short, long)]
        revoke: Option<String>,
        /// List the user's current roles
        #[arg(short, long)]
        list: bool,
    },
}

// ---------------------------------------------------------------------------
// role
// ---------------------------------------------------------------------------

/// Role management subcommands (create, list, grant, revoke).
#[derive(Subcommand)]
pub enum RoleSubcommands {
    /// Create a new role
    Create {
        /// Role name
        #[arg(required = true)]
        name: String,
        /// Human-readable description
        #[arg(short, long)]
        description: Option<String>,
        /// Role to inherit permissions from
        #[arg(short, long)]
        inherits: Option<String>,
    },
    /// List all roles
    List {
        /// Include the permissions of each role
        #[arg(long)]
        permissions: bool,
    },
    /// Grant a permission to a role
    Grant {
        /// Role name
        #[arg(required = true)]
        role: String,
        /// Permission to grant
        #[arg(required = true)]
        permission: String,
        /// Scope the grant to a namespace
        #[arg(short, long)]
        namespace: Option<String>,
    },
    /// Revoke a permission from a role
    Revoke {
        /// Role name
        #[arg(required = true)]
        role: String,
        /// Permission to revoke
        #[arg(required = true)]
        permission: String,
    },
}

// ---------------------------------------------------------------------------
// ai
// ---------------------------------------------------------------------------

/// AI/ML model subcommands (models, train, predict, analyze, anomalies).
#[derive(Subcommand)]
pub enum AiSubcommands {
    /// List available AI/ML models
    Models {
        /// Filter by model kind
        #[arg(short, long)]
        kind: Option<String>,
        /// Show verbose model details
        #[arg(long)]
        verbose: bool,
    },
    /// Train a new model
    Train {
        /// Model name
        #[arg(required = true)]
        name: String,
        /// Dataset (table) to train on
        #[arg(required = true)]
        dataset: String,
        /// Model type (regression, ...)
        #[arg(short, long, default_value = "regression")]
        model_type: String,
        /// Target column for supervised training
        #[arg(short, long)]
        target: Option<String>,
        /// Extra training parameters (JSON)
        #[arg(long)]
        params: Option<String>,
        /// Fraction of data held out for testing
        #[arg(long, default_value = "0.2")]
        test_split: f64,
        /// Maximum training time in seconds
        #[arg(long, default_value = "3600")]
        max_time: u64,
    },
    /// Make predictions using a trained model
    Predict {
        /// Model name
        #[arg(required = true)]
        model: String,
        /// Input data (table/endpoint reference)
        #[arg(required = true)]
        input: String,
        /// Print the raw prediction payload
        #[arg(long)]
        raw: bool,
        /// Number of top predictions to return
        #[arg(long, default_value = "1")]
        top_k: u32,
    },
    /// Analyze data patterns
    Analyze {
        /// Table to analyze
        #[arg(required = true)]
        table: String,
        /// Comma-separated columns to analyze
        #[arg(short, long)]
        columns: Option<String>,
        /// Analysis type (summary, ...)
        #[arg(short, long, default_value = "summary")]
        analysis_type: String,
    },
    /// Detect anomalies in a dataset
    Anomalies {
        /// Table to scan
        #[arg(required = true)]
        table: String,
        /// Comma-separated columns to scan
        #[arg(short, long)]
        columns: Option<String>,
        /// Detection sensitivity threshold
        #[arg(short, long, default_value = "0.05")]
        sensitivity: f64,
        /// Detection algorithm (zscore, ...)
        #[arg(short, long, default_value = "zscore")]
        algorithm: String,
    },
}

// ---------------------------------------------------------------------------
// vector
// ---------------------------------------------------------------------------

/// Vector search subcommands (search, index, stats, compact).
#[derive(Subcommand)]
pub enum VectorSubcommands {
    /// Perform vector similarity search
    Search {
        /// Index name
        #[arg(required = true)]
        index: String,
        /// Query vector as a comma-separated list of floats
        #[arg(required = true)]
        vector: String,
        /// Number of nearest neighbours to return
        #[arg(short, long, default_value = "10")]
        k: u32,
        /// Distance metric (cosine, ...)
        #[arg(short, long, default_value = "cosine")]
        metric: String,
        /// Include the stored vectors in the results
        #[arg(long)]
        include_vectors: bool,
    },
    /// Create or rebuild a vector index
    #[command(name = "index")]
    Index {
        /// Index name
        #[arg(required = true)]
        name: String,
        /// Table to index
        #[arg(required = true)]
        table: String,
        /// Embedding column name
        #[arg(short, long, default_value = "embedding")]
        column: String,
        /// Vector dimensionality
        #[arg(short, long)]
        dimensions: Option<u32>,
        /// Index algorithm (hnsw, ...)
        #[arg(short, long, default_value = "hnsw")]
        algorithm: String,
        /// Distance metric (cosine, ...)
        #[arg(short, long, default_value = "cosine")]
        metric: String,
    },
    /// Show vector index statistics
    Stats {
        /// Index name
        #[arg(required = true)]
        index: String,
        /// Show per-segment statistics
        #[arg(long)]
        segments: bool,
    },
    /// Compact and optimize vector indexes
    Compact {
        /// Index name
        #[arg(required = true)]
        index: String,
        /// Run garbage collection during compaction
        #[arg(long)]
        gc: bool,
        /// Target size ratio after compaction
        #[arg(long, default_value = "0.8")]
        target_ratio: f64,
    },
}

// ---------------------------------------------------------------------------
// graph
// ---------------------------------------------------------------------------

/// Graph traversal subcommands (nodes, edges, query, traverse).
#[derive(Subcommand)]
pub enum GraphSubcommands {
    /// Query graph nodes
    Nodes {
        /// Node label filter
        #[arg(short, long)]
        label: Option<String>,
        /// Property filter expression
        #[arg(short, long)]
        filter: Option<String>,
        /// Maximum number of nodes to return
        #[arg(short, long, default_value = "100")]
        limit: u64,
        /// Return counts instead of node data
        #[arg(long)]
        counts: bool,
    },
    /// Query graph edges
    Edges {
        /// Only edges from this node
        #[arg(short, long)]
        from: Option<String>,
        /// Only edges to this node
        #[arg(short, long)]
        to: Option<String>,
        /// Edge label filter
        #[arg(short, long)]
        label: Option<String>,
        /// Maximum number of edges to return
        #[arg(short, long, default_value = "100")]
        limit: u64,
    },
    /// Execute a graph query
    Query {
        /// Query text (whitespace-joined)
        #[arg(required = true)]
        query: Vec<String>,
        /// Query language (cypher, ...)
        #[arg(short, long, default_value = "cypher")]
        language: String,
    },
    /// Traverse the graph from a starting node
    Traverse {
        /// Starting node identifier
        #[arg(required = true)]
        start: String,
        /// Maximum traversal depth
        #[arg(short, long, default_value = "3")]
        depth: u64,
        /// Restrict traversal to a label
        #[arg(short, long)]
        label: Option<String>,
        /// Traversal strategy (bfs, dfs)
        #[arg(short, long, default_value = "bfs")]
        strategy: String,
    },
}

// ---------------------------------------------------------------------------
// cdc
// ---------------------------------------------------------------------------

/// Change Data Capture subcommands (status, stream, subscribe, offsets).
#[derive(Subcommand)]
pub enum CdcSubcommands {
    /// Show CDC status
    Status {
        /// Show verbose CDC details
        #[arg(long)]
        verbose: bool,
    },
    /// Manage a CDC stream
    Stream {
        /// Stream name
        #[arg(required = true)]
        name: String,
        /// Source table for the stream
        #[arg(short, long)]
        table: Option<String>,
        /// Create the stream
        #[arg(long)]
        create: bool,
        /// Stop the stream
        #[arg(long)]
        stop: bool,
        /// Delete the stream
        #[arg(long)]
        delete: bool,
    },
    /// Subscribe to a CDC stream
    Subscribe {
        /// Stream name
        #[arg(required = true)]
        stream: String,
        /// Start from the beginning instead of the current offset
        #[arg(long)]
        from_start: bool,
        /// Explicit starting offset
        #[arg(long)]
        offset: Option<String>,
        /// Record format (json, ...)
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Show CDC offset information
    Offsets {
        /// Stream name
        #[arg(required = true)]
        stream: String,
        /// Show per-partition offsets
        #[arg(long)]
        partitions: bool,
        /// Set the offset (key=value)
        #[arg(long)]
        set: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// bench
// ---------------------------------------------------------------------------

/// Benchmark subcommands (run, list, report).
#[derive(Subcommand)]
pub enum BenchSubcommands {
    /// Run a benchmark
    Run {
        /// Benchmark profile name
        #[arg(required = true)]
        name: String,
        /// Number of concurrent connections
        #[arg(short, long, default_value = "10")]
        connections: u32,
        /// Benchmark duration in seconds
        #[arg(short, long, default_value = "30")]
        duration: u64,
        /// Target request rate (unlimited if unset)
        #[arg(short, long)]
        rate: Option<u64>,
        /// Read/write mix percentage (0–100)
        #[arg(long, default_value = "50")]
        read_write_mix: u8,
        /// Write results to this file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// List available benchmark profiles
    List {
        /// Show verbose profile details
        #[arg(long)]
        verbose: bool,
    },
    /// Generate a benchmark report
    Report {
        /// Results file to report on
        #[arg(required = true)]
        path: PathBuf,
        /// Report format (text, ...)
        #[arg(short, long, default_value = "text")]
        format: String,
        /// A second results file to compare against
        #[arg(long)]
        compare: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// migrate
// ---------------------------------------------------------------------------

/// Data migration subcommands (inspect-source, plan, import, validate, report).
#[derive(Subcommand)]
pub enum MigrateSubcommands {
    /// Inspect a source database and show its schema
    InspectSource {
        /// Source database type (mysql, postgres, mongodb, ...)
        #[arg(short, long)]
        source: String,
        /// Source connection URL
        #[arg(short, long)]
        url: String,
        /// Target namespace for the schema
        #[arg(long)]
        namespace: Option<String>,
        /// Report format (table, ...)
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Generate a migration plan
    Plan {
        /// Source database type
        #[arg(short, long)]
        source: String,
        /// Source connection URL
        #[arg(short, long)]
        url: String,
        /// Target server URL
        #[arg(short, long)]
        target: Option<String>,
        /// Target namespace
        #[arg(short, long)]
        namespace: Option<String>,
        /// Mapping file (JSON)
        #[arg(long)]
        mapping: Option<PathBuf>,
        /// Plan mode (dry-run, ...)
        #[arg(long, default_value = "dry-run")]
        mode: String,
        /// Report format (table, ...)
        #[arg(long, default_value = "table")]
        format: String,
        /// Write the plan to this file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import data from a source database
    Import {
        /// Source database type
        #[arg(short, long)]
        source: String,
        /// Source connection URL
        #[arg(short, long)]
        url: String,
        /// Target server URL
        #[arg(short, long)]
        target: Option<String>,
        /// Target namespace
        #[arg(short, long)]
        namespace: Option<String>,
        /// Mapping file (JSON)
        #[arg(long)]
        mapping: Option<PathBuf>,
        /// Import mode (copy, ...)
        #[arg(long, default_value = "copy")]
        mode: String,
        /// Rows per batch
        #[arg(long, default_value = "1000")]
        batch_size: u64,
        /// Maximum number of rows to import
        #[arg(long)]
        limit: Option<u64>,
        /// Only import these tables (comma-separated)
        #[arg(long)]
        include: Option<String>,
        /// Skip these tables (comma-separated)
        #[arg(long)]
        exclude: Option<String>,
        /// Overwrite existing data
        #[arg(long)]
        overwrite: bool,
        /// Resume an interrupted import
        #[arg(long)]
        resume: bool,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Validate a completed migration
    Validate {
        /// Target server URL
        #[arg(short, long)]
        target: Option<String>,
        /// Target namespace
        #[arg(short, long)]
        namespace: Option<String>,
        /// Write the validation report to this file
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Generate a migration report
    Report {
        /// Target server URL
        #[arg(short, long)]
        target: Option<String>,
        /// Target namespace
        #[arg(short, long)]
        namespace: Option<String>,
        /// Report format (markdown, table, ...)
        #[arg(long, default_value = "markdown")]
        format: String,
        /// Write the report to this file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// governor
// ---------------------------------------------------------------------------

/// Resource governor subcommands (status, policies, inspect, metrics, violations, set).
#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum GovernorSubcommands {
    /// Show governor status (enabled, active executions, violations)
    Status,
    /// List all governance policies
    Policies {
        /// Filter by policy name
        #[arg(long)]
        name: Option<String>,
    },
    /// Inspect a specific execution
    Inspect {
        /// Execution UUID to inspect
        #[arg(required = true)]
        execution_id: String,
    },
    /// Show governor metrics snapshot
    Metrics {
        /// Keep printing metrics periodically
        #[arg(long)]
        watch: bool,
        /// Poll interval in seconds when watching
        #[arg(long, default_value = "2")]
        interval: u64,
    },
    /// List policy violations
    Violations {
        /// Only violations since this time
        #[arg(long)]
        last: Option<String>,
        /// Filter by workload type
        #[arg(short, long)]
        workload: Option<String>,
        /// Maximum number of violations to return
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Set or update a governance policy
    Set {
        /// Policy name
        #[arg(required = true)]
        name: String,
        /// Maximum memory per execution, in MB
        #[arg(long)]
        max_memory_mb: Option<u64>,
        /// Maximum number of execution steps
        #[arg(long)]
        max_execution_steps: Option<u64>,
        /// Maximum CPU time per execution, in ms
        #[arg(long)]
        max_cpu_time_ms: Option<u64>,
        /// Maximum query complexity score
        #[arg(long)]
        max_query_complexity: Option<u32>,
        /// Maximum number of joins per query
        #[arg(long)]
        max_join_count: Option<u32>,
        /// Maximum number of rows a sort may consider
        #[arg(long)]
        max_sort_rows: Option<u64>,
        /// Maximum pipeline depth
        #[arg(long)]
        max_pipeline_depth: Option<u32>,
        /// Maximum number of pipeline stages
        #[arg(long)]
        max_pipeline_stages: Option<u32>,
        /// Maximum number of FFI calls
        #[arg(long)]
        max_ffi_calls: Option<u64>,
        /// Maximum FFI memory, in MB
        #[arg(long)]
        max_ffi_memory_mb: Option<u64>,
        /// Maximum FFI time, in ms
        #[arg(long)]
        max_ffi_time_ms: Option<u64>,
        /// Maximum training iterations
        #[arg(long)]
        max_training_iterations: Option<u64>,
        /// Maximum prediction batch size
        #[arg(long)]
        max_prediction_batch_size: Option<u64>,
        /// Maximum embedding batch size
        #[arg(long)]
        max_embedding_batch_size: Option<u64>,
        /// Maximum vector search candidates
        #[arg(long)]
        max_vector_candidates: Option<u64>,
        /// Maximum vector search expansions
        #[arg(long)]
        max_vector_expansions: Option<u64>,
        /// Maximum graph traversal depth
        #[arg(long)]
        max_graph_depth: Option<u32>,
        /// Maximum number of graph nodes visited
        #[arg(long)]
        max_graph_nodes: Option<u64>,
        /// Maximum number of graph edges traversed
        #[arg(long)]
        max_graph_edges: Option<u64>,
        /// Maximum rows per import batch run
        #[arg(long)]
        max_import_rows: Option<u64>,
        /// Maximum import batches
        #[arg(long)]
        max_import_batches: Option<u64>,
        /// Maximum backup size, in bytes
        #[arg(long)]
        max_backup_size: Option<u64>,
        /// Maximum restore size, in bytes
        #[arg(long)]
        max_restore_size: Option<u64>,
        /// Enforcement action (monitor, warn, throttle, block)
        #[arg(long, default_value = "monitor")]
        action: String,
        /// Policy scope (global, ...)
        #[arg(long, default_value = "global")]
        scope: String,
    },
}

// ---------------------------------------------------------------------------
// timeseries
// ---------------------------------------------------------------------------

/// Time series operations: query, aggregate, downsample, manage retention and metrics.
#[derive(Subcommand)]
pub enum TimeSeriesSubcommands {
    /// List time series metrics
    List {
        /// Show verbose metric details
        #[arg(long)]
        verbose: bool,
    },
    /// Describe a time series metric
    Describe {
        /// Metric name
        #[arg(required = true)]
        metric: String,
    },
    /// Query time series data points
    Query {
        /// Metric name
        #[arg(required = true)]
        metric: String,
        /// Start time (unix ms, RFC 3339, or naive timestamp)
        #[arg(short, long)]
        start: Option<String>,
        /// End time (unix ms, RFC 3339, or naive timestamp)
        #[arg(short, long)]
        end: Option<String>,
        /// Tag filters as JSON
        #[arg(short, long)]
        tags: Option<String>,
        /// Comma-separated fields to return
        #[arg(short, long)]
        fields: Option<String>,
        /// Maximum number of data points
        #[arg(short, long, default_value = "100")]
        limit: u64,
        /// Resolution to query
        #[arg(long)]
        resolution: Option<String>,
    },
    /// Aggregate time series data
    Aggregate {
        /// Metric name
        #[arg(required = true)]
        metric: String,
        /// Start time
        #[arg(short, long)]
        start: Option<String>,
        /// End time
        #[arg(short, long)]
        end: Option<String>,
        /// Tag filters as JSON
        #[arg(short, long)]
        tags: Option<String>,
        /// Aggregation function (avg, sum, min, max, count)
        #[arg(short, long, default_value = "avg")]
        function: String,
        /// Aggregation interval (1h, 5m, ...)
        #[arg(short, long, default_value = "1h")]
        interval: String,
        /// Gap-fill policy
        #[arg(long)]
        fill: Option<String>,
    },
    /// Downsample a metric to a lower resolution
    Downsample {
        /// Metric name
        #[arg(required = true)]
        metric: String,
        /// Source resolution
        #[arg(short, long, default_value = "raw")]
        source: String,
        /// Target resolution
        #[arg(short, long, default_value = "1h")]
        target: String,
        /// Downsampling function (avg, ...)
        #[arg(short, long, default_value = "avg")]
        function: String,
    },
    /// Apply retention policy to a metric
    Retain {
        /// Metric name
        #[arg(required = true)]
        metric: String,
    },
    /// Add or update a resolution for a metric
    Resolution {
        /// Metric name
        #[arg(required = true)]
        metric: String,
        /// Resolution label
        #[arg(required = true)]
        resolution: String,
        /// Retention period in days
        #[arg(short, long, default_value = "365")]
        retention_days: u32,
        /// Rollup aggregation function
        #[arg(short, long, default_value = "avg")]
        agg_fn: String,
    },
    /// Show engine statistics
    Stats,
}

/// Database integrity subcommands (genesis, records, checkpoints, quarantine).
#[derive(Subcommand)]
pub enum IntegritySubcommands {
    /// Integrity subsystem status (mode, signer, ledger availability)
    Status,
    /// Verify the genesis and signed record chain of a database
    Verify {
        /// Database name
        #[arg(required = true)]
        db: String,
    },
    /// List the signed integrity records of a database
    Records {
        /// Database name
        #[arg(required = true)]
        db: String,
    },
    /// Show the signed genesis identity of a database
    Genesis {
        /// Database name
        #[arg(required = true)]
        db: String,
    },
    /// List checkpoints anchored for a database
    Checkpoints {
        /// Database name
        #[arg(required = true)]
        db: String,
    },
    /// Create and sign a new checkpoint for a database
    Checkpoint {
        /// Database name
        #[arg(required = true)]
        db: String,
    },
    /// List integrity records awaiting ledger anchoring
    Pending,
    /// Retry pending ledger submissions
    Flush,
    /// List quarantined (invalid) records
    Quarantine,
    /// Release a quarantined record
    Release {
        /// Database name
        #[arg(required = true)]
        db: String,
        /// Record sequence number
        #[arg(required = true)]
        sequence: u64,
    },
    /// Show this node's compact chain evidence for a database
    Evidence {
        /// Database name
        #[arg(required = true)]
        db: String,
    },
    /// Reconcile the integrity chain against a peer node
    ///
    /// Fetches the peer's evidence and records from `--peer-url`, compares
    /// chains and prints a repair plan. Nothing is applied automatically.
    Reconcile {
        /// Database name
        #[arg(required = true)]
        db: String,
        /// Base URL of the peer node, e.g. http://127.0.0.1:18768
        #[arg(long)]
        peer_url: String,
        /// Max records to transfer when evidence differs (default: all)
        #[arg(long)]
        max_records: Option<u64>,
    },
    /// Real Hyperledger connectivity health
    Ledger,
}

/// Unified search subcommands (full-text and vector similarity).
#[derive(Subcommand)]
pub enum SearchSubcommands {
    /// Full-text search across tables/collections of every engine
    Text {
        /// Search terms (joined with spaces)
        #[arg(required = true)]
        query: Vec<String>,
        /// Restrict engines (comma-separated: Relational,Document,KeyValue,Columnar,Vector,TimeSeries)
        #[arg(long)]
        storage_types: Option<String>,
        /// Restrict tables/collections (comma-separated)
        #[arg(long)]
        tables: Option<String>,
        /// Token matching mode: and, or, phrase
        #[arg(long, default_value = "and")]
        mode: String,
        /// Maximum number of hits
        #[arg(long, default_value = "20")]
        limit: u64,
        /// Number of hits to skip
        #[arg(long, default_value = "0")]
        offset: u64,
    },
    /// Vector similarity search against vector collections
    Vector {
        /// Query vector as a JSON array, e.g. "[1.0, 0.5, 0.0]"
        #[arg(required = true)]
        query_vector: String,
        /// Restrict tables/collections (comma-separated)
        #[arg(long)]
        tables: Option<String>,
        /// Maximum number of hits
        #[arg(long, default_value = "20")]
        limit: u64,
        /// Number of hits to skip
        #[arg(long, default_value = "0")]
        offset: u64,
    },
}
