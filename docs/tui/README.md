# PrimusDB TUI Guide

> **DEPRECATED**: This documents the TUI, which was removed from the build in v1.3.2-alpha. Use the CLI/REPL or REST API instead. See the [CLI Guide](../cli/README.md) for the current interface.

The PrimusDB Terminal User Interface (TUI) provides a friendly, mouse-capable, keyboard-driven dashboard for database administration, querying, monitoring, and configuration.

## Quick Start

```bash
# Launch the TUI (auto-discovers local instances)
primusdb tui

# Launch and auto-connect
primusdb tui --server http://localhost:8080
primusdb tui --endpoint http://localhost:8080 --namespace myapp --safe-mode
```

## Layout

```
┌─ HEADER (version | connection status) ───────────────────────┐
├─────────┬────────────────────────────────────────────────────┤
│         │                                                    │
│ Sidebar │              Content Panel                         │
│ (16     │   (section-specific content)                       │
│  nav    │                                                    │
│  items) │                                                    │
│         │                                                    │
├─────────┴────────────────────────────────────────────────────┤
│ INPUT BAR: Query / Command / Config Input                    │
├──────────────────────────────────────────────────────────────┤
│ STATUS BAR: Server ● │ v1.3.2 │ ns:default │ db:mydb │ Section│
├──────────────────────────────────────────────────────────────┤
│ EVENT BAR: Latest activity message                           │
└──────────────────────────────────────────────────────────────┘
```

## Sections

| # | Section | Description |
|---|---------|-------------|
| 1 | **Dashboard** | Server health, uptime, version, engines, cluster summary |
| 2 | **Query Console** | SQL/UQL editor with history, results, export |
| 3 | **Databases & Engines** | List engines, databases/tables |
| 4 | **Namespaces** | List, create, delete, switch active namespace |
| 5 | **Cluster** | Node list, Raft status, membership, shard distribution |
| 6 | **Federation** | Federated clusters, DataDomains, balance plans |
| 7 | **Resource Governor** | Live executions, policies, violations, metrics |
| 8 | **Backup & Restore** | Create, inspect, verify, restore backups |
| 9 | **Metrics & Logs** | Prometheus metrics, log tail with level filters |
| 10 | **Configuration Studio** | Interactive config CRUD, snapshots, export/import |
| 11 | **Table Explorer** | Browse tables across storage engines |
| 12 | **Report Builder** | Create and execute report definitions |
| 13 | **Notebook** | Multi-cell notebook (SQL, Analysis, RAG, Markdown) |
| 14 | **RAG Workspace** | Vector collection search with similarity scores |
| 15 | **Settings** | Connection, auth, refresh, theme, mouse, namespace |
| 16 | **Help** | Keyboard/mouse reference, version, tips |

## Key Features

- **Mouse support**: Click sidebar to navigate, click items to select, scroll to scroll, right-click for help
- **Command palette**: Press `:` for fuzzy-filtered command execution
- **Contextual help**: Press `?` for section-specific help
- **Event log**: Press `e` to toggle the full event log viewer
- **Safe mode**: Destructive actions require confirmation (default)
- **Onboarding**: Connection wizard on first launch with auto-discovery
- **Status bar**: Shows connection state, version, namespace, database, cluster state, section name, keyboard hints
- **Empty states**: Helpful messages with next-action guidance when disconnected or no data

## Navigation

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous section |
| `Enter` | Select / execute |
| `Esc` | Back / close overlay |
| `:` | Command palette |
| `?` | Contextual help |
| `r` | Refresh |
| `e` | Toggle event log |
| `q` | Quit (with confirmation) |

## Status Bar

The enhanced status bar shows at a glance:
- `●` / `○` connection indicator
- Server URL
- PrimusDB version
- Active namespace
- Selected database
- Cluster state
- Current section name
- Contextual keyboard shortcuts
- Latest event message

## Command Palette

Press `:` and type to filter:

| Command | Action |
|---------|--------|
| `:help` | Open help |
| `:quit` | Quit |
| `:connect <url>` | Connect |
| `:disconnect` | Disconnect |
| `:events` | Toggle event log |
| `:clear` | Clear results |
| `:dashboard` | Go to Dashboard |
| `:query` | Go to Query Console |
| `:status` | Refresh status |
| `:backup create` | Create backup |

## Detailed Documentation

- [UX Guide](ux-guide.md) — Full UX design principles and interaction patterns
- [Mouse Support](mouse-support.md) — Mouse interaction reference and troubleshooting
- [Keyboard Shortcuts](keyboard-shortcuts.md) — Complete keybinding reference
- [Screens](screens.md) — Visual reference for all 16 sections
- [Troubleshooting](troubleshooting.md) — Common issues and solutions
- [IDE Workspaces](ide-workspaces.md) — Advanced workspace documentation
