# PostgreSQL Migration

Source implementation for PostgreSQL databases.

## Requirements

- Feature flag: `postgres-source`
- Crate: `tokio-postgres` 0.7
- Build: `cargo build --features postgres-source`

## Supported Operations

| Operation | Description |
|-----------|-------------|
| List schemas | Inspects `information_schema.columns` filtered to `public` schema |
| List tables | All tables in the `public` schema |
| Inspect columns | Column name, data type, nullable, max length, primary key flag |
| Primary keys | Detected via `information_schema.table_constraints` + `key_column_usage` |
| Import rows | `SELECT *` with typed JSON conversion |

## Type Mapping

| PostgreSQL Type | PrimusDB / JSON Type |
|-----------------|----------------------|
| `BOOL` | Boolean |
| `INT2`, `INT4` | Number (i32) |
| `INT8` | Number (i64) |
| `FLOAT4` | Number (f32) or Null |
| `FLOAT8` | Number (f64) or Null |
| `NUMERIC` | String (preserves precision) |
| `VARCHAR`, `CHAR`, `TEXT`, `UUID`, `INET`, `MACADDR` | String |
| `JSON`, `JSONB` | JSON object (preserved as-is) |
| `DATE`, `TIMESTAMP`, `TIMESTAMPTZ` | String |
| `BYTEA` | String |
| `ARRAY` | String (default fallback) |
| `NULL` | Null |

## Example Workflow

```bash
# Inspect
primusdb migrate inspect-source --source postgres \
  --url "postgres://user:pass@localhost:5432/mydb"

# Plan
primusdb migrate plan --source postgres \
  --url "postgres://user:pass@localhost:5432/mydb" \
  --target http://localhost:8080 --namespace staging

# Import
primusdb migrate import --source postgres \
  --url "postgres://user:pass@localhost:5432/mydb" \
  --target http://localhost:8080 --namespace staging \
  --mode copy --batch-size 500

# Validate
primusdb migrate validate --target http://localhost:8080 \
  --namespace staging --source postgres \
  --url "postgres://user:pass@localhost:5432/mydb"
```

## Notes

- Only inspects the `public` schema. Tables in other schemas (e.g. `custom_schema.table_name`) are not discovered.
- `NUMERIC` types are converted to strings to avoid precision loss.
- Arrays and composite types fall through to the default string representation.
