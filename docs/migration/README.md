# Migration Framework

Import data from external databases into PrimusDB.

## Architecture

```
+------------------+     inspect      +------------------+
|   Source DB      | ---------------> |   Schema         |
|  (MySQL/Postgres |                  |  (databases,     |
|   MongoDB/Couch) |                  |   tables, cols)  |
+------------------+                  +--------+---------+
                                               |
                                               | plan
                                               v
+------------------+     import      +------------------+
|   PrimusDB       | <--------------- |   Migration Plan |
|  (REST API)      |                  |  (object         |
|                  |                  |   mappings)      |
+------------------+                  +--------+---------+
        |                                      ^
        | validate                              | mapping file
        v                                      | (optional TOML)
+------------------+                           |
|   Validation     | ---------------------------+
|   Report         |
+------------------+
```

## Quickstart

### Migrate from MySQL

```bash
# Inspect source
primusdb migrate inspect-source --source mysql --url "mysql://user:pass@host:3306/mydb"

# Generate a migration plan (dry-run by default)
primusdb migrate plan --source mysql --url "mysql://user:pass@host:3306/mydb" \
  --target http://localhost:8080 --namespace default

# Import data
primusdb migrate import --source mysql --url "mysql://user:pass@host:3306/mydb" \
  --target http://localhost:8080 --namespace default --mode copy

# Validate
primusdb migrate validate --target http://localhost:8080 --namespace default \
  --source mysql --url "mysql://user:pass@host:3306/mydb"
```

### Migrate from PostgreSQL

```bash
primusdb migrate inspect-source --source postgres --url "postgres://user:pass@host:5432/db"
primusdb migrate plan --source postgres --url "postgres://user:pass@host:5432/db"
primusdb migrate import --source postgres --url "postgres://user:pass@host:5432/db" --mode copy
```

### Migrate from MongoDB

```bash
primusdb migrate inspect-source --source mongodb --url "mongodb://user:pass@host:27017/db"
primusdb migrate plan --source mongodb --url "mongodb://user:pass@host:27017/db"
primusdb migrate import --source mongodb --url "mongodb://user:pass@host:27017/db" --mode copy
```

### Migrate from CouchDB

```bash
primusdb migrate inspect-source --source couchdb --url "http://user:pass@127.0.0.1:5984"
primusdb migrate plan --source couchdb --url "http://user:pass@127.0.0.1:5984"
primusdb migrate import --source couchdb --url "http://user:pass@127.0.0.1:5984" --mode copy
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `migrate inspect-source` | Connect to a source database and display its schema (tables, columns, types, primary keys) |
| `migrate plan` | Generate a migration plan showing how source objects map to PrimusDB targets |
| `migrate import` | Execute the migration — create targets and stream data from source to PrimusDB |
| `migrate validate` | Validate a completed migration by comparing row counts via the PrimusDB REST API |
| `migrate report` | Display or export a migration report (markdown or JSON) |
| `migrate mapping` | Validate a TOML mapping configuration file |

### Migration Modes

| Mode | Flag | Description |
|------|------|-------------|
| Copy | `--mode copy` | Full migration: create schema and import all data |
| Schema-only | `--mode schema-only` | Only create target objects (tables/collections), skip data import |
| Data-only | `--mode data-only` | Only import data — target objects must already exist |
| Dry-run | `--mode dry-run` | Generate the plan and validate the mapping without making any changes |

### Common Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--source` | Source database type (`mysql`, `postgres`, `mongodb`, `couchdb`) | required |
| `--url` | Source database connection URL | required |
| `--target` | PrimusDB server URL | `http://localhost:8080` |
| `--namespace` | Target namespace | `default` |
| `--mapping` | Path to TOML mapping configuration file | auto-generates 1:1 mapping |
| `--batch-size` | Number of rows per batch | `1000` |
| `--mode` | Migration mode | `dry-run` (plan), `copy` (import) |
| `--limit` | Maximum number of rows to import | unlimited |
| `--include` | Only include objects matching a glob pattern | all |
| `--exclude` | Exclude objects matching a glob pattern | none |
| `--overwrite` | Overwrite existing data in target | `false` |
| `--resume` | Resume a partially-completed migration | `false` |
| `--format` | Output format (`table`, `json`) for inspect | `table` |
| `--output` | Write plan/report to a file | stdout |

## Supported Target Engines

| Engine | Description |
|--------|-------------|
| `relational` | Row-oriented storage |
| `columnar` | Column-oriented analytics storage |
| `document` | JSON document store |
| `keyvalue` | Key-value store |
| `vector` | Vector similarity search |

## Feature Flags

| Flag | Enables | Crate |
|------|---------|-------|
| `mysql-source` | MySQL source support | `mysql` 2.x |
| `postgres-source` | PostgreSQL source support | `tokio-postgres` 0.7 |
| `mongo-source` | MongoDB source support | `mongodb` 2.x |

CouchDB support is always available (uses `reqwest` for the CouchDB REST API).

Build with specific sources:

```bash
cargo build --features "mysql-source,postgres-source"
cargo build --features "mongo-source"
cargo build --features "mysql-source,postgres-source,mongo-source"  # all
```

## Security

### Credential Masking

Connection URLs containing credentials are automatically masked in reports and log output:

```
mysql://user:password@host:3306/db  →  mysql://*****@host:3306/db
postgres://user:pass@pg.example.com:5432/mydb  →  postgres://*****@pg.example.com:5432/mydb
```

### Dry-Run Mode

Always run `primusdb migrate plan` (which defaults to dry-run mode) or `--mode dry-run`
before executing a migration. This lets you inspect the full plan — including which
objects will be created, their target engines, field mappings, and any warnings —
without touching the target database.

## Migration Examples

Real-world migration walkthroughs for each supported source are available under `examples/migration/`:

- [MySQL → PrimusDB](./mysql.md)
- [PostgreSQL → PrimusDB](./postgresql.md)
- [MongoDB → PrimusDB](./mongodb.md)
- [CouchDB → PrimusDB](./couchdb.md)

A [sample mapping configuration](./mapping.md) is also available.

## Known Limitations

- **MySQL**: inspects only the current database (`DATABASE()`). Row estimates are not fetched.
- **PostgreSQL**: inspects only the `public` schema. Row estimates are not fetched.
- **MongoDB**: schema inference is based on sampled documents only. Nested documents are not flattened — they are preserved as JSON objects.
- **CouchDB**: each CouchDB database becomes a single object in PrimusDB. Attachments are not streamed (only the `_attachments` field metadata is preserved). The `_rev` field is excluded from data import.
- **Validation**: checks row counts via the PrimusDB REST API `count` endpoint. Does not perform full data comparison.
- **All sources**: data is loaded entirely into memory before batching — large datasets may require increasing available memory.
