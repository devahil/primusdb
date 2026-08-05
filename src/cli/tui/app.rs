use crate::cli::discovery::InstanceInfo;
use crate::cli::tui::capability::{self, CapabilityAction};
use crate::cli::tui::config::TuiConfig;
use crate::cli::tui::panels;
use crate::cli::tui::workspace::Workspace;
use std::collections::HashMap;
use std::time::Instant;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SIDEBAR_WIDTH: u16 = 24;
pub const HEADER_HEIGHT: u16 = 1;
pub const INPUT_HEIGHT: u16 = 3;
pub const STATUS_HEIGHT: u16 = 4;
pub const MIN_TERMINAL_W: u16 = 60;
pub const MIN_TERMINAL_H: u16 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavSection {
    Dashboard,
    QueryConsole,
    DatabasesEngines,
    Namespaces,
    Cluster,
    Federation,
    Governor,
    BackupRestore,
    MetricsLogs,
    ConfigurationStudio,
    TableExplorer,
    ReportBuilder,
    Notebook,
    RAGWorkspace,
    SecurityCenter,
    DocumentWorkspace,
    IntegratedTerminal,
    Monitoring,
    Settings,
    FileBrowser,
    Help,
}

pub const NAV_SECTIONS: &[NavSection] = &[
    NavSection::Dashboard,
    NavSection::QueryConsole,
    NavSection::DatabasesEngines,
    NavSection::Namespaces,
    NavSection::Cluster,
    NavSection::Federation,
    NavSection::Governor,
    NavSection::BackupRestore,
    NavSection::MetricsLogs,
    NavSection::ConfigurationStudio,
    NavSection::TableExplorer,
    NavSection::ReportBuilder,
    NavSection::Notebook,
    NavSection::RAGWorkspace,
    NavSection::SecurityCenter,
    NavSection::DocumentWorkspace,
    NavSection::IntegratedTerminal,
    NavSection::Monitoring,
    NavSection::Settings,
    NavSection::FileBrowser,
    NavSection::Help,
];

pub const NAV_NAMES: &[&str] = &[
    "Dashboard",
    "Query Console",
    "DB & Engines",
    "Namespaces",
    "Cluster",
    "Federation",
    "Governor",
    "Backup/Restore",
    "Metrics & Logs",
    "Config Studio",
    "Table Explorer",
    "Report Builder",
    "Notebook",
    "RAG Workspace",
    "Security Center",
    "Document Editor",
    "Terminal",
    "Monitoring",
    "Settings",
    "File Browser",
    "Help",
];

impl NavSection {
    pub fn prev(self) -> Self {
        let idx = NAV_SECTIONS.iter().position(|s| *s == self).unwrap_or(0);
        if idx == 0 {
            NAV_SECTIONS[NAV_SECTIONS.len() - 1]
        } else {
            NAV_SECTIONS[idx - 1]
        }
    }

    pub fn next(self) -> Self {
        let idx = NAV_SECTIONS.iter().position(|s| *s == self).unwrap_or(0);
        if idx >= NAV_SECTIONS.len() - 1 {
            NAV_SECTIONS[0]
        } else {
            NAV_SECTIONS[idx + 1]
        }
    }

