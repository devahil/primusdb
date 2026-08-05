/*
 * PrimusDB REST API - Web Interface Layer
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: Collection encryption, Auth endpoints, Transactions
 */

/*!
# PrimusDB REST API - Web Interface Layer

This module implements the HTTP/WebSocket API for PrimusDB. An Axum server
exposes REST endpoints for every storage engine, the unified query engine,
auth/security, cluster and federation management, backups, time series,
config, governance and observability, plus a WebSocket/SSE real-time stream.
Requests are rate-limited globally and authenticated per-handler.

```text
Request Lifecycle
====================================================

HTTP / WS / SSE --> axum Router (APIServer::with_network_config)
      |
      v
Middleware (outermost -> innermost):
  CorsLayer -> CompressionLayer -> TraceLayer -> rate_limit_middleware
      |                                   (token bucket, 100 req / 60s default)
      v
Route dispatch --> handler --> AppState { primusdb, auth_service,
      |              |              cluster_gateway, rate_limiter, ws_state }
      |              +--> per-handler AuthService token + permission checks
      v
PrimusDB core: storage engines, UqlEngine, cluster gateway,
namespaces, governor, backups, time series, config store
      |
      v
APIResponse<T> JSON envelope (success/data/error/timestamp) + audit_log
```

## Route Groups

The full routing table (167 route declarations across ~30 functional areas) is
built in [`APIServer::with_network_config`]. The main groups are:

- **Auth** - `/api/v1/auth/login|register`, token create/revoke/list, users,
  roles, segments, MFA (`/api/v1/auth/mfa/setup|verify|disable`)
- **Health & Observability** - `/health`, `/status`, `/metrics`,
  `/protocol/health|status|peers|metrics`, `/api/v1/cache/cluster/health`
- **Query** - `POST /api/v1/query` (generic storage dispatch) and
  `POST /api/v1/uql` (unified query language)
- **CRUD** - `/api/v1/crud/:storage_type/:table` (POST/GET/PUT/DELETE) plus
  `/truncate`
- **Advanced / AI** - `/api/v1/advanced/analyze|predict|vector-search|cluster`
- **Transactions** - `/api/v1/transaction/begin|:id/commit|:id/rollback`
- **Tables & DDL** - `/api/v1/table/...`, `/api/v1/ddl/...`,
  `/api/v1/collection/:table/encrypt|decrypt`
- **Schema objects** - `/api/v1/sequence/...`, `/api/v1/view/...`,
  `/api/v1/trigger/...`, `/api/v1/info-schema/...`
- **Consensus** - `/api/v1/consensus/state|build-block|producer/start`
- **Key-Value (CouchDB-compatible)** - `/api/v1/kv/:db/...` (`_all_docs`,
  `_find`, `_index`, `_bulk_docs`, `_compact`, revisions, per-doc CRUD)
- **Namespaces & databases** - `/api/v1/namespaces/...`,
  `/api/v1/databases`
- **Cluster & federation** - `/api/v1/cluster/...`, `/api/v1/federation/...`
- **Engine lifecycle** - `/api/v1/engine/:engine_type/add|remove|upgrade`
- **Resource governor** - `/api/v1/governor/status|policies|metrics|violations|executions|policies/update`
- **Config management** - `/api/v1/config` (list/set/delete/validate/export/import)
  and `/api/v1/config/snapshots`
- **Table explorer** - `/api/v1/explorer/storage-types|tables|table/...`
- **RAG / Notebooks / Reports** - `/api/v1/rag/search`, `/api/v1/notebooks/...`,
  `/api/v1/reports/...`
- **System database** - `/api/v1/system/export|import`
- **Backups** - `/api/v1/backup/create/full|incremental`, `list`, `status`,
  `schedule`, `stop`, `restore/:backup_id`
- **Time series** - `/api/v1/timeseries/metrics`, `/metrics/:metric`,
  `/:metric/query|aggregate|downsample|retain|resolution`, `/stats`
- **Real-time streaming** - `GET /api/v1/ws` (WebSocket) and
  `GET /api/v1/sse` (Server-Sent Events)

## Standard API Response

Every endpoint returns the same JSON envelope:

```json
{
  "success": true,
  "data": { "result": "value" },
  "error": null,
  "timestamp": "2024-01-10T12:00:00Z"
}
```

## Authentication & Security

- Per-handler token validation via [`crate::auth::AuthService`]; there is no
  global auth middleware.
- Optional TLS termination via `axum_server` + `rustls` when
  [`crate::NetworkConfig::tls_enabled`] is set (mTLS supported).
- Global token-bucket rate limiting (see [`ratelimit`]) with
  `X-RateLimit-Limit` / `X-RateLimit-Remaining` response headers.

## Observability

- `GET /metrics` - Prometheus-style text metrics.
- `GET /health` / `GET /status` - liveness and detailed system status.
- TraceLayer request logging and audit logging for query/auth/DDL operations.
*/

pub mod ratelimit;
pub mod websocket;

use crate::auth::{Action, AuthService, LoginRequest, ResourceType};
use crate::namespace;
use crate::query::{QueryLanguage, UqlQuery};
use crate::storage::keyvalue::{KeyValueEngine, KvFindRequest};
use crate::{PrimusDB, Query, QueryOperation, StorageType};
use axum::{
    extract::{Path, Query as AxumQuery, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

/// Shared application state injected into all Axum HTTP handlers.
///
/// A single `Arc<AppState>` is constructed in [`APIServer::with_network_config`]
/// and wired into the router via `Router::with_state`. Handlers obtain it
/// through the `State<Arc<AppState>>` extractor.
///
/// # Fields
/// * `primusdb` - The [`PrimusDB`] core instance backing every data operation.
/// * `auth_service` - Authentication service used for login, token validation,
///   MFA and permission checks.
/// * `cluster_gateway` - Optional cluster gateway; when `None` the cluster and
///   federation handlers report the feature as disabled.
/// * `rate_limiter` - Token-bucket rate limiter shared with the
///   [`ratelimit::rate_limit_middleware`] middleware layer.
/// * `ws_state` - Broadcast channel used to push real-time events to WebSocket
///   and SSE subscribers.
pub struct AppState {
    /// Backing database engine that handles all query and storage operations.
    pub primusdb: Arc<PrimusDB>,
    /// Authentication service for login, tokens, MFA and permissions.
    pub auth_service: Arc<AuthService>,
    /// Optional cluster gateway for shard routing and cluster management.
    pub cluster_gateway: Option<Arc<crate::cluster::ClusterGateway>>,
    /// Token-bucket rate limiter applied to every request.
    pub rate_limiter: Arc<ratelimit::RateLimiter>,
    /// Broadcast channel for WebSocket/SSE real-time events.
    pub ws_state: Arc<websocket::WsState>,
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

/// Request body for REST-style CRUD create/read operations.
///
/// Accepted by the CRUD handlers when a structured body is provided; the
/// individual handlers mostly read raw `serde_json::Value` bodies instead, and
/// this type serves as the canonical shape for create/query payloads
/// (documented for API clients).
#[derive(Debug, Deserialize)]
pub struct CrudRequest {
    pub storage_type: String,
    pub table: String,
    pub data: Option<serde_json::Value>,
    pub conditions: Option<serde_json::Value>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Request body for CRUD update operations.
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub storage_type: String,
    pub table: String,
    pub data: serde_json::Value,
    pub conditions: Option<serde_json::Value>,
}

/// Request body for CRUD delete operations.
#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub storage_type: String,
    pub table: String,
    pub conditions: Option<serde_json::Value>,
}

/// Request body for data analysis operations.
#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub storage_type: String,
    pub table: String,
    pub conditions: Option<serde_json::Value>,
}

/// Request body for AI/ML prediction operations.
#[derive(Debug, Deserialize)]
pub struct PredictRequest {
    pub storage_type: String,
    pub table: String,
    pub data: serde_json::Value,
    pub prediction_type: Option<String>,
}

/// Request body for vector similarity search.
#[derive(Debug, Deserialize)]
pub struct VectorSearchRequest {
    pub table: String,
    pub query_vector: Vec<f32>,
    pub limit: Option<usize>,
}

/// Request body for data clustering operations.
#[derive(Debug, Deserialize)]
pub struct ClusterRequest {
    pub storage_type: String,
    pub table: String,
    pub algorithm: Option<String>,
    pub params: Option<serde_json::Value>,
}

/// Request body for the Unified Query Language (UQL) endpoint.
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

fn with_gateway(
    state: &AppState,
) -> std::result::Result<&crate::cluster::ClusterGateway, (StatusCode, &'static str)> {
    state.cluster_gateway.as_ref().map(|g| g.as_ref()).ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Cluster gateway not configured",
    ))
}

/// Report cluster gateway status, routing metrics and node counts.
async fn cluster_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    match with_gateway(&state) {
        Ok(gateway) => {
            let metrics = gateway.get_metrics().await;
            let nodes = gateway.get_nodes().await;
            Json(APIResponse::success(serde_json::json!({
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
                "known_nodes": nodes.len(),
            })))
        }
        Err(_) => Json(APIResponse::success(serde_json::json!({
            "status": "disabled",
            "message": "Cluster not configured",
        }))),
    }
}

/// List the nodes currently registered with the cluster gateway.
async fn cluster_nodes_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    match with_gateway(&state) {
        Ok(gateway) => {
            let nodes = gateway.get_nodes().await;
            Json(APIResponse::success(
                serde_json::to_value(nodes).unwrap_or_default(),
            ))
        }
        Err(_) => Json(APIResponse::success(serde_json::json!({
            "status": "disabled",
            "message": "Cluster not configured",
        }))),
    }
}

/// Compute a routing decision for a shard key via the cluster gateway.
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
    let preferred = request.preferred_nodes.as_deref();
    match gateway
        .get_route(request.shard_key.as_deref(), preferred)
        .await
    {
        Ok(route) => Json(APIResponse::success(Some(route))),
        Err(e) => Json(APIResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            timestamp: chrono::Utc::now(),
        }),
    }
}

/// Fetch raw cluster gateway metrics.
async fn cluster_metrics_handler(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<
    Json<APIResponse<crate::cluster::GatewayMetrics>>,
    (StatusCode, &'static str),
> {
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

/// Register a node with the cluster gateway.
async fn cluster_register_node_handler(
    State(state): State<Arc<AppState>>,
    Json(node): Json<RegisterNodeRequest>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let gateway = with_gateway(&state)?;
    gateway
        .register_node(&node.node_id, &node.host, node.port, node.shards)
        .await;
    Ok(Json(APIResponse::success(
        serde_json::json!({"status": "registered"}),
    )))
}

/// Remove a node from the cluster gateway.
async fn cluster_remove_node_handler(
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let gateway = with_gateway(&state)?;
    gateway.remove_node(&node_id).await;
    Ok(Json(APIResponse::success(
        serde_json::json!({"status": "removed"}),
    )))
}

/// Let the current node leave the cluster.
async fn cluster_leave_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    let node_id = match with_gateway(&state) {
        Ok(gateway) => gateway.node_id.clone(),
        Err(_) => {
            return Json(APIResponse::success(serde_json::json!({
                "status": "disabled",
                "message": "Cluster not configured",
            })));
        }
    };

    if let Ok(gw) = with_gateway(&state) {
        gw.remove_node(&node_id).await;
    }

    tracing::info!("Node {} left the cluster", node_id);

    Json(APIResponse::success(
        serde_json::json!({"status": "left", "node_id": node_id}),
    ))
}

// ---- Federation Handlers ----

/// Report federation status, member clusters and local domains.
async fn federation_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    match state.primusdb.get_federation_manager() {
        Some(fed) => {
            let online = fed.get_cluster_count().await;
            let clusters = fed.get_online_clusters().await;
            let domains = fed.local_domains.read().await.clone();
            Json(APIResponse::success(serde_json::json!({
                "status": "enabled",
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
            })))
        }
        None => Json(APIResponse::success(serde_json::json!({
            "status": "disabled",
            "message": "Federation not configured",
        }))),
    }
}

/// List the clusters currently online in the federation.
async fn federation_clusters_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    match state.primusdb.get_federation_manager() {
        Some(fed) => {
            let clusters = fed.get_online_clusters().await;
            Json(APIResponse::success(serde_json::json!({
                "status": "enabled",
                "clusters": serde_json::to_value(clusters).unwrap_or_default(),
            })))
        }
        None => Json(APIResponse::success(serde_json::json!({
            "status": "disabled",
            "message": "Federation not configured",
        }))),
    }
}

/// List the local domains managed by the domain manager.
async fn federation_domains_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    match state.primusdb.get_domain_manager() {
        Some(dm) => {
            let domains = dm.list_domains().await;
            Json(APIResponse::success(serde_json::json!({
                "status": "enabled",
                "domains": serde_json::to_value(domains).unwrap_or_default(),
            })))
        }
        None => Json(APIResponse::success(serde_json::json!({
            "status": "disabled",
            "message": "Domain manager not configured",
        }))),
    }
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

/// Create a new federated data domain.
async fn federation_create_domain_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDomainRequest>,
) -> std::result::Result<Json<APIResponse<crate::cluster::DataDomain>>, (StatusCode, &'static str)>
{
    let primusdb = &state.primusdb;
    let dm = primusdb.get_domain_manager().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Domain manager not configured",
    ))?;
    let mode = crate::cluster::DomainReplicationMode::from_str(
        req.replication_mode.as_deref().unwrap_or("sync"),
    );
    let domain = dm
        .create_domain(
            &req.name,
            req.description.as_deref().unwrap_or(""),
            mode,
            req.storage_types,
            req.collections,
            req.tables,
            req.member_clusters,
        )
        .await
        .map_err(|_e| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create domain"))?;
    Ok(Json(APIResponse::success(domain)))
}

/// Produce rebalancing plans for a named domain.
async fn federation_balance_domain_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let primusdb = &state.primusdb;
    let dm = primusdb.get_domain_manager().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Domain manager not configured",
    ))?;
    let plans = dm.check_balance().await;
    let domain_plans: Vec<_> = plans
        .into_iter()
        .filter(|p| p.domain_name == name)
        .collect();
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

/// Join this cluster to a federated domain.
async fn federation_join_domain_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<DomainJoinRequest>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let fed = state
        .primusdb
        .get_federation_manager()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Federation not configured"))?;
    let ack = fed
        .join_domain(
            &name,
            req.collections.unwrap_or_default(),
            req.storage_types.unwrap_or_default(),
            req.replication_mode.as_deref().unwrap_or("sync"),
        )
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to join domain"))?;
    Ok(Json(APIResponse::success(serde_json::json!({
        "domain": name,
        "accepted": ack.accepted,
        "members": ack.members,
        "status": "joined"
    }))))
}

/// Leave this cluster from a federated domain.
async fn federation_leave_domain_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let fed = state
        .primusdb
        .get_federation_manager()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Federation not configured"))?;
    fed.leave_domain(&name)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to leave domain"))?;
    Ok(Json(APIResponse::success(serde_json::json!({
        "domain": name,
        "status": "left"
    }))))
}

/// Report federation and gateway metrics.
async fn federation_metrics_handler(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<Json<APIResponse<serde_json::Value>>, (StatusCode, &'static str)> {
    let fed = state
        .primusdb
        .get_federation_manager()
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

// ── Engine Lifecycle Endpoints (v1.3.1-alpha) ────────────────

/// Request body for engine lifecycle operations.
#[derive(serde::Deserialize)]
struct EngineLifecycleRequest {
    /// Whether to drop engine data on disk (remove only).
    #[serde(default)]
    drop_data: bool,
}

/// Schedule a new storage engine type to be added (applies on restart).
async fn engine_add_handler(
    State(state): State<Arc<AppState>>,
    Path(engine_type): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    let valid = [
        "columnar",
        "vector",
        "document",
        "relational",
        "keyvalue",
        "timeseries",
    ];
    if !valid.contains(&engine_type.as_str()) {
        return Json(APIResponse::error(format!(
            "Unknown engine type '{}'. Valid types: {}",
            engine_type,
            valid.join(", ")
        )));
    }

    match state.primusdb.schedule_engine_add(&engine_type) {
        Ok(pending) => Json(APIResponse::success(serde_json::json!({
            "engine_type": engine_type,
            "message": format!("Engine '{}' add scheduled. Restart server to apply.", engine_type),
            "pending_operations": pending.len(),
            "restart_required": true,
        }))),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to schedule engine add: {}",
            e
        ))),
    }
}

/// Schedule a storage engine type to be removed (optionally dropping its data).
async fn engine_remove_handler(
    State(state): State<Arc<AppState>>,
    Path(engine_type): Path<String>,
    Json(body): Json<EngineLifecycleRequest>,
) -> Json<APIResponse<serde_json::Value>> {
    let valid = [
        "columnar",
        "vector",
        "document",
        "relational",
        "keyvalue",
        "timeseries",
    ];
    if !valid.contains(&engine_type.as_str()) {
        return Json(APIResponse::error(format!(
            "Unknown engine type '{}'. Valid types: {}",
            engine_type,
            valid.join(", ")
        )));
    }

    match state
        .primusdb
        .schedule_engine_remove(&engine_type, body.drop_data)
    {
        Ok(pending) => Json(APIResponse::success(serde_json::json!({
            "engine_type": engine_type,
            "drop_data": body.drop_data,
            "message": format!("Engine '{}' removal scheduled. Restart server to apply.", engine_type),
            "pending_operations": pending.len(),
            "restart_required": true,
        }))),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to schedule engine removal: {}",
            e
        ))),
    }
}

