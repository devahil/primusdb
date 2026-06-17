use ratatui::layout::Rect;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_loading(frame: &mut Frame, area: Rect, message: &str) {
    let spinner = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        / 100)
        % spinner.len() as u128;
    let spin_char = spinner.chars().nth(idx as usize).unwrap_or(' ');
    let text = Text::from(Line::from(vec![
        Span::styled(
            format!(" {} ", spin_char),
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(message, Style::new().fg(Color::Cyan)),
    ]));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Loading ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center),
        area,
    );
}

pub fn render_json_block(lines: &mut Vec<Line>, data: Option<&serde_json::Value>, empty_msg: &str) {
    if let Some(v) = data {
        let pretty = serde_json::to_string_pretty(v).unwrap_or_default();
        for line_str in pretty.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", line_str),
                Style::new().fg(Color::White),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            format!("  {}", empty_msg),
            Style::new().fg(Color::Gray),
        )));
    }
}

pub fn render_gauge(lines: &mut Vec<Line>, label: &str, pct: u8, color: Color) {
    let bar_width = 20;
    let filled = ((pct as u16) * bar_width / 100).min(bar_width);
    let empty = bar_width.saturating_sub(filled);
    let bar: String = format!(
        "|{}{}| {:3}%",
        "█".repeat(filled as usize),
        "░".repeat(empty as usize),
        pct
    );
    lines.push(Line::from(vec![
        Span::styled(format!("  {}: ", label), Style::new().fg(Color::Gray)),
        Span::styled(bar, Style::new().fg(color)),
    ]));
}

pub fn render_progress_bar(lines: &mut Vec<Line>, pct: u8) {
    let bar_width = 30;
    let filled = ((pct as u16) * bar_width / 100).min(bar_width);
    let empty = bar_width.saturating_sub(filled);
    let bar = format!(
        "[{}{}] {:3}%",
        "█".repeat(filled as usize),
        "░".repeat(empty as usize),
        pct
    );
    lines.push(Line::from(Span::styled(
        format!("  {}", bar),
        Style::new().fg(Color::Cyan),
    )));
}

pub fn spanned_line<'a>(parts: &[(&'a str, Color, bool)]) -> Line<'a> {
    let spans: Vec<Span> = parts
        .iter()
        .map(|(text, color, bold)| {
            let mut style = Style::new().fg(*color);
            if *bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            Span::styled(*text, style)
        })
        .collect();
    Line::from(spans)
}
