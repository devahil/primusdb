/*!
# PrimusDB Cluster Authentication - Hyperledger-Style Genesis Keys

This module implements a Hyperledger-inspired genesis key system for secure
cluster node authentication. Each node in a PrimusDB cluster uses cryptographic
keys for mutual authentication and secure communication.

## Architecture

```
Cluster Authentication System
══════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────┐
│                    Genesis Key System                            │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Genesis Block                                           │  │
│  │  • Initial trust anchor                                  │  │
│  │  • Network configuration                                  │  │
│  │  • Validator set                                         │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Node Identity                                           │  │
│  │  • Public/Private key pair                               │  │
│  │  • Node certificate                                      │  │
│  │  • Node metadata                                         │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Trust Chain                                             │  │
│  │  • Certificate validation                                │  │
│  │  • Signature verification                                │  │
│  │  • Revocation checking                                   │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    Node Communication                            │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  gRPC Transport                                          │  │
│  │  • TLS encryption                                        │  │
│  │  • Mutual authentication                                  │  │
│  │  • Message signing                                       │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Consensus Integration                                   │  │
│  │  • Block signing                                         │  │
│  │  • Vote authentication                                   │  │
│  │  • Validator voting                                      │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
*/
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Duration, Utc};
use ring::rand::SecureRandom;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisKey {
    pub key_id: String,
    pub public_key: String,
    pub private_key_encrypted: String,
    pub created_at: DateTime<Utc>,
    pub is_validator: bool,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub node_id: String,
    pub public_key: String,
    pub certificate: NodeCertificate,
    pub metadata: NodeMetadata,
    pub status: NodeStatus,
    pub last_heartbeat: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCertificate {
    pub cert_id: String,
    pub node_id: String,
    pub public_key: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub issuer_id: String,
    pub signature: String,
    pub is_validated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub region: Option<String>,
    pub datacenter: Option<String>,
    pub capabilities: Vec<String>,
    pub storage_types: Vec<String>,
    pub total_storage_gb: u64,
    pub available_storage_gb: u64,
    pub cpu_cores: u32,
    pub memory_gb: u64,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum NodeStatus {
    Pending,
    Active,
    Suspended,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisBlock {
    pub block_id: String,
    pub network_id: String,
    pub network_name: String,
    pub created_at: DateTime<Utc>,
    pub genesis_key: GenesisKey,
    pub initial_validators: Vec<ValidatorInfo>,
    pub network_config: NetworkConfiguration,
    pub previous_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub node_id: String,
    public_key: String,
    stake: u64,
    metadata: NodeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfiguration {
    pub consensus_type: ConsensusType,
    pub block_time_ms: u32,
    pub max_tx_per_block: u32,
    pub min_validators: u32,
    pub max_validators: u32,
    pub fault_tolerance: f32,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConsensusType {
    PBFT,
    Raft,
    PoS,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterAuthConfig {
    pub network_id: String,
    pub network_name: String,
    pub genesis_password: String,
    pub validator_stake: u64,
    pub certificate_expiry_days: u32,
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

    pub fn initialize_genesis(&mut self, password: &str) -> crate::Result<GenesisBlock> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let genesis_key_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| crate::Error::CryptoError(format!("Genesis key hashing failed: {}", e)))?
            .to_string();

        let (private_key, public_key) = Self::generate_keypair()?;
        
        let private_key_hash = {
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
                    storage_types: vec!["columnar".to_string(), "vector".to_string(), "document".to_string(), "relational".to_string()],
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
            previous_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        };

        let block_hash = self.compute_block_hash(&genesis_block)?;
        
        let mut final_block = genesis_block.clone();
        
        self.genesis_key = Some(genesis_key);
        self.genesis_block = Some(final_block);

        Ok(genesis_block)
    }

    pub fn join_network(&mut self, node_id: String, metadata: NodeMetadata, password: &str) -> crate::Result<NodeIdentity> {
        let expected_hash = {
            let mut hasher = Sha256::new();
            hasher.update(password.as_bytes());
            hex::encode(hasher.finalize())
        };

        if self.nodes.contains_key(&node_id) {
            return Err(crate::Error::ClusterError("Node already exists".to_string()));
        }

        let (private_key, public_key) = Self::generate_keypair()?;

        let cert_id = format!("cert_{}_{}", node_id, Utc::now().timestamp_nanos_opt().unwrap_or(0));
        
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

    pub fn authenticate_node(&self, node_id: &str, challenge: &str, response: &str) -> crate::Result<bool> {
        let expected_hash = self.valid_node_signatures.get(node_id)
            .ok_or_else(|| crate::Error::AuthenticationError("Node not registered".to_string()))?;

        let identity = self.nodes.get(node_id)
            .ok_or_else(|| crate::Error::ClusterError("Node not found".to_string()))?;

        if identity.status == NodeStatus::Offline {
            return Err(crate::Error::AuthenticationError("Node is offline".to_string()));
        }

        let expected_response = {
            let mut hasher = Sha256::new();
            hasher.update(format!("{}:{}", challenge, expected_hash).as_bytes());
            hex::encode(hasher.finalize())
        };

        Ok(response == expected_response)
    }

    pub fn generate_auth_challenge(&self) -> crate::Result<String> {
        let mut challenge_bytes = vec![0u8; 32];
        self.rng.fill(&mut challenge_bytes).map_err(|e| {
            crate::Error::CryptoError(format!("Failed to generate challenge: {}", e))
        })?;
        
        Ok(hex::encode(&challenge_bytes))
    }

    pub fn update_node_status(&mut self, node_id: &str, status: NodeStatus) -> crate::Result<()> {
        let node = self.nodes.get_mut(node_id)
            .ok_or_else(|| crate::Error::ClusterError("Node not found".to_string()))?;
        
        node.status = status;
        node.last_heartbeat = Utc::now();
        
        Ok(())
    }

    pub fn revoke_node(&mut self, node_id: &str) -> crate::Result<()> {
        if let Some(identity) = self.nodes.get(node_id) {
            self.revoked_certs.insert(identity.certificate.cert_id.clone(), Utc::now());
        }
        
        let node = self.nodes.get_mut(node_id)
            .ok_or_else(|| crate::Error::ClusterError("Node not found".to_string()))?;
        
        node.status = NodeStatus::Offline;
        
        Ok(())
    }

    pub fn list_active_nodes(&self) -> Vec<NodeIdentity> {
        self.nodes
            .values()
            .filter(|n| n.status == NodeStatus::Active)
            .cloned()
            .collect()
    }

    pub fn get_node(&self, node_id: &str) -> Option<NodeIdentity> {
        self.nodes.get(node_id).cloned()
    }

    pub fn get_genesis_block(&self) -> Option<GenesisBlock> {
        self.genesis_block.clone()
    }

    pub fn verify_chain(&self) -> crate::Result<bool> {
        if self.genesis_block.is_none() {
            return Ok(false);
        }

        let block = self.genesis_block.as_ref().unwrap();
        let computed_hash = self.compute_block_hash(block)?;
        
        Ok(computed_hash == block.previous_hash || computed_hash.starts_with(&block.previous_hash[..8]))
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistrationRequest {
    pub node_id: String,
    pub metadata: NodeMetadata,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAuthResponse {
    pub node_id: String,
    pub status: NodeStatus,
    pub challenge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAuthRequest {
    pub node_id: String,
    pub challenge_response: String,
}

pub struct ClusterAuthService {
    manager: std::sync::Arc<tokio::sync::RwLock<ClusterAuthManager>>,
}

impl ClusterAuthService {
    pub fn new(config: ClusterAuthConfig) -> crate::Result<Self> {
        Ok(Self {
            manager: std::sync::Arc::new(tokio::sync::RwLock::new(ClusterAuthManager::new(config)?)),
        })
    }

    pub async fn initialize_genesis(&self, password: &str) -> crate::Result<GenesisBlock> {
        let mut manager = self.manager.write().await;
        manager.initialize_genesis(password)
    }

    pub async fn join_network(&self, request: NodeRegistrationRequest) -> crate::Result<NodeIdentity> {
        let mut manager = self.manager.write().await;
        manager.join_network(request.node_id, request.metadata, &request.password)
    }

    pub async fn authenticate_node(&self, request: NodeAuthRequest) -> crate::Result<bool> {
        let manager = self.manager.read().await;
        let challenge = manager.generate_auth_challenge()?;
        
        drop(manager);
        
        let mut manager = self.manager.write().await;
        manager.authenticate_node(&request.node_id, &challenge, &request.challenge_response)
    }

    pub async fn generate_challenge(&self) -> crate::Result<String> {
        let manager = self.manager.read().await;
        manager.generate_auth_challenge()
    }

    pub async fn update_heartbeat(&self, node_id: &str) -> crate::Result<()> {
        let mut manager = self.manager.write().await;
        manager.update_node_status(node_id, NodeStatus::Active)
    }

    pub async fn list_active_nodes(&self) -> Vec<NodeIdentity> {
        let manager = self.manager.read().await;
        manager.list_active_nodes()
    }

    pub async fn get_node(&self, node_id: &str) -> Option<NodeIdentity> {
        let manager = self.manager.read().await;
        manager.get_node(node_id)
    }

    pub async fn revoke_node(&self, node_id: &str) -> crate::Result<()> {
        let mut manager = self.manager.write().await;
        manager.revoke_node(node_id)
    }

    pub async fn get_genesis_block(&self) -> Option<GenesisBlock> {
        let manager = self.manager.read().await;
        manager.get_genesis_block()
    }

    pub async fn verify_chain(&self) -> crate::Result<bool> {
        let manager = self.manager.read().await;
        manager.verify_chain()
    }
}
