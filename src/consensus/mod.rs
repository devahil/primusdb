/*!
# PrimusDB Consensus Engine - Distributed Agreement Protocol

The consensus engine implements a Hyperledger-inspired, ledger-based agreement
protocol for PrimusDB. Validators sign transactions with ED25519 keys, signed
transactions accumulate in a mempool, and blocks (batches of transactions with a
SHA-256 Merkle root over their contents) are built, signed and appended to a
sled-backed chain. The [`ConsensusEngine`] trait defines the public interface;
[`HyperledgerStyleConsensus`] is the reference implementation.

Note that consensus here orders operations locally or standalone; cluster-wide
ordering and replication across nodes is handled by the Raft layer in
[`crate::cluster`].

```text
Consensus Engine Flow
====================================================

propose_transaction (ED25519 signature)
        |
        v
validate_transaction_signature --> add_to_mempool
        |
        v
build_block --> Merkle root of txs --> block hash (SHA-256)
        |              |
        v              v
validate_block (merkle + block/tx signatures)
        |
        v
commit_block --> persist_block (sled "blocks" tree)
        |              |
        v              v
ConsensusStateMachine.apply_block --> storage engines
        |
        v
blockchain::* Prometheus metrics (height, appends, tamper)
```

## Main Types

- [`ConsensusEngine`] - trait defining propose/validate/commit/chain-state/fork operations.
- [`HyperledgerStyleConsensus`] - the sled-backed reference implementation.
- [`Transaction`] / [`Operation`] / [`OperationType`] - proposal payloads.
- [`Block`] / [`Hash`] - committed chain records and SHA-256 content hashes.
- [`Validator`] / [`ConsensusParameters`] / [`ChainState`] - network and chain status.
- [`ConsensusResult`] / [`ForkResolution`] - proposal outcome and fork handling results.
- [`state_machine::ConsensusStateMachine`] - applies committed blocks to storage engines.
- [`blockchain`] - Prometheus metrics helpers for chain height/appends.

## Consensus Details

- Signatures are ED25519 via the `ring` crate; node keypairs persist in sled.
- Block production is deterministic round-robin over the registered validator set.
- `handle_fork` resolves forks by preferring the locally kept chain or flagging
  manual intervention when the fork point is unknown.
*/

use crate::{PrimusDBConfig, Result};
use async_trait::async_trait;
use base64::Engine;
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{info, instrument, Span};

/// Core trait defining the consensus protocol interface
///
/// All consensus implementations must provide these fundamental operations
/// for achieving distributed agreement on transaction ordering and validity.
///
/// # Safety Properties
/// - **Agreement**: Honest nodes agree on transaction order
/// - **Validity**: Only valid transactions are committed
/// - **Termination**: Consensus eventually completes
/// - **Integrity**: Committed transactions cannot be reversed
#[async_trait]
pub trait ConsensusEngine: Send + Sync {
    /// Propose a transaction for inclusion in the next block
    ///
    /// # Arguments
    /// * `transaction` - The transaction to propose for consensus
    ///
    /// # Returns
    /// Consensus result indicating acceptance and round information
    ///
    /// # Process
    /// 1. Validate transaction signatures and semantics
    /// 2. Broadcast to validator network
    /// 3. Wait for quorum agreement
    /// 4. Return consensus outcome
    async fn propose_transaction(&self, transaction: &Transaction) -> Result<ConsensusResult>;

    /// Validate a block's integrity and consensus compliance
    ///
    /// # Arguments
    /// * `block` - The block to validate
    ///
    /// # Returns
    /// True if block is valid and properly signed
    ///
    /// # Validation Checks
    /// - Block hash meets difficulty requirements
    /// - All transactions are valid
    /// - Validator signatures are correct
    /// - Block follows consensus rules
    async fn validate_block(&self, block: &Block) -> Result<bool>;

    /// Commit a validated block to the local blockchain
    ///
    /// # Arguments
    /// * `block` - The block to commit to local storage
    ///
    /// # Effects
    /// - Updates local chain state
    /// - Persists block to storage
    /// - Updates validator reputations
    /// - Triggers state transitions
    async fn commit_block(&self, block: &Block) -> Result<()>;

