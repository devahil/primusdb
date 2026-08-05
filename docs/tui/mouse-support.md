# Mouse Support in PrimusDB TUI

> **DEPRECATED**: This documents the TUI, which was removed from the build in v1.3.2-alpha. Use the CLI/REPL or REST API instead.

## Overview

The PrimusDB TUI supports mouse interactions via crossterm, providing a terminal-native mouse experience. Mouse support is enabled by default on mouse-capable terminals.

## Prerequisites

Mouse support requires a terminal that supports mouse events:
- **Linux**: xterm, GNOME Terminal, Konsole, Alacritty, Kitty, WezTerm, Foot, rxvt
- **macOS**: Terminal.app, iTerm2, Alacritty, Kitty, WezTerm
- **Windows**: Windows Terminal, ConEmu, PowerShell 7+

## Checking Mouse Support

Run `primusdb doctor --tui` to check if your terminal supports mouse events.

## Mouse Interactions

### Sidebar Navigation
- **Left click** on a sidebar item → navigates to that section
- The sidebar is 24 characters wide on the left

### Content Area
- **Left click** on a list item → selects that item
- Works in: Backups, Databases, Namespaces, Config Studio, Reports, Notebooks, RAG collections

### Scrolling
- **Scroll up/down** (mouse wheel) → scrolls content
- Works in: Query Console results, Dashboard lists, Metrics, Backups, Governor, Config Studio, Table Explorer, Report Builder, Notebook, RAG Workspace

### Contextual Help
- **Right click** anywhere → toggles contextual help overlay
- Shows help relevant to the current section

### Planned (Future)
- Double-click on sidebar items to refresh
- Drag to resize panels
- Click to focus input boxes
- Click tabs and buttons in sub-views

## Enabling/Disabling Mouse

### Via CLI
```bash
primusdb tui --no-mouse       # Start with mouse disabled
primusdb tui                   # Start with mouse enabled (default)
```

### Via Settings (in TUI)
1. Navigate to Settings section
2. Toggle "Mouse enabled" option

### Via Config
```toml
[tui]
mouse_enabled = true
```

## Technical Details

Mouse events are captured using crossterm's `EnableMouseCapture` mode:
- **Left click**: `Down(MouseButton::Left)` → navigate sidebar / select items
- **Right click**: `Down(MouseButton::Right)` → toggle contextual help
- **Scroll**: `ScrollDown` / `ScrollUp` → scroll content

The TUI does NOT use `EnableMouseMotionCapture` (motion tracking) to avoid excessive terminal traffic. Hover effects are not currently supported.

## Troubleshooting

### Mouse clicks not working
1. Try a different terminal (alacritty, kitty, or Windows Terminal recommended)
2. Run `primusdb doctor --tui` and check "Mouse support"
3. Verify mouse is enabled in Settings
4. Ensure `TERM` environment variable is set to a mouse-capable value (e.g. `xterm-256color`)

### Scrolling not working
- Verify mouse support is enabled
- Some terminal multiplexers (tmux, screen) require additional configuration
- Try with `TERM=xterm-256color`
