/*!
# Distributed Journaling System

This module provides comprehensive journaling for distributed operations,
including transaction traces, operation logs, and recovery information.
*/

use super::messaging::{MessageHeader, Operation};
use std::collections::HashMap;
use std::sync::RwLock;

pub struct JournalManager {
    journals: RwLock<HashMap<String, Vec<JournalEntry>>>,
}

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub timestamp: u64,
    pub operation_id: String,
    pub operation: Operation,
    pub node_id: String,
    pub checksum: u32,
    pub status: OperationStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperationStatus {
    Initiated,
    InProgress,
    Completed,
    Failed,
    RolledBack,
}

impl Default for JournalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl JournalManager {
    pub fn new() -> Self {
        Self {
            journals: RwLock::new(HashMap::new()),
        }
    }

    pub fn log_message(
        &self,
        header: &MessageHeader,
        operation: &Operation,
    ) -> Result<(), JournalError> {
        let entry = JournalEntry {
            timestamp: header.timestamp,
            operation_id: format!("{}-{}", header.sender_id, header.sequence_number),
            operation: operation.clone(),
            node_id: header.sender_id.clone(),
            checksum: header.checksum,
            status: OperationStatus::Initiated,
        };

        let mut journals = self.journals.write().unwrap();
        journals
            .entry(header.sender_id.clone())
            .or_default()
            .push(entry);

        Ok(())
    }

    pub fn update_operation_status(
        &self,
        operation_id: &str,
        status: OperationStatus,
    ) -> Result<(), JournalError> {
        let mut journals = self.journals.write().unwrap();
        for entries in journals.values_mut() {
            for entry in entries {
                if entry.operation_id == operation_id {
                    entry.status = status;
                    return Ok(());
                }
            }
        }
        Err(JournalError::OperationNotFound)
    }

    pub fn get_operation_history(&self, node_id: &str) -> Vec<JournalEntry> {
        let journals = self.journals.read().unwrap();
        journals.get(node_id).cloned().unwrap_or_default()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("Operation not found")]
    OperationNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_header() -> MessageHeader {
        MessageHeader {
            version: 1,
            message_type: crate::protocol::messaging::MessageType::Operation,
            sender_id: "node-a".to_string(),
            recipient_id: "node-b".to_string(),
            timestamp: 1000,
            sequence_number: 1,
            ttl: 300,
            checksum: 0x1234,
        }
    }

    fn dummy_operation() -> Operation {
        Operation::CachePut {
            key: "test-key".to_string(),
            data: vec![1, 2, 3],
        }
    }

    #[test]
    fn test_journal_manager_creation() {
        let jm = JournalManager::new();
        let history = jm.get_operation_history("any-node");
        assert!(history.is_empty());
    }

    #[test]
    fn test_log_message() {
        let jm = JournalManager::new();
        jm.log_message(&dummy_header(), &dummy_operation()).unwrap();
        let history = jm.get_operation_history("node-a");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].operation_id, "node-a-1");
    }

    #[test]
    fn test_log_multiple_messages_same_node() {
        let jm = JournalManager::new();
        for i in 0..5 {
            let mut h = dummy_header();
            h.sequence_number = i as u64;
            h.timestamp = 1000 + i;
            jm.log_message(&h, &dummy_operation()).unwrap();
        }
        let history = jm.get_operation_history("node-a");
        assert_eq!(history.len(), 5);
        for (i, entry) in history.iter().enumerate() {
            assert_eq!(entry.operation_id, format!("node-a-{}", i));
        }
    }

    #[test]
    fn test_log_messages_multiple_nodes() {
        let jm = JournalManager::new();

        let mut h1 = dummy_header();
        h1.sender_id = "node-a".to_string();
        jm.log_message(&h1, &dummy_operation()).unwrap();

        let mut h2 = dummy_header();
        h2.sender_id = "node-b".to_string();
        jm.log_message(&h2, &dummy_operation()).unwrap();

        let history_a = jm.get_operation_history("node-a");
        assert_eq!(history_a.len(), 1);
        let history_b = jm.get_operation_history("node-b");
        assert_eq!(history_b.len(), 1);
    }

    #[test]
    fn test_get_operation_history_unknown_node() {
        let jm = JournalManager::new();
        let history = jm.get_operation_history("nonexistent");
        assert!(history.is_empty());
    }

    #[test]
    fn test_update_operation_status() {
        let jm = JournalManager::new();
        jm.log_message(&dummy_header(), &dummy_operation()).unwrap();

        jm.update_operation_status("node-a-1", OperationStatus::Completed)
            .unwrap();

        let history = jm.get_operation_history("node-a");
        assert_eq!(history[0].status, OperationStatus::Completed);
    }

    #[test]
    fn test_update_nonexistent_operation() {
        let jm = JournalManager::new();
        let result = jm.update_operation_status("no-such-op", OperationStatus::Completed);
        assert!(result.is_err());
    }

    #[test]
    fn test_status_lifecycle() {
        let jm = JournalManager::new();
        jm.log_message(&dummy_header(), &dummy_operation()).unwrap();

        jm.update_operation_status("node-a-1", OperationStatus::InProgress)
            .unwrap();
        assert_eq!(
            jm.get_operation_history("node-a")[0].status,
            OperationStatus::InProgress
        );

        jm.update_operation_status("node-a-1", OperationStatus::Completed)
            .unwrap();
        assert_eq!(
            jm.get_operation_history("node-a")[0].status,
            OperationStatus::Completed
        );
    }

    #[test]
    fn test_empty_journal_get_history() {
        let jm = JournalManager::new();
        let history = jm.get_operation_history("node-a");
        assert!(history.is_empty());
    }

    #[test]
    fn test_entry_ordering() {
        let jm = JournalManager::new();
        for i in 0..10 {
            let mut h = dummy_header();
            h.sequence_number = i;
            h.timestamp = i;
            jm.log_message(&h, &dummy_operation()).unwrap();
        }
        let history = jm.get_operation_history("node-a");
        for (i, entry) in history.iter().enumerate() {
            assert_eq!(entry.timestamp, i as u64);
            assert_eq!(entry.operation_id, format!("node-a-{}", i));
        }
    }

    #[test]
    fn test_journal_entry_creation() {
        let entry = JournalEntry {
            timestamp: 1000,
            operation_id: "test-op".to_string(),
            operation: dummy_operation(),
            node_id: "node-x".to_string(),
            checksum: 0xABCD,
            status: OperationStatus::Initiated,
        };
        assert_eq!(entry.timestamp, 1000);
        assert_eq!(entry.operation_id, "test-op");
        assert_eq!(entry.node_id, "node-x");
        assert_eq!(entry.checksum, 0xABCD);
        assert_eq!(entry.status, OperationStatus::Initiated);
    }

    #[test]
    fn test_operation_status_variants_distinct() {
        assert_ne!(OperationStatus::Initiated, OperationStatus::Completed);
        assert_ne!(OperationStatus::Completed, OperationStatus::Failed);
        assert_ne!(OperationStatus::Failed, OperationStatus::RolledBack);
    }
}
