//! Reconciliation of integrity chains across replicas.
//!
//! Reconciliation compares local integrity records against a peer's records,
//! using sequence numbers, previous-hash links and record hashes (never the
//! raw data) to detect divergence. The verdict drives a repair plan executed
//! by the operator or cluster manager; nothing is applied automatically unless
//! it passes integrity verification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::errors::IntegrityResult;
use super::record::IntegrityRecord;

/// Verdict of a reconciliation pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconciliationVerdict {
    /// Both chains match exactly.
    InSync,
    /// Local chain is missing transactions that the peer has.
    LocalBehind,
    /// Peer is missing transactions that the local chain has.
    PeerBehind,
    /// One or more transactions conflict at the same sequence.
    Conflicting,
    /// The peer chain contains invalid signatures or broken links.
    InvalidPeer,
    /// Local chain is invalid.
    InvalidLocal,
}

impl std::fmt::Display for ReconciliationVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReconciliationVerdict::InSync => write!(f, "in-sync"),
            ReconciliationVerdict::LocalBehind => write!(f, "local-behind"),
            ReconciliationVerdict::PeerBehind => write!(f, "peer-behind"),
            ReconciliationVerdict::Conflicting => write!(f, "conflicting"),
            ReconciliationVerdict::InvalidPeer => write!(f, "invalid-peer"),
            ReconciliationVerdict::InvalidLocal => write!(f, "invalid-local"),
        }
    }
}

/// Result of reconciling the local chain with a peer's chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationReport {
    pub database_id: String,
    pub verdict: ReconciliationVerdict,
    pub local_sequences: u64,
    pub peer_sequences: u64,
    /// Sequences present on the peer but missing locally.
    pub missing_on_local: Vec<u64>,
    /// Sequences present locally but missing on the peer.
    pub missing_on_peer: Vec<u64>,
    /// Sequences where local and peer hashes disagree.
    pub conflicts: Vec<u64>,
    /// Transaction ids that appear on both sides with different hashes.
    pub conflicting_txns: Vec<String>,
    /// Whether the peer chain is internally valid (signatures + links).
    pub peer_chain_valid: bool,
    /// Whether the local chain is internally valid (signatures + links).
    pub local_chain_valid: bool,
    pub reconciled_at: DateTime<Utc>,
}

impl ReconciliationReport {
    /// True when the report indicates no repair is needed.
    pub fn is_in_sync(&self) -> bool {
        self.verdict == ReconciliationVerdict::InSync
    }
}

/// Compact chain evidence exchanged between peers before a full reconciliation.
///
/// This is the "integrity-first" handshake: nodes compare lengths, last hashes
/// and checkpoint Merkle roots *before* transferring records, so a full record
/// exchange only happens when the evidence differs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainEvidence {
    /// Database identifier (genesis database id when available).
    pub database_id: String,
    /// Number of integrity records in the chain.
    pub sequence_count: u64,
    /// Hash of the last record (or "genesis" for an empty chain).
    pub last_hash: String,
    /// Merkle root of the most recent checkpoint (or "none").
    pub checkpoint_root: String,
    /// Node that produced this evidence.
    pub node_id: String,
    /// Wall-clock time the evidence was produced.
    pub produced_at: DateTime<Utc>,
}

/// Compares the local chain against a peer chain without touching storage.
pub fn compare_chains(
    database_id: &str,
    local: &[IntegrityRecord],
    peer: &[IntegrityRecord],
) -> IntegrityResult<ReconciliationReport> {
    let peer_chain_valid = verify_chain(peer)?;
    let local_chain_valid = verify_chain(local)?;

    let local_by_seq: std::collections::HashMap<u64, &IntegrityRecord> =
        local.iter().map(|r| (r.sequence, r)).collect();
    let peer_by_seq: std::collections::HashMap<u64, &IntegrityRecord> =
        peer.iter().map(|r| (r.sequence, r)).collect();

    let mut missing_on_local = Vec::new();
    let mut missing_on_peer = Vec::new();
    let mut conflicts = Vec::new();
    let mut conflicting_txns = Vec::new();

    let all_seqs: std::collections::BTreeSet<u64> = local_by_seq
        .keys()
        .chain(peer_by_seq.keys())
        .copied()
        .collect();

    for seq in &all_seqs {
        match (local_by_seq.get(seq), peer_by_seq.get(seq)) {
            (Some(l), Some(p)) => {
                if l.record_hash != p.record_hash {
                    conflicts.push(*seq);
                    if l.transaction_id == p.transaction_id {
                        conflicting_txns.push(l.transaction_id.clone());
                    }
                }
            }
            (None, Some(_)) => missing_on_local.push(*seq),
            (Some(_), None) => missing_on_peer.push(*seq),
            (None, None) => {}
        }
    }

    let verdict = if !local_chain_valid {
        ReconciliationVerdict::InvalidLocal
    } else if !peer_chain_valid {
        ReconciliationVerdict::InvalidPeer
    } else if !conflicts.is_empty() {
        ReconciliationVerdict::Conflicting
    } else if !missing_on_local.is_empty() {
        ReconciliationVerdict::LocalBehind
    } else if !missing_on_peer.is_empty() {
        ReconciliationVerdict::PeerBehind
    } else {
        ReconciliationVerdict::InSync
    };

    Ok(ReconciliationReport {
        database_id: database_id.to_string(),
        verdict,
        local_sequences: local.len() as u64,
        peer_sequences: peer.len() as u64,
        missing_on_local,
        missing_on_peer,
        conflicts,
        conflicting_txns,
        peer_chain_valid,
        local_chain_valid,
        reconciled_at: Utc::now(),
    })
}

