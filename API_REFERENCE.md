# PrimusDB API Reference
=====================

This document provides comprehensive reference for PrimusDB's REST API (v1.1.0), including all endpoints, request/response formats, error codes, and usage examples.

## API Overview

### Base URL
```
http://localhost:8080/api/v1
```

### Content Types
- **Request**: `application/json`
- **Response**: `application/json`
- **Encoding**: UTF-8

### Authentication
```bash
# API Key Authentication
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:8080/api/v1/query

# Or via query parameter
curl "http://localhost:8080/api/v1/query?api_key=YOUR_API_KEY"
```

### Rate Limiting
- **Limit**: 1000 requests per minute per IP
- **Headers**:
  - `X-RateLimit-Limit`: Maximum requests per time window
  - `X-RateLimit-Remaining`: Remaining requests in current window
  - `X-RateLimit-Reset`: Time when limit resets (Unix timestamp)

### Response Format
```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "timestamp": "2024-01-10T12:00:00Z",
  "request_id": "req_1234567890"
}
```

### Error Response Format
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid request parameters",
    "details": {
      "field": "table",
      "reason": "table name cannot be empty"
    }
  },
  "timestamp": "2024-01-10T12:00:00Z",
  "request_id": "req_1234567890"
}
```

## Authentication

PrimusDB provides a comprehensive authentication system with user/password login, API tokens, and role-based access control (RBAC).

### Authentication Flow

1. **Login**: Authenticate with username/password to get user info
2. **Get Token**: Create an API token using your credentials
3. **Use Token**: Include the token in subsequent requests

```bash
# Step 1: Login
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin123"}'

# Step 2: Create API Token
curl -X POST http://localhost:8080/api/v1/auth/token/create \
  -H "Content-Type: application/json" \
  -d '{"authorization": "YOUR_TOKEN", "name": "my-token", "scopes": [{"resource": "All", "actions": ["Read", "Write"]}], "expires_in_hours": 8760}'

# Step 3: Use the API Token
curl -X POST http://localhost:8080/api/v1/crud/document/users \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_NEW_TOKEN" \
  -d '{"name": "John", "email": "john@example.com"}'
```

### User Roles

| Role | Description | Permissions |
|------|-------------|-------------|
| `admin` | Full system access | All operations on all resources |
| `developer` | Full data access | Read, Write, Create, Delete on all resources |
| `analyst` | Read-only access | Read on columnar, vector, document, relational |
| `readonly` | Minimal read | Read on all resources |
| `cluster_node` | Node authentication | Cluster operations |

### Token Scopes

Tokens can be scoped to specific resources and actions:

```json
{
  "scopes": [
    {"resource": "Document", "actions": ["Read", "Write"]},
    {"resource": "Columnar", "actions": ["Read"]}
  ]
}
```

Resource types: `Columnar`, `Vector`, `Document`, `Relational`, `Cluster`, `Admin`, `All`
Actions: `Read`, `Write`, `Delete`, `Create`, `Admin`

## Health & Monitoring Endpoints

### GET /health
Basic health check endpoint.

**Response:**
```json
{
   "success": true,
   "data": {
     "status": "healthy",
     "version": "1.1.0",
     "uptime_seconds": 3600,
     "timestamp": "2024-01-10T12:00:00Z"
   }
}
```

### GET /status
Detailed system status.

**Response:**
```json
{
   "success": true,
   "data": {
     "status": "healthy",
     "version": "1.1.0",
     "uptime_seconds": 3600,
     "engines": {
      "columnar": "available",
      "vector": "available",
      "document": "available",
      "relational": "available"
    },
    "cluster": {
      "enabled": false,
      "nodes": 1,
      "health": "healthy"
    },
    "ai_enabled": true,
    "cache_enabled": true,
    "transactions_enabled": true,
    "timestamp": "2024-01-10T12:00:00Z"
  }
}
```

### GET /metrics
Prometheus-compatible metrics.

**Response:**
```
# HELP primusdb_up PrimusDB service availability
# TYPE primusdb_up gauge
primusdb_up 1

# HELP primusdb_version PrimusDB version
# TYPE primusdb_version gauge
primusdb_version{version="1.0.0"} 1

# HELP primusdb_uptime_seconds Service uptime in seconds
# TYPE primusdb_uptime_seconds counter
primusdb_uptime_seconds 3600

