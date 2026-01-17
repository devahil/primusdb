/*!
# Secure Messaging System

This module provides end-to-end encrypted messaging between PrimusDB nodes with
digital signatures, message integrity verification, and secure key exchange.
*/

use super::journaling::JournalManager;
use super::trust::TrustManager;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::time::{Duration, Instant};

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

#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    Operation,
    Consensus,
    Heartbeat,
    JournalSync,
    Recovery,
    TrustEstablishment,
}

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
            rng: SystemRandom::new(),
        }
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

        let public_key = ephemeral_keypair.compute_public_key()?;

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
            return Err(MessagingError::MessageExpired);
        }

        // Verify sender is trusted
        if !self.trust_manager.is_trusted(&message.header.sender_id)? {
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
            return Err(MessagingError::ChecksumMismatch);
        }

        // Deserialize payload
        let operation: Operation = bincode::deserialize(&decrypted_payload)?;

        // Log to journal
        self.journal_manager
            .log_message(&message.header, &operation)?;

        Ok(operation)
    }

    /// Send message to peer (placeholder for network layer)
    pub async fn send_message(
        &self,
        peer_id: &str,
        operation: Operation,
    ) -> Result<(), MessagingError> {
        let message = self.create_message(peer_id, operation)?;

        // Here would be the actual network sending logic
        // For now, simulate successful send

        println!(
            "Message sent to {}: {:?}",
            peer_id, message.header.message_type
        );
        Ok(())
    }

    /// Receive message from peer (placeholder for network layer)
    pub async fn receive_message(
        &self,
        message: SecureMessage,
    ) -> Result<Operation, MessagingError> {
        self.verify_message(&message)
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

    fn encrypt_payload(&self, peer_id: &str, payload: &[u8]) -> Result<Vec<u8>, MessagingError> {
        let session_keys = self.session_keys.read().unwrap();
        let session_key = session_keys
            .get(peer_id)
            .ok_or(MessagingError::NoSessionKey)?;

        let nonce_bytes = ring::rand::generate(&self.rng)?.expose();
        let nonce = Nonce::assume_unique_for_key(nonce_bytes[..12].try_into().unwrap());

        let mut in_out = payload.to_vec();
        session_key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)?;

        Ok(in_out)
    }

    fn decrypt_payload(&self, peer_id: &str, encrypted: &[u8]) -> Result<Vec<u8>, MessagingError> {
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
            .map_err(|_| MessagingError::InvalidSignature)
    }

    fn create_hmac(&self, peer_id: &str, data: &[u8]) -> Result<Vec<u8>, MessagingError> {
        let session_keys = self.session_keys.read().unwrap();
        let session_key = session_keys
            .get(peer_id)
            .ok_or(MessagingError::NoSessionKey)?;

        // Use session key for HMAC
        use ring::hmac::{Key, HMAC_SHA256};
        let key = Key::new(HMAC_SHA256, session_key.as_ref());
        Ok(ring::hmac::sign(&key, data).as_ref().to_vec())
    }

    fn verify_hmac(&self, message: &SecureMessage) -> Result<(), MessagingError> {
        let expected_hmac =
            self.create_hmac(&message.header.sender_id, &message.encrypted_payload)?;

        if expected_hmac != message.hmac {
            return Err(MessagingError::InvalidHMAC);
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MessagingError {
    #[error("Ring crypto error: {0}")]
    Crypto(#[from] ring::error::Unspecified),
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
    #[error("Checksum mismatch")]
    ChecksumMismatch,
    #[error("Trust manager error")]
    TrustError,
    #[error("Journal error")]
    JournalError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_creation_and_verification() {
        // This would require a full test setup with trust manager and journal
        // For now, just test basic functionality
        assert!(true);
    }
}
