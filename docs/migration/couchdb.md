# CouchDB Migration

Source implementation for CouchDB databases.

## Requirements

No feature flag required. Uses `reqwest` (always available).

## Supported Operations

| Operation | Description |
|-----------|-------------|
| List databases | `GET /_all_dbs` |
| Read documents | `GET /{db}/_all_docs?include_docs=true` |
| Preserve `_id` | The `_id` field is included in the data and set as the primary key |
| Exclude `_rev` | The `_rev` field is stripped from imported documents |
| Target engines | Supports all PrimusDB engines (relational, document, keyvalue, vector) |

## Type Mapping

| CouchDB JSON Type | PrimusDB / JSON Type |
|-------------------|----------------------|
| String | String |
| Number | Number |
| Boolean | Boolean |
| Array | Array |
| Object | Object |
| Null | Null |

JSON values are passed through as-is.

## Limitations

- **Attachments**: document attachments are **not** streamed. The `_attachments` metadata field (if present) is included but the binary data is not extracted. Use the CouchDB attachment API separately for binary content.
- **All docs**: reads all documents via `_all_docs?include_docs=true` in a single request. Very large databases may need to be chunked manually.
- **Design documents**: design documents (`_design/*`) are included in the `_all_docs` output. Use `--exclude` to filter them out if desired.

## Example Workflow

```bash
# Inspect
primusdb migrate inspect-source --source couchdb \
  --url "http://admin:pass@127.0.0.1:5984"

# Plan
primusdb migrate plan --source couchdb \
  --url "http://admin:pass@127.0.0.1:5984" \
  --target http://localhost:8080 --namespace staging

# Import (exclude design documents)
primusdb migrate import --source couchdb \
  --url "http://admin:pass@127.0.0.1:5984" \
  --target http://localhost:8080 --namespace staging \
  --mode copy --exclude "_design"

# Validate
primusdb migrate validate --target http://localhost:8080 \
  --namespace staging --source couchdb \
  --url "http://admin:pass@127.0.0.1:5984"
```
