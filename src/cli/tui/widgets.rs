use ratatui::layout::Rect;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_loading(frame: &mut Frame, area: Rect, message: &str) {
    let spinner = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        / 100)
        % spinner.len() as u128;
    let spin_char = spinner.chars().nth(idx as usize).unwrap_or(' ');
    let text = Text::from(Line::from(vec![
        Span::styled(
            format!(" {} ", spin_char),
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(message, Style::new().fg(Color::Cyan)),
    ]));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Loading ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center),
        area,
    );
}

pub fn render_json_block(lines: &mut Vec<Line>, data: Option<&serde_json::Value>, empty_msg: &str) {
    if let Some(v) = data {
        let pretty = serde_json::to_string_pretty(v).unwrap_or_default();
        for line_str in pretty.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", line_str),
                Style::new().fg(Color::White),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            format!("  {}", empty_msg),
            Style::new().fg(Color::Gray),
        )));
    }
}

pub fn render_gauge(lines: &mut Vec<Line>, label: &str, pct: u8, color: Color) {
    let bar_width = 20;
    let filled = ((pct as u16) * bar_width / 100).min(bar_width);
    let empty = bar_width.saturating_sub(filled);
    let bar: String = format!(
        "|{}{}| {:3}%",
        "█".repeat(filled as usize),
        "░".repeat(empty as usize),
        pct
    );
    lines.push(Line::from(vec![
        Span::styled(format!("  {}: ", label), Style::new().fg(Color::Gray)),
        Span::styled(bar, Style::new().fg(color)),
    ]));
}

pub fn render_progress_bar(lines: &mut Vec<Line>, pct: u8) {
    let bar_width = 30;
    let filled = ((pct as u16) * bar_width / 100).min(bar_width);
    let empty = bar_width.saturating_sub(filled);
    let bar = format!(
        "[{}{}] {:3}%",
        "█".repeat(filled as usize),
        "░".repeat(empty as usize),
        pct
    );
    lines.push(Line::from(Span::styled(
        format!("  {}", bar),
        Style::new().fg(Color::Cyan),
    )));
}

fn flush_sql_buf(buf: &mut String, spans: &mut Vec<Span<'static>>) {
    if !buf.is_empty() {
        spans.push(Span::styled(
            std::mem::take(buf),
            Style::new().fg(Color::White),
        ));
    }
}

pub fn highlight_sql(sql: &str) -> Vec<Span<'static>> {
    const KEYWORDS: &[&str] = &[
        "ALL",
        "ALTER",
        "AND",
        "AS",
        "BETWEEN",
        "BY",
        "CASE",
        "CHECK",
        "CONSTRAINT",
        "CREATE",
        "DEFAULT",
        "DELETE",
        "DENSE_RANK",
        "DISTINCT",
        "DROP",
        "ELSE",
        "END",
        "EXCEPT",
        "EXISTS",
        "FIRST_VALUE",
        "FOREIGN",
        "FROM",
        "GROUP",
        "HAVING",
        "IN",
        "INDEX",
        "INSERT",
        "INTERSECT",
        "INTO",
        "JOIN",
        "KEY",
        "LAG",
        "LAST_VALUE",
        "LEAD",
        "LIKE",
        "LIMIT",
        "NTH_VALUE",
        "NOT",
        "NULL",
        "OFFSET",
        "ON",
        "OR",
        "ORDER",
        "OVER",
        "PARTITION",
        "PRIMARY",
        "RANK",
        "RECURSIVE",
        "REFERENCES",
        "RETURNING",
        "ROW_NUMBER",
        "SELECT",
        "SET",
        "TABLE",
        "THEN",
        "UNION",
        "UNIQUE",
        "UPDATE",
        "VALUES",
        "WHEN",
        "WHERE",
        "WITH",
    ];

    let mut spans: Vec<Span<'static>> = Vec::new();
    let chars: Vec<char> = sql.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut buf = String::new();

    while i < len {
        if chars[i] == '-' && i + 1 < len && chars[i + 1] == '-' {
            flush_sql_buf(&mut buf, &mut spans);
            let start = i;
            i += 2;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            spans.push(Span::styled(
                chars[start..i].iter().collect::<String>(),
                Style::new().fg(Color::DarkGray),
            ));
            continue;
        }

        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '*' {
            flush_sql_buf(&mut buf, &mut spans);
            let start = i;
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            } else {
                i = len;
            }
            spans.push(Span::styled(
                chars[start..i].iter().collect::<String>(),
                Style::new().fg(Color::DarkGray),
            ));
            continue;
        }

        if chars[i] == '\'' {
            flush_sql_buf(&mut buf, &mut spans);
            let start = i;
            i += 1;
            while i < len {
                if chars[i] == '\'' {
                    if i + 1 < len && chars[i + 1] == '\'' {
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            spans.push(Span::styled(
                chars[start..i].iter().collect::<String>(),
                Style::new().fg(Color::Green),
            ));
            continue;
        }

        if chars[i].is_ascii_digit() {
            flush_sql_buf(&mut buf, &mut spans);
            let start = i;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            spans.push(Span::styled(
                chars[start..i].iter().collect::<String>(),
                Style::new().fg(Color::Cyan),
            ));
            continue;
        }

        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            flush_sql_buf(&mut buf, &mut spans);
            let start = i;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let upper = word.to_uppercase();
            let color = if KEYWORDS.binary_search(&upper.as_str()).is_ok() {
                Color::Yellow
            } else {
                Color::White
            };
            spans.push(Span::styled(word, Style::new().fg(color)));
            continue;
        }

        if "=<>!+-*/%".contains(chars[i]) {
            flush_sql_buf(&mut buf, &mut spans);
            let start = i;
            if i + 1 < len {
                let two: String = chars[i..i + 2].iter().collect();
                if matches!(two.as_str(), "<>" | "<=" | ">=" | "!=" | "||") {
                    i += 2;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
            spans.push(Span::styled(
                chars[start..i].iter().collect::<String>(),
                Style::new().fg(Color::White),
            ));
            continue;
        }

        buf.push(chars[i]);
        i += 1;
    }

    flush_sql_buf(&mut buf, &mut spans);
    spans
}

pub fn spanned_line<'a>(parts: &[(&'a str, Color, bool)]) -> Line<'a> {
    let spans: Vec<Span> = parts
        .iter()
        .map(|(text, color, bold)| {
            let mut style = Style::new().fg(*color);
            if *bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            Span::styled(*text, style)
        })
        .collect();
    Line::from(spans)
}
