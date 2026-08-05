use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub struct OnboardingWizard {
    pub step: WizardStep,
    pub server_url: String,
    pub theme_choice: usize,
    pub mouse_enabled: bool,
    pub completed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WizardStep {
    Welcome,
    TerminalCheck,
    ServerConnect,
    ThemeSelect,
    MouseSetup,
    Complete,
}

impl Default for OnboardingWizard {
    fn default() -> Self {
        Self::new()
    }
}

impl OnboardingWizard {
    pub fn new() -> Self {
        Self {
            step: WizardStep::Welcome,
            server_url: "http://localhost:8080".into(),
            theme_choice: 0,
            mouse_enabled: true,
            completed: false,
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;
        match self.step {
            WizardStep::Welcome => match key {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.step = WizardStep::TerminalCheck;
                }
                KeyCode::Esc => {
                    self.completed = true;
                }
                _ => {}
            },
            WizardStep::TerminalCheck => match key {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.step = WizardStep::ServerConnect;
                }
                KeyCode::Esc => {
                    self.completed = true;
                }
                _ => {}
            },
            WizardStep::ServerConnect => match key {
                KeyCode::Enter => {
                    self.step = WizardStep::ThemeSelect;
                }
                KeyCode::Esc => {
                    self.completed = true;
                }
                _ => {}
            },
            WizardStep::ThemeSelect => match key {
                KeyCode::Char('1') => {
                    self.theme_choice = 0;
                    self.step = WizardStep::MouseSetup;
                }
                KeyCode::Char('2') => {
                    self.theme_choice = 1;
                    self.step = WizardStep::MouseSetup;
                }
                KeyCode::Char('3') => {
                    self.theme_choice = 2;
                    self.step = WizardStep::MouseSetup;
                }
                KeyCode::Esc => {
                    self.completed = true;
                }
                _ => {}
            },
            WizardStep::MouseSetup => match key {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.mouse_enabled = true;
                    self.step = WizardStep::Complete;
                    self.completed = true;
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.mouse_enabled = false;
                    self.step = WizardStep::Complete;
                    self.completed = true;
                }
                KeyCode::Esc => {
                    self.completed = true;
                }
                _ => {}
            },
            WizardStep::Complete => {}
        }
        self.completed
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        match self.step {
            WizardStep::Welcome => {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Welcome to PrimusDB Terminal IDE",
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  This wizard will help you configure your terminal.",
                    Style::new().fg(Color::White),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Press Enter to continue or Esc to skip.",
                    Style::new().fg(Color::DarkGray),
                )));
            }
            WizardStep::TerminalCheck => {
                lines.push(Line::from(Span::styled(
                    "  Terminal Detection",
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  Color: ", Style::new().fg(Color::Gray)),
                    Span::styled("Supported", Style::new().fg(Color::Green)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  Unicode: ", Style::new().fg(Color::Gray)),
                    Span::styled("Supported", Style::new().fg(Color::Green)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  Mouse: ", Style::new().fg(Color::Gray)),
                    Span::styled("Available", Style::new().fg(Color::Green)),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Press Enter to continue or Esc to skip.",
                    Style::new().fg(Color::DarkGray),
                )));
            }
            WizardStep::ServerConnect => {
                lines.push(Line::from(Span::styled(
                    "  Server Connection",
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  URL: ", Style::new().fg(Color::Gray)),
                    Span::styled(&self.server_url, Style::new().fg(Color::Cyan)),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Press Enter to continue or Esc to skip.",
                    Style::new().fg(Color::DarkGray),
                )));
            }
            WizardStep::ThemeSelect => {
                lines.push(Line::from(Span::styled(
                    "  Theme Selection",
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  [1] ", Style::new().fg(Color::Cyan)),
                    Span::styled("Dark (default)", Style::new().fg(Color::White)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  [2] ", Style::new().fg(Color::Cyan)),
                    Span::styled("Light", Style::new().fg(Color::White)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  [3] ", Style::new().fg(Color::Cyan)),
                    Span::styled("High Contrast", Style::new().fg(Color::White)),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Press 1-3 to select, Esc to skip.",
                    Style::new().fg(Color::DarkGray),
                )));
            }
            WizardStep::MouseSetup => {
                lines.push(Line::from(Span::styled(
                    "  Mouse Support",
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Enable mouse support?",
                    Style::new().fg(Color::White),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  [Y] ", Style::new().fg(Color::Green)),
                    Span::styled("Yes", Style::new().fg(Color::White)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  [N] ", Style::new().fg(Color::Red)),
                    Span::styled("No", Style::new().fg(Color::White)),
                ]));
            }
            WizardStep::Complete => {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Setup Complete!",
                    Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Settings saved. Press any key to start.",
                    Style::new().fg(Color::DarkGray),
                )));
            }
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Onboarding Wizard ")
            .border_style(Style::new().fg(Color::Yellow));
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}
