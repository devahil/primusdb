//! MongoDB source implementation for the migration framework.
//!
//! Connects via the `mongodb` crate, listing databases and collections and
//! streaming documents as rows with a unified field set. Enabled by the
//! `mongo-source` feature; a fallback implementation reports
//! [`crate::Error::Unsupported`] when the feature is disabled.

#[allow(unused_imports)]
use super::super::source::{
    MigrationSource, RowStream, SourceColumn, SourceDatabase, SourceObject, SourceSchema,
};
use crate::Result;

/// Source implementation for MongoDB databases.
///
/// Enabled via the `mongo-source` feature flag.
#[allow(dead_code)]
pub struct MongoSource {
    url: String,
    masked_url: String,
}

impl MongoSource {
    /// Creates a source from a MongoDB URL, keeping a masked copy of the URL
    /// for display.
    pub fn new(url: &str) -> Result<Self> {
        let masked = crate::migration::report::MigrationReport::mask_url(url);
        Ok(Self {
            url: url.to_string(),
            masked_url: masked,
        })
    }
}

#[cfg(feature = "mongo-source")]
impl MigrationSource for MongoSource {
    fn name(&self) -> &str {
        "mongodb"
    }

    fn inspect_schema(&self) -> Result<SourceSchema> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::Error::DatabaseError(format!("Runtime: {}", e)))?;
        rt.block_on(async {
            let client = mongodb::Client::with_uri_str(&self.url)
                .await
                .map_err(|e| crate::Error::NetworkError(format!("MongoDB connect: {}", e)))?;

            let db_names = client
                .list_database_names()
                .await
                .map_err(|e| crate::Error::InvalidRequest(format!("MongoDB list DBs: {}", e)))?;

            let mut databases = Vec::new();

            for db_name in db_names {
                let db = client.database(&db_name);
                let collection_names = db.list_collection_names().await.map_err(|e| {
                    crate::Error::InvalidRequest(format!("MongoDB list collections: {}", e))
                })?;

                let objects: Vec<SourceObject> = collection_names
                    .into_iter()
                    .map(|coll_name| SourceObject {
                        name: coll_name,
                        object_type: "collection".to_string(),
                        columns: vec![],
                        row_estimate: None,
                        primary_key: vec![],
                    })
                    .collect();

                databases.push(SourceDatabase {
                    name: db_name,
                    objects,
                });
            }

            Ok(SourceSchema { databases })
        })
    }

    fn stream_rows(&self, object: &SourceObject) -> Result<RowStream> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::Error::DatabaseError(format!("Runtime: {}", e)))?;
        rt.block_on(async {
            let client = mongodb::Client::with_uri_str(&self.url)
                .await
                .map_err(|e| crate::Error::NetworkError(format!("MongoDB connect: {}", e)))?;

            let db_name = mongo_db_name(&self.url).unwrap_or("test");
            let db = client.database(db_name);
            let collection = db.collection::<mongodb::bson::Document>(&object.name);

            let mut cursor = collection
                .find(mongodb::bson::doc! {})
                .await
                .map_err(|e| crate::Error::InvalidRequest(format!("MongoDB find: {}", e)))?;

            let mut docs: Vec<mongodb::bson::Document> = Vec::new();
            while cursor.advance().await.map_err(|e| {
                crate::Error::InvalidRequest(format!("MongoDB cursor advance: {}", e))
            })? {
                let raw = cursor.current();
                let doc: mongodb::bson::Document = mongodb::bson::from_slice(raw.as_bytes())
                    .map_err(|e| {
                        crate::Error::InvalidRequest(format!("MongoDB deserialize: {}", e))
                    })?;
                docs.push(doc);
            }

            if docs.is_empty() {
                return Ok(RowStream {
                    columns: vec![],
                    rows: vec![],
                });
            }

            let mut field_set = std::collections::BTreeSet::new();
            for doc in &docs {
                for key in doc.keys() {
                    field_set.insert(key.clone());
                }
            }
            let columns: Vec<String> = field_set.into_iter().collect();

            let mut rows = Vec::with_capacity(docs.len());
            for doc in &docs {
                let mut values = Vec::with_capacity(columns.len());
                for col in &columns {
                    let val = doc
                        .get(col)
                        .map(bson_to_json)
                        .unwrap_or(serde_json::Value::Null);
                    values.push(val);
                }
                rows.push(values);
            }

            Ok(RowStream { columns, rows })
        })
    }
}

