/*
 * PrimusDB GraphQL Service
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.0.0
 */

//! # GraphQL service for PrimusDB
//!
//! A self-contained GraphQL endpoint that exposes the storage engines and the
//! unified search service through a standard `POST` JSON GraphQL request.
//! It is implemented in-crate (no external GraphQL dependency): a
//! recursive-descent parser produces an AST which the executor resolves
//! strictly through the service layer ([`PrimusDB::execute_query`],
//! [`PrimusDB::storage_engine`], [`crate::search::SearchService`]).
//!
//! ## Supported GraphQL surface (honest subset)
//!
//! - Operations: `query` and `mutation` (anonymous shorthand supported).
//! - Fields with aliases, arguments, nested selection sets.
//! - Scalar literals: `Int`, `Float`, `String`, `Boolean`, `null`, enums-as-strings.
//! - Lists, input objects, variable definitions with defaults, variable values.
//! - `__typename`.
//!
//! Not supported (explicit errors, never silent): fragments, directives,
//! subscriptions, interfaces, unions, and full introspection.
//!
//! ## Schema
//!
//! ```graphql
//! type Query {
//!   engines: [EngineInfo!]!
//!   table(storageType: String!, name: String!): Table!
//!   tables(storageType: String!): [Table!]!
//!   search(query: String, queryVector: String, mode: String,
//!          storageTypes: [String], tables: [String], limit: Int, offset: Int): SearchResponse!
//! }
//!
//! type Mutation {
//!   insert(storageType: String!, table: String!, data: JSON!): Int!
//!   update(storageType: String!, table: String!, data: JSON!,
//!          conditions: JSON): Int!
//!   delete(storageType: String!, table: String!, conditions: JSON): Int!
//! }
//!
//! type EngineInfo { name: String!, tables: [String!]! }
//! type Table {
//!   name: String!
//!   count: Int!
//!   records(limit: Int, offset: Int, conditions: JSON): [Record!]!
//!   record(id: String!): Record
//! }
//! type Record { id: String!, data: JSON!, metadata: JSON! }
//! type SearchResponse { query: String!, total: Int!, hits: [SearchHit!]! }
//! type SearchHit {
//!   engine: String!, table: String!, id: String!,
//!   score: Float!, similarity: Float, record: JSON!
//! }
//! ```
//!
//! The `JSON` scalar is passed as a JSON string argument (e.g.
//! `data: "{\"name\":\"ada\"}"`) or as an inline object/list.

pub mod ast;
pub mod executor;
pub mod parser;

pub use ast::{Argument, Document, Field, Operation, OperationType, TypeRef, Value};
pub use executor::{GraphQLError, GraphQLExecutor, GraphQLRequest, GraphQLResponse};
pub use parser::ParseError;

use serde::{Deserialize, Serialize};

/// GraphQL service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLConfig {
    /// Whether the GraphQL endpoint is enabled (default `true`).
    pub enabled: bool,
}

impl Default for GraphQLConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// A compact SDL description of the supported schema, used by the `GET`
/// endpoint for human consumption (honest about the supported subset).
pub const SCHEMA_SDL: &str = "\
schema { query: Query, mutation: Mutation }

type Query {
  engines: [EngineInfo!]!
  table(storageType: String!, name: String!): Table!
  tables(storageType: String!): [Table!]!
  search(query: String, queryVector: String, mode: String, storageTypes: [String], tables: [String], limit: Int, offset: Int): SearchResponse!
}

type Mutation {
  insert(storageType: String!, table: String!, data: JSON!): Int!
  update(storageType: String!, table: String!, data: JSON!, conditions: JSON): Int!
  delete(storageType: String!, table: String!, conditions: JSON): Int!
}

type EngineInfo { name: String!, tables: [String!]! }
type Table { name: String!, count: Int!, records(limit: Int, offset: Int, conditions: JSON): [Record!]!, record(id: String!): Record }
type Record { id: String!, data: JSON!, metadata: JSON! }
type SearchResponse { query: String!, total: Int!, hits: [SearchHit!]! }
type SearchHit { engine: String!, table: String!, id: String!, score: Float!, similarity: Float, record: JSON! }

scalar JSON

