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

```ignore
REST API Architecture

    HTTP Layer - Axum Web Framework
    ├── Async request handling
    ├── Type-safe routing
    └── Middleware pipeline

    API Controllers
    ├── CRUD operations
    ├── Advanced operations (AI/ML, clustering)
    ├── Transaction management
    └── Cluster management

    Business Logic Layer
    ├── Request validation
    ├── Data transformation
    ├── Authorization checks
    └── Response formatting

    Storage Engine Interface
    ├── All 5 storage engines
    ├── Transaction coordination
    └── Data routing
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

use crate::auth::{
    Action, AuthService, LoginRequest, ResourceType,
};
use crate::namespace;
use crate::query::{QueryLanguage, UqlQuery};
use crate::{PrimusDB, Query, QueryOperation, StorageType};
use axum::{
    extract::{Path, Query as AxumQuery, State},
    http::StatusCode,
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
    pub cluster_gateway: Option<Arc<crate::cluster::ClusterGateway>>,
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
    /// ```ignore
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
    /// ```ignore
    /// let response: APIResponse<String> = APIResponse::error("Something went wrong".to_string());
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

// ---- Cluster Gateway Handlers ----

#[derive(Debug, Deserialize)]
struct ClusterRouteRequest {
    shard_key: Option<String>,
    preferred_nodes: Option<Vec<String>>,
}

fn with_gateway<'a>(
    state: &'a AppState,
) -> std::result::Result<&'a crate::cluster::ClusterGateway, (StatusCode, &'static str)> {
    state.cluster_gateway.as_ref().map(|g| g.as_ref()).ok_or((StatusCode::SERVICE_UNAVAILABLE, "Cluster gateway not configured"))
}

async fn cluster_status_handler(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let gateway = with_gateway(&state)?;
    let metrics = gateway.get_metrics().await;
    let nodes = gateway.get_nodes().await;

    let status = serde_json::json!({
        "node_id": gateway.node_id,
        "strategy": format!("{:?}", metrics.strategy),
        "total_requests": metrics.total_requests,
        "routed_requests": metrics.routed_requests,
        "failed_requests": metrics.failed_requests,
        "circuit_breaks_triggered": metrics.circuit_breaks_triggered,
        "avg_latency_ms": metrics.avg_latency_ms,
        "p99_latency_ms": metrics.p99_latency_ms,
        "active_nodes": metrics.active_nodes,
        "healthy_nodes": metrics.healthy_nodes,
        "registered_nodes": nodes.len(),
    });

    Ok(Json(APIResponse::success(status)))
}

async fn cluster_nodes_handler(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<Json<APIResponse<Vec<crate::cluster::GatewayNode>>>, (StatusCode, &'static str)> {
    let gateway = with_gateway(&state)?;
    let nodes = gateway.get_nodes().await;
    Ok(Json(APIResponse::success(nodes)))
}

async fn cluster_route_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ClusterRouteRequest>,
) -> Json<APIResponse<Option<crate::cluster::RouteDecision>>> {
    let gateway = match with_gateway(&state) {
        Ok(g) => g,
        Err((_code, msg)) => {
            return Json(APIResponse {
                success: false,
                data: None,
                error: Some(msg.to_string()),
                timestamp: chrono::Utc::now(),
            });
        }
    };
    let preferred = request.preferred_nodes.as_ref().map(|v| v.as_slice());
    match gateway.get_route(request.shard_key.as_deref(), preferred).await {
        Ok(route) => Json(APIResponse::success(Some(route))),
        Err(e) => Json(APIResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            timestamp: chrono::Utc::now(),
        }),
    }
}

