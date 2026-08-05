use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub const ENGINES: &[(&str, &str, &str)] = &[
    ("relational", "SQL tables, FK, views, triggers", "users"),
    (
        "columnar",
        "OLAP analytics, LZ4 compression",
        "analytics_events",
    ),
    (
        "document",
        "JSON collections, dynamic schema",
        "app_configs",
    ),
    ("keyvalue", "CouchDB-compatible, MVCC", "session_store"),
    ("vector", "Similarity search, HNSW/IVF", "embeddings"),
    (
        "timeseries",
        "IoT metrics, tag partitioning",
        "sensor_readings",
    ),
];

pub struct CreateDbWizard {
    pub step: WizardStep,
    pub name: String,
    pub description: String,
    pub template_index: usize,
    pub engine_toggles: [bool; 6],
    pub error: Option<String>,
    pub cursor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WizardStep {
    Name,
    Description,
    Template,
    Engines,
    Confirm,
}

const TEMPLATES: &[(&str, &str)] = &[
    ("Minimal", "Empty namespace — add tables manually later"),
    ("Full Hybrid", "Pre-create one table in each engine"),
    ("Custom", "Pick which engines to scaffold"),
];

impl Default for CreateDbWizard {
    fn default() -> Self {
        Self::new()
    }
}

impl CreateDbWizard {
    pub fn new() -> Self {
        Self {
            step: WizardStep::Name,
            name: String::new(),
            description: String::new(),
            template_index: 0,
            engine_toggles: [true; 6],
            error: None,
            cursor: 0,
        }
    }

    pub fn reset(&mut self) {
        self.step = WizardStep::Name;
        self.name.clear();
        self.description.clear();
        self.template_index = 0;
        self.engine_toggles = [true; 6];
        self.error = None;
        self.cursor = 0;
    }

    fn step_number(&self) -> usize {
        match self.step {
            WizardStep::Name => 0,
            WizardStep::Description => 1,
            WizardStep::Template => 2,
            WizardStep::Engines => 3,
            WizardStep::Confirm => 4,
        }
    }

    fn active_engines(&self) -> Vec<usize> {
        self.engine_toggles
            .iter()
            .enumerate()
            .filter(|(_, &on)| on)
            .map(|(i, _)| i)
            .collect()
    }

