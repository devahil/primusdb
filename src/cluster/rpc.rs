//! Inter-node RPC protocol
//!
//! Defines the length-prefixed, bincode-serialized message envelope
//! ([`RpcMessage`]) shared by every cluster subsystem — Raft, membership gossip,
//! data replication, cluster management, sync/reconciliation, federation and
//! federated Raft. [`RpcClient`] provides a blocking request/response client
//! over a single TCP connection, while [`RpcServer`] accepts connections and
//! dispatches each message to a [`RpcHandler`].
//!
//! # Placement in the architecture
//!
//! Every cluster subsystem funnels its traffic through [`RpcMessage`], so a
//! single transport serves the whole control plane. The intra-cluster layers
//! (Raft, membership, replication, sync) use [`RpcClient`] / [`RpcServer`],
//! while the federation layers use the short-lived [`connect_and_send`]
//! helper from [`crate::cluster::federation`].
//!
//! ```text
//!   ┌─────────────────────────── Cluster node ───────────────────────────┐
//!   │                                                                     │
//!   │  Raft ──► membership ──► replication ──► sync ──► federation         │
//!   │              │                          │                           │
//!   │              ▼                          ▼                           │
//!   │   ┌────────────────────────────────────────────┐                   │
//!   │   │        RpcClient / RpcServer               │                   │
//!   │   │   bincode + u32 length prefix over TCP     │                   │
//!   │   └────────────────────────────────────────────┘                   │
//!   └────────────────────────────┬───────────────────────────────────────┘
//!                                ▼
//!                       peer node RPC server
//!   RpcMessage = one wire envelope for every subsystem's request/response
//! ```

use crate::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

/// Wire envelope for every inter-node RPC message.
///
/// One variant per request/response pair across all cluster subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcMessage {
    // Raft consensus messages
    RequestVote(RaftVoteRequest),
    VoteResponse(RaftVoteResponse),
    AppendEntries(RaftAppendRequest),
    AppendEntriesResponse(RaftAppendResponse),
    InstallSnapshot(InstallSnapshotRequest),

    // Membership gossip
    Ping(PingMessage),
    PingReq(PingReqMessage),
    Ack(AckMessage),

    // Data replication
    ReplicaWrite(ReplicaWriteRequest),
    ReplicaWriteAck(ReplicaWriteAck),
    ReplicaRead(ReplicaReadRequest),
    ReplicaReadResponse(ReplicaReadResponse),
    ShardTransfer(ShardTransferRequest),
    ShardTransferChunk(ShardTransferChunk),

    // Cluster management
    JoinRequest(JoinRequest),
    JoinResponse(JoinResponse),
    Heartbeat(HeartbeatMessage),
    MetadataSync(MetadataSyncMessage),

    // Sync & reconciliation
    SyncRequest(SyncRequest),
    SyncResponse(SyncResponse),
    MerkleRequest(MerkleRequest),
    MerkleResponse(MerkleResponse),
    ConflictResolve(ConflictResolveMessage),

    // Federation (multi-cluster super-scalar, v1.3.1-alpha)
    FedClusterAnnounce(FedClusterAnnounce),
    FedClusterAck(FedClusterAck),
    FedDomainJoin(FedDomainJoin),
    FedDomainJoinAck(FedDomainJoinAck),
    FedDomainLeave(FedDomainLeave),
    FedDataReplica(FedDataReplicaRequest),
    FedDataReplicaAck(FedDataReplicaAck),
    FedHeartbeat(FedHeartbeatMessage),
    FedNamespaceResolve(FedNamespaceResolveRequest),
    FedNamespaceResolveAck(FedNamespaceResolveAck),

    // Federated Raft (cross-cluster consensus, v1.3.1-alpha)
    FedRaftVoteRequest(FedRaftVoteRequest),
    FedRaftVoteResponse(FedRaftVoteResponse),
    FedRaftAppendEntries(FedRaftAppendRequest),
    FedRaftAppendEntriesResponse(FedRaftAppendResponse),
}

// --- Raft RPC ---

/// Raft `RequestVote` RPC sent by a candidate during leader election.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftVoteRequest {
    /// Candidate's term
    pub term: u64,
    /// Candidate requesting the vote
    pub candidate_id: String,
    /// Last log index of the candidate
    pub last_log_index: u64,
    /// Last log term of the candidate
    pub last_log_term: u64,
}

