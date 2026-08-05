//! Cluster & distributed coordination layer
//!
//! Implements the distributed control plane for PrimusDB: how nodes discover
//! each other, elect a leader, replicate writes, partition data into shards,
//! and federate across independent clusters.
//!
//! # Topology concepts
//!
//! - **Membership discovery** - [`membership::MembershipManager`] uses SWIM-style
//!   gossip (ping / ping-req) and seed-server joins to maintain a liveness view
//!   (`Alive` / `Suspect` / `Dead`) of the cluster.
//! - **Raft leader election** - [`raft::RaftNode`] implements a simplified Raft
//!   state machine (follower / candidate / leader). Only the elected leader may
//!   replicate log entries, which it does with append-entries RPCs.
//! - **Replication** - [`replication::ReplicationEngine`] fans writes out to
//!   replica nodes synchronously, asynchronously, or until a quorum acks.
//! - **Sharding** - [`shard::ShardManager`] keeps a consistent-hash ring that
//!   maps keys to shards; each shard has one primary and several replicas.
//! - **Federation** - [`federation::FederationManager`] links independent
//!   clusters into a federation so data domains and namespaces can span clusters.
//! - **Cross-domain gateway** - [`gateway::ClusterGateway`] routes requests to
//!   the best node (round-robin, least-loaded, lowest-latency, shard-aware or
//!   domain-aware) and enforces circuit breaking per node.
//!
//! # Data flow
//!
//! ```text
//! Client ──► ClusterGateway ──► node routing (least-loaded / shard-aware)
//!                                 │
//!                                 ▼
//!   ┌──────────────────────────────────────────────────────────┐
//!   │ Node                                                      │
//!   │  MembershipManager ── gossip / join ──► peer nodes        │ discovery
//!   │  RaftNode ── vote + append-entries ──► peer RaftNodes     │ election
//!   │  ReplicationEngine ── replica writes ──► replica nodes    │ fan-out
//!   │  ShardManager ── consistent-hash ring ──► shard owners    │ partition
//!   │  FederationManager ── announce / domain ──► other clusters│ federation
//!   └──────────────────────────────────────────────────────────┘
//!                                 │
//!                                 ▼
//!                             sled storage
//! ```
//!
//! # Public types
//!
//! - [`ClusterManager`] - top-level coordinator owning membership, sharding,
//!   replication, Raft, the RPC server and the sync coordinator.
//! - [`ClusterStatusInfo`] - health snapshot reported by `get_cluster_status`.
//! - [`NodeResources`], [`NodeRole`] - per-node resource and role descriptors.
//! - [`ShardMigration`] - planned shard move between two nodes.
//! - [`FailoverAction`], [`FailoverActionType`], [`ActionPriority`] - failover
//!   planning structures.
//!
//! RPC message wire types, federation messages and reconciliation types are
//! re-exported from [`rpc`], [`federation`], [`domain`], [`gateway`] and
//! [`sync`].

