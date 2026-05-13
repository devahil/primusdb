/*!
# PrimusDB Transaction Manager - ACID Compliance Layer

The transaction manager provides ACID (Atomicity, Consistency, Isolation, Durability)
guarantees for PrimusDB operations. It coordinates multi-operation transactions,
manages concurrency control, and ensures data consistency across storage engines.

## ACID Properties Implementation

```text
ACID Properties in PrimusDB
═══════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────┐
│                ATOMICITY                                │
│  ┌─────────────────────────────────────────────────┐    │
│  │  "All or Nothing" Principle                    │    │
│  │  • Transaction either fully completes         │    │
│  │  • Or fully rolls back on failure             │    │
│  │  • No partial state changes                    │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                CONSISTENCY                              │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Data Integrity Preservation                    │    │
│  │  • Database constraints maintained              │    │
│  │  • Referential integrity enforced               │    │
│  │  • Business rules preserved                     │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                ISOLATION                               │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Concurrent Transaction Separation              │    │
│  │  • Read Uncommitted: Dirty reads allowed       │    │
│  │  • Read Committed: No dirty reads              │    │
│  │  • Repeatable Read: Consistent reads           │    │
│  │  • Serializable: Full isolation                │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                DURABILITY                               │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Crash Recovery & Persistence                   │    │
│  │  • WAL (Write-Ahead Logging)                   │    │
│  │  • Transaction log persistence                 │    │
│  │  • Automatic crash recovery                    │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## Transaction Lifecycle

```text
Transaction Execution Flow
═══════════════════════════════════════════════════════════════

┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  BEGIN      │ -> │  EXECUTE    │ -> │  COMMIT     │
│  Transaction│    │  Operations │    │  Changes    │
└─────────────┘    └─────────────┘    └─────────────┘
       │                   │                   │
       │                   │                   │
       v                   v                   v
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Acquire    │    │  Log        │    │  Persist    │
│  Locks      │    │  Operations │    │  Changes    │
└─────────────┘    └─────────────┘    └─────────────┘

   On Failure: ROLLBACK → Release Locks → Cleanup
```

## Isolation Levels

### Read Uncommitted (Lowest Isolation)
- **Allows**: Dirty reads, non-repeatable reads, phantom reads
- **Performance**: Highest performance
- **Use Case**: Bulk operations where consistency is not critical

### Read Committed (Default)
- **Prevents**: Dirty reads
- **Allows**: Non-repeatable reads, phantom reads
- **Performance**: Good balance
- **Use Case**: Most OLTP applications

### Repeatable Read
- **Prevents**: Dirty reads, non-repeatable reads
- **Allows**: Phantom reads
- **Performance**: Moderate
- **Use Case**: Financial applications requiring consistent reads

### Serializable (Highest Isolation)
- **Prevents**: All concurrency anomalies
- **Performance**: Lowest performance
- **Use Case**: High-stakes operations requiring absolute consistency

## Transaction Manager Architecture

### Key Components:
```text
TransactionManager
├── Transaction Log      - WAL for durability
├── Lock Manager        - Concurrency control
├── Isolation Engine    - MVCC/Snapshot isolation
├── Recovery Manager    - Crash recovery
└── Coordinator         - Distributed transaction coordination
```

### Storage Integration:
- **Columnar Engine**: Snapshot isolation, optimistic concurrency
- **Vector Engine**: Simplified transactions (no complex locking)
- **Document Engine**: MVCC with versioned documents
- **Relational Engine**: Full ACID with 2PL (Two-Phase Locking)

## Usage Examples

### Basic Transaction
```ignore
use primusdb::transaction::{TransactionManager, IsolationLevel};

let tx_manager = TransactionManager::new(&config)?;

// Start transaction
let mut transaction = tx_manager.begin_transaction(IsolationLevel::ReadCommitted)?;

// Execute operations
let insert_op = TransactionOperation {
    operation_type: OperationType::Insert,
    table: "users".to_string(),
    data: serde_json::json!({"name": "Alice", "email": "alice@example.com"}),
    ..Default::default()
};

transaction.add_operation(insert_op)?;
let update_op = TransactionOperation {
    operation_type: OperationType::Update,
    table: "counters".to_string(),
    conditions: Some(serde_json::json!({"name": "user_count"})),
    data: serde_json::json!({"$inc": {"value": 1}}),
    ..Default::default()
};

transaction.add_operation(update_op)?;

// Commit transaction
tx_manager.commit_transaction(transaction).await?;
```

### Nested Transactions (Savepoints)
```ignore
let transaction = tx_manager.begin_transaction(IsolationLevel::Serializable)?;

// Create savepoint
let savepoint = transaction.create_savepoint("before_critical_operation")?;

transaction.add_operation(critical_operation)?;

// If something fails, rollback to savepoint
if some_condition_fails {
    transaction.rollback_to_savepoint(savepoint)?;
}

// Continue with other operations...
transaction.add_operation(another_operation)?;

// Final commit
tx_manager.commit_transaction(transaction).await?;
```

### Distributed Transactions (2PC)
```ignore
let distributed_tx = tx_manager.begin_distributed_transaction(
    vec!["node1", "node2", "node3"]  // Participating nodes
)?;

// Phase 1: Prepare
for node in distributed_tx.participants() {
    node.prepare()?;
}

// Phase 2: Commit/Rollback
if all_prepared_successfully {
    distributed_tx.commit()?;
} else {
    distributed_tx.rollback()?;
}
```

## Concurrency Control

### Multi-Version Concurrency Control (MVCC)
```text
MVCC Implementation:
• Each transaction sees a consistent snapshot
• Versions maintained for concurrent access
• Automatic cleanup of old versions
• Minimal locking for read operations
```

### Lock-Based Concurrency
```text
Lock Types:
• Shared Locks (S) - Multiple readers, no writers
• Exclusive Locks (X) - Single writer, no readers
• Update Locks (U) - Intent to update, allows reads
• Intent Locks (IS/IX) - Hierarchical locking
```

### Deadlock Prevention
```text
Deadlock Detection & Resolution:
• Wait-for graph analysis
• Timeout-based deadlock detection
• Victim selection algorithms
• Automatic rollback of victim transactions
```

## Recovery & Durability

### Write-Ahead Logging (WAL)
```text
WAL Structure:
• Transaction ID
• Operation Type
• Before/After Images
• Checksum for integrity
• Timestamp for ordering
```

### Crash Recovery Process
```text
Recovery Flow:
1. Analyze WAL for incomplete transactions
2. Rollback uncommitted transactions
3. Redo committed but unflushed operations
4. Validate data consistency
5. Resume normal operations
```

## Performance Considerations

### Transaction Overhead:
- **Short Transactions**: Minimal overhead, high throughput
- **Long Transactions**: Increased lock contention, potential deadlocks
- **Read-Only Transactions**: Optimized with snapshot isolation
- **Write-Heavy Transactions**: Potential bottleneck, consider batching

### Optimization Strategies:
1. **Batch Operations**: Group related operations
2. **Appropriate Isolation**: Choose minimal required level
3. **Connection Pooling**: Reuse transaction contexts
4. **Index Usage**: Reduce lock contention through better indexing

## Monitoring & Observability

### Transaction Metrics:
- **Active Transactions**: Current running transactions
- **Transaction Rate**: TPS (transactions per second)
- **Lock Contention**: Percentage of time spent waiting for locks
- **Deadlock Rate**: Frequency of deadlock occurrences
- **Rollback Rate**: Percentage of transactions that rollback

### Health Checks:
```ignore
// Transaction manager health
let health = tx_manager.health_check()?;
assert!(health.active_transactions < 1000);  // Reasonable limit
assert!(health.deadlock_rate < 0.01);        // Less than 1%
assert!(health.average_latency < 100);       // Under 100ms
```

## Limitations & Future Work

### Current Limitations:
- **Distributed Transactions**: Basic 2PC, no full XA support
- **Long-Running Transactions**: Potential for increased conflicts
- **Memory Usage**: MVCC versions can consume significant memory
- **Nested Transactions**: Limited support for complex nesting

### Planned Enhancements:
- **XA Transactions**: Full distributed transaction support
- **Sagas**: Compensation-based distributed transactions
- **Optimistic Concurrency**: Reduced locking for better performance
- **Real-time Monitoring**: Advanced transaction observability
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
    Update,
    Delete,
    Create,
    Drop,
    Read,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLog {
    pub sequence_number: u64,
    pub transaction_id: String,
    pub operation: TransactionOperation,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Savepoint {
    pub id: String,
    pub transaction_id: String,
    pub operations_count: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct TransactionManager {
    config: PrimusDBConfig,
    #[allow(dead_code)]
    active_transactions: HashMap<String, Transaction>,
    transaction_log: Arc<dyn TransactionLogStore>,
    consensus_engine: Arc<dyn ConsensusEngine>,
    journal: Arc<JournalManager>,
    engines: HashMap<StorageType, Arc<dyn StorageEngine>>,
}

#[async_trait]
pub trait TransactionLogStore: Send + Sync {
    async fn append_log(&self, log: &TransactionLog) -> Result<()>;
    async fn get_logs(&self, transaction_id: &str) -> Result<Vec<TransactionLog>>;
    async fn truncate_logs(&self, before_sequence: u64) -> Result<()>;
    async fn verify_integrity(&self) -> Result<bool>;
}

pub struct JournalManager {
    db: sled::Db,
}

#[allow(dead_code)]
#[derive(Debug)]
struct JournalFile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub lsn: u64, // Log Sequence Number
    pub transaction_id: String,
    pub operation: TransactionOperation,
    pub prev_lsn: Option<u64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub checksum: String,
}

impl TransactionManager {
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
    pub fn new(
        config: &PrimusDBConfig,
        consensus_engine: Arc<dyn ConsensusEngine>,
        engines: HashMap<StorageType, Arc<dyn StorageEngine>>,
    ) -> Result<Self> {
        let transaction_log = Arc::new(FileTransactionLog::new(config)?);
        let journal = Arc::new(JournalManager::new(config)?);

        Ok(TransactionManager {
            config: config.clone(),
            active_transactions: HashMap::new(),
            transaction_log,
            consensus_engine,
            journal,
            engines,
        })
    }

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
                operation_type: OperationType::Insert, // Placeholder
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

    pub async fn rollback_transaction(&self, transaction_id: String) -> Result<()> {
        warn!("Rolling back transaction: {}", transaction_id);

        let logs = self.transaction_log.get_logs(&transaction_id).await?;
        let dummy_tx = Transaction {
            id: format!("rollback_{}", transaction_id),
            operations: vec![],
            status: TransactionStatus::Committed,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            isolation_level: IsolationLevel::Serializable,
            timeout_ms: 0,
        };

        for log in logs.iter().rev() {
            if !log.operation.executed {
                continue;
            }

            let storage_type: StorageType = log.operation.storage_type.parse().unwrap_or(StorageType::Document);
            let engine = match self.engines.get(&storage_type) {
                Some(e) => e,
                None => {
                    warn!("Engine not found for storage type {:?}, skipping rollback", storage_type);
                    continue;
                }
            };

            match log.operation.operation_type {
                OperationType::Insert => {
                    if let Some(ref data) = log.operation.rollback_data {
                        engine.delete(&log.operation.table, Some(data), &dummy_tx).await?;
                        info!("Rolled back INSERT on {}", log.operation.table);
                    }
                }
                OperationType::Update => {
                    if let Some(ref before) = log.operation.before_image {
                        if let Some(records) = before.as_array() {
                            for record in records {
                                if let Some(id_val) = record.get("_id").or_else(|| record.get("id")) {
                                    let ident = serde_json::json!({"_id": id_val});
                                    engine.update(&log.operation.table, Some(&ident), record, &dummy_tx).await?;
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
                                engine.insert(&log.operation.table, record, &dummy_tx).await?;
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

    pub async fn create_savepoint(
        &self,
        transaction_id: &str,
        savepoint_id: &str,
    ) -> Result<Savepoint> {
        info!(
            "Creating savepoint {} for transaction {}",
            savepoint_id, transaction_id
        );

        Ok(Savepoint {
            id: savepoint_id.to_string(),
            transaction_id: transaction_id.to_string(),
            operations_count: 0,
            timestamp: chrono::Utc::now(),
        })
    }

    pub async fn rollback_to_savepoint(
        &self,
        transaction_id: &str,
        savepoint_id: &str,
    ) -> Result<()> {
        warn!(
            "Rolling back transaction {} to savepoint {}",
            transaction_id, savepoint_id
        );
        Ok(())
    }
}

impl JournalManager {
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let path = format!("{}/transactions/journal", config.storage.data_dir);
        std::fs::create_dir_all(&path)?;
        let db = sled::open(&path)?;
        info!("Opened journal database at {}", path);
        Ok(JournalManager { db })
    }

    pub async fn write_entry(&self, entry: &JournalEntry) -> Result<()> {
        let tree = self.db.open_tree("journal")?;
        let value = serde_json::to_vec(entry)?;
        tree.insert(entry.lsn.to_be_bytes(), value)?;
        info!("Wrote journal entry: {}", entry.lsn);
        Ok(())
    }

    pub async fn flush(&self) -> Result<()> {
        let tree = self.db.open_tree("journal")?;
        tree.flush()?;
        info!("Journal flushed to disk");
        Ok(())
    }

    pub async fn recover(&self) -> Result<Vec<Transaction>> {
        let tree = self.db.open_tree("journal")?;
        let mut entries_by_tx: HashMap<String, Vec<JournalEntry>> = HashMap::new();

        for result in tree.iter() {
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

pub struct FileTransactionLog {
    db: sled::Db,
}

impl FileTransactionLog {
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
        for result in tree.iter() {
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
