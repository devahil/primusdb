/*
 * PrimusDB GraphQL Service — AST
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.0.0
 */

//! Abstract syntax tree for the GraphQL subset supported by PrimusDB.
//!
//! The AST mirrors the GraphQL June 2018 spec for the subset the executor
//! understands: queries and mutations with fields, aliases, arguments,
//! variables, lists, objects and scalars. Fragments, directives,
//! subscriptions and full introspection are intentionally not modelled here
//! (see [`crate::graphql`] for the honest description of the supported set).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A GraphQL value literal, possibly referencing a variable.
///
/// Variables are resolved by the executor against the operation's variable
/// map before field arguments are read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// The `null` literal.
    Null,
    /// A signed 64-bit integer literal.
    Int(i64),
    /// A 64-bit float literal.
    Float(f64),
    /// A string literal.
    String(String),
    /// A boolean literal.
    Bool(bool),
    /// An enum value (a bare name without quotes), kept as a string.
    Enum(String),
    /// A list literal.
    List(Vec<Value>),
    /// An object literal (input object).
    Object(BTreeMap<String, Value>),
    /// A variable reference `$name`.
    Variable(String),
}

impl Value {
    /// Resolve a variable reference; returns `None` when the variable is
    /// unknown. Non-variable values are resolved eagerly.
    pub fn resolve_variable(
        &self,
        variables: &BTreeMap<String, serde_json::Value>,
    ) -> Option<Value> {
        match self {
            Value::Variable(name) => variables.get(name).map(|v| Value::from_json(v.clone())),
            _ => Some(self.clone()),
        }
    }

    /// Convert a JSON value into a GraphQL AST value.
    pub fn from_json(value: serde_json::Value) -> Value {
        match value {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::String(n.to_string())
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(items) => {
                Value::List(items.into_iter().map(Value::from_json).collect())
            }
            serde_json::Value::Object(map) => Value::Object(
                map.into_iter()
                    .map(|(k, v)| (k, Value::from_json(v)))
                    .collect(),
            ),
        }
    }

    /// Convert a resolved AST value back to JSON for response building.
    pub fn into_json(self) -> serde_json::Value {
        match self {
            Value::Null => serde_json::Value::Null,
            Value::Int(i) => serde_json::json!(i),
            Value::Float(f) => serde_json::json!(f),
            Value::String(s) => serde_json::json!(s),
            Value::Bool(b) => serde_json::json!(b),
            Value::Enum(e) => serde_json::json!(e),
            Value::List(items) => {
                serde_json::Value::Array(items.into_iter().map(Value::into_json).collect())
            }
            Value::Object(map) => serde_json::Value::Object(
                map.into_iter().map(|(k, v)| (k, v.into_json())).collect(),
            ),
            Value::Variable(_) => serde_json::Value::Null,
        }
    }
}

/// A GraphQL type reference (`Int`, `[String]`, `String!`, `[Int!]!`, ...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeRef {
    /// A named (scalar or object) type.
    Named(String),
    /// A list type.
    List(Box<TypeRef>),
    /// A non-null wrapper.
    NonNull(Box<TypeRef>),
}

/// A single field argument `name: value`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Argument {
    /// Argument name.
    pub name: String,
    /// Argument value (may reference a variable).
    pub value: Value,
}

/// A field selection, possibly aliased and/or with a nested selection set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// Response key when an alias is present (`alias: name`).
    pub alias: Option<String>,
    /// Field name being selected.
    pub name: String,
    /// Field arguments.
    pub arguments: Vec<Argument>,
    /// Nested selection set, when the field returns an object or list.
    pub selection_set: Vec<Field>,
}

/// A variable definition in an operation header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableDefinition {
    /// Variable name (without the `$`).
    pub name: String,
    /// Declared type.
    pub var_type: TypeRef,
    /// Default value, when provided.
    pub default: Option<Value>,
}

/// Operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    /// Read-only operation.
    Query,
    /// Write operation.
    Mutation,
}

/// A parsed GraphQL operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    /// `query` or `mutation`.
    pub operation_type: OperationType,
    /// Optional operation name (required when a document has several).
    pub name: Option<String>,
    /// Variable definitions.
    pub variable_definitions: Vec<VariableDefinition>,
    /// Root selection set.
    pub selection_set: Vec<Field>,
}

/// A parsed GraphQL document (one or more operations).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Operations defined in the document.
    pub operations: Vec<Operation>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_roundtrip() {
        let v = Value::from_json(serde_json::json!({"a": [1, true, null, "x"]}));
        let json = v.into_json();
        assert_eq!(json, serde_json::json!({"a": [1, true, null, "x"]}));
    }

    #[test]
    fn test_variable_resolution() {
        let vars: BTreeMap<String, serde_json::Value> =
            [("limit".to_string(), serde_json::json!(5))]
                .into_iter()
                .collect();
        assert_eq!(
            Value::Variable("limit".to_string()).resolve_variable(&vars),
            Some(Value::Int(5))
        );
        assert_eq!(
            Value::Variable("missing".to_string()).resolve_variable(&vars),
            None
        );
    }
}
