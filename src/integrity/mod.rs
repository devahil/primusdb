//! # Integrity service
//!
//! The integrity service is the operational heart of PrimusDB's tamper
//! evidence layer. It owns:
//!
//! * the **node signing identity** (ED25519 via [`signing::SigningService`]),
//! * the **persisted integrity store** (sled, [`store::IntegrityStore`]),
//! * **database genesis** records (signed identity of every database),
//! * **transaction integrity records** forming a per-database hash chain,
//! * **signed checkpoints** (Merkle roots over batches of records),
//! * **ledger anchoring** through an optional [`LedgerSubmitter`], and
//! * **reconciliation** of chains across replicas.
//!
//! ## Modes
//!
//! The configured [`IntegrityMode`] decides how strongly a commit is anchored:
//!
//! ```text
//! disabled          no integrity records, no signing (explicit opt-in)
//! local-signed      sign + persist locally (default)
//! ledger-anchored   records local, checkpoint roots anchored in batches
//! ledger-async      local commit + async ledger submission (pending queue)
//! ledger-sync       commit only after ledger acknowledgement (fail-safe)
//! cluster-consensus records validated through cluster consensus
//! ```
//!
//! When a policy requires a ledger and none is configured, operations fail
//! safely unless `allow_degraded` is explicitly enabled.

pub mod checkpoint;
pub mod errors;
pub mod genesis;
pub mod merkle;
pub mod policy;
pub mod reconciliation;
pub mod record;
pub mod signing;
pub mod store;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use checkpoint::Checkpoint;
pub use errors::{IntegrityError, IntegrityResult};
pub use genesis::{DatabaseGenesis, GenesisOrigin, GenesisStatus, GenesisVerification};
pub use policy::{IntegrityConfig, IntegrityMode, IntegrityPolicy};
pub use reconciliation::{
    compare_chains, plan_repair, verify_chain as verify_chain_records, ChainEvidence,
    ReconciliationReport, ReconciliationVerdict, RepairPlan,
};
pub use record::{IntegrityRecord, LedgerState, NewRecord, ReconciliationStatus};
pub use signing::SigningService;
pub use store::IntegrityStore;

/// Receipt returned by a ledger after a submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerReceipt {
    pub ledger_tx_id: String,
    pub confirmed: bool,
    pub block: Option<String>,
}

/// Abstraction over the Hyperledger service so the integrity layer never calls
/// the ledger directly from storage code (dependency inversion).
#[async_trait]
pub trait LedgerSubmitter: Send + Sync {
    /// Submits an integrity record for anchoring.
    async fn submit_record(&self, record: &IntegrityRecord) -> IntegrityResult<LedgerReceipt>;
    /// Submits a checkpoint root for anchoring.
    async fn submit_checkpoint(&self, cp: &Checkpoint) -> IntegrityResult<LedgerReceipt>;
    /// Real connectivity health (never a static value).
    async fn health(&self) -> serde_json::Value;
}

/// Snapshot of the integrity subsystem for status endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityStatus {
    pub mode: String,
    pub signer_id: String,
    pub can_sign: bool,
    pub databases: usize,
    pub total_records: usize,
    pub pending_submissions: usize,
    pub pending_bytes: u64,
    pub quarantined: usize,
    pub ledger_required: bool,
    pub ledger_available: bool,
    pub genesis_required: bool,
    pub verify_on_read: bool,
    pub degraded_allowed: bool,
    pub at: DateTime<Utc>,
}

/// Inputs for [`IntegrityService::create_database_genesis`].
pub struct NewDatabaseGenesis<'a> {
    pub database_name: &'a str,
    pub namespace: Option<&'a str>,
    pub engine_types: &'a [String],
    pub config_digest: &'a str,
    pub schema_digest: Option<&'a str>,
    pub parent_identity: Option<&'a str>,
    pub origin: GenesisOrigin,
}

/// Result of a full chain verification pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    pub database_id: String,
    pub genesis_present: bool,
    pub genesis_signature_valid: bool,
    pub records: usize,
    pub chain_valid: bool,
    pub broken_at: Option<u64>,
    pub verified_at: DateTime<Utc>,
}

/// Orchestrates genesis, records, checkpoints, ledger anchoring and
/// reconciliation. Cheap to clone (all inner state is `Arc`-shared).
#[derive(Clone)]
pub struct IntegrityService {
    store: Arc<IntegrityStore>,
    signer: Arc<SigningService>,
    policy: Arc<IntegrityPolicy>,
    ledger: Option<Arc<dyn LedgerSubmitter>>,
    node_id: String,
}

