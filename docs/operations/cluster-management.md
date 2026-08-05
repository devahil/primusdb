# Cluster Management

This guide covers cluster architecture, CLI commands, federation, and known limitations in PrimusDB v1.3.2-alpha.

---

## Cluster Architecture

A PrimusDB cluster consists of multiple nodes that communicate over the network. Each node runs a PrimusDB server instance. The cluster provides:

- **Consensus** — Raft-based leader election and log replication
- **Data distribution** — Automatic sharding across nodes
- **Health monitoring** — Heartbeat-based failure detection
- **Federation** — Multi-cluster replication across regions

### Architecture Diagram

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

### Node Roles

| Role | Description |
|------|-------------|
| Leader | Handles writes, coordinates replication |
| Follower | Replicates data, serves reads |
| Learner | Catches up with cluster state before becoming a voter |

### Cluster Components

- **Raft consensus** — Leader election and log replication (infrastructure present but not fully active)
- **Sharding** — Data partitioned across nodes (declared in API but no automatic distribution)
- **Heartbeat** — Periodic health checks between nodes
- **Federation** — Cross-region cluster linking (partially implemented)

---

## Starting a Multi-Node Cluster

For development and testing, start multiple PrimusDB instances on the same machine:

### Terminal 1: Coordinator Node

```bash
primusdb server start --bind 127.0.0.1:8080 --data-dir ./data/node1
```

### Terminal 2: Worker Node

```bash
primusdb server start --bind 127.0.0.1:8081 --data-dir ./data/node2
```

### Terminal 3: Register via API

```bash
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

Start both with config files:

```bash
primusdb server start --config node1.toml
primusdb server start --config node2.toml
```

---

## CLI Commands Reference

### `primusdb cluster status`

Show the overall status of the cluster. Calls `GET /api/v1/cluster/status`.

```bash
primusdb cluster status
primusdb cluster status --verbose
primusdb cluster status --watch           # Continuously watch
primusdb cluster status --watch --interval 5
```

**Example output:**
```json
{
  "success": true,
  "data": {
    "cluster_id": "cluster_001",
    "coordinator_node": "node_001",
    "total_nodes": 3,
    "active_nodes": 3,
    "nodes": [
      {"id": "node_001", "address": "10.0.0.1:8080", "status": "active", "role": "coordinator"}
    ],
    "shards": [],
    "replication_factor": 3,
    "health_score": 98.5
  }
}
```

### `primusdb cluster nodes`

List all nodes registered in the cluster. Calls `GET /api/v1/cluster/nodes`.

```bash
primusdb cluster nodes
primusdb cluster nodes --role leader
primusdb cluster nodes --role follower
primusdb cluster nodes --state active
primusdb cluster nodes --state suspect --verbose
```

**Example output:**
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

### `primusdb cluster health`

Check cluster health. Calls `GET /api/v1/cache/cluster/health`.

```bash
primusdb cluster health
primusdb cluster health --diagnostic
primusdb cluster health --threshold-ms 200
```

| Flag | Description | Default |
|------|-------------|---------|
| `--diagnostic` | Run detailed diagnostics on all nodes | `false` |
| `--threshold-ms <MS>` | Latency threshold (ms) for warning status | `100` |

### `primusdb cluster inspect`

Inspect a specific cluster node.

```bash
primusdb cluster inspect node-001
primusdb cluster inspect node-001 --verbose
```

### `primusdb cluster join`

Register a new node into the cluster.

```bash
primusdb cluster join 192.168.1.10:8080
primusdb cluster join 192.168.1.10:8080 --node-id node-4
primusdb cluster join 192.168.1.10:8080 --node-id node-4 --tls
primusdb cluster join 192.168.1.10:8080 --timeout 60
```

| Flag | Description | Default |
|------|-------------|---------|
| `PEER` | Peer address to join (required) | — |
| `-n, --node-id <ID>` | Custom node identifier | auto-generated |
| `--timeout <SECONDS>` | Join timeout | `30` |
| `--tls` | Use TLS for peer communication | `false` |

**Alpha status:** Sends a registration request but does not negotiate membership or replicate data automatically.

### `primusdb cluster leave`

Remove a node from the cluster.

```bash
primusdb cluster leave node-4
primusdb cluster leave node-4 --drain
primusdb cluster leave node-4 --force
```

| Flag | Description | Default |
|------|-------------|---------|
| `node` | Node ID to remove (required) | — |
| `--drain` | Migrate data away before leaving | `false` |
| `--force` | Leave immediately without draining | `false` |

**Alpha status:** Sends an API request. Data draining is not implemented.

### `primusdb cluster rebalance`

Trigger cluster rebalancing.

```bash
primusdb cluster rebalance
primusdb cluster rebalance --node node-003
primusdb cluster rebalance --strategy size
primusdb cluster rebalance --concurrency 4
```

| Flag | Description | Default |
|------|-------------|---------|
| `--node <ID>` | Target node for rebalance | all |
| `--strategy <STRATEGY>` | Rebalance strategy (`size`, `count`, `load`) | `size` |
| `--concurrency <N>` | Concurrent shard transfers | `2` |

### `primusdb cluster sync`

Synchronize cluster state across all nodes.

```bash
primusdb cluster sync
primusdb cluster sync --full
primusdb cluster sync --timeout 120
```

| Flag | Description | Default |
|------|-------------|---------|
| `--full` | Full sync (not incremental) | `false` |
| `-t, --timeout <SECONDS>` | Sync timeout | `60` |

### `primusdb cluster failover`

Trigger manual failover from one node to another.

```bash
primusdb cluster failover node-001
primusdb cluster failover node-001 --target node-002
primusdb cluster failover node-001 --force
```

| Flag | Description |
|------|-------------|
| `node` | Node to fail over from (required) |
| `-t, --target <ID>` | Target node to promote |
| `--force` | Force failover without checks |

### `primusdb cluster topology`

Show cluster topology.

```bash
primusdb cluster topology
primusdb cluster topology --format json    # table (default), json, dot
```

### `primusdb cluster config`

View or modify cluster configuration.

```bash
primusdb cluster config --list
primusdb cluster config --get replication_factor
primusdb cluster config --set replication_factor=5
```

| Flag | Description |
|------|-------------|
| `-l, --list` | List all configuration keys and values |
| `-g, --get <KEY>` | Get a specific configuration value |
| `-s, --set <KEY=VALUE>` | Set a configuration value |

---

## Federation

Federation connects multiple PrimusDB clusters across regions for global deployments.

### Starting a Federated Node

```bash
primusdb server start \
  --bind 0.0.0.0:8080 \
  --federation-id global-fed \
  --cluster-id cluster-us \
  --region us-east-1 \
  --federation-discovery cluster-eu:8080,cluster-asia:8080
