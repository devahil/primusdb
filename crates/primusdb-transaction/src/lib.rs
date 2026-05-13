//! Transaction management for PrimusDB ACID compliance.
//! This crate will contain transaction lifecycle management, WAL logging,
//! and concurrency control.
//!
//! For now, the transaction module lives in the main `primusdb` crate at `src/transaction/`.
//! This workspace crate is a placeholder for future extraction.

// Re-export core types from primusdb-core
pub use primusdb_core::*;
