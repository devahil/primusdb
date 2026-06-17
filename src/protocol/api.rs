use axum::{extract::State, http::StatusCode, Json};
use std::sync::OnceLock;
use std::time::Instant;

use crate::api::{APIResponse, AppState};

static START_TIME: OnceLock<Instant> = OnceLock::new();

fn start_time() -> Instant {
    *START_TIME.get_or_init(Instant::now)
}

pub async fn protocol_health_handler(
    State(state): State<std::sync::Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    let uptime = start_time().elapsed().as_secs();
    let blockchain_height = state
        .primusdb
        .get_chain_state()
        .await
        .map(|s| s.current_height)
        .unwrap_or(0);
    let ai_models = state.primusdb.ai_engine.model_count() as u64;
    let peer_count = if let Some(ref gateway) = state.cluster_gateway {
        gateway.get_nodes().await.len() as u64
    } else {
        0
    };

    Json(APIResponse::success(serde_json::json!({
        "status": "healthy",
        "uptime_seconds": uptime,
        "protocol_version": "1.0",
        "connected_peers": peer_count,
        "blockchain_height": blockchain_height,
        "ai_models_loaded": ai_models,
    })))
}

pub async fn protocol_status_handler(
    State(state): State<std::sync::Arc<AppState>>,
) -> Json<APIResponse<serde_json::Value>> {
    let node_id = state.primusdb.config().cluster.node_id.clone();
    let messages_sent = crate::protocol::messaging::get_messages_sent();
    let messages_received = crate::protocol::messaging::get_messages_received();
    let errors = crate::protocol::messaging::get_protocol_errors_total();
    let peers = if let Some(ref gateway) = state.cluster_gateway {
        gateway
            .get_nodes()
            .await
            .into_iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.node_id,
                    "host": n.host,
                    "port": n.port,
                    "health": format!("{:?}", n.health),
                    "latency_ms": n.ewma_latency_ms,
                    "cpu": n.cpu_usage,
                    "memory": n.memory_usage,
                })
            })
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    Json(APIResponse::success(serde_json::json!({
        "node_id": node_id,
        "role": "validator",
        "peers": peers,
        "messages_sent": messages_sent,
        "messages_received": messages_received,
        "errors": errors,
    })))
}

pub async fn protocol_peers_handler(
    State(state): State<std::sync::Arc<AppState>>,
) -> Json<APIResponse<Vec<serde_json::Value>>> {
    let peers = if let Some(ref gateway) = state.cluster_gateway {
        gateway
            .get_nodes()
            .await
            .into_iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.node_id,
                    "host": n.host,
                    "port": n.port,
                    "health": format!("{:?}", n.health),
                    "active_connections": n.active_connections,
                    "latency_ms": n.ewma_latency_ms,
                    "shards": n.shards,
                })
            })
            .collect()
    } else {
        vec![]
    };
    Json(APIResponse::success(peers))
}

pub async fn protocol_metrics_handler(
    State(_state): State<std::sync::Arc<AppState>>,
) -> Result<String, StatusCode> {
    let metrics = crate::metrics::get_metrics().encode();
    Ok(metrics)
}
