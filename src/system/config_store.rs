//! # ConfigStore — Persistent Configuration Store
//!
//! Manages key-value configuration entries with source tracking and
//! full snapshot / export / import support.
//!
//! ## Architecture
//!
//! ```text
//! ConfigStore
//!   +-> sys_config tree (sled)
//!   |     ConfigEntry { key, value, source, updated_at }
//!   |
//!   +-> sys_config_snapshots tree (sled)
//!         ConfigSnapshot { id, name, entries[], created_at, description }
//! ```
//!
//! ## Operations
//!
//! - `set(key, value, source)` — upsert with precedence source
//! - `get(key)` — read single entry
//! - `list_all()` — enumerate all entries
//! - `delete(key)` — remove entry
//! - `export_bundle()` / `import_bundle()` — JSON round-trip
//! - `create_snapshot()` / `restore_snapshot()` — point-in-time config recovery
//! - `validate(key, value)` — key/value validation rules

use serde::{Deserialize, Serialize};
use sled::Db;
use std::sync::Arc;

const CONFIG_TREE: &str = "sys_config";
const SNAPSHOT_TREE: &str = "sys_config_snapshots";

/// Source of a configuration value, used for precedence tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigSource {
    /// Built-in default value.
    Default,
    /// Value loaded from a config file.
    ConfigFile,
    /// Value loaded from an environment variable.
    EnvironmentVariable,
    /// Value persisted in the system database.
    SystemDatabase,
    /// Value set at runtime (API/CLI override).
    RuntimeOverride,
    /// Value applied from an active TUI profile.
    TuiProfile,
}

impl std::fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigSource::Default => write!(f, "default"),
            ConfigSource::ConfigFile => write!(f, "config file"),
            ConfigSource::EnvironmentVariable => write!(f, "env var"),
            ConfigSource::SystemDatabase => write!(f, "system database"),
            ConfigSource::RuntimeOverride => write!(f, "runtime override"),
            ConfigSource::TuiProfile => write!(f, "TUI profile"),
        }
    }
}

/// A single configuration entry with its provenance source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    /// Configuration key.
    pub key: String,
    /// Configuration value.
    pub value: serde_json::Value,
    /// Precedence source this value originated from.
    pub source: ConfigSource,
    /// Time of the last update.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A point-in-time capture of the entire configuration set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    /// Randomly generated snapshot identifier.
    pub id: String,
    /// Human-readable snapshot name.
    pub name: String,
    /// Entries captured at snapshot time.
    pub entries: Vec<ConfigEntry>,
    /// Wall-clock time the snapshot was taken.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Free-form snapshot description.
    pub description: String,
}

/// Persistent configuration store backed by the `sys_config` tree, with
/// source tracking, snapshots, and JSON export/import bundles.
pub struct ConfigStore {
    db: Arc<Db>,
}

fn serialize_json<T: Serialize>(value: &T) -> crate::Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| crate::Error::ConfigurationError(e.to_string()))
}

fn deserialize_json<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> crate::Result<T> {
    serde_json::from_slice(bytes).map_err(|e| crate::Error::ConfigurationError(e.to_string()))
}

impl ConfigStore {
    /// Creates a config store backed by the given sled database.
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    /// Marks the config tree as initialised on first run.
    pub fn init(&self) -> crate::Result<()> {
        let tree = self.db.open_tree(CONFIG_TREE)?;
        if tree.get("initialized")?.is_none() {
            tree.insert("initialized", serialize_json(&true)?)?;
            tree.flush()?;
        }
        Ok(())
    }

    /// Reads a single config entry by key, or `None` if absent.
    pub fn get(&self, key: &str) -> crate::Result<Option<ConfigEntry>> {
        let tree = self.db.open_tree(CONFIG_TREE)?;
        if let Some(bytes) = tree.get(key.as_bytes())? {
            Ok(Some(deserialize_json(&bytes)?))
        } else {
            Ok(None)
        }
    }

    /// Upserts a config entry under `key`, tagging it with its source.
    pub fn set(
        &self,
        key: &str,
        value: serde_json::Value,
        source: ConfigSource,
    ) -> crate::Result<()> {
        let tree = self.db.open_tree(CONFIG_TREE)?;
        let entry = ConfigEntry {
            key: key.to_string(),
            value,
            source,
            updated_at: chrono::Utc::now(),
        };
        tree.insert(key.as_bytes(), serialize_json(&entry)?)?;
        tree.flush()?;
        Ok(())
    }

    /// Lists all config entries, excluding the internal `initialized` marker.
    pub fn list_all(&self) -> crate::Result<Vec<ConfigEntry>> {
        let tree = self.db.open_tree(CONFIG_TREE)?;
        let mut entries = Vec::new();
        for result in &tree {
            let (_, value) = result?;
            if let Ok(entry) = deserialize_json::<ConfigEntry>(&value) {
                if entry.key != "initialized" {
                    entries.push(entry);
                }
            }
        }
        Ok(entries)
    }

