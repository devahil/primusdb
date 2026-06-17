use crate::cli::discovery::InstanceInfo;
use std::time::Instant;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SIDEBAR_WIDTH: u16 = 22;
pub const HEADER_HEIGHT: u16 = 1;
pub const INPUT_HEIGHT: u16 = 3;
pub const STATUS_HEIGHT: u16 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavSection {
    Dashboard,
    Instances,
    Clusters,
    Nodes,
    Engines,
    Databases,
    Namespaces,
    TablesCollections,
    VectorIndexes,
    Graph,
    AIML,
    Queries,
    Backups,
    Restores,
    Migrations,
    Users,
    Roles,
    Metrics,
    Logs,
    Diagnostics,
    Settings,
    Help,
    Governor,
}

pub const NAV_SECTIONS: &[NavSection] = &[
    NavSection::Dashboard,
    NavSection::Instances,
    NavSection::Clusters,
    NavSection::Nodes,
    NavSection::Engines,
    NavSection::Databases,
    NavSection::Namespaces,
    NavSection::TablesCollections,
    NavSection::VectorIndexes,
    NavSection::Graph,
    NavSection::AIML,
    NavSection::Queries,
    NavSection::Backups,
    NavSection::Restores,
    NavSection::Migrations,
    NavSection::Users,
    NavSection::Roles,
    NavSection::Metrics,
    NavSection::Logs,
    NavSection::Diagnostics,
    NavSection::Settings,
    NavSection::Help,
    NavSection::Governor,
];

pub const NAV_NAMES: &[&str] = &[
    "Dashboard",
    "Instances",
    "Clusters",
    "Nodes",
    "Engines",
    "Databases",
    "Namespaces",
    "Tables",
    "Vectors",
    "Graph",
    "AI/ML",
    "Queries",
    "Backups",
    "Restores",
    "Migrations",
    "Users",
    "Roles",
    "Metrics",
    "Logs",
    "Diagnostics",
    "Settings",
    "Help",
    "Governor",
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
}

#[allow(dead_code)]
pub struct TuiApp {
    pub current_section: NavSection,
    pub connected_url: Option<String>,
    pub instances: Vec<InstanceInfo>,
    pub health_status: Option<String>,
    pub server_version: Option<String>,
    pub uptime: Option<String>,
    pub engine_list: Vec<String>,
    pub error_message: Option<String>,
    pub query_input: String,
    pub query_scroll: usize,
    pub query_results: Vec<String>,
    pub event_log: Vec<String>,
    pub show_instances: bool,
    pub selected_instance: usize,
    pub should_quit: bool,
    pub last_refresh: Instant,
    pub status_data: Option<serde_json::Value>,
    pub metrics_data: Option<String>,
    pub logs_data: Option<String>,
    pub backups_data: Vec<String>,
    pub backups_detail: Option<serde_json::Value>,
    pub cluster_status: Option<serde_json::Value>,
    pub cluster_nodes: Option<serde_json::Value>,
    pub cluster_health: Option<serde_json::Value>,
    pub cluster_events: Vec<String>,
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
    pub show_command_palette: bool,
    pub command_input: String,
    pub discovery_done: bool,
    pub loading: bool,
    pub loading_message: String,
    pub backup_in_progress: bool,
    pub engine_metrics: Option<String>,
    pub query_rate: Option<String>,
    pub error_rate: Option<String>,
    pub memory_usage: Option<String>,
    pub storage_usage: Option<String>,
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
    pub governor_status: Option<String>,
    pub governor_executions: Vec<String>,
    pub governor_violations: Vec<String>,
    pub governor_metrics: Option<String>,
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
            connected_url: None,
            instances: Vec::new(),
            health_status: None,
            server_version: None,
            uptime: None,
            engine_list: Vec::new(),
            error_message: None,
            query_input: String::new(),
            query_scroll: 0,
            query_results: Vec::new(),
            event_log: Vec::new(),
            show_instances: false,
            selected_instance: 0,
            should_quit: false,
            last_refresh: Instant::now(),
            status_data: None,
            metrics_data: None,
            logs_data: None,
            backups_data: Vec::new(),
            backups_detail: None,
            cluster_status: None,
            cluster_nodes: None,
            cluster_health: None,
            cluster_events: Vec::new(),
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
            show_command_palette: false,
            command_input: String::new(),
            discovery_done: false,
            loading: true,
            loading_message: "Discovering instances...".to_string(),
            backup_in_progress: false,
            engine_metrics: None,
            query_rate: None,
            error_rate: None,
            memory_usage: None,
            storage_usage: None,
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
        }
    }

    pub fn add_event(&mut self, msg: String) {
        self.event_log.push(msg);
        if self.event_log.len() > 100 {
            self.event_log.remove(0);
        }
    }

    pub fn connected(&self) -> bool {
        self.connected_url.is_some()
    }

    pub fn connect_url(&mut self, url: &str) {
        self.connected_url = Some(url.to_string());
        self.add_event(format!("Connected to {}", url));
    }

    #[allow(dead_code)]
    pub fn disconnect(&mut self) {
        self.connected_url = None;
        self.status_data = None;
        self.health_status = None;
        self.server_version = None;
        self.uptime = None;
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
                .unwrap_or_default();
        } else {
            self.health_status = None;
            self.server_version = None;
            self.uptime = None;
            self.engine_list.clear();
        }
    }
}