#[cfg(feature = "mongo-source")]
fn mongo_db_name(url: &str) -> Option<&str> {
    let after_scheme = url.find("://")?;
    let path = &url[after_scheme + 3..];
    let slash = path.find('/')?;
    let rest = &path[slash + 1..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let db = &rest[..end];
    if db.is_empty() {
        None
    } else {
        Some(db)
    }
}

#[cfg(feature = "mongo-source")]
fn bson_to_json(value: &mongodb::bson::Bson) -> serde_json::Value {
    match value {
        mongodb::bson::Bson::Double(v) => serde_json::Number::from_f64(*v)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        mongodb::bson::Bson::String(v) => serde_json::Value::String(v.clone()),
        mongodb::bson::Bson::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(bson_to_json).collect())
        }
        mongodb::bson::Bson::Document(doc) => {
            let mut map = serde_json::Map::new();
            for (k, v) in doc {
                map.insert(k.clone(), bson_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        mongodb::bson::Bson::Boolean(v) => serde_json::Value::Bool(*v),
        mongodb::bson::Bson::Null | mongodb::bson::Bson::Undefined => serde_json::Value::Null,
        mongodb::bson::Bson::Int32(v) => serde_json::Value::Number((*v).into()),
        mongodb::bson::Bson::Int64(v) => serde_json::Value::Number((*v).into()),
        mongodb::bson::Bson::DateTime(dt) => {
            serde_json::Value::String(dt.try_to_rfc3339_string().unwrap_or_default())
        }
        _ => serde_json::Value::String(format!("{:?}", value)),
    }
}

#[cfg(not(feature = "mongo-source"))]
impl MigrationSource for MongoSource {
    fn name(&self) -> &str {
        "mongodb"
    }

    fn inspect_schema(&self) -> Result<SourceSchema> {
        Err(crate::Error::Unsupported(
            "MongoDB migration requires the 'mongo-source' feature: cargo build --features mongo-source"
                .into(),
        ))
    }

    fn stream_rows(&self, _object: &SourceObject) -> Result<RowStream> {
        Err(crate::Error::Unsupported(
            "MongoDB migration requires the 'mongo-source' feature".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_masks_url() {
        let src = MongoSource::new("mongodb://user:secret@localhost:27017/mydb").unwrap();
        assert!(src.masked_url.contains("*****"));
        assert!(!src.masked_url.contains("secret"));
    }

    #[test]
    fn test_new_no_credentials() {
        let src = MongoSource::new("mongodb://localhost:27017/mydb").unwrap();
        assert_eq!(src.url, "mongodb://localhost:27017/mydb");
    }

    #[test]
    fn test_name() {
        let src = MongoSource::new("mongodb://localhost/mydb").unwrap();
        assert_eq!(src.name(), "mongodb");
    }

    #[test]
    fn test_fallback_unsupported() {
        let src = MongoSource::new("mongodb://localhost/mydb").unwrap();
        let result = src.inspect_schema();
        assert!(result.is_err());
    }

    #[cfg(feature = "mongo-source")]
    #[test]
    fn test_mongo_db_name_simple() {
        assert_eq!(mongo_db_name("mongodb://localhost/mydb"), Some("mydb"));
    }

    #[cfg(feature = "mongo-source")]
    #[test]
    fn test_mongo_db_name_with_auth() {
        assert_eq!(
            mongo_db_name("mongodb://user:pass@localhost:27017/admin"),
            Some("admin")
        );
    }

    #[cfg(feature = "mongo-source")]
    #[test]
    fn test_mongo_db_name_no_path() {
        assert_eq!(mongo_db_name("mongodb://localhost:27017"), None);
    }

    #[cfg(feature = "mongo-source")]
    #[test]
    fn test_mongo_db_name_with_query() {
        assert_eq!(
            mongo_db_name("mongodb://localhost/mydb?ssl=true"),
            Some("mydb")
        );
    }
}
