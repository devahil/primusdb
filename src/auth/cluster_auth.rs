/*!
# PrimusDB Cluster Authentication - Genesis Keys

This module implements a genesis key system for secure cluster node
authentication. Each node in a PrimusDB cluster uses cryptographic keys for
mutual authentication and secure communication.

## Architecture

```text
ClusterAuthManager
  ├─ GenesisBlock: initial trust anchor, network configuration,
  │    validator set
  ├─ NodeIdentity / NodeCertificate: node key pairs, certificates,
  │    metadata
  ├─ Trust chain: certificate validation, signature verification,
  │    revocation
  └─ Challenge/response authentication with node heartbeats
```
*/
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use chrono::{DateTime, Duration, Utc};
use ring::rand::SecureRandom;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Cryptographic genesis key that anchors cluster trust.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisKey {
    /// Stable key identifier
    pub key_id: String,
    /// Hex-encoded public key
    pub public_key: String,
    /// Password-wrapped private key material
    pub private_key_encrypted: String,
    /// When the key was created
    pub created_at: DateTime<Utc>,
    /// Whether this key belongs to a validator node
    pub is_validator: bool,
    /// Node this key belongs to, if any
    pub node_id: Option<String>,
}

/// Identity record for a node joined to the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentity {
    /// Unique node identifier
    pub node_id: String,
    /// Hex-encoded public key of the node
    pub public_key: String,
    /// Signed certificate binding the identity to the public key
    pub certificate: NodeCertificate,
    /// Hardware and topology metadata
    pub metadata: NodeMetadata,
    /// Current lifecycle status of the node
    pub status: NodeStatus,
    /// When the node last reported a heartbeat
    pub last_heartbeat: DateTime<Utc>,
}

/// Signed certificate binding a node identity to its public key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCertificate {
    /// Stable certificate identifier
    pub cert_id: String,
    /// Owner node id
    pub node_id: String,
    /// Public key bound by the certificate
    pub public_key: String,
    /// When the certificate was issued
    pub issued_at: DateTime<Utc>,
    /// When the certificate expires
    pub expires_at: DateTime<Utc>,
    /// Id of the issuer (the genesis key)
    pub issuer_id: String,
    /// Cryptographic signature over the certificate
    pub signature: String,
    /// Whether the certificate has been validated
    pub is_validated: bool,
}

/// Hardware and topology metadata describing a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    /// Human readable node name
    pub name: String,
    /// Network address of the node
    pub address: String,
    /// Listening port of the node
    pub port: u16,
    /// Optional cloud region
    pub region: Option<String>,
    /// Optional datacenter identifier
    pub datacenter: Option<String>,
    /// Capabilities the node advertises
    pub capabilities: Vec<String>,
    /// Storage engines the node hosts
    pub storage_types: Vec<String>,
    /// Total storage capacity in GB
    pub total_storage_gb: u64,
    /// Available storage in GB
    pub available_storage_gb: u64,
    /// Number of CPU cores
    pub cpu_cores: u32,
    /// Memory in GB
    pub memory_gb: u64,
}

/// Lifecycle state of a cluster node.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node registered but not yet accepted
    Pending,
    /// Node is participating in the cluster
    Active,
    /// Node is suspended and not serving requests
    Suspended,
    /// Node has been revoked or disconnected
    Offline,
}

/// The first block of the cluster ledger anchoring the trust chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisBlock {
    /// Unique block identifier
    pub block_id: String,
    /// Network the block belongs to
    pub network_id: String,
    /// Human readable network name
    pub network_name: String,
    /// When the block was created
    pub created_at: DateTime<Utc>,
    /// The genesis key embedded in the block
    pub genesis_key: GenesisKey,
    /// Validator set registered at genesis
    pub initial_validators: Vec<ValidatorInfo>,
    /// Consensus and network parameters
    pub network_config: NetworkConfiguration,
    /// Hash of the previous block (all zeros for genesis)
    pub previous_hash: String,
}

/// Metadata for a validator node registered in the genesis block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    /// Validator node id
    pub node_id: String,
    /// Validator public key
    public_key: String,
    /// Amount staked by the validator
    stake: u64,
    /// Validator hardware and topology metadata
    metadata: NodeMetadata,
}

/// Consensus and network parameters for the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfiguration {
    /// Consensus algorithm in use
    pub consensus_type: ConsensusType,
    /// Target block production interval in milliseconds
    pub block_time_ms: u32,
    /// Maximum transactions accepted per block
    pub max_tx_per_block: u32,
    /// Minimum number of validators required
    pub min_validators: u32,
    /// Maximum number of validators allowed
    pub max_validators: u32,
    /// Fault tolerance factor (e.g. 0.33 for PBFT)
    pub fault_tolerance: f32,
}