async fn cluster_metrics_handler(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<Json<APIResponse<crate::cluster::GatewayMetrics>>, (StatusCode, &'static str)> {
    let gateway = with_gateway(&state)?;
    let metrics = gateway.get_metrics().await;
    Ok(Json(APIResponse::success(metrics)))
}

#[derive(Debug, Deserialize)]
struct RegisterNodeRequest {
    node_id: String,
    host: String,
    port: u16,
    shards: Vec<String>,
}

async fn cluster_register_node_handler(
    State(state): State<Arc<AppState>>,
    Json(node): Json<RegisterNodeRequest>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let gateway = with_gateway(&state)?;
    gateway.register_node(&node.node_id, &node.host, node.port, node.shards).await;
    Ok(Json(APIResponse::success(serde_json::json!({"status": "registered"}))))
}

async fn cluster_remove_node_handler(
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let gateway = with_gateway(&state)?;
    gateway.remove_node(&node_id).await;
    Ok(Json(APIResponse::success(serde_json::json!({"status": "removed"}))))
}

// ---- Federation Handlers ----

async fn federation_status_handler(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let fed = state.primusdb.get_federation_manager()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Federation not configured"))?;
    let online = fed.get_cluster_count().await;
    let clusters = fed.get_online_clusters().await;
    let domains = fed.local_domains.read().await.clone();

    let status = serde_json::json!({
        "federation_id": fed.config.federation_id,
        "cluster_id": fed.config.cluster_id,
        "region": fed.config.region,
        "clusters_online": online,
        "clusters_total": fed.members.read().await.len(),
        "domains": domains,
        "members": clusters.iter().map(|c| serde_json::json!({
            "cluster_id": c.cluster_id,
            "address": c.address,
            "port": c.port,
            "alive_count": c.alive_count,
            "domains": c.domains,
            "region": c.region,
            "avg_latency_ms": c.avg_latency_ms,
        })).collect::<Vec<_>>(),
    });
    Ok(Json(APIResponse::success(status)))
}

async fn federation_clusters_handler(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<Json<APIResponse<Vec<crate::cluster::rpc::FedClusterInfo>>>, (StatusCode, &'static str)> {
    let fed = state.primusdb.get_federation_manager()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Federation not configured"))?;
    let clusters = fed.get_online_clusters().await;
    Ok(Json(APIResponse::success(clusters)))
}

async fn federation_domains_handler(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<Json<APIResponse<Vec<crate::cluster::DataDomain>>>, (StatusCode, &'static str)> {
    let primusdb = &state.primusdb;
    let dm = primusdb.get_domain_manager()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Domain manager not configured"))?;
    let domains = dm.list_domains().await;
    Ok(Json(APIResponse::success(domains)))
}

#[derive(serde::Deserialize)]
struct CreateDomainRequest {
    name: String,
    description: Option<String>,
    replication_mode: Option<String>,
    storage_types: Vec<String>,
    collections: Vec<String>,
    tables: Vec<String>,
    member_clusters: Vec<String>,
}

async fn federation_create_domain_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDomainRequest>,
) -> std::result::Result<Json<APIResponse<crate::cluster::DataDomain>>, (StatusCode, &'static str)> {
    let primusdb = &state.primusdb;
    let dm = primusdb.get_domain_manager()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Domain manager not configured"))?;
    let mode = crate::cluster::DomainReplicationMode::from_str(
        req.replication_mode.as_deref().unwrap_or("sync")
    );
    let domain = dm.create_domain(
        &req.name,
        req.description.as_deref().unwrap_or(""),
        mode,
        req.storage_types,
        req.collections,
        req.tables,
        req.member_clusters,
    ).await.map_err(|_e| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create domain"))?;
    Ok(Json(APIResponse::success(domain)))
}

async fn federation_balance_domain_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let primusdb = &state.primusdb;
    let dm = primusdb.get_domain_manager()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Domain manager not configured"))?;
    let plans = dm.check_balance().await;
    let domain_plans: Vec<_> = plans.into_iter().filter(|p| p.domain_name == name).collect();
    Ok(Json(APIResponse::success(serde_json::json!({
        "domain": name,
        "plans": domain_plans.iter().map(|p| serde_json::json!({
            "reason": p.reason,
            "moves": p.moves.iter().map(|m| serde_json::json!({
                "collection": m.collection,
                "from_cluster": m.from_cluster,
                "to_cluster": m.to_cluster,
                "estimated_cost": m.estimated_cost,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    }))))
}

#[derive(serde::Deserialize)]
struct DomainJoinRequest {
    collections: Option<Vec<String>>,
    storage_types: Option<Vec<String>>,
    replication_mode: Option<String>,
}

async fn federation_join_domain_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<DomainJoinRequest>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let fed = state.primusdb.get_federation_manager()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Federation not configured"))?;
    let ack = fed.join_domain(
        &name,
        req.collections.unwrap_or_default(),
        req.storage_types.unwrap_or_default(),
        req.replication_mode.as_deref().unwrap_or("sync"),
    ).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to join domain"))?;
    Ok(Json(APIResponse::success(serde_json::json!({
        "domain": name,
        "accepted": ack.accepted,
        "members": ack.members,
        "status": "joined"
    }))))
}

async fn federation_leave_domain_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let fed = state.primusdb.get_federation_manager()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Federation not configured"))?;
    fed.leave_domain(&name).await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to leave domain"))?;
    Ok(Json(APIResponse::success(serde_json::json!({
        "domain": name,
        "status": "left"
    }))))
}

async fn federation_metrics_handler(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let fed = state.primusdb.get_federation_manager()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Federation not configured"))?;
    let online = fed.get_cluster_count().await;
    let total = fed.members.read().await.len();
    let domains = fed.local_domains.read().await.len();

    let gw_metrics = if let Some(ref g) = state.cluster_gateway {
        Some(g.get_metrics().await)
    } else {
        None
    };

    let metrics = serde_json::json!({
        "federation": {
            "clusters_online": online,
            "clusters_total": total,
            "domains_count": domains,
            "healthy_ratio": if total > 0 { online as f64 / total as f64 } else { 0.0 },
        },
        "gateway": gw_metrics.map(|m| serde_json::json!({
            "total_requests": m.total_requests,
            "routed_requests": m.routed_requests,
            "failed_requests": m.failed_requests,
            "circuit_breaks": m.circuit_breaks_triggered,
            "avg_latency_ms": m.avg_latency_ms,
            "p99_latency_ms": m.p99_latency_ms,
        })),
    });
    Ok(Json(APIResponse::success(metrics)))
}

pub struct APIServer {
    app: Router,
}

impl APIServer {
    pub fn new(
        primusdb: Arc<PrimusDB>,
        auth_service: Arc<AuthService>,
        cluster_gateway: Option<Arc<crate::cluster::ClusterGateway>>,
    ) -> Self {
        let app = Router::new()
            // Root API endpoint
            .route("/api/v1", get(api_root))
            // Authentication endpoints (public)
            .route("/api/v1/auth/login", post(login))
            .route("/api/v1/auth/register", post(register_user))
            // Protected endpoints
            .route("/api/v1/auth/token/create", post(create_api_token))
            .route(
                "/api/v1/auth/token/revoke/:token_id",
                post(revoke_api_token),
            )
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
            .route("/api/v1/transaction/:id/execute", post(execute_transaction))
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
            // ER/DDL Operations (v1.2.2+)
            .route(
                "/api/v1/ddl/:storage_type/:table/column/add",
                post(ddl_add_column),
            )
            .route(
                "/api/v1/ddl/:storage_type/:table/column/:name",
                delete(ddl_drop_column),
            )
            .route(
                "/api/v1/ddl/:storage_type/:table/column",
                put(ddl_modify_column),
            )
            .route(
                "/api/v1/ddl/:storage_type/:table/constraint",
                post(ddl_add_constraint),
            )
            .route(
                "/api/v1/ddl/:storage_type/:table/constraint/:name",
                delete(ddl_drop_constraint),
            )
            .route(
                "/api/v1/ddl/:storage_type/:table/rename",
                post(ddl_rename_table),
            )
            // Sequence Operations
            .route("/api/v1/sequence/:storage_type", post(sequence_create))
            .route(
                "/api/v1/sequence/:storage_type/:name",
                delete(sequence_drop),
            )
            .route(
                "/api/v1/sequence/:storage_type/:name/nextval",
                post(sequence_nextval),
            )
            .route(
                "/api/v1/sequence/:storage_type/:name/currval",
                get(sequence_currval),
            )
            .route(
                "/api/v1/sequence/:storage_type/:name/setval",
                post(sequence_setval),
            )
            // View Operations
            .route("/api/v1/view/:storage_type", post(view_create))
            .route("/api/v1/view/:storage_type/:name", delete(view_drop))
            .route(
                "/api/v1/view/:storage_type/:name/refresh",
                post(view_refresh),
            )
            // Trigger Operations
            .route("/api/v1/trigger/:storage_type/:table", post(trigger_create))
            .route(
                "/api/v1/trigger/:storage_type/:table/:name",
                delete(trigger_drop),
            )
            // Information Schema
            .route(
                "/api/v1/info-schema/:storage_type/tables",
                get(info_schema_tables),
            )
            .route(
                "/api/v1/info-schema/:storage_type/:table/columns",
                get(info_schema_columns),
            )
            .route(
                "/api/v1/info-schema/:storage_type/:table/constraints",
                get(info_schema_constraints),
            )
            // Consensus & Blockchain Operations
            .route("/api/v1/consensus/state", get(consensus_state))
            .route("/api/v1/consensus/build-block", post(consensus_build_block))
            .route(
                "/api/v1/consensus/producer/start",
                post(consensus_start_producer),
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
            .route(
                "/api/v1/kv/:db/_ensure_full_commit",
                post(kv_ensure_full_commit),
            )
            .route("/api/v1/kv/:db/_rev_limit", get(kv_get_rev_limit))
            .route("/api/v1/kv/:db/_rev_limit", put(kv_set_rev_limit))
            .route("/api/v1/kv/:db/:docid", get(kv_get_document))
            .route("/api/v1/kv/:db/:docid", put(kv_put_document))
            .route("/api/v1/kv/:db/:docid", delete(kv_delete_document))
            .route("/api/v1/kv/:db/:docid", post(kv_update_document))
            // Namespace Operations
            .route("/api/v1/namespaces", get(list_namespaces))
            .route("/api/v1/namespaces/:path", post(create_namespace))
            .route("/api/v1/namespaces/:path", get(get_namespace))
            .route("/api/v1/namespaces/:path", put(update_namespace))
            .route("/api/v1/namespaces/:path", delete(delete_namespace))
            .route("/api/v1/namespaces/:path/children", get(list_namespace_children))
            .route("/api/v1/namespaces/:path/effective-policy", get(get_effective_policy))
            .route("/api/v1/namespaces/:path/resources", get(list_namespace_resources))
            .route("/api/v1/namespaces/:path/resources", post(attach_resource))
            .route("/api/v1/namespaces/:path/resources/:storage_type/:resource_name", delete(detach_resource))
            .route("/api/v1/namespaces/:path/roles", get(list_namespace_roles))
            .route("/api/v1/namespaces/:path/roles", post(create_namespace_role))
            .route("/api/v1/namespaces/:path/roles/:role_id", delete(delete_namespace_role))
            .route("/api/v1/namespaces/:path/users", get(list_namespace_user_bindings))
            .route("/api/v1/namespaces/:path/users", post(add_namespace_user_binding))
            .route("/api/v1/namespaces/:path/users/:user_id", delete(remove_namespace_user_binding))
            // Cluster Gateway Endpoints (v1.3.0-alpha)
            .route("/api/v1/cluster/status", get(cluster_status_handler))
            .route("/api/v1/cluster/nodes", get(cluster_nodes_handler))
            .route("/api/v1/cluster/route", post(cluster_route_handler))
            .route("/api/v1/cluster/metrics", get(cluster_metrics_handler))
            .route("/api/v1/cluster/node/register", post(cluster_register_node_handler))
            .route("/api/v1/cluster/node/:node_id", delete(cluster_remove_node_handler))
            // Federation Endpoints (v1.3.0-alpha)
            .route("/api/v1/federation/status", get(federation_status_handler))
            .route("/api/v1/federation/clusters", get(federation_clusters_handler))
            .route("/api/v1/federation/domains", get(federation_domains_handler).post(federation_create_domain_handler))
            .route("/api/v1/federation/domains/:name/join", post(federation_join_domain_handler))
            .route("/api/v1/federation/domains/:name/leave", post(federation_leave_domain_handler))
            .route("/api/v1/federation/domains/:name/balance", post(federation_balance_domain_handler))
            // Global Observability
            .route("/api/v1/federation/metrics", get(federation_metrics_handler))
            // Middleware
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .layer(CorsLayer::permissive())
            .with_state(Arc::new(AppState {
                primusdb,
                auth_service,
                cluster_gateway,
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
        println!("🔐 Authentication enabled",);

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
    let _primusdb = &state.primusdb;
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

    let namespace = request
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let query = Query {
        storage_type,
        operation,
        table: table.to_string(),
        conditions: request.get("conditions").cloned(),
        data: request.get("data").cloned(),
        limit: request.get("limit").and_then(|v| v.as_u64()),
        offset: request.get("offset").and_then(|v| v.as_u64()),
        namespace,
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

    let mut params_map: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
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

    match state.primusdb.uql_execute_query(&uql_query) {
        Ok(result) => {
            let value = serde_json::json!({
                "success": result.success,
                "records": result.records,
                "total": result.total,
                "execution_time_ms": result.execution_time_ms,
                "engine_used": result.engine_used,
                "warnings": result.warnings,
                "affected_rows": result.affected_rows
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
            "uql": "POST /api/v1/uql",
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
                "execute": "POST /api/v1/transaction/{id}/execute",
                "commit": "POST /api/v1/transaction/{id}/commit",
                "rollback": "POST /api/v1/transaction/{id}/rollback"
            },
            "table": {
                "info": "GET /api/v1/table/{storage_type}/{table}/info",
                "create": "POST /api/v1/table/{storage_type}/{table}/create",
                "drop": "DELETE /api/v1/table/{storage_type}/{table}/drop"
            },
            "ddl": {
                "add_column": "POST /api/v1/ddl/{storage_type}/{table}/column/add",
                "drop_column": "DELETE /api/v1/ddl/{storage_type}/{table}/column/{name}",
                "modify_column": "PUT /api/v1/ddl/{storage_type}/{table}/column",
                "add_constraint": "POST /api/v1/ddl/{storage_type}/{table}/constraint",
                "drop_constraint": "DELETE /api/v1/ddl/{storage_type}/{table}/constraint/{name}",
                "rename_table": "POST /api/v1/ddl/{storage_type}/{table}/rename"
            },
            "sequence": {
                "create": "POST /api/v1/sequence/{storage_type}",
                "drop": "DELETE /api/v1/sequence/{storage_type}/{name}",
                "nextval": "POST /api/v1/sequence/{storage_type}/{name}/nextval",
                "currval": "GET /api/v1/sequence/{storage_type}/{name}/currval",
                "setval": "POST /api/v1/sequence/{storage_type}/{name}/setval"
            },
            "view": {
                "create": "POST /api/v1/view/{storage_type}",
                "drop": "DELETE /api/v1/view/{storage_type}/{name}",
                "refresh": "POST /api/v1/view/{storage_type}/{name}/refresh"
            },
            "trigger": {
                "create": "POST /api/v1/trigger/{storage_type}/{table}",
                "drop": "DELETE /api/v1/trigger/{storage_type}/{table}/{name}"
            },
            "info_schema": {
                "tables": "GET /api/v1/info-schema/{storage_type}/tables",
                "columns": "GET /api/v1/info-schema/{storage_type}/{table}/columns",
                "constraints": "GET /api/v1/info-schema/{storage_type}/{table}/constraints"
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

async fn system_status(State(_state): State<Arc<AppState>>) -> Json<APIResponse<serde_json::Value>> {
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

async fn prometheus_metrics(State(_state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    let base = format!(
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

    let federation_metrics = crate::metrics::get_metrics().encode();
    Ok(base + "\n" + &federation_metrics)
}

async fn cluster_health(
    State(_state): State<Arc<AppState>>,
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

    let namespace = data
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let query = Query {
        storage_type,
        operation: QueryOperation::Create,
        table,
        conditions: None,
        data: Some(data),
        limit: None,
            offset: None,
            namespace,
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
    let namespace = params.get("namespace").cloned();

    let query = Query {
        storage_type,
        operation: QueryOperation::Read,
        table,
        conditions,
        data: None,
        limit,
        offset,
        namespace,
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

    let namespace = request
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let query = Query {
        storage_type,
        operation: QueryOperation::Update,
        table,
        conditions: request.get("conditions").cloned(),
        data: request.get("data").cloned(),
        limit: None,
            offset: None,
            namespace,
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
    let namespace = params.get("namespace").cloned();

    let query = Query {
        storage_type,
        operation: QueryOperation::Delete,
        table,
        conditions,
        data: None,
        limit: None,
            offset: None,
            namespace,
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
    body: Option<Json<serde_json::Value>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    let data = body.as_ref().map(|b| b.0.clone());
    let namespace = body
        .as_ref()
        .and_then(|b| b.0.get("namespace"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let query = Query {
        storage_type,
        operation: QueryOperation::Truncate,
        table,
        conditions: None,
        data,
        limit: None,
            offset: None,
            namespace,
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
    let namespace = request
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let query = Query {
        storage_type,
        operation: QueryOperation::Analyze,
        table,
        conditions: request.get("conditions").cloned(),
        data: None,
        limit: None,
            offset: None,
            namespace,
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
            namespace: None,
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
            namespace: None,
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
            namespace: None,
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
async fn begin_transaction(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let transaction = state.primusdb.transaction_manager.begin_transaction().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(APIResponse::success(serde_json::json!({
        "transaction_id": transaction.id,
        "status": "started",
        "isolation_level": "ReadCommitted",
        "timeout_ms": 30000,
    }))))
}

async fn commit_transaction(
    State(state): State<Arc<AppState>>,
    Path(transaction_id): Path<String>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let tx = crate::transaction::Transaction {
        id: transaction_id.clone(),
        operations: vec![],
        status: crate::transaction::TransactionStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
        timeout_ms: 0,
        ..Default::default()
    };
    state.primusdb.transaction_manager.commit_transaction(tx).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(APIResponse::success(serde_json::json!({
        "transaction_id": transaction_id,
        "status": "committed"
    }))))
}

async fn rollback_transaction(
    State(state): State<Arc<AppState>>,
    Path(transaction_id): Path<String>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    state.primusdb.transaction_manager.rollback_transaction(transaction_id.clone()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(APIResponse::success(serde_json::json!({
        "transaction_id": transaction_id,
        "status": "rolled_back"
    }))))
}

async fn execute_transaction(
    State(_state): State<Arc<AppState>>,
    Path(transaction_id): Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    use crate::transaction::{OperationType, TransactionOperation};
    let storage_type = request.get("storage_type").and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let operation = request.get("operation").and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let table = request.get("table").and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let data = request.get("data").cloned().unwrap_or(serde_json::Value::Null);
    let conditions = request.get("conditions").cloned();

    let op_type = match operation.to_lowercase().as_str() {
        "create" | "insert" => OperationType::Insert,
        "update" => OperationType::Update,
        "delete" => OperationType::Delete,
        "read" => OperationType::Read,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let op = TransactionOperation {
        id: format!("{}_{}", transaction_id, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
        operation_type: op_type,
        table: table.to_string(),
        data,
        conditions,
        before_image: None,
        after_image: None,
        executed: false,
        rollback_data: None,
        storage_type: storage_type.to_string(),
    };

    Ok(Json(APIResponse::success(serde_json::json!({
        "transaction_id": transaction_id,
        "operation_id": op.id,
        "status": "pending"
    }))))
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

    match state
        .primusdb
        .enable_collection_encryption(storage_type, &table)
    {
        Ok(_) => Ok(Json(APIResponse::success(serde_json::json!({
            "collection": table,
            "encryption": "enabled",
            "message": "Collection encryption enabled successfully"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Failed to enable encryption: {}",
            e
        )))),
    }
}

async fn decrypt_collection(
    State(state): State<Arc<AppState>>,
    Path(table): Path<String>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = StorageType::Document;

    match state
        .primusdb
        .disable_collection_encryption(storage_type, &table)
    {
        Ok(_) => Ok(Json(APIResponse::success(serde_json::json!({
            "collection": table,
            "encryption": "disabled",
            "message": "Collection encryption disabled successfully"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Failed to disable encryption: {}",
            e
        )))),
    }
}

// ==================== ER Model / DDL Operations ====================

async fn ddl_add_column(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = data
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let query = Query {
        storage_type,
        operation: QueryOperation::AlterTableAddColumn,
        table,
        data: Some(data),
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Add column failed: {}",
            e
        )))),
    }
}

async fn ddl_drop_column(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table, name)): Path<(String, String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = params.get("namespace").cloned();
    let query = Query {
        storage_type,
        operation: QueryOperation::AlterTableDropColumn,
        table,
        data: Some(serde_json::Value::String(name)),
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Drop column failed: {}",
            e
        )))),
    }
}

async fn ddl_modify_column(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = data
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let query = Query {
        storage_type,
        operation: QueryOperation::AlterTableModifyColumn,
        table,
        data: Some(data),
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Modify column failed: {}",
            e
        )))),
    }
}

async fn ddl_add_constraint(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = data
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let query = Query {
        storage_type,
        operation: QueryOperation::AlterTableAddConstraint,
        table,
        data: Some(data),
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Add constraint failed: {}",
            e
        )))),
    }
}

async fn ddl_drop_constraint(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table, name)): Path<(String, String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = params.get("namespace").cloned();
    let query = Query {
        storage_type,
        operation: QueryOperation::AlterTableDropConstraint,
        table,
        data: Some(serde_json::Value::String(name)),
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Drop constraint failed: {}",
            e
        )))),
    }
}

async fn ddl_rename_table(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let new_name = body
        .get("new_name")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let namespace = body
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let query = Query {
        storage_type,
        operation: QueryOperation::RenameTable,
        table,
        data: Some(serde_json::Value::String(new_name.to_string())),
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Rename table failed: {}",
            e
        )))),
    }
}

// Sequence Operations

async fn sequence_create(
    State(state): State<Arc<AppState>>,
    Path(storage_type): Path<String>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = data
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let query = Query {
        storage_type,
        operation: QueryOperation::CreateSequence,
        table: String::new(),
        data: Some(data),
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Create sequence failed: {}",
            e
        )))),
    }
}

async fn sequence_drop(
    State(state): State<Arc<AppState>>,
    Path((storage_type, name)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = params.get("namespace").cloned();
    let query = Query {
        storage_type,
        operation: QueryOperation::DropSequence,
        table: name,
        data: None,
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Drop sequence failed: {}",
            e
        )))),
    }
}