    pub fn name(self) -> &'static str {
        let idx = NAV_SECTIONS.iter().position(|s| *s == self).unwrap_or(0);
        NAV_NAMES[idx]
    }

    pub fn count() -> usize {
        NAV_SECTIONS.len()
    }

    pub fn from_index(index: usize) -> Self {
        if index < NAV_SECTIONS.len() {
            NAV_SECTIONS[index]
        } else {
            NAV_SECTIONS[0]
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    None,
    Quit,
    Disconnect,
    BackupDelete,
    RestoreBackup,
    DropDatabase,
    DeleteNamespace,
    DropTable,
    DeleteFile,
    Confirm(String, String),
}

#[derive(Debug, Clone)]
pub struct QueryHistoryEntry {
    pub query: String,
    pub result: String,
    pub timestamp: String,
}

impl QueryHistoryEntry {
    pub fn new(query: String, result: String) -> Self {
        use chrono::Local;
        Self {
            query,
            result,
            timestamp: Local::now().format("%H:%M:%S").to_string(),
        }
    }
}

pub struct TuiApp {
    pub current_section: NavSection,
    pub config: TuiConfig,
    pub connected_url: Option<String>,
    pub auth_token: Option<String>,
    pub active_namespace: Option<String>,
    pub selected_database: Option<String>,

    // Server data
    pub instances: Vec<InstanceInfo>,
    pub health_status: Option<String>,
    pub server_version: Option<String>,
    pub uptime: Option<String>,
    pub server_role: Option<String>,
    pub engine_list: Vec<String>,
    pub error_message: Option<String>,

    // Query console
    pub query_input: String,
    pub query_scroll: usize,
    pub query_results: Vec<String>,
    pub query_history: Vec<QueryHistoryEntry>,
    pub query_history_pos: Option<usize>,
    pub show_query_history: bool,
    pub query_history_search: String,
    pub query_history_selection: usize,

    // Events & state
    pub event_log: Vec<String>,
    pub show_instances: bool,
    pub selected_instance: usize,
    pub selected_table_index: usize,
    pub should_quit: bool,
    pub last_refresh: Instant,

    // Data caches
    pub status_data: Option<serde_json::Value>,
    pub metrics_data: Option<String>,
    pub logs_data: Option<String>,
    pub metrics_logs_mode: MetricsLogsMode,
    pub log_level_filter: String,
    pub log_module_filter: String,
    pub logs_filtered: Option<String>,
    pub backups_data: Vec<String>,
    pub backups_detail: Option<serde_json::Value>,
    pub cluster_status: Option<serde_json::Value>,
    pub cluster_nodes: Option<serde_json::Value>,
    pub cluster_health: Option<serde_json::Value>,
    pub cluster_events: Vec<String>,
    pub selected_node_index: usize,
    pub cluster_modal: ClusterModal,
    pub cluster_join_input: String,
    pub cluster_status_msg: String,
    pub databases_data: Vec<String>,
    pub namespaces_data: Vec<String>,
    pub tables_data: Vec<String>,
    pub vector_indexes_data: Vec<String>,
    pub graph_data: Option<serde_json::Value>,
    pub aiml_data: Option<serde_json::Value>,
    pub users_data: Option<serde_json::Value>,
    pub roles_data: Option<serde_json::Value>,
    pub diagnostics_data: Option<String>,
    pub settings_data: Option<serde_json::Value>,

    // Federation
    pub federation_status: Option<serde_json::Value>,
    pub federation_clusters: Option<serde_json::Value>,
    pub federation_domains: Option<serde_json::Value>,
    pub federation_mode: FederationMode,
    pub federation_input: String,

    // Command palette
    pub show_command_palette: bool,
    pub command_input: String,
    pub command_palette_selection: usize,
    pub command_palette_filtered: Vec<String>,

    // Onboarding
    pub onboarding_mode: bool,
    pub onboarding_step: u8,

    // Confirmation dialog
    pub confirm_action: ConfirmAction,
    pub confirm_message: String,
    pub pending_action: Option<String>,

    // Loading / discovery
    pub discovery_done: bool,
    pub loading: bool,
    pub loading_message: String,

    // Backup / migration / governor
    pub backup_in_progress: bool,
    pub backup_progress_message: String,
    pub backup_operation_start: Option<Instant>,
    pub export_phase: String,
    pub export_progress: u8,
    pub export_status_line: String,
    pub engine_metrics: Option<String>,
    pub query_rate: Option<String>,
    pub error_rate: Option<String>,
    pub memory_usage: Option<String>,
    pub storage_usage: Option<String>,

    // Databases & Engines
    pub engines_mode: DatabasesEnginesMode,
    pub engines_detail: Option<serde_json::Value>,

    // Create Database Wizard
    pub db_wizard_name: String,
    pub db_wizard_engine: usize,
    pub db_wizard_storage_path: String,
    pub db_wizard_step: u8,
    pub db_wizard_error: Option<String>,
    pub db_wizard: panels::create_db_wizard::CreateDbWizard,

    pub migration_wizard_active: bool,
    pub migration_step: u8,
    pub migration_source: String,
    pub migration_url: String,
    pub migration_namespace: String,
    pub migration_mode: String,
    pub migration_source_connected: bool,
    pub migration_objects: Vec<String>,
    pub migration_selected_objects: Vec<bool>,
    pub migration_plan: String,
    pub migration_report: String,
    pub migration_progress: u8,
    pub migration_status: String,
    pub migration_dry_run_result: Option<String>,
    pub migration_error: Option<String>,

    pub governor_status: Option<serde_json::Value>,
    pub governor_executions: Vec<String>,
    pub governor_violations: Vec<String>,
    pub governor_metrics: Option<serde_json::Value>,
    pub governor_mode: GovernorMode,
    pub governor_policy_name: String,
    pub governor_policy_input: String,

    // Mouse state
    pub mouse_enabled: bool,
    pub hovered_section: Option<NavSection>,
    pub show_event_log: bool,
    pub tick_count: u64,

    // Contextual help
    pub show_contextual_help: bool,

    // CLI equivalent hints
    pub show_cli_hints: bool,

    // Config Studio (v1.3.2-alpha)
    pub config_entries: Vec<serde_json::Value>,
    pub config_selected_index: usize,
    pub config_mode: ConfigStudioMode,
    pub config_snapshots: Vec<serde_json::Value>,
    pub config_input: String,
    pub config_error: Option<String>,
    pub config_status: String,
    pub config_detail_entry: Option<serde_json::Value>,
    pub config_scroll: usize,

    // Table Explorer (v1.3.2-alpha)
    pub table_explorer_mode: TableExplorerMode,
    pub explorer_storage_types: Vec<String>,
    pub explorer_selected_st_index: usize,
    pub explorer_selected_st: Option<String>,
    pub explorer_tables_data: Option<serde_json::Value>,
    pub explorer_selected_table_index: usize,
    pub explorer_table_info: Option<serde_json::Value>,
    pub explorer_rows_data: Option<serde_json::Value>,
    pub explorer_row_offset: u64,
    pub explorer_row_limit: u64,
    pub explorer_row_total: u64,
    pub explorer_error: Option<String>,
    pub explorer_status: String,
    pub explorer_table_input: String,
    pub explorer_selected_row_index: usize,
    pub explorer_insert_input: String,
    pub explorer_analyze_result: Option<String>,

    // Report Builder (v1.3.2-alpha)
    pub report_mode: ReportBuilderMode,
    pub reports_data: Vec<serde_json::Value>,
    pub report_selected_index: usize,
    pub report_detail: Option<serde_json::Value>,
    pub report_results: Option<serde_json::Value>,
    pub report_input: String,
    pub report_input_field: u8,
    pub report_error: Option<String>,
    pub report_status: String,
    pub report_edit_id: Option<String>,

    // Notebook (v1.3.2-alpha)
    pub notebook_mode: NotebookBuilderMode,
    pub notebooks_data: Vec<serde_json::Value>,
    pub notebook_selected_index: usize,
    pub notebook_detail: Option<serde_json::Value>,
    pub notebook_cells: Vec<serde_json::Value>,
    pub notebook_selected_cell: usize,
    pub notebook_cell_edit: String,
    pub notebook_cell_result: Option<serde_json::Value>,
    pub notebook_error: Option<String>,
    pub notebook_status: String,

    // RAG Workspace (v1.3.2-alpha)
    pub rag_mode: RagWorkspaceMode,
    pub rag_collections: Vec<String>,
    pub rag_selected_index: usize,
    pub rag_query_text: String,
    pub rag_limit: usize,
    pub rag_results: Option<serde_json::Value>,
    pub rag_error: Option<String>,
    pub rag_input: String,

    // Settings (v1.3.2-alpha)
    pub settings_mode: SettingsMode,
    pub settings_input: String,
    pub doctor_results: Vec<String>,

    // ── Session Manager ─────────────────────────────────────────────
    pub sessions: Vec<SessionInfo>,
    pub active_session: usize,
    pub show_session_switcher: bool,

    // ── Search Everywhere ───────────────────────────────────────────
    pub show_search: bool,
    pub search_input: String,
    pub search_results: Vec<String>,
    pub search_selection: usize,
    pub search_scope: SearchScope,

    // ── Document Workspace ──────────────────────────────────────────
    pub doc_collections: Vec<String>,
    pub doc_collection_selected: usize,
    pub doc_documents: Vec<String>,
    pub doc_selected_index: usize,
    pub doc_current_json: String,
    pub doc_edit_buffer: String,
    pub doc_validation_error: Option<String>,
    pub doc_mode: DocEditorMode,
    pub doc_status: String,

    // ── Security Center ─────────────────────────────────────────────
    pub sec_users: Vec<serde_json::Value>,
    pub sec_roles: Vec<serde_json::Value>,
    pub sec_permissions: Vec<serde_json::Value>,
    pub sec_selected_index: usize,
    pub sec_mode: SecurityCenterMode,
    pub sec_input: String,
    pub sec_status: String,
    pub sec_error: Option<String>,
    pub sec_role_checklist: Vec<(String, bool)>,

    // ── Integrated Terminal ─────────────────────────────────────────
    pub terminal_input: String,
    pub terminal_output: Vec<String>,
    pub terminal_history: Vec<String>,
    pub terminal_history_pos: usize,
    pub terminal_cwd: String,
    pub terminal_scroll: usize,

    // ── Monitoring ──────────────────────────────────────────────────
    pub mon_alerts: Vec<serde_json::Value>,
    pub mon_metrics_history: Vec<(String, f64)>,
    pub mon_query_latency: Option<String>,
    pub mon_replication_lag: Option<String>,
    pub mon_resource_util: Option<serde_json::Value>,
    pub mon_mode: MonitoringMode,

    // File Browser
    pub file_mode: FileBrowserMode,
    pub file_current_dir: String,
    pub file_entries: Vec<String>,
    pub file_selected_index: usize,
    pub file_selected_path: String,
    pub file_content: Option<String>,
    pub file_scroll: usize,

    /// Workspace registry — maps NavSection to Workspace trait objects.
    pub workspaces: HashMap<NavSection, Box<dyn Workspace>>,
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub url: String,
    pub namespace: Option<String>,
    pub database: Option<String>,
    pub connected: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchScope {
    #[default]
    All,
    Commands,
    Objects,
    Sections,
    Capabilities,
    Files,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocEditorMode {
    View,
    Edit,
    Create,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityCenterMode {
    Users,
    Roles,
    Permissions,
    UserDetail,
    RoleDetail,
    CreateUser,
    CreateRole,
    AssignRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitoringMode {
    Overview,
    Alerts,
    Performance,
    Replication,
    Resources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterModal {
    None,
    ConfirmStart,
    ConfirmStop,
    ConfirmRestart,
    ConfirmLeave,
    ConfirmRemoveNode,
    JoinPrompt,
    MaintenanceToggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsLogsMode {
    Metrics,
    Logs,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableExplorerMode {
    StorageTypeSelect,
    TableList,
    TableDetail,
    RowBrowser,
    RowInsert,
    ConfirmDelete,
    ExportOptions,
    AnalyzeTable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportBuilderMode {
    List,
    Detail,
    Create,
    Edit,
    ConfirmDelete,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RagWorkspaceMode {
    CollectionSelect,
    CreateCollection,
    ConfirmDelete,
    SearchConfig,
    SearchResults,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsMode {
    View,
    EditRefreshInterval,
    ToggleMouse,
    EditEndpoint,
    EditToken,
    EditTheme,
    EditSafeMode,
    Doctor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernorMode {
    View,
    SetPolicy,
    ConfirmDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FederationMode {
    View,
    AddCluster,
    RemoveCluster,
    CreateDomain,
    DeleteDomain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotebookBuilderMode {
    List,
    Detail,
    CellEdit,
    CellTypeSelect,
    ConfirmDelete,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabasesEnginesMode {
    List,
    Inspect,
    ConfirmDelete,
    CreateDatabase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileBrowserMode {
    Browse,
    ReadFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigStudioMode {
    List,
    Detail,
    Edit,
    NewEntry,
    Snapshots,
    CreateSnapshot,
    ConfirmDelete,
    ImportExport,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            current_section: NavSection::Dashboard,
            config: TuiConfig::default(),
            connected_url: None,
            auth_token: None,
            active_namespace: None,
            selected_database: None,
            instances: Vec::new(),
            health_status: None,
            server_version: None,
            uptime: None,
            server_role: None,
            engine_list: Vec::new(),
            error_message: None,
            query_input: String::new(),
            query_scroll: 0,
            query_results: Vec::new(),
            query_history: Vec::new(),
            query_history_pos: None,
            show_query_history: false,
            query_history_search: String::new(),
            query_history_selection: 0,
            event_log: Vec::new(),
            show_instances: false,
            selected_instance: 0,
            selected_table_index: 0,
            should_quit: false,
            last_refresh: Instant::now(),
            status_data: None,
            metrics_data: None,
            logs_data: None,
            metrics_logs_mode: MetricsLogsMode::Both,
            log_level_filter: String::new(),
            log_module_filter: String::new(),
            logs_filtered: None,
            backups_data: Vec::new(),
            backups_detail: None,
            cluster_status: None,
            cluster_nodes: None,
            cluster_health: None,
            cluster_events: Vec::new(),
            selected_node_index: 0,
            cluster_modal: ClusterModal::None,
            cluster_join_input: String::new(),
            cluster_status_msg: String::new(),
            databases_data: Vec::new(),
            namespaces_data: Vec::new(),
            tables_data: Vec::new(),
            vector_indexes_data: Vec::new(),
            graph_data: None,
            aiml_data: None,
            users_data: None,
            roles_data: None,
            diagnostics_data: None,
            settings_data: None,
            federation_status: None,
            federation_clusters: None,
            federation_domains: None,
            federation_mode: FederationMode::View,
            federation_input: String::new(),
            show_command_palette: false,
            command_input: String::new(),
            command_palette_selection: 0,
            command_palette_filtered: Vec::new(),
            onboarding_mode: false,
            onboarding_step: 0,
            confirm_action: ConfirmAction::None,
            confirm_message: String::new(),
            pending_action: None,
            discovery_done: false,
            loading: true,
            loading_message: "Connecting...".to_string(),
            backup_in_progress: false,
            backup_progress_message: String::new(),
            backup_operation_start: None,
            export_phase: String::new(),
            export_progress: 0,
            export_status_line: String::new(),
            engine_metrics: None,
            query_rate: None,
            error_rate: None,
            memory_usage: None,
            storage_usage: None,
            engines_mode: DatabasesEnginesMode::List,
            engines_detail: None,
            db_wizard_name: String::new(),
            db_wizard_engine: 0,
            db_wizard_storage_path: String::new(),
            db_wizard_step: 0,
            db_wizard_error: None,
            db_wizard: panels::create_db_wizard::CreateDbWizard::new(),
            migration_wizard_active: false,
            migration_step: 0,
            migration_source: String::new(),
            migration_url: String::new(),
            migration_namespace: String::new(),
            migration_mode: String::new(),
            migration_source_connected: false,
            migration_objects: Vec::new(),
            migration_selected_objects: Vec::new(),
            migration_plan: String::new(),
            migration_report: String::new(),
            migration_progress: 0,
            migration_status: String::new(),
            migration_dry_run_result: None,
            migration_error: None,
            governor_status: None,
            governor_executions: Vec::new(),
            governor_violations: Vec::new(),
            governor_metrics: None,
            governor_mode: GovernorMode::View,
            governor_policy_name: String::new(),
            governor_policy_input: String::new(),
            mouse_enabled: true,
            hovered_section: None,
            show_event_log: false,
            tick_count: 0,
            show_contextual_help: false,
            show_cli_hints: true,
            config_entries: Vec::new(),
            config_selected_index: 0,
            config_mode: ConfigStudioMode::List,
            config_snapshots: Vec::new(),
            config_input: String::new(),
            config_error: None,
            config_status: String::new(),
            config_detail_entry: None,
            config_scroll: 0,

            table_explorer_mode: TableExplorerMode::StorageTypeSelect,
            explorer_storage_types: Vec::new(),
            explorer_selected_st_index: 0,
            explorer_selected_st: None,
            explorer_tables_data: None,
            explorer_selected_table_index: 0,
            explorer_table_info: None,
            explorer_rows_data: None,
            explorer_row_offset: 0,
            explorer_row_limit: 50,
            explorer_row_total: 0,
            explorer_error: None,
            explorer_status: String::new(),
            explorer_table_input: String::new(),
            explorer_selected_row_index: 0,
            explorer_insert_input: String::new(),
            explorer_analyze_result: None,

            report_mode: ReportBuilderMode::List,
            reports_data: Vec::new(),
            report_selected_index: 0,
            report_detail: None,
            report_results: None,
            report_input: String::new(),
            report_input_field: 0,
            report_error: None,
            report_status: String::new(),
            report_edit_id: None,

            notebook_mode: NotebookBuilderMode::List,
            notebooks_data: Vec::new(),
            notebook_selected_index: 0,
            notebook_detail: None,
            notebook_cells: Vec::new(),
            notebook_selected_cell: 0,
            notebook_cell_edit: String::new(),
            notebook_cell_result: None,
            notebook_error: None,
            notebook_status: String::new(),

            rag_mode: RagWorkspaceMode::CollectionSelect,
            rag_collections: Vec::new(),
            rag_selected_index: 0,
            rag_query_text: String::new(),
            rag_limit: 10,
            rag_results: None,
            rag_error: None,
            rag_input: String::new(),

            settings_mode: SettingsMode::View,
            settings_input: String::new(),
            doctor_results: Vec::new(),

            sessions: vec![SessionInfo {
                id: "default".to_string(),
                url: String::new(),
                namespace: None,
                database: None,
                connected: false,
            }],
            active_session: 0,
            show_session_switcher: false,

            show_search: false,
            search_input: String::new(),
            search_results: Vec::new(),
            search_selection: 0,
            search_scope: SearchScope::All,

            doc_collections: Vec::new(),
            doc_collection_selected: 0,
            doc_documents: Vec::new(),
            doc_selected_index: 0,
            doc_current_json: String::new(),
            doc_edit_buffer: String::new(),
            doc_validation_error: None,
            doc_mode: DocEditorMode::View,
            doc_status: String::new(),

            sec_users: Vec::new(),
            sec_roles: Vec::new(),
            sec_permissions: Vec::new(),
            sec_selected_index: 0,
            sec_mode: SecurityCenterMode::Users,
            sec_input: String::new(),
            sec_status: String::new(),
            sec_error: None,
            sec_role_checklist: Vec::new(),

            terminal_input: String::new(),
            terminal_output: vec![
                "PrimusDB Terminal — type commands or press Enter to execute".to_string(),
            ],
            terminal_history: Vec::new(),
            terminal_history_pos: 0,
            terminal_cwd: String::new(),
            terminal_scroll: 0,

            mon_alerts: Vec::new(),
            mon_metrics_history: Vec::new(),
            mon_query_latency: None,
            mon_replication_lag: None,
            mon_resource_util: None,
            mon_mode: MonitoringMode::Overview,

            file_mode: FileBrowserMode::Browse,
            file_current_dir: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            file_entries: Vec::new(),
            file_selected_index: 0,
            file_selected_path: String::new(),
            file_content: None,
            file_scroll: 0,

            workspaces: crate::cli::tui::workspaces::create_all_workspaces(),
        }
    }

    pub fn add_event(&mut self, msg: String) {
        self.event_log.push(msg);
        if self.event_log.len() > 100 {
            self.event_log.remove(0);
        }
    }

    pub fn refresh_data(
        &mut self,
        _tx: &tokio::sync::mpsc::UnboundedSender<crate::cli::tui::event::AppMessage>,
    ) {
        self.add_event("Refreshing...".to_string());
    }

    pub fn handle_command(
        &mut self,
        cmd: &str,
        tx: &tokio::sync::mpsc::UnboundedSender<crate::cli::tui::event::AppMessage>,
    ) {
        use crate::cli::tui::event::AppMessage;
        if let Some(action) = self.execute_command(cmd) {
            let _ = tx.send(AppMessage::ExecuteAction(action));
        } else {
            // Raw action string from workspaces (e.g., "db_create_full:name:desc:engines")
            let _ = tx.send(AppMessage::ExecuteAction(cmd.to_string()));
        }
    }

    pub fn connected(&self) -> bool {
        self.connected_url.is_some()
    }

    pub fn connect_url(&mut self, url: &str) {
        self.connected_url = Some(url.to_string());
        self.add_event(format!("Connected to {}", url));
    }

    pub fn disconnect(&mut self) {
        self.connected_url = None;
        self.auth_token = None;
        self.active_namespace = None;
        self.selected_database = None;
        self.status_data = None;
        self.health_status = None;
        self.server_version = None;
        self.uptime = None;
        self.server_role = None;
        self.engine_list.clear();
        self.databases_data.clear();
        self.namespaces_data.clear();
        self.tables_data.clear();
        self.vector_indexes_data.clear();
        self.graph_data = None;
        self.aiml_data = None;
        self.users_data = None;
        self.roles_data = None;
        self.diagnostics_data = None;
        self.settings_data = None;
        self.cluster_nodes = None;
        self.cluster_status = None;
        self.cluster_health = None;
        self.cluster_events.clear();
        self.selected_node_index = 0;
        self.cluster_modal = ClusterModal::None;
        self.cluster_join_input.clear();
        self.cluster_status_msg.clear();
        self.federation_status = None;
        self.federation_clusters = None;
        self.federation_domains = None;
        self.federation_mode = FederationMode::View;
        self.governor_executions.clear();
        self.governor_violations.clear();
        self.governor_metrics = None;
        self.governor_mode = GovernorMode::View;
        self.config_entries.clear();
        self.config_snapshots.clear();
        self.config_mode = ConfigStudioMode::List;
        self.config_selected_index = 0;
        self.config_error = None;
        self.config_status.clear();
        self.config_detail_entry = None;

        self.engines_mode = DatabasesEnginesMode::List;
        self.engines_detail = None;

        self.table_explorer_mode = TableExplorerMode::StorageTypeSelect;
        self.explorer_storage_types.clear();
        self.explorer_selected_st_index = 0;
        self.explorer_selected_st = None;
        self.explorer_tables_data = None;
        self.explorer_selected_table_index = 0;
        self.explorer_table_info = None;
        self.explorer_rows_data = None;
        self.explorer_row_offset = 0;
        self.explorer_row_limit = 50;
        self.explorer_row_total = 0;
        self.explorer_error = None;
        self.explorer_status.clear();
        self.explorer_table_input.clear();
        self.explorer_selected_row_index = 0;
        self.explorer_insert_input.clear();
        self.explorer_analyze_result = None;

        self.report_mode = ReportBuilderMode::List;
        self.reports_data.clear();
        self.report_selected_index = 0;
        self.report_detail = None;
        self.report_results = None;
        self.report_input.clear();
        self.report_input_field = 0;
        self.report_error = None;
        self.report_status.clear();
        self.report_edit_id = None;

        self.notebook_mode = NotebookBuilderMode::List;
        self.notebooks_data.clear();
        self.notebook_selected_index = 0;
        self.notebook_detail = None;
        self.notebook_cells.clear();
        self.notebook_selected_cell = 0;
        self.notebook_cell_edit.clear();
        self.notebook_cell_result = None;
        self.notebook_error = None;
        self.notebook_status.clear();

        self.rag_mode = RagWorkspaceMode::CollectionSelect;
        self.rag_collections.clear();
        self.rag_selected_index = 0;
        self.rag_query_text.clear();
        self.rag_limit = 10;
        self.rag_results = None;
        self.rag_error = None;
        self.rag_input.clear();

        self.settings_mode = SettingsMode::View;
        self.settings_input.clear();

        self.doc_collections.clear();
        self.doc_documents.clear();
        self.doc_current_json.clear();
        self.doc_edit_buffer.clear();
        self.doc_validation_error = None;
        self.doc_mode = DocEditorMode::View;
        self.doc_status.clear();

        self.sec_users.clear();
        self.sec_roles.clear();
        self.sec_permissions.clear();
        self.sec_selected_index = 0;
        self.sec_mode = SecurityCenterMode::Users;
        self.sec_input.clear();
        self.sec_status.clear();
        self.sec_error = None;
        self.sec_role_checklist.clear();

        self.mon_alerts.clear();
        self.mon_metrics_history.clear();
        self.mon_query_latency = None;
        self.mon_replication_lag = None;
        self.mon_resource_util = None;
        self.mon_mode = MonitoringMode::Overview;

        self.file_mode = FileBrowserMode::Browse;
        self.file_current_dir = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        self.file_entries.clear();
        self.file_selected_index = 0;
        self.file_selected_path.clear();
        self.file_content = None;
        self.file_scroll = 0;

        self.query_history_pos = None;
        self.show_query_history = false;
        self.query_history_search.clear();
        self.query_history_selection = 0;

        self.metrics_logs_mode = MetricsLogsMode::Both;
        self.log_level_filter.clear();
        self.log_module_filter.clear();
        self.logs_filtered = None;

        self.add_event("Disconnected".to_string());
    }

    pub fn apply_status(&mut self, value: Option<serde_json::Value>) {
        self.status_data = value.clone();
        if let Some(ref v) = value {
            self.health_status = v
                .get("status")
                .and_then(|s| s.as_str().map(|s| s.to_string()));
            self.server_version = v
                .get("version")
                .and_then(|s| s.as_str().map(|s| s.to_string()));
            self.uptime = v
                .get("uptime_seconds")
                .and_then(|u| u.as_u64())
                .map(|secs| {
                    let h = secs / 3600;
                    let m = (secs % 3600) / 60;
                    let s = secs % 60;
                    format!("{:02}:{:02}:{:02}", h, m, s)
                });
            self.engine_list = v
                .get("enabled_engines")
                .and_then(|e| e.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|e| e.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .or_else(|| {
                    v.get("storage_engines")
                        .and_then(|e| e.as_object())
                        .map(|map| map.keys().cloned().collect())
                })
                .unwrap_or_default();
            self.server_role = v
                .get("role")
                .and_then(|r| r.as_str().map(|s| s.to_string()))
                .or_else(|| {
                    // Detect standalone mode from cluster config
                    match v.get("cluster_mode").and_then(|c| c.as_bool()) {
                        Some(false) => Some("standalone".to_string()),
                        _ => None,
                    }
                });
        } else {
            self.health_status = None;
            self.server_version = None;
            self.uptime = None;
            self.server_role = None;
            self.engine_list.clear();
        }
    }

    pub fn command_palette_items() -> Vec<&'static str> {
        vec![
            ":help - Show help",
            ":quit - Quit PrimusDB TUI",
            ":refresh - Refresh current view",
            ":connect <url> - Connect to server",
            ":disconnect - Disconnect from server",
            ":dashboard - Go to Dashboard",
            ":query - Go to Query Console",
            ":cluster - Go to Cluster view",
            ":federation - Go to Federation view",
            ":governor - Go to Resource Governor",
            ":settings - Go to Settings",
            ":namespace - Go to Namespaces",
            ":databases - Go to Databases & Engines",
            ":backup - Go to Backup/Restore",
            ":metrics - Go to Metrics & Logs",
            ":config - Go to Config Studio",
            ":tables - Go to Table Explorer",
            ":reports - Go to Report Builder",
            ":notebook - Go to Notebook",
            ":rag - Go to RAG Workspace",
            ":rag create <name> - Create a RAG collection",
            ":rag delete <name> - Delete a RAG collection",
            ":user role <username> <role> - Assign role to user",
            ":security - Go to Security Center",
            ":document - Go to Document Editor",
            ":terminal - Go to Integrated Terminal",
            ":monitor - Go to Monitoring",
            ":search - Search everywhere",
            ":session next - Switch to next session",
            ":session prev - Switch to previous session",
            ":sessions - Show session manager",
            ":doctor - Run diagnostics on the current connection",
            ":backup create - Create a new backup",
            ":backup verify <id> - Verify backup integrity",
            ":export data <st> <table> - Export table data as JSON",
            ":export query <file> - Save query result to file",
            ":import data <st> <table> - Import data into a table",
            ":status - Get server status",
            ":health - Get server health",
            ":events - Toggle event log viewer",
            ":reconnect - Reconnect to current server",
            ":clear - Clear results and event log",
            ":explorer - Go to Table Explorer",
            ":studio - Go to Configuration Studio",
            ":files - Go to File Browser",
            ":cluster start - Start cluster",
            ":cluster stop - Stop cluster",
            ":cluster restart - Restart cluster",
            ":cluster join <url> - Join a cluster",
            ":cluster leave - Leave the cluster",
            ":cluster node remove - Remove selected node",
            ":cluster maintenance - Toggle maintenance mode",
            ":column add <st> <table> <json> - Add a column",
            ":column drop <st> <table> <name> - Drop a column",
            ":column modify <st> <table> <json> - Modify a column",
            ":constraint add <st> <table> <json> - Add a constraint",
            ":constraint drop <st> <table> <name> - Drop a constraint",
            ":table rename <st> <table> <new_name> - Rename a table",
            ":table analyze <st> <table> - Analyze table statistics",
            ":doc create <db> <id> <json> - Create a new KV document",
            ":doc update <db> <id> <json> - Update (partial) a KV document",
            ":doc delete <db/id> - Delete a KV document",
            ":doc validate - Validate current JSON",
            ":terminal clear - Clear terminal output",
            ":export <format> - Export current data",
        ]
    }

    pub fn all_commands(&self) -> Vec<String> {
        let mut items: Vec<String> = Self::command_palette_items()
            .iter()
            .map(|s| s.to_string())
            .collect();
        // Extend with dynamically registered capabilities from the registry
        for entry in capability::registry().command_palette_entries() {
            if !items.contains(&entry) {
                items.push(entry);
            }
        }
        items
    }

    pub fn filter_commands(&self) -> Vec<String> {
        let input = self.command_input.trim();
        if input.is_empty() || input == ":" {
            return self.all_commands();
        }
        let query = input.trim_start_matches(':').to_lowercase();
        self.all_commands()
            .iter()
            .filter(|item| {
                let lower = item.to_lowercase();
                lower.contains(&query)
            })
            .map(|s| s.to_string())
            .collect()
    }

    pub fn execute_command(&mut self, cmd: &str) -> Option<String> {
        let cmd = cmd.trim().to_string();
        if cmd.is_empty() || cmd == ":" {
            return None;
        }
        let action = cmd.trim_start_matches(':').trim().to_lowercase();
        if action == "quit" || action == "q" {
            return Some("quit".to_string());
        }
        if action == "help" || action == "?" {
            self.current_section = NavSection::Help;
            return Some("help".to_string());
        }
        if action == "refresh" || action == "r" {
            return Some("refresh".to_string());
        }
        if action == "disconnect" {
            return Some("disconnect".to_string());
        }
        if action == "dashboard" {
            self.current_section = NavSection::Dashboard;
            return Some("dashboard".to_string());
        }
        if action == "query" {
            self.current_section = NavSection::QueryConsole;
            return Some("query".to_string());
        }
        if action == "cluster" {
            self.current_section = NavSection::Cluster;
            return Some("cluster".to_string());
        }
        if action == "federation" {
            self.current_section = NavSection::Federation;
            return Some("federation".to_string());
        }
        if action.starts_with("federation cluster add ") {
            let rest = action
                .trim_start_matches("federation cluster add ")
                .trim()
                .to_string();
            if !rest.is_empty() {
                let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    return Some(format!("fed_cluster_add:{} {}", parts[0], parts[1]));
                }
            }
        }
        if action.starts_with("federation cluster remove ") {
            let cluster_id = action
                .trim_start_matches("federation cluster remove ")
                .trim()
                .to_string();
            if !cluster_id.is_empty() {
                return Some(format!("fed_cluster_remove:{}", cluster_id));
            }
        }
        if action.starts_with("federation domain create ") {
            let rest = action
                .trim_start_matches("federation domain create ")
                .trim()
                .to_string();
            if !rest.is_empty() {
                return Some(format!("fed_domain_create:{}", rest));
            }
        }
        if action.starts_with("federation domain delete ") {
            let name = action
                .trim_start_matches("federation domain delete ")
                .trim()
                .to_string();
            if !name.is_empty() {
                return Some(format!("fed_domain_delete:{}", name));
            }
        }
        if action == "governor" {
            self.current_section = NavSection::Governor;
            return Some("governor".to_string());
        }
        if action.starts_with("governor set ") {
            let rest = action
                .trim_start_matches("governor set ")
                .trim()
                .to_string();
            if !rest.is_empty() {
                return Some(format!("governor_set:{}", rest));
            }
        }
        if action.starts_with("governor delete ") {
            let rest = action
                .trim_start_matches("governor delete ")
                .trim()
                .to_string();
            if !rest.is_empty() {
                return Some(format!("governor_delete:{}", rest));
            }
        }
        if action == "doctor" {
            return Some("doctor".to_string());
        }
        if action == "settings" {
            self.current_section = NavSection::Settings;
            return Some("settings".to_string());
        }
        if action == "namespace" {
            self.current_section = NavSection::Namespaces;
            return Some("namespace".to_string());
        }
        if action == "databases" {
            self.current_section = NavSection::DatabasesEngines;
            return Some("databases".to_string());
        }
        if action == "backup" {
            self.current_section = NavSection::BackupRestore;
            return Some("backup".to_string());
        }
        if action == "metrics" {
            self.current_section = NavSection::MetricsLogs;
            return Some("metrics".to_string());
        }
        if action == "config" {
            self.current_section = NavSection::ConfigurationStudio;
            return Some("config".to_string());
        }
        if action == "tables" {
            self.current_section = NavSection::TableExplorer;
            return Some("tables".to_string());
        }
        if action == "reports" {
            self.current_section = NavSection::ReportBuilder;
            return Some("reports".to_string());
        }
        if action == "notebook" {
            self.current_section = NavSection::Notebook;
            return Some("notebook".to_string());
        }
        if action == "rag" {
            self.current_section = NavSection::RAGWorkspace;
            return Some("rag".to_string());
        }
        if action.starts_with("rag create ") {
            let name = action.trim_start_matches("rag create ").trim().to_string();
            if !name.is_empty() {
                return Some(format!("rag_create:{}", name));
            }
        }
        if action.starts_with("rag delete ") {
            let name = action.trim_start_matches("rag delete ").trim().to_string();
            if !name.is_empty() {
                return Some(format!("rag_delete:{}", name));
            }
        }
        if action == "security" {
            self.current_section = NavSection::SecurityCenter;
            return Some("security".to_string());
        }
        if action == "document" {
            self.current_section = NavSection::DocumentWorkspace;
            return Some("document".to_string());
        }
        if action == "terminal" {
            self.current_section = NavSection::IntegratedTerminal;
            return Some("terminal".to_string());
        }
        if action == "monitor" {
            self.current_section = NavSection::Monitoring;
            return Some("monitor".to_string());
        }
        if action == "cluster start" {
            return Some("cluster_start".to_string());
        }
        if action == "cluster stop" {
            return Some("cluster_stop".to_string());
        }
        if action == "cluster restart" {
            return Some("cluster_restart".to_string());
        }
        if action.starts_with("cluster join ") {
            let url = action
                .trim_start_matches("cluster join ")
                .trim()
                .to_string();
            if !url.is_empty() {
                return Some(format!("cluster_join:{}", url));
            }
        }
        if action == "cluster leave" {
            return Some("cluster_leave".to_string());
        }
        if action == "cluster node remove" {
            return Some("cluster_node_remove".to_string());
        }
        if action == "cluster maintenance" {
            return Some("cluster_maintenance".to_string());
        }
        if action.starts_with("namespace create ") {
            let name = action
                .trim_start_matches("namespace create ")
                .trim()
                .to_string();
            if !name.is_empty() {
                return Some(format!("namespace_create:{}", name));
            }
        }
        if action.starts_with("namespace delete ") {
            let name = action
                .trim_start_matches("namespace delete ")
                .trim()
                .to_string();
            if !name.is_empty() {
                return Some(format!("namespace_delete:{}", name));
            }
        }
        if action.starts_with("namespace use ") {
            let name = action
                .trim_start_matches("namespace use ")
                .trim()
                .to_string();
            if !name.is_empty() {
                return Some(format!("namespace_use:{}", name));
            }
        }
        if action.starts_with("table create ") {
            let rest = action
                .trim_start_matches("table create ")
                .trim()
                .to_string();
            return Some(format!("table_create:{}", rest));
        }
        if action.starts_with("table drop ") {
            let rest = action.trim_start_matches("table drop ").trim().to_string();
            return Some(format!("table_drop:{}", rest));
        }
        if action.starts_with("db create ") {
            let rest = action.trim_start_matches("db create ").trim().to_string();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                return Some(format!("table_create:{}/{}", parts[0], parts[1]));
            }
        }
        if action.starts_with("db drop ") {
            let name = action.trim_start_matches("db drop ").trim().to_string();
            if !name.is_empty() {
                return Some(format!("table_drop:relational/{}", name));
            }
        }
        if action.starts_with("column add ") {
            let rest = action.trim_start_matches("column add ").trim().to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() >= 3 {
                return Some(format!(
                    "column_add:{} {} {}",
                    parts[0],
                    parts[1],
                    parts[2..].join(" ")
                ));
            }
        }
        if action.starts_with("column drop ") {
            let rest = action.trim_start_matches("column drop ").trim().to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() == 3 {
                return Some(format!(
                    "column_drop:{} {} {}",
                    parts[0], parts[1], parts[2]
                ));
            }
        }
        if action.starts_with("column modify ") {
            let rest = action
                .trim_start_matches("column modify ")
                .trim()
                .to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() >= 3 {
                return Some(format!(
                    "column_modify:{} {} {}",
                    parts[0],
                    parts[1],
                    parts[2..].join(" ")
                ));
            }
        }
        if action.starts_with("constraint add ") {
            let rest = action
                .trim_start_matches("constraint add ")
                .trim()
                .to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() >= 3 {
                return Some(format!(
                    "constraint_add:{} {} {}",
                    parts[0],
                    parts[1],
                    parts[2..].join(" ")
                ));
            }
        }
        if action.starts_with("constraint drop ") {
            let rest = action
                .trim_start_matches("constraint drop ")
                .trim()
                .to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() == 3 {
                return Some(format!(
                    "constraint_drop:{} {} {}",
                    parts[0], parts[1], parts[2]
                ));
            }
        }
        if action.starts_with("table rename ") {
            let rest = action
                .trim_start_matches("table rename ")
                .trim()
                .to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() == 3 {
                return Some(format!(
                    "table_rename:{} {} {}",
                    parts[0], parts[1], parts[2]
                ));
            }
        }
        if action.starts_with("table analyze ") {
            let rest = action
                .trim_start_matches("table analyze ")
                .trim()
                .to_string();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                return Some(format!("table_analyze:{} {}", parts[0], parts[1]));
            }
        }
        if action.starts_with("doc create ") {
            let rest = action.trim_start_matches("doc create ").trim().to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() == 3 {
                return Some(format!("doc_create:{} {} {}", parts[0], parts[1], parts[2]));
            }
        }
        if action.starts_with("doc update ") {
            let rest = action.trim_start_matches("doc update ").trim().to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() == 3 {
                return Some(format!("doc_update:{} {} {}", parts[0], parts[1], parts[2]));
            }
        }
        if action.starts_with("doc delete ") {
            let key = action.trim_start_matches("doc delete ").trim().to_string();
            if !key.is_empty() {
                return Some(format!("doc_delete:{}", key));
            }
        }
        if action == "user delete" && self.connected() {
            let idx = self.sec_selected_index;
            if let Some(user) = self.sec_users.get(idx) {
                let name = user.get("username").and_then(|u| u.as_str()).unwrap_or("");
                if !name.is_empty() {
                    return Some(format!("user_delete:{}", name));
                }
            }
        }
        if action.starts_with("user role ") {
            let rest = action.trim_start_matches("user role ").trim().to_string();
            if !rest.is_empty() {
                return Some(format!("user_role:{}", rest));
            }
        }
        if action == "role delete" && self.connected() {
            let idx = self.sec_selected_index;
            if let Some(role) = self.sec_roles.get(idx) {
                let name = role.get("name").and_then(|u| u.as_str()).unwrap_or("");
                if !name.is_empty() {
                    return Some(format!("role_delete:{}", name));
                }
            }
        }
        if action == "search" {
            self.show_search = true;
            return Some("search".to_string());
        }
        if action == "session next" && self.sessions.len() > 1 {
            self.active_session = (self.active_session + 1) % self.sessions.len();
            return Some("session_next".to_string());
        }
        if action == "session prev" && self.sessions.len() > 1 {
            self.active_session = if self.active_session == 0 {
                self.sessions.len() - 1
            } else {
                self.active_session - 1
            };
            return Some("session_prev".to_string());
        }
        if action == "sessions" {
            self.show_session_switcher = !self.show_session_switcher;
            return Some("sessions_toggle".to_string());
        }
        if action == "doc create" {
            self.doc_mode = DocEditorMode::Create;
            return Some("doc_create".to_string());
        }
        if action == "doc validate" {
            return Some("doc_validate".to_string());
        }
        if action == "terminal clear" {
            self.terminal_output.clear();
            return Some("terminal_clear".to_string());
        }
        if action.starts_with("export data ") {
            let rest = action.trim_start_matches("export data ").trim().to_string();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                return Some(format!("export_data:{} {}", parts[0], parts[1]));
            }
        }
        if action.starts_with("export query ") {
            let filepath = action
                .trim_start_matches("export query ")
                .trim()
                .to_string();
            if !filepath.is_empty() {
                return Some(format!("export_query:{}", filepath));
            }
        }
        if action.starts_with("import data ") {
            let rest = action.trim_start_matches("import data ").trim().to_string();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                return Some(format!("import_data:{} {}", parts[0], parts[1]));
            }
        }
        if action.starts_with("export ") {
            return Some(format!("export:{}", action.trim_start_matches("export ")));
        }
        if action == "backup create" {
            return Some("backup_create".to_string());
        }
        if action.starts_with("backup verify ") {
            let id = action
                .trim_start_matches("backup verify ")
                .trim()
                .to_string();
            if !id.is_empty() {
                return Some(format!("backup_verify:{}", id));
            }
        }
        if action == "status" {
            return Some("status".to_string());
        }
        if action == "health" {
            return Some("health".to_string());
        }
        if action == "events" {
            return Some("events_toggle".to_string());
        }
        if action == "reconnect" && self.connected() {
            let url = self.connected_url.clone().unwrap_or_default();
            return Some(format!("connect:{}", url));
        }
        if action == "clear" {
            self.query_results.clear();
            self.query_input.clear();
            self.query_scroll = 0;
            self.event_log.clear();
            return Some("clear".to_string());
        }
        if action == "explorer" {
            self.current_section = NavSection::TableExplorer;
            return Some("explorer".to_string());
        }
        if action == "studio" {
            self.current_section = NavSection::ConfigurationStudio;
            return Some("studio".to_string());
        }
        if action == "files" {
            self.current_section = NavSection::FileBrowser;
            return Some("files".to_string());
        }
        if action.starts_with("mkdir ") {
            let dir = action.trim_start_matches("mkdir ").trim().to_string();
            if !dir.is_empty() {
                let full_path = if dir.starts_with('/') {
                    dir
                } else {
                    format!("{}/{}", self.file_current_dir, dir)
                };
                return Some(format!("mkdir:{}", full_path));
            }
        }
        if action.starts_with("connect ") {
            let url = action.trim_start_matches("connect ").trim().to_string();
            return Some(format!("connect:{}", url));
        }
        // Fallback: check capability registry for any matching capability
        for cap in capability::registry().all() {
            if let CapabilityAction::Command(cmd_str) = &cap.action {
                let trimmed = cmd_str.trim_start_matches(':');
                if trimmed == action {
                    return Some(format!("capability:{}", cap.id));
                }
            }
        }
        None
    }

    pub fn cli_hint_for_current_section(&self) -> &'static str {
        match self.current_section {
            NavSection::Dashboard => "primusdb status",
            NavSection::QueryConsole => "primusdb query \"SELECT 1\"",
            NavSection::DatabasesEngines => "primusdb db list",
            NavSection::Namespaces => "primusdb namespace list",
            NavSection::Cluster => "primusdb cluster status",
            NavSection::Federation => "primusdb cluster status --federation",
            NavSection::Governor => "primusdb governor status",
            NavSection::BackupRestore => "primusdb backup list",
            NavSection::MetricsLogs => "primusdb metrics",
            NavSection::ConfigurationStudio => "primusdb config show",
            NavSection::TableExplorer => "primusdb db list",
            NavSection::ReportBuilder => "primusdb query",
            NavSection::Notebook => "primusdb query",
            NavSection::RAGWorkspace => "primusdb vector list (experimental)",
            NavSection::SecurityCenter => "primusdb auth users",
            NavSection::DocumentWorkspace => "primusdb query \"SELECT * FROM documents\"",
            NavSection::IntegratedTerminal => "primusdb --help",
            NavSection::Monitoring => "primusdb metrics",
            NavSection::Settings => "primusdb config show",
            NavSection::FileBrowser => "ls -la",
            NavSection::Help => "primusdb --help",
        }
    }

    pub fn contextual_help_text(&self) -> &'static str {
        match self.current_section {
            NavSection::Dashboard => "Dashboard: overview of server health, engines, and cluster status. ↑↓ to select instance, Enter to connect, 'r' to refresh.",
            NavSection::QueryConsole => "Query Console: type SQL and press Enter. ↑↓ history, H show history panel, PgUp/PgDn scroll results, Ctrl+S save results.",
            NavSection::DatabasesEngines => "Databases & Engines: list engines and databases. ↑↓ select, Enter show tables, n new, d drop, r refresh.",
            NavSection::Namespaces => "Namespaces: manage data isolation namespaces. Create, delete, and switch namespaces.",
            NavSection::Cluster => "Cluster: ↑↓ select node, s start, S stop, r restart, j join, l leave, m maintenance, d remove, y/n confirm, Enter inspect.",
            NavSection::Federation => "Federation (experimental): cross-cluster DataDomains, federated clusters, and balance plans.",
            NavSection::Governor => "Resource Governor: monitor executions, enforce policies, view violations and metrics snapshots.",
            NavSection::BackupRestore => "Backup/Restore: create, list, verify, and restore backups. Ctrl+B to create.",
            NavSection::MetricsLogs => "Metrics & Logs: Prometheus metrics viewer and system log tail. Filters available.",
            NavSection::ConfigurationStudio => "Config Studio: edit, validate, export/import, and snapshot server config. Press 'e' to edit, 'n' for new entry, 's' for snapshots.",
            NavSection::TableExplorer => "Table Explorer: list tables/collections, view schemas, browse rows, export results.",
            NavSection::ReportBuilder => "Report Builder: saved report definitions, query-based reports, export to CSV/JSON. Press 'n' to create, Enter to run.",
            NavSection::Notebook => "Notebook: cells (markdown, SQL, analysis, RAG), execute, save to system database.",
            NavSection::RAGWorkspace => "RAG Workspace: select vector collection, enter query text, configure top-k, and execute similarity search.",
            NavSection::SecurityCenter => "Security Center: manage users, roles, and permissions. Create, edit, and delete RBAC policies.",
            NavSection::DocumentWorkspace => "Document Editor: create, view, edit, and validate JSON documents. Supports patch mode and schema validation.",
            NavSection::IntegratedTerminal => "Terminal: execute shell commands directly inside the TUI. Supports command history and working directory.",
            NavSection::Monitoring => "Monitoring: live operational metrics, alerts, query latency, replication lag, and resource utilization trends.",
            NavSection::Settings => "Settings: configure endpoint, auth token, refresh interval, theme, mouse, and output mode.",
            NavSection::FileBrowser => "File Browser: browse local filesystem. ↑↓ select, Enter open dir/file, Esc go up, d delete, r refresh, h go home.",
            NavSection::Help => "Help: keyboard shortcuts, command palette reference, CLI equivalents, and documentation links.",
        }
    }
}
