/// Global application state (connection, auth, sessions).
/// These fields are shared across all workspaces.
#[derive(Debug, Clone, Default)]
pub struct GlobalState {
    /// Current connection URL
    pub connected_url: Option<String>,
    /// Current namespace
    pub active_namespace: Option<String>,
    /// Current database
    pub active_database: Option<String>,
    /// Whether connected
    pub is_connected: bool,
    /// Session ID
    pub session_id: Option<String>,
    /// Event log
    pub events: Vec<String>,
    /// Error messages
    pub errors: Vec<String>,
}

/// Overlay state (command palette, search, dialogs).
#[derive(Debug, Clone, Default)]
pub struct OverlayState {
    pub show_command_palette: bool,
    pub command_palette_selection: usize,
    pub command_palette_filtered: Vec<String>,
    pub show_search: bool,
    pub search_query: String,
    pub search_scope: SearchScope,
    pub search_results: Vec<SearchResult>,
    pub search_selection: usize,
    pub show_help: bool,
    pub confirm_action: ConfirmAction,
    pub confirm_message: String,
    pub pending_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum SearchScope {
    #[default]
    All,
    Commands,
    Objects,
    Sections,
    Capabilities,
    Files,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub subtitle: String,
    pub section: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ConfirmAction {
    #[default]
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

/// Async task state for background operations.
#[derive(Debug, Clone, Default)]
pub struct AsyncState {
    pub backup_in_progress: bool,
    pub backup_progress_message: String,
    pub backup_operation_start: Option<std::time::Instant>,
    pub export_in_progress: bool,
    pub export_progress: u8,
    pub export_phase: String,
    pub export_status_line: String,
}

/// Tick counter for animations.
#[derive(Debug, Default)]
pub struct TickState {
    pub count: u64,
}

impl TickState {
    pub fn tick(&mut self) {
        self.count = self.count.wrapping_add(1);
    }
}
