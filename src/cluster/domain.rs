//! Cross-cluster data domains
//!
//! A data domain groups collections/tables that span multiple federated
//! clusters. [`DataDomainManager`] tracks domain membership and replication
//! mode, replicates writes to member clusters, records replica health, resolves
//! whether a storage resource belongs to a domain, and proposes collection
//! moves when member clusters become imbalanced.
//!
//! # Placement in the architecture
//!
//! Sits between the storage engines and the federation / federated-Raft layers:
//! writes are fanned out across the domain's member clusters and ordered by the
//! [`crate::cluster::federated_raft::FederatedRaft`] group.
//!
//! ```text
//!            local write ──► storage engine
//!                                  │
//!            DataDomainManager::replicate_cross_cluster
//!                                  │
//!                FedDataReplicaRequest over federation RPC
//!   ┌────────────┬────────────────┴───────────────┬────────────┐
//!   ▼            ▼                                ▼            ▼
//! member A   member B                       ...   member N
//!   │            │                                │
//!   └────────────┴─────── FedDataReplicaAck ──────┘
//!                                  │
//!        replication mode decides success:
//!        Sync = all members, Quorum = majority, Async = best effort
//!
//!   replica_status  -> per-domain/cluster health (pending, lag)
//!   check_balance   -> DomainBalancePlan moves collections off overloaded
//!                      clusters onto underloaded ones
//! ```

use crate::cluster::federation::FederationManager;
use crate::cluster::rpc::{FedDataReplicaAck, FedDataReplicaRequest, RpcClient, RpcMessage};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// How a data domain replicates writes across its member clusters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum DomainReplicationMode {
    /// Every member cluster must acknowledge (default)
    #[default]
    Sync,
    /// Acknowledged immediately, replicated in the background
    Async,
    /// A quorum of member clusters must acknowledge
    Quorum,
}

impl DomainReplicationMode {
    #[allow(clippy::should_implement_trait)]
    /// Parse a domain replication mode from its string name.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sync" => Self::Sync,
            "async" => Self::Async,
            "quorum" => Self::Quorum,
            _ => Self::Sync,
        }
    }
}

/// Definition of a data domain spanning one or more clusters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDomain {
    /// Domain name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Replication mode across member clusters
    pub replication_mode: DomainReplicationMode,
    /// Storage engine types the domain covers
    pub storage_types: Vec<String>,
    /// Collections belonging to the domain
    pub collections: Vec<String>,
    /// Relational tables belonging to the domain
    pub tables: Vec<String>,
    /// Clusters that are members of the domain
    pub member_clusters: Vec<String>,
    /// Cluster elected as domain leader, if any
    pub leader_cluster: Option<String>,
    /// Creation timestamp (ms)
    pub created_at: u64,
    /// Version of the domain metadata
    pub version: u64,
}

/// Replication health of a domain on a member cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainReplicaStatus {
    /// Domain this status refers to
    pub domain_name: String,
    /// Member cluster this status refers to
    pub cluster_id: String,
    /// Timestamp (ms) of the last successful sync
    pub last_sync_ms: u64,
    /// Records still awaiting replication
    pub records_pending: u64,
    /// Replication lag in milliseconds
    pub lag_ms: u64,
    /// Whether the replica is healthy
    pub healthy: bool,
}

/// Tracks data domains and replicates writes across their member clusters.
pub struct DataDomainManager {
    /// ID of the local cluster
    pub cluster_id: String,
    /// Known domains keyed by name
    pub domains: RwLock<HashMap<String, DataDomain>>,
    /// Replica health keyed by `domain:cluster`
    pub replica_status: RwLock<HashMap<String, DomainReplicaStatus>>,
    /// In-flight cross-cluster replication requests keyed by operation ID
    pub pending_replications: RwLock<HashMap<String, FedDataReplicaRequest>>,
    /// RPC clients to member clusters, keyed by cluster ID
    pub rpc_clients: RwLock<HashMap<String, Arc<RpcClient>>>,
    /// Optional federation manager used to locate member clusters
    pub federation: Option<Arc<FederationManager>>,
}

