use crate::cli::tui::app::TuiApp;
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_diagnostics(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Diagnostics ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(Color::Red),
        )));
    } else if let Some(ref data) = app.diagnostics_data {
        for line_str in data.lines() {
            lines.push(Line::from(Span::styled(
                format!(" {}", line_str),
                Style::new().fg(Color::White),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No diagnostics data. Press r to refresh.",
            Style::new().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  Fetches ", Color::Gray, false),
            ("/health", Color::Cyan, false),
            (" and ", Color::Gray, false),
            ("/status", Color::Cyan, false),
            (" endpoints.", Color::Gray, false),
        ]));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
