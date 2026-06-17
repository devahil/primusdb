# TUI Resource Governor Panel

The Resource Governor panel provides a live view of execution governance
activity — active executions, recent violations, and a metrics snapshot.

---

## Access

1. Open the TUI (`primusdb tui`)
2. Navigate to **Resource Governor** in the sidebar (press `Tab` or use arrow keys)
3. Or press the keybindings below from within the panel

---

## Layout

### Status section

Shows the current governor state:

```
  Status: enabled, 3 active executions, 0 blocked, 42 total violations
```

### Active executions

Lists up to 10 currently tracked executions with their workload type,
namespace, and elapsed time.

### Recent violations

Shows up to 8 most recent policy violations with the execution ID, violated
limit, usage vs limit, and enforcement action taken.

### Metrics snapshot

Displays the live metrics from the Prometheus-compatible counters.

---

## Keybindings

| Key | Action                   |
|-----|--------------------------|
| `s` | Toggle status display    |
| `v` | Toggle violations list   |
| `m` | Toggle metrics snapshot  |
| `r` | Refresh all data         |
| `q` | Return to previous panel |

---

## Data Sources

The panel fetches data from the PrimusDB REST API:

| Data              | Endpoint                          |
|-------------------|-----------------------------------|
| Status            | `GET /api/v1/governor/status`     |
| Active executions | `GET /api/v1/governor/executions` |
| Violations        | `GET /api/v1/governor/violations` |
| Metrics           | `GET /api/v1/governor/metrics`    |

---

## Example

```
┌─────────────────────────────────────────────────────────┐
│  Resource Governor                                      │
│                                                         │
│  Status: enabled, 3 active                              │
│                                                         │
│  Active Executions (3):                                 │
│    query-abc  sql       analytics  12.3s                │
│    vec-xyz    vector    ml-prod    5.1s                 │
│    backup-1   backup    ops        45.0s                │
│                                                         │
│  Recent Violations (2):                                 │
│    ⚠ max_memory_mb: 5120 > 2048  blocked               │
│    ⚠ max_import_rows: 5M > 1M  throttled               │
│                                                         │
│  Metrics: active=3 blocked=12 throttled=5 violations=49 │
│                                                         │
│  Press r to refresh                                     │
└─────────────────────────────────────────────────────────┘
```
