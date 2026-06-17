//! Legacy SQL parser — **DEPRECATED**
//!
//! This module is deprecated. Use `crate::query::UqlEngine` and the UQL parser
//! (`crate::query::parser`) instead.

use crate::{Query, QueryOperation, Result, StorageType};
use regex::Regex;

/// Parse a SQL string into a Query struct
#[deprecated(
    since = "1.2.0",
    note = "Use PrimusDB::uql_execute_query() or crate::query::UqlEngine instead"
)]
pub fn parse_sql(sql: &str) -> Result<Query> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Err(crate::Error::InvalidRequest("Empty SQL statement".into()));
    }

    let upper = sql.to_uppercase();

    if upper.starts_with("SELECT") {
        parse_select(sql)
    } else if upper.starts_with("INSERT") {
        parse_insert(sql)
    } else if upper.starts_with("UPDATE") {
        parse_update(sql)
    } else if upper.starts_with("DELETE") {
        parse_delete(sql)
    } else if upper.starts_with("CREATE TABLE") || upper.starts_with("CREATE TEMPORARY TABLE") {
        parse_create_table(sql)
    } else if upper.starts_with("ALTER TABLE") {
        parse_alter_table(sql)
    } else if upper.starts_with("DROP TABLE") {
        Ok(Query {
            storage_type: StorageType::Relational,
            operation: QueryOperation::Delete,
            table: extract_table_name(sql, "DROP TABLE").unwrap_or_default(),
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        })
    } else if upper.starts_with("TRUNCATE") {
        parse_truncate(sql)
    } else if upper.starts_with("CREATE SEQUENCE") {
        parse_create_sequence(sql)
    } else if upper.starts_with("DROP SEQUENCE") {
        let name = extract_table_name(sql, "DROP SEQUENCE").unwrap_or_default();
        Ok(Query {
            storage_type: StorageType::Relational,
            operation: QueryOperation::DropSequence,
            table: name,
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        })
    } else if upper.starts_with("CREATE VIEW") {
        parse_create_view(sql)
    } else if upper.starts_with("DROP VIEW") {
        let name = extract_table_name(sql, "DROP VIEW").unwrap_or_default();
        Ok(Query {
            storage_type: StorageType::Relational,
            operation: QueryOperation::DropView,
            table: name,
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        })
    } else if upper.starts_with("CREATE TRIGGER") {
        parse_create_trigger(sql)
    } else if upper.starts_with("DROP TRIGGER") {
        parse_drop_trigger(sql)
    } else if upper.starts_with("BEGIN")
        || upper.starts_with("COMMIT")
        || upper.starts_with("ROLLBACK")
    {
        Ok(Query {
            storage_type: StorageType::Relational,
            operation: QueryOperation::Read,
            table: String::new(),
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        })
    } else {
        Err(crate::Error::InvalidRequest(format!(
            "Unsupported SQL statement: {}",
            sql
        )))
    }
}

fn extract_table_name(sql: &str, prefix: &str) -> Option<String> {
    let re = Regex::new(&format!(r"(?i)^{}\s+([^\s;(]+)", regex::escape(prefix))).ok()?;
    re.captures(sql)
        .and_then(|c| c.get(1).map(|m| m.as_str().trim_matches('"').to_string()))
}

fn extract_value(sql: &str, after: &str) -> Option<String> {
    let pattern = format!(r"(?i){}\s+([^\s,;)]+)", regex::escape(after));
    let re = Regex::new(&pattern).ok()?;
    re.captures(sql)
        .and_then(|c| c.get(1).map(|m| m.as_str().trim_matches('\'').to_string()))
}

