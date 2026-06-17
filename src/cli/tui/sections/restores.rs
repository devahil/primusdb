use crate::cli::tui::app::TuiApp;
use crate::cli::tui::widgets::{render_json_block, spanned_line};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_restores(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Restores ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(Color::Red),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Available Backups for Restore:",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));

        if app.backups_data.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No backups available.",
                Style::new().fg(Color::Gray),
            )));
        } else {
            lines.push(spanned_line(&[
                ("  Type", Color::Yellow, true),
                ("  Size", Color::Yellow, true),
                ("  Name / ID", Color::Yellow, true),
            ]));
            lines.push(Line::from(Span::styled(
                "  ─────────────────────────────────────────────",
                Style::new().fg(Color::DarkGray),
            )));
            for entry in &app.backups_data {
                lines.push(Line::from(Span::styled(
                    format!("  {}", entry),
                    Style::new().fg(Color::White),
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Backup Details:",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        render_json_block(
            &mut lines,
            app.backups_detail.as_ref(),
            "No backup detail metadata.",
        );

        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  Restore via CLI: ", Color::Gray, false),
            ("primusdb backup restore <id>", Color::Cyan, false),
        ]));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
