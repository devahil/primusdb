/*
 * PrimusDB Rust Native Driver
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.3.1-alpha - ER Model parity, cascade truncate, RETURNING, GROUP BY
 */

use primusdb::cluster::ClusterStatusInfo;
use primusdb::governor::{EnforcementAction, GovernorMetricsSnapshot, GovernorStatus, WorkloadType};
use primusdb::query::{QueryLanguage, UqlQuery};
use primusdb::{PrimusDB, PrimusDBConfig, Query, QueryOperation, Result, StorageType};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

/// Native Rust driver for PrimusDB
pub struct NativeDriver {
    db: Arc<PrimusDB>,
}

impl NativeDriver {
    /// Create a new native driver instance
    pub fn new(config: PrimusDBConfig) -> Result<Self> {
        let db = Arc::new(PrimusDB::new(config)?);
        Ok(Self { db })
    }

    /// Get reference to underlying database
    pub fn db(&self) -> &Arc<PrimusDB> {
        &self.db
    }

    /// Execute a raw query
    pub async fn execute_query(&self, query: Query) -> Result<serde_json::Value> {
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Create a table/collection
    pub async fn create_table(
        &self,
        storage_type: StorageType,
        table: &str,
        schema: serde_json::Value,
    ) -> Result<()> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Create,
            table: table.to_string(),
            conditions: None,
            data: Some(schema),
            limit: None,
            offset: None,
            namespace: None,
        };
        self.db.execute_query(query).await?;
        Ok(())
    }

    /// Insert data
    pub async fn insert(
        &self,
        storage_type: StorageType,
        table: &str,
        data: serde_json::Value,
    ) -> Result<u64> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Create,
            table: table.to_string(),
            conditions: None,
            data: Some(data),
            limit: None,
            offset: None,
            namespace: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Insert(count) => Ok(count),
            _ => Ok(0),
        }
    }

    /// Select data
    pub async fn select(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<serde_json::Value>> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Read,
            table: table.to_string(),
            conditions,
            data: None,
            limit,
            offset,
            namespace: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Select(records) => {
                Ok(records.into_iter().map(|r| r.data).collect())
            }
            _ => Ok(vec![]),
        }
    }

    /// Update data
    pub async fn update(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
        data: serde_json::Value,
    ) -> Result<u64> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Update,
            table: table.to_string(),
            conditions,
            data: Some(data),
            limit: None,
            offset: None,
            namespace: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Update(count) => Ok(count),
            _ => Ok(0),
        }
    }

    /// Delete data
    pub async fn delete(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
    ) -> Result<u64> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Delete,
            table: table.to_string(),
            conditions,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Delete(count) => Ok(count),
            _ => Ok(0),
        }
    }

    /// Analyze data patterns
    pub async fn analyze(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Analyze,
            table: table.to_string(),
            conditions,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Explain(analysis) => Ok(serde_json::Value::String(analysis)),
            _ => Ok(serde_json::Value::Null),
        }
    }

    /// Make AI predictions
    pub async fn predict(
        &self,
        storage_type: StorageType,
        table: &str,
        data: serde_json::Value,
        prediction_type: &str,
    ) -> Result<serde_json::Value> {
        let predict_data = serde_json::json!({
            "data": data,
            "prediction_type": prediction_type
        });

        let query = Query {
            storage_type,
            operation: QueryOperation::Predict,
            table: table.to_string(),
            conditions: None,
            data: Some(predict_data),
            limit: None,
            offset: None,
            namespace: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Select(predictions) => Ok(serde_json::to_value(predictions)?),
            _ => Ok(serde_json::Value::Null),
        }
    }

    /// Perform vector similarity search
    pub async fn vector_search(
        &self,
        _table: &str,
        _query_vector: Vec<f32>,
        _limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        // Vector search would be implemented as a special query operation
        // For now, return empty result
        Ok(vec![])
    }

    /// Perform data clustering
    pub async fn cluster(
        &self,
        _storage_type: StorageType,
        _table: &str,
        _params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // Clustering would be implemented as a special query operation
        // For now, return empty result
        Ok(serde_json::Value::Null)
    }

    /// Add a column to a relational table
    pub async fn add_column(
        &self,
        storage_type: StorageType,
        table: &str,
        field: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::AlterTableAddColumn,
            table: table.to_string(),
            conditions: None,
            data: Some(field),
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Drop a column from a relational table
    pub async fn drop_column(
        &self,
        storage_type: StorageType,
        table: &str,
        column_name: &str,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::AlterTableDropColumn,
            table: table.to_string(),
            conditions: None,
            data: Some(serde_json::Value::String(column_name.to_string())),
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Modify a column definition
    pub async fn modify_column(
        &self,
        storage_type: StorageType,
        table: &str,
        field: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::AlterTableModifyColumn,
            table: table.to_string(),
            conditions: None,
            data: Some(field),
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Add a constraint to a relational table
    pub async fn add_constraint(
        &self,
        storage_type: StorageType,
        table: &str,
        constraint: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::AlterTableAddConstraint,
            table: table.to_string(),
            conditions: None,
            data: Some(constraint),
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Drop a constraint from a relational table
    pub async fn drop_constraint(
        &self,
        storage_type: StorageType,
        table: &str,
        constraint_name: &str,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::AlterTableDropConstraint,
            table: table.to_string(),
            conditions: None,
            data: Some(serde_json::Value::String(constraint_name.to_string())),
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Rename a relational table
    pub async fn rename_table(
        &self,
        storage_type: StorageType,
        table: &str,
        new_name: &str,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::RenameTable,
            table: table.to_string(),
            conditions: None,
            data: Some(serde_json::Value::String(new_name.to_string())),
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Create a sequence
    pub async fn create_sequence(
        &self,
        storage_type: StorageType,
        sequence: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::CreateSequence,
            table: String::new(),
            conditions: None,
            data: Some(sequence),
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Drop a sequence
    pub async fn drop_sequence(
        &self,
        storage_type: StorageType,
        name: &str,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::DropSequence,
            table: name.to_string(),
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Get next value from a sequence
    pub async fn nextval(
        &self,
        storage_type: StorageType,
        name: &str,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::NextVal,
            table: name.to_string(),
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Get current value from a sequence
    pub async fn currval(
        &self,
        storage_type: StorageType,
        name: &str,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::CurrVal,
            table: name.to_string(),
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Set a sequence value
    pub async fn setval(
        &self,
        storage_type: StorageType,
        name: &str,
        value: i64,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::SetVal,
            table: name.to_string(),
            conditions: None,
            data: Some(serde_json::json!(value)),
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Create a view
    pub async fn create_view(
        &self,
        storage_type: StorageType,
        view: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::CreateView,
            table: String::new(),
            conditions: None,
            data: Some(view),
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Drop a view
    pub async fn drop_view(
        &self,
        storage_type: StorageType,
        name: &str,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::DropView,
            table: name.to_string(),
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Refresh a view
    pub async fn refresh_view(
        &self,
        storage_type: StorageType,
        name: &str,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::RefreshView,
            table: name.to_string(),
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Create a trigger on a table
    pub async fn create_trigger(
        &self,
        storage_type: StorageType,
        table: &str,
        trigger: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::CreateTrigger,
            table: table.to_string(),
            conditions: None,
            data: Some(trigger),
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Drop a trigger from a table
    pub async fn drop_trigger(
        &self,
        storage_type: StorageType,
        table: &str,
        trigger_name: &str,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::DropTrigger,
            table: table.to_string(),
            conditions: None,
            data: Some(serde_json::Value::String(trigger_name.to_string())),
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Get information schema tables
    pub async fn info_schema_tables(&self, storage_type: StorageType) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::InformationSchemaTables,
            table: String::new(),
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Get information schema columns for a table
    pub async fn info_schema_columns(
        &self,
        storage_type: StorageType,
        table: &str,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::InformationSchemaColumns,
            table: table.to_string(),
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Get information schema constraints for a table
    pub async fn info_schema_constraints(
        &self,
        storage_type: StorageType,
        table: &str,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::InformationSchemaConstraints,
            table: table.to_string(),
            conditions: None,
            data: None,
            limit: None,
            offset: None,
            namespace: None,
        };
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Get cluster status
    pub async fn get_cluster_status(&self) -> Result<ClusterStatusInfo> {
        self.db.get_cluster_status().await
    }

    /// Get list of known cluster nodes with their metadata
    pub async fn cluster_nodes(&self) -> Result<serde_json::Value> {
        let status = self.db.get_cluster_status().await?;
        Ok(serde_json::json!([{
            "node_id": status.node_id,
            "health": status.health_status,
            "is_leader": status.is_leader,
            "leader_id": status.leader_id,
            "cluster_size": status.cluster_size,
            "alive_count": status.alive_count,
        }]))
    }

    /// Get route decision for a shard key (in embedded mode, returns self)
    pub async fn route_request(
        &self,
        shard_key: Option<&str>,
        _preferred_nodes: Option<&[String]>,
    ) -> Result<serde_json::Value> {
        let status = self.db.get_cluster_status().await?;
        Ok(serde_json::json!({
            "node_id": status.node_id,
            "strategy": "local",
            "shard_key": shard_key,
            "is_leader": status.is_leader,
            "health": status.health_status,
        }))
    }

    // ==================== Resource Governor ====================

    /// Start a new governor-tracked execution
    pub async fn governor_start_execution(
        &self,
        namespace: String,
        workload_type: WorkloadType,
        user: Option<&str>,
        role: Option<&str>,
    ) -> Result<String> {
        let handle = self
            .db
            .governor_engine()
            .start_execution(namespace, workload_type, user, role)
            .await;
        Ok(handle.id().to_string())
    }

    /// Finish a governor-tracked execution
    pub async fn governor_finish_execution(&self, execution_id: Uuid) -> Result<()> {
        self.db
            .governor_engine()
            .finish_execution(execution_id)
            .await;
        Ok(())
    }

    /// Check if the governor engine is enabled
    pub async fn governor_is_enabled(&self) -> Result<bool> {
        Ok(self.db.governor_engine().is_enabled().await)
    }

    /// Get the current governor status
    pub async fn governor_status(&self) -> Result<GovernorStatus> {
        Ok(self.db.governor_engine().status().await)
    }

    /// Get a metrics snapshot from the governor
    pub async fn governor_metrics(&self) -> Result<GovernorMetricsSnapshot> {
        Ok(self.db.governor_engine().metrics_snapshot().await)
    }

    /// Check a resource limit against the governor
    pub async fn governor_check_limit(
        &self,
        execution_id: Uuid,
        field: &str,
        current: u64,
        limit: Option<u64>,
    ) -> Result<EnforcementAction> {
        self.db
            .governor_engine()
            .check_limit(execution_id, field, current, limit)
            .await
            .map_err(|e| primusdb::Error::GovernorError(e))
    }

    // ==================== UQL / SQL Execution ====================

    /// Execute a raw SQL query through the UQL engine
    pub fn execute_sql(
        &self,
        sql: &str,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<primusdb::query::UqlResult> {
        let uql_query = UqlQuery {
            query: sql.to_string(),
            query_type: QueryLanguage::Sql,
            parameters: params,
        };
        self.db.uql_execute_query(&uql_query)
    }

    // ==================== ER Model Features (v1.2.2+) ====================

    /// Truncate a table, optionally cascading to dependent tables
    pub async fn truncate_table(
        &self,
        storage_type: StorageType,
        table: &str,
        cascade: bool,
    ) -> Result<u64> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Truncate,
            table: table.to_string(),
            conditions: None,
            data: Some(serde_json::json!({"cascade": cascade})),
            limit: None,
            offset: None,
            namespace: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Truncate(count) => Ok(count),
            _ => Ok(0),
        }
    }

    /// Insert data and return specified columns
    pub async fn insert_returning(
        &self,
        _storage_type: StorageType,
        table: &str,
        data: serde_json::Value,
        returning: &[&str],
    ) -> Result<Vec<serde_json::Value>> {
        let obj = data.as_object().ok_or_else(|| {
            primusdb::Error::DatabaseError("Data must be a JSON object".to_string())
        })?;
        let cols: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
        let vals: Vec<String> = obj
            .values()
            .map(|v| match v {
                serde_json::Value::String(s) => format!("'{}'", s),
                serde_json::Value::Null => "NULL".to_string(),
                _ => v.to_string(),
            })
            .collect();
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({}) RETURNING {}",
            table,
            cols.join(", "),
            vals.join(", "),
            returning.join(", ")
        );
        let result = self.execute_sql(&sql, None)?;
        Ok(result.records.into_iter().map(|r| r.data).collect())
    }

    /// Update data and return specified columns
    pub async fn update_returning(
        &self,
        _storage_type: StorageType,
        table: &str,
        conditions: serde_json::Value,
        data: serde_json::Value,
        returning: &[&str],
    ) -> Result<Vec<serde_json::Value>> {
        let data_obj = data.as_object().ok_or_else(|| {
            primusdb::Error::DatabaseError("Data must be a JSON object".to_string())
        })?;
        let conds_obj = conditions.as_object().ok_or_else(|| {
            primusdb::Error::DatabaseError("Conditions must be a JSON object".to_string())
        })?;
        let set_clause: Vec<String> = data_obj
            .iter()
            .map(|(k, v)| match v {
                serde_json::Value::String(s) => format!("{} = '{}'", k, s),
                serde_json::Value::Null => format!("{} = NULL", k),
                _ => format!("{} = {}", k, v),
            })
            .collect();
        let where_clause: Vec<String> = conds_obj
            .iter()
            .map(|(k, v)| match v {
                serde_json::Value::String(s) => format!("{} = '{}'", k, s),
                serde_json::Value::Null => format!("{} IS NULL", k),
                _ => format!("{} = {}", k, v),
            })
            .collect();
        let sql = format!(
            "UPDATE {} SET {} WHERE {} RETURNING {}",
            table,
            set_clause.join(", "),
            where_clause.join(" AND "),
            returning.join(", ")
        );
        let result = self.execute_sql(&sql, None)?;
        Ok(result.records.into_iter().map(|r| r.data).collect())
    }

    /// Delete data and return specified columns
    pub async fn delete_returning(
        &self,
        _storage_type: StorageType,
        table: &str,
        conditions: serde_json::Value,
        returning: &[&str],
    ) -> Result<Vec<serde_json::Value>> {
        let conds_obj = conditions.as_object().ok_or_else(|| {
            primusdb::Error::DatabaseError("Conditions must be a JSON object".to_string())
        })?;
        let where_clause: Vec<String> = conds_obj
            .iter()
            .map(|(k, v)| match v {
                serde_json::Value::String(s) => format!("{} = '{}'", k, s),
                serde_json::Value::Null => format!("{} IS NULL", k),
                _ => format!("{} = {}", k, v),
            })
            .collect();
        let sql = format!(
            "DELETE FROM {} WHERE {} RETURNING {}",
            table,
            where_clause.join(" AND "),
            returning.join(", ")
        );
        let result = self.execute_sql(&sql, None)?;
        Ok(result.records.into_iter().map(|r| r.data).collect())
    }

    /// Select data with GROUP BY, HAVING, DISTINCT, ORDER BY support
    #[allow(clippy::too_many_arguments)] // public API: 9 params for flexible query building
    pub async fn select_grouped(
        &self,
        _storage_type: StorageType,
        table: &str,
        columns: Option<&[&str]>,
        conditions: Option<serde_json::Value>,
        group_by: Option<&[&str]>,
        having: Option<serde_json::Value>,
        distinct: bool,
        order_by: Option<&[&str]>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<serde_json::Value>> {
        let mut sql = String::from("SELECT ");
        if distinct {
            sql.push_str("DISTINCT ");
        }
        if let Some(cols) = columns {
            sql.push_str(&cols.join(", "));
        } else {
            sql.push('*');
        }
        sql.push_str(&format!(" FROM {}", table));

        if let Some(serde_json::Value::Object(conds)) = conditions {
            if !conds.is_empty() {
                let where_clause: Vec<String> = conds
                    .iter()
                    .map(|(k, v)| match v {
                        serde_json::Value::String(s) => format!("{} = '{}'", k, s),
                        serde_json::Value::Null => format!("{} IS NULL", k),
                        _ => format!("{} = {}", k, v),
                    })
                    .collect();
                sql.push_str(&format!(" WHERE {}", where_clause.join(" AND ")));
            }
        }

        if let Some(gb) = group_by {
            if !gb.is_empty() {
                sql.push_str(&format!(" GROUP BY {}", gb.join(", ")));
            }
        }

        if let Some(serde_json::Value::Object(h)) = having {
            if !h.is_empty() {
                let having_clause: Vec<String> = h
                    .iter()
                    .map(|(k, v)| match v {
                        serde_json::Value::String(s) => format!("{} = '{}'", k, s),
                        serde_json::Value::Null => format!("{} IS NULL", k),
                        _ => format!("{} = {}", k, v),
                    })
                    .collect();
                sql.push_str(&format!(" HAVING {}", having_clause.join(" AND ")));
            }
        }

        if let Some(ob) = order_by {
            if !ob.is_empty() {
                sql.push_str(&format!(" ORDER BY {}", ob.join(", ")));
            }
        }

        if let Some(l) = limit {
            sql.push_str(&format!(" LIMIT {}", l));
        }

        if let Some(o) = offset {
            sql.push_str(&format!(" OFFSET {}", o));
        }

        let result = self.execute_sql(&sql, None)?;
        Ok(result.records.into_iter().map(|r| r.data).collect())
    }

    /// Add a foreign key constraint to a relational table
    #[allow(clippy::too_many_arguments)] // public API: 8 params for FK definition
    pub async fn add_foreign_key(
        &self,
        storage_type: StorageType,
        table: &str,
        name: &str,
        column: &str,
        references_table: &str,
        references_column: &str,
        on_delete: &str,
        on_update: &str,
    ) -> Result<serde_json::Value> {
        let constraint = serde_json::json!({
            "name": name,
            "constraint_type": "ForeignKey",
            "fields": [column],
            "definition": {
                "references_table": references_table,
                "references_field": references_column,
                "on_delete": on_delete,
                "on_update": on_update,
            }
        });
        self.add_constraint(storage_type, table, constraint).await
    }

    /// Drop a foreign key constraint from a relational table
    pub async fn drop_foreign_key(
        &self,
        storage_type: StorageType,
        table: &str,
        constraint_name: &str,
    ) -> Result<serde_json::Value> {
        self.drop_constraint(storage_type, table, constraint_name)
            .await
    }
}

/// Error type for the Rust native driver
#[derive(Debug)]
pub enum DriverError {
    ExecutionError(String),
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::ExecutionError(msg) => write!(f, "Driver execution error: {}", msg),
        }
    }
}

impl std::error::Error for DriverError {}

impl From<primusdb::Error> for DriverError {
    fn from(e: primusdb::Error) -> Self {
        DriverError::ExecutionError(e.to_string())
    }
}

impl From<serde_json::Error> for DriverError {
    fn from(e: serde_json::Error) -> Self {
        DriverError::ExecutionError(e.to_string())
    }
}

/// High-level client wrapping Arc<PrimusDB>
pub struct PrimusDBClient {
    db: Arc<PrimusDB>,
}

impl PrimusDBClient {
    /// Create a new client, wrapping a PrimusDB instance in Arc
    pub fn new(db: PrimusDB) -> Self {
        Self { db: Arc::new(db) }
    }

    /// Create a client from an existing Arc<PrimusDB>
    pub fn from_arc(db: Arc<PrimusDB>) -> Self {
        Self { db }
    }

    /// Access the underlying PrimusDB
    pub fn db(&self) -> &Arc<PrimusDB> {
        &self.db
    }

    /// Create a prepared statement from a SQL template with `?` placeholders.
    /// Requires the client to be wrapped in `Arc` so the statement can share the reference.
    pub fn prepare(self: &Arc<Self>, sql: &str) -> PreparedStatement {
        PreparedStatement::new(self.clone(), sql)
    }
}

impl Clone for PrimusDBClient {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
        }
    }
}

/// A prepared SQL statement with bound parameters
pub struct PreparedStatement {
    sql: String,
    params: Vec<serde_json::Value>,
    client: Arc<PrimusDBClient>,
}

impl PreparedStatement {
    /// Create a new prepared statement bound to a client reference
    pub fn new(client: Arc<PrimusDBClient>, sql: &str) -> Self {
        Self {
            sql: sql.to_string(),
            params: Vec::new(),
            client,
        }
    }

    /// Bind an integer parameter at the given index (0-based)
    pub fn set_int(&mut self, index: usize, value: i64) {
        self.ensure_capacity(index);
        self.params[index] = serde_json::json!(value);
    }

    /// Bind a string parameter at the given index (0-based)
    pub fn set_string(&mut self, index: usize, value: &str) {
        self.ensure_capacity(index);
        self.params[index] = serde_json::json!(value);
    }

    /// Bind a double parameter at the given index (0-based)
    pub fn set_double(&mut self, index: usize, value: f64) {
        self.ensure_capacity(index);
        self.params[index] = serde_json::json!(value);
    }

    /// Bind a boolean parameter at the given index (0-based)
    pub fn set_bool(&mut self, index: usize, value: bool) {
        self.ensure_capacity(index);
        self.params[index] = serde_json::json!(value);
    }

    /// Bind a NULL parameter at the given index (0-based)
    pub fn set_null(&mut self, index: usize) {
        self.ensure_capacity(index);
        self.params[index] = serde_json::Value::Null;
    }

    /// Ensure the params vector is large enough for the given index
    fn ensure_capacity(&mut self, index: usize) {
        if index >= self.params.len() {
            self.params.resize(index + 1, serde_json::Value::Null);
        }
    }

    /// Replace `?` placeholders with bound parameter values
    pub fn build_sql(&self) -> String {
        let mut result = self.sql.clone();
        let mut param_idx = 0;
        while let Some(pos) = result.find('?') {
            if param_idx >= self.params.len() {
                break;
            }
            let value = &self.params[param_idx];
            let replacement = match value {
                serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                serde_json::Value::Null => "NULL".to_string(),
                _ => value.to_string(),
            };
            result.replace_range(pos..pos + 1, &replacement);
            param_idx += 1;
        }
        result
    }

    /// Execute the prepared statement, returning the full result as JSON
    pub fn execute(&self) -> std::result::Result<serde_json::Value, DriverError> {
        let sql = self.build_sql();
        let uql_query = primusdb::query::UqlQuery {
            query: sql,
            query_type: primusdb::query::QueryLanguage::Sql,
            parameters: None,
        };
        let result = self.client.db.uql_execute_query(&uql_query)?;
        Ok(serde_json::to_value(result)?)
    }

    /// Execute the prepared statement and return the resulting records
    pub fn execute_query(&self) -> std::result::Result<Vec<serde_json::Value>, DriverError> {
        let sql = self.build_sql();
        let uql_query = primusdb::query::UqlQuery {
            query: sql,
            query_type: primusdb::query::QueryLanguage::Sql,
            parameters: None,
        };
        let result = self.client.db.uql_execute_query(&uql_query)?;
        Ok(result.records.into_iter().map(|r| r.data).collect())
    }
}

/// Builder pattern for NativeDriver configuration
pub struct NativeDriverBuilder {
    config: PrimusDBConfig,
}

impl NativeDriverBuilder {
    pub fn new() -> Self {
        Self {
            config: PrimusDBConfig {
                storage: primusdb::StorageConfig {
                    data_dir: "./data".to_string(),
                    max_file_size: 1024 * 1024 * 1024, // 1GB
                    compression: primusdb::CompressionType::Lz4,
                    cache_size: 100 * 1024 * 1024, // 100MB
                },
                network: primusdb::NetworkConfig {
                    bind_address: "127.0.0.1".to_string(),
                    port: 8080,
                    max_connections: 1000,
                },
                security: primusdb::SecurityConfig {
                    encryption_enabled: false,
                    key_rotation_interval: 86400,
                    auth_required: false,
                },
                cluster: primusdb::ClusterConfig {
                    enabled: false,
                    node_id: "native-driver".to_string(),
                    discovery_servers: vec![],
                },
                namespaces: Default::default(),
                federation: None,
            },
        }
    }

    pub fn data_dir(mut self, path: &str) -> Self {
        self.config.storage.data_dir = path.to_string();
        self
    }

    pub fn max_file_size(mut self, size: u64) -> Self {
        self.config.storage.max_file_size = size;
        self
    }

    pub fn compression(mut self, compression: primusdb::CompressionType) -> Self {
        self.config.storage.compression = compression;
        self
    }

    pub fn cache_size(mut self, size: u64) -> Self {
        self.config.storage.cache_size = size as usize;
        self
    }

    pub fn bind_address(mut self, address: &str) -> Self {
        self.config.network.bind_address = address.to_string();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.config.network.port = port;
        self
    }

    pub fn max_connections(mut self, max: u32) -> Self {
        self.config.network.max_connections = max as usize;
        self
    }

    pub fn encryption_enabled(mut self, enabled: bool) -> Self {
        self.config.security.encryption_enabled = enabled;
        self
    }

    pub fn auth_required(mut self, required: bool) -> Self {
        self.config.security.auth_required = required;
        self
    }

    pub fn cluster_enabled(mut self, enabled: bool) -> Self {
        self.config.cluster.enabled = enabled;
        self
    }

    pub fn node_id(mut self, id: &str) -> Self {
        self.config.cluster.node_id = id.to_string();
        self
    }

    pub fn discovery_servers(mut self, servers: Vec<String>) -> Self {
        self.config.cluster.discovery_servers = servers;
        self
    }

    pub fn build(self) -> Result<NativeDriver> {
        NativeDriver::new(self.config)
    }
}

impl Default for NativeDriverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level collection abstraction
pub struct Collection {
    driver: NativeDriver,
    storage_type: StorageType,
    name: String,
}

impl Collection {
    pub fn new(driver: NativeDriver, storage_type: StorageType, name: &str) -> Self {
        Self {
            driver,
            storage_type,
            name: name.to_string(),
        }
    }

    pub async fn insert_one(&self, data: serde_json::Value) -> Result<u64> {
        self.driver
            .insert(self.storage_type, &self.name, data)
            .await
    }

    pub async fn find(
        &self,
        conditions: Option<serde_json::Value>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<serde_json::Value>> {
        self.driver
            .select(self.storage_type, &self.name, conditions, limit, offset)
            .await
    }

    pub async fn update_one(
        &self,
        conditions: Option<serde_json::Value>,
        data: serde_json::Value,
    ) -> Result<u64> {
        self.driver
            .update(self.storage_type, &self.name, conditions, data)
            .await
    }

    pub async fn delete_one(&self, conditions: Option<serde_json::Value>) -> Result<u64> {
        self.driver
            .delete(self.storage_type, &self.name, conditions)
            .await
    }

    pub async fn count(&self, conditions: Option<serde_json::Value>) -> Result<u64> {
        let results = self.find(conditions, Some(1000000), None).await?;
        Ok(results.len() as u64)
    }

    pub async fn analyze(
        &self,
        conditions: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        self.driver
            .analyze(self.storage_type, &self.name, conditions)
            .await
    }

    pub async fn predict(
        &self,
        data: serde_json::Value,
        prediction_type: &str,
    ) -> Result<serde_json::Value> {
        self.driver
            .predict(self.storage_type, &self.name, data, prediction_type)
            .await
    }

    pub async fn add_column(&self, field: serde_json::Value) -> Result<serde_json::Value> {
        self.driver
            .add_column(self.storage_type, &self.name, field)
            .await
    }

    pub async fn drop_column(&self, column_name: &str) -> Result<serde_json::Value> {
        self.driver
            .drop_column(self.storage_type, &self.name, column_name)
            .await
    }

    pub async fn modify_column(&self, field: serde_json::Value) -> Result<serde_json::Value> {
        self.driver
            .modify_column(self.storage_type, &self.name, field)
            .await
    }

    pub async fn add_constraint(&self, constraint: serde_json::Value) -> Result<serde_json::Value> {
        self.driver
            .add_constraint(self.storage_type, &self.name, constraint)
            .await
    }

    pub async fn drop_constraint(&self, constraint_name: &str) -> Result<serde_json::Value> {
        self.driver
            .drop_constraint(self.storage_type, &self.name, constraint_name)
            .await
    }

    pub async fn rename_table(&self, new_name: &str) -> Result<serde_json::Value> {
        self.driver
            .rename_table(self.storage_type, &self.name, new_name)
            .await
    }

    pub async fn create_trigger(&self, trigger: serde_json::Value) -> Result<serde_json::Value> {
        self.driver
            .create_trigger(self.storage_type, &self.name, trigger)
            .await
    }

    pub async fn drop_trigger(&self, trigger_name: &str) -> Result<serde_json::Value> {
        self.driver
            .drop_trigger(self.storage_type, &self.name, trigger_name)
            .await
    }

    pub async fn info_schema_columns(&self) -> Result<serde_json::Value> {
        self.driver
            .info_schema_columns(self.storage_type, &self.name)
            .await
    }

    pub async fn info_schema_constraints(&self) -> Result<serde_json::Value> {
        self.driver
            .info_schema_constraints(self.storage_type, &self.name)
            .await
    }
}

/// Database abstraction
pub struct Database {
    driver: NativeDriver,
}

impl Database {
    pub fn new(driver: NativeDriver) -> Self {
        Self { driver }
    }

    pub fn collection(&self, storage_type: StorageType, name: &str) -> Collection {
        Collection::new(self.driver.clone(), storage_type, name)
    }

    pub async fn create_table(
        &self,
        storage_type: StorageType,
        table: &str,
        schema: serde_json::Value,
    ) -> Result<()> {
        self.driver.create_table(storage_type, table, schema).await
    }

    pub async fn get_cluster_status(&self) -> Result<ClusterStatusInfo> {
        self.driver.get_cluster_status().await
    }

    pub async fn cluster_nodes(&self) -> Result<serde_json::Value> {
        self.driver.cluster_nodes().await
    }

    pub async fn route_request(
        &self,
        shard_key: Option<&str>,
        preferred_nodes: Option<&[String]>,
    ) -> Result<serde_json::Value> {
        self.driver.route_request(shard_key, preferred_nodes).await
    }
}

impl Clone for NativeDriver {
    fn clone(&self) -> Self {
        // Since we have Arc<PrimusDB>, we can share the reference
        Self {
            db: Arc::clone(&self.db),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_db() -> (NativeDriver, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = PrimusDBConfig {
            storage: primusdb::StorageConfig {
                data_dir: temp_dir.path().to_string_lossy().to_string(),
                max_file_size: 1024 * 1024 * 1024,
                compression: primusdb::CompressionType::Lz4,
                cache_size: 10 * 1024 * 1024,
            },
            network: primusdb::NetworkConfig {
                bind_address: "127.0.0.1".to_string(),
                port: 8080,
                max_connections: 100,
            },
            security: primusdb::SecurityConfig {
                encryption_enabled: false,
                key_rotation_interval: 86400,
                auth_required: false,
            },
            cluster: primusdb::ClusterConfig {
                enabled: false,
                node_id: "test-driver".to_string(),
                discovery_servers: vec![],
            },
            namespaces: Default::default(),
            federation: None,
        };

        let driver = NativeDriver::new(config).unwrap();
        (driver, temp_dir)
    }

    #[tokio::test]
    async fn test_native_driver_crud() {
        let (driver, _temp_dir) = setup_test_db().await;

        // Insert data
        let data = serde_json::json!({"name": "Test Item", "value": 42});
        let count = driver
            .insert(StorageType::Document, "test_collection", data)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Select data
        let results = driver
            .select(
                StorageType::Document,
                "test_collection",
                None,
                Some(10),
                Some(0),
            )
            .await
            .unwrap();
        assert!(!results.is_empty());

        println!("✓ Native driver CRUD test passed");
    }

    #[tokio::test]
    async fn test_driver_builder() {
        let driver = NativeDriverBuilder::new()
            .data_dir("/tmp/test")
            .port(9090)
            .build()
            .unwrap();

        // Just test that it builds successfully
        assert!(driver.db().get_cluster_status().await.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_prepared_statement_bind_and_execute() {
        let (driver, _temp_dir) = setup_test_db().await;
        let client = Arc::new(PrimusDBClient::from_arc(driver.db().clone()));

        // Test SQL construction and parameter binding
        let mut stmt = client.prepare("SELECT * FROM users WHERE age > ? AND name = ?");
        stmt.set_int(0, 21);
        stmt.set_string(1, "Alice");
        let sql = stmt.build_sql();
        assert_eq!(sql, "SELECT * FROM users WHERE age > 21 AND name = 'Alice'");

        // Test all parameter types
        let mut stmt2 = client.prepare("INSERT INTO items VALUES (?, ?, ?, ?)");
        stmt2.set_int(0, 42);
        stmt2.set_string(1, "hello");
        stmt2.set_double(2, 3.14);
        stmt2.set_bool(3, true);
        let sql2 = stmt2.build_sql();
        assert_eq!(sql2, "INSERT INTO items VALUES (42, 'hello', 3.14, true)");

        // Test NULL binding
        let mut stmt3 = client.prepare("UPDATE t SET x = ? WHERE id = ?");
        stmt3.set_null(0);
        stmt3.set_int(1, 99);
        let sql3 = stmt3.build_sql();
        assert_eq!(sql3, "UPDATE t SET x = NULL WHERE id = 99");

        println!("✓ PreparedStatement bind and execute test passed");
    }
}
