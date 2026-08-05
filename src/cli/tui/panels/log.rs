use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

/// A log streaming panel with level filtering and scrolling.
pub struct LogPanel {
    pub title: String,
    pub entries: Vec<LogEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub level_filter: LogLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    All,
    Info,
    Warn,
    Error,
    Debug,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub module: String,
    pub message: String,
}

impl LogPanel {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            level_filter: LogLevel::All,
        }
    }

    pub fn add_entry(&mut self, entry: LogEntry) {
        self.entries.push(entry);
    }

    pub fn filtered_entries(&self) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| match self.level_filter {
                LogLevel::All => true,
                LogLevel::Info => e.level == LogLevel::Info,
                LogLevel::Warn => e.level == LogLevel::Warn,
                LogLevel::Error => e.level == LogLevel::Error,
                LogLevel::Debug => e.level == LogLevel::Debug,
            })
            .collect()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let filtered = self.filtered_entries();
        let mut lines: Vec<Line> = Vec::new();

        for entry in filtered.iter().skip(self.scroll_offset) {
            let level_color = match entry.level {
                LogLevel::Info => Color::Green,
                LogLevel::Warn => Color::Yellow,
                LogLevel::Error => Color::Red,
                LogLevel::Debug => Color::DarkGray,
                LogLevel::All => Color::White,
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::new().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:5} ", entry.level_str()),
                    Style::new().fg(level_color),
                ),
                Span::styled(
                    format!("{:12} ", entry.module),
                    Style::new().fg(Color::Cyan),
                ),
                Span::styled(&entry.message, Style::new().fg(Color::White)),
            ]));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.title))
            .border_style(Style::new().fg(Color::DarkGray));

        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

impl LogEntry {
    pub fn level_str(&self) -> &str {
        match self.level {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Debug => "DEBUG",
            LogLevel::All => "ALL",
        }
    }
}
