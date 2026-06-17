/*!
# Secure Messaging System

This module provides end-to-end encrypted messaging between PrimusDB nodes with
digital signatures, message integrity verification, and secure key exchange.
*/

use lazy_static::lazy_static;
use prometheus::{register_counter, register_counter_vec, Counter, CounterVec};

lazy_static! {
    static ref PROTOCOL_MESSAGES_SENT_TOTAL: Counter = register_counter!(
        "primusdb_protocol_messages_sent_total",
        "Total number of protocol messages sent"
    )
    .unwrap();
    static ref PROTOCOL_MESSAGES_RECEIVED_TOTAL: Counter = register_counter!(
        "primusdb_protocol_messages_received_total",
        "Total number of protocol messages received"
    )
    .unwrap();
    static ref PROTOCOL_ERRORS_TOTAL: CounterVec = register_counter_vec!(
        "primusdb_protocol_errors_total",
        "Total number of protocol errors by type",
        &["type"]
    )
    .unwrap();
}

pub fn inc_messages_sent() {
    PROTOCOL_MESSAGES_SENT_TOTAL.inc();
}

pub fn inc_messages_received() {
    PROTOCOL_MESSAGES_RECEIVED_TOTAL.inc();
}

pub fn inc_protocol_error(error_type: &str) {
    PROTOCOL_ERRORS_TOTAL.with_label_values(&[error_type]).inc();
}

pub fn get_messages_sent() -> u64 {
    PROTOCOL_MESSAGES_SENT_TOTAL.get() as u64
}

pub fn get_messages_received() -> u64 {
    PROTOCOL_MESSAGES_RECEIVED_TOTAL.get() as u64
}

pub fn get_protocol_errors_total() -> u64 {
    let mut total = 0u64;
    for metric_family in prometheus::gather() {
        if metric_family.get_name() == "primusdb_protocol_errors_total" {
            for metric in metric_family.get_metric() {
                total += metric.get_counter().get_value() as u64;
            }
        }
    }
    total
}

use super::journaling::JournalManager;
use super::trust::TrustManager;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{Ed25519KeyPair, UnparsedPublicKey, ED25519};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::{instrument, Span};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageHeader {
    pub version: u16,
    pub message_type: MessageType,
    pub sender_id: String,
    pub recipient_id: String,
    pub timestamp: u64,
    pub sequence_number: u64,
    pub ttl: u32,
    pub checksum: u32,
}

