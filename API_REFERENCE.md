# PrimusDB API Reference
=====================

This document provides comprehensive reference for PrimusDB's REST API (v1.3.1-alpha), including all endpoints, request/response formats, error codes, and usage examples.

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
  },
  "namespace": "myorg.production"
}
```

**Optional fields:**
- `namespace`: Namespace path for data isolation. When set, data is stored in an isolated namespace. Requires `[namespaces] enabled = true` in config (default: enabled).

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

# Document record in namespace
curl -X POST http://localhost:8080/api/v1/crud/document/users \
  -H "Content-Type: application/json" \
  -d '{"name": "John", "email": "john@example.com", "namespace": "myorg.production"}'

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
- `namespace`: Namespace path for data isolation

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

# Query within a namespace
curl "http://localhost:8080/api/v1/crud/document/users?namespace=myorg.production"
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
  "upsert": false,
  "namespace": "myorg.production"
}
```

**Optional fields:**
- `namespace`: Namespace path for data isolation.

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
  "conditions": {"status": "inactive"},
  "namespace": "myorg.production"
}
```

**Optional fields:**
- `namespace`: Namespace path for data isolation.

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
Truncate (empty) a table/collection. Supports optional cascade for relational tables with foreign key references.

**Request Body (optional):**
```json
{
  "cascade": true,
  "namespace": "myorg.production"
}
```

**Optional fields:**
- `namespace`: Namespace path for data isolation.

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
  "operation": "Read",
  "table": "users",
  "conditions": {"age": {"$gte": 25}},
  "data": null,
  "limit": 50,
  "offset": 0,
  "namespace": "myorg.production"
}
```

**Optional fields:**
- `namespace`: Namespace path for data isolation.

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

## Namespace Management

PrimusDB provides hierarchical namespace isolation, allowing multiple tenants or projects to share the same database while keeping their data fully isolated. Namespaces use dot-separated paths (e.g., `myorg.production`). All CRUD and DDL operations can optionally target a namespace.

### Configuration
Namespaces are enabled by default. See the `[namespaces]` section in `ADMIN.md` for configuration options.

### GET /api/v1/namespaces
List all namespaces.

### POST /api/v1/namespaces/{path}
Create a new namespace.

**Request Body (optional):**
```json
{
  "description": "Production namespace for MyOrg",
  "inherit_policy": true
}
```

### GET /api/v1/namespaces/{path}
Get namespace details.

### PUT /api/v1/namespaces/{path}
Update namespace metadata.

### DELETE /api/v1/namespaces/{path}
Delete a namespace.

### GET /api/v1/namespaces/{path}/children
List child namespaces.

### GET /api/v1/namespaces/{path}/effective-policy
Get the effective access policy for a namespace (inherits from parents).

### GET /api/v1/namespaces/{path}/resources
List resources attached to a namespace.

### POST /api/v1/namespaces/{path}/resources
Attach a resource to a namespace.

**Request Body:**
```json
{
  "storage_type": "relational",
  "resource_name": "users"
}
```

### DELETE /api/v1/namespaces/{path}/resources/{storage_type}/{resource_name}
Detach a resource from a namespace.

### GET /api/v1/namespaces/{path}/roles
List roles in a namespace.

### POST /api/v1/namespaces/{path}/roles
Create a role in a namespace.

**Request Body:**
```json
{
  "name": "readonly",
  "permissions": {"Read": true}
}
```

### DELETE /api/v1/namespaces/{path}/roles/{role_id}
Delete a role.

### GET /api/v1/namespaces/{path}/users
List user bindings in a namespace.

### POST /api/v1/namespaces/{path}/users
Add a user binding to a namespace.

**Request Body:**
```json
{
  "user_id": "user_abc123",
  "role_id": "role_readonly"
}
```

### DELETE /api/v1/namespaces/{path}/users/{user_id}
Remove a user binding from a namespace.

## Cluster Management

## Cluster Gateway Management

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

### GET /api/v1/cluster/nodes
List all registered cluster nodes.

**Response:**
```json
{
  "success": true,
  "data": [
    {"node_id": "node1", "address": "10.0.0.1:8080", "status": "active"},
    {"node_id": "node2", "address": "10.0.0.2:8080", "status": "active"}
  ]
}
```

### POST /api/v1/cluster/node/register
Register a new node in the cluster (REST-style DTO).

**Request Body:**
```json
{
  "node_id": "node_006",
  "host": "10.0.0.6",
  "port": 8080,
  "shards": []
}
```

**Response:**
```json
{
  "success": true,
  "data": {"node_id": "node_006", "status": "registered"}
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
    "removed_at": "2024-01-10T12:00:00Z"
  }
}
```

### POST /api/v1/cluster/route
Route a request through the cluster gateway.

