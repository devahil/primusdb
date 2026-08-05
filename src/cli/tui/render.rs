#![allow(clippy::vec_init_then_push)]
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::cli::tui::app::{
    ConfirmAction, NavSection, SearchScope, TuiApp, HEADER_HEIGHT, INPUT_HEIGHT, MIN_TERMINAL_H,
    MIN_TERMINAL_W, NAV_SECTIONS, SIDEBAR_WIDTH, STATUS_HEIGHT, VERSION,
};
use crate::cli::tui::config::resolve_palette;
use crate::cli::tui::sections;
use crate::cli::tui::widgets::{render_loading, spanned_line};

/// Normalizes error messages by stripping redundant prefixes like "Error: ",
/// converting common API error patterns to user-friendly strings,
/// and trimming whitespace.
pub fn normalize_error(msg: &str) -> String {
    let msg = msg.trim();
    let msg = msg.strip_prefix("Error: ").unwrap_or(msg);
    let msg = msg.strip_prefix("error: ").unwrap_or(msg);
    let msg = msg.strip_prefix("ERROR: ").unwrap_or(msg);
    let msg = msg.strip_prefix("API error: ").unwrap_or(msg);
    let msg = msg.strip_prefix("api error: ").unwrap_or(msg);
    if msg.contains("Connection refused") {
        return "Connection refused \u{2014} is the server running?".to_string();
    }
    if msg.contains("timed out") || msg.contains("timeout") {
        return "Request timed out \u{2014} check network connectivity.".to_string();
    }
    if msg.contains("not found") || msg.contains("does not exist") {
        return "Resource not found.".to_string();
    }
    if msg.contains("unauthorized") || msg.contains("forbidden") || msg.contains("403") {
        return "Authentication/authorization failed \u{2014} check your token.".to_string();
    }
    if msg.contains("invalid JSON") || msg.contains("parse error") || msg.contains("syntax error") {
        return "Invalid input format \u{2014} check your data.".to_string();
    }
    msg.to_string()
}

pub fn render(frame: &mut Frame, app: &mut TuiApp) {
    let area = frame.size();
    if area.width < 30 || area.height < 8 {
        let text = Text::from(Line::from(Span::styled(
            "Terminal too small \u{2014} resize to at least 60x20",
            Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("PrimusDB TUI")),
            area,
        );
        return;
    }

    if area.width < MIN_TERMINAL_W || area.height < MIN_TERMINAL_H {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(HEADER_HEIGHT), Constraint::Min(0)])
            .split(area);

        render_header(frame, chunks[0], app);
        let content_area = Rect {
            x: 0,
            y: chunks[1].y,
            width: area.width,
            height: chunks[1].height.saturating_sub(2),
        };
        render_content(frame, content_area, app);
        let hint = Line::from(Span::styled(
            " Terminal < 60x20 \u{2014} sidebar hidden \u{2014} resize for full layout ",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::DIM),
        ));
        frame.render_widget(
            Paragraph::new(hint),
            Rect {
                x: 0,
                y: area.height.saturating_sub(1),
                width: area.width,
                height: 1,
            },
        );
        render_overlays(frame, area, app);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Min(0),
            Constraint::Length(INPUT_HEIGHT),
            Constraint::Length(STATUS_HEIGHT),
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    render_main_area(frame, chunks[1], app);
    render_input_bar(frame, chunks[2], app);
    render_status_bar(frame, chunks[3], app);

    render_overlays(frame, area, app);
}

pub fn render_header(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let title = format!(" PrimusDB v{} ", VERSION);
    let conn_status = match &app.connected_url {
        Some(url) => format!(" Connected: {} ", url),
        None => " Disconnected ".to_string(),
    };
    let status_style = if app.connected() {
        Style::new().fg(Color::Green)
    } else {
        Style::new().fg(Color::Red)
    };
    let text = Line::from(vec![
        Span::styled(
            title,
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::raw("\u{2502}"),
        Span::styled(conn_status, status_style),
    ]);
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::new().fg(Color::DarkGray));
    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .style(Style::new().bg(Color::Black)),
        area,
    );
}

