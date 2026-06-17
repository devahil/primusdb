/*
 * PrimusDB Unified Query Language (UQL) Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 2.0.0 - Added: query cache with TTL, LRU eviction, stats
 */

use crate::{PrimusDBConfig, Record, Result, StorageType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub mod executor;
pub mod parser;
pub mod planner;

pub use executor::*;
pub use parser::*;
pub use planner::*;

/// Default cache TTL: 60 seconds
const DEFAULT_CACHE_TTL_SECS: u64 = 60;
/// Default max cache entries
const DEFAULT_CACHE_MAX_SIZE: usize = 1000;

/// Unified Query Language engine with query caching
pub struct UqlEngine {
    config: PrimusDBConfig,
    storage_engines:
        Arc<RwLock<HashMap<StorageType, Arc<dyn crate::storage::StorageEngine + Send + Sync>>>>,
    query_cache: Arc<RwLock<QueryCache>>,
}

/// Query cache configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QueryCacheConfig {
    /// Time-to-live for cached query plans in seconds
    pub ttl_secs: u64,
    /// Maximum number of cached query plans
    pub max_entries: usize,
    /// Whether caching is enabled
    pub enabled: bool,
}

impl Default for QueryCacheConfig {
    fn default() -> Self {
        QueryCacheConfig {
            ttl_secs: DEFAULT_CACHE_TTL_SECS,
            max_entries: DEFAULT_CACHE_MAX_SIZE,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedQuery {
    pub query: String,
    pub plan: QueryPlan,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub ttl_secs: u64,
    pub hit_count: u64,
}

impl CachedQuery {
    pub fn new(query: String, plan: QueryPlan, ttl_secs: u64) -> Self {
        CachedQuery {
            query,
            plan,
            created_at: chrono::Utc::now(),
            ttl_secs,
            hit_count: 0,
        }
    }

    pub fn is_expired(&self) -> bool {
        let elapsed = chrono::Utc::now() - self.created_at;
        elapsed.num_seconds() >= self.ttl_secs as i64
    }
}

/// Internal cache structure with LRU tracking
struct QueryCache {
    entries: HashMap<String, CachedQuery>,
    access_order: Vec<String>,
    config: QueryCacheConfig,
    miss_count: u64,
    hit_count: u64,
}

impl QueryCache {
    fn new(config: QueryCacheConfig) -> Self {
        QueryCache {
            entries: HashMap::new(),
            access_order: Vec::new(),
            config,
            miss_count: 0,
            hit_count: 0,
        }
    }

    fn get(&mut self, key: &str) -> Option<&CachedQuery> {
        if !self.config.enabled {
            return None;
        }
        if let Some(entry) = self.entries.get(key) {
            if entry.is_expired() {
                self.entries.remove(key);
                self.access_order.retain(|k| k != key);
                self.miss_count += 1;
                return None;
            }
            // Update access order for LRU
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.to_string());
            self.hit_count += 1;
            // Update hit count on the entry (need interior mutability, we'll update on next write)
            if let Some(entry) = self.entries.get_mut(key) {
                entry.hit_count += 1;
            }
            self.entries.get(key)
        } else {
            self.miss_count += 1;
            None
        }
    }

    fn insert(&mut self, key: String, value: CachedQuery) {
        if !self.config.enabled {
            return;
        }
        // Evict if at capacity
        if self.entries.len() >= self.config.max_entries && !self.entries.contains_key(&key) {
            self.evict_lru();
        }
        self.access_order.retain(|k| k != &key);
        self.access_order.push(key.clone());
        self.entries.insert(key, value);
    }

    fn evict_lru(&mut self) {
        if let Some(lru_key) = self.access_order.first().cloned() {
            self.access_order.remove(0);
            self.entries.remove(&lru_key);
        }
    }

    fn invalidate(&mut self) {
        self.entries.clear();
        self.access_order.clear();
    }

    fn invalidate_for_table(&mut self, table: &str) {
        let keys_to_remove: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| {
                if entry.plan.engine_routing.contains_key(table) {
                    return true;
                }
                entry
                    .plan
                    .stages
                    .iter()
                    .any(|stage| self.stage_references_table(stage, table))
            })
            .map(|(k, _)| k.clone())
            .collect();

        for key in keys_to_remove {
            self.entries.remove(&key);
            self.access_order.retain(|k| k != &key);
        }
    }

    fn stage_references_table(&self, stage: &ExecutionStage, table: &str) -> bool {
        match &stage.operation {
            StageOperation::Scan { table: t, .. }
            | StageOperation::Insert { table: t, .. }
            | StageOperation::Update { table: t, .. }
            | StageOperation::Delete { table: t, .. }
            | StageOperation::Create { table: t, .. }
            | StageOperation::Drop { table: t, .. }
            | StageOperation::Alter { table: t, .. }
            | StageOperation::Truncate { table: t, .. } => t == table,
            StageOperation::Join {
                left_table,
                right_table,
                ..
            } => left_table == table || right_table == table,
            _ => false,
        }
    }

    fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.entries.len(),
            max_size: self.config.max_entries,
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            enabled: self.config.enabled,
            ttl_secs: self.config.ttl_secs,
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub enabled: bool,
    pub ttl_secs: u64,
}

impl UqlEngine {
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        Ok(UqlEngine {
            config: config.clone(),
            storage_engines: Arc::new(RwLock::new(HashMap::new())),
            query_cache: Arc::new(RwLock::new(QueryCache::new(QueryCacheConfig::default()))),
        })
    }

