//! Hyperledger-backed integrity anchoring.
//!
//! This is a real integration layer with the Hyperledger Fabric **Gateway
//! REST API**: it submits integrity checkpoints and records as chaincode
//! transactions and probes the gateway for real connectivity. It is never
//! called from storage code — the integrity service drives it through the
//! [`crate::integrity::LedgerSubmitter`] trait.
//!
//! The ledger stores hashes, proofs and identifiers only — never sensitive
//! payloads.

pub mod client;
pub mod config;
pub mod health;
pub mod identity;

pub use client::HyperledgerClient;
pub use config::HyperledgerConfig;
pub use health::HyperledgerHealth;