# HELP primusdb_storage_operations_total Total storage operations
# TYPE primusdb_storage_operations_total counter
primusdb_storage_operations_total{engine="columnar"} 150
primusdb_storage_operations_total{engine="vector"} 75
primusdb_storage_operations_total{engine="document"} 200
primusdb_storage_operations_total{engine="relational"} 50
```

### GET /api/v1/cache/cluster/health
Cluster health status.

**Response:**
```json
{
  "success": true,
  "data": {
    "cluster_health": "healthy",
    "total_nodes": 3,
    "active_nodes": 3,
    "replication_factor": 3,
    "last_heartbeat": "2024-01-10T12:00:00Z"
  }
}
```

## Authentication Endpoints

### POST /api/v1/auth/login
Authenticate a user and get session information.

**Request:**
```json
{
  "username": "admin",
  "password": "admin123"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "user_id": "user_123",
    "username": "admin",
    "roles": ["admin"],
    "segment_id": null,
    "message": "Login successful. Use /api/v1/auth/token/create to generate an API token."
  }
}
```

### POST /api/v1/auth/register
Register a new user.

**Request:**
```json
{
  "username": "newuser",
  "password": "securepassword",
  "email": "user@example.com",
  "roles": ["developer"],
  "segment_id": null
}
```

### POST /api/v1/auth/token/create
Create an API token for programmatic access.

**Request:**
```json
{
  "authorization": "existing_token",
  "name": "my-api-token",
  "scopes": [
    {"resource": "All", "actions": ["Read", "Write"]}
  ],
  "expires_in_hours": 8760
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "token": "a1b2c3d4e5f6...",
    "token_id": "token_456",
    "expires_at": "2027-02-16T00:00:00Z",
    "message": "Store this token securely. It cannot be retrieved again."
  }
}
```

### POST /api/v1/auth/token/revoke/:token_id
Revoke an API token.

**Request:**
```json
{
  "authorization": "admin_token"
}
```

### GET /api/v1/auth/tokens
List all tokens for the authenticated user.

**Request:**
```json
{
  "authorization": "user_token"
}
```

### GET /api/v1/auth/users
List all users (admin only).

**Request:**
```json
{
  "authorization": "admin_token"
}
```

### GET /api/v1/auth/roles
List all available roles.

### POST /api/v1/auth/segment/create
Create a data segment for multi-tenancy (admin only).

**Request:**
```json
{
  "authorization": "admin_token",
  "name": "tenant-1",
  "description": "Data segment for tenant 1",
  "parent_segment": null
}
```

## CRUD Operations

### POST /api/v1/crud/{storage_type}/{table}
Create a new record.

**Parameters:**
- `storage_type`: `columnar`, `vector`, `document`, `relational`
- `table`: Table/collection name

**Request Body:**
```json
{
  "data": {
    "field1": "value1",
    "field2": 123,
    "field3": true
  },
  "metadata": {
    "created_by": "user123",
    "tags": ["important", "urgent"]
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "rec_1234567890",
    "inserted_at": "2024-01-10T12:00:00Z"
  }
}
```

**Examples:**
```bash
# Columnar record
curl -X POST http://localhost:8080/api/v1/crud/columnar/sales \
  -H "Content-Type: application/json" \
  -d '{"product_id": 1, "amount": 99.99, "date": "2023-12-01"}'

# Document record
curl -X POST http://localhost:8080/api/v1/crud/document/users \
  -H "Content-Type: application/json" \
  -d '{"name": "John", "email": "john@example.com", "age": 30}'

# Vector record
curl -X POST http://localhost:8080/api/v1/crud/vector/embeddings \
  -H "Content-Type: application/json" \
  -d '{"id": "vec1", "vector": [0.1, 0.2, 0.3], "metadata": {"type": "text"}}'
```

### GET /api/v1/crud/{storage_type}/{table}
Query records with optional filtering and pagination.

**Parameters:**
- `storage_type`: Storage engine type
- `table`: Table/collection name

**Query Parameters:**
- `conditions`: JSON conditions for filtering
- `limit`: Maximum number of records (default: 100, max: 1000)
- `offset`: Number of records to skip (default: 0)
- `sort`: Sort field and direction (e.g., "created_at:desc")
- `fields`: Comma-separated list of fields to return

**Examples:**
```bash
# Get all records with pagination
curl "http://localhost:8080/api/v1/crud/columnar/sales?limit=10&offset=0"

