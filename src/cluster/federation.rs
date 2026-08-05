//! Multi-cluster federation
//!
//! Links independent PrimusDB clusters into a federation so that data domains
//! and namespaces can span multiple clusters. [`FederationManager`] announces
//! the local cluster, tracks remote cluster liveness with heartbeats and
//! suspect/offline transitions, brokers cross-cluster domain membership, and
//! resolves federated namespaces. RPC helpers such as [`connect_and_send`]
//! provide the wire transport used by the federation and federated-Raft layers.
//!
//! # Placement in the architecture
//!
//! `FederationManager` is the cross-cluster counterpart of the intra-cluster
//! [`crate::cluster::membership::MembershipManager`]: it tracks remote clusters
//! instead of remote nodes. It hands member addresses to the
//! [`crate::cluster::domain::DataDomainManager`] (for replica fan-out) and to
//! the [`crate::cluster::federated_raft::FederatedRaft`] group (for quorum
//! exchanges) through [`connect_and_send`].
//!
//! ```text
//!   ┌──────────────────────────────────── Federation ─────────────────────────────────────┐
//!   │                                                                                     │
//!   ┌─────────────┐ announce/heartbeat  ┌─────────────┐ announce/heartbeat  ┌─────────────┐│
//!   │ Cluster A   │◄───────────────────►│ Cluster B   │◄───────────────────►│ Cluster C   ││
//!   │ Federation  │ FedClusterAnnounce  │ Federation  │ FedClusterAnnounce  │ Federation  ││
//!   │ Manager     │ FedHeartbeat        │ Manager     │ FedHeartbeat        │ Manager     ││
//!   └─────────────┘                     └─────────────┘                     └─────────────┘│
//!        │   ▲                          │        ▲                           │             │
//!        │   │ domain join/leave        │        │ resolve federated namespaces            │
//!        ▼   │ data replication         ▼        │                                             │
//!   DataDomainManager ◄──connect_and_send──► FederatedRaft (cross-cluster consensus)      │
//!   └───────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::cluster::rpc::{
    FedClusterAck, FedClusterAnnounce, FedClusterInfo, FedDomainJoin, FedDomainJoinAck,
    FedDomainLeave, FedHeartbeatMessage, FedNamespaceResolveRequest, RpcClient, RpcMessage,
};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Configuration for a PrimusDB federation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    /// Identifier of the federation this cluster belongs to
    pub federation_id: String,
    /// Identifier of the local cluster
    pub cluster_id: String,
    /// How often to broadcast cluster announcements (ms)
    pub announce_interval_ms: u64,
    /// How often to send federation heartbeats (ms)
    pub heartbeat_interval_ms: u64,
    /// Timeout before a missed heartbeat is treated as a failure (ms)
    pub heartbeat_timeout_ms: u64,
    /// Timeout before a silent member becomes suspect, then offline (ms)
    pub suspect_timeout_ms: u64,
    /// Maximum number of clusters tracked in the federation
    pub max_clusters: u32,
    /// Whether replicas may be stored on clusters outside the local region
    pub enable_cross_cluster_replication: bool,
    /// Whether namespaces may resolve across cluster boundaries
    pub enable_federated_namespaces: bool,
    /// Optional region identifier for this cluster
    pub region: Option<String>,
    /// Path to the TLS certificate file for mTLS federation communication
    #[serde(default)]
    pub tls_cert_path: String,
    /// Path to the TLS private key file for mTLS federation communication
    #[serde(default)]
    pub tls_key_path: String,
    /// Path to the CA certificate file for verifying peer federation nodes
    #[serde(default)]
    pub tls_ca_path: String,
    /// Whether to require mTLS for federation RPC communication
    #[serde(default)]
    pub mtls_enabled: bool,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            federation_id: "default".into(),
            cluster_id: String::new(),
            announce_interval_ms: 10_000,
            heartbeat_interval_ms: 5_000,
            heartbeat_timeout_ms: 3_000,
            suspect_timeout_ms: 30_000,
            max_clusters: 64,
            enable_cross_cluster_replication: true,
            enable_federated_namespaces: true,
            region: None,
            tls_cert_path: String::new(),
            tls_key_path: String::new(),
            tls_ca_path: String::new(),
            mtls_enabled: false,
        }
    }
}

