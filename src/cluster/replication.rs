use crate::Result;
use crate::cluster::rpc::{
    ReplicaReadRequest, ReplicaReadResponse, ReplicaWriteAck, ReplicaWriteRequest,
    RpcClient, RpcMessage, ShardTransferChunk, ShardTransferRequest,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ReplicationMode {
    Sync,
    Async,
    Quorum,
}

impl Default for ReplicationMode {
    fn default() -> Self {
        Self::Quorum
    }
}

#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    pub mode: ReplicationMode,
    pub replication_factor: usize,
    pub write_timeout_ms: u64,
    pub read_timeout_ms: u64,
    pub max_batch_size: usize,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            mode: ReplicationMode::Quorum,
            replication_factor: 3,
            write_timeout_ms: 5000,
            read_timeout_ms: 5000,
            max_batch_size: 100,
        }
    }
}

#[derive(Debug)]
pub struct ReplicationEngine {
    pub node_id: String,
    pub config: ReplicationConfig,
    pub clients: Arc<RwLock<HashMap<String, Arc<RpcClient>>>>,
    pub pending_acks: Arc<RwLock<HashMap<String, Vec<ReplicaWriteAck>>>>,
}

impl ReplicationEngine {
    pub fn new(
        node_id: String,
        config: ReplicationConfig,
        clients: Arc<RwLock<HashMap<String, Arc<RpcClient>>>>,
    ) -> Self {
        Self {
            node_id,
            config,
            clients,
            pending_acks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn replicate_write(
        &self,
        operation_id: &str,
        storage_type: &str,
        table: &str,
        key: &str,
        data: &serde_json::Value,
        term: u64,
        index: u64,
        target_nodes: &[String],
    ) -> Result<ReplicationResult> {
        let clients = self.clients.read().await;
        let mut successes = 0;
        let mut failures = Vec::new();

        for node_id in target_nodes {
            if *node_id == self.node_id {
                successes += 1;
                continue;
            }

            if let Some(client) = clients.get(node_id) {
                let req = RpcMessage::ReplicaWrite(ReplicaWriteRequest {
                    operation_id: operation_id.to_string(),
                    storage_type: storage_type.to_string(),
                    table: table.to_string(),
                    key: key.to_string(),
                    data: data.clone(),
                    term,
                    index,
                });

                match client.send(&req).await {
                    Ok(RpcMessage::ReplicaWriteAck(ack)) => {
                        if ack.success {
                            successes += 1;
                        } else {
                            failures.push(node_id.clone());
                        }
                    }
                    Ok(_) => {
                        failures.push(node_id.clone());
                    }
                    Err(_) => {
                        failures.push(node_id.clone());
                    }
                }
            } else {
                failures.push(node_id.clone());
            }
        }

        let total = target_nodes.len();
        let quorum = self.config.replication_factor / 2 + 1;

        let status = match self.config.mode {
            ReplicationMode::Sync => {
                if successes == total {
                    ReplicationStatus::Committed
                } else {
                    ReplicationStatus::Failed
                }
            }
            ReplicationMode::Quorum => {
                if successes >= quorum.min(total) {
                    ReplicationStatus::Committed
                } else {
                    ReplicationStatus::Failed
                }
            }
            ReplicationMode::Async => {
                if successes > 0 {
                    ReplicationStatus::Accepted
                } else {
                    ReplicationStatus::Failed
                }
            }
        };

        Ok(ReplicationResult {
            operation_id: operation_id.to_string(),
            status,
            successes,
            failures,
            total_targets: total,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }

    pub async fn replicate_read(
        &self,
        storage_type: &str,
        table: &str,
        key: &str,
        target_nodes: &[String],
    ) -> Result<ReplicaReadResponse> {
        let clients = self.clients.read().await;
        let mut responses = Vec::new();

        for node_id in target_nodes {
            if *node_id == self.node_id {
                continue;
            }
            if let Some(client) = clients.get(node_id) {
                let req = RpcMessage::ReplicaRead(ReplicaReadRequest {
                    storage_type: storage_type.to_string(),
                    table: table.to_string(),
                    key: key.to_string(),
                });
                if let Ok(RpcMessage::ReplicaReadResponse(resp)) = client.send(&req).await {
                    responses.push(resp);
                }
            }
        }

        if let Some(resp) = responses.into_iter().find(|r| r.found) {
            Ok(resp)
        } else {
            Ok(ReplicaReadResponse {
                found: false,
                data: None,
                node_id: self.node_id.clone(),
            })
        }
    }

    pub async fn migrate_shard(
        &self,
        shard_id: &str,
        table: &str,
        storage_type: &str,
        data_chunks: Vec<Vec<u8>>,
        target_node: &str,
    ) -> Result<()> {
        let clients = self.clients.read().await;
        let client = clients
            .get(target_node)
            .ok_or_else(|| crate::Error::ClusterError("Target node not connected".into()))?;

        let total_chunks = data_chunks.len() as u32;
        let req = RpcMessage::ShardTransfer(ShardTransferRequest {
            shard_id: shard_id.to_string(),
            table: table.to_string(),
            storage_type: storage_type.to_string(),
            total_chunks,
        });

        client.send(&req).await?;

        for (i, chunk) in data_chunks.iter().enumerate() {
            let chunk_msg = RpcMessage::ShardTransferChunk(ShardTransferChunk {
                shard_id: shard_id.to_string(),
                chunk_index: i as u32,
                data: chunk.clone(),
                is_last: i == data_chunks.len() - 1,
            });
            client.send(&chunk_msg).await?;
        }

        info!(
            "Shard {} migrated to {} ({} chunks)",
            shard_id, target_node, total_chunks
        );
        Ok(())
    }

    pub async fn check_replication_health(&self) -> Vec<ReplicaHealthStatus> {
        let clients = self.clients.read().await;
        let mut statuses = Vec::new();

        for (node_id, client) in clients.iter() {
            let healthy = client.is_connected().await;
            statuses.push(ReplicaHealthStatus {
                node_id: node_id.clone(),
                healthy,
                last_check: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            });
        }
        statuses
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationResult {
    pub operation_id: String,
    pub status: ReplicationStatus,
    pub successes: usize,
    pub failures: Vec<String>,
    pub total_targets: usize,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ReplicationStatus {
    Committed,
    Accepted,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaHealthStatus {
    pub node_id: String,
    pub healthy: bool,
    pub last_check: u64,
}
