use crate::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};

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

    // Federation (multi-cluster super-scalar, v1.3.0-alpha)
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

    // Federated Raft (cross-cluster consensus, v1.3.0-alpha)
    FedRaftVoteRequest(FedRaftVoteRequest),
    FedRaftVoteResponse(FedRaftVoteResponse),
    FedRaftAppendEntries(FedRaftAppendRequest),
    FedRaftAppendEntriesResponse(FedRaftAppendResponse),
}

// --- Raft RPC ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftVoteRequest {
    pub term: u64,
    pub candidate_id: String,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftVoteResponse {
    pub term: u64,
    pub vote_granted: bool,
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftAppendRequest {
    pub term: u64,
    pub leader_id: String,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<Vec<u8>>,
    pub leader_commit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftAppendResponse {
    pub term: u64,
    pub success: bool,
    pub match_index: u64,
    pub node_id: String,
    pub last_log_index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotRequest {
    pub term: u64,
    pub leader_id: String,
    pub last_included_index: u64,
    pub last_included_term: u64,
    pub offset: u64,
    pub data: Vec<u8>,
    pub done: bool,
}

// --- Membership ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    pub sender_id: String,
    pub sequence: u64,
    pub incarnation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingReqMessage {
    pub sender_id: String,
    pub target_id: String,
    pub sequence: u64,
    pub incarnation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckMessage {
    pub sender_id: String,
    pub sequence: u64,
    pub ok: bool,
}

// --- Replication ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaWriteRequest {
    pub operation_id: String,
    pub storage_type: String,
    pub table: String,
    pub key: String,
    pub data: serde_json::Value,
    pub term: u64,
    pub index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaWriteAck {
    pub operation_id: String,
    pub node_id: String,
    pub success: bool,
    pub term: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaReadRequest {
    pub storage_type: String,
    pub table: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaReadResponse {
    pub found: bool,
    pub data: Option<serde_json::Value>,
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardTransferRequest {
    pub shard_id: String,
    pub table: String,
    pub storage_type: String,
    pub total_chunks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardTransferChunk {
    pub shard_id: String,
    pub chunk_index: u32,
    pub data: Vec<u8>,
    pub is_last: bool,
}

// --- Cluster management ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    pub node_id: String,
    pub address: String,
    pub port: u16,
    pub cpu_cores: u32,
    pub memory_gb: u64,
    pub storage_gb: u64,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinResponse {
    pub accepted: bool,
    pub leader_id: Option<String>,
    pub cluster_nodes: Vec<ClusterNodeInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNodeInfo {
    pub node_id: String,
    pub address: String,
    pub port: u16,
    pub roles: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    pub node_id: String,
    pub term: u64,
    pub leader_id: Option<String>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub storage_usage: f64,
    pub active_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataSyncMessage {
    pub node_id: String,
    pub term: u64,
    pub shard_count: u32,
    pub total_records: u64,
    pub checksum: String,
}

// --- Sync & Reconciliation ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub node_id: String,
    pub table: String,
    pub storage_type: String,
    pub last_sync_timestamp: u64,
    pub batch_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub records: Vec<SyncRecord>,
    pub more: bool,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    pub key: String,
    pub data: serde_json::Value,
    pub version: u64,
    pub vector_clock: std::collections::HashMap<String, u64>,
    pub checksum: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleRequest {
    pub table: String,
    pub storage_type: String,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleResponse {
    pub root_hash: String,
    pub node_count: u32,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolveMessage {
    pub key: String,
    pub table: String,
    pub storage_type: String,
    pub resolution: String,
    pub resolved_data: serde_json::Value,
    pub term: u64,
}

// --- RPC Client ---

#[derive(Debug)]
pub struct RpcClient {
    node_id: String,
    addr: SocketAddr,
    stream: RwLock<Option<TcpStream>>,
    #[allow(dead_code)]
    timeout: std::time::Duration,
}

impl RpcClient {
    pub fn new(node_id: String, addr: SocketAddr) -> Self {
        Self {
            node_id,
            addr,
            stream: RwLock::new(None),
            timeout: std::time::Duration::from_secs(5),
        }
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn connect(&self) -> Result<()> {
        let stream = TcpStream::connect(self.addr).await.map_err(|e| {
            crate::Error::ClusterError(format!("RPC connect to {}: {}", self.addr, e))
        })?;
        stream.set_nodelay(true).ok();
        *self.stream.write().await = Some(stream);
        Ok(())
    }

    pub async fn disconnect(&self) {
        *self.stream.write().await = None;
    }

    pub async fn is_connected(&self) -> bool {
        self.stream.read().await.is_some()
    }

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

pub type RpcHandler = Arc<dyn Fn(RpcMessage, SocketAddr) -> Result<RpcMessage> + Send + Sync>;

pub struct RpcServer {
    bind_addr: SocketAddr,
}

impl RpcServer {
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self { bind_addr }
    }

    pub async fn start(
        self,
        handler: RpcHandler,
        shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        let listener = TcpListener::bind(self.bind_addr)
            .await
            .map_err(|e| crate::Error::ClusterError(format!("RPC bind {}: {}", self.bind_addr, e)))?;

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

/// Announce a cluster's presence in the federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedClusterAnnounce {
    pub cluster_id: String,
    pub federation_id: String,
    pub node_id: String,
    pub address: String,
    pub port: u16,
    pub cluster_size: u32,
    pub domains: Vec<String>,
    pub incarnation: u64,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedClusterAck {
    pub accepted: bool,
    pub federation_id: String,
    pub known_clusters: Vec<FedClusterInfo>,
    pub leader_hint: Option<String>,
}

/// Join a data domain across clusters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedDomainJoin {
    pub cluster_id: String,
    pub domain_name: String,
    pub node_id: String,
    pub collections: Vec<String>,
    pub storage_types: Vec<String>,
    pub replication_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedDomainJoinAck {
    pub accepted: bool,
    pub domain_name: String,
    pub members: Vec<String>,
    pub leader_hint: Option<String>,
    pub error: Option<String>,
}

/// Leave a data domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedDomainLeave {
    pub cluster_id: String,
    pub domain_name: String,
    pub node_id: String,
}

/// Cross-cluster data replication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedDataReplicaRequest {
    pub operation_id: String,
    pub domain_name: String,
    pub source_cluster: String,
    pub target_cluster: String,
    pub storage_type: String,
    pub table: String,
    pub key: String,
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub vector_clock: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedDataReplicaAck {
    pub operation_id: String,
    pub cluster_id: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Cross-cluster heartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedHeartbeatMessage {
    pub cluster_id: String,
    pub node_id: String,
    pub leader_id: Option<String>,
    pub term: u64,
    pub domain_count: u32,
    pub alive_nodes: u32,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub avg_latency_ms: f64,
    pub incarnation: u64,
}

/// Cross-cluster namespace resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedNamespaceResolveRequest {
    pub cluster_id: String,
    pub namespace_path: String,
    pub resource_name: String,
    pub storage_type: String,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedNamespaceResolveAck {
    pub request_id: String,
    pub found: bool,
    pub cluster_id: String,
    pub physical_name: Option<String>,
    pub error: Option<String>,
}

/// Information about a cluster in the federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedClusterInfo {
    pub cluster_id: String,
    pub leader_node_id: Option<String>,
    pub address: String,
    pub port: u16,
    pub cluster_size: u32,
    pub alive_count: u32,
    pub domains: Vec<String>,
    pub incarnation: u64,
    pub status: String,
    pub avg_latency_ms: f64,
    pub region: Option<String>,
}

// ==================== Federated Raft (Cross-Cluster Consensus) Messages ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedRaftVoteRequest {
    pub term: u64,
    pub candidate_id: String,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedRaftVoteResponse {
    pub term: u64,
    pub vote_granted: bool,
    pub cluster_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedRaftAppendRequest {
    pub term: u64,
    pub leader_id: String,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<FedRaftLogEntry>,
    pub leader_commit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedRaftAppendResponse {
    pub term: u64,
    pub success: bool,
    pub match_index: u64,
    pub cluster_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedRaftLogEntry {
    pub index: u64,
    pub term: u64,
    pub op_type: String,
    pub data: Vec<u8>,
    pub timestamp: u64,
}