/// Liveness state of a remote cluster in the federation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClusterStatus {
    /// Heartbeating normally
    Online,
    /// Unresponsive, awaiting confirmation of failure
    Suspect,
    /// Declared unreachable
    Offline,
    /// Voluntarily left the federation
    Left,
}

/// View of a remote cluster tracked by the local [`FederationManager`].
#[derive(Debug, Clone)]
pub struct FederationMember {
    /// Announcement information from the remote cluster
    pub info: FedClusterInfo,
    /// Current liveness status
    pub status: ClusterStatus,
    /// Timestamp (ms) of the last successful contact
    pub last_seen: u64,
    /// Incarnation counter used to detect restarts
    pub incarnation: u64,
    /// Consecutive failed heartbeats/announces
    pub consecutive_failures: u32,
    /// Optional cached RPC client to the remote cluster
    pub rpc_client: Option<Arc<RpcClient>>,
}

/// Tracks remote clusters in a federation and brokers cross-cluster operations.
pub struct FederationManager {
    /// Federation configuration
    pub config: FederationConfig,
    /// Known remote clusters keyed by cluster ID
    pub members: RwLock<HashMap<String, FederationMember>>,
    /// Data domains hosted by the local cluster
    pub local_domains: RwLock<Vec<String>>,
    /// Cache of resolved federated namespace paths
    pub namespace_cache: RwLock<HashMap<String, String>>,
    /// Whether background announce/heartbeat tasks are running
    pub running: RwLock<bool>,
    /// Local incarnation counter
    incarnation: RwLock<u64>,
}

impl FederationManager {
    /// Create a federation manager for the given configuration.
    pub fn new(config: FederationConfig) -> Self {
        Self {
            config,
            members: RwLock::new(HashMap::new()),
            local_domains: RwLock::new(Vec::new()),
            namespace_cache: RwLock::new(HashMap::new()),
            running: RwLock::new(false),
            incarnation: RwLock::new(1),
        }
    }

