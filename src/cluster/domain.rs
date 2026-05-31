use crate::cluster::rpc::{FedDataReplicaRequest, FedDataReplicaAck, RpcClient, RpcMessage};
use crate::cluster::federation::FederationManager;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DomainReplicationMode {
    Sync,
    Async,
    Quorum,
}

impl Default for DomainReplicationMode {
    fn default() -> Self {
        Self::Sync
    }
}

impl DomainReplicationMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sync" => Self::Sync,
            "async" => Self::Async,
            "quorum" => Self::Quorum,
            _ => Self::Sync,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDomain {
    pub name: String,
    pub description: String,
    pub replication_mode: DomainReplicationMode,
    pub storage_types: Vec<String>,
    pub collections: Vec<String>,
    pub tables: Vec<String>,
    pub member_clusters: Vec<String>,
    pub leader_cluster: Option<String>,
    pub created_at: u64,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainReplicaStatus {
    pub domain_name: String,
    pub cluster_id: String,
    pub last_sync_ms: u64,
    pub records_pending: u64,
    pub lag_ms: u64,
    pub healthy: bool,
}

pub struct DataDomainManager {
    pub cluster_id: String,
    pub domains: RwLock<HashMap<String, DataDomain>>,
    pub replica_status: RwLock<HashMap<String, DomainReplicaStatus>>,
    pub pending_replications: RwLock<HashMap<String, FedDataReplicaRequest>>,
    pub rpc_clients: RwLock<HashMap<String, Arc<RpcClient>>>,
    pub federation: Option<Arc<FederationManager>>,
}

impl DataDomainManager {
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

    pub fn with_federation(mut self, fed: Arc<FederationManager>) -> Self {
        self.federation = Some(fed);
        self
    }

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
        info!("DataDomain '{}' created with {} members", name, member_count);
        Ok(domain)
    }

    pub async fn get_domain(&self, name: &str) -> Option<DataDomain> {
        self.domains.read().await.get(name).cloned()
    }

    pub async fn list_domains(&self) -> Vec<DataDomain> {
        self.domains.read().await.values().cloned().collect()
    }

    pub async fn delete_domain(&self, name: &str) {
        let mut domains = self.domains.write().await;
        domains.remove(name);
        info!("DataDomain '{}' deleted", name);
    }

    pub async fn add_cluster_to_domain(
        &self,
        domain_name: &str,
        cluster_id: &str,
    ) -> Result<()> {
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

    pub async fn remove_cluster_from_domain(
        &self,
        domain_name: &str,
        cluster_id: &str,
    ) -> Result<()> {
        let mut domains = self.domains.write().await;
        if let Some(domain) = domains.get_mut(domain_name) {
            domain.member_clusters.retain(|c| c != cluster_id);
            domain.version += 1;
            info!(
                "Cluster '{}' left DataDomain '{}'",
                cluster_id, domain_name
            );
        }
        Ok(())
    }

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
            None => return Err(crate::Error::ClusterError(format!("Domain '{}' not found", domain_name))),
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
                        warn!("Target cluster '{}' not found in federation for domain replication", target);
                        continue;
                    }
                }
            } else {
                warn!("Federation not configured for cross-cluster replication to '{}'", target);
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

    pub async fn handle_replica_request(
        &self,
        req: FedDataReplicaRequest,
    ) -> FedDataReplicaAck {
        debug!(
            "Received cross-cluster replica for domain '{}', key '{}' from '{}'",
            req.domain_name, req.key, req.source_cluster
        );

        FedDataReplicaAck {
            operation_id: req.operation_id,
            cluster_id: self.cluster_id.clone(),
            success: true,
            error: None,
        }
    }

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

    /// Check if a storage type + collection pair belongs to any domain.
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

    /// Find all domains whose `storage_types` and `collections`/`tables` match
    /// the given write.  Uses case-insensitive storage type comparison so that
    /// "columnar" and "Columnar" both work.
    pub async fn find_matching_domains(
        &self,
        storage_type: &str,
        table: &str,
    ) -> Vec<String> {
        let st_lower = storage_type.to_lowercase();
        self.domains
            .read()
            .await
            .values()
            .filter(|d| {
                let type_match = d
                    .storage_types
                    .iter()
                    .any(|t| t.to_lowercase() == st_lower);
                let collection_match = d.collections.is_empty()
                    || d.collections.iter().any(|c| c == table);
                let table_match = d.tables.iter().any(|t| t == table);
                type_match && (collection_match || table_match || d.collections.is_empty())
            })
            .map(|d| d.name.clone())
            .collect()
    }

    // ---- DataDomain Auto-Balance ----

    pub async fn check_balance(&self) -> Vec<DomainBalancePlan> {
        let mut plans = Vec::new();
        let domains = self.domains.read().await;

        for (name, domain) in domains.iter() {
            if domain.member_clusters.len() < 2 {
                continue;
            }

            let load_per_cluster = if let Some(ref fed) = self.federation {
                let members = fed.members.read().await;
                domain.member_clusters.iter().map(|cid| {
                    let load = members.get(cid)
                        .map(|m| m.consecutive_failures as f64 + m.info.avg_latency_ms / 100.0)
                        .unwrap_or(1.0);
                    (cid.clone(), load)
                }).collect::<Vec<_>>()
            } else {
                continue;
            };

            if load_per_cluster.len() < 2 {
                continue;
            }

            let avg_load: f64 = load_per_cluster.iter().map(|(_, l)| l).sum::<f64>()
                / load_per_cluster.len() as f64;
            let threshold = avg_load * 0.3;

            let overloaded: Vec<_> = load_per_cluster.iter()
                .filter(|(_, l)| *l > avg_load + threshold)
                .collect();

            let underloaded: Vec<_> = load_per_cluster.iter()
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
                    reason: format!("auto-balance: {} overloaded, {} underloaded (avg={:.2})",
                        overloaded.len(), underloaded.len(), avg_load),
                });
            }
        }

        plans
    }
}

#[derive(Debug, Clone)]
pub struct DomainBalancePlan {
    pub domain_name: String,
    pub moves: Vec<CollectionMove>,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct CollectionMove {
    pub collection: String,
    pub storage_type: String,
    pub from_cluster: String,
    pub to_cluster: String,
    pub estimated_cost: f64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
