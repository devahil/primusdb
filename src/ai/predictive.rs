//! Predictive modelling re-export shim.
//!
//! The predictive module surface (training, forecasting, anomaly detection and
//! clustering) lives in the parent `ai` module; this file re-exports it so that
//! `primusdb::ai::predictive::*` remains a stable import path.
pub use super::*;
pub use crate::ai::*;