/// Reply to a [`RaftVoteRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftVoteResponse {
    /// Responder's current term
    pub term: u64,
    /// Whether the vote was granted
    pub vote_granted: bool,
    /// Node that responded
    pub node_id: String,
}

/// Raft `AppendEntries` RPC used by the leader for replication and heartbeats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftAppendRequest {
    /// Leader's term
    pub term: u64,
    /// Leader's node ID
    pub leader_id: String,
    /// Index of the log entry preceding `entries`
    pub prev_log_index: u64,
    /// Term of the entry at `prev_log_index`
    pub prev_log_term: u64,
    /// Serialized log entries to append (empty for heartbeats)
    pub entries: Vec<Vec<u8>>,
    /// Leader's commit index
    pub leader_commit: u64,
}

/// Reply to a [`RaftAppendRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftAppendResponse {
    /// Follower's current term
    pub term: u64,
    /// Whether the entries were accepted
    pub success: bool,
    /// Highest matching log index on the follower
    pub match_index: u64,
    /// Node that responded
    pub node_id: String,
    /// Follower's last log index
    pub last_log_index: u64,
}

/// Raft `InstallSnapshot` RPC streaming a snapshot to a lagging follower.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotRequest {
    /// Leader's term
    pub term: u64,
    /// Leader's node ID
    pub leader_id: String,
    /// Log index of the last entry included in the snapshot
    pub last_included_index: u64,
    /// Term of the last entry included in the snapshot
    pub last_included_term: u64,
    /// Byte offset of `data` within the snapshot
    pub offset: u64,
    /// Chunk of snapshot data
    pub data: Vec<u8>,
    /// Whether this chunk is the final one
    pub done: bool,
}

// --- Membership ---

/// Direct liveness probe sent by a gossip node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    /// Sender's node ID
    pub sender_id: String,
    /// Sequence number of the probe
    pub sequence: u64,
    /// Incarnation of the sender
    pub incarnation: u64,
}

/// Indirect liveness probe: asks a third node to check on `target_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingReqMessage {
    /// Sender's node ID
    pub sender_id: String,
    /// Node being probed indirectly
    pub target_id: String,
    /// Sequence number of the original probe
    pub sequence: u64,
    /// Incarnation of the sender
    pub incarnation: u64,
}

/// Reply to a ping or ping-req probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckMessage {
    /// Responder's node ID
    pub sender_id: String,
    /// Sequence number being acknowledged
    pub sequence: u64,
    /// Whether the probed node is alive
    pub ok: bool,
}

// --- Replication ---

/// Request to write a record on a replica node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaWriteRequest {
    /// Unique operation ID
    pub operation_id: String,
    /// Storage engine type of the write
    pub storage_type: String,
    /// Target table/collection
    pub table: String,
    /// Record key
    pub key: String,
    /// Record data
    pub data: serde_json::Value,
    /// Raft term of the write
    pub term: u64,
    /// Raft log index of the write
    pub index: u64,
}

/// Acknowledgement of a [`ReplicaWriteRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaWriteAck {
    /// Operation ID being acknowledged
    pub operation_id: String,
    /// Node that acknowledged
    pub node_id: String,
    /// Whether the write succeeded
    pub success: bool,
    /// Raft term of the acknowledged write
    pub term: u64,
}

/// Request to read a record from a replica node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaReadRequest {
    /// Storage engine type of the read
    pub storage_type: String,
    /// Target table/collection
    pub table: String,
    /// Record key
    pub key: String,
}

/// Reply to a [`ReplicaReadRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaReadResponse {
    /// Whether the record was found
    pub found: bool,
    /// Record data, if found
    pub data: Option<serde_json::Value>,
    /// Node that responded
    pub node_id: String,
}

/// Begin transferring a shard's data to the target node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardTransferRequest {
    /// Shard being transferred
    pub shard_id: String,
    /// Table the shard belongs to
    pub table: String,
    /// Storage engine type of the shard
    pub storage_type: String,
    /// Total number of chunks in the transfer
    pub total_chunks: u32,
}

/// One chunk of shard data during a transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardTransferChunk {
    /// Shard being transferred
    pub shard_id: String,
    /// Zero-based index of this chunk
    pub chunk_index: u32,
    /// Raw chunk bytes
    pub data: Vec<u8>,
    /// Whether this is the final chunk
    pub is_last: bool,
}

