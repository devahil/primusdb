//! Cryptographic operations for PrimusDB security layer.
//! This crate will contain encryption, key management, and hashing implementations.
//!
//! For now, the crypto module lives in the main `primusdb` crate at `src/crypto/`.
//! This workspace crate is a placeholder for future extraction.

// Re-export core types from primusdb-core
pub use primusdb_core::*;