fn parse_select(sql: &str) -> Result<Query> {
    let re = Regex::new(r"(?i)^SELECT\s+(.+?)\s+FROM\s+([^\s]+)(?:\s+WHERE\s+(.+?))?(?:\s+GROUP\s+BY\s+(.+?))?(?:\s+HAVING\s+(.+?))?(?:\s+ORDER\s+BY\s+(.+?))?(?:\s+LIMIT\s+(\d+))?(?:\s+OFFSET\s+(\d+))?$").ok();

    if let Some(re) = re {
        if let Some(caps) = re.captures(sql.trim_end_matches(';')) {
            let table = caps
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let conditions = caps.get(3).map(|m| {
                let cond_str = m.as_str().trim();
                if cond_str.contains("AND") {
                    let parts: Vec<&str> = cond_str.splitn(2, "(?i)\\s+AND\\s+").collect();
                    serde_json::json!({
                        "op": "and",
                        "left": parse_simple_condition(parts[0]),
                        "right": if parts.len() > 1 { parse_simple_condition(parts[1]) } else { serde_json::Value::Null }
                    })
                } else if cond_str.contains("OR") {
                    serde_json::json!({"op": "or"})
                } else {
                    parse_simple_condition(cond_str)
                }
            });

            let fields_str = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("*");
            let fields = if fields_str == "*" {
                None
            } else {
                Some(
                    fields_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect::<Vec<_>>(),
                )
            };

            let limit = caps.get(7).and_then(|m| m.as_str().parse::<u64>().ok());
            let offset = caps.get(8).and_then(|m| m.as_str().parse::<u64>().ok());

            return Ok(Query {
                storage_type: StorageType::Relational,
                operation: QueryOperation::Read,
                table,
                conditions,
                data: fields.map(|f| serde_json::json!(f)),
                limit,
                offset,
                namespace: None,
            });
        }
    }
    Err(crate::Error::InvalidRequest(
        "Invalid SELECT statement".into(),
    ))
}

fn parse_simple_condition(s: &str) -> serde_json::Value {
    let s = s.trim();
    let re_eq = Regex::new(r"^([^\s=!<>]+)\s*=\s*(.+)$").ok();
    if let Some(re) = re_eq {
        if let Some(caps) = re.captures(s) {
            let field = caps
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let val = caps
                .get(2)
                .map(|m| m.as_str().trim().trim_matches('\'').to_string())
                .unwrap_or_default();
            return serde_json::json!({"op": "eq", "field": field, "value": val});
        }
    }
    let re_gt = Regex::new(r"^([^\s=!<>]+)\s*>\s*(.+)$").ok();
    if let Some(re) = re_gt {
        if let Some(caps) = re.captures(s) {
            let field = caps
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let val = caps
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            return serde_json::json!({"op": "gt", "field": field, "value": val.parse::<f64>().unwrap_or(0.0)});
        }
    }
    let re_lt = Regex::new(r"^([^\s=!<>]+)\s*<\s*(.+)$").ok();
    if let Some(re) = re_lt {
        if let Some(caps) = re.captures(s) {
            let field = caps
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let val = caps
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            return serde_json::json!({"op": "lt", "field": field, "value": val.parse::<f64>().unwrap_or(0.0)});
        }
    }
    serde_json::json!({"op": "eq", "field": s, "value": true})
}

fn parse_insert(sql: &str) -> Result<Query> {
    let sql = sql.trim_end_matches(';');
    let re = Regex::new(r"(?i)^INSERT\s+INTO\s+([^\s(]+)(?:\s*\(([^)]*)\))?\s*(?:VALUES\s*\(([^)]*)\))?(?:\s+RETURNING\s+(.+))?$").ok();
    if let Some(re) = re {
        if let Some(caps) = re.captures(sql) {
            let table = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let columns_str = caps.get(2).map(|m| m.as_str().trim());
            let values_str = caps.get(3).map(|m| m.as_str().trim());
            let returning = caps.get(4).map(|m| m.as_str().trim().to_string());

            let mut data = serde_json::Map::new();
            if let (Some(cols), Some(vals)) = (columns_str, values_str) {
                let col_names: Vec<&str> = cols
                    .split(',')
                    .map(|s| s.trim().trim_matches('"'))
                    .collect();
                let val_items: Vec<&str> = vals
                    .split(',')
                    .map(|s| s.trim().trim_matches('\''))
                    .collect();
                for (i, col) in col_names.iter().enumerate() {
                    let val = val_items.get(i).unwrap_or(&"");
                    data.insert(col.to_string(), serde_json::Value::String(val.to_string()));
                }
            }

            let has_returning = returning.is_some();
            return Ok(Query {
                storage_type: StorageType::Relational,
                operation: if has_returning {
                    QueryOperation::Read
                } else {
                    QueryOperation::Create
                },
                table,
                conditions: None,
                data: Some(serde_json::Value::Object(data)),
                limit: None,
                offset: None,
                namespace: None,
            });
        }
    }
    Err(crate::Error::InvalidRequest(
        "Invalid INSERT statement".into(),
    ))
}

