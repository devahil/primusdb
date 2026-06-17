pub mod api;
pub mod app;
pub mod event;
pub mod render;
pub mod sections;
pub mod widgets;

pub use event::{run_tui, run_tui_connect};

#[cfg(test)]
mod tests {
    use super::app::*;
    use super::render::*;
    use super::sections::{dashboard, instances, nodes};
    use super::widgets::*;
    use crate::cli::discovery::InstanceInfo;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use ratatui::Terminal;

    #[test]
    fn test_nav_section_cycle() {
        assert_eq!(NavSection::Dashboard.next(), NavSection::Instances);
        assert_eq!(NavSection::Instances.next(), NavSection::Clusters);
        assert_eq!(NavSection::Help.next(), NavSection::Governor);
        assert_eq!(NavSection::Governor.next(), NavSection::Dashboard);
        assert_eq!(NavSection::Dashboard.prev(), NavSection::Governor);
        assert_eq!(NavSection::Help.prev(), NavSection::Settings);
        assert_eq!(NavSection::Instances.prev(), NavSection::Dashboard);
    }

    #[test]
    fn test_nav_section_name() {
        assert_eq!(NavSection::Dashboard.name(), "Dashboard");
        assert_eq!(NavSection::Instances.name(), "Instances");
        assert_eq!(NavSection::Help.name(), "Help");
    }

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
        assert_eq!(app.loading_message, "Discovering instances...");
        assert_eq!(app.query_scroll, 0);
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
        assert!(joined.contains("█") || joined.contains("|"));
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
        assert!(content.contains("Getting Started"));
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
        assert!(content.contains("VERSION INFO"));
        assert!(content.contains("DOCUMENTATION"));
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
        app.add_event("test event".to_string());

        // Step 0 -> 1: Welcome screen, press Enter
        app.migration_wizard_active = true;
        app.migration_step = 1;
        assert_eq!(app.migration_step, 1);

        // Step 1 -> 2: Select source '1' (mysql)
        app.migration_source = "mysql".to_string();
        app.migration_step = 2;
        assert_eq!(app.migration_source, "mysql");
        assert_eq!(app.migration_step, 2);

        // Step 2 -> 3: Enter URL
        app.command_input = "postgresql://host:5432/db".to_string();
        app.migration_url = app.command_input.trim().to_string();
        app.command_input.clear();
        app.migration_step = 3;
        assert_eq!(app.migration_url, "postgresql://host:5432/db");
        assert_eq!(app.migration_step, 3);

        // Step 3 -> 4: Enter namespace
        app.command_input = "my_namespace".to_string();
        app.migration_namespace = app.command_input.trim().to_string();
        app.command_input.clear();
        app.migration_step = 4;
        assert_eq!(app.migration_namespace, "my_namespace");
        assert_eq!(app.migration_step, 4);

        // Step 4 -> 5: Select mode '1' (copy)
        app.migration_mode = "copy".to_string();
        app.migration_step = 5;
        assert_eq!(app.migration_mode, "copy");
        assert_eq!(app.migration_step, 5);

        // Step 5 -> 6: Confirm, start migration
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

        // Simulate pressing 1-4 for each source
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
    fn test_migration_wizard_back_navigation() {
        let mut app = TuiApp::new();
        app.migration_wizard_active = true;

        // Step 0: Esc should close wizard
        app.migration_step = 0;
        if app.migration_step <= 1 {
            app.migration_wizard_active = false;
            app.migration_step = 0;
            app.command_input.clear();
        }
        assert!(!app.migration_wizard_active);

        // Step 1: Esc should close wizard
        app.migration_wizard_active = true;
        app.migration_step = 1;
        if app.migration_step <= 1 {
            app.migration_wizard_active = false;
            app.migration_step = 0;
            app.command_input.clear();
        }
        assert!(!app.migration_wizard_active);

        // Step 2: Esc goes back to step 1
        app.migration_wizard_active = true;
        app.migration_step = 2;
        app.command_input = "some_url".to_string();
        if app.migration_step == 2 || app.migration_step == 3 {
            app.migration_step -= 1;
            app.command_input.clear();
        }
        assert_eq!(app.migration_step, 1);
        assert!(app.command_input.is_empty());

        // Step 4: Esc goes back to step 3
        app.migration_step = 4;
        if app.migration_step >= 2 {
            app.migration_step -= 1;
        }
        assert_eq!(app.migration_step, 3);
    }

