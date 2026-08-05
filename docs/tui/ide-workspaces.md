# TUI IDE Workspaces (v1.3.2-alpha)

> **DEPRECATED**: This documents the TUI, which was removed from the build in v1.3.2-alpha. Use the CLI/REPL or REST API instead.

## Overview

The PrimusDB TUI provides 16 navigable sections ("workspaces") that collectively form a terminal IDE for database management. Five of these sections were added in v1.3.2-alpha to extend the TUI's capabilities toward a professional IDE-like experience.

## Architecture

```
+--------------------------------------------------+
| PrimusDB Terminal TUI IDE                         |
+--------------------------------------------------+
|  Header | Sidebar (16 sections) | Content Panel  |
|---------+-----------------------+-----------------|
|         | Dashboard             |                 |
|         | Query Console         |  Active section |
|         | DB & Engines          |  content        |
|         | Namespaces            |  displayed here |
|         | Cluster               |                 |
|         | Federation            |                 |
|         | Governor              |                 |
|         | Backup/Restore        |                 |
|         | Metrics & Logs        |                 |
|         | Config Studio ------> | NEW in v1.3.2   |
|         | Table Explorer -----> | NEW in v1.3.2   |
|         | Report Builder -----> | NEW in v1.3.2   |
|         | Notebook -----------> | NEW in v1.3.2   |
|         | RAG Workspace ------> | NEW in v1.3.2   |
|         | Settings -----------> | Enhanced        |
|         | Help                 |                 |
+--------------------------------------------------+
|  Command Palette | Status Bar                    |
+--------------------------------------------------+
```

## Configuration Studio

**Location:** Sidebar → Config Studio (press `e` to edit, `n` for new entry, `s` for snapshots)

Full interactive configuration management panel with 8 modes:

| Mode           | Description                                      |
|----------------|--------------------------------------------------|
| List           | Shows all config entries with key, value, source  |
| Detail         | View full details of a single entry               |
| Edit           | Modify an existing config entry                   |
| NewEntry       | Create a new config entry with validation          |
| ConfirmDelete  | Confirm before deleting an entry                  |
| Snapshots      | List, create, restore, and delete config snapshots |
| CreateSnapshot | Capture a point-in-time config snapshot            |
| ImportExport   | Export config as JSON or import from JSON          |

**Keyboard shortcuts:**
- `e` — Edit selected entry
- `n` — New entry
- `d` — Delete entry (with confirmation)
- `s` — Snapshot management
- `x` — Export/Import
- `Esc` — Back to previous mode

**API endpoints used:**
- `GET /api/v1/config` — list entries
- `POST /api/v1/config` — set entry
- `DELETE /api/v1/config` — delete entry
- `GET /api/v1/config/export` — export bundle
- `POST /api/v1/config/import` — import bundle
- `GET /api/v1/config/snapshots` — list snapshots
- `POST /api/v1/config/snapshots` — create snapshot
- `POST /api/v1/config/snapshots/:id/restore` — restore
- `DELETE /api/v1/config/snapshots/:id` — delete snapshot

## Table Explorer

**Location:** Sidebar → Table Explorer

Browse tables and collections across all storage engines.

**Modes:**

| Mode              | Description                                  |
|-------------------|----------------------------------------------|
| StorageTypeSelect | Choose storage engine (relational, document, vector, columnar, keyvalue) |
| TableList         | List tables in selected storage type          |
| TableDetail       | View schema, columns, indexes, constraints    |
| RowBrowser        | Browse rows with pagination                   |
| ExportOptions     | Export table data as JSON or CSV              |

**API endpoints used:**
- `GET /api/v1/explorer/storage-types` — list storage types
- `GET /api/v1/explorer/tables?storage_type=...` — list tables
- `GET /api/v1/explorer/table/:storage_type/:table` — table info
- `POST /api/v1/explorer/table/:storage_type/:table/rows` — paginated rows

## Report Builder

**Location:** Sidebar → Report Builder

Create, save, and execute report definitions.

