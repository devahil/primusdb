# TUI Cluster Management

The Clusters section provides visibility into cluster state, including an ASCII topology diagram, a nodes table, cluster health, and recent events.

## Prerequisites

- A connection to a PrimusDB instance must be active.
- The target server must have cluster functionality enabled.

## Topology Diagram

When cluster node data is available, the TUI renders an ASCII topology showing the leader-follower relationship:

```
  ◆ Leader node-1 (10.0.0.1:8080)
  │
  ├─◇ Follower node-2 (10.0.0.2:8080)  lag: 0.3s
  │
  └─◇ Follower node-3 (10.0.0.3:8080)  lag: 1.2s
```

### Node Symbols

| Symbol | Role | Color |
|--------|------|-------|
| `◆` | Leader (active) | Green |
| `◇` | Follower / Voter | Cyan |
| `◈` | Learner | Yellow |
| `○` | Offline / Down | Red |

### Status Colors

| Status | Color |
|--------|-------|
| `healthy`, `ok`, `online`, `active`, `leader` | Green |
| `warning`, `degraded` | Yellow |
| `error`, `down`, `offline` | Red |

### Replication Lag

Each follower node shows its replication lag in seconds (e.g. `lag: 0.3s`). High lag indicates the follower is falling behind.

## Nodes Table

The Nodes section provides a tabular view with four columns:

```
  ID      Role      Status    Address
  ───────────────────────────────────────────
  node-1  leader    active    10.0.0.1:8080
  node-2  voter     healthy   10.0.0.2:8080
  node-3  learner   warning   10.0.0.3:8080
```

## Health View

Displays raw JSON from `/api/v1/cache/cluster/health`:

```json
{
  "success": true,
  "data": {
    "overall_status": "healthy",
    "node_count": 3,
    "active_nodes": 3
  }
}
```

## Cluster Events

The TUI fetches cluster events and displays them as a time-ordered list:

```
  [10:00:01] Node node-2 joined the cluster
  [10:00:02] Leader election completed — node-1 elected
  [10:01:15] Node node-3 lag warning (>1s)
```

## Summary Line

A summary line at the bottom shows aggregate stats:

```
  Total: 3 nodes  |  Leader: node-1  |  Healthy: 3/3
```

## Refresh Mechanism

Press `r` to refresh all cluster data. The TUI fires three concurrent requests:

1. `GET /api/v1/cluster/status`
2. `GET /api/v1/cluster/nodes`
3. `GET /api/v1/cache/cluster/health`

Each request has a 5-second timeout. Partial updates are applied as they arrive.

## Limitations

The Clusters section is **read-only**. For write operations use the CLI:

| Operation | CLI Alternative |
|-----------|-----------------|
| Join a node | `primusdb cluster join <peer>` |
| Remove a node | `primusdb cluster leave <node>` |
| Trigger rebalance | `primusdb cluster rebalance` |
| Manual failover | `primusdb cluster failover <node>` |
| View full topology | `primusdb cluster topology` |

## Keybindings

| Key | Action |
|-----|--------|
| `r` | Refresh cluster data from all endpoints |
| `Tab` / `Shift+Tab` | Navigate sections |
| `Ctrl+D` | Toggle details view |
