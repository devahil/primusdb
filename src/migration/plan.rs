//! Migration plan generation.
//!
//! The plan stage takes an inspected source schema and optional mapping
//! configuration, then produces a `MigrationPlan` with per-object mappings.
//!
//! ```text
//! +-------------+
//! | SourceSchema |  (from inspect_schema)
//! +------+------+
//!        |
//!        v
//! +-------------+     +-------------+
//! | MappingConfig|  --| (optional)  |
//! +------+------+     +-------------+
//!        |
//!        v
//! +-------------+
//! | generate_plan() |
//! +------+------+
//!        |
//!        v
//! +-------------+
//! | MigrationPlan |
//! +-------------+
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

use super::mapping::{apply_mapping, MappingConfig};
use super::source::SourceSchema;
use super::target::ObjectMapping;

/// Controls what the migration operation should do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationMode {
    /// Full copy of schema and data.
    Copy,
    /// Only create the schema, do not import data.
    SchemaOnly,
    /// Only import data (target must already exist).
    DataOnly,
    /// Validate the plan without making any changes.
    DryRun,
}

impl fmt::Display for MigrationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationMode::Copy => write!(f, "copy"),
            MigrationMode::SchemaOnly => write!(f, "schema-only"),
            MigrationMode::DataOnly => write!(f, "data-only"),
            MigrationMode::DryRun => write!(f, "dry-run"),
        }
    }
}

/// A complete migration plan describing what will be migrated and how.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// Type of source database (e.g. "mysql", "postgres").
    pub source_type: String,
    /// Connection URL for the source database.
    pub source_url: String,
    /// PrimusDB server URL.
    pub target_url: String,
    /// Namespace to import into.
    pub namespace: String,
    /// Migration mode.
    pub mode: MigrationMode,
    /// Object mappings describing source-to-target translations.
    pub objects: Vec<ObjectMapping>,
    /// Number of rows per batch.
    pub batch_size: u64,
    /// Total estimated rows across all objects.
    pub estimated_rows: u64,
    /// Non-fatal warnings about the migration.
    pub warnings: Vec<String>,
}

/// Generate a migration plan from a source schema and optional mapping configuration.
///
/// If no mapping file is provided, a 1:1 mapping is generated for all objects
/// in the schema using the default storage engine "relational".
pub fn generate_plan(
    schema: &SourceSchema,
    mapping_file: Option<&MappingConfig>,
    mode: &MigrationMode,
) -> MigrationPlan {
    let objects = match mapping_file {
        Some(cfg) => match apply_mapping(schema, cfg) {
            Ok(mappings) => mappings,
            Err(e) => {
                return MigrationPlan {
                    source_type: "unknown".into(),
                    source_url: String::new(),
                    target_url: String::new(),
                    namespace: "default".into(),
                    mode: mode.clone(),
                    objects: vec![],
                    batch_size: 1000,
                    estimated_rows: 0,
                    warnings: vec![format!("Mapping error: {}", e)],
                };
            }
        },
        None => {
            let mut mappings = Vec::new();
            for db in &schema.databases {
                for obj in &db.objects {
                    let pk = if obj.primary_key.is_empty() {
                        None
                    } else {
                        Some(obj.primary_key.join(","))
                    };
                    mappings.push(ObjectMapping {
                        source: format!("{}.{}", db.name, obj.name),
                        target: obj.name.clone(),
                        engine: "relational".into(),
                        primary_key: pk,
                        field_mappings: obj
                            .columns
                            .iter()
                            .map(|c| super::target::FieldMapping {
                                source: c.name.clone(),
                                target: c.name.clone(),
                                type_override: None,
                            })
                            .collect(),
                    });
                }
            }
            mappings
        }
    };

    let estimated_rows: u64 = schema
        .databases
        .iter()
        .flat_map(|db| db.objects.iter())
        .filter_map(|obj| obj.row_estimate)
        .sum();

    MigrationPlan {
        source_type: "unknown".into(),
        source_url: String::new(),
        target_url: String::new(),
        namespace: "default".into(),
        mode: mode.clone(),
        objects,
        batch_size: 1000,
        estimated_rows,
        warnings: vec![],
    }
}

