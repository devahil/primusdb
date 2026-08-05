//! CouchDB source implementation for the migration framework.
//!
//! Uses CouchDB's REST API via the `reqwest` crate (always available): each
//! database is exposed as a single object with `_id`/`_rev` columns and
//! documents are streamed through `_all_docs?include_docs=true`.

#[allow(unused_imports)]
use super::super::source::{
    MigrationSource, RowStream, SourceColumn, SourceDatabase, SourceObject, SourceSchema,
};
use crate::Result;

/// Source implementation for CouchDB databases.
///
/// Uses CouchDB's REST API via the `reqwest` crate (always available).
#[allow(dead_code)]
pub struct CouchSource {
    url: String,
    masked_url: String,
}

impl CouchSource {
    /// Creates a source from a CouchDB URL, keeping a masked copy of the URL
    /// for display.
    pub fn new(url: &str) -> Result<Self> {
        let masked = crate::migration::report::MigrationReport::mask_url(url);
        Ok(Self {
            url: url.to_string(),
            masked_url: masked,
        })
    }

    fn base_url(&self) -> String {
        self.url.trim_end_matches('/').to_string()
    }
}

impl MigrationSource for CouchSource {
    fn name(&self) -> &str {
        "couchdb"
    }

    fn inspect_schema(&self) -> Result<SourceSchema> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::Error::DatabaseError(format!("Runtime: {}", e)))?;
        rt.block_on(async {
            let client = reqwest::Client::new();
            let base = self.base_url();

            let db_list: Vec<String> = client
                .get(format!("{}/_all_dbs", base))
                .send()
                .await?
                .json()
                .await?;

            let mut databases = Vec::new();

            for db_name in db_list {
                let info: serde_json::Value = client
                    .get(format!("{}/{}", base, db_name))
                    .send()
                    .await?
                    .json()
                    .await?;

                let doc_count = info.get("doc_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let doc_del_count = info
                    .get("doc_del_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let estimated_rows = doc_count.saturating_sub(doc_del_count);

                databases.push(SourceDatabase {
                    name: db_name.clone(),
                    objects: vec![SourceObject {
                        name: db_name,
                        object_type: "database".to_string(),
                        columns: vec![
                            SourceColumn {
                                name: "_id".to_string(),
                                data_type: "string".to_string(),
                                nullable: false,
                                is_primary_key: true,
                                max_length: None,
                            },
                            SourceColumn {
                                name: "_rev".to_string(),
                                data_type: "string".to_string(),
                                nullable: false,
                                is_primary_key: false,
                                max_length: None,
                            },
                        ],
                        row_estimate: Some(estimated_rows),
                        primary_key: vec!["_id".to_string()],
                    }],
                });
            }

            Ok(SourceSchema { databases })
        })
    }

    fn stream_rows(&self, object: &SourceObject) -> Result<RowStream> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::Error::DatabaseError(format!("Runtime: {}", e)))?;
        rt.block_on(async {
            let client = reqwest::Client::new();
            let base = self.base_url();
            let db_name = &object.name;

            let resp: serde_json::Value = client
                .get(format!("{}/{}/_all_docs?include_docs=true", base, db_name))
                .send()
                .await
                .map_err(|e| crate::Error::NetworkError(format!("CouchDB request: {}", e)))?
                .json()
                .await
                .map_err(|e| crate::Error::InvalidRequest(format!("CouchDB JSON: {}", e)))?;

            let rows_arr = resp["rows"].as_array().ok_or_else(|| {
                crate::Error::InvalidRequest("CouchDB _all_docs: missing rows".into())
            })?;

            if rows_arr.is_empty() {
                return Ok(RowStream {
                    columns: vec![],
                    rows: vec![],
                });
            }

            let mut field_set = std::collections::BTreeSet::new();
            let mut docs: Vec<&serde_json::Map<String, serde_json::Value>> = Vec::new();
            for row_val in rows_arr {
                if let Some(doc) = row_val["doc"].as_object() {
                    for key in doc.keys() {
                        if key != "_rev" {
                            field_set.insert(key.clone());
                        }
                    }
                    docs.push(doc);
                }
            }

            let columns: Vec<String> = field_set.into_iter().collect();

            let mut rows = Vec::with_capacity(docs.len());
            for doc in docs {
                let mut values = Vec::with_capacity(columns.len());
                for col in &columns {
                    values.push(doc.get(col).cloned().unwrap_or(serde_json::Value::Null));
                }
                rows.push(values);
            }

            Ok(RowStream { columns, rows })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_masks_url() {
        let src = CouchSource::new("http://user:secret@localhost:5984/mydb").unwrap();
        assert!(src.masked_url.contains("*****"));
        assert!(!src.masked_url.contains("secret"));
    }

    #[test]
    fn test_new_no_credentials() {
        let src = CouchSource::new("http://localhost:5984").unwrap();
        assert_eq!(src.url, "http://localhost:5984");
    }

    #[test]
    fn test_name() {
        let src = CouchSource::new("http://localhost:5984").unwrap();
        assert_eq!(src.name(), "couchdb");
    }

    #[test]
    fn test_base_url_strips_trailing_slash() {
        let src = CouchSource::new("http://localhost:5984/").unwrap();
        assert_eq!(src.base_url(), "http://localhost:5984");
    }

    #[test]
    fn test_base_url_no_trailing_slash() {
        let src = CouchSource::new("http://localhost:5984").unwrap();
        assert_eq!(src.base_url(), "http://localhost:5984");
    }
}
