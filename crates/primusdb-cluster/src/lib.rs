//! Cluster management module for PrimusDB distributed deployments.
//! This crate will contain node discovery, load balancing, failover,
//! and cluster coordination implementations.
//!
//! For now, the cluster module lives in the main `primusdb` crate at `src/cluster/`.
//! This workspace crate is a placeholder for future extraction.

// Re-export core types from primusdb-core
pub use primusdb_core::*;
