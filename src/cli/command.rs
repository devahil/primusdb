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
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,
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
        #[arg(short, long)]
        server: Option<String>,
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
    /// Launch the terminal user interface
    Tui {
        #[arg(short, long)]
        server: Option<String>,
        #[arg(long)]
        no_color: bool,
    },
    /// Execute a raw query
    Query {
        #[arg(required = true)]
        query: Vec<String>,
        #[arg(short, long)]
        database: Option<String>,
    },
    /// Execute a SQL query
    Sql {
        #[arg(required = true)]
        sql: Vec<String>,
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
        #[arg(required = true)]
        source: PathBuf,
        #[arg(short, long)]
        database: Option<String>,
        #[arg(long)]
        force: bool,
    },
    /// Show database and system metrics
    Metrics {
        #[arg(short, long)]
        filter: Option<String>,
        #[arg(long)]
        watch: bool,
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
        #[arg(long)]
        aggressive: bool,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Discover PrimusDB nodes on the network
    Discover {
        #[arg(short, long, default_value = "255.255.255.255")]
        broadcast: String,
        #[arg(short, long, default_value = "7890")]
        port: u16,
        #[arg(long, default_value = "5")]
        timeout: u64,
    },
    /// Resource Governor — execution governance and resource management
    #[command(subcommand)]
    Governor(GovernorSubcommands),
    /// Generate shell completion scripts
    Completion {
        #[arg(required = true)]
        shell: String,
    },
    /// Display version information
    Version {
        #[arg(long)]
        verbose: bool,
    },
}

