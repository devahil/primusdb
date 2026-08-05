//! Transaction integrity records.
//!
//! Every committed mutation produces an [`IntegrityRecord`] that is chained
//! per database: each record references the previous record's hash and is
//! signed by the node. Replaying, reordering, or editing any record breaks the
//! chain and is detected by [`crate::integrity::IntegrityService::verify_chain`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::errors::IntegrityResult;
use super::signing::SigningService;

/// State of the record with respect to external ledger anchoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LedgerState {
    /// No ledger anchoring applies to this record.
    #[default]
    None,
    /// Submitted for asynchronous anchoring, awaiting confirmation.
    Pending,
    /// Confirmed on the ledger.
    Confirmed,
    /// Submission failed and is being retried.
    Failed,
    /// Rejected by the ledger (invalid proof).
    Rejected,
}

impl std::fmt::Display for LedgerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerState::None => write!(f, "none"),
            LedgerState::Pending => write!(f, "pending"),
            LedgerState::Confirmed => write!(f, "confirmed"),
            LedgerState::Failed => write!(f, "failed"),
            LedgerState::Rejected => write!(f, "rejected"),
        }
    }
}

/// Reconciliation status of the record across replicas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ReconciliationStatus {
    #[default]
    None,
    Matched,
    Divergent,
    Quarantined,
}

/// A signed, chained record of a committed mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityRecord {
    /// Client/transaction identifier.
    pub transaction_id: String,
    /// Database identity this record belongs to (genesis database_id or path).
    pub database_id: String,
    /// Optional namespace.
    pub namespace: Option<String>,
    /// Storage engine that executed the mutation.
    pub engine_type: String,
    /// Node that executed the mutation.
    pub node_id: String,
    /// Cluster identity when running clustered.
    pub cluster_id: Option<String>,
    /// Wall-clock mutation time.
    pub timestamp: DateTime<Utc>,
    /// Monotonic per-database sequence (1-based after genesis at 0).
    pub sequence: u64,
    /// Hash of the previous record in the chain ("genesis" for the first).
    pub previous_hash: String,
    /// Operation type (insert/update/delete/create/drop/...).
    pub operation: String,
    /// Affected object identifiers (table, collection, key set).
    pub affected_objects: Vec<String>,
    /// SHA-256 of the payload (row, vector, document).
    pub payload_digest: String,
    /// SHA-256 of any metadata (conditions, schema delta).
    pub metadata_digest: String,
    /// Signer identity.
    pub signer_id: String,
    /// Hex public key of the signer.
    pub signer_public_key: String,
    /// Base64 signature over [`IntegrityRecord::canonical_bytes`].
    pub signature: String,
    /// Ledger anchoring state.
    pub ledger_state: LedgerState,
    /// Ledger transaction id when submitted.
    pub ledger_tx_id: Option<String>,
    /// Reconciliation status.
    pub reconciliation_status: ReconciliationStatus,
    /// Hash of this record (excludes signature and record_hash itself).
    pub record_hash: String,
}

/// Inputs for creating a record.
pub struct NewRecord<'a> {
    pub transaction_id: &'a str,
    pub database_id: &'a str,
    pub namespace: Option<&'a str>,
    pub engine_type: &'a str,
    pub node_id: &'a str,
    pub cluster_id: Option<&'a str>,
    pub operation: &'a str,
    pub affected_objects: &'a [String],
    pub payload_digest: &'a str,
    pub metadata_digest: &'a str,
    pub sequence: u64,
}

