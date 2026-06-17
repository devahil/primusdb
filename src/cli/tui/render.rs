use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::cli::tui::app::{
    TuiApp, HEADER_HEIGHT, INPUT_HEIGHT, NAV_SECTIONS, SIDEBAR_WIDTH, STATUS_HEIGHT, VERSION,
};
use crate::cli::tui::sections;
use crate::cli::tui::widgets::{
    render_json_block, render_loading, render_progress_bar, spanned_line,
};

pub fn render(frame: &mut Frame, app: &mut TuiApp) {
    let area = frame.size();
    if area.width < 60 || area.height < 20 {
        let text = Text::from(Line::from(Span::styled(
            "Terminal too small — resize to at least 60x20",
            Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("PrimusDB TUI")),
            area,
        );
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
        Span::raw("│"),
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
    let items: Vec<ListItem> = NAV_SECTIONS
        .iter()
        .map(|section| {
            let name = section.name();
            let is_active = *section == app.current_section;
            let prefix = if is_active { "▶ " } else { "  " };
            let style = if is_active {
                Style::new()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(Color::Gray)
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}", prefix, name),
                style,
            )))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Navigation ")
        .title_style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().bg(Color::Blue).add_modifier(Modifier::BOLD));

    frame.render_widget(list, area);
}

pub fn render_content(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    if app.loading {
        render_loading(frame, area, &app.loading_message);
        return;
    }
    sections::render_section(frame, area, app);
}

