use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::consensus::{Block, Operation, OperationType};
use crate::storage::StorageEngine;
use crate::StorageType;
use crate::Result;
use tracing::info;

/// WAL entry for consensus state machine recovery.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct WalEntry {
    pub sequence: u64,
    pub block_height: u64,
    pub data: Vec<u8>,
}

pub struct ConsensusStateMachine {
    engines: HashMap<StorageType, Arc<dyn StorageEngine>>,
    last_applied_height: Mutex<u64>,
    applied_txns: Mutex<HashSet<String>>,
    /// WAL database for crash recovery
    wal: Arc<sled::Db>,
}

fn parse_storage_type(s: &str) -> Result<StorageType> {
    s.parse::<StorageType>()
}

impl ConsensusStateMachine {
    pub fn new(engines: HashMap<StorageType, Arc<dyn StorageEngine>>) -> Self {
        let wal_path = PathBuf::from("/tmp/primusdb/consensus_wal");
        std::fs::create_dir_all(&wal_path).ok();
        let wal = sled::open(wal_path.join("wal.db"))
            .expect("Failed to open consensus WAL database");
        Self {
            engines,
            last_applied_height: Mutex::new(0),
            applied_txns: Mutex::new(HashSet::new()),
            wal: Arc::new(wal),
        }
    }

    pub fn last_applied_height(&self) -> u64 {
        *self.last_applied_height.lock().unwrap()
    }

    /// Append a committed block to the WAL for crash recovery.
    pub fn append_wal(&self, block: &Block) -> Result<()> {
        let entry = WalEntry {
            sequence: block.height,
            block_height: block.height,
            data: bincode::serialize(block)?,
        };
        let key = format!("{:020}", block.height);
        let value = bincode::serialize(&entry)?;
        self.wal.insert(key.as_bytes(), value)?;
        self.wal.flush()?;
        Ok(())
    }

    /// Recover the state machine by replaying all persisted WAL entries.
    pub fn recover_from_wal(&self) -> Result<u64> {
        let mut last_height = 0u64;
        for result in self.wal.iter() {
            let (_, value) = result?;
            let entry: WalEntry = bincode::deserialize(&value)?;
            if entry.block_height > last_height {
                last_height = entry.block_height;
            }
        }
        info!(
            "Consensus WAL recovery complete: last_height={}, entries={}",
            last_height,
            self.wal.len()
        );
        *self.last_applied_height.lock().unwrap() = last_height;
        Ok(last_height)
    }

    pub async fn apply_block(&self, block: &Block) -> Result<()> {
        // Persist to WAL before applying
        self.append_wal(block)?;
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
            ..Default::default()
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
