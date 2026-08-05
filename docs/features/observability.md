# Observability

PrimusDB exposes health checks, Prometheus metrics, structured tracing, and
diagnostic commands to help operators monitor and debug the system.

---

## Health Checks

Two HTTP endpoints provide liveness and readiness information:

| Endpoint   | Method | Description                            |
|------------|--------|----------------------------------------|
| `/health`  | GET    | Lightweight liveness check (always 200)|
| `/status`  | GET    | Detailed system status (version, uptime, engine stats) |

```bash
curl http://localhost:8080/health
# → {"success":true,"data":{"status":"healthy","node_id":"...","instance_id":"...","version":"1.3.2-alpha","uptime_seconds":12345,"architecture":"centralized"}}

curl http://localhost:8080/status
# → {"success":true,"data":{"status":"running","uptime_seconds":12345,"version":"1.3.2-alpha","storage_engines":{"columnar":"available","vector":"available","document":"available","relational":"available","keyvalue":"available"},"ai_enabled":true,"cache_enabled":true,"transactions_enabled":true}}
```

The `primusdb server health` CLI command wraps these checks:

```bash
# Quick check
primusdb server health

# Deep check — exercises each storage engine
primusdb server health --deep
```

---

## Prometheus Metrics

Metrics are served in Prometheus text exposition format at `/metrics`:

```bash
curl http://localhost:8080/metrics
```

### Key Metrics

| Metric Name                                    | Type      | Description                              |
|------------------------------------------------|-----------|------------------------------------------|
| `primusdb_federation_clusters_online`          | Gauge     | Online federation clusters               |
| `primusdb_federation_clusters_total`           | Gauge     | Registered federation clusters           |
| `primusdb_federation_domains_total`            | Gauge     | Data domains count                       |
| `primusdb_federation_healthy_ratio`            | Gauge     | Ratio of online to total clusters        |
| `primusdb_federation_announce_cycles_total`    | Counter   | Federation heartbeat cycles              |
| `primusdb_federation_replications_total`       | Counter   | Cross-cluster replications               |
| `primusdb_federation_replication_failures_total`| Counter  | Replication failures                     |
| `primusdb_federation_replication_latency_seconds`| Histogram | Replication duration                  |
| `primusdb_kv_operations_total`                 | Counter   | Key-value operations                     |
| `primusdb_kv_operation_errors_total`           | Counter   | Key-value operation errors               |
| `primusdb_kv_operation_latency_seconds`         | Histogram| Key-value operation latency              |
| `primusdb_kv_databases_total`                  | Gauge     | KV database count                        |
| `primusdb_kv_documents_total`                  | Gauge     | KV document count                        |
| `primusdb_ai_predictions_total`                | Counter   | AI predictions made                      |
| `primusdb_ai_training_total`                   | Counter   | AI training runs completed               |
| `primusdb_ai_anomaly_detected_total`           | Counter   | Anomalies detected by AI                 |

The `primusdb metrics` CLI command displays a live view:

```bash
# All metrics
primusdb metrics

# Filter by name pattern
primusdb metrics --filter storage

# Live watch, refreshed every 5 seconds
primusdb metrics --watch --interval 5
```

Metrics are implemented via the `prometheus` crate (v0.13) with a global
singleton (`get_metrics()`) and a custom `Registry` that is separate from the
default process registry.

---

## Tracing

PrimusDB uses the [`tokio-rs/tracing`](https://crates.io/crates/tracing)
framework for structured, async-aware diagnostics.

- **Spans** are created for every significant operation: `train`, `predict`,
  `detect_anomalies`, `analyze`, and all storage engine CRUD calls.
- **Events** (log messages) record errors, warnings, and informational state
  changes.
- **Timing** — each span records a `duration_ms` field automatically.

The default subscriber prints human-readable logs to stderr.  Configure the
level with the `RUST_LOG` environment variable:

```bash
# Info-level logs (default)
RUST_LOG=info primusdb server start

# Debug-level (verbose)
RUST_LOG=debug primusdb server start

# Trace-level (very verbose, includes all spans and events)
RUST_LOG=trace primusdb server start

# Per-module control
RUST_LOG=primusdb=debug,hyper=warn primusdb server start
```

Tower-http's trace middleware is enabled via the `trace` feature, providing
per-request HTTP spans.

---

## CLI Diagnostic Commands

### `primusdb doctor`

Run built-in diagnostic checks:

```bash
# Quick health check
primusdb doctor

# Comprehensive diagnostics (engine integrity, namespace consistency)
primusdb doctor --aggressive

# Save report to file
primusdb doctor --aggressive --report /tmp/diagnostic-report.txt
```

The doctor inspects:
- All registered storage engines (can they be instantiated?)
- Namespace store connectivity
- Basic CRUD round-trip (insert + select + delete)
- Encryption key availability

### `primusdb engine metrics`

Per-engine performance counters:

```bash
primusdb engine metrics columnar
primusdb engine metrics relational --filter latency
```

---

## Logging Configuration

Logging is controlled by `RUST_LOG` at runtime.  When starting via the CLI or
config file, the `--log-level` flag sets `RUST_LOG` implicitly:

```bash
primusdb server start --log-level debug
```

Equivalent to:

```bash
RUST_LOG=debug primusdb server start
```

---

## Status

**Stable.**  Health checks, Prometheus metrics, tracing, and the `primusdb
doctor` / `primusdb metrics` commands are all fully implemented and tested.

Note: the metrics registry currently focuses on **federation** and **key-value**
operations.  Per-engine metrics (cache hit ratio, compression ratio, read/write
latency) are defined in the `StorageMetrics` struct but are not yet wired to
the Prometheus registry.  That work is tracked for a future release.