use crate::cluster::membership::{ClusterMember, MemberStatus, MembershipManager};
use crate::cluster::raft::RaftNode;
use crate::cluster::replication::ReplicationEngine;
use crate::cluster::rpc::{ClusterNodeInfo, RpcHandler, RpcMessage, RpcServer};
use crate::cluster::shard::{ShardManager, ShardMigrationPlan};
use crate::{ClusterConfig, PrimusDBConfig, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

pub mod domain;
pub mod federated_raft;
pub mod federation;
pub mod gateway;
pub mod membership;
pub mod raft;
pub mod replication;
pub mod rpc;
pub mod shard;
pub mod sync;
pub use domain::*;
pub use federation::*;
pub use gateway::*;
pub use sync::*;

/// Top-level cluster coordinator for a PrimusDB node.
///
/// Owns the distributed control-plane components: membership discovery, the
/// Raft replication state machine, the replication engine, shard management,
/// the inter-node RPC server, and the sync coordinator.
///
/// A `ClusterManager` runs standalone unless `ClusterConfig::enabled` is set;
/// when enabled it opens a sled-backed cluster state database, registers itself
/// with membership, connects to seed servers, runs the gossip loop, and starts
/// Raft leader election.
pub struct ClusterManager {
    /// Cluster configuration this manager was created with
    pub config: ClusterConfig,
    /// Unique identifier of this node within the cluster
    pub node_id: String,
    /// IP address this node binds for cluster RPC
    pub address: String,
    /// TCP port this node's RPC server listens on
    pub port: u16,
    /// Gossip-based membership manager tracking peer liveness
    pub membership: Arc<MembershipManager>,
    /// Consistent-hash shard manager partitioning data across nodes
    pub shard_manager: Arc<ShardManager>,
    /// Replication engine fanning writes out to replicas
    pub replication: Arc<ReplicationEngine>,
    /// Raft consensus node; `None` until [`ClusterManager::start`] runs
    pub raft_node: Option<Arc<RaftNode>>,
    /// Handle of the background RPC server task, if running
    pub rpc_server: Option<Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>>,
    /// sled database used to persist cluster state (peers, terms, shards)
    pub db: Option<sled::Db>,
    /// Whether the cluster manager finished initialization
    pub initialized: Arc<RwLock<bool>>,
    /// Channel used by Raft to deliver committed log entries for applying
    pub apply_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Whether the peer set has been discovered and wired up
    pub peers_initialized: Arc<RwLock<bool>>,
    /// Optional coordinator for cross-node sync and reconciliation
    pub sync_coordinator: Option<Arc<SyncCoordinator>>,
}

impl ClusterManager {
    /// Create a new cluster manager for `bind_addr`.
    ///
    /// Constructs the membership manager, shard manager, replication engine and
    /// the Raft apply channel. No network activity happens until
    /// [`start`](Self::start) is called.
    pub fn new(config: &ClusterConfig, bind_addr: SocketAddr) -> Result<Self> {
        let node_id = config.node_id.clone();
        let address = bind_addr.ip().to_string();
        let port = bind_addr.port();

        let discovery_servers = config.discovery_servers.clone();

        let membership = Arc::new(MembershipManager::new(
            node_id.clone(),
            bind_addr,
            discovery_servers,
        ));

        let shard_manager = Arc::new(ShardManager::new(node_id.clone()));

        let clients = Arc::new(RwLock::new(HashMap::new()));

        let replication_config = crate::cluster::replication::ReplicationConfig::default();
        let replication = Arc::new(ReplicationEngine::new(
            node_id.clone(),
            replication_config,
            clients.clone(),
        ));

        let (apply_tx, _apply_rx) = mpsc::unbounded_channel();

        let mgr = Self {
            config: config.clone(),
            node_id: node_id.clone(),
            address: address.clone(),
            port,
            membership,
            shard_manager,
            replication,
            raft_node: None,
            rpc_server: None,
            db: None,
            initialized: Arc::new(RwLock::new(false)),
            apply_tx,
            peers_initialized: Arc::new(RwLock::new(false)),
            sync_coordinator: None,
        };

        Ok(mgr)
    }

    /// Open (or create) the sled-backed cluster state database and restore any
    /// persisted peers into the membership table.
    pub async fn init_db(&mut self, config: &PrimusDBConfig) -> Result<()> {
        let data_dir = format!("{}/cluster", config.storage.data_dir);
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| crate::Error::ClusterError(format!("Create cluster dir: {}", e)))?;

        let db = sled::open(&data_dir)
            .map_err(|e| crate::Error::ClusterError(format!("Open cluster db: {}", e)))?;
        self.db = Some(db);

        // Restore persisted state
        self.restore_state().await?;
        Ok(())
    }

    async fn restore_state(&self) -> Result<()> {
        if let Some(db) = &self.db {
            if let Some(peers_bytes) = db
                .get("peers")
                .map_err(|e| crate::Error::ClusterError(format!("DB read peers: {}", e)))?
            {
                let stored: Vec<ClusterNodeInfo> =
                    bincode::deserialize(&peers_bytes).unwrap_or_default();
                for node in &stored {
                    let member = ClusterMember {
                        node_id: node.node_id.clone(),
                        address: node.address.clone(),
                        port: node.port,
                        status: MemberStatus::Alive,
                        incarnation: 1,
                        last_seen: now_ms(),
                        roles: node.roles.clone(),
                        cpu_usage: 0.0,
                        memory_usage: 0.0,
                        storage_usage: 0.0,
                    };
                    self.membership.add_member(member).await;
                }
                info!("Restored {} peers from cluster state", stored.len());
            }
        }
        Ok(())
    }

    async fn persist_peers(&self) -> Result<()> {
        if let Some(db) = &self.db {
            let members = self.membership.members.read().await;
            let nodes: Vec<ClusterNodeInfo> = members
                .values()
                .map(|m| ClusterNodeInfo {
                    node_id: m.node_id.clone(),
                    address: m.address.clone(),
                    port: m.port,
                    roles: m.roles.clone(),
                    status: format!("{:?}", m.status),
                })
                .collect();
            let data = bincode::serialize(&nodes)
                .map_err(|e| crate::Error::ClusterError(format!("Serialize peers: {}", e)))?;
            db.insert("peers", data)
                .map_err(|e| crate::Error::ClusterError(format!("DB write peers: {}", e)))?;
            db.flush()
                .map_err(|e| crate::Error::ClusterError(format!("DB flush: {}", e)))?;
        }
        Ok(())
    }

    /// Boot the cluster: initialize state, register this node, start the RPC
    /// server, connect to seed servers, run the gossip loop, and initialize Raft.
    ///
    /// When `ClusterConfig::enabled` is `false` the node stays standalone and
    /// this method returns immediately.
    pub async fn start(&mut self, config: &PrimusDBConfig) -> Result<()> {
        if !self.config.enabled {
            info!("Cluster mode disabled, running as standalone node");
            return Ok(());
        }

        self.init_db(config).await?;

        info!(
            "Starting cluster manager for node {} on {}:{}",
            self.node_id, self.address, self.port
        );

        self.membership.register_self().await;

        let bind_addr: SocketAddr = format!("{}:{}", self.address, self.port)
            .parse()
            .map_err(|e| crate::Error::ClusterError(format!("Bind addr: {}", e)))?;

        let handler = self.create_rpc_handler();

        let (_, shutdown_rx) = mpsc::channel::<()>(1);
        let server = RpcServer::new(bind_addr);
        let server_handle = tokio::spawn(async move {
            if let Err(e) = server.start(handler, shutdown_rx).await {
                error!("RPC server error: {}", e);
            }
        });
        self.rpc_server = Some(Arc::new(tokio::sync::Mutex::new(Some(server_handle))));

        // Connect to seed servers for discovery
        if !self.config.discovery_servers.is_empty() {
            if let Err(e) = self.membership.connect_to_seeds().await {
                warn!("Failed to connect to seeds: {}", e);
            }
        }

        // Start gossip loop
        let membership = self.membership.clone();
        tokio::spawn(async move {
            membership.start_gossip_loop().await;
        });

        // Initialize Raft with peers
        self.init_raft().await?;

        // Initialize shard manager with known nodes
        {
            let members = self.membership.members.read().await;
            for member in members.keys() {
                self.shard_manager.add_node(member).await;
            }
        }

        // Persist initial state
        self.persist_peers().await?;

        *self.initialized.write().await = true;
        info!("Cluster manager initialized successfully");
        Ok(())
    }

    fn create_rpc_handler(&self) -> RpcHandler {
        let node_id = self.node_id.clone();
        let membership = self.membership.clone();
        let shard_manager = self.shard_manager.clone();
        let raft = self.raft_node.clone();
        let peers_initialized = self.peers_initialized.clone();

        Arc::new(
            move |msg: RpcMessage, addr: std::net::SocketAddr| -> Result<RpcMessage> {
                let rt = tokio::runtime::Handle::current();
                match msg {
                    RpcMessage::JoinRequest(req) => {
                        let members = rt.block_on(async { membership.members.read().await });
                        let accepted_nodes: Vec<ClusterNodeInfo> = members
                            .values()
                            .map(|m| ClusterNodeInfo {
                                node_id: m.node_id.clone(),
                                address: m.address.clone(),
                                port: m.port,
                                roles: m.roles.clone(),
                                status: format!("{:?}", m.status),
                            })
                            .collect();
                        let resp = rt.block_on(membership.handle_join(&req, &accepted_nodes));
                        rt.block_on(shard_manager.add_node(&req.node_id));
                        rt.block_on(shard_manager.persist_shards());
                        {
                            let w = rt.block_on(peers_initialized.write());
                            drop(w);
                        }
                        Ok(RpcMessage::JoinResponse(resp))
                    }
                    RpcMessage::Ping(ping) => {
                        if ping.sender_id != node_id {
                            rt.block_on(membership.mark_alive(&ping.sender_id, ping.incarnation));
                        }
                        Ok(RpcMessage::Ack(crate::cluster::rpc::AckMessage {
                            sender_id: node_id.clone(),
                            sequence: ping.sequence,
                            ok: true,
                        }))
                    }
                    RpcMessage::PingReq(req) => {
                        let target_exists = rt.block_on(async {
                            membership.get_member(&req.target_id).await.is_some()
                        });
                        Ok(RpcMessage::Ack(crate::cluster::rpc::AckMessage {
                            sender_id: node_id.clone(),
                            sequence: req.sequence,
                            ok: target_exists,
                        }))
                    }
                    RpcMessage::Heartbeat(hb) => {
                        if hb.node_id != node_id {
                            rt.block_on(membership.mark_alive(&hb.node_id, 1));
                        }
                        Ok(RpcMessage::Heartbeat(
                            crate::cluster::rpc::HeartbeatMessage {
                                node_id: node_id.clone(),
                                term: 0,
                                leader_id: None,
                                cpu_usage: 0.0,
                                memory_usage: 0.0,
                                storage_usage: 0.0,
                                active_connections: 0,
                            },
                        ))
                    }
                    RpcMessage::RequestVote(req) => {
                        if let Some(ref raft_node) = raft {
                            let resp = rt.block_on(raft_node.handle_request_vote(&req))?;
                            Ok(RpcMessage::VoteResponse(
                                crate::cluster::rpc::RaftVoteResponse {
                                    term: resp.term,
                                    vote_granted: resp.vote_granted,
                                    node_id: resp.node_id,
                                },
                            ))
                        } else {
                            Ok(RpcMessage::VoteResponse(
                                crate::cluster::rpc::RaftVoteResponse {
                                    term: 0,
                                    vote_granted: false,
                                    node_id: node_id.clone(),
                                },
                            ))
                        }
                    }
                    RpcMessage::AppendEntries(req) => {
                        if let Some(ref raft_node) = raft {
                            let resp = rt.block_on(raft_node.handle_append_entries(&req))?;
                            Ok(RpcMessage::AppendEntriesResponse(
                                crate::cluster::rpc::RaftAppendResponse {
                                    term: resp.term,
                                    success: resp.success,
                                    match_index: resp.match_index,
                                    node_id: resp.node_id,
                                    last_log_index: resp.last_log_index,
                                },
                            ))
                        } else {
                            Ok(RpcMessage::AppendEntriesResponse(
                                crate::cluster::rpc::RaftAppendResponse {
                                    term: 0,
                                    success: false,
                                    match_index: 0,
                                    node_id: node_id.clone(),
                                    last_log_index: 0,
                                },
                            ))
                        }
                    }
                    RpcMessage::ReplicaWrite(req) => {
                        // Local write acknowledgment (storage engine write handled by caller)
                        Ok(RpcMessage::ReplicaWriteAck(
                            crate::cluster::rpc::ReplicaWriteAck {
                                operation_id: req.operation_id,
                                node_id: node_id.clone(),
                                success: true,
                                term: req.term,
                            },
                        ))
                    }
                    RpcMessage::ReplicaRead(_req) => Ok(RpcMessage::ReplicaReadResponse(
                        crate::cluster::rpc::ReplicaReadResponse {
                            found: false,
                            data: None,
                            node_id: node_id.clone(),
                        },
                    )),
                    RpcMessage::MetadataSync(_) => Ok(RpcMessage::MetadataSync(
                        crate::cluster::rpc::MetadataSyncMessage {
                            node_id: node_id.clone(),
                            term: 0,
                            shard_count: 0,
                            total_records: 0,
                            checksum: String::new(),
                        },
                    )),
                    RpcMessage::ShardTransfer(req) => {
                        info!("Received shard transfer request for shard {}", req.shard_id);
                        Ok(RpcMessage::ShardTransfer(req))
                    }
                    _ => {
                        debug!("Unhandled RPC message from {}", addr);
                        Err(crate::Error::ClusterError("Unhandled RPC type".into()))
                    }
                }
            },
        )
    }

    async fn init_raft(&mut self) -> Result<()> {
        let clients_raft = self.replication.clients.clone();
        let members = self.membership.members.read().await;
        let mut peers = HashMap::new();

        for member in members.values() {
            if member.node_id != self.node_id && matches!(member.status, MemberStatus::Alive) {
                let addr: SocketAddr = format!("{}:{}", member.address, member.port)
                    .parse()
                    .map_err(|e| crate::Error::ClusterError(format!("Peer addr: {}", e)))?;
                let client = Arc::new(crate::cluster::rpc::RpcClient::new(
                    member.node_id.clone(),
                    addr,
                ));
                if client.connect().await.is_ok() {
                    peers.insert(member.node_id.clone(), client.clone());
                    clients_raft
                        .write()
                        .await
                        .insert(member.node_id.clone(), client);
                }
            }
        }

        let raft_config = ConsensusConfig::default();
        let raft = Arc::new(RaftNode::new(
            self.node_id.clone(),
            raft_config,
            peers,
            self.apply_tx.clone(),
        ));

        // Start Raft background loop
        let r = raft.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if r.is_leader().await {
                    if let Err(e) = r.send_heartbeats().await {
                        debug!("Heartbeat error: {}", e);
                    }
                } else if r.leader_id().await.is_none() {
                    if let Err(e) = r.start_election().await {
                        debug!("Election error: {}", e);
                    }
                }
            }
        });

        self.raft_node = Some(raft);
        *self.peers_initialized.write().await = true;
        Ok(())
    }

    /// Submit an operation to the Raft log for cluster-wide ordering.
    ///
    /// # Errors
    /// Returns an error if Raft is uninitialized, this node is not the leader,
    /// or the entry cannot be replicated to a quorum.
    pub async fn propose(&self, op_type: &str, data: serde_json::Value) -> Result<bool> {
        if let Some(ref raft) = self.raft_node {
            if !raft.is_leader().await {
                let leader = raft.leader_id().await;
                return Err(crate::Error::ClusterError(format!(
                    "Not the leader, leader is {:?}",
                    leader
                )));
            }
            let term = raft.current_term().await;
            let entry = crate::cluster::raft::create_log_entry(term, op_type, data);
            raft.replicate_entry(entry).await?;
            Ok(true)
        } else {
            Err(crate::Error::ClusterError("Raft not initialized".into()))
        }
    }

    /// Add a node to membership and shard management and persist the peer set.
    pub async fn register_node(
        &self,
        node_id: &str,
        address: &str,
        port: u16,
        roles: Vec<String>,
    ) -> Result<()> {
        let member = ClusterMember {
            node_id: node_id.to_string(),
            address: address.to_string(),
            port,
            status: MemberStatus::Alive,
            incarnation: 1,
            last_seen: now_ms(),
            roles,
            cpu_usage: 0.0,
            memory_usage: 0.0,
            storage_usage: 0.0,
        };
        self.membership.add_member(member).await;
        self.shard_manager.add_node(node_id).await;
        self.persist_peers().await?;
        Ok(())
    }

    /// Mark a node dead, remove it from shard management, and persist the peer set.
    pub async fn remove_node(&self, node_id: &str) -> Result<()> {
        self.membership.mark_dead(node_id).await;
        self.shard_manager.remove_node(node_id).await;
        self.persist_peers().await?;
        Ok(())
    }

    /// Report the current cluster health snapshot (size, leader, health status).
    pub async fn get_cluster_status(&self) -> ClusterStatusInfo {
        let members = self.membership.alive_members().await;
        let alive_count = members.len() + 1;

        let is_leader = if let Some(ref r) = self.raft_node {
            r.is_leader().await
        } else {
            false
        };

        let leader_id = if let Some(ref r) = self.raft_node {
            r.leader_id().await
        } else {
            None
        };

        ClusterStatusInfo {
            node_id: self.node_id.clone(),
            cluster_size: alive_count,
            alive_count,
            is_leader,
            leader_id,
            health_status: if alive_count >= 3 {
                "Healthy"
            } else if alive_count > 1 {
                "Degraded"
            } else {
                "Standalone"
            },
            replication_factor: 3,
            uptime_ms: 0,
        }
    }
}

