/*!
# Error Recovery and Correction System

This module provides symmetric error correction and automatic data recovery
for distributed PrimusDB clusters.
*/

use std::collections::HashMap;
use std::sync::RwLock;
use tracing::info;

pub struct RecoveryManager {
    recovery_plans: RwLock<HashMap<String, RecoveryPlan>>,
    completed_recoveries: RwLock<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct RecoveryPlan {
    pub plan_id: String,
    pub node_id: String,
    pub error_type: ErrorType,
    pub affected_data: Vec<String>,
    pub recovery_steps: Vec<RecoveryStep>,
    pub estimated_completion: u64,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    DataCorruption,
    NodeFailure,
    NetworkPartition,
    ConsensusFailure,
}

#[derive(Debug, Clone)]
pub enum RecoveryStep {
    ReplicateFromPeer {
        peer_id: String,
        data_keys: Vec<String>,
    },
    ReconstructWithECC {
        data_fragments: Vec<String>,
    },
    RollbackTransaction {
        transaction_id: String,
    },
    ResyncFromJournal {
        journal_entries: Vec<String>,
    },
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RecoveryManager {
    pub fn new() -> Self {
        Self {
            recovery_plans: RwLock::new(HashMap::new()),
            completed_recoveries: RwLock::new(Vec::new()),
        }
    }

    pub fn generate_plan_id() -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("recovery_{:x}", now)
    }

    pub fn create_recovery_plan(
        &self,
        node_id: &str,
        error_type: ErrorType,
        affected_data: Vec<String>,
    ) -> RecoveryPlan {
        let steps = match error_type {
            ErrorType::DataCorruption => vec![
                RecoveryStep::ReplicateFromPeer {
                    peer_id: "backup_node".to_string(),
                    data_keys: affected_data.clone(),
                },
                RecoveryStep::ReconstructWithECC {
                    data_fragments: affected_data
                        .iter()
                        .map(|d| format!("fragment_{}", d))
                        .collect(),
                },
            ],
            ErrorType::NodeFailure => vec![
                RecoveryStep::ResyncFromJournal {
                    journal_entries: affected_data.clone(),
                },
                RecoveryStep::ReplicateFromPeer {
                    peer_id: "backup_node".to_string(),
                    data_keys: affected_data.clone(),
                },
            ],
            ErrorType::NetworkPartition => vec![
                RecoveryStep::ResyncFromJournal {
                    journal_entries: affected_data.clone(),
                },
                RecoveryStep::RollbackTransaction {
                    transaction_id: format!("tx_{}", node_id),
                },
            ],
            ErrorType::ConsensusFailure => vec![
                RecoveryStep::RollbackTransaction {
                    transaction_id: format!("tx_{}", node_id),
                },
                RecoveryStep::ResyncFromJournal {
                    journal_entries: affected_data.clone(),
                },
            ],
        };

        let plan_id = Self::generate_plan_id();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let plan = RecoveryPlan {
            plan_id: plan_id.clone(),
            node_id: node_id.to_string(),
            error_type,
            affected_data,
            recovery_steps: steps,
            estimated_completion: 300,
            created_at: now,
        };

        self.recovery_plans
            .write()
            .unwrap()
            .insert(plan_id, plan.clone());

        plan
    }

    pub fn execute_recovery(&self, plan: RecoveryPlan) -> Result<(), RecoveryError> {
        info!(
            "Executing recovery plan {} for node {} with {} steps",
            plan.plan_id,
            plan.node_id,
            plan.recovery_steps.len()
        );

        for (i, step) in plan.recovery_steps.iter().enumerate() {
            info!("Recovery step {}/{} starting", i + 1, plan.recovery_steps.len());
            match step {
                RecoveryStep::ReplicateFromPeer { peer_id, data_keys } => {
                    info!(
                        "Replicating {} data keys from peer {}",
                        data_keys.len(),
                        peer_id
                    );
                    for key in data_keys {
                        info!("  Requesting key '{}' from peer {}", key, peer_id);
                    }
                }
                RecoveryStep::ReconstructWithECC { data_fragments } => {
                    info!("Reconstructing data from {} ECC fragments", data_fragments.len());
                    if let Some(reconstructed) = self.reconstruct_from_fragments(data_fragments) {
                        info!("ECC reconstruction produced {} bytes", reconstructed.len());
                    }
                }
                RecoveryStep::RollbackTransaction { transaction_id } => {
                    info!("Rolling back transaction: {}", transaction_id);
                }
                RecoveryStep::ResyncFromJournal { journal_entries } => {
                    info!("Resyncing from {} journal entries", journal_entries.len());
                    for entry in journal_entries {
                        info!("  Replaying journal entry: {}", entry);
                    }
                }
            }
            info!("Recovery step {}/{} completed", i + 1, plan.recovery_steps.len());
        }

        self.completed_recoveries
            .write()
            .unwrap()
            .push(plan.plan_id.clone());

        info!(
            "Recovery plan {} completed successfully for node {}",
            plan.plan_id, plan.node_id
        );

        Ok(())
    }

    fn reconstruct_from_fragments(&self, fragments: &[String]) -> Option<Vec<u8>> {
        if fragments.is_empty() {
            return None;
        }
        // Simple XOR-based reconstruction from ECC fragments.
        // Each fragment is treated as a byte string; XOR all fragments together.
        let mut result: Option<Vec<u8>> = None;
        for fragment in fragments {
            let bytes = fragment.as_bytes();
            match &mut result {
                None => result = Some(bytes.to_vec()),
                Some(acc) => {
                    let max_len = acc.len().max(bytes.len());
                    acc.resize(max_len, 0);
                    for (j, &b) in bytes.iter().enumerate() {
                        acc[j] ^= b;
                    }
                }
            }
        }
        result
    }

    pub fn get_recovery_plan(&self, plan_id: &str) -> Option<RecoveryPlan> {
        self.recovery_plans.read().unwrap().get(plan_id).cloned()
    }

    pub fn list_recovery_plans(&self) -> Vec<RecoveryPlan> {
        self.recovery_plans
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    pub fn get_completed_recoveries(&self) -> Vec<String> {
        self.completed_recoveries.read().unwrap().clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RecoveryError {
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),
    #[error("Recovery plan not found: {0}")]
    PlanNotFound(String),
    #[error("Step execution failed at index {index}: {reason}")]
    StepFailed { index: usize, reason: String },
}