/// Render a migration plan as a human-readable string suitable for display.
pub fn render_plan(plan: &MigrationPlan) -> String {
    let mut out = String::new();
    out.push_str("Migration Plan\n");
    out.push_str(&format!("  Source type:  {}\n", plan.source_type));
    out.push_str(&format!("  Source URL:   {}\n", plan.source_url));
    out.push_str(&format!("  Target URL:   {}\n", plan.target_url));
    out.push_str(&format!("  Namespace:    {}\n", plan.namespace));
    out.push_str(&format!("  Mode:         {}\n", plan.mode));
    out.push_str(&format!("  Batch size:   {}\n", plan.batch_size));
    out.push_str(&format!("  Est. rows:    {}\n", plan.estimated_rows));
    out.push_str(&format!("  Objects:      {}\n", plan.objects.len()));

    if !plan.warnings.is_empty() {
        out.push_str("  Warnings:\n");
        for w in &plan.warnings {
            out.push_str(&format!("    - {}\n", w));
        }
    }

    if !plan.objects.is_empty() {
        out.push_str("\n  Objects:\n");
        for obj in &plan.objects {
            out.push_str(&format!(
                "    {} -> {} (engine: {}, pk: {:?})\n",
                obj.source, obj.target, obj.engine, obj.primary_key
            ));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::source::{SourceColumn, SourceDatabase, SourceObject, SourceSchema};

    #[test]
    fn test_migration_mode_display() {
        assert_eq!(MigrationMode::Copy.to_string(), "copy");
        assert_eq!(MigrationMode::SchemaOnly.to_string(), "schema-only");
        assert_eq!(MigrationMode::DataOnly.to_string(), "data-only");
        assert_eq!(MigrationMode::DryRun.to_string(), "dry-run");
    }

    #[test]
    fn test_generate_plan_no_mapping() {
        let schema = SourceSchema {
            databases: vec![SourceDatabase {
                name: "mydb".into(),
                objects: vec![SourceObject {
                    name: "users".into(),
                    object_type: "table".into(),
                    columns: vec![
                        SourceColumn {
                            name: "id".into(),
                            data_type: "int".into(),
                            nullable: false,
                            is_primary_key: true,
                            max_length: None,
                        },
                        SourceColumn {
                            name: "name".into(),
                            data_type: "varchar".into(),
                            nullable: true,
                            is_primary_key: false,
                            max_length: Some(255),
                        },
                    ],
                    row_estimate: Some(1000),
                    primary_key: vec!["id".into()],
                }],
            }],
        };
        let plan = generate_plan(&schema, None, &MigrationMode::Copy);
        assert_eq!(plan.objects.len(), 1);
        assert_eq!(plan.objects[0].source, "mydb.users");
        assert_eq!(plan.objects[0].target, "users");
        assert_eq!(plan.estimated_rows, 1000);
        assert_eq!(plan.batch_size, 1000);
    }

    #[test]
    fn test_generate_plan_with_empty_schema() {
        let schema = SourceSchema { databases: vec![] };
        let plan = generate_plan(&schema, None, &MigrationMode::DryRun);
        assert!(plan.objects.is_empty());
        assert_eq!(plan.estimated_rows, 0);
    }

    #[test]
    fn test_render_plan() {
        let plan = MigrationPlan {
            source_type: "mysql".into(),
            source_url: "mysql://host/db".into(),
            target_url: "http://localhost:8080".into(),
            namespace: "default".into(),
            mode: MigrationMode::Copy,
            objects: vec![],
            batch_size: 1000,
            estimated_rows: 500,
            warnings: vec![],
        };
        let output = render_plan(&plan);
        assert!(output.contains("Migration Plan"));
        assert!(output.contains("mysql"));
        assert!(output.contains("localhost"));
        assert!(output.contains("copy"));
    }

    #[test]
    fn test_render_plan_with_warnings() {
        let plan = MigrationPlan {
            source_type: "postgres".into(),
            source_url: "postgres://host/db".into(),
            target_url: "http://localhost:8080".into(),
            namespace: "default".into(),
            mode: MigrationMode::Copy,
            objects: vec![],
            batch_size: 1000,
            estimated_rows: 0,
            warnings: vec!["No tables found".into()],
        };
        let output = render_plan(&plan);
        assert!(output.contains("Warnings"));
        assert!(output.contains("No tables found"));
    }
}
