//! Durable persistence for integrity data.
//!
//! A single sled database (at `{data_dir}/integrity`) stores genesis records,
//! chained transaction records, checkpoints, pending ledger submissions and
//! quarantined data using a shared tree with namespaced keys:
//!
//! ```text
//! gen:{path}                 genesis by database path
//! gen:{id}                   genesis by database uuid
//! rec:{db_id}:{seq:020}      chained integrity records (ordered)
//! rec:tx:{db_id}:{tx_id}     transaction-id index (replay detection)
//! cp:{db_id}:{ts_nanos}      checkpoints
//! pend:{db_id}:{seq}         pending ledger submissions
//! quar:{db_id}:{seq}         quarantined records
//! meta:lastseq:{db_id}       last assigned sequence per database
//! ```

use std::sync::Arc;

use super::checkpoint::Checkpoint;
use super::errors::{IntegrityError, IntegrityResult};
use super::genesis::DatabaseGenesis;
use super::record::IntegrityRecord;

/// Sled-backed integrity store.
pub struct IntegrityStore {
    db: Arc<sled::Db>,
}

impl IntegrityStore {
    /// Opens (creating if needed) the integrity store under `data_dir`.
    pub fn open(data_dir: &str) -> IntegrityResult<Self> {
        let path = format!("{}/integrity", data_dir);
        std::fs::create_dir_all(&path)
            .map_err(|e| IntegrityError::Storage(format!("cannot create integrity dir: {}", e)))?;
        let db = sled::open(&path)
            .map_err(|e| IntegrityError::Storage(format!("cannot open integrity store: {}", e)))?;
        Ok(IntegrityStore { db: Arc::new(db) })
    }

    /// Opens a store on an existing sled database (test/integration use).
    pub fn with_db(db: Arc<sled::Db>) -> Self {
        IntegrityStore { db }
    }

    fn key(db_id: &str, seq: u64) -> Vec<u8> {
        format!("rec:{}:{:020}", db_id, seq).into_bytes()
    }

    fn gen_key(id: &str) -> Vec<u8> {
        format!("gen:{}", id).into_bytes()
    }

    fn cp_key(db_id: &str, ts_nanos: i64) -> Vec<u8> {
        format!("cp:{}:{}", db_id, ts_nanos).into_bytes()
    }

    fn pend_key(db_id: &str, seq: u64) -> Vec<u8> {
        format!("pend:{}:{}", db_id, seq).into_bytes()
    }

    fn quar_key(db_id: &str, seq: u64) -> Vec<u8> {
        format!("quar:{}:{}", db_id, seq).into_bytes()
    }

    // ── Genesis ─────────────────────────────────────────────────────────────

