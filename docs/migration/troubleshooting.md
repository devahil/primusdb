# Troubleshooting

## Connection Failures

### Source Database

```
Error: NetworkError: MySQL pool: Connection refused (os error 111)
```

- Verify the source database is running and accessible from the PrimusDB host.
- Check the connection URL format:
  - MySQL: `mysql://user:password@host:3306/dbname`
  - PostgreSQL: `postgres://user:password@host:5432/dbname`
  - MongoDB: `mongodb://user:password@host:27017/dbname`
  - CouchDB: `http://user:password@host:5984`

### Target PrimusDB

```
Error: NetworkError: error sending request for url (http://localhost:8080/...)
```

- Ensure the PrimusDB server is running: `primusdb server start`
- Verify the target URL: `--target http://localhost:8080`

## Feature Flags Not Enabled

```
Error: Unsupported: MySQL migration requires the 'mysql-source' feature
```

Rebuild with the required feature flag:

```bash
cargo build --features mysql-source
cargo build --features "mysql-source,postgres-source,mongo-source"
```

## Type Mapping Errors

### Numeric Precision Loss

`NUMERIC`, `DECIMAL`, and `MONEY` types from PostgreSQL are converted to strings
to avoid precision loss. If you need numeric values, use a `type_override` in
your mapping file or post-process the data.

### Binary Data

MySQL `BLOB`/`BINARY` and PostgreSQL `BYTEA` values are hex-encoded to strings.
This is a lossless representation but may not be suitable for all use cases.

## Dry-Run Debugging

Use `--mode dry-run` with `migrate import` or the default dry-run mode of
`migrate plan` to see the full plan without making changes:

```bash
primusdb migrate plan --source mysql --url "mysql://..." --output plan.txt
```

Inspect the output for:
- Correct source-to-target object mapping
- Target engine selection
- Primary key assignment
- Any warnings

## Credential Masking

Connection URLs in migration reports are automatically masked:

```
Original:   mysql://root:supersecret@db.example.com:3306/production
Reported:   mysql://*****@db.example.com:3306/production
```

If credentials appear unmasked in output, ensure the URL uses the standard
`scheme://user:password@host:port/db` format.

## Logs and Reports

### Migration Report

After each import, the CLI prints a markdown report. Save it to a file:

```bash
primusdb migrate import --source mysql --url "..." --output report.md
```

Or generate a standalone report:

```bash
primusdb migrate report --target http://localhost:8080 --namespace default
primusdb migrate report --format json --output report.json
```

### Checking Migration Status

```bash
primusdb migrate status
```

Lists migration report files in the `data/migration_reports/` directory.

## Common Pitfalls

| Issue | Symptom | Fix |
|-------|---------|-----|
| Empty plan | "Objects: 0" | Check `--source` type matches the database |
| Wrong namespace | 404 on validate | Use `--namespace` matching the import target |
| Overwriting data | Duplicate key errors | Use `--overwrite` or `--resume` on re-import |
| Memory exhaustion | OOM during import | Reduce `--batch-size` (default 1000) |
