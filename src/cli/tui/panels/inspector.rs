use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

/// A right-side inspector panel showing details of a selected object.
pub struct InspectorPanel {
    pub title: String,
    pub sections: Vec<InspectorSection>,
}

pub struct InspectorSection {
    pub title: String,
    pub fields: Vec<(String, String)>,
}

impl InspectorPanel {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            sections: Vec::new(),
        }
    }

    pub fn add_section(&mut self, title: &str, fields: Vec<(String, String)>) {
        self.sections.push(InspectorSection {
            title: title.to_string(),
            fields,
        });
    }

    pub fn clear(&mut self) {
        self.sections.clear();
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        for section in &self.sections {
            lines.push(Line::from(Span::styled(
                format!(" {} ", section.title),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            for (key, value) in &section.fields {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {}: ", key), Style::new().fg(Color::DarkGray)),
                    Span::styled(value.as_str(), Style::new().fg(Color::White)),
                ]));
            }
            lines.push(Line::from(""));
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
