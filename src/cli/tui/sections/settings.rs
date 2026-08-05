use crate::cli::tui::app::{SettingsMode, TuiApp};
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_settings(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let title = match app.settings_mode {
        SettingsMode::View => " Settings ",
        SettingsMode::EditRefreshInterval => " Settings — Refresh Interval ",
        SettingsMode::EditEndpoint => " Settings — Endpoint ",
        SettingsMode::EditToken => " Settings — Auth Token ",
        SettingsMode::EditTheme => " Settings — Theme ",
        SettingsMode::EditSafeMode => " Settings — Safe Mode ",
        SettingsMode::ToggleMouse => " Settings — Mouse ",
        SettingsMode::Doctor => " Settings — Diagnostics ",
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
        match app.settings_mode {
            SettingsMode::View => {
                // Connection info
                lines.push(Line::from(Span::styled(
                    "Connection:",
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(vec![
                    Span::styled("  URL:       ", Style::new().fg(Color::Gray)),
                    Span::styled(
                        app.connected_url.as_deref().unwrap_or("(none)"),
                        Style::new().fg(Color::White),
                    ),
                ]));
                if let Some(ref ns) = app.active_namespace {
                    lines.push(Line::from(vec![
                        Span::styled("  Namespace: ", Style::new().fg(Color::Gray)),
                        Span::styled(ns, Style::new().fg(Color::White)),
                    ]));
                }
                if let Some(ref ver) = app.server_version {
                    lines.push(Line::from(vec![
                        Span::styled("  Version:   ", Style::new().fg(Color::Gray)),
                        Span::styled(ver, Style::new().fg(Color::White)),
                    ]));
                }

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "TUI Configuration:",
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                let mouse_status = if app.mouse_enabled {
                    "enabled"
                } else {
                    "disabled"
                };
                lines.push(Line::from(vec![
                    Span::styled("  Mouse:     ", Style::new().fg(Color::Gray)),
                    Span::styled(
                        mouse_status,
                        Style::new().fg(if app.mouse_enabled {
                            p.success
                        } else {
                            p.error
                        }),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  Theme:     ", Style::new().fg(Color::Gray)),
                    Span::styled(&app.config.theme, Style::new().fg(p.warning)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  Refresh:   ", Style::new().fg(Color::Gray)),
                    Span::styled(
                        format!("{}ms", app.config.refresh_interval_ms),
                        Style::new().fg(Color::White),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  Safe Mode: ", Style::new().fg(Color::Gray)),
                    Span::styled(
                        if app.config.confirm_destructive_actions {
                            "on"
                        } else {
                            "off"
                        },
                        Style::new().fg(if app.config.confirm_destructive_actions {
                            p.success
                        } else {
                            p.error
                        }),
                    ),
                ]));
                if let Some(ref token) = app.auth_token {
                    let masked: String = if token.len() > 8 {
                        format!("{}…{}", &token[..4], &token[token.len() - 4..])
                    } else {
                        "****".to_string()
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  Token:     ", Style::new().fg(Color::Gray)),
                        Span::styled(masked, Style::new().fg(Color::White)),
                    ]));
                }

                // Server status data
                if let Some(ref data) = app.settings_data {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Server Status:",
                        Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                    )));
                    if let Some(status) = data.get("status").and_then(|v| v.as_str()) {
                        lines.push(Line::from(vec![
                            Span::styled("  Status:    ", Style::new().fg(Color::Gray)),
                            Span::styled(
                                status,
                                Style::new().fg(match status {
                                    "ok" | "healthy" => p.success,
                                    _ => p.warning,
                                }),
                            ),
                        ]));
                    }
                }

                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [e] Endpoint", Color::DarkGray, false),
                    ("  [t] Token", Color::DarkGray, false),
                ]));
                lines.push(spanned_line(&[
                    ("  [i] Refresh", Color::DarkGray, false),
                    ("  [h] Theme", Color::DarkGray, false),
                    ("  [s] Safe Mode", Color::DarkGray, false),
                    ("  [m] Mouse", Color::DarkGray, false),
                    ("  [r] Refresh Status", Color::DarkGray, false),
                ]));
            }

            SettingsMode::EditRefreshInterval => {
                lines.push(Line::from(Span::styled(
                    "Enter refresh interval in milliseconds (e.g. 2000):",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(""));
                let display = if app.settings_input.is_empty() {
                    app.config.refresh_interval_ms.to_string()
                } else {
                    app.settings_input.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("  {}", display),
                    Style::new().fg(p.warning),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [Enter] Save", Color::DarkGray, false),
                    ("  [Esc] Cancel", Color::DarkGray, false),
                ]));
            }

            SettingsMode::EditEndpoint => {
                lines.push(Line::from(Span::styled(
                    "Enter server endpoint URL:",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("  {}", app.settings_input),
                    Style::new().fg(p.warning),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [Enter] Save", Color::DarkGray, false),
                    ("  [Esc] Cancel", Color::DarkGray, false),
                ]));
            }

            SettingsMode::EditToken => {
                lines.push(Line::from(Span::styled(
                    "Enter auth token (leave empty to clear):",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(""));
                let masked: String = if app.settings_input.len() > 4 {
                    format!("{}…", &app.settings_input[..4])
                } else if app.settings_input.is_empty() {
                    String::new()
                } else {
                    "****".to_string()
                };
                lines.push(Line::from(Span::styled(
                    format!("  {}", masked),
                    Style::new().fg(p.warning),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [Enter] Save", Color::DarkGray, false),
                    ("  [Esc] Cancel", Color::DarkGray, false),
                ]));
            }

            SettingsMode::EditTheme | SettingsMode::EditSafeMode | SettingsMode::ToggleMouse => {
                lines.push(Line::from(Span::styled(
                    match app.settings_mode {
                        SettingsMode::EditTheme => format!("Theme set to: {}", app.config.theme),
                        SettingsMode::EditSafeMode => format!(
                            "Safe mode: {}",
                            if app.config.confirm_destructive_actions {
                                "on"
                            } else {
                                "off"
                            }
                        ),
                        SettingsMode::ToggleMouse => format!(
                            "Mouse: {}",
                            if app.mouse_enabled {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        ),
                        _ => unreachable!(),
                    },
                    Style::new().fg(p.success),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[(
                    "  [Enter/Space/Esc] Dismiss",
                    Color::DarkGray,
                    false,
                )]));
            }

            SettingsMode::Doctor => {
                if app.doctor_results.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "Running diagnostics... Press Esc to cancel.",
                        Style::new().fg(p.primary),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        "Diagnostics Results:",
                        Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));
                    for line in &app.doctor_results {
                        let color = if line.contains("FAIL")
                            || line.contains("unreachable")
                            || line.contains("unavailable")
                        {
                            p.error
                        } else if line.contains("OK")
                            || line.contains("complete")
                            || line.contains("available")
                        {
                            p.success
                        } else if line.starts_with("  ") {
                            Color::DarkGray
                        } else {
                            Color::White
                        };
                        lines.push(Line::from(Span::styled(
                            format!("  {}", line),
                            Style::new().fg(color),
                        )));
                    }
                    lines.push(Line::from(""));
                    lines.push(spanned_line(&[(
                        "  [Enter/Space/Esc] Dismiss",
                        Color::DarkGray,
                        false,
                    )]));
                }
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
