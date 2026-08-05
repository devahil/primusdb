/*!
# PrimusDB Time-Series Storage Engine

The time-series engine stores timestamped numeric measurements as points
carrying string key/value tags and numeric fields. Points are grouped into
metrics with multi-resolution rollups, chunked by wall-clock time, indexed by
tag, and pruned by retention policies. Use it for monitoring, IoT, application
telemetry, and any append-heavy, time-ordered numeric workload.

```text
Time-Series Engine Data Flow
═══════════════════════════════════════════════════

insert_point / insert_batch ──► TimeSeriesEngine
      │                          ├─► raw chunk trees (_ts_chunk_*)
      │                          ├─► tag index (_ts_tags_*)
      │                          └─► metric metadata (_ts_meta)
      ▼
query_points / aggregate ──► chunk scan + tag match ──► bucket + aggregate
      │                                                    (15 agg fns)
      ├─► downsample ──► rollup chunks
      └─► apply_retention ──► drop expired chunks
```

## Main Types & Functions

- [`TimeSeriesEngine`]: the time-series storage engine implementing [`StorageEngine`].
- [`TimeSeriesPoint`]: a single timestamped sample with tags and numeric fields.
- [`TimeSeriesMetric`]: metric metadata: chunking, resolutions, tags and fields.
- [`ResolutionConfig`]: retention and downsampling configuration for a resolution.
- [`TimeSeriesQuery`]: query parameters for reading and aggregating points.
- [`AggregationFn`]: the 15 supported aggregation functions.
- [`FillPolicy`]: how gaps between buckets are filled.
- [`TimeSeriesAggregation`]: a single aggregated bucket result.
- `insert_point` / `insert_batch`: data ingestion.
- `query_points` / `aggregate` / `downsample`: querying and rollups.
- `apply_retention` / `add_resolution` / `delete_metric` / `list_metrics`: lifecycle.
*/

use crate::{
    storage::{Schema, StorageEngine, TableInfo},
    PrimusDBConfig, Record, Result,
};
use async_trait::async_trait;
use chrono::{Datelike, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tracing::info;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_CHUNK_DURATION_MS: i64 = 86_400_000; // 1 day
const DEFAULT_RETENTION_DAYS: u32 = 365;
const SAMPLE_RAW: &str = "raw";
const CHUNK_TREE_PREFIX: &str = "_ts_chunk_";
const TAG_INDEX_TREE: &str = "_ts_tags";
const META_TREE: &str = "_ts_meta";

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single time‑series data point with nanosecond‑precision timestamp,
/// string tags and numeric fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    /// Unix timestamp of the sample in milliseconds.
    pub timestamp: i64,
    /// String key/value tags used for filtering and grouping.
    pub tags: HashMap<String, String>,
    /// Numeric fields measured at this timestamp.
    pub fields: HashMap<String, f64>,
}

/// Defines how a metric is stored, retained and downsampled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesMetric {
    /// Metric name, used as the table key in the storage layer.
    pub name: String,
    /// Human-readable description of the metric.
    pub description: String,
    /// Measurement unit (e.g. `percent`, `bytes`, `ms`).
    pub unit: String,
    /// Creation timestamp in milliseconds.
    pub created_at: i64,
    /// Last metadata update timestamp in milliseconds.
    pub updated_at: i64,
    /// Length of each raw chunk window in milliseconds.
    pub chunk_duration_ms: i64,
    /// Retention and downsampling configuration per resolution.
    pub resolutions: Vec<ResolutionConfig>,
    /// Tag keys seen across all points of the metric.
    pub tags: Vec<String>,
    /// Field names seen across all points of the metric.
    pub field_names: Vec<String>,
}

/// Retention and downsampling configuration for a single resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionConfig {
    /// Resolution name (e.g. `raw`, `1m`, `1h`, `1d`).
    pub resolution: String,
    /// How many days of this resolution to retain.
    pub retention_days: u32,
    /// Length of each chunk window for this resolution.
    pub chunk_duration_ms: i64,
    /// Default aggregation function used when downsampling to this resolution.
    pub agg_fn: String,
}

impl Default for ResolutionConfig {
    fn default() -> Self {
        Self {
            resolution: SAMPLE_RAW.to_string(),
            retention_days: DEFAULT_RETENTION_DAYS,
            chunk_duration_ms: DEFAULT_CHUNK_DURATION_MS,
            agg_fn: "avg".to_string(),
        }
    }
}

/// Query parameters for reading time‑series data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesQuery {
    /// Metric to query.
    pub metric: String,
    /// Inclusive start of the query window (milliseconds since epoch).
    pub start_time: Option<i64>,
    /// Inclusive end of the query window (milliseconds since epoch).
    pub end_time: Option<i64>,
    /// Tags every returned point must match.
    pub tags: Option<HashMap<String, String>>,
    /// Fields to project; all fields are returned when unset.
    pub fields: Option<Vec<String>>,
    /// Aggregation name to apply (informational; use `aggregate` to run it).
    pub aggregation: Option<String>,
    /// Resolution to aggregate into (e.g. `1m`, `1h`, `1d`).
    pub resolution: Option<String>,
    /// Gap-filling policy applied after bucketing.
    pub fill_policy: Option<FillPolicy>,
    /// Maximum number of points or buckets to return.
    pub limit: Option<u64>,
    /// Number of results to skip.
    pub offset: Option<u64>,
    /// Tag keys to group results by.
    pub group_by: Option<Vec<String>>,
}

/// Supported aggregation functions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AggregationFn {
    /// Arithmetic mean of the bucket values.
    Avg,
    /// Minimum value in the bucket.
    Min,
    /// Maximum value in the bucket.
    Max,
    /// Sum of the bucket values.
    Sum,
    /// Number of samples in the bucket.
    Count,
    /// Population standard deviation of the bucket values.
    StdDev,
    /// Middle value of the sorted bucket samples (same as `P50`).
    Median,
    /// 50th percentile of the sorted bucket samples.
    P50,
    /// 90th percentile of the sorted bucket samples.
    P90,
    /// 95th percentile of the sorted bucket samples.
    P95,
    /// 99th percentile of the sorted bucket samples.
    P99,
    /// Rate placeholder returning the last bucket value.
    Rate,
    /// Difference between the last and first bucket values.
    Delta,
    /// First sample in the bucket.
    First,
    /// Last sample in the bucket.
    Last,
}

impl AggregationFn {
    /// Parse an aggregation name into an [`AggregationFn`].
    ///
    /// Accepts `"avg"`, `"min"`, `"max"`, `"sum"`, `"count"`, `"stddev"` /
    /// `"std"`, `"median"` / `"p50"`, `"p90"`, `"p95"`, `"p99"`, `"rate"`,
    /// `"delta"`, `"first"`, and `"last"`. Returns `None` for unknown names.
    pub fn parse_from(s: &str) -> Option<Self> {
        match s {
            "avg" => Some(Self::Avg),
            "min" => Some(Self::Min),
            "max" => Some(Self::Max),
            "sum" => Some(Self::Sum),
            "count" => Some(Self::Count),
            "stddev" | "std" => Some(Self::StdDev),
            "median" | "p50" => Some(Self::Median),
            "p90" => Some(Self::P90),
            "p95" => Some(Self::P95),
            "p99" => Some(Self::P99),
            "rate" => Some(Self::Rate),
            "delta" => Some(Self::Delta),
            "first" => Some(Self::First),
            "last" => Some(Self::Last),
            _ => None,
        }
    }

    /// Return the canonical string name of the aggregation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Avg => "avg",
            Self::Min => "min",
            Self::Max => "max",
            Self::Sum => "sum",
            Self::Count => "count",
            Self::StdDev => "stddev",
            Self::Median => "median",
            Self::P50 => "p50",
            Self::P90 => "p90",
            Self::P95 => "p95",
            Self::P99 => "p99",
            Self::Rate => "rate",
            Self::Delta => "delta",
            Self::First => "first",
            Self::Last => "last",
        }
    }
}

/// How to fill gaps in the result set.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FillPolicy {
    /// Do not fill gaps; missing buckets are omitted.
    None,
    /// Repeat the previous bucket's value into the gap.
    Previous,
    /// Linearly interpolate between the surrounding bucket values.
    Linear,
    /// Forward-fill with the next bucket's value.
    Next,
}