async fn sequence_nextval(
    State(state): State<Arc<AppState>>,
    Path((storage_type, name)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = params.get("namespace").cloned();
    let query = Query {
        storage_type,
        operation: QueryOperation::NextVal,
        table: name.clone(),
        data: None,
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(serde_json::json!({
            "sequence": name,
            "result": serde_json::to_value(result).unwrap_or_default()
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!("NextVal failed: {}", e)))),
    }
}

async fn sequence_currval(
    State(state): State<Arc<AppState>>,
    Path((storage_type, name)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = params.get("namespace").cloned();
    let query = Query {
        storage_type,
        operation: QueryOperation::CurrVal,
        table: name.clone(),
        data: None,
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(serde_json::json!({
            "sequence": name,
            "result": serde_json::to_value(result).unwrap_or_default()
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!("CurrVal failed: {}", e)))),
    }
}

async fn sequence_setval(
    State(state): State<Arc<AppState>>,
    Path((storage_type, name)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let value = body
        .get("value")
        .and_then(|v| v.as_i64())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let namespace = body
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let query = Query {
        storage_type,
        operation: QueryOperation::SetVal,
        table: name.clone(),
        data: Some(serde_json::json!(value)),
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(_) => Ok(Json(APIResponse::success(serde_json::json!({
            "sequence": name,
            "value": value,
            "status": "set"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!("SetVal failed: {}", e)))),
    }
}

// View Operations

async fn view_create(
    State(state): State<Arc<AppState>>,
    Path(storage_type): Path<String>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = data
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let query = Query {
        storage_type,
        operation: QueryOperation::CreateView,
        table: String::new(),
        data: Some(data),
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Create view failed: {}",
            e
        )))),
    }
}

async fn view_drop(
    State(state): State<Arc<AppState>>,
    Path((storage_type, name)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = params.get("namespace").cloned();
    let query = Query {
        storage_type,
        operation: QueryOperation::DropView,
        table: name,
        data: None,
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!("Drop view failed: {}", e)))),
    }
}

