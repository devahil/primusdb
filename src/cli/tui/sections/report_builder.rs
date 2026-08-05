use crate::cli::tui::app::{ReportBuilderMode, TuiApp};
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_report_builder(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let title = match app.report_mode {
        ReportBuilderMode::List => " Report Builder ",
        ReportBuilderMode::Detail => " Report Detail ",
        ReportBuilderMode::Create => " Create Report ",
        ReportBuilderMode::Edit => " Edit Report ",
        ReportBuilderMode::ConfirmDelete => " Confirm Delete ",
        ReportBuilderMode::Results => " Report Results ",
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
        match app.report_mode {
            ReportBuilderMode::List => {
                if app.reports_data.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "No saved reports. Press 'n' to create one.",
                        Style::new().fg(Color::Gray),
                    )));
                } else {
                    lines.push(spanned_line(&[
                        ("  #", p.primary, true),
                        ("  Name", p.primary, true),
                        ("  Type", p.primary, true),
                        ("  Table", p.primary, true),
                        ("  Format", p.primary, true),
                    ]));
                    lines.push(Line::from(Span::styled(
                        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                        Style::new().fg(Color::DarkGray),
                    )));

                    for (i, report) in app.reports_data.iter().enumerate() {
                        let name = report
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string();
                        let st = report
                            .get("storage_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string();
                        let tbl = report
                            .get("table_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string();
                        let fmt = report
                            .get("format")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string();
                        let is_selected = i == app.report_selected_index;
                        let prefix = if is_selected { "\u{25b8} " } else { "  " };
                        let style = if is_selected {
                            Style::new()
                                .fg(Color::White)
                                .bg(Color::Blue)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::new().fg(Color::White)
                        };
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("{}{:<3}", prefix, i + 1),
                                Style::new().fg(Color::DarkGray),
                            ),
                            Span::styled(name, style),
                            Span::styled(format!(" {}", st), Style::new().fg(p.primary)),
                            Span::styled(format!(" {}", tbl), Style::new().fg(Color::Gray)),
                            Span::styled(format!(" {}", fmt), Style::new().fg(p.warning)),
                        ]));
                    }
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [\u{2191}/\u{2193}] Navigate", Color::DarkGray, false),
                    ("  [Enter] Detail/Run", Color::DarkGray, false),
                    ("  [n] New", Color::DarkGray, false),
                    ("  [d] Delete", Color::DarkGray, false),
                    ("  [r] Refresh", Color::DarkGray, false),
                ]));
            }

            ReportBuilderMode::Detail => {
                if let Some(ref report) = app.report_detail {
                    let name = report.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    let query = report.get("query").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = report
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let st = report
                        .get("storage_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let fmt = report.get("format").and_then(|v| v.as_str()).unwrap_or("?");
                    let tbl = report
                        .get("table_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");

                    lines.push(Line::from(vec![
                        Span::styled("Name:    ", Style::new().fg(Color::Gray)),
                        Span::styled(
                            name,
                            Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Table:   ", Style::new().fg(Color::Gray)),
                        Span::styled(tbl, Style::new().fg(p.primary)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Type:    ", Style::new().fg(Color::Gray)),
                        Span::styled(st, Style::new().fg(p.primary)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Format:  ", Style::new().fg(Color::Gray)),
                        Span::styled(fmt, Style::new().fg(p.warning)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Query:   ", Style::new().fg(Color::Gray)),
                        Span::styled(query, Style::new().fg(Color::White)),
                    ]));
                    if !desc.is_empty() {
                        lines.push(Line::from(vec![
                            Span::styled("Desc:    ", Style::new().fg(Color::Gray)),
                            Span::styled(desc, Style::new().fg(Color::Gray)),
                        ]));
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "Loading report...",
                        Style::new().fg(Color::Gray),
                    )));
                }

                if let Some(ref err) = app.report_error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }

                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [Enter] Run Report", Color::DarkGray, false),
                    ("  [e] Edit", p.warning, false),
                    ("  [Esc] Back", Color::DarkGray, false),
                ]));
            }

            ReportBuilderMode::Create => {
                lines.push(Line::from(Span::styled(
                    "Create a new report definition:",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Format: name|query|description|storage_type|format|table_name",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Example:",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(Span::styled(
                    "    User Report|SELECT * FROM users|All users|relational|json|users",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));

                let input_display = if app.report_input.is_empty() {
                    "(type pipe-separated fields)".to_string()
                } else {
                    app.report_input.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("  {}", input_display),
                    Style::new().fg(if app.report_input.is_empty() {
                        Color::DarkGray
                    } else {
                        p.warning
                    }),
                )));

                if let Some(ref err) = app.report_error {
                    lines.push(Line::from(""));
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

            ReportBuilderMode::Edit => {
                lines.push(Line::from(Span::styled(
                    "Edit report definition:",
                    Style::new().fg(Color::Gray),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Format: name|query|description|storage_type|format|table_name",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
                let input_display = if app.report_input.is_empty() {
                    "(type pipe-separated fields)".to_string()
                } else {
                    app.report_input.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("  {}", input_display),
                    Style::new().fg(p.primary),
                )));
                if let Some(ref err) = app.report_error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[(
                    "  [Enter] Save Changes  [Esc] Cancel",
                    Color::DarkGray,
                    false,
                )]));
            }

            ReportBuilderMode::ConfirmDelete => {
                lines.push(Line::from(Span::styled(
                    "Are you sure you want to delete this report?",
                    Style::new().fg(Color::White),
                )));
                if let Some(report) = app.reports_data.get(app.report_selected_index) {
                    if let Some(name) = report.get("name").and_then(|v| v.as_str()) {
                        lines.push(Line::from(""));
                        lines.push(Line::from(vec![
                            Span::styled("Report: ", Style::new().fg(Color::Gray)),
                            Span::styled(
                                name,
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

            ReportBuilderMode::Results => {
                if let Some(ref data) = app.report_results {
                    if let Some(rows) = data.get("rows").and_then(|v| v.as_array()) {
                        lines.push(Line::from(Span::styled(
                            format!("  {} row(s) returned", rows.len()),
                            Style::new().fg(p.primary),
                        )));
                        lines.push(Line::from(Span::styled(
                            "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                            Style::new().fg(Color::DarkGray),
                        )));
                        for row in rows {
                            let row_str = serde_json::to_string(row).unwrap_or_default();
                            let display = if row_str.len() > area.width as usize - 6 {
                                format!("{}...", &row_str[..area.width as usize - 9])
                            } else {
                                row_str
                            };
                            lines.push(Line::from(Span::styled(
                                format!("  {}", display),
                                Style::new().fg(Color::White),
                            )));
                        }
                    } else {
                        let pretty = serde_json::to_string_pretty(data).unwrap_or_default();
                        for line_str in pretty.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("  {}", line_str),
                                Style::new().fg(Color::White),
                            )));
                        }
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "Running report...",
                        Style::new().fg(Color::Gray),
                    )));
                }

                if let Some(ref err) = app.report_error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
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