pub fn render_main_area(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(0)])
        .split(area);

    render_sidebar(frame, chunks[0], app);
    render_content(frame, chunks[1], app);
}

pub fn render_sidebar(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let items: Vec<ListItem> = NAV_SECTIONS
        .iter()
        .map(|section| {
            let name = section.name();
            let is_active = *section == app.current_section;
            let prefix = if is_active { "\u{25b6} " } else { "  " };
            let style = if is_active {
                Style::new()
                    .fg(p.selection)
                    .bg(p.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(p.text_dim)
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}", prefix, name),
                style,
            )))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(p.border))
        .title(" Navigation ")
        .title_style(Style::new().fg(p.primary).add_modifier(Modifier::BOLD));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().bg(p.highlight_bg).add_modifier(Modifier::BOLD));

    frame.render_widget(list, area);
}

fn render_empty_state(frame: &mut Frame, area: Rect, title: &str, message: &str, hints: &[&str]) {
    let p = resolve_palette("default");
    let mut lines = vec![
        Line::from(Span::styled(
            format!(" {} ", title),
            Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", message),
            Style::new().fg(p.text_dim),
        )),
        Line::from(""),
    ];
    for hint in hints {
        lines.push(Line::from(Span::styled(
            format!("  \u{2192} {}", hint),
            Style::new().fg(Color::DarkGray),
        )));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(p.border))
        .title(format!(" {} ", title))
        .title_style(Style::new().fg(p.primary).add_modifier(Modifier::BOLD));
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_action_bar(frame: &mut Frame, area: Rect, section: NavSection) {
    let hints = section_hints(&section);
    if hints.is_empty() {
        return;
    }
    let hint_str: String = hints
        .iter()
        .map(|(key, desc)| format!("[{}] {} ", key, desc))
        .collect::<Vec<_>>()
        .join("\u{2502} ");
    let text = Line::from(vec![
        Span::styled(
            " Actions ",
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2502} ", Style::new().fg(Color::DarkGray)),
        Span::styled(hint_str, Style::new().fg(Color::White)),
    ]);
    frame.render_widget(
        Paragraph::new(text).style(Style::new().bg(Color::Black)),
        area,
    );
}

pub fn render_content(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    if app.loading {
        render_loading(frame, area, &app.loading_message);
        return;
    }
    if !app.connected()
        && app.current_section != NavSection::Help
        && app.current_section != NavSection::Settings
        && app.current_section != NavSection::FileBrowser
    {
        render_empty_state(
            frame,
            area,
            "Not Connected",
            "Connect to a PrimusDB server to use this section.",
            &[
                "Press : and type :connect <url> (e.g. :connect http://localhost:8080)",
                "Run 'primusdb tui --server http://localhost:8080' to auto-connect",
                "Run 'primusdb server start' first if no server is running",
                "Press Tab to navigate sections | ? for help | e for event log",
            ],
        );
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);
    render_action_bar(frame, chunks[0], app.current_section);
    sections::render_section(frame, chunks[1], app);
}

pub fn render_input_bar(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let (title, input, cursor_offset) = if app.current_section == NavSection::ConfigurationStudio {
        use crate::cli::tui::app::ConfigStudioMode;
        let config_title = match app.config_mode {
            ConfigStudioMode::Edit => " Config Value (JSON) ",
            ConfigStudioMode::NewEntry => " New Config (key=value) ",
            ConfigStudioMode::CreateSnapshot => " Snapshot Name ",
            ConfigStudioMode::ImportExport => " Import Bundle (JSON) ",
            _ => " Config ",
        };
        (
            config_title,
            if app.config_input.is_empty() {
                Text::from(Line::from(Span::styled(
                    "Type input, then press Enter",
                    Style::new().fg(Color::DarkGray),
                )))
            } else {
                Text::from(Line::from(Span::styled(
                    &app.config_input,
                    Style::new().fg(Color::Yellow),
                )))
            },
            app.config_input.len(),
        )
    } else if app.migration_wizard_active && (app.migration_step == 2 || app.migration_step == 3) {
        let label = if app.migration_step == 2 {
            " Migration URL "
        } else {
            " Namespace "
        };
        let text = if app.command_input.is_empty() {
            Text::from(Line::from(Span::styled(
                if app.migration_step == 2 {
                    "Enter source URL (e.g. mysql://user:pass@host/db)"
                } else {
                    "Enter target namespace"
                },
                Style::new().fg(Color::DarkGray),
            )))
        } else {
            Text::from(Line::from(Span::styled(
                &app.command_input,
                Style::new().fg(Color::Yellow),
            )))
        };
        (label, text, app.command_input.len())
    } else if app.show_command_palette {
        (
            " Command ",
            if app.command_input.is_empty() {
                Text::from(Line::from(Span::styled(
                    "Type a command (:help, :quit, :refresh, :connect <url>)",
                    Style::new().fg(Color::DarkGray),
                )))
            } else {
                Text::from(Line::from(Span::styled(
                    &app.command_input,
                    Style::new().fg(Color::Yellow),
                )))
            },
            app.command_input.len(),
        )
    } else {
        (
            " Query ",
            if app.query_input.is_empty() {
                Text::from(Line::from(Span::styled(
                    "Type SQL and press Enter to execute...",
                    Style::new().fg(Color::DarkGray),
                )))
            } else {
                Text::from(Line::from(Span::styled(
                    &app.query_input,
                    Style::new().fg(Color::White),
                )))
            },
            app.query_input.len(),
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(title)
        .title_style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let input_paragraph = Paragraph::new(input)
        .block(block)
        .style(Style::new().bg(Color::Black));

    frame.render_widget(input_paragraph, area);

    let x = (SIDEBAR_WIDTH + 3 + cursor_offset as u16).min(area.right().saturating_sub(2));
    frame.set_cursor(x, area.y + 1);
}

fn status_color(app: &TuiApp) -> Color {
    if app.connected() && app.error_message.is_none() {
        Color::Green
    } else {
        Color::Red
    }
}

fn section_hints(section: &NavSection) -> Vec<(&str, &str)> {
    match section {
        NavSection::Dashboard => vec![
            ("r", "Refresh"),
            ("Enter", "Details"),
            ("Tab", "Next"),
            ("?", "Help"),
            (":", "Palette"),
        ],
        NavSection::QueryConsole => vec![
            ("\u{2191}\u{2193}", "History"),
            ("H", "History Panel"),
            ("PgUp/Dn", "Scroll"),
            ("E", "Explain"),
            ("Ctrl+L", "Clear"),
            ("Enter", "Run"),
        ],
        NavSection::DatabasesEngines => vec![
            ("r", "Refresh"),
            ("Enter", "Inspect"),
            ("n", "New DB"),
            ("Tab", "Next"),
        ],
        NavSection::Namespaces => vec![
            ("r", "Refresh"),
            ("Enter", "Inspect"),
            ("n", "New"),
            ("d", "Delete"),
            ("u", "Use/Switch"),
        ],
        NavSection::Cluster => vec![
            ("\u{2191}\u{2193}", "Select"),
            ("s", "Start"),
            ("t", "Stop"),
            ("R", "Restart"),
            ("j", "Join"),
            ("l", "Leave"),
            ("m", "Maintenance"),
            ("d", "Remove"),
            ("r", "Refresh"),
        ],
        NavSection::Federation => vec![
            ("c", "Add Cluster"),
            ("r", "Remove"),
            ("d", "Create Domain"),
            ("x", "Delete Domain"),
            ("Tab", "Next"),
        ],
        NavSection::Governor => vec![
            ("s", "Set Policy"),
            ("d", "Delete Policy"),
            ("r", "Refresh"),
            ("Tab", "Next"),
        ],
        NavSection::BackupRestore => vec![
            ("Ctrl+B", "Create Backup"),
            ("Ctrl+R", "Restore"),
            ("v", "Verify"),
            ("r", "Refresh"),
        ],
        NavSection::MetricsLogs => vec![
            ("r", "Refresh"),
            ("1/2/3", "View"),
            ("l", "Level"),
            ("m", "Module"),
        ],
        NavSection::ConfigurationStudio => vec![
            ("e", "Edit"),
            ("n", "New"),
            ("s", "Snapshots"),
            ("x", "Export"),
            ("Esc", "Back"),
        ],
        NavSection::TableExplorer => vec![
            ("\u{2191}\u{2193}", "Row"),
            ("Enter", "Select"),
            ("i", "Insert"),
            ("d", "Delete"),
            ("a", "Analyze"),
            ("n/p", "Page"),
            ("r", "Refresh"),
            ("Esc", "Back"),
        ],
        NavSection::ReportBuilder => vec![
            ("n", "New Report"),
            ("e", "Edit"),
            ("Enter", "Execute"),
            ("d", "Delete"),
            ("Esc", "Back"),
        ],
        NavSection::Notebook => vec![
            ("n", "New Notebook"),
            ("e", "Edit Cell"),
            ("Enter", "Execute"),
            ("Esc", "Back"),
        ],
        NavSection::RAGWorkspace => vec![
            ("Enter", "Search"),
            ("+/-", "Top-K"),
            ("n", "Create"),
            ("d", "Delete"),
            ("r", "Refresh"),
            ("Esc", "Back"),
        ],
        NavSection::SecurityCenter => vec![
            ("\u{2191}\u{2193}", "Select"),
            ("Enter", "Detail"),
            ("u/r/p", "Tab"),
            ("n", "Create"),
            ("d", "Delete"),
            ("a", "Assign Role"),
        ],
        NavSection::DocumentWorkspace => vec![
            ("\u{2191}\u{2193}", "Select"),
            ("e", "Edit"),
            ("c", "Create"),
            ("v", "Validate"),
        ],
        NavSection::IntegratedTerminal => {
            vec![
                ("Enter", "Exec"),
                ("\u{2191}\u{2193}", "History"),
                ("Tab", "Complete"),
            ]
        }
        NavSection::Monitoring => vec![("o/a/p/r/s", "Mode"), ("r", "Refresh")],
        NavSection::Settings => vec![
            ("e", "Endpoint"),
            ("t", "Token"),
            ("i", "Interval"),
            ("h", "Theme"),
            ("s", "Safe"),
            ("m", "Mouse"),
            ("r", "Refresh"),
            ("d", "Doctor"),
        ],
        NavSection::FileBrowser => vec![
            ("\u{2191}\u{2193}", "Select"),
            ("Enter", "Open"),
            ("Esc", "Back"),
            ("h", "Home"),
            ("r", "Refresh"),
            ("d", "Delete"),
        ],
        NavSection::Help => vec![("Esc", "Close"), ("Tab", "Next")],
    }
}

pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let conn_indicator = if app.connected() {
        "\u{25cf}"
    } else {
        "\u{25cb}"
    };
    let conn_color = status_color(app);
    let conn_url = app.connected_url.as_deref().unwrap_or("disconnected");
    let version = VERSION;
    let ns = app.active_namespace.as_deref().unwrap_or("-");
    let db = app.selected_database.as_deref().unwrap_or("-");
    let section = app.current_section.name();
    let role = app.server_role.as_deref().unwrap_or("?");
    let cluster_state = app
        .cluster_status
        .as_ref()
        .and_then(|v| v.get("status").and_then(|s| s.as_str()))
        .unwrap_or("?");

    let role_label = if app.connected() {
        format!(" {} ", role)
    } else {
        String::new()
    };

    let status_line = Line::from(vec![
        Span::styled(format!(" {} ", conn_indicator), Style::new().fg(conn_color)),
        Span::styled(
            role_label,
            Style::new().fg(p.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(conn_url.to_string(), Style::new().fg(p.text)),
        Span::styled(" | ", Style::new().fg(p.text_dim)),
        Span::styled(format!("v{}", version), Style::new().fg(p.primary)),
        Span::styled(" | ", Style::new().fg(p.text_dim)),
        Span::styled(format!("ns:{}", ns), Style::new().fg(p.secondary)),
        Span::styled(" | ", Style::new().fg(p.text_dim)),
        Span::styled(format!("db:{}", db), Style::new().fg(p.success)),
        Span::styled(" | ", Style::new().fg(p.text_dim)),
        Span::styled(
            format!("cluster:{}", cluster_state),
            Style::new().fg(p.accent),
        ),
        Span::styled(" | ", Style::new().fg(p.text_dim)),
        Span::styled(
            section,
            Style::new().fg(p.text).add_modifier(Modifier::BOLD),
        ),
    ]);

    let hints = section_hints(&app.current_section);
    let hint_str: String = hints
        .iter()
        .map(|(key, desc)| format!("[{}] {} ", key, desc))
        .collect::<Vec<_>>()
        .join("\u{2502} ");

    if app.show_event_log {
        let event_lines: Vec<Line> = app
            .event_log
            .iter()
            .rev()
            .take((area.height as usize).saturating_sub(2))
            .map(|msg| {
                let color = if msg.starts_with("Error")
                    || msg.contains("fail")
                    || msg.contains("error")
                    || msg.contains("Could not")
                {
                    Color::Red
                } else if msg.starts_with("Connected")
                    || msg.contains("success")
                    || msg.contains("healthy")
                    || msg.starts_with("Discovered")
                {
                    Color::Green
                } else if msg.starts_with("Executing")
                    || msg.contains("query")
                    || msg.contains("fetch")
                {
                    Color::Yellow
                } else if msg.starts_with("Refreshing") || msg.starts_with("Connecting") {
                    Color::Cyan
                } else {
                    Color::DarkGray
                };
                Line::from(Span::styled(format!(" {}", msg), Style::new().fg(color)))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(p.border))
            .title(" Event Log ")
            .title_style(Style::new().fg(p.primary).add_modifier(Modifier::BOLD));

        frame.render_widget(
            Paragraph::new(Text::from(event_lines))
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    } else {
        let latest_event = app.event_log.last().map(|s| s.as_str()).unwrap_or("Ready");
        let event_color = if latest_event.starts_with("Error")
            || latest_event.contains("fail")
            || latest_event.contains("error")
            || latest_event.contains("Could not")
        {
            Color::Red
        } else if latest_event.starts_with("Connected")
            || latest_event.contains("success")
            || latest_event.contains("healthy")
        {
            Color::Green
        } else {
            Color::DarkGray
        };

        let event_line = Line::from(vec![
            Span::styled(" ", Style::new().fg(Color::DarkGray)),
            Span::styled(latest_event, Style::new().fg(event_color)),
        ]);

        let text = Text::from(vec![
            status_line,
            Line::from(Span::styled(
                hint_str,
                Style::new().fg(p.text_dim).add_modifier(Modifier::BOLD),
            )),
            event_line,
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::DarkGray))
            .title(" Status ")
            .title_style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD));

        frame.render_widget(
            Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
            area,
        );
    }
}

pub fn render_clusters(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Clusters ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(Color::Red),
        )));
    } else {
        lines.push(spanned_line(&[
            ("Connected to: ", Color::Gray, false),
            (
                app.connected_url.as_deref().unwrap_or("-"),
                Color::Cyan,
                false,
            ),
        ]));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub fn render_migrations(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Migrations ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if app.migration_wizard_active {
        render_migration_wizard(&mut lines, app);
    } else {
        let sources = [
            ("MySQL", "mysql"),
            ("PostgreSQL", "tokio-postgres"),
            ("MongoDB", "mongodb"),
            ("CouchDB", "couchdb"),
        ];

        lines.push(Line::from(Span::styled(
            "Supported Sources:",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for (name, dep) in &sources {
            lines.push(spanned_line(&[
                ("  \u{2022} ", Color::DarkGray, false),
                (name, Color::Cyan, true),
                (" \u{2014} requires `", Color::Gray, false),
                (dep, Color::White, false),
                ("` crate", Color::Gray, false),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Commands:",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        lines.push(spanned_line(&[
            ("  primusdb migrate inspect-source ", Color::Gray, false),
            ("<source> <url>", Color::Cyan, false),
        ]));
        lines.push(spanned_line(&[
            ("  primusdb migrate plan ", Color::Gray, false),
            ("<source> <url> <target>", Color::Cyan, false),
        ]));
        lines.push(spanned_line(&[
            ("  primusdb migrate import ", Color::Gray, false),
            ("<source> <url> <target>", Color::Cyan, false),
        ]));
        lines.push(Line::from(""));

        lines.push(spanned_line(&[
            ("  Press ", Color::Gray, false),
            ("Ctrl+M", Color::Cyan, true),
            (" to open the migration wizard", Color::Gray, false),
        ]));
        lines.push(Line::from(""));

        if !app.connected() {
            lines.push(Line::from(Span::styled(
                "Connect to a PrimusDB instance to perform migrations.",
                Style::new().fg(Color::Gray),
            )));
        }
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_migration_wizard(lines: &mut Vec<Line>, app: &TuiApp) {
    match app.migration_step {
        0 => {
            lines.push(Line::from(Span::styled(
                " Migration Wizard ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(spanned_line(&[(
                "  This wizard will guide you through importing data",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[(
                "  from an external database into PrimusDB.",
                Color::Gray,
                false,
            )]));
            lines.push(Line::from(""));
            lines.push(spanned_line(&[("  Steps:", Color::Cyan, true)]));
            lines.push(spanned_line(&[(
                "    1. Select source type (MySQL, PostgreSQL, MongoDB, CouchDB)",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[(
                "    2. Enter the source connection URL",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[(
                "    3. Test connection to the source",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[(
                "    4. Enter the target namespace",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[(
                "    5. Select migration mode",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[(
                "    6. Inspect source objects",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[(
                "    7. Select objects to migrate",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[(
                "    8. Preview migration plan",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[(
                "    9. Dry-run validation",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[(
                "   10. Review and confirm",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[(
                "   11. Watch progress",
                Color::Gray,
                false,
            )]));
            lines.push(spanned_line(&[("   12. Save report", Color::Gray, false)]));
            lines.push(Line::from(""));
            lines.push(spanned_line(&[
                ("  Press ", Color::Gray, false),
                ("Enter", Color::Cyan, true),
                (" to begin, or ", Color::Gray, false),
                ("Esc", Color::Cyan, true),
                (" to cancel", Color::Gray, false),
            ]));
        }
        1 => {
            lines.push(Line::from(Span::styled(
                " Step 1/12: Select Source ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Choose the source database type:",
                Style::new().fg(Color::Gray),
            )));
            lines.push(Line::from(""));
            let selected = app.migration_source.as_str();
            for (i, name) in ["MySQL", "PostgreSQL", "MongoDB", "CouchDB"]
                .iter()
                .enumerate()
            {
                let is_sel = (i + 1).to_string() == selected || *name == app.migration_source;
                let marker = if is_sel { ">" } else { " " };
                let item = format!("  {} [{}] {}", marker, i + 1, name);
                let c = if is_sel { Color::Cyan } else { Color::White };
                lines.push(Line::from(Span::styled(item, Style::new().fg(c))));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Press 1-4 to select, Esc to cancel",
                Style::new().fg(Color::DarkGray),
            )));
        }
        _ => {
            lines.push(Line::from(Span::styled(
                " Migration Wizard (step details in full version) ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  Step: {}", app.migration_step),
                Style::new().fg(Color::Cyan),
            )));
            lines.push(Line::from(Span::styled(
                "  Use Ctrl+M to toggle the migration wizard.",
                Style::new().fg(Color::Gray),
            )));
        }
    }
}

pub fn render_logs(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Logs ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if let Some(ref data) = app.logs_data {
        for line_str in data.lines() {
            lines.push(Line::from(Span::styled(
                format!(" {}", line_str),
                Style::new().fg(Color::White),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No logs data. Press r to fetch (runs journalctl -u primusdb -n 50).",
            Style::new().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[(
            "  Requires systemd/journald.",
            Color::DarkGray,
            false,
        )]));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

// ── Overlay helpers ──────────────────────────────────────────────────

pub fn render_overlays(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    if app.show_command_palette {
        render_command_palette_overlay(frame, area, app);
    }
    if app.confirm_action != ConfirmAction::None {
        render_confirm_dialog(frame, area, app);
    }
    if app.onboarding_mode {
        render_onboarding_overlay(frame, area, app);
    }
    if app.show_contextual_help {
        render_contextual_help_popup(frame, area, app);
    }
    if app.show_search {
        render_search_overlay(frame, area, app);
    }
    if app.show_session_switcher {
        render_session_switcher_overlay(frame, area, app);
    }
}

fn render_search_overlay(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let popup_height = 16.min(area.height.saturating_sub(4));
    let popup_width = 55.min(area.width.saturating_sub(4));

    let popup_area = ratatui::layout::Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + 2,
        width: popup_width,
        height: popup_height,
    };

    let mut lines: Vec<Line> = Vec::new();

    let scope_name = match app.search_scope {
        SearchScope::All => "Everywhere",
        SearchScope::Commands => "Commands",
        SearchScope::Objects => "Objects",
        SearchScope::Sections => "Sections",
        SearchScope::Capabilities => "Capabilities",
        SearchScope::Files => "Files",
    };

    lines.push(spanned_line(&[
        (" Search: ", Color::Cyan, true),
        (&app.search_input, Color::White, false),
    ]));
    lines.push(Line::from(Span::styled(
        format!(" Scope: {}", scope_name),
        Style::new().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    let results = if app.search_input.is_empty() {
        vec!["Type to search across sessions, objects, commands...".to_string()]
    } else if app.search_results.is_empty() {
        vec![format!("No results for '{}'", app.search_input)]
    } else {
        app.search_results.clone()
    };

    for (i, r) in results.iter().enumerate() {
        if i >= 8 {
            break;
        }
        let style = if i == app.search_selection {
            Style::new().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::new().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(r.clone(), style)));
    }

    lines.push(Line::from(""));
    lines.push(spanned_line(&[
        (" Tab:scope ", Color::DarkGray, false),
        (" Enter:select ", Color::Green, false),
        (" Esc:close ", Color::DarkGray, false),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan))
        .title(" Search Everywhere ")
        .title_style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        popup_area,
    );
}

fn render_session_switcher_overlay(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let popup_height = (app.sessions.len() as u16 + 4).min(area.height.saturating_sub(4));
    let popup_width = 45.min(area.width.saturating_sub(4));

    let popup_area = ratatui::layout::Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + 2,
        width: popup_width,
        height: popup_height,
    };

    let mut lines: Vec<Line> = Vec::new();
    for (i, session) in app.sessions.iter().enumerate() {
        let connected = if session.connected {
            "\u{25cf}"
        } else {
            "\u{25cb}"
        };
        let active = if i == app.active_session {
            "\u{25b8}"
        } else {
            " "
        };
        let url = if session.url.is_empty() {
            "(no connection)"
        } else {
            &session.url
        };
        lines.push(Line::from(Span::styled(
            format!(" {} {} {} {}", active, connected, url, session.id),
            Style::new().fg(if i == app.active_session {
                Color::Cyan
            } else {
                Color::Gray
            }),
        )));
    }
    lines.push(Line::from(""));
    lines.push(spanned_line(&[
        (" \u{2191}\u{2193} select ", Color::DarkGray, false),
        (" Enter:switch ", Color::Green, false),
        (" n:new ", Color::Cyan, false),
        (" Esc:close ", Color::DarkGray, false),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Magenta))
        .title(" Sessions ")
        .title_style(Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD));

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        popup_area,
    );
}

fn render_command_palette_overlay(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let filtered = app.filter_commands();
    let max_visible = 10;
    let total = filtered.len();

    let popup_height = (total.min(max_visible) as u16 + 4).min(area.height.saturating_sub(4));
    let popup_width = 50.min(area.width.saturating_sub(4));

    let popup_area = ratatui::layout::Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + 2,
        width: popup_width,
        height: popup_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan))
        .title(" Command Palette ")
        .title_style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.command_palette_selection {
                Style::new().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::new().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(item.as_str(), style)))
        })
        .collect();

    let list = ratatui::widgets::List::new(items)
        .block(block)
        .highlight_style(Style::new().bg(Color::Cyan));

    frame.render_widget(list, popup_area);
}

fn render_confirm_dialog(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let msg_lines: Vec<&str> = app.confirm_message.lines().collect();
    let height = msg_lines.len() as u16 + 5;
    let width = 50.min(area.width.saturating_sub(4));

    let popup_area = ratatui::layout::Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    let lines = vec![
        Line::from(Span::styled(
            app.confirm_message.clone(),
            Style::new().fg(Color::White),
        )),
        Line::from(""),
        spanned_line(&[
            ("  [", Color::DarkGray, false),
            ("Y", Color::Green, true),
            ("] Yes  [", Color::DarkGray, false),
            ("N", Color::Red, true),
            ("] No  [", Color::DarkGray, false),
            ("Esc", Color::Cyan, true),
            ("] Cancel", Color::DarkGray, false),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Yellow))
        .title(" Confirm ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .alignment(ratatui::layout::Alignment::Center),
        popup_area,
    );
}

fn render_onboarding_overlay(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let height = 14;
    let width = 50.min(area.width.saturating_sub(4));

    let popup_area = ratatui::layout::Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        " Welcome to PrimusDB TUI ",
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if app.onboarding_step == 1 {
        lines.push(Line::from(Span::styled(
            "  Choose how to connect:",
            Style::new().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  [1] Connect to local server (http://localhost:8080)",
            Style::new().fg(Color::White),
        )));
        lines.push(Line::from(Span::styled(
            "  [2] Enter a custom endpoint URL",
            Style::new().fg(Color::White),
        )));
        lines.push(Line::from(Span::styled(
            "  [3] Skip \u{2014} browse offline",
            Style::new().fg(Color::White),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Press 1, 2, or 3 to continue",
            Style::new().fg(Color::DarkGray),
        )));
    } else if app.onboarding_step == 2 {
        let url_display = if app.command_input.is_empty() {
            "(type endpoint URL below)".to_string()
        } else {
            app.command_input.clone()
        };
        lines.push(Line::from(Span::styled(
            "  Enter server endpoint URL:",
            Style::new().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", url_display),
            Style::new().fg(if app.command_input.is_empty() {
                Color::DarkGray
            } else {
                Color::White
            }),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Press Enter to connect, Esc to skip",
            Style::new().fg(Color::DarkGray),
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan))
        .title(" Onboarding ")
        .title_style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .alignment(ratatui::layout::Alignment::Center),
        popup_area,
    );
}

fn render_contextual_help_popup(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let help_text = app.contextual_help_text();
    let lines: Vec<Line> = help_text
        .lines()
        .map(|s| Line::from(Span::styled(s.to_string(), Style::new().fg(Color::White))))
        .collect();
    let popup_height = lines.len() as u16 + 4;
    let popup_width = 55.min(area.width.saturating_sub(4));

    let popup_area = ratatui::layout::Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + area.height.saturating_sub(popup_height + 2),
        width: popup_width,
        height: popup_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan))
        .title(" Context Help ")
        .title_style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        popup_area,
    );
}
