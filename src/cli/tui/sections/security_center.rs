use crate::cli::tui::app::{SecurityCenterMode, TuiApp};
use crate::cli::tui::widgets::{render_json_block, spanned_line};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_security_center(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let p = app.config.palette();
    let title = match app.sec_mode {
        SecurityCenterMode::Users => " Security Center — Users ",
        SecurityCenterMode::Roles => " Security Center — Roles ",
        SecurityCenterMode::Permissions => " Security Center — Permissions ",
        SecurityCenterMode::UserDetail => " Security Center — User Detail ",
        SecurityCenterMode::RoleDetail => " Security Center — Role Detail ",
        SecurityCenterMode::CreateUser => " Security Center — Create User ",
        SecurityCenterMode::CreateRole => " Security Center — Create Role ",
        SecurityCenterMode::AssignRole => " Security Center — Assign Role ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(p.border))
        .title(title)
        .title_style(Style::new().fg(p.title).add_modifier(Modifier::BOLD));

    let mut lines: Vec<Line> = Vec::new();

    if !app.connected() {
        lines.push(Line::from(Span::styled(
            "  Not connected \u{2014} connect to a running PrimusDB server.",
            Style::new().fg(p.error),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Use :connect <url> or the command palette.",
            Style::new().fg(p.text_dim),
        )));
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }

    match app.sec_mode {
        SecurityCenterMode::Users => {
            if app.sec_users.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No users found. [n] Create user | [r] Refresh",
                    Style::new().fg(p.text_dim),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("  {} user(s):", app.sec_users.len()),
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                for (i, user) in app.sec_users.iter().enumerate() {
                    let username = user.get("username").and_then(|u| u.as_str()).unwrap_or("?");
                    let active = user
                        .get("is_active")
                        .and_then(|a| a.as_bool())
                        .unwrap_or(true);
                    let roles = user
                        .get("roles")
                        .and_then(|r| r.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    let marker = if i == app.sec_selected_index {
                        "\u{25b8}"
                    } else {
                        " "
                    };
                    let status = if active { "\u{25cf}" } else { "\u{25cb}" };
                    let style = if i == app.sec_selected_index {
                        Style::new().fg(p.primary).bg(Color::DarkGray)
                    } else {
                        Style::new().fg(p.text)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("  {} {} {} [{}]", marker, status, username, roles),
                        style,
                    )));
                }
            }
            lines.push(Line::from(""));
            lines.push(spanned_line(&[
                (" \u{2191}\u{2193} select ", p.text_dim, false),
                (" Enter:detail ", p.success, false),
                (" u:users ", p.primary, false),
                (" r:roles ", p.primary, false),
                (" p:perms ", p.primary, false),
                (" n:create ", p.warning, false),
            ]));
        }
        SecurityCenterMode::Roles => {
            if app.sec_roles.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No roles found. [n] Create role | [r] Refresh",
                    Style::new().fg(p.text_dim),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("  {} role(s):", app.sec_roles.len()),
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                for (i, role) in app.sec_roles.iter().enumerate() {
                    let name = role.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                    let marker = if i == app.sec_selected_index {
                        "\u{25b8}"
                    } else {
                        " "
                    };
                    let style = if i == app.sec_selected_index {
                        Style::new().fg(p.primary).bg(Color::DarkGray)
                    } else {
                        Style::new().fg(p.text)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("  {} {}", marker, name),
                        style,
                    )));
                }
            }
            lines.push(Line::from(""));
            lines.push(spanned_line(&[
                (" \u{2191}\u{2193} select ", p.text_dim, false),
                (" Enter:detail ", p.success, false),
                (" u:users ", p.primary, false),
                (" r:roles ", p.primary, false),
                (" p:perms ", p.primary, false),
                (" n:create ", p.warning, false),
            ]));
        }
        SecurityCenterMode::Permissions => {
            if app.sec_permissions.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No permission data available.",
                    Style::new().fg(p.text_dim),
                )));
            } else {
                for perm in &app.sec_permissions {
                    render_json_block(&mut lines, Some(perm), "");
                    lines.push(Line::from(""));
                }
            }
            lines.push(Line::from(""));
            lines.push(spanned_line(&[
                (" u:users ", p.primary, false),
                (" r:roles ", p.primary, false),
                (" p:perms ", p.primary, false),
            ]));
        }
        SecurityCenterMode::UserDetail => {
            if let Some(user) = app.sec_users.get(app.sec_selected_index) {
                let username = user.get("username").and_then(|u| u.as_str()).unwrap_or("?");
                let is_active = user
                    .get("is_active")
                    .and_then(|a| a.as_bool())
                    .unwrap_or(true);
                let roles: Vec<String> = user
                    .get("roles")
                    .and_then(|r| r.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                lines.push(Line::from(Span::styled(
                    format!("  User: {}", username),
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(Span::styled(
                    format!("  Active: {}", if is_active { "Yes" } else { "No" }),
                    Style::new().fg(if is_active { p.success } else { p.error }),
                )));
                lines.push(Line::from(Span::styled(
                    format!("  Roles: {}", roles.join(", ")),
                    Style::new().fg(p.text),
                )));
                for (k, v) in user.as_object().unwrap() {
                    if k != "username" && k != "is_active" && k != "roles" && k != "password" {
                        lines.push(Line::from(Span::styled(
                            format!("  {}: {}", k, v),
                            Style::new().fg(p.text_dim),
                        )));
                    }
                }
            }
            lines.push(Line::from(""));
            lines.push(spanned_line(&[
                (" Esc:back ", p.text_dim, false),
                (" d:delete ", p.error, false),
                (" a:assign role ", p.warning, false),
            ]));
        }
        SecurityCenterMode::RoleDetail => {
            if let Some(role) = app.sec_roles.get(app.sec_selected_index) {
                let name = role.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                lines.push(Line::from(Span::styled(
                    format!("  Role: {}", name),
                    Style::new().fg(p.primary).add_modifier(Modifier::BOLD),
                )));
                for (k, v) in role.as_object().unwrap() {
                    if k != "name" {
                        lines.push(Line::from(Span::styled(
                            format!("  {}: {}", k, v),
                            Style::new().fg(p.text_dim),
                        )));
                    }
                }
            }
            lines.push(Line::from(""));
            lines.push(spanned_line(&[(" Esc:back ", p.text_dim, false)]));
        }
        SecurityCenterMode::AssignRole => {
            lines.push(Line::from(Span::styled(
                "  Toggle roles with Space, then press Enter to save:",
                Style::new().fg(p.warning),
            )));
            lines.push(Line::from(""));
            if app.sec_role_checklist.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No roles available.",
                    Style::new().fg(p.text_dim),
                )));
            } else {
                for (i, (role_name, checked)) in app.sec_role_checklist.iter().enumerate() {
                    let marker = if i == app.sec_selected_index {
                        "\u{25b8}"
                    } else {
                        " "
                    };
                    let check = if *checked { "[\u{2713}]" } else { "[ ]" };
                    let style = if i == app.sec_selected_index {
                        Style::new().fg(p.primary).bg(Color::DarkGray)
                    } else {
                        Style::new().fg(p.text)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("  {} {} {}", marker, check, role_name),
                        style,
                    )));
                }
            }
            lines.push(Line::from(""));
            lines.push(spanned_line(&[
                (" Space:toggle ", p.success, false),
                (" Enter:save ", p.warning, false),
                (" Esc:cancel ", p.text_dim, false),
            ]));
        }
        SecurityCenterMode::CreateUser | SecurityCenterMode::CreateRole => {
            let input_display = if app.sec_input.is_empty() {
                "(type and press Enter)".to_string()
            } else {
                app.sec_input.clone()
            };
            lines.push(Line::from(Span::styled(
                format!("  {}", input_display),
                Style::new().fg(if app.sec_input.is_empty() {
                    p.text_dim
                } else {
                    p.text
                }),
            )));
            lines.push(Line::from(""));
            lines.push(spanned_line(&[
                (" Enter:save ", p.success, false),
                (" Esc:cancel ", p.text_dim, false),
            ]));
        }
    }

    if let Some(ref err) = app.sec_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Error: {}", err),
            Style::new().fg(p.error),
        )));
    }

    if !app.sec_status.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", app.sec_status),
            Style::new().fg(p.success),
        )));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
