//! REST API layer for PrimusDB.
//! This crate will contain the axum-based HTTP API, route handlers,
//! and middleware.
//!
//! For now, the API module lives in the main `primusdb` crate at `src/api/`.
//! This workspace crate is a placeholder for future extraction.

// Re-export core types from primusdb-core
pub use primusdb_core::*;
