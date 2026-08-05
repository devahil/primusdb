# Local Cluster

A PrimusDB cluster enables multiple server instances to work together for high availability, data distribution, and fault tolerance.

## Concept: PrimusDB Cluster

A PrimusDB cluster consists of multiple nodes that communicate over the network. Each node runs a PrimusDB server instance. The cluster provides:

- **Consensus** — Raft-based leader election and log replication
- **Data distribution** — Automatic sharding across nodes
- **Health monitoring** — Heartbeat-based failure detection
- **Federation** — Multi-cluster replication across regions

### Architecture Overview

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Node 1     │     │   Node 2     │     │   Node 3     │
│  (Leader)    │◄───►│  (Follower)  │◄───►│  (Follower)  │
│  :8080       │     │  :8081       │     │  :8082       │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
       └────────────────────┴────────────────────┘
                 Consensus (Raft) Layer
```

In v1.3.2-alpha, cluster functionality is **partially implemented**. The CLI commands for querying cluster state are functional.

## `primusdb cluster status`

Show the overall status of the cluster.

```bash
# Basic status
primusdb cluster status

# Verbose status with detailed node information
primusdb cluster status --verbose

# Continuously watch cluster status
primusdb cluster status --watch

# Watch with custom refresh interval
primusdb cluster status --watch --interval 5
```

This command calls `GET /api/v1/cluster/status` and displays the response.

### Example Output

```json
{
  "success": true,
  "data": {
    "cluster_id": "cluster_001",
    "coordinator_node": "node_001",
    "total_nodes": 3,
    "active_nodes": 3,
    "nodes": [
      {
        "id": "node_001",
        "address": "10.0.0.1:8080",
        "status": "active",
        "role": "coordinator",
        "last_heartbeat": "2026-01-15T12:00:00Z"
      },
      {
        "id": "node_002",
        "address": "10.0.0.2:8080",
        "status": "active",
        "role": "follower",
        "last_heartbeat": "2026-01-15T12:00:00Z"
      }
    ],
    "shards": [],
    "replication_factor": 3,
    "health_score": 98.5
  }
}
```

## `primusdb cluster nodes`

List all nodes registered in the cluster.

```bash
# List all nodes
primusdb cluster nodes

# Filter by role
primusdb cluster nodes --role leader
primusdb cluster nodes --role follower

# Filter by state
primusdb cluster nodes --state active
primusdb cluster nodes --state suspect

# Verbose output
primusdb cluster nodes --state active --verbose
```

This command calls `GET /api/v1/cluster/nodes` and displays the response.

### Example Output

```json
{
  "success": true,
  "data": [
    {"node_id": "node1", "address": "10.0.0.1:8080", "status": "active"},
    {"node_id": "node2", "address": "10.0.0.2:8080", "status": "active"},
    {"node_id": "node3", "address": "10.0.0.3:8080", "status": "active"}
  ]
}
```

## `primusdb cluster health`

Check the health of the cluster.

```bash
# Basic health check
primusdb cluster health

# Run detailed diagnostic checks
primusdb cluster health --diagnostic

# Set custom latency threshold for warnings
primusdb cluster health --threshold-ms 200
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--diagnostic` | Run detailed diagnostic checks on all nodes | `false` |
| `--threshold-ms <MS>` | Latency threshold (ms) for warning status | `100` |

## `primusdb cluster join`

Join an existing cluster as a new node.

```bash
# Join a cluster by specifying a peer address
primusdb cluster join 192.168.1.10:8080

# Join with a custom node identifier
primusdb cluster join 192.168.1.10:8080 --node-id node-4

# Join with TLS
primusdb cluster join 192.168.1.10:8080 --node-id node-4 --tls

# Join with custom timeout
primusdb cluster join 192.168.1.10:8080 --timeout 60
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `PEER` | Peer address to join (required positional argument) | — |
| `-n, --node-id <ID>` | Custom node identifier | auto-generated |
| `--timeout <SECONDS>` | Join timeout | `30` |
| `--tls` | Use TLS for peer communication | `false` |

### Alpha Status: Stub

The `cluster join` command sends a registration request to the cluster API endpoint but does **not** perform full cluster membership negotiation. Data shards are not automatically migrated or replicated to the joining node.

## `primusdb cluster leave`

Leave the current cluster.

