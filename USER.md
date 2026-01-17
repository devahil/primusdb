# PrimusDB User Manual

This manual provides comprehensive guidance for users working with PrimusDB databases.

## Getting Started

### Connecting to PrimusDB

#### CLI Connection
```bash
# Connect to local server
primusdb-cli --server http://localhost:8080

# Connect to remote server
primusdb-cli --server http://prod-db.example.com:8080
```

#### API Connection
```bash
# Test connection
curl http://localhost:8080/health

# Check server status
curl http://localhost:8080/status
```

### Database Concepts

PrimusDB provides four storage engines, each optimized for different use cases:

- **Columnar**: Analytical queries, aggregations
- **Document**: Flexible JSON data, content management
- **Relational**: Structured data with relationships
- **Vector**: Similarity search, ML embeddings

## Basic Operations

### Creating Tables

#### Columnar Table
```bash
primusdb-cli crud create --storage-type columnar --table sales --data '{}'
```

#### Document Collection
```bash
primusdb-cli crud create --storage-type document --table users --data '{}'
```

#### Relational Table
```bash
primusdb-cli crud create --storage-type relational --table products --data '{}'
```

### Inserting Data

#### CLI Insert
```bash
# Columnar data
primusdb-cli crud create --storage-type columnar --table sales \
  --data '{"product_id": 1, "amount": 99.99, "date": "2023-12-01"}'

# Document data
primusdb-cli crud create --storage-type document --table users \
  --data '{"name": "John Doe", "email": "john@example.com", "age": 30}'

# Vector data
primusdb-cli crud create --storage-type vector --table embeddings \
  --data '{"id": "vec1", "vector": [0.1, 0.2, 0.3]}'
```

#### API Insert
```bash
# POST /api/v1/crud/{storage_type}/{table}
curl -X POST http://localhost:8080/api/v1/crud/columnar/sales \
  -H "Content-Type: application/json" \
  -d '{"product_id": 1, "amount": 99.99, "date": "2023-12-01"}'
```

### Querying Data

#### Basic Queries
```bash
# Get all records (limit 10)
primusdb-cli crud read --storage-type columnar --table users --limit 10

# Get records with conditions
primusdb-cli crud read --storage-type columnar --table users \
  --conditions '{"amount": {"$gt": 50}}' --limit 5
```

#### API Queries
```bash
# GET /api/v1/crud/{storage_type}/{table}?limit=10&offset=0
curl "http://localhost:8080/api/v1/crud/columnar/sales?limit=10"

# With conditions
curl "http://localhost:8080/api/v1/crud/document/users?conditions=%7B%22age%22%3A%7B%22%24gte%22%3A25%7D%7D"
```

### Updating Data

#### CLI Update
```bash
primusdb-cli crud update --storage-type document --table users \
  --conditions '{"name": "John Doe"}' \
  --data '{"age": 31}'
```

#### API Update
```bash
curl -X PUT http://localhost:8080/api/v1/crud/document/users \
  -H "Content-Type: application/json" \
  -d '{
    "conditions": {"name": "John Doe"},
    "data": {"age": 31}
  }'
```

### Deleting Data

#### CLI Delete
```bash
primusdb-cli crud delete --storage-type columnar --table sales \
  --conditions '{"product_id": 1}'
```

#### API Delete
```bash
curl -X DELETE http://localhost:8080/api/v1/crud/columnar/sales \
  -H "Content-Type: application/json" \
  -d '{"product_id": 1}'
```

## Advanced Operations

### Data Analysis
```bash
# Analyze table patterns
primusdb-cli advanced analyze --storage-type columnar --table sales \
  --conditions '{"date": {"$gte": "2023-01-01"}}'
```

### Table Information
```bash
# Get table metadata
primusdb-cli advanced table-info --storage-type document --table users
```

### Custom Queries
```bash
# Using API
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "storage_type": "columnar",
    "operation": "analyze",
    "table": "sales",
    "conditions": {"amount": {"$gt": 100}}
  }'
```

## Working with Different Storage Engines

### Columnar Engine
Best for analytical queries and aggregations.

```bash
# Insert analytical data
primusdb-cli crud create --storage-type columnar --table analytics \
  --data '{"timestamp": "2023-12-01T10:00:00Z", "metric": "revenue", "value": 1500.50}'

# Query with aggregations (basic analysis)
primusdb-cli advanced analyze --storage-type columnar --table analytics
```

### Document Engine
Flexible JSON storage for unstructured or semi-structured data.