**Request Body:**
```json
{
  "strategy": "LeastLoaded",
  "required_shard": null
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "target": "node2:8080",
    "strategy": "LeastLoaded"
  }
}
```

### GET /api/v1/cluster/metrics
Get gateway metrics (total requests, routed, failed, circuit breaks, latency).

**Response:**
```json
{
  "success": true,
  "data": {
    "total_requests": 15000,
    "routed_requests": 14850,
    "failed_requests": 150,
    "circuit_breaks": 3,
    "avg_latency_ms": 12.5,
    "p99_latency_ms": 45.0
  }
}
```

## Federation Management

### GET /api/v1/federation/status
Get federation health status.

**Response:**
```json
{
  "success": true,
  "data": {
    "federation_id": "my-fed",
    "local_cluster": "cluster-us",
    "healthy_clusters": 3,
    "total_clusters": 3,
    "health_ratio": 1.0
  }
}
```

### GET /api/v1/federation/clusters
List all member clusters in the federation.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "cluster_id": "cluster-us",
      "address": "10.0.0.1:8080",
      "status": "Healthy",
      "region": "us-east",
      "avg_latency_ms": 5.2
    },
    {
      "cluster_id": "cluster-eu",
      "address": "10.0.0.2:8080",
      "status": "Healthy",
      "region": "eu-west",
      "avg_latency_ms": 85.0
    }
  ]
}
```

### GET /api/v1/federation/domains
List all DataDomains.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "name": "global-users",
      "description": "User data across all clusters",
      "replication_mode": "Quorum",
      "member_clusters": ["cluster-us", "cluster-eu", "cluster-asia"],
      "collections": ["users"],
      "tables": []
    }
  ]
}
```

### POST /api/v1/federation/domains
Create a new DataDomain.

**Request Body:**
```json
{
  "name": "global-users",
  "description": "User data across all clusters",
  "replication_mode": "Quorum",
  "storage_types": ["document", "relational"],
  "collections": ["users"],
  "tables": ["orders"],
  "member_clusters": ["cluster-us", "cluster-eu", "cluster-asia"]
}
```

### POST /api/v1/federation/domains/{name}/join
Join this cluster to a DataDomain.

**Request Body:**
```json
{
  "collections": ["users", "profiles"],
  "storage_types": ["document"],
  "replication_mode": "Async"
}
```

### POST /api/v1/federation/domains/{name}/leave
Leave a DataDomain.

**Request Body:** `{}`

### POST /api/v1/federation/domains/{name}/balance
Trigger rebalance for a DataDomain.

**Request Body:** `{}`

### GET /api/v1/federation/metrics
Get federation metrics.

**Response:**
```json
{
  "success": true,
  "data": {
    "federation_id": "my-fed",
    "healthy_clusters": 3,
    "suspected_clusters": 0,
    "offline_clusters": 0,
    "total_domains": 2,
    "domains": [
      {"name": "global-users", "member_count": 3, "healthy_members": 3}
    ]
  }
}
```

## Unified Query Language (UQL)

### POST /api/v1/uql
Execute queries across all storage engines using SQL, MongoDB, or Mango syntax.

**Request Body:**
```json
{
  "query": "SELECT * FROM users WHERE age > 25 ORDER BY name ASC LIMIT 10",
  "language": "sql",
  "parameters": null
}
```

**Query Language Options:**
- `sql` — Standard SQL syntax (SELECT, INSERT, UPDATE, DELETE, CREATE, DROP, ALTER, TRUNCATE)
- `mongodb` — MongoDB-style query documents
- `mango` — CouchDB Mango query syntax
- `uql` — Native PrimusDB JSON query format
- `auto` — Auto-detect from query content

**Supported SQL Features:**
- SELECT with WHERE, ORDER BY, LIMIT, OFFSET, DISTINCT, GROUP BY, HAVING
- INSERT, UPDATE, DELETE with RETURNING clause
- JOIN between tables within the same engine or cross-engine
- Window functions (ROW_NUMBER, RANK, DENSE_RANK, etc.)
- Common Table Expressions (WITH clauses)

**Response:**
```json
{
  "success": true,
  "records": [...],
  "total": 10,
  "execution_time_ms": 5,
  "engine_used": "uql",
  "cached": false,
  "affected_rows": 0,
  "warnings": []
}
```

## DDL Operations (Entity-Relationship Model)

> **Namespace support**: All DDL endpoints accept an optional `namespace` parameter — via JSON body (in endpoints that accept a body) or via query parameter `?namespace=myorg.production` (in GET/DELETE endpoints). When set, the operation is scoped to the given namespace and table/sequence/view/trigger names are resolved to namespace-specific physical names.