fn parse_update(sql: &str) -> Result<Query> {
    let sql = sql.trim_end_matches(';');
    let re = Regex::new(
        r"(?i)^UPDATE\s+([^\s]+)\s+SET\s+(.+?)(?:\s+WHERE\s+(.+?))?(?:\s+RETURNING\s+(.+))?$",
    )
    .ok();
    if let Some(re) = re {
        if let Some(caps) = re.captures(sql) {
            let table = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let set_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let condition_str = caps.get(3).map(|m| m.as_str());
            let _returning = caps.get(4).map(|m| m.as_str());

            let mut data = serde_json::Map::new();
            for assignment in set_str.split(',') {
                let parts: Vec<&str> = assignment.splitn(2, '=').map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    data.insert(
                        parts[0].trim_matches('"').to_string(),
                        serde_json::Value::String(parts[1].trim_matches('\'').to_string()),
                    );
                }
            }

            let conditions = condition_str.map(parse_simple_condition);

            return Ok(Query {
                storage_type: StorageType::Relational,
                operation: QueryOperation::Update,
                table,
                conditions,
                data: Some(serde_json::Value::Object(data)),
                limit: None,
                offset: None,
                namespace: None,
            });
        }
    }
    Err(crate::Error::InvalidRequest(
        "Invalid UPDATE statement".into(),
    ))
}

fn parse_delete(sql: &str) -> Result<Query> {
    let sql = sql.trim_end_matches(';');
    let re =
        Regex::new(r"(?i)^DELETE\s+FROM\s+([^\s]+)(?:\s+WHERE\s+(.+?))?(?:\s+RETURNING\s+(.+))?$")
            .ok();
    if let Some(re) = re {
        if let Some(caps) = re.captures(sql) {
            let table = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let condition_str = caps.get(2).map(|m| m.as_str());
            let _returning = caps.get(3).map(|m| m.as_str());

            let conditions = condition_str.map(parse_simple_condition);

            return Ok(Query {
                storage_type: StorageType::Relational,
                operation: QueryOperation::Delete,
                table,
                conditions,
                data: None,
                limit: None,
                offset: None,
                namespace: None,
            });
        }
    }
    Err(crate::Error::InvalidRequest(
        "Invalid DELETE statement".into(),
    ))
}

fn parse_create_table(sql: &str) -> Result<Query> {
    let sql = sql.trim_end_matches(';');
    let re = Regex::new(
        r"(?i)^CREATE\s+(TEMPORARY\s+)?TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?([^\s(]+)\s*\((.*)\)\s*$",
    )
    .ok();
    if let Some(re) = re {
        if let Some(caps) = re.captures(sql) {
            let _temp = caps.get(1).map(|m| m.as_str());
            let _table = caps
                .get(2)
                .map(|m| m.as_str().trim_matches('"').to_string())
                .unwrap_or_default();
            let _cols = caps.get(3).map(|m| m.as_str()).unwrap_or("");

            let schema = crate::storage::Schema {
                fields: vec![],
                indexes: vec![],
                constraints: vec![],
            };

            return Ok(Query {
                storage_type: StorageType::Relational,
                operation: QueryOperation::Create,
                table: _table,
                conditions: None,
                data: Some(serde_json::to_value(&schema).unwrap_or_default()),
                limit: None,
                offset: None,
                namespace: None,
            });
        }
    }
    Err(crate::Error::InvalidRequest(
        "Invalid CREATE TABLE statement".into(),
    ))
}

