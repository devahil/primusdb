/*
 * PrimusDB Distributed Consensus Protocol
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: Raft-style consensus for cluster operations
 */

//! Distributed Consensus Protocol
//!
//! This module implements a Raft-style consensus protocol for PrimusDB cluster operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Consensus role in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConsensusRole {
    /// Follower node
    Follower,
    /// Candidate for leadership
    Candidate,
    /// Current leader
    Leader,
}

/// Consensus state for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusState {
    /// Current role
    pub role: ConsensusRole,
    /// Current term
    pub term: u64,
    /// Voted for in current term
    pub voted_for: Option<String>,
    /// Leader ID if known
    pub leader_id: Option<String>,
    /// Last log index
    pub last_log_index: u64,
    /// Last log term
    pub last_log_term: u64,
    /// Commit index
    pub commit_index: u64,
    /// Applied index
    pub applied_index: u64,
}

impl Default for ConsensusState {
    fn default() -> Self {
        Self {
            role: ConsensusRole::Follower,
            term: 0,
            voted_for: None,
            leader_id: None,
            last_log_index: 0,
            last_log_term: 0,
            commit_index: 0,
            applied_index: 0,
        }
    }
}

/// Vote request for leader election
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    /// Candidate's term
    pub term: u64,
    /// Candidate requesting vote
    pub candidate_id: String,
    /// Last log index
    pub last_log_index: u64,
    /// Last log term
    pub last_log_term: u64,
}

/// Vote response from a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResponse {
    /// Responder's term
    pub term: u64,
    /// Whether vote was granted
    pub vote_granted: String,
    /// Node that responded
    pub node_id: String,
}

/// Append entries request for log replication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    /// Leader's term
    pub term: u64,
    /// Leader's ID
    pub leader_id: String,
    /// Index of log entry before new entries
    pub prev_log_index: u64,
    /// Term of prev_log_index entry
    pub prev_log_term: u64,
    /// Log entries to append
    pub entries: Vec<LogEntry>,
    /// Leader's commit index
    pub leader_commit: u64,
}

/// Response to append entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// Follower's term
    pub term: u64,
    /// Whether append was successful
    pub success: bool,
    /// Match index for leader
    pub match_index: u64,
    /// Node that responded
    pub node_id: String,
}

/// Log entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Entry index
    pub index: u64,
    /// Entry term
    pub term: u64,
    /// Operation type
    pub op_type: String,
    /// Operation data
    pub data: serde_json::Value,
    /// Timestamp
    pub timestamp: u64,
}

/// Consensus configuration
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Election timeout range (ms)
    pub election_timeout_min_ms: u64,
    pub election_timeout_max_ms: u64,
    /// Heartbeat interval (ms)
    pub heartbeat_interval_ms: u64,
    /// Maximum entries per append
    pub max_entries_per_append: usize,
    /// Snapshot interval
    pub snapshot_interval_entries: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            election_timeout_min_ms: 150,
            election_timeout_max_ms: 300,
            heartbeat_interval_ms: 50,
            max_entries_per_append: 100,
            snapshot_interval_entries: 10000,
        }
    }
}