### POST /api/v1/ddl/{storage_type}/{table}/alter
Execute ALTER TABLE operations (add/drop/modify column, add/drop constraint).

**Request Body (Add Column):**
```json
{
  "operation": "add_column",
  "field": {"name": "discount", "field_type": "Float", "nullable": true},
  "namespace": "myorg.production"
}
```

### POST /api/v1/ddl/{storage_type}/{old_name}/rename
Rename an existing table.

**Request Body:**
```json
{
  "new_name": "new_table_name",
  "namespace": "myorg.production"
}
```

### POST /api/v1/ddl/{storage_type}/_/create_sequence
Create a new sequence for auto-incrementing values.

**Request Body:**
```json
{
  "name": "order_seq",
  "increment": 1,
  "min_value": 1,
  "max_value": 999999999,
  "cycle": false,
  "cache_size": 100,
  "namespace": "myorg.production"
}
```

### POST /api/v1/ddl/{storage_type}/{sequence_name}/nextval
Get the next value from a sequence.

**Query Parameters:**
- `namespace`: Namespace path (optional)

### POST /api/v1/ddl/{storage_type}/{sequence_name}/currval
Get the current value of a sequence.

**Query Parameters:**
- `namespace`: Namespace path (optional)

### POST /api/v1/ddl/{storage_type}/{sequence_name}/setval
Set a sequence to a specific value.

**Request Body:**
```json
{
  "value": 1000,
  "namespace": "myorg.production"
}
```

### DELETE /api/v1/ddl/{storage_type}/{sequence_name}/drop_sequence
Drop a sequence.

**Query Parameters:**
- `namespace`: Namespace path (optional)

### POST /api/v1/ddl/{storage_type}/_/create_view
Create a new view (virtual table).

**Request Body:**
```json
{
  "name": "active_users",
  "query_definition": {"selector": {"status": "active"}},
  "columns": ["id", "name", "email"],
  "referenced_tables": ["users"],
  "namespace": "myorg.production"
}
```

### DELETE /api/v1/ddl/{storage_type}/{view_name}/drop_view
Drop a view.

**Query Parameters:**
- `namespace`: Namespace path (optional)

### POST /api/v1/ddl/{storage_type}/{view_name}/refresh_view
Refresh a materialized view (re-execute query).

**Query Parameters:**
- `namespace`: Namespace path (optional)

### POST /api/v1/ddl/{storage_type}/_/create_trigger
Create a new trigger on a table.

**Request Body:**
```json
{
  "name": "check_age",
  "table_name": "users",
  "timing": "Before",
  "event": "Insert",
  "operation": {"Raise": "Age must be positive"},
  "namespace": "myorg.production"
}
```

### DELETE /api/v1/ddl/{storage_type}/{table_name}/drop_trigger
Drop a trigger.

**Request Body:**
```json
{"trigger_name": "check_age", "namespace": "myorg.production"}
```

## Information Schema

### GET /api/v1/info/{storage_type}/tables
List all tables with metadata for the specified storage type.

**Query Parameters:**
- `namespace`: Namespace path (optional)

### GET /api/v1/info/{storage_type}/{table}/columns
List column definitions for a specific table.

**Query Parameters:**
- `namespace`: Namespace path (optional)

### GET /api/v1/info/{storage_type}/{table}/constraints
List constraint definitions for a specific table.

**Query Parameters:**
- `namespace`: Namespace path (optional)

## Collection Encryption

### POST /api/v1/collection/{collection_name}/encrypt
Enable AES-256-GCM encryption for a document collection.

**Response:**
```json
{
  "success": true,
  "data": {
    "collection": "my_collection",
    "encryption": "enabled",
    "message": "Collection encryption enabled successfully"
  }
}
```

### POST /api/v1/collection/{collection_name}/decrypt
Disable encryption for a document collection.

## Key-Value Store (CouchDB-Compatible API)

All Key-Value endpoints support an optional `?namespace=` query parameter for namespace isolation. When namespaces are enabled (default), the database name is resolved through the namespace hierarchy, ensuring multi-tenant data isolation.

### PUT /api/v1/kv/{database}?namespace={ns}
Create a new Key-Value database.

**Query Parameters:**
- `namespace` (optional): Namespace path (e.g., `myorg.production`)

---

### GET /api/v1/kv/{database}?namespace={ns}
Get database information (doc count, size, etc.).

**Query Parameters:**
- `namespace` (optional): Namespace path

---

### DELETE /api/v1/kv/{database}?namespace={ns}
Delete a Key-Value database and all its documents.

**Query Parameters:**
- `namespace` (optional): Namespace path

---

### PUT /api/v1/kv/{database}/{doc_id}?namespace={ns}
Create or update a document.

**Query Parameters:**
- `namespace` (optional): Namespace path