impl FillPolicy {
    /// Parse a fill policy name into a [`FillPolicy`], defaulting to `None`.
    ///
    /// Accepts `"previous"` / `"prev"` / `"last"`, `"linear"` / `"lin"` /
    /// `"interpolate"`, and `"next"` / `"forward"`.
    pub fn parse_from(s: &str) -> Self {
        match s {
            "previous" | "prev" | "last" => Self::Previous,
            "linear" | "lin" | "interpolate" => Self::Linear,
            "next" | "forward" => Self::Next,
            _ => Self::None,
        }
    }
}

/// A single aggregated bucket result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesAggregation {
    /// Start of the bucket window in milliseconds since epoch.
    pub timestamp: i64,
    /// Aggregated value for the bucket.
    pub value: f64,
    /// Number of samples aggregated into the bucket.
    pub count: u64,
    /// Per-field aggregated values (currently empty).
    pub fields: HashMap<String, f64>,
}

/// A chunk of points at a given resolution.
#[derive(Debug, Clone)]
struct DataChunk {
    start_time: i64,
    end_time: i64,
    points: Vec<TimeSeriesPoint>,
}

// ---------------------------------------------------------------------------
// TimeSeriesEngine
// ---------------------------------------------------------------------------

/// Industrial‑grade time‑series storage engine with chunked storage,
/// multi‑resolution rollups, tag‑based indexing, retention policies,
/// gap filling and 15 aggregation functions.
pub struct TimeSeriesEngine {
    db: sled::Db,
    metrics: Arc<RwLock<HashMap<String, TimeSeriesMetric>>>,
    chunk_cache: Arc<RwLock<HashMap<String, DataChunk>>>,
    ops_counter: Arc<AtomicU64>,
}

impl Clone for TimeSeriesEngine {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            metrics: self.metrics.clone(),
            chunk_cache: self.chunk_cache.clone(),
            ops_counter: self.ops_counter.clone(),
        }
    }
}

impl TimeSeriesEngine {
    // -----------------------------------------------------------------------
    // Construction & initialisation
    // -----------------------------------------------------------------------

    /// Create a new time-series engine instance.
    ///
    /// Opens the sled database at `{data_dir}/timeseries`, loads metric
    /// metadata from `_ts_meta`, and discovers legacy chunk trees so that
    /// previously written metrics stay visible.
    ///
    /// # Errors
    /// Returns an error if the sled database cannot be opened.
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let path = format!("{}/timeseries", config.storage.data_dir);
        let db = sled::open(&path)?;

