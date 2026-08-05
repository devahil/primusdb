use crate::cli::tui::app::TuiApp;
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_namespaces(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Namespaces ")
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(p.error),
        )));
    } else if app.namespaces_data.is_empty() {
        lines.push(Line::from(Span::styled(
            "No namespaces found.",
            Style::new().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  n:", Color::Gray, false),
            (" create ", Color::White, true),
            ("u:", Color::Gray, false),
            (" use ", Color::White, true),
            ("d:", Color::Gray, false),
            (" delete ", Color::White, true),
            ("r:", Color::Gray, false),
            (" refresh", Color::White, true),
        ]));
    } else {
        let count_str = format!("  {} namespace(s):", app.namespaces_data.len());
        lines.push(Line::from(Span::styled(
            count_str,
            Style::new().fg(p.success).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for ns in &app.namespaces_data {
            let is_active = app.active_namespace.as_deref() == Some(ns.as_str());
            lines.push(Line::from(vec![
                Span::styled(
                    if is_active { "  ◆ " } else { "  • " },
                    Style::new().fg(if is_active {
                        p.selection
                    } else {
                        Color::DarkGray
                    }),
                ),
                Span::styled(
                    format!("{}{}", ns, if is_active { " (active)" } else { "" }),
                    Style::new()
                        .fg(if is_active { p.selection } else { Color::White })
                        .add_modifier(if is_active {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  n:", Color::Gray, false),
            (" create ", Color::White, true),
            ("u:", Color::Gray, false),
            (" use ", Color::White, true),
            ("d:", Color::Gray, false),
            (" delete ", Color::White, true),
            ("r:", Color::Gray, false),
            (" refresh", Color::White, true),
        ]));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
