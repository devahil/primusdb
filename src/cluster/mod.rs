use crate::{ClusterConfig, PrimusDBConfig, Result};
use crate::cluster::rpc::{ClusterNodeInfo, RpcMessage, RpcServer, RpcHandler};
use crate::cluster::membership::{ClusterMember, MemberStatus, MembershipManager};
use crate::cluster::raft::RaftNode;
use crate::cluster::shard::{ShardManager, ShardMigrationPlan};
use crate::cluster::replication::ReplicationEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

pub mod rpc;
pub mod raft;
pub mod membership;
pub mod shard;
pub mod replication;
pub mod gateway;
pub mod sync;
pub mod federation;
pub mod domain;
pub mod federated_raft;
pub use sync::*;
pub use gateway::*;
pub use federation::*;
pub use domain::*;

pub struct ClusterManager {
    pub config: ClusterConfig,
    pub node_id: String,
    pub address: String,
    pub port: u16,
    pub membership: Arc<MembershipManager>,
    pub shard_manager: Arc<ShardManager>,
    pub replication: Arc<ReplicationEngine>,
    pub raft_node: Option<Arc<RaftNode>>,
    pub rpc_server: Option<Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>>,
    pub db: Option<sled::Db>,
    pub initialized: Arc<RwLock<bool>>,
    #[allow(dead_code)]
    pub apply_tx: mpsc::UnboundedSender<Vec<u8>>,
    #[allow(dead_code)]
    pub apply_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>,
    pub peers_initialized: Arc<RwLock<bool>>,
    pub sync_coordinator: Option<Arc<SyncCoordinator>>,
}

impl ClusterManager {
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