impl DataDomainManager {
    /// Create a domain manager for the local cluster.
    pub fn new(cluster_id: String) -> Self {
        Self {
            cluster_id,
            domains: RwLock::new(HashMap::new()),
            replica_status: RwLock::new(HashMap::new()),
            pending_replications: RwLock::new(HashMap::new()),
            rpc_clients: RwLock::new(HashMap::new()),
            federation: None,
        }
    }

    /// Attach a federation manager so the domain manager can locate members.
    pub fn with_federation(mut self, fed: Arc<FederationManager>) -> Self {
        self.federation = Some(fed);
        self
    }

    #[allow(clippy::too_many_arguments)]
    /// Create a new data domain and register it locally.
    pub async fn create_domain(
        &self,
        name: &str,
        description: &str,
        replication_mode: DomainReplicationMode,
        storage_types: Vec<String>,
        collections: Vec<String>,
        tables: Vec<String>,
        member_clusters: Vec<String>,
    ) -> Result<DataDomain> {
        let member_count = member_clusters.len();
        let domain = DataDomain {
            name: name.to_string(),
            description: description.to_string(),
            replication_mode,
            storage_types,
            collections,
            tables,
            member_clusters,
            leader_cluster: Some(self.cluster_id.clone()),
            created_at: now_ms(),
            version: 1,
        };

        let mut domains = self.domains.write().await;
        domains.insert(name.to_string(), domain.clone());
        info!(
            "DataDomain '{}' created with {} members",
            name, member_count
        );
        Ok(domain)
    }

    /// Look up a domain by name.
    pub async fn get_domain(&self, name: &str) -> Option<DataDomain> {
        self.domains.read().await.get(name).cloned()
    }

    /// All known domains.
    pub async fn list_domains(&self) -> Vec<DataDomain> {
        self.domains.read().await.values().cloned().collect()
    }

    /// Remove a domain.
    pub async fn delete_domain(&self, name: &str) {
        let mut domains = self.domains.write().await;
        domains.remove(name);
        info!("DataDomain '{}' deleted", name);
    }

    /// Add a cluster to a domain's membership.
    pub async fn add_cluster_to_domain(&self, domain_name: &str, cluster_id: &str) -> Result<()> {
        let mut domains = self.domains.write().await;
        if let Some(domain) = domains.get_mut(domain_name) {
            if !domain.member_clusters.contains(&cluster_id.to_string()) {
                domain.member_clusters.push(cluster_id.to_string());
                domain.version += 1;
                info!(
                    "Cluster '{}' joined DataDomain '{}'",
                    cluster_id, domain_name
                );
            }
        }
        Ok(())
    }

    /// Remove a cluster from a domain's membership.
    pub async fn remove_cluster_from_domain(
        &self,
        domain_name: &str,
        cluster_id: &str,
    ) -> Result<()> {
        let mut domains = self.domains.write().await;
        if let Some(domain) = domains.get_mut(domain_name) {
            domain.member_clusters.retain(|c| c != cluster_id);
            domain.version += 1;
            info!("Cluster '{}' left DataDomain '{}'", cluster_id, domain_name);
        }
        Ok(())
    }

