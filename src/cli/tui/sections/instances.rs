use crate::cli::tui::app::TuiApp;
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_instances(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Instances ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if app.instances.is_empty() {
        lines.push(Line::from(Span::styled(
            "No PrimusDB instances discovered.",
            Style::new().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "To start a server:",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        lines.push(spanned_line(&[
            ("  ", Color::Reset, false),
            ("primusdb server start", Color::Cyan, false),
        ]));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  Press ", Color::Gray, false),
            ("r", Color::White, true),
            (" to refresh discovery", Color::Gray, false),
        ]));
    } else {
        let count_str = format!("  {} instance(s) found:", app.instances.len());
        lines.push(Line::from(Span::styled(
            count_str,
            Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for (i, inst) in app.instances.iter().enumerate() {
            let is_selected = i == app.selected_instance;
            let prefix = if is_selected { "▸ " } else { "  " };
            let is_connected = app
                .connected_url
                .as_ref()
                .is_some_and(|u| *u == inst.endpoint);
            let conn_mark = if is_connected { " ●" } else { "" };
            let status_color = match inst.status.as_str() {
                "healthy" | "ok" => Color::Green,
                _ => Color::Red,
            };
            let endpoint_fg = if is_connected {
                Color::Green
            } else {
                Color::Cyan
            };
            let line_style = if is_selected {
                Style::new().bg(Color::DarkGray)
            } else {
                Style::new()
            };
            lines.push(
                Line::from(vec![
                    Span::styled(
                        format!("{}{}", prefix, inst.endpoint),
                        Style::new().fg(endpoint_fg),
                    ),
                    Span::styled(format!(" [{}]", inst.status), Style::new().fg(status_color)),
                    Span::styled(
                        conn_mark,
                        Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                ])
                .style(line_style),
            );
            let ver = inst.version.as_deref().unwrap_or("-");
            lines.push(Line::from(Span::styled(
                format!("    v{}  engines: {:?}", ver, inst.enabled_engines),
                Style::new().fg(Color::DarkGray),
            )));
        }

        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  ↑↓ Select  ", Color::Gray, false),
            ("Enter", Color::White, true),
            (" connect  ", Color::Gray, false),
            ("r", Color::White, true),
            (" refresh", Color::Gray, false),
        ]));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