# Filter with conditions
curl "http://localhost:8080/api/v1/crud/document/users?conditions=%7B%22age%22%3A%7B%22%24gte%22%3A25%7D%7D"

# Select specific fields
curl "http://localhost:8080/api/v1/crud/relational/products?fields=id,name,price"

# Sort results
curl "http://localhost:8080/api/v1/crud/columnar/sales?sort=amount:desc"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "records": [
      {
        "id": "rec_123",
        "data": {"name": "John", "age": 30},
        "metadata": {"created_at": "2024-01-10T12:00:00Z"}
      }
    ],
    "total_count": 150,
    "limit": 10,
    "offset": 0,
    "has_more": true
  }
}
```

### PUT /api/v1/crud/{storage_type}/{table}
Update existing records.

**Request Body:**
```json
{
  "conditions": {"id": "rec_123"},
  "data": {"age": 31, "updated_at": "2024-01-10T12:30:00Z"},
  "upsert": false
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "updated_count": 1,
    "modified_at": "2024-01-10T12:30:00Z"
  }
}
```

### DELETE /api/v1/crud/{storage_type}/{table}
Delete records.

**Request Body:**
```json
{
  "conditions": {"status": "inactive"}
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "deleted_count": 5,
    "deleted_at": "2024-01-10T12:30:00Z"
  }
}
```

## Table Management Endpoints

### POST /api/v1/crud/{storage_type}/{table}
Create a new table/collection.

**Request Body:**
```json
{
  "operation": "CreateTable",
  "schema": {
    "fields": [
      {"name": "id", "type": "integer"},
      {"name": "name", "type": "string"}
    ]
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Table created successfully"
  }
}
```

### DELETE /api/v1/crud/{storage_type}/{table}
Drop (delete) a table/collection.

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Table dropped successfully"
  }
}
```

### POST /api/v1/crud/{storage_type}/{table}/truncate
Truncate (empty) a table/collection.

**Response:**
```json
{
  "success": true,
  "data": {
    "truncated_count": 1000,
    "truncated_at": "2024-01-10T12:30:00Z"
  }
}
```

### GET /api/v1/table/{storage_type}/{table}/info
Get table/collection information.

**Response:**
```json
{
  "success": true,
  "data": {
    "table_info": {
      "name": "sales",
      "storage_type": "columnar",
      "record_count": 10000,
      "size_bytes": 5242880,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-10T12:00:00Z",
      "schema": {
        "fields": [
          {"name": "id", "type": "integer"},
          {"name": "amount", "type": "decimal"}
        ]
      }
    }
  }
}
```

## Advanced Analytics Endpoints

### POST /api/v1/advanced/analyze/{storage_type}/{table}
Perform data analysis on a table.

**Request Body:**
```json
{
  "conditions": {"date": {"$gte": "2023-01-01"}},
  "metrics": ["count", "sum", "avg", "min", "max"],
  "group_by": ["category", "month"],
  "time_window": "30d"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "analysis": {
      "total_records": 1000,
      "data_patterns": [
        {
          "pattern": "seasonal_trend",
          "confidence": 0.85,
          "description": "Sales increase during holiday seasons"
        }
      ],
      "statistics": {
        "revenue": {
          "sum": 150000.50,
          "avg": 150.00,
          "min": 10.00,
          "max": 5000.00
        }
      },
      "recommendations": [
        "Consider increasing inventory for high-demand periods",
        "Implement dynamic pricing strategy"
      ]
    }
  }
}
```

### POST /api/v1/advanced/predict/{storage_type}/{table}
Make AI predictions using trained models.

**Request Body:**
```json
{
  "model_id": "sales_forecast_model",
  "input_data": {
    "month": "2024-02",
    "marketing_budget": 50000,
    "season": "winter"
  },
  "prediction_count": 3,
  "include_confidence": true
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "predictions": [
      {
        "value": 125000.00,
        "confidence": 0.92,
        "confidence_interval": {
          "lower": 110000.00,
          "upper": 140000.00
        }
      }
    ],
    "model_info": {
      "model_type": "linear_regression",
      "accuracy": 0.89,
      "last_trained": "2024-01-01T00:00:00Z"
    }
  }
}
```

