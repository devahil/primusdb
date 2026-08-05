//! Integrity error types.
//!
//! All integrity, signing, genesis, checkpoint, and reconciliation failures
//! surface as [`IntegrityError`] so callers can react to the specific class of
//! failure (missing genesis, bad signature, ledger unavailable, etc.) instead
//! of swallowing generic errors.

use std::fmt;

/// Errors produced by the integrity subsystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityError {
    /// A database has no genesis identity yet.
    MissingGenesis(String),
    /// A database already has a genesis identity and cannot be re-initialized.
    GenesisAlreadyExists(String),
    /// A signature did not verify against the given public key.
    SignatureVerificationFailed,
    /// The signing identity is not available (key material missing/corrupt).
    SigningIdentityUnavailable(String),
    /// The persisted hash chain is broken at the given sequence.
    ChainBroken { database_id: String, sequence: u64 },
    /// A record's previous hash does not match the persisted chain.
    PreviousHashMismatch { sequence: u64 },
    /// A record sequence is out of order.
    OutOfOrderSequence { expected: u64, got: u64 },
    /// A payload digest does not match the record content.
    DigestMismatch,
    /// The Hyperledger service is required by policy but unavailable.
    LedgerUnavailable(String),
    /// A synchronous ledger submission was not confirmed in time.
    LedgerConfirmationTimeout(String),
    /// The mutation was rejected as a replay or duplicate.
    ReplayRejected(String),
    /// Policy forbids the operation (e.g. genesis required but mode disabled).
    PolicyViolation(String),
    /// Data was quarantined because it failed validation.
    Quarantined(String),
    /// Underlying storage failure.
    Storage(String),
    /// Generic internal error.
    Internal(String),
}

impl fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntegrityError::MissingGenesis(db) => {
                write!(f, "database '{}' has no genesis identity", db)
            }
            IntegrityError::GenesisAlreadyExists(db) => {
                write!(f, "database '{}' already has a genesis identity", db)
            }
            IntegrityError::SignatureVerificationFailed => {
                write!(f, "signature verification failed")
            }
            IntegrityError::SigningIdentityUnavailable(reason) => {
                write!(f, "signing identity unavailable: {}", reason)
            }
            IntegrityError::ChainBroken {
                database_id,
                sequence,
            } => write!(
                f,
                "integrity chain for '{}' is broken at sequence {}",
                database_id, sequence
            ),
            IntegrityError::PreviousHashMismatch { sequence } => {
                write!(f, "previous hash mismatch at sequence {}", sequence)
            }
            IntegrityError::OutOfOrderSequence { expected, got } => write!(
                f,
                "out-of-order sequence: expected {}, got {}",
                expected, got
            ),
            IntegrityError::DigestMismatch => write!(f, "payload digest mismatch"),
            IntegrityError::LedgerUnavailable(reason) => {
                write!(f, "ledger unavailable: {}", reason)
            }
            IntegrityError::LedgerConfirmationTimeout(tx) => {
                write!(f, "ledger confirmation timed out for submission '{}'", tx)
            }
            IntegrityError::ReplayRejected(id) => {
                write!(f, "replayed transaction rejected: {}", id)
            }
            IntegrityError::PolicyViolation(msg) => {
                write!(f, "integrity policy violation: {}", msg)
            }
            IntegrityError::Quarantined(msg) => write!(f, "data quarantined: {}", msg),
            IntegrityError::Storage(msg) => write!(f, "integrity storage error: {}", msg),
            IntegrityError::Internal(msg) => write!(f, "internal integrity error: {}", msg),
        }
    }
}

impl std::error::Error for IntegrityError {}

impl From<sled::Error> for IntegrityError {
    fn from(e: sled::Error) -> Self {
        IntegrityError::Storage(e.to_string())
    }
}

impl From<serde_json::Error> for IntegrityError {
    fn from(e: serde_json::Error) -> Self {
        IntegrityError::Internal(format!("serialization error: {}", e))
    }
}

/// Convenience alias used throughout the integrity subsystem.
pub type IntegrityResult<T> = std::result::Result<T, IntegrityError>;
