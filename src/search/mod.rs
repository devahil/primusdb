/*
 * PrimusDB Unified Search Service
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.0.0
 */

/*!
# Unified Search — route one query across every storage engine

The unified search service lets a single call search across tables/collections
in *all* storage engines (relational, document, key-value, columnar, vector,
time-series) and returns merged, provenance-tagged hits.

Two query kinds are supported:

- **Full-text** (`query`): an inverted-index (TF-IDF) scan of the string
  values of every record in scope. Each table/collection is scored with its
  own [`FullTextIndex`]; merged results are ranked by descending score.
- **Vector** (`query_vector`): routed to the vector engine, which runs a
  cosine-similarity scan and reports the similarity in each hit.

## Routing

Tables are discovered through each engine's [`StorageEngine::list_tables`]
(capability registry pattern — new engines appear automatically once they
implement enumeration). The caller can restrict the scope with
`storage_types`, `tables`, and per-engine offsets/limits.

```text
SearchRequest
   ├─ query  ──────► FullTextIndex per table (all engines)
   └─ query_vector ─► VectorEngine cosine similarity (vector engine only)
                        ▼
                  merged hits (score/similarity, provenance)
```

## Honest limitations

- Full-text search is backed by a **persistent inverted index** ([`index`]) when
  enabled (the default): `Create` mutations index incrementally, while
  `Update`/`Delete`/`Truncate` mark segments dirty and rebuild them lazily on
  the next search. Searches fall back to live scans when the index is disabled
  or unavailable.
- TF-IDF scores are computed per table, so raw scores are only comparable
  within a table; merged ordering is a best-effort union of the per-table
  rankings.
- Vector search requires the vector engine to be in scope.
- The index caches a record snapshot per document; writes that bypass
  `execute_query` are covered when the table's segment is next marked dirty or
  rebuilt.
*/

use crate::{fulltext::FullTextIndex, PrimusDB, Result, StorageType};
use serde::{Deserialize, Serialize};

/// Persistent, sled-backed inverted index for unified full-text search.
pub mod index;

/// Re-exported for REST/CLI consumers of unified search.
pub use crate::fulltext::{FullTextIndex as SearchFullTextIndex, SearchMode};
pub use index::{IndexedDoc, PersistentSearchIndex, SearchSegment};

/// Configuration for the unified search service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Persist an inverted index (sled) under the data directory and maintain
    /// it on committed mutations. Defaults to `true`.
    pub persistent_index: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            persistent_index: true,
        }
    }
}

/// A unified search request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Full-text query tokenized into terms (all engines).
    pub query: Option<String>,
    /// Vector query (JSON array of numbers) routed to the vector engine.
    pub query_vector: Option<serde_json::Value>,
    /// Token matching mode for full-text search (defaults to `And`).
    pub mode: Option<SearchMode>,
    /// Restrict the engines searched; defaults to all six.
    pub storage_types: Option<Vec<StorageType>>,
    /// Restrict the tables/collections searched by name.
    pub tables: Option<Vec<String>>,
    /// Maximum number of merged hits to return (default 20).
    pub limit: Option<u64>,
    /// Number of merged hits to skip (default 0).
    pub offset: Option<u64>,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: None,
            query_vector: None,
            mode: None,
            storage_types: None,
            tables: None,
            limit: Some(20),
            offset: Some(0),
        }
    }
}

/// A single search hit with provenance (engine, table, record id) and score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    /// Engine that produced the hit.
    pub engine: String,
    /// Table/collection/database the record lives in.
    pub table: String,
    /// Record id within the table.
    pub id: String,
    /// Ranking score: TF-IDF for full-text hits, cosine similarity for
    /// vector hits.
    pub score: f64,
    /// Cosine similarity for vector hits (None for full-text hits).
    pub similarity: Option<f64>,
    /// The matched record.
    pub record: serde_json::Value,
}

/// The result of a unified search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Echo of the query text (empty for pure vector searches).
    pub query: String,
    /// Ordered, offset/limit-applied hits.
    pub hits: Vec<SearchHit>,
    /// Total number of hits before offset/limit.
    pub total: u64,
    /// Names of the engines actually searched.
    pub engines_searched: Vec<String>,
}

/// All storage engines known to the unified search router.
pub const ALL_ENGINES: [StorageType; 6] = [
    StorageType::Relational,
    StorageType::Document,
    StorageType::KeyValue,
    StorageType::Columnar,
    StorageType::Vector,
    StorageType::TimeSeries,
];

/// The unified search service. Stateless: it routes live scans over the
/// engines registered in the provided [`PrimusDB`] instance.
pub struct SearchService;

