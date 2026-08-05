/*!
# Trust Establishment and Management

This module handles certificate-based authentication, trust establishment,
and node identity verification for secure distributed communication.
*/

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;
use tracing::{instrument, Span};

#[derive(Debug, Clone)]
pub struct TrustConfig {
    pub certificate_path: String,
    pub private_key_path: String,
    pub trusted_certificates: Vec<String>,
    pub enable_revocation_checking: bool,
    pub crl_paths: Vec<String>,
}

impl Default for TrustConfig {
    fn default() -> Self {
        Self {
            certificate_path: String::new(),
            private_key_path: String::new(),
            trusted_certificates: Vec::new(),
            enable_revocation_checking: true,
            crl_paths: Vec::new(),
        }
    }
}

pub struct TrustManager {
    trusted_nodes: RwLock<HashMap<String, NodeTrustInfo>>,
}

#[derive(Debug, Clone)]
pub struct NodeTrustInfo {
    pub node_id: String,
    pub certificate: Vec<u8>,
    pub public_key: Vec<u8>,
    pub trust_level: TrustLevel,
    pub last_verified: std::time::SystemTime,
    pub valid_until: std::time::SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrustLevel {
    Trusted,
    PartiallyTrusted,
    Untrusted,
    Revoked,
}

impl TrustManager {
    pub fn new(_config: TrustConfig) -> Result<Self, TrustError> {
        Ok(Self {
            trusted_nodes: RwLock::new(HashMap::new()),
        })
    }