        let (apply_tx, apply_rx) = mpsc::unbounded_channel();

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
            apply_rx: Arc::new(tokio::sync::Mutex::new(apply_rx)),
            peers_initialized: Arc::new(RwLock::new(false)),
            sync_coordinator: None,
        };

        Ok(mgr)
    }

    pub async fn init_db(&mut self, config: &PrimusDBConfig) -> Result<()> {
        let data_dir = format!("{}/cluster", config.storage.data_dir);
        std::fs::create_dir_all(&data_dir).map_err(|e|
            crate::Error::ClusterError(format!("Create cluster dir: {}", e))
        )?;

        let db = sled::open(&data_dir).map_err(|e|
            crate::Error::ClusterError(format!("Open cluster db: {}", e))
        )?;
        self.db = Some(db);

        // Restore persisted state
        self.restore_state().await?;
        Ok(())
    }

    async fn restore_state(&self) -> Result<()> {
        if let Some(db) = &self.db {
            if let Some(peers_bytes) = db.get("peers").map_err(|e|
                crate::Error::ClusterError(format!("DB read peers: {}", e))
            )? {
                let stored: Vec<ClusterNodeInfo> = bincode::deserialize(&peers_bytes)
                    .unwrap_or_default();
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
            db.insert("peers", data).map_err(|e|
                crate::Error::ClusterError(format!("DB write peers: {}", e))
            )?;
            db.flush().map_err(|e|
                crate::Error::ClusterError(format!("DB flush: {}", e))
            )?;
        }
        Ok(())
    }

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

        let bind_addr: SocketAddr = format!("{}:{}", self.address, self.port).parse()
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

        Arc::new(move |msg: RpcMessage, addr: std::net::SocketAddr| -> Result<RpcMessage> {
            let rt = tokio::runtime::Handle::current();
            match msg {
                RpcMessage::JoinRequest(req) => {
                    let members = rt.block_on(async {
                        membership.members.read().await
                    });
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
                    let _ = rt.block_on(shard_manager.persist_shards());
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
                    Ok(RpcMessage::Heartbeat(crate::cluster::rpc::HeartbeatMessage {
                        node_id: node_id.clone(),
                        term: 0,
                        leader_id: None,
                        cpu_usage: 0.0,
                        memory_usage: 0.0,
                        storage_usage: 0.0,
                        active_connections: 0,
                    }))
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
                RpcMessage::ReplicaRead(_req) => {
                    Ok(RpcMessage::ReplicaReadResponse(
                        crate::cluster::rpc::ReplicaReadResponse {
                            found: false,
                            data: None,
                            node_id: node_id.clone(),
                        },
                    ))
                }
                RpcMessage::MetadataSync(_) => {
                    Ok(RpcMessage::MetadataSync(
                        crate::cluster::rpc::MetadataSyncMessage {
                            node_id: node_id.clone(),
                            term: 0,
                            shard_count: 0,
                            total_records: 0,
                            checksum: String::new(),
                        },
                    ))
                }
                RpcMessage::ShardTransfer(req) => {
                    info!("Received shard transfer request for shard {}", req.shard_id);
                    Ok(RpcMessage::ShardTransfer(req))
                }
                _ => {
                    debug!("Unhandled RPC message from {}", addr);
                    Err(crate::Error::ClusterError("Unhandled RPC type".into()))
                }
            }
        })
    }

    async fn init_raft(&mut self) -> Result<()> {
        let clients_raft = self.replication.clients.clone();
        let members = self.membership.members.read().await;
        let mut peers = HashMap::new();

        for member in members.values() {
            if member.node_id != self.node_id
                && matches!(member.status, MemberStatus::Alive)
            {
                let addr: SocketAddr = format!("{}:{}", member.address, member.port).parse()
                    .map_err(|e| crate::Error::ClusterError(format!("Peer addr: {}", e)))?;
                let client = Arc::new(
                    crate::cluster::rpc::RpcClient::new(member.node_id.clone(), addr)
                );
                if client.connect().await.is_ok() {
                    peers.insert(member.node_id.clone(), client.clone());
                    clients_raft.write().await.insert(member.node_id.clone(), client);
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

    pub async fn remove_node(&self, node_id: &str) -> Result<()> {
        self.membership.mark_dead(node_id).await;
        self.shard_manager.remove_node(node_id).await;
        self.persist_peers().await?;
        Ok(())
    }

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
            health_status: if alive_count >= 3 { "Healthy" }
                else if alive_count > 1 { "Degraded" }
                else { "Standalone" },
            replication_factor: 3,
            uptime_ms: 0,
        }
    }
}

impl ClusterManager {
    pub fn get_node_for_operation(&self, _operation_type: &str) -> Option<String> {
        // Use consistent hash ring to find nodes
        None // Simplified; actual routing uses shard manager
    }

    pub async fn check_rebalance_needed(&self) -> Vec<ShardMigrationPlan> {
        self.shard_manager.check_rebalance_needed().await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatusInfo {
    pub node_id: String,
    pub cluster_size: usize,
    pub alive_count: usize,
    pub is_leader: bool,
    pub leader_id: Option<String>,
    pub health_status: &'static str,
    pub replication_factor: u32,
    pub uptime_ms: u64,
}

#[derive(Debug, Clone)]
pub struct NodeResources {
    pub cpu_cores: u32,
    pub memory_gb: u64,
    pub storage_gb: u64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub storage_usage: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeRole {
    Coordinator,
    Worker,
    Storage,
    Api,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMigration {
    pub shard_id: String,
    pub source_node: String,
    pub target_node: String,
    pub estimated_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverAction {
    pub action_type: FailoverActionType,
    pub target_node: String,
    pub description: String,
    pub priority: ActionPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailoverActionType {
    PromoteReplica,
    ElectNewCoordinator,
    RedistributeData,
    RestartService,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionPriority {
    Critical,
    High,
    Medium,
    Low,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// Helper for Raft's apply channel
pub fn create_apply_channel() -> (mpsc::UnboundedSender<Vec<u8>>, mpsc::UnboundedReceiver<Vec<u8>>) {
    mpsc::unbounded_channel()
}
