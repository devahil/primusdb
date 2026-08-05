# PrimusDB TUI UX Guide

> **DEPRECATED**: This documents the TUI, which was removed from the build in v1.3.2-alpha. Use the CLI/REPL or REST API instead.

## Overview

The PrimusDB Terminal User Interface (TUI) is designed to be a friendly, guided, discoverable, and mouse-capable interface for day-to-day database administration. This guide explains the UX philosophy, layout, and interaction patterns.

## Design Principles

1. **Discoverability** — Every action has a visible hint or shortcut.
2. **Guided onboarding** — First-time users see a connection wizard and recommended next steps.
3. **Mouse-first, keyboard-always** — All actions work with both mouse and keyboard.
4. **Safe by default** — Destructive actions require confirmation in safe mode.
5. **Contextual help** — Press `?` for help on any screen.

## Layout

```
┌──────────────────────────────────────────────────────────┐
│ HEADER: Version │ Connection Status                      │
├────────┬─────────────────────────────────────────────────┤
│        │                                                 │
│ Sidebar│              Content Panel                      │
│ (Nav)  │   (Active section rendered here)                │
│        │                                                 │
│        │                                                 │
├────────┴─────────────────────────────────────────────────┤
│ INPUT BAR: Query / Command / Config Input                │
├──────────────────────────────────────────────────────────┤
│ STATUS BAR: Server │ Version │ NS │ DB │ Cluster │ Hints │
├──────────────────────────────────────────────────────────┤
│ EVENT BAR: Latest event message                          │
└──────────────────────────────────────────────────────────┘
```

## Interaction Patterns

### Navigation
- **Keyboard**: `Tab`/`Shift+Tab` to cycle sections, Enter to select
- **Mouse**: Click sidebar items to navigate, scroll to scroll content
- **Palette**: Press `:` to open command palette, type to filter

### Status Bar
The status bar (bottom 3 lines) shows:
- Connection indicator (`●` connected, `○` disconnected)
- Server URL
- PrimusDB version
- Active namespace (ns:)
- Selected database (db:)
- Cluster state
- Current section name
- Contextual keyboard hints
- Latest event message

Press `e`/`E` to toggle the full event log view. Press `Ctrl+L` to clear.

### Contextual Help
Press `?` anywhere to see help for the current section.
Press `h` to jump to the Help section for the full reference.

### Command Palette
Press `:` to open the command palette. Type to fuzzy-filter commands:
- `:help` — Open help
- `:quit` — Quit with confirmation
- `:connect http://host:port` — Connect to server
- `:disconnect` — Disconnect from server
- `:dashboard`, `:query`, `:cluster` — Navigate to section
- `:events` — Toggle event log
- `:clear` — Clear results and event log

### Confirmation Dialogs
In safe mode (default), all destructive actions show a confirmation dialog:
- Quit
- Disconnect
- Delete backup
- Drop table
- Delete namespace
- Restore backup

Use `Tab`/`Shift+Tab` to switch between "Yes" and "No", Enter to confirm.

### Onboarding Flow
On first launch without a server:
1. Auto-discovery scans localhost ports 8080–9093
2. Shows discovered instances
3. Connect manually via `:connect <url>` or palette
4. Once connected, the Dashboard shows server health and recommended actions

### Empty States
Each section shows helpful messages when:
- **Disconnected**: "Connect to a PrimusDB server to use this section" with action hints
- **No data**: "No data available" with refresh hint
- **Error**: Red error message with what happened and what to do next

### Section List (Sidebar)
1. **Dashboard** — Server health, uptime, engines, storage usage
2. **Query Console** — Multiline SQL/UQL editor with history; `E` explain, `H` history panel, `↑↓` history cycling
3. **Databases & Engines** — List storage engines and databases
4. **Namespaces** — List/create/delete namespaces
5. **Cluster** — Node list, Raft status, membership; server start/stop/restart, join/leave/rebalance
6. **Federation** — Federated clusters, DataDomains
7. **Resource Governor** — Executions, policies, violations
8. **Backup/Restore** — Create, inspect, verify, restore backups; `v` verify integrity
9. **Metrics & Logs** — Tabbed Metrics/Logs/Both views; log level and module filters
10. **Config Studio** — Interactive configuration management
11. **Table Explorer** — Browse tables across engines
12. **Report Builder** — Create and execute reports
13. **Notebook** — Multi-cell notebook (SQL/Analysis/RAG)
14. **RAG Workspace** — Vector search and RAG queries
15. **Security Center** — Users and roles management; `a` assign roles
16. **Document Editor** — Document collections CRUD
17. **Terminal** — Integrated shell with real `sh -c` execution
18. **Monitoring** — Overview, Alerts, Performance, Replication, Resources tabs
19. **Settings** — Connection, auth, refresh, theme, mouse; `d` Doctor diagnostics
20. **File Browser** — Local filesystem navigation (read dirs, view files, delete)
21. **Help** — Full keyboard/mouse reference

## Responsive Layout

The TUI adapts to small terminals with three tiers:
- **Hard minimum**: 40×10 (shows error if smaller, refuses to start)
- **Compact mode**: 60×20 (sidebar hidden, content fills full width)
- **Recommended**: 80×24 or larger (full layout with visible sidebar)
- The status bar and event bar compress on narrow terminals
- The sidebar width is fixed at 24 characters

## Safe Mode

When `--safe-mode` is enabled (default: on):
- All destructive actions show a confirmation dialog
- Quit requires confirmation
- Disconnect requires confirmation
- Backup restore requires confirmation
- Table/namespace deletion requires confirmation

Toggle safe mode in Settings or via `--no-safe-mode` CLI flag.

## Mouse Support

Mouse support is enabled by default on mouse-capable terminals. See [mouse-support.md](mouse-support.md) for details.

## Keyboard Shortcuts

See [keyboard-shortcuts.md](keyboard-shortcuts.md) for the full reference.
