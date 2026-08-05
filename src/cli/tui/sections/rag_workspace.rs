use crate::cli::tui::app::{RagWorkspaceMode, TuiApp};
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_rag_workspace(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let title = match app.rag_mode {
        RagWorkspaceMode::CollectionSelect => " RAG Workspace — Collections ",
        RagWorkspaceMode::CreateCollection => " RAG — Create Collection ",
        RagWorkspaceMode::ConfirmDelete => " RAG — Confirm Delete ",
        RagWorkspaceMode::SearchConfig => " RAG Workspace — Search Config ",
        RagWorkspaceMode::SearchResults => " RAG Workspace — Results ",
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
        match app.rag_mode {
            RagWorkspaceMode::CollectionSelect => {
                if app.rag_collections.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "No vector collections found. [n] Create collection",
                        Style::new().fg(p.text_dim),
                    )));
                } else {
                    lines.push(spanned_line(&[
                        ("  #", p.primary, true),
                        ("  Collection", p.primary, true),
                    ]));
                    lines.push(Line::from(Span::styled(
                        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                        Style::new().fg(p.border),
                    )));
                    for (i, col) in app.rag_collections.iter().enumerate() {
                        let is_selected = i == app.rag_selected_index;
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
                            Span::styled(col, style),
                        ]));
                    }
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [\u{2191}/\u{2193}] Navigate", p.text_dim, false),
                    ("  [Enter] Configure Search", p.text_dim, false),
                    ("  [n] Create Collection", p.text_dim, false),
                    ("  [d] Delete", p.text_dim, false),
                ]));
            }

            RagWorkspaceMode::CreateCollection => {
                lines.push(Line::from(Span::styled(
                    "Enter collection name:",
                    Style::new().fg(p.text_dim),
                )));
                lines.push(Line::from(""));
                let display = if app.rag_input.is_empty() {
                    "(type collection name)".to_string()
                } else {
                    app.rag_input.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("  Name: {}", display),
                    Style::new().fg(if app.rag_input.is_empty() {
                        p.text_dim
                    } else {
                        p.warning
                    }),
                )));
                if let Some(ref err) = app.rag_error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[(
                    "  [Enter] Create  [Esc] Cancel",
                    p.text_dim,
                    false,
                )]));
            }

            RagWorkspaceMode::ConfirmDelete => {
                let collection = app
                    .rag_collections
                    .get(app.rag_selected_index)
                    .map(|s| s.as_str())
                    .unwrap_or("?");
                lines.push(Line::from(Span::styled(
                    format!("Delete collection \"{}\"?", collection),
                    Style::new().fg(p.text),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[(
                    "  [y] Yes  [n] No  [Esc] Cancel",
                    p.text_dim,
                    false,
                )]));
            }

            RagWorkspaceMode::SearchConfig => {
                let collection = app
                    .rag_collections
                    .get(app.rag_selected_index)
                    .map(|s| s.as_str())
                    .unwrap_or("?");
                lines.push(Line::from(vec![
                    Span::styled("Collection: ", Style::new().fg(p.text_dim)),
                    Span::styled(
                        collection,
                        Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Enter your search query and press Enter to execute:",
                    Style::new().fg(p.text_dim),
                )));
                lines.push(Line::from(""));
                let display = if app.rag_query_text.is_empty() {
                    "(type query text)".to_string()
                } else {
                    app.rag_query_text.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("  Query: {}", display),
                    Style::new().fg(if app.rag_query_text.is_empty() {
                        p.text_dim
                    } else {
                        p.warning
                    }),
                )));
                lines.push(Line::from(Span::styled(
                    format!("  Top-K: {}", app.rag_limit),
                    Style::new().fg(p.text_dim),
                )));
                if let Some(ref err) = app.rag_error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  Error: {}", err),
                        Style::new().fg(p.error),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  [Enter] Execute Search", p.text_dim, false),
                    ("  [+/-] Adjust Top-K", p.text_dim, false),
                    ("  [Esc] Back", p.text_dim, false),
                ]));
            }

            RagWorkspaceMode::SearchResults => {
                if let Some(ref data) = app.rag_results {
                    if let Some(rows) = data.get("rows").and_then(|v| v.as_array()) {
                        lines.push(Line::from(Span::styled(
                            format!("  {} result(s) found", rows.len()),
                            Style::new().fg(p.primary),
                        )));
                        lines.push(Line::from(Span::styled(
                            "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                            Style::new().fg(p.border),
                        )));
                        for row in rows {
                            let row_str = serde_json::to_string(row).unwrap_or_default();
                            let truncated = if row_str.len() > area.width as usize - 6 {
                                format!("{}...", &row_str[..area.width as usize - 9])
                            } else {
                                row_str
                            };
                            lines.push(Line::from(Span::styled(
                                format!("  {}", truncated),
                                Style::new().fg(p.text),
                            )));
                        }
                    } else {
                        let pretty = serde_json::to_string_pretty(data).unwrap_or_default();
                        for line_str in pretty.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("  {}", line_str),
                                Style::new().fg(p.text),
                            )));
                        }
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "Running search...".to_string(),
                        Style::new().fg(p.text_dim),
                    )));
                }
                if let Some(ref err) = app.rag_error {
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
