/*
 * PrimusDB - Change Data Capture (CDC) Module
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.3.1-alpha
 */

/*!
# Change Data Capture (CDC) Module

Tracks all data mutations (INSERT, UPDATE, DELETE) via a write-ahead log
and provides stream/poll APIs for consumers.

## Usage

```ignore
use primusdb::cdc::{CdcEngine, ChangeType, CdcConfig};

let mut engine = CdcEngine::new(10000);

// Record a change
let seq = engine.record_change(
    "users",
    "user123",
    ChangeType::Insert,
    None,
    Some(serde_json::json!({"name": "Alice"})),
);

// Poll for new events
let events = engine.events_after(0);
assert_eq!(events.len(), 1);
```

## Architecture

```text
┌──────────────────────────────────────────────────────────────┐
│                    CDC Engine                                 │
│  ┌────────────────────────────────────────────────────┐      │
│  │  Write-Ahead Log (VecDeque<ChangeEvent>)           │      │
│  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐    │      │
│  │  │ seq1 │ │ seq2 │ │ seq3 │ │ ...  │ │ seqN │    │      │
│  │  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘    │      │
│  │  └────────────────────────────────────────────────┘      │
│  │              ▲                      │                     │
│  │              │ record_change        │ events_after        │
│  │              │                      ▼                     │
│  │  ┌────────────────────────────────────────────────────┐   │
│  │  │           Stream / Poll Consumers                  │   │
│  │  └────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

## Features

- **WAL-based tracking**: All mutations are recorded as ordered change events
- **Bounded memory**: Configurable maximum WAL size with automatic pruning
- **Monotonic sequencing**: Each event gets a unique, monotonically increasing sequence number
- **Poll-based consumption**: Consumers can poll for new events since a given sequence
- **Activation control**: CDC can be enabled or disabled at runtime
- **Zero external dependencies**: Uses only the standard library and serde
*/

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Type of data change
///
/// Represents the kind of mutation that occurred on a document or record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    /// A new record was created
    Insert,
    /// An existing record was modified
    Update,
    /// A record was removed
    Delete,
}

/// A single change event in the CDC log
///
/// Contains all metadata about a data mutation including the type of change,
/// the target collection and document, and before/after values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvent {
    /// Monotonically increasing sequence number
    pub sequence: u64,
    /// Timestamp (unix millis) when the change occurred
    pub timestamp: u64,
    /// Collection/table name
    pub collection: String,
    /// Document/row ID
    pub document_id: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Previous value (for UPDATE/DELETE)
    pub old_value: Option<serde_json::Value>,
    /// New value (for INSERT/UPDATE)
    pub new_value: Option<serde_json::Value>,
}

/// Configuration for the CDC engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdcConfig {
    /// Maximum number of WAL entries in memory
    /// When the WAL exceeds this limit, the oldest entries are pruned
    pub max_wal_size: usize,
    /// Whether CDC starts active
    /// When false, record_change() will return 0 and not store events
    pub auto_start: bool,
    /// Optional path to persist the WAL to disk via sled
    /// When set, every record_change() will also flush to durable storage
    pub persist_path: Option<PathBuf>,
}

impl Default for CdcConfig {
    fn default() -> Self {
        Self {
            max_wal_size: 10000,
            auto_start: true,
            persist_path: None,
        }
    }
}

/// CDC engine state
///
/// Manages the write-ahead log of change events and provides
/// APIs for recording and consuming data mutations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdcEngine {
    /// WAL entries (in-memory, bounded)
    wal: VecDeque<ChangeEvent>,
    /// Current sequence number
    current_sequence: u64,
    /// Maximum WAL size (0 = unlimited)
    max_wal_size: usize,
    /// Whether CDC is active
    active: bool,
    /// Optional sled database for persisting the WAL to disk
    #[serde(skip)]
    db: Option<sled::Db>,
    /// Persist path for re-opening the DB on load
    #[allow(dead_code)]
    persist_path: Option<PathBuf>,
}

