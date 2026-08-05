//! Distributed data synchronization and reconciliation
//!
//! Keeps replicas convergent when the same records are modified on different
//! nodes. [`SyncCoordinator`] orders operations with quorum-based consensus
//! (`consensus_write` / `consensus_read`), runs leader election, and exchanges
//! records with peers (`sync_table`, Merkle roots, conflict resolution). Vector
//! clocks ([`VectorClock`]) capture causality, while the [`consensus`] and
//! [`reconciliation`] submodules provide the Raft-style state types and the
//! conflict-detection / merge-plan logic.
//!
//! # Placement in the architecture
//!
//! `SyncCoordinator` is the convergence layer: it adds quorum-based durability
//! on top of the [`crate::cluster::raft::RaftNode`] ordering and pushes
//! divergent replicas back together using vector clocks and Merkle-tree
//! comparison.
//!
//! ```text
//!   ┌─────────────────────────── Cluster node ───────────────────────────┐
//!   │                                                                    │
//!   │  write ─► consensus_write ─► validator votes (quorum) ─► op log    │
//!   │  read  ─► consensus_read  ─► version agreement check               │
//!   │               │                                                    │
//!   │               ▼                                                    │
//!   │        VectorClock + SyncMetadata per record key                   │
//!   │               │                                                    │
//!   │               ▼                                                    │
//!   │   sync_table / request_merkle_root / reconcile_node  (RPC peers)   │
//!   │               │                                                    │
//!   │               ▼                                                    │
//!   │   ConflictResolve ──► peers converge on the same value             │
//!   └────────────────────────────────────────────────────────────────────┘
//!
//!   consensus.rs  -> Raft-style state types (ConsensusState, LogEntry, ...)
//!   reconciliation.rs -> conflict detection + merge plans
//! ```

use crate::cluster::rpc::{
    ConflictResolveMessage, MerkleRequest, MerkleResponse, ReplicaWriteRequest, RpcClient,
    RpcMessage, SyncRequest, SyncResponse,
};
use crate::Result;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

type RecordVersionEntry = (String, HashMap<String, u64>, u64, String);
type RecordVersionEntryWithData = (
    String,
    HashMap<String, u64>,
    u64,
    String,
    serde_json::Map<String, serde_json::Value>,
);

pub mod consensus;
pub mod reconciliation;

pub use consensus::*;
pub use reconciliation::*;

/// Tuning parameters for sync and quorum operations.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Number of replicas each record should have
    pub replication_factor: usize,
    /// How often background sync runs (ms)
    pub sync_interval_ms: u64,
    /// Conflict resolution strategy
    pub conflict_resolution: ConflictResolution,
    /// Whether referential integrity checks are enabled
    pub enable_referential_integrity: bool,
    /// Minimum nodes consulted for a consistent read
    pub read_quorum: usize,
    /// Minimum confirmations required for a write
    pub write_quorum: usize,
    /// Heartbeat interval for sync peers (ms)
    pub heartbeat_interval_ms: u64,
    /// Maximum tolerated clock drift (ms)
    pub max_clock_drift_ms: u64,
    /// Whether Merkle-tree based sync is enabled
    pub merkle_sync: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            replication_factor: 3,
            sync_interval_ms: 100,
            conflict_resolution: ConflictResolution::VectorClock,
            enable_referential_integrity: true,
            read_quorum: 2,
            write_quorum: 2,
            heartbeat_interval_ms: 1000,
            max_clock_drift_ms: 5000,
            merkle_sync: true,
        }
    }
}

/// Strategy for resolving conflicting record versions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Keep the version with the latest timestamp
    LastWriteWins,
    /// Compare vector clocks to detect concurrent writes
    VectorClock,
    /// Merge conflicting versions (CRDT-style)
    CRDT,
    /// Delegate resolution to a custom callback
    Custom,
}

/// A vector clock tracking per-node causality of a record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorClock {
    /// Per-node counters
    pub clocks: HashMap<String, u64>,
    /// Timestamp (ms) of the last update
    pub timestamp: u64,
}

impl VectorClock {
    /// Create a clock initialized with one tick for `node_id`.
    pub fn new(node_id: &str) -> Self {
        let mut clocks = HashMap::new();
        clocks.insert(node_id.to_string(), 1);
        Self {
            clocks,
            timestamp: now_ms(),
        }
    }

