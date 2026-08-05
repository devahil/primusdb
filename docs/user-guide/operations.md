# PrimusDB User Manual
===================

This manual provides comprehensive guidance for users working with PrimusDB v1.3.2-alpha databases.

## Authentication

For authentication setup, API token management, user registration, roles, and multi-tenant segments, see Security Guide: `../security/overview.md`.

## Encryption

See Security Guide: `../security/overview.md`.

## Getting Started

For connection instructions and database concepts, see Quickstart: `../getting-started/quickstart.md` and Storage Engines: `../features/storage-engines.md`.

## Basic Operations

For basic CRUD operations including creating tables, inserting, querying, updating, and deleting data, see Querying: `../usage/querying.md`.

### Unified Query Language (UQL)

PrimusDB supports querying across all storage engines using the Unified Query Language (UQL). This allows you to use SQL, MongoDB, or Mango syntax to query any storage type.

#### Using SQL
```bash
# Execute SQL query via UQL
curl -X POST http://localhost:8080/api/v1/uql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "SELECT * FROM users WHERE age > 25",
    "language": "sql"
  }'
```

#### Using MongoDB-style Queries
```bash
# Execute MongoDB query via UQL
curl -X POST http://localhost:8080/api/v1/uql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{\"users\": {\"age\": {\"$gt\": 25}}}",
    "language": "mongodb"
  }'
```

#### Using Mango Queries
```bash
# Execute Mango query via UQL
curl -X POST http://localhost:8080/api/v1/uql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{\"selector\": {\"age\": {\"$gt\": 25}}}",
    "language": "mango"
  }'
```

#### Cross-Engine Joins
```bash
# Join data from multiple storage engines
curl -X POST http://localhost:8080/api/v1/uql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "SELECT u.name, v.embedding_score FROM users u JOIN vectors v ON u.id = v.user_id",
    "language": "sql"
  }'
```

For advanced SQL queries (GROUP BY, HAVING, ORDER BY, DISTINCT, RETURNING), see Querying: `../usage/querying.md`.

For updating and deleting data, see Querying: `../usage/querying.md`.

## Advanced Operations

For data analysis, table metadata, and custom queries, see CLI Usage: `../usage/cli.md`.

## Working with Different Storage Engines

See Storage Engines: `../features/storage-engines.md`.

## Table Management Operations

### Creating Tables
```bash
# Create a columnar table
primusdb db create sales --engine columnar

# Create a document collection
primusdb db create users --engine document

# Create a relational table
primusdb db create products --engine relational
```

### Dropping Tables
```bash
# Drop a table completely
primusdb db drop old_sales

# Drop a collection
primusdb db drop temp_users
```

### Truncating Tables
```bash
# Empty a table but keep structure
primusdb query "TRUNCATE TABLE sales"

# Truncate a collection
primusdb query "TRUNCATE TABLE users"

# Truncate with CASCADE (also truncates child tables with FK references)
curl -X POST http://localhost:8080/api/v1/crud/relational/orders/truncate \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{"cascade": true}'
```

### ALTER TABLE Operations

#### Add Column
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/products/alter \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "operation": "add_column",
    "field": {"name": "discount", "field_type": "Float", "nullable": true, "default_value": 0.0}
  }'
```

#### Drop Column
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/products/alter \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "operation": "drop_column",
    "column": "old_field"
  }'
```

#### Modify Column
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/products/alter \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "operation": "modify_column",
    "field": {"name": "price", "field_type": "Decimal(10,2)"}
  }'
```

#### Add Constraint (Foreign Key, Unique, Check)
```bash
# Add foreign key constraint
curl -X POST http://localhost:8080/api/v1/ddl/relational/orders/alter \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "operation": "add_constraint",
    "constraint": {
      "name": "fk_user_id",
      "constraint_type": "ForeignKey",
      "fields": ["user_id"],
      "definition": {
        "references_table": "users",
        "references_field": "id",
        "on_delete": "Cascade",
        "on_update": "Restrict"
      }
    }
  }'
```

#### Drop Constraint
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/orders/alter \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "operation": "drop_constraint",
    "constraint_name": "fk_user_id"
  }'
```

#### Rename Table
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/old_name/rename \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{"new_name": "new_name"}'
```

### Sequences

Sequences generate unique numeric values for auto-incrementing columns.

#### Create Sequence
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/_/create_sequence \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "name": "order_number_seq",
    "increment": 1,
    "min_value": 1,
    "max_value": 999999999,
    "cycle": false,
    "cache_size": 100
  }'
```

#### Next Value
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/order_number_seq/nextval \
  -H "Authorization: Bearer YOUR_TOKEN"
```

#### Current Value
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/order_number_seq/currval \
  -H "Authorization: Bearer YOUR_TOKEN"
```