impl CdcEngine {
    /// Create a new CDC engine with the given maximum WAL size
    ///
    /// # Arguments
    /// * `max_wal_size` - Maximum number of WAL entries to keep in memory.
    ///   Use 0 for unlimited. When exceeded, oldest entries are pruned.
    ///
    /// # Example
    /// ```ignore
    /// let mut engine = CdcEngine::new(10000);
    /// ```
    pub fn new(max_wal_size: usize) -> Self {
        Self {
            wal: VecDeque::new(),
            current_sequence: 0,
            max_wal_size,
            active: true,
            db: None,
            persist_path: None,
        }
    }

    /// Create a new CDC engine from a configuration
    ///
    /// # Arguments
    /// * `config` - CDC configuration specifying max WAL size, auto-start behavior, and optional persistence path
    ///
    /// If `config.persist_path` is set, the engine will open a sled database
    /// at that path and automatically restore any previously persisted events.
    pub fn with_config(config: &CdcConfig) -> Self {
        let db = config.persist_path.as_ref().and_then(|path| {
            std::fs::create_dir_all(path).ok();
            sled::open(path).ok()
        });

        let mut engine = Self {
            wal: VecDeque::new(),
            current_sequence: 0,
            max_wal_size: config.max_wal_size,
            active: config.auto_start,
            db,
            persist_path: config.persist_path.clone(),
        };

        // Restore previously persisted state
        if engine.db.is_some() {
            let _ = engine.load();
        }

        engine
    }

    /// Record a change event
    ///
    /// Adds a new change event to the WAL and returns its sequence number.
    /// If CDC is inactive, no event is recorded and 0 is returned.
    ///
    /// # Arguments
    /// * `collection` - The collection/table name where the change occurred
    /// * `document_id` - The document/row identifier that was changed
    /// * `change_type` - The type of change (Insert, Update, Delete)
    /// * `old_value` - The previous value of the document (for UPDATE/DELETE)
    /// * `new_value` - The new value of the document (for INSERT/UPDATE)
    ///
    /// # Returns
    /// The sequence number assigned to the change event, or 0 if CDC is inactive
    pub fn record_change(
        &mut self,
        collection: &str,
        document_id: &str,
        change_type: ChangeType,
        old_value: Option<serde_json::Value>,
        new_value: Option<serde_json::Value>,
    ) -> u64 {
        if !self.active {
            return 0;
        }

        self.current_sequence += 1;
        let sequence = self.current_sequence;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let event = ChangeEvent {
            sequence,
            timestamp,
            collection: collection.to_string(),
            document_id: document_id.to_string(),
            change_type,
            old_value,
            new_value,
        };

        if self.max_wal_size > 0 && self.wal.len() >= self.max_wal_size {
            self.wal.pop_front();
        }
        self.wal.push_back(event);

        // Persist to sled if available
        if self.db.is_some() {
            let _ = self.persist_single(sequence);
        }

        sequence
    }

    /// Get events after a given sequence number (for polling)
    ///
    /// Returns all change events with a sequence number greater than the given
    /// value. Useful for consumers that want to poll for new changes since
    /// their last checkpoint.
    ///
    /// # Arguments
    /// * `sequence` - The sequence number to start after (exclusive)
    ///
    /// # Returns
    /// A vector of change events ordered by sequence number
    pub fn events_after(&self, sequence: u64) -> Vec<ChangeEvent> {
        self.wal
            .iter()
            .filter(|e| e.sequence > sequence)
            .cloned()
            .collect()
    }