    /// Persists a genesis record keyed by both its path and uuid.
    pub fn save_genesis(&self, genesis: &DatabaseGenesis) -> IntegrityResult<()> {
        let bytes = serde_json::to_vec(genesis)?;
        self.db
            .insert(Self::gen_key(&genesis.database_name), bytes.clone())?;
        self.db.insert(Self::gen_key(&genesis.database_id), bytes)?;
        self.db
            .flush()
            .map_err(|e| IntegrityError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Loads a genesis record by uuid or database path.
    pub fn load_genesis(&self, id: &str) -> IntegrityResult<Option<DatabaseGenesis>> {
        match self.db.get(Self::gen_key(id))? {
            Some(ivec) => Ok(Some(serde_json::from_slice(&ivec)?)),
            None => Ok(None),
        }
    }

    /// Lists every persisted genesis record.
    pub fn list_genesis(&self) -> IntegrityResult<Vec<DatabaseGenesis>> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for item in self.db.scan_prefix(b"gen:") {
            let (_, value) = item?;
            let genesis: DatabaseGenesis = serde_json::from_slice(&value)?;
            if seen.insert(genesis.database_id.clone()) {
                out.push(genesis);
            }
        }
        Ok(out)
    }

    /// Resolves the canonical database uuid for a path (or returns the input if
    /// it already is a known uuid).
    pub fn resolve_db_id(&self, path_or_id: &str) -> IntegrityResult<Option<String>> {
        if let Some(gen) = self.load_genesis(path_or_id)? {
            return Ok(Some(gen.database_id));
        }
        // Path may be a nested namespace; try the direct name key too.
        Ok(None)
    }

    /// True when any genesis exists for the given path or uuid.
    pub fn has_genesis(&self, path_or_id: &str) -> IntegrityResult<bool> {
        Ok(self.load_genesis(path_or_id)?.is_some())
    }

    // ── Sequences ───────────────────────────────────────────────────────────

    fn last_seq_key(db_id: &str) -> Vec<u8> {
        format!("meta:lastseq:{}", db_id).into_bytes()
    }

    /// Returns the next sequence number for a database (1-based).
    pub fn next_sequence(&self, db_id: &str) -> IntegrityResult<u64> {
        match self.db.get(Self::last_seq_key(db_id))? {
            Some(ivec) => {
                let seq: u64 = bincode::deserialize(&ivec)
                    .map_err(|e| IntegrityError::Storage(e.to_string()))?;
                Ok(seq + 1)
            }
            None => Ok(1),
        }
    }

    fn set_last_sequence(&self, db_id: &str, seq: u64) -> IntegrityResult<()> {
        let bytes = bincode::serialize(&seq).map_err(|e| IntegrityError::Storage(e.to_string()))?;
        self.db.insert(Self::last_seq_key(db_id), bytes)?;
        Ok(())
    }

    // ── Records ─────────────────────────────────────────────────────────────

    /// Persists a record, advancing the per-database sequence.
    pub fn save_record(&self, db_id: &str, record: &IntegrityRecord) -> IntegrityResult<()> {
        let bytes = serde_json::to_vec(record)?;
        self.db.insert(Self::key(db_id, record.sequence), bytes)?;
        let tx_key = format!("rec:tx:{}:{}", db_id, record.transaction_id).into_bytes();
        self.db.insert(tx_key, b"1")?;
        self.set_last_sequence(db_id, record.sequence)?;
        Ok(())
    }

    /// True when a transaction id has already been recorded for the database.
    pub fn is_replay(&self, db_id: &str, transaction_id: &str) -> IntegrityResult<bool> {
        let key = format!("rec:tx:{}:{}", db_id, transaction_id).into_bytes();
        Ok(self.db.get(key)?.is_some())
    }

    /// Loads all records for a database ordered by sequence.
    pub fn load_records(&self, db_id: &str) -> IntegrityResult<Vec<IntegrityRecord>> {
        let mut out: Vec<IntegrityRecord> = Vec::new();
        let prefix = format!("rec:{}:", db_id).into_bytes();
        for item in self.db.scan_prefix(prefix) {
            let (_, value) = item?;
            out.push(serde_json::from_slice(&value)?);
        }
        out.sort_by_key(|r| r.sequence);
        Ok(out)
    }

    /// Hash of the last record in the chain ("genesis" when the chain is empty).
    pub fn last_record_hash(&self, db_id: &str) -> IntegrityResult<String> {
        let records = self.load_records(db_id)?;
        Ok(records
            .last()
            .map(|r| r.record_hash.clone())
            .unwrap_or_else(|| "genesis".to_string()))
    }

    // ── Checkpoints ─────────────────────────────────────────────────────────

    /// Persists a checkpoint.
    pub fn save_checkpoint(&self, db_id: &str, cp: &Checkpoint) -> IntegrityResult<()> {
        let bytes = serde_json::to_vec(cp)?;
        self.db.insert(
            Self::cp_key(db_id, cp.timestamp.timestamp_nanos_opt().unwrap_or(0)),
            bytes,
        )?;
        Ok(())
    }

    /// Loads all checkpoints for a database ordered by creation time.
    pub fn load_checkpoints(&self, db_id: &str) -> IntegrityResult<Vec<Checkpoint>> {
        let mut out: Vec<Checkpoint> = Vec::new();
        let prefix = format!("cp:{}:", db_id).into_bytes();
        for item in self.db.scan_prefix(prefix) {
            let (_, value) = item?;
            out.push(serde_json::from_slice(&value)?);
        }
        out.sort_by_key(|c| c.end_sequence);
        Ok(out)
    }

    /// Hash of the last checkpoint ("genesis" when none exists).
    pub fn last_checkpoint_hash(&self, db_id: &str) -> IntegrityResult<String> {
        let cps = self.load_checkpoints(db_id)?;
        Ok(cps
            .last()
            .map(|c| c.checkpoint_hash.clone())
            .unwrap_or_else(|| "genesis".to_string()))
    }

    // ── Pending ledger submissions ──────────────────────────────────────────

    pub fn save_pending(&self, db_id: &str, record: &IntegrityRecord) -> IntegrityResult<()> {
        let bytes = serde_json::to_vec(record)?;
        self.db
            .insert(Self::pend_key(db_id, record.sequence), bytes)?;
        Ok(())
    }

    pub fn list_pending(&self) -> IntegrityResult<Vec<IntegrityRecord>> {
        let mut out: Vec<IntegrityRecord> = Vec::new();
        for item in self.db.scan_prefix(b"pend:") {
            let (_, value) = item?;
            out.push(serde_json::from_slice(&value)?);
        }
        out.sort_by_key(|r| r.sequence);
        Ok(out)
    }

    pub fn pending_bytes(&self) -> IntegrityResult<u64> {
        let mut total = 0u64;
        for item in self.db.scan_prefix(b"pend:") {
            let (_, value) = item?;
            total += value.len() as u64;
        }
        Ok(total)
    }

    pub fn remove_pending(&self, db_id: &str, seq: u64) -> IntegrityResult<()> {
        self.db.remove(Self::pend_key(db_id, seq))?;
        Ok(())
    }

    // ── Quarantine ──────────────────────────────────────────────────────────

    pub fn save_quarantined(&self, db_id: &str, record: &IntegrityRecord) -> IntegrityResult<()> {
        let bytes = serde_json::to_vec(record)?;
        self.db
            .insert(Self::quar_key(db_id, record.sequence), bytes)?;
        Ok(())
    }

    pub fn list_quarantined(&self) -> IntegrityResult<Vec<IntegrityRecord>> {
        let mut out: Vec<IntegrityRecord> = Vec::new();
        for item in self.db.scan_prefix(b"quar:") {
            let (_, value) = item?;
            out.push(serde_json::from_slice(&value)?);
        }
        out.sort_by_key(|r| r.sequence);
        Ok(out)
    }

    pub fn remove_quarantined(&self, db_id: &str, seq: u64) -> IntegrityResult<()> {
        self.db.remove(Self::quar_key(db_id, seq))?;
        Ok(())
    }

    /// Deletes all integrity data for a database (used on database drop).
    pub fn delete_database(&self, db_id: &str) -> IntegrityResult<()> {
        let prefixes = [
            format!("rec:{}:", db_id),
            format!("rec:tx:{}:", db_id),
            format!("cp:{}:", db_id),
            format!("pend:{}:", db_id),
            format!("quar:{}:", db_id),
        ];
        for prefix in prefixes {
            let keys: Vec<sled::IVec> = self
                .db
                .scan_prefix(prefix.as_bytes())
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|(k, _)| k)
                .collect();
            for k in keys {
                self.db.remove(k)?;
            }
        }
        self.db.remove(Self::last_seq_key(db_id))?;
        // Remove genesis by uuid (path key remains for lookup history).
        if let Some(gen) = self.load_genesis(db_id)? {
            self.db.remove(Self::gen_key(&gen.database_id))?;
        }
        Ok(())
    }

    pub fn flush(&self) -> IntegrityResult<()> {
        self.db
            .flush()
            .map_err(|e| IntegrityError::Storage(e.to_string()))?;
        Ok(())
    }
}