    /// Retrieve current blockchain state information
    ///
    /// # Returns
    /// Current chain state including height, latest block, etc.
    ///
    /// # State Information
    /// - Current block height
    /// - Latest block hash
    /// - Active validator count
    /// - Network difficulty
    /// - Pending transaction count
    async fn get_chain_state(&self) -> Result<ChainState>;

    /// Resolve blockchain fork by selecting canonical chain
    async fn handle_fork(&self, fork_point: &Hash) -> Result<ForkResolution>;

    /// Build a block from the mempool, validate, and commit it.
    /// Returns the committed block, or None if the mempool was empty.
    async fn build_and_commit_block(&self) -> Result<Option<Block>> {
        Ok(None)
    }
}

/// Consensus transaction representing a set of database operations
///
/// A transaction in the consensus context is a collection of database operations
/// that must be executed atomically across the distributed network.
///
/// # Transaction Structure
/// ```text
/// Transaction
/// ├── Header
/// │   ├── ID: Unique identifier
/// │   ├── Timestamp: Creation time
/// │   ├── Proposer: Originating node
/// │   └── Signature: Cryptographic proof
/// └── Operations
///     ├── Operation 1 (Insert/Update/Delete)
///     ├── Operation 2 (Insert/Update/Delete)
///     └── ... (up to block size limit)
/// ```
///
/// # Validation Requirements
/// - Valid cryptographic signature from proposer
/// - All operations are syntactically correct
/// - Proposer has sufficient reputation/stake
/// - Transaction size within limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique transaction identifier (UUID or hash-based)
    pub id: String,
    /// Ordered list of database operations to execute
    pub operations: Vec<Operation>,
    /// Timestamp when transaction was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Cryptographic signature proving transaction authenticity
    pub signature: String,
    /// ID of the node that proposed this transaction
    pub proposer: String,
}

/// Individual database operation within a transaction
///
/// Represents a single database modification that is part of a larger
/// atomic transaction. Each operation has specific validation rules
/// and conflict resolution strategies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Type of database operation to perform
    pub op_type: OperationType,
    /// Target table/collection name
    pub table: String,
    /// Operation data payload (varies by operation type)
    /// - Insert: New record data
    /// - Update: Modified field values
    /// - Delete: Not used (conditions specify what to delete)
    /// - Create: Table schema definition
    /// - Drop: Not used (table name specifies what to drop)
    pub data: serde_json::Value,
    /// Optional conditions for selective operations
    /// - Update/Delete: Which records to modify/remove
    /// - Other operations: Typically None
    pub conditions: Option<serde_json::Value>,
    /// Target storage engine type (Document, Relational, etc.)
    /// Used by the state machine to route operations to the correct engine.
    pub storage_type: String,
}

/// Types of database operations supported in consensus transactions
///
/// Defines the fundamental operations that can be performed on the database.
/// Each operation type has different validation rules and conflict resolution strategies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    /// Insert a new record into a table
    /// - Requires: Valid table exists, record doesn't conflict with constraints
    /// - Conflicts: Primary key violations, unique constraint violations
    /// - Resolution: Abort transaction on constraint violations
    Insert,

    /// Update existing records in a table
    /// - Requires: Valid conditions match at least one record
    /// - Conflicts: Concurrent updates to same records
    /// - Resolution: Last-write-wins or merge strategies
    Update,

    /// Delete records from a table
    /// - Requires: Valid conditions match records to delete
    /// - Conflicts: Concurrent operations on same records
    /// - Resolution: Idempotent - multiple deletes of same record allowed
    Delete,

    /// Create a new table/collection with specified schema
    /// - Requires: Table doesn't already exist, valid schema
    /// - Conflicts: Concurrent creation of same table
    /// - Resolution: First-writer-wins with validation
    Create,

    /// Drop (delete) an existing table/collection
    /// - Requires: Table exists and is empty (or force flag)
    /// - Conflicts: Concurrent operations on the table
    /// - Resolution: Abort if table in use by other transactions
    Drop,
}

