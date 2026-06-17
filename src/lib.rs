/*
 * PrimusDB - Hybrid Database Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.3.1-alpha
 */

/*!
# PrimusDB - Hybrid Database Engine

PrimusDB is a next-generation hybrid database engine that combines the power of traditional
relational databases with modern document stores, columnar analytics, and vector similarity search.
Enhanced with integrated AI/ML capabilities and enterprise-grade security.

## Architecture Overview

```text
PrimusDB Engine Architecture
═══════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────┐
│                    API Layer                            │
│  ┌─────────────────────────────────────────────────┐    │
│  │  REST API (15+ endpoints)                     │    │
│  │  • CRUD operations                            │    │
│  │  • AI/ML predictions                          │    │
│  │  • Analytics & clustering                     │    │
│  │  • Vector search                              │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  Multi-Language Drivers: Rust • Python • Ruby • Java    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                 Processing Layer                        │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Query Processor                               │    │
│  │  • SQL parser                                   │    │
│  │  • Query optimization                           │    │
│  │  • Result aggregation                           │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │  AI/ML Engine                                  │    │
│  │  • Prediction models                            │    │
│  │  • Clustering algorithms                        │    │
│  │  • Anomaly detection                            │    │
│  │  • Pattern analysis                             │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                Storage Layer                            │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┐ │
│  │ Columnar    │ Vector      │ Document    │ Relational  │ │
│  │ Engine      │ Engine      │ Engine      │ Engine      │ │
│  │ • Analytics │ • Similarity│ • JSON      │ • SQL       │ │
│  │ • LZ4       │ • FAISS     │ • Dynamic   │ • ACID      │ │
│  │ • Bitmap    │ • SIMD      │ • Indexing  │ • Foreign   │ │
│  │ • Indexes   │ • Search    │ • Schema    │ • Keys      │ │
│  └─────────────┴─────────────┴─────────────┴─────────────┘ │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│              Infrastructure Layer                       │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Consensus Engine                               │    │
│  │  • Hyperledger-style validation                 │    │
│  │  • Block validation                             │    │
│  │  • Fork resolution                              │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Cluster Manager                               │    │
│  │  • Node discovery                              │    │
│  │  • Load balancing                              │    │
│  │  • Failover                                    │    │
│  │  • Shard rebalancing                           │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Security Manager                              │    │
│  │  • AES-256-GCM encryption                      │    │
│  │  • RBAC system                                 │    │
│  │  • Key rotation                                │    │
│  │  • Audit logging                               │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## Features

### Hybrid Storage Engines
- **Columnar Engine**: Optimized for analytical workloads with LZ4 compression and bitmap indexes
- **Vector Engine**: High-performance similarity search with FAISS-style indexing and SIMD operations
- **Document Engine**: Flexible JSON storage with dynamic indexing and schema validation
- **Relational Engine**: Traditional SQL with ACID transactions, foreign keys, and constraints

### AI/ML Integration
- **Predictive Analytics**: Linear regression, time series forecasting, and custom models
- **Clustering**: K-means, density-based algorithms, and hierarchical clustering
- **Anomaly Detection**: Statistical outlier detection and pattern-based analysis
- **Pattern Recognition**: Trend identification and correlation analysis

### Enterprise Features
- **Security**: AES-256-GCM encryption, RBAC, key rotation, and audit logging
- **Clustering**: Auto-discovery, load balancing, automatic failover, and shard rebalancing
- **Consensus**: Hyperledger-style validation with configurable validator networks
- **Performance**: SIMD acceleration, async processing, and connection pooling

### Multi-Language Support
- **Rust**: Native high-performance bindings with zero-cost abstractions
- **Python**: PyO3 extension with async support and type hints
- **Ruby**: Native gem with Rails integration and ActiveRecord-style API
- **Java**: JDBC driver with enterprise connection pooling

## Quick Start

```ignore
use primusdb::{PrimusDBConfig, PrimusDB};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure PrimusDB
    let config = PrimusDBConfig {
        storage: StorageConfig {
            data_dir: "./data".to_string(),
            max_file_size: 1024 * 1024 * 1024, // 1GB
            compression: CompressionType::Lz4,
            cache_size: 100 * 1024 * 1024, // 100MB
        },
        network: NetworkConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
        },
        security: SecurityConfig {
            encryption_enabled: true,
            key_rotation_interval: 86400, // 24 hours
            auth_required: false,
        },
        cluster: ClusterConfig {
            enabled: false,
            node_id: "local_node".to_string(),
            discovery_servers: vec![],
        },
        federation: None,
    };

    // Create and start PrimusDB instance
    let primusdb = PrimusDB::new(config).await?;
    println!("PrimusDB started successfully!");

    Ok(())
}
```

## Development Started: January 10, 2024

PrimusDB development began on January 10, 2024, with the goal of creating
a unified database engine that combines the best features of modern
database technologies while maintaining enterprise-grade reliability
and performance.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use crate::governor::engine::GovernorEngine;
use crate::governor::GovernorConfig;
use crate::query::UqlEngine;

/// SQL parser module for driver SQL string support
/// **DEPRECATED** — Use `crate::query::UqlEngine` / `PrimusDB::uql_execute_query()` instead.
pub mod parser;

/// Core modules for PrimusDB functionality
pub mod ai;
pub mod api;
pub mod auth;
pub mod cache;
pub mod cdc;
pub mod cli;
pub mod cluster;
pub mod consensus;
pub mod crypto;
pub mod drivers;
pub mod error;
pub mod governor;
pub mod namespace;
// pub mod protocol; // Temporarily disabled for compilation
pub mod migration;
pub mod query;
pub mod storage;
pub mod transaction;

/// Re-export error types for convenience
pub use error::*;
/// Re-export namespace types for convenience
pub use namespace::*;

/// Re-export cache types for convenience
pub use cache::*;

/// Main configuration structure for PrimusDB
///
/// This structure contains all configuration options for a PrimusDB instance,
/// including storage, network, security, and clustering settings.
///
/// # Example
/// ```ignore
/// use primusdb::PrimusDBConfig;
///
/// let config = PrimusDBConfig {
///     storage: StorageConfig {
///         data_dir: "./data".to_string(),
///         max_file_size: 1024 * 1024 * 1024,
///         compression: CompressionType::Lz4,
///         cache_size: 100 * 1024 * 1024,
///     },
///     network: NetworkConfig {
///         bind_address: "127.0.0.1".to_string(),
///         port: 8080,
///         max_connections: 1000,
///     },
///     security: SecurityConfig {
///         encryption_enabled: true,
///         key_rotation_interval: 86400,
///         auth_required: false,
///     },
///     cluster: ClusterConfig {
///         enabled: false,
///         node_id: "local_node".to_string(),
///         discovery_servers: vec![],
///     },
///     federation: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimusDBConfig {
    /// Storage-related configuration options
    pub storage: StorageConfig,
    /// Network configuration for server binding and connections
    pub network: NetworkConfig,
    /// Security settings including encryption and authentication
    pub security: SecurityConfig,
    /// Clustering configuration for distributed deployments
    pub cluster: ClusterConfig,
    /// Namespace configuration for multi-model isolation
    pub namespaces: namespace::NamespaceConfig,
    /// (Optional) Federation configuration for multi-cluster SuperScalar mode
    /// When set, enables cluster-of-clusters federation
    pub federation: Option<cluster::FederationConfig>,
}

/// Configuration for storage-related settings
///
/// Controls how data is stored, compressed, and cached within PrimusDB.
/// These settings affect all storage engines (columnar, vector, document, relational).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Directory path where database files will be stored
    /// Default: "./data"
    pub data_dir: String,
    /// Maximum size for individual data files in bytes
    /// Default: 1GB (1,073,741,824 bytes)
    pub max_file_size: u64,
    /// Compression algorithm to use for data storage
    /// Options: None, Lz4, Zstd
    /// Default: Lz4
    pub compression: CompressionType,
    /// Size of the in-memory cache in bytes
    /// Used to cache frequently accessed data blocks
    /// Default: 100MB (104,857,600 bytes)
    pub cache_size: usize,
}

/// Network configuration for server binding and client connections
///
/// Defines how PrimusDB binds to network interfaces and handles
/// incoming client connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// IP address or hostname to bind the server to
    /// Use "0.0.0.0" to bind to all available interfaces
    /// Default: "127.0.0.1"
    pub bind_address: String,
    /// Port number for the server to listen on
    /// Default: 8080
    pub port: u16,
    /// Maximum number of concurrent client connections
    /// This affects both REST API and driver connections
    /// Default: 1000
    pub max_connections: usize,
}

/// Security configuration for encryption, authentication, and access control
///
/// Controls all security-related aspects of PrimusDB including
/// data encryption, user authentication, and access control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Whether to encrypt data at rest using AES-256-GCM
    /// When enabled, all data files are encrypted
    /// Default: true
    pub encryption_enabled: bool,
    /// How often to rotate encryption keys in seconds
    /// Keys are rotated automatically to maintain security
    /// Default: 86400 (24 hours)
    pub key_rotation_interval: u64,
    /// Whether client authentication is required
    /// When enabled, all connections must be authenticated
    /// Default: false (for development)
    pub auth_required: bool,
}

/// Clustering configuration for distributed PrimusDB deployments
///
/// Controls how PrimusDB operates in a clustered environment,
/// including node discovery, communication, and coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Whether clustering is enabled for this instance
    /// When disabled, PrimusDB runs in single-node mode
    /// Default: false
    pub enabled: bool,
    /// Unique identifier for this node in the cluster
    /// Must be unique across all nodes in the cluster
    /// Default: "local_node"
    pub node_id: String,
    /// List of discovery server addresses for node discovery
    /// Used to find other nodes when joining the cluster
    /// Format: ["host:port", "host:port"]
    /// Default: []
    pub discovery_servers: Vec<String>,
}

/// Compression algorithms supported by PrimusDB
///
/// Defines the available compression methods for data storage.
/// Each compression type has different trade-offs between speed and compression ratio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression - fastest but uses most storage space
    None,
    /// LZ4 compression - good balance of speed and compression ratio
    /// Recommended for most use cases
    Lz4,
    /// Zstandard compression with configurable level
    /// Higher levels provide better compression but slower performance
    /// The i32 parameter represents the compression level (1-22)
    Zstd(i32),
}

/// Main PrimusDB database engine instance
///
/// This is the core structure that manages all aspects of a PrimusDB instance.
/// It coordinates between storage engines, security, consensus, transactions,
/// and AI/ML functionality.
///
/// # Example
/// ```ignore
/// use primusdb::{PrimusDB, PrimusDBConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = PrimusDBConfig::default();
///     let db = PrimusDB::new(config).await?;
///
///     // Database is now ready for operations
///     println!("PrimusDB started successfully!");
///
///     Ok(())
/// }
/// ```
#[allow(dead_code)]
pub struct PrimusDB {
    /// Configuration used to initialize this instance
    config: PrimusDBConfig,
    /// Map of storage engines by type (columnar, vector, document, relational)
    storage_engines: HashMap<StorageType, Arc<dyn storage::StorageEngine>>,
    /// Cryptographic operations manager for encryption/decryption
    crypto_manager: Arc<crypto::CryptoManager>,
    /// Consensus engine for distributed operations
    consensus_engine: Arc<dyn consensus::ConsensusEngine>,
    /// Transaction manager for ACID operations
    transaction_manager: Arc<transaction::TransactionManager>,
    /// AI/ML engine for analytics and predictions
    ai_engine: Arc<ai::AIEngine>,
    /// UQL engine for unified queries across all storage engines
    uql_engine: Arc<UqlEngine>,
    /// Cluster manager for distributed node coordination
    cluster_manager: Arc<RwLock<cluster::ClusterManager>>,
    /// Sync coordinator for distributed data synchronization
    sync_coordinator: Arc<cluster::sync::SyncCoordinator>,
    /// Cluster authentication manager (Hyperledger-style genesis keys)
    cluster_auth: Arc<tokio::sync::RwLock<auth::ClusterAuthManager>>,
    /// Namespace controller for multi-model isolation
    namespace_controller: Arc<namespace::NamespaceController>,
    /// Background block producer handle
    producer_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Federation manager for cross-cluster orchestration (optional)
    federation_manager: Option<Arc<cluster::FederationManager>>,
    /// Data domain manager for cross-cluster replication domains (optional)
    domain_manager: Option<Arc<cluster::DataDomainManager>>,
    /// Pending engine lifecycle operations scheduled via the API
    /// Applied on next server restart.
    pending_engine_changes: Mutex<Vec<EngineLifecycleOp>>,
    /// Resource governor engine for policy enforcement
    governor_engine: Arc<GovernorEngine>,
    /// Active transactions started via the API (keyed by transaction ID)
    active_transactions: Mutex<HashMap<String, transaction::Transaction>>,
}

/// Runtime engine lifecycle operation recorded via the API.
/// Changes take effect on the next server restart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineLifecycleOp {
    Add {
        engine_type: String,
    },
    Remove {
        engine_type: String,
        drop_data: bool,
    },
    Upgrade {
        engine_type: String,
    },
}

impl std::fmt::Display for EngineLifecycleOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineLifecycleOp::Add { engine_type } => write!(f, "add engine '{}'", engine_type),
            EngineLifecycleOp::Remove { engine_type, .. } => {
                write!(f, "remove engine '{}'", engine_type)
            }
            EngineLifecycleOp::Upgrade { engine_type } => {
                write!(f, "upgrade engine '{}'", engine_type)
            }
        }
    }
}

/// Types of storage engines available in PrimusDB
///
/// Each storage type is optimized for different use cases and query patterns.
/// Choose the appropriate type based on your data access patterns and requirements.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StorageType {
    Columnar,
    Vector,
    Document,
    Relational,
    KeyValue,
}

impl std::fmt::Display for StorageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageType::Columnar => write!(f, "Columnar"),
            StorageType::Vector => write!(f, "Vector"),
            StorageType::Document => write!(f, "Document"),
            StorageType::Relational => write!(f, "Relational"),
            StorageType::KeyValue => write!(f, "KeyValue"),
        }
    }
}

impl std::str::FromStr for StorageType {
    type Err = crate::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Columnar" => Ok(StorageType::Columnar),
            "Vector" => Ok(StorageType::Vector),
            "Document" => Ok(StorageType::Document),
            "Relational" => Ok(StorageType::Relational),
            "KeyValue" => Ok(StorageType::KeyValue),
            _ => Err(crate::Error::ValidationError(format!(
                "Unknown storage type: {}",
                s
            ))),
        }
    }
}

/// Result types returned by database operations
///
/// Represents the outcome of various database operations including
/// queries, inserts, updates, and deletes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResult {
    /// Result of a SELECT query containing multiple records
    Select(Vec<Record>),
    /// Result of an INSERT operation showing number of records inserted
    Insert(u64),
    /// Result of an UPDATE operation showing number of records modified
    Update(u64),
    /// Result of a DELETE operation showing number of records removed
    Delete(u64),
    /// Result of a TRUNCATE operation
    Truncate(u64),
    /// Explanation of query execution plan (for debugging/performance analysis)
    Explain(String),
}

/// Represents a single database record/document
///
/// A record contains the actual data along with metadata and a unique identifier.
/// The data field uses serde_json::Value for flexibility across different storage types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    /// Unique identifier for this record
    /// Generated automatically or provided by the application
    pub id: String,
    /// The actual data content of the record
    /// Can be any valid JSON value (object, array, primitive)
    pub data: serde_json::Value,
    /// Additional metadata associated with the record
    /// Used for indexing, timestamps, version information, etc.
    pub metadata: HashMap<String, String>,
}

/// Database query structure
///
/// Represents a complete database operation including the target storage type,
/// operation type, target table/collection, and associated parameters.
///
/// # Example
/// ```ignore
/// use primusdb::{Query, StorageType, QueryOperation};
///
/// let select_query = Query {
///     storage_type: StorageType::Document,
///     operation: QueryOperation::Read,
///     table: "users".to_string(),
///     conditions: Some(serde_json::json!({"age": {"$gt": 25}})),
///     data: None,
///     limit: Some(100),
///     offset: Some(0),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    /// Which storage engine to use for this query
    pub storage_type: StorageType,
    /// Type of operation (Read, Create, Update, Delete)
    pub operation: QueryOperation,
    /// Target table/collection name
    pub table: String,
    /// Conditions for filtering records (WHERE clause equivalent)
    /// Uses JSON for flexible query expressions
    pub conditions: Option<serde_json::Value>,
    /// Data payload for insert/update operations
    pub data: Option<serde_json::Value>,
    /// Maximum number of records to return (LIMIT clause)
    /// None means no limit (return all matching records)
    pub limit: Option<u64>,
    /// Number of records to skip (OFFSET clause)
    /// Used for pagination along with limit
    pub offset: Option<u64>,
    /// Namespace path for resource isolation
    /// When set, the table name is translated to a namespace-specific physical name
    pub namespace: Option<String>,
}

impl Default for Query {
    fn default() -> Self {
        Self {
            storage_type: StorageType::Document,
            operation: QueryOperation::Read,
            table: String::new(),
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Types of database operations supported by PrimusDB
///
/// Defines the fundamental CRUD operations plus additional specialized operations.
/// Each operation type has different semantics and return values.
pub enum QueryOperation {
    Read,
    Create,
    Update,
    Delete,
    Truncate,
    Analyze,
    Predict,

    // ER Model Operations (v1.2.2+)
    AlterTableAddColumn,
    AlterTableDropColumn,
    AlterTableModifyColumn,
    AlterTableAddConstraint,
    AlterTableDropConstraint,
    RenameTable,
    CreateSequence,
    DropSequence,
    NextVal,
    CurrVal,
    SetVal,
    CreateView,
    DropView,
    RefreshView,
    CreateTrigger,
    DropTrigger,
    InformationSchemaTables,
    InformationSchemaColumns,
    InformationSchemaConstraints,
}

impl PrimusDB {
    pub fn new(config: PrimusDBConfig) -> Result<Self> {
        let config_clone = config.clone();

        let mut storage_engines: HashMap<StorageType, Arc<dyn storage::StorageEngine>> =
            HashMap::new();

        // Initialize storage engines
        storage_engines.insert(
            StorageType::Columnar,
            Arc::new(storage::columnar::ColumnarEngine::new(&config_clone)?),
        );
        storage_engines.insert(
            StorageType::Vector,
            Arc::new(storage::vector::VectorEngine::new(&config_clone)?),
        );
        storage_engines.insert(
            StorageType::Document,
            Arc::new(storage::document::DocumentEngine::new(&config_clone)?),
        );
        storage_engines.insert(
            StorageType::Relational,
            Arc::new(storage::relational::RelationalEngine::new(&config_clone)?),
        );
        storage_engines.insert(
            StorageType::KeyValue,
            Arc::new(storage::keyvalue::KeyValueEngine::new(&config_clone)?),
        );

        let crypto_manager = Arc::new(crypto::CryptoManager::new(&config_clone.security)?);
        let consensus_engine = Arc::new(consensus::HyperledgerStyleConsensus::new(
            &config_clone,
            storage_engines.clone(),
        )?);
        let transaction_manager = Arc::new(transaction::TransactionManager::new(
            &config_clone,
            consensus_engine.clone(),
            storage_engines.clone(),
        )?);
        let ai_engine = Arc::new(ai::AIEngine::new(&config_clone)?);

        let mut uql_engines: HashMap<StorageType, Arc<dyn storage::StorageEngine + Send + Sync>> =
            HashMap::new();
        for (&k, v) in &storage_engines {
            uql_engines.insert(k, v.clone());
        }
        let uql_engine = Arc::new(UqlEngine::with_storage_engines(
            &config,
            Arc::new(RwLock::new(uql_engines)),
        ));

        let bind_addr: std::net::SocketAddr = format!(
            "{}:{}",
            config_clone.network.bind_address, config_clone.network.port
        )
        .parse()
        .unwrap_or_else(|_| "127.0.0.1:8080".parse().unwrap());
        let cluster_manager = Arc::new(RwLock::new(cluster::ClusterManager::new(
            &config_clone.cluster,
            bind_addr,
        )?));
        let sync_config = cluster::sync::SyncConfig {
            replication_factor: 3,
            sync_interval_ms: 100,
            conflict_resolution: cluster::sync::ConflictResolution::VectorClock,
            enable_referential_integrity: true,
            read_quorum: 2,
            write_quorum: 2,
            heartbeat_interval_ms: 1000,
            max_clock_drift_ms: 5000,
            merkle_sync: true,
        };
        let sync_clients: Arc<std::sync::RwLock<HashMap<String, Arc<cluster::rpc::RpcClient>>>> =
            Arc::new(std::sync::RwLock::new(HashMap::new()));
        let sync_db = {
            let data_dir = format!("{}/sync", config_clone.storage.data_dir);
            std::fs::create_dir_all(&data_dir).ok();
            sled::open(&data_dir).ok()
        };
        let sync_coordinator = Arc::new(cluster::sync::SyncCoordinator::new(
            sync_config,
            config_clone.cluster.node_id.clone(),
            sync_clients,
            sync_db,
        )?);

        let cluster_auth = Arc::new(tokio::sync::RwLock::new(auth::ClusterAuthManager::new(
            auth::ClusterAuthConfig::default(),
        )?));

        let namespace_controller = Arc::new(namespace::NamespaceController::new(&config)?);
        if config.namespaces.enabled {
            namespace_controller.init()?;
        }

        // Initialize federation components if configured
        let (federation_manager, domain_manager) = if let Some(ref fed_config) = config.federation {
            let fed = Arc::new(cluster::FederationManager::new(fed_config.clone()));
            let dm = Arc::new(
                cluster::DataDomainManager::new(config.cluster.node_id.clone())
                    .with_federation(fed.clone()),
            );
            (Some(fed), Some(dm))
        } else {
            (None, None)
        };

        Ok(PrimusDB {
            config,
            storage_engines,
            crypto_manager,
            consensus_engine,
            transaction_manager,
            ai_engine,
            uql_engine,
            cluster_manager,
            sync_coordinator,
            cluster_auth,
            namespace_controller,
            producer_handle: Mutex::new(None),
            federation_manager,
            domain_manager,
            pending_engine_changes: Mutex::new(Vec::new()),
            governor_engine: Arc::new(GovernorEngine::new(GovernorConfig::default())),
            active_transactions: Mutex::new(HashMap::new()),
        })
    }

    pub fn get_federation_manager(&self) -> Option<Arc<cluster::FederationManager>> {
        self.federation_manager.clone()
    }

    pub fn set_federation_manager(&mut self, mgr: Arc<cluster::FederationManager>) {
        self.federation_manager = Some(mgr);
    }

    pub fn get_domain_manager(&self) -> Option<Arc<cluster::DataDomainManager>> {
        self.domain_manager.clone()
    }

    pub fn set_domain_manager(&mut self, mgr: Arc<cluster::DataDomainManager>) {
        self.domain_manager = Some(mgr);
    }

    /// Schedule adding a new storage engine via the lifecycle API.
    ///
    /// Records the operation — the change takes effect on next server restart.
    /// Returns the list of all pending lifecycle operations.
    pub fn schedule_engine_add(&self, engine_type: &str) -> Result<Vec<EngineLifecycleOp>> {
        let mut pending = self.pending_engine_changes.lock().unwrap();
        pending.push(EngineLifecycleOp::Add {
            engine_type: engine_type.to_string(),
        });
        Ok(pending.clone())
    }

    /// Schedule removing a storage engine via the lifecycle API.
    ///
    /// If `drop_data` is true the engine data directory will also be removed
    /// on next restart. Returns all pending operations.
    pub fn schedule_engine_remove(
        &self,
        engine_type: &str,
        drop_data: bool,
    ) -> Result<Vec<EngineLifecycleOp>> {
        let mut pending = self.pending_engine_changes.lock().unwrap();
        pending.push(EngineLifecycleOp::Remove {
            engine_type: engine_type.to_string(),
            drop_data,
        });
        Ok(pending.clone())
    }

    /// Schedule upgrading a storage engine via the lifecycle API.
    ///
    /// Records the operation — the upgrade takes effect on next restart
    /// (the server re-initialises the engine from the updated binary).
    pub fn schedule_engine_upgrade(&self, engine_type: &str) -> Result<Vec<EngineLifecycleOp>> {
        let mut pending = self.pending_engine_changes.lock().unwrap();
        pending.push(EngineLifecycleOp::Upgrade {
            engine_type: engine_type.to_string(),
        });
        Ok(pending.clone())
    }

    /// Return all currently scheduled (un-applied) lifecycle operations.
    pub fn pending_engine_changes(&self) -> Vec<EngineLifecycleOp> {
        self.pending_engine_changes.lock().unwrap().clone()
    }

    /// Access the resource governor engine
    pub fn governor_engine(&self) -> &GovernorEngine {
        &self.governor_engine
    }

    /// Start federation background tasks (announce + heartbeat loops).
    /// Should be called after the cluster has been started.
    pub async fn start_federation(&self) {
        if let Some(fed) = &self.federation_manager {
            let address = self.config.network.bind_address.clone();
            let port = self.config.network.port;
            let node_id = self.config.cluster.node_id.clone();
            fed.clone()
                .start_background_tasks(address, port, node_id, 1)
                .await;
        }
    }

    pub fn config(&self) -> &PrimusDBConfig {
        &self.config
    }

    pub fn storage_engine(
        &self,
        engine_type: StorageType,
    ) -> Option<Arc<dyn storage::StorageEngine>> {
        self.storage_engines.get(&engine_type).cloned()
    }

    pub fn uql_engine(&self) -> Arc<UqlEngine> {
        self.uql_engine.clone()
    }

    pub fn uql_execute_query(
        &self,
        query: &crate::query::UqlQuery,
    ) -> crate::Result<crate::query::UqlResult> {
        self.uql_engine.execute_query(query)
    }

    pub async fn get_chain_state(&self) -> Result<consensus::ChainState> {
        self.consensus_engine.get_chain_state().await
    }

    pub async fn build_and_commit_block(&self) -> Result<Option<consensus::Block>> {
        self.consensus_engine.build_and_commit_block().await
    }

    pub async fn execute_query(&self, query: Query) -> Result<QueryResult> {
        let mut transaction = self.transaction_manager.begin_transaction().await?;

        let result = match query.operation {
            QueryOperation::Create => self.handle_create(&query, &mut transaction).await?,
            QueryOperation::Read => self.handle_read(&query, &transaction).await?,
            QueryOperation::Update => self.handle_update(&query, &mut transaction).await?,
            QueryOperation::Delete => self.handle_delete(&query, &mut transaction).await?,
            QueryOperation::Truncate => self.handle_truncate(&query, &transaction).await?,
            QueryOperation::Analyze => self.handle_analyze(&query, &transaction).await?,
            QueryOperation::Predict => self.handle_predict(&query, &transaction).await?,

            // ER Model Operations (v1.2.2+)
            QueryOperation::AlterTableAddColumn => self.handle_alter_add_column(&query).await?,
            QueryOperation::AlterTableDropColumn => self.handle_alter_drop_column(&query).await?,
            QueryOperation::AlterTableModifyColumn => {
                self.handle_alter_modify_column(&query).await?
            }
            QueryOperation::AlterTableAddConstraint => {
                self.handle_alter_add_constraint(&query).await?
            }
            QueryOperation::AlterTableDropConstraint => {
                self.handle_alter_drop_constraint(&query).await?
            }
            QueryOperation::RenameTable => self.handle_rename_table(&query).await?,
            QueryOperation::CreateSequence => self.handle_create_sequence(&query).await?,
            QueryOperation::DropSequence => self.handle_drop_sequence(&query).await?,
            QueryOperation::NextVal => self.handle_nextval(&query).await?,
            QueryOperation::CurrVal => self.handle_currval(&query).await?,
            QueryOperation::SetVal => self.handle_setval(&query).await?,
            QueryOperation::CreateView => self.handle_create_view(&query).await?,
            QueryOperation::DropView => self.handle_drop_view(&query).await?,
            QueryOperation::RefreshView => self.handle_refresh_view(&query).await?,
            QueryOperation::CreateTrigger => self.handle_create_trigger(&query).await?,
            QueryOperation::DropTrigger => self.handle_drop_trigger(&query).await?,
            QueryOperation::InformationSchemaTables => {
                self.handle_info_schema_tables(&query).await?
            }
            QueryOperation::InformationSchemaColumns => {
                self.handle_info_schema_columns(&query).await?
            }
            QueryOperation::InformationSchemaConstraints => {
                self.handle_info_schema_constraints(&query).await?
            }
        };

        self.transaction_manager
            .commit_transaction(transaction)
            .await?;
        Ok(result)
    }

    async fn handle_create(
        &self,
        query: &Query,
        transaction: &mut transaction::Transaction,
    ) -> Result<QueryResult> {
        let engine = self.get_engine_for_query(query.storage_type, query.namespace.as_deref())?;

        let data = query.data.clone().unwrap_or(serde_json::Value::Null);
        let op_id = format!("op_{}", transaction.operations.len());

        transaction
            .operations
            .push(transaction::TransactionOperation {
                id: op_id,
                operation_type: transaction::OperationType::Insert,
                table: query.table.clone(),
                data: data.clone(),
                conditions: None,
                before_image: None,
                after_image: Some(data.clone()),
                executed: true,
                rollback_data: Some(data.clone()),
                storage_type: query.storage_type.to_string(),
            });

        let count = engine
            .insert(query.table.as_str(), &data, transaction)
            .await?;
        Ok(QueryResult::Insert(count))
    }

    async fn handle_read(
        &self,
        query: &Query,
        transaction: &transaction::Transaction,
    ) -> Result<QueryResult> {
        let engine = self.get_engine_for_query(query.storage_type, query.namespace.as_deref())?;

        let records = engine
            .select(
                query.table.as_str(),
                query.conditions.as_ref(),
                query.limit.unwrap_or(100),
                query.offset.unwrap_or(0),
                transaction,
            )
            .await?;
        Ok(QueryResult::Select(records))
    }

    async fn handle_update(
        &self,
        query: &Query,
        transaction: &mut transaction::Transaction,
    ) -> Result<QueryResult> {
        let engine = self.get_engine_for_query(query.storage_type, query.namespace.as_deref())?;

        // Capture before images for rollback
        let before_records = engine
            .select(
                query.table.as_str(),
                query.conditions.as_ref(),
                u64::MAX,
                0,
                transaction,
            )
            .await?;

        let data = query.data.clone().unwrap_or(serde_json::Value::Null);
        let op_id = format!("op_{}", transaction.operations.len());

        transaction
            .operations
            .push(transaction::TransactionOperation {
                id: op_id,
                operation_type: transaction::OperationType::Update,
                table: query.table.clone(),
                data: data.clone(),
                conditions: query.conditions.clone(),
                before_image: Some(serde_json::json!(before_records)),
                after_image: None,
                executed: true,
                rollback_data: Some(serde_json::json!(before_records)),
                storage_type: query.storage_type.to_string(),
            });

        let count = engine
            .update(
                query.table.as_str(),
                query.conditions.as_ref(),
                &data,
                transaction,
            )
            .await?;
        Ok(QueryResult::Update(count))
    }

    async fn handle_delete(
        &self,
        query: &Query,
        transaction: &mut transaction::Transaction,
    ) -> Result<QueryResult> {
        let engine = self.get_engine_for_query(query.storage_type, query.namespace.as_deref())?;

        // Capture deleted records for rollback
        let deleted_records = engine
            .select(
                query.table.as_str(),
                query.conditions.as_ref(),
                u64::MAX,
                0,
                transaction,
            )
            .await?;

        let op_id = format!("op_{}", transaction.operations.len());

        transaction
            .operations
            .push(transaction::TransactionOperation {
                id: op_id,
                operation_type: transaction::OperationType::Delete,
                table: query.table.clone(),
                data: serde_json::Value::Null,
                conditions: query.conditions.clone(),
                before_image: Some(serde_json::json!(deleted_records)),
                after_image: None,
                executed: true,
                rollback_data: Some(serde_json::json!(deleted_records)),
                storage_type: query.storage_type.to_string(),
            });

        let count = engine
            .delete(query.table.as_str(), query.conditions.as_ref(), transaction)
            .await?;
        Ok(QueryResult::Delete(count))
    }

    async fn handle_truncate(
        &self,
        query: &Query,
        _transaction: &transaction::Transaction,
    ) -> Result<QueryResult> {
        let engine = self.get_engine_for_query(query.storage_type, query.namespace.as_deref())?;

        let cascade = query
            .data
            .as_ref()
            .and_then(|d| d.get("cascade"))
            .and_then(|c| c.as_bool())
            .unwrap_or(false);

        engine.truncate_table(query.table.as_str(), cascade).await?;
        Ok(QueryResult::Truncate(1))
    }

    async fn handle_analyze(
        &self,
        query: &Query,
        transaction: &transaction::Transaction,
    ) -> Result<QueryResult> {
        let engine = self.get_engine_for_query(query.storage_type, query.namespace.as_deref())?;

        let analysis = engine
            .analyze(query.table.as_str(), query.conditions.as_ref(), transaction)
            .await?;
        Ok(QueryResult::Explain(analysis))
    }

    async fn handle_predict(
        &self,
        query: &Query,
        _transaction: &transaction::Transaction,
    ) -> Result<QueryResult> {
        let predictions = self
            .ai_engine
            .predict(query.table.as_str(), query.conditions.as_ref())
            .await?;
        Ok(QueryResult::Select(predictions))
    }

    // ── ER Model Handler Helpers ──────────────────────────────────────

    fn get_relational_engine(
        &self,
        st: StorageType,
    ) -> Result<&storage::relational::RelationalEngine> {
        let engine = self
            .storage_engines
            .get(&st)
            .ok_or_else(|| Error::StorageEngineNotFound(st))?;
        engine
            .as_ref()
            .as_any()
            .downcast_ref::<storage::relational::RelationalEngine>()
            .ok_or_else(|| {
                Error::ValidationError("Operation requires relational storage engine".to_string())
            })
    }

    fn conv_rq(rq: storage::relational::QueryResult) -> QueryResult {
        match rq {
            storage::relational::QueryResult::Records(recs) => QueryResult::Select(recs),
            storage::relational::QueryResult::AffectedRows(n) => QueryResult::Update(n),
        }
    }

    async fn handle_alter_add_column(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        let field: storage::Field = serde_json::from_value(
            query
                .data
                .clone()
                .ok_or_else(|| Error::ValidationError("Missing column definition".to_string()))?,
        )?;
        rel.alter_table_add_column(&table, field)?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_alter_drop_column(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        let col = query
            .data
            .as_ref()
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ValidationError("Missing column name".to_string()))?;
        rel.alter_table_drop_column(&table, col)?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_alter_modify_column(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        let field: storage::Field = serde_json::from_value(
            query
                .data
                .clone()
                .ok_or_else(|| Error::ValidationError("Missing column definition".to_string()))?,
        )?;
        rel.alter_table_modify_column(&table, field)?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_alter_add_constraint(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        let constraint: storage::Constraint =
            serde_json::from_value(query.data.clone().ok_or_else(|| {
                Error::ValidationError("Missing constraint definition".to_string())
            })?)?;
        rel.alter_table_add_constraint(&table, constraint)?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_alter_drop_constraint(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        let name = query
            .data
            .as_ref()
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ValidationError("Missing constraint name".to_string()))?;
        rel.alter_table_drop_constraint(&table, name)?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_rename_table(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        let new_name = query
            .data
            .as_ref()
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ValidationError("Missing new table name".to_string()))?;
        let resolved_new = self.resolve_table_name(new_name, query.namespace.as_deref())?;
        rel.rename_table(&table, &resolved_new)?;
        Ok(QueryResult::Truncate(1))
    }

    async fn handle_create_sequence(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let mut seq: storage::Sequence =
            serde_json::from_value(query.data.clone().ok_or_else(|| {
                Error::ValidationError("Missing sequence definition".to_string())
            })?)?;
        seq.name = self.resolve_table_name(&seq.name, query.namespace.as_deref())?;
        rel.create_sequence(
            &seq.name,
            seq.increment,
            seq.min_value,
            seq.max_value,
            seq.cycle,
            seq.cache_size,
        )?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_drop_sequence(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        rel.drop_sequence(&table)?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_nextval(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        let val = rel.nextval(&table)?;
        Ok(QueryResult::Insert(val as u64))
    }

    async fn handle_currval(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        let val = rel.currval(&table)?;
        Ok(QueryResult::Insert(val as u64))
    }

    async fn handle_setval(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        let val = query
            .data
            .as_ref()
            .and_then(|v| v.as_i64())
            .ok_or_else(|| Error::ValidationError("Missing sequence value".to_string()))?;
        rel.setval(&table, val)?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_create_view(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let mut view: storage::View = serde_json::from_value(
            query
                .data
                .clone()
                .ok_or_else(|| Error::ValidationError("Missing view definition".to_string()))?,
        )?;
        view.name = self.resolve_table_name(&view.name, query.namespace.as_deref())?;
        view.referenced_tables = view
            .referenced_tables
            .into_iter()
            .map(|t| {
                self.resolve_table_name(&t, query.namespace.as_deref())
                    .unwrap_or(t)
            })
            .collect();
        rel.create_view(
            &view.name,
            view.query_definition,
            view.columns,
            view.referenced_tables,
        )?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_drop_view(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        rel.drop_view(&table)?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_refresh_view(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        rel.refresh_view(&table)?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_create_trigger(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let mut trig: storage::Trigger =
            serde_json::from_value(query.data.clone().ok_or_else(|| {
                Error::ValidationError("Missing trigger definition".to_string())
            })?)?;
        trig.name = self.resolve_table_name(&trig.name, query.namespace.as_deref())?;
        trig.table_name = self.resolve_table_name(&trig.table_name, query.namespace.as_deref())?;
        let timing = match trig.timing {
            storage::TriggerTiming::Before => storage::relational::TriggerTiming::Before,
            storage::TriggerTiming::After => storage::relational::TriggerTiming::After,
            storage::TriggerTiming::InsteadOf => storage::relational::TriggerTiming::InsteadOf,
        };
        let event = match trig.event {
            storage::TriggerEvent::Insert => storage::relational::TriggerEvent::Insert,
            storage::TriggerEvent::Update => storage::relational::TriggerEvent::Update,
            storage::TriggerEvent::Delete => storage::relational::TriggerEvent::Delete,
            storage::TriggerEvent::All => storage::relational::TriggerEvent::All,
        };
        let op = match trig.operation {
            storage::TriggerOperation::Function(f) => {
                storage::relational::TriggerOperation::Function(f)
            }
            storage::TriggerOperation::Execute(s) => {
                storage::relational::TriggerOperation::Execute(s)
            }
            storage::TriggerOperation::Raise(m) => storage::relational::TriggerOperation::Raise(m),
        };
        rel.create_trigger(&trig.name, &trig.table_name, timing, event, op)?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_drop_trigger(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        let trig_name = query
            .data
            .as_ref()
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ValidationError("Missing trigger name".to_string()))?;
        let resolved_name = self.resolve_table_name(trig_name, query.namespace.as_deref())?;
        rel.drop_trigger(&table, &resolved_name)?;
        Ok(QueryResult::Truncate(0))
    }

    async fn handle_info_schema_tables(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        Ok(Self::conv_rq(rel.get_information_schema_tables()?))
    }

    async fn handle_info_schema_columns(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        Ok(Self::conv_rq(rel.get_information_schema_columns(&table)?))
    }

    async fn handle_info_schema_constraints(&self, query: &Query) -> Result<QueryResult> {
        let rel = self.get_relational_engine(query.storage_type)?;
        let table = self.resolve_table_name(&query.table, query.namespace.as_deref())?;
        Ok(Self::conv_rq(
            rel.get_information_schema_constraints(&table)?,
        ))
    }

    pub async fn rollback_transaction(&self, transaction_id: String) -> Result<()> {
        self.transaction_manager
            .rollback_transaction(transaction_id)
            .await
    }

    pub async fn begin_transaction(&self) -> Result<String> {
        let tx = self.transaction_manager.begin_transaction().await?;
        let tx_id = tx.id.clone();
        self.active_transactions
            .lock()
            .unwrap()
            .insert(tx_id.clone(), tx);
        Ok(tx_id)
    }

    pub async fn commit_transaction(&self, transaction_id: String) -> Result<()> {
        let tx = self
            .active_transactions
            .lock()
            .unwrap()
            .remove(&transaction_id)
            .ok_or_else(|| {
                Error::ValidationError(format!(
                    "Transaction '{}' not found or already completed",
                    transaction_id
                ))
            })?;
        self.transaction_manager.commit_transaction(tx).await
    }

    pub async fn create_table(&self, storage_type: StorageType, table: &str) -> Result<()> {
        let engine = self
            .storage_engines
            .get(&storage_type)
            .ok_or_else(|| Error::StorageEngineNotFound(storage_type))?;
        let schema = storage::Schema {
            fields: vec![],
            indexes: vec![],
            constraints: vec![],
        };
        engine.create_table(table, &schema).await
    }

    pub async fn drop_table(&self, storage_type: StorageType, table: &str) -> Result<()> {
        let engine = self
            .storage_engines
            .get(&storage_type)
            .ok_or_else(|| Error::StorageEngineNotFound(storage_type))?;
        engine.drop_table(table).await
    }

    pub async fn table_info(
        &self,
        storage_type: StorageType,
        table: &str,
    ) -> Result<storage::TableInfo> {
        let engine = self
            .storage_engines
            .get(&storage_type)
            .ok_or_else(|| Error::StorageEngineNotFound(storage_type))?;
        engine.table_info(table).await
    }

    #[allow(clippy::await_holding_lock)]
    pub async fn get_cluster_status(&self) -> Result<cluster::ClusterStatusInfo> {
        let cm = self.cluster_manager.read().unwrap();
        Ok(cm.get_cluster_status().await)
    }

    pub fn get_cluster_manager(&self) -> Arc<RwLock<cluster::ClusterManager>> {
        self.cluster_manager.clone()
    }

    pub fn get_sync_coordinator(&self) -> Arc<cluster::sync::SyncCoordinator> {
        self.sync_coordinator.clone()
    }

    pub fn get_cluster_auth(&self) -> Arc<tokio::sync::RwLock<auth::ClusterAuthManager>> {
        self.cluster_auth.clone()
    }

    pub fn get_namespace_controller(&self) -> Arc<namespace::NamespaceController> {
        self.namespace_controller.clone()
    }

    fn get_engine_for_query(
        &self,
        storage_type: StorageType,
        namespace: Option<&str>,
    ) -> Result<Arc<dyn storage::StorageEngine>> {
        let engine = self
            .storage_engines
            .get(&storage_type)
            .ok_or_else(|| Error::StorageEngineNotFound(storage_type))?;

        match namespace {
            Some(ns_path) if !ns_path.is_empty() && self.config.namespaces.enabled => {
                let ns = self
                    .namespace_controller
                    .get_by_path(ns_path)?
                    .ok_or_else(|| {
                        Error::ValidationError(format!("Namespace '{}' not found", ns_path))
                    })?;
                let ns_path = ns.path.clone();
                Ok(Arc::new(namespace::storage::NamespacedStorageEngine::new(
                    engine.clone(),
                    self.namespace_controller.clone(),
                    ns_path,
                    storage_type,
                )))
            }
            _ => Ok(engine.clone()),
        }
    }

    fn resolve_table_name(&self, table: &str, namespace: Option<&str>) -> Result<String> {
        match namespace {
            Some(ns_path) if !ns_path.is_empty() && self.config.namespaces.enabled => {
                let ns = self
                    .namespace_controller
                    .get_by_path(ns_path)?
                    .ok_or_else(|| {
                        Error::ValidationError(format!("Namespace '{}' not found", ns_path))
                    })?;
                Ok(namespace::compute_physical_name(&ns.path, table))
            }
            _ => Ok(table.to_string()),
        }
    }

    pub fn enable_collection_encryption(
        &self,
        storage_type: StorageType,
        collection: &str,
    ) -> Result<()> {
        let engine = self
            .storage_engines
            .get(&storage_type)
            .ok_or_else(|| Error::StorageEngineNotFound(storage_type))?;

        if let Some(doc_engine) = engine
            .as_ref()
            .as_any()
            .downcast_ref::<storage::document::DocumentEngine>()
        {
            doc_engine.enable_collection_encryption(collection)
        } else {
            Err(crate::Error::ValidationError(
                "Collection encryption only supported for Document storage".to_string(),
            ))
        }
    }

    pub fn disable_collection_encryption(
        &self,
        storage_type: StorageType,
        collection: &str,
    ) -> Result<()> {
        let engine = self
            .storage_engines
            .get(&storage_type)
            .ok_or_else(|| Error::StorageEngineNotFound(storage_type))?;

        if let Some(doc_engine) = engine
            .as_ref()
            .as_any()
            .downcast_ref::<storage::document::DocumentEngine>()
        {
            doc_engine.disable_collection_encryption(collection)
        } else {
            Err(crate::Error::ValidationError(
                "Collection encryption only supported for Document storage".to_string(),
            ))
        }
    }

    pub fn is_collection_encrypted(
        &self,
        storage_type: StorageType,
        collection: &str,
    ) -> Result<bool> {
        let engine = self
            .storage_engines
            .get(&storage_type)
            .ok_or_else(|| Error::StorageEngineNotFound(storage_type))?;

        if let Some(doc_engine) = engine
            .as_ref()
            .as_any()
            .downcast_ref::<storage::document::DocumentEngine>()
        {
            doc_engine.is_collection_encrypted(collection)
        } else {
            Err(crate::Error::ValidationError(
                "Collection encryption only supported for Document storage".to_string(),
            ))
        }
    }

    pub fn get_encrypted_collections(&self, storage_type: StorageType) -> Result<Vec<String>> {
        let engine = self
            .storage_engines
            .get(&storage_type)
            .ok_or_else(|| Error::StorageEngineNotFound(storage_type))?;

        if let Some(doc_engine) = engine
            .as_ref()
            .as_any()
            .downcast_ref::<storage::document::DocumentEngine>()
        {
            doc_engine.get_encrypted_collections()
        } else {
            Err(crate::Error::ValidationError(
                "Collection encryption only supported for Document storage".to_string(),
            ))
        }
    }

    /// Start a background task that periodically builds and commits blocks
    /// from the mempool. Runs every `interval_ms` milliseconds.
    pub fn start_background_producer(&self, interval_ms: u64) {
        let consensus = self.consensus_engine.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
            loop {
                interval.tick().await;
                match consensus.build_and_commit_block().await {
                    Ok(Some(block)) => {
                        tracing::info!(
                            "Background producer committed block at height {}",
                            block.height
                        );
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!("Background producer error: {}", e);
                    }
                }
            }
        });
        *self.producer_handle.lock().unwrap() = Some(handle);
    }
}
