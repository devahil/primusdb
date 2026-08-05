# Time Series Engine

PrimusDB includes a dedicated time series storage engine for ingesting, storing, querying, and analyzing time-series data at scale. It supports multi-resolution rollups, tag-based indexing, 15 aggregation functions, configurable retention policies, and gap filling.

---

## Status

> **Stable.** The time series engine is feature-complete and ready for production use. All functionality is covered by unit and integration tests.

---

## Features

| Feature | Description |
|---|---|
| **Chunked Storage** | Data is organized into time-bounded chunks (default 1 day) backed by sled trees for efficient range scans |
| **Multi-Resolution Rollups** | 6 built-in resolutions: raw, 1m, 5m, 15m, 1h, 1d. Custom resolutions can be added per metric |
| **15 Aggregation Functions** | avg, min, max, sum, count, stddev, median, p50, p90, p95, p99, rate, delta, first, last |
| **Tag Inverted Index** | Tags are indexed via an inverted index stored in sled for fast tag-based filtering |
| **Gap Filling** | 3 policies: previous (last observation carried forward), linear (linear interpolation), next (next observation carried backward) |
| **Configurable Retention** | Per-resolution retention policies in days; automatic purging of expired chunks |
| **REST API** | Full REST endpoints at `/api/v1/timeseries/*` |
| **CLI Commands** | 8 subcommands via `primusdb ts` |
| **Batch Operations** | Batch insert, point deletion by time range, whole-metric deletion |
| **Downsampling** | On-demand downsampling from a higher-resolution source to a lower-resolution target |

---

## Architecture

The time series engine stores data in **sled** trees with three categories of trees:

```
sled database
├── _ts_meta_*               # metric metadata trees
│   └── metric:<name>        # per-metric JSON metadata (chunk_duration, resolutions, tags, fields)
├── _ts_tags_*               # tag inverted index trees
│   └── metric:<name>        # tag -> point_id mappings for fast lookup
├── _ts_chunk_<metric>_raw_<chunk_id>   # raw data chunks
└── _ts_chunk_<metric>_<resolution>_<chunk_id>  # rollup data chunks
```

### Data Flow

```
Insert → serialize point → write raw chunk → update tag index → update metadata

Query → resolve metric → scan raw chunks → filter by time/tags → aggregate (if requested) → apply gap fill → return
```

### Chunk Format

Each chunk stores a batch of `(timestamp, tags, fields)` records sorted by timestamp. Chunk boundaries are determined by `chunk_duration_ms` (default 86,400,000 ms = 1 day). The chunk ID is a human-readable timestamp (e.g., `20260728T120000`).

---

## Configuration

Time series engine settings are part of the main `primusdb.toml` config file under `[storage]`:

```toml
[storage]
data_dir = "/var/lib/primusdb"

[timeseries]
# Default chunk duration in milliseconds (default: 86400000 = 1 day)
default_chunk_duration_ms = 86400000

# Default retention in days for raw data (0 = unlimited)
default_retention_days = 0
```

---

## CLI Quick Reference

```bash
primusdb ts list                              # List all metrics
primusdb ts describe <metric>                 # Describe a metric
primusdb ts query <metric> [options]          # Query data points
primusdb ts aggregate <metric> <fn> [options]  # Aggregate data
primusdb ts downsample <metric> <resolution>   # Downsample
primusdb ts retain <metric> <days>             # Set retention
primusdb ts resolution <metric> <resolution>   # Add rollup resolution
primusdb ts stats                             # Engine statistics
```

---

## Supported Field Types

Fields are floating-point values (`f64`). Tags are string key-value pairs used for filtering and grouping.

---

## Storage Engine Integration

The time series engine implements the `StorageEngine` trait, making it accessible through the unified PrimusDB query interface:

- **Insert** — `POST /api/v1/crud/timeseries/{metric}` with `{"timestamp": ..., "fields": {...}, "tags": {...}}`
- **Select** — `GET /api/v1/crud/timeseries/{metric}` with optional query parameters
- **Delete** — `DELETE /api/v1/crud/timeseries/{metric}` with optional time range conditions
- **Delete Metric** — `DELETE /api/v1/crud/timeseries/{metric}` with empty conditions

For timeseries-specific operations (aggregation, downsampling, retention, rollups), use the dedicated REST endpoints at `/api/v1/timeseries/*`.