# Supported subset: queries, mutations, fields, aliases, arguments, variables.
# Not supported: fragments, directives, subscriptions, interfaces, unions,
# full introspection (only __typename).
";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PrimusDB, PrimusDBConfig, Query, QueryOperation, StorageType};
    use std::collections::BTreeMap;

    fn setup() -> (PrimusDB, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_string_lossy().into_owned();
        config.integrity.genesis_required = false;
        (PrimusDB::new(config).unwrap(), dir)
    }

    async fn seed(db: &PrimusDB) {
        for doc in [
            serde_json::json!({"title": "cargo internals", "body": "rust borrow checker"}),
            serde_json::json!({"title": "gardening", "body": "tomatoes in spring"}),
        ] {
            db.execute_query(Query {
                storage_type: StorageType::Document,
                operation: QueryOperation::Create,
                table: "notes".to_string(),
                conditions: None,
                data: Some(doc),
                limit: None,
                offset: None,
                namespace: None,
            })
            .await
            .unwrap();
        }
        db.execute_query(Query {
            storage_type: StorageType::Vector,
            operation: QueryOperation::Create,
            table: "emb".to_string(),
            conditions: None,
            data: Some(serde_json::json!({"id": "a", "vector": [1.0, 0.0]})),
            limit: None,
            offset: None,
            namespace: None,
        })
        .await
        .unwrap();
    }

    fn request(query: &str) -> GraphQLRequest {
        GraphQLRequest {
            query: query.to_string(),
            operation_name: None,
            variables: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn test_engines_query() {
        let (db, _dir) = setup();
        seed(&db).await;
        let resp = GraphQLExecutor::execute(&db, &request("{ engines { name tables } }")).await;
        assert!(
            resp.errors.is_empty(),
            "unexpected errors: {:?}",
            resp.errors
        );
        let data = resp.data.unwrap();
        let engines = data["engines"].as_array().unwrap();
        let names: Vec<&str> = engines.iter().filter_map(|e| e["name"].as_str()).collect();
        assert!(names.contains(&"Document"), "{names:?}");
        let document_eng = engines.iter().find(|e| e["name"] == "Document").unwrap();
        assert!(document_eng["tables"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t == "notes"));
    }

    #[tokio::test]
    async fn test_table_records_query() {
        let (db, _dir) = setup();
        seed(&db).await;
        let resp = GraphQLExecutor::execute(
            &db,
            &request(
                r#"{
                    table(storageType: "Document", name: "notes") {
                        name
                        count
                        records { id data }
                    }
                }"#,
            ),
        )
        .await;
        assert!(
            resp.errors.is_empty(),
            "unexpected errors: {:?}",
            resp.errors
        );
        let data = resp.data.unwrap();
        assert_eq!(data["table"]["name"], "notes");
        assert_eq!(data["table"]["count"], 2);
        assert_eq!(data["table"]["records"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_search_query_with_variables() {
        let (db, _dir) = setup();
        seed(&db).await;
        let mut variables = BTreeMap::new();
        variables.insert("q".to_string(), serde_json::json!("rust"));
        variables.insert("lim".to_string(), serde_json::json!(5));
        let resp = GraphQLExecutor::execute(
            &db,
            &GraphQLRequest {
                query: r#"
                    query SearchNotes($q: String!, $lim: Int) {
                        search(query: $q, limit: $lim) {
                            query
                            total
                            hits { table score }
                        }
                    }
                "#
                .to_string(),
                operation_name: Some("SearchNotes".to_string()),
                variables,
            },
        )
        .await;
        assert!(
            resp.errors.is_empty(),
            "unexpected errors: {:?}",
            resp.errors
        );
        let data = resp.data.unwrap();
        assert_eq!(data["search"]["query"], "rust");
        assert_eq!(data["search"]["total"], 1);
    }

    #[tokio::test]
    async fn test_parser_errors_are_surfaced() {
        let (db, _dir) = setup();
        let resp = GraphQLExecutor::execute(&db, &request("fragment F on T { id }")).await;
        assert!(resp.data.is_none());
        assert_eq!(resp.errors.len(), 1);
        assert!(resp.errors[0].message.contains("fragments"));
    }

    #[tokio::test]
    async fn test_unknown_field_error_path() {
        let (db, _dir) = setup();
        let resp = GraphQLExecutor::execute(&db, &request("{ nonexistent }")).await;
        assert!(resp.data.is_none());
        assert_eq!(resp.errors[0].path, vec!["nonexistent"]);
    }

    #[tokio::test]
    async fn test_mutation_insert_then_query() {
        let (db, _dir) = setup();
        let resp = GraphQLExecutor::execute(
            &db,
            &request(
                r#"mutation {
                    insert(storageType: "Document", table: "widgets", data: "{\"name\":\"gear\",\"qty\":3}")
                }"#,
            ),
        )
        .await;
        assert!(
            resp.errors.is_empty(),
            "unexpected errors: {:?}",
            resp.errors
        );
        assert_eq!(resp.data.unwrap()["insert"], 1);

        let resp = GraphQLExecutor::execute(
            &db,
            &request(
                r#"{
                    table(storageType: "Document", name: "widgets") {
                        records { data }
                    }
                }"#,
            ),
        )
        .await;
        let data = resp.data.unwrap();
        assert_eq!(data["table"]["records"][0]["data"]["name"], "gear");
    }

    #[tokio::test]
    async fn test_mutation_requires_storage_type() {
        let (db, _dir) = setup();
        let resp = GraphQLExecutor::execute(
            &db,
            &request(r#"mutation { insert(table: "x", data: "{}") }"#),
        )
        .await;
        assert!(resp.errors[0].message.contains("storageType"));
    }

    #[tokio::test]
    async fn test_typename() {
        let (db, _dir) = setup();
        let resp = GraphQLExecutor::execute(&db, &request("{ __typename }")).await;
        assert_eq!(resp.data.unwrap()["__typename"], "Query");
    }
}