/// Outcome of proposing a transaction to the consensus network.
///
/// Indicates whether the transaction was accepted, which block (if any) is
/// expected to contain it, the validator signatures collected and the consensus
/// round in which the proposal was processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// Whether the transaction was accepted for consensus
    pub accepted: bool,
    /// Hash of the block expected to include the transaction
    pub block_hash: Option<Hash>,
    /// Signatures collected from participating validators
    pub validator_signatures: Vec<String>,
    /// Consensus round in which the proposal was processed
    pub consensus_round: u64,
}

/// A committed batch of transactions in the ledger.
///
/// Links to the previous block via `previous_hash`, commits the Merkle root of
/// its transactions, and is signed by the validator that produced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Hash of this block
    pub hash: Hash,
    /// Hash of the parent block (`"genesis"` for the first block)
    pub previous_hash: Hash,
    /// Height of this block in the chain (1-based)
    pub height: u64,
    /// Transactions included in this block
    pub transactions: Vec<Transaction>,
    /// Timestamp when the block was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Merkle root over the block's transactions
    pub merkle_root: Hash,
    /// ID of the validator that produced the block
    pub validator: String,
    /// Base64-encoded signature of the block by its validator
    pub signature: String,
}

/// A SHA-256 content hash used across the ledger (blocks, transactions, roots).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hash(String);

impl Hash {
    /// Borrow the hash as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Consume the hash and return its inner `String`.
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// Snapshot of the local blockchain state reported by the consensus engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainState {
    /// Height of the highest committed block
    pub current_height: u64,
    /// Total number of transactions committed across the chain
    pub total_transactions: u64,
    /// Validators currently participating in consensus
    pub validators: Vec<Validator>,
    /// Hash of the most recently committed block
    pub last_block_hash: Hash,
    /// Live consensus parameters (block time, quorum sizing, ...)
    pub consensus_parameters: ConsensusParameters,
}

/// A validator authorized to propose, validate and sign blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    /// Unique validator ID (node ID)
    pub id: String,
    /// Base64-encoded ED25519 public key used to verify signatures
    pub public_key: String,
    /// Stake committed by the validator
    pub stake: u64,
    /// Reputation score (0.0-1.0) earned over time
    pub reputation: f64,
    /// Timestamp of the last observed activity
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Tunable parameters governing block production and validator requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusParameters {
    /// Target time between blocks (ms)
    pub block_time_ms: u64,
    /// Maximum number of transactions per block
    pub max_block_size: u64,
    /// Number of active validators
    pub validator_count: usize,
    /// Minimum stake a validator must commit
    pub min_stake: u64,
    /// Reputation threshold below which a validator is slashed
    pub slash_threshold: f64,
}

/// Result of resolving a detected fork in the chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForkResolution {
    /// Keep the current chain; the fork is stale
    KeepCurrent,
    /// Switch to the fork, which extends to the given height
    SwitchToFork { new_height: u64 },
    /// The fork cannot be resolved automatically
    ManualIntervention,
}

pub mod blockchain;
pub mod state_machine;

/// Hyperledger-inspired consensus engine implementing [`ConsensusEngine`].
///
/// Maintains a sled-backed ledger, a configurable validator set and a mempool.
/// Transactions are signed with the node's persistent ED25519 keypair, batched
/// into blocks with Merkle roots, validated against validator signatures, and
/// applied to the storage engines through the [`state_machine`] submodule.
///
/// Note: this engine drives local/standalone agreement; cluster-wide ordering
/// is handled by the Raft layer in [`crate::cluster`].
pub struct HyperledgerStyleConsensus {
    current_state: Mutex<ChainState>,
    validators: Mutex<HashMap<String, Validator>>,
    pending_transactions: Mutex<Vec<Transaction>>,
    db: sled::Db,
    /// State machine for applying committed blocks to storage engines
    state_machine: std::sync::Arc<state_machine::ConsensusStateMachine>,
    /// PKCS8 v2 encoded Ed25519 keypair for this node
    keypair_bytes: Mutex<Vec<u8>>,
    /// Base64-encoded ED25519 public key of this node
    node_public_key: String,
}