impl SearchService {
    /// Run a unified search across the engines in scope.
    ///
    /// # Errors
    /// Returns an error when a requested storage type is not registered or an
    /// engine scan fails.
    pub async fn search(db: &PrimusDB, request: &SearchRequest) -> Result<SearchResponse> {
        let mode = request.mode.clone().unwrap_or(SearchMode::And);
        let query_text = request.query.clone().unwrap_or_default().trim().to_string();
        let limit = request.limit.unwrap_or(20);
        let offset = request.offset.unwrap_or(0);
        let storage_types: Vec<StorageType> = request
            .storage_types
            .clone()
            .unwrap_or_else(|| ALL_ENGINES.to_vec());
        let table_filter: Option<Vec<String>> = request.tables.clone();

        let tx = crate::transaction::Transaction {
            id: format!("search-{}", uuid::Uuid::new_v4()),
            operations: vec![],
            status: crate::transaction::TransactionStatus::Prepared,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
            timeout_ms: 0,
        };

        let mut hits: Vec<SearchHit> = Vec::new();
        let mut engines_searched: Vec<String> = Vec::new();

        for st in &storage_types {
            let engine = match db.storage_engine(*st) {
                Some(e) => e,
                None => continue,
            };
            let mut tables = engine.list_tables()?;
            if let Some(filter) = &table_filter {
                tables.retain(|t| filter.contains(t));
            }
            if tables.is_empty() {
                continue;
            }
            engines_searched.push(st.to_string().to_lowercase());

            // Vector routing: cosine similarity against vector collections.
            if st == &StorageType::Vector {
                if let Some(query_vector) = &request.query_vector {
                    for table in &tables {
                        let records = engine
                            .select(
                                table,
                                Some(&serde_json::json!({ "query_vector": query_vector })),
                                u64::MAX,
                                0,
                                &tx,
                            )
                            .await?;
                        for record in records {
                            let similarity = record
                                .metadata
                                .get("similarity")
                                .and_then(|s| s.parse::<f64>().ok());
                            let score = similarity.unwrap_or(0.0);
                            hits.push(SearchHit {
                                engine: st.to_string().to_lowercase(),
                                table: table.clone(),
                                id: record.id.clone(),
                                score,
                                similarity,
                                record: record.data,
                            });
                        }
                    }
                }
                continue;
            }

            // Full-text routing: per-table TF-IDF over the persistent index
            // (with lazy rebuild of dirty segments) or a live scan fallback.
            if !query_text.is_empty() {
                for table in &tables {
                    let hits_for_table =
                        SearchService::fulltext_table(db, *st, table, &query_text, &mode, &tx)
                            .await?;
                    hits.extend(hits_for_table);
                }
            }
        }

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let total = hits.len() as u64;
        let hits = hits
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        Ok(SearchResponse {
            query: query_text,
            hits,
            total,
            engines_searched,
        })
    }

    /// Full-text search a single table, using the persistent index when it is
    /// available and rebuilding dirty segments on demand.
    async fn fulltext_table(
        db: &PrimusDB,
        st: StorageType,
        table: &str,
        query_text: &str,
        mode: &SearchMode,
        tx: &crate::transaction::Transaction,
    ) -> Result<Vec<SearchHit>> {
        // Prefer the persistent index.
        if let Some(index) = db.search_index() {
            let engine = st.to_string().to_lowercase();
            if index.is_dirty(&engine, table) || index.get_segment(&engine, table).is_none() {
                // Rebuild this table's segment from live data.
                let segment = rebuild_segment(db, st, table, tx).await?;
                index.set_segment(&engine, table, segment);
            }
            if let Some(segment) = index.get_segment(&engine, table) {
                let mut hits = Vec::new();
                for (doc_id, score) in segment.index.search(query_text, mode.clone()) {
                    let doc = &segment.docs[doc_id as usize];
                    hits.push(SearchHit {
                        engine: engine.clone(),
                        table: table.to_string(),
                        id: doc.id.clone(),
                        score,
                        similarity: None,
                        record: doc.record.clone(),
                    });
                }
                return Ok(hits);
            }
            // Fall through to a live scan if the segment is unavailable.
        }

        // Live-scan fallback.
        let engine = db.storage_engine(st);
        let engine = match engine {
            Some(e) => e,
            None => return Ok(Vec::new()),
        };
        let records = engine.select(table, None, u64::MAX, 0, tx).await?;
        if records.is_empty() {
            return Ok(Vec::new());
        }
        let mut index = FullTextIndex::new();
        let mut by_id: Vec<(String, serde_json::Value)> = Vec::with_capacity(records.len());
        for (i, record) in records.into_iter().enumerate() {
            let text = collect_searchable_text(&record.data);
            index.index_document(i as u64, &text);
            by_id.push((record.id, record.data));
        }
        let mut hits = Vec::new();
        for (doc_id, score) in index.search(query_text, mode.clone()) {
            let (id, data) = &by_id[doc_id as usize];
            hits.push(SearchHit {
                engine: st.to_string().to_lowercase(),
                table: table.to_string(),
                id: id.clone(),
                score,
                similarity: None,
                record: data.clone(),
            });
        }
        Ok(hits)
    }
}