// --- Cluster management ---

/// Request to join the cluster, sent to a seed server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    /// Joining node's ID
    pub node_id: String,
    /// Joining node's address
    pub address: String,
    /// Joining node's RPC port
    pub port: u16,
    /// Number of CPU cores of the joining node
    pub cpu_cores: u32,
    /// Memory of the joining node in GB
    pub memory_gb: u64,
    /// Storage of the joining node in GB
    pub storage_gb: u64,
    /// Roles the joining node can perform
    pub roles: Vec<String>,
}

/// Reply to a [`JoinRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinResponse {
    /// Whether the join was accepted
    pub accepted: bool,
    /// ID of the current cluster leader, if known
    pub leader_id: Option<String>,
    /// Snapshot of the cluster's nodes
    pub cluster_nodes: Vec<ClusterNodeInfo>,
    /// Error description if the join failed
    pub error: Option<String>,
}

/// Serializable summary of a cluster node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNodeInfo {
    /// Node ID
    pub node_id: String,
    /// Node address
    pub address: String,
    /// Node RPC port
    pub port: u16,
    /// Roles the node performs
    pub roles: Vec<String>,
    /// Node status (e.g. `"Alive"`)
    pub status: String,
}

/// Periodic cluster heartbeat carrying load metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    /// Sender's node ID
    pub node_id: String,
    /// Raft term of the sender
    pub term: u64,
    /// Current leader ID, if known
    pub leader_id: Option<String>,
    /// CPU usage (0.0-1.0)
    pub cpu_usage: f64,
    /// Memory usage (0.0-1.0)
    pub memory_usage: f64,
    /// Storage usage (0.0-1.0)
    pub storage_usage: f64,
    /// Active connections
    pub active_connections: u32,
}

/// Metadata exchanged during membership gossip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataSyncMessage {
    /// Node the metadata describes
    pub node_id: String,
    /// Term/incarnation of the entry
    pub term: u64,
    /// Number of shards hosted
    pub shard_count: u32,
    /// Total records stored
    pub total_records: u64,
    /// Checksum over the gossiped member data
    pub checksum: String,
}

// --- Sync & Reconciliation ---

/// Request to pull records changed since `last_sync_timestamp`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    /// Requesting node
    pub node_id: String,
    /// Table to sync
    pub table: String,
    /// Storage engine type of the table
    pub storage_type: String,
    /// Only return records changed after this timestamp (ms)
    pub last_sync_timestamp: u64,
    /// Maximum records per response batch
    pub batch_size: u32,
}

/// Page of records returned for a [`SyncRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    /// Records in this batch
    pub records: Vec<SyncRecord>,
    /// Whether more batches follow
    pub more: bool,
    /// Cursor for the next batch
    pub cursor: Option<String>,
}

/// A single record's sync metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    /// Record key
    pub key: String,
    /// Record data
    pub data: serde_json::Value,
    /// Version number
    pub version: u64,
    /// Vector clock of the record
    pub vector_clock: std::collections::HashMap<String, u64>,
    /// Content checksum
    pub checksum: String,
    /// Last modification timestamp (ms)
    pub timestamp: u64,
}

/// Request for a Merkle root to compare table contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleRequest {
    /// Table to hash
    pub table: String,
    /// Storage engine type of the table
    pub storage_type: String,
    /// Merkle tree depth
    pub depth: u32,
}

/// Reply to a [`MerkleRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleResponse {
    /// Root hash of the table's Merkle tree
    pub root_hash: String,
    /// Number of records hashed
    pub node_count: u32,
    /// Tree depth used
    pub depth: u32,
}

/// Instruct a node to apply a conflict resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolveMessage {
    /// Record key
    pub key: String,
    /// Table the record belongs to
    pub table: String,
    /// Storage engine type of the record
    pub storage_type: String,
    /// Resolution strategy applied
    pub resolution: String,
    /// Data to persist after resolution
    pub resolved_data: serde_json::Value,
    /// Raft term of the resolution
    pub term: u64,
}

// --- RPC Client ---

/// Length-prefixed, bincode-serialized RPC client bound to a single peer node.
#[derive(Debug)]
pub struct RpcClient {
    node_id: String,
    addr: SocketAddr,
    stream: RwLock<Option<TcpStream>>,
}

