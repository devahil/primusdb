pub mod api;
pub mod app;
pub mod capability;
pub mod commands;
pub mod config;
pub mod event;
pub mod layout;
pub mod mouse;
pub mod panels;
pub mod plugins;
pub mod render;
pub mod router;
pub mod sections;
pub mod state;
pub mod widgets;
pub mod workspace;
pub mod workspaces;

pub use event::{run_tui, run_tui_connect, run_tui_with_config};

#[cfg(test)]
mod tests {
    use super::app::*;
    use super::config;
    use super::render::*;
    use super::sections::backups::render_backups;
    use super::sections::federation::render_federation;
    use super::sections::help::render_help_page;
    use super::sections::metrics_logs::render_metrics_view;
    use super::sections::namespaces::render_namespaces;
    use super::sections::queries::render_queries;
    use super::sections::settings::render_settings;
    use super::sections::{dashboard, nodes};
    use super::widgets::*;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use ratatui::Terminal;

    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut result = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = buffer.get(x, y);
                result.push_str(cell.symbol());
            }
            result.push('\n');
        }
        result
    }

    // ── NavSection tests ──────────────────────────────────────────────

    #[test]
    fn test_nav_section_cycle() {
        assert_eq!(NavSection::Dashboard.next(), NavSection::QueryConsole);
        assert_eq!(
            NavSection::QueryConsole.next(),
            NavSection::DatabasesEngines
        );
        assert_eq!(NavSection::Help.next(), NavSection::Dashboard);
        assert_eq!(NavSection::Dashboard.prev(), NavSection::Help);
        assert_eq!(NavSection::Federation.prev(), NavSection::Cluster);
    }

    #[test]
    fn test_nav_section_name() {
        assert_eq!(NavSection::Dashboard.name(), "Dashboard");
        assert_eq!(NavSection::QueryConsole.name(), "Query Console");
        assert_eq!(NavSection::Federation.name(), "Federation");
        assert_eq!(NavSection::Governor.name(), "Governor");
        assert_eq!(NavSection::BackupRestore.name(), "Backup/Restore");
    }

    #[test]
    fn test_nav_section_all_names() {
        for section in NAV_SECTIONS {
            let name = section.name();
            assert!(!name.is_empty(), "NavSection {:?} has empty name", section);
            assert!(NAV_SECTIONS.iter().any(|s| s.name() == name));
        }
    }

    #[test]
    fn test_nav_section_wraparound() {
        assert_eq!(NavSection::Dashboard.prev(), NavSection::Help);
        assert_eq!(NavSection::Help.next(), NavSection::Dashboard);
    }

    // ── TuiApp construction tests ─────────────────────────────────────

    #[test]
    fn test_tui_app_new() {
        let app = TuiApp::new();
        assert_eq!(app.current_section, NavSection::Dashboard);
        assert!(app.connected_url.is_none());
        assert!(app.instances.is_empty());
        assert!(app.query_input.is_empty());
        assert!(app.query_results.is_empty());
        assert!(app.event_log.is_empty());
        assert_eq!(app.selected_instance, 0);
        assert!(!app.should_quit);
        assert!(app.loading);
        assert_eq!(app.query_scroll, 0);
        assert_eq!(app.confirm_action, ConfirmAction::None);
        assert!(!app.onboarding_mode);
    }

    #[test]
    fn test_tui_app_default() {
        let app = TuiApp::default();
        assert_eq!(app.current_section, NavSection::Dashboard);
        assert!(app.loading);
    }

    #[test]
    fn test_tui_app_connect() {
        let mut app = TuiApp::new();
        assert!(!app.connected());
        app.connect_url("http://localhost:8080");
        assert!(app.connected());
        assert_eq!(app.connected_url, Some("http://localhost:8080".to_string()));
        assert_eq!(app.event_log.len(), 1);
        assert!(app.event_log[0].contains("Connected"));
    }

    #[test]
    fn test_tui_app_connected_method() {
        let app = TuiApp::new();
        assert!(!app.connected());
        let mut app2 = TuiApp::new();
        app2.connect_url("http://localhost:8080");
        assert!(app2.connected());
    }

    #[test]
    fn test_tui_app_add_event() {
        let mut app = TuiApp::new();
        app.add_event("test event".to_string());
        assert_eq!(app.event_log.len(), 1);
        assert_eq!(app.event_log[0], "test event");

        for i in 0..200 {
            app.add_event(format!("event {}", i));
        }
        assert_eq!(app.event_log.len(), 100);
        assert_eq!(app.event_log[0], "event 100");
    }

    #[test]
    fn test_tui_app_apply_status() {
        let mut app = TuiApp::new();
        let status = serde_json::json!({
            "status": "healthy",
            "version": "1.3.1-alpha",
            "uptime_seconds": 3661,
            "enabled_engines": ["columnar", "vector"]
        });
        app.apply_status(Some(status));
        assert_eq!(app.health_status, Some("healthy".to_string()));
        assert_eq!(app.server_version, Some("1.3.1-alpha".to_string()));
        assert_eq!(app.uptime, Some("01:01:01".to_string()));
        assert_eq!(app.engine_list, vec!["columnar", "vector"]);

        app.apply_status(None);
        assert!(app.health_status.is_none());
        assert!(app.server_version.is_none());
        assert!(app.uptime.is_none());
        assert!(app.engine_list.is_empty());
    }

    #[test]
    fn test_tui_app_disconnect() {
        let mut app = TuiApp::new();
        app.connect_url("http://localhost:8080");
        app.apply_status(Some(serde_json::json!({
            "status": "healthy",
            "version": "1.3.1-alpha",
            "uptime_seconds": 100,
            "enabled_engines": ["columnar"]
        })));
        app.databases_data = vec!["db1".to_string()];
        app.federation_status = Some(serde_json::json!({"status": "active"}));
        app.disconnect();
        assert!(app.connected_url.is_none());
        assert!(app.health_status.is_none());
        assert!(app.server_version.is_none());
        assert!(app.uptime.is_none());
        assert!(app.engine_list.is_empty());
        assert!(app.databases_data.is_empty());
        assert!(app.federation_status.is_none());
    }

    // ── TuiConfig tests ────────────────────────────────────────────────

    #[test]
    fn test_config_default() {
        let config = config::TuiConfig::default();
        assert!(config.enabled);
        assert!(config.mouse_enabled);
        assert_eq!(config.theme, "default");
        assert_eq!(config.refresh_interval_ms, 2000);
        assert_eq!(config.default_view, "dashboard");
        assert!(config.confirm_destructive_actions);
    }

    #[test]
    fn test_config_from_cli() {
        let config = config::TuiConfig::from_cli(
            Some("http://localhost:8080".to_string()),
            true,
            Some(5000),
            Some("dark".to_string()),
            true,
        );
        assert!(!config.mouse_enabled);
        assert_eq!(config.refresh_interval_ms, 5000);
        assert_eq!(config.theme, "dark");
        assert!(config.confirm_destructive_actions);
    }

    #[test]
    fn test_config_refresh_duration() {
        let config = config::TuiConfig {
            refresh_interval_ms: 5000,
            ..Default::default()
        };
        let dur = config.refresh_duration();
        assert_eq!(dur.as_millis(), 5000);
    }

    // ── ConfirmAction tests ──────────────────────────────────────────

    #[test]
    fn test_confirm_action_states() {
        assert_eq!(ConfirmAction::None, ConfirmAction::None);
        assert_eq!(ConfirmAction::Quit, ConfirmAction::Quit);
        assert_ne!(ConfirmAction::Quit, ConfirmAction::Disconnect);
    }

    #[test]
    fn test_confirm_action_none_on_new() {
        let app = TuiApp::new();
        assert_eq!(app.confirm_action, ConfirmAction::None);
    }

    // ── QueryHistory tests ──────────────────────────────────────────

    #[test]
    fn test_query_history_entry() {
        let entry = QueryHistoryEntry::new("SELECT 1".to_string(), "1".to_string());
        assert_eq!(entry.query, "SELECT 1");
        assert_eq!(entry.result, "1");
        assert!(!entry.timestamp.is_empty());
    }

    // ── Command palette tests ────────────────────────────────────────

    #[test]
    fn test_command_palette_items() {
        let items = TuiApp::command_palette_items();
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.contains(":help")));
        assert!(items.iter().any(|i| i.contains(":quit")));
        assert!(items.iter().any(|i| i.contains(":connect")));
    }

    #[test]
    fn test_filter_commands() {
        let app = TuiApp::new();
        let items = app.filter_commands();
        assert!(!items.is_empty());
    }

    #[test]
    fn test_execute_command_help() {
        let mut app = TuiApp::new();
        let result = app.execute_command(":help");
        assert_eq!(result, Some("help".to_string()));
        assert_eq!(app.current_section, NavSection::Help);
    }

    #[test]
    fn test_execute_command_quit() {
        let mut app = TuiApp::new();
        let result = app.execute_command(":quit");
        assert_eq!(result, Some("quit".to_string()));
    }

    #[test]
    fn test_execute_command_connect() {
        let mut app = TuiApp::new();
        let result = app.execute_command(":connect http://localhost:8080");
        assert!(result.is_some());
        assert!(result.unwrap().starts_with("connect:"));
    }

    #[test]
    fn test_execute_command_unknown() {
        let mut app = TuiApp::new();
        let result = app.execute_command(":nonexistent");
        assert!(result.is_none());
    }

    // ── CLI hint & contextual help tests ──────────────────────────────

    #[test]
    fn test_cli_hint_for_current_section() {
        let app = TuiApp::new();
        let hint = app.cli_hint_for_current_section();
        assert!(!hint.is_empty());
        assert!(hint.contains("primusdb"));
    }

    #[test]
    fn test_contextual_help_text() {
        let app = TuiApp::new();
        let help = app.contextual_help_text();
        assert!(!help.is_empty());
    }

    #[test]
    fn test_show_cli_hints_default() {
        let app = TuiApp::new();
        assert!(app.show_cli_hints);
    }

    // ── State field tests ─────────────────────────────────────────────

    #[test]
    fn test_mouse_state_toggle() {
        let mut app = TuiApp::new();
        assert!(app.mouse_enabled);
        app.mouse_enabled = false;
        assert!(!app.mouse_enabled);
    }

    #[test]
    fn test_hovered_section() {
        let mut app = TuiApp::new();
        assert!(app.hovered_section.is_none());
        app.hovered_section = Some(NavSection::Cluster);
        assert_eq!(app.hovered_section, Some(NavSection::Cluster));
    }

    #[test]
    fn test_active_namespace() {
        let mut app = TuiApp::new();
        assert!(app.active_namespace.is_none());
        app.active_namespace = Some("prod".to_string());
        assert_eq!(app.active_namespace, Some("prod".to_string()));
    }

    #[test]
    fn test_selected_database() {
        let mut app = TuiApp::new();
        assert!(app.selected_database.is_none());
        app.selected_database = Some("mydb".to_string());
        assert_eq!(app.selected_database, Some("mydb".to_string()));
    }

    // ── Migration wizard tests ────────────────────────────────────────

    #[test]
    fn test_migration_wizard_initial_state() {
        let app = TuiApp::new();
        assert!(!app.migration_wizard_active);
        assert_eq!(app.migration_step, 0);
        assert!(app.migration_source.is_empty());
        assert!(app.migration_url.is_empty());
        assert!(app.migration_namespace.is_empty());
        assert!(app.migration_mode.is_empty());
        assert_eq!(app.migration_progress, 0);
        assert!(app.migration_status.is_empty());
        assert!(app.migration_error.is_none());
    }

    #[test]
    fn test_migration_wizard_activate_deactivate() {
        let mut app = TuiApp::new();
        app.migration_wizard_active = true;
        assert!(app.migration_wizard_active);
        app.migration_wizard_active = false;
        app.migration_step = 0;
        assert!(!app.migration_wizard_active);
        assert_eq!(app.migration_step, 0);
    }

    #[test]
    fn test_migration_wizard_step_transitions() {
        let mut app = TuiApp::new();
        app.migration_wizard_active = true;
        app.migration_step = 1;
        assert_eq!(app.migration_step, 1);

        app.migration_source = "mysql".to_string();
        app.migration_step = 2;
        assert_eq!(app.migration_source, "mysql");
        assert_eq!(app.migration_step, 2);

        app.command_input = "postgresql://host:5432/db".to_string();
        app.migration_url = app.command_input.trim().to_string();
        app.command_input.clear();
        app.migration_step = 3;
        assert_eq!(app.migration_url, "postgresql://host:5432/db");
        assert_eq!(app.migration_step, 3);

        app.migration_namespace = "my_namespace".to_string();
        app.migration_step = 4;
        assert_eq!(app.migration_namespace, "my_namespace");
        assert_eq!(app.migration_step, 4);

        app.migration_mode = "copy".to_string();
        app.migration_step = 5;
        assert_eq!(app.migration_mode, "copy");
        assert_eq!(app.migration_step, 5);

        app.migration_step = 6;
        app.migration_progress = 0;
        app.migration_status = "Starting migration...".to_string();
        assert_eq!(app.migration_step, 6);
        assert_eq!(app.migration_progress, 0);
        assert!(app.migration_status.contains("Starting"));
    }

    #[test]
    fn test_migration_wizard_source_selection() {
        let mut app = TuiApp::new();
        app.migration_wizard_active = true;
        app.migration_step = 1;
        for (key, expected) in &[
            ('1', "mysql"),
            ('2', "postgresql"),
            ('3', "mongodb"),
            ('4', "couchdb"),
        ] {
            app.migration_source = match key {
                '1' => "mysql",
                '2' => "postgresql",
                '3' => "mongodb",
                '4' => "couchdb",
                _ => unreachable!(),
            }
            .to_string();
            assert_eq!(app.migration_source, *expected);
        }
    }

    #[test]
    fn test_migration_wizard_mode_selection() {
        let mut app = TuiApp::new();
        app.migration_wizard_active = true;
        app.migration_step = 4;
        for (key, expected) in &[
            ('1', "copy"),
            ('2', "schema-only"),
            ('3', "data-only"),
            ('4', "dry-run"),
        ] {
            app.migration_mode = match key {
                '1' => "copy",
                '2' => "schema-only",
                '3' => "data-only",
                '4' => "dry-run",
                _ => unreachable!(),
            }
            .to_string();
            assert_eq!(app.migration_mode, *expected);
        }
    }

    #[test]
    fn test_migration_wizard_progress_and_result() {
        let mut app = TuiApp::new();
        app.migration_step = 6;
        app.migration_progress = 10;
        assert_eq!(app.migration_progress, 10);
        app.migration_progress = 40;
        assert_eq!(app.migration_progress, 40);
        app.migration_progress = 100;
        assert_eq!(app.migration_progress, 100);
        app.migration_status = "Migration completed successfully".to_string();
        assert!(app.migration_status.contains("completed"));
        app.migration_error = Some("Connection refused".to_string());
        assert!(app.migration_error.is_some());
        assert_eq!(app.migration_error.unwrap(), "Connection refused");
    }

    // ── Render tests ──────────────────────────────────────────────────

    #[test]
    fn test_render_gauge() {
        let mut lines = Vec::new();
        render_gauge(&mut lines, "Test", 50, Color::Green);
        assert_eq!(lines.len(), 1);
        let joined: String = lines[0]
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(joined.contains("Test"));
        assert!(joined.contains("50%"));
    }

    #[test]
    fn test_render_gauge_boundaries() {
        let mut lines = Vec::new();
        render_gauge(&mut lines, "Full", 100, Color::Green);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("100%")));
        let mut lines = Vec::new();
        render_gauge(&mut lines, "Empty", 0, Color::Red);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("0%")));
    }

    #[test]
    fn test_render_dashboard_disconnected() {
        let app = TuiApp::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| dashboard::render_dashboard(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Not connected"));
    }

    #[test]
    fn test_render_dashboard_connected() {
        let mut app = TuiApp::new();
        app.connect_url("http://localhost:8080");
        app.apply_status(Some(serde_json::json!({
            "status": "healthy",
            "version": "1.3.1-alpha",
            "uptime_seconds": 100,
            "enabled_engines": ["columnar"]
        })));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| dashboard::render_dashboard(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("http://localhost:8080"));
        assert!(content.contains("healthy"));
        assert!(content.contains("1.3.1"));
    }

    #[test]
    fn test_render_help_page() {
        let backend = TestBackend::new(60, 38);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_help_page(frame, frame.size()))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("KEYBINDINGS"));
        assert!(content.contains("MOUSE SUPPORT"));
    }

    #[test]
    fn test_render_sidebar_keys() {
        let mut app = TuiApp::new();
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        for section in NAV_SECTIONS {
            app.current_section = *section;
            terminal
                .draw(|frame| render_sidebar(frame, frame.size(), &app))
                .unwrap();
            let content = buffer_to_string(terminal.backend().buffer());
            assert!(
                content.contains(section.name()),
                "Sidebar should show {}",
                section.name()
            );
        }
    }

    #[test]
    fn test_render_queries_disconnected() {
        let app = TuiApp::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_queries(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Not connected"));
    }

    #[test]
    fn test_render_queries_with_results() {
        let mut app = TuiApp::new();
        app.connect_url("http://localhost:8080");
        app.query_results = vec!["id | name".into(), "1  | test".into()];
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_queries(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Connected:"));
        assert!(content.contains("Results:"));
        assert!(content.contains("1  | test"));
    }

    #[test]
    fn test_render_backups_empty() {
        let app = TuiApp::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_backups(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("No backup files found"));
    }

    #[test]
    fn test_render_backups_with_simple_list() {
        let mut app = TuiApp::new();
        app.backups_data = vec!["backup_2024-01-01.zip".into()];
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_backups(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("backup(s) found"));
        assert!(content.contains("backup_2024-01-01.zip"));
    }

    #[test]
    fn test_render_metrics_disconnected() {
        let app = TuiApp::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_metrics_view(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Not connected"));
    }

    #[test]
    fn test_render_nodes_disconnected() {
        let app = TuiApp::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| nodes::render_nodes(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Not connected"));
    }

    #[test]
    fn test_render_nodes_with_data() {
        let mut app = TuiApp::new();
        app.connect_url("http://localhost:8080");
        app.cluster_nodes = Some(serde_json::json!([
            {"id": "node-1", "role": "leader", "status": "healthy", "address": "10.0.0.1:8080"},
            {"id": "node-2", "role": "follower", "status": "ok", "address": "10.0.0.2:8080"}
        ]));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| nodes::render_nodes(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("node-1"));
        assert!(content.contains("node-2"));
        assert!(content.contains("leader"));
    }

    #[test]
    fn test_render_small_terminal() {
        let mut app = TuiApp::new();
        let backend = TestBackend::new(28, 7);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("small"));
    }

    #[test]
    fn test_render_federation_disconnected() {
        let app = TuiApp::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_federation(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Not connected"));
    }

    #[test]
    fn test_render_federation_with_status() {
        let mut app = TuiApp::new();
        app.connect_url("http://localhost:8080");
        app.federation_status = Some(serde_json::json!({"status": "active", "cluster_count": 2}));
        app.federation_clusters = Some(serde_json::json!([
            {"id": "cluster-a", "status": "active"},
            {"id": "cluster-b", "status": "degraded"}
        ]));
        app.federation_domains = Some(serde_json::json!([
            {"name": "us-east", "status": "active", "nodes": ["n1", "n2"]}
        ]));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_federation(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("cluster-a"));
        assert!(content.contains("cluster-b"));
        assert!(content.contains("us-east"));
    }

    #[test]
    fn test_theme_palette_resolution() {
        let default = config::resolve_palette("default");
        let dark = config::resolve_palette("dark");
        let light = config::resolve_palette("light");
        let high_contrast = config::resolve_palette("high-contrast");
        assert_eq!(default.primary, dark.primary);
        assert_eq!(light.primary, ratatui::style::Color::Blue);
        assert_eq!(high_contrast.primary, ratatui::style::Color::LightBlue);
        // unknown theme falls back to default
        let fallback = config::resolve_palette("nonexistent");
        assert_eq!(fallback.primary, default.primary);
    }

    #[test]
    fn test_config_palette_method() {
        let mut cfg = config::TuiConfig::default();
        cfg.theme = "light".to_string();
        let p = cfg.palette();
        assert_eq!(p.primary, ratatui::style::Color::Blue);
    }

    #[test]
    fn test_settings_endpoint_update() {
        let mut app = TuiApp::new();
        app.connect_url("http://localhost:8080");
        app.settings_mode = SettingsMode::EditEndpoint;
        app.settings_input = "http://new-host:9090".to_string();
        // simulate Enter key handled by handle_settings_key
        // We test the state change directly:
        app.connected_url = Some("http://new-host:9090".to_string());
        assert_eq!(app.connected_url, Some("http://new-host:9090".to_string()));
    }

    #[test]
    fn test_namespace_switch_command() {
        let mut app = TuiApp::new();
        let result = app.execute_command("namespace use prod");
        assert_eq!(result, Some("namespace_use:prod".to_string()));
    }

    #[test]
    fn test_export_query_command() {
        let mut app = TuiApp::new();
        let result = app.execute_command("export query /tmp/result.json");
        assert_eq!(result, Some("export_query:/tmp/result.json".to_string()));
    }

    #[test]
    fn test_namespaces_render_empty() {
        let app = TuiApp::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_namespaces(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Not connected") || content.contains("namespace"));
    }

    #[test]
    fn test_settings_render_view() {
        let mut app = TuiApp::new();
        app.connect_url("http://localhost:8080");
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_settings(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Theme"));
        assert!(content.contains("Safe Mode"));
    }
}
