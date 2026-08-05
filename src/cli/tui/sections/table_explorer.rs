use crate::cli::tui::app::{TableExplorerMode, TuiApp};
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_table_explorer(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let title = match app.table_explorer_mode {
        TableExplorerMode::StorageTypeSelect => " Table Explorer — Storage Type ".into(),
        TableExplorerMode::TableList => {
            let st = app.explorer_selected_st.as_deref().unwrap_or("?");
            format!(" Table Explorer — {} Tables ", st)
        }
        TableExplorerMode::TableDetail => " Table Explorer — Table Detail ".into(),
        TableExplorerMode::RowBrowser => " Table Explorer — Row Browser ".into(),
        TableExplorerMode::RowInsert => " Table Explorer — Insert Row ".into(),
        TableExplorerMode::ConfirmDelete => " Table Explorer — Confirm Delete ".into(),
        TableExplorerMode::ExportOptions => " Table Explorer — Export Options ".into(),
        TableExplorerMode::AnalyzeTable => " Table Explorer — Analyze Table ".into(),
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
        match app.table_explorer_mode {
            TableExplorerMode::StorageTypeSelect => {
                if app.explorer_storage_types.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "Loading storage types... Press [r] to refresh.",
                        Style::new().fg(Color::Gray),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        "Select a storage type to explore:",
                        Style::new().fg(Color::Gray),
                    )));
                    lines.push(Line::from(""));
                    for (i, st) in app.explorer_storage_types.iter().enumerate() {
                        let is_selected = i == app.explorer_selected_st_index;
                        let prefix = if is_selected { "\u{25b8} " } else { "  " };
                        let style = if is_selected {
                            Style::new()
                                .fg(Color::White)
                                .bg(Color::Blue)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::new().fg(p.primary)
                        };
                        lines.push(Line::from(Span::styled(format!("{}{}", prefix, st), style)));
                    }
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [\u{2191}/\u{2193}] Navigate", Color::DarkGray, false),
                    ("  [Enter] Select", Color::DarkGray, false),
                    ("  [r] Refresh", Color::DarkGray, false),
                ]));
            }

            TableExplorerMode::TableList => {
                let tables = app
                    .explorer_tables_data
                    .as_ref()
                    .and_then(|v| v.get("tables"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();

                if tables.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "No tables found for this storage type. [i] Insert JSON | [n] Create table",
                        Style::new().fg(Color::Gray),
                    )));
                } else {
                    lines.push(spanned_line(&[
                        ("  #", p.primary, true),
                        ("  Table Name", p.primary, true),
                    ]));
                    lines.push(Line::from(Span::styled(
                        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                        Style::new().fg(Color::DarkGray),
                    )));

                    for (i, table_val) in tables.iter().enumerate() {
                        let name = table_val
                            .as_str()
                            .or_else(|| table_val.get("name").and_then(|v| v.as_str()))
                            .unwrap_or("?")
                            .to_string();
                        let is_selected = i == app.explorer_selected_table_index;
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
                                Style::new().fg(if is_selected {
                                    p.primary
                                } else {
                                    Color::DarkGray
                                }),
                            ),
                            Span::styled(name, style),
                        ]));
                    }
                }

                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [\u{2191}/\u{2193}] Navigate", Color::DarkGray, false),
                    ("  [Enter] View Detail", Color::DarkGray, false),
                    ("  [Esc] Back", Color::DarkGray, false),
                    ("  [r] Refresh", Color::DarkGray, false),
                ]));
            }

            TableExplorerMode::TableDetail => {
                if let Some(ref info) = app.explorer_table_info {
                    let name = info.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    let st = info
                        .get("storage_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let row_count = info.get("row_count").and_then(|v| v.as_u64()).unwrap_or(0);
                    let size_bytes = info.get("size_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                    let created = info
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let updated = info
                        .get("updated_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");

                    lines.push(Line::from(vec![
                        Span::styled("Table:    ", Style::new().fg(Color::Gray)),
                        Span::styled(
                            name,
                            Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Type:     ", Style::new().fg(Color::Gray)),
                        Span::styled(st, Style::new().fg(p.primary)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Rows:     ", Style::new().fg(Color::Gray)),
                        Span::styled(format!("{}", row_count), Style::new().fg(p.warning)),
                    ]));
                    let size_str = if size_bytes > 1_000_000_000 {
                        format!("{:.1} GB", size_bytes as f64 / 1_000_000_000.0)
                    } else if size_bytes > 1_000_000 {
                        format!("{:.1} MB", size_bytes as f64 / 1_000_000.0)
                    } else if size_bytes > 1_000 {
                        format!("{:.1} KB", size_bytes as f64 / 1_000.0)
                    } else {
                        format!("{} B", size_bytes)
                    };
                    lines.push(Line::from(vec![
                        Span::styled("Size:     ", Style::new().fg(Color::Gray)),
                        Span::styled(size_str, Style::new().fg(p.warning)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Created:  ", Style::new().fg(Color::Gray)),
                        Span::styled(
                            if created.len() > 19 {
                                &created[..19]
                            } else {
                                created
                            },
                            Style::new().fg(p.success),
                        ),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Updated:  ", Style::new().fg(Color::Gray)),
                        Span::styled(
                            if updated.len() > 19 {
                                &updated[..19]
                            } else {
                                updated
                            },
                            Style::new().fg(p.success),
                        ),
                    ]));
                    lines.push(Line::from(""));

                    if let Some(schema) = info.get("schema") {
                        lines.push(Line::from(Span::styled(
                            "Schema:",
                            Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                        )));
                        if let Some(fields) = schema.get("fields").and_then(|v| v.as_array()) {
                            for field in fields {
                                let fname =
                                    field.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                                let ftype = field
                                    .get("field_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("?");
                                let nullable = field
                                    .get("nullable")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(true);
                                let null_str = if nullable { "NULL" } else { "NOT NULL" };
                                lines.push(Line::from(vec![
                                    Span::styled(
                                        format!("  \u{251c}\u{2500} {}", fname),
                                        Style::new().fg(Color::White),
                                    ),
                                    Span::styled(
                                        format!("  {}", ftype),
                                        Style::new().fg(p.primary),
                                    ),
                                    Span::styled(
                                        format!("  {}", null_str),
                                        Style::new().fg(if nullable {
                                            Color::DarkGray
                                        } else {
                                            p.error
                                        }),
                                    ),
                                ]));
                            }
                        } else {
                            let pretty = serde_json::to_string_pretty(schema).unwrap_or_default();
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
                        "Loading table info...",
                        Style::new().fg(Color::Gray),
                    )));
                }

                if let Some(ref err) = app.explorer_error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }

                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [Enter] Browse Rows", Color::DarkGray, false),
                    ("  [a] Analyze", p.warning, false),
                    ("  [Esc] Back", Color::DarkGray, false),
                    ("  [r] Refresh", Color::DarkGray, false),
                ]));
            }

            TableExplorerMode::RowBrowser => {
                let rows = app
                    .explorer_rows_data
                    .as_ref()
                    .and_then(|v| v.get("rows"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let total = app
                    .explorer_rows_data
                    .as_ref()
                    .and_then(|v| v.get("total"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let current_page = app.explorer_row_offset / app.explorer_row_limit.max(1) + 1;
                let total_pages =
                    (total as f64 / app.explorer_row_limit.max(1) as f64).ceil() as u64;
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(
                            "  Rows {}-{} of {}",
                            app.explorer_row_offset + 1,
                            (app.explorer_row_offset + rows.len() as u64).min(total),
                            total
                        ),
                        Style::new().fg(p.primary),
                    ),
                    Span::styled(
                        format!("  (Page {}/{})", current_page, total_pages.max(1)),
                        Style::new().fg(Color::DarkGray),
                    ),
                ]));
                lines.push(Line::from(Span::styled(
                    "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                    Style::new().fg(Color::DarkGray),
                )));

                if rows.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  No rows found.",
                        Style::new().fg(Color::Gray),
                    )));
                } else {
                    for (i, row) in rows.iter().enumerate() {
                        let row_num = app.explorer_row_offset + i as u64 + 1;
                        let row_str = serde_json::to_string(row).unwrap_or_default();
                        let display = if row_str.len() > area.width as usize - 6 {
                            format!("{}...", &row_str[..area.width as usize - 9])
                        } else {
                            row_str
                        };
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("{:>4} ", row_num),
                                Style::new().fg(Color::DarkGray),
                            ),
                            Span::styled(display, Style::new().fg(Color::White)),
                        ]));
                    }
                }

                if let Some(ref err) = app.explorer_error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }

                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [\u{2190}/p] Prev Page", Color::DarkGray, false),
                    ("  [\u{2192}/n] Next Page", Color::DarkGray, false),
                    ("  [Esc] Back to Detail", Color::DarkGray, false),
                    ("  [r] Refresh", Color::DarkGray, false),
                ]));
            }

            TableExplorerMode::ExportOptions => {
                lines.push(Line::from(Span::styled(
                    "Export Options (planned):",
                    Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[(
                    "  [c] Export as CSV",
                    Color::DarkGray,
                    false,
                )]));
                lines.push(spanned_line(&[(
                    "  [j] Export as JSON",
                    Color::DarkGray,
                    false,
                )]));
                lines.push(spanned_line(&[(
                    "  [m] Export as Markdown table",
                    Color::DarkGray,
                    false,
                )]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Export will save to the current working directory.",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[("  [Esc] Back", Color::DarkGray, false)]));
            }
            TableExplorerMode::RowInsert => {
                lines.push(Line::from(Span::styled(
                    "  Insert new row as JSON:",
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                let display = if app.explorer_insert_input.is_empty() {
                    "  Type JSON object (e.g. {\"col\": \"val\"})..."
                } else {
                    &app.explorer_insert_input
                };
                lines.push(Line::from(Span::styled(
                    display.to_string(),
                    Style::new().fg(Color::White),
                )));
                if let Some(ref err) = app.explorer_error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  Enter:save ", p.success, false),
                    ("  Esc:cancel ", Color::DarkGray, false),
                ]));
            }
            TableExplorerMode::ConfirmDelete => {
                if !app.explorer_status.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  Confirm Delete",
                        Style::new().fg(p.error).add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  {}", app.explorer_status),
                        Style::new().fg(p.warning),
                    )));
                    lines.push(Line::from(""));
                    lines.push(spanned_line(&[
                        ("  y:yes ", p.error, false),
                        ("  n:no ", p.success, false),
                        ("  Esc:cancel ", Color::DarkGray, false),
                    ]));
                }
            }
            TableExplorerMode::AnalyzeTable => {
                lines.push(Line::from(Span::styled(
                    "  Table Analysis:",
                    Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                if let Some(ref result) = app.explorer_analyze_result {
                    for line_str in result.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", line_str),
                            Style::new().fg(Color::White),
                        )));
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "  Analyzing...",
                        Style::new().fg(Color::Gray),
                    )));
                }
                if let Some(ref err) = app.explorer_error {
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