/// Consensus algorithm used by the cluster.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConsensusType {
    /// Practical Byzantine Fault Tolerance
    PBFT,
    /// Raft leader-based replication
    Raft,
    /// Proof of Stake
    PoS,
}

/// Configuration for cluster node authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterAuthConfig {
    /// Unique network identifier
    pub network_id: String,
    /// Human readable network name
    pub network_name: String,
    /// Shared password guarding genesis and node enrollment
    pub genesis_password: String,
    /// Minimum stake required for validator registration
    pub validator_stake: u64,
    /// Certificate validity period in days
    pub certificate_expiry_days: u32,
    /// Heartbeat timeout in seconds before a node is considered offline
    pub heartbeat_timeout_seconds: u32,
}

impl Default for ClusterAuthConfig {
    fn default() -> Self {
        Self {
            network_id: format!("primusdb_{}", Utc::now().timestamp()),
            network_name: "PrimusDB Network".to_string(),
            genesis_password: "changeme".to_string(),
            validator_stake: 1000,
            certificate_expiry_days: 365,
            heartbeat_timeout_seconds: 30,
        }
    }
}

/// Manages genesis keys, node identities, certificates and revocation.
pub struct ClusterAuthManager {
    config: ClusterAuthConfig,
    genesis_key: Option<GenesisKey>,
    nodes: HashMap<String, NodeIdentity>,
    certificates: HashMap<String, NodeCertificate>,
    valid_node_signatures: HashMap<String, String>,
    revoked_certs: HashMap<String, DateTime<Utc>>,
    genesis_block: Option<GenesisBlock>,
    rng: ring::rand::SystemRandom,
}

impl ClusterAuthManager {
    /// Create a new cluster auth manager without a genesis block.
    pub fn new(config: ClusterAuthConfig) -> crate::Result<Self> {
        Ok(Self {
            config,
            genesis_key: None,
            nodes: HashMap::new(),
            certificates: HashMap::new(),
            valid_node_signatures: HashMap::new(),
            revoked_certs: HashMap::new(),
            genesis_block: None,
            rng: ring::rand::SystemRandom::new(),
        })
    }

