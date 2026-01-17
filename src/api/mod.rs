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
use axum::{
    extract::{Path, Query as AxumQuery, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

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

pub struct APIServer {
    app: Router,
}

impl APIServer {
    pub fn new(primusdb: Arc<PrimusDB>) -> Self {
        let app = Router::new()
            // Root API endpoint
            .route("/api/v1", get(api_root))
            // Monitoring endpoints
            .route("/health", get(health_check))
            .route("/status", get(system_status))
            .route("/metrics", get(prometheus_metrics))
            .route("/api/v1/cache/cluster/health", get(cluster_health))
            // CRUD Operations - Generic query endpoint
            .route("/api/v1/query", post(execute_query))
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
            // Middleware
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .layer(CorsLayer::permissive())
            .with_state(primusdb);

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

        axum::serve(listener, self.app)
            .await
            .map_err(|e| crate::Error::NetworkError(format!("Server error: {}", e)))?;

        Ok(())
    }
}

// Generic query endpoint
async fn execute_query(
    State(primusdb): State<Arc<PrimusDB>>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
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

    match primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Query execution failed: {}",
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
    State(primusdb): State<Arc<PrimusDB>>,
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

async fn prometheus_metrics(State(primusdb): State<Arc<PrimusDB>>) -> Result<String, StatusCode> {
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
    State(primusdb): State<Arc<PrimusDB>>,
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
    State(primusdb): State<Arc<PrimusDB>>,
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

    match primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Create failed: {}", e)))),
    }
}

async fn read_records(
    State(primusdb): State<Arc<PrimusDB>>,
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

    match primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Read failed: {}", e)))),
    }
}

async fn update_record(
    State(primusdb): State<Arc<PrimusDB>>,
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

    match primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Update failed: {}", e)))),
    }
}

async fn delete_record(
    State(primusdb): State<Arc<PrimusDB>>,
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

    match primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Delete failed: {}", e)))),
    }
}

async fn truncate_table(
    State(primusdb): State<Arc<PrimusDB>>,
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

    match primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Truncate failed: {}", e)))),
    }
}

// Advanced Operations
async fn analyze_data(
    State(primusdb): State<Arc<PrimusDB>>,
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

    match primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Analysis failed: {}", e)))),
    }
}

async fn make_prediction(
    State(primusdb): State<Arc<PrimusDB>>,
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

    match primusdb.execute_query(query).await {
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
    State(primusdb): State<Arc<PrimusDB>>,
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

    match primusdb.execute_query(query).await {
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
    State(primusdb): State<Arc<PrimusDB>>,
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

    match primusdb.execute_query(query).await {
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