async fn view_refresh(
    State(state): State<Arc<AppState>>,
    Path((storage_type, name)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = params.get("namespace").cloned();
    let query = Query {
        storage_type,
        operation: QueryOperation::RefreshView,
        table: name,
        data: None,
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Refresh view failed: {}",
            e
        )))),
    }
}

// Trigger Operations

async fn trigger_create(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = data
        .get("namespace")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let query = Query {
        storage_type,
        operation: QueryOperation::CreateTrigger,
        table,
        data: Some(data),
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Create trigger failed: {}",
            e
        )))),
    }
}

async fn trigger_drop(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table, name)): Path<(String, String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = params.get("namespace").cloned();
    let query = Query {
        storage_type,
        operation: QueryOperation::DropTrigger,
        table,
        data: Some(serde_json::Value::String(name)),
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Drop trigger failed: {}",
            e
        )))),
    }
}

// Information Schema

async fn info_schema_tables(
    State(state): State<Arc<AppState>>,
    Path(storage_type): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = params.get("namespace").cloned();
    let query = Query {
        storage_type,
        operation: QueryOperation::InformationSchemaTables,
        table: String::new(),
        data: None,
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Info schema failed: {}",
            e
        )))),
    }
}

async fn info_schema_columns(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = params.get("namespace").cloned();
    let query = Query {
        storage_type,
        operation: QueryOperation::InformationSchemaColumns,
        table,
        data: None,
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Info schema failed: {}",
            e
        )))),
    }
}

