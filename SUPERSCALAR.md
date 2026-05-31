# PrimusDB SuperScalar: Multi-Cluster Federation

## Overview

PrimusDB SuperScalar extends the single-cluster architecture (Raft + SWIM + sharding) to a **Cluster Federation** or **Cluster-of-Clusters** model. A SuperCluster can span multiple physical clusters across different geographic regions, clouds, or data centers, coordinating selective replication, namespace resolution, and intelligent routing through a federation layer.

```
┌─────────────────────────────────────────────────────────────────────┐
│                    SUPERCLUSTER (Global Federation)                  │
│                                                                     │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐  │
│  │   FederationManager  │   FederationManager  │   FederationManager  │
│  │  Cluster A (us-east) │  Cluster B (eu-west) │  Cluster C (ap-south)│
│  │                      │                      │                      │
│  │  ┌────────────────┐  │  ┌────────────────┐  │  ┌────────────────┐  │
│  │  │ Raft Consensus  │  │  │ Raft Consensus  │  │  │ Raft Consensus  │  │
│  │  │ SWIM Gossip     │  │  │ SWIM Gossip     │  │  │ SWIM Gossip     │  │
│  │  │ Shard Manager   │  │  │ Shard Manager   │  │  │ Shard Manager   │  │
│  │  │ Replication     │  │  │ Replication     │  │  │ Replication     │  │
│  │  │ ClusterGateway  │  │  │ ClusterGateway  │  │  │ ClusterGateway  │  │
│  │  └────────────────┘  │  └────────────────┘  │  └────────────────┘  │
│  │                      │                      │                      │
│  │  DataDomains:        │  DataDomains:        │  DataDomains:        │
│  │  ├─ users (Sync)     │  ├─ users (Sync)     │  ├─ users (Async)    │
│  │  ├─ products (Local) │  ├─ analytics (Local)│  └─ products (Async) │
│  │  └─ orders (Local)   │  └─ orders (Sync)    │                     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘  │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              Cross-Cluster RPC (TCP + bincode)                │   │
│  │  FedClusterAnnounce | FedHeartbeat | FedDomainJoin           │   │
│  │  FedDataReplica | FedNamespaceResolve                        │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

## 1. Cluster Federation

### FederationManager

Each cluster runs a `FederationManager` that maintains a registry of all member clusters in the federation. Cross-cluster communication uses the same TCP/bincode RPC protocol as the internal cluster, with specific federation messages.

```
FederationManager (per cluster)
├── Config: federation_id, cluster_id, region, timeouts
├── Members: HashMap<cluster_id → FederationMember>
│   ├── FedClusterInfo (address, port, size, domains, status)
│   ├── RpcClient (TCP connection to federation peer)
│   └── Health tracking (consecutive_failures, last_seen)
├── Background Tasks:
│   ├── Announce Loop (every 10s): FedClusterAnnounce to all members
│   └── Heartbeat Loop (every 5s): FedHeartbeat to all members
└── Suspect Detection: timeout 30s → Suspect, 60s → Offline
```

**New cluster join flow:**
1. Node A is configured with `federation_id = "prod-global"` and `discovery_servers = ["cluster-b:9100"]`
2. Node A sends `FedClusterAnnounce` to Node B's federation port
3. Node B responds with `FedClusterAck` including its known cluster list
4. Both nodes add the clusters to their member table
5. Periodic heartbeats keep the state up to date

**Federation Messages (new RpcMessage variants):**
| Message | Direction | Purpose |
|---------|-----------|---------|
| `FedClusterAnnounce` | Node → Peer | Announce presence and capabilities |
| `FedClusterAck` | Peer → Node | Accept and share topology |
| `FedHeartbeat` | Node → Peer | Heartbeat with health metrics |
| `FedDomainJoin` | Node → Peer | Join a data domain |
| `FedDomainJoinAck` | Peer → Node | Confirm domain membership |
| `FedDomainLeave` | Node → Peer | Leave a domain |
| `FedDataReplica` | Node → Peer | Replicate data between clusters |
| `FedDataReplicaAck` | Peer → Node | Confirm replication |
| `FedNamespaceResolve` | Node → Peer | Resolve physical name across clusters |

## 2. DataDomains: Selective Multi-Cluster Replication

A **DataDomain** is a logical group of data that is replicated across a subset of clusters. It allows deciding which data goes to which clusters and with which replication mode.

```
DataDomain "users" (Sync, 3 clusters)
├── Cluster A (us-east): Primary (leader)
├── Cluster B (eu-west): Sync Replica
└── Cluster C (ap-south): Async Replica
Collections: ["users", "profiles", "sessions"]
StorageTypes: ["document", "relational"]

