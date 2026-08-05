use crate::cli::tui::app::{NotebookBuilderMode, TuiApp};
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_notebook(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let title = match app.notebook_mode {
        NotebookBuilderMode::List => " Notebook ",
        NotebookBuilderMode::Detail => " Notebook Detail ",
        NotebookBuilderMode::CellEdit => " Edit Cell ",
        NotebookBuilderMode::CellTypeSelect => " Cell Type ",
        NotebookBuilderMode::ConfirmDelete => " Confirm Delete ",
        NotebookBuilderMode::Results => " Cell Result ",
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
        match app.notebook_mode {
            NotebookBuilderMode::List => {
                if app.notebooks_data.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "No notebooks yet. Press 'n' to create one (type the name and Enter).",
                        Style::new().fg(p.text_dim),
                    )));
                } else {
                    lines.push(spanned_line(&[
                        ("  #", p.primary, true),
                        ("  Name", p.primary, true),
                        ("  Cells", p.primary, true),
                    ]));
                    lines.push(Line::from(Span::styled(
                        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                        Style::new().fg(p.border),
                    )));
                    for (i, nb) in app.notebooks_data.iter().enumerate() {
                        let name = nb
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string();
                        let cell_count = nb
                            .get("cells")
                            .and_then(|c| c.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);
                        let is_selected = i == app.notebook_selected_index;
                        let prefix = if is_selected { "\u{25b8} " } else { "  " };
                        let style = if is_selected {
                            Style::new()
                                .fg(p.text)
                                .bg(Color::Blue)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::new().fg(p.text)
                        };
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("{}{:<3}", prefix, i + 1),
                                Style::new().fg(p.text_dim),
                            ),
                            Span::styled(name, style),
                            Span::styled(
                                format!("  {} cell(s)", cell_count),
                                Style::new().fg(p.primary),
                            ),
                        ]));
                    }
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [\u{2191}/\u{2193}] Navigate", p.text_dim, false),
                    ("  [Enter] Open", p.text_dim, false),
                    ("  [n] New", p.text_dim, false),
                    ("  [d] Delete", p.text_dim, false),
                    ("  [r] Refresh", p.text_dim, false),
                ]));
            }

            NotebookBuilderMode::Detail => {
                if let Some(ref nb) = app.notebook_detail {
                    let name = nb.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    lines.push(Line::from(vec![
                        Span::styled("  Notebook: ", Style::new().fg(p.text_dim)),
                        Span::styled(name, Style::new().fg(p.text).add_modifier(Modifier::BOLD)),
                    ]));
                    lines.push(Line::from(Span::styled(
                        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                        Style::new().fg(p.border),
                    )));

                    if app.notebook_cells.is_empty() {
                        lines.push(Line::from(Span::styled(
                            "  (no cells \u{2014} press 'n' to add one)",
                            Style::new().fg(p.text_dim),
                        )));
                    } else {
                        for (i, cell) in app.notebook_cells.iter().enumerate() {
                            let ct = cell.get("type").and_then(|v| v.as_str()).unwrap_or("md");
                            let content =
                                cell.get("content").and_then(|v| v.as_str()).unwrap_or("");
                            let is_sel = i == app.notebook_selected_cell;
                            let prefix = if is_sel { "\u{25b8} " } else { "  " };
                            let cell_style = if is_sel {
                                Style::new().fg(p.text).bg(Color::Blue)
                            } else {
                                Style::new().fg(p.text)
                            };
                            let display = if content.len() > 60 {
                                format!("{}...", &content[..57])
                            } else if content.is_empty() {
                                "(empty)".to_string()
                            } else {
                                content.to_string()
                            };
                            let type_color = match ct {
                                "md" => p.success,
                                "sql" => p.warning,
                                "analysis" => p.accent,
                                "rag" => p.primary,
                                _ => p.text_dim,
                            };
                            lines.push(Line::from(vec![
                                Span::styled(format!("{}[", prefix), cell_style),
                                Span::styled(ct, type_color),
                                Span::styled("] ", cell_style),
                                Span::styled(display, cell_style),
                            ]));
                        }
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "Loading notebook...",
                        Style::new().fg(p.text_dim),
                    )));
                }

                if let Some(ref err) = app.notebook_error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }

                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [\u{2191}/\u{2193}] Navigate Cells", p.text_dim, false),
                    ("  [Enter] Edit", p.text_dim, false),
                    ("  [e] Execute", p.text_dim, false),
                    ("  [t] Type", p.text_dim, false),
                    ("  [s] Save", p.text_dim, false),
                    ("  [n] New Cell", p.text_dim, false),
                    ("  [x] Delete Cell", p.text_dim, false),
                ]));
                lines.push(spanned_line(&[("  [Esc] Back", p.text_dim, false)]));
            }

            NotebookBuilderMode::CellEdit => {
                lines.push(Line::from(Span::styled(
                    "Edit cell content:",
                    Style::new().fg(p.text_dim),
                )));
                lines.push(Line::from(""));

                let display = if app.notebook_cell_edit.is_empty() {
                    "(type cell content)".to_string()
                } else {
                    app.notebook_cell_edit.clone()
                };
                lines.push(Line::from(Span::styled(
                    display,
                    Style::new().fg(if app.notebook_cell_edit.is_empty() {
                        p.text_dim
                    } else {
                        p.warning
                    }),
                )));

                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [Enter] Save Cell", p.text_dim, false),
                    ("  [Esc] Cancel", p.text_dim, false),
                ]));
            }

            NotebookBuilderMode::CellTypeSelect => {
                lines.push(Line::from(Span::styled(
                    "Select cell type:",
                    Style::new().fg(p.text_dim),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("  [m] Markdown", p.success)));
                lines.push(Line::from(Span::styled("  [s] SQL", p.warning)));
                lines.push(Line::from(Span::styled("  [a] Analysis", p.accent)));
                lines.push(Line::from(Span::styled("  [r] RAG", p.primary)));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[("  [Esc] Cancel", p.text_dim, false)]));
            }

            NotebookBuilderMode::ConfirmDelete => {
                lines.push(Line::from(Span::styled(
                    "Are you sure you want to delete this notebook?",
                    Style::new().fg(p.text),
                )));
                if let Some(nb) = app.notebooks_data.get(app.notebook_selected_index) {
                    if let Some(name) = nb.get("name").and_then(|v| v.as_str()) {
                        lines.push(Line::from(""));
                        lines.push(Line::from(vec![
                            Span::styled("Notebook: ", Style::new().fg(p.text_dim)),
                            Span::styled(
                                name,
                                Style::new().fg(p.error).add_modifier(Modifier::BOLD),
                            ),
                        ]));
                    }
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [", p.text_dim, false),
                    ("Y", p.success, true),
                    ("/", p.text_dim, false),
                    ("Enter", p.success, true),
                    ("] Yes  [", p.text_dim, false),
                    ("N", p.error, true),
                    ("/", p.text_dim, false),
                    ("Esc", p.error, true),
                    ("] No", p.text_dim, false),
                ]));
            }

            NotebookBuilderMode::Results => {
                if let Some(ref data) = app.notebook_cell_result {
                    if let Some(cell_type) = data.get("cell_type").and_then(|v| v.as_str()) {
                        lines.push(Line::from(vec![
                            Span::styled("Cell type: ", Style::new().fg(p.text_dim)),
                            Span::styled(cell_type, Style::new().fg(p.primary)),
                        ]));
                        lines.push(Line::from(""));
                    }
                    if let Some(result) = data.get("result") {
                        match result {
                            serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                                let pretty =
                                    serde_json::to_string_pretty(result).unwrap_or_default();
                                for line_str in pretty.lines() {
                                    let truncated = if line_str.len() > area.width as usize - 4 {
                                        format!("{}...", &line_str[..area.width as usize - 7])
                                    } else {
                                        line_str.to_string()
                                    };
                                    lines.push(Line::from(Span::styled(
                                        truncated,
                                        Style::new().fg(p.text),
                                    )));
                                }
                            }
                            _ => {
                                let s = result.as_str().unwrap_or("");
                                for line_str in s.lines() {
                                    lines.push(Line::from(Span::styled(
                                        format!("  {}", line_str),
                                        Style::new().fg(p.text),
                                    )));
                                }
                            }
                        }
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "Executing cell...",
                        Style::new().fg(p.text_dim),
                    )));
                }

                if let Some(ref err) = app.notebook_error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }

                lines.push(Line::from(""));
                lines.push(spanned_line(&[("  [Esc] Back", p.text_dim, false)]));
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
