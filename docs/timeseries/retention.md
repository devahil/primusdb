# Retention and Rollups

The time series engine supports configurable per-resolution retention policies and automatic rollup management.

---

## Retention Policies

Retention policies control how long data is kept at each resolution. When data exceeds the retention window, it is automatically purged.

### Default Behavior

- Raw data has unlimited retention by default (`retention_days = 0`)
- Rollup resolutions inherit the metric's default unless explicitly set

### Setting Retention

#### REST API

```
POST /api/v1/timeseries/{metric}/retain
```

```json
{
  "metric": "cpu_usage",
  "retention_days": 30
}
```

#### CLI

```bash
# Keep raw data for 90 days
primusdb ts retain cpu_usage 90

# Keep 1-minute rollups for 30 days
primusdb ts resolution cpu_usage 1m --retention 30
```

---

## Resolution Configuration

Each metric can have multiple resolutions (rollup tiers). Resolutions define the bucketing interval for aggregated data.

### Default Resolutions

The metric starts with a default `raw` resolution (no aggregation). Additional resolutions can be added via the API or CLI.

| Resolution | Bucket (ms) | Typical Use |
|---|---|---|
| `raw` | N/A | Original data |
| `1m` | 60,000 | Real-time dashboards |
| `5m` | 300,000 | Short-term trends |
| `15m` | 900,000 | Medium-term monitoring |
| `1h` | 3,600,000 | Long-term analysis |
| `1d` | 86,400,000 | Capacity planning |

### Adding a Resolution

```
POST /api/v1/timeseries/{metric}/resolution
```

```json
{
  "metric": "cpu_usage",
  "resolution": "1h",
  "retention_days": 365,
  "aggregation_fn": "avg"
}
```

### CLI

```bash
primusdb ts resolution cpu_usage 1h --retention 365 --agg-fn avg
```

### Parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `resolution` | string | required | Resolution identifier (`1m`, `5m`, `15m`, `1h`, `1d`) |
| `retention_days` | integer | `0` | Retention period in days (`0` = unlimited) |
| `aggregation_fn` | string | `avg` | Aggregation function for rollup computation |

---

## Retention Enforcement

Retention is enforced by the `apply_retention` method, which is called automatically during engine operations. Manual triggering is also available.

### How It Works

1. For each resolution configured on a metric, the engine calculates a cutoff timestamp: `now - (retention_days * 86400000)`
2. Chunks with `end_time < cutoff` are identified
3. The chunk trees are removed from sled
4. Updated metadata is persisted

### Force Retention Check

```
POST /api/v1/timeseries/{metric}/retain
```

with the same payload as setting retention will also trigger an immediate retention sweep.

---

## Rollup Strategy

### When Rollups Are Created

1. **Automatic**: During insert operations, if a rollup chunk for the current time window doesn't exist yet, it's created on read
2. **Manual**: Via the `downsample` endpoint, which processes existing raw data to build rollup chunks

### Rollup Storage

Rollups are stored in separate sled trees:

```
_ts_chunk_<metric>_<resolution>_<chunk_id>
```

For example, 1-hour rollups for `cpu_usage` are stored in trees prefixed with `_ts_chunk_cpu_usage_1h_`.

### Query Resolution Selection

When querying with a specific resolution, the engine:
1. Checks if rollup chunks exist at that resolution
2. If found, reads from rollup chunks (faster, less data)
3. If not found, falls back to raw data and aggregates in memory

---

## Example: Complete Retention Strategy

```bash
# Create a metric
primusdb ts describe server_metrics

# Add multiple rollup tiers
primusdb ts resolution server_metrics 5m --retention 7 --agg-fn avg
primusdb ts resolution server_metrics 1h --retention 90 --agg-fn avg
primusdb ts resolution server_metrics 1d --retention 365 --agg-fn avg

# Set raw retention
primusdb ts retain server_metrics 3
```

This configuration gives you:
- Raw data: 3 days
- 5-minute rollups: 7 days
- 1-hour rollups: 90 days
- 1-day rollups: 365 days
