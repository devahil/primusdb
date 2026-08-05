use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

/// A reusable table panel with selection, scrolling, and sorting.
pub struct TablePanel {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub title: String,
    pub column_widths: Option<Vec<u16>>,
}

impl TablePanel {
    pub fn new(title: &str, headers: Vec<String>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            title: title.to_string(),
            column_widths: None,
        }
    }

    pub fn set_data(&mut self, rows: Vec<Vec<String>>) {
        self.rows = rows;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if !self.rows.is_empty() {
            self.selected = (self.selected + 1).min(self.rows.len() - 1);
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        if !self.rows.is_empty() {
            self.selected = (self.selected + page_size).min(self.rows.len() - 1);
        }
    }

    pub fn selected_item(&self) -> Option<&Vec<String>> {
        self.rows.get(self.selected)
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let header_cells = self.headers.iter().map(|h| {
            Cell::from(Span::styled(
                h.as_str(),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ))
        });
        let header = Row::new(header_cells).height(1);

        let rows = self.rows.iter().map(|row| {
            let cells = row.iter().map(|cell| Cell::from(cell.as_str()));
            Row::new(cells)
        });

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.title));

        let num_cols = self.headers.len();
        let col_widths: Vec<Constraint> = if let Some(ref widths) = self.column_widths {
            widths.iter().map(|w| Constraint::Length(*w)).collect()
        } else {
            (0..num_cols)
                .map(|_| Constraint::Percentage((100 / num_cols as u16).max(1)))
                .collect()
        };

        let table = Table::new(rows, col_widths)
            .header(header)
            .block(block)
            .highlight_style(
                Style::new()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");

        let mut state = TableState::default();
        state.select(Some(self.selected));

        frame.render_stateful_widget(table, area, &mut state);
    }
}