#### Set Value
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/order_number_seq/setval \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{"value": 1000}'
```

#### Drop Sequence
```bash
curl -X DELETE http://localhost:8080/api/v1/ddl/relational/order_number_seq/drop_sequence \
  -H "Authorization: Bearer YOUR_TOKEN"
```

### Views (Virtual Tables)

Views are stored queries that act like virtual tables.

#### Create View
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/_/create_view \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "name": "active_users",
    "query_definition": {"selector": {"status": "active"}},
    "columns": ["id", "name", "email"],
    "referenced_tables": ["users"]
  }'
```

#### Drop View
```bash
curl -X DELETE http://localhost:8080/api/v1/ddl/relational/active_users/drop_view \
  -H "Authorization: Bearer YOUR_TOKEN"
```

#### Refresh View (Materialized)
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/active_users/refresh_view \
  -H "Authorization: Bearer YOUR_TOKEN"
```

### Triggers

Triggers automatically execute actions when specified events occur on a table.

#### Create Trigger
```bash
curl -X POST http://localhost:8080/api/v1/ddl/relational/_/create_trigger \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "name": "check_age_before_insert",
    "table_name": "users",
    "timing": "Before",
    "event": "Insert",
    "operation": {"Raise": "Age must be positive"}
  }'
```

#### Drop Trigger
```bash
curl -X DELETE http://localhost:8080/api/v1/ddl/relational/users/drop_trigger \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{"trigger_name": "check_age_before_insert"}'
```

### Information Schema

Query database metadata programmatically.

#### List All Tables
```bash
curl -X GET http://localhost:8080/api/v1/info/relational/tables \
  -H "Authorization: Bearer YOUR_TOKEN"
```

#### List Table Columns
```bash
curl -X GET http://localhost:8080/api/v1/info/relational/users/columns \
  -H "Authorization: Bearer YOUR_TOKEN"
```

#### List Table Constraints
```bash
curl -X GET http://localhost:8080/api/v1/info/relational/users/constraints \
  -H "Authorization: Bearer YOUR_TOKEN"
```
```bash
# Get table metadata
primusdb db describe sales --schema

# Get collection info
primusdb db describe users
```

## Transactions

### Basic Transaction Flow
```bash
# Begin transaction
curl -X POST http://localhost:8080/api/v1/transaction/begin \
  -H "Content-Type: application/json" \
  -d '{"isolation_level": "read_committed"}'
# Response: {"transaction_id": "tx_123"}

# Execute operations within transaction
curl -X POST http://localhost:8080/api/v1/crud/columnar/sales \
  -H "Content-Type: application/json" \
  -H "X-Transaction-ID: tx_123" \
  -d '{"product_id": 2, "amount": 49.99}'

# Commit transaction
curl -X POST http://localhost:8080/api/v1/transaction/tx_123/commit

# Or rollback
curl -X POST http://localhost:8080/api/v1/transaction/tx_123/rollback
```

## Language-Specific Usage

For Node.js, Python, Java, Ruby, and Rust driver usage, see Drivers: `../usage/drivers.md`.

## Query Patterns

For filtering conditions, pagination, and sorting, see Querying: `../usage/querying.md`.

## Cluster Gateway Operations

For cluster status, node management, and request routing, see Cluster Management: `../operations/cluster-management.md`.

## Federation Operations

For federated cluster management, DataDomains, and replication, see Cluster Management: `../operations/cluster-management.md`.

## Error Handling

### Common Error Codes
- `400 Bad Request`: Invalid query parameters
- `404 Not Found`: Table or record not found
- `409 Conflict`: Constraint violation
- `500 Internal Server Error`: Server-side error

### Error Response Format
```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "Invalid query parameters",
    "details": {
      "field": "limit",
      "value": -1,
      "reason": "must be positive"
    }
  }
}
```

### Handling Errors in Code
```javascript
try {
  const result = await db.read('document', 'users', {}, 10, 0);
  console.log(result);
} catch (error) {
  if (error.code === 'TABLE_NOT_FOUND') {
    console.log('Table does not exist');
  } else {
    console.error('Database error:', error.message);
  }
}
```

## Performance Best Practices

### Choose the Right Storage Engine
- **Columnar**: Analytical queries, aggregations
- **Document**: Flexible schemas, nested data
- **Relational**: Complex relationships, ACID guarantees
- **Vector**: Similarity search, ML applications

### Indexing Strategy
```bash
# Create indexes for frequently queried fields
# (Index creation not yet implemented in CLI)
```

### Query Optimization
- Use specific conditions to reduce data scanning
- Limit result sets appropriately
- Consider pagination for large datasets

### Connection Management
- Reuse connections when possible
- Implement connection pooling in applications
- Close connections when done

## Monitoring and Debugging

For health checks, query performance monitoring, and resource metrics, see Health Checks: `../operations/health-checks.md`, Metrics: `../operations/metrics.md`, and Observability: `../features/observability.md`.

## Troubleshooting

See Troubleshooting: `../operations/troubleshooting.md`.

## Migration Guide

For migration from MongoDB, PostgreSQL, Elasticsearch, and other databases, see Migration: `../migration/README.md`.

## Key-Value Database (CouchDB-Compatible API)

PrimusDB includes a Key-Value storage engine with full CouchDB-compatible REST API.

### Creating a Database

```bash
# Create a Key-Value database
curl -X PUT http://localhost:8080/api/v1/kv/my_database \
  -H "Authorization: Bearer YOUR_TOKEN"
