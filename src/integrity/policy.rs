//! Integrity policy resolution.
//!
//! The integrity mode decides how committed mutations are anchored. Modes are
//! resolved from the server configuration (optionally overridden per database)
//! so operators can choose a cost/latency tradeoff per workload.

use serde::{Deserialize, Serialize};

/// Configured integrity mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IntegrityMode {
    /// No cryptographic anchoring or ledger integration. Explicit opt-in only.
    #[default]
    Disabled,
    /// Mutations are signed locally and persisted in the integrity store.
    LocalSigned,
    /// Checkpoint roots are anchored to the ledger in batches.
    LedgerAnchored,
    /// A mutation commits only after the ledger acknowledges it (highest cost).
    LedgerSynchronous,
    /// Local commit first; integrity records are submitted asynchronously.
    LedgerAsynchronous,
    /// Integrity records are validated through cluster consensus.
    ClusterConsensus,
}

impl IntegrityMode {
    /// Parses a mode from its canonical name (case-insensitive).
    pub fn parse(s: &str) -> Option<IntegrityMode> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "disabled" => Some(IntegrityMode::Disabled),
            "local_signed" | "local" => Some(IntegrityMode::LocalSigned),
            "ledger_anchored" => Some(IntegrityMode::LedgerAnchored),
            "ledger_synchronous" | "synchronous" => Some(IntegrityMode::LedgerSynchronous),
            "ledger_asynchronous" | "asynchronous" => Some(IntegrityMode::LedgerAsynchronous),
            "cluster_consensus" => Some(IntegrityMode::ClusterConsensus),
            _ => None,
        }
    }

    /// True when this mode requires a Hyperledger service to satisfy commits.
    pub fn requires_ledger(&self) -> bool {
        matches!(
            self,
            IntegrityMode::LedgerAnchored
                | IntegrityMode::LedgerSynchronous
                | IntegrityMode::LedgerAsynchronous
        )
    }
}

impl std::fmt::Display for IntegrityMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrityMode::Disabled => write!(f, "disabled"),
            IntegrityMode::LocalSigned => write!(f, "local-signed"),
            IntegrityMode::LedgerAnchored => write!(f, "ledger-anchored"),
            IntegrityMode::LedgerSynchronous => write!(f, "ledger-synchronous"),
            IntegrityMode::LedgerAsynchronous => write!(f, "ledger-asynchronous"),
            IntegrityMode::ClusterConsensus => write!(f, "cluster-consensus"),
        }
    }
}

/// Integrity configuration (lives in `PrimusDBConfig.integrity`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityConfig {
    /// Integrity mode applied to every database unless overridden.
    #[serde(default)]
    pub mode: IntegrityMode,
    /// Require a genesis identity for every database.
    #[serde(default = "default_true")]
    pub genesis_required: bool,
    /// Verify the chain on every read-back.
    #[serde(default)]
    pub verify_on_read: bool,
    /// Verify integrity metadata before applying replicated mutations.
    #[serde(default = "default_true")]
    pub verify_on_replication: bool,
    /// Quarantine data that fails validation instead of serving it.
    #[serde(default = "default_true")]
    pub quarantine_invalid_data: bool,
    /// Seconds between automatic checkpoint creation.
    #[serde(default = "default_checkpoint_interval")]
    pub checkpoint_interval_seconds: u64,
    /// Maximum records per anchored batch.
    #[serde(default = "default_batch_size")]
    pub batch_size: u64,
    /// Maximum pending ledger bytes before forcing a flush.
    #[serde(default = "default_max_pending")]
    pub max_pending_bytes: u64,
    /// Retry strategy for failed ledger submissions: retries.
    #[serde(default = "default_retries")]
    pub retry_attempts: u32,
    /// Confirmation timeout for synchronous submissions (ms).
    #[serde(default = "default_timeout")]
    pub confirmation_timeout_ms: u64,
    /// Allow degraded operation when the ledger is unavailable but the policy
    /// requires it. Never enabled by default.
    #[serde(default)]
    pub allow_degraded: bool,
    /// Signing identity name (defaults to the node id).
    #[serde(default)]
    pub signer_id: Option<String>,
    /// Do not persist private key material locally (verification-only).
    #[serde(default)]
    pub external_signer: bool,
}

