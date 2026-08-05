/*
 * PrimusDB Persistent Search Index
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.0.0
 */

/*!
# Persistent inverted index for unified search

Unified full-text search historically scanned tables live on every query.
This module makes the inverted index **persistent** (sled-backed) with a
lazy-rebuild invalidation model:

- **Create** mutations incrementally index the new record (append-friendly:
  logs, time-series and document streams benefit immediately).
- **Update / Delete / Truncate** mark the affected table's segment as *dirty*
  instead of re-indexing on every write; the segment is rebuilt from live data
  on the next search that touches it.
- Segments are serialized to sled, so indexed data survives restarts (only
  dirty segments are rebuilt after a restart).

## Honest limitations

- The index caches the searchable text plus the record snapshot. Records that
  change outside a `Create` (e.g. through direct engine APIs that bypass the
  query engine) are covered by the dirty-marking in [`crate::PrimusDB`]
  whenever the mutation goes through `execute_query`.
- `FullTextIndex` uses `u64` document ids scoped per table; rebuilding a
  segment reassigns ids, which is safe because ids are only meaningful inside
  a segment.
- The index is best-effort: an indexing failure is logged and never allowed to
  fail an already-committed write.
*/

use crate::fulltext::FullTextIndex;
use crate::search::SearchMode;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

/// One indexed document within a table segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedDoc {
    /// Stable record identifier (engine-specific).
    pub id: String,
    /// The record snapshot at index time.
    pub record: serde_json::Value,
}

/// The full-text segment for one table: its inverted index plus the documents
/// the doc ids refer to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSegment {
    /// TF-IDF inverted index over the table's searchable text.
    pub index: FullTextIndex,
    /// Documents in doc-id order (matches `index`'s doc ids).
    pub docs: Vec<IndexedDoc>,
}

impl SearchSegment {
    pub fn new() -> Self {
        Self {
            index: FullTextIndex::new(),
            docs: Vec::new(),
        }
    }

    /// Append a document, assigning the next sequential doc id.
    pub fn push_doc(&mut self, id: String, record: serde_json::Value, text: &str) {
        let doc_id = self.docs.len() as u64;
        self.index.index_document(doc_id, text);
        self.docs.push(IndexedDoc { id, record });
    }

    /// Remove a document by its record id, rebuilding the index.
    pub fn remove_doc(&mut self, id: &str) {
        let before = self.docs.len();
        self.docs.retain(|d| d.id != id);
        if self.docs.len() != before {
            self.rebuild_index();
        }
    }

    /// Rebuild the inverted index from the stored documents after a removal.
    fn rebuild_index(&mut self) {
        let docs = std::mem::take(&mut self.docs);
        self.index = FullTextIndex::new();
        for (i, doc) in docs.iter().enumerate() {
            self.index.index_document(
                i as u64,
                &crate::search::collect_searchable_text(&doc.record),
            );
        }
        self.docs = docs;
    }
}

impl Default for SearchSegment {
    fn default() -> Self {
        Self::new()
    }
}

/// Composite key for a table segment: `engine \t table`.
fn segment_key(engine: &str, table: &str) -> String {
    format!("{engine}\t{table}")
}

/// Persistent, sled-backed inverted index used by unified search.
pub struct PersistentSearchIndex {
    db: sled::Db,
    /// In-memory cache of segments, keyed by `segment_key`.
    segments: RwLock<HashMap<String, SearchSegment>>,
}

const SEGMENTS_TREE: &[u8] = b"segments";
const DIRTY_TREE: &[u8] = b"dirty";

impl PersistentSearchIndex {
    /// Open (or create) the persistent index at `path`.
    pub fn open(path: &Path) -> Result<Self> {
        let db = sled::Config::new().path(path).open()?;

        // Warm the segment cache from disk.
        let mut segments = HashMap::new();
        for item in db.open_tree(SEGMENTS_TREE)?.iter() {
            let (key, value) = match item {
                Ok(kv) => kv,
                Err(_) => continue,
            };
            if let Ok(key_str) = std::str::from_utf8(&key) {
                if let Ok(segment) = serde_json::from_slice::<SearchSegment>(&value) {
                    segments.insert(key_str.to_string(), segment);
                }
            }
        }

        Ok(Self {
            db,
            segments: RwLock::new(segments),
        })
    }

    /// Whether the given table's segment is dirty (needs a rebuild).
    pub fn is_dirty(&self, engine: &str, table: &str) -> bool {
        self.db
            .open_tree(DIRTY_TREE)
            .ok()
            .and_then(|tree| {
                tree.contains_key(segment_key(engine, table).as_bytes())
                    .ok()
            })
            .unwrap_or(false)
    }

