/*!
# PrimusDB Audit Ledger – Immutable Append-Only Blockchain

Provides a verifiable, immutable audit trail for all consensus transactions.
This is a lightweight, append-only ledger for auditability — not a PoW
cryptocurrency system. It leverages the existing `Block` and `Transaction`
structures from the consensus engine.

## Security Properties
- **Append-only**: Once committed, blocks cannot be modified.
- **Hash-linked**: Each block references the previous block's hash.
- **Merkle-verified**: Transaction integrity is proven via Merkle root.
- **Namespace-aware**: Each block is tagged with a namespace.
- **Tamper-evident**: Any alteration invalidates the entire chain from that point.
*/

use crate::consensus::{Block, Hash, Transaction};
use crate::Result;
use sha2::Digest;
use std::collections::HashMap;
use tracing::info;

/// Immutable audit ledger backed by sled.
pub struct AuditLedger {
    /// sled database for persistent storage
    db: sled::Db,
    /// In-memory cache of blocks by height for fast lookups
    blocks: HashMap<u64, Block>,
    /// Tree for namespace-indexed block references
    ns_tree: sled::Tree,
    /// Tree for transaction-id → block-height mapping
    tx_tree: sled::Tree,
}

impl AuditLedger {
    /// Open or create an audit ledger at the given path.
    pub fn open(path: &str) -> Result<Self> {
        std::fs::create_dir_all(path)?;
        let db: sled::Db = sled::open(format!("{}/audit_ledger", path))?;
        let ns_tree = db.open_tree("namespaces")?;
        let tx_tree = db.open_tree("transactions")?;

        // Restore blocks from sled into memory
        let mut blocks = HashMap::new();
        for result in db.iter() {
            let (key, value) = result?;
            if key.starts_with(b"block_") {
                if let Ok(block) = serde_json::from_slice::<Block>(&value) {
                    blocks.insert(block.height, block);
                }
            }
        }
        info!("AuditLedger opened with {} blocks at {}", blocks.len(), path);
        Ok(Self {
            db,
            blocks,
            ns_tree,
            tx_tree,
        })
    }

    /// Append a block to the ledger. Validates the hash chain before inserting.
    pub fn append_block(&mut self, block: &Block) -> Result<()> {
        // Validate the block before appending
        self.validate_block_chain(block)?;

        let key = format!("block_{:020}", block.height);
        self.db.insert(key.as_bytes(), serde_json::to_vec(block)?)?;

        // Index by namespace — extract from transactions or use "default"
        let ns = self.extract_namespace(block);
        let mut ns_blocks: Vec<u64> = self
            .ns_tree
            .get(ns.as_bytes())?
            .map(|v| bincode::deserialize(&v).ok())
            .flatten()
            .unwrap_or_default();
        ns_blocks.push(block.height);
        self.ns_tree
            .insert(ns.as_bytes(), bincode::serialize(&ns_blocks)?)?;

        // Index transactions by ID
        for tx in &block.transactions {
            self.tx_tree
                .insert(tx.id.as_bytes(), bincode::serialize(&block.height)?)?;
        }

        self.db.flush()?;
        self.blocks.insert(block.height, block.clone());
        Ok(())
    }

    /// Verify the hash chain for a block.
    fn validate_block_chain(&self, block: &Block) -> Result<()> {
        // Genesis block (height 0) is always valid
        if block.height == 0 {
            return Ok(());
        }

        // Check previous block exists
        let prev = self.blocks.get(&(block.height - 1)).ok_or_else(|| {
            crate::Error::ValidationError(format!(
                "Missing previous block at height {}",
                block.height - 1
            ))
        })?;

        // Verify previous_hash link
        if prev.hash != block.previous_hash {
            return Err(crate::Error::ValidationError(format!(
                "Hash chain broken at height {}: prev_hash mismatch",
                block.height
            )));
        }

        // Verify merkle root
        let computed_root = Self::compute_merkle_root(&block.transactions);
        if computed_root != block.merkle_root {
            return Err(crate::Error::ValidationError(format!(
                "Merkle root mismatch at height {}",
                block.height
            )));
        }

        Ok(())
    }

    /// Compute a SHA-256 Merkle root over a list of transactions.
    pub fn compute_merkle_root(transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash("empty".to_string());
        }
        let mut hashes: Vec<String> = transactions
            .iter()
            .map(|tx| {
                let serialized = serde_json::to_string(tx).unwrap_or_default();
                format!("{:x}", sha2::Sha256::digest(serialized.as_bytes()))
            })
            .collect();