async fn info_schema_constraints(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let namespace = params.get("namespace").cloned();
    let query = Query {
        storage_type,
        operation: QueryOperation::InformationSchemaConstraints,
        table,
        data: None,
        conditions: None,
        limit: None,
            offset: None,
            namespace,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => Ok(Json(APIResponse::success(
            serde_json::to_value(result).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Info schema failed: {}",
            e
        )))),
    }
}

// ==================== Key-Value (CouchDB-compatible) API ====================

// ── Key-Value Database Operations (CouchDB-compatible API) ──

fn kv_state_err() -> (StatusCode, &'static str) {
    (StatusCode::SERVICE_UNAVAILABLE, "Key-Value engine not available")
}

fn resolve_kv_ns(
    state: &AppState,
    namespace: Option<&str>,
    db: &str,
) -> std::result::Result<String, (StatusCode, &'static str)> {
    match namespace {
        Some(ns) if !ns.is_empty() && state.primusdb.config().namespaces.enabled => {
            match state
                .primusdb
                .get_namespace_controller()
                .resolve_physical_name(ns, StorageType::KeyValue, db)
            {
                Ok(name) => Ok(name),
                Err(_) => {
                    Err((StatusCode::BAD_REQUEST, "Namespace or resource not found"))
                }
            }
        }
        _ => Ok(db.to_string()),
    }
}

async fn kv_get_db_info(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    let info = engine.get_db_info(&db).map_err(|_| (StatusCode::NOT_FOUND, "Database not found"))?;
    Ok(Json(info))
}

async fn kv_create_db(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db).map_err(|_| StatusCode::BAD_REQUEST)?;
    let engine = state.primusdb.get_kv_engine().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    engine.create_database(&db).map_err(|_| StatusCode::CONFLICT)?;
    Ok(Json(APIResponse::success(serde_json::json!({"ok": true, "id": db}))))
}

async fn kv_delete_db(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db).map_err(|_| StatusCode::BAD_REQUEST)?;
    let engine = state.primusdb.get_kv_engine().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    engine.delete_database(&db).map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(APIResponse::success(serde_json::json!({"ok": true}))))
}

async fn kv_all_docs(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    let include_docs = params.get("include_docs").map(|v| v == "true").unwrap_or(false);
    let limit: Option<usize> = params.get("limit").and_then(|v| v.parse().ok());
    let skip: Option<usize> = params.get("skip").and_then(|v| v.parse().ok());
    let result = engine.all_docs(&db, include_docs, limit, skip)
        .map_err(|_| (StatusCode::NOT_FOUND, "Database not found"))?;
    Ok(Json(result))
}

async fn kv_find(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
    Json(body): Json<serde_json::Value>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    let request = crate::storage::keyvalue::KvFindRequest {
        selector: body.get("selector").cloned().unwrap_or(serde_json::Value::Null),
        limit: body.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize),
        skip: body.get("skip").and_then(|v| v.as_u64()).map(|v| v as usize),
        sort: body.get("sort").and_then(|v| v.as_array().cloned()),
    };
    let result = engine.find(&db, request)
        .map_err(|_| (StatusCode::NOT_FOUND, "Database not found"))?;
    Ok(Json(result))
}

