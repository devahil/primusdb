//! MySQL source implementation for the migration framework.
//!
//! Connects via the `mysql` crate and reads column metadata from
//! `INFORMATION_SCHEMA.COLUMNS`. Enabled by the `mysql-source` feature; a
//! fallback implementation reports [`crate::Error::Unsupported`] when the
//! feature is disabled.

#[allow(unused_imports)]
use super::super::source::{
    MigrationSource, RowStream, SourceColumn, SourceDatabase, SourceObject, SourceSchema,
};
use crate::Result;

/// Source implementation for MySQL databases.
///
/// Enabled via the `mysql-source` feature flag.
#[allow(dead_code)]
pub struct MySqlSource {
    url: String,
    masked_url: String,
}

impl MySqlSource {
    /// Creates a source from a MySQL URL, keeping a masked copy of the URL for
    /// display.
    pub fn new(url: &str) -> Result<Self> {
        let masked = crate::migration::report::MigrationReport::mask_url(url);
        Ok(Self {
            url: url.to_string(),
            masked_url: masked,
        })
    }
}

#[cfg(feature = "mysql-source")]
impl MigrationSource for MySqlSource {
    fn name(&self) -> &str {
        "mysql"
    }

    fn inspect_schema(&self) -> Result<SourceSchema> {
        use mysql::prelude::*;
        use mysql::*;

        let pool = Pool::new(
            Opts::from_url(&self.url)
                .map_err(|e| crate::Error::InvalidRequest(format!("Invalid MySQL URL: {}", e)))?,
        )
        .map_err(|e| crate::Error::NetworkError(format!("MySQL pool: {}", e)))?;

        let mut conn = pool
            .get_conn()
            .map_err(|e| crate::Error::NetworkError(format!("MySQL connect: {}", e)))?;

        type ColRow = (String, String, String, String, Option<u64>, String, String);
        let rows: Vec<ColRow> = conn
            .exec(
                "SELECT TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME, DATA_TYPE, \
                 CHARACTER_MAXIMUM_LENGTH, IS_NULLABLE, COLUMN_KEY \
                 FROM INFORMATION_SCHEMA.COLUMNS \
                 WHERE TABLE_SCHEMA = DATABASE() \
                 ORDER BY TABLE_NAME, ORDINAL_POSITION",
                (),
            )
            .map_err(|e| crate::Error::InvalidRequest(format!("MySQL schema: {}", e)))?;

        let mut db_map: std::collections::BTreeMap<
            String,
            std::collections::BTreeMap<String, Vec<SourceColumn>>,
        > = std::collections::BTreeMap::new();

        for (db_name, table_name, col_name, data_type, max_len, nullable_raw, col_key) in rows {
            let nullable = nullable_raw == "YES";
            let is_pk = col_key == "PRI";
            db_map
                .entry(db_name)
                .or_default()
                .entry(table_name)
                .or_default()
                .push(SourceColumn {
                    name: col_name,
                    data_type,
                    nullable,
                    is_primary_key: is_pk,
                    max_length: max_len,
                });
        }

        let databases = db_map
            .into_iter()
            .map(|(db_name, tables)| SourceDatabase {
                name: db_name,
                objects: tables
                    .into_iter()
                    .map(|(tbl_name, cols)| SourceObject {
                        name: tbl_name,
                        object_type: "table".to_string(),
                        columns: cols.clone(),
                        row_estimate: None,
                        primary_key: cols
                            .iter()
                            .filter(|c| c.is_primary_key)
                            .map(|c| c.name.clone())
                            .collect(),
                    })
                    .collect(),
            })
            .collect();

        Ok(SourceSchema { databases })
    }

    fn stream_rows(&self, object: &SourceObject) -> Result<RowStream> {
        use mysql::prelude::*;
        use mysql::*;

        let pool = Pool::new(
            Opts::from_url(&self.url)
                .map_err(|e| crate::Error::InvalidRequest(format!("Invalid MySQL URL: {}", e)))?,
        )
        .map_err(|e| crate::Error::NetworkError(format!("MySQL pool: {}", e)))?;

        let mut conn = pool
            .get_conn()
            .map_err(|e| crate::Error::NetworkError(format!("MySQL connect: {}", e)))?;

        let query_str = format!("SELECT * FROM `{}`", object.name);
        let result = conn
            .query_iter(query_str)
            .map_err(|e| crate::Error::InvalidRequest(format!("MySQL query: {}", e)))?;

        let columns: Vec<String> = result
            .columns()
            .as_ref()
            .iter()
            .map(|c| c.name_str().to_string())
            .collect();

        let mut rows: Vec<Vec<serde_json::Value>> = Vec::new();
        for row_result in result {
            let row: Row = row_result
                .map_err(|e| crate::Error::InvalidRequest(format!("MySQL row: {}", e)))?;
            let mut values = Vec::with_capacity(columns.len());
            for i in 0..columns.len() {
                let val: Value = row.get(i).unwrap_or(Value::NULL);
                let json_val = match val {
                    Value::NULL => serde_json::Value::Null,
                    Value::Int(v) => serde_json::Value::Number(v.into()),
                    Value::UInt(v) => serde_json::Value::Number(v.into()),
                    Value::Float(v) => serde_json::Number::from_f64(v as f64)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null),
                    Value::Double(v) => serde_json::Number::from_f64(v)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null),
                    Value::Date(y, m, d, h, min, s, _) => serde_json::Value::String(format!(
                        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                        y, m, d, h, min, s
                    )),
                    Value::Time(neg, d, h, m, s, _) => {
                        let sign = if neg { "-" } else { "" };
                        serde_json::Value::String(format!(
                            "{}{} days {:02}:{:02}:{:02}",
                            sign, d, h, m, s
                        ))
                    }
                    Value::Bytes(ref bytes) => {
                        let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
                        serde_json::Value::String(hex)
                    }
                };
                values.push(json_val);
            }
            rows.push(values);
        }

        Ok(RowStream { columns, rows })
    }
}

#[cfg(not(feature = "mysql-source"))]
impl MigrationSource for MySqlSource {
    fn name(&self) -> &str {
        "mysql"
    }

    fn inspect_schema(&self) -> Result<SourceSchema> {
        Err(crate::Error::Unsupported(
            "MySQL migration requires the 'mysql-source' feature: cargo build --features mysql-source"
                .into(),
        ))
    }

    fn stream_rows(&self, _object: &SourceObject) -> Result<RowStream> {
        Err(crate::Error::Unsupported(
            "MySQL migration requires the 'mysql-source' feature".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_masks_url() {
        let src = MySqlSource::new("mysql://user:secret@localhost:3306/mydb").unwrap();
        assert!(src.masked_url.contains("*****"));
        assert!(!src.masked_url.contains("secret"));
    }

    #[test]
    fn test_new_no_credentials() {
        let src = MySqlSource::new("mysql://localhost:3306/mydb").unwrap();
        assert_eq!(src.url, "mysql://localhost:3306/mydb");
    }

    #[test]
    fn test_name() {
        let src = MySqlSource::new("mysql://localhost/mydb").unwrap();
        assert_eq!(src.name(), "mysql");
    }

    #[test]
    fn test_fallback_unsupported() {
        let src = MySqlSource::new("mysql://localhost/mydb").unwrap();
        let result = src.inspect_schema();
        assert!(result.is_err());
    }
}
