//! Hyperledger configuration.
//!
//! The client targets the Hyperledger Fabric Gateway REST API exposed by a
//! gateway endpoint. All fields are optional at the config level so the
//! service can report honest "unreachable" health when nothing is configured.

use serde::{Deserialize, Serialize};

/// Configuration for the Hyperledger integrity client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperledgerConfig {
    /// Whether the Hyperledger service is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Gateway base URL, e.g. `https://peer0.org1.example.com:8443`.
    #[serde(default)]
    pub gateway_url: String,
    /// Fabric channel name.
    #[serde(default = "default_channel")]
    pub channel: String,
    /// Chaincode name.
    #[serde(default = "default_chaincode")]
    pub chaincode: String,
    /// Signing identity name (must match a Fabric identity the gateway knows).
    #[serde(default = "default_identity")]
    pub identity: String,
    /// Timeout (ms) for a single submission to confirm.
    #[serde(default = "default_timeout")]
    pub confirmation_timeout_ms: u64,
    /// Client certificate (PEM) for mTLS.
    #[serde(default)]
    pub client_cert_pem: String,
    /// Client private key (PEM) for mTLS.
    #[serde(default)]
    pub client_key_pem: String,
    /// CA certificate (PEM) for TLS verification.
    #[serde(default)]
    pub ca_cert_pem: String,
}

fn default_channel() -> String {
    "primus-integrity".to_string()
}
fn default_chaincode() -> String {
    "primus-integrity".to_string()
}
fn default_identity() -> String {
    "primus-node".to_string()
}
fn default_timeout() -> u64 {
    15000
}

impl Default for HyperledgerConfig {
    fn default() -> Self {
        HyperledgerConfig {
            enabled: false,
            gateway_url: String::new(),
            channel: default_channel(),
            chaincode: default_chaincode(),
            identity: default_identity(),
            confirmation_timeout_ms: default_timeout(),
            client_cert_pem: String::new(),
            client_key_pem: String::new(),
            ca_cert_pem: String::new(),
        }
    }
}

impl HyperledgerConfig {
    /// True when the config points at a real gateway endpoint.
    pub fn is_configured(&self) -> bool {
        self.enabled && !self.gateway_url.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_not_configured() {
        let cfg = HyperledgerConfig::default();
        assert!(!cfg.is_configured());
    }

    #[test]
    fn test_configured_when_endpoint_set() {
        let cfg = HyperledgerConfig {
            enabled: true,
            gateway_url: "http://localhost:8443".to_string(),
            ..Default::default()
        };
        assert!(cfg.is_configured());
    }
}