/// Schedule a storage engine upgrade (applies on restart).
async fn engine_upgrade_handler(
    State(state): State<Arc<AppState>>,
    Path(engine_type): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    let valid = [
        "columnar",
        "vector",
        "document",
        "relational",
        "keyvalue",
        "timeseries",
    ];
    if !valid.contains(&engine_type.as_str()) {
        return Json(APIResponse::error(format!(
            "Unknown engine type '{}'. Valid types: {}",
            engine_type,
            valid.join(", ")
        )));
    }

    match state.primusdb.schedule_engine_upgrade(&engine_type) {
        Ok(pending) => Json(APIResponse::success(serde_json::json!({
            "engine_type": engine_type,
            "message": format!("Engine '{}' upgrade scheduled. Restart server to apply.", engine_type),
            "pending_operations": pending.len(),
            "restart_required": true,
        }))),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to schedule engine upgrade: {}",
            e
        ))),
    }
}

/// HTTP API server built on Axum, routing all REST and WebSocket endpoints.
///
/// `APIServer` owns the fully assembled [`Router`], the middleware stack
/// (CORS, compression, tracing and rate limiting) and the network configuration
/// used at bind time. Construct it with [`APIServer::new`] or
/// [`APIServer::with_network_config`], then call [`APIServer::run`] to serve.
///
/// The complete route table served by this server is documented in the
/// [module documentation](crate::api) (see the "Routing Table" section).
pub struct APIServer {
    app: Router,
    network_config: crate::NetworkConfig,
}

impl APIServer {
    /// Construct an [`APIServer`] with default network configuration.
    ///
    /// This is shorthand for [`APIServer::with_network_config`] using
    /// [`crate::NetworkConfig::default`].
    ///
    /// # Arguments
    /// * `primusdb` - Shared [`PrimusDB`] core instance backing every handler.
    /// * `auth_service` - Authentication service used for login, token
    ///   validation, MFA and permission checks.
    /// * `cluster_gateway` - Optional cluster gateway; pass `None` to have the
    ///   cluster and federation endpoints report themselves as disabled.
    pub fn new(
        primusdb: Arc<PrimusDB>,
        auth_service: Arc<AuthService>,
        cluster_gateway: Option<Arc<crate::cluster::ClusterGateway>>,
    ) -> Self {
        Self::with_network_config(
            primusdb,
            auth_service,
            cluster_gateway,
            crate::NetworkConfig::default(),
        )
    }

    /// Build the fully assembled Axum [`Router`] and wrap it with the given
    /// network configuration.
    ///
    /// This constructor registers every route documented in the
    /// [module documentation](crate::api) (see the "Routing Table" section),
    /// creates the shared [`ratelimit::RateLimiter`] and
    /// [`websocket::WsState`], applies the middleware stack (rate limiter,
    /// tracing, compression, CORS) and injects a single [`Arc<AppState>`] into
    /// all handlers.
    ///
    /// # Arguments
    /// * `primusdb` - Shared [`PrimusDB`] core instance.
    /// * `auth_service` - Authentication service for token validation and
    ///   permission checks.
    /// * `cluster_gateway` - Optional [`crate::cluster::ClusterGateway`].
    /// * `network_config` - Network settings (TLS, timeouts) used at bind time.
    ///
    /// # Returns
    /// A configured `APIServer` ready for [`APIServer::run`].
    pub fn with_network_config(
        primusdb: Arc<PrimusDB>,
        auth_service: Arc<AuthService>,
        cluster_gateway: Option<Arc<crate::cluster::ClusterGateway>>,
        network_config: crate::NetworkConfig,
    ) -> Self {
        let rate_limiter = Arc::new(ratelimit::RateLimiter::new(
            ratelimit::RateLimitConfig::default(),
        ));
        let ws_state = Arc::new(websocket::WsState::new(256));

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
            // MFA endpoints
            .route("/api/v1/auth/mfa/setup", post(mfa_setup))
            .route("/api/v1/auth/mfa/verify", post(mfa_verify))
            .route("/api/v1/auth/mfa/disable", post(mfa_disable))
            // Monitoring endpoints
            .route("/health", get(health_check))
            .route("/status", get(system_status))
            .route("/metrics", get(prometheus_metrics))
            .route("/protocol/health", get(protocol_health_handler))
            .route("/protocol/status", get(protocol_status_handler))
            .route("/protocol/peers", get(protocol_peers_handler))
            .route("/protocol/metrics", get(protocol_metrics_handler))
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
            .route(
                "/api/v1/namespaces/:path/children",
                get(list_namespace_children),
            )
            .route(
                "/api/v1/namespaces/:path/effective-policy",
                get(get_effective_policy),
            )
            .route(
                "/api/v1/namespaces/:path/resources",
                get(list_namespace_resources),
            )
            .route("/api/v1/namespaces/:path/resources", post(attach_resource))
            .route(
                "/api/v1/namespaces/:path/resources/:storage_type/:resource_name",
                delete(detach_resource),
            )
            .route("/api/v1/namespaces/:path/roles", get(list_namespace_roles))
            .route(
                "/api/v1/namespaces/:path/roles",
                post(create_namespace_role),
            )
            .route(
                "/api/v1/namespaces/:path/roles/:role_id",
                delete(delete_namespace_role),
            )
            .route(
                "/api/v1/namespaces/:path/users",
                get(list_namespace_user_bindings),
            )
            .route(
                "/api/v1/namespaces/:path/users",
                post(add_namespace_user_binding),
            )
            .route(
                "/api/v1/namespaces/:path/users/:user_id",
                delete(remove_namespace_user_binding),
            )
            // Database Management Endpoints (v1.3.2-alpha)
            .route("/api/v1/databases", get(list_databases))
            .route("/api/v1/databases", post(create_database_handler))
            // Integrity Endpoints (v1.3.2-alpha engine-integrity-graphql)
            .route("/api/v1/integrity/status", get(integrity_status_handler))
            .route(
                "/api/v1/databases/:db/integrity/genesis",
                get(integrity_genesis_handler),
            )
            .route(
                "/api/v1/databases/:db/integrity/records",
                get(integrity_records_handler),
            )
            .route(
                "/api/v1/databases/:db/integrity/verify",
                get(integrity_verify_handler),
            )
            .route(
                "/api/v1/databases/:db/integrity/checkpoints",
                get(integrity_checkpoints_handler),
            )
            .route(
                "/api/v1/databases/:db/integrity/checkpoints",
                post(integrity_checkpoint_create_handler),
            )
            .route("/api/v1/integrity/pending", get(integrity_pending_handler))
            .route(
                "/api/v1/integrity/pending",
                post(integrity_pending_flush_handler),
            )
            .route(
                "/api/v1/integrity/quarantine",
                get(integrity_quarantine_handler),
            )
            .route(
                "/api/v1/integrity/quarantine/:db/:sequence",
                delete(integrity_quarantine_release_handler),
            )
            .route(
                "/api/v1/databases/:db/integrity/reconcile/evidence",
                get(integrity_reconcile_evidence_handler),
            )
            .route(
                "/api/v1/databases/:db/integrity/reconcile",
                post(integrity_reconcile_handler),
            )
            .route("/api/v1/ledger/status", get(ledger_status_handler))
            // Unified Search (v1.3.2-alpha engine-integrity-graphql)
            .route("/api/v1/search", get(search_handler))
            // Capability negotiation (drivers, REPL, discovery)
            .route("/api/v1/capabilities", get(capabilities_handler))
            // GraphQL Service (v1.3.2-alpha engine-integrity-graphql)
            .route(
                "/api/v1/graphql",
                get(graphql_schema_handler).post(graphql_handler),
            )
            // Cluster Gateway Endpoints (v1.3.1-alpha)
            .route("/api/v1/cluster/status", get(cluster_status_handler))
            .route("/api/v1/cluster/nodes", get(cluster_nodes_handler))
            .route("/api/v1/cluster/route", post(cluster_route_handler))
            .route("/api/v1/cluster/metrics", get(cluster_metrics_handler))
            .route(
                "/api/v1/cluster/node/register",
                post(cluster_register_node_handler),
            )
            .route(
                "/api/v1/cluster/node/:node_id",
                delete(cluster_remove_node_handler),
            )
            .route("/api/v1/cluster/leave", post(cluster_leave_handler))
            // Federation Endpoints (v1.3.1-alpha)
            .route("/api/v1/federation/status", get(federation_status_handler))
            .route(
                "/api/v1/federation/clusters",
                get(federation_clusters_handler),
            )
            .route(
                "/api/v1/federation/domains",
                get(federation_domains_handler).post(federation_create_domain_handler),
            )
            .route(
                "/api/v1/federation/domains/:name/join",
                post(federation_join_domain_handler),
            )
            .route(
                "/api/v1/federation/domains/:name/leave",
                post(federation_leave_domain_handler),
            )
            .route(
                "/api/v1/federation/domains/:name/balance",
                post(federation_balance_domain_handler),
            )
            // Global Observability
            .route(
                "/api/v1/federation/metrics",
                get(federation_metrics_handler),
            )
            // Engine Lifecycle (v1.3.1-alpha)
            .route("/api/v1/engine/:engine_type/add", post(engine_add_handler))
            .route(
                "/api/v1/engine/:engine_type/remove",
                post(engine_remove_handler),
            )
            .route(
                "/api/v1/engine/:engine_type/upgrade",
                post(engine_upgrade_handler),
            )
            // Resource Governor Endpoints (v1.3.1-alpha)
            .route("/api/v1/governor/status", get(governor_status_handler))
            .route("/api/v1/governor/policies", get(governor_policies_handler))
            .route("/api/v1/governor/metrics", get(governor_metrics_handler))
            .route(
                "/api/v1/governor/violations",
                get(governor_violations_handler),
            )
            .route(
                "/api/v1/governor/executions",
                get(governor_executions_handler),
            )
            .route(
                "/api/v1/governor/executions/start",
                post(governor_start_execution_handler),
            )
            .route(
                "/api/v1/governor/executions/:id/finish",
                post(governor_finish_execution_handler),
            )
            .route(
                "/api/v1/governor/executions/:id/check",
                post(governor_check_limit_handler),
            )
            .route(
                "/api/v1/governor/policies/update",
                post(governor_update_policy_handler),
            )
            // Config Management Endpoints (v1.3.2-alpha)
            .route("/api/v1/config", get(list_config_entries))
            .route("/api/v1/config", post(set_config_entry))
            .route("/api/v1/config", delete(delete_config_entry_handler))
            .route("/api/v1/config/validate", post(validate_config_entry))
            .route("/api/v1/config/export", get(export_config_bundle))
            .route("/api/v1/config/import", post(import_config_bundle))
            .route("/api/v1/config/snapshots", get(list_config_snapshots))
            .route("/api/v1/config/snapshots", post(create_config_snapshot))
            .route(
                "/api/v1/config/snapshots/:id/restore",
                post(restore_config_snapshot),
            )
            .route(
                "/api/v1/config/snapshots/:id",
                delete(delete_config_snapshot),
            )
            // Table Explorer Endpoints (v1.3.2-alpha)
            .route(
                "/api/v1/explorer/storage-types",
                get(explorer_storage_types),
            )
            .route("/api/v1/explorer/tables", get(explorer_tables))
            .route(
                "/api/v1/explorer/table/:storage_type/:table",
                get(explorer_table_info),
            )
            .route(
                "/api/v1/explorer/table/:storage_type/:table/rows",
                post(explorer_table_rows),
            )
            // RAG Workspace Endpoints (v1.3.2-alpha)
            .route("/api/v1/rag/search", post(rag_search_handler))
            // Notebook Endpoints (v1.3.2-alpha)
            .route("/api/v1/notebooks", get(list_notebooks))
            .route("/api/v1/notebooks", post(create_notebook_handler))
            .route("/api/v1/notebooks/:id", get(get_notebook))
            .route("/api/v1/notebooks/:id", put(update_notebook_handler))
            .route("/api/v1/notebooks/:id", delete(delete_notebook_handler))
            .route(
                "/api/v1/notebooks/:id/execute",
                post(execute_notebook_cell_handler),
            )
            // Report Builder Endpoints (v1.3.2-alpha)
            .route("/api/v1/reports", get(list_reports))
            .route("/api/v1/reports", post(create_report))
            .route("/api/v1/reports/:id", get(get_report))
            .route("/api/v1/reports/:id", put(update_report))
            .route("/api/v1/reports/:id", delete(delete_report))
            .route("/api/v1/reports/:id/execute", post(execute_report_handler))
            // System Database Endpoints (v1.3.2-alpha)
            .route("/api/v1/system/export", get(system_db_export_handler))
            .route("/api/v1/system/import", post(system_db_import_handler))
            // Backup Management Endpoints (v1.3.2-alpha)
            .route(
                "/api/v1/backup/create/full",
                post(backup_create_full_handler),
            )
            .route(
                "/api/v1/backup/create/incremental",
                post(backup_create_incremental_handler),
            )
            .route("/api/v1/backup/list", get(backup_list_handler))
            .route("/api/v1/backup/status", get(backup_status_handler))
            .route("/api/v1/backup/schedule", post(backup_schedule_handler))
            .route("/api/v1/backup/stop", post(backup_stop_scheduler_handler))
            .route(
                "/api/v1/backup/restore/:backup_id",
                post(backup_restore_handler),
            )
            // Time Series Endpoints (v1.3.2-alpha)
            .route("/api/v1/timeseries/metrics", get(ts_list_metrics))
            .route(
                "/api/v1/timeseries/metrics/:metric",
                get(ts_describe_metric),
            )
            .route("/api/v1/timeseries/:metric/query", post(ts_query))
            .route("/api/v1/timeseries/:metric/aggregate", post(ts_aggregate))
            .route("/api/v1/timeseries/:metric/downsample", post(ts_downsample))
            .route("/api/v1/timeseries/:metric/retain", post(ts_retain))
            .route("/api/v1/timeseries/:metric/resolution", post(ts_resolution))
            .route("/api/v1/timeseries/stats", get(ts_stats))
            // Real-time Subscription Endpoints
            .route("/api/v1/ws", get(websocket::ws_handler))
            .route("/api/v1/sse", get(websocket::sse_handler))
            // Middleware
            .layer(axum::middleware::from_fn_with_state(
                rate_limiter.clone(),
                ratelimit::rate_limit_middleware,
            ))
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .layer(CorsLayer::permissive())
            .with_state(Arc::new(AppState {
                primusdb,
                auth_service,
                cluster_gateway,
                rate_limiter,
                ws_state,
            }));

        APIServer {
            app,
            network_config,
        }
    }

    /// Bind to `bind_addr` and serve HTTP (or HTTPS when TLS is enabled).
    ///
    /// When [`crate::NetworkConfig::tls_enabled`] is set, a [`RustlsConfig`] is
    /// loaded from the configured certificate and key paths and the server is
    /// served over TLS with [`axum_server`]; otherwise a plain HTTP server is
    /// bound on a [`tokio::net::TcpListener`].
    ///
    /// # Errors
    /// Returns [`crate::Error`] if the TLS configuration cannot be loaded, the
    /// listener cannot be bound to `bind_addr`, or the server fails while
    /// serving.
    pub async fn run(self, bind_addr: &str) -> std::result::Result<(), crate::Error> {
        if self.network_config.tls_enabled {
            let tls_config = RustlsConfig::from_pem_file(
                &self.network_config.tls_cert_path,
                &self.network_config.tls_key_path,
            )
            .await
            .map_err(|e| {
                crate::Error::NetworkError(format!(
                    "Failed to load TLS config (cert={}, key={}): {}",
                    self.network_config.tls_cert_path, self.network_config.tls_key_path, e
                ))
            })?;

            println!("🚀 PrimusDB API server listening on: https://{}", bind_addr);
            println!("📡 API root: https://{}/api/v1", bind_addr);
            println!("🔐 Authentication enabled (TLS)");

            axum_server::bind_rustls(
                bind_addr.parse::<std::net::SocketAddr>().unwrap(),
                tls_config,
            )
            .serve(self.app.into_make_service())
            .await
            .map_err(|e| crate::Error::NetworkError(format!("Server error: {}", e)))?;
        } else {
            let listener = tokio::net::TcpListener::bind(bind_addr)
                .await
                .map_err(|e| {
                    crate::Error::NetworkError(format!("Failed to bind to {}: {}", bind_addr, e))
                })?;

            println!("🚀 PrimusDB API server listening on: http://{}", bind_addr);
            println!("📡 API root: http://{}/api/v1", bind_addr);
            println!("🔐 Authentication enabled");

            axum::serve(listener, self.app)
                .await
                .map_err(|e| crate::Error::NetworkError(format!("Server error: {}", e)))?;
        }

        Ok(())
    }
}

/// Execute a generic query against any storage engine.
///
/// Reads `storage_type`, `operation`, `table`, `conditions`, `data`, `limit`,
/// `offset` and an optional `namespace` from the JSON body and dispatches a
/// [`Query`] through [`PrimusDB::execute_query`].
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
    let op_str = format!("{:?}", operation);
    let st_str = format!("{:?}", storage_type);

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
        Ok(result) => {
            state.primusdb.audit_log(
                "query.execute",
                "api_user",
                "query",
                "execute",
                serde_json::json!({"storage_type": st_str, "operation": op_str, "table": table}),
                true,
            );
            Ok(Json(APIResponse::success(
                serde_json::to_value(result).unwrap_or_default(),
            )))
        }
        Err(e) => {
            state.primusdb.audit_log(
                "query.execute",
                "api_user",
                "query",
                "execute",
                serde_json::json!({"storage_type": st_str, "operation": op_str, "table": table, "error": e.to_string()}),
                false,
            );
            Ok(Json(APIResponse::error(format!(
                "Query execution failed: {}",
                e
            ))))
        }
    }
}

