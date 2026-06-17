use crate::cli::tui::app::TuiApp;
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_governor(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Resource Governor ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(Color::Red),
        )));
    } else if app.governor_status.is_none() && app.governor_executions.is_empty() {
        lines.push(Line::from(Span::styled(
            "Resource Governor: Active",
            Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No active executions.",
            Style::new().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  Press ", Color::Gray, false),
            ("s", Color::White, true),
            (" for status, ", Color::Gray, false),
            ("v", Color::White, true),
            (" for violations, ", Color::Gray, false),
            ("m", Color::White, true),
            (" for metrics", Color::Gray, false),
        ]));
    } else {
        if let Some(ref status) = app.governor_status {
            lines.push(Line::from(Span::styled(
                format!("  Status: {}", status),
                Style::new().fg(Color::Cyan),
            )));
            lines.push(Line::from(""));
        }

        if !app.governor_executions.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  Active Executions ({})", app.governor_executions.len()),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            for exec in app.governor_executions.iter().take(10) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", exec),
                    Style::new().fg(Color::White),
                )));
            }
            lines.push(Line::from(""));
        }

        if !app.governor_violations.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  Recent Violations ({})", app.governor_violations.len()),
                Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            for v in app.governor_violations.iter().take(8) {
                lines.push(Line::from(Span::styled(
                    format!("    ⚠ {}", v),
                    Style::new().fg(Color::LightRed),
                )));
            }
            lines.push(Line::from(""));
        }

        if let Some(ref metrics) = app.governor_metrics {
            lines.push(Line::from(Span::styled(
                format!("  Metrics: {}", metrics),
                Style::new().fg(Color::Green),
            )));
        }

        lines.push(spanned_line(&[
            ("  Press ", Color::Gray, false),
            ("r", Color::White, true),
            (" to refresh", Color::Gray, false),
        ]));
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