impl RpcClient {
    /// Create a client for the peer at `addr`. Call [`connect`](Self::connect)
    /// before sending messages.
    pub fn new(node_id: String, addr: SocketAddr) -> Self {
        Self {
            node_id,
            addr,
            stream: RwLock::new(None),
        }
    }

    /// The node ID this client is bound to.
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// The peer address this client connects to.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Open the TCP connection to the peer.
    pub async fn connect(&self) -> Result<()> {
        let stream = TcpStream::connect(self.addr).await.map_err(|e| {
            crate::Error::ClusterError(format!("RPC connect to {}: {}", self.addr, e))
        })?;
        stream.set_nodelay(true).ok();
        *self.stream.write().await = Some(stream);
        Ok(())
    }

    /// Close the connection to the peer.
    pub async fn disconnect(&self) {
        *self.stream.write().await = None;
    }

    /// Whether the connection to the peer is currently open.
    pub async fn is_connected(&self) -> bool {
        self.stream.read().await.is_some()
    }

    /// Send one message and wait for the reply on the same connection.
    pub async fn send(&self, msg: &RpcMessage) -> Result<RpcMessage> {
        let mut stream_guard = self.stream.write().await;
        let stream = stream_guard
            .as_mut()
            .ok_or_else(|| crate::Error::ClusterError("RPC not connected".into()))?;

        let data = bincode::serialize(msg)
            .map_err(|e| crate::Error::ClusterError(format!("RPC serialize: {}", e)))?;
        let len = data.len() as u32;
        stream
            .write_all(&len.to_le_bytes())
            .await
            .map_err(|e| crate::Error::ClusterError(format!("RPC write: {}", e)))?;
        stream
            .write_all(&data)
            .await
            .map_err(|e| crate::Error::ClusterError(format!("RPC write: {}", e)))?;

        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| crate::Error::ClusterError(format!("RPC read: {}", e)))?;
        let resp_len = u32::from_le_bytes(len_buf) as usize;
        let mut resp_buf = vec![0u8; resp_len];
        stream
            .read_exact(&mut resp_buf)
            .await
            .map_err(|e| crate::Error::ClusterError(format!("RPC read: {}", e)))?;

        bincode::deserialize(&resp_buf)
            .map_err(|e| crate::Error::ClusterError(format!("RPC deserialize: {}", e)))
    }
}

// --- RPC Server ---

/// Signature of an RPC request handler: maps an inbound message plus its source
/// address to a response message.
pub type RpcHandler = Arc<dyn Fn(RpcMessage, SocketAddr) -> Result<RpcMessage> + Send + Sync>;

/// TCP server that accepts inter-node RPC connections and dispatches each
/// message to a [`RpcHandler`].
pub struct RpcServer {
    bind_addr: SocketAddr,
}

impl RpcServer {
    /// Create a server that will bind to `bind_addr`.
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self { bind_addr }
    }

    /// Accept connections in a loop, dispatching each message to `handler`,
    /// until a shutdown signal is received on `shutdown_rx`.
    pub async fn start(self, handler: RpcHandler, shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await.map_err(|e| {
            crate::Error::ClusterError(format!("RPC bind {}: {}", self.bind_addr, e))
        })?;

        info!("RPC server listening on {}", self.bind_addr);

        let handler = Arc::clone(&handler);
        tokio::pin!(shutdown_rx);

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("RPC server shutting down");
                    break;
                }
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            let h = Arc::clone(&handler);
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, addr, h).await {
                                    debug!("RPC connection from {}: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            warn!("RPC accept error: {}", e);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    handler: RpcHandler,
) -> Result<()> {
    stream.set_nodelay(true).ok();
    let timeout = std::time::Duration::from_secs(30);
    let mut len_buf = [0u8; 4];

    loop {
        match tokio::time::timeout(timeout, stream.read_exact(&mut len_buf)).await {
            Ok(Ok(_len)) => {}
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                debug!("RPC client {} disconnected", addr);
                return Ok(());
            }
            Ok(Err(e)) => {
                return Err(crate::Error::ClusterError(format!("RPC read len: {}", e)));
            }
            Err(_) => {
                debug!("RPC read timeout from {}", addr);
                return Ok(());
            }
        }

        let msg_len = u32::from_le_bytes(len_buf) as usize;
        let mut msg_buf = vec![0u8; msg_len];
        stream
            .read_exact(&mut msg_buf)
            .await
            .map_err(|e| crate::Error::ClusterError(format!("RPC read body: {}", e)))?;

        let msg: RpcMessage = bincode::deserialize(&msg_buf)
            .map_err(|e| crate::Error::ClusterError(format!("RPC deserialize: {}", e)))?;

        let response = handler(msg, addr)?;

        let resp_data = bincode::serialize(&response)
            .map_err(|e| crate::Error::ClusterError(format!("RPC serialize resp: {}", e)))?;
        let resp_len = resp_data.len() as u32;
        stream
            .write_all(&resp_len.to_le_bytes())
            .await
            .map_err(|e| crate::Error::ClusterError(format!("RPC write resp: {}", e)))?;
        stream
            .write_all(&resp_data)
            .await
            .map_err(|e| crate::Error::ClusterError(format!("RPC write resp: {}", e)))?;
    }
}

