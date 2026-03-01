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
