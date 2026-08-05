//! Subcommand handlers for the PrimusDB CLI.
//!
//! Each module implements the `handle_*` entry point for one top-level
//! subcommand group. Handlers are invoked by [`crate::cli::run_cli`] after
//! clap parsing and produce an [`crate::cli::output::OutputData`] value that
//! the shared output formatter renders.

pub mod ai;
pub mod auth;
pub mod backup;
pub mod cdc;
pub mod cluster;
pub mod config;
pub mod db;
pub mod discover;
pub mod doctor;
pub mod engine;
pub mod governor;
pub mod graph;
pub mod instance;
pub mod integrity;
pub mod namespace;
pub mod protocol;
pub mod query;
pub mod search;
pub mod server;
pub mod timeseries;
pub mod vector;
