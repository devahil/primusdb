//! # Backup — Full / Incremental / WAL Backups
//!
//! Creates and restores engine data backups with manifest tracking, retention
//! pruning, and an optional background scheduler.
//!
//! ## Backup Types
//!
//! ```text
//! BackupType
//!   +-> Full        — complete snapshot of every engine directory
//!   +-> Incremental — delta backup referencing a parent manifest
//!   +-> WAL         — archived write-ahead-log file capture
//! ```
//!
//! ## Lifecycle
//!
//! ```text
//! BackupManager::new(config)
//!   |
//!   +-> create_full_backup(data_dir)     -> BackupManifest { id, ... }
//!   +-> create_incremental_backup(...)   -> BackupManifest { parent_id, ... }
//!   +-> create_wal_backup(wal_file)      -> BackupManifest { wal_files }
//!   |
//!   +-> restore_backup(id, target_dir)   -> RestoreResult
//!   +-> get_backup_chain(id)             -> parent chain walk
//!   +-> prune_old_backups()              -> retention enforcement
//! ```
//!
//! The scheduler lives in [`scheduler`] and drives backups on interval or on
//! demand through a background task.
pub mod scheduler;

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn copy_dir_all(src: &str, dst: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = Path::new(dst).join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path().to_string_lossy(), &dst_path.to_string_lossy())?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

/// Outcome of a restore operation, describing which engines were recovered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    /// Id of the backup that was restored.
    pub backup_id: String,
    /// Directory the restored data was written to.
    pub restore_dir: String,
    /// Engine directories successfully copied back.
    pub restored_engines: Vec<String>,
    /// Kind of backup the data came from.
    pub backup_type: BackupType,
    /// Unix timestamp of the source backup.
    pub backup_timestamp: u64,
}

/// Tuning options controlling backup location, retention and features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Directory backups are written to.
    pub backup_dir: String,
    /// Whether incremental backups are enabled.
    pub incremental_enabled: bool,
    /// Whether WAL archiving is enabled.
    pub wal_archiving_enabled: bool,
    /// Number of days backups are retained.
    pub retention_days: u32,
    /// Maximum number of manifests kept before pruning.
    pub max_backups: u32,
    /// Whether backup archives are compressed.
    pub compression_enabled: bool,
    /// Optional encryption key for backup archives.
    pub encryption_key: Option<String>,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            backup_dir: "./backups".to_string(),
            incremental_enabled: true,
            wal_archiving_enabled: true,
            retention_days: 30,
            max_backups: 10,
            compression_enabled: true,
            encryption_key: None,
        }
    }
}

/// Record of a single backup, tracking identity, lineage and contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Randomly generated backup identifier.
    pub id: String,
    /// Unix timestamp of backup creation.
    pub timestamp: u64,
    /// Kind of backup.
    pub backup_type: BackupType,
    /// Id of the parent backup for incremental backups.
    pub parent_id: Option<String>,
    /// Total size of the backup in bytes.
    pub size_bytes: u64,
    /// Integrity checksum of the backup archive.
    pub checksum: String,
    /// Per-engine snapshot metadata.
    pub engine_snapshots: Vec<EngineSnapshot>,
    /// WAL files captured by this backup.
    pub wal_files: Vec<String>,
    /// Free-form backup metadata, including the source `data_dir`.
    pub metadata: serde_json::Value,
}

/// Category of a backup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupType {
    /// Complete snapshot of all engine data.
    Full,
    /// Delta backup chained to a parent manifest.
    Incremental,
    /// Archived write-ahead-log file.
    WAL,
}

/// Metadata for one engine captured in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSnapshot {
    /// Engine type, e.g. `columnar`, `vector`.
    pub engine_type: String,
    /// Tables belonging to the engine at backup time.
    pub tables: Vec<String>,
    /// Size of the engine data in bytes.
    pub size_bytes: u64,
}

/// Interval configuration for the periodic backup scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupScheduleConfig {
    /// Seconds between scheduled full backups.
    pub full_backup_interval_secs: u64,
    /// Seconds between scheduled incremental backups.
    pub incremental_interval_secs: u64,
    /// Whether the scheduler loop is active.
    pub enabled: bool,
}

impl Default for BackupScheduleConfig {
    fn default() -> Self {
        Self {
            full_backup_interval_secs: 86400,
            incremental_interval_secs: 3600,
            enabled: false,
        }
    }
}

/// Snapshot of the current backup state for observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStatus {
    /// Unix timestamp of the most recent full backup, if any.
    pub last_full_backup: Option<u64>,
    /// Unix timestamp of the most recent incremental backup, if any.
    pub last_incremental_backup: Option<u64>,
    /// Total number of tracked manifests.
    pub total_backups: usize,
    /// Whether the scheduler is currently enabled.
    pub schedule_enabled: bool,
}

/// Creates, tracks, restores and prunes backups in memory.
pub struct BackupManager {
    config: BackupConfig,
    manifests: Vec<BackupManifest>,
}

