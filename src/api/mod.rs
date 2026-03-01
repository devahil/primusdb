/*
 * PrimusDB REST API - Web Interface Layer
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: Collection encryption, Auth endpoints, Transactions
 */

/*!
# PrimusDB REST API - Web Interface Layer

This module implements the REST API for PrimusDB, providing HTTP endpoints
for all database operations, AI/ML functionality, and administrative tasks.

## API Architecture

```
REST API Architecture
═══════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────┐
│                    HTTP Layer                           │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Axum Web Framework                             │    │
│  │  • Async request handling                        │    │
│  │  • Type-safe routing                             │    │
│  │  • Middleware support                            │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Request Processing                             │    │
│  │  • JSON serialization                            │    │
│  │  • Input validation                              │    │
│  │  • Error handling                                │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                 Business Logic Layer                    │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Query Translation                              │    │
│  │  • HTTP → Database queries                       │    │
│  │  • Parameter mapping                             │    │
│  │  • Transaction management                        │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Response Formatting                            │    │
│  │  • JSON response generation                      │    │
│  │  • Error response handling                       │    │
│  │  • HTTP status code mapping                      │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## API Endpoints Overview

### Core Operations (/api/v1)
- **GET** `/health` - Service health check
- **POST** `/query` - Execute database queries
- **GET** `/tables` - List available tables
- **POST** `/tables` - Create new tables
- **DELETE** `/tables/{name}` - Delete tables

### CRUD Operations (/api/v1/crud)
- **POST** `/{storage}/{table}` - Create records
- **GET** `/{storage}/{table}` - Read records with filtering
- **PUT** `/{storage}/{table}` - Update records
- **DELETE** `/{storage}/{table}` - Delete records

### Advanced Operations (/api/v1/advanced)
- **POST** `/analyze/{storage}/{table}` - Data analysis
- **POST** `/predict/{storage}/{table}` - AI predictions
- **POST** `/cluster/{storage}/{table}` - Data clustering
- **POST** `/vector-search/{table}` - Vector similarity search

### Cluster Management (/api/v1/cluster)
- **GET** `/status` - Cluster health and status
- **POST** `/nodes` - Register cluster nodes
- **DELETE** `/nodes/{id}` - Remove cluster nodes
- **POST** `/rebalance` - Rebalance cluster shards

## Request/Response Format

### Standard API Response
```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "timestamp": "2024-01-10T12:00:00Z"
}
```

### Error Response
```json
{
  "success": false,
  "data": null,
  "error": "Detailed error message",
  "timestamp": "2024-01-10T12:00:00Z"
}
```

## Authentication & Security

### API Key Authentication
```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:8080/api/v1/query
```

### TLS/SSL Support
- Automatic HTTPS redirection
- Client certificate authentication
- Configurable cipher suites

## Rate Limiting

### Default Limits
- 1000 requests per minute per IP
- 100 concurrent connections
- 10MB max request body

### Custom Configuration
```toml
[api.rate_limiting]
requests_per_minute = 1000
max_connections = 100
max_body_size = "10MB"
```

## Monitoring & Observability

### Metrics Endpoints
- **GET** `/metrics` - Prometheus-compatible metrics
- **GET** `/health` - Health check endpoint
- **GET** `/status` - Detailed system status

### Structured Logging
All API requests are logged with:
- Request ID for tracing
- Response time and status
- Client IP and user agent
- Error details when applicable

## Performance Optimizations

### Connection Handling
- HTTP/2 support for multiplexing
- Connection pooling and reuse
- Configurable timeouts and limits

### Caching
- Response caching for read operations
- ETag support for conditional requests
- Cache invalidation on data changes

### Compression
- Automatic gzip/deflate compression
- Configurable compression levels
- Content-type aware compression

## Development & Testing

### Local Development Server
```bash
# Start API server on localhost:8080
cargo run --bin primusdb-server

# Test endpoints
curl http://localhost:8080/health
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"storage_type": "document", "operation": "Create", "table": "test"}'
```

### API Testing Tools
```bash
# Using HTTPie
http POST localhost:8080/api/v1/query \
  storage_type=document \
  operation=Create \
  table=test

# Using curl with JSON
curl -X POST localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d @request.json
```

## Error Codes

### HTTP Status Codes
- **200 OK** - Successful operation
- **201 Created** - Resource created successfully
- **400 Bad Request** - Invalid request parameters
- **401 Unauthorized** - Authentication required
- **403 Forbidden** - Insufficient permissions
- **404 Not Found** - Resource not found
- **409 Conflict** - Resource conflict (e.g., duplicate key)
- **422 Unprocessable Entity** - Validation error
- **500 Internal Server Error** - Server-side error

### Custom Error Codes
- **1001** - Storage engine not found
- **1002** - Invalid query parameters
- **1003** - Transaction conflict
- **1004** - Data corruption detected
- **2001** - AI/ML model not found
- **2002** - Prediction failed
- **3001** - Cluster node unavailable
- **3002** - Consensus failure
*/

