use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tokio::sync::RwLock;
use tracing::info;

/// Region location for geo-distributed shards
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShardRegion {
    pub name: String,
    pub priority: u32,
}

impl ShardRegion {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    pub shard_id: String,
    pub table: String,
    pub storage_type: String,
    pub hash_range_start: u64,
    pub hash_range_end: u64,
    pub primary_node: String,
    pub replica_nodes: Vec<String>,
    pub record_count: u64,
    pub size_bytes: u64,
    pub version: u64,
    /// Primary region for this shard
    pub primary_region: ShardRegion,
    /// Cross-region replica assignments (region → list of nodes)
    pub cross_region_replicas: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardDistribution {
    pub num_shards: u32,
    pub replication_factor: u32,
    pub ring: Vec<ShardRingEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardRingEntry {
    pub hash: u64,
    pub node_id: String,
    pub shard_id: String,
    pub virtual_node: String,
}

#[derive(Debug, Clone)]
pub struct ShardManagerConfig {
    pub num_shards: u32,
    pub replication_factor: u32,
    pub virtual_nodes_per_node: u32,
    pub rebalance_threshold: f64,
    pub migrate_batch_size: u32,
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

#[derive(Debug)]
pub struct ShardManager {
    pub config: ShardManagerConfig,
    pub node_id: String,
    pub shards: RwLock<HashMap<String, ShardInfo>>,
    pub ring: RwLock<Vec<ShardRingEntry>>,
    pub nodes: RwLock<Vec<String>>,
    /// Region membership: region_name → list of node_ids
    pub region_map: RwLock<HashMap<String, Vec<String>>>,
}

impl ShardManager {
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

    pub async fn add_node(&self, node_id: &str) {
        let mut nodes = self.nodes.write().await;
        if !nodes.contains(&node_id.to_string()) {
            nodes.push(node_id.to_string());
            self.rebuild_ring().await;
            info!("Added node {} to consistent hash ring", node_id);
        }
    }

    pub async fn remove_node(&self, node_id: &str) {
        let mut nodes = self.nodes.write().await;
        nodes.retain(|n| n != node_id);
        self.rebuild_ring().await;
        info!("Removed node {} from consistent hash ring", node_id);
    }

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

    pub async fn get_shard_for_key(&self, key: &str) -> Option<ShardRingEntry> {
        let key_hash = hash_string(key);
        let ring = self.ring.read().await;
        ring.iter()
            .find(|e| e.hash >= key_hash)
            .or_else(|| ring.first())
            .cloned()
    }

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

    pub async fn register_shard(&self, shard: ShardInfo) {
        let key = format!("{}:{}", shard.table, shard.shard_id);
        self.shards.write().await.insert(key, shard);
    }

    pub async fn get_shard(&self, table: &str, shard_id: &str) -> Option<ShardInfo> {
        let key = format!("{}:{}", table, shard_id);
        self.shards.read().await.get(&key).cloned()
    }

    pub async fn table_shards(&self, table: &str) -> Vec<ShardInfo> {
        self.shards
            .read()
            .await
            .values()
            .filter(|s| s.table == table)
            .cloned()
            .collect()
    }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMigrationPlan {
    pub shard_id: String,
    pub source_node: String,
    pub target_node: String,
    pub reason: String,
}

fn hash_string(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}