pub fn render_input_bar(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let (title, input, cursor_offset) =
        if app.migration_wizard_active && (app.migration_step == 2 || app.migration_step == 3) {
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

pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Events ")
        .title_style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let lines: Vec<Line> = app
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
            } else if msg.starts_with("Executing") || msg.contains("query") || msg.contains("fetch")
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

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

// ── Section rendering functions ──────────────────────────────────────

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
        lines.push(Line::from(""));

        let leader_count = app
            .cluster_nodes
            .as_ref()
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|n| n.get("role").and_then(|r| r.as_str()) == Some("leader"))
                    .count()
            })
            .unwrap_or(0);
        let total_nodes = app
            .cluster_nodes
            .as_ref()
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        let healthy_nodes = app
            .cluster_nodes
            .as_ref()
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|n| {
                        let s = n
                            .get("status")
                            .or(n.get("health"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        matches!(s, "healthy" | "ok" | "online" | "active" | "leader")
                    })
                    .count()
            })
            .unwrap_or(0);

        lines.push(Line::from(Span::styled(
            format!(
                "  Topology: {} nodes ({} leader, {} healthy)",
                total_nodes, leader_count, healthy_nodes
            ),
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));

        if total_nodes > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  ╔══════════════════════════════╗",
                Style::new().fg(Color::DarkGray),
            )));
            for node in app
                .cluster_nodes
                .as_ref()
                .and_then(|v| v.as_array())
                .unwrap_or(&vec![])
            {
                let id = node
                    .get("id")
                    .or(node.get("node_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let role = node
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("follower");
                let status = node
                    .get("status")
                    .or(node.get("health"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let addr = node
                    .get("address")
                    .or(node.get("addr"))
                    .or(node.get("endpoint"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let lag = node
                    .get("replication_lag")
                    .or(node.get("lag"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let status_color = match status {
                    "healthy" | "ok" | "online" | "active" | "leader" => Color::Green,
                    "warning" | "degraded" => Color::Yellow,
                    "error" | "down" | "offline" => Color::Red,
                    _ => Color::White,
                };

                let icon = match role {
                    "leader" | "primary" => "◆",
                    "follower" | "replica" => "◇",
                    "candidate" => "◈",
                    _ => "○",
                };

                let role_arrow = if role == "leader" || role == "primary" {
                    format!("  {} LDR ", icon)
                } else if role == "follower" || role == "replica" {
                    format!("  {} FOL ←", icon)
                } else {
                    format!(
                        "  {} {} ",
                        icon,
                        role.chars().take(3).collect::<String>().to_uppercase()
                    )
                };

                lines.push(Line::from(vec![
                    Span::styled(role_arrow, Style::new().fg(status_color)),
                    Span::styled(format!(" {}", id), Style::new().fg(Color::White)),
                    Span::styled(format!("  {}", addr), Style::new().fg(Color::DarkGray)),
                    Span::styled(
                        format!("  [{}]", status),
                        Style::new().fg(status_color).add_modifier(Modifier::BOLD),
                    ),
                ]));

                if lag > 0 && role != "leader" {
                    lines.push(Line::from(Span::styled(
                        format!("           lag: {}ms", lag),
                        Style::new().fg(Color::Yellow),
                    )));
                }

                if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        let cid = child.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                        lines.push(Line::from(Span::styled(
                            format!("             └─ {}", cid),
                            Style::new().fg(Color::DarkGray),
                        )));
                    }
                }
            }
            lines.push(Line::from(Span::styled(
                "  ╚══════════════════════════════╝",
                Style::new().fg(Color::DarkGray),
            )));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            "Cluster Status:",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        render_json_block(
            &mut lines,
            app.cluster_status.as_ref(),
            "No cluster status data. Press r to refresh.",
        );

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Cluster Nodes:",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));

        if let Some(ref nodes_val) = app.cluster_nodes {
            if let Some(nodes_arr) = nodes_val.as_array() {
                if nodes_arr.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  No nodes found.",
                        Style::new().fg(Color::Gray),
                    )));
                } else {
                    lines.push(spanned_line(&[
                        ("  ID", Color::Cyan, true),
                        ("  Role", Color::Cyan, true),
                        ("  Status", Color::Cyan, true),
                        ("  Address", Color::Cyan, true),
                    ]));
                    lines.push(Line::from(Span::styled(
                        "  ───────────────────────────────────────────",
                        Style::new().fg(Color::DarkGray),
                    )));

                    for node in nodes_arr {
                        let id = node
                            .get("id")
                            .or(node.get("node_id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("-");
                        let role = node.get("role").and_then(|v| v.as_str()).unwrap_or("-");
                        let status = node
                            .get("status")
                            .or(node.get("health"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("-");
                        let addr = node
                            .get("address")
                            .or(node.get("addr"))
                            .or(node.get("endpoint"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("-");

                        let status_color = match status {
                            "healthy" | "ok" | "online" | "active" | "leader" => Color::Green,
                            "warning" | "degraded" => Color::Yellow,
                            "error" | "down" | "offline" => Color::Red,
                            _ => Color::White,
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!("  {}", id), Style::new().fg(Color::White)),
                            Span::styled(format!("  {}", role), Style::new().fg(Color::DarkGray)),
                            Span::styled(
                                format!("  {}", status),
                                Style::new().fg(status_color).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(format!("  {}", addr), Style::new().fg(Color::Cyan)),
                        ]));
                    }
                }
            } else if let Some(_nodes_obj) = nodes_val.as_object() {
                let pretty = serde_json::to_string_pretty(nodes_val).unwrap_or_default();
                for line_str in pretty.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line_str),
                        Style::new().fg(Color::White),
                    )));
                }
            } else {
                lines.push(Line::from(Span::styled(
                    "  No cluster nodes data. Press r to refresh.",
                    Style::new().fg(Color::Gray),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "  No cluster nodes data. Press r to refresh.",
                Style::new().fg(Color::Gray),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Cluster Health:",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        render_json_block(
            &mut lines,
            app.cluster_health.as_ref(),
            "No cluster health data. Press r to refresh.",
        );

        if !app.cluster_events.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Recent Events:",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            for ev in app.cluster_events.iter().rev().take(10) {
                lines.push(Line::from(Span::styled(
                    format!("  • {}", ev),
                    Style::new().fg(Color::White),
                )));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub fn render_queries(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Queries ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance to run queries.",
            Style::new().fg(Color::Red),
        )));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  Go to ", Color::Gray, false),
            ("Instances", Color::Cyan, true),
            (" to connect, or use ", Color::Gray, false),
            ("Dashboard", Color::Cyan, true),
            (" to see options.", Color::Gray, false),
        ]));
    } else {
        let conn = app.connected_url.as_deref().unwrap_or("-");
        lines.push(spanned_line(&[
            ("Connected: ", Color::Green, false),
            (conn, Color::Cyan, false),
        ]));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            (
                "  Type your query in the input bar below and press ",
                Color::Gray,
                false,
            ),
            ("Enter", Color::White, true),
        ]));
        lines.push(Line::from(""));

        if app.query_results.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No results yet.",
                Style::new().fg(Color::Gray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "Results:",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            let max_visible = (area.height as usize).saturating_sub(8);
            let total = app.query_results.len();
            let scroll = app.query_scroll.min(total.saturating_sub(max_visible));
            let visible: Vec<&String> = app
                .query_results
                .iter()
                .skip(scroll)
                .take(max_visible)
                .collect();

            for result in &visible {
                for line_str in result.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line_str),
                        Style::new().fg(Color::White),
                    )));
                }
            }

            let showing = format!(
                "  Showing {}-{} of {}  (PgUp/PgDn to scroll)",
                scroll + 1,
                (scroll + visible.len()).min(total),
                total
            );
            lines.push(Line::from(Span::styled(
                showing,
                Style::new().fg(Color::DarkGray),
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

pub fn render_backups(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Backups ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if app.backup_in_progress {
        lines.push(Line::from(Span::styled(
            "  Creating backup... (Ctrl+B pressed)",
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }

    if app.backups_data.is_empty() {
        lines.push(Line::from(Span::styled(
            "No backup files found in the backups/ directory.",
            Style::new().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  Backups are stored in ", Color::Gray, false),
            ("./backups/", Color::Cyan, false),
        ]));
        lines.push(spanned_line(&[
            ("  Create one with: ", Color::Gray, false),
            ("primusdb backup create", Color::Cyan, false),
        ]));
        lines.push(spanned_line(&[
            ("  Or press: ", Color::Gray, false),
            ("Ctrl+B", Color::Cyan, true),
        ]));
    } else {
        let count_str = format!("  {} backup(s) found:", app.backups_data.len());
        lines.push(Line::from(Span::styled(
            count_str,
            Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));

        if let Some(ref detail) = app.backups_detail {
            if let Some(backups_arr) = detail.get("backups").and_then(|b| b.as_array()) {
                if !backups_arr.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(spanned_line(&[
                        ("  ID", Color::Cyan, true),
                        ("  Date", Color::Cyan, true),
                        ("  Size", Color::Cyan, true),
                        ("  Engines", Color::Cyan, true),
                        ("  Status", Color::Cyan, true),
                    ]));
                    lines.push(Line::from(Span::styled(
                        "  ───────────────────────────────────────────────────────────────────",
                        Style::new().fg(Color::DarkGray),
                    )));
                    for backup in backups_arr {
                        let id = backup.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                        let created = backup
                            .get("created_at")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let size = backup
                            .get("size_bytes")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let engines = backup
                            .get("engines")
                            .and_then(|v| v.as_str())
                            .unwrap_or("-");
                        let status = backup
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let compression = backup
                            .get("compression")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let encrypted = backup
                            .get("encrypted")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        let size_str = if size > 1024 * 1024 * 1024 {
                            format!("{:.1}GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
                        } else if size > 1024 * 1024 {
                            format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
                        } else if size > 1024 {
                            format!("{:.1}KB", size as f64 / 1024.0)
                        } else {
                            format!("{}B", size)
                        };

                        let status_color = match status {
                            "completed" | "ok" => Color::Green,
                            "in_progress" | "running" => Color::Cyan,
                            "failed" | "error" => Color::Red,
                            "verified" => Color::Yellow,
                            _ => Color::White,
                        };

                        let mut extra = String::new();
                        if !compression.is_empty() && compression != "none" {
                            extra.push_str(&format!(" [{}]", compression));
                        }
                        if encrypted {
                            extra.push_str(" [enc]");
                        }
                        let engines_display = if engines.len() > 12 {
                            format!("{}...", &engines[..12])
                        } else {
                            engines.to_string()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!("  {}", id), Style::new().fg(Color::White)),
                            Span::styled(
                                format!("  {}", created),
                                Style::new().fg(Color::DarkGray),
                            ),
                            Span::styled(
                                format!("  {:>7}", size_str),
                                Style::new().fg(Color::Cyan),
                            ),
                            Span::styled(
                                format!("  {}", engines_display),
                                Style::new().fg(Color::Yellow),
                            ),
                            Span::styled(
                                format!("  {}", status),
                                Style::new().fg(status_color).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(extra, Style::new().fg(Color::DarkGray)),
                        ]));
                    }
                }
            }
        }

        if app.backups_detail.is_none() {
            lines.push(Line::from(""));
            lines.push(spanned_line(&[(
                "  Type   Size       Name",
                Color::Yellow,
                true,
            )]));
            lines.push(Line::from(Span::styled(
                "  ─────────────────────────────",
                Style::new().fg(Color::DarkGray),
            )));
            for entry in &app.backups_data {
                lines.push(Line::from(Span::styled(
                    format!("  {}", entry),
                    Style::new().fg(Color::White),
                )));
            }
        }
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
                ("  • ", Color::DarkGray, false),
                (name, Color::Cyan, true),
                (" — requires `", Color::Gray, false),
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
        2 => {
            lines.push(Line::from(Span::styled(
                " Step 2/12: Source URL ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Source type: ", Style::new().fg(Color::Gray)),
                Span::styled(
                    app.migration_source.clone(),
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Enter the connection URL:",
                Style::new().fg(Color::Gray),
            )));
            let url_display = if app.command_input.is_empty() {
                "(type URL below, then press Enter)".to_string()
            } else {
                app.command_input.clone()
            };
            lines.push(Line::from(Span::styled(
                url_display,
                Style::new().fg(if app.command_input.is_empty() {
                    Color::DarkGray
                } else {
                    Color::White
                }),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Esc to go back",
                Style::new().fg(Color::DarkGray),
            )));
        }
        3 => {
            lines.push(Line::from(Span::styled(
                " Step 3/12: Test Connection ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Source: ", Style::new().fg(Color::Gray)),
                Span::styled(
                    app.migration_source.clone(),
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  URL: ", Style::new().fg(Color::Gray)),
                Span::styled(app.migration_url.clone(), Style::new().fg(Color::Cyan)),
            ]));
            lines.push(Line::from(""));
            if app.migration_source_connected {
                lines.push(Line::from(Span::styled(
                    "  ✓ Connection successful!",
                    Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
                )));
                if !app.migration_status.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", app.migration_status),
                        Style::new().fg(Color::White),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  Press ", Color::Gray, false),
                    ("Enter", Color::Cyan, true),
                    (" to continue, or ", Color::Gray, false),
                    ("Esc", Color::Cyan, true),
                    (" to go back", Color::Gray, false),
                ]));
            } else if let Some(ref err) = app.migration_error {
                lines.push(Line::from(Span::styled(
                    format!("  ✗ Connection failed: {}", err),
                    Style::new().fg(Color::Red),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  Press ", Color::Gray, false),
                    ("Enter", Color::Cyan, true),
                    (" to retry, or ", Color::Gray, false),
                    ("Esc", Color::Cyan, true),
                    (" to go back", Color::Gray, false),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    "  Testing connection... (this may take a moment)",
                    Style::new().fg(Color::Cyan),
                )));
            }
        }
        4 => {
            lines.push(Line::from(Span::styled(
                " Step 4/12: Target Namespace ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Source: ", Style::new().fg(Color::Gray)),
                Span::styled(
                    app.migration_source.clone(),
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  URL: ", Style::new().fg(Color::Gray)),
                Span::styled(app.migration_url.clone(), Style::new().fg(Color::Cyan)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Enter the target namespace in PrimusDB:",
                Style::new().fg(Color::Gray),
            )));
            let ns_display = if app.command_input.is_empty() {
                "(type namespace below, then press Enter)".to_string()
            } else {
                app.command_input.clone()
            };
            lines.push(Line::from(Span::styled(
                ns_display,
                Style::new().fg(if app.command_input.is_empty() {
                    Color::DarkGray
                } else {
                    Color::White
                }),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Esc to go back",
                Style::new().fg(Color::DarkGray),
            )));
        }
        5 => {
            lines.push(Line::from(Span::styled(
                " Step 5/12: Migration Mode ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Choose the migration mode:",
                Style::new().fg(Color::Gray),
            )));
            lines.push(Line::from(""));
            let modes = [
                ("1", "copy", "Full copy — schema + data"),
                ("2", "schema-only", "Only schema/DDL"),
                ("3", "data-only", "Only data/DML"),
                ("4", "dry-run", "Validate without importing"),
            ];
            for (key, name, desc) in &modes {
                let is_sel = *name == app.migration_mode;
                let marker = if is_sel { ">" } else { " " };
                let item = format!("  {} [{}] {} — {}", marker, key, name, desc);
                let c = if is_sel { Color::Cyan } else { Color::White };
                lines.push(Line::from(Span::styled(item, Style::new().fg(c))));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Press 1-4 to select, Esc to go back",
                Style::new().fg(Color::DarkGray),
            )));
        }
        6 => {
            lines.push(Line::from(Span::styled(
                " Step 6/12: Inspect Source Objects ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            if app.migration_objects.is_empty() && app.migration_error.is_none() {
                lines.push(Line::from(Span::styled(
                    "  Inspecting source objects...",
                    Style::new().fg(Color::Cyan),
                )));
            } else if let Some(ref err) = app.migration_error {
                lines.push(Line::from(Span::styled(
                    format!("  ✗ Inspection failed: {}", err),
                    Style::new().fg(Color::Red),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  Press ", Color::Gray, false),
                    ("Enter", Color::Cyan, true),
                    (" to retry", Color::Gray, false),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("  Found {} object(s):", app.migration_objects.len()),
                    Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                for obj in &app.migration_objects {
                    lines.push(Line::from(Span::styled(
                        format!("  • {}", obj),
                        Style::new().fg(Color::White),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  Press ", Color::Gray, false),
                    ("Enter", Color::Cyan, true),
                    (" to continue, or ", Color::Gray, false),
                    ("Esc", Color::Cyan, true),
                    (" to go back", Color::Gray, false),
                ]));
            }
        }
        7 => {
            lines.push(Line::from(Span::styled(
                " Step 7/12: Select Objects ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Toggle objects to include (space to toggle, Enter to confirm):",
                Style::new().fg(Color::Gray),
            )));
            lines.push(Line::from(""));
            if app.migration_objects.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No objects available. Press Enter to skip.",
                    Style::new().fg(Color::DarkGray),
                )));
            } else {
                for (i, obj) in app.migration_objects.iter().enumerate() {
                    let selected = app
                        .migration_selected_objects
                        .get(i)
                        .copied()
                        .unwrap_or(true);
                    let check = if selected { "[x]" } else { "[ ]" };
                    lines.push(Line::from(Span::styled(
                        format!("  {} {}", check, obj),
                        Style::new().fg(if selected {
                            Color::White
                        } else {
                            Color::DarkGray
                        }),
                    )));
                }
            }
            lines.push(Line::from(""));
            lines.push(spanned_line(&[
                ("  Space", Color::Cyan, true),
                (" to toggle, ", Color::Gray, false),
                ("Enter", Color::Cyan, true),
                (" to confirm, ", Color::Gray, false),
                ("Esc", Color::Cyan, true),
                (" to go back", Color::Gray, false),
            ]));
        }
        8 => {
            lines.push(Line::from(Span::styled(
                " Step 8/12: Preview Migration Plan ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Source:     ", Style::new().fg(Color::Gray)),
                Span::styled(
                    app.migration_source.clone(),
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  URL:        ", Style::new().fg(Color::Gray)),
                Span::styled(app.migration_url.clone(), Style::new().fg(Color::White)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Namespace:  ", Style::new().fg(Color::Gray)),
                Span::styled(
                    app.migration_namespace.clone(),
                    Style::new().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Mode:       ", Style::new().fg(Color::Gray)),
                Span::styled(app.migration_mode.clone(), Style::new().fg(Color::White)),
            ]));
            let obj_count = app
                .migration_selected_objects
                .iter()
                .filter(|&&s| s)
                .count();
            lines.push(Line::from(vec![
                Span::styled("  Objects:    ", Style::new().fg(Color::Gray)),
                Span::styled(
                    format!("{} of {}", obj_count, app.migration_objects.len()),
                    Style::new().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(""));
            if !app.migration_plan.is_empty() {
                for line_str in app.migration_plan.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line_str),
                        Style::new().fg(Color::White),
                    )));
                }
            } else {
                lines.push(Line::from(Span::styled(
                    "  (plan preview generated automatically)",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Will migrate selected objects from source to PrimusDB.",
                    Style::new().fg(Color::Gray),
                )));
            }
            lines.push(Line::from(""));
            lines.push(spanned_line(&[
                ("  Press ", Color::Gray, false),
                ("Enter", Color::Cyan, true),
                (" to continue, or ", Color::Gray, false),
                ("Esc", Color::Cyan, true),
                (" to go back", Color::Gray, false),
            ]));
        }
        9 => {
            lines.push(Line::from(Span::styled(
                " Step 9/12: Dry-Run Validation ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            if app.migration_dry_run_result.is_some() {
                let result = app.migration_dry_run_result.as_deref().unwrap_or("");
                if result.contains("Error") || result.contains("FAILED") {
                    lines.push(Line::from(Span::styled(
                        "  ✗ Dry-run completed with issues:",
                        Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        "  ✓ Dry-run completed successfully!",
                        Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
                    )));
                }
                lines.push(Line::from(""));
                for line_str in result.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line_str),
                        Style::new().fg(Color::White),
                    )));
                }
                lines.push(Line::from(""));
                let mode = app.migration_mode.as_str();
                if mode != "dry-run" {
                    lines.push(spanned_line(&[
                        ("  Press ", Color::Gray, false),
                        ("Enter", Color::Cyan, true),
                        (" to proceed to confirm, or ", Color::Gray, false),
                        ("Esc", Color::Cyan, true),
                        (" to go back", Color::Gray, false),
                    ]));
                } else {
                    lines.push(spanned_line(&[
                        ("  Mode is ", Color::Gray, false),
                        ("dry-run", Color::Cyan, true),
                        (" — this was a validation only.", Color::Gray, false),
                    ]));
                }
            } else if let Some(ref err) = app.migration_error {
                lines.push(Line::from(Span::styled(
                    format!("  ✗ Dry-run failed: {}", err),
                    Style::new().fg(Color::Red),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  Press ", Color::Gray, false),
                    ("Enter", Color::Cyan, true),
                    (" to retry, or ", Color::Gray, false),
                    ("Esc", Color::Cyan, true),
                    (" to go back", Color::Gray, false),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    "  Running dry-run validation...",
                    Style::new().fg(Color::Cyan),
                )));
            }
        }
        10 => {
            lines.push(Line::from(Span::styled(
                " Step 10/12: Summary & Confirm ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Source:     ", Style::new().fg(Color::Gray)),
                Span::styled(
                    app.migration_source.clone(),
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  URL:        ", Style::new().fg(Color::Gray)),
                Span::styled(app.migration_url.clone(), Style::new().fg(Color::White)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Namespace:  ", Style::new().fg(Color::Gray)),
                Span::styled(
                    app.migration_namespace.clone(),
                    Style::new().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Mode:       ", Style::new().fg(Color::Gray)),
                Span::styled(app.migration_mode.clone(), Style::new().fg(Color::White)),
            ]));
            if let Some(ref target) = app.connected_url {
                let target_clone = target.clone();
                lines.push(Line::from(vec![
                    Span::styled("  Target:     ", Style::new().fg(Color::Gray)),
                    Span::styled(target_clone, Style::new().fg(Color::Cyan)),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    "  Target:     Not connected!",
                    Style::new().fg(Color::Red),
                )));
            }
            lines.push(Line::from(""));
            if app.connected() {
                lines.push(spanned_line(&[
                    ("  Press ", Color::Gray, false),
                    ("Enter", Color::Cyan, true),
                    (" to run the migration, or ", Color::Gray, false),
                    ("Esc", Color::Cyan, true),
                    (" to go back", Color::Gray, false),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    "  Connect to a PrimusDB instance first, then return here.",
                    Style::new().fg(Color::Red),
                )));
            }
        }
        _ => {
            lines.push(Line::from(Span::styled(
                " Step 11-12/12: Migration in Progress ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Source:     ", Style::new().fg(Color::Gray)),
                Span::styled(
                    app.migration_source.clone(),
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Namespace:  ", Style::new().fg(Color::Gray)),
                Span::styled(
                    app.migration_namespace.clone(),
                    Style::new().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Mode:       ", Style::new().fg(Color::Gray)),
                Span::styled(app.migration_mode.clone(), Style::new().fg(Color::White)),
            ]));
            lines.push(Line::from(""));
            render_progress_bar(lines, app.migration_progress);

            lines.push(Line::from(""));
            if let Some(ref err) = app.migration_error {
                lines.push(Line::from(Span::styled(
                    format!("  Error: {}", err),
                    Style::new().fg(Color::Red),
                )));
            } else if app.migration_progress >= 100 && !app.migration_report.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  ✓ Migration completed!",
                    Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                for line_str in app.migration_report.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line_str),
                        Style::new().fg(Color::White),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  Press ", Color::Gray, false),
                    ("Enter", Color::Cyan, true),
                    (" to save report and finish, or ", Color::Gray, false),
                    ("Esc", Color::Cyan, true),
                    (" to exit", Color::Gray, false),
                ]));
            } else if !app.migration_status.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", app.migration_status),
                    Style::new().fg(Color::Cyan),
                )));
            }
        }
    }
}