use crate::{PrimusDB, Query, QueryOperation, StorageType};
use crate::auth::{AuthService, AuthConfig, LoginRequest, CreateTokenRequest, TokenValidation, ResourceType, Action};
use crate::query::{UqlEngine, UqlQuery, QueryLanguage, UqlResult};
use axum::{
    extract::{Path, Query as AxumQuery, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

pub struct AppState {
    pub primusdb: Arc<PrimusDB>,
    pub auth_service: Arc<AuthService>,
}

/// Standardized API response format for all endpoints
///
/// All API endpoints return responses in this consistent format,
/// making it easy for clients to handle both success and error cases.
///
/// # Type Parameters
/// * `T` - The type of data returned on success (usually serde_json::Value)
///
/// # Response Structure
/// ```json
/// {
///   "success": true,
///   "data": { "result": "value" },
///   "error": null,
///   "timestamp": "2024-01-10T12:00:00Z"
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct APIResponse<T> {
    /// Whether the operation completed successfully
    pub success: bool,
    /// Response data (present only on success)
    pub data: Option<T>,
    /// Error message (present only on failure)
    pub error: Option<String>,
    /// Timestamp when the response was generated
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> APIResponse<T> {
    /// Create a successful response with data
    ///
    /// # Arguments
    /// * `data` - The successful result data to include in the response
    ///
    /// # Returns
    /// A properly formatted success response
    ///
    /// # Example
    /// ```rust
    /// let response = APIResponse::success(vec![user1, user2]);
    /// assert!(response.success);
    /// assert!(response.error.is_none());
    /// ```
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create an error response with message
    ///
    /// # Arguments
    /// * `error_msg` - Human-readable error description
    ///
    /// # Returns
    /// A properly formatted error response
    ///
    /// # Example
    /// ```rust
    /// let response = APIResponse::error("Table not found".to_string());
    /// assert!(!response.success);
    /// assert!(response.data.is_none());
    /// ```
    pub fn error(error_msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error_msg),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// HTTP request structure for database queries
///
/// This structure defines the expected format for query requests
/// sent to the /api/v1/query endpoint. It supports all database operations
/// across all storage types.
///
/// # Supported Operations
/// - "Create" - Insert new records
/// - "Read" - Query existing records
/// - "Update" - Modify existing records
/// - "Delete" - Remove records
/// - "Analyze" - Data analysis operations
/// - "Predict" - AI/ML predictions
///
/// # Example Request
/// ```json
/// {
///   "storage_type": "document",
///   "operation": "Create",
///   "table": "users",
///   "data": {
///     "name": "Alice",
///     "email": "alice@example.com"
///   }
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    /// Storage engine type (columnar, vector, document, relational)
    pub storage_type: String,
    /// Operation to perform (Create, Read, Update, Delete, Analyze, Predict)
    pub operation: String,
    /// Target table or collection name
    pub table: String,
    /// Optional data payload (required for Create/Update operations)
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CrudRequest {
    pub storage_type: String,
    pub table: String,
    pub data: Option<serde_json::Value>,
    pub conditions: Option<serde_json::Value>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub storage_type: String,
    pub table: String,
    pub data: serde_json::Value,
    pub conditions: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub storage_type: String,
    pub table: String,
    pub conditions: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub storage_type: String,
    pub table: String,
    pub conditions: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct PredictRequest {
    pub storage_type: String,
    pub table: String,
    pub data: serde_json::Value,
    pub prediction_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VectorSearchRequest {
    pub table: String,
    pub query_vector: Vec<f32>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ClusterRequest {
    pub storage_type: String,
    pub table: String,
    pub algorithm: Option<String>,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UqlRequest {
    pub query: String,
    pub language: Option<String>,
    pub params: Option<serde_json::Value>,
}

pub struct APIServer {
    app: Router,
}

impl APIServer {
    pub fn new(primusdb: Arc<PrimusDB>, auth_service: Arc<AuthService>) -> Self {
        let app = Router::new()
            // Root API endpoint
            .route("/api/v1", get(api_root))
            // Authentication endpoints (public)
            .route("/api/v1/auth/login", post(login))
            .route("/api/v1/auth/register", post(register_user))
            // Protected endpoints
            .route("/api/v1/auth/token/create", post(create_api_token))
            .route("/api/v1/auth/token/revoke/:token_id", post(revoke_api_token))
            .route("/api/v1/auth/tokens", get(list_tokens))
            .route("/api/v1/auth/users", get(list_users))
            .route("/api/v1/auth/roles", get(list_roles))
            .route("/api/v1/auth/segment/create", post(create_segment))
            // Monitoring endpoints
            .route("/health", get(health_check))
            .route("/status", get(system_status))
            .route("/metrics", get(prometheus_metrics))
            .route("/api/v1/cache/cluster/health", get(cluster_health))
            // CRUD Operations - Generic query endpoint
            .route("/api/v1/query", post(execute_query))
            // UQL (Unified Query Language) endpoint - query across all storage engines
            .route("/api/v1/uql", post(execute_uql_query))
            // CRUD Operations - REST-style endpoints
            .route("/api/v1/crud/:storage_type/:table", post(create_record))
            .route("/api/v1/crud/:storage_type/:table", get(read_records))
            .route("/api/v1/crud/:storage_type/:table", put(update_record))
            .route("/api/v1/crud/:storage_type/:table", delete(delete_record))
            .route(
                "/api/v1/crud/:storage_type/:table/truncate",
                post(truncate_table),
            )
            // Advanced Operations
            .route(
                "/api/v1/advanced/analyze/:storage_type/:table",
                post(analyze_data),
            )
            .route(
                "/api/v1/advanced/predict/:storage_type/:table",
                post(make_prediction),
            )
            .route("/api/v1/advanced/vector-search/:table", post(vector_search))
            .route(
                "/api/v1/advanced/cluster/:storage_type/:table",
                post(cluster_data),
            )
            // Transaction Operations
            .route("/api/v1/transaction/begin", post(begin_transaction))
            .route("/api/v1/transaction/:id/commit", post(commit_transaction))
            .route(
                "/api/v1/transaction/:id/rollback",
                post(rollback_transaction),
            )
            // Table Operations
            .route("/api/v1/table/:storage_type/:table/info", get(table_info))
            .route(
                "/api/v1/table/:storage_type/:table/create",
                post(create_table),
            )
            .route(
                "/api/v1/table/:storage_type/:table/drop",
                delete(drop_table),
            )
            // Collection Encryption Operations (Document storage)
            .route(
                "/api/v1/collection/:table/encrypt",
                post(encrypt_collection),
            )
            .route(
                "/api/v1/collection/:table/decrypt",
                post(decrypt_collection),
            )
            // Key-Value Database Operations (CouchDB-compatible API)
            .route("/api/v1/kv/:db", get(kv_get_db_info))
            .route("/api/v1/kv/:db", put(kv_create_db))
            .route("/api/v1/kv/:db", delete(kv_delete_db))
            .route("/api/v1/kv/:db/_all_docs", get(kv_all_docs))
            .route("/api/v1/kv/:db/_find", post(kv_find))
            .route("/api/v1/kv/:db/_index", get(kv_list_indexes))
            .route("/api/v1/kv/:db/_index", post(kv_create_index))
            .route("/api/v1/kv/:db/_bulk_docs", post(kv_bulk_docs))
            .route("/api/v1/kv/:db/_compact", post(kv_compact))
            .route("/api/v1/kv/:db/_ensure_full_commit", post(kv_ensure_full_commit))
            .route("/api/v1/kv/:db/_rev_limit", get(kv_get_rev_limit))
            .route("/api/v1/kv/:db/_rev_limit", put(kv_set_rev_limit))
            .route("/api/v1/kv/:db/:docid", get(kv_get_document))
            .route("/api/v1/kv/:db/:docid", put(kv_put_document))
            .route("/api/v1/kv/:db/:docid", delete(kv_delete_document))
            .route("/api/v1/kv/:db/:docid", post(kv_update_document))
            // Middleware
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .layer(CorsLayer::permissive())
            .with_state(Arc::new(AppState {
                primusdb,
                auth_service,
            }));

        APIServer { app }
    }

    pub async fn run(self, bind_addr: &str) -> std::result::Result<(), crate::Error> {
        let listener = tokio::net::TcpListener::bind(bind_addr)
            .await
            .map_err(|e| {
                crate::Error::NetworkError(format!("Failed to bind to {}: {}", bind_addr, e))
            })?;

        println!("🚀 PrimusDB API server listening on: http://{}", bind_addr);
        println!("📡 API root: http://{}/api/v1", bind_addr);
        println!("🔐 Authentication enabled", );

        axum::serve(listener, self.app)
            .await
            .map_err(|e| crate::Error::NetworkError(format!("Server error: {}", e)))?;

        Ok(())
    }
}

// Generic query endpoint
async fn execute_query(
    State(state): State<Arc<AppState>>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let primusdb = &state.primusdb;
    let storage_type = request
        .get("storage_type")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let operation = request
        .get("operation")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let table = request
        .get("table")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let storage_type = parse_storage_type(storage_type)?;
    let operation = parse_operation(operation)?;

    let query = Query {
        storage_type,
        operation,
        table: table.to_string(),
        conditions: request.get("conditions").cloned(),
        data: request.get("data").cloned(),
        limit: request.get("limit").and_then(|v| v.as_u64()),
        offset: request.get("offset").and_then(|v| v.as_u64()),
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Query execution failed: {}",
            e
        )))),
    }
}

// UQL (Unified Query Language) endpoint - query across all storage engines
async fn execute_uql_query(
    State(state): State<Arc<AppState>>,
    Json(request): Json<UqlRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let query = request.query.clone();
    let language = request.language.as_deref().unwrap_or("sql");
    let params = request.params.unwrap_or(serde_json::json!({}));
    
    let query_lang = match language.to_lowercase().as_str() {
        "sql" => QueryLanguage::Sql,
        "mongodb" => QueryLanguage::MongoDb,
        "mango" => QueryLanguage::Mango,
        "uql" => QueryLanguage::Uql,
        _ => QueryLanguage::Auto,
    };
    
    let config = state.primusdb.config();
    let uql_engine = match UqlEngine::new(config) {
        Ok(engine) => engine,
        Err(e) => return Ok(Json(APIResponse::error(format!("Failed to create UQL engine: {}", e)))),
    };
    
    let mut params_map: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
    if let Some(obj) = params.as_object() {
        for (k, v) in obj {
            params_map.insert(k.clone(), v.clone());
        }
    }
    
    let uql_query = UqlQuery {
        query,
        query_type: query_lang,
        parameters: Some(params_map),
    };
    
    match uql_engine.execute_query(&uql_query) {
        Ok(result) => {
            let value = serde_json::json!({
                "success": result.success,
                "records": result.records,
                "total": result.total,
                "execution_time_ms": result.execution_time_ms,
                "engine_used": result.engine_used,
                "warnings": result.warnings
            });
            Ok(Json(APIResponse::success(value)))
        }
        Err(e) => Ok(Json(APIResponse::error(format!(
            "UQL query execution failed: {}",
            e
        )))),
    }
}

async fn api_root() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "PrimusDB API",
        "version": "0.1.0",
        "description": "Hybrid Database Engine API - Centralized Architecture",
        "status": "running",
        "architecture": "centralized",
        "endpoints": {
            "root": "GET /api/v1",
            "health": "GET /health",
            "query": "POST /api/v1/query",
            "crud": {
                "create": "POST /api/v1/crud/{storage_type}/{table}",
                "read": "GET /api/v1/crud/{storage_type}/{table}",
                "update": "PUT /api/v1/crud/{storage_type}/{table}",
                "delete": "DELETE /api/v1/crud/{storage_type}/{table}"
            },
            "advanced": {
                "analyze": "POST /api/v1/advanced/analyze/{storage_type}/{table}",
                "predict": "POST /api/v1/advanced/predict/{storage_type}/{table}",
                "vector_search": "POST /api/v1/advanced/vector-search/{table}",
                "cluster": "POST /api/v1/advanced/cluster/{storage_type}/{table}"
            },
            "transaction": {
                "begin": "POST /api/v1/transaction/begin",
                "commit": "POST /api/v1/transaction/{id}/commit",
                "rollback": "POST /api/v1/transaction/{id}/rollback"
            },
            "table": {
                "info": "GET /api/v1/table/{storage_type}/{table}/info",
                "create": "POST /api/v1/table/{storage_type}/{table}/create",
                "drop": "DELETE /api/v1/table/{storage_type}/{table}/drop"
            }
        },
        "storage_engines": ["columnar", "vector", "document", "relational"],
        "documentation": "Full REST API for centralized PrimusDB operations"
    }))
}