### POST /api/v1/advanced/vector-search/{table}
Perform similarity search on vector data.

**Request Body:**
```json
{
  "query_vector": [0.1, 0.2, 0.3, 0.4, 0.5],
  "limit": 10,
  "distance_metric": "cosine",
  "threshold": 0.8,
  "include_metadata": true
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "results": [
      {
        "id": "vec_123",
        "similarity": 0.95,
        "vector": [0.11, 0.19, 0.31, 0.42, 0.48],
        "metadata": {
          "filename": "image1.jpg",
          "category": "nature"
        }
      }
    ],
    "search_time_ms": 15,
    "total_candidates": 10000
  }
}
```

### POST /api/v1/advanced/cluster/{storage_type}/{table}
Perform clustering analysis on data.

**Request Body:**
```json
{
  "algorithm": "kmeans",
  "num_clusters": 5,
  "features": ["feature1", "feature2", "feature3"],
  "max_iterations": 100,
  "tolerance": 0.001
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "clusters": [
      {
        "id": 0,
        "center": [1.2, 3.4, 2.1],
        "size": 150,
        "members": ["rec_001", "rec_002", "rec_003"]
      }
    ],
    "silhouette_score": 0.75,
    "iterations": 25,
    "converged": true
  }
}
```

### GET /api/v1/table/{storage_type}/{table}/info
Get detailed table/collection information.

**Response:**
```json
{
  "success": true,
  "data": {
    "table_info": {
      "name": "sales",
      "storage_type": "columnar",
      "record_count": 10000,
      "size_bytes": 5242880,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-10T12:00:00Z",
      "indexes": [
        {
          "name": "date_idx",
          "type": "btree",
          "fields": ["date"]
        }
      ],
      "schema": {
        "product_id": "integer",
        "amount": "decimal",
        "date": "date"
      }
    }
  }
}
```

## Transaction Endpoints

### POST /api/v1/transaction/begin
Begin a new transaction.

**Request Body:**
```json
{
  "isolation_level": "read_committed",
  "timeout_seconds": 300,
  "read_only": false
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "transaction_id": "tx_1234567890",
    "isolation_level": "read_committed",
    "started_at": "2024-01-10T12:00:00Z"
  }
}
```

### POST /api/v1/transaction/{transaction_id}/execute
Execute operations within a transaction.

**Request Body:**
```json
{
  "operations": [
    {
      "type": "insert",
      "storage_type": "columnar",
      "table": "sales",
      "data": {"product_id": 1, "amount": 99.99}
    },
    {
      "type": "update",
      "storage_type": "document",
      "table": "inventory",
      "conditions": {"product_id": 1},
      "data": {"stock": {"$inc": -1}}
    }
  ]
}
```

### POST /api/v1/transaction/{transaction_id}/commit
Commit a transaction.

**Response:**
```json
{
  "success": true,
  "data": {
    "transaction_id": "tx_1234567890",
    "committed_at": "2024-01-10T12:00:05Z",
    "operations_count": 2
  }
}
```

### POST /api/v1/transaction/{transaction_id}/rollback
Rollback a transaction.

**Response:**
```json
{
  "success": true,
  "data": {
    "transaction_id": "tx_1234567890",
    "rolled_back_at": "2024-01-10T12:00:10Z",
    "operations_reverted": 2
  }
}
```

## Query Interface

### POST /api/v1/query
Execute complex queries using PrimusDB's query language.

**Request Body:**
```json
{
  "storage_type": "document",
  "query": {
    "collection": "users",
    "filter": {
      "age": {"$gte": 25},
      "status": "active"
    },
    "projection": {"name": 1, "email": 1},
    "sort": {"created_at": -1},
    "limit": 50,
    "skip": 0
  },
  "options": {
    "explain": false,
    "timeout_ms": 5000
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "results": [...],
    "execution_stats": {
      "total_docs_examined": 1000,
      "total_docs_returned": 50,
      "execution_time_ms": 45,
      "index_used": "age_status_idx"
    }
  }
}
```

## Cluster Management

### GET /api/v1/cluster/status
Get cluster status information.

**Response:**
```json
{
  "success": true,
  "data": {
    "cluster_id": "cluster_001",
    "coordinator_node": "node_001",
    "total_nodes": 5,
    "active_nodes": 5,
    "nodes": [
      {
        "id": "node_001",
        "address": "10.0.0.1:8080",
        "status": "active",
        "role": "coordinator",
        "last_heartbeat": "2024-01-10T12:00:00Z"
      }
    ],
    "shards": [...],
    "replication_factor": 3,
    "health_score": 98.5
  }
}
```