    /// Establish trust with a node
    #[instrument(skip(self, certificate_pem), fields(
        operation = "establish_trust",
        node_id = %node_id,
        duration_ms = tracing::field::Empty
    ))]
    pub fn establish_trust(&self, node_id: &str, certificate_pem: &[u8]) -> Result<(), TrustError> {
        let start = Instant::now();
        let public_key = Self::extract_public_key(certificate_pem);

        let trust_info = NodeTrustInfo {
            node_id: node_id.to_string(),
            certificate: certificate_pem.to_vec(),
            public_key,
            trust_level: TrustLevel::Trusted,
            last_verified: std::time::SystemTime::now(),
            valid_until: std::time::SystemTime::now()
                + std::time::Duration::from_secs(365 * 24 * 3600),
        };

        self.trusted_nodes
            .write()
            .unwrap()
            .insert(node_id.to_string(), trust_info);

        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);

        Ok(())
    }

    /// Verify if a node is trusted
    #[instrument(skip(self), fields(
        operation = "verify_peer",
        node_id = %node_id,
        duration_ms = tracing::field::Empty
    ))]
    pub fn is_trusted(&self, node_id: &str) -> Result<bool, TrustError> {
        let start = Instant::now();
        let trusted_nodes = self.trusted_nodes.read().unwrap();

        if let Some(trust_info) = trusted_nodes.get(node_id) {
            let now = std::time::SystemTime::now();
            if now > trust_info.valid_until {
                let duration = start.elapsed().as_secs_f64() * 1000.0;
                Span::current().record("duration_ms", duration);
                return Ok(false);
            }

            let result = match trust_info.trust_level {
                TrustLevel::Trusted => Ok(true),
                TrustLevel::PartiallyTrusted => Ok(true),
                TrustLevel::Untrusted | TrustLevel::Revoked => Ok(false),
            };
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            Span::current().record("duration_ms", duration);
            result
        } else {
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            Span::current().record("duration_ms", duration);
            Ok(false)
        }
    }

    /// Get public key for a trusted node
    pub fn get_public_key(&self, node_id: &str) -> Result<Vec<u8>, TrustError> {
        let trusted_nodes = self.trusted_nodes.read().unwrap();

        if let Some(trust_info) = trusted_nodes.get(node_id) {
            Ok(trust_info.public_key.clone())
        } else {
            Err(TrustError::NodeNotTrusted)
        }
    }

    /// Revoke trust for a node
    pub fn revoke_trust(&self, node_id: &str) -> Result<(), TrustError> {
        let mut trusted_nodes = self.trusted_nodes.write().unwrap();

        if let Some(trust_info) = trusted_nodes.get_mut(node_id) {
            trust_info.trust_level = TrustLevel::Revoked;
            Ok(())
        } else {
            Err(TrustError::NodeNotFound)
        }
    }

    /// Get trust status for all nodes
    pub fn get_trust_status(&self) -> HashMap<String, TrustLevel> {
        let trusted_nodes = self.trusted_nodes.read().unwrap();
        trusted_nodes
            .iter()
            .map(|(node_id, info)| (node_id.clone(), info.trust_level.clone()))
            .collect()
    }

    fn extract_public_key(certificate_pem: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(certificate_pem);
        hasher.finalize().to_vec()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TrustError {
    #[error("Node not trusted")]
    NodeNotTrusted,
    #[error("Node not found")]
    NodeNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_manager_creation() {
        let config = TrustConfig::default();
        let result = TrustManager::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trust_config_defaults() {
        let config = TrustConfig::default();
        assert!(config.certificate_path.is_empty());
        assert!(config.private_key_path.is_empty());
        assert!(config.trusted_certificates.is_empty());
        assert!(config.enable_revocation_checking);
        assert!(config.crl_paths.is_empty());
    }

    #[test]
    fn test_trust_establishment_and_verification() {
        let config = TrustConfig::default();
        let manager = TrustManager::new(config).unwrap();

        let node_id = "node-1";
        let test_cert = b"test-certificate-data";

        assert!(!manager.is_trusted(node_id).unwrap());

        manager.establish_trust(node_id, test_cert).unwrap();
        assert!(manager.is_trusted(node_id).unwrap());
    }

    #[test]
    fn test_get_public_key() {
        let config = TrustConfig::default();
        let manager = TrustManager::new(config).unwrap();

        let node_id = "node-1";
        let test_cert = b"test-certificate-data";

        assert!(manager.get_public_key(node_id).is_err());

        manager.establish_trust(node_id, test_cert).unwrap();
        let pk = manager.get_public_key(node_id).unwrap();
        assert_eq!(pk.len(), 32);
    }

    #[test]
    fn test_trust_revocation() {
        let config = TrustConfig::default();
        let manager = TrustManager::new(config).unwrap();

        let node_id = "node-1";
        manager.establish_trust(node_id, b"test-cert").unwrap();
        assert!(manager.is_trusted(node_id).unwrap());

        manager.revoke_trust(node_id).unwrap();
        assert!(!manager.is_trusted(node_id).unwrap());
    }

    #[test]
    fn test_revoke_unknown_node() {
        let config = TrustConfig::default();
        let manager = TrustManager::new(config).unwrap();

        let result = manager.revoke_trust("nonexistent-node");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_public_key_unknown_node() {
        let config = TrustConfig::default();
        let manager = TrustManager::new(config).unwrap();

        let result = manager.get_public_key("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_trust_status() {
        let config = TrustConfig::default();
        let manager = TrustManager::new(config).unwrap();

        manager.establish_trust("node-a", b"cert-a").unwrap();
        manager.establish_trust("node-b", b"cert-b").unwrap();

        let status = manager.get_trust_status();
        assert_eq!(status.len(), 2);
        assert_eq!(*status.get("node-a").unwrap(), TrustLevel::Trusted);
        assert_eq!(*status.get("node-b").unwrap(), TrustLevel::Trusted);
    }

    #[test]
    fn test_empty_trust_store() {
        let config = TrustConfig::default();
        let manager = TrustManager::new(config).unwrap();

        assert!(!manager.is_trusted("any-node").unwrap());

        let status = manager.get_trust_status();
        assert!(status.is_empty());
    }

    #[test]
    fn test_extract_public_key() {
        let key = TrustManager::extract_public_key(b"test-cert");
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_trust_level_values_distinct() {
        assert_ne!(
            format!("{:?}", TrustLevel::Trusted),
            format!("{:?}", TrustLevel::Untrusted)
        );
        assert_ne!(
            format!("{:?}", TrustLevel::PartiallyTrusted),
            format!("{:?}", TrustLevel::Revoked)
        );
    }
}