DataDomain "analytics" (Async, 2 clusters)
├── Cluster B (eu-west): Primary
└── Cluster A (us-east): Async Replica
Collections: ["events", "metrics"]
StorageTypes: ["columnar"]
```

**Cross-Cluster Replication Modes:**
- **Sync**: All replicas must acknowledge before the operation is considered successful. Strong consistency across clusters.
- **Quorum**: Simple majority `(n/2 + 1)` of replicas. Balance between consistency and availability.
- **Async**: At least one replica acknowledges. Maximum availability, eventual consistency.

**Write flow with DataDomain:**
1. Client writes `users:{"id": "u1"}` to Cluster A
2. ClusterGateway identifies that `users` belongs to domain "users" (Sync mode)
3. Cluster A: writes locally + internal Raft replication
4. Cluster A: `DataDomainManager.replicate_cross_cluster()` sends `FedDataReplica` to Clusters B and C
5. Cluster A waits for acknowledgment from Cluster B (Sync) but not Cluster C (Async)
6. If both confirm → operation successful

## 3. Federation-Aware Gateway

The `ClusterGateway` now supports the `DomainAware` strategy that considers DataDomains when routing requests:

```
Client → ClusterGateway (DomainAware)
    ├── Does the key belong to a domain?
    │   ├── Yes → Is the local cluster a domain member?
    │   │   ├── Yes → Route locally
    │   │   └── No → Route to nearest member cluster
    │   └── No → Use regular strategy (shard-aware, round-robin, etc.)
    │
    └── Circuit Breaker and Latency Tracking at cluster level
```

## 4. Federated Namespaces

Namespaces can now cross cluster boundaries. A `root.global.users` namespace can exist on multiple clusters with automatic resolution:

```
Namespace: "root.global.users" (Federated)
├── Cluster A: physical resource "users_table"
├── Cluster B: physical resource "global_users"
└── Resolution: FederationManager queries cross-cluster
```

When a cluster cannot find a resource locally, the `FederationManager` sends `FedNamespaceResolveRequest` to sibling clusters, caching the result.

## 5. Consensus Integration (Raft)

The gateway and federation do not replace each cluster's internal Raft consensus. Instead:

| Layer | Consensus | Scope |
|-------|-----------|-------|
| **Intra-Cluster** | Raft | Each individual cluster has its own Raft |
| **Federation** | Gossip + Heartbeats | Clusters coordinate via federation messages |
| **DataDomain** | Sync/Quorum/Async | Cross-cluster replication configurable per domain |

For critical federation metadata (cluster registry, global domains), optional **Federated Raft** can be enabled, providing atomic cross-cluster operations.

## 6. Federation REST API

New endpoints exposed by each node running the FederationManager:

```
GET    /api/v1/federation/status        → Federation status
GET    /api/v1/federation/clusters      → List member clusters
POST   /api/v1/federation/domains       → Create DataDomain
GET    /api/v1/federation/domains       → List DataDomains
POST   /api/v1/federation/domains/:name/balance → Trigger domain rebalance
GET    /api/v1/federation/metrics       → Federation metrics
```

## 7. Cross-Cutting Data Architecture

All distributed capabilities operate across **all storage types**:

| Operation | Columnar | Vector | Document | Relational | KeyValue |
|-----------|----------|--------|----------|------------|----------|
| Raft Consensus | ✓ | ✓ | ✓ | ✓ | ✓ |
| SWIM Membership | ✓ | ✓ | ✓ | ✓ | ✓ |
| Sharding (Hash Ring) | ✓ | ✓ | ✓ | ✓ | ✓ |
| Replication (Sync/Async/Quorum) | ✓ | ✓ | ✓ | ✓ | ✓ |
| ClusterGateway Routing | ✓ | ✓ | ✓ | ✓ | ✓ |
| Federation (DataDomain) | ✓ | ✓ | ✓ | ✓ | ✓ |
| Federated Namespace | ✓ | ✓ | ✓ | ✓ | ✓ |

## 8. Usage Examples

### Configure Federation (2 clusters)

```bash
# Cluster A (us-east)
primusdb-server \
  --cluster-enabled \
  --node-id "node-us-1" \
  --federation-id "prod-global" \
  --cluster-id "us-east" \
  --discovery-servers "cluster-eu:9100" \
  --region "us-east-1"

# Cluster B (eu-west)
primusdb-server \
  --cluster-enabled \
  --node-id "node-eu-1" \
  --federation-id "prod-global" \
  --cluster-id "eu-west" \
  --discovery-servers "cluster-us:9100" \
  --region "eu-west-1"
```

### Create a DataDomain from driver

```python
# Python
client = PrimusDBClient()
await client.connect()

