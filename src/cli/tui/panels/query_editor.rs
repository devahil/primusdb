use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub struct QueryEditor {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    pub modified: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum CursorDir {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
}

impl Default for QueryEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryEditor {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            modified: false,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            line.insert(self.cursor_col, c);
            self.cursor_col += 1;
            self.modified = true;
        }
    }

    pub fn delete_char(&mut self) {
        if self.cursor_col > 0 {
            if let Some(line) = self.lines.get_mut(self.cursor_row) {
                self.cursor_col -= 1;
                line.remove(self.cursor_col);
                self.modified = true;
            }
        } else if self.cursor_row > 0 {
            let rem = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&rem);
            self.modified = true;
        }
    }

    pub fn insert_newline(&mut self) {
        let col = self.cursor_col;
        if let Some(line) = self.lines.get(self.cursor_row) {
            let rem: String = line[col..].to_string();
            self.lines[self.cursor_row].truncate(col);
            self.cursor_row += 1;
            self.cursor_col = 0;
            self.lines.insert(self.cursor_row, rem);
            self.modified = true;
        }
    }

    pub fn move_cursor(&mut self, d: CursorDir) {
        match d {
            CursorDir::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.lines[self.cursor_row].len();
                }
            }
            CursorDir::Right => {
                if self.cursor_col < self.lines[self.cursor_row].len() {
                    self.cursor_col += 1;
                } else if self.cursor_row + 1 < self.lines.len() {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                }
            }
            CursorDir::Up => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                }
            }
            CursorDir::Down => {
                if self.cursor_row + 1 < self.lines.len() {
                    self.cursor_row += 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                }
            }
            CursorDir::Home => self.cursor_col = 0,
            CursorDir::End => self.cursor_col = self.lines[self.cursor_row].len(),
        }
    }

    pub fn get_text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_text(&mut self, text: &str) {
        self.lines = text.lines().map(|l| l.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.modified = false;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        let vis_end = (self.scroll_offset + area.height as usize).min(self.lines.len());
        for (i, line) in self
            .lines
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(vis_end.saturating_sub(self.scroll_offset))
        {
            let mut spans = vec![Span::styled(
                format!("{:>3} ", i + 1),
                Style::new().fg(Color::DarkGray),
            )];
            if line.is_empty() {
                spans.push(Span::styled(" ", Style::new()));
            } else {
                spans.extend(crate::cli::tui::widgets::highlight_sql(line));
            }
            lines.push(Line::from(spans));
        }
        let title = if self.modified {
            " Query Editor * "
        } else {
            " Query Editor "
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::new().fg(Color::DarkGray));
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}