```bash
# Insert document
primusdb-cli crud create --storage-type document --table articles \
  --data '{
    "title": "PrimusDB Guide",
    "content": "Complete database guide...",
    "tags": ["database", "guide"],
    "published": true,
    "author": {"name": "Admin", "email": "admin@example.com"}
  }'

# Query with nested conditions
primusdb-cli crud read --storage-type document --table articles \
  --conditions '{"published": true, "tags": {"$in": ["database"]}}'
```

### Relational Engine
Structured data with table relationships.

```bash
# Insert related data
primusdb-cli crud create --storage-type relational --table orders \
  --data '{"order_id": 1, "user_id": 1, "total": 99.99}'

primusdb-cli crud create --storage-type relational --table order_items \
  --data '{"order_id": 1, "product_id": 1, "quantity": 2}'
```

### Vector Engine
Similarity search for embeddings and ML data.

```bash
# Insert vector data
primusdb-cli crud create --storage-type vector --table image_embeddings \
  --data '{
    "id": "image_001",
    "vector": [0.1, 0.2, 0.3, 0.4, 0.5],
    "metadata": {"filename": "photo.jpg", "category": "nature"}
  }'

# Vector similarity search
primusdb-cli advanced vector-search --table image_embeddings \
  --query-vector "[0.1, 0.2, 0.3, 0.4, 0.5]"
```

## Table Management Operations

### Creating Tables
```bash
# Create a columnar table
primusdb-cli table create --storage-type columnar --table sales \
  --schema '{"fields": [{"name": "id", "type": "integer"}, {"name": "amount", "type": "decimal"}]}'

# Create a document collection
primusdb-cli table create --storage-type document --table users

# Create a relational table
primusdb-cli table create --storage-type relational --table products \
  --schema '{"fields": [{"name": "id", "type": "integer"}, {"name": "name", "type": "string"}]}'
```

### Dropping Tables
```bash
# Drop a table completely
primusdb-cli table drop --storage-type columnar --table old_sales

# Drop a collection
primusdb-cli table drop --storage-type document --table temp_users
```

### Truncating Tables
```bash
# Empty a table but keep structure
primusdb-cli table truncate --storage-type columnar --table sales

# Truncate a collection
primusdb-cli table truncate --storage-type document --table users
```

### Table Information
```bash
# Get table metadata
primusdb-cli table info --storage-type columnar --table sales

# Get collection info
primusdb-cli table info --storage-type document --table users
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

### Node.js Driver
```javascript
const { PrimusDB } = require('primusdb');

async function example() {
  const db = new PrimusDB('localhost', 8080);
  await db.connect();

  // Create
  await db.create('document', 'users', {
    name: 'Alice',
    email: 'alice@example.com'
  });

  // Read
  const users = await db.read('document', 'users',
    { name: 'Alice' }, 10, 0);

  // Update
  await db.update('document', 'users',
    { name: 'Alice' }, { age: 25 });

  // Delete
  await db.delete('document', 'users', { name: 'Alice' });

  await db.disconnect();
}
```

### Python Driver
```python
from primusdb import PrimusDB
import asyncio

async def example():
    db = PrimusDB('localhost', 8080)
    await db.connect()

    # Create
    await db.create('columnar', 'sales', {
        'product_id': 1,
        'amount': 99.99,
        'date': '2023-12-01'
    })

    # Read
    sales = await db.read('columnar', 'sales', {}, 10, 0)

    # Update
    await db.update('columnar', 'sales',
        {'product_id': 1}, {'amount': 109.99})

    # Delete
    await db.delete('columnar', 'sales', {'product_id': 1})

    await db.disconnect()
```

### Java Driver
```java
import com.primusdb.PrimusDB;
import java.util.Map;
import java.util.List;

public class Example {
    public static void main(String[] args) {
        PrimusDB db = new PrimusDB("localhost", 8080);
        db.connect();

        // Create
        Map<String, Object> user = Map.of(
            "name", "Bob",
            "email", "bob@example.com",
            "age", 30
        );
        db.create("document", "users", user);

        // Read
        List<Map<String, Object>> users = db.read("document", "users",
            Map.of("age", Map.of("$gte", 25)), 10, 0);

        // Update
        db.update("document", "users",
            Map.of("name", "Bob"), Map.of("age", 31));

        // Delete
        db.delete("document", "users", Map.of("name", "Bob"));

        db.disconnect();
    }
}
```

### Ruby Driver
```ruby
require 'primusdb'