    /// Get all events since the given sequence, with a limit
    ///
    /// Similar to `events_after`, but allows capping the number of returned
    /// events for pagination.
    ///
    /// # Arguments
    /// * `sequence` - The sequence number to start after (exclusive)
    /// * `limit` - Maximum number of events to return
    ///
    /// # Returns
    /// A vector of change events, ordered by sequence number, up to `limit` events
    pub fn get_since(&self, sequence: u64, limit: usize) -> Vec<ChangeEvent> {
        self.wal
            .iter()
            .filter(|e| e.sequence > sequence)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get the latest sequence number
    ///
    /// Returns the highest sequence number assigned so far.
    /// Useful for consumers to checkpoint their position.
    pub fn latest_sequence(&self) -> u64 {
        self.current_sequence
    }

    /// Enable or disable CDC
    ///
    /// When disabled, `record_change()` will not store events and returns 0.
    /// Existing events in the WAL remain accessible.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Check if CDC is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Clear all events from the WAL
    ///
    /// Removes all change events and resets the sequence counter.
    pub fn clear(&mut self) {
        self.wal.clear();
        self.current_sequence = 0;
    }

    /// Get the current number of events in the WAL
    pub fn len(&self) -> usize {
        self.wal.len()
    }

    /// Returns true if the WAL is empty
    pub fn is_empty(&self) -> bool {
        self.wal.is_empty()
    }

    /// Persist the entire WAL to sled storage
    ///
    /// Writes all in-memory change events to the sled database so they survive
    /// a process restart. Events are stored keyed by their big-endian sequence
    /// number in a dedicated "changes" tree, and the current sequence counter
    /// is stored under a "meta" key.
    ///
    /// Returns an error if no sled database was configured or if the write fails.
    pub fn persist(&self) -> Result<(), String> {
        let db = self.db.as_ref().ok_or_else(|| {
            "CDC persistence not configured: no persist_path set in CdcConfig".to_string()
        })?;

        let tree = db.open_tree("changes").map_err(|e| e.to_string())?;

        for event in &self.wal {
            let key = event.sequence.to_be_bytes();
            let value = serde_json::to_vec(event).map_err(|e| e.to_string())?;
            tree.insert(key, value).map_err(|e| e.to_string())?;
        }

        let meta = db.open_tree("meta").map_err(|e| e.to_string())?;
        meta.insert("last_sequence", &self.current_sequence.to_be_bytes())
            .map_err(|e| e.to_string())?;

        db.flush().map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Persist a single event by sequence number
    ///
    /// Called automatically by `record_change()` when a sled database is configured.
    /// Writes only the event with the given sequence number to the "changes" tree
    /// and updates the "meta/last_sequence" key.
    fn persist_single(&self, sequence: u64) -> Result<(), String> {
        let db = self.db.as_ref().ok_or_else(|| {
            "CDC persistence not configured: no persist_path set in CdcConfig".to_string()
        })?;

        // Find the event by sequence number
        let event = self
            .wal
            .iter()
            .find(|e| e.sequence == sequence)
            .ok_or_else(|| format!("Event with sequence {} not found in WAL", sequence))?;

        let tree = db.open_tree("changes").map_err(|e| e.to_string())?;
        let key = sequence.to_be_bytes();
        let value = serde_json::to_vec(event).map_err(|e| e.to_string())?;
        tree.insert(key, value).map_err(|e| e.to_string())?;

        let meta = db.open_tree("meta").map_err(|e| e.to_string())?;
        meta.insert("last_sequence", &self.current_sequence.to_be_bytes())
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Load the WAL from sled storage
    ///
    /// Restores all previously persisted change events from the sled database
    /// into the in-memory WAL. Also restores the last sequence counter.
    ///
    /// Returns an error if no sled database was configured or if the read fails.
    pub fn load(&mut self) -> Result<(), String> {
        let db = self.db.as_ref().ok_or_else(|| {
            "CDC persistence not configured: no persist_path set in CdcConfig".to_string()
        })?;

        let tree = db.open_tree("changes").map_err(|e| e.to_string())?;
        let meta = db.open_tree("meta").map_err(|e| e.to_string())?;

        // Restore last sequence number
        if let Some(last_seq_bytes) = meta.get("last_sequence").map_err(|e| e.to_string())? {
            let arr: [u8; 8] = last_seq_bytes[..8]
                .try_into()
                .map_err(|_| "Corrupted last_sequence in CDC store".to_string())?;
            self.current_sequence = u64::from_be_bytes(arr);
        }

        // Restore all change events, ordered by sequence
        let mut events: Vec<(u64, ChangeEvent)> = Vec::new();
        for result in tree.iter() {
            let (key_bytes, value_bytes) = result.map_err(|e| e.to_string())?;
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&key_bytes);
            let seq = u64::from_be_bytes(arr);

            if let Ok(event) = serde_json::from_slice::<ChangeEvent>(&value_bytes) {
                events.push((seq, event));
            }
        }

        // Sort by sequence number and populate the WAL
        events.sort_by_key(|(seq, _)| *seq);
        self.wal.clear();
        let memory_limit = if self.max_wal_size > 0 {
            self.max_wal_size
        } else {
            events.len()
        };
        for (_, event) in events.into_iter().rev().take(memory_limit).rev() {
            self.wal.push_back(event);
        }

        Ok(())
    }
}

impl Default for CdcEngine {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_retrieve() {
        let mut engine = CdcEngine::new(100);
        let seq = engine.record_change(
            "users",
            "user123",
            ChangeType::Insert,
            None,
            Some(serde_json::json!({"name": "Alice"})),
        );
        assert_eq!(seq, 1);

        let all: Vec<ChangeEvent> = engine.wal.into_iter().collect();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].sequence, 1);
        assert_eq!(all[0].collection, "users");
        assert_eq!(all[0].document_id, "user123");
        assert_eq!(all[0].change_type, ChangeType::Insert);
    }