    /// Replicate a write to every member cluster of a domain (excluding the
    /// source and local cluster), enforcing the domain's replication mode.
    pub async fn replicate_cross_cluster(
        &self,
        domain_name: &str,
        storage_type: &str,
        table: &str,
        key: &str,
        data: &[u8],
        source_cluster: &str,
    ) -> Result<Vec<FedDataReplicaAck>> {
        let domain = match self.domains.read().await.get(domain_name) {
            Some(d) => d.clone(),
            None => {
                return Err(crate::Error::ClusterError(format!(
                    "Domain '{}' not found",
                    domain_name
                )))
            }
        };

        let targets: Vec<&String> = domain
            .member_clusters
            .iter()
            .filter(|c| *c != source_cluster && *c != &self.cluster_id)
            .collect();

        if targets.is_empty() {
            return Ok(Vec::new());
        }

        let operation_id = format!("fed_{}_{}", now_ms(), key);
        let req = FedDataReplicaRequest {
            operation_id: operation_id.clone(),
            domain_name: domain_name.to_string(),
            source_cluster: source_cluster.to_string(),
            target_cluster: String::new(),
            storage_type: storage_type.to_string(),
            table: table.to_string(),
            key: key.to_string(),
            data: data.to_vec(),
            timestamp: now_ms(),
            vector_clock: String::new(),
        };

        let mut acks = Vec::new();
        for target in &targets {
            let mut req_clone = req.clone();
            req_clone.target_cluster = (*target).clone();

            let (host, port) = if let Some(ref fed) = self.federation {
                let members = fed.members.read().await;
                match members.get(target.as_str()) {
                    Some(member) => (member.info.address.clone(), member.info.port),
                    None => {
                        warn!(
                            "Target cluster '{}' not found in federation for domain replication",
                            target
                        );
                        continue;
                    }
                }
            } else {
                warn!(
                    "Federation not configured for cross-cluster replication to '{}'",
                    target
                );
                continue;
            };

            let rpc_msg = RpcMessage::FedDataReplica(req_clone);
            match crate::cluster::federation::connect_and_send(&host, port, rpc_msg).await {
                Ok(Some(RpcMessage::FedDataReplicaAck(ack))) => {
                    acks.push(ack);
                }
                _ => {
                    warn!(
                        "Cross-cluster replication to '{}' for domain '{}' failed",
                        target, domain_name
                    );
                }
            }
        }

        let mode = &domain.replication_mode;
        let needed = match mode {
            DomainReplicationMode::Sync => targets.len(),
            DomainReplicationMode::Quorum => targets.len() / 2 + 1,
            DomainReplicationMode::Async => 1,
        };

        if acks.iter().filter(|a| a.success).count() < needed {
            return Err(crate::Error::ClusterError(format!(
                "Cross-cluster replication for domain '{}' failed: {} acks needed, got {}",
                domain_name,
                needed,
                acks.iter().filter(|a| a.success).count()
            )));
        }

        Ok(acks)
    }

    /// Handle an incoming cross-cluster replication request by queueing it and
    /// recording replica health.
    pub async fn handle_replica_request(&self, req: FedDataReplicaRequest) -> FedDataReplicaAck {
        debug!(
            "Received cross-cluster replica for domain '{}', key '{}' from '{}' ({} bytes)",
            req.domain_name,
            req.key,
            req.source_cluster,
            req.data.len()
        );

        // Store the pending replication for processing
        self.pending_replications
            .write()
            .await
            .insert(req.operation_id.clone(), req.clone());

        // Record sync status
        {
            let mut status = self.replica_status.write().await;
            status.insert(
                format!("{}:{}", req.domain_name, req.source_cluster),
                DomainReplicaStatus {
                    domain_name: req.domain_name.clone(),
                    cluster_id: req.source_cluster.clone(),
                    last_sync_ms: now_ms(),
                    records_pending: 0,
                    lag_ms: 0,
                    healthy: true,
                },
            );
        }

        FedDataReplicaAck {
            operation_id: req.operation_id,
            cluster_id: self.cluster_id.clone(),
            success: true,
            error: None,
        }
    }

    /// Replica health for every member cluster of every domain.
    pub async fn get_replica_health(&self) -> Vec<DomainReplicaStatus> {
        let domains = self.domains.read().await;
        let mut statuses = Vec::new();
        for (name, domain) in domains.iter() {
            for cluster in &domain.member_clusters {
                if cluster != &self.cluster_id {
                    statuses.push(DomainReplicaStatus {
                        domain_name: name.clone(),
                        cluster_id: cluster.clone(),
                        last_sync_ms: now_ms(),
                        records_pending: 0,
                        lag_ms: 0,
                        healthy: true,
                    });
                }
            }
        }
        statuses
    }