/// Execute a UQL (Unified Query Language) query across all storage engines.
///
/// Translates the `language` hint (`sql`, `mongodb`, `mango`, `uql`, or auto)
/// and `params` into a [`UqlQuery`] executed via
/// [`PrimusDB::uql_execute_query`].
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
            state.primusdb.audit_log(
                "query.uql",
                "api_user",
                "query",
                "execute",
                serde_json::json!({"language": language, "success": result.success, "engine_used": result.engine_used}),
                true,
            );
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
        Err(e) => {
            state.primusdb.audit_log(
                "query.uql",
                "api_user",
                "query",
                "execute",
                serde_json::json!({"language": language, "error": e.to_string()}),
                false,
            );
            Ok(Json(APIResponse::error(format!(
                "UQL query execution failed: {}",
                e
            ))))
        }
    }
}

/// Serve the API root metadata document describing available endpoints.
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

/// Service health check reporting node, instance, version and uptime.
async fn health_check(State(state): State<Arc<AppState>>) -> Json<APIResponse<serde_json::Value>> {
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Json(APIResponse::success(serde_json::json!({
        "status": "healthy",
        "node_id": state.primusdb.node_id(),
        "instance_id": state.primusdb.instance_id(),
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": uptime,
        "architecture": "centralized"
    })))
}

/// Report detailed system status including per-storage-engine availability.
async fn system_status(State(state): State<Arc<AppState>>) -> Json<APIResponse<serde_json::Value>> {
    use crate::StorageType;

    let engines = [
        (StorageType::Columnar, "columnar"),
        (StorageType::Vector, "vector"),
        (StorageType::Document, "document"),
        (StorageType::Relational, "relational"),
        (StorageType::KeyValue, "keyvalue"),
    ];

    let mut storage_engines = serde_json::Map::new();
    for (st, name) in &engines {
        let available = state.primusdb.storage_engine(*st).is_some();
        storage_engines.insert(
            name.to_string(),
            serde_json::Value::String(if available {
                "available".to_string()
            } else {
                "unavailable".to_string()
            }),
        );
    }

    let status = serde_json::json!({
        "status": "running",
        "uptime_seconds": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "version": env!("CARGO_PKG_VERSION"),
        "storage_engines": storage_engines,
        "ai_enabled": true,
        "cache_enabled": true,
        "transactions_enabled": true
    });

    Json(APIResponse::success(status))
}

/// Expose Prometheus-formatted metrics (uptime, memory, per-engine availability).
async fn prometheus_metrics(State(state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    use crate::StorageType;

    // Uptime from /proc/self/stat (start time in jiffies, compared to system boot)
    let stat =
        std::fs::read_to_string("/proc/stat").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let uptime = stat
        .lines()
        .find(|l| l.starts_with("btime"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse::<u64>().ok())
        .map(|boot_time| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            now.saturating_sub(boot_time)
        })
        .unwrap_or(0);

    // Memory usage from /proc/self/status (Linux)
    let mem_str = std::fs::read_to_string("/proc/self/status")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rss_kb = mem_str
        .lines()
        .find(|l| l.starts_with("VmRSS:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let memory_bytes = rss_kb * 1024;

    // Count active engines and list their names
    let engine_types = [
        StorageType::Columnar,
        StorageType::Vector,
        StorageType::Document,
        StorageType::Relational,
        StorageType::KeyValue,
    ];
    let mut active_engine_count = 0u64;
    let mut engine_lines = String::new();
    for st in &engine_types {
        let available = state.primusdb.storage_engine(*st).is_some() as u64;
        active_engine_count += available;
        engine_lines.push_str(&format!(
            "primusdb_engine{{type=\"{}\"}} {}\n",
            st, available
        ));
    }

    let metrics = format!(
        r#"# HELP primusdb_up PrimusDB service availability
# TYPE primusdb_up gauge
primusdb_up 1

# HELP primusdb_version PrimusDB version
# TYPE primusdb_version gauge
primusdb_version{{version="{}"}} 1

# HELP primusdb_uptime_seconds Service uptime in seconds
# TYPE primusdb_uptime_seconds counter
primusdb_uptime_seconds {uptime}

# HELP primusdb_memory_usage_bytes Current memory usage
# TYPE primusdb_memory_usage_bytes gauge
primusdb_memory_usage_bytes {memory_bytes}

# HELP primusdb_engines_total Number of storage engines loaded
# TYPE primusdb_engines_total gauge
primusdb_engines_total {active_engine_count}

# HELP primusdb_engine Storage engine availability (1 = available, 0 = unavailable)
# TYPE primusdb_engine gauge
{engine_lines}"#,
        env!("CARGO_PKG_VERSION"),
    );

    Ok(metrics)
}

/// Report cached cluster health status.
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

// ── Protocol Health Endpoints ─────────────────────────────────────

/// Protocol health probe used by peer discovery.
async fn protocol_health_handler(
    State(_state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    Json(APIResponse::success(
        serde_json::json!({"status": "healthy", "protocol": "primusdb"}),
    ))
}

/// Protocol status probe reporting the protocol version.
async fn protocol_status_handler(
    State(_state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    Json(APIResponse::success(
        serde_json::json!({"status": "running", "version": "1.3.1-alpha"}),
    ))
}

/// List known protocol peers.
async fn protocol_peers_handler(
    State(_state): State<Arc<AppState>>,
) -> Json<APIResponse<Vec<serde_json::Value>>> {
    Json(APIResponse::success(Vec::new()))
}

/// Expose protocol metrics in Prometheus text format.
async fn protocol_metrics_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<String, StatusCode> {
    Ok("# HELP primusdb_protocol_peers Number of protocol peers\n# TYPE primusdb_protocol_peers gauge\nprimusdb_protocol_peers 0\n".to_string())
}

/// Create a record in the target storage engine and table.
///
/// Accepts an optional `namespace` field in the body; broadcasts a
/// `record.created` event on success.
async fn create_record(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let table_name = table.clone();

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
        Ok(result) => {
            state.ws_state.broadcast(websocket::WsMessage {
                event_type: "record.created".to_string(),
                data: serde_json::json!({
                    "storage_type": format!("{:?}", storage_type),
                    "table": table_name,
                }),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
            Ok(Json(APIResponse::success(
                serde_json::to_value(result).unwrap_or_default(),
            )))
        }
        Err(e) => Ok(Json(APIResponse::error(format!("Create failed: {}", e)))),
    }
}

/// Read records from the target storage engine and table with filtering.
///
/// Query parameters: `conditions` (JSON string), `limit`, `offset`,
/// `namespace`.
async fn read_records(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    let conditions = match params.get("conditions") {
        Some(c) => Some(serde_json::from_str(c).map_err(|_| StatusCode::BAD_REQUEST)?),
        None => None,
    };
    let limit: Option<u64> = match params.get("limit") {
        Some(l) => Some(l.parse().map_err(|_| StatusCode::BAD_REQUEST)?),
        None => None,
    };
    let offset: Option<u64> = match params.get("offset") {
        Some(o) => Some(o.parse().map_err(|_| StatusCode::BAD_REQUEST)?),
        None => None,
    };
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

/// Update records matching the given conditions in the target table.
///
/// Accepts `conditions` and `data` in the JSON body and broadcasts a
/// `record.updated` event on success.
async fn update_record(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let table_name = table.clone();

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
        Ok(result) => {
            state.ws_state.broadcast(websocket::WsMessage {
                event_type: "record.updated".to_string(),
                data: serde_json::json!({
                    "storage_type": format!("{:?}", storage_type),
                    "table": table_name,
                }),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
            Ok(Json(APIResponse::success(
                serde_json::to_value(result).unwrap_or_default(),
            )))
        }
        Err(e) => Ok(Json(APIResponse::error(format!("Update failed: {}", e)))),
    }
}

/// Delete records matching the given conditions from the target table.
///
/// Query parameters: `conditions` (JSON string), `namespace`. Broadcasts a
/// `record.deleted` event on success.
async fn delete_record(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let table_name = table.clone();

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
        Ok(result) => {
            state.ws_state.broadcast(websocket::WsMessage {
                event_type: "record.deleted".to_string(),
                data: serde_json::json!({
                    "storage_type": format!("{:?}", storage_type),
                    "table": table_name,
                }),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
            Ok(Json(APIResponse::success(
                serde_json::to_value(result).unwrap_or_default(),
            )))
        }
        Err(e) => Ok(Json(APIResponse::error(format!("Delete failed: {}", e)))),
    }
}

/// Truncate (empty) the target table.
///
/// Accepts an optional `namespace` field in the body and broadcasts a
/// `table.truncated` event on success.
async fn truncate_table(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    body: Option<Json<serde_json::Value>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;
    let table_name = table.clone();

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
        Ok(result) => {
            state.ws_state.broadcast(websocket::WsMessage {
                event_type: "table.truncated".to_string(),
                data: serde_json::json!({
                    "storage_type": format!("{:?}", storage_type),
                    "table": table_name,
                }),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
            Ok(Json(APIResponse::success(
                serde_json::to_value(result).unwrap_or_default(),
            )))
        }
        Err(e) => Ok(Json(APIResponse::error(format!("Truncate failed: {}", e)))),
    }
}

/// Run a data analysis operation against the target table.
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

/// Make an AI/ML prediction against the target table.
///
/// `prediction_type` (from the body) is passed as a condition; `data` carries
/// the input payload.
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

/// Run a vector similarity search against the vector engine.
///
/// Body fields: `query_vector` (array of floats), optional `limit` (default
/// 10).
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

/// Cluster data in the target table using an analyze operation.
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

/// Begin a new transaction and return its transaction ID.
async fn begin_transaction(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state.primusdb.begin_transaction().await {
        Ok(tx_id) => Ok(Json(APIResponse::success(serde_json::json!({
            "transaction_id": tx_id,
            "status": "started"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Failed to start transaction: {}",
            e
        )))),
    }
}

/// Commit a previously started transaction.
async fn commit_transaction(
    State(state): State<Arc<AppState>>,
    Path(transaction_id): Path<String>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state
        .primusdb
        .commit_transaction(transaction_id.clone())
        .await
    {
        Ok(_) => Ok(Json(APIResponse::success(serde_json::json!({
            "transaction_id": transaction_id,
            "status": "committed"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Failed to commit transaction: {}",
            e
        )))),
    }
}

/// Roll back a previously started transaction.
async fn rollback_transaction(
    State(state): State<Arc<AppState>>,
    Path(transaction_id): Path<String>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state
        .primusdb
        .rollback_transaction(transaction_id.clone())
        .await
    {
        Ok(_) => Ok(Json(APIResponse::success(serde_json::json!({
            "transaction_id": transaction_id,
            "status": "rolled_back"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Failed to rollback transaction: {}",
            e
        )))),
    }
}

/// Return metadata (row count, size, schema) for a table.
async fn table_info(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    match state.primusdb.table_info(storage_type, &table).await {
        Ok(info) => Ok(Json(APIResponse::success(serde_json::json!({
            "name": info.name,
            "row_count": info.row_count,
            "size_bytes": info.size_bytes,
            "created_at": info.created_at.to_rfc3339(),
            "updated_at": info.updated_at.to_rfc3339(),
            "schema": info.schema,
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Failed to get table info: {}",
            e
        )))),
    }
}

/// Create a new table in the given storage engine (writes an audit log).
async fn create_table(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    match state.primusdb.create_table(storage_type, &table).await {
        Ok(_) => {
            state.primusdb.audit_log(
                "ddl.create_table",
                "api_user",
                &format!("table:{}", table),
                "create",
                serde_json::json!({"storage_type": format!("{:?}", storage_type).to_lowercase(), "table": table}),
                true,
            );
            Ok(Json(APIResponse::success(serde_json::json!({
                "storage_type": format!("{:?}", storage_type).to_lowercase(),
                "table": table,
                "status": "created"
            }))))
        }
        Err(e) => {
            state.primusdb.audit_log(
                "ddl.create_table",
                "api_user",
                &format!("table:{}", table),
                "create",
                serde_json::json!({"error": e.to_string()}),
                false,
            );
            Ok(Json(APIResponse::error(format!(
                "Failed to create table: {}",
                e
            ))))
        }
    }
}

/// Drop a table from the given storage engine (writes an audit log).
async fn drop_table(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let storage_type = parse_storage_type(&storage_type)?;

    match state.primusdb.drop_table(storage_type, &table).await {
        Ok(_) => {
            state.primusdb.audit_log(
                "ddl.drop_table",
                "api_user",
                &format!("table:{}", table),
                "drop",
                serde_json::json!({"storage_type": format!("{:?}", storage_type).to_lowercase(), "table": table}),
                true,
            );
            Ok(Json(APIResponse::success(serde_json::json!({
                "storage_type": format!("{:?}", storage_type).to_lowercase(),
                "table": table,
                "status": "dropped"
            }))))
        }
        Err(e) => {
            state.primusdb.audit_log(
                "ddl.drop_table",
                "api_user",
                &format!("table:{}", table),
                "drop",
                serde_json::json!({"error": e.to_string()}),
                false,
            );
            Ok(Json(APIResponse::error(format!(
                "Failed to drop table: {}",
                e
            ))))
        }
    }
}

/// Enable encryption for a document collection.
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

/// Disable encryption for a document collection.
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

/// Add a column to a table (ALTER TABLE).
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

/// Drop a column from a table (ALTER TABLE).
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

/// Modify the definition of a column (ALTER TABLE).
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

/// Add a constraint to a table (ALTER TABLE).
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

/// Drop a constraint from a table (ALTER TABLE).
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

/// Rename a table; `new_name` is required in the body.
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

/// Create a sequence in the given storage engine.
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

/// Drop a sequence.
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

/// Advance a sequence and return its next value.
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

/// Return the current value of a sequence without advancing it.
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

/// Set a sequence's current value (`value` is required in the body).
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

/// Create a view in the given storage engine.
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

/// Drop a view.
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

/// Refresh (recompute) a materialized view.
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

/// Create a trigger on the target table.
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

/// Drop a trigger by name from the target table.
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

/// List tables visible in the information schema of a storage engine.
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

/// List columns of a table from the information schema.
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

/// List constraints of a table from the information schema.
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

/// Return metadata for a key-value database.
async fn kv_get_db_info(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Json<serde_json::Value> {
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.get_db_info(&db))
    }) {
        Some(Ok(info)) => Json(info),
        Some(Err(e)) => Json(serde_json::json!({"error": e.to_string()})),
        None => Json(serde_json::json!({"error": "Key-Value engine not available"})),
    }
}

/// Create a key-value database.
async fn kv_create_db(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.create_database(&db))
    }) {
        Some(Ok(_)) => Json(APIResponse::success(
            serde_json::json!({"ok": true, "id": db}),
        )),
        Some(Err(e)) => Json(APIResponse::error(e.to_string())),
        None => Json(APIResponse::error(
            "Key-Value engine not available".to_string(),
        )),
    }
}

/// Delete a key-value database.
async fn kv_delete_db(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.delete_database(&db))
    }) {
        Some(Ok(_)) => Json(APIResponse::success(serde_json::json!({"ok": true}))),
        Some(Err(e)) => Json(APIResponse::error(e.to_string())),
        None => Json(APIResponse::error(
            "Key-Value engine not available".to_string(),
        )),
    }
}

/// List all document IDs in a database (CouchDB `_all_docs`).
///
/// Query parameters: `include_docs`, `limit`, `skip`.
async fn kv_all_docs(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let include_docs = params
        .get("include_docs")
        .map(|v| v == "true")
        .unwrap_or(false);
    let limit: Option<usize> = match params.get("limit") {
        Some(v) => Some(v.parse().map_err(|_| StatusCode::BAD_REQUEST)?),
        None => None,
    };
    let skip: Option<usize> = match params.get("skip") {
        Some(v) => Some(v.parse().map_err(|_| StatusCode::BAD_REQUEST)?),
        None => None,
    };

    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.all_docs(&db, include_docs, limit, skip))
    }) {
        Some(Ok(result)) => Ok(Json(result)),
        Some(Err(e)) => Ok(Json(
            serde_json::json!({"error": e.to_string(), "rows": []}),
        )),
        None => Ok(Json(
            serde_json::json!({"error": "Key-Value engine not available", "rows": []}),
        )),
    }
}

/// Run a Mango-style find query against a database (CouchDB `_find`).
async fn kv_find(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    Json(selector): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let limit = selector
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let skip = selector
        .get("skip")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let sort = selector.get("sort").and_then(|v| v.as_array()).cloned();
    let selector_obj = selector
        .get("selector")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let request = KvFindRequest {
        selector: selector_obj,
        limit,
        skip,
        sort,
    };

    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.find(&db, request))
    }) {
        Some(Ok(result)) => Json(result),
        Some(Err(e)) => Json(serde_json::json!({"error": e.to_string(), "docs": []})),
        None => Json(serde_json::json!({"error": "Key-Value engine not available", "docs": []})),
    }
}

/// List the secondary indexes defined on a database.
async fn kv_list_indexes(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Json<serde_json::Value> {
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.list_indexes(&db))
    }) {
        Some(Ok(indexes)) => {
            let json_indexes: Vec<serde_json::Value> = indexes
                .iter()
                .map(|idx| {
                    serde_json::json!({
                        "name": idx.name,
                        "fields": idx.fields,
                    })
                })
                .collect();
            Json(serde_json::json!({"indexes": json_indexes}))
        }
        Some(Err(e)) => Json(serde_json::json!({"error": e.to_string(), "indexes": []})),
        None => Json(serde_json::json!({"error": "Key-Value engine not available", "indexes": []})),
    }
}

