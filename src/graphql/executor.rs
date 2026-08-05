/*
 * PrimusDB GraphQL Service — Executor
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.0.0
 */

//! Executor that resolves a parsed GraphQL document against the PrimusDB
//! storage engines and unified search service.
//!
//! Every resolver lives in the service layer (engines, search, query) — the
//! executor never talks to storage directly; it routes through
//! [`PrimusDB::execute_query`], [`PrimusDB::storage_engine`] and
//! [`crate::search::SearchService`], so integrity/consensus wrapping and
//! capability routing apply uniformly.

use super::ast::{Document, Field, Operation, OperationType, Value};
use crate::search::{SearchRequest, SearchService};
use crate::{PrimusDB, Query, QueryOperation, Record, StorageType};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A single error entry in a GraphQL response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    /// Human-readable error message.
    pub message: String,
    /// Field path to the failing resolver.
    pub path: Vec<String>,
}

/// A GraphQL execution request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLRequest {
    /// The GraphQL document source.
    pub query: String,
    /// Optional operation name (required when several operations are defined).
    pub operation_name: Option<String>,
    /// Variable values keyed by variable name.
    pub variables: BTreeMap<String, serde_json::Value>,
}

impl GraphQLRequest {
    /// Build a request from a raw JSON payload (the canonical wire format).
    pub fn from_json(value: serde_json::Value) -> Self {
        let variables = value
            .get("variables")
            .and_then(|v| v.as_object())
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        Self {
            query: value
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            operation_name: value
                .get("operationName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            variables,
        }
    }
}

/// A GraphQL execution response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLResponse {
    /// Resolved root value (absent on fatal parse errors).
    pub data: Option<serde_json::Value>,
    /// Non-fatal per-field errors.
    pub errors: Vec<GraphQLError>,
}

/// The GraphQL executor. Stateless: each call parses and executes a document.
pub struct GraphQLExecutor;

/// Runtime context for a resolver: what the currently selected object is.
#[derive(Debug, Clone)]
enum Ctx {
    /// Root of a `query` operation.
    QueryRoot,
    /// Root of a `mutation` operation.
    MutationRoot,
    /// A storage engine info object.
    EngineInfo {
        /// Engine name.
        name: String,
        /// Tables/collections exposed by the engine.
        tables: Vec<String>,
    },
    /// A table/collection with its engine and name.
    Table {
        /// Storage type name (as configured by the caller).
        storage_type: String,
        /// Table/collection name.
        name: String,
    },
    /// A single record.
    Record {
        /// Record id.
        id: String,
        /// Record payload.
        data: serde_json::Value,
        /// Record metadata.
        metadata: serde_json::Value,
    },
    /// A unified-search response.
    SearchResponse {
        /// Echo of the query text.
        query: String,
        /// Total hits before pagination.
        total: u64,
        /// Merged hits (raw JSON for cheap field access).
        hits: Vec<serde_json::Value>,
    },
    /// A single merged search hit.
    SearchHit(serde_json::Value),
}

impl GraphQLExecutor {
    /// Execute a GraphQL request against a PrimusDB instance.
    pub async fn execute(db: &PrimusDB, request: &GraphQLRequest) -> GraphQLResponse {
        let document = match super::parser::Parser::parse_document(&request.query) {
            Ok(doc) => doc,
            Err(e) => {
                return GraphQLResponse {
                    data: None,
                    errors: vec![GraphQLError {
                        message: e.message,
                        path: vec!["parse".to_string()],
                    }],
                }
            }
        };

        let operation = match select_operation(&document, request.operation_name.as_deref()) {
            Ok(op) => op,
            Err(e) => {
                return GraphQLResponse {
                    data: None,
                    errors: vec![GraphQLError {
                        message: e,
                        path: vec![],
                    }],
                }
            }
        };

        let variables = bind_variables(operation, &request.variables);
        let root = match operation.operation_type {
            OperationType::Query => Ctx::QueryRoot,
            OperationType::Mutation => Ctx::MutationRoot,
        };

        match resolve_selection_set(db, &root, &operation.selection_set, &variables).await {
            Ok(data) => GraphQLResponse {
                data: Some(data),
                errors: vec![],
            },
            Err(e) => GraphQLResponse {
                data: None,
                errors: vec![e],
            },
        }
    }
}