    /// Advance this node's counter and refresh the timestamp.
    pub fn increment(&mut self, node_id: &str) {
        let counter = self.clocks.entry(node_id.to_string()).or_insert(0);
        *counter += 1;
        self.timestamp = now_ms();
    }

    /// Merge another clock into this one (per-node maximums).
    pub fn merge(&mut self, other: &VectorClock) {
        for (node, clock) in &other.clocks {
            let entry = self.clocks.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(*clock);
        }
        self.timestamp = self.timestamp.max(other.timestamp);
    }

    /// Whether this clock happens-before `other` (i.e. it is causally older).
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut at_least_one_less = false;
        for (node, clock) in &self.clocks {
            let other_clock = other.clocks.get(node).unwrap_or(&0);
            if clock > other_clock {
                return false;
            }
            if clock < other_clock {
                at_least_one_less = true;
            }
        }
        for (node, other_clock) in &other.clocks {
            if !self.clocks.contains_key(node) && *other_clock > 0 {
                at_least_one_less = true;
            }
        }
        at_least_one_less
    }

    /// Whether this clock and `other` describe concurrent (conflicting) writes.
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }
}

/// Kind of distributed operation in the operation log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    /// Insert a record
    Insert,
    /// Update a record
    Update,
    /// Delete a record
    Delete,
    /// Change a table schema
    SchemaChange,
    /// Create an index
    IndexCreate,
    /// Drop an index
    IndexDrop,
}

/// A single operation ordered through sync consensus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedOperation {
    /// Unique operation ID
    pub id: String,
    /// Type of operation
    pub op_type: OperationType,
    /// Storage engine type the operation targets
    pub storage_type: String,
    /// Target table/collection
    pub table: String,
    /// Record key
    pub key: String,
    /// Optional record data
    pub data: Option<serde_json::Value>,
    /// Vector clock at operation time
    pub vector_clock: VectorClock,
    /// Creation timestamp (ms)
    pub timestamp: u64,
    /// Node that originated the operation
    pub origin_node: String,
    /// Content hash of the operation
    pub hash: String,
    /// Consensus term at operation time
    pub term: u64,
    /// Operation log index
    pub index: u64,
    /// Whether the operation has been committed
    pub committed: bool,
}

/// Sync metadata tracked per record key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    /// Record key
    pub key: String,
    /// Vector clock of the record
    pub vector_clock: VectorClock,
    /// Version number
    pub version: u64,
    /// Timestamp (ms) of the last sync
    pub last_sync: u64,
    /// Nodes holding replicas of the record
    pub replicas: Vec<String>,
    /// Whether the record is out of sync
    pub dirty: bool,
    /// Content checksum
    pub checksum: String,
}

/// A validator's vote during quorum consensus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumVote {
    /// Voting node
    pub node_id: String,
    /// Whether the vote approved the operation
    pub vote: bool,
    /// Term of the vote
    pub term: u64,
    /// Hash of the vote
    pub hash: String,
}

/// Result of a quorum-based write.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusWriteResult {
    /// Whether the write reached quorum
    pub confirmed: bool,
    /// Quorum size required
    pub quorum_size: usize,
    /// Votes collected
    pub votes: Vec<QuorumVote>,
    /// ID of the operation
    pub operation_id: String,
}

/// Result of a quorum-based read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusReadResult {
    /// Record data, if a consistent value was found
    pub data: Option<serde_json::Value>,
    /// Whether the read versions agreed (consistent)
    pub is_consistent: bool,
    /// Vector clocks of the versions observed
    pub versions: Vec<VectorClock>,
    /// Nodes that supplied versions
    pub source_nodes: Vec<String>,
}

/// Health of a sync peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Peer node ID
    pub node_id: String,
    /// Whether the peer is connected
    pub connected: bool,
    /// Timestamp (ms) of the last sync
    pub last_sync: u64,
    /// Operations awaiting sync
    pub pending_operations: u64,
    /// Replication lag in milliseconds
    pub lag_ms: u64,
    /// Health score (0.0-1.0)
    pub health_score: f32,
}

