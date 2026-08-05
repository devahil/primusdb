use crate::cli::tui::app::TuiApp;
use crate::cli::tui::widgets::spanned_line;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

fn detect_operation_type(message: &str) -> &'static str {
    let lower = message.to_lowercase();
    if lower.contains("restor") {
        "Restore"
    } else if lower.contains("delet") {
        "Delete"
    } else if lower.contains("creat") || lower.contains("backup") {
        "Create"
    } else if lower.contains("verify") {
        "Verify"
    } else {
        "Operation"
    }
}

fn format_elapsed(elapsed_secs: u64) -> String {
    let m = elapsed_secs / 60;
    let s = elapsed_secs % 60;
    if m > 0 {
        format!("{}m {:02}s", m, s)
    } else {
        format!("{}s", s)
    }
}

fn render_backup_progress_bar(
    lines: &mut Vec<Line>,
    pct: u8,
    palette: &crate::cli::tui::config::ThemePalette,
) {
    let bar_width = 36;
    let filled = ((pct as u16) * bar_width / 100).min(bar_width) as usize;
    let empty = bar_width as usize - filled;

    let bar_color = if pct >= 100 {
        palette.success
    } else if pct >= 60 {
        palette.primary
    } else {
        palette.warning
    };

    lines.push(Line::from(vec![
        Span::styled("  [", Style::new().fg(Color::DarkGray)),
        Span::styled(
            "█".repeat(filled),
            Style::new().fg(bar_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled("░".repeat(empty), Style::new().fg(Color::DarkGray)),
        Span::styled(
            format!("] {:>3}%", pct),
            Style::new().fg(bar_color).add_modifier(Modifier::BOLD),
        ),
    ]));
}

pub fn render_backups(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Backups ")
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    // ── Active backup/restore/delete progress ────────────────────────
    if app.backup_in_progress {
        let op_type = detect_operation_type(&app.backup_progress_message);
        let elapsed = app
            .backup_operation_start
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);

        if app.export_progress > 0 && app.export_progress < 100 {
            // We have a percentage-based progress (export/import path)
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  ⏳ {} ", op_type),
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&app.export_phase, Style::new().fg(p.title)),
                Span::styled(
                    format!("  [{}]", format_elapsed(elapsed)),
                    Style::new().fg(Color::DarkGray),
                ),
            ]));
            render_backup_progress_bar(&mut lines, app.export_progress, &p);
            if !app.export_status_line.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", app.export_status_line),
                    Style::new().fg(Color::Gray),
                )));
            }
        } else {
            // Spinner mode — no percentage yet
            let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let spinner = spinner_chars[app.tick_count as usize % spinner_chars.len()];

            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", spinner),
                    Style::new().fg(p.warning).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", op_type),
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if app.backup_progress_message.is_empty() {
                        "Processing..."
                    } else {
                        &app.backup_progress_message
                    },
                    Style::new().fg(p.title),
                ),
                Span::styled(
                    format!("  [{}]", format_elapsed(elapsed)),
                    Style::new().fg(Color::DarkGray),
                ),
            ]));

            // Animated braille dots to show activity
            let dot_count = (app.tick_count as usize / 3) % 4;
            if dot_count > 0 {
                lines.push(Line::from(Span::styled(
                    format!("         {}", ".".repeat(dot_count)),
                    Style::new().fg(Color::DarkGray),
                )));
            }
        }
        lines.push(Line::from(""));
    } else if !app.export_status_line.is_empty() && app.export_progress >= 100 {
        // ── Completed state ──────────────────────────────────────────
        let op_type = detect_operation_type(&app.export_status_line);
        lines.push(Line::from(vec![
            Span::styled(
                "  ✓ ",
                Style::new().fg(p.success).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} completed ", op_type),
                Style::new().fg(p.success).add_modifier(Modifier::BOLD),
            ),
            Span::styled(&app.export_status_line, Style::new().fg(Color::Gray)),
        ]));
        render_backup_progress_bar(&mut lines, 100, &p);
        lines.push(Line::from(""));
    } else if !app.export_phase.is_empty() && app.export_progress > 0 && app.export_progress < 100 {
        lines.push(Line::from(Span::styled(
            format!("  {} {}", app.export_phase, app.export_status_line),
            Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
        )));
        render_backup_progress_bar(&mut lines, app.export_progress, &p);
        lines.push(Line::from(""));
    }

    if app.backups_data.is_empty() {
        lines.push(Line::from(Span::styled(
            "No backup files found in the backups/ directory.",
            Style::new().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(spanned_line(&[
            ("  Backups are stored in ", Color::Gray, false),
            ("./backups/", p.primary, false),
        ]));
        lines.push(spanned_line(&[
            ("  Create one with: ", Color::Gray, false),
            ("primusdb backup create", p.primary, false),
        ]));
        lines.push(spanned_line(&[
            ("  Or press: ", Color::Gray, false),
            ("Ctrl+B", p.primary, true),
        ]));
    } else {
        let count_str = format!("  {} backup(s) found:", app.backups_data.len());
        lines.push(Line::from(Span::styled(
            count_str,
            Style::new().fg(p.success).add_modifier(Modifier::BOLD),
        )));

        if let Some(ref detail) = app.backups_detail {
            if let Some(backups_arr) = detail.get("backups").and_then(|b| b.as_array()) {
                if !backups_arr.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(spanned_line(&[
                        ("  ID", p.primary, true),
                        ("  Date", p.primary, true),
                        ("  Size", p.primary, true),
                        ("  Engines", p.primary, true),
                        ("  Status", p.primary, true),
                    ]));
                    lines.push(Line::from(Span::styled(
                        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                        Style::new().fg(Color::DarkGray),
                    )));
                    for backup in backups_arr {
                        let id = backup.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                        let created = backup
                            .get("created_at")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let size = backup
                            .get("size_bytes")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let engines = backup
                            .get("engines")
                            .and_then(|v| v.as_str())
                            .unwrap_or("-");
                        let status = backup
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let compression = backup
                            .get("compression")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let encrypted = backup
                            .get("encrypted")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        let size_str = if size > 1024 * 1024 * 1024 {
                            format!("{:.1}GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
                        } else if size > 1024 * 1024 {
                            format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
                        } else if size > 1024 {
                            format!("{:.1}KB", size as f64 / 1024.0)
                        } else {
                            format!("{}B", size)
                        };

                        let status_color = match status {
                            "completed" | "ok" => p.success,
                            "in_progress" | "running" => p.primary,
                            "failed" | "error" => p.error,
                            "verified" => p.warning,
                            _ => Color::White,
                        };

                        let mut extra = String::new();
                        if !compression.is_empty() && compression != "none" {
                            extra.push_str(&format!(" [{}]", compression));
                        }
                        if encrypted {
                            extra.push_str(" [enc]");
                        }
                        let engines_display = if engines.len() > 12 {
                            format!("{}...", &engines[..12])
                        } else {
                            engines.to_string()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!("  {}", id), Style::new().fg(Color::White)),
                            Span::styled(
                                format!("  {}", created),
                                Style::new().fg(Color::DarkGray),
                            ),
                            Span::styled(format!("  {:>7}", size_str), Style::new().fg(p.primary)),
                            Span::styled(
                                format!("  {}", engines_display),
                                Style::new().fg(p.warning),
                            ),
                            Span::styled(
                                format!("  {}", status),
                                Style::new().fg(status_color).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(extra, Style::new().fg(Color::DarkGray)),
                        ]));
                    }
                }
            }
        }

        if app.backups_detail.is_none() {
            lines.push(Line::from(""));
            lines.push(spanned_line(&[("  Type   Size       Name", p.title, true)]));
            lines.push(Line::from(Span::styled(
                "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                Style::new().fg(Color::DarkGray),
            )));
            for entry in &app.backups_data {
                lines.push(Line::from(Span::styled(
                    format!("  {}", entry),
                    Style::new().fg(Color::White),
                )));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
