# TUI Migrations & Migration Wizard

> **DEPRECATED**: This documents the TUI, which was removed from the build in v1.3.2-alpha. Use the CLI/REPL or REST API instead.

The Migrations section provides a guided **12-step migration wizard** for importing data from external databases into PrimusDB. When the wizard is not active, it shows supported sources and CLI commands.

## Supported Sources

| Source | Feature Flag |
|--------|-------------|
| MySQL | `mysql-source` |
| PostgreSQL | `postgres-source` |
| MongoDB | `mongo-source` |
| CouchDB | Always available |

CouchDB support is always compiled in. MySQL, PostgreSQL, and MongoDB require their respective feature flags.

## 12-Step Migration Wizard

Press `Ctrl+M` in the Migrations section to launch the wizard.

### Step 0 — Intro

```
Migration Wizard
  This wizard will guide you through importing data
  from an external database into PrimusDB.

  Steps:
    1. Select source type
    2. Enter the source connection URL
    3. Test connection to the source
    4. Enter the target namespace
    5. Select migration mode
    6. Inspect source objects
    7. Select objects to import
    8. Preview migration plan
    9. Dry-run migration
    10. Confirm and start
    11. Progress + Save report

  Press Enter to begin.
```

### Step 1 — Source Type

```
Select source database type:
  1: MySQL
  2: PostgreSQL
  3: MongoDB
  4: CouchDB
```

Press `1`–`4` to select.

### Step 2 — Source URL

Enter the connection URL for the source database. The URL is typed into the input bar:

```
Examples:
  mysql://user:pass@host:3306/mydb
  postgres://user:pass@host:5432/mydb
  mongodb://user:pass@host:27017/mydb
  http://user:pass@host:5984/mydb
```

### Step 3 — Test Connection

Press `Enter` to test the connection. The TUI sends an `inspect-source` request asynchronously and shows the result:

```
✓ Connection successful — 5 objects found
```

On failure:

```
✗ Connection failed: Connection refused
```

Use `Esc` to go back and edit the URL.

### Step 4 — Namespace

Enter the target PrimusDB namespace (typed into the input bar). This is where the imported data will be stored.

### Step 5 — Migration Mode

```
Select migration mode:
  1: Copy (schema + data)
  2: Schema only
  3: Data only
  4: Dry-run (preview only)
```

Press `1`–`4` to select. **Dry-run mode** skips the actual import and runs a trial only.

### Step 6 — Inspect Objects

Press `Enter` to inspect the source schema. The TUI discovers tables/collections from the source:

```
Inspecting source objects... (async)
```

Once complete, objects appear in step 7 for selection.

### Step 7 — Select Objects

Toggle individual objects with `Space`:

```
Select objects to import (Space to toggle):
  [✓] users
  [✓] orders
  [ ] analytics
```

Press `Enter` when done.

### Step 8 — Preview Plan

```
Migration Plan:
  Source: mysql at mysql://user:pass@host/db
  Target: mynamespace
  Mode: copy
  Objects: users, orders
```

### Step 9 — Dry-Run

Press `Enter` to execute a dry-run import. The TUI runs the migration in `--dry-run` mode and shows results:

```
Dry-run completed: 2 tables, 1500 rows would be imported
```

If step 5 selected "Dry-run" mode, this step confirms and skips to the report.

### Step 10 — Confirm

```
Ready to start migration?

  Source:  mysql at mysql://user:pass@host/db
  Target:  mynamespace
  Mode:    copy
  Objects: users (500 rows), orders (1000 rows)

Press Enter to start the migration.
```

### Step 11+ — Progress & Report

During import, a progress bar updates in real time:

```
  Importing... ████████░░░░░░░░░░░░  40%
  Status: Importing table users...
```

When complete (≥100%):

```
  Import complete!
  Status: Migration completed successfully.
  Report: 2 tables imported, 1500 rows, 0 errors

  Press Enter to save the report.
```

The report can be saved via `Enter` (stored in `app.migration_report`).

## Command Palette Reference

Without the wizard, the Migrations section shows the CLI equivalents:

```
Supported Sources:
  • MySQL     — requires `mysql-source` crate
  • PostgreSQL — requires `postgres-source` crate
  • MongoDB   — requires `mongo-source` crate
  • CouchDB   — requires `couchdb` crate

Commands:
  primusdb migrate inspect-source <source> <url>
  primusdb migrate plan <source> <url> <target>
  primusdb migrate import <source> <url> <target>

  Press Ctrl+M to open the migration wizard
```

## Keybindings

| Key | Action |
|-----|--------|
| `Ctrl+M` | Toggle migration wizard |
| `Enter` | Confirm step / Advance |
| `Esc` | Back one step / Close wizard |
| `1`–`4` | Select source type or mode |
| `Space` | Toggle object selection (step 7) |
| `r` | Refresh (wizard inactive) |