impl IntegrityRecord {
    /// Creates a signed record linking to `previous_hash` at the given
    /// sequence.
    pub fn create(
        input: NewRecord<'_>,
        previous_hash: &str,
        signer: &SigningService,
    ) -> IntegrityResult<IntegrityRecord> {
        let mut record = IntegrityRecord {
            transaction_id: input.transaction_id.to_string(),
            database_id: input.database_id.to_string(),
            namespace: input.namespace.map(String::from),
            engine_type: input.engine_type.to_string(),
            node_id: input.node_id.to_string(),
            cluster_id: input.cluster_id.map(String::from),
            timestamp: Utc::now(),
            sequence: input.sequence,
            previous_hash: previous_hash.to_string(),
            operation: input.operation.to_string(),
            affected_objects: input.affected_objects.to_vec(),
            payload_digest: input.payload_digest.to_string(),
            metadata_digest: input.metadata_digest.to_string(),
            signer_id: signer.signer_id().to_string(),
            signer_public_key: signer.public_key_hex().to_string(),
            signature: String::new(),
            ledger_state: LedgerState::default(),
            ledger_tx_id: None,
            reconciliation_status: ReconciliationStatus::default(),
            record_hash: String::new(),
        };
        record.record_hash = record.compute_hash();
        record.signature = signer.sign(&record.canonical_bytes())?;
        Ok(record)
    }

    /// Re-signs the record using the service signer (after any field change).
    pub fn re_sign(&mut self, signer: &SigningService) -> IntegrityResult<()> {
        self.record_hash = self.compute_hash();
        self.signature = signer.sign(&self.canonical_bytes())?;
        Ok(())
    }

    /// Canonical bytes signed/verified by the signature. Excludes the signature
    /// and the derived record_hash.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut value = serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}));
        if let Some(obj) = value.as_object_mut() {
            obj.remove("signature");
            obj.remove("record_hash");
        }
        serde_json::to_vec(&value).unwrap_or_default()
    }

    /// Computes the record hash over canonical bytes.
    pub fn compute_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(self.canonical_bytes()))
    }

    /// Verifies both the embedded signature and that `record_hash` matches.
    pub fn verify(&self) -> IntegrityResult<bool> {
        if self.record_hash != self.compute_hash() {
            return Ok(false);
        }
        SigningService::verify(
            &self.signer_public_key,
            &self.canonical_bytes(),
            &self.signature,
        )
    }
}

/// Compute a payload digest from arbitrary JSON.
pub fn payload_digest(value: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(
        serde_json::to_vec(value).unwrap_or_default(),
    ))
}

/// Compute a metadata digest from arbitrary JSON.
pub fn metadata_digest(value: &serde_json::Value) -> String {
    payload_digest(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn signer() -> SigningService {
        let dir = tempdir().unwrap();
        SigningService::load_or_create("node-1", Some(&dir.path().join("k"))).unwrap()
    }

    fn new_record(prev: &str, tx: &str, seq: u64) -> IntegrityRecord {
        let s = signer();
        IntegrityRecord::create(
            NewRecord {
                transaction_id: tx,
                database_id: "db-1",
                namespace: Some("ns"),
                engine_type: "relational",
                node_id: "node-1",
                cluster_id: None,
                operation: "insert",
                affected_objects: &["users".to_string()],
                payload_digest: &payload_digest(&serde_json::json!({"id": 1})),
                metadata_digest: &metadata_digest(&serde_json::json!({})),
                sequence: seq,
            },
            prev,
            &s,
        )
        .unwrap()
    }

    #[test]
    fn test_record_verify_ok() {
        let r = new_record("genesis", "tx-1", 1);
        assert!(r.verify().unwrap());
    }

    #[test]
    fn test_record_tamper_detected() {
        let mut r = new_record("genesis", "tx-1", 1);
        r.payload_digest = "tampered".to_string();
        assert!(!r.verify().unwrap());
    }

    #[test]
    fn test_previous_hash_links() {
        let r1 = new_record("genesis", "tx-1", 1);
        let r2 = new_record(&r1.record_hash, "tx-2", 2);
        assert_eq!(r2.previous_hash, r1.record_hash);
    }

    #[test]
    fn test_replay_detection_by_hash() {
        let r1 = new_record("genesis", "tx-1", 1);
        let r2 = new_record("genesis", "tx-1", 1);
        // Same inputs but different timestamp -> distinct hashes. Replays are
        // detected by transaction_id duplication at the store level.
        assert_ne!(r1.record_hash, r2.record_hash);
    }
}
