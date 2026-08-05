use std::collections::HashMap;

/// Unique identifier for a registered capability.
/// Use reverse-domain notation: e.g. "namespace.create", "cluster.rebalance"
pub type CapabilityId = &'static str;

/// Broad category for grouping/organizing capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityCategory {
    Namespace,
    Database,
    Engine,
    Table,
    Schema,
    Data,
    Security,
    Cluster,
    Config,
    Backup,
    Vector,
    AIML,
    Monitoring,
    Admin,
    Query,
    File,
    Job,
    Analytics,
    Protocol,
    CDC,
    Migration,
    Governor,
    Report,
    Notebook,
    RAG,
    Document,
    Terminal,
    Instance,
    Federation,
    Settings,
    Help,
    Custom(&'static str),
}

impl CapabilityCategory {
    pub fn name(&self) -> &'static str {
        match self {
            CapabilityCategory::Namespace => "Namespace",
            CapabilityCategory::Database => "Database",
            CapabilityCategory::Engine => "Engine",
            CapabilityCategory::Table => "Table",
            CapabilityCategory::Schema => "Schema",
            CapabilityCategory::Data => "Data",
            CapabilityCategory::Security => "Security",
            CapabilityCategory::Cluster => "Cluster",
            CapabilityCategory::Config => "Configuration",
            CapabilityCategory::Backup => "Backup",
            CapabilityCategory::Vector => "Vector",
            CapabilityCategory::AIML => "AI/ML",
            CapabilityCategory::Monitoring => "Monitoring",
            CapabilityCategory::Admin => "Administration",
            CapabilityCategory::Query => "Query",
            CapabilityCategory::File => "File",
            CapabilityCategory::Job => "Job",
            CapabilityCategory::Analytics => "Analytics",
            CapabilityCategory::Protocol => "Protocol",
            CapabilityCategory::CDC => "CDC",
            CapabilityCategory::Migration => "Migration",
            CapabilityCategory::Governor => "Governor",
            CapabilityCategory::Report => "Report",
            CapabilityCategory::Notebook => "Notebook",
            CapabilityCategory::RAG => "RAG",
            CapabilityCategory::Document => "Document",
            CapabilityCategory::Terminal => "Terminal",
            CapabilityCategory::Instance => "Instance",
            CapabilityCategory::Federation => "Federation",
            CapabilityCategory::Settings => "Settings",
            CapabilityCategory::Help => "Help",
            CapabilityCategory::Custom(s) => s,
        }
    }
}

/// What happens when a capability is activated.
#[derive(Debug, Clone)]
pub enum CapabilityAction {
    /// Navigate to a section
    Navigate,
    /// Execute a command string (e.g. ":backup create")
    Command(&'static str),
    /// Toggle a feature on/off
    Toggle(&'static str),
}

/// A registered capability that the TUI can discover and expose.
#[derive(Debug, Clone)]
pub struct Capability {
    pub id: CapabilityId,
    pub name: &'static str,
    pub description: &'static str,
    pub category: CapabilityCategory,
    pub keywords: &'static [&'static str],
    pub needs_connection: bool,
    pub action: CapabilityAction,
}

impl Capability {
    pub const fn new(
        id: CapabilityId,
        name: &'static str,
        description: &'static str,
        category: CapabilityCategory,
        keywords: &'static [&'static str],
        needs_connection: bool,
        action: CapabilityAction,
    ) -> Self {
        Self {
            id,
            name,
            description,
            category,
            keywords,
            needs_connection,
            action,
        }
    }

    /// Format as a command palette entry (e.g. ":create namespace - Create a new namespace")
    pub fn palette_entry(&self) -> String {
        let cmd = match &self.action {
            CapabilityAction::Command(cmd) => cmd,
            CapabilityAction::Navigate => "",
            CapabilityAction::Toggle(_) => "",
        };
        if !cmd.is_empty() {
            format!("{} - {} [{}]", cmd, self.name, self.category.name())
        } else {
            format!(
                ":{} - {} [{}]",
                self.id.replace('.', " "),
                self.name,
                self.category.name()
            )
        }
    }
}