    pub fn with_storage_engines(
        config: &PrimusDBConfig,
        engines: Arc<
            RwLock<HashMap<StorageType, Arc<dyn crate::storage::StorageEngine + Send + Sync>>>,
        >,
    ) -> Self {
        UqlEngine {
            config: config.clone(),
            storage_engines: engines,
            query_cache: Arc::new(RwLock::new(QueryCache::new(QueryCacheConfig::default()))),
        }
    }

    pub fn with_cache_config(config: &PrimusDBConfig, cache_config: QueryCacheConfig) -> Self {
        UqlEngine {
            config: config.clone(),
            storage_engines: Arc::new(RwLock::new(HashMap::new())),
            query_cache: Arc::new(RwLock::new(QueryCache::new(cache_config))),
        }
    }

    pub fn register_storage_engine(
        &self,
        storage_type: StorageType,
        engine: Arc<dyn crate::storage::StorageEngine + Send + Sync>,
    ) {
        if let Ok(mut engines) = self.storage_engines.write() {
            engines.insert(storage_type, engine);
        }
    }

    pub fn execute_query(&self, query: &UqlQuery) -> Result<UqlResult> {
        let cache_key = self.cache_key(query);

        // Check cache for read operations
        let is_read = matches!(query.query_type, QueryLanguage::Sql | QueryLanguage::Auto)
            && !self.is_write_query(&query.query);

        if is_read {
            if let Ok(mut cache) = self.query_cache.write() {
                if let Some(_cached) = cache.get(&cache_key) {
                    return Ok(UqlResult::success(vec![], 0).with_cache_hit());
                }
            }
        }

        // Parse and plan
        let parsed = self.parse_query(query)?;
        let plan = self.create_execution_plan(&parsed)?;

        // Cache the plan for read queries
        if is_read {
            if let Ok(mut cache) = self.query_cache.write() {
                cache.insert(
                    cache_key.clone(),
                    CachedQuery::new(query.query.clone(), plan.clone(), DEFAULT_CACHE_TTL_SECS),
                );
            }
        }

        // Execute
        let result = self.execute_plan(&plan)?;

        // Invalidate cache for write operations
        if !is_read {
            if let Ok(mut cache) = self.query_cache.write() {
                if let Some(ref target) = parsed.target_table {
                    cache.invalidate_for_table(target);
                } else {
                    cache.invalidate();
                }
            }
        }

        Ok(result)
    }

    fn cache_key(&self, query: &UqlQuery) -> String {
        format!(
            "{:?}:{}:{}",
            query.query_type,
            query.query.trim(),
            query
                .parameters
                .as_ref()
                .map(|p| format!("{:?}", p))
                .unwrap_or_default()
        )
    }

    fn is_write_query(&self, query: &str) -> bool {
        let upper = query.trim().to_uppercase();
        upper.starts_with("INSERT")
            || upper.starts_with("UPDATE")
            || upper.starts_with("DELETE")
            || upper.starts_with("CREATE")
            || upper.starts_with("DROP")
            || upper.starts_with("ALTER")
            || upper.starts_with("TRUNCATE")
    }

