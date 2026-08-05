# Metrics and Monitoring

PrimusDB exposes Prometheus-compatible metrics for monitoring server health, performance, and cluster operations.

## GET /metrics

The `/metrics` endpoint returns metrics in the standard Prometheus text format.

```bash
curl http://localhost:8080/metrics
```

**Example Output:**

```
# HELP primusdb_up PrimusDB service availability
# TYPE primusdb_up gauge
primusdb_up 1

# HELP primusdb_version PrimusDB version
# TYPE primusdb_version gauge
primusdb_version{version="1.3.2-alpha"} 1

# HELP primusdb_uptime_seconds Service uptime in seconds
# TYPE primusdb_uptime_seconds counter
primusdb_uptime_seconds 3600

# HELP primusdb_storage_operations_total Total storage operations
# TYPE primusdb_storage_operations_total counter
primusdb_storage_operations_total{engine="columnar"} 150
primusdb_storage_operations_total{engine="vector"} 75
primusdb_storage_operations_total{engine="document"} 200
primusdb_storage_operations_total{engine="relational"} 50
```

## `primusdb metrics`

View metrics from the CLI:

```bash
# Show all metrics
primusdb metrics

# Filter by metric name pattern
primusdb metrics --filter storage

# Continuously watch metrics (refresh every 2 seconds)
primusdb metrics --watch

# Watch with custom interval
primusdb metrics --watch --interval 5
```

## Available Metrics

### General

| Metric | Type | Description |
|--------|------|-------------|
| `primusdb_up` | gauge | 1 if the server is running, 0 otherwise |
| `primusdb_version` | gauge | Version label for metric disambiguation |
| `primusdb_uptime_seconds` | counter | Seconds since the server started |
| `primusdb_memory_usage_bytes` | gauge | Current memory usage in bytes |
| `primusdb_cpu_usage_ratio` | gauge | CPU usage as a ratio (0.0–1.0) |
| `primusdb_http_requests_total` | counter | Total HTTP requests by method and status |
| `primusdb_http_request_duration_seconds` | histogram | HTTP request latency distribution |

### Storage Engine Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `primusdb_storage_operations_total` | counter | Total operations per engine |
| `primusdb_storage_operation_errors_total` | counter | Operation errors per engine |
| `primusdb_storage_operation_duration_seconds` | histogram | Operation latency distribution |
| `primusdb_storage_bytes_total` | gauge | Total data size per engine |
| `primusdb_storage_cache_hits_total` | counter | Cache hit count per engine |
| `primusdb_storage_cache_misses_total` | counter | Cache miss count per engine |

### Consensus / Cluster Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `primusdb_cluster_nodes_total` | gauge | Total cluster nodes |
| `primusdb_cluster_nodes_active` | gauge | Currently active nodes |
| `primusdb_cluster_leader_changes_total` | counter | Leader election events |
| `primusdb_cluster_round_trip_seconds` | histogram | Inter-node latency distribution |
| `primusdb_consensus_proposals_total` | counter | Total consensus proposals |
| `primusdb_consensus_commits_total` | counter | Total consensus commits |
| `primusdb_consensus_failures_total` | counter | Consensus failures |
| `primusdb_consensus_term` | gauge | Current consensus term number |

### Federation Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `primusdb_federation_clusters_online` | gauge | Federation clusters currently online |
| `primusdb_federation_clusters_total` | gauge | Registered federation clusters |
| `primusdb_federation_domains_total` | gauge | Total data domains |
| `primusdb_federation_healthy_ratio` | gauge | Ratio of online to total clusters |
| `primusdb_federation_announce_cycles_total` | counter | Heartbeat/announce cycles completed |
| `primusdb_federation_replications_total` | counter | Cross-cluster replication requests |
| `primusdb_federation_replication_failures` | counter | Replication failures |
| `primusdb_federation_replication_latency_seconds` | histogram | Replication latency distribution |
| `primusdb_domain_healthy` | gauge | Per-domain health indicator |

### AI/ML Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `primusdb_ai_predictions_total` | counter | Total predictions made |
| `primusdb_ai_training_runs_total` | counter | Total training runs |
| `primusdb_ai_training_duration_seconds` | histogram | Training duration distribution |
| `primusdb_ai_inference_latency_seconds` | histogram | Inference latency distribution |
| `primusdb_ai_models_loaded` | gauge | Currently loaded models |
| `primusdb_ai_prediction_errors_total` | counter | Prediction errors |