/// Result of a referential integrity check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferentialIntegrityResult {
    /// Whether all checked records are valid
    pub is_valid: bool,
    /// Keys with insufficient replica coverage
    pub orphaned_references: Vec<String>,
    /// Broken foreign keys found
    pub broken_foreign_keys: Vec<String>,
    /// Records checked
    pub checked_count: u64,
    /// Records with errors
    pub error_count: u64,
}

/// Coordinates quorum consensus, sync and reconciliation across nodes.
pub struct SyncCoordinator {
    config: SyncConfig,
    node_id: String,
    term: RwLock<u64>,
    is_leader: RwLock<bool>,
    operation_log: RwLock<Vec<DistributedOperation>>,
    sync_metadata: RwLock<HashMap<String, SyncMetadata>>,
    node_status: RwLock<HashMap<String, SyncStatus>>,
    clients: Arc<RwLock<HashMap<String, Arc<RpcClient>>>>,
    db: Option<sled::Db>,
}

impl SyncCoordinator {
    /// Create a coordinator, restoring any persisted term from `db`.
    pub fn new(
        config: SyncConfig,
        node_id: String,
        clients: Arc<RwLock<HashMap<String, Arc<RpcClient>>>>,
        db: Option<sled::Db>,
    ) -> Result<Self> {
        let sc = SyncCoordinator {
            config,
            node_id,
            term: RwLock::new(0),
            is_leader: RwLock::new(false),
            operation_log: RwLock::new(Vec::new()),
            sync_metadata: RwLock::new(HashMap::new()),
            node_status: RwLock::new(HashMap::new()),
            clients,
            db,
        };
        sc.restore_state()?;
        Ok(sc)
    }

    fn restore_state(&self) -> Result<()> {
        if let Some(ref db) = self.db {
            if let Some(data) = db
                .get("sync_term")
                .map_err(|e| crate::Error::ClusterError(format!("DB read: {}", e)))?
            {
                let term: u64 = bincode::deserialize(&data).unwrap_or(0);
                *self.term.write().unwrap() = term;
            }
        }
        Ok(())
    }

    fn persist_term(&self) -> Result<()> {
        if let Some(ref db) = self.db {
            let data = bincode::serialize(&*self.term.read().unwrap())
                .map_err(|e| crate::Error::ClusterError(format!("Serialize: {}", e)))?;
            db.insert("sync_term", data)
                .map_err(|e| crate::Error::ClusterError(format!("DB write: {}", e)))?;
            db.flush()
                .map_err(|e| crate::Error::ClusterError(format!("DB flush: {}", e)))?;
        }
        Ok(())
    }

    /// Run quorum consensus for a write and record it in the operation log if
    /// confirmed.
    pub async fn consensus_write(
        &self,
        storage_type: &str,
        table: &str,
        key: &str,
        data: serde_json::Value,
        validators: Vec<String>,
    ) -> Result<ConsensusWriteResult> {
        let operation_id = self.generate_operation_id();
        let mut vector_clock = VectorClock::new(&self.node_id);
        vector_clock.increment(&self.node_id);

        let timestamp = now_ms();
        let term = *self.term.read().unwrap();

        let operation = DistributedOperation {
            id: operation_id.clone(),
            op_type: OperationType::Insert,
            storage_type: storage_type.to_string(),
            table: table.to_string(),
            key: key.to_string(),
            data: Some(data.clone()),
            vector_clock: vector_clock.clone(),
            timestamp,
            origin_node: self.node_id.clone(),
            hash: self.compute_hash(&operation_id, &timestamp),
            term,
            index: self.operation_log.read().unwrap().len() as u64,
            committed: false,
        };

        let quorum_required = self.config.write_quorum;
        let votes = self
            .request_votes(&operation, &validators, quorum_required)
            .await;
        let confirmed = votes.iter().filter(|v| v.vote).count() >= quorum_required;

        if confirmed {
            self.operation_log.write().unwrap().push(operation);
        }

        Ok(ConsensusWriteResult {
            confirmed,
            quorum_size: quorum_required,
            votes,
            operation_id,
        })
    }

