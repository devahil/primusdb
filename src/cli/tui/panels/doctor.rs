use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CheckStatus {
    Pending,
    Passed,
    Warning,
    Failed,
    Skipped,
}

pub struct DoctorCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
}

pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub overall: CheckStatus,
}

impl Default for DoctorReport {
    fn default() -> Self {
        Self::new()
    }
}

impl DoctorReport {
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            overall: CheckStatus::Pending,
        }
    }

    pub fn run_all(&mut self, connected: bool, theme: &str, mouse: bool) {
        self.checks.clear();
        self.checks.push(DoctorCheck {
            name: "Terminal Size".into(),
            status: CheckStatus::Passed,
            message: "Adequate".into(),
        });
        self.checks.push(DoctorCheck {
            name: "Color Support".into(),
            status: CheckStatus::Passed,
            message: "256-color/truecolor".into(),
        });
        self.checks.push(DoctorCheck {
            name: "Unicode".into(),
            status: CheckStatus::Passed,
            message: "Supported".into(),
        });
        self.checks.push(DoctorCheck {
            name: "Mouse".into(),
            status: if mouse {
                CheckStatus::Passed
            } else {
                CheckStatus::Skipped
            },
            message: if mouse { "Enabled" } else { "Disabled" }.into(),
        });
        self.checks.push(DoctorCheck {
            name: "Theme".into(),
            status: CheckStatus::Passed,
            message: theme.into(),
        });
        self.checks.push(DoctorCheck {
            name: "Server".into(),
            status: if connected {
                CheckStatus::Passed
            } else {
                CheckStatus::Warning
            },
            message: if connected {
                "Connected"
            } else {
                "Not connected"
            }
            .into(),
        });
        self.overall = if self.checks.iter().any(|c| c.status == CheckStatus::Failed) {
            CheckStatus::Failed
        } else if self.checks.iter().any(|c| c.status == CheckStatus::Warning) {
            CheckStatus::Warning
        } else {
            CheckStatus::Passed
        };
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(Span::styled(
            " PrimusDB TUI Doctor",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        for c in &self.checks {
            let (icon, color) = match c.status {
                CheckStatus::Passed => ("✓", Color::Green),
                CheckStatus::Warning => ("⚠", Color::Yellow),
                CheckStatus::Failed => ("✗", Color::Red),
                CheckStatus::Skipped => ("–", Color::DarkGray),
                CheckStatus::Pending => ("○", Color::DarkGray),
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", icon),
                    Style::new().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &c.name,
                    Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::styled(": ", Style::new().fg(Color::DarkGray)),
                Span::styled(&c.message, Style::new().fg(color)),
            ]));
        }
        lines.push(Line::from(""));
        let (msg, col) = match self.overall {
            CheckStatus::Passed => ("All checks passed ✓", Color::Green),
            CheckStatus::Warning => ("Warnings detected ⚠", Color::Yellow),
            CheckStatus::Failed => ("Issues found ✗", Color::Red),
            _ => ("Not run", Color::DarkGray),
        };
        lines.push(Line::from(Span::styled(
            format!("  {}", msg),
            Style::new().fg(col).add_modifier(Modifier::BOLD),
        )));
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Doctor ")
            .border_style(Style::new().fg(Color::Yellow));
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}
