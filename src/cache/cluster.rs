/*!
# Distributed Cache Clustering System

This module documents the distributed caching cluster provided by the
PrimusDB cache layer. The implementation lives in [`super::manager`] and
combines:

- **Consistent hashing** ([`super::hashing`]) to map keys to nodes with
  minimal redistribution on scaling
- **Per-node memory caches** ([`super::cache`]) with LZ4 compression,
  CRC32 checksums and search
- **A local consensus simulation** ([`super::consensus`]) that votes on
  cache operations before execution

## Public API (from `manager.rs`)

```text
CacheCluster::new(config)            (sync)
  +-> join_cluster()                 initialize local cache + hash ring
  +-> put(key, data) / get(key)      distributed access via hash ring
  +-> search(pattern, limit)         search across cluster nodes
  +-> add_node(addr) / remove_node(addr)
  +-> get_cluster_health()           ClusterHealth { overall_health, ... }
```

## Usage

### Basic Cluster Setup
```ignore
use primusdb::cache::{CacheCluster, ClusterConfig};

let cluster_config = ClusterConfig::default();
let cluster = CacheCluster::new(cluster_config);
cluster.join_cluster().await?;

cluster.put("user:123", b"user data").await?;
let data = cluster.get("user:123").await?;
```

### Cluster Management
```ignore
// Add a cache node
cluster.add_node("cache-node-4:8080").await?;

// Monitor cluster health
let health = cluster.get_cluster_health().await?;
println!("Cluster health: {}%", health.overall_health * 100.0);

// Remove a node
cluster.remove_node("cache-node-4:8080").await?;
```
*/
