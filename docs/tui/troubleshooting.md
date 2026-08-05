# TUI Troubleshooting

> **DEPRECATED**: This documents the TUI, which was removed from the build in v1.3.2-alpha. Use the CLI/REPL or REST API instead.

## Common Issues

### "Terminal too small" Error
**Problem**: TUI shows "Terminal too small — resize to at least 60x20".

**Solution**: Resize your terminal window to at least 60 columns × 20 rows.
- Run `stty size` to check current size
- Most modern terminals can be resized with the mouse

### Mouse Clicks Not Working
**Problem**: Mouse clicks in the sidebar or content area don't work.

**Check**: Run `primusdb doctor --tui` and verify:
1. `TERM` shows a mouse-capable terminal (e.g., `xterm-256color`)
2. "Mouse support" shows "Available"
3. "Server connectivity" shows the server is reachable

**Solutions**:
1. Ensure mouse is enabled: Settings → Mouse enabled → Yes
2. Try a different terminal (Alacritty, Kitty, Windows Terminal recommended)
3. If using tmux/screen: ensure `set -g mouse on` in tmux config
4. Start with `primusdb tui` (mouse enabled by default)

### TUI Won't Start
**Problem**: TUI crashes or exits immediately on launch.

**Possible causes**:
1. **Missing dependencies**: Ensure `primusdb` binary is built with `cargo build --release`
2. **Terminal emulation**: Some SSH clients or terminal multiplexers may not fully support ratatui
3. **Raw mode failure**: The TUI needs raw mode access. Try running in a regular terminal (no `script`, `expect`, etc.)

### Disconnected State
**Problem**: "Not Connected" shown in most sections.

**Solutions**:
1. Start the server: `primusdb server start`
2. Connect from within TUI: `:connect http://localhost:8080`
3. Connect on launch: `primusdb tui --server http://localhost:8080`

### Slow Refresh
**Problem**: Data takes too long to refresh.

**Solution**: Adjust refresh interval in Settings or via CLI:
```bash
primusdb tui --refresh-interval 5000  # 5 seconds
```

### Garbled Display
**Problem**: Screen shows random characters or corrupted layout.

**Solutions**:
1. Press `Ctrl+L` to clear and redraw
2. Resize the terminal slightly to trigger re-render
3. If persistent, try `Ctrl+C` to quit and restart
4. Check your `TERM` setting: `echo $TERM`

### Event Log Too Large
**Problem**: Event log fills up and slows the TUI.

**Solution**: The event log is limited to 100 entries. Press `Ctrl+L` to clear. Press `e` to toggle the full event log view.

### "No Data" in Sections
**Problem**: Sections show no data when connected.

**Possible causes**:
1. Server hasn't populated the data yet
2. API endpoint may not be available in your PrimusDB version
3. Press `r` to manually refresh

### Keyboard Shortcuts Not Working
**Problem**: Some keyboard shortcuts don't respond.

**Solutions**:
1. Check if you're in the right section (some shortcuts are section-specific)
2. Check if an overlay is active (command palette, confirmation dialog, etc.)
3. Press `Esc` to close any overlays
4. Check keyboard-shortcuts.md for the full reference

## Diagnostics

Run comprehensive diagnostics:
```bash
primusdb doctor                    # Full system diagnostics
primusdb doctor --tui              # TUI-specific checks
primusdb doctor --aggressive       # Deep diagnostics
primusdb doctor --report report.txt # Write diagnostic report
```

## Getting Help

- Documentation: https://primusdb.dev/docs
- API Reference: https://primusdb.dev/api
- Report issues: https://github.com/devahil/primusdb/issues