impl ClusterManager {
    /// Deterministically pick a node for an operation type by hashing it across
    /// the current member set.
    pub async fn get_node_for_operation(&self, operation_type: &str) -> Option<String> {
        let members = self.membership.members.read().await;
        if members.is_empty() {
            return None;
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        operation_type.hash(&mut hasher);
        let idx = (hasher.finish() as usize) % members.len();
        let node_id = members.keys().nth(idx)?.clone();
        Some(node_id)
    }

    /// Ask the shard manager whether shards need to be moved to balance load.
    pub async fn check_rebalance_needed(&self) -> Vec<ShardMigrationPlan> {
        self.shard_manager.check_rebalance_needed().await
    }
}

/// Snapshot of cluster health and leadership reported by
/// [`ClusterManager::get_cluster_status`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatusInfo {
    /// ID of the reporting node
    pub node_id: String,
    /// Total number of nodes in the cluster (including this node)
    pub cluster_size: usize,
    /// Number of nodes currently considered alive
    pub alive_count: usize,
    /// Whether this node is the current Raft leader
    pub is_leader: bool,
    /// ID of the current leader, if known
    pub leader_id: Option<String>,
    /// Human-readable health classification
    pub health_status: &'static str,
    /// Configured replication factor
    pub replication_factor: u32,
    /// Uptime of the cluster manager in milliseconds
    pub uptime_ms: u64,
}

