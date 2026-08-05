use crate::cli::tui::app::{NavSection, TuiApp};
use ratatui::layout::Rect;
use ratatui::Frame;

/// Actions that a workspace can emit to the global event loop.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceAction {
    /// No action needed
    None,
    /// Switch to a different workspace
    SwitchTo(NavSection),
    /// Show a confirmation dialog
    Confirm(String, String), // (message, pending_action)
    /// Show a message in the status bar
    StatusMessage(String),
    /// Show an error
    ErrorMessage(String),
    /// Refresh the current view
    Refresh,
    /// Open command palette
    OpenCommandPalette,
    /// Open search
    OpenSearch,
    /// Execute a CLI command
    ExecCommand(String),
    /// Push a notification
    Notify(String),
}

/// Result of handling an event in a workspace.
#[derive(Debug, Clone, PartialEq)]
pub enum EventResult {
    /// Event was consumed, continue processing
    Consumed,
    /// Event was not consumed, pass to next handler
    NotConsumed,
    /// Event caused an action that needs global handling
    Action(WorkspaceAction),
}

/// Keyboard shortcut descriptor for help generation.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub keys: &'static str,
    pub description: &'static str,
    pub group: &'static str,
}

/// Composable layout specification for a workspace.
#[derive(Debug, Clone)]
pub enum LayoutSpec {
    /// Single panel filling the area
    Single,
    /// Horizontal split: top/bottom
    Horizontal {
        top: Box<LayoutSpec>,
        bottom: Box<LayoutSpec>,
        ratio: f32,
    },
    /// Vertical split: left/right
    Vertical {
        left: Box<LayoutSpec>,
        right: Box<LayoutSpec>,
        ratio: f32,
    },
    /// Tabbed panels
    Tabs { panels: Vec<String>, active: usize },
    /// Main content + right inspector
    WithInspector {
        main: Box<LayoutSpec>,
        inspector_width: u16,
    },
    /// Main content + floating overlay
    WithOverlay {
        main: Box<LayoutSpec>,
        overlay: Box<LayoutSpec>,
    },
}

/// The core workspace trait. Every section of the TUI implements this.
pub trait Workspace {
    /// Return the NavSection this workspace represents.
    fn section(&self) -> NavSection;

    /// Render the workspace into the given area.
    fn render(&self, frame: &mut Frame, area: Rect, app: &TuiApp);

    /// Handle a key event. Returns an EventResult.
    fn handle_key(&mut self, app: &mut TuiApp, key: crossterm::event::KeyCode) -> EventResult;

    /// Handle a mouse event. Returns an EventResult.
    fn handle_mouse(
        &mut self,
        _app: &mut TuiApp,
        _mouse: crossterm::event::MouseEvent,
    ) -> EventResult {
        EventResult::NotConsumed
    }

    /// Return available actions for the command palette.
    fn actions(&self) -> Vec<(String, WorkspaceAction)> {
        Vec::new()
    }

    /// Return key bindings for the help overlay.
    fn key_bindings(&self) -> Vec<KeyBinding> {
        Vec::new()
    }

    /// Return the layout specification for this workspace.
    fn layout(&self) -> LayoutSpec {
        LayoutSpec::Single
    }

    /// Called when this workspace becomes active.
    fn on_activate(&mut self, _app: &mut TuiApp) {}

    /// Called when this workspace becomes inactive.
    fn on_deactivate(&mut self, _app: &mut TuiApp) {}

    /// Return the title for the workspace tab.
    fn title(&self) -> &str {
        ""
    }
}