/// Merge provided variables with declared defaults.
fn bind_variables(
    operation: &Operation,
    provided: &BTreeMap<String, serde_json::Value>,
) -> BTreeMap<String, serde_json::Value> {
    let mut bound = BTreeMap::new();
    for def in &operation.variable_definitions {
        if let Some(v) = provided.get(&def.name) {
            bound.insert(def.name.clone(), v.clone());
        } else if let Some(default) = &def.default {
            bound.insert(def.name.clone(), default.clone().into_json());
        }
    }
    // Extra provided variables (not declared) are still made available.
    for (k, v) in provided {
        bound.entry(k.clone()).or_insert_with(|| v.clone());
    }
    bound
}

/// Pick the operation to run (named match or the only/first one).
fn select_operation<'a>(
    document: &'a Document,
    name: Option<&str>,
) -> std::result::Result<&'a Operation, String> {
    match name {
        Some(name) => document
            .operations
            .iter()
            .find(|op| op.name.as_deref() == Some(name))
            .ok_or_else(|| format!("Unknown operation '{name}'")),
        None => {
            if document.operations.len() == 1 {
                Ok(&document.operations[0])
            } else {
                Err(
                    "Operation name is required when a document contains several operations"
                        .to_string(),
                )
            }
        }
    }
}

/// Resolve a selection set into a JSON object.
///
/// Selection resolution is mutually recursive with [`resolve_field`] and
/// [`resolve_list`], so each is boxed to keep the future type finite.
fn resolve_selection_set<'a>(
    db: &'a PrimusDB,
    ctx: &'a Ctx,
    fields: &'a [Field],
    variables: &'a BTreeMap<String, serde_json::Value>,
) -> BoxFuture<'a, std::result::Result<serde_json::Value, GraphQLError>> {
    Box::pin(async move {
        let mut out = serde_json::Map::new();
        for field in fields {
            let key = field.alias.clone().unwrap_or_else(|| field.name.clone());
            match resolve_field(db, ctx, field, variables).await {
                Ok(value) => {
                    out.insert(key, value);
                }
                Err(mut err) => {
                    err.path.insert(0, key);
                    return Err(err);
                }
            }
        }
        Ok(serde_json::Value::Object(out))
    })
}