# Create user domain with Sync replication between US and EU
domain = await client.create_data_domain(
    name="users",
    description="User profiles - global sync",
    replication_mode="sync",
    storage_types=["document"],
    collections=["users", "profiles"],
    member_clusters=["us-east", "eu-west"]
)
```

### Query federation status

```bash
curl http://localhost:8080/api/v1/federation/status
```

```json
{
  "success": true,
  "data": {
    "federation_id": "prod-global",
    "local_cluster": "us-east",
    "region": "us-east-1",
    "clusters_online": 3,
    "clusters_total": 5,
    "domains": ["users", "analytics"],
    "members": [
      {"cluster_id": "us-east", "status": "online", "size": 3},
      {"cluster_id": "eu-west", "status": "online", "size": 3},
      {"cluster_id": "ap-south", "status": "suspect", "size": 3}
    ]
  }
}
```

### Federated namespace from driver

```python
# The namespace "root.global.users" exists in EU but the client connects to US
# Automatic cross-cluster resolution
result = await client.select(
    storage_type="document",
    table="root.global.users",
    query={"email": "user@example.com"}
)
# → Gateway detects namespace is federated
# → Queries FederationManager
# → FedNamespaceResolveRequest → Cluster EU → response
# → Routes query to EU cluster
```

## 9. Cluster Inheritance

The **inheritance** concept allows a child cluster to inherit DataDomains, namespaces, and policies from a parent cluster:

```
Parent Cluster "prod-core" (Primary Datacenter)
├── DataDomain: "users" (Sync)
├── Namespace: "root.global"
└── Policies: retention=90d, encryption=required

Child Cluster "prod-dr" (Disaster Recovery)
├── Inherits DataDomain "users" from parent (Async)
├── Inherits Namespace "root.global" + adds "root.global.dr"
├── Inherits retention and encryption policies
│   (can override inheritance with ExplicitOnly)
└── If parent goes down, child can promote to primary
```

**Inheritance Modes:**
- `DenyOverride`: Parent policies are restrictive and binding
- `ExplicitOnly`: Only explicitly defined policies on the child
- `AllowOverride`: Child can override any parent policy

## 10. Complete Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    SUPERSCALAR OPERATION                         │
│                                                                 │
│  1. Client sends write("users", {"id": "u1"}) to Gateway       │
│                                                                 │
│  2. Gateway applies DomainAware routing                         │
│     ├── Is "users" in a DataDomain? → Yes ("users" domain)     │
│     ├── Is this cluster a member? → Yes                         │
│     └── Route to local node                                     │
│                                                                 │
│  3. Local node processes:                                       │
│     ├── Raft: proposes entry to local consensus                 │
│     ├── SWIM: marks operation in gossip                         │
│     ├── Shard Manager: determines shard by hash of "u1"        │
│     └── Storage Engine: writes to DocumentEngine                │
│                                                                 │
│  4. Intra-cluster replication (Raft + ReplicationEngine)       │
│     ├── Raft replicates log to followers (AppendEntries)        │
│     └── ReplicationEngine replicates to shard replicas         │
│                                                                 │
│  5. Cross-cluster replication (FedDataReplica)                  │
│     ├── DataDomain "users" Sync mode                            │
│     ├── Send FedDataReplica to: Cluster B (eu-west)             │
│     └── Cluster B responds FedDataReplicaAck → success          │
│                                                                 │
│  6. Federated namespace (if applicable)                        │
│     ├── If namespace is cross-cluster, resolve via              │
│     │   FedNamespaceResolveRequest                              │
│     └── Cache result for subsequent queries                     │
│                                                                 │
│  7. Respond to client with success                              │
└─────────────────────────────────────────────────────────────────┘
```

## 11. Fault Tolerance and Survival

| Scenario | Behavior |
|----------|----------|
| Member cluster stops sending heartbeats | Marked as Suspect (30s), then Offline (60s) |
| Offline cluster has data in a Sync DataDomain | DataDomain degrades to Async until recovery |
| Domain primary cluster goes down | New domain leader elected among remaining members |
| Federation split-brain | Uses `federation_id` to identify partitions; only the partition with majority of clusters continues operating |
| Unresolvable federated namespace | Gateway returns 503 error with hint of recommended cluster |

## 12. Future Extensibility

- **Federated Raft**: Cross-cluster consensus for critical metadata (domain state, global membership) using the same Raft algorithm at federation level — **implemented in `federated_raft.rs`**
- **DataDomain Auto-Balance**: Automatic collection movement between clusters based on load, latency, and cost — **implemented in `domain.rs::check_balance()`**
- **Multi-Region Active-Active**: Concurrent operations across multiple regions with conflict resolution based on vector clocks (already available in SyncCoordinator) — **pending cross-cluster integration**
- **Geo-Distributed Sharding**: Shards that cross cluster boundaries with replicas in different regions — **pending implementation**
- **Global Observability**: Federation metrics exportable to Prometheus/Grafana — **basic endpoint implemented in `/api/v1/federation/metrics`**
