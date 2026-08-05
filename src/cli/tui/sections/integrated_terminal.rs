use crate::cli::tui::app::TuiApp;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_integrated_terminal(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(p.border))
        .title(" Terminal ")
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    let max_lines = area.height.saturating_sub(6) as usize;
    let start = app.terminal_output.len().saturating_sub(max_lines);
    for line_str in app.terminal_output.iter().skip(start) {
        let style = if line_str.contains("Error") || line_str.contains("error") {
            Style::new().fg(p.error)
        } else if line_str.starts_with("$") {
            Style::new().fg(p.success).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(p.text)
        };
        lines.push(Line::from(Span::styled(format!("  {}", line_str), style)));
    }

    lines.push(Line::from(""));
    let prompt = format!("  $ {}", app.terminal_input);
    lines.push(Line::from(Span::styled(
        prompt,
        Style::new().fg(p.success).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "  Enter:execute  \u{2191}\u{2193}:history  Tab:complete  Esc:clear",
        Style::new().fg(p.text_dim),
    )));

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