fn parse_alter_table(sql: &str) -> Result<Query> {
    let sql = sql.trim_end_matches(';');
    // ALTER TABLE name ADD [COLUMN] col_name type ...
    let add_re =
        Regex::new(r"(?i)^ALTER\s+TABLE\s+([^\s]+)\s+ADD\s+(COLUMN\s+)?([^\s]+)\s+(.+)$").ok();
    if let Some(re) = add_re {
        if let Some(caps) = re.captures(sql) {
            let table = caps
                .get(1)
                .map(|m| m.as_str().trim_matches('"').to_string())
                .unwrap_or_default();
            let col_name = caps
                .get(3)
                .map(|m| m.as_str().trim_matches('"').to_string())
                .unwrap_or_default();
            let col_type = caps
                .get(4)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            let field_type = match col_type.to_uppercase().as_str() {
                t if t.starts_with("INTEGER") || t.starts_with("INT") => {
                    crate::storage::FieldType::Integer
                }
                t if t.starts_with("BIGINT") => crate::storage::FieldType::BigInt,
                t if t.starts_with("SMALLINT") => crate::storage::FieldType::SmallInt,
                t if t.starts_with("SERIAL") => crate::storage::FieldType::Serial,
                t if t.starts_with("BIGSERIAL") => crate::storage::FieldType::BigSerial,
                t if t.starts_with("VARCHAR") => {
                    let re_len = Regex::new(r"VARCHAR\s*\((\d+)\)").ok();
                    let len = re_len
                        .and_then(|r| r.captures(&col_type))
                        .and_then(|c| c.get(1))
                        .and_then(|m| m.as_str().parse::<u64>().ok())
                        .unwrap_or(255);
                    crate::storage::FieldType::Varchar(len)
                }
                t if t.starts_with("CHAR") => {
                    let re_len = Regex::new(r"CHAR\s*\((\d+)\)").ok();
                    let len = re_len
                        .and_then(|r| r.captures(&col_type))
                        .and_then(|c| c.get(1))
                        .and_then(|m| m.as_str().parse::<u64>().ok())
                        .unwrap_or(1);
                    crate::storage::FieldType::Char(len)
                }
                t if t.starts_with("TEXT") => crate::storage::FieldType::Text,
                t if t.starts_with("BOOLEAN") => crate::storage::FieldType::Boolean,
                t if t.starts_with("FLOAT") || t.starts_with("DOUBLE") || t.starts_with("REAL") => {
                    crate::storage::FieldType::Float
                }
                t if t.starts_with("DECIMAL") || t.starts_with("NUMERIC") => {
                    let re_dec = Regex::new(r"DECIMAL\s*\((\d+)\s*,\s*(\d+)\)").ok();
                    let (p, s) = re_dec
                        .and_then(|r| r.captures(&col_type))
                        .map(|c| {
                            (
                                c.get(1)
                                    .and_then(|m| m.as_str().parse::<u64>().ok())
                                    .unwrap_or(10),
                                c.get(2)
                                    .and_then(|m| m.as_str().parse::<u64>().ok())
                                    .unwrap_or(0),
                            )
                        })
                        .unwrap_or((10, 0));
                    crate::storage::FieldType::Decimal(p, s)
                }
                t if t.starts_with("DATE") => crate::storage::FieldType::Date,
                t if t.starts_with("TIMESTAMP") => crate::storage::FieldType::Timestamp,
                t if t.starts_with("TIME") => crate::storage::FieldType::Time,
                t if t.starts_with("UUID") => crate::storage::FieldType::Uuid,
                t if t.starts_with("JSON") || t.starts_with("JSONB") => {
                    crate::storage::FieldType::Json
                }
                t if t.starts_with("MONEY") => crate::storage::FieldType::Money,
                t if t.starts_with("INTERVAL") => crate::storage::FieldType::Interval,
                t if t.starts_with("BLOB") || t.starts_with("BYTEA") => {
                    crate::storage::FieldType::Binary
                }
                _ => crate::storage::FieldType::Text,
            };

            let field = crate::storage::Field {
                name: col_name,
                field_type,
                nullable: !col_type.to_uppercase().contains("NOT NULL"),
                default_value: None,
                constraints: vec![],
            };
            let data = serde_json::to_value(&field).unwrap_or_default();
            return Ok(Query {
                storage_type: StorageType::Relational,
                operation: QueryOperation::AlterTableAddColumn,
                table,
                conditions: None,
                data: Some(data),
                limit: None,
                offset: None,
                namespace: None,
            });
        }
    }
    // ALTER TABLE name DROP [COLUMN] col_name
    let drop_re = Regex::new(r"(?i)^ALTER\s+TABLE\s+([^\s]+)\s+DROP\s+(COLUMN\s+)?([^\s]+)$").ok();
    if let Some(re) = drop_re {
        if let Some(caps) = re.captures(sql) {
            let table = caps
                .get(1)
                .map(|m| m.as_str().trim_matches('"').to_string())
                .unwrap_or_default();
            let col_name = caps
                .get(3)
                .map(|m| m.as_str().trim_matches('"').to_string())
                .unwrap_or_default();
            return Ok(Query {
                storage_type: StorageType::Relational,
                operation: QueryOperation::AlterTableDropColumn,
                table,
                conditions: None,
                data: Some(serde_json::Value::String(col_name)),
                limit: None,
                offset: None,
                namespace: None,
            });
        }
    }
    Err(crate::Error::InvalidRequest(
        "Invalid ALTER TABLE statement".into(),
    ))
}

