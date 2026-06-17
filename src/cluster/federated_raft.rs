use crate::cluster::rpc::{
    FedRaftAppendRequest, FedRaftAppendResponse, FedRaftLogEntry, FedRaftVoteRequest,
    FedRaftVoteResponse, RpcMessage,
};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FedRaftRole {
    Follower,
    Candidate,
    Leader,
}

#[derive(Debug, Clone)]
pub struct FedRaftState {
    pub role: FedRaftRole,
    pub current_term: u64,
    pub voted_for: Option<String>,
    pub leader_id: Option<String>,
    pub commit_index: u64,
    pub last_applied: u64,
}

#[derive(Debug, Clone)]
pub struct FedRaftConfig {
    pub cluster_id: String,
    pub heartbeat_interval_ms: u64,
    pub election_timeout_ms: u64,
    pub max_log_entries: usize,
}

impl Default for FedRaftConfig {
    fn default() -> Self {
        Self {
            cluster_id: String::new(),
            heartbeat_interval_ms: 1500,
            election_timeout_ms: 5000,
            max_log_entries: 10000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FedRaftOpType {
    DomainCreate,
    DomainUpdate,
    DomainDelete,
    ClusterJoin,
    ClusterLeave,
    NamespaceRegister,
    MetadataUpdate,
}

pub struct FederatedRaft {
    pub config: FedRaftConfig,
    pub state: RwLock<FedRaftState>,
    pub log: RwLock<Vec<FedRaftLogEntry>>,
    pub cluster_peers: RwLock<Vec<String>>,
    pub election_reset: RwLock<u64>,
}

impl FederatedRaft {
    pub fn new(config: FedRaftConfig) -> Self {
        Self {
            state: RwLock::new(FedRaftState {
                role: FedRaftRole::Follower,
                current_term: 0,
                voted_for: None,
                leader_id: None,
                commit_index: 0,
                last_applied: 0,
            }),
            log: RwLock::new(Vec::new()),
            cluster_peers: RwLock::new(Vec::new()),
            election_reset: RwLock::new(0),
            config,
        }
    }

    pub async fn become_follower(&self, term: u64, leader: Option<&str>) {
        let mut state = self.state.write().await;
        if term > state.current_term {
            state.current_term = term;
            state.voted_for = None;
        }
        state.role = FedRaftRole::Follower;
        state.leader_id = leader.map(String::from);
        *self.election_reset.write().await = now_ms();
    }

    pub async fn become_candidate(&self) {
        let mut state = self.state.write().await;
        state.current_term += 1;
        state.role = FedRaftRole::Candidate;
        state.voted_for = Some(self.config.cluster_id.clone());
        state.leader_id = None;
        *self.election_reset.write().await = now_ms();
        debug!(
            "FedRaft: {} became candidate for term {}",
            self.config.cluster_id, state.current_term
        );
    }

    pub async fn become_leader(&self) {
        let mut state = self.state.write().await;
        state.role = FedRaftRole::Leader;
        state.leader_id = Some(self.config.cluster_id.clone());
        info!(
            "FedRaft: {} became leader for term {}",
            self.config.cluster_id, state.current_term
        );
    }

    pub async fn start_election(&self, peers: &[FedRaftPeer]) {
        self.become_candidate().await;
        let term = self.state.read().await.current_term;
        let log = self.log.read().await;
        let last_log_index = log.len() as u64;
        let last_log_term = if last_log_index > 0 {
            log[last_log_index as usize - 1].term
        } else {
            0
        };
        drop(log);

        let req = FedRaftVoteRequest {
            term,
            candidate_id: self.config.cluster_id.clone(),
            last_log_index,
            last_log_term,
        };

        let mut votes = 1;
        let quorum = peers.len() / 2 + 1;

        for peer in peers {
            if let Ok(Some(RpcMessage::FedRaftVoteResponse(resp))) =
                crate::cluster::federation::connect_and_send(
                    &peer.address,
                    peer.port,
                    RpcMessage::FedRaftVoteRequest(req.clone()),
                )
                .await
            {
                if resp.vote_granted {
                    votes += 1;
                }
                if resp.term > term {
                    self.become_follower(resp.term, None).await;
                    return;
                }
            }
        }

        if votes >= quorum {
            self.become_leader().await;
            self.send_heartbeats(peers).await;
        } else {
            self.become_follower(term, None).await;
            warn!(
                "FedRaft: election lost ({} votes, needed {})",
                votes, quorum
            );
        }
    }

    pub async fn send_heartbeats(&self, peers: &[FedRaftPeer]) {
        let state = self.state.read().await;
        let log = self.log.read().await;
        let term = state.current_term;
        let commit = state.commit_index;
        let prev_log_index = log.len() as u64;
        let prev_log_term = if prev_log_index > 0 {
            log[prev_log_index as usize - 1].term
        } else {
            0
        };
        drop(state);
        drop(log);

        let req = FedRaftAppendRequest {
            term,
            leader_id: self.config.cluster_id.clone(),
            prev_log_index,
            prev_log_term,
            entries: vec![],
            leader_commit: commit,
        };

        for peer in peers {
            let rpc_msg = RpcMessage::FedRaftAppendEntries(req.clone());
            let _ = crate::cluster::federation::connect_and_send(&peer.address, peer.port, rpc_msg)
                .await;
        }
    }

    pub async fn propose(
        &self,
        op_type: FedRaftOpType,
        data: Vec<u8>,
        peers: &[FedRaftPeer],
    ) -> Result<bool> {
        let state = self.state.read().await;
        if state.role != FedRaftRole::Leader {
            return Err(crate::Error::ClusterError("FedRaft: not the leader".into()));
        }
        let term = state.current_term;
        drop(state);

        let mut log = self.log.write().await;
        let index = log.len() as u64 + 1;
        let entry = FedRaftLogEntry {
            index,
            term,
            op_type: format!("{:?}", op_type),
            data,
            timestamp: now_ms(),
        };
        log.push(entry.clone());
        let prev_log_index = index - 1;
        let prev_log_term = if prev_log_index > 0 {
            log[prev_log_index as usize - 1].term
        } else {
            0
        };
        drop(log);

        let req = FedRaftAppendRequest {
            term,
            leader_id: self.config.cluster_id.clone(),
            prev_log_index,
            prev_log_term,
            entries: vec![entry],
            leader_commit: 0,
        };

        let mut successes = 1;
        let quorum = peers.len() / 2 + 1;

        for peer in peers {
            if let Ok(Some(RpcMessage::FedRaftAppendEntriesResponse(resp))) =
                crate::cluster::federation::connect_and_send(
                    &peer.address,
                    peer.port,
                    RpcMessage::FedRaftAppendEntries(req.clone()),
                )
                .await
            {
                if resp.success {
                    successes += 1;
                }
                if resp.term > term {
                    self.become_follower(resp.term, None).await;
                    return Err(crate::Error::ClusterError(
                        "FedRaft: stale leader detected".into(),
                    ));
                }
            }
        }

        if successes >= quorum {
            let mut state = self.state.write().await;
            state.commit_index = index;
            state.last_applied = index;
            info!(
                "FedRaft: committed entry {} (term {}, op={:?})",
                index, term, op_type
            );
            Ok(true)
        } else {
            warn!(
                "FedRaft: proposal failed ({} successes, needed {})",
                successes, quorum
            );
            Ok(false)
        }
    }

    pub async fn handle_vote_request(&self, req: &FedRaftVoteRequest) -> FedRaftVoteResponse {
        let mut state = self.state.write().await;

        if req.term > state.current_term {
            state.current_term = req.term;
            state.voted_for = None;
            state.role = FedRaftRole::Follower;
        }

        let log = self.log.read().await;
        let last_log_index = log.len() as u64;
        let last_log_term = if last_log_index > 0 {
            log[last_log_index as usize - 1].term
        } else {
            0
        };
        drop(log);

        let log_ok = req.last_log_term > last_log_term
            || (req.last_log_term == last_log_term && req.last_log_index >= last_log_index);

        let grant = req.term >= state.current_term
            && (state.voted_for.is_none() || state.voted_for.as_deref() == Some(&req.candidate_id))
            && log_ok;

        if grant {
            state.voted_for = Some(req.candidate_id.clone());
            *self.election_reset.write().await = now_ms();
        }

        FedRaftVoteResponse {
            term: state.current_term,
            vote_granted: grant,
            cluster_id: self.config.cluster_id.clone(),
        }
    }

    pub async fn handle_append_entries(&self, req: &FedRaftAppendRequest) -> FedRaftAppendResponse {
        let mut state = self.state.write().await;

        if req.term >= state.current_term {
            state.current_term = req.term;
            state.role = FedRaftRole::Follower;
            state.leader_id = Some(req.leader_id.clone());
            *self.election_reset.write().await = now_ms();
        }

        if req.term < state.current_term {
            return FedRaftAppendResponse {
                term: state.current_term,
                success: false,
                match_index: state.commit_index,
                cluster_id: self.config.cluster_id.clone(),
            };
        }

        let mut log = self.log.write().await;

        // Check log consistency
        if req.prev_log_index > 0 {
            if req.prev_log_index > log.len() as u64 {
                return FedRaftAppendResponse {
                    term: state.current_term,
                    success: false,
                    match_index: state.commit_index,
                    cluster_id: self.config.cluster_id.clone(),
                };
            }
            if req.prev_log_index > 0 && req.prev_log_index <= log.len() as u64 {
                let idx = req.prev_log_index as usize - 1;
                if log[idx].term != req.prev_log_term {
                    log.truncate(idx);
                }
            }
        }

        // Append new entries
        for entry in &req.entries {
            if entry.index > log.len() as u64 {
                log.push(entry.clone());
            }
        }

        // Update commit index
        if req.leader_commit > state.commit_index {
            let new_commit = req.leader_commit.min(log.len() as u64);
            state.commit_index = new_commit;
            state.last_applied = new_commit;
        }

        let match_index = log.len() as u64;

        FedRaftAppendResponse {
            term: state.current_term,
            success: true,
            match_index,
            cluster_id: self.config.cluster_id.clone(),
        }
    }

    pub async fn is_leader(&self) -> bool {
        self.state.read().await.role == FedRaftRole::Leader
    }

    pub async fn leader_id(&self) -> Option<String> {
        self.state.read().await.leader_id.clone()
    }

    pub async fn current_term(&self) -> u64 {
        self.state.read().await.current_term
    }

    pub async fn get_committed_entries(&self) -> Vec<FedRaftLogEntry> {
        let state = self.state.read().await;
        let log = self.log.read().await;
        log.iter()
            .take_while(|e| e.index <= state.last_applied)
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct FedRaftPeer {
    pub cluster_id: String,
    pub address: String,
    pub port: u16,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