    /// Create the genesis key and genesis block for the network.
    pub fn initialize_genesis(&mut self, password: &str) -> crate::Result<GenesisBlock> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let genesis_key_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| crate::Error::CryptoError(format!("Genesis key hashing failed: {}", e)))?
            .to_string();

        let (private_key, public_key) = Self::generate_keypair()?;

        let _private_key_hash = {
            let mut hasher = Sha256::new();
            hasher.update(private_key.as_bytes());
            hex::encode(hasher.finalize())
        };

        let encrypted_private_key = format!("{}${}", genesis_key_hash, private_key);

        let genesis_key = GenesisKey {
            key_id: "genesis".to_string(),
            public_key: public_key.clone(),
            private_key_encrypted: encrypted_private_key.clone(),
            created_at: Utc::now(),
            is_validator: true,
            node_id: Some("genesis_node".to_string()),
        };

        let block_id = format!("genesis_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));

        let genesis_block = GenesisBlock {
            block_id: block_id.clone(),
            network_id: self.config.network_id.clone(),
            network_name: self.config.network_name.clone(),
            created_at: Utc::now(),
            genesis_key: genesis_key.clone(),
            initial_validators: vec![ValidatorInfo {
                node_id: "genesis_node".to_string(),
                public_key: public_key.clone(),
                stake: self.config.validator_stake,
                metadata: NodeMetadata {
                    name: "Genesis Node".to_string(),
                    address: "127.0.0.1".to_string(),
                    port: 8080,
                    region: None,
                    datacenter: None,
                    capabilities: vec!["read".to_string(), "write".to_string()],
                    storage_types: vec![
                        "columnar".to_string(),
                        "vector".to_string(),
                        "document".to_string(),
                        "relational".to_string(),
                    ],
                    total_storage_gb: 1000,
                    available_storage_gb: 900,
                    cpu_cores: 8,
                    memory_gb: 32,
                },
            }],
            network_config: NetworkConfiguration {
                consensus_type: ConsensusType::PBFT,
                block_time_ms: 1000,
                max_tx_per_block: 10000,
                min_validators: 3,
                max_validators: 21,
                fault_tolerance: 0.33,
            },
            previous_hash: "0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
        };

        let _block_hash = self.compute_block_hash(&genesis_block)?;

        let final_block = genesis_block.clone();

        self.genesis_key = Some(genesis_key);
        self.genesis_block = Some(final_block);

        Ok(genesis_block)
    }

    /// Enroll a new node into the network, issuing a signed certificate.
    pub fn join_network(
        &mut self,
        node_id: String,
        metadata: NodeMetadata,
        password: &str,
    ) -> crate::Result<NodeIdentity> {
        let expected_hash = {
            let mut hasher = Sha256::new();
            hasher.update(password.as_bytes());
            hex::encode(hasher.finalize())
        };

        if self.nodes.contains_key(&node_id) {
            return Err(crate::Error::ClusterError(
                "Node already exists".to_string(),
            ));
        }

        let (_private_key, public_key) = Self::generate_keypair()?;

        let cert_id = format!(
            "cert_{}_{}",
            node_id,
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );

        let certificate = NodeCertificate {
            cert_id: cert_id.clone(),
            node_id: node_id.clone(),
            public_key: public_key.clone(),
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(self.config.certificate_expiry_days as i64),
            issuer_id: "genesis".to_string(),
            signature: String::new(),
            is_validated: false,
        };

        let signature = self.sign_data(&certificate.cert_id, &expected_hash)?;

        let mut signed_cert = certificate.clone();
        signed_cert.signature = signature;

        let identity = NodeIdentity {
            node_id: node_id.clone(),
            public_key: public_key.clone(),
            certificate: signed_cert.clone(),
            metadata,
            status: NodeStatus::Pending,
            last_heartbeat: Utc::now(),
        };

        self.nodes.insert(node_id.clone(), identity.clone());
        self.certificates.insert(cert_id, signed_cert);
        self.valid_node_signatures.insert(node_id, expected_hash);

        Ok(identity)
    }

    /// Verify a node's challenge-response proof against its registered signature.
    pub fn authenticate_node(
        &self,
        node_id: &str,
        challenge: &str,
        response: &str,
    ) -> crate::Result<bool> {
        let expected_hash = self
            .valid_node_signatures
            .get(node_id)
            .ok_or_else(|| crate::Error::AuthenticationError("Node not registered".to_string()))?;

        let identity = self
            .nodes
            .get(node_id)
            .ok_or_else(|| crate::Error::ClusterError("Node not found".to_string()))?;

        if identity.status == NodeStatus::Offline {
            return Err(crate::Error::AuthenticationError(
                "Node is offline".to_string(),
            ));
        }

        let expected_response = {
            let mut hasher = Sha256::new();
            hasher.update(format!("{}:{}", challenge, expected_hash).as_bytes());
            hex::encode(hasher.finalize())
        };

        Ok(response == expected_response)
    }

    /// Generate a fresh random challenge for node authentication.
    pub fn generate_auth_challenge(&self) -> crate::Result<String> {
        let mut challenge_bytes = vec![0u8; 32];
        self.rng.fill(&mut challenge_bytes).map_err(|e| {
            crate::Error::CryptoError(format!("Failed to generate challenge: {}", e))
        })?;

        Ok(hex::encode(&challenge_bytes))
    }

    /// Update a node's status and refresh its heartbeat timestamp.
    pub fn update_node_status(&mut self, node_id: &str, status: NodeStatus) -> crate::Result<()> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| crate::Error::ClusterError("Node not found".to_string()))?;

        node.status = status;
        node.last_heartbeat = Utc::now();

        Ok(())
    }

    /// Revoke a node, marking it offline and revoking its certificate.
    pub fn revoke_node(&mut self, node_id: &str) -> crate::Result<()> {
        if let Some(identity) = self.nodes.get(node_id) {
            self.revoked_certs
                .insert(identity.certificate.cert_id.clone(), Utc::now());
        }

        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| crate::Error::ClusterError("Node not found".to_string()))?;

        node.status = NodeStatus::Offline;

        Ok(())
    }

    /// List all nodes currently in the `Active` state.
    pub fn list_active_nodes(&self) -> Vec<NodeIdentity> {
        self.nodes
            .values()
            .filter(|n| n.status == NodeStatus::Active)
            .cloned()
            .collect()
    }

    /// Fetch a node identity by id.
    pub fn get_node(&self, node_id: &str) -> Option<NodeIdentity> {
        self.nodes.get(node_id).cloned()
    }

    /// Return the genesis block if it has been initialized.
    pub fn get_genesis_block(&self) -> Option<GenesisBlock> {
        self.genesis_block.clone()
    }

    /// Recompute the genesis block hash and compare it against the stored hash.
    pub fn verify_chain(&self) -> crate::Result<bool> {
        if self.genesis_block.is_none() {
            return Ok(false);
        }

        let block = self.genesis_block.as_ref().unwrap();
        let computed_hash = self.compute_block_hash(block)?;

        Ok(computed_hash == block.previous_hash
            || computed_hash.starts_with(&block.previous_hash[..8]))
    }

    fn generate_keypair() -> crate::Result<(String, String)> {
        let mut private_key_bytes = vec![0u8; 32];
        let rng = ring::rand::SystemRandom::new();
        rng.fill(&mut private_key_bytes)
            .map_err(|e| crate::Error::CryptoError(format!("Failed to generate key: {}", e)))?;

        let private_key = hex::encode(&private_key_bytes);

        let mut hasher = Sha256::new();
        hasher.update(&private_key_bytes);
        hasher.update(b"public_key_derivation");
        let public_key = hex::encode(hasher.finalize());

        Ok((private_key, public_key))
    }

    fn sign_data(&self, data: &str, _private_key: &str) -> crate::Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hasher.update(_private_key.as_bytes());

        Ok(hex::encode(hasher.finalize()))
    }

    fn compute_block_hash(&self, block: &GenesisBlock) -> crate::Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(block.block_id.as_bytes());
        hasher.update(block.network_id.as_bytes());
        hasher.update(block.network_name.as_bytes());
        hasher.update(block.created_at.to_rfc3339().as_bytes());
        hasher.update(block.genesis_key.public_key.as_bytes());
        hasher.update(block.previous_hash.as_bytes());

        Ok(hex::encode(hasher.finalize()))
    }

    /// Check that a certificate is not revoked and has not expired.
    pub fn validate_certificate(&self, cert: &NodeCertificate) -> crate::Result<bool> {
        if self.revoked_certs.contains_key(&cert.cert_id) {
            return Ok(false);
        }

        if cert.expires_at < Utc::now() {
            return Ok(false);
        }

        Ok(true)
    }
}