/// Verifies that a chain is internally consistent: every record verifies its
/// signature and hash, sequences are contiguous, and previous hashes link.
pub fn verify_chain(records: &[IntegrityRecord]) -> IntegrityResult<bool> {
    let mut prev_hash = "genesis".to_string();
    for (index, r) in records.iter().enumerate() {
        let expected_seq = index as u64 + 1;
        if r.sequence != expected_seq {
            return Ok(false);
        }
        if r.previous_hash != prev_hash {
            return Ok(false);
        }
        if !r.verify()? {
            return Ok(false);
        }
        prev_hash = r.record_hash.clone();
    }
    Ok(true)
}

/// Builds a repair plan from a report: sequences to fetch or reject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairPlan {
    pub database_id: String,
    /// Sequences to fetch from the peer (safe to replay).
    pub fetch_from_peer: Vec<u64>,
    /// Sequences to reject (conflicts or invalid).
    pub reject: Vec<u64>,
    pub requires_operator: bool,
}

pub fn plan_repair(report: &ReconciliationReport) -> RepairPlan {
    let requires_operator = report.verdict == ReconciliationVerdict::Conflicting
        || report.verdict == ReconciliationVerdict::InvalidPeer
        || report.verdict == ReconciliationVerdict::InvalidLocal;
    RepairPlan {
        database_id: report.database_id.clone(),
        fetch_from_peer: report.missing_on_local.clone(),
        reject: report.conflicts.clone(),
        requires_operator,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::record::{payload_digest, IntegrityRecord, NewRecord};
    use crate::integrity::signing::SigningService;
    use std::sync::OnceLock;

    fn signer() -> &'static SigningService {
        static SIGNER: OnceLock<SigningService> = OnceLock::new();
        SIGNER.get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            SigningService::load_or_create("node-1", Some(&dir.path().join("k"))).unwrap()
        })
    }

    fn chain(n: u64) -> Vec<IntegrityRecord> {
        let s = signer();
        let mut out = Vec::new();
        let mut prev = "genesis".to_string();
        for i in 1..=n {
            let r = IntegrityRecord::create(
                NewRecord {
                    transaction_id: &format!("tx-{}", i),
                    database_id: "db-1",
                    namespace: None,
                    engine_type: "relational",
                    node_id: "node-1",
                    cluster_id: None,
                    operation: "insert",
                    affected_objects: &[],
                    payload_digest: &payload_digest(&serde_json::json!({"i": i})),
                    metadata_digest: &payload_digest(&serde_json::json!({})),
                    sequence: i,
                },
                &prev,
                s,
            )
            .unwrap();
            prev = r.record_hash.clone();
            out.push(r);
        }
        out
    }

    #[test]
    fn test_in_sync() {
        let a = chain(5);
        let report = compare_chains("db-1", &a, &a).unwrap();
        assert_eq!(report.verdict, ReconciliationVerdict::InSync);
        assert!(report.is_in_sync());
    }

    #[test]
    fn test_local_behind() {
        let peer = chain(5);
        let local = &peer[..3];
        let report = compare_chains("db-1", local, &peer).unwrap();
        assert_eq!(report.verdict, ReconciliationVerdict::LocalBehind);
        assert_eq!(report.missing_on_local, vec![4, 5]);
    }

    #[test]
    fn test_conflict_detected() {
        let a = chain(3);
        let mut b = a.clone();
        // Tamper with the last record in the copy -> conflicting hash at seq 3.
        b[2].payload_digest = "tampered".to_string();
        b[2].re_sign(signer()).unwrap();
        let report = compare_chains("db-1", &b, &a).unwrap();
        assert_eq!(report.verdict, ReconciliationVerdict::Conflicting);
        assert_eq!(report.conflicts, vec![3]);
    }

    #[test]
    fn test_invalid_peer_chain() {
        let a = chain(5);
        let mut b = a.clone();
        b[1].sequence = 99; // break contiguity
        let report = compare_chains("db-1", &a, &b).unwrap();
        assert_eq!(report.verdict, ReconciliationVerdict::InvalidPeer);
        assert!(!report.peer_chain_valid);
        assert!(report.local_chain_valid);
    }

    #[test]
    fn test_invalid_local_chain() {
        let a = chain(5);
        let mut local = a.clone();
        local[1].previous_hash = "0".repeat(64);
        let report = compare_chains("db-1", &local, &a).unwrap();
        assert_eq!(report.verdict, ReconciliationVerdict::InvalidLocal);
        assert!(!report.local_chain_valid);
    }

    #[test]
    fn test_verify_chain_ok_and_broken() {
        assert!(verify_chain(&chain(4)).unwrap());
        let mut c = chain(4);
        c[2].previous_hash = "0".repeat(64);
        assert!(!verify_chain(&c).unwrap());
    }

    #[test]
    fn test_repair_plan() {
        let peer = chain(5);
        let local = &peer[..3];
        let report = compare_chains("db-1", local, &peer).unwrap();
        let plan = plan_repair(&report);
        assert_eq!(plan.fetch_from_peer, vec![4, 5]);
        assert!(!plan.requires_operator);
    }
}
