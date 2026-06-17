use lazy_static::lazy_static;
use prometheus::{register_counter, register_gauge, Counter, Gauge};

lazy_static! {
    static ref BLOCKCHAIN_HEIGHT: Gauge =
        register_gauge!("primusdb_blockchain_height", "Current blockchain height").unwrap();
    static ref BLOCKCHAIN_APPEND_TOTAL: Counter = register_counter!(
        "primusdb_blockchain_append_total",
        "Total number of blockchain appends"
    )
    .unwrap();
    static ref BLOCKCHAIN_TAMPER_DETECTED_TOTAL: Counter = register_counter!(
        "primusdb_blockchain_tamper_detected_total",
        "Total number of blockchain tamper events detected"
    )
    .unwrap();
}

pub fn set_blockchain_height(height: u64) {
    BLOCKCHAIN_HEIGHT.set(height as f64);
}

pub fn inc_blockchain_append() {
    BLOCKCHAIN_APPEND_TOTAL.inc();
}

pub fn inc_blockchain_tamper_detected() {
    BLOCKCHAIN_TAMPER_DETECTED_TOTAL.inc();
}

use super::*;
use std::time::Instant;
use tracing::{instrument, Span};

impl HyperledgerStyleConsensus {
    #[instrument(skip(self), fields(
        operation = "append_block",
        block_height = %block.height,
        duration_ms = tracing::field::Empty
    ))]
    pub fn append_block(&self, block: &Block) -> Result<()> {
        let start = Instant::now();
        self.persist_block(block)?;
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);
        tracing::info!("Appended block {} at height {}", block.hash.0, block.height);
        Ok(())
    }

    #[instrument(skip(self), fields(
        operation = "verify_chain",
        from_height = %from_height,
        duration_ms = tracing::field::Empty
    ))]
    pub fn verify_chain(&self, from_height: u64) -> Result<bool> {
        let start = Instant::now();
        let blocks = self.list_blocks()?;
        let mut valid = true;
        for block in &blocks {
            if block.height < from_height {
                continue;
            }
            let calculated_root =
                HyperledgerStyleConsensus::calculate_merkle_root(&block.transactions);
            if calculated_root != block.merkle_root {
                valid = false;
                break;
            }
        }
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);
        tracing::info!("Chain verified: {}", valid);
        Ok(valid)
    }

    #[instrument(skip(self), fields(
        operation = "get_block_by_height",
        height = %height,
        duration_ms = tracing::field::Empty
    ))]
    pub fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        let start = Instant::now();
        let blocks = self.list_blocks()?;
        let result = blocks.into_iter().find(|b| b.height == height);
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);
        tracing::info!("Block by height {}: {}", height, result.is_some());
        Ok(result)
    }

    #[instrument(skip(self), fields(
        operation = "get_block_by_hash",
        duration_ms = tracing::field::Empty
    ))]
    pub fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>> {
        let start = Instant::now();
        let blocks = self.list_blocks()?;
        let result = blocks.into_iter().find(|b| b.hash == *hash);
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);
        tracing::info!("Block by hash {}: {}", hash.as_str(), result.is_some());
        Ok(result)
    }
}
