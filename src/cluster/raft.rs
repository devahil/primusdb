//! Raft consensus implementation
//!
//! Implements a simplified Raft state machine for PrimusDB cluster operations.
//! Each node tracks its role (follower / candidate / leader), a monotonically
//! increasing term, a replicated log of serialized [`LogEntry`] values, and a
//! commit index. A leader is elected by majority vote, then replicates entries
//! to followers via append-entries RPCs; entries are committed once a quorum
//! acknowledges them and are forwarded to storage through the apply channel.
//!
//! Snapshots (see [`RaftSnapshot`]) allow a lagging follower to catch up by
//! installing the leader's state instead of replaying the full log.
//!
//! # Placement in the architecture
//!
//! `RaftNode` is the ordering engine of a single cluster. The
//! [`crate::cluster::ClusterManager`] owns it and forwards committed entries to
//! storage through the apply channel; peers come from the
//! [`crate::cluster::membership::MembershipManager`]. For agreement *between*
//! clusters, see [`crate::cluster::federated_raft::FederatedRaft`].
//!
//! ```text
//!   client operation ──► ClusterManager::propose
//!                             │
//!                             ▼
//!               RaftNode (leader) appends LogEntry
//!                             │
//!       AppendEntries RPC  ┌──┴──────┐
//!       ┌─────────────────►│ followers │
//!       │                  └──────────┘
//!       ▼
//!   quorum of success responses ──► commit_index advances
//!                             │
//!                             ▼
//!                     apply_tx ──► storage layer
//!
//!   lagging follower: InstallSnapshot ──► RaftSnapshot replaces log tail
//! ```

use crate::cluster::rpc::{
    InstallSnapshotRequest, RaftAppendRequest, RaftAppendResponse, RaftVoteRequest,
    RaftVoteResponse, RpcClient, RpcMessage,
};
use crate::cluster::sync::consensus::{ConsensusConfig, ConsensusRole, ConsensusState, LogEntry};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

/// Type of the replicated log: an ordered list of serialized [`LogEntry`] bytes.
pub type LogStore = Vec<Vec<u8>>;

/// A single Raft peer with replicated state and an RPC client to its node.
///
/// Wraps the consensus state machine (`term`, `role`, `voted_for`), the
/// replicated log, commit/apply cursors, and the outbound channel that delivers
/// committed entries to the storage layer.
pub struct RaftNode {
    /// Unique ID of this node in the cluster
    pub node_id: String,
    /// Consensus state machine (role, term, leader hint)
    pub state: RwLock<ConsensusState>,
    /// Consensus tuning parameters
    pub config: ConsensusConfig,
    /// RPC clients to peer nodes, keyed by peer node ID
    pub peers: RwLock<HashMap<String, Arc<RpcClient>>>,
    /// Replicated log of serialized entries
    pub log: RwLock<Vec<Vec<u8>>>,
    /// Highest log index known to be committed by the leader
    pub commit_index: RwLock<u64>,
    /// Highest committed index already applied to storage
    pub last_applied: RwLock<u64>,
    /// Latest installed snapshot, if any
    pub snapshot: RwLock<Option<RaftSnapshot>>,
    /// Channel delivering committed entry bytes to the apply layer
    pub apply_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Cached ID of the most recently observed leader
    pub leader_hint: RwLock<Option<String>>,
}

/// A snapshot of the replicated state installed on a lagging follower.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftSnapshot {
    /// Log index of the last entry included in the snapshot
    pub last_included_index: u64,
    /// Term of the last entry included in the snapshot
    pub last_included_term: u64,
    /// Serialized state data of the snapshot
    pub data: Vec<u8>,
}

