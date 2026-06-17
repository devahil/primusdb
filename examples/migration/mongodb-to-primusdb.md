# MongoDB to PrimusDB Migration

This example migrates a MongoDB database called `shop` into PrimusDB.

## Prerequisites

```bash
cargo build --features mongo-source
```

## Step 1: Inspect the Source

```bash
primusdb migrate inspect-source --source mongodb \
  --url "mongodb://user:pass@localhost:27017/shop"
```

Expected output (abbreviated):

```json
{
  "databases": [
    {
      "name": "shop",
      "objects": [
        { "name": "users", "object_type": "collection" },
        { "name": "products", "object_type": "collection" }
      ]
    }
  ]
}
```

## Step 2: Generate a Migration Plan

```bash
primusdb migrate plan --source mongodb \
  --url "mongodb://user:pass@localhost:27017/shop" \
  --target http://localhost:8080 --namespace shop
```

## Step 3: Import the Data

```bash
primusdb migrate import --source mongodb \
  --url "mongodb://user:pass@localhost:27017/shop" \
  --target http://localhost:8080 --namespace shop \
  --mode copy --batch-size 500
```

## Step 4: Validate

```bash
primusdb migrate validate --target http://localhost:8080 \
  --namespace shop --source mongodb \
  --url "mongodb://user:pass@localhost:27017/shop"
```

## Using the Document Engine

MongoDB documents often contain nested objects. Use the `document` engine
for best results:

```toml
[source]
type = "mongodb"

[target]
namespace = "shop"
default_engine = "document"

[[objects]]
source = "shop.products"
target = "products"
engine = "document"
```
