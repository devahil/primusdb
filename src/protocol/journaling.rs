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
            .or_insert_with(Vec::new)
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
