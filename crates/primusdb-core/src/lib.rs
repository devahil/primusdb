use serde::{Deserialize, Serialize};
use std::fmt;

/// Core types and traits for PrimusDB
pub type Result<T> = std::result::Result<T, Error>;

/// Storage types supported by PrimusDB
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    Columnar,
    Vector,
    Document,
    Relational,
}

impl fmt::Display for StorageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageType::Columnar => write!(f, "columnar"),
            StorageType::Vector => write!(f, "vector"),
            StorageType::Document => write!(f, "document"),
            StorageType::Relational => write!(f, "relational"),
        }
    }
}

/// Query operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryOperation {
    Create,
    Read,
    Update,
    Delete,
    Analyze,
    Predict,
}

/// Main query structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub storage_type: StorageType,
    pub operation: QueryOperation,
    pub table: String,
    pub conditions: Option<serde_json::Value>,
    pub data: Option<serde_json::Value>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Configuration for PrimusDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimusDBConfig {
    pub storage: StorageConfig,
    pub network: NetworkConfig,
    pub security: SecurityConfig,
    pub cluster: ClusterConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: String,
    pub max_file_size: u64,
    pub compression: CompressionType,
    pub cache_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub bind_address: String,
    pub port: u16,
    pub max_connections: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub encryption_enabled: bool,
    pub key_rotation_interval: u64,
    pub auth_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub enabled: bool,
    pub node_id: String,
    pub discovery_servers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Lz4,
    Zstd,
}

impl Default for CompressionType {
    fn default() -> Self {
        CompressionType::Lz4
    }
}

/// Error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Request error: {0}")]
    RequestError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Consensus error: {0}")]
    ConsensusError(String),
    #[error("Security error: {0}")]
    SecurityError(String),
    #[error("Transaction error: {0}")]
    TransactionError(String),
    #[error("AI/ML error: {0}")]
    AiError(String),
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

/// Placeholder for PrimusDB main struct (simplified for drivers)
#[derive(Debug)]
pub struct PrimusDB;

impl PrimusDB {
    pub fn new(_config: PrimusDBConfig) -> Result<Self> {
        Ok(PrimusDB)
    }

    pub async fn execute_query(&self, _query: Query) -> Result<QueryResult> {
        // Placeholder implementation
        Ok(QueryResult::Select(vec![]))
    }

    pub fn get_cluster_status(&self) -> Result<ClusterStatus> {
        Ok(ClusterStatus {
            nodes: vec![],
            leader: None,
            status: "ok".to_string(),
        })
    }
}

/// Query result types
#[derive(Debug, Serialize, Deserialize)]
pub enum QueryResult {
    Insert(u64),
    Update(u64),
    Delete(u64),
    Select(Vec<serde_json::Value>),
    Explain(String),
}

/// Cluster status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    pub nodes: Vec<String>,
    pub leader: Option<String>,
    pub status: String,
}