/// Create a secondary index on a database (CouchDB `_index`).
async fn kv_create_index(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    Json(index_def): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = index_def
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("default");
    let fields = index_def
        .get("index")
        .and_then(|i| i.get("fields"))
        .and_then(|f| f.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.create_index(&db, name, fields.clone(), None))
    }) {
        Some(Ok(_)) => Json(serde_json::json!({
            "ok": true,
            "id": format!("_design/{}", name),
            "name": name,
            "fields": fields
        })),
        Some(Err(e)) => Json(serde_json::json!({"error": e.to_string()})),
        None => Json(serde_json::json!({"error": "Key-Value engine not available"})),
    }
}

/// Insert or update multiple documents atomically or individually (CouchDB
/// `_bulk_docs`).
async fn kv_bulk_docs(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    use crate::storage::keyvalue::KvDocument;

    let docs: Vec<KvDocument> = request
        .get("docs")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|d| {
                    serde_json::from_value(d.clone()).unwrap_or(KvDocument {
                        _id: d
                            .get("_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        _rev: d.get("_rev").and_then(|v| v.as_str()).map(String::from),
                        value: d.clone(),
                        created_at: None,
                        updated_at: None,
                        deleted: false,
                        expires_at: d.get("expires_at").and_then(|v| v.as_i64()),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let all_or_nothing = request
        .get("all_or_nothing")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.bulk_docs(&db, docs, all_or_nothing))
    }) {
        Some(Ok(results)) => {
            let json_results: Vec<serde_json::Value> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "rev": r.rev,
                        "error": r.error
                    })
                })
                .collect();
            Json(serde_json::Value::Array(json_results))
        }
        Some(Err(e)) => Json(serde_json::json!([{"error": e.to_string()}])),
        None => Json(serde_json::json!([{"error": "Key-Value engine not available"}])),
    }
}

/// Compact a database to reclaim storage (CouchDB `_compact`).
async fn kv_compact(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Json<serde_json::Value> {
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.compact(&db))
    }) {
        Some(Ok(result)) => Json(result),
        Some(Err(e)) => Json(serde_json::json!({"error": e.to_string()})),
        None => Json(serde_json::json!({"error": "Key-Value engine not available"})),
    }
}

/// Flush all writes to disk (CouchDB `_ensure_full_commit`).
async fn kv_ensure_full_commit(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Json<serde_json::Value> {
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.ensure_full_commit(&db))
    }) {
        Some(Ok(result)) => Json(result),
        Some(Err(e)) => Json(serde_json::json!({"error": e.to_string()})),
        None => Json(serde_json::json!({"error": "Key-Value engine not available"})),
    }
}

/// Read the revision limit of a database (CouchDB `_rev_limit`).
async fn kv_get_rev_limit(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Json<serde_json::Value> {
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.get_revision_limit(&db))
    }) {
        Some(Ok(limit)) => Json(serde_json::json!({"rev_limit": limit})),
        Some(Err(e)) => Json(serde_json::json!({"error": e.to_string(), "rev_limit": 0})),
        None => {
            Json(serde_json::json!({"error": "Key-Value engine not available", "rev_limit": 0}))
        }
    }
}

/// Set the revision limit of a database (CouchDB `_rev_limit`).
async fn kv_set_rev_limit(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    Json(limit): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let rev_limit = limit
        .get("rev_limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(1000);
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.set_revision_limit(&db, rev_limit))
    }) {
        Some(Ok(_)) => Json(serde_json::json!({"ok": true, "rev_limit": rev_limit})),
        Some(Err(e)) => Json(serde_json::json!({"error": e.to_string()})),
        None => Json(serde_json::json!({"error": "Key-Value engine not available"})),
    }
}

/// Fetch a single document by ID.
async fn kv_get_document(
    State(state): State<Arc<AppState>>,
    Path((db, docid)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.get_document(&db, &docid))
    }) {
        Some(Ok(doc)) => {
            let mut result = doc.value.clone();
            result["_id"] = serde_json::json!(doc._id);
            if let Some(rev) = &doc._rev {
                result["_rev"] = serde_json::json!(rev);
            }
            Json(result)
        }
        Some(Err(e)) => Json(serde_json::json!({
            "_id": docid,
            "error": "not_found",
            "reason": e.to_string()
        })),
        None => Json(serde_json::json!({
            "_id": docid,
            "error": "not_found",
            "reason": "Key-Value engine not available"
        })),
    }
}

/// Insert or overwrite a document by ID (returns the new revision).
async fn kv_put_document(
    State(state): State<Arc<AppState>>,
    Path((db, docid)): Path<(String, String)>,
    Json(doc): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.put_document(&db, &docid, doc))
    }) {
        Some(Ok(document)) => Json(serde_json::json!({
            "ok": true,
            "id": docid,
            "rev": document._rev
        })),
        Some(Err(e)) => Json(serde_json::json!({"error": e.to_string()})),
        None => Json(serde_json::json!({"error": "Key-Value engine not available"})),
    }
}

/// Delete a document by ID; the `rev` query parameter is required.
async fn kv_delete_document(
    State(state): State<Arc<AppState>>,
    Path((db, docid)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let rev = params.get("rev").cloned().unwrap_or_default();
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.delete_document(&db, &docid, &rev))
    }) {
        Some(Ok(doc)) => Json(serde_json::json!({
            "ok": true,
            "id": docid,
            "rev": doc._rev
        })),
        Some(Err(e)) => Json(serde_json::json!({"error": e.to_string()})),
        None => Json(serde_json::json!({"error": "Key-Value engine not available"})),
    }
}

/// Update a document by ID (POST variant, returns the new revision).
async fn kv_update_document(
    State(state): State<Arc<AppState>>,
    Path((db, docid)): Path<(String, String)>,
    Json(doc): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let engine = state.primusdb.storage_engine(StorageType::KeyValue);
    match engine.and_then(|e| {
        e.as_any()
            .downcast_ref::<KeyValueEngine>()
            .map(|kv| kv.put_document(&db, &docid, doc))
    }) {
        Some(Ok(document)) => Json(serde_json::json!({
            "ok": true,
            "id": docid,
            "rev": document._rev
        })),
        Some(Err(e)) => Json(serde_json::json!({"error": e.to_string()})),
        None => Json(serde_json::json!({"error": "Key-Value engine not available"})),
    }
}

/// Authenticate a user and return their profile plus a login hint.
async fn login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let username = request.username.clone();
    match state.auth_service.login(request).await {
        Ok(result) => {
            state.primusdb.audit_log(
                "auth.login",
                &username,
                "user",
                "login",
                serde_json::json!({"user_id": result.user_id, "username": result.username}),
                true,
            );
            Ok(Json(APIResponse::success(serde_json::json!({
                "user_id": result.user_id,
                "username": result.username,
                "roles": result.roles,
                "segment_id": result.segment_id,
                "message": "Login successful. Use /api/v1/auth/token/create to generate an API token."
            }))))
        }
        Err(e) => {
            state.primusdb.audit_log(
                "auth.login",
                &username,
                "user",
                "login",
                serde_json::json!({"error": e.to_string()}),
                false,
            );
            Ok(Json(APIResponse::error(format!("Login failed: {}", e))))
        }
    }
}

/// Register a new user account.
async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterUserRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let username = request.username.clone();
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
        Ok(user_id) => {
            state.primusdb.audit_log(
                "auth.register",
                &username,
                "user",
                "create",
                serde_json::json!({"user_id": user_id}),
                true,
            );
            Ok(Json(APIResponse::success(serde_json::json!({
                "user_id": user_id,
                "message": "User created successfully"
            }))))
        }
        Err(e) => {
            state.primusdb.audit_log(
                "auth.register",
                &username,
                "user",
                "create",
                serde_json::json!({"error": e.to_string()}),
                false,
            );
            Ok(Json(APIResponse::error(format!(
                "Registration failed: {}",
                e
            ))))
        }
    }
}

/// Initiate MFA setup for the authenticated user.
async fn mfa_setup(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MfaSetupRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state
        .auth_service
        .validate_token(&request.authorization)
        .await
    {
        Ok(validation) => match state.auth_service.mfa_setup(&validation.username).await {
            Ok(setup) => {
                state.primusdb.audit_log(
                    "auth.mfa.setup",
                    &validation.username,
                    "user",
                    "mfa_setup",
                    serde_json::json!({"user_id": validation.user_id}),
                    true,
                );
                Ok(Json(APIResponse::success(serde_json::json!({
                    "secret": setup.secret,
                    "qr_code_url": setup.qr_code_url,
                    "backup_codes": setup.backup_codes,
                    "message": "MFA setup initiated. Verify with /api/v1/auth/mfa/verify to activate."
                }))))
            }
            Err(e) => Ok(Json(APIResponse::error(format!("MFA setup failed: {}", e)))),
        },
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Authorization failed: {}",
            e
        )))),
    }
}

/// Verify an MFA code and activate MFA for the authenticated user.
async fn mfa_verify(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MfaVerifyRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state
        .auth_service
        .validate_token(&request.authorization)
        .await
    {
        Ok(validation) => {
            match state
                .auth_service
                .mfa_verify(&validation.username, &request.code)
                .await
            {
                Ok(verified) => {
                    state.primusdb.audit_log(
                        "auth.mfa.verify",
                        &validation.username,
                        "user",
                        "mfa_verify",
                        serde_json::json!({"user_id": validation.user_id, "verified": verified}),
                        verified,
                    );
                    if verified {
                        Ok(Json(APIResponse::success(serde_json::json!({
                            "verified": true,
                            "message": "MFA enabled successfully"
                        }))))
                    } else {
                        Ok(Json(APIResponse::error("Invalid MFA code".to_string())))
                    }
                }
                Err(e) => Ok(Json(APIResponse::error(format!(
                    "MFA verification failed: {}",
                    e
                )))),
            }
        }
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Authorization failed: {}",
            e
        )))),
    }
}

/// Disable MFA for the authenticated user after verifying their code.
async fn mfa_disable(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MfaDisableRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state
        .auth_service
        .validate_token(&request.authorization)
        .await
    {
        Ok(validation) => {
            match state
                .auth_service
                .mfa_disable(&validation.username, &request.code)
                .await
            {
                Ok(()) => {
                    state.primusdb.audit_log(
                        "auth.mfa.disable",
                        &validation.username,
                        "user",
                        "mfa_disable",
                        serde_json::json!({"user_id": validation.user_id}),
                        true,
                    );
                    Ok(Json(APIResponse::success(serde_json::json!({
                        "message": "MFA disabled successfully"
                    }))))
                }
                Err(e) => Ok(Json(APIResponse::error(format!(
                    "MFA disable failed: {}",
                    e
                )))),
            }
        }
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Authorization failed: {}",
            e
        )))),
    }
}

/// Create an API token for the authenticated user.
async fn create_api_token(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateTokenRequestWithAuth>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let token_request = crate::auth::CreateTokenRequest {
        name: request.name.clone(),
        scopes: request.scopes.clone(),
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
                Ok((raw_token, token)) => {
                    state.primusdb.audit_log(
                        "auth.token.create",
                        &validation.user_id,
                        "token",
                        "create",
                        serde_json::json!({"token_id": token.id, "name": request.name}),
                        true,
                    );
                    Ok(Json(APIResponse::success(serde_json::json!({
                        "token": raw_token,
                        "token_id": token.id,
                        "expires_at": token.expires_at,
                        "message": "Store this token securely. It cannot be retrieved again."
                    }))))
                }
                Err(e) => {
                    state.primusdb.audit_log(
                        "auth.token.create",
                        &validation.user_id,
                        "token",
                        "create",
                        serde_json::json!({"error": e.to_string()}),
                        false,
                    );
                    Ok(Json(APIResponse::error(format!(
                        "Token creation failed: {}",
                        e
                    ))))
                }
            }
        }
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Authentication failed: {}",
            e
        )))),
    }
}

/// Revoke an API token by ID for the authenticated user.
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
        Ok(validation) => match state.auth_service.revoke_token(&token_id).await {
            Ok(()) => {
                state.primusdb.audit_log(
                    "auth.token.revoke",
                    &validation.user_id,
                    "token",
                    "revoke",
                    serde_json::json!({"token_id": token_id}),
                    true,
                );
                Ok(Json(APIResponse::success(serde_json::json!({
                    "message": "Token revoked successfully"
                }))))
            }
            Err(e) => {
                state.primusdb.audit_log(
                    "auth.token.revoke",
                    &validation.user_id,
                    "token",
                    "revoke",
                    serde_json::json!({"token_id": token_id, "error": e.to_string()}),
                    false,
                );
                Ok(Json(APIResponse::error(format!("Revoke failed: {}", e))))
            }
        },
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Authentication failed: {}",
            e
        )))),
    }
}

/// List the API tokens owned by the authenticated user.
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

/// List all users (admin-only, requires `Admin` permission).
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

/// List all defined roles.
async fn list_roles(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let roles = state.auth_service.list_roles().await;
    Ok(Json(APIResponse::success(serde_json::json!({
        "roles": roles
    }))))
}

/// Create a segment (admin-only, requires `Admin` permission).
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

/// Request body for user registration.
#[derive(Debug, Deserialize)]
pub struct RegisterUserRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub segment_id: Option<String>,
}

/// Request body for creating a new segment.
#[derive(Debug, Deserialize)]
pub struct CreateSegmentRequest {
    pub name: String,
    pub description: String,
    pub parent_segment: Option<String>,
}

/// Request body for API token creation, including the caller's authorization.
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequestWithAuth {
    pub authorization: String,
    pub name: String,
    pub scopes: Vec<crate::auth::TokenScope>,
    pub expires_in_hours: Option<u32>,
}

/// Request body for token revocation.
#[derive(Debug, Deserialize)]
pub struct RevokeTokenRequest {
    pub authorization: String,
}

/// Request body for listing a user's tokens.
#[derive(Debug, Deserialize)]
pub struct ListTokensRequest {
    pub authorization: String,
}

/// Request body for listing users.
#[derive(Debug, Deserialize)]
pub struct ListUsersRequest {
    pub authorization: String,
}

/// Request body for segment creation, including the caller's authorization.
#[derive(Debug, Deserialize)]
pub struct CreateSegmentRequestWithAuth {
    pub authorization: String,
    pub name: String,
    pub description: String,
    pub parent_segment: Option<String>,
}

/// Request body for initiating MFA setup.
#[derive(Debug, Deserialize)]
pub struct MfaSetupRequest {
    pub authorization: String,
}

/// Request body for verifying an MFA code during setup.
#[derive(Debug, Deserialize)]
pub struct MfaVerifyRequest {
    pub authorization: String,
    pub code: String,
}

/// Request body for disabling MFA.
#[derive(Debug, Deserialize)]
pub struct MfaDisableRequest {
    pub authorization: String,
    pub code: String,
}

/// Return the current consensus chain state.
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

/// Build and commit a block from the transaction mempool.
async fn consensus_build_block(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state.primusdb.build_and_commit_block().await {
        Ok(Some(block)) => Ok(Json(APIResponse::success(serde_json::json!({
            "hash": block.hash.as_str(),
            "height": block.height,
            "num_transactions": block.transactions.len(),
            "validator": block.validator,
        })))),
        Ok(None) => Ok(Json(APIResponse::success(serde_json::json!({
            "message": "No pending transactions in mempool"
        })))),
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Failed to build block: {}",
            e
        )))),
    }
}

/// Start the background block producer with the given interval (`interval_ms`,
/// default 5000).
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

/// Request body for creating a namespace.
#[derive(Debug, Deserialize)]
pub struct CreateNamespaceRequest {
    pub description: Option<String>,
    pub segment_id: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Request body for updating a namespace.
#[derive(Debug, Deserialize)]
pub struct UpdateNamespaceRequest {
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub segment_id: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Request body for attaching a storage resource to a namespace.
#[derive(Debug, Deserialize)]
pub struct AttachResourceRequest {
    pub storage_type: String,
    pub resource_name: String,
}

/// Request body for creating a namespace role.
#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
    pub inheritable: Option<bool>,
}

/// Request body for binding a user to a namespace role.
#[derive(Debug, Deserialize)]
pub struct AddUserBindingRequest {
    pub user_id: String,
    pub role_id: String,
    pub granted_by: String,
    pub expires_at: Option<String>,
}

// ── Database Management Types (v1.3.2-alpha) ──────────────────────────────

/// Summary of a database (namespace) as returned by the database endpoints.
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub description: String,
    pub engines: Vec<String>,
    pub table_count: usize,
    pub namespace_path: Option<String>,
}

/// Request body for creating a database.
///
/// When `namespace` is set, the database is created as the nested namespace
/// `parent.name`; otherwise `name` is used directly as the namespace path.
/// `engines` optionally lists storage engine types (`columnar`, `vector`,
/// `document`, `relational`, `keyvalue`/`kv`, `timeseries`/`ts`) for which a
/// starter table is created.
#[derive(Debug, Deserialize)]
pub struct CreateDatabaseRequest {
    pub name: String,
    pub description: Option<String>,
    pub engines: Option<Vec<String>>,
    pub namespace: Option<String>,
}

// ── Database Handlers ─────────────────────────────────────────────────────