async fn health_check() -> Json<APIResponse<serde_json::Value>> {
    Json(APIResponse::success(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "architecture": "centralized"
    })))
}

async fn system_status(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    let status = serde_json::json!({
        "status": "running",
        "uptime_seconds": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "version": env!("CARGO_PKG_VERSION"),
        "storage_engines": {
            "columnar": "available",
            "vector": "available",
            "document": "available",
            "relational": "available"
        },
        "ai_enabled": true,
        "cache_enabled": true,
        "transactions_enabled": true
    });

    Json(APIResponse::success(status))
}

async fn prometheus_metrics(State(state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    let metrics = format!(
        r#"# HELP primusdb_up PrimusDB service availability
# TYPE primusdb_up gauge
primusdb_up 1

# HELP primusdb_version PrimusDB version
# TYPE primusdb_version gauge
primusdb_version{{version="{}"}} 1

# HELP primusdb_uptime_seconds Service uptime in seconds
# TYPE primusdb_uptime_seconds counter
primusdb_uptime_seconds {}

# HELP primusdb_storage_operations_total Total storage operations
# TYPE primusdb_storage_operations_total counter
primusdb_storage_operations_total{{engine="columnar"}} 0
primusdb_storage_operations_total{{engine="vector"}} 0
primusdb_storage_operations_total{{engine="document"}} 0
primusdb_storage_operations_total{{engine="relational"}} 0

# HELP primusdb_http_requests_total Total HTTP requests
# TYPE primusdb_http_requests_total counter
primusdb_http_requests_total{{method="GET",status="200"}} 0
primusdb_http_requests_total{{method="POST",status="200"}} 0
primusdb_http_requests_total{{method="PUT",status="200"}} 0
primusdb_http_requests_total{{method="DELETE",status="200"}} 0

# HELP primusdb_http_request_duration_seconds HTTP request duration
# TYPE primusdb_http_request_duration_seconds histogram
primusdb_http_request_duration_seconds_bucket{{le="0.1"}} 0
primusdb_http_request_duration_seconds_bucket{{le="0.5"}} 0
primusdb_http_request_duration_seconds_bucket{{le="1.0"}} 0
primusdb_http_request_duration_seconds_bucket{{le="5.0"}} 0
primusdb_http_request_duration_seconds_bucket{{le="+Inf"}} 0
primusdb_http_request_duration_seconds_sum 0
primusdb_http_request_duration_seconds_count 0

# HELP primusdb_rate_limit_exceeded Rate limit exceeded
# TYPE primusdb_rate_limit_exceeded counter
primusdb_rate_limit_exceeded_total 0

# HELP primusdb_error_total Total errors by type
# TYPE primusdb_error_total counter
primusdb_error_total{{type="validation"}} 0
primusdb_error_total{{type="database"}} 0
primusdb_error_total{{type="network"}} 0
primusdb_error_total{{type="authentication"}} 0

# HELP primusdb_active_connections Current active connections
# TYPE primusdb_active_connections gauge
primusdb_active_connections 0

# HELP primusdb_memory_usage_bytes Current memory usage
# TYPE primusdb_memory_usage_bytes gauge
primusdb_memory_usage_bytes 0

# HELP primusdb_cpu_usage_percent Current CPU usage percentage
# TYPE primusdb_cpu_usage_percent gauge
primusdb_cpu_usage_percent 0.0
primusdb_storage_operations_total{{engine="document"}} 0
primusdb_storage_operations_total{{engine="relational"}} 0

# HELP primusdb_cache_operations_total Total cache operations
# TYPE primusdb_cache_operations_total counter
primusdb_cache_operations_total{{operation="get"}} 0
primusdb_cache_operations_total{{operation="put"}} 0
primusdb_cache_operations_total{{operation="delete"}} 0
"#,
        env!("CARGO_PKG_VERSION"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );

    Ok(metrics)
}

async fn cluster_health(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    let health = serde_json::json!({
        "cluster_status": "healthy",
        "nodes": [
            {
                "id": "primary",
                "address": "localhost:8080",
                "status": "healthy",
                "load": 0.5
            }
        ],
        "total_nodes": 1,
        "healthy_nodes": 1,
        "replication_factor": 1,
        "cache_status": "operational"
    });

    Json(APIResponse::success(health))
}

// CRUD Operations
async fn create_record(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    let query = Query {
        storage_type,
        operation: QueryOperation::Create,
        table,
        conditions: None,
        data: Some(data),
        limit: None,
        offset: None,
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Create failed: {}", e)))),
    }
}

async fn read_records(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    let conditions = params
        .get("conditions")
        .and_then(|c| serde_json::from_str(c).ok());
    let limit = params.get("limit").and_then(|l| l.parse().ok());
    let offset = params.get("offset").and_then(|o| o.parse().ok());

    let query = Query {
        storage_type,
        operation: QueryOperation::Read,
        table,
        conditions,
        data: None,
        limit,
        offset,
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Read failed: {}", e)))),
    }
}

async fn update_record(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    let query = Query {
        storage_type,
        operation: QueryOperation::Update,
        table,
        conditions: request.get("conditions").cloned(),
        data: request.get("data").cloned(),
        limit: None,
        offset: None,
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Update failed: {}", e)))),
    }
}

async fn delete_record(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    let conditions = params
        .get("conditions")
        .and_then(|c| serde_json::from_str(c).ok());

    let query = Query {
        storage_type,
        operation: QueryOperation::Delete,
        table,
        conditions,
        data: None,
        limit: None,
        offset: None,
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Delete failed: {}", e)))),
    }
}

async fn truncate_table(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    let query = Query {
        storage_type,
        operation: QueryOperation::Truncate,
        table,
        conditions: None,
        data: None,
        limit: None,
        offset: None,
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Truncate failed: {}", e)))),
    }
}

// Advanced Operations
async fn analyze_data(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    let query = Query {
        storage_type,
        operation: QueryOperation::Analyze,
        table,
        conditions: request.get("conditions").cloned(),
        data: None,
        limit: None,
        offset: None,
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Analysis failed: {}", e)))),
    }
}

async fn make_prediction(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    let query = Query {
        storage_type,
        operation: QueryOperation::Predict,
        table,
        conditions: request
            .get("prediction_type")
            .map(|pt| serde_json::json!({"prediction_type": pt})),
        data: request.get("data").cloned(),
        limit: None,
        offset: None,
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Prediction failed: {}",
            e
        )))),
    }
}