fn parse_truncate(sql: &str) -> Result<Query> {
    let sql = sql.trim_end_matches(';');
    let has_cascade = sql.to_uppercase().contains("CASCADE");
    let table = extract_table_name(sql, "TRUNCATE TABLE")
        .or_else(|| extract_table_name(sql, "TRUNCATE"))
        .unwrap_or_default();
    Ok(Query {
        storage_type: StorageType::Relational,
        operation: QueryOperation::Truncate,
        table,
        conditions: None,
        data: Some(serde_json::json!({"cascade": has_cascade})),
        limit: None,
        offset: None,
        namespace: None,
    })
}

fn parse_create_sequence(sql: &str) -> Result<Query> {
    let sql = sql.trim_end_matches(';');
    let name = extract_table_name(sql, "CREATE SEQUENCE").unwrap_or_default();
    let increment = extract_value(sql, "INCREMENT")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1);
    let min_value = extract_value(sql, "MINVALUE")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1);
    let max_value = extract_value(sql, "MAXVALUE")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(i64::MAX);
    let cache = extract_value(sql, "CACHE")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1);
    let cycle = sql.to_uppercase().contains("CYCLE");

    let seq = crate::storage::Sequence {
        name: name.clone(),
        current_value: 1,
        increment,
        min_value,
        max_value,
        cycle,
        cache_size: cache,
        owned_by: None,
    };
    let data = serde_json::to_value(&seq).unwrap_or_default();
    Ok(Query {
        storage_type: StorageType::Relational,
        operation: QueryOperation::CreateSequence,
        table: name,
        conditions: None,
        data: Some(data),
        limit: None,
        offset: None,
        namespace: None,
    })
}

fn parse_create_view(sql: &str) -> Result<Query> {
    let sql = sql.trim_end_matches(';');
    let re = Regex::new(r"(?i)^CREATE\s+(OR\s+REPLACE\s+)?VIEW\s+([^\s]+)\s+AS\s+(.+)$").ok();
    if let Some(re) = re {
        if let Some(caps) = re.captures(sql) {
            let view_name = caps
                .get(2)
                .map(|m| m.as_str().trim_matches('"').to_string())
                .unwrap_or_default();
            let _query_body = caps
                .get(3)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            let view = crate::storage::View {
                name: view_name.clone(),
                query_definition: serde_json::json!({}),
                columns: vec![],
                materialized: false,
                referenced_tables: vec![],
            };
            let data = serde_json::to_value(&view).unwrap_or_default();
            return Ok(Query {
                storage_type: StorageType::Relational,
                operation: QueryOperation::CreateView,
                table: view_name,
                conditions: None,
                data: Some(data),
                limit: None,
                offset: None,
                namespace: None,
            });
        }
    }
    Err(crate::Error::InvalidRequest(
        "Invalid CREATE VIEW statement".into(),
    ))
}

fn parse_create_trigger(sql: &str) -> Result<Query> {
    let sql = sql.trim_end_matches(';');
    let re = Regex::new(r"(?i)^CREATE\s+TRIGGER\s+([^\s]+)\s+(BEFORE|AFTER|INSTEAD\s+OF)\s+(INSERT|UPDATE|DELETE|INSERT\s+OR\s+UPDATE|INSERT\s+OR\s+DELETE|UPDATE\s+OR\s+DELETE)\s+ON\s+([^\s]+)\s*(?:FOR\s+EACH\s+ROW\s*)?(.*)$").ok();
    if let Some(re) = re {
        if let Some(caps) = re.captures(sql) {
            let trig_name = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let timing_str = caps
                .get(2)
                .map(|m| m.as_str().to_uppercase())
                .unwrap_or_default();
            let event_str = caps
                .get(3)
                .map(|m| m.as_str().to_uppercase())
                .unwrap_or_default();
            let table_name = caps
                .get(4)
                .map(|m| m.as_str().trim_matches('"').to_string())
                .unwrap_or_default();
            let action_str = caps
                .get(5)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            let timing = match timing_str.as_str() {
                "BEFORE" => crate::storage::TriggerTiming::Before,
                "AFTER" => crate::storage::TriggerTiming::After,
                _ => crate::storage::TriggerTiming::After,
            };
            let event = match event_str.as_str() {
                "INSERT" => crate::storage::TriggerEvent::Insert,
                "UPDATE" => crate::storage::TriggerEvent::Update,
                "DELETE" => crate::storage::TriggerEvent::Delete,
                _ => crate::storage::TriggerEvent::All,
            };
            let operation = crate::storage::TriggerOperation::Execute(action_str);

            let trigger = crate::storage::Trigger {
                name: trig_name,
                table_name: table_name.clone(),
                timing,
                event,
                operation,
                enabled: true,
                columns: None,
            };
            let data = serde_json::to_value(&trigger).unwrap_or_default();
            return Ok(Query {
                storage_type: StorageType::Relational,
                operation: QueryOperation::CreateTrigger,
                table: table_name,
                conditions: None,
                data: Some(data),
                limit: None,
                offset: None,
                namespace: None,
            });
        }
    }
    Err(crate::Error::InvalidRequest(
        "Invalid CREATE TRIGGER statement".into(),
    ))
}