/// Resolve one field of the current context to a JSON value.
fn resolve_field<'a>(
    db: &'a PrimusDB,
    ctx: &'a Ctx,
    field: &'a Field,
    variables: &'a BTreeMap<String, serde_json::Value>,
) -> BoxFuture<'a, std::result::Result<serde_json::Value, GraphQLError>> {
    Box::pin(async move {
        let err = |message: String| GraphQLError {
            message,
            path: vec![],
        };

        match ctx {
            Ctx::QueryRoot => match field.name.as_str() {
                "__typename" => Ok(serde_json::json!("Query")),
                "engines" => {
                    let engines = list_engines(db)?;
                    resolve_list(db, engines, &field.selection_set, variables).await
                }
                "table" => {
                    let storage_type =
                        arg_str(field, "storageType", variables).ok_or_else(|| {
                            err("field 'table' requires a 'storageType' argument".to_string())
                        })?;
                    let name = arg_str(field, "name", variables).ok_or_else(|| {
                        err("field 'table' requires a 'name' argument".to_string())
                    })?;
                    let ctx = Ctx::Table { storage_type, name };
                    require_selection(field, &ctx)?;
                    resolve_selection_set(db, &ctx, &field.selection_set, variables).await
                }
                "tables" => {
                    let storage_type =
                        arg_str(field, "storageType", variables).ok_or_else(|| {
                            err("field 'tables' requires a 'storageType' argument".to_string())
                        })?;
                    let tables = list_tables(db, &storage_type)?;
                    let ctxs: Vec<Ctx> = tables
                        .into_iter()
                        .map(|name| Ctx::Table {
                            storage_type: storage_type.clone(),
                            name,
                        })
                        .collect();
                    resolve_list(db, ctxs, &field.selection_set, variables).await
                }
                "search" => {
                    let resp = run_search(db, field, variables).await?;
                    let ctx = Ctx::SearchResponse {
                        query: resp.query,
                        total: resp.total,
                        hits: resp
                            .hits
                            .into_iter()
                            .map(|h| serde_json::to_value(h).map_err(|e| err(e.to_string())))
                            .collect::<std::result::Result<_, _>>()?,
                    };
                    require_selection(field, &ctx)?;
                    resolve_selection_set(db, &ctx, &field.selection_set, variables).await
                }
                other => Err(err(format!("Unknown field '{other}' on Query"))),
            },
            Ctx::MutationRoot => match field.name.as_str() {
                "__typename" => Ok(serde_json::json!("Mutation")),
                "insert" | "update" | "delete" => run_mutation(db, field, variables).await,
                other => Err(err(format!("Unknown field '{other}' on Mutation"))),
            },
            Ctx::EngineInfo { name, tables } => match field.name.as_str() {
                "__typename" => Ok(serde_json::json!("EngineInfo")),
                "name" => Ok(serde_json::json!(name)),
                "tables" => Ok(serde_json::json!(tables)),
                other => Err(err(format!("Unknown field '{other}' on EngineInfo"))),
            },
            Ctx::Table { storage_type, name } => match field.name.as_str() {
                "__typename" => Ok(serde_json::json!("Table")),
                "name" => Ok(serde_json::json!(name)),
                "count" => {
                    let engine = engine_for(db, storage_type)?;
                    let info = engine
                        .table_info(name)
                        .await
                        .map_err(|e| err(e.to_string()))?;
                    Ok(serde_json::json!(info.row_count))
                }
                "records" => {
                    let records = select_records(db, storage_type, name, field, variables).await?;
                    let ctxs: Vec<Ctx> = records.into_iter().map(record_ctx).collect();
                    resolve_list(db, ctxs, &field.selection_set, variables).await
                }
                "record" => {
                    let id = arg_str(field, "id", variables).ok_or_else(|| {
                        err("field 'record' requires an 'id' argument".to_string())
                    })?;
                    let engine = engine_for(db, storage_type)?;
                    let conditions = serde_json::json!({ "id": id });
                    let records = engine
                        .select(name, Some(&conditions), 1, 0, &tx())
                        .await
                        .map_err(|e| err(e.to_string()))?;
                    match records.into_iter().next() {
                        Some(record) => {
                            let ctx = record_ctx(record);
                            require_selection(field, &ctx)?;
                            resolve_selection_set(db, &ctx, &field.selection_set, variables).await
                        }
                        None => Ok(serde_json::Value::Null),
                    }
                }
                other => Err(err(format!("Unknown field '{other}' on Table"))),
            },
            Ctx::Record { id, data, metadata } => match field.name.as_str() {
                "__typename" => Ok(serde_json::json!("Record")),
                "id" => Ok(serde_json::json!(id)),
                "data" => Ok(data.clone()),
                "metadata" => Ok(metadata.clone()),
                other => Err(err(format!("Unknown field '{other}' on Record"))),
            },
            Ctx::SearchResponse { query, total, hits } => match field.name.as_str() {
                "__typename" => Ok(serde_json::json!("SearchResponse")),
                "query" => Ok(serde_json::json!(query)),
                "total" => Ok(serde_json::json!(total)),
                "hits" => {
                    let ctxs: Vec<Ctx> = hits.iter().cloned().map(Ctx::SearchHit).collect();
                    resolve_list(db, ctxs, &field.selection_set, variables).await
                }
                other => Err(err(format!("Unknown field '{other}' on SearchResponse"))),
            },
            Ctx::SearchHit(hit) => match field.name.as_str() {
                "__typename" => Ok(serde_json::json!("SearchHit")),
                "engine" => Ok(hit
                    .get("engine")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)),
                "table" => Ok(hit.get("table").cloned().unwrap_or(serde_json::Value::Null)),
                "id" => Ok(hit.get("id").cloned().unwrap_or(serde_json::Value::Null)),
                "score" => Ok(hit.get("score").cloned().unwrap_or(serde_json::Value::Null)),
                "similarity" => Ok(hit
                    .get("similarity")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)),
                "record" => Ok(hit
                    .get("record")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)),
                other => Err(err(format!("Unknown field '{other}' on SearchHit"))),
            },
        }
    })
}

