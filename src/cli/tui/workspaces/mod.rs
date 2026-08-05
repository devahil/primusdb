use crate::cli::tui::app::{NavSection, TuiApp};
use crate::cli::tui::workspace::{EventResult, KeyBinding, Workspace, WorkspaceAction};
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::collections::HashMap;

// ── DashboardWorkspace ───────────────────────────────────────────

pub struct DashboardWorkspace;

impl Workspace for DashboardWorkspace {
    fn section(&self) -> NavSection {
        NavSection::Dashboard
    }

    fn title(&self) -> &str {
        "Dashboard"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::dashboard::render_dashboard(frame, area, app);
    }

    fn handle_key(&mut self, _app: &mut TuiApp, key: KeyCode) -> EventResult {
        match key {
            KeyCode::Char('r') | KeyCode::Char('R') => {
                EventResult::Action(WorkspaceAction::Refresh)
            }
            KeyCode::Char('?') => EventResult::Action(WorkspaceAction::SwitchTo(NavSection::Help)),
            _ => EventResult::NotConsumed,
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![("Refresh Dashboard".into(), WorkspaceAction::Refresh)]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "r",
                description: "Refresh",
                group: "Dashboard",
            },
            KeyBinding {
                keys: "?",
                description: "Help",
                group: "Global",
            },
        ]
    }
}

// ── QueryConsoleWorkspace ────────────────────────────────────────

pub struct QueryConsoleWorkspace;

impl Workspace for QueryConsoleWorkspace {
    fn section(&self) -> NavSection {
        NavSection::QueryConsole
    }

    fn title(&self) -> &str {
        "Query Console"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::queries::render_queries(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        match key {
            KeyCode::Char('h') | KeyCode::Char('H') => {
                app.show_query_history = !app.show_query_history;
                EventResult::Consumed
            }
            KeyCode::Up => {
                if app.show_query_history && !app.query_history.is_empty() {
                    app.query_history_selection = app.query_history_selection.saturating_sub(1);
                } else if !app.query_results.is_empty() {
                    app.query_scroll = app.query_scroll.saturating_sub(1);
                }
                EventResult::Consumed
            }
            KeyCode::Down => {
                if app.show_query_history && !app.query_history.is_empty() {
                    let max = app.query_history.len().saturating_sub(1);
                    app.query_history_selection =
                        app.query_history_selection.saturating_add(1).min(max);
                } else if !app.query_results.is_empty() {
                    let max_scroll = app.query_results.len().saturating_sub(1);
                    app.query_scroll = app.query_scroll.saturating_add(1).min(max_scroll);
                }
                EventResult::Consumed
            }
            KeyCode::PageUp => {
                if !app.query_results.is_empty() {
                    app.query_scroll = app.query_scroll.saturating_sub(10);
                }
                EventResult::Consumed
            }
            KeyCode::PageDown => {
                if !app.query_results.is_empty() {
                    let max_scroll = app.query_results.len().saturating_sub(1);
                    app.query_scroll = app.query_scroll.saturating_add(10).min(max_scroll);
                }
                EventResult::Consumed
            }
            KeyCode::Char('e') | KeyCode::Char('E') => EventResult::Action(
                WorkspaceAction::StatusMessage("Explain: not yet implemented".into()),
            ),
            _ => EventResult::NotConsumed,
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Toggle History".into(), WorkspaceAction::Refresh),
            (
                "Explain Query".into(),
                WorkspaceAction::StatusMessage("Explain: not yet implemented".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "h",
                description: "Toggle history",
                group: "Query",
            },
            KeyBinding {
                keys: "↑↓",
                description: "Navigate",
                group: "Query",
            },
            KeyBinding {
                keys: "PgUp/PgDn",
                description: "Scroll results",
                group: "Query",
            },
            KeyBinding {
                keys: "e",
                description: "Explain query",
                group: "Query",
            },
        ]
    }
}

// ── NamespacesWorkspace ──────────────────────────────────────────

pub struct NamespacesWorkspace;

impl Workspace for NamespacesWorkspace {
    fn section(&self) -> NavSection {
        NavSection::Namespaces
    }

    fn title(&self) -> &str {
        "Namespaces"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::namespaces::render_namespaces(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        if !app.connected() {
            return EventResult::NotConsumed;
        }
        match key {
            KeyCode::Char('n') => {
                app.command_input = ":namespace create ".to_string();
                app.show_command_palette = true;
                app.command_palette_filtered = app.filter_commands();
                app.command_palette_selection = 0;
                EventResult::Consumed
            }
            KeyCode::Char('d') => {
                if let Some(ns) = app.namespaces_data.first().cloned() {
                    EventResult::Action(WorkspaceAction::Confirm(
                        format!("Delete namespace '{}'?", ns),
                        format!("namespace_delete:{}", ns),
                    ))
                } else {
                    EventResult::NotConsumed
                }
            }
            KeyCode::Char('u') => {
                if let Some(ns) = app.namespaces_data.first().cloned() {
                    EventResult::Action(WorkspaceAction::ExecCommand(format!(
                        "namespace_use:{}",
                        ns
                    )))
                } else {
                    EventResult::NotConsumed
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                EventResult::Action(WorkspaceAction::Refresh)
            }
            _ => EventResult::NotConsumed,
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            (
                "Create Namespace".into(),
                WorkspaceAction::OpenCommandPalette,
            ),
            ("Refresh".into(), WorkspaceAction::Refresh),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "n",
                description: "Create",
                group: "Namespaces",
            },
            KeyBinding {
                keys: "u",
                description: "Use (switch)",
                group: "Namespaces",
            },
            KeyBinding {
                keys: "d",
                description: "Delete",
                group: "Namespaces",
            },
            KeyBinding {
                keys: "r",
                description: "Refresh",
                group: "Namespaces",
            },
        ]
    }
}

// ── HelpWorkspace ────────────────────────────────────────────────

pub struct HelpWorkspace;

impl Workspace for HelpWorkspace {
    fn section(&self) -> NavSection {
        NavSection::Help
    }

    fn title(&self) -> &str {
        "Help"
    }

    fn render(&self, frame: &mut Frame, area: Rect, _app: &TuiApp) {
        crate::cli::tui::sections::help::render_help_page(frame, area);
    }

    fn handle_key(&mut self, _app: &mut TuiApp, key: KeyCode) -> EventResult {
        match key {
            KeyCode::Esc => EventResult::Action(WorkspaceAction::SwitchTo(NavSection::Dashboard)),
            KeyCode::Char('1')
            | KeyCode::Char('2')
            | KeyCode::Char('3')
            | KeyCode::Char('4')
            | KeyCode::Char('5')
            | KeyCode::Char('6')
            | KeyCode::Char('7')
            | KeyCode::Char('8')
            | KeyCode::Char('9') => EventResult::Consumed,
            KeyCode::Up | KeyCode::Down => EventResult::Consumed,
            _ => EventResult::NotConsumed,
        }
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "Esc",
                description: "Close help",
                group: "Help",
            },
            KeyBinding {
                keys: "1-9",
                description: "Jump to section",
                group: "Help",
            },
            KeyBinding {
                keys: "↑↓",
                description: "Scroll",
                group: "Help",
            },
        ]
    }
}

// ── DatabasesEnginesWorkspace ────────────────────────────────────

pub struct DatabasesEnginesWorkspace;

impl Workspace for DatabasesEnginesWorkspace {
    fn section(&self) -> NavSection {
        NavSection::DatabasesEngines
    }