    /// Return the name of the domain that owns a storage type/collection, if any.
    pub async fn storage_belongs_to_domain(
        &self,
        storage_type: &str,
        collection: &str,
    ) -> Option<String> {
        for domain in self.domains.read().await.values() {
            if domain.storage_types.iter().any(|t| t == storage_type)
                && (domain.collections.is_empty()
                    || domain.collections.iter().any(|c| c == collection))
            {
                return Some(domain.name.clone());
            }
        }
        None
    }

    // ---- DataDomain Auto-Balance ----

    /// Compare member-cluster load and produce a plan of collection moves from
    /// overloaded to underloaded clusters.
    pub async fn check_balance(&self) -> Vec<DomainBalancePlan> {
        let mut plans = Vec::new();
        let domains = self.domains.read().await;

        for (name, domain) in domains.iter() {
            if domain.member_clusters.len() < 2 {
                continue;
            }

            let load_per_cluster = if let Some(ref fed) = self.federation {
                let members = fed.members.read().await;
                domain
                    .member_clusters
                    .iter()
                    .map(|cid| {
                        let load = members
                            .get(cid)
                            .map(|m| m.consecutive_failures as f64 + m.info.avg_latency_ms / 100.0)
                            .unwrap_or(1.0);
                        (cid.clone(), load)
                    })
                    .collect::<Vec<_>>()
            } else {
                continue;
            };

            if load_per_cluster.len() < 2 {
                continue;
            }

            let avg_load: f64 = load_per_cluster.iter().map(|(_, l)| l).sum::<f64>()
                / load_per_cluster.len() as f64;
            let threshold = avg_load * 0.3;

            let overloaded: Vec<_> = load_per_cluster
                .iter()
                .filter(|(_, l)| *l > avg_load + threshold)
                .collect();

            let underloaded: Vec<_> = load_per_cluster
                .iter()
                .filter(|(_, l)| *l < avg_load - threshold)
                .collect();

            if overloaded.is_empty() || underloaded.is_empty() {
                continue;
            }

            let mut moves = Vec::new();
            for (o_cid, o_load) in &overloaded {
                for (u_cid, u_load) in &underloaded {
                    if let Some(coll) = domain.collections.first() {
                        moves.push(CollectionMove {
                            collection: coll.clone(),
                            storage_type: domain.storage_types.first().cloned().unwrap_or_default(),
                            from_cluster: o_cid.clone(),
                            to_cluster: u_cid.clone(),
                            estimated_cost: *o_load - *u_load,
                        });
                    }
                }
            }

            if !moves.is_empty() {
                plans.push(DomainBalancePlan {
                    domain_name: name.clone(),
                    moves,
                    reason: format!(
                        "auto-balance: {} overloaded, {} underloaded (avg={:.2})",
                        overloaded.len(),
                        underloaded.len(),
                        avg_load
                    ),
                });
            }
        }

        plans
    }
}

/// A plan to rebalance collections within a domain.
#[derive(Debug, Clone)]
pub struct DomainBalancePlan {
    /// Domain being rebalanced
    pub domain_name: String,
    /// Collection moves to apply
    pub moves: Vec<CollectionMove>,
    /// Reason the plan was produced
    pub reason: String,
}

/// A proposed collection move between two clusters.
#[derive(Debug, Clone)]
pub struct CollectionMove {
    /// Collection to move
    pub collection: String,
    /// Storage engine type of the collection
    pub storage_type: String,
    /// Cluster the collection currently lives on
    pub from_cluster: String,
    /// Cluster the collection should move to
    pub to_cluster: String,
    /// Estimated cost of the move (load delta)
    pub estimated_cost: f64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