fn parse_drop_trigger(sql: &str) -> Result<Query> {
    let re = Regex::new(r"(?i)^DROP\s+TRIGGER\s+(?:IF\s+EXISTS\s+)?([^\s]+)\s+ON\s+([^\s;]+)").ok();
    if let Some(re) = re {
        if let Some(caps) = re.captures(sql) {
            let trig_name = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let table_name = caps
                .get(2)
                .map(|m| m.as_str().trim_matches('"').to_string())
                .unwrap_or_default();
            return Ok(Query {
                storage_type: StorageType::Relational,
                operation: QueryOperation::DropTrigger,
                table: table_name,
                conditions: None,
                data: Some(serde_json::Value::String(trig_name)),
                limit: None,
                offset: None,
                namespace: None,
            });
        }
    }
    Err(crate::Error::InvalidRequest(
        "Invalid DROP TRIGGER statement".into(),
    ))
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_select() {
        let q = parse_sql("SELECT * FROM users WHERE id = 1").unwrap();
        assert_eq!(q.table, "users");
        assert!(matches!(q.operation, QueryOperation::Read));
    }

    #[test]
    fn test_parse_insert() {
        let q = parse_sql("INSERT INTO users (name, age) VALUES ('Alice', 30)").unwrap();
        assert_eq!(q.table, "users");
        assert!(matches!(q.operation, QueryOperation::Create));
    }

    #[test]
    fn test_parse_update() {
        let q = parse_sql("UPDATE users SET name = 'Bob' WHERE id = 1").unwrap();
        assert_eq!(q.table, "users");
        assert!(matches!(q.operation, QueryOperation::Update));
    }

    #[test]
    fn test_parse_delete() {
        let q = parse_sql("DELETE FROM users WHERE id = 1").unwrap();
        assert_eq!(q.table, "users");
        assert!(matches!(q.operation, QueryOperation::Delete));
    }

    #[test]
    fn test_parse_create_sequence() {
        let q =
            parse_sql("CREATE SEQUENCE my_seq INCREMENT 1 MINVALUE 1 MAXVALUE 1000 CACHE 1 CYCLE")
                .unwrap();
        assert!(matches!(q.operation, QueryOperation::CreateSequence));
    }

    #[test]
    fn test_parse_truncate() {
        let q = parse_sql("TRUNCATE TABLE users CASCADE").unwrap();
        assert!(matches!(q.operation, QueryOperation::Truncate));
    }

    #[test]
    fn test_parse_alter_table_add() {
        let q = parse_sql("ALTER TABLE users ADD COLUMN email VARCHAR(255)").unwrap();
        assert!(matches!(q.operation, QueryOperation::AlterTableAddColumn));
    }

    #[test]
    fn test_parse_alter_table_drop() {
        let q = parse_sql("ALTER TABLE users DROP COLUMN email").unwrap();
        assert!(matches!(q.operation, QueryOperation::AlterTableDropColumn));
    }

    #[test]
    fn test_parse_create_trigger() {
        let q = parse_sql("CREATE TRIGGER check_age BEFORE INSERT ON users FOR EACH ROW RAISE 'Age must be positive'").unwrap();
        assert!(matches!(q.operation, QueryOperation::CreateTrigger));
    }
}
