# TUI Backups

The Backups section lists backup files found in the local `./backups/` directory. It supports both a **simple list** view and an **enhanced detail** view when a `.index.json` manifest is present.

## How It Works

On render (and on refresh via `r`), the TUI scans the `backups/` directory. If a `backups/.index.json` file exists, the TUI parses it for rich metadata columns.

## Enhanced Detail View

When `.index.json` is present, the TUI renders a table with these columns:

```
  ID    Date                Size   Engines        Status
  ───────────────────────────────────────────────────────────────────
  bk01  2026-01-15T10:00Z   2.3MB  columnar,vector completed
  bk02  2026-01-16T10:00Z   1.1GB  columnar        completed [zstd] [enc]
```

### Columns

| Column | Source | Description |
|--------|--------|-------------|
| **ID** | `id` | Backup identifier |
| **Date** | `created_at` | ISO 8601 timestamp |
| **Size** | `size_bytes` | Formatted as B/KB/MB/GB |
| **Engines** | `engines` | Comma-separated engine names (truncated to 12 chars) |
| **Status** | `status` | Color-coded status badge |

### Status Colors

| Status | Color |
|--------|-------|
| `completed`, `ok` | Green |
| `in_progress`, `running` | Cyan |
| `failed`, `error` | Red |
| `verified` | Yellow |

### Compression & Encryption Tags

Additional tags are appended after the status:

| Tag | Meaning |
|-----|---------|
| `[zstd]` | Compressed with Zstandard |
| `[lz4]` | Compressed with LZ4 |
| `[enc]` | Backup is encrypted |

## Simple List View

When `.index.json` is absent, the TUI falls back to a basic file listing:

```
  Type   Size       Name
  ─────────────────────────────
  SQL      2.3 KB   mydb_20260115.sql
  JSON    14.2 KB   analytics.json
  Parquet  1.5 MB   events.parquet
```

### File Type Detection

| Extension | Label |
|-----------|-------|
| `.sql` | SQL |
| `.json` | JSON |
| `.parquet` | Parquet |
| (directory) | Directory |
| Other | Unknown |

## Backup in Progress

When `backup_in_progress` is `true` (triggered by `Ctrl+B`), a status line appears at the top:

```
  Creating backup... (Ctrl+B pressed)
```

## Limitations

| Operation | Supported in TUI? | CLI Alternative |
|-----------|-------------------|-----------------|
| List backups | Yes (filesystem + index.json) | `primusdb backup list` |
| Create backup | Via `Ctrl+B` | `primusdb backup create` |
| Restore backup | Via `Ctrl+R` | `primusdb backup restore <path>` |
| Verify backup | No | `primusdb backup verify <path>` |
| Inspect backup | No | `primusdb backup inspect <path>` |
| Delete backup | No | `primusdb backup delete <name>` |

## Keybindings

| Key | Action |
|-----|--------|
| `r` | Re-scan the `backups/` directory and reload `.index.json` |
| `Ctrl+B` | Create a new backup |
| `Ctrl+R` | Restore a backup |