async fn kv_list_indexes(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    let indexes = engine.list_indexes(&db)
        .map_err(|_| (StatusCode::NOT_FOUND, "Database not found"))?;
    let index_list: Vec<serde_json::Value> = indexes.into_iter().map(|idx| {
        serde_json::json!({
            "name": idx.name,
            "fields": idx.fields,
            "selector": idx.selector,
            "type": "json"
        })
    }).collect();
    Ok(Json(serde_json::json!({
        "total_rows": index_list.len(),
        "indexes": index_list
    })))
}

async fn kv_create_index(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
    Json(body): Json<serde_json::Value>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("default").to_string();
    let fields: Vec<String> = body.get("index")
        .and_then(|i| i.get("fields"))
        .and_then(|f| f.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let selector = body.get("index").and_then(|i| i.get("selector").cloned());
    let idx = engine.create_index(&db, &name, fields.clone(), selector)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Failed to create index"))?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "id": format!("_design/{}", idx.name),
        "name": idx.name,
        "fields": idx.fields
    })))
}

async fn kv_bulk_docs(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
    Json(body): Json<serde_json::Value>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    let docs_val = body.get("docs").cloned().unwrap_or(serde_json::Value::Null);
    let all_or_nothing = body.get("all_or_nothing").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut kv_docs = Vec::new();
    if let Some(arr) = docs_val.as_array() {
        for doc_val in arr {
            let kv_doc = crate::storage::keyvalue::KvDocument {
                _id: doc_val.get("_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                _rev: doc_val.get("_rev").and_then(|v| v.as_str()).map(String::from),
                value: doc_val.clone(),
                created_at: None,
                updated_at: None,
                deleted: doc_val.get("deleted").and_then(|v| v.as_bool()).unwrap_or(false),
            };
            kv_docs.push(kv_doc);
        }
    }

    let results = engine.bulk_docs(&db, kv_docs, all_or_nothing)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Bulk docs failed"))?;
    let out: Vec<serde_json::Value> = results.into_iter().map(|r| {
        serde_json::json!({
            "id": r.id,
            "rev": r.rev,
            "error": r.error,
        })
    }).collect();
    Ok(Json(serde_json::Value::Array(out)))
}

async fn kv_compact(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    let result = engine.compact(&db)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Compact failed"))?;
    Ok(Json(result))
}

async fn kv_ensure_full_commit(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    let result = engine.ensure_full_commit(&db)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Flush failed"))?;
    Ok(Json(result))
}

async fn kv_get_rev_limit(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    let limit = engine.get_revision_limit(&db)
        .map_err(|_| (StatusCode::NOT_FOUND, "Database not found"))?;
    Ok(Json(serde_json::json!({"rev_limit": limit})))
}

async fn kv_set_rev_limit(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
    Json(body): Json<serde_json::Value>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    let limit = body.get("rev_limit").and_then(|v| v.as_u64()).unwrap_or(1000);
    engine.set_revision_limit(&db, limit)
        .map_err(|_| (StatusCode::NOT_FOUND, "Database not found"))?;
    Ok(Json(serde_json::json!({"ok": true, "rev_limit": limit})))
}

async fn kv_get_document(
    State(state): State<Arc<AppState>>,
    Path((db, docid)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    match engine.get_document(&db, &docid) {
        Ok(doc) => {
            let mut resp = doc.value.clone();
            resp.as_object_mut().map(|obj| {
                obj.insert("_id".to_string(), serde_json::json!(doc._id));
                obj.insert("_rev".to_string(), serde_json::json!(doc._rev));
                obj.insert("created_at".to_string(), serde_json::json!(doc.created_at));
                obj.insert("updated_at".to_string(), serde_json::json!(doc.updated_at));
            });
            Ok(Json(resp))
        }
        Err(_) => Ok(Json(serde_json::json!({
            "_id": docid,
            "_rev": "0-0",
            "error": "not_found",
            "reason": "missing"
        }))),
    }
}

async fn kv_put_document(
    State(state): State<Arc<AppState>>,
    Path((db, docid)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
    Json(body): Json<serde_json::Value>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    match engine.put_document(&db, &docid, body) {
        Ok(doc) => Ok(Json(serde_json::json!({
            "ok": true,
            "id": doc._id,
            "rev": doc._rev
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "error": "conflict",
            "reason": e.to_string()
        }))),
    }
}

async fn kv_delete_document(
    State(state): State<Arc<AppState>>,
    Path((db, docid)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    let rev = params.get("rev").cloned().unwrap_or_default();
    match engine.delete_document(&db, &docid, &rev) {
        Ok(doc) => Ok(Json(serde_json::json!({
            "ok": true,
            "id": doc._id,
            "rev": doc._rev
        }))),
        Err(_) => Ok(Json(serde_json::json!({
            "error": "conflict",
            "reason": "Revision mismatch or document not found"
        }))),
    }
}

async fn kv_update_document(
    State(state): State<Arc<AppState>>,
    Path((db, docid)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
    Json(body): Json<serde_json::Value>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    // POST update is identical to PUT: upsert the document
    let db = resolve_kv_ns(&state, params.get("namespace").map(|s| s.as_str()), &db)?;
    let engine = state.primusdb.get_kv_engine().ok_or_else(kv_state_err)?;
    match engine.put_document(&db, &docid, body) {
        Ok(doc) => Ok(Json(serde_json::json!({
            "ok": true,
            "id": doc._id,
            "rev": doc._rev
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "error": "conflict",
            "reason": e.to_string()
        }))),
    }
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
    match state
        .auth_service
        .create_user(
            request.username,
            request.password,
            request.email,
            request.roles,
            request.segment_id,
        )
        .await
    {
        Ok(user_id) => Ok(Json(APIResponse::success(serde_json::json!({
            "user_id": user_id,
            "message": "User created successfully"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Registration failed: {}",
            e
        )))),
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

    match state
        .auth_service
        .validate_token(&request.authorization)
        .await
    {
        Ok(validation) => {
            match state
                .auth_service
                .create_token(&validation.user_id, token_request)
                .await
            {
                Ok((raw_token, token)) => Ok(Json(APIResponse::success(serde_json::json!({
                    "token": raw_token,
                    "token_id": token.id,
                    "expires_at": token.expires_at,
                    "message": "Store this token securely. It cannot be retrieved again."
                })))),
                Err(e) => Ok(Json(APIResponse::error(format!(
                    "Token creation failed: {}",
                    e
                )))),
            }
        }
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Authentication failed: {}",
            e
        )))),
    }
}

async fn revoke_api_token(
    State(state): State<Arc<AppState>>,
    Path(token_id): Path<String>,
    Json(request): Json<RevokeTokenRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state
        .auth_service
        .validate_token(&request.authorization)
        .await
    {
        Ok(_) => match state.auth_service.revoke_token(&token_id).await {
            Ok(()) => Ok(Json(APIResponse::success(serde_json::json!({
                "message": "Token revoked successfully"
            })))),
            Err(e) => Ok(Json(APIResponse::error(format!("Revoke failed: {}", e)))),
        },
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Authentication failed: {}",
            e
        )))),
    }
}

