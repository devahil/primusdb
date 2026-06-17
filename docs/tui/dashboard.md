# TUI Dashboard

The Dashboard is the default section shown on TUI startup. It provides an at-a-glance overview: connection info, health gauges, ASCII bar charts, and discovery results.

## Connected View

### Connection Info

| Field | Source |
|-------|--------|
| Instance URL | Connected server endpoint |
| Health | `healthy` / `ok` / `<error>` |
| Version | Server version string |
| Uptime | Formatted as `HH:MM:SS` |
| Engines | Comma-separated enabled engines |

### ASCII Bar Charts

When connected, the Dashboard renders horizontal bar charts for key metrics:

```
  QPS:       ████████░░░░░░░░░░░░  8,421
  Errors:    ░░░░░░░░░░░░░░░░░░░░  0
  Memory:    ████████░░░░░░░░░░░░  4.2 GB / 16 GB
  Storage:   ██████████░░░░░░░░░░  234 GB / 1 TB
  Health:    ████████████████████  100%
```

Each bar uses Unicode block characters (`█` for filled, `░` for empty) scaled to 20 characters. The raw value is shown at the end of the bar.

### Cluster Health Dots

When cluster node data is available, a row of health indicators is shown:

```
  Cluster Nodes:  ● node-1  ● node-2  ◒ node-3
```

| Dot | Meaning |
|-----|---------|
| `●` (Green) | Healthy node |
| `◒` (Yellow) | Degraded / warning |
| `○` (Red) | Down / offline |

### Backups Summary

A summary of the backups directory is shown:

```
  Backups: 5 files  |  Latest: 2026-01-15 10:00 UTC
```

### Import Throughput

If a migration import is in progress, a throughput gauge is displayed:

```
  Import:  ████████░░░░░░░░░░░░  2.3 MB/s
```

### Discovery Results

Discovered instances are listed below the gauges:

```
  • http://127.0.0.1:8080 healthy  1.3.1-alpha
  • http://127.0.0.1:8081 degraded 1.3.1-alpha
```

### Health Gauges

Three visual gauges using the `render_gauge` widget:

```
  Health:  |████████████████████| 100%
  Engines: |██████████████░░░░░░|  40%
  Uptime:  |████████████████████| 100%
```

- **Health gauge** — 100% when status is `healthy`/`ok`, 30% otherwise, 0% if missing.
- **Engines gauge** — `(engine_count / 5) * 100`, capped at 100%.
- **Uptime gauge** — 100% when uptime data is present.

## Disconnected View

When not connected, the Dashboard shows a "Getting Started" guide:

```
Not connected to any PrimusDB instance.

Getting Started:
  1. Start a server:   primusdb server start
  2. Connect to it:    primusdb tui --server http://localhost:8080
  3. Or discover:      use Instances section to find running servers
```

## Auto-Refresh

The TUI fetches `/status` every 10 seconds while connected. Health, uptime, version, and engine data update silently. Metrics (`/metrics`) and cluster data are fetched in parallel on each tick.

## Keybindings

| Key | Action |
|-----|--------|
| `r` | Refresh (re-fetches status, metrics, cluster data) |
| `Ctrl+D` | Toggle details view |
| `Tab` | Next section |
| `Shift+Tab` | Previous section |
| `?` | Open help overlay |
| `q` / `Ctrl+C` | Quit TUI |
