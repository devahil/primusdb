use crate::Result;
use crate::cluster::rpc::{
    RpcClient, RpcMessage, RaftAppendRequest, RaftAppendResponse, RaftVoteRequest,
    RaftVoteResponse, InstallSnapshotRequest,
};
use crate::cluster::sync::consensus::{ConsensusConfig, ConsensusRole, ConsensusState, LogEntry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, mpsc, Mutex};
use tracing::{debug, info, warn};

pub type LogStore = Vec<Vec<u8>>;

pub struct RaftNode {
    pub node_id: String,
    pub state: RwLock<ConsensusState>,
    pub config: ConsensusConfig,
    pub peers: RwLock<HashMap<String, Arc<RpcClient>>>,
    pub log: RwLock<Vec<Vec<u8>>>,
    pub commit_index: RwLock<u64>,
    pub last_applied: RwLock<u64>,
    pub snapshot: RwLock<Option<RaftSnapshot>>,
    pub apply_tx: mpsc::UnboundedSender<Vec<u8>>,
    pub leader_hint: RwLock<Option<String>>,
    #[allow(dead_code)]
    election_reset: Mutex<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftSnapshot {
    pub last_included_index: u64,
    pub last_included_term: u64,
    pub data: Vec<u8>,
}

impl RaftNode {
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
            election_reset: Mutex::new(()),
        }
    }

    pub async fn current_term(&self) -> u64 {
        self.state.read().await.term
    }

    pub async fn is_leader(&self) -> bool {
        matches!(self.state.read().await.role, ConsensusRole::Leader)
    }

    pub async fn leader_id(&self) -> Option<String> {
        self.state.read().await.leader_id.clone()
    }

    pub async fn last_log_index(&self) -> u64 {
        self.log.read().await.len() as u64
    }

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

    pub async fn append_entry(&self, data: Vec<u8>) -> u64 {
        let mut log = self.log.write().await;
        let index = log.len() as u64 + 1;
        log.push(data);
        index
    }

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

    pub async fn become_candidate(&self) {
        let mut state = self.state.write().await;
        state.term += 1;
        state.role = ConsensusRole::Candidate;
        state.voted_for = Some(self.node_id.clone());
        state.leader_id = None;
    }

    pub async fn become_leader(&self) {
        let mut state = self.state.write().await;
        state.role = ConsensusRole::Leader;
        state.leader_id = Some(self.node_id.clone());
        *self.leader_hint.write().await = Some(self.node_id.clone());
    }

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

        let quorum = (peers.len() + 1) / 2 + 1;
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

    pub async fn replicate_entry(&self, entry_data: Vec<u8>) -> Result<()> {
        let index = self.append_entry(entry_data.clone()).await;
        let term = self.state.read().await.term;
        let peers = self.peers.read().await;
        let log = self.log.read().await;
        let prev_log_index = if index > 1 { index - 1 } else { 0 };
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
            match client.send(&req).await {
                Ok(RpcMessage::AppendEntriesResponse(resp)) => {
                    if resp.success {
                        successes += 1;
                    } else if resp.term > term {
                        self.become_follower(resp.term, None).await;
                        return Err(crate::Error::ClusterError("Stale leader".into()));
                    }
                }
                _ => {}
            }
        }

        let quorum = (peers.len() + 1) / 2 + 1;
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

    pub async fn handle_request_vote(
        &self,
        req: &RaftVoteRequest,
    ) -> Result<RaftVoteResponse> {
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

    pub async fn handle_install_snapshot(
        &self,
        req: &InstallSnapshotRequest,
    ) -> Result<()> {
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

fn deserialize_log_entry(data: &[u8]) -> Result<LogEntry> {
    bincode::deserialize(data)
        .map_err(|e| crate::Error::ClusterError(format!("LogEntry deserialize: {}", e)))
}

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
