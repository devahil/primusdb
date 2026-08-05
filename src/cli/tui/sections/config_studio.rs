use crate::cli::tui::app::{ConfigStudioMode, TuiApp};
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_config_studio(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(match app.config_mode {
            ConfigStudioMode::List => " Config Studio ",
            ConfigStudioMode::Detail => " Config Entry Detail ",
            ConfigStudioMode::Edit => " Edit Config Entry ",
            ConfigStudioMode::NewEntry => " New Config Entry ",
            ConfigStudioMode::ConfirmDelete => " Confirm Delete ",
            ConfigStudioMode::Snapshots => " Config Snapshots ",
            ConfigStudioMode::CreateSnapshot => " Create Snapshot ",
            ConfigStudioMode::ImportExport => " Import / Export ",
        })
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(p.error),
        )));
    } else {
        match app.config_mode {
            ConfigStudioMode::List => {
                if app.config_entries.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "No config entries found. Press 'n' to create one.",
                        Style::new().fg(Color::Gray),
                    )));
                } else {
                    lines.push(spanned_line(&[
                        ("  Key", p.primary, true),
                        ("  Source", p.primary, true),
                        ("  Value", p.primary, true),
                        ("  Updated", p.primary, true),
                    ]));
                    lines.push(Line::from(Span::styled(
                        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                        Style::new().fg(Color::DarkGray),
                    )));

                    for (i, entry) in app.config_entries.iter().enumerate() {
                        let is_selected = i == app.config_selected_index;
                        let prefix = if is_selected { "\u{25b8} " } else { "  " };
                        let style = if is_selected {
                            Style::new()
                                .fg(Color::White)
                                .bg(Color::Blue)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::new().fg(Color::White)
                        };

                        let key = entry.get("key").and_then(|v| v.as_str()).unwrap_or("?");
                        let source = entry.get("source").and_then(|v| v.as_str()).unwrap_or("?");
                        let value = entry
                            .get("value")
                            .map(|v| {
                                let s = serde_json::to_string(v).unwrap_or_default();
                                if s.len() > 40 {
                                    format!("{}...", &s[..37])
                                } else {
                                    s
                                }
                            })
                            .unwrap_or_default();
                        let updated = entry
                            .get("updated_at")
                            .and_then(|v| v.as_str())
                            .map(|s| if s.len() > 19 { &s[..19] } else { s })
                            .unwrap_or("?");

                        lines.push(Line::from(vec![
                            Span::styled(format!("{}{}", prefix, key), style),
                            Span::styled(
                                format!("  {}", source),
                                Style::new().fg(if is_selected {
                                    p.primary
                                } else {
                                    Color::DarkGray
                                }),
                            ),
                            Span::styled(
                                format!("  {}", value),
                                Style::new().fg(if is_selected { p.warning } else { Color::Gray }),
                            ),
                            Span::styled(
                                format!("  {}", updated),
                                Style::new().fg(if is_selected {
                                    p.success
                                } else {
                                    Color::DarkGray
                                }),
                            ),
                        ]));
                    }
                }

                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [\u{2191}/\u{2193}] Navigate", Color::DarkGray, false),
                    ("  [Enter] Detail", Color::DarkGray, false),
                    ("  [e] Edit", Color::DarkGray, false),
                    ("  [n] New", Color::DarkGray, false),
                    ("  [d] Delete", Color::DarkGray, false),
                    ("  [s] Snapshots", Color::DarkGray, false),
                    ("  [x] Import/Export", Color::DarkGray, false),
                    ("  [r] Refresh", Color::DarkGray, false),
                ]));
            }

            ConfigStudioMode::Detail => {
                if let Some(ref entry) = app.config_detail_entry {
                    let key = entry.get("key").and_then(|v| v.as_str()).unwrap_or("?");
                    let source = entry.get("source").and_then(|v| v.as_str()).unwrap_or("?");
                    let updated = entry
                        .get("updated_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let value = entry.get("value");

                    lines.push(Line::from(vec![
                        Span::styled("Key:     ", Style::new().fg(Color::Gray)),
                        Span::styled(
                            key,
                            Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Source:  ", Style::new().fg(Color::Gray)),
                        Span::styled(source, Style::new().fg(p.primary)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Updated: ", Style::new().fg(Color::Gray)),
                        Span::styled(updated, Style::new().fg(p.success)),
                    ]));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Value:",
                        Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                    )));
                    if let Some(v) = value {
                        let pretty = serde_json::to_string_pretty(v).unwrap_or_default();
                        for line_str in pretty.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("  {}", line_str),
                                Style::new().fg(Color::White),
                            )));
                        }
                    } else {
                        lines.push(Line::from(Span::styled(
                            "  (null)",
                            Style::new().fg(Color::DarkGray),
                        )));
                    }
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [e] Edit", Color::DarkGray, false),
                    ("  [Esc] Back", Color::DarkGray, false),
                ]));
            }

            ConfigStudioMode::Edit => {
                lines.push(Line::from(Span::styled(
                    "Edit the JSON value below, then press Enter:",
                    Style::new().fg(Color::Gray),
                )));
                if let Some(ref entry) = app.config_detail_entry {
                    if let Some(key) = entry.get("key").and_then(|v| v.as_str()) {
                        lines.push(Line::from(vec![
                            Span::styled("Key: ", Style::new().fg(Color::Gray)),
                            Span::styled(
                                key,
                                Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                            ),
                        ]));
                    }
                }
                lines.push(Line::from(""));
                let val_display = if app.config_input.is_empty() {
                    "(type JSON value)".to_string()
                } else {
                    app.config_input.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("  {}", val_display),
                    Style::new().fg(if app.config_input.is_empty() {
                        Color::DarkGray
                    } else {
                        p.warning
                    }),
                )));
                if let Some(ref err) = app.config_error {
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [Enter] Save", Color::DarkGray, false),
                    ("  [Esc] Cancel", Color::DarkGray, false),
                ]));
            }

            ConfigStudioMode::NewEntry => {
                lines.push(Line::from(Span::styled(
                    "Create a new config entry:",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Format: key=value  (value must be valid JSON)",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(Span::styled(
                    "  Examples:",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(Span::styled(
                    "    server.port=9090",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(Span::styled(
                    "    logging.level=\"debug\"",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(Span::styled(
                    "    cache.enabled=true",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(Span::styled(
                    "    vector.dimensions=384",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
                let input_display = if app.config_input.is_empty() {
                    "(type key=value)".to_string()
                } else {
                    app.config_input.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("  {}", input_display),
                    Style::new().fg(if app.config_input.is_empty() {
                        Color::DarkGray
                    } else {
                        p.warning
                    }),
                )));
                if let Some(ref err) = app.config_error {
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [Enter] Save", Color::DarkGray, false),
                    ("  [Esc] Cancel", Color::DarkGray, false),
                ]));
            }

            ConfigStudioMode::ConfirmDelete => {
                lines.push(Line::from(Span::styled(
                    "Are you sure you want to delete this config entry?",
                    Style::new().fg(Color::White),
                )));
                if let Some(entry) = app.config_entries.get(app.config_selected_index) {
                    if let Some(key) = entry.get("key").and_then(|v| v.as_str()) {
                        lines.push(Line::from(""));
                        lines.push(Line::from(vec![
                            Span::styled("Key: ", Style::new().fg(Color::Gray)),
                            Span::styled(
                                key,
                                Style::new().fg(p.error).add_modifier(Modifier::BOLD),
                            ),
                        ]));
                    }
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [", Color::DarkGray, false),
                    ("Y", p.success, true),
                    ("/", Color::DarkGray, false),
                    ("Enter", p.success, true),
                    ("] Yes  [", Color::DarkGray, false),
                    ("N", p.error, true),
                    ("/", Color::DarkGray, false),
                    ("Esc", p.error, true),
                    ("] No", Color::DarkGray, false),
                ]));
            }

            ConfigStudioMode::Snapshots => {
                if app.config_snapshots.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "No snapshots found. Press 'c' to create one.",
                        Style::new().fg(Color::Gray),
                    )));
                } else {
                    lines.push(spanned_line(&[
                        ("  Name", p.primary, true),
                        ("  Entries", p.primary, true),
                        ("  Created", p.primary, true),
                        ("  Description", p.primary, true),
                    ]));
                    lines.push(Line::from(Span::styled(
                        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                        Style::new().fg(Color::DarkGray),
                    )));
                    for snap in &app.config_snapshots {
                        let name = snap.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                        let entries = snap
                            .get("entries_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let created = snap
                            .get("created_at")
                            .and_then(|v| v.as_str())
                            .map(|s| if s.len() > 19 { &s[..19] } else { s })
                            .unwrap_or("?");
                        let desc = snap
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        lines.push(Line::from(vec![
                            Span::styled(format!("  {}", name), Style::new().fg(Color::White)),
                            Span::styled(format!("  {:>7}", entries), Style::new().fg(p.primary)),
                            Span::styled(
                                format!("  {}", created),
                                Style::new().fg(Color::DarkGray),
                            ),
                            Span::styled(format!("  {}", desc), Style::new().fg(Color::Gray)),
                        ]));
                    }
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [c] Create", Color::DarkGray, false),
                    ("  [r] Restore (first)", Color::DarkGray, false),
                    ("  [d] Delete (first)", Color::DarkGray, false),
                    ("  [Esc] Back", Color::DarkGray, false),
                ]));
            }

            ConfigStudioMode::CreateSnapshot => {
                lines.push(Line::from(Span::styled(
                    "Enter a name for the new snapshot:",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(""));
                let input_display = if app.config_input.is_empty() {
                    "(type snapshot name)".to_string()
                } else {
                    app.config_input.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("  {}", input_display),
                    Style::new().fg(if app.config_input.is_empty() {
                        Color::DarkGray
                    } else {
                        p.warning
                    }),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [Enter] Create", Color::DarkGray, false),
                    ("  [Esc] Cancel", Color::DarkGray, false),
                ]));
            }

            ConfigStudioMode::ImportExport => {
                lines.push(Line::from(Span::styled(
                    "Import / Export Configuration",
                    Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [e] Export bundle", Color::DarkGray, false),
                    (
                        " \u{2014} download all config entries as JSON",
                        Color::Gray,
                        false,
                    ),
                ]));
                lines.push(spanned_line(&[
                    ("  [i] Import bundle", Color::DarkGray, false),
                    (
                        " \u{2014} paste a JSON bundle and press i",
                        Color::Gray,
                        false,
                    ),
                ]));
                lines.push(Line::from(""));

                if !app.config_input.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  Import bundle JSON (press i to import):",
                        Style::new().fg(Color::DarkGray),
                    )));
                    let preview: Vec<&str> = app.config_input.lines().take(5).collect();
                    for line_str in &preview {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", line_str),
                            Style::new().fg(Color::White),
                        )));
                    }
                    if app.config_input.lines().count() > 5 {
                        lines.push(Line::from(Span::styled(
                            "  ... (truncated)",
                            Style::new().fg(Color::DarkGray),
                        )));
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "  Paste import JSON in the input bar below, then press i.",
                        Style::new().fg(Color::DarkGray),
                    )));
                }

                if let Some(ref err) = app.config_error {
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }
                if !app.config_status.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  Status: {}", app.config_status),
                        Style::new().fg(p.success),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[("  [Esc] Back", Color::DarkGray, false)]));
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