/// Build a transaction handle for read-only engine calls.
fn tx() -> crate::transaction::Transaction {
    crate::transaction::Transaction {
        id: format!("graphql-{}", uuid::Uuid::new_v4()),
        operations: vec![],
        status: crate::transaction::TransactionStatus::Prepared,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
        timeout_ms: 0,
    }
}

/// Enumerate engines registered in the instance with their tables.
fn list_engines(db: &PrimusDB) -> std::result::Result<Vec<Ctx>, GraphQLError> {
    let mut infos = Vec::new();
    for st in crate::search::ALL_ENGINES {
        let Some(engine) = db.storage_engine(st) else {
            continue;
        };
        let tables = engine.list_tables().map_err(|e| GraphQLError {
            message: e.to_string(),
            path: vec![],
        })?;
        infos.push(Ctx::EngineInfo {
            name: st.to_string(),
            tables,
        });
    }
    Ok(infos)
}

/// List tables of a storage type (error when the type is not registered).
fn list_tables(
    db: &PrimusDB,
    storage_type: &str,
) -> std::result::Result<Vec<String>, GraphQLError> {
    let st = parse_storage_type(storage_type)?;
    let engine = db.storage_engine(st).ok_or_else(|| GraphQLError {
        message: format!("Storage type '{storage_type}' is not available"),
        path: vec![],
    })?;
    engine.list_tables().map_err(|e| GraphQLError {
        message: e.to_string(),
        path: vec![],
    })
}

/// The engine for a storage type name.
fn engine_for(
    db: &PrimusDB,
    storage_type: &str,
) -> std::result::Result<std::sync::Arc<dyn crate::storage::StorageEngine>, GraphQLError> {
    let st = parse_storage_type(storage_type)?;
    db.storage_engine(st).ok_or_else(|| GraphQLError {
        message: format!("Storage type '{storage_type}' is not available"),
        path: vec![],
    })
}

/// Resolve a list of nested contexts through a selection set.
fn resolve_list<'a>(
    db: &'a PrimusDB,
    ctxs: Vec<Ctx>,
    selection: &'a [Field],
    variables: &'a BTreeMap<String, serde_json::Value>,
) -> BoxFuture<'a, std::result::Result<serde_json::Value, GraphQLError>> {
    Box::pin(async move {
        let mut out = Vec::with_capacity(ctxs.len());
        for ctx in ctxs {
            if selection.is_empty() {
                out.push(ctx.into_scalar());
            } else {
                out.push(resolve_selection_set(db, &ctx, selection, variables).await?);
            }
        }
        Ok(serde_json::Value::Array(out))
    })
}

impl Ctx {
    /// Collapse a context without a selection set into a scalar.
    fn into_scalar(self) -> serde_json::Value {
        match self {
            Ctx::EngineInfo { name, .. } => serde_json::json!(name),
            Ctx::Table { name, .. } => serde_json::json!(name),
            Ctx::SearchHit(hit) => hit,
            other => serde_json::json!({ "ctx": format!("{other:?}") }),
        }
    }
}

