# CouchDB to PrimusDB Migration

This example migrates a CouchDB instance into PrimusDB.

## Prerequisites

No feature flags needed — CouchDB uses `reqwest` (always available).

## Step 1: Inspect the Source

```bash
primusdb migrate inspect-source --source couchdb \
  --url "http://admin:pass@127.0.0.1:5984"
```

Expected output (abbreviated):

```json
{
  "databases": [
    {
      "name": "users",
      "objects": [{ "name": "users", "row_estimate": 1500 }]
    },
    {
      "name": "blog_posts",
      "objects": [{ "name": "blog_posts", "row_estimate": 320 }]
    }
  ]
}
```

## Step 2: Generate a Migration Plan

```bash
primusdb migrate plan --source couchdb \
  --url "http://admin:pass@127.0.0.1:5984" \
  --target http://localhost:8080 --namespace cms
```

## Step 3: Import the Data

Exclude design documents with `--exclude`:

```bash
primusdb migrate import --source couchdb \
  --url "http://admin:pass@127.0.0.1:5984" \
  --target http://localhost:8080 --namespace cms \
  --mode copy --exclude "_design"
```

## Step 4: Validate

```bash
primusdb migrate validate --target http://localhost:8080 \
  --namespace cms --source couchdb \
  --url "http://admin:pass@127.0.0.1:5984"
```

## Notes

- Each CouchDB database becomes a single object in PrimusDB.
- The `_id` field is preserved and set as the primary key.
- The `_rev` field is automatically excluded from imported data.
- Attachments are not streamed — only the `_attachments` metadata is preserved.