**Request Body:**
```json
{
  "_id": "my_doc",
  "type": "user",
  "name": "John Doe",
  "tags": ["developer"]
}
```

---

### GET /api/v1/kv/{database}/{doc_id}?namespace={ns}
Get a document by ID.

**Query Parameters:**
- `namespace` (optional): Namespace path

---

### DELETE /api/v1/kv/{database}/{doc_id}?rev={rev}&namespace={ns}
Delete a document (requires current revision).

**Query Parameters:**
- `rev` (required): Current document revision
- `namespace` (optional): Namespace path

---

### GET /api/v1/kv/{database}/_all_docs?namespace={ns}
List all documents. Supports `?include_docs=true`, `?limit=`, `?skip=`.

**Query Parameters:**
- `include_docs` (optional): Include full document bodies (`true`/`false`)
- `limit` (optional): Maximum number of rows
- `skip` (optional): Number of rows to skip
- `namespace` (optional): Namespace path

---

### POST /api/v1/kv/{database}/_find?namespace={ns}
Find documents using Mango-style query selectors.

**Query Parameters:**
- `namespace` (optional): Namespace path

**Request Body:**
```json
{
  "selector": {"age": {"$gte": 25}},
  "limit": 10,
  "skip": 0,
  "sort": [{"age": "desc"}]
}
```

---

### POST /api/v1/kv/{database}/_bulk_docs?namespace={ns}
Insert or update multiple documents in bulk.

**Query Parameters:**
- `namespace` (optional): Namespace path

**Request Body:**
```json
{
  "docs": [{"_id": "doc1"}, {"_id": "doc2"}],
  "all_or_nothing": false
}
```

---

### POST /api/v1/kv/{database}/_index?namespace={ns}
Create a new index for Mango queries.

**Query Parameters:**
- `namespace` (optional): Namespace path

**Request Body:**
```json
{
  "index": {"fields": ["type", "age"]},
  "name": "type-age-index"
}
```

---

### GET /api/v1/kv/{database}/_index?namespace={ns}
List all indexes in the database.

**Query Parameters:**
- `namespace` (optional): Namespace path

---

### POST /api/v1/kv/{database}/_compact?namespace={ns}
Compact the database to reclaim space.

**Query Parameters:**
- `namespace` (optional): Namespace path

---

### POST /api/v1/kv/{database}/_ensure_full_commit?namespace={ns}
Ensure all writes are flushed to disk.

**Query Parameters:**
- `namespace` (optional): Namespace path

---

### GET /api/v1/kv/{database}/_rev_limit?namespace={ns}
Get the revision limit for conflict resolution.

**Query Parameters:**
- `namespace` (optional): Namespace path

---

### PUT /api/v1/kv/{database}/_rev_limit?namespace={ns}
Set the revision limit.

**Query Parameters:**
- `namespace` (optional): Namespace path

**Request Body:**
```json
{"rev_limit": 1000}
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

## Backup & Restore

PrimusDB backup uses a structured binary format with Blake3 checksum verification.

### CLI Backup
```bash
# Full database backup
primusdb backup --destination /path/to/backup

# Client mode
primusdb --mode client --server http://localhost:8080 backup
```

**Backup Format:**
- Magic header: `PRIMUSDBBACKUP` (13 bytes)
- Manifest: Version, timestamp, engine metadata
- Data segments: Typed payloads per storage engine (columnar, vector, document, relational, key-value)
- Schema & index definitions preserved
- Embedded WAL entries for transaction consistency
- Blake3 checksum per segment for integrity

### CLI Restore
```bash
primusdb restore --source /path/to/backup

# Client mode
primusdb --mode client --server http://localhost:8080 restore
```

The restore process validates the magic header, verifies all Blake3 checksums, and reconstructs engines + indexes + WAL.

---

## Blockchain Audit Ledger

The blockchain audit ledger provides an immutable transaction trail. Operations are available through the Rust API (no dedicated REST endpoints; the ledger is accessed programmatically):

```rust
use primusdb::blockchain::AuditLedger;

// Append transactions to the ledger
ledger.append_block(transactions)?;

// Verify chain integrity
let report = ledger.verify_chain()?;

// Look up a transaction
if let Some(tx) = ledger.get_transaction("tx_123")? {
    println!("Found: {:?}", tx);
}

// List all blocks
for block in ledger.list_blocks()? {
    println!("Block {}: {}", block.index, block.block_hash);
}
```

### Transaction Signing

Transactions can be cryptographically signed with Ed25519:

```rust
use primusdb::types::Transaction;

let mut tx = Transaction::new(/* ... */);
let keypair = ed25519_dalek::Keypair::generate(&mut OsRng);
tx.sign(&keypair);

assert!(tx.verify_signature());
```

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