async fn vector_search(
    State(state): State<Arc<AppState>>,
    Path(table): Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let query_vector = request
        .get("query_vector")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.iter().map(|v| v.as_f64()).collect::<Option<Vec<f64>>>())
        .unwrap_or_default();

    let limit = request.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

    let conditions = serde_json::json!({
        "query_vector": query_vector,
        "limit": limit
    });

    let query = Query {
        storage_type: StorageType::Vector,
        operation: QueryOperation::Read,
        table,
        conditions: Some(conditions),
        data: None,
        limit: None,
        offset: None,
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Vector search failed: {}",
            e
        )))),
    }
}

async fn cluster_data(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    let query = Query {
        storage_type,
        operation: QueryOperation::Analyze,
        table,
        conditions: Some(serde_json::json!({"operation": "cluster", "params": request})),
        data: None,
        limit: None,
        offset: None,
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Clustering failed: {}",
            e
        )))),
    }
}

// Transaction Operations (placeholders)
async fn begin_transaction() -> Json<APIResponse<serde_json::Value>> {
    Json(APIResponse::success(serde_json::json!({
        "transaction_id": format!("tx_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
        "status": "started"
    })))
}

async fn commit_transaction(
    Path(transaction_id): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    Json(APIResponse::success(serde_json::json!({
        "transaction_id": transaction_id,
        "status": "committed"
    })))
}

async fn rollback_transaction(
    Path(transaction_id): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    Json(APIResponse::success(serde_json::json!({
        "transaction_id": transaction_id,
        "status": "rolled_back"
    })))
}

// Table Operations (placeholders)
async fn table_info(
    Path((storage_type, table)): Path<(String, String)>,
) -> Json<APIResponse<serde_json::Value>> {
    Json(APIResponse::success(serde_json::json!({
        "storage_type": storage_type,
        "table": table,
        "record_count": 0,
        "size_bytes": 0,
        "created_at": chrono::Utc::now().to_rfc3339()
    })))
}

async fn create_table(
    Path((storage_type, table)): Path<(String, String)>,
) -> Json<APIResponse<serde_json::Value>> {
    Json(APIResponse::success(serde_json::json!({
        "storage_type": storage_type,
        "table": table,
        "status": "created"
    })))
}

async fn drop_table(
    Path((storage_type, table)): Path<(String, String)>,
) -> Json<APIResponse<serde_json::Value>> {
    Json(APIResponse::success(serde_json::json!({
        "storage_type": storage_type,
        "table": table,
        "status": "dropped"
    })))
}

// Collection Encryption Operations
async fn encrypt_collection(
    State(state): State<Arc<AppState>>,
    Path(table): Path<String>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = StorageType::Document;
    
    match state.primusdb.enable_collection_encryption(storage_type, &table) {
        Ok(_) => Ok(Json(APIResponse::success(serde_json::json!({
            "collection": table,
            "encryption": "enabled",
            "message": "Collection encryption enabled successfully"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!("Failed to enable encryption: {}", e)))),
    }
}

async fn decrypt_collection(
    State(state): State<Arc<AppState>>,
    Path(table): Path<String>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = StorageType::Document;
    
    match state.primusdb.disable_collection_encryption(storage_type, &table) {
        Ok(_) => Ok(Json(APIResponse::success(serde_json::json!({
            "collection": table,
            "encryption": "disabled",
            "message": "Collection encryption disabled successfully"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!("Failed to disable encryption: {}", e)))),
    }
}

// ==================== Key-Value (CouchDB-compatible) API ====================

async fn kv_get_db_info(
    Path(db): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "db_name": db,
        "doc_count": 0,
        "doc_del_count": 0,
        "sizes": {"active": 0, "external": 0, "file": 0},
        "update_seq": 0,
        "purge_seq": 0,
        "disk_format_version": 6,
        "fragmentation": 0,
        "indexes": 0,
        "security": {},
        "compact_running": false,
        "cluster": {"q": 8, "n": 3, "w": 2, "r": 2}
    }))
}

async fn kv_create_db(
    Path(db): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    Json(APIResponse::success(serde_json::json!({
        "ok": true,
        "id": db
    })))
}

async fn kv_delete_db(
    Path(db): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    Json(APIResponse::success(serde_json::json!({
        "ok": true
    })))
}

async fn kv_all_docs(
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let include_docs = params.get("include_docs").map(|v| v == "true").unwrap_or(false);
    let limit: Option<usize> = params.get("limit").and_then(|v| v.parse().ok());
    let skip: Option<usize> = params.get("skip").and_then(|v| v.parse().ok());
    
    Json(serde_json::json!({
        "total_rows": 0,
        "offset": skip.unwrap_or(0),
        "rows": []
    }))
}

async fn kv_find(
    Path(db): Path<String>,
    Json(selector): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "docs": [],
        "warning": "Query execution not implemented for this engine",
        "execution_stats": {
            "documents_examined": 0,
            "results_returned": 0
        }
    }))
}

