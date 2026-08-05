use crate::cli::tui::app::TuiApp;
use crate::cli::tui::widgets::{render_gauge, spanned_line};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_dashboard(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(p.border))
        .title(" Dashboard ")
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if let Some(ref url) = app.connected_url {
        lines.push(spanned_line(&[
            ("Instance URL: ", Color::Gray, false),
            (url, p.primary, false),
        ]));

        let health_color = match app.health_status.as_deref() {
            Some("healthy") | Some("ok") => p.success,
            Some(_) => p.error,
            None => p.error,
        };
        lines.push(spanned_line(&[
            ("Health:       ", Color::Gray, false),
            (
                app.health_status.as_deref().unwrap_or("unknown"),
                health_color,
                true,
            ),
        ]));

        lines.push(spanned_line(&[
            ("Version:      ", Color::Gray, false),
            (
                app.server_version.as_deref().unwrap_or("-"),
                Color::White,
                false,
            ),
        ]));

        lines.push(spanned_line(&[
            ("Uptime:       ", Color::Gray, false),
            (app.uptime.as_deref().unwrap_or("-"), Color::White, false),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Engines:      ", Style::new().fg(p.text_dim)),
            Span::styled(
                if app.engine_list.is_empty() {
                    "none".to_string()
                } else {
                    app.engine_list.join(", ")
                },
                Style::new().fg(p.primary),
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Real-time Metrics",
            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
        )));
        lines.push(spanned_line(&[
            ("  Query Rate:   ", Color::Gray, false),
            (
                app.query_rate.as_deref().unwrap_or("not available"),
                p.primary,
                false,
            ),
        ]));
        lines.push(spanned_line(&[
            ("  Error Rate:   ", Color::Gray, false),
            (
                app.error_rate.as_deref().unwrap_or("not available"),
                p.primary,
                false,
            ),
        ]));
        lines.push(spanned_line(&[
            ("  Memory:       ", Color::Gray, false),
            (
                app.memory_usage.as_deref().unwrap_or("not available"),
                p.primary,
                false,
            ),
        ]));
        lines.push(spanned_line(&[
            ("  Storage:      ", Color::Gray, false),
            (
                app.storage_usage.as_deref().unwrap_or("not available"),
                p.primary,
                false,
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ASCII Charts",
            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
        )));
        let bar_width = 15usize;
        let render_bar = |lines: &mut Vec<Line>, label: &str, pct: u8, color: Color| {
            let filled = ((pct as usize) * bar_width / 100).min(bar_width);
            let empty = bar_width.saturating_sub(filled);
            let bar: String = format!("{}{} {:3}%", "█".repeat(filled), "░".repeat(empty), pct);
            lines.push(Line::from(vec![
                Span::styled(format!("  {}: ", label), Style::new().fg(p.text_dim)),
                Span::styled(bar, Style::new().fg(color)),
            ]));
        };

        let query_pct = app
            .query_rate
            .as_ref()
            .and_then(|v| {
                v.trim_end_matches(" qps")
                    .parse::<f64>()
                    .ok()
                    .map(|q| (q as u8 * 10).min(100))
            })
            .unwrap_or(0);
        render_bar(&mut lines, "QPS", query_pct, p.success);

        let err_pct = app
            .error_rate
            .as_ref()
            .and_then(|v| v.parse::<f64>().ok().map(|e| (e as u8 * 20).min(100)))
            .unwrap_or(0);
        render_bar(&mut lines, "Errors", err_pct, p.error);

        let mem_bar_pct = app
            .memory_usage
            .as_ref()
            .and_then(|v| {
                if v.ends_with("GB") {
                    v.trim_end_matches(" GB")
                        .parse::<f64>()
                        .ok()
                        .map(|g| (g * 10.0) as u8)
                } else if v.ends_with("MB") {
                    v.trim_end_matches(" MB")
                        .parse::<f64>()
                        .ok()
                        .map(|m| (m / 102.4) as u8)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        render_bar(&mut lines, "Memory", mem_bar_pct.min(100), p.primary);

        let stg_bar_pct = app
            .storage_usage
            .as_ref()
            .and_then(|v| {
                if v.ends_with("GB") {
                    v.trim_end_matches(" GB")
                        .parse::<f64>()
                        .ok()
                        .map(|g| (g * 10.0) as u8)
                } else if v.ends_with("MB") {
                    v.trim_end_matches(" MB")
                        .parse::<f64>()
                        .ok()
                        .map(|m| (m / 102.4) as u8)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        render_bar(&mut lines, "Storage", stg_bar_pct.min(100), p.accent);

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Cluster Node Health",
            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
        )));
        if let Some(ref nodes_val) = app.cluster_nodes {
            if let Some(nodes_arr) = nodes_val.as_array() {
                if nodes_arr.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  No nodes in cluster.",
                        Style::new().fg(p.text_dim),
                    )));
                } else {
                    for node in nodes_arr {
                        let id = node
                            .get("id")
                            .or(node.get("node_id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let status = node
                            .get("status")
                            .or(node.get("health"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let role = node.get("role").and_then(|v| v.as_str()).unwrap_or("?");
                        let dot = match status {
                            "healthy" | "ok" | "online" | "active" | "leader" => "●",
                            "warning" | "degraded" => "◒",
                            "error" | "down" | "offline" => "○",
                            _ => "◌",
                        };
                        let dot_color = match status {
                            "healthy" | "ok" | "online" | "active" | "leader" => p.success,
                            "warning" | "degraded" => p.warning,
                            "error" | "down" | "offline" => p.error,
                            _ => p.text_dim,
                        };
                        lines.push(Line::from(vec![
                            Span::styled(format!("  {} ", dot), Style::new().fg(dot_color)),
                            Span::styled(format!("{} ", id), Style::new().fg(p.text)),
                            Span::styled(format!("({})", role), Style::new().fg(p.text_dim)),
                        ]));
                    }

                    let healthy_count = nodes_arr
                        .iter()
                        .filter(|n| {
                            let s = n
                                .get("status")
                                .or(n.get("health"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            matches!(s, "healthy" | "ok" | "online" | "active" | "leader")
                        })
                        .count();
                    let total = nodes_arr.len();
                    let health_pct2 = if total > 0 {
                        ((healthy_count * 100).checked_div(total).unwrap_or(0)) as u8
                    } else {
                        0
                    };
                    render_bar(&mut lines, " Health", health_pct2, p.success);
                }
            } else {
                lines.push(Line::from(Span::styled(
                    "  No cluster data available.",
                    Style::new().fg(p.text_dim),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "  No cluster data. Connect and press r to refresh.",
                Style::new().fg(p.text_dim),
            )));
        }

        if !app.backups_data.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Backups Summary",
                Style::new().fg(p.title).add_modifier(Modifier::BOLD),
            )));
            let count_str = format!("{}", app.backups_data.len());
            lines.push(Line::from(vec![
                Span::styled("  Count: ", Style::new().fg(p.text_dim)),
                Span::styled(count_str, Style::new().fg(p.primary)),
            ]));
            if let Some(ref detail) = app.backups_detail {
                if let Some(arr) = detail.get("backups").and_then(|b| b.as_array()) {
                    if let Some(latest) = arr.last() {
                        let latest_date = latest
                            .get("created_at")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        lines.push(spanned_line(&[
                            ("  Latest: ", Color::Gray, false),
                            (latest_date, p.primary, false),
                        ]));
                    }
                }
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Import Throughput",
            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
        )));
        if app.migration_status.contains("Migration") || app.migration_progress > 0 {
            render_bar(&mut lines, " Import", app.migration_progress, p.primary);
            lines.push(spanned_line(&[
                ("  Status: ", Color::Gray, false),
                (app.migration_status.as_str(), p.primary, false),
            ]));
        } else {
            lines.push(Line::from(Span::styled(
                "  No active migration. Use Migrations section.",
                Style::new().fg(p.text_dim),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Health Gauges",
            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
        )));
        let health_pct = match app.health_status.as_deref() {
            Some("healthy") | Some("ok") => 100,
            Some(_) => 30,
            None => 0,
        };
        render_gauge(&mut lines, "Health", health_pct, p.success);

        let eng_pct = ((app.engine_list.len() as u8) * 100 / 5).min(100);
        render_gauge(&mut lines, "Engines", eng_pct, p.primary);

        let up_pct = if app.uptime.is_some() { 100 } else { 0 };
        render_gauge(&mut lines, "Uptime", up_pct, p.warning);

        let mem_pct = app
            .memory_usage
            .as_ref()
            .and_then(|v| {
                if v.ends_with("GB") {
                    v.trim_end_matches(" GB")
                        .parse::<f64>()
                        .ok()
                        .map(|g| (g * 10.0) as u8)
                } else if v.ends_with("MB") {
                    v.trim_end_matches(" MB")
                        .parse::<f64>()
                        .ok()
                        .map(|m| (m / 102.4) as u8)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        render_gauge(&mut lines, "Memory", mem_pct.min(100), p.primary);

        let stg_pct = app
            .storage_usage
            .as_ref()
            .and_then(|v| {
                if v.ends_with("GB") {
                    v.trim_end_matches(" GB")
                        .parse::<f64>()
                        .ok()
                        .map(|g| (g * 10.0) as u8)
                } else if v.ends_with("MB") {
                    v.trim_end_matches(" MB")
                        .parse::<f64>()
                        .ok()
                        .map(|m| (m / 102.4) as u8)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        render_gauge(&mut lines, "Storage", stg_pct.min(100), p.accent);

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Discovery Results",
            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
        )));

        if app.instances.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No local instances found.",
                Style::new().fg(p.text_dim),
            )));
        } else {
            for inst in &app.instances {
                let status_color = match inst.status.as_str() {
                    "healthy" | "ok" => p.success,
                    _ => p.error,
                };
                lines.push(spanned_line(&[
                    ("  • ", Color::Gray, false),
                    (&inst.endpoint, p.primary, false),
                    (" ", Color::Reset, false),
                    (&inst.status, status_color, false),
                    (" ", Color::Reset, false),
                    (
                        inst.version.as_deref().unwrap_or("-"),
                        Color::DarkGray,
                        false,
                    ),
                ]));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Not connected to any PrimusDB instance.",
            Style::new().fg(p.error),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Getting Started:",
            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
        )));
        lines.push(spanned_line(&[
            ("  1. Start a server: ", Color::Gray, false),
            ("primusdb server start", p.primary, false),
        ]));
        lines.push(spanned_line(&[
            ("  2. Connect to it:   ", Color::Gray, false),
            (
                "primusdb tui --server http://localhost:8080",
                p.primary,
                false,
            ),
        ]));
        lines.push(spanned_line(&[
            ("  3. Or discover:     ", Color::Gray, false),
            (
                "use Instances section to find running servers",
                p.primary,
                false,
            ),
        ]));

        if !app.instances.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Discovered instances:",
                Style::new().fg(p.title).add_modifier(Modifier::BOLD),
            )));
            for inst in &app.instances {
                let status_color = match inst.status.as_str() {
                    "healthy" | "ok" => p.success,
                    _ => p.error,
                };
                lines.push(spanned_line(&[
                    ("  • ", Color::Gray, false),
                    (&inst.endpoint, p.primary, false),
                    (" ", Color::Reset, false),
                    (&inst.status, status_color, false),
                ]));
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