### POST /api/v1/cluster/nodes
Register a new node in the cluster.

**Request Body:**
```json
{
  "node_id": "node_006",
  "address": "10.0.0.6:8080",
  "role": "worker",
  "resources": {
    "cpu_cores": 8,
    "memory_gb": 32,
    "storage_gb": 1000
  }
}
```

### DELETE /api/v1/cluster/nodes/{node_id}
Remove a node from the cluster.

**Response:**
```json
{
  "success": true,
  "data": {
    "node_id": "node_006",
    "removed_at": "2024-01-10T12:00:00Z",
    "data_migration_status": "completed"
  }
}
```

## Error Codes

### HTTP Status Codes
- `200 OK`: Successful operation
- `201 Created`: Resource created successfully
- `400 Bad Request`: Invalid request parameters
- `401 Unauthorized`: Authentication required
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: Resource not found
- `409 Conflict`: Resource conflict (e.g., duplicate key)
- `422 Unprocessable Entity`: Validation error
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Server error
- `503 Service Unavailable`: Service temporarily unavailable

### Application Error Codes

#### Validation Errors (4xx)
- `INVALID_REQUEST`: Malformed request
- `MISSING_PARAMETER`: Required parameter missing
- `INVALID_PARAMETER`: Parameter value invalid
- `UNSUPPORTED_OPERATION`: Operation not supported for engine
- `QUOTA_EXCEEDED`: Resource quota exceeded

#### Database Errors (5xx)
- `CONNECTION_ERROR`: Database connection failed
- `QUERY_ERROR`: Query execution failed
- `TRANSACTION_ERROR`: Transaction failed
- `LOCK_TIMEOUT`: Lock acquisition timeout
- `DEADLOCK_DETECTED`: Transaction deadlock

#### Cluster Errors (5xx)
- `NODE_UNAVAILABLE`: Cluster node unavailable
- `CONSENSUS_FAILURE`: Consensus algorithm failed
- `REPLICATION_ERROR`: Data replication failed
- `SHARD_UNAVAILABLE`: Data shard unavailable

#### AI/ML Errors (5xx)
- `MODEL_NOT_FOUND`: Requested model not found
- `PREDICTION_FAILED`: ML prediction failed
- `TRAINING_FAILED`: Model training failed
- `INVALID_MODEL_FORMAT`: Model format invalid

## SDK Examples

### JavaScript/Node.js
```javascript
const PrimusDB = require('primusdb');

const db = new PrimusDB('localhost', 8080);

// CRUD operations
await db.create('document', 'users', { name: 'Alice', age: 30 });
const users = await db.read('document', 'users', { age: { $gte: 25 } });

// Transactions
const tx = await db.beginTransaction();
await db.insert('document', 'users', { name: 'Bob' }, { transactionId: tx.id });
await db.commitTransaction(tx.id);

// AI operations
const analysis = await db.analyze('columnar', 'sales', {
  groupBy: ['category'],
  metrics: ['sum', 'avg']
});
```

### Python
```python
from primusdb import PrimusDB

db = PrimusDB('localhost', 8080)

# Vector search
results = db.vector_search('embeddings', [0.1, 0.2, 0.3], limit=5)

# Analytics
stats = db.analyze('columnar', 'transactions', {
    'metrics': ['sum', 'count'],
    'group_by': ['category']
})

# Clustering
clusters = db.cluster('document', 'customers', num_clusters=3)
```

### Java
```java
PrimusDB db = new PrimusDB("localhost", 8080);

// Batch operations
List<Map<String, Object>> batchData = Arrays.asList(
    Map.of("name", "User1", "score", 85),
    Map.of("name", "User2", "score", 92)
);

List<String> ids = db.batchInsert("document", "users", batchData);

// Advanced queries
QueryResult result = db.query()
    .from("columnar", "sales")
    .where("amount", ">", 100)
    .groupBy("category")
    .aggregate("sum", "amount")
    .limit(10)
    .execute();
```

This API reference provides complete documentation for integrating with PrimusDB. All endpoints support JSON request/response formats and include comprehensive error handling.