    fn parse_query(&self, query: &UqlQuery) -> Result<ParsedQuery> {
        let parser = QueryParser::new();
        parser.parse(query)
    }

    fn create_execution_plan(&self, parsed: &ParsedQuery) -> Result<QueryPlan> {
        let planner = QueryPlanner::new(&self.config);
        let engines_map: HashMap<String, crate::storage::StorageEngineType> = {
            let engines = self.storage_engines.read().map_err(|_| {
                crate::Error::DatabaseError("Storage engines lock poisoned".to_string())
            })?;
            engines
                .keys()
                .map(|k| {
                    let name = match k {
                        StorageType::Columnar => "columnar",
                        StorageType::Vector => "vector",
                        StorageType::Document => "document",
                        StorageType::Relational => "relational",
                        StorageType::KeyValue => "keyvalue",
                    };
                    (
                        name.to_string(),
                        crate::storage::StorageEngineType::from_str(name)
                            .unwrap_or(crate::storage::StorageEngineType::Columnar),
                    )
                })
                .collect()
        };
        planner.create_plan(parsed, &engines_map)
    }

    fn execute_plan(&self, plan: &QueryPlan) -> Result<UqlResult> {
        let executor =
            QueryExecutor::with_storage_engines(&self.config, self.storage_engines.clone());
        executor.execute(plan)
    }

    /// Returns cache statistics for monitoring
    pub fn cache_stats(&self) -> Result<CacheStats> {
        let cache = self
            .query_cache
            .read()
            .map_err(|_| crate::Error::DatabaseError("Cache lock poisoned".to_string()))?;
        Ok(cache.stats())
    }

    /// Manually invalidate the entire query cache
    pub fn invalidate_cache(&self) -> Result<()> {
        let mut cache = self
            .query_cache
            .write()
            .map_err(|_| crate::Error::DatabaseError("Cache lock poisoned".to_string()))?;
        cache.invalidate();
        Ok(())
    }

    /// Invalidate cache entries for a specific table
    pub fn invalidate_cache_for_table(&self, table: &str) -> Result<()> {
        let mut cache = self
            .query_cache
            .write()
            .map_err(|_| crate::Error::DatabaseError("Cache lock poisoned".to_string()))?;
        cache.invalidate_for_table(table);
        Ok(())
    }

