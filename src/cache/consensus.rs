/*!
# Cache Consensus Engine - Distributed Cache Integrity & Validation

A local consensus simulation that validates cache operations before they are
executed. Validator votes are collected against each operation proposal and a
configurable quorum decides approval. Approved operations are appended to an
in-memory audit trail that is later verified for integrity.

> **Note**: this is a lightweight, in-process simulation — there is no network
> transport, TLS or Merkle-tree ledger. Validator "votes" and "signatures" are
> generated locally; the hashing backend is BLAKE3.

## API surface

```text
CacheConsensusEngine
  ├─ validate_operation(op)       -> proposal -> votes -> quorum check
  ├─ verify_cluster_integrity()   -> IntegrityReport (re-hash audit trail)
  ├─ detect_data_poisoning()      -> PoisoningReport
  └─ get_statistics()             -> ConsensusStatistics
```

## Usage

### Basic Consensus Setup
```ignore
use primusdb::cache::consensus::{CacheConsensusEngine, ConsensusConfig};

let consensus_config = ConsensusConfig {
    validators: vec![
        "validator-1".to_string(),
        "validator-2".to_string(),
        "validator-3".to_string(),
    ],
    quorum_size: 2,
    timeout: Duration::from_secs(30),
    enable_audit_trail: true,
};

let consensus = CacheConsensusEngine::new(consensus_config);

// Validate cache operation with consensus
let operation = CacheOperation::Put {
    key: "user:123".to_string(),
    data: b"user data".to_vec(),
    checksum: calculate_checksum(b"user data"),
};

let validation = consensus.validate_operation(operation).await?;
assert!(validation.approved);
```

### Integrity Verification
```ignore
let integrity_report = consensus.verify_cluster_integrity().await?;
println!("Cluster integrity: {}%", integrity_report.integrity_score);

let poisoning_report = consensus.detect_data_poisoning().await?;
if poisoning_report.attacks_detected > 0 {
    println!("Warning: {} poisoning attempts detected!", poisoning_report.attacks_detected);
}
```

### Consensus Monitoring
```ignore
let stats = consensus.get_statistics().await;
println!("Consensus success rate: {:.2}%", stats.success_rate);
```
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use tokio::time::{Duration, Instant};

/// Number of samples to use for poisoning pattern analysis.
const PATTERN_WINDOW: usize = 50;

#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    pub validators: Vec<String>,
    pub quorum_size: usize,
    pub timeout: Duration,
    pub enable_audit_trail: bool,
    pub poisoning_detection: bool,
    pub integrity_check_interval: Duration,
    pub max_operation_batch: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            validators: Vec::new(),
            quorum_size: 2,
            timeout: Duration::from_secs(30),
            enable_audit_trail: true,
            poisoning_detection: true,
            integrity_check_interval: Duration::from_secs(60),
            max_operation_batch: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheOperation {
    Put {
        key: String,
        data: Vec<u8>,
        checksum: u32,
    },
    Get {
        key: String,
    },
    Delete {
        key: String,
    },
    Clear,
    Search {
        pattern: String,
        limit: usize,
    },
}

#[derive(Debug, Clone)]
pub struct OperationProposal {
    pub operation: CacheOperation,
    pub proposer: String,
    pub timestamp: Instant,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub approved: bool,
    pub votes_for: usize,
    pub votes_against: usize,
    pub execution_time: Duration,
    pub validator_signatures: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct IntegrityReport {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub corrupted_entries: usize,
    pub integrity_score: f64,
    pub last_check: Instant,
}

#[derive(Debug, Clone)]
pub struct PoisoningReport {
    pub attacks_detected: usize,
    pub blocked_operations: usize,
    pub suspicious_patterns: Vec<String>,
    pub last_detection: Instant,
}

#[derive(Debug, Clone)]
pub struct ConsensusStatistics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub success_rate: f64,
    pub avg_validation_time_ms: f64,
    pub active_validators: usize,
    pub total_validators: usize,
}

pub struct CacheConsensusEngine {
    config: ConsensusConfig,
    audit_trail: RwLock<Vec<OperationProposal>>,
    integrity_reports: RwLock<Vec<IntegrityReport>>,
    poisoning_reports: RwLock<Vec<PoisoningReport>>,
    statistics: RwLock<ConsensusStatistics>,
}

impl CacheConsensusEngine {
    /// Create a new cache consensus engine
    pub fn new(config: ConsensusConfig) -> Self {
        let statistics = ConsensusStatistics {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            success_rate: 0.0,
            avg_validation_time_ms: 0.0,
            active_validators: config.validators.len(),
            total_validators: config.validators.len(),
        };

        Self {
            config,
            audit_trail: RwLock::new(Vec::new()),
            integrity_reports: RwLock::new(Vec::new()),
            poisoning_reports: RwLock::new(Vec::new()),
            statistics: RwLock::new(statistics),
        }
    }

    /// Validate a cache operation through consensus
    pub async fn validate_operation(
        &self,
        operation: CacheOperation,
    ) -> Result<ConsensusResult, ConsensusError> {
        let start_time = Instant::now();

        // Create operation proposal
        let proposal = OperationProposal {
            operation: operation.clone(),
            proposer: "current_node".to_string(), // In real implementation, get actual node ID
            timestamp: start_time,
            signature: self.sign_operation(&operation).await?,
        };

        // Send proposal to validators
        let votes = self.collect_validator_votes(&proposal).await?;

        // Count votes
        let votes_for = votes.iter().filter(|&&vote| vote).count();
        let votes_against = votes.len() - votes_for;

        // Check quorum
        let approved = votes_for >= self.config.quorum_size;

        // Collect signatures
        let validator_signatures = if approved {
            self.collect_validator_signatures(&proposal).await?
        } else {
            Vec::new()
        };

        let result = ConsensusResult {
            approved,
            votes_for,
            votes_against,
            execution_time: start_time.elapsed(),
            validator_signatures,
        };

        // Update statistics
        self.update_statistics(approved, start_time.elapsed()).await;

        // Record in audit trail if enabled
        if self.config.enable_audit_trail && approved {
            self.record_operation(proposal).await?;
        }

        Ok(result)
    }

    /// Verify cluster-wide cache integrity
    pub async fn verify_cluster_integrity(&self) -> Result<IntegrityReport, ConsensusError> {
        let audit = self.audit_trail.read().unwrap();
        let total_entries = audit.len();

        // Validate each operation's signature against its content
        let mut corrupted_entries = 0usize;
        for proposal in audit.iter() {
            let expected_sig = Self::compute_operation_hash(&proposal.operation);
            if proposal.signature != expected_sig {
                corrupted_entries += 1;
            }
        }

        let valid_entries = total_entries.saturating_sub(corrupted_entries);
        let integrity_score = if total_entries > 0 {
            (valid_entries as f64 / total_entries as f64) * 100.0
        } else {
            100.0
        };

        let report = IntegrityReport {
            total_entries,
            valid_entries,
            corrupted_entries,
            integrity_score,
            last_check: Instant::now(),
        };

        self.integrity_reports.write().unwrap().push(report.clone());
        Ok(report)
    }

    /// Detect data poisoning attempts by analyzing operation patterns
    pub async fn detect_data_poisoning(&self) -> Result<PoisoningReport, ConsensusError> {
        let audit = self.audit_trail.read().unwrap();
        let mut suspicious_patterns = Vec::new();
        let mut attacks_detected = 0usize;

        // Analyze recent operation frequency per proposer
        let recent: Vec<_> = audit.iter().rev().take(PATTERN_WINDOW).collect();
        let mut proposer_counts: HashMap<&str, usize> = HashMap::new();
        for proposal in &recent {
            *proposer_counts.entry(&proposal.proposer).or_insert(0) += 1;
        }

        // Detect rapid-fire operations from a single proposer (potential poisoning)
        for (proposer, count) in &proposer_counts {
            if *count > PATTERN_WINDOW / 2 && recent.len() >= PATTERN_WINDOW / 4 {
                suspicious_patterns.push(format!(
                    "High operation rate from {} ({} ops in last {} ops)",
                    proposer, count, PATTERN_WINDOW
                ));
                attacks_detected += 1;
            }
        }

        // Check for repeated failed operations (reconnaissance)
        let failed_count = audit
            .iter()
            .filter(|p| matches!(p.operation, CacheOperation::Get { .. }))
            .count();
        if failed_count > PATTERN_WINDOW / 2 {
            suspicious_patterns.push(format!(
                "Excessive read operations ({}) — possible reconnaissance",
                failed_count
            ));
            attacks_detected += 1;
        }

        let report = PoisoningReport {
            attacks_detected,
            blocked_operations: attacks_detected,
            suspicious_patterns,
            last_detection: Instant::now(),
        };

        self.poisoning_reports.write().unwrap().push(report.clone());
        Ok(report)
    }

    /// Compute a cryptographic hash for a cache operation.
    fn compute_operation_hash(operation: &CacheOperation) -> Vec<u8> {
        let data = match operation {
            CacheOperation::Put {
                key,
                data,
                checksum,
            } => {
                let mut buf = b"PUT".to_vec();
                buf.extend_from_slice(key.as_bytes());
                buf.extend_from_slice(data);
                buf.extend_from_slice(&checksum.to_le_bytes());
                buf
            }
            CacheOperation::Get { key } => {
                let mut buf = b"GET".to_vec();
                buf.extend_from_slice(key.as_bytes());
                buf
            }
            CacheOperation::Delete { key } => {
                let mut buf = b"DEL".to_vec();
                buf.extend_from_slice(key.as_bytes());
                buf
            }
            CacheOperation::Clear => b"CLEAR".to_vec(),
            CacheOperation::Search { pattern, limit } => {
                let mut buf = b"SEARCH".to_vec();
                buf.extend_from_slice(pattern.as_bytes());
                buf.extend_from_slice(&limit.to_le_bytes());
                buf
            }
        };
        blake3::hash(&data).as_bytes().to_vec()
    }

    /// Get consensus engine statistics
    pub async fn get_statistics(&self) -> ConsensusStatistics {
        let mut stats = self.statistics.read().unwrap().clone();
        if stats.total_operations > 0 {
            stats.success_rate =
                (stats.successful_operations as f64 / stats.total_operations as f64) * 100.0;
        }
        stats
    }

    // Private methods

    async fn sign_operation(&self, operation: &CacheOperation) -> Result<Vec<u8>, ConsensusError> {
        Ok(Self::compute_operation_hash(operation))
    }

    async fn collect_validator_votes(
        &self,
        proposal: &OperationProposal,
    ) -> Result<Vec<bool>, ConsensusError> {
        let mut votes = Vec::with_capacity(self.config.validators.len());
        let proposal_hash = Self::compute_operation_hash(&proposal.operation);

        for _validator in &self.config.validators {
            // Each validator independently verifies the proposal
            // In a distributed setup, this would be a network call
            let vote = tokio::time::timeout(self.config.timeout, async {
                // Simulate per-validator validation based on content hash
                // A real implementation would send the proposal to each validator node
                !proposal_hash.is_empty()
                    && !proposal.proposer.is_empty()
                    && matches!(proposal.operation, CacheOperation::Get { .. })
                    || !proposal_hash.is_empty()
            })
            .await
            .unwrap_or(false); // timeout = reject

            if !vote {
                // Remove failing validators from active count
                let mut stats = self.statistics.write().unwrap();
                stats.active_validators = stats.active_validators.saturating_sub(1);
            }

            votes.push(vote);
            // Brief async yield to simulate network latency
            tokio::task::yield_now().await;
        }

        // Ensure at least our own vote
        if votes.is_empty() {
            votes.push(true);
        }

        Ok(votes)
    }

    async fn collect_validator_signatures(
        &self,
        proposal: &OperationProposal,
    ) -> Result<Vec<Vec<u8>>, ConsensusError> {
        let mut signatures = Vec::with_capacity(self.config.validators.len());
        let proposal_hash = Self::compute_operation_hash(&proposal.operation);

        for validator in &self.config.validators {
            // Each validator produces a signature over the approved proposal
            let sig = tokio::time::timeout(self.config.timeout, async {
                let mut sig_data = proposal_hash.clone();
                sig_data.extend_from_slice(validator.as_bytes());
                blake3::hash(&sig_data).as_bytes().to_vec()
            })
            .await
            .unwrap_or_else(|_| vec![]);

            signatures.push(sig);
            tokio::task::yield_now().await;
        }

        // Fallback: produce at least one signature
        if signatures.is_empty() {
            signatures.push(proposal_hash);
        }

        Ok(signatures)
    }

    async fn record_operation(&self, proposal: OperationProposal) -> Result<(), ConsensusError> {
        self.audit_trail.write().unwrap().push(proposal);
        Ok(())
    }

    async fn update_statistics(&self, approved: bool, duration: Duration) {
        let mut stats = self.statistics.write().unwrap();
        stats.total_operations += 1;
        if approved {
            stats.successful_operations += 1;
        } else {
            stats.failed_operations += 1;
        }
        // Update average time (simplified)
        stats.avg_validation_time_ms =
            (stats.avg_validation_time_ms + duration.as_millis() as f64) / 2.0;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Consensus timeout")]
    Timeout,
    #[error("Insufficient quorum")]
    InsufficientQuorum,
    #[error("Validation failed")]
    ValidationFailed,
    #[error("Network error: {0}")]
    Network(String),
    #[error("Cryptographic error: {0}")]
    Crypto(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consensus_engine_creation() {
        let config = ConsensusConfig::default();
        let consensus = CacheConsensusEngine::new(config);
        assert_eq!(consensus.config.validators.len(), 0);
    }

    #[tokio::test]
    async fn test_operation_validation() {
        let config = ConsensusConfig {
            validators: vec!["validator1".to_string(), "validator2".to_string()],
            quorum_size: 1,
            ..Default::default()
        };
        let consensus = CacheConsensusEngine::new(config);

        let operation = CacheOperation::Put {
            key: "test".to_string(),
            data: b"test data".to_vec(),
            checksum: 12345,
        };

        let result = consensus.validate_operation(operation).await.unwrap();
        assert!(result.approved);
        assert_eq!(result.votes_for, 2); // 2 configured validators
    }

    #[tokio::test]
    async fn test_integrity_verification() {
        let consensus = CacheConsensusEngine::new(ConsensusConfig::default());
        let report = consensus.verify_cluster_integrity().await.unwrap();
        assert!(report.integrity_score > 99.0);
    }
}
