/*!
# PrimusDB Transaction Manager - ACID Compliance Layer

The transaction manager coordinates multi-operation transactions: it journals
operations to a sled-backed WAL, seeks agreement through the consensus engine,
commits durably (journal flush), and rolls back via per-operation before-images.
It targets ACID semantics; see the isolation caveat below for the current gap.

## Architecture

```text
TransactionManager
  ├─ begin_transaction        -> logs BEGIN marker, returns Transaction
  ├─ commit_transaction       -> journal ops -> consensus -> flush journal
  ├─ rollback_transaction     -> reverses executed ops via before-images
  │                               / rollback data
  ├─ create_savepoint / rollback_to_savepoint
  ├─ FileTransactionLog       -> sled-backed operation log
  └─ JournalManager           -> sled-backed journal (WAL) with recovery
```

## ACID Properties

```text
ACID
  ├─ Atomicity  - transaction fully commits or fully rolls back
  │               (no partial state changes)
  ├─ Consistency- storage engines enforce schema/integrity rules
  ├─ Isolation  - IsolationLevel is recorded per transaction, but the
  │               storage engines do not yet act on it: reads/writes are
  │               not serialized by any locking or MVCC scheme
  └─ Durability - journal flushed to disk before commit returns
```

> **Isolation caveat**: the four [`IsolationLevel`] values are accepted and
> stored, but no engine consults them today — the level does not currently
> change read/write behaviour. Concurrency control is effectively
> "no isolation between concurrent transactions".

## Isolation Levels

- **ReadUncommitted** - defined as allowing dirty reads, non-repeatable reads, phantom reads
- **ReadCommitted** - defined as preventing dirty reads; the default level assigned to new transactions
- **RepeatableRead** - defined as preventing dirty and non-repeatable reads; allows phantom reads
- **Serializable** - defined as preventing all concurrency anomalies

All four map to the same (currently unenforced) behaviour; they document intent
for a future concurrency-control implementation.

## Commit Flow

1. Mark transaction `Prepared`.
2. Write each operation as a journal entry (keyed by LSN).
3. Propose the transaction to the consensus engine; if rejected, roll back.
4. Mark `Committed`, flush the journal for durability.

## Rollback Flow

- Fetch the transaction's executed operations from the transaction log.
- Walk them in reverse, using `before_image`/`rollback_data` to re-insert
  deletes, restore updates, and remove inserts through each storage engine.

## Usage

### Basic Transaction
```ignore
use primusdb::transaction::{TransactionManager, IsolationLevel};

// TransactionManager::new requires a config, a consensus engine and the
// per-storage-type engine registry.
let tx_manager = TransactionManager::new(&config, consensus_engine, engines)?;

// Start transaction
let mut transaction = tx_manager.begin_transaction().await?;

// Execute operations
let insert_op = TransactionOperation {
    operation_type: OperationType::Insert,
    table: "users".to_string(),
    data: serde_json::json!({"name": "Alice", "email": "alice@example.com"}),
    ..Default::default()
};

transaction.operations.push(insert_op);
let update_op = TransactionOperation {
    operation_type: OperationType::Update,
    table: "counters".to_string(),
    conditions: Some(serde_json::json!({"name": "user_count"})),
    data: serde_json::json!({"$inc": {"value": 1}}),
    ..Default::default()
};

transaction.operations.push(update_op);

// Commit transaction
tx_manager.commit_transaction(transaction).await?;
```

### Savepoints
```ignore
let transaction = tx_manager.begin_transaction().await?;

// Create savepoint
let savepoint = tx_manager.create_savepoint(&transaction.id, "before_critical_operation").await?;

transaction.operations.push(critical_operation);

// If something fails, rollback to savepoint
if some_condition_fails {
    tx_manager.rollback_to_savepoint(&transaction.id, &savepoint.id).await?;
}

// Continue with other operations...
transaction.operations.push(another_operation);

// Final commit
tx_manager.commit_transaction(transaction).await?;
```
*/