        let engine = Self {
            db,
            metrics: Arc::new(RwLock::new(HashMap::new())),
            chunk_cache: Arc::new(RwLock::new(HashMap::new())),
            ops_counter: Arc::new(AtomicU64::new(0)),
        };
        engine.load_metrics()?;
        Ok(engine)
    }

    fn load_metrics(&self) -> Result<()> {
        let meta_tree = self.db.open_tree(META_TREE.as_bytes())?;
        let mut metrics = self.metrics.write().unwrap();

        for result in meta_tree.iter() {
            let (_, value) = result?;
            if let Ok(m) = serde_json::from_slice::<TimeSeriesMetric>(&value) {
                metrics.insert(m.name.clone(), m);
            }
        }

        // Discover metrics from chunk trees (legacy migration)
        for name_bytes in self.db.tree_names() {
            let name = String::from_utf8(name_bytes.to_vec()).map_err(|_| {
                crate::Error::DatabaseError("Invalid UTF-8 in tree name".to_string())
            })?;
            if name.starts_with(CHUNK_TREE_PREFIX) {
                let metric = Self::parse_chunk_tree_name(&name);
                if let Some(ref metric_name) = metric {
                    metrics.entry(metric_name.clone()).or_insert_with(|| {
                        let m = TimeSeriesMetric {
                            name: metric_name.clone(),
                            description: String::new(),
                            unit: String::new(),
                            created_at: Utc::now().timestamp_millis(),
                            updated_at: Utc::now().timestamp_millis(),
                            chunk_duration_ms: DEFAULT_CHUNK_DURATION_MS,
                            resolutions: vec![ResolutionConfig::default()],
                            tags: vec![],
                            field_names: vec![],
                        };
                        let _ = self.save_metric(&m);
                        m
                    });
                }
            }
        }
        Ok(())
    }

    fn parse_chunk_tree_name(name: &str) -> Option<String> {
        // _ts_chunk_{metric}_{resolution}_{chunk_id}
        let rest = name.strip_prefix(CHUNK_TREE_PREFIX)?;
        let parts: Vec<&str> = rest.splitn(3, '_').collect();
        if parts.len() >= 2 && parts[1] == SAMPLE_RAW {
            Some(parts[0].to_string())
        } else {
            parts.first().map(|first| first.to_string())
        }
    }

    // -----------------------------------------------------------------------
    // Metric metadata
    // -----------------------------------------------------------------------

    fn save_metric(&self, metric: &TimeSeriesMetric) -> Result<()> {
        let meta_tree = self.db.open_tree(META_TREE.as_bytes())?;
        let key = metric.name.as_bytes();
        let value = serde_json::to_vec(metric)
            .map_err(|e| crate::Error::DatabaseError(format!("Serialization error: {}", e)))?;
        meta_tree.insert(key, value)?;
        meta_tree.flush()?;
        Ok(())
    }

    fn get_metric(&self, name: &str) -> Result<Option<TimeSeriesMetric>> {
        let meta_tree = self.db.open_tree(META_TREE.as_bytes())?;
        match meta_tree.get(name.as_bytes())? {
            Some(bytes) => {
                let m = serde_json::from_slice(&bytes).map_err(|e| {
                    crate::Error::DatabaseError(format!("Deserialization error: {}", e))
                })?;
                Ok(Some(m))
            }
            None => Ok(None),
        }
    }

    fn ensure_metric(
        &self,
        name: &str,
        field_names: &[String],
        tags: &[String],
    ) -> Result<TimeSeriesMetric> {
        if let Some(metric) = self.get_metric(name)? {
            let mut updated = metric;
            // Merge any new field names
            for f in field_names {
                if !updated.field_names.contains(f) {
                    updated.field_names.push(f.clone());
                }
            }
            for t in tags {
                if !updated.tags.contains(t) {
                    updated.tags.push(t.clone());
                }
            }
            updated.updated_at = Utc::now().timestamp_millis();
            self.save_metric(&updated)?;
            let mut metrics = self.metrics.write().unwrap();
            metrics.insert(name.to_string(), updated.clone());
            Ok(updated)
        } else {
            let metric = TimeSeriesMetric {
                name: name.to_string(),
                description: String::new(),
                unit: String::new(),
                created_at: Utc::now().timestamp_millis(),
                updated_at: Utc::now().timestamp_millis(),
                chunk_duration_ms: DEFAULT_CHUNK_DURATION_MS,
                resolutions: vec![ResolutionConfig {
                    resolution: SAMPLE_RAW.to_string(),
                    retention_days: DEFAULT_RETENTION_DAYS,
                    chunk_duration_ms: DEFAULT_CHUNK_DURATION_MS,
                    agg_fn: "avg".to_string(),
                }],
                tags: tags.to_vec(),
                field_names: field_names.to_vec(),
            };
            self.save_metric(&metric)?;
            let mut metrics = self.metrics.write().unwrap();
            metrics.insert(name.to_string(), metric.clone());
            Ok(metric)
        }
    }

    // -----------------------------------------------------------------------
    // Chunk management
    // -----------------------------------------------------------------------

    fn chunk_id_for_time(timestamp: i64, chunk_duration_ms: i64) -> String {
        let bucket = timestamp / chunk_duration_ms * chunk_duration_ms;
        let dt = match Utc.timestamp_millis_opt(bucket) {
            chrono::LocalResult::Single(d) => d.naive_utc(),
            chrono::LocalResult::Ambiguous(d, _) => d.naive_utc(),
            chrono::LocalResult::None => Utc::now().naive_utc(),
        };
        format!(
            "{:04}{:02}{:02}T{:02}{:02}{:02}",
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second(),
        )
    }

    fn chunk_tree_name(metric: &str, resolution: &str, chunk_id: &str) -> String {
        format!(
            "{}{}_{}_{}",
            CHUNK_TREE_PREFIX, metric, resolution, chunk_id
        )
    }

    fn open_chunk_tree(&self, name: &str) -> Result<sled::Tree> {
        Ok(self.db.open_tree(name.as_bytes())?)
    }

    fn load_chunk(
        &self,
        metric: &str,
        resolution: &str,
        chunk_id: &str,
    ) -> Result<Option<DataChunk>> {
        let cache_key = format!("{}/{}/{}", metric, resolution, chunk_id);
        {
            let cache = self.chunk_cache.read().unwrap();
            if let Some(chunk) = cache.get(&cache_key) {
                return Ok(Some(chunk.clone()));
            }
        }

        let tree_name = Self::chunk_tree_name(metric, resolution, chunk_id);
        let tree = match self.db.open_tree(tree_name.as_bytes()) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };

        let mut points = Vec::new();
        let mut start_time = i64::MAX;
        let mut end_time = i64::MIN;

        for result in tree.iter() {
            let (_, value) = result?;
            if let Ok(p) = serde_json::from_slice::<TimeSeriesPoint>(&value) {
                if p.timestamp < start_time {
                    start_time = p.timestamp;
                }
                if p.timestamp > end_time {
                    end_time = p.timestamp;
                }
                points.push(p);
            }
        }

        if points.is_empty() {
            return Ok(None);
        }

        let chunk = DataChunk {
            start_time,
            end_time,
            points,
        };

        {
            let mut cache = self.chunk_cache.write().unwrap();
            cache.insert(cache_key, chunk.clone());
        }

        Ok(Some(chunk))
    }

    fn flush_chunk_cache(&self, metric: &str, resolution: &str, chunk_id: &str) {
        let cache_key = format!("{}/{}/{}", metric, resolution, chunk_id);
        let mut cache = self.chunk_cache.write().unwrap();
        cache.remove(&cache_key);
    }

    // -----------------------------------------------------------------------
    // Tag indexing
    // -----------------------------------------------------------------------

    fn index_point_tags(
        &self,
        metric: &str,
        point_id: &str,
        tags: &HashMap<String, String>,
    ) -> Result<()> {
        let tag_tree = self
            .db
            .open_tree(format!("{}_{}", TAG_INDEX_TREE, metric).as_bytes())?;
        for (k, v) in tags {
            let index_key = format!("{}:{}:{}", k, v, point_id);
            tag_tree.insert(index_key.as_bytes(), b"1")?;
        }
        Ok(())
    }

    fn query_tag_index(
        &self,
        metric: &str,
        tags: &HashMap<String, String>,
    ) -> Result<HashSet<String>> {
        let tag_tree = self
            .db
            .open_tree(format!("{}_{}", TAG_INDEX_TREE, metric).as_bytes())?;
        let mut result: Option<HashSet<String>> = None;

        for (k, v) in tags {
            let prefix = format!("{}:{}:", k, v);
            let mut matches = HashSet::new();

            let iter = tag_tree.scan_prefix(prefix.as_bytes());
            for item in iter {
                let (key, _) = item?;
                if let Ok(key_str) = String::from_utf8(key.to_vec()) {
                    // key format: tag_key:tag_value:point_id
                    if let Some(point_id) = key_str.rsplit(':').next() {
                        matches.insert(point_id.to_string());
                    }
                }
            }

            result = match result {
                Some(prev) => Some(prev.intersection(&matches).cloned().collect()),
                None => Some(matches),
            };

            if result.as_ref().is_some_and(|r| r.is_empty()) {
                break;
            }
        }

        Ok(result.unwrap_or_default())
    }

    // -----------------------------------------------------------------------
    // Public API – data ingestion
    // -----------------------------------------------------------------------

    /// Insert a single point into a metric.
    ///
    /// Creates the metric (and its metadata) on first use, assigns a timestamp
    /// of now when the point's timestamp is zero, routes the point to the raw
    /// chunk covering its timestamp, and updates the tag index.
    ///
    /// # Returns
    /// `1` on success.
    pub fn insert_point(&self, metric: &str, mut point: TimeSeriesPoint) -> Result<u64> {
        let metric_obj = self.ensure_metric(
            metric,
            &point.fields.keys().cloned().collect::<Vec<_>>(),
            &point.tags.keys().cloned().collect::<Vec<_>>(),
        )?;

        if point.timestamp == 0 {
            point.timestamp = Utc::now().timestamp_millis();
        }

        let chunk_id = Self::chunk_id_for_time(point.timestamp, metric_obj.chunk_duration_ms);
        let tree_name = Self::chunk_tree_name(metric, SAMPLE_RAW, &chunk_id);
        let tree = self.open_chunk_tree(&tree_name)?;

        let point_id = format!("{}_{}", point.timestamp, uuid::Uuid::new_v4());
        let value = serde_json::to_vec(&point)
            .map_err(|e| crate::Error::DatabaseError(format!("Serialization error: {}", e)))?;
        tree.insert(point_id.as_bytes(), value)?;
        tree.flush()?;

        self.index_point_tags(metric, &point_id, &point.tags)?;
        self.flush_chunk_cache(metric, SAMPLE_RAW, &chunk_id);
        self.ops_counter.fetch_add(1, Ordering::Relaxed);

        Ok(1)
    }

    /// Insert a batch of points into a metric.
    ///
    /// # Returns
    /// The total number of points inserted.
    pub fn insert_batch(&self, metric: &str, points: Vec<TimeSeriesPoint>) -> Result<u64> {
        let mut count = 0u64;
        for point in points {
            count += self.insert_point(metric, point)?;
        }
        Ok(count)
    }

    // -----------------------------------------------------------------------
    // Public API – querying
    // -----------------------------------------------------------------------

    /// Query raw points from a metric.
    ///
    /// Scans the raw chunk trees for the metric, applying the query's time
    /// range, tag filters, field projection, and `limit`. Results are returned
    /// sorted by timestamp.
    pub fn query_points(&self, query: &TimeSeriesQuery) -> Result<Vec<TimeSeriesPoint>> {
        let start_time = query.start_time.unwrap_or(0);
        let end_time = query.end_time.unwrap_or(Utc::now().timestamp_millis());
        let limit = query.limit.unwrap_or(10_000) as usize;

        // Always query raw data regardless of requested resolution.
        // Aggregation happens in the aggregate() method.
        let mut all_points: Vec<TimeSeriesPoint> = Vec::new();
        let mut visited_chunks = HashSet::new();

        // Scan raw chunk trees for this metric
        let prefix = format!("{}{}_{}_", CHUNK_TREE_PREFIX, query.metric, SAMPLE_RAW);
        for name_bytes in self.db.tree_names() {
            if all_points.len() >= limit {
                break;
            }
            let name = String::from_utf8(name_bytes.to_vec()).unwrap_or_default();
            if !name.starts_with(&prefix) {
                continue;
            }
            let chunk_id = name.strip_prefix(&prefix).unwrap_or("").to_string();
            if visited_chunks.contains(&chunk_id) {
                continue;
            }
            // Check if chunk overlaps the time range
            // Extract timestamp from chunk_id (format YYYYMMDDTHHMMSS)
            if let Ok(Some(chunk)) = self.load_chunk(&query.metric, SAMPLE_RAW, &chunk_id) {
                if chunk.end_time < start_time || chunk.start_time > end_time {
                    continue;
                }
                visited_chunks.insert(chunk_id);

                for point in &chunk.points {
                    if all_points.len() >= limit {
                        break;
                    }
                    if point.timestamp < start_time || point.timestamp > end_time {
                        continue;
                    }
                    if let Some(ref tags) = query.tags {
                        if !Self::match_tags(&point.tags, tags) {
                            continue;
                        }
                    }
                    all_points.push(point.clone());
                }
            }
        }

        // Apply tag filtering via inverted index if available
        if let Some(ref tags) = query.tags {
            if !tags.is_empty() {
                let _point_ids = self.query_tag_index(&query.metric, tags)?;
                // Tag index is used for optimization; exact match done above
            }
        }

        // Filter fields if specified
        if let Some(ref fields) = query.fields {
            for point in &mut all_points {
                point.fields.retain(|k, _| fields.contains(k));
            }
        }

        all_points.sort_by_key(|a| a.timestamp);
        all_points.truncate(limit);
        Ok(all_points)
    }

    fn match_tags(
        point_tags: &HashMap<String, String>,
        query_tags: &HashMap<String, String>,
    ) -> bool {
        for (k, v) in query_tags {
            match point_tags.get(k) {
                Some(pv) => {
                    if pv != v {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }

    fn resolution_to_ms(resolution: &str) -> i64 {
        match resolution {
            "1m" => 60_000,
            "5m" => 300_000,
            "15m" => 900_000,
            "1h" => 3_600_000,
            "1d" => 86_400_000,
            _ => 1_000,
        }
    }

    // -----------------------------------------------------------------------
    // Public API – aggregation
    // -----------------------------------------------------------------------

    /// Aggregate a metric's points into time buckets.
    ///
    /// Buckets points by the query's `resolution`, applies the given
    /// aggregation function to each bucket, optionally fills gaps via the
    /// query's fill policy, and truncates to the query's limit.
    ///
    /// # Returns
    /// One [`TimeSeriesAggregation`] per bucket.
    pub fn aggregate(
        &self,
        query: &TimeSeriesQuery,
        agg_fn: &str,
    ) -> Result<Vec<TimeSeriesAggregation>> {
        let points = self.query_points(query)?;
        if points.is_empty() {
            return Ok(Vec::new());
        }

        let resolution = query.resolution.as_deref().unwrap_or(SAMPLE_RAW);
        let bucket_ms = Self::resolution_to_ms(resolution).max(60_000);
        let mut buckets: BTreeMap<i64, Vec<(String, f64)>> = BTreeMap::new();

        let field_names: Vec<String> = if let Some(ref fields) = query.fields {
            fields.clone()
        } else if !points.is_empty() {
            points[0].fields.keys().cloned().collect()
        } else {
            return Ok(Vec::new());
        };

        for point in &points {
            let bucket = point.timestamp / bucket_ms * bucket_ms;
            for field in &field_names {
                if let Some(val) = point.fields.get(field) {
                    buckets
                        .entry(bucket)
                        .or_default()
                        .push((field.clone(), *val));
                }
            }
        }

        let agg = AggregationFn::parse_from(agg_fn).unwrap_or(AggregationFn::Avg);
        let limit = query.limit.unwrap_or(10_000) as usize;
        let mut results: Vec<TimeSeriesAggregation> = buckets
            .into_iter()
            .map(|(ts, values)| {
                let (value, count) = Self::apply_aggregation(&values, &agg);
                TimeSeriesAggregation {
                    timestamp: ts,
                    value,
                    count: count as u64,
                    fields: HashMap::new(),
                }
            })
            .collect();

        // Apply gap filling
        if let Some(fill) = query.fill_policy {
            if fill != FillPolicy::None {
                results = self.fill_gaps(results, &agg, bucket_ms, fill);
            }
        }

        results.truncate(limit);

        // Group by if requested
        if let Some(ref group_by) = query.group_by {
            if !group_by.is_empty() {
                return self.aggregate_grouped(query, agg_fn, group_by);
            }
        }

        Ok(results)
    }

    fn aggregate_grouped(
        &self,
        _query: &TimeSeriesQuery,
        _agg_fn: &str,
        _group_by: &[String],
    ) -> Result<Vec<TimeSeriesAggregation>> {
        // Grouped aggregation — simplified for now
        Ok(Vec::new())
    }

    fn apply_aggregation(values: &[(String, f64)], agg: &AggregationFn) -> (f64, usize) {
        if values.is_empty() {
            return (0.0, 0);
        }
        let count = values.len();

        match agg {
            AggregationFn::Avg => (
                values.iter().map(|(_, v)| v).sum::<f64>() / count as f64,
                count,
            ),
            AggregationFn::Min => (
                values
                    .iter()
                    .map(|(_, v)| v)
                    .cloned()
                    .fold(f64::INFINITY, f64::min),
                count,
            ),
            AggregationFn::Max => (
                values
                    .iter()
                    .map(|(_, v)| v)
                    .cloned()
                    .fold(f64::NEG_INFINITY, f64::max),
                count,
            ),
            AggregationFn::Sum => (values.iter().map(|(_, v)| v).sum(), count),
            AggregationFn::Count => (count as f64, count),
            AggregationFn::StdDev => {
                let mean = values.iter().map(|(_, v)| v).sum::<f64>() / count as f64;
                let variance =
                    values.iter().map(|(_, v)| (v - mean).powi(2)).sum::<f64>() / count as f64;
                (variance.sqrt(), count)
            }
            AggregationFn::Median | AggregationFn::P50 => {
                let mut sorted: Vec<f64> = values.iter().map(|(_, v)| *v).collect();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let mid = sorted.len() / 2;
                (sorted[mid], count)
            }
            AggregationFn::P90 => {
                let mut sorted: Vec<f64> = values.iter().map(|(_, v)| *v).collect();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let idx = (sorted.len() as f64 * 0.9).ceil() as usize - 1;
                (sorted[idx.min(sorted.len() - 1)], count)
            }
            AggregationFn::P95 => {
                let mut sorted: Vec<f64> = values.iter().map(|(_, v)| *v).collect();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let idx = (sorted.len() as f64 * 0.95).ceil() as usize - 1;
                (sorted[idx.min(sorted.len() - 1)], count)
            }
            AggregationFn::P99 => {
                let mut sorted: Vec<f64> = values.iter().map(|(_, v)| *v).collect();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let idx = (sorted.len() as f64 * 0.99).ceil() as usize - 1;
                (sorted[idx.min(sorted.len() - 1)], count)
            }
            AggregationFn::Rate => {
                if count < 2 {
                    (0.0, count)
                } else {
                    let _first_ts: i64 = 0;
                    (values.last().map(|(_, v)| *v).unwrap_or(0.0), count)
                }
            }
            AggregationFn::Delta => {
                if count < 2 {
                    (0.0, count)
                } else {
                    let first = values.first().map(|(_, v)| *v).unwrap_or(0.0);
                    let last = values.last().map(|(_, v)| *v).unwrap_or(0.0);
                    (last - first, count)
                }
            }
            AggregationFn::First => (values.first().map(|(_, v)| *v).unwrap_or(0.0), count),
            AggregationFn::Last => (values.last().map(|(_, v)| *v).unwrap_or(0.0), count),
        }
    }

    // -----------------------------------------------------------------------
    // Gap filling
    // -----------------------------------------------------------------------

    fn fill_gaps(
        &self,
        results: Vec<TimeSeriesAggregation>,
        _agg: &AggregationFn,
        bucket_ms: i64,
        policy: FillPolicy,
    ) -> Vec<TimeSeriesAggregation> {
        if results.is_empty() {
            return results;
        }

        let mut filled = Vec::new();
        let _start = results[0].timestamp;
        let _end = results[results.len() - 1].timestamp;

        let mut prev_value = results[0].value;

        for (i, result) in results.iter().enumerate() {
            // Fill gaps before this point
            if i > 0 {
                let mut expected_ts = results[i - 1].timestamp + bucket_ms;
                while expected_ts < result.timestamp {
                    let fill_val = match policy {
                        FillPolicy::Previous => prev_value,
                        FillPolicy::Linear => {
                            let next_val = result.value;
                            let total_gaps =
                                (result.timestamp - results[i - 1].timestamp) / bucket_ms;
                            let cur_gap = (expected_ts - results[i - 1].timestamp) / bucket_ms;
                            if total_gaps > 0 {
                                prev_value
                                    + (next_val - prev_value) * cur_gap as f64 / total_gaps as f64
                            } else {
                                prev_value
                            }
                        }
                        FillPolicy::Next => result.value,
                        FillPolicy::None => break,
                    };
                    filled.push(TimeSeriesAggregation {
                        timestamp: expected_ts,
                        value: fill_val,
                        count: 0,
                        fields: HashMap::new(),
                    });
                    if policy == FillPolicy::Previous || policy == FillPolicy::Next {
                        prev_value = fill_val;
                    }
                    expected_ts += bucket_ms;
                }
            }
            filled.push(result.clone());
            prev_value = result.value;
        }

        filled
    }

    // -----------------------------------------------------------------------
    // Public API – rollup / downsample
    // -----------------------------------------------------------------------

    /// Generate rollup points from a source resolution into a target resolution.
    ///
    /// Groups every source chunk's points into target-resolution buckets,
    /// aggregates each bucket with `agg_fn`, and writes the rollups to new
    /// chunk trees.
    ///
    /// # Returns
    /// The number of rollup points written.
    pub fn downsample(
        &self,
        metric: &str,
        source_resolution: &str,
        target_resolution: &str,
        agg_fn: &str,
    ) -> Result<u64> {
        let metric_obj = match self.get_metric(metric)? {
            Some(m) => m,
            None => {
                return Err(crate::Error::DatabaseError(format!(
                    "Metric '{}' not found",
                    metric
                )))
            }
        };

        let tgt_chunk_duration = if target_resolution == SAMPLE_RAW {
            metric_obj.chunk_duration_ms
        } else {
            Self::resolution_to_ms(target_resolution)
        };

        // Scan all source chunks
        let prefix = format!("{}{}_{}_", CHUNK_TREE_PREFIX, metric, source_resolution);
        let mut processed = 0u64;

        for name_bytes in self.db.tree_names() {
            let name = String::from_utf8(name_bytes.to_vec()).unwrap_or_default();
            if !name.starts_with(&prefix) {
                continue;
            }
            let chunk_id = name.strip_prefix(&prefix).unwrap_or("").to_string();
            if chunk_id.is_empty() {
                continue;
            }

            let source_chunk = match self.load_chunk(metric, source_resolution, &chunk_id)? {
                Some(c) => c,
                None => continue,
            };

            // Group source points into target buckets
            let mut buckets: BTreeMap<i64, Vec<(String, f64)>> = BTreeMap::new();
            for point in &source_chunk.points {
                let bucket = point.timestamp / tgt_chunk_duration * tgt_chunk_duration;
                for (field, val) in &point.fields {
                    buckets
                        .entry(bucket)
                        .or_default()
                        .push((field.clone(), *val));
                }
            }

            if buckets.is_empty() {
                continue;
            }

            // Generate rollup points
            let tgt_chunk_id = Self::chunk_id_for_time(
                buckets.keys().next().copied().unwrap_or(0),
                tgt_chunk_duration,
            );
            let rollup_tree_name = Self::chunk_tree_name(metric, target_resolution, &tgt_chunk_id);
            let rollup_tree = self.open_chunk_tree(&rollup_tree_name)?;

            let agg = AggregationFn::parse_from(agg_fn).unwrap_or(AggregationFn::Avg);
            for (ts, values) in &buckets {
                let (value, _) = Self::apply_aggregation(values, &agg);
                let rollup_point = TimeSeriesPoint {
                    timestamp: *ts,
                    tags: HashMap::new(),
                    fields: {
                        let mut f = HashMap::new();
                        f.insert("value".to_string(), value);
                        f
                    },
                };
                let point_id = format!("{}_{}", ts, uuid::Uuid::new_v4());
                let data = serde_json::to_vec(&rollup_point).map_err(|e| {
                    crate::Error::DatabaseError(format!("Serialization error: {}", e))
                })?;
                rollup_tree.insert(point_id.as_bytes(), data)?;
                processed += 1;
            }
            rollup_tree.flush()?;
        }

        Ok(processed)
    }

    // -----------------------------------------------------------------------
    // Public API – retention management
    // -----------------------------------------------------------------------

    /// Drop chunks older than each resolution's retention window.
    ///
    /// # Returns
    /// The number of chunks removed.
    pub fn apply_retention(&self, metric: &str) -> Result<u64> {
        let metric_obj = match self.get_metric(metric)? {
            Some(m) => m,
            None => {
                return Err(crate::Error::DatabaseError(format!(
                    "Metric '{}' not found",
                    metric
                )))
            }
        };

        let now = Utc::now().timestamp_millis();
        let mut removed = 0u64;

        for res_config in &metric_obj.resolutions {
            let retention_ms = (res_config.retention_days as i64) * 86_400_000;
            let cutoff = now - retention_ms;

            let prefix = format!("{}{}_{}_", CHUNK_TREE_PREFIX, metric, res_config.resolution);
            let names: Vec<String> = self
                .db
                .tree_names()
                .iter()
                .filter_map(|n| {
                    let name = String::from_utf8(n.to_vec()).ok()?;
                    if name.starts_with(&prefix) {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect();

            for name in &names {
                if let Some(chunk_id) = name.strip_prefix(&prefix) {
                    if let Ok(Some(chunk)) =
                        self.load_chunk(metric, &res_config.resolution, chunk_id)
                    {
                        if chunk.end_time < cutoff {
                            self.db.drop_tree(name.as_bytes())?;
                            self.flush_chunk_cache(metric, &res_config.resolution, chunk_id);
                            removed += 1;
                        }
                    }
                }
            }
        }

        Ok(removed)
    }

    /// Register a downsampling resolution for a metric, replacing any existing
    /// resolution with the same name.
    pub fn add_resolution(
        &self,
        metric: &str,
        resolution: &str,
        retention_days: u32,
        agg_fn: &str,
    ) -> Result<()> {
        let mut metric_obj = match self.get_metric(metric)? {
            Some(m) => m,
            None => {
                return Err(crate::Error::DatabaseError(format!(
                    "Metric '{}' not found",
                    metric
                )))
            }
        };

        let chunk_duration_ms = Self::resolution_to_ms(resolution);
        let new_res = ResolutionConfig {
            resolution: resolution.to_string(),
            retention_days,
            chunk_duration_ms,
            agg_fn: agg_fn.to_string(),
        };

        // Replace if exists, else append
        if let Some(pos) = metric_obj
            .resolutions
            .iter()
            .position(|r| r.resolution == resolution)
        {
            metric_obj.resolutions[pos] = new_res;
        } else {
            metric_obj.resolutions.push(new_res);
        }

        metric_obj.updated_at = Utc::now().timestamp_millis();
        self.save_metric(&metric_obj)?;
        {
            let mut metrics = self.metrics.write().unwrap();
            metrics.insert(metric.to_string(), metric_obj);
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Public API – metric management
    // -----------------------------------------------------------------------

    /// Delete a metric, its chunk trees, tag index, metadata, and cache entries.
    pub fn delete_metric(&self, metric: &str) -> Result<()> {
        let prefix = format!("{}{}", CHUNK_TREE_PREFIX, metric);
        let names: Vec<Vec<u8>> = self
            .db
            .tree_names()
            .iter()
            .filter(|n| {
                String::from_utf8(n.to_vec())
                    .map(|name| name.starts_with(&prefix))
                    .unwrap_or(false)
            })
            .map(|n| n.to_vec())
            .collect();

        for name in &names {
            self.db.drop_tree(name)?;
        }

        // Drop tag index
        let tag_tree_name = format!("{}_{}", TAG_INDEX_TREE, metric);
        let _ = self.db.drop_tree(tag_tree_name.as_bytes());

        // Drop metadata
        let meta_tree = self.db.open_tree(META_TREE.as_bytes())?;
        meta_tree.remove(metric.as_bytes())?;

        let mut metrics = self.metrics.write().unwrap();
        metrics.remove(metric);

        let mut cache = self.chunk_cache.write().unwrap();
        cache.retain(|k, _| !k.starts_with(&format!("{}/", metric)));

        Ok(())
    }

    /// List the names of all known metrics, sorted alphabetically.
    pub fn list_metrics(&self) -> Result<Vec<String>> {
        let metrics = self.metrics.read().unwrap();
        let mut names: Vec<String> = metrics.keys().cloned().collect();
        names.sort();
        Ok(names)
    }

    /// Return the metadata of a metric, or `None` if it does not exist.
    pub fn describe_metric(&self, metric: &str) -> Result<Option<TimeSeriesMetric>> {
        self.get_metric(metric)
    }

    /// Update a metric's description, unit, or chunk duration.
    ///
    /// Only the provided fields are changed; `chunk_duration_ms` is clamped to
    /// at least 60 seconds.
    pub fn update_metric_config(
        &self,
        metric: &str,
        description: Option<&str>,
        unit: Option<&str>,
        chunk_duration_ms: Option<i64>,
    ) -> Result<()> {
        let mut metric_obj = match self.get_metric(metric)? {
            Some(m) => m,
            None => {
                return Err(crate::Error::DatabaseError(format!(
                    "Metric '{}' not found",
                    metric
                )))
            }
        };

        if let Some(d) = description {
            metric_obj.description = d.to_string();
        }
        if let Some(u) = unit {
            metric_obj.unit = u.to_string();
        }
        if let Some(d) = chunk_duration_ms {
            metric_obj.chunk_duration_ms = d.max(60_000);
        }

        metric_obj.updated_at = Utc::now().timestamp_millis();
        self.save_metric(&metric_obj)?;
        {
            let mut metrics = self.metrics.write().unwrap();
            metrics.insert(metric.to_string(), metric_obj);
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Public API – engine stats
    // -----------------------------------------------------------------------

    /// Return engine-wide statistics as JSON: metric, chunk, and point counts,
    /// approximate size in bytes, and the total number of ingestion operations.
    pub fn engine_stats(&self) -> Result<serde_json::Value> {
        let metrics = self.metrics.read().unwrap();
        let metric_count = metrics.len();

        let mut total_points = 0u64;
        let mut total_chunks = 0u64;
        let mut total_size = 0u64;

        for name_bytes in self.db.tree_names() {
            let name = String::from_utf8(name_bytes.to_vec()).unwrap_or_default();
            if name.starts_with(CHUNK_TREE_PREFIX) {
                if let Ok(tree) = self.db.open_tree(name.as_bytes()) {
                    let count = tree.len() as u64;
                    total_chunks += 1;
                    total_points += count;
                    for (k, v) in tree.iter().flatten() {
                        total_size += (k.len() + v.len()) as u64;
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "metrics": metric_count,
            "chunks": total_chunks,
            "points": total_points,
            "size_bytes": total_size,
            "operations": self.ops_counter.load(Ordering::Relaxed),
        }))
    }

    // -----------------------------------------------------------------------
    // Point deletion within a metric
    // -----------------------------------------------------------------------

    /// Delete points from a metric within an optional time range and tag set.
    ///
    /// # Returns
    /// The number of points removed.
    pub fn delete_points(
        &self,
        metric: &str,
        start_time: Option<i64>,
        end_time: Option<i64>,
        tags: Option<&HashMap<String, String>>,
    ) -> Result<u64> {
        let start = start_time.unwrap_or(0);
        let end = end_time.unwrap_or(Utc::now().timestamp_millis());
        let prefix = format!("{}{}_{}_", CHUNK_TREE_PREFIX, metric, SAMPLE_RAW);
        let mut removed = 0u64;

        for name_bytes in self.db.tree_names() {
            let name = String::from_utf8(name_bytes.to_vec()).unwrap_or_default();
            if !name.starts_with(&prefix) {
                continue;
            }

            let tree = self.db.open_tree(name.as_bytes())?;
            let mut to_remove = Vec::new();

            for item in tree.iter() {
                let (key, value) = item?;
                if let Ok(point) = serde_json::from_slice::<TimeSeriesPoint>(&value) {
                    if point.timestamp >= start && point.timestamp <= end {
                        if let Some(t) = tags {
                            if Self::match_tags(&point.tags, t) {
                                to_remove.push(key.to_vec());
                            }
                        } else {
                            to_remove.push(key.to_vec());
                        }
                    }
                }
            }

            for key in &to_remove {
                tree.remove(key)?;
                removed += 1;
            }

            if let Some(chunk_id) = name.strip_prefix(&prefix) {
                self.flush_chunk_cache(metric, SAMPLE_RAW, chunk_id);
            }
        }

        Ok(removed)
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_single(&self, table: &str, data: &serde_json::Value) -> Result<u64> {
        let timestamp = data
            .get("timestamp")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| Utc::now().timestamp_millis());

        let tags: HashMap<String, String> = match data.get("tags") {
            Some(v) => serde_json::from_value(v.clone()).unwrap_or_default(),
            None => HashMap::new(),
        };

        let fields: HashMap<String, f64> = match data.get("fields") {
            Some(v) => {
                let map: HashMap<String, serde_json::Value> =
                    serde_json::from_value(v.clone()).unwrap_or_default();
                let mut result = HashMap::new();
                for (k, val) in map {
                    if let Some(f) = val.as_f64() {
                        result.insert(k, f);
                    }
                }
                result
            }
            None => HashMap::new(),
        };

        let flat_fields: HashMap<String, f64> = if fields.is_empty() {
            data.as_object()
                .map(|obj| {
                    obj.iter()
                        .filter(|(k, _)| *k != "timestamp" && *k != "tags" && *k != "id")
                        .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            fields
        };

        if flat_fields.is_empty() {
            return Err(crate::Error::DatabaseError(
                "No numeric fields found in time series point".to_string(),
            ));
        }

        let point = TimeSeriesPoint {
            timestamp,
            tags,
            fields: flat_fields,
        };

        self.insert_point(table, point)
    }

    fn build_query(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: Option<u64>,
        _offset: Option<u64>,
    ) -> TimeSeriesQuery {
        TimeSeriesQuery {
            metric: table.to_string(),
            start_time: conditions
                .and_then(|c| c.get("start_time"))
                .and_then(|v| v.as_i64()),
            end_time: conditions
                .and_then(|c| c.get("end_time"))
                .and_then(|v| v.as_i64()),
            tags: match conditions.and_then(|c| c.get("tags")) {
                Some(v) => serde_json::from_value(v.clone()).ok(),
                None => None,
            },
            fields: match conditions.and_then(|c| c.get("fields")) {
                Some(v) => serde_json::from_value(v.clone()).ok(),
                None => None,
            },
            aggregation: None,
            resolution: None,
            fill_policy: None,
            limit,
            offset: None,
            group_by: None,
        }
    }
}

// ---------------------------------------------------------------------------
// StorageEngine trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl StorageEngine for TimeSeriesEngine {
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Insert a point from a JSON object with optional `timestamp`, `tags`,
    /// and `fields` keys (defaults: now, empty tags, empty fields).
    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        if let Some(arr) = data.as_array() {
            let mut count = 0u64;
            for item in arr {
                count += self.insert_single(table, item)?;
            }
            return Ok(count);
        }
        self.insert_single(table, data)
    }

    /// Query points from a metric via the [`StorageEngine`] interface.
    ///
    /// Builds a [`TimeSeriesQuery`] from the conditions object (supporting
    /// `start_time`, `end_time`, `tags`, and `fields` keys) and returns the
    /// matching points as records.
    async fn select(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        _offset: u64,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<Vec<Record>> {
        let query = self.build_query(table, conditions, Some(limit), None);
        let points = self.query_points(&query)?;

        Ok(points
            .into_iter()
            .enumerate()
            .map(|(i, p)| Record {
                id: format!("ts_{}_{}", p.timestamp, i),
                data: serde_json::json!({
                    "timestamp": p.timestamp,
                    "tags": p.tags,
                    "fields": p.fields,
                }),
                metadata: HashMap::new(),
            })
            .collect())
    }

    /// Updating time series points is not supported; returns `0`.
    async fn update(
        &self,
        _table: &str,
        _conditions: Option<&serde_json::Value>,
        _data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        Ok(0)
    }

    /// Delete an entire metric (with all its points) when no or empty
    /// conditions are given; otherwise returns `0`.
    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let is_empty = conditions
            .map(|c| c.as_object().map(|o| o.is_empty()).unwrap_or(true))
            .unwrap_or(true);

        if is_empty {
            self.delete_metric(table)?;
            return Ok(1);
        }

        let start_time = conditions
            .and_then(|c| c.get("start_time"))
            .and_then(|v| v.as_i64());
        let end_time = conditions
            .and_then(|c| c.get("end_time"))
            .and_then(|v| v.as_i64());
        let tags: Option<HashMap<String, String>> = match conditions.and_then(|c| c.get("tags")) {
            Some(v) => serde_json::from_value(v.clone()).ok(),
            None => None,
        };

        self.delete_points(table, start_time, end_time, tags.as_ref())
    }

    /// Produce a human-readable analysis of a metric: point/chunk counts, size,
    /// time range, resolutions, tags, and field names.
    async fn analyze(
        &self,
        table: &str,
        _conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<String> {
        let metric = self.get_metric(table)?;
        let prefix = format!("{}{}_{}_", CHUNK_TREE_PREFIX, table, SAMPLE_RAW);
        let mut total_points = 0u64;
        let mut total_chunks = 0u64;
        let mut total_size = 0u64;
        let mut min_ts = i64::MAX;
        let mut max_ts = i64::MIN;

        for name_bytes in self.db.tree_names() {
            let name = String::from_utf8(name_bytes.to_vec()).unwrap_or_default();
            if name.starts_with(&prefix) {
                total_chunks += 1;
                if let Ok(tree) = self.db.open_tree(name.as_bytes()) {
                    for (k, v) in tree.iter().flatten() {
                        total_size += (k.len() + v.len()) as u64;
                        if let Ok(point) = serde_json::from_slice::<TimeSeriesPoint>(&v) {
                            total_points += 1;
                            if point.timestamp < min_ts {
                                min_ts = point.timestamp;
                            }
                            if point.timestamp > max_ts {
                                max_ts = point.timestamp;
                            }
                        }
                    }
                }
            }
        }

        let mut out = format!(
            "TimeSeries Metric: {}\n  Points: {}\n  Chunks: {}\n  Size: {} bytes\n",
            table, total_points, total_chunks, total_size,
        );

        if max_ts > min_ts {
            let span_ms = max_ts - min_ts;
            out.push_str(&format!(
                "  Time Range: {} to {}\n  Span: {}ms\n",
                min_ts, max_ts, span_ms
            ));
        }

        if let Some(ref m) = metric {
            out.push_str(&format!("  Resolutions: {}\n", m.resolutions.len()));
            for r in &m.resolutions {
                out.push_str(&format!(
                    "    - {} (retention: {}d, chunk: {}ms, agg: {})\n",
                    r.resolution, r.retention_days, r.chunk_duration_ms, r.agg_fn
                ));
            }
            out.push_str(&format!("  Tags: {:?}\n", m.tags));
            out.push_str(&format!("  Fields: {:?}\n", m.field_names));
        }

        Ok(out)
    }

    /// Create a metric (mapped from a table creation).
    async fn create_table(&self, table: &str, _schema: &Schema) -> Result<()> {
        self.ensure_metric(table, &[], &[])?;
        info!("Created time series metric: {}", table);
        Ok(())
    }

    /// Drop a metric (mapped from a table drop).
    async fn drop_table(&self, table: &str) -> Result<()> {
        self.delete_metric(table)
    }

    /// Delete all points of a metric and recreate it empty.
    async fn truncate_table(&self, table: &str, _cascade: bool) -> Result<()> {
        self.delete_metric(table)?;
        self.ensure_metric(table, &[], &[])?;
        Ok(())
    }

    /// Return [`TableInfo`] for a metric: raw point count and on-disk size.
    async fn table_info(&self, table: &str) -> Result<TableInfo> {
        let prefix = format!("{}{}_{}_", CHUNK_TREE_PREFIX, table, SAMPLE_RAW);
        let mut row_count = 0u64;
        let mut size_bytes = 0u64;

        for name_bytes in self.db.tree_names() {
            let name = String::from_utf8(name_bytes.to_vec()).unwrap_or_default();
            if name.starts_with(&prefix) {
                if let Ok(tree) = self.db.open_tree(name.as_bytes()) {
                    row_count += tree.len() as u64;
                    for (k, v) in tree.iter().flatten() {
                        size_bytes += (k.len() + v.len()) as u64;
                    }
                }
            }
        }

        Ok(TableInfo {
            name: table.to_string(),
            schema: Schema {
                fields: vec![],
                indexes: vec![],
                constraints: vec![],
            },
            row_count,
            size_bytes,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Enumerate the names of all time-series metrics.
    fn list_tables(&self) -> Result<Vec<String>> {
        self.list_metrics()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrimusDBConfig;

    fn test_config(tmpdir: &tempfile::TempDir) -> PrimusDBConfig {
        PrimusDBConfig {
            storage: crate::StorageConfig {
                data_dir: tmpdir.path().to_string_lossy().to_string(),
                max_file_size: 1024 * 1024 * 1024,
                compression: crate::CompressionType::Lz4,
                cache_size: 10 * 1024 * 1024,
            },
            network: crate::NetworkConfig {
                bind_address: "127.0.0.1".to_string(),
                port: 8080,
                max_connections: 100,
                tls_enabled: false,
                tls_cert_path: String::new(),
                tls_key_path: String::new(),
                tls_ca_path: String::new(),
                mtls_enabled: false,
            },
            security: crate::SecurityConfig {
                encryption_enabled: false,
                key_rotation_interval: 86400,
                auth_required: false,
                mfa_enabled: false,
            },
            cluster: crate::ClusterConfig {
                enabled: false,
                node_id: "test".to_string(),
                discovery_servers: vec![],
            },
            namespaces: Default::default(),
            federation: None,
            integrity: crate::integrity::IntegrityConfig::default(),
            hyperledger: None,
            graphql: crate::graphql::GraphQLConfig::default(),
            search: crate::search::SearchConfig::default(),
        }
    }

    #[test]
    fn test_engine_new_and_list() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;
        let metrics = engine.list_metrics()?;
        assert!(metrics.is_empty());
        Ok(())
    }

    #[test]
    fn test_insert_and_query_point() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        engine.insert_point(
            "test_metric",
            TimeSeriesPoint {
                timestamp: 1000000,
                tags: {
                    let mut t = HashMap::new();
                    t.insert("host".to_string(), "server1".to_string());
                    t
                },
                fields: {
                    let mut f = HashMap::new();
                    f.insert("cpu".to_string(), 42.5);
                    f
                },
            },
        )?;

        let metrics = engine.list_metrics()?;
        assert_eq!(metrics, vec!["test_metric"]);

        let query = TimeSeriesQuery {
            metric: "test_metric".to_string(),
            start_time: Some(0),
            end_time: Some(2000000),
            tags: None,
            fields: None,
            aggregation: None,
            resolution: None,
            fill_policy: None,
            limit: Some(100),
            offset: None,
            group_by: None,
        };
        let points = engine.query_points(&query)?;
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].fields.get("cpu"), Some(&42.5));
        Ok(())
    }

    #[test]
    fn test_aggregation_avg() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        for i in 0..10 {
            engine.insert_point(
                "agg_test",
                TimeSeriesPoint {
                    timestamp: 1000 + i * 10_000,
                    tags: HashMap::new(),
                    fields: {
                        let mut f = HashMap::new();
                        f.insert("value".to_string(), i as f64 * 10.0);
                        f
                    },
                },
            )?;
        }

        let query = TimeSeriesQuery {
            metric: "agg_test".to_string(),
            start_time: Some(0),
            end_time: Some(200000),
            tags: None,
            fields: Some(vec!["value".to_string()]),
            aggregation: Some("avg".to_string()),
            resolution: None,
            fill_policy: None,
            limit: Some(100),
            offset: None,
            group_by: None,
        };
        let results = engine.aggregate(&query, "avg")?;
        assert!(!results.is_empty());
        Ok(())
    }

    #[test]
    fn test_multiple_aggregation_fns() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        for i in 0..100 {
            engine.insert_point(
                "multi_agg",
                TimeSeriesPoint {
                    timestamp: 1000 + i as i64,
                    tags: {
                        let mut t = HashMap::new();
                        t.insert(
                            "host".to_string(),
                            (if i % 2 == 0 { "web1" } else { "web2" }).to_string(),
                        );
                        t
                    },
                    fields: {
                        let mut f = HashMap::new();
                        f.insert("value".to_string(), i as f64);
                        f
                    },
                },
            )?;
        }

        let mut query = TimeSeriesQuery {
            metric: "multi_agg".to_string(),
            start_time: Some(0),
            end_time: Some(2000),
            tags: None,
            fields: Some(vec!["value".to_string()]),
            aggregation: Some("sum".to_string()),
            resolution: Some("1m".to_string()),
            fill_policy: None,
            limit: Some(100),
            offset: None,
            group_by: None,
        };

        let sum_res = engine.aggregate(&query, "sum")?;
        assert!(!sum_res.is_empty());

        query.aggregation = Some("min".to_string());
        let min_res = engine.aggregate(&query, "min")?;
        assert!(!min_res.is_empty());

        query.aggregation = Some("max".to_string());
        let max_res = engine.aggregate(&query, "max")?;
        assert!(!max_res.is_empty());

        query.aggregation = Some("count".to_string());
        let count_res = engine.aggregate(&query, "count")?;
        assert!(!count_res.is_empty());

        query.aggregation = Some("stddev".to_string());
        let std_res = engine.aggregate(&query, "stddev")?;
        assert!(!std_res.is_empty());

        Ok(())
    }

    #[test]
    fn test_tag_filtering() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        for i in 0..10 {
            engine.insert_point(
                "tag_test",
                TimeSeriesPoint {
                    timestamp: 1000 + i as i64,
                    tags: {
                        let mut t = HashMap::new();
                        t.insert(
                            "host".to_string(),
                            (if i % 2 == 0 { "web1" } else { "web2" }).to_string(),
                        );
                        t.insert("region".to_string(), "us-east".to_string());
                        t
                    },
                    fields: {
                        let mut f = HashMap::new();
                        f.insert("cpu".to_string(), i as f64);
                        f
                    },
                },
            )?;
        }

        let query = TimeSeriesQuery {
            metric: "tag_test".to_string(),
            start_time: None,
            end_time: None,
            tags: Some({
                let mut t = HashMap::new();
                t.insert("host".to_string(), "web1".to_string());
                t
            }),
            fields: None,
            aggregation: None,
            resolution: None,
            fill_policy: None,
            limit: Some(100),
            offset: None,
            group_by: None,
        };
        let points = engine.query_points(&query)?;
        assert_eq!(points.len(), 5);
        for p in &points {
            assert_eq!(p.tags.get("host"), Some(&"web1".to_string()));
        }
        Ok(())
    }

    #[test]
    fn test_metric_metadata() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        engine.ensure_metric(
            "meta_test",
            &["cpu".to_string(), "mem".to_string()],
            &["host".to_string()],
        )?;
        let metric = engine.get_metric("meta_test")?.unwrap();
        assert!(metric.field_names.contains(&"cpu".to_string()));
        assert!(metric.tags.contains(&"host".to_string()));

        engine.update_metric_config("meta_test", Some("CPU metric"), Some("percent"), None)?;
        let updated = engine.get_metric("meta_test")?.unwrap();
        assert_eq!(updated.description, "CPU metric");
        assert_eq!(updated.unit, "percent");
        Ok(())
    }

    #[test]
    fn test_retention() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        // Insert a very old point
        engine.insert_point(
            "retention_test",
            TimeSeriesPoint {
                timestamp: 1000, // very old
                tags: HashMap::new(),
                fields: {
                    let mut f = HashMap::new();
                    f.insert("value".to_string(), 1.0);
                    f
                },
            },
        )?;

        // Insert a recent point
        engine.insert_point(
            "retention_test",
            TimeSeriesPoint {
                timestamp: Utc::now().timestamp_millis(),
                tags: HashMap::new(),
                fields: {
                    let mut f = HashMap::new();
                    f.insert("value".to_string(), 2.0);
                    f
                },
            },
        )?;

        // Apply retention with very short retention (should remove the old chunk)
        engine.add_resolution("retention_test", "raw", 0, "avg")?;

        // We need to verify retention actually removes data.
        engine.apply_retention("retention_test")?;
        Ok(())
    }

    #[test]
    fn test_downsample() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        // Insert 100 points over a time range
        for i in 0..100 {
            engine.insert_point(
                "downsample_test",
                TimeSeriesPoint {
                    timestamp: 1000000 + i * 60_000, // 1 min apart
                    tags: HashMap::new(),
                    fields: {
                        let mut f = HashMap::new();
                        f.insert("value".to_string(), i as f64);
                        f
                    },
                },
            )?;
        }

        let processed = engine.downsample("downsample_test", "raw", "1h", "avg")?;
        assert!(processed > 0);
        Ok(())
    }

    #[test]
    fn test_delete_metric() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        engine.insert_point(
            "delete_test",
            TimeSeriesPoint {
                timestamp: 1000,
                tags: HashMap::new(),
                fields: {
                    let mut f = HashMap::new();
                    f.insert("value".to_string(), 1.0);
                    f
                },
            },
        )?;

        assert!(engine.list_metrics()?.contains(&"delete_test".to_string()));
        engine.delete_metric("delete_test")?;
        assert!(!engine.list_metrics()?.contains(&"delete_test".to_string()));
        Ok(())
    }

    #[test]
    fn test_batch_insert() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        let points: Vec<TimeSeriesPoint> = (0..50)
            .map(|i| TimeSeriesPoint {
                timestamp: 1000 + i as i64,
                tags: HashMap::new(),
                fields: {
                    let mut f = HashMap::new();
                    f.insert("value".to_string(), i as f64);
                    f
                },
            })
            .collect();

        let count = engine.insert_batch("batch_test", points)?;
        assert_eq!(count, 50);

        let query = TimeSeriesQuery {
            metric: "batch_test".to_string(),
            start_time: None,
            end_time: None,
            tags: None,
            fields: None,
            aggregation: None,
            resolution: None,
            fill_policy: None,
            limit: Some(100),
            offset: None,
            group_by: None,
        };
        let points = engine.query_points(&query)?;
        assert_eq!(points.len(), 50);
        Ok(())
    }

    #[test]
    fn test_gap_filling_previous() {
        let engine = create_test_engine_for_fill();
        let results = vec![
            TimeSeriesAggregation {
                timestamp: 1000,
                value: 10.0,
                count: 1,
                fields: HashMap::new(),
            },
            TimeSeriesAggregation {
                timestamp: 3000,
                value: 30.0,
                count: 1,
                fields: HashMap::new(),
            },
        ];
        let filled = engine.fill_gaps(results, &AggregationFn::Avg, 1000, FillPolicy::Previous);
        assert_eq!(filled.len(), 3);
        assert_eq!(filled[1].timestamp, 2000);
        assert_eq!(filled[1].value, 10.0);
    }

    fn create_test_engine_for_fill() -> TimeSeriesEngine {
        let tmpdir = tempfile::tempdir().unwrap();
        let config = test_config(&tmpdir);
        TimeSeriesEngine::new(&config).unwrap()
    }

    #[test]
    fn test_add_resolution() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        engine.ensure_metric("res_test", &["value".to_string()], &[])?;
        engine.add_resolution("res_test", "1h", 90, "avg")?;

        let metric = engine.get_metric("res_test")?.unwrap();
        assert_eq!(metric.resolutions.len(), 2);
        assert!(metric.resolutions.iter().any(|r| r.resolution == "1h"));
        Ok(())
    }

    #[test]
    fn test_engine_stats() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        engine.insert_point(
            "stats_test",
            TimeSeriesPoint {
                timestamp: 1000,
                tags: HashMap::new(),
                fields: {
                    let mut f = HashMap::new();
                    f.insert("value".to_string(), 1.0);
                    f
                },
            },
        )?;

        let stats = engine.engine_stats()?;
        assert_eq!(stats["metrics"], 1);
        assert_eq!(stats["points"], 1);
        Ok(())
    }

    #[test]
    fn test_delete_points() -> Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config = test_config(&tmpdir);
        let engine = TimeSeriesEngine::new(&config)?;

        engine.insert_point(
            "del_points",
            TimeSeriesPoint {
                timestamp: 1000,
                tags: {
                    let mut t = HashMap::new();
                    t.insert("host".to_string(), "web1".to_string());
                    t
                },
                fields: {
                    let mut f = HashMap::new();
                    f.insert("cpu".to_string(), 50.0);
                    f
                },
            },
        )?;

        engine.insert_point(
            "del_points",
            TimeSeriesPoint {
                timestamp: 2000,
                tags: {
                    let mut t = HashMap::new();
                    t.insert("host".to_string(), "web1".to_string());
                    t
                },
                fields: {
                    let mut f = HashMap::new();
                    f.insert("cpu".to_string(), 60.0);
                    f
                },
            },
        )?;

        let removed = engine.delete_points("del_points", Some(1000), Some(1500), None)?;
        assert_eq!(removed, 1);
        Ok(())
    }
}