pub fn render_metrics_view(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Metrics ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(Color::Red),
        )));
    } else if let Some(ref data) = app.metrics_data {
        let max_lines = area.height as usize - 4;
        for line_str in data.lines().take(max_lines) {
            lines.push(Line::from(Span::styled(
                format!(" {}", line_str),
                Style::new().fg(Color::White),
            )));
        }
        let total = data.lines().count();
        if total > max_lines {
            let msg = format!("  ... ({} more lines)", total - max_lines);
            lines.push(Line::from(Span::styled(
                msg,
                Style::new().fg(Color::DarkGray),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No metrics data. Press r to refresh.",
            Style::new().fg(Color::Gray),
        )));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
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

pub fn render_help_page(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Help ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let help_data = help_lines();
    let lines: Vec<Line> = help_data
        .iter()
        .map(|s| {
            if s.starts_with("KEYBINDINGS")
                || s.starts_with("VERSION INFO")
                || s.starts_with("DOCUMENTATION")
            {
                Line::from(Span::styled(
                    s.as_str(),
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(s.as_str(), Style::new().fg(Color::White)))
            }
        })
        .collect();

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn help_lines() -> Vec<String> {
    vec![
        "KEYBINDINGS".to_string(),
        "".to_string(),
        "  q / Ctrl+C    Quit".to_string(),
        "  Tab           Next section".to_string(),
        "  Shift+Tab     Previous section".to_string(),
        "  Up/Down       Navigate list".to_string(),
        "  Enter         Select / Connect".to_string(),
        "  r             Refresh current view".to_string(),
        "  Ctrl+B        Create backup".to_string(),
        "  Ctrl+R        Restore backup (via CLI)".to_string(),
        "  Ctrl+E        Execute query (Queries section)".to_string(),
        "  Ctrl+D        Toggle details view".to_string(),
        "  Ctrl+L        Clear results / logs".to_string(),
        "  :             Open command palette".to_string(),
        "  Esc           Back / Close help / Close palette".to_string(),
        "  ?             Toggle this help".to_string(),
        "".to_string(),
        "COMMAND PALETTE".to_string(),
        "".to_string(),
        "  :help         Open this help".to_string(),
        "  :quit         Quit the TUI".to_string(),
        "  :refresh      Refresh current view".to_string(),
        "  :connect <url>  Connect to a server".to_string(),
        "".to_string(),
        "VERSION INFO".to_string(),
        "".to_string(),
        format!("  PrimusDB v{}", VERSION),
        "  Hybrid • Columnar • Vector • Document".to_string(),
        "".to_string(),
        "DOCUMENTATION".to_string(),
        "".to_string(),
        "  https://primusdb.dev/docs".to_string(),
        "  https://primusdb.dev/api".to_string(),
    ]
}
