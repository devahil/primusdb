//! Hyperledger Fabric Gateway REST client.
//!
//! Implements the Fabric Gateway REST API surface used by PrimusDB:
//!
//! * submit a transaction: `POST /gateway/{channel}/{chaincode}/transactions`
//! * evaluate a transaction: `POST /gateway/{channel}/{chaincode}/evaluate`
//! * health probe: `GET /healthz`
//!
//! Submissions carry the record/checkpoint JSON (hashes, proofs, identifiers)
//! as chaincode arguments; responses include the Fabric transaction id used
//! for later confirmation lookups.

use async_trait::async_trait;
use serde_json::json;

use crate::integrity::{
    checkpoint::Checkpoint, record::IntegrityRecord, IntegrityResult, LedgerReceipt,
    LedgerSubmitter,
};

use super::config::HyperledgerConfig;
use super::health::HyperledgerHealth;

/// HTTP client for the Fabric Gateway REST API.
#[derive(Clone)]
pub struct HyperledgerClient {
    config: HyperledgerConfig,
    http: reqwest::Client,
}

impl HyperledgerClient {
    /// Builds a client from configuration. A configured TLS key/cert pair is
    /// applied to the reqwest client; otherwise plain HTTPS/TLS is used.
    pub fn new(config: HyperledgerConfig) -> IntegrityResult<Self> {
        let mut builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(
                config.confirmation_timeout_ms.max(1000),
            ))
            .danger_accept_invalid_certs(false);
        if !config.client_cert_pem.is_empty() && !config.client_key_pem.is_empty() {
            let ident = reqwest::Identity::from_pem(
                format!("{}\n{}", config.client_cert_pem, config.client_key_pem).as_bytes(),
            )
            .map_err(|e| crate::integrity::IntegrityError::Internal(format!("bad PEM: {}", e)))?;
            builder = builder.identity(ident);
        }
        if !config.ca_cert_pem.is_empty() {
            let ca =
                reqwest::Certificate::from_pem(config.ca_cert_pem.as_bytes()).map_err(|e| {
                    crate::integrity::IntegrityError::Internal(format!("bad CA PEM: {}", e))
                })?;
            builder = builder.add_root_certificate(ca);
        }
        let http = builder.build().map_err(|e| {
            crate::integrity::IntegrityError::Internal(format!("client build: {}", e))
        })?;
        Ok(HyperledgerClient { config, http })
    }

    fn submit_url(&self) -> String {
        let base = self.config.gateway_url.trim_end_matches('/');
        format!(
            "{}/gateway/{}/{}/transactions",
            base, self.config.channel, self.config.chaincode
        )
    }

    fn evaluate_url(&self) -> String {
        let base = self.config.gateway_url.trim_end_matches('/');
        format!(
            "{}/gateway/{}/{}/evaluate",
            base, self.config.channel, self.config.chaincode
        )
    }

    fn health_url(&self) -> String {
        format!("{}/healthz", self.config.gateway_url.trim_end_matches('/'))
    }

    /// Probes the gateway for real connectivity and health.
    pub async fn health(&self) -> HyperledgerHealth {
        if !self.config.is_configured() {
            return HyperledgerHealth::unconfigured(&self.config);
        }
        let probe = self.http.get(self.health_url()).send().await;
        match probe {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                let healthy = status.is_success() && body.contains("ok");
                HyperledgerHealth {
                    configured: true,
                    reachable: true,
                    healthy,
                    status_code: Some(status.as_u16()),
                    channel: self.config.channel.clone(),
                    chaincode: self.config.chaincode.clone(),
                    identity: self.config.identity.clone(),
                    message: if healthy {
                        "gateway reachable and healthy".to_string()
                    } else {
                        format!("gateway reachable but health endpoint reported {}", status)
                    },
                    checked_at: chrono::Utc::now(),
                }
            }
            Err(e) => HyperledgerHealth {
                configured: true,
                reachable: false,
                healthy: false,
                status_code: None,
                channel: self.config.channel.clone(),
                chaincode: self.config.chaincode.clone(),
                identity: self.config.identity.clone(),
                message: format!("gateway unreachable: {}", e),
                checked_at: chrono::Utc::now(),
            },
        }
    }

    /// Submits a chaincode function call and returns the Fabric transaction id.
    pub async fn submit_transaction(
        &self,
        _function: &str,
        args: Vec<String>,
    ) -> IntegrityResult<LedgerReceipt> {
        let payload = json!({
            "proposal": {
                "arguments": args,
                "transient": {}
            }
        });
        let resp = self
            .http
            .post(self.submit_url())
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                crate::integrity::IntegrityError::LedgerUnavailable(format!(
                    "gateway request failed: {}",
                    e
                ))
            })?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await.unwrap_or_else(|_| json!({}));
        if status.is_success() {
            let tx_id = body
                .get("transaction_id")
                .or_else(|| body.get("result").and_then(|r| r.get("transaction_id")))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let block = body
                .get("result")
                .and_then(|r| r.get("block"))
                .and_then(|v| v.as_str())
                .map(String::from);
            if tx_id.is_empty() {
                return Err(crate::integrity::IntegrityError::LedgerUnavailable(
                    "gateway accepted submission but returned no transaction id".to_string(),
                ));
            }
            Ok(LedgerReceipt {
                ledger_tx_id: tx_id,
                confirmed: true,
                block,
            })
        } else {
            Err(crate::integrity::IntegrityError::LedgerUnavailable(
                format!(
                    "gateway rejected submission ({}): {}",
                    status,
                    body.get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown")
                ),
            ))
        }
    }

    /// Evaluates a read-only chaincode function.
    pub async fn evaluate_transaction(
        &self,
        _function: &str,
        args: Vec<String>,
    ) -> IntegrityResult<serde_json::Value> {
        let payload = json!({
            "proposal": {
                "arguments": args,
                "transient": {}
            }
        });
        let resp = self
            .http
            .post(self.evaluate_url())
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                crate::integrity::IntegrityError::LedgerUnavailable(format!(
                    "gateway request failed: {}",
                    e
                ))
            })?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await.unwrap_or_else(|_| json!({}));
        if status.is_success() {
            Ok(body)
        } else {
            Err(crate::integrity::IntegrityError::LedgerUnavailable(
                format!(
                    "evaluate failed ({}): {}",
                    status,
                    body.get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown")
                ),
            ))
        }
    }

    fn chaincode_function(&self, base: &str) -> String {
        // Namespace chaincode functions per chaincode to avoid collisions.
        format!("{}_{}", self.config.chaincode, base)
    }
}