/// List all databases (namespaces) with their engine and table summary.
async fn list_databases(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<Vec<DatabaseInfo>>> {
    let ns_ctrl = state.primusdb.get_namespace_controller();
    match ns_ctrl.list_all() {
        Ok(namespaces) => {
            let databases: Vec<DatabaseInfo> = namespaces
                .iter()
                .map(|ns| {
                    let resources = ns_ctrl.list_resources(&ns.id).unwrap_or_default();
                    let engines: Vec<String> = resources
                        .iter()
                        .map(|r| format!("{:?}", r.storage_type).to_lowercase())
                        .collect();
                    DatabaseInfo {
                        name: ns.path.clone(),
                        description: ns.description.clone(),
                        engines,
                        table_count: resources.len(),
                        namespace_path: Some(ns.path.clone()),
                    }
                })
                .collect();
            Json(APIResponse::success(databases))
        }
        Err(e) => Json(APIResponse::error(e.to_string())),
    }
}

/// Create a database (namespace), optionally creating starter tables for the
/// requested engine types. Idempotent for existing namespaces.
async fn create_database_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateDatabaseRequest>,
) -> Result<Json<APIResponse<DatabaseInfo>>, StatusCode> {
    let ns_ctrl = state.primusdb.get_namespace_controller();
    let description = request.description.unwrap_or_default();

    // With --namespace, the database is created as a nested namespace `parent.name`.
    let path = match &request.namespace {
        Some(ns) if !ns.is_empty() => format!("{}.{}", ns, request.name),
        _ => request.name.clone(),
    };

    let ns = match ns_ctrl.create(
        &path,
        &description,
        None,
        None,
        std::collections::HashMap::new(),
    ) {
        Ok(ns) => ns,
        Err(crate::Error::ValidationError(msg))
            if msg == format!("Namespace '{}' already exists", path) =>
        {
            // Idempotent create: the database/namespace already exists.
            let existing = match ns_ctrl.get_by_path(&path) {
                Ok(Some(existing)) => existing,
                _ => {
                    return Ok(Json(APIResponse::error(format!(
                        "Failed to create database '{}': namespace '{}' already exists but could not be loaded",
                        request.name, path
                    ))))
                }
            };
            let resources = ns_ctrl.list_resources(&existing.id).unwrap_or_default();
            return Ok(Json(APIResponse::success(DatabaseInfo {
                name: existing.path.clone(),
                description: existing.description.clone(),
                engines: resources
                    .iter()
                    .map(|r| format!("{:?}", r.storage_type).to_lowercase())
                    .collect(),
                table_count: resources.len(),
                namespace_path: Some(existing.path),
            })));
        }
        Err(e) => {
            tracing::error!("Failed to create database '{}': {}", request.name, e);
            return Ok(Json(APIResponse::error(format!(
                "Failed to create database '{}': {}",
                request.name, e
            ))));
        }
    };

    let engines = request.engines.unwrap_or_default();
    let mut created_tables = Vec::new();
    for engine_str in &engines {
        let st = match engine_str.to_lowercase().as_str() {
            "columnar" => StorageType::Columnar,
            "vector" => StorageType::Vector,
            "document" => StorageType::Document,
            "relational" => StorageType::Relational,
            "keyvalue" | "kv" => StorageType::KeyValue,
            "timeseries" | "ts" => StorageType::TimeSeries,
            _ => continue,
        };
        let table_name = match engine_str.to_lowercase().as_str() {
            "relational" => "users",
            "columnar" => "analytics_events",
            "document" => "app_configs",
            "keyvalue" | "kv" => "session_store",
            "vector" => "embeddings",
            "timeseries" | "ts" => "sensor_readings",
            _ => continue,
        };
        if state.primusdb.create_table(st, table_name).await.is_ok() {
            created_tables.push(engine_str.clone());
        }
    }
    match state
        .primusdb
        .integrity()
        .create_database_genesis(crate::integrity::NewDatabaseGenesis {
            database_name: &ns.path,
            namespace: request.namespace.as_deref(),
            engine_types: &engines,
            config_digest: &crate::integrity::genesis::digest_value(&serde_json::json!({
                "description": description,
            })),
            schema_digest: None,
            parent_identity: None,
            origin: crate::integrity::GenesisOrigin::Created,
        }) {
        Ok(_) => {}
        // Genesis already present: idempotent behaviour, not an error.
        Err(crate::integrity::IntegrityError::GenesisAlreadyExists(_)) => {}
        Err(e) => tracing::warn!("database '{}' created without genesis: {}", ns.path, e),
    }
    let db_info = DatabaseInfo {
        name: ns.path.clone(),
        description: ns.description.clone(),
        engines: created_tables,
        table_count: 0,
        namespace_path: Some(ns.path.clone()),
    };
    Ok(Json(APIResponse::success(db_info)))
}

// ── Integrity Handlers (v1.3.2-alpha engine-integrity-graphql) ──────────────

/// Global integrity subsystem status.
async fn integrity_status_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<crate::integrity::IntegrityStatus>>, StatusCode> {
    let status = state.primusdb.integrity().status().await;
    Ok(Json(APIResponse::success(status)))
}

/// GET `/api/v1/graphql`: describe the supported GraphQL schema (SDL).
async fn graphql_schema_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    if !state.primusdb.config().graphql.enabled {
        return Ok(Json(APIResponse::error(
            "GraphQL service is disabled".to_string(),
        )));
    }
    Ok(Json(APIResponse::success(serde_json::json!({
        "schema": crate::graphql::SCHEMA_SDL,
        "notes": "Fragments, directives, subscriptions and full introspection are not supported",
    }))))
}

/// POST `/api/v1/graphql`: execute a GraphQL document.
///
/// The canonical wire format is `{"query": "...", "operationName": "...",
/// "variables": {...}}`. The response is a standard
/// `{"data": ..., "errors": [...]}` payload.
async fn graphql_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<crate::graphql::GraphQLResponse>, StatusCode> {
    if !state.primusdb.config().graphql.enabled {
        return Err(StatusCode::NOT_FOUND);
    }
    let request = crate::graphql::GraphQLRequest::from_json(body);
    let response = crate::graphql::GraphQLExecutor::execute(&state.primusdb, &request).await;
    Ok(Json(response))
}

