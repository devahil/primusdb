use crate::cli::tui::app::{MonitoringMode, TuiApp};
use crate::cli::tui::widgets::{render_json_block, spanned_line};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_monitoring(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let title = match app.mon_mode {
        MonitoringMode::Overview => " Monitoring — Overview ",
        MonitoringMode::Alerts => " Monitoring — Alerts ",
        MonitoringMode::Performance => " Monitoring — Performance ",
        MonitoringMode::Replication => " Monitoring — Replication ",
        MonitoringMode::Resources => " Monitoring — Resources ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(p.border))
        .title(title)
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "  Not connected \u{2014} connect to a running PrimusDB server.",
            Style::new().fg(p.error),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Use :connect <url> or the command palette.",
            Style::new().fg(p.text_dim),
        )));
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }

    match app.mon_mode {
        MonitoringMode::Overview => {
            lines.push(Line::from(Span::styled(
                "  Server Status",
                Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
            )));
            if let Some(ref health) = app.health_status {
                let color = if health == "healthy" || health == "ok" {
                    p.success
                } else {
                    p.error
                };
                lines.push(Line::from(Span::styled(
                    format!("  Health: {}", health),
                    Style::new().fg(color),
                )));
            }
            if let Some(ref version) = app.server_version {
                lines.push(Line::from(Span::styled(
                    format!("  Version: {}", version),
                    Style::new().fg(p.primary),
                )));
            }
            if let Some(ref uptime) = app.uptime {
                lines.push(Line::from(Span::styled(
                    format!("  Uptime: {}", uptime),
                    Style::new().fg(p.text),
                )));
            }
            if let Some(ref qps) = app.query_rate {
                lines.push(Line::from(Span::styled(
                    format!("  Query Rate: {}/s", qps),
                    Style::new().fg(p.success),
                )));
            }
            if let Some(ref lag) = app.mon_replication_lag {
                lines.push(Line::from(Span::styled(
                    format!("  Replication Lag: {}", lag),
                    Style::new().fg(p.warning),
                )));
            }
            if !app.mon_alerts.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("  Active Alerts: {}", app.mon_alerts.len()),
                    Style::new().fg(p.error).add_modifier(Modifier::BOLD),
                )));
                for alert in app.mon_alerts.iter().take(5) {
                    if let Some(msg) = alert.get("message").and_then(|m| m.as_str()) {
                        lines.push(Line::from(Span::styled(
                            format!("  \u{26a0} {}", msg),
                            Style::new().fg(p.error),
                        )));
                    }
                }
            }
        }
        MonitoringMode::Alerts => {
            if app.mon_alerts.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No alerts.",
                    Style::new().fg(p.success),
                )));
            } else {
                for alert in &app.mon_alerts {
                    render_json_block(&mut lines, Some(alert), "");
                    lines.push(Line::from(""));
                }
            }
        }
        MonitoringMode::Performance => {
            if let Some(ref qps) = app.query_rate {
                lines.push(Line::from(Span::styled(
                    format!("  Avg Query Rate: {}/s", qps),
                    Style::new().fg(p.primary),
                )));
            }
            if let Some(ref latency) = app.mon_query_latency {
                lines.push(Line::from(Span::styled(
                    format!("  P99 Latency: {}ms", latency),
                    Style::new().fg(p.warning),
                )));
            }
            if !app.mon_metrics_history.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Recent Metrics:",
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                for (ts, val) in app.mon_metrics_history.iter().rev().take(10).rev() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}  {:.2}", ts, val),
                        Style::new().fg(p.text),
                    )));
                }
            }
        }
        MonitoringMode::Replication => {
            if let Some(ref lag) = app.mon_replication_lag {
                lines.push(Line::from(Span::styled(
                    format!("  Replication Lag: {}", lag),
                    Style::new().fg(p.warning),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "  No replication data available.",
                    Style::new().fg(p.text_dim),
                )));
            }
            if let Some(ref nodes) = app.cluster_nodes {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Cluster Nodes:",
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                render_json_block(&mut lines, Some(nodes), "");
            }
        }
        MonitoringMode::Resources => {
            if let Some(ref mem) = app.memory_usage {
                lines.push(Line::from(Span::styled(
                    format!("  Memory: {}", mem),
                    Style::new().fg(p.primary),
                )));
            }
            if let Some(ref storage) = app.storage_usage {
                lines.push(Line::from(Span::styled(
                    format!("  Storage: {}", storage),
                    Style::new().fg(p.primary),
                )));
            }
            if let Some(ref res) = app.mon_resource_util {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Resource Utilization:",
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                render_json_block(&mut lines, Some(res), "");
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(spanned_line(&[
        (" o:overview ", p.primary, false),
        (" a:alerts ", p.error, false),
        (" p:perf ", p.warning, false),
        (" r:repl ", p.accent, false),
        (" s:resources ", p.success, false),
        (" r:refresh ", p.primary, false),
    ]));

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
