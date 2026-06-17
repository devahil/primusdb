use crate::cli::tui::app::TuiApp;
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_vector_indexes(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Vector Indexes ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(Color::Red),
        )));
    } else if app.vector_indexes_data.is_empty() {
        lines.push(Line::from(Span::styled(
            "No vector indexes found.",
            Style::new().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  Press ", Color::Gray, false),
            ("r", Color::White, true),
            (" to refresh", Color::Gray, false),
        ]));
    } else {
        let count_str = format!("  {} vector index(es):", app.vector_indexes_data.len());
        lines.push(Line::from(Span::styled(
            count_str,
            Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for idx in &app.vector_indexes_data {
            lines.push(Line::from(vec![
                Span::styled("  ◆ ", Style::new().fg(Color::Magenta)),
                Span::styled(idx, Style::new().fg(Color::White)),
            ]));
        }
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