/// Build a record context from a [`Record`].
fn record_ctx(record: Record) -> Ctx {
    Ctx::Record {
        id: record.id,
        data: record.data,
        metadata: serde_json::to_value(&record.metadata).unwrap_or(serde_json::Value::Null),
    }
}

/// Run a `select` on a table with limit/offset/conditions arguments.
async fn select_records(
    db: &PrimusDB,
    storage_type: &str,
    table: &str,
    field: &Field,
    variables: &BTreeMap<String, serde_json::Value>,
) -> std::result::Result<Vec<Record>, GraphQLError> {
    let engine = engine_for(db, storage_type)?;
    let conditions = arg_json(field, "conditions", variables);
    let limit = arg_i64(field, "limit", variables).unwrap_or(100).max(0) as u64;
    let offset = arg_i64(field, "offset", variables).unwrap_or(0).max(0) as u64;
    engine
        .select(table, conditions.as_ref(), limit, offset, &tx())
        .await
        .map_err(|e| GraphQLError {
            message: e.to_string(),
            path: vec![],
        })
}

/// Run the unified search service with this field's arguments.
async fn run_search(
    db: &PrimusDB,
    field: &Field,
    variables: &BTreeMap<String, serde_json::Value>,
) -> std::result::Result<crate::search::SearchResponse, GraphQLError> {
    let storage_types = arg_list_of_strings(field, "storageTypes", variables).map(|types| {
        types
            .iter()
            .filter_map(|t| parse_storage_type(t).ok())
            .collect::<Vec<_>>()
    });
    let mode = arg_str(field, "mode", variables).map(|m| match m.to_lowercase().as_str() {
        "or" => crate::fulltext::SearchMode::Or,
        "phrase" => crate::fulltext::SearchMode::Phrase,
        _ => crate::fulltext::SearchMode::And,
    });
    let request = SearchRequest {
        query: arg_str(field, "query", variables),
        query_vector: arg_json(field, "queryVector", variables),
        mode,
        storage_types,
        tables: arg_list_of_strings(field, "tables", variables),
        limit: arg_i64(field, "limit", variables).map(|l| l.max(0) as u64),
        offset: arg_i64(field, "offset", variables).map(|o| o.max(0) as u64),
    };
    SearchService::search(db, &request)
        .await
        .map_err(|e| GraphQLError {
            message: e.to_string(),
            path: vec![],
        })
}

/// Execute an insert/update/delete mutation through the query engine.
async fn run_mutation(
    db: &PrimusDB,
    field: &Field,
    variables: &BTreeMap<String, serde_json::Value>,
) -> std::result::Result<serde_json::Value, GraphQLError> {
    let err = |message: String| GraphQLError {
        message,
        path: vec![],
    };
    let storage_type = arg_str(field, "storageType", variables).ok_or_else(|| {
        err(format!(
            "field '{}' requires a 'storageType' argument",
            field.name
        ))
    })?;
    let st = parse_storage_type(&storage_type)?;
    let table = arg_str(field, "table", variables).ok_or_else(|| {
        err(format!(
            "field '{}' requires a 'table' argument",
            field.name
        ))
    })?;
    let data = arg_json(field, "data", variables);
    let conditions = arg_json(field, "conditions", variables);

    let operation = match field.name.as_str() {
        "insert" => {
            if data.is_none() {
                return Err(err("field 'insert' requires a 'data' argument".to_string()));
            }
            QueryOperation::Create
        }
        "update" => {
            if data.is_none() {
                return Err(err("field 'update' requires a 'data' argument".to_string()));
            }
            QueryOperation::Update
        }
        _ => QueryOperation::Delete,
    };

    let query = Query {
        storage_type: st,
        operation,
        table,
        conditions,
        data,
        limit: None,
        offset: None,
        namespace: None,
    };
    let result = db
        .execute_query(query)
        .await
        .map_err(|e| err(e.to_string()))?;
    match result {
        crate::QueryResult::Insert(n) => Ok(serde_json::json!(n)),
        crate::QueryResult::Update(n) => Ok(serde_json::json!(n)),
        crate::QueryResult::Delete(n) => Ok(serde_json::json!(n)),
        other => Ok(serde_json::json!({ "result": format!("{other:?}") })),
    }
}

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

