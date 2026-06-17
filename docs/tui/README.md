# PrimusDB TUI Guide

The PrimusDB Terminal User Interface (TUI) provides an interactive, keyboard-driven dashboard for managing databases, running queries, monitoring instances, and performing migrations.

## Quick Start

```bash
# Launch the TUI (auto-discovers local instances)
primusdb tui

# Launch and auto-connect to a specific server
primusdb tui --server http://localhost:8080
```

## Layout

The TUI is split into four horizontal regions:

```
┌─ Header ──────────────────────────────────────────┐
│  PrimusDB  ● connected  http://localhost:8080      │
├── Navigation ──┬─ Content ─────────────────────────┤
│  ▶ Dashboard   │  [section content here]           │
│    Instances   │                                   │
│    Clusters    │                                   │
│    ...         │                                   │
├────────────────┴─ Input ──────────────────────────┤
│  Query  Type SQL and press Enter to execute...     │
├────────────────── Status ─────────────────────────┤
│  12:34:56  Connected to http://localhost:8080      │
└───────────────────────────────────────────────────┘
```

### Regions

| Region | Description |
|--------|-------------|
| **Header** | App name, connection status indicator (●/○), current URL |
| **Sidebar** | Vertical navigation list of all 22 sections |
| **Content** | Main panel — renders the currently selected section |
| **Input bar** | Query input, command palette input, or migration wizard URL/namespace input |
| **Status bar** | Event log — shows recent activity messages |

## Sections

The TUI has 22 navigable sections:

| # | Section | Description |
|---|---------|-------------|
| 1 | **Dashboard** | Health overview, ASCII charts, discovery results, backups summary |
| 2 | **Instances** | Discovered PrimusDB instances — select to connect |
| 3 | **Clusters** | Cluster status, topology diagram, nodes, events |
| 4 | **Nodes** | Cluster node table |
| 5 | **Engines** | Storage engine information |
| 6 | **Databases** | List databases |
| 7 | **Namespaces** | List namespaces |
| 8 | **Tables/Collections** | Table listing |
| 9 | **Vector Indexes** | Vector index listing |
| 10 | **Graph** | Graph data |
| 11 | **AIML** | AI/ML data |
| 12 | **Queries** | Execute SQL queries, view results with scrolling |
| 13 | **Backups** | Backup file listing with detail columns |
| 14 | **Restores** | Backup restore entries |
| 15 | **Migrations** | Migration info + interactive 12-step migration wizard |
| 16 | **Users** | User management |
| 17 | **Roles** | Role management |
| 18 | **Metrics** | Real-time performance metrics |
| 19 | **Logs** | Server logs (journalctl) |
| 20 | **Diagnostics** | Server diagnostics |
| 21 | **Settings** | Server configuration |
| 22 | **Help** | Keybindings reference, version info |

## Migration Wizard

Press `Ctrl+M` in the Migrations section to launch the interactive 12-step wizard:

1. **Intro** — Overview of the migration process
2. **Source type** — Select MySQL, PostgreSQL, MongoDB, or CouchDB
3. **Source URL** — Enter the connection string
4. **Test connection** — Automatically tests the source connection
5. **Namespace** — Enter the target PrimusDB namespace
6. **Migration mode** — Choose copy, schema-only, data-only, or dry-run
7. **Inspect objects** — Automatically inspects source schema
8. **Select objects** — Toggle individual tables/collections with Space
9. **Preview plan** — Shows the generated migration plan
10. **Dry-run** — Executes a trial run (or skips if dry-run mode selected)
11. **Confirm** — Final confirmation before import
12. **Progress + Report** — Real-time progress bar, then saveable report

Use `Esc` to go back one step at any time.

## Command Palette

Press `:` to open the command palette:

| Command | Action |
|---------|--------|
| `:help` | Open the help page |
| `:quit` | Quit the TUI |
| `:refresh` | Refresh the current section |
| `:connect <url>` | Connect to a server |

## States

Each section handles the following states:

| State | Indicator |
|-------|-----------|
| **Loading** | Spinner + message (e.g. "Discovering instances...") |
| **Disconnected** | Red "Not connected" message with guidance |
| **Empty** | Gray "No data" message with refresh hint |
| **Error** | Red error message |
| **OK** | Normal data display with color-coded status |
