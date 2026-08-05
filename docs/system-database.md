# PrimusDB System Database

## Overview

The system database is an internal metadata, configuration, and audit persistence layer built into PrimusDB. It uses sled for embedded storage and lives at `{data_dir}/system/`. It is automatically initialized on server startup.

## Architecture

```
+----------------------------------------------------+
|                 PrimusDB Server                     |
+----------------------------------------------------+
                         |
                         v
+----------------------------------------------------+
|              SystemDatabase (primus_system)         |
|  sled::Db at {data_dir}/system/                    |
+----------------------------------------------------+
         |            |              |           |
         v            v              v           v
+---------------+ +----------+ +-----------+ +-----------+
| SystemCatalog | | Config   | | Audit     | | Migration |
| Key-value     | | Store    | | Logger    | | Manager   |
| metadata      | | Settings | | Events    | | Schema    |
| (sys_catalog) | | +snapshots| | (sys_audit)| | versioning|
+---------------+ +----------+ +-----------+ +-----------+
```

## Sub-modules

### 1. SystemCatalog (`sys_catalog` tree)
Stores key-value metadata with category labels. Seeded with defaults on first init:

| Key                  | Purpose                        |
|----------------------|--------------------------------|
| `server.version`     | Cargo package version          |
| `server.status`      | Server run state               |
| `engine.registry`    | JSON array of registered engines |
| `system.version`     | Internal schema version        |
| `system.created_at`  | System DB creation timestamp   |

Methods: `get`, `set`, `delete`, `list_by_category`, `list_all`, `to_map`

### 2. ConfigStore (`sys_config` + `sys_config_snapshots` trees)
Persistent key-value configuration with source tracking and snapshot/restore.

**ConfigSource precedence:**
1. `Default` (lowest)
2. `ConfigFile`
3. `EnvironmentVariable`
4. `SystemDatabase`
5. `RuntimeOverride` (highest)

**Validation rules:**
- Key must be non-empty (≤256 chars)
- Only alphanumeric, `.`, `_`, `-` allowed in keys
- Value must not be null

**Operations:**
- `set(key, value, source)` — upsert with source tracking
- `get(key)` — read entry
- `list_all()` / `delete(key)` — enumeration and removal
- `export_bundle()` — serialize all entries as JSON (`ConfigBundle`)
- `import_bundle(bundle)` — bulk load entries
- `create_snapshot(name, description)` — point-in-time snapshot
- `list_snapshots()` / `get_snapshot(id)` / `delete_snapshot(id)` — snapshot lifecycle
- `restore_snapshot(id)` — roll back to snapshot

### 3. AuditLogger (`sys_audit` tree)
Structured event logging with automatic pruning at 10,000 events.

```
AuditEvent {
    id: UUID,
    timestamp: DateTime<Utc>,
    event_type: String,   // e.g. "system.startup", "config.change"
    actor: String,        // e.g. "system", "admin"
    resource: String,     // e.g. "server", "server.port"
    action: String,       // e.g. "init", "update"
    detail: JSON Value,   // arbitrary payload
    success: bool,
}
```

Methods: `log`, `recent(limit)`, `by_type(event_type, limit)`, `count`

### 4. MigrationManager (`sys_migrations` tree)
Schema versioning that tracks and applies pending migrations on startup.

| Version | Name             | Description                |
|---------|------------------|----------------------------|
| 1       | `initial_schema` | Initial system DB schema   |

Methods: `run_pending`, `current_version`, `applied_migrations`, `is_migrated`

## REST API

### Export System Bundle
```
GET /api/v1/system/export
```
Returns all config entries, catalog entries, audit events, and server info as JSON.

### Import Config Bundle
```
POST /api/v1/system/import
Content-Type: application/json

{
    "config_entries": [ ... ]
}
```
Merges config entries into the system database.

## Configuration Endpoints (v1.3.2-alpha)

```
GET    /api/v1/config              — list all config entries
POST   /api/v1/config              — create/update config entry
DELETE /api/v1/config              — delete config entry
POST   /api/v1/config/validate     — validate key/value
GET    /api/v1/config/export       — export config bundle
POST   /api/v1/config/import       — import config bundle
GET    /api/v1/config/snapshots    — list snapshots
POST   /api/v1/config/snapshots    — create snapshot
POST   /api/v1/config/snapshots/:id/restore — restore snapshot
DELETE /api/v1/config/snapshots/:id — delete snapshot
```

## Developer Guide

### Using the System Database

```rust
use primusdb::system::SystemDatabase;

// Open or create
let sys_db = SystemDatabase::open("./data")?;
sys_db.init()?;

// Catalog
sys_db.catalog.set("myapp.key", serde_json::json!("value"), "myapp")?;
let entry = sys_db.catalog.get("myapp.key")?;

// Config
sys_db.config.set("server.port", serde_json::json!(9090), ConfigSource::RuntimeOverride)?;
let bundle = sys_db.config.export_bundle()?;
let snap_id = sys_db.config.create_snapshot("pre-upgrade", "Before upgrade")?;

// Audit
sys_db.audit.log("myapp.event", "system", "resource", "action",
    serde_json::json!({"detail": "value"}), true)?;

// Server info
sys_db.set_server_info(&ServerInfo {
    server_id: "uuid".into(),
    version: "1.3.2-alpha".into(),
    node_id: "node-1".into(),
    cluster_mode: false,
    started_at: chrono::Utc::now(),
    engine_types: vec!["columnar".into()],
})?;
```

### Adding a Migration

Edit `src/system/migrations.rs`:

```rust
fn apply_migration(&self, version: u64) -> crate::Result<()> {
    let name = match version {
        1 => "initial_schema",
        2 => "my_new_migration",  // <-- add here
        _ => return Err(...)
    };
    // ... migration logic
}
```

Then bump `SYSTEM_SCHEMA_VERSION` in `src/system/mod.rs`.

## Export / Backup Integration

The system database can be exported as a JSON bundle for backup purposes:

```bash
curl http://localhost:8080/api/v1/system/export > system-backup.json
```

To restore:

```bash
curl -X POST http://localhost:8080/api/v1/system/import \
  -H "Content-Type: application/json" \
  -d @system-backup.json
```

This allows CLI and external tools to read, backup, and restore server configuration.