```

| Flag | Description |
|------|-------------|
| `--federation-id <ID>` | Federated group identifier |
| `--cluster-id <ID>` | Unique identifier for this cluster |
| `--region <REGION>` | Geographic region label |
| `--federation-discovery <ADDRS>` | Comma-separated peer cluster addresses |

### DataDomains

DataDomains define data replication policies across federated clusters:

```bash
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

### Federation API Endpoints

```bash
curl http://localhost:8080/api/v1/federation/status
curl http://localhost:8080/api/v1/federation/clusters
curl http://localhost:8080/api/v1/federation/domains
curl http://localhost:8080/api/v1/federation/metrics
```

> **Note:** Federation is partially implemented. API endpoints return data structures but cross-cluster replication and DataDomain enforcement are not fully operational.

---

## Known Limitations (v1.3.2-alpha)

| Feature | Status |
|---------|--------|
| `cluster join` | Sends registration request; no membership negotiation or data replication |
| `cluster leave` | Sends API request; data draining not integrated |
| `cluster rebalance` | Reads federation status; no automatic shard redistribution |
| `cluster failover` | Sends failover request; no automatic leader election |
| Sharding | Declared in API responses; no automatic distribution |
| Consensus (Raft) | Infrastructure present; not active in default configuration |
| Multi-node clusters | Nodes operate independently unless manually coordinated via API |
| Federation replication | API stubs respond; cross-cluster sync not operational |

---

## Security Considerations

- **Authentication** — Cluster API endpoints should be behind TLS in production. Use `--tls` on join commands where supported.
- **Network isolation** — Run cluster nodes on a private network or VPN. Do not expose cluster API ports to the public internet.
- **Node identity** — Always assign explicit `node-id` values in production configurations to prevent identity confusion.
- **Firewall rules** — Restrict inter-node communication ports to trusted IP ranges only.
- **Audit logging** — Enable audit logging to track cluster membership changes and configuration modifications.