    /// Broadcast the local cluster's presence to all online federation members
    /// and fold any newly discovered clusters into the membership table.
    pub async fn announce_cluster(
        &self,
        address: &str,
        port: u16,
        node_id: &str,
        cluster_size: u32,
    ) -> Result<()> {
        let inc = {
            let mut inc = self.incarnation.write().await;
            *inc += 1;
            *inc
        };

        let domains = self.local_domains.read().await.clone();
        let announce = FedClusterAnnounce {
            cluster_id: self.config.cluster_id.clone(),
            federation_id: self.config.federation_id.clone(),
            node_id: node_id.to_string(),
            address: address.to_string(),
            port,
            cluster_size,
            domains,
            incarnation: inc,
            capabilities: vec![
                "raft".into(),
                "swim".into(),
                "sharding".into(),
                "replication".into(),
                "namespaces".into(),
            ],
        };

        let members = self.members.read().await;
        let targets: Vec<(String, String, u16)> = members
            .iter()
            .filter(|(_, m)| m.status == ClusterStatus::Online)
            .map(|(id, m)| (id.clone(), m.info.address.clone(), m.info.port))
            .collect();
        drop(members);

        for (cid, host, p) in &targets {
            match connect_and_send(host, *p, RpcMessage::FedClusterAnnounce(announce.clone())).await
            {
                Ok(Some(RpcMessage::FedClusterAck(ack))) => {
                    let mut members = self.members.write().await;
                    if let Some(member) = members.get_mut(cid) {
                        member.status = ClusterStatus::Online;
                        member.last_seen = now_ms();
                        member.consecutive_failures = 0;
                    }
                    for remote in &ack.known_clusters {
                        if remote.cluster_id != self.config.cluster_id
                            && !members.contains_key(&remote.cluster_id)
                        {
                            members.insert(
                                remote.cluster_id.clone(),
                                FederationMember {
                                    info: remote.clone(),
                                    status: ClusterStatus::Online,
                                    last_seen: now_ms(),
                                    incarnation: remote.incarnation,
                                    consecutive_failures: 0,
                                    rpc_client: None,
                                },
                            );
                        }
                    }
                }
                _ => {
                    let mut members = self.members.write().await;
                    if let Some(member) = members.get_mut(cid) {
                        member.consecutive_failures += 1;
                        if member.consecutive_failures > 3 {
                            member.status = ClusterStatus::Suspect;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Register a remote cluster received via an announce and acknowledge it
    /// with the list of other clusters this node already knows.
    pub async fn register_remote_cluster(&self, info: FedClusterInfo) -> Result<FedClusterAck> {
        let mut members = self.members.write().await;
        let known: Vec<FedClusterInfo> = members
            .iter()
            .filter(|(id, _)| *id != &info.cluster_id)
            .map(|(_, m)| m.info.clone())
            .collect();

        members.insert(
            info.cluster_id.clone(),
            FederationMember {
                info: info.clone(),
                status: ClusterStatus::Online,
                last_seen: now_ms(),
                incarnation: info.incarnation,
                consecutive_failures: 0,
                rpc_client: None,
            },
        );

        Ok(FedClusterAck {
            accepted: true,
            federation_id: self.config.federation_id.clone(),
            known_clusters: known,
            leader_hint: None,
        })
    }

    /// Ask every online remote cluster to join the local cluster into the given
    /// data domain, returning an acknowledgment with the resulting membership.
    pub async fn join_domain(
        &self,
        domain_name: &str,
        collections: Vec<String>,
        storage_types: Vec<String>,
        replication_mode: &str,
    ) -> Result<FedDomainJoinAck> {
        let join = FedDomainJoin {
            cluster_id: self.config.cluster_id.clone(),
            domain_name: domain_name.to_string(),
            node_id: format!("{}:{}", self.config.cluster_id, "gateway"),
            collections,
            storage_types,
            replication_mode: replication_mode.to_string(),
        };

        let mut ack = FedDomainJoinAck {
            accepted: false,
            domain_name: domain_name.to_string(),
            members: vec![self.config.cluster_id.clone()],
            leader_hint: None,
            error: None,
        };

        let members = self.members.read().await;
        for (cid, member) in members.iter() {
            if cid == &self.config.cluster_id {
                continue;
            }
            if member.status != ClusterStatus::Online {
                continue;
            }
            match connect_and_send(
                &member.info.address,
                member.info.port,
                RpcMessage::FedDomainJoin(join.clone()),
            )
            .await
            {
                Ok(Some(RpcMessage::FedDomainJoinAck(remote_ack))) => {
                    if remote_ack.accepted {
                        ack.accepted = true;
                        ack.members.extend(remote_ack.members);
                        ack.leader_hint = remote_ack.leader_hint;
                    }
                }
                _ => {
                    warn!("Failed to join domain {} on cluster {}", domain_name, cid);
                }
            }
        }

        let mut domains = self.local_domains.write().await;
        if !domains.contains(&domain_name.to_string()) {
            domains.push(domain_name.to_string());
        }

        if ack.members.len() > 1 {
            ack.accepted = true;
        }

        Ok(ack)
    }

    /// Remove the local cluster from a data domain and notify remote members.
    pub async fn leave_domain(&self, domain_name: &str) -> Result<()> {
        let leave = FedDomainLeave {
            cluster_id: self.config.cluster_id.clone(),
            node_id: format!("{}:{}", self.config.cluster_id, "gateway"),
            domain_name: domain_name.to_string(),
        };

        self.local_domains
            .write()
            .await
            .retain(|d| d != domain_name);

        for (cid, member) in self.members.read().await.iter() {
            if cid == &self.config.cluster_id {
                continue;
            }
            if member.status != ClusterStatus::Online {
                continue;
            }
            let _ = connect_and_send(
                &member.info.address,
                member.info.port,
                RpcMessage::FedDomainLeave(leave.clone()),
            )
            .await;
        }

        Ok(())
    }

    /// Handle a remote cluster's request to join a data domain hosted locally.
    pub async fn handle_domain_join(&self, req: FedDomainJoin) -> FedDomainJoinAck {
        let mut domains = self.local_domains.write().await;
        if !domains.contains(&req.domain_name) {
            domains.push(req.domain_name.clone());
        }

        let members = self.members.read().await;
        let member_ids: Vec<String> = members
            .iter()
            .filter(|(_, m)| {
                m.status == ClusterStatus::Online && m.info.domains.contains(&req.domain_name)
            })
            .map(|(id, _)| id.clone())
            .collect();

        FedDomainJoinAck {
            accepted: true,
            domain_name: req.domain_name,
            members: member_ids,
            leader_hint: None,
            error: None,
        }
    }

    /// Handle a remote cluster's request to leave a data domain hosted locally.
    pub async fn handle_domain_leave(&self, req: crate::cluster::rpc::FedDomainLeave) {
        let mut domains = self.local_domains.write().await;
        domains.retain(|d| d != &req.domain_name);
    }

    /// Update the tracked member for an incoming federation heartbeat.
    pub async fn handle_heartbeat(&self, hb: FedHeartbeatMessage) {
        let mut members = self.members.write().await;
        if let Some(member) = members.get_mut(&hb.cluster_id) {
            member.info.alive_count = hb.alive_nodes;
            member.info.avg_latency_ms = hb.avg_latency_ms;
            member.last_seen = now_ms();
            member.status = ClusterStatus::Online;
            member.consecutive_failures = 0;
            member.info.incarnation = hb.incarnation;
        }
    }

    /// Send a heartbeat to every online federation member.
    pub async fn send_heartbeat(&self, node_id: &str) -> Result<()> {
        let members = self.members.read().await;
        let targets: Vec<(String, String, u16)> = members
            .iter()
            .filter(|(_, m)| m.status == ClusterStatus::Online)
            .map(|(id, m)| (id.clone(), m.info.address.clone(), m.info.port))
            .collect();
        let domain_count = self.local_domains.read().await.len() as u32;
        drop(members);

        let hb = FedHeartbeatMessage {
            cluster_id: self.config.cluster_id.clone(),
            node_id: node_id.to_string(),
            leader_id: None,
            term: 0,
            domain_count,
            alive_nodes: 1,
            cpu_usage: 0.0,
            memory_usage: 0.0,
            avg_latency_ms: 0.0,
            incarnation: *self.incarnation.read().await,
        };

        for (cid, host, port) in &targets {
            match connect_and_send(host, *port, RpcMessage::FedHeartbeat(hb.clone())).await {
                Ok(Some(RpcMessage::FedHeartbeat(_))) => {
                    let mut members = self.members.write().await;
                    if let Some(member) = members.get_mut(cid) {
                        member.last_seen = now_ms();
                        member.status = ClusterStatus::Online;
                        member.consecutive_failures = 0;
                    }
                }
                _ => {
                    let mut members = self.members.write().await;
                    if let Some(member) = members.get_mut(cid) {
                        member.consecutive_failures += 1;
                        if member.consecutive_failures > 5 {
                            member.status = ClusterStatus::Suspect;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Transition silent members from `Online` to `Suspect`, and stale suspects
    /// to `Offline`, based on the configured suspect timeout.
    pub async fn check_suspects(&self) {
        let now = now_ms();
        let timeout = self.config.suspect_timeout_ms;
        let mut members = self.members.write().await;
        for member in members.values_mut() {
            if member.status == ClusterStatus::Online && now - member.last_seen > timeout {
                member.status = ClusterStatus::Suspect;
                warn!(
                    "Federation member {} is suspect (last seen: {}ms ago)",
                    member.info.cluster_id,
                    now - member.last_seen
                );
            }
            if member.status == ClusterStatus::Suspect && now - member.last_seen > timeout * 2 {
                member.status = ClusterStatus::Offline;
                info!(
                    "Federation member {} is now offline",
                    member.info.cluster_id
                );
            }
        }
    }

    /// Resolve a federated namespace path against remote clusters, caching the
    /// first positive result.
    pub async fn resolve_namespace_cross_cluster(
        &self,
        namespace_path: &str,
        resource_name: &str,
        storage_type: &str,
    ) -> Result<Option<String>> {
        let cache_key = format!("{}:{}:{}", namespace_path, resource_name, storage_type);
        {
            let cache = self.namespace_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(Some(cached.clone()));
            }
        }

        let req = FedNamespaceResolveRequest {
            cluster_id: self.config.cluster_id.clone(),
            namespace_path: namespace_path.to_string(),
            resource_name: resource_name.to_string(),
            storage_type: storage_type.to_string(),
            request_id: format!("ns_{}", now_ms()),
        };

        let members = self.members.read().await;
        for (cid, member) in members.iter() {
            if cid == &self.config.cluster_id {
                continue;
            }
            if member.status != ClusterStatus::Online {
                continue;
            }
            if let Ok(Some(RpcMessage::FedNamespaceResolveAck(ack))) = connect_and_send(
                &member.info.address,
                member.info.port,
                RpcMessage::FedNamespaceResolve(req.clone()),
            )
            .await
            {
                if ack.found {
                    if let Some(physical) = &ack.physical_name {
                        let mut cache = self.namespace_cache.write().await;
                        cache.insert(cache_key, physical.clone());
                        return Ok(Some(physical.clone()));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Return the announcements of all online federation members.
    pub async fn get_online_clusters(&self) -> Vec<FedClusterInfo> {
        self.members
            .read()
            .await
            .iter()
            .filter(|(_, m)| m.status == ClusterStatus::Online)
            .map(|(_, m)| m.info.clone())
            .collect()
    }

    /// Number of online federation members.
    pub async fn get_cluster_count(&self) -> usize {
        self.members
            .read()
            .await
            .iter()
            .filter(|(_, m)| m.status == ClusterStatus::Online)
            .count()
    }

    /// Spawn the background announce and heartbeat loops for this federation.
    pub async fn start_background_tasks(
        self: Arc<Self>,
        address: String,
        port: u16,
        node_id: String,
        cluster_size: u32,
    ) {
        *self.running.write().await = true;

        let node_id1 = node_id.clone();
        let node_id2 = node_id;
        let address1 = address;

        let fed = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(fed.config.announce_interval_ms)).await;

                if let Err(e) = fed
                    .announce_cluster(&address1, port, &node_id1, cluster_size)
                    .await
                {
                    warn!("Federation announce error: {}", e);
                }
                fed.check_suspects().await;
            }
        });

        let fed2 = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(fed2.config.heartbeat_interval_ms)).await;
                if let Err(e) = fed2.send_heartbeat(&node_id2).await {
                    warn!("Federation heartbeat error: {}", e);
                }
            }
        });
    }
}

/// Open a short-lived TCP connection, send one bincode-serialized [`RpcMessage`],
/// and wait for the reply.
///
/// Used by the federation and federated-Raft layers to talk to remote clusters.
/// Returns `Ok(None)` when the peer is unreachable.
pub async fn connect_and_send(
    host: &str,
    port: u16,
    msg: RpcMessage,
) -> Result<Option<RpcMessage>> {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let addr = format!("{}:{}", host, port);
    let mut stream =
        match tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(&addr)).await {
            Ok(Ok(s)) => {
                let _ = s.set_nodelay(true);
                s
            }
            _ => return Ok(None),
        };

    let data = bincode::serialize(&msg)
        .map_err(|e| crate::Error::ClusterError(format!("Federation serialize: {}", e)))?;
    let len = data.len() as u32;

    stream
        .write_all(&len.to_le_bytes())
        .await
        .map_err(|e| crate::Error::ClusterError(format!("Federation write len: {}", e)))?;
    stream
        .write_all(&data)
        .await
        .map_err(|e| crate::Error::ClusterError(format!("Federation write data: {}", e)))?;

    let mut len_buf = [0u8; 4];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut len_buf))
        .await
        .map_err(|_| crate::Error::ClusterError("Federation read len timeout".into()))?
        .map_err(|e| crate::Error::ClusterError(format!("Federation read len: {}", e)))?;

    let resp_len = u32::from_le_bytes(len_buf) as usize;
    let mut resp_buf = vec![0u8; resp_len];
    stream
        .read_exact(&mut resp_buf)
        .await
        .map_err(|e| crate::Error::ClusterError(format!("Federation read resp: {}", e)))?;

    let resp: RpcMessage = bincode::deserialize(&resp_buf)
        .map_err(|e| crate::Error::ClusterError(format!("Federation deserialize: {}", e)))?;

    Ok(Some(resp))
}

/// Current wall-clock time in milliseconds.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
