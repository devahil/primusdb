# Mapping File Format

A TOML configuration file that defines how source database objects map to PrimusDB
targets. Use `--mapping path/to/mapping.toml` with `migrate plan`, `migrate import`,
or validate with `migrate mapping path/to/mapping.toml`.

## Structure

```toml
[source]
type = "mysql"
database = "mydb"

[target]
namespace = "staging"
default_engine = "relational"

[[objects]]
source = "mydb.users"
target = "users"
engine = "document"
primary_key = "id"

[[objects.field_mappings]]
source = "id"
target = "user_id"
type_override = "bigint"

[[objects.field_mappings]]
source = "name"
target = "full_name"
```

## Top-Level Sections

### `[source]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | Source database type: `mysql`, `postgres`, `mongodb`, `couchdb` |
| `database` | string | no | Scopes migration to a specific database |

### `[target]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `namespace` | string | yes | PrimusDB namespace to import into |
| `default_engine` | string | yes | Default storage engine when an object doesn't specify one |

Supported engines: `relational`, `columnar`, `document`, `keyvalue`, `vector`.

### `[[objects]]`

Each object entry maps one source object to a PrimusDB target.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `source` | string | yes | Qualified source name (`database.table` or `collection` name) |
| `target` | string | yes | Target object name in PrimusDB |
| `engine` | string | no | Storage engine (defaults to `target.default_engine`) |
| `primary_key` | string | no | Primary key column name |

### `[[objects.field_mappings]]`

Per-column overrides for the mapping.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `source` | string | yes | Source column/field name |
| `target` | string | yes | Target column/field name |
| `type_override` | string | no | Type override hint |

If no field mappings are specified, a 1:1 mapping is generated automatically from
the source schema.

## Default Engine Resolution

If an object's `engine` field is empty or omitted, it inherits the value of
`target.default_engine` from the `[target]` section.

## Examples

### Basic 1:1 Mapping

```toml
[source]
type = "postgres"
database = "analytics"

[target]
namespace = "prod"
default_engine = "columnar"

[[objects]]
source = "analytics.events"
target = "events"
```

### With Field Renames and Type Override

```toml
[source]
type = "mysql"

[target]
namespace = "staging"
default_engine = "relational"

[[objects]]
source = "mydb.orders"
target = "orders"
engine = "relational"
primary_key = "order_id"

[[objects.field_mappings]]
source = "id"
target = "order_id"

[[objects.field_mappings]]
source = "total"
target = "amount"
type_override = "decimal"
```

### Multiple Objects

```toml
[source]
type = "mongodb"
database = "shop"

[target]
namespace = "default"
default_engine = "document"

[[objects]]
source = "shop.users"
target = "users"

[[objects]]
source = "shop.products"
target = "products"
engine = "relational"
primary_key = "sku"
```

## Validation

Use `migrate mapping` to validate a mapping file without running a migration:

```bash
primusdb migrate mapping ./mapping.toml
```

This checks:
- `source.type` is not empty
- `target.namespace` is not empty
- Each object has a non-empty `source`, `target`, and `engine`
- All source objects exist in the schema (when used with `plan` or `import`)