        while hashes.len() > 1 {
            let mut next = Vec::new();
            for chunk in hashes.chunks(2) {
                if chunk.len() == 2 {
                    let combined = format!("{}{}", chunk[0], chunk[1]);
                    next.push(format!("{:x}", sha2::Sha256::digest(combined.as_bytes())));
                } else {
                    next.push(chunk[0].clone());
                }
            }
            hashes = next;
        }
        Hash(hashes[0].clone())
    }

    /// Get a block by height.
    pub fn get_block(&self, height: u64) -> Option<&Block> {
        self.blocks.get(&height)
    }

    /// Get the latest block height in the ledger.
    pub fn latest_height(&self) -> u64 {
        self.blocks.keys().max().copied().unwrap_or(0)
    }

    /// Get the latest block.
    pub fn latest_block(&self) -> Option<&Block> {
        let h = self.latest_height();
        self.blocks.get(&h)
    }

    /// Get all blocks for a given namespace.
    pub fn blocks_by_namespace(&self, namespace: &str) -> Result<Vec<Block>> {
        let heights: Vec<u64> = self
            .ns_tree
            .get(namespace.as_bytes())?
            .map(|v| bincode::deserialize(&v).ok())
            .flatten()
            .unwrap_or_default();
        let mut result = Vec::new();
        for h in heights {
            if let Some(block) = self.blocks.get(&h) {
                result.push(block.clone());
            }
        }
        Ok(result)
    }

    /// Find the block height containing a specific transaction.
    pub fn find_transaction(&self, tx_id: &str) -> Result<Option<u64>> {
        Ok(self
            .tx_tree
            .get(tx_id.as_bytes())?
            .and_then(|v| bincode::deserialize(&v).ok()))
    }

    /// Verify the entire chain from genesis to the latest block.
    /// Returns `Ok(())` if the chain is intact, or an error describing the first break.
    pub fn verify_chain(&self) -> Result<()> {
        let mut heights: Vec<u64> = self.blocks.keys().copied().collect();
        heights.sort();
        for h in &heights {
            if let Some(block) = self.blocks.get(h) {
                // Re-verify each block in sequence
                let mut temp_ledger = Self::open_in_memory();
                // Add all previous blocks
                for prev_h in heights.iter().take_while(|ph| **ph < *h) {
                    if let Some(pb) = self.blocks.get(prev_h) {
                        temp_ledger.blocks.insert(*prev_h, pb.clone());
                    }
                }
                temp_ledger.validate_block_chain(block)?;
            }
        }
        Ok(())
    }

    /// Check if a specific block has been tampered with, and report details.
    pub fn detect_tamper(&self, height: u64) -> Result<BlockAuditReport> {
        let block = self
            .blocks
            .get(&height)
            .ok_or_else(|| crate::Error::ValidationError(format!("Block {} not found", height)))?;

        let mut issues = Vec::new();

        // Verify merkle root
        let computed = Self::compute_merkle_root(&block.transactions);
        if computed != block.merkle_root {
            issues.push(TamperIssue::MerkleRootMismatch {
                expected: block.merkle_root.clone(),
                actual: computed,
            });
        }

        // Verify hash chain link with next block
        if let Some(next) = self.blocks.get(&(height + 1)) {
            if next.previous_hash != block.hash {
                issues.push(TamperIssue::HashChainBroken {
                    height: height + 1,
                    expected: block.hash.clone(),
                    actual: next.previous_hash.clone(),
                });
            }
        }

        // Verify hash chain link with previous block
        if height > 0 {
            if let Some(prev) = self.blocks.get(&(height - 1)) {
                if block.previous_hash != prev.hash {
                    issues.push(TamperIssue::HashChainBroken {
                        height,
                        expected: prev.hash.clone(),
                        actual: block.previous_hash.clone(),
                    });
                }
            }
        }

        Ok(BlockAuditReport {
            height,
            block_hash: block.hash.clone(),
            transaction_count: block.transactions.len(),
            tampered: !issues.is_empty(),
            issues,
        })
    }

    /// Returns the total number of blocks.
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    fn extract_namespace(&self, block: &Block) -> String {
        // Try to derive namespace from the first transaction's proposer
        block
            .transactions
            .first()
            .map(|tx| {
                if tx.proposer == "genesis" {
                    "default".to_string()
                } else {
                    tx.proposer.split('_').next().unwrap_or("default").to_string()
                }
            })
            .unwrap_or_else(|| "default".to_string())
    }

    fn open_in_memory() -> Self {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("Failed to open temporary sled");
        let ns_tree = db.open_tree("namespaces").unwrap();
        let tx_tree = db.open_tree("transactions").unwrap();
        Self {
            db,
            blocks: HashMap::new(),
            ns_tree,
            tx_tree,
        }
    }
}

/// Report from a tamper detection audit on a specific block.
#[derive(Debug, Clone)]
pub struct BlockAuditReport {
    pub height: u64,
    pub block_hash: Hash,
    pub transaction_count: usize,
    pub tampered: bool,
    pub issues: Vec<TamperIssue>,
}

/// Specific tamper issue detected during audit.
#[derive(Debug, Clone)]
pub enum TamperIssue {
    MerkleRootMismatch { expected: Hash, actual: Hash },
    HashChainBroken { height: u64, expected: Hash, actual: Hash },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::{Hash, Transaction};

    fn make_test_block(height: u64, prev_hash: Hash, transactions: Vec<Transaction>) -> Block {
        let merkle_root = AuditLedger::compute_merkle_root(&transactions);
        let hash = Hash(format!("{:x}", sha2::Sha256::digest(
            serde_json::to_vec(&(height, prev_hash.as_str(), merkle_root.as_str())).unwrap()
        )));
        Block {
            hash,
            previous_hash: prev_hash,
            height,
            transactions,
            timestamp: chrono::Utc::now(),
            merkle_root,
            validator: "test".to_string(),
            signature: "sig".to_string(),
        }
    }

