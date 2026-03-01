/*
 * PrimusDB Unified Query Language (UQL) Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: Unified Query Language Engine
 */

//! # Unified Query Language (UQL) Engine
//!
//! This module implements PrimusDB's Unified Query Language, a powerful query
//! system that allows querying across multiple storage engines using a single
//! consistent interface. UQL supports SQL-like syntax, MongoDB-style queries,
//! Mango queries, and native PrimusDB extensions.
//!
//! ## Key Features
//!
//! - **Cross-Engine Queries**: Join data from columnar, vector, document,
//!   relational, and key-value engines in a single query
//! - **Multi-Language Support**: SQL, MongoDB, Mango, and native UQL syntax
//! - **Unified Abstraction**: Single API for all storage backends
//! - **Query Optimization**: Intelligent routing to optimal storage engines
//! - **Federated Queries**: Query across multiple nodes and clusters
//!
//! ## Query Examples
//!
//! ### SQL-like Query
//! ```json
//! {
//!   "query": "SELECT * FROM users WHERE age > 25 JOIN orders ON users.id = orders.user_id"
//! }
//! ```
//!
//! ### MongoDB-style Query
//! ```json
//! {
//!   "query_type": "mongodb",
//!   "query": { "users": { "age": { "$gt": 25 } } }
//! }
//! ```
//!
//! ### Cross-Engine Query
//! ```json
//! {
//!   "query": "SELECT u.name, v.embedding_score FROM users u JOIN vectors v ON u.id = v.user_id"
//! }
//! ```
//!
//! ## Architecture
//!
//! The UQL engine consists of several components:
//!
//! 1. **Query Parser**: Parses incoming queries and detects query language
//! 2. **Query Normalizer**: Converts queries to intermediate representation
//! 3. **Query Planner**: Creates optimal execution plan across engines
//! 4. **Query Executor**: Executes plan and combines results
//! 5. **Result Aggregator**: Merges results from multiple sources

use crate::{PrimusDBConfig, Record, Result, StorageType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub mod parser;
pub mod planner;
pub mod executor;

pub use parser::*;
pub use planner::*;
pub use executor::*;

/// Unified Query Language engine
pub struct UqlEngine {
    config: PrimusDBConfig,
    storage_engines: Arc<RwLock<HashMap<StorageType, Arc<dyn crate::storage::StorageEngine + Send + Sync>>>>,
    query_cache: Arc<RwLock<HashMap<String, CachedQuery>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedQuery {
    pub query: String,
    pub plan: QueryPlan,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl UqlEngine {
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        Ok(UqlEngine {
            config: config.clone(),
            storage_engines: Arc::new(RwLock::new(HashMap::new())),
            query_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn with_storage_engines(
        config: &PrimusDBConfig,
        engines: Arc<RwLock<HashMap<StorageType, Arc<dyn crate::storage::StorageEngine + Send + Sync>>>>,
    ) -> Self {
        UqlEngine {
            config: config.clone(),
            storage_engines: engines,
            query_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_storage_engine(&self, storage_type: StorageType, engine: Arc<dyn crate::storage::StorageEngine + Send + Sync>) {
        let mut engines = self.storage_engines.write().unwrap();
        engines.insert(storage_type, engine);
    }

    pub fn execute_query(&self, query: &UqlQuery) -> Result<UqlResult> {
        let parsed = self.parse_query(query)?;
        let plan = self.create_execution_plan(&parsed)?;
        self.execute_plan(&plan)
    }

    fn parse_query(&self, query: &UqlQuery) -> Result<ParsedQuery> {
        let parser = QueryParser::new();
        parser.parse(query)
    }

    fn create_execution_plan(&self, parsed: &ParsedQuery) -> Result<QueryPlan> {
        let planner = QueryPlanner::new(&self.config);
        let engines_map: HashMap<String, crate::storage::StorageEngineType> = self.storage_engines
            .read()
            .unwrap()
            .iter()
            .map(|(k, _)| (format!("{:?}", k).to_lowercase(), crate::storage::StorageEngineType::from_str(&format!("{:?}", k).to_lowercase()).unwrap_or(crate::storage::StorageEngineType::Columnar)))
            .collect();
        planner.create_plan(parsed, &engines_map)
    }

    fn execute_plan(&self, plan: &QueryPlan) -> Result<UqlResult> {
        let executor = QueryExecutor::with_storage_engines(&self.config, self.storage_engines.clone());
        executor.execute(plan)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UqlQuery {
    pub query: String,
    pub query_type: QueryLanguage,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
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
    Auto,
}

impl Default for QueryLanguage {
    fn default() -> Self {
        QueryLanguage::Auto
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UqlResult {
    pub success: bool,
    pub records: Vec<Record>,
    pub total: usize,
    pub execution_time_ms: u64,
    pub engine_used: String,
    pub warnings: Vec<String>,
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
        }
    }
}