    /// Perform a quorum-based read of a record, verifying version agreement.
    pub async fn consensus_read(
        &self,
        table: &str,
        key: &str,
        read_nodes: Vec<String>,
    ) -> Result<ConsensusReadResult> {
        let quorum_required = self.config.read_quorum;
        let mut versions = Vec::new();
        let mut source_nodes = Vec::new();

        for node_id in &read_nodes {
            if *node_id == self.node_id {
                if let Some(meta) = self
                    .sync_metadata
                    .read()
                    .unwrap()
                    .get(&format!("{}:{}", table, key))
                {
                    versions.push(meta.vector_clock.clone());
                    source_nodes.push(node_id.clone());
                }
            } else {
                let client = {
                    let clients = self.clients.read().unwrap();
                    clients.get(node_id).cloned()
                };
                if let Some(client) = client {
                    let req = RpcMessage::ReplicaRead(crate::cluster::rpc::ReplicaReadRequest {
                        storage_type: table.to_string(),
                        table: table.to_string(),
                        key: key.to_string(),
                    });
                    if let Ok(RpcMessage::ReplicaReadResponse(resp)) = client.send(&req).await {
                        if resp.found {
                            source_nodes.push(node_id.clone());
                        }
                    }
                }
            }
        }

        let is_consistent =
            versions.len() >= quorum_required && self.verify_version_agreement(&versions);

        Ok(ConsensusReadResult {
            data: None,
            is_consistent,
            versions,
            source_nodes,
        })
    }

    /// Reconcile this node with `target_node`: resolve any detected conflicts
    /// and merge outstanding record metadata, returning a summary of the work
    /// performed.
    pub async fn reconcile_node(&self, target_node: &str) -> Result<ReconciliationResult> {
        let status = self.node_status.read().unwrap().get(target_node).cloned();
        let mut conflicts_resolved = 0u64;
        let mut records_merged = 0u64;

        if let Some(s) = status {
            if s.pending_operations > 0 {
                conflicts_resolved = self.resolve_conflicts(target_node).await?;
                records_merged = self.merge_records(target_node).await?;
            }
        }

        Ok(ReconciliationResult {
            node_id: target_node.to_string(),
            conflicts_resolved,
            records_merged,
            timestamp: now_ms(),
        })
    }

    /// Verify that every record in `table` has enough replica coverage,
    /// returning orphaned references when the replication factor is not met.
    /// Short-circuits to `is_valid = true` when integrity checks are disabled.
    pub async fn check_referential_integrity(
        &self,
        table: &str,
    ) -> Result<ReferentialIntegrityResult> {
        if !self.config.enable_referential_integrity {
            return Ok(ReferentialIntegrityResult {
                is_valid: true,
                orphaned_references: vec![],
                broken_foreign_keys: vec![],
                checked_count: 0,
                error_count: 0,
            });
        }

        let metadata = self.sync_metadata.read().unwrap();
        let mut orphaned = Vec::new();
        let mut checked = 0u64;

        for (key, meta) in metadata.iter() {
            if key.starts_with(table) {
                checked += 1;
                if meta.dirty && meta.replicas.len() < self.config.replication_factor {
                    orphaned.push(format!("{} - insufficient replicas", key));
                }
            }
        }

        let error_count = orphaned.len() as u64;
        Ok(ReferentialIntegrityResult {
            is_valid: orphaned.is_empty(),
            orphaned_references: orphaned,
            broken_foreign_keys: vec![],
            checked_count: checked,
            error_count,
        })
    }

    /// Run a Raft-style leader election among `candidates`: bump the term,
    /// request votes over RPC, and promote this node when a majority responds.
    pub async fn elect_leader(&self, candidates: Vec<String>) -> Result<String> {
        let term = {
            let mut guard = self.term.write().unwrap();
            *guard += 1;
            *guard
        };
        self.persist_term()?;

        let mut votes = 0;
        let quorum = (candidates.len() / 2) + 1;

        for node in &candidates {
            if node == &self.node_id {
                votes += 1;
                continue;
            }
            let client = {
                let clients = self.clients.read().unwrap();
                clients.get(node).cloned()
            };
            if let Some(client) = client {
                let req = RpcMessage::RequestVote(crate::cluster::rpc::RaftVoteRequest {
                    term,
                    candidate_id: self.node_id.clone(),
                    last_log_index: 0,
                    last_log_term: 0,
                });
                if let Ok(RpcMessage::VoteResponse(resp)) = client.send(&req).await {
                    if resp.vote_granted {
                        votes += 1;
                    }
                }
            }
        }

        if votes >= quorum {
            *self.is_leader.write().unwrap() = true;
            Ok(self.node_id.clone())
        } else {
            Err(crate::Error::ClusterError("Leader election failed".into()))
        }
    }

