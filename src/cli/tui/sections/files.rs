use crate::cli::tui::app::{FileBrowserMode, TuiApp};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render_file_browser(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray));

    match app.file_mode {
        FileBrowserMode::Browse => {
            let title = format!(" File Browser — {} ", app.file_current_dir);
            let block = block
                .title(title)
                .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

            let items: Vec<ListItem> = app
                .file_entries
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let style = if entry.starts_with("[DIR]") {
                        Style::new().fg(p.primary)
                    } else {
                        Style::new().fg(Color::White)
                    };
                    let prefix = if i == app.file_selected_index {
                        "▸ "
                    } else {
                        "  "
                    };
                    ListItem::new(Line::from(Span::styled(
                        format!("{}{}", prefix, entry),
                        style,
                    )))
                })
                .collect();

            let list = List::new(items).block(block).highlight_style(
                Style::new()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(list, area);
        }
        FileBrowserMode::ReadFile => {
            let title = format!(" File: {} ", app.file_selected_path);
            let block = block
                .title(title)
                .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));
            let content = app.file_content.as_deref().unwrap_or("(empty or binary)");
            let lines: Vec<Line> = content.lines().map(|l| Line::from(Span::raw(l))).collect();
            let paragraph = Paragraph::new(Text::from(lines))
                .block(block)
                .scroll((app.file_scroll as u16, 0));
            frame.render_widget(paragraph, area);
        }
    }
}