// ---------------------------------------------------------------------------
// server
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum ServerSubcommands {
    /// Start the PrimusDB server daemon
    Start {
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
        #[arg(short, long)]
        bind: Option<String>,
        #[arg(short, long)]
        data_dir: Option<PathBuf>,
        #[arg(long)]
        daemon: bool,
        #[arg(long, default_value = "info")]
        log_level: String,
    },
    /// Stop the running server
    Stop {
        #[arg(long, default_value = "30")]
        timeout: u64,
        #[arg(long)]
        force: bool,
    },
    /// Restart the server
    Restart {
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
    /// Show server status
    Status {
        #[arg(long)]
        verbose: bool,
    },
    /// Check server health
    Health {
        #[arg(long)]
        deep: bool,
    },
    /// View or modify configuration
    Config {
        #[arg(short, long)]
        get: Option<String>,
        #[arg(short, long)]
        set: Option<Vec<String>>,
        #[arg(short, long)]
        list: bool,
        #[arg(short, long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// config
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum ConfigSubcommands {
    /// Generate a default PrimusDB configuration file
    Init {
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
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },
    /// Display the current configuration
    Show {
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// db
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum DbSubcommands {
    /// List all databases
    List {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        engine: Option<String>,
    },
    /// Create a new database
    Create {
        #[arg(required = true)]
        name: String,
        #[arg(short, long, default_value = "document")]
        engine: String,
        #[arg(short, long)]
        namespace: Option<String>,
    },
    /// Drop a database
    Drop {
        #[arg(required = true)]
        name: String,
        #[arg(short, long)]
        force: bool,
    },
    /// Describe a database
    Describe {
        #[arg(required = true)]
        name: String,
        #[arg(long)]
        schema: bool,
    },
    /// Switch active database
    #[command(name = "use")]
    Use {
        #[arg(required = true)]
        name: String,
    },
}

// ---------------------------------------------------------------------------
// engine
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum EngineSubcommands {
    /// List registered storage engines
    List {
        #[arg(long)]
        verbose: bool,
    },
    /// Show engine status
    Status {
        #[arg(required = true)]
        name: String,
    },
    /// Inspect internal engine state
    Inspect {
        #[arg(required = true)]
        name: String,
        #[arg(short, long)]
        component: Option<String>,
        #[arg(long)]
        raw: bool,
    },
    /// Show engine metrics
    Metrics {
        #[arg(required = true)]
        name: String,
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

#[derive(Subcommand)]
pub enum NamespaceSubcommands {
    /// List namespaces
    List {
        #[arg(short, long)]
        parent: Option<String>,
        #[arg(long)]
        full_paths: bool,
    },
    /// Create a namespace
    Create {
        #[arg(required = true)]
        path: String,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short, long)]
        parent: Option<String>,
        #[arg(long)]
        quota: Option<String>,
    },
    /// Drop a namespace
    Drop {
        #[arg(required = true)]
        path: String,
        #[arg(short, long)]
        recursive: bool,
        #[arg(short, long)]
        force: bool,
    },
    /// Describe a namespace
    Describe {
        #[arg(required = true)]
        path: String,
        #[arg(long)]
        resources: bool,
    },
    /// View or set namespace policy
    Policy {
        #[arg(required = true)]
        path: String,
        #[arg(short, long)]
        set: Option<Vec<String>>,
        #[arg(short, long)]
        unset: Option<String>,
        #[arg(short, long)]
        list: bool,
    },
}

// ---------------------------------------------------------------------------
// instance
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum InstanceSubcommands {
    /// List all local PrimusDB instances
    List {
        #[arg(long)]
        all: bool,
        #[arg(short, long)]
        format: Option<String>,
    },
    /// Discover PrimusDB instances on the network
    Discover {
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,
        #[arg(short, long, default_value = "8080")]
        start_port: u16,
        #[arg(short, long, default_value = "5")]
        max_ports: u16,
        #[arg(long, default_value = "5")]
        timeout: u64,
    },
    /// Inspect a specific instance
    Inspect {
        #[arg(required = true)]
        endpoint: String,
        #[arg(long)]
        verbose: bool,
    },
    /// Connect to an instance interactively
    Connect {
        #[arg(required = true)]
        endpoint: String,
        #[arg(long, default_value = "10")]
        timeout: u64,
    },
    /// Stop a running instance
    Stop {
        #[arg(required = true)]
        endpoint: String,
        #[arg(long)]
        force: bool,
    },
    /// Show logs from an instance
    Logs {
        #[arg(required = true)]
        endpoint: String,
        #[arg(short, long, default_value = "50")]
        lines: u32,
        #[arg(long)]
        follow: bool,
    },
}

// ---------------------------------------------------------------------------
// cluster
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum ClusterSubcommands {
    /// Show cluster status
    Status {
        #[arg(long)]
        verbose: bool,
        #[arg(long)]
        watch: bool,
        #[arg(long, default_value = "2")]
        interval: u64,
    },
    /// List cluster nodes
    Nodes {
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        state: Option<String>,
    },
    /// Join an existing cluster
    #[command(name = "join")]
    Join {
        #[arg(required = true)]
        peer: String,
        #[arg(short, long)]
        node_id: Option<String>,
        /// Node ID to register (alternative to node_id)
        #[arg(short = 'n', long)]
        node: Option<String>,
        /// Seed server address (if provided, use as registration target)
        #[arg(short = 's', long)]
        seed: Option<String>,
        #[arg(long, default_value = "30")]
        timeout: u64,
        #[arg(long)]
        tls: bool,
    },
    /// Leave the current cluster
    #[command(name = "leave")]
    Leave {
        /// Node to remove from the cluster
        #[arg(required = true)]
        node: String,
        #[arg(long)]
        drain: bool,
        #[arg(long)]
        force: bool,
    },
    /// Trigger cluster rebalance
    Rebalance {
        #[arg(short, long)]
        node: Option<String>,
        #[arg(long, default_value = "size")]
        strategy: String,
        #[arg(long, default_value = "2")]
        concurrency: u32,
    },
    /// Trigger manual failover
    Failover {
        #[arg(required = true)]
        node: String,
        #[arg(short, long)]
        target: Option<String>,
        #[arg(long)]
        force: bool,
    },
    /// Check cluster health
    Health {
        #[arg(long)]
        diagnostic: bool,
        #[arg(long, default_value = "100")]
        threshold_ms: u64,
    },
    /// Trigger cluster synchronization
    Sync {
        #[arg(long)]
        full: bool,
        #[arg(short, long, default_value = "60")]
        timeout: u64,
    },
    /// View or modify cluster configuration
    Config {
        #[arg(short, long)]
        get: Option<String>,
        #[arg(short, long)]
        set: Option<Vec<String>>,
        #[arg(short, long)]
        list: bool,
    },
    /// Inspect a specific cluster node
    Inspect {
        #[arg(required = true)]
        node: String,
        #[arg(long)]
        verbose: bool,
    },
    /// Show cluster topology
    Topology {
        #[arg(long, default_value = "table")]
        format: String,
    },
}

// ---------------------------------------------------------------------------
// protocol
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum ProtocolSubcommands {
    /// Check protocol layer health
    Health {
        #[arg(short, long)]
        module: Option<String>,
    },
    /// Show protocol status and capabilities
    Status {
        #[arg(long)]
        versions: bool,
        #[arg(long)]
        connections: bool,
    },
    /// List protocol peers
    Peers {
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        verbose: bool,
    },
    /// Show protocol metrics
    Metrics {
        #[arg(required = true)]
        protocol: String,
        #[arg(long)]
        raw: bool,
    },
}

// ---------------------------------------------------------------------------
// backup
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum BackupSubcommands {
    /// Create a new backup
    Create {
        #[arg(long)]
        destination: Option<PathBuf>,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        databases: Option<String>,
        #[arg(long, default_value = "zstd")]
        compression: String,
        #[arg(long)]
        encrypt: bool,
        #[arg(long)]
        description: Option<String>,
    },
    /// List available backups
    List {
        #[arg(short, long)]
        directory: Option<PathBuf>,
        #[arg(long)]
        verbose: bool,
    },
    /// Inspect a backup archive
    Inspect {
        #[arg(required = true)]
        path: PathBuf,
        #[arg(long)]
        contents: bool,
        #[arg(long)]
        metadata: bool,
    },
    /// Restore from a backup
    Restore {
        #[arg(required = true)]
        source: PathBuf,
        #[arg(short, long)]
        database: Option<String>,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        pitr: Option<String>,
    },
    /// Verify backup integrity
    Verify {
        #[arg(required = true)]
        path: PathBuf,
        #[arg(long)]
        full: bool,
        #[arg(long)]
        compare: bool,
    },
    /// Delete a backup
    Delete {
        #[arg(required = true)]
        name: String,
        #[arg(long)]
        force: bool,
    },
    /// Export backup manifest metadata
    ExportManifest {
        #[arg(required = true)]
        name: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// auth
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum AuthSubcommands {
    /// Authenticate and obtain a session token
    Login {
        #[arg(required = true)]
        username: String,
        #[arg(short, long)]
        password: Option<String>,
        #[arg(short, long, default_value = "default")]
        realm: String,
        #[arg(long, default_value = "86400")]
        ttl: u64,
    },
    /// Invalidate the current session
    Logout {
        #[arg(long)]
        all: bool,
    },
    /// Manage authentication tokens
    Token {
        #[arg(long)]
        create: bool,
        #[arg(long)]
        revoke: Option<String>,
        #[arg(long)]
        list: bool,
    },
    /// Display current user identity
    Whoami {
        #[arg(long)]
        verbose: bool,
    },
}

// ---------------------------------------------------------------------------
// user
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum UserSubcommands {
    /// Create a new user
    Create {
        #[arg(required = true)]
        username: String,
        #[arg(short, long)]
        password: Option<String>,
        #[arg(short, long)]
        role: Option<String>,
        #[arg(short, long)]
        email: Option<String>,
        #[arg(long, default_value = "true")]
        active: bool,
    },
    /// List users
    List {
        #[arg(short, long)]
        role: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Disable or re-enable a user
    Disable {
        #[arg(required = true)]
        username: String,
        #[arg(short, long)]
        reason: Option<String>,
        #[arg(long)]
        reenable: bool,
    },
    /// Manage user role assignments
    Roles {
        #[arg(required = true)]
        username: String,
        #[arg(short, long)]
        grant: Option<String>,
        #[arg(short, long)]
        revoke: Option<String>,
        #[arg(short, long)]
        list: bool,
    },
}

// ---------------------------------------------------------------------------
// role
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum RoleSubcommands {
    /// Create a new role
    Create {
        #[arg(required = true)]
        name: String,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short, long)]
        inherits: Option<String>,
    },
    /// List all roles
    List {
        #[arg(long)]
        permissions: bool,
    },
    /// Grant a permission to a role
    Grant {
        #[arg(required = true)]
        role: String,
        #[arg(required = true)]
        permission: String,
        #[arg(short, long)]
        namespace: Option<String>,
    },
    /// Revoke a permission from a role
    Revoke {
        #[arg(required = true)]
        role: String,
        #[arg(required = true)]
        permission: String,
    },
}

// ---------------------------------------------------------------------------
// ai
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum AiSubcommands {
    /// List available AI/ML models
    Models {
        #[arg(short, long)]
        kind: Option<String>,
        #[arg(long)]
        verbose: bool,
    },
    /// Train a new model
    Train {
        #[arg(required = true)]
        name: String,
        #[arg(required = true)]
        dataset: String,
        #[arg(short, long, default_value = "regression")]
        model_type: String,
        #[arg(short, long)]
        target: Option<String>,
        #[arg(long)]
        params: Option<String>,
        #[arg(long, default_value = "0.2")]
        test_split: f64,
        #[arg(long, default_value = "3600")]
        max_time: u64,
    },
    /// Make predictions using a trained model
    Predict {
        #[arg(required = true)]
        model: String,
        #[arg(required = true)]
        input: String,
        #[arg(long)]
        raw: bool,
        #[arg(long, default_value = "1")]
        top_k: u32,
    },
    /// Analyze data patterns
    Analyze {
        #[arg(required = true)]
        table: String,
        #[arg(short, long)]
        columns: Option<String>,
        #[arg(short, long, default_value = "summary")]
        analysis_type: String,
    },
    /// Detect anomalies in a dataset
    Anomalies {
        #[arg(required = true)]
        table: String,
        #[arg(short, long)]
        columns: Option<String>,
        #[arg(short, long, default_value = "0.05")]
        sensitivity: f64,
        #[arg(short, long, default_value = "zscore")]
        algorithm: String,
    },
}

// ---------------------------------------------------------------------------
// vector
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum VectorSubcommands {
    /// Perform vector similarity search
    Search {
        #[arg(required = true)]
        index: String,
        #[arg(required = true)]
        vector: String,
        #[arg(short, long, default_value = "10")]
        k: u32,
        #[arg(short, long, default_value = "cosine")]
        metric: String,
        #[arg(long)]
        include_vectors: bool,
    },
    /// Create or rebuild a vector index
    #[command(name = "index")]
    Index {
        #[arg(required = true)]
        name: String,
        #[arg(required = true)]
        table: String,
        #[arg(short, long, default_value = "embedding")]
        column: String,
        #[arg(short, long)]
        dimensions: Option<u32>,
        #[arg(short, long, default_value = "hnsw")]
        algorithm: String,
        #[arg(short, long, default_value = "cosine")]
        metric: String,
    },
    /// Show vector index statistics
    Stats {
        #[arg(required = true)]
        index: String,
        #[arg(long)]
        segments: bool,
    },
    /// Compact and optimize vector indexes
    Compact {
        #[arg(required = true)]
        index: String,
        #[arg(long)]
        gc: bool,
        #[arg(long, default_value = "0.8")]
        target_ratio: f64,
    },
}

// ---------------------------------------------------------------------------
// graph
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum GraphSubcommands {
    /// Query graph nodes
    Nodes {
        #[arg(short, long)]
        label: Option<String>,
        #[arg(short, long)]
        filter: Option<String>,
        #[arg(short, long, default_value = "100")]
        limit: u64,
        #[arg(long)]
        counts: bool,
    },
    /// Query graph edges
    Edges {
        #[arg(short, long)]
        from: Option<String>,
        #[arg(short, long)]
        to: Option<String>,
        #[arg(short, long)]
        label: Option<String>,
        #[arg(short, long, default_value = "100")]
        limit: u64,
    },
    /// Execute a graph query
    Query {
        #[arg(required = true)]
        query: Vec<String>,
        #[arg(short, long, default_value = "cypher")]
        language: String,
    },
    /// Traverse the graph from a starting node
    Traverse {
        #[arg(required = true)]
        start: String,
        #[arg(short, long, default_value = "3")]
        depth: u64,
        #[arg(short, long)]
        label: Option<String>,
        #[arg(short, long, default_value = "bfs")]
        strategy: String,
    },
}

// ---------------------------------------------------------------------------
// cdc
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum CdcSubcommands {
    /// Show CDC status
    Status {
        #[arg(long)]
        verbose: bool,
    },
    /// Manage a CDC stream
    Stream {
        #[arg(required = true)]
        name: String,
        #[arg(short, long)]
        table: Option<String>,
        #[arg(long)]
        create: bool,
        #[arg(long)]
        stop: bool,
        #[arg(long)]
        delete: bool,
    },
    /// Subscribe to a CDC stream
    Subscribe {
        #[arg(required = true)]
        stream: String,
        #[arg(long)]
        from_start: bool,
        #[arg(long)]
        offset: Option<String>,
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Show CDC offset information
    Offsets {
        #[arg(required = true)]
        stream: String,
        #[arg(long)]
        partitions: bool,
        #[arg(long)]
        set: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// bench
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum BenchSubcommands {
    /// Run a benchmark
    Run {
        #[arg(required = true)]
        name: String,
        #[arg(short, long, default_value = "10")]
        connections: u32,
        #[arg(short, long, default_value = "30")]
        duration: u64,
        #[arg(short, long)]
        rate: Option<u64>,
        #[arg(long, default_value = "50")]
        read_write_mix: u8,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// List available benchmark profiles
    List {
        #[arg(long)]
        verbose: bool,
    },
    /// Generate a benchmark report
    Report {
        #[arg(required = true)]
        path: PathBuf,
        #[arg(short, long, default_value = "text")]
        format: String,
        #[arg(long)]
        compare: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// migrate
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum MigrateSubcommands {
    /// Inspect a source database and show its schema
    InspectSource {
        #[arg(short, long)]
        source: String,
        #[arg(short, long)]
        url: String,
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Generate a migration plan
    Plan {
        #[arg(short, long)]
        source: String,
        #[arg(short, long)]
        url: String,
        #[arg(short, long)]
        target: Option<String>,
        #[arg(short, long)]
        namespace: Option<String>,
        #[arg(long)]
        mapping: Option<PathBuf>,
        #[arg(long, default_value = "dry-run")]
        mode: String,
        #[arg(long, default_value = "table")]
        format: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import data from a source database
    Import {
        #[arg(short, long)]
        source: String,
        #[arg(short, long)]
        url: String,
        #[arg(short, long)]
        target: Option<String>,
        #[arg(short, long)]
        namespace: Option<String>,
        #[arg(long)]
        mapping: Option<PathBuf>,
        #[arg(long, default_value = "copy")]
        mode: String,
        #[arg(long, default_value = "1000")]
        batch_size: u64,
        #[arg(long)]
        limit: Option<u64>,
        #[arg(long)]
        include: Option<String>,
        #[arg(long)]
        exclude: Option<String>,
        #[arg(long)]
        overwrite: bool,
        #[arg(long)]
        resume: bool,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Validate a completed migration
    Validate {
        #[arg(short, long)]
        target: Option<String>,
        #[arg(short, long)]
        namespace: Option<String>,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Generate a migration report
    Report {
        #[arg(short, long)]
        target: Option<String>,
        #[arg(short, long)]
        namespace: Option<String>,
        #[arg(long, default_value = "markdown")]
        format: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// governor
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum GovernorSubcommands {
    /// Show governor status (enabled, active executions, violations)
    Status,
    /// List all governance policies
    Policies {
        #[arg(long)]
        name: Option<String>,
    },
    /// Inspect a specific execution
    Inspect {
        #[arg(required = true)]
        execution_id: String,
    },
    /// Show governor metrics snapshot
    Metrics {
        #[arg(long)]
        watch: bool,
        #[arg(long, default_value = "2")]
        interval: u64,
    },
    /// List policy violations
    Violations {
        #[arg(long)]
        last: Option<String>,
        #[arg(short, long)]
        workload: Option<String>,
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Set or update a governance policy
    Set {
        #[arg(required = true)]
        name: String,
        #[arg(long)]
        max_memory_mb: Option<u64>,
        #[arg(long)]
        max_execution_steps: Option<u64>,
        #[arg(long)]
        max_cpu_time_ms: Option<u64>,
        #[arg(long)]
        max_query_complexity: Option<u32>,
        #[arg(long)]
        max_join_count: Option<u32>,
        #[arg(long)]
        max_sort_rows: Option<u64>,
        #[arg(long)]
        max_pipeline_depth: Option<u32>,
        #[arg(long)]
        max_pipeline_stages: Option<u32>,
        #[arg(long)]
        max_ffi_calls: Option<u64>,
        #[arg(long)]
        max_ffi_memory_mb: Option<u64>,
        #[arg(long)]
        max_ffi_time_ms: Option<u64>,
        #[arg(long)]
        max_training_iterations: Option<u64>,
        #[arg(long)]
        max_prediction_batch_size: Option<u64>,
        #[arg(long)]
        max_embedding_batch_size: Option<u64>,
        #[arg(long)]
        max_vector_candidates: Option<u64>,
        #[arg(long)]
        max_vector_expansions: Option<u64>,
        #[arg(long)]
        max_graph_depth: Option<u32>,
        #[arg(long)]
        max_graph_nodes: Option<u64>,
        #[arg(long)]
        max_graph_edges: Option<u64>,
        #[arg(long)]
        max_import_rows: Option<u64>,
        #[arg(long)]
        max_import_batches: Option<u64>,
        #[arg(long)]
        max_backup_size: Option<u64>,
        #[arg(long)]
        max_restore_size: Option<u64>,
        #[arg(long, default_value = "monitor")]
        action: String,
        #[arg(long, default_value = "global")]
        scope: String,
    },
}