### Key-Value Engine Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `primusdb_kv_operations_total` | counter | Total KV operations |
| `primusdb_kv_operation_errors_total` | counter | KV operation errors |
| `primusdb_kv_operation_latency_seconds` | histogram | KV operation latency |
| `primusdb_kv_databases_total` | gauge | Total KV databases |
| `primusdb_kv_documents_total` | gauge | Total KV documents |

### Protocol / Network Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `primusdb_protocol_messages_sent_total` | counter | Total protocol messages sent |
| `primusdb_protocol_messages_received_total` | counter | Total protocol messages received |
| `primusdb_protocol_active_peers` | gauge | Currently connected peers |
| `primusdb_protocol_reconnect_attempts_total` | counter | Reconnection attempts |

## Prometheus Integration

### Scrape Configuration

Add the following job to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'primusdb'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 10s
    scrape_timeout: 5s

  - job_name: 'primusdb-cluster'
    static_configs:
      - targets:
        - 'node1:8080'
        - 'node2:8080'
        - 'node3:8080'
    metrics_path: '/metrics'
    scrape_interval: 15s
```

### Docker Compose

```yaml
services:
  primusdb:
    image: primusdb:latest
    ports:
      - "8080:8080"

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus

volumes:
  prometheus_data:
```

## Grafana Dashboards

While no pre-built dashboard JSON is included in this release, the following panels provide a starting point:

### Server Overview

```json
{
  "panels": [
    {
      "title": "Uptime",
      "type": "stat",
      "targets": [{"expr": "primusdb_uptime_seconds"}]
    },
    {
      "title": "Query Rate",
      "type": "graph",
      "targets": [{
        "expr": "rate(primusdb_http_requests_total[5m])",
        "legendFormat": "{{method}}"
      }]
    },
    {
      "title": "Memory Usage",
      "type": "graph",
      "targets": [{
        "expr": "primusdb_memory_usage_bytes / 1024 / 1024",
        "legendFormat": "Memory (MB)"
      }]
    }
  ]
}
```

### Performance Monitoring

Suggested queries for common monitoring panels:

| Panel | PromQL Query |
|-------|-------------|
| Request rate | `rate(primusdb_http_requests_total[5m])` |
| Error rate | `rate(primusdb_http_requests_total{status=~"5.."}[5m])` |
| p99 latency | `histogram_quantile(0.99, rate(primusdb_http_request_duration_seconds_bucket[5m]))` |
| Storage ops | `rate(primusdb_storage_operations_total[5m])` |
| Cache hit ratio | `primusdb_storage_cache_hits_total / (primusdb_storage_cache_hits_total + primusdb_storage_cache_misses_total)` |
| Cluster health | `primusdb_cluster_nodes_active / primusdb_cluster_nodes_total` |

### Alerting Rules

```yaml
groups:
  - name: primusdb
    rules:
      - alert: PrimusDBDown
        expr: primusdb_up == 0
        for: 1m
        annotations:
          summary: "PrimusDB server is down"

      - alert: HighMemoryUsage
        expr: primusdb_memory_usage_bytes > 8e9
        for: 5m
        annotations:
          summary: "Memory usage above 8 GB"

      - alert: HighErrorRate
        expr: rate(primusdb_http_requests_total{status=~"5.."}[5m]) > 0.1
        for: 5m
        annotations:
          summary: "HTTP 5xx error rate above 10%"

      - alert: ClusterDegraded
        expr: primusdb_cluster_nodes_active / primusdb_cluster_nodes_total < 0.75
        for: 2m
        annotations:
          summary: "Less than 75% of cluster nodes are active"
```

## Alpha Limitations

As of v1.3.2-alpha:

- **Not all engine metrics are wired** — per-engine operation durations and cache metrics may report zero.
- **Histogram buckets** are pre-configured and not user-configurable at runtime.
- **Custom metrics** cannot be added without modifying the source.
- **Metrics endpoint** is served on the same port as the API (no separate metrics port in this release).