**Modes:**

| Mode           | Description                              |
|----------------|------------------------------------------|
| List           | Shows saved report definitions           |
| Detail         | View report query and metadata           |
| Create         | Define new report (name, storage type, table, query, format) |
| ConfirmDelete  | Confirm before deleting a report         |
| Results        | Execute report and view results          |

**Report format options:** table, JSON, CSV

**API endpoints used:**
- `GET /api/v1/reports` — list reports
- `POST /api/v1/reports` — create report
- `GET /api/v1/reports/:id` — get report
- `PUT /api/v1/reports/:id` — update report
- `DELETE /api/v1/reports/:id` — delete report
- `POST /api/v1/reports/:id/execute` — execute report

## Notebook

**Location:** Sidebar → Notebook

Multi-cell notebook supporting markdown, SQL, analysis, and RAG cells.

**Cell types:**

| Cell Type  | Description                         |
|------------|-------------------------------------|
| SQL        | Execute SQL queries against engines |
| Analysis   | Run analysis on table data          |
| RAG        | Retrieval-augmented generation search |
| Markdown   | Documentation and notes             |

**Modes:**

| Mode             | Description                              |
|------------------|------------------------------------------|
| List             | Show saved notebooks                     |
| Detail           | View notebook with cell list             |
| CellEdit         | Edit selected cell content               |
| CellTypeSelect   | Change cell type                         |
| ConfirmDelete    | Confirm before deleting notebook/cell    |
| Results          | Execute cell and view results            |

**API endpoints used:**
- `GET /api/v1/notebooks` — list notebooks
- `POST /api/v1/notebooks` — create notebook
- `GET /api/v1/notebooks/:id` — get notebook
- `PUT /api/v1/notebooks/:id` — update notebook
- `DELETE /api/v1/notebooks/:id` — delete notebook
- `POST /api/v1/notebooks/:id/execute` — execute cell

## RAG Workspace

**Location:** Sidebar → RAG Workspace

Perform retrieval-augmented generation searches against vector collections.

**Modes:**

| Mode              | Description                              |
|-------------------|------------------------------------------|
| CollectionSelect  | Choose vector collection to search       |
| SearchConfig      | Enter query text and configure top-k     |
| SearchResults     | View similarity search results           |

**API endpoints used:**
- `GET /api/v1/rag/search?collection=...&query=...&limit=...` — RAG search
- Returns documents with similarity scores and metadata

## Settings (Enhanced)

**Location:** Sidebar → Settings

**Modes:**

| Mode                | Description                              |
|---------------------|------------------------------------------|
| View                | Show connection info, TUI config, server status |
| EditRefreshInterval | Change auto-refresh interval (ms)         |
| ToggleMouse         | Enable/disable mouse capture              |

Displays:
- Connected server URL and status
- Server version
- Current TUI config (mouse, theme, refresh, safe mode)
- Quick actions to modify settings

## Keyboard Shortcuts

| Key          | Action                     |
|--------------|----------------------------|
| Tab / Arrows | Navigate sections          |
| Enter        | Select/Execute             |
| `:`          | Open command palette       |
| `q`          | Toggle quit confirmation   |
| `r`          | Refresh current section    |
| Esc          | Back / Cancel              |
| Ctrl+B       | Create backup              |
| Ctrl+R       | Restore backup             |
| Ctrl+M       | Migration wizard           |
| `?`          | Toggle contextual help     |

## Data Flow

```
TUI Application
      |
      v
HTTP Client (src/cli/tui/api.rs)
      |
      v
REST API (axum server)
      |
      v
PrimusDB Core
      |
      +-> SystemDatabase (config/catalog persistence)
      +-> Storage Engines (queries, table metadata)
      +-> AI/ML Engine (RAG search, analysis)
```

## Persistence

TUI configuration (mouse, theme, refresh interval, safe mode) is persisted to the system database via `SystemDatabase::set_tui_config()` and loaded on startup. See `src/cli/tui/config.rs` for details.