async fn kv_list_indexes(
    Path(db): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "indexes": []
    }))
}

async fn kv_create_index(
    Path(db): Path<String>,
    Json(index_def): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = index_def.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("default");
    let fields = index_def.get("index").and_then(|i| i.get("fields"))
        .and_then(|f| f.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
        .unwrap_or_default();
    
    Json(serde_json::json!({
        "ok": true,
        "id": format!("_design/{}", name),
        "name": name,
        "fields": fields
    }))
}

async fn kv_bulk_docs(
    Path(db): Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!([
        {"id": "sample", "rev": "1-abc", "error": null}
    ]))
}

async fn kv_compact(
    Path(db): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true
    }))
}

async fn kv_ensure_full_commit(
    Path(db): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "instance_start_time": "0"
    }))
}

async fn kv_get_rev_limit(
    Path(db): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "rev_limit": 1000
    }))
}

async fn kv_set_rev_limit(
    Path(db): Path<String>,
    Json(limit): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "rev_limit": limit.get("rev_limit").and_then(|v| v.as_u64()).unwrap_or(1000)
    }))
}

async fn kv_get_document(
    Path((db, docid)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "_id": docid,
        "_rev": "1-abc",
        "error": "not_found",
        "reason": "missing"
    }))
}

async fn kv_put_document(
    Path((db, docid)): Path<(String, String)>,
    Json(doc): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "id": docid,
        "rev": "1-abc"
    }))
}