    #[test]
    fn test_events_after() {
        let mut engine = CdcEngine::new(100);
        engine.record_change("t", "1", ChangeType::Insert, None, None);
        engine.record_change("t", "2", ChangeType::Update, None, None);
        engine.record_change("t", "3", ChangeType::Delete, None, None);

        let after_0 = engine.events_after(0);
        assert_eq!(after_0.len(), 3);

        let after_1 = engine.events_after(1);
        assert_eq!(after_1.len(), 2);
        assert_eq!(after_1[0].sequence, 2);
        assert_eq!(after_1[1].sequence, 3);

        let after_3 = engine.events_after(3);
        assert!(after_3.is_empty());
    }

    #[test]
    fn test_max_wal_size_prunes() {
        let mut engine = CdcEngine::new(3);
        engine.record_change("t", "1", ChangeType::Insert, None, None);
        engine.record_change("t", "2", ChangeType::Insert, None, None);
        engine.record_change("t", "3", ChangeType::Insert, None, None);
        assert_eq!(engine.wal.len(), 3);

        engine.record_change("t", "4", ChangeType::Insert, None, None);
        assert_eq!(engine.wal.len(), 3);

        let events = engine.events_after(0);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].document_id, "2");
        assert_eq!(events[1].document_id, "3");
        assert_eq!(events[2].document_id, "4");
    }

    #[test]
    fn test_inactive_no_recording() {
        let mut engine = CdcEngine::new(100);
        engine.set_active(false);

        let seq = engine.record_change("t", "1", ChangeType::Insert, None, None);
        assert_eq!(seq, 0);
        assert!(engine.is_empty());

        engine.set_active(true);
        let seq = engine.record_change("t", "2", ChangeType::Insert, None, None);
        assert_eq!(seq, 1);
        assert_eq!(engine.len(), 1);
    }

    #[test]
    fn test_since_with_limit() {
        let mut engine = CdcEngine::new(100);
        engine.record_change("t", "1", ChangeType::Insert, None, None);
        engine.record_change("t", "2", ChangeType::Insert, None, None);
        engine.record_change("t", "3", ChangeType::Insert, None, None);
        engine.record_change("t", "4", ChangeType::Insert, None, None);
        engine.record_change("t", "5", ChangeType::Insert, None, None);

        let since = engine.get_since(2, 2);
        assert_eq!(since.len(), 2);
        assert_eq!(since[0].sequence, 3);
        assert_eq!(since[1].sequence, 4);

        let since_all = engine.get_since(0, 100);
        assert_eq!(since_all.len(), 5);
    }

    #[test]
    fn test_sequence_monotonic() {
        let mut engine = CdcEngine::new(100);

        for i in 1..=100 {
            let seq = engine.record_change("t", &i.to_string(), ChangeType::Insert, None, None);
            assert_eq!(seq, i as u64);
        }

        assert_eq!(engine.latest_sequence(), 100);
    }

    #[test]
    fn test_old_value_for_update() {
        let mut engine = CdcEngine::new(100);
        let seq = engine.record_change(
            "users",
            "user123",
            ChangeType::Update,
            Some(serde_json::json!({"name": "Alice", "age": 30})),
            Some(serde_json::json!({"name": "Alice", "age": 31})),
        );
        assert_eq!(seq, 1);

        let events = engine.events_after(0);
        assert!(events[0].old_value.is_some());
        assert!(events[0].new_value.is_some());
        assert_eq!(
            events[0].old_value.as_ref().unwrap().get("age"),
            Some(&serde_json::json!(30))
        );
        assert_eq!(
            events[0].new_value.as_ref().unwrap().get("age"),
            Some(&serde_json::json!(31))
        );
    }

    #[test]
    fn test_new_value_for_insert() {
        let mut engine = CdcEngine::new(100);
        let data = serde_json::json!({"name": "Bob", "email": "bob@example.com"});
        let seq = engine.record_change(
            "users",
            "user456",
            ChangeType::Insert,
            None,
            Some(data.clone()),
        );
        assert_eq!(seq, 1);

        let events = engine.events_after(0);
        assert!(events[0].old_value.is_none());
        assert!(events[0].new_value.is_some());
        assert_eq!(events[0].new_value.as_ref().unwrap(), &data);
    }

    #[test]
    fn test_empty_wal() {
        let engine = CdcEngine::new(100);

        assert!(engine.is_empty());
        assert_eq!(engine.len(), 0);
        assert_eq!(engine.latest_sequence(), 0);
        assert!(engine.events_after(0).is_empty());
        assert!(engine.get_since(0, 10).is_empty());

        assert!(engine.is_active());
    }

    #[test]
    fn test_cdc_config_default() {
        let config = CdcConfig::default();
        assert_eq!(config.max_wal_size, 10000);
        assert!(config.auto_start);

        let engine = CdcEngine::with_config(&config);
        assert!(engine.is_active());
        assert_eq!(engine.len(), 0);
    }

    #[test]
    fn test_clear_resets_engine() {
        let mut engine = CdcEngine::new(100);
        engine.record_change("t", "1", ChangeType::Insert, None, None);
        engine.record_change("t", "2", ChangeType::Update, None, None);
        assert_eq!(engine.len(), 2);

        engine.clear();
        assert!(engine.is_empty());
        assert_eq!(engine.latest_sequence(), 0);
        assert!(engine.events_after(0).is_empty());
    }

    #[test]
    fn test_unlimited_wal() {
        let mut engine = CdcEngine::new(0);
        for i in 1..=10000 {
            engine.record_change("t", &i.to_string(), ChangeType::Insert, None, None);
        }
        assert_eq!(engine.len(), 10000);
        assert_eq!(engine.latest_sequence(), 10000);
    }

    #[test]
    fn test_timestamp_is_set() {
        let mut engine = CdcEngine::new(100);
        engine.record_change("t", "1", ChangeType::Insert, None, None);
        let events = engine.events_after(0);
        assert!(events[0].timestamp > 0);
    }

    #[test]
    fn test_persist_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let persist_path = dir.path().join("cdc_test");
        let config = CdcConfig {
            max_wal_size: 1000,
            auto_start: true,
            persist_path: Some(persist_path.clone()),
        };

        // Create engine with persistence, record some events
        let mut engine = CdcEngine::with_config(&config);
        engine.record_change(
            "users",
            "u1",
            ChangeType::Insert,
            None,
            Some(serde_json::json!({"name": "Alice"})),
        );
        engine.record_change(
            "users",
            "u2",
            ChangeType::Insert,
            None,
            Some(serde_json::json!({"name": "Bob"})),
        );
        engine.record_change(
            "orders",
            "o1",
            ChangeType::Update,
            Some(serde_json::json!({"status": "pending"})),
            Some(serde_json::json!({"status": "shipped"})),
        );
        assert_eq!(engine.len(), 3);
        assert_eq!(engine.latest_sequence(), 3);

        // Explicit persist
        assert!(engine.persist().is_ok());

        // Drop the engine
        drop(engine);

        // Re-create engine from the same path — should auto-restore
        let config2 = CdcConfig {
            max_wal_size: 1000,
            auto_start: true,
            persist_path: Some(persist_path.clone()),
        };
        let restored = CdcEngine::with_config(&config2);

        assert_eq!(restored.len(), 3);
        assert_eq!(restored.latest_sequence(), 3);

        let events = restored.events_after(0);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].collection, "users");
        assert_eq!(events[0].document_id, "u1");
        assert_eq!(events[0].change_type, ChangeType::Insert);
        assert_eq!(events[1].collection, "users");
        assert_eq!(events[1].document_id, "u2");
        assert_eq!(events[2].collection, "orders");
        assert_eq!(events[2].change_type, ChangeType::Update);
    }

    #[test]
    fn test_persist_no_path_returns_error() {
        let engine = CdcEngine::new(100);
        let result = engine.persist();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("no persist_path set in CdcConfig"));
    }

    #[test]
    fn test_load_after_persist_preserves_all_fields() {
        let dir = tempfile::tempdir().unwrap();
        let persist_path = dir.path().join("cdc_fields");
        let config = CdcConfig {
            max_wal_size: 100,
            auto_start: true,
            persist_path: Some(persist_path),
        };

        let mut engine = CdcEngine::with_config(&config);
        let old_value = serde_json::json!({"name": "Alice", "age": 30});
        let new_value = serde_json::json!({"name": "Alice", "age": 31});

        engine.record_change(
            "users",
            "u1",
            ChangeType::Update,
            Some(old_value.clone()),
            Some(new_value.clone()),
        );

        // Verify fields before persist
        let events_before = engine.events_after(0);
        assert_eq!(events_before[0].old_value, Some(old_value.clone()));
        assert_eq!(events_before[0].new_value, Some(new_value.clone()));
        assert!(events_before[0].timestamp > 0);

        engine.persist().unwrap();
        drop(engine);

        let config2 = CdcConfig {
            max_wal_size: 100,
            auto_start: true,
            persist_path: Some(dir.path().join("cdc_fields")),
        };
        let restored = CdcEngine::with_config(&config2);
        let events = restored.events_after(0);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].collection, "users");
        assert_eq!(events[0].document_id, "u1");
        assert_eq!(events[0].change_type, ChangeType::Update);
        assert_eq!(events[0].old_value, Some(old_value));
        assert_eq!(events[0].new_value, Some(new_value));
        assert!(events[0].timestamp > 0);
    }

    #[test]
    fn test_delete_has_no_new_value() {
        let mut engine = CdcEngine::new(100);
        let seq = engine.record_change(
            "users",
            "user123",
            ChangeType::Delete,
            Some(serde_json::json!({"name": "Alice"})),
            None,
        );
        assert_eq!(seq, 1);

        let events = engine.events_after(0);
        assert_eq!(events[0].change_type, ChangeType::Delete);
        assert!(events[0].old_value.is_some());
        assert!(events[0].new_value.is_none());
    }

    #[test]
    fn test_default_engine() {
        let engine = CdcEngine::default();
        assert_eq!(engine.max_wal_size, 10000);
        assert!(engine.is_active());
        assert!(engine.is_empty());
    }

    #[test]
    fn test_events_after_unknown_sequence() {
        let mut engine = CdcEngine::new(100);
        engine.record_change("t", "1", ChangeType::Insert, None, None);
        engine.record_change("t", "2", ChangeType::Insert, None, None);

        let events = engine.events_after(999);
        assert!(events.is_empty());
    }

    #[test]
    fn test_get_since_exact_boundary() {
        let mut engine = CdcEngine::new(100);
        engine.record_change("t", "1", ChangeType::Insert, None, None);
        engine.record_change("t", "2", ChangeType::Insert, None, None);
        engine.record_change("t", "3", ChangeType::Insert, None, None);

        let events = engine.get_since(1, 1);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].sequence, 2);

        let events = engine.get_since(1, 2);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence, 2);
        assert_eq!(events[1].sequence, 3);
    }
}
