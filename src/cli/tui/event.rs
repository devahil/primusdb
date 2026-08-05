#![allow(clippy::collapsible_match)]
use crate::cli::discovery::InstanceInfo;
use crate::cli::tui::api::{self, fetch_databases, fetch_namespaces, list_backups, run_discovery};
use crate::cli::tui::app::{
    ConfigStudioMode, ConfirmAction, NavSection, NotebookBuilderMode, QueryHistoryEntry,
    RagWorkspaceMode, ReportBuilderMode, SettingsMode, TableExplorerMode, TuiApp, NAV_SECTIONS,
    SIDEBAR_WIDTH,
};
use crate::cli::tui::config::TuiConfig;
use crate::cli::tui::render;
use crate::Result;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use std::io;
use std::time::Duration;

pub enum AppMessage {
    Discovery(Vec<InstanceInfo>),
    Status(Option<serde_json::Value>),
    QueryResult(String),
    Metrics(String),
    Logs(String),
    Backups(Vec<String>),
    BackupsDetail(Option<serde_json::Value>),
    Databases(Vec<String>),
    Namespaces(Vec<String>),
    TablesData(Vec<String>),
    VectorIndexesData(Vec<String>),
    GraphData(Option<serde_json::Value>),
    AIMLData(Option<serde_json::Value>),
    UsersData(Option<serde_json::Value>),
    RolesData(Option<serde_json::Value>),
    Diagnostics(String),
    Settings(Option<serde_json::Value>),
    ClusterStatus(Option<serde_json::Value>),
    ClusterNodes(Option<serde_json::Value>),
    ClusterHealth(Option<serde_json::Value>),
    ClusterEvents(Vec<String>),
    FederationStatus(Option<serde_json::Value>),
    FederationClusters(Option<serde_json::Value>),
    FederationDomains(Option<serde_json::Value>),
    Connected(String),
    Error(String),
    Tick,
    BackupCreated(String),
    BackupRestored(String),
    EngineMetrics(String),
    ClusterSummary(Option<serde_json::Value>),
    MigrationResult(String),
    MigrationError(String),
    MigrationProgress(u8),
    ConfigEntries(Option<serde_json::Value>),
    ConfigEntrySet(String),
    ConfigEntryDeleted(String),
    ConfigSnapshots(Option<serde_json::Value>),
    ConfigSnapshotCreated(String),
    ConfigSnapshotRestored(String),
    ConfigError(String),

    // Table Explorer messages
    ExplorerStorageTypes(Vec<String>),
    ExplorerTables(Option<serde_json::Value>),
    ExplorerTableInfo(Option<serde_json::Value>),
    ExplorerRows(Option<serde_json::Value>),
    ExplorerError(String),

    // Report Builder messages
    ReportsList(Option<serde_json::Value>),
    ReportCreated(String),
    ReportDeleted(String),
    ReportDetail(Option<serde_json::Value>),
    ReportResults(Option<serde_json::Value>),
    ReportError(String),

    // Notebook messages
    NotebooksList(Option<serde_json::Value>),
    NotebookCreated(String),
    NotebookDeleted(String),
    NotebookDetail(Option<serde_json::Value>),
    NotebookCellResult(Option<serde_json::Value>),
    NotebookError(String),

    // RAG Workspace messages
    RagCollections(Vec<String>),
    RagSearchResults(Option<serde_json::Value>),
    RagError(String),

    // Table CRUD messages
    TableCreated(String),
    TableDropped(String),
    TableError(String),

    // Database creation messages
    DatabaseCreated(String),
    DatabaseError(String),

    // Document CRUD messages
    DocumentCreated(String),
    DocumentUpdated(String),
    DocumentDeleted(String),
    DocumentError(String),

    // DDL messages
    ColumnAdded(String),
    ColumnDropped(String),
    ColumnModified(String),
    ConstraintAdded(String),
    ConstraintDropped(String),
    TableRenamed(String),
    DdlError(String),

    // User CRUD messages
    UserCreated(String),
    UserDeleted(String),
    UserError(String),

    // Role CRUD messages
    RoleCreated(String),
    RoleDeleted(String),
    RoleError(String),

    // Permission / RBAC messages
    PermissionsData(Option<serde_json::Value>),
    UserRoleAssigned(String),
    UserRoleRemoved(String),
    PermissionError(String),

    // Namespace CRUD messages
    NamespaceCreated(String),
    NamespaceDeleted(String),
    NamespaceError(String),

    // Governor messages
    GovernorStatusData(Option<serde_json::Value>),
    GovernorExecutionsData(Option<serde_json::Value>),
    GovernorViolationsData(Option<serde_json::Value>),
    GovernorMetricsData(Option<serde_json::Value>),
    GovernorPolicySet(String),
    GovernorPolicyDeleted(String),
    GovernorError(String),

    // Federation messages
    FederationClusterAdded(String),
    FederationClusterRemoved(String),
    FederationDomainCreated(String),
    FederationDomainDeleted(String),
    FederationError(String),

    // Cluster management messages
    ClusterStarted(String),
    ClusterStopped(String),
    ClusterRestarted(String),
    ClusterJoined(String),
    ClusterLeft(String),
    ClusterNodeRemoved(String),
    ClusterMaintenanceToggled(String),
    ClusterError(String),

    // Backup messages
    BackupVerified(String),
    BackupDeleted(String),

    // Export/Import progress
    ExportProgress(String, u8, String),

    // Doctor results
    DoctorResult(String),

    // Files
    FileEntries(Vec<String>),
    FileContent(Option<String>),
    FileDeleted(String),
    FileError(String),

    // Command dispatch from workspaces
    ExecuteAction(String),

    // Refresh
    RefreshAll,
}

fn setup_terminal(mouse_enabled: bool) -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if mouse_enabled {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    } else {
        execute!(stdout, EnterAlternateScreen)?;
    }
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mouse_enabled: bool,
) -> io::Result<()> {
    disable_raw_mode()?;
    if mouse_enabled {
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
    } else {
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    }
    terminal.show_cursor()?;
    Ok(())
}

/// Format an action result with CLI equivalent and REST endpoint annotation.
fn action_info(msg: &str, cli: &str, rest: &str) -> String {
    format!(
        "{}\n  \u{2328}  CLI: {}\n  \u{2192}  REST: {}",
        msg, cli, rest
    )
}