    fn handle_text_input(
        field: &mut String,
        key: crossterm::event::KeyCode,
        cursor: &mut usize,
    ) -> bool {
        use crossterm::event::KeyCode;
        match key {
            KeyCode::Char(c) => {
                field.insert(*cursor, c);
                *cursor += 1;
                true
            }
            KeyCode::Backspace => {
                if *cursor > 0 {
                    *cursor -= 1;
                    field.remove(*cursor);
                }
                true
            }
            KeyCode::Left => {
                *cursor = cursor.saturating_sub(1);
                true
            }
            KeyCode::Right => {
                if *cursor < field.len() {
                    *cursor += 1;
                }
                true
            }
            _ => false,
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> WizardAction {
        use crossterm::event::KeyCode;
        self.error = None;
        match self.step {
            WizardStep::Name => match key {
                KeyCode::Enter => {
                    if self.name.trim().is_empty() {
                        self.error = Some("Database name cannot be empty".into());
                    } else if self.name.contains(' ') {
                        self.error = Some("Database name cannot contain spaces".into());
                    } else {
                        self.step = WizardStep::Description;
                        self.cursor = self.description.len();
                    }
                    WizardAction::Continue
                }
                KeyCode::Esc => WizardAction::Cancel,
                _ => {
                    Self::handle_text_input(&mut self.name, key, &mut self.cursor);
                    WizardAction::Continue
                }
            },
            WizardStep::Description => match key {
                KeyCode::Enter => {
                    self.step = WizardStep::Template;
                    WizardAction::Continue
                }
                KeyCode::Esc => {
                    self.step = WizardStep::Name;
                    self.cursor = self.name.len();
                    WizardAction::Continue
                }
                KeyCode::Tab => {
                    self.step = WizardStep::Template;
                    WizardAction::Continue
                }
                _ => {
                    Self::handle_text_input(&mut self.description, key, &mut self.cursor);
                    WizardAction::Continue
                }
            },
            WizardStep::Template => match key {
                KeyCode::Up => {
                    self.template_index = self.template_index.saturating_sub(1);
                    WizardAction::Continue
                }
                KeyCode::Down => {
                    if self.template_index + 1 < TEMPLATES.len() {
                        self.template_index += 1;
                    }
                    WizardAction::Continue
                }
                KeyCode::Char('1') if !TEMPLATES.is_empty() => {
                    self.template_index = 0;
                    WizardAction::Continue
                }
                KeyCode::Char('2') if TEMPLATES.len() >= 2 => {
                    self.template_index = 1;
                    WizardAction::Continue
                }
                KeyCode::Char('3') if TEMPLATES.len() >= 3 => {
                    self.template_index = 2;
                    WizardAction::Continue
                }
                KeyCode::Enter => {
                    match self.template_index {
                        0 => {
                            self.engine_toggles = [false; 6];
                            self.step = WizardStep::Confirm;
                        }
                        1 => {
                            self.engine_toggles = [true; 6];
                            self.step = WizardStep::Confirm;
                        }
                        2 => {
                            self.step = WizardStep::Engines;
                        }
                        _ => {}
                    }
                    WizardAction::Continue
                }
                KeyCode::Esc => {
                    self.step = WizardStep::Description;
                    self.cursor = self.description.len();
                    WizardAction::Continue
                }
                _ => WizardAction::Continue,
            },
            WizardStep::Engines => match key {
                KeyCode::Up => {
                    // Scroll through engine list
                    WizardAction::Continue
                }
                KeyCode::Down => WizardAction::Continue,
                KeyCode::Char('1') => {
                    self.engine_toggles[0] = !self.engine_toggles[0];
                    WizardAction::Continue
                }
                KeyCode::Char('2') => {
                    self.engine_toggles[1] = !self.engine_toggles[1];
                    WizardAction::Continue
                }
                KeyCode::Char('3') => {
                    self.engine_toggles[2] = !self.engine_toggles[2];
                    WizardAction::Continue
                }
                KeyCode::Char('4') => {
                    self.engine_toggles[3] = !self.engine_toggles[3];
                    WizardAction::Continue
                }
                KeyCode::Char('5') => {
                    self.engine_toggles[4] = !self.engine_toggles[4];
                    WizardAction::Continue
                }
                KeyCode::Char('6') => {
                    self.engine_toggles[5] = !self.engine_toggles[5];
                    WizardAction::Continue
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.engine_toggles = [true; 6];
                    WizardAction::Continue
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.engine_toggles = [false; 6];
                    WizardAction::Continue
                }
                KeyCode::Enter => {
                    if !self.engine_toggles.iter().any(|&x| x) {
                        self.error = Some("Select at least one engine".into());
                    } else {
                        self.step = WizardStep::Confirm;
                    }
                    WizardAction::Continue
                }
                KeyCode::Esc => {
                    self.step = WizardStep::Template;
                    WizardAction::Continue
                }
                _ => WizardAction::Continue,
            },
            WizardStep::Confirm => match key {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    let engines: Vec<String> = self
                        .active_engines()
                        .iter()
                        .map(|&i| ENGINES[i].0.to_string())
                        .collect();
                    WizardAction::Create {
                        name: self.name.clone(),
                        description: self.description.clone(),
                        engines,
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => WizardAction::Cancel,
                _ => WizardAction::Continue,
            },
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        // Header
        lines.push(Line::from(Span::styled(
            " Create New Hybrid Database",
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            " ────────────────────────────────────────",
            Style::new().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        // Step indicator
        let step_names = ["1.Name", "2.Desc", "3.Type", "4.Engines", "5.Create"];
        let step_line: Vec<Span> = step_names
            .iter()
            .enumerate()
            .flat_map(|(i, s)| {
                let color = if i == self.step_number() {
                    Color::Cyan
                } else if i < self.step_number() {
                    Color::Green
                } else {
                    Color::DarkGray
                };
                vec![
                    Span::styled(format!(" {} ", s), Style::new().fg(color)),
                    Span::styled("│ ", Style::new().fg(Color::DarkGray)),
                ]
            })
            .collect();
        lines.push(Line::from(step_line));
        lines.push(Line::from(""));

        match self.step {
            WizardStep::Name => {
                lines.push(Line::from(Span::styled(
                    " Database Name:",
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                if self.name.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "│_",
                        Style::new().fg(Color::DarkGray),
                    )));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("│", Style::new().fg(Color::DarkGray)),
                        Span::styled(&self.name, Style::new().fg(Color::White)),
                        Span::styled("_", Style::new().fg(Color::Cyan)),
                    ]));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    " This creates a namespace (database container) that spans",
                    Style::new().fg(Color::DarkGray),
                )));
                lines.push(Line::from(Span::styled(
                    " all storage engines. Spaces not allowed.",
                    Style::new().fg(Color::DarkGray),
                )));
            }
            WizardStep::Description => {
                lines.push(Line::from(Span::styled(
                    " Description (optional):",
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                if self.description.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "│_  (Tab to skip)",
                        Style::new().fg(Color::DarkGray),
                    )));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("│", Style::new().fg(Color::DarkGray)),
                        Span::styled(&self.description, Style::new().fg(Color::White)),
                        Span::styled("_", Style::new().fg(Color::Cyan)),
                    ]));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    " Enter for next step, Tab to skip description.",
                    Style::new().fg(Color::DarkGray),
                )));
            }
            WizardStep::Template => {
                lines.push(Line::from(Span::styled(
                    " Scaffold Template:",
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                for (i, (name, desc)) in TEMPLATES.iter().enumerate() {
                    let marker = if i == self.template_index { "▸" } else { " " };
                    let style = if i == self.template_index {
                        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::new().fg(Color::White)
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!(" {} [{}] ", marker, i + 1), style),
                        Span::styled(
                            format!("{:14}", name),
                            Style::new()
                                .fg(if i == self.template_index {
                                    Color::Cyan
                                } else {
                                    Color::DarkGray
                                })
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(format!(" {}", desc), Style::new().fg(Color::Gray)),
                    ]));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    " Minimal = no tables, Full = all 6 engines, Custom = pick.",
                    Style::new().fg(Color::DarkGray),
                )));
            }
            WizardStep::Engines => {
                lines.push(Line::from(Span::styled(
                    " Select Engines to Scaffold:",
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                for (i, &(engine, desc, example)) in ENGINES.iter().enumerate() {
                    let checked = self.engine_toggles[i];
                    let checkbox = if checked { "[✓]" } else { "[ ]" };
                    let box_color = if checked {
                        Color::Green
                    } else {
                        Color::DarkGray
                    };
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!(" {} [{}] ", checkbox, i + 1),
                            Style::new().fg(box_color),
                        ),
                        Span::styled(
                            format!("{:12}", engine),
                            Style::new()
                                .fg(if checked {
                                    Color::Cyan
                                } else {
                                    Color::DarkGray
                                })
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(format!("{} → ", desc), Style::new().fg(Color::Gray)),
                        Span::styled(example, Style::new().fg(Color::DarkGray)),
                    ]));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    " 1-6 toggle  |  A all  |  N none  |  Enter confirm",
                    Style::new().fg(Color::DarkGray),
                )));
            }
            WizardStep::Confirm => {
                let active = self.active_engines();
                lines.push(Line::from(Span::styled(
                    " Confirm Creation:",
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  Name:   ", Style::new().fg(Color::DarkGray)),
                    Span::styled(
                        &self.name,
                        Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                ]));
                if !self.description.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("  Desc:   ", Style::new().fg(Color::DarkGray)),
                        Span::styled(&self.description, Style::new().fg(Color::Gray)),
                    ]));
                }
                lines.push(Line::from(vec![
                    Span::styled("  Type:   ", Style::new().fg(Color::DarkGray)),
                    Span::styled("Hybrid Database", Style::new().fg(Color::Cyan)),
                ]));
                lines.push(Line::from(""));
                if active.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  Scaffold: (none — add tables manually)",
                        Style::new().fg(Color::DarkGray),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("  Scaffold {} engine(s):", active.len()),
                        Style::new().fg(Color::DarkGray),
                    )));
                    for &idx in &active {
                        let (engine, _, example) = ENGINES[idx];
                        lines.push(Line::from(vec![
                            Span::styled("    ● ", Style::new().fg(Color::Green)),
                            Span::styled(
                                engine,
                                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(" → table '{}'", example),
                                Style::new().fg(Color::Gray),
                            ),
                        ]));
                    }
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    " Create this hybrid database?",
                    Style::new().fg(Color::White),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  [", Style::new().fg(Color::DarkGray)),
                    Span::styled(
                        "Y",
                        Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("] Yes  [", Style::new().fg(Color::DarkGray)),
                    Span::styled(
                        "N",
                        Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("] Cancel", Style::new().fg(Color::DarkGray)),
                ]));
            }
        }

        if let Some(ref err) = self.error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(" ✗ {}", err),
                Style::new().fg(Color::Red),
            )));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Create Hybrid Database ")
            .border_style(Style::new().fg(Color::Cyan));

        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

#[derive(Debug, Clone)]
pub enum WizardAction {
    Continue,
    Cancel,
    Create {
        name: String,
        description: String,
        engines: Vec<String>,
    },
}
