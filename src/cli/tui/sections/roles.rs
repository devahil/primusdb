use crate::cli::tui::app::TuiApp;
use crate::cli::tui::widgets::render_json_block;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_roles(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Roles ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(ratatui::text::Span::styled(
            "Not connected. Connect to an instance first.",
            ratatui::style::Style::new().fg(Color::Red),
        )));
    } else {
        lines.push(Line::from(ratatui::text::Span::styled(
            "Roles:",
            ratatui::style::Style::new()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        render_json_block(
            &mut lines,
            app.roles_data.as_ref(),
            "No roles data. Press r to refresh.",
        );
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
