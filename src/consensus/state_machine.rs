use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::consensus::{Block, Operation, OperationType};
use crate::storage::StorageEngine;
use crate::StorageType;
use crate::Result;

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
        _ => Err(crate::Error::ValidationError(format!(
            "Unknown storage type: {}",
            s
        ))),
    }
}

impl ConsensusStateMachine {
    pub fn new(engines: HashMap<StorageType, Arc<dyn StorageEngine>>) -> Self {
        Self {
            engines,
            last_applied_height: Mutex::new(0),
            applied_txns: Mutex::new(HashSet::new()),
        }
    }

    pub fn last_applied_height(&self) -> u64 {
        *self.last_applied_height.lock().unwrap()
    }

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
        let dummy_tx = crate::transaction::Transaction {
            id: format!("sys_{}", tx.id),
            operations: vec![],
            status: crate::transaction::TransactionStatus::Committed,
            created_at: tx.timestamp,
            updated_at: tx.timestamp,
            isolation_level: crate::transaction::IsolationLevel::Serializable,
            timeout_ms: 0,
        };

        for op in &tx.operations {
            self.apply_operation(op, &dummy_tx).await?;
        }
        Ok(())
    }

    async fn apply_operation(
        &self,
        op: &Operation,
        dummy_tx: &crate::transaction::Transaction,
    ) -> Result<()> {
        let storage_type = parse_storage_type(&op.storage_type)?;

        let engine = self.engines.get(&storage_type).ok_or_else(|| {
            crate::Error::StorageEngineNotFound(storage_type)
        })?;

        match op.op_type {
            OperationType::Create => {
                let schema: crate::storage::Schema = serde_json::from_value(op.data.clone())
                    .map_err(|e| crate::Error::ValidationError(format!("Invalid schema: {}", e)))?;
                engine.create_table(&op.table, &schema).await?;
            }
            OperationType::Insert => {
                engine.insert(&op.table, &op.data, dummy_tx).await?;
            }
            OperationType::Update => {
                engine
                    .update(&op.table, op.conditions.as_ref(), &op.data, dummy_tx)
                    .await?;
            }
            OperationType::Delete => {
                engine
                    .delete(&op.table, op.conditions.as_ref(), dummy_tx)
                    .await?;
            }
            OperationType::Drop => {
                engine.drop_table(&op.table).await?;
            }
        }
        Ok(())
    }
}