impl RaftNode {
    /// Create a Raft node with the given ID, config, peer clients and apply channel.
    pub fn new(
        node_id: String,
        config: ConsensusConfig,
        peers: HashMap<String, Arc<RpcClient>>,
        apply_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Self {
        Self {
            node_id,
            state: RwLock::new(ConsensusState::default()),
            config,
            peers: RwLock::new(peers),
            log: RwLock::new(Vec::new()),
            commit_index: RwLock::new(0),
            last_applied: RwLock::new(0),
            snapshot: RwLock::new(None),
            apply_tx,
            leader_hint: RwLock::new(None),
        }
    }

    /// The current term number.
    pub async fn current_term(&self) -> u64 {
        self.state.read().await.term
    }

    /// Whether this node is currently the elected leader.
    pub async fn is_leader(&self) -> bool {
        matches!(self.state.read().await.role, ConsensusRole::Leader)
    }

    /// ID of the current leader, if known.
    pub async fn leader_id(&self) -> Option<String> {
        self.state.read().await.leader_id.clone()
    }

    /// Index of the last entry in the replicated log.
    pub async fn last_log_index(&self) -> u64 {
        self.log.read().await.len() as u64
    }

    /// Term of the last entry in the replicated log (0 if empty).
    pub async fn last_log_term(&self) -> u64 {
        let log = self.log.read().await;
        if log.is_empty() {
            0
        } else {
            let last: Vec<u8> = log.last().unwrap().clone();
            drop(log);
            deserialize_log_entry(&last).map(|e| e.term).unwrap_or(0)
        }
    }

    /// Append raw entry bytes to the local log and return the new entry index.
    pub async fn append_entry(&self, data: Vec<u8>) -> u64 {
        let mut log = self.log.write().await;
        let index = log.len() as u64 + 1;
        log.push(data);
        index
    }

    /// Step down to follower for `term`, optionally recording the leader.
    pub async fn become_follower(&self, term: u64, leader_id: Option<String>) {
        let mut state = self.state.write().await;
        if term > state.term {
            state.term = term;
            state.role = ConsensusRole::Follower;
            state.voted_for = None;
            state.leader_id = leader_id.clone();
            *self.leader_hint.write().await = leader_id;
        }
    }

    /// Enter candidate state for a new election: increment the term and vote
    /// for self.
    pub async fn become_candidate(&self) {
        let mut state = self.state.write().await;
        state.term += 1;
        state.role = ConsensusRole::Candidate;
        state.voted_for = Some(self.node_id.clone());
        state.leader_id = None;
    }

    /// Promote this node to leader for the current term.
    pub async fn become_leader(&self) {
        let mut state = self.state.write().await;
        state.role = ConsensusRole::Leader;
        state.leader_id = Some(self.node_id.clone());
        *self.leader_hint.write().await = Some(self.node_id.clone());
    }

    /// Begin a leader election by requesting votes from all connected peers.
    /// Becomes leader if a quorum of votes is received.
    pub async fn start_election(&self) -> Result<()> {
        self.become_candidate().await;
        let term = self.state.read().await.term;
        let last_log_index = self.last_log_index().await;
        let last_log_term = self.last_log_term().await;

        info!(
            "Starting election term {} on node {} (last_log: idx={}, term={})",
            term, self.node_id, last_log_index, last_log_term
        );

        let peers = self.peers.read().await;
        let mut votes_received = 1;

        for (peer_id, client) in peers.iter() {
            if !client.is_connected().await {
                continue;
            }
            let req = RpcMessage::RequestVote(RaftVoteRequest {
                term,
                candidate_id: self.node_id.clone(),
                last_log_index,
                last_log_term,
            });
            match client.send(&req).await {
                Ok(RpcMessage::VoteResponse(resp)) => {
                    if resp.vote_granted {
                        debug!("Vote granted by {}", peer_id);
                        votes_received += 1;
                    } else if resp.term > term {
                        self.become_follower(resp.term, None).await;
                        return Ok(());
                    }
                }
                _ => {
                    debug!("No vote from {} (unreachable)", peer_id);
                }
            }
        }

        let quorum = peers.len().div_ceil(2) + 1;
        if votes_received >= quorum {
            info!(
                "Elected leader for term {} with {}/{} votes",
                term,
                votes_received,
                peers.len() + 1
            );
            self.become_leader().await;
            self.send_heartbeats().await?;
        } else {
            debug!(
                "Election lost: {}/{} votes needed {}",
                votes_received,
                peers.len() + 1,
                quorum
            );
            self.become_follower(term, None).await;
        }
        Ok(())
    }

    /// Send empty append-entries (heartbeat) messages to all connected peers.
    pub async fn send_heartbeats(&self) -> Result<()> {
        let term = self.state.read().await.term;
        let log_len = self.last_log_index().await;
        let peers = self.peers.read().await;

        for client in peers.values() {
            if !client.is_connected().await {
                continue;
            }
            let req = RpcMessage::AppendEntries(RaftAppendRequest {
                term,
                leader_id: self.node_id.clone(),
                prev_log_index: log_len,
                prev_log_term: self.last_log_term().await,
                entries: vec![],
                leader_commit: *self.commit_index.read().await,
            });
            if let Err(e) = client.send(&req).await {
                debug!("Heartbeat to {} failed: {}", client.node_id(), e);
            }
        }
        Ok(())
    }

    /// Append an entry to the local log and replicate it to peers.
    ///
    /// The entry is committed (and forwarded to the apply channel) once a quorum
    /// of nodes acknowledges it. Returns an error if replication fails.
    pub async fn replicate_entry(&self, entry_data: Vec<u8>) -> Result<()> {
        let index = self.append_entry(entry_data.clone()).await;
        let term = self.state.read().await.term;
        let peers = self.peers.read().await;
        let log = self.log.read().await;
        let prev_log_index = index.saturating_sub(1);
        let prev_log_term = if prev_log_index > 0 && (prev_log_index as usize) <= log.len() {
            let prev: Vec<u8> = log[prev_log_index as usize - 1].clone();
            drop(log);
            deserialize_log_entry(&prev).map(|e| e.term).unwrap_or(0)
        } else {
            drop(log);
            0
        };

        let mut successes = 1;
        for client in peers.values() {
            if !client.is_connected().await {
                continue;
            }
            let req = RpcMessage::AppendEntries(RaftAppendRequest {
                term,
                leader_id: self.node_id.clone(),
                prev_log_index,
                prev_log_term,
                entries: vec![entry_data.clone()],
                leader_commit: *self.commit_index.read().await,
            });
            if let Ok(RpcMessage::AppendEntriesResponse(resp)) = client.send(&req).await {
                if resp.success {
                    successes += 1;
                } else if resp.term > term {
                    self.become_follower(resp.term, None).await;
                    return Err(crate::Error::ClusterError("Stale leader".into()));
                }
            }
        }

        let quorum = peers.len().div_ceil(2) + 1;
        if successes >= quorum {
            let mut ci = self.commit_index.write().await;
            *ci = (*ci).max(index);
            self.apply_committed().await?;
            Ok(())
        } else {
            Err(crate::Error::ClusterError(
                "Failed to replicate to quorum".into(),
            ))
        }
    }

    /// Deliver committed entries from `last_applied` up to `commit_index` to
    /// the apply channel, advancing `last_applied` as each entry is handed off.
    ///
    /// Invoked by the leader after an entry reaches quorum and by followers
    /// whenever the leader advances their commit index.
    async fn apply_committed(&self) -> Result<()> {
        let ci = *self.commit_index.read().await;
        let mut la = self.last_applied.write().await;
        let log = self.log.read().await;

        while *la < ci && (*la as usize) < log.len() {
            let entry_data = log[*la as usize].clone();
            if self.apply_tx.send(entry_data).is_err() {
                warn!("Apply channel closed");
                break;
            }
            *la += 1;
        }
        Ok(())
    }

    /// Handle an incoming `RequestVote` RPC from a candidate.
    ///
    /// Grants the vote when the candidate's term and log are at least as fresh
    /// as this node's and no vote has been cast yet in the term.
    pub async fn handle_request_vote(&self, req: &RaftVoteRequest) -> Result<RaftVoteResponse> {
        let mut state = self.state.write().await;
        let last_log_index = self.last_log_index().await;
        let last_log_term = self.last_log_term().await;

        let mut vote_granted = false;

        if req.term > state.term {
            state.term = req.term;
            state.role = ConsensusRole::Follower;
            state.voted_for = None;
            state.leader_id = None;
        }

        if req.term >= state.term
            && (state.voted_for.is_none() || state.voted_for.as_deref() == Some(&req.candidate_id))
            && req.last_log_term >= last_log_term
            && req.last_log_index >= last_log_index
        {
            vote_granted = true;
            state.voted_for = Some(req.candidate_id.clone());
            state.term = req.term;
        }

        Ok(RaftVoteResponse {
            term: state.term,
            vote_granted,
            node_id: self.node_id.clone(),
        })
    }

    /// Handle an incoming `AppendEntries` RPC (or heartbeat) from the leader.
    ///
    /// Verifies log consistency at `prev_log_index`, appends any new entries,
    /// truncating conflicting entries, and advances the commit index.
    pub async fn handle_append_entries(
        &self,
        req: &RaftAppendRequest,
    ) -> Result<RaftAppendResponse> {
        {
            let mut state = self.state.write().await;
            if req.term >= state.term {
                state.term = req.term;
                state.role = ConsensusRole::Follower;
                state.leader_id = Some(req.leader_id.clone());
                *self.leader_hint.write().await = Some(req.leader_id.clone());
            }
        }

        let state = self.state.read().await;
        if req.term < state.term {
            return Ok(RaftAppendResponse {
                term: state.term,
                success: false,
                match_index: 0,
                node_id: self.node_id.clone(),
                last_log_index: self.last_log_index().await,
            });
        }

        let log = self.log.write().await;
        if req.prev_log_index > 0 && (req.prev_log_index as usize) > log.len() {
            return Ok(RaftAppendResponse {
                term: state.term,
                success: false,
                match_index: 0,
                node_id: self.node_id.clone(),
                last_log_index: log.len() as u64,
            });
        }

        if req.prev_log_index > 0 {
            let prev_idx = req.prev_log_index as usize - 1;
            if prev_idx < log.len() {
                let prev: Vec<u8> = log[prev_idx].clone();
                drop(log);
                if let Ok(prev_entry) = deserialize_log_entry(&prev) {
                    if prev_entry.term != req.prev_log_term {
                        let mut log = self.log.write().await;
                        log.truncate(req.prev_log_index as usize);
                    }
                }
            }
        }

        let mut log = self.log.write().await;
        for entry_data in &req.entries {
            log.push(entry_data.clone());
        }

        let commit = req.leader_commit.min(log.len() as u64);
        *self.commit_index.write().await = commit;
        drop(log);

        self.apply_committed().await?;

        let log_len = self.log.read().await.len() as u64;
        Ok(RaftAppendResponse {
            term: state.term,
            success: true,
            match_index: log_len,
            node_id: self.node_id.clone(),
            last_log_index: log_len,
        })
    }

    /// Handle an incoming `InstallSnapshot` RPC from the leader.
    ///
    /// Streams snapshot data (either replacing or appending to the in-progress
    /// snapshot), then on the final chunk (`done`) replaces the log with the
    /// snapshot's included index.
    pub async fn handle_install_snapshot(&self, req: &InstallSnapshotRequest) -> Result<()> {
        {
            let mut state = self.state.write().await;
            if req.term >= state.term {
                state.term = req.term;
                state.role = ConsensusRole::Follower;
                state.leader_id = Some(req.leader_id.clone());
            }
        }

        let mut snapshot = self.snapshot.write().await;
        let mut existing = snapshot.take().unwrap_or(RaftSnapshot {
            last_included_index: 0,
            last_included_term: 0,
            data: Vec::new(),
        });

        if req.offset == 0 {
            existing = RaftSnapshot {
                last_included_index: req.last_included_index,
                last_included_term: req.last_included_term,
                data: req.data.clone(),
            };
        } else {
            existing.data.extend_from_slice(&req.data);
        }

        if req.done {
            let mut log = self.log.write().await;
            log.retain(|_| false);
            let ci = existing.last_included_index;
            *self.commit_index.write().await = ci;
            *self.last_applied.write().await = ci;
        }

        *snapshot = Some(existing);
        Ok(())
    }
}

/// Decode a bincode-serialized [`LogEntry`] stored in the replicated log.
fn deserialize_log_entry(data: &[u8]) -> Result<LogEntry> {
    bincode::deserialize(data)
        .map_err(|e| crate::Error::ClusterError(format!("LogEntry deserialize: {}", e)))
}

/// Serialize a cluster operation into a bincode-encoded [`LogEntry`] that can be
/// appended to the Raft log.
pub fn create_log_entry(term: u64, op_type: &str, data: serde_json::Value) -> Vec<u8> {
    let entry = LogEntry {
        index: 0,
        term,
        op_type: op_type.to_string(),
        data,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };
    bincode::serialize(&entry).unwrap_or_default()
}
