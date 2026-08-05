# Keyboard Shortcuts

> **DEPRECATED**: This documents the TUI, which was removed from the build in v1.3.2-alpha. Use the CLI/REPL or REST API instead.

## Global Navigation

| Key | Action |
|-----|--------|
| `Tab` | Next section |
| `Shift+Tab` / `BackTab` | Previous section |
| `Up` / `Down` | Navigate list / select item |
| `Enter` | Select / Connect / Execute |
| `Esc` | Back / Close help / Close palette |
| `h` | Go to Help section |
| `r` / `R` | Refresh current view |
| `e` / `E` | Toggle event log viewer |
| `?` | Toggle contextual help |
| `:` | Open command palette |
| `q` | Quit (with confirmation in safe mode) |
| `Ctrl+C` | Quit (ignored in Query Console) |

## Query Console

| Key | Action |
|-----|--------|
| `Ctrl+E` | Execute current query |
| `Ctrl+D` | Toggle detail view |
| `Ctrl+L` | Clear results and input |
| `PageUp` / `PageDown` | Scroll results by 10 lines |
| `Backspace` | Delete last character |
| `Delete` | Clear input |
| `Up` / `Down` | Navigate query history |
| `Ctrl+S` | Export query result to file |
| `E` | Explain current query |
| `H` | Toggle query history panel |

## Namespaces

| Key | Action |
|-----|--------|
| `n` | Create new namespace |
| `u` | Switch to selected namespace (make active) |
| `d` | Delete selected namespace |
| `r` | Refresh |
| `Enter` | Inspect namespace details |

## Backup & Restore

| Key | Action |
|-----|--------|
| `Ctrl+B` | Create backup (runs `primusdb backup create`) |
| `Ctrl+R` | Restore selected backup |
| `v` | Verify selected backup |
| `r` | Refresh backup list |
| `Enter` | Inspect selected backup |

## Migration Wizard

| Key | Action |
|-----|--------|
| `Ctrl+M` | Toggle migration wizard |
| `Up` / `Down` | Select option |
| `Enter` | Confirm step |
| `Esc` | Back to previous step |
| `Tab` | Next field |

## Command Palette

| Key | Action |
|-----|--------|
| `:` | Open palette |
| `Esc` | Close palette |
| `Enter` | Execute selected command |
| `Up` / `Down` | Navigate filtered results |
| `Tab` | Autocomplete from selection |
| `Backspace` | Delete last character |
| `Char` | Type to filter commands |

## Configuration Studio

| Key | Action |
|-----|--------|
| `Up` / `Down` | Navigate entries |
| `Enter` | View detail / confirm |
| `e` | Edit selected entry |
| `n` | Create new entry |
| `d` | Delete entry (with confirmation) |
| `s` | Snapshot management |
| `x` | Export/Import |
| `Esc` | Back to list |

## Table Explorer

| Key | Action |
|-----|--------|
| `Up` / `Down` | Navigate items |
| `Enter` | Select storage type / view table |
| `r` | Refresh |
| `Esc` | Back to previous mode |

## Report Builder

| Key | Action |
|-----|--------|
| `Up` / `Down` | Navigate reports |
| `n` | Create new report |
| `Enter` | Execute / view results |
| `d` | Delete report (with confirmation) |
| `Esc` | Back |

## Notebook

| Key | Action |
|-----|--------|
| `Up` / `Down` | Navigate notebooks / cells |
| `n` | Create new notebook |
| `e` | Edit selected cell |
| `Enter` | Execute cell |
| `d` | Delete notebook (with confirmation) |
| `Esc` | Back |

## RAG Workspace

| Key | Action |
|-----|--------|
| `Up` / `Down` | Navigate collections |
| `Enter` | Configure search / execute |
| `+` / `-` | Adjust top-K limit |
| `r` | Refresh |
| `Esc` | Back |

## Settings

| Key | Action |
|-----|--------|
| `e` | Edit server endpoint URL |
| `t` | Set or clear auth token |
| `i` | Edit refresh interval |
| `h` | Cycle theme (default → dark → light → high-contrast) |
| `s` | Toggle safe mode |
| `m` | Toggle mouse support |
| `r` | Refresh server status |
| `d` | Run doctor diagnostics |
| `Esc` | Back |

## Security Center

| Key | Action |
|-----|--------|
| `Up` / `Down` | Navigate users/roles |
| `n` | Create new user/role |
| `d` | Delete selected item (with confirmation) |
| `a` | Assign roles to selected user |
| `Enter` | View user/role detail |
| `Esc` | Back |

## Cluster Management

| Key | Action |
|-----|--------|
| `Up` / `Down` | Select node |
| `s` | Start server |
| `x` | Stop server |
| `r` | Restart server |
| `j` | Join cluster (enter URL) |
| `l` | Leave cluster |
| `b` | Rebalance cluster |
| `m` | Toggle maintenance mode |
| `d` | Remove selected node |

## Metrics & Logs

| Key | Action |
|-----|--------|
| `1` | Metrics view only |
| `2` | Logs view only |
| `3` | Split view (both) |
| `l` | Cycle log level filter |
| `m` | Cycle module filter |

## File Browser

| Key | Action |
|-----|--------|
| `Up` / `Down` | Navigate files |
| `Enter` | Open directory / read file |
| `Esc` | Go up / back |
| `h` | Go home |
| `r` | Refresh |
| `d` | Delete selected file |

## Input Bar Editing

| Key | Action |
|-----|--------|
| `Char` | Type character |
| `Backspace` | Delete character before cursor |
| `Delete` | Clear entire input |
| `Enter` | Submit / execute |
| `Esc` | Cancel / close |

## Confirmation Dialogs

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch between Yes / No |
| `Enter` | Confirm |
| `Esc` | Cancel |