// ==================== Federation (Multi-Cluster SuperScalar) Messages ====================

/// Announce a cluster's presence in the federation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedClusterAnnounce {
    /// ID of the announcing cluster
    pub cluster_id: String,
    /// Federation this cluster belongs to
    pub federation_id: String,
    /// Node that sent the announcement
    pub node_id: String,
    /// Address of the announcing cluster's federation endpoint
    pub address: String,
    /// Port of the announcing cluster's federation endpoint
    pub port: u16,
    /// Number of nodes in the announcing cluster
    pub cluster_size: u32,
    /// Data domains hosted by the announcing cluster
    pub domains: Vec<String>,
    /// Incarnation counter used to detect restarts
    pub incarnation: u64,
    /// Capabilities advertised by the cluster (e.g. `raft`, `swim`)
    pub capabilities: Vec<String>,
}

/// Reply to a [`FedClusterAnnounce`], acknowledging the cluster and listing
/// other clusters the responder knows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedClusterAck {
    /// Whether the announcement was accepted
    pub accepted: bool,
    /// Federation ID of the responder
    pub federation_id: String,
    /// Clusters known to the responder
    pub known_clusters: Vec<FedClusterInfo>,
    /// Hint about the federation leader
    pub leader_hint: Option<String>,
}

/// Join a data domain across clusters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedDomainJoin {
    /// ID of the cluster joining the domain
    pub cluster_id: String,
    /// Name of the domain being joined
    pub domain_name: String,
    /// Node requesting the join
    pub node_id: String,
    /// Collections the joining cluster brings into the domain
    pub collections: Vec<String>,
    /// Storage engine types covered by the domain
    pub storage_types: Vec<String>,
    /// Desired replication mode across domain members
    pub replication_mode: String,
}

/// Reply to a [`FedDomainJoin`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedDomainJoinAck {
    /// Whether the join was accepted
    pub accepted: bool,
    /// Domain that was joined
    pub domain_name: String,
    /// Clusters that are members of the domain
    pub members: Vec<String>,
    /// Hint about the domain leader cluster
    pub leader_hint: Option<String>,
    /// Error description if the join failed
    pub error: Option<String>,
}

/// Leave a data domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedDomainLeave {
    /// ID of the cluster leaving the domain
    pub cluster_id: String,
    /// Name of the domain being left
    pub domain_name: String,
    /// Node requesting the leave
    pub node_id: String,
}

/// Cross-cluster data replication request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedDataReplicaRequest {
    /// Unique operation ID for deduplication
    pub operation_id: String,
    /// Data domain the write belongs to
    pub domain_name: String,
    /// Cluster that originated the write
    pub source_cluster: String,
    /// Cluster this request is destined for
    pub target_cluster: String,
    /// Storage engine type of the write
    pub storage_type: String,
    /// Target table/collection
    pub table: String,
    /// Record key
    pub key: String,
    /// Serialized record data
    pub data: Vec<u8>,
    /// Creation timestamp (ms)
    pub timestamp: u64,
    /// Vector clock capturing write causality
    pub vector_clock: String,
}

/// Reply to a [`FedDataReplicaRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedDataReplicaAck {
    /// Operation ID being acknowledged
    pub operation_id: String,
    /// Cluster that acknowledged
    pub cluster_id: String,
    /// Whether the replica write succeeded
    pub success: bool,
    /// Error description if it failed
    pub error: Option<String>,
}

