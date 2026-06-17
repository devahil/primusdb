use crate::cli::tui::app::TuiApp;
use crate::cli::tui::widgets::{render_json_block, spanned_line};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_nodes(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Nodes ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance first.",
            Style::new().fg(Color::Red),
        )));
    } else if let Some(ref nodes_val) = app.cluster_nodes {
        if let Some(nodes_arr) = nodes_val.as_array() {
            if nodes_arr.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No nodes found.",
                    Style::new().fg(Color::Gray),
                )));
            } else {
                let count_str = format!("  {} node(s):", nodes_arr.len());
                lines.push(Line::from(Span::styled(
                    count_str,
                    Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(spanned_line(&[
                    ("  ID", Color::Cyan, true),
                    ("  Role", Color::Cyan, true),
                    ("  Status", Color::Cyan, true),
                    ("  Address", Color::Cyan, true),
                ]));
                lines.push(Line::from(Span::styled(
                    "  ───────────────────────────────────────────",
                    Style::new().fg(Color::DarkGray),
                )));

                for node in nodes_arr {
                    let id = node
                        .get("id")
                        .or(node.get("node_id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");
                    let role = node.get("role").and_then(|v| v.as_str()).unwrap_or("-");
                    let status = node
                        .get("status")
                        .or(node.get("health"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");
                    let addr = node
                        .get("address")
                        .or(node.get("addr"))
                        .or(node.get("endpoint"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");

                    let status_color = match status {
                        "healthy" | "ok" | "online" | "active" | "leader" => Color::Green,
                        "warning" | "degraded" => Color::Yellow,
                        "error" | "down" | "offline" => Color::Red,
                        _ => Color::White,
                    };

                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}", id), Style::new().fg(Color::White)),
                        Span::styled(format!("  {}", role), Style::new().fg(Color::DarkGray)),
                        Span::styled(
                            format!("  {}", status),
                            Style::new().fg(status_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(format!("  {}", addr), Style::new().fg(Color::Cyan)),
                    ]));
                }
            }
        } else {
            render_json_block(&mut lines, Some(nodes_val), "No node data available.");
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  No node data. Press r to refresh.",
            Style::new().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  Press ", Color::Gray, false),
            ("r", Color::White, true),
            (" to refresh", Color::Gray, false),
        ]));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