    /// Removes a config entry by key.
    pub fn delete(&self, key: &str) -> crate::Result<()> {
        let tree = self.db.open_tree(CONFIG_TREE)?;
        tree.remove(key.as_bytes())?;
        tree.flush()?;
        Ok(())
    }

    /// Exports all entries as a portable [`ConfigBundle`].
    pub fn export_bundle(&self) -> crate::Result<ConfigBundle> {
        let entries = self.list_all()?;
        Ok(ConfigBundle {
            format_version: 1,
            exported_at: chrono::Utc::now(),
            entries,
        })
    }

    /// Imports all entries from a [`ConfigBundle`], returning the number of
    /// entries written.
    pub fn import_bundle(&self, bundle: &ConfigBundle) -> crate::Result<usize> {
        let tree = self.db.open_tree(CONFIG_TREE)?;
        let mut count = 0;
        for entry in &bundle.entries {
            tree.insert(entry.key.as_bytes(), serialize_json(entry)?)?;
            count += 1;
        }
        tree.flush()?;
        Ok(count)
    }

    /// Validates a config key and value, returning an error string describing
    /// the first violated rule.
    pub fn validate(&self, key: &str, value: &serde_json::Value) -> Result<(), String> {
        if key.is_empty() {
            return Err("Config key cannot be empty".to_string());
        }
        if key.len() > 256 {
            return Err("Config key too long (max 256 chars)".to_string());
        }
        if !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        {
            return Err("Config key contains invalid characters. Use alphanumeric, dots, underscores, or hyphens.".to_string());
        }
        if value.is_null() {
            return Err("Config value cannot be null".to_string());
        }
        Ok(())
    }

    /// Captures the current config as a snapshot and returns its id.
    pub fn create_snapshot(&self, name: &str, description: &str) -> crate::Result<String> {
        let entries = self.list_all()?;
        let snapshot = ConfigSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            entries,
            created_at: chrono::Utc::now(),
            description: description.to_string(),
        };
        let tree = self.db.open_tree(SNAPSHOT_TREE)?;
        let id = snapshot.id.clone();
        tree.insert(id.as_bytes(), serialize_json(&snapshot)?)?;
        tree.flush()?;
        Ok(id)
    }

    /// Lists all snapshots, newest first.
    pub fn list_snapshots(&self) -> crate::Result<Vec<ConfigSnapshot>> {
        let tree = self.db.open_tree(SNAPSHOT_TREE)?;
        let mut snapshots = Vec::new();
        for result in &tree {
            let (_, value) = result?;
            if let Ok(snapshot) = deserialize_json::<ConfigSnapshot>(&value) {
                snapshots.push(snapshot);
            }
        }
        snapshots.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        Ok(snapshots)
    }

    /// Reads a snapshot by id, or `None` if it does not exist.
    pub fn get_snapshot(&self, id: &str) -> crate::Result<Option<ConfigSnapshot>> {
        let tree = self.db.open_tree(SNAPSHOT_TREE)?;
        if let Some(bytes) = tree.get(id.as_bytes())? {
            Ok(Some(deserialize_json(&bytes)?))
        } else {
            Ok(None)
        }
    }

    /// Restores config entries from a snapshot, overwriting current values,
    /// and returns the number of entries restored.
    pub fn restore_snapshot(&self, id: &str) -> crate::Result<usize> {
        let snapshot = self.get_snapshot(id)?.ok_or_else(|| {
            crate::Error::ConfigurationError(format!("Snapshot '{}' not found", id))
        })?;
        let tree = self.db.open_tree(CONFIG_TREE)?;
        let mut count = 0;
        for entry in &snapshot.entries {
            tree.insert(entry.key.as_bytes(), serialize_json(entry)?)?;
            count += 1;
        }
        tree.flush()?;
        Ok(count)
    }

    /// Deletes a snapshot by id.
    pub fn delete_snapshot(&self, id: &str) -> crate::Result<()> {
        let tree = self.db.open_tree(SNAPSHOT_TREE)?;
        tree.remove(id.as_bytes())?;
        tree.flush()?;
        Ok(())
    }

    /// Total number of stored config entries, including internal markers.
    pub fn count(&self) -> crate::Result<usize> {
        let tree = self.db.open_tree(CONFIG_TREE)?;
        Ok(tree.len())
    }
}

/// Portable export/import representation of a configuration set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigBundle {
    /// Format version of the bundle schema.
    pub format_version: u64,
    /// Wall-clock time the bundle was exported.
    pub exported_at: chrono::DateTime<chrono::Utc>,
    /// Entries contained in the bundle.
    pub entries: Vec<ConfigEntry>,
}