#[async_trait]
impl LedgerSubmitter for HyperledgerClient {
    async fn submit_record(&self, record: &IntegrityRecord) -> IntegrityResult<LedgerReceipt> {
        let payload = serde_json::to_string(record)
            .map_err(|e| crate::integrity::IntegrityError::Internal(e.to_string()))?;
        self.submit_transaction(
            &self.chaincode_function("submitIntegrityRecord"),
            vec![payload],
        )
        .await
    }

    async fn submit_checkpoint(&self, cp: &Checkpoint) -> IntegrityResult<LedgerReceipt> {
        let payload = serde_json::to_string(cp)
            .map_err(|e| crate::integrity::IntegrityError::Internal(e.to_string()))?;
        self.submit_transaction(&self.chaincode_function("submitCheckpoint"), vec![payload])
            .await
    }

    async fn health(&self) -> serde_json::Value {
        let h = self.health().await;
        serde_json::to_value(h).unwrap_or_else(|_| json!({"reachable": false}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urls_built_from_config() {
        let cfg = HyperledgerConfig {
            enabled: true,
            gateway_url: "http://localhost:8443".to_string(),
            channel: "ch".to_string(),
            chaincode: "cc".to_string(),
            ..Default::default()
        };
        let client = HyperledgerClient::new(cfg).unwrap();
        assert!(client.submit_url().ends_with("/gateway/ch/cc/transactions"));
        assert!(client.evaluate_url().ends_with("/gateway/ch/cc/evaluate"));
        assert_eq!(client.health_url(), "http://localhost:8443/healthz");
    }
}
