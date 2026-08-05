//! Applies committed consensus blocks to the storage engines.
//!
//! [`ConsensusStateMachine`] tracks the last applied block height and a set of
//! already-applied transaction IDs so that blocks can be applied idempotently.
//! Each transaction's operations are translated into
//! [`crate::transaction::TransactionOperation`] values and executed through
//! the per-storage-type engine registry.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::consensus::{Block, Operation, OperationType};
use crate::storage::StorageEngine;
use crate::Result;
use crate::StorageType;

/// Applies committed blocks to storage engines, tracking progress so that
/// blocks are never applied twice.
pub struct ConsensusStateMachine {
    engines: HashMap<StorageType, Arc<dyn StorageEngine>>,
    last_applied_height: Mutex<u64>,
    applied_txns: Mutex<HashSet<String>>,
}

fn parse_storage_type(s: &str) -> Result<StorageType> {
    match s {
        "Document" => Ok(StorageType::Document),
        "Relational" => Ok(StorageType::Relational),
        "Columnar" => Ok(StorageType::Columnar),
        "Vector" => Ok(StorageType::Vector),
        "KeyValue" => Ok(StorageType::KeyValue),
        "TimeSeries" => Ok(StorageType::TimeSeries),
        _ => Err(crate::Error::ValidationError(format!(
            "Unknown storage type: {}",
            s
        ))),
    }
}

impl ConsensusStateMachine {
    /// Create a state machine bound to the given engine registry.
    pub fn new(engines: HashMap<StorageType, Arc<dyn StorageEngine>>) -> Self {
        Self {
            engines,
            last_applied_height: Mutex::new(0),
            applied_txns: Mutex::new(HashSet::new()),
        }
    }

    /// Height of the most recently applied block.
    pub fn last_applied_height(&self) -> u64 {
        *self.last_applied_height.lock().unwrap()
    }

    /// Apply a committed block, skipping transactions already applied.
    pub async fn apply_block(&self, block: &Block) -> Result<()> {
        for tx in &block.transactions {
            if self.applied_txns.lock().unwrap().contains(&tx.id) {
                continue;
            }
            self.apply_transaction(tx).await?;
            self.applied_txns.lock().unwrap().insert(tx.id.clone());
        }
        *self.last_applied_height.lock().unwrap() = block.height;
        Ok(())
    }

    async fn apply_transaction(&self, tx: &crate::consensus::Transaction) -> Result<()> {
        let consensus_ops: Vec<crate::transaction::TransactionOperation> = tx
            .operations
            .iter()
            .map(|op| crate::transaction::TransactionOperation {
                id: format!("{}_{}", tx.id, op.table),
                operation_type: match op.op_type {
                    OperationType::Create => crate::transaction::OperationType::Create,
                    OperationType::Insert => crate::transaction::OperationType::Insert,
                    OperationType::Update => crate::transaction::OperationType::Update,
                    OperationType::Delete => crate::transaction::OperationType::Delete,
                    OperationType::Drop => crate::transaction::OperationType::Drop,
                    // No Read variant in consensus OperationType
                },
                table: op.table.clone(),
                data: op.data.clone(),
                conditions: op.conditions.clone(),
                before_image: None,
                after_image: None,
                executed: true,
                rollback_data: None,
                storage_type: op.storage_type.clone(),
            })
            .collect();

        let app_tx = crate::transaction::Transaction {
            id: format!("sys_{}", tx.id),
            operations: consensus_ops,
            status: crate::transaction::TransactionStatus::Active,
            created_at: tx.timestamp,
            updated_at: tx.timestamp,
            isolation_level: crate::transaction::IsolationLevel::Serializable,
            timeout_ms: 60000,
        };

        for op in &tx.operations {
            self.apply_operation(op, &app_tx).await?;
        }
        Ok(())
    }

    async fn apply_operation(
        &self,
        op: &Operation,
        app_tx: &crate::transaction::Transaction,
    ) -> Result<()> {
        let storage_type = parse_storage_type(&op.storage_type)?;

        let engine = self
            .engines
            .get(&storage_type)
            .ok_or_else(|| crate::Error::StorageEngineNotFound(storage_type))?;

        match op.op_type {
            OperationType::Create => {
                let schema: crate::storage::Schema = serde_json::from_value(op.data.clone())
                    .map_err(|e| crate::Error::ValidationError(format!("Invalid schema: {}", e)))?;
                engine.create_table(&op.table, &schema).await?;
            }
            OperationType::Insert => {
                engine.insert(&op.table, &op.data, app_tx).await?;
            }
            OperationType::Update => {
                engine
                    .update(&op.table, op.conditions.as_ref(), &op.data, app_tx)
                    .await?;
            }
            OperationType::Delete => {
                engine
                    .delete(&op.table, op.conditions.as_ref(), app_tx)
                    .await?;
            }
            OperationType::Drop => {
                engine.drop_table(&op.table).await?;
            }
        }
        Ok(())
    }
}
