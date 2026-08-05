# Time Series API Reference

Complete REST API reference for the time series engine. All endpoints are under the `/api/v1/timeseries/` path.

---

## List Metrics

List all registered time series metrics.

```
GET /api/v1/timeseries/metrics
```

**Response:**

```json
{
  "success": true,
  "data": [
    {
      "name": "cpu_usage",
      "description": "",
      "unit": "",
      "created_at": 1720000000000,
      "updated_at": 1720000000000,
      "chunk_duration_ms": 86400000,
      "resolutions": [
        {"resolution": "raw", "retention_days": 0, "aggregation_fn": "avg"}
      ],
      "tags": ["host"],
      "field_names": ["cpu", "mem"]
    }
  ]
}
```

---

## Describe Metric

Get detailed metadata for a specific metric.

```
GET /api/v1/timeseries/metrics/{metric}
```

**Response:**

```json
{
  "success": true,
  "data": {
    "name": "cpu_usage",
    "description": "Server CPU utilization",
    "unit": "percent",
    "created_at": 1720000000000,
    "updated_at": 1720000000000,
    "chunk_duration_ms": 86400000,
    "tag_index_count": 2,
    "resolutions": [
      {"resolution": "raw", "retention_days": 0, "aggregation_fn": "avg"}
    ]
  }
}
```

---

## Insert Data Point

Insert a single time series data point.

```
POST /api/v1/timeseries/insert
```

**Request:**

```json
{
  "metric": "cpu_usage",
  "timestamp": 1720000000000,
  "fields": {"cpu": 55.5, "mem": 2048.0},
  "tags": {"host": "web1", "region": "us-east"}
}
```

**Response:**

```json
{
  "success": true,
  "data": "Inserted 1 point(s) for metric 'cpu_usage' at timestamp 1720000000000"
}
```

---

## Query Data Points

Query raw data points with optional filtering.

```
GET /api/v1/timeseries/{metric}/query
```

**Query Parameters:**

| Parameter | Type | Default | Description |
|---|---|---|---|
| `start_time` | int | `0` | Start timestamp (ms) |
| `end_time` | int | now | End timestamp (ms) |
| `tags` | string (JSON) | `null` | Tag filter |
| `fields` | string (csv) | `null` | Field filter |
| `limit` | int | `10000` | Max points |

**Response:**

```json
{
  "success": true,
  "data": [
    {
      "timestamp": 1000,
      "tags": {"host": "web1"},
      "fields": {"cpu": 50.5, "mem": 1024.0}
    }
  ]
}
```

---

## Aggregate

Aggregate data points into time buckets.

```
POST /api/v1/timeseries/{metric}/aggregate
```

**Request:**

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

**Parameters:**

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `fn` | string | yes | — | Aggregation function |
| `resolution` | string | no | `"raw"` | Bucket size |
| `start_time` | int | no | `0` | Start of range |
| `end_time` | int | no | now | End of range |
| `tags` | object | no | `null` | Tag filter |
| `fill_policy` | string | no | `null` | Gap fill policy |

**Response:**

```json
{
  "success": true,
  "data": [
    {
      "timestamp": 300000,
      "fields": {"cpu_avg": 55.3}
    }
  ]
}
```

---

## Downsample

Downsample data from one resolution to another.

```
POST /api/v1/timeseries/{metric}/downsample
```

**Request:**

```json
{
  "metric": "cpu_usage",
  "target_resolution": "1h",
  "source_resolution": "raw"
}
```

**Response:**

```json
{
  "success": true,
  "data": "Downsampled 'cpu_usage' from 'raw' to '1h'"
}
```

---

## Set Retention

Set or update the retention policy for a metric.

```
POST /api/v1/timeseries/{metric}/retain
```

**Request:**

```json
{
  "metric": "cpu_usage",
  "retention_days": 30
}
```

**Response:**

```json
{
  "success": true,
  "data": "Retention for 'cpu_usage' set to 30 day(s)"
}
```

---

## Add Resolution

Add a rollup resolution to a metric.

```
POST /api/v1/timeseries/{metric}/resolution
```

**Request:**

```json
{
  "metric": "cpu_usage",
  "resolution": "1h",
  "retention_days": 365,
  "aggregation_fn": "avg"
}
```

**Response:**

```json
{
  "success": true,
  "data": "Added resolution '1h' to metric 'cpu_usage'"
}
```

---

## Engine Statistics

Get global time series engine statistics.

```
GET /api/v1/timeseries/stats
```

**Response:**

```json
{
  "success": true,
  "data": {
    "total_metrics": 5,
    "total_chunks": 12,
    "total_points": 15000,
    "total_tag_entries": 300
  }
}
```

---

## CRUD Operations (Generic)

The time series engine also responds to the generic CRUD endpoints using `StorageType::TimeSeries`:

| Operation | Endpoint | Description |
|---|---|---|
| Insert | `POST /api/v1/crud/timeseries/{metric}` | Insert one or more points |
| Select | `GET /api/v1/crud/timeseries/{metric}` | Query points |
| Delete | `DELETE /api/v1/crud/timeseries/{metric}` | Delete points or metric |

### Batch Insert via CRUD

```
POST /api/v1/crud/timeseries/{metric}
```

```json
[
  {"timestamp": 1000, "fields": {"cpu": 50.5}, "tags": {"host": "web1"}},
  {"timestamp": 2000, "fields": {"cpu": 65.0}, "tags": {"host": "web1"}}
]
```

**Response:**

```json
{
  "success": true,
  "data": {"count": 2}
}
```

### Delete via CRUD

```
DELETE /api/v1/crud/timeseries/{metric}
```

with body `{"start_time": 1000, "end_time": 5000}` — deletes points in the range.

with empty body `{}` — deletes the entire metric.