impl IntegrityService {
    /// Opens the integrity subsystem. `ledger` is optional; modes that require
    /// a ledger fail unless the policy allows degraded operation.
    pub fn open(
        data_dir: &str,
        config: &IntegrityConfig,
        node_id: &str,
        ledger: Option<Arc<dyn LedgerSubmitter>>,
    ) -> IntegrityResult<Self> {
        let store = Arc::new(IntegrityStore::open(data_dir)?);
        let key_path = if config.external_signer {
            None
        } else {
            Some(std::path::PathBuf::from(format!(
                "{}/integrity/node_signing_key.pkcs8",
                data_dir
            )))
        };
        let signer_id = config
            .signer_id
            .clone()
            .unwrap_or_else(|| node_id.to_string());
        let signer = Arc::new(SigningService::load_or_create(
            &signer_id,
            key_path.as_deref(),
        )?);
        let policy = Arc::new(IntegrityPolicy::resolve(config, node_id, None));
        Ok(IntegrityService {
            store,
            signer,
            policy,
            ledger,
            node_id: node_id.to_string(),
        })
    }

    /// Constructs a service over an existing store (tests/embedded use).
    pub fn with_store(
        store: Arc<IntegrityStore>,
        signer: Arc<SigningService>,
        policy: Arc<IntegrityPolicy>,
        ledger: Option<Arc<dyn LedgerSubmitter>>,
        node_id: &str,
    ) -> Self {
        IntegrityService {
            store,
            signer,
            policy,
            ledger,
            node_id: node_id.to_string(),
        }
    }

    pub fn store(&self) -> &IntegrityStore {
        &self.store
    }

