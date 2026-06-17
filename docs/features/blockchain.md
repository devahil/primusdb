# Blockchain Consensus in PrimusDB

Blockchain consensus brings Hyperledger-style distributed agreement to
multi-model database operations.  Every write transaction is ordered,
signed, and committed into an immutable chain of blocks, giving PrimusDB
the same safety and auditability guarantees found in permissioned
blockchain networks.

> **Status.** Production deployments should use PBFT with ≥4 validator
> nodes.  PoW mode is for development and testing only.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Block & Transaction Structure](#block--transaction-structure)
4. [Validator Management](#validator-management)
5. [Merkle Tree Verification](#merkle-tree-verification)
6. [State Machine](#state-machine)
7. [REST API Endpoints](#rest-api-endpoints)
8. [Configuration](#configuration)
9. [Use Cases & Examples](#use-cases--examples)
10. [Monitoring & Performance](#monitoring--performance)
11. [Security](#security)

---

## Overview

The consensus engine is implemented in `src/consensus/` and exposes a
trait-based interface (`ConsensusEngine`) backed by the
`HyperledgerStyleConsensus` struct.

| Property             | Guarantee                                        |
|----------------------|--------------------------------------------------|
| Safety               | Correct nodes never disagree on committed data   |
| Liveness             | System makes progress with a quorum              |
| Fault Tolerance      | Tolerates `f` faulty nodes with `3f+1` total     |
| Finality             | Committed blocks are irreversible                |
| Atomicity            | All operations in a transaction apply atomically  |
| Auditability         | Every write recorded in an immutable ledger       |

**Two algorithms:**

- **PBFT** (primary, production) — leader-based voting with quorum.
- **Simplified PoW** (dev only) — CPU-based mining, no BFT.

### Source Layout

```
src/consensus/
├── mod.rs             # Core trait, types, and HyperledgerStyleConsensus
├── blockchain.rs      # Block storage, chain verification, Prometheus metrics
└── state_machine.rs   # Applies committed blocks to storage engines
```

---

## Architecture

### Consensus Flow

```text
  Client ──▶ Transaction Proposal ──▶ Consensus Voting ──▶ Block Formation
                • Submit tx               • PBFT-style       • Batch txs
                • Leader validates        • Quorum checks    • Merkle tree
                • Broadcast               • BFT              • Commit
```

### PBFT Protocol Phases

PBFT uses five phases per request:

| Phase        | Actor     | Description                              |
|--------------|-----------|------------------------------------------|
| 1. Request   | Client    | Submit signed transaction                |
| 2. Pre-Prepare | Leader | Assign sequence number & broadcast      |
| 3. Prepare   | Validators | Agree on sequence number               |
| 4. Commit    | Validators | Confirm prepared state                 |
| 5. Reply     | Leader    | Return committed result                  |

**Fault tolerance:** `N = 3f + 1` nodes tolerate `f` Byzantine faults.

### Consensus Engine Trait

```rust
#[async_trait]
pub trait ConsensusEngine: Send + Sync {
    async fn propose_transaction(&self, tx: &Transaction) -> Result<ConsensusResult>;
    async fn validate_block(&self, block: &Block) -> Result<bool>;
    async fn commit_block(&self, block: &Block) -> Result<()>;
    async fn get_chain_state(&self) -> Result<ChainState>;
    async fn handle_fork(&self, fork_point: &Hash) -> Result<ForkResolution>;
    async fn build_and_commit_block(&self) -> Result<Option<Block>>;
}
```

### Node Identity

Each node generates or loads an **Ed25519 keypair** (PKCS#8 v2) via
`ring`, stored in Sled under `node_keypair`.  Public keys are
Base64-encoded and used to verify all transaction and block signatures.

---

## Block & Transaction Structure

### Transaction

```rust
pub struct Transaction {
    pub id: String,                    // Unique ID (UUID or hash)
    pub operations: Vec<Operation>,    // Database operations
    pub timestamp: DateTime<Utc>,      // Creation time
    pub signature: String,             // Ed25519 (Base64)
    pub proposer: String,              // Originating node ID
}
```

### Operation

```rust
pub struct Operation {
    pub op_type: OperationType,   // Insert, Update, Delete, Create, Drop
    pub table: String,            // Target table/collection
    pub data: serde_json::Value,  // Payload (varies by type)
    pub conditions: Option<serde_json::Value>, // Filter for Update/Delete
    pub storage_type: String,     // "Document", "Relational", "Columnar", etc.
}
```

| Type     | Data           | Conditions   | Description                    |
|----------|----------------|--------------|--------------------------------|
| `Insert` | New record     | Unused       | Insert record into table       |
| `Update` | Field changes  | Selector     | Modify matching records        |
| `Delete` | Unused         | Selector     | Remove matching records        |
| `Create` | Schema def     | Unused       | Create table with schema       |
| `Drop`   | Unused         | Unused       | Drop existing table            |

### Block

```rust
pub struct Block {
    pub hash: Hash,                    // SHA-256 header hash
    pub previous_hash: Hash,           // Parent block hash
    pub height: u64,                   // Sequential block number
    pub transactions: Vec<Transaction>,
    pub timestamp: DateTime<Utc>,
    pub merkle_root: Hash,             // Merkle tree root
    pub validator: String,             // Producing node ID
    pub signature: String,             // Validator Ed25519 sig (Base64)
}
```

**Block format:**
```
┌────────────────────────────────────────────┐
│ Header: hash, previous_hash, merkle_root,  │
│         timestamp, height                   │
├────────────────────────────────────────────┤
│ Transactions: [Tx{id, ops[], signature}...] │
├────────────────────────────────────────────┤
│ Validator Signature (Ed25519, Base64)       │
└────────────────────────────────────────────┘
```

The signing payload for a block covers:
`hash, previous_hash, height, merkle_root, timestamp, validator`.

---

## Validator Management

### Validator Struct

```rust
pub struct Validator {
    pub id: String,               // Unique node ID
    pub public_key: String,       // Base64 Ed25519 public key
    pub stake: u64,               // Staked resources
    pub reputation: f64,          // Trust score (0.0–1.0)
    pub last_seen: DateTime<Utc>, // Last activity
}
```

### Lifecycle Methods

```rust
// Add / remove / update
consensus.add_validator("node-4".into(), pubkey, 1000);
consensus.remove_validator("node-4");
consensus.update_validator_stake("node-4", 2000);
consensus.get_validator("node-4");
```

**Auto-registration.** When `propose_transaction` is called, the
proposer is automatically registered as a validator if not already
known (`mod.rs:942`).

### Leader Selection

Block proposers are chosen by **round-robin** over the validator set:

```rust
fn select_validator(&self, round: u64) -> Option<Validator> {
    let index = (round as usize) % validator_list.len();
    Some(validator_list[index].clone())
}
```

### Validator Duties

1. **Transaction Validation** — verify signatures & semantics
2. **Block Proposal** — create blocks from mempool
3. **Block Validation** — confirm Merkle roots, signatures, ordering
4. **Network Security** — detect/misreport beacon

---

## Merkle Tree Verification

### Construction

At `mod.rs:843`, `calculate_merkle_root` builds a binary Merkle tree
over all transactions in a block:

```rust
fn calculate_merkle_root(transactions: &[Transaction]) -> Hash {
    let mut hashes: Vec<String> = transactions
        .iter()
        .map(Self::hash_transaction)
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
```

```
         Root (stored in block.merkle_root)
        /    \
      H12    H34
     /  \   /  \
   H1    H2 H3  H4           ← SHA-256(transaction data)
   / \  / \  / \  / \
  T1  T2 T3 T4 T5  T6 T7 T8  ← Raw transaction data
```

### Verification During Validation

`validate_block` recomputes the root from the block's transactions and
rejects if it does not match `block.merkle_root` (`mod.rs:1002`).

### Chain Verification

`verify_chain` (`blockchain.rs:55`) iterates all stored blocks and
recomputes their Merkle roots, detecting tampering or corruption.

---

## State Machine

`ConsensusStateMachine` (`state_machine.rs`) applies committed blocks to
the storage engines deterministically.

```rust
pub struct ConsensusStateMachine {
    engines: HashMap<StorageType, Arc<dyn StorageEngine>>,
    last_applied_height: Mutex<u64>,
    applied_txns: Mutex<HashSet<String>>,  // Idempotency guard
}
```

### Idempotent Application

Transactions are tracked by ID.  During replay, already-applied
transactions are skipped:

```rust
for tx in &block.transactions {
    if self.applied_txns.lock().unwrap().contains(&tx.id) {
        continue;
    }
    self.apply_transaction(tx).await?;
    self.applied_txns.lock().unwrap().insert(tx.id.clone());
}
```

### Operation Routing

Each operation routes to the correct engine based on `storage_type`:

| `storage_type` | Storage Engine Targeted     |
|----------------|-----------------------------|
| `"Document"`   | Document engine             |
| `"Relational"` | Relational engine           |
| `"Columnar"`   | Columnar engine             |
| `"Vector"`     | Vector engine               |
| `"KeyValue"`   | Key-Value engine            |

Operations are converted to internal `TransactionOperation` objects
with `before_image`/`after_image` fields for full audit trails.

---

## REST API Endpoints

Registered under `/api/v1/consensus/` in `src/api/mod.rs:1005`.

### `GET /api/v1/consensus/state`

Returns the full `ChainState` as JSON:

```json
{
  "current_height": 42,
  "total_transactions": 8712,
  "validators": [ { "id": "node-1", "public_key": "...", "stake": 1000, "reputation": 1.0 } ],
  "last_block_hash": "a1b2c3...",
  "consensus_parameters": { "block_time_ms": 5000, "max_block_size": 1000000, "validator_count": 3 }
}
```

### `POST /api/v1/consensus/build-block`

Drains mempool, builds a block, validates, and commits.

```json
// Success
{ "hash": "a1b2c3...", "height": 43, "num_transactions": 5, "validator": "node-1" }
// Empty mempool
{ "message": "No pending transactions in mempool" }
```

### `POST /api/v1/consensus/producer/start`

Starts a background tokio task building blocks at a configurable interval:

```json
// Request:  { "interval_ms": 5000 }
// Response: { "message": "Background producer started with 5000ms interval" }
```

### `GET /api/v1/protocol/health`

Exposes blockchain height in the protocol health response:

```json
{ "status": "healthy", "blockchain_height": 42, "connected_peers": 3 }
```

---

## Configuration

### Consensus Parameters (defaults in `HyperledgerStyleConsensus::new`)

| Parameter          | Default | Description                      |
|--------------------|---------|----------------------------------|
| `block_time_ms`    | 5000    | Target block interval (ms)       |
| `max_block_size`   | 1000000 | Max bytes per block              |
| `validator_count`  | 7       | Expected validator set size      |
| `min_stake`        | 1000    | Minimum validator stake          |
| `slash_threshold`  | 0.1     | Reputation threshold for slashing|

### Storage Path

Consensus state is stored in Sled at `{data_dir}/consensus/`:

| Key               | Value                        |
|-------------------|------------------------------|
| `height`          | Current blockchain height     |
| `total_tx`        | Total transactions committed  |
| `last_hash`       | Hash of latest block          |
| `node_keypair`    | PKCS#8 Ed25519 private key    |
| `node_public_key` | Base64 public key             |
| `block_{N}`       | Serialized block at height N  |

### Planned TOML Configuration

The module rustdoc shows the intended external shape:

```toml
[consensus]
algorithm = "pbft"
validators = ["node1", "node2", "node3", "node4"]
quorum_size = 3
block_time = 1000
max_block_size = 1000
```

---

## Use Cases & Examples

### Basic Transaction Submission

```rust
use primusdb::consensus::{ConsensusEngine, Transaction, OperationType};

let tx = Transaction {
    id: "tx-001".to_string(),
    operations: vec![Operation {
        op_type: OperationType::Insert,
        table: "users".to_string(),
        data: serde_json::json!({"name": "Alice"}),
        conditions: None,
        storage_type: "Document".to_string(),
    }],
    timestamp: chrono::Utc::now(),
    signature: String::new(),
    proposer: "node-1".to_string(),
};

let result = consensus_engine.propose_transaction(&tx).await?;
if result.accepted {
    println!("Tx accepted, round {}", result.consensus_round);
}
```

### Block Validation & Commitment

```rust
let is_valid = consensus_engine.validate_block(&block).await?;
if is_valid {
    consensus_engine.commit_block(&block).await?;
    println!("Block {} committed", block.hash.as_str());
}
```

### REST Examples

```sh
# Start background producer (5s interval)
curl -X POST http://localhost:8080/api/v1/consensus/producer/start \
  -H "Content-Type: application/json" -d '{"interval_ms": 5000}'

# Query chain state
curl http://localhost:8080/api/v1/consensus/state

# Manual block build
curl -X POST http://localhost:8080/api/v1/consensus/build-block
```

### Validator Lifecycle

```rust
consensus.add_validator("node-4".into(), "Base64PubKey".into(), 5000);
consensus.propose_transaction(&tx).await?;  // Auto-registers if needed
consensus.update_validator_stake("node-4", 10000);
consensus.remove_validator("node-4");
```

### Chain Audit

```rust
let chain_ok = consensus.verify_chain(0)?;
assert!(chain_ok, "Blockchain integrity compromised!");

let blocks = consensus.list_blocks()?;
for b in &blocks {
    println!("Block #{}: {} txs, hash={}", b.height, b.transactions.len(), b.hash.as_str());
}
```

### Fork Detection

```rust
match consensus_engine.handle_fork(&fork_point).await? {
    ForkResolution::KeepCurrent => println!("Staying on current chain"),
    ForkResolution::SwitchToFork { new_height } => { /* reorg */ }
    ForkResolution::ManualIntervention => { /* alert operator */ }
}
```

---

## Monitoring & Performance

### Prometheus Metrics (from `blockchain.rs`)

| Metric                                    | Type    | Description                 |
|-------------------------------------------|---------|-----------------------------|
| `primusdb_blockchain_height`              | Gauge   | Current chain height        |
| `primusdb_blockchain_append_total`        | Counter | Successful block appends    |
| `primusdb_blockchain_tamper_detected_total` | Counter | Tamper events detected    |

### Tracing Spans

Every consensus operation is instrumented with `tracing`:

| Operation                | Span Fields                          |
|--------------------------|--------------------------------------|
| `propose_transaction`    | `tx_id`, `proposer`, `duration_ms`   |
| `build_and_commit_block` | `height`, `duration_ms`              |
| `append_block`           | `block_height`, `duration_ms`        |
| `verify_chain`           | `from_height`, `duration_ms`         |
| `get_block_by_*`         | `height`/`hash`, `duration_ms`       |

### Throughput

| Metric            | PBFT (production) | PoW (dev)         |
|-------------------|-------------------|-------------------|
| Throughput        | 1,000–5,000 TPS   | 10–100 TPS        |
| Block time        | 1–10s configurable | Difficulty-based |
| Latency           | 3–5 RTTs          | Single-node       |
| Fault tolerance   | `f` of `3f+1`     | None              |

### Resource Usage

| Resource | Notes                                   |
|----------|-----------------------------------------|
| CPU      | Moderate (signature verification)       |
| Memory   | Block cache + validator state           |
| Network  | Broadcast between validators (PBFT)     |
| Storage  | Full blockchain history (Sled)          |

---

## Security

### Cryptography

| Component          | Algorithm | Library |
|--------------------|-----------|---------|
| Signatures         | Ed25519   | `ring`  |
| Block/tx hashing   | SHA-256   | `sha2`  |
| Key serialisation  | PKCS#8 v2 | `ring`  |
| Key encoding       | Base64    | `base64`|

### Attack Mitigations

| Attack             | Prevention                                   |
|--------------------|----------------------------------------------|
| Sybil              | Permissioned validator set                   |
| Double spending    | Tx deduplication + strict ordering           |
| Long-range         | Chain history verification (`verify_chain`)  |
| Block tampering    | Merkle root verification on every block      |
| Signature forgery  | Ed25519                                     |
| Replay             | Unique tx IDs + idempotent state machine     |

### Fork Resolution (`handle_fork`, `mod.rs:1084`)

| Scenario           | Behaviour                          |
|--------------------|------------------------------------|
| Fork at chain tip  | Manual intervention required       |
| Fork at known ancestor | Keep current chain             |
| Unknown fork point | Manual intervention required       |

---

## Key Source Files

| File                                | Contents                                  |
|-------------------------------------|-------------------------------------------|
| `src/consensus/mod.rs`              | Core trait, types, `HyperledgerStyleConsensus` implementation |
| `src/consensus/blockchain.rs`       | Block storage, chain verification, metrics |
| `src/consensus/state_machine.rs`    | State machine applying blocks to engines   |
| `src/api/mod.rs` (lines 1005–1022)  | REST route registration                   |
| `src/lib.rs` (lines 838–844, 1555)  | `get_chain_state`, `build_and_commit_block`, `start_background_producer` |
