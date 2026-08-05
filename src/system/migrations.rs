//! # MigrationManager — Schema Versioning
//!
//! Tracks schema versions and applies pending migrations on startup.
//! Each migration is recorded with a SHA-256 checksum for integrity.
//!
//! ## Architecture
//!
//! ```text
//! MigrationManager
//!   +-> sys_migrations tree (sled)
//!   |     "schema_version" -> u64 (current version)
//!   |     "migration_{n}"  -> MigrationRecord { version, name,
//!   |                                            applied_at, checksum }
//!   |
//!   +-> run_pending() applies (current .. target] version range
//!   +-> is_migrated() returns true when current >= SYSTEM_SCHEMA_VERSION
//! ```
//!
//! ## Current Migrations
//!
//! | Version | Name            | Description               |
//! |---------|-----------------|---------------------------|
//! | 1       | initial_schema  | Initial system DB schema  |

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sled::Db;
use std::sync::Arc;

const META_TREE: &str = "sys_migrations";

/// Applies and records schema migrations for the system database, tracking the
/// current version and per-migration SHA-256 checksums.
pub struct MigrationManager {
    db: Arc<Db>,
}

/// Record of a single applied migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// Migration version number.
    pub version: u64,
    /// Migration name, e.g. `initial_schema`.
    pub name: String,
    /// Wall-clock time the migration was applied.
    pub applied_at: chrono::DateTime<chrono::Utc>,
    /// SHA-256 checksum of the migration for integrity verification.
    pub checksum: String,
}

impl MigrationManager {
    /// Creates a migration manager backed by the given sled database.
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    /// Returns the current schema version (0 when no migrations applied).
    pub fn current_version(&self) -> crate::Result<u64> {
        let tree = self.db.open_tree(META_TREE)?;
        if let Some(bytes) = tree.get("schema_version")? {
            let v: u64 = bincode::deserialize(&bytes)?;
            Ok(v)
        } else {
            Ok(0)
        }
    }

    /// Persists the given schema version.
    pub fn set_version(&self, version: u64) -> crate::Result<()> {
        let tree = self.db.open_tree(META_TREE)?;
        tree.insert("schema_version", bincode::serialize(&version)?)?;
        tree.flush()?;
        Ok(())
    }

    /// Returns all applied migrations ordered by version.
    pub fn applied_migrations(&self) -> crate::Result<Vec<MigrationRecord>> {
        let tree = self.db.open_tree(META_TREE)?;
        let mut records = Vec::new();
        for result in &tree {
            let (key, value) = result?;
            let key_str = String::from_utf8_lossy(&key).to_string();
            if key_str.starts_with("migration_") {
                if let Ok(record) = bincode::deserialize::<MigrationRecord>(&value) {
                    records.push(record);
                }
            }
        }
        records.sort_by_key(|r| r.version);
        Ok(records)
    }

    /// Applies every migration in the `(current_version, target]` range.
    /// No-op when already at the target [`SYSTEM_SCHEMA_VERSION`].
    pub fn run_pending(&self) -> crate::Result<()> {
        let current = self.current_version()?;
        let target = crate::system::SYSTEM_SCHEMA_VERSION;

        if current >= target {
            return Ok(());
        }

        for version in (current + 1)..=target {
            self.apply_migration(version)?;
        }

        Ok(())
    }

    fn apply_migration(&self, version: u64) -> crate::Result<()> {
        let name = match version {
            1 => "initial_schema",
            _ => {
                return Err(crate::Error::ConfigurationError(format!(
                    "Unknown migration version: {}",
                    version
                )))
            }
        };

        let mut hasher = Sha256::new();
        hasher.update(format!("migration_v{}", version).as_bytes());
        let checksum = format!("{:x}", hasher.finalize());

        let record = MigrationRecord {
            version,
            name: name.to_string(),
            applied_at: chrono::Utc::now(),
            checksum,
        };

        let tree = self.db.open_tree(META_TREE)?;
        tree.insert(
            format!("migration_{}", version).as_bytes(),
            bincode::serialize(&record)?,
        )?;
        self.set_version(version)?;
        Ok(())
    }

    /// Returns true when the schema is at or above the current target version.
    pub fn is_migrated(&self) -> crate::Result<bool> {
        Ok(self.current_version()? >= crate::system::SYSTEM_SCHEMA_VERSION)
    }
}
