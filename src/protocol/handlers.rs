/*!
# Node Communication Handlers

This module provides handlers for secure node-to-node communication,
managing connections, message routing, and protocol state.
*/

use super::journaling::JournalManager;
use super::messaging::{MessagingEngine, Operation, SecureMessage};
use super::recovery::RecoveryManager;
use super::trust::TrustManager;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{instrument, Span};

pub struct NodeCommunicationHandler {
    node_id: String,
    messaging_engine: Arc<MessagingEngine>,
    trust_manager: Arc<TrustManager>,
    journal_manager: Arc<JournalManager>,
    recovery_manager: Arc<RecoveryManager>,
    connections: RwLock<HashMap<String, mpsc::Sender<SecureMessage>>>,
}

impl NodeCommunicationHandler {
    pub fn new(
        node_id: String,
        messaging_engine: Arc<MessagingEngine>,
        trust_manager: Arc<TrustManager>,
        journal_manager: Arc<JournalManager>,
        recovery_manager: Arc<RecoveryManager>,
    ) -> Self {
        Self {
            node_id,
            messaging_engine,
            trust_manager,
            journal_manager,
            recovery_manager,
            connections: RwLock::new(HashMap::new()),
        }
    }

    pub async fn start_server(&self, address: &str) -> Result<(), HandlerError> {
        let listener = TcpListener::bind(address).await?;

        println!("Node {} listening on {}", self.node_id, address);

        loop {
            let (socket, _) = listener.accept().await?;
            let handler = self.clone();

            tokio::spawn(async move {
                if let Err(e) = handler.handle_connection(socket).await {
                    eprintln!("Connection error: {}", e);
                }
            });
        }
    }

    pub async fn connect_to_peer(&self, peer_address: &str) -> Result<(), HandlerError> {
        let stream = TcpStream::connect(peer_address).await?;
        let handler = self.clone();

        tokio::spawn(async move {
            if let Err(e) = handler.handle_connection(stream).await {
                eprintln!("Peer connection error: {}", e);
            }
        });

        Ok(())
    }

    #[instrument(skip(self, socket), fields(
        operation = "handle_message",
        duration_ms = tracing::field::Empty
    ))]
    async fn handle_connection(&self, mut socket: TcpStream) -> Result<(), HandlerError> {
        let start = Instant::now();
        let mut buffer = [0u8; 8192];

        // Read handshake
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            return Ok(());
        }

        // Parse message (simplified - in real implementation would use proper framing)
        let message_data = &buffer[..n];
        let message: SecureMessage = bincode::deserialize(message_data)?;

        // Verify and process message
        let operation = self.messaging_engine.verify_message(&message)?;

        // Route operation to appropriate handler
        self.route_operation(&operation).await?;

        // Send acknowledgment
        let response = b"ACK";
        socket.write_all(response).await?;

        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);

        Ok(())
    }

    pub async fn send_operation(
        &self,
        peer_id: &str,
        operation: Operation,
    ) -> Result<(), HandlerError> {
        self.messaging_engine
            .send_message(peer_id, operation)
            .await?;
        Ok(())
    }

    #[instrument(skip(self, operation), fields(
        operation = "broadcast_operation",
        peer_count = tracing::field::Empty,
        duration_ms = tracing::field::Empty
    ))]
    pub async fn broadcast_operation(&self, operation: Operation) -> Result<(), HandlerError> {
        let start = Instant::now();
        let peer_ids: Vec<String> = {
            let connections = self.connections.read().unwrap();
            connections.keys().cloned().collect()
        };
        Span::current().record("peer_count", peer_ids.len());
        for peer_id in peer_ids {
            self.messaging_engine
                .send_message(&peer_id, operation.clone())
                .await?;
        }
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);
        Ok(())
    }
}

    /// Route an operation to its handler
    async fn route_operation(&self, operation: &Operation) -> Result<(), HandlerError> {
        use super::messaging::{MessageHeader, MessageType};

        let header = MessageHeader {
            version: 1,
            message_type: MessageType::Operation,
            sender_id: self.node_id.clone(),
            recipient_id: String::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            sequence_number: 0,
            ttl: 300,
            checksum: 0,
        };

        self.journal_manager.log_message(&header, operation)?;

        match operation {
            Operation::CachePut { key, data } => {
                tracing::info!("Cache put for key: {} ({} bytes)", key, data.len());
            }
            Operation::CacheGet { key } => {
                tracing::info!("Cache get for key: {}", key);
            }
            Operation::CacheDelete { key } => {
                tracing::info!("Cache delete for key: {}", key);
            }
            Operation::CacheSearch { pattern, limit } => {
                tracing::info!("Cache search for pattern '{}' (limit {})", pattern, limit);
            }
            Operation::StorageInsert { table, data } => {
                tracing::info!("Storage insert into table '{}' ({} bytes)", table, data.len());
            }
            Operation::StorageUpdate { table, conditions, data } => {
                tracing::info!("Storage update on table '{}' ({} conditions, {} bytes)", table, conditions.len(), data.len());
            }
            Operation::StorageDelete { table, conditions } => {
                tracing::info!("Storage delete on table '{}' ({} conditions)", table, conditions.len());
            }
            Operation::TransactionBegin { id } => {
                tracing::info!("Transaction begin: {}", id);
            }
            Operation::TransactionCommit { id } => {
                tracing::info!("Transaction commit: {}", id);
            }
            Operation::TransactionRollback { id } => {
                tracing::info!("Transaction rollback: {}", id);
            }
            Operation::ConsensusPropose { operation: inner } => {
                tracing::info!("Consensus proposal received");
                Box::pin(self.route_operation(inner)).await?;
            }
            Operation::ConsensusVote { proposal_id, vote } => {
                tracing::info!("Consensus vote on {}: {}", proposal_id, vote);
            }
            Operation::ConsensusCommit { proposal_id } => {
                tracing::info!("Consensus commit: {}", proposal_id);
            }
            Operation::RecoveryRequest { node_id, data_range } => {
                tracing::info!("Recovery request from {} (keys {}..{})", node_id, data_range.start_key, data_range.end_key);
                let plan = self.recovery_manager.create_recovery_plan(
                    &node_id,
                    crate::protocol::recovery::ErrorType::DataCorruption,
                    vec![data_range.start_key.clone(), data_range.end_key.clone()],
                );
                let _ = self.recovery_manager.execute_recovery(plan);
            }
            Operation::RecoveryResponse { node_id, data } => {
                tracing::info!("Recovery response from {} ({} bytes)", node_id, data.len());
            }
        }
        Ok(())
    }

    /// Register a connected peer
    pub fn register_connection(&self, peer_id: String, sender: mpsc::Sender<SecureMessage>) {
        self.connections
            .write()
            .unwrap()
            .insert(peer_id, sender);
    }

    /// Unregister a disconnected peer
    pub fn unregister_connection(&self, peer_id: &str) {
        self.connections.write().unwrap().remove(peer_id);
    }

    /// Get list of connected peer IDs
    pub fn connected_peers(&self) -> Vec<String> {
        self.connections.read().unwrap().keys().cloned().collect()
    }
}

