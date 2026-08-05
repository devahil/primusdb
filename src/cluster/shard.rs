//! Data sharding and consistent-hash ring
//!
//! Partitions tables into shards and places them across cluster nodes.
//! [`ShardManager`] builds a consistent-hash ring of nodes (with virtual nodes
//! for balance), resolves the shard owner(s) for any key, tracks per-shard
//! primary/replica assignments (including geo-distributed cross-region
//! replicas), detects load imbalance, and persists shard state to disk.
//!
//! # Placement in the architecture
//!
//! `ShardManager` is the partitioning layer of the cluster. Its ring decides
//! where each key lives (feeding the
//! [`crate::cluster::replication::ReplicationEngine`] its replica set), and its
//! rebalance plans drive shard migrations.
//!
//! ```text
//!   key ──hash──► consistent-hash ring (virtual nodes per physical node)
//!                         │
//!   get_shard_for_key ────┤ clockwise walk gives up to replication_factor
//!   get_nodes_for_key ────┤ owner nodes
//!                         ▼
//!        shard primary (node X) + replicas (nodes Y, Z)
//!                         │
//!     add/remove node ────► rebuild_ring (only ~1/N of keys move)
//!     load imbalance ─────► check_rebalance_needed ──► ShardMigrationPlan
//!     regions ────────────► create_geo_shard (primary + cross-region replicas)
//!     persist_shards ─────► data_dir/shards.json (sled for cluster state)
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tokio::sync::RwLock;
use tracing::info;

/// Region location for geo-distributed shards
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShardRegion {
    /// Region name
    pub name: String,
    /// Region priority (lower = more preferred)
    pub priority: u32,
}

impl ShardRegion {
    /// Create a region with the given name and placement priority (lower is
    /// preferred).
    pub fn new(name: &str, priority: u32) -> Self {
        Self {
            name: name.to_string(),
            priority,
        }
    }
}

impl Default for ShardRegion {
    fn default() -> Self {
        Self {
            name: "default".into(),
            priority: 0,
        }
    }
}

/// Metadata for one shard of a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    /// Unique shard identifier
    pub shard_id: String,
    /// Table the shard belongs to
    pub table: String,
    /// Storage engine type of the shard
    pub storage_type: String,
    /// Start of the hash range covered by this shard
    pub hash_range_start: u64,
    /// End of the hash range covered by this shard
    pub hash_range_end: u64,
    /// Node hosting the shard primary
    pub primary_node: String,
    /// Nodes hosting replicas of this shard
    pub replica_nodes: Vec<String>,
    /// Number of records in the shard
    pub record_count: u64,
    /// Estimated size of the shard in bytes
    pub size_bytes: u64,
    /// Version of the shard metadata
    pub version: u64,
    /// Primary region for this shard
    pub primary_region: ShardRegion,
    /// Cross-region replica assignments (region → list of nodes)
    pub cross_region_replicas: HashMap<String, Vec<String>>,
}

/// Overall distribution of shards across the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardDistribution {
    /// Number of shards in the ring
    pub num_shards: u32,
    /// Replication factor in use
    pub replication_factor: u32,
    /// Ordered consistent-hash ring entries
    pub ring: Vec<ShardRingEntry>,
}

/// A single point on the consistent-hash ring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardRingEntry {
    /// Hash value of the virtual node
    pub hash: u64,
    /// Physical node owning this ring point
    pub node_id: String,
    /// Shard assigned at this ring point
    pub shard_id: String,
    /// Virtual node identifier (`node_id:vnN`)
    pub virtual_node: String,
}

/// Tuning parameters for shard placement and rebalancing.
#[derive(Debug, Clone)]
pub struct ShardManagerConfig {
    /// Number of shards in the hash space
    pub num_shards: u32,
    /// Replication factor for shard replicas
    pub replication_factor: u32,
    /// Virtual nodes per physical node for ring balance
    pub virtual_nodes_per_node: u32,
    /// Load deviation threshold that triggers rebalancing
    pub rebalance_threshold: f64,
    /// Number of records migrated per batch
    pub migrate_batch_size: u32,
    /// Directory where shard state is persisted
    pub data_dir: String,
}

impl Default for ShardManagerConfig {
    fn default() -> Self {
        Self {
            num_shards: 128,
            replication_factor: 3,
            virtual_nodes_per_node: 64,
            rebalance_threshold: 0.2,
            migrate_batch_size: 100,
            data_dir: "./data/shard_state".to_string(),
        }
    }
}

/// Manages the consistent-hash ring and per-shard placement metadata.
#[derive(Debug)]
pub struct ShardManager {
    /// Shard placement tuning parameters
    pub config: ShardManagerConfig,
    /// ID of the local node
    pub node_id: String,
    /// Shard metadata keyed by `table:shard_id`
    pub shards: RwLock<HashMap<String, ShardInfo>>,
    /// Ordered consistent-hash ring
    pub ring: RwLock<Vec<ShardRingEntry>>,
    /// Nodes participating in the ring
    pub nodes: RwLock<Vec<String>>,
    /// Region membership: region_name → list of node_ids
    pub region_map: RwLock<HashMap<String, Vec<String>>>,
}

