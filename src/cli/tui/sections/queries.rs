use crate::cli::tui::app::{QueryHistoryEntry, TuiApp};
use crate::cli::tui::widgets::{highlight_sql, spanned_line};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_queries(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(p.border))
        .title(" Queries ")
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "Not connected. Connect to an instance to run queries.",
            Style::new().fg(p.error),
        )));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  Go to ", Color::Gray, false),
            ("Instances", p.primary, true),
            (" to connect, or use ", Color::Gray, false),
            ("Dashboard", p.primary, true),
            (" to see options.", Color::Gray, false),
        ]));
    } else {
        let conn = app.connected_url.as_deref().unwrap_or("-");
        lines.push(spanned_line(&[
            ("Connected: ", p.success, false),
            (conn, p.primary, false),
        ]));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            (
                "  Type your query in the input bar below and press ",
                p.text_dim,
                false,
            ),
            ("Enter", p.text, true),
        ]));
        lines.push(Line::from(""));

        if app.query_results.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No results yet.",
                Style::new().fg(p.text_dim),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "Results:",
                Style::new().fg(p.title).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            let max_visible = (area.height as usize).saturating_sub(8);
            let total = app.query_results.len();
            let scroll = app.query_scroll.min(total.saturating_sub(max_visible));
            let visible: Vec<&String> = app
                .query_results
                .iter()
                .skip(scroll)
                .take(max_visible)
                .collect();

            for result in &visible {
                for line_str in result.lines() {
                    let mut line_spans: Vec<Span> = vec![Span::styled("  ", Style::new())];
                    line_spans.extend(highlight_sql(line_str));
                    lines.push(Line::from(line_spans));
                }
            }

            let showing = format!(
                "  Showing {}-{} of {}  (PgUp/PgDn to scroll)",
                scroll + 1,
                (scroll + visible.len()).min(total),
                total
            );
            lines.push(Line::from(Span::styled(
                showing,
                Style::new().fg(p.text_dim),
            )));
        }

        if app.show_query_history {
            if !app.query_history.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "── Query History ──",
                    Style::new().fg(p.title).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(Span::styled(
                    format!("  Search: {}", app.query_history_search),
                    Style::new().fg(p.text_dim),
                )));
                lines.push(Line::from(""));

                let search_lower = app.query_history_search.to_lowercase();
                let filtered: Vec<&QueryHistoryEntry> = app
                    .query_history
                    .iter()
                    .filter(|e| {
                        e.query.to_lowercase().contains(&search_lower)
                            || e.result.to_lowercase().contains(&search_lower)
                    })
                    .collect();

                let max_visible = (area.height as usize).saturating_sub(15);
                let start = app.query_history_selection.saturating_sub(max_visible / 2);
                let visible: Vec<&&QueryHistoryEntry> =
                    filtered.iter().skip(start).take(max_visible).collect();

                for (i, entry) in visible.iter().enumerate() {
                    let abs_idx = start + i;
                    let marker = if abs_idx == app.query_history_selection {
                        "\u{25b8}"
                    } else {
                        " "
                    };
                    let style = if abs_idx == app.query_history_selection {
                        Style::new().fg(p.primary).bg(Color::DarkGray)
                    } else {
                        Style::new().fg(p.text_dim)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("  {} [{}] {}", marker, entry.timestamp, entry.query),
                        style,
                    )));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    " Enter:load  H:close  Type to filter  \u{2191}\u{2193} navigate",
                    Style::new().fg(p.text_dim),
                )));
            } else {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  No query history yet.",
                    Style::new().fg(p.text_dim),
                )));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
