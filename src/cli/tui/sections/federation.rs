use crate::cli::tui::app::{FederationMode, TuiApp};
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_federation(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let title = match app.federation_mode {
        FederationMode::View => " Federation ",
        FederationMode::AddCluster => " Federation — Add Cluster ",
        FederationMode::RemoveCluster => " Federation — Remove Cluster ",
        FederationMode::CreateDomain => " Federation — Create Domain ",
        FederationMode::DeleteDomain => " Federation — Delete Domain ",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(title)
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(p.error),
        )));
    } else {
        match app.federation_mode {
            FederationMode::AddCluster => {
                lines.push(Line::from(Span::styled(
                    "Enter cluster ID and seed URL:",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(Span::styled(
                    format!("  {}", app.federation_input),
                    Style::new().fg(p.warning),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[(
                    "  [Enter] Submit  [Esc] Cancel",
                    Color::DarkGray,
                    false,
                )]));
            }
            FederationMode::RemoveCluster => {
                lines.push(Line::from(Span::styled(
                    "Enter cluster ID to remove:",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(Span::styled(
                    format!("  {}", app.federation_input),
                    Style::new().fg(p.warning),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[(
                    "  [Enter] Submit  [Esc] Cancel",
                    Color::DarkGray,
                    false,
                )]));
            }
            FederationMode::CreateDomain => {
                lines.push(Line::from(Span::styled(
                    "Enter domain name and comma-separated cluster IDs:",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(Span::styled(
                    format!("  {}", app.federation_input),
                    Style::new().fg(p.warning),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[(
                    "  [Enter] Submit  [Esc] Cancel",
                    Color::DarkGray,
                    false,
                )]));
            }
            FederationMode::DeleteDomain => {
                lines.push(Line::from(Span::styled(
                    "Enter domain name to delete:",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(Span::styled(
                    format!("  {}", app.federation_input),
                    Style::new().fg(p.warning),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[(
                    "  [Enter] Submit  [Esc] Cancel",
                    Color::DarkGray,
                    false,
                )]));
            }
            FederationMode::View => {
                let fed_disabled = |lines: &mut Vec<Line>| {
                    lines.push(Line::from(Span::styled(
                        "  Federation not available.",
                        Style::new().fg(p.warning),
                    )));
                    lines.push(Line::from(""));
                    lines.push(spanned_line(&[
                        ("  Start server with ", Color::Gray, false),
                        ("--federation-discovery", p.primary, true),
                        (" to enable federation.", Color::Gray, false),
                    ]));
                    lines.push(spanned_line(&[
                        ("  CLI: ", Color::Gray, false),
                        (
                            "primusdb server start --federation-discovery <seed>",
                            p.primary,
                            false,
                        ),
                    ]));
                };

                if let Some(ref status) = app.federation_status {
                    if status.get("status").and_then(|v| v.as_str()) == Some("disabled") {
                        fed_disabled(&mut lines);
                    } else {
                        lines.push(Line::from(Span::styled(
                            "Federation Status:",
                            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                        )));
                        lines.push(Line::from(""));
                        let pretty = serde_json::to_string_pretty(status).unwrap_or_default();
                        for line_str in pretty.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("  {}", line_str),
                                Style::new().fg(Color::White),
                            )));
                        }
                    }
                } else {
                    fed_disabled(&mut lines);
                }
                lines.push(Line::from(""));

                if let Some(ref clusters) = app.federation_clusters {
                    if clusters.get("status").and_then(|v| v.as_str()) == Some("disabled") {
                        lines.push(Line::from(Span::styled(
                            "  Federated clusters: not configured",
                            Style::new().fg(Color::Gray),
                        )));
                    } else {
                        lines.push(Line::from(Span::styled(
                            "Federated Clusters:",
                            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                        )));
                        lines.push(Line::from(""));
                        if let Some(arr) = clusters.as_array() {
                            if arr.is_empty() {
                                lines.push(Line::from(Span::styled(
                                    "  No federated clusters. [c] Add cluster",
                                    Style::new().fg(Color::Gray),
                                )));
                            } else {
                                for cluster in arr {
                                    let id =
                                        cluster.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                                    let status = cluster
                                        .get("status")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown");
                                    let color = match status {
                                        "active" | "healthy" => p.success,
                                        "degraded" | "warning" => p.warning,
                                        "error" | "down" => p.error,
                                        _ => Color::White,
                                    };
                                    lines.push(Line::from(vec![
                                        Span::styled(
                                            "  \u{2022} ",
                                            Style::new().fg(Color::DarkGray),
                                        ),
                                        Span::styled(id, Style::new().fg(p.primary)),
                                        Span::styled(" [", Style::new().fg(Color::DarkGray)),
                                        Span::styled(
                                            status,
                                            Style::new().fg(color).add_modifier(Modifier::BOLD),
                                        ),
                                        Span::styled("]", Style::new().fg(Color::DarkGray)),
                                    ]));
                                }
                            }
                        } else {
                            let pretty = serde_json::to_string_pretty(clusters).unwrap_or_default();
                            for line_str in pretty.lines() {
                                lines.push(Line::from(Span::styled(
                                    format!("  {}", line_str),
                                    Style::new().fg(Color::White),
                                )));
                            }
                        }
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "  Federation clusters unavailable.",
                        Style::new().fg(Color::Gray),
                    )));
                }
                lines.push(Line::from(""));

                if let Some(ref domains) = app.federation_domains {
                    if domains.get("status").and_then(|v| v.as_str()) == Some("disabled") {
                        lines.push(Line::from(Span::styled(
                            "  DataDomains: not configured",
                            Style::new().fg(Color::Gray),
                        )));
                    } else {
                        lines.push(Line::from(Span::styled(
                            "DataDomains:",
                            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                        )));
                        lines.push(Line::from(""));
                        if let Some(arr) = domains.as_array() {
                            if arr.is_empty() {
                                lines.push(Line::from(Span::styled(
                                    "  No DataDomains configured. [d] Create domain",
                                    Style::new().fg(Color::Gray),
                                )));
                            } else {
                                for domain in arr {
                                    let name =
                                        domain.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                                    let status = domain
                                        .get("status")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown");
                                    let nodes = domain
                                        .get("nodes")
                                        .and_then(|v| v.as_array())
                                        .map(|a| a.len())
                                        .unwrap_or(0);
                                    let color = match status {
                                        "active" | "healthy" => p.success,
                                        _ => p.warning,
                                    };
                                    lines.push(Line::from(vec![
                                        Span::styled(
                                            "  \u{2022} ",
                                            Style::new().fg(Color::DarkGray),
                                        ),
                                        Span::styled(name, Style::new().fg(Color::White)),
                                        Span::styled(
                                            format!(" ({} nodes) ", nodes),
                                            Style::new().fg(Color::DarkGray),
                                        ),
                                        Span::styled(
                                            status,
                                            Style::new().fg(color).add_modifier(Modifier::BOLD),
                                        ),
                                    ]));
                                }
                            }
                        } else {
                            let pretty = serde_json::to_string_pretty(domains).unwrap_or_default();
                            for line_str in pretty.lines() {
                                lines.push(Line::from(Span::styled(
                                    format!("  {}", line_str),
                                    Style::new().fg(Color::White),
                                )));
                            }
                        }
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "  Federation domains unavailable.",
                        Style::new().fg(Color::Gray),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [c]", Color::Gray, false),
                    (" Add Cluster ", Color::White, true),
                    ("[r]", Color::Gray, false),
                    (" Remove ", Color::White, true),
                    ("[d]", Color::Gray, false),
                    (" Create Domain ", Color::White, true),
                    ("[x]", Color::Gray, false),
                    (" Delete Domain", Color::White, true),
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