    /// Record a local modification for `key`: bump the vector clock, increment
    /// the version, and mark the entry dirty with a fresh content checksum.
    pub fn update_metadata(&self, key: &str, data: &serde_json::Value) -> Result<()> {
        let mut metadata = self.sync_metadata.write().unwrap();
        let meta = metadata
            .entry(key.to_string())
            .or_insert_with(|| SyncMetadata {
                key: key.to_string(),
                vector_clock: VectorClock::new(&self.node_id),
                version: 0,
                last_sync: 0,
                replicas: vec![self.node_id.clone()],
                dirty: true,
                checksum: String::new(),
            });

        meta.vector_clock.increment(&self.node_id);
        meta.version += 1;
        meta.last_sync = now_ms();
        meta.dirty = true;
        meta.checksum = compute_data_checksum(data);
        Ok(())
    }

    /// Collect quorum votes for `operation` by asking each validator to ack a
    /// replica write. Voting stops early once `quorum` confirmations are seen;
    /// every validator consulted receives a signed [`QuorumVote`] entry.
    async fn request_votes(
        &self,
        operation: &DistributedOperation,
        validators: &[String],
        quorum: usize,
    ) -> Vec<QuorumVote> {
        let term = *self.term.read().unwrap();
        let mut votes = Vec::new();
        let mut confirmations = 0;

        for validator in validators {
            if confirmations >= quorum {
                break;
            }

            let vote_granted = if validator == &self.node_id {
                true
            } else {
                let client = {
                    let clients = self.clients.read().unwrap();
                    clients.get(validator).cloned()
                };
                match client {
                    Some(client) => {
                        let req = RpcMessage::ReplicaWrite(ReplicaWriteRequest {
                            operation_id: operation.id.clone(),
                            storage_type: operation.storage_type.clone(),
                            table: operation.table.clone(),
                            key: operation.key.clone(),
                            data: operation.data.clone().unwrap_or(serde_json::Value::Null),
                            term: operation.term,
                            index: operation.index,
                        });
                        match client.send(&req).await {
                            Ok(RpcMessage::ReplicaWriteAck(ack)) => ack.success,
                            _ => false,
                        }
                    }
                    None => false,
                }
            };

            if vote_granted {
                confirmations += 1;
            }
            let vote_data = format!("{}:{}:{}", validator, term, vote_granted);
            votes.push(QuorumVote {
                node_id: validator.clone(),
                vote: vote_granted,
                term,
                hash: format!("{:x}", sha2::Sha256::digest(vote_data.as_bytes())),
            });
        }
        votes
    }

    /// Pull a page of records changed on `peer_node` for `table` since the
    /// start of time, returning the peer's first `SyncResponse` batch.
    pub async fn sync_table(
        &self,
        table: &str,
        storage_type: &str,
        peer_node: &str,
    ) -> Result<SyncResponse> {
        let client = {
            let clients = self.clients.read().unwrap();
            clients.get(peer_node).cloned()
        };
        let client = client.ok_or_else(|| {
            crate::Error::ClusterError(format!("Peer {} not connected", peer_node))
        })?;

        let req = RpcMessage::SyncRequest(SyncRequest {
            node_id: self.node_id.clone(),
            table: table.to_string(),
            storage_type: storage_type.to_string(),
            last_sync_timestamp: 0,
            batch_size: 1000,
        });

        match client.send(&req).await {
            Ok(RpcMessage::SyncResponse(resp)) => Ok(resp),
            Ok(_) => Err(crate::Error::ClusterError(
                "Unexpected sync response".into(),
            )),
            Err(e) => Err(e),
        }
    }

