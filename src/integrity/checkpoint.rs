//! Signed integrity checkpoints.
//!
//! A checkpoint compresses a range of the per-database hash chain into a
//! Merkle root, signed by the node. In anchored modes the checkpoint root is
//! what gets submitted to the ledger, so a large number of small mutations
//! never maps to one ledger transaction each.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::errors::{IntegrityError, IntegrityResult};
use super::merkle::merkle_root;
use super::record::{IntegrityRecord, LedgerState};
use super::signing::SigningService;

/// A signed Merkle-root checkpoint over a contiguous run of records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Database this checkpoint belongs to.
    pub database_id: String,
    /// Unique checkpoint identifier (uuid).
    pub checkpoint_id: String,
    /// Sequence of the last record included in this checkpoint.
    pub end_sequence: u64,
    /// Sequence of the first record included (after the previous checkpoint).
    pub start_sequence: u64,
    /// Hash of the previous checkpoint ("genesis" for the first).
    pub previous_checkpoint_hash: String,
    /// Merkle root over the included records' hashes.
    pub merkle_root: String,
    /// Number of records covered.
    pub record_count: u64,
    /// Creation timestamp.
    pub timestamp: DateTime<Utc>,
    /// Signer identity.
    pub signer_id: String,
    /// Hex public key of the signer.
    pub signer_public_key: String,
    /// Base64 signature over [`Checkpoint::canonical_bytes`].
    pub signature: String,
    /// Ledger anchoring state.
    pub ledger_state: LedgerState,
    /// Ledger transaction id when anchored.
    pub ledger_tx_id: Option<String>,
    /// Hash of this checkpoint (excludes signature).
    pub checkpoint_hash: String,
}

impl Checkpoint {
    /// Builds a signed checkpoint over `records` linking to
    /// `previous_checkpoint_hash`.
    pub fn create(
        database_id: &str,
        records: &[IntegrityRecord],
        previous_checkpoint_hash: &str,
        signer: &SigningService,
    ) -> IntegrityResult<Checkpoint> {
        if records.is_empty() {
            return Err(IntegrityError::PolicyViolation(
                "cannot create an empty checkpoint".to_string(),
            ));
        }
        let leaf_hashes: Vec<String> = records.iter().map(|r| r.record_hash.clone()).collect();
        let start_sequence = records
            .first()
            .map(|r| r.sequence)
            .ok_or_else(|| IntegrityError::Internal("empty records".to_string()))?;
        let end_sequence = records
            .last()
            .map(|r| r.sequence)
            .ok_or_else(|| IntegrityError::Internal("empty records".to_string()))?;

        let mut cp = Checkpoint {
            database_id: database_id.to_string(),
            checkpoint_id: uuid::Uuid::new_v4().to_string(),
            end_sequence,
            start_sequence,
            previous_checkpoint_hash: previous_checkpoint_hash.to_string(),
            merkle_root: merkle_root(&leaf_hashes),
            record_count: records.len() as u64,
            timestamp: Utc::now(),
            signer_id: signer.signer_id().to_string(),
            signer_public_key: signer.public_key_hex().to_string(),
            signature: String::new(),
            ledger_state: LedgerState::None,
            ledger_tx_id: None,
            checkpoint_hash: String::new(),
        };
        cp.checkpoint_hash = cp.compute_hash();
        cp.signature = signer.sign(&cp.canonical_bytes())?;
        Ok(cp)
    }

    /// Canonical bytes signed/verified by the signature.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut value = serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}));
        if let Some(obj) = value.as_object_mut() {
            obj.remove("signature");
            obj.remove("checkpoint_hash");
        }
        serde_json::to_vec(&value).unwrap_or_default()
    }

    /// Computes the checkpoint hash over canonical bytes.
    pub fn compute_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(self.canonical_bytes()))
    }

    /// Verifies the signature and hash integrity.
    pub fn verify(&self) -> IntegrityResult<bool> {
        if self.checkpoint_hash != self.compute_hash() {
            return Ok(false);
        }
        SigningService::verify(
            &self.signer_public_key,
            &self.canonical_bytes(),
            &self.signature,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::record::{payload_digest, IntegrityRecord, NewRecord};
    use tempfile::tempdir;

    fn signer() -> SigningService {
        let dir = tempdir().unwrap();
        SigningService::load_or_create("node-1", Some(&dir.path().join("k"))).unwrap()
    }

    fn records(n: u64) -> Vec<IntegrityRecord> {
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
                &s,
            )
            .unwrap();
            prev = r.record_hash.clone();
            out.push(r);
        }
        out
    }

    #[test]
    fn test_checkpoint_signature_verifies() {
        let cp = Checkpoint::create("db-1", &records(5), "genesis", &signer()).unwrap();
        assert!(cp.verify().unwrap());
        assert_eq!(cp.record_count, 5);
        assert_eq!(cp.end_sequence, 5);
        assert_eq!(cp.start_sequence, 1);
    }

    #[test]
    fn test_checkpoint_tamper_detected() {
        let mut cp = Checkpoint::create("db-1", &records(5), "genesis", &signer()).unwrap();
        cp.merkle_root = "0".repeat(64);
        assert!(!cp.verify().unwrap());
    }

    #[test]
    fn test_checkpoint_empty_rejected() {
        assert!(Checkpoint::create("db-1", &[], "genesis", &signer()).is_err());
    }

    #[test]
    fn test_checkpoint_merkle_changes_with_content() {
        let a = Checkpoint::create("db-1", &records(5), "genesis", &signer()).unwrap();
        let b = Checkpoint::create("db-1", &records(6), "genesis", &signer()).unwrap();
        assert_ne!(a.merkle_root, b.merkle_root);
    }
}
