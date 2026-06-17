use crate::query::{QueryLanguage, UqlQuery, UqlResult};
use crate::{PrimusDB, PrimusDBConfig, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

pub struct DriverManager {
    drivers: HashMap<String, Box<dyn DatabaseDriver>>,
}

#[async_trait]
pub trait DatabaseDriver: Send + Sync {
    async fn connect(&self, connection_string: &str) -> Result<Box<dyn Connection>>;
    fn driver_name(&self) -> &'static str;
    fn supported_features(&self) -> Vec<DriverFeature>;
}

#[async_trait]
pub trait Connection: Send + Sync {
    async fn execute_query(
        &mut self,
        query: &str,
        params: Option<&[serde_json::Value]>,
    ) -> Result<QueryResult>;
    async fn begin_transaction(&mut self) -> Result<Transaction>;
    async fn commit_transaction(&mut self, transaction: Transaction) -> Result<()>;
    async fn rollback_transaction(&mut self, transaction: Transaction) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub isolation_level: IsolationLevel,
}

#[derive(Debug, Clone)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

#[derive(Debug, Clone)]
pub enum DriverFeature {
    Transactions,
    PreparedStatements,
    AsyncOperations,
    ConnectionPooling,
    SSL,
    Compression,
    ReferentialActions,
    Sequences,
    Views,
    Triggers,
    AlterTable,
    ReturningClause,
    GroupByQuery,
    InformationSchema,
    TruncateCascade,
    ExtendedDataTypes,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub rows: Vec<Row>,
    pub affected_rows: u64,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub columns: HashMap<String, serde_json::Value>,
}

impl Default for DriverManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DriverManager {
    pub fn new() -> Self {
        DriverManager {
            drivers: HashMap::new(),
        }
    }

    pub fn register_driver(&mut self, driver: Box<dyn DatabaseDriver>) {
        let name = driver.driver_name().to_string();
        self.drivers.insert(name, driver);
    }

    pub fn get_driver(&self, name: &str) -> Option<&dyn DatabaseDriver> {
        self.drivers.get(name).map(|d| d.as_ref())
    }

    pub fn list_drivers(&self) -> Vec<String> {
        self.drivers.keys().map(|k| k.to_string()).collect()
    }
}

// ── Shared connection logic ──────────────────────────────────────────

fn create_primusdb(config: &PrimusDBConfig) -> Result<Arc<PrimusDB>> {
    Ok(Arc::new(PrimusDB::new(config.clone())?))
}

fn convert_uql_result(r: UqlResult) -> QueryResult {
    if r.affected_rows > 0 {
        QueryResult {
            rows: vec![],
            affected_rows: r.affected_rows,
            execution_time_ms: r.execution_time_ms,
        }
    } else {
        let rows = r
            .records
            .into_iter()
            .map(|rec| {
                let mut cols = HashMap::new();
                cols.insert("_id".to_string(), serde_json::Value::String(rec.id));
                if let serde_json::Value::Object(map) = rec.data {
                    for (k, v) in map {
                        cols.insert(k, v);
                    }
                }
                for (k, v) in rec.metadata {
                    cols.insert(k, serde_json::Value::String(v));
                }
                Row { columns: cols }
            })
            .collect();
        QueryResult {
            rows,
            affected_rows: 0,
            execution_time_ms: r.execution_time_ms,
        }
    }
}

async fn execute_on_primusdb(primusdb: &PrimusDB, sql: &str) -> Result<QueryResult> {
    let uql_query = UqlQuery {
        query: sql.to_string(),
        query_type: QueryLanguage::Sql,
        parameters: None,
    };
    let result = primusdb.uql_execute_query(&uql_query)?;
    Ok(convert_uql_result(result))
}

// ── Rust Driver ──────────────────────────────────────────────────────

pub struct RustDriver;

#[async_trait]
impl DatabaseDriver for RustDriver {
    async fn connect(&self, connection_string: &str) -> Result<Box<dyn Connection>> {
        let config = parse_connection_string(connection_string);
        let primusdb = create_primusdb(&config)?;
        Ok(Box::new(RustConnection {
            primusdb,
            connected: true,
        }))
    }

    fn driver_name(&self) -> &'static str {
        "rust"
    }

    fn supported_features(&self) -> Vec<DriverFeature> {
        vec![
            DriverFeature::Transactions,
            DriverFeature::AsyncOperations,
            DriverFeature::ConnectionPooling,
            DriverFeature::ReferentialActions,
            DriverFeature::Sequences,
            DriverFeature::Views,
            DriverFeature::Triggers,
            DriverFeature::AlterTable,
            DriverFeature::ReturningClause,
            DriverFeature::GroupByQuery,
            DriverFeature::InformationSchema,
            DriverFeature::TruncateCascade,
            DriverFeature::ExtendedDataTypes,
        ]
    }
}

pub struct RustConnection {
    primusdb: Arc<PrimusDB>,
    connected: bool,
}

