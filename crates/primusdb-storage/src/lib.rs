//! Storage engines for PrimusDB hybrid database.
//! This crate will contain columnar, vector, document, and relational storage engines.
//!
//! For now, the storage module lives in the main `primusdb` crate at `src/storage/`.
//! This workspace crate is a placeholder for future extraction.

// Re-export core types from primusdb-core
pub use primusdb_core::*;
