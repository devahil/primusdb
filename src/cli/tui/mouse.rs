use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::cli::tui::app::{NavSection, TuiApp, HEADER_HEIGHT, SIDEBAR_WIDTH};

pub struct MouseHandler;

impl MouseHandler {
    pub fn handle(app: &mut TuiApp, mouse: MouseEvent) {
        if !app.mouse_enabled {
            return;
        }
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                Self::handle_click(app, mouse.row, mouse.column);
            }
            MouseEventKind::ScrollUp => {
                Self::scroll(app, true);
            }
            MouseEventKind::ScrollDown => {
                Self::scroll(app, false);
            }
            MouseEventKind::Moved => {
                Self::handle_hover(app, mouse.row, mouse.column);
            }
            MouseEventKind::Down(MouseButton::Right) => {
                app.show_contextual_help = !app.show_contextual_help;
            }
            _ => {}
        }
    }

    fn handle_click(app: &mut TuiApp, row: u16, col: u16) {
        if row < HEADER_HEIGHT {
            return;
        }
        if col < SIDEBAR_WIDTH && row >= HEADER_HEIGHT {
            let idx = row.saturating_sub(2) as usize;
            if idx < NavSection::count() {
                app.current_section = NavSection::from_index(idx);
            }
        } else {
            let content_row = row.saturating_sub(3) as usize;
            match app.current_section {
                NavSection::BackupRestore if content_row < app.backups_data.len() => {
                    app.selected_table_index = content_row;
                }
                NavSection::DatabasesEngines if content_row < app.databases_data.len() => {
                    app.selected_table_index = content_row;
                }
                NavSection::Namespaces if content_row < app.namespaces_data.len() => {
                    app.selected_table_index = content_row;
                }
                NavSection::ConfigurationStudio if content_row < app.config_entries.len() => {
                    app.config_selected_index = content_row;
                }
                NavSection::ReportBuilder if content_row < app.reports_data.len() => {
                    app.report_selected_index = content_row;
                }
                NavSection::Notebook if content_row < app.notebooks_data.len() => {
                    app.notebook_selected_index = content_row;
                }
                NavSection::RAGWorkspace if content_row < app.rag_collections.len() => {
                    app.rag_selected_index = content_row;
                }
                NavSection::SecurityCenter => {
                    use crate::cli::tui::app::SecurityCenterMode;
                    let max = match app.sec_mode {
                        SecurityCenterMode::Users => app.sec_users.len(),
                        SecurityCenterMode::Roles => app.sec_roles.len(),
                        SecurityCenterMode::Permissions => app.sec_permissions.len(),
                        _ => 0,
                    };
                    if content_row < max {
                        app.sec_selected_index = content_row;
                    }
                }
                NavSection::DocumentWorkspace if content_row < app.doc_collections.len() => {
                    app.doc_collection_selected = content_row;
                }
                _ => {}
            }
        }
    }

    fn scroll(app: &mut TuiApp, up: bool) {
        match app.current_section {
            NavSection::QueryConsole => {
                if up {
                    app.query_scroll = app.query_scroll.saturating_sub(3);
                } else {
                    app.query_scroll = app.query_scroll.saturating_add(3);
                }
            }
            NavSection::FileBrowser => {
                if up {
                    app.file_scroll = app.file_scroll.saturating_sub(3);
                } else {
                    app.file_scroll = app.file_scroll.saturating_add(3);
                }
            }
            NavSection::Dashboard
            | NavSection::MetricsLogs
            | NavSection::BackupRestore
            | NavSection::Governor
            | NavSection::ConfigurationStudio
            | NavSection::TableExplorer
            | NavSection::ReportBuilder
            | NavSection::Notebook
            | NavSection::RAGWorkspace => {
                if up {
                    app.selected_table_index = app.selected_table_index.saturating_sub(1);
                } else {
                    app.selected_table_index = app.selected_table_index.saturating_add(1);
                }
            }
            _ => {}
        }
    }

    fn handle_hover(app: &mut TuiApp, row: u16, col: u16) {
        if col < SIDEBAR_WIDTH && row >= HEADER_HEIGHT {
            let idx = row.saturating_sub(2) as usize;
            if idx < NavSection::count() {
                app.hovered_section = Some(NavSection::from_index(idx));
            }
        } else {
            app.hovered_section = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::tui::app::NAV_SECTIONS;

    #[test]
    fn test_mouse_handler_disabled() {
        let mut app = TuiApp::new();
        app.mouse_enabled = false;
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 3,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        MouseHandler::handle(&mut app, mouse);
        assert_eq!(app.current_section, NavSection::Dashboard);
    }

    #[test]
    fn test_sidebar_click_navigates() {
        let mut app = TuiApp::new();
        app.mouse_enabled = true;
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 3,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        MouseHandler::handle(&mut app, mouse);
        assert_eq!(app.current_section, NavSection::QueryConsole);
    }

    #[test]
    fn test_sidebar_click_out_of_range() {
        let mut app = TuiApp::new();
        app.mouse_enabled = true;
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 100,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        MouseHandler::handle(&mut app, mouse);
        assert_eq!(app.current_section, NavSection::Dashboard);
    }

    #[test]
    fn test_scroll_up_query_console() {
        let mut app = TuiApp::new();
        app.mouse_enabled = true;
        app.current_section = NavSection::QueryConsole;
        app.query_scroll = 10;
        let mouse = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 30,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        MouseHandler::handle(&mut app, mouse);
        assert_eq!(app.query_scroll, 7);
    }

    #[test]
    fn test_scroll_down_query_console() {
        let mut app = TuiApp::new();
        app.mouse_enabled = true;
        app.current_section = NavSection::QueryConsole;
        app.query_scroll = 0;
        let mouse = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 30,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        MouseHandler::handle(&mut app, mouse);
        assert_eq!(app.query_scroll, 3);
    }

    #[test]
    fn test_scroll_up_file_browser() {
        let mut app = TuiApp::new();
        app.mouse_enabled = true;
        app.current_section = NavSection::FileBrowser;
        app.file_scroll = 5;
        let mouse = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 30,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        MouseHandler::handle(&mut app, mouse);
        assert_eq!(app.file_scroll, 2);
    }

    #[test]
    fn test_hover_sidebar() {
        let mut app = TuiApp::new();
        app.mouse_enabled = true;
        let mouse = MouseEvent {
            kind: MouseEventKind::Moved,
            column: 10,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        MouseHandler::handle(&mut app, mouse);
        assert!(app.hovered_section.is_some());
    }

    #[test]
    fn test_hover_outside_sidebar() {
        let mut app = TuiApp::new();
        app.mouse_enabled = true;
        app.hovered_section = Some(NavSection::Dashboard);
        let mouse = MouseEvent {
            kind: MouseEventKind::Moved,
            column: 30,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        MouseHandler::handle(&mut app, mouse);
        assert!(app.hovered_section.is_none());
    }

    #[test]
    fn test_right_click_toggles_help() {
        let mut app = TuiApp::new();
        app.mouse_enabled = true;
        assert!(!app.show_contextual_help);
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 30,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        MouseHandler::handle(&mut app, mouse);
        assert!(app.show_contextual_help);
        MouseHandler::handle(&mut app, mouse);
        assert!(!app.show_contextual_help);
    }

    #[test]
    fn test_scroll_up_saturates_at_zero() {
        let mut app = TuiApp::new();
        app.mouse_enabled = true;
        app.current_section = NavSection::QueryConsole;
        app.query_scroll = 0;
        let mouse = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 30,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        MouseHandler::handle(&mut app, mouse);
        assert_eq!(app.query_scroll, 0);
    }

    #[test]
    fn test_content_area_click_selects_item() {
        let mut app = TuiApp::new();
        app.mouse_enabled = true;
        app.current_section = NavSection::DatabasesEngines;
        app.databases_data = vec!["db1".into(), "db2".into(), "db3".into()];
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 30,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        MouseHandler::handle(&mut app, mouse);
        assert_eq!(app.selected_table_index, 2);
    }

    #[test]
    fn test_nav_section_count_and_from_index() {
        assert_eq!(NavSection::count(), NAV_SECTIONS.len());
        assert_eq!(NavSection::from_index(0), NavSection::Dashboard);
        assert_eq!(NavSection::from_index(1), NavSection::QueryConsole);
        assert_eq!(
            NavSection::from_index(NAV_SECTIONS.len() - 1),
            *NAV_SECTIONS.last().unwrap()
        );
        assert_eq!(NavSection::from_index(999), NavSection::Dashboard);
    }
}
