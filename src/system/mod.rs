//! # PrimusDB System Database
//!
//! The system database is an internal metadata, configuration, and audit
//! persistence layer. It is sled-backed and lives at `{data_dir}/system/`.
//!
//! ## Architecture
//!
//! ```text
//! +--------------------------------------------+
//! |              SystemDatabase                 |
//! |  (sled::Db at {data_dir}/system/)          |
//! +--------------------------------------------+
//!          |            |           |          |
//!          v            v           v          v
//! +-------------+ +----------+ +---------+ +--------+
//! | SystemCatalog| | Config   | | Audit   | |Migration|
//! | Key-value   | | Store    | | Logger  | |Manager  |
//! | metadata    | | Config   | | Events  | | Schema  |
//! | (catalog)   | | snapshots| | (audit) | | version |
//! +-------------+ +----------+ +---------+ +--------+
//! ```
//!
//! ## Initialization Flow
//!
//! ```text
//! PrimusDB::new()
//!   |
//!   +-> SystemDatabase::open(data_dir)
//!   |     +-> sled::open(path)
//!   |     +-> MigrationManager::new(db)
//!   |     +-> SystemCatalog::new(db)
//!   |     +-> ConfigStore::new(db)
//!   |     +-> AuditLogger::new(db)
//!   |
//!   +-> SystemDatabase::init()
//!         +-> MigrationManager::run_pending()
//!         +-> SystemCatalog::init()
//!         +-> ConfigStore::init()
//!         +-> AuditLogger::init()
//! ```
//!
//! ## Config Precedence
//!
//! Values are tracked by `ConfigSource` for precedence resolution:
//! `Default < ConfigFile < EnvVar < SystemDb < RuntimeOverride < TuiProfile`

use std::sync::Arc;

pub mod audit;
pub mod catalog;
pub mod config_store;
pub mod migrations;

pub const SYSTEM_DB_NAME: &str = "primus_system";

/// Directory name of the system database under the configured data directory.
pub const SYSTEM_DB_DIR: &str = "system";

/// Current schema version of the system database. Pending migrations are
/// applied when the stored version is lower than this constant.
pub const SYSTEM_SCHEMA_VERSION: u64 = 1;

/// In-memory handle to the sled-backed system database.
///
/// Owns the four persisted subsystems and hands them out as public fields so
/// they can be used independently after [`SystemDatabase::init`].
pub struct SystemDatabase {
    db: Arc<sled::Db>,
    pub migrations: migrations::MigrationManager,
    pub catalog: catalog::SystemCatalog,
    pub config: config_store::ConfigStore,
    pub audit: audit::AuditLogger,
}

/// Runtime identity metadata for this server instance, persisted as the
/// `server.info` key of the system database.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerInfo {
    /// Unique server identifier.
    pub server_id: String,
    /// PrimusDB crate version at startup.
    pub version: String,
    /// Identifier of the node in cluster mode.
    pub node_id: String,
    /// Whether the server joined a cluster.
    pub cluster_mode: bool,
    /// Wall-clock time of server start.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Storage engines the server was started with.
    pub engine_types: Vec<String>,
}

impl SystemDatabase {
    /// Opens (creating if needed) the system database directory under
    /// `{data_dir}/system` and constructs the four subsystems.
    pub fn open(data_dir: &str) -> crate::Result<Self> {
        let path = format!("{}/{}", data_dir, SYSTEM_DB_DIR);
        std::fs::create_dir_all(&path).map_err(|e| {
            crate::Error::ConfigurationError(format!(
                "Failed to create system database directory: {}",
                e
            ))
        })?;
        let db = sled::open(&path).map_err(|e| {
            crate::Error::ConfigurationError(format!("Failed to open system database: {}", e))
        })?;
        let db = Arc::new(db);
        Ok(Self {
            db: db.clone(),
            migrations: migrations::MigrationManager::new(db.clone()),
            catalog: catalog::SystemCatalog::new(db.clone()),
            config: config_store::ConfigStore::new(db.clone()),
            audit: audit::AuditLogger::new(db.clone()),
        })
    }

    /// Applies pending migrations, then initialises the catalog, config store
    /// and audit logger. Safe to call repeatedly (idempotent).
    pub fn init(&self) -> crate::Result<()> {
        self.migrations.run_pending()?;
        self.catalog.init()?;
        self.config.init()?;
        self.audit.init()?;
        Ok(())
    }