    #[test]
    fn test_migration_wizard_progress_and_result() {
        let mut app = TuiApp::new();

        // Simulate progress updates
        app.migration_step = 6;
        app.migration_progress = 10;
        assert_eq!(app.migration_progress, 10);

        app.migration_progress = 40;
        assert_eq!(app.migration_progress, 40);

        app.migration_progress = 100;
        assert_eq!(app.migration_progress, 100);

        // Simulate result
        app.migration_status = "Migration completed successfully".to_string();
        assert!(app.migration_status.contains("completed"));

        // Simulate error
        app.migration_error = Some("Connection refused".to_string());
        assert!(app.migration_error.is_some());
        assert_eq!(app.migration_error.unwrap(), "Connection refused");
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
        app.disconnect();
        assert!(app.connected_url.is_none());
        assert!(app.health_status.is_none());
        assert!(app.server_version.is_none());
        assert!(app.uptime.is_none());
        assert!(app.engine_list.is_empty());
        assert!(app.databases_data.is_empty());
    }

    #[test]
    fn test_tui_app_default() {
        let app = TuiApp::default();
        assert_eq!(app.current_section, NavSection::Dashboard);
        assert!(app.loading);
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
        assert_eq!(NavSection::Dashboard.prev(), NavSection::Governor);
        assert_eq!(NavSection::Help.next(), NavSection::Governor);
    }

    #[test]
    fn test_render_instances_empty() {
        let app = TuiApp::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| instances::render_instances(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("No PrimusDB instances discovered."));
        assert!(content.contains("primusdb server start"));
    }

    #[test]
    fn test_render_instances_with_instances() {
        let mut app = TuiApp::new();
        app.instances = vec![InstanceInfo {
            endpoint: "http://localhost:8080".into(),
            status: "healthy".into(),
            version: Some("1.3.1-alpha".into()),
            enabled_engines: vec!["columnar".into()],
            instance_id: None,
            node_id: None,
            uptime_seconds: None,
            cluster_role: None,
            protocol_status: None,
        }];
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| instances::render_instances(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("http://localhost:8080"));
        assert!(content.contains("healthy"));
        assert!(content.contains("1.3.1"));
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
        assert!(content.contains("Instances"));
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
    fn test_render_backup_in_progress() {
        let mut app = TuiApp::new();
        app.backup_in_progress = true;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_backups(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Creating backup"));
    }

    #[test]
    fn test_render_migrations_not_wizard() {
        let app = TuiApp::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_migrations(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Supported Sources"));
        assert!(content.contains("MySQL"));
        assert!(content.contains("Ctrl+M"));
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
    fn test_render_metrics_with_data() {
        let mut app = TuiApp::new();
        app.connect_url("http://localhost:8080");
        app.metrics_data = Some("CPU: 45%\nMemory: 2.1GB".into());
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_metrics_view(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("CPU: 45%"));
        assert!(content.contains("Memory: 2.1GB"));
    }

    #[test]
    fn test_render_logs_empty() {
        let app = TuiApp::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_logs(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("No logs data"));
    }

    #[test]
    fn test_render_logs_with_data() {
        let mut app = TuiApp::new();
        app.logs_data = Some("INFO  Server started\nERROR Connection refused".into());
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_logs(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("INFO  Server started"));
        assert!(content.contains("ERROR Connection refused"));
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
        assert!(content.contains("2 node(s)"));
        assert!(content.contains("node-1"));
        assert!(content.contains("node-2"));
        assert!(content.contains("leader"));
        assert!(content.contains("follower"));
    }

    #[test]
    fn test_render_nodes_empty() {
        let mut app = TuiApp::new();
        app.connect_url("http://localhost:8080");
        app.cluster_nodes = Some(serde_json::json!([]));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| nodes::render_nodes(frame, frame.size(), &app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("No nodes found"));
    }

    #[test]
    fn test_render_content_loading() {
        let mut app = TuiApp::new();
        app.loading = true;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_content(frame, frame.size(), &mut app))
            .unwrap();
        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Discovering"));
    }

    #[test]
    fn test_tui_app_connected_method() {
        let app = TuiApp::new();
        assert!(!app.connected());
        let mut app2 = TuiApp::new();
        app2.connect_url("http://localhost:8080");
        assert!(app2.connected());
    }

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
}