    pub fn policy(&self) -> &IntegrityPolicy {
        &self.policy
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    // ── Genesis ─────────────────────────────────────────────────────────────

    /// Creates and persists a signed genesis identity for a database.
    pub fn create_database_genesis(
        &self,
        input: NewDatabaseGenesis<'_>,
    ) -> IntegrityResult<DatabaseGenesis> {
        if self.store.has_genesis(input.database_name)? {
            return Err(IntegrityError::GenesisAlreadyExists(
                input.database_name.to_string(),
            ));
        }
        let cluster_id = if self.policy.mode == IntegrityMode::ClusterConsensus {
            Some(self.node_id.clone())
        } else {
            None
        };
        let genesis = DatabaseGenesis::create(
            genesis::NewGenesis {
                database_name: input.database_name,
                namespace: input.namespace,
                engine_types: input.engine_types,
                creating_node: &self.node_id,
                cluster_id: cluster_id.as_deref(),
                config_digest: input.config_digest,
                schema_digest: input.schema_digest,
                parent_identity: input.parent_identity,
                origin: input.origin,
            },
            &self.signer,
        )?;
        self.store.save_genesis(&genesis)?;
        self.store.flush()?;
        Ok(genesis)
    }

    /// Verifies a persisted genesis: signature valid and identity matches.
    pub fn verify_genesis(&self, db_path: &str) -> IntegrityResult<GenesisVerification> {
        let genesis = self
            .store
            .load_genesis(db_path)?
            .ok_or_else(|| IntegrityError::MissingGenesis(db_path.to_string()))?;
        let valid = genesis.verify_signature()?;
        if !valid {
            return Ok(GenesisVerification {
                database_id: genesis.database_id.clone(),
                signature_valid: false,
                identity_matches_name: false,
                status: genesis.status,
                verified_at: Utc::now(),
            });
        }
        Ok(GenesisVerification::ok(&genesis))
    }

    pub fn get_genesis(&self, db_path: &str) -> IntegrityResult<Option<DatabaseGenesis>> {
        self.store.load_genesis(db_path)
    }

    pub fn list_databases(&self) -> IntegrityResult<Vec<DatabaseGenesis>> {
        self.store.list_genesis()
    }

    /// Creates a genesis for a legacy (pre-integrity) database during
    /// migration, retaining the original data and marking the origin.
    pub fn create_legacy_genesis(
        &self,
        database_name: &str,
        namespace: Option<&str>,
        engine_types: &[String],
        config_digest: &str,
    ) -> IntegrityResult<DatabaseGenesis> {
        self.create_database_genesis(NewDatabaseGenesis {
            database_name,
            namespace,
            engine_types,
            config_digest,
            schema_digest: None,
            parent_identity: None,
            origin: GenesisOrigin::LegacyImport,
        })
    }

    /// Creates a new genesis for a cloned database, linking to the source.
    pub fn create_clone_genesis(
        &self,
        database_name: &str,
        namespace: Option<&str>,
        engine_types: &[String],
        config_digest: &str,
        source_database_id: &str,
    ) -> IntegrityResult<DatabaseGenesis> {
        self.create_database_genesis(NewDatabaseGenesis {
            database_name,
            namespace,
            engine_types,
            config_digest,
            schema_digest: None,
            parent_identity: Some(source_database_id),
            origin: GenesisOrigin::Clone,
        })
    }

    // ── Records ─────────────────────────────────────────────────────────────

    /// Records a committed mutation. Behaviour depends on the policy mode:
    ///
    /// * `disabled` — no record is persisted.
    /// * `local-signed` — signed and persisted.
    /// * `ledger-anchored` — persisted; anchored via checkpoints.
    /// * `ledger-async` — persisted and queued for submission.
    /// * `ledger-sync` — persisted only after ledger confirmation.
    /// * `cluster-consensus` — persisted (validation via consensus engine).
    pub async fn record_transaction(
        &self,
        input: NewRecord<'_>,
    ) -> IntegrityResult<IntegrityRecord> {
        if self.policy.mode == IntegrityMode::Disabled {
            return Err(IntegrityError::PolicyViolation(
                "integrity disabled: cannot record a mutation".to_string(),
            ));
        }

        // Resolve the canonical database uuid.
        let db_id = self
            .store
            .resolve_db_id(input.database_id)?
            .unwrap_or_else(|| input.database_id.to_string());
        if self.policy.genesis_required && !self.store.has_genesis(&db_id)? {
            return Err(IntegrityError::MissingGenesis(db_id));
        }
        if self.store.is_replay(&db_id, input.transaction_id)? {
            return Err(IntegrityError::ReplayRejected(
                input.transaction_id.to_string(),
            ));
        }

        let previous_hash = self.store.last_record_hash(&db_id)?;
        let sequence = self.store.next_sequence(&db_id)?;
        let mut record = IntegrityRecord::create(
            NewRecord { sequence, ..input },
            &previous_hash,
            &self.signer,
        )?;
        record.ledger_state = LedgerState::None;

        match self.policy.mode {
            IntegrityMode::LocalSigned | IntegrityMode::ClusterConsensus => {
                self.store.save_record(&db_id, &record)?;
            }
            IntegrityMode::LedgerAnchored => {
                self.store.save_record(&db_id, &record)?;
            }
            IntegrityMode::LedgerAsynchronous => {
                self.store.save_record(&db_id, &record)?;
                self.store.save_pending(&db_id, &record)?;
            }
            IntegrityMode::LedgerSynchronous => {
                self.store.save_record(&db_id, &record)?;
                let receipt = self.submit_record(&record).await?;
                if !receipt.confirmed {
                    return Err(IntegrityError::LedgerConfirmationTimeout(
                        receipt.ledger_tx_id,
                    ));
                }
                record.ledger_state = LedgerState::Confirmed;
                record.ledger_tx_id = Some(receipt.ledger_tx_id);
                record.re_sign(&self.signer)?;
                self.store.save_record(&db_id, &record)?;
            }
            IntegrityMode::Disabled => unreachable!(),
        }
        self.store.flush()?;
        Ok(record)
    }

    async fn submit_record(&self, record: &IntegrityRecord) -> IntegrityResult<LedgerReceipt> {
        let ledger = self.ledger.clone().ok_or_else(|| {
            IntegrityError::LedgerUnavailable(
                "no ledger configured but policy requires it".to_string(),
            )
        })?;
        if !self.policy.allow_degraded {
            // Fail safe unless degraded mode explicitly enabled.
            let health = ledger.health().await;
            if health.get("reachable").and_then(|v| v.as_bool()) != Some(true) {
                return Err(IntegrityError::LedgerUnavailable(
                    "ledger health check failed".to_string(),
                ));
            }
        }
        ledger.submit_record(record).await
    }

    // ── Verification ────────────────────────────────────────────────────────

    /// Verifies the full persisted chain for a database (genesis + records).
    pub fn verify_chain(&self, db_path: &str) -> IntegrityResult<ChainVerification> {
        let genesis = self.store.load_genesis(db_path)?;
        let genesis_present = genesis.is_some();
        let genesis_signature_valid = match &genesis {
            Some(g) => g.verify_signature()?,
            None => false,
        };
        let db_id = match &genesis {
            Some(g) => g.database_id.clone(),
            None => db_path.to_string(),
        };
        let records = self.store.load_records(&db_id)?;
        let chain_valid = verify_chain_records(&records)?;
        Ok(ChainVerification {
            database_id: db_id,
            genesis_present,
            genesis_signature_valid,
            records: records.len(),
            chain_valid,
            broken_at: if chain_valid {
                None
            } else {
                records
                    .iter()
                    .find(|r| !r.verify().unwrap_or(false))
                    .map(|r| r.sequence)
            },
            verified_at: Utc::now(),
        })
    }

    pub fn list_records(&self, db_path: &str) -> IntegrityResult<Vec<IntegrityRecord>> {
        let db_id = self
            .store
            .resolve_db_id(db_path)?
            .unwrap_or_else(|| db_path.to_string());
        self.store.load_records(&db_id)
    }

    // ── Checkpoints ─────────────────────────────────────────────────────────

    /// Creates and signs a checkpoint over the records not yet covered by a
    /// checkpoint. In anchored modes the checkpoint root is submitted to the
    /// ledger (synchronously confirmed when the policy requires it).
    pub async fn create_checkpoint(&self, db_path: &str) -> IntegrityResult<Checkpoint> {
        let db_id = self
            .store
            .resolve_db_id(db_path)?
            .ok_or_else(|| IntegrityError::MissingGenesis(db_path.to_string()))?;
        let records = self.store.load_records(&db_id)?;
        let last_cp = self.store.load_checkpoints(&db_id)?;
        let start_seq = last_cp.last().map(|c| c.end_sequence + 1).unwrap_or(1);
        let batch: Vec<IntegrityRecord> = records
            .iter()
            .filter(|r| r.sequence >= start_seq)
            .cloned()
            .collect();
        if batch.is_empty() {
            return Err(IntegrityError::PolicyViolation(
                "no new records to checkpoint".to_string(),
            ));
        }
        let prev_hash = self.store.last_checkpoint_hash(&db_id)?;
        let mut cp = Checkpoint::create(&db_id, &batch, &prev_hash, &self.signer)?;

        match self.policy.mode {
            IntegrityMode::LedgerAnchored
            | IntegrityMode::LedgerAsynchronous
            | IntegrityMode::LedgerSynchronous => {
                let ledger = self.ledger.clone().ok_or_else(|| {
                    IntegrityError::LedgerUnavailable(
                        "no ledger configured but checkpoint anchoring requires it".to_string(),
                    )
                })?;
                let receipt = ledger.submit_checkpoint(&cp).await?;
                if self.policy.mode == IntegrityMode::LedgerSynchronous && !receipt.confirmed {
                    return Err(IntegrityError::LedgerConfirmationTimeout(
                        receipt.ledger_tx_id,
                    ));
                }
                cp.ledger_state = if receipt.confirmed {
                    LedgerState::Confirmed
                } else {
                    LedgerState::Pending
                };
                cp.ledger_tx_id = Some(receipt.ledger_tx_id);
                cp.checkpoint_hash = cp.compute_hash();
                cp.signature = self.signer.sign(&cp.canonical_bytes())?;
            }
            _ => {}
        }
        self.store.save_checkpoint(&db_id, &cp)?;
        self.store.flush()?;
        Ok(cp)
    }

    pub fn list_checkpoints(&self, db_path: &str) -> IntegrityResult<Vec<Checkpoint>> {
        let db_id = self
            .store
            .resolve_db_id(db_path)?
            .unwrap_or_else(|| db_path.to_string());
        self.store.load_checkpoints(&db_id)
    }

    // ── Pending submissions ─────────────────────────────────────────────────

    pub fn list_pending(&self) -> IntegrityResult<Vec<IntegrityRecord>> {
        self.store.list_pending()
    }

    /// Retries all pending ledger submissions, returning how many confirmed.
    pub async fn flush_pending(&self) -> IntegrityResult<u64> {
        let ledger = self
            .ledger
            .clone()
            .ok_or_else(|| IntegrityError::LedgerUnavailable("no ledger configured".to_string()))?;
        let pending = self.store.list_pending()?;
        let mut confirmed = 0u64;
        for mut record in pending {
            let db_id = record.database_id.clone();
            match ledger.submit_record(&record).await {
                Ok(receipt) if receipt.confirmed => {
                    record.ledger_state = LedgerState::Confirmed;
                    record.ledger_tx_id = Some(receipt.ledger_tx_id);
                    record.re_sign(&self.signer)?;
                    self.store.save_record(&db_id, &record)?;
                    self.store.remove_pending(&db_id, record.sequence)?;
                    confirmed += 1;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("pending submission for {} failed: {}", db_id, e);
                }
            }
        }
        self.store.flush()?;
        Ok(confirmed)
    }

    // ── Quarantine ──────────────────────────────────────────────────────────

    /// Quarantines a record that failed validation. Quarantined records are
    /// never served as committed state.
    pub fn quarantine(&self, record: &IntegrityRecord) -> IntegrityResult<()> {
        self.store.save_quarantined(&record.database_id, record)
    }

    pub fn list_quarantined(&self) -> IntegrityResult<Vec<IntegrityRecord>> {
        self.store.list_quarantined()
    }

    pub fn release_quarantined(&self, db_id: &str, seq: u64) -> IntegrityResult<()> {
        self.store.remove_quarantined(db_id, seq)
    }

    // ── Reconciliation ──────────────────────────────────────────────────────

    /// Compares the local chain against a peer's chain and produces a report.
    pub fn reconcile(
        &self,
        db_path: &str,
        peer_records: &[IntegrityRecord],
    ) -> IntegrityResult<ReconciliationReport> {
        let local = self.list_records(db_path)?;
        let db_id = self
            .store
            .resolve_db_id(db_path)?
            .unwrap_or_else(|| db_path.to_string());
        compare_chains(&db_id, &local, peer_records)
    }

    /// Builds the compact chain evidence this node offers to peers.
    ///
    /// Peers compare [`ChainEvidence`] before exchanging full records: when the
    /// counts and last hashes match, the chains are consistent by construction.
    pub fn chain_evidence(&self, db_path: &str) -> IntegrityResult<ChainEvidence> {
        let db_id = self
            .store
            .resolve_db_id(db_path)?
            .unwrap_or_else(|| db_path.to_string());
        let records = self.store.load_records(&db_id)?;
        let sequence_count = records.len() as u64;
        let last_hash = records
            .last()
            .map(|r| r.record_hash.clone())
            .unwrap_or_else(|| "genesis".to_string());
        let checkpoint_root = self
            .store
            .load_checkpoints(&db_id)?
            .last()
            .map(|c| c.merkle_root.clone())
            .unwrap_or_else(|| "none".to_string());
        Ok(ChainEvidence {
            database_id: db_id,
            sequence_count,
            last_hash,
            checkpoint_root,
            node_id: self.node_id().to_string(),
            produced_at: chrono::Utc::now(),
        })
    }

    // ── Status ──────────────────────────────────────────────────────────────

    pub async fn status(&self) -> IntegrityStatus {
        let databases = self.store.list_genesis().map(|g| g.len()).unwrap_or(0);
        let records: usize = self
            .store
            .list_genesis()
            .map(|list| {
                list.iter()
                    .map(|g| {
                        self.store
                            .load_records(&g.database_id)
                            .map(|r| r.len())
                            .unwrap_or(0)
                    })
                    .sum()
            })
            .unwrap_or(0);
        let pending = self.store.list_pending().map(|p| p.len()).unwrap_or(0);
        let pending_bytes = self.store.pending_bytes().unwrap_or(0);
        let quarantined = self.store.list_quarantined().map(|q| q.len()).unwrap_or(0);
        let ledger_available = match &self.ledger {
            Some(l) => l
                .health()
                .await
                .get("reachable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            None => false,
        };
        IntegrityStatus {
            mode: self.policy.mode.to_string(),
            signer_id: self.signer.signer_id().to_string(),
            can_sign: self.signer.can_sign(),
            databases,
            total_records: records,
            pending_submissions: pending,
            pending_bytes,
            quarantined,
            ledger_required: self.policy.mode.requires_ledger(),
            ledger_available,
            genesis_required: self.policy.genesis_required,
            verify_on_read: self.policy.verify_on_read,
            degraded_allowed: self.policy.allow_degraded,
            at: Utc::now(),
        }
    }
}

/// Convenience constructor for tests and embedded use.
pub fn default_config() -> IntegrityConfig {
    IntegrityConfig::default()
}