db = PrimusDB.new('localhost', 8080)
db.connect

# Create
db.create('vector', 'embeddings', {
  id: 'vec1',
  vector: [0.1, 0.2, 0.3],
  metadata: { type: 'text' }
})

# Read
embeddings = db.read('vector', 'embeddings', {}, 10, 0)

# Update
db.update('vector', 'embeddings',
  { id: 'vec1' }, { metadata: { type: 'image' } })

# Delete
db.delete('vector', 'embeddings', { id: 'vec1' })

db.disconnect
```

### Rust Driver
```rust
use primusdb::PrimusDB;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = PrimusDB::new("localhost:8080").await?;
    db.connect().await?;

    // Create
    db.create("relational", "products",
        json!({"name": "Widget", "price": 19.99})).await?;

    // Read
    let products = db.read("relational", "products", None, Some(10), Some(0)).await?;

    // Update
    db.update("relational", "products",
        Some(json!({"name": "Widget"})), json!({"price": 24.99})).await?;

    // Delete
    db.delete("relational", "products", Some(json!({"name": "Widget"}))).await?;

    db.disconnect().await?;
    Ok(())
}
```

## Query Patterns

### Filtering Conditions

#### Equality
```json
{"name": "John"}
```

#### Comparison
```json
{"age": {"$gt": 25}}
{"price": {"$lte": 100}}
```

#### Logical Operators
```json
{"$and": [{"age": {"$gte": 18}}, {"status": "active"}]}
{"$or": [{"category": "electronics"}, {"category": "books"}]}
```

#### Array Operations
```json
{"tags": {"$in": ["urgent", "important"]}}
{"tags": {"$all": ["urgent", "important"]}}
```

### Pagination
```bash
# Page 1 (offset 0)
primusdb-cli crud read --storage-type document --table users --limit 10 --offset 0

# Page 2 (offset 10)
primusdb-cli crud read --storage-type document --table users --limit 10 --offset 10
```

### Sorting (API)
```bash
curl "http://localhost:8080/api/v1/crud/columnar/sales?sort=amount&order=desc&limit=10"
```

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

### Health Checks
```bash
# CLI status
primusdb-cli status

# API health
curl http://localhost:8080/health

# Detailed status
curl http://localhost:8080/status
```

### Query Performance
```bash
# Enable query logging
export RUST_LOG=debug
primusdb-server --log-level debug

# Monitor slow queries in logs
tail -f /var/log/primusdb/primusdb.log | grep "slow\|query"
```

### Resource Monitoring
```bash
# Check server metrics
curl http://localhost:8080/metrics

# Monitor system resources
top -p $(pidof primusdb-server)
```

## Troubleshooting

### Connection Issues
```bash
# Test basic connectivity
ping localhost

# Check if server is running
ps aux | grep primusdb

# Verify port
netstat -tlnp | grep 8080
```

### Query Problems
```bash
# Validate JSON syntax
echo '{"name": "test"}' | jq .

# Check table existence
primusdb-cli advanced table-info --storage-type document --table users
```

### Data Issues
```bash
# Verify data integrity
primusdb-cli advanced analyze --storage-type columnar --table sales

# Check for corruption
curl http://localhost:8080/api/v1/cache/cluster/health
```

## Migration Guide

### From Other Databases

#### MongoDB to Document Engine
```javascript
// MongoDB
db.users.find({age: {$gte: 25}})

// PrimusDB
primusdb-cli crud read --storage-type document --table users \
  --conditions '{"age": {"$gte": 25}}'
```

#### PostgreSQL to Relational Engine
```sql
-- PostgreSQL
SELECT * FROM users WHERE age >= 25 LIMIT 10;

-- PrimusDB
primusdb-cli crud read --storage-type relational --table users \
  --conditions '{"age": {"$gte": 25}}' --limit 10
```

#### Elasticsearch to Vector Engine
```javascript
// Elasticsearch
GET /images/_search
{
  "query": {
    "script_score": {
      "query": {"match_all": {}},
      "script": {
        "source": "cosineSimilarity(params.query_vector, 'vector')",
        "params": {"query_vector": [0.1, 0.2, 0.3]}
      }
    }
  }
}

// PrimusDB
primusdb-cli advanced vector-search --table images \
  --query-vector "[0.1, 0.2, 0.3]"
```

This user manual covers the essential operations and patterns for working with PrimusDB. For administration tasks, refer to the administration manual.