use crate::cli::tui::app::{DatabasesEnginesMode, TuiApp};
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_engines(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(p.border))
        .title(" Databases & Engines ")
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(p.error),
        )));
    } else {
        // Node info header
        let role = app.server_role.as_deref().unwrap_or("standalone");
        let ns = app.active_namespace.as_deref().unwrap_or("-");
        lines.push(Line::from(vec![
            Span::styled("  Node: ", Style::new().fg(p.text_dim)),
            Span::styled(
                role,
                Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  |  Namespace: ", Style::new().fg(p.text_dim)),
            Span::styled(ns, Style::new().fg(p.warning)),
        ]));
        lines.push(Line::from(""));

        match app.engines_mode {
            DatabasesEnginesMode::List => {
                if app.engine_list.is_empty() && app.databases_data.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "No engines or databases found.",
                        Style::new().fg(p.text_dim),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("  Storage Engines ({}):", app.engine_list.len()),
                        Style::new().fg(p.success).add_modifier(Modifier::BOLD),
                    )));
                    for engine in &app.engine_list {
                        lines.push(Line::from(vec![
                            Span::styled("  ● ", Style::new().fg(p.success)),
                            Span::styled(
                                format!("{}  ", engine),
                                Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                "(relational, document, key-value, vector, time-series)",
                                Style::new().fg(p.text_dim),
                            ),
                        ]));
                    }
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  Databases/Tables ({}):", app.databases_data.len()),
                        Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                    )));
                    for (i, db) in app.databases_data.iter().enumerate() {
                        let is_sel = i == app.selected_table_index;
                        let prefix = if is_sel { "  ▸ " } else { "    " };
                        let style = if is_sel {
                            Style::new().bg(Color::DarkGray)
                        } else {
                            Style::new()
                        };
                        lines.push(
                            Line::from(Span::styled(
                                format!("{}{}", prefix, db),
                                Style::new().fg(p.text).add_modifier(if is_sel {
                                    Modifier::BOLD
                                } else {
                                    Modifier::empty()
                                }),
                            ))
                            .style(style),
                        );
                    }
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  ↑↓ Select  ", p.text_dim, false),
                    ("Enter", p.text, true),
                    (" inspect  ", p.text_dim, false),
                    ("n", p.text, true),
                    (" new DB  ", p.text_dim, false),
                    ("d", p.text, true),
                    (" drop  ", p.text_dim, false),
                    ("r", p.text, true),
                    (" refresh  ", p.text_dim, false),
                    ("e", p.text, true),
                    (" log", p.text_dim, false),
                ]));
            }
            DatabasesEnginesMode::Inspect => {
                if let Some(db) = app.databases_data.get(app.selected_table_index) {
                    lines.push(Line::from(Span::styled(
                        format!("  Database: {}", db),
                        Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  Storage Type: relational",
                        Style::new().fg(p.warning),
                    )));
                    lines.push(Line::from(""));
                    if let Some(ref detail) = app.engines_detail {
                        lines.push(Line::from(Span::styled(
                            "  ── Table Info ──",
                            Style::new().fg(p.border),
                        )));
                        let text = serde_json::to_string_pretty(detail).unwrap_or_default();
                        for line in text.lines().take(25) {
                            lines.push(Line::from(Span::styled(
                                format!("  {}", line),
                                Style::new().fg(p.primary),
                            )));
                        }
                    } else {
                        lines.push(Line::from(Span::styled(
                            "  Loading...",
                            Style::new().fg(p.text_dim),
                        )));
                    }
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  Esc", p.text, true),
                    (" back  ", p.text_dim, false),
                ]));
            }
            DatabasesEnginesMode::ConfirmDelete => {
                if let Some(db) = app.databases_data.get(app.selected_table_index) {
                    lines.push(Line::from(Span::styled(
                        format!("  Delete database '{}'?", db),
                        Style::new().fg(p.error).add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(Span::styled(
                        "  This will drop the table and all its data.",
                        Style::new().fg(p.error),
                    )));
                    lines.push(Line::from(""));
                    lines.push(spanned_line(&[
                        ("  ", Color::Reset, false),
                        ("y", p.text, true),
                        ("/", p.text_dim, false),
                        ("Y", p.text, true),
                        (" or ", p.text_dim, false),
                        ("Enter", p.text, true),
                        (" to confirm  |  ", p.text_dim, false),
                        ("n", p.text, true),
                        ("/", p.text_dim, false),
                        ("N", p.text, true),
                        (" or ", p.text_dim, false),
                        ("Esc", p.text, true),
                        (" to cancel", p.text_dim, false),
                    ]));
                }
            }
            DatabasesEnginesMode::CreateDatabase => {
                // Rendered by workspace's CreateDbWizard
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
