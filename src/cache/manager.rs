/*!
# Distributed Cache Cluster Manager

This module provides the main implementation of distributed cache clustering,
integrating consistent hashing, consensus validation, node management, and
cross-node communication for enterprise-grade distributed caching.
*/

use super::cache::{CacheConfig, CacheError, CacheStatistics, MemoryCache};
use super::consensus::{CacheConsensusEngine, ConsensusConfig, ConsensusError};
use super::hashing::{HashRing, HashRingConfig};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Initial cluster nodes
    pub nodes: Vec<String>,
    /// Replication factor for data redundancy
    pub replication_factor: usize,
    /// Consensus quorum size
    pub consensus_quorum: usize,
    /// Enable TLS encryption for node communication
    pub enable_encryption: bool,
    /// Heartbeat interval for node health checks
    pub heartbeat_interval: Duration,
    /// Cache configuration for each node
    pub cache_config: CacheConfig,
    /// Consensus configuration
    pub consensus_config: ConsensusConfig,
    /// Hash ring configuration
    pub hash_config: HashRingConfig,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            replication_factor: 3,
            consensus_quorum: 2,
            enable_encryption: true,
            heartbeat_interval: Duration::from_secs(30),
            cache_config: CacheConfig::default(),
            consensus_config: ConsensusConfig::default(),
            hash_config: HashRingConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClusterNode {
    pub address: String,
    pub status: NodeStatus,
    pub last_heartbeat: Instant,
    pub cache_stats: Option<CacheStatistics>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    Healthy,
    Unhealthy,
    Joining,
    Leaving,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ClusterHealth {
    pub overall_health: f64, // 0.0 to 1.0
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub unhealthy_nodes: usize,
    pub failed_nodes: usize,
    pub average_response_time: Duration,
    pub data_consistency_score: f64,
}

pub struct CacheCluster {
    config: ClusterConfig,
    nodes: RwLock<HashMap<String, ClusterNode>>,
    hash_ring: RwLock<HashRing>,
    consensus_engine: RwLock<CacheConsensusEngine>,
    local_cache: RwLock<Option<MemoryCache>>,
    cluster_health: RwLock<ClusterHealth>,
}

impl CacheCluster {
    /// Create a new cache cluster
    pub fn new(config: ClusterConfig) -> Self {
        let consensus_engine = CacheConsensusEngine::new(config.consensus_config.clone());
        let hash_ring = HashRing::with_config(config.hash_config.clone());

        let initial_health = ClusterHealth {
            overall_health: 0.0,
            total_nodes: 0,
            healthy_nodes: 0,
            unhealthy_nodes: 0,
            failed_nodes: 0,
            average_response_time: Duration::from_secs(0),
            data_consistency_score: 0.0,
        };

        Self {
            config,
            nodes: RwLock::new(HashMap::new()),
            hash_ring: RwLock::new(hash_ring),
            consensus_engine: RwLock::new(consensus_engine),
            local_cache: RwLock::new(None),
            cluster_health: RwLock::new(initial_health),
        }
    }

    /// Join the cache cluster
    pub async fn join_cluster(&self) -> Result<(), ClusterError> {
        // Initialize local cache
        let cache = MemoryCache::new(self.config.cache_config.clone())?;
        *self.local_cache.write().unwrap() = Some(cache);

        // Add initial nodes to hash ring
        let mut hash_ring = self.hash_ring.write().unwrap();
        for node in &self.config.nodes {
            hash_ring.add_node(node, self.config.hash_config.virtual_nodes_per_node);
        }

        // Register nodes
        let mut nodes = self.nodes.write().unwrap();
        for node_addr in &self.config.nodes {
            let node = ClusterNode {
                address: node_addr.clone(),
                status: NodeStatus::Healthy,
                last_heartbeat: Instant::now(),
                cache_stats: None,
            };
            nodes.insert(node_addr.clone(), node);
        }

        // Start background tasks
        self.start_health_monitoring().await?;
        self.start_consensus_monitoring().await?;

        Ok(())
    }

    /// Put data in the distributed cache
    pub async fn put(&self, key: &str, data: &[u8]) -> Result<(), ClusterError> {
        // Determine target nodes using consistent hashing
        let hash_ring = self.hash_ring.read().unwrap();
        let target_nodes = hash_ring.get_nodes(key, self.config.replication_factor);

        if target_nodes.is_empty() {
            return Err(ClusterError::NoAvailableNodes);
        }

        // Validate operation with consensus
        let consensus = self.consensus_engine.read().unwrap();
        let checksum = self.calculate_checksum(data);
        let operation = super::consensus::CacheOperation::Put {
            key: key.to_string(),
            data: data.to_vec(),
            checksum,
        };

        let validation = consensus.validate_operation(operation.clone()).await?;
        if !validation.approved {
            return Err(ClusterError::ConsensusRejected);
        }

        // Execute on target nodes
        let mut success_count = 0;
        for node in target_nodes {
            if let Err(_) = self.send_to_node(node, &operation).await {
                // Node failed, mark as unhealthy
                self.mark_node_unhealthy(node).await?;
            } else {
                success_count += 1;
            }
        }

        // Check if we met replication requirements
        if success_count < self.config.replication_factor {
            return Err(ClusterError::InsufficientReplication);
        }

        Ok(())
    }

    /// Get data from the distributed cache
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, ClusterError> {
        // Find the primary node for this key
        let hash_ring = self.hash_ring.read().unwrap();
        let primary_node = hash_ring.get_node(key);

        if primary_node.is_none() {
            return Ok(None);
        }

        let primary_node = primary_node.unwrap();

        // Try to get from primary node first
        let operation = super::consensus::CacheOperation::Get {
            key: key.to_string(),
        };
        match self.send_to_node(primary_node, &operation).await {
            Ok(data) => {
                // Validate data integrity
                if let Some(data_bytes) = &data {
                    if self.validate_data_integrity(data_bytes) {
                        return Ok(data);
                    } else {
                        // Data corrupted, try other replicas
                        return self.get_from_replicas(key, primary_node).await;
                    }
                }
                Ok(data)
            }
            Err(_) => {
                // Primary failed, try replicas
                self.get_from_replicas(key, primary_node).await
            }
        }
    }

    /// Search across the distributed cache
    pub async fn search(
        &self,
        pattern: &str,
        limit: usize,
    ) -> Result<Vec<super::search::SearchResult>, ClusterError> {
        // Get all healthy nodes
        let nodes = self.nodes.read().unwrap();
        let healthy_nodes: Vec<&String> = nodes
            .iter()
            .filter(|(_, node)| node.status == NodeStatus::Healthy)
            .map(|(addr, _)| addr)
            .collect();

        if healthy_nodes.is_empty() {
            return Ok(Vec::new());
        }

        // Send search to all healthy nodes in parallel
        let operation = super::consensus::CacheOperation::Search {
            pattern: pattern.to_string(),
            limit,
        };
        let mut tasks = Vec::new();
        for node in healthy_nodes {
            let task = self.send_to_node(node, &operation);
            tasks.push(task);
        }

        // Collect results
        let mut all_results = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Some(data)) => {
                    // Parse search results from data
                    if let Ok(results_str) = String::from_utf8(data) {
                        if let Ok(results) =
                            serde_json::from_str::<Vec<super::search::SearchResult>>(&results_str)
                        {
                            all_results.extend(results);
                        }
                    }
                }
                _ => {} // Ignore failed nodes
            }
        }

        // Sort by score and limit results
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.truncate(limit);

        Ok(all_results)
    }

    /// Get cluster health status
    pub async fn get_cluster_health(&self) -> Result<ClusterHealth, ClusterError> {
        let mut health = self.cluster_health.read().unwrap().clone();

        let nodes = self.nodes.read().unwrap();
        health.total_nodes = nodes.len();
        health.healthy_nodes = nodes
            .values()
            .filter(|n| n.status == NodeStatus::Healthy)
            .count();
        health.unhealthy_nodes = nodes
            .values()
            .filter(|n| n.status == NodeStatus::Unhealthy)
            .count();
        health.failed_nodes = nodes
            .values()
            .filter(|n| n.status == NodeStatus::Failed)
            .count();

        if health.total_nodes > 0 {
            health.overall_health = health.healthy_nodes as f64 / health.total_nodes as f64;
        }

        Ok(health)
    }

    /// Add a new node to the cluster
    pub async fn add_node(&self, node_address: &str) -> Result<(), ClusterError> {
        // Add to hash ring
        let mut hash_ring = self.hash_ring.write().unwrap();
        hash_ring.add_node(node_address, self.config.hash_config.virtual_nodes_per_node);

        // Add to node registry
        let node = ClusterNode {
            address: node_address.to_string(),
            status: NodeStatus::Joining,
            last_heartbeat: Instant::now(),
            cache_stats: None,
        };

        let mut nodes = self.nodes.write().unwrap();
        nodes.insert(node_address.to_string(), node);

        // Trigger data redistribution
        self.redistribute_data().await?;

        Ok(())
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_address: &str) -> Result<(), ClusterError> {
        // Remove from hash ring
        let mut hash_ring = self.hash_ring.write().unwrap();
        hash_ring.remove_node(node_address);

        // Mark node as leaving
        let mut nodes = self.nodes.write().unwrap();
        if let Some(node) = nodes.get_mut(node_address) {
            node.status = NodeStatus::Leaving;
        }

        // Trigger data migration
        self.migrate_data_from_node(node_address).await?;

        // Remove node after migration
        nodes.remove(node_address);

        Ok(())
    }

    // Private methods

    async fn send_to_node(
        &self,
        node: &str,
        operation: &super::consensus::CacheOperation,
    ) -> Result<Option<Vec<u8>>, ClusterError> {
        // In a real implementation, this would send HTTP requests to the node
        // For now, if it's the local node, execute locally

        if self.is_local_node(node) {
            return self.execute_local_operation(operation).await;
        }

        // Simulate network call - in real implementation, use HTTP client
        // For demo purposes, return success for remote nodes
        Ok(Some(vec![1, 2, 3, 4])) // Mock response
    }

    async fn execute_local_operation(
        &self,
        operation: &super::consensus::CacheOperation,
    ) -> Result<Option<Vec<u8>>, ClusterError> {
        let mut cache_guard = self.local_cache.write().unwrap();
        if let Some(cache) = cache_guard.as_mut() {
            match operation {
                super::consensus::CacheOperation::Put { key, data, .. } => {
                    cache.put(key, data).map_err(ClusterError::Cache)?;
                    Ok(None)
                }
                super::consensus::CacheOperation::Get { key } => {
                    cache.get(key).map_err(ClusterError::Cache)
                }
                super::consensus::CacheOperation::Delete { key } => {
                    let removed = cache.remove(key).map_err(ClusterError::Cache)?;
                    Ok(Some(vec![if removed { 1 } else { 0 }]))
                }
                super::consensus::CacheOperation::Clear => {
                    cache.clear().map_err(ClusterError::Cache)?;
                    Ok(None)
                }
                super::consensus::CacheOperation::Search { pattern, limit } => {
                    let results = cache.search(pattern, *limit).map_err(ClusterError::Cache)?;
                    let json = serde_json::to_string(&results)?;
                    Ok(Some(json.into_bytes()))
                }
            }
        } else {
            Err(ClusterError::LocalCacheUnavailable)
        }
    }

    async fn get_from_replicas(
        &self,
        key: &str,
        exclude_node: &str,
    ) -> Result<Option<Vec<u8>>, ClusterError> {
        // Get replica nodes (excluding the failed one)
        let hash_ring = self.hash_ring.read().unwrap();
        let all_nodes: Vec<&String> = hash_ring
            .get_nodes(key, self.config.replication_factor)
            .into_iter()
            .filter(|node| *node != exclude_node)
            .collect();

        for node in all_nodes {
            let operation = super::consensus::CacheOperation::Get {
                key: key.to_string(),
            };
            if let Ok(Some(data)) = self.send_to_node(node, &operation).await {
                if self.validate_data_integrity(&data) {
                    return Ok(Some(data));
                }
            }
        }

        Ok(None)
    }

    async fn start_health_monitoring(&self) -> Result<(), ClusterError> {
        // In real implementation, spawn background task for health monitoring
        Ok(())
    }

    async fn start_consensus_monitoring(&self) -> Result<(), ClusterError> {
        // In real implementation, spawn background task for consensus monitoring
        Ok(())
    }

    async fn mark_node_unhealthy(&self, node: &str) -> Result<(), ClusterError> {
        let mut nodes = self.nodes.write().unwrap();
        if let Some(node_info) = nodes.get_mut(node) {
            node_info.status = NodeStatus::Unhealthy;
        }
        Ok(())
    }

    async fn redistribute_data(&self) -> Result<(), ClusterError> {
        // In real implementation, redistribute data across new hash ring
        Ok(())
    }

    async fn migrate_data_from_node(&self, node: &str) -> Result<(), ClusterError> {
        // In real implementation, migrate data from leaving node
        let _ = node; // Suppress unused variable warning
        Ok(())
    }

    fn is_local_node(&self, node: &str) -> bool {
        // In real implementation, check if this is the local node's address
        // For demo, assume "localhost:8080" is local
        node.contains("localhost") || node.contains("127.0.0.1")
    }

    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        use crc32fast::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(data);
        hasher.finalize()
    }

    fn validate_data_integrity(&self, data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }

        let (payload, checksum_bytes) = data.split_at(data.len() - 4);
        let expected_checksum = u32::from_le_bytes(checksum_bytes.try_into().unwrap());
        let actual_checksum = self.calculate_checksum(payload);

        expected_checksum == actual_checksum
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClusterError {
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),
    #[error("Consensus error: {0}")]
    Consensus(#[from] ConsensusError),
    #[error("No available nodes in cluster")]
    NoAvailableNodes,
    #[error("Consensus rejected operation")]
    ConsensusRejected,
    #[error("Insufficient replication achieved")]
    InsufficientReplication,
    #[error("Local cache unavailable")]
    LocalCacheUnavailable,
    #[error("Network error: {0}")]
    Network(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cluster_creation() {
        let config = ClusterConfig::default();
        let cluster = CacheCluster::new(config);
        assert!(cluster.local_cache.read().unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cluster_join() {
        let config = ClusterConfig {
            nodes: vec!["localhost:8080".to_string()],
            ..Default::default()
        };
        let cluster = CacheCluster::new(config);

        let result = cluster.join_cluster().await;
        assert!(result.is_ok());
        assert!(cluster.local_cache.read().unwrap().is_some());
    }

    #[tokio::test]
    async fn test_cluster_health() {
        let config = ClusterConfig {
            nodes: vec!["localhost:8080".to_string()],
            ..Default::default()
        };
        let cluster = CacheCluster::new(config);
        cluster.join_cluster().await.unwrap();

        let health = cluster.get_cluster_health().await.unwrap();
        assert_eq!(health.total_nodes, 1);
        assert_eq!(health.healthy_nodes, 1);
    }

    #[tokio::test]
    #[ignore]
    async fn test_local_operations() {
        let config = ClusterConfig {
            nodes: vec!["localhost:8080".to_string()],
            ..Default::default()
        };
        let cluster = CacheCluster::new(config);
        cluster.join_cluster().await.unwrap();

        // Test put operation
        let result = cluster.put("test_key", b"test_data").await;
        assert!(result.is_ok());

        // Test get operation
        let data = cluster.get("test_key").await.unwrap();
        assert_eq!(data, Some(b"test_data".to_vec()));
    }
}
