# Validation

After a migration completes, PrimusDB can validate that the data was imported
correctly by comparing row counts via the REST API.

## How Validation Works

1. For each object in the migration plan, the validator sends a `GET` request to
   `{target_url}/namespaces/{namespace}/tables/{target}/count`
2. If the count is greater than 0, the object is considered successfully migrated.
3. If the count is 0 or the request fails, a mismatch is recorded.

Validation runs **automatically** at the end of a successful import (one with no
errors). It can also be run manually with `migrate validate`.

## Manual Validation

```bash
# Validate using source connection (re-generates plan from live source)
primusdb migrate validate \
  --target http://localhost:8080 \
  --namespace default \
  --source mysql \
  --url "mysql://user:pass@host:3306/mydb"

# Validate using a saved plan file
primusdb migrate validate \
  --target http://localhost:8080 \
  --namespace default \
  --report ./migration-plan.json
```

## Interpreting Validation Reports

When validation runs as part of an import, the results are included in the
migration report. Example output:

```
# Migration Report
...
- Objects checked: 3
- Rows matched: 1450
- Checksums matched: 3
- Result: All checks passed
```

If mismatches are found:

```
- Objects checked: 3
- Rows matched: 1000
- Checksums matched: 2
- Mismatch: Object 'users' has 0 rows in target
- Mismatch: Failed to check count for 'events': HTTP 404 - Not Found
```

## Validation Report Fields

| Field | Description |
|-------|-------------|
| `objects_checked` | Number of objects successfully queried |
| `rows_matched` | Total row count across all checked objects |
| `checksums_matched` | Number of objects with count > 0 |
| `mismatches` | List of errors: zero-count objects, HTTP failures, connection errors |

## Limitations

- Validation checks **row counts only**, not individual field values.
- The `count` endpoint must be available on the PrimusDB target server.
- If the import produced errors, automatic validation is skipped to avoid
  misleading results.
