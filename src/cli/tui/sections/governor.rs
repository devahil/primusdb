use crate::cli::tui::app::{GovernorMode, TuiApp};
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_governor(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let title = match app.governor_mode {
        GovernorMode::View => " Resource Governor ",
        GovernorMode::SetPolicy => " Governor — Set Policy ",
        GovernorMode::ConfirmDelete => " Governor — Confirm Delete ",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(p.border))
        .title(title)
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(p.error),
        )));
    } else {
        match app.governor_mode {
            GovernorMode::SetPolicy => {
                lines.push(Line::from(Span::styled(
                    "Enter policy name and JSON:",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("  {}", app.governor_policy_input),
                    Style::new().fg(p.warning),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [Enter] Submit via palette", Color::DarkGray, false),
                    ("  [Esc] Cancel", Color::DarkGray, false),
                ]));
            }
            GovernorMode::ConfirmDelete => {
                lines.push(Line::from(Span::styled(
                    format!("  Delete policy '{}'?", app.governor_policy_name),
                    Style::new().fg(p.error).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[(
                    "  [y] Yes  [n] No  [Esc] Cancel",
                    Color::DarkGray,
                    false,
                )]));
            }
            GovernorMode::View => {
                if app.settings_data.is_none()
                    && app.governor_status.is_none()
                    && app.governor_executions.is_empty()
                {
                    lines.push(Line::from(Span::styled(
                        "  Resource Governor not available.",
                        Style::new().fg(p.warning),
                    )));
                    lines.push(Line::from(""));
                    lines.push(spanned_line(&[
                        ("  Start server with ", Color::Gray, false),
                        ("--governor", p.primary, true),
                        (" to enable.", Color::Gray, false),
                    ]));
                    lines.push(spanned_line(&[
                        ("  CLI: ", Color::Gray, false),
                        ("primusdb server start --governor", p.primary, false),
                    ]));
                } else {
                    if let Some(ref data) = app.settings_data {
                        lines.push(Line::from(Span::styled(
                            "Governor Status:",
                            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                        )));
                        lines.push(Line::from(""));
                        let pretty = serde_json::to_string_pretty(data).unwrap_or_default();
                        for line_str in pretty.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("  {}", line_str),
                                Style::new().fg(Color::White),
                            )));
                        }
                        lines.push(Line::from(""));
                    }
                    if let Some(ref status) = app.governor_status {
                        let status_str = serde_json::to_string_pretty(status).unwrap_or_default();
                        for line_str in status_str.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("  {}", line_str),
                                Style::new().fg(p.primary),
                            )));
                        }
                        lines.push(Line::from(""));
                    }

                    if !app.governor_executions.is_empty() {
                        lines.push(Line::from(Span::styled(
                            format!("  Active Executions ({})", app.governor_executions.len()),
                            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                        )));
                        for exec in app.governor_executions.iter().take(10) {
                            lines.push(Line::from(Span::styled(
                                format!("    {}", exec),
                                Style::new().fg(Color::White),
                            )));
                        }
                        lines.push(Line::from(""));
                    }

                    if !app.governor_violations.is_empty() {
                        lines.push(Line::from(Span::styled(
                            format!("  Recent Violations ({})", app.governor_violations.len()),
                            Style::new().fg(p.error).add_modifier(Modifier::BOLD),
                        )));
                        for v in app.governor_violations.iter().take(8) {
                            lines.push(Line::from(Span::styled(
                                format!("    ⚠ {}", v),
                                Style::new().fg(Color::LightRed),
                            )));
                        }
                        lines.push(Line::from(""));
                    }

                    if let Some(ref metrics) = app.governor_metrics {
                        lines.push(Line::from(Span::styled(
                            "  Metrics:",
                            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                        )));
                        lines.push(Line::from(""));
                        match metrics {
                            serde_json::Value::Object(map) => {
                                for (key, val) in map.iter().take(10) {
                                    let val_str = match val {
                                        serde_json::Value::String(s) => s.clone(),
                                        serde_json::Value::Number(n) => n.to_string(),
                                        serde_json::Value::Bool(b) => b.to_string(),
                                        serde_json::Value::Array(arr) => {
                                            format!("[{} items]", arr.len())
                                        }
                                        serde_json::Value::Object(obj) => {
                                            format!("{{{} keys}}", obj.len())
                                        }
                                        serde_json::Value::Null => "null".to_string(),
                                    };
                                    lines.push(Line::from(Span::styled(
                                        format!("    {}: {}", key, val_str),
                                        Style::new().fg(p.success),
                                    )));
                                }
                                if map.len() > 10 {
                                    lines.push(Line::from(Span::styled(
                                        format!("    ... ({} more)", map.len() - 10),
                                        Style::new().fg(Color::DarkGray),
                                    )));
                                }
                            }
                            _ => {
                                let metrics_str =
                                    serde_json::to_string_pretty(metrics).unwrap_or_default();
                                for line_str in metrics_str.lines().take(5) {
                                    lines.push(Line::from(Span::styled(
                                        format!("  {}", line_str),
                                        Style::new().fg(p.success),
                                    )));
                                }
                            }
                        }
                    }

                    lines.push(spanned_line(&[
                        ("  [s]", Color::Gray, false),
                        (" Set Policy ", Color::White, true),
                        ("[d]", Color::Gray, false),
                        (" Delete ", Color::White, true),
                        ("[r]", Color::Gray, false),
                        (" Refresh", Color::White, true),
                    ]));
                }
            }
        }
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