    fn make_dummy_tx(id: &str) -> Transaction {
        Transaction {
            id: id.to_string(),
            operations: vec![],
            timestamp: chrono::Utc::now(),
            signature: String::new(),
            proposer: "test".to_string(),
            public_key: String::new(),
        }
    }

    #[test]
    fn test_genesis_block() {
        let mut ledger = AuditLedger::open_in_memory();
        let genesis = make_test_block(0, Hash("genesis".to_string()), vec![]);
        assert!(ledger.append_block(&genesis).is_ok());
        assert_eq!(ledger.block_count(), 1);
        assert_eq!(ledger.latest_height(), 0);
    }

    #[test]
    fn test_append_block() {
        let mut ledger = AuditLedger::open_in_memory();
        let genesis = make_test_block(0, Hash("genesis".to_string()), vec![]);
        ledger.append_block(&genesis).unwrap();

        let block1 = make_test_block(
            1,
            genesis.hash.clone(),
            vec![make_dummy_tx("tx1")],
        );
        assert!(ledger.append_block(&block1).is_ok());
        assert_eq!(ledger.block_count(), 2);
        assert_eq!(ledger.latest_height(), 1);
    }

    #[test]
    fn test_validate_chain() {
        let mut ledger = AuditLedger::open_in_memory();
        let genesis = make_test_block(0, Hash("genesis".to_string()), vec![]);
        ledger.append_block(&genesis).unwrap();

        let block1 = make_test_block(1, genesis.hash.clone(), vec![make_dummy_tx("tx1")]);
        ledger.append_block(&block1).unwrap();

        assert!(ledger.verify_chain().is_ok());
    }

    #[test]
    fn test_detect_tamper_merkle() {
        let mut ledger = AuditLedger::open_in_memory();
        let genesis = make_test_block(0, Hash("genesis".to_string()), vec![]);
        ledger.append_block(&genesis).unwrap();

        let tx = make_dummy_tx("tx1");
        let block1 = make_test_block(1, genesis.hash.clone(), vec![tx]);
        ledger.append_block(&block1).unwrap();

        // Tamper with the stored block by adding a transaction directly
        if let Some(stored) = ledger.blocks.get_mut(&1) {
            stored.transactions.push(make_dummy_tx("tx_tamper"));
        }

        let report = ledger.detect_tamper(1).unwrap();
        assert!(report.tampered);
        assert!(matches!(
            report.issues.first(),
            Some(TamperIssue::MerkleRootMismatch { .. })
        ));
    }

    #[test]
    fn test_invalid_chain_rejected() {
        let mut ledger = AuditLedger::open_in_memory();
        let genesis = make_test_block(0, Hash("genesis".to_string()), vec![]);
        ledger.append_block(&genesis).unwrap();

        // Block with wrong previous_hash
        let bad_block = make_test_block(1, Hash("wrong_hash".to_string()), vec![]);
        assert!(ledger.append_block(&bad_block).is_err());
    }

    #[test]
    fn test_find_transaction() {
        let mut ledger = AuditLedger::open_in_memory();
        let genesis = make_test_block(0, Hash("genesis".to_string()), vec![]);
        ledger.append_block(&genesis).unwrap();

        let tx = make_dummy_tx("find_me");
        let block1 = make_test_block(1, genesis.hash.clone(), vec![tx]);
        ledger.append_block(&block1).unwrap();

        let result = ledger.find_transaction("find_me").unwrap();
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_namespace_indexing() {
        let mut ledger = AuditLedger::open_in_memory();
        let genesis = make_test_block(0, Hash("genesis".to_string()), vec![]);
        ledger.append_block(&genesis).unwrap();

        let tx = Transaction {
            id: "ns_tx".to_string(),
            operations: vec![],
            timestamp: chrono::Utc::now(),
            signature: String::new(),
            proposer: "tenant1_node".to_string(),
            public_key: String::new(),
        };
        let block1 = make_test_block(1, genesis.hash.clone(), vec![tx]);
        ledger.append_block(&block1).unwrap();

        let ns_blocks = ledger.blocks_by_namespace("tenant1").unwrap();
        assert_eq!(ns_blocks.len(), 1);
        assert_eq!(ns_blocks[0].height, 1);
    }

    #[test]
    fn test_ledger_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        {
            let mut ledger = AuditLedger::open(path).unwrap();
            let genesis = make_test_block(0, Hash("genesis".to_string()), vec![]);
            ledger.append_block(&genesis).unwrap();
            let block1 = make_test_block(1, genesis.hash.clone(), vec![make_dummy_tx("tx1")]);
            ledger.append_block(&block1).unwrap();
        }

        // Re-open and verify
        let ledger = AuditLedger::open(path).unwrap();
        assert_eq!(ledger.block_count(), 2);
        assert_eq!(ledger.latest_height(), 1);
        assert!(ledger.verify_chain().is_ok());
    }
}