fn default_true() -> bool {
    true
}
fn default_checkpoint_interval() -> u64 {
    30
}
fn default_batch_size() -> u64 {
    1000
}
fn default_max_pending() -> u64 {
    1048576
}
fn default_retries() -> u32 {
    5
}
fn default_timeout() -> u64 {
    15000
}

impl Default for IntegrityConfig {
    fn default() -> Self {
        IntegrityConfig {
            mode: IntegrityMode::LocalSigned,
            genesis_required: true,
            verify_on_read: false,
            verify_on_replication: true,
            quarantine_invalid_data: true,
            checkpoint_interval_seconds: default_checkpoint_interval(),
            batch_size: default_batch_size(),
            max_pending_bytes: default_max_pending(),
            retry_attempts: default_retries(),
            confirmation_timeout_ms: default_timeout(),
            allow_degraded: false,
            signer_id: None,
            external_signer: false,
        }
    }
}

/// Resolved policy for a specific database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityPolicy {
    pub mode: IntegrityMode,
    pub genesis_required: bool,
    pub verify_on_read: bool,
    pub verify_on_replication: bool,
    pub quarantine_invalid_data: bool,
    pub batch_size: u64,
    pub confirmation_timeout_ms: u64,
    pub allow_degraded: bool,
    pub signer_id: String,
}

impl IntegrityPolicy {
    /// Resolves a policy from server config, applying per-database overrides
    /// when provided.
    pub fn resolve(
        config: &IntegrityConfig,
        node_id: &str,
        override_mode: Option<IntegrityMode>,
    ) -> Self {
        IntegrityPolicy {
            mode: override_mode.unwrap_or(config.mode),
            genesis_required: config.genesis_required,
            verify_on_read: config.verify_on_read,
            verify_on_replication: config.verify_on_replication,
            quarantine_invalid_data: config.quarantine_invalid_data,
            batch_size: config.batch_size.max(1),
            confirmation_timeout_ms: config.confirmation_timeout_ms,
            allow_degraded: config.allow_degraded,
            signer_id: config
                .signer_id
                .clone()
                .unwrap_or_else(|| node_id.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_parse() {
        assert_eq!(
            IntegrityMode::parse("local-signed"),
            Some(IntegrityMode::LocalSigned)
        );
        assert_eq!(
            IntegrityMode::parse("ledger_synchronous"),
            Some(IntegrityMode::LedgerSynchronous)
        );
        assert_eq!(
            IntegrityMode::parse("cluster-consensus"),
            Some(IntegrityMode::ClusterConsensus)
        );
        assert_eq!(IntegrityMode::parse("nope"), None);
    }

    #[test]
    fn test_mode_requires_ledger() {
        assert!(IntegrityMode::LedgerSynchronous.requires_ledger());
        assert!(IntegrityMode::LedgerAsynchronous.requires_ledger());
        assert!(IntegrityMode::LedgerAnchored.requires_ledger());
        assert!(!IntegrityMode::LocalSigned.requires_ledger());
        assert!(!IntegrityMode::Disabled.requires_ledger());
    }

    #[test]
    fn test_policy_resolution_defaults() {
        let cfg = IntegrityConfig::default();
        let policy = IntegrityPolicy::resolve(&cfg, "node-7", None);
        assert_eq!(policy.mode, IntegrityMode::LocalSigned);
        assert_eq!(policy.signer_id, "node-7");
        assert!(policy.genesis_required);
    }

    #[test]
    fn test_policy_resolution_override() {
        let cfg = IntegrityConfig::default();
        let policy = IntegrityPolicy::resolve(&cfg, "node-7", Some(IntegrityMode::Disabled));
        assert_eq!(policy.mode, IntegrityMode::Disabled);
    }
}