#[async_trait]
impl Connection for RustConnection {
    async fn execute_query(
        &mut self,
        query: &str,
        _params: Option<&[serde_json::Value]>,
    ) -> Result<QueryResult> {
        execute_on_primusdb(&self.primusdb, query).await
    }

    async fn begin_transaction(&mut self) -> Result<Transaction> {
        Ok(Transaction {
            id: format!(
                "tx_{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            isolation_level: IsolationLevel::ReadCommitted,
        })
    }

    async fn commit_transaction(&mut self, _transaction: Transaction) -> Result<()> {
        Ok(())
    }

    async fn rollback_transaction(&mut self, _transaction: Transaction) -> Result<()> {
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }
}

// ── Python Driver ────────────────────────────────────────────────────

pub struct PythonDriver;

#[async_trait]
impl DatabaseDriver for PythonDriver {
    async fn connect(&self, connection_string: &str) -> Result<Box<dyn Connection>> {
        let config = parse_connection_string(connection_string);
        let primusdb = create_primusdb(&config)?;
        Ok(Box::new(PythonConnection {
            primusdb,
            connected: true,
        }))
    }

    fn driver_name(&self) -> &'static str {
        "python"
    }

    fn supported_features(&self) -> Vec<DriverFeature> {
        vec![
            DriverFeature::Transactions,
            DriverFeature::PreparedStatements,
            DriverFeature::AsyncOperations,
            DriverFeature::ReferentialActions,
            DriverFeature::Sequences,
            DriverFeature::Views,
            DriverFeature::Triggers,
            DriverFeature::AlterTable,
            DriverFeature::ReturningClause,
            DriverFeature::GroupByQuery,
            DriverFeature::InformationSchema,
            DriverFeature::TruncateCascade,
            DriverFeature::ExtendedDataTypes,
        ]
    }
}

pub struct PythonConnection {
    primusdb: Arc<PrimusDB>,
    connected: bool,
}

#[async_trait]
impl Connection for PythonConnection {
    async fn execute_query(
        &mut self,
        query: &str,
        _params: Option<&[serde_json::Value]>,
    ) -> Result<QueryResult> {
        execute_on_primusdb(&self.primusdb, query).await
    }

    async fn begin_transaction(&mut self) -> Result<Transaction> {
        Ok(Transaction {
            id: format!(
                "py_tx_{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            isolation_level: IsolationLevel::ReadCommitted,
        })
    }

    async fn commit_transaction(&mut self, _transaction: Transaction) -> Result<()> {
        Ok(())
    }

    async fn rollback_transaction(&mut self, _transaction: Transaction) -> Result<()> {
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }
}

// ── Node.js Driver ───────────────────────────────────────────────────

pub struct NodeDriver;

#[async_trait]
impl DatabaseDriver for NodeDriver {
    async fn connect(&self, connection_string: &str) -> Result<Box<dyn Connection>> {
        let config = parse_connection_string(connection_string);
        let primusdb = create_primusdb(&config)?;
        Ok(Box::new(NodeConnection {
            primusdb,
            connected: true,
        }))
    }

    fn driver_name(&self) -> &'static str {
        "node"
    }

    fn supported_features(&self) -> Vec<DriverFeature> {
        vec![
            DriverFeature::Transactions,
            DriverFeature::AsyncOperations,
            DriverFeature::ConnectionPooling,
            DriverFeature::ReferentialActions,
            DriverFeature::Sequences,
            DriverFeature::Views,
            DriverFeature::Triggers,
            DriverFeature::AlterTable,
            DriverFeature::ReturningClause,
            DriverFeature::GroupByQuery,
            DriverFeature::InformationSchema,
            DriverFeature::TruncateCascade,
            DriverFeature::ExtendedDataTypes,
        ]
    }
}

pub struct NodeConnection {
    primusdb: Arc<PrimusDB>,
    connected: bool,
}

#[async_trait]
impl Connection for NodeConnection {
    async fn execute_query(
        &mut self,
        query: &str,
        _params: Option<&[serde_json::Value]>,
    ) -> Result<QueryResult> {
        execute_on_primusdb(&self.primusdb, query).await
    }

    async fn begin_transaction(&mut self) -> Result<Transaction> {
        Ok(Transaction {
            id: format!(
                "node_tx_{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            isolation_level: IsolationLevel::ReadCommitted,
        })
    }

    async fn commit_transaction(&mut self, _transaction: Transaction) -> Result<()> {
        Ok(())
    }

    async fn rollback_transaction(&mut self, _transaction: Transaction) -> Result<()> {
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }
}

// ── Java/JDBC Driver ─────────────────────────────────────────────────

pub struct JavaDriver;

#[async_trait]
impl DatabaseDriver for JavaDriver {
    async fn connect(&self, connection_string: &str) -> Result<Box<dyn Connection>> {
        let config = parse_connection_string(connection_string);
        let primusdb = create_primusdb(&config)?;
        Ok(Box::new(JavaConnection {
            primusdb,
            connected: true,
        }))
    }

