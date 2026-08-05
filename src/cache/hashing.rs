/*!
# Consistent Hashing - Key Distribution for Cache Clusters

Implements consistent hashing for distributing cache keys across cluster nodes,
providing load balancing and minimal key redistribution when nodes join or leave.

## How It Works

```text
HashRing
  +-- add_node(node, virtual_nodes)   place <virtual_nodes> replicas on ring
  +-- remove_node(node)               drop all replicas for a node
  +-- get_node(key)   -> Option<&String>   nearest replica clockwise
  +-- get_nodes(key, count)           first <count> replicas (for replication)
  +-- get_all_nodes()
  +-- get_load_distribution()         HashMap<node, key-count>
  +-- get_balance_score()             f64 in [0,1], 1.0 = perfectly balanced
```

Nodes map to several positions on a ring (virtual nodes / replicas) so that
key ownership is statistically uniform. The default hash algorithm is SipHash
(see [`HashAlgorithm`]); distribution is unaffected by node scaling except for
the `~1/N` keys that naturally migrate.

## Usage Examples

### Basic Consistent Hashing
```ignore
use primusdb::cache::hashing::HashRing;

let mut hash_ring = HashRing::new();

// Add nodes to the ring (3 virtual nodes each)
hash_ring.add_node("cache-node-1:8080", 3);
hash_ring.add_node("cache-node-2:8080", 3);
hash_ring.add_node("cache-node-3:8080", 3);

// Find node for a key
if let Some(node) = hash_ring.get_node("user:12345") {
    println!("Key goes to: {}", node);
}

// Handle node failure — keys automatically redistribute
hash_ring.remove_node("cache-node-2:8080");
```

### Load Monitoring
```ignore
let load_distribution = hash_ring.get_load_distribution();
for (node, load) in load_distribution {
    println!("Node {}: {} keys", node, load);
}

let balance_score = hash_ring.get_balance_score();
if balance_score < 0.8 {
    println!("Warning: Cluster load imbalance detected!");
}
```
*/

use siphasher::sip::SipHasher24;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

/// Consistent hash ring mapping keys to cluster nodes.
///
/// Keys are hashed with SipHash-2-4 onto a circular key space and each node
/// is represented by one or more virtual nodes to improve load balance and
/// minimize key redistribution when nodes join or leave.
#[derive(Debug, Clone)]
pub struct HashRing {
    ring: BTreeMap<u64, String>,              // hash -> node
    nodes: HashMap<String, usize>,            // node -> virtual node count
    virtual_nodes: HashMap<String, Vec<u64>>, // node -> virtual node hashes
    config: HashRingConfig,
}

/// Configuration for a [`HashRing`].
#[derive(Debug, Clone)]
pub struct HashRingConfig {
    /// Number of virtual nodes placed on the ring per physical node
    pub virtual_nodes_per_node: usize,
    /// Whether load distribution monitoring is enabled
    pub enable_load_monitoring: bool,
}

impl Default for HashRingConfig {
    fn default() -> Self {
        Self {
            virtual_nodes_per_node: 256,
            enable_load_monitoring: true,
        }
    }
}

impl Default for HashRing {
    fn default() -> Self {
        Self::new()
    }
}

impl HashRing {
    /// Create a new consistent hash ring
    pub fn new() -> Self {
        Self::with_config(HashRingConfig::default())
    }

    /// Create a new hash ring with custom configuration
    pub fn with_config(config: HashRingConfig) -> Self {
        Self {
            ring: BTreeMap::new(),
            nodes: HashMap::new(),
            virtual_nodes: HashMap::new(),
            config,
        }
    }

    /// Add a node to the hash ring
    pub fn add_node(&mut self, node: &str, virtual_nodes: usize) {
        let virtual_nodes = if virtual_nodes == 0 {
            self.config.virtual_nodes_per_node
        } else {
            virtual_nodes
        };

        // Remove existing virtual nodes for this node
        if let Some(old_hashes) = self.virtual_nodes.remove(node) {
            for hash in old_hashes {
                self.ring.remove(&hash);
            }
        }

        let mut hashes = Vec::with_capacity(virtual_nodes);

        // Add virtual nodes
        for i in 0..virtual_nodes {
            let virtual_node_key = format!("{}:{}", node, i);
            let hash = self.hash(&virtual_node_key);
            self.ring.insert(hash, node.to_string());
            hashes.push(hash);
        }

        self.nodes.insert(node.to_string(), virtual_nodes);
        self.virtual_nodes.insert(node.to_string(), hashes);
    }

    /// Remove a node from the hash ring
    pub fn remove_node(&mut self, node: &str) {
        if let Some(hashes) = self.virtual_nodes.remove(node) {
            for hash in hashes {
                self.ring.remove(&hash);
            }
        }
        self.nodes.remove(node);
    }

    /// Get the node responsible for a key
    pub fn get_node(&self, key: &str) -> Option<&String> {
        if self.ring.is_empty() {
            return None;
        }

        let hash = self.hash(key);

        // Find the first node with hash >= key_hash
        if let Some((_, node)) = self.ring.range(hash..).next() {
            return Some(node);
        }

        // Wrap around to the first node
        self.ring.values().next()
    }