/// Global capability registry for self-discovery.
/// Modules register their capabilities at initialization time.
#[derive(Default)]
pub struct CapabilityRegistry {
    capabilities: Vec<Capability>,
    by_id: HashMap<&'static str, usize>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, cap: Capability) {
        self.by_id.insert(cap.id, self.capabilities.len());
        self.capabilities.push(cap);
    }

    pub fn get(&self, id: &str) -> Option<&Capability> {
        self.by_id.get(id).map(|&i| &self.capabilities[i])
    }

    pub fn all(&self) -> &[Capability] {
        &self.capabilities
    }

    pub fn filter_by_category(&self, category: CapabilityCategory) -> Vec<&Capability> {
        self.capabilities
            .iter()
            .filter(|c| c.category == category)
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&Capability> {
        let q = query.to_lowercase();
        self.capabilities
            .iter()
            .filter(|c| {
                c.id.to_lowercase().contains(&q)
                    || c.name.to_lowercase().contains(&q)
                    || c.description.to_lowercase().contains(&q)
                    || c.keywords.iter().any(|k| k.to_lowercase().contains(&q))
                    || c.category.name().to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn command_palette_entries(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .map(|c| c.palette_entry())
            .collect()
    }

    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

static REGISTRY: std::sync::OnceLock<CapabilityRegistry> = std::sync::OnceLock::new();

/// Get or initialize the global capability registry.
/// Modules should call `register_capability()` at startup to register their features.
pub fn registry() -> &'static CapabilityRegistry {
    REGISTRY.get_or_init(|| {
        let mut reg = CapabilityRegistry::new();
        register_builtins(&mut reg);
        reg
    })
}

/// Register a single capability into the global registry.
/// Should be called during module initialization.
#[allow(unused)]
pub fn register_capability(_cap: Capability) {
    // Placeholder for future dynamic registration.
    // Currently all built-in capabilities are registered in `register_builtins()`.
    // This function will be used by external modules to extend the registry at runtime.
}

fn register_builtins(reg: &mut CapabilityRegistry) {
    use CapabilityAction::*;
    use CapabilityCategory::*;

    // ── Namespace operations ──
    reg.register(Capability::new(
        "namespace.create",
        "Create Namespace",
        "Create a new namespace for data isolation",
        Namespace,
        &["namespace", "create", "new", "add"],
        true,
        Command(":namespace create"),
    ));
    reg.register(Capability::new(
        "namespace.delete",
        "Delete Namespace",
        "Delete an existing namespace",
        Namespace,
        &["namespace", "delete", "remove", "drop"],
        true,
        Command(":namespace delete"),
    ));
    reg.register(Capability::new(
        "namespace.rename",
        "Rename Namespace",
        "Rename a namespace",
        Namespace,
        &["namespace", "rename"],
        true,
        Command(":namespace rename"),
    ));
    reg.register(Capability::new(
        "namespace.use",
        "Switch Namespace",
        "Switch the active namespace",
        Namespace,
        &["namespace", "use", "switch", "select", "active"],
        true,
        Command(":namespace use"),
    ));
    reg.register(Capability::new(
        "namespace.list",
        "List Namespaces",
        "List all available namespaces",
        Namespace,
        &["namespace", "list", "all"],
        true,
        Command(":namespace"),
    ));

    // ── Database operations ──
    reg.register(Capability::new(
        "database.create",
        "Create Database",
        "Create a new database",
        Database,
        &["db", "database", "create", "new", "add"],
        true,
        Command(":db create"),
    ));
    reg.register(Capability::new(
        "database.delete",
        "Delete Database",
        "Delete a database permanently",
        Database,
        &["db", "database", "delete", "remove", "drop"],
        true,
        Command(":db drop"),
    ));
    reg.register(Capability::new(
        "database.list",
        "List Databases",
        "List all databases on the server",
        Database,
        &["db", "database", "list", "all"],
        true,
        Command(":databases"),
    ));

    // ── Engine operations ──
    reg.register(Capability::new(
        "engine.add",
        "Add Storage Engine",
        "Add a new storage engine",
        Engine,
        &["engine", "add", "storage"],
        true,
        Command(":engine add"),
    ));
    reg.register(Capability::new(
        "engine.remove",
        "Remove Storage Engine",
        "Remove an existing storage engine",
        Engine,
        &["engine", "remove", "delete"],
        true,
        Command(":engine remove"),
    ));
    reg.register(Capability::new(
        "engine.list",
        "List Engines",
        "List all storage engines",
        Engine,
        &["engine", "list", "storage"],
        true,
        Command(":databases"),
    ));

    // ── Table / Collection operations ──
    reg.register(Capability::new(
        "table.create",
        "Create Table",
        "Create a new table or collection",
        Table,
        &["table", "collection", "create", "new"],
        true,
        Command(":table create"),
    ));
    reg.register(Capability::new(
        "table.drop",
        "Drop Table",
        "Drop a table or collection",
        Table,
        &["table", "collection", "drop", "delete", "remove"],
        true,
        Command(":table drop"),
    ));
    reg.register(Capability::new(
        "table.list",
        "List Tables",
        "List all tables in the current database",
        Table,
        &["table", "collection", "list", "all"],
        true,
        Command(":tables"),
    ));
    reg.register(Capability::new(
        "table.analyze",
        "Analyze Table",
        "Run ANALYZE TABLE to gather statistics",
        Table,
        &["table", "analyze", "stats", "statistics"],
        true,
        Command(":table analyze"),
    ));

    // ── Data operations ──
    reg.register(Capability::new(
        "data.insert",
        "Insert Data",
        "Insert records into a table",
        Data,
        &["insert", "data", "add", "create"],
        true,
        Command(":insert"),
    ));
    reg.register(Capability::new(
        "data.import.csv",
        "Import CSV",
        "Import data from a CSV file",
        Data,
        &["import", "csv", "data", "load"],
        true,
        Command(":import csv"),
    ));
    reg.register(Capability::new(
        "data.import.json",
        "Import JSON",
        "Import data from a JSON file",
        Data,
        &["import", "json", "data", "load"],
        true,
        Command(":import json"),
    ));
    reg.register(Capability::new(
        "data.export",
        "Export Data",
        "Export data to file",
        Data,
        &["export", "data", "save"],
        true,
        Command(":export"),
    ));

    // ── Security operations ──
    reg.register(Capability::new(
        "user.create",
        "Create User",
        "Create a new user account",
        Security,
        &["user", "create", "new", "account", "security"],
        true,
        Command(":user create"),
    ));
    reg.register(Capability::new(
        "user.delete",
        "Delete User",
        "Delete a user account",
        Security,
        &["user", "delete", "remove", "security"],
        true,
        Command(":user delete"),
    ));
    reg.register(Capability::new(
        "role.create",
        "Create Role",
        "Create a new role for RBAC",
        Security,
        &["role", "create", "new", "rbac"],
        true,
        Command(":role create"),
    ));
    reg.register(Capability::new(
        "role.delete",
        "Delete Role",
        "Delete a role",
        Security,
        &["role", "delete", "remove", "rbac"],
        true,
        Command(":role delete"),
    ));
    reg.register(Capability::new(
        "user.list",
        "List Users",
        "List all users",
        Security,
        &["user", "list", "all", "security"],
        true,
        Command(":security"),
    ));
    reg.register(Capability::new(
        "role.list",
        "List Roles",
        "List all roles",
        Security,
        &["role", "list", "all", "security"],
        true,
        Command(":security"),
    ));
    reg.register(Capability::new(
        "permission.list",
        "List Permissions",
        "List all RBAC permissions",
        Security,
        &["permission", "list", "all", "rbac", "security"],
        true,
        Command(":security"),
    ));
    reg.register(Capability::new(
        "user.role.assign",
        "Assign Role",
        "Assign a role to a user",
        Security,
        &["user", "role", "assign", "rbac"],
        true,
        Command(":user role"),
    ));

    // ── Cluster operations ──
    reg.register(Capability::new(
        "cluster.status",
        "Cluster Status",
        "View cluster status and health",
        Cluster,
        &["cluster", "status", "health"],
        true,
        Command(":cluster"),
    ));
    reg.register(Capability::new(
        "cluster.nodes",
        "Cluster Nodes",
        "List cluster nodes",
        Cluster,
        &["cluster", "nodes", "members"],
        true,
        Command(":cluster"),
    ));
    reg.register(Capability::new(
        "cluster.join",
        "Join Cluster",
        "Join a node to the cluster",
        Cluster,
        &["cluster", "join", "add", "node"],
        true,
        Command(":cluster join"),
    ));
    reg.register(Capability::new(
        "cluster.leave",
        "Leave Cluster",
        "Remove a node from the cluster",
        Cluster,
        &["cluster", "leave", "remove", "node"],
        true,
        Command(":cluster leave"),
    ));
    reg.register(Capability::new(
        "cluster.rebalance",
        "Rebalance Cluster",
        "Rebalance data across cluster nodes",
        Cluster,
        &["cluster", "rebalance", "balance"],
        true,
        Command(":cluster rebalance"),
    ));

    // ── Backup operations ──
    reg.register(Capability::new(
        "backup.create",
        "Create Backup",
        "Create a new backup",
        Backup,
        &["backup", "create", "save", "snapshot"],
        true,
        Command(":backup create"),
    ));
    reg.register(Capability::new(
        "backup.restore",
        "Restore Backup",
        "Restore data from a backup",
        Backup,
        &["backup", "restore", "recover"],
        true,
        Command(":backup restore"),
    ));
    reg.register(Capability::new(
        "backup.list",
        "List Backups",
        "List all available backups",
        Backup,
        &["backup", "list", "all"],
        true,
        Command(":backup"),
    ));
    reg.register(Capability::new(
        "backup.verify",
        "Verify Backup",
        "Verify backup integrity",
        Backup,
        &["backup", "verify", "check", "integrity"],
        true,
        Command(":backup verify"),
    ));

    // ── Config operations ──
    reg.register(Capability::new(
        "config.list",
        "View Configuration",
        "View server configuration",
        Config,
        &["config", "configuration", "settings", "view"],
        true,
        Command(":config"),
    ));
    reg.register(Capability::new(
        "config.set",
        "Set Config Entry",
        "Modify a configuration entry",
        Config,
        &["config", "set", "modify", "change", "update"],
        true,
        Command(":config set"),
    ));
    reg.register(Capability::new(
        "config.validate",
        "Validate Config",
        "Validate the server configuration",
        Config,
        &["config", "validate", "check"],
        true,
        Command(":config validate"),
    ));
    reg.register(Capability::new(
        "config.export",
        "Export Config",
        "Export configuration to file",
        Config,
        &["config", "export", "save"],
        true,
        Command(":config export"),
    ));
    reg.register(Capability::new(
        "config.import",
        "Import Config",
        "Import configuration from file",
        Config,
        &["config", "import", "load"],
        true,
        Command(":config import"),
    ));
    reg.register(Capability::new(
        "config.snapshot",
        "Config Snapshots",
        "Manage configuration snapshots",
        Config,
        &["config", "snapshot", "version", "history"],
        true,
        Command(":config snapshots"),
    ));

    // ── Governor operations ──
    reg.register(Capability::new(
        "governor.status",
        "Governor Status",
        "View resource governor status and policies",
        Governor,
        &["governor", "rg", "resource", "status"],
        true,
        Command(":governor"),
    ));
    reg.register(Capability::new(
        "governor.set",
        "Set Governor Policy",
        "Create or modify a governor policy",
        Governor,
        &["governor", "policy", "set", "create"],
        true,
        Command(":governor set"),
    ));

    // ── Vector operations ──
    reg.register(Capability::new(
        "vector.search",
        "Vector Search",
        "Execute a vector similarity search",
        Vector,
        &["vector", "search", "similarity", "ann"],
        true,
        Command(":vector search"),
    ));
    reg.register(Capability::new(
        "vector.index",
        "Vector Index",
        "Create or manage vector indexes",
        Vector,
        &["vector", "index", "create", "hnsw", "ivf"],
        true,
        Command(":vector index"),
    ));

    // ── AI/ML operations ──
    reg.register(Capability::new(
        "ai.models",
        "AI Models",
        "List and manage AI/ML models",
        AIML,
        &["ai", "model", "ml", "machine learning"],
        true,
        Command(":rag"),
    ));
    reg.register(Capability::new(
        "ai.analyze",
        "AI Analyze",
        "Run AI analysis on data",
        AIML,
        &["ai", "analyze", "predict", "ml"],
        true,
        Command(":ai analyze"),
    ));

    // ── Monitoring operations ──
    reg.register(Capability::new(
        "monitor.overview",
        "Monitoring Overview",
        "View monitoring overview dashboard",
        Monitoring,
        &["monitor", "overview", "dashboard"],
        true,
        Command(":monitor"),
    ));
    reg.register(Capability::new(
        "monitor.alerts",
        "Monitoring Alerts",
        "View and manage alerts",
        Monitoring,
        &["monitor", "alert", "notification"],
        true,
        Command(":monitor"),
    ));
    reg.register(Capability::new(
        "monitor.performance",
        "Performance Metrics",
        "View performance metrics",
        Monitoring,
        &["monitor", "performance", "metrics", "latency"],
        true,
        Command(":monitor"),
    ));
    reg.register(Capability::new(
        "monitor.replication",
        "Replication Status",
        "View replication lag and status",
        Monitoring,
        &["monitor", "replication", "lag", "sync"],
        true,
        Command(":monitor"),
    ));
    reg.register(Capability::new(
        "health.check",
        "Health Check",
        "Run a server health check",
        Monitoring,
        &["health", "status", "server", "ping"],
        true,
        Command(":health"),
    ));

    // ── Admin operations ──
    reg.register(Capability::new(
        "server.status",
        "Server Status",
        "View detailed server status information",
        Admin,
        &["server", "status", "info", "version"],
        true,
        Command(":status"),
    ));
    reg.register(Capability::new(
        "server.start",
        "Start Server",
        "Start the PrimusDB server",
        Admin,
        &["server", "start", "launch", "boot"],
        false,
        Command(":server start"),
    ));
    reg.register(Capability::new(
        "server.stop",
        "Stop Server",
        "Stop the PrimusDB server",
        Admin,
        &["server", "stop", "shutdown", "halt"],
        true,
        Command(":server stop"),
    ));
    reg.register(Capability::new(
        "server.restart",
        "Restart Server",
        "Restart the PrimusDB server",
        Admin,
        &["server", "restart", "reboot"],
        true,
        Command(":server restart"),
    ));
    reg.register(Capability::new(
        "maintenance.on",
        "Enable Maintenance Mode",
        "Enable maintenance mode",
        Admin,
        &["maintenance", "on", "enable", "maintain"],
        true,
        Command(":maintenance on"),
    ));
    reg.register(Capability::new(
        "maintenance.off",
        "Disable Maintenance Mode",
        "Disable maintenance mode",
        Admin,
        &["maintenance", "off", "disable"],
        true,
        Command(":maintenance off"),
    ));

    // ── Migration operations ──
    reg.register(Capability::new(
        "migration.start",
        "Start Migration",
        "Start a data migration from an external source",
        Migration,
        &["migrate", "migration", "import", "data"],
        true,
        Command(":migrate"),
    ));
    reg.register(Capability::new(
        "migration.validate",
        "Validate Migration",
        "Validate a migration plan",
        Migration,
        &["migrate", "validate", "plan", "check"],
        true,
        Command(":migrate validate"),
    ));

    // ── Query operations ──
    reg.register(Capability::new(
        "query.execute",
        "Execute Query",
        "Execute a SQL or UQL query",
        Query,
        &["query", "sql", "uql", "execute", "run"],
        true,
        Command(":query"),
    ));
    reg.register(Capability::new(
        "query.explain",
        "Explain Query",
        "Show the execution plan for a query",
        Query,
        &["query", "explain", "plan", "analyze"],
        true,
        Command(":explain"),
    ));

    // ── Report operations ──
    reg.register(Capability::new(
        "report.create",
        "Create Report",
        "Create a new report definition",
        Report,
        &["report", "create", "new"],
        true,
        Command(":reports"),
    ));
    reg.register(Capability::new(
        "report.run",
        "Run Report",
        "Execute a report and view results",
        Report,
        &["report", "run", "execute", "generate"],
        true,
        Command(":report run"),
    ));
    reg.register(Capability::new(
        "report.update",
        "Update Report",
        "Edit an existing report definition",
        Report,
        &["report", "update", "edit", "modify"],
        true,
        Command(":report edit"),
    ));

    // ── Notebook operations ──
    reg.register(Capability::new(
        "notebook.create",
        "Create Notebook",
        "Create a new analysis notebook",
        Notebook,
        &["notebook", "create", "new", "analysis"],
        true,
        Command(":notebook"),
    ));
    reg.register(Capability::new(
        "notebook.run",
        "Run Notebook",
        "Execute all cells in a notebook",
        Notebook,
        &["notebook", "run", "execute"],
        true,
        Command(":notebook run"),
    ));

    // ── RAG operations ──
    reg.register(Capability::new(
        "rag.search",
        "RAG Search",
        "Execute a RAG similarity search",
        RAG,
        &["rag", "search", "semantic", "vector"],
        true,
        Command(":rag"),
    ));
    reg.register(Capability::new(
        "rag.collection.create",
        "Create RAG Collection",
        "Create a new vector collection for RAG",
        RAG,
        &["rag", "collection", "create", "vector"],
        true,
        Command(":rag create"),
    ));
    reg.register(Capability::new(
        "rag.collection.delete",
        "Delete RAG Collection",
        "Delete a vector collection",
        RAG,
        &["rag", "collection", "delete", "remove"],
        true,
        Command(":rag delete"),
    ));

    // ── Federation operations ──
    reg.register(Capability::new(
        "federation.status",
        "Federation Status",
        "View federation cluster status",
        Federation,
        &["federation", "cluster", "domain", "status"],
        true,
        Command(":federation"),
    ));
    reg.register(Capability::new(
        "federation.cluster.add",
        "Add Cluster",
        "Add a cluster to the federation",
        Federation,
        &["federation", "cluster", "add", "join"],
        true,
        Command(":federation cluster add"),
    ));
    reg.register(Capability::new(
        "federation.cluster.remove",
        "Remove Cluster",
        "Remove a cluster from the federation",
        Federation,
        &["federation", "cluster", "remove", "delete", "leave"],
        true,
        Command(":federation cluster remove"),
    ));
    reg.register(Capability::new(
        "federation.domain.create",
        "Create Domain",
        "Create a new DataDomain in the federation",
        Federation,
        &["federation", "domain", "create", "add"],
        true,
        Command(":federation domain create"),
    ));
    reg.register(Capability::new(
        "federation.domain.delete",
        "Delete Domain",
        "Delete a DataDomain from the federation",
        Federation,
        &["federation", "domain", "delete", "remove"],
        true,
        Command(":federation domain delete"),
    ));

    // ── Log operations ──
    reg.register(Capability::new(
        "logs.view",
        "View Logs",
        "View server logs and metrics",
        Monitoring,
        &["log", "metrics", "view", "tail"],
        true,
        Command(":metrics"),
    ));

    // ── Session operations ──
    reg.register(Capability::new(
        "session.switch",
        "Switch Session",
        "Switch between connected sessions",
        Instance,
        &["session", "switch", "change"],
        true,
        Command(":session next"),
    ));
    reg.register(Capability::new(
        "session.manager",
        "Session Manager",
        "Open the session manager overlay",
        Instance,
        &["session", "manager", "list"],
        true,
        Command(":sessions"),
    ));

    // ── Document operations ──
    reg.register(Capability::new(
        "document.create",
        "Create Document",
        "Create a new JSON document",
        Document,
        &["document", "create", "json", "new"],
        true,
        Command(":doc create"),
    ));
    reg.register(Capability::new(
        "document.validate",
        "Validate Document",
        "Validate a JSON document",
        Document,
        &["document", "validate", "json", "check"],
        true,
        Command(":doc validate"),
    ));

    // ── Terminal operations ──
    reg.register(Capability::new(
        "terminal.open",
        "Open Terminal",
        "Open the integrated terminal",
        Terminal,
        &["terminal", "shell", "console", "bash"],
        true,
        Command(":terminal"),
    ));
    reg.register(Capability::new(
        "terminal.clear",
        "Clear Terminal",
        "Clear the terminal output",
        Terminal,
        &["terminal", "clear", "clean", "reset"],
        true,
        Command(":terminal clear"),
    ));

    // ── File operations ──
    reg.register(Capability::new(
        "file.browse",
        "Browse Filesystem",
        "Browse the local filesystem from the TUI",
        File,
        &["file", "browse", "filesystem", "explorer"],
        false,
        Command(":files"),
    ));
    reg.register(Capability::new(
        "file.read",
        "Read File",
        "Read file contents in the TUI",
        File,
        &["file", "read", "view", "cat"],
        false,
        Command(":files"),
    ));
    reg.register(Capability::new(
        "file.delete",
        "Delete File",
        "Delete a local file from the TUI",
        File,
        &["file", "delete", "remove", "rm"],
        false,
        Command(":files"),
    ));

    // ── Export operations ──
    reg.register(Capability::new(
        "export.query",
        "Export Query Result",
        "Save the last query result to a file",
        Query,
        &["export", "query", "save", "file", "result"],
        true,
        Command(":export query"),
    ));
}