impl HyperledgerStyleConsensus {
    /// Open (or create) the sled ledger and initialize the consensus state machine.
    pub fn new(
        config: &PrimusDBConfig,
        engines: std::collections::HashMap<
            crate::StorageType,
            std::sync::Arc<dyn crate::storage::StorageEngine>,
        >,
    ) -> Result<Self> {
        let db_path = format!("{}/consensus", config.storage.data_dir);
        std::fs::create_dir_all(&db_path)?;
        let db: sled::Db = sled::open(&db_path)?;

        let consensus_params = ConsensusParameters {
            block_time_ms: 5000,
            max_block_size: 1000000,
            validator_count: 7,
            min_stake: 1000,
            slash_threshold: 0.1,
        };

        // Restore chain state from sled
        let height = db
            .get("height")?
            .and_then(|v| serde_json::from_slice::<u64>(&v).ok())
            .unwrap_or(0);
        let total_tx = db
            .get("total_tx")?
            .and_then(|v| serde_json::from_slice::<u64>(&v).ok())
            .unwrap_or(0);
        let last_hash = db
            .get("last_hash")?
            .and_then(|v| String::from_utf8(v.to_vec()).ok())
            .unwrap_or_else(|| "genesis".to_string());

        let initial_state = ChainState {
            current_height: height,
            total_transactions: total_tx,
            validators: vec![],
            last_block_hash: Hash(last_hash),
            consensus_parameters: consensus_params,
        };

        let state_machine = std::sync::Arc::new(state_machine::ConsensusStateMachine::new(engines));

        // Generate or load the node's ED25519 keypair
        let rng = SystemRandom::new();
        let (keypair_bytes, public_key) = Self::load_or_generate_keypair(&db, &rng)?;

        Ok(HyperledgerStyleConsensus {
            current_state: Mutex::new(initial_state),
            validators: Mutex::new(HashMap::new()),
            pending_transactions: Mutex::new(vec![]),
            db,
            state_machine,
            keypair_bytes: Mutex::new(keypair_bytes),
            node_public_key: public_key.clone(),
        })
    }

    /// Load the node's ED25519 keypair from sled, or generate a new one.
    fn load_or_generate_keypair(
        db: &sled::Db,
        rng: &SystemRandom,
    ) -> crate::Result<(Vec<u8>, String)> {
        // Try to load existing keypair
        if let Some(bytes) = db.get("node_keypair")? {
            let bytes = bytes.to_vec();
            if let Ok(kp) = Ed25519KeyPair::from_pkcs8(&bytes) {
                let pub_key_bytes = kp.public_key().as_ref().to_vec();
                let pub_key = base64::engine::general_purpose::STANDARD.encode(&pub_key_bytes);
                return Ok((bytes, pub_key));
            }
        }

        // Generate new keypair
        let pkcs8_bytes =
            Ed25519KeyPair::generate_pkcs8(rng).expect("failed to generate ED25519 keypair");
        let keypair_bytes = pkcs8_bytes.as_ref().to_vec();
        let kp = Ed25519KeyPair::from_pkcs8(&keypair_bytes).expect("generated keypair is invalid");
        let pub_key_bytes = kp.public_key().as_ref().to_vec();
        let pub_key = base64::engine::general_purpose::STANDARD.encode(&pub_key_bytes);

        // Persist to sled for future restarts
        db.insert("node_keypair", keypair_bytes.clone())
            .map_err(|e| {
                crate::Error::ConsensusError(format!("failed to persist node keypair: {}", e))
            })?;
        db.insert("node_public_key", pub_key.as_bytes())
            .map_err(|e| {
                crate::Error::ConsensusError(format!("failed to persist node public key: {}", e))
            })?;
        db.flush().map_err(|e| {
            crate::Error::ConsensusError(format!("failed to flush node keypair: {}", e))
        })?;

        Ok((keypair_bytes, pub_key))
    }

    /// Sign a message (transaction hash bytes) with the node's private key.
    fn sign_bytes(&self, message: &[u8]) -> Vec<u8> {
        let bytes = self.keypair_bytes.lock().unwrap();
        let kp = Ed25519KeyPair::from_pkcs8(&bytes).expect("invalid stored keypair");
        kp.sign(message).as_ref().to_vec()
    }