```

### Getting Database Info

```bash
# Get database information
curl -X GET http://localhost:8080/api/v1/kv/my_database \
  -H "Authorization: Bearer YOUR_TOKEN"

# Response:
# {
#   "db_name": "my_database",
#   "doc_count": 150,
#   "doc_del_count": 5,
#   "sizes": {"active": 50000, "external": 45000, "file": 60000},
#   "update_seq": 155,
#   "cluster": {"q": 8, "n": 3, "w": 2, "r": 2}
# }
```

### Creating/Updating Documents

```bash
# Create or update a document
curl -X PUT http://localhost:8080/api/v1/kv/my_database/my_doc_id \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "_id": "my_doc_id",
    "type": "user",
    "name": "John Doe",
    "age": 30,
    "tags": ["developer", "admin"]
  }'

# Response:
# {
#   "ok": true,
#   "id": "my_doc_id",
#   "rev": "1-abc123"
# }
```

### Getting a Document

```bash
# Get document by ID
curl -X GET http://localhost:8080/api/v1/kv/my_database/my_doc_id \
  -H "Authorization: Bearer YOUR_TOKEN"
```

### Deleting a Document

```bash
# Delete a document (requires current revision)
curl -X DELETE "http://localhost:8080/api/v1/kv/my_database/my_doc_id?rev=1-abc123" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

### All Documents (_all_docs)

```bash
# Get all documents
curl -X GET http://localhost:8080/api/v1/kv/my_database/_all_docs \
  -H "Authorization: Bearer YOUR_TOKEN"

# With document content
curl -X GET "http://localhost:8080/api/v1/kv/my_database/_all_docs?include_docs=true" \
  -H "Authorization: Bearer YOUR_TOKEN"

# With pagination
curl -X GET "http://localhost:8080/api/v1/kv/my_database/_all_docs?limit=10&skip=5" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

### Find Documents (Mango Query)

```bash
# Find documents using MongoDB-style selector
curl -X POST http://localhost:8080/api/v1/kv/my_database/_find \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "selector": {
      "age": {"$gte": 25},
      "type": "user"
    },
    "limit": 10,
    "skip": 0,
    "sort": [{"age": "desc"}]
  }'
```

### Bulk Operations

```bash
# Bulk document insert/update
curl -X POST http://localhost:8080/api/v1/kv/my_database/_bulk_docs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "docs": [
      {"_id": "doc1", "value": 1},
      {"_id": "doc2", "value": 2},
      {"_id": "doc3", "value": 3}
    ]
  }'

# All or nothing mode (all succeed or all fail)
curl -X POST http://localhost:8080/api/v1/kv/my_database/_bulk_docs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "docs": [...],
    "all_or_nothing": true
  }'
```

### Indexes

```bash
# Create an index
curl -X POST http://localhost:8080/api/v1/kv/my_database/_index \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "index": {
      "fields": ["type", "age"]
    },
    "name": "type-age-index"
  }'

# List all indexes
curl -X GET http://localhost:8080/api/v1/kv/my_database/_index \
  -H "Authorization: Bearer YOUR_TOKEN"
```

### Database Maintenance

```bash
# Compact database
curl -X POST http://localhost:8080/api/v1/kv/my_database/_compact \
  -H "Authorization: Bearer YOUR_TOKEN"

# Ensure full commit
curl -X POST http://localhost:8080/api/v1/kv/my_database/_ensure_full_commit \
  -H "Authorization: Bearer YOUR_TOKEN"

# Get revision limit
curl -X GET http://localhost:8080/api/v1/kv/my_database/_rev_limit \
  -H "Authorization: Bearer YOUR_TOKEN"

# Set revision limit
curl -X PUT http://localhost:8080/api/v1/kv/my_database/_rev_limit \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{"rev_limit": 1000}'
```

### Delete Database

```bash
# Delete a Key-Value database
curl -X DELETE http://localhost:8080/api/v1/kv/my_database \
  -H "Authorization: Bearer YOUR_TOKEN"
```

This user manual covers the essential operations and patterns for working with PrimusDB. For administration tasks, refer to the administration manual.