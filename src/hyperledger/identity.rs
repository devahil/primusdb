//! Signing identity references for Hyperledger.
//!
//! The identity name in the Hyperledger configuration references a Fabric
//! identity (and its enrollment material) that the gateway resolves. PrimusDB
//! never holds Fabric enrollment private keys in plaintext in its own store —
//! the key material lives with the gateway/HSM. This module only records the
//! *reference* plus an optional enrollment certificate fingerprint for
//! diagnostics.

use serde::{Deserialize, Serialize};

/// A reference to the Fabric identity used for ledger submissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningIdentityRef {
    /// Identity name as known to the gateway (e.g. `primus-node`).
    pub identity: String,
    /// MSP id of the organization.
    pub msp_id: String,
    /// Certificate fingerprint (SHA-256 of the DER cert) when available.
    pub cert_fingerprint: Option<String>,
}

impl SigningIdentityRef {
    pub fn new(identity: &str, msp_id: &str) -> Self {
        SigningIdentityRef {
            identity: identity.to_string(),
            msp_id: msp_id.to_string(),
            cert_fingerprint: None,
        }
    }
}

/// Computes a SHA-256 fingerprint of a DER/PEM certificate body for
/// diagnostics.
pub fn cert_fingerprint(pem: &str) -> String {
    use sha2::{Digest, Sha256};
    let body = pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");
    hex::encode(Sha256::digest(body.as_bytes()))
}
