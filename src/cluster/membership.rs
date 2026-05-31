use crate::Result;
use crate::cluster::rpc::{
    ClusterNodeInfo, JoinRequest, JoinResponse, PingMessage, PingReqMessage,
    RpcClient, RpcMessage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemberStatus {
    Alive,
    Suspect,
    Dead,
    Left,
}

#[derive(Debug, Clone)]
pub struct ClusterMember {
    pub node_id: String,
    pub address: String,
    pub port: u16,
    pub status: MemberStatus,
    pub incarnation: u64,
    pub last_seen: u64,
    pub roles: Vec<String>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub storage_usage: f64,
}

#[derive(Debug, Clone)]
pub struct MembershipConfig {
    pub gossip_interval_ms: u64,
    pub probe_interval_ms: u64,
    pub suspect_timeout_ms: u64,
    pub gossip_fanout: usize,
    pub ping_req_count: usize,
    pub cleanup_interval_ms: u64,
}

impl Default for MembershipConfig {
    fn default() -> Self {
        Self {
            gossip_interval_ms: 1000,
            probe_interval_ms: 5000,
            suspect_timeout_ms: 10000,
            gossip_fanout: 3,
            ping_req_count: 3,
            cleanup_interval_ms: 30000,
        }
    }
}

#[derive(Debug)]
pub struct MembershipManager {
    pub node_id: String,
    pub bind_addr: SocketAddr,
    pub config: MembershipConfig,
    pub members: RwLock<HashMap<String, ClusterMember>>,
    pub clients: RwLock<HashMap<String, Arc<RpcClient>>>,
    pub local_seq: RwLock<u64>,
    pub running: RwLock<bool>,
    pub seed_servers: Vec<String>,
}

impl MembershipManager {
    pub fn new(
        node_id: String,
        bind_addr: SocketAddr,
        seed_servers: Vec<String>,
    ) -> Self {
        Self {
            node_id,
            bind_addr,
            config: MembershipConfig::default(),
            members: RwLock::new(HashMap::new()),
            clients: RwLock::new(HashMap::new()),
            local_seq: RwLock::new(0),
            running: RwLock::new(true),
            seed_servers,
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    pub async fn add_member(&self, member: ClusterMember) {
        let mut members = self.members.write().await;
        let entry = members.entry(member.node_id.clone()).or_insert_with(|| member.clone());
        if member.incarnation > entry.incarnation || entry.status == MemberStatus::Dead {
            *entry = member;
        }
    }

    pub async fn get_member(&self, node_id: &str) -> Option<ClusterMember> {
        self.members.read().await.get(node_id).cloned()
    }

    pub async fn alive_members(&self) -> Vec<ClusterMember> {
        self.members
            .read()
            .await
            .values()
            .filter(|m| matches!(m.status, MemberStatus::Alive))
            .cloned()
            .collect()
    }

    pub async fn suspect_members(&self) -> Vec<ClusterMember> {
        self.members
            .read()
            .await
            .values()
            .filter(|m| matches!(m.status, MemberStatus::Suspect))
            .cloned()
            .collect()
    }

    pub async fn mark_suspect(&self, node_id: &str, incarnation: u64) {
        let mut members = self.members.write().await;
        if let Some(member) = members.get_mut(node_id) {
            if incarnation >= member.incarnation {
                member.status = MemberStatus::Suspect;
                member.incarnation = incarnation;
            }
        }
    }

    pub async fn mark_alive(&self, node_id: &str, incarnation: u64) {
        let mut members = self.members.write().await;
        if let Some(member) = members.get_mut(node_id) {
            if incarnation >= member.incarnation {
                member.status = MemberStatus::Alive;
                member.incarnation = incarnation;
                member.last_seen = Self::now();
            }
        }
    }

    pub async fn mark_dead(&self, node_id: &str) {
        let mut members = self.members.write().await;
        if let Some(member) = members.get_mut(node_id) {
            member.status = MemberStatus::Dead;
        }
    }

    pub async fn register_self(&self) {
        let mut members = self.members.write().await;
        members.insert(
            self.node_id.clone(),
            ClusterMember {
                node_id: self.node_id.clone(),
                address: self.bind_addr.ip().to_string(),
                port: self.bind_addr.port(),
                status: MemberStatus::Alive,
                incarnation: 1,
                last_seen: Self::now(),
                roles: vec!["Coordinator".into(), "Worker".into(), "Storage".into()],
                cpu_usage: 0.0,
                memory_usage: 0.0,
                storage_usage: 0.0,
            },
        );
    }

    pub async fn connect_to_seeds(&self) -> Result<()> {
        for seed in &self.seed_servers {
            let addr: SocketAddr = match seed.parse() {
                Ok(a) => a,
                Err(_) => {
                    warn!("Invalid seed address: {}", seed);
                    continue;
                }
            };
            let client = Arc::new(RpcClient::new(format!("seed@{}", seed), addr));
            if client.connect().await.is_ok() {
                let join_req = RpcMessage::JoinRequest(JoinRequest {
                    node_id: self.node_id.clone(),
                    address: self.bind_addr.ip().to_string(),
                    port: self.bind_addr.port(),
                    cpu_cores: num_cpus::get() as u32,
                    memory_gb: 0,
                    storage_gb: 0,
                    roles: vec!["Worker".into(), "Storage".into()],
                });
                match client.send(&join_req).await {
                    Ok(RpcMessage::JoinResponse(resp)) => {
                        if resp.accepted {
                            info!("Joined cluster via seed {}", seed);

                            for node in &resp.cluster_nodes {
                                let member = ClusterMember {
                                    node_id: node.node_id.clone(),
                                    address: node.address.clone(),
                                    port: node.port,
                                    status: MemberStatus::Alive,
                                    incarnation: 1,
                                    last_seen: Self::now(),
                                    roles: node.roles.clone(),
                                    cpu_usage: 0.0,
                                    memory_usage: 0.0,
                                    storage_usage: 0.0,
                                };
                                self.add_member(member).await;

                                let naddr: SocketAddr =
                                    format!("{}:{}", node.address, node.port).parse().unwrap();
                                let peer_client =
                                    Arc::new(RpcClient::new(node.node_id.clone(), naddr));
                                if peer_client.connect().await.is_ok() {
                                    self.clients
                                        .write()
                                        .await
                                        .insert(node.node_id.clone(), peer_client);
                                }
                            }
                        }
                    }
                    _ => {
                        debug!("Join via seed {} not accepted", seed);
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn start_gossip_loop(&self) {
        let probe_interval = self.config.probe_interval_ms;
        let gossip_interval = self.config.gossip_interval_ms;
        let suspect_timeout = self.config.suspect_timeout_ms;
        let cleanup_interval = self.config.cleanup_interval_ms;

        loop {
            if !*self.running.read().await {
                break;
            }

            // Probe a random member
            self.probe_random_member().await;

            // Check suspects
            self.check_suspects(suspect_timeout).await;

            // Gossip membership
            self.gossip_membership().await;

            // Cleanup dead members
            self.cleanup_dead_members(cleanup_interval).await;

            tokio::time::sleep(Duration::from_millis(
                probe_interval.min(gossip_interval),
            ))
            .await;
        }
    }

    async fn probe_random_member(&self) {
        let alive: Vec<ClusterMember> = self.alive_members().await;
        let target = alive
            .iter()
            .filter(|m| m.node_id != self.node_id)
            .max_by_key(|m| m.last_seen);

        if let Some(target) = target {
            let seq = {
                let mut s = self.local_seq.write().await;
                *s += 1;
                *s
            };

            let ping = RpcMessage::Ping(PingMessage {
                sender_id: self.node_id.clone(),
                sequence: seq,
                incarnation: 1,
            });

            let client = self.clients.read().await.get(&target.node_id).cloned();
            if let Some(client) = client {
                match client.send(&ping).await {
                    Ok(RpcMessage::Ack(ack)) if ack.ok => {
                        self.mark_alive(&target.node_id, 1).await;
                    }
                    _ => {
                        self.mark_suspect(&target.node_id, 1).await;
                        self.ping_req(&target.node_id, seq).await;
                    }
                }
            } else {
                self.mark_suspect(&target.node_id, 1).await;
            }
        }
    }

    async fn ping_req(&self, target_id: &str, seq: u64) {
        let alive: Vec<ClusterMember> = self.alive_members().await;
        let alt_nodes: Vec<&ClusterMember> = alive
            .iter()
            .filter(|m| m.node_id != self.node_id && m.node_id != target_id)
            .take(self.config.ping_req_count)
            .collect();

        for alt in &alt_nodes {
            let client = self.clients.read().await.get(&alt.node_id).cloned();
            if let Some(client) = client {
                let msg = RpcMessage::PingReq(PingReqMessage {
                    sender_id: self.node_id.clone(),
                    target_id: target_id.to_string(),
                    sequence: seq,
                    incarnation: 1,
                });
                if let Ok(RpcMessage::Ack(ack)) = client.send(&msg).await {
                    if ack.ok {
                        self.mark_alive(target_id, 1).await;
                        return;
                    }
                }
            }
        }

        self.mark_dead(target_id).await;
        info!("Node {} marked dead after failed ping-req", target_id);
    }

    async fn check_suspects(&self, timeout_ms: u64) {
        let now = Self::now();
        let suspects: Vec<ClusterMember> = self.suspect_members().await;
        for member in suspects {
            if now - member.last_seen > timeout_ms {
                self.mark_dead(&member.node_id).await;
                info!("Suspect {} timed out, marking dead", member.node_id);
            }
        }
    }

    async fn gossip_membership(&self) {
        let members_snapshot: Vec<ClusterMember> = {
            self.members
                .read()
                .await
                .values()
                .filter(|m| matches!(m.status, MemberStatus::Alive | MemberStatus::Suspect))
                .take(10)
                .cloned()
                .collect()
        };

        let clients = self.clients.read().await;
        for client in clients.values() {
            for member in &members_snapshot {
                // Send membership updates as heartbeat metadata
                let meta = RpcMessage::MetadataSync(
                    crate::cluster::rpc::MetadataSyncMessage {
                        node_id: member.node_id.clone(),
                        term: 0,
                        shard_count: 0,
                        total_records: 0,
                        checksum: String::new(),
                    },
                );
                let _ = client.send(&meta).await;
            }
        }
    }

    async fn cleanup_dead_members(&self, cleanup_interval_ms: u64) {
        static mut LAST_CLEANUP: u64 = 0;
        let now = Self::now();
        unsafe {
            if now - LAST_CLEANUP < cleanup_interval_ms {
                return;
            }
            LAST_CLEANUP = now;
        }

        let mut members = self.members.write().await;
        let dead: Vec<String> = members
            .iter()
            .filter(|(_, m)| matches!(m.status, MemberStatus::Dead | MemberStatus::Left))
            .map(|(k, _)| k.clone())
            .collect();
        for id in dead {
            members.remove(&id);
            self.clients.write().await.remove(&id);
        }
    }

    pub async fn handle_join(
        &self,
        req: &JoinRequest,
        accepted_nodes: &[ClusterNodeInfo],
    ) -> JoinResponse {
        let member = ClusterMember {
            node_id: req.node_id.clone(),
            address: req.address.clone(),
            port: req.port,
            status: MemberStatus::Alive,
            incarnation: 1,
            last_seen: Self::now(),
            roles: req.roles.clone(),
            cpu_usage: 0.0,
            memory_usage: 0.0,
            storage_usage: 0.0,
        };
        self.add_member(member).await;

        JoinResponse {
            accepted: true,
            leader_id: Some(self.node_id.clone()),
            cluster_nodes: accepted_nodes.to_vec(),
            error: None,
        }
    }
}