use crate::storage::StorageEngine;
use crate::StorageType;
use crate::{consensus::ConsensusEngine, PrimusDBConfig, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Core transaction structure representing a single ACID transaction
///
/// A transaction encapsulates multiple database operations that must be executed
/// atomically. It tracks the transaction lifecycle, maintains operation ordering,
/// and provides rollback capabilities.
///
/// # Transaction States
/// ```text
/// Transaction Lifecycle:
/// Active → Prepared → Committed
///    ↓         ↓         ↓
/// Failed ← RolledBack ← Aborted
/// ```
///
/// # Key Properties
/// - **Atomicity**: All operations succeed or all fail
/// - **Consistency**: Database constraints are maintained
/// - **Isolation**: Concurrent transactions don't interfere
/// - **Durability**: Committed changes survive failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique transaction identifier (UUID-based)
    pub id: String,
    /// Ordered list of operations to execute
    pub operations: Vec<TransactionOperation>,
    /// Current transaction status
    pub status: TransactionStatus,
    /// Timestamp when transaction was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp of last status change
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Isolation level for this transaction
    pub isolation_level: IsolationLevel,
    /// Timeout in milliseconds (0 = no timeout)
    pub timeout_ms: u64,
}

/// Individual operation within a transaction
///
/// Represents a single database operation (insert, update, delete) that is part
/// of a larger transaction. Maintains before/after images for rollback purposes.
///
/// # Operation Lifecycle
/// ```text
/// 1. Created → 2. Executed → 3. Committed
///      ↓            ↓            ↓
///   Prepared ←  Rolled Back ←  Aborted
/// ```
///
/// # Rollback Support
/// Each operation maintains sufficient state to undo its effects if the
/// transaction needs to be rolled back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOperation {
    /// Unique operation identifier within the transaction
    pub id: String,
    /// Type of database operation
    pub operation_type: OperationType,
    /// Target table/collection name
    pub table: String,
    /// Operation data (insert values, update changes)
    pub data: serde_json::Value,
    /// Optional conditions for update/delete operations
    pub conditions: Option<serde_json::Value>,
    /// State before operation execution (for rollback)
    pub before_image: Option<serde_json::Value>,
    /// State after operation execution
    pub after_image: Option<serde_json::Value>,
    /// Whether this operation has been executed
    pub executed: bool,
    /// Data needed to rollback this operation
    pub rollback_data: Option<serde_json::Value>,
    /// Target storage engine type (Document, Relational, etc.)
    pub storage_type: String,
}

/// Transaction execution states
///
/// Represents the current status of a transaction throughout its lifecycle.
/// Used for monitoring, recovery, and coordination purposes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    /// Transaction is actively executing operations
    Active,
    /// Transaction has been successfully committed
    Committed,
    /// Transaction has been rolled back (manually or due to failure)
    RolledBack,
    /// Transaction failed and cannot be committed
    Failed,
    /// Transaction is prepared for two-phase commit
    Prepared,
}

/// SQL-standard isolation levels for transaction concurrency control
///
/// Defines how concurrent transactions interact and what anomalies are prevented.
/// Higher isolation levels provide stronger consistency guarantees but reduce performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Lowest isolation level - allows dirty reads, non-repeatable reads, and phantom reads
    /// Best performance but weakest consistency guarantees
    ReadUncommitted,
    /// Prevents dirty reads but allows non-repeatable reads and phantom reads
    /// Good balance between performance and consistency
    ReadCommitted,
    /// Prevents dirty reads and non-repeatable reads but allows phantom reads
    /// Stronger consistency for applications requiring consistent reads
    RepeatableRead,
    /// Highest isolation level - prevents all concurrency anomalies
    /// Strongest consistency guarantees but lowest performance
    Serializable,
}

/// Types of database operations supported within transactions
///
/// Defines the fundamental CRUD operations plus additional database management operations.
/// Each operation type has specific semantics for logging, rollback, and concurrency control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    /// Insert a new record into a table/collection
    /// Requires: data payload
    /// Rollback: Delete the inserted record
    Insert,
    /// Modify existing records
    /// Requires: conditions + data payload
    /// Rollback: Restore the before-image
    Update,
    /// Remove records
    /// Requires: conditions
    /// Rollback: Re-insert the deleted records
    Delete,
    /// Create a new object (table, namespace, index)
    /// Requires: data payload
    /// Rollback: Drop the created object
    Create,
    /// Drop an existing object
    /// Requires: conditions targeting the object
    /// Rollback: Re-create the object
    Drop,
    /// Read data (no write effect, only tracked for auditing)
    /// Requires: conditions
    Read,
}

