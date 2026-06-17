# MongoDB Migration

Source implementation for MongoDB databases.

## Requirements

- Feature flag: `mongo-source`
- Crate: `mongodb` 2.x
- Build: `cargo build --features mongo-source`

## Supported Operations

| Operation | Description |
|-----------|-------------|
| List databases | Uses `list_database_names()` |
| List collections | All collections in each database |
| Sample documents | Reads all documents from a collection |
| Infer schema | Field names are collected across all documents |
| Import documents | Full BSON-to-JSON conversion of all fields |

## BSON-to-JSON Type Mapping

| BSON Type | JSON Type |
|-----------|-----------|
| `Double` | Number or Null |
| `String` | String |
| `Array` | Array |
| `Document` | Nested JSON Object |
| `Boolean` | Boolean |
| `Null`, `Undefined` | Null |
| `Int32`, `Int64` | Number |
| `DateTime` | String (RFC 3339) |
| `Binary`, `ObjectId`, `Regex`, etc. | String (Debug representation) |

## Document Flattening

Nested BSON documents are **not** flattened. They are preserved as nested JSON objects
in the target. For example:

```json
{
  "name": "Alice",
  "address": {
    "city": "NYC",
    "zip": 10001
  }
}
```

Targets using the `document` engine are best suited for this structure. For `relational`
targets, consider using a TOML mapping file to rename or flatten fields, though
the framework does not automatically unnest nested objects.

## Example Workflow

```bash
# Inspect
primusdb migrate inspect-source --source mongodb \
  --url "mongodb://user:pass@localhost:27017/mydb"

# Plan
primusdb migrate plan --source mongodb \
  --url "mongodb://user:pass@localhost:27017/mydb" \
  --target http://localhost:8080 --namespace staging

# Import
primusdb migrate import --source mongodb \
  --url "mongodb://user:pass@localhost:27017/mydb" \
  --target http://localhost:8080 --namespace staging \
  --mode copy --batch-size 500

# Validate
primusdb migrate validate --target http://localhost:8080 \
  --namespace staging --source mongodb \
  --url "mongodb://user:pass@localhost:27017/mydb"
```

## Notes

- Schema inference reads **all** documents in the collection into memory. For large collections this may be memory-intensive.
- The `_id` field is included in the data and can serve as a primary key.
- Documents are converted using `bson_to_json` — nested documents become nested JSON objects, arrays become JSON arrays.