/// Scan a table's live records into a search segment.
async fn rebuild_segment(
    db: &PrimusDB,
    st: StorageType,
    table: &str,
    tx: &crate::transaction::Transaction,
) -> Result<index::SearchSegment> {
    let engine = db
        .storage_engine(st)
        .ok_or_else(|| crate::Error::ValidationError(format!("engine {st} unavailable")))?;
    let records = engine.select(table, None, u64::MAX, 0, tx).await?;
    let mut segment = index::SearchSegment::new();
    for record in records {
        let text = collect_searchable_text(&record.data);
        segment.push_doc(record.id, record.data, &text);
    }
    Ok(segment)
}

/// Concatenate every string and scalar number leaf in a JSON value into a
/// single searchable text blob (nested objects/arrays are traversed).
pub fn collect_searchable_text(value: &serde_json::Value) -> String {
    fn collect(value: &serde_json::Value, out: &mut String) {
        match value {
            serde_json::Value::String(s) => {
                out.push(' ');
                out.push_str(s);
            }
            serde_json::Value::Number(n) => {
                out.push(' ');
                out.push_str(&n.to_string());
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    collect(item, out);
                }
            }
            serde_json::Value::Object(map) => {
                for item in map.values() {
                    collect(item, out);
                }
            }
            _ => {}
        }
    }
    let mut out = String::new();
    collect(value, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PrimusDBConfig, Query, QueryOperation};

    fn setup() -> (PrimusDB, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_string_lossy().into_owned();
        config.integrity.genesis_required = false;
        (PrimusDB::new(config).unwrap(), dir)
    }

    async fn insert(
        db: &PrimusDB,
        st: StorageType,
        table: &str,
        data: serde_json::Value,
    ) -> Result<()> {
        db.execute_query(Query {
            storage_type: st,
            operation: QueryOperation::Create,
            table: table.to_string(),
            conditions: None,
            data: Some(data),
            limit: None,
            offset: None,
            namespace: None,
        })
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_fulltext_search_across_engines() -> Result<()> {
        let (db, _dir) = setup();

        db.storage_engine(StorageType::KeyValue)
            .unwrap()
            .create_table(
                "kvdb",
                &crate::storage::Schema {
                    fields: vec![],
                    indexes: vec![],
                    constraints: vec![],
                },
            )
            .await?;
        insert(
            &db,
            StorageType::Document,
            "notes",
            serde_json::json!({"title": "quick brown fox", "body": "jumps over the lazy dog"}),
        )
        .await?;
        insert(
            &db,
            StorageType::KeyValue,
            "kvdb",
            serde_json::json!({"name": "quick sort", "tags": ["algorithm", "fast"]}),
        )
        .await?;
        insert(
            &db,
            StorageType::Document,
            "notes",
            serde_json::json!({"title": "gardening", "body": "plant tomatoes in spring"}),
        )
        .await?;

        let resp = SearchService::search(
            &db,
            &SearchRequest {
                query: Some("quick fox".to_string()),
                ..Default::default()
            },
        )
        .await?;

        assert_eq!(resp.total, 2);
        let tables: Vec<&str> = resp.hits.iter().map(|h| h.table.as_str()).collect();
        assert!(tables.contains(&"notes"));
        assert!(tables.contains(&"kvdb"));
        Ok(())
    }

    #[tokio::test]
    async fn test_vector_routing() -> Result<()> {
        let (db, _dir) = setup();

        insert(
            &db,
            StorageType::Vector,
            "emb",
            serde_json::json!({"id": "a", "vector": [1.0, 0.0, 0.0]}),
        )
        .await?;
        insert(
            &db,
            StorageType::Vector,
            "emb",
            serde_json::json!({"id": "b", "vector": [0.0, 1.0, 0.0]}),
        )
        .await?;

        let resp = SearchService::search(
            &db,
            &SearchRequest {
                query_vector: Some(serde_json::json!([1.0, 0.0, 0.0])),
                storage_types: Some(vec![StorageType::Vector]),
                ..Default::default()
            },
        )
        .await?;

        assert_eq!(resp.total, 2);
        assert_eq!(resp.hits[0].table, "emb");
        assert_eq!(resp.hits[0].similarity.unwrap_or(0.0), 1.0);
        Ok(())
    }

    #[test]
    fn test_collect_searchable_text() {
        let v = serde_json::json!({"a": "hello", "b": 42, "c": {"d": "world"}, "e": [1, "x"]});
        let text = collect_searchable_text(&v);
        assert!(text.contains("hello"));
        assert!(text.contains("42"));
        assert!(text.contains("world"));
        assert!(text.contains("x"));
    }
}
