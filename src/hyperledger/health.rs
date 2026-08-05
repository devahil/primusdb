//! Hyperledger connectivity and operational health.
//!
//! Health reflects a **real probe** of the configured gateway, never a static
//! value. `reachable` means an HTTP response was received; `healthy` means the
//! gateway's `/healthz` endpoint reported ok.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::config::HyperledgerConfig;

/// Operational health of the Hyperledger integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperledgerHealth {
    pub configured: bool,
    pub reachable: bool,
    pub healthy: bool,
    pub status_code: Option<u16>,
    pub channel: String,
    pub chaincode: String,
    pub identity: String,
    pub message: String,
    pub checked_at: DateTime<Utc>,
}

impl HyperledgerHealth {
    /// Health state when no gateway is configured.
    pub fn unconfigured(config: &HyperledgerConfig) -> Self {
        HyperledgerHealth {
            configured: false,
            reachable: false,
            healthy: false,
            status_code: None,
            channel: config.channel.clone(),
            chaincode: config.chaincode.clone(),
            identity: config.identity.clone(),
            message: "hyperledger not configured".to_string(),
            checked_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unconfigured_is_honest() {
        let cfg = HyperledgerConfig::default();
        let h = HyperledgerHealth::unconfigured(&cfg);
        assert!(!h.configured);
        assert!(!h.reachable);
        assert!(!h.healthy);
    }
}