impl Clone for NodeCommunicationHandler {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id.clone(),
            messaging_engine: Arc::clone(&self.messaging_engine),
            trust_manager: Arc::clone(&self.trust_manager),
            journal_manager: Arc::clone(&self.journal_manager),
            recovery_manager: Arc::clone(&self.recovery_manager),
            connections: RwLock::new(HashMap::new()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("Messaging error: {0}")]
    MessagingError(#[from] super::messaging::MessagingError),
    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_error_display() {
        let err = HandlerError::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "conn refused",
        ));
        assert!(err.to_string().contains("conn refused"));
    }

    #[test]
    fn test_handler_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err: HandlerError = io_err.into();
        assert!(matches!(err, HandlerError::Io(_)));
    }

    #[test]
    fn test_handler_error_serialization() {
        let bincode_err = bincode::ErrorKind::Custom("bad data".to_string());
        let err: HandlerError = bincode::Error::from(bincode_err).into();
        assert!(matches!(err, HandlerError::Serialization(_)));
    }

    #[test]
    fn test_handler_error_messaging() {
        let err = HandlerError::MessagingError(super::super::messaging::MessagingError::NoKeyPair);
        assert!(err.to_string().contains("No key pair"));
    }

    #[test]
    fn test_handler_error_locked_poisoned() {
        let err = HandlerError::LockPoisoned("lock was poisoned".to_string());
        assert!(err.to_string().contains("lock was poisoned"));
    }

    #[test]
    fn test_node_communication_handler_creation() {
        let trust_config = crate::protocol::trust::TrustConfig::default();
        let trust_manager =
            Arc::new(crate::protocol::trust::TrustManager::new(trust_config).unwrap());
        let journal_manager = Arc::new(crate::protocol::journaling::JournalManager::new());
        let recovery_manager = Arc::new(crate::protocol::recovery::RecoveryManager::new());
        let messaging_engine = Arc::new(crate::protocol::messaging::MessagingEngine::new(
            "test-node".to_string(),
            trust_manager,
            journal_manager.clone(),
        ));
        let _handler = NodeCommunicationHandler::new(
            "test-node".to_string(),
            messaging_engine,
            Arc::new(
                crate::protocol::trust::TrustManager::new(
                    crate::protocol::trust::TrustConfig::default(),
                )
                .unwrap(),
            ),
            journal_manager,
            recovery_manager,
        );
    }
}
