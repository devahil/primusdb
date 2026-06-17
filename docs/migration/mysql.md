# MySQL Migration

Source implementation for MySQL databases.

## Requirements

- Feature flag: `mysql-source`
- Crate: `mysql` 2.x
- Build: `cargo build --features mysql-source`

## Supported Operations

| Operation | Description |
|-----------|-------------|
| List databases | Inspects `INFORMATION_SCHEMA.COLUMNS` filtered by `DATABASE()` |
| List tables | All tables in the current database |
| Inspect columns | Column name, data type, nullable, max length, primary key flag |
| Primary keys | Detected via `COLUMN_KEY = 'PRI'` |
| Import rows | `SELECT *` with full type conversion |

## Type Mapping

| MySQL Type | PrimusDB / JSON Type |
|------------|----------------------|
| `INT`, `TINYINT`, `SMALLINT`, `MEDIUMINT`, `BIGINT` | Number (signed/unsigned) |
| `FLOAT`, `DOUBLE`, `DECIMAL` | Number or Null |
| `VARCHAR`, `CHAR`, `TEXT`, `ENUM`, `SET` | String |
| `DATE`, `DATETIME`, `TIMESTAMP` | String (`YYYY-MM-DD HH:MM:SS`) |
| `TIME` | String (`[sign]N days HH:MM:SS`) |
| `BLOB`, `BINARY`, `VARBINARY` | String (hex-encoded) |
| `NULL` | Null |

## Example Workflow

```bash
# Inspect
primusdb migrate inspect-source --source mysql \
  --url "mysql://root:secret@localhost:3306/mydb"

# Plan (dry-run)
primusdb migrate plan --source mysql \
  --url "mysql://root:secret@localhost:3306/mydb" \
  --target http://localhost:8080 --namespace staging

# Import
primusdb migrate import --source mysql \
  --url "mysql://root:secret@localhost:3306/mydb" \
  --target http://localhost:8080 --namespace staging \
  --mode copy --batch-size 500

# Validate
primusdb migrate validate --target http://localhost:8080 \
  --namespace staging --source mysql \
  --url "mysql://root:secret@localhost:3306/mydb"
```

## Notes

- Only inspects the database returned by `DATABASE()`. To migrate a different database, change the connection URL.
- Binary types (`BLOB`, `BINARY`) are hex-encoded to strings.
- `GEOMETRY` and spatial types are not explicitly handled — they fall through to the hex-encoded `Bytes` branch.
