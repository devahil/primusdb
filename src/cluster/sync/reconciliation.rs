/*
 * PrimusDB Data Reconciliation Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: Cross-node data reconciliation
 */

//! Data Reconciliation Engine
//!
//! This module handles cross-node data reconciliation, conflict detection,
//! and resolution for distributed PrimusDB operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Conflict between two versions of a record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConflict {
    /// Conflict key
    pub key: String,
    /// Local version
    pub local_version: RecordVersion,
    /// Remote version
    pub remote_version: RecordVersion,
    /// Resolution applied
    pub resolution: ConflictResolutionStrategy,
}

/// Version information for a record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordVersion {
    /// Record key
    pub key: String,
    /// Version number
    pub version: u64,
    /// Vector clock
    pub vector_clock: HashMap<String, u64>,
    /// Last modified timestamp
    pub timestamp: u64,
    /// Node that last modified
    pub modified_by: String,
    /// Data checksum
    pub checksum: String,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    /// Keep local version
    KeepLocal,
    /// Keep remote version
    KeepRemote,
    /// Keep most recent by timestamp
    KeepMostRecent,
    /// Merge both versions (CRDT)
    Merge,
    /// Manual resolution required
    Manual,
}

/// Reconciliation plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationPlan {
    /// Records to pull from remote
    pub pull_records: Vec<String>,
    /// Records to push to remote
    pub push_records: Vec<String>,
    /// Conflicts to resolve
    pub conflicts: Vec<DataConflict>,
    /// Estimated transfer size (bytes)
    pub estimated_bytes: u64,
}

/// Merkle tree node for efficient comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// Node hash
    pub hash: String,
    /// Child hashes (if internal node)
    pub children: Option<Vec<String>>,
    /// Key range (if leaf)
    pub key_range: Option<(String, String)>,
    /// Whether this node is a leaf
    pub is_leaf: bool,
}

/// Reconciliation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationStats {
    /// Total conflicts found
    pub conflicts_found: u64,
    /// Conflicts resolved automatically
    pub conflicts_resolved: u64,
    /// Records pulled
    pub records_pulled: u64,
    /// Records pushed
    pub records_pushed: u64,
    /// Transfer bytes
    pub bytes_transferred: u64,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl Default for ReconciliationStats {
    fn default() -> Self {
        Self {
            conflicts_found: 0,
            conflicts_resolved: 0,
            records_pulled: 0,
            records_pushed: 0,
            bytes_transferred: 0,
            duration_ms: 0,
        }
    }
}

/// Vector clock ordering result
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VClockOrder {
    /// `a` happens-before `b` → keep `b`
    Before,
    /// `a` happens-after `b` → keep `a`
    After,
    /// Concurrent → merge or manual
    Concurrent,
    /// Identical
    Equal,
}

/// Compare two vector clocks to determine causality.
/// Returns how `local` relates to `remote`.
pub fn compare_vector_clocks(local: &HashMap<String, u64>, remote: &HashMap<String, u64>) -> VClockOrder {
    let mut local_newer = false;
    let mut remote_newer = false;

    let all_keys: std::collections::HashSet<&String> =
        local.keys().chain(remote.keys()).collect();

    for key in all_keys {
        let lv = local.get(key).copied().unwrap_or(0);
        let rv = remote.get(key).copied().unwrap_or(0);
        if lv > rv {
            local_newer = true;
        }
        if rv > lv {
            remote_newer = true;
        }
    }

    match (local_newer, remote_newer) {
        (false, false) => VClockOrder::Equal,
        (true, false) => VClockOrder::After,
        (false, true) => VClockOrder::Before,
        (true, true) => VClockOrder::Concurrent,
    }
}

/// Resolve a cross-cluster data conflict using vector clocks.
/// Returns the winning RecordVersion and the chosen strategy.
pub fn resolve_cross_cluster_conflict(
    local: &RecordVersion,
    remote: &RecordVersion,
) -> (RecordVersion, ConflictResolutionStrategy) {
    match compare_vector_clocks(&local.vector_clock, &remote.vector_clock) {
        VClockOrder::Equal | VClockOrder::After => {
            (local.clone(), ConflictResolutionStrategy::KeepLocal)
        }
        VClockOrder::Before => {
            (remote.clone(), ConflictResolutionStrategy::KeepRemote)
        }
        VClockOrder::Concurrent => {
            // Concurrent writes: merge by taking the one with more recent timestamp
            if local.timestamp >= remote.timestamp {
                (local.clone(), ConflictResolutionStrategy::KeepMostRecent)
            } else {
                (remote.clone(), ConflictResolutionStrategy::KeepMostRecent)
            }
        }
    }
}

/// Build a reconciliation plan between two clusters based on merkle
/// comparison and vector clock analysis.
pub fn build_cross_cluster_reconciliation_plan(
    local_versions: &HashMap<String, RecordVersion>,
    remote_versions: &HashMap<String, RecordVersion>,
) -> ReconciliationPlan {
    let mut pull_records = Vec::new();
    let mut push_records = Vec::new();
    let mut conflicts = Vec::new();

    for (key, local_ver) in local_versions {
        match remote_versions.get(key) {
            None => {
                // Only we have it → push to remote
                push_records.push(key.clone());
            }
            Some(remote_ver) => {
                match compare_vector_clocks(&local_ver.vector_clock, &remote_ver.vector_clock) {
                    VClockOrder::Equal => {} // identical, skip
                    VClockOrder::After => {
                        push_records.push(key.clone());
                    }
                    VClockOrder::Before => {
                        pull_records.push(key.clone());
                    }
                    VClockOrder::Concurrent => {
                        let (_, strategy) = resolve_cross_cluster_conflict(local_ver, remote_ver);
                        conflicts.push(DataConflict {
                            key: key.clone(),
                            local_version: local_ver.clone(),
                            remote_version: remote_ver.clone(),
                            resolution: strategy,
                        });
                    }
                }
            }
        }
    }

    // Also find keys only on remote
    for key in remote_versions.keys() {
        if !local_versions.contains_key(key) {
            pull_records.push(key.clone());
        }
    }

    let estimated_bytes = (pull_records.len() + push_records.len()) as u64 * 1024; // rough estimate

    ReconciliationPlan {
        pull_records,
        push_records,
        conflicts,
        estimated_bytes,
    }
}

impl RecordVersion {
    pub fn new(key: &str, modified_by: &str, clock: HashMap<String, u64>) -> Self {
        Self {
            key: key.to_string(),
            version: 1,
            vector_clock: clock,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            modified_by: modified_by.to_string(),
            checksum: String::new(),
        }
    }

    pub fn advance(&mut self, node_id: &str) {
        let counter = self.vector_clock.entry(node_id.to_string()).or_insert(0);
        *counter += 1;
        self.version += 1;
        self.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.modified_by = node_id.to_string();
    }
}