    /// Update cache configuration at runtime
    pub fn set_cache_config(&self, config: QueryCacheConfig) -> Result<()> {
        let mut cache = self
            .query_cache
            .write()
            .map_err(|_| crate::Error::DatabaseError("Cache lock poisoned".to_string()))?;
        cache.config = config;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UqlQuery {
    pub query: String,
    pub query_type: QueryLanguage,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum QueryLanguage {
    #[serde(rename = "sql")]
    Sql,
    #[serde(rename = "mongodb")]
    MongoDb,
    #[serde(rename = "mango")]
    Mango,
    #[serde(rename = "uql")]
    Uql,
    #[serde(rename = "auto")]
    #[default]
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UqlResult {
    pub success: bool,
    pub records: Vec<Record>,
    pub total: usize,
    pub execution_time_ms: u64,
    pub engine_used: String,
    pub warnings: Vec<String>,
    pub cached: bool,
    /// Number of rows affected by DML/DDL operations (0 for SELECT)
    #[serde(default)]
    pub affected_rows: u64,
}

impl UqlResult {
    pub fn success(records: Vec<Record>, execution_time_ms: u64) -> Self {
        let total = records.len();
        UqlResult {
            success: true,
            records,
            total,
            execution_time_ms,
            engine_used: "uql".to_string(),
            warnings: vec![],
            cached: false,
            affected_rows: 0,
        }
    }

    pub fn mutation_success(
        records: Vec<Record>,
        affected_rows: u64,
        execution_time_ms: u64,
    ) -> Self {
        let total = records.len();
        UqlResult {
            success: true,
            records,
            total,
            execution_time_ms,
            engine_used: "uql".to_string(),
            warnings: vec![],
            cached: false,
            affected_rows,
        }
    }

    pub fn error(message: String) -> Self {
        UqlResult {
            success: false,
            records: vec![],
            total: 0,
            execution_time_ms: 0,
            engine_used: "uql".to_string(),
            warnings: vec![message],
            cached: false,
            affected_rows: 0,
        }
    }

    pub fn with_cache_hit(mut self) -> Self {
        self.cached = true;
        self
    }
}

impl std::fmt::Display for UqlResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UqlResult {{ success: {}, total: {}, time_ms: {}, engine: {}, cached: {}, warnings: {:?} }}",
            self.success, self.total, self.execution_time_ms, self.engine_used, self.cached, self.warnings
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> PrimusDBConfig {
        PrimusDBConfig {
            storage: crate::StorageConfig {
                data_dir: "./data".to_string(),
                max_file_size: 1024 * 1024,
                compression: crate::CompressionType::Lz4,
                cache_size: 1024,
            },
            network: crate::NetworkConfig {
                bind_address: "127.0.0.1".to_string(),
                port: 8080,
                max_connections: 100,
            },
            security: crate::SecurityConfig {
                encryption_enabled: false,
                key_rotation_interval: 86400,
                auth_required: false,
            },
            cluster: crate::ClusterConfig {
                enabled: false,
                node_id: "test".to_string(),
                discovery_servers: vec![],
            },
            namespaces: Default::default(),
            federation: None,
        }
    }

    #[test]
    fn test_cache_key_generation() {
        let engine = UqlEngine::new(&test_config()).unwrap();
        let query = UqlQuery {
            query: "SELECT * FROM users".to_string(),
            query_type: QueryLanguage::Sql,
            parameters: None,
        };
        let key = engine.cache_key(&query);
        assert!(key.contains("Sql"));
        assert!(key.contains("SELECT * FROM users"));
    }

    #[test]
    fn test_is_write_query() {
        let engine = UqlEngine::new(&test_config()).unwrap();
        assert!(engine.is_write_query("INSERT INTO users VALUES (1)"));
        assert!(engine.is_write_query("UPDATE users SET name='x'"));
        assert!(engine.is_write_query("DELETE FROM users"));
        assert!(engine.is_write_query("CREATE TABLE t (id INT)"));
        assert!(engine.is_write_query("DROP TABLE t"));
        assert!(engine.is_write_query("ALTER TABLE t ADD COLUMN x INT"));
        assert!(!engine.is_write_query("SELECT * FROM users"));
    }

    #[test]
    fn test_cache_stats_initial() {
        let engine = UqlEngine::new(&test_config()).unwrap();
        let stats = engine.cache_stats().unwrap();
        assert_eq!(stats.size, 0);
        assert_eq!(stats.hit_count, 0);
        assert_eq!(stats.miss_count, 0);
        assert!(stats.enabled);
    }

    #[test]
    fn test_cache_invalidate() {
        let engine = UqlEngine::new(&test_config()).unwrap();
        // Insert something into cache
        {
            let mut cache = engine.query_cache.write().unwrap();
            let plan = QueryPlan {
                operation: QueryOperation::Select,
                stages: vec![],
                engine_routing: HashMap::new(),
                cross_engine_joins: vec![],
            };
            cache.insert(
                "key1".to_string(),
                CachedQuery::new("SELECT 1".to_string(), plan, 60),
            );
        }
        let stats = engine.cache_stats().unwrap();
        assert_eq!(stats.size, 1);

        engine.invalidate_cache().unwrap();
        let stats = engine.cache_stats().unwrap();
        assert_eq!(stats.size, 0);
    }

    #[test]
    fn test_cache_config_runtime() {
        let engine = UqlEngine::new(&test_config()).unwrap();
        let config = QueryCacheConfig {
            enabled: false,
            ttl_secs: 0,
            max_entries: 0,
        };
        engine.set_cache_config(config).unwrap();
        let stats = engine.cache_stats().unwrap();
        assert!(!stats.enabled);
    }

    #[test]
    fn test_cache_entry_expiry() {
        let entry = CachedQuery::new(
            "SELECT 1".to_string(),
            QueryPlan {
                operation: QueryOperation::Select,
                stages: vec![],
                engine_routing: HashMap::new(),
                cross_engine_joins: vec![],
            },
            0, // TTL = 0, expired immediately
        );
        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_entry_not_expired() {
        let entry = CachedQuery::new(
            "SELECT 1".to_string(),
            QueryPlan {
                operation: QueryOperation::Select,
                stages: vec![],
                engine_routing: HashMap::new(),
                cross_engine_joins: vec![],
            },
            3600, // TTL = 1 hour
        );
        assert!(!entry.is_expired());
    }
}
