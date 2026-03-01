/*
 * PrimusDB Query Parser Module
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha
 */

//! # Query Parser Module
//!
//! This module handles parsing of various query languages into a unified intermediate representation.
//! Supported query languages include SQL, MongoDB, Mango, and native UQL.

use crate::query::QueryLanguage;
use crate::query::UqlQuery;
use crate::Result;
use serde::{Deserialize, Serialize};

/// Query parser for multiple query languages
pub struct QueryParser;

impl QueryParser {
    pub fn new() -> Self {
        QueryParser
    }

    pub fn parse(&self, query: &UqlQuery) -> Result<ParsedQuery> {
        match query.query_type {
            QueryLanguage::Sql => self.parse_sql(&query.query),
            QueryLanguage::MongoDb => self.parse_mongodb(&query.query),
            QueryLanguage::Mango => self.parse_mango(&query.query),
            QueryLanguage::Uql => self.parse_uql(&query.query),
            QueryLanguage::Auto => self.detect_and_parse(&query.query),
        }
    }

    fn detect_and_parse(&self, query: &str) -> Result<ParsedQuery> {
        let trimmed = query.trim();

        if trimmed.starts_with("SELECT")
            || trimmed.starts_with("select")
            || trimmed.starts_with("INSERT")
            || trimmed.starts_with("UPDATE")
            || trimmed.starts_with("DELETE")
        {
            return self.parse_sql(trimmed);
        }

        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(_) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if trimmed.contains("$and")
                    || trimmed.contains("$or")
                    || trimmed.contains("$gt")
                    || trimmed.contains("$lt")
                {
                    return self.parse_mango(trimmed);
                }
                return self.parse_mongodb(trimmed);
            }
        }

        self.parse_uql(trimmed)
    }

    /// Parse SQL-like queries
    ///
    /// # Supported Syntax
    ///
    /// - SELECT columns FROM table [WHERE conditions] [JOIN table ON condition] [ORDER BY column] [LIMIT n]
    /// - INSERT INTO table (columns) VALUES (values)
    /// - UPDATE table SET column=value [WHERE conditions]
    /// - DELETE FROM table [WHERE conditions]
    ///
    /// # Cross-Engine Joins
    ///
    /// UQL supports joins across different storage engines:
    /// ```sql
    /// SELECT u.name, v.embedding_score
    /// FROM users u
    /// JOIN vectors v ON u.id = v.entity_id
    /// WHERE u.age > 25
    /// ```
    fn parse_sql(&self, query: &str) -> Result<ParsedQuery> {
        let query_lower = query.to_lowercase();
        let query_lower = query_lower.trim();

        let operation = if query_lower.starts_with("select") {
            QueryOperation::Select
        } else if query_lower.starts_with("insert") {
            QueryOperation::Insert
        } else if query_lower.starts_with("update") {
            QueryOperation::Update
        } else if query_lower.starts_with("delete") {
            QueryOperation::Delete
        } else if query_lower.starts_with("create") {
            QueryOperation::Create
        } else if query_lower.starts_with("drop") {
            QueryOperation::Drop
        } else {
            return Err(crate::Error::ValidationError(
                "Unknown SQL operation".to_string(),
            ));
        };

        let mut parsed = ParsedQuery {
            operation,
            source_tables: vec![],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
        };

        // Extract table names
        if let Some(from_pos) = query_lower.find(" from ") {
            let rest = &query[from_pos + 7..];
            if let Some(where_pos) = rest.find(" where ") {
                let table_part = &rest[..where_pos].trim();
                parsed.source_tables = self.extract_tables(table_part);
            } else if let Some(join_pos) = rest.to_lowercase().find(" join ") {
                let table_part = &rest[..join_pos].trim();
                parsed.source_tables = self.extract_tables(table_part);
            } else {
                parsed.source_tables = self.extract_tables(rest.trim());
            }
        }

        // Extract target table for INSERT/UPDATE/DELETE
        if let Some(into_pos) = query_lower.find(" into ") {
            let rest = &query[into_pos + 6..];
            if let Some(space_pos) = rest.find(' ') {
                parsed.target_table = Some(rest[..space_pos].trim().to_string());
            } else {
                parsed.target_table = Some(rest.trim().to_string());
            }
        } else if let Some(update_pos) = query_lower.find(" update ") {
            let rest = &query[update_pos + 8..];
            if let Some(space_pos) = rest.find(' ') {
                parsed.target_table = Some(rest[..space_pos].trim().to_string());
            }
        } else if let Some(delete_pos) = query_lower.find(" from ") {
            let rest = &query[delete_pos + 7..];
            if let Some(space_pos) = rest.find(' ') {
                parsed.target_table = Some(rest[..space_pos].trim().to_string());
            }
        }

        // Extract columns for SELECT
        if let Some(select_pos) = query_lower.find("select ") {
            let rest = &query[select_pos + 7..];
            if let Some(from_pos) = rest.to_lowercase().find(" from ") {
                let cols = rest[..from_pos].trim();
                if cols != "*" {
                    parsed.columns = cols.split(',').map(|s| s.trim().to_string()).collect();
                }
            }
        }

        // Extract WHERE conditions
        if let Some(where_pos) = query_lower.find(" where ") {
            let rest = &query[where_pos + 7..];
            let cond_end = rest
                .to_lowercase()
                .find(" order by ")
                .or_else(|| rest.to_lowercase().find(" group by "))
                .or_else(|| rest.to_lowercase().find(" limit "))
                .unwrap_or(rest.len());
            let conditions = &rest[..cond_end].trim();
            parsed.conditions = Some(conditions.to_string());
        }

        // Extract JOINs (including cross-engine joins)
        if let Some(join_pos) = query_lower.find(" join ") {
            let rest = &query[join_pos + 6..];
            let join_end = rest.to_lowercase().find(" on ").unwrap_or(rest.len());
            let join_table = rest[..join_end].trim();

            if let Some(on_pos) = rest.to_lowercase().find(" on ") {
                let on_condition = rest[on_pos + 4..].trim();
                let on_end = on_condition
                    .to_lowercase()
                    .find(" where ")
                    .or_else(|| on_condition.to_lowercase().find(" order by "))
                    .or_else(|| on_condition.to_lowercase().find(" group by "))
                    .or_else(|| on_condition.to_lowercase().find(" limit "))
                    .unwrap_or(on_condition.len());

                let join = JoinClause {
                    join_type: JoinType::Inner,
                    table: join_table.to_string(),
                    condition: on_condition[..on_end].to_string(),
                    engine_hint: self.detect_engine_from_table(join_table),
                };
                parsed.joins.push(join);
            }
        }

        // Extract ORDER BY
        if let Some(order_pos) = query_lower.find(" order by ") {
            let rest = &query[order_pos + 10..];
            let order_end = rest.to_lowercase().find(" limit ").unwrap_or(rest.len());
            let order_cols = rest[..order_end].trim();
            parsed.order_by = order_cols
                .split(',')
                .map(|s| {
                    let parts: Vec<&str> = s.trim().split_whitespace().collect();
                    if parts.len() > 1 {
                        OrderByClause {
                            column: parts[0].to_string(),
                            direction: if parts[1].to_lowercase() == "desc" {
                                "DESC".to_string()
                            } else {
                                "ASC".to_string()
                            },
                        }
                    } else {
                        OrderByClause {
                            column: s.trim().to_string(),
                            direction: "ASC".to_string(),
                        }
                    }
                })
                .collect();
        }

        // Extract LIMIT
        if let Some(limit_pos) = query_lower.find(" limit ") {
            let rest = &query[limit_pos + 7..];
            if let Ok(limit) = rest.trim().parse::<usize>() {
                parsed.limit = Some(limit);
            }
        }

        Ok(parsed)
    }

    /// Parse MongoDB-style queries
    fn parse_mongodb(&self, query: &str) -> Result<ParsedQuery> {
        let value: serde_json::Value = serde_json::from_str(query)
            .map_err(|e| crate::Error::ValidationError(format!("Invalid JSON: {}", e)))?;

        let mut parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec![],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: Some(query.to_string()),
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
        };

        if let Some(obj) = value.as_object() {
            for (key, _val) in obj {
                if !key.starts_with('$') {
                    parsed.source_tables.push(key.clone());
                }
            }
        }

        Ok(parsed)
    }

    /// Parse Mango queries (CouchDB-style)
    fn parse_mango(&self, query: &str) -> Result<ParsedQuery> {
        let value: serde_json::Value = serde_json::from_str(query)
            .map_err(|e| crate::Error::ValidationError(format!("Invalid JSON: {}", e)))?;

        let mut parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec![],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: Some(query.to_string()),
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
        };

        if let Some(obj) = value.as_object() {
            if let Some(selector) = obj.get("selector") {
                // Extract table from selector if available
                if let Some(selector_obj) = selector.as_object() {
                    for (key, _) in selector_obj {
                        if !key.starts_with('$') {
                            parsed.source_tables.push(key.clone());
                            break;
                        }
                    }
                }
            }

            if let Some(limit_val) = obj.get("limit").and_then(|v| v.as_u64()) {
                parsed.limit = Some(limit_val as usize);
            }

            if let Some(skip_val) = obj.get("skip").and_then(|v| v.as_u64()) {
                parsed.offset = Some(skip_val as usize);
            }

            if let Some(sort_arr) = obj.get("sort").and_then(|v| v.as_array()) {
                for item in sort_arr {
                    if let Some(obj) = item.as_object() {
                        for (col, dir) in obj {
                            parsed.order_by.push(OrderByClause {
                                column: col.clone(),
                                direction: if dir.as_str() == Some("desc") {
                                    "DESC".to_string()
                                } else {
                                    "ASC".to_string()
                                },
                            });
                        }
                    }
                }
            }
        }

        Ok(parsed)
    }

    /// Parse native UQL format
    fn parse_uql(&self, query: &str) -> Result<ParsedQuery> {
        let value: serde_json::Value = serde_json::from_str(query)
            .map_err(|e| crate::Error::ValidationError(format!("Invalid UQL: {}", e)))?;

        let mut parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec![],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
        };

        if let Some(obj) = value.as_object() {
            if let Some(from) = obj.get("from").and_then(|v| v.as_str()) {
                parsed.source_tables.push(from.to_string());
            }

            if let Some(select) = obj.get("select") {
                if let Some(cols) = select.as_array() {
                    parsed.columns = cols
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                }
            }

            if let Some(where_cond) = obj.get("where") {
                parsed.conditions = Some(where_cond.to_string());
            }

            if let Some(joins) = obj.get("joins").and_then(|v| v.as_array()) {
                for join_val in joins {
                    if let Some(join_obj) = join_val.as_object() {
                        let join = JoinClause {
                            join_type: JoinType::Inner,
                            table: join_obj
                                .get("table")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            condition: join_obj
                                .get("on")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            engine_hint: None,
                        };
                        parsed.joins.push(join);
                    }
                }
            }

            if let Some(limit_val) = obj.get("limit").and_then(|v| v.as_u64()) {
                parsed.limit = Some(limit_val as usize);
            }
        }

        Ok(parsed)
    }

    fn extract_tables(&self, table_part: &str) -> Vec<String> {
        table_part
            .split(',')
            .map(|s| {
                let trimmed = s.trim();
                // Handle aliases: "users AS u" or "users u"
                if let Some(as_pos) = trimmed.to_lowercase().find(" as ") {
                    trimmed[..as_pos].trim().to_string()
                } else if let Some(alias_pos) = trimmed.find(' ') {
                    trimmed[..alias_pos].trim().to_string()
                } else {
                    trimmed.to_string()
                }
            })
            .collect()
    }

    fn detect_engine_from_table(&self, table: &str) -> Option<String> {
        // Infer engine from table naming conventions or metadata
        if table.to_lowercase().contains("vector") || table.to_lowercase().contains("embedding") {
            Some("vector".to_string())
        } else if table.to_lowercase().contains("doc") {
            Some("document".to_string())
        } else if table.to_lowercase().contains("kv") || table.to_lowercase().contains("cache") {
            Some("keyvalue".to_string())
        } else if table.to_lowercase().contains("col") || table.to_lowercase().contains("analytics")
        {
            Some("columnar".to_string())
        } else {
            Some("relational".to_string())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuery {
    pub operation: QueryOperation,
    pub source_tables: Vec<String>,
    pub target_table: Option<String>,
    pub columns: Vec<String>,
    pub conditions: Option<String>,
    pub joins: Vec<JoinClause>,
    pub order_by: Vec<OrderByClause>,
    pub group_by: Vec<String>,
    pub aggregations: Vec<AggregationClause>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub set_operations: Vec<SetOperation>,
    pub nested_queries: Vec<Box<ParsedQuery>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryOperation {
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
    Alter,
    Truncate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: String,
    pub condition: String,
    pub engine_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderByClause {
    pub column: String,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationClause {
    pub agg_type: AggregationType,
    pub column: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationType {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    GroupConcat,
    ArrayAgg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetOperation {
    pub operation_type: SetOperationType,
    pub query: Box<ParsedQuery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SetOperationType {
    Union,
    Intersect,
    Except,
}
