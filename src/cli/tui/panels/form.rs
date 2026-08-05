use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// A form panel with labeled input fields.
pub struct FormPanel {
    pub title: String,
    pub fields: Vec<FormField>,
    pub focused: usize,
}

pub struct FormField {
    pub label: String,
    pub value: String,
    pub placeholder: String,
    pub field_type: FieldType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Text,
    Password,
    Number,
    Boolean,
    Select(Vec<String>),
}

impl FormPanel {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            fields: Vec::new(),
            focused: 0,
        }
    }

    pub fn add_field(&mut self, label: &str, placeholder: &str, field_type: FieldType) {
        self.fields.push(FormField {
            label: label.to_string(),
            value: String::new(),
            placeholder: placeholder.to_string(),
            field_type,
        });
    }

    pub fn focus_next(&mut self) {
        if !self.fields.is_empty() {
            self.focused = (self.focused + 1) % self.fields.len();
        }
    }

    pub fn focus_prev(&mut self) {
        if !self.fields.is_empty() {
            self.focused = self.focused.saturating_sub(1);
            if self.focused == 0 && self.fields.len() > 1 {
                self.focused = self.fields.len() - 1;
            }
        }
    }

    pub fn set_value(&mut self, value: String) {
        if let Some(field) = self.fields.get_mut(self.focused) {
            field.value = value;
        }
    }

    pub fn get_values(&self) -> Vec<(&str, &str)> {
        self.fields
            .iter()
            .map(|f| (f.label.as_str(), f.value.as_str()))
            .collect()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        for (i, field) in self.fields.iter().enumerate() {
            let marker = if i == self.focused { "▸" } else { " " };
            let style = if i == self.focused {
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(Color::White)
            };
            let display_value = if field.value.is_empty() {
                &field.placeholder
            } else {
                &field.value
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {} {}: ", marker, field.label), style),
                Span::styled(display_value.as_str(), Style::new().fg(Color::DarkGray)),
            ]));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.title))
            .border_style(Style::new().fg(Color::DarkGray));

        frame.render_widget(Paragraph::new(Text::from(lines)).block(block), area);
    }
}