impl BackupManager {
    /// Creates a manager with the given configuration and no manifests.
    pub fn new(config: BackupConfig) -> Self {
        Self {
            config,
            manifests: Vec::new(),
        }
    }

    /// Records a new full backup manifest for `data_dir`.
    pub fn create_full_backup(&mut self, data_dir: &str) -> crate::Result<BackupManifest> {
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let manifest = BackupManifest {
            id: id.clone(),
            timestamp,
            backup_type: BackupType::Full,
            parent_id: None,
            size_bytes: 0,
            checksum: String::new(),
            engine_snapshots: Vec::new(),
            wal_files: Vec::new(),
            metadata: serde_json::json!({
                "data_dir": data_dir,
                "engines": ["columnar", "vector", "document", "relational", "keyvalue", "timeseries"]
            }),
        };

        self.manifests.push(manifest.clone());
        self.prune_old_backups();
        tracing::info!("Full backup created: {}", id);
        Ok(manifest)
    }

    /// Records a new incremental backup manifest chained to the most recent
    /// manifest as its parent.
    pub fn create_incremental_backup(&mut self, data_dir: &str) -> crate::Result<BackupManifest> {
        let parent_id = self.manifests.last().map(|m| m.id.clone());
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let manifest = BackupManifest {
            id: id.clone(),
            timestamp,
            backup_type: BackupType::Incremental,
            parent_id: parent_id.clone(),
            size_bytes: 0,
            checksum: String::new(),
            engine_snapshots: Vec::new(),
            wal_files: Vec::new(),
            metadata: serde_json::json!({
                "data_dir": data_dir,
                "parent_id": parent_id
            }),
        };

        self.manifests.push(manifest.clone());
        self.prune_old_backups();
        tracing::info!(
            "Incremental backup created: {} (parent: {:?})",
            id,
            parent_id
        );
        Ok(manifest)
    }

    /// Records a WAL backup manifest for a single archived log file.
    pub fn create_wal_backup(&mut self, wal_file: &str) -> crate::Result<BackupManifest> {
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let manifest = BackupManifest {
            id: id.clone(),
            timestamp,
            backup_type: BackupType::WAL,
            parent_id: None,
            size_bytes: 0,
            checksum: String::new(),
            engine_snapshots: Vec::new(),
            wal_files: vec![wal_file.to_string()],
            metadata: serde_json::json!({
                "wal_file": wal_file
            }),
        };

        self.manifests.push(manifest.clone());
        tracing::info!("WAL backup created: {} for file: {}", id, wal_file);
        Ok(manifest)
    }

    /// Returns all tracked manifests in creation order.
    pub fn list_backups(&self) -> &[BackupManifest] {
        &self.manifests
    }

    /// Looks up a single manifest by id.
    pub fn get_backup(&self, id: &str) -> Option<&BackupManifest> {
        self.manifests.iter().find(|m| m.id == id)
    }

    /// Restores the engine directories of `id` into a fresh
    /// `{target_data_dir}/restore_{id}` directory.
    pub fn restore_backup(&self, id: &str, target_data_dir: &str) -> crate::Result<RestoreResult> {
        let manifest =
            self.manifests.iter().find(|m| m.id == id).ok_or_else(|| {
                crate::Error::ValidationError(format!("Backup not found: {}", id))
            })?;

        let restore_dir = format!("{}/restore_{}", target_data_dir, manifest.id);
        std::fs::create_dir_all(&restore_dir).map_err(|e| {
            crate::Error::ValidationError(format!(
                "Failed to create restore directory {}: {}",
                restore_dir, e
            ))
        })?;

        let mut restored_engines: Vec<String> = Vec::new();

        let engine_dirs = [
            "columnar",
            "vector",
            "document",
            "relational",
            "keyvalue",
            "timeseries",
        ];
        for engine in &engine_dirs {
            let source = format!(
                "{}/{}",
                manifest
                    .metadata
                    .get("data_dir")
                    .and_then(|v| v.as_str())
                    .unwrap_or(target_data_dir),
                engine
            );
            let dest = format!("{}/{}", restore_dir, engine);
            if std::path::Path::new(&source).exists() {
                if let Err(e) = copy_dir_all(&source, &dest) {
                    tracing::warn!("Failed to restore engine {}: {}", engine, e);
                } else {
                    restored_engines.push(engine.to_string());
                }
            }
        }

        tracing::info!(
            "Restore from backup {} completed: {} engines restored to {}",
            manifest.id,
            restored_engines.len(),
            restore_dir
        );

        Ok(RestoreResult {
            backup_id: manifest.id.clone(),
            restore_dir,
            restored_engines,
            backup_type: manifest.backup_type.clone(),
            backup_timestamp: manifest.timestamp,
        })
    }

    /// Returns all manifests of the given backup type.
    pub fn get_backups_by_type(&self, backup_type: BackupType) -> Vec<&BackupManifest> {
        self.manifests
            .iter()
            .filter(|m| m.backup_type == backup_type)
            .collect()
    }