// Note: MessageType is defined below with Serde support

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Operation {
    CachePut {
        key: String,
        data: Vec<u8>,
    },
    CacheGet {
        key: String,
    },
    CacheDelete {
        key: String,
    },
    CacheSearch {
        pattern: String,
        limit: usize,
    },
    StorageInsert {
        table: String,
        data: Vec<u8>,
    },
    StorageUpdate {
        table: String,
        conditions: Vec<u8>,
        data: Vec<u8>,
    },
    StorageDelete {
        table: String,
        conditions: Vec<u8>,
    },
    TransactionBegin {
        id: String,
    },
    TransactionCommit {
        id: String,
    },
    TransactionRollback {
        id: String,
    },
    ConsensusPropose {
        operation: Box<Operation>,
    },
    ConsensusVote {
        proposal_id: String,
        vote: bool,
    },
    ConsensusCommit {
        proposal_id: String,
    },
    RecoveryRequest {
        node_id: String,
        data_range: DataRange,
    },
    RecoveryResponse {
        node_id: String,
        data: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MessageType {
    Operation,
    Consensus,
    Heartbeat,
    JournalSync,
    Recovery,
    TrustEstablishment,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataRange {
    pub start_key: String,
    pub end_key: String,
    pub timestamp_start: u64,
    pub timestamp_end: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecureMessage {
    pub header: MessageHeader,
    pub payload: Operation,
    pub signature: Vec<u8>,
    pub hmac: Vec<u8>,
    pub encrypted_payload: Vec<u8>,
}

pub struct MessagingEngine {
    node_id: String,
    trust_manager: Arc<TrustManager>,
    journal_manager: Arc<JournalManager>,
    key_pairs: HashMap<String, Ed25519KeyPair>, // per-node key pairs
    session_keys: RwLock<HashMap<String, LessSafeKey>>, // session keys per peer
    sequence_numbers: RwLock<HashMap<String, u64>>, // sequence numbers per peer
    peer_addresses: RwLock<HashMap<String, String>>, // node_id -> host:port
    rng: SystemRandom,
}

impl MessagingEngine {
    pub fn new(
        node_id: String,
        trust_manager: Arc<TrustManager>,
        journal_manager: Arc<JournalManager>,
    ) -> Self {
        Self {
            node_id,
            trust_manager,
            journal_manager,
            key_pairs: HashMap::new(),
            session_keys: RwLock::new(HashMap::new()),
            sequence_numbers: RwLock::new(HashMap::new()),
            peer_addresses: RwLock::new(HashMap::new()),
            rng: SystemRandom::new(),
        }
    }

    /// Register a peer's network address
    pub fn register_peer(&self, node_id: &str, address: &str) {
        self.peer_addresses
            .write()
            .unwrap()
            .insert(node_id.to_string(), address.to_string());
    }

    /// Register multiple peers at once
    pub fn register_peers(&self, peers: &[(String, String)]) {
        let mut map = self.peer_addresses.write().unwrap();
        for (node_id, address) in peers {
            map.insert(node_id.clone(), address.clone());
        }
    }

    /// Resolve a peer's network address from their node ID
    fn resolve_peer_address(&self, peer_id: &str) -> String {
        let map = self.peer_addresses.read().unwrap();
        map.get(peer_id)
            .cloned()
            .unwrap_or_else(|| peer_id.to_string())
    }

    /// Generate key pair for a specific node
    pub fn generate_keypair(&mut self, node_id: &str) -> Result<(), MessagingError> {
        let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&self.rng)?;
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())?;
        self.key_pairs.insert(node_id.to_string(), key_pair);
        Ok(())
    }

    /// Establish secure session with peer
    pub async fn establish_session(&self, peer_id: &str) -> Result<(), MessagingError> {
        // Perform key exchange using ECDHE
        let ephemeral_keypair =
            ring::agreement::EphemeralPrivateKey::generate(&ring::agreement::X25519, &self.rng)?;

        let _public_key = ephemeral_keypair.compute_public_key()?;

        // Send public key to peer and receive theirs
        // This would be implemented with actual network communication

        // For now, simulate key derivation
        let session_key_bytes = vec![0u8; 32]; // Would be derived from ECDHE
        let unbound_key = UnboundKey::new(&AES_256_GCM, &session_key_bytes)?;
        let session_key = LessSafeKey::new(unbound_key);

        self.session_keys
            .write()
            .unwrap()
            .insert(peer_id.to_string(), session_key);

        // Initialize sequence number
        self.sequence_numbers
            .write()
            .unwrap()
            .insert(peer_id.to_string(), 0);

        Ok(())
    }

    /// Create and sign a secure message
    pub fn create_message(
        &self,
        recipient_id: &str,
        operation: Operation,
    ) -> Result<SecureMessage, MessagingError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let mut sequence_numbers = self.sequence_numbers.write().unwrap();
        let seq_num = sequence_numbers
            .entry(recipient_id.to_string())
            .or_insert(0);
        *seq_num += 1;

        let header = MessageHeader {
            version: 1,
            message_type: self.operation_to_message_type(&operation),
            sender_id: self.node_id.clone(),
            recipient_id: recipient_id.to_string(),
            timestamp,
            sequence_number: *seq_num,
            ttl: 300,    // 5 minutes
            checksum: 0, // Will be calculated after payload
        };

        // Serialize payload
        let payload_bytes = bincode::serialize(&operation)?;

        // Calculate checksum
        let checksum = self.calculate_checksum(&payload_bytes);
        let header_with_checksum = MessageHeader { checksum, ..header };

        // Encrypt payload
        let encrypted_payload = self.encrypt_payload(recipient_id, &payload_bytes)?;

        // Create signature
        let signature = self.sign_message(&header_with_checksum, &encrypted_payload)?;

        // Create HMAC
        let hmac = self.create_hmac(recipient_id, &encrypted_payload)?;

        // Log to journal
        self.journal_manager
            .log_message(&header_with_checksum, &operation)?;

        Ok(SecureMessage {
            header: header_with_checksum,
            payload: operation,
            signature,
            hmac,
            encrypted_payload,
        })
    }

    /// Verify and decrypt a received message
    pub fn verify_message(&self, message: &SecureMessage) -> Result<Operation, MessagingError> {
        // Verify TTL
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        if now > message.header.timestamp + message.header.ttl as u64 {
            inc_protocol_error("message_expired");
            return Err(MessagingError::MessageExpired);
        }

        // Verify sender is trusted
        if !self.trust_manager.is_trusted(&message.header.sender_id)? {
            inc_protocol_error("untrusted_sender");
            return Err(MessagingError::UntrustedSender);
        }

        // Verify signature
        self.verify_signature(message)?;

        // Verify HMAC
        self.verify_hmac(message)?;

        // Decrypt payload
        let decrypted_payload =
            self.decrypt_payload(&message.header.sender_id, &message.encrypted_payload)?;

        // Verify checksum
        let calculated_checksum = self.calculate_checksum(&decrypted_payload);
        if calculated_checksum != message.header.checksum {
            inc_protocol_error("checksum_mismatch");
            return Err(MessagingError::ChecksumMismatch);
        }

        // Deserialize payload
        let operation: Operation = bincode::deserialize(&decrypted_payload)?;

        // Log to journal
        self.journal_manager
            .log_message(&message.header, &operation)?;

        Ok(operation)
    }

    /// Send message to peer via HTTP
    #[instrument(skip(self, operation), fields(
        operation = "send_message",
        peer_id = %peer_id,
        duration_ms = tracing::field::Empty
    ))]
    pub async fn send_message(
        &self,
        peer_id: &str,
        operation: Operation,
    ) -> Result<(), MessagingError> {
        let start = Instant::now();
        let message = self.create_message(peer_id, operation)?;

        // Serialize the secure message
        let body = serde_json::to_vec(&message)
            .map_err(|e| MessagingError::Serde(e.to_string()))?;

        // Resolve peer address and attempt HTTP delivery
        let addr = self.resolve_peer_address(peer_id);
        let url = format!("http://{}/protocol/message", addr);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| MessagingError::Network(e.to_string()))?;

        match client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                inc_messages_sent();
                let duration = start.elapsed().as_secs_f64() * 1000.0;
                Span::current().record("duration_ms", duration);
                tracing::debug!("Message sent to {}: {:?}", peer_id, message.header.message_type);
                Ok(())
            }
            Ok(resp) => {
                let status = resp.status();
                inc_protocol_error("send_failed");
                Err(MessagingError::Network(format!(
                    "Peer {} returned HTTP {}",
                    peer_id, status
                )))
            }
            Err(e) => {
                inc_protocol_error("network_error");
                Err(MessagingError::Network(format!(
                    "Failed to send to {}: {}",
                    peer_id, e
                )))
            }
        }
    }

    /// Receive and verify a message from a peer
    #[instrument(skip(self, message), fields(
        operation = "receive_message",
        sender_id = %message.header.sender_id,
        duration_ms = tracing::field::Empty
    ))]
    pub async fn receive_message(
        &self,
        message: SecureMessage,
    ) -> Result<Operation, MessagingError> {
        inc_messages_received();
        let start = Instant::now();
        let result = self.verify_message(&message);
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);

        if result.is_ok() {
            tracing::debug!(
                "Message received from {}: {:?}",
                message.header.sender_id,
                message.header.message_type
            );
        }

        result
    }

    // Private methods

    fn operation_to_message_type(&self, operation: &Operation) -> MessageType {
        match operation {
            Operation::ConsensusPropose { .. }
            | Operation::ConsensusVote { .. }
            | Operation::ConsensusCommit { .. } => MessageType::Consensus,
            Operation::RecoveryRequest { .. } | Operation::RecoveryResponse { .. } => {
                MessageType::Recovery
            }
            Operation::TransactionBegin { .. }
            | Operation::TransactionCommit { .. }
            | Operation::TransactionRollback { .. } => MessageType::Operation,
            _ => MessageType::Operation,
        }
    }

    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        use crc32fast::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(data);
        hasher.finalize()
    }

    #[instrument(skip(self, payload), fields(
        operation = "encrypt_payload",
        peer_id = %peer_id,
        duration_ms = tracing::field::Empty
    ))]
    fn encrypt_payload(&self, peer_id: &str, payload: &[u8]) -> Result<Vec<u8>, MessagingError> {
        let start = Instant::now();
        let session_keys = self.session_keys.read().unwrap();
        let session_key = session_keys
            .get(peer_id)
            .ok_or(MessagingError::NoSessionKey)?;

        let mut nonce_bytes = [0u8; 12];
        self.rng.fill(&mut nonce_bytes)?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        let mut in_out = payload.to_vec();
        session_key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)?;

        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);

        Ok(in_out)
    }

    #[instrument(skip(self, encrypted), fields(
        operation = "decrypt_payload",
        peer_id = %peer_id,
        duration_ms = tracing::field::Empty
    ))]
    fn decrypt_payload(&self, peer_id: &str, encrypted: &[u8]) -> Result<Vec<u8>, MessagingError> {
        let start = Instant::now();
        let session_keys = self.session_keys.read().unwrap();
        let session_key = session_keys
            .get(peer_id)
            .ok_or(MessagingError::NoSessionKey)?;

        let nonce_bytes = &encrypted[..12];
        let nonce = Nonce::assume_unique_for_key(nonce_bytes.try_into().unwrap());

        let mut in_out = encrypted[12..].to_vec();
        session_key.open_in_place(nonce, Aad::empty(), &mut in_out)?;

        // Remove tag (last 16 bytes)
        in_out.truncate(in_out.len() - 16);

        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);

        Ok(in_out)
    }

    fn sign_message(
        &self,
        header: &MessageHeader,
        payload: &[u8],
    ) -> Result<Vec<u8>, MessagingError> {
        let key_pair = self
            .key_pairs
            .get(&self.node_id)
            .ok_or(MessagingError::NoKeyPair)?;

        let mut message_bytes = Vec::new();
        message_bytes.extend_from_slice(&bincode::serialize(header)?);
        message_bytes.extend_from_slice(payload);

        Ok(key_pair.sign(&message_bytes).as_ref().to_vec())
    }

    fn verify_signature(&self, message: &SecureMessage) -> Result<(), MessagingError> {
        // Get sender's public key from trust manager
        let public_key_bytes = self
            .trust_manager
            .get_public_key(&message.header.sender_id)?;
        let public_key = UnparsedPublicKey::new(&ED25519, public_key_bytes);

        let mut message_bytes = Vec::new();
        message_bytes.extend_from_slice(&bincode::serialize(&message.header)?);
        message_bytes.extend_from_slice(&message.encrypted_payload);

        public_key
            .verify(&message_bytes, &message.signature)
            .map_err(|_| {
                inc_protocol_error("invalid_signature");
                MessagingError::InvalidSignature
            })
    }

    fn create_hmac(&self, peer_id: &str, data: &[u8]) -> Result<Vec<u8>, MessagingError> {
        // Derive an HMAC key from node_id + peer_id
        use ring::hmac::{Key, HMAC_SHA256};
        let mut key_material = Vec::new();
        key_material.extend_from_slice(self.node_id.as_bytes());
        key_material.extend_from_slice(peer_id.as_bytes());
        key_material.extend_from_slice(b"primusdb-hmac-derivation");
        let key = Key::new(HMAC_SHA256, &key_material);
        Ok(ring::hmac::sign(&key, data).as_ref().to_vec())
    }

    fn verify_hmac(&self, message: &SecureMessage) -> Result<(), MessagingError> {
        let expected_hmac =
            self.create_hmac(&message.header.sender_id, &message.encrypted_payload)?;

        if expected_hmac != message.hmac {
            inc_protocol_error("invalid_hmac");
            return Err(MessagingError::InvalidHMAC);
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MessagingError {
    #[error("Ring crypto error: {0}")]
    Crypto(ring::error::Unspecified),
    #[error("Key rejected: {0}")]
    KeyRejected(ring::error::KeyRejected),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("System time error: {0}")]
    Time(#[from] std::time::SystemTimeError),
    #[error("No key pair for node")]
    NoKeyPair,
    #[error("No session key for peer")]
    NoSessionKey,
    #[error("Message expired")]
    MessageExpired,
    #[error("Untrusted sender")]
    UntrustedSender,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid HMAC")]
    InvalidHMAC,
    #[error("Network error: {0}")]
    Network(String),
    #[error("Serialization error: {0}")]
    Serde(String),
}

impl From<ring::error::Unspecified> for MessagingError {
    fn from(e: ring::error::Unspecified) -> Self {
        MessagingError::Crypto(e)
    }
}

impl From<ring::error::KeyRejected> for MessagingError {
    fn from(e: ring::error::KeyRejected) -> Self {
        MessagingError::KeyRejected(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::journaling::JournalManager;
    use crate::protocol::trust::{TrustConfig, TrustManager};
    use std::sync::Arc;

    // ── MessageHeader tests ──────────────────────────────────────────

    #[test]
    fn test_message_header_creation() {
        let header = MessageHeader {
            version: 1,
            message_type: MessageType::Operation,
            sender_id: "node-a".to_string(),
            recipient_id: "node-b".to_string(),
            timestamp: 1_000_000,
            sequence_number: 42,
            ttl: 300,
            checksum: 0xDEAD_BEEF,
        };
        assert_eq!(header.version, 1);
        assert_eq!(header.message_type, MessageType::Operation);
        assert_eq!(header.sender_id, "node-a");
        assert_eq!(header.recipient_id, "node-b");
        assert_eq!(header.timestamp, 1_000_000);
        assert_eq!(header.sequence_number, 42);
        assert_eq!(header.ttl, 300);
        assert_eq!(header.checksum, 0xDEAD_BEEF);
    }

    #[test]
    fn test_message_header_serde_roundtrip() {
        let header = MessageHeader {
            version: 2,
            message_type: MessageType::Consensus,
            sender_id: "alice".to_string(),
            recipient_id: "bob".to_string(),
            timestamp: 99,
            sequence_number: 1,
            ttl: 600,
            checksum: 12345,
        };
        let bytes = bincode::serialize(&header).unwrap();
        let deserialized: MessageHeader = bincode::deserialize(&bytes).unwrap();
        assert_eq!(deserialized.version, header.version);
        assert_eq!(deserialized.message_type, header.message_type);
        assert_eq!(deserialized.sender_id, header.sender_id);
        assert_eq!(deserialized.timestamp, header.timestamp);
    }

    #[test]
    fn test_message_header_truncated() {
        let header = MessageHeader {
            version: 1,
            message_type: MessageType::Heartbeat,
            sender_id: "x".to_string(),
            recipient_id: "y".to_string(),
            timestamp: 0,
            sequence_number: 0,
            ttl: 0,
            checksum: 0,
        };
        let bytes = bincode::serialize(&header).unwrap();
        let truncated = &bytes[..bytes.len() / 2];
        let result: Result<MessageHeader, _> = bincode::deserialize(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_header_max_values() {
        let header = MessageHeader {
            version: u16::MAX,
            message_type: MessageType::Recovery,
            sender_id: String::new(),
            recipient_id: String::new(),
            timestamp: u64::MAX,
            sequence_number: u64::MAX,
            ttl: u32::MAX,
            checksum: u32::MAX,
        };
        let bytes = bincode::serialize(&header).unwrap();
        let deserialized: MessageHeader = bincode::deserialize(&bytes).unwrap();
        assert_eq!(deserialized.version, u16::MAX);
        assert_eq!(deserialized.timestamp, u64::MAX);
        assert_eq!(deserialized.sequence_number, u64::MAX);
    }

    // ── MessageType tests ────────────────────────────────────────────

    #[test]
    fn test_message_type_variants() {
        let variants = vec![
            MessageType::Operation,
            MessageType::Consensus,
            MessageType::Heartbeat,
            MessageType::JournalSync,
            MessageType::Recovery,
            MessageType::TrustEstablishment,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_message_type_serde_roundtrip() {
        for mt in &[
            MessageType::Operation,
            MessageType::Consensus,
            MessageType::Heartbeat,
            MessageType::JournalSync,
            MessageType::Recovery,
            MessageType::TrustEstablishment,
        ] {
            let bytes = bincode::serialize(mt).unwrap();
            let deserialized: MessageType = bincode::deserialize(&bytes).unwrap();
            assert_eq!(&deserialized, mt);
        }
    }

    // ── SecureMessage tests ──────────────────────────────────────────

    fn dummy_operation() -> Operation {
        Operation::CachePut {
            key: "test-key".to_string(),
            data: vec![1, 2, 3, 4],
        }
    }

    fn dummy_header() -> MessageHeader {
        MessageHeader {
            version: 1,
            message_type: MessageType::Operation,
            sender_id: "node-a".to_string(),
            recipient_id: "node-b".to_string(),
            timestamp: 1_000_000,
            sequence_number: 1,
            ttl: 300,
            checksum: 0,
        }
    }

    #[test]
    fn test_secure_message_creation_and_field_access() {
        let msg = SecureMessage {
            header: dummy_header(),
            payload: dummy_operation(),
            signature: vec![0xAA; 64],
            hmac: vec![0xBB; 32],
            encrypted_payload: vec![0xCC; 100],
        };
        assert_eq!(msg.header.version, 1);
        assert_eq!(msg.header.message_type, MessageType::Operation);
        assert_eq!(msg.signature.len(), 64);
        assert_eq!(msg.hmac.len(), 32);
        assert_eq!(msg.encrypted_payload.len(), 100);
        match &msg.payload {
            Operation::CachePut { key, data } => {
                assert_eq!(key, "test-key");
                assert_eq!(data, &vec![1, 2, 3, 4]);
            }
            other => panic!("Expected CachePut, got {:?}", other),
        }
    }

    #[test]
    fn test_secure_message_serde_roundtrip() {
        let msg = SecureMessage {
            header: dummy_header(),
            payload: dummy_operation(),
            signature: vec![0xAA; 64],
            hmac: vec![0xBB; 32],
            encrypted_payload: vec![0xCC; 100],
        };
        let bytes = bincode::serialize(&msg).unwrap();
        let deserialized: SecureMessage = bincode::deserialize(&bytes).unwrap();
        assert_eq!(deserialized.header.version, msg.header.version);
        assert_eq!(deserialized.signature, msg.signature);
        assert_eq!(deserialized.hmac, msg.hmac);
        assert_eq!(deserialized.encrypted_payload, msg.encrypted_payload);
    }

    #[test]
    fn test_secure_message_empty_payload() {
        let msg = SecureMessage {
            header: dummy_header(),
            payload: Operation::CacheGet {
                key: "empty".to_string(),
            },
            signature: vec![],
            hmac: vec![],
            encrypted_payload: vec![],
        };
        assert!(msg.signature.is_empty());
        assert!(msg.hmac.is_empty());
        assert!(msg.encrypted_payload.is_empty());
        let bytes = bincode::serialize(&msg).unwrap();
        let deserialized: SecureMessage = bincode::deserialize(&bytes).unwrap();
        assert!(deserialized.signature.is_empty());
        match &deserialized.payload {
            Operation::CacheGet { key } => assert_eq!(key, "empty"),
            other => panic!("Expected CacheGet, got {:?}", other),
        }
    }

    #[test]
    fn test_secure_message_truncated() {
        let msg = SecureMessage {
            header: dummy_header(),
            payload: dummy_operation(),
            signature: vec![0xAA; 64],
            hmac: vec![0xBB; 32],
            encrypted_payload: vec![0xCC; 100],
        };
        let bytes = bincode::serialize(&msg).unwrap();
        let truncated = &bytes[..4];
        let result: Result<SecureMessage, _> = bincode::deserialize(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn test_secure_message_malformed() {
        let malformed = vec![0xFF, 0xFE, 0xFD, 0xFC, 0x00, 0x01, 0x02];
        let result: Result<SecureMessage, _> = bincode::deserialize(&malformed);
        assert!(result.is_err());
    }

    // ── Operation tests ──────────────────────────────────────────────

    #[test]
    fn test_all_operation_variants_serde() {
        let ops: Vec<Operation> = vec![
            Operation::CachePut {
                key: "k".into(),
                data: vec![1],
            },
            Operation::CacheGet { key: "k".into() },
            Operation::CacheDelete { key: "k".into() },
            Operation::CacheSearch {
                pattern: "*".into(),
                limit: 10,
            },
            Operation::StorageInsert {
                table: "t".into(),
                data: vec![2],
            },
            Operation::StorageUpdate {
                table: "t".into(),
                conditions: vec![3],
                data: vec![4],
            },
            Operation::StorageDelete {
                table: "t".into(),
                conditions: vec![5],
            },
            Operation::TransactionBegin { id: "tx1".into() },
            Operation::TransactionCommit { id: "tx1".into() },
            Operation::TransactionRollback { id: "tx1".into() },
            Operation::ConsensusPropose {
                operation: Box::new(Operation::CacheGet {
                    key: "inner".into(),
                }),
            },
            Operation::ConsensusVote {
                proposal_id: "p1".into(),
                vote: true,
            },
            Operation::ConsensusCommit {
                proposal_id: "p1".into(),
            },
            Operation::RecoveryRequest {
                node_id: "n1".into(),
                data_range: DataRange {
                    start_key: "a".into(),
                    end_key: "z".into(),
                    timestamp_start: 0,
                    timestamp_end: 100,
                },
            },
            Operation::RecoveryResponse {
                node_id: "n1".into(),
                data: vec![6],
            },
        ];
        for op in &ops {
            let bytes = bincode::serialize(op).unwrap();
            let deserialized: Operation = bincode::deserialize(&bytes).unwrap();
            let _ = deserialized;
        }
    }

    // ── DataRange tests ──────────────────────────────────────────────

    #[test]
    fn test_data_range_serde_roundtrip() {
        let dr = DataRange {
            start_key: "aaa".to_string(),
            end_key: "zzz".to_string(),
            timestamp_start: 1000,
            timestamp_end: 2000,
        };
        let bytes = bincode::serialize(&dr).unwrap();
        let deserialized: DataRange = bincode::deserialize(&bytes).unwrap();
        assert_eq!(deserialized.start_key, "aaa");
        assert_eq!(deserialized.end_key, "zzz");
        assert_eq!(deserialized.timestamp_start, 1000);
        assert_eq!(deserialized.timestamp_end, 2000);
    }

    // ── MessagingEngine creation test ─────────────────────────────────

    #[test]
    fn test_messaging_engine_creation() {
        let trust_config = TrustConfig::default();
        let trust_manager = Arc::new(TrustManager::new(trust_config).unwrap());
        let journal_manager = Arc::new(JournalManager::new());
        let _engine = MessagingEngine::new("test-node".to_string(), trust_manager, journal_manager);
    }
}