impl ShardManager {
    /// Create a shard manager for the local node.
    pub fn new(node_id: String) -> Self {
        Self {
            config: ShardManagerConfig::default(),
            node_id,
            shards: RwLock::new(HashMap::new()),
            ring: RwLock::new(Vec::new()),
            nodes: RwLock::new(Vec::new()),
            region_map: RwLock::new(HashMap::new()),
        }
    }

    /// Add a node to the ring, rebuilding the consistent-hash ring.
    pub async fn add_node(&self, node_id: &str) {
        let mut nodes = self.nodes.write().await;
        if !nodes.contains(&node_id.to_string()) {
            nodes.push(node_id.to_string());
            self.rebuild_ring().await;
            info!("Added node {} to consistent hash ring", node_id);
        }
    }

    /// Remove a node from the ring, rebuilding the consistent-hash ring.
    pub async fn remove_node(&self, node_id: &str) {
        let mut nodes = self.nodes.write().await;
        nodes.retain(|n| n != node_id);
        self.rebuild_ring().await;
        info!("Removed node {} from consistent hash ring", node_id);
    }

    /// Rebuild the consistent-hash ring from the current node set, placing one
    /// virtual node per configured slot.
    pub async fn rebuild_ring(&self) {
        let nodes = self.nodes.read().await;
        let mut entries: Vec<ShardRingEntry> = Vec::new();

        for node in nodes.iter() {
            for vn in 0..self.config.virtual_nodes_per_node {
                let vnode_id = format!("{}:vn{}", node, vn);
                let hash = hash_string(&vnode_id);
                let shard_id = format!("shard_{:016x}", hash % self.config.num_shards as u64);
                entries.push(ShardRingEntry {
                    hash,
                    node_id: node.clone(),
                    shard_id,
                    virtual_node: vnode_id,
                });
            }
        }

        entries.sort_by_key(|e| e.hash);
        *self.ring.write().await = entries;
    }

    /// Find the ring entry responsible for a key.
    pub async fn get_shard_for_key(&self, key: &str) -> Option<ShardRingEntry> {
        let key_hash = hash_string(key);
        let ring = self.ring.read().await;
        ring.iter()
            .find(|e| e.hash >= key_hash)
            .or_else(|| ring.first())
            .cloned()
    }

    /// Return up to `replication_factor` distinct nodes responsible for a key,
    /// walking the ring clockwise from the key's position.
    pub async fn get_nodes_for_key(&self, key: &str) -> Vec<String> {
        let rf = self.config.replication_factor as usize;
        let ring = self.ring.read().await;
        let key_hash = hash_string(key);

        let start_idx = ring.iter().position(|e| e.hash >= key_hash).unwrap_or(0);
        let mut nodes = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for i in 0..ring.len() {
            let idx = (start_idx + i) % ring.len();
            let entry = &ring[idx];
            if seen.insert(entry.node_id.clone()) {
                nodes.push(entry.node_id.clone());
                if nodes.len() >= rf {
                    break;
                }
            }
        }
        nodes
    }

    /// Record shard metadata keyed by `table:shard_id`.
    pub async fn register_shard(&self, shard: ShardInfo) {
        let key = format!("{}:{}", shard.table, shard.shard_id);
        self.shards.write().await.insert(key, shard);
    }

    /// Look up shard metadata by table and shard ID.
    pub async fn get_shard(&self, table: &str, shard_id: &str) -> Option<ShardInfo> {
        let key = format!("{}:{}", table, shard_id);
        self.shards.read().await.get(&key).cloned()
    }

    /// All shards belonging to a table.
    pub async fn table_shards(&self, table: &str) -> Vec<ShardInfo> {
        self.shards
            .read()
            .await
            .values()
            .filter(|s| s.table == table)
            .cloned()
            .collect()
    }

    /// All shards a node hosts as primary or replica.
    pub async fn node_shards(&self, node_id: &str) -> Vec<ShardInfo> {
        self.shards
            .read()
            .await
            .values()
            .filter(|s| s.primary_node == node_id || s.replica_nodes.contains(&node_id.to_string()))
            .cloned()
            .collect()
    }

    /// Register a node with an optional region for geo-distributed sharding.
    pub async fn add_node_with_region(&self, node_id: &str, region: Option<&str>) {
        self.add_node(node_id).await;
        if let Some(r) = region {
            let mut rm = self.region_map.write().await;
            rm.entry(r.to_string())
                .or_default()
                .push(node_id.to_string());
            info!("Registered node {} in region {}", node_id, r);
        }
    }

