use crate::cli::tui::app::{MetricsLogsMode, TuiApp};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_metrics_view(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let (metrics_area, logs_area) =
        if app.metrics_logs_mode == MetricsLogsMode::Both && area.height >= 12 {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            (Some(chunks[0]), Some(chunks[1]))
        } else if app.metrics_logs_mode == MetricsLogsMode::Logs {
            (None, Some(area))
        } else {
            (Some(area), None)
        };

    let filter_bar = format!(
        " [1:Metrics/2:Logs/3:Both]  Level:{}  Module:{}",
        if app.log_level_filter.is_empty() {
            "-"
        } else {
            &app.log_level_filter
        },
        if app.log_module_filter.is_empty() {
            "-"
        } else {
            &app.log_module_filter
        },
    );

    if let Some(metrics_area) = metrics_area {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::DarkGray))
            .title(" Metrics ")
            .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(Span::styled(
            &filter_bar,
            Style::new().fg(Color::DarkGray),
        )));
        if !app.connected() {
            lines.push(Line::from(Span::styled(
                "Not connected.",
                Style::new().fg(p.error),
            )));
        } else if let Some(ref data) = app.metrics_data {
            let max_lines = metrics_area.height as usize - 4;
            for line_str in data.lines().take(max_lines) {
                lines.push(Line::from(Span::styled(
                    format!(" {}", line_str),
                    Style::new().fg(Color::White),
                )));
            }
            if data.lines().count() > max_lines {
                lines.push(Line::from(Span::styled(
                    format!("  ... ({} more)", data.lines().count() - max_lines),
                    Style::new().fg(Color::DarkGray),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "No data. Press r to refresh.",
                Style::new().fg(Color::Gray),
            )));
        }
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(block)
                .wrap(Wrap { trim: false }),
            metrics_area,
        );
    }

    if let Some(logs_area) = logs_area {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::DarkGray))
            .title(" Logs ")
            .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(Span::styled(
            &filter_bar,
            Style::new().fg(Color::DarkGray),
        )));
        if let Some(ref data) = app.logs_data {
            let filter_level = app.log_level_filter.to_lowercase();
            let filter_module = app.log_module_filter.to_lowercase();
            let max_lines = logs_area.height as usize - 4;
            let mut count = 0;
            for line_str in data.lines() {
                if !filter_level.is_empty() && !line_str.to_lowercase().contains(&filter_level) {
                    continue;
                }
                if !filter_module.is_empty() && !line_str.to_lowercase().contains(&filter_module) {
                    continue;
                }
                if count >= max_lines {
                    break;
                }
                lines.push(Line::from(Span::styled(
                    format!(" {}", line_str),
                    Style::new().fg(Color::White),
                )));
                count += 1;
            }
            if count == 0 {
                lines.push(Line::from(Span::styled(
                    "No matching log entries.",
                    Style::new().fg(Color::Gray),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "No logs. Press r to refresh.",
                Style::new().fg(Color::Gray),
            )));
            lines.push(Line::from(Span::styled(
                "  (fetches via journalctl -u primusdb)",
                Style::new().fg(Color::DarkGray),
            )));
        }
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(block)
                .wrap(Wrap { trim: false }),
            logs_area,
        );
    }
}