/// Resource snapshot for a node (capacities and current usage).
#[derive(Debug, Clone)]
pub struct NodeResources {
    /// Number of CPU cores available
    pub cpu_cores: u32,
    /// Total memory in GB
    pub memory_gb: u64,
    /// Total storage in GB
    pub storage_gb: u64,
    /// Current CPU usage as a fraction (0.0-1.0)
    pub cpu_usage: f64,
    /// Current memory usage as a fraction (0.0-1.0)
    pub memory_usage: f64,
    /// Current storage usage as a fraction (0.0-1.0)
    pub storage_usage: f64,
}

/// Role a node can play in the cluster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeRole {
    /// Coordinates cluster-wide operations
    Coordinator,
    /// Executes work on behalf of the cluster
    Worker,
    /// Stores data shards
    Storage,
    /// Serves the external API
    Api,
}

/// A planned shard movement from a source node to a target node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMigration {
    /// ID of the shard being moved
    pub shard_id: String,
    /// Node currently hosting the shard
    pub source_node: String,
    /// Node that should host the shard
    pub target_node: String,
    /// Estimated time for the migration in milliseconds
    pub estimated_time_ms: u64,
}

/// A concrete action to take when a node or role fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverAction {
    /// Kind of failover action to perform
    pub action_type: FailoverActionType,
    /// Node the action applies to
    pub target_node: String,
    /// Human-readable description of the action
    pub description: String,
    /// How urgent the action is
    pub priority: ActionPriority,
}

/// Kind of failover action to perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailoverActionType {
    /// Promote a replica to primary
    PromoteReplica,
    /// Elect a new coordinator
    ElectNewCoordinator,
    /// Redistribute data across nodes
    RedistributeData,
    /// Restart a failed service
    RestartService,
}

/// Priority of a failover action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionPriority {
    /// Must be handled immediately
    Critical,
    /// Should be handled soon
    High,
    /// Best-effort handling
    Medium,
    /// Handle when convenient
    Low,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// Helper for Raft's apply channel
/// Create a channel used to deliver committed Raft log entries for applying.
///
/// The sender is held by the [`ClusterManager`] (and its [`RaftNode`]); the
/// receiver is consumed by the caller that applies committed entries to storage.
pub fn create_apply_channel() -> (
    mpsc::UnboundedSender<Vec<u8>>,
    mpsc::UnboundedReceiver<Vec<u8>>,
) {
    mpsc::unbounded_channel()
}
