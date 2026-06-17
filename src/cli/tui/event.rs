use crate::cli::discovery::InstanceInfo;
use crate::cli::tui::api::{
    self, fetch_databases, fetch_diagnostics, fetch_journalctl, fetch_namespaces, fetch_settings,
    fetch_users, list_backups, run_discovery,
};
use crate::cli::tui::app::{NavSection, TuiApp};
use crate::cli::tui::render;
use crate::Result;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
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
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
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
        tokio::spawn(async move {
            if let Some(status) = api::fetch_status(&url_clone).await {
                let _ = section_tx.send(AppMessage::Status(Some(status)));
            }
            match section {
                NavSection::Clusters => {
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
                NavSection::Metrics => {
                    if let Some(v) = api::fetch_metrics(&url_clone).await {
                        let _ = section_tx.send(AppMessage::Metrics(v));
                    }
                }
                NavSection::Databases => {
                    let dbs = fetch_databases(&url_clone).await;
                    let _ = section_tx.send(AppMessage::Databases(dbs));
                }
                NavSection::Namespaces => {
                    let ns = fetch_namespaces(&url_clone).await;
                    let _ = section_tx.send(AppMessage::Namespaces(ns));
                }
                NavSection::Users => {
                    let users = fetch_users(&url_clone).await;
                    let _ = section_tx.send(AppMessage::UsersData(users));
                }
                NavSection::Diagnostics => {
                    let diag = fetch_diagnostics(&url_clone).await;
                    let _ = section_tx.send(AppMessage::Diagnostics(diag));
                }
                NavSection::TablesCollections => {
                    let data = api::fetch_tables(&url_clone).await;
                    let _ = section_tx.send(AppMessage::TablesData(data));
                }
                NavSection::VectorIndexes => {
                    let data = api::fetch_vector_indexes(&url_clone).await;
                    let _ = section_tx.send(AppMessage::VectorIndexesData(data));
                }
                NavSection::Graph => {
                    let data = api::fetch_graph_data(&url_clone).await;
                    let _ = section_tx.send(AppMessage::GraphData(data));
                }
                NavSection::AIML => {
                    let data = api::fetch_aiml_data(&url_clone).await;
                    let _ = section_tx.send(AppMessage::AIMLData(data));
                }
                NavSection::Roles => {
                    let data = api::fetch_roles(&url_clone).await;
                    let _ = section_tx.send(AppMessage::RolesData(data));
                }
                NavSection::Restores => {
                    let backups = api::list_backups();
                    let _ = section_tx.send(AppMessage::Backups(backups));
                    if let Some(detail) = api::list_backups_detail() {
                        let _ = section_tx.send(AppMessage::BackupsDetail(Some(detail)));
                    }
                }
                NavSection::Nodes => {
                    if let Some(v) = api::fetch_cluster_nodes(&url_clone).await {
                        let _ = section_tx.send(AppMessage::ClusterNodes(Some(v)));
                    }
                }
                NavSection::Settings => {
                    let settings = fetch_settings(&url_clone).await;
                    let _ = section_tx.send(AppMessage::Settings(settings));
                }
                _ => {}
            }
        });
    }

    let section = app.current_section;
    if section == NavSection::Backups || section == NavSection::Restores {
        let backups = list_backups();
        let _ = tx.send(AppMessage::Backups(backups));
        if let Some(detail) = api::list_backups_detail() {
            let _ = tx.send(AppMessage::BackupsDetail(Some(detail)));
        }
    }
    if section == NavSection::Logs {
        let logs = fetch_journalctl();
        let _ = tx.send(AppMessage::Logs(logs));
    }
}