async fn list_tokens(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ListTokensRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state
        .auth_service
        .validate_token(&request.authorization)
        .await
    {
        Ok(validation) => {
            let tokens = state
                .auth_service
                .list_user_tokens(&validation.user_id)
                .await;
            Ok(Json(APIResponse::success(serde_json::json!({
                "tokens": tokens
            }))))
        }
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Authentication failed: {}",
            e
        )))),
    }
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ListUsersRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state
        .auth_service
        .validate_token(&request.authorization)
        .await
    {
        Ok(validation) => {
            if let Ok(true) = state
                .auth_service
                .check_permission(&validation, ResourceType::Admin, Action::Admin)
                .await
            {
                let users = state.auth_service.list_users().await;
                let sanitized: Vec<_> = users
                    .into_iter()
                    .map(|u| {
                        serde_json::json!({
                            "id": u.id,
                            "username": u.username,
                            "email": u.email,
                            "roles": u.roles,
                            "segment_id": u.segment_id,
                            "is_active": u.is_active,
                            "created_at": u.created_at
                        })
                    })
                    .collect();
                Ok(Json(APIResponse::success(serde_json::json!({
                    "users": sanitized
                }))))
            } else {
                Ok(Json(APIResponse::error(
                    "Insufficient permissions".to_string(),
                )))
            }
        }
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Authentication failed: {}",
            e
        )))),
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
    match state
        .auth_service
        .validate_token(&request.authorization)
        .await
    {
        Ok(validation) => {
            if let Ok(true) = state
                .auth_service
                .check_permission(&validation, ResourceType::Admin, Action::Admin)
                .await
            {
                match state
                    .auth_service
                    .create_segment(request.name, request.description, request.parent_segment)
                    .await
                {
                    Ok(segment_id) => Ok(Json(APIResponse::success(serde_json::json!({
                        "segment_id": segment_id,
                        "message": "Segment created successfully"
                    })))),
                    Err(e) => Ok(Json(APIResponse::error(format!(
                        "Segment creation failed: {}",
                        e
                    )))),
                }
            } else {
                Ok(Json(APIResponse::error(
                    "Insufficient permissions".to_string(),
                )))
            }
        }
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Authentication failed: {}",
            e
        )))),
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

// ── Consensus Handlers ─────────────────────────────────────────────

async fn consensus_state(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state.primusdb.get_chain_state().await {
        Ok(chain_state) => Ok(Json(APIResponse::success(
            serde_json::to_value(chain_state).unwrap_or_default(),
        ))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Failed to get chain state: {}",
            e
        )))),
    }
}

