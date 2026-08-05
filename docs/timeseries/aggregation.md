# Aggregation and Gap Filling

The time series engine provides 15 aggregation functions and 3 gap-filling policies for downsampling and analyzing time-series data.

---

## Aggregation Functions

All functions operate on time-bucketed data. Points within each bucket are reduced to a single value using the selected function.

### Function Reference

| Function | Description | Output Range |
|---|---|---|
| `avg` | Arithmetic mean of values | `(-∞, ∞)` |
| `min` | Minimum value | `(-∞, ∞)` |
| `max` | Maximum value | `(-∞, ∞)` |
| `sum` | Sum of all values | `(-∞, ∞)` |
| `count` | Number of data points | `[0, ∞)` |
| `stddev` | Population standard deviation | `[0, ∞)` |
| `median` | Median value (P50) | `(-∞, ∞)` |
| `p50` | 50th percentile | `(-∞, ∞)` |
| `p90` | 90th percentile | `(-∞, ∞)` |
| `p95` | 95th percentile | `(-∞, ∞)` |
| `p99` | 99th percentile | `(-∞, ∞)` |
| `rate` | Per-second rate of change | `(-∞, ∞)` |
| `delta` | Difference between last and first value in bucket | `(-∞, ∞)` |
| `first` | First value in the bucket (by timestamp) | `(-∞, ∞)` |
| `last` | Last value in the bucket (by timestamp) | `(-∞, ∞)` |

---

## REST API

### Aggregate Endpoint

```
POST /api/v1/timeseries/{metric}/aggregate
```

```json
{
  "fn": "avg",
  "resolution": "5m",
  "start_time": 1000,
  "end_time": 60000,
  "tags": {"host": "web1"},
  "fill_policy": "previous"
}
```

**Response:**

```json
{
  "success": true,
  "data": [
    {
      "timestamp": 300000,
      "fields": {"cpu_avg": 55.3, "mem_avg": 1408.0}
    },
    {
      "timestamp": 600000,
      "fields": {"cpu_avg": 62.1, "mem_avg": 1792.0}
    }
  ]
}
```

### Parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `fn` | string | required | Aggregation function name |
| `resolution` | string | `"raw"` | Bucket size: `1m`, `5m`, `15m`, `1h`, `1d` |
| `start_time` | integer | `0` | Start of time range (ms) |
| `end_time` | integer | now | End of time range (ms) |
| `tags` | object | `null` | Filter by exact tag match |
| `fill_policy` | string | `null` | Gap filling: `previous`, `linear`, `next` |

---

## Gap Filling

Gap filling handles empty time buckets by interpolating or carrying forward values.

### Policies

| Policy | Description | Use Case |
|---|---|---|
| `previous` | Last observation carried forward (LOCF). Empty buckets take the value of the most recent non-empty bucket | System metrics where values persist until changed |
| `linear` | Linear interpolation between the nearest non-empty buckets on either side | Smooth sensor data with occasional missing samples |
| `next` | Next observation carried backward (NOCB). Empty buckets take the value of the next non-empty bucket | Leading-edge metrics where the latest value is most relevant |

### Example

Given this raw data with a gap at `t=3000`:

```
t=1000: cpu=50
t=2000: cpu=65
(t=3000: missing)
t=4000: cpu=45
```

Aggregation at `1m` resolution with different fill policies:

| Bucket | `previous` | `linear` | `next` |
|---|---|---|---|
| 1000-1999 | 50.0 | 50.0 | 50.0 |
| 2000-2999 | 65.0 | 65.0 | 65.0 |
| 3000-3999 | 65.0 | 55.0 | 45.0 |
| 4000-4999 | 45.0 | 45.0 | 45.0 |

---

## CLI

```bash
# Average CPU usage in 5-minute buckets
primusdb ts aggregate cpu_usage avg --resolution 5m

# P99 latency with previous-value gap filling
primusdb ts aggregate latency p99 --resolution 1m --fill-policy previous

# Count of requests by tag
primusdb ts aggregate requests count --resolution 1h --tags '{"region": "us-east"}'
```

---

## Downsampling

Downsampling creates a lower-resolution rollup by aggregating existing data. This is useful for pre-computing rollups for frequently-queried time ranges.

```
POST /api/v1/timeseries/{metric}/downsample
```

```json
{
  "metric": "cpu_usage",
  "target_resolution": "1h",
  "source_resolution": "raw"
}
```

The downsampling operation:
1. Reads all raw (or source resolution) chunks
2. Applies the metric's configured aggregation function for each target bucket
3. Stores the result in `_ts_chunk_<metric>_<resolution>_<chunk_id>` trees
