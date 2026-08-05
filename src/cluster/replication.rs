//! Replica write/read fan-out
//!
//! Copies committed writes to replica nodes so data survives node loss.
//! [`ReplicationEngine`] sends each operation to the target replica set and
//! classifies the outcome according to the configured [`ReplicationMode`]:
//! `Sync` (all targets must ack), `Quorum` (a majority must ack, the default)
//! or `Async` (fire-and-forget). It also streams shard migrations to new hosts
//! and reports per-replica health.
//!
//! # Placement in the architecture
//!
//! `ReplicationEngine` sits below the Raft commit point: once an operation is
//! ordered by [`crate::cluster::raft::RaftNode`], the engine durably fans it
//! out to the replica nodes chosen by the
//! [`crate::cluster::shard::ShardManager`].
//!
//! ```text
//!   committed write (Raft log)
//!             │
//!             ▼
//!   ┌─────────────────────┐
//!   │  ReplicationEngine  │  fans out ReplicaWrite to target_nodes
//!   └─────────────────────┘
//!             │
//!    ┌────────┼────────────┐
//!    ▼        ▼            ▼
//!  replica   replica     replica
//!    1        2            N
//!    │        │            │
//!    └────────┴─── ReplicaWriteAck ──┘
//!             │
//!             ▼
//!   mode decides status:
//!     Sync    = all acks        ──► Committed
//!     Quorum  = majority acks   ──► Committed (default)
//!     Async   = any success     ──► Accepted
//!
//!   migrate_shard streams chunks to a new owner during rebalancing.
//! ```

use crate::cluster::rpc::{
    ReplicaReadRequest, ReplicaReadResponse, ReplicaWriteAck, ReplicaWriteRequest, RpcClient,
    RpcMessage, ShardTransferChunk, ShardTransferRequest,
};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::info;

/// How aggressively writes are replicated to replicas.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum ReplicationMode {
    /// All targets must acknowledge before the write is committed
    Sync,
    /// Writes are acknowledged immediately and replicated in the background
    Async,
    /// A quorum (majority) of targets must acknowledge (default)
    #[default]
    Quorum,
}

/// Tuning parameters for the replication engine.
#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    /// Replication mode (sync / async / quorum)
    pub mode: ReplicationMode,
    /// Number of replicas each write is fanned out to
    pub replication_factor: usize,
    /// Timeout for replica writes (ms)
    pub write_timeout_ms: u64,
    /// Timeout for replica reads (ms)
    pub read_timeout_ms: u64,
    /// Maximum entries per batch operation
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

/// Fans committed writes out to replica nodes and reads them back on demand.
#[derive(Debug)]
pub struct ReplicationEngine {
    /// ID of the local node
    pub node_id: String,
    /// Replication tuning parameters
    pub config: ReplicationConfig,
    /// RPC clients to peer nodes, keyed by node ID
    pub clients: Arc<RwLock<HashMap<String, Arc<RpcClient>>>>,
    /// Pending replica acknowledgements keyed by operation ID
    pub pending_acks: Arc<RwLock<HashMap<String, Vec<ReplicaWriteAck>>>>,
}

impl ReplicationEngine {
    /// Create a replication engine sharing the given RPC client map.
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

    #[allow(clippy::too_many_arguments)]
    /// Send a write to every node in `target_nodes` and classify the outcome
    /// according to the configured replication mode.
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

    /// Read a key from the given nodes, returning the first positive response.
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

    /// Stream a shard's data chunks to a target node to migrate the shard.
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

    /// Report connection health for every replica client.
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

/// Outcome of a replicated write operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationResult {
    /// ID of the replicated operation
    pub operation_id: String,
    /// Final replication status
    pub status: ReplicationStatus,
    /// Number of successful replica acknowledgements
    pub successes: usize,
    /// Nodes that failed to acknowledge
    pub failures: Vec<String>,
    /// Total number of target nodes
    pub total_targets: usize,
    /// Timestamp (ms) of the replication attempt
    pub timestamp: u64,
}

/// Final state of a replicated write.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ReplicationStatus {
    /// Write reached the required number of replicas
    Committed,
    /// Write accepted but not yet durable on replicas
    Accepted,
    /// Write could not be replicated
    Failed,
}

/// Health of a single replica node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaHealthStatus {
    /// Node ID of the replica
    pub node_id: String,
    /// Whether the replica is currently reachable
    pub healthy: bool,
    /// Timestamp (ms) of the last health check
    pub last_check: u64,
}
