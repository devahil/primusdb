use crate::cli::tui::app::{NavSection, TuiApp};
use crate::cli::tui::workspace::{EventResult, Workspace, WorkspaceAction};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

/// The event router implements a priority-based dispatch chain:
/// 1. Global hotkeys (Ctrl+Q, Ctrl+P, Ctrl+Shift+S, etc.)
/// 2. Active overlay (command palette, search, confirm dialog)
/// 3. Active workspace
/// 4. Default handling
pub struct EventRouter;

impl EventRouter {
    /// Route a key event through the priority chain.
    pub fn route(
        app: &mut TuiApp,
        key: KeyEvent,
        workspaces: &mut HashMap<NavSection, Box<dyn Workspace>>,
    ) -> Option<WorkspaceAction> {
        // 1. Global hotkeys first
        if let Some(action) = Self::handle_global_hotkeys(app, key) {
            return Some(action);
        }

        // 2. Active overlay handling
        if app.confirm_action != crate::cli::tui::app::ConfirmAction::None {
            return Self::handle_confirm_dialog(app, key);
        }
        if app.show_command_palette {
            return Self::handle_command_palette(app, key);
        }
        if app.show_search {
            return Self::handle_search_overlay(app, key);
        }

        // 3. Active workspace
        let section = app.current_section;
        if let Some(workspace) = workspaces.get_mut(&section) {
            match workspace.handle_key(app, key.code) {
                EventResult::Consumed => return None,
                EventResult::NotConsumed => {}
                EventResult::Action(action) => return Some(action),
            }
        }

        None
    }

    fn handle_global_hotkeys(app: &mut TuiApp, key: KeyEvent) -> Option<WorkspaceAction> {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('q')) => Some(WorkspaceAction::Confirm(
                "Quit PrimusDB?".to_string(),
                "quit".to_string(),
            )),
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                app.show_command_palette = !app.show_command_palette;
                None
            }
            (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                app.show_search = !app.show_search;
                None
            }
            (KeyModifiers::CONTROL, KeyCode::Char('r')) => Some(WorkspaceAction::Refresh),
            _ => None,
        }
    }

    fn handle_confirm_dialog(app: &mut TuiApp, key: KeyEvent) -> Option<WorkspaceAction> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                let action = app.pending_action.clone().unwrap_or_default();
                app.confirm_action = crate::cli::tui::app::ConfirmAction::None;
                app.confirm_message.clear();
                app.pending_action = None;
                Some(WorkspaceAction::ExecCommand(action))
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.confirm_action = crate::cli::tui::app::ConfirmAction::None;
                app.confirm_message.clear();
                app.pending_action = None;
                None
            }
            _ => None,
        }
    }

    fn handle_command_palette(app: &mut TuiApp, key: KeyEvent) -> Option<WorkspaceAction> {
        match key.code {
            KeyCode::Esc => {
                app.show_command_palette = false;
                None
            }
            KeyCode::Up => {
                app.command_palette_selection = app.command_palette_selection.saturating_sub(1);
                None
            }
            KeyCode::Down => {
                app.command_palette_selection += 1;
                None
            }
            KeyCode::Enter => {
                if let Some(item) = app
                    .command_palette_filtered
                    .get(app.command_palette_selection)
                {
                    let cmd = item.clone();
                    app.show_command_palette = false;
                    Some(WorkspaceAction::ExecCommand(cmd))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn handle_search_overlay(app: &mut TuiApp, key: KeyEvent) -> Option<WorkspaceAction> {
        match key.code {
            KeyCode::Esc => {
                app.show_search = false;
                None
            }
            KeyCode::Up => {
                app.search_selection = app.search_selection.saturating_sub(1);
                None
            }
            KeyCode::Down => {
                app.search_selection += 1;
                None
            }
            _ => None,
        }
    }
}