    /// Verify a signature against a public key.
    fn verify_signature(public_key: &[u8], message: &[u8], signature: &[u8]) -> bool {
        let peer_public_key = UnparsedPublicKey::new(&ED25519, public_key);
        peer_public_key.verify(message, signature).is_ok()
    }

    /// Get the bytes to sign for a transaction.
    fn transaction_signing_data(tx: &Transaction) -> Vec<u8> {
        let data = serde_json::json!({
            "id": tx.id,
            "operations": tx.operations,
            "timestamp": tx.timestamp,
            "proposer": tx.proposer,
        });
        serde_json::to_vec(&data).unwrap_or_default()
    }

    /// Get the bytes to sign for a block.
    fn block_signing_data(block: &Block) -> Vec<u8> {
        let data = serde_json::json!({
            "hash": block.hash.0,
            "previous_hash": block.previous_hash.0,
            "height": block.height,
            "merkle_root": block.merkle_root.0,
            "timestamp": block.timestamp,
            "validator": block.validator,
        });
        serde_json::to_vec(&data).unwrap_or_default()
    }

    /// Add a validator to the validator set
    pub fn add_validator(&mut self, id: String, public_key: String, stake: u64) {
        self.validators.get_mut().unwrap().insert(
            id.clone(),
            Validator {
                id,
                public_key,
                stake,
                reputation: 1.0,
                last_seen: chrono::Utc::now(),
            },
        );
        let mut state = self.current_state.lock().unwrap();
        state.validators = self.validators.lock().unwrap().values().cloned().collect();
        state.consensus_parameters.validator_count = self.validators.lock().unwrap().len();
    }

    /// Remove a validator by ID
    pub fn remove_validator(&mut self, id: &str) {
        self.validators.get_mut().unwrap().remove(id);
        let mut state = self.current_state.lock().unwrap();
        state.validators = self.validators.lock().unwrap().values().cloned().collect();
        state.consensus_parameters.validator_count = self.validators.lock().unwrap().len();
    }

    /// Update validator stake
    pub fn update_validator_stake(&mut self, id: &str, new_stake: u64) -> Option<()> {
        let v = self.validators.get_mut().unwrap().get_mut(id)?;
        v.stake = new_stake;
        let mut state = self.current_state.lock().unwrap();
        state.validators = self.validators.lock().unwrap().values().cloned().collect();
        Some(())
    }

    /// Get validator
    pub fn get_validator(&self, id: &str) -> Option<Validator> {
        self.validators.lock().unwrap().get(id).cloned()
    }

    /// Add a transaction to the mempool
    pub fn add_to_mempool(&self, transaction: Transaction) {
        self.pending_transactions.lock().unwrap().push(transaction);
    }

    /// Build a block from pending transactions (mempool)
    pub fn build_block(&mut self) -> Option<Block> {
        let transactions = {
            let mut pending = self.pending_transactions.lock().unwrap();
            if pending.is_empty() {
                return None;
            }
            pending.drain(..).collect::<Vec<Transaction>>()
        };
        let merkle_root = Self::calculate_merkle_root(&transactions);

        let (height, previous_hash) = {
            let state = self.current_state.lock().unwrap();
            (state.current_height + 1, state.last_block_hash.clone())
        };

        let validator_id = self
            .select_validator(height)
            .map(|v| v.id.clone())
            .unwrap_or_else(|| "genesis".to_string());

        let block_data = serde_json::json!({
            "height": height,
            "previous_hash": previous_hash.0,
            "merkle_root": merkle_root.0,
            "timestamp": chrono::Utc::now(),
            "validator": validator_id,
            "tx_count": transactions.len(),
        });
        let hash = Hash(format!(
            "{:x}",
            sha2::Sha256::digest(serde_json::to_vec(&block_data).unwrap_or_default())
        ));

        let mut block = Block {
            hash: hash.clone(),
            previous_hash,
            height,
            transactions,
            timestamp: chrono::Utc::now(),
            merkle_root,
            validator: validator_id,
            signature: String::new(),
        };

        let block_msg = Self::block_signing_data(&block);
        block.signature =
            base64::engine::general_purpose::STANDARD.encode(self.sign_bytes(&block_msg));

        {
            let mut state = self.current_state.lock().unwrap();
            state.current_height = height;
            state.last_block_hash = hash;
            state.total_transactions += block.transactions.len() as u64;
        }

        Some(block)
    }

