/*!
# Node Communication Handlers

This module provides handlers for secure node-to-node communication,
managing connections, message routing, and protocol state.
*/

use super::journaling::JournalManager;
use super::messaging::{MessageType, MessagingEngine, Operation, SecureMessage};
use super::recovery::RecoveryManager;
use super::trust::TrustManager;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

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

    async fn handle_connection(&self, mut socket: TcpStream) -> Result<(), HandlerError> {
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

        // Route operation
        match operation {
            Operation::CachePut { key, data } => {
                // Handle cache put
                println!("Received cache put for key: {}", key);
            }
            Operation::ConsensusPropose { operation } => {
                // Handle consensus proposal
                println!("Received consensus proposal");
            }
            _ => {
                // Handle other operations
            }
        }

        // Send acknowledgment
        let response = b"ACK";
        socket.write_all(response).await?;

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

    pub async fn broadcast_operation(&self, operation: Operation) -> Result<(), HandlerError> {
        let connections = self.connections.read().unwrap();
        for peer_id in connections.keys() {
            self.messaging_engine
                .send_message(peer_id, operation.clone())
                .await?;
        }
        Ok(())
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
    #[error("Messaging error")]
    MessagingError,
}
