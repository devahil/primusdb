# TUI Keybindings Reference

All keyboard shortcuts available in the PrimusDB multipanel TUI.

## Global Navigation

| Key | Action |
|-----|--------|
| `Tab` | Next section |
| `Shift+Tab` | Previous section |
| `Up Arrow` / `Down Arrow` | Navigate lists (instances, query results, etc.) |
| `Enter` | Select / Connect to selected instance |
| `r` or `R` | Refresh current section data |
| `?` | Toggle help page |
| `:` | Open command palette |
| `Esc` | Back / Close help / Close palette / Wizard back |
| `q` or `Ctrl+C` | Quit TUI |

## Queries Section

| Key | Action |
|-----|--------|
| `Enter` | Execute current query |
| `Ctrl+E` | Execute query (alternative) |
| `Ctrl+L` | Clear query results and input |
| `PgUp` / `PgDn` | Scroll query results |

## Backups Section

| Key | Action |
|-----|--------|
| `Ctrl+B` | Create backup (triggers CLI command) |
| `Ctrl+R` | Restore backup (triggers CLI command) |

## Migration Wizard

| Key | Action |
|-----|--------|
| `Ctrl+M` | Toggle migration wizard (from Migrations section) |
| `Esc` | Go back one step / Close wizard |
| `Enter` | Confirm current step / Advance |
| `1`-`4` | Select source type or migration mode |
| `Space` | Toggle object selection (step 7) |
| `Backspace` | Edit URL/namespace input |

## Dashboard

| Key | Action |
|-----|--------|
| `Ctrl+D` | Toggle details view (show_instances) |
| `r` | Refresh status data |

## Command Palette (`:`)

| Key | Action |
|-----|--------|
| `Esc` | Close palette |
| `Enter` | Execute command |
| Any char | Type command |
| `Backspace` / `Delete` | Edit command |

## Input Bar Editing

| Key | Action |
|-----|--------|
| Any char | Type into input |
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `Home` or `Ctrl+A` | Move cursor to start |
| `End` or `Ctrl+E` | Move cursor to end |
| `Left Arrow` / `Right Arrow` | Move cursor |
| `Ctrl+W` | Delete word before cursor |
| `Ctrl+U` | Delete entire line |

## Summary

```
Navigation:   Tab  Shift+Tab  ↑  ↓  Enter  Esc  ?
Refresh:      r  Ctrl+L
Actions:      Ctrl+B  Ctrl+R  Ctrl+E  Ctrl+D  Ctrl+M
Quit:         q  Ctrl+C
```
