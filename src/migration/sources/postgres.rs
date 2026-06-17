#[allow(unused_imports)]
use super::super::source::{
    MigrationSource, RowStream, SourceColumn, SourceDatabase, SourceObject, SourceSchema,
};
use crate::Result;

/// Source implementation for PostgreSQL databases.
///
/// Enabled via the `postgres-source` feature flag.
#[allow(dead_code)]
pub struct PostgresSource {
    url: String,
    masked_url: String,
}

impl PostgresSource {
    pub fn new(url: &str) -> Result<Self> {
        let masked = crate::migration::report::MigrationReport::mask_url(url);
        Ok(Self {
            url: url.to_string(),
            masked_url: masked,
        })
    }
}

#[cfg(feature = "postgres-source")]
impl MigrationSource for PostgresSource {
    fn name(&self) -> &str {
        "postgres"
    }

    fn inspect_schema(&self) -> Result<SourceSchema> {
        use tokio_postgres::{connect, NoTls};

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::Error::DatabaseError(format!("Runtime: {}", e)))?;
        rt.block_on(async {
            let (client, connection) = connect(&self.url, NoTls)
                .await
                .map_err(|e| crate::Error::NetworkError(format!("Postgres connect: {}", e)))?;

            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    tracing::warn!("Postgres connection error: {}", e);
                }
            });

            let rows = client
                .query(
                    "SELECT table_catalog, table_name, column_name, data_type, \
                     character_maximum_length::bigint, is_nullable = 'YES', \
                     EXISTS(SELECT 1 FROM information_schema.table_constraints tc \
                            JOIN information_schema.key_column_usage kcu \
                            USING (constraint_catalog, constraint_schema, constraint_name) \
                            WHERE tc.constraint_type = 'PRIMARY KEY' \
                            AND kcu.table_name = information_schema.columns.table_name \
                            AND kcu.table_schema = information_schema.columns.table_schema \
                            AND kcu.column_name = information_schema.columns.column_name) \
                     FROM information_schema.columns \
                     WHERE table_schema = 'public' \
                     ORDER BY table_name, ordinal_position",
                    &[],
                )
                .await
                .map_err(|e| crate::Error::InvalidRequest(format!("Postgres schema: {}", e)))?;

            let mut db_map: std::collections::BTreeMap<
                String,
                std::collections::BTreeMap<String, Vec<SourceColumn>>,
            > = std::collections::BTreeMap::new();

            for row in rows {
                let db_name: String = row.get(0);
                let table_name: String = row.get(1);
                let col = SourceColumn {
                    name: row.get(2),
                    data_type: row.get(3),
                    nullable: row.get(5),
                    is_primary_key: row.get(6),
                    max_length: row.get::<_, Option<i64>>(4).map(|v| v as u64),
                };
                db_map
                    .entry(db_name)
                    .or_default()
                    .entry(table_name)
                    .or_default()
                    .push(col);
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
        })
    }

    fn stream_rows(&self, object: &SourceObject) -> Result<RowStream> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::Error::DatabaseError(format!("Runtime: {}", e)))?;
        rt.block_on(async {
            use tokio_postgres::{connect, NoTls};

            let (client, connection) = connect(&self.url, NoTls)
                .await
                .map_err(|e| crate::Error::NetworkError(format!("Postgres connect: {}", e)))?;

            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    tracing::warn!("Postgres connection error: {}", e);
                }
            });

            let query_str = format!("SELECT * FROM \"{}\"", object.name);
            let rows = client
                .query(&query_str, &[])
                .await
                .map_err(|e| crate::Error::InvalidRequest(format!("Postgres query: {}", e)))?;

            let columns: Vec<String> = if rows.is_empty() {
                object.columns.iter().map(|c| c.name.clone()).collect()
            } else {
                rows[0]
                    .columns()
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect()
            };

            let mut result_rows: Vec<Vec<serde_json::Value>> = Vec::with_capacity(rows.len());
            for row in &rows {
                let mut values = Vec::with_capacity(columns.len());
                for i in 0..columns.len() {
                    values.push(pg_value_to_json(row, i));
                }
                result_rows.push(values);
            }

            Ok(RowStream {
                columns,
                rows: result_rows,
            })
        })
    }
}

#[cfg(feature = "postgres-source")]
fn pg_value_to_json(row: &tokio_postgres::Row, i: usize) -> serde_json::Value {
    use tokio_postgres::types::Type;

    match *row.columns()[i].type_() {
        Type::BOOL => row
            .try_get::<_, bool>(i)
            .map(serde_json::Value::Bool)
            .unwrap_or(serde_json::Value::Null),
        Type::INT2 | Type::INT4 => row
            .try_get::<_, i32>(i)
            .map(|v| serde_json::Value::Number(v.into()))
            .unwrap_or(serde_json::Value::Null),
        Type::INT8 => row
            .try_get::<_, i64>(i)
            .map(|v| serde_json::Value::Number(v.into()))
            .unwrap_or(serde_json::Value::Null),
        Type::FLOAT4 => row
            .try_get::<_, f32>(i)
            .map(|v| {
                serde_json::Number::from_f64(v as f64)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            })
            .unwrap_or(serde_json::Value::Null),
        Type::FLOAT8 => row
            .try_get::<_, f64>(i)
            .map(|v| {
                serde_json::Number::from_f64(v)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            })
            .unwrap_or(serde_json::Value::Null),
        Type::NUMERIC => row
            .try_get::<_, String>(i)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        Type::JSON | Type::JSONB => row
            .try_get::<_, serde_json::Value>(i)
            .unwrap_or(serde_json::Value::Null),
        _ => row
            .try_get::<_, String>(i)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    }
}

#[cfg(not(feature = "postgres-source"))]
impl MigrationSource for PostgresSource {
    fn name(&self) -> &str {
        "postgres"
    }

    fn inspect_schema(&self) -> Result<SourceSchema> {
        Err(crate::Error::Unsupported(
            "PostgreSQL migration requires the 'postgres-source' feature: cargo build --features postgres-source"
                .into(),
        ))
    }

    fn stream_rows(&self, _object: &SourceObject) -> Result<RowStream> {
        Err(crate::Error::Unsupported(
            "PostgreSQL migration requires the 'postgres-source' feature".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_masks_url() {
        let src = PostgresSource::new("postgres://user:secret@localhost:5432/mydb").unwrap();
        assert!(src.masked_url.contains("*****"));
        assert!(!src.masked_url.contains("secret"));
    }

    #[test]
    fn test_new_no_credentials() {
        let src = PostgresSource::new("postgres://localhost:5432/mydb").unwrap();
        assert_eq!(src.url, "postgres://localhost:5432/mydb");
    }

    #[test]
    fn test_name() {
        let src = PostgresSource::new("postgres://localhost/mydb").unwrap();
        assert_eq!(src.name(), "postgres");
    }

    #[test]
    fn test_fallback_unsupported() {
        let src = PostgresSource::new("postgres://localhost/mydb").unwrap();
        let result = src.inspect_schema();
        assert!(result.is_err());
    }
}
