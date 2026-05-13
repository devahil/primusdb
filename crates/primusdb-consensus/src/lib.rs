//! Consensus engine for PrimusDB distributed agreement protocol.
//! This crate will contain PBFT-style consensus, block validation,
//! and chain state management.
//!
//! For now, the consensus module lives in the main `primusdb` crate at `src/consensus/`.
//! This workspace crate is a placeholder for future extraction.

// Re-export core types from primusdb-core
pub use primusdb_core::*;