async fn consensus_build_block(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state.primusdb.build_and_commit_block().await {
        Ok(Some(block)) => Ok(Json(APIResponse::success(
            serde_json::json!({
                "hash": block.hash.as_str(),
                "height": block.height,
                "num_transactions": block.transactions.len(),
                "validator": block.validator,
            }),
        ))),
        Ok(None) => Ok(Json(APIResponse::success(serde_json::json!({
            "message": "No pending transactions in mempool"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Failed to build block: {}",
            e
        )))),
    }
}

async fn consensus_start_producer(
    State(state): State<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let interval_ms = params
        .get("interval_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(5000);
    state.primusdb.start_background_producer(interval_ms);
    Ok(Json(APIResponse::success(serde_json::json!({
        "message": format!("Background producer started with {}ms interval", interval_ms)
    }))))
}

// ── Namespace Request / Response types ───────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateNamespaceRequest {
    pub description: Option<String>,
    pub segment_id: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNamespaceRequest {
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub segment_id: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct AttachResourceRequest {
    pub storage_type: String,
    pub resource_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
    pub inheritable: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AddUserBindingRequest {
    pub user_id: String,
    pub role_id: String,
    pub granted_by: String,
    pub expires_at: Option<String>,
}

// ── Namespace Handlers ───────────────────────────────────────────────────────

async fn list_namespaces(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<Vec<namespace::Namespace>>>, StatusCode> {
    match state.primusdb.get_namespace_controller().list_all() {
        Ok(namespaces) => Ok(Json(APIResponse::success(namespaces))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn create_namespace(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Json(request): Json<CreateNamespaceRequest>,
) -> Result<Json<APIResponse<namespace::Namespace>>, StatusCode> {
    match state.primusdb.get_namespace_controller().create(
        &path,
        request.description.as_deref().unwrap_or(""),
        None,
        request.segment_id,
        request.metadata.unwrap_or_default(),
    ) {
        Ok(ns) => Ok(Json(APIResponse::success(ns))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn get_namespace(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<namespace::Namespace>>, StatusCode> {
    match state.primusdb.get_namespace_controller().get_by_path(&path) {
        Ok(Some(ns)) => Ok(Json(APIResponse::success(ns))),
        Ok(None) => Ok(Json(APIResponse::error(format!("Namespace '{}' not found", path)))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn update_namespace(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Json(request): Json<UpdateNamespaceRequest>,
) -> Result<Json<APIResponse<namespace::Namespace>>, StatusCode> {
    let update = namespace::NamespaceUpdate {
        description: request.description,
        policies: None,
        segment_id: request.segment_id,
        is_active: request.is_active,
        metadata: request.metadata,
    };
    match state.primusdb.get_namespace_controller().update(&path, update) {
        Ok(ns) => Ok(Json(APIResponse::success(ns))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn delete_namespace(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<()>>, StatusCode> {
    match state.primusdb.get_namespace_controller().delete(&path) {
        Ok(()) => Ok(Json(APIResponse::success(()))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn list_namespace_children(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<Vec<namespace::Namespace>>>, StatusCode> {
    match state.primusdb.get_namespace_controller().list_children(&path) {
        Ok(children) => Ok(Json(APIResponse::success(children))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn get_effective_policy(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<namespace::NamespacePolicies>>, StatusCode> {
    match state.primusdb.get_namespace_controller().effective_policy(&path) {
        Ok(policy) => Ok(Json(APIResponse::success(policy))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn list_namespace_resources(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<Vec<namespace::NamespaceResource>>>, StatusCode> {
    match state.primusdb.get_namespace_controller().get_by_path(&path) {
        Ok(Some(ns)) => match state.primusdb.get_namespace_controller().list_resources(&ns.id) {
            Ok(resources) => Ok(Json(APIResponse::success(resources))),
            Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
        },
        Ok(None) => Ok(Json(APIResponse::error(format!("Namespace '{}' not found", path)))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn attach_resource(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Json(request): Json<AttachResourceRequest>,
) -> Result<Json<APIResponse<namespace::NamespaceResource>>, StatusCode> {
    let st = match parse_storage_type(&request.storage_type) {
        Ok(t) => t,
        Err(e) => return Ok(Json(APIResponse::error(format!("Invalid storage type: {}", e)))),
    };
    match state.primusdb.get_namespace_controller().attach_resource(&path, st, &request.resource_name) {
        Ok(resource) => Ok(Json(APIResponse::success(resource))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn detach_resource(
    State(state): State<Arc<AppState>>,
    Path((path, storage_type, resource_name)): Path<(String, String, String)>,
) -> Result<Json<APIResponse<()>>, StatusCode> {
    let st = match parse_storage_type(&storage_type) {
        Ok(t) => t,
        Err(e) => return Ok(Json(APIResponse::error(format!("Invalid storage type: {}", e)))),
    };
    match state.primusdb.get_namespace_controller().detach_resource(&path, st, &resource_name) {
        Ok(()) => Ok(Json(APIResponse::success(()))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn list_namespace_roles(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<Vec<namespace::NamespaceRole>>>, StatusCode> {
    match state.primusdb.get_namespace_controller().get_by_path(&path) {
        Ok(Some(ns)) => match state.primusdb.get_namespace_controller().list_roles(&ns.id) {
            Ok(roles) => Ok(Json(APIResponse::success(roles))),
            Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
        },
        Ok(None) => Ok(Json(APIResponse::error(format!("Namespace '{}' not found", path)))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn create_namespace_role(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Json(request): Json<CreateRoleRequest>,
) -> Result<Json<APIResponse<namespace::NamespaceRole>>, StatusCode> {
    let permissions: Vec<namespace::NamespacePermission> = request
        .permissions
        .iter()
        .filter_map(|p| match p.to_lowercase().as_str() {
            "create" => Some(namespace::NamespacePermission::Create),
            "read" => Some(namespace::NamespacePermission::Read),
            "update" => Some(namespace::NamespacePermission::Update),
            "delete" => Some(namespace::NamespacePermission::Delete),
            "attach_resource" | "attachresource" => Some(namespace::NamespacePermission::AttachResource),
            "detach_resource" | "detachresource" => Some(namespace::NamespacePermission::DetachResource),
            "manage_users" | "manageusers" => Some(namespace::NamespacePermission::ManageUsers),
            "manage_roles" | "manageroles" => Some(namespace::NamespacePermission::ManageRoles),
            "manage_policies" | "managepolicies" => Some(namespace::NamespacePermission::ManagePolicies),
            "cross_namespace_read" | "crossnamespaceread" => Some(namespace::NamespacePermission::CrossNamespaceRead),
            "cross_namespace_write" | "crossnamespacewrite" => Some(namespace::NamespacePermission::CrossNamespaceWrite),
            "full_access" | "fullaccess" => Some(namespace::NamespacePermission::FullAccess),
            _ => None,
        })
        .collect();

    match state.primusdb.get_namespace_controller().add_role(
        &path,
        &request.name,
        &request.description,
        permissions,
        request.inheritable.unwrap_or(true),
    ) {
        Ok(role) => Ok(Json(APIResponse::success(role))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn delete_namespace_role(
    State(state): State<Arc<AppState>>,
    Path((path, role_id)): Path<(String, String)>,
) -> Result<Json<APIResponse<()>>, StatusCode> {
    match state.primusdb.get_namespace_controller().remove_role(&path, &role_id) {
        Ok(()) => Ok(Json(APIResponse::success(()))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn list_namespace_user_bindings(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<Vec<namespace::NamespaceUserBinding>>>, StatusCode> {
    match state.primusdb.get_namespace_controller().get_by_path(&path) {
        Ok(Some(ns)) => match state.primusdb.get_namespace_controller().list_user_bindings(&ns.id) {
            Ok(bindings) => Ok(Json(APIResponse::success(bindings))),
            Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
        },
        Ok(None) => Ok(Json(APIResponse::error(format!("Namespace '{}' not found", path)))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn add_namespace_user_binding(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Json(request): Json<AddUserBindingRequest>,
) -> Result<Json<APIResponse<namespace::NamespaceUserBinding>>, StatusCode> {
    let expires_at = request.expires_at.and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&chrono::Utc))
    });

    match state.primusdb.get_namespace_controller().add_user_binding(
        &path,
        &request.user_id,
        &request.role_id,
        &request.granted_by,
        expires_at,
    ) {
        Ok(binding) => Ok(Json(APIResponse::success(binding))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

async fn remove_namespace_user_binding(
    State(state): State<Arc<AppState>>,
    Path((path, user_id)): Path<(String, String)>,
) -> Result<Json<APIResponse<()>>, StatusCode> {
    match state.primusdb.get_namespace_controller().remove_user_binding(&path, &user_id) {
        Ok(()) => Ok(Json(APIResponse::success(()))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

// Helper functions
fn parse_storage_type(storage_type: &str) -> Result<StorageType, StatusCode> {
    match storage_type {
        "columnar" => Ok(StorageType::Columnar),
        "vector" => Ok(StorageType::Vector),
        "document" => Ok(StorageType::Document),
        "relational" => Ok(StorageType::Relational),
        "kv" | "keyvalue" => Ok(StorageType::KeyValue),
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
        "alter_add_column" => Ok(QueryOperation::AlterTableAddColumn),
        "alter_drop_column" => Ok(QueryOperation::AlterTableDropColumn),
        "alter_modify_column" => Ok(QueryOperation::AlterTableModifyColumn),
        "alter_add_constraint" => Ok(QueryOperation::AlterTableAddConstraint),
        "alter_drop_constraint" => Ok(QueryOperation::AlterTableDropConstraint),
        "rename_table" => Ok(QueryOperation::RenameTable),
        "create_sequence" => Ok(QueryOperation::CreateSequence),
        "drop_sequence" => Ok(QueryOperation::DropSequence),
        "nextval" => Ok(QueryOperation::NextVal),
        "currval" => Ok(QueryOperation::CurrVal),
        "setval" => Ok(QueryOperation::SetVal),
        "create_view" => Ok(QueryOperation::CreateView),
        "drop_view" => Ok(QueryOperation::DropView),
        "refresh_view" => Ok(QueryOperation::RefreshView),
        "create_trigger" => Ok(QueryOperation::CreateTrigger),
        "drop_trigger" => Ok(QueryOperation::DropTrigger),
        "info_schema_tables" => Ok(QueryOperation::InformationSchemaTables),
        "info_schema_columns" => Ok(QueryOperation::InformationSchemaColumns),
        "info_schema_constraints" => Ok(QueryOperation::InformationSchemaConstraints),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}