/// Resolve the raw value of an argument (variables substituted), if present.
fn arg(
    field: &Field,
    name: &str,
    variables: &BTreeMap<String, serde_json::Value>,
) -> Option<Value> {
    field
        .arguments
        .iter()
        .find(|a| a.name == name)
        .and_then(|a| a.value.resolve_variable(variables))
}

/// Resolve an argument as a string.
fn arg_str(
    field: &Field,
    name: &str,
    variables: &BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    match arg(field, name, variables)? {
        Value::String(s) => Some(s),
        Value::Enum(s) => Some(s),
        Value::Int(i) => Some(i.to_string()),
        Value::Float(f) => Some(f.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Resolve an argument as a signed integer.
fn arg_i64(
    field: &Field,
    name: &str,
    variables: &BTreeMap<String, serde_json::Value>,
) -> Option<i64> {
    match arg(field, name, variables)? {
        Value::Int(i) => Some(i),
        Value::Float(f) => Some(f as i64),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Resolve an argument as JSON: inline objects/lists pass through; strings are
/// parsed as JSON text.
fn arg_json(
    field: &Field,
    name: &str,
    variables: &BTreeMap<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    match arg(field, name, variables)? {
        Value::Null => Some(serde_json::Value::Null),
        Value::Int(i) => Some(serde_json::json!(i)),
        Value::Float(f) => Some(serde_json::json!(f)),
        Value::Bool(b) => Some(serde_json::json!(b)),
        Value::String(s) => serde_json::from_str(&s).ok(),
        Value::Enum(s) => Some(serde_json::json!(s)),
        Value::List(items) => Some(serde_json::Value::Array(
            items.into_iter().map(|v| v.into_json()).collect(),
        )),
        Value::Object(map) => Some(serde_json::Value::Object(
            map.into_iter().map(|(k, v)| (k, v.into_json())).collect(),
        )),
        Value::Variable(_) => None,
    }
}

/// Resolve an argument as a list of strings.
fn arg_list_of_strings(
    field: &Field,
    name: &str,
    variables: &BTreeMap<String, serde_json::Value>,
) -> Option<Vec<String>> {
    match arg(field, name, variables)? {
        Value::List(items) => Some(
            items
                .iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Enum(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
        ),
        Value::String(s) => Some(vec![s]),
        _ => None,
    }
}

/// Require a field that returns an object to have a selection set.
fn require_selection(field: &Field, ctx: &Ctx) -> std::result::Result<(), GraphQLError> {
    if field.selection_set.is_empty() {
        let type_name = match ctx {
            Ctx::QueryRoot => "Query",
            Ctx::MutationRoot => "Mutation",
            Ctx::EngineInfo { .. } => "EngineInfo",
            Ctx::Table { .. } => "Table",
            Ctx::Record { .. } => "Record",
            Ctx::SearchResponse { .. } => "SearchResponse",
            Ctx::SearchHit(_) => "SearchHit",
        };
        return Err(GraphQLError {
            message: format!(
                "field '{}' on {type_name} must have a selection set",
                field.name
            ),
            path: vec![],
        });
    }
    Ok(())
}

/// Parse a storage type name with a clear GraphQL-flavoured error.
fn parse_storage_type(name: &str) -> std::result::Result<StorageType, GraphQLError> {
    name.parse::<StorageType>().map_err(|_| GraphQLError {
        message: format!(
            "Unknown storage type '{name}'. Valid values: \
             Relational, Document, KeyValue, Columnar, Vector, TimeSeries"
        ),
        path: vec![],
    })
}