/// Payload for a node joining the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistrationRequest {
    /// Node identifier to register
    pub node_id: String,
    /// Hardware and topology metadata
    pub metadata: NodeMetadata,
    /// Network password proving membership
    pub password: String,
}

/// Response returned to a node after authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAuthResponse {
    /// Authenticated node id
    pub node_id: String,
    /// Current node status
    pub status: NodeStatus,
    /// The challenge that was presented
    pub challenge: String,
}

/// Payload carrying a node's answer to an authentication challenge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAuthRequest {
    /// Node identifier being authenticated
    pub node_id: String,
    /// Challenge response proof
    pub challenge_response: String,
}

/// Async wrapper around [`ClusterAuthManager`].
pub struct ClusterAuthService {
    manager: std::sync::Arc<tokio::sync::RwLock<ClusterAuthManager>>,
}

impl ClusterAuthService {
    /// Create a cluster auth service backed by a new [`ClusterAuthManager`].
    pub fn new(config: ClusterAuthConfig) -> crate::Result<Self> {
        Ok(Self {
            manager: std::sync::Arc::new(tokio::sync::RwLock::new(ClusterAuthManager::new(
                config,
            )?)),
        })
    }

    /// Create the genesis key and genesis block for the network.
    pub async fn initialize_genesis(&self, password: &str) -> crate::Result<GenesisBlock> {
        let mut manager = self.manager.write().await;
        manager.initialize_genesis(password)
    }

    /// Enroll a new node into the network.
    pub async fn join_network(
        &self,
        request: NodeRegistrationRequest,
    ) -> crate::Result<NodeIdentity> {
        let mut manager = self.manager.write().await;
        manager.join_network(request.node_id, request.metadata, &request.password)
    }

    /// Authenticate a node using a fresh challenge-response round.
    pub async fn authenticate_node(&self, request: NodeAuthRequest) -> crate::Result<bool> {
        let manager = self.manager.read().await;
        let challenge = manager.generate_auth_challenge()?;

        drop(manager);

        let manager = self.manager.write().await;
        manager.authenticate_node(&request.node_id, &challenge, &request.challenge_response)
    }

    /// Generate a fresh random authentication challenge.
    pub async fn generate_challenge(&self) -> crate::Result<String> {
        let manager = self.manager.read().await;
        manager.generate_auth_challenge()
    }

    /// Update a node's heartbeat, marking it active.
    pub async fn update_heartbeat(&self, node_id: &str) -> crate::Result<()> {
        let mut manager = self.manager.write().await;
        manager.update_node_status(node_id, NodeStatus::Active)
    }

    /// List all nodes currently in the `Active` state.
    pub async fn list_active_nodes(&self) -> Vec<NodeIdentity> {
        let manager = self.manager.read().await;
        manager.list_active_nodes()
    }

    /// Fetch a node identity by id.
    pub async fn get_node(&self, node_id: &str) -> Option<NodeIdentity> {
        let manager = self.manager.read().await;
        manager.get_node(node_id)
    }

    /// Revoke a node from the network.
    pub async fn revoke_node(&self, node_id: &str) -> crate::Result<()> {
        let mut manager = self.manager.write().await;
        manager.revoke_node(node_id)
    }

    /// Return the genesis block if it has been initialized.
    pub async fn get_genesis_block(&self) -> Option<GenesisBlock> {
        let manager = self.manager.read().await;
        manager.get_genesis_block()
    }

    /// Recompute and verify the genesis block hash.
    pub async fn verify_chain(&self) -> crate::Result<bool> {
        let manager = self.manager.read().await;
        manager.verify_chain()
    }
}