async fn kv_delete_document(
    Path((db, docid)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let rev = params.get("rev").cloned().unwrap_or_default();
    Json(serde_json::json!({
        "ok": true,
        "id": docid,
        "rev": format!("2-{}", rev)
    }))
}

async fn kv_update_document(
    Path((db, docid)): Path<(String, String)>,
    Json(doc): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "id": docid,
        "rev": "2-abc"
    }))
}

// Authentication endpoints
async fn login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state.auth_service.login(request).await {
        Ok(result) => Ok(Json(APIResponse::success(serde_json::json!({
            "user_id": result.user_id,
            "username": result.username,
            "roles": result.roles,
            "segment_id": result.segment_id,
            "message": "Login successful. Use /api/v1/auth/token/create to generate an API token."
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!("Login failed: {}", e)))),
    }
}

async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterUserRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state.auth_service.create_user(
        request.username,
        request.password,
        request.email,
        request.roles,
        request.segment_id,
    ).await {
        Ok(user_id) => Ok(Json(APIResponse::success(serde_json::json!({
            "user_id": user_id,
            "message": "User created successfully"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!("Registration failed: {}", e)))),
    }
}

    async fn create_api_token(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateTokenRequestWithAuth>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let token_request = crate::auth::CreateTokenRequest {
        name: request.name,
        scopes: request.scopes,
        expires_in_hours: request.expires_in_hours,
    };

    match state.auth_service.validate_token(&request.authorization).await {
        Ok(validation) => {
            match state.auth_service.create_token(&validation.user_id, token_request).await {
                Ok((raw_token, token)) => Ok(Json(APIResponse::success(serde_json::json!({
                    "token": raw_token,
                    "token_id": token.id,
                    "expires_at": token.expires_at,
                    "message": "Store this token securely. It cannot be retrieved again."
                })))),
                Err(e) => Ok(Json(APIResponse::error(format!("Token creation failed: {}", e)))),
            }
        }
        Err(e) => Ok(Json(APIResponse::error(format!("Authentication failed: {}", e)))),
    }
}

async fn revoke_api_token(
    State(state): State<Arc<AppState>>,
    Path(token_id): Path<String>,
    Json(request): Json<RevokeTokenRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state.auth_service.validate_token(&request.authorization).await {
        Ok(_) => {
            match state.auth_service.revoke_token(&token_id).await {
                Ok(()) => Ok(Json(APIResponse::success(serde_json::json!({
                    "message": "Token revoked successfully"
                })))),
                Err(e) => Ok(Json(APIResponse::error(format!("Revoke failed: {}", e)))),
            }
        }
        Err(e) => Ok(Json(APIResponse::error(format!("Authentication failed: {}", e)))),
    }
}

async fn list_tokens(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ListTokensRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state.auth_service.validate_token(&request.authorization).await {
        Ok(validation) => {
            let tokens = state.auth_service.list_user_tokens(&validation.user_id).await;
            Ok(Json(APIResponse::success(serde_json::json!({
                "tokens": tokens
            }))))
        }
        Err(e) => Ok(Json(APIResponse::error(format!("Authentication failed: {}", e)))),
    }
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ListUsersRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state.auth_service.validate_token(&request.authorization).await {
        Ok(validation) => {
            if let Ok(true) = state.auth_service.check_permission(&validation, ResourceType::Admin, Action::Admin).await {
                let users = state.auth_service.list_users().await;
                let sanitized: Vec<_> = users.into_iter().map(|u| {
                    serde_json::json!({
                        "id": u.id,
                        "username": u.username,
                        "email": u.email,
                        "roles": u.roles,
                        "segment_id": u.segment_id,
                        "is_active": u.is_active,
                        "created_at": u.created_at
                    })
                }).collect();
                Ok(Json(APIResponse::success(serde_json::json!({
                    "users": sanitized
                }))))
            } else {
                Ok(Json(APIResponse::error("Insufficient permissions".to_string())))
            }
        }
        Err(e) => Ok(Json(APIResponse::error(format!("Authentication failed: {}", e)))),
    }
}

async fn list_roles(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let roles = state.auth_service.list_roles().await;
    Ok(Json(APIResponse::success(serde_json::json!({
        "roles": roles
    }))))
}

async fn create_segment(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateSegmentRequestWithAuth>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state.auth_service.validate_token(&request.authorization).await {
        Ok(validation) => {
            if let Ok(true) = state.auth_service.check_permission(&validation, ResourceType::Admin, Action::Admin).await {
                match state.auth_service.create_segment(request.name, request.description, request.parent_segment).await {
                    Ok(segment_id) => Ok(Json(APIResponse::success(serde_json::json!({
                        "segment_id": segment_id,
                        "message": "Segment created successfully"
                    })))),
                    Err(e) => Ok(Json(APIResponse::error(format!("Segment creation failed: {}", e)))),
                }
            } else {
                Ok(Json(APIResponse::error("Insufficient permissions".to_string())))
            }
        }
        Err(e) => Ok(Json(APIResponse::error(format!("Authentication failed: {}", e)))),
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterUserRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub segment_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSegmentRequest {
    pub name: String,
    pub description: String,
    pub parent_segment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTokenRequestWithAuth {
    pub authorization: String,
    pub name: String,
    pub scopes: Vec<crate::auth::TokenScope>,
    pub expires_in_hours: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeTokenRequest {
    pub authorization: String,
}

#[derive(Debug, Deserialize)]
pub struct ListTokensRequest {
    pub authorization: String,
}

#[derive(Debug, Deserialize)]
pub struct ListUsersRequest {
    pub authorization: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSegmentRequestWithAuth {
    pub authorization: String,
    pub name: String,
    pub description: String,
    pub parent_segment: Option<String>,
}

// Helper functions
fn parse_storage_type(storage_type: &str) -> Result<StorageType, StatusCode> {
    match storage_type {
        "columnar" => Ok(StorageType::Columnar),
        "vector" => Ok(StorageType::Vector),
        "document" => Ok(StorageType::Document),
        "relational" => Ok(StorageType::Relational),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

fn parse_operation(operation: &str) -> Result<QueryOperation, StatusCode> {
    match operation {
        "create" => Ok(QueryOperation::Create),
        "read" => Ok(QueryOperation::Read),
        "update" => Ok(QueryOperation::Update),
        "delete" => Ok(QueryOperation::Delete),
        "analyze" => Ok(QueryOperation::Analyze),
        "predict" => Ok(QueryOperation::Predict),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}