/// Dispatch an action string (from workspace ExecCommand or command palette).
/// This is the unified action handler — both workspaces and command palette route through here.
fn dispatch_action(
    app: &mut TuiApp,
    action: &str,
    tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>,
) {
    match action {
        "quit" => {
            if app.config.confirm_destructive_actions {
                app.confirm_action = ConfirmAction::Quit;
                app.confirm_message = "Are you sure you want to quit PrimusDB TUI?".to_string();
            } else {
                app.should_quit = true;
            }
        }
        "refresh" | "status" | "health" => trigger_refresh(app, tx),
        "disconnect" => {
            if app.config.confirm_destructive_actions && app.connected() {
                app.confirm_action = ConfirmAction::Disconnect;
                app.confirm_message = "Disconnect from the current server?".to_string();
            } else {
                app.disconnect();
            }
        }
        "backup_create" => {
            if app.connected() {
                app.backup_in_progress = true;
                app.backup_progress_message = "Creating backup...".to_string();
                let backup_tx = tx.clone();
                tokio::spawn(async move {
                    let output = std::process::Command::new("primusdb")
                        .args(["backup", "create"])
                        .output();
                    match output {
                        Ok(out) if out.status.success() => {
                            let msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
                            let _ = backup_tx.send(AppMessage::BackupCreated(msg));
                        }
                        Ok(out) => {
                            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                            let _ = backup_tx
                                .send(AppMessage::BackupCreated(format!("Error: {}", err)));
                        }
                        Err(e) => {
                            let _ =
                                backup_tx.send(AppMessage::BackupCreated(format!("Error: {}", e)));
                        }
                    }
                });
            }
        }
        "events_toggle" => {
            app.show_event_log = !app.show_event_log;
        }
        _ if action.starts_with("connect:") => {
            let url = action.trim_start_matches("connect:").to_string();
            let connect_tx = tx.clone();
            app.loading = true;
            app.loading_message = format!("Connecting to {}...", url);
            let url2 = url.clone();
            tokio::spawn(async move {
                let healthy = api::fetch_health(&url).await.is_some();
                if healthy {
                    let _ = connect_tx.send(AppMessage::Connected(url));
                    if let Some(status) = api::fetch_status(&url2).await {
                        let _ = connect_tx.send(AppMessage::Status(Some(status)));
                    }
                } else {
                    let _ = connect_tx.send(AppMessage::Error(format!("Could not reach {}", url)));
                }
            });
        }
        _ if action.starts_with("db_create_full:") => {
            let rest = action.trim_start_matches("db_create_full:").to_string();
            let parts: Vec<String> = rest.splitn(3, ':').map(|s| s.to_string()).collect();
            if parts.len() == 3 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let name = parts[0].clone();
                let description = parts[1].clone();
                let engines_csv = parts[2].clone();
                let db_tx = tx.clone();
                let url2 = url.clone();
                let name2 = name.clone();
                let desc2 = description.clone();
                tokio::spawn(async move {
                    let engines: Vec<&str> =
                        engines_csv.split(',').filter(|s| !s.is_empty()).collect();
                    match api::create_database(&url2, &name2, &desc2, &engines).await {
                        Ok(msg) => {
                            let cli = format!(
                                "primusdb database create {} --engines {}",
                                name2, engines_csv
                            );
                            let rest = "POST /api/v1/databases";
                            let _ = db_tx
                                .send(AppMessage::DatabaseCreated(action_info(&msg, &cli, rest)));
                        }
                        Err(e) => {
                            let _ = db_tx.send(AppMessage::DatabaseError(e));
                        }
                    }
                });
            } else if parts.len() < 3 {
                app.add_event("Usage: db_create_full:name:description:engines_csv".to_string());
            }
        }
        _ if action.starts_with("table_create:") => {
            let rest = action.trim_start_matches("table_create:").to_string();
            let parts: Vec<String> = if rest.contains('/') {
                rest.splitn(2, '/').map(|s| s.to_string()).collect()
            } else {
                rest.splitn(2, ' ').map(|s| s.to_string()).collect()
            };
            if parts.len() == 2 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let st = parts[0].clone();
                let table = parts[1].clone();
                let tb_tx = tx.clone();
                let st2 = st.clone();
                let table2 = table.clone();
                tokio::spawn(async move {
                    match api::create_table(&url, &st2, &table2, None).await {
                        Ok(msg) => {
                            let cli = format!("primusdb table create {} {}", st2, table2);
                            let rest = format!("POST /api/v1/table/{}/{}/create", st2, table2);
                            let _ = tb_tx
                                .send(AppMessage::TableCreated(action_info(&msg, &cli, &rest)));
                        }
                        Err(e) => {
                            let _ = tb_tx.send(AppMessage::TableError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("table_drop:") => {
            let rest = action.trim_start_matches("table_drop:").to_string();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let st = parts[0].to_string();
                let table = parts[1].to_string();
                let td_tx = tx.clone();
                let st2 = st.clone();
                let table2 = table.clone();
                tokio::spawn(async move {
                    match api::drop_table(&url, &st2, &table2).await {
                        Ok(msg) => {
                            let cli = format!("primusdb table drop {} {}", st2, table2);
                            let rest = format!("DELETE /api/v1/table/{}/{}/drop", st2, table2);
                            let _ = td_tx
                                .send(AppMessage::TableDropped(action_info(&msg, &cli, &rest)));
                        }
                        Err(e) => {
                            let _ = td_tx.send(AppMessage::TableError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("db_drop:") => {
            let name = action.trim_start_matches("db_drop:").to_string();
            if !name.is_empty() && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let st_tx = tx.clone();
                let name2 = name.clone();
                tokio::spawn(async move {
                    match api::drop_table(&url, "relational", &name2).await {
                        Ok(msg) => {
                            let cli = format!("primusdb database drop {}", name2);
                            let rest = format!("DELETE /api/v1/table/relational/{}/drop", name2);
                            let _ = st_tx
                                .send(AppMessage::TableDropped(action_info(&msg, &cli, &rest)));
                        }
                        Err(e) => {
                            let _ = st_tx.send(AppMessage::TableError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("namespace_create:") => {
            let name = action.trim_start_matches("namespace_create:").to_string();
            if !name.is_empty() && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let ns_tx = tx.clone();
                let name2 = name.clone();
                tokio::spawn(async move {
                    match api::create_namespace(&url, &name2).await {
                        Ok(msg) => {
                            let cli = format!("primusdb namespace create {}", name2);
                            let rest = format!("POST /api/v1/namespaces/{}", name2);
                            let _ = ns_tx
                                .send(AppMessage::NamespaceCreated(action_info(&msg, &cli, &rest)));
                        }
                        Err(e) => {
                            let _ = ns_tx.send(AppMessage::NamespaceError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("namespace_use:") => {
            let name = action.trim_start_matches("namespace_use:").to_string();
            if !name.is_empty() {
                app.active_namespace = Some(name.clone());
                app.add_event(format!(
                    "Switched to namespace: {}\n  \u{2328}  CLI: primusdb namespace use {}",
                    name, name
                ));
            }
        }
        _ if action.starts_with("export_data:") => {
            let rest = action.trim_start_matches("export_data:").to_string();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let st = parts[0].to_string();
                let table = parts[1].to_string();
                let exp_tx = tx.clone();
                tokio::spawn(async move {
                    let _ = exp_tx.send(AppMessage::ExportProgress(
                        "Exporting...".to_string(),
                        50,
                        format!("Exporting {} from {}", table, st),
                    ));
                    match api::export_table_data(&url, &st, &table, 10000).await {
                        Ok(_data) => {
                            let _ = exp_tx.send(AppMessage::ExportProgress(
                                "Complete".to_string(),
                                100,
                                format!("Exported from {}/{}", st, table),
                            ));
                        }
                        Err(e) => {
                            let _ = exp_tx.send(AppMessage::ExportProgress(
                                "Error".to_string(),
                                0,
                                format!("Export failed: {}", e),
                            ));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("import_data:") => {
            let rest = action.trim_start_matches("import_data:").to_string();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let st = parts[0].to_string();
                let table = parts[1].to_string();
                let imp_tx = tx.clone();
                tokio::spawn(async move {
                    let dummy_data = serde_json::json!([{"_import": true}]);
                    match api::import_table_data(&url, &st, &table, &dummy_data).await {
                        Ok(msg) => {
                            let _ = imp_tx.send(AppMessage::ExportProgress(
                                "Complete".to_string(),
                                100,
                                msg,
                            ));
                        }
                        Err(e) => {
                            let _ = imp_tx.send(AppMessage::ExportProgress(
                                "Error".to_string(),
                                0,
                                format!("Import failed: {}", e),
                            ));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("column_add:") => {
            let rest = action.trim_start_matches("column_add:").to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() >= 3 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let st = parts[0].to_string();
                let table = parts[1].to_string();
                let col_json = parts[2..].join(" ");
                if let Ok(col_def) = serde_json::from_str::<serde_json::Value>(&col_json) {
                    let ca_tx = tx.clone();
                    let st2 = st.clone();
                    let table2 = table.clone();
                    tokio::spawn(async move {
                        match api::add_column(&url, &st2, &table2, &col_def).await {
                            Ok(msg) => {
                                let cli = format!(":column add {} {} ...", st2, table2);
                                let rest =
                                    format!("POST /api/v1/ddl/{}/{}/column/add", st2, table2);
                                let _ = ca_tx
                                    .send(AppMessage::ColumnAdded(action_info(&msg, &cli, &rest)));
                            }
                            Err(e) => {
                                let _ = ca_tx.send(AppMessage::DdlError(e));
                            }
                        }
                    });
                }
            }
        }
        _ if action.starts_with("column_drop:") => {
            let rest = action.trim_start_matches("column_drop:").to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() == 3 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let st = parts[0].to_string();
                let table = parts[1].to_string();
                let col_name = parts[2].to_string();
                let cd_tx = tx.clone();
                let st2 = st.clone();
                let table2 = table.clone();
                let col2 = col_name.clone();
                tokio::spawn(async move {
                    match api::drop_column(&url, &st2, &table2, &col2).await {
                        Ok(msg) => {
                            let cli = format!(":column drop {} {} {}", st2, table2, col2);
                            let rest =
                                format!("DELETE /api/v1/ddl/{}/{}/column/{}", st2, table2, col2);
                            let _ = cd_tx
                                .send(AppMessage::ColumnDropped(action_info(&msg, &cli, &rest)));
                        }
                        Err(e) => {
                            let _ = cd_tx.send(AppMessage::DdlError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("column_modify:") => {
            let rest = action.trim_start_matches("column_modify:").to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() >= 3 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let st = parts[0].to_string();
                let table = parts[1].to_string();
                let col_json = parts[2..].join(" ");
                if let Ok(col_def) = serde_json::from_str::<serde_json::Value>(&col_json) {
                    let cm_tx = tx.clone();
                    let st2 = st.clone();
                    let table2 = table.clone();
                    tokio::spawn(async move {
                        match api::modify_column(&url, &st2, &table2, &col_def).await {
                            Ok(msg) => {
                                let cli = format!(":column modify {} {} ...", st2, table2);
                                let rest = format!("PUT /api/v1/ddl/{}/{}/column", st2, table2);
                                let _ = cm_tx.send(AppMessage::ColumnModified(action_info(
                                    &msg, &cli, &rest,
                                )));
                            }
                            Err(e) => {
                                let _ = cm_tx.send(AppMessage::DdlError(e));
                            }
                        }
                    });
                }
            }
        }
        _ if action.starts_with("constraint_add:") => {
            let rest = action.trim_start_matches("constraint_add:").to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() >= 3 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let st = parts[0].to_string();
                let table = parts[1].to_string();
                let con_json = parts[2..].join(" ");
                if let Ok(con_def) = serde_json::from_str::<serde_json::Value>(&con_json) {
                    let cadd_tx = tx.clone();
                    let st2 = st.clone();
                    let table2 = table.clone();
                    tokio::spawn(async move {
                        match api::add_constraint(&url, &st2, &table2, &con_def).await {
                            Ok(msg) => {
                                let cli = format!(":constraint add {} {} ...", st2, table2);
                                let rest =
                                    format!("POST /api/v1/ddl/{}/{}/constraint", st2, table2);
                                let _ = cadd_tx.send(AppMessage::ConstraintAdded(action_info(
                                    &msg, &cli, &rest,
                                )));
                            }
                            Err(e) => {
                                let _ = cadd_tx.send(AppMessage::DdlError(e));
                            }
                        }
                    });
                }
            }
        }
        _ if action.starts_with("constraint_drop:") => {
            let rest = action.trim_start_matches("constraint_drop:").to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() == 3 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let st = parts[0].to_string();
                let table = parts[1].to_string();
                let con_name = parts[2].to_string();
                let cdrop_tx = tx.clone();
                let st2 = st.clone();
                let table2 = table.clone();
                let con2 = con_name.clone();
                tokio::spawn(async move {
                    match api::drop_constraint(&url, &st2, &table2, &con2).await {
                        Ok(msg) => {
                            let cli = format!(":constraint drop {} {} {}", st2, table2, con2);
                            let rest = format!(
                                "DELETE /api/v1/ddl/{}/{}/constraint/{}",
                                st2, table2, con2
                            );
                            let _ = cdrop_tx.send(AppMessage::ConstraintDropped(action_info(
                                &msg, &cli, &rest,
                            )));
                        }
                        Err(e) => {
                            let _ = cdrop_tx.send(AppMessage::DdlError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("table_rename:") => {
            let rest = action.trim_start_matches("table_rename:").to_string();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() == 3 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let st = parts[0].to_string();
                let table = parts[1].to_string();
                let new_name = parts[2].to_string();
                let tr_tx = tx.clone();
                let st2 = st.clone();
                let table2 = table.clone();
                let new2 = new_name.clone();
                tokio::spawn(async move {
                    match api::rename_table(&url, &st2, &table2, &new2).await {
                        Ok(msg) => {
                            let cli = format!(":table rename {} {} {}", st2, table2, new2);
                            let rest = format!("POST /api/v1/ddl/{}/{}/rename", st2, table2);
                            let _ = tr_tx
                                .send(AppMessage::TableRenamed(action_info(&msg, &cli, &rest)));
                        }
                        Err(e) => {
                            let _ = tr_tx.send(AppMessage::DdlError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("doc_create:") => {
            let rest = action.trim_start_matches("doc_create:").to_string();
            let parts: Vec<String> = rest.splitn(3, ' ').map(|s| s.to_string()).collect();
            if parts.len() == 3 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let db = parts[0].clone();
                let doc_id = parts[1].clone();
                let json = parts[2].clone();
                let dc_tx = tx.clone();
                let db2 = db.clone();
                let doc2 = doc_id.clone();
                tokio::spawn(async move {
                    match api::put_kv_document(&url, &db2, &doc2, &json).await {
                        Ok(msg) => {
                            let cli = format!(":doc create {} {} ...", db2, doc2);
                            let rest = format!("PUT /api/v1/kv/{}/{}", db2, doc2);
                            let _ = dc_tx
                                .send(AppMessage::DocumentCreated(action_info(&msg, &cli, &rest)));
                        }
                        Err(e) => {
                            let _ = dc_tx.send(AppMessage::DocumentError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("doc_delete:") => {
            let key = action.trim_start_matches("doc_delete:").to_string();
            if !key.is_empty() && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let parts: Vec<String> = key.splitn(2, '/').map(|s| s.to_string()).collect();
                let db = parts.first().cloned().unwrap_or_default();
                let doc_id = parts.get(1).cloned().unwrap_or_default();
                if !db.is_empty() && !doc_id.is_empty() {
                    let dd_tx = tx.clone();
                    let db2 = db.clone();
                    let doc2 = doc_id.clone();
                    tokio::spawn(async move {
                        match api::delete_kv_document(&url, &db2, &doc2).await {
                            Ok(msg) => {
                                let cli = format!(":doc delete {} {}", db2, doc2);
                                let rest = format!("DELETE /api/v1/kv/{}/{}", db2, doc2);
                                let _ = dd_tx.send(AppMessage::DocumentDeleted(action_info(
                                    &msg, &cli, &rest,
                                )));
                            }
                            Err(e) => {
                                let _ = dd_tx.send(AppMessage::DocumentError(e));
                            }
                        }
                    });
                }
            }
        }
        _ if action.starts_with("user_delete:") => {
            let name = action.trim_start_matches("user_delete:").to_string();
            if !name.is_empty() && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let ud_tx = tx.clone();
                let name2 = name.clone();
                tokio::spawn(async move {
                    match api::delete_user(&url, &name2).await {
                        Ok(msg) => {
                            let cli = format!("primusdb user delete {}", name2);
                            let rest = format!("DELETE /api/v1/auth/users/{}", name2);
                            let _ =
                                ud_tx.send(AppMessage::UserDeleted(action_info(&msg, &cli, &rest)));
                        }
                        Err(e) => {
                            let _ = ud_tx.send(AppMessage::UserError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("role_delete:") => {
            let name = action.trim_start_matches("role_delete:").to_string();
            if !name.is_empty() && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let rd_tx = tx.clone();
                let name2 = name.clone();
                tokio::spawn(async move {
                    match api::delete_role(&url, &name2).await {
                        Ok(msg) => {
                            let cli = format!("primusdb role delete {}", name2);
                            let rest = format!("DELETE /api/v1/auth/roles/{}", name2);
                            let _ =
                                rd_tx.send(AppMessage::RoleDeleted(action_info(&msg, &cli, &rest)));
                        }
                        Err(e) => {
                            let _ = rd_tx.send(AppMessage::RoleError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("governor_set:") => {
            let rest = action.trim_start_matches("governor_set:").to_string();
            if !rest.is_empty() && app.connected() {
                let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let url = app.connected_url.clone().unwrap();
                    let name = parts[0].to_string();
                    let policy = parts[1].to_string();
                    let gv_tx = tx.clone();
                    let name2 = name.clone();
                    tokio::spawn(async move {
                        match api::set_governor_policy(&url, &name2, &policy).await {
                            Ok(msg) => {
                                let cli = format!(":governor set {} {}", name2, policy);
                                let rest = "POST /api/v1/governor/policies/update";
                                let _ = gv_tx.send(AppMessage::GovernorPolicySet(action_info(
                                    &msg, &cli, rest,
                                )));
                            }
                            Err(e) => {
                                let _ = gv_tx.send(AppMessage::GovernorError(e));
                            }
                        }
                    });
                }
            }
        }
        _ if action.starts_with("governor_delete:") => {
            let name = action.trim_start_matches("governor_delete:").to_string();
            if !name.is_empty() && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let gv_tx = tx.clone();
                let name2 = name.clone();
                tokio::spawn(async move {
                    match api::delete_governor_policy(&url, &name2).await {
                        Ok(msg) => {
                            let cli = format!(":governor delete {}", name2);
                            let rest = "DELETE /api/v1/governor/policies/{name}";
                            let _ = gv_tx.send(AppMessage::GovernorPolicyDeleted(action_info(
                                &msg, &cli, rest,
                            )));
                        }
                        Err(e) => {
                            let _ = gv_tx.send(AppMessage::GovernorError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("fed_cluster_add:") => {
            let rest = action.trim_start_matches("fed_cluster_add:").to_string();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let cluster_id = parts[0].to_string();
                let seed = parts[1].to_string();
                let fed_tx = tx.clone();
                let cid = cluster_id.clone();
                let seed2 = seed.clone();
                tokio::spawn(async move {
                    match api::create_federation_cluster(&url, &cid, &seed2).await {
                        Ok(msg) => {
                            let cli = format!(":federation cluster add {} {}", cid, seed2);
                            let rest = "POST /api/v1/federation/clusters";
                            let _ = fed_tx.send(AppMessage::FederationClusterAdded(action_info(
                                &msg, &cli, rest,
                            )));
                        }
                        Err(e) => {
                            let _ = fed_tx.send(AppMessage::FederationError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("fed_cluster_remove:") => {
            let cluster_id = action.trim_start_matches("fed_cluster_remove:").to_string();
            if !cluster_id.is_empty() && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let fed_tx = tx.clone();
                let cid = cluster_id.clone();
                tokio::spawn(async move {
                    match api::delete_federation_cluster(&url, &cid).await {
                        Ok(msg) => {
                            let cli = format!(":federation cluster remove {}", cid);
                            let rest = "DELETE /api/v1/federation/clusters/{id}";
                            let _ = fed_tx.send(AppMessage::FederationClusterRemoved(action_info(
                                &msg, &cli, rest,
                            )));
                        }
                        Err(e) => {
                            let _ = fed_tx.send(AppMessage::FederationError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("fed_domain_create:") => {
            let rest = action.trim_start_matches("fed_domain_create:").to_string();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let domain = parts[0].to_string();
                let config = parts[1].to_string();
                let cluster_ids: Vec<String> = config
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let fed_tx = tx.clone();
                let domain2 = domain.clone();
                tokio::spawn(async move {
                    match api::create_federation_domain(&url, &domain2, &cluster_ids).await {
                        Ok(msg) => {
                            let cli = format!(":federation domain create {} {}", domain2, config);
                            let rest = "POST /api/v1/federation/domains";
                            let _ = fed_tx.send(AppMessage::FederationDomainCreated(action_info(
                                &msg, &cli, rest,
                            )));
                        }
                        Err(e) => {
                            let _ = fed_tx.send(AppMessage::FederationError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("fed_domain_delete:") => {
            let name = action.trim_start_matches("fed_domain_delete:").to_string();
            if !name.is_empty() && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let fed_tx = tx.clone();
                let name2 = name.clone();
                tokio::spawn(async move {
                    match api::delete_federation_domain(&url, &name2).await {
                        Ok(msg) => {
                            let cli = format!(":federation domain delete {}", name2);
                            let rest = "DELETE /api/v1/federation/domains/{name}";
                            let _ = fed_tx.send(AppMessage::FederationDomainDeleted(action_info(
                                &msg, &cli, rest,
                            )));
                        }
                        Err(e) => {
                            let _ = fed_tx.send(AppMessage::FederationError(e));
                        }
                    }
                });
            }
        }
        "cluster_start" => {
            if let Some(ref url) = app.connected_url.clone() {
                let url = url.clone();
                let cl_tx = tx.clone();
                app.loading = true;
                app.loading_message = "Starting cluster...".to_string();
                tokio::spawn(async move {
                    let msg = api::cluster_start(&url).await;
                    let _ = cl_tx.send(AppMessage::ClusterStarted(action_info(
                        &msg,
                        "primusdb cluster start",
                        "POST /api/v1/cluster/status",
                    )));
                });
            }
        }
        "cluster_stop" => {
            if let Some(ref url) = app.connected_url.clone() {
                let url = url.clone();
                let cl_tx = tx.clone();
                app.loading = true;
                app.loading_message = "Stopping cluster...".to_string();
                tokio::spawn(async move {
                    let msg = api::cluster_stop(&url).await;
                    let _ = cl_tx.send(AppMessage::ClusterStopped(action_info(
                        &msg,
                        "primusdb cluster stop",
                        "POST /api/v1/cluster/status",
                    )));
                });
            }
        }
        "cluster_restart" => {
            if let Some(ref url) = app.connected_url.clone() {
                let url = url.clone();
                let cl_tx = tx.clone();
                app.loading = true;
                app.loading_message = "Restarting cluster...".to_string();
                tokio::spawn(async move {
                    let msg = api::cluster_restart(&url).await;
                    let _ = cl_tx.send(AppMessage::ClusterRestarted(action_info(
                        &msg,
                        "primusdb cluster restart",
                        "POST /api/v1/cluster/status",
                    )));
                });
            }
        }
        _ if action.starts_with("cluster_join:") => {
            let target = action.trim_start_matches("cluster_join:").to_string();
            if !target.is_empty() && app.connected() {
                let url = app.connected_url.clone().unwrap();
                let cl_tx = tx.clone();
                app.loading = true;
                app.loading_message = format!("Joining cluster at {}...", target);
                let target2 = target.clone();
                tokio::spawn(async move {
                    let msg = api::cluster_join(&url, &target2).await;
                    let cli = format!("primusdb cluster join {}", target2);
                    let _ = cl_tx.send(AppMessage::ClusterJoined(action_info(
                        &msg,
                        &cli,
                        "POST /api/v1/cluster/route",
                    )));
                });
            }
        }
        "cluster_leave" => {
            if let Some(ref url) = app.connected_url.clone() {
                let url = url.clone();
                let cl_tx = tx.clone();
                app.loading = true;
                app.loading_message = "Leaving cluster...".to_string();
                tokio::spawn(async move {
                    let msg = api::cluster_leave(&url).await;
                    let _ = cl_tx.send(AppMessage::ClusterLeft(action_info(
                        &msg,
                        "primusdb cluster leave",
                        "POST /api/v1/cluster/status",
                    )));
                });
            }
        }
        "cluster_node_remove" => {
            if let Some(ref url) = app.connected_url.clone() {
                let url = url.clone();
                let cl_tx = tx.clone();
                app.loading = true;
                app.loading_message = "Removing node...".to_string();
                tokio::spawn(async move {
                    let msg = api::cluster_remove_node(&url).await;
                    let _ = cl_tx.send(AppMessage::ClusterNodeRemoved(action_info(
                        &msg,
                        "primusdb cluster node remove",
                        "DELETE /api/v1/cluster/node/{id}",
                    )));
                });
            }
        }
        "cluster_maintenance" => {
            if let Some(ref url) = app.connected_url.clone() {
                let url = url.clone();
                let cl_tx = tx.clone();
                app.loading = true;
                app.loading_message = "Toggling maintenance...".to_string();
                tokio::spawn(async move {
                    let msg = api::cluster_maintenance(&url).await;
                    let _ = cl_tx.send(AppMessage::ClusterMaintenanceToggled(action_info(
                        &msg,
                        "primusdb cluster maintenance",
                        "POST /api/v1/cluster/maintenance",
                    )));
                });
            }
        }
        "doctor" => {
            app.add_event("Running diagnostics...".to_string());
            let url = app.connected_url.clone();
            let diag_tx = tx.clone();
            tokio::spawn(async move {
                let mut results = Vec::new();
                if let Some(ref u) = url {
                    results.push(format!("Endpoint: {}", u));
                    let healthy = api::fetch_health(u).await.is_some();
                    results.push(if healthy {
                        "Health: OK".to_string()
                    } else {
                        "Health: FAIL".to_string()
                    });
                    if let Some(status) = api::fetch_status(u).await {
                        if let Some(v) = status.get("version").and_then(|s| s.as_str()) {
                            results.push(format!("Version: {}", v));
                        }
                    }
                    results.push("Diagnostics complete.".to_string());
                } else {
                    results.push("Not connected to any server.".to_string());
                }
                let _ = diag_tx.send(AppMessage::DoctorResult(results.join("\n")));
            });
        }
        _ if action.starts_with("capability:") => {
            let cap_id = action.trim_start_matches("capability:").to_string();
            if !crate::cli::tui::commands::dispatch(&cap_id, app, tx) {
                app.add_event(format!("Unknown capability '{}'", cap_id));
            }
        }
        _ if action.starts_with("backup_delete:") => {
            let id = action.trim_start_matches("backup_delete:").to_string();
            match api::delete_backup_local(&id) {
                Ok(msg) => app.add_event(msg),
                Err(e) => app.add_event(format!("Backup delete error: {}", e)),
            }
            trigger_refresh(app, tx);
        }
        _ if action.starts_with("file_delete:") => {
            let path = action.trim_start_matches("file_delete:").to_string();
            match api::delete_file_local(&path) {
                Ok(msg) => app.add_event(msg),
                Err(e) => app.add_event(format!("File delete error: {}", e)),
            }
            trigger_refresh(app, tx);
        }
        _ if action.starts_with("namespace_delete:") => {
            let name = action.trim_start_matches("namespace_delete:").to_string();
            if app.connected() {
                let url = app.connected_url.clone().unwrap();
                let del_tx = tx.clone();
                let name2 = name.clone();
                tokio::spawn(async move {
                    match api::delete_namespace(&url, &name2).await {
                        Ok(msg) => {
                            let cli = format!("primusdb namespace delete {}", name2);
                            let rest = format!("DELETE /api/v1/namespaces/{}", name2);
                            let _ = del_tx
                                .send(AppMessage::NamespaceDeleted(action_info(&msg, &cli, &rest)));
                        }
                        Err(e) => {
                            let _ = del_tx.send(AppMessage::NamespaceError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("rag_delete:") => {
            let name = action.trim_start_matches("rag_delete:").to_string();
            if app.connected() {
                let url = app.connected_url.clone().unwrap();
                let rag_tx = tx.clone();
                let name2 = name.clone();
                tokio::spawn(async move {
                    match api::delete_rag_collection(&url, &name2).await {
                        Ok(msg) => {
                            let cli = format!("primusdb rag delete {}", name2);
                            let rest = format!("DELETE /api/v1/table/vector/{}/drop", name2);
                            let _ = rag_tx
                                .send(AppMessage::TableDropped(action_info(&msg, &cli, &rest)));
                        }
                        Err(e) => {
                            let _ = rag_tx.send(AppMessage::TableError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("user_create:") => {
            let input = action.trim_start_matches("user_create:").to_string();
            if app.connected() {
                let url = app.connected_url.clone().unwrap();
                let parts: Vec<&str> = input.splitn(2, ':').collect();
                let username = parts[0].to_string();
                let password = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
                let uc_tx = tx.clone();
                let user2 = username.clone();
                tokio::spawn(async move {
                    match api::create_user(&url, &user2, &password).await {
                        Ok(msg) => {
                            let cli = format!("primusdb user create {}", user2);
                            let _ = uc_tx.send(AppMessage::UserCreated(action_info(
                                &msg,
                                &cli,
                                "POST /api/v1/auth/register",
                            )));
                        }
                        Err(e) => {
                            let _ = uc_tx.send(AppMessage::UserError(e));
                        }
                    }
                });
            }
        }
        _ if action.starts_with("role_create:") => {
            let input = action.trim_start_matches("role_create:").to_string();
            if app.connected() {
                let url = app.connected_url.clone().unwrap();
                let rc_tx = tx.clone();
                let name = input.clone();
                let name2 = name.clone();
                tokio::spawn(async move {
                    match api::create_role(&url, &name2).await {
                        Ok(msg) => {
                            let cli = format!("primusdb role create {}", name2);
                            let _ = rc_tx.send(AppMessage::RoleCreated(action_info(
                                &msg,
                                &cli,
                                "POST /api/v1/auth/segment/create",
                            )));
                        }
                        Err(e) => {
                            let _ = rc_tx.send(AppMessage::RoleError(e));
                        }
                    }
                });
            }
        }
        _ if action == "terminal_clear" => {
            app.event_log.clear();
        }
        _ => {
            app.add_event(format!("Executed: {}", action));
        }
    }
}

fn trigger_refresh(app: &mut TuiApp, tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>) {
    app.add_event("Refreshing...".to_string());

    let discover_tx = tx.clone();
    tokio::spawn(async move {
        let instances = run_discovery().await;
        let _ = discover_tx.send(AppMessage::Discovery(instances));
    });

    if let Some(ref url) = app.connected_url.clone() {
        let section = app.current_section;
        let section_tx = tx.clone();
        let url_clone = url.clone();
        let explorer_st = app.explorer_selected_st.clone();
        tokio::spawn(async move {
            if let Some(status) = api::fetch_status(&url_clone).await {
                let _ = section_tx.send(AppMessage::Status(Some(status)));
            }
            match section {
                NavSection::Cluster => {
                    if let Some(v) = api::fetch_cluster_status(&url_clone).await {
                        let _ = section_tx.send(AppMessage::ClusterStatus(Some(v)));
                    }
                    if let Some(v) = api::fetch_cluster_nodes(&url_clone).await {
                        let _ = section_tx.send(AppMessage::ClusterNodes(Some(v)));
                    }
                    if let Some(v) = api::fetch_cluster_health(&url_clone).await {
                        let _ = section_tx.send(AppMessage::ClusterHealth(Some(v)));
                    }
                }
                NavSection::Federation => {
                    if let Some(v) = api::fetch_cluster_status(&url_clone).await {
                        let _ = section_tx.send(AppMessage::FederationStatus(Some(v)));
                    }
                    if let Some(v) = api::fetch_federation_clusters(&url_clone).await {
                        let _ = section_tx.send(AppMessage::FederationClusters(Some(v)));
                    }
                    if let Some(v) = api::fetch_federation_domains(&url_clone).await {
                        let _ = section_tx.send(AppMessage::FederationDomains(Some(v)));
                    }
                }
                NavSection::MetricsLogs => {
                    if let Some(v) = api::fetch_metrics(&url_clone).await {
                        let _ = section_tx.send(AppMessage::Metrics(v));
                    }
                }
                NavSection::DatabasesEngines => {
                    let dbs = fetch_databases(&url_clone).await;
                    let _ = section_tx.send(AppMessage::Databases(dbs));
                }
                NavSection::Namespaces => {
                    let ns = fetch_namespaces(&url_clone).await;
                    let _ = section_tx.send(AppMessage::Namespaces(ns));
                }
                NavSection::Governor => {
                    if let Some(v) = api::fetch_governor_status(&url_clone).await {
                        let _ = section_tx.send(AppMessage::Settings(Some(v)));
                    }
                }
                NavSection::ConfigurationStudio => {
                    if let Some(entries) = api::fetch_config_entries(&url_clone).await {
                        let _ = section_tx.send(AppMessage::ConfigEntries(Some(entries)));
                    }
                }
                NavSection::TableExplorer => {
                    let types = api::fetch_explorer_storage_types(&url_clone).await;
                    let _ = section_tx.send(AppMessage::ExplorerStorageTypes(types));
                    if let Some(ref st) = explorer_st {
                        if let Some(tables) = api::fetch_explorer_tables(&url_clone, st).await {
                            let _ = section_tx.send(AppMessage::ExplorerTables(Some(tables)));
                        }
                    }
                }
                NavSection::ReportBuilder => {
                    if let Some(reports) = api::fetch_reports(&url_clone).await {
                        let _ = section_tx.send(AppMessage::ReportsList(Some(reports)));
                    }
                }
                NavSection::Notebook => {
                    if let Some(notebooks) = api::fetch_notebooks(&url_clone).await {
                        let _ = section_tx.send(AppMessage::NotebooksList(Some(notebooks)));
                    }
                }
                NavSection::RAGWorkspace => {
                    if let Some(collections) = api::fetch_rag_collections(&url_clone).await {
                        let _ = section_tx.send(AppMessage::RagCollections(collections));
                    }
                }
                NavSection::Settings => {
                    if let Some(settings) = api::fetch_status(&url_clone).await {
                        let _ = section_tx.send(AppMessage::Settings(Some(settings)));
                    }
                }
                _ => {}
            }
        });
    }

    if app.current_section == NavSection::BackupRestore {
        let backups = list_backups();
        let _ = tx.send(AppMessage::Backups(backups));
        if let Some(detail) = api::list_backups_detail() {
            let _ = tx.send(AppMessage::BackupsDetail(Some(detail)));
        }
    }
}

async fn run_loop(
    mut terminal: Terminal<CrosstermBackend<io::Stdout>>,
    mut app: TuiApp,
    initial_url: Option<String>,
    config: TuiConfig,
) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<AppMessage>();

    let refresh_ms = config.refresh_interval_ms;

    let discover_tx = tx.clone();
    tokio::spawn(async move {
        let instances = run_discovery().await;
        let _ = discover_tx.send(AppMessage::Discovery(instances));
    });

    if let Some(url) = initial_url {
        let connect_tx = tx.clone();
        app.loading_message = format!("Connecting to {}...", url);
        app.onboarding_mode = false;
        tokio::spawn(async move {
            let healthy = api::fetch_health(&url).await.is_some();
            if healthy {
                let _ = connect_tx.send(AppMessage::Connected(url.clone()));
                if let Some(status) = api::fetch_status(&url).await {
                    let _ = connect_tx.send(AppMessage::Status(Some(status)));
                }
            } else {
                let _ = connect_tx.send(AppMessage::Error(format!("Could not reach {}", url)));
            }
        });
    } else {
        app.onboarding_mode = true;
        app.onboarding_step = 1;
    }

    let tick_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(refresh_ms));
        interval.tick().await;
        loop {
            interval.tick().await;
            let _ = tick_tx.send(AppMessage::Tick);
        }
    });

    loop {
        terminal.draw(|f| render::render(f, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    // Confirmation dialog handling
                    if app.confirm_action != ConfirmAction::None {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                                match app.confirm_action {
                                    ConfirmAction::Quit => {
                                        app.should_quit = true;
                                    }
                                    ConfirmAction::Disconnect => {
                                        app.disconnect();
                                        app.current_section = NavSection::Dashboard;
                                    }
                                    ConfirmAction::BackupDelete => {
                                        let backup_name =
                                            app.backups_data.first().cloned().unwrap_or_default();
                                        if !backup_name.is_empty() {
                                            let backup_tx = tx.clone();
                                            let name = backup_name.clone();
                                            tokio::spawn(async move {
                                                let output = std::process::Command::new("primusdb")
                                                    .args(["backup", "delete", &name, "--force"])
                                                    .output();
                                                match output {
                                                    Ok(out) if out.status.success() => {
                                                        let msg =
                                                            String::from_utf8_lossy(&out.stdout)
                                                                .trim()
                                                                .to_string();
                                                        let _ = backup_tx
                                                            .send(AppMessage::BackupCreated(msg));
                                                    }
                                                    Ok(out) => {
                                                        let err =
                                                            String::from_utf8_lossy(&out.stderr)
                                                                .trim()
                                                                .to_string();
                                                        let _ = backup_tx.send(
                                                            AppMessage::BackupCreated(format!(
                                                                "Delete failed: {}",
                                                                err
                                                            )),
                                                        );
                                                    }
                                                    Err(e) => {
                                                        let _ = backup_tx.send(
                                                            AppMessage::BackupCreated(format!(
                                                                "Delete error: {}",
                                                                e
                                                            )),
                                                        );
                                                    }
                                                }
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                                app.confirm_action = ConfirmAction::None;
                                app.confirm_message.clear();
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                app.confirm_action = ConfirmAction::None;
                                app.confirm_message.clear();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Onboarding mode handling
                    if app.onboarding_mode {
                        match key.code {
                            KeyCode::Char('1') if app.onboarding_step == 1 => {
                                app.onboarding_step = 2;
                                app.loading_message = "Select connection endpoint...".to_string();
                            }
                            KeyCode::Char('2') if app.onboarding_step == 1 => {
                                app.onboarding_step = 2;
                                app.loading_message = "Enter custom endpoint...".to_string();
                            }
                            KeyCode::Char('3') if app.onboarding_step == 1 => {
                                app.onboarding_mode = false;
                                app.loading = false;
                            }
                            KeyCode::Enter if app.onboarding_step == 2 => {
                                let url = app.command_input.trim().to_string();
                                if !url.is_empty() {
                                    let connect_tx = tx.clone();
                                    app.loading = true;
                                    app.loading_message = format!("Connecting to {}...", url);
                                    tokio::spawn(async move {
                                        let healthy = api::fetch_health(&url).await.is_some();
                                        if healthy {
                                            let _ =
                                                connect_tx.send(AppMessage::Connected(url.clone()));
                                            if let Some(status) = api::fetch_status(&url).await {
                                                let _ = connect_tx
                                                    .send(AppMessage::Status(Some(status)));
                                            }
                                        } else {
                                            let _ = connect_tx.send(AppMessage::Error(format!(
                                                "Could not reach {}",
                                                url
                                            )));
                                        }
                                    });
                                    app.onboarding_mode = false;
                                    app.command_input.clear();
                                }
                            }
                            KeyCode::Esc => {
                                app.onboarding_mode = false;
                                app.loading = false;
                            }
                            KeyCode::Char(c) if app.onboarding_step == 2 => {
                                app.command_input.push(c);
                            }
                            KeyCode::Backspace if app.onboarding_step == 2 => {
                                app.command_input.pop();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Migration wizard handling
                    if app.migration_wizard_active {
                        match key.code {
                            KeyCode::Esc => {
                                if app.migration_step <= 1 {
                                    app.migration_wizard_active = false;
                                    app.migration_step = 0;
                                    app.command_input.clear();
                                    app.migration_source_connected = false;
                                    app.migration_objects.clear();
                                    app.migration_selected_objects.clear();
                                    app.migration_plan.clear();
                                    app.migration_report.clear();
                                    app.migration_dry_run_result = None;
                                } else if app.migration_step == 2 || app.migration_step == 4 {
                                    app.migration_step -= 1;
                                    app.command_input.clear();
                                } else {
                                    app.migration_step -= 1;
                                }
                            }
                            KeyCode::Enter if app.migration_step == 0 => {
                                app.migration_step = 1;
                            }
                            KeyCode::Char(c @ '1')
                            | KeyCode::Char(c @ '2')
                            | KeyCode::Char(c @ '3')
                            | KeyCode::Char(c @ '4')
                                if app.migration_step == 1 =>
                            {
                                app.migration_source = match c {
                                    '1' => "mysql",
                                    '2' => "postgresql",
                                    '3' => "mongodb",
                                    '4' => "couchdb",
                                    _ => unreachable!(),
                                }
                                .to_string();
                                app.migration_step = 2;
                                app.command_input.clear();
                                app.add_event(format!(
                                    "Migration source: {}",
                                    app.migration_source
                                ));
                            }
                            KeyCode::Enter if app.migration_step == 2 => {
                                let url = app.command_input.trim().to_string();
                                if !url.is_empty() {
                                    app.migration_url = url.clone();
                                    app.command_input.clear();
                                    app.migration_step = 3;
                                    app.migration_source_connected = false;
                                    app.migration_error = None;
                                    app.add_event(format!("Migration URL: {}", url));
                                    let source = app.migration_source.clone();
                                    let url2 = url.clone();
                                    let test_tx = tx.clone();
                                    tokio::spawn(async move {
                                        let result =
                                            api::migrate_inspect_source(&source, &url2).await;
                                        match result {
                                            Ok(msg) => {
                                                let _ =
                                                    test_tx.send(AppMessage::MigrationResult(msg));
                                            }
                                            Err(e) => {
                                                let _ = test_tx.send(AppMessage::MigrationError(e));
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Enter if app.migration_step == 3 => {
                                if app.migration_source_connected {
                                    app.migration_step = 4;
                                    app.command_input.clear();
                                } else {
                                    app.migration_source_connected = false;
                                    app.migration_error = None;
                                    let source = app.migration_source.clone();
                                    let url = app.migration_url.clone();
                                    let test_tx = tx.clone();
                                    tokio::spawn(async move {
                                        let result =
                                            api::migrate_inspect_source(&source, &url).await;
                                        match result {
                                            Ok(msg) => {
                                                let _ =
                                                    test_tx.send(AppMessage::MigrationResult(msg));
                                            }
                                            Err(e) => {
                                                let _ = test_tx.send(AppMessage::MigrationError(e));
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Enter if app.migration_step == 4 => {
                                let ns = app.command_input.trim().to_string();
                                if !ns.is_empty() {
                                    app.migration_namespace = ns.clone();
                                    app.command_input.clear();
                                    app.migration_step = 5;
                                    app.add_event(format!("Migration namespace: {}", ns));
                                }
                            }
                            KeyCode::Char(c @ '1')
                            | KeyCode::Char(c @ '2')
                            | KeyCode::Char(c @ '3')
                            | KeyCode::Char(c @ '4')
                                if app.migration_step == 5 =>
                            {
                                app.migration_mode = match c {
                                    '1' => "copy",
                                    '2' => "schema-only",
                                    '3' => "data-only",
                                    '4' => "dry-run",
                                    _ => unreachable!(),
                                }
                                .to_string();
                                app.add_event(format!("Migration mode: {}", app.migration_mode));
                                app.migration_step = 6;
                                app.migration_objects.clear();
                                app.migration_error = None;
                                let source = app.migration_source.clone();
                                let url = app.migration_url.clone();
                                let inspect_tx = tx.clone();
                                tokio::spawn(async move {
                                    let result = api::migrate_inspect_source(&source, &url).await;
                                    match result {
                                        Ok(msg) => {
                                            let _ =
                                                inspect_tx.send(AppMessage::MigrationResult(msg));
                                        }
                                        Err(e) => {
                                            let _ = inspect_tx.send(AppMessage::MigrationError(e));
                                        }
                                    }
                                });
                            }
                            KeyCode::Enter if app.migration_step == 6 => {
                                if app.migration_objects.is_empty() && app.migration_error.is_some()
                                {
                                    let source = app.migration_source.clone();
                                    let url = app.migration_url.clone();
                                    let inspect_tx = tx.clone();
                                    tokio::spawn(async move {
                                        let result =
                                            api::migrate_inspect_source(&source, &url).await;
                                        match result {
                                            Ok(msg) => {
                                                let _ = inspect_tx
                                                    .send(AppMessage::MigrationResult(msg));
                                            }
                                            Err(e) => {
                                                let _ =
                                                    inspect_tx.send(AppMessage::MigrationError(e));
                                            }
                                        }
                                    });
                                } else {
                                    if app.migration_selected_objects.is_empty()
                                        && !app.migration_objects.is_empty()
                                    {
                                        app.migration_selected_objects =
                                            vec![true; app.migration_objects.len()];
                                    }
                                    let mut plan = String::new();
                                    plan.push_str(&format!("Migration Plan:\n  Source: {} at {}\n  Target: {}\n  Namespace: {}\n  Mode: {}\n",
                                        app.migration_source, app.migration_url,
                                        app.connected_url.as_deref().unwrap_or("(not connected)"),
                                        app.migration_namespace, app.migration_mode));
                                    let obj_count = app
                                        .migration_selected_objects
                                        .iter()
                                        .filter(|&&s| s)
                                        .count();
                                    plan.push_str(&format!("  Objects: {} selected\n", obj_count));
                                    app.migration_plan = plan;
                                    app.migration_step = 8;
                                }
                            }
                            KeyCode::Enter if app.migration_step == 7 => {
                                if app.migration_selected_objects.is_empty()
                                    && !app.migration_objects.is_empty()
                                {
                                    app.migration_selected_objects =
                                        vec![true; app.migration_objects.len()];
                                }
                                let mut plan = String::new();
                                plan.push_str(&format!("Migration Plan:\n  Source: {} at {}\n  Target: {}\n  Namespace: {}\n  Mode: {}\n",
                                    app.migration_source, app.migration_url,
                                    app.connected_url.as_deref().unwrap_or("(not connected)"),
                                    app.migration_namespace, app.migration_mode));
                                let obj_count = app
                                    .migration_selected_objects
                                    .iter()
                                    .filter(|&&s| s)
                                    .count();
                                plan.push_str(&format!("  Objects: {} selected\n", obj_count));
                                app.migration_plan = plan;
                                app.migration_step = 8;
                            }
                            KeyCode::Char(' ') if app.migration_step == 7 => {
                                if app.migration_selected_objects.is_empty()
                                    && !app.migration_objects.is_empty()
                                {
                                    app.migration_selected_objects =
                                        vec![true; app.migration_objects.len()];
                                    if !app.migration_selected_objects.is_empty() {
                                        app.migration_selected_objects[0] = false;
                                    }
                                } else if !app.migration_selected_objects.is_empty() {
                                    let all_selected =
                                        app.migration_selected_objects.iter().all(|&s| s);
                                    if all_selected {
                                        for s in &mut app.migration_selected_objects {
                                            *s = false;
                                        }
                                    } else {
                                        let first_unsel = app
                                            .migration_selected_objects
                                            .iter()
                                            .position(|&s| !s)
                                            .unwrap_or(0);
                                        app.migration_selected_objects[first_unsel] = true;
                                    }
                                }
                            }
                            KeyCode::Enter if app.migration_step == 8 => {
                                app.migration_step = 9;
                                app.migration_dry_run_result = None;
                                app.migration_error = None;
                                let source = app.migration_source.clone();
                                let url = app.migration_url.clone();
                                let target = app.connected_url.clone().unwrap_or_default();
                                let ns = app.migration_namespace.clone();
                                let dry_tx = tx.clone();
                                tokio::spawn(async move {
                                    let result = api::migrate_run_import(
                                        &source, &url, &target, &ns, "dry-run",
                                    )
                                    .await;
                                    match result {
                                        Ok(msg) => {
                                            let _ = dry_tx.send(AppMessage::MigrationResult(msg));
                                        }
                                        Err(e) => {
                                            let _ = dry_tx.send(AppMessage::MigrationError(e));
                                        }
                                    }
                                });
                            }
                            KeyCode::Enter if app.migration_step == 9 => {
                                let mode = app.migration_mode.as_str();
                                if mode == "dry-run" || app.migration_dry_run_result.is_some() {
                                    app.migration_step = 10;
                                } else {
                                    let source = app.migration_source.clone();
                                    let url = app.migration_url.clone();
                                    let target = app.connected_url.clone().unwrap_or_default();
                                    let ns = app.migration_namespace.clone();
                                    let dry_tx = tx.clone();
                                    tokio::spawn(async move {
                                        let result = api::migrate_run_import(
                                            &source, &url, &target, &ns, "dry-run",
                                        )
                                        .await;
                                        match result {
                                            Ok(msg) => {
                                                let _ =
                                                    dry_tx.send(AppMessage::MigrationResult(msg));
                                            }
                                            Err(e) => {
                                                let _ = dry_tx.send(AppMessage::MigrationError(e));
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Enter if app.migration_step == 10 => {
                                app.migration_step = 11;
                                app.migration_progress = 0;
                                app.migration_status = "Starting migration...".to_string();
                                app.add_event("Starting migration...".to_string());
                                let source = app.migration_source.clone();
                                let url = app.migration_url.clone();
                                let target = app.connected_url.clone().unwrap_or_default();
                                let ns = app.migration_namespace.clone();
                                let mode = app.migration_mode.clone();
                                let result_tx = tx.clone();
                                tokio::spawn(async move {
                                    let _ = result_tx.send(AppMessage::MigrationProgress(10));
                                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                                    let _ = result_tx.send(AppMessage::MigrationProgress(40));
                                    let result =
                                        api::migrate_run_import(&source, &url, &target, &ns, &mode)
                                            .await;
                                    match result {
                                        Ok(msg) => {
                                            let _ =
                                                result_tx.send(AppMessage::MigrationProgress(100));
                                            let _ =
                                                result_tx.send(AppMessage::MigrationResult(msg));
                                        }
                                        Err(e) => {
                                            let _ =
                                                result_tx.send(AppMessage::MigrationProgress(100));
                                            let _ = result_tx.send(AppMessage::MigrationError(e));
                                        }
                                    }
                                });
                            }
                            KeyCode::Enter
                                if app.migration_step >= 11
                                    && app.migration_progress >= 100
                                    && !app.migration_report.is_empty() =>
                            {
                                app.add_event("Migration report saved.".to_string());
                                app.migration_wizard_active = false;
                                app.migration_step = 0;
                            }
                            KeyCode::Char(c)
                                if app.migration_step == 2 || app.migration_step == 4 =>
                            {
                                app.command_input.push(c);
                            }
                            KeyCode::Backspace
                                if app.migration_step == 2 || app.migration_step == 4 =>
                            {
                                app.command_input.pop();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Command palette handling
                    if app.show_command_palette {
                        match key.code {
                            KeyCode::Esc => {
                                app.show_command_palette = false;
                                app.command_input.clear();
                                app.command_palette_filtered.clear();
                                app.command_palette_selection = 0;
                            }
                            KeyCode::Enter => {
                                let cmd = app.command_input.trim().to_string();
                                app.show_command_palette = false;
                                app.command_input.clear();
                                app.command_palette_filtered.clear();
                                app.command_palette_selection = 0;
                                if let Some(action) = app.execute_command(&cmd) {
                                    match action.as_str() {
                                        "quit" => {
                                            if app.config.confirm_destructive_actions {
                                                app.confirm_action = ConfirmAction::Quit;
                                                app.confirm_message =
                                                    "Are you sure you want to quit PrimusDB TUI?"
                                                        .to_string();
                                            } else {
                                                app.should_quit = true;
                                            }
                                        }
                                        "refresh" => trigger_refresh(&mut app, &tx),
                                        "disconnect" => {
                                            if app.config.confirm_destructive_actions
                                                && app.connected()
                                            {
                                                app.confirm_action = ConfirmAction::Disconnect;
                                                app.confirm_message =
                                                    "Disconnect from the current server?"
                                                        .to_string();
                                            } else {
                                                app.disconnect();
                                            }
                                        }
                                        "backup_create" => {
                                            if app.connected() {
                                                app.backup_in_progress = true;
                                                app.add_event("Creating backup...".to_string());
                                                let backup_tx = tx.clone();
                                                tokio::spawn(async move {
                                                    let output =
                                                        std::process::Command::new("primusdb")
                                                            .args(["backup", "create"])
                                                            .output();
                                                    match output {
                                                        Ok(out) if out.status.success() => {
                                                            let msg = String::from_utf8_lossy(
                                                                &out.stdout,
                                                            )
                                                            .trim()
                                                            .to_string();
                                                            let _ = backup_tx.send(
                                                                AppMessage::BackupCreated(msg),
                                                            );
                                                        }
                                                        Ok(out) => {
                                                            let err = String::from_utf8_lossy(
                                                                &out.stderr,
                                                            )
                                                            .trim()
                                                            .to_string();
                                                            let _ = backup_tx.send(
                                                                AppMessage::BackupCreated(format!(
                                                                    "Error: {}",
                                                                    err
                                                                )),
                                                            );
                                                        }
                                                        Err(e) => {
                                                            let _ = backup_tx.send(
                                                                AppMessage::BackupCreated(format!(
                                                                    "Error: {}",
                                                                    e
                                                                )),
                                                            );
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                        "status" => trigger_refresh(&mut app, &tx),
                                        "health" => trigger_refresh(&mut app, &tx),
                                        _ if action.starts_with("connect:") => {
                                            let url =
                                                action.trim_start_matches("connect:").to_string();
                                            let connect_tx = tx.clone();
                                            app.loading = true;
                                            app.loading_message =
                                                format!("Connecting to {}...", url);
                                            let url2 = url.clone();
                                            tokio::spawn(async move {
                                                let healthy =
                                                    api::fetch_health(&url).await.is_some();
                                                if healthy {
                                                    let _ =
                                                        connect_tx.send(AppMessage::Connected(url));
                                                    if let Some(status) =
                                                        api::fetch_status(&url2).await
                                                    {
                                                        let _ = connect_tx
                                                            .send(AppMessage::Status(Some(status)));
                                                    }
                                                } else {
                                                    let _ = connect_tx.send(AppMessage::Error(
                                                        format!("Could not reach {}", url),
                                                    ));
                                                }
                                            });
                                        }
                                        _ => {
                                            app.add_event(format!("Executed: :{}", action));
                                        }
                                    }
                                }
                            }
                            KeyCode::Up => {
                                let filtered = app.filter_commands();
                                if !filtered.is_empty() {
                                    app.command_palette_selection =
                                        app.command_palette_selection.saturating_sub(1);
                                }
                            }
                            KeyCode::Down => {
                                let filtered = app.filter_commands();
                                if !filtered.is_empty() {
                                    app.command_palette_selection = (app.command_palette_selection
                                        + 1)
                                    .min(filtered.len().saturating_sub(1));
                                }
                            }
                            KeyCode::Tab => {
                                let filtered = app.filter_commands();
                                if !filtered.is_empty() {
                                    if let Some(item) = filtered.get(app.command_palette_selection)
                                    {
                                        let cmd_part = item.split(" - ").next().unwrap_or(item);
                                        app.command_input = cmd_part.trim().to_string();
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                if c == ':' && app.command_input.is_empty() {
                                    app.command_input.push(':');
                                } else {
                                    app.command_input.push(c);
                                }
                                app.command_palette_selection = 0;
                            }
                            KeyCode::Backspace => {
                                app.command_input.pop();
                                app.command_palette_selection = 0;
                            }
                            KeyCode::Delete => {
                                app.command_input.clear();
                                app.command_palette_selection = 0;
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Normal mode keyboard handling
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                if app.config.confirm_destructive_actions {
                                    app.confirm_action = ConfirmAction::Quit;
                                    app.confirm_message =
                                        "Are you sure you want to quit PrimusDB TUI?".to_string();
                                } else {
                                    app.should_quit = true;
                                }
                            } else if app.current_section == NavSection::QueryConsole {
                                app.query_input.push('q');
                            }
                        }
                        KeyCode::Esc => {
                            if app.onboarding_mode {
                                app.onboarding_mode = false;
                                app.loading = false;
                            } else if app.show_contextual_help {
                                app.show_contextual_help = false;
                            } else {
                                if app.config.confirm_destructive_actions {
                                    app.confirm_action = ConfirmAction::Quit;
                                    app.confirm_message =
                                        "Are you sure you want to quit PrimusDB TUI?".to_string();
                                } else {
                                    app.should_quit = true;
                                }
                            }
                        }
                        KeyCode::Tab => {
                            app.current_section = app.current_section.next();
                        }
                        KeyCode::BackTab => {
                            app.current_section = app.current_section.prev();
                        }
                        KeyCode::Up => {
                            if !app.instances.is_empty()
                                && app.current_section == NavSection::Dashboard
                            {
                                app.selected_instance = app.selected_instance.saturating_sub(1);
                            }
                        }
                        KeyCode::Down => {
                            if !app.instances.is_empty()
                                && app.current_section == NavSection::Dashboard
                            {
                                app.selected_instance =
                                    (app.selected_instance + 1).min(app.instances.len() - 1);
                            }
                        }
                        KeyCode::Enter => {
                            if app.current_section == NavSection::QueryConsole
                                && !app.query_input.is_empty()
                                && app.connected()
                            {
                                let sql = app.query_input.trim().to_string();
                                let url = app.connected_url.clone().unwrap();
                                let query_tx = tx.clone();
                                app.add_event(format!("Executing: {}", sql));
                                let history_entry =
                                    QueryHistoryEntry::new(sql.clone(), "Running...".to_string());
                                app.query_history.push(history_entry);
                                tokio::spawn(async move {
                                    let result = api::fetch_query(&url, &sql).await;
                                    let _ = query_tx.send(AppMessage::QueryResult(result));
                                });
                            }
                        }
                        KeyCode::Char(':') => {
                            app.show_command_palette = true;
                            app.command_input.clear();
                            app.command_input.push(':');
                            app.command_palette_filtered = app.filter_commands();
                            app.command_palette_selection = 0;
                        }
                        KeyCode::Char('?') => {
                            app.show_contextual_help = !app.show_contextual_help;
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            trigger_refresh(&mut app, &tx);
                        }
                        KeyCode::Char('h') | KeyCode::Char('H') => {
                            app.current_section = NavSection::Help;
                        }
                        KeyCode::Char('m') | KeyCode::Char('M')
                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            if app.migration_wizard_active {
                                app.migration_wizard_active = false;
                                app.migration_step = 0;
                                app.command_input.clear();
                            } else {
                                app.migration_wizard_active = true;
                                app.migration_step = 0;
                                app.command_input.clear();
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Char('D')
                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            if app.connected() && app.config.confirm_destructive_actions {
                                app.confirm_action = ConfirmAction::Disconnect;
                                app.confirm_message =
                                    "Disconnect from the current server?".to_string();
                            } else if app.connected() {
                                app.disconnect();
                            }
                        }
                        KeyCode::Char('l') | KeyCode::Char('L')
                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            app.query_results.clear();
                            app.query_input.clear();
                            app.query_scroll = 0;
                            app.event_log.clear();
                            app.add_event("Display cleared".to_string());
                        }
                        KeyCode::Char('b') | KeyCode::Char('B')
                            if key.modifiers.contains(KeyModifiers::CONTROL) && app.connected() =>
                        {
                            app.backup_in_progress = true;
                            app.add_event("Creating backup...".to_string());
                            let backup_tx = tx.clone();
                            tokio::spawn(async move {
                                let output = std::process::Command::new("primusdb")
                                    .args(["backup", "create"])
                                    .output();
                                match output {
                                    Ok(out) if out.status.success() => {
                                        let msg =
                                            String::from_utf8_lossy(&out.stdout).trim().to_string();
                                        let _ = backup_tx.send(AppMessage::BackupCreated(msg));
                                    }
                                    Ok(out) => {
                                        let err =
                                            String::from_utf8_lossy(&out.stderr).trim().to_string();
                                        let _ = backup_tx.send(AppMessage::BackupCreated(format!(
                                            "Error: {}",
                                            err
                                        )));
                                    }
                                    Err(e) => {
                                        let _ = backup_tx.send(AppMessage::BackupCreated(format!(
                                            "Error: {}",
                                            e
                                        )));
                                    }
                                }
                            });
                        }
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            if app.current_section == NavSection::QueryConsole {
                                // Ctrl+C handling for query console
                                // Do nothing to avoid accidental quit
                            }
                        }
                        KeyCode::Char(c) if app.current_section == NavSection::QueryConsole => {
                            app.query_input.push(c);
                        }
                        KeyCode::Backspace if app.current_section == NavSection::QueryConsole => {
                            app.query_input.pop();
                        }
                        KeyCode::Delete if app.current_section == NavSection::QueryConsole => {
                            app.query_input.clear();
                        }
                        KeyCode::PageDown if app.current_section == NavSection::QueryConsole => {
                            app.query_scroll = app.query_scroll.saturating_add(10);
                        }
                        KeyCode::PageUp if app.current_section == NavSection::QueryConsole => {
                            app.query_scroll = app.query_scroll.saturating_sub(10);
                        }

                        _ => {
                            // Workspace-based dispatch — takes priority over legacy handlers
                            let section = app.current_section;
                            let mut consumed = false;
                            if let Some(mut workspace) = app.workspaces.remove(&section) {
                                let result = workspace.handle_key(&mut app, key.code);
                                app.workspaces.insert(section, workspace);
                                match result {
                                    crate::cli::tui::workspace::EventResult::Consumed => {
                                        consumed = true;
                                    }
                                    crate::cli::tui::workspace::EventResult::NotConsumed => {}
                                    crate::cli::tui::workspace::EventResult::Action(action) => {
                                        consumed = true;
                                        match action {
                                            crate::cli::tui::workspace::WorkspaceAction::Refresh => {
                                                if app.connected() {
                                                    trigger_refresh(&mut app, &tx);
                                                }
                                            }
                                            crate::cli::tui::workspace::WorkspaceAction::SwitchTo(new_section) => {
                                                app.current_section = new_section;
                                            }
                                            crate::cli::tui::workspace::WorkspaceAction::ExecCommand(cmd) => {
                                                app.handle_command(&cmd, &tx);
                                            }
                                            crate::cli::tui::workspace::WorkspaceAction::StatusMessage(msg) => {
                                                app.add_event(msg);
                                            }
                                            crate::cli::tui::workspace::WorkspaceAction::ErrorMessage(msg) => {
                                                app.add_event(format!("Error: {}", msg));
                                            }
                                            crate::cli::tui::workspace::WorkspaceAction::OpenCommandPalette => {
                                                app.show_command_palette = true;
                                                app.command_palette_filtered = app.filter_commands();
                                                app.command_palette_selection = 0;
                                            }
                                            crate::cli::tui::workspace::WorkspaceAction::Confirm(message, pending) => {
                                                app.confirm_message = message;
                                                app.pending_action = Some(pending);
                                                app.confirm_action = ConfirmAction::BackupDelete;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            if !consumed {
                                if app.current_section == NavSection::ConfigurationStudio {
                                    handle_config_studio_key(&mut app, key.code, &tx);
                                } else if app.current_section == NavSection::TableExplorer {
                                    handle_table_explorer_key(&mut app, key.code, &tx);
                                } else if app.current_section == NavSection::ReportBuilder {
                                    handle_report_builder_key(&mut app, key.code, &tx);
                                } else if app.current_section == NavSection::Notebook {
                                    handle_notebook_key(&mut app, key.code, &tx);
                                } else if app.current_section == NavSection::RAGWorkspace {
                                    handle_rag_key(&mut app, key.code, &tx);
                                } else if app.current_section == NavSection::Settings {
                                    handle_settings_key(&mut app, key.code, &tx);
                                }
                            } // end if !consumed
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse_event(&mut app, mouse, &tx);
                }
                Event::Resize { .. } => {}
                _ => {}
            }
        }

        while let Ok(msg) = rx.try_recv() {
            match msg {
                AppMessage::Discovery(instances) => {
                    let count = instances.len();
                    app.instances = instances;
                    app.discovery_done = true;
                    app.loading = false;
                    app.add_event(format!("Discovered {} instance(s)", count));
                }
                AppMessage::Connected(url) => {
                    app.connect_url(&url);
                    app.loading = false;
                    app.onboarding_mode = false;
                }
                AppMessage::Status(value) => {
                    app.apply_status(value);
                }
                AppMessage::QueryResult(result) => {
                    app.query_results = result.lines().map(|s| s.to_string()).collect();
                    if let Some(last) = app.query_history.last_mut() {
                        last.result = result.clone();
                    }
                    app.add_event("Query completed".to_string());
                }
                AppMessage::Metrics(data) => {
                    app.metrics_data = Some(data);
                    app.add_event("Metrics refreshed".to_string());
                }
                AppMessage::Logs(data) => {
                    app.logs_data = Some(data);
                    app.add_event("Logs refreshed".to_string());
                }
                AppMessage::Backups(data) => {
                    app.backups_data = data;
                    app.add_event("Backups refreshed".to_string());
                }
                AppMessage::Databases(data) => {
                    app.databases_data = data;
                    app.add_event("Databases refreshed".to_string());
                }
                AppMessage::Namespaces(data) => {
                    app.namespaces_data = data;
                    app.add_event("Namespaces refreshed".to_string());
                }
                AppMessage::UsersData(data) => {
                    app.users_data = data;
                    app.add_event("Users data refreshed".to_string());
                }
                AppMessage::Diagnostics(data) => {
                    app.diagnostics_data = Some(data);
                    app.add_event("Diagnostics refreshed".to_string());
                }
                AppMessage::Settings(data) => {
                    app.settings_data = data;
                }
                AppMessage::ClusterStatus(data) => {
                    app.cluster_status = data;
                    app.add_event("Cluster status updated".to_string());
                }
                AppMessage::ClusterNodes(data) => {
                    app.cluster_nodes = data;
                }
                AppMessage::ClusterHealth(data) => {
                    app.cluster_health = data;
                }
                AppMessage::ClusterEvents(data) => {
                    app.cluster_events = data;
                    app.add_event("Cluster events refreshed".to_string());
                }
                AppMessage::FederationStatus(data) => {
                    app.federation_status = data;
                    app.add_event("Federation status updated".to_string());
                }
                AppMessage::FederationClusters(data) => {
                    app.federation_clusters = data;
                }
                AppMessage::FederationDomains(data) => {
                    app.federation_domains = data;
                }
                AppMessage::BackupsDetail(data) => {
                    app.backups_detail = data;
                    app.add_event("Backup details refreshed".to_string());
                }
                AppMessage::TablesData(data) => {
                    app.tables_data = data;
                }
                AppMessage::VectorIndexesData(data) => {
                    app.vector_indexes_data = data;
                }
                AppMessage::GraphData(data) => {
                    app.graph_data = data;
                }
                AppMessage::AIMLData(data) => {
                    app.aiml_data = data;
                }
                AppMessage::RolesData(data) => {
                    app.roles_data = data;
                }
                AppMessage::Error(msg) => {
                    app.loading = false;
                    app.error_message = Some(msg.clone());
                    app.add_event(format!("Error: {}", msg));
                }
                AppMessage::MigrationResult(msg) => {
                    handle_migration_result(&mut app, msg);
                }
                AppMessage::MigrationError(msg) => {
                    handle_migration_error(&mut app, msg);
                }
                AppMessage::MigrationProgress(pct) => {
                    app.migration_progress = pct;
                    if pct >= 100 {
                        app.migration_report = format!(
                            "Migration completed.\nSource: {}\nNamespace: {}\nMode: {}",
                            app.migration_source, app.migration_namespace, app.migration_mode
                        );
                    }
                }
                AppMessage::Tick => {
                    if app.connected() {
                        let tick_tx = tx.clone();
                        if let Some(ref url) = app.connected_url.clone() {
                            let url2 = url.clone();
                            tokio::spawn(async move {
                                if let Some(status) = api::fetch_status(&url2).await {
                                    let _ = tick_tx.send(AppMessage::Status(Some(status)));
                                }
                                if let Some(metrics) = api::fetch_engine_metrics(&url2).await {
                                    let _ = tick_tx.send(AppMessage::EngineMetrics(metrics));
                                }
                            });
                        }
                    }
                }
                AppMessage::BackupCreated(result) => {
                    app.backup_in_progress = false;
                    app.add_event(format!("Backup result: {}", result));
                    let backups = api::list_backups();
                    app.backups_data = backups;
                }
                AppMessage::BackupRestored(msg) => {
                    app.add_event(msg);
                }
                AppMessage::EngineMetrics(data) => {
                    app.engine_metrics = Some(data.clone());
                    app.query_rate = api::parse_prometheus_metric(&data, "primusdb_queries_total");
                    app.error_rate = api::parse_prometheus_metric(&data, "primusdb_errors_total");
                    app.memory_usage =
                        api::parse_prometheus_metric(&data, "process_resident_memory_bytes");
                    app.storage_usage =
                        api::parse_prometheus_metric(&data, "primusdb_storage_bytes");
                }
                AppMessage::ClusterSummary(data) => {
                    app.cluster_status = data;
                }
                AppMessage::ConfigEntries(data) => {
                    if let Some(ref v) = data {
                        if let Some(arr) = v.as_array() {
                            app.config_entries = arr.clone();
                        }
                    }
                    app.config_error = None;
                }
                AppMessage::ConfigEntrySet(msg) => {
                    app.config_status = msg;
                    app.add_event("Config entry saved".to_string());
                }
                AppMessage::ConfigEntryDeleted(msg) => {
                    app.config_status = msg;
                    app.add_event("Config entry deleted".to_string());
                    let url = app.connected_url.clone();
                    if let Some(ref u) = url {
                        let fetch_tx = tx.clone();
                        let u2 = u.clone();
                        tokio::spawn(async move {
                            let entries = api::fetch_config_entries(&u2).await;
                            let _ = fetch_tx.send(AppMessage::ConfigEntries(entries));
                        });
                    }
                }
                AppMessage::ConfigSnapshots(data) => {
                    if let Some(ref v) = data {
                        if let Some(arr) = v.as_array() {
                            app.config_snapshots = arr.clone();
                        }
                    }
                }
                AppMessage::ConfigSnapshotCreated(msg) => {
                    app.config_status = msg;
                    app.add_event("Snapshot created".to_string());
                }
                AppMessage::ConfigSnapshotRestored(msg) => {
                    app.config_status = msg;
                    app.add_event("Snapshot restored".to_string());
                }
                AppMessage::ConfigError(msg) => {
                    app.config_error = Some(msg.clone());
                    app.add_event(format!("Config error: {}", msg));
                }

                // Report Builder message handling
                AppMessage::ReportsList(data) => {
                    if let Some(ref v) = data {
                        if let Some(arr) = v.as_array() {
                            app.reports_data = arr.clone();
                        }
                    }
                    app.report_error = None;
                }
                AppMessage::ReportCreated(msg) => {
                    app.report_status = msg;
                    app.add_event("Report created".to_string());
                    let url = app.connected_url.clone();
                    if let Some(ref u) = url {
                        let u2 = u.clone();
                        let fetch_tx = tx.clone();
                        tokio::spawn(async move {
                            let reports = api::fetch_reports(&u2).await;
                            let _ = fetch_tx.send(AppMessage::ReportsList(reports));
                        });
                    }
                }
                AppMessage::ReportDeleted(msg) => {
                    app.report_status = msg;
                    app.add_event("Report deleted".to_string());
                    let url = app.connected_url.clone();
                    if let Some(ref u) = url {
                        let u2 = u.clone();
                        let fetch_tx = tx.clone();
                        tokio::spawn(async move {
                            let reports = api::fetch_reports(&u2).await;
                            let _ = fetch_tx.send(AppMessage::ReportsList(reports));
                        });
                    }
                }
                AppMessage::ReportDetail(data) => {
                    app.report_detail = data;
                    app.report_error = None;
                }
                AppMessage::ReportResults(data) => {
                    app.report_results = data;
                    app.report_error = None;
                }
                AppMessage::ReportError(msg) => {
                    app.report_error = Some(msg.clone());
                    app.add_event(format!("Report error: {}", msg));
                }

                // Notebook message handling
                AppMessage::NotebooksList(data) => {
                    if let Some(ref v) = data {
                        if let Some(arr) = v.as_array() {
                            app.notebooks_data = arr.clone();
                        }
                    }
                    app.notebook_error = None;
                }
                AppMessage::NotebookCreated(_msg) => {
                    app.notebook_status = "Notebook created".to_string();
                    let url = app.connected_url.clone();
                    if let Some(ref u) = url {
                        let u2 = u.clone();
                        let fetch_tx = tx.clone();
                        tokio::spawn(async move {
                            let notebooks = api::fetch_notebooks(&u2).await;
                            let _ = fetch_tx.send(AppMessage::NotebooksList(notebooks));
                        });
                    }
                }
                AppMessage::NotebookDeleted(_msg) => {
                    app.notebook_status = "Notebook deleted".to_string();
                    app.add_event("Notebook deleted".to_string());
                    let url = app.connected_url.clone();
                    if let Some(ref u) = url {
                        let u2 = u.clone();
                        let fetch_tx = tx.clone();
                        tokio::spawn(async move {
                            let notebooks = api::fetch_notebooks(&u2).await;
                            let _ = fetch_tx.send(AppMessage::NotebooksList(notebooks));
                        });
                    }
                }
                AppMessage::NotebookDetail(data) => {
                    app.notebook_detail = data;
                    if let Some(ref v) = app.notebook_detail {
                        if let Some(cells) = v.get("cells").and_then(|c| c.as_array()) {
                            app.notebook_cells = cells.clone();
                        }
                    }
                    app.notebook_error = None;
                }
                AppMessage::NotebookCellResult(data) => {
                    app.notebook_cell_result = data;
                    app.notebook_error = None;
                }
                AppMessage::NotebookError(msg) => {
                    app.notebook_error = Some(msg.clone());
                    app.add_event(format!("Notebook error: {}", msg));
                }

                // RAG Workspace message handling
                AppMessage::RagCollections(data) => {
                    app.rag_collections = data;
                    app.rag_error = None;
                }
                AppMessage::RagSearchResults(data) => {
                    app.rag_results = data;
                    app.rag_error = None;
                }
                AppMessage::RagError(msg) => {
                    app.rag_error = Some(msg.clone());
                    app.add_event(format!("RAG error: {}", msg));
                }

                // ExecuteAction — dispatched from workspace ExecCommand via handle_command
                AppMessage::ExecuteAction(action) => {
                    dispatch_action(&mut app, &action, &tx);
                }

                // Table CRUD
                AppMessage::TableCreated(msg) => {
                    app.add_event(msg.clone());
                    trigger_refresh(&mut app, &tx);
                }
                AppMessage::TableDropped(msg) => {
                    app.add_event(msg.clone());
                    trigger_refresh(&mut app, &tx);
                }
                AppMessage::TableError(e) => {
                    app.add_event(format!("Table error: {}", e));
                }

                // Database creation
                AppMessage::DatabaseCreated(msg) => {
                    app.add_event(msg);
                    trigger_refresh(&mut app, &tx);
                }
                AppMessage::DatabaseError(e) => {
                    app.add_event(format!("Database error: {}", e));
                }

                // Document CRUD
                AppMessage::DocumentCreated(msg) => {
                    app.add_event(msg);
                }
                AppMessage::DocumentUpdated(msg) => {
                    app.add_event(msg);
                }
                AppMessage::DocumentDeleted(msg) => {
                    app.add_event(msg);
                }
                AppMessage::DocumentError(e) => {
                    app.add_event(format!("Document error: {}", e));
                }

                // DDL
                AppMessage::ColumnAdded(msg) => {
                    app.add_event(msg);
                }
                AppMessage::ColumnDropped(msg) => {
                    app.add_event(msg);
                }
                AppMessage::ColumnModified(msg) => {
                    app.add_event(msg);
                }
                AppMessage::ConstraintAdded(msg) => {
                    app.add_event(msg);
                }
                AppMessage::ConstraintDropped(msg) => {
                    app.add_event(msg);
                }
                AppMessage::TableRenamed(msg) => {
                    app.add_event(msg);
                }
                AppMessage::DdlError(e) => {
                    app.add_event(format!("DDL error: {}", e));
                }

                // User CRUD
                AppMessage::UserCreated(msg) => {
                    app.add_event(msg);
                }
                AppMessage::UserDeleted(msg) => {
                    app.add_event(msg);
                }
                AppMessage::UserError(e) => {
                    app.add_event(format!("User error: {}", e));
                }

                // Role CRUD
                AppMessage::RoleCreated(msg) => {
                    app.add_event(msg);
                }
                AppMessage::RoleDeleted(msg) => {
                    app.add_event(msg);
                }
                AppMessage::RoleError(e) => {
                    app.add_event(format!("Role error: {}", e));
                }

                // Permissions
                AppMessage::PermissionsData(_data) => { /* permissions_data field not yet wired */ }
                AppMessage::UserRoleAssigned(msg) => {
                    app.add_event(msg);
                }
                AppMessage::UserRoleRemoved(msg) => {
                    app.add_event(msg);
                }
                AppMessage::PermissionError(e) => {
                    app.add_event(format!("Permission error: {}", e));
                }

                // Namespace CRUD
                AppMessage::NamespaceCreated(msg) => {
                    app.add_event(msg);
                    trigger_refresh(&mut app, &tx);
                }
                AppMessage::NamespaceDeleted(msg) => {
                    app.add_event(msg);
                    trigger_refresh(&mut app, &tx);
                }
                AppMessage::NamespaceError(e) => {
                    app.add_event(format!("Namespace error: {}", e));
                }

                // Governor
                AppMessage::GovernorStatusData(d) => {
                    app.governor_status = d;
                }
                AppMessage::GovernorExecutionsData(d) => {
                    if let Some(arr) = d.and_then(|v| v.as_array().cloned()) {
                        app.governor_executions = arr.iter().map(|v| v.to_string()).collect();
                    }
                }
                AppMessage::GovernorViolationsData(d) => {
                    if let Some(arr) = d.and_then(|v| v.as_array().cloned()) {
                        app.governor_violations = arr.iter().map(|v| v.to_string()).collect();
                    }
                }
                AppMessage::GovernorMetricsData(d) => {
                    app.governor_metrics = d;
                }
                AppMessage::GovernorPolicySet(msg) => {
                    app.add_event(msg);
                }
                AppMessage::GovernorPolicyDeleted(msg) => {
                    app.add_event(msg);
                }
                AppMessage::GovernorError(e) => {
                    app.add_event(format!("Governor error: {}", e));
                }

                // Federation
                AppMessage::FederationClusterAdded(msg) => {
                    app.add_event(msg);
                }
                AppMessage::FederationClusterRemoved(msg) => {
                    app.add_event(msg);
                }
                AppMessage::FederationDomainCreated(msg) => {
                    app.add_event(msg);
                }
                AppMessage::FederationDomainDeleted(msg) => {
                    app.add_event(msg);
                }
                AppMessage::FederationError(e) => {
                    app.add_event(format!("Federation error: {}", e));
                }

                // Cluster
                AppMessage::ClusterStarted(msg) => {
                    app.add_event(msg);
                    app.loading = false;
                }
                AppMessage::ClusterStopped(msg) => {
                    app.add_event(msg);
                    app.loading = false;
                }
                AppMessage::ClusterRestarted(msg) => {
                    app.add_event(msg);
                    app.loading = false;
                }
                AppMessage::ClusterJoined(msg) => {
                    app.add_event(msg);
                    app.loading = false;
                }
                AppMessage::ClusterLeft(msg) => {
                    app.add_event(msg);
                    app.loading = false;
                }
                AppMessage::ClusterNodeRemoved(msg) => {
                    app.add_event(msg);
                    app.loading = false;
                }
                AppMessage::ClusterMaintenanceToggled(msg) => {
                    app.add_event(msg);
                    app.loading = false;
                }
                AppMessage::ClusterError(e) => {
                    app.add_event(format!("Cluster error: {}", e));
                    app.loading = false;
                }

                // Backup
                AppMessage::BackupVerified(msg) => {
                    app.add_event(msg);
                }
                AppMessage::BackupDeleted(msg) => {
                    app.add_event(msg);
                }

                // Export/Import
                AppMessage::ExportProgress(phase, progress, msg) => {
                    app.export_phase = phase;
                    app.export_progress = progress;
                    app.add_event(msg);
                }

                // Doctor
                AppMessage::DoctorResult(result) => {
                    app.doctor_results = result.lines().map(|s| s.to_string()).collect();
                    app.add_event("Diagnostics complete.".to_string());
                }

                // Files
                AppMessage::FileEntries(entries) => {
                    app.file_entries = entries;
                }
                AppMessage::FileContent(content) => {
                    app.file_content = content;
                }
                AppMessage::FileDeleted(msg) => {
                    app.add_event(msg);
                }
                AppMessage::FileError(e) => {
                    app.add_event(format!("File error: {}", e));
                }

                // RefreshAll
                AppMessage::RefreshAll => {
                    if app.connected() {
                        trigger_refresh(&mut app, &tx);
                    }
                }

                // Table Explorer message handling
                AppMessage::ExplorerStorageTypes(data) => {
                    app.explorer_storage_types = data;
                }
                AppMessage::ExplorerTables(data) => {
                    app.explorer_tables_data = data;
                    app.explorer_error = None;
                }
                AppMessage::ExplorerTableInfo(data) => {
                    app.explorer_table_info = data;
                    app.explorer_error = None;
                }
                AppMessage::ExplorerRows(data) => {
                    app.explorer_rows_data = data;
                    app.explorer_error = None;
                }
                AppMessage::ExplorerError(msg) => {
                    app.explorer_error = Some(msg.clone());
                    app.add_event(format!("Table Explorer error: {}", msg));
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    restore_terminal(&mut terminal, app.mouse_enabled)?;
    Ok(())
}

fn handle_mouse_event(
    app: &mut TuiApp,
    mouse: MouseEvent,
    _tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>,
) {
    use MouseEventKind::*;

    let sidebar_start = 0u16;
    let sidebar_end = SIDEBAR_WIDTH + 1;

    match mouse.kind {
        ScrollDown => {
            if app.current_section == NavSection::QueryConsole {
                app.query_scroll = app.query_scroll.saturating_add(3);
            }
        }
        ScrollUp => {
            if app.current_section == NavSection::QueryConsole {
                app.query_scroll = app.query_scroll.saturating_sub(3);
            }
        }
        Down(MouseButton::Left) => {
            let col = mouse.column;
            let row = mouse.row;

            // Sidebar click -> navigate
            if col >= sidebar_start && col < sidebar_end {
                let sidebar_item = row.saturating_sub(2) as usize;
                if sidebar_item < NAV_SECTIONS.len() {
                    app.current_section = NAV_SECTIONS[sidebar_item];
                    app.add_event(format!("Navigated to {}", app.current_section.name()));
                }
            }
        }
        _ => {}
    }
}

fn handle_config_studio_key(
    app: &mut TuiApp,
    key: KeyCode,
    tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>,
) {
    match app.config_mode {
        ConfigStudioMode::List => match key {
            KeyCode::Up => {
                app.config_selected_index = app.config_selected_index.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = app.config_entries.len().saturating_sub(1);
                app.config_selected_index = app.config_selected_index.min(max);
                if app.config_selected_index < max {
                    app.config_selected_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(entry) = app.config_entries.get(app.config_selected_index) {
                    app.config_detail_entry = Some(entry.clone());
                    app.config_mode = ConfigStudioMode::Detail;
                }
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if let Some(entry) = app.config_entries.get(app.config_selected_index) {
                    app.config_detail_entry = Some(entry.clone());
                    app.config_mode = ConfigStudioMode::Edit;
                    if let Some(v) = entry.get("value") {
                        app.config_input = serde_json::to_string_pretty(v).unwrap_or_default();
                    } else {
                        app.config_input = String::new();
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.config_mode = ConfigStudioMode::NewEntry;
                app.config_input = String::new();
                app.config_error = None;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if !app.config_entries.is_empty() {
                    app.config_mode = ConfigStudioMode::ConfirmDelete;
                    app.config_error = None;
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                let url = app.connected_url.clone();
                if let Some(ref u) = url {
                    let fetch_tx = tx.clone();
                    let u2 = u.clone();
                    tokio::spawn(async move {
                        let snapshots = api::fetch_config_snapshots(&u2).await;
                        let _ = fetch_tx.send(AppMessage::ConfigSnapshots(snapshots));
                    });
                }
                app.config_mode = ConfigStudioMode::Snapshots;
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                app.config_mode = ConfigStudioMode::ImportExport;
            }
            _ => {}
        },
        ConfigStudioMode::Detail => match key {
            KeyCode::Esc => {
                app.config_mode = ConfigStudioMode::List;
                app.config_detail_entry = None;
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                app.config_mode = ConfigStudioMode::Edit;
                if let Some(ref entry) = app.config_detail_entry {
                    if let Some(v) = entry.get("value") {
                        app.config_input = serde_json::to_string_pretty(v).unwrap_or_default();
                    }
                }
            }
            _ => {}
        },
        ConfigStudioMode::Edit => match key {
            KeyCode::Esc => {
                app.config_mode = ConfigStudioMode::Detail;
                app.config_input.clear();
                app.config_error = None;
            }
            KeyCode::Enter => {
                let entry = app.config_detail_entry.clone();
                if let Some(ref e) = entry {
                    if let Some(key) = e.get("key").and_then(|k| k.as_str()) {
                        let url = app.connected_url.clone();
                        if let Some(ref u) = url {
                            let u2 = u.clone();
                            let k2 = key.to_string();
                            let input = app.config_input.clone();
                            let set_tx = tx.clone();
                            tokio::spawn(async move {
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&input) {
                                    match api::set_config_entry(&u2, &k2, val, "runtime").await {
                                        Ok(data) => {
                                            let msg = serde_json::to_string_pretty(&data)
                                                .unwrap_or_default();
                                            let _ = set_tx.send(AppMessage::ConfigEntrySet(msg));
                                            let entries = api::fetch_config_entries(&u2).await;
                                            let _ = set_tx.send(AppMessage::ConfigEntries(entries));
                                        }
                                        Err(e) => {
                                            let _ = set_tx.send(AppMessage::ConfigError(e));
                                        }
                                    }
                                } else {
                                    let _ = set_tx.send(AppMessage::ConfigError(
                                        "Invalid JSON value".to_string(),
                                    ));
                                }
                            });
                        }
                        app.config_mode = ConfigStudioMode::List;
                        app.config_detail_entry = None;
                        app.config_input.clear();
                    }
                }
            }
            KeyCode::Char(c) => {
                app.config_input.push(c);
            }
            KeyCode::Backspace => {
                app.config_input.pop();
            }
            _ => {}
        },
        ConfigStudioMode::NewEntry => match key {
            KeyCode::Esc => {
                app.config_mode = ConfigStudioMode::List;
                app.config_input.clear();
                app.config_error = None;
            }
            KeyCode::Enter => {
                if !app.config_input.is_empty() {
                    let url = app.connected_url.clone();
                    if let Some(ref u) = url {
                        let u2 = u.clone();
                        let input = app.config_input.clone();
                        let set_tx = tx.clone();
                        tokio::spawn(async move {
                            let parts: Vec<&str> = input.splitn(2, '=').collect();
                            if parts.len() == 2 {
                                let key = parts[0].trim();
                                let val_str = parts[1].trim();
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(val_str)
                                {
                                    match api::set_config_entry(&u2, key, val, "runtime").await {
                                        Ok(data) => {
                                            let msg = serde_json::to_string_pretty(&data)
                                                .unwrap_or_default();
                                            let _ = set_tx.send(AppMessage::ConfigEntrySet(msg));
                                            let entries = api::fetch_config_entries(&u2).await;
                                            let _ = set_tx.send(AppMessage::ConfigEntries(entries));
                                        }
                                        Err(e) => {
                                            let _ = set_tx.send(AppMessage::ConfigError(e));
                                        }
                                    }
                                } else {
                                    let _ = set_tx.send(AppMessage::ConfigError(
                                        "Invalid JSON value".to_string(),
                                    ));
                                }
                            } else {
                                let _ = set_tx.send(AppMessage::ConfigError(
                                    "Use format: key=value".to_string(),
                                ));
                            }
                        });
                    }
                    app.config_mode = ConfigStudioMode::List;
                    app.config_input.clear();
                }
            }
            KeyCode::Char(c) => {
                app.config_input.push(c);
            }
            KeyCode::Backspace => {
                app.config_input.pop();
            }
            _ => {}
        },
        ConfigStudioMode::ConfirmDelete => match key {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if let Some(entry) = app.config_entries.get(app.config_selected_index) {
                    if let Some(key) = entry.get("key").and_then(|k| k.as_str()) {
                        let url = app.connected_url.clone();
                        if let Some(ref u) = url {
                            let u2 = u.clone();
                            let k2 = key.to_string();
                            let del_tx = tx.clone();
                            tokio::spawn(async move {
                                match api::delete_config_entry(&u2, &k2).await {
                                    Ok(()) => {
                                        let _ = del_tx.send(AppMessage::ConfigEntryDeleted(
                                            "Deleted".to_string(),
                                        ));
                                    }
                                    Err(e) => {
                                        let _ = del_tx.send(AppMessage::ConfigError(e));
                                    }
                                }
                            });
                        }
                    }
                }
                app.config_mode = ConfigStudioMode::List;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.config_mode = ConfigStudioMode::List;
            }
            _ => {}
        },
        ConfigStudioMode::Snapshots => match key {
            KeyCode::Esc => {
                app.config_mode = ConfigStudioMode::List;
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                app.config_mode = ConfigStudioMode::CreateSnapshot;
                app.config_input = String::new();
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if !app.config_snapshots.is_empty() {
                    if let Some(snap) = app.config_snapshots.first() {
                        if let Some(id) = snap.get("id").and_then(|v| v.as_str()) {
                            let url = app.connected_url.clone();
                            if let Some(ref u) = url {
                                let u2 = u.clone();
                                let sid = id.to_string();
                                let rest_tx = tx.clone();
                                tokio::spawn(async move {
                                    match api::restore_config_snapshot(&u2, &sid).await {
                                        Ok(count) => {
                                            let _ =
                                                rest_tx.send(AppMessage::ConfigSnapshotRestored(
                                                    format!("Restored {} entries", count),
                                                ));
                                            let entries = api::fetch_config_entries(&u2).await;
                                            let _ =
                                                rest_tx.send(AppMessage::ConfigEntries(entries));
                                        }
                                        Err(e) => {
                                            let _ = rest_tx.send(AppMessage::ConfigError(e));
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if !app.config_snapshots.is_empty() {
                    if let Some(snap) = app.config_snapshots.first() {
                        if let Some(id) = snap.get("id").and_then(|v| v.as_str()) {
                            let url = app.connected_url.clone();
                            if let Some(ref u) = url {
                                let u2 = u.clone();
                                let sid = id.to_string();
                                let del_tx = tx.clone();
                                tokio::spawn(async move {
                                    match api::delete_config_snapshot(&u2, &sid).await {
                                        Ok(()) => {
                                            let snapshots = api::fetch_config_snapshots(&u2).await;
                                            let _ =
                                                del_tx.send(AppMessage::ConfigSnapshots(snapshots));
                                        }
                                        Err(e) => {
                                            let _ = del_tx.send(AppMessage::ConfigError(e));
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        },
        ConfigStudioMode::CreateSnapshot => match key {
            KeyCode::Esc => {
                app.config_mode = ConfigStudioMode::Snapshots;
                app.config_input.clear();
            }
            KeyCode::Enter => {
                if !app.config_input.is_empty() {
                    let url = app.connected_url.clone();
                    if let Some(ref u) = url {
                        let u2 = u.clone();
                        let name = app.config_input.clone();
                        let snap_tx = tx.clone();
                        tokio::spawn(async move {
                            match api::create_config_snapshot(
                                &u2,
                                &name,
                                "Created from TUI Config Studio",
                            )
                            .await
                            {
                                Ok(id) => {
                                    let _ = snap_tx.send(AppMessage::ConfigSnapshotCreated(
                                        format!("Snapshot '{}' created (id: {})", name, id),
                                    ));
                                    let snapshots = api::fetch_config_snapshots(&u2).await;
                                    let _ = snap_tx.send(AppMessage::ConfigSnapshots(snapshots));
                                }
                                Err(e) => {
                                    let _ = snap_tx.send(AppMessage::ConfigError(e));
                                }
                            }
                        });
                    }
                    app.config_mode = ConfigStudioMode::Snapshots;
                    app.config_input.clear();
                }
            }
            KeyCode::Char(c) => {
                app.config_input.push(c);
            }
            KeyCode::Backspace => {
                app.config_input.pop();
            }
            _ => {}
        },
        ConfigStudioMode::ImportExport => match key {
            KeyCode::Esc => {
                app.config_mode = ConfigStudioMode::List;
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                let url = app.connected_url.clone();
                if let Some(ref u) = url {
                    let u2 = u.clone();
                    let exp_tx = tx.clone();
                    tokio::spawn(async move {
                        match api::export_config_bundle(&u2).await {
                            Ok(bundle) => {
                                let pretty =
                                    serde_json::to_string_pretty(&bundle).unwrap_or_default();
                                let _ = exp_tx.send(AppMessage::ConfigEntrySet(pretty));
                            }
                            Err(e) => {
                                let _ = exp_tx.send(AppMessage::ConfigError(e));
                            }
                        }
                    });
                }
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                if !app.config_input.is_empty() {
                    let url = app.connected_url.clone();
                    if let Some(ref u) = url {
                        let u2 = u.clone();
                        let input = app.config_input.clone();
                        let imp_tx = tx.clone();
                        tokio::spawn(async move {
                            if let Ok(bundle) = serde_json::from_str::<serde_json::Value>(&input) {
                                match api::import_config_bundle(&u2, &bundle).await {
                                    Ok(count) => {
                                        let _ = imp_tx.send(AppMessage::ConfigEntrySet(format!(
                                            "Imported {} entries",
                                            count
                                        )));
                                        let entries = api::fetch_config_entries(&u2).await;
                                        let _ = imp_tx.send(AppMessage::ConfigEntries(entries));
                                    }
                                    Err(e) => {
                                        let _ = imp_tx.send(AppMessage::ConfigError(e));
                                    }
                                }
                            } else {
                                let _ = imp_tx
                                    .send(AppMessage::ConfigError("Invalid JSON".to_string()));
                            }
                        });
                    }
                    app.config_input.clear();
                }
            }
            KeyCode::Char(c) => {
                app.config_input.push(c);
            }
            KeyCode::Backspace => {
                app.config_input.pop();
            }
            _ => {}
        },
    }
}

fn handle_table_explorer_key(
    app: &mut TuiApp,
    key: KeyCode,
    tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>,
) {
    match app.table_explorer_mode {
        TableExplorerMode::StorageTypeSelect => match key {
            KeyCode::Up => {
                app.explorer_selected_st_index = app.explorer_selected_st_index.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = app.explorer_storage_types.len().saturating_sub(1);
                app.explorer_selected_st_index = app.explorer_selected_st_index.min(max);
                if app.explorer_selected_st_index < max {
                    app.explorer_selected_st_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(st) = app
                    .explorer_storage_types
                    .get(app.explorer_selected_st_index)
                {
                    app.explorer_selected_st = Some(st.clone());
                    app.explorer_selected_table_index = 0;
                    app.explorer_table_info = None;
                    app.explorer_rows_data = None;
                    app.explorer_error = None;
                    let u2 = app.connected_url.clone();
                    let st2 = st.clone();
                    let fetch_tx = tx.clone();
                    if let Some(ref url) = u2 {
                        let url2 = url.clone();
                        tokio::spawn(async move {
                            let tables = api::fetch_explorer_tables(&url2, &st2).await;
                            let _ = fetch_tx.send(AppMessage::ExplorerTables(tables));
                        });
                    }
                    app.table_explorer_mode = TableExplorerMode::TableList;
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {}
            _ => {}
        },
        TableExplorerMode::TableList => match key {
            KeyCode::Up => {
                app.explorer_selected_table_index =
                    app.explorer_selected_table_index.saturating_sub(1);
            }
            KeyCode::Down => {
                let table_count = app
                    .explorer_tables_data
                    .as_ref()
                    .and_then(|v| v.get("tables"))
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0)
                    .saturating_sub(1);
                app.explorer_selected_table_index =
                    app.explorer_selected_table_index.min(table_count);
                if app.explorer_selected_table_index < table_count {
                    app.explorer_selected_table_index += 1;
                }
            }
            KeyCode::Enter => {
                let st = app.explorer_selected_st.clone().unwrap_or_default();
                let table_name = app
                    .explorer_tables_data
                    .as_ref()
                    .and_then(|v| v.get("tables"))
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.get(app.explorer_selected_table_index))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        app.explorer_tables_data
                            .as_ref()
                            .and_then(|v| v.get("tables"))
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.get(app.explorer_selected_table_index))
                            .and_then(|v| v.get("name"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    });
                if let Some(ref table) = table_name {
                    if let Some(ref url) = app.connected_url {
                        let u2 = url.clone();
                        let st2 = st.clone();
                        let t2 = table.clone();
                        let info_tx = tx.clone();
                        tokio::spawn(async move {
                            let info = api::fetch_explorer_table_info(&u2, &st2, &t2).await;
                            let _ = info_tx.send(AppMessage::ExplorerTableInfo(info));
                        });
                    }
                    app.table_explorer_mode = TableExplorerMode::TableDetail;
                }
            }
            KeyCode::Esc => {
                app.table_explorer_mode = TableExplorerMode::StorageTypeSelect;
                app.explorer_selected_st = None;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {}
            _ => {}
        },
        TableExplorerMode::TableDetail => match key {
            KeyCode::Esc => {
                app.table_explorer_mode = TableExplorerMode::TableList;
                app.explorer_table_info = None;
            }
            KeyCode::Enter => {
                let st = app.explorer_selected_st.clone().unwrap_or_default();
                let t2 = app
                    .explorer_table_info
                    .as_ref()
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !t2.is_empty() {
                    if let Some(ref url) = app.connected_url {
                        let u2 = url.clone();
                        let st2 = st.clone();
                        let rows_tx = tx.clone();
                        tokio::spawn(async move {
                            let rows = api::fetch_explorer_rows(&u2, &st2, &t2, 50, 0, None).await;
                            let _ = rows_tx.send(AppMessage::ExplorerRows(rows));
                        });
                    }
                    app.explorer_row_offset = 0;
                    app.table_explorer_mode = TableExplorerMode::RowBrowser;
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {}
            _ => {}
        },
        TableExplorerMode::RowBrowser => match key {
            KeyCode::Esc => {
                app.table_explorer_mode = TableExplorerMode::TableDetail;
                app.explorer_rows_data = None;
            }
            KeyCode::Right | KeyCode::Char('n') | KeyCode::Char('N') => {
                let total = app.explorer_row_total;
                let limit = app.explorer_row_limit;
                let new_offset = app.explorer_row_offset + limit;
                if new_offset < total {
                    app.explorer_row_offset = new_offset;
                    let u2 = app.connected_url.clone();
                    let st2 = app.explorer_selected_st.clone().unwrap_or_default();
                    let t2 = app
                        .explorer_table_info
                        .as_ref()
                        .and_then(|v| v.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let offset = app.explorer_row_offset;
                    let rows_tx = tx.clone();
                    if !t2.is_empty() {
                        tokio::spawn(async move {
                            let rows = api::fetch_explorer_rows(
                                &u2.unwrap_or_default(),
                                &st2,
                                &t2,
                                50,
                                offset,
                                None,
                            )
                            .await;
                            let _ = rows_tx.send(AppMessage::ExplorerRows(rows));
                        });
                    }
                }
            }
            KeyCode::Left | KeyCode::Char('p') | KeyCode::Char('P') => {
                let limit = app.explorer_row_limit;
                let new_offset = app.explorer_row_offset.saturating_sub(limit);
                if new_offset != app.explorer_row_offset {
                    app.explorer_row_offset = new_offset;
                    let u2 = app.connected_url.clone();
                    let st2 = app.explorer_selected_st.clone().unwrap_or_default();
                    let t2 = app
                        .explorer_table_info
                        .as_ref()
                        .and_then(|v| v.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let offset = app.explorer_row_offset;
                    let rows_tx = tx.clone();
                    if !t2.is_empty() {
                        tokio::spawn(async move {
                            let rows = api::fetch_explorer_rows(
                                &u2.unwrap_or_default(),
                                &st2,
                                &t2,
                                50,
                                offset,
                                None,
                            )
                            .await;
                            let _ = rows_tx.send(AppMessage::ExplorerRows(rows));
                        });
                    }
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {}
            _ => {}
        },
        TableExplorerMode::ExportOptions => {
            if key == KeyCode::Esc {
                app.table_explorer_mode = TableExplorerMode::TableDetail;
            }
        }
        _ => {}
    }
}

fn handle_settings_key(
    app: &mut TuiApp,
    key: KeyCode,
    _tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>,
) {
    match app.settings_mode {
        SettingsMode::View => match key {
            KeyCode::Char('i') | KeyCode::Char('I') => {
                app.settings_mode = SettingsMode::EditRefreshInterval;
                app.settings_input.clear();
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                app.mouse_enabled = !app.mouse_enabled;
                app.config.mouse_enabled = app.mouse_enabled;
                app.settings_mode = SettingsMode::ToggleMouse;
            }
            _ => {}
        },
        SettingsMode::EditRefreshInterval => match key {
            KeyCode::Enter => {
                let val = app.settings_input.trim();
                if !val.is_empty() {
                    if let Ok(ms) = val.parse::<u64>() {
                        if (500..=60000).contains(&ms) {
                            app.config.refresh_interval_ms = ms;
                            app.add_event(format!("Refresh interval set to {}ms", ms));
                        }
                    }
                }
                app.settings_mode = SettingsMode::View;
                app.settings_input.clear();
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                app.settings_input.push(c);
            }
            KeyCode::Backspace => {
                app.settings_input.pop();
            }
            KeyCode::Esc => {
                app.settings_mode = SettingsMode::View;
                app.settings_input.clear();
            }
            _ => {}
        },
        SettingsMode::ToggleMouse => {
            if key == KeyCode::Esc || key == KeyCode::Enter || key == KeyCode::Char(' ') {
                app.settings_mode = SettingsMode::View;
            }
        }
        _ => {}
    }
}

fn handle_rag_key(
    app: &mut TuiApp,
    key: KeyCode,
    tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>,
) {
    match app.rag_mode {
        RagWorkspaceMode::CollectionSelect => match key {
            KeyCode::Up => {
                app.rag_selected_index = app.rag_selected_index.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = app.rag_collections.len().saturating_sub(1);
                app.rag_selected_index = app.rag_selected_index.min(max);
                if app.rag_selected_index < max {
                    app.rag_selected_index += 1;
                }
            }
            KeyCode::Enter => {
                if !app.rag_collections.is_empty() {
                    app.rag_query_text.clear();
                    app.rag_results = None;
                    app.rag_error = None;
                    app.rag_mode = RagWorkspaceMode::SearchConfig;
                }
            }
            _ => {}
        },
        RagWorkspaceMode::SearchConfig => match key {
            KeyCode::Enter => {
                let collection = app
                    .rag_collections
                    .get(app.rag_selected_index)
                    .cloned()
                    .unwrap_or_default();
                if !collection.is_empty() && !app.rag_query_text.is_empty() {
                    let u2 = app.connected_url.clone();
                    let collection2 = collection;
                    let query = app.rag_query_text.clone();
                    let limit = app.rag_limit;
                    let search_tx = tx.clone();
                    if let Some(ref url) = u2 {
                        let url2 = url.clone();
                        tokio::spawn(async move {
                            let result = api::rag_search(&url2, &collection2, &query, limit).await;
                            let _ = search_tx.send(AppMessage::RagSearchResults(result));
                        });
                    }
                    app.rag_mode = RagWorkspaceMode::SearchResults;
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                app.rag_limit = app.rag_limit.saturating_add(5).min(100);
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                app.rag_limit = app.rag_limit.saturating_sub(5).max(1);
            }
            KeyCode::Backspace => {
                app.rag_query_text.pop();
            }
            KeyCode::Char(c) => {
                app.rag_query_text.push(c);
            }
            KeyCode::Esc => {
                app.rag_mode = RagWorkspaceMode::CollectionSelect;
                app.rag_query_text.clear();
                app.rag_results = None;
                app.rag_error = None;
            }
            _ => {}
        },
        RagWorkspaceMode::SearchResults => {
            if key == KeyCode::Esc {
                app.rag_mode = RagWorkspaceMode::SearchConfig;
                app.rag_results = None;
            }
        }
        _ => {}
    }
}

fn handle_notebook_key(
    app: &mut TuiApp,
    key: KeyCode,
    tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>,
) {
    match app.notebook_mode {
        NotebookBuilderMode::List => match key {
            KeyCode::Up => {
                app.notebook_selected_index = app.notebook_selected_index.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = app.notebooks_data.len().saturating_sub(1);
                app.notebook_selected_index = app.notebook_selected_index.min(max);
                if app.notebook_selected_index < max {
                    app.notebook_selected_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(nb) = app.notebooks_data.get(app.notebook_selected_index) {
                    let id = nb
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if !id.is_empty() {
                        let u2 = app.connected_url.clone();
                        let id2 = id;
                        let info_tx = tx.clone();
                        if let Some(ref url) = u2 {
                            let url2 = url.clone();
                            tokio::spawn(async move {
                                let detail = api::fetch_notebook(&url2, &id2).await;
                                let _ = info_tx.send(AppMessage::NotebookDetail(detail));
                            });
                        }
                        app.notebook_mode = NotebookBuilderMode::Detail;
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.notebook_mode = NotebookBuilderMode::CellEdit;
                app.notebook_cell_edit = String::new();
                app.notebook_error = None;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if !app.notebooks_data.is_empty() {
                    app.notebook_mode = NotebookBuilderMode::ConfirmDelete;
                }
            }
            _ => {}
        },
        NotebookBuilderMode::Detail => match key {
            KeyCode::Up => {
                app.notebook_selected_cell = app.notebook_selected_cell.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = app.notebook_cells.len().saturating_sub(1);
                app.notebook_selected_cell = app.notebook_selected_cell.min(max);
                if app.notebook_selected_cell < max {
                    app.notebook_selected_cell += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(cell) = app.notebook_cells.get(app.notebook_selected_cell) {
                    let content = cell
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    app.notebook_cell_edit = content;
                    app.notebook_mode = NotebookBuilderMode::CellEdit;
                }
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                let id = app
                    .notebook_detail
                    .as_ref()
                    .and_then(|v| v.get("id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if !id.is_empty() {
                    let u2 = app.connected_url.clone();
                    let id2 = id;
                    let idx = app.notebook_selected_cell;
                    let exec_tx = tx.clone();
                    if let Some(ref url) = u2 {
                        let url2 = url.clone();
                        tokio::spawn(async move {
                            let result = api::execute_notebook_cell(&url2, &id2, idx).await;
                            let _ = exec_tx.send(AppMessage::NotebookCellResult(result));
                        });
                    }
                    app.notebook_mode = NotebookBuilderMode::Results;
                }
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                app.notebook_mode = NotebookBuilderMode::CellTypeSelect;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                let id = app
                    .notebook_detail
                    .as_ref()
                    .and_then(|v| v.get("id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if !id.is_empty() {
                    let u2 = app.connected_url.clone();
                    let id2 = id;
                    let cells = app.notebook_cells.clone();
                    let save_tx = tx.clone();
                    if let Some(ref url) = u2 {
                        let url2 = url.clone();
                        tokio::spawn(async move {
                            match api::update_notebook(&url2, &id2, cells).await {
                                Ok(()) => {
                                    let _ = save_tx
                                        .send(AppMessage::NotebookCreated("Saved".to_string()));
                                }
                                Err(e) => {
                                    let _ = save_tx.send(AppMessage::NotebookError(e));
                                }
                            }
                        });
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.notebook_cells.push(serde_json::json!({
                    "type": "md",
                    "content": "",
                    "result": null,
                }));
                app.notebook_selected_cell = app.notebook_cells.len().saturating_sub(1);
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if !app.notebook_cells.is_empty()
                    && app.notebook_selected_cell < app.notebook_cells.len()
                {
                    let idx = app.notebook_selected_cell;
                    let new_type = app
                        .notebook_cells
                        .get(idx)
                        .and_then(|c| c.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("md")
                        .to_string();
                    let new_content = app
                        .notebook_cells
                        .get(idx)
                        .and_then(|c| c.get("content"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    app.notebook_cells[idx] = serde_json::json!({
                        "type": new_type,
                        "content": new_content,
                        "result": null,
                    });
                }
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                if !app.notebook_cells.is_empty() {
                    app.notebook_cells.remove(app.notebook_selected_cell);
                    if app.notebook_selected_cell >= app.notebook_cells.len() {
                        app.notebook_selected_cell = app.notebook_cells.len().saturating_sub(1);
                    }
                }
            }
            KeyCode::Esc => {
                app.notebook_mode = NotebookBuilderMode::List;
                app.notebook_detail = None;
                app.notebook_cells.clear();
                app.notebook_cell_result = None;
            }
            _ => {}
        },
        NotebookBuilderMode::CellEdit => match key {
            KeyCode::Enter => {
                if !app.notebook_cell_edit.is_empty() {
                    if app.notebook_cells.is_empty() {
                        app.notebook_cells.push(serde_json::json!({
                            "type": "md",
                            "content": app.notebook_cell_edit,
                            "result": null,
                        }));
                        app.notebook_selected_cell = 0;
                    } else {
                        let idx = app.notebook_selected_cell;
                        let cell_type = app
                            .notebook_cells
                            .get(idx)
                            .and_then(|c| c.get("type"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("md")
                            .to_string();
                        if idx < app.notebook_cells.len() {
                            app.notebook_cells[idx] = serde_json::json!({
                                "type": cell_type,
                                "content": app.notebook_cell_edit,
                                "result": null,
                            });
                        }
                    }
                    app.notebook_cell_edit.clear();
                    app.notebook_mode = NotebookBuilderMode::Detail;
                }
            }
            KeyCode::Char(c) => {
                app.notebook_cell_edit.push(c);
            }
            KeyCode::Backspace => {
                app.notebook_cell_edit.pop();
            }
            KeyCode::Esc => {
                app.notebook_cell_edit.clear();
                app.notebook_mode = NotebookBuilderMode::Detail;
            }
            _ => {}
        },
        NotebookBuilderMode::CellTypeSelect => match key {
            KeyCode::Char('m') | KeyCode::Char('M') => {
                if app.notebook_selected_cell < app.notebook_cells.len() {
                    let content = app
                        .notebook_cells
                        .get(app.notebook_selected_cell)
                        .and_then(|c| c.get("content"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    app.notebook_cells[app.notebook_selected_cell] = serde_json::json!({
                        "type": "md",
                        "content": content,
                        "result": null,
                    });
                }
                app.notebook_mode = NotebookBuilderMode::Detail;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if app.notebook_selected_cell < app.notebook_cells.len() {
                    let content = app
                        .notebook_cells
                        .get(app.notebook_selected_cell)
                        .and_then(|c| c.get("content"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    app.notebook_cells[app.notebook_selected_cell] = serde_json::json!({
                        "type": "sql",
                        "content": content,
                        "result": null,
                    });
                }
                app.notebook_mode = NotebookBuilderMode::Detail;
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if app.notebook_selected_cell < app.notebook_cells.len() {
                    let content = app
                        .notebook_cells
                        .get(app.notebook_selected_cell)
                        .and_then(|c| c.get("content"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    app.notebook_cells[app.notebook_selected_cell] = serde_json::json!({
                        "type": "analysis",
                        "content": content,
                        "result": null,
                    });
                }
                app.notebook_mode = NotebookBuilderMode::Detail;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if app.notebook_selected_cell < app.notebook_cells.len() {
                    let content = app
                        .notebook_cells
                        .get(app.notebook_selected_cell)
                        .and_then(|c| c.get("content"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    app.notebook_cells[app.notebook_selected_cell] = serde_json::json!({
                        "type": "rag",
                        "content": content,
                        "result": null,
                    });
                }
                app.notebook_mode = NotebookBuilderMode::Detail;
            }
            KeyCode::Esc => {
                app.notebook_mode = NotebookBuilderMode::Detail;
            }
            _ => {}
        },
        NotebookBuilderMode::ConfirmDelete => match key {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if let Some(nb) = app.notebooks_data.get(app.notebook_selected_index) {
                    if let Some(id) = nb.get("id").and_then(|v| v.as_str()) {
                        let u2 = app.connected_url.clone();
                        let id2 = id.to_string();
                        let del_tx = tx.clone();
                        if let Some(ref url) = u2 {
                            let url2 = url.clone();
                            tokio::spawn(async move {
                                match api::delete_notebook(&url2, &id2).await {
                                    Ok(()) => {
                                        let _ = del_tx.send(AppMessage::NotebookDeleted(
                                            "Deleted".to_string(),
                                        ));
                                    }
                                    Err(e) => {
                                        let _ = del_tx.send(AppMessage::NotebookError(e));
                                    }
                                }
                            });
                        }
                    }
                }
                app.notebook_mode = NotebookBuilderMode::List;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.notebook_mode = NotebookBuilderMode::List;
            }
            _ => {}
        },
        NotebookBuilderMode::Results => {
            if key == KeyCode::Esc {
                app.notebook_mode = NotebookBuilderMode::Detail;
                app.notebook_cell_result = None;
            }
        }
    }
}

fn handle_report_builder_key(
    app: &mut TuiApp,
    key: KeyCode,
    tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>,
) {
    match app.report_mode {
        ReportBuilderMode::List => match key {
            KeyCode::Up => {
                app.report_selected_index = app.report_selected_index.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = app.reports_data.len().saturating_sub(1);
                app.report_selected_index = app.report_selected_index.min(max);
                if app.report_selected_index < max {
                    app.report_selected_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(report) = app.reports_data.get(app.report_selected_index) {
                    app.report_detail = Some(report.clone());
                    app.report_mode = ReportBuilderMode::Detail;
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.report_mode = ReportBuilderMode::Create;
                app.report_input = String::new();
                app.report_input_field = 0;
                app.report_error = None;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if !app.reports_data.is_empty() {
                    app.report_mode = ReportBuilderMode::ConfirmDelete;
                }
            }
            _ => {}
        },
        ReportBuilderMode::Detail => match key {
            KeyCode::Esc => {
                app.report_mode = ReportBuilderMode::List;
                app.report_detail = None;
                app.report_results = None;
            }
            KeyCode::Enter => {
                if let Some(ref report) = app.report_detail {
                    if let Some(id) = report.get("id").and_then(|v| v.as_str()) {
                        let u2 = app.connected_url.clone();
                        let id2 = id.to_string();
                        let info_tx = tx.clone();
                        if let Some(ref url) = u2 {
                            let url2 = url.clone();
                            tokio::spawn(async move {
                                let result = api::execute_report(&url2, &id2, Some(50), None).await;
                                let _ = info_tx.send(AppMessage::ReportResults(result));
                            });
                        }
                        app.report_mode = ReportBuilderMode::Results;
                    }
                }
            }
            _ => {}
        },
        ReportBuilderMode::Create => match key {
            KeyCode::Esc => {
                app.report_mode = ReportBuilderMode::List;
                app.report_input.clear();
                app.report_error = None;
            }
            KeyCode::Enter => {
                if !app.report_input.is_empty() {
                    let u2 = app.connected_url.clone();
                    let input = app.report_input.clone();
                    let create_tx = tx.clone();
                    if let Some(ref url) = u2 {
                        let url2 = url.clone();
                        tokio::spawn(async move {
                            let parts: Vec<&str> = input.splitn(6, '|').collect();
                            if parts.len() >= 2 {
                                let name = parts[0].trim();
                                let query = parts[1].trim();
                                let desc = parts.get(2).map(|s| s.trim()).unwrap_or("");
                                let st = parts.get(3).map(|s| s.trim()).unwrap_or("relational");
                                let fmt = parts.get(4).map(|s| s.trim()).unwrap_or("json");
                                let tbl = parts.get(5).map(|s| s.trim()).unwrap_or("");
                                match api::create_report(&url2, name, query, desc, st, fmt, tbl)
                                    .await
                                {
                                    Ok(data) => {
                                        let msg =
                                            serde_json::to_string_pretty(&data).unwrap_or_default();
                                        let _ = create_tx.send(AppMessage::ReportCreated(msg));
                                    }
                                    Err(e) => {
                                        let _ = create_tx.send(AppMessage::ReportError(e));
                                    }
                                }
                            } else {
                                let _ = create_tx.send(AppMessage::ReportError(
                                    "Format: name|query|desc|type|format|table".to_string(),
                                ));
                            }
                        });
                    }
                    app.report_mode = ReportBuilderMode::List;
                    app.report_input.clear();
                }
            }
            KeyCode::Char(c) => {
                app.report_input.push(c);
            }
            KeyCode::Backspace => {
                app.report_input.pop();
            }
            _ => {}
        },
        ReportBuilderMode::ConfirmDelete => match key {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if let Some(report) = app.reports_data.get(app.report_selected_index) {
                    if let Some(id) = report.get("id").and_then(|v| v.as_str()) {
                        let u2 = app.connected_url.clone();
                        let id2 = id.to_string();
                        let del_tx = tx.clone();
                        if let Some(ref url) = u2 {
                            let url2 = url.clone();
                            tokio::spawn(async move {
                                match api::delete_report(&url2, &id2).await {
                                    Ok(()) => {
                                        let _ = del_tx
                                            .send(AppMessage::ReportDeleted("Deleted".to_string()));
                                    }
                                    Err(e) => {
                                        let _ = del_tx.send(AppMessage::ReportError(e));
                                    }
                                }
                            });
                        }
                    }
                }
                app.report_mode = ReportBuilderMode::List;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.report_mode = ReportBuilderMode::List;
            }
            _ => {}
        },
        ReportBuilderMode::Results => {
            if key == KeyCode::Esc {
                app.report_mode = ReportBuilderMode::Detail;
                app.report_results = None;
            }
        }
        _ => {}
    }
}

fn handle_migration_result(app: &mut TuiApp, msg: String) {
    if app.migration_wizard_active {
        if app.migration_step == 3 {
            app.migration_source_connected = true;
            app.migration_status = msg.clone();
            app.migration_error = None;
            app.add_event(format!("Connection test: {}", msg));
        } else if app.migration_step == 6 {
            let lines: Vec<&str> = msg.lines().collect();
            let mut objects: Vec<String> = Vec::new();
            for line in &lines {
                let trimmed = line.trim();
                if !trimmed.is_empty()
                    && !trimmed.starts_with("Source")
                    && !trimmed.starts_with("Found")
                {
                    let clean = trimmed.trim_start_matches("•").trim();
                    if !clean.is_empty()
                        && !clean.contains("reachable")
                        && !clean.contains("simulated")
                    {
                        objects.push(clean.to_string());
                    }
                }
            }
            if objects.is_empty() {
                objects.push("default".to_string());
            }
            app.migration_objects = objects.clone();
            app.migration_selected_objects = vec![true; objects.len()];
            app.migration_status = msg.clone();
            app.add_event(format!("Inspect found {} objects", objects.len()));
        } else if app.migration_step == 9 {
            app.migration_dry_run_result = Some(msg.clone());
            app.migration_status = msg.clone();
            app.add_event("Dry-run completed".to_string());
        } else {
            app.migration_status = msg.clone();
            app.add_event(format!("Migration: {}", msg));
            if app.migration_step >= 11 {
                app.migration_report = format!(
                    "Migration completed.\nSource: {}\nNamespace: {}\nMode: {}\n\n{}",
                    app.migration_source, app.migration_namespace, app.migration_mode, msg
                );
            }
        }
    } else {
        app.migration_status = msg.clone();
        app.add_event(format!("Migration completed: {}", msg));
    }
}

fn handle_migration_error(app: &mut TuiApp, msg: String) {
    if app.migration_wizard_active {
        if app.migration_step == 3 {
            app.migration_source_connected = false;
            app.migration_error = Some(msg.clone());
            app.add_event(format!("Connection test failed: {}", msg));
        } else if app.migration_step == 9 {
            app.migration_dry_run_result = Some(format!("Error: {}", msg));
            app.migration_error = Some(msg.clone());
            app.add_event(format!("Dry-run failed: {}", msg));
        } else {
            app.migration_error = Some(msg.clone());
            app.add_event(format!("Migration error: {}", msg));
        }
    } else {
        app.migration_error = Some(msg.clone());
        app.add_event(format!("Migration error: {}", msg));
    }
}

pub async fn run_tui() -> Result<()> {
    let config = TuiConfig::default();
    let mouse_enabled = config.mouse_enabled;
    let terminal = setup_terminal(mouse_enabled).map_err(crate::Error::IOError)?;
    let app = TuiApp::new();
    run_loop(terminal, app, None, config).await
}

pub async fn run_tui_connect(url: &str) -> Result<()> {
    let config = TuiConfig::default();
    let mouse_enabled = config.mouse_enabled;
    let terminal = setup_terminal(mouse_enabled).map_err(crate::Error::IOError)?;
    let app = TuiApp::new();
    run_loop(terminal, app, Some(url.to_string()), config).await
}

pub async fn run_tui_with_config(config: TuiConfig, url: Option<String>) -> Result<()> {
    let mouse_enabled = config.mouse_enabled;
    let terminal = setup_terminal(mouse_enabled).map_err(crate::Error::IOError)?;
    let mut app = TuiApp::new();
    app.mouse_enabled = mouse_enabled;
    app.config = config.clone();
    run_loop(terminal, app, url, config).await
}