    fn title(&self) -> &str {
        "Databases & Engines"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        if app.engines_mode == crate::cli::tui::app::DatabasesEnginesMode::CreateDatabase {
            app.db_wizard.render(frame, area);
            return;
        }
        crate::cli::tui::sections::engines::render_engines(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        match app.engines_mode {
            crate::cli::tui::app::DatabasesEnginesMode::List => match key {
                KeyCode::Up => {
                    app.selected_table_index = app.selected_table_index.saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    let max = app.databases_data.len().saturating_sub(1);
                    app.selected_table_index = app.selected_table_index.min(max);
                    if app.selected_table_index < max {
                        app.selected_table_index += 1;
                    }
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    app.engines_mode = crate::cli::tui::app::DatabasesEnginesMode::Inspect;
                    EventResult::Action(WorkspaceAction::Refresh)
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if !app.databases_data.is_empty() {
                        app.engines_mode =
                            crate::cli::tui::app::DatabasesEnginesMode::ConfirmDelete;
                        EventResult::Consumed
                    } else {
                        EventResult::NotConsumed
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') if app.connected() => {
                    app.engines_mode = crate::cli::tui::app::DatabasesEnginesMode::CreateDatabase;
                    app.db_wizard.reset();
                    EventResult::Consumed
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    EventResult::Action(WorkspaceAction::Refresh)
                }
                _ => EventResult::NotConsumed,
            },
            crate::cli::tui::app::DatabasesEnginesMode::Inspect => {
                if key == KeyCode::Esc {
                    app.engines_mode = crate::cli::tui::app::DatabasesEnginesMode::List;
                    app.engines_detail = None;
                    EventResult::Consumed
                } else {
                    EventResult::NotConsumed
                }
            }
            crate::cli::tui::app::DatabasesEnginesMode::ConfirmDelete => match key {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    if let Some(db) = app.databases_data.get(app.selected_table_index) {
                        let db_name = db.clone();
                        app.confirm_action = crate::cli::tui::app::ConfirmAction::DropDatabase;
                        app.confirm_message = format!(
                            "Drop database '{}'?\nThis will permanently delete all tables and data.",
                            db_name
                        );
                        app.pending_action = Some(format!("db_drop:{}", db_name));
                    }
                    app.engines_mode = crate::cli::tui::app::DatabasesEnginesMode::List;
                    app.engines_detail = None;
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.engines_mode = crate::cli::tui::app::DatabasesEnginesMode::List;
                    app.engines_detail = None;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            crate::cli::tui::app::DatabasesEnginesMode::CreateDatabase => {
                use crate::cli::tui::panels::create_db_wizard::WizardAction;
                match app.db_wizard.handle_key(key) {
                    WizardAction::Continue => EventResult::Consumed,
                    WizardAction::Cancel => {
                        app.engines_mode = crate::cli::tui::app::DatabasesEnginesMode::List;
                        EventResult::Consumed
                    }
                    WizardAction::Create {
                        name,
                        description,
                        engines,
                    } => {
                        app.engines_mode = crate::cli::tui::app::DatabasesEnginesMode::List;
                        let engines_csv = engines.join(",");
                        let cmd =
                            format!("db_create_full:{}:{}:{}", name, description, engines_csv);
                        EventResult::Action(WorkspaceAction::ExecCommand(cmd))
                    }
                }
            }
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "Create Database".into(),
                WorkspaceAction::OpenCommandPalette,
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "Enter",
                description: "Inspect table",
                group: "Databases",
            },
            KeyBinding {
                keys: "n",
                description: "New database",
                group: "Databases",
            },
            KeyBinding {
                keys: "d",
                description: "Drop database",
                group: "Databases",
            },
            KeyBinding {
                keys: "r",
                description: "Refresh",
                group: "Databases",
            },
        ]
    }
}

// ── ClusterWorkspace ─────────────────────────────────────────────

pub struct ClusterWorkspace;

impl Workspace for ClusterWorkspace {
    fn section(&self) -> NavSection {
        NavSection::Cluster
    }

    fn title(&self) -> &str {
        "Cluster"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::nodes::render_nodes(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        if app.cluster_modal != crate::cli::tui::app::ClusterModal::None
            && app.cluster_modal != crate::cli::tui::app::ClusterModal::JoinPrompt
        {
            match key {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    EventResult::Action(WorkspaceAction::Refresh)
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.cluster_modal = crate::cli::tui::app::ClusterModal::None;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            }
        } else if app.cluster_modal == crate::cli::tui::app::ClusterModal::JoinPrompt {
            match key {
                KeyCode::Esc => {
                    app.cluster_modal = crate::cli::tui::app::ClusterModal::None;
                    app.cluster_join_input.clear();
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if !app.cluster_join_input.trim().is_empty() {
                        let target = app.cluster_join_input.trim().to_string();
                        app.cluster_modal = crate::cli::tui::app::ClusterModal::None;
                        app.cluster_join_input.clear();
                        EventResult::Action(WorkspaceAction::ExecCommand(format!(
                            "cluster_join:{}",
                            target
                        )))
                    } else {
                        EventResult::NotConsumed
                    }
                }
                KeyCode::Backspace => {
                    app.cluster_join_input.pop();
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.cluster_join_input.push(c);
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            }
        } else {
            match key {
                KeyCode::Up => {
                    app.selected_node_index = app.selected_node_index.saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    let max = app
                        .cluster_nodes
                        .as_ref()
                        .and_then(|v| v.as_array())
                        .map(|a| a.len().saturating_sub(1))
                        .unwrap_or(0);
                    app.selected_node_index = (app.selected_node_index + 1).min(max);
                    EventResult::Consumed
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    app.cluster_modal = crate::cli::tui::app::ClusterModal::ConfirmStart;
                    EventResult::Consumed
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    app.cluster_modal = crate::cli::tui::app::ClusterModal::ConfirmStop;
                    EventResult::Consumed
                }
                KeyCode::Char('R') => {
                    app.cluster_modal = crate::cli::tui::app::ClusterModal::ConfirmRestart;
                    EventResult::Consumed
                }
                KeyCode::Char('j') | KeyCode::Char('J') => {
                    app.cluster_modal = crate::cli::tui::app::ClusterModal::JoinPrompt;
                    app.cluster_join_input.clear();
                    EventResult::Consumed
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    app.cluster_modal = crate::cli::tui::app::ClusterModal::ConfirmLeave;
                    EventResult::Consumed
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    app.cluster_modal = crate::cli::tui::app::ClusterModal::MaintenanceToggle;
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    app.cluster_modal = crate::cli::tui::app::ClusterModal::ConfirmRemoveNode;
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if let Some(ref nodes) = app.cluster_nodes.clone() {
                        if let Some(arr) = nodes.as_array() {
                            if let Some(node) = arr.get(app.selected_node_index) {
                                app.cluster_status_msg =
                                    serde_json::to_string_pretty(node).unwrap_or_default();
                            }
                        }
                    }
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            }
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "Start Cluster".into(),
                WorkspaceAction::ExecCommand("cluster_start".into()),
            ),
            (
                "Stop Cluster".into(),
                WorkspaceAction::ExecCommand("cluster_stop".into()),
            ),
            (
                "Join Cluster".into(),
                WorkspaceAction::ExecCommand("cluster_join".into()),
            ),
            (
                "Leave Cluster".into(),
                WorkspaceAction::ExecCommand("cluster_leave".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "s",
                description: "Start cluster",
                group: "Cluster",
            },
            KeyBinding {
                keys: "t",
                description: "Stop cluster",
                group: "Cluster",
            },
            KeyBinding {
                keys: "R",
                description: "Restart cluster",
                group: "Cluster",
            },
            KeyBinding {
                keys: "j",
                description: "Join cluster",
                group: "Cluster",
            },
            KeyBinding {
                keys: "l",
                description: "Leave cluster",
                group: "Cluster",
            },
            KeyBinding {
                keys: "m",
                description: "Maintenance toggle",
                group: "Cluster",
            },
            KeyBinding {
                keys: "d",
                description: "Remove node",
                group: "Cluster",
            },
            KeyBinding {
                keys: "Enter",
                description: "Inspect node",
                group: "Cluster",
            },
        ]
    }
}

// ── FederationWorkspace ──────────────────────────────────────────

pub struct FederationWorkspace;

impl Workspace for FederationWorkspace {
    fn section(&self) -> NavSection {
        NavSection::Federation
    }

    fn title(&self) -> &str {
        "Federation"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::federation::render_federation(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        match app.federation_mode {
            crate::cli::tui::app::FederationMode::View => match key {
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    app.federation_mode = crate::cli::tui::app::FederationMode::AddCluster;
                    app.federation_input.clear();
                    app.add_event("Add cluster: enter <cluster_id> <seed_url>".to_string());
                    EventResult::Consumed
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    app.federation_mode = crate::cli::tui::app::FederationMode::RemoveCluster;
                    app.federation_input.clear();
                    app.add_event("Remove cluster: enter cluster_id".to_string());
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    app.federation_mode = crate::cli::tui::app::FederationMode::CreateDomain;
                    app.federation_input.clear();
                    app.add_event(
                        "Create domain: enter <name> <cluster_ids comma-separated>".to_string(),
                    );
                    EventResult::Consumed
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    app.federation_mode = crate::cli::tui::app::FederationMode::DeleteDomain;
                    app.federation_input.clear();
                    app.add_event("Delete domain: enter domain name".to_string());
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            crate::cli::tui::app::FederationMode::AddCluster
            | crate::cli::tui::app::FederationMode::RemoveCluster
            | crate::cli::tui::app::FederationMode::CreateDomain
            | crate::cli::tui::app::FederationMode::DeleteDomain => match key {
                KeyCode::Enter => {
                    let input = app.federation_input.trim().to_string();
                    if !input.is_empty() {
                        let cmd = match app.federation_mode {
                            crate::cli::tui::app::FederationMode::AddCluster => {
                                let parts: Vec<&str> = input.splitn(2, ' ').collect();
                                if parts.len() == 2 {
                                    Some(format!(
                                        ":federation cluster add {} {}",
                                        parts[0], parts[1]
                                    ))
                                } else {
                                    app.add_event("Usage: <cluster_id> <seed_url>".to_string());
                                    None
                                }
                            }
                            crate::cli::tui::app::FederationMode::RemoveCluster => {
                                Some(format!(":federation cluster remove {}", input))
                            }
                            crate::cli::tui::app::FederationMode::CreateDomain => {
                                Some(format!(":federation domain create {}", input))
                            }
                            crate::cli::tui::app::FederationMode::DeleteDomain => {
                                Some(format!(":federation domain delete {}", input))
                            }
                            _ => None,
                        };
                        app.federation_mode = crate::cli::tui::app::FederationMode::View;
                        app.federation_input.clear();
                        if let Some(cmd) = cmd {
                            return EventResult::Action(WorkspaceAction::ExecCommand(cmd));
                        }
                    } else {
                        app.federation_mode = crate::cli::tui::app::FederationMode::View;
                        app.federation_input.clear();
                    }
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.federation_input.push(c);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.federation_input.pop();
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.federation_mode = crate::cli::tui::app::FederationMode::View;
                    app.federation_input.clear();
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "Add Cluster".into(),
                WorkspaceAction::StatusMessage("Add cluster: enter <cluster_id> <seed_url>".into()),
            ),
            (
                "Remove Cluster".into(),
                WorkspaceAction::StatusMessage("Remove cluster: enter cluster_id".into()),
            ),
            (
                "Create Domain".into(),
                WorkspaceAction::StatusMessage("Create domain: enter <name> <cluster_ids>".into()),
            ),
            (
                "Delete Domain".into(),
                WorkspaceAction::StatusMessage("Delete domain: enter domain name".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "c",
                description: "Add cluster",
                group: "Federation",
            },
            KeyBinding {
                keys: "r",
                description: "Remove cluster",
                group: "Federation",
            },
            KeyBinding {
                keys: "d",
                description: "Create domain",
                group: "Federation",
            },
            KeyBinding {
                keys: "x",
                description: "Delete domain",
                group: "Federation",
            },
        ]
    }
}

// ── GovernorWorkspace ────────────────────────────────────────────

pub struct GovernorWorkspace;

impl Workspace for GovernorWorkspace {
    fn section(&self) -> NavSection {
        NavSection::Governor
    }

    fn title(&self) -> &str {
        "Governor"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::governor::render_governor(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        match app.governor_mode {
            crate::cli::tui::app::GovernorMode::View => match key {
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    app.governor_mode = crate::cli::tui::app::GovernorMode::SetPolicy;
                    app.governor_policy_input.clear();
                    app.add_event("Enter: <policy_name> <json> to set a policy".to_string());
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    app.governor_mode = crate::cli::tui::app::GovernorMode::ConfirmDelete;
                    app.governor_policy_name = app
                        .governor_status
                        .as_ref()
                        .and_then(|v| v.get("default_policy"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("default")
                        .to_string();
                    app.add_event("Delete default policy? (y/n)".to_string());
                    EventResult::Consumed
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    EventResult::Action(WorkspaceAction::Refresh)
                }
                _ => EventResult::NotConsumed,
            },
            crate::cli::tui::app::GovernorMode::SetPolicy => match key {
                KeyCode::Enter => {
                    let input = app.governor_policy_input.trim().to_string();
                    if !input.is_empty() {
                        if let Some((name, policy)) = input.split_once(' ') {
                            app.command_input = format!(":governor set {} {}", name, policy);
                            app.show_command_palette = true;
                        } else {
                            app.add_event("Usage: <policy_name> <json>".to_string());
                        }
                    }
                    app.governor_mode = crate::cli::tui::app::GovernorMode::View;
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.governor_policy_input.push(c);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.governor_policy_input.pop();
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.governor_mode = crate::cli::tui::app::GovernorMode::View;
                    app.governor_policy_input.clear();
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            crate::cli::tui::app::GovernorMode::ConfirmDelete => match key {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let name = app.governor_policy_name.clone();
                    app.governor_mode = crate::cli::tui::app::GovernorMode::View;
                    if !name.is_empty() && app.connected() {
                        EventResult::Action(WorkspaceAction::ExecCommand(format!(
                            "governor_delete:{}",
                            name
                        )))
                    } else {
                        EventResult::Consumed
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.governor_mode = crate::cli::tui::app::GovernorMode::View;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "Set Policy".into(),
                WorkspaceAction::StatusMessage("Enter: <policy_name> <json>".into()),
            ),
            (
                "Delete Policy".into(),
                WorkspaceAction::StatusMessage("Delete default policy? (y/n)".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "s",
                description: "Set policy",
                group: "Governor",
            },
            KeyBinding {
                keys: "d",
                description: "Delete policy",
                group: "Governor",
            },
            KeyBinding {
                keys: "r",
                description: "Refresh",
                group: "Governor",
            },
        ]
    }
}

// ── BackupRestoreWorkspace ───────────────────────────────────────

pub struct BackupRestoreWorkspace;

impl Workspace for BackupRestoreWorkspace {
    fn section(&self) -> NavSection {
        NavSection::BackupRestore
    }

    fn title(&self) -> &str {
        "Backup & Restore"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::backups::render_backups(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        match key {
            KeyCode::Up => {
                app.selected_table_index = app.selected_table_index.saturating_sub(1);
                EventResult::Consumed
            }
            KeyCode::Down => {
                let max = app.backups_data.len().saturating_sub(1);
                app.selected_table_index = (app.selected_table_index + 1).min(max);
                EventResult::Consumed
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                let idx = app.selected_table_index;
                if let Some(detail) = app.backups_detail.as_ref() {
                    if let Some(backups) = detail.get("backups").and_then(|b| b.as_array()) {
                        if let Some(backup) = backups.get(idx) {
                            let id = backup
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if !id.is_empty() {
                                app.add_event(format!("Verifying backup {}...", id));
                                return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                    "backup_verify:{}",
                                    id
                                )));
                            }
                        }
                    }
                }
                EventResult::NotConsumed
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                let idx = app.selected_table_index;
                if let Some(detail) = app.backups_detail.as_ref() {
                    if let Some(backups) = detail.get("backups").and_then(|b| b.as_array()) {
                        if let Some(backup) = backups.get(idx) {
                            let id = backup
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if !id.is_empty() {
                                return EventResult::Action(WorkspaceAction::Confirm(
                                    format!("Delete backup '{}'?", id),
                                    format!("backup_delete:{}", id),
                                ));
                            }
                        }
                    }
                }
                EventResult::NotConsumed
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                EventResult::Action(WorkspaceAction::Refresh)
            }
            _ => EventResult::NotConsumed,
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "Verify Backup".into(),
                WorkspaceAction::StatusMessage("Select a backup and press 'v' to verify".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "Ctrl+B",
                description: "Create backup",
                group: "Backup",
            },
            KeyBinding {
                keys: "Ctrl+R",
                description: "Restore backup",
                group: "Backup",
            },
            KeyBinding {
                keys: "v",
                description: "Verify backup",
                group: "Backup",
            },
            KeyBinding {
                keys: "d",
                description: "Delete backup",
                group: "Backup",
            },
            KeyBinding {
                keys: "r",
                description: "Refresh",
                group: "Backup",
            },
        ]
    }
}

// ── MetricsLogsWorkspace ─────────────────────────────────────────

pub struct MetricsLogsWorkspace;

impl Workspace for MetricsLogsWorkspace {
    fn section(&self) -> NavSection {
        NavSection::MetricsLogs
    }

    fn title(&self) -> &str {
        "Metrics & Logs"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::metrics_logs::render_metrics_view(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        match key {
            KeyCode::Char('1') => {
                app.metrics_logs_mode = crate::cli::tui::app::MetricsLogsMode::Metrics;
                EventResult::Consumed
            }
            KeyCode::Char('2') => {
                app.metrics_logs_mode = crate::cli::tui::app::MetricsLogsMode::Logs;
                EventResult::Consumed
            }
            KeyCode::Char('3') => {
                app.metrics_logs_mode = crate::cli::tui::app::MetricsLogsMode::Both;
                EventResult::Consumed
            }
            KeyCode::Char('l') | KeyCode::Char('L') => match app.log_level_filter.as_str() {
                "" => {
                    app.log_level_filter = "error".to_string();
                    EventResult::Consumed
                }
                "error" => {
                    app.log_level_filter = "warn".to_string();
                    EventResult::Consumed
                }
                "warn" => {
                    app.log_level_filter = "info".to_string();
                    EventResult::Consumed
                }
                "info" => {
                    app.log_level_filter = "debug".to_string();
                    EventResult::Consumed
                }
                "debug" => {
                    app.log_level_filter = "trace".to_string();
                    EventResult::Consumed
                }
                _ => {
                    app.log_level_filter = String::new();
                    EventResult::Consumed
                }
            },
            KeyCode::Char('m') | KeyCode::Char('M') => match app.log_module_filter.as_str() {
                "" => {
                    app.log_module_filter = "primusdb_core".to_string();
                    EventResult::Consumed
                }
                "primusdb_core" => {
                    app.log_module_filter = "primusdb_tui".to_string();
                    EventResult::Consumed
                }
                "primusdb_tui" => {
                    app.log_module_filter = "primusdb_api".to_string();
                    EventResult::Consumed
                }
                "primusdb_api" => {
                    app.log_module_filter = "primusdb_storage".to_string();
                    EventResult::Consumed
                }
                _ => {
                    app.log_module_filter = String::new();
                    EventResult::Consumed
                }
            },
            _ => EventResult::NotConsumed,
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            (
                "Show Metrics".into(),
                WorkspaceAction::StatusMessage("Mode: Metrics".into()),
            ),
            (
                "Show Logs".into(),
                WorkspaceAction::StatusMessage("Mode: Logs".into()),
            ),
            (
                "Show Both".into(),
                WorkspaceAction::StatusMessage("Mode: Both".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "1",
                description: "Metrics only",
                group: "Metrics & Logs",
            },
            KeyBinding {
                keys: "2",
                description: "Logs only",
                group: "Metrics & Logs",
            },
            KeyBinding {
                keys: "3",
                description: "Both",
                group: "Metrics & Logs",
            },
            KeyBinding {
                keys: "l",
                description: "Cycle log level",
                group: "Metrics & Logs",
            },
            KeyBinding {
                keys: "m",
                description: "Cycle module filter",
                group: "Metrics & Logs",
            },
        ]
    }
}

// ── ConfigStudioWorkspace ────────────────────────────────────────

pub struct ConfigStudioWorkspace;

impl Workspace for ConfigStudioWorkspace {
    fn section(&self) -> NavSection {
        NavSection::ConfigurationStudio
    }

    fn title(&self) -> &str {
        "Config Studio"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::config_studio::render_config_studio(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        match app.config_mode {
            crate::cli::tui::app::ConfigStudioMode::List => match key {
                KeyCode::Up => {
                    app.config_selected_index = app.config_selected_index.saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    let max = app.config_entries.len().saturating_sub(1);
                    app.config_selected_index = app.config_selected_index.min(max);
                    if app.config_selected_index < max {
                        app.config_selected_index += 1;
                    }
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if let Some(entry) = app.config_entries.get(app.config_selected_index) {
                        app.config_detail_entry = Some(entry.clone());
                        app.config_mode = crate::cli::tui::app::ConfigStudioMode::Detail;
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    if let Some(entry) = app.config_entries.get(app.config_selected_index) {
                        app.config_detail_entry = Some(entry.clone());
                        app.config_mode = crate::cli::tui::app::ConfigStudioMode::Edit;
                        if let Some(v) = entry.get("value") {
                            app.config_input = serde_json::to_string_pretty(v).unwrap_or_default();
                        } else {
                            app.config_input = String::new();
                        }
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    app.config_mode = crate::cli::tui::app::ConfigStudioMode::NewEntry;
                    app.config_input = String::new();
                    app.config_error = None;
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if !app.config_entries.is_empty() {
                        app.config_mode = crate::cli::tui::app::ConfigStudioMode::ConfirmDelete;
                        app.config_error = None;
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    app.config_mode = crate::cli::tui::app::ConfigStudioMode::Snapshots;
                    EventResult::Action(WorkspaceAction::Refresh)
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    app.config_mode = crate::cli::tui::app::ConfigStudioMode::ImportExport;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            crate::cli::tui::app::ConfigStudioMode::Detail => match key {
                KeyCode::Esc => {
                    app.config_mode = crate::cli::tui::app::ConfigStudioMode::List;
                    app.config_detail_entry = None;
                    EventResult::Consumed
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    app.config_mode = crate::cli::tui::app::ConfigStudioMode::Edit;
                    if let Some(ref entry) = app.config_detail_entry {
                        if let Some(v) = entry.get("value") {
                            app.config_input = serde_json::to_string_pretty(v).unwrap_or_default();
                        }
                    }
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            crate::cli::tui::app::ConfigStudioMode::Edit => match key {
                KeyCode::Esc => {
                    app.config_mode = crate::cli::tui::app::ConfigStudioMode::Detail;
                    app.config_input.clear();
                    app.config_error = None;
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if let Some(ref e) = app.config_detail_entry.clone() {
                        if let Some(k) = e.get("key").and_then(|k| k.as_str()) {
                            let key = k.to_string();
                            let input = app.config_input.clone();
                            app.config_mode = crate::cli::tui::app::ConfigStudioMode::List;
                            app.config_detail_entry = None;
                            app.config_input.clear();
                            return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                "config_set:{}:{}",
                                key, input
                            )));
                        }
                    }
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.config_input.push(c);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.config_input.pop();
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            crate::cli::tui::app::ConfigStudioMode::NewEntry => match key {
                KeyCode::Esc => {
                    app.config_mode = crate::cli::tui::app::ConfigStudioMode::List;
                    app.config_input.clear();
                    app.config_error = None;
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if !app.config_input.is_empty() {
                        let input = app.config_input.clone();
                        app.config_mode = crate::cli::tui::app::ConfigStudioMode::List;
                        app.config_input.clear();
                        return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                            "config_create:{}",
                            input
                        )));
                    }
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.config_input.push(c);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.config_input.pop();
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            crate::cli::tui::app::ConfigStudioMode::ConfirmDelete => match key {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    let idx = app.config_selected_index;
                    if let Some(entry) = app.config_entries.get(idx) {
                        let key = entry
                            .get("key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        app.config_mode = crate::cli::tui::app::ConfigStudioMode::List;
                        app.config_error = None;
                        if !key.is_empty() {
                            return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                "config_delete:{}",
                                key
                            )));
                        }
                    } else {
                        app.config_mode = crate::cli::tui::app::ConfigStudioMode::List;
                        app.config_error = None;
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.config_mode = crate::cli::tui::app::ConfigStudioMode::List;
                    app.config_error = None;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            _ => EventResult::NotConsumed,
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "Create Entry".into(),
                WorkspaceAction::StatusMessage("Enter key=value for new config entry".into()),
            ),
            (
                "Snapshots".into(),
                WorkspaceAction::StatusMessage("Config snapshots".into()),
            ),
            (
                "Import/Export".into(),
                WorkspaceAction::StatusMessage("Import/export config".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "Enter",
                description: "Inspect entry",
                group: "Config",
            },
            KeyBinding {
                keys: "e",
                description: "Edit entry",
                group: "Config",
            },
            KeyBinding {
                keys: "n",
                description: "New entry",
                group: "Config",
            },
            KeyBinding {
                keys: "d",
                description: "Delete entry",
                group: "Config",
            },
            KeyBinding {
                keys: "s",
                description: "Snapshots",
                group: "Config",
            },
            KeyBinding {
                keys: "x",
                description: "Import/Export",
                group: "Config",
            },
        ]
    }
}

// ── TableExplorerWorkspace ───────────────────────────────────────

pub struct TableExplorerWorkspace;

impl Workspace for TableExplorerWorkspace {
    fn section(&self) -> NavSection {
        NavSection::TableExplorer
    }

    fn title(&self) -> &str {
        "Table Explorer"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::table_explorer::render_table_explorer(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        use crate::cli::tui::app::TableExplorerMode;
        match app.table_explorer_mode {
            TableExplorerMode::StorageTypeSelect => match key {
                KeyCode::Up => {
                    app.explorer_selected_st_index =
                        app.explorer_selected_st_index.saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    let max = app.explorer_storage_types.len().saturating_sub(1);
                    app.explorer_selected_st_index = app.explorer_selected_st_index.min(max);
                    if app.explorer_selected_st_index < max {
                        app.explorer_selected_st_index += 1;
                    }
                    EventResult::Consumed
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
                        app.table_explorer_mode = TableExplorerMode::TableList;
                    }
                    EventResult::Action(WorkspaceAction::Refresh)
                }
                _ => EventResult::NotConsumed,
            },
            TableExplorerMode::TableList => match key {
                KeyCode::Up => {
                    app.explorer_selected_table_index =
                        app.explorer_selected_table_index.saturating_sub(1);
                    EventResult::Consumed
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
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    app.table_explorer_mode = TableExplorerMode::TableDetail;
                    EventResult::Action(WorkspaceAction::Refresh)
                }
                KeyCode::Esc => {
                    app.table_explorer_mode = TableExplorerMode::StorageTypeSelect;
                    app.explorer_selected_st = None;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            TableExplorerMode::TableDetail => match key {
                KeyCode::Esc => {
                    app.table_explorer_mode = TableExplorerMode::TableList;
                    app.explorer_table_info = None;
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    app.explorer_row_offset = 0;
                    app.table_explorer_mode = TableExplorerMode::RowBrowser;
                    EventResult::Action(WorkspaceAction::Refresh)
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    let st = app.explorer_selected_st.clone().unwrap_or_default();
                    let t2 = app
                        .explorer_table_info
                        .as_ref()
                        .and_then(|v| v.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if !t2.is_empty() && app.connected() {
                        EventResult::Action(WorkspaceAction::ExecCommand(format!(
                            "analyze:{}:{}",
                            st, t2
                        )))
                    } else {
                        EventResult::Consumed
                    }
                }
                _ => EventResult::NotConsumed,
            },
            TableExplorerMode::RowBrowser => match key {
                KeyCode::Up => {
                    let rows = app.explorer_rows_data.as_ref().and_then(|v| v.as_array());
                    let count = rows.map(|a| a.len()).unwrap_or(0);
                    if count > 0 {
                        app.explorer_selected_row_index =
                            app.explorer_selected_row_index.saturating_sub(1);
                    }
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    let rows = app.explorer_rows_data.as_ref().and_then(|v| v.as_array());
                    let count = rows.map(|a| a.len()).unwrap_or(0);
                    if count > 0 {
                        app.explorer_selected_row_index =
                            (app.explorer_selected_row_index + 1).min(count.saturating_sub(1));
                    }
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.table_explorer_mode = TableExplorerMode::TableDetail;
                    app.explorer_rows_data = None;
                    EventResult::Consumed
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    app.table_explorer_mode = TableExplorerMode::RowInsert;
                    app.explorer_insert_input.clear();
                    app.explorer_status.clear();
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    let rows = app.explorer_rows_data.as_ref().and_then(|v| v.as_array());
                    if let Some(r) = rows.and_then(|a| a.get(app.explorer_selected_row_index)) {
                        let first_key = r
                            .as_object()
                            .and_then(|o| o.keys().next().cloned())
                            .unwrap_or_default();
                        let first_val = r.get(&first_key).and_then(|v| v.as_str()).unwrap_or("");
                        if !first_val.is_empty() {
                            app.explorer_status =
                                format!("Delete row where {} = '{}'? (y/n)", first_key, first_val);
                            app.table_explorer_mode = TableExplorerMode::ConfirmDelete;
                        }
                    }
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            TableExplorerMode::RowInsert => match key {
                KeyCode::Enter => {
                    if !app.explorer_insert_input.is_empty() && app.connected() {
                        let st = app.explorer_selected_st.clone().unwrap_or_default();
                        let table = app
                            .explorer_table_info
                            .as_ref()
                            .and_then(|v| v.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let json = app.explorer_insert_input.clone();
                        app.table_explorer_mode = TableExplorerMode::RowBrowser;
                        app.explorer_insert_input.clear();
                        if !table.is_empty() {
                            return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                "insert_row:{}:{}:{}",
                                st, table, json
                            )));
                        }
                    }
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.table_explorer_mode = TableExplorerMode::RowBrowser;
                    app.explorer_insert_input.clear();
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.explorer_insert_input.push(c);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.explorer_insert_input.pop();
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            TableExplorerMode::ConfirmDelete => match key {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if app.connected() {
                        let rows = app.explorer_rows_data.as_ref().and_then(|v| v.as_array());
                        if let Some(r) = rows.and_then(|a| a.get(app.explorer_selected_row_index)) {
                            let first_key = r
                                .as_object()
                                .and_then(|o| o.keys().next().cloned())
                                .unwrap_or_default();
                            let first_val =
                                r.get(&first_key).and_then(|v| v.as_str()).unwrap_or("");
                            let st = app.explorer_selected_st.clone().unwrap_or_default();
                            let table = app
                                .explorer_table_info
                                .as_ref()
                                .and_then(|v| v.get("name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            app.table_explorer_mode = TableExplorerMode::RowBrowser;
                            app.explorer_status.clear();
                            if !first_val.is_empty() && !table.is_empty() {
                                return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                    "delete_row:{}:{}:{}:{}",
                                    st, table, first_key, first_val
                                )));
                            }
                        } else {
                            app.table_explorer_mode = TableExplorerMode::RowBrowser;
                            app.explorer_status.clear();
                        }
                    } else {
                        app.table_explorer_mode = TableExplorerMode::RowBrowser;
                        app.explorer_status.clear();
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.table_explorer_mode = TableExplorerMode::RowBrowser;
                    app.explorer_status.clear();
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            _ => EventResult::NotConsumed,
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "Insert Row".into(),
                WorkspaceAction::StatusMessage("Enter JSON for new row".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "Enter",
                description: "Open/Drill down",
                group: "Explorer",
            },
            KeyBinding {
                keys: "i",
                description: "Insert row",
                group: "Explorer",
            },
            KeyBinding {
                keys: "d",
                description: "Delete row",
                group: "Explorer",
            },
            KeyBinding {
                keys: "a",
                description: "Analyze table",
                group: "Explorer",
            },
            KeyBinding {
                keys: "Esc",
                description: "Go back",
                group: "Explorer",
            },
        ]
    }
}

// ── ReportBuilderWorkspace ───────────────────────────────────────

pub struct ReportBuilderWorkspace;

impl Workspace for ReportBuilderWorkspace {
    fn section(&self) -> NavSection {
        NavSection::ReportBuilder
    }

    fn title(&self) -> &str {
        "Report Builder"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::report_builder::render_report_builder(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        use crate::cli::tui::app::ReportBuilderMode;
        match app.report_mode {
            ReportBuilderMode::List => match key {
                KeyCode::Up => {
                    app.report_selected_index = app.report_selected_index.saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    let max = app.reports_data.len().saturating_sub(1);
                    app.report_selected_index = app.report_selected_index.min(max);
                    if app.report_selected_index < max {
                        app.report_selected_index += 1;
                    }
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if let Some(report) = app.reports_data.get(app.report_selected_index) {
                        app.report_detail = Some(report.clone());
                        app.report_mode = ReportBuilderMode::Detail;
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    app.report_mode = ReportBuilderMode::Create;
                    app.report_input = String::new();
                    app.report_input_field = 0;
                    app.report_error = None;
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if !app.reports_data.is_empty() {
                        app.report_mode = ReportBuilderMode::ConfirmDelete;
                    }
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            ReportBuilderMode::Detail => match key {
                KeyCode::Esc => {
                    app.report_mode = ReportBuilderMode::List;
                    app.report_detail = None;
                    app.report_results = None;
                    EventResult::Consumed
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    if let Some(ref report) = app.report_detail.clone() {
                        let name = report.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let query = report.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        let desc = report
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let st = report
                            .get("storage_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("relational");
                        let fmt = report
                            .get("format")
                            .and_then(|v| v.as_str())
                            .unwrap_or("json");
                        let tbl = report
                            .get("table_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let id = report
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        app.report_input =
                            format!("{}|{}|{}|{}|{}|{}", name, query, desc, st, fmt, tbl);
                        app.report_edit_id = Some(id);
                        app.report_mode = ReportBuilderMode::Edit;
                        app.report_error = None;
                    }
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if let Some(ref report) = app.report_detail {
                        if let Some(id) = report.get("id").and_then(|v| v.as_str()) {
                            let id2 = id.to_string();
                            app.report_mode = ReportBuilderMode::Results;
                            return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                "report_execute:{}",
                                id2
                            )));
                        }
                    }
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            ReportBuilderMode::Create | ReportBuilderMode::Edit => match key {
                KeyCode::Esc => {
                    app.report_mode = ReportBuilderMode::List;
                    app.report_input.clear();
                    app.report_error = None;
                    app.report_edit_id = None;
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if !app.report_input.is_empty() {
                        let input = app.report_input.clone();
                        let edit_id = app.report_edit_id.clone();
                        app.report_mode = ReportBuilderMode::List;
                        app.report_input.clear();
                        app.report_edit_id = None;
                        if let Some(id) = edit_id {
                            return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                "report_update:{}:{}",
                                id, input
                            )));
                        } else {
                            return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                "report_create:{}",
                                input
                            )));
                        }
                    }
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.report_input.push(c);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.report_input.pop();
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            ReportBuilderMode::ConfirmDelete => match key {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    let idx = app.report_selected_index;
                    if let Some(report) = app.reports_data.get(idx) {
                        let id = report
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        app.report_mode = ReportBuilderMode::List;
                        if !id.is_empty() {
                            return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                "report_delete:{}",
                                id
                            )));
                        }
                    } else {
                        app.report_mode = ReportBuilderMode::List;
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.report_mode = ReportBuilderMode::List;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            ReportBuilderMode::Results => {
                if key == KeyCode::Esc {
                    app.report_mode = ReportBuilderMode::Detail;
                    app.report_results = None;
                    EventResult::Consumed
                } else {
                    EventResult::NotConsumed
                }
            }
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "Create Report".into(),
                WorkspaceAction::StatusMessage("Format: name|query|desc|type|format|table".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "Enter",
                description: "View/Execute report",
                group: "Reports",
            },
            KeyBinding {
                keys: "n",
                description: "New report",
                group: "Reports",
            },
            KeyBinding {
                keys: "e",
                description: "Edit report",
                group: "Reports",
            },
            KeyBinding {
                keys: "d",
                description: "Delete report",
                group: "Reports",
            },
        ]
    }
}

// ── NotebookWorkspace ────────────────────────────────────────────

pub struct NotebookWorkspace;

impl Workspace for NotebookWorkspace {
    fn section(&self) -> NavSection {
        NavSection::Notebook
    }

    fn title(&self) -> &str {
        "Notebook"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::notebook::render_notebook(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        use crate::cli::tui::app::NotebookBuilderMode;
        match app.notebook_mode {
            NotebookBuilderMode::List => match key {
                KeyCode::Up => {
                    app.notebook_selected_index = app.notebook_selected_index.saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    let max = app.notebooks_data.len().saturating_sub(1);
                    app.notebook_selected_index = app.notebook_selected_index.min(max);
                    if app.notebook_selected_index < max {
                        app.notebook_selected_index += 1;
                    }
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if let Some(nb) = app.notebooks_data.get(app.notebook_selected_index) {
                        let id = nb
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !id.is_empty() {
                            app.notebook_mode = NotebookBuilderMode::Detail;
                            return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                "notebook_fetch:{}",
                                id
                            )));
                        }
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    app.notebook_mode = NotebookBuilderMode::CellEdit;
                    app.notebook_cell_edit = String::new();
                    app.notebook_error = None;
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if !app.notebooks_data.is_empty() {
                        app.notebook_mode = NotebookBuilderMode::ConfirmDelete;
                    }
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            NotebookBuilderMode::Detail => match key {
                KeyCode::Up => {
                    app.notebook_selected_cell = app.notebook_selected_cell.saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    let max = app.notebook_cells.len().saturating_sub(1);
                    app.notebook_selected_cell = app.notebook_selected_cell.min(max);
                    if app.notebook_selected_cell < max {
                        app.notebook_selected_cell += 1;
                    }
                    EventResult::Consumed
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
                    EventResult::Consumed
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    let id = app
                        .notebook_detail
                        .as_ref()
                        .and_then(|v| v.get("id"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let idx = app.notebook_selected_cell;
                    app.notebook_mode = NotebookBuilderMode::Results;
                    if !id.is_empty() {
                        return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                            "notebook_execute:{}:{}",
                            id, idx
                        )));
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    app.notebook_mode = NotebookBuilderMode::CellTypeSelect;
                    EventResult::Consumed
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
                        let cells = app.notebook_cells.clone();
                        let cells_json = serde_json::to_string(&cells).unwrap_or_default();
                        return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                            "notebook_save:{}:{}",
                            id, cells_json
                        )));
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    app.notebook_cells.push(serde_json::json!({
                        "type": "md",
                        "content": "",
                        "result": null,
                    }));
                    app.notebook_selected_cell = app.notebook_cells.len().saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    if !app.notebook_cells.is_empty() {
                        app.notebook_cells.remove(app.notebook_selected_cell);
                        if app.notebook_selected_cell >= app.notebook_cells.len() {
                            app.notebook_selected_cell = app.notebook_cells.len().saturating_sub(1);
                        }
                    }
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.notebook_mode = NotebookBuilderMode::List;
                    app.notebook_detail = None;
                    app.notebook_cells.clear();
                    app.notebook_cell_result = None;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
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
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.notebook_cell_edit.push(c);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.notebook_cell_edit.pop();
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.notebook_cell_edit.clear();
                    app.notebook_mode = NotebookBuilderMode::Detail;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
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
                    EventResult::Consumed
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
                    EventResult::Consumed
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
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.notebook_mode = NotebookBuilderMode::Detail;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            NotebookBuilderMode::ConfirmDelete => match key {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    let idx = app.notebook_selected_index;
                    if let Some(nb) = app.notebooks_data.get(idx) {
                        let id = nb
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        app.notebook_mode = NotebookBuilderMode::List;
                        if !id.is_empty() {
                            return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                "notebook_delete:{}",
                                id
                            )));
                        }
                    } else {
                        app.notebook_mode = NotebookBuilderMode::List;
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.notebook_mode = NotebookBuilderMode::List;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            NotebookBuilderMode::Results => {
                if key == KeyCode::Esc {
                    app.notebook_mode = NotebookBuilderMode::Detail;
                    app.notebook_cell_result = None;
                    EventResult::Consumed
                } else {
                    EventResult::NotConsumed
                }
            }
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "New Cell".into(),
                WorkspaceAction::StatusMessage("Add a new cell to the notebook".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "Enter",
                description: "Open/Edit cell",
                group: "Notebook",
            },
            KeyBinding {
                keys: "n",
                description: "New cell",
                group: "Notebook",
            },
            KeyBinding {
                keys: "e",
                description: "Execute cell",
                group: "Notebook",
            },
            KeyBinding {
                keys: "s",
                description: "Save notebook",
                group: "Notebook",
            },
            KeyBinding {
                keys: "t",
                description: "Change cell type",
                group: "Notebook",
            },
            KeyBinding {
                keys: "x",
                description: "Delete cell",
                group: "Notebook",
            },
        ]
    }
}

// ── RAGWorkspaceImpl ─────────────────────────────────────────────

pub struct RAGWorkspaceImpl;

impl Workspace for RAGWorkspaceImpl {
    fn section(&self) -> NavSection {
        NavSection::RAGWorkspace
    }

    fn title(&self) -> &str {
        "RAG Workspace"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::rag_workspace::render_rag_workspace(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        use crate::cli::tui::app::RagWorkspaceMode;
        match app.rag_mode {
            RagWorkspaceMode::CollectionSelect => match key {
                KeyCode::Up => {
                    app.rag_selected_index = app.rag_selected_index.saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    let max = app.rag_collections.len().saturating_sub(1);
                    app.rag_selected_index = app.rag_selected_index.min(max);
                    if app.rag_selected_index < max {
                        app.rag_selected_index += 1;
                    }
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if !app.rag_collections.is_empty() {
                        app.rag_query_text.clear();
                        app.rag_results = None;
                        app.rag_error = None;
                        app.rag_mode = RagWorkspaceMode::SearchConfig;
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    app.rag_mode = RagWorkspaceMode::CreateCollection;
                    app.rag_input.clear();
                    app.rag_error = None;
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if !app.rag_collections.is_empty() {
                        app.rag_mode = RagWorkspaceMode::ConfirmDelete;
                    }
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            RagWorkspaceMode::CreateCollection => match key {
                KeyCode::Esc => {
                    app.rag_mode = RagWorkspaceMode::CollectionSelect;
                    app.rag_input.clear();
                    app.rag_error = None;
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if !app.rag_input.is_empty() && app.connected() {
                        let name = app.rag_input.clone();
                        app.rag_mode = RagWorkspaceMode::CollectionSelect;
                        app.rag_input.clear();
                        return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                            "rag_create:{}",
                            name
                        )));
                    }
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.rag_input.pop();
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.rag_input.push(c);
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            RagWorkspaceMode::ConfirmDelete => match key {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    if let Some(collection) = app.rag_collections.get(app.rag_selected_index) {
                        let name = collection.clone();
                        app.confirm_action = crate::cli::tui::app::ConfirmAction::DropTable;
                        app.confirm_message = format!("Delete RAG collection '{}'?", name);
                        app.pending_action = Some(format!("rag_delete:{}", name));
                    }
                    app.rag_mode = RagWorkspaceMode::CollectionSelect;
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.rag_mode = RagWorkspaceMode::CollectionSelect;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            RagWorkspaceMode::SearchConfig => match key {
                KeyCode::Enter => {
                    let collection = app
                        .rag_collections
                        .get(app.rag_selected_index)
                        .cloned()
                        .unwrap_or_default();
                    if !collection.is_empty() && !app.rag_query_text.is_empty() {
                        let query = app.rag_query_text.clone();
                        let limit = app.rag_limit;
                        app.rag_mode = RagWorkspaceMode::SearchResults;
                        return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                            "rag_search:{}:{}:{}",
                            collection, query, limit
                        )));
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    app.rag_limit = app.rag_limit.saturating_add(5).min(100);
                    EventResult::Consumed
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    app.rag_limit = app.rag_limit.saturating_sub(5).max(1);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.rag_query_text.pop();
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.rag_query_text.push(c);
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.rag_mode = RagWorkspaceMode::CollectionSelect;
                    app.rag_query_text.clear();
                    app.rag_results = None;
                    app.rag_error = None;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            RagWorkspaceMode::SearchResults => {
                if key == KeyCode::Esc {
                    app.rag_mode = RagWorkspaceMode::SearchConfig;
                    app.rag_results = None;
                    EventResult::Consumed
                } else {
                    EventResult::NotConsumed
                }
            }
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "New Collection".into(),
                WorkspaceAction::StatusMessage("Enter collection name".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "Enter",
                description: "Search collection",
                group: "RAG",
            },
            KeyBinding {
                keys: "n",
                description: "New collection",
                group: "RAG",
            },
            KeyBinding {
                keys: "d",
                description: "Delete collection",
                group: "RAG",
            },
            KeyBinding {
                keys: "+/-",
                description: "Adjust limit",
                group: "RAG",
            },
        ]
    }
}

// ── SecurityCenterWorkspace ──────────────────────────────────────

pub struct SecurityCenterWorkspace;

impl Workspace for SecurityCenterWorkspace {
    fn section(&self) -> NavSection {
        NavSection::SecurityCenter
    }

    fn title(&self) -> &str {
        "Security Center"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::security_center::render_security_center(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        use crate::cli::tui::app::SecurityCenterMode;
        match app.sec_mode {
            SecurityCenterMode::Users
            | SecurityCenterMode::Roles
            | SecurityCenterMode::Permissions => match key {
                KeyCode::Up => {
                    app.sec_selected_index = app.sec_selected_index.saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    let max = match app.sec_mode {
                        SecurityCenterMode::Users => app.sec_users.len().saturating_sub(1),
                        SecurityCenterMode::Roles => app.sec_roles.len().saturating_sub(1),
                        _ => app.sec_permissions.len().saturating_sub(1),
                    };
                    app.sec_selected_index = (app.sec_selected_index + 1).min(max);
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    app.sec_mode = match app.sec_mode {
                        SecurityCenterMode::Users => SecurityCenterMode::UserDetail,
                        SecurityCenterMode::Roles => SecurityCenterMode::RoleDetail,
                        _ => app.sec_mode,
                    };
                    EventResult::Consumed
                }
                KeyCode::Char('u') | KeyCode::Char('U') => {
                    app.sec_mode = SecurityCenterMode::Users;
                    app.sec_selected_index = 0;
                    EventResult::Consumed
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    app.sec_mode = SecurityCenterMode::Roles;
                    app.sec_selected_index = 0;
                    EventResult::Consumed
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    app.sec_mode = SecurityCenterMode::Permissions;
                    app.sec_selected_index = 0;
                    EventResult::Consumed
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    app.sec_mode = match app.sec_mode {
                        SecurityCenterMode::Users => SecurityCenterMode::CreateUser,
                        SecurityCenterMode::Roles => SecurityCenterMode::CreateRole,
                        _ => app.sec_mode,
                    };
                    app.sec_input.clear();
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            SecurityCenterMode::UserDetail => match key {
                KeyCode::Esc => {
                    app.sec_mode = SecurityCenterMode::Users;
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if app.connected() {
                        let idx = app.sec_selected_index;
                        if let Some(user) = app.sec_users.get(idx) {
                            let name = user
                                .get("username")
                                .and_then(|u| u.as_str())
                                .unwrap_or("")
                                .to_string();
                            app.sec_mode = SecurityCenterMode::Users;
                            if !name.is_empty() {
                                return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                    "user_delete:{}",
                                    name
                                )));
                            }
                        } else {
                            app.sec_mode = SecurityCenterMode::Users;
                        }
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    app.sec_role_checklist = app
                        .sec_roles
                        .iter()
                        .map(|r| {
                            let name = r
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("?")
                                .to_string();
                            if let Some(user) = app.sec_users.get(app.sec_selected_index) {
                                let user_roles: Vec<String> = user
                                    .get("roles")
                                    .and_then(|r| r.as_array())
                                    .map(|a| {
                                        a.iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                (name.clone(), user_roles.contains(&name))
                            } else {
                                (name, false)
                            }
                        })
                        .collect();
                    app.sec_mode = SecurityCenterMode::AssignRole;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            SecurityCenterMode::RoleDetail => match key {
                KeyCode::Esc => {
                    app.sec_mode = SecurityCenterMode::Roles;
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if app.connected() {
                        let idx = app.sec_selected_index;
                        if let Some(role) = app.sec_roles.get(idx) {
                            let name = role
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("")
                                .to_string();
                            app.sec_mode = SecurityCenterMode::Roles;
                            if !name.is_empty() {
                                return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                    "role_delete:{}",
                                    name
                                )));
                            }
                        } else {
                            app.sec_mode = SecurityCenterMode::Roles;
                        }
                    }
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            SecurityCenterMode::CreateUser | SecurityCenterMode::CreateRole => match key {
                KeyCode::Char(c) => {
                    app.sec_input.push(c);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.sec_input.pop();
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if !app.sec_input.is_empty() && app.connected() {
                        let input_str = app.sec_input.clone();
                        let mode = app.sec_mode;
                        app.sec_input.clear();
                        app.sec_mode = SecurityCenterMode::Users;
                        let cmd = match mode {
                            SecurityCenterMode::CreateUser => {
                                format!("user_create:{}", input_str)
                            }
                            SecurityCenterMode::CreateRole => {
                                format!("role_create:{}", input_str)
                            }
                            _ => return EventResult::Consumed,
                        };
                        return EventResult::Action(WorkspaceAction::ExecCommand(cmd));
                    }
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.sec_input.clear();
                    app.sec_mode = SecurityCenterMode::Users;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            SecurityCenterMode::AssignRole => match key {
                KeyCode::Esc => {
                    app.sec_mode = SecurityCenterMode::UserDetail;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "Create User".into(),
                WorkspaceAction::StatusMessage("Enter: <username> <password>".into()),
            ),
            (
                "Create Role".into(),
                WorkspaceAction::StatusMessage("Enter role name".into()),
            ),
            (
                "View Users".into(),
                WorkspaceAction::StatusMessage("Tab: Users".into()),
            ),
            (
                "View Roles".into(),
                WorkspaceAction::StatusMessage("Tab: Roles".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "u",
                description: "Users tab",
                group: "Security",
            },
            KeyBinding {
                keys: "r",
                description: "Roles tab",
                group: "Security",
            },
            KeyBinding {
                keys: "p",
                description: "Permissions tab",
                group: "Security",
            },
            KeyBinding {
                keys: "n",
                description: "New user/role",
                group: "Security",
            },
            KeyBinding {
                keys: "d",
                description: "Delete user/role",
                group: "Security",
            },
            KeyBinding {
                keys: "a",
                description: "Assign role",
                group: "Security",
            },
            KeyBinding {
                keys: "Enter",
                description: "View detail",
                group: "Security",
            },
        ]
    }
}

// ── DocumentWorkspaceImpl ────────────────────────────────────────

pub struct DocumentWorkspaceImpl;

impl Workspace for DocumentWorkspaceImpl {
    fn section(&self) -> NavSection {
        NavSection::DocumentWorkspace
    }

    fn title(&self) -> &str {
        "Document Editor"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::document_workspace::render_document_workspace(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        use crate::cli::tui::app::DocEditorMode;
        match app.doc_mode {
            DocEditorMode::View => match key {
                KeyCode::Up => {
                    if !app.doc_collections.is_empty() {
                        app.doc_collection_selected = app.doc_collection_selected.saturating_sub(1);
                    }
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    if !app.doc_collections.is_empty() {
                        app.doc_collection_selected = (app.doc_collection_selected + 1)
                            .min(app.doc_collections.len().saturating_sub(1));
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    if !app.doc_current_json.is_empty() {
                        app.doc_edit_buffer = app.doc_current_json.clone();
                        app.doc_mode = DocEditorMode::Edit;
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    app.doc_mode = DocEditorMode::Create;
                    app.doc_edit_buffer.clear();
                    app.doc_validation_error = None;
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if !app.doc_collections.is_empty()
                        && app.doc_collection_selected < app.doc_collections.len()
                    {
                        let col = &app.doc_collections[app.doc_collection_selected];
                        let key = if !app.doc_documents.is_empty()
                            && app.doc_selected_index < app.doc_documents.len()
                        {
                            app.doc_documents[app.doc_selected_index].clone()
                        } else {
                            String::new()
                        };
                        if !key.is_empty() {
                            return EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                ":doc delete {}/{}",
                                col, key
                            )));
                        }
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('v') | KeyCode::Char('V') => {
                    if !app.doc_current_json.is_empty() {
                        app.doc_validation_error =
                            serde_json::from_str::<serde_json::Value>(&app.doc_current_json)
                                .err()
                                .map(|e| e.to_string());
                        if app.doc_validation_error.is_none() {
                            app.doc_status = "JSON is valid".to_string();
                        }
                    }
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            DocEditorMode::Edit | DocEditorMode::Create => match key {
                KeyCode::Enter => {
                    if !app.doc_edit_buffer.is_empty() {
                        match serde_json::from_str::<serde_json::Value>(&app.doc_edit_buffer) {
                            Ok(_) => {
                                app.doc_current_json = app.doc_edit_buffer.clone();
                                app.doc_status = "Document saved".to_string();
                                app.doc_validation_error = None;
                                app.doc_mode = DocEditorMode::View;
                            }
                            Err(e) => {
                                app.doc_validation_error = Some(e.to_string());
                            }
                        }
                    }
                    EventResult::Consumed
                }
                KeyCode::Char('v') | KeyCode::Char('V') => {
                    if !app.doc_edit_buffer.is_empty() {
                        app.doc_validation_error =
                            serde_json::from_str::<serde_json::Value>(&app.doc_edit_buffer)
                                .err()
                                .map(|e| e.to_string());
                        if app.doc_validation_error.is_none() {
                            app.doc_status = "JSON is valid".to_string();
                        }
                    }
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.doc_edit_buffer.push(c);
                    app.doc_validation_error = None;
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.doc_edit_buffer.pop();
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.doc_edit_buffer = app.doc_current_json.clone();
                    app.doc_validation_error = None;
                    app.doc_mode = DocEditorMode::View;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "Create Document".into(),
                WorkspaceAction::StatusMessage("Enter JSON document".into()),
            ),
            (
                "Validate JSON".into(),
                WorkspaceAction::StatusMessage("Press 'v' to validate".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "e",
                description: "Edit document",
                group: "Documents",
            },
            KeyBinding {
                keys: "c",
                description: "Create document",
                group: "Documents",
            },
            KeyBinding {
                keys: "d",
                description: "Delete document",
                group: "Documents",
            },
            KeyBinding {
                keys: "v",
                description: "Validate JSON",
                group: "Documents",
            },
            KeyBinding {
                keys: "Enter",
                description: "Save document",
                group: "Documents",
            },
        ]
    }
}

// ── IntegratedTerminalWorkspace ──────────────────────────────────

pub struct IntegratedTerminalWorkspace;

impl Workspace for IntegratedTerminalWorkspace {
    fn section(&self) -> NavSection {
        NavSection::IntegratedTerminal
    }

    fn title(&self) -> &str {
        "Terminal"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::integrated_terminal::render_integrated_terminal(
            frame, area, app,
        );
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        match key {
            KeyCode::Char(c) => {
                app.terminal_input.push(c);
                EventResult::Consumed
            }
            KeyCode::Backspace => {
                app.terminal_input.pop();
                EventResult::Consumed
            }
            KeyCode::Enter => {
                let cmd = app.terminal_input.trim().to_string();
                if !cmd.is_empty() {
                    app.terminal_output.push(format!("$ {}", cmd));
                    app.terminal_history.push(cmd.clone());
                    app.terminal_history_pos = app.terminal_history.len();

                    let output = if cmd == "clear" || cmd == "cls" {
                        app.terminal_output.clear();
                        String::new()
                    } else if cmd.starts_with("cd ") {
                        let dir = cmd.trim_start_matches("cd ").trim();
                        app.terminal_cwd = dir.to_string();
                        format!("Changed directory to: {}", dir)
                    } else if cmd == "pwd" {
                        app.terminal_cwd.to_string()
                    } else if cmd == "help" || cmd == "?" {
                        "Available commands:\n  clear/cls  - Clear terminal\n  help/?     - Show this help\n  pwd        - Show working directory\n  cd <dir>   - Change directory\n  echo       - Print text".to_string()
                    } else if cmd.starts_with("echo ") {
                        cmd.trim_start_matches("echo ").to_string()
                    } else {
                        let cwd = if app.terminal_cwd.is_empty() {
                            ".".to_string()
                        } else {
                            app.terminal_cwd.clone()
                        };
                        match std::process::Command::new("sh")
                            .args(["-c", &cmd])
                            .current_dir(&cwd)
                            .output()
                        {
                            Ok(out) => {
                                let stdout =
                                    String::from_utf8_lossy(&out.stdout).trim().to_string();
                                let stderr =
                                    String::from_utf8_lossy(&out.stderr).trim().to_string();
                                if !stdout.is_empty() && !stderr.is_empty() {
                                    format!("{}\n{}", stdout, stderr)
                                } else if !stdout.is_empty() {
                                    stdout
                                } else if !stderr.is_empty() {
                                    format!("Error: {}", stderr)
                                } else {
                                    format!(
                                        "Command completed (exit code: {})",
                                        out.status.code().unwrap_or(-1)
                                    )
                                }
                            }
                            Err(e) => format!("Error: {}", e),
                        }
                    };

                    if !output.is_empty() {
                        app.terminal_output.push(output);
                    }
                }
                app.terminal_input.clear();
                EventResult::Consumed
            }
            KeyCode::Up => {
                if !app.terminal_history.is_empty() && app.terminal_history_pos > 0 {
                    app.terminal_history_pos -= 1;
                    app.terminal_input = app.terminal_history[app.terminal_history_pos].clone();
                }
                EventResult::Consumed
            }
            KeyCode::Down => {
                if app.terminal_history_pos < app.terminal_history.len() {
                    app.terminal_history_pos += 1;
                    if app.terminal_history_pos < app.terminal_history.len() {
                        app.terminal_input = app.terminal_history[app.terminal_history_pos].clone();
                    } else {
                        app.terminal_input.clear();
                    }
                }
                EventResult::Consumed
            }
            _ => EventResult::NotConsumed,
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![(
            "Clear Terminal".into(),
            WorkspaceAction::ExecCommand("terminal_clear".into()),
        )]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "Enter",
                description: "Execute command",
                group: "Terminal",
            },
            KeyBinding {
                keys: "↑↓",
                description: "Command history",
                group: "Terminal",
            },
            KeyBinding {
                keys: "Tab",
                description: "Autocomplete",
                group: "Terminal",
            },
        ]
    }
}

// ── MonitoringWorkspace ──────────────────────────────────────────

pub struct MonitoringWorkspace;

impl Workspace for MonitoringWorkspace {
    fn section(&self) -> NavSection {
        NavSection::Monitoring
    }

    fn title(&self) -> &str {
        "Monitoring"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::monitoring::render_monitoring(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        match key {
            KeyCode::Char('o') | KeyCode::Char('O') => {
                app.mon_mode = crate::cli::tui::app::MonitoringMode::Overview;
                EventResult::Consumed
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                app.mon_mode = crate::cli::tui::app::MonitoringMode::Alerts;
                EventResult::Consumed
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                app.mon_mode = crate::cli::tui::app::MonitoringMode::Performance;
                EventResult::Consumed
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                app.mon_mode = crate::cli::tui::app::MonitoringMode::Replication;
                EventResult::Consumed
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                app.mon_mode = crate::cli::tui::app::MonitoringMode::Resources;
                EventResult::Consumed
            }
            _ => EventResult::NotConsumed,
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            (
                "Overview".into(),
                WorkspaceAction::StatusMessage("Tab: Overview".into()),
            ),
            (
                "Alerts".into(),
                WorkspaceAction::StatusMessage("Tab: Alerts".into()),
            ),
            (
                "Performance".into(),
                WorkspaceAction::StatusMessage("Tab: Performance".into()),
            ),
            (
                "Replication".into(),
                WorkspaceAction::StatusMessage("Tab: Replication".into()),
            ),
            (
                "Resources".into(),
                WorkspaceAction::StatusMessage("Tab: Resources".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "o",
                description: "Overview tab",
                group: "Monitoring",
            },
            KeyBinding {
                keys: "a",
                description: "Alerts tab",
                group: "Monitoring",
            },
            KeyBinding {
                keys: "p",
                description: "Performance tab",
                group: "Monitoring",
            },
            KeyBinding {
                keys: "r",
                description: "Replication tab",
                group: "Monitoring",
            },
            KeyBinding {
                keys: "s",
                description: "Resources tab",
                group: "Monitoring",
            },
        ]
    }
}

// ── SettingsWorkspace ────────────────────────────────────────────

pub struct SettingsWorkspace;

impl Workspace for SettingsWorkspace {
    fn section(&self) -> NavSection {
        NavSection::Settings
    }

    fn title(&self) -> &str {
        "Settings"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::settings::render_settings(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        use crate::cli::tui::app::SettingsMode;
        match app.settings_mode {
            SettingsMode::View => match key {
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    app.settings_mode = SettingsMode::EditRefreshInterval;
                    app.settings_input.clear();
                    EventResult::Consumed
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    app.mouse_enabled = !app.mouse_enabled;
                    app.config.mouse_enabled = app.mouse_enabled;
                    app.settings_mode = SettingsMode::ToggleMouse;
                    EventResult::Consumed
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    app.settings_mode = SettingsMode::EditEndpoint;
                    app.settings_input = app.connected_url.clone().unwrap_or_default();
                    EventResult::Consumed
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    app.settings_mode = SettingsMode::EditToken;
                    app.settings_input = app.auth_token.clone().unwrap_or_default();
                    EventResult::Consumed
                }
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    let themes = ["default", "dark", "light", "high-contrast"];
                    let current = app.config.theme.as_str();
                    let next = themes
                        .iter()
                        .position(|t| *t == current)
                        .map(|i| themes[(i + 1) % themes.len()])
                        .unwrap_or(themes[0]);
                    app.config.theme = next.to_string();
                    app.settings_mode = SettingsMode::EditTheme;
                    app.add_event(format!("Theme set to: {}", app.config.theme));
                    EventResult::Consumed
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    app.config.confirm_destructive_actions =
                        !app.config.confirm_destructive_actions;
                    app.settings_mode = SettingsMode::EditSafeMode;
                    app.add_event(format!(
                        "Safe mode: {}",
                        if app.config.confirm_destructive_actions {
                            "on"
                        } else {
                            "off"
                        }
                    ));
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    app.settings_mode = SettingsMode::Doctor;
                    app.doctor_results.clear();
                    app.add_event("Running diagnostics...".to_string());
                    EventResult::Action(WorkspaceAction::ExecCommand("doctor".into()))
                }
                _ => EventResult::NotConsumed,
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
                    EventResult::Consumed
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    app.settings_input.push(c);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.settings_input.pop();
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.settings_mode = SettingsMode::View;
                    app.settings_input.clear();
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            SettingsMode::EditEndpoint => match key {
                KeyCode::Enter => {
                    let val = app.settings_input.trim().to_string();
                    if !val.is_empty() {
                        app.connected_url = Some(val.clone());
                        app.config.endpoint = val;
                        app.add_event("Endpoint updated.".to_string());
                    }
                    app.settings_mode = SettingsMode::View;
                    app.settings_input.clear();
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.settings_input.push(c);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.settings_input.pop();
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.settings_mode = SettingsMode::View;
                    app.settings_input.clear();
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            SettingsMode::EditToken => match key {
                KeyCode::Enter => {
                    let val = app.settings_input.trim().to_string();
                    if !val.is_empty() {
                        app.auth_token = Some(val.clone());
                        app.add_event("Auth token updated.".to_string());
                    } else {
                        app.auth_token = None;
                        app.add_event("Auth token cleared.".to_string());
                    }
                    app.settings_mode = SettingsMode::View;
                    app.settings_input.clear();
                    EventResult::Consumed
                }
                KeyCode::Char(c) => {
                    app.settings_input.push(c);
                    EventResult::Consumed
                }
                KeyCode::Backspace => {
                    app.settings_input.pop();
                    EventResult::Consumed
                }
                KeyCode::Esc => {
                    app.settings_mode = SettingsMode::View;
                    app.settings_input.clear();
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
            SettingsMode::EditTheme | SettingsMode::EditSafeMode | SettingsMode::ToggleMouse => {
                if key == KeyCode::Esc || key == KeyCode::Enter || key == KeyCode::Char(' ') {
                    app.settings_mode = SettingsMode::View;
                    EventResult::Consumed
                } else {
                    EventResult::NotConsumed
                }
            }
            SettingsMode::Doctor => {
                if key == KeyCode::Esc || key == KeyCode::Enter || key == KeyCode::Char(' ') {
                    app.settings_mode = SettingsMode::View;
                    app.doctor_results.clear();
                    EventResult::Consumed
                } else {
                    EventResult::NotConsumed
                }
            }
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "Run Diagnostics".into(),
                WorkspaceAction::ExecCommand("doctor".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "i",
                description: "Edit refresh interval",
                group: "Settings",
            },
            KeyBinding {
                keys: "e",
                description: "Edit endpoint",
                group: "Settings",
            },
            KeyBinding {
                keys: "t",
                description: "Edit auth token",
                group: "Settings",
            },
            KeyBinding {
                keys: "h",
                description: "Cycle theme",
                group: "Settings",
            },
            KeyBinding {
                keys: "s",
                description: "Toggle safe mode",
                group: "Settings",
            },
            KeyBinding {
                keys: "m",
                description: "Toggle mouse",
                group: "Settings",
            },
            KeyBinding {
                keys: "d",
                description: "Run diagnostics",
                group: "Settings",
            },
        ]
    }
}

// ── FileBrowserWorkspace ─────────────────────────────────────────

pub struct FileBrowserWorkspace;

impl Workspace for FileBrowserWorkspace {
    fn section(&self) -> NavSection {
        NavSection::FileBrowser
    }

    fn title(&self) -> &str {
        "File Browser"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        crate::cli::tui::sections::files::render_file_browser(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        use crate::cli::tui::app::FileBrowserMode;
        match app.file_mode {
            FileBrowserMode::Browse => match key {
                KeyCode::Up => {
                    app.file_selected_index = app.file_selected_index.saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    app.file_selected_index =
                        (app.file_selected_index + 1).min(app.file_entries.len().saturating_sub(1));
                    EventResult::Consumed
                }
                KeyCode::Enter => {
                    if let Some(entry) = app.file_entries.get(app.file_selected_index) {
                        let clean = entry
                            .trim_start_matches("[DIR] ")
                            .trim_start_matches("[FILE] ")
                            .trim();
                        let path = format!("{}/{}", app.file_current_dir, clean);
                        if entry.starts_with("[DIR]") {
                            app.file_current_dir = path;
                            app.file_selected_index = 0;
                            EventResult::Action(WorkspaceAction::Refresh)
                        } else if entry.starts_with("[FILE]") {
                            app.file_selected_path = path.clone();
                            app.file_mode = FileBrowserMode::ReadFile;
                            app.file_scroll = 0;
                            EventResult::Action(WorkspaceAction::ExecCommand(format!(
                                "file_read:{}",
                                path
                            )))
                        } else {
                            EventResult::Consumed
                        }
                    } else {
                        EventResult::Consumed
                    }
                }
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    app.file_current_dir = std::env::current_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| "/".to_string());
                    app.file_selected_index = 0;
                    EventResult::Action(WorkspaceAction::Refresh)
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    EventResult::Action(WorkspaceAction::Refresh)
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    app.command_input = ":mkdir ".to_string();
                    app.show_command_palette = true;
                    app.command_palette_filtered = app.filter_commands();
                    app.command_palette_selection = 0;
                    EventResult::Consumed
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if let Some(entry) = app.file_entries.get(app.file_selected_index) {
                        let clean = entry
                            .trim_start_matches("[DIR] ")
                            .trim_start_matches("[FILE] ")
                            .trim();
                        let path = format!("{}/{}", app.file_current_dir, clean);
                        if std::fs::metadata(&path)
                            .map(|m| m.is_dir())
                            .unwrap_or(false)
                        {
                            return EventResult::Action(WorkspaceAction::StatusMessage(
                                "Cannot delete directory from here".into(),
                            ));
                        }
                        EventResult::Action(WorkspaceAction::Confirm(
                            format!("Delete file '{}'?", clean),
                            format!("file_delete:{}", path),
                        ))
                    } else {
                        EventResult::Consumed
                    }
                }
                KeyCode::Esc => {
                    let parent = std::path::Path::new(&app.file_current_dir)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "/".to_string());
                    app.file_current_dir = parent;
                    app.file_selected_index = 0;
                    EventResult::Action(WorkspaceAction::Refresh)
                }
                _ => EventResult::NotConsumed,
            },
            FileBrowserMode::ReadFile => match key {
                KeyCode::Up => {
                    app.file_scroll = app.file_scroll.saturating_sub(1);
                    EventResult::Consumed
                }
                KeyCode::Down => {
                    app.file_scroll += 1;
                    EventResult::Consumed
                }
                KeyCode::PageUp => {
                    app.file_scroll = app.file_scroll.saturating_sub(20);
                    EventResult::Consumed
                }
                KeyCode::PageDown => {
                    app.file_scroll += 20;
                    EventResult::Consumed
                }
                KeyCode::Esc | KeyCode::Backspace => {
                    app.file_mode = FileBrowserMode::Browse;
                    app.file_content = None;
                    app.file_scroll = 0;
                    EventResult::Consumed
                }
                _ => EventResult::NotConsumed,
            },
        }
    }

    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        vec![
            ("Refresh".into(), WorkspaceAction::Refresh),
            (
                "New Directory".into(),
                WorkspaceAction::StatusMessage("Use :mkdir <name> to create directory".into()),
            ),
        ]
    }

    fn key_bindings(&self) -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                keys: "Enter",
                description: "Open file/directory",
                group: "Files",
            },
            KeyBinding {
                keys: "h",
                description: "Home directory",
                group: "Files",
            },
            KeyBinding {
                keys: "r",
                description: "Refresh",
                group: "Files",
            },
            KeyBinding {
                keys: "n",
                description: "New directory",
                group: "Files",
            },
            KeyBinding {
                keys: "d",
                description: "Delete file",
                group: "Files",
            },
            KeyBinding {
                keys: "Esc",
                description: "Go to parent",
                group: "Files",
            },
        ]
    }
}

// ── LegacyAdapter ────────────────────────────────────────────────

/// Adapter that wraps existing free-function sections into Workspace trait.
/// Used for incremental migration — sections can be migrated one at a time.
pub struct LegacyAdapter {
    section: NavSection,
    render_fn: fn(&mut Frame, Rect, &TuiApp),
    key_handler: Option<fn(&mut TuiApp, KeyCode)>,
}

impl LegacyAdapter {
    pub fn new(section: NavSection, render_fn: fn(&mut Frame, Rect, &TuiApp)) -> Self {
        Self {
            section,
            render_fn,
            key_handler: None,
        }
    }

    pub fn with_key_handler(mut self, handler: fn(&mut TuiApp, KeyCode)) -> Self {
        self.key_handler = Some(handler);
        self
    }
}

impl Workspace for LegacyAdapter {
    fn section(&self) -> NavSection {
        self.section
    }

    fn title(&self) -> &str {
        ""
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp) {
        (self.render_fn)(frame, area, app);
    }

    fn handle_key(&mut self, app: &mut TuiApp, key: KeyCode) -> EventResult {
        if let Some(handler) = self.key_handler {
            handler(app, key);
            EventResult::Consumed
        } else {
            EventResult::NotConsumed
        }
    }
}

// ── create_all_workspaces ────────────────────────────────────────

pub fn create_all_workspaces() -> HashMap<NavSection, Box<dyn Workspace>> {
    let mut map: HashMap<NavSection, Box<dyn Workspace>> = HashMap::new();

    // Phase 2: already migrated
    map.insert(NavSection::Dashboard, Box::new(DashboardWorkspace));
    map.insert(NavSection::QueryConsole, Box::new(QueryConsoleWorkspace));
    map.insert(NavSection::Namespaces, Box::new(NamespacesWorkspace));
    map.insert(NavSection::Help, Box::new(HelpWorkspace));

    // Phase 3: remaining workspaces
    map.insert(
        NavSection::DatabasesEngines,
        Box::new(DatabasesEnginesWorkspace),
    );
    map.insert(NavSection::Cluster, Box::new(ClusterWorkspace));
    map.insert(NavSection::Federation, Box::new(FederationWorkspace));
    map.insert(NavSection::Governor, Box::new(GovernorWorkspace));
    map.insert(NavSection::BackupRestore, Box::new(BackupRestoreWorkspace));
    map.insert(NavSection::MetricsLogs, Box::new(MetricsLogsWorkspace));
    map.insert(
        NavSection::ConfigurationStudio,
        Box::new(ConfigStudioWorkspace),
    );
    map.insert(NavSection::TableExplorer, Box::new(TableExplorerWorkspace));
    map.insert(NavSection::ReportBuilder, Box::new(ReportBuilderWorkspace));
    map.insert(NavSection::Notebook, Box::new(NotebookWorkspace));
    map.insert(NavSection::RAGWorkspace, Box::new(RAGWorkspaceImpl));
    map.insert(
        NavSection::SecurityCenter,
        Box::new(SecurityCenterWorkspace),
    );
    map.insert(
        NavSection::DocumentWorkspace,
        Box::new(DocumentWorkspaceImpl),
    );
    map.insert(
        NavSection::IntegratedTerminal,
        Box::new(IntegratedTerminalWorkspace),
    );
    map.insert(NavSection::Monitoring, Box::new(MonitoringWorkspace));
    map.insert(NavSection::Settings, Box::new(SettingsWorkspace));
    map.insert(NavSection::FileBrowser, Box::new(FileBrowserWorkspace));

    map
}