    /// Get multiple nodes for replication
    pub fn get_nodes(&self, key: &str, count: usize) -> Vec<&String> {
        if self.ring.is_empty() || count == 0 {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let hash = self.hash(key);

        // Start from the key's position and collect unique nodes
        for (_, node) in self.ring.range(hash..).chain(self.ring.iter()) {
            if seen.insert(node) {
                result.push(node);
                if result.len() >= count {
                    break;
                }
            }
        }

        // If we didn't get enough unique nodes, wrap around again
        if result.len() < count {
            for node in self.ring.values() {
                if seen.insert(node) {
                    result.push(node);
                    if result.len() >= count {
                        break;
                    }
                }
            }
        }

        result
    }

    /// Get all nodes in the ring
    pub fn get_all_nodes(&self) -> Vec<&String> {
        self.nodes.keys().collect()
    }

    /// Get load distribution statistics
    pub fn get_load_distribution(&self) -> HashMap<&String, usize> {
        let mut distribution = HashMap::new();

        // Initialize all nodes with 0
        for node in self.nodes.keys() {
            distribution.insert(node, 0);
        }

        // Simulate key distribution (in real implementation, track actual keys)
        for i in 0..10000 {
            let key = format!("test_key_{}", i);
            if let Some(node) = self.get_node(&key) {
                *distribution.entry(node).or_insert(0) += 1;
            }
        }

        distribution
    }

    /// Get balance score (0.0 to 1.0, higher is better)
    pub fn get_balance_score(&self) -> f64 {
        let distribution = self.get_load_distribution();
        if distribution.is_empty() {
            return 0.0;
        }

        let values: Vec<usize> = distribution.values().cloned().collect();
        let mean = values.iter().sum::<usize>() as f64 / values.len() as f64;

        if mean == 0.0 {
            return 1.0; // All nodes have 0 load, perfectly balanced
        }

        let variance = values
            .iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>()
            / values.len() as f64;

        let std_dev = variance.sqrt();
        let cv = std_dev / mean; // Coefficient of variation

        // Convert to balance score (lower CV = higher balance)
        let balance_score = 1.0 / (1.0 + cv);
        balance_score.min(1.0)
    }

    /// Get ring information
    pub fn info(&self) -> HashRingInfo {
        HashRingInfo {
            total_nodes: self.nodes.len(),
            total_virtual_nodes: self.ring.len(),
            virtual_nodes_per_node: self.config.virtual_nodes_per_node,
            balance_score: self.get_balance_score(),
        }
    }

    // Private methods

    fn hash(&self, key: &str) -> u64 {
        let mut hasher = SipHasher24::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

/// Summary information about the current state of a [`HashRing`].
#[derive(Debug, Clone)]
pub struct HashRingInfo {
    /// Number of physical nodes in the ring
    pub total_nodes: usize,
    /// Total number of virtual node positions on the ring
    pub total_virtual_nodes: usize,
    /// Configured number of virtual nodes per physical node
    pub virtual_nodes_per_node: usize,
    /// Load balance score between 0.0 and 1.0 (higher is better)
    pub balance_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_ring_creation() {
        let ring = HashRing::new();
        assert_eq!(ring.get_all_nodes().len(), 0);
    }

    #[test]
    fn test_add_and_remove_nodes() {
        let mut ring = HashRing::new();

        ring.add_node("node1", 2);
        assert_eq!(ring.get_all_nodes().len(), 1);

        ring.add_node("node2", 2);
        assert_eq!(ring.get_all_nodes().len(), 2);

        ring.remove_node("node1");
        assert_eq!(ring.get_all_nodes().len(), 1);
    }

    #[test]
    fn test_key_distribution() {
        let mut ring = HashRing::new();

        ring.add_node("node1", 10);
        ring.add_node("node2", 10);

        // Test that we get consistent results
        let node1 = ring.get_node("test_key").unwrap();
        let node2 = ring.get_node("test_key").unwrap();
        assert_eq!(node1, node2);
    }

    #[test]
    fn test_multiple_nodes() {
        let mut ring = HashRing::new();

        ring.add_node("node1", 5);
        ring.add_node("node2", 5);
        ring.add_node("node3", 5);

        let nodes = ring.get_nodes("test_key", 2);
        assert_eq!(nodes.len(), 2);
        assert_ne!(nodes[0], nodes[1]);
    }

    #[test]
    fn test_balance_score() {
        let mut ring = HashRing::new();

        ring.add_node("node1", 10);
        ring.add_node("node2", 10);

        let score = ring.get_balance_score();
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn test_ring_info() {
        let mut ring = HashRing::new();

        ring.add_node("node1", 8);
        ring.add_node("node2", 8);

        let info = ring.info();
        assert_eq!(info.total_nodes, 2);
        assert_eq!(info.total_virtual_nodes, 16); // 8 + 8
    }
}