/// Cross-cluster heartbeat carrying liveness and load metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedHeartbeatMessage {
    /// ID of the sending cluster
    pub cluster_id: String,
    /// Node that sent the heartbeat
    pub node_id: String,
    /// Current cluster leader, if known
    pub leader_id: Option<String>,
    /// Raft/federated term of the sender
    pub term: u64,
    /// Number of data domains hosted by the sender
    pub domain_count: u32,
    /// Number of alive nodes in the sender cluster
    pub alive_nodes: u32,
    /// CPU usage (0.0-1.0)
    pub cpu_usage: f64,
    /// Memory usage (0.0-1.0)
    pub memory_usage: f64,
    /// Observed average latency of the sender (ms)
    pub avg_latency_ms: f64,
    /// Incarnation counter used to detect restarts
    pub incarnation: u64,
}

/// Cross-cluster namespace resolution request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedNamespaceResolveRequest {
    /// ID of the requesting cluster
    pub cluster_id: String,
    /// Federated namespace path to resolve
    pub namespace_path: String,
    /// Logical resource name being looked up
    pub resource_name: String,
    /// Storage engine type of the resource
    pub storage_type: String,
    /// Request ID for correlating the reply
    pub request_id: String,
}

/// Reply to a [`FedNamespaceResolveRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedNamespaceResolveAck {
    /// Request ID echoed from the request
    pub request_id: String,
    /// Whether the namespace resolved on this cluster
    pub found: bool,
    /// Cluster that answered
    pub cluster_id: String,
    /// Physical resource name if the namespace was found
    pub physical_name: Option<String>,
    /// Error description if resolution failed
    pub error: Option<String>,
}

/// Information about a cluster in the federation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedClusterInfo {
    /// ID of the cluster
    pub cluster_id: String,
    /// ID of the cluster's leader node, if known
    pub leader_node_id: Option<String>,
    /// Address of the cluster's federation endpoint
    pub address: String,
    /// Port of the cluster's federation endpoint
    pub port: u16,
    /// Total number of nodes in the cluster
    pub cluster_size: u32,
    /// Number of alive nodes in the cluster
    pub alive_count: u32,
    /// Data domains hosted by the cluster
    pub domains: Vec<String>,
    /// Incarnation counter used to detect restarts
    pub incarnation: u64,
    /// Liveness status string (e.g. `"Online"`)
    pub status: String,
    /// Observed average latency of the cluster (ms)
    pub avg_latency_ms: f64,
    /// Optional region identifier of the cluster
    pub region: Option<String>,
}

// ==================== Federated Raft (Cross-Cluster Consensus) Messages ====================

/// Federated Raft `RequestVote` RPC between clusters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedRaftVoteRequest {
    /// Candidate's federated term
    pub term: u64,
    /// Candidate cluster ID
    pub candidate_id: String,
    /// Last federated log index of the candidate
    pub last_log_index: u64,
    /// Last federated log term of the candidate
    pub last_log_term: u64,
}

/// Reply to a [`FedRaftVoteRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedRaftVoteResponse {
    /// Responder's federated term
    pub term: u64,
    /// Whether the vote was granted
    pub vote_granted: bool,
    /// Cluster that responded
    pub cluster_id: String,
}

/// Federated Raft `AppendEntries` RPC between clusters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedRaftAppendRequest {
    /// Leader's federated term
    pub term: u64,
    /// Leader cluster ID
    pub leader_id: String,
    /// Index of the entry preceding `entries`
    pub prev_log_index: u64,
    /// Term of the entry at `prev_log_index`
    pub prev_log_term: u64,
    /// Log entries to append
    pub entries: Vec<FedRaftLogEntry>,
    /// Leader's federated commit index
    pub leader_commit: u64,
}

/// Reply to a [`FedRaftAppendRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedRaftAppendResponse {
    /// Responder's federated term
    pub term: u64,
    /// Whether the entries were accepted
    pub success: bool,
    /// Highest matching log index on the responder
    pub match_index: u64,
    /// Cluster that responded
    pub cluster_id: String,
}

/// A single entry in the federated log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedRaftLogEntry {
    /// Entry index
    pub index: u64,
    /// Entry term
    pub term: u64,
    /// Operation type (see [`crate::cluster::federated_raft::FedRaftOpType`])
    pub op_type: String,
    /// Operation payload
    pub data: Vec<u8>,
    /// Creation timestamp (ms)
    pub timestamp: u64,
}
