//! Source database implementations for the PrimusDB migration framework.
//!
//! Each submodule corresponds to a supported source database type.
//! When the required database driver crate is not present in `Cargo.toml`,
//! the constructors return [`crate::Error::Unsupported`] with a clear
//! message describing what dependency is needed.

pub mod couchdb;
pub mod mongodb;
pub mod mysql;
pub mod postgres;

use super::source::MigrationSource;
use crate::Result;

/// Create a [`MigrationSource`] for the given source database type and URL.
///
/// Supported source types: `mysql`, `postgres`, `mongodb`, `couchdb`.
///
/// Returns [`crate::Error::Unsupported`] for unknown or unavailable source types.
pub fn create_source(source_type: &str, url: &str) -> Result<Box<dyn MigrationSource>> {
    match source_type {
        "mysql" => Ok(Box::new(mysql::MySqlSource::new(url)?)),
        "postgres" | "postgresql" => Ok(Box::new(postgres::PostgresSource::new(url)?)),
        "mongodb" | "mongo" => Ok(Box::new(mongodb::MongoSource::new(url)?)),
        "couchdb" | "couch" => Ok(Box::new(couchdb::CouchSource::new(url)?)),
        other => Err(crate::Error::Unsupported(format!(
            "Unknown source type '{}'. Supported sources: mysql, postgres, mongodb, couchdb",
            other
        ))),
    }
}