async fn run_loop(
    mut terminal: Terminal<CrosstermBackend<io::Stdout>>,
    mut app: TuiApp,
    initial_url: Option<String>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<AppMessage>();

    let discover_tx = tx.clone();
    tokio::spawn(async move {
        let instances = run_discovery().await;
        let _ = discover_tx.send(AppMessage::Discovery(instances));
    });

    if let Some(url) = initial_url {
        let connect_tx = tx.clone();
        app.loading_message = format!("Connecting to {}...", url);
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
    }

    let tick_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
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
                                    plan.push_str("Migration Plan:\n");
                                    plan.push_str(&format!(
                                        "  Source: {} at {}\n",
                                        app.migration_source, app.migration_url
                                    ));
                                    plan.push_str(&format!(
                                        "  Target: {}\n",
                                        app.connected_url.as_deref().unwrap_or("(not connected)")
                                    ));
                                    plan.push_str(&format!(
                                        "  Namespace: {}\n",
                                        app.migration_namespace
                                    ));
                                    plan.push_str(&format!("  Mode: {}\n", app.migration_mode));
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
                                plan.push_str("Migration Plan:\n");
                                plan.push_str(&format!(
                                    "  Source: {} at {}\n",
                                    app.migration_source, app.migration_url
                                ));
                                plan.push_str(&format!(
                                    "  Target: {}\n",
                                    app.connected_url.as_deref().unwrap_or("(not connected)")
                                ));
                                plan.push_str(&format!(
                                    "  Namespace: {}\n",
                                    app.migration_namespace
                                ));
                                plan.push_str(&format!("  Mode: {}\n", app.migration_mode));
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
                    } else if app.show_command_palette {
                        match key.code {
                            KeyCode::Esc => {
                                app.show_command_palette = false;
                                app.command_input.clear();
                            }
                            KeyCode::Enter => {
                                let cmd = app.command_input.trim().to_string();
                                app.command_input.clear();
                                app.show_command_palette = false;
                                if cmd == ":quit" || cmd == ":q" {
                                    app.should_quit = true;
                                } else if cmd == ":help" || cmd == ":?" {
                                    app.current_section = NavSection::Help;
                                } else if cmd == ":refresh" || cmd == ":r" {
                                    trigger_refresh(&mut app, &tx);
                                } else if cmd.starts_with(":connect ") {
                                    let url =
                                        cmd.trim_start_matches(":connect ").trim().to_string();
                                    let connect_tx = tx.clone();
                                    app.loading = true;
                                    app.loading_message = format!("Connecting to {}...", url);
                                    let url2 = url.clone();
                                    tokio::spawn(async move {
                                        let healthy = api::fetch_health(&url).await.is_some();
                                        if healthy {
                                            let _ = connect_tx.send(AppMessage::Connected(url));
                                            if let Some(status) = api::fetch_status(&url2).await {
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
                                } else if !cmd.is_empty() {
                                    app.add_event(format!("Unknown command: {}", cmd));
                                }
                            }
                            KeyCode::Char(c) => {
                                app.command_input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.command_input.pop();
                            }
                            KeyCode::Delete => {
                                app.command_input.clear();
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
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
                                    app.current_section = NavSection::Migrations;
                                    app.command_input.clear();
                                }
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    app.should_quit = true;
                                } else if app.current_section == NavSection::Queries {
                                    app.query_input.push('q');
                                }
                            }
                            KeyCode::Esc => {
                                app.should_quit = true;
                            }
                            KeyCode::Tab => {
                                app.current_section = app.current_section.next();
                            }
                            KeyCode::BackTab => {
                                app.current_section = app.current_section.prev();
                            }
                            KeyCode::Up
                                if app.current_section == NavSection::Instances
                                    && !app.instances.is_empty() =>
                            {
                                app.selected_instance = app.selected_instance.saturating_sub(1);
                            }
                            KeyCode::Down
                                if app.current_section == NavSection::Instances
                                    && !app.instances.is_empty() =>
                            {
                                app.selected_instance =
                                    (app.selected_instance + 1).min(app.instances.len() - 1);
                            }
                            KeyCode::Enter
                                if app.current_section == NavSection::Queries
                                    && !app.query_input.is_empty()
                                    && app.connected() =>
                            {
                                let sql = app.query_input.trim().to_string();
                                app.query_input.clear();
                                let url = app.connected_url.clone().unwrap();
                                let query_tx = tx.clone();
                                app.add_event(format!("Executing: {}", sql));
                                tokio::spawn(async move {
                                    let result = api::fetch_query(&url, &sql).await;
                                    let _ = query_tx.send(AppMessage::QueryResult(result));
                                });
                            }
                            KeyCode::Enter
                                if app.current_section == NavSection::Instances
                                    && !app.instances.is_empty()
                                    && app.selected_instance < app.instances.len() =>
                            {
                                let url = app.instances[app.selected_instance].endpoint.clone();
                                let connect_tx = tx.clone();
                                app.loading = true;
                                app.loading_message = format!("Connecting to {}...", url);
                                let url2 = url.clone();
                                tokio::spawn(async move {
                                    let healthy = api::fetch_health(&url).await.is_some();
                                    if healthy {
                                        let _ = connect_tx.send(AppMessage::Connected(url));
                                        if let Some(status) = api::fetch_status(&url2).await {
                                            let _ =
                                                connect_tx.send(AppMessage::Status(Some(status)));
                                        }
                                    } else {
                                        let _ = connect_tx.send(AppMessage::Error(format!(
                                            "Could not reach {}",
                                            url
                                        )));
                                    }
                                });
                            }
                            KeyCode::Char(':') => {
                                app.show_command_palette = true;
                                app.command_input.clear();
                                app.command_input.push(':');
                            }
                            KeyCode::Char('b') | KeyCode::Char('B')
                                if key.modifiers.contains(KeyModifiers::CONTROL)
                                    && app.connected() =>
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
                                            let msg = String::from_utf8_lossy(&out.stdout)
                                                .trim()
                                                .to_string();
                                            let _ = backup_tx.send(AppMessage::BackupCreated(msg));
                                        }
                                        Ok(out) => {
                                            let err = String::from_utf8_lossy(&out.stderr)
                                                .trim()
                                                .to_string();
                                            let _ = backup_tx.send(AppMessage::BackupCreated(
                                                format!("Error: {}", err),
                                            ));
                                        }
                                        Err(e) => {
                                            let _ = backup_tx.send(AppMessage::BackupCreated(
                                                format!("Error: {}", e),
                                            ));
                                        }
                                    }
                                });
                            }
                            KeyCode::Char('r') | KeyCode::Char('R')
                                if key.modifiers.contains(KeyModifiers::CONTROL)
                                    && app.current_section == NavSection::Backups =>
                            {
                                let backup_name =
                                    app.backups_data.first().cloned().unwrap_or_default();
                                if backup_name.is_empty() {
                                    app.add_event("No backups available to restore.".to_string());
                                } else {
                                    app.add_event(format!("Restoring backup: {}...", backup_name));
                                    let backup_tx = tx.clone();
                                    let name = backup_name.clone();
                                    tokio::spawn(async move {
                                        let output = std::process::Command::new("primusdb")
                                            .args(["backup", "restore", &name, "--force"])
                                            .output();
                                        match output {
                                            Ok(out) if out.status.success() => {
                                                let msg = String::from_utf8_lossy(&out.stdout)
                                                    .trim()
                                                    .to_string();
                                                let _ =
                                                    backup_tx.send(AppMessage::BackupRestored(msg));
                                            }
                                            Ok(out) => {
                                                let err = String::from_utf8_lossy(&out.stderr)
                                                    .trim()
                                                    .to_string();
                                                let _ = backup_tx.send(AppMessage::BackupRestored(
                                                    format!("Restore failed: {}", err),
                                                ));
                                            }
                                            Err(e) => {
                                                let _ = backup_tx.send(AppMessage::BackupRestored(
                                                    format!("Restore error: {}", e),
                                                ));
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Char('v') | KeyCode::Char('V')
                                if key.modifiers.contains(KeyModifiers::CONTROL)
                                    && app.current_section == NavSection::Backups =>
                            {
                                let backup_name =
                                    app.backups_data.first().cloned().unwrap_or_default();
                                if backup_name.is_empty() {
                                    app.add_event("No backups available to verify.".to_string());
                                } else {
                                    app.add_event(format!("Verifying backup: {}...", backup_name));
                                    let backup_tx = tx.clone();
                                    let name = backup_name.clone();
                                    tokio::spawn(async move {
                                        let output = std::process::Command::new("primusdb")
                                            .args(["backup", "verify", &name])
                                            .output();
                                        match output {
                                            Ok(out) if out.status.success() => {
                                                let msg = String::from_utf8_lossy(&out.stdout)
                                                    .trim()
                                                    .to_string();
                                                let _ =
                                                    backup_tx.send(AppMessage::BackupCreated(msg));
                                            }
                                            Ok(out) => {
                                                let err = String::from_utf8_lossy(&out.stderr)
                                                    .trim()
                                                    .to_string();
                                                let _ = backup_tx.send(AppMessage::BackupCreated(
                                                    format!("Verify failed: {}", err),
                                                ));
                                            }
                                            Err(e) => {
                                                let _ = backup_tx.send(AppMessage::BackupCreated(
                                                    format!("Verify error: {}", e),
                                                ));
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Char('x') | KeyCode::Char('X')
                                if key.modifiers.contains(KeyModifiers::CONTROL)
                                    && app.current_section == NavSection::Backups =>
                            {
                                let backup_name =
                                    app.backups_data.first().cloned().unwrap_or_default();
                                if backup_name.is_empty() {
                                    app.add_event("No backups available to delete.".to_string());
                                } else if app.backup_in_progress {
                                    app.add_event(
                                        "Press Ctrl+X again to confirm deletion.".to_string(),
                                    );
                                    app.backup_in_progress = true;
                                } else {
                                    app.add_event(format!("Deleting backup: {}...", backup_name));
                                    app.backup_in_progress = false;
                                    let backup_tx = tx.clone();
                                    let name = backup_name.clone();
                                    tokio::spawn(async move {
                                        let output = std::process::Command::new("primusdb")
                                            .args(["backup", "delete", &name, "--force"])
                                            .output();
                                        match output {
                                            Ok(out) if out.status.success() => {
                                                let msg = String::from_utf8_lossy(&out.stdout)
                                                    .trim()
                                                    .to_string();
                                                let _ =
                                                    backup_tx.send(AppMessage::BackupCreated(msg));
                                            }
                                            Ok(out) => {
                                                let err = String::from_utf8_lossy(&out.stderr)
                                                    .trim()
                                                    .to_string();
                                                let _ = backup_tx.send(AppMessage::BackupCreated(
                                                    format!("Delete failed: {}", err),
                                                ));
                                            }
                                            Err(e) => {
                                                let _ = backup_tx.send(AppMessage::BackupCreated(
                                                    format!("Delete error: {}", e),
                                                ));
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Char('e') | KeyCode::Char('E')
                                if key.modifiers.contains(KeyModifiers::CONTROL)
                                    && app.current_section == NavSection::Queries
                                    && !app.query_input.is_empty()
                                    && app.connected() =>
                            {
                                let sql = app.query_input.trim().to_string();
                                app.query_input.clear();
                                let url = app.connected_url.clone().unwrap();
                                let query_tx = tx.clone();
                                app.add_event(format!("Executing: {}", sql));
                                tokio::spawn(async move {
                                    let result = api::fetch_query(&url, &sql).await;
                                    let _ = query_tx.send(AppMessage::QueryResult(result));
                                });
                            }
                            KeyCode::Char('d') | KeyCode::Char('D')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                app.add_event("Details view toggled".to_string());
                                app.show_instances = !app.show_instances;
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
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                trigger_refresh(&mut app, &tx);
                            }
                            KeyCode::Char('?') => {
                                app.current_section = NavSection::Help;
                            }
                            KeyCode::Char(c) if app.current_section == NavSection::Queries => {
                                app.query_input.push(c);
                            }
                            KeyCode::Backspace if app.current_section == NavSection::Queries => {
                                app.query_input.pop();
                            }
                            KeyCode::Delete if app.current_section == NavSection::Queries => {
                                app.query_input.clear();
                            }
                            KeyCode::PageDown if app.current_section == NavSection::Queries => {
                                app.query_scroll = app.query_scroll.saturating_add(10);
                            }
                            KeyCode::PageUp if app.current_section == NavSection::Queries => {
                                app.query_scroll = app.query_scroll.saturating_sub(10);
                            }
                            _ => {}
                        }
                    }
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
                }
                AppMessage::Status(value) => {
                    app.apply_status(value);
                }
                AppMessage::QueryResult(result) => {
                    app.query_results = result.lines().map(|s| s.to_string()).collect();
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
                    app.add_event("Settings refreshed".to_string());
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
                                let report = format!(
                                    "Migration completed.\nSource: {}\nNamespace: {}\nMode: {}\n\n{}",
                                    app.migration_source, app.migration_namespace, app.migration_mode, msg
                                );
                                app.migration_report = report;
                            }
                        }
                    } else {
                        app.migration_status = msg.clone();
                        app.add_event(format!("Migration completed: {}", msg));
                    }
                }
                AppMessage::MigrationError(msg) => {
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
                AppMessage::MigrationProgress(pct) => {
                    app.migration_progress = pct;
                    if pct >= 100 {
                        let report = format!(
                            "Migration completed.\nSource: {}\nNamespace: {}\nMode: {}",
                            app.migration_source, app.migration_namespace, app.migration_mode
                        );
                        app.migration_report = report;
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
                                let summary = api::fetch_cluster_summary(&url2).await;
                                let _ = tick_tx.send(AppMessage::ClusterSummary(summary));
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
            }
        }

        if app.should_quit {
            break;
        }
    }

    restore_terminal(&mut terminal)?;
    Ok(())
}

pub async fn run_tui() -> Result<()> {
    let terminal = setup_terminal().map_err(crate::Error::IOError)?;
    let app = TuiApp::new();
    run_loop(terminal, app, None).await
}

pub async fn run_tui_connect(url: &str) -> Result<()> {
    let terminal = setup_terminal().map_err(crate::Error::IOError)?;
    let app = TuiApp::new();
    run_loop(terminal, app, Some(url.to_string())).await
}