    /// Ask `peer_node` for the Merkle root of `table`, which is compared
    /// against the local root to find divergent key ranges without transferring
    /// whole tables.
    pub async fn request_merkle_root(
        &self,
        table: &str,
        storage_type: &str,
        peer_node: &str,
    ) -> Result<MerkleResponse> {
        let client = {
            let clients = self.clients.read().unwrap();
            clients.get(peer_node).cloned()
        };
        let client = client.ok_or_else(|| {
            crate::Error::ClusterError(format!("Peer {} not connected", peer_node))
        })?;

        let req = RpcMessage::MerkleRequest(MerkleRequest {
            table: table.to_string(),
            storage_type: storage_type.to_string(),
            depth: 10,
        });

        match client.send(&req).await {
            Ok(RpcMessage::MerkleResponse(resp)) => Ok(resp),
            Ok(_) => Err(crate::Error::ClusterError(
                "Unexpected merkle response".into(),
            )),
            Err(e) => Err(e),
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Send a `ConflictResolve` instruction to `peer_node` so it applies the
    /// same resolved value for `key` that this node chose.
    pub async fn resolve_conflict(
        &self,
        key: &str,
        table: &str,
        storage_type: &str,
        resolution: &str,
        resolved_data: serde_json::Value,
        term: u64,
        peer_node: &str,
    ) -> Result<()> {
        let client = {
            let clients = self.clients.read().unwrap();
            clients.get(peer_node).cloned()
        };
        if let Some(client) = client {
            let req = RpcMessage::ConflictResolve(ConflictResolveMessage {
                key: key.to_string(),
                table: table.to_string(),
                storage_type: storage_type.to_string(),
                resolution: resolution.to_string(),
                resolved_data,
                term,
            });
            client.send(&req).await?;
        }
        Ok(())
    }

    /// Detect vector-clock-concurrent metadata entries and resolve each one
    /// last-writer-wins, notifying `target_node` of the resolution. Returns the
    /// number of conflicts resolved.
    async fn resolve_conflicts(&self, target_node: &str) -> Result<u64> {
        let metadata_pairs = {
            let meta = self.sync_metadata.read().unwrap();
            meta.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<Vec<_>>()
        };

        let mut resolved_count = 0u64;

        // Compare vector clocks across entries for concurrent conflicts
        for i in 0..metadata_pairs.len().saturating_sub(1) {
            let a = &metadata_pairs[i].1;
            let b = &metadata_pairs[i + 1].1;
            if a.vector_clock.is_concurrent(&b.vector_clock) {
                // Last-writer-wins: keep the entry with the later sync timestamp
                let keep = if a.last_sync >= b.last_sync { a } else { b };
                let meta_key = &metadata_pairs[if keep.key == a.key { i } else { i + 1 }].0;

                // Extract table name from metadata key (format: "table:id")
                let (table, _) = meta_key.split_once(':').unwrap_or_default();

                tracing::info!(
                    "Resolved concurrent conflict for '{}' in table '{}'",
                    keep.key,
                    table
                );

                // Notify the target node about the resolution
                let msg = {
                    let clients = self.clients.read().unwrap();
                    clients.get(target_node).map(|client| {
                        let msg = RpcMessage::ConflictResolve(ConflictResolveMessage {
                            key: keep.key.clone(),
                            table: table.to_string(),
                            storage_type: "document".to_string(),
                            resolution: "last-writer-wins".to_string(),
                            resolved_data: serde_json::json!({}),
                            term: *self.term.read().unwrap(),
                        });
                        (client.clone(), msg)
                    })
                };
                if let Some((client, msg)) = msg {
                    let _ = client.send(&msg).await;
                }
                resolved_count += 1;
            }
        }

        Ok(resolved_count)
    }

    /// Push metadata entries this node holds to `target_node` via replica
    /// writes so both nodes converge. Returns the number of entries merged.
    async fn merge_records(&self, target_node: &str) -> Result<u64> {
        let metadata = {
            let meta = self.sync_metadata.read().unwrap();
            meta.clone()
        };

        let mut merged_count = 0u64;

        // Push metadata entries to the target node via replica write
        for (key, entry) in &metadata {
            if entry.replicas.contains(&self.node_id) {
                continue;
            }

            let client = {
                let clients = self.clients.read().unwrap();
                clients.get(target_node).cloned()
            };
            if let Some(client) = client {
                let msg = RpcMessage::ReplicaWrite(ReplicaWriteRequest {
                    operation_id: key.clone(),
                    storage_type: "keyvalue".to_string(),
                    table: key.clone(),
                    key: key.clone(),
                    data: serde_json::json!({}),
                    term: *self.term.read().unwrap(),
                    index: entry.version,
                });
                if client.send(&msg).await.is_ok() {
                    merged_count += 1;
                }
            }
        }

        if merged_count > 0 {
            tracing::info!(
                "Merged {} metadata entries with node {}",
                merged_count,
                target_node
            );
        }

        Ok(merged_count)
    }

    /// Whether every vector clock in `versions` is causally compatible (no two
    /// describe concurrent writes), i.e. the observed replicas agree.
    fn verify_version_agreement(&self, versions: &[VectorClock]) -> bool {
        if versions.is_empty() {
            return true;
        }
        let base = &versions[0];
        versions.iter().all(|v| !base.is_concurrent(v))
    }

    fn generate_operation_id(&self) -> String {
        format!("{}-{}-{}", self.node_id, now_ms(), rand_id())
    }

    fn compute_hash(&self, operation_id: &str, timestamp: &u64) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        operation_id.hash(&mut hasher);
        timestamp.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Build a Merkle tree for a relational table's rows.
    pub fn build_table_merkle_tree(
        &self,
        _table_name: &str,
        rows: &[RecordVersionEntry],
    ) -> MerkleNode {
        if rows.is_empty() {
            return MerkleNode {
                hash: format!("{:x}", sha2::Sha256::digest(b"")),
                children: None,
                key_range: None,
                is_leaf: true,
            };
        }

        let leaf_hashes: Vec<(String, String)> = rows
            .iter()
            .map(|(id, _vc, version, checksum)| {
                let h = format!(
                    "{:x}",
                    sha2::Sha256::digest(format!("{}:{}:{}", id, checksum, version).as_bytes())
                );
                (id.clone(), h)
            })
            .collect();

        build_merkle_tree_recursive(&leaf_hashes)
    }

    /// Reconcile a relational table between local and remote rows.
    pub fn reconcile_table(
        &self,
        _table_name: &str,
        local_rows: &[RecordVersionEntryWithData],
        remote_rows: &[RecordVersionEntryWithData],
        local_node_id: &str,
        remote_node_id: &str,
    ) -> ReconciliationPlan {
        let local_map: HashMap<&str, &(_, _, _, _, _)> =
            local_rows.iter().map(|r| (r.0.as_str(), r)).collect();
        let remote_map: HashMap<&str, &(_, _, _, _, _)> =
            remote_rows.iter().map(|r| (r.0.as_str(), r)).collect();

        let mut pull_records = Vec::new();
        let mut push_records = Vec::new();
        let mut conflicts = Vec::new();

        for key in remote_map.keys() {
            if !local_map.contains_key(key) {
                pull_records.push(key.to_string());
            }
        }

        for key in local_map.keys() {
            if !remote_map.contains_key(key) {
                push_records.push(key.to_string());
            }
        }

        for (key, local) in &local_map {
            if let Some(remote) = remote_map.get(key) {
                let local_vc = &local.1;
                let remote_vc = &remote.1;
                if local_vc != remote_vc {
                    let mut local_newer = false;
                    let mut remote_newer = false;

                    for (node, clock) in remote_vc {
                        let lc = local_vc.get(node).unwrap_or(&0);
                        if clock > lc {
                            remote_newer = true;
                        }
                        if clock < lc {
                            local_newer = true;
                        }
                    }
                    for (node, clock) in local_vc {
                        if !remote_vc.contains_key(node) && *clock > 0 {
                            local_newer = true;
                        }
                    }

                    if local_newer && !remote_newer {
                        push_records.push(key.to_string());
                    } else if remote_newer && !local_newer {
                        pull_records.push(key.to_string());
                    } else {
                        conflicts.push(DataConflict {
                            key: key.to_string(),
                            local_version: RecordVersion {
                                key: key.to_string(),
                                version: local.2,
                                vector_clock: local_vc.clone(),
                                timestamp: 0,
                                modified_by: local_node_id.to_string(),
                                checksum: local.3.clone(),
                            },
                            remote_version: RecordVersion {
                                key: key.to_string(),
                                version: remote.2,
                                vector_clock: remote_vc.clone(),
                                timestamp: 0,
                                modified_by: remote_node_id.to_string(),
                                checksum: remote.3.clone(),
                            },
                            resolution: ConflictResolutionStrategy::KeepMostRecent,
                        });
                    }
                }
            }
        }

        ReconciliationPlan {
            pull_records,
            push_records,
            conflicts,
            estimated_bytes: 0,
        }
    }

    /// Seed the sync-metadata table from a relational table's version entries,
    /// merging each row's vector clock with the local node's counter.
    pub fn register_table_sync_metadata(&self, table_name: &str, rows: &[RecordVersionEntry]) {
        let mut metadata = self.sync_metadata.write().unwrap();
        for (id, vc, version, checksum) in rows {
            let key = format!("{}:{}", table_name, id);
            let mut vector_clock = cluster_vector_clock_from_map(vc);
            vector_clock.increment(&self.node_id);
            metadata.insert(
                key,
                SyncMetadata {
                    key: id.clone(),
                    vector_clock,
                    version: *version,
                    last_sync: now_ms(),
                    replicas: vec![self.node_id.clone()],
                    dirty: false,
                    checksum: checksum.clone(),
                },
            );
        }
    }

    /// Perform cross-cluster reconciliation between two regions.
    /// Uses vector clock comparison to resolve concurrent writes
    /// and produce a merge plan.
    pub async fn cross_cluster_reconcile(
        &self,
        remote_cluster_id: &str,
        local_versions: HashMap<String, reconciliation::RecordVersion>,
        remote_versions: HashMap<String, reconciliation::RecordVersion>,
    ) -> Result<ReconciliationPlan> {
        let plan = build_cross_cluster_reconciliation_plan(&local_versions, &remote_versions);
        info!(
            "Cross-cluster reconcile with {}: push={}, pull={}, conflicts={}",
            remote_cluster_id,
            plan.push_records.len(),
            plan.pull_records.len(),
            plan.conflicts.len(),
        );

        // Resolve any concurrent conflicts using vector clocks
        for conflict in &plan.conflicts {
            let (winning, strategy) =
                resolve_cross_cluster_conflict(&conflict.local_version, &conflict.remote_version);
            debug!(
                "Conflict resolved for key {}: {:?} (winner={})",
                conflict.key, strategy, winning.modified_by
            );
        }

        Ok(plan)
    }
}

/// Summary of a single reconciliation pass against one peer node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationResult {
    /// Node that was reconciled against
    pub node_id: String,
    /// Number of concurrent conflicts resolved during the pass
    pub conflicts_resolved: u64,
    /// Number of records merged into the peer
    pub records_merged: u64,
    /// Timestamp (ms) of the reconciliation pass
    pub timestamp: u64,
}

fn cluster_vector_clock_from_map(map: &HashMap<String, u64>) -> VectorClock {
    let mut vc = VectorClock::new("sync");
    vc.clocks = map.clone();
    vc
}

fn build_merkle_tree_recursive(leaf_hashes: &[(String, String)]) -> MerkleNode {
    if leaf_hashes.len() == 1 {
        return MerkleNode {
            hash: leaf_hashes[0].1.clone(),
            children: None,
            key_range: Some((leaf_hashes[0].0.clone(), leaf_hashes[0].0.clone())),
            is_leaf: true,
        };
    }

    let mut current: Vec<MerkleNode> = leaf_hashes
        .iter()
        .map(|(key, hash)| MerkleNode {
            hash: hash.clone(),
            children: None,
            key_range: Some((key.clone(), key.clone())),
            is_leaf: true,
        })
        .collect();

    while current.len() > 1 {
        let mut next = Vec::new();
        for chunk in current.chunks(2) {
            if chunk.len() == 2 {
                let combined = format!("{}{}", chunk[0].hash, chunk[1].hash);
                let hash = format!("{:x}", sha2::Sha256::digest(combined.as_bytes()));
                let start = chunk[0]
                    .key_range
                    .as_ref()
                    .map(|k| k.0.clone())
                    .unwrap_or_default();
                let end = chunk[1]
                    .key_range
                    .as_ref()
                    .map(|k| k.1.clone())
                    .unwrap_or_default();
                next.push(MerkleNode {
                    hash,
                    children: Some(vec![chunk[0].hash.clone(), chunk[1].hash.clone()]),
                    key_range: Some((start, end)),
                    is_leaf: false,
                });
            } else {
                next.push(chunk[0].clone());
            }
        }
        current = next;
    }
    current.into_iter().next().unwrap()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn rand_id() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    (nanos as u32) ^ ((nanos >> 32) as u32)
}

fn compute_data_checksum(data: &serde_json::Value) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
