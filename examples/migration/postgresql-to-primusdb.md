# PostgreSQL to PrimusDB Migration

This example migrates a PostgreSQL database called `analytics` into PrimusDB.

## Prerequisites

```bash
cargo build --features postgres-source
```

## Step 1: Inspect the Source

```bash
primusdb migrate inspect-source --source postgres \
  --url "postgres://user:pass@localhost:5432/analytics"
```

Expected output (abbreviated):

```json
{
  "databases": [
    {
      "name": "analytics",
      "objects": [
        { "name": "events", "columns": [ ... ] },
        { "name": "sessions", "columns": [ ... ] }
      ]
    }
  ]
}
```

## Step 2: Generate a Migration Plan

```bash
primusdb migrate plan --source postgres \
  --url "postgres://user:pass@localhost:5432/analytics" \
  --target http://localhost:8080 --namespace analytics
```

## Step 3: Import the Data

```bash
primusdb migrate import --source postgres \
  --url "postgres://user:pass@localhost:5432/analytics" \
  --target http://localhost:8080 --namespace analytics \
  --mode copy --batch-size 1000
```

## Step 4: Validate

```bash
primusdb migrate validate --target http://localhost:8080 \
  --namespace analytics --source postgres \
  --url "postgres://user:pass@localhost:5432/analytics"
```

## Using a Mapping File

Save the following as `analytics-mapping.toml`:

```toml
[source]
type = "postgres"
database = "analytics"

[target]
namespace = "analytics"
default_engine = "columnar"

[[objects]]
source = "analytics.events"
target = "events"
engine = "columnar"
primary_key = "event_id"

[[objects.field_mappings]]
source = "id"
target = "event_id"
```

Apply it:

```bash
primusdb migrate import --source postgres \
  --url "postgres://user:pass@localhost:5432/analytics" \
  --target http://localhost:8080 --namespace analytics \
  --mapping analytics-mapping.toml --mode copy
```