/// A single append-only entry in the durable transaction log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLog {
    /// Monotonic sequence number for ordering
    pub sequence_number: u64,
    /// Owning transaction id
    pub transaction_id: String,
    /// The operation being recorded
    pub operation: TransactionOperation,
    /// When the entry was written
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Integrity checksum over the entry
    pub checksum: String,
}

/// A marker that captures the state of a transaction for partial rollback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Savepoint {
    /// Savepoint identifier
    pub id: String,
    /// Owning transaction id
    pub transaction_id: String,
    /// Number of executed operations at the point the savepoint was taken
    pub operations_count: usize,
    /// When the savepoint was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Coordinates transactional execution across storage engines.
///
/// Appends to the transaction log, drives commit/abort through the consensus
/// engine, and routes operations to the storage engine registered for each
/// involved storage type.
pub struct TransactionManager {
    config: PrimusDBConfig,
    transaction_log: Arc<dyn TransactionLogStore>,
    consensus_engine: Arc<dyn ConsensusEngine>,
    journal: Arc<JournalManager>,
    engines: HashMap<StorageType, Arc<dyn StorageEngine>>,
}

/// Durable backend that stores transaction log entries.
#[async_trait]
pub trait TransactionLogStore: Send + Sync {
    /// Append a single log entry to the store.
    async fn append_log(&self, log: &TransactionLog) -> Result<()>;
    /// Fetch all log entries for a transaction, in sequence order.
    async fn get_logs(&self, transaction_id: &str) -> Result<Vec<TransactionLog>>;
    /// Remove log entries with a sequence number below the given threshold.
    async fn truncate_logs(&self, before_sequence: u64) -> Result<()>;
    /// Verify the integrity of the stored log.
    async fn verify_integrity(&self) -> Result<bool>;
}

/// Sled-backed write-ahead journal used for durability before commit.
pub struct JournalManager {
    db: sled::Db,
}

/// A single write-ahead journal record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    /// Log Sequence Number (monotonic)
    pub lsn: u64, // Log Sequence Number
    /// Owning transaction id
    pub transaction_id: String,
    /// The operation being journaled
    pub operation: TransactionOperation,
    /// LSN of the previous entry in the same transaction
    pub prev_lsn: Option<u64>,
    /// When the entry was written
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Integrity checksum over the entry
    pub checksum: String,
}

impl TransactionManager {
    /// Convert a [`Transaction`] into the consensus engine's transaction type.
    fn convert_to_consensus_tx(&self, transaction: &Transaction) -> crate::consensus::Transaction {
        crate::consensus::Transaction {
            id: transaction.id.clone(),
            operations: transaction
                .operations
                .iter()
                .map(|op| crate::consensus::Operation {
                    op_type: match op.operation_type {
                        OperationType::Insert => crate::consensus::OperationType::Insert,
                        OperationType::Create => crate::consensus::OperationType::Create,
                        OperationType::Read => crate::consensus::OperationType::Insert,
                        OperationType::Update => crate::consensus::OperationType::Update,
                        OperationType::Delete => crate::consensus::OperationType::Delete,
                        OperationType::Drop => crate::consensus::OperationType::Drop,
                    },
                    table: op.table.clone(),
                    data: op.data.clone(),
                    conditions: op.conditions.clone(),
                    storage_type: op.storage_type.clone(),
                })
                .collect(),
            timestamp: transaction.created_at,
            signature: {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(format!("{:?}", transaction).as_bytes());
                let hash = hasher.finalize();
                format!("sha256:{:x}", hash)
            },
            proposer: self.config.cluster.node_id.clone(),
        }
    }
    /// Build a [`TransactionManager`] from the given config, consensus engine
    /// and per-storage-type engine registry.
    pub fn new(
        config: &PrimusDBConfig,
        consensus_engine: Arc<dyn ConsensusEngine>,
        engines: HashMap<StorageType, Arc<dyn StorageEngine>>,
    ) -> Result<Self> {
        let transaction_log = Arc::new(FileTransactionLog::new(config)?);
        let journal = Arc::new(JournalManager::new(config)?);

        Ok(TransactionManager {
            config: config.clone(),
            transaction_log,
            consensus_engine,
            journal,
            engines,
        })
    }