    fn driver_name(&self) -> &'static str {
        "java"
    }

    fn supported_features(&self) -> Vec<DriverFeature> {
        vec![
            DriverFeature::Transactions,
            DriverFeature::PreparedStatements,
            DriverFeature::SSL,
            DriverFeature::ConnectionPooling,
            DriverFeature::ReferentialActions,
            DriverFeature::Sequences,
            DriverFeature::Views,
            DriverFeature::Triggers,
            DriverFeature::AlterTable,
            DriverFeature::ReturningClause,
            DriverFeature::GroupByQuery,
            DriverFeature::InformationSchema,
            DriverFeature::TruncateCascade,
            DriverFeature::ExtendedDataTypes,
        ]
    }
}

pub struct JavaConnection {
    primusdb: Arc<PrimusDB>,
    connected: bool,
}

#[async_trait]
impl Connection for JavaConnection {
    async fn execute_query(
        &mut self,
        query: &str,
        _params: Option<&[serde_json::Value]>,
    ) -> Result<QueryResult> {
        execute_on_primusdb(&self.primusdb, query).await
    }

    async fn begin_transaction(&mut self) -> Result<Transaction> {
        Ok(Transaction {
            id: format!(
                "java_tx_{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            isolation_level: IsolationLevel::ReadCommitted,
        })
    }

    async fn commit_transaction(&mut self, _transaction: Transaction) -> Result<()> {
        Ok(())
    }

    async fn rollback_transaction(&mut self, _transaction: Transaction) -> Result<()> {
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }
}

// ── Ruby Driver ──────────────────────────────────────────────────────

pub struct RubyDriver;

#[async_trait]
impl DatabaseDriver for RubyDriver {
    async fn connect(&self, connection_string: &str) -> Result<Box<dyn Connection>> {
        let config = parse_connection_string(connection_string);
        let primusdb = create_primusdb(&config)?;
        Ok(Box::new(RubyConnection {
            primusdb,
            connected: true,
        }))
    }

    fn driver_name(&self) -> &'static str {
        "ruby"
    }

    fn supported_features(&self) -> Vec<DriverFeature> {
        vec![
            DriverFeature::Transactions,
            DriverFeature::AsyncOperations,
            DriverFeature::ReferentialActions,
            DriverFeature::Sequences,
            DriverFeature::Views,
            DriverFeature::Triggers,
            DriverFeature::AlterTable,
            DriverFeature::ReturningClause,
            DriverFeature::GroupByQuery,
            DriverFeature::InformationSchema,
            DriverFeature::TruncateCascade,
            DriverFeature::ExtendedDataTypes,
        ]
    }
}

pub struct RubyConnection {
    primusdb: Arc<PrimusDB>,
    connected: bool,
}

#[async_trait]
impl Connection for RubyConnection {
    async fn execute_query(
        &mut self,
        query: &str,
        _params: Option<&[serde_json::Value]>,
    ) -> Result<QueryResult> {
        execute_on_primusdb(&self.primusdb, query).await
    }

    async fn begin_transaction(&mut self) -> Result<Transaction> {
        Ok(Transaction {
            id: format!(
                "ruby_tx_{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            isolation_level: IsolationLevel::ReadCommitted,
        })
    }

    async fn commit_transaction(&mut self, _transaction: Transaction) -> Result<()> {
        Ok(())
    }

    async fn rollback_transaction(&mut self, _transaction: Transaction) -> Result<()> {
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }
}

// ── Connection string parser ─────────────────────────────────────────

fn parse_connection_string(s: &str) -> PrimusDBConfig {
    let mut config = PrimusDBConfig::default();
    for part in s.split(';') {
        let kv: Vec<&str> = part.splitn(2, '=').collect();
        if kv.len() == 2 {
            match kv[0].trim().to_lowercase().as_str() {
                "data_dir" | "datadir" => config.storage.data_dir = kv[1].trim().to_string(),
                "port" => config.network.port = kv[1].trim().parse().unwrap_or(8080),
                _ => {}
            }
        }
    }
    config
}

impl Default for PrimusDBConfig {
    fn default() -> Self {
        PrimusDBConfig {
            storage: crate::StorageConfig {
                data_dir: "./data".to_string(),
                max_file_size: 1073741824,
                compression: crate::CompressionType::Lz4,
                cache_size: 104857600,
            },
            network: crate::NetworkConfig {
                bind_address: "127.0.0.1".to_string(),
                port: 8080,
                max_connections: 1000,
            },
            security: crate::SecurityConfig {
                encryption_enabled: false,
                key_rotation_interval: 86400,
                auth_required: false,
            },
            cluster: crate::ClusterConfig {
                enabled: false,
                node_id: "driver_node".to_string(),
                discovery_servers: vec![],
            },
            namespaces: Default::default(),
            federation: None,
        }
    }
}
