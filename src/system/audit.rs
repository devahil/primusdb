//! # AuditLogger — Security and Operational Event Logging
//!
//! Records structured audit events with automatic pruning at
//! `MAX_AUDIT_EVENTS` (10 000) entries.
//!
//! ## Architecture
//!
//! ```text
//! AuditLogger
//!   +-> sys_audit tree (sled)
//!   |     Key: "{timestamp_nanos}_{uuid}"
//!   |     Value: AuditEvent { id, timestamp, event_type, actor,
//!   |                         resource, action, detail, success }
//!   |
//!   +-> Automatic pruning when count exceeds MAX_AUDIT_EVENTS
//!   |     Removes oldest entries to stay within limit
//!   |
//!   +-> Query methods: recent(), by_type(), count()
//! ```
//!
//! ## Event Types (examples)
//!
//! - `system.startup` — server started
//! - `config.change` — configuration modified
//! - `config.export` — configuration exported
//! - `backup.create` — backup created
//! - `backup.restore` — backup restored
//! - `user.login` — user authenticated

use serde::{Deserialize, Serialize};
use sled::Db;
use std::sync::Arc;

const AUDIT_TREE: &str = "sys_audit";

/// Maximum number of audit events retained. When `log` exceeds this limit the
/// oldest events are pruned.
pub const MAX_AUDIT_EVENTS: usize = 10_000;

/// Persists structured audit events in the `sys_audit` sled tree and provides
/// recent / by-type / count queries.
pub struct AuditLogger {
    db: Arc<Db>,
}

/// A single recorded audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Randomly generated event identifier.
    pub id: String,
    /// Wall-clock time the event was recorded.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event category, e.g. `system.startup`, `config.change`, `backup.create`.
    pub event_type: String,
    /// Identity of the acting user or system component.
    pub actor: String,
    /// Resource the event refers to, e.g. `server.port`.
    pub resource: String,
    /// Verb describing the operation, e.g. `update`, `read`, `create`.
    pub action: String,
    /// Arbitrary JSON payload with operation-specific context.
    pub detail: serde_json::Value,
    /// Whether the operation succeeded.
    pub success: bool,
}

fn serialize_json<T: Serialize>(value: &T) -> crate::Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| crate::Error::ConfigurationError(e.to_string()))
}

fn deserialize_json<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> crate::Result<T> {
    serde_json::from_slice(bytes).map_err(|e| crate::Error::ConfigurationError(e.to_string()))
}

impl AuditLogger {
    /// Creates a logger backed by the given sled database.
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    /// Ensures the audit tree exists and is marked as initialised.
    pub fn init(&self) -> crate::Result<()> {
        let tree = self.db.open_tree(AUDIT_TREE)?;
        if tree.get("initialized")?.is_none() {
            tree.insert("initialized", serialize_json(&true)?)?;
            tree.flush()?;
        }
        Ok(())
    }

    /// Records a new audit event, pruning oldest entries when the tree exceeds
    /// [`MAX_AUDIT_EVENTS`], and returns the persisted event.
    pub fn log(
        &self,
        event_type: &str,
        actor: &str,
        resource: &str,
        action: &str,
        detail: serde_json::Value,
        success: bool,
    ) -> crate::Result<AuditEvent> {
        let tree = self.db.open_tree(AUDIT_TREE)?;
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            event_type: event_type.to_string(),
            actor: actor.to_string(),
            resource: resource.to_string(),
            action: action.to_string(),
            detail,
            success,
        };
        let key = format!(
            "{}_{}",
            event.timestamp.timestamp_nanos_opt().unwrap_or(0),
            event.id
        );
        tree.insert(key.as_bytes(), serialize_json(&event)?)?;

        let count = tree.len();
        if count > MAX_AUDIT_EVENTS {
            self.prune_oldest(count - MAX_AUDIT_EVENTS)?;
        }

        tree.flush()?;
        Ok(event)
    }

    /// Returns up to `limit` most recent events, newest first.
    pub fn recent(&self, limit: usize) -> crate::Result<Vec<AuditEvent>> {
        let tree = self.db.open_tree(AUDIT_TREE)?;
        let mut events: Vec<AuditEvent> = Vec::new();
        for result in tree.iter().rev() {
            let (_, value) = result?;
            if let Ok(event) = deserialize_json::<AuditEvent>(&value) {
                events.push(event);
                if events.len() >= limit {
                    break;
                }
            }
        }
        Ok(events)
    }

    /// Returns up to `limit` events matching `event_type`, newest first.
    pub fn by_type(&self, event_type: &str, limit: usize) -> crate::Result<Vec<AuditEvent>> {
        let tree = self.db.open_tree(AUDIT_TREE)?;
        let mut events: Vec<AuditEvent> = Vec::new();
        for result in tree.iter().rev() {
            let (_, value) = result?;
            if let Ok(event) = deserialize_json::<AuditEvent>(&value) {
                if event.event_type == event_type {
                    events.push(event);
                    if events.len() >= limit {
                        break;
                    }
                }
            }
        }
        Ok(events)
    }

    /// Total number of stored audit events.
    pub fn count(&self) -> crate::Result<usize> {
        let tree = self.db.open_tree(AUDIT_TREE)?;
        Ok(tree.len())
    }

    fn prune_oldest(&self, count: usize) -> crate::Result<()> {
        let tree = self.db.open_tree(AUDIT_TREE)?;
        let to_remove: Vec<Vec<u8>> = tree
            .iter()
            .take(count)
            .flatten()
            .map(|(key, _)| key.to_vec())
            .collect();
        for key in to_remove {
            tree.remove(key)?;
        }
        Ok(())
    }
}
