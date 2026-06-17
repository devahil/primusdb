# MySQL to PrimusDB Migration

This example migrates a MySQL database called `ecommerce` into PrimusDB.

## Prerequisites

```bash
cargo build --features mysql-source
```

## Step 1: Inspect the Source

```bash
primusdb migrate inspect-source --source mysql \
  --url "mysql://root:secret@localhost:3306/ecommerce"
```

Expected output (abbreviated):

```json
{
  "databases": [
    {
      "name": "ecommerce",
      "objects": [
        { "name": "users", "columns": [ ... ] },
        { "name": "orders", "columns": [ ... ] },
        { "name": "products", "columns": [ ... ] }
      ]
    }
  ]
}
```

## Step 2: Generate a Migration Plan

```bash
primusdb migrate plan --source mysql \
  --url "mysql://root:secret@localhost:3306/ecommerce" \
  --target http://localhost:8080 --namespace ecommerce
```

## Step 3: Import the Data

```bash
primusdb migrate import --source mysql \
  --url "mysql://root:secret@localhost:3306/ecommerce" \
  --target http://localhost:8080 --namespace ecommerce \
  --mode copy --batch-size 1000
```

## Step 4: Validate

```bash
primusdb migrate validate --target http://localhost:8080 \
  --namespace ecommerce --source mysql \
  --url "mysql://root:secret@localhost:3306/ecommerce"
```

Expected output:

```
Validation Report:
  Objects checked: 3
  Rows matched: 1450
  Checksums matched: 3
  Result: All checks passed
```