```bash
# Graceful leave
primusdb cluster leave

# Drain data before leaving
primusdb cluster leave --drain

# Force leave without draining
primusdb cluster leave --force
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--drain` | Migrate data away from this node before leaving | `false` |
| `--force` | Leave immediately without draining | `false` |

### Alpha Status: Stub

In v1.3.2-alpha, `cluster leave` prints a placeholder message. The node is **not** removed from the cluster membership. Data draining is not implemented.

## Starting a Local Multi-Node Cluster

For development and testing, you can start multiple PrimusDB instances on the same machine:

### Terminal 1: Coordinator Node

```bash
primusdb server start --bind 127.0.0.1:8080 --data-dir ./data/node1
```

### Terminal 2: Worker Node

```bash
primusdb server start --bind 127.0.0.1:8081 --data-dir ./data/node2
```

### Terminal 3: Register the Cluster

```bash
# Register via API
curl -X POST http://localhost:8080/api/v1/cluster/node/register \
  -H "Content-Type: application/json" \
  -d '{"node_id": "worker-1", "host": "127.0.0.1", "port": 8081, "shards": []}'

# Check cluster status
curl http://localhost:8080/api/v1/cluster/status | jq .
```

### Using Config Files

**node1.toml:**

```toml
[storage]
data_dir = "./data/node1"

[network]
bind_address = "127.0.0.1"
port = 8080

[cluster]
enabled = true
node_id = "node-1"
discovery_servers = []

[logging]
level = "info"
```

**node2.toml:**

```toml
[storage]
data_dir = "./data/node2"

[network]
bind_address = "127.0.0.1"
port = 8081

[cluster]
enabled = true
node_id = "node-2"
discovery_servers = ["127.0.0.1:8080"]

[logging]
level = "info"
```

Start both:

```bash
primusdb server start --config node1.toml
primusdb server start --config node2.toml
```

## Known Limitations in v1.3.2-alpha

- **`cluster join`** sends a registration request but does not negotiate membership, replicate data, or participate in consensus automatically.
- **`cluster leave`** / **`cluster failover`** send API requests to the cluster gateway. Data draining is not yet integrated.
- **`cluster rebalance`** reads federation status — automatic shard redistribution is not implemented.
- **Sharding** is declared in the API responses but no automatic shard distribution occurs.
- **Consensus** (Raft) structure is present in the codebase but is not active in the default cluster configuration.
- **Multi-node clusters** are effectively manual; each node operates independently unless manually coordinated via the API.

## Federation: `primusdb server start --federation-id`

For global deployments, PrimusDB supports a federation layer that connects multiple clusters across regions.

```bash
# Start a federated cluster node
primusdb server start \
  --bind 0.0.0.0:8080 \
  --federation-id global-fed \
  --cluster-id cluster-us \
  --region us-east-1 \
  --federation-discovery cluster-eu:8080,cluster-asia:8080
```

### Federation Command-Line Flags

| Flag | Description |
|------|-------------|
| `--federation-id <ID>` | Federated group identifier (all clusters in the same federation use the same ID) |
| `--cluster-id <ID>` | Unique identifier for this cluster within the federation |
| `--region <REGION>` | Geographic region label |
| `--federation-discovery <ADDRS>` | Comma-separated list of peer cluster addresses |

### DataDomains

DataDomains define which data is replicated across federated clusters:

```bash
# Create a DataDomain for global user data
curl -X POST http://localhost:8080/api/v1/federation/domains \
  -H "Content-Type: application/json" \
  -d '{
    "name": "global-users",
    "description": "User data replicated across all regions",
    "replication_mode": "Quorum",
    "storage_types": ["document"],
    "collections": ["users"],
    "member_clusters": ["cluster-us", "cluster-eu", "cluster-asia"]
  }'
```

### Federation Status

```bash
# Check federation health
curl http://localhost:8080/api/v1/federation/status

# List member clusters
curl http://localhost:8080/api/v1/federation/clusters

# List DataDomains
curl http://localhost:8080/api/v1/federation/domains

# Federation metrics
curl http://localhost:8080/api/v1/federation/metrics
```

> **Note:** Federation is also partially implemented in v1.3.2-alpha. API endpoints respond with data structures, but cross-cluster replication and DataDomain enforcement are not fully operational.