/// Capability snapshot for drivers, the REPL and discovery tooling.
///
/// Advertises the node identity, protocol version, every storage engine with
/// its tables (capability registry) and additive feature flags. Clients must
/// only fail on *missing* features they require, never on unknown ones.
async fn capabilities_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<crate::capabilities::ServerCapabilities>>, StatusCode> {
    match state.primusdb.capabilities() {
        Ok(caps) => Ok(Json(APIResponse::success(caps))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Unified search across all storage engines.
///
/// Query parameters: `q` (full-text), `query_vector` (JSON array),
/// `mode` (`and`|`or`|`phrase`), `storage_types` (comma-separated),
/// `tables` (comma-separated), `limit`, `offset`.
async fn search_handler(
    State(state): State<Arc<AppState>>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Result<Json<APIResponse<crate::search::SearchResponse>>, StatusCode> {
    use crate::search::{SearchMode, SearchRequest, SearchService, ALL_ENGINES};
    use std::str::FromStr;

    let mut storage_types = ALL_ENGINES.to_vec();
    if let Some(types) = params.get("storage_types") {
        storage_types = types
            .split(',')
            .filter_map(|s| crate::StorageType::from_str(s.trim()).ok())
            .collect();
        if storage_types.is_empty() {
            return Ok(Json(APIResponse::error(
                "No valid storage_types provided".to_string(),
            )));
        }
    }

    let tables = params.get("tables").map(|t| {
        t.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    });

    let query_vector = match params.get("query_vector") {
        Some(v) => Some(
            serde_json::from_str::<serde_json::Value>(v).map_err(|_| StatusCode::BAD_REQUEST)?,
        ),
        None => None,
    };

    let mode = params.get("mode").map(|m| match m.to_lowercase().as_str() {
        "or" => SearchMode::Or,
        "phrase" => SearchMode::Phrase,
        _ => SearchMode::And,
    });

    let request = SearchRequest {
        query: params.get("q").cloned(),
        query_vector,
        mode,
        storage_types: Some(storage_types),
        tables,
        limit: params
            .get("limit")
            .and_then(|l| l.parse().ok())
            .or(Some(20)),
        offset: params.get("offset").and_then(|o| o.parse().ok()),
    };

    match SearchService::search(&state.primusdb, &request).await {
        Ok(resp) => Ok(Json(APIResponse::success(resp))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Signed genesis identity of a database.
async fn integrity_genesis_handler(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Result<Json<APIResponse<Option<crate::integrity::DatabaseGenesis>>>, StatusCode> {
    match state.primusdb.integrity().get_genesis(&db) {
        Ok(genesis) => Ok(Json(APIResponse::success(genesis))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Integrity records (signed hash chain) of a database.
async fn integrity_records_handler(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Result<Json<APIResponse<Vec<crate::integrity::IntegrityRecord>>>, StatusCode> {
    match state.primusdb.integrity().list_records(&db) {
        Ok(records) => Ok(Json(APIResponse::success(records))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Verifies genesis + full record chain for a database.
async fn integrity_verify_handler(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Result<Json<APIResponse<crate::integrity::ChainVerification>>, StatusCode> {
    match state.primusdb.integrity().verify_chain(&db) {
        Ok(verification) => Ok(Json(APIResponse::success(verification))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Checkpoints anchored for a database.
async fn integrity_checkpoints_handler(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Result<Json<APIResponse<Vec<crate::integrity::Checkpoint>>>, StatusCode> {
    match state.primusdb.integrity().list_checkpoints(&db) {
        Ok(checkpoints) => Ok(Json(APIResponse::success(checkpoints))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Creates a new signed checkpoint for a database.
async fn integrity_checkpoint_create_handler(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Result<Json<APIResponse<crate::integrity::Checkpoint>>, StatusCode> {
    match state.primusdb.integrity().create_checkpoint(&db).await {
        Ok(cp) => Ok(Json(APIResponse::success(cp))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Records awaiting ledger anchoring.
async fn integrity_pending_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<Vec<crate::integrity::IntegrityRecord>>>, StatusCode> {
    match state.primusdb.integrity().list_pending() {
        Ok(pending) => Ok(Json(APIResponse::success(pending))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Retries pending ledger submissions.
async fn integrity_pending_flush_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state.primusdb.integrity().flush_pending().await {
        Ok(confirmed) => Ok(Json(APIResponse::success(serde_json::json!({
            "confirmed": confirmed
        })))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Quarantined (invalid) records.
async fn integrity_quarantine_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<Vec<crate::integrity::IntegrityRecord>>>, StatusCode> {
    match state.primusdb.integrity().list_quarantined() {
        Ok(quarantined) => Ok(Json(APIResponse::success(quarantined))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Releases a quarantined record.
async fn integrity_quarantine_release_handler(
    State(state): State<Arc<AppState>>,
    Path((db, sequence)): Path<(String, u64)>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    match state
        .primusdb
        .integrity()
        .release_quarantined(&db, sequence)
    {
        Ok(()) => Ok(Json(APIResponse::success(serde_json::json!({
            "released": format!("{}/{}", db, sequence)
        })))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Compact chain evidence offered to peers before a reconciliation.
///
/// Nodes compare counts + last hashes first; full records are only exchanged
/// when this evidence differs. This is the integrity-first handshake.
async fn integrity_reconcile_evidence_handler(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
) -> Result<Json<APIResponse<crate::integrity::ChainEvidence>>, StatusCode> {
    match state.primusdb.integrity().chain_evidence(&db) {
        Ok(evidence) => Ok(Json(APIResponse::success(evidence))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Reconciles the local integrity chain against a peer's chain.
///
/// The body carries the peer's full records; the handler returns the
/// `ReconciliationReport` plus the `RepairPlan` the operator may execute.
/// Nothing is applied automatically.
async fn integrity_reconcile_handler(
    State(state): State<Arc<AppState>>,
    Path(db): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let peer_records: Vec<crate::integrity::IntegrityRecord> =
        match serde_json::from_value(body.get("peer_records").cloned().unwrap_or_default()) {
            Ok(records) => records,
            Err(e) => {
                return Ok(Json(APIResponse::error(format!(
                    "invalid peer_records: {e}"
                ))))
            }
        };
    match state.primusdb.integrity().reconcile(&db, &peer_records) {
        Ok(report) => {
            let plan = crate::integrity::plan_repair(&report);
            Ok(Json(APIResponse::success(serde_json::json!({
                "report": report,
                "repair_plan": plan,
            }))))
        }
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Real Hyperledger connectivity health.
async fn ledger_status_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<crate::hyperledger::HyperledgerHealth>>, StatusCode> {
    let health = match state.primusdb.hyperledger() {
        Some(client) => client.health().await,
        None => crate::hyperledger::HyperledgerHealth::unconfigured(
            &crate::hyperledger::HyperledgerConfig::default(),
        ),
    };
    Ok(Json(APIResponse::success(health)))
}

// ── Namespace Handlers ───────────────────────────────────────────────────────

/// List all namespaces.
async fn list_namespaces(
    State(state): State<Arc<AppState>>,
) -> Result<Json<APIResponse<Vec<namespace::Namespace>>>, StatusCode> {
    match state.primusdb.get_namespace_controller().list_all() {
        Ok(namespaces) => Ok(Json(APIResponse::success(namespaces))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Create a namespace at the given dot-separated path.
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

/// Fetch a single namespace by path.
async fn get_namespace(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<namespace::Namespace>>, StatusCode> {
    match state.primusdb.get_namespace_controller().get_by_path(&path) {
        Ok(Some(ns)) => Ok(Json(APIResponse::success(ns))),
        Ok(None) => Ok(Json(APIResponse::error(format!(
            "Namespace '{}' not found",
            path
        )))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Update a namespace's description, activity, segment or metadata.
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
    match state
        .primusdb
        .get_namespace_controller()
        .update(&path, update)
    {
        Ok(ns) => Ok(Json(APIResponse::success(ns))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Delete a namespace by path.
async fn delete_namespace(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<()>>, StatusCode> {
    match state.primusdb.get_namespace_controller().delete(&path) {
        Ok(()) => Ok(Json(APIResponse::success(()))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// List the child namespaces of the given path.
async fn list_namespace_children(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<Vec<namespace::Namespace>>>, StatusCode> {
    match state
        .primusdb
        .get_namespace_controller()
        .list_children(&path)
    {
        Ok(children) => Ok(Json(APIResponse::success(children))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Return the effective (inherited) policy for a namespace path.
async fn get_effective_policy(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<namespace::NamespacePolicies>>, StatusCode> {
    match state
        .primusdb
        .get_namespace_controller()
        .effective_policy(&path)
    {
        Ok(policy) => Ok(Json(APIResponse::success(policy))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// List the storage resources attached to a namespace.
async fn list_namespace_resources(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<Vec<namespace::NamespaceResource>>>, StatusCode> {
    match state.primusdb.get_namespace_controller().get_by_path(&path) {
        Ok(Some(ns)) => match state
            .primusdb
            .get_namespace_controller()
            .list_resources(&ns.id)
        {
            Ok(resources) => Ok(Json(APIResponse::success(resources))),
            Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
        },
        Ok(None) => Ok(Json(APIResponse::error(format!(
            "Namespace '{}' not found",
            path
        )))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Attach a storage resource to a namespace.
async fn attach_resource(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Json(request): Json<AttachResourceRequest>,
) -> Result<Json<APIResponse<namespace::NamespaceResource>>, StatusCode> {
    let st = match parse_storage_type(&request.storage_type) {
        Ok(t) => t,
        Err(e) => {
            return Ok(Json(APIResponse::error(format!(
                "Invalid storage type: {}",
                e
            ))))
        }
    };
    match state.primusdb.get_namespace_controller().attach_resource(
        &path,
        st,
        &request.resource_name,
    ) {
        Ok(resource) => Ok(Json(APIResponse::success(resource))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Detach a storage resource from a namespace.
async fn detach_resource(
    State(state): State<Arc<AppState>>,
    Path((path, storage_type, resource_name)): Path<(String, String, String)>,
) -> Result<Json<APIResponse<()>>, StatusCode> {
    let st = match parse_storage_type(&storage_type) {
        Ok(t) => t,
        Err(e) => {
            return Ok(Json(APIResponse::error(format!(
                "Invalid storage type: {}",
                e
            ))))
        }
    };
    match state
        .primusdb
        .get_namespace_controller()
        .detach_resource(&path, st, &resource_name)
    {
        Ok(()) => Ok(Json(APIResponse::success(()))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// List the roles defined on a namespace.
async fn list_namespace_roles(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<Vec<namespace::NamespaceRole>>>, StatusCode> {
    match state.primusdb.get_namespace_controller().get_by_path(&path) {
        Ok(Some(ns)) => match state.primusdb.get_namespace_controller().list_roles(&ns.id) {
            Ok(roles) => Ok(Json(APIResponse::success(roles))),
            Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
        },
        Ok(None) => Ok(Json(APIResponse::error(format!(
            "Namespace '{}' not found",
            path
        )))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Create a role on a namespace from its permission strings.
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
            "attach_resource" | "attachresource" => {
                Some(namespace::NamespacePermission::AttachResource)
            }
            "detach_resource" | "detachresource" => {
                Some(namespace::NamespacePermission::DetachResource)
            }
            "manage_users" | "manageusers" => Some(namespace::NamespacePermission::ManageUsers),
            "manage_roles" | "manageroles" => Some(namespace::NamespacePermission::ManageRoles),
            "manage_policies" | "managepolicies" => {
                Some(namespace::NamespacePermission::ManagePolicies)
            }
            "cross_namespace_read" | "crossnamespaceread" => {
                Some(namespace::NamespacePermission::CrossNamespaceRead)
            }
            "cross_namespace_write" | "crossnamespacewrite" => {
                Some(namespace::NamespacePermission::CrossNamespaceWrite)
            }
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

/// Delete a role from a namespace.
async fn delete_namespace_role(
    State(state): State<Arc<AppState>>,
    Path((path, role_id)): Path<(String, String)>,
) -> Result<Json<APIResponse<()>>, StatusCode> {
    match state
        .primusdb
        .get_namespace_controller()
        .remove_role(&path, &role_id)
    {
        Ok(()) => Ok(Json(APIResponse::success(()))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// List the user→role bindings on a namespace.
async fn list_namespace_user_bindings(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<APIResponse<Vec<namespace::NamespaceUserBinding>>>, StatusCode> {
    match state.primusdb.get_namespace_controller().get_by_path(&path) {
        Ok(Some(ns)) => match state
            .primusdb
            .get_namespace_controller()
            .list_user_bindings(&ns.id)
        {
            Ok(bindings) => Ok(Json(APIResponse::success(bindings))),
            Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
        },
        Ok(None) => Ok(Json(APIResponse::error(format!(
            "Namespace '{}' not found",
            path
        )))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

/// Bind a user to a role on a namespace, optionally with an expiry.
async fn add_namespace_user_binding(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Json(request): Json<AddUserBindingRequest>,
) -> Result<Json<APIResponse<namespace::NamespaceUserBinding>>, StatusCode> {
    let expires_at = match request.expires_at {
        Some(s) => Some(
            chrono::DateTime::parse_from_rfc3339(&s)
                .map_err(|_| StatusCode::BAD_REQUEST)?
                .with_timezone(&chrono::Utc),
        ),
        None => None,
    };

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

/// Remove a user's role binding from a namespace.
async fn remove_namespace_user_binding(
    State(state): State<Arc<AppState>>,
    Path((path, user_id)): Path<(String, String)>,
) -> Result<Json<APIResponse<()>>, StatusCode> {
    match state
        .primusdb
        .get_namespace_controller()
        .remove_user_binding(&path, &user_id)
    {
        Ok(()) => Ok(Json(APIResponse::success(()))),
        Err(e) => Ok(Json(APIResponse::error(e.to_string()))),
    }
}

// ── Resource Governor handlers ────────────────────────────────

use crate::governor::engine::GovernorEngine;
use crate::governor::GovernorConfig;

fn governor_engine() -> &'static GovernorEngine {
    static ENGINE: std::sync::OnceLock<GovernorEngine> = std::sync::OnceLock::new();
    ENGINE.get_or_init(|| GovernorEngine::new(GovernorConfig::default()))
}

#[derive(Serialize)]
struct GovernorStatusResponse {
    enabled: bool,
    active_executions: usize,
    total_violations: u64,
    blocked_count: u64,
    throttled_count: u64,
    policies_loaded: usize,
    uptime_seconds: u64,
}

#[derive(Serialize)]
struct GovernorPolicyResponse {
    name: String,
    scope: String,
    scope_name: String,
    action: String,
    max_memory_mb: Option<u64>,
    max_execution_steps: Option<u64>,
}

#[derive(Serialize)]
struct GovernorMetricsResponse {
    active_executions: usize,
    blocked_total: u64,
    throttled_total: u64,
    policy_violations_total: u64,
    memory_usage_bytes: u64,
    cpu_time_ms: u64,
    ffi_calls_total: u64,
}

#[derive(Serialize)]
struct ViolationResponse {
    id: String,
    timestamp: String,
    execution_id: String,
    namespace: String,
    workload_type: String,
    limit_name: String,
    limit_value: String,
    usage_value: String,
    action: String,
}

#[derive(Serialize)]
struct ExecutionResponse {
    execution_id: String,
    namespace: String,
    workload_type: String,
    action: String,
    created_at: String,
    elapsed_ms: i64,
}

/// Report resource governor status (enabled, active executions, violations).
async fn governor_status_handler() -> Json<APIResponse<GovernorStatusResponse>> {
    let status = governor_engine().status().await;
    Json(APIResponse::success(GovernorStatusResponse {
        enabled: status.enabled,
        active_executions: status.active_executions,
        total_violations: status.total_violations,
        blocked_count: status.blocked_count,
        throttled_count: status.throttled_count,
        policies_loaded: status.policies_loaded,
        uptime_seconds: status.uptime_seconds,
    }))
}

/// List the resource governor policies currently loaded.
async fn governor_policies_handler() -> Json<APIResponse<Vec<GovernorPolicyResponse>>> {
    let policies = governor_engine().policies().await;
    let response: Vec<GovernorPolicyResponse> = policies
        .into_iter()
        .map(|p| GovernorPolicyResponse {
            name: p.name,
            scope: p.scope.as_str().to_string(),
            scope_name: p.scope.name().to_string(),
            action: p.action.as_str().to_string(),
            max_memory_mb: p.limits.memory.max_memory_mb,
            max_execution_steps: p.limits.cpu.max_execution_steps,
        })
        .collect();
    Json(APIResponse::success(response))
}

/// Return a snapshot of resource governor metrics.
async fn governor_metrics_handler() -> Json<APIResponse<GovernorMetricsResponse>> {
    let metrics = governor_engine().metrics_snapshot().await;
    Json(APIResponse::success(GovernorMetricsResponse {
        active_executions: metrics.active_executions,
        blocked_total: metrics.blocked_total,
        throttled_total: metrics.throttled_total,
        policy_violations_total: metrics.policy_violations_total,
        memory_usage_bytes: metrics.memory_usage_bytes,
        cpu_time_ms: metrics.cpu_time_ms,
        ffi_calls_total: metrics.ffi_calls_total,
    }))
}

/// List recorded resource governor policy violations.
async fn governor_violations_handler() -> Json<APIResponse<Vec<ViolationResponse>>> {
    let violations = governor_engine().list_violations().await;
    let response: Vec<ViolationResponse> = violations
        .into_iter()
        .map(|v| ViolationResponse {
            id: v.id.to_string(),
            timestamp: v.timestamp.to_rfc3339(),
            execution_id: v.execution_id.to_string(),
            namespace: v.namespace,
            workload_type: v.workload_type.as_str().to_string(),
            limit_name: v.limit_name,
            limit_value: v.limit_value,
            usage_value: v.usage_value,
            action: v.action.as_str().to_string(),
        })
        .collect();
    Json(APIResponse::success(response))
}

/// List active and recent resource governor executions.
async fn governor_executions_handler() -> Json<APIResponse<Vec<ExecutionResponse>>> {
    let executions = governor_engine().list_executions().await;
    let response: Vec<ExecutionResponse> = executions
        .into_iter()
        .map(|e| ExecutionResponse {
            execution_id: e.execution_id.to_string(),
            namespace: e.namespace.clone(),
            workload_type: e.workload_type.as_str().to_string(),
            action: e.action.as_str().to_string(),
            created_at: e.created_at.to_rfc3339(),
            elapsed_ms: e.elapsed_ms(),
        })
        .collect();
    Json(APIResponse::success(response))
}

// ── Governor POST request/response types ─────────────────────

#[derive(Deserialize)]
struct GovernorStartRequest {
    namespace: String,
    workload_type: String,
    user: Option<String>,
    role: Option<String>,
}

#[derive(Serialize)]
struct GovernorStartResponse {
    execution_id: String,
    action: String,
}

#[derive(Deserialize)]
struct GovernorCheckRequest {
    check_type: String,
    value: u64,
}

#[derive(Serialize)]
struct GovernorCheckResponse {
    action: String,
    message: Option<String>,
}

#[derive(Deserialize)]
struct GovernorUpdatePolicyRequest {
    name: String,
    limits: serde_json::Value,
    action: String,
    scope: String,
}

// ── Governor POST handlers ───────────────────────────────────

fn parse_workload_type(s: &str) -> Result<crate::governor::WorkloadType, StatusCode> {
    match s {
        "sql" => Ok(crate::governor::WorkloadType::Sql),
        "vector_search" => Ok(crate::governor::WorkloadType::VectorSearch),
        "ai_ml" => Ok(crate::governor::WorkloadType::AIML),
        "graph_traversal" => Ok(crate::governor::WorkloadType::GraphTraversal),
        "cdc_pipeline" => Ok(crate::governor::WorkloadType::CdcPipeline),
        "backup" => Ok(crate::governor::WorkloadType::Backup),
        "restore" => Ok(crate::governor::WorkloadType::Restore),
        "migration" => Ok(crate::governor::WorkloadType::Migration),
        "cluster_operation" => Ok(crate::governor::WorkloadType::ClusterOperation),
        "protocol_processing" => Ok(crate::governor::WorkloadType::ProtocolProcessing),
        "udf" => Ok(crate::governor::WorkloadType::UserDefinedFunction),
        "stored_procedure" => Ok(crate::governor::WorkloadType::StoredProcedure),
        "plugin" => Ok(crate::governor::WorkloadType::Plugin),
        "ffi" => Ok(crate::governor::WorkloadType::Ffi),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

fn parse_enforcement_action(s: &str) -> Result<crate::governor::EnforcementAction, StatusCode> {
    match s {
        "monitor" => Ok(crate::governor::EnforcementAction::Monitor),
        "warn" => Ok(crate::governor::EnforcementAction::Warn),
        "throttle" => Ok(crate::governor::EnforcementAction::Throttle),
        "block" => Ok(crate::governor::EnforcementAction::Block),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// Start a governed execution for a workload (returns the execution ID and the
/// enforcement action applied).
async fn governor_start_execution_handler(
    Json(body): Json<GovernorStartRequest>,
) -> Result<Json<APIResponse<GovernorStartResponse>>, StatusCode> {
    let engine = governor_engine();
    if !engine.is_enabled().await {
        return Ok(Json(APIResponse::error("Governor is disabled".to_string())));
    }
    let wt = parse_workload_type(&body.workload_type)?;
    let handle = engine
        .start_execution(
            body.namespace,
            wt,
            body.user.as_deref(),
            body.role.as_deref(),
        )
        .await;
    Ok(Json(APIResponse::success(GovernorStartResponse {
        execution_id: handle.id().to_string(),
        action: handle.action().as_str().to_string(),
    })))
}

/// Finish (release) a governed execution by ID.
async fn governor_finish_execution_handler(
    Path(execution_id): Path<String>,
) -> Result<Json<APIResponse<()>>, StatusCode> {
    let eid = uuid::Uuid::parse_str(&execution_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    governor_engine().finish_execution(eid).await;
    Ok(Json(APIResponse::success(())))
}

/// Check a resource limit for a governed execution (returns the action to
/// take).
async fn governor_check_limit_handler(
    Path(execution_id): Path<String>,
    Json(body): Json<GovernorCheckRequest>,
) -> Result<Json<APIResponse<GovernorCheckResponse>>, StatusCode> {
    let eid = uuid::Uuid::parse_str(&execution_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let engine = governor_engine();
    let result = engine
        .check_limit(eid, &body.check_type, body.value, None)
        .await;
    match result {
        Ok(action) => Ok(Json(APIResponse::success(GovernorCheckResponse {
            action: action.as_str().to_string(),
            message: None,
        }))),
        Err(msg) => Ok(Json(APIResponse::success(GovernorCheckResponse {
            action: "block".to_string(),
            message: Some(msg),
        }))),
    }
}

/// Update a resource governor policy (limits, enforcement action, scope).
async fn governor_update_policy_handler(
    Json(body): Json<GovernorUpdatePolicyRequest>,
) -> Result<Json<APIResponse<()>>, StatusCode> {
    let engine = governor_engine();
    let action = parse_enforcement_action(&body.action)?;
    let limits = serde_json::from_value::<crate::governor::ExecutionLimits>(body.limits)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    engine
        .update_policy(&body.name, limits, action, body.scope)
        .await;
    Ok(Json(APIResponse::success(())))
}

// ── Config Management Handlers (v1.3.2-alpha) ─────────────────────

#[derive(Deserialize)]
struct SetConfigRequest {
    key: String,
    value: serde_json::Value,
    source: Option<String>,
}

#[derive(Deserialize)]
struct DeleteConfigRequest {
    key: String,
}

#[derive(Deserialize)]
struct ValidateConfigRequest {
    key: String,
    value: serde_json::Value,
}

#[derive(Serialize)]
struct ConfigEntryResponse {
    key: String,
    value: serde_json::Value,
    source: String,
    updated_at: String,
}

#[derive(Serialize)]
struct ConfigSnapshotResponse {
    id: String,
    name: String,
    entries_count: usize,
    created_at: String,
    description: String,
}

fn get_config_store(state: &AppState) -> Option<&crate::system::config_store::ConfigStore> {
    state.primusdb.system_db().map(|sys| &sys.config)
}

fn sysdb_error<T>() -> Json<APIResponse<T>> {
    Json(APIResponse::error(
        "System database not initialized".to_string(),
    ))
}

/// List all configuration entries stored in the system database.
async fn list_config_entries(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<Vec<ConfigEntryResponse>>> {
    let store = match get_config_store(&state) {
        Some(s) => s,
        None => return sysdb_error(),
    };
    match store.list_all() {
        Ok(entries) => {
            let response: Vec<ConfigEntryResponse> = entries
                .into_iter()
                .map(|e| ConfigEntryResponse {
                    key: e.key,
                    value: e.value,
                    source: e.source.to_string(),
                    updated_at: e.updated_at.to_rfc3339(),
                })
                .collect();
            Json(APIResponse::success(response))
        }
        Err(e) => Json(APIResponse::error(format!("Failed to list config: {}", e))),
    }
}

/// Set or override a configuration entry.
async fn set_config_entry(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetConfigRequest>,
) -> Json<APIResponse<ConfigEntryResponse>> {
    let store = match get_config_store(&state) {
        Some(s) => s,
        None => return sysdb_error(),
    };
    let source = match body.source.as_deref() {
        Some("config file") | Some("config_file") => {
            crate::system::config_store::ConfigSource::ConfigFile
        }
        Some("env var") | Some("env_var") | Some("environment") => {
            crate::system::config_store::ConfigSource::EnvironmentVariable
        }
        Some("system database") | Some("system_db") => {
            crate::system::config_store::ConfigSource::SystemDatabase
        }
        Some("runtime override") | Some("runtime") => {
            crate::system::config_store::ConfigSource::RuntimeOverride
        }
        Some("TUI profile") | Some("tui") => crate::system::config_store::ConfigSource::TuiProfile,
        _ => crate::system::config_store::ConfigSource::RuntimeOverride,
    };
    if let Err(msg) = store.validate(&body.key, &body.value) {
        return Json(APIResponse::error(format!("Validation failed: {}", msg)));
    }
    if let Err(e) = store.set(&body.key, body.value.clone(), source) {
        return Json(APIResponse::error(format!("Failed to set config: {}", e)));
    }
    match store.get(&body.key) {
        Ok(Some(e)) => Json(APIResponse::success(ConfigEntryResponse {
            key: e.key,
            value: e.value,
            source: e.source.to_string(),
            updated_at: e.updated_at.to_rfc3339(),
        })),
        Ok(None) => Json(APIResponse::error(
            "Config entry not found after write".to_string(),
        )),
        Err(e) => Json(APIResponse::error(format!("Failed to read config: {}", e))),
    }
}

/// Delete a configuration entry.
async fn delete_config_entry_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<DeleteConfigRequest>,
) -> Json<APIResponse<()>> {
    let store = match get_config_store(&state) {
        Some(s) => s,
        None => return sysdb_error(),
    };
    match store.delete(&body.key) {
        Ok(()) => Json(APIResponse::success(())),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to delete config: {}",
            e
        ))),
    }
}

/// Validate a configuration value against the config store's rules.
async fn validate_config_entry(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ValidateConfigRequest>,
) -> Json<APIResponse<serde_json::Value>> {
    let store = match get_config_store(&state) {
        Some(s) => s,
        None => return sysdb_error(),
    };
    match store.validate(&body.key, &body.value) {
        Ok(()) => Json(APIResponse::success(serde_json::json!({
            "valid": true,
            "key": body.key,
        }))),
        Err(msg) => Json(APIResponse::success(serde_json::json!({
            "valid": false,
            "key": body.key,
            "error": msg,
        }))),
    }
}

/// Export the configuration store as a bundle.
async fn export_config_bundle(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    let store = match get_config_store(&state) {
        Some(s) => s,
        None => return sysdb_error(),
    };
    match store.export_bundle() {
        Ok(bundle) => match serde_json::to_value(&bundle) {
            Ok(value) => Json(APIResponse::success(value)),
            Err(e) => Json(APIResponse::error(format!("Serialization error: {}", e))),
        },
        Err(e) => Json(APIResponse::error(format!("Failed to export: {}", e))),
    }
}

#[derive(Deserialize)]
struct ImportBundleBody {
    bundle: serde_json::Value,
}

/// Import a configuration bundle (returns the number of entries imported).
async fn import_config_bundle(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ImportBundleBody>,
) -> Json<APIResponse<serde_json::Value>> {
    let store = match get_config_store(&state) {
        Some(s) => s,
        None => return sysdb_error(),
    };
    let bundle: crate::system::config_store::ConfigBundle =
        match serde_json::from_value(body.bundle) {
            Ok(b) => b,
            Err(e) => {
                return Json(APIResponse::error(format!("Invalid bundle format: {}", e)));
            }
        };
    match store.import_bundle(&bundle) {
        Ok(count) => Json(APIResponse::success(serde_json::json!({
            "imported": count,
        }))),
        Err(e) => Json(APIResponse::error(format!("Failed to import: {}", e))),
    }
}

#[derive(Deserialize)]
struct CreateSnapshotRequest {
    name: String,
    description: Option<String>,
}

/// List configuration snapshots.
async fn list_config_snapshots(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<Vec<ConfigSnapshotResponse>>> {
    let store = match get_config_store(&state) {
        Some(s) => s,
        None => return sysdb_error(),
    };
    match store.list_snapshots() {
        Ok(snapshots) => {
            let response: Vec<ConfigSnapshotResponse> = snapshots
                .into_iter()
                .map(|s| ConfigSnapshotResponse {
                    id: s.id,
                    name: s.name,
                    entries_count: s.entries.len(),
                    created_at: s.created_at.to_rfc3339(),
                    description: s.description,
                })
                .collect();
            Json(APIResponse::success(response))
        }
        Err(e) => Json(APIResponse::error(format!(
            "Failed to list snapshots: {}",
            e
        ))),
    }
}

/// Create a named configuration snapshot.
async fn create_config_snapshot(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateSnapshotRequest>,
) -> Json<APIResponse<serde_json::Value>> {
    let store = match get_config_store(&state) {
        Some(s) => s,
        None => return sysdb_error(),
    };
    match store.create_snapshot(&body.name, &body.description.unwrap_or_default()) {
        Ok(id) => Json(APIResponse::success(serde_json::json!({
            "id": id,
            "name": body.name,
        }))),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to create snapshot: {}",
            e
        ))),
    }
}

/// Restore configuration from a snapshot by ID.
async fn restore_config_snapshot(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    let store = match get_config_store(&state) {
        Some(s) => s,
        None => return sysdb_error(),
    };
    match store.restore_snapshot(&id) {
        Ok(count) => Json(APIResponse::success(serde_json::json!({
            "restored": count,
            "snapshot_id": id,
        }))),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to restore snapshot: {}",
            e
        ))),
    }
}

/// Delete a configuration snapshot by ID.
async fn delete_config_snapshot(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<APIResponse<()>> {
    let store = match get_config_store(&state) {
        Some(s) => s,
        None => return sysdb_error(),
    };
    match store.delete_snapshot(&id) {
        Ok(()) => Json(APIResponse::success(())),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to delete snapshot: {}",
            e
        ))),
    }
}

// ── Table Explorer Handlers (v1.3.2-alpha) ───────────────────

/// List the supported storage engine types for the table explorer.
async fn explorer_storage_types(
    State(_state): State<Arc<AppState>>,
) -> Json<APIResponse<Vec<String>>> {
    let types = vec![
        "relational".to_string(),
        "document".to_string(),
        "vector".to_string(),
        "columnar".to_string(),
        "keyvalue".to_string(),
        "timeseries".to_string(),
    ];
    Json(APIResponse::success(types))
}

/// List tables for a storage type (query param `storage_type` required).
async fn explorer_tables(
    State(state): State<Arc<AppState>>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
) -> Json<APIResponse<serde_json::Value>> {
    let storage_type_str = match params.get("storage_type") {
        Some(s) => s,
        None => {
            return Json(APIResponse::error(
                "Missing 'storage_type' query parameter".to_string(),
            ))
        }
    };
    let st = match parse_storage_type(storage_type_str) {
        Ok(s) => s,
        Err(_) => {
            return Json(APIResponse::error(format!(
                "Invalid storage type: {}",
                storage_type_str
            )))
        }
    };

    let db = &state.primusdb;
    let query = Query {
        storage_type: st,
        operation: QueryOperation::InformationSchemaTables,
        table: String::new(),
        conditions: None,
        data: None,
        limit: None,
        offset: None,
        namespace: None,
    };
    match db.execute_query(query).await {
        Ok(result) => {
            let json_result = serde_json::to_value(&result).unwrap_or_default();
            let tables_data: Vec<serde_json::Value> = json_result
                .get("rows")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|row| {
                            row.get("columns")
                                .cloned()
                                .unwrap_or(serde_json::Value::Null)
                        })
                        .collect()
                })
                .unwrap_or_default();
            Json(APIResponse::success(serde_json::json!({
                "storage_type": storage_type_str,
                "tables": tables_data
            })))
        }
        Err(e) => Json(APIResponse::error(format!("Failed to list tables: {}", e))),
    }
}

/// Return table metadata and schema for the table explorer.
async fn explorer_table_info(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let st = parse_storage_type(&storage_type)?;

    let info = state.primusdb.table_info(st, &table).await.map_err(|e| {
        eprintln!("table_info error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let schema_json = serde_json::to_value(&info.schema).unwrap_or_default();

    Ok(Json(APIResponse::success(serde_json::json!({
        "name": info.name,
        "storage_type": storage_type,
        "row_count": info.row_count,
        "size_bytes": info.size_bytes,
        "created_at": info.created_at.to_rfc3339(),
        "updated_at": info.updated_at.to_rfc3339(),
        "schema": schema_json,
    }))))
}

#[derive(Deserialize)]
struct ExplorerRowsRequest {
    limit: Option<u64>,
    offset: Option<u64>,
    filter: Option<serde_json::Value>,
}

/// Read table rows with pagination and an optional filter for the explorer.
async fn explorer_table_rows(
    State(state): State<Arc<AppState>>,
    Path((storage_type, table)): Path<(String, String)>,
    Json(body): Json<ExplorerRowsRequest>,
) -> Result<Json<APIResponse<serde_json::Value>>, StatusCode> {
    let st = parse_storage_type(&storage_type)?;

    let query = Query {
        storage_type: st,
        operation: QueryOperation::Read,
        table: table.clone(),
        conditions: body.filter,
        data: None,
        limit: body.limit.or(Some(50)),
        offset: body.offset.or(Some(0)),
        namespace: None,
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => {
            let json_result = serde_json::to_value(&result).unwrap_or_default();
            let rows: Vec<serde_json::Value> = json_result
                .get("rows")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|row| {
                            row.get("columns")
                                .cloned()
                                .unwrap_or(serde_json::Value::Null)
                        })
                        .collect()
                })
                .unwrap_or_default();
            let total = json_result
                .get("affected_rows")
                .and_then(|v| v.as_u64())
                .unwrap_or(rows.len() as u64);
            Ok(Json(APIResponse::success(serde_json::json!({
                "rows": rows,
                "total": total,
                "limit": body.limit.unwrap_or(50),
                "offset": body.offset.unwrap_or(0),
            }))))
        }
        Err(e) => Ok(Json(APIResponse::error(format!(
            "Failed to read rows: {}",
            e
        )))),
    }
}

// ── Notebook Handler Types (v1.3.2-alpha) ────────────────────

#[derive(Deserialize)]
struct CreateNotebookRequest {
    name: String,
}

#[derive(Deserialize)]
struct UpdateNotebookRequest {
    cells: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct ExecuteNotebookCellRequest {
    cell_index: usize,
}

#[derive(Serialize)]
struct NotebookResponse {
    id: String,
    name: String,
    cells: Vec<serde_json::Value>,
    created_at: String,
    updated_at: String,
}

impl NotebookResponse {
    fn from_entry(entry: &crate::system::catalog::CatalogEntry) -> Self {
        let v = &entry.value;
        let cells = v
            .get("cells")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        Self {
            id: entry.key.trim_start_matches("notebook.").to_string(),
            name: v
                .get("name")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            cells,
            created_at: entry.updated_at.to_rfc3339(),
            updated_at: entry.updated_at.to_rfc3339(),
        }
    }
}

async fn list_notebooks(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<Vec<NotebookResponse>>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    match catalog.list_by_category("notebooks") {
        Ok(entries) => {
            let notebooks: Vec<NotebookResponse> =
                entries.iter().map(NotebookResponse::from_entry).collect();
            Json(APIResponse::success(notebooks))
        }
        Err(e) => Json(APIResponse::error(format!(
            "Failed to list notebooks: {}",
            e
        ))),
    }
}

async fn create_notebook_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateNotebookRequest>,
) -> Json<APIResponse<NotebookResponse>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    let key = format!("notebook.{}", req.name);
    let value = serde_json::json!({
        "name": req.name,
        "cells": [],
        "created_at": chrono::Utc::now().to_rfc3339(),
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });
    match catalog.set(&key, value, "notebooks") {
        Ok(()) => {
            if let Ok(Some(entry)) = catalog.get(&key) {
                Json(APIResponse::success(NotebookResponse::from_entry(&entry)))
            } else {
                Json(APIResponse::error(
                    "Notebook created but not found".to_string(),
                ))
            }
        }
        Err(e) => Json(APIResponse::error(format!(
            "Failed to create notebook: {}",
            e
        ))),
    }
}

async fn get_notebook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<APIResponse<NotebookResponse>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    let key = format!("notebook.{}", id);
    match catalog.get(&key) {
        Ok(Some(entry)) => Json(APIResponse::success(NotebookResponse::from_entry(&entry))),
        Ok(None) => Json(APIResponse::error(format!("Notebook '{}' not found", id))),
        Err(e) => Json(APIResponse::error(format!("Failed to get notebook: {}", e))),
    }
}

async fn update_notebook_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateNotebookRequest>,
) -> Json<APIResponse<NotebookResponse>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    let key = format!("notebook.{}", id);
    let existing = match catalog.get(&key) {
        Ok(Some(e)) => e,
        Ok(None) => return Json(APIResponse::error(format!("Notebook '{}' not found", id))),
        Err(e) => return Json(APIResponse::error(format!("Failed to get notebook: {}", e))),
    };
    let mut value = existing.value.clone();
    value["cells"] = serde_json::Value::Array(req.cells);
    value["updated_at"] = serde_json::Value::String(chrono::Utc::now().to_rfc3339());
    match catalog.set(&key, value, "notebooks") {
        Ok(()) => {
            if let Ok(Some(entry)) = catalog.get(&key) {
                Json(APIResponse::success(NotebookResponse::from_entry(&entry)))
            } else {
                Json(APIResponse::error(
                    "Notebook updated but not found".to_string(),
                ))
            }
        }
        Err(e) => Json(APIResponse::error(format!(
            "Failed to update notebook: {}",
            e
        ))),
    }
}

async fn delete_notebook_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<APIResponse<()>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    let key = format!("notebook.{}", id);
    match catalog.delete(&key) {
        Ok(()) => Json(APIResponse::success(())),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to delete notebook: {}",
            e
        ))),
    }
}

async fn execute_notebook_cell_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ExecuteNotebookCellRequest>,
) -> Json<APIResponse<serde_json::Value>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    let key = format!("notebook.{}", id);
    let entry = match catalog.get(&key) {
        Ok(Some(e)) => e,
        Ok(None) => return Json(APIResponse::error(format!("Notebook '{}' not found", id))),
        Err(e) => return Json(APIResponse::error(format!("Failed to get notebook: {}", e))),
    };
    let cells = entry.value.get("cells").and_then(|c| c.as_array());
    let cell = match cells.and_then(|arr| arr.get(body.cell_index)) {
        Some(c) => c,
        None => {
            return Json(APIResponse::error(format!(
                "Cell index {} out of range",
                body.cell_index
            )))
        }
    };
    let cell_type = cell.get("type").and_then(|s| s.as_str()).unwrap_or("md");
    let content = cell.get("content").and_then(|s| s.as_str()).unwrap_or("");

    if content.is_empty() {
        return Json(APIResponse::success(serde_json::json!({
            "result": "empty",
            "message": "Cell is empty — nothing to execute."
        })));
    }

    match cell_type {
        "sql" => {
            let query = Query {
                storage_type: StorageType::Relational,
                operation: QueryOperation::Read,
                table: content.to_string(),
                conditions: None,
                data: None,
                limit: Some(50),
                offset: None,
                namespace: None,
            };
            match state.primusdb.execute_query(query).await {
                Ok(result) => {
                    let json = serde_json::to_value(&result).unwrap_or_default();
                    Json(APIResponse::success(serde_json::json!({
                        "cell_type": "sql",
                        "result": json
                    })))
                }
                Err(e) => Json(APIResponse::error(format!("SQL execution failed: {}", e))),
            }
        }
        "analysis" => {
            let parts: Vec<&str> = content.splitn(2, '.').collect();
            let table = parts.first().unwrap_or(&content);
            let query = Query {
                storage_type: StorageType::Relational,
                operation: QueryOperation::Analyze,
                table: table.to_string(),
                conditions: None,
                data: None,
                limit: Some(50),
                offset: None,
                namespace: None,
            };
            match state.primusdb.execute_query(query).await {
                Ok(result) => {
                    let json = serde_json::to_value(&result).unwrap_or_default();
                    Json(APIResponse::success(serde_json::json!({
                        "cell_type": "analysis",
                        "result": json
                    })))
                }
                Err(e) => Json(APIResponse::error(format!("Analysis failed: {}", e))),
            }
        }
        "rag" => {
            let parts: Vec<&str> = content.splitn(2, '|').collect();
            let table = parts.first().unwrap_or(&content);
            let query_text = parts.get(1).unwrap_or(&"");
            let query = Query {
                storage_type: StorageType::Vector,
                operation: QueryOperation::Read,
                table: table.to_string(),
                conditions: if query_text.is_empty() {
                    None
                } else {
                    Some(serde_json::json!({
                        "query_text": query_text
                    }))
                },
                data: None,
                limit: Some(10),
                offset: None,
                namespace: None,
            };
            match state.primusdb.execute_query(query).await {
                Ok(result) => {
                    let json = serde_json::to_value(&result).unwrap_or_default();
                    Json(APIResponse::success(serde_json::json!({
                        "cell_type": "rag",
                        "result": json
                    })))
                }
                Err(e) => Json(APIResponse::error(format!("RAG search failed: {}", e))),
            }
        }
        _ => Json(APIResponse::success(serde_json::json!({
            "result": "rendered",
            "cell_type": cell_type,
            "message": "Markdown cell \u{2014} no execution needed."
        }))),
    }
}

// ── RAG Workspace Handlers (v1.3.2-alpha) ────────────────────

#[derive(Deserialize)]
struct RagSearchRequest {
    collection: String,
    query_text: String,
    limit: Option<usize>,
}

async fn rag_search_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RagSearchRequest>,
) -> Json<APIResponse<serde_json::Value>> {
    let query = Query {
        storage_type: StorageType::Vector,
        operation: QueryOperation::Read,
        table: req.collection.clone(),
        conditions: Some(serde_json::json!({
            "query_text": req.query_text,
        })),
        data: None,
        limit: req.limit.map(|l| l as u64),
        offset: None,
        namespace: None,
    };
    match state.primusdb.execute_query(query).await {
        Ok(result) => {
            let json = serde_json::to_value(&result).unwrap_or_default();
            Json(APIResponse::success(json))
        }
        Err(e) => Json(APIResponse::error(format!("RAG search failed: {}", e))),
    }
}

// ── Report Builder Handlers (v1.3.2-alpha) ───────────────────

fn get_system_catalog(state: &AppState) -> Option<&crate::system::catalog::SystemCatalog> {
    state.primusdb.system_db().map(|sys| &sys.catalog)
}

#[derive(Deserialize)]
struct CreateReportRequest {
    name: String,
    query: String,
    description: Option<String>,
    storage_type: Option<String>,
    format: Option<String>,
    table_name: Option<String>,
}

#[derive(Serialize)]
struct ReportResponse {
    id: String,
    name: String,
    query: String,
    description: String,
    storage_type: String,
    format: String,
    table_name: String,
    created_at: String,
    updated_at: String,
}

impl ReportResponse {
    fn from_entry(entry: &crate::system::catalog::CatalogEntry) -> Self {
        let v = &entry.value;
        Self {
            id: entry.key.trim_start_matches("report.").to_string(),
            name: v
                .get("name")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            query: v
                .get("query")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            description: v
                .get("description")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            storage_type: v
                .get("storage_type")
                .and_then(|s| s.as_str())
                .unwrap_or("relational")
                .to_string(),
            format: v
                .get("format")
                .and_then(|s| s.as_str())
                .unwrap_or("json")
                .to_string(),
            table_name: v
                .get("table_name")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            created_at: entry.updated_at.to_rfc3339(),
            updated_at: entry.updated_at.to_rfc3339(),
        }
    }
}

async fn list_reports(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<Vec<ReportResponse>>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    match catalog.list_by_category("reports") {
        Ok(entries) => {
            let reports: Vec<ReportResponse> =
                entries.iter().map(ReportResponse::from_entry).collect();
            Json(APIResponse::success(reports))
        }
        Err(e) => Json(APIResponse::error(format!("Failed to list reports: {}", e))),
    }
}

async fn create_report(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateReportRequest>,
) -> Json<APIResponse<ReportResponse>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    let id = format!("report.{}", req.name);
    let value = serde_json::json!({
        "name": req.name,
        "query": req.query,
        "description": req.description.unwrap_or_default(),
        "storage_type": req.storage_type.unwrap_or_else(|| "relational".to_string()),
        "format": req.format.unwrap_or_else(|| "json".to_string()),
        "table_name": req.table_name.unwrap_or_default(),
    });
    match catalog.set(&id, value, "reports") {
        Ok(()) => {
            if let Ok(Some(entry)) = catalog.get(&id) {
                Json(APIResponse::success(ReportResponse::from_entry(&entry)))
            } else {
                Json(APIResponse::error(
                    "Report created but not found".to_string(),
                ))
            }
        }
        Err(e) => Json(APIResponse::error(format!(
            "Failed to create report: {}",
            e
        ))),
    }
}

async fn get_report(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<APIResponse<ReportResponse>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    let key = format!("report.{}", id);
    match catalog.get(&key) {
        Ok(Some(entry)) => Json(APIResponse::success(ReportResponse::from_entry(&entry))),
        Ok(None) => Json(APIResponse::error(format!("Report '{}' not found", id))),
        Err(e) => Json(APIResponse::error(format!("Failed to get report: {}", e))),
    }
}

async fn update_report(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateReportRequest>,
) -> Json<APIResponse<ReportResponse>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    let key = format!("report.{}", id);
    let value = serde_json::json!({
        "name": req.name,
        "query": req.query,
        "description": req.description.unwrap_or_default(),
        "storage_type": req.storage_type.unwrap_or_else(|| "relational".to_string()),
        "format": req.format.unwrap_or_else(|| "json".to_string()),
        "table_name": req.table_name.unwrap_or_default(),
    });
    match catalog.set(&key, value, "reports") {
        Ok(()) => {
            if let Ok(Some(entry)) = catalog.get(&key) {
                Json(APIResponse::success(ReportResponse::from_entry(&entry)))
            } else {
                Json(APIResponse::error(
                    "Report updated but not found".to_string(),
                ))
            }
        }
        Err(e) => Json(APIResponse::error(format!(
            "Failed to update report: {}",
            e
        ))),
    }
}

async fn delete_report(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<APIResponse<()>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    let key = format!("report.{}", id);
    match catalog.delete(&key) {
        Ok(()) => Json(APIResponse::success(())),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to delete report: {}",
            e
        ))),
    }
}

#[derive(Deserialize)]
struct ExecuteReportRequest {
    limit: Option<u64>,
    offset: Option<u64>,
}

async fn execute_report_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ExecuteReportRequest>,
) -> Json<APIResponse<serde_json::Value>> {
    let catalog = match get_system_catalog(&state) {
        Some(c) => c,
        None => return sysdb_error(),
    };
    let key = format!("report.{}", id);
    let entry = match catalog.get(&key) {
        Ok(Some(e)) => e,
        Ok(None) => return Json(APIResponse::error(format!("Report '{}' not found", id))),
        Err(e) => return Json(APIResponse::error(format!("Failed to get report: {}", e))),
    };

    let storage_type_str = entry
        .value
        .get("storage_type")
        .and_then(|s| s.as_str())
        .unwrap_or("relational");
    let table = entry
        .value
        .get("table_name")
        .and_then(|s| s.as_str())
        .unwrap_or("");

    let st = match parse_storage_type(storage_type_str) {
        Ok(s) => s,
        Err(_) => {
            return Json(APIResponse::error(format!(
                "Invalid storage type: {}",
                storage_type_str
            )))
        }
    };

    if table.is_empty() {
        return Json(APIResponse::error(
            "Report has no table_name set. Edit the report to specify a table.".to_string(),
        ));
    }

    let query = Query {
        storage_type: st,
        operation: QueryOperation::Read,
        table: table.to_string(),
        conditions: None,
        data: None,
        limit: body.limit,
        offset: body.offset,
        namespace: None,
    };

    match state.primusdb.execute_query(query).await {
        Ok(result) => {
            let json_result = serde_json::to_value(&result).unwrap_or_default();
            Json(APIResponse::success(json_result))
        }
        Err(e) => Json(APIResponse::error(format!(
            "Report execution failed: {}",
            e
        ))),
    }
}

// ── System Database Handlers (v1.3.2-alpha) ────────────────────────────

/// Export the entire system database as a JSON bundle.
/// Includes config entries, catalog entries, audit events, and server info.
async fn system_db_export_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    let sys_db = match state.primusdb.system_db() {
        Some(s) => s,
        None => return sysdb_error(),
    };
    match sys_db.export_system_bundle() {
        Ok(bundle) => Json(APIResponse::success(bundle)),
        Err(e) => Json(APIResponse::error(format!("Export failed: {}", e))),
    }
}

/// Import a system database configuration bundle.
/// Accepts a JSON body with config entries and catalog entries to merge.
async fn system_db_import_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<APIResponse<serde_json::Value>> {
    let sys_db = match state.primusdb.system_db() {
        Some(s) => s,
        None => return sysdb_error(),
    };

    // Import config entries if present
    let config_imported =
        if let Some(entries) = body.get("config_entries").and_then(|v| v.as_array()) {
            let bundle = crate::system::config_store::ConfigBundle {
                format_version: 1,
                exported_at: chrono::Utc::now(),
                entries: entries
                    .iter()
                    .filter_map(|e| serde_json::from_value(e.clone()).ok())
                    .collect(),
            };
            sys_db.config.import_bundle(&bundle).unwrap_or_default()
        } else {
            0
        };

    Json(APIResponse::success(serde_json::json!({
        "config_entries_imported": config_imported,
        "message": "System database import complete",
    })))
}

// ---------------------------------------------------------------------------
// Backup Management Handlers
// ---------------------------------------------------------------------------

/// Create a full backup
async fn backup_create_full_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    let data_dir = &state.primusdb.config().storage.data_dir;
    match state.primusdb.create_full_backup(data_dir) {
        Ok(manifest) => Json(APIResponse::success(serde_json::json!({
            "backup_id": manifest.id,
            "backup_type": manifest.backup_type,
            "timestamp": manifest.timestamp,
            "status": "created",
        }))),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to create backup: {}",
            e
        ))),
    }
}

/// Create an incremental backup
async fn backup_create_incremental_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    let data_dir = &state.primusdb.config().storage.data_dir;
    match state.primusdb.create_incremental_backup(data_dir) {
        Ok(manifest) => Json(APIResponse::success(serde_json::json!({
            "backup_id": manifest.id,
            "backup_type": manifest.backup_type,
            "timestamp": manifest.timestamp,
            "parent_id": manifest.parent_id,
            "status": "created",
        }))),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to create backup: {}",
            e
        ))),
    }
}

/// List all backups
async fn backup_list_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    match state.primusdb.list_backups() {
        Ok(backups) => {
            let backup_list: Vec<serde_json::Value> = backups
                .iter()
                .map(|b| {
                    serde_json::json!({
                        "id": b.id,
                        "backup_type": b.backup_type,
                        "timestamp": b.timestamp,
                        "parent_id": b.parent_id,
                        "size_bytes": b.size_bytes,
                    })
                })
                .collect();
            Json(APIResponse::success(serde_json::json!({
                "backups": backup_list,
                "count": backups.len(),
            })))
        }
        Err(e) => Json(APIResponse::error(format!("Failed to list backups: {}", e))),
    }
}

/// Get backup status
async fn backup_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    match state.primusdb.backup_status() {
        Ok(status) => Json(APIResponse::success(serde_json::json!({
            "last_full_backup": status.last_full_backup,
            "last_incremental_backup": status.last_incremental_backup,
            "total_backups": status.total_backups,
            "schedule_enabled": status.schedule_enabled,
        }))),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to get backup status: {}",
            e
        ))),
    }
}

/// Start backup scheduler
async fn backup_schedule_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<APIResponse<serde_json::Value>> {
    let full_interval = body
        .get("full_backup_interval_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(86400);
    let incremental_interval = body
        .get("incremental_interval_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);

    let schedule_config = crate::backup::BackupScheduleConfig {
        full_backup_interval_secs: full_interval,
        incremental_interval_secs: incremental_interval,
        enabled: true,
    };

    state.primusdb.shutdown_backup_scheduler();

    let data_dir = state.primusdb.config().storage.data_dir.clone();
    let backup_manager = state.primusdb.backup_manager().clone();
    let scheduler =
        crate::backup::scheduler::BackupScheduler::new(schedule_config, backup_manager, data_dir);
    state
        .primusdb
        .set_backup_scheduler(Some(Arc::new(scheduler)));

    Json(APIResponse::success(serde_json::json!({
        "status": "started",
        "full_backup_interval_secs": full_interval,
        "incremental_interval_secs": incremental_interval,
        "message": "Backup scheduler started successfully",
    })))
}

/// Stop backup scheduler
async fn backup_stop_scheduler_handler(
    State(state): State<Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    state.primusdb.shutdown_backup_scheduler();
    Json(APIResponse::success(serde_json::json!({
        "status": "stopped",
        "message": "Backup scheduler stopped",
    })))
}

/// Restore from a backup
async fn backup_restore_handler(
    State(state): State<Arc<AppState>>,
    Path(backup_id): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    let target_dir = &state.primusdb.config().storage.data_dir;
    match state.primusdb.restore_from_backup(&backup_id, target_dir) {
        Ok(result) => Json(APIResponse::success(serde_json::json!({
            "status": "restored",
            "backup_id": result.backup_id,
            "restore_dir": result.restore_dir,
            "restored_engines": result.restored_engines,
            "backup_type": format!("{}", result.backup_type),
            "backup_timestamp": result.backup_timestamp,
        }))),
        Err(e) => Json(APIResponse::error(format!(
            "Failed to restore backup: {}",
            e
        ))),
    }
}

// ── Time Series Handlers (v1.3.2-alpha) ──────────────────────────────────

use crate::storage::timeseries;

async fn get_ts_engine(state: &AppState) -> Result<Arc<timeseries::TimeSeriesEngine>, StatusCode> {
    state
        .primusdb
        .storage_engine(StorageType::TimeSeries)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
        .map(|e| {
            e.as_any()
                .downcast_ref::<timeseries::TimeSeriesEngine>()
                .unwrap()
                .clone()
        })
        .map(Arc::new)
}

async fn ts_list_metrics(State(state): State<Arc<AppState>>) -> Json<APIResponse<Vec<String>>> {
    match get_ts_engine(&state).await {
        Ok(engine) => match engine.list_metrics() {
            Ok(metrics) => Json(APIResponse::success(metrics)),
            Err(e) => Json(APIResponse::error(e.to_string())),
        },
        Err(_) => Json(APIResponse::error(
            "TimeSeries engine not available".to_string(),
        )),
    }
}

async fn ts_describe_metric(
    State(state): State<Arc<AppState>>,
    Path(metric): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    match get_ts_engine(&state).await {
        Ok(engine) => match engine.describe_metric(&metric) {
            Ok(Some(m)) => Json(APIResponse::success(
                serde_json::to_value(m).unwrap_or_default(),
            )),
            Ok(None) => Json(APIResponse::error(format!("Metric '{}' not found", metric))),
            Err(e) => Json(APIResponse::error(e.to_string())),
        },
        Err(_) => Json(APIResponse::error(
            "TimeSeries engine not available".to_string(),
        )),
    }
}

async fn ts_query(
    State(state): State<Arc<AppState>>,
    Path(metric): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<APIResponse<serde_json::Value>> {
    let engine = match get_ts_engine(&state).await {
        Ok(e) => e,
        Err(_) => {
            return Json(APIResponse::error(
                "TimeSeries engine not available".to_string(),
            ))
        }
    };

    let start_time = body.get("start_time").and_then(|v| v.as_i64());
    let end_time = body.get("end_time").and_then(|v| v.as_i64());
    let tags: Option<std::collections::HashMap<String, String>> = body
        .get("tags")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let fields: Option<Vec<String>> = body
        .get("fields")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let limit = body.get("limit").and_then(|v| v.as_u64()).unwrap_or(1000);
    let resolution = body
        .get("resolution")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let query = timeseries::TimeSeriesQuery {
        metric: metric.clone(),
        start_time,
        end_time,
        tags,
        fields,
        aggregation: None,
        resolution,
        fill_policy: None,
        limit: Some(limit),
        offset: None,
        group_by: None,
    };

    match engine.query_points(&query) {
        Ok(points) => {
            let json_points: Vec<serde_json::Value> = points
                .into_iter()
                .map(|p| {
                    serde_json::json!({
                        "timestamp": p.timestamp,
                        "tags": p.tags,
                        "fields": p.fields,
                    })
                })
                .collect();
            Json(APIResponse::success(serde_json::json!({
                "metric": metric,
                "points": json_points,
                "count": json_points.len(),
            })))
        }
        Err(e) => Json(APIResponse::error(e.to_string())),
    }
}

async fn ts_aggregate(
    State(state): State<Arc<AppState>>,
    Path(metric): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<APIResponse<serde_json::Value>> {
    let engine = match get_ts_engine(&state).await {
        Ok(e) => e,
        Err(_) => {
            return Json(APIResponse::error(
                "TimeSeries engine not available".to_string(),
            ))
        }
    };

    let start_time = body.get("start_time").and_then(|v| v.as_i64());
    let end_time = body.get("end_time").and_then(|v| v.as_i64());
    let tags: Option<std::collections::HashMap<String, String>> = body
        .get("tags")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let agg_fn = body
        .get("aggregation")
        .and_then(|v| v.as_str())
        .unwrap_or("avg");
    let resolution = body
        .get("resolution")
        .and_then(|v| v.as_str())
        .unwrap_or("1h");
    let fill_policy = body.get("fill_policy").and_then(|v| v.as_str());
    let fields: Option<Vec<String>> = body
        .get("fields")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let query = timeseries::TimeSeriesQuery {
        metric: metric.clone(),
        start_time,
        end_time,
        tags,
        fields,
        aggregation: Some(agg_fn.to_string()),
        resolution: Some(resolution.to_string()),
        fill_policy: fill_policy.map(timeseries::FillPolicy::parse_from),
        limit: Some(10000),
        offset: None,
        group_by: None,
    };

    match engine.aggregate(&query, agg_fn) {
        Ok(results) => {
            let json_results: Vec<serde_json::Value> = results
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "timestamp": r.timestamp,
                        "value": r.value,
                        "count": r.count,
                    })
                })
                .collect();
            Json(APIResponse::success(serde_json::json!({
                "metric": metric,
                "aggregation": agg_fn,
                "resolution": resolution,
                "results": json_results,
                "count": json_results.len(),
            })))
        }
        Err(e) => Json(APIResponse::error(e.to_string())),
    }
}

async fn ts_downsample(
    State(state): State<Arc<AppState>>,
    Path(metric): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<APIResponse<serde_json::Value>> {
    let engine = match get_ts_engine(&state).await {
        Ok(e) => e,
        Err(_) => {
            return Json(APIResponse::error(
                "TimeSeries engine not available".to_string(),
            ))
        }
    };

    let source = body
        .get("source_resolution")
        .and_then(|v| v.as_str())
        .unwrap_or("raw");
    let target = body
        .get("target_resolution")
        .and_then(|v| v.as_str())
        .unwrap_or("1h");
    let agg_fn = body.get("agg_fn").and_then(|v| v.as_str()).unwrap_or("avg");

    match engine.downsample(&metric, source, target, agg_fn) {
        Ok(processed) => Json(APIResponse::success(serde_json::json!({
            "metric": metric,
            "source_resolution": source,
            "target_resolution": target,
            "agg_fn": agg_fn,
            "points_created": processed,
        }))),
        Err(e) => Json(APIResponse::error(e.to_string())),
    }
}

async fn ts_retain(
    State(state): State<Arc<AppState>>,
    Path(metric): Path<String>,
) -> Json<APIResponse<serde_json::Value>> {
    let engine = match get_ts_engine(&state).await {
        Ok(e) => e,
        Err(_) => {
            return Json(APIResponse::error(
                "TimeSeries engine not available".to_string(),
            ))
        }
    };

    match engine.apply_retention(&metric) {
        Ok(removed) => Json(APIResponse::success(serde_json::json!({
            "metric": metric,
            "chunks_removed": removed,
        }))),
        Err(e) => Json(APIResponse::error(e.to_string())),
    }
}

async fn ts_resolution(
    State(state): State<Arc<AppState>>,
    Path(metric): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<APIResponse<serde_json::Value>> {
    let engine = match get_ts_engine(&state).await {
        Ok(e) => e,
        Err(_) => {
            return Json(APIResponse::error(
                "TimeSeries engine not available".to_string(),
            ))
        }
    };

    let resolution = body
        .get("resolution")
        .and_then(|v| v.as_str())
        .unwrap_or("1h");
    let retention_days = body
        .get("retention_days")
        .and_then(|v| v.as_u64())
        .unwrap_or(365) as u32;
    let agg_fn = body.get("agg_fn").and_then(|v| v.as_str()).unwrap_or("avg");

    match engine.add_resolution(&metric, resolution, retention_days, agg_fn) {
        Ok(()) => Json(APIResponse::success(serde_json::json!({
            "metric": metric,
            "resolution": resolution,
            "retention_days": retention_days,
            "agg_fn": agg_fn,
            "status": "added",
        }))),
        Err(e) => Json(APIResponse::error(e.to_string())),
    }
}

async fn ts_stats(State(state): State<Arc<AppState>>) -> Json<APIResponse<serde_json::Value>> {
    match get_ts_engine(&state).await {
        Ok(engine) => match engine.engine_stats() {
            Ok(stats) => Json(APIResponse::success(stats)),
            Err(e) => Json(APIResponse::error(e.to_string())),
        },
        Err(_) => Json(APIResponse::error(
            "TimeSeries engine not available".to_string(),
        )),
    }
}

// Helper functions
fn parse_storage_type(storage_type: &str) -> Result<StorageType, StatusCode> {
    match storage_type.to_lowercase().as_str() {
        "columnar" => Ok(StorageType::Columnar),
        "vector" => Ok(StorageType::Vector),
        "document" => Ok(StorageType::Document),
        "relational" => Ok(StorageType::Relational),
        "keyvalue" | "kv" => Ok(StorageType::KeyValue),
        "timeseries" | "ts" => Ok(StorageType::TimeSeries),
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