    /// Returns a reference to the underlying sled database.
    pub fn db(&self) -> &sled::Db {
        &self.db
    }

    /// Persists the given [`ServerInfo`] snapshot under the `server.info` key.
    pub fn set_server_info(&self, info: &ServerInfo) -> crate::Result<()> {
        let json = serde_json::to_vec(info)
            .map_err(|e| crate::Error::ConfigurationError(format!("Serialization error: {}", e)))?;
        self.db.insert("server.info", json).map_err(|e| {
            crate::Error::ConfigurationError(format!("Failed to save server info: {}", e))
        })?;
        if let Err(e) = self.db.flush() {
            tracing::warn!("Failed to flush server info to disk: {}", e);
        }
        Ok(())
    }

    /// Loads the persisted [`ServerInfo`] snapshot, or `None` if not set.
    pub fn server_info(&self) -> crate::Result<Option<ServerInfo>> {
        match self.db.get("server.info") {
            Ok(Some(ivec)) => {
                let info: ServerInfo = serde_json::from_slice(&ivec).map_err(|e| {
                    crate::Error::ConfigurationError(format!("Deserialization error: {}", e))
                })?;
                Ok(Some(info))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(crate::Error::ConfigurationError(format!(
                "Failed to read server info: {}",
                e
            ))),
        }
    }

    /// Save the server's runtime configuration (PrimusDBConfig) snapshot
    /// to the system database for export/backup integration.
    pub fn set_runtime_config(&self, config: &crate::PrimusDBConfig) -> crate::Result<()> {
        let json = serde_json::to_vec(config).map_err(|e| {
            crate::Error::ConfigurationError(format!("Runtime config serialization error: {}", e))
        })?;
        self.db.insert("runtime.config", json).map_err(|e| {
            crate::Error::ConfigurationError(format!("Failed to save runtime config: {}", e))
        })?;
        if let Err(e) = self.db.flush() {
            tracing::warn!("Failed to flush runtime config to disk: {}", e);
        }
        Ok(())
    }

    /// Load the persisted runtime configuration snapshot.
    pub fn get_runtime_config(&self) -> crate::Result<Option<crate::PrimusDBConfig>> {
        match self.db.get("runtime.config") {
            Ok(Some(ivec)) => {
                let config: crate::PrimusDBConfig = serde_json::from_slice(&ivec).map_err(|e| {
                    crate::Error::ConfigurationError(format!(
                        "Runtime config deserialization error: {}",
                        e
                    ))
                })?;
                Ok(Some(config))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(crate::Error::ConfigurationError(format!(
                "Failed to read runtime config: {}",
                e
            ))),
        }
    }

    /// Export all system database data as a JSON bundle for backup integration.
    /// Returns a serialised JSON value containing config entries, catalog entries,
    /// and audit events.
    pub fn export_system_bundle(&self) -> crate::Result<serde_json::Value> {
        let config_entries = self.config.list_all()?;
        let catalog_entries = self.catalog.list_all()?;
        let audit_events = self.audit.recent(1000)?;
        let server_info = self.server_info()?;

        Ok(serde_json::json!({
            "format_version": 1,
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "server_info": server_info,
            "config_entries": config_entries,
            "catalog_entries": catalog_entries,
            "audit_events": audit_events,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::audit::AuditLogger;
    use crate::system::catalog::SystemCatalog;
    use crate::system::config_store::{ConfigBundle, ConfigEntry, ConfigSource};
    use crate::system::migrations::MigrationManager;
    use sled::Db;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn with_system_db<F>(f: F)
    where
        F: FnOnce(SystemDatabase),
    {
        let dir = tempdir().unwrap();
        let sys_db = SystemDatabase::open(dir.path().to_str().unwrap()).unwrap();
        sys_db.init().unwrap();
        f(sys_db);
    }

    fn with_db<F>(name: &str, f: F)
    where
        F: FnOnce(Arc<Db>),
    {
        let dir = tempdir().unwrap();
        let path = format!("{}/{}", dir.path().to_str().unwrap(), name);
        std::fs::create_dir_all(&path).unwrap();
        let db = Arc::new(sled::open(&path).unwrap());
        f(db);
    }

    #[test]
    fn test_system_database_open() {
        with_system_db(|sys_db| {
            assert!(sys_db.db().len() >= 0);
        });
    }

    #[test]
    fn test_system_database_init_idempotent() {
        let dir = tempdir().unwrap();
        let sys_db = SystemDatabase::open(dir.path().to_str().unwrap()).unwrap();
        sys_db.init().unwrap();
        let v1 = sys_db.migrations.current_version().unwrap();
        sys_db.init().unwrap();
        let v2 = sys_db.migrations.current_version().unwrap();
        assert_eq!(v1, v2, "init must be idempotent");
    }

    #[test]
    fn test_server_info_roundtrip() {
        with_system_db(|sys_db| {
            let info = ServerInfo {
                server_id: "test-id".into(),
                version: "1.3.2-alpha".into(),
                node_id: "node-1".into(),
                cluster_mode: false,
                started_at: chrono::Utc::now(),
                engine_types: vec!["columnar".into(), "vector".into()],
            };
            sys_db.set_server_info(&info).unwrap();
            let loaded = sys_db.server_info().unwrap().unwrap();
            assert_eq!(loaded.server_id, "test-id");
            assert_eq!(loaded.version, "1.3.2-alpha");
            assert_eq!(loaded.node_id, "node-1");
        });
    }

    #[test]
    fn test_migration_initial_version() {
        with_db("mig_test", |db| {
            let mgr = MigrationManager::new(db);
            assert_eq!(mgr.current_version().unwrap(), 0);
        });
    }

    #[test]
    fn test_migration_run_pending() {
        with_db("mig_run", |db| {
            let mgr = MigrationManager::new(db);
            assert!(!mgr.is_migrated().unwrap());
            mgr.run_pending().unwrap();
            assert!(mgr.is_migrated().unwrap());
            assert_eq!(mgr.current_version().unwrap(), 1);
        });
    }

    #[test]
    fn test_migration_idempotent() {
        with_db("mig_idem", |db| {
            let mgr = MigrationManager::new(db);
            mgr.run_pending().unwrap();
            let v1 = mgr.current_version().unwrap();
            mgr.run_pending().unwrap();
            let v2 = mgr.current_version().unwrap();
            assert_eq!(v1, v2);
            assert_eq!(v1, 1);
        });
    }

    #[test]
    fn test_migration_applied_records() {
        with_db("mig_records", |db| {
            let mgr = MigrationManager::new(db);
            mgr.run_pending().unwrap();
            let records = mgr.applied_migrations().unwrap();
            assert!(!records.is_empty());
            assert_eq!(records[0].version, 1);
            assert_eq!(records[0].name, "initial_schema");
        });
    }

    #[test]
    fn test_catalog_init() {
        with_db("cat_init", |db| {
            let catalog = SystemCatalog::new(db);
            catalog.init().unwrap();
            let entry = catalog.get("server.version").unwrap().unwrap();
            assert!(!entry.value.as_str().unwrap_or("").is_empty());
        });
    }

    #[test]
    fn test_catalog_set_and_get() {
        with_db("cat_setget", |db| {
            let catalog = SystemCatalog::new(db);
            catalog.init().unwrap();
            catalog
                .set("test.key", serde_json::json!("test_value"), "testing")
                .unwrap();
            let entry = catalog.get("test.key").unwrap().unwrap();
            assert_eq!(entry.value, serde_json::json!("test_value"));
            assert_eq!(entry.category, "testing");
        });
    }

    #[test]
    fn test_catalog_list_by_category() {
        with_db("cat_list", |db| {
            let catalog = SystemCatalog::new(db);
            catalog.init().unwrap();
            catalog.set("cat.a", serde_json::json!(1), "alpha").unwrap();
            catalog.set("cat.b", serde_json::json!(2), "alpha").unwrap();
            catalog.set("cat.c", serde_json::json!(3), "beta").unwrap();
            let alpha = catalog.list_by_category("alpha").unwrap();
            assert_eq!(alpha.len(), 2);
            let beta = catalog.list_by_category("beta").unwrap();
            assert_eq!(beta.len(), 1);
        });
    }

    #[test]
    fn test_catalog_delete() {
        with_db("cat_del", |db| {
            let catalog = SystemCatalog::new(db);
            catalog.init().unwrap();
            catalog
                .set("temp.key", serde_json::json!("temp"), "testing")
                .unwrap();
            assert!(catalog.get("temp.key").unwrap().is_some());
            catalog.delete("temp.key").unwrap();
            assert!(catalog.get("temp.key").unwrap().is_none());
        });
    }

    #[test]
    fn test_catalog_to_map() {
        with_db("cat_map", |db| {
            let catalog = SystemCatalog::new(db);
            catalog.init().unwrap();
            let map = catalog.to_map().unwrap();
            assert!(map.contains_key("server.version"));
        });
    }

    #[test]
    fn test_config_store_init() {
        with_db("cfg_init", |db| {
            let store = config_store::ConfigStore::new(db);
            store.init().unwrap();
            assert_eq!(store.count().unwrap(), 1);
        });
    }

    #[test]
    fn test_config_store_set_get() {
        with_db("cfg_setget", |db| {
            let store = config_store::ConfigStore::new(db);
            store
                .set(
                    "test.key",
                    serde_json::json!("hello"),
                    ConfigSource::TuiProfile,
                )
                .unwrap();
            let entry = store.get("test.key").unwrap().unwrap();
            assert_eq!(entry.value, serde_json::json!("hello"));
            assert_eq!(entry.source, ConfigSource::TuiProfile);
        });
    }

    #[test]
    fn test_config_store_delete() {
        with_db("cfg_del", |db| {
            let store = config_store::ConfigStore::new(db);
            store
                .set("tmp", serde_json::json!(1), ConfigSource::Default)
                .unwrap();
            assert!(store.get("tmp").unwrap().is_some());
            store.delete("tmp").unwrap();
            assert!(store.get("tmp").unwrap().is_none());
        });
    }

    #[test]
    fn test_config_store_export_import_bundle() {
        with_db("cfg_bundle", |db| {
            let store = config_store::ConfigStore::new(db);
            store
                .set("key1", serde_json::json!("val1"), ConfigSource::ConfigFile)
                .unwrap();
            store
                .set(
                    "key2",
                    serde_json::json!(42),
                    ConfigSource::EnvironmentVariable,
                )
                .unwrap();

            let bundle = store.export_bundle().unwrap();
            assert_eq!(bundle.entries.len(), 2);
            assert_eq!(bundle.format_version, 1);

            let dir2 = tempdir().unwrap();
            let db2 = Arc::new(sled::open(dir2.path().join("import_test")).unwrap());
            let store2 = config_store::ConfigStore::new(db2);
            store2.init().unwrap();
            let count = store2.import_bundle(&bundle).unwrap();
            assert_eq!(count, 2);

            let e1 = store2.get("key1").unwrap().unwrap();
            assert_eq!(e1.value, serde_json::json!("val1"));
        });
    }

    #[test]
    fn test_config_store_validate() {
        with_db("cfg_val", |db| {
            let store = config_store::ConfigStore::new(db);
            assert!(store
                .validate("valid.key", &serde_json::json!("ok"))
                .is_ok());
            assert!(store.validate("", &serde_json::json!("x")).is_err());
            assert!(store.validate("key", &serde_json::Value::Null).is_err());
            assert!(store
                .validate("key with spaces", &serde_json::json!("x"))
                .is_err());
        });
    }

    #[test]
    fn test_config_snapshot_create_and_restore() {
        with_db("cfg_snap", |db| {
            let store = config_store::ConfigStore::new(db);
            store
                .set("k1", serde_json::json!("v1"), ConfigSource::Default)
                .unwrap();
            let snap_id = store
                .create_snapshot("test-snap", "A test snapshot")
                .unwrap();

            store
                .set("k1", serde_json::json!("v2"), ConfigSource::Default)
                .unwrap();
            assert_eq!(
                store.get("k1").unwrap().unwrap().value,
                serde_json::json!("v2")
            );

            store.restore_snapshot(&snap_id).unwrap();
            assert_eq!(
                store.get("k1").unwrap().unwrap().value,
                serde_json::json!("v1")
            );
        });
    }

    #[test]
    fn test_config_snapshot_list() {
        with_db("cfg_snaplist", |db| {
            let store = config_store::ConfigStore::new(db);
            store.create_snapshot("s1", "first").unwrap();
            store.create_snapshot("s2", "second").unwrap();
            let snapshots = store.list_snapshots().unwrap();
            assert_eq!(snapshots.len(), 2);
        });
    }

    #[test]
    fn test_config_snapshot_delete() {
        with_db("cfg_snapdel", |db| {
            let store = config_store::ConfigStore::new(db);
            let id = store.create_snapshot("del-me", "to delete").unwrap();
            store.delete_snapshot(&id).unwrap();
            assert!(store.get_snapshot(&id).unwrap().is_none());
        });
    }

    #[test]
    fn test_config_store_list_all() {
        with_db("cfg_listall", |db| {
            let store = config_store::ConfigStore::new(db);
            store
                .set("a", serde_json::json!(1), ConfigSource::Default)
                .unwrap();
            store
                .set("b", serde_json::json!(2), ConfigSource::ConfigFile)
                .unwrap();
            let entries = store.list_all().unwrap();
            assert_eq!(entries.len(), 2);
        });
    }

    #[test]
    fn test_config_source_display() {
        assert_eq!(format!("{}", ConfigSource::Default), "default");
        assert_eq!(format!("{}", ConfigSource::ConfigFile), "config file");
        assert_eq!(format!("{}", ConfigSource::EnvironmentVariable), "env var");
        assert_eq!(
            format!("{}", ConfigSource::SystemDatabase),
            "system database"
        );
        assert_eq!(
            format!("{}", ConfigSource::RuntimeOverride),
            "runtime override"
        );
        assert_eq!(format!("{}", ConfigSource::TuiProfile), "TUI profile");
    }

    #[test]
    fn test_config_entry_serde_roundtrip() {
        let entry = ConfigEntry {
            key: "server.port".into(),
            value: serde_json::json!(8080),
            source: ConfigSource::ConfigFile,
            updated_at: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ConfigEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.key, "server.port");
        assert_eq!(deserialized.value, serde_json::json!(8080));
    }

    #[test]
    fn test_config_bundle_serde_roundtrip() {
        let bundle = ConfigBundle {
            format_version: 1,
            exported_at: chrono::Utc::now(),
            entries: vec![ConfigEntry {
                key: "k".into(),
                value: serde_json::json!("v"),
                source: ConfigSource::Default,
                updated_at: chrono::Utc::now(),
            }],
        };
        let json = serde_json::to_string_pretty(&bundle).unwrap();
        let deserialized: ConfigBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.entries.len(), 1);
    }

    #[test]
    fn test_audit_log_event() {
        with_db("audit_log", |db| {
            let logger = AuditLogger::new(db);
            logger.init().unwrap();
            let event = logger
                .log(
                    "config.change",
                    "admin",
                    "server.port",
                    "update",
                    serde_json::json!({"from": 8080, "to": 9090}),
                    true,
                )
                .unwrap();
            assert_eq!(event.event_type, "config.change");
            assert_eq!(event.actor, "admin");
            assert_eq!(event.resource, "server.port");
            assert!(event.success);
        });
    }

    #[test]
    fn test_audit_recent_events() {
        with_db("audit_recent", |db| {
            let logger = AuditLogger::new(db);
            logger.init().unwrap();
            for i in 0..5 {
                logger
                    .log(
                        "test",
                        "user",
                        &format!("resource_{}", i),
                        "read",
                        serde_json::json!({"i": i}),
                        true,
                    )
                    .unwrap();
            }
            let recent = logger.recent(3).unwrap();
            assert_eq!(recent.len(), 3);
        });
    }

    #[test]
    fn test_audit_by_type() {
        with_db("audit_type", |db| {
            let logger = AuditLogger::new(db);
            logger.init().unwrap();
            logger
                .log("type_a", "u", "r1", "read", serde_json::json!({}), true)
                .unwrap();
            logger
                .log("type_b", "u", "r2", "read", serde_json::json!({}), true)
                .unwrap();
            logger
                .log("type_a", "u", "r3", "read", serde_json::json!({}), true)
                .unwrap();
            let type_a = logger.by_type("type_a", 10).unwrap();
            assert_eq!(type_a.len(), 2);
        });
    }

    #[test]
    fn test_audit_count() {
        with_db("audit_count", |db| {
            let logger = AuditLogger::new(db);
            logger.init().unwrap();
            assert_eq!(logger.count().unwrap(), 1);
            logger
                .log("ev", "u", "r", "act", serde_json::json!({}), true)
                .unwrap();
            assert_eq!(logger.count().unwrap(), 2);
        });
    }

    #[test]
    fn test_audit_max_events() {
        with_db("audit_max", |db| {
            let logger = AuditLogger::new(db);
            logger.init().unwrap();
            let max = crate::system::audit::MAX_AUDIT_EVENTS;
            for i in 0..(max + 10) {
                logger
                    .log(
                        "test",
                        "user",
                        &format!("r{}", i),
                        "act",
                        serde_json::json!({}),
                        true,
                    )
                    .unwrap();
            }
            let count = logger.count().unwrap();
            assert!(count <= max, "count {} should be <= max {}", count, max);
        });
    }
}