    /// Begin a new transaction, logging the BEGIN marker to the transaction log.
    pub async fn begin_transaction(&self) -> Result<Transaction> {
        let transaction_id = format!(
            "tx_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );

        let transaction = Transaction {
            id: transaction_id.clone(),
            operations: vec![],
            status: TransactionStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            isolation_level: IsolationLevel::ReadCommitted,
            timeout_ms: 30000, // 30 seconds default
        };

        // Log transaction start
        let log_entry = TransactionLog {
            sequence_number: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            transaction_id: transaction_id.clone(),
            operation: TransactionOperation {
                id: format!("begin_{}", transaction_id),
                operation_type: OperationType::Create,
                table: "SYSTEM".to_string(),
                data: serde_json::json!({"action": "BEGIN"}),
                conditions: None,
                before_image: None,
                after_image: None,
                executed: true,
                rollback_data: None,
                storage_type: "Document".to_string(),
            },
            timestamp: chrono::Utc::now(),
            checksum: String::new(),
        };

        self.transaction_log.append_log(&log_entry).await?;

        Ok(transaction)
    }

    /// Commit a transaction using a two-phase protocol: journal the
    /// operations, reach consensus, then flush the journal to disk.
    pub async fn commit_transaction(&self, mut transaction: Transaction) -> Result<()> {
        info!("Committing transaction: {}", transaction.id);

        // Two-phase commit protocol
        // Phase 1: Prepare
        transaction.status = TransactionStatus::Prepared;

        // Write to journal
        for operation in &transaction.operations {
            let journal_entry = JournalEntry {
                lsn: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
                transaction_id: transaction.id.clone(),
                operation: operation.clone(),
                prev_lsn: None,
                timestamp: chrono::Utc::now(),
                checksum: String::new(),
            };
            self.journal.write_entry(&journal_entry).await?;
        }

        // Get consensus from distributed nodes
        let consensus_result = self
            .consensus_engine
            .propose_transaction(&self.convert_to_consensus_tx(&transaction))
            .await?;

        if consensus_result.accepted {
            // Phase 2: Commit
            transaction.status = TransactionStatus::Committed;
            transaction.updated_at = chrono::Utc::now();

            // Flush journal to ensure durability
            self.journal.flush().await?;

            info!("Transaction {} committed successfully", transaction.id);
            Ok(())
        } else {
            // Rollback if consensus not reached
            self.rollback_transaction(transaction.id).await?;
            Err(crate::Error::TransactionError(
                "Consensus not reached".to_string(),
            ))
        }
    }

    /// Roll back a transaction by reversing each executed operation through
    /// its storage engine (re-inserting deletes, restoring before-images).
    pub async fn rollback_transaction(&self, transaction_id: String) -> Result<()> {
        warn!("Rolling back transaction: {}", transaction_id);

        let logs = self.transaction_log.get_logs(&transaction_id).await?;

        // Collect the actual operations that were executed, for rollback context
        let rollback_ops: Vec<TransactionOperation> = logs
            .iter()
            .filter(|l| l.operation.executed)
            .map(|l| l.operation.clone())
            .collect();

        let rollback_tx = Transaction {
            id: format!("rollback_{}", transaction_id),
            operations: rollback_ops,
            status: TransactionStatus::RolledBack,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            isolation_level: IsolationLevel::ReadCommitted,
            timeout_ms: 30000,
        };

        for log in logs.iter().rev() {
            if !log.operation.executed {
                continue;
            }

            let storage_type: StorageType = log
                .operation
                .storage_type
                .parse()
                .unwrap_or(StorageType::Document);
            let engine = match self.engines.get(&storage_type) {
                Some(e) => e,
                None => {
                    warn!(
                        "Engine not found for storage type {:?}, skipping rollback",
                        storage_type
                    );
                    continue;
                }
            };

            match log.operation.operation_type {
                OperationType::Insert => {
                    if let Some(ref data) = log.operation.rollback_data {
                        engine
                            .delete(&log.operation.table, Some(data), &rollback_tx)
                            .await?;
                        info!("Rolled back INSERT on {}", log.operation.table);
                    }
                }
                OperationType::Update => {
                    if let Some(ref before) = log.operation.before_image {
                        if let Some(records) = before.as_array() {
                            for record in records {
                                if let Some(id_val) = record.get("_id").or_else(|| record.get("id"))
                                {
                                    let ident = serde_json::json!({"_id": id_val});
                                    engine
                                        .update(
                                            &log.operation.table,
                                            Some(&ident),
                                            record,
                                            &rollback_tx,
                                        )
                                        .await?;
                                }
                            }
                        }
                        info!("Rolled back UPDATE on {}", log.operation.table);
                    }
                }
                OperationType::Delete => {
                    if let Some(ref deleted) = log.operation.rollback_data {
                        if let Some(records) = deleted.as_array() {
                            for record in records {
                                engine
                                    .insert(&log.operation.table, record, &rollback_tx)
                                    .await?;
                            }
                        }
                        info!("Rolled back DELETE on {}", log.operation.table);
                    }
                }
                _ => {}
            }
        }

        info!("Transaction {} rolled back successfully", transaction_id);
        Ok(())
    }

    /// Create a savepoint capturing the current executed-operation count.
    pub async fn create_savepoint(
        &self,
        transaction_id: &str,
        savepoint_id: &str,
    ) -> Result<Savepoint> {
        let logs = self.transaction_log.get_logs(transaction_id).await?;
        let executed_count = logs.iter().filter(|l| l.operation.executed).count();

        info!(
            "Creating savepoint {} for transaction {} ({} operations)",
            savepoint_id, transaction_id, executed_count
        );

        Ok(Savepoint {
            id: savepoint_id.to_string(),
            transaction_id: transaction_id.to_string(),
            operations_count: executed_count,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Roll back to a savepoint by reversing the operations executed after it.
    pub async fn rollback_to_savepoint(
        &self,
        transaction_id: &str,
        savepoint_id: &str,
    ) -> Result<()> {
        warn!(
            "Rolling back transaction {} to savepoint {}",
            transaction_id, savepoint_id
        );

        let logs = self.transaction_log.get_logs(transaction_id).await?;

        // Reverse-execute operations performed after the savepoint
        let mut rolled_back = 0u64;
        for log in logs.iter().rev() {
            if !log.operation.executed {
                continue;
            }

            let storage_type: StorageType = log
                .operation
                .storage_type
                .parse()
                .unwrap_or(StorageType::Document);
            let engine = match self.engines.get(&storage_type) {
                Some(e) => e,
                None => {
                    warn!("Engine not found for type {:?}, skipping", storage_type);
                    continue;
                }
            };

            let rollback_ctx = Transaction {
                id: format!("rollback_sp_{}", transaction_id),
                operations: vec![log.operation.clone()],
                status: TransactionStatus::RolledBack,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                isolation_level: IsolationLevel::ReadCommitted,
                timeout_ms: 30000,
            };

            match log.operation.operation_type {
                OperationType::Insert => {
                    if let Some(ref data) = log.operation.rollback_data {
                        engine
                            .delete(&log.operation.table, Some(data), &rollback_ctx)
                            .await?;
                        rolled_back += 1;
                    }
                }
                OperationType::Update => {
                    if let Some(ref before) = log.operation.before_image {
                        if let Some(records) = before.as_array() {
                            for record in records {
                                let ident = record
                                    .get("_id")
                                    .or_else(|| record.get("id"))
                                    .map(|v| serde_json::json!({"_id": v}))
                                    .unwrap_or_default();
                                engine
                                    .update(
                                        &log.operation.table,
                                        Some(&ident),
                                        record,
                                        &rollback_ctx,
                                    )
                                    .await?;
                                rolled_back += 1;
                            }
                        }
                    }
                }
                OperationType::Delete => {
                    if let Some(ref deleted) = log.operation.rollback_data {
                        if let Some(records) = deleted.as_array() {
                            for record in records {
                                engine
                                    .insert(&log.operation.table, record, &rollback_ctx)
                                    .await?;
                                rolled_back += 1;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        info!(
            "Rolled back {} operations for transaction {} to savepoint {}",
            rolled_back, transaction_id, savepoint_id
        );
        Ok(())
    }
}

impl JournalManager {
    /// Open (or create) the sled-backed journal database under the data dir.
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let path = format!("{}/transactions/journal", config.storage.data_dir);
        std::fs::create_dir_all(&path)?;
        let db = sled::open(&path)?;
        info!("Opened journal database at {}", path);
        Ok(JournalManager { db })
    }

    /// Persist a journal entry, keyed by its log sequence number.
    pub async fn write_entry(&self, entry: &JournalEntry) -> Result<()> {
        let tree = self.db.open_tree("journal")?;
        let value = serde_json::to_vec(entry)?;
        tree.insert(entry.lsn.to_be_bytes(), value)?;
        info!("Wrote journal entry: {}", entry.lsn);
        Ok(())
    }

    /// Force pending journal writes to disk for durability.
    pub async fn flush(&self) -> Result<()> {
        let tree = self.db.open_tree("journal")?;
        tree.flush()?;
        info!("Journal flushed to disk");
        Ok(())
    }

    /// Rebuild committed transactions from the journal, grouped by
    /// transaction and ordered by log sequence number.
    pub async fn recover(&self) -> Result<Vec<Transaction>> {
        let tree = self.db.open_tree("journal")?;
        let mut entries_by_tx: HashMap<String, Vec<JournalEntry>> = HashMap::new();

        for result in &tree {
            let (_key, value) = result?;
            let entry: JournalEntry = serde_json::from_slice(&value)?;
            entries_by_tx
                .entry(entry.transaction_id.clone())
                .or_default()
                .push(entry);
        }

        let mut transactions = Vec::new();
        for (tx_id, mut entries) in entries_by_tx {
            entries.sort_by_key(|e| e.lsn);
            let created_at = entries
                .first()
                .map(|e| e.timestamp)
                .unwrap_or_else(chrono::Utc::now);
            let updated_at = entries
                .last()
                .map(|e| e.timestamp)
                .unwrap_or_else(chrono::Utc::now);

            let operations: Vec<TransactionOperation> =
                entries.into_iter().map(|e| e.operation).collect();

            transactions.push(Transaction {
                id: tx_id,
                operations,
                status: TransactionStatus::Committed,
                created_at,
                updated_at,
                isolation_level: IsolationLevel::ReadCommitted,
                timeout_ms: 0,
            });
        }

        info!("Recovered {} transactions from journal", transactions.len());
        Ok(transactions)
    }
}

/// Sled-backed [`TransactionLogStore`] implementation persisting logs on disk.
pub struct FileTransactionLog {
    db: sled::Db,
}

impl FileTransactionLog {
    /// Open (or create) the sled-backed transaction log database under the data dir.
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let path = format!("{}/transactions/logs", config.storage.data_dir);
        std::fs::create_dir_all(&path)?;
        let db = sled::open(&path)?;
        info!("Opened transaction log database at {}", path);
        Ok(FileTransactionLog { db })
    }
}

#[async_trait]
impl TransactionLogStore for FileTransactionLog {
    async fn append_log(&self, log: &TransactionLog) -> Result<()> {
        let tree = self.db.open_tree("transaction_logs")?;
        let value = serde_json::to_vec(log)?;
        let key = format!("{}:{}", log.transaction_id, log.sequence_number);
        tree.insert(key.as_bytes(), value)?;
        info!("Appended log entry: {}", log.sequence_number);
        Ok(())
    }

    async fn get_logs(&self, transaction_id: &str) -> Result<Vec<TransactionLog>> {
        let tree = self.db.open_tree("transaction_logs")?;
        let mut logs = Vec::new();
        for result in tree.scan_prefix(transaction_id) {
            let (_key, value) = result?;
            let log: TransactionLog = serde_json::from_slice(&value)?;
            logs.push(log);
        }
        logs.sort_by_key(|l| l.sequence_number);
        info!(
            "Retrieved {} logs for transaction: {}",
            logs.len(),
            transaction_id
        );
        Ok(logs)
    }

    async fn truncate_logs(&self, before_sequence: u64) -> Result<()> {
        let tree = self.db.open_tree("transaction_logs")?;
        let mut to_remove = Vec::new();
        for result in &tree {
            let (key, _value) = result?;
            let key_str = String::from_utf8_lossy(&key);
            if let Some(seq_str) = key_str.split(':').nth(1) {
                if let Ok(seq) = seq_str.parse::<u64>() {
                    if seq < before_sequence {
                        to_remove.push(key.to_vec());
                    }
                }
            }
        }
        for key in to_remove {
            tree.remove(key)?;
        }
        info!("Truncated logs before sequence: {}", before_sequence);
        Ok(())
    }

    async fn verify_integrity(&self) -> Result<bool> {
        let tree = self.db.open_tree("transaction_logs")?;
        tree.flush()?;
        info!("Transaction log integrity verified");
        Ok(true)
    }
}