    /// Mark a table's segment dirty and evict the cached copy.
    pub fn mark_dirty(&self, engine: &str, table: &str) {
        let key = segment_key(engine, table);
        if let Ok(tree) = self.db.open_tree(DIRTY_TREE) {
            let _ = tree.insert(key.as_bytes(), &[1u8]);
            let _ = self.db.flush();
        }
        if let Ok(mut segments) = self.segments.write() {
            segments.remove(&key);
        }
    }

    /// Clear a table's dirty flag.
    pub fn clear_dirty(&self, engine: &str, table: &str) {
        let key = segment_key(engine, table);
        if let Ok(tree) = self.db.open_tree(DIRTY_TREE) {
            let _ = tree.remove(key.as_bytes());
        }
    }

    /// Get the cached segment for a table, if present.
    pub fn get_segment(&self, engine: &str, table: &str) -> Option<SearchSegment> {
        let key = segment_key(engine, table);
        self.segments.read().ok().and_then(|s| s.get(&key).cloned())
    }

    /// Store a rebuilt segment for a table and clear its dirty flag.
    pub fn set_segment(&self, engine: &str, table: &str, segment: SearchSegment) {
        let key = segment_key(engine, table);
        if let Ok(mut segments) = self.segments.write() {
            segments.insert(key.clone(), segment.clone());
        }
        if let Ok(tree) = self.db.open_tree(SEGMENTS_TREE) {
            if let Ok(json) = serde_json::to_vec(&segment) {
                let _ = tree.insert(key.as_bytes(), json);
            }
        }
        self.clear_dirty(engine, table);
        let _ = self.db.flush();
    }

    /// Incrementally index a new document into its table's segment.
    ///
    /// Loads the segment (from cache or disk), appends the document and
    /// persists. If the segment is dirty it is left dirty: a subsequent
    /// rebuild will cover it.
    pub fn insert_document(
        &self,
        engine: &str,
        table: &str,
        id: &str,
        record: serde_json::Value,
        text: &str,
    ) -> Result<()> {
        let key = segment_key(engine, table);
        let mut segment = self.get_segment(engine, table).unwrap_or_default();
        segment.push_doc(id.to_string(), record, text);
        if let Ok(mut segments) = self.segments.write() {
            segments.insert(key.clone(), segment.clone());
        }
        let json = serde_json::to_vec(&segment)?;
        self.db
            .open_tree(SEGMENTS_TREE)?
            .insert(key.as_bytes(), json)?;
        Ok(())
    }

    /// Remove a document from its table's segment.
    pub fn remove_document(&self, engine: &str, table: &str, id: &str) -> Result<()> {
        let key = segment_key(engine, table);
        let mut segment = self.get_segment(engine, table).unwrap_or_default();
        segment.remove_doc(id);
        if let Ok(mut segments) = self.segments.write() {
            segments.insert(key.clone(), segment.clone());
        }
        let json = serde_json::to_vec(&segment)?;
        self.db
            .open_tree(SEGMENTS_TREE)?
            .insert(key.as_bytes(), json)?;
        Ok(())
    }

    /// Drop a table's segment entirely (used by `Truncate`/`Drop`).
    pub fn drop_table(&self, engine: &str, table: &str) -> Result<()> {
        let key = segment_key(engine, table);
        if let Ok(mut segments) = self.segments.write() {
            segments.remove(&key);
        }
        self.db.open_tree(SEGMENTS_TREE)?.remove(key.as_bytes())?;
        self.db.open_tree(DIRTY_TREE)?.remove(key.as_bytes())?;
        Ok(())
    }

    /// Run a full-text search over the segments in scope.
    ///
    /// `segments` maps a `segment_key` to its segment. Returns hits with the
    /// engine/table provenance and score.
    pub fn search(
        &self,
        segments: &[(String, SearchSegment)],
        query: &str,
        mode: SearchMode,
    ) -> Vec<(String, SearchSegment, u64, f64)> {
        let mut out = Vec::new();
        for (key, segment) in segments {
            for (doc_id, score) in segment.index.search(query, mode.clone()) {
                out.push((key.clone(), segment.clone(), doc_id, score));
            }
        }
        out
    }

    /// Total number of indexed documents across all segments.
    pub fn document_count(&self) -> usize {
        self.segments
            .read()
            .map(|s| s.values().map(|seg| seg.docs.len()).sum())
            .unwrap_or(0)
    }

    /// Number of cached segments.
    pub fn segment_count(&self) -> usize {
        self.segments.read().map(|s| s.len()).unwrap_or(0)
    }
}
