//! # TUI Configuration
//!
//! Configuration model for the PrimusDB Terminal UI.
//! Supports persistence via the system database so settings survive
//! server restarts.
//!
//! ## Persistence Flow
//!
//! ```text
//! TUI Startup
//!   |
//!   +-> Load TuiConfig from SystemDatabase (if available)
//!   |     +-> Fall back to defaults if no persisted config
//!   |
//!   +-> User modifies settings in TUI (Settings section)
//!   |     +-> Save to SystemDatabase via set_tui_config()
//!   |
//!   +-> TUI restart loads last saved config from system DB
//! ```
//!
//! ## Config Sources (precedence)
//!
//! 1. CLI flags (highest priority)
//! 2. System database persisted config
//! 3. Default values

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// Named color palette for TUI themes.
#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub border: Color,
    pub title: Color,
    pub text: Color,
    pub text_dim: Color,
    pub selection: Color,
    pub highlight_bg: Color,
}

fn dark_palette() -> ThemePalette {
    ThemePalette {
        primary: Color::Cyan,
        secondary: Color::Yellow,
        accent: Color::Magenta,
        success: Color::Green,
        warning: Color::Yellow,
        error: Color::Red,
        border: Color::DarkGray,
        title: Color::Yellow,
        text: Color::White,
        text_dim: Color::Gray,
        selection: Color::Cyan,
        highlight_bg: Color::DarkGray,
    }
}

fn light_palette() -> ThemePalette {
    ThemePalette {
        primary: Color::Blue,
        secondary: Color::Yellow,
        accent: Color::Magenta,
        success: Color::Green,
        warning: Color::Yellow,
        error: Color::Red,
        border: Color::DarkGray,
        title: Color::Blue,
        text: Color::Black,
        text_dim: Color::Gray,
        selection: Color::Blue,
        highlight_bg: Color::White,
    }
}

fn high_contrast_palette() -> ThemePalette {
    ThemePalette {
        primary: Color::LightBlue,
        secondary: Color::LightYellow,
        accent: Color::LightMagenta,
        success: Color::LightGreen,
        warning: Color::LightYellow,
        error: Color::LightRed,
        border: Color::White,
        title: Color::LightYellow,
        text: Color::White,
        text_dim: Color::Gray,
        selection: Color::LightCyan,
        highlight_bg: Color::DarkGray,
    }
}

fn default_palette() -> ThemePalette {
    dark_palette()
}

/// Resolve a theme name to its color palette.
pub fn resolve_palette(theme: &str) -> ThemePalette {
    match theme {
        "dark" => dark_palette(),
        "light" => light_palette(),
        "high-contrast" => high_contrast_palette(),
        _ => default_palette(),
    }
}

/// TUI configuration model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Whether the TUI is enabled for interactive use.
    pub enabled: bool,
    /// Whether mouse events are captured by the TUI.
    pub mouse_enabled: bool,
    /// UI theme name (e.g. "default", "dark").
    pub theme: String,
    /// Auto-refresh interval in milliseconds.
    pub refresh_interval_ms: u64,
    /// Default section shown on startup.
    pub default_view: String,
    /// Whether destructive actions require confirmation.
    pub confirm_destructive_actions: bool,
    /// Last connected endpoint URL.
    pub endpoint: String,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mouse_enabled: true,
            theme: "default".to_string(),
            refresh_interval_ms: 2000,
            default_view: "dashboard".to_string(),
            confirm_destructive_actions: true,
            endpoint: "http://localhost:8080".to_string(),
        }
    }
}

impl TuiConfig {
    /// Return the color palette for the current theme.
    pub fn palette(&self) -> ThemePalette {
        resolve_palette(&self.theme)
    }

    /// Build a TuiConfig from CLI arguments, layered over defaults.
    pub fn from_cli(
        _endpoint: Option<String>,
        no_mouse: bool,
        refresh_interval: Option<u64>,
        theme: Option<String>,
        safe_mode: bool,
    ) -> Self {
        let mut config = Self::default();
        if no_mouse {
            config.mouse_enabled = false;
        }
        if let Some(interval) = refresh_interval {
            config.refresh_interval_ms = interval;
        }
        if let Some(t) = theme {
            config.theme = t;
        }
        if safe_mode {
            config.confirm_destructive_actions = true;
        }
        config
    }

    /// Return the refresh interval as a `Duration`.
    pub fn refresh_duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.refresh_interval_ms)
    }

    /// Load TUI config from the system database, falling back to
    /// default values if none is persisted.
    pub fn from_system_db(sys_db: Option<&crate::system::SystemDatabase>) -> Self {
        if let Some(db) = sys_db {
            if let Ok(Some(config)) = db.get_tui_config() {
                return config;
            }
        }
        Self::default()
    }

    /// Persist this config to the system database.
    pub fn save_to_system_db(&self, sys_db: Option<&crate::system::SystemDatabase>) {
        if let Some(db) = sys_db {
            if let Err(e) = db.set_tui_config(self) {
                tracing::warn!("Failed to persist TUI config: {}", e);
            }
        }
    }
}
