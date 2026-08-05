# Querying Time Series Data

The time series engine supports querying raw data points with optional filtering by time range, tags, and fields.

---

## Query Parameters

| Parameter | Type | Description |
|---|---|---|
| `metric` | string | Metric name |
| `start_time` | integer | Start timestamp in milliseconds (Unix epoch) |
| `end_time` | integer | End timestamp in milliseconds (Unix epoch) |
| `tags` | object | Filter by tags (e.g., `{"host": "web1"}`) |
| `fields` | array | Limit to specific field names (e.g., `["cpu", "mem"]`) |
| `limit` | integer | Maximum number of points to return (default: 10000) |
| `resolution` | string | Not used for raw queries; aggregate with `resolution` instead |

---

## REST API

### Query Data Points

```
GET /api/v1/timeseries/{metric}/query?start_time=1000&end_time=5000&limit=100
```

**Response:**

```json
{
  "success": true,
  "data": [
    {
      "timestamp": 1000,
      "tags": {"host": "web1"},
      "fields": {"cpu": 50.5, "mem": 1024.0}
    },
    {
      "timestamp": 2000,
      "tags": {"host": "web1"},
      "fields": {"cpu": 65.0, "mem": 2048.0}
    }
  ]
}
```

### Tag Filtering

Filter points by exact tag match:

```
GET /api/v1/timeseries/{metric}/query?tags={"host":"web1"}
```

The tag index is used to quickly locate matching points.

### Field Filtering

Return only specific fields:

```
GET /api/v1/timeseries/{metric}/query?fields=cpu,mem
```

---

## CLI

```bash
# Query all points for a metric
primusdb ts query cpu_usage

# Query with time range
primusdb ts query cpu_usage --start-time 1000 --end-time 5000

# Query with tag filter
primusdb ts query cpu_usage --tags '{"host": "web1"}'

# Limit results
primusdb ts query cpu_usage --limit 10
```

---

## Inserting Data

### Insert a Single Point

```
POST /api/v1/timeseries/insert
```

```json
{
  "metric": "cpu_usage",
  "timestamp": 1000,
  "fields": {"cpu": 50.5, "mem": 1024.0},
  "tags": {"host": "web1"}
}
```

### Batch Insert

Pass an array of points to the generic CRUD endpoint:

```
POST /api/v1/crud/timeseries/cpu_usage
```

```json
[
  {"timestamp": 1000, "fields": {"cpu": 50.5}, "tags": {"host": "web1"}},
  {"timestamp": 2000, "fields": {"cpu": 65.0}, "tags": {"host": "web1"}},
  {"timestamp": 3000, "fields": {"cpu": 45.0}, "tags": {"host": "web2"}}
]
```

---

## Deleting Data

### Delete Points by Time Range

```
DELETE /api/v1/crud/timeseries/{metric}
```

```json
{
  "start_time": 1000,
  "end_time": 5000
}
```

### Delete an Entire Metric

```
DELETE /api/v1/crud/timeseries/{metric}
```

```json
{}
```

---

## Resolution Configuration

Each metric stores its resolution configuration as metadata. The default resolution `raw` stores data at original granularity.

| Resolution | Bucket Size | Description |
|---|---|---|
| `raw` | N/A | Original data points |
| `1m` | 60,000 ms | 1-minute buckets |
| `5m` | 300,000 ms | 5-minute buckets |
| `15m` | 900,000 ms | 15-minute buckets |
| `1h` | 3,600,000 ms | 1-hour buckets |
| `1d` | 86,400,000 ms | 1-day buckets |
