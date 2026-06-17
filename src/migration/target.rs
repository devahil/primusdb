//! Target writer trait and REST implementation.
//!
//! Defines the `PrimusMigrationWriter` trait for writing data to a PrimusDB
//! server, along with `DataBatch`, `WriteResult`, and `ObjectMapping` types.
//!
//! The built-in `RestWriter` implementation writes via the PrimusDB REST API.
//!
//! ```text
//! +-----------------------+
//! | PrimusMigrationWriter |  (trait)
//! +-----------------------+
//!            |
//!    +-------+-------+
//!    |               |
//! RestWriter    ... (future)
//!    |
//!    v
//! +-----------------------+
//! |   PrimusDB REST API   |
//! +-----------------------+
//! ```

use serde::{Deserialize, Serialize};

use crate::Result;

/// A batch of data to be written to a PrimusDB target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBatch {
    /// Target object/table name in PrimusDB.
    pub target: String,
    /// Storage engine to use (e.g. "relational", "document", "columnar").
    pub engine: String,
    /// Column names in order matching the rows.
    pub columns: Vec<String>,
    /// Row data as JSON values.
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// Result of a single write batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    /// Number of rows successfully written.
    pub rows_written: u64,
    /// Error messages for any rows that failed.
    pub errors: Vec<String>,
}

/// Specifies how a source object maps to a PrimusDB target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMapping {
    /// Source object name.
    pub source: String,
    /// Target object name in PrimusDB.
    pub target: String,
    /// Storage engine for the target (e.g. "relational", "document").
    pub engine: String,
    /// Optional primary key column name.
    pub primary_key: Option<String>,
    /// Per-field mappings from source to target.
    pub field_mappings: Vec<FieldMapping>,
}

/// Maps a single source field to a target field with optional type override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    /// Source column/field name.
    pub source: String,
    /// Target column/field name.
    pub target: String,
    /// Optional type override to apply during migration.
    pub type_override: Option<String>,
}

/// Core trait for writing data to a PrimusDB target via its REST API.
pub trait PrimusMigrationWriter {
    /// Human-readable name of this writer.
    fn name(&self) -> &str;
    /// Create the target object (table/collection) before writing data.
    fn create_target(&self, mapping: &ObjectMapping) -> Result<()>;
    /// Write a batch of data to the target.
    fn write_batch(&self, batch: DataBatch) -> Result<WriteResult>;
}

/// Writer that communicates with a running PrimusDB server via its REST API.
pub struct RestWriter {
    /// Base URL of the PrimusDB server.
    server_url: String,
    /// Namespace to write into.
    namespace: String,
    /// Shared blocking HTTP client.
    client: reqwest::blocking::Client,
}

impl RestWriter {
    /// Create a new `RestWriter` targeting the given PrimusDB server and namespace.
    pub fn new(server_url: String, namespace: String) -> Self {
        Self {
            server_url,
            namespace,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!(
            "{}/namespaces/{}/{}",
            self.server_url.trim_end_matches('/'),
            self.namespace,
            path.trim_start_matches('/'),
        )
    }
}

impl PrimusMigrationWriter for RestWriter {
    fn name(&self) -> &str {
        "primusdb-rest"
    }

    fn create_target(&self, mapping: &ObjectMapping) -> Result<()> {
        let body = serde_json::json!({
            "name": mapping.target,
            "engine": mapping.engine,
            "primary_key": mapping.primary_key,
        });
        let resp = self
            .client
            .post(self.api_url("tables"))
            .json(&body)
            .send()
            .map_err(|e| crate::Error::NetworkError(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(crate::Error::HttpError(format!(
                "Failed to create target '{}': HTTP {} - {}",
                mapping.target, status, text
            )));
        }
        Ok(())
    }

    fn write_batch(&self, batch: DataBatch) -> Result<WriteResult> {
        let body = serde_json::json!({
            "table": batch.target,
            "engine": batch.engine,
            "columns": batch.columns,
            "rows": batch.rows,
        });
        let resp = self
            .client
            .post(self.api_url("data/batch"))
            .json(&body)
            .send()
            .map_err(|e| crate::Error::NetworkError(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(crate::Error::HttpError(format!(
                "Batch write failed for '{}': HTTP {} - {}",
                batch.target, status, text
            )));
        }
        let result: WriteResult = resp
            .json()
            .map_err(|e| crate::Error::NetworkError(e.to_string()))?;
        Ok(result)
    }
}
