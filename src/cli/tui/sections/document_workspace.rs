use crate::cli::tui::app::{DocEditorMode, TuiApp};
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_document_workspace(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let title = match app.doc_mode {
        DocEditorMode::View => " Document Editor ",
        DocEditorMode::Edit => " Document Editor — Edit ",
        DocEditorMode::Create => " Document Editor — Create ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(p.border))
        .title(title)
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() && app.doc_mode != DocEditorMode::Create {
        lines.push(Line::from(Span::styled(
            "  Not connected \u{2014} connect to a running PrimusDB server.",
            Style::new().fg(p.error),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Use :connect <url> or the command palette.",
            Style::new().fg(p.text_dim),
        )));
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }

    match app.doc_mode {
        DocEditorMode::View => {
            if app.doc_collections.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No collections available. [c] Create document collection | [r] Refresh",
                    Style::new().fg(p.text_dim),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("  {} collection(s):", app.doc_collections.len()),
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                for (i, col) in app.doc_collections.iter().enumerate() {
                    let marker = if i == app.doc_collection_selected {
                        "\u{25b8}"
                    } else {
                        " "
                    };
                    let style = if i == app.doc_collection_selected {
                        Style::new().fg(p.primary).bg(Color::DarkGray)
                    } else {
                        Style::new().fg(p.text)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("  {} {}", marker, col),
                        style,
                    )));
                }
            }

            if !app.doc_current_json.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Document Preview:",
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                for line_str in app.doc_current_json.lines() {
                    let truncated = if line_str.len() > 60 {
                        format!("{}...", &line_str[..60])
                    } else {
                        line_str.to_string()
                    };
                    lines.push(Line::from(Span::styled(
                        format!("  {}", truncated),
                        Style::new().fg(p.text),
                    )));
                }
            }

            lines.push(Line::from(""));
            lines.push(spanned_line(&[
                (" \u{2191}\u{2193} select ", p.text_dim, false),
                (" e:edit ", p.success, false),
                (" c:create ", p.warning, false),
                (" v:validate ", p.primary, false),
                (" r:refresh ", p.primary, false),
            ]));
        }
        DocEditorMode::Edit | DocEditorMode::Create => {
            let display = if app.doc_edit_buffer.is_empty() {
                if app.doc_mode == DocEditorMode::Create {
                    "Type JSON document..."
                } else {
                    "Edit JSON document..."
                }
            } else {
                &app.doc_edit_buffer
            };
            lines.push(Line::from(Span::styled(
                format!("  {}", display),
                Style::new().fg(p.text),
            )));
            if let Some(ref err) = app.doc_validation_error {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("  Validation: {}", err),
                    Style::new().fg(p.error),
                )));
            }
            lines.push(Line::from(""));
            lines.push(spanned_line(&[
                (" Enter:save ", p.success, false),
                (" v:validate ", p.primary, false),
                (" Esc:cancel ", p.text_dim, false),
            ]));
        }
    }

    if !app.doc_status.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", app.doc_status),
            Style::new().fg(p.success),
        )));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
