# TUI Screens Reference

> **DEPRECATED**: This documents the TUI, which was removed from the build in v1.3.2-alpha. Use the CLI/REPL or REST API instead.

## 1. Dashboard

```
┌─ Dashboard ─────────────────────────────────────────────────┐
│ Instance URL: http://localhost:8080                          │
│ Health:       ● healthy                                      │
│ Version:      1.3.2-alpha                                    │
│ Uptime:       2h 15m                                         │
│ Engines:      columnar, vector, document, relational, k/v    │
│                                                              │
│ ┌─ Cluster ───────────────────────────────────────────────┐  │
│ │ ● Node-1 (leader)  ● Node-2  ○ Node-3 (unreachable)    │  │
│ └──────────────────────────────────────────────────────────┘  │
│                                                              │
│ ┌─ Storage Usage ──────────────────────────────────────────┐  │
│ │ [████████░░░░░░░░░]  256 MB / 1 GB (25%)                 │  │
│ └──────────────────────────────────────────────────────────┘  │
│                                                              │
│ ┌─ Recent Events ──────────────────────────────────────────┐  │
│ │ [12:00] Connected to server                              │  │
│ │ [12:01] Query executed (2 rows)                          │  │
│ └──────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## 2. Query Console

```
┌─ Query Console ──────────────────────────────────────────────┐
│ SELECT * FROM users                                           │
│ WHERE age > 21                                                │
│ ORDER BY name                                                 │
│                                                               │
│ ┌─ Results ────────────────────────────────────────────────┐  │
│ │  id  │  name   │  age  │  email                         │  │
│ │──────┼─────────┼───────┼────────────────────────────────│  │
│ │  1   │ Alice   │  30   │ alice@example.com              │  │
│ │  2   │ Bob     │  25   │ bob@example.com                │  │
│ └──────────────────────────────────────────────────────────┘  │
│                                                               │
│ [Ctrl+E] Run  [Ctrl+D] Details  [Ctrl+L] Clear               │
└──────────────────────────────────────────────────────────────┘
```

## 3. Databases & Engines

```
┌─ Databases & Engines ────────────────────────────────────────┐
│ Engines: columnar  vector  document  relational  keyvalue     │
│                                                               │
│ Databases:                                                     │
│  ▸ myapp_db       (relational)  📊 12 tables                  │
│    analytics_db   (columnar)    📊 5 tables                   │
│    vectors_db     (vector)      🔍 3 collections              │
│    docs_db        (document)    📄 8 collections              │
│                                                               │
│ [r] Refresh  [Enter] Inspect                                  │
└──────────────────────────────────────────────────────────────┘
```

## 4. Namespaces

```
┌─ Namespaces ─────────────────────────────────────────────────┐
│ ▸ default            (active)    policies: 3   resources: 12 │
│   myapp.prod                     policies: 5   resources: 28 │
│   myapp.staging                  policies: 3   resources: 15 │
│                                                               │
│ [n] New  [d] Delete  [r] Refresh                               │
└──────────────────────────────────────────────────────────────┘
```

## 5. Cluster

```
┌─ Cluster ────────────────────────────────────────────────────┐
│ Status: Healthy                                               │
│ Leader: node-1:8080                                           │
│ Nodes: 3/3                                                    │
│                                                               │
│ Node ID    │ Status  │ Role    │ Shards │ Region              │
│────────────┼─────────┼─────────┼────────┼────────────────────│
│ node-1     │ Online  │ Leader  │  12    │ us-east             │
│ node-2     │ Online  │ Follower│  10    │ us-west             │
│ node-3     │ Online  │ Follower│  10    │ eu-central          │
│                                                               │
│ Raft: Term 5 │ Committed: 42 │ Applied: 42                    │
│                                                               │
│ [r] Refresh  [?] Help                                         │
└──────────────────────────────────────────────────────────────┘
```

## 6. Federation

```
┌─ Federation ─────────────────────────────────────────────────┐
│ Status: Connected to 2 clusters                                │
│                                                               │
│ Cluster ID │ Status   │ Nodes │ Domains │ Latency             │
│────────────┼──────────┼───────┼─────────┼────────────────────│
│ cluster-1  │ Online   │   3   │    2    │  12ms               │
│ cluster-2  │ Online   │   5   │    3    │  45ms               │
│                                                               │
│ DataDomains:                                                   │
│  ▸ global-users   (clusters: 2,  mode: async)                │
│    eu-finance     (clusters: 1,  mode: sync)                  │
│                                                               │
│ [r] Refresh  [?] Help                                         │
└──────────────────────────────────────────────────────────────┘
```

## 7. Resource Governor

```
┌─ Resource Governor ──────────────────────────────────────────┐
│ Status: Active                                                │
│                                                               │
│ Live Executions: 3                                             │
│  ▶ qry_001  SELECT * FROM large_table   12s  CPU:45%          │
│  ▶ qry_002  ANALYZE sales_data           8s  CPU:22%          │
│  ▶ qry_003  INSERT INTO logs             3s  CPU:8%           │
│                                                               │
│ Policies:                                                      │
│  ▸ default      CPU:80%  MEM:512MB  BLOCK on exceed           │
│    analytics    CPU:95%  MEM:2GB    THROTTLE at 80%            │
│                                                               │
│ Violations (last 24h): 12                                      │
│  [WARN] Query blocked: CPU quota exceeded (analytics)         │
│  [BLOCK] Query blocked: Memory limit reached (default)        │
│                                                               │
│ [r] Refresh  [?] Help                                         │
└──────────────────────────────────────────────────────────────┘
```

## 8. Backup & Restore

```
┌─ Backup & Restore ───────────────────────────────────────────┐
│ Available Backups:                                             │
│  ▸ backup_20260627_120000   Jun 27 12:00  256 MB  ✅ verified │
│    backup_20260627_080000   Jun 27 08:00  251 MB  ✅ verified │
│    backup_20260626_235959   Jun 26 23:59  248 MB  ❌ corrupt  │
│                                                               │
│ [Ctrl+B] Create Backup  [Ctrl+R] Restore  [r] Refresh          │
└──────────────────────────────────────────────────────────────┘
```

## 9. Metrics & Logs

```
┌─ Metrics ────────────────────────────────────────────────────┐
│ ┌─ Requests/sec ───────────────────────────────────────────┐ │
│ │ ████░░░░░░░░░░░░░░  45/s                                │ │
│ └──────────────────────────────────────────────────────────┘ │
│ ┌─ Error Rate ─────────────────────────────────────────────┐ │
│ │ ██░░░░░░░░░░░░░░░░░░  0.5%                              │ │
│ └──────────────────────────────────────────────────────────┘ │
│ ┌─ Memory ─────────────────────────────────────────────────┐ │
│ │ ████████░░░░░░░░░░░░  312 MB / 1 GB                      │ │
│ └──────────────────────────────────────────────────────────┘ │
│                                                               │
│ ┌─ Logs ───────────────────────────────────────────────────┐  │
│ │ [12:00:01] INFO  query executed: SELECT * FROM users    │  │
│ │ [12:00:02] INFO  backup created: backup_20260627        │  │
│ └──────────────────────────────────────────────────────────┘  │
│ [r] Refresh                                                   │
└──────────────────────────────────────────────────────────────┘
```

## 10. Settings

```
┌─ Settings ───────────────────────────────────────────────────┐
│ Connection                                                     │
│   Endpoint:  http://localhost:8080         [Edit]             │
│   Status:    ● Connected                                      │
│   Version:   1.3.2-alpha                                      │
│                                                               │
│ General                                                        │
│   Refresh interval:  2000 ms        [Edit]                    │
│   Mouse enabled:     Yes            [Toggle]                  │
│   Safe mode:         Yes            [Toggle]                  │
│   Theme:             default         [Edit]                   │
│                                                               │
│ Active Context                                                 │
│   Namespace:  default                                          │
│   Database:   myapp_db                                         │
│                                                               │
│ [e] Edit  [?] Help                                             │
└──────────────────────────────────────────────────────────────┘
```

## 11. Help

```
┌─ Help ───────────────────────────────────────────────────────┐
│ KEYBINDINGS                                                    │
│   q / Ctrl+C    Quit (with confirmation in safe mode)         │
│   Tab           Next section                                   │
│   Shift+Tab     Previous section                               │
│   Up/Down       Navigate list                                  │
│   Enter         Select / Connect                               │
│   r             Refresh current view                            │
│   e             Toggle event log viewer                        │
│   ?             Toggle contextual help                          │
│   :             Open command palette                            │
│   Esc           Back / Close help / Close palette              │
│   h             Go to Help section                              │
│   Ctrl+B        Create backup                                  │
│   Ctrl+E        Execute query                                  │
│   Ctrl+D        Disconnect                                     │
│   Ctrl+L        Clear query results / logs                     │
│                                                               │
│ MOUSE SUPPORT                                                  │
│   Left Click    Sidebar: navigate to section                   │
│   Left Click    Content: select item in list                   │
│   Scroll        Scroll content / results                       │
│   Right Click   Toggle contextual help                         │
│                                                               │
│ COMMAND PALETTE                                                │
│   :help           Open this help                               │
│   :quit           Quit the TUI                                 │
│   :connect <url>  Connect to a server                          │
│   :disconnect     Disconnect from server                       │
│   :events         Toggle event log                             │
│   :clear          Clear results and event log                  │
│                                                               │
│ VERSION: 1.3.2-alpha                                           │
└──────────────────────────────────────────────────────────────┘
```

## 12. Config Studio

Interactive configuration management with 8 modes: List, Detail, Edit, NewEntry, Snapshots, CreateSnapshot, ImportExport, ConfirmDelete. Full REST API integration for config CRUD and snapshot lifecycle.

## 13. Table Explorer

Browse tables across storage engines with 5 modes: StorageTypeSelect, TableList, TableDetail, RowBrowser, ExportOptions.

## 14. Report Builder

Create and execute report definitions with 5 modes: List, Detail, Create, ConfirmDelete, Results.

## 15. Notebook

Multi-cell notebook with SQL/Analysis/RAG/Markdown cell types. 6 modes: List, Detail, CellEdit, CellTypeSelect, ConfirmDelete, Results.

## 16. RAG Workspace

Vector search with 3 modes: CollectionSelect, SearchConfig (query + top-K), SearchResults (similarity scores).

## 17. File Browser

Local filesystem browser with 2 modes: Browse (directory listing) and ReadFile (file content viewer). Navigate with ↑↓, Enter opens dirs/files, Esc goes up, h goes home, r refreshes, d deletes files.

## 18. Security Center

User and role management with list/detail/create/delete modes, plus AssignRole mode with space-toggle role checklist for RBAC assignment.

## 19. Monitoring

Observability dashboard with 5 tabs: Overview, Alerts, Performance, Replication, Resources. Health metrics and cluster-wide stats displayed.

## 20. Metrics & Logs

3-mode split pane: Metrics only, Logs only, or both. Log level cycling (error/warn/info/debug/trace) and module filtering.

## 21. Cluster Management

Node list with ID, Role, Status, Address. Confirmation modals for start/stop/restart/join/leave/maintenance toggle.