    /// Create a geo-distributed shard with replicas spread across regions.
    /// The primary is placed in the specified region, and replicas are assigned
    /// to other regions for disaster recovery.
    pub async fn create_geo_shard(
        &self,
        table: &str,
        storage_type: &str,
        hash_start: u64,
        hash_end: u64,
        primary_region: &str,
        replica_regions: &[String],
    ) -> Option<ShardInfo> {
        let rm = self.region_map.read().await;
        let primary_nodes = rm.get(primary_region)?;
        let primary_node = primary_nodes.first()?.clone();

        let mut cross_region_replicas: HashMap<String, Vec<String>> = HashMap::new();
        let mut all_replicas = Vec::new();

        for region in replica_regions {
            if let Some(nodes) = rm.get(region) {
                let replicas: Vec<String> = nodes.iter().take(2).cloned().collect();
                if !replicas.is_empty() {
                    cross_region_replicas.insert(region.clone(), replicas.clone());
                    all_replicas.extend(replicas);
                }
            }
        }

        let shard_id = format!("geo_{:016x}", hash_start);
        let shard = ShardInfo {
            shard_id: shard_id.clone(),
            table: table.to_string(),
            storage_type: storage_type.to_string(),
            hash_range_start: hash_start,
            hash_range_end: hash_end,
            primary_node: primary_node.clone(),
            replica_nodes: all_replicas,
            record_count: 0,
            size_bytes: 0,
            version: 1,
            primary_region: ShardRegion::new(primary_region, 0),
            cross_region_replicas,
        };

        self.register_shard(shard.clone()).await;
        info!(
            "Created geo-shard {} primary={} region={} replicas={:?}",
            shard_id, primary_node, primary_region, replica_regions
        );
        Some(shard)
    }

    /// Get all nodes in a specific region.
    pub async fn nodes_in_region(&self, region: &str) -> Vec<String> {
        self.region_map
            .read()
            .await
            .get(region)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all known regions.
    pub async fn regions(&self) -> Vec<String> {
        self.region_map.read().await.keys().cloned().collect()
    }

    /// Check if a shard has cross-region replicas for disaster recovery.
    pub async fn has_cross_region_redundancy(&self, shard_id: &str) -> bool {
        let shards = self.shards.read().await;
        shards
            .values()
            .any(|s| s.shard_id == shard_id && !s.cross_region_replicas.is_empty())
    }

    /// Persist the current shard state (shards, nodes, ring) to
    /// `config.data_dir/shards.json`.
    pub async fn persist_shards(&self) {
        let path = std::path::Path::new(&self.config.data_dir).join("shards.json");
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let snapshot = {
            let shards = self.shards.read().await;
            let nodes = self.nodes.read().await;
            let ring = self.ring.read().await;
            serde_json::json!({
                "shards": *shards,
                "nodes": *nodes,
                "ring": *ring,
            })
        };

        match tokio::fs::write(
            &path,
            serde_json::to_string_pretty(&snapshot).unwrap_or_default(),
        )
        .await
        {
            Ok(_) => tracing::debug!("Shard state persisted to {}", path.display()),
            Err(e) => tracing::error!("Failed to persist shards: {}", e),
        }
    }

    /// Detect overloaded nodes and return a plan of shards to migrate from them
    /// to underloaded nodes.
    pub async fn check_rebalance_needed(&self) -> Vec<ShardMigrationPlan> {
        let shards = self.shards.read().await;
        let nodes = self.nodes.read().await;
        let mut migrations = Vec::new();

        if nodes.len() < 2 {
            return migrations;
        }

        let node_count = nodes.len() as f64;
        let shard_count = shards.len() as f64;
        let ideal_per_node = if node_count > 0.0 {
            shard_count / node_count
        } else {
            0.0
        };

        let mut node_loads: HashMap<String, usize> = HashMap::new();
        for shard in shards.values() {
            *node_loads.entry(shard.primary_node.clone()).or_insert(0) += 1;
        }

        let threshold = self.config.rebalance_threshold;
        for (node, load) in &node_loads {
            let load_f = *load as f64;
            if ideal_per_node > 0.0 && load_f > ideal_per_node * (1.0 + threshold) {
                let overload = *load;
                let target_count = ideal_per_node.ceil() as usize;
                let excess = overload.saturating_sub(target_count);

                let underloaded: Vec<String> = node_loads
                    .iter()
                    .filter(|(n, l)| {
                        *n != node && (**l as f64) < ideal_per_node * (1.0 - threshold)
                    })
                    .map(|(n, _)| n.clone())
                    .collect();

                if !underloaded.is_empty() {
                    for target in underloaded.iter().take(excess) {
                        let shards_to_move: Vec<String> = shards
                            .values()
                            .filter(|s| s.primary_node == *node)
                            .take(1)
                            .map(|s| s.shard_id.clone())
                            .collect();

                        for sid in shards_to_move {
                            migrations.push(ShardMigrationPlan {
                                shard_id: sid,
                                source_node: node.clone(),
                                target_node: target.clone(),
                                reason: "Rebalance".into(),
                            });
                        }
                    }
                }
            }
        }

        migrations
    }
}

/// A proposed shard migration from one node to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMigrationPlan {
    /// Shard to move
    pub shard_id: String,
    /// Node currently hosting the shard
    pub source_node: String,
    /// Node that should host the shard
    pub target_node: String,
    /// Reason for the migration (e.g. `Rebalance`)
    pub reason: String,
}

fn hash_string(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}