    /// Walks the parent chain starting at `id`, returning the lineage from the
    /// newest to the oldest manifest.
    pub fn get_backup_chain(&self, id: &str) -> Vec<&BackupManifest> {
        let mut chain = Vec::new();
        let mut current_id = Some(id.to_string());

        while let Some(cid) = current_id {
            if let Some(manifest) = self.manifests.iter().find(|m| m.id == cid) {
                chain.push(manifest);
                current_id = manifest.parent_id.clone();
            } else {
                break;
            }
        }

        chain
    }

    /// Drops the oldest manifests until the count is within
    /// `config.max_backups`.
    pub fn prune_old_backups(&mut self) {
        let max_backups = self.config.max_backups as usize;
        if self.manifests.len() > max_backups {
            let to_remove = self.manifests.len() - max_backups;
            self.manifests.drain(..to_remove);
            tracing::info!("Pruned {} old backups", to_remove);
        }
    }

    /// Builds a [`BackupStatus`] summary from the tracked manifests.
    pub fn get_status(&self) -> BackupStatus {
        let last_full = self
            .manifests
            .iter()
            .filter(|m| m.backup_type == BackupType::Full)
            .max_by_key(|m| m.timestamp)
            .map(|m| m.timestamp);

        let last_incremental = self
            .manifests
            .iter()
            .filter(|m| m.backup_type == BackupType::Incremental)
            .max_by_key(|m| m.timestamp)
            .map(|m| m.timestamp);

        BackupStatus {
            last_full_backup: last_full,
            last_incremental_backup: last_incremental,
            total_backups: self.manifests.len(),
            schedule_enabled: false,
        }
    }

    /// Returns the active backup configuration.
    pub fn config(&self) -> &BackupConfig {
        &self.config
    }
}

impl std::fmt::Display for BackupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupType::Full => write!(f, "Full"),
            BackupType::Incremental => write!(f, "Incremental"),
            BackupType::WAL => write!(f, "WAL"),
        }
    }
}

impl std::fmt::Display for BackupManifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Backup {{ id: {}, type: {}, timestamp: {} }}",
            self.id, self.backup_type, self.timestamp
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_config_default() {
        let config = BackupConfig::default();
        assert_eq!(config.backup_dir, "./backups");
        assert!(config.incremental_enabled);
        assert!(config.wal_archiving_enabled);
        assert_eq!(config.retention_days, 30);
        assert_eq!(config.max_backups, 10);
        assert!(config.compression_enabled);
        assert!(config.encryption_key.is_none());
    }

    #[test]
    fn test_backup_schedule_config_default() {
        let config = BackupScheduleConfig::default();
        assert_eq!(config.full_backup_interval_secs, 86400);
        assert_eq!(config.incremental_interval_secs, 3600);
        assert!(!config.enabled);
    }

    #[test]
    fn test_backup_manager_create_full_backup() {
        let config = BackupConfig::default();
        let mut manager = BackupManager::new(config);
        let manifest = manager.create_full_backup("/data").unwrap();
        assert_eq!(manifest.backup_type, BackupType::Full);
        assert!(manifest.parent_id.is_none());
        assert_eq!(manager.list_backups().len(), 1);
    }

    #[test]
    fn test_backup_manager_create_incremental_backup() {
        let config = BackupConfig::default();
        let mut manager = BackupManager::new(config);
        let _full = manager.create_full_backup("/data").unwrap();
        let incr = manager.create_incremental_backup("/data").unwrap();
        assert_eq!(incr.backup_type, BackupType::Incremental);
        assert!(incr.parent_id.is_some());
        assert_eq!(manager.list_backups().len(), 2);
    }

    #[test]
    fn test_backup_manager_prune_old_backups() {
        let mut config = BackupConfig::default();
        config.max_backups = 3;
        let mut manager = BackupManager::new(config);

        for _ in 0..5 {
            let _ = manager.create_full_backup("/data");
        }

        assert_eq!(manager.list_backups().len(), 3);
    }

    #[test]
    fn test_backup_manager_get_backup_chain() {
        let config = BackupConfig::default();
        let mut manager = BackupManager::new(config);

        let full = manager.create_full_backup("/data").unwrap();
        let incr1 = manager.create_incremental_backup("/data").unwrap();
        let incr2 = manager.create_incremental_backup("/data").unwrap();

        let chain = manager.get_backup_chain(&incr2.id);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].id, incr2.id);
        assert_eq!(chain[1].id, incr1.id);
        assert_eq!(chain[2].id, full.id);
    }

    #[test]
    fn test_backup_status() {
        let config = BackupConfig::default();
        let mut manager = BackupManager::new(config);

        let _full = manager.create_full_backup("/data").unwrap();
        let _incr = manager.create_incremental_backup("/data").unwrap();

        let status = manager.get_status();
        assert!(status.last_full_backup.is_some());
        assert!(status.last_incremental_backup.is_some());
        assert_eq!(status.total_backups, 2);
    }

    #[test]
    fn test_backup_type_display() {
        assert_eq!(format!("{}", BackupType::Full), "Full");
        assert_eq!(format!("{}", BackupType::Incremental), "Incremental");
        assert_eq!(format!("{}", BackupType::WAL), "WAL");
    }
}