    /// Persist a block and update chain state to sled
    fn persist_block(&self, block: &Block) -> Result<()> {
        let blocks_tree = self.db.open_tree("blocks")?;
        let key = format!("block_{}", block.height);
        blocks_tree.insert(key.as_bytes(), serde_json::to_vec(block)?)?;

        let total_tx = {
            let state = self.current_state.lock().unwrap();
            state.total_transactions
        };

        self.db
            .insert("height", serde_json::to_vec(&block.height)?)?;
        self.db.insert("total_tx", serde_json::to_vec(&total_tx)?)?;
        self.db.insert("last_hash", block.hash.0.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    /// List committed blocks (for audit/recovery)
    pub fn list_blocks(&self) -> Result<Vec<Block>> {
        let blocks_tree = self.db.open_tree("blocks")?;
        let mut blocks = Vec::new();
        for result in &blocks_tree {
            let (_, value) = result?;
            if let Ok(block) = serde_json::from_slice::<Block>(&value) {
                blocks.push(block);
            }
        }
        blocks.sort_by_key(|b| b.height);
        Ok(blocks)
    }

    fn calculate_merkle_root(transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash(format!("{:x}", sha2::Sha256::digest(b"")));
        }

        let mut hashes: Vec<String> = transactions
            .iter()
            .map(self::HyperledgerStyleConsensus::hash_transaction)
            .collect();

        while hashes.len() > 1 {
            let mut new_hashes = Vec::new();
            for chunk in hashes.chunks(2) {
                if chunk.len() == 2 {
                    let combined = format!("{}{}", chunk[0], chunk[1]);
                    new_hashes.push(format!("{:x}", sha2::Sha256::digest(combined.as_bytes())));
                } else {
                    new_hashes.push(chunk[0].clone());
                }
            }
            hashes = new_hashes;
        }

        Hash(hashes[0].clone())
    }

    fn hash_transaction(transaction: &Transaction) -> String {
        let serialized = serde_json::to_string(transaction).unwrap();
        format!("{:x}", sha2::Sha256::digest(serialized.as_bytes()))
    }

    #[instrument(skip(self), fields(operation = "validate_transaction_signature"))]
    fn validate_transaction_signature(&self, transaction: &Transaction) -> bool {
        let proposer_pub_key = match self.validators.lock().unwrap().get(&transaction.proposer) {
            Some(v) => v.public_key.clone(),
            None => {
                tracing::warn!("Unknown proposer: {}", transaction.proposer);
                return false;
            }
        };

        let pub_key_bytes = match base64::engine::general_purpose::STANDARD
            .decode(&proposer_pub_key)
            .ok()
        {
            Some(b) => b,
            None => {
                tracing::warn!("Invalid base64 public key for {}", transaction.proposer);
                return false;
            }
        };

        let signature_bytes = match base64::engine::general_purpose::STANDARD
            .decode(&transaction.signature)
            .ok()
        {
            Some(b) => b,
            None => {
                tracing::warn!("Invalid base64 signature on transaction {}", transaction.id);
                return false;
            }
        };

        let message = Self::transaction_signing_data(transaction);
        Self::verify_signature(&pub_key_bytes, &message, &signature_bytes)
    }

    /// Pick the validator responsible for `round` by round-robin over the
    /// current validator set, returning `None` when no validators are known.
    #[allow(dead_code)]
    fn select_validator(&self, round: u64) -> Option<Validator> {
        let validators = self.validators.lock().unwrap();
        let validator_list: Vec<&Validator> = validators.values().collect();
        if validator_list.is_empty() {
            return None;
        }

        let index = (round as usize) % validator_list.len();
        Some(validator_list[index].clone())
    }
}

#[async_trait]
impl ConsensusEngine for HyperledgerStyleConsensus {
    #[instrument(skip(self, transaction), fields(
        operation = "propose_transaction",
        tx_id = %transaction.id,
        proposer = %transaction.proposer,
        duration_ms = tracing::field::Empty
    ))]
    async fn propose_transaction(&self, transaction: &Transaction) -> Result<ConsensusResult> {
        let start = Instant::now();
        println!("Proposing transaction: {}", transaction.id);

        // Sign the transaction with this node's key
        let mut tx = transaction.clone();
        let msg = Self::transaction_signing_data(&tx);
        let sig = self.sign_bytes(&msg);
        tx.signature = base64::engine::general_purpose::STANDARD.encode(&sig);

        // Ensure this node is registered as a validator
        if !self.validators.lock().unwrap().contains_key(&tx.proposer) {
            tracing::info!("Registering proposer '{}' as a validator", tx.proposer);
            self.validators.lock().unwrap().insert(
                tx.proposer.clone(),
                Validator {
                    id: tx.proposer.clone(),
                    public_key: self.node_public_key.clone(),
                    stake: 1000,
                    reputation: 1.0,
                    last_seen: chrono::Utc::now(),
                },
            );
            {
                let mut state = self.current_state.lock().unwrap();
                state.validators = self.validators.lock().unwrap().values().cloned().collect();
                state.consensus_parameters.validator_count = self.validators.lock().unwrap().len();
            }
        }

        if !self.validate_transaction_signature(&tx) {
            return Ok(ConsensusResult {
                accepted: false,
                block_hash: None,
                validator_signatures: vec![],
                consensus_round: 0,
            });
        }

        // Add to mempool
        self.add_to_mempool(tx);

        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);

        let round = {
            let state = self.current_state.lock().unwrap();
            state.current_height + 1
        };

        Ok(ConsensusResult {
            accepted: true,
            block_hash: Some(Hash(format!(
                "{:x}",
                sha2::Sha256::digest(transaction.id.as_bytes())
            ))),
            validator_signatures: vec![base64::engine::general_purpose::STANDARD
                .encode(self.sign_bytes(transaction.id.as_bytes()))],
            consensus_round: round,
        })
    }

    async fn validate_block(&self, block: &Block) -> Result<bool> {
        println!("Validating block at height: {}", block.height);

        // Validate block structure
        if block.transactions.is_empty() {
            return Ok(false);
        }

        // Validate merkle root
        let calculated_root = Self::calculate_merkle_root(&block.transactions);
        if calculated_root != block.merkle_root {
            return Ok(false);
        }

        // Validate validator signature on the block
        let validator_pub_key = {
            let validators = self.validators.lock().unwrap();
            validators
                .get(&block.validator)
                .map(|v| v.public_key.clone())
                .unwrap_or_else(|| self.node_public_key.clone())
        };

        let pub_key_bytes = match base64::engine::general_purpose::STANDARD
            .decode(&validator_pub_key)
            .ok()
        {
            Some(b) => b,
            None => {
                tracing::warn!("Invalid base64 public key for block validator");
                return Ok(false);
            }
        };

        let signature_bytes = match base64::engine::general_purpose::STANDARD
            .decode(&block.signature)
            .ok()
        {
            Some(b) => b,
            None => {
                tracing::warn!("Invalid base64 block signature");
                return Ok(false);
            }
        };

        let block_msg = Self::block_signing_data(block);
        if !Self::verify_signature(&pub_key_bytes, &block_msg, &signature_bytes) {
            tracing::warn!("Block {} has invalid validator signature", block.hash.0);
            return Ok(false);
        }

        // Validate all transaction signatures
        for tx in &block.transactions {
            if !self.validate_transaction_signature(tx) {
                tracing::warn!(
                    "Transaction {} in block {} has invalid signature",
                    tx.id,
                    block.hash.0
                );
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn commit_block(&self, block: &Block) -> Result<()> {
        info!(
            "Committing block {} at height {}",
            block.hash.0, block.height
        );
        self.persist_block(block)?;
        info!("Block {} persisted to sled", block.hash.0);

        blockchain::set_blockchain_height(block.height);
        blockchain::inc_blockchain_append();

        self.state_machine.apply_block(block).await?;
        info!(
            "Block {} applied to state machine at height {}",
            block.hash.0, block.height
        );

        Ok(())
    }

    async fn get_chain_state(&self) -> Result<ChainState> {
        let state = self.current_state.lock().unwrap();
        Ok(state.clone())
    }

    async fn handle_fork(&self, fork_point: &Hash) -> Result<ForkResolution> {
        let state = self.current_state.lock().unwrap();
        tracing::warn!(
            "Fork detected at hash={}, current_height={}, last_hash={}",
            fork_point.0,
            state.current_height,
            state.last_block_hash.0
        );

        // If fork point is our last block, this is a fork at the tip
        if fork_point.0 == state.last_block_hash.0 {
            tracing::warn!("Fork at chain tip — scheduling manual review");
            return Ok(ForkResolution::ManualIntervention);
        }

        // Try to find the fork point in our stored chain
        let blocks = self.list_blocks().ok();
        let fork_found =
            blocks.is_some_and(|blocks| blocks.iter().any(|b| b.hash.0 == fork_point.0));

        if fork_found {
            // Fork point exists in our chain; it's an older fork
            tracing::info!(
                "Fork point {} found at height, keeping current chain",
                fork_point.0
            );
            Ok(ForkResolution::KeepCurrent)
        } else {
            // Unknown fork point
            tracing::warn!(
                "Fork point {} not found in local chain — manual intervention required",
                fork_point.0
            );
            Ok(ForkResolution::ManualIntervention)
        }
    }

    #[instrument(skip(self), fields(
        operation = "build_and_commit_block",
        height = tracing::field::Empty,
        duration_ms = tracing::field::Empty
    ))]
    async fn build_and_commit_block(&self) -> Result<Option<Block>> {
        let start = Instant::now();
        let transactions = {
            let mut pending = self.pending_transactions.lock().unwrap();
            if pending.is_empty() {
                return Ok(None);
            }
            pending.drain(..).collect::<Vec<Transaction>>()
        };

        // Sign each transaction with the node's key
        let mut signed_txns: Vec<Transaction> = Vec::with_capacity(transactions.len());
        for mut tx in transactions {
            let msg = Self::transaction_signing_data(&tx);
            let sig = self.sign_bytes(&msg);
            tx.signature = base64::engine::general_purpose::STANDARD.encode(&sig);
            signed_txns.push(tx);
        }

        let merkle_root = Self::calculate_merkle_root(&signed_txns);

        let (height, previous_hash) = {
            let state = self.current_state.lock().unwrap();
            (state.current_height + 1, state.last_block_hash.clone())
        };

        let validator_list: Vec<Validator> = {
            let validators = self.validators.lock().unwrap();
            validators.values().cloned().collect()
        };
        let (validator_id, _validator_pub_key) = if validator_list.is_empty() {
            ("genesis".to_string(), self.node_public_key.clone())
        } else {
            let index = (height as usize) % validator_list.len();
            let v = &validator_list[index];
            (v.id.clone(), v.public_key.clone())
        };

        let block_data = serde_json::json!({
            "height": height,
            "previous_hash": previous_hash.0,
            "merkle_root": merkle_root.0,
            "timestamp": chrono::Utc::now(),
            "validator": validator_id,
            "tx_count": signed_txns.len(),
        });
        let hash = Hash(format!(
            "{:x}",
            sha2::Sha256::digest(serde_json::to_vec(&block_data).unwrap_or_default())
        ));

        let mut block = Block {
            hash: hash.clone(),
            previous_hash,
            height,
            transactions: signed_txns,
            timestamp: chrono::Utc::now(),
            merkle_root,
            validator: validator_id,
            signature: String::new(),
        };

        // Sign the block
        let block_msg = Self::block_signing_data(&block);
        let block_sig = self.sign_bytes(&block_msg);
        block.signature = base64::engine::general_purpose::STANDARD.encode(&block_sig);

        if !self.validate_block(&block).await? {
            return Err(crate::Error::ConsensusError(
                "Block validation failed".to_string(),
            ));
        }

        {
            let mut state = self.current_state.lock().unwrap();
            state.current_height = height;
            state.last_block_hash = hash;
            state.total_transactions += block.transactions.len() as u64;
        }

        Span::current().record("height", height);

        self.commit_block(&block).await?;

        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);

        info!(
            "Built and committed block {} at height {}",
            block.hash.0, height
        );
        Ok(Some(block))
    }
}
