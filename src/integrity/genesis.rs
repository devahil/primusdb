//! Database genesis identity.
//!
//! Every database created by PrimusDB receives a unique, signed genesis
//! record that binds its name, namespace, engines, creating node and
//! configuration digest. The genesis is the root of the database's integrity
//! chain: all subsequent transaction records link back through a hash chain
//! whose anchor is this record.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::errors::IntegrityResult;
use super::signing::SigningService;

/// Origin of a genesis identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GenesisOrigin {
    /// Created through the standard database creation path.
    #[default]
    Created,
    /// Created during migration of a legacy (pre-integrity) database.
    LegacyImport,
    /// Created when a database is cloned from an existing source.
    Clone,
}

impl std::fmt::Display for GenesisOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenesisOrigin::Created => write!(f, "created"),
            GenesisOrigin::LegacyImport => write!(f, "legacy-import"),
            GenesisOrigin::Clone => write!(f, "clone"),
        }
    }
}

/// Lifecycle state of the genesis identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GenesisStatus {
    #[default]
    Active,
    LegacyImported,
    Revoked,
}

/// The signed cryptographic identity of a database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseGenesis {
    /// Unique database identity (uuid v4). Never reused.
    pub database_id: String,
    /// Human-readable database name (namespace path).
    pub database_name: String,
    /// Optional namespace the database lives in.
    pub namespace: Option<String>,
    /// Storage engines attached to the database.
    pub engine_types: Vec<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Node that created the database.
    pub creating_node: String,
    /// Cluster identity when created inside a cluster.
    pub cluster_id: Option<String>,
    /// SHA-256 digest of the server configuration at creation time.
    pub config_digest: String,
    /// SHA-256 digest of the initial schema (when known).
    pub schema_digest: Option<String>,
    /// Identity of the parent database for clones.
    pub parent_identity: Option<String>,
    /// Origin of this identity.
    pub origin: GenesisOrigin,
    /// Current lifecycle status.
    pub status: GenesisStatus,
    /// Signer identity that produced `signature`.
    pub signer_id: String,
    /// Hex public key of the signer (self-describing for verification).
    pub signer_public_key: String,
    /// Base64 ED25519 signature over [`DatabaseGenesis::canonical_bytes`].
    pub signature: String,
}

/// Inputs needed to construct a genesis record.
pub struct NewGenesis<'a> {
    pub database_name: &'a str,
    pub namespace: Option<&'a str>,
    pub engine_types: &'a [String],
    pub creating_node: &'a str,
    pub cluster_id: Option<&'a str>,
    pub config_digest: &'a str,
    pub schema_digest: Option<&'a str>,
    pub parent_identity: Option<&'a str>,
    pub origin: GenesisOrigin,
}

impl DatabaseGenesis {
    /// Builds, signs, and returns a new genesis record.
    pub fn create(
        input: NewGenesis<'_>,
        signer: &SigningService,
    ) -> IntegrityResult<DatabaseGenesis> {
        let mut genesis = DatabaseGenesis {
            database_id: uuid::Uuid::new_v4().to_string(),
            database_name: input.database_name.to_string(),
            namespace: input.namespace.map(String::from),
            engine_types: input.engine_types.to_vec(),
            created_at: Utc::now(),
            creating_node: input.creating_node.to_string(),
            cluster_id: input.cluster_id.map(String::from),
            config_digest: input.config_digest.to_string(),
            schema_digest: input.schema_digest.map(String::from),
            parent_identity: input.parent_identity.map(String::from),
            origin: input.origin,
            status: GenesisStatus::Active,
            signer_id: signer.signer_id().to_string(),
            signer_public_key: signer.public_key_hex().to_string(),
            signature: String::new(),
        };
        genesis.signature = signer.sign(&genesis.canonical_bytes())?;
        Ok(genesis)
    }

    /// Canonical bytes signed/verified by the signature. Excludes the signature
    /// field itself so the record can be serialized with or without it.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut value = serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}));
        if let Some(obj) = value.as_object_mut() {
            obj.remove("signature");
        }
        serde_json::to_vec(&value).unwrap_or_default()
    }

    /// Verifies the embedded signature against the embedded public key.
    pub fn verify_signature(&self) -> IntegrityResult<bool> {
        SigningService::verify(
            &self.signer_public_key,
            &self.canonical_bytes(),
            &self.signature,
        )
    }
}

/// Result of verifying a persisted genesis against stored metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisVerification {
    pub database_id: String,
    pub signature_valid: bool,
    pub identity_matches_name: bool,
    pub status: GenesisStatus,
    pub verified_at: DateTime<Utc>,
}

impl GenesisVerification {
    pub fn ok(genesis: &DatabaseGenesis) -> Self {
        GenesisVerification {
            database_id: genesis.database_id.clone(),
            signature_valid: true,
            identity_matches_name: true,
            status: genesis.status,
            verified_at: Utc::now(),
        }
    }
}

/// Digest helpers used to bind configuration and schema into the genesis.
pub fn digest_bytes(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(data))
}

pub fn digest_value(value: &serde_json::Value) -> String {
    digest_bytes(&serde_json::to_vec(value).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn signer() -> SigningService {
        let dir = tempdir().unwrap();
        SigningService::load_or_create("node-1", Some(&dir.path().join("k"))).unwrap()
    }

    fn new_genesis(name: &str) -> DatabaseGenesis {
        DatabaseGenesis::create(
            NewGenesis {
                database_name: name,
                namespace: Some("prod"),
                engine_types: &["relational".to_string(), "vector".to_string()],
                creating_node: "node-1",
                cluster_id: None,
                config_digest: &digest_bytes(b"config"),
                schema_digest: None,
                parent_identity: None,
                origin: GenesisOrigin::Created,
            },
            &signer(),
        )
        .unwrap()
    }

    #[test]
    fn test_genesis_signature_verifies() {
        let g = new_genesis("analytics");
        assert!(g.verify_signature().unwrap());
    }

    #[test]
    fn test_genesis_tamper_detected() {
        let mut g = new_genesis("analytics");
        g.database_name = "analytics-x".to_string();
        assert!(!g.verify_signature().unwrap());
    }

    #[test]
    fn test_genesis_ids_are_unique() {
        let a = new_genesis("a");
        let b = new_genesis("b");
        assert_ne!(a.database_id, b.database_id);
    }
}
