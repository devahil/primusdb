//! Source database trait and schema types.
//!
//! Defines the `MigrationSource` trait that all source connectors implement,
//! along with the common data types (`SourceSchema`, `SourceDatabase`,
//! `SourceObject`, `SourceColumn`, `RowStream`).
//!
//! ```text
//! +-------------------+
//! |  MigrationSource  |  (trait)
//! +-------------------+
//!          |
//!    +-----+-----+-----+-----+
//!    |     |     |     |     |
//!  MySQL  PG   Mongo CouchDB ...
//!    |     |     |     |
//!    +-----+-----+-----+
//!          |
//!          v
//! +-------------------+
//! |   SourceSchema    |
//! +-------------------+
//! ```

use serde::{Deserialize, Serialize};

use crate::Result;

/// Schema information extracted from a source database.
///
/// Contains a list of databases and their objects (tables, collections, etc.)
/// that can be migrated to PrimusDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSchema {
    /// The databases discovered in the source system.
    pub databases: Vec<SourceDatabase>,
}

/// Represents a single database within the source system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDatabase {
    /// Name of the database.
    pub name: String,
    /// Objects (tables, collections, etc.) within this database.
    pub objects: Vec<SourceObject>,
}

/// Represents a single object (table/collection) in the source schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceObject {
    /// Name of the object.
    pub name: String,
    /// Type of object (e.g. "table", "collection", "view").
    pub object_type: String,
    /// Columns/fields in this object.
    pub columns: Vec<SourceColumn>,
    /// Estimated number of rows/documents.
    pub row_estimate: Option<u64>,
    /// Names of columns that form the primary key.
    pub primary_key: Vec<String>,
}

/// Describes a single column/field in a source object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceColumn {
    /// Column name.
    pub name: String,
    /// Native data type as reported by the source (e.g. "varchar(255)", "int").
    pub data_type: String,
    /// Whether the column allows NULL values.
    pub nullable: bool,
    /// Whether this column is part of the primary key.
    pub is_primary_key: bool,
    /// Maximum length for variable-length types.
    pub max_length: Option<u64>,
}

/// A stream of rows from a source object.
///
/// Contains column names and a batch of row data as JSON values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowStream {
    /// Column names in order matching the rows.
    pub columns: Vec<String>,
    /// Batched row data.
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// Core trait for extracting schema and data from external database systems.
///
/// Implementations connect to the source database and provide schema inspection
/// and row streaming capabilities. When the required database driver is not
/// available, implementations return [`crate::Error::Unsupported`].
pub trait MigrationSource {
    /// Human-readable name of the source database type.
    fn name(&self) -> &str;
    /// Inspect the source database and return its schema.
    fn inspect_schema(&self) -> Result<SourceSchema>;
    /// Stream rows from a specific source object.
    ///
    /// The object must come from a schema returned by [`inspect_schema`].
    fn stream_rows(&self, object: &SourceObject) -> Result<RowStream>;
}
